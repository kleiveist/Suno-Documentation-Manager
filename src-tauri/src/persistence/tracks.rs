use super::Persistence;
use crate::error::{AppError, Result};
use crate::model::{EvidenceItem, Profile, TrackRecord};
use rusqlite::{params, OptionalExtension};

impl Persistence {
    pub fn save_profile_and_tracks(&self, profile: &Profile, tracks: &[TrackRecord]) -> Result<()> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let profile_json = serde_json::to_string(profile)?;
        transaction.execute(
            "INSERT INTO profile(singleton,data_json) VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET data_json=excluded.data_json",
            [profile_json],
        )?;
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

    /// Persists evidence records and the track state derived from them as one
    /// database unit. Filesystem callers stage or publish the managed bytes
    /// first and roll those changes back if this transaction fails.
    pub fn save_track_and_evidence(
        &self,
        track: &TrackRecord,
        evidence: &[EvidenceItem],
    ) -> Result<()> {
        let track_json = serde_json::to_string(track)?;
        let evidence_json = evidence
            .iter()
            .map(|item| serde_json::to_string(&item.metadata))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;

        for (item, metadata_json) in evidence.iter().zip(evidence_json) {
            let result = transaction.execute(
                "INSERT INTO evidence(id,track_id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text,metadata_json)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
                 ON CONFLICT(id) DO UPDATE SET role=excluded.role,file_name=excluded.file_name,
                 relative_path=excluded.relative_path,sha256=excluded.sha256,size_bytes=excluded.size_bytes,
                 imported_at=excluded.imported_at,
                 verified=excluded.verified,verification_error=excluded.verification_error,
                 source_global_evidence_id=excluded.source_global_evidence_id,coverage_start=excluded.coverage_start,coverage_end=excluded.coverage_end,
                 provenance=excluded.provenance,derived_from_evidence_id=excluded.derived_from_evidence_id,
                 generator_version=excluded.generator_version,generated_disclosure_text=excluded.generated_disclosure_text,
                 metadata_json=excluded.metadata_json",
                params![
                    item.id,
                    track.id,
                    item.role.as_str(),
                    item.file_name,
                    item.relative_path,
                    item.sha256,
                    item.size_bytes as i64,
                    item.imported_at,
                    item.verified as i64,
                    item.verification_error,
                    item.source_global_evidence_id,
                    item.coverage_start,
                    item.coverage_end,
                    item.provenance.as_str(),
                    item.derived_from_evidence_id,
                    item.generator_version,
                    item.generated_disclosure_text,
                    metadata_json
                ],
            );
            if let Err(error) = result {
                if matches!(
                    error,
                    rusqlite::Error::SqliteFailure(ref code, _)
                        if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                ) {
                    return Err(AppError::Validation(
                        "Die Evidence konnte wegen einer bereits belegten Zuordnung nicht gespeichert werden. Verwende zum Austausch den Upload-Button an der vorhandenen Evidence."
                            .into(),
                    ));
                }
                return Err(AppError::Database(error));
            }
        }

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
                track_json,
                track.created_at,
                track.updated_at,
                track.legacy as i64
            ],
        )?;
        transaction.commit()?;
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
}
