use super::*;

#[test]
fn historical_artwork_freetext_loads_without_data_loss_and_serializes_as_a_list() {
    let fields: TrackFields = serde_json::from_value(serde_json::json!({
        "humanArtworkModifications": "Historischer frei beschreibbarer Wert",
        "codeAudioPostProcessingOperations": "Historische Nachbearbeitung"
    }))
    .expect("historical track fields");

    assert_eq!(
        fields.human_artwork_modifications,
        vec!["Historischer frei beschreibbarer Wert"]
    );
    assert_eq!(
        fields.code_audio_post_processing_operations,
        vec!["Historische Nachbearbeitung"]
    );
    let serialized = serde_json::to_value(fields).expect("serialize migrated fields");
    assert!(serialized["humanArtworkModifications"].is_array());
    assert!(serialized["codeAudioPostProcessingOperations"].is_array());
}

#[test]
fn documentation_enums_use_stable_wire_values() {
    assert_eq!(
        serde_json::to_value(DocumentationAnswer::NotDocumented).expect("answer"),
        "not_documented"
    );
    for (classification, expected) in [
        (SunoContentClassification::StructureOnly, "STRUCTURE_ONLY"),
        (
            SunoContentClassification::VocalLyricsOnly,
            "VOCAL_LYRICS_ONLY",
        ),
        (SunoContentClassification::Mixed, "MIXED"),
        (SunoContentClassification::Empty, "EMPTY"),
        (SunoContentClassification::Other, "OTHER"),
    ] {
        assert_eq!(
            serde_json::to_value(classification).expect("content classification"),
            expected
        );
    }
    for (intent, expected) in [
        (VocalIntent::Vocal, "VOCAL"),
        (VocalIntent::Instrumental, "INSTRUMENTAL"),
        (VocalIntent::Unspecified, "UNSPECIFIED"),
    ] {
        assert_eq!(
            serde_json::to_value(intent).expect("vocal intent"),
            expected
        );
    }
    assert_eq!(
        serde_json::to_value(SunoLyricsContentType::StructureInstructions).expect("content type"),
        "structure_instructions"
    );
    assert_eq!(
        serde_json::to_value(SunoLyricsContentSource::Ai).expect("content source"),
        "ai"
    );
    assert_eq!(
        serde_json::to_value(TimestampType::QualifiedElectronicTimestampUserDeclared)
            .expect("timestamp type"),
        "qualified_electronic_timestamp_user_declared"
    );
    assert_eq!(
        serde_json::to_value(TimestampReferencedArtifact::EvidenceManifest)
            .expect("timestamp artifact"),
        "evidence_manifest"
    );
}

#[test]
fn new_suno_semantics_are_optional_without_legacy_inference() {
    let fields: TrackFields = serde_json::from_value(serde_json::json!({
        "sunoLyricsFieldContent": true,
        "sunoLyricsContentTypes": ["vocal_lyrics", "structure_instructions"],
        "vocalLyricsPresent": true
    }))
    .expect("legacy Suno semantic fields");

    assert_eq!(fields.suno_content_classification, None);
    assert_eq!(fields.vocal_intent, None);
    assert_eq!(
        fields.suno_lyrics_content_types,
        vec![
            SunoLyricsContentType::VocalLyrics,
            SunoLyricsContentType::StructureInstructions,
        ]
    );

    let empty = serde_json::to_value(TrackFields::default()).expect("empty track fields");
    assert!(empty.get("sunoLyricsContentTypes").is_none());
    assert!(empty["sunoContentClassification"].is_null());
    assert!(empty["vocalIntent"].is_null());
}

