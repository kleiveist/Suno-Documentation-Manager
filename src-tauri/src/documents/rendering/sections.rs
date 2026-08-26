use super::context::RenderContext;
use crate::model::EvidenceRole;

use super::super::presentation::{
    deliberate_non_application, documented_list, english_guided_list, english_guided_value,
    value_or_missing, yes_no, HUMAN_WORK_CHOICES, POST_EXPORT_CHOICES, RELEASE_CHOICES,
    RIGHTS_CHOICES, SOURCE_CHOICES,
};
use super::super::ARTWORK_IMPORT_TIMESTAMP_NOTICE;

pub(super) fn source_declarations(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let source_code_file = context.evidence_path(EvidenceRole::SourceCodeFile);
    let code_generated_audio_file = context.evidence_path(EvidenceRole::CodeGeneratedAudioFile);
    let mut declarations = format!(
        "\n## Source declarations\n\n- External audio uploaded: {}\n",
        yes_no(fields.external_audio_uploaded)
    );
    append_external_audio(&mut declarations, fields);
    declarations.push_str(&format!(
        "- Own audio uploaded: {}\n",
        yes_no(fields.own_audio_uploaded)
    ));
    append_own_audio(&mut declarations, fields);
    declarations.push_str(&format!(
        "- Code-based generation: {}\n",
        yes_no(fields.code_based_generation)
    ));
    append_code_generation(
        &mut declarations,
        fields,
        source_code_file,
        code_generated_audio_file,
    );
    declarations.push_str(&format!(
        "- Third-party samples uploaded: {}\n",
        yes_no(fields.third_party_samples_uploaded)
    ));
    append_third_party_samples(&mut declarations, fields);
    declarations
}

fn append_external_audio(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.external_audio_uploaded == Some(true) {
        output.push_str(&format!(
            "- External audio source category: {}\n- External audio rights basis: {}\n",
            english_guided_value(&fields.external_audio_source, SOURCE_CHOICES),
            english_guided_value(&fields.external_audio_ownership, RIGHTS_CHOICES)
        ));
    }
}

fn append_own_audio(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.own_audio_uploaded == Some(true) {
        output.push_str(&format!(
            "- Own audio source category: {}\n- Own audio rights basis: {}\n",
            english_guided_value(&fields.own_audio_source, SOURCE_CHOICES),
            english_guided_value(&fields.own_audio_ownership, RIGHTS_CHOICES)
        ));
    }
}

fn append_code_generation(
    output: &mut String,
    fields: &crate::model::TrackFields,
    source_code_file: &str,
    code_generated_audio_file: &str,
) {
    if fields.code_based_generation != Some(true) {
        return;
    }
    output.push_str(&format!("- Source-code evidence: {source_code_file}\n"));
    output.push_str(&format!(
        "- Post-processing performed: {}\n",
        yes_no(fields.code_audio_post_processed)
    ));
    append_code_post_processing(output, fields);
    output.push_str(&format!(
        "- Code-generated audio evidence: {code_generated_audio_file}\n"
    ));
}

fn append_code_post_processing(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.code_audio_post_processed != Some(true) {
        return;
    }
    output.push_str(&format!(
        "- Post-processing operations: {}\n",
        documented_list(&fields.code_audio_post_processing_operations)
    ));
    if !fields.code_audio_post_processing_note.trim().is_empty() {
        output.push_str(&format!(
            "- Other post-processing note: {}\n",
            value_or_missing(&fields.code_audio_post_processing_note)
        ));
    }
}

fn append_third_party_samples(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.third_party_samples_uploaded == Some(true) {
        output.push_str(&format!(
            "- Third-party sample source category: {}\n- Third-party sample rights basis: {}\n",
            english_guided_value(&fields.third_party_sample_source, SOURCE_CHOICES),
            english_guided_value(&fields.third_party_sample_ownership, RIGHTS_CHOICES)
        ));
    }
}

pub(super) fn confirmed_work(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let mut confirmed = format!(
        "\n## Confirmed work and release choices\n\n- Human editing performed: {}\n",
        yes_no(fields.human_editing_performed)
    );
    if fields.human_editing_performed == Some(true) {
        confirmed.push_str(&format!(
            "- Confirmed human work: {}\n",
            english_guided_list(&fields.human_editing_details, HUMAN_WORK_CHOICES)
        ));
    }
    confirmed.push_str(&format!(
        "- Desktop-PC editing after the Suno WAV: {}\n",
        yes_no(fields.post_export_editing_performed)
    ));
    if fields.post_export_editing_performed == Some(true) {
        confirmed.push_str(&format!(
            "- Confirmed desktop-PC editing work: {}\n",
            english_guided_list(&fields.post_export_editing_details, POST_EXPORT_CHOICES)
        ));
    }
    confirmed.push_str(&format!(
        "- Release notes: {}\n",
        english_guided_list(&fields.release_notes, RELEASE_CHOICES)
    ));
    confirmed
}

pub(super) fn ai_artwork_usage(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let mut usage = format!("- Origin: {}\n", value_or_missing(&fields.artwork_origin));
    if !context.artwork_present {
        return usage;
    }
    if context.ai_artwork {
        append_ai_artwork_usage(context, &mut usage);
    }
    usage.push_str(&format!(
        "- Final output: {}\n- Human-edited/final artwork comparison [System verification]: {}\n- Artwork import timestamp scope: {}\n",
        context.final_artwork(),
        context.artwork_hash_status,
        ARTWORK_IMPORT_TIMESTAMP_NOTICE
    ));
    usage
}

fn append_ai_artwork_usage(context: &RenderContext<'_>, output: &mut String) {
    let fields = &context.fields;
    output.push_str(&format!(
        "- AI service: {}\n- AI-generated base image: {}\n",
        value_or_missing(&fields.ai_image_service),
        context.ai_original()
    ));
    append_human_modifications(output, fields);
    output.push_str(&format!(
        "- Project transparency policy: {}\n- Visible disclosure applied: {}\n- Visible disclosure deliberately not applied: {}\n",
        context.profile.artwork_transparency_policy,
        yes_no(fields.disclosure_applied),
        deliberate_non_application(fields.disclosure_applied)
    ));
    if fields.disclosure_applied == Some(true) {
        output.push_str(&format!(
            "- Disclosure text: {}\n",
            value_or_missing(&fields.disclosure_text)
        ));
    }
}

fn append_human_modifications(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.artwork_origin != "ai_assisted" {
        return;
    }
    output.push_str(&format!(
        "- Human modifications: {}\n",
        documented_list(&fields.human_artwork_modifications)
    ));
    if !fields.custom_artwork_change.trim().is_empty() {
        output.push_str(&format!(
            "- Other human editing details: {}\n",
            value_or_missing(&fields.custom_artwork_change)
        ));
    }
}

pub(super) fn timestamp_recommendation(context: &RenderContext<'_>) -> &'static str {
    if context.fields.commercial_use_intended {
        "\nFor long-term evidentiary preservation, an external timestamp can be added after technical finalization.\n"
    } else {
        ""
    }
}
