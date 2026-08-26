use super::*;

#[test]
fn blocking_deviation_prevents_validation_and_finalization_until_resolved() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Blocking Deviation",
    );
    let with_deviation = app
        .add_deviation(
            &ready.id,
            DeviationInput {
                description: "Rights review is still open".into(),
                blocking: true,
            },
        )
        .expect("blocking deviation");
    let deviation = with_deviation
        .blocking_deviations
        .iter()
        .find(|item| item.blocking && !item.resolved)
        .expect("unresolved blocking deviation")
        .clone();

    let blocked = app.validate_track(&ready.id).expect("blocked validation");
    assert!(!blocked.valid);
    assert!(blocked
        .blocking_items
        .iter()
        .any(|item| item.contains("Rights review is still open")));
    assert!(matches!(
        app.finalize_track(&ready.id),
        Err(AppError::Validation(_))
    ));
    assert!(!app
        .root()
        .join(&ready.relative_path)
        .join(certificate::CERTIFICATE_FILE)
        .exists());

    let resolved = app
        .resolve_deviation(&ready.id, &deviation.id)
        .expect("resolve deviation");
    assert!(resolved
        .blocking_deviations
        .iter()
        .any(|item| item.id == deviation.id && item.resolved && item.resolved_at.is_some()));
    let allowed = app
        .validate_track(&ready.id)
        .expect("validation after resolution");
    assert!(
        allowed.valid,
        "missing={:?}; blocking={:?}",
        allowed.missing_items, allowed.blocking_items
    );
    let finalized = app
        .finalize_track(&ready.id)
        .expect("finalization after resolution")
        .track
        .expect("finalized detail");
    assert_eq!(finalized.status, TrackStatus::Finalized);
    assert!(finalized.certificate.valid);
}

#[test]
fn external_change_invalidates_certificate_state_without_rewriting_certificate_files() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "External Mutation",
    );
    let track_root = app.root().join(&finalized.relative_path);
    let certificate_before = certificate_file_snapshot(&track_root);
    let release = finalized
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");
    fs::write(
        track_root.join(&release.relative_path),
        b"RIFF\x08\0\0\0WAVEexternally changed release",
    )
    .expect("external track mutation");

    let loaded = app
        .load_track(&finalized.id)
        .expect("load externally changed finalized track");
    assert_eq!(loaded.status, TrackStatus::Finalized);
    assert!(!loaded.integrity.verified);
    assert!(!loaded.certificate.valid);
    assert!(loaded.certificate.invalidated_at.is_some());
    assert_eq!(certificate_file_snapshot(&track_root), certificate_before);
    certificate::verify(&track_root).expect("unchanged certificate file set remains intact");
}

#[test]
fn technical_pdf_mutation_fails_certificate_verification_and_invalidates_state() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized =
        finalize_acceptance_track(&app, &directory.path().join("fixtures"), "PDF Mutation");
    let track_root = app.root().join(&finalized.relative_path);
    let pdf_path = track_root.join(certificate::PDF_FILE);
    let mut changed = fs::read(&pdf_path).expect("technical PDF");
    changed.push(b'X');
    fs::write(&pdf_path, changed).expect("mutate technical PDF");

    let error = certificate::verify(&track_root).expect_err("modified PDF must fail");
    assert!(error.to_string().contains(certificate::PDF_FILE));
    let loaded = app
        .load_track(&finalized.id)
        .expect("load track with modified PDF");
    assert_eq!(loaded.status, TrackStatus::Finalized);
    assert!(!loaded.certificate.valid);
    assert!(loaded.certificate.invalidated_at.is_some());
}

#[test]
fn workspace_scan_does_not_index_the_root_pdf_as_evidence() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "PDF Scan Exclusion",
    );
    let evidence_count = finalized.evidence.len();

    app.scan_workspace().expect("workspace scan");
    let rescanned = app.load_track(&finalized.id).expect("rescanned track");
    assert_eq!(
        rescanned.evidence.len(),
        evidence_count,
        "rescanned evidence paths: {:?}",
        rescanned
            .evidence
            .iter()
            .map(|item| item.relative_path.as_str())
            .collect::<Vec<_>>()
    );
    assert!(!rescanned
        .evidence
        .iter()
        .any(|item| item.relative_path == certificate::PDF_FILE));
    assert!(rescanned.certificate.valid);
}

