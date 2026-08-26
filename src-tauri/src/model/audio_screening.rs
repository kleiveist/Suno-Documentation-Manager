use serde::{Deserialize, Serialize};

/// Technical outcome of a pre-release audio screening operation. These states
/// intentionally describe only fingerprinting or a provider response; they do
/// not make a copyright, licence, originality, or legal-safety determination.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioScreeningStatus {
    #[default]
    NotRun,
    FingerprintGenerated,
    NoMatchDetected,
    MatchDetected,
    SkippedNotConfigured,
    ProviderUnavailable,
    AuthenticationFailed,
    ConfigurationInvalid,
    EngineUnavailable,
    UnsupportedFormat,
    ProcessingFailed,
    Stale,
}

/// Describes how the external catalog screening selected audio from a release.
/// `SingleSample` remains the deserialization default for records written by
/// versions before configurable coverage was introduced.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioScreeningMode {
    #[default]
    SingleSample,
    MultiSample,
}

impl AudioScreeningMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleSample => "SINGLE-SAMPLE",
            Self::MultiSample => "MULTI-SAMPLE",
        }
    }
}

/// Configuration health for the optional ACRCloud provider. It is deliberately
/// separate from `AudioScreeningStatus`: a provider configuration is not a
/// per-track screening result.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioScreeningProviderStatus {
    #[default]
    Disabled,
    NotConfigured,
    Ready,
    AuthenticationFailed,
    ProviderUnavailable,
    ConfigurationInvalid,
}

/// Non-secret global ACRCloud configuration. Credentials are write-only and
/// stored outside SQLite so they cannot be copied to profiles, tracks,
/// manifests, certificates, revisions, or public DTOs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningSettings {
    pub enabled: bool,
    pub host: String,
    pub timeout_seconds: u32,
    /// Percentage of the configured duration basis requested for external
    /// screening. The execution planner applies its independent 25-request
    /// and 12-second-per-request safety bounds.
    pub intensity_percent: u8,
    /// When false, `reference_duration_seconds` is used as the duration basis
    /// and the resulting target is still capped at the actual track duration.
    pub dynamic_by_track_duration: bool,
    pub reference_duration_seconds: u64,
    pub status: AudioScreeningProviderStatus,
    pub status_message: String,
    pub credentials_configured: bool,
    pub local_engine_available: bool,
    pub local_engine_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tested_at: Option<String>,
}

impl Default for AudioScreeningSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            timeout_seconds: 30,
            intensity_percent: 5,
            dynamic_by_track_duration: true,
            reference_duration_seconds: 300,
            status: AudioScreeningProviderStatus::Disabled,
            status_message: "External ACRCloud screening is disabled.".into(),
            credentials_configured: false,
            local_engine_available: false,
            local_engine_version: String::new(),
            last_tested_at: None,
        }
    }
}

/// Write-only input for ACRCloud credentials. This deliberately does not
/// implement `Serialize`, preventing accidental return or normal persistence.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningSecretInput {
    pub access_key: Option<String>,
    pub access_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioScreeningProviderTestResult {
    pub status: AudioScreeningProviderStatus,
    pub message: String,
    pub tested_at: String,
}

/// A factual match summary copied from an ACRCloud response. Every optional
/// field remains absent when the provider did not supply it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningMatch {
    pub title: String,
    pub artists: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acrid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// Technical result of one non-overlapping ACRCloud sample. Raw provider
/// payloads remain only in the portable response archive named by the optional
/// path and digest; they are deliberately not returned through normal track
/// summaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningSampleRecord {
    pub sequence: u32,
    pub offset_milliseconds: u64,
    pub end_offset_milliseconds: u64,
    pub duration_milliseconds: u64,
    pub status: AudioScreeningStatus,
    pub message: String,
    /// Original provider status fields copied from the safe ACRCloud response
    /// archive. They are optional so older screening records remain readable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_status_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_status_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_api_version: Option<String>,
    pub matches: Vec<AudioScreeningMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_relative_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_sha256: Option<String>,
}

