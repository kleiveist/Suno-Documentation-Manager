use super::*;

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

pub(super) fn verify_record_in_directory(
    track_root: &Path,
    directory: &Path,
    record: &ExternalTimestampRecord,
    revision_root: Option<&Path>,
) -> Result<()> {
    let names = sidecar_artifact_names(record)?;
    verify_sidecar_file_set(directory, &names)?;
    let stored_record = verified_immutable_record(directory, record)?;
    verify_sidecar_artifact_integrity(directory, record, &stored_record, &names)?;
    verify_referenced_record_artifact(track_root, revision_root, record)
}

struct SidecarArtifactNames {
    managed_evidence: String,
    provider_response: Option<String>,
}

fn sidecar_artifact_names(record: &ExternalTimestampRecord) -> Result<SidecarArtifactNames> {
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
    let provider_response_name = provider_response_name_for_record(record, &managed_evidence_name)?;
    Ok(SidecarArtifactNames {
        managed_evidence: managed_evidence_name,
        provider_response: provider_response_name,
    })
}

fn verify_sidecar_file_set(directory: &Path, names: &SidecarArtifactNames) -> Result<()> {
    let mut expected_names = BTreeSet::from([
        RECORD_FILE.to_owned(),
        names.managed_evidence.clone(),
        MARKDOWN_FILE.to_owned(),
        PDF_FILE.to_owned(),
        HASH_LIST_FILE.to_owned(),
    ]);
    if let Some(provider_response_name) = &names.provider_response {
        if provider_response_name != &names.managed_evidence {
            expected_names.insert(provider_response_name.clone());
        }
    }
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(directory).map_err(|error| AppError::io(directory, error))? {
        let entry = entry.map_err(|error| AppError::io(directory, error))?;
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
    Ok(())
}

fn verified_immutable_record(
    directory: &Path,
    record: &ExternalTimestampRecord,
) -> Result<ExternalTimestampRecord> {
    let record_path = directory.join(RECORD_FILE);
    let record_bytes = fs::read(&record_path).map_err(|error| AppError::io(&record_path, error))?;
    let stored_record: ExternalTimestampRecord = serde_json::from_slice(&record_bytes)?;
    verify_immutable_record_version(&stored_record, record, &record_bytes)?;
    Ok(stored_record)
}

fn verify_immutable_record_version(
    stored_record: &ExternalTimestampRecord,
    record: &ExternalTimestampRecord,
    record_bytes: &[u8],
) -> Result<()> {
    match stored_record.sidecar_format_version {
        0 if !stored_record.integrity_verified => {
            return Err(AppError::Validation(
                "Legacy timestamp sidecar has no publication-time integrity assertion.".into(),
            ));
        }
        0 if !immutable_records_match(stored_record, record) => {
            return Err(AppError::Validation(
                "Legacy TIMESTAMP_RECORD.json differs from the registered timestamp record.".into(),
            ));
        }
        0 => {}
        LEGACY_SIDECAR_FORMAT_VERSION | SIDECAR_FORMAT_VERSION
            if !stored_record.integrity_verified_at_publication =>
        {
            return Err(AppError::Validation(
                "Timestamp sidecar does not record successful publication-time integrity verification."
                    .into(),
            ));
        }
        LEGACY_SIDECAR_FORMAT_VERSION | SIDECAR_FORMAT_VERSION => {
            // Versioned sidecars have one canonical immutable JSON
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
    Ok(())
}

fn verify_sidecar_artifact_integrity(
    directory: &Path,
    record: &ExternalTimestampRecord,
    stored_record: &ExternalTimestampRecord,
    names: &SidecarArtifactNames,
) -> Result<()> {
    // Verify the exact immutable bytes that were published. Do not re-render
    // Markdown or PDF: renderer changes must not invalidate historical records.
    let hashes = artifact_hashes_with_provider_response(
        directory,
        &names.managed_evidence,
        names.provider_response.as_deref(),
    )?;
    verify_evidence_hash(record, &names.managed_evidence, &hashes)?;
    verify_provider_response_hash(record, names.provider_response.as_deref(), &hashes)?;
    verify_sidecar_hash_list(directory, stored_record.sidecar_format_version, &hashes)?;
    verify_rendered_artifact_hashes(stored_record, &hashes)
}

fn verify_evidence_hash(
    record: &ExternalTimestampRecord,
    managed_evidence_name: &str,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    let evidence_sha256 = hashes
        .get(managed_evidence_name)
        .ok_or_else(|| AppError::Data("External timestamp evidence hash is missing.".into()))?;
    if evidence_sha256 != &record.evidence_sha256 {
        return Err(AppError::Validation(
            "External timestamp evidence SHA-256 no longer matches its registered value.".into(),
        ));
    }
    Ok(())
}

fn verify_provider_response_hash(
    record: &ExternalTimestampRecord,
    provider_response_name: Option<&str>,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    if let (Some(metadata), Some(provider_response_name)) =
        (&record.provider_metadata, provider_response_name)
    {
        let provider_response_sha256 = hashes
            .get(provider_response_name)
            .ok_or_else(|| AppError::Data("Provider response archive hash is missing.".into()))?;
        if provider_response_sha256 != &metadata.provider_response_sha256 {
            return Err(AppError::Validation(
                "Provider response archive SHA-256 no longer matches its immutable metadata."
                    .into(),
            ));
        }
    }
    Ok(())
}

fn verify_sidecar_hash_list(
    directory: &Path,
    sidecar_format_version: u32,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    let expected_hash_list = render_hash_list(sidecar_format_version, hashes)?;
    let hash_list_path = directory.join(HASH_LIST_FILE);
    let hash_list =
        fs::read(&hash_list_path).map_err(|error| AppError::io(&hash_list_path, error))?;
    let previous_v2_hash_list = (sidecar_format_version == SIDECAR_FORMAT_VERSION)
        .then(|| render_hash_list_with_header(HASH_LIST_V2_HEADER, hashes));
    if hash_list != expected_hash_list.as_bytes()
        && previous_v2_hash_list
            .as_ref()
            .is_none_or(|legacy| hash_list != legacy.as_bytes())
    {
        return Err(AppError::Validation(
            "Timestamp sidecar SHA-256 list is incomplete or no longer matches.".into(),
        ));
    }
    Ok(())
}

fn verify_rendered_artifact_hashes(
    stored_record: &ExternalTimestampRecord,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    if matches!(
        stored_record.sidecar_format_version,
        LEGACY_SIDECAR_FORMAT_VERSION | SIDECAR_FORMAT_VERSION
    ) {
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
    Ok(())
}

fn verify_referenced_record_artifact(
    track_root: &Path,
    revision_root: Option<&Path>,
    record: &ExternalTimestampRecord,
) -> Result<()> {
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
pub(super) struct PublishedRecordLocation {
    pub(super) directory: PathBuf,
    pub(super) revision_root: Option<PathBuf>,
}

pub(super) fn resolve_published_record(
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

    let mut matches = archived_record_locations(track_root, record)?;
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => missing_published_record(track_root, record),
        _ => Err(AppError::Validation(format!(
            "External timestamp sidecar {} exists in multiple revision archives.",
            record.id
        ))),
    }
}

fn archived_record_locations(
    track_root: &Path,
    record: &ExternalTimestampRecord,
) -> Result<Vec<PublishedRecordLocation>> {
    let revisions_relative = Path::new(".archive/revisions");
    let revisions = contained_path(track_root, revisions_relative, false)?;
    let mut matches = Vec::new();
    if !revisions.is_dir() {
        return Ok(matches);
    }
    let nested_timestamp_directory = Path::new(EXTERNAL_TIMESTAMPS_DIR)
        .strip_prefix(certificate::CERTIFICATE_DIR)
        .map_err(|_| AppError::PathEscape)?;
    for entry in fs::read_dir(&revisions).map_err(|error| AppError::io(&revisions, error))? {
        let entry = entry.map_err(|error| AppError::io(&revisions, error))?;
        if let Some(location) =
            archived_record_location(track_root, entry, nested_timestamp_directory, record)?
        {
            matches.push(location);
        }
    }
    Ok(matches)
}

fn archived_record_location(
    track_root: &Path,
    entry: fs::DirEntry,
    nested_timestamp_directory: &Path,
    record: &ExternalTimestampRecord,
) -> Result<Option<PublishedRecordLocation>> {
    let entry_path = entry.path();
    let metadata =
        fs::symlink_metadata(&entry_path).map_err(|error| AppError::io(&entry_path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(None);
    }
    let candidate = entry_path
        .join("certificate")
        .join(nested_timestamp_directory)
        .join(&record.id);
    if !candidate.is_dir() {
        return Ok(None);
    }
    verify_revision_metadata(track_root, &entry_path, record)?;
    let candidate_relative = candidate
        .strip_prefix(track_root)
        .map_err(|_| AppError::PathEscape)?;
    let candidate = contained_path(track_root, candidate_relative, true)?;
    Ok(Some(PublishedRecordLocation {
        directory: candidate,
        revision_root: Some(entry_path),
    }))
}

fn verify_revision_metadata(
    track_root: &Path,
    entry_path: &Path,
    record: &ExternalTimestampRecord,
) -> Result<()> {
    let revision_metadata = entry_path.join("revision.json");
    let revision_metadata_relative = revision_metadata
        .strip_prefix(track_root)
        .map_err(|_| AppError::PathEscape)?;
    let revision_metadata = contained_path(track_root, revision_metadata_relative, true)?;
    let metadata = fs::symlink_metadata(&revision_metadata)
        .map_err(|error| AppError::io(&revision_metadata, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Validation(format!(
            "Revision metadata for external timestamp {} is not a regular file.",
            record.id
        )));
    }
    let revision: serde_json::Value = serde_json::from_slice(
        &fs::read(&revision_metadata).map_err(|error| AppError::io(&revision_metadata, error))?,
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
    Ok(())
}

fn missing_published_record(
    track_root: &Path,
    record: &ExternalTimestampRecord,
) -> Result<PublishedRecordLocation> {
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

/// Reconcile the two durable states used by phase-two publication. A registered
/// database row makes its matching stage recoverable; an unregistered stage is
/// an uncommitted operation and is removed. An unexpected live sidecar is never
/// silently adopted as a user-confirmed database fact.
pub fn reconcile_publications(
    track_root: &Path,
    registered: &[ExternalTimestampRecord],
) -> Result<bool> {
    let registered_ids = registered
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let recovered = recover_registered_publications(track_root, registered)?;
    verify_live_publication_directory(track_root, &registered_ids)?;
    reconcile_staging_directory(track_root, &registered_ids, recovered)
}

fn recover_registered_publications(
    track_root: &Path,
    registered: &[ExternalTimestampRecord],
) -> Result<bool> {
    let mut recovered = false;
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
    Ok(recovered)
}

fn verify_live_publication_directory(
    track_root: &Path,
    registered_ids: &BTreeSet<&str>,
) -> Result<()> {
    let live_parent = contained_path(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR), false)?;
    if live_parent.is_dir() {
        for entry in
            fs::read_dir(&live_parent).map_err(|error| AppError::io(&live_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&live_parent, error))?;
            let id = live_publication_id(entry)?;
            if !registered_ids.contains(id.as_str()) {
                return Err(AppError::Data(format!(
                    "Unregistered external timestamp sidecar detected: {id}. It was not adopted automatically."
                )));
            }
        }
    }
    Ok(())
}

fn live_publication_id(entry: fs::DirEntry) -> Result<String> {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
    let file_name = entry.file_name();
    let id = file_name
        .to_str()
        .ok_or_else(|| AppError::Data("External timestamp directory name is not UTF-8.".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || Uuid::parse_str(id).is_err() {
        return Err(AppError::Data(format!(
            "Unexpected entry in the external timestamp publication directory: {}.",
            path.display()
        )));
    }
    Ok(id.to_owned())
}

fn reconcile_staging_directory(
    track_root: &Path,
    registered_ids: &BTreeSet<&str>,
    mut recovered: bool,
) -> Result<bool> {
    let staging_parent = contained_path(track_root, Path::new(STAGING_DIR), false)?;
    if staging_parent.is_dir() {
        for entry in
            fs::read_dir(&staging_parent).map_err(|error| AppError::io(&staging_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&staging_parent, error))?;
            let (id, path) = staging_publication(entry)?;
            if registered_ids.contains(id.as_str()) {
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

fn staging_publication(entry: fs::DirEntry) -> Result<(String, PathBuf)> {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
    let file_name = entry.file_name();
    let id = file_name
        .to_str()
        .ok_or_else(|| AppError::Data("Timestamp staging directory name is not UTF-8.".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || Uuid::parse_str(id).is_err() {
        return Err(AppError::Data(format!(
            "Unexpected entry in the timestamp staging directory: {}.",
            path.display()
        )));
    }
    Ok((id.to_owned(), path))
}

pub(super) fn immutable_record_bytes(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    let mut value = serde_json::to_value(record)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::Data("Timestamp record serialization is not an object.".into()))?;
    object.remove("integrityVerified");
    object.remove("integrityIssues");
    Ok(serde_json::to_vec_pretty(&value)?)
}

pub(super) fn immutable_records_match(
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

pub(super) fn provider_response_artifact_name(
    raw_provider_response: &ProviderRawResponse,
) -> Result<String> {
    let extension = raw_provider_response.extension.trim().to_ascii_lowercase();
    if extension.is_empty()
        || extension.len() > 16
        || !extension
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "Provider response archive extension is invalid.".into(),
        ));
    }
    Ok(format!("{PROVIDER_RESPONSE_FILE_PREFIX}.{extension}"))
}

/// Resolve the optional extra raw-provider artifact recorded in a modern
/// automatic sidecar. Empty metadata remains valid for older automatic
/// records created before raw-response archive fields existed.
pub(super) fn provider_response_name_for_record(
    record: &ExternalTimestampRecord,
    managed_evidence_name: &str,
) -> Result<Option<String>> {
    let Some(metadata) = &record.provider_metadata else {
        return Ok(None);
    };
    let name = metadata.provider_response_file_name.trim();
    let digest = metadata.provider_response_sha256.trim();
    if name.is_empty() && digest.is_empty() {
        return Ok(None);
    }
    if name.is_empty() || digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::Validation(
            "Provider response archive metadata is incomplete or invalid.".into(),
        ));
    }
    if name == managed_evidence_name {
        return Ok(Some(name.to_owned()));
    }
    let extension = name
        .strip_prefix(&format!("{PROVIDER_RESPONSE_FILE_PREFIX}."))
        .ok_or_else(|| {
            AppError::Validation(
                "Provider response archive has an unexpected managed filename.".into(),
            )
        })?;
    if extension.is_empty()
        || extension.len() > 16
        || !extension
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "Provider response archive filename is invalid.".into(),
        ));
    }
    Ok(Some(name.to_owned()))
}

#[cfg(test)]
pub(super) fn artifact_hashes(
    directory: &Path,
    evidence_name: &str,
) -> Result<BTreeMap<String, String>> {
    artifact_hashes_with_provider_response(directory, evidence_name, None)
}

pub(super) fn artifact_hashes_with_provider_response(
    directory: &Path,
    evidence_name: &str,
    provider_response_name: Option<&str>,
) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for name in [RECORD_FILE, evidence_name, MARKDOWN_FILE, PDF_FILE] {
        hashes.insert(name.to_owned(), sha256_file(&directory.join(name))?);
    }
    if let Some(provider_response_name) = provider_response_name {
        if provider_response_name != evidence_name {
            hashes.insert(
                provider_response_name.to_owned(),
                sha256_file(&directory.join(provider_response_name))?,
            );
        }
    }
    Ok(hashes)
}

pub(super) fn render_hash_list(version: u32, hashes: &BTreeMap<String, String>) -> Result<String> {
    let header = match version {
        0 => "",
        // The hash-list byte contract has remained v1 even though the
        // immutable JSON sidecar gained qualification-audit fields in v2.
        // Keep accepting the briefly emitted v2-labelled header above so
        // already published local records remain verifiable.
        LEGACY_SIDECAR_FORMAT_VERSION | SIDECAR_FORMAT_VERSION => HASH_LIST_V1_HEADER,
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported external timestamp sidecar format version: {other}."
            )));
        }
    };
    Ok(render_hash_list_with_header(header, hashes))
}

pub(super) fn render_hash_list_with_header(
    header: &str,
    hashes: &BTreeMap<String, String>,
) -> String {
    let mut output = header.to_owned();
    for (name, digest) in hashes {
        output.push_str(&format!("{digest}  {name}\n"));
    }
    output
}

pub(super) fn verify_referenced_artifact(
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
pub(super) fn sync_directory(directory: &Path) -> Result<()> {
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| AppError::io(directory, error))
}

#[cfg(not(unix))]
pub(super) fn sync_directory(_directory: &Path) -> Result<()> {
    // Opening a directory for fsync is not portable (notably on Windows).
    // The files themselves are still atomically written and synced before the
    // rename; directory durability remains a best-effort platform boundary.
    Ok(())
}
