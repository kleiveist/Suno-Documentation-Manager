use super::super::presentation::LEGACY_SELECTION_NOTICE;
use super::super::*;
use super::support::*;
use crate::model::{
    AudioScreeningMode, AudioScreeningProviderStatus, AudioScreeningStatus, DocumentationAnswer,
    EvidenceRole, Profile, SunoContentClassification, SunoLyricsContentSource,
};
use std::collections::BTreeMap;
use std::fs;

#[test]
fn all_documents_are_deterministic_and_exclude_forbidden_content() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let (track, profile, evidence) = fixture_input();

    let generated = generate(workspace.path(), &track, &profile, &evidence, &[], false)
        .expect("generate fixture documents");
    assert_eq!(generated.len(), DOCUMENT_PATHS.len());

    let mut first_generation = BTreeMap::new();
    let mut combined = String::new();
    for relative in DOCUMENT_PATHS {
        let actual = fs::read(workspace.path().join(relative))
            .unwrap_or_else(|error| panic!("read generated document {relative}: {error}"));
        combined.push_str(
            std::str::from_utf8(&actual)
                .unwrap_or_else(|error| panic!("generated UTF-8 for {relative}: {error}")),
        );
        first_generation.insert(relative, actual);
    }
    assert!(
        combined.contains(&format!("Template version: `{TEMPLATE_VERSION}`"))
            || combined.contains(&format!("Template version: {TEMPLATE_VERSION}"))
    );
    assert!(combined.contains("Suno Generation Text Field"));
    assert!(combined.contains("Vocal Lyrics Present [Classification-derived presentation]: NO"));
    assert!(combined.contains("Final generation ID [User-confirmed fact]: generation-golden"));
    assert!(!combined.contains("Suno project/version ID"));
    assert!(!combined.contains("project-version-golden"));
    assert!(
        combined.contains("External timestamp evidence at technical finalization: NOT RECORDED")
    );

    generate(workspace.path(), &track, &profile, &evidence, &[], false)
        .expect("regenerate identical fixture documents");
    for (relative, first_bytes) in first_generation {
        let second_bytes = fs::read(workspace.path().join(relative))
            .unwrap_or_else(|error| panic!("read regenerated document {relative}: {error}"));
        assert_eq!(
            second_bytes, first_bytes,
            "nondeterministic bytes for {relative}"
        );
    }

    for inactive in INACTIVE_FIXTURE_VALUES {
        assert!(
            !combined.contains(inactive),
            "inactive conditional value was rendered: {inactive}"
        );
    }
    for private in PRIVATE_FIXTURE_VALUES {
        assert!(
            !combined.contains(private),
            "private metadata was rendered: {private}"
        );
    }
    let lowercase = combined.to_lowercase();
    for forbidden in FORBIDDEN_LEGAL_CLAIMS {
        assert!(
            !lowercase.contains(forbidden),
            "forbidden legal claim was rendered: {forbidden}"
        );
    }
}

#[test]
fn document_generation_reports_each_managed_output() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let (track, profile, evidence) = fixture_input();
    let mut progress_events = Vec::new();

    let generated = generate_with_progress(
        workspace.path(),
        &track,
        &profile,
        &evidence,
        &[],
        false,
        &mut |progress| progress_events.push(progress),
    )
    .expect("generate fixture documents with progress");

    assert_eq!(generated.len(), DOCUMENT_PATHS.len());
    for expected_stage in [
        "preparing_documents",
        "rendering_documents",
        "writing_documents",
        "finalizing_documents",
    ] {
        assert!(
            progress_events
                .iter()
                .any(|progress| progress.stage == expected_stage),
            "missing progress stage {expected_stage}"
        );
    }

    for relative in DOCUMENT_PATHS {
        assert!(
            progress_events.iter().any(|progress| {
                progress.stage == "writing_documents"
                    && progress.current_file.as_deref() == Some(relative)
            }),
            "missing progress event for {relative}"
        );
    }

    let completed = progress_events
        .iter()
        .find(|progress| progress.stage == "finalizing_documents")
        .expect("final document progress");
    assert_eq!(completed.processed_files, DOCUMENT_PATHS.len() as u32);
    assert_eq!(completed.total_files, DOCUMENT_PATHS.len() as u32);
}

