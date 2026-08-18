use crate::certificate;
use crate::certificate_pdf::{self, ExternalTimestampPdfSnapshot};
use crate::error::{AppError, Result};
use crate::evidence;
use crate::integrity;
use crate::model::{
    ExternalTimestampInput, ExternalTimestampRecord, FinalizationAnchor,
    TimestampReferencedArtifact, TimestampType,
};
use crate::security::{
    atomic_write_new, contained_path, copy_new_hashed, ensure_contained_directory,
    portable_relative, sha256_file, validate_relative,
};
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

pub const EXTERNAL_TIMESTAMPS_DIR: &str = "06_CERTIFICATE/EXTERNAL_TIMESTAMPS";
const STAGING_DIR: &str = ".archive/timestamp-staging";
const RECORD_FILE: &str = "TIMESTAMP_RECORD.json";
const MARKDOWN_FILE: &str = "EXTERNAL_TIMESTAMP_ADDENDUM.md";
const PDF_FILE: &str = "EXTERNAL_TIMESTAMP_ADDENDUM.pdf";
const HASH_LIST_FILE: &str = "TIMESTAMP_RECORD_SHA256.txt";
const SIDECAR_FORMAT_VERSION: u32 = 1;
const HASH_LIST_V1_HEADER: &str = "# SunoDM external timestamp sidecar SHA-256 v1\n";
const DISCLAIMER: &str = "The application records the external timestamp evidence and its referenced hash. It does not independently determine the timestamp's legal qualification unless explicitly technically verified.";

#[derive(Debug)]
pub struct StagedExternalTimestamp {
    pub record: ExternalTimestampRecord,
    stage_relative: PathBuf,
    live_relative: PathBuf,
}

/// Build and durably stage a complete timestamp sidecar. The caller must
/// register `record` in SQLite before calling [`publish`], so a process exit can
/// never leave a new live sidecar that is invisible to the database.
pub fn stage(
    track_root: &Path,
    certificate_id: &str,
    source: &Path,
    input: ExternalTimestampInput,
) -> Result<StagedExternalTimestamp> {
    validate_input(&input)?;
    evidence::validate_type(&crate::model::EvidenceRole::ExternalTimestamp, source)?;
    let evidence_file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Timestamp evidence file name is invalid.".into()))?
        .to_owned();
    if evidence_file_name
        .chars()
        .any(|value| value.is_control() || value == '/' || value == '\\')
    {
        return Err(AppError::Validation(
            "Timestamp evidence file name contains unsafe characters.".into(),
        ));
    }

    let referenced_relative = referenced_artifact_path(&input)?;
    let referenced_path = contained_path(track_root, &referenced_relative, true)?;
    let metadata = fs::symlink_metadata(&referenced_path)
        .map_err(|error| AppError::io(&referenced_path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Validation(
            "The referenced timestamp artifact must be a regular managed file.".into(),
        ));
    }
    let actual_sha256 = sha256_file(&referenced_path)?;
    if input.referenced_artifact == TimestampReferencedArtifact::Other
        && integrity::listed_hash(track_root, &referenced_relative)?.as_deref()
            != Some(actual_sha256.as_str())
    {
        return Err(AppError::Validation(
            "Other timestamp artifacts must be an unchanged entry in the verified phase-one SHA256SUMS.txt file."
                .into(),
        ));
    }
    let referenced_sha256 = input.referenced_sha256.trim().to_ascii_lowercase();
    let referenced_hash_match = Some(actual_sha256 == referenced_sha256);

    let id = Uuid::new_v4().to_string();
    let live_relative = PathBuf::from(EXTERNAL_TIMESTAMPS_DIR).join(&id);
    let live_directory = contained_path(track_root, &live_relative, false)?;
    if live_directory.exists() {
        return Err(AppError::Collision(live_directory.display().to_string()));
    }
    let staging_parent = ensure_contained_directory(track_root, Path::new(STAGING_DIR))?;
    sync_directory(&staging_parent)?;
    sync_directory(
        staging_parent
            .parent()
            .ok_or_else(|| AppError::PathEscape)?,
    )?;
    let live_parent = ensure_contained_directory(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR))?;
    sync_directory(&live_parent)?;
    sync_directory(live_parent.parent().ok_or_else(|| AppError::PathEscape)?)?;
    let stage_relative = PathBuf::from(STAGING_DIR).join(&id);
    let stage_directory = ensure_contained_directory(track_root, &stage_relative)?;

    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let managed_evidence_name = format!("TIMESTAMP_EVIDENCE.{extension}");
    let live_record_relative = live_relative.join(RECORD_FILE);
    let live_markdown_relative = live_relative.join(MARKDOWN_FILE);
    let live_pdf_relative = live_relative.join(PDF_FILE);
    let live_hash_list_relative = live_relative.join(HASH_LIST_FILE);
    let imported_at = Utc::now().to_rfc3339();

    let staging = (|| -> Result<StagedExternalTimestamp> {
        let evidence_path = stage_directory.join(&managed_evidence_name);
        let (evidence_sha256, _) = copy_new_hashed(source, &evidence_path)?;
        let mut record = ExternalTimestampRecord {
            id: id.clone(),
            certificate_id: certificate_id.to_owned(),
            sidecar_format_version: SIDECAR_FORMAT_VERSION,
            provider: input.provider.trim().to_owned(),
            timestamp_type: input.timestamp_type,
            timestamp_value: input.timestamp_value.trim().to_owned(),
            referenced_artifact: input.referenced_artifact,
            referenced_artifact_path: portable_relative(&referenced_relative),
            referenced_sha256,
            actual_sha256,
            referenced_hash_match,
            external_reference_id: input.external_reference_id.trim().to_owned(),
            provider_verification_url: input.provider_verification_url.trim().to_owned(),
            note: input.note.trim().to_owned(),
            evidence_file_name,
            evidence_sha256,
            markdown_sha256: String::new(),
            pdf_sha256: String::new(),
            imported_at,
            provenance: "Managed copy; user-confirmed metadata; system-verified SHA-256 comparison"
                .into(),
            record_relative_path: portable_relative(&live_record_relative),
            markdown_relative_path: portable_relative(&live_markdown_relative),
            pdf_relative_path: portable_relative(&live_pdf_relative),
            hash_list_relative_path: portable_relative(&live_hash_list_relative),
            integrity_verified_at_publication: true,
            integrity_verified: true,
            integrity_issues: Vec::new(),
        };

        let markdown = render_markdown(&record);
        atomic_write_new(&stage_directory.join(MARKDOWN_FILE), markdown.as_bytes())?;
        let pdf = render_pdf(&record)?;
        atomic_write_new(&stage_directory.join(PDF_FILE), &pdf)?;
        record.markdown_sha256 = sha256_file(&stage_directory.join(MARKDOWN_FILE))?;
        record.pdf_sha256 = sha256_file(&stage_directory.join(PDF_FILE))?;
        let record_bytes = immutable_record_bytes(&record)?;
        atomic_write_new(&stage_directory.join(RECORD_FILE), &record_bytes)?;

        let hashes = artifact_hashes(&stage_directory, &managed_evidence_name)?;
        let hash_list = render_hash_list(record.sidecar_format_version, &hashes)?;
        atomic_write_new(&stage_directory.join(HASH_LIST_FILE), hash_list.as_bytes())?;
        verify_staged_hashes(&stage_directory, &hashes)?;
        verify_record_in_directory(track_root, &stage_directory, &record, None)?;
        // The database row is written only after `stage` returns. Sync both the
        // completed directory contents and its parent entry so that a crash can
        // leave either a recoverable complete stage or no registered record.
        sync_directory(&stage_directory)?;
        if let Some(parent) = stage_directory.parent() {
            sync_directory(parent)?;
        }

        Ok(StagedExternalTimestamp {
            record,
            stage_relative,
            live_relative,
        })
    })();

    if staging.is_err() && stage_directory.exists() {
        let _ = fs::remove_dir_all(&stage_directory);
    }
    staging
}

