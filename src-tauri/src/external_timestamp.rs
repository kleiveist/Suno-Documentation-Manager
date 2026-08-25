use crate::certificate;
use crate::certificate_pdf::{self, ExternalTimestampPdfSnapshot};
use crate::error::{AppError, Result};
use crate::evidence;
use crate::integrity;
use crate::model::{
    CustomRfc3161Settings, ExternalTimestampInput, ExternalTimestampRecord,
    ExternalTimestampStatus, FinalizationAnchor, FinalizationTimestampSnapshot,
    TimestampAuthenticationMode, TimestampProviderCapabilities,
    TimestampProviderConfigurationStatus, TimestampProviderKind, TimestampProviderMetadata,
    TimestampProviderTestResult, TimestampQualificationRecord, TimestampQualificationStatus,
    TimestampReferencedArtifact, TimestampServiceIdentity, TimestampSettings, TimestampType,
};
#[cfg(test)]
use crate::model::{TrustedListEvidence, TrustedListValidationStatus};
use crate::security::{
    atomic_write_new, contained_path, copy_new_hashed, ensure_contained_directory,
    portable_relative, sha256_file, validate_relative,
};
#[cfg(test)]
use chrono::DateTime;
use chrono::Utc;
use rustls_pki_types::CertificateDer;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;
use uuid::Uuid;

pub const EXTERNAL_TIMESTAMPS_DIR: &str = "06_CERTIFICATE/EXTERNAL_TIMESTAMPS";
const STAGING_DIR: &str = ".archive/timestamp-staging";
const RECORD_FILE: &str = "TIMESTAMP_RECORD.json";
const MARKDOWN_FILE: &str = "EXTERNAL_TIMESTAMP_ADDENDUM.md";
const PDF_FILE: &str = "EXTERNAL_TIMESTAMP_ADDENDUM.pdf";
const HASH_LIST_FILE: &str = "TIMESTAMP_RECORD_SHA256.txt";
const PROVIDER_RESPONSE_FILE_PREFIX: &str = "PROVIDER_RESPONSE";
const LEGACY_SIDECAR_FORMAT_VERSION: u32 = 1;
const SIDECAR_FORMAT_VERSION: u32 = 2;
const HASH_LIST_V1_HEADER: &str = "# SunoDM external timestamp sidecar SHA-256 v1\n";
const HASH_LIST_V2_HEADER: &str =
    "# SunoDM external timestamp sidecar SHA-256 v2 (provider qualification audit)\n";
const DISCLAIMER: &str = "The application records technical timestamp evidence separately from provider qualification. It does not infer legal effect; a regulatory qualification is reported only when independently verified.";

/// Centrally defined public presets. They intentionally live only here, so
/// UI components and archive records cannot drift into provider-specific
/// endpoint logic.
pub const FREETSA_ENDPOINT: &str = "https://freetsa.org/tsr";
pub const SIGSTORE_PUBLIC_TSA_ENDPOINT: &str = "https://timestamp.sigstore.dev/api/v1/timestamp";
/// Public chain endpoint retained for a future explicit CMS/trust-chain
/// verifier. It is intentionally not treated as proof of verification today.
pub const SIGSTORE_PUBLIC_TSA_CERTCHAIN_ENDPOINT: &str =
    "https://timestamp.sigstore.dev/api/v1/timestamp/certchain";
pub const OPEN_TIMESTAMPS_POOL_ENDPOINT: &str = "https://a.pool.opentimestamps.org/digest";
const MAX_PROVIDER_RESPONSE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_TRUST_ANCHOR_BYTES: u64 = 5 * 1024 * 1024;
// `RemoteCalendar.submit` returns a serialized `Timestamp`, not a complete
// detached proof file. A usable OpenTimestamps `.ots` file wraps that response
// with this official DetachedTimestampFile prefix, version and SHA-256 file
// hash operation before the original digest and timestamp serialization.
const OPEN_TIMESTAMPS_DETACHED_MAGIC: &[u8] = &[
    0x00, b'O', b'p', b'e', b'n', b'T', b'i', b'm', b'e', b's', b't', b'a', b'm', b'p', b's', 0x00,
    0x00, b'P', b'r', b'o', b'o', b'f', 0x00, 0xbf, 0x89, 0xe2, 0xe8, 0x84, 0xe8, 0x92, 0x94,
];
const OPEN_TIMESTAMPS_DETACHED_VERSION: u8 = 0x01;
const OPEN_TIMESTAMPS_SHA256_FILE_HASH_OP: u8 = 0x08;
pub const RFC3161_CRYPTOGRAPHIC_VERIFIER: &str =
    "sunodm-rfc3161-v2/sigstore-tsa-0.10.0-rsa-pkcs1-pss-strict-eku";

