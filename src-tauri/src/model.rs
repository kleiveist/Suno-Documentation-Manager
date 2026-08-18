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
    ArtworkSunoOriginal,
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
    Lyrics,
    Style,
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
            Self::ArtworkSunoOriginal => "artwork_suno_original",
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
            Self::Lyrics => "lyrics",
            Self::Style => "style",
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
            | Self::ArtworkSunoOriginal
            | Self::AiArtworkOriginal
            | Self::AiArtworkEdited
            | Self::HumanEditedArtwork
            | Self::FinalArtwork => "05_ARTWORK",
            Self::ExternalAudioFile
            | Self::OwnAudioFile
            | Self::SourceCodeFile
            | Self::CodeGeneratedAudioFile
            | Self::ThirdPartySampleFile => "02_SUNO",
            Self::Lyrics | Self::Style => "02_SUNO",
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
            Self::SunoTermsRights => &["pdf"],
            Self::ExternalTimestamp => &[
                "pdf", "txt", "md", "json", "html", "htm", "png", "jpg", "jpeg", "tsr", "tst",
                "p7s",
            ],
            Self::ReleaseArtwork
            | Self::ArtworkSunoOriginal
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
            Self::Lyrics | Self::Style => &["txt", "md"],
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

/// The primary language used when a new technical certificate is finalized.
///
/// The application UI remains German for now; this setting controls the
/// certificate artifacts only. Keeping the compact ISO-like values on the
/// wire makes the persisted profile and the TypeScript DTO unambiguous.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CertificateLanguage {
    De,
    #[default]
    En,
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
    /// Added after the first persisted profile schema. A field-level default
    /// keeps all pre-language workspaces readable and preserves their former
    /// English-certificate behaviour.
    #[serde(default)]
    pub certificate_language: CertificateLanguage,
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
            certificate_language: CertificateLanguage::En,
        }
    }
}

