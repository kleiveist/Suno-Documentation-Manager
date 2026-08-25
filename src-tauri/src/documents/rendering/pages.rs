use super::context::RenderContext;
use crate::model::EvidenceRole;

use super::super::evidence::{evidence_list, release_identity_status, terms_evidence_status};
use super::super::presentation::{
    conditional_value, content_classification, documented_list, fact_origin_label,
    generation_text_field_used, render_ai_audio, suno_content_source, value_or_missing,
    vocal_intent, yes_no,
};
use super::super::{marker, MANAGED_MARKER, TEMPLATE_VERSION};

pub(super) fn suno_project(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let automation = &context.automation;
    let source_code_file = context.evidence_path(EvidenceRole::SourceCodeFile);
    let code_generated_audio_file = context.evidence_path(EvidenceRole::CodeGeneratedAudioFile);
    format!(
        "# {MANAGED_MARKER}\nTemplate version: {TEMPLATE_VERSION}\nTrack: {}\nFinal generation date [{}]: {}\nFinal generation ID [{}]: {}\nSuno project URL [User-confirmed fact]: {}\nDownload/export date [{}]: {}\nSuno Studio metadata detected [System verification]: {}\nMetadata origin: {}\nSuno model [User-confirmed fact]: {}\nSuno plan at generation [User-confirmed fact]: {}\nLegacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\nRelease identical to Suno final export [System verification]: {}\nProduction start: {}\nProduction end: {}\nLast editing date: {}\nActual Suno export filename: {}\nSuno Instrumental Mode Selected [User-confirmed fact]: {}\nContent Classification [User-confirmed fact]: {}\nVocal Intent [User-confirmed fact]: {}\nFinal Audio Contains Vocals [User-confirmed fact]: {}\nGeneration Text Field Used [User-confirmed fact]: {}\nContent Source [User-confirmed fact]: {}\nExternal audio uploaded: {}\nOwn audio uploaded: {}\nCode-based generation: {}\nSource-code evidence: {}\nCode-audio post-processing performed: {}\nCode-audio post-processing operations: {}\nCode-generated audio evidence: {}\nThird-party samples uploaded: {}\n",
        fields.title,
        fact_origin_label(automation.final_generation_origin),
        value_or_missing(&fields.suno_final_generation_date),
        fact_origin_label(automation.final_generation_id_origin),
        value_or_missing(&fields.suno_final_generation_id),
        value_or_missing(&fields.suno_project_url),
        fact_origin_label(automation.download_export_origin),
        value_or_missing(&fields.suno_download_export_date),
        if automation.suno_metadata_detected {
            "YES"
        } else {
            "NO"
        },
        if automation.suno_metadata_detected {
            "Evidence-derived metadata"
        } else {
            "NOT DOCUMENTED"
        },
        value_or_missing(&fields.suno_model),
        value_or_missing(&fields.suno_plan_at_generation),
        value_or_missing(&fields.legacy_suno_plan_at_creation),
        release_identity_status(&context.evidence),
        value_or_missing(&fields.production_start_date),
        value_or_missing(&fields.production_end_date),
        value_or_missing(&fields.final_export_date),
        crate::workflow::original_evidence_file_name(
            &context.evidence,
            EvidenceRole::SunoFinalExport,
        )
        .unwrap_or("NOT RECORDED"),
        yes_no(fields.instrumental_track),
        content_classification(fields),
        vocal_intent(fields),
        yes_no(fields.vocal_lyrics_present),
        generation_text_field_used(fields),
        suno_content_source(fields),
        yes_no(fields.external_audio_uploaded),
        yes_no(fields.own_audio_uploaded),
        yes_no(fields.code_based_generation),
        conditional_value(fields.code_based_generation, source_code_file),
        if fields.code_based_generation == Some(true) {
            yes_no(fields.code_audio_post_processed)
        } else {
            "N/A"
        },
        if fields.code_based_generation == Some(true)
            && fields.code_audio_post_processed == Some(true)
        {
            documented_list(&fields.code_audio_post_processing_operations)
        } else {
            "N/A".into()
        },
        conditional_value(fields.code_based_generation, code_generated_audio_file),
        yes_no(fields.third_party_samples_uploaded),
    )
}

