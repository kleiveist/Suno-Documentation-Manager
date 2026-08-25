use super::*;

#[cfg(test)]
pub(super) fn apply_patch(fields: &mut crate::model::TrackFields, patch: TrackPatch) {
    apply_patch_with_explicit_nulls(fields, patch, &[]);
}

pub(super) fn apply_patch_with_explicit_nulls(
    fields: &mut crate::model::TrackFields,
    mut patch: TrackPatch,
    explicit_null_fields: &[String],
) {
    apply_identity_patch(fields, &mut patch);
    apply_source_patch(fields, &mut patch);
    apply_editing_patch(fields, &mut patch);
    apply_disclosure_patch(fields, &mut patch);
    apply_artwork_patch(fields, &mut patch);
    for field in explicit_null_fields {
        clear_source_field(fields, field);
        clear_artwork_field(fields, field);
        clear_disclosure_field(fields, field);
    }
    fields.normalize_conditionals();
}

fn apply_identity_patch(fields: &mut crate::model::TrackFields, patch: &mut TrackPatch) {
    if let Some(value) = patch.title.take() {
        fields.title = value;
    }
    if let Some(value) = patch.production_start_date.take() {
        fields.production_start_date = value;
    }
    if let Some(value) = patch.production_end_date.take() {
        fields.production_end_date = value;
    }
    if let Some(value) = patch.suno_model.take() {
        fields.suno_model = value;
    }
    if let Some(value) = patch.suno_project_url.take() {
        fields.suno_project_url = value;
    }
    if let Some(value) = patch.suno_project_version_id.take() {
        fields.suno_project_version_id = value;
    }
    if let Some(value) = patch.suno_final_generation_id.take() {
        fields.suno_final_generation_id = value;
    }
    if let Some(value) = patch.suno_final_generation_date.take() {
        fields.suno_final_generation_date = value;
    }
    if let Some(value) = patch.suno_final_generation_time.take() {
        fields.suno_final_generation_time = value;
    }
    if let Some(value) = patch.suno_download_export_date.take() {
        fields.suno_download_export_date = value;
    }
    if let Some(value) = patch.suno_plan_at_generation.take() {
        fields.suno_plan_at_generation = value;
    }
    if let Some(value) = patch.legacy_suno_plan_at_creation.take() {
        fields.legacy_suno_plan_at_creation = value;
    }
    if let Some(value) = patch.final_export_date.take() {
        fields.final_export_date = value;
    }
}

fn apply_source_patch(fields: &mut crate::model::TrackFields, patch: &mut TrackPatch) {
    if let Some(value) = patch.instrumental_track.take() {
        fields.instrumental_track = Some(value);
    }
    if let Some(value) = patch.vocal_lyrics_present.take() {
        fields.vocal_lyrics_present = Some(value);
    }
    if let Some(value) = patch.vocal_intent.take() {
        fields.vocal_intent = Some(value);
    }
    if let Some(value) = patch.suno_content_classification.take() {
        fields.suno_content_classification = Some(value);
    }
    if let Some(value) = patch.suno_lyrics_content_source.take() {
        fields.suno_lyrics_content_source = Some(value);
    }
    if let Some(value) = patch.suno_lyrics_field_text.take() {
        fields.suno_lyrics_field_text = value;
    }
    if let Some(value) = patch.suno_lyrics_other_content_type.take() {
        fields.suno_lyrics_other_content_type = value;
    }
    if let Some(value) = patch.lyrics_source.take() {
        fields.lyrics_source = value;
    }
    if let Some(value) = patch.lyrics_text.take() {
        fields.lyrics_text = value;
    }
    if let Some(value) = patch.suno_style_prompt.take() {
        fields.suno_style_prompt = value;
    }
    if let Some(value) = patch.external_audio_uploaded.take() {
        fields.external_audio_uploaded = Some(value);
    }
    if let Some(value) = patch.external_audio_source.take() {
        fields.external_audio_source = value;
    }
    if let Some(value) = patch.external_audio_ownership.take() {
        fields.external_audio_ownership = value;
    }
    if let Some(value) = patch.own_audio_uploaded.take() {
        fields.own_audio_uploaded = Some(value);
    }
    if let Some(value) = patch.own_audio_source.take() {
        fields.own_audio_source = value;
    }
    if let Some(value) = patch.own_audio_ownership.take() {
        fields.own_audio_ownership = value;
    }
}

