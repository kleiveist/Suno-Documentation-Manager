use crate::error::{AppError, Result};
use crate::model::{
    BlockingDeviation, EvidenceItem, EvidenceProvenance, EvidenceRole, GlobalEvidenceItem, Profile,
    StepState, StepStatus, TrackRecord,
};
use crate::security::{contained_path, ensure_contained_directory};
use crate::workflow;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub const DATABASE_RELATIVE_PATH: &str = ".suno-doc/workspace.sqlite";
pub const SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Clone)]
pub struct Persistence {
    root: PathBuf,
}

impl Persistence {
    pub fn initialize(root: &Path) -> Result<Self> {
        ensure_contained_directory(root, Path::new(".suno-doc"))?;
        ensure_contained_directory(root, Path::new(".suno-doc/config"))?;
        ensure_contained_directory(root, Path::new(".suno-doc/global-evidence"))?;
        let this = Self {
            root: root.to_owned(),
        };
        let mut connection = this.open()?;
        migrate(&mut connection)?;
        Ok(this)
    }

    pub fn open(&self) -> Result<Connection> {
        let path = contained_path(&self.root, Path::new(DATABASE_RELATIVE_PATH), false)?;
        let connection = Connection::open(&path).map_err(AppError::Database)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(AppError::Database)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(AppError::Database)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(AppError::Database)?;
        Ok(connection)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .open()?
            .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn profile(&self) -> Result<Profile> {
        let json: Option<String> = self
            .open()?
            .query_row(
                "SELECT data_json FROM profile WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .unwrap_or_else(|| Ok(Profile::default()))
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        let json = serde_json::to_string(profile)?;
        self.open()?.execute(
            "INSERT INTO profile(singleton,data_json) VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET data_json=excluded.data_json",
            [json],
        )?;
        Ok(())
    }

    pub fn save_track(&self, track: &TrackRecord) -> Result<()> {
        let json = serde_json::to_string(track)?;
        self.open()?.execute(
            "INSERT INTO tracks(id,title,relative_path,status,workflow_id,workflow_version,data_json,created_at,updated_at,legacy)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,relative_path=excluded.relative_path,
             status=excluded.status,workflow_id=excluded.workflow_id,workflow_version=excluded.workflow_version,
             data_json=excluded.data_json,updated_at=excluded.updated_at,legacy=excluded.legacy",
            params![
                track.id,
                track.fields.title,
                track.relative_path,
                track.status.as_str(),
                track.workflow_id,
                track.workflow_version,
                json,
                track.created_at,
                track.updated_at,
                track.legacy as i64
            ],
        )?;
        Ok(())
    }

    pub fn save_tracks(&self, tracks: &[TrackRecord]) -> Result<()> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        for track in tracks {
            let json = serde_json::to_string(track)?;
            transaction.execute(
                "INSERT INTO tracks(id,title,relative_path,status,workflow_id,workflow_version,data_json,created_at,updated_at,legacy)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title,relative_path=excluded.relative_path,
                 status=excluded.status,workflow_id=excluded.workflow_id,workflow_version=excluded.workflow_version,
                 data_json=excluded.data_json,updated_at=excluded.updated_at,legacy=excluded.legacy",
                params![
                    track.id,
                    track.fields.title,
                    track.relative_path,
                    track.status.as_str(),
                    track.workflow_id,
                    track.workflow_version,
                    json,
                    track.created_at,
                    track.updated_at,
                    track.legacy as i64
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_track(&self, id: &str) -> Result<()> {
        self.open()?
            .execute("DELETE FROM tracks WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn save_track_clearing_steps(&self, track: &TrackRecord) -> Result<()> {
        let json = serde_json::to_string(track)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO tracks(id,title,relative_path,status,workflow_id,workflow_version,data_json,created_at,updated_at,legacy)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,relative_path=excluded.relative_path,
             status=excluded.status,workflow_id=excluded.workflow_id,workflow_version=excluded.workflow_version,
             data_json=excluded.data_json,updated_at=excluded.updated_at,legacy=excluded.legacy",
            params![
                track.id,
                track.fields.title,
                track.relative_path,
                track.status.as_str(),
                track.workflow_id,
                track.workflow_version,
                json,
                track.created_at,
                track.updated_at,
                track.legacy as i64
            ],
        )?;
        transaction.execute("DELETE FROM step_states WHERE track_id=?1", [&track.id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn track(&self, id: &str) -> Result<TrackRecord> {
        let json: Option<String> = self
            .open()?
            .query_row("SELECT data_json FROM tracks WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        let json = json.ok_or_else(|| AppError::TrackNotFound(id.into()))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn tracks(&self) -> Result<Vec<TrackRecord>> {
        let connection = self.open()?;
        let mut statement =
            connection.prepare("SELECT data_json FROM tracks ORDER BY lower(title), id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(serde_json::from_str(&row?)?);
        }
        Ok(tracks)
    }

    pub fn track_by_relative_path(&self, relative_path: &str) -> Result<Option<TrackRecord>> {
        let json: Option<String> = self
            .open()?
            .query_row(
                "SELECT data_json FROM tracks WHERE relative_path=?1",
                [relative_path],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .transpose()
    }

    pub fn save_evidence(&self, track_id: &str, evidence: &EvidenceItem) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO evidence(id,track_id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(id) DO UPDATE SET sha256=excluded.sha256,size_bytes=excluded.size_bytes,
             verified=excluded.verified,verification_error=excluded.verification_error,
             source_global_evidence_id=excluded.source_global_evidence_id,coverage_start=excluded.coverage_start,coverage_end=excluded.coverage_end,
             provenance=excluded.provenance,derived_from_evidence_id=excluded.derived_from_evidence_id,
             generator_version=excluded.generator_version,generated_disclosure_text=excluded.generated_disclosure_text",
            params![
                evidence.id,
                track_id,
                evidence.role.as_str(),
                evidence.file_name,
                evidence.relative_path,
                evidence.sha256,
                evidence.size_bytes as i64,
                evidence.imported_at,
                evidence.verified as i64,
                evidence.verification_error,
                evidence.source_global_evidence_id,
                evidence.coverage_start,
                evidence.coverage_end,
                evidence.provenance.as_str(),
                evidence.derived_from_evidence_id,
                evidence.generator_version,
                evidence.generated_disclosure_text
            ],
        )?;
        Ok(())
    }

    pub fn evidence(&self, track_id: &str) -> Result<Vec<EvidenceItem>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text
             FROM evidence WHERE track_id=?1 ORDER BY imported_at,id",
        )?;
        let rows = statement.query_map([track_id], evidence_from_row)?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        Ok(values)
    }

    pub fn evidence_item(&self, track_id: &str, id: &str) -> Result<EvidenceItem> {
        self.open()?
            .query_row(
                "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text
                 FROM evidence WHERE track_id=?1 AND id=?2",
                params![track_id, id],
                evidence_from_row,
            )
            .optional()?
            .ok_or_else(|| AppError::EvidenceNotFound(id.into()))
    }

    pub fn remove_evidence(&self, track_id: &str, id: &str) -> Result<()> {
        let count = self.open()?.execute(
            "DELETE FROM evidence WHERE track_id=?1 AND id=?2",
            params![track_id, id],
        )?;
        if count == 0 {
            return Err(AppError::EvidenceNotFound(id.into()));
        }
        Ok(())
    }

    pub fn stored_steps(&self, track_id: &str) -> Result<Vec<StepState>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT step_id,status,na_reason,updated_at FROM step_states WHERE track_id=?1 ORDER BY step_id",
        )?;
        let rows = statement.query_map([track_id], |row| {
            let status: String = row.get(1)?;
            Ok(StepState {
                id: row.get(0)?,
                status: parse_step_status(&status),
                na_reason: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn save_step(&self, track_id: &str, state: &StepState) -> Result<()> {
        let status = serde_json::to_value(&state.status)?
            .as_str()
            .unwrap_or("NOT_RUN")
            .to_owned();
        self.open()?.execute(
            "INSERT INTO step_states(track_id,step_id,status,na_reason,updated_at) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(track_id,step_id) DO UPDATE SET status=excluded.status,na_reason=excluded.na_reason,updated_at=excluded.updated_at",
            params![track_id, state.id, status, state.na_reason, state.updated_at],
        )?;
        Ok(())
    }

    pub fn clear_step(&self, track_id: &str, step_id: &str) -> Result<()> {
        self.open()?.execute(
            "DELETE FROM step_states WHERE track_id=?1 AND step_id=?2",
            params![track_id, step_id],
        )?;
        Ok(())
    }

    pub fn deviations(&self, track_id: &str) -> Result<Vec<BlockingDeviation>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id,title,description,blocking,resolved,created_at,resolved_at FROM deviations WHERE track_id=?1 ORDER BY created_at,id",
        )?;
        let rows = statement.query_map([track_id], |row| {
            Ok(BlockingDeviation {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                blocking: row.get::<_, i64>(3)? != 0,
                resolved: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
                resolved_at: row.get(6)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn save_deviation(&self, track_id: &str, deviation: &BlockingDeviation) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO deviations(id,track_id,title,description,blocking,resolved,created_at,resolved_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,
             blocking=excluded.blocking,resolved=excluded.resolved,resolved_at=excluded.resolved_at",
            params![
                deviation.id,
                track_id,
                deviation.title,
                deviation.description,
                deviation.blocking as i64,
                deviation.resolved as i64,
                deviation.created_at,
                deviation.resolved_at
            ],
        )?;
        Ok(())
    }

    pub fn remove_deviation(&self, track_id: &str, id: &str) -> Result<()> {
        let count = self.open()?.execute(
            "DELETE FROM deviations WHERE track_id=?1 AND id=?2",
            params![track_id, id],
        )?;
        if count == 0 {
            return Err(AppError::Validation(format!("Deviation not found: {id}")));
        }
        Ok(())
    }

    pub fn save_global_evidence(&self, evidence: &GlobalEvidenceItem) -> Result<()> {
        let e = &evidence.evidence;
        self.open()?.execute(
            "INSERT INTO global_evidence(id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,coverage_start,coverage_end,notes)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(id) DO UPDATE SET sha256=excluded.sha256,size_bytes=excluded.size_bytes,
             verified=excluded.verified,verification_error=excluded.verification_error,
             coverage_start=excluded.coverage_start,coverage_end=excluded.coverage_end,notes=excluded.notes",
            params![e.id,e.role.as_str(),e.file_name,e.relative_path,e.sha256,e.size_bytes as i64,e.imported_at,e.verified as i64,e.verification_error,e.coverage_start,e.coverage_end,evidence.notes],
        )?;
        Ok(())
    }

    pub fn global_evidence(&self) -> Result<Vec<GlobalEvidenceItem>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,coverage_start,coverage_end,notes FROM global_evidence ORDER BY imported_at,id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(GlobalEvidenceItem {
                evidence: EvidenceItem {
                    id: row.get(0)?,
                    role: parse_role_sql(row.get::<_, String>(1)?)?,
                    file_name: row.get(2)?,
                    relative_path: row.get(3)?,
                    sha256: row.get(4)?,
                    size_bytes: row.get::<_, i64>(5)? as u64,
                    imported_at: row.get(6)?,
                    verified: row.get::<_, i64>(7)? != 0,
                    verification_error: row.get(8)?,
                    source_global_evidence_id: None,
                    coverage_start: row.get(9)?,
                    coverage_end: row.get(10)?,
                    provenance: EvidenceProvenance::ManagedCopy,
                    derived_from_evidence_id: None,
                    generator_version: None,
                    generated_disclosure_text: None,
                },
                notes: row.get(11)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn global_evidence_item(&self, id: &str) -> Result<GlobalEvidenceItem> {
        self.global_evidence()?
            .into_iter()
            .find(|item| item.evidence.id == id)
            .ok_or_else(|| AppError::EvidenceNotFound(id.into()))
    }

    pub fn remove_global_evidence(&self, id: &str) -> Result<()> {
        let count = self
            .open()?
            .execute("DELETE FROM global_evidence WHERE id=?1", [id])?;
        if count == 0 {
            return Err(AppError::EvidenceNotFound(id.into()));
        }
        Ok(())
    }
}

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
    transaction.commit()?;
    Ok(())
}

fn evidence_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceItem> {
    Ok(EvidenceItem {
        id: row.get(0)?,
        role: parse_role_sql(row.get::<_, String>(1)?)?,
        file_name: row.get(2)?,
        relative_path: row.get(3)?,
        sha256: row.get(4)?,
        size_bytes: row.get::<_, i64>(5)? as u64,
        imported_at: row.get(6)?,
        verified: row.get::<_, i64>(7)? != 0,
        verification_error: row.get(8)?,
        source_global_evidence_id: row.get(9)?,
        coverage_start: row.get(10)?,
        coverage_end: row.get(11)?,
        provenance: parse_provenance_sql(row.get::<_, String>(12)?)?,
        derived_from_evidence_id: row.get(13)?,
        generator_version: row.get(14)?,
        generated_disclosure_text: row.get(15)?,
    })
}

fn parse_provenance_sql(value: String) -> rusqlite::Result<EvidenceProvenance> {
    match value.as_str() {
        "managed_copy" => Ok(EvidenceProvenance::ManagedCopy),
        "indexed_legacy" => Ok(EvidenceProvenance::IndexedLegacy),
        "generated_disclosure" => Ok(EvidenceProvenance::GeneratedDisclosure),
        "global_copy" => Ok(EvidenceProvenance::GlobalCopy),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            12,
            rusqlite::types::Type::Text,
            format!("Unknown evidence provenance: {value}").into(),
        )),
    }
}

fn parse_role_sql(value: String) -> rusqlite::Result<EvidenceRole> {
    workflow::evidence_role_from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn parse_step_status(value: &str) -> StepStatus {
    match value {
        "PASS" => StepStatus::Pass,
        "FAIL" => StepStatus::Fail,
        "BLOCKED" => StepStatus::Blocked,
        "N_A" => StepStatus::NotApplicable,
        "NOT_VERIFIED" => StepStatus::NotVerified,
        _ => StepStatus::NotRun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                 ('metadata','profile','tracks','evidence','step_states','deviations','global_evidence')",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(tables, 7);
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
        };

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
        std::fs::create_dir_all(workspace.join("Track/01_RELEASE"))
            .expect("portable track directory");
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
}
