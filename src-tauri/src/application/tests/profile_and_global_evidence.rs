use super::*;

#[test]
fn profile_updates_refresh_open_tracks_but_preserve_finalized_snapshots() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let original_profile = complete_profile();
    app.update_profile(original_profile.clone())
        .expect("original profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("finalized-fixtures"),
        "Frozen Profile",
    );
    let active = app
        .create_track(CreateTrackInput {
            title: "Current Profile".into(),
            production_start_date: "2026-08-10".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("active track");
    app.generate_documents(&active.id, false)
        .expect("initial active documents");

    let mut changed_profile = original_profile.clone();
    changed_profile.artist_name = "Updated Artist".into();
    changed_profile.suno_profile_name = "updated-profile".into();
    changed_profile.suno_handle = "@updated".into();
    app.update_profile(changed_profile.clone())
        .expect("updated profile");

    let refreshed = app.load_track(&active.id).expect("refreshed active track");
    assert_eq!(refreshed.profile_snapshot, changed_profile);
    assert!(!refreshed.documents.current);
    let frozen = app
        .load_track(&finalized.id)
        .expect("unchanged finalized track");
    assert_eq!(frozen.profile_snapshot, original_profile);

    app.generate_documents(&active.id, false)
        .expect("regenerated active documents");
    let readme = fs::read_to_string(
        app.root()
            .join(&refreshed.relative_path)
            .join("03_DOCUMENTATION/README.md"),
    )
    .expect("generated README");
    assert!(readme.contains("- Artist: Updated Artist"));
    assert!(readme.contains("- Suno profile: updated-profile"));
    assert!(readme.contains("- Suno handle: @updated"));
    assert!(!readme.contains("Artist: Not documented"));
}

#[test]
fn certificate_language_change_preserves_open_outputs_and_freezes_finalization_options() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let original_profile = complete_profile();
    app.update_profile(original_profile.clone())
        .expect("original profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("ready-fixtures"),
        "Language-only Profile Update",
    );
    let before = app.load_track(&ready.id).expect("ready track");
    let documents_before = serde_json::to_value(&before.documents).expect("document state");
    let mut integrity_before = serde_json::to_value(&before.integrity).expect("integrity state");
    // Loading re-runs the integrity check and therefore refreshes only
    // this observation timestamp; the protected state must stay equal.
    integrity_before["verifiedAt"] = serde_json::Value::Null;
    assert!(before.documents.current);
    assert!(before.integrity.verified);

    let mut german_profile = original_profile.clone();
    german_profile.certificate_language = CertificateLanguage::De;
    app.update_profile(german_profile.clone())
        .expect("language-only profile update");

    let unchanged = app.load_track(&ready.id).expect("unchanged ready track");
    assert_eq!(app.profile().expect("saved profile"), german_profile);
    assert_eq!(unchanged.profile_snapshot, original_profile);
    assert_eq!(
        serde_json::to_value(&unchanged.documents).expect("document state after update"),
        documents_before
    );
    let mut integrity_after =
        serde_json::to_value(&unchanged.integrity).expect("integrity state after update");
    integrity_after["verifiedAt"] = serde_json::Value::Null;
    assert_eq!(integrity_after, integrity_before);

    let finalized = app
        .finalize_track_with_options(&ready.id, FinalizeOptions { bilingual: false })
        .expect("finalization with fixed dual-language PDF output")
        .track
        .expect("finalized detail");
    assert_eq!(
        finalized.certificate.certificate_language,
        CertificateLanguage::De
    );
    assert!(finalized.certificate.bilingual);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            app.root()
                .join(&finalized.relative_path)
                .join(certificate::MANIFEST_FILE),
        )
        .expect("finalized evidence manifest"),
    )
    .expect("manifest JSON");
    assert_eq!(manifest["certificate"]["rendering"]["language"], "de");
    assert_eq!(manifest["certificate"]["rendering"]["bilingual"], true);

    let mut english_profile = german_profile;
    english_profile.certificate_language = CertificateLanguage::En;
    app.update_profile(english_profile)
        .expect("subsequent language-only profile update");
    let frozen = app.load_track(&ready.id).expect("frozen finalized track");
    assert_eq!(frozen.status, TrackStatus::Finalized);
    assert_eq!(
        frozen.certificate.certificate_language,
        CertificateLanguage::De
    );
    assert!(frozen.certificate.bilingual);
}