#[test]
fn singular_suno_semantics_round_trip_as_canonical_values() {
    let fields: TrackFields = serde_json::from_value(serde_json::json!({
        "sunoContentClassification": "MIXED",
        "vocalIntent": "VOCAL"
    }))
    .expect("canonical Suno semantic fields");

    assert_eq!(
        fields.suno_content_classification,
        Some(SunoContentClassification::Mixed)
    );
    assert_eq!(fields.vocal_intent, Some(VocalIntent::Vocal));

    let serialized = serde_json::to_value(fields).expect("serialize Suno semantic fields");
    assert_eq!(serialized["sunoContentClassification"], "MIXED");
    assert_eq!(serialized["vocalIntent"], "VOCAL");
    assert!(serialized.get("sunoLyricsContentTypes").is_none());

    assert!(
        serde_json::from_value::<SunoContentClassification>(serde_json::json!("mixed")).is_err()
    );
    assert!(serde_json::from_value::<VocalIntent>(serde_json::json!("vocal")).is_err());
}

#[test]
fn legacy_profile_and_certificate_state_default_to_english_rendering() {
    let profile: Profile = serde_json::from_value(serde_json::json!({
        "artistName": "Legacy Artist",
        "sunoProfileName": "legacy-profile",
        "sunoHandle": "@legacy",
        "sunoPlan": "Pro",
        "subscriptionStartDate": "2026-01-01",
        "defaultCommercialUse": true,
        "defaultAiImageService": "Legacy tool",
        "artworkTransparencyPolicy": "always",
        "disclosureText": "AI-assisted"
    }))
    .expect("pre-language profile remains readable");
    let certificate: CertificateState = serde_json::from_value(serde_json::json!({
        "valid": true,
        "certificateId": "SDM-legacy",
        "finalizedAt": "2026-08-18T00:00:00Z",
        "workflowVersion": "1.7"
    }))
    .expect("pre-language certificate state remains readable");

    assert_eq!(profile.certificate_language, CertificateLanguage::En);
    assert_eq!(certificate.certificate_language, CertificateLanguage::En);
    assert!(!certificate.bilingual);
    assert_eq!(
        serde_json::to_value(CertificateLanguage::De).expect("certificate language"),
        "de"
    );
}

#[test]
fn no_not_documented_and_not_applicable_remain_distinct_states() {
    let mut fields = TrackFields::default();
    let unanswered = serde_json::to_value(&fields).expect("unanswered fields");
    assert!(unanswered["vocalLyricsPresent"].is_null());

    fields.vocal_lyrics_present = Some(false);
    fields.audio_disclosure_applied = Some(DocumentationAnswer::No);
    let documented_no = serde_json::to_value(&fields).expect("documented no");
    assert_eq!(documented_no["vocalLyricsPresent"], false);
    assert_eq!(documented_no["audioDisclosureApplied"], "no");

    fields.audio_disclosure_applied = Some(DocumentationAnswer::NotDocumented);
    let not_documented = serde_json::to_value(&fields).expect("not documented");
    assert_eq!(not_documented["audioDisclosureApplied"], "not_documented");
    assert_eq!(
        serde_json::to_value(StepStatus::NotApplicable).expect("not applicable"),
        "N_A"
    );
}

#[test]
fn historical_lyrics_and_plan_keys_remain_unclassified_legacy_values() {
    let fields: TrackFields = serde_json::from_value(serde_json::json!({
        "lyricsSource": "mixed",
        "lyricsText": "Historical lyrics",
        "sunoPlanAtCreation": "Pro"
    }))
    .expect("historical fields");

    assert_eq!(fields.lyrics_source, "mixed");
    assert_eq!(fields.lyrics_text, "Historical lyrics");
    assert!(fields.suno_plan_at_generation.is_empty());
    assert_eq!(fields.legacy_suno_plan_at_creation, "Pro");

    let serialized = serde_json::to_value(fields).expect("serialize fields");
    assert_eq!(serialized["legacyLyricsSource"], "mixed");
    assert_eq!(serialized["legacyLyricsText"], "Historical lyrics");
    assert_eq!(serialized["sunoPlanAtGeneration"], "");
    assert_eq!(serialized["legacySunoPlanAtCreation"], "Pro");
    assert!(serialized.get("lyricsSource").is_none());
    assert!(serialized.get("lyricsText").is_none());
    assert!(serialized.get("sunoPlanAtCreation").is_none());
}

