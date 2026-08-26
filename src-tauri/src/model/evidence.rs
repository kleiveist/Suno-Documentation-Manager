use super::{CertificateLanguage, EvidenceRole, StepStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceItem {
    pub id: String,
    pub role: EvidenceRole,
    pub file_name: String,
    pub relative_path: String,
    pub sha256: Option<String>,
    pub size_bytes: u64,
    pub imported_at: String,
    pub verified: bool,
    pub verification_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_global_evidence_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_end: Option<String>,
    #[serde(default)]
    pub provenance: EvidenceProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from_evidence_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_disclosure_text: Option<String>,
    #[serde(default)]
    pub metadata: EvidenceMetadata,
}

/// Role-specific evidence metadata and compatibility fields. The terms-PDF
/// importer combines user-entered descriptive context with system-derived file
/// properties. Empty values mean "not recorded".
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct EvidenceMetadata {
    /// Original local source name captured during import (system-derived metadata).
    pub original_file_name: String,
    pub document_title: String,
    pub provider: String,
    pub source_url: String,
    pub retrieval_date: String,
    pub effective_date: String,
    pub applicable_production_period: String,
    pub factual_note: String,
    pub timestamp_type: String,
    pub external_timestamp: String,
    pub referenced_hash: String,
    pub referenced_artifact: String,
    pub external_reference_id: String,
    pub provider_verification_url: String,
    /// File properties captured by the native importer. These values are
    /// evidence-derived and never requested from the user.
    pub file_extension: String,
    pub mime_type: String,
    pub audio_format: String,
    pub audio_channels: Option<u16>,
    pub audio_sample_rate_hz: Option<u32>,
    pub audio_duration_milliseconds: Option<u64>,
    pub audio_bit_depth: Option<u16>,
    pub embedded_metadata: Vec<EmbeddedMetadata>,
    /// Structured values extracted from WAV evidence. Only the currently
    /// registered Suno final export may feed track-level facts;
    /// `suno_raw_metadata` preserves the embedded source value.
    pub suno_studio_detected: bool,
    pub suno_created_timestamp: String,
    pub suno_created_date: String,
    pub suno_id: String,
    pub suno_raw_metadata: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct EmbeddedMetadata {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct EvidenceDerivedField {
    pub value: String,
    pub original_value: String,
    pub evidence_id: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackFieldOrigins {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suno_final_generation_id: Option<EvidenceDerivedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suno_final_generation_date: Option<EvidenceDerivedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_end_date: Option<EvidenceDerivedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suno_download_export_date: Option<EvidenceDerivedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_export_date: Option<EvidenceDerivedField>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactOrigin {
    UserConfirmedFact,
    EvidenceDerivedMetadata,
    #[default]
    NotDocumented,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ByteIdenticalPair {
    pub left_evidence_id: String,
    pub left_role: EvidenceRole,
    pub right_evidence_id: String,
    pub right_role: EvidenceRole,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyIssue {
    pub code: String,
    pub message: String,
    pub step_id: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrackAutomation {
    #[serde(default)]
    pub final_generation_id_origin: FactOrigin,
    pub final_generation_origin: FactOrigin,
    pub production_end_origin: FactOrigin,
    pub download_export_origin: FactOrigin,
    pub final_export_origin: FactOrigin,
    pub suno_metadata_detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suno_created_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suno_id: Option<String>,
    pub release_identical_to_suno_export: bool,
    pub byte_identical_pairs: Vec<ByteIdenticalPair>,
    pub consistency_issues: Vec<ConsistencyIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidencePreview {
    pub evidence_id: String,
    pub role: EvidenceRole,
    pub file_name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub data_url: Option<String>,
    pub text_content: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackCoverPreview {
    pub evidence_id: String,
    pub data_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProvenance {
    #[default]
    ManagedCopy,
    IndexedLegacy,
    GeneratedDisclosure,
    GlobalCopy,
}

impl EvidenceProvenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManagedCopy => "managed_copy",
            Self::IndexedLegacy => "indexed_legacy",
            Self::GeneratedDisclosure => "generated_disclosure",
            Self::GlobalCopy => "global_copy",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalEvidenceItem {
    #[serde(flatten)]
    pub evidence: EvidenceItem,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepState {
    pub id: String,
    pub status: StepStatus,
    pub na_reason: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityState {
    pub generated: bool,
    pub verified: bool,
    pub file_count: u32,
    pub verified_count: u32,
    pub generated_at: Option<String>,
    pub verified_at: Option<String>,
    pub mismatch_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentState {
    pub generated: bool,
    pub current: bool,
    pub generated_at: Option<String>,
    pub template_version: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub input_fingerprint: String,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            generated: false,
            current: false,
            generated_at: None,
            template_version: crate::documents::TEMPLATE_VERSION.into(),
            files: Vec::new(),
            input_fingerprint: String::new(),
        }
    }
}

/// Ephemeral choices supplied when the finalization transaction starts.
///
/// The primary language intentionally lives in the workspace profile. The
/// former per-action bilingual flag remains on the wire for compatibility
/// with older clients but is ignored by finalization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FinalizeOptions {
    /// Deprecated compatibility input. New finalizations always create both
    /// language PDFs regardless of this value.
    pub bilingual: bool,
}

/// Resolved presentation choices for one immutable certificate set.
///
/// The application combines the current workspace setting with the fixed
/// dual-language PDF policy before certificate generation, so this value is
/// also ready to be recorded in the manifest.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CertificateRenderOptions {
    pub language: CertificateLanguage,
    pub bilingual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertificateState {
    pub valid: bool,
    pub certificate_id: Option<String>,
    /// Stable identity of this phase-one finalization snapshot. This differs
    /// from the human-facing certificate ID so a later timestamp addendum can
    /// bind to the original snapshot even after it is archived as a revision.
    #[serde(default)]
    pub finalization_snapshot_id: Option<String>,
    pub finalized_at: Option<String>,
    pub workflow_version: Option<String>,
    /// The actual language used to render this immutable certificate set.
    #[serde(default)]
    pub certificate_language: CertificateLanguage,
    /// Whether the immutable certificate set contains both supported PDF
    /// languages. New finalizations always persist `true`; `false` is kept
    /// readable for older single-language snapshots.
    #[serde(default)]
    pub bilingual: bool,
    pub invalidated_at: Option<String>,
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockingDeviation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub blocking: bool,
    pub resolved: bool,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviationInput {
    pub description: String,
    pub blocking: bool,
}