#[test]
fn generation_moves_managed_lyrics_and_style_documents_into_suno_folder() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let legacy_directory = workspace.path().join("03_DOCUMENTATION");
    fs::create_dir_all(&legacy_directory).expect("legacy document directory");
    fs::write(
        legacy_directory.join("Lyrics.md"),
        format!("{}# Legacy lyrics\n", marker()),
    )
    .expect("legacy lyrics");
    fs::write(
        legacy_directory.join("Styles.md"),
        format!("{}# Legacy styles\n", marker()),
    )
    .expect("legacy styles");
    let (track, profile, evidence) = fixture_input();

    generate(workspace.path(), &track, &profile, &evidence, &[], false)
        .expect("generate migrated documents");

    assert!(workspace.path().join("02_SUNO/Lyrics.md").is_file());
    assert!(workspace.path().join("02_SUNO/Style.md").is_file());
    assert!(!legacy_directory.join("Lyrics.md").exists());
    assert!(!legacy_directory.join("Styles.md").exists());
}

#[test]
fn guided_german_ui_labels_render_as_english_document_values() {
    let (mut track, profile, mut evidence) = fixture_input();
    track.fields.external_audio_uploaded = Some(true);
    track.fields.external_audio_source = "Lizenzierte Sample-Bibliothek".into();
    track.fields.external_audio_ownership = "Lizenz für kommerzielle Nutzung".into();
    track.fields.code_based_generation = Some(true);
    track.fields.human_editing_details = "Timing und Cuts | Lautheitsanpassung".into();
    track.fields.post_export_editing_performed = Some(true);
    track.fields.post_export_editing_details = "Schnitt | Mastering".into();
    track.fields.release_notes = "Originale Suno-Fassung | Radio Edit".into();
    evidence.push(fixture_evidence(
        "source-code",
        crate::model::EvidenceRole::SourceCodeFile,
        "02_SUNO/generator.py",
        '4',
    ));
    evidence.push(fixture_evidence(
        "code-generated-audio",
        crate::model::EvidenceRole::CodeGeneratedAudioFile,
        "02_SUNO/generated.wav",
        '5',
    ));

    let rendered = render(&track, &profile, &evidence, &[]);
    let combined = rendered.values().cloned().collect::<String>();

    for expected in [
        "External audio source category: Audio from a licensed sample library",
        "External audio rights basis: Commercial-use license",
        "Source-code evidence: 02_SUNO/generator.py",
        "Code-generated audio evidence: 02_SUNO/generated.wav",
        "Confirmed human work: Timing and cuts, Loudness adjustment",
        "Confirmed desktop-PC editing work: Editing and cuts, Mastering",
        "Release notes: Original Suno version, Radio edit",
    ] {
        assert!(
            combined.contains(expected),
            "missing English value: {expected}"
        );
    }
    for german_label in [
        "Lizenzierte Sample-Bibliothek",
        "Lizenz für kommerzielle Nutzung",
        "Timing und Cuts",
        "Lautheitsanpassung",
        "Originale Suno-Fassung",
    ] {
        assert!(
            !combined.contains(german_label),
            "localized UI label leaked into generated documents: {german_label}"
        );
    }
}

