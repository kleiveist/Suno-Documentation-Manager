use crate::model::{
    EvidenceItem, Profile, SunoContentClassification, SunoLyricsContentSource, TrackRecord,
};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const INACTIVE_FIXTURE_VALUES: [&str; 8] = [
    "INACTIVE-LYRICS",
    "INACTIVE-EXTERNAL-SOURCE",
    "INACTIVE-EXTERNAL-OWNERSHIP",
    "INACTIVE-SAMPLE-SOURCE",
    "INACTIVE-SAMPLE-OWNERSHIP",
    "INACTIVE-POST-EXPORT-EDIT",
    "INACTIVE-REAL-PERSON-NOTE",
    "INACTIVE-TRADEMARK-NOTE",
];

pub(super) const PRIVATE_FIXTURE_VALUES: [&str; 5] = [
    "private.person@example.invalid",
    "+49 30 555 0100",
    "1990-01-01",
    "/home/fixture-user",
    "password-secret",
];

pub(super) const FORBIDDEN_LEGAL_CLAIMS: [&str; 7] = [
    "guaranteed not to infringe copyright",
    "license is legally sufficient",
    "governmentally certified",
    "we certify legal compliance",
    "copyright ownership is confirmed",
    "owns all copyrights",
    "universal legal requirement",
];

pub(super) const ADOPTION_SENTINEL: &[u8] =
    b"user-authored sentinel document\n\0preserve these exact bytes\n";

pub(super) fn record_with_fields(fields: crate::model::TrackFields) -> TrackRecord {
    TrackRecord {
        id: "conditional-render-test".into(),
        relative_path: "conditional-render-test".into(),
        status: crate::model::TrackStatus::Active,
        workflow_id: "suno-track-documentation".into(),
        workflow_version: "1.3".into(),
        profile_snapshot: Profile::default(),
        library: Default::default(),
        field_origins: Default::default(),
        fields,
        audio_screening: Default::default(),
        documents: crate::model::DocumentState::default(),
        integrity: crate::model::IntegrityState::default(),
        certificate: crate::model::CertificateState::default(),
        created_at: "2026-08-13T00:00:00Z".into(),
        updated_at: "2026-08-13T00:00:00Z".into(),
        legacy: false,
    }
}

