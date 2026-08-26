use super::*;

/// Writes local JSON, its detached SHA-256, and the human-readable technical
/// summary.  The local JSON omits `artifactSha256` to avoid a self-hash cycle;
/// the exact digest is placed in the detached file and in the track state.
pub fn publish_local_screening_artifacts(
    track_root: &Path,
    record: &mut AudioScreeningLocalRecord,
    external: Option<&AudioScreeningExternalRecord>,
) -> Result<()> {
    ensure_screening_directory(track_root)?;
    let loaded_external = match external {
        Some(record) => Some(record.clone()),
        None => read_existing_external_record(track_root)?,
    };
    let current_external = loaded_external.filter(|external| {
        external.status != AudioScreeningStatus::Stale
            && external.source_evidence_id == record.source_evidence_id
            && external.source_relative_path == record.source_relative_path
            && external.source_sha256 == record.source_sha256
            && external.source_size_bytes == record.source_size_bytes
    });
    let mut replacements = vec![
        LOCAL_FINGERPRINT_FILE,
        LOCAL_FINGERPRINT_HASH_FILE,
        AUDIO_SCREENING_MARKDOWN_FILE,
    ];
    if current_external.is_none() {
        replacements.push(EXTERNAL_SCREENING_FILE);
        replacements.push(ACRCLOUD_RESPONSE_FILE);
    }
    archive_managed_artifacts(track_root, &replacements)?;
    record.artifact_relative_path = LOCAL_FINGERPRINT_FILE.into();
    record.artifact_sha256.clear();
    let mut value = serde_json::to_value(&*record)?;
    if let Some(object) = value.as_object_mut() {
        object.remove("artifactSha256");
    }
    let bytes = serde_json::to_vec_pretty(&value)?;
    let digest = sha256_bytes(&bytes);
    write_managed(track_root, LOCAL_FINGERPRINT_FILE, &bytes)?;
    write_managed(
        track_root,
        LOCAL_FINGERPRINT_HASH_FILE,
        format!("{digest}  LOCAL_FINGERPRINT.json\n").as_bytes(),
    )?;
    record.artifact_sha256 = digest;
    publish_screening_markdown(track_root, Some(record), current_external.as_ref())
}

/// Writes the structured external result, optionally the sanitized JSON
/// response, and refreshes the portable technical summary.  An old raw
/// response is moved below `.archive` before a newer result replaces it.
pub(super) fn publish_external_screening_artifacts(
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    record: &mut AudioScreeningExternalRecord,
    responses: Vec<PendingProviderResponse>,
) -> Result<()> {
    // Do this before `archive_managed_artifacts`: a malformed in-memory
    // record must leave the currently published directory untouched.
    if !external_matches_have_finite_scores(record) {
        return Err(AppError::Validation(
            "The external audio-screening record contains a non-finite provider score.".into(),
        ));
    }
    let response_archive = build_provider_response_archive(record, responses)?;
    if !record.samples.is_empty() {
        // Defensive archive validation can turn a formerly successful sample
        // into a controlled failure. Recompute the aggregate so `NO MATCH`
        // is never reported if a different submitted sample did not finish.
        finalize_external_sample_statistics(record);
    }
    ensure_screening_directory(track_root)?;
    archive_managed_artifacts(
        track_root,
        &[
            EXTERNAL_SCREENING_FILE,
            ACRCLOUD_RESPONSE_FILE,
            AUDIO_SCREENING_MARKDOWN_FILE,
        ],
    )?;
    if let Some((response, sequences)) = response_archive {
        write_managed(track_root, ACRCLOUD_RESPONSE_FILE, &response)?;
        let digest = sha256_bytes(&response);
        record.response_relative_path = Some(ACRCLOUD_RESPONSE_FILE.into());
        record.response_sha256 = Some(digest.clone());
        for sample in &mut record.samples {
            if sequences.contains(&sample.sequence) {
                sample.response_relative_path = Some(ACRCLOUD_RESPONSE_FILE.into());
                sample.response_sha256 = Some(digest.clone());
            }
        }
    } else {
        // A current result without a provider response must not accidentally
        // present an older response as current documentation.
        record.response_relative_path = None;
        record.response_sha256 = None;
        for sample in &mut record.samples {
            sample.response_relative_path = None;
            sample.response_sha256 = None;
        }
    }
    write_managed(
        track_root,
        EXTERNAL_SCREENING_FILE,
        &serde_json::to_vec_pretty(&*record)?,
    )?;
    publish_screening_markdown(track_root, local, Some(record))
}

