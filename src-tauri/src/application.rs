use crate::audio_screening;
use crate::certificate;
use crate::documents;
use crate::error::{AppError, Result};
use crate::evidence;
use crate::external_timestamp;
use crate::folder_import::{self, FolderImportExecutionInput, FolderImportProposal};
use crate::integrity;
#[cfg(test)]
use crate::model::ExternalTimestampInput;
use crate::model::{
    ActionResult, AudioScreeningProviderStatus, AudioScreeningProviderTestResult,
    AudioScreeningSecretInput, AudioScreeningSettings, AudioScreeningStatus, AudioScreeningSummary,
    BlockingDeviation, CertificateRenderOptions, CertificateState, CreateTrackInput,
    DeviationInput, DocumentPreview, DocumentState, EvidenceDerivedField, EvidenceItem,
    EvidenceMetadata, EvidencePreview, EvidenceProvenance, EvidenceRole, ExternalTimestampRecord,
    ExternalTimestampStatus, ExternalTimestampSummary, FinalizationAnchor, FinalizeOptions,
    GlobalEvidenceItem, IntegrityState, LegacyCandidate, OperationProgress, Profile, StepState,
    StepStatus, SubscriptionBillingCycle, SunoContentClassification, SunoLyricsContentType,
    TimestampProviderConfigurationStatus, TimestampProviderTestResult, TimestampSecretInput,
    TimestampSettings, TrackCoverPreview, TrackDetail, TrackLibraryPlacement, TrackLibrarySection,
    TrackPatch, TrackPatchRequest, TrackRecord, TrackStatus, TrackSummary, ValidationResult,
    WorkspaceScan, WorkspaceSummary,
};
use crate::persistence::Persistence;
use crate::security::{
    atomic_write, atomic_write_new, canonical_workspace, contained_path, copy_new,
    ensure_contained_directory, portable_relative, sha256_bytes, sha256_file, slugify,
};
use crate::workflow;
use base64::Engine;
use chrono::{Months, NaiveDate, Utc};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;
use walkdir::WalkDir;

pub const TRACK_FOLDERS: [&str; 8] = [
    ".archive",
    "01_RELEASE",
    "02_SUNO",
    "03_DOCUMENTATION",
    "04_LICENSES",
    "05_ARTWORK",
    "06_CERTIFICATE",
    ".archive/revisions",
];

const SINGLES_DIRECTORY: &str = "Singles";
const TRACK_IDENTITY_FILE: &str = ".summary/track.json";

#[derive(Debug)]
pub struct WorkspaceApp {
    root: PathBuf,
    persistence: Persistence,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FinalizationFailure {
    DatabaseCommit,
    PdfGeneration,
    PdfStaging,
    PdfPublication,
    PostPublishVerification,
}

#[cfg(test)]
impl FinalizationFailure {
    fn certificate_failure(self) -> Option<certificate::CertificateGenerationFailure> {
        match self {
            Self::DatabaseCommit => None,
            Self::PdfGeneration => Some(certificate::CertificateGenerationFailure::PdfGeneration),
            Self::PdfStaging => Some(certificate::CertificateGenerationFailure::PdfStaging),
            Self::PdfPublication => Some(certificate::CertificateGenerationFailure::PdfPublication),
            Self::PostPublishVerification => {
                Some(certificate::CertificateGenerationFailure::PostPublishVerification)
            }
        }
    }
}

mod derived_fields;
mod detail_operations;
mod discovery_operations;
mod document_operations;
mod evidence_operations;
mod finalization_operations;
mod global_evidence_operations;
mod legacy_support;
mod patch_support;
mod revision_operations;
mod revision_support;
mod screening_operations;
mod timestamp_operations;
mod track_operations;
mod validation_support;
mod workspace_operations;

use self::derived_fields::*;
use self::legacy_support::*;
use self::patch_support::*;
use self::revision_support::*;
use self::validation_support::*;

#[cfg(test)]
mod tests;
