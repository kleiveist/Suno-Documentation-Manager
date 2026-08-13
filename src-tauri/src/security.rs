use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
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
    let temp = parent.join(format!(".create-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| AppError::io(&temp, e))?;
        file.write_all(bytes).map_err(|e| AppError::io(&temp, e))?;
        file.sync_all().map_err(|e| AppError::io(&temp, e))?;
        fs::hard_link(&temp, path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                AppError::Collision(path.display().to_string())
            } else {
                AppError::io(path, error)
            }
        })?;
        fs::remove_file(&temp).map_err(|e| AppError::io(&temp, e))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
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
    let temp = parent.join(format!(".import-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        fs::copy(source, &temp).map_err(|e| AppError::io(&temp, e))?;
        let file = OpenOptions::new()
            .read(true)
            .open(&temp)
            .map_err(|e| AppError::io(&temp, e))?;
        file.sync_all().map_err(|e| AppError::io(&temp, e))?;
        // Publishing by hard-link is a single no-clobber filesystem operation. Unlike
        // rename, it cannot replace a destination created between validation and publish.
        fs::hard_link(&temp, destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                AppError::Collision(destination.display().to_string())
            } else {
                AppError::io(destination, error)
            }
        })?;
        fs::remove_file(&temp).map_err(|e| AppError::io(&temp, e))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| AppError::io(path, e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| AppError::io(path, e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
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
