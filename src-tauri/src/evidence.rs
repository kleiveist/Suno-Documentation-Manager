use crate::error::{AppError, Result};
use crate::model::{EvidenceItem, EvidenceProvenance, EvidenceRole, GlobalEvidenceItem};
use crate::security::{
    contained_path, copy_new, copy_new_hashed, ensure_contained_directory, portable_relative,
    sha256_file,
};
use chrono::Utc;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const AUTOMATIC_HASH_LIMIT_BYTES: u64 = 64 * 1024 * 1024;

pub fn validate_type(role: &EvidenceRole, source: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|error| AppError::io(source, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Validation(
            "Evidence source must be a regular, non-symbolic-link file.".into(),
        ));
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !role.allowed_extensions().contains(&extension.as_str()) {
        return Err(AppError::FileType {
            role: role.as_str().into(),
            extension,
        });
    }
    validate_signature(source, &extension)?;
    Ok(())
}

fn validate_signature(source: &Path, extension: &str) -> Result<()> {
    let mut file = fs::File::open(source).map_err(|error| AppError::io(source, error))?;
    let mut header = [0_u8; 64];
    let count = file
        .read(&mut header)
        .map_err(|error| AppError::io(source, error))?;
    let bytes = &header[..count];
    let matches = match extension {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "webp" => riff_kind(bytes, b"WEBP"),
        "pdf" => bytes.starts_with(b"%PDF-"),
        "zip" => {
            bytes.starts_with(b"PK\x03\x04")
                || bytes.starts_with(b"PK\x05\x06")
                || bytes.starts_with(b"PK\x07\x08")
        }
        "wav" => riff_kind(bytes, b"WAVE"),
        "aif" | "aiff" => {
            bytes.starts_with(b"FORM")
                && bytes.len() >= 12
                && (&bytes[8..12] == b"AIFF" || &bytes[8..12] == b"AIFC")
        }
        "mp3" => {
            bytes.starts_with(b"ID3")
                || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
        }
        "flac" => bytes.starts_with(b"fLaC"),
        "ogg" => bytes.starts_with(b"OggS"),
        "mp4" | "m4v" | "m4a" => bytes.len() >= 12 && &bytes[4..8] == b"ftyp",
        "txt" | "md" | "json" | "rb" | "py" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs"
        | "java" | "kt" | "kts" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "cs" | "rs" | "go"
        | "php" | "swift" | "scala" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "lua" | "r"
        | "jl" | "ex" | "exs" | "erl" | "hrl" | "fs" | "fsx" | "vb" | "sql" | "html" | "htm"
        | "css" | "scss" | "sass" | "less" | "xml" | "yaml" | "yml" | "toml" | "csv" | "ipynb"
        | "svg" => valid_text_prefix(bytes),
        _ => false,
    };
    if count == 0 || !matches {
        return Err(AppError::Validation(format!(
            "Evidence file contents do not match the .{extension} file type."
        )));
    }
    Ok(())
}

fn riff_kind(bytes: &[u8], kind: &[u8; 4]) -> bool {
    bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == kind
}

fn valid_text_prefix(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }
    match std::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none() && error.valid_up_to() + 4 >= bytes.len(),
    }
}

pub fn import(
    track_root: &Path,
    track_title: &str,
    role: EvidenceRole,
    source: &Path,
) -> Result<EvidenceItem> {
    validate_type(&role, source)?;
    let relative = managed_relative_path(track_title, &role, source)?;
    let destination = contained_path(track_root, &relative, false)?;
    let (sha256, size_bytes) = copy_new_hashed(source, &destination)?;
    Ok(build_item_from_copy(
        role,
        relative,
        &destination,
        sha256,
        size_bytes,
    ))
}