pub(super) const MAX_ARCHIVED_PROVIDER_RESPONSES_BYTES: usize =
    // Pretty-printing a valid JSON response can add structural whitespace,
    // so reserve headroom above the sum of the independently bounded raw
    // responses. The 25-request cap still keeps this archive finite.
    MAX_PROVIDER_RESPONSE_BYTES * MAX_ACRCLOUD_REQUESTS as usize * 2 + 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ArchivedAcrCloudResponses<'a> {
    schema_version: u32,
    provider: &'a str,
    source_sha256: &'a str,
    checked_at: Option<&'a str>,
    samples: Vec<ArchivedAcrCloudResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ArchivedAcrCloudResponse {
    sequence: u32,
    offset_milliseconds: u64,
    end_offset_milliseconds: u64,
    duration_milliseconds: u64,
    status: AudioScreeningStatus,
    response: Value,
}

/// Archives all safe provider payloads in one structured JSON document. Each
/// entry repeats its deterministic sample coordinates, so a response is never
/// ambiguous even though the document itself has one detached SHA-256 anchor.
pub(super) fn build_provider_response_archive(
    record: &mut AudioScreeningExternalRecord,
    responses: Vec<PendingProviderResponse>,
) -> Result<Option<(Vec<u8>, Vec<u32>)>> {
    let mut archived = Vec::new();
    let mut sequences = Vec::new();
    let mut raw_bytes = 0_usize;
    for pending in responses {
        let response = pending.raw_response.as_bytes();
        raw_bytes = raw_bytes.saturating_add(response.len());
        if response.len() > MAX_PROVIDER_RESPONSE_BYTES
            || raw_bytes > MAX_ARCHIVED_PROVIDER_RESPONSES_BYTES
        {
            mark_response_archive_unsafe(record, pending.sequence);
            continue;
        }
        let value: Value = match serde_json::from_slice(response) {
            Ok(value) if !response_contains_credential_like_field(&value) => value,
            _ => {
                mark_response_archive_unsafe(record, pending.sequence);
                continue;
            }
        };
        sequences.push(pending.sequence);
        archived.push(ArchivedAcrCloudResponse {
            sequence: pending.sequence,
            offset_milliseconds: pending.offset_milliseconds,
            end_offset_milliseconds: pending.end_offset_milliseconds,
            duration_milliseconds: pending.duration_milliseconds,
            status: pending.status,
            response: value,
        });
    }
    if archived.is_empty() {
        return Ok(None);
    }
    let archive = ArchivedAcrCloudResponses {
        schema_version: 1,
        provider: &record.provider,
        source_sha256: &record.source_sha256,
        checked_at: record.checked_at.as_deref(),
        samples: archived,
    };
    let bytes = serde_json::to_vec_pretty(&archive)?;
    if bytes.len() > MAX_ARCHIVED_PROVIDER_RESPONSES_BYTES {
        return Err(AppError::Validation(
            "The combined ACRCloud response archive exceeds the supported size limit.".into(),
        ));
    }
    Ok(Some((bytes, sequences)))
}

pub(super) fn mark_response_archive_unsafe(
    record: &mut AudioScreeningExternalRecord,
    sequence: u32,
) {
    if let Some(sample) = record
        .samples
        .iter_mut()
        .find(|sample| sample.sequence == sequence)
    {
        sample.status = AudioScreeningStatus::ProcessingFailed;
        sample.message =
            "The provider response contained unsafe data and was not documented.".into();
        sample.matches.clear();
        sample.response_relative_path = None;
        sample.response_sha256 = None;
    }
}

pub(super) fn external_matches_have_finite_scores(record: &AudioScreeningExternalRecord) -> bool {
    record
        .matches
        .iter()
        .all(|item| item.score.is_none_or(f64::is_finite))
        && record.samples.iter().all(|sample| {
            sample
                .matches
                .iter()
                .all(|item| item.score.is_none_or(f64::is_finite))
        })
}

/// Refreshes only the derived Markdown summary after the application has
/// updated a non-network external status such as `SKIPPED_NOT_CONFIGURED`.
/// The local JSON and any provider response remain untouched.
pub fn refresh_screening_markdown(
    track_root: &Path,
    local: &AudioScreeningLocalRecord,
    external: &AudioScreeningExternalRecord,
) -> Result<()> {
    ensure_screening_directory(track_root)?;
    publish_screening_markdown(track_root, Some(local), Some(external))
}

