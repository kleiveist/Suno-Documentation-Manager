use crate::error::{AppError, Result};
use crate::model::{IntegrityState, OperationProgress};
use crate::security::{atomic_write, contained_path, sha256_file_with_progress};
use chrono::Utc;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const HASH_FILE: &str = "03_DOCUMENTATION/SHA256SUMS.txt";
const PROGRESS_REPORT_BYTES: u64 = 8 * 1024 * 1024;

#[cfg(test)]
pub fn calculate(track_root: &Path) -> Result<IntegrityState> {
    calculate_with_progress(track_root, &mut |_| {})
}

pub fn calculate_with_progress(
    track_root: &Path,
    on_progress: &mut impl FnMut(OperationProgress),
) -> Result<IntegrityState> {
    on_progress(progress("discovering_files", 0, 0, 0, 0, None));
    let mut entries = hash_entries_with_progress(track_root, "hashing", on_progress)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    if entries.is_empty() {
        return Err(AppError::Validation(
            "There are no track files to hash.".into(),
        ));
    }
    let content = entries
        .iter()
        .map(|(path, hash)| format!("{hash}  {path}\n"))
        .collect::<String>();
    let target = contained_path(track_root, Path::new(HASH_FILE), false)?;
    on_progress(progress(
        "writing_hash_list",
        0,
        0,
        entries.len() as u32,
        entries.len() as u32,
        Some(HASH_FILE.to_owned()),
    ));
    atomic_write(&target, content.as_bytes())?;
    on_progress(progress(
        "preparing_verification",
        0,
        0,
        entries.len() as u32,
        entries.len() as u32,
        None,
    ));
    let mut state = verify_with_progress(track_root, on_progress)?;
    state.generated = true;
    state.generated_at = Some(Utc::now().to_rfc3339());
    Ok(state)
}

pub fn verify(track_root: &Path) -> Result<IntegrityState> {
    verify_with_progress(track_root, &mut |_| {})
}

pub fn verify_with_progress(
    track_root: &Path,
    on_progress: &mut impl FnMut(OperationProgress),
) -> Result<IntegrityState> {
    on_progress(progress(
        "reading_hash_list",
        0,
        0,
        0,
        0,
        Some(HASH_FILE.to_owned()),
    ));
    let manifest = contained_path(track_root, Path::new(HASH_FILE), true)?;
    let content = fs::read_to_string(&manifest).map_err(|e| AppError::io(&manifest, e))?;
    let current: BTreeMap<String, String> =
        hash_entries_with_progress(track_root, "verifying", on_progress)?
            .into_iter()
            .collect();
    on_progress(progress(
        "comparing_hashes",
        0,
        0,
        current.len() as u32,
        current.len() as u32,
        None,
    ));
    let mut mismatch_files = Vec::new();
    let listed = parse_hash_list(&content)?;
    for path in current.keys().chain(listed.keys()) {
        if current.get(path) != listed.get(path) && !mismatch_files.contains(path) {
            mismatch_files.push(path.clone());
        }
    }
    let file_count = current.len().max(listed.len()) as u32;
    let verified_count = current
        .iter()
        .filter(|(path, hash)| listed.get(*path) == Some(*hash))
        .count() as u32;
    Ok(IntegrityState {
        generated: true,
        verified: file_count > 0 && verified_count == file_count,
        file_count,
        verified_count,
        generated_at: None,
        verified_at: Some(Utc::now().to_rfc3339()),
        mismatch_files,
    })
}

pub fn listed_hash(track_root: &Path, relative: &Path) -> Result<Option<String>> {
    crate::security::validate_relative(relative)?;
    if excluded(relative) {
        return Ok(None);
    }
    let portable = relative
        .to_str()
        .ok_or_else(|| AppError::Validation("Non-UTF-8 track paths cannot be hashed.".into()))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    if portable.contains('\\') || portable.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Track path contains characters unsupported by SHA256SUMS.".into(),
        ));
    }
    let manifest = contained_path(track_root, Path::new(HASH_FILE), true)?;
    let content = fs::read_to_string(&manifest).map_err(|error| AppError::io(&manifest, error))?;
    Ok(parse_hash_list(&content)?.get(&portable).cloned())
}