pub fn managed_relative_path(
    track_title: &str,
    role: &EvidenceRole,
    source: &Path,
) -> Result<PathBuf> {
    let original_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Evidence file name is invalid.".into()))?;
    reject_unsafe_file_name(original_name)?;
    Ok(
        PathBuf::from(role.destination()).join(managed_file_name(
            track_title,
            role,
            original_name,
        )?),
    )
}

/// Replaces one explicitly selected evidence record without overwriting or
/// deleting its previous bytes. The old file is moved into the excluded local
/// archive, and filesystem changes are rolled back if persistence fails.
pub fn replace<F>(
    track_root: &Path,
    track_title: &str,
    role: EvidenceRole,
    source: &Path,
    previous: &EvidenceItem,
    persist: F,
) -> Result<EvidenceItem>
where
    F: FnOnce(&EvidenceItem) -> Result<()>,
{
    if previous.role != role {
        return Err(AppError::Validation(
            "The selected evidence replacement does not match the existing role.".into(),
        ));
    }
    validate_type(&role, source)?;
    let relative = managed_relative_path(track_title, &role, source)?;
    let destination = contained_path(track_root, &relative, false)?;
    let previous_path = contained_path(track_root, Path::new(&previous.relative_path), false)?;
    let previous_name = previous_path.file_name().ok_or_else(|| {
        AppError::Validation("The existing evidence path has no file name.".into())
    })?;
    let transaction_id = Uuid::new_v4().to_string();
    let archive_relative = PathBuf::from(".archive/evidence-replacements").join(&transaction_id);
    let archived_path = contained_path(track_root, &archive_relative.join(previous_name), false)?;
    let mut archived_previous: Option<PathBuf> = None;

    let (sha256, size_bytes) = if destination == previous_path {
        if previous_path.is_file() {
            let stage_relative =
                relative.with_file_name(format!(".replacement-{transaction_id}.tmp"));
            let stage_path = contained_path(track_root, &stage_relative, false)?;
            let copied = copy_new_hashed(source, &stage_path)?;
            ensure_contained_directory(track_root, &archive_relative)?;
            if let Err(error) = fs::rename(&previous_path, &archived_path) {
                let _ = fs::remove_file(&stage_path);
                return Err(AppError::io(&previous_path, error));
            }
            archived_previous = Some(archived_path.clone());
            if let Err(error) = fs::rename(&stage_path, &destination) {
                let _ = fs::rename(&archived_path, &previous_path);
                let _ = fs::remove_file(&stage_path);
                let _ = fs::remove_dir_all(track_root.join(&archive_relative));
                return Err(AppError::io(&destination, error));
            }
            copied
        } else {
            copy_new_hashed(source, &destination)?
        }
    } else {
        let copied = copy_new_hashed(source, &destination)?;
        if previous_path.is_file() {
            ensure_contained_directory(track_root, &archive_relative)?;
            if let Err(error) = fs::rename(&previous_path, &archived_path) {
                let _ = fs::remove_file(&destination);
                let _ = fs::remove_dir_all(track_root.join(&archive_relative));
                return Err(AppError::io(&previous_path, error));
            }
            archived_previous = Some(archived_path);
        }
        copied
    };

    let mut item = build_item_from_copy(role, relative, &destination, sha256, size_bytes);
    item.id = previous.id.clone();
    if let Err(error) = persist(&item) {
        let _ = fs::remove_file(&destination);
        if let Some(archived) = &archived_previous {
            let _ = fs::rename(archived, &previous_path);
        }
        let _ = fs::remove_dir_all(track_root.join(&archive_relative));
        return Err(error);
    }
    Ok(item)
}

pub fn register_global(
    root: &Path,
    role: EvidenceRole,
    source: &Path,
) -> Result<GlobalEvidenceItem> {
    if role != EvidenceRole::SubscriptionPayment && role != EvidenceRole::Other {
        return Err(AppError::Validation(
            "Only subscription/payment or other reusable evidence may be registered globally."
                .into(),
        ));
    }
    validate_type(&role, source)?;
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Evidence file name is invalid.".into()))?;
    reject_unsafe_file_name(name)?;
    let relative = PathBuf::from(".suno-doc/global-evidence").join(name);
    let destination = contained_path(root, &relative, false)?;
    copy_new(source, &destination)?;
    Ok(GlobalEvidenceItem {
        evidence: build_item(role, relative, &destination)?,
        notes: None,
    })
}

