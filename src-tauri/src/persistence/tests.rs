use super::{migrate, Persistence, DATABASE_RELATIVE_PATH, SCHEMA_VERSION};
use crate::error::AppError;
use crate::model::{
    AudioScreeningSecretInput, AudioScreeningSettings, EvidenceItem, EvidenceProvenance,
    EvidenceRole, TimestampSettings,
};
use rusqlite::{params, Connection};
use tempfile::tempdir;

#[test]
fn sqlite_migrations_are_idempotent() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    migrate(&mut connection).expect("first migration");
    migrate(&mut connection).expect("idempotent migration");

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("schema version");
    let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('metadata','profile','tracks','evidence','step_states','deviations','global_evidence','external_timestamp_records','timestamp_settings','timestamp_attachment_status','audio_screening_settings')",
                [],
                |row| row.get(0),
            )
            .expect("table count");
    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(tables, 11);
}

#[test]
fn timestamp_settings_and_secrets_are_stored_separately() {
    let directory = tempdir().expect("temporary workspace");
    let persistence = Persistence::initialize(directory.path()).expect("persistence");
    let secret = "never-export-this-timestamp-token";
    let settings = TimestampSettings {
        enabled: true,
        provider: crate::model::TimestampProviderKind::CustomRfc3161,
        auto_after_finalization: true,
        custom: crate::model::CustomRfc3161Settings {
            provider_name: "Private TSA".into(),
            endpoint: "https://timestamp.example.test/rfc3161".into(),
            authentication_mode: crate::model::TimestampAuthenticationMode::BearerToken,
            timeout_seconds: 12,
            ..Default::default()
        },
        ..Default::default()
    };
    persistence
        .save_timestamp_settings(&settings)
        .expect("save non-secret settings");
    persistence
        .save_timestamp_secret(Some(secret))
        .expect("save secret");

    assert_eq!(
        persistence
            .timestamp_settings()
            .expect("load settings")
            .provider,
        crate::model::TimestampProviderKind::CustomRfc3161
    );
    assert!(persistence
        .timestamp_secret_present()
        .expect("secret presence"));
    assert_eq!(
        persistence
            .timestamp_secret()
            .expect("read secret")
            .as_deref(),
        Some(secret)
    );
    let config = directory
        .path()
        .join(".suno-doc/config/timestamp-secrets.json");
    assert!(config.is_file());
    let settings_json: String = persistence
        .open()
        .expect("connection")
        .query_row(
            "SELECT data_json FROM timestamp_settings WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .expect("settings json");
    assert!(!settings_json.contains(secret));
    assert!(!serde_json::to_string(&settings)
        .expect("public settings JSON")
        .contains(secret));
    assert!(
        std::fs::read_to_string(directory.path().join(".suno-doc/.gitignore"))
            .expect("workspace gitignore")
            .contains("/config/timestamp-secrets.json")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(config)
                .expect("secret permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn audio_screening_settings_and_credentials_are_stored_separately() {
    let directory = tempdir().expect("temporary workspace");
    let persistence = Persistence::initialize(directory.path()).expect("persistence");
    let access_key = ["test", "-access-key"].concat();
    let access_secret = ["test", "-access-secret"].concat();
    let settings = AudioScreeningSettings {
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com".into(),
        timeout_seconds: 20,
        ..Default::default()
    };
    persistence
        .save_audio_screening_settings(&settings)
        .expect("save public settings");
    persistence
        .save_audio_screening_secret(AudioScreeningSecretInput {
            access_key: Some(access_key.clone()),
            access_secret: Some(access_secret.clone()),
        })
        .expect("save credentials");

    assert!(persistence
        .audio_screening_credentials_present()
        .expect("credentials present"));
    assert_eq!(
        persistence
            .audio_screening_credentials()
            .expect("read adapter credentials"),
        Some((access_key.clone(), access_secret.clone()))
    );
    let settings_json: String = persistence
        .open()
        .expect("connection")
        .query_row(
            "SELECT data_json FROM audio_screening_settings WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .expect("settings json");
    assert!(!settings_json.contains(&access_key));
    assert!(!settings_json.contains(&access_secret));
    assert!(!serde_json::to_string(&settings)
        .expect("public settings JSON")
        .contains(&access_secret));
    let config = directory
        .path()
        .join(".suno-doc/config/audio-screening-secrets.json");
    assert!(config.is_file());
    assert!(
        std::fs::read_to_string(directory.path().join(".suno-doc/.gitignore"))
            .expect("workspace gitignore")
            .contains("/config/audio-screening-secrets.json")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(config)
                .expect("credential permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn legacy_audio_screening_settings_load_new_coverage_defaults() {
    let directory = tempdir().expect("temporary workspace");
    let persistence = Persistence::initialize(directory.path()).expect("persistence");
    persistence
        .open()
        .expect("connection")
        .execute(
            "INSERT INTO audio_screening_settings(singleton,data_json) VALUES(1,?1)",
            [r#"{"enabled":true,"host":"identify-eu-west-1.acrcloud.com","timeoutSeconds":20}"#],
        )
        .expect("legacy settings row");

    let settings = persistence
        .audio_screening_settings()
        .expect("read legacy settings");
    assert_eq!(settings.intensity_percent, 5);
    assert!(settings.dynamic_by_track_duration);
    assert_eq!(settings.reference_duration_seconds, 300);
}

#[test]
fn sqlite_v1_migration_backfills_legacy_provenance_conservatively() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    connection
            .execute_batch(
                "CREATE TABLE tracks(id TEXT PRIMARY KEY,legacy INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE evidence(
                   id TEXT PRIMARY KEY,track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                   role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL,
                   sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
                   verification_error TEXT,source_global_evidence_id TEXT,coverage_start TEXT,coverage_end TEXT,
                   UNIQUE(track_id,relative_path)
                 );
                 CREATE TABLE global_evidence(
                   id TEXT PRIMARY KEY,role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,
                   sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
                   verification_error TEXT,coverage_start TEXT,coverage_end TEXT,notes TEXT
                 );
                 INSERT INTO tracks(id,legacy) VALUES('legacy-track',1),('managed-track',0);
                 INSERT INTO evidence(
                   id,track_id,role,file_name,relative_path,size_bytes,imported_at,verified
                 ) VALUES
                   ('legacy-evidence','legacy-track','other','history.txt','03_DOCUMENTATION/history.txt',7,'2026-08-01T00:00:00Z',0),
                   ('managed-evidence','managed-track','other','managed.txt','03_DOCUMENTATION/managed.txt',7,'2026-08-01T00:00:00Z',1);
                 PRAGMA user_version=1;",
            )
            .expect("v1 schema fixture");

    migrate(&mut connection).expect("v1 to v2 migration");

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("schema version");
    let legacy: (String, Option<String>, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT provenance,derived_from_evidence_id,generator_version,generated_disclosure_text
                 FROM evidence WHERE id='legacy-evidence'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migrated legacy evidence");
    let managed: String = connection
        .query_row(
            "SELECT provenance FROM evidence WHERE id='managed-evidence'",
            [],
            |row| row.get(0),
        )
        .expect("migrated managed evidence");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(legacy.0, "indexed_legacy");
    assert_eq!((legacy.1, legacy.2, legacy.3), (None, None, None));
    assert_eq!(managed, "managed_copy");
}

#[test]
fn evidence_provenance_fields_round_trip_and_update() {
    let directory = tempdir().expect("temporary directory");
    let persistence = Persistence::initialize(directory.path()).expect("persistence");
    persistence
            .open()
            .expect("connection")
            .execute(
                "INSERT INTO tracks(
                   id,title,relative_path,status,workflow_id,workflow_version,data_json,created_at,updated_at,legacy
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    "track-1",
                    "Track",
                    "Track",
                    "ACTIVE",
                    "suno-documentation-v1",
                    "1.0.0",
                    "{}",
                    "2026-08-01T00:00:00Z",
                    "2026-08-01T00:00:00Z",
                    0_i64
                ],
            )
            .expect("track fixture");
    let mut evidence = EvidenceItem {
        id: "generated-artwork".into(),
        role: EvidenceRole::AiArtworkEdited,
        file_name: "track_AI_EDITED.png".into(),
        relative_path: "05_ARTWORK/track_AI_EDITED.png".into(),
        sha256: Some("abc123".into()),
        size_bytes: 42,
        imported_at: "2026-08-01T00:00:00Z".into(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: EvidenceProvenance::GeneratedDisclosure,
        derived_from_evidence_id: Some("original-artwork".into()),
        generator_version: Some("local-disclosure-v1".into()),
        generated_disclosure_text: Some("AI-assisted".into()),
        metadata: Default::default(),
    };
    evidence.metadata.document_title = "Archived Suno Terms".into();
    evidence.metadata.provider = "Suno".into();
    evidence.metadata.source_url = "https://suno.example/terms".into();
    evidence.metadata.retrieval_date = "2026-08-01".into();

    persistence
        .save_evidence("track-1", &evidence)
        .expect("insert evidence");
    let loaded = persistence
        .evidence_item("track-1", &evidence.id)
        .expect("load inserted evidence");
    assert_eq!(loaded.provenance, EvidenceProvenance::GeneratedDisclosure);
    assert_eq!(
        loaded.derived_from_evidence_id.as_deref(),
        Some("original-artwork")
    );
    assert_eq!(
        loaded.generator_version.as_deref(),
        Some("local-disclosure-v1")
    );
    assert_eq!(
        loaded.generated_disclosure_text.as_deref(),
        Some("AI-assisted")
    );
    assert_eq!(loaded.metadata, evidence.metadata);

    evidence.provenance = EvidenceProvenance::GlobalCopy;
    evidence.derived_from_evidence_id = None;
    evidence.generator_version = None;
    evidence.generated_disclosure_text = None;
    evidence.source_global_evidence_id = Some("subscription-proof".into());
    persistence
        .save_evidence("track-1", &evidence)
        .expect("update evidence");

    let updated = persistence.evidence("track-1").expect("list evidence");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].provenance, EvidenceProvenance::GlobalCopy);
    assert_eq!(
        updated[0].source_global_evidence_id.as_deref(),
        Some("subscription-proof")
    );
    assert_eq!(updated[0].derived_from_evidence_id, None);
    assert_eq!(updated[0].generator_version, None);
    assert_eq!(updated[0].generated_disclosure_text, None);

    let duplicate = EvidenceItem {
        id: "duplicate-id".into(),
        ..evidence
    };
    let error = persistence
        .save_evidence("track-1", &duplicate)
        .expect_err("duplicate relative path must be controlled");
    assert!(matches!(error, AppError::Validation(message) if message.contains("Upload-Button")));
}

#[test]
fn sqlite_v2_migration_adds_empty_evidence_metadata_without_inventing_values() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    connection
            .execute_batch(
                "CREATE TABLE tracks(id TEXT PRIMARY KEY,legacy INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE evidence(
                   id TEXT PRIMARY KEY,track_id TEXT NOT NULL,role TEXT NOT NULL,file_name TEXT NOT NULL,
                   relative_path TEXT NOT NULL,sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,
                   verified INTEGER NOT NULL,verification_error TEXT,source_global_evidence_id TEXT,
                   coverage_start TEXT,coverage_end TEXT,provenance TEXT NOT NULL DEFAULT 'managed_copy',
                   derived_from_evidence_id TEXT,generator_version TEXT,generated_disclosure_text TEXT,
                   UNIQUE(track_id,relative_path)
                 );
                 CREATE TABLE global_evidence(
                   id TEXT PRIMARY KEY,role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,
                   sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
                   verification_error TEXT,coverage_start TEXT,coverage_end TEXT,notes TEXT
                 );
                 INSERT INTO tracks(id,legacy) VALUES('track-1',0);
                 INSERT INTO evidence(id,track_id,role,file_name,relative_path,size_bytes,imported_at,verified)
                 VALUES('evidence-1','track-1','other','old.txt','03_DOCUMENTATION/old.txt',3,'2026-01-01T00:00:00Z',1);
                 PRAGMA user_version=2;",
            )
            .expect("v2 fixture");
    migrate(&mut connection).expect("v2 to current migration");
    let value: String = connection
        .query_row(
            "SELECT metadata_json FROM evidence WHERE id='evidence-1'",
            [],
            |row| row.get(0),
        )
        .expect("metadata JSON");
    let metadata: crate::model::EvidenceMetadata =
        serde_json::from_str(&value).expect("metadata object");
    assert_eq!(metadata, crate::model::EvidenceMetadata::default());
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
}

#[test]
fn sqlite_v3_migration_adds_empty_global_evidence_metadata() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    connection
            .execute_batch(
                "CREATE TABLE global_evidence(
                   id TEXT PRIMARY KEY,role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,
                   sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
                   verification_error TEXT,coverage_start TEXT,coverage_end TEXT,notes TEXT
                 );
                 INSERT INTO global_evidence(
                   id,role,file_name,relative_path,size_bytes,imported_at,verified
                 ) VALUES(
                   'terms-1','suno_terms_rights','terms.html','.suno-doc/global-evidence/terms.html',
                   12,'2026-08-16T00:00:00Z',1
                 );
                 PRAGMA user_version=3;",
            )
            .expect("v3 fixture");

    migrate(&mut connection).expect("v3 to current migration");
    let value: String = connection
        .query_row(
            "SELECT metadata_json FROM global_evidence WHERE id='terms-1'",
            [],
            |row| row.get(0),
        )
        .expect("global metadata JSON");
    let metadata: crate::model::EvidenceMetadata =
        serde_json::from_str(&value).expect("metadata object");
    assert_eq!(metadata, crate::model::EvidenceMetadata::default());
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .expect("schema version"),
        SCHEMA_VERSION
    );
}

