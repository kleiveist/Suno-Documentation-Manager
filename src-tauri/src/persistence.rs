use std::path::PathBuf;

mod connection;
mod evidence;
mod migration;
mod timestamps;
mod tracks;

pub use self::migration::migrate;

pub const DATABASE_RELATIVE_PATH: &str = ".suno-doc/workspace.sqlite";
pub const SCHEMA_VERSION: i64 = 7;

#[derive(Debug, Clone)]
pub struct Persistence {
    root: PathBuf,
}

#[cfg(test)]
mod tests;
