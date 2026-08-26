use super::*;

pub struct ExternalAudioScreeningRequest<'a> {
    pub settings: &'a AudioScreeningSettings,
    pub credentials: Option<(&'a str, &'a str)>,
    pub source_path: &'a Path,
    pub track_id: &'a str,
    pub evidence: &'a EvidenceItem,
    pub track_root: &'a Path,
    pub local: Option<&'a AudioScreeningLocalRecord>,
}

/// Variant for the private persistence layer. Credentials are accepted only
/// transiently and are not copied to records, artifacts, error messages, or
/// returned response data.
pub fn run_external_audio_screening_with_credentials(
    request: ExternalAudioScreeningRequest<'_>,
    progress: impl FnMut(&str, &str),
) -> Result<AudioScreeningExternalRecord> {
    run_external_audio_screening_with_transport(request, progress, &UreqAcrCloudHttpTransport)
}

pub(super) fn run_external_audio_screening_with_transport(
    request: ExternalAudioScreeningRequest<'_>,
    mut progress: impl FnMut(&str, &str),
    transport: &dyn AcrCloudHttpTransport,
) -> Result<AudioScreeningExternalRecord> {
    let ExternalAudioScreeningRequest {
        settings,
        credentials,
        source_path,
        track_id,
        evidence,
        track_root,
        local,
    } = request;
    progress(
        "preparing_external_check",
        "Preparing external catalog screening",
    );
    let mut record = initialized_external_record(settings, track_id, evidence);
    if !external_run_preconditions_are_met(
        settings,
        credentials,
        track_id,
        evidence,
        track_root,
        local,
        &mut record,
    ) {
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    }
    let Some((access_key, access_secret)) = credentials else {
        // Defensive only: the configuration branch above must have returned.
        record.status = AudioScreeningStatus::SkippedNotConfigured;
        record.message = "ACRCloud access key and access secret are not configured.".into();
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    };
    let Some(prepared) =
        prepare_external_source(settings, source_path, evidence, track_root, &mut record)
    else {
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    };
    let archived_responses = ExternalSampleRunner {
        settings,
        access_key,
        access_secret,
        snapshot: &prepared.snapshot,
        parsed_wav: &prepared.parsed_wav,
        transport,
        progress: &mut progress,
        record: &mut record,
        used_ranges: Vec::new(),
        archived_responses: Vec::new(),
    }
    .run(&prepared.planned_ranges.ranges);
    finalize_external_sample_statistics(&mut record);
    finish_external_record(track_root, record, local, archived_responses, &mut progress)
}

fn initialized_external_record(
    settings: &AudioScreeningSettings,
    track_id: &str,
    evidence: &EvidenceItem,
) -> AudioScreeningExternalRecord {
    let mut record = external_record_base(track_id, evidence);
    record.screening_mode = AudioScreeningMode::MultiSample;
    record.requested_intensity_percent = settings.intensity_percent;
    record.dynamic_by_track_duration = settings.dynamic_by_track_duration;
    record.reference_duration_seconds = Some(settings.reference_duration_seconds);
    record
}

fn external_run_preconditions_are_met(
    settings: &AudioScreeningSettings,
    credentials: Option<(&str, &str)>,
    track_id: &str,
    evidence: &EvidenceItem,
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    record: &mut AudioScreeningExternalRecord,
) -> bool {
    let credentials_available = credentials
        .is_some_and(|(key, secret)| !key.trim().is_empty() && !secret.trim().is_empty());
    let (provider_status, provider_message) =
        provider_configuration_status(settings, credentials_available);
    record.provider_status = provider_status;
    if provider_status != AudioScreeningProviderStatus::Ready {
        record.status = match provider_status {
            AudioScreeningProviderStatus::Disabled
            | AudioScreeningProviderStatus::NotConfigured => {
                AudioScreeningStatus::SkippedNotConfigured
            }
            AudioScreeningProviderStatus::ConfigurationInvalid => {
                AudioScreeningStatus::ConfigurationInvalid
            }
            AudioScreeningProviderStatus::AuthenticationFailed => {
                AudioScreeningStatus::AuthenticationFailed
            }
            AudioScreeningProviderStatus::ProviderUnavailable => {
                AudioScreeningStatus::ProviderUnavailable
            }
            AudioScreeningProviderStatus::Ready => AudioScreeningStatus::NotRun,
        };
        record.message = provider_message;
        return false;
    }
    // The application performs this check before reaching the adapter, but
    // the adapter is also callable from internal code and tests.  Keep the
    // no-upload precondition at the last shared boundary so stale/copied
    // local records can never trigger an external request by bypassing UI or
    // command-layer validation.
    let local_is_current = local.is_some_and(|local| {
        local_record_matches_source(local, track_id, evidence)
            && local_artifact_is_current(track_root, local).unwrap_or(false)
    });
    if !local_is_current {
        record.status = AudioScreeningStatus::ProcessingFailed;
        record.message =
            "A current local Chromaprint fingerprint is required before external screening.".into();
        return false;
    }
    true
}

