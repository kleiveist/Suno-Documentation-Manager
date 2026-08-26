use super::*;

impl WorkspaceApp {
    pub fn open(path: &Path, create: bool) -> Result<Self> {
        let root = canonical_workspace(path, create)?;
        let persistence = Persistence::initialize(&root)?;
        if persistence.get_meta("workspace_id")?.is_none() {
            persistence.set_meta("workspace_id", &Uuid::new_v4().to_string())?;
        }
        let app = Self { root, persistence };
        ensure_contained_directory(&app.root, Path::new(SINGLES_DIRECTORY))?;
        app.reconcile_physical_library()?;
        app.recover_interrupted_operations()?;
        let profile = app.profile()?;
        app.synchronize_open_track_profiles(&profile, false)?;
        Ok(app)
    }

    pub(super) fn synchronize_open_track_profiles(
        &self,
        profile: &Profile,
        save_profile: bool,
    ) -> Result<()> {
        let mut tracks = self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
            .collect::<Vec<_>>();
        let updated_at = now();
        let mut changed = false;
        for track in &mut tracks {
            if matches!(
                track.status,
                TrackStatus::Finalized | TrackStatus::Superseded
            ) || track
                .profile_snapshot
                .same_track_documentation_profile(profile)
            {
                continue;
            }
            track.profile_snapshot = profile.clone();
            mark_content_changed(track);
            track.status = TrackStatus::Active;
            track.updated_at = updated_at.clone();
            changed = true;
        }
        if save_profile {
            self.persistence.save_profile_and_tracks(profile, &tracks)?;
        } else if changed {
            self.persistence.save_tracks(&tracks)?;
        }
        Ok(())
    }

    pub(super) fn recover_interrupted_operations(&self) -> Result<()> {
        let mut recovered = false;
        for track in self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
        {
            if self.recover_track_operations(&track)? {
                recovered = true;
            }
        }
        if recovered {
            self.persistence.set_meta("last_recovery_at", &now())?;
        }
        Ok(())
    }

    fn recover_track_operations(&self, track: &TrackRecord) -> Result<bool> {
        if !self.root.join(&track.relative_path).is_dir() {
            return Ok(false);
        }
        let root = self.track_root(track)?;
        let live = contained_path(&root, Path::new(certificate::CERTIFICATE_DIR), false)?;
        let mut recovered = self.recover_interrupted_revision(track, &root, &live)?;
        if self.recover_finalization_marker(track, &root, &live)? {
            recovered = true;
        }
        let timestamp_records = self.persistence.external_timestamps(&track.id)?;
        if external_timestamp::reconcile_publications(&root, &timestamp_records)? {
            recovered = true;
        }
        Ok(recovered)
    }

    fn recover_interrupted_revision(
        &self,
        track: &TrackRecord,
        root: &Path,
        live: &Path,
    ) -> Result<bool> {
        if track.status != TrackStatus::Finalized {
            return Ok(false);
        }
        let mut recovered = false;
        if finalized_artifacts_need_revision_restore(root, live)?
            && self.restore_interrupted_revision_staging(track, root, live)?
        {
            recovered = true;
        }
        if finalized_artifacts_need_revision_restore(root, live)?
            && self.restore_interrupted_revision_archive(track, root, live)?
        {
            recovered = true;
        }
        Ok(recovered)
    }