impl Profile {
    /// Returns whether a profile change affects documents, validation, or the
    /// embedded profile snapshot of an editable track. Certificate language is
    /// deliberately excluded: it is read afresh at finalization and recorded
    /// in the immutable certificate state, so changing only that setting must
    /// not force document and hash regeneration.
    pub fn same_track_documentation_profile(&self, other: &Self) -> bool {
        self.artist_name == other.artist_name
            && self.suno_profile_name == other.suno_profile_name
            && self.suno_handle == other.suno_handle
            && self.suno_plan == other.suno_plan
            && self.subscription_start_date == other.subscription_start_date
            && self.default_commercial_use == other.default_commercial_use
            && self.default_ai_image_service == other.default_ai_image_service
            && self.artwork_transparency_policy == other.artwork_transparency_policy
            && self.disclosure_text == other.disclosure_text
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentationAnswer {
    Yes,
    No,
    NotDocumented,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SunoLyricsContentType {
    VocalLyrics,
    StructureInstructions,
    SoundInstructions,
    ArrangementInstructions,
    Mixed,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SunoLyricsContentSource {
    Human,
    Ai,
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampType {
    QualifiedElectronicTimestampUserDeclared,
    ElectronicTimestamp,
    ExternalIntegrityTimestamp,
    Other,
    NotDocumented,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TimestampReferencedArtifact {
    EvidenceManifest,
    Sha256sums,
    DocumentationCertificateMarkdown,
    CertificatePdf,
    FinalEvidencePackage,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTimestampInput {
    pub provider: String,
    pub timestamp_type: TimestampType,
    pub timestamp_value: String,
    pub referenced_artifact: TimestampReferencedArtifact,
    pub other_referenced_artifact: String,
    pub referenced_sha256: String,
    pub external_reference_id: String,
    pub provider_verification_url: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTimestampRecord {
    pub id: String,
    pub certificate_id: String,
    /// Version of the immutable sidecar record and hash-list contract. Version
    /// zero denotes records written before explicit artifact hashes were pinned.
    #[serde(default)]
    pub sidecar_format_version: u32,
    pub provider: String,
    pub timestamp_type: TimestampType,
    pub timestamp_value: String,
    pub referenced_artifact: TimestampReferencedArtifact,
    pub referenced_artifact_path: String,
    pub referenced_sha256: String,
    pub actual_sha256: String,
    pub referenced_hash_match: Option<bool>,
    pub external_reference_id: String,
    pub provider_verification_url: String,
    pub note: String,
    pub evidence_file_name: String,
    pub evidence_sha256: String,
    /// Hashes of the exact immutable addendum bytes as they were published.
    /// These are deliberately verified without invoking the current renderer.
    #[serde(default)]
    pub markdown_sha256: String,
    #[serde(default)]
    pub pdf_sha256: String,
    pub imported_at: String,
    pub provenance: String,
    pub record_relative_path: String,
    pub markdown_relative_path: String,
    pub pdf_relative_path: String,
    pub hash_list_relative_path: String,
    /// A publication-time fact recorded in the immutable sidecar. This is not
    /// the current integrity result; `integrity_verified` is recomputed at load.
    #[serde(default)]
    pub integrity_verified_at_publication: bool,
    #[serde(default)]
    pub integrity_verified: bool,
    #[serde(default)]
    pub integrity_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizationAnchor {
    pub artifact: TimestampReferencedArtifact,
    pub label: String,
    pub relative_path: String,
    pub sha256: String,
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
    pub suno_plan_at_generation: String,
    #[serde(rename = "legacySunoPlanAtCreation", alias = "sunoPlanAtCreation")]
    pub legacy_suno_plan_at_creation: String,
    pub final_export_date: String,
    pub instrumental_track: Option<bool>,
    pub vocal_lyrics_present: Option<bool>,
    pub suno_lyrics_field_content: Option<bool>,
    pub suno_lyrics_content_types: Vec<SunoLyricsContentType>,
    pub suno_lyrics_content_source: Option<SunoLyricsContentSource>,
    pub suno_lyrics_field_text: String,
    pub suno_lyrics_other_content_type: String,
    #[serde(rename = "legacyLyricsSource", alias = "lyricsSource")]
    pub lyrics_source: String,
    #[serde(rename = "legacyLyricsText", alias = "lyricsText")]
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
    pub generative_ai_used: Option<bool>,
    pub audio_ai_system: String,
    pub ai_assisted_audio_elements: Option<DocumentationAnswer>,
    pub ai_generated_audio_elements: Option<DocumentationAnswer>,
    pub real_person_voice_intentionally_imitated: Option<DocumentationAnswer>,
    pub real_person_identity_intentionally_represented: Option<DocumentationAnswer>,
    pub real_event_represented_as_authentic_recording: Option<DocumentationAnswer>,
    pub real_location_institution_event_presented_as_authentic_ai_recording:
        Option<DocumentationAnswer>,
    pub audio_disclosure_applied: Option<DocumentationAnswer>,
    pub audio_disclosure_locations: Vec<String>,
    pub audio_disclosure_text: String,
    pub audio_disclosure_reason: String,
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
            suno_plan_at_generation: String::new(),
            legacy_suno_plan_at_creation: String::new(),
            final_export_date: String::new(),
            instrumental_track: None,
            vocal_lyrics_present: None,
            suno_lyrics_field_content: None,
            suno_lyrics_content_types: Vec::new(),
            suno_lyrics_content_source: None,
            suno_lyrics_field_text: String::new(),
            suno_lyrics_other_content_type: String::new(),
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
            generative_ai_used: None,
            audio_ai_system: String::new(),
            ai_assisted_audio_elements: None,
            ai_generated_audio_elements: None,
            real_person_voice_intentionally_imitated: None,
            real_person_identity_intentionally_represented: None,
            real_event_represented_as_authentic_recording: None,
            real_location_institution_event_presented_as_authentic_ai_recording: None,
            audio_disclosure_applied: None,
            audio_disclosure_locations: Vec::new(),
            audio_disclosure_text: String::new(),
            audio_disclosure_reason: String::new(),
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
        if self.suno_lyrics_field_content == Some(false) {
            self.suno_lyrics_content_types.clear();
            self.suno_lyrics_content_source = None;
            self.suno_lyrics_field_text.clear();
            self.suno_lyrics_other_content_type.clear();
        } else if !self
            .suno_lyrics_content_types
            .contains(&SunoLyricsContentType::Other)
        {
            self.suno_lyrics_other_content_type.clear();
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

        if self.generative_ai_used == Some(false) {
            self.audio_ai_system.clear();
            self.ai_assisted_audio_elements = None;
            self.ai_generated_audio_elements = None;
            self.real_person_voice_intentionally_imitated = None;
            self.real_person_identity_intentionally_represented = None;
            self.real_event_represented_as_authentic_recording = None;
            self.real_location_institution_event_presented_as_authentic_ai_recording = None;
            self.audio_disclosure_applied = None;
            self.audio_disclosure_locations.clear();
            self.audio_disclosure_text.clear();
            self.audio_disclosure_reason.clear();
        } else if self.generative_ai_used == Some(true) {
            match self.audio_disclosure_applied {
                Some(DocumentationAnswer::Yes) => self.audio_disclosure_reason.clear(),
                Some(DocumentationAnswer::No) => {
                    self.audio_disclosure_locations.clear();
                    self.audio_disclosure_text.clear();
                }
                Some(DocumentationAnswer::NotDocumented) | None => {
                    self.audio_disclosure_locations.clear();
                    self.audio_disclosure_text.clear();
                    self.audio_disclosure_reason.clear();
                }
            }
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
    pub suno_plan_at_generation: Option<String>,
    #[serde(rename = "legacySunoPlanAtCreation", alias = "sunoPlanAtCreation")]
    pub legacy_suno_plan_at_creation: Option<String>,
    pub final_export_date: Option<String>,
    pub instrumental_track: Option<bool>,
    pub vocal_lyrics_present: Option<bool>,
    pub suno_lyrics_field_content: Option<bool>,
    pub suno_lyrics_content_types: Option<Vec<SunoLyricsContentType>>,
    pub suno_lyrics_content_source: Option<SunoLyricsContentSource>,
    pub suno_lyrics_field_text: Option<String>,
    pub suno_lyrics_other_content_type: Option<String>,
    #[serde(rename = "legacyLyricsSource", alias = "lyricsSource")]
    pub lyrics_source: Option<String>,
    #[serde(rename = "legacyLyricsText", alias = "lyricsText")]
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
    pub generative_ai_used: Option<bool>,
    pub audio_ai_system: Option<String>,
    pub ai_assisted_audio_elements: Option<DocumentationAnswer>,
    pub ai_generated_audio_elements: Option<DocumentationAnswer>,
    pub real_person_voice_intentionally_imitated: Option<DocumentationAnswer>,
    pub real_person_identity_intentionally_represented: Option<DocumentationAnswer>,
    pub real_event_represented_as_authentic_recording: Option<DocumentationAnswer>,
    pub real_location_institution_event_presented_as_authentic_ai_recording:
        Option<DocumentationAnswer>,
    pub audio_disclosure_applied: Option<DocumentationAnswer>,
    pub audio_disclosure_locations: Option<Vec<String>>,
    pub audio_disclosure_text: Option<String>,
    pub audio_disclosure_reason: Option<String>,
    pub release_notes: Option<String>,
}

/// Transport wrapper that preserves the difference between an omitted patch
/// property and an explicitly submitted JSON `null` value. Serde's ordinary
/// `Option<T>` representation intentionally treats both cases as `None`, while
/// the guided UI needs `null` to clear a previously documented nullable fact.
#[derive(Debug, Clone)]
pub struct TrackPatchRequest {
    pub patch: TrackPatch,
    pub explicit_null_fields: Vec<String>,
}

impl<'de> Deserialize<'de> for TrackPatchRequest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("track patch must be a JSON object"))?;
        let explicit_null_fields = object
            .iter()
            .filter(|(_, value)| value.is_null())
            .map(|(name, _)| name.clone())
            .collect();
        let patch = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            patch,
            explicit_null_fields,
        })
    }
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
/// server resolves and freezes that value together with this per-action
/// bilingual flag once finalization succeeds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FinalizeOptions {
    pub bilingual: bool,
}

/// Resolved presentation choices for one immutable certificate set.
///
/// `FinalizeOptions` carries only the transient UI switch. The application
/// combines it with the current workspace setting before certificate
/// generation, so this value is also ready to be recorded in the manifest.
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
    pub finalized_at: Option<String>,
    pub workflow_version: Option<String>,
    /// The actual language used to render this immutable certificate set.
    #[serde(default)]
    pub certificate_language: CertificateLanguage,
    /// Whether the immutable certificate set contains both supported
    /// languages in addition to its configured primary language.
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
    #[serde(default)]
    pub external_timestamps: Vec<ExternalTimestampRecord>,
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
    fn documentation_enums_use_stable_snake_case_values() {
        assert_eq!(
            serde_json::to_value(DocumentationAnswer::NotDocumented).expect("answer"),
            "not_documented"
        );
        assert_eq!(
            serde_json::to_value(SunoLyricsContentType::StructureInstructions)
                .expect("content type"),
            "structure_instructions"
        );
        assert_eq!(
            serde_json::to_value(SunoLyricsContentSource::Ai).expect("content source"),
            "ai"
        );
        assert_eq!(
            serde_json::to_value(TimestampType::QualifiedElectronicTimestampUserDeclared)
                .expect("timestamp type"),
            "qualified_electronic_timestamp_user_declared"
        );
        assert_eq!(
            serde_json::to_value(TimestampReferencedArtifact::EvidenceManifest)
                .expect("timestamp artifact"),
            "evidence_manifest"
        );
    }

    #[test]
    fn legacy_profile_and_certificate_state_default_to_english_rendering() {
        let profile: Profile = serde_json::from_value(serde_json::json!({
            "artistName": "Legacy Artist",
            "sunoProfileName": "legacy-profile",
            "sunoHandle": "@legacy",
            "sunoPlan": "Pro",
            "subscriptionStartDate": "2026-01-01",
            "defaultCommercialUse": true,
            "defaultAiImageService": "Legacy tool",
            "artworkTransparencyPolicy": "always",
            "disclosureText": "AI-assisted"
        }))
        .expect("pre-language profile remains readable");
        let certificate: CertificateState = serde_json::from_value(serde_json::json!({
            "valid": true,
            "certificateId": "SDM-legacy",
            "finalizedAt": "2026-08-18T00:00:00Z",
            "workflowVersion": "1.7"
        }))
        .expect("pre-language certificate state remains readable");

        assert_eq!(profile.certificate_language, CertificateLanguage::En);
        assert_eq!(certificate.certificate_language, CertificateLanguage::En);
        assert!(!certificate.bilingual);
        assert_eq!(
            serde_json::to_value(CertificateLanguage::De).expect("certificate language"),
            "de"
        );
    }

    #[test]
    fn no_not_documented_and_not_applicable_remain_distinct_states() {
        let mut fields = TrackFields::default();
        let unanswered = serde_json::to_value(&fields).expect("unanswered fields");
        assert!(unanswered["vocalLyricsPresent"].is_null());

        fields.vocal_lyrics_present = Some(false);
        fields.audio_disclosure_applied = Some(DocumentationAnswer::No);
        let documented_no = serde_json::to_value(&fields).expect("documented no");
        assert_eq!(documented_no["vocalLyricsPresent"], false);
        assert_eq!(documented_no["audioDisclosureApplied"], "no");

        fields.audio_disclosure_applied = Some(DocumentationAnswer::NotDocumented);
        let not_documented = serde_json::to_value(&fields).expect("not documented");
        assert_eq!(not_documented["audioDisclosureApplied"], "not_documented");
        assert_eq!(
            serde_json::to_value(StepStatus::NotApplicable).expect("not applicable"),
            "N_A"
        );
    }

    #[test]
    fn historical_lyrics_and_plan_keys_remain_unclassified_legacy_values() {
        let fields: TrackFields = serde_json::from_value(serde_json::json!({
            "lyricsSource": "mixed",
            "lyricsText": "Historical lyrics",
            "sunoPlanAtCreation": "Pro"
        }))
        .expect("historical fields");

        assert_eq!(fields.lyrics_source, "mixed");
        assert_eq!(fields.lyrics_text, "Historical lyrics");
        assert!(fields.suno_plan_at_generation.is_empty());
        assert_eq!(fields.legacy_suno_plan_at_creation, "Pro");

        let serialized = serde_json::to_value(fields).expect("serialize fields");
        assert_eq!(serialized["legacyLyricsSource"], "mixed");
        assert_eq!(serialized["legacyLyricsText"], "Historical lyrics");
        assert_eq!(serialized["sunoPlanAtGeneration"], "");
        assert_eq!(serialized["legacySunoPlanAtCreation"], "Pro");
        assert!(serialized.get("lyricsSource").is_none());
        assert!(serialized.get("lyricsText").is_none());
        assert!(serialized.get("sunoPlanAtCreation").is_none());
    }

    #[test]
    fn patch_request_distinguishes_explicit_null_from_an_omitted_field() {
        let request: TrackPatchRequest = serde_json::from_value(serde_json::json!({
            "generativeAiUsed": null,
            "instrumentalTrack": null,
            "title": "Updated title"
        }))
        .expect("track patch request");

        assert_eq!(request.patch.title.as_deref(), Some("Updated title"));
        assert!(request.patch.generative_ai_used.is_none());
        assert!(request.patch.instrumental_track.is_none());
        assert!(request
            .explicit_null_fields
            .contains(&"generativeAiUsed".to_owned()));
        assert!(request
            .explicit_null_fields
            .contains(&"instrumentalTrack".to_owned()));
        assert!(!request
            .explicit_null_fields
            .contains(&"vocalLyricsPresent".to_owned()));
    }

    #[test]
    fn instrumental_normalization_preserves_legacy_and_structure_field_text() {
        let mut fields = TrackFields {
            instrumental_track: Some(true),
            vocal_lyrics_present: Some(false),
            suno_lyrics_field_content: Some(true),
            suno_lyrics_content_types: vec![SunoLyricsContentType::StructureInstructions],
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
            suno_lyrics_field_text: "[Intro]\n[Instrumental]".into(),
            suno_lyrics_other_content_type: "inactive note".into(),
            lyrics_source: "instrumental".into(),
            lyrics_text: "historical field value".into(),
            ..TrackFields::default()
        };

        fields.normalize_conditionals();

        assert_eq!(fields.lyrics_text, "historical field value");
        assert_eq!(fields.suno_lyrics_field_text, "[Intro]\n[Instrumental]");
        assert_eq!(
            fields.suno_lyrics_content_types,
            vec![SunoLyricsContentType::StructureInstructions]
        );
        assert!(fields.suno_lyrics_other_content_type.is_empty());
    }

    #[test]
    fn inactive_new_lyrics_and_audio_branches_are_cleared_without_inventing_answers() {
        let mut fields = TrackFields {
            suno_lyrics_field_content: Some(false),
            suno_lyrics_content_types: vec![SunoLyricsContentType::Other],
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: "stale field text".into(),
            suno_lyrics_other_content_type: "stale other type".into(),
            generative_ai_used: Some(false),
            audio_ai_system: "stale system".into(),
            ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
            audio_disclosure_applied: Some(DocumentationAnswer::No),
            audio_disclosure_reason: "stale reason".into(),
            ..TrackFields::default()
        };

        fields.normalize_conditionals();

        assert!(fields.suno_lyrics_content_types.is_empty());
        assert!(fields.suno_lyrics_content_source.is_none());
        assert!(fields.suno_lyrics_field_text.is_empty());
        assert!(fields.suno_lyrics_other_content_type.is_empty());
        assert!(fields.audio_ai_system.is_empty());
        assert!(fields.ai_assisted_audio_elements.is_none());
        assert!(fields.audio_disclosure_applied.is_none());
        assert!(fields.audio_disclosure_reason.is_empty());
    }

    #[test]
    fn unanswered_controlling_questions_do_not_erase_pending_new_facts() {
        let mut fields = TrackFields {
            suno_lyrics_field_content: None,
            suno_lyrics_field_text: "pending field text".into(),
            generative_ai_used: None,
            audio_ai_system: "pending system".into(),
            ..TrackFields::default()
        };

        fields.normalize_conditionals();

        assert_eq!(fields.suno_lyrics_field_text, "pending field text");
        assert_eq!(fields.audio_ai_system, "pending system");
    }

    #[test]
    fn external_timestamp_role_accepts_standard_timestamp_container_extensions() {
        let extensions = EvidenceRole::ExternalTimestamp.allowed_extensions();
        assert!(extensions.contains(&"tsr"));
        assert!(extensions.contains(&"tst"));
        assert!(extensions.contains(&"p7s"));
        assert_eq!(
            EvidenceRole::ExternalTimestamp.destination(),
            "03_DOCUMENTATION"
        );
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
