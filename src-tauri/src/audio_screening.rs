//! Narrow, native implementation of the two pre-release audio-screening
//! operations.  Chromaprint stays completely local; the ACRCloud path is only
//! entered by an explicit caller after it has obtained the user's credentials
//! from the private configuration store.
//!
//! This module deliberately does not decide anything about copyright, licence,
//! authorship, originality, or legal safety.  Its statuses only describe a
//! local acoustic fingerprint or a factual provider response for one bounded
//! audio sample.

use crate::error::{AppError, Result};
use crate::model::{
    AudioScreeningExternalRecord, AudioScreeningLocalRecord, AudioScreeningMatch,
    AudioScreeningMode, AudioScreeningProviderStatus, AudioScreeningProviderTestResult,
    AudioScreeningSampleRecord, AudioScreeningSettings, AudioScreeningState, AudioScreeningStatus,
    EvidenceItem,
};
use crate::security::{
    atomic_write, contained_path, ensure_contained_directory, sha256_bytes, sha256_file,
    validate_relative,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::Value;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::{Builder as TempfileBuilder, TempDir};
use url::Url;
use uuid::Uuid;

pub const AUDIO_SCREENING_DIR: &str = "03_DOCUMENTATION/AUDIO_SCREENING";
pub const LOCAL_FINGERPRINT_FILE: &str = "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json";
/// A detached digest is used deliberately: a JSON document cannot contain the
/// digest of its own final bytes without a hash cycle.
pub const LOCAL_FINGERPRINT_HASH_FILE: &str =
    "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.sha256";
pub const EXTERNAL_SCREENING_FILE: &str =
    "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_SCREENING.json";
pub const ACRCLOUD_RESPONSE_FILE: &str = "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE.json";
pub const AUDIO_SCREENING_MARKDOWN_FILE: &str =
    "03_DOCUMENTATION/AUDIO_SCREENING/AUDIO_SCREENING.md";

pub const CHROMAPRINT_ENGINE: &str = "chromaprint";
pub const CHROMAPRINT_VERSION: &str = "1.6.1";
pub const FINGERPRINT_ALGORITHM: &str = "2";
pub const ACRCLOUD_PROVIDER: &str = "ACRCloud";

const FPCALC_TIMEOUT_SECONDS: u64 = 90;
const FPCALC_STDOUT_LIMIT: usize = 1024 * 1024;
const FPCALC_STDERR_LIMIT: usize = 64 * 1024;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_SAMPLE_AUDIO_BYTES: u64 = 4 * 1024 * 1024;
/// ACRCloud accepts at most twelve seconds in one identification request.
pub const MAX_ACRCLOUD_SAMPLE_SECONDS: u64 = 12;
/// No release can cause more than this many provider requests.
pub const MAX_ACRCLOUD_REQUESTS: u32 = 25;
pub const MAX_ACRCLOUD_UNIQUE_SAMPLE_SECONDS: u64 =
    MAX_ACRCLOUD_SAMPLE_SECONDS * MAX_ACRCLOUD_REQUESTS as u64;
const MAX_SAMPLE_SECONDS: u64 = MAX_ACRCLOUD_SAMPLE_SECONDS;
const MAX_RIFF_CHUNKS: usize = 4_096;
const MAX_PROVIDER_MATCHES: usize = 20;
const MAX_PROVIDER_TEXT_BYTES: usize = 512;
/// `build_pcm_wav` emits a 44-byte RIFF/PCM header and may add one byte of
/// padding for odd-sized data chunks.  Keep both below the provider upload
/// cap, rather than treating the cap as a raw PCM-only limit.
const PCM_WAV_HEADER_BYTES: u64 = 44;
const PCM_WAV_MAX_PADDING_BYTES: u64 = 1;
const MAX_PROVIDER_RESPONSE_STRING_DECODE_DEPTH: usize = 8;

type HmacSha1 = Hmac<Sha1>;

/// A bounded, standalone WAV sample generated only in memory.  It contains no
/// original metadata chunks and is never written into the track as evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedWavSample {
    pub bytes: Vec<u8>,
    pub offset_milliseconds: u64,
    pub duration_milliseconds: u64,
    pub source_duration_milliseconds: u64,
}

/// A deterministic, track-relative range selected for one ACRCloud request.
/// The public millisecond variant is useful for previews and tests; the WAV
/// adapter plans the same ranges in PCM frames before it reads audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrCloudSampleRange {
    pub offset_milliseconds: u64,
    pub end_offset_milliseconds: u64,
    pub duration_milliseconds: u64,
}

/// Explanation of deterministic sample planning. `planned_request_count` is
/// already constrained by the hard 25-request limit and by the number of
/// non-overlapping twelve-second portions in the track. A short non-empty
/// track is the sole exception and receives one correspondingly short range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrCloudSamplingPlan {
    pub target_duration_milliseconds: u64,
    pub requested_request_count: u32,
    pub planned_request_count: u32,
    pub maximum_unique_duration_milliseconds: u64,
    pub samples: Vec<AcrCloudSampleRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameSampleRange {
    start_frame: u64,
    frame_count: u64,
}

/// Error categories deliberately stay non-diagnostic: callers turn them into
/// a controlled technical status and never expose arbitrary decoder messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavSampleError {
    UnsupportedFormat,
    InvalidAudio,
    Io,
}

#[derive(Debug, Clone)]
struct ProviderFailure {
    status: AudioScreeningStatus,
    message: &'static str,
}

#[derive(Debug, Clone)]
struct AcrCloudRequest {
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout_seconds: u32,
}

#[derive(Debug, Clone)]
struct AcrCloudResponse {
    status: u16,
    body: Vec<u8>,
}

/// Request-sensitive values remain in memory only for the duration of one
/// explicit provider call.  They are used solely to reject a provider echo
/// before that response can enter a record, artifact, or public DTO.
///
/// Do not derive `Debug`: accidental logging of this small guard must not be
/// able to reveal the access secret.
struct RequestSensitiveValues<'a> {
    access_key: &'a str,
    access_secret: &'a str,
    signature: &'a str,
}

impl<'a> RequestSensitiveValues<'a> {
    fn new(access_key: &'a str, access_secret: &'a str, signature: &'a str) -> Self {
        Self {
            access_key,
            access_secret,
            signature,
        }
    }

    fn occurs_in(&self, text: &str) -> bool {
        // The request itself uses the original key/secret.  Checking trimmed
        // variants as well catches a provider that strips incidental outer
        // whitespace before echoing an otherwise valid credential.
        [
            self.access_key,
            self.access_key.trim(),
            self.access_secret,
            self.access_secret.trim(),
            self.signature,
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .any(|value| text.contains(value))
    }
}

/// A response reaches this type only after it has been parsed and checked
/// against the exact request key, secret, and signature.  Keeping the bytes
/// opaque makes it impossible for a normal call path to accidentally publish
/// unvetted provider output.
struct SanitizedProviderResponse(Vec<u8>);

impl SanitizedProviderResponse {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Kept intentionally small so request construction and response parsing can
/// be tested without a real network or an ACRCloud account.
trait AcrCloudHttpTransport {
    fn post(
        &self,
        request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure>;
    fn get(
        &self,
        request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure>;
}

struct UreqAcrCloudHttpTransport;

impl AcrCloudHttpTransport for UreqAcrCloudHttpTransport {
    fn post(
        &self,
        request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        execute_ureq_request("POST", request)
    }

    fn get(
        &self,
        request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        execute_ureq_request("GET", request)
    }
}

fn execute_ureq_request(
    method: &str,
    request: AcrCloudRequest,
) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
    let timeout = Duration::from_secs(u64::from(request.timeout_seconds.max(1)));
    // Redirects are intentionally disabled.  A validated ACRCloud host is the
    // sole destination of this provider adapter.
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .redirects(0)
        .build();
    let mut outgoing = match method {
        "GET" => agent.get(&request.url),
        _ => agent.post(&request.url),
    };
    for (name, value) in request.headers {
        outgoing = outgoing.set(&name, &value);
    }
    let response = if method == "GET" {
        outgoing.call()
    } else {
        outgoing.send_bytes(&request.body)
    };
    match response {
        Ok(response) => read_acrcloud_response(response),
        Err(ureq::Error::Status(_, response)) => read_acrcloud_response(response),
        Err(ureq::Error::Transport(_)) => Err(ProviderFailure {
            status: AudioScreeningStatus::ProviderUnavailable,
            message: "ACRCloud could not be reached.",
        }),
    }
}

fn read_acrcloud_response(
    response: ureq::Response,
) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
    let status = response.status() as u16;
    let mut reader = response
        .into_reader()
        .take((MAX_PROVIDER_RESPONSE_BYTES + 1) as u64);
    let mut body = Vec::new();
    reader.read_to_end(&mut body).map_err(|_| ProviderFailure {
        status: AudioScreeningStatus::ProviderUnavailable,
        message: "The ACRCloud response could not be read.",
    })?;
    if body.len() > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The ACRCloud response exceeds the supported size limit.",
        });
    }
    Ok(AcrCloudResponse { status, body })
}

/// Human-readable, technical labels for status chips and portable Markdown.
pub fn audio_screening_status_label(status: AudioScreeningStatus) -> &'static str {
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

const MIN_ACRCLOUD_INTENSITY_PERCENT: u8 = 1;
const MAX_ACRCLOUD_INTENSITY_PERCENT: u8 = 100;
const MAX_ACRCLOUD_REFERENCE_DURATION_SECONDS: u64 = 86_400;

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

fn requested_target_duration(
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

fn ceil_percentage(value: u64, percentage: u64) -> u64 {
    let product = u128::from(value).saturating_mul(u128::from(percentage));
    let rounded = product.saturating_add(99) / 100;
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

fn planned_request_counts(
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
fn planned_sample_unit_lengths(
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

fn ceil_div_u64(numerator: u64, denominator: u64) -> u64 {
    numerator / denominator + u64::from(numerator % denominator != 0)
}

fn evenly_distributed_millisecond_ranges(
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

fn evenly_distributed_unit_ranges(
    total_units: u64,
    sample_lengths: &[u64],
) -> Vec<FrameSampleRange> {
    if sample_lengths.is_empty()
        || total_units == 0
        || sample_lengths.iter().any(|length| *length == 0)
    {
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

/// Returns an app-controlled `fpcalc` sidecar if it is present and has the
/// pinned digest.  It never searches `PATH` and never accepts a user path.
pub fn bundled_fpcalc_path() -> std::result::Result<PathBuf, &'static str> {
    let mut candidates = Vec::new();
    if cfg!(debug_assertions) {
        if let Some(name) = development_sidecar_file_name() {
            candidates.push(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("binaries")
                    .join(name),
            );
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            let sidecar_name = packaged_sidecar_file_name();
            candidates.push(parent.join(sidecar_name));
            candidates.push(parent.join("binaries").join(sidecar_name));
            #[cfg(target_os = "macos")]
            if let Some(contents) = parent.parent() {
                candidates.push(contents.join("Resources").join(sidecar_name));
            }
        }
    }

    for candidate in candidates {
        if validate_pinned_fpcalc(&candidate).is_ok() {
            return Ok(candidate);
        }
    }
    Err("The bundled Chromaprint engine is unavailable for this installation.")
}

/// Availability is intentionally a local fact.  No network request is made.
pub fn local_engine_availability() -> (bool, String) {
    match bundled_fpcalc_path() {
        Ok(_) => (true, CHROMAPRINT_VERSION.into()),
        Err(_) => (false, String::new()),
    }
}

pub fn refresh_local_engine_status(settings: &mut AudioScreeningSettings) {
    let (available, version) = local_engine_availability();
    settings.local_engine_available = available;
    settings.local_engine_version = version;
}

fn development_sidecar_file_name() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Some("fpcalc-x86_64-unknown-linux-gnu");
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Some("fpcalc-aarch64-unknown-linux-gnu");
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Some("fpcalc-x86_64-apple-darwin");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some("fpcalc-aarch64-apple-darwin");
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Some("fpcalc-x86_64-pc-windows-msvc.exe");
    }
    #[allow(unreachable_code)]
    None
}

fn packaged_sidecar_file_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "fpcalc.exe";
    }
    #[cfg(not(target_os = "windows"))]
    {
        "fpcalc"
    }
}

fn expected_fpcalc_sha256() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Some("e7b14fbf9d544f6ba99b7aced3c07786258e09e37cfcb054a41d2a6eeb0887a7");
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Some("9b6fb816312af0b3ca6052a973ba42f61b23e7a919dce4e3ee18e57c34bf3103");
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Some("c1c368de7db49541320624d5f7d4ad827cbbaca96ee104ca6d4c4e0c917c575e");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some("23046544591f275c6da7b0fa57c1290535eb844df271e186e37af1715040921f");
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Some("00dcc56d911f2dea84737aa9dc8e2d118c9eb7a037d815d1ed001d8593e8fbee");
    }
    #[allow(unreachable_code)]
    None
}

