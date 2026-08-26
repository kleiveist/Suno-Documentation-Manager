use super::Persistence;
use crate::error::{AppError, Result};
use crate::model::{
    AudioScreeningSecretInput, AudioScreeningSettings, ExternalTimestampRecord,
    ExternalTimestampSummary, TimestampSettings,
};
use crate::security::contained_path;
use rusqlite::{params, OptionalExtension};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

const TIMESTAMP_SECRETS_RELATIVE_PATH: &str = ".suno-doc/config/timestamp-secrets.json";
const AUDIO_SCREENING_SECRETS_RELATIVE_PATH: &str = ".suno-doc/config/audio-screening-secrets.json";

/// This type stays private to persistence so credentials can never become part
/// of a serializable public settings DTO. The file is separate from SQLite,
/// profile JSON, tracks, revisions, and certificate artifacts.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TimestampSecrets {
    secret: String,
}

/// Kept deliberately private to persistence: neither credential may enter a
/// serializable settings DTO, track JSON, manifest, certificate, or revision.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AudioScreeningSecrets {
    access_key: String,
    access_secret: String,
}

impl Persistence {
    /// Persist a post-finalization timestamp record without mutating the
    /// finalized track JSON or its phase-one evidence snapshot.
    pub fn save_external_timestamp(
        &self,
        track_id: &str,
        record: &ExternalTimestampRecord,
    ) -> Result<()> {
        // Current integrity is a load-time observation, not an immutable or
        // database-persisted assertion. It is recomputed whenever the record is
        // exposed through TrackDetail.
        let mut persisted = record.clone();
        persisted.integrity_verified = false;
        persisted.integrity_issues.clear();
        self.open()?.execute(
            "INSERT INTO external_timestamp_records(id,track_id,certificate_id,imported_at,data_json)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                persisted.id,
                track_id,
                persisted.certificate_id,
                persisted.imported_at,
                serde_json::to_string(&persisted)?
            ],
        )?;
        Ok(())
    }

    pub fn remove_external_timestamp(&self, track_id: &str, record_id: &str) -> Result<()> {
        self.open()?.execute(
            "DELETE FROM external_timestamp_records WHERE track_id=?1 AND id=?2",
            params![track_id, record_id],
        )?;
        Ok(())
    }

    pub fn external_timestamps(&self, track_id: &str) -> Result<Vec<ExternalTimestampRecord>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT data_json FROM external_timestamp_records
             WHERE track_id=?1 ORDER BY imported_at,id",
        )?;
        let rows = statement.query_map([track_id], |row| {
            let json = row.get::<_, String>(0)?;
            serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Global, non-secret provider settings. This deliberately has its own
    /// singleton table instead of living in `Profile`, because profiles are
    /// copied into editable track snapshots.
    pub fn timestamp_settings(&self) -> Result<TimestampSettings> {
        let json: Option<String> = self
            .open()?
            .query_row(
                "SELECT data_json FROM timestamp_settings WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .unwrap_or_else(|| Ok(TimestampSettings::default()))
    }

    pub fn save_timestamp_settings(&self, settings: &TimestampSettings) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO timestamp_settings(singleton,data_json) VALUES(1,?1)
             ON CONFLICT(singleton) DO UPDATE SET data_json=excluded.data_json",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    /// Returns only presence to public callers. The plaintext is read only by
    /// the provider adapter while creating an HTTP Authorization header.
    pub fn timestamp_secret_present(&self) -> Result<bool> {
        Ok(self.timestamp_secret()?.is_some())
    }

    pub(crate) fn timestamp_secret(&self) -> Result<Option<String>> {
        let path = contained_path(
            &self.root,
            Path::new(TIMESTAMP_SECRETS_RELATIVE_PATH),
            false,
        )?;
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(|error| AppError::io(&path, error))?;
        let secrets: TimestampSecrets = serde_json::from_slice(&bytes)?;
        Ok((!secrets.secret.trim().is_empty()).then_some(secrets.secret))
    }

    pub fn save_timestamp_secret(&self, secret: Option<&str>) -> Result<()> {
        let path = contained_path(
            &self.root,
            Path::new(TIMESTAMP_SECRETS_RELATIVE_PATH),
            false,
        )?;
        let Some(secret) = secret.map(str::trim).filter(|value| !value.is_empty()) else {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| AppError::io(&path, error))?;
            }
            return Ok(());
        };
        let contents = serde_json::to_vec(&TimestampSecrets {
            secret: secret.to_owned(),
        })?;
        atomic_write_secret(&path, &contents)?;
        Ok(())
    }

    /// Global non-secret settings for the optional ACRCloud operation. These
    /// live outside `Profile` because editable track snapshots inherit that
    /// profile and must never inherit provider configuration.
    pub fn audio_screening_settings(&self) -> Result<AudioScreeningSettings> {
        let json: Option<String> = self
            .open()?
            .query_row(
                "SELECT data_json FROM audio_screening_settings WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .unwrap_or_else(|| Ok(AudioScreeningSettings::default()))
    }

    pub fn save_audio_screening_settings(&self, settings: &AudioScreeningSettings) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO audio_screening_settings(singleton,data_json) VALUES(1,?1)
             ON CONFLICT(singleton) DO UPDATE SET data_json=excluded.data_json",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    /// Public callers can learn only whether a complete pair exists.
    pub fn audio_screening_credentials_present(&self) -> Result<bool> {
        Ok(self.audio_screening_credentials()?.is_some())
    }

    /// Credentials are intentionally crate-private and read only by the
    /// dedicated ACRCloud adapter. A missing half-pair is treated as absent.
    pub(crate) fn audio_screening_credentials(&self) -> Result<Option<(String, String)>> {
        let path = contained_path(
            &self.root,
            Path::new(AUDIO_SCREENING_SECRETS_RELATIVE_PATH),
            false,
        )?;
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(|error| AppError::io(&path, error))?;
        let secrets: AudioScreeningSecrets = serde_json::from_slice(&bytes)?;
        let access_key = secrets.access_key.trim();
        let access_secret = secrets.access_secret.trim();
        Ok((!access_key.is_empty() && !access_secret.is_empty())
            .then(|| (access_key.to_owned(), access_secret.to_owned())))
    }

    /// Applies only explicitly supplied fields. Submit empty strings for both
    /// values to clear the credential pair; partial pairs are never retained.
    pub fn save_audio_screening_secret(&self, input: AudioScreeningSecretInput) -> Result<()> {
        if input.access_key.is_none() && input.access_secret.is_none() {
            return Ok(());
        }
        let path = contained_path(
            &self.root,
            Path::new(AUDIO_SCREENING_SECRETS_RELATIVE_PATH),
            false,
        )?;
        let existing = if path.exists() {
            let bytes = std::fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            serde_json::from_slice::<AudioScreeningSecrets>(&bytes)?
        } else {
            AudioScreeningSecrets::default()
        };
        let access_key = input
            .access_key
            .as_deref()
            .map(str::trim)
            .map(ToOwned::to_owned)
            .unwrap_or(existing.access_key);
        let access_secret = input
            .access_secret
            .as_deref()
            .map(str::trim)
            .map(ToOwned::to_owned)
            .unwrap_or(existing.access_secret);
        if access_key.is_empty() || access_secret.is_empty() {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| AppError::io(&path, error))?;
            }
            return Ok(());
        }
        let contents = serde_json::to_vec(&AudioScreeningSecrets {
            access_key,
            access_secret,
        })?;
        atomic_write_secret(&path, &contents)
    }

    /// An attachment attempt is separate from the immutable timestamp record:
    /// errors such as an unavailable provider have no provider-response file,
    /// but must remain visible and retryable for the same finalized snapshot.
    pub fn timestamp_attachment_summary(
        &self,
        track_id: &str,
        certificate_id: &str,
    ) -> Result<Option<ExternalTimestampSummary>> {
        let json: Option<String> = self
            .open()?
            .query_row(
                "SELECT data_json FROM timestamp_attachment_status
                 WHERE track_id=?1 AND certificate_id=?2",
                params![track_id, certificate_id],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .transpose()
    }

    pub fn save_timestamp_attachment_summary(
        &self,
        track_id: &str,
        certificate_id: &str,
        summary: &ExternalTimestampSummary,
    ) -> Result<()> {
        self.open()?.execute(
            "INSERT INTO timestamp_attachment_status(track_id,certificate_id,updated_at,data_json)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(track_id,certificate_id) DO UPDATE SET
               updated_at=excluded.updated_at,data_json=excluded.data_json",
            params![
                track_id,
                certificate_id,
                summary.updated_at.as_deref().unwrap_or_default(),
                serde_json::to_string(summary)?
            ],
        )?;
        Ok(())
    }
}

