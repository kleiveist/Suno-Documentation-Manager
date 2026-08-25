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
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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

mod artifacts;
mod configuration;
mod engine;
mod external;
mod protocol;
mod sampling;
mod transport;
mod wav;

pub use self::artifacts::{
    archive_current_screening_artifacts, external_record_matches_source,
    external_response_artifact_is_current, local_artifact_is_current, local_record_matches_source,
    mark_screening_stale, publish_local_screening_artifacts, refresh_screening_markdown,
};
pub use self::configuration::{
    apply_provider_configuration_status, normalize_acrcloud_host, provider_configuration_status,
    test_acrcloud_provider,
};
pub use self::engine::{local_fingerprint, refresh_local_engine_status};
pub use self::external::{
    run_external_audio_screening_with_credentials, ExternalAudioScreeningRequest,
};
#[cfg(test)]
pub use self::sampling::plan_acrcloud_sample_ranges;
pub use self::sampling::validate_acrcloud_sampling_settings;
#[cfg(test)]
pub use self::wav::extract_bounded_pcm_wav_sample;

pub fn bundled_fpcalc_path() -> std::result::Result<PathBuf, &'static str> {
    engine::bundled_fpcalc_path()
}

pub fn local_engine_availability() -> (bool, String) {
    engine::local_engine_availability()
}

use self::artifacts::*;
use self::configuration::*;
#[cfg(test)]
use self::engine::{local_record_base, parse_fpcalc_output, FpcalcFailure};
use self::external::*;
use self::protocol::*;
use self::sampling::*;
use self::transport::*;
use self::wav::*;

#[cfg(test)]
mod tests;