/// Publish a staged record after its database row exists. Both directories are
/// synced around the rename so startup recovery sees either the complete stage
/// or the complete live sidecar.
pub fn publish(
    track_root: &Path,
    staged: &StagedExternalTimestamp,
) -> Result<ExternalTimestampRecord> {
    let stage_directory = contained_path(track_root, &staged.stage_relative, true)?;
    let live_directory = contained_path(track_root, &staged.live_relative, false)?;
    if live_directory.exists() {
        return Err(AppError::Collision(live_directory.display().to_string()));
    }
    verify_record_in_directory(track_root, &stage_directory, &staged.record, None)?;
    fs::rename(&stage_directory, &live_directory)
        .map_err(|error| AppError::io(&live_directory, error))?;
    sync_directory(
        live_directory
            .parent()
            .ok_or_else(|| AppError::PathEscape)?,
    )?;
    if let Some(stage_parent) = stage_directory.parent() {
        sync_directory(stage_parent)?;
    }
    verify_published_record(track_root, &staged.record)?;
    Ok(staged.record.clone())
}

pub fn discard_staged(track_root: &Path, staged: &StagedExternalTimestamp) -> Result<()> {
    let directory = contained_path(track_root, &staged.stage_relative, false)?;
    if directory.exists() {
        fs::remove_dir_all(&directory).map_err(|error| AppError::io(&directory, error))?;
        if let Some(parent) = directory.parent() {
            sync_directory(parent)?;
        }
    }
    Ok(())
}

pub fn remove_published_record(track_root: &Path, record: &ExternalTimestampRecord) -> Result<()> {
    let record_path = Path::new(&record.record_relative_path);
    let Some(relative_directory) = record_path.parent() else {
        return Err(AppError::PathEscape);
    };
    if relative_directory.parent() != Some(Path::new(EXTERNAL_TIMESTAMPS_DIR)) {
        return Err(AppError::PathEscape);
    }
    let directory = contained_path(track_root, relative_directory, false)?;
    if directory.exists() {
        let parent = directory.parent().ok_or_else(|| AppError::PathEscape)?;
        fs::remove_dir_all(&directory).map_err(|error| AppError::io(&directory, error))?;
        // Keep the database registration until the directory removal is
        // durably visible. If this sync fails, the caller deliberately retains
        // the row so startup recovery can never encounter an unregistered live
        // sidecar after a power loss.
        sync_directory(parent)?;
    }
    Ok(())
}

