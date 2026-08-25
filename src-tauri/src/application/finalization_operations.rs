use super::*;

struct FinalizationPlan {
    track: TrackRecord,
    certificate_track: TrackRecord,
    render_options: CertificateRenderOptions,
    evidence: Vec<EvidenceItem>,
    deviations: Vec<BlockingDeviation>,
    evaluation: workflow::WorkflowEvaluation,
    finalized_at: String,
    certificate_id: String,
    snapshot_id: String,
    transaction_id: String,
    track_root: PathBuf,
    live_certificate: PathBuf,
    live_pdfs: [PathBuf; 2],
    certificate_staging: PathBuf,
    marker_path: PathBuf,
    marker_bytes: Vec<u8>,
}

struct FinalizationTimestampCapture {
    settings: TimestampSettings,
    secret: Option<String>,
    setup_error: Option<String>,
    settings_unavailable: bool,
    snapshot: crate::model::FinalizationTimestampSnapshot,
    response: Option<external_timestamp::ProviderTimestampResponse>,
    anchor_sha256: String,
}

impl FinalizationTimestampCapture {
    fn resolve(
        &mut self,
        digest: &str,
        manifest_bytes: &[u8],
    ) -> crate::model::FinalizationTimestampSnapshot {
        let attempt = if let Some(message) = self.setup_error.as_deref() {
            external_timestamp::FinalizationTimestampAttempt {
                snapshot: crate::model::FinalizationTimestampSnapshot {
                    provider: if self.settings_unavailable {
                        String::new()
                    } else {
                        external_timestamp::provider_display_name(&self.settings)
                    },
                    provider_configuration_status:
                        TimestampProviderConfigurationStatus::ProviderError,
                    provider_configuration_message: message.to_owned(),
                    automatic_request_enabled: self.settings.enabled
                        && self.settings.auto_after_finalization,
                    technical_status: ExternalTimestampStatus::NotRecorded,
                    technical_message: "No automatic external timestamp was requested because its local provider configuration was unavailable."
                        .into(),
                    ..Default::default()
                },
                response: None,
            }
        } else {
            external_timestamp::attempt_finalization_timestamp(
                &self.settings,
                self.secret.as_deref(),
                digest,
                manifest_bytes,
            )
        };
        self.anchor_sha256 = digest.to_owned();
        self.snapshot = attempt.snapshot.clone();
        self.response = attempt.response;
        attempt.snapshot
    }
}

impl WorkspaceApp {
    pub fn validate_track(&self, id: &str) -> Result<ValidationResult> {
        let track = self.persistence.track(id)?;
        self.validation_for(track)
    }

    #[cfg(test)]
    pub fn finalize_track(&self, id: &str) -> Result<ActionResult> {
        self.finalize_track_with_options(id, FinalizeOptions::default())
    }

    #[cfg(test)]
    pub fn finalize_track_with_options(
        &self,
        id: &str,
        options: FinalizeOptions,
    ) -> Result<ActionResult> {
        self.finalize_track_with_options_and_progress(id, options, &mut |_| {})
    }

