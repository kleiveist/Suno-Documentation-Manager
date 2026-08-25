use super::*;

#[test]
fn byte_identity_is_a_system_verification_over_verified_hashes() {
    let suno = suno_export("2026-08-17");
    let mut release = verified_evidence(EvidenceRole::ReleaseWav);
    release.sha256 = suno.sha256.clone();
    let pairs = byte_identical_pairs(&[suno.clone(), release.clone()]);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].sha256, suno.sha256.clone().unwrap());

    release.verified = false;
    assert!(byte_identical_pairs(&[suno, release]).is_empty());
}

#[test]
fn human_edited_final_artwork_match_is_role_specific_and_informational() {
    let human_edited = verified_evidence(EvidenceRole::HumanEditedArtwork);
    let mut final_artwork = verified_evidence(EvidenceRole::FinalArtwork);
    assert_eq!(
        human_edited_final_artwork_sha256_match(&[human_edited.clone(), final_artwork.clone()]),
        Some(true)
    );
    assert_eq!(
        human_edited_final_artwork_status(&[human_edited.clone(), final_artwork.clone()]),
        "BYTE-IDENTICAL / SHA-256 MATCH"
    );

    final_artwork.sha256 = Some("b".repeat(64));
    assert_eq!(
        human_edited_final_artwork_sha256_match(&[human_edited.clone(), final_artwork.clone()]),
        Some(false)
    );
    assert_eq!(
        human_edited_final_artwork_status(&[human_edited.clone(), final_artwork.clone()]),
        "NO SHA-256 MATCH"
    );
    let mut track = disclosure_track("ai_assisted", Some(false));
    track.fields.human_artwork_modifications = vec!["Color correction".into()];
    assert!(!consistency_issues(&track, &[human_edited, final_artwork])
        .iter()
        .any(|issue| issue.message.contains("SHA-256 MATCH")));
}

#[test]
fn generation_date_differences_do_not_create_manual_conflict_issues() {
    let mut track = disclosure_track("none", None);
    track.fields.suno_final_generation_date = "2026-08-16".into();
    let evidence = vec![suno_export("2026-08-17")];
    let issues = consistency_issues(&track, &evidence);
    assert!(!issues
        .iter()
        .any(|issue| issue.code == "suno_generation_date_conflict"));
}

#[test]
fn stored_suno_metadata_requires_exact_embedded_raw_and_complete_parsed_fields() {
    fn mismatch(track: &TrackRecord, item: EvidenceItem) -> bool {
        consistency_issues(track, &[item])
            .iter()
            .any(|issue| issue.code == "suno_stored_metadata_mismatch" && issue.blocking)
    }

    let track = disclosure_track("none", None);
    let valid = suno_export("2026-08-17");
    assert!(!mismatch(&track, valid.clone()));

    let mut valid_alias = valid.clone();
    let alias_raw =
        "made with suno; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
    valid_alias.metadata.suno_raw_metadata = alias_raw.into();
    valid_alias.metadata.embedded_metadata[0].value = alias_raw.into();
    assert!(!mismatch(&track, valid_alias));

    let mut missing_embedded_raw = valid.clone();
    missing_embedded_raw.metadata.embedded_metadata.clear();
    assert!(mismatch(&track, missing_embedded_raw));

    let mut missing_timestamp = valid.clone();
    missing_timestamp.metadata.suno_created_timestamp.clear();
    assert!(mismatch(&track, missing_timestamp));

    let mut missing_detection_flag = valid.clone();
    missing_detection_flag.metadata.suno_studio_detected = false;
    assert!(mismatch(&track, missing_detection_flag));

    let mut wrong_date = valid.clone();
    wrong_date.metadata.suno_created_date = "2026-08-18".into();
    assert!(mismatch(&track, wrong_date));

    let mut raw_differs_from_embedded = valid.clone();
    raw_differs_from_embedded
        .metadata
        .suno_raw_metadata
        .push(' ');
    assert!(mismatch(&track, raw_differs_from_embedded));

    // The old contains-based check accepted this because the stored UUID
    // occurs in an unrelated note. The structured id= value is different.
    let mut misleading_uuid_substring = valid.clone();
    let misleading_raw = "made with suno studio; created=2026-08-17T06:38:06Z; id=180ee4f0-977b-4db8-8968-e93e3ac9d506; note=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
    misleading_uuid_substring.metadata.suno_raw_metadata = misleading_raw.into();
    misleading_uuid_substring.metadata.embedded_metadata[0].value = misleading_raw.into();
    assert!(mismatch(&track, misleading_uuid_substring));

    // A stored timestamp appearing as a substring is not enough when the
    // actual created= value is not strict RFC3339.
    let mut invalid_timestamp = valid;
    let invalid_raw = "made with suno studio; created=2026-08-17 06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828; note=2026-08-17T06:38:06Z";
    invalid_timestamp.metadata.suno_raw_metadata = invalid_raw.into();
    invalid_timestamp.metadata.embedded_metadata[0].value = invalid_raw.into();
    assert!(mismatch(&track, invalid_timestamp));

    let mut invalid_uuid = suno_export("2026-08-17");
    let invalid_uuid_raw = "made with suno studio; created=2026-08-17T06:38:06Z; id=not-a-uuid; note=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
    invalid_uuid.metadata.suno_raw_metadata = invalid_uuid_raw.into();
    invalid_uuid.metadata.embedded_metadata[0].value = invalid_uuid_raw.into();
    assert!(mismatch(&track, invalid_uuid));
}

