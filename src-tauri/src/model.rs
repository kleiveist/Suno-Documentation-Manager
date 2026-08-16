use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackStatus {
    Draft,
    Active,
    Ready,
    Finalized,
    Superseded,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackLibrarySection {
    #[default]
    Single,
    Album,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackLibraryPlacement {
    pub section: TrackLibrarySection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_title: Option<String>,
}

impl TrackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Active => "ACTIVE",
            Self::Ready => "READY",
            Self::Finalized => "FINALIZED",
            Self::Superseded => "SUPERSEDED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StepStatus {
    NotRun,
    Pass,
    Fail,
    Blocked,
    #[serde(rename = "N_A")]
    NotApplicable,
    NotVerified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRole {
    SunoFinalExport,
    SunoProjectZip,
    SunoScreenshot,
    SubscriptionPayment,
    ReleaseWav,
    ReleaseMp3,
    ReleaseMp4,
    ReleaseArtwork,
    AiArtworkOriginal,
    AiArtworkEdited,
    HumanEditedArtwork,
    FinalArtwork,
    ExternalAudioLicense,
    ExternalAudioFile,
    OwnAudioFile,
    SourceCodeFile,
    CodeGeneratedAudioFile,
    ThirdPartySampleFile,
    ThirdPartySampleLicense,
    SunoTermsRights,
    ExternalTimestamp,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionBillingCycle {
    Monthly,
    Annual,
}

impl EvidenceRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SunoFinalExport => "suno_final_export",
            Self::SunoProjectZip => "suno_project_zip",
            Self::SunoScreenshot => "suno_screenshot",
            Self::SubscriptionPayment => "subscription_payment",
            Self::ReleaseWav => "release_wav",
            Self::ReleaseMp3 => "release_mp3",
            Self::ReleaseMp4 => "release_mp4",
            Self::ReleaseArtwork => "release_artwork",
            Self::AiArtworkOriginal => "ai_artwork_original",
            Self::AiArtworkEdited => "ai_artwork_edited",
            Self::HumanEditedArtwork => "human_edited_artwork",
            Self::FinalArtwork => "final_artwork",
            Self::ExternalAudioLicense => "external_audio_license",
            Self::ExternalAudioFile => "external_audio_file",
            Self::OwnAudioFile => "own_audio_file",
            Self::SourceCodeFile => "source_code_file",
            Self::CodeGeneratedAudioFile => "code_generated_audio_file",
            Self::ThirdPartySampleFile => "third_party_sample_file",
            Self::ThirdPartySampleLicense => "third_party_sample_license",
            Self::SunoTermsRights => "suno_terms_rights",
            Self::ExternalTimestamp => "external_timestamp",
            Self::Other => "other",
        }
    }

    pub fn destination(&self) -> &'static str {
        match self {
            Self::ReleaseWav | Self::ReleaseMp3 | Self::ReleaseMp4 => "01_RELEASE",
            Self::SunoFinalExport | Self::SunoProjectZip | Self::SunoScreenshot => "02_SUNO",
            Self::SubscriptionPayment
            | Self::ExternalAudioLicense
            | Self::ThirdPartySampleLicense
            | Self::SunoTermsRights => "04_LICENSES",
            Self::ReleaseArtwork
            | Self::AiArtworkOriginal
            | Self::AiArtworkEdited
            | Self::HumanEditedArtwork
            | Self::FinalArtwork => "05_ARTWORK",
            Self::ExternalAudioFile
            | Self::OwnAudioFile
            | Self::SourceCodeFile
            | Self::CodeGeneratedAudioFile
            | Self::ThirdPartySampleFile => "02_SUNO",
            Self::ExternalTimestamp | Self::Other => "03_DOCUMENTATION",
        }
    }

    pub fn allowed_extensions(&self) -> &'static [&'static str] {
        match self {
            // `release_wav` is the historical authoritative final-audio role. Keep
            // the persisted role name for compatibility while accepting the actual
            // imported release format and preserving its extension.
            Self::ReleaseWav => &["wav", "mp3", "flac", "m4a", "aiff", "aif", "ogg"],
            Self::ReleaseMp3 => &["mp3"],
            Self::ReleaseMp4 => &["mp4", "m4v"],
            Self::SunoProjectZip => &["zip"],
            Self::SunoScreenshot => &["png", "jpg", "jpeg", "webp", "pdf"],
            Self::SubscriptionPayment
            | Self::ExternalAudioLicense
            | Self::ThirdPartySampleLicense => &["pdf", "png", "jpg", "jpeg", "txt", "md"],
            Self::SunoTermsRights => &["pdf", "txt", "md", "html", "htm", "png", "jpg", "jpeg"],
            Self::ExternalTimestamp => &[
                "pdf", "txt", "md", "json", "html", "htm", "png", "jpg", "jpeg",
            ],
            Self::ReleaseArtwork
            | Self::AiArtworkOriginal
            | Self::AiArtworkEdited
            | Self::HumanEditedArtwork
            | Self::FinalArtwork => &["png", "jpg", "jpeg"],
            Self::SunoFinalExport
            | Self::ExternalAudioFile
            | Self::OwnAudioFile
            | Self::ThirdPartySampleFile => &["wav", "mp3", "flac", "m4a", "aiff", "aif", "ogg"],
            Self::CodeGeneratedAudioFile => &["wav", "mp3"],
            Self::SourceCodeFile => &[
                "rb", "py", "txt", "md", "js", "jsx", "ts", "tsx", "mjs", "cjs", "java", "kt",
                "kts", "c", "h", "cc", "cpp", "cxx", "hpp", "cs", "rs", "go", "php", "swift",
                "scala", "sh", "bash", "zsh", "fish", "ps1", "lua", "r", "jl", "ex", "exs", "erl",
                "hrl", "fs", "fsx", "vb", "sql", "html", "htm", "css", "scss", "sass", "less",
                "xml", "yaml", "yml", "toml", "json", "csv", "ipynb", "svg",
            ],
            Self::Other => &[
                "pdf", "png", "jpg", "jpeg", "txt", "md", "json", "zip", "wav", "mp3", "mp4",
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub track_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub artist_name: String,
    pub suno_profile_name: String,
    pub suno_handle: String,
    pub suno_plan: String,
    pub subscription_start_date: String,
    pub default_commercial_use: bool,
    pub default_ai_image_service: String,
    pub artwork_transparency_policy: String,
    pub disclosure_text: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            artist_name: String::new(),
            suno_profile_name: String::new(),
            suno_handle: String::new(),
            suno_plan: String::new(),
            subscription_start_date: String::new(),
            default_commercial_use: true,
            default_ai_image_service: String::new(),
            artwork_transparency_policy: "always".into(),
            disclosure_text: "AI-assisted".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    String(String),
    Vec(Vec<String>),
}

fn string_or_vec(value: StringOrVec) -> Vec<String> {
    match value {
        StringOrVec::String(value) if value.trim().is_empty() => Vec::new(),
        StringOrVec::String(value) => vec![value],
        StringOrVec::Vec(values) => values,
    }
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    StringOrVec::deserialize(deserializer).map(string_or_vec)
}

fn deserialize_optional_string_or_vec<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrVec>::deserialize(deserializer).map(|value| value.map(string_or_vec))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackFields {
    pub title: String,
    pub production_start_date: String,
    pub production_end_date: String,
    pub suno_model: String,
    pub suno_project_url: String,
    pub suno_project_version_id: String,
    pub suno_final_generation_id: String,
    pub suno_final_generation_date: String,
    pub suno_final_generation_time: String,
    pub suno_download_export_date: String,
    pub suno_plan_at_creation: String,
    pub final_export_date: String,
    pub instrumental_track: Option<bool>,
    pub lyrics_source: String,
    pub lyrics_text: String,
    pub suno_style_prompt: String,
    pub external_audio_uploaded: Option<bool>,
    pub external_audio_source: String,
    pub external_audio_ownership: String,
    pub own_audio_uploaded: Option<bool>,
    pub own_audio_source: String,
    pub own_audio_ownership: String,
    pub code_based_generation: Option<bool>,
    pub code_audio_post_processed: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub code_audio_post_processing_operations: Vec<String>,
    pub code_audio_post_processing_note: String,
    pub third_party_samples_uploaded: Option<bool>,
    pub third_party_sample_source: String,
    pub third_party_sample_ownership: String,
    pub human_editing_performed: Option<bool>,
    pub human_editing_details: String,
    pub post_export_editing_performed: Option<bool>,
    pub post_export_editing_details: String,
    pub commercial_use_intended: bool,
    pub release_filename_difference_confirmed: Option<bool>,
    pub suno_export_filename_difference_confirmed: Option<bool>,
    pub suno_terms_evidence_not_available: Option<bool>,
    pub artwork_origin: String,
    pub ai_image_service: String,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub human_artwork_process_operations: Vec<String>,
    pub human_artwork_process_notes: String,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub human_artwork_modifications: Vec<String>,
    pub custom_artwork_change: String,
    pub depicts_real_person: Option<bool>,
    pub real_person_notes: String,
    pub depicts_real_event: Option<bool>,
    pub real_event_notes: String,
    pub contains_trademark: Option<bool>,
    pub trademark_notes: String,
    pub disclosure_applied: Option<bool>,
    pub disclosure_text: String,
    pub release_notes: String,
}

impl Default for TrackFields {
    fn default() -> Self {
        Self {
            title: String::new(),
            production_start_date: String::new(),
            production_end_date: String::new(),
            suno_model: String::new(),
            suno_project_url: String::new(),
            suno_project_version_id: String::new(),
            suno_final_generation_id: String::new(),
            suno_final_generation_date: String::new(),
            suno_final_generation_time: String::new(),
            suno_download_export_date: String::new(),
            suno_plan_at_creation: String::new(),
            final_export_date: String::new(),
            instrumental_track: None,
            lyrics_source: String::new(),
            lyrics_text: String::new(),
            suno_style_prompt: String::new(),
            external_audio_uploaded: None,
            external_audio_source: String::new(),
            external_audio_ownership: String::new(),
            own_audio_uploaded: None,
            own_audio_source: String::new(),
            own_audio_ownership: String::new(),
            code_based_generation: None,
            code_audio_post_processed: None,
            code_audio_post_processing_operations: Vec::new(),
            code_audio_post_processing_note: String::new(),
            third_party_samples_uploaded: None,
            third_party_sample_source: String::new(),
            third_party_sample_ownership: String::new(),
            human_editing_performed: None,
            human_editing_details: String::new(),
            post_export_editing_performed: None,
            post_export_editing_details: String::new(),
            commercial_use_intended: true,
            release_filename_difference_confirmed: None,
            suno_export_filename_difference_confirmed: None,
            suno_terms_evidence_not_available: None,
            artwork_origin: String::new(),
            ai_image_service: String::new(),
            human_artwork_process_operations: Vec::new(),
            human_artwork_process_notes: String::new(),
            human_artwork_modifications: Vec::new(),
            custom_artwork_change: String::new(),
            depicts_real_person: None,
            real_person_notes: String::new(),
            depicts_real_event: None,
            real_event_notes: String::new(),
            contains_trademark: None,
            trademark_notes: String::new(),
            disclosure_applied: None,
            disclosure_text: "AI-assisted".into(),
            release_notes: String::new(),
        }
    }
}

impl TrackFields {
    /// Remove answers that are no longer applicable after a controlling answer changes.
    ///
    /// The UI hides these fields, but the native model is authoritative: callers can submit
    /// partial patches directly and older workspaces can still contain values written by an
    /// earlier application version.
    pub fn normalize_conditionals(&mut self) {
        if self.external_audio_uploaded != Some(true) {
            self.external_audio_source.clear();
            self.external_audio_ownership.clear();
        }
        if self.own_audio_uploaded != Some(true) {
            self.own_audio_source.clear();
            self.own_audio_ownership.clear();
        }
        if self.code_based_generation != Some(true) {
            self.code_audio_post_processed = None;
            self.code_audio_post_processing_operations.clear();
            self.code_audio_post_processing_note.clear();
        } else if self.code_audio_post_processed != Some(true) {
            self.code_audio_post_processing_operations.clear();
            self.code_audio_post_processing_note.clear();
        } else if !self
            .code_audio_post_processing_operations
            .iter()
            .any(|value| value == "Other post-processing")
        {
            self.code_audio_post_processing_note.clear();
        }
        if self.third_party_samples_uploaded != Some(true) {
            self.third_party_sample_source.clear();
            self.third_party_sample_ownership.clear();
        }
        if matches!(self.lyrics_source.as_str(), "" | "instrumental") {
            self.lyrics_text.clear();
        }
        if self.human_editing_performed != Some(true) {
            self.human_editing_details.clear();
        }
        if self.post_export_editing_performed != Some(true) {
            self.post_export_editing_details.clear();
        }

        let artwork_present = !matches!(self.artwork_origin.as_str(), "" | "none");
        if self.artwork_origin != "human" {
            self.human_artwork_process_operations.clear();
            self.human_artwork_process_notes.clear();
        }
        if matches!(self.artwork_origin.as_str(), "human" | "none") {
            self.ai_image_service.clear();
            self.human_artwork_modifications.clear();
            self.custom_artwork_change.clear();
            self.disclosure_applied = None;
            self.disclosure_text.clear();
        } else if self.artwork_origin != "ai_assisted" {
            self.human_artwork_modifications.clear();
            self.custom_artwork_change.clear();
        } else if !self
            .human_artwork_modifications
            .iter()
            .any(|value| value == "Other human editing")
        {
            self.custom_artwork_change.clear();
        }

        if !artwork_present {
            self.depicts_real_person = None;
            self.depicts_real_event = None;
            self.contains_trademark = None;
        }
        if self.depicts_real_person != Some(true) {
            self.real_person_notes.clear();
        }
        if self.depicts_real_event != Some(true) {
            self.real_event_notes.clear();
        }
        if self.contains_trademark != Some(true) {
            self.trademark_notes.clear();
        }
    }

    pub fn normalized_conditionals(&self) -> Self {
        let mut normalized = self.clone();
        normalized.normalize_conditionals();
        normalized
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTrackInput {
    pub title: String,
    #[serde(default)]
    pub production_start_date: String,
    #[serde(default = "default_true")]
    pub commercial_use_intended: bool,
    #[serde(default)]
    pub library: TrackLibraryPlacement,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackPatch {
    pub title: Option<String>,
    pub production_start_date: Option<String>,
    pub production_end_date: Option<String>,
    pub suno_model: Option<String>,
    pub suno_project_url: Option<String>,
    pub suno_project_version_id: Option<String>,
    pub suno_final_generation_id: Option<String>,
    pub suno_final_generation_date: Option<String>,
    pub suno_final_generation_time: Option<String>,
    pub suno_download_export_date: Option<String>,
    pub suno_plan_at_creation: Option<String>,
    pub final_export_date: Option<String>,
    pub instrumental_track: Option<bool>,
    pub lyrics_source: Option<String>,
    pub lyrics_text: Option<String>,
    pub suno_style_prompt: Option<String>,
    pub external_audio_uploaded: Option<bool>,
    pub external_audio_source: Option<String>,
    pub external_audio_ownership: Option<String>,
    pub own_audio_uploaded: Option<bool>,
    pub own_audio_source: Option<String>,
    pub own_audio_ownership: Option<String>,
    pub code_based_generation: Option<bool>,
    pub code_audio_post_processed: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_vec")]
    pub code_audio_post_processing_operations: Option<Vec<String>>,
    pub code_audio_post_processing_note: Option<String>,
    pub third_party_samples_uploaded: Option<bool>,
    pub third_party_sample_source: Option<String>,
    pub third_party_sample_ownership: Option<String>,
    pub human_editing_performed: Option<bool>,
    pub human_editing_details: Option<String>,
    pub post_export_editing_performed: Option<bool>,
    pub post_export_editing_details: Option<String>,
    pub commercial_use_intended: Option<bool>,
    pub release_filename_difference_confirmed: Option<bool>,
    pub suno_export_filename_difference_confirmed: Option<bool>,
    pub suno_terms_evidence_not_available: Option<bool>,
    pub artwork_origin: Option<String>,
    pub ai_image_service: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_vec")]
    pub human_artwork_process_operations: Option<Vec<String>>,
    pub human_artwork_process_notes: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_vec")]
    pub human_artwork_modifications: Option<Vec<String>>,
    pub custom_artwork_change: Option<String>,
    pub depicts_real_person: Option<bool>,
    pub real_person_notes: Option<String>,
    pub depicts_real_event: Option<bool>,
    pub real_event_notes: Option<String>,
    pub contains_trademark: Option<bool>,
    pub trademark_notes: Option<String>,
    pub disclosure_applied: Option<bool>,
    pub disclosure_text: Option<String>,
    pub release_notes: Option<String>,
}

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

/// User-supplied factual metadata for locally archived service-terms or timestamp evidence.
/// Empty values mean "not recorded"; SunoDM never fetches or infers these fields.
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
    pub factual_note: String,
    pub external_timestamp: String,
    pub referenced_hash: String,
    pub referenced_artifact: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertificateState {
    pub valid: bool,
    pub certificate_id: Option<String>,
    pub finalized_at: Option<String>,
    pub workflow_version: Option<String>,
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
    pub fields: TrackFields,
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
    pub fields: TrackFields,
    pub steps: Vec<StepState>,
    pub evidence: Vec<EvidenceItem>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_artwork_freetext_loads_without_data_loss_and_serializes_as_a_list() {
        let fields: TrackFields = serde_json::from_value(serde_json::json!({
            "humanArtworkModifications": "Historischer frei beschreibbarer Wert",
            "codeAudioPostProcessingOperations": "Historische Nachbearbeitung"
        }))
        .expect("historical track fields");

        assert_eq!(
            fields.human_artwork_modifications,
            vec!["Historischer frei beschreibbarer Wert"]
        );
        assert_eq!(
            fields.code_audio_post_processing_operations,
            vec!["Historische Nachbearbeitung"]
        );
        let serialized = serde_json::to_value(fields).expect("serialize migrated fields");
        assert!(serialized["humanArtworkModifications"].is_array());
        assert!(serialized["codeAudioPostProcessingOperations"].is_array());
    }

    #[test]
    fn inactive_code_and_artwork_branches_remove_unclaimed_operations() {
        let mut fields = TrackFields {
            code_based_generation: Some(false),
            code_audio_post_processed: Some(true),
            code_audio_post_processing_operations: vec!["Mixing".into()],
            artwork_origin: "ai_generated".into(),
            human_artwork_process_operations: vec!["Photographed".into()],
            human_artwork_modifications: vec!["Cropping".into()],
            ..TrackFields::default()
        };

        fields.normalize_conditionals();

        assert_eq!(fields.code_audio_post_processed, None);
        assert!(fields.code_audio_post_processing_operations.is_empty());
        assert!(fields.human_artwork_process_operations.is_empty());
        assert!(fields.human_artwork_modifications.is_empty());
    }
}