pub fn portable_global_copy(
    track_root: &Path,
    global: &GlobalEvidenceItem,
    source: &Path,
) -> Result<EvidenceItem> {
    if sha256_file(source)? != global.evidence.sha256.clone().unwrap_or_default() {
        return Err(AppError::Validation(
            "The registered global evidence has changed and cannot be attached.".into(),
        ));
    }
    let original_name = Path::new(&global.evidence.file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Global evidence file name is invalid.".into()))?;
    reject_unsafe_file_name(original_name)?;
    let short_id: String = global
        .evidence
        .id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    if short_id.len() < 4 {
        return Err(AppError::Data("Global evidence ID is invalid.".into()));
    }
    let name = format!("subscription_{short_id}_{original_name}");
    let relative = PathBuf::from("04_LICENSES").join(name);
    let destination = contained_path(track_root, &relative, false)?;
    copy_new(source, &destination)?;
    let mut item = build_item(EvidenceRole::SubscriptionPayment, relative, &destination)?;
    item.source_global_evidence_id = Some(global.evidence.id.clone());
    item.coverage_start = global.evidence.coverage_start.clone();
    item.coverage_end = global.evidence.coverage_end.clone();
    item.provenance = EvidenceProvenance::GlobalCopy;
    Ok(item)
}

pub fn inspect(track_root: &Path, item: EvidenceItem) -> Result<EvidenceItem> {
    inspect_internal(track_root, item)
}

/// Normal track loading performs only a bounded check for large evidence.
/// Full SHA-256 verification remains available through `verify` and the
/// finalization integrity gate.
fn inspect_internal(track_root: &Path, mut item: EvidenceItem) -> Result<EvidenceItem> {
    let path = contained_path(track_root, Path::new(&item.relative_path), false)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            item.verified = false;
            item.size_bytes = 0;
            item.verification_error = Some("Evidence file is missing.".into());
            return Ok(item);
        }
        Err(error) => return Err(AppError::io(&path, error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(AppError::Symlink(path.display().to_string()));
    }
    if !metadata.is_file() {
        item.verified = false;
        item.size_bytes = 0;
        item.verification_error = Some("Evidence path is not a regular file.".into());
        return Ok(item);
    }
    if metadata.len() != item.size_bytes {
        item.verified = false;
        item.size_bytes = metadata.len();
        item.verification_error =
            Some("Evidence file size changed; run verification again.".into());
        return Ok(item);
    }
    if metadata.len() > AUTOMATIC_HASH_LIMIT_BYTES {
        return Ok(item);
    }
    verify_internal(track_root, item, false)
}

pub fn verify(track_root: &Path, item: EvidenceItem) -> Result<EvidenceItem> {
    if item.provenance == EvidenceProvenance::IndexedLegacy {
        let path = contained_path(track_root, Path::new(&item.relative_path), false)?;
        if let Err(error) = validate_type(&item.role, &path) {
            let mut rejected = item;
            rejected.verified = false;
            rejected.verification_error =
                Some(format!("Legacy evidence type verification failed: {error}"));
            return Ok(rejected);
        }
    }
    verify_internal(track_root, item, true)
}

fn verify_internal(
    track_root: &Path,
    mut item: EvidenceItem,
    accept_legacy_provenance: bool,
) -> Result<EvidenceItem> {
    // External deletion is an integrity failure, not a reason to make the
    // entire track unloadable. Existing symlink components are still rejected.
    let path = contained_path(track_root, Path::new(&item.relative_path), false)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            item.verified = false;
            item.size_bytes = 0;
            item.verification_error = Some("Evidence file is missing.".into());
            return Ok(item);
        }
        Err(error) => return Err(AppError::io(&path, error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(AppError::Symlink(path.display().to_string()));
    }
    if !metadata.is_file() {
        item.verified = false;
        item.size_bytes = 0;
        item.verification_error = Some("Evidence path is not a regular file.".into());
        return Ok(item);
    }
    let actual = sha256_file(&path)?;
    let expected = item.sha256.clone().unwrap_or_default();
    item.size_bytes = metadata.len();
    let matches_hash = !expected.is_empty() && actual == expected;
    let previous_error = item.verification_error.clone();
    let legacy_unverified = item.provenance == EvidenceProvenance::IndexedLegacy
        && (!item.verified || previous_error.is_some());
    if matches_hash && legacy_unverified && !accept_legacy_provenance {
        item.verified = false;
        item.verification_error = previous_error
            .or_else(|| Some("Indexed track evidence requires explicit verification.".into()));
    } else {
        item.verified = matches_hash;
        item.verification_error = (!item.verified).then(|| "SHA-256 mismatch.".into());
    }
    Ok(item)
}

fn build_item(role: EvidenceRole, relative: PathBuf, destination: &Path) -> Result<EvidenceItem> {
    let sha256 = sha256_file(destination)?;
    let size_bytes = fs::metadata(destination)
        .map_err(|e| AppError::io(destination, e))?
        .len();
    Ok(build_item_from_copy(
        role,
        relative,
        destination,
        sha256,
        size_bytes,
    ))
}

fn build_item_from_copy(
    role: EvidenceRole,
    relative: PathBuf,
    destination: &Path,
    sha256: String,
    size_bytes: u64,
) -> EvidenceItem {
    EvidenceItem {
        id: Uuid::new_v4().to_string(),
        role,
        file_name: destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("evidence")
            .to_owned(),
        relative_path: portable_relative(&relative),
        sha256: Some(sha256),
        size_bytes,
        imported_at: Utc::now().to_rfc3339(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: EvidenceProvenance::ManagedCopy,
        derived_from_evidence_id: None,
        generator_version: None,
        generated_disclosure_text: None,
    }
}

fn reject_unsafe_file_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|c| c.is_control() || c == '\\' || c == '/')
    {
        return Err(AppError::Validation(
            "Evidence file name contains unsafe characters.".into(),
        ));
    }
    Ok(())
}