#[test]
fn sqlite_migration_refuses_newer_schema_without_modifying_it() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    connection
        .execute_batch("CREATE TABLE future_data(value TEXT); PRAGMA user_version=99;")
        .expect("future schema setup");

    let error = migrate(&mut connection).expect_err("newer schema must be refused");
    assert!(matches!(error, AppError::Data(_)));
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("unchanged schema version");
    let future_table: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='future_data'",
            [],
            |row| row.get(0),
        )
        .expect("future table count");
    assert_eq!(version, 99);
    assert_eq!(future_table, 1);
}

#[test]
fn sqlite_failed_migration_rolls_back_columns_data_and_user_version() {
    let mut connection = Connection::open_in_memory().expect("in-memory database");
    connection
            .execute_batch(
                "CREATE TABLE tracks(id TEXT PRIMARY KEY,legacy INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE evidence(
                   id TEXT PRIMARY KEY,track_id TEXT NOT NULL,
                   role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL,
                   sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
                   verification_error TEXT,source_global_evidence_id TEXT,coverage_start TEXT,coverage_end TEXT,
                   derived_from_evidence_id TEXT,
                   UNIQUE(track_id,relative_path)
                 );
                 CREATE TABLE migration_sentinel(value TEXT NOT NULL);
                 INSERT INTO tracks(id,legacy) VALUES('legacy-track',1);
                 INSERT INTO evidence(
                   id,track_id,role,file_name,relative_path,size_bytes,imported_at,verified,
                   derived_from_evidence_id
                 ) VALUES(
                   'evidence-1','legacy-track','other','history.txt','03_DOCUMENTATION/history.txt',7,
                   '2026-08-01T00:00:00Z',0,'preexisting-column-for-failure'
                 );
                 INSERT INTO migration_sentinel(value) VALUES('preserve-me');
                 PRAGMA user_version=1;",
            )
            .expect("failing v1 fixture");

    let error = migrate(&mut connection).expect_err("duplicate migration column must fail");
    assert!(matches!(error, AppError::Database(_)));

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("rolled-back schema version");
    assert_eq!(version, 1);
    let columns = connection
        .prepare("PRAGMA table_info(evidence)")
        .expect("evidence columns statement")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("evidence columns")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("column names");
    assert!(columns.contains(&"derived_from_evidence_id".to_owned()));
    assert!(!columns.contains(&"provenance".to_owned()));
    assert!(!columns.contains(&"generator_version".to_owned()));
    assert!(!columns.contains(&"generated_disclosure_text".to_owned()));
    let sentinel: String = connection
        .query_row("SELECT value FROM migration_sentinel", [], |row| row.get(0))
        .expect("preserved sentinel");
    assert_eq!(sentinel, "preserve-me");
    let evidence_count: i64 = connection
        .query_row("SELECT count(*) FROM evidence", [], |row| row.get(0))
        .expect("preserved evidence row");
    assert_eq!(evidence_count, 1);
}

