use super::{
    AudioScreeningState, AudioScreeningSummary, BlockingDeviation, CertificateState, DocumentState,
    EvidenceItem, ExternalTimestampRecord, ExternalTimestampSummary, FinalizationAnchor,
    IntegrityState, Profile, StepState, TrackAutomation, TrackFieldOrigins, TrackFields,
    TrackLibraryPlacement, TrackStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRecord {
    pub id: String,
    pub relative_path: String,
    pub status: TrackStatus,
    pub workflow_id: String,
    pub workflow_version: String,
    #[serde(default)]
    pub profile_snapshot: Profile,
    #[serde(default)]
    pub library: TrackLibraryPlacement,
    #[serde(default)]
    pub field_origins: TrackFieldOrigins,
    pub fields: TrackFields,
    /// Pre-release audio screening is mutable only while a track is editable.
    /// Older stored tracks deserialize to the neutral default without a
    /// backfill, preserving finalized snapshots byte-for-byte.
    #[serde(default)]
    pub audio_screening: AudioScreeningState,
    pub documents: DocumentState,
    pub integrity: IntegrityState,
    pub certificate: CertificateState,
    pub created_at: String,
    pub updated_at: String,
    pub legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub id: String,
    pub title: String,
    pub relative_path: String,
    pub status: TrackStatus,
    pub updated_at: String,
    pub progress: u8,
    pub missing_count: u32,
    pub certificate_valid: Option<bool>,
    pub legacy: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_evidence_id: Option<String>,
    #[serde(default)]
    pub library: TrackLibraryPlacement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDetail {
    pub id: String,
    pub title: String,
    pub relative_path: String,
    pub status: TrackStatus,
    pub updated_at: String,
    pub progress: u8,
    pub missing_count: u32,
    pub certificate_valid: Option<bool>,
    pub legacy: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_evidence_id: Option<String>,
    #[serde(default)]
    pub library: TrackLibraryPlacement,
    pub workflow_id: String,
    pub workflow_version: String,
    pub profile_snapshot: Profile,
    pub automation: TrackAutomation,
    pub fields: TrackFields,
    pub steps: Vec<StepState>,
    pub evidence: Vec<EvidenceItem>,
    /// Public summary intentionally includes neither the full Chromaprint
    /// fingerprint nor raw provider response bytes or credentials.
    #[serde(default)]
    pub audio_screening: AudioScreeningSummary,
    #[serde(default)]
    pub external_timestamps: Vec<ExternalTimestampRecord>,
    #[serde(default)]
    pub external_timestamp_summary: ExternalTimestampSummary,
    #[serde(default)]
    pub finalization_anchors: Vec<FinalizationAnchor>,
    pub documents: DocumentState,
    pub integrity: IntegrityState,
    pub certificate: CertificateState,
    pub blocking_deviations: Vec<BlockingDeviation>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub valid: bool,
    pub missing_items: Vec<String>,
    pub blocking_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<TrackDetail>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgress {
    pub stage: String,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub processed_files: u32,
    pub total_files: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCandidate {
    pub name: String,
    pub relative_path: String,
    pub status: String,
    pub missing_items: Vec<String>,
    pub has_managed_document_collision: bool,
    pub recognized_folders: Vec<String>,
    pub documents: Vec<String>,
    pub evidence_files: Vec<String>,
    pub hash_manifest_present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScan {
    pub discovered: u32,
    pub indexed: u32,
    pub unchanged: u32,
    pub warnings: Vec<String>,
    pub candidates: Vec<LegacyCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPreview {
    pub files: Vec<String>,
    pub collisions: Vec<String>,
    pub adoption_required: bool,
}