fn validate_pinned_fpcalc(path: &Path) -> std::result::Result<(), &'static str> {
    let Some(expected) = expected_fpcalc_sha256() else {
        return Err("No bundled Chromaprint engine is available for this platform.");
    };
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "Bundled Chromaprint engine is missing.")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Bundled Chromaprint engine is not a regular file.");
    }
    let actual = sha256_file(path).map_err(|_| "Bundled Chromaprint engine cannot be verified.")?;
    if actual != expected {
        return Err("Bundled Chromaprint engine verification failed.");
    }
    Ok(())
}

/// Evaluate public ACRCloud configuration without a network operation.
/// `credentials_available` must be derived from the private secret store, not
/// trusted from the public `credentials_configured` display field.
pub fn provider_configuration_status(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
) -> (AudioScreeningProviderStatus, String) {
    if !settings.enabled {
        return (
            AudioScreeningProviderStatus::Disabled,
            "External ACRCloud screening is disabled.".into(),
        );
    }
    if normalize_acrcloud_host(&settings.host).is_err()
        || settings.timeout_seconds == 0
        || settings.timeout_seconds > 120
    {
        return (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "ACRCloud host or timeout is invalid.".into(),
        );
    }
    if !credentials_available {
        return (
            AudioScreeningProviderStatus::NotConfigured,
            "ACRCloud access key and access secret are not configured.".into(),
        );
    }
    (
        AudioScreeningProviderStatus::Ready,
        "ACRCloud is configured for explicitly started audio screening.".into(),
    )
}

pub fn apply_provider_configuration_status(
    settings: &mut AudioScreeningSettings,
    credentials_available: bool,
) {
    let (status, message) = provider_configuration_status(settings, credentials_available);
    settings.credentials_configured = credentials_available;
    settings.status = status;
    settings.status_message = message;
    refresh_local_engine_status(settings);
}

/// Normalizes only a dedicated public ACRCloud project host.  Embedded paths,
/// credentials, ports, IP addresses, redirects, and arbitrary destinations are
/// rejected before an HTTP request can be constructed.
pub fn normalize_acrcloud_host(value: &str) -> std::result::Result<String, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 253 || trimmed.contains(char::is_whitespace) {
        return Err(());
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let parsed = Url::parse(&candidate).map_err(|_| ())?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !(parsed.path().is_empty() || parsed.path() == "/")
    {
        return Err(());
    }
    let host = parsed.host_str().ok_or(())?.to_ascii_lowercase();
    if !host.is_ascii()
        || host == "acrcloud.com"
        || !host.ends_with(".acrcloud.com")
        || host.split('.').any(|part| part.is_empty())
    {
        return Err(());
    }
    Ok(host)
}

fn acrcloud_identify_url(
    settings: &AudioScreeningSettings,
) -> std::result::Result<String, ProviderFailure> {
    let host = normalize_acrcloud_host(&settings.host).map_err(|_| ProviderFailure {
        status: AudioScreeningStatus::ConfigurationInvalid,
        message: "ACRCloud host or timeout is invalid.",
    })?;
    if settings.timeout_seconds == 0 || settings.timeout_seconds > 120 {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud host or timeout is invalid.",
        });
    }
    Ok(format!("https://{host}/v1/identify"))
}

/// Tests only configuration and HTTPS reachability.  It intentionally sends no
/// audio, no multipart data, and no credentials.
pub fn test_acrcloud_provider(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
) -> AudioScreeningProviderTestResult {
    test_acrcloud_provider_with_transport(
        settings,
        credentials_available,
        &UreqAcrCloudHttpTransport,
    )
}

fn test_acrcloud_provider_with_transport(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
    transport: &dyn AcrCloudHttpTransport,
) -> AudioScreeningProviderTestResult {
    let tested_at = Utc::now().to_rfc3339();
    let (configuration_status, configuration_message) =
        provider_configuration_status(settings, credentials_available);
    if configuration_status != AudioScreeningProviderStatus::Ready {
        return AudioScreeningProviderTestResult {
            status: configuration_status,
            message: configuration_message,
            tested_at,
        };
    }
    let request = match acrcloud_identify_url(settings) {
        Ok(url) => AcrCloudRequest {
            url,
            headers: Vec::new(),
            body: Vec::new(),
            timeout_seconds: settings.timeout_seconds,
        },
        Err(failure) => {
            return AudioScreeningProviderTestResult {
                status: provider_status_from_screening(failure.status),
                message: failure.message.into(),
                tested_at,
            };
        }
    };
    match transport.get(request) {
        Ok(response) => {
            let (status, message) = provider_test_status_for_http(response.status);
            AudioScreeningProviderTestResult {
                status,
                message: message.into(),
                tested_at,
            }
        }
        Err(failure) => AudioScreeningProviderTestResult {
            status: provider_status_from_screening(failure.status),
            message: failure.message.into(),
            tested_at,
        },
    }
}

/// A connection test intentionally sends no credentials and uses GET, while
/// the provider's identify endpoint is normally POST.  Therefore 401/403 and
/// 405 still prove the configured HTTPS endpoint is reachable; redirects and
/// other unexpected responses never become a misleading ready state.
fn provider_test_status_for_http(status: u16) -> (AudioScreeningProviderStatus, &'static str) {
    match status {
        200..=299 | 401 | 403 | 405 => (
            AudioScreeningProviderStatus::Ready,
            "ACRCloud host responded. No audio or credentials were sent by this test.",
        ),
        400 | 404 | 300..=399 => (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "The ACRCloud host did not expose the expected identification endpoint.",
        ),
        408 | 425 | 429 | 500..=599 => (
            AudioScreeningProviderStatus::ProviderUnavailable,
            "The ACRCloud host is temporarily unavailable for a connection test.",
        ),
        _ => (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "The ACRCloud host returned an unexpected response to the connection test.",
        ),
    }
}

fn provider_status_from_screening(status: AudioScreeningStatus) -> AudioScreeningProviderStatus {
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
        _ => AudioScreeningProviderStatus::ProviderUnavailable,
    }
}

/// Generates and publishes a local fingerprint record.  Engine failures are a
/// persisted controlled status rather than a fabricated successful result.
pub fn local_fingerprint(
    source_path: &Path,
    track_id: &str,
    evidence: &EvidenceItem,
    track_root: &Path,
    mut progress: impl FnMut(&str, &str),
) -> Result<AudioScreeningLocalRecord> {
    progress("preparing_audio", "Preparing authoritative release audio");
    let mut record = local_record_base(track_id, evidence);
    // `fpcalc` must read a private immutable snapshot, not the live managed
    // file. This closes the check-then-use window where a release file could
    // otherwise change after its SHA-256 binding was verified but before the
    // native decoder opened it.
    let snapshot = match create_verified_source_snapshot(source_path, evidence, track_root) {
        Ok(snapshot) => snapshot,
        Err(()) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message =
                "The authoritative release audio could not be verified for fingerprinting.".into();
            return finish_local_record(track_root, record, None, &mut progress);
        }
    };

    let binary = match bundled_fpcalc_path() {
        Ok(path) => path,
        Err(message) => {
            record.status = AudioScreeningStatus::EngineUnavailable;
            record.message = message.into();
            return finish_local_record(track_root, record, None, &mut progress);
        }
    };
    progress(
        "fingerprinting_audio",
        "Generating local Chromaprint fingerprint",
    );
    match invoke_fpcalc(&binary, &snapshot.path) {
        Ok(output) => {
            record.status = AudioScreeningStatus::FingerprintGenerated;
            record.message = "A local Chromaprint fingerprint was generated from the authoritative release audio.".into();
            record.engine_version = CHROMAPRINT_VERSION.into();
            record.duration_milliseconds = Some(output.duration_milliseconds);
            record.fingerprint = output.fingerprint;
            record.generated_at = Some(Utc::now().to_rfc3339());
            finish_local_record(track_root, record, None, &mut progress)
        }
        Err(failure) => {
            record.status = failure.status();
            record.message = failure.message().into();
            finish_local_record(track_root, record, None, &mut progress)
        }
    }
}

fn finish_local_record(
    track_root: &Path,
    mut record: AudioScreeningLocalRecord,
    external: Option<&AudioScreeningExternalRecord>,
    progress: &mut impl FnMut(&str, &str),
) -> Result<AudioScreeningLocalRecord> {
    progress(
        "fingerprint_complete",
        "Local fingerprint operation completed",
    );
    progress("saving_screening_result", "Saving audio-screening record");
    publish_local_screening_artifacts(track_root, &mut record, external)?;
    progress("complete", "Audio screening completed");
    Ok(record)
}

fn local_record_base(track_id: &str, evidence: &EvidenceItem) -> AudioScreeningLocalRecord {
    AudioScreeningLocalRecord {
        schema_version: 1,
        status: AudioScreeningStatus::NotRun,
        message: "No local Chromaprint fingerprint has been generated yet.".into(),
        engine: CHROMAPRINT_ENGINE.into(),
        engine_version: String::new(),
        fingerprint_algorithm: FINGERPRINT_ALGORITHM.into(),
        track_id: track_id.to_owned(),
        source_evidence_id: evidence.id.clone(),
        source_relative_path: evidence.relative_path.clone(),
        source_sha256: evidence.sha256.clone().unwrap_or_default(),
        source_size_bytes: evidence.size_bytes,
        duration_milliseconds: None,
        fingerprint: String::new(),
        generated_at: None,
        artifact_relative_path: LOCAL_FINGERPRINT_FILE.into(),
        artifact_sha256: String::new(),
    }
}

#[derive(Debug)]
struct FpcalcOutput {
    fingerprint: String,
    duration_milliseconds: u64,
}

#[derive(Debug)]
enum FpcalcFailure {
    EngineUnavailable,
    UnsupportedFormat,
    ProcessingFailed,
}

impl FpcalcFailure {
    fn status(&self) -> AudioScreeningStatus {
        match self {
            Self::EngineUnavailable => AudioScreeningStatus::EngineUnavailable,
            Self::UnsupportedFormat => AudioScreeningStatus::UnsupportedFormat,
            Self::ProcessingFailed => AudioScreeningStatus::ProcessingFailed,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::EngineUnavailable => "The bundled Chromaprint engine is unavailable.",
            Self::UnsupportedFormat => {
                "The authoritative release audio format is not supported by the bundled Chromaprint decoder."
            }
            Self::ProcessingFailed => {
                "Chromaprint could not generate a fingerprint for the authoritative release audio."
            }
        }
    }
}

