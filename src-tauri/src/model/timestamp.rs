use serde::{Deserialize, Serialize};

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

/// The globally configured provider used for post-finalization timestamp
/// evidence.  These are deliberately provider *kinds*, rather than free-form
/// labels, so the workflow can select a dedicated adapter without putting
/// provider-specific behavior into the UI.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampProviderKind {
    #[default]
    Disabled,
    FreeTsa,
    OpenTimestamps,
    SigstorePublicTsa,
    CustomRfc3161,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampAuthenticationMode {
    #[default]
    None,
    Basic,
    BearerToken,
    ApiKey,
    ClientCertificate,
}

/// Technical status of one concrete external-timestamp attempt/evidence item.
/// Provider configuration and regulatory qualification deliberately use
/// separate status types below.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalTimestampStatus {
    #[default]
    NotRecorded,
    Requesting,
    Attached,
    Verified,
    VerificationFailed,
    ProviderUnavailable,
    AuthenticationFailed,
    AnchorMismatch,
    Disabled,
    Ready,
    ConfigurationIncomplete,
    AuthenticationRequired,
    ConnectionFailed,
    UnsupportedResponse,
    VerificationConfigurationIncomplete,
}

/// Technical readiness of the globally configured timestamp provider. This
/// says nothing about a concrete timestamp and nothing about eIDAS or another
/// regulatory qualification.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampProviderConfigurationStatus {
    #[default]
    Disabled,
    #[serde(alias = "configuration_incomplete")]
    NotConfigured,
    Ready,
    AuthenticationRequired,
    AuthenticationFailed,
    #[serde(alias = "provider_unavailable")]
    ConnectionFailed,
    VerificationConfigurationIncomplete,
    #[serde(
        alias = "not_recorded",
        alias = "requesting",
        alias = "attached",
        alias = "verified",
        alias = "verification_failed",
        alias = "anchor_mismatch",
        alias = "unsupported_response"
    )]
    ProviderError,
}

/// Independent classification of the externally verifiable identity and
/// qualification of a timestamp service. This never describes the validity
/// of an individual timestamp.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampQualificationStatus {
    #[default]
    NotChecked,
    NotDocumented,
    NotVerified,
    ProviderIdentityVerified,
    TrustServiceVerified,
    QualifiedServiceVerified,
    CheckFailed,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustedListValidationStatus {
    #[default]
    NotChecked,
    Verified,
    Failed,
}

/// Cryptographically recognized identity from the timestamp response. The
/// freely configured provider label and endpoint are intentionally absent.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TimestampServiceIdentity {
    pub certificate_sha256: String,
    pub certificate_subject: String,
    pub certificate_issuer: String,
    pub certificate_serial_number: String,
    pub policy_oid: String,
    pub service_identifier: String,
}

/// Immutable evidence identifying the exact official Trusted List snapshot
/// used by a qualification verifier.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrustedListEvidence {
    pub source: String,
    pub territory: String,
    pub version: String,
    pub sequence_number: String,
    pub issued_at: String,
    pub next_update: String,
    pub sha256: String,
    pub validation_status: TrustedListValidationStatus,
    pub validated_at: String,
}

/// Provider-neutral audit result for identity/trust-service/qualification
/// lookup. It is independent from ExternalTimestampStatus, including when a
/// lookup fails or historical status cannot be established.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TimestampQualificationRecord {
    pub status: TimestampQualificationStatus,
    pub provider_identity_status: TimestampQualificationStatus,
    pub trust_service_status: TimestampQualificationStatus,
    pub eidas_qualification_status: TimestampQualificationStatus,
    pub current_qualification_status: TimestampQualificationStatus,
    pub qualification_at_timestamp: TimestampQualificationStatus,
    pub checked_at: String,
    pub message: String,
    pub identity: TimestampServiceIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trusted_list: Option<TrustedListEvidence>,
    pub trust_service_provider: String,
    pub trust_service_name: String,
    pub service_type: String,
    /// Official service status applicable at the documented timestamp time.
    pub service_status: String,
    /// Official service status applicable at the qualification-check time.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_service_status: String,
    pub service_identifier: String,
    pub qualification_type: String,
    /// Service-status period applicable at the documented timestamp time.
    pub status_valid_from: String,
    pub status_valid_until: String,
    /// Service-status period applicable at the qualification-check time.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_status_valid_from: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_status_valid_until: String,
}

/// Immutable provider/configuration/technical/trust state captured between
/// creation of the manifest anchor and the single rendering of the final
/// certificate. Provider response bytes are deliberately not part of this
/// presentation snapshot; successful responses are archived separately.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FinalizationTimestampSnapshot {
    pub provider: String,
    pub provider_configuration_status: TimestampProviderConfigurationStatus,
    pub provider_configuration_message: String,
    pub automatic_request_enabled: bool,
    pub technical_status: ExternalTimestampStatus,
    pub technical_message: String,
    pub timestamp_value: String,
    pub external_reference_id: String,
    pub provider_verification_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<TimestampProviderMetadata>,
}