#[test]
fn corrupted_certificate_can_be_preserved_in_a_recovery_revision() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Certificate Recovery",
    );
    let track_root = app.root().join(&finalized.relative_path);
    fs::write(
        track_root.join(certificate::CERTIFICATE_HASH_FILE),
        b"damaged certificate hash list\n",
    )
    .expect("damage certificate hashes");

    let invalid = app
        .load_track(&finalized.id)
        .expect("invalid finalized snapshot remains loadable");
    assert!(!invalid.certificate.valid);
    let revision = app
        .create_revision(&finalized.id)
        .expect("recovery revision archives even an invalid set")
        .track
        .expect("active recovery detail");
    assert_ne!(revision.status, TrackStatus::Finalized);
    let archives = fs::read_dir(track_root.join(".archive/revisions"))
        .expect("revision archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("archive entries");
    assert_eq!(archives.len(), 1);
    let archive = archives[0].path();
    assert!(archive.join("certificate/CERTIFICATE_SHA256.txt").is_file());
    assert!(archive.join(certificate::PDF_FILE).is_file());
    assert!(archive.join(integrity::HASH_FILE).is_file());
    let metadata = fs::read_to_string(archive.join("revision.json")).expect("revision metadata");
    assert!(metadata.contains("invalid_or_incomplete"));
}

#[test]
fn finalization_with_historical_certificate_sentinel_collides_before_marker_creation() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Historical Certificate Collision",
    );
    let track_root = app.root().join(&ready.relative_path);
    let sentinel = track_root.join("06_CERTIFICATE/historical-sentinel.bin");
    let sentinel_bytes = b"historical certificate bytes must never be overwritten";
    fs::write(&sentinel, sentinel_bytes).expect("historical certificate sentinel");
    let marker = track_root.join(".archive/finalization-in-progress.json");

    let error = app
        .finalize_track(&ready.id)
        .expect_err("historical certificate content must block finalization");
    assert!(matches!(error, AppError::Collision(_)));
    assert_eq!(
        fs::read(&sentinel).expect("unchanged historical sentinel"),
        sentinel_bytes
    );
    assert!(
        !marker.exists(),
        "collision must precede marker publication"
    );
    assert_eq!(
        fs::read_dir(track_root.join("06_CERTIFICATE"))
            .expect("certificate directory")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("certificate entries")
            .len(),
        1
    );
    let unchanged = app.load_track(&ready.id).expect("track remains loadable");
    assert_ne!(unchanged.status, TrackStatus::Finalized);
    assert!(!unchanged.certificate.valid);
}

#[test]
fn finalization_never_overwrites_an_existing_root_pdf() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Historical PDF Collision",
    );
    let track_root = app.root().join(&ready.relative_path);
    let pdf = track_root.join(certificate::PDF_FILE);
    let sentinel = b"existing technical PDF must not be overwritten";
    fs::write(&pdf, sentinel).expect("historical PDF sentinel");

    let error = app
        .finalize_track(&ready.id)
        .expect_err("existing PDF must block finalization");
    assert!(matches!(error, AppError::Collision(_)));
    assert_eq!(fs::read(&pdf).expect("unchanged PDF sentinel"), sentinel);
    assert!(!track_root
        .join(".archive/finalization-in-progress.json")
        .exists());
    assert!(
        directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
            .expect("empty certificate directory")
    );
}