    fn restore_interrupted_revision_staging(
        &self,
        track: &TrackRecord,
        root: &Path,
        live: &Path,
    ) -> Result<bool> {
        let staging = contained_path(root, Path::new(".archive/revision-staging"), false)?;
        if !staging.is_dir() {
            return Ok(false);
        }
        for entry in fs::read_dir(&staging).map_err(|error| AppError::io(&staging, error))? {
            let entry = entry.map_err(|error| AppError::io(&staging, error))?;
            let entry_path = entry.path();
            let entry_metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| AppError::io(&entry_path, error))?;
            if entry_metadata.file_type().is_symlink() || !entry_metadata.is_dir() {
                continue;
            }
            let entry_relative = entry_path
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?
                .to_owned();
            if let Some(candidate) = matching_revision_certificate(root, &entry_relative, track)? {
                if restore_revision_artifacts(live, &candidate)? {
                    let _ = fs::remove_dir_all(&entry_path);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn restore_interrupted_revision_archive(
        &self,
        track: &TrackRecord,
        root: &Path,
        live: &Path,
    ) -> Result<bool> {
        let revisions = contained_path(root, Path::new(".archive/revisions"), false)?;
        if !revisions.is_dir() {
            return Ok(false);
        }
        let mut entries = fs::read_dir(&revisions)
            .map_err(|error| AppError::io(&revisions, error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| AppError::io(&revisions, error))?;
        entries.sort_by_key(|entry| entry.file_name());
        entries.reverse();
        for entry in entries {
            let entry_metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| AppError::io(entry.path(), error))?;
            if entry_metadata.file_type().is_symlink() || !entry_metadata.is_dir() {
                continue;
            }
            let entry_path = entry.path();
            let entry_relative = entry_path
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?;
            let Some(candidate) = matching_revision_certificate(root, entry_relative, track)?
            else {
                continue;
            };
            if restore_revision_artifacts(live, &candidate)? {
                let _ = fs::remove_dir_all(&entry_path);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn recover_finalization_marker(
        &self,
        track: &TrackRecord,
        root: &Path,
        live: &Path,
    ) -> Result<bool> {
        // A non-finalized track may be an imported legacy folder whose historical
        // certificate bytes must remain untouched. Finalization crash recovery is
        // therefore performed only when a transaction marker created immediately
        // before certificate publication is present.
        let marker_path = contained_path(
            root,
            Path::new(".archive/finalization-in-progress.json"),
            false,
        )?;
        if !marker_path.is_file() {
            return Ok(false);
        }
        if track.status == TrackStatus::Finalized {
            fs::remove_file(&marker_path).map_err(|error| AppError::io(&marker_path, error))?;
            return Ok(true);
        }
        self.archive_interrupted_finalization(track, root, live, &marker_path)?;
        // Recovery is idempotent across process exits between the individual
        // moves. Recreate the managed empty directory even when a prior attempt
        // already moved every correlated artifact.
        ensure_contained_directory(root, Path::new(certificate::CERTIFICATE_DIR))?;
        fs::remove_file(&marker_path).map_err(|error| AppError::io(&marker_path, error))?;
        Ok(true)
    }

    fn archive_interrupted_finalization(
        &self,
        track: &TrackRecord,
        root: &Path,
        live: &Path,
        marker_path: &Path,
    ) -> Result<()> {
        let recovery_id = Self::finalization_recovery_id(track, marker_path)?;
        let staging_relative = PathBuf::from(".archive/certificate-staging").join(&recovery_id);
        let staging = contained_path(root, &staging_relative, false)?;
        let live_needs_recovery = !directory_is_empty_or_missing(live)?;
        let live_pdf_paths = [certificate::PDF_FILE, certificate::PDF_FILE_DE];
        let live_pdf_needs_recovery = live_pdf_paths.iter().any(|name| root.join(name).exists());
        let staging_needs_recovery = staging.exists();
        if !(live_needs_recovery || live_pdf_needs_recovery || staging_needs_recovery) {
            return Ok(());
        }
        let recovery_relative = PathBuf::from(".archive/recovery").join(&recovery_id);
        let recovery = ensure_contained_directory(root, &recovery_relative)?;
        let metadata = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "track_id": track.id,
            "recovered_at": now(),
            "reason": "certificate publication was interrupted before the database committed FINALIZED",
            "live_certificate_recovered": live_needs_recovery,
            "live_pdfs_recovered": live_pdf_needs_recovery,
            "staging_recovered": staging_needs_recovery,
        }))?;
        let recovery_metadata = recovery.join("recovery.json");
        if !recovery_metadata.exists() {
            atomic_write_new(&recovery_metadata, &metadata)?;
        }
        Self::move_interrupted_finalization_artifacts(
            root,
            live,
            &staging,
            &recovery,
            live_needs_recovery,
            live_pdf_needs_recovery,
            staging_needs_recovery,
        )?;
        Ok(())
    }

    fn finalization_recovery_id(track: &TrackRecord, marker_path: &Path) -> Result<String> {
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(marker_path).map_err(|error| AppError::io(marker_path, error))?,
        )?;
        if marker.get("track_id").and_then(|value| value.as_str()) != Some(track.id.as_str()) {
            return Err(AppError::Data(
                "Finalization recovery marker does not match its track.".into(),
            ));
        }
        marker
            .get("transaction_id")
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::Data("Finalization recovery marker has no transaction ID.".into())
            })
    }