impl Default for FinalizationTimestampSnapshot {
    fn default() -> Self {
        Self {
            provider: "Disabled".into(),
            provider_configuration_status: TimestampProviderConfigurationStatus::Disabled,
            provider_configuration_message: "External timestamp service is disabled.".into(),
            automatic_request_enabled: false,
            technical_status: ExternalTimestampStatus::NotRecorded,
            technical_message:
                "No automatic external timestamp was requested for this finalization.".into(),
            timestamp_value: String::new(),
            external_reference_id: String::new(),
            provider_verification_url: String::new(),
            provider_metadata: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TimestampProviderCapabilities {
    pub rfc3161: bool,
    pub open_timestamps: bool,
    pub requires_authentication: bool,
    pub supports_sha256: bool,
    pub supports_offline_verification: bool,
    pub returns_signed_timestamp: bool,
    pub external_trust_root_available: bool,
    /// Intentionally only an informational capability. A successful provider
    /// response must never be presented as a qualified timestamp solely from
    /// this value.
    pub qualification_status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomRfc3161Settings {
    pub provider_name: String,
    pub endpoint: String,
    pub authentication_mode: TimestampAuthenticationMode,
    pub username: String,
    pub client_certificate_path: String,
    pub ca_certificate_path: String,
    pub policy_oid: String,
    pub timeout_seconds: u32,
}

/// Public global settings. Secrets are deliberately absent: they are held in
/// a separate local configuration file and are never copied to a profile,
/// track, revision, manifest, or certificate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimestampSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: TimestampProviderKind,
    #[serde(default)]
    pub auto_after_finalization: bool,
    #[serde(default)]
    pub custom: CustomRfc3161Settings,
    /// Derived on read/update; update payloads cannot choose a misleading
    /// status. It is persisted only as harmless UX history.
    #[serde(default)]
    pub status: TimestampProviderConfigurationStatus,
    #[serde(default)]
    pub status_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tested_at: Option<String>,
}

impl Default for TimestampSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: TimestampProviderKind::Disabled,
            auto_after_finalization: false,
            custom: CustomRfc3161Settings {
                timeout_seconds: 15,
                ..Default::default()
            },
            status: TimestampProviderConfigurationStatus::Disabled,
            status_message: "External timestamp service is disabled.".into(),
            last_tested_at: None,
        }
    }
}

/// Write-only input for a Custom RFC 3161 secret. It intentionally does not
/// implement `Serialize`, which prevents accidental inclusion in returned DTOs
/// or normal JSON persistence.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampSecretInput {
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimestampProviderTestResult {
    pub provider: TimestampProviderKind,
    pub status: TimestampProviderConfigurationStatus,
    pub message: String,
    pub tested_at: String,
    pub capabilities: TimestampProviderCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TimestampProviderMetadata {
    pub adapter: String,
    pub protocol: String,
    pub request_algorithm: String,
    pub response_format: String,
    pub provider_endpoint_identifier: String,
    /// Name and hash of the untouched bytes received from the provider. For
    /// RFC 3161 this is normally the main `.tsr` evidence file; OpenTimestamps
    /// additionally archives its raw calendar response next to a usable `.ots`
    /// detached proof wrapper.
    pub provider_response_file_name: String,
    pub provider_response_sha256: String,
    /// Immutable phase-one snapshot identity captured when this evidence was
    /// attached. It is distinct from the certificate ID.
    pub referenced_revision_id: String,
    pub issuer: String,
    pub certificate_subject: String,
    pub certificate_serial_number: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub certificate_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_identity_verified: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_verification_applicable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_chain_verification_applicable: Option<bool>,
    /// Request/response binding values retained for independent review of the
    /// RFC 3161 exchange. Empty values keep historical sidecars byte-compatible.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub request_nonce: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response_nonce: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce_match: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub requested_policy_oid: String,
    pub policy_oid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_match: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cryptographic_verifier: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_anchor_sha256: Vec<String>,
    pub response_structure_valid: Option<bool>,
    pub provider_digest_match: Option<bool>,
    pub signature_verified: Option<bool>,
    pub trust_chain_verified: Option<bool>,
    #[serde(default)]
    pub verification_result: ExternalTimestampStatus,
    pub verification_message: String,
    pub verification_timestamp: String,
    /// Separate immutable trust/qualification audit evidence. Its absence on
    /// historical records means NOT CHECKED and never "not qualified".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification: Option<TimestampQualificationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTimestampSummary {
    pub status: ExternalTimestampStatus,
    pub message: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Default for ExternalTimestampSummary {
    fn default() -> Self {
        Self {
            status: ExternalTimestampStatus::NotRecorded,
            message:
                "No external timestamp evidence has been recorded for this finalized snapshot."
                    .into(),
            provider: String::new(),
            record_id: None,
            updated_at: None,
        }
    }
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
    /// Provider-derived response metadata. Legacy manually recorded evidence
    /// leaves this absent and is never silently promoted to verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<TimestampProviderMetadata>,
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
