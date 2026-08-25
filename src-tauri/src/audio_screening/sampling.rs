use super::*;

pub(super) const MIN_ACRCLOUD_INTENSITY_PERCENT: u8 = 1;
pub(super) const MAX_ACRCLOUD_INTENSITY_PERCENT: u8 = 100;
pub(super) const MAX_ACRCLOUD_REFERENCE_DURATION_SECONDS: u64 = 86_400;

/// Validates the non-secret coverage controls before they are persisted. The
/// execution planner repeats its hard request and range limits independently,
/// so a stale or hand-edited settings row can never increase provider usage.
pub fn validate_acrcloud_sampling_settings(settings: &AudioScreeningSettings) -> Result<()> {
    if !(MIN_ACRCLOUD_INTENSITY_PERCENT..=MAX_ACRCLOUD_INTENSITY_PERCENT)
        .contains(&settings.intensity_percent)
    {
        return Err(AppError::Validation(
            "ACRCloud screening intensity must be between 1 and 100 percent.".into(),
        ));
    }
    if settings.reference_duration_seconds == 0
        || settings.reference_duration_seconds > MAX_ACRCLOUD_REFERENCE_DURATION_SECONDS
    {
        return Err(AppError::Validation(
            "The ACRCloud reference duration must be between 1 second and 24 hours.".into(),
        ));
    }
    Ok(())
}

/// Plans deterministic, non-overlapping millisecond ranges for a release.
/// This has no file-system or network side effects and is shared conceptually
/// by the PCM-frame planner used at upload time. Ranges are anchored at the
/// middle for a single request and at both ends (with equal spacing) for two
/// or more requests.
#[cfg(test)]
pub fn plan_acrcloud_sample_ranges(
    track_duration_milliseconds: u64,
    settings: &AudioScreeningSettings,
) -> AcrCloudSamplingPlan {
    let intensity = settings.intensity_percent.clamp(
        MIN_ACRCLOUD_INTENSITY_PERCENT,
        MAX_ACRCLOUD_INTENSITY_PERCENT,
    );
    let target_duration_milliseconds = requested_target_duration(
        track_duration_milliseconds,
        u64::from(intensity),
        settings.dynamic_by_track_duration,
        settings.reference_duration_seconds.saturating_mul(1_000),
    );
    let full_sample_milliseconds = MAX_ACRCLOUD_SAMPLE_SECONDS.saturating_mul(1_000);
    let (requested_request_count, planned_request_count) = planned_request_counts(
        track_duration_milliseconds,
        target_duration_milliseconds,
        full_sample_milliseconds,
    );
    let sample_lengths = planned_sample_unit_lengths(
        track_duration_milliseconds,
        target_duration_milliseconds,
        full_sample_milliseconds,
        requested_request_count,
        planned_request_count,
    );
    let samples =
        evenly_distributed_millisecond_ranges(track_duration_milliseconds, &sample_lengths);
    AcrCloudSamplingPlan {
        target_duration_milliseconds,
        requested_request_count,
        planned_request_count,
        maximum_unique_duration_milliseconds: sample_lengths
            .iter()
            .copied()
            .sum::<u64>()
            .min(MAX_ACRCLOUD_UNIQUE_SAMPLE_SECONDS.saturating_mul(1_000)),
        samples,
    }
}

pub(super) fn requested_target_duration(
    track_duration: u64,
    intensity_percent: u64,
    dynamic_by_track_duration: bool,
    reference_duration: u64,
) -> u64 {
    if track_duration == 0 || intensity_percent == 0 {
        return 0;
    }
    let basis = if dynamic_by_track_duration {
        track_duration
    } else {
        reference_duration
    };
    ceil_percentage(basis, intensity_percent).min(track_duration)
}

