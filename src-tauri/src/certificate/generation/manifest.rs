use super::*;

pub(super) struct GeneratedManifest<'a> {
    pub(super) hash_manifest_sha: String,
    pub(super) evidence_values: Vec<&'a EvidenceItem>,
    pub(super) archived_revisions: Vec<String>,
    pub(super) automation: crate::model::TrackAutomation,
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) manifest_sha: String,
}

pub(super) fn prepare_manifest<'a>(input: &GenerationInput<'a>) -> Result<GeneratedManifest<'a>> {
    let hash_manifest = contained_path(input.track_root, Path::new(HASH_FILE), true)?;
    let hash_manifest_sha = sha256_file(&hash_manifest)?;
    let hashes = parse_hashes(&hash_manifest)?;
    let evidence_values = verified_evidence(input.track, input.evidence)?;
    let archived_revisions = archived_revision_references(input.track_root)?;
    let evidence_manifest = manifest_evidence(&evidence_values)?;
    let automation = crate::workflow::automation_summary(input.track, input.evidence);
    let manifest = build_manifest(
        input,
        &evidence_values,
        &evidence_manifest,
        &archived_revisions,
        &hash_manifest_sha,
        &hashes,
        &automation,
    );
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    manifest_bytes.push(b'\n');
    let manifest_sha = sha256_bytes(&manifest_bytes);
    Ok(GeneratedManifest {
        hash_manifest_sha,
        evidence_values,
        archived_revisions,
        automation,
        manifest_bytes,
        manifest_sha,
    })
}