struct PreparedExternalSource {
    snapshot: VerifiedSourceSnapshot,
    parsed_wav: ParsedPcmWav,
    planned_ranges: PcmWavSamplingPlan,
}

fn prepare_external_source(
    settings: &AudioScreeningSettings,
    source_path: &Path,
    evidence: &EvidenceItem,
    track_root: &Path,
    record: &mut AudioScreeningExternalRecord,
) -> Option<PreparedExternalSource> {
    // The external sample is extracted only from a private byte-verified
    // snapshot. No bytes are uploaded until their SHA-256 and size have been
    // proven to match the authoritative release evidence.
    let snapshot = match create_verified_source_snapshot(source_path, evidence, track_root) {
        Ok(snapshot) => snapshot,
        Err(()) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message =
                "The authoritative release audio could not be verified for external screening."
                    .into();
            return None;
        }
    };
    let parsed_wav = match parse_pcm_wav(&snapshot.path) {
        Ok(parsed) => parsed,
        Err(WavSampleError::UnsupportedFormat) => {
            record.status = AudioScreeningStatus::UnsupportedFormat;
            record.message =
                "External ACRCloud screening currently supports PCM WAV release audio.".into();
            return None;
        }
        Err(_) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message = "A bounded WAV sample could not be prepared for ACRCloud.".into();
            return None;
        }
    };
    let source_duration_milliseconds =
        frames_to_milliseconds(parsed_wav.total_frames, parsed_wav.format.sample_rate);
    record.source_duration_milliseconds = Some(source_duration_milliseconds);
    let planned_ranges = match plan_pcm_wav_sample_ranges(&parsed_wav, settings) {
        Ok(plan) => plan,
        Err(_) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message = "A bounded WAV sample could not be prepared for ACRCloud.".into();
            return None;
        }
    };
    record.target_duration_milliseconds = planned_ranges.target_duration_milliseconds;
    record.planned_request_count = planned_ranges.planned_request_count;
    if planned_ranges.ranges.is_empty() {
        record.status = AudioScreeningStatus::ProcessingFailed;
        record.message =
            "No non-overlapping ACRCloud sample could be planned for this release.".into();
        return None;
    }
    Some(PreparedExternalSource {
        snapshot,
        parsed_wav,
        planned_ranges,
    })
}

struct ExternalSampleRunner<'a, F> {
    settings: &'a AudioScreeningSettings,
    access_key: &'a str,
    access_secret: &'a str,
    snapshot: &'a VerifiedSourceSnapshot,
    parsed_wav: &'a ParsedPcmWav,
    transport: &'a dyn AcrCloudHttpTransport,
    progress: &'a mut F,
    record: &'a mut AudioScreeningExternalRecord,
    used_ranges: Vec<FrameSampleRange>,
    archived_responses: Vec<PendingProviderResponse>,
}