/// Re-verify every published sidecar artifact against the certificate-bound
/// database record. This deliberately does not fold the addendum into the
/// phase-one integrity set, so a damaged addendum is reported independently
/// without changing the finalized certificate bytes or their validity.
pub fn verify_published_record(track_root: &Path, record: &ExternalTimestampRecord) -> Result<()> {
    let location = resolve_published_record(track_root, record)?;
    verify_record_in_directory(
        track_root,
        &location.directory,
        record,
        location.revision_root.as_deref(),
    )
}

fn verify_record_in_directory(
    track_root: &Path,
    directory: &Path,
    record: &ExternalTimestampRecord,
    revision_root: Option<&Path>,
) -> Result<()> {
    record_directory(record)?;
    let extension = Path::new(&record.evidence_file_name)
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::Validation("Timestamp evidence record has no valid extension.".into())
        })?
        .to_ascii_lowercase();
    if !crate::model::EvidenceRole::ExternalTimestamp
        .allowed_extensions()
        .contains(&extension.as_str())
    {
        return Err(AppError::Validation(
            "Timestamp evidence record has an unsupported extension.".into(),
        ));
    }
    let managed_evidence_name = format!("TIMESTAMP_EVIDENCE.{extension}");
    let expected_names = BTreeSet::from([
        RECORD_FILE.to_owned(),
        managed_evidence_name.clone(),
        MARKDOWN_FILE.to_owned(),
        PDF_FILE.to_owned(),
        HASH_LIST_FILE.to_owned(),
    ]);
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(&directory).map_err(|error| AppError::io(&directory, error))? {
        let entry = entry.map_err(|error| AppError::io(&directory, error))?;
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| AppError::Validation("Timestamp sidecar filename is invalid.".into()))?
            .to_owned();
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::Validation(format!(
                "Timestamp sidecar contains a non-regular file: {name}"
            )));
        }
        actual_names.insert(name);
    }
    if actual_names != expected_names {
        return Err(AppError::Validation(
            "Timestamp sidecar file set does not match its managed record.".into(),
        ));
    }

    let record_path = directory.join(RECORD_FILE);
    let record_bytes = fs::read(&record_path).map_err(|error| AppError::io(&record_path, error))?;
    let stored_record: ExternalTimestampRecord = serde_json::from_slice(&record_bytes)?;
    match stored_record.sidecar_format_version {
        0 if !stored_record.integrity_verified => {
            return Err(AppError::Validation(
                "Legacy timestamp sidecar has no publication-time integrity assertion.".into(),
            ));
        }
        0 if !immutable_records_match(&stored_record, record) => {
            return Err(AppError::Validation(
                "Legacy TIMESTAMP_RECORD.json differs from the registered timestamp record.".into(),
            ));
        }
        0 => {}
        SIDECAR_FORMAT_VERSION if !stored_record.integrity_verified_at_publication => {
            return Err(AppError::Validation(
                "Timestamp sidecar does not record successful publication-time integrity verification."
                    .into(),
            ));
        }
        SIDECAR_FORMAT_VERSION => {
            // Current sidecars have one canonical immutable JSON
            // representation. A semantic deserialize/compare would accept
            // unknown or runtime-only claims after an attacker regenerated the
            // self-contained hash list; exact bytes reject that ambiguity.
            if record_bytes != immutable_record_bytes(record)? {
                return Err(AppError::Validation(
                    "TIMESTAMP_RECORD.json is not the exact immutable registered record.".into(),
                ));
            }
        }
        version => {
            return Err(AppError::Validation(format!(
                "Unsupported external timestamp sidecar format version: {version}."
            )));
        }
    }

    // Verify the exact immutable bytes that were published. Do not re-render
    // Markdown or PDF: renderer changes must not invalidate historical records.
    let hashes = artifact_hashes(directory, &managed_evidence_name)?;
    let evidence_sha256 = hashes
        .get(&managed_evidence_name)
        .ok_or_else(|| AppError::Data("External timestamp evidence hash is missing.".into()))?;
    if evidence_sha256 != &record.evidence_sha256 {
        return Err(AppError::Validation(
            "External timestamp evidence SHA-256 no longer matches its registered value.".into(),
        ));
    }
    let expected_hash_list = render_hash_list(stored_record.sidecar_format_version, &hashes)?;
    let hash_list_path = directory.join(HASH_LIST_FILE);
    let hash_list =
        fs::read(&hash_list_path).map_err(|error| AppError::io(&hash_list_path, error))?;
    if hash_list != expected_hash_list.as_bytes() {
        return Err(AppError::Validation(
            "Timestamp sidecar SHA-256 list is incomplete or no longer matches.".into(),
        ));
    }
    if stored_record.sidecar_format_version == SIDECAR_FORMAT_VERSION {
        let markdown_sha256 = hashes
            .get(MARKDOWN_FILE)
            .ok_or_else(|| AppError::Data("Timestamp Markdown hash is missing.".into()))?;
        let pdf_sha256 = hashes
            .get(PDF_FILE)
            .ok_or_else(|| AppError::Data("Timestamp PDF hash is missing.".into()))?;
        if &stored_record.markdown_sha256 != markdown_sha256 {
            return Err(AppError::Validation(
                "External timestamp Markdown bytes no longer match their publication hash.".into(),
            ));
        }
        if &stored_record.pdf_sha256 != pdf_sha256 {
            return Err(AppError::Validation(
                "External timestamp PDF bytes no longer match their publication hash.".into(),
            ));
        }
    }

    let referenced_relative = PathBuf::from(&record.referenced_artifact_path);
    validate_stable_artifact_relative(&referenced_relative)?;
    let actual_referenced_sha256 =
        verify_referenced_artifact(track_root, revision_root, &referenced_relative, record)?;
    if actual_referenced_sha256 != record.actual_sha256
        || record.referenced_hash_match
            != Some(actual_referenced_sha256 == record.referenced_sha256)
    {
        return Err(AppError::Validation(
            "Timestamp record no longer matches the selected finalized artifact.".into(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct PublishedRecordLocation {
    directory: PathBuf,
    revision_root: Option<PathBuf>,
}

fn resolve_published_record(
    track_root: &Path,
    record: &ExternalTimestampRecord,
) -> Result<PublishedRecordLocation> {
    let live_relative = record_directory(record)?;
    let live = contained_path(track_root, &live_relative, false)?;
    if live.is_dir() {
        return Ok(PublishedRecordLocation {
            directory: live,
            revision_root: None,
        });
    }

    let revisions_relative = Path::new(".archive/revisions");
    let revisions = contained_path(track_root, revisions_relative, false)?;
    let mut matches = Vec::new();
    if revisions.is_dir() {
        let nested_timestamp_directory = Path::new(EXTERNAL_TIMESTAMPS_DIR)
            .strip_prefix(certificate::CERTIFICATE_DIR)
            .map_err(|_| AppError::PathEscape)?;
        for entry in fs::read_dir(&revisions).map_err(|error| AppError::io(&revisions, error))? {
            let entry = entry.map_err(|error| AppError::io(&revisions, error))?;
            let entry_path = entry.path();
            let metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| AppError::io(&entry_path, error))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            let candidate = entry_path
                .join("certificate")
                .join(nested_timestamp_directory)
                .join(&record.id);
            if candidate.is_dir() {
                let revision_metadata = entry_path.join("revision.json");
                let revision_metadata_relative = revision_metadata
                    .strip_prefix(track_root)
                    .map_err(|_| AppError::PathEscape)?;
                let revision_metadata =
                    contained_path(track_root, revision_metadata_relative, true)?;
                let metadata = fs::symlink_metadata(&revision_metadata)
                    .map_err(|error| AppError::io(&revision_metadata, error))?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(AppError::Validation(format!(
                        "Revision metadata for external timestamp {} is not a regular file.",
                        record.id
                    )));
                }
                let revision: serde_json::Value = serde_json::from_slice(
                    &fs::read(&revision_metadata)
                        .map_err(|error| AppError::io(&revision_metadata, error))?,
                )?;
                let archived_certificate_id = revision
                    .get("previous_certificate")
                    .and_then(|value| value.get("certificateId"))
                    .and_then(|value| value.as_str());
                if archived_certificate_id != Some(record.certificate_id.as_str()) {
                    return Err(AppError::Validation(format!(
                        "Revision metadata certificate ID does not match external timestamp {}.",
                        record.id
                    )));
                }
                let candidate_relative = candidate
                    .strip_prefix(track_root)
                    .map_err(|_| AppError::PathEscape)?;
                let candidate = contained_path(track_root, candidate_relative, true)?;
                matches.push(PublishedRecordLocation {
                    directory: candidate,
                    revision_root: Some(entry_path),
                });
            }
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => {
            let stage = contained_path(
                track_root,
                &PathBuf::from(STAGING_DIR).join(&record.id),
                false,
            )?;
            if stage.is_dir() {
                Err(AppError::Validation(format!(
                    "External timestamp publication {} is still staged and requires recovery.",
                    record.id
                )))
            } else {
                Err(AppError::Validation(format!(
                    "External timestamp sidecar {} is missing.",
                    record.id
                )))
            }
        }
        _ => Err(AppError::Validation(format!(
            "External timestamp sidecar {} exists in multiple revision archives.",
            record.id
        ))),
    }
}

/// Reconcile the two durable states used by phase-two publication. A registered
/// database row makes its matching stage recoverable; an unregistered stage is
/// an uncommitted operation and is removed. An unexpected live sidecar is never
/// silently adopted as a user-confirmed database fact.
pub fn reconcile_publications(
    track_root: &Path,
    registered: &[ExternalTimestampRecord],
) -> Result<bool> {
    let mut recovered = false;
    let registered_ids = registered
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();

    for record in registered {
        let live = contained_path(track_root, &record_directory(record)?, false)?;
        if live.is_dir() {
            continue;
        }
        // An archived record is already durably published and must not be
        // restored into the current certificate revision.
        if resolve_published_record(track_root, record).is_ok() {
            continue;
        }
        let stage_relative = PathBuf::from(STAGING_DIR).join(&record.id);
        let stage = contained_path(track_root, &stage_relative, false)?;
        if !stage.is_dir() {
            continue;
        }
        verify_record_in_directory(track_root, &stage, record, None)?;
        ensure_contained_directory(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR))?;
        fs::rename(&stage, &live).map_err(|error| AppError::io(&live, error))?;
        if let Some(parent) = live.parent() {
            sync_directory(parent)?;
        }
        if let Some(parent) = stage.parent() {
            sync_directory(parent)?;
        }
        verify_published_record(track_root, record)?;
        recovered = true;
    }

    let live_parent = contained_path(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR), false)?;
    if live_parent.is_dir() {
        for entry in
            fs::read_dir(&live_parent).map_err(|error| AppError::io(&live_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&live_parent, error))?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
            let file_name = entry.file_name();
            let id = file_name.to_str().ok_or_else(|| {
                AppError::Data("External timestamp directory name is not UTF-8.".into())
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || Uuid::parse_str(id).is_err()
            {
                return Err(AppError::Data(format!(
                    "Unexpected entry in the external timestamp publication directory: {}.",
                    path.display()
                )));
            }
            if !registered_ids.contains(id) {
                return Err(AppError::Data(format!(
                    "Unregistered external timestamp sidecar detected: {id}. It was not adopted automatically."
                )));
            }
        }
    }

    let staging_parent = contained_path(track_root, Path::new(STAGING_DIR), false)?;
    if staging_parent.is_dir() {
        for entry in
            fs::read_dir(&staging_parent).map_err(|error| AppError::io(&staging_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&staging_parent, error))?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
            let file_name = entry.file_name();
            let id = file_name.to_str().ok_or_else(|| {
                AppError::Data("Timestamp staging directory name is not UTF-8.".into())
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || Uuid::parse_str(id).is_err()
            {
                return Err(AppError::Data(format!(
                    "Unexpected entry in the timestamp staging directory: {}.",
                    path.display()
                )));
            }
            if registered_ids.contains(id) {
                // A registered stage that could not be recovered above remains
                // visible through its database record and must not be discarded.
                continue;
            }
            fs::remove_dir_all(&path).map_err(|error| AppError::io(&path, error))?;
            recovered = true;
        }
        if recovered {
            sync_directory(&staging_parent)?;
        }
    }
    Ok(recovered)
}

fn immutable_record_bytes(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    let mut value = serde_json::to_value(record)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::Data("Timestamp record serialization is not an object.".into()))?;
    object.remove("integrityVerified");
    object.remove("integrityIssues");
    Ok(serde_json::to_vec_pretty(&value)?)
}

fn immutable_records_match(
    stored: &ExternalTimestampRecord,
    registered: &ExternalTimestampRecord,
) -> bool {
    let mut stored = stored.clone();
    let mut registered = registered.clone();
    stored.integrity_verified = false;
    stored.integrity_issues.clear();
    registered.integrity_verified = false;
    registered.integrity_issues.clear();
    stored == registered
}

fn artifact_hashes(directory: &Path, evidence_name: &str) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for name in [RECORD_FILE, evidence_name, MARKDOWN_FILE, PDF_FILE] {
        hashes.insert(name.to_owned(), sha256_file(&directory.join(name))?);
    }
    Ok(hashes)
}

fn render_hash_list(version: u32, hashes: &BTreeMap<String, String>) -> Result<String> {
    let mut output = match version {
        0 => String::new(),
        SIDECAR_FORMAT_VERSION => HASH_LIST_V1_HEADER.to_owned(),
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported external timestamp sidecar format version: {other}."
            )));
        }
    };
    for (name, digest) in hashes {
        output.push_str(&format!("{digest}  {name}\n"));
    }
    Ok(output)
}