#[test]
fn finalization_database_commit_failure_rolls_back_publication_and_reopens_cleanly() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Database Commit Failure",
    );
    let track_root = workspace.join(&ready.relative_path);

    let error = app
        .finalize_track_impl(
            &ready.id,
            Some(FinalizationFailure::DatabaseCommit),
            &mut |_| {},
        )
        .expect_err("injected database commit failure");
    assert!(error
        .to_string()
        .contains("Injected finalization database commit failure"));
    let stored = app
        .persistence
        .track(&ready.id)
        .expect("stored active track");
    assert_ne!(stored.status, TrackStatus::Finalized);
    assert!(!stored.certificate.valid);
    assert!(
        directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
            .expect("empty certificate directory")
    );
    assert!(!track_root.join(certificate::PDF_FILE).exists());
    assert!(!track_root
        .join(".archive/finalization-in-progress.json")
        .exists());
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("clean workspace reopen");
    let detail = reopened.load_track(&ready.id).expect("load active track");
    assert_ne!(detail.status, TrackStatus::Finalized);
    assert!(!detail.certificate.valid);
    assert!(
        directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
            .expect("empty certificate after reopen")
    );
    assert!(!track_root.join(certificate::PDF_FILE).exists());
}

#[test]
fn technical_pdf_failures_leave_no_partial_finalization() {
    for (failure, label) in [
        (FinalizationFailure::PdfGeneration, "generation"),
        (FinalizationFailure::PdfStaging, "staging"),
        (FinalizationFailure::PdfPublication, "publication"),
        (
            FinalizationFailure::PostPublishVerification,
            "post-publish-verification",
        ),
    ] {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            &format!("PDF Failure {label}"),
        );
        let track_root = workspace.join(&ready.relative_path);

        app.finalize_track_impl(&ready.id, Some(failure), &mut |_| {})
            .expect_err("injected PDF finalization failure");
        let stored = app
            .persistence
            .track(&ready.id)
            .expect("stored active track");
        assert_ne!(stored.status, TrackStatus::Finalized, "{label}");
        assert!(!stored.certificate.valid, "{label}");
        assert!(!track_root.join(certificate::PDF_FILE).exists(), "{label}");
        assert!(
            directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
                .expect("empty live certificate"),
            "{label}"
        );
        assert!(
            !track_root
                .join(".archive/finalization-in-progress.json")
                .exists(),
            "{label}"
        );
        let staging = track_root.join(".archive/certificate-staging");
        assert!(
            !staging.exists()
                || fs::read_dir(&staging)
                    .expect("certificate staging")
                    .next()
                    .is_none(),
            "{label}"
        );
    }
}

#[test]
fn workspace_reopen_recovers_filesystem_database_commit_windows() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let reopened = assert_interrupted_finalization_recovery(&workspace);
    assert_interrupted_revision_recovery(reopened, &workspace, &directory.path().join("fixtures"));
}