fn invoke_fpcalc(binary: &Path, source: &Path) -> std::result::Result<FpcalcOutput, FpcalcFailure> {
    let mut child = Command::new(binary)
        .arg("-json")
        .arg("-algorithm")
        .arg(FINGERPRINT_ALGORITHM)
        .arg("-length")
        .arg("0")
        .arg("--")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| FpcalcFailure::EngineUnavailable)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(FpcalcFailure::EngineUnavailable)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(FpcalcFailure::EngineUnavailable)?;
    let stdout_reader = read_bounded(stdout, FPCALC_STDOUT_LIMIT);
    let stderr_reader = read_bounded(stderr, FPCALC_STDERR_LIMIT);
    let status = wait_for_child(&mut child, Duration::from_secs(FPCALC_TIMEOUT_SECONDS))?;
    let stdout = stdout_reader
        .join()
        .ok()
        .and_then(|result| result.ok())
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    let stderr = stderr_reader
        .join()
        .ok()
        .and_then(|result| result.ok())
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    if stdout.exceeded || stderr.exceeded {
        return Err(FpcalcFailure::ProcessingFailed);
    }
    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr.bytes).to_ascii_lowercase();
        return if stderr_text.contains("unsupported")
            || stderr_text.contains("unknown format")
            || stderr_text.contains("invalid data")
        {
            Err(FpcalcFailure::UnsupportedFormat)
        } else {
            Err(FpcalcFailure::ProcessingFailed)
        };
    }
    parse_fpcalc_output(&stdout.bytes)
}

fn parse_fpcalc_output(bytes: &[u8]) -> std::result::Result<FpcalcOutput, FpcalcFailure> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| FpcalcFailure::ProcessingFailed)?;
    let fingerprint = value
        .get("fingerprint")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| value.len() <= FPCALC_STDOUT_LIMIT)
        .ok_or(FpcalcFailure::ProcessingFailed)?
        .to_owned();
    let duration_milliseconds = value
        .get("duration")
        .and_then(Value::as_f64)
        .and_then(seconds_to_milliseconds)
        .filter(|duration| *duration > 0)
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    Ok(FpcalcOutput {
        fingerprint,
        duration_milliseconds,
    })
}

fn seconds_to_milliseconds(seconds: f64) -> Option<u64> {
    if !seconds.is_finite() || seconds < 0.0 || seconds > (u64::MAX as f64 / 1000.0) {
        return None;
    }
    Some((seconds * 1000.0).round() as u64)
}

#[derive(Debug)]
struct BoundedRead {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn read_bounded<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> thread::JoinHandle<io::Result<BoundedRead>> {
    thread::spawn(move || {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 8192];
        let mut exceeded = false;
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let remaining = limit.saturating_sub(retained.len());
            if count > remaining {
                retained.extend_from_slice(&buffer[..remaining]);
                exceeded = true;
            } else {
                retained.extend_from_slice(&buffer[..count]);
            }
        }
        Ok(BoundedRead {
            bytes: retained,
            exceeded,
        })
    })
}

fn wait_for_child(
    child: &mut Child,
    timeout: Duration,
) -> std::result::Result<ExitStatus, FpcalcFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| FpcalcFailure::ProcessingFailed)?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(FpcalcFailure::ProcessingFailed);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Variant for the private persistence layer. Credentials are accepted only
/// transiently and are not copied to records, artifacts, error messages, or
/// returned response data.
pub fn run_external_audio_screening_with_credentials(
    settings: &AudioScreeningSettings,
    credentials: Option<(&str, &str)>,
    source_path: &Path,
    track_id: &str,
    evidence: &EvidenceItem,
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    progress: impl FnMut(&str, &str),
) -> Result<AudioScreeningExternalRecord> {
    run_external_audio_screening_with_transport(
        settings,
        credentials,
        source_path,
        track_id,
        evidence,
        track_root,
        local,
        progress,
        &UreqAcrCloudHttpTransport,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_external_audio_screening_with_transport(
    settings: &AudioScreeningSettings,
    credentials: Option<(&str, &str)>,
    source_path: &Path,
    track_id: &str,
    evidence: &EvidenceItem,
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    mut progress: impl FnMut(&str, &str),
    transport: &dyn AcrCloudHttpTransport,
) -> Result<AudioScreeningExternalRecord> {
    progress(
        "preparing_external_check",
        "Preparing external catalog screening",
    );
    let mut record = external_record_base(track_id, evidence);
    record.screening_mode = AudioScreeningMode::MultiSample;
    record.requested_intensity_percent = settings.intensity_percent;
    record.dynamic_by_track_duration = settings.dynamic_by_track_duration;
    record.reference_duration_seconds = settings.reference_duration_seconds;
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
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
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
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    }
    let Some((access_key, access_secret)) = credentials else {
        // Defensive only: the configuration branch above must have returned.
        record.status = AudioScreeningStatus::SkippedNotConfigured;
        record.message = "ACRCloud access key and access secret are not configured.".into();
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    };
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
            return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
        }
    };
    let parsed_wav = match parse_pcm_wav(&snapshot.path) {
        Ok(parsed) => parsed,
        Err(WavSampleError::UnsupportedFormat) => {
            record.status = AudioScreeningStatus::UnsupportedFormat;
            record.message =
                "External ACRCloud screening currently supports PCM WAV release audio.".into();
            return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
        }
        Err(_) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message = "A bounded WAV sample could not be prepared for ACRCloud.".into();
            return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
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
            return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
        }
    };
    record.target_duration_milliseconds = planned_ranges.target_duration_milliseconds;
    record.planned_request_count = planned_ranges.planned_request_count;
    if planned_ranges.ranges.is_empty() {
        record.status = AudioScreeningStatus::ProcessingFailed;
        record.message =
            "No non-overlapping ACRCloud sample could be planned for this release.".into();
        return finish_external_record(track_root, record, local, Vec::new(), &mut progress);
    }

    let mut used_ranges = Vec::<FrameSampleRange>::new();
    let mut archived_responses = Vec::<PendingProviderResponse>::new();
    for (index, range) in planned_ranges.ranges.iter().copied().enumerate() {
        // This is deliberately immediately before request construction and
        // POST: a future planner change cannot accidentally make the network
        // path upload a duplicate, overlapping, or out-of-track range.
        if !frame_range_is_available(&used_ranges, range, parsed_wav.total_frames) {
            if used_ranges.iter().any(|used| {
                used.start_frame == range.start_frame && used.frame_count == range.frame_count
            }) {
                record.duplicate_sample_count = record.duplicate_sample_count.saturating_add(1);
            } else {
                record.overlapping_sample_count = record.overlapping_sample_count.saturating_add(1);
            }
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message =
                "An overlapping or duplicate ACRCloud sample was rejected before upload.".into();
            break;
        }
        let sample = match extract_pcm_wav_sample_at(
            &snapshot.path,
            &parsed_wav,
            range.start_frame,
            range.frame_count,
        ) {
            Ok(sample) => sample,
            Err(WavSampleError::UnsupportedFormat) => {
                record.status = AudioScreeningStatus::UnsupportedFormat;
                record.message =
                    "External ACRCloud screening currently supports PCM WAV release audio.".into();
                break;
            }
            Err(_) => {
                record.status = AudioScreeningStatus::ProcessingFailed;
                record.message = "A bounded WAV sample could not be prepared for ACRCloud.".into();
                break;
            }
        };
        let end_offset_milliseconds = frames_to_milliseconds(
            range.start_frame.saturating_add(range.frame_count),
            parsed_wav.format.sample_rate,
        );
        let sample_record = AudioScreeningSampleRecord {
            sequence: u32::try_from(index + 1).unwrap_or(MAX_ACRCLOUD_REQUESTS),
            offset_milliseconds: sample.offset_milliseconds,
            end_offset_milliseconds,
            duration_milliseconds: end_offset_milliseconds
                .saturating_sub(sample.offset_milliseconds),
            status: AudioScreeningStatus::ProcessingFailed,
            message: "ACRCloud request was not completed.".into(),
            matches: Vec::new(),
            response_relative_path: None,
            response_sha256: None,
        };
        progress(
            "sending_provider_request",
            "Sending bounded audio sample to ACRCloud",
        );
        let timestamp = Utc::now().timestamp().to_string();
        let (request, signature) = match build_acrcloud_request(
            settings,
            access_key,
            access_secret,
            &timestamp,
            &sample.bytes,
        ) {
            Ok(request) => request,
            Err(failure) => {
                record.status = failure.status;
                record.message = failure.message.into();
                break;
            }
        };
        if record.sample_offset_milliseconds.is_none() {
            // Legacy fields remain a compatibility view of the first submitted
            // range; new consumers must use `samples` for the full run.
            record.sample_offset_milliseconds = Some(sample_record.offset_milliseconds);
            record.sample_duration_milliseconds = Some(sample_record.duration_milliseconds);
        }
        used_ranges.push(range);
        record.samples.push(sample_record);
        record.executed_request_count =
            u32::try_from(record.samples.len()).unwrap_or(MAX_ACRCLOUD_REQUESTS);
        record.request_count = record.executed_request_count;
        record
            .checked_at
            .get_or_insert_with(|| Utc::now().to_rfc3339());
        progress("waiting_provider_response", "Waiting for ACRCloud response");
        let response = match transport.post(request) {
            Ok(response) => response,
            Err(failure) => {
                update_last_sample_failure(&mut record, failure.status, failure.message);
                record.status = failure.status;
                record.message = failure.message.into();
                if should_stop_after_sample(failure.status) {
                    break;
                }
                continue;
            }
        };
        progress(
            "processing_provider_response",
            "Processing ACRCloud response",
        );
        let request_sensitive_values =
            RequestSensitiveValues::new(access_key, access_secret, &signature);
        match parse_acrcloud_response(response.status, &response.body, &request_sensitive_values) {
            Ok(parsed) => {
                let status = parsed.status;
                let message = parsed.message;
                let matches = parsed.matches;
                if let Some(sample_record) = record.samples.last_mut() {
                    sample_record.status = status;
                    sample_record.message = message.into();
                    sample_record.matches = matches;
                }
                if let Some(raw_response) = parsed.raw_response {
                    if let Some(sample_record) = record.samples.last() {
                        archived_responses.push(PendingProviderResponse {
                            sequence: sample_record.sequence,
                            offset_milliseconds: sample_record.offset_milliseconds,
                            end_offset_milliseconds: sample_record.end_offset_milliseconds,
                            duration_milliseconds: sample_record.duration_milliseconds,
                            status,
                            raw_response,
                        });
                    }
                }
                if should_stop_after_sample(status) {
                    break;
                }
            }
            Err(failure) => {
                update_last_sample_failure(&mut record, failure.status, failure.message);
                record.status = failure.status;
                record.message = failure.message.into();
                if should_stop_after_sample(failure.status) {
                    break;
                }
            }
        }
    }
    finalize_external_sample_statistics(&mut record);
    finish_external_record(track_root, record, local, archived_responses, &mut progress)
}

#[derive(Debug)]
struct PcmWavSamplingPlan {
    target_duration_milliseconds: u64,
    planned_request_count: u32,
    ranges: Vec<FrameSampleRange>,
}

