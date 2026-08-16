use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tempfile::{Builder, NamedTempFile, PathPersistError};
use uuid::Uuid;

pub fn canonical_workspace(path: &Path, create: bool) -> Result<PathBuf> {
    if create {
        fs::create_dir_all(path).map_err(|e| AppError::io(path, e))?;
    }
    let canonical = fs::canonicalize(path).map_err(|e| AppError::io(path, e))?;
    if !canonical.is_dir() {
        return Err(AppError::InvalidWorkspace(path.display().to_string()));
    }
    if fs::symlink_metadata(path)
        .map_err(|e| AppError::io(path, e))?
        .file_type()
        .is_symlink()
    {
        return Err(AppError::Symlink(path.display().to_string()));
    }
    Ok(canonical)
}

pub fn validate_relative(relative: &Path) -> Result<()> {
    if relative.as_os_str().is_empty() || relative.is_absolute() {
        return Err(AppError::PathEscape);
    }
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(AppError::PathEscape);
        }
    }
    Ok(())
}

/// Resolve a managed path without following a symlink outside the canonical root.
pub fn contained_path(root: &Path, relative: &Path, require_exists: bool) -> Result<PathBuf> {
    validate_relative(relative)?;
    let root_metadata = fs::symlink_metadata(root).map_err(|e| AppError::io(root, e))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(AppError::Symlink(root.display().to_string()));
    }
    let supplied_root = root.to_owned();
    let root = fs::canonicalize(root).map_err(|e| AppError::io(root, e))?;
    if supplied_root != root {
        return Err(AppError::PathEscape);
    }
    let candidate = root.join(relative);

    let mut current = root.clone();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(AppError::PathEscape);
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                // `Path::exists` follows links and reports false for a dangling
                // symlink. Inspect the directory entry itself so even a link to a
                // not-yet-created outside target is rejected before a writer opens it.
                if metadata.file_type().is_symlink() {
                    return Err(AppError::Symlink(current.display().to_string()));
                }
                let canonical =
                    fs::canonicalize(&current).map_err(|e| AppError::io(&current, e))?;
                if !canonical.starts_with(&root) {
                    return Err(AppError::PathEscape);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::io(&current, error)),
        }
    }

    if require_exists && !candidate.exists() {
        return Err(AppError::io(
            &candidate,
            std::io::Error::new(std::io::ErrorKind::NotFound, "managed path does not exist"),
        ));
    }
    Ok(candidate)
}

pub fn ensure_contained_directory(root: &Path, relative: &Path) -> Result<PathBuf> {
    let path = contained_path(root, relative, false)?;
    fs::create_dir_all(&path).map_err(|e| AppError::io(&path, e))?;
    // Re-resolve after creation to catch races or unexpected links.
    contained_path(root, relative, true)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Validation("A managed file needs a parent directory.".into()))?;
    fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Validation("Invalid managed file name.".into()))?;
    let temporary = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4()));
    let write_result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| AppError::io(&temporary, e))?;
        file.write_all(bytes)
            .map_err(|e| AppError::io(&temporary, e))?;
        file.sync_all().map_err(|e| AppError::io(&temporary, e))?;
        fs::rename(&temporary, path).map_err(|e| AppError::io(path, e))?;
        if let Ok(parent_file) = fs::File::open(parent) {
            let _ = parent_file.sync_all();
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

pub fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        return Err(AppError::Collision(path.display().to_string()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Validation("A managed file needs a parent directory.".into()))?;
    fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    let mut temporary = temporary_file(parent, ".create-")?;
    temporary
        .write_all(bytes)
        .map_err(|e| AppError::io(temporary.path(), e))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| AppError::io(temporary.path(), e))?;
    publish_new(temporary, path)
}