pub(super) fn publish_screening_markdown(
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    external: Option<&AudioScreeningExternalRecord>,
) -> Result<()> {
    let mut markdown = String::from("# Pre-Release Audio Screening\n\n");
    markdown.push_str(
        "Technical audio recognition screening only. This record does not determine authorship, copyright ownership, licence validity, non-infringement, melodic originality, or legal safety.\n\n",
    );
    markdown.push_str("## Local audio fingerprint\n\n");
    write_local_screening_markdown(&mut markdown, local);
    markdown.push_str("\n## External catalog screening\n\n");
    write_external_screening_markdown(&mut markdown, external);
    write_managed(
        track_root,
        AUDIO_SCREENING_MARKDOWN_FILE,
        markdown.as_bytes(),
    )
}

fn write_local_screening_markdown(
    markdown: &mut String,
    local: Option<&AudioScreeningLocalRecord>,
) {
    if let Some(local) = local {
        markdown.push_str(&format!(
            "- Status: {}\n- Engine: {}\n- Engine version: {}\n- Algorithm: {}\n- Source evidence: {}\n- Source path: {}\n- Source SHA-256: {}\n- Source size: {} bytes\n",
            audio_screening_status_label(local.status),
            markdown_text(&local.engine),
            markdown_text(&local.engine_version),
            markdown_text(&local.fingerprint_algorithm),
            markdown_text(&local.source_evidence_id),
            markdown_text(&local.source_relative_path),
            markdown_text(&local.source_sha256),
            local.source_size_bytes,
        ));
        if let Some(duration) = local.duration_milliseconds {
            markdown.push_str(&format!("- Audio duration: {duration} ms\n"));
        }
        if let Some(generated_at) = &local.generated_at {
            markdown.push_str(&format!(
                "- Generated at: {}\n",
                markdown_text(generated_at)
            ));
        }
        markdown.push_str(&format!(
            "- Fingerprint record: {}\n- Fingerprint record SHA-256: {}\n- Note: {}\n",
            markdown_text(&local.artifact_relative_path),
            markdown_text(&local.artifact_sha256),
            markdown_text(&local.message),
        ));
    } else {
        markdown.push_str("- Status: NOT RUN\n");
    }
}

fn write_external_screening_markdown(
    markdown: &mut String,
    external: Option<&AudioScreeningExternalRecord>,
) {
    if let Some(external) = external {
        markdown.push_str(&format!(
            "- Provider: {}\n- Status: {}\n- Source evidence: {}\n- Source path: {}\n- Source SHA-256: {}\n- Source size: {} bytes\n- Requests: {}\n",
            markdown_text(&external.provider),
            audio_screening_status_label(external.status),
            markdown_text(&external.source_evidence_id),
            markdown_text(&external.source_relative_path),
            markdown_text(&external.source_sha256),
            external.source_size_bytes,
            external.request_count,
        ));
        if external.screening_mode == AudioScreeningMode::MultiSample
            || !external.samples.is_empty()
        {
            write_external_sampling_summary(markdown, external);
        }
        write_external_optional_fields(markdown, external);
        if !external.samples.is_empty() {
            write_external_samples(markdown, external);
        }
        if !external.matches.is_empty() {
            write_external_matches(markdown, external);
        }
    } else {
        markdown.push_str("- Status: NOT RUN\n");
    }
}

fn write_external_sampling_summary(markdown: &mut String, external: &AudioScreeningExternalRecord) {
    let reference_duration = external
        .reference_duration_seconds
        .map(|seconds| format!("{seconds} seconds"))
        .unwrap_or_else(|| "N/A".into());
    markdown.push_str(&format!(
        "- Screening mode: {}\n- Requested intensity: {} %\n- Calculation mode: {}\n- Reference duration: {}\n- Target duration: {} ms\n- Planned requests: {}\n- Executed requests: {}\n- Unique samples: {}\n- Duplicate samples: {}\n- Overlapping samples: {}\n- Unique sampled duration: {} ms\n- Track coverage: {:.2} %\n- Provider status: {:?}\n",
        external.screening_mode.as_str(),
        external.requested_intensity_percent,
        if external.dynamic_by_track_duration { "DYNAMIC_TRACK_DURATION" } else { "FIXED_REFERENCE_DURATION" },
        reference_duration,
        external.target_duration_milliseconds,
        external.planned_request_count,
        external.executed_request_count,
        external.unique_sample_count,
        external.duplicate_sample_count,
        external.overlapping_sample_count,
        external.unique_sample_duration_milliseconds,
        external.track_coverage_percent,
        external.provider_status,
    ));
}