fn plan_pcm_wav_sample_ranges(
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

fn frame_range_is_available(
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

fn frame_ranges_overlap_or_duplicate(left: FrameSampleRange, right: FrameSampleRange) -> bool {
    let left_end = left.start_frame.saturating_add(left.frame_count);
    let right_end = right.start_frame.saturating_add(right.frame_count);
    left.start_frame == right.start_frame
        || (left.start_frame < right_end && right.start_frame < left_end)
}

fn frame_ranges_are_non_overlapping(ranges: &[FrameSampleRange], total_frames: u64) -> bool {
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

fn update_last_sample_failure(
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

fn should_stop_after_sample(status: AudioScreeningStatus) -> bool {
    matches!(
        status,
        AudioScreeningStatus::AuthenticationFailed
            | AudioScreeningStatus::ConfigurationInvalid
            | AudioScreeningStatus::ProviderUnavailable
    )
}

fn finalize_external_sample_statistics(record: &mut AudioScreeningExternalRecord) {
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

fn all_samples_completed(record: &AudioScreeningExternalRecord) -> bool {
    record.executed_request_count == record.planned_request_count
        && record.samples.iter().all(|sample| {
            matches!(
                sample.status,
                AudioScreeningStatus::NoMatchDetected | AudioScreeningStatus::MatchDetected
            )
        })
}

fn provider_status_for_result(
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

struct PendingProviderResponse {
    sequence: u32,
    offset_milliseconds: u64,
    end_offset_milliseconds: u64,
    duration_milliseconds: u64,
    status: AudioScreeningStatus,
    raw_response: SanitizedProviderResponse,
}

fn finish_external_record(
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

fn external_record_base(track_id: &str, evidence: &EvidenceItem) -> AudioScreeningExternalRecord {
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
        reference_duration_seconds: 300,
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

fn build_acrcloud_request(
    settings: &AudioScreeningSettings,
    access_key: &str,
    access_secret: &str,
    timestamp: &str,
    sample: &[u8],
) -> std::result::Result<(AcrCloudRequest, String), ProviderFailure> {
    if sample.is_empty() || sample.len() > MAX_SAMPLE_AUDIO_BYTES as usize {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The bounded ACRCloud sample is outside the supported size limit.",
        });
    }
    if access_key.trim().is_empty()
        || access_secret.trim().is_empty()
        || access_key.contains(['\r', '\n'])
        || access_secret.contains(['\r', '\n'])
    {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud credentials are invalid.",
        });
    }
    let url = acrcloud_identify_url(settings)?;
    let signature = acrcloud_signature(access_key, access_secret, timestamp)?;
    let boundary = format!("----SunoDMAcrCloud{}", Uuid::new_v4().simple());
    let body = build_acrcloud_multipart(&boundary, access_key, timestamp, &signature, sample);
    Ok((
        AcrCloudRequest {
            url,
            headers: vec![(
                "Content-Type".into(),
                format!("multipart/form-data; boundary={boundary}"),
            )],
            body,
            timeout_seconds: settings.timeout_seconds,
        },
        signature,
    ))
}

/// ACRCloud Identification API v1 signature:
/// `POST\n/v1/identify\n{access_key}\naudio\n1\n{timestamp}` HMAC-SHA1,
/// encoded with standard Base64.
fn acrcloud_signature(
    access_key: &str,
    access_secret: &str,
    timestamp: &str,
) -> std::result::Result<String, ProviderFailure> {
    let canonical = format!("POST\n/v1/identify\n{access_key}\naudio\n1\n{timestamp}");
    let mut mac =
        HmacSha1::new_from_slice(access_secret.as_bytes()).map_err(|_| ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud credentials are invalid.",
        })?;
    mac.update(canonical.as_bytes());
    Ok(BASE64_STANDARD.encode(mac.finalize().into_bytes()))
}

fn build_acrcloud_multipart(
    boundary: &str,
    access_key: &str,
    timestamp: &str,
    signature: &str,
    sample: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(sample.len().saturating_add(1024));
    multipart_text_part(&mut body, boundary, "access_key", access_key);
    multipart_text_part(&mut body, boundary, "data_type", "audio");
    multipart_text_part(&mut body, boundary, "signature_version", "1");
    multipart_text_part(&mut body, boundary, "signature", signature);
    multipart_text_part(&mut body, boundary, "timestamp", timestamp);
    multipart_text_part(
        &mut body,
        boundary,
        "sample_bytes",
        &sample.len().to_string(),
    );
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"sample\"; filename=\"sunodm-screening.wav\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(sample);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn multipart_text_part(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
}

struct ParsedAcrCloudResponse {
    status: AudioScreeningStatus,
    message: &'static str,
    matches: Vec<AudioScreeningMatch>,
    raw_response: Option<SanitizedProviderResponse>,
}

fn parse_acrcloud_response(
    http_status: u16,
    body: &[u8],
    request_sensitive_values: &RequestSensitiveValues<'_>,
) -> std::result::Result<ParsedAcrCloudResponse, ProviderFailure> {
    if body.len() > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The ACRCloud response exceeds the supported size limit.",
        });
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| ProviderFailure {
        status: status_for_http_error(http_status),
        message: message_for_http_error(http_status),
    })?;
    // A provider response is archival material, so fail closed before even
    // parsing a match: it may not contain a credential-shaped field *or* any
    // exact value sent in this request.  `serde_json` has already decoded
    // normal JSON escapes; the helper additionally walks embedded escaped
    // JSON strings so an echo cannot hide behind a benign key or `\\u` form.
    if response_contains_sensitive_content(&value, request_sensitive_values) {
        return Ok(ParsedAcrCloudResponse {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The provider response contained unsafe credential-like fields and was not documented.",
            matches: Vec::new(),
            raw_response: None,
        });
    }
    let raw_response = SanitizedProviderResponse(body.to_vec());

    if !(200..300).contains(&http_status) {
        return Ok(ParsedAcrCloudResponse {
            status: status_for_http_error(http_status),
            message: message_for_http_error(http_status),
            matches: Vec::new(),
            raw_response: Some(raw_response),
        });
    }
    let status_value = value
        .get("status")
        .and_then(Value::as_object)
        .ok_or(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "ACRCloud returned an unexpected response.",
        })?;
    let provider_code = json_integer(status_value.get("code"));
    let provider_message = status_value
        .get("msg")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if provider_code == Some(0) {
        let matches = match parse_matches(&value) {
            Ok(matches) => matches,
            Err(failure) => {
                return Ok(ParsedAcrCloudResponse {
                    status: failure.status,
                    message: failure.message,
                    matches: Vec::new(),
                    raw_response: None,
                });
            }
        };
        return Ok(ParsedAcrCloudResponse {
            status: if matches.is_empty() {
                AudioScreeningStatus::NoMatchDetected
            } else {
                AudioScreeningStatus::MatchDetected
            },
            message: if matches.is_empty() {
                "ACRCloud returned no catalog match for the submitted audio sample."
            } else {
                "ACRCloud returned one or more catalog matches for the submitted audio sample."
            },
            matches,
            raw_response: Some(raw_response),
        });
    }
    if provider_message.contains("no result") || provider_message.contains("no match") {
        return Ok(ParsedAcrCloudResponse {
            status: AudioScreeningStatus::NoMatchDetected,
            message: "ACRCloud returned no catalog match for the submitted audio sample.",
            matches: Vec::new(),
            raw_response: Some(raw_response),
        });
    }
    let status = if provider_message.contains("access key")
        || provider_message.contains("signature")
        || provider_message.contains("authentication")
        || provider_message.contains("authorization")
    {
        AudioScreeningStatus::AuthenticationFailed
    } else if provider_message.contains("busy")
        || provider_message.contains("unavailable")
        || provider_message.contains("limit")
        || provider_message.contains("timeout")
    {
        AudioScreeningStatus::ProviderUnavailable
    } else {
        AudioScreeningStatus::ProcessingFailed
    };
    Ok(ParsedAcrCloudResponse {
        status,
        message: match status {
            AudioScreeningStatus::AuthenticationFailed => {
                "ACRCloud did not accept the configured credentials."
            }
            AudioScreeningStatus::ProviderUnavailable => {
                "ACRCloud is temporarily unavailable for this screening request."
            }
            _ => "ACRCloud returned an unexpected response.",
        },
        matches: Vec::new(),
        raw_response: Some(raw_response),
    })
}

fn status_for_http_error(status: u16) -> AudioScreeningStatus {
    match status {
        401 | 403 => AudioScreeningStatus::AuthenticationFailed,
        408 | 425 | 429 | 500..=599 => AudioScreeningStatus::ProviderUnavailable,
        _ => AudioScreeningStatus::ProcessingFailed,
    }
}

fn message_for_http_error(status: u16) -> &'static str {
    match status_for_http_error(status) {
        AudioScreeningStatus::AuthenticationFailed => {
            "ACRCloud did not accept the configured credentials."
        }
        AudioScreeningStatus::ProviderUnavailable => {
            "ACRCloud is temporarily unavailable for this screening request."
        }
        _ => "ACRCloud returned an unexpected HTTP response.",
    }
}

fn json_integer(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64).or_else(|| {
        value
            .and_then(Value::as_str)
            .and_then(|value| value.trim().parse::<i64>().ok())
    })
}

fn parse_matches(value: &Value) -> std::result::Result<Vec<AudioScreeningMatch>, ProviderFailure> {
    let Some(music) = value
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("music"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut matches = Vec::new();
    for item in music.iter().take(MAX_PROVIDER_MATCHES) {
        if let Some(parsed) = parse_match(item)? {
            matches.push(parsed);
        }
    }
    Ok(matches)
}

fn parse_match(value: &Value) -> std::result::Result<Option<AudioScreeningMatch>, ProviderFailure> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    // Validate score before deciding whether the object has enough display
    // data to become a match.  A malformed entry without a title must not
    // bypass the non-finite-value barrier and leave raw JSON to be archived.
    let score = object.get("score").and_then(Value::as_f64).or_else(|| {
        object
            .get("score")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<f64>().ok())
    });
    if score.is_some_and(|score| !score.is_finite()) {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "ACRCloud returned a non-finite provider score.",
        });
    }
    let Some(title) = object.get("title").and_then(provider_text) else {
        return Ok(None);
    };
    let artists = object
        .get("artists")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artist| artist.get("name").and_then(provider_text))
        .collect();
    let album = object
        .get("album")
        .and_then(Value::as_object)
        .and_then(|album| album.get("name"))
        .and_then(provider_text);
    let external_ids = object.get("external_ids").and_then(Value::as_object);
    let isrc = external_ids
        .and_then(|identifiers| identifiers.get("isrc"))
        .and_then(provider_text);
    let acrid = object.get("acrid").and_then(provider_text);
    Ok(Some(AudioScreeningMatch {
        title,
        artists,
        album,
        isrc,
        acrid,
        score,
    }))
}

fn provider_text(value: &Value) -> Option<String> {
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    Some(truncate_utf8(text, MAX_PROVIDER_TEXT_BYTES))
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value[..boundary].to_owned()
}

fn response_contains_sensitive_content(
    value: &Value,
    request_sensitive_values: &RequestSensitiveValues<'_>,
) -> bool {
    response_contains_sensitive_content_at_depth(value, request_sensitive_values, 0)
}

fn response_contains_sensitive_content_at_depth(
    value: &Value,
    request_sensitive_values: &RequestSensitiveValues<'_>,
    depth: usize,
) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            credential_like_field_name(key)
                || response_string_contains_sensitive_content(key, request_sensitive_values, depth)
                || response_contains_sensitive_content_at_depth(
                    value,
                    request_sensitive_values,
                    depth,
                )
        }),
        Value::Array(values) => values.iter().any(|value| {
            response_contains_sensitive_content_at_depth(value, request_sensitive_values, depth)
        }),
        Value::String(text) => {
            response_string_contains_sensitive_content(text, request_sensitive_values, depth)
        }
        Value::Number(number) => request_sensitive_values.occurs_in(&number.to_string()),
        Value::Bool(value) => {
            request_sensitive_values.occurs_in(if *value { "true" } else { "false" })
        }
        Value::Null => false,
    }
}