fn apply_editing_patch(fields: &mut crate::model::TrackFields, patch: &mut TrackPatch) {
    if let Some(value) = patch.code_based_generation.take() {
        fields.code_based_generation = Some(value);
    }
    if let Some(value) = patch.code_audio_post_processed.take() {
        fields.code_audio_post_processed = Some(value);
    }
    if let Some(value) = patch.code_audio_post_processing_operations.take() {
        fields.code_audio_post_processing_operations = value;
    }
    if let Some(value) = patch.code_audio_post_processing_note.take() {
        fields.code_audio_post_processing_note = value;
    }
    if let Some(value) = patch.third_party_samples_uploaded.take() {
        fields.third_party_samples_uploaded = Some(value);
    }
    if let Some(value) = patch.third_party_sample_source.take() {
        fields.third_party_sample_source = value;
    }
    if let Some(value) = patch.third_party_sample_ownership.take() {
        fields.third_party_sample_ownership = value;
    }
    if let Some(value) = patch.human_editing_performed.take() {
        fields.human_editing_performed = Some(value);
    }
    if let Some(value) = patch.human_editing_details.take() {
        fields.human_editing_details = value;
    }
    if let Some(value) = patch.post_export_editing_performed.take() {
        fields.post_export_editing_performed = Some(value);
    }
    if let Some(value) = patch.post_export_editing_details.take() {
        fields.post_export_editing_details = value;
    }
    if let Some(value) = patch.commercial_use_intended.take() {
        fields.commercial_use_intended = value;
    }
    if let Some(value) = patch.release_filename_difference_confirmed.take() {
        fields.release_filename_difference_confirmed = Some(value);
    }
    if let Some(value) = patch.suno_export_filename_difference_confirmed.take() {
        fields.suno_export_filename_difference_confirmed = Some(value);
    }
    if let Some(value) = patch.suno_terms_evidence_not_available.take() {
        fields.suno_terms_evidence_not_available = Some(value);
    }
}

fn apply_disclosure_patch(fields: &mut crate::model::TrackFields, patch: &mut TrackPatch) {
    if let Some(value) = patch.generative_ai_used.take() {
        fields.generative_ai_used = Some(value);
    }
    if let Some(value) = patch.audio_ai_system.take() {
        fields.audio_ai_system = value;
    }
    if let Some(value) = patch.ai_assisted_audio_elements.take() {
        fields.ai_assisted_audio_elements = Some(value);
    }
    if let Some(value) = patch.ai_generated_audio_elements.take() {
        fields.ai_generated_audio_elements = Some(value);
    }
    if let Some(value) = patch.real_person_voice_intentionally_imitated.take() {
        fields.real_person_voice_intentionally_imitated = Some(value);
    }
    if let Some(value) = patch.real_person_identity_intentionally_represented.take() {
        fields.real_person_identity_intentionally_represented = Some(value);
    }
    if let Some(value) = patch.real_event_represented_as_authentic_recording.take() {
        fields.real_event_represented_as_authentic_recording = Some(value);
    }
    if let Some(value) = patch
        .real_location_institution_event_presented_as_authentic_ai_recording
        .take()
    {
        fields.real_location_institution_event_presented_as_authentic_ai_recording = Some(value);
    }
    if let Some(value) = patch.audio_disclosure_applied.take() {
        fields.audio_disclosure_applied = Some(value);
    }
    if let Some(value) = patch.audio_disclosure_locations.take() {
        fields.audio_disclosure_locations = value;
    }
    if let Some(value) = patch.audio_disclosure_text.take() {
        fields.audio_disclosure_text = value;
    }
    if let Some(value) = patch.audio_disclosure_reason.take() {
        fields.audio_disclosure_reason = value;
    }
}