#[test]
fn distinct_embedded_suno_records_are_reported_as_ambiguous() {
    let track = disclosure_track("none", None);
    let mut item = suno_export("2026-08-17");
    item.metadata
        .embedded_metadata
        .push(crate::model::EmbeddedMetadata {
        key: "ICMT".into(),
        value:
            "made with suno; created=2026-08-18T06:38:06Z; id=180ee4f0-977b-4db8-8968-e93e3ac9d506"
                .into(),
    });

    let issues = consistency_issues(&track, &[item]);

    assert!(issues
        .iter()
        .any(|issue| issue.code == "suno_metadata_ambiguous" && issue.blocking));
    assert!(!issues
        .iter()
        .any(|issue| issue.code == "suno_stored_metadata_mismatch"));
}

#[test]
fn production_end_differences_do_not_create_manual_conflict_issues() {
    let mut track = disclosure_track("none", None);
    track.fields.production_start_date = "2026-08-01".into();
    track.fields.production_end_date = "2026-08-18".into();
    track.fields.post_export_editing_performed = Some(false);
    let evidence = vec![suno_export("2026-08-17")];

    assert!(!consistency_issues(&track, &evidence)
        .iter()
        .any(|issue| issue.code == "production_end_date_conflict"));
}

#[test]
fn automatic_fact_origin_requires_the_current_evidence_hash_and_timestamp() {
    let mut track = disclosure_track("none", None);
    let suno = suno_export("2026-08-17");
    track.fields.suno_final_generation_date = "2026-08-17".into();
    track.field_origins.suno_final_generation_date = Some(EvidenceDerivedField {
        value: "2026-08-17".into(),
        original_value: "2026-08-17T06:38:06Z".into(),
        evidence_id: suno.id.clone(),
        evidence_sha256: suno.sha256.clone().unwrap(),
    });
    assert_eq!(
        automation_summary(&track, std::slice::from_ref(&suno)).final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    track
        .field_origins
        .suno_final_generation_date
        .as_mut()
        .unwrap()
        .evidence_sha256 = "b".repeat(64);
    let summary = automation_summary(&track, &[suno]);
    assert_eq!(
        summary.final_generation_origin,
        FactOrigin::UserConfirmedFact
    );
    assert!(summary
        .consistency_issues
        .iter()
        .any(|issue| issue.code == "suno_generation_origin_stale"));
}

#[test]
fn human_edited_artwork_requires_a_documented_artwork_process() {
    let mut track = disclosure_track("ai_generated", None);
    let evidence = vec![verified_evidence(EvidenceRole::HumanEditedArtwork)];
    assert!(consistency_issues(&track, &evidence)
        .iter()
        .any(|issue| issue.code == "human_artwork_editing_undocumented"));

    track.fields.artwork_origin = "ai_assisted".into();
    track.fields.human_artwork_modifications = vec!["Color correction".into()];
    assert!(!consistency_issues(&track, &evidence)
        .iter()
        .any(|issue| issue.code == "human_artwork_editing_undocumented"));
}

#[test]
fn verified_terms_evidence_conflicts_with_an_unavailable_claim() {
    let mut track = disclosure_track("none", None);
    track.fields.suno_terms_evidence_not_available = Some(true);
    let evidence = vec![verified_evidence(EvidenceRole::SunoTermsRights)];

    assert!(consistency_issues(&track, &evidence).iter().any(|issue| {
        issue.code == "terms_evidence_availability_conflict"
            && issue.step_id == "evidence_licenses"
            && issue.blocking
    }));
}