#[derive(Debug, Clone)]
pub struct ProviderFailure {
    pub status: ExternalTimestampStatus,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ProviderRawResponse {
    /// Byte-for-byte provider response kept in addition to the evidence
    /// artifact when an adapter must wrap or otherwise derive that artifact.
    pub bytes: Vec<u8>,
    /// A conservative, non-user-controlled filename extension for the raw
    /// response archive. It is deliberately independent of evidence types.
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct ProviderTimestampResponse {
    pub provider: String,
    pub evidence_extension: String,
    /// The managed timestamp-evidence artifact. For RFC 3161 this is the
    /// untouched `.tsr`; for OpenTimestamps it is a complete detached `.ots`
    /// proof which embeds the untouched calendar response.
    pub evidence_bytes: Vec<u8>,
    /// An optional untouched provider response archive. OpenTimestamps needs
    /// this because its `/digest` response is only a serialized Timestamp, not
    /// itself a complete `.ots` detached proof.
    pub raw_provider_response: Option<ProviderRawResponse>,
    pub timestamp_value: String,
    pub external_reference_id: String,
    pub provider_verification_url: String,
    pub note: String,
    pub metadata: TimestampProviderMetadata,
    pub status: ExternalTimestampStatus,
    pub message: String,
}

/// Outcome captured before the final certificate is rendered. A successful
/// provider response remains in memory so the caller can archive the exact
/// same bytes without issuing a second request.
#[derive(Debug, Clone)]
pub struct FinalizationTimestampAttempt {
    pub snapshot: FinalizationTimestampSnapshot,
    pub response: Option<ProviderTimestampResponse>,
}

mod anchors;
mod configuration;
mod der;
mod labels;
mod provider_request;
mod qualification;
mod rendering;
mod rfc3161_request;
mod rfc3161_verification;
mod sidecar;
mod staging;
mod transport;

use anchors::*;
use configuration::*;
use der::*;
pub use labels::{
    qualification_status_label, referenced_artifact_label, timestamp_status_label,
    timestamp_type_label,
};
#[cfg(test)]
use provider_request::{
    attempt_finalization_timestamp_with_transport, request_timestamp_with_transport,
    test_provider_with_transport,
};
use qualification::load_trust_anchors;
#[cfg(test)]
use qualification::{QUALIFIED_SERVICE_GRANTED_STATUS, QUALIFIED_TIMESTAMP_SERVICE_TYPE};
use rendering::*;
use rfc3161_request::*;
use rfc3161_verification::*;
use sidecar::*;
use transport::*;

pub use anchors::{finalization_anchors, finalized_manifest_anchor};
pub(crate) use configuration::timestamp_status_for_configuration;
pub use configuration::{provider_capabilities, provider_display_name, settings_status};
pub use provider_request::{
    attempt_finalization_timestamp, request_timestamp_for_artifact, test_provider,
};
#[cfg(test)]
pub(crate) use qualification::{
    verify_provider_qualification, TrustedServiceStatusPeriod, ValidatedTrustedListSnapshot,
    ValidatedTrustedService,
};
pub use sidecar::{reconcile_publications, verify_published_record};
#[cfg(test)]
pub use staging::stage;
pub use staging::{
    discard_staged, publish, remove_published_record, stage_provider_response,
    StagedExternalTimestamp,
};

#[cfg(test)]
mod tests;