#[test]
fn reopening_assigns_saved_global_profile_to_existing_legacy_track() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let profile = complete_profile();
    app.update_profile(profile.clone())
        .expect("saved global profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("legacy-profile-fixtures"),
        "Legacy Global Profile",
    );
    let mut stale = app
        .persistence
        .track(&ready.id)
        .expect("stored ready track");
    stale.profile_snapshot = Profile::default();
    stale.legacy = true;
    app.persistence
        .save_track(&stale)
        .expect("stale track fixture");
    for step_id in ["track", "suno", "integrity", "finalize"] {
        app.persistence
            .save_step(
                &stale.id,
                &StepState {
                    id: step_id.into(),
                    status: StepStatus::NotVerified,
                    na_reason: None,
                    updated_at: Some("2026-08-14T00:00:00Z".into()),
                },
            )
            .expect("stored legacy status");
    }
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
    let recovered = reopened.load_track(&ready.id).expect("recovered track");

    assert_eq!(recovered.profile_snapshot, profile);
    for step_id in ["track", "suno", "integrity", "finalize"] {
        assert_eq!(
            recovered
                .steps
                .iter()
                .find(|step| step.id == step_id)
                .map(|step| &step.status),
            Some(&StepStatus::Pass),
            "{step_id} did not recover from NOT_VERIFIED"
        );
    }
    let validation = reopened
        .validate_track(&ready.id)
        .expect("validate recovered track");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
}

#[test]
fn monthly_subscription_coverage_uses_one_calendar_month() {
    assert_eq!(
        subscription_coverage_end("2026-08-15", SubscriptionBillingCycle::Monthly)
            .expect("monthly coverage"),
        "2026-09-14"
    );
    assert_eq!(
        serde_json::to_string(&SubscriptionBillingCycle::Monthly)
            .expect("monthly billing-cycle serialization"),
        "\"monthly\""
    );
}

#[test]
fn annual_subscription_coverage_uses_twelve_calendar_months() {
    assert_eq!(
        subscription_coverage_end("2026-08-15", SubscriptionBillingCycle::Annual)
            .expect("annual coverage"),
        "2027-08-14"
    );
    assert_eq!(
        serde_json::to_string(&SubscriptionBillingCycle::Annual)
            .expect("annual billing-cycle serialization"),
        "\"annual\""
    );
}

#[test]
fn monthly_subscription_coverage_clamps_month_end_before_subtracting_a_day() {
    assert_eq!(
        subscription_coverage_end("2026-01-31", SubscriptionBillingCycle::Monthly)
            .expect("month-end coverage"),
        "2026-02-27"
    );
}

#[test]
fn subscription_coverage_handles_leap_years() {
    assert_eq!(
        subscription_coverage_end("2024-02-01", SubscriptionBillingCycle::Monthly)
            .expect("leap-month coverage"),
        "2024-02-29"
    );
    assert_eq!(
        subscription_coverage_end("2023-03-01", SubscriptionBillingCycle::Annual)
            .expect("annual coverage ending in a leap year"),
        "2024-02-29"
    );
    assert_eq!(
        subscription_coverage_end("2024-02-29", SubscriptionBillingCycle::Annual)
            .expect("annual coverage beginning on leap day"),
        "2025-02-27"
    );
}

#[test]
fn subscription_coverage_rejects_invalid_start_dates() {
    let error = subscription_coverage_end("2026-02-30", SubscriptionBillingCycle::Monthly)
        .expect_err("invalid coverage start must fail");
    assert!(matches!(error, AppError::Validation(_)));
    assert!(error
        .to_string()
        .contains("Subscription coverage start must use YYYY-MM-DD"));
}

#[test]
fn billing_cycle_registration_derives_and_persists_exact_coverage_dates() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let monthly_source = directory.path().join("subscription-monthly.pdf");
    let annual_source = directory.path().join("subscription-annual.pdf");
    fs::write(&monthly_source, b"%PDF-1.7\nmonthly receipt\n%%EOF\n")
        .expect("monthly subscription fixture");
    fs::write(&annual_source, b"%PDF-1.7\nannual receipt\n%%EOF\n")
        .expect("annual subscription fixture");

    let monthly = app
        .register_global_evidence_for_billing_cycle(
            EvidenceRole::SubscriptionPayment,
            &monthly_source,
            "2026-08-01",
            SubscriptionBillingCycle::Monthly,
        )
        .expect("monthly billing-cycle registration");
    let annual = app
        .register_global_evidence_for_billing_cycle(
            EvidenceRole::SubscriptionPayment,
            &annual_source,
            "2026-08-01",
            SubscriptionBillingCycle::Annual,
        )
        .expect("annual billing-cycle registration");

    assert_eq!(
        monthly.evidence.coverage_start.as_deref(),
        Some("2026-08-01")
    );
    assert_eq!(monthly.evidence.coverage_end.as_deref(), Some("2026-08-31"));
    assert_eq!(
        annual.evidence.coverage_start.as_deref(),
        Some("2026-08-01")
    );
    assert_eq!(annual.evidence.coverage_end.as_deref(), Some("2027-07-31"));
    assert_eq!(
        fs::read(&monthly_source).expect("preserved monthly source"),
        b"%PDF-1.7\nmonthly receipt\n%%EOF\n"
    );
    assert_eq!(
        fs::read(&annual_source).expect("preserved annual source"),
        b"%PDF-1.7\nannual receipt\n%%EOF\n"
    );

    drop(app);
    let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
    let persisted = reopened
        .global_evidence()
        .expect("persisted global evidence");
    assert_eq!(persisted.len(), 2);
    for registered in [monthly, annual] {
        let stored = persisted
            .iter()
            .find(|item| item.evidence.id == registered.evidence.id)
            .expect("registered billing-cycle evidence persisted");
        assert_eq!(
            stored.evidence.coverage_start,
            registered.evidence.coverage_start
        );
        assert_eq!(
            stored.evidence.coverage_end,
            registered.evidence.coverage_end
        );
    }
}