fn assert_interrupted_finalization_recovery(workspace: &Path) -> WorkspaceApp {
    let app = WorkspaceApp::open(workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let active = app
        .create_track(CreateTrackInput {
            title: "Interrupted Finalization".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("active track");
    let active_root = workspace.join(&active.relative_path);
    fs::write(
        active_root.join("06_CERTIFICATE/orphan-certificate.md"),
        b"published before database commit",
    )
    .expect("orphan certificate fixture");
    fs::write(
        active_root.join(certificate::PDF_FILE),
        b"root PDF published before database commit",
    )
    .expect("orphan root PDF fixture");
    let interrupted_transaction = "interrupted-finalization-fixture";
    fs::write(
        active_root.join(".archive/finalization-in-progress.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "transaction_id": interrupted_transaction,
            "track_id": active.id,
            "certificate_id": "SDM-interrupted-fixture",
            "started_at": "2026-08-13T12:00:00Z"
        }))
        .expect("finalization recovery marker"),
    )
    .expect("write finalization recovery marker");
    let interrupted_stage = active_root
        .join(".archive/certificate-staging")
        .join(interrupted_transaction);
    fs::create_dir_all(&interrupted_stage).expect("interrupted staging directory");
    fs::write(
        interrupted_stage.join("EVIDENCE_MANIFEST.json"),
        b"staged before process exit",
    )
    .expect("interrupted staged artifact");
    drop(app);

    let reopened = WorkspaceApp::open(workspace, false).expect("recovered workspace");
    assert!(
        directory_is_empty_or_missing(&active_root.join("06_CERTIFICATE"))
            .expect("empty live certificate")
    );
    let recovered_files = WalkDir::new(active_root.join(".archive/recovery"))
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("recovery archive");
    assert!(recovered_files
        .iter()
        .any(|entry| entry.file_name() == "orphan-certificate.md"));
    assert_eq!(
        fs::read(
            active_root
                .join(".archive/recovery")
                .join(interrupted_transaction)
                .join("certificate/orphan-certificate.md")
        )
        .expect("marker-selected recovery snapshot"),
        b"published before database commit"
    );
    assert_eq!(
        fs::read(
            active_root
                .join(".archive/recovery")
                .join(interrupted_transaction)
                .join(certificate::PDF_FILE)
        )
        .expect("recovered root PDF"),
        b"root PDF published before database commit"
    );
    assert_eq!(
        fs::read(
            active_root
                .join(".archive/recovery")
                .join(interrupted_transaction)
                .join("certificate-staging/EVIDENCE_MANIFEST.json")
        )
        .expect("correlated staging recovery"),
        b"staged before process exit"
    );
    assert!(!interrupted_stage.exists());
    assert!(!active_root
        .join(".archive/finalization-in-progress.json")
        .exists());
    reopened
}

fn assert_interrupted_revision_recovery(reopened: WorkspaceApp, workspace: &Path, fixtures: &Path) {
    let finalized = finalize_acceptance_track(&reopened, fixtures, "Interrupted Revision");
    let finalized_root = workspace.join(&finalized.relative_path);
    let wrong_archive = finalized_root.join(".archive/revisions/zzzz-wrong-certificate");
    fs::create_dir(&wrong_archive).expect("wrong revision archive");
    fs::create_dir(wrong_archive.join("certificate")).expect("wrong certificate directory");
    fs::write(
        wrong_archive.join("certificate/DOCUMENTATION_CERTIFICATE.md"),
        b"wrong certificate snapshot",
    )
    .expect("wrong archived certificate");
    let mut wrong_certificate = finalized.certificate.clone();
    wrong_certificate.certificate_id = Some("SDM-different-revision".into());
    fs::write(
        wrong_archive.join("revision.json"),
        serde_json::to_vec(&serde_json::json!({
            "track_id": finalized.id,
            "previous_certificate": wrong_certificate,
            "reason": "older unrelated revision"
        }))
        .expect("wrong revision metadata"),
    )
    .expect("wrong revision metadata file");
    let crash_archive = finalized_root.join(".archive/revisions/crash-fixture");
    fs::create_dir(&crash_archive).expect("crash archive");
    fs::write(
        crash_archive.join("revision.json"),
        serde_json::to_vec(&serde_json::json!({
            "track_id": finalized.id,
            "previous_certificate": finalized.certificate,
            "reason": "simulated process exit before database commit"
        }))
        .expect("crash metadata"),
    )
    .expect("crash metadata file");
    fs::rename(
        finalized_root.join(certificate::CERTIFICATE_DIR),
        crash_archive.join("certificate"),
    )
    .expect("simulate revision publish before DB commit");
    fs::rename(
        finalized_root.join(certificate::PDF_FILE),
        crash_archive.join(certificate::PDF_FILE),
    )
    .expect("simulate revision PDF archive before DB commit");
    fs::create_dir(finalized_root.join(certificate::CERTIFICATE_DIR))
        .expect("empty live certificate directory");
    drop(reopened);

    let recovered = WorkspaceApp::open(workspace, false).expect("revision recovery");
    certificate::verify(&finalized_root).expect("certificate restored to live snapshot");
    assert!(!crash_archive.join("certificate").exists());
    assert!(!crash_archive.join(certificate::PDF_FILE).exists());
    assert!(finalized_root.join(certificate::PDF_FILE).is_file());
    assert!(wrong_archive.join("certificate").is_dir());
    assert_eq!(
        fs::read(wrong_archive.join("certificate/DOCUMENTATION_CERTIFICATE.md"))
            .expect("unrelated archived certificate remains untouched"),
        b"wrong certificate snapshot"
    );
    let detail = recovered
        .load_track(&finalized.id)
        .expect("recovered finalized track");
    assert_eq!(detail.status, TrackStatus::Finalized);
    assert!(detail.certificate.valid);
}
