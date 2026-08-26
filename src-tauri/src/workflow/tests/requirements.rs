use super::*;

#[test]
fn vocal_intent_classification_and_final_audio_are_independent() {
    let mut track = disclosure_track("none", None);
    track.fields.instrumental_track = Some(true);
    track.fields.vocal_lyrics_present = Some(true);
    track.fields.vocal_intent = Some(crate::model::VocalIntent::Instrumental);
    track.fields.suno_content_classification = Some(SunoContentClassification::Mixed);
    track.fields.suno_lyrics_content_source = Some(crate::model::SunoLyricsContentSource::Human);
    track.fields.suno_lyrics_field_text = "Vocal line\n[Instrumental break]".into();
    track.fields.suno_style_prompt = "vocal ambient".into();
    track.fields.human_editing_performed = Some(false);

    for key in [
        "human_work.instrumental_answer",
        "human_work.vocal_lyrics_present",
        "human_work.vocal_intent",
        "human_work.suno_content_classification",
        "human_work.suno_lyrics_content_source",
        "human_work.suno_lyrics_field_text",
    ] {
        assert!(
            field_requirement_met(key, &track, &Profile::default(), &[]),
            "{key}"
        );
    }
    let valid = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate instrumental intent with vocal output");
    assert_eq!(
        valid
            .steps
            .iter()
            .find(|step| step.id == "human_work")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );

    track.fields.vocal_intent = Some(crate::model::VocalIntent::Vocal);
    track.fields.vocal_lyrics_present = Some(false);
    let opposite = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate vocal intent without vocal output");
    assert_eq!(
        opposite
            .steps
            .iter()
            .find(|step| step.id == "human_work")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}

#[test]
fn other_requires_a_label_while_empty_makes_content_details_not_applicable() {
    let mut track = disclosure_track("none", None);
    track.fields.instrumental_track = Some(false);
    track.fields.vocal_lyrics_present = Some(true);
    track.fields.vocal_intent = Some(crate::model::VocalIntent::Unspecified);
    track.fields.suno_content_classification = Some(SunoContentClassification::Other);
    track.fields.suno_lyrics_content_source = Some(crate::model::SunoLyricsContentSource::Mixed);
    track.fields.suno_lyrics_field_text = "A documented direction".into();
    track.fields.suno_style_prompt = "vocal pop".into();
    track.fields.human_editing_performed = Some(false);

    assert!(condition_applies("suno_content_other", &track));
    assert!(!field_requirement_met(
        "human_work.suno_lyrics_other_content_type",
        &track,
        &Profile::default(),
        &[]
    ));
    track.fields.suno_lyrics_other_content_type = "Performance direction".into();
    assert!(field_requirement_met(
        "human_work.suno_lyrics_other_content_type",
        &track,
        &Profile::default(),
        &[]
    ));
    let evaluation =
        evaluate(&track, &Profile::default(), &[], &[], &[]).expect("evaluate vocal lyrics case");
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "human_work")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );

    track.fields.suno_content_classification = Some(SunoContentClassification::Empty);
    track.fields.suno_lyrics_content_source = None;
    track.fields.suno_lyrics_field_text.clear();
    track.fields.suno_lyrics_other_content_type.clear();
    assert!(!condition_applies("suno_content_non_empty", &track));
    assert!(!condition_applies("suno_content_other", &track));
    let empty = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate empty generation text field");
    assert_eq!(
        empty
            .steps
            .iter()
            .find(|step| step.id == "human_work")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}

