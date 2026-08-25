use super::*;

struct FrozenLegacyMetadataContext {
    ready: TrackDetail,
    existing: EvidenceItem,
    alias_raw: String,
    track_root: PathBuf,
    certificate_before: BTreeMap<String, Vec<u8>>,
}

#[test]
fn p0_finalized_marker_alias_is_immutable_until_revision_reanalysis() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let fixture_root = directory.path().join("fixtures");
    let context = prepare_frozen_legacy_metadata(&app, &fixture_root);
    let raw_before = assert_finalized_legacy_metadata_is_immutable(&app, &context);
    assert_revision_reanalyzes_legacy_metadata(app, &directory, &context, raw_before);
}

fn prepare_frozen_legacy_metadata(
    app: &WorkspaceApp,
    fixture_root: &Path,
) -> FrozenLegacyMetadataContext {
    let ready = prepare_ready_track(app, fixture_root, "P0 Frozen Legacy Metadata");
    let existing = p0_evidence(&ready, EvidenceRole::SunoFinalExport).clone();
    let source = fixture_root.join("suno-export.wav");
    let alias_raw = p0_suno_marker_alias_comment("2026-08-18T07:17:52Z", P0_SUNO_MARKER_ALIAS_ID);
    fs::write(&source, p0_pcm_wav(Some(&alias_raw))).expect("real Suno WAV bytes");
    let replaced = app
        .replace_evidence_from(
            &ready.id,
            &existing.id,
            EvidenceRole::SunoFinalExport,
            &source,
        )
        .expect("replace fixture with real WAV");
    assert!(
        p0_evidence(&replaced, EvidenceRole::SunoFinalExport)
            .metadata
            .suno_studio_detected
    );
    app.update_track(
        &ready.id,
        TrackPatch {
            suno_export_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("reconfirm fixture filename");

    // Simulate a pre-feature evidence JSON row: the managed file already
    // contains Suno metadata, while the persisted additive fields do not.
    let mut legacy_evidence = app
        .persistence
        .evidence_item(&ready.id, &existing.id)
        .expect("stored Suno evidence");
    let original_file_name = legacy_evidence.metadata.original_file_name.clone();
    legacy_evidence.metadata = EvidenceMetadata {
        original_file_name,
        ..EvidenceMetadata::default()
    };
    app.persistence
        .save_evidence(&ready.id, &legacy_evidence)
        .expect("seed pre-metadata evidence row");
    let mut legacy_track = app.persistence.track(&ready.id).expect("stored track");
    // Mirror a finalized pre-detection snapshot whose manually recorded
    // dates differ from the subsequently recognized provider record.
    legacy_track.fields.production_end_date = "2026-08-17".into();
    legacy_track.fields.suno_final_generation_date = "2026-08-16".into();
    legacy_track.fields.suno_final_generation_id = P0_SUNO_MARKER_ALIAS_ID.into();
    legacy_track.fields.suno_download_export_date = "2026-08-16".into();
    legacy_track.fields.final_export_date = "2026-08-16".into();
    legacy_track.field_origins = Default::default();
    app.persistence
        .save_track(&legacy_track)
        .expect("seed pre-metadata track row");

    app.generate_documents(&ready.id, false)
        .expect("legacy-compatible documents");
    app.calculate_hashes(&ready.id)
        .expect("legacy-compatible hashes");
    let validation = app
        .validate_track(&ready.id)
        .expect("legacy-compatible gate");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
    let finalized = app
        .finalize_track(&ready.id)
        .expect("finalize pre-metadata record")
        .track
        .expect("finalized detail");
    let track_root = app.root().join(&finalized.relative_path);
    let certificate_before = certificate_file_snapshot(&track_root);
    FrozenLegacyMetadataContext {
        ready,
        existing,
        alias_raw,
        track_root,
        certificate_before,
    }
}

fn assert_finalized_legacy_metadata_is_immutable(
    app: &WorkspaceApp,
    context: &FrozenLegacyMetadataContext,
) -> P0RawFinalizedRows {
    let ready = &context.ready;
    let existing = &context.existing;
    let track_root = &context.track_root;
    let certificate_before = &context.certificate_before;
    // Simulate exact pre-feature JSON shapes. This catches a load path
    // that deserializes additive defaults and needlessly writes the
    // expanded current structures back.
    let connection = app.persistence.open().expect("raw workspace database");
    connection
        .execute(
            "UPDATE evidence SET metadata_json='{}' WHERE track_id=?1 AND id=?2",
            rusqlite::params![ready.id, existing.id],
        )
        .expect("seed exact legacy metadata JSON");
    let stored_track_json: String = connection
        .query_row(
            "SELECT data_json FROM tracks WHERE id=?1",
            [ready.id.as_str()],
            |row| row.get(0),
        )
        .expect("read current track JSON");
    let mut legacy_track_json: serde_json::Value =
        serde_json::from_str(&stored_track_json).expect("parse current track JSON");
    legacy_track_json
        .as_object_mut()
        .expect("track JSON object")
        .remove("fieldOrigins");
    connection
        .execute(
            "UPDATE tracks SET data_json=?1 WHERE id=?2",
            rusqlite::params![legacy_track_json.to_string(), ready.id],
        )
        .expect("seed exact legacy track JSON");
    drop(connection);
    let record_before = app.persistence.track(&ready.id).expect("frozen record");
    let metadata_before = app
        .persistence
        .evidence_item(&ready.id, &existing.id)
        .expect("frozen evidence")
        .metadata;
    assert!(!metadata_before.suno_studio_detected);
    assert!(metadata_before.suno_created_timestamp.is_empty());
    let raw_before = p0_raw_finalized_rows(app, &ready.id, &existing.id);
    assert_eq!(raw_before.evidence_metadata_json, "{}");
    assert!(!raw_before.track_data_json.contains("fieldOrigins"));

    let loaded = app.load_track(&ready.id).expect("load finalized record");
    let metadata_after = p0_evidence(&loaded, EvidenceRole::SunoFinalExport)
        .metadata
        .clone();
    let record_after = app.persistence.track(&ready.id).expect("record after load");
    let raw_after = p0_raw_finalized_rows(app, &ready.id, &existing.id);

    assert_eq!(loaded.status, TrackStatus::Finalized);
    assert!(loaded.certificate.valid);
    assert_eq!(metadata_after, metadata_before);
    assert_eq!(raw_after, raw_before);
    assert_eq!(record_after.updated_at, record_before.updated_at);
    assert_eq!(record_after.fields, record_before.fields);
    assert_eq!(record_after.field_origins, record_before.field_origins);
    assert_eq!(&certificate_file_snapshot(track_root), certificate_before);
    raw_before
}

fn assert_revision_reanalyzes_legacy_metadata(
    app: WorkspaceApp,
    directory: &tempfile::TempDir,
    context: &FrozenLegacyMetadataContext,
    raw_before: P0RawFinalizedRows,
) {
    let ready = &context.ready;
    let existing = &context.existing;
    let alias_raw = &context.alias_raw;
    let track_root = &context.track_root;
    let certificate_before = &context.certificate_before;
    app.list_tracks().expect("list finalized legacy track");
    assert_eq!(
        p0_raw_finalized_rows(&app, &ready.id, &existing.id),
        raw_before
    );
    drop(app);

    let reopened =
        WorkspaceApp::open(&directory.path().join("workspace"), false).expect("reopen workspace");
    let reopened_track = reopened
        .load_track(&ready.id)
        .expect("load finalized track after reopen");
    assert_eq!(reopened_track.status, TrackStatus::Finalized);
    assert_eq!(
        p0_raw_finalized_rows(&reopened, &ready.id, &existing.id),
        raw_before
    );
    assert_eq!(&certificate_file_snapshot(track_root), certificate_before);

    let revision = reopened
        .create_revision(&ready.id)
        .expect("explicit mutable revision")
        .track
        .expect("revision detail");
    let analyzed = p0_evidence(&revision, EvidenceRole::SunoFinalExport);
    assert_eq!(revision.status, TrackStatus::Active);
    assert!(analyzed.metadata.suno_studio_detected);
    assert_eq!(
        analyzed.metadata.suno_created_timestamp,
        "2026-08-18T07:17:52Z"
    );
    assert_eq!(analyzed.metadata.suno_created_date, "2026-08-18");
    assert_eq!(analyzed.metadata.suno_id, P0_SUNO_MARKER_ALIAS_ID);
    assert_eq!(analyzed.metadata.suno_raw_metadata, alias_raw.as_str());
    assert_eq!(revision.fields.production_end_date, "2026-08-18");
    assert_eq!(revision.fields.suno_final_generation_date, "2026-08-18");
    assert_eq!(
        revision.fields.suno_final_generation_id,
        P0_SUNO_MARKER_ALIAS_ID
    );
    assert_eq!(revision.fields.suno_download_export_date, "2026-08-18");
    assert_eq!(revision.fields.final_export_date, "2026-08-18");
    assert_eq!(
        revision.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        revision.automation.final_generation_id_origin,
        FactOrigin::UserConfirmedFact
    );
    assert_eq!(
        revision.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        revision.automation.download_export_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        revision.automation.final_export_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let archives = fs::read_dir(track_root.join(".archive/revisions"))
        .expect("revision archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("revision entries");
    assert_eq!(archives.len(), 1);
    for (relative, bytes) in certificate_before {
        let archived = if relative == certificate::PDF_FILE {
            archives[0].path().join(certificate::PDF_FILE)
        } else {
            archives[0]
                .path()
                .join("certificate")
                .join(Path::new(relative).file_name().expect("certificate name"))
        };
        assert_eq!(
            fs::read(archived).expect("archived finalized bytes"),
            *bytes
        );
    }
}
#[test]
fn evidence_import_rolls_back_file_and_database_when_track_commit_fails() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "Atomic Import", Some(false), false);
    let source = directory.path().join("atomic-import.wav");
    fs::write(
        &source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("atomic import WAV");
    let relative = evidence::managed_relative_path(
        &track.fields.title,
        &EvidenceRole::SunoFinalExport,
        &source,
    )
    .expect("planned managed path");
    let track_before = serde_json::to_value(
        app.persistence
            .track(&track.id)
            .expect("track before import"),
    )
    .expect("serialize track before import");
    app.persistence
        .open()
        .expect("database")
        .execute_batch(
            "CREATE TRIGGER injected_track_save_failure BEFORE UPDATE ON tracks
                 BEGIN SELECT RAISE(ABORT, 'injected track save failure'); END;",
        )
        .expect("failure trigger");

    assert!(matches!(
        app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source),
        Err(AppError::Database(_))
    ));
    assert!(app
        .persistence
        .evidence(&track.id)
        .expect("evidence after failed import")
        .is_empty());
    assert_eq!(
        serde_json::to_value(
            app.persistence
                .track(&track.id)
                .expect("track after import")
        )
        .expect("serialize track after import"),
        track_before
    );
    assert!(!app
        .root()
        .join(&track.relative_path)
        .join(relative)
        .exists());
}

#[test]
fn evidence_replace_rolls_back_bytes_evidence_and_track_together() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "Atomic Replace", Some(false), false);
    let first_source = directory.path().join("atomic-first.wav");
    fs::write(
        &first_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("first replacement WAV");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
        .expect("initial import");
    let previous = p0_evidence(&imported, EvidenceRole::SunoFinalExport).clone();
    let track_before = serde_json::to_value(
        app.persistence
            .track(&track.id)
            .expect("track before replace"),
    )
    .expect("serialize track before replace");
    let evidence_before = serde_json::to_value(
        app.persistence
            .evidence_item(&track.id, &previous.id)
            .expect("evidence before replace"),
    )
    .expect("serialize evidence before replace");
    let track_root = app.root().join(&track.relative_path);
    let previous_path = track_root.join(&previous.relative_path);
    let previous_bytes = fs::read(&previous_path).expect("previous managed bytes");
    let second_source = directory.path().join("atomic-second.wav");
    fs::write(
        &second_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-18T06:38:06Z"))),
    )
    .expect("second replacement WAV");
    let second_relative = evidence::managed_relative_path(
        &track.fields.title,
        &EvidenceRole::SunoFinalExport,
        &second_source,
    )
    .expect("second managed path");
    app.persistence
        .open()
        .expect("database")
        .execute_batch(
            "CREATE TRIGGER injected_track_save_failure BEFORE UPDATE ON tracks
                 BEGIN SELECT RAISE(ABORT, 'injected track save failure'); END;",
        )
        .expect("failure trigger");

    assert!(matches!(
        app.replace_evidence_from(
            &track.id,
            &previous.id,
            EvidenceRole::SunoFinalExport,
            &second_source,
        ),
        Err(AppError::Database(_))
    ));
    assert_eq!(
        serde_json::to_value(
            app.persistence
                .track(&track.id)
                .expect("track after replace")
        )
        .expect("serialize track after replace"),
        track_before
    );
    assert_eq!(
        serde_json::to_value(
            app.persistence
                .evidence_item(&track.id, &previous.id)
                .expect("evidence after replace")
        )
        .expect("serialize evidence after replace"),
        evidence_before
    );
    assert_eq!(
        fs::read(&previous_path).expect("restored managed bytes"),
        previous_bytes
    );
    assert!(!track_root.join(second_relative).exists());
}

#[test]
fn active_v15_reevaluation_adopts_authoritative_metadata_dates() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "Active 1.5 Metadata Upgrade", None, false);
    let source = directory.path().join("active-v15.wav");
    fs::write(
        &source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("Suno WAV");
    app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("initial metadata import");

    let mut stale = app.persistence.track(&track.id).expect("stored track");
    stale.workflow_version = "1.5".into();
    stale.fields.suno_final_generation_date = "2026-08-16".into();
    stale.fields.production_end_date = "2026-08-18".into();
    stale.field_origins = Default::default();
    app.persistence
        .save_track(&stale)
        .expect("seed 1.5 fallback dates");

    let upgraded = app
        .re_evaluate_track(&track.id)
        .expect("explicit 1.9 reevaluation")
        .track
        .expect("reevaluated track");

    assert_eq!(upgraded.workflow_version, "1.9");
    assert_eq!(upgraded.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(upgraded.fields.production_end_date, "2026-08-17");
    assert_eq!(
        upgraded.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        upgraded.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
}

#[test]
fn outdated_workflow_blocks_new_outputs_until_explicit_reevaluation() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let created = p0_track(&app, "Outdated Workflow", None, false);
    let mut stale = app.persistence.track(&created.id).expect("stored track");
    stale.workflow_version = "1.3".into();
    stale.fields.production_end_date = "2026-08-01".into();
    app.persistence
        .save_track(&stale)
        .expect("seed old workflow");

    let validation = app.validate_track(&created.id).expect("validation result");
    assert!(!validation.valid);
    assert!(validation
        .blocking_items
        .iter()
        .any(|item| item.contains("Re-evaluate the track explicitly")));
    for result in [
        app.generate_documents(&created.id, false),
        app.calculate_hashes(&created.id),
        app.finalize_track(&created.id),
    ] {
        assert!(matches!(
            result,
            Err(AppError::Validation(message))
                if message.contains("Re-evaluate the track explicitly")
        ));
    }

    let upgraded = app
        .re_evaluate_track(&created.id)
        .expect("explicit reevaluation")
        .track
        .expect("reevaluated track");
    assert_eq!(upgraded.workflow_version, "1.9");
}

#[test]
fn superseded_tracks_reject_content_and_workflow_mutations() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let created = p0_track(&app, "Superseded Snapshot", None, false);
    let mut superseded = app.persistence.track(&created.id).expect("stored track");
    superseded.status = TrackStatus::Superseded;
    superseded.workflow_version = "1.3".into();
    app.persistence
        .save_track(&superseded)
        .expect("seed superseded snapshot");

    assert!(matches!(
        app.update_track(
            &created.id,
            TrackPatch {
                title: Some("Changed title".into()),
                ..TrackPatch::default()
            },
        ),
        Err(AppError::Finalized)
    ));
    assert!(matches!(
        app.update_track_library(&created.id, TrackLibraryPlacement::default()),
        Err(AppError::Finalized)
    ));
    assert!(matches!(
        app.import_evidence_from(
            &created.id,
            EvidenceRole::SunoFinalExport,
            directory.path().join("unused.wav").as_path(),
        ),
        Err(AppError::Finalized)
    ));
    assert!(matches!(
        app.re_evaluate_track(&created.id),
        Err(AppError::Finalized)
    ));
}

#[test]
fn revision_with_missing_suno_bytes_is_created_without_partial_failure() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Missing Suno Revision",
    );
    let suno = p0_evidence(&finalized, EvidenceRole::SunoFinalExport).clone();
    let track_root = app.root().join(&finalized.relative_path);
    fs::remove_file(track_root.join(&suno.relative_path)).expect("remove Suno bytes");

    let revision = app
        .create_revision(&finalized.id)
        .expect("revision despite missing Suno bytes")
        .track
        .expect("active revision");
    assert_eq!(revision.status, TrackStatus::Active);
    assert!(revision
        .evidence
        .iter()
        .any(|item| item.id == suno.id && !item.verified));
    assert_eq!(
        fs::read_dir(track_root.join(".archive/revisions"))
            .expect("revision archive")
            .count(),
        1
    );
}