pub fn copy_new(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Err(AppError::Collision(destination.display().to_string()));
    }
    let metadata = fs::symlink_metadata(source).map_err(|e| AppError::io(source, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Symlink(source.display().to_string()));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::Validation("Evidence destination has no parent.".into()))?;
    fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    let temporary = temporary_file(parent, ".import-")?;
    fs::copy(source, temporary.path()).map_err(|e| AppError::io(temporary.path(), e))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| AppError::io(temporary.path(), e))?;
    publish_new(temporary, destination)
}

/// Copies a file once while calculating the digest from the same byte stream.
/// This avoids reading large evidence a second time from removable storage.
pub fn copy_new_hashed(source: &Path, destination: &Path) -> Result<(String, u64)> {
    if destination.exists() {
        return Err(AppError::Collision(destination.display().to_string()));
    }
    let metadata = fs::symlink_metadata(source).map_err(|e| AppError::io(source, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Symlink(source.display().to_string()));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::Validation("Evidence destination has no parent.".into()))?;
    fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    let mut input = fs::File::open(source).map_err(|e| AppError::io(source, e))?;
    let mut temporary = temporary_file(parent, ".import-")?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|e| AppError::io(source, e))?;
        if count == 0 {
            break;
        }
        temporary
            .write_all(&buffer[..count])
            .map_err(|e| AppError::io(temporary.path(), e))?;
        hasher.update(&buffer[..count]);
        size += count as u64;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| AppError::io(temporary.path(), e))?;
    publish_new(temporary, destination)?;
    Ok((format!("{:x}", hasher.finalize()), size))
}

fn temporary_file(parent: &Path, prefix: &str) -> Result<NamedTempFile> {
    Builder::new()
        .prefix(prefix)
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|error| AppError::io(parent, error))
}

fn publish_new(temporary: NamedTempFile, destination: &Path) -> Result<()> {
    match temporary.into_temp_path().persist_noclobber(destination) {
        Ok(()) => {
            if let Some(parent) = destination.parent() {
                if let Ok(parent_file) = fs::File::open(parent) {
                    let _ = parent_file.sync_all();
                }
            }
            Ok(())
        }
        Err(PathPersistError { error, path }) => {
            drop(path);
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Err(AppError::Collision(destination.display().to_string()))
            } else {
                Err(AppError::io(destination, error))
            }
        }
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    sha256_file_with_progress(path, |_| {})
}