fn verify_referenced_artifact(
    track_root: &Path,
    revision_root: Option<&Path>,
    referenced_relative: &Path,
    record: &ExternalTimestampRecord,
) -> Result<String> {
    if let Some(revision_root) = revision_root {
        let archived_path = if let Ok(certificate_relative) =
            referenced_relative.strip_prefix(certificate::CERTIFICATE_DIR)
        {
            Some(revision_root.join("certificate").join(certificate_relative))
        } else if referenced_relative == Path::new(certificate::PDF_FILE)
            || referenced_relative == Path::new(integrity::HASH_FILE)
        {
            Some(revision_root.join(referenced_relative))
        } else {
            None
        };
        if let Some(path) = archived_path {
            let relative = path
                .strip_prefix(track_root)
                .map_err(|_| AppError::PathEscape)?;
            let path = contained_path(track_root, relative, false)?;
            if path.exists() {
                let metadata =
                    fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(AppError::Validation(
                        "The timestamp's archived referenced artifact is not a regular file."
                            .into(),
                    ));
                }
                return sha256_file(&path);
            }
        }
        if record.referenced_artifact == TimestampReferencedArtifact::Other
            && integrity::listed_hash(revision_root, referenced_relative)?.as_deref()
                == Some(record.actual_sha256.as_str())
        {
            // Revision archives retain the verified phase-one hash list even
            // when an arbitrary `Other` source byte is not duplicated.
            return Ok(record.actual_sha256.clone());
        }
        return Err(AppError::Validation(
            "The timestamp's referenced artifact is missing from its revision archive.".into(),
        ));
    }

    let referenced_path = contained_path(track_root, referenced_relative, true)?;
    let actual = sha256_file(&referenced_path)?;
    if record.referenced_artifact == TimestampReferencedArtifact::Other
        && integrity::listed_hash(track_root, referenced_relative)?.as_deref()
            != Some(actual.as_str())
    {
        return Err(AppError::Validation(
            "The Other timestamp artifact is not an unchanged phase-one SHA256SUMS entry.".into(),
        ));
    }
    Ok(actual)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> Result<()> {
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| AppError::io(directory, error))
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> Result<()> {
    // Opening a directory for fsync is not portable (notably on Windows).
    // The files themselves are still atomically written and synced before the
    // rename; directory durability remains a best-effort platform boundary.
    Ok(())
}