#[test]
fn global_terms_pdf_import_requires_core_metadata_and_propagates_to_mutable_tracks() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Local Evidence Metadata".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: true,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    app.update_track(
        &track.id,
        TrackPatch {
            suno_terms_evidence_not_available: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("explicit unavailable status");
    let immutable_track = app
        .create_track(CreateTrackInput {
            title: "Immutable Before Global Terms".into(),
            production_start_date: "2026-07-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track that will represent a finalized snapshot");
    let mut immutable_record = app
        .persistence
        .track(&immutable_track.id)
        .expect("immutable track record");
    immutable_record.status = TrackStatus::Finalized;
    app.persistence
        .save_track(&immutable_record)
        .expect("mark immutable fixture finalized");
    let global_terms = register_and_assert_global_terms(&app, directory.path());
    assert_global_terms_propagation(&app, &track, &immutable_track, &global_terms);
    assert_global_terms_metadata_edit(&app, &track, &immutable_track, &global_terms);
    assert_later_terms_attachment_and_timestamp_rejection(
        &app,
        directory.path(),
        &track,
        &global_terms,
    );
    drop(app);
    assert_global_terms_survive_reopen(&workspace, &track, &global_terms);
}

fn register_and_assert_global_terms(app: &WorkspaceApp, directory: &Path) -> GlobalEvidenceItem {
    let invalid_terms_source = directory.join("suno-terms.txt");
    fs::write(&invalid_terms_source, b"not an accepted terms PDF\n")
        .expect("invalid terms fixture");
    let terms_metadata = EvidenceMetadata {
        document_title: "Suno Terms of Service".into(),
        provider: "Suno, Inc.".into(),
        source_url: "https://suno.com/terms".into(),
        retrieval_date: "2026-08-01".into(),
        effective_date: "2026-07-01".into(),
        applicable_production_period: "2026-07 through 2026-08".into(),
        factual_note: "Locally archived terms edition.".into(),
        ..EvidenceMetadata::default()
    };
    assert!(matches!(
        app.register_global_terms_evidence(&invalid_terms_source, terms_metadata.clone()),
        Err(AppError::FileType { .. })
    ));
    let disguised_pdf = directory.join("disguised-terms.pdf");
    fs::write(&disguised_pdf, b"plain text with a PDF extension\n")
        .expect("disguised terms fixture");
    assert!(matches!(
        app.register_global_terms_evidence(&disguised_pdf, terms_metadata.clone()),
        Err(AppError::Validation(_))
    ));

    let terms_source = directory.join("suno-terms-2026-08-01.pdf");
    fs::write(
        &terms_source,
        b"%PDF-1.7\n1 0 obj\n<</Type /Terms>>\nendobj\n%%EOF\n",
    )
    .expect("terms PDF fixture");
    assert!(app
        .register_global_terms_evidence(&terms_source, EvidenceMetadata::default())
        .expect_err("incomplete terms metadata")
        .to_string()
        .contains("descriptive metadata is incomplete"));
    let global_terms = app
        .register_global_terms_evidence(&terms_source, terms_metadata.clone())
        .expect("global terms import");
    assert_eq!(global_terms.evidence.role, EvidenceRole::SunoTermsRights);
    assert_eq!(
        global_terms.evidence.metadata.original_file_name,
        "suno-terms-2026-08-01.pdf"
    );
    assert_eq!(
        global_terms.evidence.metadata.document_title,
        "Suno Terms of Service"
    );
    assert_eq!(global_terms.evidence.metadata.provider, "Suno, Inc.");
    assert_eq!(global_terms.evidence.metadata.retrieval_date, "2026-08-01");
    global_terms
}

fn assert_global_terms_propagation(
    app: &WorkspaceApp,
    track: &TrackDetail,
    immutable_track: &TrackDetail,
    global_terms: &GlobalEvidenceItem,
) {
    let terms = app.load_track(&track.id).expect("track with global terms");
    assert_eq!(terms.fields.suno_terms_evidence_not_available, Some(false));
    let error = app
        .update_track(
            &track.id,
            TrackPatch {
                suno_terms_evidence_not_available: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect_err("verified Terms evidence must reject an unavailable claim");
    assert!(error.to_string().contains(
        "Terms evidence cannot be marked unavailable while a verified local Terms evidence file is attached."
    ));
    let terms = app.load_track(&track.id).expect("unchanged Terms status");
    assert_eq!(terms.fields.suno_terms_evidence_not_available, Some(false));
    let terms_item = terms
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoTermsRights)
        .expect("terms evidence");
    assert_eq!(
        terms_item.metadata.original_file_name,
        "suno-terms-2026-08-01.pdf"
    );
    assert_eq!(terms_item.metadata.provider, "Suno, Inc.");
    assert_eq!(
        terms_item.source_global_evidence_id.as_deref(),
        Some(global_terms.evidence.id.as_str())
    );
    assert_eq!(terms_item.provenance, EvidenceProvenance::GlobalCopy);
    assert!(!app
        .load_track(&immutable_track.id)
        .expect("unchanged finalized track")
        .evidence
        .iter()
        .any(|item| item.role == EvidenceRole::SunoTermsRights));
}

fn assert_global_terms_metadata_edit(
    app: &WorkspaceApp,
    track: &TrackDetail,
    immutable_track: &TrackDetail,
    global_terms: &GlobalEvidenceItem,
) {
    let edited_metadata = EvidenceMetadata {
        document_title: "Suno Terms of Service — archived edition".into(),
        provider: "Suno, Inc.".into(),
        source_url: "https://suno.com/terms".into(),
        retrieval_date: "2026-08-01".into(),
        effective_date: "2026-07-01".into(),
        applicable_production_period: "Production during August 2026".into(),
        factual_note: "Context corrected before track finalization.".into(),
        ..EvidenceMetadata::default()
    };
    let edited = app
        .update_global_terms_evidence_metadata(&global_terms.evidence.id, edited_metadata.clone())
        .expect("edit terms metadata");
    assert_eq!(
        edited.evidence.metadata.document_title,
        "Suno Terms of Service — archived edition"
    );
    let updated_copy = app
        .load_track(&track.id)
        .expect("track after terms metadata edit")
        .evidence
        .into_iter()
        .find(|item| item.role == EvidenceRole::SunoTermsRights)
        .expect("updated portable terms copy");
    assert_eq!(updated_copy.metadata, edited.evidence.metadata);
    assert!(!app
        .load_track(&immutable_track.id)
        .expect("finalized track remains unchanged after metadata edit")
        .evidence
        .iter()
        .any(|item| item.role == EvidenceRole::SunoTermsRights));
}

fn assert_later_terms_attachment_and_timestamp_rejection(
    app: &WorkspaceApp,
    directory: &Path,
    track: &TrackDetail,
    global_terms: &GlobalEvidenceItem,
) {
    let later_track = app
        .create_track(CreateTrackInput {
            title: "Created After Global Terms".into(),
            production_start_date: "2026-08-02".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("later track");
    assert!(later_track.evidence.iter().any(|item| {
        item.role == EvidenceRole::SunoTermsRights
            && item.source_global_evidence_id.as_deref() == Some(global_terms.evidence.id.as_str())
    }));

    let timestamp_source = directory.join("external-timestamp.json");
    fs::write(&timestamp_source, b"{\"timestamp\":\"fixture\"}\n").expect("timestamp fixture");
    let invalid = app.import_evidence_with_metadata_from(
        &track.id,
        EvidenceRole::ExternalTimestamp,
        &timestamp_source,
        EvidenceMetadata::default(),
    );
    assert!(invalid
        .expect_err("pre-finalization timestamp route is disabled")
        .to_string()
        .contains("after technical finalization"));
}

fn assert_global_terms_survive_reopen(
    workspace: &Path,
    track: &TrackDetail,
    global_terms: &GlobalEvidenceItem,
) {
    let reopened = WorkspaceApp::open(workspace, false).expect("reopened workspace");
    let stored_global = reopened.global_evidence().expect("global evidence");
    assert!(stored_global.iter().any(|item| {
        item.evidence.id == global_terms.evidence.id
            && item.evidence.metadata.original_file_name == "suno-terms-2026-08-01.pdf"
    }));
    let evidence = reopened
        .load_track(&track.id)
        .expect("reopened track")
        .evidence;
    assert!(evidence.iter().any(|item| {
        item.role == EvidenceRole::SunoTermsRights
            && item.metadata.original_file_name == "suno-terms-2026-08-01.pdf"
            && item.metadata.document_title == "Suno Terms of Service — archived edition"
    }));
}