#[test]
fn conditional_post_processing_and_artwork_facts_render_without_legal_inference() {
    let (mut track, profile, evidence) = fixture_input();
    track.fields.code_based_generation = Some(true);
    track.fields.code_audio_post_processed = Some(false);
    track.fields.code_audio_post_processing_operations = vec!["STALE-MIXING".into()];
    track.fields.code_audio_post_processing_note = "STALE-NOTE".into();

    let rendered = render(&track, &profile, &evidence, &[]);
    let readme = &rendered["03_DOCUMENTATION/README.md"];
    assert!(readme.contains("Post-processing performed: NO"));
    assert!(!readme.contains("Post-processing operations:"));
    assert!(!readme.contains("STALE-MIXING"));
    assert!(!readme.contains("STALE-NOTE"));

    track.fields.code_audio_post_processed = Some(true);
    track.fields.code_audio_post_processing_operations = vec![
        "Mixing".into(),
        "EQ".into(),
        "Compression".into(),
        "Mastering".into(),
        "Other post-processing".into(),
    ];
    track.fields.code_audio_post_processing_note = "Manual spectral repair".into();
    track.fields.artwork_origin = "human".into();
    track.fields.human_artwork_process_operations = vec![
        "Photographed".into(),
        "Retouching".into(),
        "Typography added".into(),
    ];
    track.fields.human_artwork_process_notes = "Manual darkroom scan".into();
    track.fields.depicts_real_person = Some(true);
    track.fields.real_person_notes = "The performing artist".into();
    track.fields.depicts_real_event = Some(false);
    track.fields.contains_trademark = Some(true);
    track.fields.trademark_notes = "A user-supplied company logo".into();

    let rendered = render(&track, &profile, &evidence, &[]);
    let combined = rendered.values().cloned().collect::<String>();
    for factual_statement in [
        "Post-processing performed: YES",
        "Post-processing operations: Mixing, EQ, Compression, Mastering, Other post-processing",
        "Other post-processing note: Manual spectral repair",
        "Human process operations: Photographed, Retouching, Typography added",
        "Human process notes: Manual darkroom scan",
        "Real person intentionally depicted: YES",
        "Real-person note: The performing artist",
        "Real event represented as authentic: NO",
        "Trademark or company logo reproduced: YES",
        "Trademark/logo note: A user-supplied company logo",
    ] {
        assert!(
            combined.contains(factual_statement),
            "missing factual document statement: {factual_statement}"
        );
    }
    let lowercase = combined.to_lowercase();
    for forbidden in FORBIDDEN_LEGAL_CLAIMS {
        assert!(!lowercase.contains(forbidden));
    }
}

#[test]
fn explicit_vocal_and_structure_fields_render_without_legacy_inference() {
    let (mut track, profile, evidence) = fixture_input();
    track.fields.instrumental_track = Some(false);
    track.fields.vocal_lyrics_present = Some(true);
    track.fields.vocal_intent = Some(crate::model::VocalIntent::Vocal);
    track.fields.suno_content_classification = Some(SunoContentClassification::VocalLyricsOnly);
    track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
    track.fields.suno_lyrics_field_text = "Explicit vocal text".into();
    track.fields.lyrics_source.clear();
    track.fields.lyrics_text.clear();

    let rendered = render(&track, &profile, &evidence, &[]);
    let lyrics = &rendered["02_SUNO/Lyrics.md"];
    assert!(lyrics.contains("# Suno Generation Text Field"));
    assert!(lyrics.contains("Suno Instrumental Mode Selected [User-confirmed fact]: NO"));
    assert!(lyrics.contains("Content Classification [User-confirmed fact]: VOCAL_LYRICS_ONLY"));
    assert!(lyrics.contains("Vocal Intent [User-confirmed fact]: VOCAL"));
    assert!(lyrics.contains("Final Audio Contains Vocals [User-confirmed fact]: YES"));
    assert!(lyrics.contains("Explicit vocal text"));
    assert!(!lyrics.contains("Unclassified legacy lyrics data"));
}

#[test]
fn complete_terms_metadata_uses_same_evidence_id_in_summary_and_register() {
    let (track, profile, mut evidence) = fixture_input();
    let mut terms = fixture_evidence(
        "terms-record",
        EvidenceRole::SunoTermsRights,
        "04_LICENSES/suno-terms.pdf",
        '8',
    );
    terms.metadata.document_title = "Suno Terms of Service".into();
    terms.metadata.provider = "Suno, Inc.".into();
    terms.metadata.source_url = "https://suno.example/terms".into();
    terms.metadata.retrieval_date = "2026-02-01".into();
    terms.metadata.effective_date = "2026-01-01".into();
    terms.metadata.applicable_production_period = "2026-02-03 to 2026-02-05".into();
    terms.metadata.factual_note = "Locally archived copy".into();
    terms.verification_error = None;
    let evidence_id = terms.id.clone();
    evidence.push(terms);

    let rendered = render(&track, &profile, &evidence, &[]);
    let license = &rendered["04_LICENSES/suno_account_and_license.md"];
    assert!(license.contains("Terms evidence exists [System verification]: YES"));
    assert!(license.contains(&format!("Terms evidence IDs: {evidence_id}")));
    assert!(license.contains(&format!("Evidence ID `{evidence_id}`")));
    for value in [
        "Suno Terms of Service",
        "Suno, Inc.",
        "https://suno.example/terms",
        "2026-02-01",
        "2026-01-01",
        "2026-02-03 to 2026-02-05",
        "Locally archived copy",
        "04_LICENSES/suno-terms.pdf",
    ] {
        assert!(license.contains(value), "missing terms value: {value}");
    }
}