fn write_external_optional_fields(markdown: &mut String, external: &AudioScreeningExternalRecord) {
    if let Some(checked_at) = &external.checked_at {
        markdown.push_str(&format!("- Checked at: {}\n", markdown_text(checked_at)));
    }
    if let Some(offset) = external.sample_offset_milliseconds {
        markdown.push_str(&format!("- Sample offset: {offset} ms\n"));
    }
    if let Some(duration) = external.sample_duration_milliseconds {
        markdown.push_str(&format!("- Sample duration: {duration} ms\n"));
    }
    if let Some(duration) = external.source_duration_milliseconds {
        markdown.push_str(&format!("- Source duration: {duration} ms\n"));
    }
    if let Some(path) = &external.response_relative_path {
        markdown.push_str(&format!("- Provider response: {}\n", markdown_text(path)));
    }
    if let Some(hash) = &external.response_sha256 {
        markdown.push_str(&format!(
            "- Provider response SHA-256: {}\n",
            markdown_text(hash)
        ));
    }
    markdown.push_str(&format!("- Note: {}\n", markdown_text(&external.message)));
}

fn write_external_samples(markdown: &mut String, external: &AudioScreeningExternalRecord) {
    markdown.push_str("\n### Submitted samples\n\n");
    for sample in &external.samples {
        markdown.push_str(&format!(
            "- Sample {:02}: Offset {} ms · End {} ms · Duration {} ms · {}\n  - Note: {}\n",
            sample.sequence,
            sample.offset_milliseconds,
            sample.end_offset_milliseconds,
            sample.duration_milliseconds,
            audio_screening_status_label(sample.status),
            markdown_text(&sample.message),
        ));
        if let Some(details) = sample.provider_status_details() {
            markdown.push_str(&format!("  - {}\n", markdown_text(&details)));
        }
        if let Some(path) = &sample.response_relative_path {
            markdown.push_str(&format!(
                "  - Provider response archive: {}\n",
                markdown_text(path)
            ));
        }
        if let Some(hash) = &sample.response_sha256 {
            markdown.push_str(&format!(
                "  - Provider response SHA-256: {}\n",
                markdown_text(hash)
            ));
        }
        write_sample_matches(markdown, sample);
    }
}

