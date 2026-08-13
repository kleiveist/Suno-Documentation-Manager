use serde::Serialize;
use std::path::Path;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No workspace is open.")]
    NoWorkspace,
    #[error("The selected path is not a valid workspace: {0}")]
    InvalidWorkspace(String),
    #[error("The requested path is outside the workspace.")]
    PathEscape,
    #[error("Symbolic links are not accepted for managed paths: {0}")]
    Symlink(String),
    #[error("A file already exists at the managed destination: {0}")]
    Collision(String),
    #[error("The file type is not allowed for evidence role {role}: {extension}")]
    FileType { role: String, extension: String },
    #[error("Track not found: {0}")]
    TrackNotFound(String),
    #[error("Evidence not found: {0}")]
    EvidenceNotFound(String),
    #[error("The track is finalized. Create a revision before changing it.")]
    Finalized,
    #[error("Existing unmanaged documents require explicit adoption and archival: {0}")]
    AdoptionRequired(String),
    #[error("The operation is blocked: {0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("File operation failed for {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid stored data: {0}")]
    Data(String),
    #[error("Image processing failed: {0}")]
    Image(String),
}

impl AppError {
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Data(value.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