fn apply_artwork_patch(fields: &mut crate::model::TrackFields, patch: &mut TrackPatch) {
    if let Some(value) = patch.artwork_origin.take() {
        fields.artwork_origin = value;
    }
    if let Some(value) = patch.ai_image_service.take() {
        fields.ai_image_service = value;
    }
    if let Some(value) = patch.human_artwork_process_operations.take() {
        fields.human_artwork_process_operations = value;
    }
    if let Some(value) = patch.human_artwork_process_notes.take() {
        fields.human_artwork_process_notes = value;
    }
    if let Some(value) = patch.human_artwork_modifications.take() {
        fields.human_artwork_modifications = value;
    }
    if let Some(value) = patch.custom_artwork_change.take() {
        fields.custom_artwork_change = value;
    }
    if let Some(value) = patch.depicts_real_person.take() {
        fields.depicts_real_person = Some(value);
    }
    if let Some(value) = patch.real_person_notes.take() {
        fields.real_person_notes = value;
    }
    if let Some(value) = patch.depicts_real_event.take() {
        fields.depicts_real_event = Some(value);
    }
    if let Some(value) = patch.real_event_notes.take() {
        fields.real_event_notes = value;
    }
    if let Some(value) = patch.contains_trademark.take() {
        fields.contains_trademark = Some(value);
    }
    if let Some(value) = patch.trademark_notes.take() {
        fields.trademark_notes = value;
    }
    if let Some(value) = patch.disclosure_applied.take() {
        fields.disclosure_applied = Some(value);
    }
    if let Some(value) = patch.disclosure_text.take() {
        fields.disclosure_text = value;
    }
    if let Some(value) = patch.release_notes.take() {
        fields.release_notes = value;
    }
}

fn clear_source_field(fields: &mut crate::model::TrackFields, field: &str) {
    match field {
        "instrumentalTrack" => fields.instrumental_track = None,
        "vocalLyricsPresent" => fields.vocal_lyrics_present = None,
        "vocalIntent" => fields.vocal_intent = None,
        "sunoContentClassification" => fields.suno_content_classification = None,
        "sunoLyricsContentSource" => fields.suno_lyrics_content_source = None,
        "externalAudioUploaded" => fields.external_audio_uploaded = None,
        "ownAudioUploaded" => fields.own_audio_uploaded = None,
        "codeBasedGeneration" => fields.code_based_generation = None,
        "codeAudioPostProcessed" => fields.code_audio_post_processed = None,
        "thirdPartySamplesUploaded" => fields.third_party_samples_uploaded = None,
        "humanEditingPerformed" => fields.human_editing_performed = None,
        "postExportEditingPerformed" => fields.post_export_editing_performed = None,
        _ => {}
    }
}

fn clear_artwork_field(fields: &mut crate::model::TrackFields, field: &str) {
    match field {
        "releaseFilenameDifferenceConfirmed" => {
            fields.release_filename_difference_confirmed = None;
        }
        "sunoExportFilenameDifferenceConfirmed" => {
            fields.suno_export_filename_difference_confirmed = None;
        }
        "sunoTermsEvidenceNotAvailable" => fields.suno_terms_evidence_not_available = None,
        "depictsRealPerson" => fields.depicts_real_person = None,
        "depictsRealEvent" => fields.depicts_real_event = None,
        "containsTrademark" => fields.contains_trademark = None,
        "disclosureApplied" => fields.disclosure_applied = None,
        _ => {}
    }
}

fn clear_disclosure_field(fields: &mut crate::model::TrackFields, field: &str) {
    match field {
        "generativeAiUsed" => fields.generative_ai_used = None,
        "aiAssistedAudioElements" => fields.ai_assisted_audio_elements = None,
        "aiGeneratedAudioElements" => fields.ai_generated_audio_elements = None,
        "realPersonVoiceIntentionallyImitated" => {
            fields.real_person_voice_intentionally_imitated = None;
        }
        "realPersonIdentityIntentionallyRepresented" => {
            fields.real_person_identity_intentionally_represented = None;
        }
        "realEventRepresentedAsAuthenticRecording" => {
            fields.real_event_represented_as_authentic_recording = None;
        }
        "realLocationInstitutionEventPresentedAsAuthenticAiRecording" => {
            fields.real_location_institution_event_presented_as_authentic_ai_recording = None;
        }
        "audioDisclosureApplied" => fields.audio_disclosure_applied = None,
        _ => {}
    }
}
