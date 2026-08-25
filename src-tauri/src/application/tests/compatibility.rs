use super::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

mod sqlite_snapshot;

use sqlite_snapshot::{
    assert_expected_database_changes, logical_database_digest, logical_database_snapshot,
    LogicalDatabaseSnapshot,
};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
struct Phase24DiffClasses {
    expected_test_output: BTreeMap<String, String>,
    permitted_fixture_metadata_change: BTreeMap<String, String>,
    unexpected_product_data_change: BTreeMap<String, String>,
}

struct Phase24Fixture {
    _directory: tempfile::TempDir,
    workspace: PathBuf,
    fixture_root: PathBuf,
    track_root: PathBuf,
    ready: TrackDetail,
    track_before: TrackRecord,
    evidence_before: BTreeMap<String, serde_json::Value>,
    profile_before: Profile,
    fixture_hashes_before: BTreeMap<String, String>,
    track_hashes_before: BTreeMap<String, String>,
    workspace_hashes_before: BTreeMap<String, String>,
    schema_before: i64,
    database_before: LogicalDatabaseSnapshot,
}

struct Phase24FinalizedPass {
    controlled_profile: Profile,
    finalized_record: TrackRecord,
    finalized_evidence: BTreeMap<String, serde_json::Value>,
    finalized_database: LogicalDatabaseSnapshot,
}

struct Phase24AfterPass {
    fixture_hashes: BTreeMap<String, String>,
    track_hashes: BTreeMap<String, String>,
    workspace_hashes: BTreeMap<String, String>,
    schema_version: i64,
    database: LogicalDatabaseSnapshot,
}

#[test]
fn phase_24_workspace_round_trip_classifies_tree_hash_changes() {
    let fixture = prepare_phase_24_fixture();
    let finalized = run_phase_24_open_and_write_pass(&fixture);
    let after = run_phase_24_restart_pass(&fixture, &finalized);
    let classes = assert_phase_24_hash_classification(&fixture, &after);
    emit_phase_24_evidence(&fixture, &after, &classes);
}

fn prepare_phase_24_fixture() -> Phase24Fixture {
    let directory = tempdir().expect("temporary Phase 24 directory");
    let workspace = directory.path().join("workspace");
    let fixture_root = directory.path().join("fixtures");

    // Fixture setup is outside the numbered compatibility pass. It creates
    // only synthetic data below `tempdir` and leaves an active track with
    // generated documents and a verified SHA256SUMS file.
    let setup = WorkspaceApp::open(&workspace, true).expect("temporary workspace setup");
    setup
        .update_profile(complete_profile())
        .expect("synthetic fixture profile");
    let ready = prepare_ready_track(&setup, &fixture_root, "Phase 24 Compatibility");
    let track_root = workspace.join(&ready.relative_path);
    let track_before = setup
        .persistence
        .track(&ready.id)
        .expect("fixture track before compatibility pass");
    let evidence_before = evidence_snapshot(
        &setup
            .persistence
            .evidence(&ready.id)
            .expect("fixture evidence before compatibility pass"),
    );
    let profile_before = setup
        .profile()
        .expect("fixture profile before controlled write");

    let schema_before = schema_version(&setup);
    let database_before = logical_database_snapshot(&setup);

    // 1. Capture before hashes for every regular file in the source fixture,
    // portable track, and isolated workspace before the compatibility pass.
    let fixture_hashes_before = file_hashes(&fixture_root);
    let track_hashes_before = file_hashes(&track_root);
    let workspace_hashes_before = file_hashes(&workspace);
    drop(setup);

    Phase24Fixture {
        _directory: directory,
        workspace,
        fixture_root,
        track_root,
        ready,
        track_before,
        evidence_before,
        profile_before,
        fixture_hashes_before,
        track_hashes_before,
        workspace_hashes_before,
        schema_before,
        database_before,
    }
}