#[test]
fn patch_request_distinguishes_explicit_null_from_an_omitted_field() {
    let request: TrackPatchRequest = serde_json::from_value(serde_json::json!({
        "generativeAiUsed": null,
        "instrumentalTrack": null,
        "title": "Updated title"
    }))
    .expect("track patch request");

    assert_eq!(request.patch.title.as_deref(), Some("Updated title"));
    assert!(request.patch.generative_ai_used.is_none());
    assert!(request.patch.instrumental_track.is_none());
    assert!(request
        .explicit_null_fields
        .contains(&"generativeAiUsed".to_owned()));
    assert!(request
        .explicit_null_fields
        .contains(&"instrumentalTrack".to_owned()));
    assert!(!request
        .explicit_null_fields
        .contains(&"vocalLyricsPresent".to_owned()));
}

#[test]
fn instrumental_normalization_preserves_legacy_and_structure_field_text() {
    let mut fields = TrackFields {
        instrumental_track: Some(true),
        vocal_lyrics_present: Some(false),
        suno_lyrics_field_content: Some(true),
        suno_lyrics_content_types: vec![SunoLyricsContentType::StructureInstructions],
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
        suno_lyrics_field_text: "[Intro]\n[Instrumental]".into(),
        suno_lyrics_other_content_type: "inactive note".into(),
        lyrics_source: "instrumental".into(),
        lyrics_text: "historical field value".into(),
        ..TrackFields::default()
    };

    fields.normalize_conditionals();

    assert_eq!(fields.lyrics_text, "historical field value");
    assert_eq!(fields.suno_lyrics_field_text, "[Intro]\n[Instrumental]");
    assert_eq!(
        fields.suno_lyrics_content_types,
        vec![SunoLyricsContentType::StructureInstructions]
    );
    assert!(fields.suno_lyrics_other_content_type.is_empty());
}

#[test]
fn inactive_new_lyrics_and_audio_branches_are_cleared_without_inventing_answers() {
    let mut fields = TrackFields {
        suno_content_classification: Some(SunoContentClassification::Empty),
        suno_lyrics_field_content: Some(true),
        suno_lyrics_content_types: vec![SunoLyricsContentType::Other],
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
        suno_lyrics_field_text: "stale field text".into(),
        suno_lyrics_other_content_type: "stale other type".into(),
        generative_ai_used: Some(false),
        audio_ai_system: "stale system".into(),
        ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
        audio_disclosure_applied: Some(DocumentationAnswer::No),
        audio_disclosure_reason: "stale reason".into(),
        ..TrackFields::default()
    };

    fields.normalize_conditionals();

    assert_eq!(
        fields.suno_content_classification,
        Some(SunoContentClassification::Empty)
    );
    assert_eq!(fields.suno_lyrics_field_content, None);
    assert!(fields.suno_lyrics_content_types.is_empty());
    assert!(fields.suno_lyrics_content_source.is_none());
    assert!(fields.suno_lyrics_field_text.is_empty());
    assert!(fields.suno_lyrics_other_content_type.is_empty());
    assert!(fields.audio_ai_system.is_empty());
    assert!(fields.ai_assisted_audio_elements.is_none());
    assert!(fields.audio_disclosure_applied.is_none());
    assert!(fields.audio_disclosure_reason.is_empty());
}