fn verified_evidence<'a>(
    track: &TrackRecord,
    evidence: &'a [EvidenceItem],
) -> Result<Vec<&'a EvidenceItem>> {
    let mut values = evidence
        .iter()
        .filter(|item| {
            item.role != EvidenceRole::ExternalTimestamp
                && item.verified
                && item.sha256.is_some()
                && item.verification_error.is_none()
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if track.fields.suno_terms_evidence_not_available == Some(true)
        && values
            .iter()
            .any(|item| item.role == EvidenceRole::SunoTermsRights)
    {
        return Err(AppError::Validation(
            "Certificate generation cannot combine verified Terms evidence with an unavailable claim."
                .into(),
        ));
    }
    Ok(values)
}

fn manifest_evidence<'a>(
    evidence_values: &[&'a EvidenceItem],
) -> Result<Vec<ManifestEvidence<'a>>> {
    evidence_values
        .iter()
        .map(|item| -> Result<ManifestEvidence<'_>> {
            Ok(ManifestEvidence {
                id: &item.id,
                role: item.role.as_str(),
                file_name: &item.file_name,
                relative_path: &item.relative_path,
                sha256: item.sha256.as_deref(),
                size_bytes: item.size_bytes,
                imported_at: &item.imported_at,
                source_global_evidence_id: item.source_global_evidence_id.as_deref(),
                coverage_start: item.coverage_start.as_deref(),
                coverage_end: item.coverage_end.as_deref(),
                provenance: &item.provenance,
                derived_from_evidence_id: item.derived_from_evidence_id.as_deref(),
                generator_version: item.generator_version.as_deref(),
                generated_disclosure_text: item.generated_disclosure_text.as_deref(),
                metadata: phase_one_metadata(&item.metadata)?,
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn build_manifest(
    input: &GenerationInput<'_>,
    evidence_values: &[&EvidenceItem],
    evidence_manifest: &[ManifestEvidence<'_>],
    archived_revisions: &[String],
    hash_manifest_sha: &str,
    hashes: &BTreeMap<String, String>,
    automation: &crate::model::TrackAutomation,
) -> serde_json::Value {
    let system_verification = system_verification_manifest(input, evidence_values, automation);
    let semantic_fields = input.track.fields.normalized_conditionals();
    let semantic_snapshot = semantic_snapshot_manifest(&semantic_fields);
    let pdf_archive = certificate_pdf::certificate_pdf_archive_metadata();
    let certificate = certificate_manifest(input, &pdf_archive, hash_manifest_sha);
    json!({
        "schema_version": EVIDENCE_MANIFEST_SCHEMA_VERSION,
        "track": {
            "id": input.track.id,
            "title": input.track.fields.title,
            "relative_path": ".",
            "production_start_date": input.track.fields.production_start_date,
            "production_end_date": input.track.fields.production_end_date,
            "final_export_date": input.track.fields.final_export_date,
        },
        "artist": {
            "name": input.profile.artist_name,
            "suno_profile_name": input.profile.suno_profile_name,
            "suno_handle": input.profile.suno_handle,
        },
        "documented_facts": &semantic_fields,
        "semantic_snapshot": semantic_snapshot,
        "profile_snapshot": input.profile,
        "workflow": {
            "id": input.track.workflow_id,
            "version": input.track.workflow_version,
            "application_version": env!("CARGO_PKG_VERSION"),
        },
        "finalization": {
            "timestamp": input.finalized_at,
            "result": "DOCUMENTATION COMPLETE",
            "meaning": "configured documentation requirements completed",
        },
        "steps": input.steps,
        "evidence": evidence_manifest,
        "audio_screening": audio_screening_manifest(&input.track.audio_screening),
        "hashes": hashes,
        "certificate": certificate,
        "origin_labels": {
            "user_confirmed_fact": "Values explicitly entered or confirmed by the user",
            "evidence_derived_metadata": "Metadata read from or captured with a local evidence import",
            "system_verification": "Local structural, hash, consistency, and configured workflow checks"
        },
        "limitations": {
            "artwork_import_timestamps": crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE,
        },
        "evidence_derived_metadata": {
            "suno_created_timestamp": automation.suno_created_timestamp,
            "suno_id": automation.suno_id,
        },
        "system_verification": system_verification,
        "external_timestamp": {
            "anchor_artifact": MANIFEST_FILE,
            "attempt_timing": "after_manifest_anchor_before_single_certificate_render",
            "status_recorded_in_final_certificate": true,
            "successful_response_archived_as_immutable_addendum": true,
            "later_retry_changes_certificate_pdf": false,
        },
        "deviations": input.deviations,
        "revision_archives": archived_revisions,
    })
}

fn semantic_snapshot_manifest(fields: &TrackFields) -> serde_json::Value {
    json!({
        "suno_lyrics_structure": {
            "suno_instrumental_mode_selected": recorded_bool(fields.instrumental_track),
            "content_classification": content_classification(fields),
            "vocal_intent": suno_vocal_intent(fields),
            "final_audio_contains_vocals": final_audio_contains_vocals(fields),
            "generation_text_field_used": generation_text_field_used(fields),
            "structure_instructions_present": structure_instructions_present(fields),
            "content_source": suno_content_source(fields),
            "exact_field_text": match fields.suno_content_classification {
                Some(SunoContentClassification::Empty) => "N/A",
                Some(_) => documented(&fields.suno_lyrics_field_text),
                None => "NOT DOCUMENTED",
            },
            "legacy_data_classification": if fields.lyrics_source.trim().is_empty()
                && fields.lyrics_text.trim().is_empty()
            {
                "N/A"
            } else {
                "NOT DOCUMENTED"
            },
        },
        "ai_transparency_audio": {
            "generative_ai_used": recorded_bool(fields.generative_ai_used),
            "ai_assisted_audio_elements": conditional_documentation_answer(
                fields.generative_ai_used,
                fields.ai_assisted_audio_elements,
            ),
            "ai_generated_audio_elements": conditional_documentation_answer(
                fields.generative_ai_used,
                fields.ai_generated_audio_elements,
            ),
            "audio_disclosure_applied": conditional_documentation_answer(
                fields.generative_ai_used,
                fields.audio_disclosure_applied,
            ),
            "deepfake_related_indicator_summary": deepfake_indicator_summary(fields),
        },
    })
}

fn certificate_manifest(
    input: &GenerationInput<'_>,
    pdf_archive: &certificate_pdf::CertificatePdfArchiveMetadata,
    hash_manifest_sha: &str,
) -> serde_json::Value {
    json!({
        "id": input.certificate_id,
        "format_version": CERTIFICATE_FORMAT_VERSION,
        "rendering": input.render_options,
        "pdf_languages": ["en", "de"],
        "pdf_files": [PDF_FILE, PDF_FILE_DE],
        "pdf_archive": {
            "archive_format": pdf_archive.archive_format,
            "font_embedding": pdf_archive.font_embedding,
            "embedded_fonts": pdf_archive.embedded_fonts,
            "embedded_font_sha256": pdf_archive.embedded_font_sha256,
            "font_version": pdf_archive.font_version,
            "font_license": pdf_archive.font_license,
            "output_intent": pdf_archive.output_intent,
        },
        "status": "DOCUMENTATION COMPLETE",
        "status_meaning": "configured documentation requirements completed",
        "workflow_pass_meaning": "Configured documentation requirements for this step were satisfied.",
        "sha256sums_sha256": hash_manifest_sha,
        "statement_scope": {
            "confirms": [
                "recorded user inputs", "finalized snapshot", "registered local evidence",
                "recorded provenance", "SHA-256 values", "configured workflow checks"
            ],
            "does_not_confirm": [
                "authorship", "rights ownership", "non-infringement", "legality",
                "license validity", "judicial evidentiary weight", "statutory compliance",
                "governmental certification"
            ]
        },
    })
}

fn system_verification_manifest(
    input: &GenerationInput<'_>,
    evidence_values: &[&EvidenceItem],
    automation: &crate::model::TrackAutomation,
) -> serde_json::Value {
    let byte_identical_pairs = crate::workflow::byte_identical_pairs(input.evidence);
    let artwork_sha256_match =
        crate::workflow::human_edited_final_artwork_sha256_match(input.evidence);
    let artwork_status = crate::workflow::human_edited_final_artwork_status(input.evidence);
    let automatic_relationships = automatic_role_relationships(evidence_values);
    let automatic_global_relationships =
        automatic_global_track_relationships(&input.track.id, evidence_values);
    json!({
        "subscription_final_generation_coverage": crate::workflow::subscription_generation_coverage(input.track, input.evidence),
        "subscription_production_coverage": crate::workflow::subscription_production_coverage(input.track, input.evidence),
        "release_original_file_name": crate::workflow::original_evidence_file_name(input.evidence, EvidenceRole::ReleaseWav),
        "suno_export_original_file_name": crate::workflow::original_evidence_file_name(input.evidence, EvidenceRole::SunoFinalExport),
        "external_timestamp_anchor_prepared": true,
        "fact_origins": {
            "final_suno_generation_id": automation.final_generation_id_origin,
            "final_suno_generation_date": automation.final_generation_origin,
            "production_end_date": automation.production_end_origin,
            "download_export_date": automation.download_export_origin,
            "last_editing_date": automation.final_export_origin,
        },
        "suno_metadata_detected": automation.suno_metadata_detected,
        "release_identical_to_suno_export": automation.release_identical_to_suno_export,
        "human_edited_final_artwork_sha256_match": artwork_sha256_match,
        "human_edited_final_artwork_status": artwork_status,
        "byte_identical_pairs": &byte_identical_pairs,
        "automatic_role_relationships": &automatic_relationships,
        "automatic_global_track_relationships": &automatic_global_relationships,
        "consistency_issues": &automation.consistency_issues,
    })
}
