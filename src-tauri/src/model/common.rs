use serde::{Deserialize, Serialize};

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
        const ROLE_NAMES: [&str; 25] = [
            "suno_final_export",
            "suno_project_zip",
            "suno_screenshot",
            "subscription_payment",
            "release_wav",
            "release_mp3",
            "release_mp4",
            "release_artwork",
            "artwork_suno_original",
            "ai_artwork_original",
            "ai_artwork_edited",
            "human_edited_artwork",
            "final_artwork",
            "external_audio_license",
            "external_audio_file",
            "own_audio_file",
            "source_code_file",
            "code_generated_audio_file",
            "third_party_sample_file",
            "third_party_sample_license",
            "suno_terms_rights",
            "external_timestamp",
            "lyrics",
            "style",
            "other",
        ];

        ROLE_NAMES[*self as usize]
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
                "p7s", "ots",
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