pub fn finalization_anchors(track_root: &Path) -> Result<Vec<FinalizationAnchor>> {
    let definitions = [
        (
            TimestampReferencedArtifact::EvidenceManifest,
            "Evidence manifest (recommended timestamp anchor)",
            certificate::MANIFEST_FILE,
        ),
        (
            TimestampReferencedArtifact::Sha256sums,
            "Track SHA-256 manifest",
            integrity::HASH_FILE,
        ),
        (
            TimestampReferencedArtifact::DocumentationCertificateMarkdown,
            "Documentation certificate (Markdown)",
            certificate::CERTIFICATE_FILE,
        ),
        (
            TimestampReferencedArtifact::CertificatePdf,
            "Documentation certificate (PDF)",
            certificate::PDF_FILE,
        ),
        (
            TimestampReferencedArtifact::FinalEvidencePackage,
            "Final evidence package certificate hash set",
            certificate::CERTIFICATE_HASH_FILE,
        ),
    ];
    definitions
        .into_iter()
        .map(|(artifact, label, relative)| {
            let path = contained_path(track_root, Path::new(relative), true)?;
            Ok(FinalizationAnchor {
                artifact,
                label: label.into(),
                relative_path: relative.into(),
                sha256: sha256_file(&path)?,
            })
        })
        .collect()
}