impl AudioScreeningSampleRecord {
    /// Formats only provider status metadata copied from the response. This is
    /// deliberately separate from `status`/`message`, which are SunoDM's
    /// internal technical interpretation of the sample.
    pub fn provider_status_details(&self) -> Option<String> {
        let mut details = Vec::new();
        if let Some(code) = self.provider_status_code {
            details.push(format!("Provider Code: {code}"));
        }
        if let Some(message) = self
            .provider_status_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details.push(format!("Provider Message: {message}"));
        }
        if let Some(version) = self
            .provider_api_version
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details.push(format!("Provider Version: {version}"));
        }
        (!details.is_empty()).then(|| details.join(" · "))
    }

    /// Compact provider presentation used by the PDF sample rows.
    pub fn provider_status_compact(&self) -> Option<String> {
        let mut details = Vec::new();
        if let Some(code) = self.provider_status_code {
            details.push(code.to_string());
        }
        if let Some(message) = self
            .provider_status_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details.push(message.to_owned());
        }
        if let Some(version) = self
            .provider_api_version
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details.push(format!("API {version}"));
        }
        (!details.is_empty()).then(|| format!("ACRCloud: {}", details.join(" · ")))
    }
}

/// Durable state for the local Chromaprint run. The fingerprint itself is
/// retained only in the portable JSON artifact and persisted track state; it
/// is deliberately omitted from certificates and public track summaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningLocalRecord {
    pub schema_version: u32,
    pub status: AudioScreeningStatus,
    pub message: String,
    pub engine: String,
    pub engine_version: String,
    pub fingerprint_algorithm: String,
    pub track_id: String,
    pub source_evidence_id: String,
    pub source_relative_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_milliseconds: Option<u64>,
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    pub artifact_relative_path: String,
    pub artifact_sha256: String,
}

impl Default for AudioScreeningLocalRecord {
    fn default() -> Self {
        Self {
            schema_version: 1,
            status: AudioScreeningStatus::NotRun,
            message: "No local Chromaprint fingerprint has been generated yet.".into(),
            engine: "chromaprint".into(),
            engine_version: String::new(),
            fingerprint_algorithm: "2".into(),
            track_id: String::new(),
            source_evidence_id: String::new(),
            source_relative_path: String::new(),
            source_sha256: String::new(),
            source_size_bytes: 0,
            duration_milliseconds: None,
            fingerprint: String::new(),
            generated_at: None,
            artifact_relative_path: String::new(),
            artifact_sha256: String::new(),
        }
    }
}

/// Durable state for an explicitly user-triggered ACRCloud request. No
/// credential, request signature, or request header is ever recorded here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningExternalRecord {
    pub schema_version: u32,
    pub provider: String,
    pub status: AudioScreeningStatus,
    pub message: String,
    pub track_id: String,
    pub source_evidence_id: String,
    pub source_relative_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<String>,
    /// New records use `MULTI-SAMPLE`; the default keeps historical records
    /// readable without a migration.
    pub screening_mode: AudioScreeningMode,
    pub requested_intensity_percent: u8,
    pub dynamic_by_track_duration: bool,
    /// Fixed-duration calculation basis captured when the screening was run.
    /// Historical records that predate this field remain distinguishable from
    /// records that explicitly captured a configured value.
    #[serde(default)]
    pub reference_duration_seconds: Option<u64>,
    pub target_duration_milliseconds: u64,
    pub planned_request_count: u32,
    pub executed_request_count: u32,
    pub unique_sample_count: u32,
    pub overlapping_sample_count: u32,
    pub duplicate_sample_count: u32,
    pub unique_sample_duration_milliseconds: u64,
    pub track_coverage_percent: f64,
    pub provider_status: AudioScreeningProviderStatus,
    pub samples: Vec<AudioScreeningSampleRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_offset_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_duration_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_duration_milliseconds: Option<u64>,
    pub request_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_relative_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_sha256: Option<String>,
    pub matches: Vec<AudioScreeningMatch>,
    /// Frozen only at finalization, so historical certificates can truthfully
    /// explain why the optional provider was not used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_at_snapshot: Option<bool>,
}

