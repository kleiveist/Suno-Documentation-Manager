use crate::error::{AppError, Result};
use crate::model::{EvidenceItem, EvidenceProvenance, EvidenceRole, GlobalEvidenceItem};
use crate::security::{contained_path, copy_new, portable_relative, sha256_file};
use chrono::Utc;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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
        "txt" | "md" | "json" => valid_text_prefix(bytes),
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
    let original_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Evidence file name is invalid.".into()))?;
    reject_unsafe_file_name(original_name)?;
    let name = managed_file_name(track_title, &role, original_name)?;
    let relative = PathBuf::from(role.destination()).join(name);
    let destination = contained_path(track_root, &relative, false)?;
    copy_new(source, &destination)?;
    build_item(role, relative, &destination)
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
    Ok(EvidenceItem {
        id: Uuid::new_v4().to_string(),
        role,
        file_name: destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("evidence")
            .to_owned(),
        relative_path: portable_relative(&relative),
        sha256: Some(sha256_file(destination)?),
        size_bytes: fs::metadata(destination)
            .map_err(|e| AppError::io(destination, e))?
            .len(),
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
    })
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
    let safe_title = crate::security::slugify(track_title)?;
    let suffix = match role {
        EvidenceRole::AiArtworkOriginal => Some("AI_ORIGINAL"),
        EvidenceRole::AiArtworkEdited => Some("AI_EDITED"),
        EvidenceRole::HumanEditedArtwork => Some("EDITED"),
        EvidenceRole::FinalArtwork => Some("FINAL"),
        _ => None,
    };
    Ok(match suffix {
        Some(suffix) => format!("{safe_title}_{suffix}.{extension}"),
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
        assert_eq!(imported.relative_path, "01_RELEASE/release.wav");
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
}