/// Scans normal decoded strings, JSON nested inside a string, and a bounded
/// number of JSON-escape layers.  The outer `serde_json` parse handles the
/// usual `"\\u0061"` case; the extra bounded walk covers a provider that
/// embeds a second JSON document as a string.
fn response_string_contains_sensitive_content(
    text: &str,
    request_sensitive_values: &RequestSensitiveValues<'_>,
    depth: usize,
) -> bool {
    if request_sensitive_values.occurs_in(text) {
        return true;
    }
    if depth >= MAX_PROVIDER_RESPONSE_STRING_DECODE_DEPTH {
        return false;
    }

    if let Ok(nested) = serde_json::from_str::<Value>(text) {
        let differs_from_same_string = !matches!(&nested, Value::String(value) if value == text);
        if differs_from_same_string
            && response_contains_sensitive_content_at_depth(
                &nested,
                request_sensitive_values,
                depth + 1,
            )
        {
            return true;
        }
    }

    // If this is a literal JSON-escape sequence left after one encoded layer
    // (for example `\\u0061ccess-key`), decode it once and scan again.  A raw
    // quote/control character simply makes this speculative decode fail.
    if text.contains('\\') {
        let encoded = format!("\"{text}\"");
        if let Ok(decoded) = serde_json::from_str::<String>(&encoded) {
            if decoded != text
                && response_string_contains_sensitive_content(
                    &decoded,
                    request_sensitive_values,
                    depth + 1,
                )
            {
                return true;
            }
        }
    }
    false
}

fn response_contains_credential_like_field(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            credential_like_field_name(key) || response_contains_credential_like_field(value)
        }),
        Value::Array(values) => values.iter().any(response_contains_credential_like_field),
        _ => false,
    }
}