impl Default for AudioScreeningExternalRecord {
    fn default() -> Self {
        Self {
            schema_version: 1,
            provider: "ACRCloud".into(),
            status: AudioScreeningStatus::NotRun,
            message: "No external catalog screening has been run.".into(),
            track_id: String::new(),
            source_evidence_id: String::new(),
            source_relative_path: String::new(),
            source_sha256: String::new(),
            source_size_bytes: 0,
            checked_at: None,
            screening_mode: AudioScreeningMode::SingleSample,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioScreeningState {
    pub local: AudioScreeningLocalRecord,
    pub external: AudioScreeningExternalRecord,
}

/// Browser-safe view of a local record. The full acoustic fingerprint remains
/// only in the internal track state and `LOCAL_FINGERPRINT.json`; it is not
/// needed to render workflow status and must not cross the Tauri IPC boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioScreeningLocalSummary {
    pub status: AudioScreeningStatus,
    pub message: String,
    pub engine: String,
    pub engine_version: String,
    pub fingerprint_algorithm: String,
    pub source_evidence_id: String,
    pub source_relative_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    pub artifact_relative_path: String,
    pub artifact_sha256: String,
}

impl From<&AudioScreeningLocalRecord> for AudioScreeningLocalSummary {
    fn from(record: &AudioScreeningLocalRecord) -> Self {
        Self {
            status: record.status,
            message: record.message.clone(),
            engine: record.engine.clone(),
            engine_version: record.engine_version.clone(),
            fingerprint_algorithm: record.fingerprint_algorithm.clone(),
            source_evidence_id: record.source_evidence_id.clone(),
            source_relative_path: record.source_relative_path.clone(),
            source_sha256: record.source_sha256.clone(),
            source_size_bytes: record.source_size_bytes,
            duration_milliseconds: record.duration_milliseconds,
            generated_at: record.generated_at.clone(),
            artifact_relative_path: record.artifact_relative_path.clone(),
            artifact_sha256: record.artifact_sha256.clone(),
        }
    }
}

/// Browser-safe view of the factual external provider result. The raw response
/// stays in the portable artifact; request credentials and signatures are not
/// represented by either record type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioScreeningExternalSummary {
    pub provider: String,
    pub status: AudioScreeningStatus,
    pub message: String,
    pub source_evidence_id: String,
    pub source_relative_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<String>,
    pub screening_mode: AudioScreeningMode,
    pub requested_intensity_percent: u8,
    pub dynamic_by_track_duration: bool,
    #[serde(default)]
    pub reference_duration_seconds: Option<u64>,
    pub target_duration_milliseconds: u64,
    pub planned_request_count: u32,
    pub executed_request_count: u32,
    pub unique_sample_count: u32,
    pub overlapping_sample_count: u32,
    pub duplicate_sample_count: u32,
    pub unique_sample_duration_milliseconds: u64,
    pub track_coverage_percent: f64,
    pub provider_status: AudioScreeningProviderStatus,
    pub samples: Vec<AudioScreeningSampleRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_offset_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_duration_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_duration_milliseconds: Option<u64>,
    pub request_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_relative_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_sha256: Option<String>,
    pub matches: Vec<AudioScreeningMatch>,
}

impl From<&AudioScreeningExternalRecord> for AudioScreeningExternalSummary {
    fn from(record: &AudioScreeningExternalRecord) -> Self {
        Self {
            provider: record.provider.clone(),
            status: record.status,
            message: record.message.clone(),
            source_evidence_id: record.source_evidence_id.clone(),
            source_relative_path: record.source_relative_path.clone(),
            source_sha256: record.source_sha256.clone(),
            source_size_bytes: record.source_size_bytes,
            checked_at: record.checked_at.clone(),
            screening_mode: record.screening_mode,
            requested_intensity_percent: record.requested_intensity_percent,
            dynamic_by_track_duration: record.dynamic_by_track_duration,
            reference_duration_seconds: record.reference_duration_seconds,
            target_duration_milliseconds: record.target_duration_milliseconds,
            planned_request_count: record.planned_request_count,
            executed_request_count: record.executed_request_count,
            unique_sample_count: record.unique_sample_count,
            overlapping_sample_count: record.overlapping_sample_count,
            duplicate_sample_count: record.duplicate_sample_count,
            unique_sample_duration_milliseconds: record.unique_sample_duration_milliseconds,
            track_coverage_percent: record.track_coverage_percent,
            provider_status: record.provider_status,
            samples: record.samples.clone(),
            sample_offset_milliseconds: record.sample_offset_milliseconds,
            sample_duration_milliseconds: record.sample_duration_milliseconds,
            source_duration_milliseconds: record.source_duration_milliseconds,
            request_count: record.request_count,
            response_relative_path: record.response_relative_path.clone(),
            response_sha256: record.response_sha256.clone(),
            matches: record.matches.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioScreeningSummary {
    pub local: AudioScreeningLocalSummary,
    pub external: AudioScreeningExternalSummary,
}

impl From<&AudioScreeningState> for AudioScreeningSummary {
    fn from(state: &AudioScreeningState) -> Self {
        Self {
            local: AudioScreeningLocalSummary::from(&state.local),
            external: AudioScreeningExternalSummary::from(&state.external),
        }
    }
}

impl Default for AudioScreeningSummary {
    fn default() -> Self {
        Self::from(&AudioScreeningState::default())
    }
}
