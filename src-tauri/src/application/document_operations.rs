use super::*;

impl WorkspaceApp {
    pub fn preview_documents(&self, id: &str) -> Result<DocumentPreview> {
        let track = self.persistence.track(id)?;
        documents::preview(&self.track_root(&track)?)
    }

    #[cfg(test)]
    pub fn generate_documents(&self, id: &str, adopt_existing: bool) -> Result<ActionResult> {
        self.generate_documents_with_progress(id, adopt_existing, &mut |_| {})
    }

    pub fn generate_documents_with_progress(
        &self,
        id: &str,
        adopt_existing: bool,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        on_progress(OperationProgress {
            stage: "preparing_documents".into(),
            ..OperationProgress::default()
        });
        let mut track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
        validate_track_fields(&track.fields)?;
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
        let track_root = self.track_root(&track)?;
        let files = documents::generate_with_progress(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
            adopt_existing,
            on_progress,
        )?;
        let generated_file_count = files.len() as u32;
        track.documents = DocumentState {
            generated: true,
            current: true,
            generated_at: Some(now()),
            template_version: documents::TEMPLATE_VERSION.into(),
            files,
            input_fingerprint: documents::input_fingerprint(
                &track,
                &track.profile_snapshot,
                &evidence,
            )?,
        };
        track.documents.current = documents::is_current(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
        )?;
        track.integrity = IntegrityState::default();
        track.status = TrackStatus::Active;
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: generated_file_count,
            total_files: generated_file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let detail = self.detail_from_record(track, false)?;
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: generated_file_count,
            total_files: generated_file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!("{} documents generated.", detail.documents.files.len()),
            track: Some(detail),
        })
    }

    pub fn generate_artwork_disclosure(
        &self,
        id: &str,
        disclosure_text: Option<String>,
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        if !matches!(
            track.fields.artwork_origin.as_str(),
            "ai_generated" | "ai_assisted"
        ) {
            return Err(AppError::Validation(
                "Visible disclosure is available only for AI-generated or AI-assisted artwork."
                    .into(),
            ));
        }
        let evidence_items = self.verified_evidence(&track)?;
        let source = evidence_items
            .iter()
            .rev()
            .find(|item| item.role == EvidenceRole::AiArtworkOriginal)
            .ok_or_else(|| AppError::Validation("Import the AI artwork original first.".into()))?;
        let text = disclosure_text
            .as_deref()
            .unwrap_or(&track.fields.disclosure_text)
            .trim()
            .to_owned();
        if evidence_items.iter().any(|item| {
            item.role == EvidenceRole::AiArtworkEdited
                && item.provenance == EvidenceProvenance::GeneratedDisclosure
                && item.verified
                && item.verification_error.is_none()
                && item.derived_from_evidence_id.as_deref() == Some(source.id.as_str())
                && item.generator_version.as_deref()
                    == Some(crate::artwork::DISCLOSURE_GENERATOR_VERSION)
                && item.generated_disclosure_text.as_deref() == Some(text.as_str())
        }) {
            if track.fields.disclosure_applied != Some(true) || track.fields.disclosure_text != text
            {
                track.fields.disclosure_applied = Some(true);
                track.fields.disclosure_text = text;
                mark_content_changed(&mut track);
                track.updated_at = now();
                self.persistence.save_track(&track)?;
            }
            return Ok(ActionResult {
                message: "Der aktuelle sichtbare KI-Hinweis ist bereits vorhanden; es wurde keine doppelte Datei erzeugt."
                    .into(),
                track: Some(self.detail_from_record(track, false)?),
            });
        }
        let track_root = self.track_root(&track)?;
        let generated =
            crate::artwork::generate_disclosure(&track_root, &track.fields.title, source, &text)?;
        if let Err(error) = self.persistence.save_evidence(id, &generated) {
            if let Ok(path) = contained_path(&track_root, Path::new(&generated.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        track.fields.disclosure_applied = Some(true);
        track.fields.disclosure_text = text;
        mark_content_changed(&mut track);
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: "Visible AI disclosure generated locally; select a final artwork separately."
                .into(),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    #[cfg(test)]
    pub fn calculate_hashes(&self, id: &str) -> Result<ActionResult> {
        self.calculate_hashes_with_progress(id, &mut |_| {})
    }

    pub fn calculate_hashes_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
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
        let track_root = self.track_root(&track)?;
        track.documents.current = documents::is_current(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
        )?;
        if !track.documents.current {
            return Err(AppError::Validation(
                "Generate the current managed documents before calculating hashes.".into(),
            ));
        }
        track.integrity = integrity::calculate_with_progress(&track_root, on_progress)?;
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: track.integrity.file_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let count = track.integrity.file_count;
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: count,
            total_files: count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!("{count} files hashed and re-verified."),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    pub fn verify_hashes_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.persistence.track(id)?;
        let track_root = self.track_root(&track)?;
        track.integrity = integrity::verify_with_progress(&track_root, on_progress)?;
        if !track.integrity.verified && track.status == TrackStatus::Finalized {
            invalidate_state(&mut track, "Track integrity changed after finalization");
        }
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: track.integrity.verified_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let message = format!(
            "{} of {} files verified.",
            track.integrity.verified_count, track.integrity.file_count
        );
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: track.integrity.verified_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message,
            track: Some(self.detail_from_record(track, false)?),
        })
    }
}