#[test]
fn canonical_mixed_classification_supersedes_legacy_controllers_without_touching_intent() {
    let mut fields = TrackFields {
        vocal_intent: Some(VocalIntent::Instrumental),
        suno_content_classification: Some(SunoContentClassification::Mixed),
        suno_lyrics_field_content: Some(false),
        suno_lyrics_content_types: vec![SunoLyricsContentType::Other],
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
        suno_lyrics_field_text: "Vocal line\n[Drop]".into(),
        suno_lyrics_other_content_type: "stale legacy label".into(),
        ..TrackFields::default()
    };

    fields.normalize_conditionals();

    assert_eq!(fields.suno_lyrics_field_content, None);
    assert!(fields.suno_lyrics_content_types.is_empty());
    assert_eq!(fields.suno_lyrics_field_text, "Vocal line\n[Drop]");
    assert!(fields.suno_lyrics_other_content_type.is_empty());
    assert_eq!(fields.vocal_intent, Some(VocalIntent::Instrumental));
}

#[test]
fn unanswered_controlling_questions_do_not_erase_pending_new_facts() {
    let mut fields = TrackFields {
        suno_lyrics_field_content: None,
        suno_lyrics_field_text: "pending field text".into(),
        generative_ai_used: None,
        audio_ai_system: "pending system".into(),
        ..TrackFields::default()
    };

    fields.normalize_conditionals();

    assert_eq!(fields.suno_lyrics_field_text, "pending field text");
    assert_eq!(fields.audio_ai_system, "pending system");
}

#[test]
fn legacy_timestamp_provider_statuses_deserialize_into_separate_configuration_states() {
    for (legacy, expected) in [
        (
            "configuration_incomplete",
            TimestampProviderConfigurationStatus::NotConfigured,
        ),
        (
            "provider_unavailable",
            TimestampProviderConfigurationStatus::ConnectionFailed,
        ),
        (
            "unsupported_response",
            TimestampProviderConfigurationStatus::ProviderError,
        ),
    ] {
        let status: TimestampProviderConfigurationStatus =
            serde_json::from_value(serde_json::json!(legacy)).expect("legacy provider status");
        assert_eq!(status, expected);
    }

    assert_eq!(
        serde_json::to_value(TimestampProviderConfigurationStatus::NotConfigured)
            .expect("current provider status"),
        "not_configured"
    );
}

#[test]
fn empty_current_qualification_period_fields_remain_absent_from_legacy_json() {
    let legacy_shape = serde_json::to_value(TimestampQualificationRecord::default())
        .expect("serialize default qualification record");
    assert!(legacy_shape.get("currentServiceStatus").is_none());
    assert!(legacy_shape.get("currentStatusValidFrom").is_none());
    assert!(legacy_shape.get("currentStatusValidUntil").is_none());

    let current = TimestampQualificationRecord {
        current_service_status: "granted".into(),
        current_status_valid_from: "2026-01-01T00:00:00Z".into(),
        ..Default::default()
    };
    let current_shape =
        serde_json::to_value(current).expect("serialize current qualification record");
    assert_eq!(current_shape["currentServiceStatus"], "granted");
    assert_eq!(
        current_shape["currentStatusValidFrom"],
        "2026-01-01T00:00:00Z"
    );
}

#[test]
fn external_timestamp_role_accepts_standard_timestamp_container_extensions() {
    let extensions = EvidenceRole::ExternalTimestamp.allowed_extensions();
    assert!(extensions.contains(&"tsr"));
    assert!(extensions.contains(&"tst"));
    assert!(extensions.contains(&"p7s"));
    assert_eq!(
        EvidenceRole::ExternalTimestamp.destination(),
        "03_DOCUMENTATION"
    );
}

#[test]
fn public_audio_screening_summary_omits_fingerprint_and_internal_track_ids() {
    let mut state = AudioScreeningState::default();
    state.local.fingerprint = "RAW_CHROMAPRINT_MUST_NOT_REACH_WEBVIEW".into();
    state.local.track_id = "INTERNAL_LOCAL_TRACK_ID".into();
    state.external.track_id = "INTERNAL_EXTERNAL_TRACK_ID".into();
    state.external.message = "Technical result only.".into();

    let public = serde_json::to_string(&AudioScreeningSummary::from(&state))
        .expect("serialize public audio-screening summary");
    assert!(!public.contains("RAW_CHROMAPRINT_MUST_NOT_REACH_WEBVIEW"));
    assert!(!public.contains("INTERNAL_LOCAL_TRACK_ID"));
    assert!(!public.contains("INTERNAL_EXTERNAL_TRACK_ID"));
    assert!(public.contains("Technical result only."));
}

