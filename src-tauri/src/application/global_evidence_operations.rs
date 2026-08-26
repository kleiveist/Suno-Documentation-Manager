use super::*;

impl WorkspaceApp {
    pub fn global_evidence(&self) -> Result<Vec<GlobalEvidenceItem>> {
        self.persistence.global_evidence()
    }

    pub fn register_global_evidence(
        &self,
        role: EvidenceRole,
        source: &Path,
        coverage_start: Option<String>,
        coverage_end: Option<String>,
    ) -> Result<GlobalEvidenceItem> {
        if role == EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Register the Suno terms PDF with the dedicated global importer.".into(),
            ));
        }
        if role == EvidenceRole::SubscriptionPayment {
            let start = coverage_start.as_deref().ok_or_else(|| {
                AppError::Validation("Subscription coverage start is required.".into())
            })?;
            let end = coverage_end.as_deref().ok_or_else(|| {
                AppError::Validation("Subscription coverage end is required.".into())
            })?;
            validate_date_range("Subscription coverage", start, end)?;
        } else {
            if let Some(start) = coverage_start.as_deref() {
                validate_optional_date("Evidence coverage start", start)?;
            }
            if let Some(end) = coverage_end.as_deref() {
                validate_optional_date("Evidence coverage end", end)?;
            }
        }
        let mut item = evidence::register_global(&self.root, role, source)?;
        let mut metadata =
            evidence::capture_automatic_metadata(source, EvidenceMetadata::default());
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        item.evidence.metadata = metadata;
        item.evidence.coverage_start = coverage_start;
        item.evidence.coverage_end = coverage_end;
        if let Err(error) = self.persistence.save_global_evidence(&item) {
            if let Ok(path) =
                contained_path(&self.root, Path::new(&item.evidence.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        Ok(item)
    }

    pub fn register_global_terms_evidence(
        &self,
        source: &Path,
        mut metadata: EvidenceMetadata,
    ) -> Result<GlobalEvidenceItem> {
        validate_evidence_metadata(&EvidenceRole::SunoTermsRights, &metadata)?;
        let mut item =
            evidence::register_global(&self.root, EvidenceRole::SunoTermsRights, source)?;
        metadata = evidence::capture_automatic_metadata(source, metadata);
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        item.evidence.metadata = metadata;
        if let Err(error) = self.persistence.save_global_evidence(&item) {
            if let Ok(path) =
                contained_path(&self.root, Path::new(&item.evidence.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }

        for track in self.persistence.tracks()?.into_iter().filter(|track| {
            !is_hidden_workspace_path(Path::new(&track.relative_path))
                && !matches!(
                    track.status,
                    TrackStatus::Finalized | TrackStatus::Superseded
                )
        }) {
            self.attach_global_evidence(&track.id, &item.evidence.id)?;
        }
        Ok(item)
    }

    pub fn update_global_terms_evidence_metadata(
        &self,
        evidence_id: &str,
        supplied: EvidenceMetadata,
    ) -> Result<GlobalEvidenceItem> {
        let mut global = self.persistence.global_evidence_item(evidence_id)?;
        if global.evidence.role != EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Only Suno terms/rights evidence has editable descriptive metadata here.".into(),
            ));
        }
        apply_descriptive_evidence_metadata(&mut global.evidence.metadata, &supplied);
        validate_evidence_metadata(&EvidenceRole::SunoTermsRights, &global.evidence.metadata)?;

        let mut copies = Vec::new();
        for mut track in self.persistence.tracks()?.into_iter().filter(|track| {
            !is_hidden_workspace_path(Path::new(&track.relative_path))
                && !matches!(
                    track.status,
                    TrackStatus::Finalized | TrackStatus::Superseded
                )
        }) {
            let Some(mut copy) = self
                .persistence
                .evidence(&track.id)?
                .into_iter()
                .find(|item| {
                    item.role == EvidenceRole::SunoTermsRights
                        && item.provenance == EvidenceProvenance::GlobalCopy
                        && item.source_global_evidence_id.as_deref() == Some(evidence_id)
                })
            else {
                continue;
            };
            copy.metadata = global.evidence.metadata.clone();
            mark_content_changed(&mut track);
            track.status = TrackStatus::Active;
            track.updated_at = now();
            copies.push((track, copy));
        }
        self.persistence
            .save_global_evidence_and_copies(&global, &copies)?;
        Ok(global)
    }

    pub fn register_global_evidence_for_billing_cycle(
        &self,
        role: EvidenceRole,
        source: &Path,
        coverage_start: &str,
        billing_cycle: SubscriptionBillingCycle,
    ) -> Result<GlobalEvidenceItem> {
        if role != EvidenceRole::SubscriptionPayment {
            return Err(AppError::Validation(
                "A billing cycle can only be used for subscription/payment evidence.".into(),
            ));
        }
        let coverage_end = subscription_coverage_end(coverage_start, billing_cycle)?;
        self.register_global_evidence(
            role,
            source,
            Some(coverage_start.to_owned()),
            Some(coverage_end),
        )
    }

    pub fn remove_global_evidence(&self, evidence_id: &str) -> Result<()> {
        let item = self.persistence.global_evidence_item(evidence_id)?;
        let path = contained_path(&self.root, Path::new(&item.evidence.relative_path), false)?;
        let mut removal_dir = None;
        let mut archived = None;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let removal_relative =
                    PathBuf::from(".suno-doc/removals").join(Uuid::new_v4().to_string());
                let directory = ensure_contained_directory(&self.root, &removal_relative)?;
                let archive_path = directory.join(
                    path.file_name()
                        .ok_or_else(|| AppError::Data("Evidence path has no file name.".into()))?,
                );
                fs::rename(&path, &archive_path).map_err(|error| AppError::io(&path, error))?;
                removal_dir = Some(directory);
                archived = Some(archive_path);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::Symlink(path.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::io(&path, error)),
        }
        if let Err(error) = self.persistence.remove_global_evidence(evidence_id) {
            return Err(match archived.as_deref() {
                Some(archive_path) => rollback_removed_file(archive_path, &path, error),
                None => error,
            });
        }
        if let Some(archive_path) = archived.as_deref() {
            let _ = fs::remove_file(archive_path);
        }
        if let Some(directory) = removal_dir.as_deref() {
            let _ = fs::remove_dir(directory);
        }
        Ok(())
    }

    pub fn attach_global_evidence(&self, id: &str, evidence_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let global = self.persistence.global_evidence_item(evidence_id)?;
        if self.persistence.evidence(id)?.iter().any(|item| {
            item.source_global_evidence_id.as_deref() == Some(global.evidence.id.as_str())
        }) {
            return self.detail_from_record(track, false);
        }

        Self::validate_global_evidence_attachment(&track, &global)?;
        let source = contained_path(&self.root, Path::new(&global.evidence.relative_path), true)?;
        let track_root = self.track_root(&track)?;
        let item = evidence::portable_global_copy(&track_root, &global, &source)?;
        if let Err(error) = self.persistence.save_evidence(id, &item) {
            if let Ok(path) = contained_path(&track_root, Path::new(&item.relative_path), true) {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        let all_evidence = self.persistence.evidence(id)?;
        reconcile_evidence_derived_fields(&mut track, &all_evidence);
        mark_content_changed(&mut track);
        if global.evidence.role == EvidenceRole::SunoTermsRights {
            track.fields.suno_terms_evidence_not_available = Some(false);
        }
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    fn validate_global_evidence_attachment(
        track: &TrackRecord,
        global: &GlobalEvidenceItem,
    ) -> Result<()> {
        if global.evidence.role == EvidenceRole::SubscriptionPayment {
            validate_required_production_range(track)?;
            let start = global.evidence.coverage_start.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage start.".into())
            })?;
            let end = global.evidence.coverage_end.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage end.".into())
            })?;
            validate_date_range("Subscription coverage", start, end)?;
            let overlaps_production = start <= track.fields.production_end_date.as_str()
                && end >= track.fields.production_start_date.as_str();
            let covers_generation = !track.fields.suno_final_generation_date.trim().is_empty()
                && start <= track.fields.suno_final_generation_date.as_str()
                && end >= track.fields.suno_final_generation_date.as_str();
            if !overlaps_production && !covers_generation {
                return Err(AppError::Validation(
                    "The selected subscription evidence neither overlaps the recorded production period nor covers the recorded final-generation date."
                        .into(),
                ));
            }
        } else if global.evidence.role != EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Only subscription/payment or Suno terms/rights evidence can be attached to a track."
                    .into(),
            ));
        }
        Ok(())
    }
}
