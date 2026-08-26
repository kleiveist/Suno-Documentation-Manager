use super::*;

#[test]
fn externally_deleted_evidence_remains_loadable_and_invalidates_finalized_state() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized =
        finalize_acceptance_track(&app, &directory.path().join("fixtures"), "Deleted Evidence");
    let deleted = finalized
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence")
        .clone();
    let track_root = app.root().join(&finalized.relative_path);
    fs::remove_file(track_root.join(&deleted.relative_path)).expect("external evidence deletion");

    let loaded = app
        .load_track(&finalized.id)
        .expect("track remains loadable after external deletion");
    let missing = loaded
        .evidence
        .iter()
        .find(|item| item.id == deleted.id)
        .expect("missing evidence remains indexed");
    assert!(!missing.verified);
    assert_eq!(missing.size_bytes, 0);
    assert_eq!(
        missing.verification_error.as_deref(),
        Some("Evidence file is missing.")
    );
    assert_eq!(loaded.status, TrackStatus::Finalized);
    assert!(!loaded.certificate.valid);
    assert!(loaded.certificate.invalidated_at.is_some());
    assert!(loaded
        .certificate
        .invalidation_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("changed after finalization")));
}

#[test]
fn legacy_artwork_role_detection_accepts_old_and_new_human_edited_names() {
    assert_eq!(
        infer_legacy_role("05_ARTWORK/Gravity-SUNO_ORIGINAL.jpeg"),
        EvidenceRole::ArtworkSunoOriginal
    );
    assert_eq!(
        infer_legacy_role("05_ARTWORK/Gravity_EDITED.jpeg"),
        EvidenceRole::HumanEditedArtwork
    );
    assert_eq!(
        infer_legacy_role("05_ARTWORK/Gravity_HUMAN_EDITED.jpeg"),
        EvidenceRole::HumanEditedArtwork
    );
    assert_eq!(
        infer_legacy_role("05_ARTWORK/Gravity_AI_EDITED.png"),
        EvidenceRole::AiArtworkEdited
    );
}

#[test]
fn legacy_scan_is_read_only_and_indexes_evidence_as_historically_unverified() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Legacy Session");
    for relative in ["01_RELEASE", "02_SUNO", "03_DOCUMENTATION"] {
        fs::create_dir_all(legacy_root.join(relative)).expect("legacy folder");
    }
    fs::write(
        legacy_root.join("01_RELEASE/legacy-master.wav"),
        b"RIFF\x08\0\0\0WAVElegacy master",
    )
    .expect("legacy release");
    fs::write(
        legacy_root.join("02_SUNO/legacy-export.wav"),
        b"RIFF\x08\0\0\0WAVElegacy Suno export",
    )
    .expect("legacy Suno evidence");
    fs::write(
        legacy_root.join("03_DOCUMENTATION/README.md"),
        b"# Existing legacy notes\n",
    )
    .expect("legacy documentation");
    let before = track_tree_snapshot(&legacy_root);

    let scan = app.scan_workspace().expect("legacy scan");
    assert_eq!(scan.discovered, 1);
    assert_eq!(scan.indexed, 1);
    assert_eq!(scan.unchanged, 0);
    let candidate = scan.candidates.first().expect("legacy candidate");
    assert_eq!(candidate.name, "Legacy Session");
    assert_eq!(candidate.relative_path, "Legacy Session");
    assert_eq!(candidate.status, "NOT_VERIFIED");
    assert!(candidate
        .recognized_folders
        .contains(&"01_RELEASE".to_owned()));
    assert!(candidate
        .documents
        .contains(&"03_DOCUMENTATION/README.md".to_owned()));
    assert!(candidate
        .evidence_files
        .contains(&"01_RELEASE/legacy-master.wav".to_owned()));
    assert!(candidate
        .evidence_files
        .contains(&"02_SUNO/legacy-export.wav".to_owned()));
    assert_eq!(track_tree_snapshot(&legacy_root), before);

    let record = app
        .persistence
        .track_by_relative_path("Legacy Session")
        .expect("legacy index lookup")
        .expect("indexed legacy record");
    assert!(record.legacy);
    assert_eq!(record.fields.title, "Legacy Session");
    let evidence = app
        .persistence
        .evidence(&record.id)
        .expect("indexed legacy evidence");
    assert_eq!(evidence.len(), 2);
    assert!(evidence.iter().all(|item| !item.verified));
    assert!(evidence.iter().all(|item| {
        item.verification_error
            .as_deref()
            .is_some_and(|error| error.contains("not been independently verified"))
    }));

    app.update_profile(complete_profile())
        .expect("complete current profile");
    let adopted = app
        .adopt_legacy_profile(&record.id)
        .expect("explicit legacy profile adoption");
    assert_eq!(adopted.profile_snapshot, complete_profile());

    fs::write(
        legacy_root.join("02_SUNO/later-export.wav"),
        b"RIFF\x08\0\0\0WAVElater legacy evidence",
    )
    .expect("later legacy evidence");
    let before_rescan = track_tree_snapshot(&legacy_root);
    let rescan = app.scan_workspace().expect("idempotent legacy rescan");
    assert_eq!(rescan.indexed, 0);
    assert_eq!(rescan.unchanged, 1);
    assert_eq!(track_tree_snapshot(&legacy_root), before_rescan);
    let reconciled = app
        .persistence
        .evidence(&record.id)
        .expect("reconciled legacy evidence");
    assert_eq!(reconciled.len(), 3);
    assert!(reconciled.iter().all(|item| !item.verified));
}