fn credential_like_field_name(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "accesskey",
        "accesssecret",
        "apikey",
        "privatekey",
        "secretkey",
        "signature",
        "authorization",
        "password",
        "token",
        "secret",
        "clientsecret",
        "credential",
        "bearer",
        "session",
        "cookie",
        "refresh",
        "idtoken",
        "csrf",
        "jwt",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

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
fn publish_external_screening_artifacts(
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

const MAX_ARCHIVED_PROVIDER_RESPONSES_BYTES: usize =
    // Pretty-printing a valid JSON response can add structural whitespace,
    // so reserve headroom above the sum of the independently bounded raw
    // responses. The 25-request cap still keeps this archive finite.
    MAX_PROVIDER_RESPONSE_BYTES * MAX_ACRCLOUD_REQUESTS as usize * 2 + 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchivedAcrCloudResponses<'a> {
    schema_version: u32,
    provider: &'a str,
    source_sha256: &'a str,
    checked_at: Option<&'a str>,
    samples: Vec<ArchivedAcrCloudResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchivedAcrCloudResponse {
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
fn build_provider_response_archive(
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

fn mark_response_archive_unsafe(record: &mut AudioScreeningExternalRecord, sequence: u32) {
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

fn external_matches_have_finite_scores(record: &AudioScreeningExternalRecord) -> bool {
    record
        .matches
        .iter()
        .all(|item| item.score.map_or(true, f64::is_finite))
        && record.samples.iter().all(|sample| {
            sample
                .matches
                .iter()
                .all(|item| item.score.map_or(true, f64::is_finite))
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

fn publish_screening_markdown(
    track_root: &Path,
    local: Option<&AudioScreeningLocalRecord>,
    external: Option<&AudioScreeningExternalRecord>,
) -> Result<()> {
    let mut markdown = String::from("# Pre-Release Audio Screening\n\n");
    markdown.push_str(
        "Technical audio recognition screening only. This record does not determine authorship, copyright ownership, licence validity, non-infringement, melodic originality, or legal safety.\n\n",
    );
    markdown.push_str("## Local audio fingerprint\n\n");
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

    markdown.push_str("\n## External catalog screening\n\n");
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
            markdown.push_str(&format!(
                "- Screening mode: {}\n- Requested intensity: {} %\n- Calculation mode: {}\n- Reference duration: {} seconds\n- Target duration: {} ms\n- Planned requests: {}\n- Executed requests: {}\n- Unique samples: {}\n- Duplicate samples: {}\n- Overlapping samples: {}\n- Unique sampled duration: {} ms\n- Track coverage: {:.2} %\n- Provider status: {:?}\n",
                external.screening_mode.as_str(),
                external.requested_intensity_percent,
                if external.dynamic_by_track_duration { "DYNAMIC_TRACK_DURATION" } else { "FIXED_REFERENCE_DURATION" },
                external.reference_duration_seconds,
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
        if !external.samples.is_empty() {
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
        }
        if !external.matches.is_empty() {
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
    } else {
        markdown.push_str("- Status: NOT RUN\n");
    }
    write_managed(
        track_root,
        AUDIO_SCREENING_MARKDOWN_FILE,
        markdown.as_bytes(),
    )
}

fn markdown_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .collect::<String>()
        .replace('|', "\\|")
        .replace('`', "\\`")
}

fn ensure_screening_directory(track_root: &Path) -> Result<PathBuf> {
    ensure_contained_directory(track_root, Path::new(AUDIO_SCREENING_DIR))
}

fn write_managed(track_root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    validate_relative(Path::new(relative))?;
    let target = contained_path(track_root, Path::new(relative), false)?;
    atomic_write(&target, bytes)
}

fn archive_managed_artifacts(track_root: &Path, relatives: &[&str]) -> Result<()> {
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
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(AppError::Symlink(destination.display().to_string()));
        }
        Ok(_) => return Err(AppError::Collision(destination.display().to_string())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(AppError::io(&destination, error)),
    }

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

fn ensure_regular_screening_tree(directory: &Path) -> Result<()> {
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

fn read_existing_external_record(
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
    // Multi-sample records may contain successful requests with no safe raw
    // response (for example a transport failure). Every retained response
    // must nevertheless point to the same verified aggregate archive.
    for sample in &record.samples {
        match (
            sample.response_relative_path.as_deref(),
            sample.response_sha256.as_deref(),
        ) {
            (None, None) => {}
            (Some(relative), Some(expected))
                if record_archive.is_some_and(|(record_relative, record_expected)| {
                    relative == record_relative && expected == record_expected
                }) => {}
            _ => return Ok(false),
        }
    }
    if !record.samples.is_empty() && record_archive.is_some() {
        let Some(bytes) = archive_bytes else {
            return Ok(false);
        };
        let archive: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => return Ok(false),
        };
        let Some(entries) = archive.get("samples").and_then(Value::as_array) else {
            return Ok(false);
        };
        for sample in record.samples.iter().filter(|sample| {
            sample.response_relative_path.is_some() || sample.response_sha256.is_some()
        }) {
            let matches_coordinates = entries.iter().any(|entry| {
                entry.get("sequence").and_then(Value::as_u64) == Some(u64::from(sample.sequence))
                    && entry.get("offsetMilliseconds").and_then(Value::as_u64)
                        == Some(sample.offset_milliseconds)
                    && entry.get("endOffsetMilliseconds").and_then(Value::as_u64)
                        == Some(sample.end_offset_milliseconds)
                    && entry.get("durationMilliseconds").and_then(Value::as_u64)
                        == Some(sample.duration_milliseconds)
            });
            if !matches_coordinates {
                return Ok(false);
            }
        }
    }
    Ok(true)
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

fn source_binding_matches(
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

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// A process-private source copy whose bytes were streamed and hashed before
/// any decoder or network adapter can consume them. `TempDir` removes the
/// working audio automatically and has owner-only permissions on supported
/// platforms; the temporary copy is never placed in the track tree.
struct VerifiedSourceSnapshot {
    _directory: TempDir,
    path: PathBuf,
}

fn create_verified_source_snapshot(
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

fn validate_source_binding(
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

#[derive(Debug, Clone, Copy)]
struct PcmFormat {
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bit_depth: u16,
}

#[derive(Debug, Clone, Copy)]
struct DataSegment {
    offset: u64,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct ParsedPcmWav {
    format: PcmFormat,
    data_segments: Vec<DataSegment>,
    total_frames: u64,
}

/// Supports ordinary RIFF PCM WAV only.  Other accepted release formats still
/// get local Chromaprint analysis through bundled `fpcalc`; external sampling
/// refuses them explicitly instead of silently converting or uploading them.
pub fn extract_bounded_pcm_wav_sample(
    source: &Path,
) -> std::result::Result<ExtractedWavSample, WavSampleError> {
    let parsed = parse_pcm_wav(source)?;
    let sample_frames = parsed.total_frames.min(max_pcm_sample_frames(&parsed)?);
    if sample_frames == 0 {
        return Err(WavSampleError::InvalidAudio);
    }
    let start_frame = (parsed.total_frames - sample_frames) / 2;
    extract_pcm_wav_sample_at(source, &parsed, start_frame, sample_frames)
}

fn parse_pcm_wav(source: &Path) -> std::result::Result<ParsedPcmWav, WavSampleError> {
    let metadata = fs::symlink_metadata(source).map_err(|_| WavSampleError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WavSampleError::Io);
    }
    let mut file = File::open(source).map_err(|_| WavSampleError::Io)?;
    let file_length = metadata.len();
    if file_length < 12 {
        return Err(WavSampleError::UnsupportedFormat);
    }
    let mut header = [0_u8; 12];
    file.read_exact(&mut header)
        .map_err(|_| WavSampleError::InvalidAudio)?;
    if header[0..4] != *b"RIFF" || header[8..12] != *b"WAVE" {
        return Err(WavSampleError::UnsupportedFormat);
    }
    let declared_size = u32::from_le_bytes(header[4..8].try_into().expect("four bytes"));
    if declared_size < 4 {
        return Err(WavSampleError::InvalidAudio);
    }
    let riff_end = 8_u64
        .checked_add(u64::from(declared_size))
        .ok_or(WavSampleError::InvalidAudio)?;
    if riff_end > file_length {
        return Err(WavSampleError::InvalidAudio);
    }

    let mut position = 12_u64;
    let mut chunk_count = 0_usize;
    let mut format = None;
    let mut data_segments = Vec::new();
    while position < riff_end {
        chunk_count += 1;
        if chunk_count > MAX_RIFF_CHUNKS || riff_end - position < 8 {
            return Err(WavSampleError::InvalidAudio);
        }
        file.seek(SeekFrom::Start(position))
            .map_err(|_| WavSampleError::Io)?;
        let mut chunk_header = [0_u8; 8];
        file.read_exact(&mut chunk_header)
            .map_err(|_| WavSampleError::InvalidAudio)?;
        let chunk_size = u64::from(u32::from_le_bytes(
            chunk_header[4..8].try_into().expect("four bytes"),
        ));
        let data_start = position
            .checked_add(8)
            .ok_or(WavSampleError::InvalidAudio)?;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or(WavSampleError::InvalidAudio)?;
        let padded_end = data_end
            .checked_add(chunk_size & 1)
            .ok_or(WavSampleError::InvalidAudio)?;
        if padded_end > riff_end {
            return Err(WavSampleError::InvalidAudio);
        }
        match &chunk_header[0..4] {
            b"fmt " if format.is_none() => {
                format = Some(read_pcm_format(&mut file, data_start, chunk_size)?);
            }
            b"data" => data_segments.push(DataSegment {
                offset: data_start,
                bytes: chunk_size,
            }),
            _ => {}
        }
        position = padded_end;
    }
    let format = format.ok_or(WavSampleError::UnsupportedFormat)?;
    if data_segments.is_empty() {
        return Err(WavSampleError::InvalidAudio);
    }
    let total_data_bytes = data_segments.iter().try_fold(0_u64, |total, segment| {
        total
            .checked_add(segment.bytes)
            .ok_or(WavSampleError::InvalidAudio)
    })?;
    if total_data_bytes == 0 || total_data_bytes % u64::from(format.block_align) != 0 {
        return Err(WavSampleError::InvalidAudio);
    }
    let total_frames = total_data_bytes / u64::from(format.block_align);
    Ok(ParsedPcmWav {
        format,
        data_segments,
        total_frames,
    })
}

fn max_pcm_sample_frames(parsed: &ParsedPcmWav) -> std::result::Result<u64, WavSampleError> {
    let duration_limit_frames = u64::from(parsed.format.sample_rate)
        .checked_mul(MAX_SAMPLE_SECONDS)
        .ok_or(WavSampleError::InvalidAudio)?;
    // The provider's upload cap applies to the complete RIFF document, not
    // only its PCM data chunk. Reserve the fixed header and a possible pad
    // byte before rounding down to whole PCM frames.
    let max_pcm_payload_bytes = MAX_SAMPLE_AUDIO_BYTES
        .checked_sub(PCM_WAV_HEADER_BYTES + PCM_WAV_MAX_PADDING_BYTES)
        .ok_or(WavSampleError::InvalidAudio)?;
    let upload_limit_frames = max_pcm_payload_bytes / u64::from(parsed.format.block_align);
    Ok(duration_limit_frames.min(upload_limit_frames))
}

fn extract_pcm_wav_sample_at(
    source: &Path,
    parsed: &ParsedPcmWav,
    start_frame: u64,
    sample_frames: u64,
) -> std::result::Result<ExtractedWavSample, WavSampleError> {
    if sample_frames == 0
        || sample_frames > max_pcm_sample_frames(parsed)?
        || start_frame
            .checked_add(sample_frames)
            .map_or(true, |end| end > parsed.total_frames)
    {
        return Err(WavSampleError::InvalidAudio);
    }
    let sample_bytes_len = sample_frames
        .checked_mul(u64::from(parsed.format.block_align))
        .ok_or(WavSampleError::InvalidAudio)?;
    let sample_bytes_len =
        usize::try_from(sample_bytes_len).map_err(|_| WavSampleError::InvalidAudio)?;
    let mut file = File::open(source).map_err(|_| WavSampleError::Io)?;
    let samples = copy_pcm_range(
        &mut file,
        &parsed.data_segments,
        start_frame * u64::from(parsed.format.block_align),
        sample_bytes_len,
    )?;
    let bytes = build_pcm_wav(parsed.format, &samples)?;
    if bytes.len() > MAX_SAMPLE_AUDIO_BYTES as usize {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(ExtractedWavSample {
        bytes,
        offset_milliseconds: frames_to_milliseconds(start_frame, parsed.format.sample_rate),
        duration_milliseconds: frames_to_milliseconds(sample_frames, parsed.format.sample_rate),
        source_duration_milliseconds: frames_to_milliseconds(
            parsed.total_frames,
            parsed.format.sample_rate,
        ),
    })
}

fn read_pcm_format(
    file: &mut File,
    data_start: u64,
    chunk_size: u64,
) -> std::result::Result<PcmFormat, WavSampleError> {
    if chunk_size < 16 {
        return Err(WavSampleError::InvalidAudio);
    }
    file.seek(SeekFrom::Start(data_start))
        .map_err(|_| WavSampleError::Io)?;
    let mut bytes = [0_u8; 16];
    file.read_exact(&mut bytes)
        .map_err(|_| WavSampleError::InvalidAudio)?;
    let format_tag = u16::from_le_bytes(bytes[0..2].try_into().expect("two bytes"));
    let channels = u16::from_le_bytes(bytes[2..4].try_into().expect("two bytes"));
    let sample_rate = u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes"));
    let byte_rate = u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes"));
    let block_align = u16::from_le_bytes(bytes[12..14].try_into().expect("two bytes"));
    let bit_depth = u16::from_le_bytes(bytes[14..16].try_into().expect("two bytes"));
    if format_tag != 1 {
        return Err(WavSampleError::UnsupportedFormat);
    }
    if channels == 0 || channels > 8 || sample_rate == 0 || !matches!(bit_depth, 8 | 16 | 24 | 32) {
        return Err(WavSampleError::InvalidAudio);
    }
    let bytes_per_sample = u32::from(bit_depth) / 8;
    let expected_align = u32::from(channels)
        .checked_mul(bytes_per_sample)
        .ok_or(WavSampleError::InvalidAudio)?;
    let expected_rate = sample_rate
        .checked_mul(expected_align)
        .ok_or(WavSampleError::InvalidAudio)?;
    if u32::from(block_align) != expected_align || byte_rate != expected_rate {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(PcmFormat {
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bit_depth,
    })
}

fn copy_pcm_range(
    file: &mut File,
    segments: &[DataSegment],
    mut global_offset: u64,
    requested_bytes: usize,
) -> std::result::Result<Vec<u8>, WavSampleError> {
    let mut output = Vec::with_capacity(requested_bytes);
    let mut remaining = requested_bytes as u64;
    for segment in segments {
        if global_offset >= segment.bytes {
            global_offset -= segment.bytes;
            continue;
        }
        let available = segment.bytes - global_offset;
        let to_copy = available.min(remaining);
        file.seek(SeekFrom::Start(
            segment
                .offset
                .checked_add(global_offset)
                .ok_or(WavSampleError::InvalidAudio)?,
        ))
        .map_err(|_| WavSampleError::Io)?;
        let mut left = to_copy;
        let mut buffer = [0_u8; 64 * 1024];
        while left > 0 {
            let count = usize::try_from(left.min(buffer.len() as u64))
                .map_err(|_| WavSampleError::InvalidAudio)?;
            file.read_exact(&mut buffer[..count])
                .map_err(|_| WavSampleError::InvalidAudio)?;
            output.extend_from_slice(&buffer[..count]);
            left -= count as u64;
        }
        remaining -= to_copy;
        if remaining == 0 {
            break;
        }
        global_offset = 0;
    }
    if remaining != 0 || output.len() != requested_bytes {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(output)
}

fn build_pcm_wav(
    format: PcmFormat,
    samples: &[u8],
) -> std::result::Result<Vec<u8>, WavSampleError> {
    let sample_length = u32::try_from(samples.len()).map_err(|_| WavSampleError::InvalidAudio)?;
    let data_padding = sample_length & 1;
    let riff_size = 4_u32
        .checked_add(8 + 16)
        .and_then(|size| size.checked_add(8))
        .and_then(|size| size.checked_add(sample_length))
        .and_then(|size| size.checked_add(data_padding))
        .ok_or(WavSampleError::InvalidAudio)?;
    let mut bytes = Vec::with_capacity(riff_size as usize + 8);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&format.channels.to_le_bytes());
    bytes.extend_from_slice(&format.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&format.byte_rate.to_le_bytes());
    bytes.extend_from_slice(&format.block_align.to_le_bytes());
    bytes.extend_from_slice(&format.bit_depth.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&sample_length.to_le_bytes());
    bytes.extend_from_slice(samples);
    if data_padding != 0 {
        bytes.push(0);
    }
    Ok(bytes)
}

fn frames_to_milliseconds(frames: u64, sample_rate: u32) -> u64 {
    let milliseconds = (u128::from(frames) * 1000) / u128::from(sample_rate);
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EvidenceMetadata, EvidenceRole};
    use std::cell::RefCell;
    use std::fs;
    use tempfile::tempdir;

    struct FixedGetTransport {
        status: u16,
    }

    impl AcrCloudHttpTransport for FixedGetTransport {
        fn post(
            &self,
            _request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            panic!("connection test must not send a POST request")
        }

        fn get(
            &self,
            _request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            Ok(AcrCloudResponse {
                status: self.status,
                body: Vec::new(),
            })
        }
    }

    struct NoUploadTransport;

    impl AcrCloudHttpTransport for NoUploadTransport {
        fn post(
            &self,
            _request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            panic!("external request must not be sent without a current local fingerprint")
        }

        fn get(
            &self,
            _request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            panic!("external screening must not run a connection test")
        }
    }

    struct RecordingTransport {
        posts: RefCell<Vec<AcrCloudRequest>>,
        responses: RefCell<Vec<AcrCloudResponse>>,
    }

    impl RecordingTransport {
        fn no_match(count: usize) -> Self {
            Self {
                posts: RefCell::new(Vec::new()),
                responses: RefCell::new(
                    (0..count)
                        .map(|_| AcrCloudResponse {
                            status: 200,
                            body: br#"{"status":{"code":1001,"msg":"No result"}}"#.to_vec(),
                        })
                        .collect(),
                ),
            }
        }
    }

    impl AcrCloudHttpTransport for RecordingTransport {
        fn post(
            &self,
            request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            self.posts.borrow_mut().push(request);
            self.responses
                .borrow_mut()
                .drain(..1)
                .next()
                .ok_or(ProviderFailure {
                    status: AudioScreeningStatus::ProviderUnavailable,
                    message: "ACRCloud could not be reached.",
                })
        }

        fn get(
            &self,
            _request: AcrCloudRequest,
        ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
            panic!("screening run must not issue a GET request")
        }
    }

    #[test]
    fn acrcloud_signature_uses_the_official_canonical_string() {
        let signature =
            acrcloud_signature("test_key", "test_secret", "1234567890").expect("signature");
        assert_eq!(signature, "ECjW5VypntRDhMHRNLGVSuzqpCg=");
    }

    #[test]
    fn multipart_contains_required_fields_but_never_the_access_secret() {
        let signature = acrcloud_signature("key", "secret-value", "1").expect("signature");
        let body = build_acrcloud_multipart("boundary", "key", "1", &signature, b"wav");
        let text = String::from_utf8_lossy(&body);
        for field in [
            "access_key",
            "data_type",
            "signature_version",
            "signature",
            "timestamp",
            "sample_bytes",
            "name=\"sample\"",
        ] {
            assert!(text.contains(field), "missing {field}");
        }
        assert!(!text.contains("secret-value"));
        assert!(text.contains("\r\n\r\nwav\r\n--boundary--"));
    }

    #[test]
    fn provider_response_maps_no_match_and_match_without_legal_statuses() {
        let request_sensitive_values =
            RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
        let no_match = br#"{"status":{"code":1001,"msg":"No result"}}"#;
        let parsed = parse_acrcloud_response(200, no_match, &request_sensitive_values)
            .expect("no match parse");
        assert_eq!(parsed.status, AudioScreeningStatus::NoMatchDetected);
        assert!(parsed.matches.is_empty());

        let matched = br#"{
          "status":{"code":0,"msg":"Success"},
          "metadata":{"music":[{
             "title":"Track", "artists":[{"name":"Artist"}],
             "album":{"name":"Album"}, "external_ids":{"isrc":"ISRC"},
             "acrid":"acrid-1", "score":88
          }]}
        }"#;
        let parsed =
            parse_acrcloud_response(200, matched, &request_sensitive_values).expect("match parse");
        assert_eq!(parsed.status, AudioScreeningStatus::MatchDetected);
        assert_eq!(parsed.matches.len(), 1);
        assert_eq!(parsed.matches[0].title, "Track");
        assert_eq!(parsed.matches[0].score, Some(88.0));
    }

    #[test]
    fn provider_response_echoing_request_sensitive_values_is_not_retained() {
        let access_key = ["test", "access", "key"].join("-");
        let access_secret = ["test", "access", "secret"].join("-");
        let signature =
            acrcloud_signature(&access_key, &access_secret, "123").expect("test signature");
        let request_sensitive_values =
            RequestSensitiveValues::new(&access_key, &access_secret, &signature);
        let escaped_secret = access_secret
            .bytes()
            .map(|byte| format!("\\u{byte:04x}"))
            .collect::<String>();
        let responses = [
            serde_json::json!({
                "status": {"code": 0},
                "metadata": {"music": [{"title": format!("echo: {access_key}")}]}
            })
            .to_string(),
            format!(
                r#"{{"status":{{"code":0}},"metadata":{{"music":[{{"title":"{escaped_secret}"}}]}}}}"#
            ),
            format!(
                r#"{{"{signature}":"ordinary value","status":{{"code":1001,"msg":"No result"}}}}"#
            ),
            r#"{"secret":"unrelated value","status":{"code":1001,"msg":"No result"}}"#.into(),
        ];
        for response in responses {
            let parsed =
                parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
                    .expect("controlled response result");
            assert_eq!(parsed.status, AudioScreeningStatus::ProcessingFailed);
            assert!(parsed.matches.is_empty());
            assert!(parsed.raw_response.is_none());
        }

        // The controlled result is what the application persists.  It has no
        // provider response or parsed match text to forward to Track JSON,
        // the WebView, Markdown, a certificate, or a manifest.
        let response = serde_json::json!({
            "status": {"code": 0},
            "metadata": {"music": [{"title": format!("echo: {access_secret}")}]}
        })
        .to_string();
        let parsed = parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
            .expect("controlled response result");
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        let evidence = evidence_item();
        let mut record = external_record_base("track-1", &evidence);
        record.status = parsed.status;
        record.message = parsed.message.into();
        record.matches = parsed.matches;
        let persisted = finish_external_record(&root, record, None, Vec::new(), &mut |_, _| {})
            .expect("persist controlled response");
        assert_eq!(persisted.status, AudioScreeningStatus::ProcessingFailed);
        assert!(persisted.matches.is_empty());
        assert!(persisted.response_relative_path.is_none());
        assert!(!root.join(ACRCLOUD_RESPONSE_FILE).exists());
        let stored = fs::read_to_string(root.join(EXTERNAL_SCREENING_FILE))
            .expect("stored controlled result");
        let markdown = fs::read_to_string(root.join(AUDIO_SCREENING_MARKDOWN_FILE))
            .expect("controlled screening summary");
        for sensitive in [&access_key, &access_secret, &signature] {
            assert!(!stored.contains(sensitive));
            assert!(!markdown.contains(sensitive));
        }
    }

    #[test]
    fn provider_response_recursively_rejects_common_credential_field_names() {
        let request_sensitive_values =
            RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
        for field in [
            "secret",
            "session_id",
            "set_cookie",
            "refresh_token",
            "id_token",
            "csrf_token",
            "jwt_claim",
        ] {
            let response = format!(
                r#"{{"status":{{"code":1001,"msg":"No result"}},"nested":{{"{field}":"unrelated value"}}}}"#
            );
            let parsed =
                parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
                    .expect("controlled credential-like response");
            assert_eq!(
                parsed.status,
                AudioScreeningStatus::ProcessingFailed,
                "{field}"
            );
            assert!(parsed.matches.is_empty(), "{field}");
            assert!(parsed.raw_response.is_none(), "{field}");
        }
    }

    #[test]
    fn non_finite_provider_score_becomes_controlled_failure_before_publication() {
        let request_sensitive_values =
            RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
        for response in [
            br#"{
              "status":{"code":0},
              "metadata":{"music":[{"title":"Track","score":"NaN"}]}
            }"# as &[u8],
            br#"{
              "status":{"code":0},
              "metadata":{"music":[{"score":"NaN"}]}
            }"#,
        ] {
            let parsed = parse_acrcloud_response(200, response, &request_sensitive_values)
                .expect("controlled score result");
            assert_eq!(parsed.status, AudioScreeningStatus::ProcessingFailed);
            assert!(parsed.matches.is_empty());
            assert!(parsed.raw_response.is_none());
        }

        // A defensive direct-publication check happens before archival, so a
        // malformed in-memory record cannot partially replace current output.
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        ensure_screening_directory(&root).expect("screening directory");
        fs::write(root.join(EXTERNAL_SCREENING_FILE), b"previous result")
            .expect("previous external result");
        let evidence = evidence_item();
        let mut record = external_record_base("track-1", &evidence);
        record.matches.push(AudioScreeningMatch {
            title: "Track".into(),
            artists: Vec::new(),
            album: None,
            isrc: None,
            acrid: None,
            score: Some(f64::NAN),
        });
        assert!(
            publish_external_screening_artifacts(&root, None, &mut record, Vec::new()).is_err()
        );
        assert_eq!(
            fs::read(root.join(EXTERNAL_SCREENING_FILE)).expect("unchanged previous result"),
            b"previous result"
        );
        assert!(!root.join(".archive/audio-screening").exists());
    }

    #[test]
    fn configuration_status_distinguishes_disabled_not_configured_and_ready() {
        let mut settings = AudioScreeningSettings::default();
        assert_eq!(
            provider_configuration_status(&settings, false).0,
            AudioScreeningProviderStatus::Disabled
        );
        settings.enabled = true;
        settings.host = "identify-eu-west-1.acrcloud.com".into();
        assert_eq!(
            provider_configuration_status(&settings, false).0,
            AudioScreeningProviderStatus::NotConfigured
        );
        assert_eq!(
            provider_configuration_status(&settings, true).0,
            AudioScreeningProviderStatus::Ready
        );
        settings.host = "http://127.0.0.1:8080".into();
        assert_eq!(
            provider_configuration_status(&settings, true).0,
            AudioScreeningProviderStatus::ConfigurationInvalid
        );
    }

    #[test]
    fn coverage_settings_reject_out_of_range_values() {
        let mut settings = AudioScreeningSettings::default();
        settings.intensity_percent = 0;
        assert!(validate_acrcloud_sampling_settings(&settings).is_err());
        settings.intensity_percent = 101;
        assert!(validate_acrcloud_sampling_settings(&settings).is_err());
        settings.intensity_percent = 25;
        settings.reference_duration_seconds = 0;
        assert!(validate_acrcloud_sampling_settings(&settings).is_err());
        settings.reference_duration_seconds = 86_401;
        assert!(validate_acrcloud_sampling_settings(&settings).is_err());
        settings.reference_duration_seconds = 300;
        assert!(validate_acrcloud_sampling_settings(&settings).is_ok());
    }

    #[test]
    fn connection_test_only_marks_expected_http_statuses_as_reachable() {
        let settings = configured_provider_settings();
        for status in [200, 204, 401, 403, 405] {
            let result = test_acrcloud_provider_with_transport(
                &settings,
                true,
                &FixedGetTransport { status },
            );
            assert_eq!(
                result.status,
                AudioScreeningProviderStatus::Ready,
                "{status}"
            );
        }
        for status in [400, 404, 301, 302, 418] {
            let result = test_acrcloud_provider_with_transport(
                &settings,
                true,
                &FixedGetTransport { status },
            );
            assert_eq!(
                result.status,
                AudioScreeningProviderStatus::ConfigurationInvalid,
                "{status}"
            );
        }
        for status in [429, 500, 503] {
            let result = test_acrcloud_provider_with_transport(
                &settings,
                true,
                &FixedGetTransport { status },
            );
            assert_eq!(
                result.status,
                AudioScreeningProviderStatus::ProviderUnavailable,
                "{status}"
            );
        }
    }

    #[test]
    fn adapter_does_not_upload_without_a_current_local_fingerprint() {
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        let evidence = evidence_item();
        let result = run_external_audio_screening_with_transport(
            &configured_provider_settings(),
            Some(("access-key", "access-secret")),
            &root.join("01_RELEASE/release.wav"),
            "track-1",
            &evidence,
            &root,
            None,
            no_progress,
            &NoUploadTransport,
        )
        .expect("controlled no-upload result");
        assert_eq!(result.status, AudioScreeningStatus::ProcessingFailed);
        assert_eq!(result.request_count, 0);
        assert!(result.matches.is_empty());
        assert!(result.response_relative_path.is_none());
        assert!(!root.join(ACRCLOUD_RESPONSE_FILE).exists());
    }

    #[test]
    fn wav_extraction_is_bounded_deterministic_and_does_not_change_source() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("release.wav");
        let original = pcm_wave(48_000, 2, 16, 30);
        fs::write(&source, &original).expect("write WAV");
        let extracted = extract_bounded_pcm_wav_sample(&source).expect("extract sample");
        assert_eq!(fs::read(&source).expect("read original"), original);
        assert_eq!(extracted.duration_milliseconds, 12_000);
        assert_eq!(extracted.offset_milliseconds, 9_000);
        assert_eq!(extracted.source_duration_milliseconds, 30_000);
        assert!(extracted.bytes.starts_with(b"RIFF"));
        assert!(extracted.bytes.len() < original.len());
    }

    #[test]
    fn wav_extraction_reserves_riff_overhead_inside_the_provider_cap() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("release.wav");
        // 8-bit mono permits an odd data payload, exercising the extra RIFF
        // pad byte as well as the fixed header reservation.
        fs::write(&source, pcm_wave(1_000_000, 1, 8, 5)).expect("write large WAV");
        let extracted = extract_bounded_pcm_wav_sample(&source).expect("extract bounded sample");
        assert!(extracted.bytes.len() <= MAX_SAMPLE_AUDIO_BYTES as usize);
        assert_eq!(extracted.bytes.len(), MAX_SAMPLE_AUDIO_BYTES as usize);
    }

    #[test]
    fn sampling_plan_is_deterministic_evenly_distributed_and_hard_capped() {
        let settings = AudioScreeningSettings {
            intensity_percent: 25,
            dynamic_by_track_duration: true,
            ..AudioScreeningSettings::default()
        };
        let first = plan_acrcloud_sample_ranges(600_000, &settings);
        let second = plan_acrcloud_sample_ranges(600_000, &settings);
        assert_eq!(first, second);
        assert_eq!(first.target_duration_milliseconds, 150_000);
        assert_eq!(first.requested_request_count, 13);
        assert_eq!(first.planned_request_count, 13);
        assert_eq!(first.maximum_unique_duration_milliseconds, 150_000);
        assert_eq!(
            first
                .samples
                .first()
                .map(|sample| sample.offset_milliseconds),
            Some(0)
        );
        assert_eq!(
            first
                .samples
                .last()
                .map(|sample| sample.end_offset_milliseconds),
            Some(600_000)
        );
        assert_eq!(
            first
                .samples
                .last()
                .map(|sample| sample.duration_milliseconds),
            Some(6_000)
        );
        assert_non_overlapping_millisecond_ranges(&first.samples, 600_000);

        let maximum = plan_acrcloud_sample_ranges(
            7_200_000,
            &AudioScreeningSettings {
                intensity_percent: 100,
                ..AudioScreeningSettings::default()
            },
        );
        assert_eq!(maximum.planned_request_count, MAX_ACRCLOUD_REQUESTS);
        assert_eq!(
            maximum.maximum_unique_duration_milliseconds,
            MAX_ACRCLOUD_UNIQUE_SAMPLE_SECONDS * 1_000
        );
        assert_non_overlapping_millisecond_ranges(&maximum.samples, 7_200_000);
    }

    #[test]
    fn sampling_plan_caps_fixed_reference_at_track_and_handles_short_tracks() {
        let fixed = AudioScreeningSettings {
            intensity_percent: 25,
            dynamic_by_track_duration: false,
            reference_duration_seconds: 300,
            ..AudioScreeningSettings::default()
        };
        let capped = plan_acrcloud_sample_ranges(60_000, &fixed);
        assert_eq!(capped.target_duration_milliseconds, 60_000);
        assert_eq!(capped.planned_request_count, 5);
        assert_eq!(capped.maximum_unique_duration_milliseconds, 60_000);
        assert_non_overlapping_millisecond_ranges(&capped.samples, 60_000);

        let short = plan_acrcloud_sample_ranges(9_000, &AudioScreeningSettings::default());
        assert_eq!(short.planned_request_count, 1);
        assert_eq!(short.samples[0].offset_milliseconds, 0);
        assert_eq!(short.samples[0].duration_milliseconds, 9_000);
        assert_eq!(short.samples[0].end_offset_milliseconds, 9_000);
    }

    #[test]
    fn external_run_archives_every_non_overlapping_multi_sample_response() {
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        let release_directory = root.join("01_RELEASE");
        fs::create_dir_all(&release_directory).expect("release directory");
        let source = release_directory.join("release.wav");
        fs::write(&source, pcm_wave(1_000, 1, 8, 600)).expect("release WAV");
        let mut evidence = evidence_item();
        evidence.sha256 = Some(sha256_file(&source).expect("release hash"));
        evidence.size_bytes = fs::metadata(&source).expect("release metadata").len();
        let mut local = local_record_base("track-1", &evidence);
        local.status = AudioScreeningStatus::FingerprintGenerated;
        local.fingerprint = "1,2,3".into();
        local.duration_milliseconds = Some(600_000);
        publish_local_screening_artifacts(&root, &mut local, None).expect("local record");
        let settings = AudioScreeningSettings {
            enabled: true,
            host: "identify-eu-west-1.acrcloud.com".into(),
            intensity_percent: 25,
            dynamic_by_track_duration: true,
            ..AudioScreeningSettings::default()
        };
        let transport = RecordingTransport::no_match(13);
        let record = run_external_audio_screening_with_transport(
            &settings,
            Some(("access-key", "access-secret")),
            &source,
            "track-1",
            &evidence,
            &root,
            Some(&local),
            no_progress,
            &transport,
        )
        .expect("multi-sample external screening");

        assert_eq!(record.screening_mode, AudioScreeningMode::MultiSample);
        assert_eq!(record.status, AudioScreeningStatus::NoMatchDetected);
        assert_eq!(record.planned_request_count, 13);
        assert_eq!(record.executed_request_count, 13);
        assert_eq!(record.request_count, 13);
        assert_eq!(record.unique_sample_count, 13);
        assert_eq!(record.duplicate_sample_count, 0);
        assert_eq!(record.overlapping_sample_count, 0);
        assert_eq!(record.unique_sample_duration_milliseconds, 150_000);
        assert!((record.track_coverage_percent - 25.0).abs() < f64::EPSILON);
        assert_eq!(record.samples.len(), 13);
        assert_eq!(transport.posts.borrow().len(), 13);
        assert_non_overlapping_sample_records(&record.samples, 600_000);
        assert!(record
            .samples
            .iter()
            .all(|sample| sample.status == AudioScreeningStatus::NoMatchDetected));
        assert!(record
            .samples
            .iter()
            .all(|sample| sample.duration_milliseconds <= 12_000));
        assert!(external_response_artifact_is_current(&root, &record).expect("response hash"));
        let archive: Value = serde_json::from_slice(
            &fs::read(root.join(ACRCLOUD_RESPONSE_FILE)).expect("response archive"),
        )
        .expect("structured response archive");
        let archived_samples = archive
            .get("samples")
            .and_then(Value::as_array)
            .expect("archived samples");
        assert_eq!(archived_samples.len(), 13);
        for (sample, archived) in record.samples.iter().zip(archived_samples) {
            assert_eq!(
                archived.get("offsetMilliseconds").and_then(Value::as_u64),
                Some(sample.offset_milliseconds)
            );
            assert_eq!(
                archived
                    .get("endOffsetMilliseconds")
                    .and_then(Value::as_u64),
                Some(sample.end_offset_milliseconds)
            );
        }
        let mut mismatched_record = record.clone();
        mismatched_record.samples[0].offset_milliseconds += 1;
        assert!(
            !external_response_artifact_is_current(&root, &mismatched_record)
                .expect("mismatched archive is rejected")
        );
    }

    #[test]
    fn stale_helpers_never_leave_a_previous_result_current() {
        let mut state = AudioScreeningState::default();
        state.local.status = AudioScreeningStatus::FingerprintGenerated;
        state.local.fingerprint = "1,2,3".into();
        state.external.status = AudioScreeningStatus::NoMatchDetected;
        state.external.response_relative_path = Some(ACRCLOUD_RESPONSE_FILE.into());
        state.external.response_sha256 = Some("b".repeat(64));
        mark_screening_stale(&mut state);
        assert_eq!(state.local.status, AudioScreeningStatus::Stale);
        assert_eq!(state.external.status, AudioScreeningStatus::Stale);
        assert!(state.external.response_relative_path.is_none());
        assert!(state.external.response_sha256.is_none());
    }

    #[test]
    fn positive_local_record_requires_duration_and_expected_track_binding() {
        let evidence = evidence_item();
        let mut record = local_record_base("track-a", &evidence);
        record.status = AudioScreeningStatus::FingerprintGenerated;
        record.fingerprint = "1,2,3".into();
        record.artifact_sha256 = "c".repeat(64);
        assert!(!local_record_matches_source(&record, "track-a", &evidence));
        record.duration_milliseconds = Some(1);
        assert!(local_record_matches_source(&record, "track-a", &evidence));
        assert!(!local_record_matches_source(&record, "track-b", &evidence));

        let mut external = external_record_base("track-a", &evidence);
        external.status = AudioScreeningStatus::NoMatchDetected;
        assert!(external_record_matches_source(
            &external, "track-a", &evidence
        ));
        assert!(!external_record_matches_source(
            &external, "track-b", &evidence
        ));
    }

    #[test]
    fn fpcalc_output_requires_a_positive_finite_duration() {
        for output in [
            &br#"{"fingerprint":"1,2,3"}"#[..],
            &br#"{"fingerprint":"1,2,3","duration":0}"#[..],
            &br#"{"fingerprint":"1,2,3","duration":-1}"#[..],
            &br#"{"fingerprint":"1,2,3","duration":"NaN"}"#[..],
        ] {
            assert!(matches!(
                parse_fpcalc_output(output),
                Err(FpcalcFailure::ProcessingFailed)
            ));
        }
        let parsed = parse_fpcalc_output(br#"{"fingerprint":"1,2,3","duration":1.25}"#)
            .expect("valid fpcalc output");
        assert_eq!(parsed.duration_milliseconds, 1_250);
    }

    #[test]
    fn archive_current_screening_artifacts_moves_the_complete_directory() {
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        let current = root.join(AUDIO_SCREENING_DIR);
        fs::create_dir_all(current.join("nested")).expect("current screening directory");
        fs::write(current.join("LOCAL_FINGERPRINT.json"), b"local").expect("local artifact");
        fs::write(current.join("nested/extra.txt"), b"nested").expect("nested artifact");

        assert!(archive_current_screening_artifacts(&root).expect("archive current directory"));
        assert!(!current.exists());
        let archive_root = root.join(".archive/audio-screening");
        let archive = fs::read_dir(&archive_root)
            .expect("archive entries")
            .next()
            .expect("archive entry")
            .expect("archive path")
            .path()
            .join("AUDIO_SCREENING");
        assert_eq!(
            fs::read(archive.join("LOCAL_FINGERPRINT.json")).expect("local"),
            b"local"
        );
        assert_eq!(
            fs::read(archive.join("nested/extra.txt")).expect("nested"),
            b"nested"
        );
        assert!(!archive_current_screening_artifacts(&root).expect("no current directory"));
    }

    #[cfg(unix)]
    #[test]
    fn archive_current_screening_artifacts_rejects_a_symlinked_directory() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("temporary directory");
        let target = tempdir().expect("symlink target");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        fs::create_dir_all(root.join("03_DOCUMENTATION")).expect("documentation parent");
        symlink(target.path(), root.join(AUDIO_SCREENING_DIR)).expect("screening symlink");
        assert!(matches!(
            archive_current_screening_artifacts(&root),
            Err(AppError::Symlink(_))
        ));
    }

    #[test]
    fn local_record_artifact_uses_detached_self_hash() {
        let directory = tempdir().expect("temporary directory");
        let root = fs::canonicalize(directory.path()).expect("canonical root");
        let evidence = evidence_item();
        let mut record = local_record_base("track-1", &evidence);
        record.status = AudioScreeningStatus::FingerprintGenerated;
        record.fingerprint = "1,2,3".into();
        publish_local_screening_artifacts(&root, &mut record, None).expect("publish");
        assert!(local_artifact_is_current(&root, &record).expect("validate artifact"));
        let hash_file = root.join(LOCAL_FINGERPRINT_HASH_FILE);
        assert!(fs::read_to_string(hash_file)
            .expect("hash file")
            .contains(&record.artifact_sha256));
    }

    fn assert_non_overlapping_millisecond_ranges(
        ranges: &[AcrCloudSampleRange],
        track_duration_milliseconds: u64,
    ) {
        let mut previous_end = 0_u64;
        for range in ranges {
            assert!(range.duration_milliseconds > 0);
            assert!(range.duration_milliseconds <= 12_000);
            assert!(range.offset_milliseconds >= previous_end);
            assert_eq!(
                range.end_offset_milliseconds,
                range.offset_milliseconds + range.duration_milliseconds
            );
            assert!(range.end_offset_milliseconds <= track_duration_milliseconds);
            previous_end = range.end_offset_milliseconds;
        }
    }

    fn assert_non_overlapping_sample_records(
        samples: &[AudioScreeningSampleRecord],
        track_duration_milliseconds: u64,
    ) {
        let mut previous_end = 0_u64;
        for sample in samples {
            assert!(sample.duration_milliseconds > 0);
            assert!(sample.duration_milliseconds <= 12_000);
            assert!(sample.offset_milliseconds >= previous_end);
            assert_eq!(
                sample.end_offset_milliseconds,
                sample.offset_milliseconds + sample.duration_milliseconds
            );
            assert!(sample.end_offset_milliseconds <= track_duration_milliseconds);
            previous_end = sample.end_offset_milliseconds;
        }
    }

    fn evidence_item() -> EvidenceItem {
        EvidenceItem {
            id: "release-1".into(),
            role: EvidenceRole::ReleaseWav,
            file_name: "release.wav".into(),
            relative_path: "01_RELEASE/release.wav".into(),
            sha256: Some("a".repeat(64)),
            size_bytes: 123,
            imported_at: "2026-01-01T00:00:00Z".into(),
            verified: true,
            verification_error: None,
            source_global_evidence_id: None,
            coverage_start: None,
            coverage_end: None,
            provenance: Default::default(),
            derived_from_evidence_id: None,
            generator_version: None,
            generated_disclosure_text: None,
            metadata: EvidenceMetadata::default(),
        }
    }

    fn configured_provider_settings() -> AudioScreeningSettings {
        AudioScreeningSettings {
            enabled: true,
            host: "identify-eu-west-1.acrcloud.com".into(),
            ..AudioScreeningSettings::default()
        }
    }

    fn no_progress(_: &str, _: &str) {}

    fn pcm_wave(sample_rate: u32, channels: u16, bit_depth: u16, seconds: u32) -> Vec<u8> {
        let block_align = channels * (bit_depth / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let samples = vec![0x7f_u8; (byte_rate * seconds) as usize];
        build_pcm_wav(
            PcmFormat {
                channels,
                sample_rate,
                byte_rate,
                block_align,
                bit_depth,
            },
            &samples,
        )
        .expect("PCM WAV")
    }
}
