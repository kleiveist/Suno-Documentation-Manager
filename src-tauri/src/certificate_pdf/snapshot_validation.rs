use super::*;

pub(super) fn validate_snapshot(
    snapshot: &CertificatePdfSnapshot<'_>,
) -> Result<workflow::WorkflowConfig> {
    validate_snapshot_header(snapshot)?;
    validate_snapshot_evidence(snapshot)?;
    let config = validate_snapshot_workflow(snapshot)?;
    if snapshot
        .deviations
        .iter()
        .any(|deviation| deviation.blocking && !deviation.resolved)
    {
        return Err(AppError::Validation(
            "Certificate PDF snapshot contains an open blocking deviation.".into(),
        ));
    }
    validate_audio_screening_consistency(&snapshot.track.audio_screening.external)?;
    validate_rendered_text(snapshot, &config)?;
    Ok(config)
}

fn validate_snapshot_header(snapshot: &CertificatePdfSnapshot<'_>) -> Result<()> {
    for (label, value) in [
        ("certificate ID", snapshot.certificate_id),
        ("finalization timestamp", snapshot.finalized_at),
        ("certificate version", snapshot.certificate_version),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::Data(format!(
                "Certificate PDF snapshot has no {label}."
            )));
        }
    }
    for (label, digest) in [
        ("SHA256SUMS.txt", snapshot.sha256sums_sha256),
        ("EVIDENCE_MANIFEST.json", snapshot.evidence_manifest_sha256),
        (
            "DOCUMENTATION_CERTIFICATE.md",
            snapshot.markdown_certificate_sha256,
        ),
    ] {
        validate_sha256(label, digest)?;
    }
    for (label, value) in [
        (
            "timestamp provider",
            snapshot.finalization_timestamp.provider.as_str(),
        ),
        (
            "timestamp provider configuration message",
            snapshot
                .finalization_timestamp
                .provider_configuration_message
                .as_str(),
        ),
        (
            "concrete timestamp message",
            snapshot.finalization_timestamp.technical_message.as_str(),
        ),
        (
            "timestamp value",
            snapshot.finalization_timestamp.timestamp_value.as_str(),
        ),
        (
            "timestamp external reference ID",
            snapshot
                .finalization_timestamp
                .external_reference_id
                .as_str(),
        ),
        (
            "timestamp provider endpoint",
            snapshot
                .finalization_timestamp
                .provider_verification_url
                .as_str(),
        ),
    ] {
        validate_win_ansi(label, value)?;
    }
    if let Some(metadata) = snapshot.finalization_timestamp.provider_metadata.as_ref() {
        validate_timestamp_qualification_metadata(metadata)?;
    }
    Ok(())
}