#[test]
fn reopening_scanned_legacy_track_preserves_existing_certificate_sentinel_in_place() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Legacy Certificate Sentinel");
    fs::create_dir_all(legacy_root.join("06_CERTIFICATE")).expect("legacy certificate directory");
    fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy release directory");
    let sentinel = legacy_root.join("06_CERTIFICATE/sentinel.bin");
    let sentinel_bytes = b"historical certificate bytes\0must stay in place";
    fs::write(&sentinel, sentinel_bytes).expect("legacy certificate sentinel");
    fs::write(
        legacy_root.join("01_RELEASE/legacy.wav"),
        b"RIFF\x08\0\0\0WAVElegacy release",
    )
    .expect("legacy release fixture");

    let scan = app.scan_workspace().expect("legacy scan");
    assert_eq!(scan.indexed, 1);
    let indexed = app
        .persistence
        .track_by_relative_path("Legacy Certificate Sentinel")
        .expect("legacy lookup")
        .expect("legacy track indexed");
    assert!(indexed.legacy);
    assert_eq!(
        fs::read(&sentinel).expect("sentinel after scan"),
        sentinel_bytes
    );
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("workspace reopen");
    assert_eq!(
        fs::read(&sentinel).expect("sentinel after reopen"),
        sentinel_bytes
    );
    assert!(!legacy_root.join(".archive/recovery").exists());
    let detail = reopened
        .load_track(&indexed.id)
        .expect("reopened legacy track");
    assert!(detail.legacy.unwrap_or(false));
}

#[test]
fn removing_indexed_legacy_evidence_archives_it_and_rescan_does_not_reindex_it() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Legacy Evidence Removal");
    fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy evidence directory");
    let original = legacy_root.join("02_SUNO/legacy-source.wav");
    let original_bytes = b"RIFF\x08\0\0\0WAVErecoverable legacy source";
    fs::write(&original, original_bytes).expect("legacy evidence fixture");
    app.scan_workspace().expect("initial legacy scan");
    let track = app
        .persistence
        .track_by_relative_path("Legacy Evidence Removal")
        .expect("legacy lookup")
        .expect("legacy track indexed");
    let item = app
        .persistence
        .evidence(&track.id)
        .expect("legacy evidence index")
        .into_iter()
        .find(|item| item.relative_path == "02_SUNO/legacy-source.wav")
        .expect("indexed legacy evidence");
    assert_eq!(item.provenance, EvidenceProvenance::IndexedLegacy);

    let removed = app
        .remove_evidence(&track.id, &item.id)
        .expect("archive indexed legacy evidence");
    assert!(!removed
        .evidence
        .iter()
        .any(|evidence| evidence.id == item.id));
    assert!(!original.exists());
    let removal_entries = fs::read_dir(legacy_root.join(".archive/removals"))
        .expect("removal archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("removal entries");
    assert_eq!(removal_entries.len(), 1);
    let removal = removal_entries[0].path();
    assert_eq!(
        fs::read(removal.join("legacy-source.wav")).expect("recoverable archived bytes"),
        original_bytes
    );
    let metadata = fs::read_to_string(removal.join("removal.json")).expect("removal metadata");
    assert!(metadata.contains("02_SUNO/legacy-source.wav"));
    assert!(metadata.contains("indexed_legacy"));

    let rescan = app.scan_workspace().expect("legacy rescan after removal");
    assert_eq!(rescan.indexed, 0);
    assert_eq!(rescan.unchanged, 1);
    assert!(app
        .persistence
        .evidence(&track.id)
        .expect("evidence after rescan")
        .is_empty());
    assert!(!rescan.candidates[0]
        .evidence_files
        .iter()
        .any(|relative| relative == "02_SUNO/legacy-source.wav"));
    assert_eq!(
        fs::read(removal.join("legacy-source.wav"))
            .expect("archive remains recoverable after rescan"),
        original_bytes
    );
}

