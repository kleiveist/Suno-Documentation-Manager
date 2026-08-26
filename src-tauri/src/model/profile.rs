use serde::{Deserialize, Serialize};

/// The language selected for the application UI and the primary Markdown
/// certificate presentation. Finalization always emits both supported PDF
/// languages; the setting does not suppress either PDF.
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