fn referenced_artifact_path(input: &ExternalTimestampInput) -> Result<PathBuf> {
    let fixed = match input.referenced_artifact {
        TimestampReferencedArtifact::EvidenceManifest => Some(certificate::MANIFEST_FILE),
        TimestampReferencedArtifact::Sha256sums => Some(integrity::HASH_FILE),
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            Some(certificate::CERTIFICATE_FILE)
        }
        TimestampReferencedArtifact::CertificatePdf => Some(certificate::PDF_FILE),
        TimestampReferencedArtifact::FinalEvidencePackage => {
            Some(certificate::CERTIFICATE_HASH_FILE)
        }
        TimestampReferencedArtifact::Other => None,
    };
    if let Some(relative) = fixed {
        return Ok(PathBuf::from(relative));
    }
    let relative = PathBuf::from(input.other_referenced_artifact.trim());
    validate_stable_artifact_relative(&relative)?;
    Ok(relative)
}

fn validate_stable_artifact_relative(relative: &Path) -> Result<()> {
    validate_relative(&relative)?;
    let portable = portable_relative(&relative);
    if portable.contains('\\')
        || portable.chars().any(char::is_control)
        || portable == ".archive"
        || portable.starts_with(".archive/")
        || portable == EXTERNAL_TIMESTAMPS_DIR
        || portable.starts_with(&format!("{EXTERNAL_TIMESTAMPS_DIR}/"))
    {
        return Err(AppError::Validation(
            "Other timestamp artifacts must identify a stable phase-one track file.".into(),
        ));
    }
    Ok(())
}

fn record_directory(record: &ExternalTimestampRecord) -> Result<PathBuf> {
    Uuid::parse_str(&record.id)
        .map_err(|_| AppError::Validation("Timestamp record ID is invalid.".into()))?;
    let directory = PathBuf::from(EXTERNAL_TIMESTAMPS_DIR).join(&record.id);
    validate_relative(&directory)?;
    for (actual, expected) in [
        (&record.record_relative_path, directory.join(RECORD_FILE)),
        (
            &record.markdown_relative_path,
            directory.join(MARKDOWN_FILE),
        ),
        (&record.pdf_relative_path, directory.join(PDF_FILE)),
        (
            &record.hash_list_relative_path,
            directory.join(HASH_LIST_FILE),
        ),
    ] {
        if actual != &portable_relative(&expected) {
            return Err(AppError::Validation(
                "Timestamp record contains an inconsistent managed path.".into(),
            ));
        }
    }
    Ok(directory)
}