fn validate_snapshot_evidence(snapshot: &CertificatePdfSnapshot<'_>) -> Result<()> {
    if snapshot
        .evidence
        .windows(2)
        .any(|items| items[0].relative_path > items[1].relative_path)
    {
        return Err(AppError::Data(
            "Certificate PDF evidence is not sorted by relative path.".into(),
        ));
    }
    for item in snapshot.evidence {
        if item.role == EvidenceRole::ExternalTimestamp {
            return Err(AppError::Data(
                "Phase-one certificate PDF cannot contain legacy external timestamp evidence."
                    .into(),
            ));
        }
        if !item.verified || item.verification_error.is_some() {
            return Err(AppError::Data(format!(
                "Certificate PDF evidence is not verified: {}",
                item.relative_path
            )));
        }
        let digest = item.sha256.as_deref().ok_or_else(|| {
            AppError::Data(format!(
                "Certificate PDF evidence has no SHA-256: {}",
                item.relative_path
            ))
        })?;
        validate_sha256(&item.relative_path, digest)?;
    }
    let mut preview_roles = HashSet::new();
    let mut preview_resources = HashMap::new();
    for preview in snapshot.artwork_previews {
        if !preview_roles.insert(preview.role)
            || preview.width_pixels == 0
            || preview.height_pixels == 0
            || preview.width_pixels > ARTWORK_PREVIEW_MAX_PIXELS
            || preview.height_pixels > ARTWORK_PREVIEW_MAX_PIXELS
            || preview.cmyk_pixels.len()
                != preview.width_pixels as usize * preview.height_pixels as usize * 4
        {
            return Err(AppError::Data(
                "Certificate PDF contains an invalid or duplicate artwork preview.".into(),
            ));
        }
        let resource_identity = (
            preview.sha256.as_str(),
            preview.width_pixels,
            preview.height_pixels,
        );
        if preview_resources
            .insert(preview.resource_id.as_str(), resource_identity)
            .is_some_and(|existing| existing != resource_identity)
        {
            return Err(AppError::Data(
                "Certificate PDF artwork resource ID is bound to different images.".into(),
            ));
        }
        let registered = snapshot.evidence.iter().copied().any(|item| {
            item.role == preview.role
                && item.id == preview.evidence_id
                && item.file_name == preview.file_name
                && item.relative_path == preview.relative_path
                && item
                    .sha256
                    .as_deref()
                    .is_some_and(|digest| digest.eq_ignore_ascii_case(&preview.sha256))
        });
        if !registered {
            return Err(AppError::Data(
                "Certificate PDF artwork preview is not bound to registered evidence.".into(),
            ));
        }
    }
    if snapshot.track.fields.suno_terms_evidence_not_available == Some(true)
        && snapshot
            .evidence
            .iter()
            .any(|item| item.role == EvidenceRole::SunoTermsRights)
    {
        return Err(AppError::Validation(
            "Certificate PDF cannot combine verified Terms evidence with an unavailable claim."
                .into(),
        ));
    }
    Ok(())
}

fn validate_snapshot_workflow(
    snapshot: &CertificatePdfSnapshot<'_>,
) -> Result<workflow::WorkflowConfig> {
    let config = workflow::config()?;
    if snapshot.track.workflow_id != config.id || snapshot.track.workflow_version != config.version
    {
        return Err(AppError::Data(
            "Certificate PDF workflow metadata does not match the configured workflow.".into(),
        ));
    }
    let configured_ids = config
        .steps
        .iter()
        .map(|step| step.id.as_str())
        .collect::<HashSet<_>>();
    let mut supplied_ids = HashSet::new();
    for step in snapshot.steps {
        if !configured_ids.contains(step.id.as_str()) {
            return Err(AppError::Data(format!(
                "Certificate PDF contains an unknown workflow step: {}",
                step.id
            )));
        }
        if !supplied_ids.insert(step.id.as_str()) {
            return Err(AppError::Data(format!(
                "Certificate PDF contains a duplicate workflow step: {}",
                step.id
            )));
        }
        match step.status {
            StepStatus::Pass => {}
            StepStatus::NotApplicable => {
                if step
                    .na_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(AppError::Data(format!(
                        "Certificate PDF workflow step has no N/A reason: {}",
                        step.id
                    )));
                }
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "Certificate PDF workflow step is not complete: {} ({})",
                    step.id,
                    step_status(&step.status)
                )));
            }
        }
    }
    if supplied_ids.len() != configured_ids.len() {
        return Err(AppError::Data(
            "Certificate PDF workflow snapshot is incomplete.".into(),
        ));
    }
    Ok(config)
}

pub(super) fn validate_audio_screening_consistency(
    external: &AudioScreeningExternalRecord,
) -> Result<()> {
    validate_audio_screening_coverage(external)?;
    let sample_match_count = validate_audio_sample_matches(external)?;
    validate_audio_overall_matches(external, sample_match_count)?;
    if sample_match_count > 0
        && !external.matches.is_empty()
        && !aggregate_matches_are_repeated_in_samples(external)
    {
        return Err(AppError::Data(
            "Certificate PDF ACRCloud aggregate matches contradict the per-sample records.".into(),
        ));
    }
    Ok(())
}

