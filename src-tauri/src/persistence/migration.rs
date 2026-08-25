use super::SCHEMA_VERSION;
use crate::error::{AppError, Result};
use rusqlite::Connection;

pub fn migrate(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction()?;
    let version: i64 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(AppError::Data(format!(
            "Workspace database schema {version} is newer than supported schema {SCHEMA_VERSION}."
        )));
    }
    if version < 1 {
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS profile(singleton INTEGER PRIMARY KEY CHECK(singleton=1),data_json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS tracks(
               id TEXT PRIMARY KEY,title TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,status TEXT NOT NULL,
               workflow_id TEXT NOT NULL,workflow_version TEXT NOT NULL,data_json TEXT NOT NULL,
               created_at TEXT NOT NULL,updated_at TEXT NOT NULL,legacy INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS evidence(
               id TEXT PRIMARY KEY,track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
               role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL,
               sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
               verification_error TEXT,source_global_evidence_id TEXT,coverage_start TEXT,coverage_end TEXT,
               UNIQUE(track_id,relative_path)
             );
             CREATE INDEX IF NOT EXISTS evidence_track_role ON evidence(track_id,role);
             CREATE TABLE IF NOT EXISTS step_states(
               track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,step_id TEXT NOT NULL,status TEXT NOT NULL,
               na_reason TEXT,updated_at TEXT,PRIMARY KEY(track_id,step_id)
             );
             CREATE TABLE IF NOT EXISTS deviations(
               id TEXT PRIMARY KEY,track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,title TEXT NOT NULL,
               description TEXT NOT NULL,blocking INTEGER NOT NULL,resolved INTEGER NOT NULL,
               created_at TEXT NOT NULL,resolved_at TEXT
             );
             CREATE INDEX IF NOT EXISTS deviations_track ON deviations(track_id);
             CREATE TABLE IF NOT EXISTS global_evidence(
               id TEXT PRIMARY KEY,role TEXT NOT NULL,file_name TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,
               sha256 TEXT,size_bytes INTEGER NOT NULL,imported_at TEXT NOT NULL,verified INTEGER NOT NULL,
               verification_error TEXT,coverage_start TEXT,coverage_end TEXT,notes TEXT
             );
             PRAGMA user_version=1;",
        )?;
    }
    if version < 2 {
        transaction.execute_batch(
            "ALTER TABLE evidence ADD COLUMN provenance TEXT NOT NULL DEFAULT 'managed_copy';
             ALTER TABLE evidence ADD COLUMN derived_from_evidence_id TEXT;
             ALTER TABLE evidence ADD COLUMN generator_version TEXT;
             ALTER TABLE evidence ADD COLUMN generated_disclosure_text TEXT;
             UPDATE evidence
             SET provenance = CASE
               WHEN EXISTS (
                 SELECT 1 FROM tracks
                 WHERE tracks.id = evidence.track_id AND tracks.legacy <> 0
               ) THEN 'indexed_legacy'
               ELSE 'managed_copy'
             END;
             PRAGMA user_version=2;",
        )?;
    }
    if version < 3 {
        transaction.execute_batch(
            "ALTER TABLE evidence ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';
             PRAGMA user_version=3;",
        )?;
    }
    if version < 4 {
        transaction.execute_batch(
            "ALTER TABLE global_evidence ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';
             PRAGMA user_version=4;",
        )?;
    }
    if version < 5 {
        transaction.execute_batch(
            "CREATE TABLE external_timestamp_records(
               id TEXT PRIMARY KEY,
               track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
               certificate_id TEXT NOT NULL,
               imported_at TEXT NOT NULL,
               data_json TEXT NOT NULL
             );
             CREATE INDEX external_timestamp_track_certificate
             ON external_timestamp_records(track_id,certificate_id,imported_at,id);
             PRAGMA user_version=5;",
        )?;
    }
    if version < 6 {
        transaction.execute_batch(
            "CREATE TABLE timestamp_settings(
               singleton INTEGER PRIMARY KEY CHECK(singleton=1),
               data_json TEXT NOT NULL
             );
             CREATE TABLE timestamp_attachment_status(
               track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
               certificate_id TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               data_json TEXT NOT NULL,
               PRIMARY KEY(track_id,certificate_id)
             );
             CREATE INDEX timestamp_attachment_status_track
             ON timestamp_attachment_status(track_id,updated_at);
             PRAGMA user_version=6;",
        )?;
    }
    if version < 7 {
        transaction.execute_batch(
            "CREATE TABLE audio_screening_settings(
               singleton INTEGER PRIMARY KEY CHECK(singleton=1),
               data_json TEXT NOT NULL
             );
             PRAGMA user_version=7;",
        )?;
    }
    transaction.commit()?;
    Ok(())
}