#[test]
fn managed_documents_do_not_call_unverified_terms_or_missing_hash_pairs_verified() {
    let (track, profile, mut evidence) = fixture_input();
    let unverified_terms = fixture_evidence(
        "unverified-terms",
        EvidenceRole::SunoTermsRights,
        "04_LICENSES/unverified-terms.pdf",
        '8',
    );
    evidence.push(unverified_terms);

    let rendered = render(&track, &profile, &evidence, &[]);
    assert!(rendered["04_LICENSES/suno_account_and_license.md"]
        .contains("Terms evidence exists [System verification]: NOT VERIFIED"));
    for path in ["02_SUNO/suno_project.txt", "03_DOCUMENTATION/AI_USAGE.md"] {
        assert!(rendered[path].contains(
            "Release identical to Suno final export [System verification]: NOT VERIFIED"
        ));
    }

    let release = evidence
        .iter_mut()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release fixture");
    release.verification_error = None;
    let mut suno = fixture_evidence(
        "suno-export",
        EvidenceRole::SunoFinalExport,
        "02_SUNO/suno-export.wav",
        '1',
    );
    suno.verification_error = None;
    evidence.push(suno);
    let rendered = render(&track, &profile, &evidence, &[]);
    assert!(rendered["02_SUNO/suno_project.txt"]
        .contains("Release identical to Suno final export [System verification]: YES"));

    evidence
        .iter_mut()
        .find(|item| item.role == EvidenceRole::SunoFinalExport)
        .expect("Suno export fixture")
        .sha256 = Some("2".repeat(64));
    let rendered = render(&track, &profile, &evidence, &[]);
    assert!(rendered["02_SUNO/suno_project.txt"]
        .contains("Release identical to Suno final export [System verification]: NO"));
}

#[test]
fn audio_transparency_distinguishes_complete_no_and_not_documented() {
    let (mut track, profile, evidence) = fixture_input();
    track.fields.generative_ai_used = Some(true);
    track.fields.audio_ai_system = "Suno".into();
    track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::No);
    track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::Yes);
    track.fields.real_person_voice_intentionally_imitated = Some(DocumentationAnswer::No);
    track.fields.real_person_identity_intentionally_represented = Some(DocumentationAnswer::No);
    track.fields.real_event_represented_as_authentic_recording = Some(DocumentationAnswer::No);
    track
        .fields
        .real_location_institution_event_presented_as_authentic_ai_recording =
        Some(DocumentationAnswer::No);
    track.fields.audio_disclosure_applied = Some(DocumentationAnswer::No);
    track.fields.audio_disclosure_reason = "Documented user reason".into();

    let rendered = render(&track, &profile, &evidence, &[]);
    let ai = &rendered["03_DOCUMENTATION/AI_USAGE.md"];
    assert!(ai.contains("Generative AI used [User-confirmed fact]: YES"));
    assert!(ai.contains("AI-assisted audio elements [User-confirmed fact]: NO"));
    assert!(ai.contains("Disclosure locations [User-confirmed fact]: N/A"));
    assert!(ai.contains("Documented user reason"));
    assert!(ai.contains("Documented deepfake-related indicators: none recorded"));
    assert!(!ai.contains("AI Act compliant"));

    track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::NotDocumented);
    let rendered = render(&track, &profile, &evidence, &[]);
    assert!(rendered["03_DOCUMENTATION/AI_USAGE.md"]
        .contains("AI-generated audio elements [User-confirmed fact]: NOT DOCUMENTED"));
}

