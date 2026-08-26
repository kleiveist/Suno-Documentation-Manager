use crate::certificate_pdf::{self, CertificatePdfSnapshot};
use crate::error::{AppError, Result};
use crate::integrity::HASH_FILE;
use crate::model::{
    AudioScreeningExternalRecord, AudioScreeningMode, AudioScreeningProviderStatus,
    AudioScreeningState, AudioScreeningStatus, BlockingDeviation, CertificateLanguage,
    CertificateRenderOptions, DocumentationAnswer, EvidenceItem, EvidenceMetadata,
    EvidenceProvenance, EvidenceRole, ExternalTimestampStatus, FactOrigin,
    FinalizationTimestampSnapshot, Profile, StepState, StepStatus, SunoContentClassification,
    SunoLyricsContentSource, TimestampProviderConfigurationStatus, TimestampQualificationStatus,
    TrackFields, TrackRecord, TrustedListValidationStatus, VocalIntent,
};
use crate::security::{
    atomic_write_new, contained_path, copy_new, ensure_contained_directory, portable_relative,
    sha256_bytes, sha256_file,
};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const CERTIFICATE_DIR: &str = "06_CERTIFICATE";
pub const CERTIFICATE_FILE: &str = "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md";
pub const MANIFEST_FILE: &str = "06_CERTIFICATE/EVIDENCE_MANIFEST.json";
pub const CERTIFICATE_HASH_FILE: &str = "06_CERTIFICATE/CERTIFICATE_SHA256.txt";
/// Stable English PDF filename. Older certificate sets use this filename for
/// their single language-selected PDF; new sets keep it as the English PDF.
pub const PDF_FILE: &str = "SunoDM_DOCUMENTATION_CERTIFICATE.pdf";
pub const PDF_FILE_EN: &str = PDF_FILE;
pub const PDF_FILE_DE: &str = "SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf";
pub const CERTIFICATE_FORMAT_VERSION: &str = "6.2";
pub const EVIDENCE_MANIFEST_SCHEMA_VERSION: u32 = 9;

mod audio_screening;
mod generation;
mod integrity;
mod label_replacements_a;
mod label_replacements_b;
mod label_replacements_c;
mod localization;
mod manifest;
mod markdown;
mod publication;
mod relationships;
mod timestamp;
mod verification;

use audio_screening::*;
#[cfg(test)]
pub(crate) use generation::generate;
#[cfg(test)]
pub(crate) use generation::{generate_with_failure, CertificateGenerationFailure};
pub use generation::{generate_with_finalization_timestamp, GenerationInput};
use integrity::*;
use label_replacements_a::*;
use label_replacements_b::*;
use label_replacements_c::*;
#[cfg(test)]
use localization::materialize_markdown_values;
pub(crate) use localization::{localized_certificate_label, localized_certificate_paragraph};
use localization::{localized_markdown_certificate, MARKDOWN_VALUE_END, MARKDOWN_VALUE_START};
use manifest::*;
use markdown::*;
use publication::*;
use relationships::*;
use timestamp::*;
pub use verification::verify;
pub(crate) use verification::{expects_pdf, required_pdf_files};

#[cfg(test)]
mod tests;