pub(super) fn fixture_input() -> (TrackRecord, Profile, Vec<EvidenceItem>) {
    let mut fields = crate::model::TrackFields {
        title: "Golden Signal".into(),
        production_start_date: "2026-02-03".into(),
        production_end_date: "2026-02-05".into(),
        suno_model: "v4.5".into(),
        suno_project_url: "https://suno.example/projects/golden-signal".into(),
        suno_project_version_id: "project-version-golden".into(),
        suno_final_generation_id: "generation-golden".into(),
        suno_final_generation_date: "2026-02-05".into(),
        suno_final_generation_time: "14:35".into(),
        suno_download_export_date: "2026-02-06".into(),
        suno_plan_at_generation: "Pro".into(),
        final_export_date: "2026-02-06".into(),
        instrumental_track: Some(true),
        vocal_lyrics_present: Some(false),
        vocal_intent: Some(crate::model::VocalIntent::Instrumental),
        suno_content_classification: Some(SunoContentClassification::StructureOnly),
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
        suno_lyrics_field_text: "[Intro]\n[Drop]\n[Outro]".into(),
        suno_lyrics_other_content_type: INACTIVE_FIXTURE_VALUES[0].into(),
        suno_style_prompt: "dark synthwave, driving bass, cinematic".into(),
        external_audio_uploaded: Some(false),
        external_audio_source: INACTIVE_FIXTURE_VALUES[1].into(),
        external_audio_ownership: INACTIVE_FIXTURE_VALUES[2].into(),
        own_audio_uploaded: Some(true),
        own_audio_source: "Original field recording".into(),
        own_audio_ownership: "Solely owned by the artist".into(),
        code_based_generation: Some(false),
        code_audio_post_processed: None,
        code_audio_post_processing_operations: Vec::new(),
        code_audio_post_processing_note: String::new(),
        third_party_samples_uploaded: Some(false),
        third_party_sample_source: INACTIVE_FIXTURE_VALUES[3].into(),
        third_party_sample_ownership: INACTIVE_FIXTURE_VALUES[4].into(),
        human_editing_performed: Some(true),
        human_editing_details: "Timing and cuts | EQ".into(),
        post_export_editing_performed: Some(false),
        post_export_editing_details: INACTIVE_FIXTURE_VALUES[5].into(),
        commercial_use_intended: true,
        suno_terms_evidence_not_available: Some(true),
        artwork_origin: "ai_assisted".into(),
        ai_image_service: "Example Image Service".into(),
        human_artwork_process_operations: Vec::new(),
        human_artwork_process_notes: String::new(),
        human_artwork_modifications: vec!["Cropping".into(), "Brightness/contrast adjusted".into()],
        custom_artwork_change: String::new(),
        depicts_real_person: Some(false),
        real_person_notes: INACTIVE_FIXTURE_VALUES[6].into(),
        depicts_real_event: Some(true),
        real_event_notes: "Synthetic night-sky scene.".into(),
        contains_trademark: Some(false),
        trademark_notes: INACTIVE_FIXTURE_VALUES[7].into(),
        disclosure_applied: Some(false),
        disclosure_text: String::new(),
        release_notes: "Streaming master".into(),
        ..Default::default()
    };
    fields.normalize_conditionals();

    let profile = Profile {
        artist_name: "Fixture Artist".into(),
        suno_profile_name: "Fixture Profile".into(),
        suno_handle: "@fixture-artist".into(),
        suno_plan: "Pro".into(),
        subscription_start_date: "2026-01-15".into(),
        default_commercial_use: true,
        default_ai_image_service: "Example Image Service".into(),
        artwork_transparency_policy: "always".into(),
        disclosure_text: "AI-assisted".into(),
        certificate_language: crate::model::CertificateLanguage::En,
    };

    let evidence = vec![
        fixture_evidence(
            "final-artwork",
            crate::model::EvidenceRole::FinalArtwork,
            "05_ARTWORK/final-cover.png",
            '3',
        ),
        fixture_evidence(
            "release-wav",
            crate::model::EvidenceRole::ReleaseWav,
            "01_RELEASE/golden-signal.wav",
            '1',
        ),
        fixture_evidence(
            "ai-original",
            crate::model::EvidenceRole::AiArtworkOriginal,
            "05_ARTWORK/ai-original.png",
            '2',
        ),
    ];

    (record_with_fields(fields), profile, evidence)
}

pub(super) fn fixture_evidence(
    id: &str,
    role: crate::model::EvidenceRole,
    relative_path: &str,
    hash_digit: char,
) -> EvidenceItem {
    EvidenceItem {
        id: id.to_owned(),
        role,
        file_name: "birthday-1990-01-01.png".into(),
        relative_path: relative_path.into(),
        sha256: Some(std::iter::repeat_n(hash_digit, 64).collect()),
        size_bytes: 1_024,
        imported_at: "2026-02-06T12:00:00Z".into(),
        verified: true,
        verification_error: Some("password-secret +49 30 555 0100 /home/fixture-user".into()),
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: crate::model::EvidenceProvenance::ManagedCopy,
        derived_from_evidence_id: None,
        generator_version: None,
        generated_disclosure_text: None,
        metadata: crate::model::EvidenceMetadata {
            original_file_name: Path::new(relative_path)
                .file_name()
                .and_then(|name| name.to_str())
                .expect("portable fixture file name")
                .into(),
            ..Default::default()
        },
    }
}

pub(super) fn write_adoption_sentinel(track_root: &Path) -> PathBuf {
    let relative = Path::new("03_DOCUMENTATION/README.md");
    let path = track_root.join(relative);
    fs::create_dir_all(path.parent().expect("sentinel parent")).expect("create sentinel parent");
    fs::write(&path, ADOPTION_SENTINEL).expect("write adoption sentinel");
    path
}
