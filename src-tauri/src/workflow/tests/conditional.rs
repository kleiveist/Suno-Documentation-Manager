use super::*;

#[test]
fn disclosed_final_artwork_requirement_depends_only_on_ai_origin_and_yes_decision() {
    for origin in ["ai_generated", "ai_assisted", "human", "none"] {
        for policy in ["always", "per_artwork", "none"] {
            for applied in [None, Some(false), Some(true)] {
                let track = disclosure_track(origin, applied);
                let profile = Profile {
                    artwork_transparency_policy: policy.into(),
                    ..Default::default()
                };
                let expected =
                    matches!(origin, "ai_generated" | "ai_assisted") && applied == Some(true);
                assert_eq!(
                    disclosure_required(&track, &profile),
                    expected,
                    "origin={origin}, policy={policy}, applied={applied:?}"
                );
            }
        }
    }
}

#[test]
fn three_negative_artwork_checks_do_not_disable_the_audio_ai_step() {
    let mut track = disclosure_track("ai_assisted", None);
    track.fields.depicts_real_person = Some(false);
    track.fields.depicts_real_event = Some(false);
    track.fields.contains_trademark = Some(false);
    track.fields.generative_ai_used = Some(false);
    let profile = Profile {
        artwork_transparency_policy: "always".into(),
        ..Default::default()
    };

    assert!(content_check_all_negative(&track));
    assert!(!condition_applies("ai_transparency_required", &track));
    assert!(!disclosure_required(&track, &profile));
    assert!(!can_mark_na("ai_transparency", &track).expect("AI step applicability"));

    let evidence = vec![
        verified_evidence(EvidenceRole::AiArtworkOriginal),
        verified_evidence(EvidenceRole::FinalArtwork),
    ];
    let evaluation =
        evaluate(&track, &profile, &evidence, &[], &[]).expect("evaluate negative content checks");
    assert!(evaluation
        .missing
        .iter()
        .any(|item| item.contains("YES-/NO-Entscheidung")));

    track.fields.disclosure_applied = Some(false);
    let evaluation =
        evaluate(&track, &profile, &evidence, &[], &[]).expect("evaluate explicit non-application");
    assert!(!evaluation
        .missing
        .iter()
        .any(|item| item.contains("YES-/NO-Entscheidung")));
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "ai_transparency")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}

#[test]
fn complete_audio_ai_questionnaire_accepts_explicit_not_documented_indicators() {
    let mut track = disclosure_track("none", None);
    document_complete_audio_ai(&mut track, DocumentationAnswer::Yes);

    let evaluation = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate complete audio AI questionnaire");
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "ai_transparency")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}

#[test]
fn each_audio_ai_indicator_requires_an_explicit_tri_state_answer() {
    let mut track = disclosure_track("none", None);
    document_complete_audio_ai(&mut track, DocumentationAnswer::No);
    track.fields.real_person_identity_intentionally_represented = None;

    assert!(!field_requirement_met(
        "ai_transparency.real_person_identity_intentionally_represented",
        &track,
        &Profile::default(),
        &[]
    ));
    let evaluation = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate incomplete indicator questionnaire");
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "ai_transparency")
            .map(|step| &step.status),
        Some(&StepStatus::Blocked)
    );
}

#[test]
fn commercial_generative_audio_blocks_not_documented_disclosure() {
    let mut track = disclosure_track("none", None);
    document_complete_audio_ai(&mut track, DocumentationAnswer::NotDocumented);
    track.fields.commercial_use_intended = true;

    assert!(!field_requirement_met(
        "ai_transparency.audio_disclosure_decision",
        &track,
        &Profile::default(),
        &[]
    ));
    let commercial = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate commercial disclosure");
    assert_eq!(
        commercial
            .steps
            .iter()
            .find(|step| step.id == "ai_transparency")
            .map(|step| &step.status),
        Some(&StepStatus::Blocked)
    );

    track.fields.commercial_use_intended = false;
    assert!(field_requirement_met(
        "ai_transparency.audio_disclosure_decision",
        &track,
        &Profile::default(),
        &[]
    ));
}

#[test]
fn explicit_no_audio_disclosure_is_complete_without_a_reason() {
    let mut track = disclosure_track("none", None);
    document_complete_audio_ai(&mut track, DocumentationAnswer::No);
    track.fields.audio_disclosure_reason.clear();

    let evaluation = evaluate(&track, &Profile::default(), &[], &[], &[])
        .expect("evaluate explicit no disclosure");
    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "ai_transparency")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}