pub(super) fn readme(
    context: &RenderContext<'_>,
    source_declarations: &str,
    confirmed_work: &str,
    audio_screening_summary: &str,
    timestamp_recommendation: &str,
) -> String {
    let fields = &context.fields;
    let automation = &context.automation;
    format!(
        "{}# Track documentation: {}\n\nTemplate version: `{}`\nWorkflow: `{}` version `{}`\n\n## Snapshot\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Suno plan at generation: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Commercial use intended: {}\n- Production period: {} to {}\n- Last editing date: {}\n- Final generation date [{}]: {}\n- Final generation ID [{}]: {}\n- Suno project URL: {}\n- Download/export date [{}]: {}\n- Actual release filename: {}\n- Actual Suno export filename: {}\n- Suno Instrumental Mode Selected: {}\n- Content Classification: {}\n- Vocal Intent: {}\n- Final Audio Contains Vocals: {}\n{}{}\n{}\n## Workflow status\n\n- Documentation status meaning: configured documentation requirements completed.\n- PASS means: Configured documentation requirements for this step were satisfied.\n- The authoritative evaluated step results are stored in the completion certificate after finalization.\n\n## External Timestamp Evidence\n\n- External timestamp evidence at technical finalization: NOT RECORDED\n- No external timestamp evidence recorded.\n{}\n## Evidence\n\n{}",
        marker(),
        fields.title,
        TEMPLATE_VERSION,
        context.track.workflow_id,
        context.track.workflow_version,
        value_or_missing(&context.profile.artist_name),
        value_or_missing(&context.profile.suno_profile_name),
        value_or_missing(&context.profile.suno_handle),
        value_or_missing(&fields.suno_plan_at_generation),
        value_or_missing(&fields.legacy_suno_plan_at_creation),
        if fields.commercial_use_intended {
            "YES"
        } else {
            "NO"
        },
        value_or_missing(&fields.production_start_date),
        value_or_missing(&fields.production_end_date),
        value_or_missing(&fields.final_export_date),
        fact_origin_label(automation.final_generation_origin),
        value_or_missing(&fields.suno_final_generation_date),
        fact_origin_label(automation.final_generation_id_origin),
        value_or_missing(&fields.suno_final_generation_id),
        value_or_missing(&fields.suno_project_url),
        fact_origin_label(automation.download_export_origin),
        value_or_missing(&fields.suno_download_export_date),
        crate::workflow::original_evidence_file_name(&context.evidence, EvidenceRole::ReleaseWav)
            .unwrap_or("NOT RECORDED"),
        crate::workflow::original_evidence_file_name(
            &context.evidence,
            EvidenceRole::SunoFinalExport,
        )
        .unwrap_or("NOT RECORDED"),
        yes_no(fields.instrumental_track),
        content_classification(fields),
        vocal_intent(fields),
        yes_no(fields.vocal_lyrics_present),
        source_declarations,
        confirmed_work,
        audio_screening_summary,
        timestamp_recommendation,
        evidence_list(&context.evidence)
    )
}

