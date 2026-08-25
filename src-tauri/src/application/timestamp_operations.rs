use super::*;

struct ConfiguredTimestampContext {
    track: TrackRecord,
    certificate_id: String,
    track_root: PathBuf,
    settings: TimestampSettings,
    secret: Option<String>,
    provider: String,
    configuration_status: TimestampProviderConfigurationStatus,
    configuration_message: String,
}

enum TimestampRequestOutcome {
    Complete(Box<TrackDetail>),
    Response(Box<ConfiguredTimestampResponse>),
}

struct ConfiguredTimestampResponse {
    anchor: FinalizationAnchor,
    response: external_timestamp::ProviderTimestampResponse,
}

struct ConfiguredTimestampPublication {
    staged: external_timestamp::StagedExternalTimestamp,
    record: ExternalTimestampRecord,
    response_status: ExternalTimestampStatus,
    response_message: String,
    response_provider: String,
    anchors_before: Vec<FinalizationAnchor>,
}

impl WorkspaceApp {
    #[cfg(test)]
    pub fn attach_external_timestamp_from(
        &self,
        id: &str,
        source: &Path,
        input: ExternalTimestampInput,
    ) -> Result<TrackDetail> {
        let track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized
            || !track.certificate.valid
            || track.certificate.certificate_id.is_none()
        {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached to a valid technically finalized snapshot."
                    .into(),
            ));
        }
        let track_root = self.track_root(&track)?;
        certificate::verify(&track_root)?;
        let integrity = integrity::verify(&track_root)?;
        if !integrity.verified {
            return Err(AppError::Validation(format!(
                "The finalized track integrity check failed: {}",
                integrity.mismatch_files.join(", ")
            )));
        }
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?;
        let anchors_before = external_timestamp::finalization_anchors(&track_root)?;
        let staged = external_timestamp::stage(&track_root, certificate_id, source, input)?;
        let record = staged.record.clone();

        // Register the complete stage first. A process exit from this point on
        // leaves a database-visible pending record that workspace recovery can
        // deterministically publish; it cannot leave an invisible live orphan.
        if let Err(error) = self.persistence.save_external_timestamp(id, &record) {
            return Err(match external_timestamp::discard_staged(&track_root, &staged) {
                Ok(()) => error,
                Err(cleanup) => AppError::Data(format!(
                    "External timestamp database registration failed ({error}); staging cleanup also failed ({cleanup})."
                )),
            });
        }

        let post_publish_check = (|| -> Result<()> {
            external_timestamp::publish(&track_root, &staged)?;
            external_timestamp::verify_published_record(&track_root, &record)?;
            certificate::verify(&track_root)?;
            let post_integrity = integrity::verify(&track_root)?;
            if !post_integrity.verified {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed the phase-one integrity set.".into(),
                ));
            }
            let anchors_after = external_timestamp::finalization_anchors(&track_root)?;
            if anchors_after != anchors_before {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed a finalized anchor.".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = post_publish_check {
            let filesystem_cleanup =
                external_timestamp::remove_published_record(&track_root, &record)
                    .and_then(|()| external_timestamp::discard_staged(&track_root, &staged));
            if let Err(cleanup) = filesystem_cleanup {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); filesystem cleanup also failed ({cleanup}). The registered record was retained for recovery."
                )));
            }
            if let Err(cleanup) = self.persistence.remove_external_timestamp(id, &record.id) {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); database rollback also failed ({cleanup}). The registered failed record remains visible."
                )));
            }
            return Err(error);
        }
        self.detail_from_record(track, false)
    }

    /// Automatically request and attach a provider response for the fixed
    /// phase-one `EVIDENCE_MANIFEST.json` anchor. No timestamp metadata or
    /// source file is accepted from the UI.
    pub fn attach_configured_external_timestamp(&self, id: &str) -> Result<TrackDetail> {
        let context = self.configured_timestamp_context(id)?;
        if context.configuration_status != TimestampProviderConfigurationStatus::Ready {
            self.save_timestamp_attachment_summary(
                &context.track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::NotRecorded,
                    message: context.configuration_message.clone(),
                    provider: context.provider.clone(),
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return self.detail_from_record(context.track, false);
        }
        if let Some(detail) = self.existing_configured_timestamp(&context)? {
            return Ok(detail);
        }
        let (anchor, response) = match self.request_configured_timestamp(&context)? {
            TimestampRequestOutcome::Complete(detail) => return Ok(*detail),
            TimestampRequestOutcome::Response(response) => (response.anchor, response.response),
        };
        let publication = self.stage_configured_timestamp(&context, &anchor, response)?;
        self.publish_configured_timestamp(id, context, publication)
    }

    fn configured_timestamp_context(&self, id: &str) -> Result<ConfiguredTimestampContext> {
        let track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized
            || !track.certificate.valid
            || track.certificate.certificate_id.is_none()
        {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached to a valid technically finalized snapshot."
                    .into(),
            ));
        }
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?
            .to_owned();
        let track_root = self.track_root(&track)?;
        let settings = self.persistence.timestamp_settings()?;
        let provider = external_timestamp::provider_display_name(&settings);
        let secret = self.persistence.timestamp_secret()?;
        let (configuration_status, configuration_message) = external_timestamp::settings_status(
            &settings,
            secret
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
        );
        Ok(ConfiguredTimestampContext {
            track,
            certificate_id,
            track_root,
            settings,
            secret,
            provider,
            configuration_status,
            configuration_message,
        })
    }

    fn existing_configured_timestamp(
        &self,
        context: &ConfiguredTimestampContext,
    ) -> Result<Option<TrackDetail>> {
        let (current_records, _, _) = self.external_timestamp_context(&context.track)?;
        let Some(verified_record) = current_records
            .iter()
            .rev()
            .find(|record| currently_verified_provider_timestamp(record))
        else {
            return Ok(None);
        };
        self.save_timestamp_attachment_summary(
            &context.track,
            ExternalTimestampSummary {
                status: ExternalTimestampStatus::Verified,
                message: "A technically verified external timestamp is already attached to this finalized snapshot."
                    .into(),
                provider: context.provider.clone(),
                record_id: Some(verified_record.id.clone()),
                updated_at: Some(now()),
            },
        )?;
        Ok(Some(self.detail_from_record(context.track.clone(), false)?))
    }

    fn request_configured_timestamp(
        &self,
        context: &ConfiguredTimestampContext,
    ) -> Result<TimestampRequestOutcome> {
        let anchor = match self.verified_timestamp_anchor(&context.track_root) {
            Ok(anchor) => anchor,
            Err(error) => {
                self.save_timestamp_attachment_summary(
                    &context.track,
                    ExternalTimestampSummary {
                        status: ExternalTimestampStatus::AnchorMismatch,
                        message: "The selected timestamp anchor no longer matches the finalized snapshot."
                            .into(),
                        provider: context.provider.clone(),
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                let _ = error;
                return Ok(TimestampRequestOutcome::Complete(Box::new(
                    self.detail_from_record(context.track.clone(), false)?,
                )));
            }
        };
        self.save_timestamp_attachment_summary(
            &context.track,
            ExternalTimestampSummary {
                status: ExternalTimestampStatus::Requesting,
                message:
                    "Requesting external timestamp evidence for the finalized manifest anchor."
                        .into(),
                provider: context.provider.clone(),
                record_id: None,
                updated_at: Some(now()),
            },
        )?;
        let anchor_path =
            contained_path(&context.track_root, Path::new(&anchor.relative_path), true)?;
        let anchor_bytes =
            fs::read(&anchor_path).map_err(|error| AppError::io(&anchor_path, error))?;
        if !sha256_bytes(&anchor_bytes).eq_ignore_ascii_case(&anchor.sha256) {
            self.save_timestamp_attachment_summary(
                &context.track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::AnchorMismatch,
                    message:
                        "The selected timestamp anchor changed before the provider request started."
                            .into(),
                    provider: context.provider.clone(),
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return Ok(TimestampRequestOutcome::Complete(Box::new(
                self.detail_from_record(context.track.clone(), false)?,
            )));
        }
        let response = match external_timestamp::request_timestamp_for_artifact(
            &context.settings,
            context.secret.as_deref(),
            &anchor.sha256,
            &anchor_bytes,
        ) {
            Ok(response) => response,
            Err(failure) => {
                self.save_timestamp_attachment_summary(
                    &context.track,
                    ExternalTimestampSummary {
                        status: ExternalTimestampStatus::VerificationFailed,
                        message: failure.message,
                        provider: context.provider.clone(),
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                return Ok(TimestampRequestOutcome::Complete(Box::new(
                    self.detail_from_record(context.track.clone(), false)?,
                )));
            }
        };
        if !self
            .verified_timestamp_anchor(&context.track_root)
            .as_ref()
            .is_ok_and(|current| {
                current.relative_path == anchor.relative_path
                    && current.sha256.eq_ignore_ascii_case(&anchor.sha256)
            })
        {
            self.save_timestamp_attachment_summary(
                &context.track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::AnchorMismatch,
                    message: "The selected timestamp anchor changed while the provider request was in progress."
                        .into(),
                    provider: response.provider,
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return Ok(TimestampRequestOutcome::Complete(Box::new(
                self.detail_from_record(context.track.clone(), false)?,
            )));
        }
        Ok(TimestampRequestOutcome::Response(Box::new(
            ConfiguredTimestampResponse { anchor, response },
        )))
    }

    fn stage_configured_timestamp(
        &self,
        context: &ConfiguredTimestampContext,
        anchor: &FinalizationAnchor,
        response: external_timestamp::ProviderTimestampResponse,
    ) -> Result<ConfiguredTimestampPublication> {
        let temporary_source =
            provider_response_staging_path(&context.track_root, &response.evidence_extension)?;
        atomic_write_new(&temporary_source, &response.evidence_bytes)?;
        let snapshot_id =
            referenced_finalization_snapshot_id(&context.track, &context.certificate_id);
        let response_status = response.status;
        let response_message = response.message.clone();
        let response_provider = response.provider.clone();
        let staged = external_timestamp::stage_provider_response(
            &context.track_root,
            &context.certificate_id,
            &snapshot_id,
            &anchor.sha256,
            &temporary_source,
            response,
        );
        let _ = fs::remove_file(&temporary_source);
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                let status = if self.verified_timestamp_anchor(&context.track_root).is_err() {
                    ExternalTimestampStatus::AnchorMismatch
                } else {
                    ExternalTimestampStatus::VerificationFailed
                };
                self.save_timestamp_attachment_summary(
                    &context.track,
                    ExternalTimestampSummary {
                        status,
                        message: "Timestamp provider response could not be archived safely.".into(),
                        provider: response_provider,
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                return Err(error);
            }
        };
        let record = staged.record.clone();
        let anchors_before = external_timestamp::finalization_anchors(&context.track_root)?;
        Ok(ConfiguredTimestampPublication {
            staged,
            record,
            response_status,
            response_message,
            response_provider,
            anchors_before,
        })
    }

    fn publish_configured_timestamp(
        &self,
        id: &str,
        context: ConfiguredTimestampContext,
        publication: ConfiguredTimestampPublication,
    ) -> Result<TrackDetail> {
        if let Err(error) = self
            .persistence
            .save_external_timestamp(id, &publication.record)
        {
            return Err(match external_timestamp::discard_staged(
                &context.track_root,
                &publication.staged,
            ) {
                Ok(()) => error,
                Err(cleanup) => AppError::Data(format!(
                    "External timestamp database registration failed ({error}); staging cleanup also failed ({cleanup})."
                )),
            });
        }
        if let Err(error) = self.verify_configured_timestamp_publication(&context, &publication) {
            self.rollback_configured_timestamp_publication(id, &context, &publication, error)?;
        }
        self.save_timestamp_attachment_summary(
            &context.track,
            ExternalTimestampSummary {
                status: publication.response_status,
                message: publication.response_message,
                provider: publication.response_provider,
                record_id: Some(publication.record.id),
                updated_at: Some(now()),
            },
        )?;
        self.detail_from_record(context.track, false)
    }

    fn verify_configured_timestamp_publication(
        &self,
        context: &ConfiguredTimestampContext,
        publication: &ConfiguredTimestampPublication,
    ) -> Result<()> {
        external_timestamp::publish(&context.track_root, &publication.staged)?;
        external_timestamp::verify_published_record(&context.track_root, &publication.record)?;
        certificate::verify(&context.track_root)?;
        let post_integrity = integrity::verify(&context.track_root)?;
        if !post_integrity.verified {
            return Err(AppError::Validation(
                "Attaching timestamp evidence changed the phase-one integrity set.".into(),
            ));
        }
        let anchors_after = external_timestamp::finalization_anchors(&context.track_root)?;
        if anchors_after != publication.anchors_before {
            return Err(AppError::Validation(
                "Attaching timestamp evidence changed a finalized anchor.".into(),
            ));
        }
        Ok(())
    }

    fn rollback_configured_timestamp_publication(
        &self,
        id: &str,
        context: &ConfiguredTimestampContext,
        publication: &ConfiguredTimestampPublication,
        error: AppError,
    ) -> Result<()> {
        let filesystem_cleanup =
            external_timestamp::remove_published_record(&context.track_root, &publication.record)
                .and_then(|()| {
                    external_timestamp::discard_staged(&context.track_root, &publication.staged)
                });
        if let Err(cleanup) = filesystem_cleanup {
            return Err(AppError::Data(format!(
                "External timestamp publication failed ({error}); filesystem cleanup also failed ({cleanup}). The registered record was retained for recovery."
            )));
        }
        if let Err(cleanup) = self
            .persistence
            .remove_external_timestamp(id, &publication.record.id)
        {
            return Err(AppError::Data(format!(
                "External timestamp publication failed ({error}); database rollback also failed ({cleanup}). The registered failed record remains visible."
            )));
        }
        self.save_timestamp_attachment_summary(
            &context.track,
            ExternalTimestampSummary {
                status: ExternalTimestampStatus::VerificationFailed,
                message: "Timestamp provider response could not be published safely.".into(),
                provider: publication.response_provider.clone(),
                record_id: None,
                updated_at: Some(now()),
            },
        )?;
        Err(error)
    }

    pub(super) fn verified_timestamp_anchor(
        &self,
        track_root: &Path,
    ) -> Result<FinalizationAnchor> {
        let anchor = external_timestamp::finalized_manifest_anchor(track_root)?;
        // The certificate hash set itself must also remain valid. The specific
        // manifest mismatch above has already produced the user-facing anchor
        // status; this prevents a request when another phase-one certificate
        // artifact was modified.
        certificate::verify(track_root)?;
        let integrity = integrity::verify(track_root)?;
        if !integrity.verified {
            return Err(AppError::Validation(
                "The finalized track integrity check failed before timestamp attachment.".into(),
            ));
        }
        Ok(anchor)
    }

    pub(super) fn save_timestamp_attachment_summary(
        &self,
        track: &TrackRecord,
        summary: ExternalTimestampSummary,
    ) -> Result<()> {
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?;
        self.persistence
            .save_timestamp_attachment_summary(&track.id, certificate_id, &summary)
    }
}