fn run_phase_24_open_and_write_pass(fixture: &Phase24Fixture) -> Phase24FinalizedPass {
    // 2. Open the isolated test workspace.
    let app = WorkspaceApp::open(&fixture.workspace, false).expect("open Phase 24 workspace");

    assert_phase_24_opened_state(&app, fixture);
    regenerate_verify_and_finalize(&app, fixture);

    // 9. Perform one controlled write in the fixture. A workspace profile
    // update is intentionally outside the immutable finalized track snapshot.
    let mut controlled_profile = app.profile().expect("profile before controlled write");
    controlled_profile.artist_name = "Phase 24 Controlled Fixture Update".into();
    assert_eq!(
        app.update_profile(controlled_profile.clone())
            .expect("controlled fixture metadata write"),
        controlled_profile
    );
    let finalized_record = app
        .persistence
        .track(&fixture.ready.id)
        .expect("finalized record before restart");
    let finalized_evidence = evidence_snapshot(
        &app.persistence
            .evidence(&fixture.ready.id)
            .expect("finalized evidence before restart"),
    );
    assert_eq!(finalized_evidence, fixture.evidence_before);
    assert_eq!(finalized_record.fields, fixture.track_before.fields);
    assert_eq!(finalized_record.profile_snapshot, fixture.profile_before);
    assert_eq!(
        finalized_record.workflow_id,
        fixture.track_before.workflow_id
    );
    assert_eq!(
        finalized_record.workflow_version,
        fixture.track_before.workflow_version
    );
    assert_eq!(finalized_record.library, fixture.track_before.library);
    let finalized_database = logical_database_snapshot(&app);
    assert_expected_database_changes(
        &fixture.database_before,
        &finalized_database,
        &fixture.track_before,
        &finalized_record,
        &controlled_profile,
    );

    // 10. Close the application object cleanly.
    drop(app);

    Phase24FinalizedPass {
        controlled_profile,
        finalized_record,
        finalized_evidence,
        finalized_database,
    }
}

fn assert_phase_24_opened_state(app: &WorkspaceApp, fixture: &Phase24Fixture) {
    // 3. Detect the existing track through the typed SQLite repository. The
    // public list view refreshes observation timestamps, while this compatibility
    // pass must keep the pre-finalization logical database byte-for-byte stable.
    let detected = app
        .persistence
        .tracks()
        .expect("detect existing fixture track");
    assert!(
        detected.iter().any(|track| track.id == fixture.ready.id),
        "prepared track was not detected after opening the workspace"
    );

    // 4. Load SQLite data and confirm that the schema did not change merely
    // because the migrated application opened it.
    let opened_track = app
        .persistence
        .track(&fixture.ready.id)
        .expect("load fixture track from SQLite");
    assert_eq!(schema_version(app), fixture.schema_before);
    assert_eq!(fixture.schema_before, crate::persistence::SCHEMA_VERSION);
    assert_eq!(opened_track.fields, fixture.track_before.fields);
    assert_eq!(
        opened_track.profile_snapshot,
        fixture.track_before.profile_snapshot
    );

    // 5. Detect every evidence record and verify its current managed bytes.
    let opened_evidence = app
        .persistence
        .evidence(&fixture.ready.id)
        .expect("load fixture evidence from SQLite");
    assert_eq!(evidence_snapshot(&opened_evidence), fixture.evidence_before);
    for item in &opened_evidence {
        let expected = item
            .sha256
            .as_deref()
            .expect("prepared evidence has a SHA-256 digest");
        assert_eq!(
            sha256_file(&fixture.track_root.join(&item.relative_path))
                .expect("hash managed evidence"),
            expected,
            "managed evidence bytes changed for {}",
            item.relative_path
        );
    }
    assert_eq!(
        logical_database_snapshot(app),
        fixture.database_before,
        "opening and reading the workspace changed its logical SQLite contents"
    );
}

fn regenerate_verify_and_finalize(app: &WorkspaceApp, fixture: &Phase24Fixture) {
    // 6. Regenerate documents directly from the loaded persisted state. The
    // compatibility pass verifies output bytes without adding a second, hidden
    // database write before the explicit finalization/profile operations.
    let track = app
        .persistence
        .track(&fixture.ready.id)
        .expect("load track for deterministic document regeneration");
    let evidence = app
        .persistence
        .evidence(&fixture.ready.id)
        .expect("load evidence for deterministic document regeneration");
    let deviations = app
        .persistence
        .deviations(&fixture.ready.id)
        .expect("load deviations for deterministic document regeneration");
    let stored_steps = app
        .persistence
        .stored_steps(&fixture.ready.id)
        .expect("load stored steps for deterministic document regeneration");
    let evaluation = workflow::evaluate(
        &track,
        &track.profile_snapshot,
        &evidence,
        &deviations,
        &stored_steps,
    )
    .expect("evaluate deterministic document fixture");
    documents::generate(
        &fixture.track_root,
        &track,
        &track.profile_snapshot,
        &evidence,
        &evaluation.steps,
        false,
    )
    .expect("regenerate fixture documents");
    let hashes_after_document_generation = file_hashes(&fixture.track_root);
    for relative in documents::DOCUMENT_PATHS {
        assert_eq!(
            hashes_after_document_generation.get(relative),
            fixture.track_hashes_before.get(relative),
            "document regeneration changed {relative}"
        );
    }

    // 7. Verify the existing main hash list after deterministic regeneration.
    assert!(
        integrity::verify(&fixture.track_root)
            .expect("verify existing fixture hashes")
            .verified,
        "the existing fixture SHA256SUMS did not verify"
    );
    assert_eq!(
        logical_database_snapshot(app),
        fixture.database_before,
        "document/hash verification changed logical SQLite contents"
    );

    // 8. Generate and verify a fresh certificate from the isolated fixture.
    let finalized = app
        .finalize_track(&fixture.ready.id)
        .expect("finalize fixture track")
        .track
        .expect("finalized fixture detail");
    assert_eq!(finalized.status, TrackStatus::Finalized);
    assert!(finalized.certificate.valid);
    certificate::verify(&fixture.track_root).expect("verify generated fixture certificate");
}