pub(super) fn ceil_percentage(value: u64, percentage: u64) -> u64 {
    let product = u128::from(value).saturating_mul(u128::from(percentage));
    let rounded = product.saturating_add(99) / 100;
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

pub(super) fn planned_request_counts(
    track_duration: u64,
    target_duration: u64,
    full_sample_duration: u64,
) -> (u32, u32) {
    if track_duration == 0 || target_duration == 0 || full_sample_duration == 0 {
        return (0, 0);
    }
    let requested = ceil_div_u64(target_duration, full_sample_duration);
    let requested = u32::try_from(requested).unwrap_or(u32::MAX);
    // The explicit short-track exception is intentionally evaluated before
    // floor(track/12s), which is zero for every non-empty sub-12s release.
    let capacity = if track_duration < full_sample_duration {
        1
    } else {
        u32::try_from(track_duration / full_sample_duration).unwrap_or(u32::MAX)
    };
    (
        requested,
        requested.min(capacity).min(MAX_ACRCLOUD_REQUESTS),
    )
}

/// Builds exact requested coverage where possible: every range is a full
/// twelve seconds except the final target remainder. If the non-overlap slot
/// cap reduces the number of requests, all available slots remain full and
/// the recorded unique duration truthfully shows the reduced coverage.
pub(super) fn planned_sample_unit_lengths(
    track_duration: u64,
    target_duration: u64,
    full_sample_duration: u64,
    requested_count: u32,
    planned_count: u32,
) -> Vec<u64> {
    if planned_count == 0 || track_duration == 0 || full_sample_duration == 0 {
        return Vec::new();
    }
    if track_duration < full_sample_duration {
        // Preserve the useful legacy short-track behaviour: one request can
        // safely carry the whole (sub-12-second) release. The requested
        // target remains recorded separately, while actual coverage reports
        // the intentional full short-track sample.
        return vec![track_duration];
    }
    let mut lengths = vec![full_sample_duration; planned_count as usize];
    if planned_count == requested_count {
        let preceding = full_sample_duration.saturating_mul(u64::from(planned_count - 1));
        let final_length = target_duration.saturating_sub(preceding);
        if final_length > 0 && final_length <= full_sample_duration {
            if let Some(last) = lengths.last_mut() {
                *last = final_length;
            }
        }
    }
    lengths
}

pub(super) fn ceil_div_u64(numerator: u64, denominator: u64) -> u64 {
    numerator / denominator + u64::from(!numerator.is_multiple_of(denominator))
}

#[cfg(test)]
pub(super) fn evenly_distributed_millisecond_ranges(
    track_duration: u64,
    sample_lengths: &[u64],
) -> Vec<AcrCloudSampleRange> {
    evenly_distributed_unit_ranges(track_duration, sample_lengths)
        .into_iter()
        .map(|range| AcrCloudSampleRange {
            offset_milliseconds: range.start_frame,
            end_offset_milliseconds: range.start_frame.saturating_add(range.frame_count),
            duration_milliseconds: range.frame_count,
        })
        .collect()
}

pub(super) fn evenly_distributed_unit_ranges(
    total_units: u64,
    sample_lengths: &[u64],
) -> Vec<FrameSampleRange> {
    if sample_lengths.is_empty() || total_units == 0 || sample_lengths.contains(&0) {
        return Vec::new();
    }
    let sample_total = sample_lengths
        .iter()
        .try_fold(0_u64, |total, length| total.checked_add(*length));
    let Some(sample_total) = sample_total else {
        return Vec::new();
    };
    if sample_total > total_units {
        return Vec::new();
    }
    if sample_lengths.len() == 1 {
        let sample_units = sample_lengths[0];
        return vec![FrameSampleRange {
            start_frame: (total_units - sample_units) / 2,
            frame_count: sample_units,
        }];
    }
    // Start at the beginning and finish at the end. Distribute the remaining
    // space only between samples with integer Bresenham-style gaps, so every
    // run is deterministic, range-safe, and as evenly spread as possible.
    let gap_total = total_units - sample_total;
    let gap_count = u64::try_from(sample_lengths.len() - 1).unwrap_or(u64::MAX);
    let mut start = 0_u64;
    sample_lengths
        .iter()
        .enumerate()
        .map(|(index, &length)| {
            let range = FrameSampleRange {
                start_frame: start,
                frame_count: length,
            };
            if index + 1 < sample_lengths.len() {
                let before = u64::try_from(index).unwrap_or(u64::MAX);
                let after = before.saturating_add(1);
                let gap = ((u128::from(after) * u128::from(gap_total)) / u128::from(gap_count))
                    .saturating_sub(
                        (u128::from(before) * u128::from(gap_total)) / u128::from(gap_count),
                    );
                start = start
                    .saturating_add(length)
                    .saturating_add(u64::try_from(gap).unwrap_or(u64::MAX));
            }
            range
        })
        .collect()
}

#[derive(Debug)]
pub(super) struct PcmWavSamplingPlan {
    pub(super) target_duration_milliseconds: u64,
    pub(super) planned_request_count: u32,
    pub(super) ranges: Vec<FrameSampleRange>,
}

pub(super) fn plan_pcm_wav_sample_ranges(
    parsed: &ParsedPcmWav,
    settings: &AudioScreeningSettings,
) -> std::result::Result<PcmWavSamplingPlan, WavSampleError> {
    let full_sample_frames = u64::from(parsed.format.sample_rate)
        .checked_mul(MAX_ACRCLOUD_SAMPLE_SECONDS)
        .ok_or(WavSampleError::InvalidAudio)?;
    let intensity = u64::from(settings.intensity_percent.clamp(
        MIN_ACRCLOUD_INTENSITY_PERCENT,
        MAX_ACRCLOUD_INTENSITY_PERCENT,
    ));
    let reference_frames = u64::try_from(
        u128::from(settings.reference_duration_seconds)
            .saturating_mul(u128::from(parsed.format.sample_rate)),
    )
    .unwrap_or(u64::MAX);
    let target_frames = requested_target_duration(
        parsed.total_frames,
        intensity,
        settings.dynamic_by_track_duration,
        reference_frames,
    );
    let (requested_count, planned_count) =
        planned_request_counts(parsed.total_frames, target_frames, full_sample_frames);
    let max_frames = max_pcm_sample_frames(parsed)?;
    if max_frames == 0 {
        return Err(WavSampleError::InvalidAudio);
    }
    let desired_lengths = planned_sample_unit_lengths(
        parsed.total_frames,
        target_frames,
        full_sample_frames,
        requested_count,
        planned_count,
    );
    let actual_lengths = desired_lengths
        .into_iter()
        .map(|length| length.min(max_frames))
        .collect::<Vec<_>>();
    let ranges = evenly_distributed_unit_ranges(parsed.total_frames, &actual_lengths);
    if ranges.len() != planned_count as usize
        || !frame_ranges_are_non_overlapping(&ranges, parsed.total_frames)
    {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(PcmWavSamplingPlan {
        target_duration_milliseconds: frames_to_milliseconds(
            target_frames,
            parsed.format.sample_rate,
        ),
        planned_request_count: planned_count,
        ranges,
    })
}

pub(super) fn frame_range_is_available(
    used: &[FrameSampleRange],
    candidate: FrameSampleRange,
    total_frames: u64,
) -> bool {
    candidate.frame_count > 0
        && candidate
            .start_frame
            .checked_add(candidate.frame_count)
            .is_some_and(|end| end <= total_frames)
        && !used
            .iter()
            .any(|range| frame_ranges_overlap_or_duplicate(*range, candidate))
}

pub(super) fn frame_ranges_overlap_or_duplicate(
    left: FrameSampleRange,
    right: FrameSampleRange,
) -> bool {
    let left_end = left.start_frame.saturating_add(left.frame_count);
    let right_end = right.start_frame.saturating_add(right.frame_count);
    left.start_frame == right.start_frame
        || (left.start_frame < right_end && right.start_frame < left_end)
}

pub(super) fn frame_ranges_are_non_overlapping(
    ranges: &[FrameSampleRange],
    total_frames: u64,
) -> bool {
    ranges.iter().enumerate().all(|(index, range)| {
        range.frame_count > 0
            && range
                .start_frame
                .checked_add(range.frame_count)
                .is_some_and(|end| end <= total_frames)
            && ranges[..index]
                .iter()
                .all(|previous| !frame_ranges_overlap_or_duplicate(*previous, *range))
    })
}

pub(super) fn update_last_sample_failure(
    record: &mut AudioScreeningExternalRecord,
    status: AudioScreeningStatus,
    message: &str,
) {
    if let Some(sample) = record.samples.last_mut() {
        sample.status = status;
        sample.message = message.into();
        sample.matches.clear();
    }
}

pub(super) fn should_stop_after_sample(status: AudioScreeningStatus) -> bool {
    matches!(
        status,
        AudioScreeningStatus::AuthenticationFailed
            | AudioScreeningStatus::ConfigurationInvalid
            | AudioScreeningStatus::ProviderUnavailable
    )
}

pub(super) fn finalize_external_sample_statistics(record: &mut AudioScreeningExternalRecord) {
    record.executed_request_count =
        u32::try_from(record.samples.len()).unwrap_or(MAX_ACRCLOUD_REQUESTS);
    record.request_count = record.executed_request_count;
    record.unique_sample_count = record.executed_request_count;
    record.unique_sample_duration_milliseconds =
        record.samples.iter().fold(0_u64, |total, sample| {
            total.saturating_add(sample.duration_milliseconds)
        });
    let source_duration = record.source_duration_milliseconds.unwrap_or_default();
    record.track_coverage_percent = if source_duration == 0 {
        0.0
    } else {
        (record.unique_sample_duration_milliseconds as f64 * 100.0) / source_duration as f64
    };
    record.matches = record
        .samples
        .iter()
        .flat_map(|sample| sample.matches.iter().cloned())
        .collect();
    if record.samples.is_empty() {
        if record.status == AudioScreeningStatus::NotRun {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message = "No ACRCloud request was executed for this release.".into();
        }
        record.provider_status = provider_status_for_result(record.provider_status, record.status);
        return;
    }
    let has_match = record
        .samples
        .iter()
        .any(|sample| sample.status == AudioScreeningStatus::MatchDetected);
    let all_no_match = record
        .samples
        .iter()
        .all(|sample| sample.status == AudioScreeningStatus::NoMatchDetected);
    if has_match {
        record.status = AudioScreeningStatus::MatchDetected;
        record.message = if all_samples_completed(record) {
            "ACRCloud returned one or more catalog matches for the submitted audio samples.".into()
        } else {
            "ACRCloud returned one or more catalog matches; not every planned sample completed."
                .into()
        };
    } else if all_no_match {
        record.status = AudioScreeningStatus::NoMatchDetected;
        record.message =
            "ACRCloud returned no catalog match for the submitted audio samples.".into();
    } else {
        let failure = record
            .samples
            .iter()
            .find(|sample| sample.status != AudioScreeningStatus::NoMatchDetected)
            .map(|sample| sample.status)
            .unwrap_or(AudioScreeningStatus::ProcessingFailed);
        record.status = failure;
        record.message =
            "ACRCloud screening did not complete successfully for every submitted sample.".into();
    }
    record.provider_status = provider_status_for_result(record.provider_status, record.status);
}

pub(super) fn all_samples_completed(record: &AudioScreeningExternalRecord) -> bool {
    record.executed_request_count == record.planned_request_count
        && record.samples.iter().all(|sample| {
            matches!(
                sample.status,
                AudioScreeningStatus::NoMatchDetected | AudioScreeningStatus::MatchDetected
            )
        })
}

pub(super) fn provider_status_for_result(
    current: AudioScreeningProviderStatus,
    status: AudioScreeningStatus,
) -> AudioScreeningProviderStatus {
    match status {
        AudioScreeningStatus::AuthenticationFailed => {
            AudioScreeningProviderStatus::AuthenticationFailed
        }
        AudioScreeningStatus::ProviderUnavailable => {
            AudioScreeningProviderStatus::ProviderUnavailable
        }
        AudioScreeningStatus::ConfigurationInvalid => {
            AudioScreeningProviderStatus::ConfigurationInvalid
        }
        _ => current,
    }
}