/// Atomic secret writer which applies the restrictive Unix mode before any
/// secret bytes are written. The generic artifact writer intentionally cannot
/// make that promise because normal evidence files have different permissions.
fn atomic_write_secret(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Validation("Timestamp secret configuration has no parent.".into())
    })?;
    std::fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::Validation("Timestamp secret configuration name is invalid.".into())
        })?;
    let temporary = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true).mode(0o600);
            options
        };
        #[cfg(not(unix))]
        let options = {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            options
        };
        let mut file = options
            .open(&temporary)
            .map_err(|error| AppError::io(&temporary, error))?;
        // Keep this explicit even when OpenOptionsExt::mode is available: a
        // restrictive umask or an existing platform policy cannot loosen it.
        restrict_secret_permissions(&temporary)?;
        file.write_all(bytes)
            .map_err(|error| AppError::io(&temporary, error))?;
        file.sync_all()
            .map_err(|error| AppError::io(&temporary, error))?;
        std::fs::rename(&temporary, path).map_err(|error| AppError::io(path, error))?;
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
#[cfg(unix)]
fn restrict_secret_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| AppError::io(path, error))
}

#[cfg(not(unix))]
fn restrict_secret_permissions(_path: &Path) -> Result<()> {
    // Windows ACL configuration is environment-specific. The secret remains
    // isolated from all project artifacts; the OS account's normal ACL is the
    // security boundary on this platform.
    Ok(())
}
