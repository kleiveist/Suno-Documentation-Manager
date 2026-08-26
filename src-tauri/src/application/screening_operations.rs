use super::*;

impl WorkspaceApp {
    /// Explicit retry endpoint for the fully local stage. Imports and
    /// replacements also call this after persisting new release evidence; it
    /// never performs a network request.
    pub fn run_local_audio_screening_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        let evidence = self.persistence.evidence(id)?;
        let release = evidence
            .iter()
            .find(|item| {
                item.role == EvidenceRole::ReleaseWav
                    && item.verified
                    && item.verification_error.is_none()
                    && item.sha256.is_some()
            })
            .cloned()
            .ok_or_else(|| {
                AppError::Validation(
                    "Import and verify the authoritative final release audio before audio screening."
                        .into(),
                )
            })?;
        let root = self.track_root(&track)?;
        let record = audio_screening::local_fingerprint(
            &contained_path(&root, Path::new(&release.relative_path), true)?,
            &track.id,
            &release,
            &root,
            |stage, message| {
                on_progress(OperationProgress {
                    stage: stage.into(),
                    current_file: Some(message.into()),
                    ..OperationProgress::default()
                });
            },
        )?;
        track.audio_screening.local = record;
        self.ensure_release_unchanged_after_screening(&mut track, &release, &root)?;
        // A fresh local result belongs to this release only.  Do not carry an
        // old provider result (or its STALE invalidation marker) across an
        // explicit release replacement; the optional level starts clean and
        // is then represented from the current global configuration.
        if track.audio_screening.local.status == AudioScreeningStatus::FingerprintGenerated
            && (track.audio_screening.external.status == AudioScreeningStatus::Stale
                || !audio_screening::external_record_matches_source(
                    &track.audio_screening.external,
                    &track.id,
                    &release,
                ))
        {
            track.audio_screening.external = Default::default();
        }
        self.record_external_configuration_status_if_unrun(&mut track)?;
        // `local_fingerprint` writes the initial portable summary together
        // with its JSON record. The optional-provider status is resolved only
        // afterwards, so refresh just the Markdown summary to keep the
        // portable record and the persisted state in lockstep.
        audio_screening::refresh_screening_markdown(
            &root,
            &track.audio_screening.local,
            &track.audio_screening.external,
        )?;
        // A local record changes portable documentation artifacts. It has no
        // bearing on a valid external result for the same source bytes.
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: format!(
                "Local Chromaprint screening recorded: {}.",
                audio_screening::audio_screening_status_label(track.audio_screening.local.status)
            ),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    /// A missing optional provider is a documented skip, not a failed local
    /// screening or a legal conclusion. Once a record exists (including a
    /// stale record after release replacement), it is never overwritten here.
    pub(super) fn record_external_configuration_status_if_unrun(
        &self,
        track: &mut TrackRecord,
    ) -> Result<()> {
        if track.audio_screening.external.status != AudioScreeningStatus::NotRun {
            return Ok(());
        }
        let settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        let (status, message) =
            audio_screening::provider_configuration_status(&settings, credentials_present);
        match status {
            AudioScreeningProviderStatus::Ready => {}
            AudioScreeningProviderStatus::Disabled
            | AudioScreeningProviderStatus::NotConfigured => {
                track.audio_screening.external.status = AudioScreeningStatus::SkippedNotConfigured;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::ConfigurationInvalid => {
                track.audio_screening.external.status = AudioScreeningStatus::ConfigurationInvalid;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::AuthenticationFailed => {
                track.audio_screening.external.status = AudioScreeningStatus::AuthenticationFailed;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::ProviderUnavailable => {
                track.audio_screening.external.status = AudioScreeningStatus::ProviderUnavailable;
                track.audio_screening.external.message = message;
            }
        }
        Ok(())
    }

    /// The portable screening directory is system-owned.  Once the source
    /// binding is no longer current, preserve the old technical record below
    /// `.archive` instead of leaving a positive-looking result in the live
    /// track tree.  A subsequent local run publishes a fresh directory.
    pub(super) fn archive_current_audio_screening_artifacts(
        &self,
        track: &TrackRecord,
    ) -> Result<()> {
        let root = self.track_root(track)?;
        audio_screening::archive_current_screening_artifacts(&root)?;
        Ok(())
    }

    /// `audio_screening` operates on a private, byte-verified snapshot so an
    /// external edit cannot change the audio consumed by fpcalc/ACRCloud.
    /// Re-verify the managed source immediately before accepting the produced
    /// record, too: otherwise an out-of-band edit during the long operation
    /// could make its old snapshot appear current in the database.
    pub(super) fn ensure_release_unchanged_after_screening(
        &self,
        track: &mut TrackRecord,
        release: &EvidenceItem,
        root: &Path,
    ) -> Result<()> {
        let verification = evidence::verify(root, release.clone());
        let still_current = verification.as_ref().is_ok_and(|item| {
            item.verified
                && item.verification_error.is_none()
                && item.id == release.id
                && item.relative_path == release.relative_path
                && item.sha256 == release.sha256
                && item.size_bytes == release.size_bytes
        });
        if still_current {
            return Ok(());
        }

        // Keep the evidence record honest when verification produced a
        // controlled mismatch/missing result.  A low-level filesystem error
        // still results in a stale screening state below, without exposing
        // its raw path or decoder details to the user.
        if let Ok(verified) = verification {
            self.persistence.save_evidence(&track.id, &verified)?;
        }
        audio_screening::mark_screening_stale(&mut track.audio_screening);
        mark_content_changed(track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(track)?;
        self.archive_current_audio_screening_artifacts(track)?;
        Err(AppError::Validation(
            "The authoritative release audio changed while audio screening was running. The result was discarded; verify or replace the release file and run screening again."
                .into(),
        ))
    }

    /// Track state stores the expected hashes for the portable local record
    /// and (when retained) the raw provider response.  Hash generation must
    /// never make a modified artifact look current merely by hashing its new
    /// bytes into SHA256SUMS, so verify these state-to-artifact anchors before
    /// documents, hashes, or finalization use them.
    pub(super) fn ensure_current_audio_screening_artifacts(
        &self,
        track: &TrackRecord,
        evidence: &[EvidenceItem],
    ) -> Result<()> {
        let root = self.track_root(track)?;

        // Stale state is a durable invalidation marker. If a filesystem
        // failure prevented the best-effort archival move, never let
        // document/hash generation re-adopt the still-live old directory.
        // A local rerun (or a fresh release import) repairs it by archiving
        // and publishing a new source-bound artifact set.
        if matches!(
            track.audio_screening.local.status,
            AudioScreeningStatus::Stale
        ) || matches!(
            track.audio_screening.external.status,
            AudioScreeningStatus::Stale
        ) {
            let live_directory = contained_path(
                &root,
                Path::new(audio_screening::AUDIO_SCREENING_DIR),
                false,
            )?;
            match fs::symlink_metadata(&live_directory) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(AppError::Symlink(live_directory.display().to_string()));
                }
                Ok(_) => {
                    return Err(AppError::Validation(
                        "Stale audio-screening artifacts are still present in the live track directory. Run the local screening again after restoring or replacing the release audio."
                            .into(),
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::io(&live_directory, error)),
            }
            return Err(AppError::Validation(
                "Audio screening is stale because the authoritative release changed. Verify or replace the release audio and run local screening again before generating documents or hashes."
                    .into(),
            ));
        }

        let Some(release) = evidence.iter().find(|item| {
            item.role == EvidenceRole::ReleaseWav
                && item.verified
                && item.verification_error.is_none()
                && item.sha256.is_some()
        }) else {
            return Ok(());
        };

        if audio_screening::local_record_matches_source(
            &track.audio_screening.local,
            &track.id,
            release,
        ) && !audio_screening::local_artifact_is_current(&root, &track.audio_screening.local)
            .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The local audio-screening record no longer matches its portable artifact. Run the local Chromaprint screening again before continuing."
                    .into(),
            ));
        }

        if audio_screening::external_record_matches_source(
            &track.audio_screening.external,
            &track.id,
            release,
        ) && !audio_screening::external_response_artifact_is_current(
            &root,
            &track.audio_screening.external,
        )
        .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The external audio-screening response no longer matches its portable artifact. Run the explicit ACRCloud screening again before continuing."
                    .into(),
            ));
        }

        Ok(())
    }

    /// The sole call path that can upload an audio sample. The Tauri command is
    /// reached only from the explicit Step-07 user action; it is never invoked
    /// by opening a workspace, importing evidence, generating hashes, or
    /// finalizing a track.
    pub fn run_external_audio_screening_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        let evidence = self.persistence.evidence(id)?;
        let release = evidence
            .iter()
            .find(|item| {
                item.role == EvidenceRole::ReleaseWav
                    && item.verified
                    && item.verification_error.is_none()
                    && item.sha256.is_some()
            })
            .cloned()
            .ok_or_else(|| {
                AppError::Validation(
                    "Import and verify the authoritative final release audio before external screening."
                        .into(),
                )
            })?;
        if !audio_screening::local_record_matches_source(
            &track.audio_screening.local,
            &track.id,
            &release,
        ) {
            return Err(AppError::Validation(
                "Generate a current local Chromaprint fingerprint for the authoritative release audio before starting external screening."
                    .into(),
            ));
        }
        let root = self.track_root(&track)?;
        if !audio_screening::local_artifact_is_current(&root, &track.audio_screening.local)
            .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The current local Chromaprint record is missing or has changed. Run the local screening again before starting external screening."
                    .into(),
            ));
        }
        let settings = self.persistence.audio_screening_settings()?;
        let credentials = self.persistence.audio_screening_credentials()?;
        let source_path = contained_path(&root, Path::new(&release.relative_path), true)?;
        let record = audio_screening::run_external_audio_screening_with_credentials(
            audio_screening::ExternalAudioScreeningRequest {
                settings: &settings,
                credentials: credentials.as_ref().map(|(access_key, access_secret)| {
                    (access_key.as_str(), access_secret.as_str())
                }),
                source_path: &source_path,
                track_id: &track.id,
                evidence: &release,
                track_root: &root,
                local: Some(&track.audio_screening.local),
            },
            |stage, message| {
                on_progress(OperationProgress {
                    stage: stage.into(),
                    current_file: Some(message.into()),
                    ..OperationProgress::default()
                });
            },
        )?;
        track.audio_screening.external = record;
        self.ensure_release_unchanged_after_screening(&mut track, &release, &root)?;
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: format!(
                "External ACRCloud screening recorded: {}.",
                audio_screening::audio_screening_status_label(
                    track.audio_screening.external.status
                )
            ),
            track: Some(self.detail_from_record(track, false)?),
        })
    }
}