#[test]
fn yes_audio_disclosure_requires_both_location_and_text() {
    let mut track = disclosure_track("none", None);
    document_complete_audio_ai(&mut track, DocumentationAnswer::Yes);
    assert!(field_requirement_met(
        "ai_transparency.audio_disclosure_locations",
        &track,
        &Profile::default(),
        &[]
    ));
    assert!(field_requirement_met(
        "ai_transparency.audio_disclosure_text",
        &track,
        &Profile::default(),
        &[]
    ));

    track.fields.audio_disclosure_locations.clear();
    track.fields.audio_disclosure_text.clear();
    assert!(!field_requirement_met(
        "ai_transparency.audio_disclosure_locations",
        &track,
        &Profile::default(),
        &[]
    ));
    assert!(!field_requirement_met(
        "ai_transparency.audio_disclosure_text",
        &track,
        &Profile::default(),
        &[]
    ));
}

#[test]
fn code_based_generation_requires_source_code_and_generated_audio_evidence() {
    let mut track = disclosure_track("none", None);
    let profile = Profile::default();

    let unanswered = evaluate(&track, &profile, &[], &[], &[])
        .expect("evaluate unanswered code-generation branch");
    assert!(unanswered
        .missing
        .iter()
        .any(|item| item.contains("codebasierten Erzeugung")));

    track.fields.code_based_generation = Some(false);
    let negative = evaluate(&track, &profile, &[], &[], &[])
        .expect("evaluate negative code-generation branch");
    assert!(!negative
        .missing
        .iter()
        .any(|item| item.contains("Quellcode oder die Quelldatei")));
    assert!(!negative
        .missing
        .iter()
        .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

    track.fields.code_based_generation = Some(true);
    let positive_without_file = evaluate(&track, &profile, &[], &[], &[])
        .expect("evaluate positive code-generation branch without evidence");
    assert!(positive_without_file
        .missing
        .iter()
        .any(|item| item.contains("Quellcode oder die Quelldatei")));
    assert!(positive_without_file
        .missing
        .iter()
        .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

    let source_only = vec![verified_evidence(EvidenceRole::SourceCodeFile)];
    let positive_with_source_only = evaluate(&track, &profile, &source_only, &[], &[])
        .expect("evaluate code-generation branch with source only");
    assert!(!positive_with_source_only
        .missing
        .iter()
        .any(|item| item.contains("Quellcode oder die Quelldatei")));
    assert!(positive_with_source_only
        .missing
        .iter()
        .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

    let complete_evidence = vec![
        verified_evidence(EvidenceRole::SourceCodeFile),
        verified_evidence(EvidenceRole::CodeGeneratedAudioFile),
    ];
    let complete = evaluate(&track, &profile, &complete_evidence, &[], &[])
        .expect("evaluate complete code-generation branch");
    assert!(!complete
        .missing
        .iter()
        .any(|item| item.contains("Quellcode oder die Quelldatei")));
    assert!(!complete
        .missing
        .iter()
        .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));
}

#[test]
fn finalize_is_blocked_while_a_preceding_step_is_incomplete() {
    let track = disclosure_track("none", None);
    let evaluation =
        evaluate(&track, &Profile::default(), &[], &[], &[]).expect("evaluate incomplete track");

    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "finalize")
            .map(|step| &step.status),
        Some(&StepStatus::Blocked)
    );
}

#[test]
fn fulfilled_legacy_step_recovers_from_stored_not_verified_status() {
    let mut track = disclosure_track("none", None);
    track.legacy = true;
    track.fields.title = "Recovered Track".into();
    track.fields.production_start_date = "2026-08-01".into();
    track.fields.production_end_date = "2026-08-02".into();
    let profile = Profile {
        artist_name: "Recovered Artist".into(),
        artwork_transparency_policy: "always".into(),
        ..Default::default()
    };
    let stored = StepState {
        id: "track".into(),
        status: StepStatus::NotVerified,
        na_reason: None,
        updated_at: Some("2026-08-01T00:00:00Z".into()),
    };

    let evaluation =
        evaluate(&track, &profile, &[], &[], &[stored]).expect("evaluate recovered legacy step");

    assert_eq!(
        evaluation
            .steps
            .iter()
            .find(|step| step.id == "track")
            .map(|step| &step.status),
        Some(&StepStatus::Pass)
    );
}