#[test]
fn artwork_documents_record_explicit_no_hash_match_and_import_timestamp_scope() {
    let (track, profile, mut evidence) = fixture_input();
    let final_artwork = evidence
        .iter_mut()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .expect("final artwork fixture");
    final_artwork.verification_error = None;
    let final_hash = final_artwork.sha256.clone();
    let mut human_edited = fixture_evidence(
        "human-edited",
        EvidenceRole::HumanEditedArtwork,
        "05_ARTWORK/GOLDEN_SIGNAL_HUMAN_EDITED.png",
        '3',
    );
    human_edited.verification_error = None;
    human_edited.sha256 = final_hash;
    evidence.push(human_edited);

    let rendered = render(&track, &profile, &evidence, &[]);
    for path in [
        "03_DOCUMENTATION/AI_USAGE.md",
        "04_LICENSES/openai_image_generation.md",
        "05_ARTWORK/artwork_process.md",
    ] {
        assert!(rendered[path].contains(ARTWORK_IMPORT_TIMESTAMP_NOTICE));
    }
    assert!(rendered["03_DOCUMENTATION/AI_USAGE.md"]
        .contains("Visible disclosure deliberately not applied: YES"));
    assert!(rendered["05_ARTWORK/artwork_process.md"]
        .contains("Disclosure deliberately not applied: YES"));
    assert!(rendered["05_ARTWORK/artwork_process.md"].contains("BYTE-IDENTICAL / SHA-256 MATCH"));
}

#[test]
fn legacy_external_timestamp_is_excluded_from_phase_one_documents_and_fingerprint() {
    let (track, profile, mut evidence) = fixture_input();
    let without_timestamp = input_fingerprint(&track, &profile, &evidence)
        .expect("phase-one fingerprint without timestamp");
    let mut timestamp = fixture_evidence(
        "legacy-timestamp",
        EvidenceRole::ExternalTimestamp,
        "03_DOCUMENTATION/legacy-timestamp.pdf",
        '9',
    );
    timestamp.metadata.provider = "MUST-NOT-APPEAR-TIMESTAMP-PROVIDER".into();
    timestamp.metadata.external_timestamp = "2026-02-06T12:00:00Z".into();
    evidence.push(timestamp);

    assert_eq!(
        input_fingerprint(&track, &profile, &evidence)
            .expect("phase-one fingerprint with ignored timestamp"),
        without_timestamp
    );
    let rendered = render(&track, &profile, &evidence, &[]);
    let combined = rendered.values().cloned().collect::<String>();
    assert!(!combined.contains("MUST-NOT-APPEAR-TIMESTAMP-PROVIDER"));
    assert!(!combined.contains("legacy-timestamp.pdf"));
    assert!(
        combined.contains("External timestamp evidence at technical finalization: NOT RECORDED")
    );
}

#[test]
fn audio_screening_changes_freshness_and_readme_without_raw_fingerprint() {
    let (mut track, profile, evidence) = fixture_input();
    let before = input_fingerprint(&track, &profile, &evidence).expect("initial fingerprint");
    track.audio_screening.local.status = AudioScreeningStatus::FingerprintGenerated;
    track.audio_screening.local.engine_version = "1.6.1".into();
    track.audio_screening.local.source_evidence_id = "release-wav".into();
    track.audio_screening.local.source_relative_path = "01_RELEASE/golden-signal.wav".into();
    track.audio_screening.local.source_sha256 = "1".repeat(64);
    track.audio_screening.local.fingerprint = "RAW_FINGERPRINT_MUST_NOT_RENDER".into();
    track.audio_screening.local.message = "ACCESS_SECRET_MUST_NOT_RENDER".into();
    track.audio_screening.local.artifact_relative_path =
        "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json".into();
    track.audio_screening.local.artifact_sha256 = "2".repeat(64);

    assert_ne!(
        before,
        input_fingerprint(&track, &profile, &evidence).expect("screening fingerprint")
    );
    let rendered = render(&track, &profile, &evidence, &[]);
    let readme = &rendered["03_DOCUMENTATION/README.md"];
    assert!(readme.contains("## Pre-release audio screening"));
    assert!(readme.contains("FINGERPRINT GENERATED"));
    assert!(readme.contains("LOCAL_FINGERPRINT.json"));
    assert!(!readme.contains("RAW_FINGERPRINT_MUST_NOT_RENDER"));
    assert!(!readme.contains("ACCESS_SECRET_MUST_NOT_RENDER"));
}