#[test]
fn deleted_database_is_recreated_without_touching_portable_track_files() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).expect("workspace");
    let persistence = Persistence::initialize(&workspace).expect("initial persistence");
    persistence
        .set_meta("fixture", "before deletion")
        .expect("seed metadata");
    let portable_file = workspace.join("Portable Track/03_DOCUMENTATION/README.md");
    std::fs::create_dir_all(portable_file.parent().expect("portable parent"))
        .expect("portable track directories");
    std::fs::write(&portable_file, b"portable track bytes").expect("portable track fixture");
    std::fs::remove_file(workspace.join(DATABASE_RELATIVE_PATH)).expect("delete database");

    let recreated = Persistence::initialize(&workspace).expect("recreate database");
    assert_eq!(
        std::fs::read(&portable_file).expect("portable track remains"),
        b"portable track bytes"
    );
    assert_eq!(recreated.get_meta("fixture").expect("new metadata"), None);
    let version: i64 = recreated
        .open()
        .expect("recreated connection")
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("recreated schema version");
    assert_eq!(version, SCHEMA_VERSION);
}

#[test]
fn corrupted_database_returns_controlled_error_without_touching_track_files() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).expect("workspace");
    std::fs::create_dir_all(workspace.join(".suno-doc")).expect("admin directory");
    std::fs::create_dir_all(workspace.join("Track/01_RELEASE")).expect("portable track directory");
    let track_file = workspace.join("Track/01_RELEASE/final.wav");
    std::fs::write(&track_file, b"portable evidence bytes").expect("track evidence");
    std::fs::write(
        workspace.join(DATABASE_RELATIVE_PATH),
        b"this is deliberately not a SQLite database",
    )
    .expect("corrupt database fixture");

    let error = Persistence::initialize(&workspace).expect_err("corrupt database must fail");
    assert!(matches!(error, AppError::Database(_)));
    assert_eq!(
        std::fs::read(&track_file).expect("track evidence remains"),
        b"portable evidence bytes"
    );
    assert_eq!(
        std::fs::read(workspace.join(DATABASE_RELATIVE_PATH))
            .expect("corrupt database remains for recovery"),
        b"this is deliberately not a SQLite database"
    );
}

#[cfg(unix)]
#[test]
fn persistence_rejects_symlinked_admin_directory() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let outside = directory.path().join("outside");
    std::fs::create_dir(&workspace).expect("workspace");
    std::fs::create_dir(&outside).expect("outside");
    symlink(&outside, workspace.join(".suno-doc")).expect("admin symlink");

    let error = Persistence::initialize(&workspace).expect_err("symlink must be refused");
    assert!(matches!(error, AppError::Symlink(_)));
    assert!(!outside.join("workspace.sqlite").exists());
}