#[test]
fn managed_track_scan_recovers_unindexed_file_then_archives_and_allows_regular_reimport() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Managed Crash Recovery".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("managed track");
    assert!(!track.legacy.unwrap_or(true));
    let track_root = workspace.join(&track.relative_path);
    let crash_file = track_root.join("02_SUNO/crash-copy.wav");
    let crash_bytes = b"RIFF\x08\0\0\0WAVEcopied before SQLite commit";
    fs::write(&crash_file, crash_bytes).expect("simulate copy-before-commit crash");
    assert!(app
        .persistence
        .evidence(&track.id)
        .expect("empty evidence index")
        .is_empty());

    let scan = app
        .scan_workspace()
        .expect("managed track source-of-truth scan");
    assert_eq!(scan.indexed, 0);
    assert_eq!(scan.unchanged, 1);
    let recovered = app
        .persistence
        .evidence(&track.id)
        .expect("recovered evidence index");
    assert_eq!(recovered.len(), 1);
    let recovered_item = &recovered[0];
    assert_eq!(recovered_item.relative_path, "02_SUNO/crash-copy.wav");
    assert_eq!(recovered_item.role, EvidenceRole::SunoFinalExport);
    assert_eq!(recovered_item.provenance, EvidenceProvenance::IndexedLegacy);
    assert!(!recovered_item.verified);
    assert!(recovered_item
        .verification_error
        .as_deref()
        .is_some_and(|message| message.contains("Recovered unindexed track evidence")));

    let removed = app
        .remove_evidence(&track.id, &recovered_item.id)
        .expect("archive recovered unindexed evidence");
    assert!(removed.evidence.is_empty());
    assert!(!crash_file.exists());
    let removal_entries = fs::read_dir(track_root.join(".archive/removals"))
        .expect("recovery removals")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("recovery removal entries");
    assert_eq!(removal_entries.len(), 1);
    let removal = removal_entries[0].path();
    assert_eq!(
        fs::read(removal.join("crash-copy.wav")).expect("recoverable crash copy"),
        crash_bytes
    );

    let fixtures = directory.path().join("fixtures");
    fs::create_dir(&fixtures).expect("fixture directory");
    let source = fixtures.join("crash-copy.wav");
    fs::write(&source, b"RIFF\x08\0\0\0WAVEregular reimport").expect("regular reimport fixture");
    let reimported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("regular evidence reimport");
    let managed = reimported
        .evidence
        .iter()
        .find(|item| item.relative_path == "02_SUNO/crash-copy.wav")
        .expect("managed reimport");
    assert_eq!(managed.provenance, EvidenceProvenance::ManagedCopy);
    assert!(managed.verified);
    assert_ne!(managed.id, recovered_item.id);
    assert_eq!(
        fs::read(&crash_file).expect("managed reimport bytes"),
        b"RIFF\x08\0\0\0WAVEregular reimport"
    );
}