pub(super) fn ai_usage(context: &RenderContext<'_>, ai_artwork_usage: &str) -> String {
    let fields = &context.fields;
    let automation = &context.automation;
    let source_code_file = context.evidence_path(EvidenceRole::SourceCodeFile);
    let code_generated_audio_file = context.evidence_path(EvidenceRole::CodeGeneratedAudioFile);
    format!(
        "{}# AI usage\n\n## Final Suno Generation\n\n- Final generation date [{}]: {}\n- Final generation ID [{}]: {}\n- Suno project URL [User-confirmed fact]: {}\n- Download/export date [{}]: {}\n- Suno Studio metadata detected [System verification]: {}\n- Suno model [User-confirmed fact]: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Release identical to Suno final export [System verification]: {}\n- Suno Instrumental Mode Selected [User-confirmed fact]: {}\n- Content Classification [User-confirmed fact]: {}\n- Vocal Intent [User-confirmed fact]: {}\n- Final Audio Contains Vocals [User-confirmed fact]: {}\n- Content Source [User-confirmed fact]: {}\n- External audio uploaded: {}\n- Code-based generation: {}\n- Source-code evidence: {}\n- Code-audio post-processing performed: {}\n- Code-audio post-processing operations: {}\n- Code-generated audio evidence: {}\n\n## AI Transparency Assessment – Audio\n\n{}\nNo AI Act compliance, legal necessity, or legal safety determination is made.\n\n## AI Transparency Assessment – Artwork\n\n{}",
        marker(),
        fact_origin_label(automation.final_generation_origin),
        value_or_missing(&fields.suno_final_generation_date),
        fact_origin_label(automation.final_generation_id_origin),
        value_or_missing(&fields.suno_final_generation_id),
        value_or_missing(&fields.suno_project_url),
        fact_origin_label(automation.download_export_origin),
        value_or_missing(&fields.suno_download_export_date),
        if automation.suno_metadata_detected {
            "YES"
        } else {
            "NO"
        },
        value_or_missing(&fields.suno_model),
        value_or_missing(&fields.suno_plan_at_generation),
        value_or_missing(&fields.legacy_suno_plan_at_creation),
        release_identity_status(&context.evidence),
        yes_no(fields.instrumental_track),
        content_classification(fields),
        vocal_intent(fields),
        yes_no(fields.vocal_lyrics_present),
        suno_content_source(fields),
        yes_no(fields.external_audio_uploaded),
        yes_no(fields.code_based_generation),
        conditional_value(fields.code_based_generation, source_code_file),
        if fields.code_based_generation == Some(true) {
            yes_no(fields.code_audio_post_processed)
        } else {
            "N/A"
        },
        if fields.code_based_generation == Some(true)
            && fields.code_audio_post_processed == Some(true)
        {
            documented_list(&fields.code_audio_post_processing_operations)
        } else {
            "N/A".into()
        },
        conditional_value(fields.code_based_generation, code_generated_audio_file),
        render_ai_audio(fields),
        ai_artwork_usage
    )
}

pub(super) fn suno_account_and_license(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let terms_evidence = context
        .evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .cloned()
        .collect::<Vec<_>>();
    let terms_ids = if terms_evidence.is_empty() {
        "N/A".to_owned()
    } else {
        terms_evidence
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{}# Suno account, subscription, and archived terms evidence\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Workspace subscription start date: {}\n- Commercial use intended: {}\n- Final generation date: {}\n- Assigned subscription evidence jointly covers the production period [System verification]: {}\n- Final-generation date covered [System verification]: {}\n- Terms evidence exists [System verification]: {}\n- Terms evidence IDs: {}\n- Terms evidence not available [User-confirmed fact]: {}\n\n## Archived service-terms evidence\n\n{}\nThis page records supplied account facts and locally archived evidence only. It does not confirm rights ownership, license validity, legality, or non-infringement.\n",
        marker(),
        value_or_missing(&context.profile.artist_name),
        value_or_missing(&context.profile.suno_profile_name),
        value_or_missing(&context.profile.suno_handle),
        value_or_missing(&fields.suno_plan_at_generation),
        value_or_missing(&fields.legacy_suno_plan_at_creation),
        value_or_missing(&context.profile.subscription_start_date),
        if fields.commercial_use_intended {
            "YES"
        } else {
            "NO"
        },
        value_or_missing(&fields.suno_final_generation_date),
        match crate::workflow::subscription_production_coverage(
            context.track,
            &context.evidence,
        ) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        },
        match crate::workflow::subscription_generation_coverage(
            context.track,
            &context.evidence,
        ) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        },
        terms_evidence_status(&context.evidence),
        terms_ids,
        yes_no(fields.suno_terms_evidence_not_available),
        evidence_list(&terms_evidence)
    )
}