fn parse_hash_list(content: &str) -> Result<BTreeMap<String, String>> {
    let mut listed = BTreeMap::new();
    let mut seen = HashSet::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            return Err(AppError::Data(format!(
                "Empty SHA256SUMS line {}.",
                line_number + 1
            )));
        }
        let (expected, path) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!("Invalid SHA256SUMS line {}.", line_number + 1))
        })?;
        if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::Data(format!(
                "Invalid SHA-256 digest on line {}.",
                line_number + 1
            )));
        }
        let relative = Path::new(path);
        crate::security::validate_relative(relative)?;
        if path.contains('\\') || path.chars().any(char::is_control) || excluded(relative) {
            return Err(AppError::Data(format!(
                "Invalid or excluded SHA256SUMS path on line {}.",
                line_number + 1
            )));
        }
        if !seen.insert(path.to_owned()) {
            return Err(AppError::Data(format!("Duplicate SHA256SUMS path: {path}")));
        }
        listed.insert(path.to_owned(), expected.to_ascii_lowercase());
    }
    Ok(listed)
}

#[cfg(test)]
pub fn hash_entries(track_root: &Path) -> Result<Vec<(String, String)>> {
    hash_entries_with_progress(track_root, "hashing", &mut |_| {})
}

struct HashCandidate {
    path: PathBuf,
    portable: String,
    size: u64,
}

fn hash_entries_with_progress(
    track_root: &Path,
    stage: &str,
    on_progress: &mut impl FnMut(OperationProgress),
) -> Result<Vec<(String, String)>> {
    let candidates = hash_candidates(track_root)?;
    let total_bytes = candidates.iter().fold(0_u64, |total, candidate| {
        total.saturating_add(candidate.size)
    });
    let total_files = candidates.len() as u32;
    let mut result = Vec::new();
    let mut completed_bytes = 0_u64;
    on_progress(progress(stage, 0, total_bytes, 0, total_files, None));
    for (index, candidate) in candidates.into_iter().enumerate() {
        let mut last_reported = 0_u64;
        let current_file = candidate.portable.clone();
        let hash = sha256_file_with_progress(&candidate.path, |file_bytes| {
            if file_bytes.saturating_sub(last_reported) >= PROGRESS_REPORT_BYTES
                || file_bytes == candidate.size
            {
                last_reported = file_bytes;
                on_progress(progress(
                    stage,
                    completed_bytes.saturating_add(file_bytes),
                    total_bytes,
                    index as u32,
                    total_files,
                    Some(current_file.clone()),
                ));
            }
        })?;
        completed_bytes = completed_bytes.saturating_add(candidate.size);
        result.push((candidate.portable, hash));
        on_progress(progress(
            stage,
            completed_bytes,
            total_bytes,
            index as u32 + 1,
            total_files,
            None,
        ));
    }
    Ok(result)
}

fn hash_candidates(track_root: &Path) -> Result<Vec<HashCandidate>> {
    let mut result = Vec::new();
    for entry in WalkDir::new(track_root).follow_links(false).into_iter() {
        let entry = entry.map_err(|error| {
            AppError::io(
                error.path().unwrap_or(track_root),
                std::io::Error::other(error.to_string()),
            )
        })?;
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            return Err(AppError::Symlink(entry.path().display().to_string()));
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(track_root)
            .map_err(|_| AppError::PathEscape)?;
        if excluded(relative) {
            continue;
        }
        let portable = relative
            .to_str()
            .ok_or_else(|| AppError::Validation("Non-UTF-8 track paths cannot be hashed.".into()))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        if portable.contains('\\') || portable.chars().any(char::is_control) {
            return Err(AppError::Validation(
                "Track path contains characters unsupported by SHA256SUMS.".into(),
            ));
        }
        let size = entry
            .metadata()
            .map_err(|error| AppError::io(entry.path(), std::io::Error::other(error.to_string())))?
            .len();
        result.push(HashCandidate {
            path: entry.path().to_owned(),
            portable,
            size,
        });
    }
    Ok(result)
}

fn progress(
    stage: &str,
    processed_bytes: u64,
    total_bytes: u64,
    processed_files: u32,
    total_files: u32,
    current_file: Option<String>,
) -> OperationProgress {
    OperationProgress {
        stage: stage.to_owned(),
        processed_bytes,
        total_bytes,
        processed_files,
        total_files,
        current_file,
    }
}

fn excluded(relative: &Path) -> bool {
    if relative == Path::new(HASH_FILE)
        || relative == Path::new(crate::certificate::PDF_FILE)
        || relative == Path::new(crate::certificate::PDF_FILE_DE)
    {
        return true;
    }
    matches!(
        relative.components().next(),
        Some(std::path::Component::Normal(value))
            if value == ".archive" || value == ".summary" || value == "06_CERTIFICATE"
    )
}