#[test]
fn legacy_scan_never_arbitrarily_selects_duplicate_singular_candidates() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Legacy Artwork Candidates");
    fs::create_dir_all(legacy_root.join("05_ARTWORK")).expect("legacy artwork directory");
    fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy Suno directory");
    fs::write(
        legacy_root.join("05_ARTWORK/a_final.png"),
        b"\x89PNG\r\n\x1a\nfirst legacy final",
    )
    .expect("first final artwork candidate");
    fs::write(
        legacy_root.join("05_ARTWORK/b_final.png"),
        b"\x89PNG\r\n\x1a\nsecond legacy final",
    )
    .expect("second final artwork candidate");
    fs::write(
        legacy_root.join("02_SUNO/a-export.wav"),
        b"RIFF\x08\0\0\0WAVEfirst legacy Suno candidate",
    )
    .expect("first Suno candidate");
    fs::write(
        legacy_root.join("02_SUNO/b-export.wav"),
        b"RIFF\x08\0\0\0WAVEsecond legacy Suno candidate",
    )
    .expect("second Suno candidate");

    app.scan_workspace().expect("legacy artwork scan");
    let track = app
        .persistence
        .track_by_relative_path("Legacy Artwork Candidates")
        .expect("legacy lookup")
        .expect("legacy artwork track indexed");
    let evidence = app
        .persistence
        .evidence(&track.id)
        .expect("legacy artwork evidence");
    let ambiguous_items = evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::Other)
        .collect::<Vec<_>>();
    assert!(!evidence.iter().any(|item| matches!(
        item.role,
        EvidenceRole::FinalArtwork | EvidenceRole::SunoFinalExport
    )));
    assert_eq!(ambiguous_items.len(), 4);
    assert!(evidence
        .iter()
        .all(|item| { item.provenance == EvidenceProvenance::IndexedLegacy && !item.verified }));
    assert!(ambiguous_items.iter().all(|item| item
        .verification_error
        .as_deref()
        .is_some_and(|message| message.contains("ambiguous duplicate"))));
}

#[test]
fn legacy_verification_rejects_disguised_file_types_and_accepts_valid_magic_bytes() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Legacy Type Verification");
    fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy release directory");
    fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy Suno directory");
    fs::write(
        legacy_root.join("01_RELEASE/final.wav"),
        b"plain text disguised as wave audio",
    )
    .expect("disguised legacy WAV");
    fs::write(
        legacy_root.join("02_SUNO/valid-export.wav"),
        b"RIFF\x08\0\0\0WAVEvalid legacy export",
    )
    .expect("valid legacy WAV");
    app.scan_workspace().expect("legacy type scan");
    let track = app
        .persistence
        .track_by_relative_path("Legacy Type Verification")
        .expect("legacy lookup")
        .expect("legacy track indexed");
    app.update_profile(complete_profile())
        .expect("complete current profile");
    app.adopt_legacy_profile(&track.id)
        .expect("adopt profile for controlled native validation");
    app.update_track(
        &track.id,
        TrackPatch {
            production_start_date: Some("2026-08-01".into()),
            production_end_date: Some("2026-08-02".into()),
            ..TrackPatch::default()
        },
    )
    .expect("legacy production range");
    let indexed = app
        .persistence
        .evidence(&track.id)
        .expect("legacy evidence index");
    let disguised = indexed
        .iter()
        .find(|item| item.relative_path == "01_RELEASE/final.wav")
        .expect("disguised indexed evidence")
        .clone();
    let valid = indexed
        .iter()
        .find(|item| item.relative_path == "02_SUNO/valid-export.wav")
        .expect("valid indexed evidence")
        .clone();
    assert_eq!(disguised.role, EvidenceRole::ReleaseWav);
    assert_eq!(valid.role, EvidenceRole::SunoFinalExport);
    assert!(!disguised.verified);
    assert!(!valid.verified);

    let rejected = app
        .verify_evidence(&track.id, Some(&disguised.id))
        .expect("controlled disguised-type rejection");
    let rejected_item = rejected
        .evidence
        .iter()
        .find(|item| item.id == disguised.id)
        .expect("rejected disguised evidence remains indexed");
    assert!(!rejected_item.verified);
    assert!(rejected_item
        .verification_error
        .as_deref()
        .is_some_and(|message| message.contains("type verification failed")));
    assert!(rejected
        .missing_items
        .iter()
        .any(|item| item.contains("finale Release-Audiodatei")));
    let validation = app
        .validate_track(&track.id)
        .expect("legacy validation remains controlled");
    assert!(!validation.valid);
    assert!(validation
        .missing_items
        .iter()
        .any(|item| item.contains("finale Release-Audiodatei")));

    let accepted = app
        .verify_evidence(&track.id, Some(&valid.id))
        .expect("valid legacy magic-byte verification");
    let accepted_item = app
        .persistence
        .evidence_item(&track.id, &valid.id)
        .expect("stored verified valid legacy evidence");
    assert!(accepted_item.verified);
    assert!(accepted_item.verification_error.is_none());
    assert_eq!(accepted_item.provenance, EvidenceProvenance::IndexedLegacy);
    let disguised_after = app
        .persistence
        .evidence_item(&track.id, &disguised.id)
        .expect("disguised evidence still indexed");
    assert!(!disguised_after.verified);
    assert!(!accepted.missing_items.is_empty());
}
