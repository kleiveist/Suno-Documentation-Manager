use super::manifest::GeneratedManifest;
use super::*;

pub(super) struct GeneratedMarkdown {
    pub(super) certificate: String,
    pub(super) sha256: String,
}

struct WorkflowMarkdown {
    na_steps: String,
    completed_steps: String,
    open_blocking: usize,
    release_file_name: String,
    suno_export_file_name: String,
    generation_coverage: &'static str,
    production_coverage: &'static str,
    final_generation_origin: &'static str,
    final_generation_id_origin: &'static str,
    download_export_origin: &'static str,
    last_editing_origin: &'static str,
    last_editing_date: String,
    suno_metadata_detected: &'static str,
    release_identical_to_suno_export: &'static str,
    revision_archives: String,
}

struct EvidenceMarkdown {
    release_wav: String,
    final_artwork: String,
    evidence_register: String,
    terms_exist: bool,
    terms_ids: String,
    terms_details: String,
}

struct NarrativeMarkdown {
    source_provenance: String,
    suno_plan: String,
    suno_field: String,
    human_contribution: String,
    ai_audio: String,
    ai_artwork: String,
    audio_screening: String,
    finalization_timestamp: String,
}

pub(super) fn render_markdown_certificate(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
    finalization_timestamp: &FinalizationTimestampSnapshot,
) -> GeneratedMarkdown {
    let evidence = evidence_markdown(&manifest.evidence_values);
    let workflow = workflow_markdown(input, manifest);
    let narrative = narrative_markdown(input, manifest, finalization_timestamp);
    let english_certificate =
        render_english_certificate(input, manifest, &workflow, &evidence, &narrative);
    let english_certificate = english_certificate.replacen(
        &format!(
            "- Suno plan at generation [User-confirmed fact]: {}\n",
            markdown_documented(&input.track.fields.suno_plan_at_generation)
        ),
        &narrative.suno_plan,
        1,
    );
    let certificate = localized_markdown_certificate(&english_certificate, input.render_options);
    let sha256 = sha256_bytes(certificate.as_bytes());
    GeneratedMarkdown {
        certificate,
        sha256,
    }
}