#[test]
fn audio_screening_coverage_fields_default_for_legacy_settings_and_records() {
    assert_eq!(
        AudioScreeningExternalRecord::default().reference_duration_seconds,
        None
    );

    let settings: AudioScreeningSettings = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "host": "identify-eu-west-1.acrcloud.com",
        "timeoutSeconds": 30
    }))
    .expect("legacy settings");
    assert_eq!(settings.intensity_percent, 5);
    assert!(settings.dynamic_by_track_duration);
    assert_eq!(settings.reference_duration_seconds, 300);

    let record: AudioScreeningExternalRecord = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "provider": "ACRCloud",
        "status": "no_match_detected",
        "message": "Legacy record",
        "trackId": "track-1",
        "sourceEvidenceId": "release-1",
        "sourceRelativePath": "01_RELEASE/release.wav",
        "sourceSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sourceSizeBytes": 1,
        "requestCount": 1,
        "matches": []
    }))
    .expect("legacy external record");
    assert_eq!(record.screening_mode, AudioScreeningMode::SingleSample);
    assert_eq!(record.requested_intensity_percent, 5);
    assert!(record.dynamic_by_track_duration);
    assert_eq!(record.reference_duration_seconds, None);
    assert!(record.samples.is_empty());
    assert_eq!(record.executed_request_count, 0);

    let serialized = serde_json::to_value(&record).expect("serialize legacy record");
    assert!(serialized["referenceDurationSeconds"].is_null());

    let current: AudioScreeningExternalRecord = serde_json::from_value(serde_json::json!({
        "referenceDurationSeconds": 300
    }))
    .expect("current external record");
    assert_eq!(current.reference_duration_seconds, Some(300));

    let summary = AudioScreeningExternalSummary::from(&record);
    assert_eq!(summary.reference_duration_seconds, None);
    let summary_json = serde_json::to_value(summary).expect("serialize external summary");
    assert!(summary_json["referenceDurationSeconds"].is_null());
}

#[test]
fn legacy_acrcloud_samples_default_provider_status_fields() {
    let sample: AudioScreeningSampleRecord = serde_json::from_value(serde_json::json!({
        "sequence": 1,
        "offsetMilliseconds": 0,
        "endOffsetMilliseconds": 12000,
        "durationMilliseconds": 12000,
        "status": "no_match_detected",
        "message": "Legacy sample",
        "matches": []
    }))
    .expect("legacy sample");

    assert!(sample.provider_status_code.is_none());
    assert!(sample.provider_status_message.is_none());
    assert!(sample.provider_api_version.is_none());
    assert!(sample.provider_status_details().is_none());
    assert!(sample.provider_status_compact().is_none());
}

#[test]
fn inactive_code_and_artwork_branches_remove_unclaimed_operations() {
    let mut fields = TrackFields {
        code_based_generation: Some(false),
        code_audio_post_processed: Some(true),
        code_audio_post_processing_operations: vec!["Mixing".into()],
        artwork_origin: "ai_generated".into(),
        human_artwork_process_operations: vec!["Photographed".into()],
        human_artwork_modifications: vec!["Cropping".into()],
        ..TrackFields::default()
    };

    fields.normalize_conditionals();

    assert_eq!(fields.code_audio_post_processed, None);
    assert!(fields.code_audio_post_processing_operations.is_empty());
    assert!(fields.human_artwork_process_operations.is_empty());
    assert!(fields.human_artwork_modifications.is_empty());
}