impl<F> ExternalSampleRunner<'_, F>
where
    F: FnMut(&str, &str),
{
    fn run(mut self, ranges: &[FrameSampleRange]) -> Vec<PendingProviderResponse> {
        for (index, range) in ranges.iter().copied().enumerate() {
            if !self.run_sample(index, range) {
                break;
            }
        }
        self.archived_responses
    }

    fn run_sample(&mut self, index: usize, range: FrameSampleRange) -> bool {
        // This is deliberately immediately before request construction and
        // POST: a future planner change cannot accidentally make the network
        // path upload a duplicate, overlapping, or out-of-track range.
        if !self.range_is_available(range) {
            return false;
        }
        let Some((request, signature)) = self.prepare_sample_request(index, range) else {
            return false;
        };
        self.send_sample_request(request, &signature)
    }

    fn range_is_available(&mut self, range: FrameSampleRange) -> bool {
        if frame_range_is_available(&self.used_ranges, range, self.parsed_wav.total_frames) {
            return true;
        }
        if self.used_ranges.iter().any(|used| {
            used.start_frame == range.start_frame && used.frame_count == range.frame_count
        }) {
            self.record.duplicate_sample_count =
                self.record.duplicate_sample_count.saturating_add(1);
        } else {
            self.record.overlapping_sample_count =
                self.record.overlapping_sample_count.saturating_add(1);
        }
        self.record.status = AudioScreeningStatus::ProcessingFailed;
        self.record.message =
            "An overlapping or duplicate ACRCloud sample was rejected before upload.".into();
        false
    }

    fn extract_sample(&mut self, range: FrameSampleRange) -> Option<ExtractedWavSample> {
        let sample = match extract_pcm_wav_sample_at(
            &self.snapshot.path,
            self.parsed_wav,
            range.start_frame,
            range.frame_count,
        ) {
            Ok(sample) => sample,
            Err(WavSampleError::UnsupportedFormat) => {
                self.record.status = AudioScreeningStatus::UnsupportedFormat;
                self.record.message =
                    "External ACRCloud screening currently supports PCM WAV release audio.".into();
                return None;
            }
            Err(_) => {
                self.record.status = AudioScreeningStatus::ProcessingFailed;
                self.record.message =
                    "A bounded WAV sample could not be prepared for ACRCloud.".into();
                return None;
            }
        };
        Some(sample)
    }

    fn prepare_sample_request(
        &mut self,
        index: usize,
        range: FrameSampleRange,
    ) -> Option<(AcrCloudRequest, String)> {
        let sample = self.extract_sample(range)?;
        let end_offset_milliseconds = frames_to_milliseconds(
            range.start_frame.saturating_add(range.frame_count),
            self.parsed_wav.format.sample_rate,
        );
        let sample_record = AudioScreeningSampleRecord {
            sequence: u32::try_from(index + 1).unwrap_or(MAX_ACRCLOUD_REQUESTS),
            offset_milliseconds: sample.offset_milliseconds,
            end_offset_milliseconds,
            duration_milliseconds: end_offset_milliseconds
                .saturating_sub(sample.offset_milliseconds),
            status: AudioScreeningStatus::ProcessingFailed,
            message: "ACRCloud request was not completed.".into(),
            provider_status_code: None,
            provider_status_message: None,
            provider_api_version: None,
            matches: Vec::new(),
            response_relative_path: None,
            response_sha256: None,
        };
        (self.progress)(
            "sending_provider_request",
            "Sending bounded audio sample to ACRCloud",
        );
        let timestamp = Utc::now().timestamp().to_string();
        let (request, signature) = match build_acrcloud_request(
            self.settings,
            self.access_key,
            self.access_secret,
            &timestamp,
            &sample.bytes,
        ) {
            Ok(request) => request,
            Err(failure) => {
                self.record.status = failure.status;
                self.record.message = failure.message.into();
                return None;
            }
        };
        self.commit_sample(range, sample_record);
        Some((request, signature))
    }

    fn commit_sample(
        &mut self,
        range: FrameSampleRange,
        sample_record: AudioScreeningSampleRecord,
    ) {
        if self.record.sample_offset_milliseconds.is_none() {
            // Legacy fields remain a compatibility view of the first submitted
            // range; new consumers must use `samples` for the full run.
            self.record.sample_offset_milliseconds = Some(sample_record.offset_milliseconds);
            self.record.sample_duration_milliseconds = Some(sample_record.duration_milliseconds);
        }
        self.used_ranges.push(range);
        self.record.samples.push(sample_record);
        self.record.executed_request_count =
            u32::try_from(self.record.samples.len()).unwrap_or(MAX_ACRCLOUD_REQUESTS);
        self.record.request_count = self.record.executed_request_count;
        self.record
            .checked_at
            .get_or_insert_with(|| Utc::now().to_rfc3339());
    }

    fn send_sample_request(&mut self, request: AcrCloudRequest, signature: &str) -> bool {
        (self.progress)("waiting_provider_response", "Waiting for ACRCloud response");
        let response = match self.transport.post(request) {
            Ok(response) => response,
            Err(failure) => {
                update_last_sample_failure(self.record, failure.status, failure.message);
                self.record.status = failure.status;
                self.record.message = failure.message.into();
                return !should_stop_after_sample(failure.status);
            }
        };
        (self.progress)(
            "processing_provider_response",
            "Processing ACRCloud response",
        );
        let request_sensitive_values =
            RequestSensitiveValues::new(self.access_key, self.access_secret, signature);
        match parse_acrcloud_response(response.status, &response.body, &request_sensitive_values) {
            Ok(parsed) => {
                let status = parsed.status;
                self.apply_parsed_response(parsed);
                !should_stop_after_sample(status)
            }
            Err(failure) => {
                update_last_sample_failure(self.record, failure.status, failure.message);
                self.record.status = failure.status;
                self.record.message = failure.message.into();
                !should_stop_after_sample(failure.status)
            }
        }
    }

    fn apply_parsed_response(&mut self, parsed: ParsedAcrCloudResponse) {
        let status = parsed.status;
        let message = parsed.message;
        let provider_status_code = parsed.provider_status_code;
        let provider_status_message = parsed.provider_status_message;
        let provider_api_version = parsed.provider_api_version;
        let matches = parsed.matches;
        if let Some(sample_record) = self.record.samples.last_mut() {
            sample_record.status = status;
            sample_record.message = message.into();
            sample_record.provider_status_code = provider_status_code;
            sample_record.provider_status_message = provider_status_message;
            sample_record.provider_api_version = provider_api_version;
            sample_record.matches = matches;
        }
        if let Some(raw_response) = parsed.raw_response {
            if let Some(sample_record) = self.record.samples.last() {
                self.archived_responses.push(PendingProviderResponse {
                    sequence: sample_record.sequence,
                    offset_milliseconds: sample_record.offset_milliseconds,
                    end_offset_milliseconds: sample_record.end_offset_milliseconds,
                    duration_milliseconds: sample_record.duration_milliseconds,
                    status,
                    raw_response,
                });
            }
        }
    }
}