fn run_phase_24_restart_pass(
    fixture: &Phase24Fixture,
    finalized: &Phase24FinalizedPass,
) -> Phase24AfterPass {
    // 11. Start a new application object.
    let reopened =
        WorkspaceApp::open(&fixture.workspace, false).expect("restart Phase 24 workspace");

    // 12. Confirm that the newly opened object owns the same workspace.
    assert_eq!(reopened.root(), fixture.workspace.as_path());

    // 13. Reload track, profile, SQLite, and evidence state.
    let reloaded = reopened
        .load_track(&fixture.ready.id)
        .expect("reload finalized fixture track");
    let reloaded_record = reopened
        .persistence
        .track(&fixture.ready.id)
        .expect("reload finalized SQLite record");
    let reloaded_evidence = evidence_snapshot(
        &reopened
            .persistence
            .evidence(&fixture.ready.id)
            .expect("reload finalized evidence"),
    );
    assert_eq!(
        reopened.profile().expect("reload controlled profile"),
        finalized.controlled_profile
    );
    assert_eq!(reloaded.status, TrackStatus::Finalized);
    assert!(reloaded.certificate.valid);
    assert_eq!(
        serde_json::to_value(&reloaded_record).expect("serialize reloaded record"),
        serde_json::to_value(&finalized.finalized_record).expect("serialize finalized record")
    );
    assert_eq!(reloaded_evidence, finalized.finalized_evidence);
    let schema_after = schema_version(&reopened);
    assert_eq!(schema_after, fixture.schema_before);
    certificate::verify(&fixture.track_root).expect("verify certificate after restart");
    assert!(
        integrity::verify(&fixture.track_root)
            .expect("verify main hashes after restart")
            .verified
    );
    let database_after = logical_database_snapshot(&reopened);
    assert_eq!(
        database_after, finalized.finalized_database,
        "restart/reload changed normalized logical SQLite contents"
    );
    assert_expected_database_changes(
        &fixture.database_before,
        &database_after,
        &fixture.track_before,
        &finalized.finalized_record,
        &finalized.controlled_profile,
    );
    drop(reopened);

    // 14. Capture after hashes from the same three regular-file sets.
    Phase24AfterPass {
        fixture_hashes: file_hashes(&fixture.fixture_root),
        track_hashes: file_hashes(&fixture.track_root),
        workspace_hashes: file_hashes(&fixture.workspace),
        schema_version: schema_after,
        database: database_after,
    }
}

fn assert_phase_24_hash_classification(
    fixture: &Phase24Fixture,
    after: &Phase24AfterPass,
) -> Phase24DiffClasses {
    // 15. Split every changed regular-file path into the required classes.
    let expected_output_paths = expected_certificate_paths(&fixture.ready.relative_path);
    let permitted_metadata_paths =
        BTreeSet::from([crate::persistence::DATABASE_RELATIVE_PATH.into()]);
    let classes = classify_changes(
        &fixture.workspace_hashes_before,
        &after.workspace_hashes,
        &expected_output_paths,
        &permitted_metadata_paths,
    );
    assert_eq!(
        classes
            .expected_test_output
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_output_paths
    );
    assert_eq!(
        classes
            .permitted_fixture_metadata_change
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        permitted_metadata_paths
    );
    assert!(
        classes.unexpected_product_data_change.is_empty(),
        "unexpected product-data changes: {:?}",
        classes.unexpected_product_data_change
    );

    // 16. Confirm that no source, evidence, existing portable file, schema,
    // or finalized product fact changed outside the explicit classifications.
    assert_eq!(after.fixture_hashes, fixture.fixture_hashes_before);
    for (relative, digest) in &fixture.track_hashes_before {
        assert_eq!(
            after.track_hashes.get(relative),
            Some(digest),
            "pre-existing portable fixture file changed: {relative}"
        );
    }
    assert_eq!(after.schema_version, fixture.schema_before);

    classes
}

