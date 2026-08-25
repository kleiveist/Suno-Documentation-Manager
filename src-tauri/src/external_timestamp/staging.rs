use super::*;

#[derive(Debug)]
pub struct StagedExternalTimestamp {
    pub record: ExternalTimestampRecord,
    pub(super) stage_relative: PathBuf,
    pub(super) live_relative: PathBuf,
}

/// Build and durably stage a complete timestamp sidecar. The caller must
/// register `record` in SQLite before calling [`publish`], so a process exit can
/// never leave a new live sidecar that is invisible to the database.
#[cfg(test)]
pub fn stage(
    track_root: &Path,
    certificate_id: &str,
    source: &Path,
    input: ExternalTimestampInput,
) -> Result<StagedExternalTimestamp> {
    stage_with_provider_metadata(track_root, certificate_id, source, input, None, None)
}

/// Stage an immutable provider response that SunoDM itself requested. All
/// values are derived from the configured adapter and its raw response; the
/// caller supplies only the already-selected finalized anchor identity.
pub fn stage_provider_response(
    track_root: &Path,
    certificate_id: &str,
    referenced_revision_id: &str,
    referenced_sha256: &str,
    source: &Path,
    response: ProviderTimestampResponse,
) -> Result<StagedExternalTimestamp> {
    let mut metadata = response.metadata;
    metadata.referenced_revision_id = referenced_revision_id.to_owned();
    let raw_provider_response = response.raw_provider_response;
    let input = ExternalTimestampInput {
        provider: response.provider,
        timestamp_type: TimestampType::ExternalIntegrityTimestamp,
        timestamp_value: response.timestamp_value,
        referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
        other_referenced_artifact: String::new(),
        referenced_sha256: referenced_sha256.to_owned(),
        external_reference_id: response.external_reference_id,
        provider_verification_url: response.provider_verification_url,
        note: response.note,
    };
    stage_with_provider_metadata(
        track_root,
        certificate_id,
        source,
        input,
        Some(metadata),
        raw_provider_response.as_ref(),
    )
}

pub(super) fn stage_with_provider_metadata(
    track_root: &Path,
    certificate_id: &str,
    source: &Path,
    input: ExternalTimestampInput,
    provider_metadata: Option<TimestampProviderMetadata>,
    raw_provider_response: Option<&ProviderRawResponse>,
) -> Result<StagedExternalTimestamp> {
    validate_input(&input)?;
    if raw_provider_response.is_some() && provider_metadata.is_none() {
        return Err(AppError::Validation(
            "A raw provider response archive requires provider-derived metadata.".into(),
        ));
    }
    let evidence_file_name = validated_evidence_file_name(source)?;
    let anchor = validated_referenced_anchor(track_root, &input, provider_metadata.is_some())?;
    let directories = prepare_stage_directories(track_root)?;

    let managed_evidence_name = managed_evidence_name(source);
    let live_record_relative = directories.live_relative.join(RECORD_FILE);
    let live_markdown_relative = directories.live_relative.join(MARKDOWN_FILE);
    let live_pdf_relative = directories.live_relative.join(PDF_FILE);
    let live_hash_list_relative = directories.live_relative.join(HASH_LIST_FILE);
    let imported_at = Utc::now().to_rfc3339();
    let raw_provider_response_name = raw_provider_response
        .map(provider_response_artifact_name)
        .transpose()?;
    let mut provider_metadata = provider_metadata;
    let cleanup_directory = directories.stage_directory.clone();

    let staging = write_staged_timestamp(StageWriteContext {
        track_root,
        certificate_id,
        source,
        input,
        provider_metadata: provider_metadata.take(),
        raw_provider_response,
        evidence_file_name,
        referenced_relative: anchor.relative,
        actual_sha256: anchor.actual_sha256,
        referenced_sha256: anchor.referenced_sha256,
        referenced_hash_match: anchor.hash_match,
        id: directories.id,
        live_relative: directories.live_relative,
        stage_relative: directories.stage_relative,
        stage_directory: directories.stage_directory.clone(),
        managed_evidence_name,
        live_record_relative,
        live_markdown_relative,
        live_pdf_relative,
        live_hash_list_relative,
        imported_at,
        raw_provider_response_name,
    });

    if staging.is_err() && cleanup_directory.exists() {
        let _ = fs::remove_dir_all(&cleanup_directory);
    }
    staging
}

