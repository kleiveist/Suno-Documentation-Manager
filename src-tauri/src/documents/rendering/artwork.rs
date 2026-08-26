use super::context::RenderContext;
use crate::model::EvidenceRole;

use super::super::evidence::evidence_list;
use super::super::presentation::{
    deliberate_non_application, documented_list, value_or_missing, yes_no,
};
use super::super::{marker, ARTWORK_IMPORT_TIMESTAMP_NOTICE};

pub(super) fn image_generation_document(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    if !context.ai_artwork {
        return format!(
            "{}# AI image generation record\n\n- Artwork origin: {}\n\nNo AI image generation record applies to the documented artwork origin.\n",
            marker(),
            value_or_missing(&fields.artwork_origin)
        );
    }
    let mut content = format!(
        "{}# AI image generation record\n\n- AI image service: {}\n- Artwork origin: {}\n",
        marker(),
        value_or_missing(&fields.ai_image_service),
        value_or_missing(&fields.artwork_origin)
    );
    content.push_str(&format!(
        "- Project transparency policy: {}\n- Disclosure applied: {}\n- Disclosure deliberately not applied: {}\n",
        context.profile.artwork_transparency_policy,
        yes_no(fields.disclosure_applied),
        deliberate_non_application(fields.disclosure_applied)
    ));
    if fields.disclosure_applied == Some(true) {
        content.push_str(&format!(
            "- Disclosure text: {}\n",
            value_or_missing(&fields.disclosure_text)
        ));
    }
    content.push_str(
        "\nThe service name is a user-supplied fact; this document does not assert license rights.\n\n",
    );
    content.push_str(ARTWORK_IMPORT_TIMESTAMP_NOTICE);
    content.push('\n');
    content
}

pub(super) fn artwork_document(context: &RenderContext<'_>) -> String {
    let fields = &context.fields;
    let mut content = format!(
        "{}# Artwork process\n\n- Origin: {}\n",
        marker(),
        value_or_missing(&fields.artwork_origin)
    );
    if !context.artwork_present {
        content.push_str("\nNo artwork process applies to this track.\n");
        return content;
    }
    append_human_process(&mut content, fields);
    append_ai_process(&mut content, context);
    append_artwork_checks(&mut content, context);
    append_artwork_evidence(&mut content, context);
    content
}

fn append_human_process(output: &mut String, fields: &crate::model::TrackFields) {
    if fields.artwork_origin != "human" {
        return;
    }
    output.push_str(&format!(
        "- Human process operations: {}\n",
        documented_list(&fields.human_artwork_process_operations)
    ));
    if !fields.human_artwork_process_notes.trim().is_empty() {
        output.push_str(&format!(
            "- Human process notes: {}\n",
            value_or_missing(&fields.human_artwork_process_notes)
        ));
    }
}

fn append_ai_process(output: &mut String, context: &RenderContext<'_>) {
    if !context.ai_artwork {
        return;
    }
    let fields = &context.fields;
    output.push_str(&format!(
        "- AI service: {}\n- AI-generated base image: {}\n",
        value_or_missing(&fields.ai_image_service),
        context.ai_original()
    ));
    append_human_modifications(output, fields);
    output.push_str(&format!(
        "- Disclosure policy: {}\n- Disclosure applied: {}\n- Disclosure deliberately not applied: {}\n",
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

fn append_artwork_checks(output: &mut String, context: &RenderContext<'_>) {
    let fields = &context.fields;
    output.push_str(&format!(
        "- Final output: {}\n- Human-edited/final artwork comparison [System verification]: {}\n- Real person intentionally depicted: {}\n",
        context.final_artwork(),
        context.artwork_hash_status,
        yes_no(fields.depicts_real_person)
    ));
    if fields.depicts_real_person == Some(true) {
        output.push_str(&format!(
            "- Real-person note: {}\n",
            value_or_missing(&fields.real_person_notes)
        ));
    }
    output.push_str(&format!(
        "- Real event represented as authentic: {}\n",
        yes_no(fields.depicts_real_event)
    ));
    if fields.depicts_real_event == Some(true) {
        output.push_str(&format!(
            "- Real-event note: {}\n",
            value_or_missing(&fields.real_event_notes)
        ));
    }
    output.push_str(&format!(
        "- Trademark or company logo reproduced: {}\n",
        yes_no(fields.contains_trademark)
    ));
    if fields.contains_trademark == Some(true) {
        output.push_str(&format!(
            "- Trademark/logo note: {}\n",
            value_or_missing(&fields.trademark_notes)
        ));
    }
}

fn append_artwork_evidence(output: &mut String, context: &RenderContext<'_>) {
    let artwork_evidence = context
        .evidence
        .iter()
        .filter(|item| {
            item.relative_path.starts_with("05_ARTWORK/")
                && (context.ai_artwork
                    || !matches!(
                        item.role,
                        EvidenceRole::AiArtworkOriginal | EvidenceRole::AiArtworkEdited
                    ))
        })
        .cloned()
        .collect::<Vec<_>>();
    output.push_str(&format!(
        "\n## Artwork evidence\n\n{ARTWORK_IMPORT_TIMESTAMP_NOTICE}\n\n{}",
        evidence_list(&artwork_evidence)
    ));
}
