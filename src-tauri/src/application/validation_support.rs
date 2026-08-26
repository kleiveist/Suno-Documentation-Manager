use super::*;

pub(super) fn validate_short_text(
    name: &str,
    value: &str,
    max: usize,
    required: bool,
) -> Result<()> {
    let trimmed = value.trim();
    if required && trimmed.is_empty() {
        return Err(AppError::Validation(format!("{name} is required.")));
    }
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!(
            "{name} is invalid or too long."
        )));
    }
    Ok(())
}

pub(super) fn validate_multiline_text(name: &str, value: &str, max: usize) -> Result<()> {
    if value.chars().count() > max
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AppError::Validation(format!(
            "{name} is invalid or too long."
        )));
    }
    Ok(())
}

pub(super) fn validate_text_list(
    name: &str,
    values: &[String],
    max_items: usize,
    max_item: usize,
) -> Result<()> {
    if values.len() > max_items {
        return Err(AppError::Validation(format!(
            "{name} contains too many entries."
        )));
    }
    for value in values {
        validate_multiline_text(name, value, max_item)?;
    }
    Ok(())
}

pub(super) fn parse_date(name: &str, value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("{name} must use YYYY-MM-DD.")))
}

pub(super) fn subscription_coverage_end(
    coverage_start: &str,
    billing_cycle: SubscriptionBillingCycle,
) -> Result<String> {
    let start = parse_date("Subscription coverage start", coverage_start)?;
    let months = match billing_cycle {
        SubscriptionBillingCycle::Monthly => 1,
        SubscriptionBillingCycle::Annual => 12,
    };
    let next_period_start = start
        .checked_add_months(Months::new(months))
        .ok_or_else(|| {
            AppError::Validation(
                "Subscription coverage date is outside the supported range.".into(),
            )
        })?;
    let end = next_period_start.pred_opt().ok_or_else(|| {
        AppError::Validation("Subscription coverage date is outside the supported range.".into())
    })?;
    Ok(end.format("%Y-%m-%d").to_string())
}

pub(super) fn validate_optional_date(name: &str, value: &str) -> Result<()> {
    if !value.trim().is_empty() {
        parse_date(name, value)?;
    }
    Ok(())
}

pub(super) fn validate_date_range(name: &str, start: &str, end: &str) -> Result<()> {
    let start_date = parse_date(&format!("{name} start"), start)?;
    let end_date = parse_date(&format!("{name} end"), end)?;
    if end_date < start_date {
        return Err(AppError::Validation(format!(
            "{name} end cannot be before its start."
        )));
    }
    Ok(())
}

