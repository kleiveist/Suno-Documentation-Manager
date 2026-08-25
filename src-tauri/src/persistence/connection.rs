use super::{migrate, Persistence, DATABASE_RELATIVE_PATH};
use crate::error::{AppError, Result};
use crate::model::Profile;
use crate::security::{atomic_write, atomic_write_new, contained_path, ensure_contained_directory};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

impl Persistence {
    pub fn initialize(root: &Path) -> Result<Self> {
        ensure_contained_directory(root, Path::new(".suno-doc"))?;
        ensure_contained_directory(root, Path::new(".suno-doc/config"))?;
        ensure_contained_directory(root, Path::new(".suno-doc/global-evidence"))?;
        ensure_secret_gitignore(root)?;
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
}

fn ensure_secret_gitignore(root: &Path) -> Result<()> {
    let path = contained_path(root, Path::new(".suno-doc/.gitignore"), false)?;
    const TIMESTAMP_RULE: &str = "/config/timestamp-secrets.json";
    const AUDIO_SCREENING_RULE: &str = "/config/audio-screening-secrets.json";
    if !path.exists() {
        return atomic_write_new(
            &path,
            b"# Local provider credentials; never export these files.\n/config/timestamp-secrets.json\n/config/audio-screening-secrets.json\n",
        );
    }
    let content = std::fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    let has_timestamp = content.lines().any(|line| line.trim() == TIMESTAMP_RULE);
    let has_audio_screening = content
        .lines()
        .any(|line| line.trim() == AUDIO_SCREENING_RULE);
    if has_timestamp && has_audio_screening {
        return Ok(());
    }
    let mut updated = content;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str("# Local provider credentials; never export these files.\n");
    if !has_timestamp {
        updated.push_str(TIMESTAMP_RULE);
        updated.push('\n');
    }
    if !has_audio_screening {
        updated.push_str(AUDIO_SCREENING_RULE);
        updated.push('\n');
    }
    atomic_write(&path, updated.as_bytes())
}