fn validate_input(input: &ExternalTimestampInput) -> Result<()> {
    for (name, value, max, required) in [
        (
            "Timestamp provider / issuer",
            input.provider.as_str(),
            1000,
            true,
        ),
        (
            "Timestamp value",
            input.timestamp_value.as_str(),
            500,
            false,
        ),
        (
            "Other referenced artifact",
            input.other_referenced_artifact.as_str(),
            4000,
            input.referenced_artifact == TimestampReferencedArtifact::Other,
        ),
        (
            "External reference ID",
            input.external_reference_id.as_str(),
            1000,
            false,
        ),
        (
            "Provider verification URL",
            input.provider_verification_url.as_str(),
            4000,
            false,
        ),
        ("Timestamp note", input.note.as_str(), 20_000, false),
    ] {
        if required && value.trim().is_empty() {
            return Err(AppError::Validation(format!("{name} is required.")));
        }
        if value.len() > max || value.chars().any(|character| character == '\0') {
            return Err(AppError::Validation(format!(
                "{name} is invalid or too long."
            )));
        }
    }
    let digest = input.referenced_sha256.trim();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Validation(
            "Referenced hash must be a SHA-256 value.".into(),
        ));
    }
    if !input.provider_verification_url.trim().is_empty() {
        let parsed = Url::parse(input.provider_verification_url.trim())
            .map_err(|_| AppError::Validation("Provider verification URL is invalid.".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(AppError::Validation(
                "Provider verification URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

fn verify_staged_hashes(directory: &Path, hashes: &BTreeMap<String, String>) -> Result<()> {
    for (name, expected) in hashes {
        let actual = sha256_file(&directory.join(name))?;
        if &actual != expected {
            return Err(AppError::Validation(format!(
                "External timestamp addendum integrity mismatch: {name}"
            )));
        }
    }
    Ok(())
}

fn render_pdf(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    certificate_pdf::generate_external_timestamp_addendum_pdf(&ExternalTimestampPdfSnapshot {
        certificate_id: &record.certificate_id,
        provider: &record.provider,
        timestamp_type: timestamp_type_label(record.timestamp_type),
        timestamp_value: &record.timestamp_value,
        referenced_artifact: referenced_artifact_label(record.referenced_artifact),
        referenced_artifact_path: &record.referenced_artifact_path,
        referenced_sha256: &record.referenced_sha256,
        actual_sha256: &record.actual_sha256,
        referenced_hash_match: record.referenced_hash_match,
        evidence_file_name: &record.evidence_file_name,
        evidence_sha256: &record.evidence_sha256,
        imported_at: &record.imported_at,
        provenance: &record.provenance,
        external_reference_id: &record.external_reference_id,
        provider_verification_url: &record.provider_verification_url,
        note: &record.note,
    })
}

fn render_markdown(record: &ExternalTimestampRecord) -> String {
    format!(
        "# SunoDM External Timestamp Evidence Addendum\n\n> Post-finalization technical evidence record — no legal qualification asserted.\n\n## Certificate association\n\n- Certificate ID: `{}`\n- Timestamp record ID: `{}`\n- Imported at [System value]: {}\n\n## External Timestamp Evidence\n\n- Provider / issuer [User-confirmed fact]: {}\n- Timestamp type [User-confirmed fact]: {}\n- Timestamp value [User-confirmed fact]: {}\n- Referenced artifact [User-confirmed fact]: {}\n- Referenced artifact path [System value]: `{}`\n- Referenced SHA-256 [User-confirmed fact]: `{}`\n- Actual artifact SHA-256 [System verification]: `{}`\n- Referenced hash match [System verification]: **{}**\n- Timestamp evidence filename [Evidence-derived metadata]: {}\n- Timestamp evidence SHA-256 [System verification]: `{}`\n- External reference ID [User-confirmed fact]: {}\n- Provider verification URL [User-confirmed fact]: {}\n- Note [User-confirmed fact]: {}\n- Provenance [System value]: {}\n\n{}\n",
        md(&record.certificate_id),
        md(&record.id),
        md(&record.imported_at),
        documented_md(&record.provider),
        timestamp_type_label(record.timestamp_type),
        documented_md(&record.timestamp_value),
        referenced_artifact_label(record.referenced_artifact),
        md(&record.referenced_artifact_path),
        record.referenced_sha256,
        record.actual_sha256,
        match record.referenced_hash_match {
            Some(true) => "YES",
            Some(false) => "NO",
            None => "NOT VERIFIED",
        },
        documented_md(&record.evidence_file_name),
        record.evidence_sha256,
        documented_md(&record.external_reference_id),
        documented_md(&record.provider_verification_url),
        documented_md(&record.note),
        documented_md(&record.provenance),
        DISCLAIMER,
    )
}

fn md(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('\r', "")
        .replace('\n', "<br>")
}

fn documented_md(value: &str) -> String {
    if value.trim().is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        md(value)
    }
}

pub fn timestamp_type_label(value: TimestampType) -> &'static str {
    match value {
        TimestampType::QualifiedElectronicTimestampUserDeclared => {
            "Qualified electronic timestamp — user declared"
        }
        TimestampType::ElectronicTimestamp => "Electronic timestamp",
        TimestampType::ExternalIntegrityTimestamp => "External integrity timestamp",
        TimestampType::Other => "Other",
        TimestampType::NotDocumented => "NOT DOCUMENTED",
    }
}

pub fn referenced_artifact_label(value: TimestampReferencedArtifact) -> &'static str {
    match value {
        TimestampReferencedArtifact::EvidenceManifest => "EVIDENCE_MANIFEST.json",
        TimestampReferencedArtifact::Sha256sums => "SHA256SUMS.txt",
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            "DOCUMENTATION_CERTIFICATE.md"
        }
        TimestampReferencedArtifact::CertificatePdf => "Certificate PDF",
        TimestampReferencedArtifact::FinalEvidencePackage => "Final Evidence Package",
        TimestampReferencedArtifact::Other => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn qualified_type_is_explicitly_user_declared() {
        assert!(
            timestamp_type_label(TimestampType::QualifiedElectronicTimestampUserDeclared)
                .contains("user declared")
        );
    }

    #[test]
    fn markdown_keeps_no_and_not_documented_distinct() {
        assert_eq!(documented_md(""), "NOT DOCUMENTED");
        assert_eq!(documented_md("NO"), "NO");
    }

    #[test]
    fn verification_pins_published_bytes_and_never_requires_current_renderer_output() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let anchor = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&anchor, b"{\"historical\":true}\n").expect("manifest anchor");
        let source = track_root.join("timestamp.json");
        fs::write(&source, b"{\"provider\":\"fixture\"}\n").expect("timestamp source");
        let anchor_sha256 = sha256_file(&anchor).expect("anchor hash");
        let staged = stage(
            track_root,
            "CERT-RENDERER-INDEPENDENCE",
            &source,
            ExternalTimestampInput {
                provider: "Fixture Provider".into(),
                timestamp_type: TimestampType::ElectronicTimestamp,
                timestamp_value: "2026-08-17T16:00:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: anchor_sha256,
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: "renderer independence".into(),
            },
        )
        .expect("stage timestamp");
        let mut record = publish(track_root, &staged).expect("publish timestamp");
        let record_directory = track_root
            .join(&record.record_relative_path)
            .parent()
            .expect("record directory")
            .to_path_buf();

        let historical_markdown = b"# Historical addendum bytes\n\nThese bytes intentionally do not come from the current renderer.\n";
        assert_ne!(
            historical_markdown.as_slice(),
            render_markdown(&record).as_bytes()
        );
        fs::write(record_directory.join(MARKDOWN_FILE), historical_markdown)
            .expect("historical markdown bytes");
        record.markdown_sha256 =
            sha256_file(&record_directory.join(MARKDOWN_FILE)).expect("markdown hash");
        fs::write(
            record_directory.join(RECORD_FILE),
            immutable_record_bytes(&record).expect("immutable record bytes"),
        )
        .expect("updated immutable record fixture");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("artifact hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).expect("versioned hash list"),
        )
        .expect("updated hash list fixture");

        verify_published_record(track_root, &record)
            .expect("persisted historical bytes verify without re-rendering");
        let immutable: serde_json::Value = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("record bytes"),
        )
        .expect("record JSON");
        assert_eq!(
            immutable["integrityVerifiedAtPublication"].as_bool(),
            Some(true)
        );
        assert!(immutable.get("integrityVerified").is_none());
        assert!(fs::read_to_string(record_directory.join(HASH_LIST_FILE))
            .expect("hash list")
            .starts_with(HASH_LIST_V1_HEADER));

        // Even a self-consistent rewritten hash list cannot authorize extra
        // runtime/trust claims in the immutable v1 JSON record.
        let mut injected: serde_json::Value = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("immutable record bytes"),
        )
        .expect("immutable record JSON");
        let object = injected.as_object_mut().expect("record object");
        object.insert("integrityVerified".into(), serde_json::Value::Bool(true));
        object.insert(
            "providerQualificationVerifiedBySunoDM".into(),
            serde_json::Value::Bool(true),
        );
        fs::write(
            record_directory.join(RECORD_FILE),
            serde_json::to_vec_pretty(&injected).expect("injected JSON bytes"),
        )
        .expect("injected record");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("injected hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).expect("injected hash list"),
        )
        .expect("self-consistent injected hash list");
        let error = verify_published_record(track_root, &record)
            .expect_err("injected immutable claims must fail verification");
        assert!(error.to_string().contains("exact immutable"));
    }

    #[test]
    fn legacy_v0_sidecars_remain_self_consistently_verifiable_without_rendering() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let anchor = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&anchor, b"legacy anchor").expect("manifest anchor");
        let source = track_root.join("legacy-timestamp.json");
        fs::write(&source, b"legacy timestamp evidence").expect("timestamp source");
        let staged = stage(
            track_root,
            "CERT-LEGACY-V0",
            &source,
            ExternalTimestampInput {
                provider: "Legacy Provider".into(),
                timestamp_type: TimestampType::ElectronicTimestamp,
                timestamp_value: String::new(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: sha256_file(&anchor).expect("anchor hash"),
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: String::new(),
            },
        )
        .expect("stage timestamp");
        let current = publish(track_root, &staged).expect("publish timestamp");
        let record_directory = track_root
            .join(&current.record_relative_path)
            .parent()
            .expect("record directory")
            .to_path_buf();

        let mut legacy_value = serde_json::to_value(&current).expect("legacy record value");
        let legacy_object = legacy_value.as_object_mut().expect("record object");
        legacy_object.remove("sidecarFormatVersion");
        legacy_object.remove("markdownSha256");
        legacy_object.remove("pdfSha256");
        legacy_object.remove("integrityVerifiedAtPublication");
        let legacy_bytes = serde_json::to_vec_pretty(&legacy_value).expect("legacy bytes");
        fs::write(record_directory.join(RECORD_FILE), legacy_bytes).expect("legacy record");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("legacy hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(0, &hashes).expect("legacy hash list"),
        )
        .expect("legacy hash list fixture");
        let registered: ExternalTimestampRecord = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("legacy record bytes"),
        )
        .expect("deserialize legacy record");

        verify_published_record(track_root, &registered)
            .expect("legacy sidecar self-consistency verifies without renderer equality");
    }
}