fn managed_file_name(track_title: &str, role: &EvidenceRole, original: &str) -> Result<String> {
    let extension = Path::new(original)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let release_role = matches!(
        role,
        EvidenceRole::ReleaseWav | EvidenceRole::ReleaseMp3 | EvidenceRole::ReleaseMp4
    );
    let safe_title = if release_role {
        crate::security::safe_file_stem(track_title)?
    } else {
        crate::security::slugify(track_title)?
    };
    let suffix = match role {
        EvidenceRole::AiArtworkOriginal => Some("AI_ORIGINAL"),
        EvidenceRole::AiArtworkEdited => Some("AI_EDITED"),
        EvidenceRole::HumanEditedArtwork => Some("EDITED"),
        EvidenceRole::FinalArtwork => Some("FINAL"),
        _ => None,
    };
    Ok(match suffix {
        Some(suffix) => format!("{safe_title}_{suffix}.{extension}"),
        None if release_role => format!("{safe_title}.{extension}"),
        None => original.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn evidence_import_validates_type_preserves_source_and_rejects_collision() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let source = directory.path().join("release.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVEevidence").expect("source evidence");

        let imported = import(&track_root, "Test Track", EvidenceRole::ReleaseWav, &source)
            .expect("evidence import");
        assert_eq!(imported.relative_path, "01_RELEASE/Test Track.wav");
        assert!(imported.verified);
        assert_eq!(
            fs::read(&source).expect("source remains"),
            b"RIFF\x08\0\0\0WAVEevidence"
        );
        assert_eq!(
            fs::read(track_root.join(&imported.relative_path)).expect("managed copy"),
            b"RIFF\x08\0\0\0WAVEevidence"
        );

        let collision = import(&track_root, "Test Track", EvidenceRole::ReleaseWav, &source)
            .expect_err("duplicate target must not be overwritten");
        assert!(matches!(collision, AppError::Collision(_)));
        assert_eq!(
            fs::read(track_root.join(&imported.relative_path)).expect("preserved managed copy"),
            b"RIFF\x08\0\0\0WAVEevidence"
        );

        let wrong_type = directory.path().join("not-a-wave.txt");
        fs::write(&wrong_type, b"not wave evidence").expect("wrong type source");
        assert!(matches!(
            import(
                &track_root,
                "Test Track",
                EvidenceRole::ReleaseWav,
                &wrong_type
            ),
            Err(AppError::FileType { .. })
        ));
    }

    #[test]
    fn release_import_uses_a_safe_track_title_and_preserves_the_actual_extension() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let mp3 = directory.path().join("source-master.mp3");
        fs::write(&mp3, b"ID3release evidence").expect("MP3 source");

        let imported = import(
            &track_root,
            "Neon: Universe?",
            EvidenceRole::ReleaseWav,
            &mp3,
        )
        .expect("release import");

        assert_eq!(imported.file_name, "Neon-Universe.mp3");
        assert_eq!(imported.relative_path, "01_RELEASE/Neon-Universe.mp3");
        assert!(track_root.join("01_RELEASE/Neon-Universe.mp3").is_file());
        assert!(!track_root.join("01_RELEASE/source-master.mp3").exists());
    }

    #[test]
    fn artwork_import_uses_documented_role_naming() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let source = directory.path().join("original.png");
        fs::write(&source, b"\x89PNG\r\n\x1a\nfixture bytes").expect("artwork source");

        let imported = import(
            &track_root,
            "My Track",
            EvidenceRole::AiArtworkOriginal,
            &source,
        )
        .expect("artwork import");
        assert_eq!(
            imported.relative_path,
            "05_ARTWORK/My-Track_AI_ORIGINAL.png"
        );

        let disguised = directory.path().join("disguised.png");
        fs::write(&disguised, b"plain text, not a PNG").expect("disguised artwork");
        assert!(matches!(
            import(
                &track_root,
                "Other Track",
                EvidenceRole::AiArtworkOriginal,
                &disguised
            ),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn source_code_import_accepts_text_formats_and_rejects_binary_content() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let source = directory.path().join("generator.py");
        fs::write(&source, b"def render_note(value):\n    return value * 2\n")
            .expect("Python source");

        let imported = import(
            &track_root,
            "Code Track",
            EvidenceRole::SourceCodeFile,
            &source,
        )
        .expect("source-code import");

        assert_eq!(imported.relative_path, "02_SUNO/generator.py");
        assert_eq!(
            fs::read(track_root.join(&imported.relative_path)).expect("managed source code"),
            b"def render_note(value):\n    return value * 2\n"
        );

        let binary = directory.path().join("binary.py");
        fs::write(&binary, b"\0\x01\x02not source text").expect("binary fixture");
        assert!(matches!(
            import(
                &track_root,
                "Binary Track",
                EvidenceRole::SourceCodeFile,
                &binary
            ),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn code_generated_audio_import_accepts_only_wav_or_mp3() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");

        let wav = directory.path().join("generated.wav");
        fs::write(&wav, b"RIFF\x08\0\0\0WAVEgenerated audio").expect("WAV fixture");
        let imported_wav = import(
            &track_root,
            "Code Track",
            EvidenceRole::CodeGeneratedAudioFile,
            &wav,
        )
        .expect("generated WAV import");
        assert_eq!(imported_wav.relative_path, "02_SUNO/generated.wav");

        let mp3 = directory.path().join("generated.mp3");
        fs::write(&mp3, b"ID3generated audio").expect("MP3 fixture");
        let second_track_root = directory.path().join("second-track");
        fs::create_dir(&second_track_root).expect("second track root");
        let imported_mp3 = import(
            &second_track_root,
            "Code Track 2",
            EvidenceRole::CodeGeneratedAudioFile,
            &mp3,
        )
        .expect("generated MP3 import");
        assert_eq!(imported_mp3.relative_path, "02_SUNO/generated.mp3");

        let flac = directory.path().join("generated.flac");
        fs::write(&flac, b"fLaCgenerated audio").expect("FLAC fixture");
        assert!(matches!(
            import(
                &second_track_root,
                "Code Track 2",
                EvidenceRole::CodeGeneratedAudioFile,
                &flac,
            ),
            Err(AppError::FileType { .. })
        ));
    }

    #[test]
    fn large_evidence_load_is_bounded_but_explicit_verification_hashes_it() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let source = directory.path().join("project.zip");
        fs::write(&source, b"PK\x03\x04small fixture").expect("ZIP source");
        let mut item = import(
            &track_root,
            "Large Project",
            EvidenceRole::SunoProjectZip,
            &source,
        )
        .expect("initial import");
        let managed = track_root.join(&item.relative_path);
        let large_size = AUTOMATIC_HASH_LIMIT_BYTES + 1;
        fs::OpenOptions::new()
            .write(true)
            .open(&managed)
            .expect("managed file")
            .set_len(large_size)
            .expect("sparse large fixture");
        item.size_bytes = large_size;
        item.sha256 = Some("0".repeat(64));
        item.verified = true;

        let inspected = inspect(&track_root, item.clone()).expect("bounded inspection");
        assert!(
            inspected.verified,
            "normal loading must not re-hash a large file"
        );

        let verified = verify(&track_root, item).expect("explicit full verification");
        assert!(!verified.verified);
        assert_eq!(
            verified.verification_error.as_deref(),
            Some("SHA-256 mismatch.")
        );
    }

    #[test]
    fn explicit_replacement_archives_previous_bytes_and_reuses_database_identity() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        fs::create_dir(&track_root).expect("track root");
        let old_source_root = directory.path().join("old");
        let new_source_root = directory.path().join("new");
        fs::create_dir_all(&old_source_root).expect("old source root");
        fs::create_dir_all(&new_source_root).expect("new source root");
        let old_source = old_source_root.join("project.zip");
        let new_source = new_source_root.join("project.zip");
        fs::write(&old_source, b"PK\x03\x04old project").expect("old ZIP");
        fs::write(&new_source, b"PK\x03\x04new project").expect("new ZIP");
        let previous = import(
            &track_root,
            "Replace Project",
            EvidenceRole::SunoProjectZip,
            &old_source,
        )
        .expect("initial import");
        let previous_id = previous.id.clone();

        let replacement = replace(
            &track_root,
            "Replace Project",
            EvidenceRole::SunoProjectZip,
            &new_source,
            &previous,
            |_| Ok(()),
        )
        .expect("safe replacement");

        assert_eq!(replacement.id, previous_id);
        assert_eq!(replacement.relative_path, previous.relative_path);
        assert_eq!(
            fs::read(track_root.join(&replacement.relative_path)).expect("replacement bytes"),
            b"PK\x03\x04new project"
        );
        assert_eq!(
            fs::read(&new_source).expect("replacement source remains"),
            b"PK\x03\x04new project"
        );
        let archived = fs::read_dir(track_root.join(".archive/evidence-replacements"))
            .expect("replacement archive")
            .next()
            .expect("archive transaction")
            .expect("archive entry")
            .path()
            .join(&previous.file_name);
        assert_eq!(
            fs::read(archived).expect("archived bytes"),
            b"PK\x03\x04old project"
        );
    }
}