    #[cfg(test)]
    pub fn finalize_track_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_with_options_and_progress(id, FinalizeOptions::default(), on_progress)
    }

    pub fn finalize_track_with_options_and_progress(
        &self,
        id: &str,
        options: FinalizeOptions,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_impl_with_options(
            id,
            options,
            #[cfg(test)]
            None,
            on_progress,
        )
    }

    #[cfg(test)]
    pub(super) fn finalize_track_impl(
        &self,
        id: &str,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_impl_with_options(
            id,
            FinalizeOptions::default(),
            #[cfg(test)]
            failure,
            on_progress,
        )
    }

    pub(super) fn finalize_track_impl_with_options(
        &self,
        id: &str,
        _options: FinalizeOptions,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        on_progress(OperationProgress {
            stage: "validating_finalization_gate".into(),
            ..OperationProgress::default()
        });
        self.ensure_finalization_gate(id)?;
        on_progress(OperationProgress {
            stage: "collecting_final_snapshot".into(),
            ..OperationProgress::default()
        });
        let mut plan = self.prepare_finalization_plan(id)?;
        on_progress(OperationProgress {
            stage: "writing_finalization_marker".into(),
            ..OperationProgress::default()
        });
        atomic_write_new(&plan.marker_path, &plan.marker_bytes)?;
        on_progress(OperationProgress {
            stage: "generating_certificate".into(),
            processed_files: plan.evidence.len() as u32,
            total_files: plan.evidence.len() as u32,
            ..OperationProgress::default()
        });
        let mut timestamp = self.finalization_timestamp_capture();
        let publication = self.generate_finalization_certificate(
            &plan,
            &mut timestamp,
            #[cfg(test)]
            failure,
        );
        if let Err(error) = publication {
            self.cleanup_failed_certificate_generation(&plan);
            return Err(error);
        }
        let integrity = self.verify_finalization_publication(&plan, on_progress)?;
        let initial_timestamp =
            self.archive_initial_finalization_timestamp(&plan, &mut timestamp)?;
        Self::apply_finalized_state(&mut plan, integrity);
        self.commit_finalization_snapshot(
            &plan,
            initial_timestamp.as_ref(),
            #[cfg(test)]
            failure,
            on_progress,
        )?;
        // The database commit is authoritative. A stale marker is harmless and is
        // removed during the next workspace recovery if this best-effort cleanup fails.
        let _ = fs::remove_file(&plan.marker_path);
        let _ = self.save_timestamp_attachment_summary(
            &plan.track,
            ExternalTimestampSummary {
                status: timestamp.snapshot.technical_status,
                message: timestamp.snapshot.technical_message.clone(),
                provider: timestamp.snapshot.provider.clone(),
                record_id: initial_timestamp.as_ref().map(|record| record.id.clone()),
                updated_at: Some(now()),
            },
        );
        let detail = self.detail_from_record(plan.track, false)?;
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: detail.integrity.verified_count,
            total_files: detail.integrity.file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!(
                "Documentation finalized and certificate set verified. German and English technical documentation PDFs created: {}, {}",
                certificate::PDF_FILE_DE,
                certificate::PDF_FILE_EN
            ),
            track: Some(detail),
        })
    }

    fn ensure_finalization_gate(&self, id: &str) -> Result<()> {
        let track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
        let validation = self.validation_for(track)?;
        if !validation.valid {
            return Err(AppError::Validation(
                validation
                    .missing_items
                    .iter()
                    .chain(validation.blocking_items.iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        Ok(())
    }

    fn prepare_finalization_plan(&self, id: &str) -> Result<FinalizationPlan> {
        // Re-read all state after validation so the certificate uses exactly the gate input.
        let track = self.persistence.track(id)?;
        let certificate_track = self.certificate_track_snapshot(&track)?;
        // Certificate language is intentionally resolved from the current
        // workspace setting rather than the editable track profile snapshot.
        let render_options = CertificateRenderOptions {
            language: self.profile()?.certificate_language,
            bilingual: true,
        };
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
        self.ensure_current_audio_screening_artifacts(&track, &evidence)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let finalized_at = now();
        let certificate_id = format!("SDM-{}", Uuid::new_v4());
        let snapshot_id = Uuid::new_v4().to_string();
        let transaction_id = Uuid::new_v4().to_string();
        let track_root = self.track_root(&track)?;
        let live_certificate =
            contained_path(&track_root, Path::new(certificate::CERTIFICATE_DIR), false)?;
        let live_pdfs = [
            contained_path(&track_root, Path::new(certificate::PDF_FILE), false)?,
            contained_path(&track_root, Path::new(certificate::PDF_FILE_DE), false)?,
        ];
        if !directory_is_empty_or_missing(&live_certificate)? {
            return Err(AppError::Collision(
                "The certificate directory already contains files. Preserve or archive them before finalizing."
                    .into(),
            ));
        }
        if let Some(existing_pdf) = live_pdfs.iter().find(|path| path.exists()) {
            return Err(AppError::Collision(format!(
                "The technical documentation PDF already exists: {}",
                existing_pdf.display()
            )));
        }
        let certificate_staging = contained_path(
            &track_root,
            &PathBuf::from(".archive/certificate-staging").join(&transaction_id),
            false,
        )?;
        let marker_path = contained_path(
            &track_root,
            Path::new(".archive/finalization-in-progress.json"),
            false,
        )?;
        let marker_bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "transaction_id": transaction_id,
            "track_id": track.id,
            "certificate_id": certificate_id,
            "finalization_snapshot_id": snapshot_id,
            "certificate_render_options": render_options,
            "started_at": now(),
        }))?;
        Ok(FinalizationPlan {
            track,
            certificate_track,
            render_options,
            evidence,
            deviations,
            evaluation,
            finalized_at,
            certificate_id,
            snapshot_id,
            transaction_id,
            track_root,
            live_certificate,
            live_pdfs,
            certificate_staging,
            marker_path,
            marker_bytes,
        })
    }

    fn certificate_track_snapshot(&self, track: &TrackRecord) -> Result<TrackRecord> {
        // Freeze the optional provider's actual configuration state in the
        // immutable certificate snapshot.
        let mut certificate_track = track.clone();
        let settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        let (provider_status, provider_message) =
            audio_screening::provider_configuration_status(&settings, credentials_present);
        certificate_track
            .audio_screening
            .external
            .configured_at_snapshot = Some(matches!(
            provider_status,
            AudioScreeningProviderStatus::Ready
        ));
        if certificate_track.audio_screening.external.status == AudioScreeningStatus::NotRun
            && !matches!(provider_status, AudioScreeningProviderStatus::Ready)
        {
            certificate_track.audio_screening.external.status = match provider_status {
                AudioScreeningProviderStatus::Disabled
                | AudioScreeningProviderStatus::NotConfigured => {
                    AudioScreeningStatus::SkippedNotConfigured
                }
                AudioScreeningProviderStatus::ConfigurationInvalid => {
                    AudioScreeningStatus::ConfigurationInvalid
                }
                AudioScreeningProviderStatus::AuthenticationFailed => {
                    AudioScreeningStatus::AuthenticationFailed
                }
                AudioScreeningProviderStatus::ProviderUnavailable => {
                    AudioScreeningStatus::ProviderUnavailable
                }
                AudioScreeningProviderStatus::Ready => AudioScreeningStatus::NotRun,
            };
            certificate_track.audio_screening.external.message = provider_message;
        }
        Ok(certificate_track)
    }

    fn finalization_timestamp_capture(&self) -> FinalizationTimestampCapture {
        let (settings, settings_error) = match self.persistence.timestamp_settings() {
            Ok(settings) => (settings, None),
            Err(_) => (
                TimestampSettings::default(),
                Some(
                    "The local timestamp-provider configuration could not be read for this finalization."
                        .to_owned(),
                ),
            ),
        };
        let secret_required = settings.provider
            == crate::model::TimestampProviderKind::CustomRfc3161
            && matches!(
                settings.custom.authentication_mode,
                crate::model::TimestampAuthenticationMode::Basic
                    | crate::model::TimestampAuthenticationMode::BearerToken
                    | crate::model::TimestampAuthenticationMode::ApiKey
            );
        let (secret, secret_error) = if settings_error.is_none() && secret_required {
            match self.persistence.timestamp_secret() {
                Ok(secret) => (secret, None),
                Err(_) => (
                    None,
                    Some(
                        "The local timestamp-provider credential could not be read for this finalization."
                            .to_owned(),
                    ),
                ),
            }
        } else {
            (None, None)
        };
        FinalizationTimestampCapture {
            settings,
            secret,
            setup_error: settings_error.clone().or(secret_error),
            settings_unavailable: settings_error.is_some(),
            snapshot: crate::model::FinalizationTimestampSnapshot::default(),
            response: None,
            anchor_sha256: String::new(),
        }
    }

    fn generate_finalization_certificate(
        &self,
        plan: &FinalizationPlan,
        timestamp: &mut FinalizationTimestampCapture,
        #[cfg(test)] failure: Option<FinalizationFailure>,
    ) -> Result<()> {
        let mut resolver =
            |digest: &str, manifest_bytes: &[u8]| timestamp.resolve(digest, manifest_bytes);
        let generation_input = certificate::GenerationInput {
            track_root: &plan.track_root,
            track: &plan.certificate_track,
            profile: &plan.certificate_track.profile_snapshot,
            steps: &plan.evaluation.steps,
            evidence: &plan.evidence,
            deviations: &plan.deviations,
            certificate_id: &plan.certificate_id,
            finalized_at: &plan.finalized_at,
            transaction_id: &plan.transaction_id,
            render_options: plan.render_options,
        };
        #[cfg(test)]
        {
            if let Some(certificate_failure) =
                failure.and_then(FinalizationFailure::certificate_failure)
            {
                return certificate::generate_with_failure(generation_input, certificate_failure);
            }
        }
        certificate::generate_with_finalization_timestamp(generation_input, &mut resolver)
    }

    fn cleanup_failed_certificate_generation(&self, plan: &FinalizationPlan) {
        if directory_is_empty_or_missing(&plan.live_certificate).unwrap_or(false)
            && !plan.live_pdfs.iter().any(|path| path.exists())
            && !plan.certificate_staging.exists()
        {
            let _ = fs::remove_file(&plan.marker_path);
        }
    }

    fn verify_finalization_publication(
        &self,
        plan: &FinalizationPlan,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<IntegrityState> {
        on_progress(OperationProgress {
            stage: "verifying_certificate".into(),
            ..OperationProgress::default()
        });
        if let Err(error) = certificate::verify(&plan.track_root) {
            return Err(self.rollback_failed_finalization(plan, error));
        }
        on_progress(OperationProgress {
            stage: "verifying_final_snapshot".into(),
            processed_files: 0,
            total_files: plan.track.integrity.file_count,
            ..OperationProgress::default()
        });
        match integrity::verify_with_progress(&plan.track_root, on_progress) {
            Ok(state) if state.verified => Ok(state),
            Ok(state) => Err(self.rollback_failed_finalization(
                plan,
                AppError::Validation(format!(
                    "Track files changed during finalization: {}",
                    state.mismatch_files.join(", ")
                )),
            )),
            Err(error) => Err(self.rollback_failed_finalization(plan, error)),
        }
    }

    fn archive_initial_finalization_timestamp(
        &self,
        plan: &FinalizationPlan,
        timestamp: &mut FinalizationTimestampCapture,
    ) -> Result<Option<ExternalTimestampRecord>> {
        let Some(response) = timestamp.response.take() else {
            return Ok(None);
        };
        match self.archive_finalization_provider_response(
            &plan.track.id,
            &plan.track_root,
            &plan.certificate_id,
            &plan.snapshot_id,
            &timestamp.anchor_sha256,
            response,
        ) {
            Ok(record) => Ok(Some(record)),
            Err(error) => Err(self.rollback_failed_finalization(plan, error)),
        }
    }

    fn rollback_failed_finalization(&self, plan: &FinalizationPlan, error: AppError) -> AppError {
        let rolled_back = rollback_certificate_set(&plan.track_root, error);
        if rolled_back.complete {
            let _ = fs::remove_file(&plan.marker_path);
        }
        rolled_back.error
    }

    fn apply_finalized_state(plan: &mut FinalizationPlan, integrity: IntegrityState) {
        plan.track.integrity = integrity;
        plan.track.audio_screening.external.configured_at_snapshot = plan
            .certificate_track
            .audio_screening
            .external
            .configured_at_snapshot;
        plan.track.status = TrackStatus::Finalized;
        plan.track.certificate = CertificateState {
            valid: true,
            certificate_id: Some(plan.certificate_id.clone()),
            finalization_snapshot_id: Some(plan.snapshot_id.clone()),
            finalized_at: Some(plan.finalized_at.clone()),
            workflow_version: Some(plan.track.workflow_version.clone()),
            certificate_language: plan.render_options.language,
            bilingual: plan.render_options.bilingual,
            invalidated_at: None,
            invalidation_reason: None,
        };
        plan.track.updated_at = now();
    }

    fn commit_finalization_snapshot(
        &self,
        plan: &FinalizationPlan,
        initial_timestamp: Option<&ExternalTimestampRecord>,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<()> {
        on_progress(OperationProgress {
            stage: "saving_final_snapshot".into(),
            processed_files: plan.track.integrity.verified_count,
            total_files: plan.track.integrity.file_count,
            ..OperationProgress::default()
        });
        #[cfg(test)]
        let database_commit = if failure == Some(FinalizationFailure::DatabaseCommit) {
            Err(AppError::Data(
                "Injected finalization database commit failure.".into(),
            ))
        } else {
            self.persistence.save_track(&plan.track)
        };
        #[cfg(not(test))]
        let database_commit = self.persistence.save_track(&plan.track);
        if let Err(error) = database_commit {
            let error = self.rollback_initial_timestamp_after_commit_failure(
                plan,
                initial_timestamp,
                error,
            );
            return Err(self.rollback_failed_finalization(plan, error));
        }
        Ok(())
    }

    fn rollback_initial_timestamp_after_commit_failure(
        &self,
        plan: &FinalizationPlan,
        initial_timestamp: Option<&ExternalTimestampRecord>,
        error: AppError,
    ) -> AppError {
        let Some(record) = initial_timestamp else {
            return error;
        };
        match self.rollback_finalization_provider_response(
            &plan.track.id,
            &plan.track_root,
            record,
        ) {
            Ok(()) => error,
            Err(cleanup) => AppError::Data(format!(
                "Final snapshot database commit failed ({error}); initial timestamp rollback also failed ({cleanup})."
            )),
        }
    }

    pub(super) fn archive_finalization_provider_response(
        &self,
        track_id: &str,
        track_root: &Path,
        certificate_id: &str,
        finalization_snapshot_id: &str,
        expected_manifest_sha256: &str,
        response: external_timestamp::ProviderTimestampResponse,
    ) -> Result<ExternalTimestampRecord> {
        let anchor = external_timestamp::finalized_manifest_anchor(track_root)?;
        if !anchor.sha256.eq_ignore_ascii_case(expected_manifest_sha256) {
            return Err(AppError::Validation(
                "The manifest anchor changed between the timestamp request and evidence archival."
                    .into(),
            ));
        }
        let anchors_before = external_timestamp::finalization_anchors(track_root)?;
        let temporary_source =
            provider_response_staging_path(track_root, &response.evidence_extension)?;
        atomic_write_new(&temporary_source, &response.evidence_bytes)?;
        let staged = external_timestamp::stage_provider_response(
            track_root,
            certificate_id,
            finalization_snapshot_id,
            &anchor.sha256,
            &temporary_source,
            response,
        );
        let _ = fs::remove_file(&temporary_source);
        let staged = staged?;
        let record = staged.record.clone();

        if let Err(error) = self.persistence.save_external_timestamp(track_id, &record) {
            return Err(match external_timestamp::discard_staged(track_root, &staged) {
                Ok(()) => error,
                Err(cleanup) => AppError::Data(format!(
                    "Initial timestamp database registration failed ({error}); staging cleanup also failed ({cleanup})."
                )),
            });
        }

        let publication = (|| -> Result<()> {
            external_timestamp::publish(track_root, &staged)?;
            external_timestamp::verify_published_record(track_root, &record)?;
            certificate::verify(track_root)?;
            let integrity = integrity::verify(track_root)?;
            if !integrity.verified {
                return Err(AppError::Validation(
                    "Archiving the initial timestamp changed the finalized integrity set.".into(),
                ));
            }
            if external_timestamp::finalization_anchors(track_root)? != anchors_before {
                return Err(AppError::Validation(
                    "Archiving the initial timestamp changed a finalized anchor.".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = publication {
            let filesystem_cleanup =
                external_timestamp::remove_published_record(track_root, &record)
                    .and_then(|()| external_timestamp::discard_staged(track_root, &staged));
            if let Err(cleanup) = filesystem_cleanup {
                return Err(AppError::Data(format!(
                    "Initial timestamp publication failed ({error}); filesystem cleanup also failed ({cleanup}). The database record was retained for recovery."
                )));
            }
            if let Err(cleanup) = self
                .persistence
                .remove_external_timestamp(track_id, &record.id)
            {
                return Err(AppError::Data(format!(
                    "Initial timestamp publication failed ({error}); database rollback also failed ({cleanup})."
                )));
            }
            return Err(error);
        }
        Ok(record)
    }

    pub(super) fn rollback_finalization_provider_response(
        &self,
        track_id: &str,
        track_root: &Path,
        record: &ExternalTimestampRecord,
    ) -> Result<()> {
        external_timestamp::remove_published_record(track_root, record)?;
        self.persistence
            .remove_external_timestamp(track_id, &record.id)
    }
}