fn emit_phase_24_evidence(
    fixture: &Phase24Fixture,
    after: &Phase24AfterPass,
    classes: &Phase24DiffClasses,
) {
    let steps = phase_24_step_results();
    assert_eq!(steps.len(), 16);
    eprintln!(
        "PHASE_24_COMPATIBILITY_EVIDENCE={} ",
        serde_json::to_string_pretty(&serde_json::json!({
            "steps": steps,
            "before": {
                "fixtureRegularFileTreeSha256": hash_map_digest(&fixture.fixture_hashes_before),
                "trackRegularFileTreeSha256": hash_map_digest(&fixture.track_hashes_before),
                "workspaceRegularFileTreeSha256": hash_map_digest(&fixture.workspace_hashes_before),
                "logicalDatabaseSha256": logical_database_digest(&fixture.database_before),
                "files": fixture.workspace_hashes_before,
            },
            "after": {
                "fixtureRegularFileTreeSha256": hash_map_digest(&after.fixture_hashes),
                "trackRegularFileTreeSha256": hash_map_digest(&after.track_hashes),
                "workspaceRegularFileTreeSha256": hash_map_digest(&after.workspace_hashes),
                "logicalDatabaseSha256": logical_database_digest(&after.database),
                "files": after.workspace_hashes,
            },
            "classification": classes,
            "schemaVersionBefore": fixture.schema_before,
            "schemaVersionAfter": after.schema_version,
        }))
        .expect("serialize Phase 24 evidence")
    );
}

fn file_hashes(root: &Path) -> BTreeMap<String, String> {
    WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .map(|entry| entry.expect("walk isolated fixture tree"))
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .expect("fixture-relative file path");
            (
                portable_relative(relative),
                sha256_file(entry.path()).expect("hash isolated fixture file"),
            )
        })
        .collect()
}

fn evidence_snapshot(items: &[EvidenceItem]) -> BTreeMap<String, serde_json::Value> {
    items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                serde_json::to_value(item).expect("serialize evidence snapshot"),
            )
        })
        .collect()
}

fn schema_version(app: &WorkspaceApp) -> i64 {
    app.persistence
        .open()
        .expect("open isolated fixture database")
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read isolated fixture schema version")
}

fn expected_certificate_paths(track_relative: &str) -> BTreeSet<String> {
    [
        certificate::CERTIFICATE_FILE,
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_HASH_FILE,
        certificate::PDF_FILE,
        certificate::PDF_FILE_DE,
    ]
    .into_iter()
    .map(|relative| portable_relative(&Path::new(track_relative).join(relative)))
    .collect()
}

fn classify_changes(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    expected_output_paths: &BTreeSet<String>,
    permitted_metadata_paths: &BTreeSet<String>,
) -> Phase24DiffClasses {
    let paths = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut result = Phase24DiffClasses::default();
    for path in paths {
        let old = before.get(&path);
        let new = after.get(&path);
        if old == new {
            continue;
        }
        let description = match (old, new) {
            (None, Some(digest)) => format!("ADDED {digest}"),
            (Some(digest), None) => format!("REMOVED {digest}"),
            (Some(old), Some(new)) => format!("MODIFIED {old} -> {new}"),
            (None, None) => unreachable!("union path must exist in one tree"),
        };
        if expected_output_paths.contains(&path) {
            result.expected_test_output.insert(path, description);
        } else if permitted_metadata_paths.contains(&path) {
            result
                .permitted_fixture_metadata_change
                .insert(path, description);
        } else {
            result
                .unexpected_product_data_change
                .insert(path, description);
        }
    }
    result
}

fn hash_map_digest(hashes: &BTreeMap<String, String>) -> String {
    let mut canonical = Vec::new();
    for (relative, digest) in hashes {
        canonical.extend_from_slice(relative.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(digest.as_bytes());
        canonical.push(b'\n');
    }
    sha256_bytes(&canonical)
}

fn phase_24_step_results() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("01_before_hashes", "PASS"),
        ("02_open_workspace", "PASS"),
        ("03_detect_tracks", "PASS"),
        ("04_load_sqlite", "PASS"),
        ("05_detect_evidence", "PASS"),
        ("06_generate_documents", "PASS"),
        ("07_verify_existing_hashes", "PASS"),
        ("08_generate_and_verify_certificate", "PASS"),
        ("09_controlled_fixture_write", "PASS"),
        ("10_clean_close", "PASS"),
        ("11_restart", "PASS"),
        ("12_reopen_same_workspace", "PASS"),
        ("13_reload_data", "PASS"),
        ("14_after_hashes", "PASS"),
        ("15_classify_changes", "PASS"),
        ("16_no_unintended_data_migration", "PASS"),
    ])
}
