use super::Persistence;
use crate::error::{AppError, Result};
use crate::model::{
    BlockingDeviation, EvidenceItem, EvidenceProvenance, EvidenceRole, GlobalEvidenceItem,
    StepState, StepStatus, TrackRecord,
};
use crate::workflow;
use rusqlite::{params, OptionalExtension};

impl Persistence {
    pub fn save_evidence(&self, track_id: &str, evidence: &EvidenceItem) -> Result<()> {
        let result = self.open()?.execute(
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
                evidence.generated_disclosure_text,
                serde_json::to_string(&evidence.metadata)?
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
        Ok(())
    }

    pub fn evidence_by_relative_path(
        &self,
        track_id: &str,
        relative_path: &str,
    ) -> Result<Option<EvidenceItem>> {
        self.open()?
            .query_row(
                "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text,metadata_json
                 FROM evidence WHERE track_id=?1 AND relative_path=?2",
                params![track_id, relative_path],
                evidence_from_row,
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn evidence(&self, track_id: &str) -> Result<Vec<EvidenceItem>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text,metadata_json
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
                "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,source_global_evidence_id,coverage_start,coverage_end,provenance,derived_from_evidence_id,generator_version,generated_disclosure_text,metadata_json
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
            "INSERT INTO global_evidence(id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,coverage_start,coverage_end,notes,metadata_json)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET sha256=excluded.sha256,size_bytes=excluded.size_bytes,
             verified=excluded.verified,verification_error=excluded.verification_error,
             coverage_start=excluded.coverage_start,coverage_end=excluded.coverage_end,notes=excluded.notes,
             metadata_json=excluded.metadata_json",
            params![e.id,e.role.as_str(),e.file_name,e.relative_path,e.sha256,e.size_bytes as i64,e.imported_at,e.verified as i64,e.verification_error,e.coverage_start,e.coverage_end,evidence.notes,serde_json::to_string(&e.metadata)?],
        )?;
        Ok(())
    }

    /// Updates descriptive metadata on one reusable evidence record and all
    /// explicitly supplied, still-mutable portable copies as one database unit.
    /// Finalized/superseded tracks are filtered by the application before this
    /// method is called; this transaction prevents partially propagated context.
    pub fn save_global_evidence_and_copies(
        &self,
        global: &GlobalEvidenceItem,
        copies: &[(TrackRecord, EvidenceItem)],
    ) -> Result<()> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let global_metadata = serde_json::to_string(&global.evidence.metadata)?;
        let global_count = transaction.execute(
            "UPDATE global_evidence SET metadata_json=?1,notes=?2 WHERE id=?3",
            params![global_metadata, global.notes, global.evidence.id],
        )?;
        if global_count == 0 {
            return Err(AppError::EvidenceNotFound(global.evidence.id.clone()));
        }

        for (track, evidence) in copies {
            let evidence_count = transaction.execute(
                "UPDATE evidence SET metadata_json=?1 WHERE id=?2 AND track_id=?3",
                params![
                    serde_json::to_string(&evidence.metadata)?,
                    evidence.id,
                    track.id
                ],
            )?;
            if evidence_count == 0 {
                return Err(AppError::EvidenceNotFound(evidence.id.clone()));
            }
            let track_json = serde_json::to_string(track)?;
            let track_count = transaction.execute(
                "UPDATE tracks SET title=?1,relative_path=?2,status=?3,workflow_id=?4,
                 workflow_version=?5,data_json=?6,updated_at=?7,legacy=?8 WHERE id=?9",
                params![
                    track.fields.title,
                    track.relative_path,
                    track.status.as_str(),
                    track.workflow_id,
                    track.workflow_version,
                    track_json,
                    track.updated_at,
                    track.legacy as i64,
                    track.id
                ],
            )?;
            if track_count == 0 {
                return Err(AppError::TrackNotFound(track.id.clone()));
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn global_evidence(&self) -> Result<Vec<GlobalEvidenceItem>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id,role,file_name,relative_path,sha256,size_bytes,imported_at,verified,verification_error,coverage_start,coverage_end,notes,metadata_json FROM global_evidence ORDER BY imported_at,id",
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
                    metadata: serde_json::from_str(&row.get::<_, String>(12)?).map_err(
                        |error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                12,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        },
                    )?,
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

fn evidence_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceItem> {
    let metadata = evidence_metadata_from_row(row)?;
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
        metadata,
    })
}

fn evidence_metadata_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<crate::model::EvidenceMetadata> {
    serde_json::from_str(&row.get::<_, String>(16)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(16, rusqlite::types::Type::Text, Box::new(error))
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