pub fn sha256_file_with_progress(path: &Path, mut on_progress: impl FnMut(u64)) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| AppError::io(path, e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut processed = 0_u64;
    loop {
        let count = file.read(&mut buffer).map_err(|e| AppError::io(path, e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        processed = processed.saturating_add(count as u64);
        on_progress(processed);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn slugify(title: &str) -> Result<String> {
    let mut result = String::new();
    let mut dash = false;
    for c in title.trim().chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            dash = false;
        } else if !dash && !result.is_empty() {
            result.push('-');
            dash = true;
        }
    }
    while result.ends_with('-') {
        result.pop();
    }
    if result.is_empty() || result == "." || result == ".." {
        return Err(AppError::Validation(
            "Track title does not form a safe folder name.".into(),
        ));
    }
    Ok(result)
}

/// Build a portable, human-readable file stem from a validated track title.
/// Path separators, Windows-reserved punctuation, control characters, trailing
/// dots/spaces, and device names are never emitted.
pub fn safe_file_stem(title: &str) -> Result<String> {
    let mut result = String::new();
    let mut replacement = false;
    for character in title.trim().chars() {
        let unsafe_character = character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            );
        if unsafe_character {
            if !replacement && !result.is_empty() {
                while result.ends_with(' ') {
                    result.pop();
                }
                result.push('-');
                replacement = true;
            }
        } else if replacement && character == ' ' {
            continue;
        } else {
            result.push(character);
            replacement = false;
        }
    }
    let trimmed = result.trim_matches(|character| matches!(character, ' ' | '.' | '-'));
    if trimmed.is_empty() || matches!(trimmed, "." | "..") {
        return Err(AppError::Validation(
            "Track title does not form a safe file name.".into(),
        ));
    }
    let upper = trimmed.to_ascii_uppercase();
    let reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    Ok(if reserved {
        format!("_{trimmed}")
    } else {
        trimmed.to_owned()
    })
}

pub fn portable_relative(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn no_temporary_files(directory: &Path) -> bool {
        fs::read_dir(directory)
            .expect("directory entries")
            .all(|entry| {
                !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            })
    }

    #[test]
    fn safe_path_rejects_traversal_and_absolute_paths() {
        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("workspace");
        fs::create_dir(&root).expect("workspace directory");

        assert!(matches!(
            contained_path(&root, Path::new("../outside"), false),
            Err(AppError::PathEscape)
        ));
        assert!(matches!(
            contained_path(&root, directory.path(), false),
            Err(AppError::PathEscape)
        ));
        assert!(matches!(
            contained_path(&root, Path::new("nested/../../outside"), false),
            Err(AppError::PathEscape)
        ));

        for invalid in ["", ".", "..", "../outside", "nested/../../../outside"] {
            assert!(matches!(
                validate_relative(Path::new(invalid)),
                Err(AppError::PathEscape)
            ));
        }
        assert!(validate_relative(Path::new("nested/contained/file.txt")).is_ok());

        let outside_sentinel = directory.path().join("outside-sentinel.txt");
        fs::write(&outside_sentinel, b"outside sentinel").expect("outside sentinel");
        for invalid in [
            "../outside-sentinel.txt",
            "nested/../../outside-sentinel.txt",
        ] {
            assert!(contained_path(&root, Path::new(invalid), false).is_err());
        }
        assert_eq!(
            fs::read(&outside_sentinel).expect("unchanged outside sentinel"),
            b"outside sentinel"
        );
    }

    #[cfg(unix)]
    #[test]
    fn safe_path_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("workspace");
        let outside = directory.path().join("outside");
        fs::create_dir(&root).expect("workspace directory");
        fs::create_dir(&outside).expect("outside directory");
        symlink(&outside, root.join("escape")).expect("escape symlink");

        assert!(matches!(
            contained_path(&root, Path::new("escape/file.txt"), false),
            Err(AppError::Symlink(_))
        ));

        let missing_outside = directory.path().join("not-created-yet.sqlite");
        symlink(&missing_outside, root.join("dangling.sqlite")).expect("dangling symlink");
        assert!(matches!(
            contained_path(&root, Path::new("dangling.sqlite"), false),
            Err(AppError::Symlink(_))
        ));
        assert!(!missing_outside.exists());

        let original_root = directory.path().join("original-root");
        let replacement = directory.path().join("replacement-outside");
        fs::create_dir(&original_root).expect("original root");
        fs::create_dir(&replacement).expect("replacement outside");
        fs::remove_dir(&original_root).expect("remove original root");
        symlink(&replacement, &original_root).expect("replaced root symlink");
        assert!(matches!(
            contained_path(&original_root, Path::new("file.txt"), false),
            Err(AppError::Symlink(_))
        ));

        let nested = root.join("nested");
        fs::create_dir(&nested).expect("nested directory");
        symlink(&outside, nested.join("escape")).expect("nested escape symlink");
        assert!(matches!(
            contained_path(&root, Path::new("nested/escape/file.txt"), false),
            Err(AppError::Symlink(_))
        ));

        let contained_target = root.join("contained-target");
        fs::create_dir(&contained_target).expect("contained target");
        symlink(&contained_target, root.join("contained-link")).expect("contained symlink");
        assert!(matches!(
            contained_path(&root, Path::new("contained-link/file.txt"), false),
            Err(AppError::Symlink(_))
        ));

        let outside_sentinel = outside.join("sentinel.txt");
        fs::write(&outside_sentinel, b"outside sentinel").expect("outside sentinel");
        assert_eq!(
            fs::read(&outside_sentinel).expect("unchanged outside sentinel"),
            b"outside sentinel"
        );
    }

    #[test]
    fn atomic_writes_publish_complete_bytes_and_never_clobber_new_files() {
        let directory = tempdir().expect("temporary directory");
        let target = directory.path().join("document.md");

        atomic_write(&target, b"first complete version").expect("initial atomic write");
        atomic_write(&target, b"second complete version").expect("replacement atomic write");
        assert_eq!(
            fs::read(&target).expect("read result"),
            b"second complete version"
        );
        assert!(fs::read_dir(directory.path())
            .expect("directory entries")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));

        let error = atomic_write_new(&target, b"must not replace").expect_err("collision");
        assert!(matches!(error, AppError::Collision(_)));
        assert_eq!(
            fs::read(&target).expect("read preserved result"),
            b"second complete version"
        );

        let create_only = directory.path().join("created-once.md");
        atomic_write_new(&create_only, b"complete create-only file")
            .expect("create-only atomic write");
        assert_eq!(
            fs::read(&create_only).expect("read create-only result"),
            b"complete create-only file"
        );
        assert!(no_temporary_files(directory.path()));
    }

    #[test]
    fn no_clobber_publish_preserves_a_destination_created_after_staging() {
        let directory = tempdir().expect("temporary directory");
        let destination = directory.path().join("concurrent-destination.bin");
        let mut temporary =
            temporary_file(directory.path(), ".publish-test-").expect("temporary file");
        temporary
            .write_all(b"staged bytes")
            .expect("staged contents");
        temporary.as_file().sync_all().expect("sync staged file");

        fs::write(&destination, b"concurrent bytes").expect("concurrent destination");
        let error = publish_new(temporary, &destination).expect_err("publish collision");

        assert!(matches!(error, AppError::Collision(_)));
        assert_eq!(
            fs::read(&destination).expect("preserved concurrent destination"),
            b"concurrent bytes"
        );
        assert!(no_temporary_files(directory.path()));
    }

    #[test]
    fn copy_new_publishes_complete_bytes_and_preserves_the_source() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("source.bin");
        let destination = directory.path().join("destination.bin");
        fs::write(&source, b"source bytes").expect("source fixture");

        copy_new(&source, &destination).expect("copy into new destination");

        assert_eq!(
            fs::read(&source).expect("preserved source"),
            b"source bytes"
        );
        assert_eq!(
            fs::read(&destination).expect("published destination"),
            b"source bytes"
        );
        assert!(no_temporary_files(directory.path()));
    }

    #[test]
    fn copy_new_hashed_returns_the_digest_from_the_copy_stream() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("large.zip");
        let destination = directory.path().join("managed/large.zip");
        let bytes = (0..(3 * 1024 * 1024 + 17))
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        fs::write(&source, &bytes).expect("source fixture");

        let (digest, size) = copy_new_hashed(&source, &destination).expect("copy and hash");

        assert_eq!(size, bytes.len() as u64);
        assert_eq!(digest, sha256_bytes(&bytes));
        assert_eq!(fs::read(destination).expect("managed bytes"), bytes);
        assert!(source.is_file());
    }

    #[test]
    #[ignore = "requires SUNO_DOC_REMOVABLE_FS_TEST_ROOT to name a disposable writable filesystem root"]
    fn no_clobber_publish_works_on_configured_removable_filesystem() {
        let configured_root = std::env::var_os("SUNO_DOC_REMOVABLE_FS_TEST_ROOT")
            .map(PathBuf::from)
            .expect("SUNO_DOC_REMOVABLE_FS_TEST_ROOT must be set explicitly");
        assert!(
            configured_root.is_dir(),
            "configured test root must be a directory"
        );
        let fixture = Builder::new()
            .prefix(".suno-doc-fs-compat-")
            .tempdir_in(&configured_root)
            .expect("create isolated removable-filesystem fixture");

        let created = fixture.path().join("created-once.bin");
        atomic_write_new(&created, b"complete create-only bytes")
            .expect("create-only publish on removable filesystem");
        assert_eq!(
            fs::read(&created).expect("read create-only destination"),
            b"complete create-only bytes"
        );

        let source = fixture.path().join("source.bin");
        let destination = fixture.path().join("destination.bin");
        fs::write(&source, b"first source bytes").expect("source fixture");
        copy_new(&source, &destination).expect("copy publish on removable filesystem");
        assert_eq!(
            sha256_file(&source).expect("source digest"),
            sha256_file(&destination).expect("destination digest")
        );
        fs::write(&source, b"changed source bytes").expect("changed source fixture");
        assert!(matches!(
            copy_new(&source, &destination),
            Err(AppError::Collision(_))
        ));
        assert_eq!(
            fs::read(&source).expect("preserved changed source"),
            b"changed source bytes"
        );
        assert_eq!(
            fs::read(&destination).expect("preserved first destination"),
            b"first source bytes"
        );
        assert!(no_temporary_files(fixture.path()));
    }

    #[test]
    fn atomic_and_copy_failures_preserve_existing_state_and_clean_temporaries() {
        let directory = tempdir().expect("temporary directory");
        let occupied_directory = directory.path().join("occupied");
        fs::create_dir(&occupied_directory).expect("occupied destination directory");
        let sentinel = occupied_directory.join("sentinel.txt");
        fs::write(&sentinel, b"preserve me").expect("destination sentinel");

        assert!(atomic_write(&occupied_directory, b"must fail").is_err());
        assert_eq!(
            fs::read(&sentinel).expect("preserved destination sentinel"),
            b"preserve me"
        );
        assert!(no_temporary_files(directory.path()));

        let existing = directory.path().join("existing.bin");
        fs::write(&existing, b"existing bytes").expect("existing destination");
        let source = directory.path().join("source.bin");
        fs::write(&source, b"source bytes").expect("copy source");
        assert!(matches!(
            copy_new(&source, &existing),
            Err(AppError::Collision(_))
        ));
        assert_eq!(
            fs::read(&source).expect("source preserved"),
            b"source bytes"
        );
        assert_eq!(
            fs::read(&existing).expect("destination preserved"),
            b"existing bytes"
        );
        assert!(no_temporary_files(directory.path()));

        let missing = directory.path().join("missing.bin");
        let destination = directory.path().join("new.bin");
        assert!(matches!(
            copy_new(&missing, &destination),
            Err(AppError::Io { .. })
        ));
        assert!(!destination.exists());
        assert!(no_temporary_files(directory.path()));
    }

    #[cfg(unix)]
    #[test]
    fn copy_and_atomic_creation_reject_symlink_endpoints() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("temporary directory");
        let real_source = directory.path().join("real-source.bin");
        let source_link = directory.path().join("source-link.bin");
        fs::write(&real_source, b"source bytes").expect("real source");
        symlink(&real_source, &source_link).expect("source symlink");
        let destination = directory.path().join("destination.bin");
        assert!(matches!(
            copy_new(&source_link, &destination),
            Err(AppError::Symlink(_))
        ));
        assert!(!destination.exists());

        let dangling_target = directory.path().join("outside-target.bin");
        let destination_link = directory.path().join("destination-link.bin");
        symlink(&dangling_target, &destination_link).expect("destination symlink");
        assert!(matches!(
            atomic_write_new(&destination_link, b"must not escape"),
            Err(AppError::Collision(_))
        ));
        assert!(!dangling_target.exists());
        assert!(no_temporary_files(directory.path()));
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_rejects_non_utf8_managed_file_name_without_side_effects() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let directory = tempdir().expect("temporary directory");
        let invalid_name = OsString::from_vec(vec![b'f', b'i', 0xff]);
        let target = directory.path().join(invalid_name);
        assert!(matches!(
            atomic_write(&target, b"must not be written"),
            Err(AppError::Validation(_))
        ));
        assert!(!target.exists());
        assert!(no_temporary_files(directory.path()));
    }
}