fn validate_audio_screening_coverage(external: &AudioScreeningExternalRecord) -> Result<()> {
    if !external.track_coverage_percent.is_finite()
        || !(0.0..=100.0).contains(&external.track_coverage_percent)
    {
        return Err(AppError::Data(
            "Certificate PDF ACRCloud track coverage is outside the factual percentage range."
                .into(),
        ));
    }
    if let Some(source_duration) = external.source_duration_milliseconds {
        if source_duration == 0 && external.unique_sample_duration_milliseconds > 0 {
            return Err(AppError::Data(
                "Certificate PDF ACRCloud sampled duration has no positive source duration.".into(),
            ));
        }
        if source_duration > 0 {
            let expected = (external.unique_sample_duration_milliseconds as f64 * 100.0)
                / source_duration as f64;
            if format!("{:.2}", external.track_coverage_percent) != format!("{expected:.2}") {
                return Err(AppError::Data(
                    "Certificate PDF ACRCloud track coverage contradicts the documented durations."
                        .into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_audio_sample_matches(external: &AudioScreeningExternalRecord) -> Result<usize> {
    let mut sample_match_count = 0usize;
    for sample in &external.samples {
        let has_matches = !sample.matches.is_empty();
        match sample.status {
            AudioScreeningStatus::MatchDetected if !has_matches => {
                return Err(AppError::Data(format!(
                    "Certificate PDF ACRCloud sample {} reports MATCH DETECTED without a provider match record.",
                    sample.sequence
                )));
            }
            AudioScreeningStatus::MatchDetected => {
                sample_match_count += sample.matches.len();
            }
            _ if has_matches => {
                return Err(AppError::Data(format!(
                    "Certificate PDF ACRCloud sample {} contains provider matches but its result is not MATCH DETECTED.",
                    sample.sequence
                )));
            }
            _ => {}
        }
    }
    Ok(sample_match_count)
}

fn validate_audio_overall_matches(
    external: &AudioScreeningExternalRecord,
    sample_match_count: usize,
) -> Result<()> {
    let has_any_match = sample_match_count > 0 || !external.matches.is_empty();
    match external.status {
        AudioScreeningStatus::MatchDetected if !has_any_match => {
            return Err(AppError::Data(
                "Certificate PDF ACRCloud overall result reports MATCH DETECTED without a provider match record."
                    .into(),
            ));
        }
        AudioScreeningStatus::MatchDetected => {}
        _ if has_any_match => {
            return Err(AppError::Data(
                "Certificate PDF ACRCloud overall result contradicts its provider match records."
                    .into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_sha256(label: &str, digest: &str) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Data(format!(
            "Certificate PDF has an invalid SHA-256 for {label}."
        )));
    }
    Ok(())
}

pub(super) fn validate_rendered_text(
    snapshot: &CertificatePdfSnapshot<'_>,
    config: &workflow::WorkflowConfig,
) -> Result<()> {
    let fields = snapshot.track.fields.normalized_conditionals();
    validate_track_scalar_text(snapshot, &fields)?;
    validate_track_list_text(&fields)?;
    validate_workflow_reference_text(snapshot, config)?;
    validate_evidence_text(snapshot)?;
    validate_screening_text(snapshot)?;
    Ok(())
}

fn validate_track_scalar_text(
    snapshot: &CertificatePdfSnapshot<'_>,
    fields: &TrackFields,
) -> Result<()> {
    validate_track_identity_text(snapshot, fields)?;
    validate_track_detail_text(snapshot, fields)
}

fn validate_track_identity_text(
    snapshot: &CertificatePdfSnapshot<'_>,
    fields: &TrackFields,
) -> Result<()> {
    for (label, value) in [
        ("track title", fields.title.as_str()),
        ("artist", snapshot.profile.artist_name.as_str()),
        ("Suno profile", snapshot.profile.suno_profile_name.as_str()),
        ("Suno handle", snapshot.profile.suno_handle.as_str()),
        (
            "Suno subscription start",
            snapshot.profile.subscription_start_date.as_str(),
        ),
        ("production start", fields.production_start_date.as_str()),
        ("production end", fields.production_end_date.as_str()),
        ("last editing date", fields.final_export_date.as_str()),
        ("Suno model", fields.suno_model.as_str()),
        ("Suno project URL", fields.suno_project_url.as_str()),
        (
            "Suno final generation ID",
            fields.suno_final_generation_id.as_str(),
        ),
        (
            "Suno final generation date",
            fields.suno_final_generation_date.as_str(),
        ),
        (
            "Suno download/export date",
            fields.suno_download_export_date.as_str(),
        ),
        (
            "Suno plan at generation",
            fields.suno_plan_at_generation.as_str(),
        ),
        (
            "legacy Suno plan-at-creation value",
            fields.legacy_suno_plan_at_creation.as_str(),
        ),
        (
            "Suno lyrics/structure field text",
            fields.suno_lyrics_field_text.as_str(),
        ),
        (
            "Suno other content type",
            fields.suno_lyrics_other_content_type.as_str(),
        ),
        (
            "unclassified legacy lyrics source",
            fields.lyrics_source.as_str(),
        ),
        (
            "unclassified legacy lyrics text",
            fields.lyrics_text.as_str(),
        ),
        ("Suno style prompt", fields.suno_style_prompt.as_str()),
        (
            "external audio source",
            fields.external_audio_source.as_str(),
        ),
        (
            "external audio provenance statement",
            fields.external_audio_ownership.as_str(),
        ),
        ("own audio source", fields.own_audio_source.as_str()),
        (
            "own audio provenance statement",
            fields.own_audio_ownership.as_str(),
        ),
        (
            "third-party sample source",
            fields.third_party_sample_source.as_str(),
        ),
        (
            "third-party sample provenance statement",
            fields.third_party_sample_ownership.as_str(),
        ),
        (
            "human editing details",
            fields.human_editing_details.as_str(),
        ),
        (
            "post-export editing details",
            fields.post_export_editing_details.as_str(),
        ),
    ] {
        validate_win_ansi(label, value)?;
    }
    Ok(())
}

fn validate_track_detail_text(
    snapshot: &CertificatePdfSnapshot<'_>,
    fields: &TrackFields,
) -> Result<()> {
    for (label, value) in [
        ("artwork origin", fields.artwork_origin.as_str()),
        ("AI image service", fields.ai_image_service.as_str()),
        (
            "code-audio post-processing note",
            fields.code_audio_post_processing_note.as_str(),
        ),
        (
            "human artwork process notes",
            fields.human_artwork_process_notes.as_str(),
        ),
        (
            "custom artwork change",
            fields.custom_artwork_change.as_str(),
        ),
        ("artwork disclosure text", fields.disclosure_text.as_str()),
        ("real-person note", fields.real_person_notes.as_str()),
        ("real-event note", fields.real_event_notes.as_str()),
        ("trademark/logo note", fields.trademark_notes.as_str()),
        ("audio AI system", fields.audio_ai_system.as_str()),
        (
            "audio disclosure text",
            fields.audio_disclosure_text.as_str(),
        ),
        (
            "audio disclosure reason",
            fields.audio_disclosure_reason.as_str(),
        ),
        ("release notes", fields.release_notes.as_str()),
        ("workflow ID", snapshot.track.workflow_id.as_str()),
        ("workflow version", snapshot.track.workflow_version.as_str()),
        ("certificate ID", snapshot.certificate_id),
        ("finalization timestamp", snapshot.finalized_at),
        ("certificate version", snapshot.certificate_version),
    ] {
        validate_win_ansi(label, value)?;
    }
    Ok(())
}

fn validate_track_list_text(fields: &TrackFields) -> Result<()> {
    for (label, values) in [
        (
            "code-audio post-processing operation",
            fields.code_audio_post_processing_operations.as_slice(),
        ),
        (
            "human artwork process operation",
            fields.human_artwork_process_operations.as_slice(),
        ),
        (
            "human artwork modification",
            fields.human_artwork_modifications.as_slice(),
        ),
        (
            "audio disclosure location",
            fields.audio_disclosure_locations.as_slice(),
        ),
    ] {
        for value in values {
            validate_win_ansi(label, value)?;
        }
    }
    Ok(())
}

fn validate_workflow_reference_text(
    snapshot: &CertificatePdfSnapshot<'_>,
    config: &workflow::WorkflowConfig,
) -> Result<()> {
    for step in snapshot.steps {
        if let Some(reason) = step.na_reason.as_deref() {
            validate_win_ansi("workflow N/A reason", reason)?;
        }
    }
    for step in &config.steps {
        validate_win_ansi("workflow step label", &step.name)?;
    }
    if let Some(suno_id) = snapshot.automation.suno_id.as_deref() {
        validate_win_ansi("evidence-derived Suno ID", suno_id)?;
    }
    for revision in snapshot.revision_references {
        validate_win_ansi("revision archive reference", revision)?;
    }
    Ok(())
}

fn validate_evidence_text(snapshot: &CertificatePdfSnapshot<'_>) -> Result<()> {
    for item in snapshot.evidence {
        validate_win_ansi("evidence ID", &item.id)?;
        validate_win_ansi("evidence file name", &item.file_name)?;
        validate_win_ansi("evidence relative path", &item.relative_path)?;
        validate_win_ansi("evidence imported timestamp", &item.imported_at)?;
        for (label, value) in [
            (
                "evidence original file name",
                item.metadata.original_file_name.as_str(),
            ),
            (
                "evidence document title",
                item.metadata.document_title.as_str(),
            ),
            ("evidence provider", item.metadata.provider.as_str()),
            ("evidence source URL", item.metadata.source_url.as_str()),
            (
                "evidence retrieval date",
                item.metadata.retrieval_date.as_str(),
            ),
            (
                "evidence effective date",
                item.metadata.effective_date.as_str(),
            ),
            (
                "evidence applicable production period",
                item.metadata.applicable_production_period.as_str(),
            ),
            ("evidence factual note", item.metadata.factual_note.as_str()),
            (
                "evidence file extension",
                item.metadata.file_extension.as_str(),
            ),
            ("evidence MIME type", item.metadata.mime_type.as_str()),
            ("evidence audio format", item.metadata.audio_format.as_str()),
            (
                "evidence Suno created timestamp",
                item.metadata.suno_created_timestamp.as_str(),
            ),
            (
                "evidence Suno created date",
                item.metadata.suno_created_date.as_str(),
            ),
            ("evidence Suno ID", item.metadata.suno_id.as_str()),
            (
                "evidence raw Suno metadata",
                item.metadata.suno_raw_metadata.as_str(),
            ),
        ] {
            validate_win_ansi(label, value)?;
        }
        for entry in &item.metadata.embedded_metadata {
            validate_win_ansi("embedded metadata key", &entry.key)?;
            validate_win_ansi("embedded metadata value", &entry.value)?;
        }
        for (label, value) in [
            ("evidence coverage start", item.coverage_start.as_deref()),
            ("evidence coverage end", item.coverage_end.as_deref()),
            (
                "source global evidence ID",
                item.source_global_evidence_id.as_deref(),
            ),
            (
                "derived evidence ID",
                item.derived_from_evidence_id.as_deref(),
            ),
            (
                "evidence generator version",
                item.generator_version.as_deref(),
            ),
            (
                "generated disclosure text",
                item.generated_disclosure_text.as_deref(),
            ),
        ] {
            if let Some(value) = value {
                validate_win_ansi(label, value)?;
            }
        }
    }
    Ok(())
}

fn validate_screening_text(snapshot: &CertificatePdfSnapshot<'_>) -> Result<()> {
    let screening = &snapshot.track.audio_screening;
    for (label, value) in [
        ("local screening engine", screening.local.engine.as_str()),
        (
            "local screening engine version",
            screening.local.engine_version.as_str(),
        ),
        (
            "local fingerprint algorithm",
            screening.local.fingerprint_algorithm.as_str(),
        ),
        (
            "local screening source evidence ID",
            screening.local.source_evidence_id.as_str(),
        ),
        (
            "local screening source path",
            screening.local.source_relative_path.as_str(),
        ),
        (
            "local screening record path",
            screening.local.artifact_relative_path.as_str(),
        ),
        (
            "external screening provider",
            screening.external.provider.as_str(),
        ),
        (
            "external screening source evidence ID",
            screening.external.source_evidence_id.as_str(),
        ),
        (
            "external screening source path",
            screening.external.source_relative_path.as_str(),
        ),
    ] {
        validate_win_ansi(label, value)?;
    }
    for (label, value) in [
        (
            "external screening checked timestamp",
            screening.external.checked_at.as_deref(),
        ),
        (
            "external response archive path",
            screening.external.response_relative_path.as_deref(),
        ),
    ] {
        if let Some(value) = value {
            validate_win_ansi(label, value)?;
        }
    }
    for (index, item) in screening.external.matches.iter().enumerate() {
        validate_audio_screening_match_text(item, &format!("provider match {}", index + 1))?;
    }
    for sample in &screening.external.samples {
        for (label, value) in [
            (
                "provider status message",
                sample.provider_status_message.as_deref(),
            ),
            (
                "provider API version",
                sample.provider_api_version.as_deref(),
            ),
            (
                "sample response archive",
                sample.response_relative_path.as_deref(),
            ),
        ] {
            if let Some(value) = value {
                validate_win_ansi(label, value)?;
            }
        }
        for (index, item) in sample.matches.iter().enumerate() {
            validate_audio_screening_match_text(
                item,
                &format!("sample {} provider match {}", sample.sequence, index + 1),
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_audio_screening_match_text(
    item: &crate::model::AudioScreeningMatch,
    context: &str,
) -> Result<()> {
    validate_win_ansi(&format!("{context} title"), &item.title)?;
    for artist in &item.artists {
        validate_win_ansi(&format!("{context} artist"), artist)?;
    }
    for (label, value) in [
        ("album", item.album.as_deref()),
        ("ISRC", item.isrc.as_deref()),
        ("ACRCloud ID", item.acrid.as_deref()),
    ] {
        if let Some(value) = value {
            validate_win_ansi(&format!("{context} {label}"), value)?;
        }
    }
    Ok(())
}

pub(super) fn validate_win_ansi(label: &str, value: &str) -> Result<()> {
    if let Some(character) = value.chars().find(|character| !is_win_ansi(*character)) {
        return Err(AppError::Data(format!(
            "Certificate PDF {label} contains a character unsupported by the bundled archive fonts: U+{:04X}.",
            character as u32
        )));
    }
    Ok(())
}

pub(super) fn is_win_ansi(character: char) -> bool {
    if matches!(character, '\n' | '\r' | '\t') {
        return true;
    }
    match character as u32 {
        0x20..=0x7e | 0xa0..=0xff => true,
        _ => matches!(
            character,
            '\u{20ac}'
                | '\u{201a}'
                | '\u{0192}'
                | '\u{201e}'
                | '\u{2026}'
                | '\u{2020}'
                | '\u{2021}'
                | '\u{02c6}'
                | '\u{2030}'
                | '\u{0160}'
                | '\u{2039}'
                | '\u{0152}'
                | '\u{017d}'
                | '\u{2018}'
                | '\u{2019}'
                | '\u{201c}'
                | '\u{201d}'
                | '\u{2022}'
                | '\u{2013}'
                | '\u{2014}'
                | '\u{02dc}'
                | '\u{2122}'
                | '\u{0161}'
                | '\u{203a}'
                | '\u{0153}'
                | '\u{017e}'
                | '\u{0178}'
        ),
    }
}