#[test]
fn legacy_lyrics_fields_alone_do_not_answer_the_new_documentation_questions() {
    let mut track = disclosure_track("none", None);
    track.fields.lyrics_source = "instrumental".into();
    track.fields.lyrics_text = "[Intro]\n[Instrumental]".into();

    assert!(!field_requirement_met(
        "human_work.instrumental_answer",
        &track,
        &Profile::default(),
        &[]
    ));
    assert!(!field_requirement_met(
        "human_work.vocal_lyrics_present",
        &track,
        &Profile::default(),
        &[]
    ));
    assert!(!field_requirement_met(
        "human_work.vocal_intent",
        &track,
        &Profile::default(),
        &[]
    ));
    assert!(!field_requirement_met(
        "human_work.suno_content_classification",
        &track,
        &Profile::default(),
        &[]
    ));
    let evaluation = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate legacy-only lyrics fields");
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "human_work")
            .map(|step| &step.status),
        Some(&StepStatus::Blocked)
    );
}

#[test]
fn legacy_plan_at_creation_does_not_satisfy_plan_at_generation() {
    let mut track = disclosure_track("none", None);
    let migrated: crate::model::TrackFields = serde_json::from_value(serde_json::json!({
        "sunoPlanAtCreation": "Historical Pro plan"
    }))
    .expect("historical plan field");
    track.fields.legacy_suno_plan_at_creation = migrated.legacy_suno_plan_at_creation;

    assert_eq!(
        track.fields.legacy_suno_plan_at_creation,
        "Historical Pro plan"
    );
    assert!(track.fields.suno_plan_at_generation.is_empty());
    assert!(!field_requirement_met(
        "suno.plan_at_generation",
        &track,
        &Profile::default(),
        &[]
    ));
}

#[test]
fn commercial_generation_must_be_inside_verified_subscription_coverage() {
    let mut track = disclosure_track("none", None);
    track.fields.suno_final_generation_date = "2026-08-15".into();
    let mut subscription = verified_evidence(EvidenceRole::SubscriptionPayment);
    subscription.coverage_start = Some("2026-08-01".into());
    subscription.coverage_end = Some("2026-08-31".into());
    assert_eq!(
        subscription_generation_coverage(&track, &[subscription.clone()]),
        CoverageStatus::Yes
    );
    subscription.coverage_end = Some("2026-08-14".into());
    assert_eq!(
        subscription_generation_coverage(&track, &[subscription]),
        CoverageStatus::No
    );
    assert_eq!(
        subscription_generation_coverage(&track, &[]),
        CoverageStatus::NotVerified
    );

    let mut malformed = verified_evidence(EvidenceRole::SubscriptionPayment);
    malformed.coverage_start = Some("2026-8-1".into());
    malformed.coverage_end = Some("2026-8-31".into());
    assert_eq!(
        subscription_generation_coverage(&track, &[malformed]),
        CoverageStatus::NotVerified
    );
    track.fields.suno_final_generation_date = "2026-8-15".into();
    assert_eq!(
        subscription_generation_coverage(&track, &[]),
        CoverageStatus::NotVerified
    );
}

#[test]
fn adjacent_subscription_receipts_jointly_cover_the_production_period() {
    let mut track = disclosure_track("none", None);
    track.fields.production_start_date = "2026-07-18".into();
    track.fields.production_end_date = "2026-08-17".into();
    let mut july = verified_evidence(EvidenceRole::SubscriptionPayment);
    july.source_global_evidence_id = Some("july".into());
    july.coverage_start = Some("2026-07-14".into());
    july.coverage_end = Some("2026-08-13".into());
    let mut august = verified_evidence(EvidenceRole::SubscriptionPayment);
    august.source_global_evidence_id = Some("august".into());
    august.coverage_start = Some("2026-08-14".into());
    august.coverage_end = Some("2026-09-13".into());

    assert_eq!(
        subscription_production_coverage(&track, &[july.clone()]),
        CoverageStatus::No
    );
    assert_eq!(
        subscription_production_coverage(&track, &[july, august]),
        CoverageStatus::Yes
    );
}

#[test]
fn mismatching_evidence_filename_requires_explicit_confirmation() {
    let mut track = disclosure_track("none", None);
    track.fields.title = "Gravaty".into();
    let mut release = verified_evidence(EvidenceRole::ReleaseWav);
    release.metadata.original_file_name = "GRAVITY.wav".into();
    let evidence = vec![release];
    assert!(!filename_requirement_met(
        &track,
        &evidence,
        EvidenceRole::ReleaseWav,
        None
    ));
    assert!(filename_requirement_met(
        &track,
        &evidence,
        EvidenceRole::ReleaseWav,
        Some(true)
    ));
    assert_eq!(track.fields.title, "Gravaty");
}

