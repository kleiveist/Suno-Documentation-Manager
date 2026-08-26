use super::*;

/// The certificate's K.2 section intentionally contains a concise, factual
/// screening summary. It never displays a raw local fingerprint, raw provider
/// response, request signature, or credential.
pub(super) fn audio_screening_markdown(state: &AudioScreeningState) -> String {
    let local = &state.local;
    let external = &state.external;
    let multi_sample = has_multi_sample_data(external);
    let legacy_sample_block = if multi_sample {
        String::new()
    } else {
        format!(
            "- External sample offset (ms) [System value]: {}\n- External sample duration (ms) [System value]: {}\n",
            external
                .sample_offset_milliseconds
                .map(|value| value.to_string())
                .unwrap_or_else(|| "NOT DOCUMENTED".into()),
            external
                .sample_duration_milliseconds
                .map(|value| value.to_string())
                .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        )
    };
    let mut output = format!(
        "- Local screening status [System verification]: **{}**\n- Local engine [System verification]: {}\n- Local engine version [System verification]: {}\n- Fingerprint algorithm [System verification]: {}\n- Local source Evidence ID [System verification]: {}\n- Local source path [System verification]: `{}`\n- Local source SHA-256 [System verification]: `{}`\n- Local source size (bytes) [System verification]: {}\n- Local measured duration (ms) [System verification]: {}\n- Local record path [System verification]: `{}`\n- Local record SHA-256 [System verification]: `{}`\n- Local generated at [System value]: {}\n\n- External screening provider [System value]: {}\n- External screening status [System verification]: **{}**\n- External provider configured at snapshot [System value]: {}\n- External source Evidence ID [System verification]: {}\n- External source path [System verification]: `{}`\n- External source SHA-256 [System verification]: `{}`\n- External checked at [System value]: {}\n{legacy_sample_block}- External source duration (ms) [System value]: {}\n- External request count [System value]: {}\n- External response archive [System verification]: `{}`\n- External response SHA-256 [System verification]: `{}`\n",
        audio_screening_status_label(local.status),
        markdown_documented(&local.engine),
        markdown_documented(&local.engine_version),
        markdown_documented(&local.fingerprint_algorithm),
        markdown_documented(&local.source_evidence_id),
        markdown_documented(&local.source_relative_path),
        markdown_documented(&local.source_sha256),
        local.source_size_bytes,
        local
            .duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        markdown_documented(&local.artifact_relative_path),
        markdown_documented(&local.artifact_sha256),
        markdown_optional_value(local.generated_at.as_deref(), "NOT DOCUMENTED"),
        markdown_documented(&external.provider),
        audio_screening_status_label(external.status),
        recorded_bool(external.configured_at_snapshot),
        markdown_documented(&external.source_evidence_id),
        markdown_documented(&external.source_relative_path),
        markdown_documented(&external.source_sha256),
        markdown_optional_value(external.checked_at.as_deref(), "NOT DOCUMENTED"),
        external
            .source_duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external.request_count,
        markdown_optional_value(external.response_relative_path.as_deref(), "NOT RECORDED"),
        markdown_optional_value(external.response_sha256.as_deref(), "NOT RECORDED"),
    );

    output.push_str(&multi_sample_markdown_block(external));

    if external.matches.is_empty() {
        output.push_str("- Provider matches [Provider-derived metadata]: NONE RECORDED\n");
    } else {
        for (index, item) in external.matches.iter().enumerate() {
            let artists = if item.artists.is_empty() {
                "NOT DOCUMENTED".to_owned()
            } else {
                item.artists.join(", ")
            };
            let mut value = format!("{} — {artists}", documented(&item.title));
            if let Some(album) = item
                .album
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; album {album}"));
            }
            if let Some(isrc) = item
                .isrc
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; ISRC {isrc}"));
            }
            if let Some(acrid) = item
                .acrid
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; ACRID {acrid}"));
            }
            if let Some(score) = item.score {
                value.push_str(&format!("; score {score}"));
            }
            output.push_str(&format!(
                "- Provider match {} [Provider-derived metadata]: {}\n",
                index + 1,
                markdown_raw_value(&value),
            ));
        }
    }
    output.push_str("\nAudio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion.\n");
    output
}

