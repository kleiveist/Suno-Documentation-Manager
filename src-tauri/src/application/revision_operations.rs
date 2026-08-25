use super::*;

struct RevisionArchivePlan {
    revision_id: String,
    root: PathBuf,
    archive: PathBuf,
    stage: PathBuf,
    live_certificate: PathBuf,
    staged_certificate: PathBuf,
    certificate_existed: bool,
    live_pdfs: [(&'static str, PathBuf); 2],
    pdf_existed: [bool; 2],
    pdf_moved: Vec<&'static str>,
    live_audio_screening: PathBuf,
    staged_audio_screening: PathBuf,
    audio_screening_existed: bool,
    audio_screening_moved: bool,
}

impl WorkspaceApp {
    pub fn invalidate_certificate(&self, id: &str) -> Result<ActionResult> {
        let mut track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized {
            return Err(AppError::Validation("The track is not finalized.".into()));
        }
        invalidate_state(&mut track, "Certificate invalidated by the user");
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: "Certificate marked invalid; certificate files were not overwritten.".into(),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    pub fn create_revision(&self, id: &str) -> Result<ActionResult> {
        let (mut track, analyzed_evidence, certificate_integrity, mut plan) =
            self.prepare_revision(id)?;
        self.prepare_revision_staging(&track, certificate_integrity, &plan)?;
        self.stage_revision_certificate(&plan)?;
        self.stage_revision_pdfs(&mut plan)?;
        self.stage_revision_audio_screening(&mut plan)?;
        self.publish_revision_archive(&plan)?;
        Self::reset_track_for_revision(&mut track);
        self.save_revision_track(&track, &analyzed_evidence, &plan)?;
        let detail = self.revision_detail(id, track, &analyzed_evidence)?;
        let revision_id = &plan.revision_id;
        Ok(ActionResult {
            message: format!("Previous certificate archived as revision {revision_id}."),
            track: Some(detail),
        })
    }

    fn prepare_revision(
        &self,
        id: &str,
    ) -> Result<(
        TrackRecord,
        Vec<EvidenceItem>,
        &'static str,
        RevisionArchivePlan,
    )> {
        let track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized {
            return Err(AppError::Validation(
                "Only a finalized track can start a new revision.".into(),
            ));
        }
        let root = self.track_root(&track)?;
        // Analyze legacy Suno evidence before any certificate artifacts are moved.
        let (mut track, analyzed_evidence) = self.prepare_revision_suno_analysis(track)?;
        migrate_legacy_suno_semantics(&mut track.fields);
        let certificate_integrity = if certificate::verify(&root).is_ok() {
            "valid"
        } else {
            "invalid_or_incomplete"
        };
        let plan = self.revision_archive_plan(&track, root)?;
        Ok((track, analyzed_evidence, certificate_integrity, plan))
    }

    fn revision_archive_plan(
        &self,
        track: &TrackRecord,
        root: PathBuf,
    ) -> Result<RevisionArchivePlan> {
        let revision_id = Uuid::new_v4().to_string();
        ensure_contained_directory(&root, Path::new(".archive/revisions"))?;
        let archive_relative = PathBuf::from(".archive/revisions").join(&revision_id);
        let archive = contained_path(&root, &archive_relative, false)?;
        if archive.exists() {
            return Err(AppError::Collision(archive.display().to_string()));
        }
        let live_certificate =
            contained_path(&root, Path::new(certificate::CERTIFICATE_DIR), false)?;
        let certificate_existed = live_certificate.exists();
        let live_pdfs = [
            (
                certificate::PDF_FILE,
                contained_path(&root, Path::new(certificate::PDF_FILE), false)?,
            ),
            (
                certificate::PDF_FILE_DE,
                contained_path(&root, Path::new(certificate::PDF_FILE_DE), false)?,
            ),
        ];
        let pdf_existed = [
            regular_file_if_present(&live_pdfs[0].1, "The technical documentation PDF")?,
            regular_file_if_present(&live_pdfs[1].1, "The technical documentation PDF")?,
        ];
        let live_audio_screening =
            contained_path(&root, Path::new("03_DOCUMENTATION/AUDIO_SCREENING"), false)?;
        let audio_screening_existed = regular_directory_if_present(
            &live_audio_screening,
            "The pre-release audio-screening directory",
        )?;
        let stage_relative = PathBuf::from(".archive/revision-staging").join(&revision_id);
        let stage = ensure_contained_directory(&root, &stage_relative)?;
        let staged_certificate = stage.join("certificate");
        let staged_audio_screening = stage.join("03_DOCUMENTATION/AUDIO_SCREENING");
        let _ = track;
        Ok(RevisionArchivePlan {
            revision_id,
            root,
            archive,
            stage,
            live_certificate,
            staged_certificate,
            certificate_existed,
            live_pdfs,
            pdf_existed,
            pdf_moved: Vec::new(),
            live_audio_screening,
            staged_audio_screening,
            audio_screening_existed,
            audio_screening_moved: false,
        })
    }

    fn prepare_revision_staging(
        &self,
        track: &TrackRecord,
        certificate_integrity: &str,
        plan: &RevisionArchivePlan,
    ) -> Result<()> {
        let live_hashes = contained_path(&plan.root, Path::new(integrity::HASH_FILE), false)?;
        let preparation = (|| -> Result<()> {
            if live_hashes.is_file() {
                let archived_hashes = plan.stage.join(integrity::HASH_FILE);
                if let Some(parent) = archived_hashes.parent() {
                    fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
                }
                copy_new(&live_hashes, &archived_hashes)?;
                if sha256_file(&live_hashes)? != sha256_file(&archived_hashes)? {
                    return Err(AppError::Validation(
                        "The revision SHA256SUMS archive copy could not be verified.".into(),
                    ));
                }
            }
            let metadata = serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "revision_id": plan.revision_id,
                "track_id": track.id,
                "archived_at": now(),
                "previous_certificate": track.certificate,
                "certificate_integrity_at_archive": certificate_integrity,
            }))?;
            atomic_write_new(&plan.stage.join("revision.json"), &metadata)?;
            Ok(())
        })();
        match preparation {
            Ok(()) => Ok(()),
            Err(error) => Err(cleanup_revision_staging(&plan.stage, error)),
        }
    }