pub(super) fn validated_evidence_file_name(source: &Path) -> Result<String> {
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
    Ok(evidence_file_name)
}

pub(super) struct ReferencedAnchor {
    relative: PathBuf,
    actual_sha256: String,
    referenced_sha256: String,
    hash_match: Option<bool>,
}

pub(super) fn validated_referenced_anchor(
    track_root: &Path,
    input: &ExternalTimestampInput,
    provider_metadata_available: bool,
) -> Result<ReferencedAnchor> {
    let referenced_relative = referenced_artifact_path(input)?;
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
    if provider_metadata_available && referenced_hash_match != Some(true) {
        return Err(AppError::Validation(
            "The finalized manifest anchor changed before the provider response could be staged."
                .into(),
        ));
    }
    Ok(ReferencedAnchor {
        relative: referenced_relative,
        actual_sha256,
        referenced_sha256,
        hash_match: referenced_hash_match,
    })
}

pub(super) fn managed_evidence_name(source: &Path) -> String {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    format!("TIMESTAMP_EVIDENCE.{extension}")
}

pub(super) struct StageDirectories {
    id: String,
    live_relative: PathBuf,
    stage_relative: PathBuf,
    stage_directory: PathBuf,
}

pub(super) fn prepare_stage_directories(track_root: &Path) -> Result<StageDirectories> {
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
    Ok(StageDirectories {
        id,
        live_relative,
        stage_relative,
        stage_directory,
    })
}

pub(super) struct StageWriteContext<'a> {
    track_root: &'a Path,
    certificate_id: &'a str,
    source: &'a Path,
    input: ExternalTimestampInput,
    provider_metadata: Option<TimestampProviderMetadata>,
    raw_provider_response: Option<&'a ProviderRawResponse>,
    evidence_file_name: String,
    referenced_relative: PathBuf,
    actual_sha256: String,
    referenced_sha256: String,
    referenced_hash_match: Option<bool>,
    id: String,
    live_relative: PathBuf,
    stage_relative: PathBuf,
    stage_directory: PathBuf,
    managed_evidence_name: String,
    live_record_relative: PathBuf,
    live_markdown_relative: PathBuf,
    live_pdf_relative: PathBuf,
    live_hash_list_relative: PathBuf,
    imported_at: String,
    raw_provider_response_name: Option<String>,
}

pub(super) fn write_staged_timestamp(
    mut context: StageWriteContext<'_>,
) -> Result<StagedExternalTimestamp> {
    let evidence_path = context.stage_directory.join(&context.managed_evidence_name);
    let (evidence_sha256, _) = copy_new_hashed(context.source, &evidence_path)?;
    let provider_response_sha256 = archive_provider_response(&context, &evidence_sha256)?;
    if let Some(metadata) = context.provider_metadata.as_mut() {
        metadata.provider_response_file_name = context
            .raw_provider_response_name
            .clone()
            .unwrap_or_else(|| context.managed_evidence_name.clone());
        metadata.provider_response_sha256 = provider_response_sha256;
    }
    let record = staged_record(&context, evidence_sha256);
    let record = write_staged_artifacts(&context, record)?;
    Ok(StagedExternalTimestamp {
        record,
        stage_relative: context.stage_relative,
        live_relative: context.live_relative,
    })
}

pub(super) fn archive_provider_response(
    context: &StageWriteContext<'_>,
    evidence_sha256: &str,
) -> Result<String> {
    let Some(raw_provider_response) = context.raw_provider_response else {
        return Ok(evidence_sha256.to_owned());
    };
    let raw_name = context
        .raw_provider_response_name
        .as_deref()
        .ok_or_else(|| AppError::Data("Raw provider response archive name is missing.".into()))?;
    let raw_path = context.stage_directory.join(raw_name);
    atomic_write_new(&raw_path, &raw_provider_response.bytes)?;
    sha256_file(&raw_path)
}