#[test]
fn readme_documents_multi_sample_screening_plan_and_each_sample_without_messages() {
    let (mut track, profile, evidence) = fixture_input();
    let external = &mut track.audio_screening.external;
    external.screening_mode = AudioScreeningMode::MultiSample;
    external.status = AudioScreeningStatus::NoMatchDetected;
    external.provider_status = AudioScreeningProviderStatus::Ready;
    external.requested_intensity_percent = 25;
    external.dynamic_by_track_duration = false;
    external.reference_duration_seconds = Some(300);
    external.source_duration_milliseconds = Some(646_000);
    external.target_duration_milliseconds = 75_000;
    external.planned_request_count = 7;
    external.executed_request_count = 2;
    external.request_count = 2;
    external.unique_sample_count = 2;
    external.duplicate_sample_count = 0;
    external.overlapping_sample_count = 0;
    external.unique_sample_duration_milliseconds = 24_000;
    external.track_coverage_percent = 3.72;
    external.samples = vec![
        crate::model::AudioScreeningSampleRecord {
            sequence: 1,
            offset_milliseconds: 30_000,
            end_offset_milliseconds: 42_000,
            duration_milliseconds: 12_000,
            status: AudioScreeningStatus::NoMatchDetected,
            message: "RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER".into(),
            provider_status_code: Some(1001),
            provider_status_message: Some("No result".into()),
            provider_api_version: Some("1.0".into()),
            matches: Vec::new(),
            response_relative_path: Some(
                "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE_01.json".into(),
            ),
            response_sha256: Some("a".repeat(64)),
        },
        crate::model::AudioScreeningSampleRecord {
            sequence: 2,
            offset_milliseconds: 95_000,
            end_offset_milliseconds: 107_000,
            duration_milliseconds: 12_000,
            status: AudioScreeningStatus::MatchDetected,
            message: "SECOND_RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER".into(),
            provider_status_code: Some(0),
            provider_status_message: Some("Success".into()),
            provider_api_version: Some("1.0".into()),
            matches: (0..6)
                .map(|index| crate::model::AudioScreeningMatch {
                    title: format!("Sample match title {index}"),
                    artists: vec![format!("Sample match artist {index}")],
                    ..Default::default()
                })
                .collect(),
            response_relative_path: Some(
                "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE_02.json".into(),
            ),
            response_sha256: Some("b".repeat(64)),
        },
    ];

    let rendered = render(&track, &profile, &evidence, &[]);
    let readme = &rendered["03_DOCUMENTATION/README.md"];
    for expected in [
            "External screening mode [System value]: MULTI-SAMPLE",
            "Requested coverage [System value]: 25 %",
            "Calculation mode [System value]: FIXED REFERENCE DURATION",
            "Reference duration (seconds) [System value]: 300",
            "Planned requests [System value]: 7",
            "Executed requests [System verification]: 2",
            "Unique samples [System verification]: 2",
            "Duplicate samples [System verification]: 0",
            "Overlapping samples [System verification]: 0",
            "Unique sampled duration (ms) [System verification]: 24000",
            "Track coverage (%) [System verification]: 3.72",
            "Overall result [System verification]: NO MATCH DETECTED",
            "Sample 01 [System verification]: Offset 30000 ms · End offset 42000 ms · Duration 12000 ms · Result NO MATCH",
            "Provider Code: 1001 · Provider Message: No result · Provider Version: 1.0",
            "Sample 02 [System verification]: Offset 95000 ms · End offset 107000 ms · Duration 12000 ms · Result MATCH DETECTED",
            "Sample match title 5 — Sample match artist 5",
            "Audio-screening results are technical comparison records only.",
        ] {
            assert!(readme.contains(expected), "README omitted {expected}");
        }
    assert!(!readme.contains("Reference duration (seconds) [System value]: N/A"));
    for forbidden in [
        "RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER",
        "SECOND_RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER",
    ] {
        assert!(!readme.contains(forbidden), "README leaked {forbidden}");
    }
}