fn evidence_markdown(evidence_values: &[&EvidenceItem]) -> EvidenceMarkdown {
    let release_wav = evidence_values
        .iter()
        .copied()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("NOT DOCUMENTED")
        .to_owned();
    let final_artwork = evidence_values
        .iter()
        .copied()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("N/A")
        .to_owned();
    let evidence_register = evidence_register_markdown(evidence_values);
    let terms = evidence_values
        .iter()
        .copied()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .collect::<Vec<_>>();
    let terms_ids = if terms.is_empty() {
        "N/A".to_owned()
    } else {
        terms
            .iter()
            .map(|item| format!("`{}`", markdown_raw_value(&item.id)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let terms_details = terms_evidence_markdown(&terms);
    EvidenceMarkdown {
        release_wav,
        final_artwork,
        evidence_register,
        terms_exist: !terms.is_empty(),
        terms_ids,
        terms_details,
    }
}

fn workflow_markdown(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
) -> WorkflowMarkdown {
    let na_steps = input
        .steps
        .iter()
        .filter(|step| step.status == StepStatus::NotApplicable)
        .map(|step| {
            format!(
                "- {} — {}\n",
                step.id,
                step.na_reason
                    .as_deref()
                    .map(markdown_documented)
                    .unwrap_or_else(|| "NOT DOCUMENTED".into())
            )
        })
        .collect::<String>();
    let completed_steps = input
        .steps
        .iter()
        .filter(|step| matches!(step.status, StepStatus::Pass | StepStatus::NotApplicable))
        .map(|step| format!("- {}: {}\n", step.id, step_status_label(&step.status)))
        .collect::<String>();
    let release_file_name =
        crate::workflow::original_evidence_file_name(input.evidence, EvidenceRole::ReleaseWav)
            .map(markdown_raw_value)
            .unwrap_or_else(|| "NOT RECORDED".into());
    let suno_export_file_name =
        crate::workflow::original_evidence_file_name(input.evidence, EvidenceRole::SunoFinalExport)
            .map(markdown_raw_value)
            .unwrap_or_else(|| "NOT RECORDED".into());
    let generation_coverage =
        match crate::workflow::subscription_generation_coverage(input.track, input.evidence) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        };
    let production_coverage =
        match crate::workflow::subscription_production_coverage(input.track, input.evidence) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        };
    WorkflowMarkdown {
        na_steps,
        completed_steps,
        open_blocking: input
            .deviations
            .iter()
            .filter(|deviation| deviation.blocking && !deviation.resolved)
            .count(),
        release_file_name,
        suno_export_file_name,
        generation_coverage,
        production_coverage,
        final_generation_origin: fact_origin_label(manifest.automation.final_generation_origin),
        final_generation_id_origin: fact_origin_label(
            manifest.automation.final_generation_id_origin,
        ),
        download_export_origin: fact_origin_label(manifest.automation.download_export_origin),
        last_editing_origin: fact_origin_label(manifest.automation.final_export_origin),
        last_editing_date: markdown_documented(&input.track.fields.final_export_date),
        suno_metadata_detected: yes_no(manifest.automation.suno_metadata_detected),
        release_identical_to_suno_export: yes_no(
            manifest.automation.release_identical_to_suno_export,
        ),
        revision_archives: if manifest.archived_revisions.is_empty() {
            "NONE RECORDED".to_owned()
        } else {
            markdown_raw_value(&manifest.archived_revisions.join(", "))
        },
    }
}

fn narrative_markdown(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
    finalization_timestamp: &FinalizationTimestampSnapshot,
) -> NarrativeMarkdown {
    NarrativeMarkdown {
        source_provenance: source_provenance_markdown(
            &input.track.fields,
            &manifest.evidence_values,
        ),
        suno_plan: suno_plan_context_markdown(&input.track.fields),
        suno_field: suno_field_markdown(&input.track.fields),
        human_contribution: human_contribution_markdown(&input.track.fields),
        ai_audio: ai_audio_markdown(&input.track.fields),
        ai_artwork: ai_artwork_markdown(&input.track.fields, input.evidence),
        audio_screening: audio_screening_markdown(&input.track.audio_screening),
        finalization_timestamp: finalization_timestamp_markdown(
            finalization_timestamp,
            &manifest.manifest_sha,
            input.track.fields.commercial_use_intended,
        ),
    }
}

fn render_english_certificate(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
    workflow: &WorkflowMarkdown,
    evidence: &EvidenceMarkdown,
    narrative: &NarrativeMarkdown,
) -> String {
    let track = input.track;
    let profile = input.profile;
    let automation = &manifest.automation;
    let certificate_id = input.certificate_id;
    let finalized_at = input.finalized_at;
    let release_file_name = &workflow.release_file_name;
    let suno_export_file_name = &workflow.suno_export_file_name;
    let last_editing_origin = workflow.last_editing_origin;
    let last_editing_date = &workflow.last_editing_date;
    let final_generation_origin = workflow.final_generation_origin;
    let final_generation_id_origin = workflow.final_generation_id_origin;
    let download_export_origin = workflow.download_export_origin;
    let suno_metadata_detected = workflow.suno_metadata_detected;
    let release_identical_to_suno_export = workflow.release_identical_to_suno_export;
    let source_provenance_md = &narrative.source_provenance;
    let human_contribution_md = &narrative.human_contribution;
    let suno_field_md = &narrative.suno_field;
    let ai_audio_md = &narrative.ai_audio;
    let ai_artwork_md = &narrative.ai_artwork;
    let production_coverage = workflow.production_coverage;
    let generation_coverage = workflow.generation_coverage;
    let terms_ids = &evidence.terms_ids;
    let terms_details_md = &evidence.terms_details;
    let finalization_timestamp_md = &narrative.finalization_timestamp;
    let evidence_values = &manifest.evidence_values;
    let evidence_register_md = &evidence.evidence_register;
    let release_wav = &evidence.release_wav;
    let final_artwork = &evidence.final_artwork;
    let hash_manifest_sha = &manifest.hash_manifest_sha;
    let manifest_sha = &manifest.manifest_sha;
    let open_blocking = workflow.open_blocking;
    let revision_archives = &workflow.revision_archives;
    let completed_steps = &workflow.completed_steps;
    let na_steps = &workflow.na_steps;
    let audio_screening_md = &narrative.audio_screening;
    format!(
        "# SunoDM Technical Documentation and Evidence Certificate\n\n> Technical documentation only — not a legal or governmental certification.\n\n## A. Certificate / Snapshot Identity\n\n- Certificate ID: `{certificate_id}`\n- Application version: `{}`\n- Workflow: `{}` / `{}`\n- Certificate schema: `{CERTIFICATE_FORMAT_VERSION}`\n- Finalized at: `{finalized_at}`\n- Documentation status: **DOCUMENTATION COMPLETE**\n- Meaning: configured documentation requirements completed\n- PASS definition: Configured documentation requirements for this step were satisfied.\n\n## B. Track identity\n\n- Documented title [User-confirmed fact]: {}\n- Artist [User-confirmed fact]: {}\n- Actual release filename [Evidence-derived metadata]: `{release_file_name}`\n- Actual Suno export filename [Evidence-derived metadata]: `{suno_export_file_name}`\n- Last editing date [{last_editing_origin}]: {last_editing_date}\n\n## C. Final Suno Generation\n\n- Final generation date [{final_generation_origin}]: {}\n- Final generation date origin: **{final_generation_origin}**\n- Final generation ID [{final_generation_id_origin}]: {}\n- Suno project URL [User-confirmed fact]: {}\n- Download/export date [{download_export_origin}]: {}\n- Download/export date origin: **{download_export_origin}**\n- Suno Studio metadata detected: **{suno_metadata_detected}**\n- Metadata detection origin: **System verification**\n- Metadata origin: {}\n- Suno model [User-confirmed fact]: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Release identical to Suno final export: **{release_identical_to_suno_export}**\n- Release identity origin: **System verification**\n\n## D. Source provenance\n\n{source_provenance_md}\n## E. Human contribution\n\n{human_contribution_md}\n{suno_field_md}\n## G. AI Transparency Assessment\n\n### G.1 Audio\n\n{ai_audio_md}\n### G.2 Artwork\n\n{ai_artwork_md}\n## H. License and rights evidence\n\n- Assigned subscription evidence jointly covers the production period [System verification]: **{production_coverage}**\n- Final-generation date covered [System verification]: **{generation_coverage}**\n- Terms evidence exists [System verification]: **{}**\n- Terms evidence IDs [System value]: {terms_ids}\n- Terms evidence not available [User-confirmed fact]: {}\n\n### Archived service-terms evidence\n\n{terms_details_md}\nThis is a factual coverage and archive status only; it is not a rights determination.\n\n## I. External Timestamp Evidence\n\n{finalization_timestamp_md}\n\n## J. Evidence register\n\n- Evidence file count: {}\n\n{evidence_register_md}\n## K. Integrity anchors and workflow\n\n- Release audio SHA-256: `{release_wav}`\n- Final artwork SHA-256: `{final_artwork}`\n- SHA256SUMS.txt SHA-256: `{hash_manifest_sha}`\n- Evidence manifest SHA-256: `{manifest_sha}`\n- Blocking deviations: {open_blocking}\n- Previous revision archives [System verification]: `{revision_archives}`\n- Final result: **DOCUMENTATION COMPLETE**\n\n### K.1 Configured workflow checks\n\n{completed_steps}\n### N/A steps with reasons\n\n{}\n### K.2 Pre-release audio screening\n\n{audio_screening_md}\n## L. Technical certificate statement\n\nThis certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks.\n\nIt does **not** confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification.\n\nOrigin labels used: **User-confirmed fact**, **Evidence-derived metadata**, **System verification**, and **System value**.\n",
        env!("CARGO_PKG_VERSION"),
        track.workflow_id,
        track.workflow_version,
        markdown_documented(&track.fields.title),
        markdown_documented(&profile.artist_name),
        markdown_documented(&track.fields.suno_final_generation_date),
        markdown_documented(&track.fields.suno_final_generation_id),
        markdown_documented(&track.fields.suno_project_url),
        markdown_documented(&track.fields.suno_download_export_date),
        if automation.suno_metadata_detected {
            "Evidence-derived metadata"
        } else {
            "NOT DOCUMENTED"
        },
        markdown_documented(&track.fields.suno_model),
        markdown_documented(&track.fields.suno_plan_at_generation),
        if evidence.terms_exist { "YES" } else { "NO" },
        recorded_bool(track.fields.suno_terms_evidence_not_available),
        evidence_values.len(),
        if na_steps.is_empty() {
            "- NONE\n"
        } else {
            na_steps
        }
    )
}