    fn stage_revision_certificate(&self, plan: &RevisionArchivePlan) -> Result<()> {
        let result = if plan.certificate_existed {
            fs::rename(&plan.live_certificate, &plan.staged_certificate)
                .map_err(|error| AppError::io(&plan.live_certificate, error))
        } else {
            fs::create_dir(&plan.staged_certificate)
                .map_err(|error| AppError::io(&plan.staged_certificate, error))
        };
        match result {
            Ok(()) => Ok(()),
            Err(error) => Err(cleanup_revision_staging(&plan.stage, error)),
        }
    }

    fn stage_revision_pdfs(&self, plan: &mut RevisionArchivePlan) -> Result<()> {
        for ((name, live_pdf), existed) in plan.live_pdfs.iter().zip(plan.pdf_existed) {
            if !existed {
                continue;
            }
            let staged_pdf = plan.stage.join(name);
            if let Err(error) = fs::rename(live_pdf, &staged_pdf) {
                return Err(rollback_revision_state(
                    RevisionRollbackContext {
                        live_certificate: &plan.live_certificate,
                        archived_certificate: &plan.staged_certificate,
                        live_root: &plan.root,
                        archived_root: &plan.stage,
                        revision_directory: &plan.stage,
                        certificate_existed: plan.certificate_existed,
                        pdf_moved: &plan.pdf_moved,
                    },
                    AppError::io(live_pdf, error),
                ));
            }
            plan.pdf_moved.push(*name);
        }
        Ok(())
    }

    fn stage_revision_audio_screening(&self, plan: &mut RevisionArchivePlan) -> Result<()> {
        if !plan.audio_screening_existed {
            return Ok(());
        }
        if let Some(parent) = plan.staged_audio_screening.parent() {
            fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
        }
        if let Err(error) = fs::rename(&plan.live_audio_screening, &plan.staged_audio_screening) {
            return Err(rollback_revision_state(
                RevisionRollbackContext {
                    live_certificate: &plan.live_certificate,
                    archived_certificate: &plan.staged_certificate,
                    live_root: &plan.root,
                    archived_root: &plan.stage,
                    revision_directory: &plan.stage,
                    certificate_existed: plan.certificate_existed,
                    pdf_moved: &plan.pdf_moved,
                },
                AppError::io(&plan.live_audio_screening, error),
            ));
        }
        plan.audio_screening_moved = true;
        Ok(())
    }

    fn publish_revision_archive(&self, plan: &RevisionArchivePlan) -> Result<()> {
        if let Err(error) =
            ensure_contained_directory(&plan.root, Path::new(certificate::CERTIFICATE_DIR))
        {
            self.restore_revision_audio_or_return(plan, error, false)?;
        }
        if let Err(error) = fs::rename(&plan.stage, &plan.archive) {
            let cause = AppError::io(&plan.archive, error);
            return self.restore_revision_audio_or_return(plan, cause, true);
        }
        Ok(())
    }

    fn restore_revision_audio_or_return(
        &self,
        plan: &RevisionArchivePlan,
        error: AppError,
        archive_publication: bool,
    ) -> Result<()> {
        let restore_error = if plan.audio_screening_moved {
            restore_audio_screening_directory(
                &plan.staged_audio_screening,
                &plan.live_audio_screening,
            )
            .err()
        } else {
            None
        };
        if let Some(restore_error) = restore_error {
            return Err(if archive_publication {
                AppError::Data(format!(
                    "Revision archive publication failed ({error}); audio-screening rollback failed: {restore_error}"
                ))
            } else {
                AppError::Data(format!(
                    "Revision failed ({error}); audio-screening rollback failed: {restore_error}"
                ))
            });
        }
        Err(rollback_revision_state(
            RevisionRollbackContext {
                live_certificate: &plan.live_certificate,
                archived_certificate: &plan.staged_certificate,
                live_root: &plan.root,
                archived_root: &plan.stage,
                revision_directory: &plan.stage,
                certificate_existed: plan.certificate_existed,
                pdf_moved: &plan.pdf_moved,
            },
            error,
        ))
    }

