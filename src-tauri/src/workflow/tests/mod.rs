use super::*;

fn disclosure_track(origin: &str, applied: Option<bool>) -> TrackRecord {
    TrackRecord {
        id: "track".into(),
        relative_path: "track".into(),
        status: crate::model::TrackStatus::Active,
        workflow_id: "suno-track".into(),
        workflow_version: "1.0".into(),
        profile_snapshot: Profile::default(),
        library: Default::default(),
        field_origins: Default::default(),
        fields: crate::model::TrackFields {
            artwork_origin: origin.into(),
            disclosure_applied: applied,
            ..Default::default()
        },
        audio_screening: Default::default(),
        documents: Default::default(),
        integrity: Default::default(),
        certificate: Default::default(),
        created_at: "2026-08-13T00:00:00Z".into(),
        updated_at: "2026-08-13T00:00:00Z".into(),
        legacy: false,
    }
}

fn verified_evidence(role: EvidenceRole) -> EvidenceItem {
    EvidenceItem {
        id: role.as_str().into(),
        role,
        file_name: "fixture.png".into(),
        relative_path: "05_ARTWORK/fixture.png".into(),
        sha256: Some("a".repeat(64)),
        size_bytes: 42,
        imported_at: "2026-08-15T00:00:00Z".into(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: crate::model::EvidenceProvenance::ManagedCopy,
        derived_from_evidence_id: None,
        generator_version: None,
        generated_disclosure_text: None,
        metadata: Default::default(),
    }
}

fn suno_export(created_date: &str) -> EvidenceItem {
    let mut item = verified_evidence(EvidenceRole::SunoFinalExport);
    item.file_name = "suno.wav".into();
    item.relative_path = "02_SUNO/suno.wav".into();
    item.metadata.suno_studio_detected = true;
    item.metadata.suno_created_timestamp = format!("{created_date}T06:38:06Z");
    item.metadata.suno_created_date = created_date.into();
    item.metadata.suno_id = "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into();
    let raw = format!(
        "made with suno studio; created={created_date}T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828"
    );
    item.metadata.suno_raw_metadata = raw.clone();
    item.metadata.embedded_metadata = vec![crate::model::EmbeddedMetadata {
        key: "ICMT".into(),
        value: raw,
    }];
    item
}

fn document_complete_audio_ai(track: &mut TrackRecord, disclosure: DocumentationAnswer) {
    track.fields.generative_ai_used = Some(true);
    track.fields.audio_ai_system = "Suno".into();
    track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::Yes);
    track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::Yes);
    track.fields.real_person_voice_intentionally_imitated = Some(DocumentationAnswer::No);
    track.fields.real_person_identity_intentionally_represented =
        Some(DocumentationAnswer::NotDocumented);
    track.fields.real_event_represented_as_authentic_recording = Some(DocumentationAnswer::No);
    track
        .fields
        .real_location_institution_event_presented_as_authentic_ai_recording =
        Some(DocumentationAnswer::NotDocumented);
    track.fields.audio_disclosure_applied = Some(disclosure);
    if disclosure == DocumentationAnswer::Yes {
        track.fields.audio_disclosure_locations = vec!["Release description".into()];
        track.fields.audio_disclosure_text = "Contains AI-generated audio".into();
    }
}

mod automation;
mod conditional;
mod configuration;
mod requirements;

use configuration::embedded_config;