#[test]
fn unknown_legacy_selection_is_retained_in_data_but_not_copied_into_english_documents() {
    let (mut track, profile, evidence) = fixture_input();
    track.fields.own_audio_source = "Historischer deutscher Freitext".into();

    let rendered = render(&track, &profile, &evidence, &[]);
    let readme = &rendered["03_DOCUMENTATION/README.md"];

    assert!(!readme.contains("Historischer deutscher Freitext"));
    assert!(readme.contains(LEGACY_SELECTION_NOTICE));
    assert_eq!(
        track.fields.own_audio_source,
        "Historischer deutscher Freitext"
    );
}

#[test]
fn rendering_and_fingerprints_ignore_inactive_conditional_values() {
    let stale_values = [
        "STALE-LYRICS",
        "STALE-EXTERNAL-SOURCE",
        "STALE-EXTERNAL-RIGHTS",
        "STALE-OWN-SOURCE",
        "STALE-OWN-RIGHTS",
        "STALE-SAMPLE-SOURCE",
        "STALE-SAMPLE-RIGHTS",
        "STALE-HUMAN-EDIT",
        "STALE-POST-EDIT",
        "STALE-AI-SERVICE",
        "STALE-ARTWORK-EDIT",
        "STALE-PERSON-NOTE",
        "STALE-EVENT-NOTE",
        "STALE-TRADEMARK-NOTE",
        "STALE-DISCLOSURE",
    ];
    let fields = crate::model::TrackFields {
        title: "Conditional Track".into(),
        suno_content_classification: Some(SunoContentClassification::Empty),
        suno_lyrics_field_text: stale_values[0].into(),
        external_audio_uploaded: Some(false),
        external_audio_source: stale_values[1].into(),
        external_audio_ownership: stale_values[2].into(),
        own_audio_uploaded: Some(false),
        own_audio_source: stale_values[3].into(),
        own_audio_ownership: stale_values[4].into(),
        third_party_samples_uploaded: Some(false),
        third_party_sample_source: stale_values[5].into(),
        third_party_sample_ownership: stale_values[6].into(),
        human_editing_performed: Some(false),
        human_editing_details: stale_values[7].into(),
        post_export_editing_performed: Some(false),
        post_export_editing_details: stale_values[8].into(),
        artwork_origin: "none".into(),
        ai_image_service: stale_values[9].into(),
        human_artwork_modifications: vec![stale_values[10].into()],
        depicts_real_person: Some(false),
        real_person_notes: stale_values[11].into(),
        depicts_real_event: Some(false),
        real_event_notes: stale_values[12].into(),
        contains_trademark: Some(false),
        trademark_notes: stale_values[13].into(),
        disclosure_applied: Some(true),
        disclosure_text: stale_values[14].into(),
        ..crate::model::TrackFields::default()
    };
    let stale = record_with_fields(fields);
    let rendered = render(&stale, &Profile::default(), &[], &[]);
    let combined = rendered.values().cloned().collect::<String>();

    for stale_value in stale_values {
        assert!(
            !combined.contains(stale_value),
            "inactive value was rendered: {stale_value}"
        );
    }
    assert!(!rendered["02_SUNO/Lyrics.md"].contains("## Text"));
    assert!(!rendered["03_DOCUMENTATION/README.md"].contains("Confirmed human work"));
    assert!(!rendered["03_DOCUMENTATION/README.md"].contains("Confirmed post-export work"));
    assert!(!rendered["03_DOCUMENTATION/AI_USAGE.md"].contains("AI service:"));
    assert!(!rendered["05_ARTWORK/artwork_process.md"].contains("Real-person note:"));

    let clean = record_with_fields(stale.fields.normalized_conditionals());
    assert_eq!(
        input_fingerprint(&stale, &Profile::default(), &[]).expect("stale fingerprint"),
        input_fingerprint(&clean, &Profile::default(), &[]).expect("clean fingerprint")
    );
}
