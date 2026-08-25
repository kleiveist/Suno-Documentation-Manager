use super::*;

struct IndexedEvidenceRemoval {
    removal_id: String,
    directory: PathBuf,
    path: PathBuf,
    archived: PathBuf,
    moved: bool,
}

struct ManagedEvidenceRemoval {
    path: PathBuf,
    directory: Option<PathBuf>,
    archived: Option<PathBuf>,
}

struct LoadedEvidencePreview {
    mime_type: Option<String>,
    data_url: Option<String>,
    text_content: Option<String>,
    message: Option<String>,
}

impl WorkspaceApp {
    #[cfg(test)]
    pub fn import_evidence_from(
        &self,
        id: &str,
        role: EvidenceRole,
        source: &Path,
    ) -> Result<TrackDetail> {
        self.import_evidence_with_metadata_from(id, role, source, EvidenceMetadata::default())
    }

    pub fn import_evidence_with_metadata_from(
        &self,
        id: &str,
        role: EvidenceRole,
        source: &Path,
        mut metadata: EvidenceMetadata,
    ) -> Result<TrackDetail> {
        if role == EvidenceRole::ExternalTimestamp {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached after technical finalization."
                    .into(),
            ));
        }
        if matches!(
            role,
            EvidenceRole::SubscriptionPayment | EvidenceRole::SunoTermsRights
        ) {
            return Err(AppError::Validation(
                "Register subscription and Suno terms/rights evidence globally in Settings and attach a portable copy."
                    .into(),
            ));
        }
        let mut track = self.mutable_track(id)?;
        if matches!(
            role,
            EvidenceRole::ReleaseWav | EvidenceRole::SunoFinalExport | EvidenceRole::FinalArtwork
        ) && self
            .persistence
            .evidence(id)?
            .iter()
            .any(|item| item.role == role)
        {
            return Err(AppError::Validation(format!(
                "Die Evidence-Rolle '{}' ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen.",
                role.as_str()
            )));
        }
        let track_root = self.track_root(&track)?;
        let planned_relative = evidence::managed_relative_path(&track.fields.title, &role, source)?;
        let planned_portable = portable_relative(&planned_relative);
        if self
            .persistence
            .evidence_by_relative_path(id, &planned_portable)?
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "Unter {planned_portable} ist bereits Evidence registriert. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen."
            )));
        }
        let original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let mut item = evidence::import(&track_root, &track.fields.title, role, source)?;
        let managed_path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        let persist_result = (|| -> Result<()> {
            metadata = evidence::capture_automatic_metadata(&managed_path, metadata);
            metadata.original_file_name = original_file_name;
            validate_evidence_metadata(&role, &metadata)?;
            item.metadata = metadata;

            let mut all_evidence = self.persistence.evidence(id)?;
            all_evidence.push(item.clone());
            reconcile_evidence_derived_fields(&mut track, &all_evidence);
            if role == EvidenceRole::ReleaseWav {
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            if role == EvidenceRole::ReleaseWav {
                track.fields.release_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoFinalExport {
                track.fields.suno_export_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoTermsRights {
                track.fields.suno_terms_evidence_not_available = Some(false);
            }
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence
                .save_track_and_evidence(&track, std::slice::from_ref(&item))
        })();
        if let Err(error) = persist_result {
            let _ = fs::remove_file(&managed_path);
            return Err(error);
        }
        if role == EvidenceRole::ReleaseWav {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        let detail = self.detail_from_record(track, false)?;
        if role == EvidenceRole::ReleaseWav {
            return self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                // The evidence import remains durable if a subsequent local
                // best-effort run hits an unexpected filesystem error. The
                // unfulfilled workflow requirement then makes the condition
                // visible rather than presenting an old fingerprint as PASS.
                .or(Ok(detail));
        }
        Ok(detail)
    }

    #[cfg(test)]
    pub fn replace_evidence_from(
        &self,
        id: &str,
        evidence_id: &str,
        role: EvidenceRole,
        source: &Path,
    ) -> Result<TrackDetail> {
        self.replace_evidence_with_metadata_from(
            id,
            evidence_id,
            role,
            source,
            EvidenceMetadata::default(),
        )
    }

    pub fn replace_evidence_with_metadata_from(
        &self,
        id: &str,
        evidence_id: &str,
        role: EvidenceRole,
        source: &Path,
        metadata: EvidenceMetadata,
    ) -> Result<TrackDetail> {
        if role == EvidenceRole::ExternalTimestamp {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached after technical finalization."
                    .into(),
            ));
        }
        if matches!(
            role,
            EvidenceRole::SubscriptionPayment | EvidenceRole::SunoTermsRights
        ) {
            return Err(AppError::Validation(
                "Replace subscription and Suno terms/rights evidence in the global evidence register."
                    .into(),
            ));
        }
        let mut track = self.mutable_track(id)?;
        let previous = self.persistence.evidence_item(id, evidence_id)?;
        if previous.role != role {
            return Err(AppError::Validation(
                "The selected replacement role does not match the existing evidence.".into(),
            ));
        }
        let planned_relative = evidence::managed_relative_path(&track.fields.title, &role, source)?;
        let planned_portable = portable_relative(&planned_relative);
        if self
            .persistence
            .evidence_by_relative_path(id, &planned_portable)?
            .is_some_and(|item| item.id != previous.id)
        {
            return Err(AppError::Validation(format!(
                "Another evidence record already uses {planned_portable}. Remove that record before replacing this file."
            )));
        }
        let track_root = self.track_root(&track)?;
        let original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let track_title = track.fields.title.clone();
        evidence::replace(&track_root, &track_title, role, source, &previous, |item| {
            let mut item = item.clone();
            let managed_path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
            let mut captured =
                evidence::capture_automatic_metadata(&managed_path, metadata.clone());
            captured.original_file_name = original_file_name.clone();
            validate_evidence_metadata(&role, &captured)?;
            item.metadata = captured;

            let mut all_evidence = self.persistence.evidence(id)?;
            let stored = all_evidence
                .iter_mut()
                .find(|stored| stored.id == item.id)
                .ok_or_else(|| AppError::EvidenceNotFound(item.id.clone()))?;
            *stored = item.clone();
            reconcile_evidence_derived_fields(&mut track, &all_evidence);
            if role == EvidenceRole::ReleaseWav {
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            if role == EvidenceRole::ReleaseWav {
                track.fields.release_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoFinalExport {
                track.fields.suno_export_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoTermsRights {
                track.fields.suno_terms_evidence_not_available = Some(false);
            }
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence
                .save_track_and_evidence(&track, std::slice::from_ref(&item))
        })?;
        if role == EvidenceRole::ReleaseWav {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        let detail = self.detail_from_record(track, false)?;
        if role == EvidenceRole::ReleaseWav {
            return self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .or(Ok(detail));
        }
        Ok(detail)
    }

    pub fn remove_evidence(&self, id: &str, evidence_id: &str) -> Result<TrackDetail> {
        let track = self.mutable_track(id)?;
        let item = self.persistence.evidence_item(id, evidence_id)?;
        if item.provenance == EvidenceProvenance::IndexedLegacy {
            self.remove_indexed_legacy_evidence(id, evidence_id, track, item)
        } else {
            self.remove_managed_evidence(id, evidence_id, track, item)
        }
    }

    fn remove_indexed_legacy_evidence(
        &self,
        id: &str,
        evidence_id: &str,
        mut track: TrackRecord,
        item: EvidenceItem,
    ) -> Result<TrackDetail> {
        let removed_authoritative_release = item.role == EvidenceRole::ReleaseWav;
        let track_root = self.track_root(&track)?;
        let removal = Self::archive_indexed_legacy_evidence(&track_root, &item)?;
        Self::write_indexed_removal_metadata(&track, &item, &removal)?;
        if let Err(error) = self.persistence.remove_evidence(id, evidence_id) {
            Self::rollback_indexed_unregistration(&removal);
            return Err(error);
        }
        let remaining_evidence = self.persistence.evidence(id)?;
        Self::update_track_after_evidence_removal(&mut track, &item, &remaining_evidence);
        if let Err(error) = self.persistence.save_track(&track) {
            return Err(self.rollback_indexed_track_save(id, &item, &removal, error));
        }
        if removed_authoritative_release {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        self.detail_from_record(track, false)
    }

    fn archive_indexed_legacy_evidence(
        track_root: &Path,
        item: &EvidenceItem,
    ) -> Result<IndexedEvidenceRemoval> {
        let path = contained_path(track_root, Path::new(&item.relative_path), false)?;
        let removal_id = Uuid::new_v4().to_string();
        let removal_relative = PathBuf::from(".archive/removals").join(&removal_id);
        let directory = ensure_contained_directory(track_root, &removal_relative)?;
        let archived = directory.join(
            path.file_name()
                .ok_or_else(|| AppError::Data("Legacy evidence path has no file name.".into()))?,
        );
        let moved = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::Symlink(path.display().to_string()));
            }
            Ok(metadata) if metadata.is_file() => {
                fs::rename(&path, &archived).map_err(|error| AppError::io(&path, error))?;
                true
            }
            Ok(_) => {
                return Err(AppError::Validation(
                    "Indexed legacy evidence is not a regular file.".into(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(AppError::io(&path, error)),
        };
        Ok(IndexedEvidenceRemoval {
            removal_id,
            directory,
            path,
            archived,
            moved,
        })
    }

    fn write_indexed_removal_metadata(
        track: &TrackRecord,
        item: &EvidenceItem,
        removal: &IndexedEvidenceRemoval,
    ) -> Result<()> {
        let metadata = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "removal_id": removal.removal_id,
            "track_id": track.id,
            "removed_at": now(),
            "reason": "legacy evidence removed from the application index",
            "original_relative_path": item.relative_path,
            "evidence": item,
        }))?;
        if let Err(error) = atomic_write_new(&removal.directory.join("removal.json"), &metadata) {
            if removal.moved {
                let _ = fs::rename(&removal.archived, &removal.path);
            }
            let _ = fs::remove_dir(&removal.directory);
            return Err(error);
        }
        Ok(())
    }

    fn rollback_indexed_unregistration(removal: &IndexedEvidenceRemoval) {
        if removal.moved {
            let _ = fs::rename(&removal.archived, &removal.path);
        }
        let _ = fs::remove_file(removal.directory.join("removal.json"));
        let _ = fs::remove_dir(&removal.directory);
    }

    fn rollback_indexed_track_save(
        &self,
        id: &str,
        item: &EvidenceItem,
        removal: &IndexedEvidenceRemoval,
        error: AppError,
    ) -> AppError {
        let database_rollback = self.persistence.save_evidence(id, item);
        let file_rollback = if removal.moved {
            fs::rename(&removal.archived, &removal.path)
        } else {
            Ok(())
        };
        if database_rollback.is_ok() && file_rollback.is_ok() {
            let _ = fs::remove_file(removal.directory.join("removal.json"));
            let _ = fs::remove_dir(&removal.directory);
            return error;
        }
        AppError::Data(format!(
            "Legacy evidence removal failed ({error}); rollback was incomplete."
        ))
    }

    fn remove_managed_evidence(
        &self,
        id: &str,
        evidence_id: &str,
        mut track: TrackRecord,
        item: EvidenceItem,
    ) -> Result<TrackDetail> {
        let removed_authoritative_release = item.role == EvidenceRole::ReleaseWav;
        let track_root = self.track_root(&track)?;
        let removal = Self::archive_managed_evidence(&track_root, &item)?;
        if let Err(error) = self.persistence.remove_evidence(id, evidence_id) {
            return Err(match removal.archived.as_deref() {
                Some(archived) => rollback_removed_file(archived, &removal.path, error),
                None => error,
            });
        }
        let remaining_evidence = self.persistence.evidence(id)?;
        Self::update_track_after_evidence_removal(&mut track, &item, &remaining_evidence);
        if let Err(error) = self.persistence.save_track(&track) {
            return Err(self.rollback_managed_track_save(id, &item, &removal, error));
        }
        Self::cleanup_managed_removal(&removal);
        if removed_authoritative_release {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        self.detail_from_record(track, false)
    }

    fn archive_managed_evidence(
        track_root: &Path,
        item: &EvidenceItem,
    ) -> Result<ManagedEvidenceRemoval> {
        let path = contained_path(track_root, Path::new(&item.relative_path), false)?;
        let mut directory = None;
        let mut archived = None;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let removal_relative =
                    PathBuf::from(".archive/removals").join(Uuid::new_v4().to_string());
                let removal_directory = ensure_contained_directory(track_root, &removal_relative)?;
                let archive_path = removal_directory.join(
                    path.file_name()
                        .ok_or_else(|| AppError::Data("Evidence path has no file name.".into()))?,
                );
                fs::rename(&path, &archive_path).map_err(|error| AppError::io(&path, error))?;
                directory = Some(removal_directory);
                archived = Some(archive_path);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::Symlink(path.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::io(&path, error)),
        }
        Ok(ManagedEvidenceRemoval {
            path,
            directory,
            archived,
        })
    }

    fn update_track_after_evidence_removal(
        track: &mut TrackRecord,
        item: &EvidenceItem,
        remaining_evidence: &[EvidenceItem],
    ) {
        reconcile_evidence_derived_fields(track, remaining_evidence);
        if item.role == EvidenceRole::ReleaseWav {
            audio_screening::mark_screening_stale(&mut track.audio_screening);
        }
        mark_content_changed(track);
        if item.provenance == EvidenceProvenance::GeneratedDisclosure {
            track.fields.disclosure_applied = None;
        }
        if item.role == EvidenceRole::ReleaseWav {
            track.fields.release_filename_difference_confirmed = None;
        } else if item.role == EvidenceRole::SunoFinalExport {
            track.fields.suno_export_filename_difference_confirmed = None;
        }
        track.updated_at = now();
    }

    fn rollback_managed_track_save(
        &self,
        id: &str,
        item: &EvidenceItem,
        removal: &ManagedEvidenceRemoval,
        error: AppError,
    ) -> AppError {
        let database_rollback = self.persistence.save_evidence(id, item);
        let file_rollback = removal
            .archived
            .as_deref()
            .map_or(Ok(()), |archived| fs::rename(archived, &removal.path));
        if let (Ok(()), Ok(())) = (database_rollback, file_rollback) {
            if let Some(directory) = removal.directory.as_deref() {
                let _ = fs::remove_dir(directory);
            }
            return error;
        }
        AppError::Data(format!(
            "Evidence removal failed ({error}); rollback was incomplete."
        ))
    }

    fn cleanup_managed_removal(removal: &ManagedEvidenceRemoval) {
        if let Some(archived) = removal.archived.as_deref() {
            let _ = fs::remove_file(archived);
        }
        if let Some(directory) = removal.directory.as_deref() {
            let _ = fs::remove_dir(directory);
        }
    }

    pub fn preview_evidence(&self, id: &str, evidence_id: &str) -> Result<EvidencePreview> {
        let track = self.persistence.track(id)?;
        let item = self.persistence.evidence_item(id, evidence_id)?;
        let track_root = self.track_root(&track)?;
        let path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::Validation(
                "Evidence preview requires a regular managed file.".into(),
            ));
        }
        evidence::validate_type(&item.role, &path)?;
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (image_mime, text_mime) = Self::evidence_preview_mime_types(&item, &extension);
        let preview =
            Self::load_evidence_preview(&path, metadata.len(), image_mime, text_mime, &extension)?;
        Ok(EvidencePreview {
            evidence_id: item.id,
            role: item.role,
            file_name: item.file_name,
            relative_path: item.relative_path,
            size_bytes: metadata.len(),
            mime_type: preview.mime_type,
            data_url: preview.data_url,
            text_content: preview.text_content,
            message: preview.message,
        })
    }

    fn evidence_preview_mime_types(
        item: &EvidenceItem,
        extension: &str,
    ) -> (Option<&'static str>, Option<&'static str>) {
        let image_mime = match extension {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            _ => None,
        };
        let text_mime = match extension {
            "txt" => Some("text/plain"),
            "md" => Some("text/markdown"),
            "json" => Some("application/json"),
            _ if item.role == EvidenceRole::SourceCodeFile => Some("text/plain"),
            _ => None,
        };
        (image_mime, text_mime)
    }

    fn load_evidence_preview(
        path: &Path,
        size_bytes: u64,
        image_mime: Option<&str>,
        text_mime: Option<&str>,
        extension: &str,
    ) -> Result<LoadedEvidencePreview> {
        const IMAGE_PREVIEW_LIMIT: u64 = 16 * 1024 * 1024;
        const TEXT_PREVIEW_LIMIT: u64 = 512 * 1024;
        if let Some(mime) = image_mime {
            if size_bytes <= IMAGE_PREVIEW_LIMIT {
                let bytes = fs::read(path).map_err(|error| AppError::io(path, error))?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                return Ok(LoadedEvidencePreview {
                    mime_type: Some(mime.into()),
                    data_url: Some(format!("data:{mime};base64,{encoded}")),
                    text_content: None,
                    message: None,
                });
            }
            return Ok(LoadedEvidencePreview {
                mime_type: Some(mime.into()),
                data_url: None,
                text_content: None,
                message: Some(
                    "Das Bild ist größer als 16 MB und wird deshalb nicht in den Arbeitsspeicher geladen."
                        .into(),
                ),
            });
        }
        if let Some(mime) = text_mime {
            if size_bytes <= TEXT_PREVIEW_LIMIT {
                let bytes = fs::read(path).map_err(|error| AppError::io(path, error))?;
                let text = String::from_utf8_lossy(&bytes).into_owned();
                return Ok(LoadedEvidencePreview {
                    mime_type: Some(mime.into()),
                    data_url: None,
                    text_content: Some(text),
                    message: None,
                });
            }
            return Ok(LoadedEvidencePreview {
                mime_type: Some(mime.into()),
                data_url: None,
                text_content: None,
                message: Some(
                    "Die Textdatei ist größer als 512 KB und wird deshalb nicht vollständig geladen."
                        .into(),
                ),
            });
        }
        let message = if extension == "zip" {
            "ZIP-Dateien werden für die Vorschau nicht entpackt oder in den Arbeitsspeicher geladen."
        } else {
            "Für diesen Dateityp ist keine sichere Vorschau innerhalb der App verfügbar."
        };
        Ok(LoadedEvidencePreview {
            mime_type: None,
            data_url: None,
            text_content: None,
            message: Some(message.into()),
        })
    }

    pub fn track_cover(&self, id: &str) -> Result<Option<TrackCoverPreview>> {
        let track = self.persistence.track(id)?;
        let Some(item) = self.persistence.evidence(id)?.into_iter().find(|item| {
            item.role == EvidenceRole::FinalArtwork
                && item.verified
                && item.sha256.is_some()
                && item.verification_error.is_none()
        }) else {
            return Ok(None);
        };
        let track_root = self.track_root(&track)?;
        let path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        evidence::validate_type(&item.role, &path)?;
        let encoded = crate::artwork::centered_cover_thumbnail(&path)?;
        Ok(Some(TrackCoverPreview {
            evidence_id: item.id,
            data_url: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(encoded)
            ),
        }))
    }

    pub fn verify_evidence(&self, id: &str, evidence_id: Option<&str>) -> Result<TrackDetail> {
        let mut track = self.persistence.track(id)?;
        let track_root = self.track_root(&track)?;
        let items = self.persistence.evidence(id)?;
        if let Some(wanted) = evidence_id {
            if !items.iter().any(|item| item.id == wanted) {
                return Err(AppError::EvidenceNotFound(wanted.into()));
            }
        }
        let mut mismatch = false;
        let mut authoritative_release_mismatch = false;
        for item in items {
            if evidence_id.is_none() || evidence_id == Some(item.id.as_str()) {
                let is_authoritative_release = item.role == EvidenceRole::ReleaseWav;
                let verified = evidence::verify(&track_root, item)?;
                mismatch |= !verified.verified;
                authoritative_release_mismatch |= is_authoritative_release && !verified.verified;
                self.persistence.save_evidence(id, &verified)?;
            }
        }
        if authoritative_release_mismatch
            && !matches!(
                track.status,
                TrackStatus::Finalized | TrackStatus::Superseded
            )
        {
            // The byte-integrity verifier discovered that the authoritative
            // source no longer matches the recorded release.  Preserve old
            // screening artifacts below `.archive` and make both levels
            // visibly stale; do not leave a prior positive record live.
            audio_screening::mark_screening_stale(&mut track.audio_screening);
            mark_content_changed(&mut track);
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence.save_track(&track)?;
            self.archive_current_audio_screening_artifacts(&track)?;
        } else if mismatch && track.status == TrackStatus::Finalized {
            invalidate_state(
                &mut track,
                "Evidence integrity mismatch detected after finalization",
            );
            self.persistence.save_track(&track)?;
        }
        self.detail_from_record(track, false)
    }
}