pub(super) fn multi_sample_markdown_block(external: &AudioScreeningExternalRecord) -> String {
    if !has_multi_sample_data(external) {
        return String::new();
    }

    let calculation_mode = if external.dynamic_by_track_duration {
        "DYNAMIC BY TRACK DURATION"
    } else {
        "FIXED REFERENCE DURATION"
    };
    let reference_duration = external
        .reference_duration_seconds
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".into());
    let mut output = format!(
        "\n- External screening mode [System value]: **{}**\n- Requested coverage [System value]: {} %\n- Calculation mode [System value]: {}\n- Reference duration (seconds) [System value]: {}\n- Target screening duration (ms) [System value]: {}\n- Planned requests [System value]: {}\n- Executed requests [System verification]: {}\n- Unique samples [System verification]: {}\n- Duplicate samples [System verification]: {}\n- Overlapping samples [System verification]: {}\n- Unique sampled duration (ms) [System verification]: {}\n- Track coverage (%) [System verification]: {:.2}\n- ACRCloud provider status [System verification]: {}\n- Overall result [System verification]: **{}**\n",
        external.screening_mode.as_str(),
        external.requested_intensity_percent,
        calculation_mode,
        reference_duration,
        external.target_duration_milliseconds,
        external.planned_request_count,
        external.executed_request_count,
        external.unique_sample_count,
        external.duplicate_sample_count,
        external.overlapping_sample_count,
        external.unique_sample_duration_milliseconds,
        external.track_coverage_percent,
        audio_screening_provider_status_label(external.provider_status),
        audio_screening_status_label(external.status),
    );

    if external.samples.is_empty() {
        output.push_str("- Sample results [System verification]: NONE RECORDED\n");
        return output;
    }

    for sample in &external.samples {
        let response_archive =
            markdown_optional_value(sample.response_relative_path.as_deref(), "NOT RECORDED");
        let response_sha256 =
            markdown_optional_value(sample.response_sha256.as_deref(), "NOT RECORDED");
        output.push_str(&format!(
            "- Sample {:02} [System verification]: Offset {} ms · End offset {} ms · Duration {} ms · Result {}{} · Response archive `{}` · Response SHA-256 `{}`\n",
            sample.sequence,
            sample.offset_milliseconds,
            sample.end_offset_milliseconds,
            sample.duration_milliseconds,
            sample_result_label(sample.status),
            markdown_provider_status_details(sample)
                .map(|details| format!(" · {details}"))
                .unwrap_or_default(),
            response_archive,
            response_sha256,
        ));
        for (index, item) in sample.matches.iter().enumerate() {
            output.push_str(&format!(
                "- Sample {:02} Provider match {} [Provider-derived metadata]: {}\n",
                sample.sequence,
                index + 1,
                markdown_raw_value(&audio_screening_match_summary(item)),
            ));
        }
    }
    output
}

pub(super) fn markdown_provider_status_details(
    sample: &crate::model::AudioScreeningSampleRecord,
) -> Option<String> {
    let mut details = Vec::new();
    if let Some(code) = sample.provider_status_code {
        details.push(format!("Provider Code: {code}"));
    }
    if let Some(message) = sample
        .provider_status_message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        details.push(format!("Provider Message: {}", markdown_raw_value(message)));
    }
    if let Some(version) = sample
        .provider_api_version
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        details.push(format!("Provider Version: {}", markdown_raw_value(version)));
    }
    (!details.is_empty()).then(|| details.join(" · "))
}

pub(super) fn has_multi_sample_data(external: &AudioScreeningExternalRecord) -> bool {
    external.screening_mode == AudioScreeningMode::MultiSample || !external.samples.is_empty()
}

pub(super) fn audio_screening_match_summary(item: &crate::model::AudioScreeningMatch) -> String {
    let artists = if item.artists.is_empty() {
        "NOT DOCUMENTED".to_owned()
    } else {
        item.artists.join(", ")
    };
    let mut value = format!("{} — {artists}", documented(&item.title));
    if let Some(album) = item
        .album
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        value.push_str(&format!("; album {album}"));
    }
    if let Some(isrc) = item
        .isrc
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        value.push_str(&format!("; ISRC {isrc}"));
    }
    if let Some(acrid) = item
        .acrid
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        value.push_str(&format!("; ACRID {acrid}"));
    }
    if let Some(score) = item.score {
        value.push_str(&format!("; score {score}"));
    }
    value
}

pub(super) fn sample_result_label(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::NoMatchDetected => "NO MATCH",
        _ => audio_screening_status_label(status),
    }
}

pub(super) fn audio_screening_provider_status_label(
    status: AudioScreeningProviderStatus,
) -> &'static str {
    match status {
        AudioScreeningProviderStatus::Disabled => "DISABLED",
        AudioScreeningProviderStatus::NotConfigured => "NOT CONFIGURED",
        AudioScreeningProviderStatus::Ready => "READY",
        AudioScreeningProviderStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        AudioScreeningProviderStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        AudioScreeningProviderStatus::ConfigurationInvalid => "CONFIGURATION INVALID",
    }
}

pub(super) fn audio_screening_status_label(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::NotRun => "NOT RUN",
        AudioScreeningStatus::FingerprintGenerated => "FINGERPRINT GENERATED",
        AudioScreeningStatus::NoMatchDetected => "NO MATCH DETECTED",
        AudioScreeningStatus::MatchDetected => "MATCH DETECTED",
        AudioScreeningStatus::SkippedNotConfigured => "SKIPPED NOT CONFIGURED",
        AudioScreeningStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        AudioScreeningStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        AudioScreeningStatus::ConfigurationInvalid => "CONFIGURATION INVALID",
        AudioScreeningStatus::EngineUnavailable => "ENGINE UNAVAILABLE",
        AudioScreeningStatus::UnsupportedFormat => "UNSUPPORTED FORMAT",
        AudioScreeningStatus::ProcessingFailed => "PROCESSING FAILED",
        AudioScreeningStatus::Stale => "STALE",
    }
}