pub(super) struct PendingProviderResponse {
    pub(super) sequence: u32,
    pub(super) offset_milliseconds: u64,
    pub(super) end_offset_milliseconds: u64,
    pub(super) duration_milliseconds: u64,
    pub(super) status: AudioScreeningStatus,
    pub(super) raw_response: SanitizedProviderResponse,
}

pub(super) fn finish_external_record(
    track_root: &Path,
    mut record: AudioScreeningExternalRecord,
    local: Option<&AudioScreeningLocalRecord>,
    responses: Vec<PendingProviderResponse>,
    progress: &mut impl FnMut(&str, &str),
) -> Result<AudioScreeningExternalRecord> {
    let responses = if external_matches_have_finite_scores(&record) {
        responses
    } else {
        // This is a second, pre-publication barrier in addition to the parser.
        // It keeps a future parser change or direct in-module call from
        // serializing NaN/Infinity after older artifacts have been archived.
        record.status = AudioScreeningStatus::ProcessingFailed;
        record.message = "ACRCloud returned a non-finite provider score.".into();
        record.matches.clear();
        for sample in &mut record.samples {
            sample.matches.clear();
            sample.status = AudioScreeningStatus::ProcessingFailed;
            sample.message = "ACRCloud returned a non-finite provider score.".into();
        }
        finalize_external_sample_statistics(&mut record);
        Vec::new()
    };
    progress("saving_screening_result", "Saving audio-screening result");
    publish_external_screening_artifacts(track_root, local, &mut record, responses)?;
    progress("complete", "Audio screening completed");
    Ok(record)
}

pub(super) fn external_record_base(
    track_id: &str,
    evidence: &EvidenceItem,
) -> AudioScreeningExternalRecord {
    AudioScreeningExternalRecord {
        schema_version: 2,
        provider: ACRCLOUD_PROVIDER.into(),
        status: AudioScreeningStatus::NotRun,
        message: "No external catalog screening has been run.".into(),
        track_id: track_id.to_owned(),
        source_evidence_id: evidence.id.clone(),
        source_relative_path: evidence.relative_path.clone(),
        source_sha256: evidence.sha256.clone().unwrap_or_default(),
        source_size_bytes: evidence.size_bytes,
        checked_at: None,
        screening_mode: AudioScreeningMode::MultiSample,
        requested_intensity_percent: 5,
        dynamic_by_track_duration: true,
        reference_duration_seconds: None,
        target_duration_milliseconds: 0,
        planned_request_count: 0,
        executed_request_count: 0,
        unique_sample_count: 0,
        overlapping_sample_count: 0,
        duplicate_sample_count: 0,
        unique_sample_duration_milliseconds: 0,
        track_coverage_percent: 0.0,
        provider_status: AudioScreeningProviderStatus::Disabled,
        samples: Vec::new(),
        sample_offset_milliseconds: None,
        sample_duration_milliseconds: None,
        source_duration_milliseconds: None,
        request_count: 0,
        response_relative_path: None,
        response_sha256: None,
        matches: Vec::new(),
        configured_at_snapshot: None,
    }
}