fn write_sample_matches(markdown: &mut String, sample: &AudioScreeningSampleRecord) {
    for item in sample.matches.iter().take(MAX_PROVIDER_MATCHES) {
        markdown.push_str(&format!(
            "  - Match title: {}\n",
            markdown_text(&item.title)
        ));
        if !item.artists.is_empty() {
            markdown.push_str(&format!(
                "    - Match artists: {}\n",
                item.artists
                    .iter()
                    .map(|artist| markdown_text(artist))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }
}

fn write_external_matches(markdown: &mut String, external: &AudioScreeningExternalRecord) {
    markdown.push_str("\n### Provider-reported matches\n\n");
    for item in external.matches.iter().take(MAX_PROVIDER_MATCHES) {
        markdown.push_str(&format!("- Title: {}\n", markdown_text(&item.title)));
        if !item.artists.is_empty() {
            markdown.push_str(&format!(
                "  - Artists: {}\n",
                item.artists
                    .iter()
                    .map(|item| markdown_text(item))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        if let Some(album) = &item.album {
            markdown.push_str(&format!("  - Album: {}\n", markdown_text(album)));
        }
        if let Some(isrc) = &item.isrc {
            markdown.push_str(&format!("  - ISRC: {}\n", markdown_text(isrc)));
        }
        if let Some(acrid) = &item.acrid {
            markdown.push_str(&format!("  - ACRID: {}\n", markdown_text(acrid)));
        }
        if let Some(score) = item.score {
            markdown.push_str(&format!("  - Provider score: {score}\n"));
        }
    }
}

pub(super) fn markdown_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .collect::<String>()
        .replace('|', "\\|")
        .replace('`', "\\`")
}

pub(super) fn ensure_screening_directory(track_root: &Path) -> Result<PathBuf> {
    ensure_contained_directory(track_root, Path::new(AUDIO_SCREENING_DIR))
}

pub(super) fn write_managed(track_root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    validate_relative(Path::new(relative))?;
    let target = contained_path(track_root, Path::new(relative), false)?;
    atomic_write(&target, bytes)
}

pub(super) fn archive_managed_artifacts(track_root: &Path, relatives: &[&str]) -> Result<()> {
    let mut existing = Vec::new();
    for relative in relatives {
        let target = contained_path(track_root, Path::new(relative), false)?;
        if !target.exists() {
            continue;
        }
        let metadata =
            fs::symlink_metadata(&target).map_err(|error| AppError::io(&target, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::Symlink(target.display().to_string()));
        }
        existing.push((relative, target));
    }
    if existing.is_empty() {
        return Ok(());
    }
    let archive_relative = PathBuf::from(".archive")
        .join("audio-screening")
        .join(Uuid::new_v4().to_string());
    ensure_contained_directory(track_root, &archive_relative)?;
    for (relative, target) in existing {
        let file_name = Path::new(relative)
            .file_name()
            .ok_or_else(|| AppError::Validation("Invalid audio-screening artifact name.".into()))?;
        let archived = contained_path(track_root, &archive_relative.join(file_name), false)?;
        fs::rename(&target, &archived).map_err(|error| AppError::io(&target, error))?;
    }
    Ok(())
}

/// Atomically archives the complete current screening directory below the
/// track-local archive tree.  It is deliberately separate from the selective
/// artifact archiver above: a release replacement must not leave a mixture of
/// old and newly generated screening files in the live directory.
///
/// Returns `false` without creating anything when no current directory is
/// present.  Symlinks are rejected both at the directory boundary and inside
/// it before the same-filesystem rename takes place.
pub fn archive_current_screening_artifacts(track_root: &Path) -> Result<bool> {
    let source_relative = Path::new(AUDIO_SCREENING_DIR);
    let source = contained_path(track_root, source_relative, false)?;
    let source_metadata = match fs::symlink_metadata(&source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(AppError::io(&source, error)),
    };
    if source_metadata.file_type().is_symlink() {
        return Err(AppError::Symlink(source.display().to_string()));
    }
    if !source_metadata.is_dir() {
        return Err(AppError::Validation(
            "The current audio-screening path is not a directory.".into(),
        ));
    }
    ensure_regular_screening_tree(&source)?;

    let archive_relative = PathBuf::from(".archive")
        .join("audio-screening")
        .join(Uuid::new_v4().to_string());
    ensure_contained_directory(track_root, &archive_relative)?;
    let destination_relative = archive_relative.join("AUDIO_SCREENING");
    let destination = contained_path(track_root, &destination_relative, false)?;
    ensure_archive_destination_available(&destination)?;

    // Both locations are contained below `track_root`, so `rename` is a
    // same-filesystem, atomic directory move on supported desktop targets.
    // Re-check the source entry immediately before the move to narrow the
    // validation-to-use race without following a substituted symlink.
    let source_metadata =
        fs::symlink_metadata(&source).map_err(|error| AppError::io(&source, error))?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
        return Err(AppError::Symlink(source.display().to_string()));
    }
    fs::rename(&source, &destination).map_err(|error| AppError::io(&source, error))?;
    Ok(true)
}

fn ensure_archive_destination_available(destination: &Path) -> Result<()> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(AppError::Symlink(destination.display().to_string()));
        }
        Ok(_) => return Err(AppError::Collision(destination.display().to_string())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(AppError::io(destination, error)),
    }
    Ok(())
}

pub(super) fn ensure_regular_screening_tree(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|error| AppError::io(directory, error))? {
        let entry = entry.map_err(|error| AppError::io(directory, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(AppError::Symlink(path.display().to_string()));
        }
        if metadata.is_dir() {
            ensure_regular_screening_tree(&path)?;
        } else if !metadata.is_file() {
            return Err(AppError::Validation(
                "The current audio-screening directory contains an unsupported file type.".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn read_existing_external_record(
    track_root: &Path,
) -> Result<Option<AudioScreeningExternalRecord>> {
    let path = contained_path(track_root, Path::new(EXTERNAL_SCREENING_FILE), false)?;
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Symlink(path.display().to_string()));
    }
    let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
    Ok(serde_json::from_slice(&bytes).ok())
}

/// Checks that a local record belongs to this exact track and authoritative
/// release evidence.  The fingerprint string itself is never used as a byte
/// integrity substitute.
pub fn local_record_matches_source(
    record: &AudioScreeningLocalRecord,
    expected_track_id: &str,
    evidence: &EvidenceItem,
) -> bool {
    record.track_id == expected_track_id
        && record.status == AudioScreeningStatus::FingerprintGenerated
        && source_binding_matches(
            &record.source_evidence_id,
            &record.source_relative_path,
            &record.source_sha256,
            record.source_size_bytes,
            evidence,
        )
        && !record.fingerprint.trim().is_empty()
        && record
            .duration_milliseconds
            .is_some_and(|duration| duration > 0)
        && !record.artifact_relative_path.trim().is_empty()
        && is_sha256(&record.artifact_sha256)
}

/// Checks that an external record belongs to this exact track and release
/// evidence.  It does not interpret a provider result as a legal conclusion.
pub fn external_record_matches_source(
    record: &AudioScreeningExternalRecord,
    expected_track_id: &str,
    evidence: &EvidenceItem,
) -> bool {
    record.track_id == expected_track_id
        && record.status != AudioScreeningStatus::Stale
        && source_binding_matches(
            &record.source_evidence_id,
            &record.source_relative_path,
            &record.source_sha256,
            record.source_size_bytes,
            evidence,
        )
}

pub fn local_artifact_is_current(
    track_root: &Path,
    record: &AudioScreeningLocalRecord,
) -> Result<bool> {
    if !is_sha256(&record.artifact_sha256)
        || record.artifact_relative_path != LOCAL_FINGERPRINT_FILE
    {
        return Ok(false);
    }
    let path = contained_path(track_root, Path::new(&record.artifact_relative_path), true)?;
    Ok(sha256_file(&path)? == record.artifact_sha256)
}

pub fn external_response_artifact_is_current(
    track_root: &Path,
    record: &AudioScreeningExternalRecord,
) -> Result<bool> {
    let record_archive = match (
        record.response_relative_path.as_deref(),
        record.response_sha256.as_deref(),
    ) {
        (None, None) => None,
        (Some(relative), Some(expected))
            if relative == ACRCLOUD_RESPONSE_FILE && is_sha256(expected) =>
        {
            Some((relative, expected))
        }
        _ => return Ok(false),
    };
    let mut archive_bytes = None;
    if let Some((relative, expected)) = record_archive {
        let path = contained_path(track_root, Path::new(relative), true)?;
        let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
        if sha256_bytes(&bytes) != expected {
            return Ok(false);
        }
        archive_bytes = Some(bytes);
    }
    if !sample_archive_references_are_current(record, record_archive) {
        return Ok(false);
    }
    if !record.samples.is_empty()
        && record_archive.is_some()
        && !sample_archive_coordinates_are_current(record, archive_bytes.as_deref())
    {
        return Ok(false);
    }
    Ok(true)
}

fn sample_archive_references_are_current(
    record: &AudioScreeningExternalRecord,
    record_archive: Option<(&str, &str)>,
) -> bool {
    // Multi-sample records may contain successful requests with no safe raw
    // response (for example a transport failure). Every retained response
    // must nevertheless point to the same verified aggregate archive.
    record.samples.iter().all(|sample| {
        match (
            sample.response_relative_path.as_deref(),
            sample.response_sha256.as_deref(),
        ) {
            (None, None) => true,
            (Some(relative), Some(expected)) => {
                record_archive.is_some_and(|(record_relative, record_expected)| {
                    relative == record_relative && expected == record_expected
                })
            }
            _ => false,
        }
    })
}

fn sample_archive_coordinates_are_current(
    record: &AudioScreeningExternalRecord,
    archive_bytes: Option<&[u8]>,
) -> bool {
    let Some(bytes) = archive_bytes else {
        return false;
    };
    let archive: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let Some(entries) = archive.get("samples").and_then(Value::as_array) else {
        return false;
    };
    record
        .samples
        .iter()
        .filter(|sample| {
            sample.response_relative_path.is_some() || sample.response_sha256.is_some()
        })
        .all(|sample| sample_coordinates_are_archived(entries, sample))
}

fn sample_coordinates_are_archived(entries: &[Value], sample: &AudioScreeningSampleRecord) -> bool {
    entries.iter().any(|entry| {
        entry.get("sequence").and_then(Value::as_u64) == Some(u64::from(sample.sequence))
            && entry.get("offsetMilliseconds").and_then(Value::as_u64)
                == Some(sample.offset_milliseconds)
            && entry.get("endOffsetMilliseconds").and_then(Value::as_u64)
                == Some(sample.end_offset_milliseconds)
            && entry.get("durationMilliseconds").and_then(Value::as_u64)
                == Some(sample.duration_milliseconds)
    })
}

pub fn mark_screening_stale(state: &mut AudioScreeningState) {
    if state.local.status != AudioScreeningStatus::NotRun {
        state.local.status = AudioScreeningStatus::Stale;
        state.local.message =
            "The authoritative release audio changed; the local fingerprint is stale.".into();
    }
    if state.external.status != AudioScreeningStatus::NotRun {
        state.external.status = AudioScreeningStatus::Stale;
        state.external.message =
            "The authoritative release audio changed; the external catalog result is stale.".into();
        // A replacement/retry archives the old provider response below
        // `.archive/audio-screening`. Do not leave its former live path on a
        // stale current-track record while that archival happens.
        state.external.response_relative_path = None;
        state.external.response_sha256 = None;
        for sample in &mut state.external.samples {
            sample.response_relative_path = None;
            sample.response_sha256 = None;
        }
    }
}

pub(super) fn source_binding_matches(
    record_evidence_id: &str,
    record_relative_path: &str,
    record_sha256: &str,
    record_size_bytes: u64,
    evidence: &EvidenceItem,
) -> bool {
    evidence.verified
        && evidence.verification_error.is_none()
        && record_evidence_id == evidence.id
        && record_relative_path == evidence.relative_path
        && record_size_bytes == evidence.size_bytes
        && evidence.sha256.as_deref() == Some(record_sha256)
        && is_sha256(record_sha256)
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// A process-private source copy whose bytes were streamed and hashed before
/// any decoder or network adapter can consume them. `TempDir` removes the
/// working audio automatically and has owner-only permissions on supported
/// platforms; the temporary copy is never placed in the track tree.
pub(super) struct VerifiedSourceSnapshot {
    _directory: TempDir,
    pub(super) path: PathBuf,
}

pub(super) fn create_verified_source_snapshot(
    source_path: &Path,
    evidence: &EvidenceItem,
    track_root: &Path,
) -> std::result::Result<VerifiedSourceSnapshot, ()> {
    let source = validate_source_binding(source_path, evidence, track_root)?;
    let expected_hash = evidence
        .sha256
        .as_deref()
        .filter(|hash| is_sha256(hash))
        .ok_or(())?;
    let source_metadata = fs::metadata(&source).map_err(|_| ())?;
    if !source_metadata.is_file() || source_metadata.len() != evidence.size_bytes {
        return Err(());
    }

    let directory = TempfileBuilder::new()
        .prefix("sunodm-audio-screening-")
        .tempdir()
        .map_err(|_| ())?;
    let path = directory.path().join("authoritative-release.audio");
    let mut input = File::open(&source).map_err(|_| ())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| ())?;
    let mut hasher = Sha256::new();
    let mut copied_bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer).map_err(|_| ())?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).map_err(|_| ())?;
        hasher.update(&buffer[..read]);
        copied_bytes = copied_bytes.checked_add(read as u64).ok_or(())?;
    }
    output.sync_all().map_err(|_| ())?;
    let copied_hash = format!("{:x}", hasher.finalize());
    if copied_bytes != evidence.size_bytes || copied_hash != expected_hash {
        return Err(());
    }
    Ok(VerifiedSourceSnapshot {
        _directory: directory,
        path,
    })
}

pub(super) fn validate_source_binding(
    source_path: &Path,
    evidence: &EvidenceItem,
    track_root: &Path,
) -> std::result::Result<PathBuf, ()> {
    if !evidence.verified
        || evidence.verification_error.is_some()
        || !is_sha256(evidence.sha256.as_deref().unwrap_or_default())
    {
        return Err(());
    }
    let relative = Path::new(&evidence.relative_path);
    validate_relative(relative).map_err(|_| ())?;
    let managed = contained_path(track_root, relative, true).map_err(|_| ())?;
    let metadata = fs::symlink_metadata(&managed).map_err(|_| ())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let supplied = fs::canonicalize(source_path).map_err(|_| ())?;
    if supplied != managed {
        return Err(());
    }
    let actual = sha256_file(&managed).map_err(|_| ())?;
    if evidence.sha256.as_deref() != Some(actual.as_str()) {
        return Err(());
    }
    Ok(managed)
}
