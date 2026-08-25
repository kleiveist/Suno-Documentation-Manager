use super::*;

impl WorkspaceApp {
    pub(super) fn verified_evidence(&self, track: &TrackRecord) -> Result<Vec<EvidenceItem>> {
        let root = self.track_root(track)?;
        let mut result = Vec::new();
        for item in self.persistence.evidence(&track.id)? {
            let previous_verification = (
                item.sha256.clone(),
                item.size_bytes,
                item.verified,
                item.verification_error.clone(),
            );
            let verified = evidence::inspect(&root, item)?;
            let current_verification = (
                verified.sha256.clone(),
                verified.size_bytes,
                verified.verified,
                verified.verification_error.clone(),
            );
            // A read-only load must not reserialize legacy metadata_json just
            // because additive serde-default fields exist in a newer build.
            if current_verification != previous_verification {
                self.persistence.save_evidence(&track.id, &verified)?;
            }
            result.push(verified);
        }
        Ok(result)
    }

    /// Explicit revision creation is the sole backfill path for evidence that
    /// predates automatic WAV inspection. Normal loading intentionally never
    /// enriches historical/finalized records.
    pub(super) fn prepare_revision_suno_analysis(
        &self,
        mut track: TrackRecord,
    ) -> Result<(TrackRecord, Vec<EvidenceItem>)> {
        let root = self.track_root(&track)?;
        let mut evidence_items = self.persistence.evidence(&track.id)?;
        let mut changed_items = Vec::new();
        for item in evidence_items
            .iter_mut()
            .filter(|item| item.role == EvidenceRole::SunoFinalExport)
        {
            let path = contained_path(&root, Path::new(&item.relative_path), false)?;
            let bytes_match = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    item.sha256.as_deref().is_some_and(|expected| {
                        sha256_file(&path).is_ok_and(|actual| actual == expected)
                    })
                }
                _ => false,
            };
            if !item.verified || item.verification_error.is_some() || !bytes_match {
                // Keep the persisted snapshot untouched, but exclude stale
                // evidence from facts prepared for the mutable revision.
                item.verified = false;
                continue;
            }
            let captured = evidence::capture_automatic_metadata(&path, item.metadata.clone());
            if validate_evidence_metadata(&item.role, &captured).is_err() {
                item.verified = false;
                continue;
            }
            if captured != item.metadata {
                item.metadata = captured;
                changed_items.push(item.clone());
            }
        }
        reconcile_evidence_derived_fields(&mut track, &evidence_items);
        Ok((track, changed_items))
    }

    pub(super) fn validation_for(&self, mut track: TrackRecord) -> Result<ValidationResult> {
        validate_profile(&track.profile_snapshot, true)?;
        validate_track_fields(&track.fields)?;
        validate_required_production_range(&track)?;
        let root = self.track_root(&track)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let screening_artifact_error = self
            .ensure_current_audio_screening_artifacts(&track, &evidence)
            .err();
        let first = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        track.documents.current = documents::is_current(
            &root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &first.steps,
        )?;
        track.integrity = if root.join(integrity::HASH_FILE).is_file() {
            integrity::verify(&root)?
        } else {
            IntegrityState::default()
        };
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        self.persistence.save_track(&track)?;
        let blocking_items = Self::validation_blocking_items(
            &track,
            &evaluation,
            &deviations,
            screening_artifact_error,
        )?;
        Ok(ValidationResult {
            valid: evaluation.missing.is_empty() && blocking_items.is_empty(),
            missing_items: evaluation.missing,
            blocking_items,
        })
    }

    fn validation_blocking_items(
        track: &TrackRecord,
        evaluation: &workflow::WorkflowEvaluation,
        deviations: &[BlockingDeviation],
        screening_artifact_error: Option<AppError>,
    ) -> Result<Vec<String>> {
        let mut blocking_items = Vec::new();
        if let Some(message) = workflow_version_mismatch(track)? {
            blocking_items.push(message);
        }
        if let Some(error) = screening_artifact_error {
            blocking_items.push(format!("Pre-release audio screening: {error}"));
        }
        for step in &evaluation.steps {
            if matches!(
                step.status,
                StepStatus::Fail | StepStatus::Blocked | StepStatus::NotVerified
            ) {
                blocking_items.push(format!("{}: {:?}", step.id, step.status));
            }
            if step.status == StepStatus::NotApplicable
                && step
                    .na_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                blocking_items.push(format!("{}: N/A requires a reason", step.id));
            }
        }
        for deviation in deviations {
            if deviation.blocking && !deviation.resolved {
                blocking_items.push(format!("Deviation: {}", deviation.description));
            }
        }
        for mismatch in &track.integrity.mismatch_files {
            blocking_items.push(format!("Integrity mismatch: {mismatch}"));
        }
        Ok(blocking_items)
    }

    pub(super) fn detail_from_record(
        &self,
        mut track: TrackRecord,
        inspect_finalized: bool,
    ) -> Result<TrackDetail> {
        let immutable_snapshot = matches!(
            track.status,
            TrackStatus::Finalized | TrackStatus::Superseded
        );
        let stored_track_state = serde_json::to_vec(&track)?;
        if !immutable_snapshot && self.migrate_legacy_release_evidence(&track)? {
            mark_content_changed(&mut track);
            track.updated_at = now();
        }
        let root = self.track_root(&track)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let first = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        if !immutable_snapshot {
            track.documents.current = documents::is_current(
                &root,
                &track,
                &track.profile_snapshot,
                &evidence,
                &first.steps,
            )?;
        }
        let inspected_integrity =
            Self::inspect_detail_integrity(&root, &evidence, &track.integrity);
        Self::apply_detail_integrity(
            &mut track,
            &root,
            &evidence,
            &inspected_integrity,
            immutable_snapshot,
            inspect_finalized,
        );
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let progress = workflow::progress(&track, &track.profile_snapshot, &evidence, &deviations)?;
        Self::update_detail_status(&mut track, &evaluation);
        // Avoid normalizing old JSON records on a read-only load merely due to
        // new serde-default fields. Persist only a real semantic state change.
        if serde_json::to_vec(&track)? != stored_track_state {
            self.persistence.save_track(&track)?;
        }
        let cover_evidence_id = final_artwork_evidence_id(&evidence);
        let automation = workflow::automation_summary(&track, &evidence);
        let (external_timestamps, finalization_anchors, external_timestamp_summary) =
            self.external_timestamp_context(&track)?;
        Ok(TrackDetail {
            id: track.id.clone(),
            title: track.fields.title.clone(),
            relative_path: track.relative_path.clone(),
            status: track.status.clone(),
            updated_at: track.updated_at.clone(),
            progress,
            missing_count: evaluation.missing.len() as u32,
            certificate_valid: Some(track.certificate.valid),
            legacy: Some(track.legacy),
            cover_evidence_id,
            library: track.library.clone(),
            workflow_id: track.workflow_id.clone(),
            workflow_version: track.workflow_version.clone(),
            profile_snapshot: track.profile_snapshot.clone(),
            automation,
            fields: track.fields.clone(),
            audio_screening: AudioScreeningSummary::from(&track.audio_screening),
            steps: evaluation.steps,
            evidence,
            external_timestamps,
            external_timestamp_summary,
            finalization_anchors,
            documents: track.documents.clone(),
            integrity: track.integrity.clone(),
            certificate: track.certificate.clone(),
            blocking_deviations: deviations,
            missing_items: evaluation.missing,
        })
    }

    fn inspect_detail_integrity(
        root: &Path,
        evidence: &[EvidenceItem],
        current: &IntegrityState,
    ) -> IntegrityState {
        let hash_file_exists = root.join(integrity::HASH_FILE).is_file();
        let contains_large_evidence = evidence
            .iter()
            .any(|item| item.size_bytes > evidence::AUTOMATIC_HASH_LIMIT_BYTES);
        if hash_file_exists && !contains_large_evidence {
            integrity::verify(root).unwrap_or_else(|_| IntegrityState {
                generated: true,
                verified: false,
                file_count: 0,
                verified_count: 0,
                generated_at: None,
                verified_at: Some(now()),
                mismatch_files: vec![
                    "03_DOCUMENTATION/SHA256SUMS.txt (invalid or unreadable)".into()
                ],
            })
        } else if !hash_file_exists {
            IntegrityState::default()
        } else {
            current.clone()
        }
    }

    fn apply_detail_integrity(
        track: &mut TrackRecord,
        root: &Path,
        evidence: &[EvidenceItem],
        inspected_integrity: &IntegrityState,
        immutable_snapshot: bool,
        inspect_finalized: bool,
    ) {
        if !immutable_snapshot || !inspected_integrity.verified {
            track.integrity = inspected_integrity.clone();
        }
        if inspect_finalized && track.status == TrackStatus::Finalized {
            let certificate_valid = certificate::verify(root).is_ok();
            let evidence_valid = evidence.iter().all(|item| {
                item.verified && item.sha256.is_some() && item.verification_error.is_none()
            });
            if (!inspected_integrity.verified || !certificate_valid || !evidence_valid)
                && track.certificate.valid
            {
                invalidate_state(track, "Documentation changed after finalization");
            }
        }
    }

    fn update_detail_status(track: &mut TrackRecord, evaluation: &workflow::WorkflowEvaluation) {
        if track.status == TrackStatus::Finalized || track.status == TrackStatus::Superseded {
            return;
        }
        track.status = if evaluation.missing.is_empty()
            && evaluation
                .steps
                .iter()
                .all(|step| matches!(step.status, StepStatus::Pass | StepStatus::NotApplicable))
        {
            TrackStatus::Ready
        } else if track.status == TrackStatus::Draft {
            TrackStatus::Draft
        } else {
            TrackStatus::Active
        };
    }

    pub(super) fn detail_from_stored_record(&self, track: TrackRecord) -> Result<TrackDetail> {
        let evidence = self.persistence.evidence(&track.id)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let progress = workflow::progress(&track, &track.profile_snapshot, &evidence, &deviations)?;
        let cover_evidence_id = final_artwork_evidence_id(&evidence);
        let automation = workflow::automation_summary(&track, &evidence);
        let (external_timestamps, finalization_anchors, external_timestamp_summary) =
            self.external_timestamp_context(&track)?;
        Ok(TrackDetail {
            id: track.id.clone(),
            title: track.fields.title.clone(),
            relative_path: track.relative_path.clone(),
            status: track.status.clone(),
            updated_at: track.updated_at.clone(),
            progress,
            missing_count: evaluation.missing.len() as u32,
            certificate_valid: Some(track.certificate.valid),
            legacy: Some(track.legacy),
            cover_evidence_id,
            library: track.library.clone(),
            workflow_id: track.workflow_id.clone(),
            workflow_version: track.workflow_version.clone(),
            profile_snapshot: track.profile_snapshot.clone(),
            automation,
            fields: track.fields.clone(),
            audio_screening: AudioScreeningSummary::from(&track.audio_screening),
            steps: evaluation.steps,
            evidence,
            external_timestamps,
            external_timestamp_summary,
            finalization_anchors,
            documents: track.documents.clone(),
            integrity: track.integrity.clone(),
            certificate: track.certificate.clone(),
            blocking_deviations: deviations,
            missing_items: evaluation.missing,
        })
    }

    pub(super) fn external_timestamp_context(
        &self,
        track: &TrackRecord,
    ) -> Result<(
        Vec<ExternalTimestampRecord>,
        Vec<FinalizationAnchor>,
        ExternalTimestampSummary,
    )> {
        let root = self.track_root(track)?;
        let mut records = self.persistence.external_timestamps(&track.id)?;
        for record in &mut records {
            match external_timestamp::verify_published_record(&root, record) {
                Ok(()) => {
                    record.integrity_verified = true;
                    record.integrity_issues.clear();
                }
                Err(error) => {
                    record.integrity_verified = false;
                    record.integrity_issues = vec![error.to_string()];
                }
            }
        }
        if track.status != TrackStatus::Finalized || !track.certificate.valid {
            return Ok((records, Vec::new(), ExternalTimestampSummary::default()));
        }
        if let Some(certificate_id) = track.certificate.certificate_id.as_deref() {
            // A track can retain archived addenda from prior revisions. The
            // live finalized view must never present those as evidence for the
            // current certificate snapshot.
            records.retain(|record| record.certificate_id == certificate_id);
        }
        let summary = self.timestamp_summary_for_record(track, &records)?;
        let anchors = if certificate::verify(&root).is_ok() {
            external_timestamp::finalization_anchors(&root).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok((records, anchors, summary))
    }

    pub(super) fn timestamp_summary_for_record(
        &self,
        track: &TrackRecord,
        records: &[ExternalTimestampRecord],
    ) -> Result<ExternalTimestampSummary> {
        let Some(certificate_id) = track.certificate.certificate_id.as_deref() else {
            return Ok(ExternalTimestampSummary::default());
        };
        let stored_summary = self
            .persistence
            .timestamp_attachment_summary(&track.id, certificate_id)?;
        if let Some(record) = records
            .iter()
            .rev()
            .find(|record| currently_verified_provider_timestamp(record))
        {
            return Ok(ExternalTimestampSummary {
                status: ExternalTimestampStatus::Verified,
                message: record
                    .provider_metadata
                    .as_ref()
                    .map(|metadata| metadata.verification_message.clone())
                    .filter(|message| !message.trim().is_empty())
                    .unwrap_or_else(|| {
                        "The RFC 3161 response and its current sidecar integrity are technically verified."
                            .into()
                    }),
                provider: record.provider.clone(),
                record_id: Some(record.id.clone()),
                updated_at: Some(record.imported_at.clone()),
            });
        }
        if let Some(summary) = stored_summary
            .as_ref()
            .filter(|summary| summary.status != ExternalTimestampStatus::Verified)
        {
            return Ok(summary.clone());
        }
        let Some(record) = records
            .iter()
            .rev()
            .find(|record| record.certificate_id == certificate_id)
        else {
            return Ok(if let Some(summary) = stored_summary {
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::VerificationFailed,
                    message: "A stored VERIFIED timestamp summary has no currently verifiable published sidecar."
                        .into(),
                    provider: summary.provider,
                    record_id: summary.record_id,
                    updated_at: summary.updated_at,
                }
            } else {
                ExternalTimestampSummary::default()
            });
        };
        if !record.integrity_verified {
            return Ok(ExternalTimestampSummary {
                status: ExternalTimestampStatus::VerificationFailed,
                message: "External timestamp evidence is present, but its current sidecar or referenced-anchor integrity verification failed."
                    .into(),
                provider: record.provider.clone(),
                record_id: Some(record.id.clone()),
                updated_at: Some(record.imported_at.clone()),
            });
        }
        // Historical sidecars only record addendum-file integrity, not a
        // provider cryptographic result. A missing summary therefore never
        // promotes legacy or recovered evidence to VERIFIED.
        let status = record
            .provider_metadata
            .as_ref()
            .map(|metadata| match metadata.verification_result {
                ExternalTimestampStatus::Verified => ExternalTimestampStatus::VerificationFailed,
                status => status,
            })
            .unwrap_or(ExternalTimestampStatus::Attached);
        Ok(ExternalTimestampSummary {
            status,
            message: if record.provider_metadata.as_ref().is_some_and(|metadata| {
                metadata.verification_result == ExternalTimestampStatus::Verified
            }) {
                "The immutable record claims VERIFIED, but the complete current RFC 3161 verification predicate is not satisfied."
                    .into()
            } else if record.provider_metadata.is_some() {
                "External timestamp evidence is attached; its immutable provider verification status is shown without promotion."
                    .into()
            } else {
                "Legacy manually recorded timestamp evidence is attached; it has not been automatically promoted to provider-verified evidence."
                    .into()
            },
            provider: record.provider.clone(),
            record_id: Some(record.id.clone()),
            updated_at: Some(record.imported_at.clone()),
        })
    }
}