pub(super) fn staged_record(
    context: &StageWriteContext<'_>,
    evidence_sha256: String,
) -> ExternalTimestampRecord {
    let automatic_record = context.provider_metadata.is_some();
    ExternalTimestampRecord {
        id: context.id.clone(),
        certificate_id: context.certificate_id.to_owned(),
        sidecar_format_version: SIDECAR_FORMAT_VERSION,
        provider: context.input.provider.trim().to_owned(),
        timestamp_type: context.input.timestamp_type,
        timestamp_value: context.input.timestamp_value.trim().to_owned(),
        referenced_artifact: context.input.referenced_artifact,
        referenced_artifact_path: portable_relative(&context.referenced_relative),
        referenced_sha256: context.referenced_sha256.clone(),
        actual_sha256: context.actual_sha256.clone(),
        referenced_hash_match: context.referenced_hash_match,
        external_reference_id: context.input.external_reference_id.trim().to_owned(),
        provider_verification_url: context.input.provider_verification_url.trim().to_owned(),
        note: context.input.note.trim().to_owned(),
        evidence_file_name: context.evidence_file_name.clone(),
        evidence_sha256,
        markdown_sha256: String::new(),
        pdf_sha256: String::new(),
        imported_at: context.imported_at.clone(),
        provenance: if automatic_record {
            "Provider-derived metadata; managed provider response; system-verified finalized anchor comparison"
                .into()
        } else {
            "Managed copy; user-confirmed metadata; system-verified SHA-256 comparison".into()
        },
        provider_metadata: context.provider_metadata.clone(),
        record_relative_path: portable_relative(&context.live_record_relative),
        markdown_relative_path: portable_relative(&context.live_markdown_relative),
        pdf_relative_path: portable_relative(&context.live_pdf_relative),
        hash_list_relative_path: portable_relative(&context.live_hash_list_relative),
        integrity_verified_at_publication: true,
        integrity_verified: true,
        integrity_issues: Vec::new(),
    }
}

pub(super) fn write_staged_artifacts(
    context: &StageWriteContext<'_>,
    mut record: ExternalTimestampRecord,
) -> Result<ExternalTimestampRecord> {
    let markdown = render_markdown(&record);
    atomic_write_new(
        &context.stage_directory.join(MARKDOWN_FILE),
        markdown.as_bytes(),
    )?;
    let pdf = render_pdf(&record)?;
    atomic_write_new(&context.stage_directory.join(PDF_FILE), &pdf)?;
    record.markdown_sha256 = sha256_file(&context.stage_directory.join(MARKDOWN_FILE))?;
    record.pdf_sha256 = sha256_file(&context.stage_directory.join(PDF_FILE))?;
    let record_bytes = immutable_record_bytes(&record)?;
    atomic_write_new(&context.stage_directory.join(RECORD_FILE), &record_bytes)?;

    let hashes = artifact_hashes_with_provider_response(
        &context.stage_directory,
        &context.managed_evidence_name,
        context.raw_provider_response_name.as_deref(),
    )?;
    let hash_list = render_hash_list(record.sidecar_format_version, &hashes)?;
    atomic_write_new(
        &context.stage_directory.join(HASH_LIST_FILE),
        hash_list.as_bytes(),
    )?;
    verify_staged_hashes(&context.stage_directory, &hashes)?;
    verify_record_in_directory(context.track_root, &context.stage_directory, &record, None)?;
    // The database row is written only after `stage` returns. Sync both the
    // completed directory contents and its parent entry so that a crash can
    // leave either a recoverable complete stage or no registered record.
    sync_directory(&context.stage_directory)?;
    if let Some(parent) = context.stage_directory.parent() {
        sync_directory(parent)?;
    }
    Ok(record)
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