    fn move_interrupted_finalization_artifacts(
        root: &Path,
        live: &Path,
        staging: &Path,
        recovery: &Path,
        live_needs_recovery: bool,
        live_pdf_needs_recovery: bool,
        staging_needs_recovery: bool,
    ) -> Result<()> {
        if live_needs_recovery {
            fs::rename(live, recovery.join("certificate"))
                .map_err(|error| AppError::io(live, error))?;
        }
        if live_pdf_needs_recovery {
            for name in [certificate::PDF_FILE, certificate::PDF_FILE_DE] {
                let live_pdf = root.join(name);
                if live_pdf.exists() {
                    fs::rename(&live_pdf, recovery.join(name))
                        .map_err(|error| AppError::io(&live_pdf, error))?;
                }
            }
        }
        if staging_needs_recovery {
            fs::rename(staging, recovery.join("certificate-staging"))
                .map_err(|error| AppError::io(staging, error))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn summary(&self) -> Result<WorkspaceSummary> {
        let id = self
            .persistence
            .get_meta("workspace_id")?
            .unwrap_or_else(|| "local-workspace".into());
        let name = self
            .root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workspace")
            .to_owned();
        Ok(WorkspaceSummary {
            id,
            name,
            path: self.root.display().to_string(),
            track_count: self
                .persistence
                .tracks()?
                .into_iter()
                .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
                .count() as u32,
            last_scanned_at: self.persistence.get_meta("last_scanned_at")?,
        })
    }

    pub fn profile(&self) -> Result<Profile> {
        self.persistence.profile()
    }

    pub fn update_profile(&self, profile: Profile) -> Result<Profile> {
        validate_profile(&profile, false)?;
        self.synchronize_open_track_profiles(&profile, true)?;
        Ok(profile)
    }

    /// Returns the workspace-global, non-secret timestamp configuration. It
    /// never travels through a track profile snapshot or a certificate.
    pub fn timestamp_settings(&self) -> Result<TimestampSettings> {
        let mut settings = self.persistence.timestamp_settings()?;
        let (status, message) = external_timestamp::settings_status(
            &settings,
            self.persistence.timestamp_secret_present()?,
        );
        settings.status = status;
        settings.status_message = message;
        Ok(settings)
    }

    pub fn update_timestamp_settings(
        &self,
        mut settings: TimestampSettings,
    ) -> Result<TimestampSettings> {
        // These fields are derived server-side. A UI payload must not be able
        // to persist a misleading green status or an arbitrary test timestamp.
        let previous = self.persistence.timestamp_settings()?;
        settings.status = TimestampProviderConfigurationStatus::Disabled;
        settings.status_message.clear();
        settings.last_tested_at = previous.last_tested_at;
        if settings.provider == crate::model::TimestampProviderKind::Disabled {
            settings.enabled = false;
        }
        let (status, message) = external_timestamp::settings_status(
            &settings,
            self.persistence.timestamp_secret_present()?,
        );
        settings.status = status;
        settings.status_message = message;
        self.persistence.save_timestamp_settings(&settings)?;
        Ok(settings)
    }

    /// Stores a write-only Custom RFC 3161 credential outside ordinary
    /// workspace JSON. The method intentionally returns no secret value.
    pub fn update_timestamp_secret(&self, input: TimestampSecretInput) -> Result<()> {
        self.persistence
            .save_timestamp_secret(input.secret.as_deref())
    }

    pub fn test_timestamp_provider(&self) -> Result<TimestampProviderTestResult> {
        let mut settings = self.persistence.timestamp_settings()?;
        let secret = self.persistence.timestamp_secret()?;
        let result = external_timestamp::test_provider(&settings, secret.as_deref());
        settings.status = result.status;
        settings.status_message = result.message.clone();
        settings.last_tested_at = Some(result.tested_at.clone());
        self.persistence.save_timestamp_settings(&settings)?;
        Ok(result)
    }

    /// Returns only global, non-secret ACRCloud configuration. The local
    /// engine status is checked from the app-controlled sidecar; credentials
    /// are represented only by a boolean derived from the private file.
    pub fn audio_screening_settings(&self) -> Result<AudioScreeningSettings> {
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        Ok(settings)
    }

    pub fn update_audio_screening_settings(
        &self,
        mut settings: AudioScreeningSettings,
    ) -> Result<AudioScreeningSettings> {
        // Never trust UI-provided state labels, credential flags, engine data,
        // or timestamps. All four are derived in the native layer.
        let previous = self.persistence.audio_screening_settings()?;
        settings.status = AudioScreeningProviderStatus::Disabled;
        settings.status_message.clear();
        settings.credentials_configured = false;
        settings.local_engine_available = false;
        settings.local_engine_version.clear();
        settings.last_tested_at = previous.last_tested_at;
        if settings.timeout_seconds == 0 {
            settings.timeout_seconds = 30;
        }
        audio_screening::validate_acrcloud_sampling_settings(&settings)?;
        if let Ok(normalized) = audio_screening::normalize_acrcloud_host(&settings.host) {
            settings.host = normalized;
        }
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        self.persistence.save_audio_screening_settings(&settings)?;
        Ok(settings)
    }

    /// Stores the pair outside all serializable workspace state. No credential
    /// is returned, logged, added to a profile, or copied to a track.
    pub fn update_audio_screening_secret(&self, input: AudioScreeningSecretInput) -> Result<()> {
        self.persistence.save_audio_screening_secret(input)?;
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        self.persistence.save_audio_screening_settings(&settings)
    }

    /// This sends no audio and no credentials. It is a bounded HTTPS
    /// reachability/configuration test for the explicitly configured host.
    pub fn test_audio_screening_provider(&self) -> Result<AudioScreeningProviderTestResult> {
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        let result = audio_screening::test_acrcloud_provider(&settings, credentials_present);
        settings.status = result.status;
        settings.status_message = result.message.clone();
        settings.credentials_configured = credentials_present;
        settings.last_tested_at = Some(result.tested_at.clone());
        audio_screening::refresh_local_engine_status(&mut settings);
        self.persistence.save_audio_screening_settings(&settings)?;
        Ok(result)
    }
}