pub(super) fn validate_profile(profile: &Profile, complete: bool) -> Result<()> {
    for (name, value) in [
        ("Artist name", profile.artist_name.as_str()),
        ("Suno profile name", profile.suno_profile_name.as_str()),
        ("Suno handle", profile.suno_handle.as_str()),
        ("Suno plan", profile.suno_plan.as_str()),
        (
            "Default AI image service",
            profile.default_ai_image_service.as_str(),
        ),
        ("Disclosure text", profile.disclosure_text.as_str()),
    ] {
        validate_short_text(name, value, 500, complete)?;
    }
    validate_short_text(
        "Subscription start date",
        &profile.subscription_start_date,
        10,
        complete,
    )?;
    validate_optional_date("Subscription start date", &profile.subscription_start_date)?;
    if !matches!(
        profile.artwork_transparency_policy.as_str(),
        "always" | "per_artwork" | "none"
    ) {
        return Err(AppError::Validation(
            "Artwork transparency policy is invalid.".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_track_fields(fields: &crate::model::TrackFields) -> Result<()> {
    let normalized_fields = fields.normalized_conditionals();
    let fields = &normalized_fields;
    validate_track_title(&fields.title)?;
    validate_track_short_fields(fields)?;
    validate_track_multiline_fields(fields)?;
    validate_track_lists_and_dates(fields)?;
    validate_track_relationships(fields)
}

fn validate_track_short_fields(fields: &crate::model::TrackFields) -> Result<()> {
    for (name, value, max) in [
        ("Suno model", fields.suno_model.as_str(), 200),
        (
            "Suno plan at generation",
            fields.suno_plan_at_generation.as_str(),
            200,
        ),
        (
            "Legacy Suno plan at creation",
            fields.legacy_suno_plan_at_creation.as_str(),
            200,
        ),
        ("Audio AI system", fields.audio_ai_system.as_str(), 500),
        (
            "Other Suno lyrics/structure content type",
            fields.suno_lyrics_other_content_type.as_str(),
            1000,
        ),
        ("AI image service", fields.ai_image_service.as_str(), 500),
        ("Disclosure text", fields.disclosure_text.as_str(), 80),
    ] {
        validate_short_text(name, value, max, false)?;
    }
    Ok(())
}

fn validate_track_multiline_fields(fields: &crate::model::TrackFields) -> Result<()> {
    for (name, value, max) in [
        ("Lyrics", fields.lyrics_text.as_str(), 1_000_000),
        (
            "Suno lyrics/structure field content",
            fields.suno_lyrics_field_text.as_str(),
            1_000_000,
        ),
        (
            "Suno style prompt",
            fields.suno_style_prompt.as_str(),
            100_000,
        ),
        (
            "External audio source",
            fields.external_audio_source.as_str(),
            4000,
        ),
        (
            "External audio ownership",
            fields.external_audio_ownership.as_str(),
            4000,
        ),
        ("Own audio source", fields.own_audio_source.as_str(), 4000),
        (
            "Own audio ownership",
            fields.own_audio_ownership.as_str(),
            4000,
        ),
        (
            "Sample source",
            fields.third_party_sample_source.as_str(),
            4000,
        ),
        (
            "Sample ownership",
            fields.third_party_sample_ownership.as_str(),
            4000,
        ),
        (
            "Human editing details",
            fields.human_editing_details.as_str(),
            20_000,
        ),
        (
            "Post-export editing details",
            fields.post_export_editing_details.as_str(),
            20_000,
        ),
        (
            "Code-audio post-processing note",
            fields.code_audio_post_processing_note.as_str(),
            20_000,
        ),
        (
            "Human artwork process notes",
            fields.human_artwork_process_notes.as_str(),
            20_000,
        ),
        (
            "Custom artwork change",
            fields.custom_artwork_change.as_str(),
            20_000,
        ),
        (
            "Real-person note",
            fields.real_person_notes.as_str(),
            20_000,
        ),
        ("Real-event note", fields.real_event_notes.as_str(), 20_000),
        ("Trademark note", fields.trademark_notes.as_str(), 20_000),
        (
            "Audio disclosure text",
            fields.audio_disclosure_text.as_str(),
            20_000,
        ),
        (
            "Audio disclosure reason",
            fields.audio_disclosure_reason.as_str(),
            20_000,
        ),
        ("Release notes", fields.release_notes.as_str(), 20_000),
    ] {
        validate_multiline_text(name, value, max)?;
    }
    Ok(())
}

fn validate_track_lists_and_dates(fields: &crate::model::TrackFields) -> Result<()> {
    for (name, values) in [
        (
            "Code-audio post-processing operations",
            fields.code_audio_post_processing_operations.as_slice(),
        ),
        (
            "Human artwork process operations",
            fields.human_artwork_process_operations.as_slice(),
        ),
        (
            "Human artwork modifications",
            fields.human_artwork_modifications.as_slice(),
        ),
        (
            "Audio disclosure locations",
            fields.audio_disclosure_locations.as_slice(),
        ),
    ] {
        validate_text_list(name, values, 100, 4_000)?;
    }
    for (name, value) in [
        ("Production start", fields.production_start_date.as_str()),
        ("Production end", fields.production_end_date.as_str()),
        ("Last editing date", fields.final_export_date.as_str()),
        (
            "Final generation date",
            fields.suno_final_generation_date.as_str(),
        ),
        (
            "Suno download/export date",
            fields.suno_download_export_date.as_str(),
        ),
    ] {
        validate_optional_date(name, value)?;
    }
    Ok(())
}

fn validate_track_relationships(fields: &crate::model::TrackFields) -> Result<()> {
    if !fields.production_start_date.is_empty() && !fields.production_end_date.is_empty() {
        validate_date_range(
            "Production period",
            &fields.production_start_date,
            &fields.production_end_date,
        )?;
    }
    if !fields.final_export_date.is_empty() && !fields.production_start_date.is_empty() {
        let start = parse_date("Production start", &fields.production_start_date)?;
        let export = parse_date("Last editing date", &fields.final_export_date)?;
        if export < start {
            return Err(AppError::Validation(
                "Last editing date cannot be before production start.".into(),
            ));
        }
    }
    if !fields.lyrics_source.is_empty()
        && !matches!(
            fields.lyrics_source.as_str(),
            "instrumental" | "human" | "suno" | "mixed"
        )
    {
        return Err(AppError::Validation("Lyrics source is invalid.".into()));
    }
    if !fields.artwork_origin.is_empty()
        && !matches!(
            fields.artwork_origin.as_str(),
            "none" | "human" | "ai_generated" | "ai_assisted"
        )
    {
        return Err(AppError::Validation("Artwork origin is invalid.".into()));
    }
    if !fields.suno_project_url.trim().is_empty() {
        let url = Url::parse(fields.suno_project_url.trim())
            .map_err(|_| AppError::Validation("Suno project URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Suno project URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_evidence_metadata(
    role: &EvidenceRole,
    metadata: &EvidenceMetadata,
) -> Result<()> {
    validate_evidence_metadata_text(metadata)?;
    validate_evidence_metadata_dates_and_urls(metadata)?;
    validate_evidence_metadata_for_role(role, metadata)
}

fn validate_evidence_metadata_text(metadata: &EvidenceMetadata) -> Result<()> {
    for (name, value, max) in [
        (
            "Evidence document title",
            metadata.document_title.as_str(),
            1000,
        ),
        ("Evidence provider", metadata.provider.as_str(), 1000),
        ("Evidence source URL", metadata.source_url.as_str(), 4000),
        (
            "Evidence factual note",
            metadata.factual_note.as_str(),
            20_000,
        ),
        (
            "Applicable production period",
            metadata.applicable_production_period.as_str(),
            1000,
        ),
        ("Timestamp type", metadata.timestamp_type.as_str(), 200),
        ("Timestamp value", metadata.external_timestamp.as_str(), 200),
        ("Referenced hash", metadata.referenced_hash.as_str(), 512),
        (
            "Referenced artifact",
            metadata.referenced_artifact.as_str(),
            4000,
        ),
        (
            "External reference ID",
            metadata.external_reference_id.as_str(),
            1000,
        ),
        (
            "Provider verification URL",
            metadata.provider_verification_url.as_str(),
            4000,
        ),
        (
            "Evidence file extension",
            metadata.file_extension.as_str(),
            32,
        ),
        ("Evidence MIME type", metadata.mime_type.as_str(), 200),
        ("Evidence audio format", metadata.audio_format.as_str(), 200),
        (
            "Suno created timestamp",
            metadata.suno_created_timestamp.as_str(),
            200,
        ),
        ("Suno created date", metadata.suno_created_date.as_str(), 10),
        ("Suno technical ID", metadata.suno_id.as_str(), 200),
        (
            "Raw embedded Suno metadata",
            metadata.suno_raw_metadata.as_str(),
            65_536,
        ),
    ] {
        validate_multiline_text(name, value, max)?;
    }
    if metadata.embedded_metadata.len() > 256 {
        return Err(AppError::Validation(
            "Evidence contains too many embedded metadata entries.".into(),
        ));
    }
    for entry in &metadata.embedded_metadata {
        validate_multiline_text("Embedded metadata key", &entry.key, 128)?;
        validate_multiline_text("Embedded metadata value", &entry.value, 65_536)?;
    }
    Ok(())
}

fn validate_evidence_metadata_dates_and_urls(metadata: &EvidenceMetadata) -> Result<()> {
    for (name, value) in [
        ("Evidence retrieval date", metadata.retrieval_date.as_str()),
        ("Evidence effective date", metadata.effective_date.as_str()),
    ] {
        validate_optional_date(name, value)?;
    }
    if !metadata.source_url.trim().is_empty() {
        let url = Url::parse(metadata.source_url.trim())
            .map_err(|_| AppError::Validation("Evidence source URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Evidence source URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    if !metadata.provider_verification_url.trim().is_empty() {
        let url = Url::parse(metadata.provider_verification_url.trim())
            .map_err(|_| AppError::Validation("Provider verification URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Provider verification URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

fn validate_evidence_metadata_for_role(
    role: &EvidenceRole,
    metadata: &EvidenceMetadata,
) -> Result<()> {
    match role {
        EvidenceRole::SunoTermsRights => {
            if metadata.document_title.trim().is_empty()
                || metadata.provider.trim().is_empty()
                || metadata.retrieval_date.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "Terms evidence exists, but descriptive metadata is incomplete: document title, provider/source, and retrieval date are required."
                        .into(),
                ));
            }
        }
        EvidenceRole::ExternalTimestamp => {
            if metadata.provider.trim().is_empty()
                || metadata.external_timestamp.trim().is_empty()
                || metadata.referenced_hash.trim().is_empty()
                || metadata.referenced_artifact.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "External timestamp evidence requires provider/issuer, timestamp, referenced hash, and referenced artifact.".into(),
                ));
            }
            if metadata.referenced_hash.len() != 64
                || !metadata
                    .referenced_hash
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(AppError::Validation(
                    "External timestamp evidence referenced hash must be a SHA-256 value.".into(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn apply_descriptive_evidence_metadata(
    target: &mut EvidenceMetadata,
    supplied: &EvidenceMetadata,
) {
    target.document_title = supplied.document_title.clone();
    target.provider = supplied.provider.clone();
    target.source_url = supplied.source_url.clone();
    target.retrieval_date = supplied.retrieval_date.clone();
    target.effective_date = supplied.effective_date.clone();
    target.applicable_production_period = supplied.applicable_production_period.clone();
    target.factual_note = supplied.factual_note.clone();
}

pub(super) fn normalize_track_library(
    input: TrackLibraryPlacement,
) -> Result<TrackLibraryPlacement> {
    match input.section {
        TrackLibrarySection::Single => Ok(TrackLibraryPlacement::default()),
        TrackLibrarySection::Album => {
            let raw_title = input.album_title.as_deref().unwrap_or_default();
            if raw_title.chars().any(char::is_control) {
                return Err(AppError::Validation(
                    "Album title is invalid or too long.".into(),
                ));
            }
            let title = raw_title.trim();
            validate_short_text("Album title", title, 200, true)?;
            safe_album_directory(title)?;
            Ok(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some(title.to_owned()),
            })
        }
    }
}

pub(super) fn safe_album_directory(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty()
        || title.starts_with('.')
        || title.eq_ignore_ascii_case(SINGLES_DIRECTORY)
        || title.contains(['/', '\\'])
        || title.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "Album title does not form a safe folder name.".into(),
        ));
    }
    Ok(title.to_owned())
}

pub(super) fn physical_library_parent(library: &TrackLibraryPlacement) -> Result<PathBuf> {
    match library.section {
        TrackLibrarySection::Single => Ok(PathBuf::from(SINGLES_DIRECTORY)),
        TrackLibrarySection::Album => Ok(PathBuf::from(safe_album_directory(
            library.album_title.as_deref().unwrap_or_default(),
        )?)),
    }
}

pub(super) fn physical_track_relative(
    library: &TrackLibraryPlacement,
    title: &str,
) -> Result<String> {
    Ok(portable_relative(
        &physical_library_parent(library)?.join(safe_track_directory(title)?),
    ))
}

pub(super) fn safe_track_directory(title: &str) -> Result<String> {
    let title = title.trim();
    slugify(title)?;
    if title.starts_with('.') || title.contains(['/', '\\']) || title.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "Track title does not form a safe folder name.".into(),
        ));
    }
    Ok(title.to_owned())
}

pub(super) fn physical_library_from_relative(
    relative: &str,
) -> Result<Option<TrackLibraryPlacement>> {
    let relative = Path::new(relative);
    crate::security::validate_relative(relative)?;
    let components = relative.components().collect::<Vec<_>>();
    if components.len() != 2 {
        return Ok(None);
    }
    let parent = components[0]
        .as_os_str()
        .to_str()
        .ok_or_else(|| AppError::Validation("Track library folder name must use UTF-8.".into()))?;
    if parent == SINGLES_DIRECTORY {
        return Ok(Some(TrackLibraryPlacement::default()));
    }
    Ok(Some(normalize_track_library(TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some(parent.to_owned()),
    })?))
}

pub(super) fn is_hidden_workspace_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
}

pub(super) fn discover_track_identities(root: &Path) -> Result<HashMap<String, String>> {
    let mut identities = HashMap::new();
    for entry in WalkDir::new(root)
        .min_depth(3)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry.depth() > 2
                || !entry.file_name().to_string_lossy().starts_with('.')
        })
    {
        let entry = entry.map_err(|error| {
            AppError::io(
                error.path().unwrap_or(root),
                std::io::Error::other(error.to_string()),
            )
        })?;
        if entry.file_type().is_symlink() {
            continue;
        }
        if !entry.file_type().is_file()
            || entry.file_name() != std::ffi::OsStr::new("track.json")
            || entry.path().parent().and_then(Path::file_name)
                != Some(std::ffi::OsStr::new(".summary"))
        {
            continue;
        }
        let Some(track_root) = entry.path().parent().and_then(Path::parent) else {
            continue;
        };
        if track_root.starts_with(root.join(".suno-doc")) {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(id) = value.get("trackId").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let relative = portable_relative(
            track_root
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?,
        );
        match identities.entry(id.to_owned()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(relative);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(String::new());
            }
        }
    }
    identities.retain(|_, relative| !relative.is_empty());
    Ok(identities)
}

pub(super) fn find_unclaimed_track_in_library(
    root: &Path,
    library: &TrackLibraryPlacement,
    claimed: &HashSet<String>,
) -> Result<Option<String>> {
    let parent_relative = physical_library_parent(library)?;
    let parent = contained_path(root, &parent_relative, false)?;
    if !parent.is_dir() {
        return Ok(None);
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&parent).map_err(|error| AppError::io(&parent, error))? {
        let entry = entry.map_err(|error| AppError::io(&parent, error))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let relative = portable_relative(
            entry
                .path()
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?,
        );
        if !claimed.contains(&relative) {
            candidates.push(relative);
        }
    }
    Ok((candidates.len() == 1).then(|| candidates.remove(0)))
}

pub(super) fn discover_workspace_tracks(
    root: &Path,
    warnings: &mut Vec<String>,
) -> Result<Vec<(String, PathBuf, TrackLibraryPlacement)>> {
    let mut result = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| AppError::io(root, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| AppError::io(root, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() {
            warnings.push(format!("Skipped symbolic-link candidate: {name}"));
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }

        if name == SINGLES_DIRECTORY {
            collect_library_children(
                root,
                &entry.path(),
                TrackLibraryPlacement::default(),
                warnings,
                &mut result,
            )?;
        } else if looks_like_track_root(&entry.path()) {
            result.push((
                name.clone(),
                PathBuf::from(name),
                TrackLibraryPlacement::default(),
            ));
        } else {
            let library = match normalize_track_library(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some(name.clone()),
            }) {
                Ok(library) => library,
                Err(error) => {
                    warnings.push(format!("Skipped invalid album folder {name}: {error}"));
                    continue;
                }
            };
            collect_library_children(root, &entry.path(), library, warnings, &mut result)?;
        }
    }
    result.sort_by(|left, right| {
        portable_relative(&left.1)
            .to_lowercase()
            .cmp(&portable_relative(&right.1).to_lowercase())
    });
    Ok(result)
}

pub(super) fn collect_library_children(
    root: &Path,
    parent: &Path,
    library: TrackLibraryPlacement,
    warnings: &mut Vec<String>,
    result: &mut Vec<(String, PathBuf, TrackLibraryPlacement)>,
) -> Result<()> {
    for entry in fs::read_dir(parent).map_err(|error| AppError::io(parent, error))? {
        let entry = entry.map_err(|error| AppError::io(parent, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() {
            warnings.push(format!(
                "Skipped symbolic-link track candidate: {}",
                entry.path().display()
            ));
            continue;
        }
        if !metadata.is_dir() || name.starts_with('.') {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| AppError::PathEscape)?
            .to_owned();
        crate::security::validate_relative(&relative)?;
        result.push((name, relative, library.clone()));
    }
    Ok(())
}

pub(super) fn looks_like_track_root(path: &Path) -> bool {
    path.join(TRACK_IDENTITY_FILE).is_file()
        || [
            "01_RELEASE",
            "02_SUNO",
            "03_DOCUMENTATION",
            "04_LICENSES",
            "05_ARTWORK",
            "06_CERTIFICATE",
        ]
        .iter()
        .any(|folder| path.join(folder).is_dir())
}

pub(super) fn validate_track_title(title: &str) -> Result<()> {
    validate_short_text("Track title", title, 200, true)?;
    if title.contains(['/', '\\']) || title.split_whitespace().any(|part| part == "..") {
        return Err(AppError::Validation(
            "Track title must not contain path separators or traversal components.".into(),
        ));
    }
    slugify(title)?;
    Ok(())
}

pub(super) fn validate_required_production_range(track: &TrackRecord) -> Result<()> {
    if track.fields.production_start_date.is_empty() || track.fields.production_end_date.is_empty()
    {
        return Err(AppError::Validation(
            "Production start and end dates are required for this operation.".into(),
        ));
    }
    validate_date_range(
        "Production period",
        &track.fields.production_start_date,
        &track.fields.production_end_date,
    )
}
