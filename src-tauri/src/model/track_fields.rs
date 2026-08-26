use super::TrackLibraryPlacement;
use serde::{Deserialize, Deserializer, Serialize};

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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SunoContentClassification {
    StructureOnly,
    VocalLyricsOnly,
    Mixed,
    Empty,
    Other,
}

impl SunoContentClassification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StructureOnly => "STRUCTURE_ONLY",
            Self::VocalLyricsOnly => "VOCAL_LYRICS_ONLY",
            Self::Mixed => "MIXED",
            Self::Empty => "EMPTY",
            Self::Other => "OTHER",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VocalIntent {
    Vocal,
    Instrumental,
    Unspecified,
}

impl VocalIntent {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vocal => "VOCAL",
            Self::Instrumental => "INSTRUMENTAL",
            Self::Unspecified => "UNSPECIFIED",
        }
    }
}

/// Legacy multi-value classification retained only so existing track JSON
/// remains readable. New records use `SunoContentClassification` instead.
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
    pub vocal_intent: Option<VocalIntent>,
    /// Legacy YES/NO controller retained only while reading historical track
    /// records. New records use `sunoContentClassification` exclusively.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suno_lyrics_field_content: Option<bool>,
    pub suno_content_classification: Option<SunoContentClassification>,
    /// Legacy multi-value classification. New records leave this empty and
    /// serialize only `sunoContentClassification` once it is documented.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
            vocal_intent: None,
            suno_lyrics_field_content: None,
            suno_content_classification: None,
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
        self.normalize_source_inputs();
        self.normalize_suno_content();
        self.normalize_editing_fields();
        self.normalize_audio_disclosure();
    }

    fn normalize_source_inputs(&mut self) {
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
    }

    fn normalize_suno_content(&mut self) {
        if self.suno_content_classification == Some(SunoContentClassification::Empty) {
            self.suno_lyrics_field_content = None;
            self.suno_lyrics_content_types.clear();
            self.suno_lyrics_content_source = None;
            self.suno_lyrics_field_text.clear();
            self.suno_lyrics_other_content_type.clear();
        } else if self.suno_content_classification.is_some() {
            // A canonical scalar supersedes the historical multi-value field.
            self.suno_lyrics_field_content = None;
            self.suno_lyrics_content_types.clear();
            if self.suno_content_classification != Some(SunoContentClassification::Other) {
                self.suno_lyrics_other_content_type.clear();
            }
        } else if self.suno_lyrics_field_content == Some(false) {
            // Preserve the old conditional cleanup for records that have not
            // yet gone through an explicit workflow upgrade or revision.
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
    }

    fn normalize_editing_fields(&mut self) {
        if self.human_editing_performed != Some(true) {
            self.human_editing_details.clear();
        }
        if self.post_export_editing_performed != Some(true) {
            self.post_export_editing_details.clear();
        }

        let artwork_present = !matches!(self.artwork_origin.as_str(), "" | "none");
        self.normalize_artwork_origin();
        self.normalize_depictions(artwork_present);
    }

    fn normalize_artwork_origin(&mut self) {
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
    }

    fn normalize_depictions(&mut self, artwork_present: bool) {
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

    fn normalize_audio_disclosure(&mut self) {
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
    pub vocal_intent: Option<VocalIntent>,
    pub suno_content_classification: Option<SunoContentClassification>,
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