#[cfg(test)]
pub fn invalidate_on_mismatch(track_root: &Path) -> Result<Option<Vec<String>>> {
    let manifest = track_root.join(HASH_FILE);
    if !manifest.exists() {
        return Ok(None);
    }
    let state = verify(track_root)?;
    Ok((!state.verified).then_some(state.mismatch_files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir_all(track_root.join("01_RELEASE")).expect("release directory");
        fs::create_dir_all(track_root.join("03_DOCUMENTATION")).expect("documentation directory");
        fs::write(track_root.join("01_RELEASE/final.wav"), b"release audio")
            .expect("release fixture");
        (directory, track_root)
    }

    fn write_manifest(track_root: &Path, content: &str) {
        fs::write(track_root.join(HASH_FILE), content).expect("SHA256SUMS fixture");
    }

    #[test]
    fn hash_generation_verifies_exact_set_and_detects_added_file() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir_all(track_root.join("01_RELEASE")).expect("release directory");
        fs::create_dir_all(track_root.join("03_DOCUMENTATION")).expect("documentation directory");
        fs::create_dir_all(track_root.join(".archive")).expect("archive directory");
        fs::create_dir_all(track_root.join("06_CERTIFICATE")).expect("certificate directory");
        fs::write(track_root.join("01_RELEASE/final.wav"), b"release audio")
            .expect("release fixture");
        fs::write(
            track_root.join("03_DOCUMENTATION/README.md"),
            b"documentation",
        )
        .expect("document fixture");
        fs::write(track_root.join(".archive/ignored.txt"), b"ignored").expect("archive fixture");
        fs::write(track_root.join("06_CERTIFICATE/ignored.txt"), b"ignored")
            .expect("certificate fixture");
        fs::write(
            track_root.join(crate::certificate::PDF_FILE),
            b"root certificate PDF",
        )
        .expect("root PDF fixture");
        fs::write(
            track_root
                .join("01_RELEASE")
                .join(crate::certificate::PDF_FILE),
            b"same name in a hashable directory",
        )
        .expect("nested same-name fixture");

        let generated = calculate(&track_root).expect("hash generation");
        assert!(generated.verified);
        assert_eq!(generated.file_count, 3);
        let verified = verify(&track_root).expect("hash verification");
        assert!(verified.verified);
        assert_eq!(verified.verified_count, 3);
        let entries = hash_entries(&track_root).expect("hash candidates");
        assert!(!entries
            .iter()
            .any(|(path, _)| path == crate::certificate::PDF_FILE));
        assert!(entries
            .iter()
            .any(|(path, _)| { path == &format!("01_RELEASE/{}", crate::certificate::PDF_FILE) }));

        fs::write(
            track_root.join("01_RELEASE/unlisted.mp3"),
            b"new relevant file",
        )
        .expect("additional relevant file");
        let changed = verify(&track_root).expect("changed-set verification");
        assert!(!changed.verified);
        assert!(changed
            .mismatch_files
            .iter()
            .any(|path| path == "01_RELEASE/unlisted.mp3"));
        assert!(invalidate_on_mismatch(&track_root)
            .expect("mismatch status")
            .is_some());
    }

    #[test]
    fn hashing_progress_reports_real_bytes_files_and_verification_stages() {
        let (_directory, track_root) = fixture();
        let large = vec![0x5a; (PROGRESS_REPORT_BYTES + 1024) as usize];
        fs::write(track_root.join("01_RELEASE/large.wav"), large).expect("large hash fixture");
        let mut events = Vec::new();

        let state = calculate_with_progress(&track_root, &mut |progress| events.push(progress))
            .expect("hash calculation with progress");

        assert!(state.verified);
        assert!(events
            .first()
            .is_some_and(|event| event.stage == "discovering_files"));
        assert!(events.iter().any(|event| {
            event.stage == "hashing"
                && event.processed_bytes > 0
                && event.processed_bytes < event.total_bytes
                && event.current_file.as_deref() == Some("01_RELEASE/large.wav")
        }));
        assert!(events
            .iter()
            .any(|event| event.stage == "writing_hash_list"));
        assert!(events.iter().any(|event| {
            event.stage == "verifying"
                && event.total_bytes > 0
                && event.total_files == state.file_count
        }));
        assert!(events.iter().any(|event| event.stage == "comparing_hashes"));
    }

    #[test]
    fn nested_suno_doc_directory_is_track_content_and_detects_changes() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        let nested_admin_named_directory = track_root.join("01_RELEASE/.suno-doc");
        fs::create_dir_all(&nested_admin_named_directory).expect("nested track directory");
        fs::create_dir_all(track_root.join("03_DOCUMENTATION")).expect("documentation directory");
        let provenance = nested_admin_named_directory.join("provenance.json");
        fs::write(&provenance, b"{\"source\":\"release\"}\n").expect("nested provenance fixture");

        let generated = calculate(&track_root).expect("hash nested track content");
        assert!(generated.verified);
        assert_eq!(generated.file_count, 1);
        let manifest = fs::read_to_string(track_root.join(HASH_FILE)).expect("SHA256SUMS contents");
        assert!(manifest.contains("01_RELEASE/.suno-doc/provenance.json"));

        fs::write(&provenance, b"{\"source\":\"externally changed\"}\n")
            .expect("external nested-file change");
        let changed = verify(&track_root).expect("verify changed nested track content");
        assert!(!changed.verified);
        assert!(changed
            .mismatch_files
            .iter()
            .any(|path| path == "01_RELEASE/.suno-doc/provenance.json"));
    }

    #[test]
    fn hash_verification_detects_changed_deleted_and_added_files() {
        let (_directory, track_root) = fixture();
        calculate(&track_root).expect("initial hashes");

        fs::write(track_root.join("01_RELEASE/final.wav"), b"changed release")
            .expect("change listed file");
        let changed = verify(&track_root).expect("changed-file result");
        assert!(!changed.verified);
        assert_eq!(changed.mismatch_files, vec!["01_RELEASE/final.wav"]);

        calculate(&track_root).expect("accept changed revision");
        fs::remove_file(track_root.join("01_RELEASE/final.wav")).expect("delete listed file");
        let deleted = verify(&track_root).expect("deleted-file result");
        assert!(!deleted.verified);
        assert_eq!(deleted.mismatch_files, vec!["01_RELEASE/final.wav"]);

        fs::write(
            track_root.join("01_RELEASE/replacement.wav"),
            b"replacement release",
        )
        .expect("add unlisted file");
        let replaced = verify(&track_root).expect("replacement-set result");
        assert!(!replaced.verified);
        assert!(replaced
            .mismatch_files
            .iter()
            .any(|path| path == "01_RELEASE/final.wav"));
        assert!(replaced
            .mismatch_files
            .iter()
            .any(|path| path == "01_RELEASE/replacement.wav"));
    }

    #[test]
    fn hash_verifier_rejects_malformed_and_unsafe_manifest_entries() {
        let (_directory, track_root) = fixture();
        let invalid_manifests = [
            format!("{DIGEST}  01_RELEASE/final.wav\n{DIGEST}  01_RELEASE/final.wav\n"),
            "short  01_RELEASE/final.wav\n".into(),
            format!("{DIGEST}  /absolute.wav\n"),
            format!("{DIGEST}  ../escape.wav\n"),
            format!("{DIGEST}  .archive/hidden.wav\n"),
            format!("{DIGEST}  .summary/hidden.wav\n"),
            format!("{DIGEST}  06_CERTIFICATE/hidden.wav\n"),
            format!("{DIGEST}  {HASH_FILE}\n"),
            format!("{DIGEST}  01_RELEASE/control\tname.wav\n"),
            format!("{DIGEST}  01_RELEASE/final.wav\n\n{DIGEST}  01_RELEASE/other.wav\n"),
        ];

        for manifest in invalid_manifests {
            write_manifest(&track_root, &manifest);
            assert!(
                verify(&track_root).is_err(),
                "unsafe manifest was accepted: {manifest:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn hash_generation_rejects_file_and_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let (directory, track_root) = fixture();
        let outside_file = directory.path().join("outside.wav");
        fs::write(&outside_file, b"outside").expect("outside file");
        symlink(&outside_file, track_root.join("01_RELEASE/link.wav")).expect("file symlink");
        assert!(matches!(
            hash_entries(&track_root),
            Err(AppError::Symlink(_))
        ));

        fs::remove_file(track_root.join("01_RELEASE/link.wav")).expect("remove file symlink");
        let outside_directory = directory.path().join("outside-directory");
        fs::create_dir(&outside_directory).expect("outside directory");
        symlink(
            &outside_directory,
            track_root.join("01_RELEASE/linked-directory"),
        )
        .expect("directory symlink");
        assert!(matches!(
            hash_entries(&track_root),
            Err(AppError::Symlink(_))
        ));
    }
}