    fn reset_track_for_revision(track: &mut TrackRecord) {
        track.status = TrackStatus::Active;
        track.certificate = CertificateState::default();
        track.documents.current = false;
        track.integrity = IntegrityState::default();
        track.audio_screening = Default::default();
        track.updated_at = now();
    }

    fn save_revision_track(
        &self,
        track: &TrackRecord,
        analyzed_evidence: &[EvidenceItem],
        plan: &RevisionArchivePlan,
    ) -> Result<()> {
        if let Err(error) = self
            .persistence
            .save_track_and_evidence(track, analyzed_evidence)
        {
            let archived_audio_screening = plan.archive.join("03_DOCUMENTATION/AUDIO_SCREENING");
            let restore_error = if plan.audio_screening_moved {
                restore_audio_screening_directory(
                    &archived_audio_screening,
                    &plan.live_audio_screening,
                )
                .err()
            } else {
                None
            };
            if let Some(restore_error) = restore_error {
                return Err(AppError::Data(format!(
                    "Revision database save failed ({error}); audio-screening rollback failed: {restore_error}"
                )));
            }
            return Err(rollback_revision_state(
                RevisionRollbackContext {
                    live_certificate: &plan.live_certificate,
                    archived_certificate: &plan.archive.join("certificate"),
                    live_root: &plan.root,
                    archived_root: &plan.archive,
                    revision_directory: &plan.archive,
                    certificate_existed: plan.certificate_existed,
                    pdf_moved: &plan.pdf_moved,
                },
                error,
            ));
        }
        Ok(())
    }

    fn revision_detail(
        &self,
        id: &str,
        track: TrackRecord,
        analyzed_evidence: &[EvidenceItem],
    ) -> Result<TrackDetail> {
        let detail = self.detail_from_record(track, false)?;
        if analyzed_evidence
            .iter()
            .any(|item| item.role == EvidenceRole::ReleaseWav)
        {
            return Ok(self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .unwrap_or(detail));
        }
        Ok(detail)
    }

    pub fn re_evaluate_track(&self, id: &str) -> Result<ActionResult> {
        let current = workflow::config()?;
        self.re_evaluate_track_with_workflow(id, &current)
    }

    pub(super) fn re_evaluate_track_with_workflow(
        &self,
        id: &str,
        current: &workflow::WorkflowConfig,
    ) -> Result<ActionResult> {
        let previous = self.persistence.track(id)?;
        if previous.status == TrackStatus::Superseded {
            return Err(AppError::Finalized);
        }
        if previous.workflow_id == current.id && previous.workflow_version == current.version {
            return Err(AppError::Validation(
                "The track already uses the current workflow version.".into(),
            ));
        }

        let archived = previous.status == TrackStatus::Finalized;
        if archived {
            self.create_revision(id)?;
        }

        let mut track = self.persistence.track(id)?;
        let evidence = self.persistence.evidence(id)?;
        migrate_legacy_suno_semantics(&mut track.fields);
        reconcile_evidence_derived_fields(&mut track, &evidence);
        track.workflow_id = current.id.clone();
        track.workflow_version = current.version.clone();
        track.status = TrackStatus::Active;
        track.documents.current = false;
        track.integrity = IntegrityState::default();
        track.certificate = CertificateState::default();
        track.updated_at = now();

        let root = self.track_root(&track)?;
        let live_hashes = contained_path(&root, Path::new(integrity::HASH_FILE), false)?;
        if live_hashes.is_file() {
            fs::remove_file(&live_hashes).map_err(|error| AppError::io(&live_hashes, error))?;
        } else if live_hashes.exists() {
            return Err(AppError::Validation(
                "The current SHA256SUMS path is not a regular file.".into(),
            ));
        }
        self.persistence.save_track_clearing_steps(&track)?;

        // A workflow upgrade can make an older editable track subject to the
        // new local-screening requirement even though no release file was
        // imported in this invocation.  Keep that path automatic and local;
        // archived finalizations have already run this as part of
        // `create_revision` above.
        let has_verified_release = evidence.iter().any(|item| {
            item.role == EvidenceRole::ReleaseWav
                && item.verified
                && item.verification_error.is_none()
                && item.sha256.is_some()
        });
        let detail = self.detail_from_record(track, false)?;
        let detail = if !archived && has_verified_release {
            self.run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .unwrap_or(detail)
        } else {
            detail
        };

        Ok(ActionResult {
            message: if archived {
                format!(
                    "Previous certificate archived; track is ready for reevaluation with workflow {} {}.",
                    current.id, current.version
                )
            } else {
                format!(
                    "Track is ready for reevaluation with workflow {} {}.",
                    current.id, current.version
                )
            },
            track: Some(detail),
        })
    }
}