#[test]
fn commercial_terms_require_verified_evidence_with_complete_core_metadata() {
    let mut track = disclosure_track("none", None);
    let config = embedded_config();
    let requirement = config
        .requirements
        .iter()
        .find(|requirement| requirement.key == "evidence_licenses.terms_complete")
        .expect("terms requirement");
    assert!(!requirement_met(
        requirement,
        &track,
        &Profile::default(),
        &[],
        &HashSet::new(),
        &[]
    ));
    track.fields.suno_terms_evidence_not_available = Some(true);
    assert!(!requirement_met(
        requirement,
        &track,
        &Profile::default(),
        &[],
        &HashSet::new(),
        &[]
    ));

    let mut terms = verified_evidence(EvidenceRole::SunoTermsRights);
    assert!(!requirement_met(
        requirement,
        &track,
        &Profile::default(),
        std::slice::from_ref(&terms),
        &HashSet::from(["suno_terms_rights"]),
        &[]
    ));

    terms.metadata.document_title = "Suno Terms of Service".into();
    terms.metadata.provider = "Suno, Inc.".into();
    terms.metadata.retrieval_date = "2026-08-17".into();
    assert!(!requirement_met(
        requirement,
        &track,
        &Profile::default(),
        std::slice::from_ref(&terms),
        &HashSet::from(["suno_terms_rights"]),
        &[]
    ));
    track.fields.suno_terms_evidence_not_available = Some(false);
    assert!(requirement_met(
        requirement,
        &track,
        &Profile::default(),
        std::slice::from_ref(&terms),
        &HashSet::from(["suno_terms_rights"]),
        &[]
    ));

    terms.metadata.retrieval_date = "17.08.2026".into();
    assert!(!requirement_met(
        requirement,
        &track,
        &Profile::default(),
        &[terms],
        &HashSet::from(["suno_terms_rights"]),
        &[]
    ));
}

#[test]
fn code_audio_post_processing_requirements_follow_both_controlling_answers() {
    let mut track = disclosure_track("none", None);
    track.fields.code_based_generation = Some(false);
    assert!(!condition_applies("code_audio_post_processed", &track));

    track.fields.code_based_generation = Some(true);
    track.fields.code_audio_post_processed = Some(false);
    assert!(!condition_applies("code_audio_post_processed", &track));
    assert!(field_requirement_met(
        "source.code_audio_post_processed",
        &track,
        &Profile::default(),
        &[]
    ));

    track.fields.code_audio_post_processed = Some(true);
    assert!(condition_applies("code_audio_post_processed", &track));
    assert!(!field_requirement_met(
        "source.code_audio_post_processing_operations",
        &track,
        &Profile::default(),
        &[]
    ));
    track.fields.code_audio_post_processing_operations =
        vec!["Mixing".into(), "EQ".into(), "Mastering".into()];
    assert!(field_requirement_met(
        "source.code_audio_post_processing_operations",
        &track,
        &Profile::default(),
        &[]
    ));
}

#[test]
fn ai_assisted_artwork_requires_one_or_more_factual_human_changes() {
    let mut track = disclosure_track("ai_assisted", None);
    assert!(condition_applies("ai_assisted_artwork", &track));
    assert!(!field_requirement_met(
        "artwork.human_modifications",
        &track,
        &Profile::default(),
        &[]
    ));
    track.fields.human_artwork_modifications =
        vec!["Prompt written manually".into(), "Cropping".into()];
    assert!(field_requirement_met(
        "artwork.human_modifications",
        &track,
        &Profile::default(),
        &[]
    ));
    track.fields.artwork_origin = "human".into();
    assert!(!condition_applies("ai_assisted_artwork", &track));
}
