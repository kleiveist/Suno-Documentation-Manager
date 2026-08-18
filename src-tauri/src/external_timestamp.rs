use crate::certificate;
use crate::certificate_pdf::{self, ExternalTimestampPdfSnapshot};
use crate::error::{AppError, Result};
use crate::evidence;
use crate::integrity;
use crate::model::{
    CustomRfc3161Settings, ExternalTimestampInput, ExternalTimestampRecord,
    ExternalTimestampStatus, FinalizationAnchor, TimestampAuthenticationMode,
    TimestampProviderCapabilities, TimestampProviderKind, TimestampProviderMetadata,
    TimestampProviderTestResult, TimestampReferencedArtifact, TimestampSettings, TimestampType,
};
use crate::security::{
    atomic_write_new, contained_path, copy_new_hashed, ensure_contained_directory,
    portable_relative, sha256_file, validate_relative,
};
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
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
const SIDECAR_FORMAT_VERSION: u32 = 1;
const HASH_LIST_V1_HEADER: &str = "# SunoDM external timestamp sidecar SHA-256 v1\n";
const DISCLAIMER: &str = "The application records the external timestamp evidence and its referenced hash. It does not determine any legal qualification of the timestamp.";

/// Centrally defined public presets. They intentionally live only here, so
/// UI components and archive records cannot drift into provider-specific
/// endpoint logic.
pub const FREETSA_ENDPOINT: &str = "https://freetsa.org/tsr";
pub const SIGSTORE_PUBLIC_TSA_ENDPOINT: &str = "https://timestamp.sigstore.dev/api/v1/timestamp";
/// Public chain endpoint retained for a future explicit CMS/trust-chain
/// verifier. It is intentionally not treated as proof of verification today.
#[allow(dead_code)]
pub const SIGSTORE_PUBLIC_TSA_CERTCHAIN_ENDPOINT: &str =
    "https://timestamp.sigstore.dev/api/v1/timestamp/certchain";
pub const OPEN_TIMESTAMPS_POOL_ENDPOINT: &str = "https://a.pool.opentimestamps.org/digest";
const MAX_PROVIDER_RESPONSE_BYTES: u64 = 10 * 1024 * 1024;
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

#[derive(Debug, Clone)]
struct HttpRequest {
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout_seconds: u32,
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

/// Kept behind a narrow interface so provider parsing and attachment behavior
/// can be tested with deterministic byte-for-byte fake responses without a
/// network connection or a real TSA account.
trait TimestampHttpTransport {
    fn post(&self, request: HttpRequest) -> std::result::Result<HttpResponse, ProviderFailure>;
}

struct UreqTimestampHttpTransport;

impl TimestampHttpTransport for UreqTimestampHttpTransport {
    fn post(&self, request: HttpRequest) -> std::result::Result<HttpResponse, ProviderFailure> {
        let timeout = Duration::from_secs(u64::from(request.timeout_seconds.max(1)));
        let agent = ureq::AgentBuilder::new().timeout(timeout).build();
        let mut outgoing = agent.post(&request.url);
        for (name, value) in request.headers {
            outgoing = outgoing.set(&name, &value);
        }
        match outgoing.send_bytes(&request.body) {
            Ok(response) => read_http_response(response),
            Err(ureq::Error::Status(_, response)) => read_http_response(response),
            Err(ureq::Error::Transport(_)) => Err(ProviderFailure {
                status: ExternalTimestampStatus::ProviderUnavailable,
                message: "Timestamp provider could not be reached.".into(),
            }),
        }
    }
}

fn read_http_response(
    response: ureq::Response,
) -> std::result::Result<HttpResponse, ProviderFailure> {
    let status = response.status() as u16;
    let mut reader = response.into_reader().take(MAX_PROVIDER_RESPONSE_BYTES + 1);
    let mut body = Vec::new();
    reader.read_to_end(&mut body).map_err(|_| ProviderFailure {
        status: ExternalTimestampStatus::ProviderUnavailable,
        message: "Timestamp provider response could not be read.".into(),
    })?;
    if body.len() as u64 > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::UnsupportedResponse,
            message: "Timestamp provider response exceeds the supported size limit.".into(),
        });
    }
    Ok(HttpResponse { status, body })
}

trait TimestampProviderAdapter {
    fn display_name(&self, settings: &TimestampSettings) -> String;
    fn capabilities(&self) -> TimestampProviderCapabilities;
    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure>;
}

struct Rfc3161TimestampAdapter {
    provider: &'static str,
    endpoint: &'static str,
    adapter: &'static str,
    trust_root_available: bool,
}

struct OpenTimestampsAdapter;

struct CustomRfc3161Adapter;

fn provider_adapter(kind: TimestampProviderKind) -> Option<Box<dyn TimestampProviderAdapter>> {
    match kind {
        TimestampProviderKind::FreeTsa => Some(Box::new(Rfc3161TimestampAdapter {
            provider: "FreeTSA",
            endpoint: FREETSA_ENDPOINT,
            adapter: "freetsa_rfc3161",
            trust_root_available: false,
        })),
        TimestampProviderKind::SigstorePublicTsa => Some(Box::new(Rfc3161TimestampAdapter {
            provider: "Sigstore Public TSA",
            endpoint: SIGSTORE_PUBLIC_TSA_ENDPOINT,
            adapter: "sigstore_public_tsa_rfc3161",
            trust_root_available: true,
        })),
        TimestampProviderKind::OpenTimestamps => Some(Box::new(OpenTimestampsAdapter)),
        TimestampProviderKind::CustomRfc3161 => Some(Box::new(CustomRfc3161Adapter)),
        TimestampProviderKind::Disabled => None,
    }
}

pub fn provider_capabilities(kind: TimestampProviderKind) -> TimestampProviderCapabilities {
    provider_adapter(kind)
        .map(|adapter| adapter.capabilities())
        .unwrap_or_default()
}

pub fn provider_display_name(settings: &TimestampSettings) -> String {
    provider_adapter(settings.provider)
        .map(|adapter| adapter.display_name(settings))
        .unwrap_or_else(|| "Disabled".into())
}

/// Validate public settings without performing a network operation. The
/// returned text is suitable for the UI and never includes credentials.
pub fn settings_status(
    settings: &TimestampSettings,
    secret_available: bool,
) -> (ExternalTimestampStatus, String) {
    if !settings.enabled || settings.provider == TimestampProviderKind::Disabled {
        return (
            ExternalTimestampStatus::Disabled,
            "External timestamp service is disabled.".into(),
        );
    }
    if settings.provider != TimestampProviderKind::CustomRfc3161 {
        return (
            ExternalTimestampStatus::Ready,
            "Timestamp service is ready to attach external timestamp evidence.".into(),
        );
    }
    match validate_custom_settings(&settings.custom, secret_available) {
        Ok(()) => (
            ExternalTimestampStatus::Ready,
            "Custom RFC 3161 timestamp service is configured.".into(),
        ),
        Err(failure) => (failure.status, failure.message),
    }
}

fn validate_custom_settings(
    custom: &CustomRfc3161Settings,
    secret_available: bool,
) -> std::result::Result<(), ProviderFailure> {
    if custom.endpoint.trim().is_empty() {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::ConfigurationIncomplete,
            message: "Custom RFC 3161 TSA endpoint is required.".into(),
        });
    }
    let parsed = Url::parse(custom.endpoint.trim()).map_err(|_| ProviderFailure {
        status: ExternalTimestampStatus::ConfigurationIncomplete,
        message: "Custom RFC 3161 TSA endpoint is not a valid HTTP(S) URL.".into(),
    })?;
    if !matches!(parsed.scheme(), "https" | "http")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::ConfigurationIncomplete,
            message: "Custom RFC 3161 TSA endpoint must be a plain HTTP(S) URL without embedded credentials or query values."
                .into(),
        });
    }
    if custom.timeout_seconds == 0 || custom.timeout_seconds > 120 {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::ConfigurationIncomplete,
            message: "Custom RFC 3161 timeout must be between 1 and 120 seconds.".into(),
        });
    }
    if !custom.policy_oid.trim().is_empty() && !valid_oid(custom.policy_oid.trim()) {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::ConfigurationIncomplete,
            message: "Custom RFC 3161 policy OID is invalid.".into(),
        });
    }
    match custom.authentication_mode {
        TimestampAuthenticationMode::None => Ok(()),
        TimestampAuthenticationMode::ClientCertificate => {
            if custom.client_certificate_path.trim().is_empty() {
                return Err(ProviderFailure {
                    status: ExternalTimestampStatus::ConfigurationIncomplete,
                    message: "A client certificate path is required for client-certificate authentication."
                        .into(),
                });
            }
            Err(ProviderFailure {
                status: ExternalTimestampStatus::VerificationConfigurationIncomplete,
                message: "Client-certificate authentication is prepared but is not enabled by this provider adapter yet."
                    .into(),
            })
        }
        TimestampAuthenticationMode::Basic => {
            if custom.username.trim().is_empty() {
                return Err(ProviderFailure {
                    status: ExternalTimestampStatus::ConfigurationIncomplete,
                    message: "A username is required for Basic authentication.".into(),
                });
            }
            if !secret_available {
                return Err(ProviderFailure {
                    status: ExternalTimestampStatus::AuthenticationRequired,
                    message:
                        "A password or token is required for the configured timestamp service."
                            .into(),
                });
            }
            Ok(())
        }
        TimestampAuthenticationMode::BearerToken | TimestampAuthenticationMode::ApiKey => {
            if !secret_available {
                return Err(ProviderFailure {
                    status: ExternalTimestampStatus::AuthenticationRequired,
                    message:
                        "A password or token is required for the configured timestamp service."
                            .into(),
                });
            }
            Ok(())
        }
    }
}

pub fn test_provider(
    settings: &TimestampSettings,
    secret: Option<&str>,
) -> TimestampProviderTestResult {
    test_provider_with_transport(settings, secret, &UreqTimestampHttpTransport)
}

fn test_provider_with_transport(
    settings: &TimestampSettings,
    secret: Option<&str>,
    transport: &dyn TimestampHttpTransport,
) -> TimestampProviderTestResult {
    let tested_at = Utc::now().to_rfc3339();
    let secret_available = secret.is_some_and(|value| !value.trim().is_empty());
    let (configuration_status, configuration_message) = settings_status(settings, secret_available);
    let capabilities = provider_capabilities(settings.provider);
    if configuration_status != ExternalTimestampStatus::Ready {
        return TimestampProviderTestResult {
            provider: settings.provider,
            status: configuration_status,
            message: configuration_message,
            tested_at,
            capabilities,
        };
    }
    match request_timestamp_with_transport(settings, secret, &"00".repeat(32), transport) {
        Ok(response) => {
            let (status, message) =
                if response.status == ExternalTimestampStatus::VerificationFailed {
                    (
                    ExternalTimestampStatus::UnsupportedResponse,
                    "Provider responded, but its test response could not be technically verified."
                        .into(),
                )
                } else {
                    (
                        ExternalTimestampStatus::Ready,
                        "Timestamp service ready.".into(),
                    )
                };
            TimestampProviderTestResult {
                provider: settings.provider,
                status,
                message,
                tested_at,
                capabilities,
            }
        }
        Err(failure) => TimestampProviderTestResult {
            provider: settings.provider,
            status: match failure.status {
                ExternalTimestampStatus::ProviderUnavailable => {
                    ExternalTimestampStatus::ConnectionFailed
                }
                other => other,
            },
            message: failure.message,
            tested_at,
            capabilities,
        },
    }
}

pub fn request_timestamp(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
    request_timestamp_with_transport(settings, secret, digest, &UreqTimestampHttpTransport)
}

fn request_timestamp_with_transport(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    transport: &dyn TimestampHttpTransport,
) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::AnchorMismatch,
            message: "Timestamp anchor digest is not a SHA-256 value.".into(),
        });
    }
    let secret_available = secret.is_some_and(|value| !value.trim().is_empty());
    let (status, message) = settings_status(settings, secret_available);
    if status != ExternalTimestampStatus::Ready {
        return Err(ProviderFailure { status, message });
    }
    let adapter = provider_adapter(settings.provider).ok_or_else(|| ProviderFailure {
        status: ExternalTimestampStatus::Disabled,
        message: "External timestamp service is disabled.".into(),
    })?;
    adapter.request(settings, secret, digest, transport)
}

impl TimestampProviderAdapter for Rfc3161TimestampAdapter {
    fn display_name(&self, _settings: &TimestampSettings) -> String {
        self.provider.into()
    }

    fn capabilities(&self) -> TimestampProviderCapabilities {
        TimestampProviderCapabilities {
            rfc3161: true,
            open_timestamps: false,
            requires_authentication: false,
            supports_sha256: true,
            supports_offline_verification: true,
            returns_signed_timestamp: true,
            external_trust_root_available: self.trust_root_available,
            qualification_status: "unknown_not_qualified".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
        request_rfc3161(
            self.provider,
            self.endpoint,
            self.adapter,
            None,
            settings,
            secret,
            digest,
            transport,
        )
    }
}

impl TimestampProviderAdapter for CustomRfc3161Adapter {
    fn display_name(&self, settings: &TimestampSettings) -> String {
        let name = settings.custom.provider_name.trim();
        if name.is_empty() {
            "Custom RFC 3161".into()
        } else {
            name.into()
        }
    }

    fn capabilities(&self) -> TimestampProviderCapabilities {
        TimestampProviderCapabilities {
            rfc3161: true,
            open_timestamps: false,
            requires_authentication: true,
            supports_sha256: true,
            supports_offline_verification: true,
            returns_signed_timestamp: true,
            external_trust_root_available: false,
            qualification_status: "unknown_not_qualified".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
        let provider = self.display_name(settings);
        request_rfc3161(
            &provider,
            settings.custom.endpoint.trim(),
            "custom_rfc3161",
            Some(&settings.custom),
            settings,
            secret,
            digest,
            transport,
        )
    }
}

impl TimestampProviderAdapter for OpenTimestampsAdapter {
    fn display_name(&self, _settings: &TimestampSettings) -> String {
        "OpenTimestamps".into()
    }

    fn capabilities(&self) -> TimestampProviderCapabilities {
        TimestampProviderCapabilities {
            rfc3161: false,
            open_timestamps: true,
            requires_authentication: false,
            supports_sha256: true,
            supports_offline_verification: true,
            returns_signed_timestamp: false,
            external_trust_root_available: false,
            qualification_status: "unknown_not_qualified".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        _secret: Option<&str>,
        digest: &str,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
        let body = decode_sha256_hex(digest).map_err(|message| ProviderFailure {
            status: ExternalTimestampStatus::AnchorMismatch,
            message,
        })?;
        let response = transport.post(HttpRequest {
            url: OPEN_TIMESTAMPS_POOL_ENDPOINT.into(),
            headers: vec![
                (
                    "Content-Type".into(),
                    "application/vnd.opentimestamps.v1".into(),
                ),
                ("Accept".into(), "application/vnd.opentimestamps.v1".into()),
            ],
            body,
            timeout_seconds: configured_timeout_seconds(settings),
        })?;
        ensure_successful_provider_http_response(&response)?;
        if response.body.is_empty() {
            return Err(ProviderFailure {
                status: ExternalTimestampStatus::UnsupportedResponse,
                message: "OpenTimestamps returned an empty proof.".into(),
            });
        }
        // A calendar response is only a serialized OTS `Timestamp`. Build the
        // official DetachedTimestampFile wrapper around the exact requested
        // SHA-256 digest so `ots verify` can consume `TIMESTAMP_EVIDENCE.ots`.
        // Keep the response itself byte-for-byte as a separate provider
        // artifact; it remains useful for independent parser diagnostics.
        let raw_provider_response = response.body;
        let evidence_bytes = open_timestamps_detached_proof(digest, &raw_provider_response)
            .map_err(|message| ProviderFailure {
                status: ExternalTimestampStatus::AnchorMismatch,
                message,
            })?;
        Ok(ProviderTimestampResponse {
            provider: "OpenTimestamps".into(),
            evidence_extension: "ots".into(),
            evidence_bytes,
            raw_provider_response: Some(ProviderRawResponse {
                bytes: raw_provider_response,
                // The raw response is not a complete `.ots` file, so give it
                // a neutral extension rather than misleading a verifier.
                extension: "bin".into(),
            }),
            timestamp_value: String::new(),
            external_reference_id: String::new(),
            provider_verification_url: OPEN_TIMESTAMPS_POOL_ENDPOINT.into(),
            note: "OpenTimestamps detached proof and the unchanged calendar response were archived. Proof verification or upgrade may be performed later. No legal qualification is determined."
                .into(),
            metadata: TimestampProviderMetadata {
                adapter: "open_timestamps".into(),
                protocol: "OpenTimestamps / Bitcoin-backed timestamp proof".into(),
                request_algorithm: "SHA-256".into(),
                response_format: "OpenTimestamps DetachedTimestampFile (.ots); raw calendar Timestamp response archived separately".into(),
                provider_endpoint_identifier: OPEN_TIMESTAMPS_POOL_ENDPOINT.into(),
                response_structure_valid: None,
                // The detached proof wrapper is locally bound to the exact
                // digest selected from the finalized manifest. This is not a
                // provider-signature or calendar-attestation verification.
                provider_digest_match: Some(true),
                verification_result: ExternalTimestampStatus::Attached,
                verification_message: "Detached proof is locally bound to the requested SHA-256; explicit OpenTimestamps verification or upgrade is pending."
                    .into(),
                verification_timestamp: Utc::now().to_rfc3339(),
                ..Default::default()
            },
            // Initial OTS calendar proofs are deliberately not represented as
            // RFC 3161 verification. They remain ATTACHED until an explicit
            // proof verification/upgrade confirms them.
            status: ExternalTimestampStatus::Attached,
            message: "OpenTimestamps detached proof attached; later verification or upgrade is available."
                .into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn request_rfc3161(
    provider: &str,
    endpoint: &str,
    adapter: &str,
    custom: Option<&CustomRfc3161Settings>,
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    transport: &dyn TimestampHttpTransport,
) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
    let policy_oid = custom
        .map(|value| value.policy_oid.trim())
        .filter(|value| !value.is_empty());
    let request = rfc3161_request(digest, policy_oid).map_err(|message| ProviderFailure {
        status: ExternalTimestampStatus::ConfigurationIncomplete,
        message,
    })?;
    let mut headers = vec![
        ("Content-Type".into(), "application/timestamp-query".into()),
        ("Accept".into(), "application/timestamp-reply".into()),
    ];
    headers.extend(authentication_headers(custom, secret)?);
    let response = transport.post(HttpRequest {
        url: endpoint.into(),
        headers,
        body: request,
        timeout_seconds: configured_timeout_seconds(settings),
    })?;
    ensure_successful_provider_http_response(&response)?;
    if response.body.is_empty() {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::UnsupportedResponse,
            message: "Timestamp provider returned an empty response.".into(),
        });
    }
    let parsed = parse_rfc3161_response(&response.body, digest);
    let structure_and_digest_match = parsed.digest_match == Some(true);
    // The local parser validates the RFC 3161 container and message imprint,
    // but does not yet implement CMS signature/trust-chain verification. Do
    // not overstate that limited check as a cryptographically verified TSA.
    let status = if structure_and_digest_match {
        ExternalTimestampStatus::Attached
    } else {
        ExternalTimestampStatus::VerificationFailed
    };
    let message = if structure_and_digest_match {
        "RFC 3161 response structure and SHA-256 message imprint match the requested anchor; CMS signature and trust-chain verification are not asserted."
            .into()
    } else {
        format!(
            "Timestamp response was archived, but technical digest verification failed: {}",
            parsed.error.as_deref().unwrap_or(
                "the returned message imprint does not match the requested SHA-256 digest"
            )
        )
    };
    Ok(ProviderTimestampResponse {
        provider: provider.into(),
        evidence_extension: "tsr".into(),
        evidence_bytes: response.body,
        raw_provider_response: None,
        timestamp_value: parsed.timestamp_value,
        external_reference_id: parsed.serial_number,
        provider_verification_url: endpoint.into(),
        note: format!(
            "{message} No legal qualification, eIDAS qualification, or legally binding effect is determined by SunoDM."
        ),
        metadata: TimestampProviderMetadata {
            adapter: adapter.into(),
            protocol: "RFC 3161 Timestamp Protocol".into(),
            request_algorithm: "SHA-256".into(),
            response_format: "RFC 3161 TimeStampResp (.tsr)".into(),
            provider_endpoint_identifier: endpoint.into(),
            policy_oid: parsed.policy_oid,
            response_structure_valid: Some(parsed.error.is_none()),
            provider_digest_match: parsed.digest_match,
            signature_verified: None,
            trust_chain_verified: None,
            verification_result: status,
            verification_message: message.clone(),
            verification_timestamp: Utc::now().to_rfc3339(),
            ..Default::default()
        },
        status,
        message,
    })
}

fn ensure_successful_provider_http_response(
    response: &HttpResponse,
) -> std::result::Result<(), ProviderFailure> {
    if (200..300).contains(&response.status) {
        return Ok(());
    }
    if matches!(response.status, 401 | 403) {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::AuthenticationFailed,
            message: "Timestamp provider rejected the configured authentication.".into(),
        });
    }
    Err(ProviderFailure {
        status: ExternalTimestampStatus::ProviderUnavailable,
        message: format!(
            "Timestamp provider returned HTTP status {}.",
            response.status
        ),
    })
}

fn configured_timeout_seconds(settings: &TimestampSettings) -> u32 {
    if settings.provider == TimestampProviderKind::CustomRfc3161 {
        settings.custom.timeout_seconds.max(1)
    } else {
        15
    }
}

fn authentication_headers(
    custom: Option<&CustomRfc3161Settings>,
    secret: Option<&str>,
) -> std::result::Result<Vec<(String, String)>, ProviderFailure> {
    let Some(custom) = custom else {
        return Ok(Vec::new());
    };
    let secret = secret.map(str::trim).filter(|value| !value.is_empty());
    match custom.authentication_mode {
        TimestampAuthenticationMode::None => Ok(Vec::new()),
        TimestampAuthenticationMode::Basic => {
            let secret = secret.ok_or_else(|| ProviderFailure {
                status: ExternalTimestampStatus::AuthenticationRequired,
                message: "A password or token is required for the configured timestamp service."
                    .into(),
            })?;
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{secret}", custom.username.trim()));
            Ok(vec![("Authorization".into(), format!("Basic {encoded}"))])
        }
        TimestampAuthenticationMode::BearerToken => {
            let secret = secret.ok_or_else(|| ProviderFailure {
                status: ExternalTimestampStatus::AuthenticationRequired,
                message: "A password or token is required for the configured timestamp service."
                    .into(),
            })?;
            Ok(vec![("Authorization".into(), format!("Bearer {secret}"))])
        }
        TimestampAuthenticationMode::ApiKey => {
            let secret = secret.ok_or_else(|| ProviderFailure {
                status: ExternalTimestampStatus::AuthenticationRequired,
                message: "A password or token is required for the configured timestamp service."
                    .into(),
            })?;
            Ok(vec![("X-API-Key".into(), secret.into())])
        }
        TimestampAuthenticationMode::ClientCertificate => Err(ProviderFailure {
            status: ExternalTimestampStatus::VerificationConfigurationIncomplete,
            message: "Client-certificate authentication is prepared but is not enabled by this provider adapter yet."
                .into(),
        }),
    }
}

fn decode_sha256_hex(value: &str) -> std::result::Result<Vec<u8>, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Timestamp anchor digest is not a SHA-256 value.".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| "Invalid SHA-256 digest.".into())
        })
        .collect()
}

fn valid_oid(value: &str) -> bool {
    let mut values = value
        .split('.')
        .map(|part| part.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok();
    let Some(values) = values.take() else {
        return false;
    };
    values.len() >= 2 && values[0] <= 2 && (values[0] < 2 || values[1] <= 39)
}

/// Encode the proof format used by the official OpenTimestamps
/// `DetachedTimestampFile.serialize` implementation for a SHA-256 file hash.
/// The timestamp bytes are deliberately appended without parsing or altering
/// them, so the raw calendar response survives unchanged inside the proof.
fn open_timestamps_detached_proof(
    digest: &str,
    serialized_timestamp: &[u8],
) -> std::result::Result<Vec<u8>, String> {
    let digest = decode_sha256_hex(digest)?;
    if digest.len() != 32 {
        return Err("OpenTimestamps detached proofs require a SHA-256 digest.".into());
    }
    let mut proof = Vec::with_capacity(
        OPEN_TIMESTAMPS_DETACHED_MAGIC.len() + 2 + digest.len() + serialized_timestamp.len(),
    );
    proof.extend_from_slice(OPEN_TIMESTAMPS_DETACHED_MAGIC);
    proof.push(OPEN_TIMESTAMPS_DETACHED_VERSION);
    proof.push(OPEN_TIMESTAMPS_SHA256_FILE_HASH_OP);
    proof.extend_from_slice(&digest);
    proof.extend_from_slice(serialized_timestamp);
    Ok(proof)
}

fn rfc3161_request(digest: &str, policy_oid: Option<&str>) -> std::result::Result<Vec<u8>, String> {
    // RFC 3161 TimeStampReq (v1), carrying only the SHA-256 message imprint.
    // No user, track, title, media, or project payload is placed in the
    // request.
    let digest = decode_sha256_hex(digest)?;
    let sha256_algorithm = der_sequence(&[der_oid("2.16.840.1.101.3.4.2.1")?, der_tlv(0x05, &[])]);
    let message_imprint = der_sequence(&[sha256_algorithm, der_tlv(0x04, &digest)]);
    let mut elements = vec![der_integer_unsigned(&[1]), message_imprint];
    if let Some(policy_oid) = policy_oid {
        elements.push(der_oid(policy_oid)?);
    }
    // A nonce makes distinct requests distinguishable. It is generated as a
    // positive ASN.1 INTEGER; no nonce is stored in evidence because it is not
    // needed to verify the selected local anchor.
    let nonce = Uuid::new_v4();
    elements.push(der_integer_unsigned(nonce.as_bytes()));
    elements.push(der_tlv(0x01, &[0xff])); // certReq = TRUE
    Ok(der_sequence(&elements))
}

fn der_sequence(elements: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for element in elements {
        content.extend_from_slice(element);
    }
    der_tlv(0x30, &content)
}

fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
    result.extend_from_slice(&der_length(content.len()));
    result.extend_from_slice(content);
    result
}

fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let mut bytes = length.to_be_bytes().to_vec();
    while bytes.first() == Some(&0) {
        bytes.remove(0);
    }
    let mut result = vec![0x80 | bytes.len() as u8];
    result.extend_from_slice(&bytes);
    result
}

fn der_integer_unsigned(value: &[u8]) -> Vec<u8> {
    let mut content = value.to_vec();
    while content.len() > 1 && content.first() == Some(&0) {
        content.remove(0);
    }
    if content.first().is_some_and(|byte| byte & 0x80 != 0) {
        content.insert(0, 0);
    }
    der_tlv(0x02, &content)
}

fn der_oid(value: &str) -> std::result::Result<Vec<u8>, String> {
    let arcs = value
        .split('.')
        .map(|part| {
            part.parse::<u64>()
                .map_err(|_| "Invalid object identifier.".into())
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    if arcs.len() < 2 || arcs[0] > 2 || (arcs[0] < 2 && arcs[1] > 39) {
        return Err("Invalid object identifier.".into());
    }
    let mut encoded = encode_base128(arcs[0] * 40 + arcs[1]);
    for arc in arcs.iter().skip(2) {
        encoded.extend_from_slice(&encode_base128(*arc));
    }
    Ok(der_tlv(0x06, &encoded))
}

fn encode_base128(mut value: u64) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push(0x80 | (value & 0x7f) as u8);
        value >>= 7;
    }
    bytes.reverse();
    bytes
}

#[derive(Clone, Copy)]
struct DerElement<'a> {
    tag: u8,
    content: &'a [u8],
}

fn der_element(bytes: &[u8]) -> std::result::Result<(DerElement<'_>, &[u8]), String> {
    if bytes.len() < 2 {
        return Err("DER element is truncated.".into());
    }
    let tag = bytes[0];
    let first_length = bytes[1];
    let (length, header_length) = if first_length & 0x80 == 0 {
        (first_length as usize, 2)
    } else {
        let count = (first_length & 0x7f) as usize;
        if count == 0 || count > std::mem::size_of::<usize>() || bytes.len() < 2 + count {
            return Err("DER length is invalid.".into());
        }
        let mut length = 0_usize;
        for byte in &bytes[2..2 + count] {
            length = length
                .checked_mul(256)
                .and_then(|value| value.checked_add(*byte as usize))
                .ok_or_else(|| "DER length overflows.".to_owned())?;
        }
        if length < 128 {
            return Err("DER length is not canonical.".into());
        }
        (length, 2 + count)
    };
    let end = header_length
        .checked_add(length)
        .ok_or_else(|| "DER element length overflows.".to_owned())?;
    if end > bytes.len() {
        return Err("DER element is truncated.".into());
    }
    Ok((
        DerElement {
            tag,
            content: &bytes[header_length..end],
        },
        &bytes[end..],
    ))
}

fn der_elements(mut bytes: &[u8]) -> std::result::Result<Vec<DerElement<'_>>, String> {
    let mut output = Vec::new();
    while !bytes.is_empty() {
        let (element, remaining) = der_element(bytes)?;
        output.push(element);
        bytes = remaining;
    }
    Ok(output)
}

fn der_sequence_content(bytes: &[u8]) -> std::result::Result<&[u8], String> {
    let (element, remaining) = der_element(bytes)?;
    if element.tag != 0x30 || !remaining.is_empty() {
        return Err("Expected one DER SEQUENCE.".into());
    }
    Ok(element.content)
}

fn der_oid_text(element: DerElement<'_>) -> std::result::Result<String, String> {
    if element.tag != 0x06 || element.content.is_empty() {
        return Err("Expected an object identifier.".into());
    }
    let mut bytes = element.content.iter().copied();
    let first = decode_base128(&mut bytes)?;
    let (first_arc, second_arc) = if first < 40 {
        (0, first)
    } else if first < 80 {
        (1, first - 40)
    } else {
        (2, first - 80)
    };
    let mut arcs = vec![first_arc.to_string(), second_arc.to_string()];
    while bytes.clone().next().is_some() {
        arcs.push(decode_base128(&mut bytes)?.to_string());
    }
    Ok(arcs.join("."))
}

fn decode_base128(
    iterator: &mut std::iter::Copied<std::slice::Iter<'_, u8>>,
) -> std::result::Result<u64, String> {
    let mut value = 0_u64;
    let mut count = 0_u8;
    loop {
        let byte = iterator
            .next()
            .ok_or_else(|| "Object identifier is truncated.".to_owned())?;
        value = value
            .checked_mul(128)
            .and_then(|current| current.checked_add((byte & 0x7f) as u64))
            .ok_or_else(|| "Object identifier is too large.".to_owned())?;
        count = count.saturating_add(1);
        if byte & 0x80 == 0 {
            break;
        }
        if count > 10 {
            return Err("Object identifier is too large.".into());
        }
    }
    Ok(value)
}

fn der_integer_hex(element: DerElement<'_>) -> std::result::Result<String, String> {
    if element.tag != 0x02 || element.content.is_empty() || element.content[0] & 0x80 != 0 {
        return Err("Expected a non-negative INTEGER.".into());
    }
    Ok(element
        .content
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn der_integer_status(element: DerElement<'_>) -> std::result::Result<u64, String> {
    let raw = der_integer_hex(element)?;
    let normalized = raw.trim_start_matches('0');
    if normalized.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(normalized, 16)
        .map_err(|_| "Timestamp response status is too large.".into())
}

#[derive(Default)]
struct ParsedRfc3161Response {
    timestamp_value: String,
    serial_number: String,
    policy_oid: String,
    digest_match: Option<bool>,
    error: Option<String>,
}

fn parse_rfc3161_response(bytes: &[u8], expected_digest: &str) -> ParsedRfc3161Response {
    match parse_rfc3161_response_inner(bytes, expected_digest) {
        Ok(value) => value,
        Err(error) => ParsedRfc3161Response {
            error: Some(error),
            ..Default::default()
        },
    }
}

fn parse_rfc3161_response_inner(
    bytes: &[u8],
    expected_digest: &str,
) -> std::result::Result<ParsedRfc3161Response, String> {
    let response = der_elements(der_sequence_content(bytes)?)?;
    let status_info = response
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 response has no status information.".to_owned())?;
    if status_info.tag != 0x30 {
        return Err("RFC 3161 status information is invalid.".into());
    }
    let status = der_elements(status_info.content)?
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 response status is missing.".to_owned())?;
    let status = der_integer_status(status)?;
    if !matches!(status, 0 | 1) {
        return Err(format!(
            "Timestamp authority rejected the request (RFC 3161 status {status})."
        ));
    }
    let token = response
        .get(1)
        .copied()
        .ok_or_else(|| "RFC 3161 response has no timestamp token.".to_owned())?;
    let token_contents = der_elements(token.content)?;
    if token.tag != 0x30 || token_contents.len() < 2 {
        return Err("RFC 3161 timestamp token is invalid.".into());
    }
    if der_oid_text(token_contents[0])? != "1.2.840.113549.1.7.2" {
        return Err("RFC 3161 timestamp token is not CMS SignedData.".into());
    }
    let signed_wrapper = token_contents[1];
    if signed_wrapper.tag != 0xa0 {
        return Err("RFC 3161 CMS SignedData wrapper is missing.".into());
    }
    let (signed_data, remaining) = der_element(signed_wrapper.content)?;
    if signed_data.tag != 0x30 || !remaining.is_empty() {
        return Err("RFC 3161 CMS SignedData is invalid.".into());
    }
    let signed_values = der_elements(signed_data.content)?;
    let encapsulated = signed_values
        .get(2)
        .copied()
        .ok_or_else(|| "RFC 3161 CMS encapsulated content is missing.".to_owned())?;
    if encapsulated.tag != 0x30 {
        return Err("RFC 3161 CMS encapsulated content is invalid.".into());
    }
    let encapsulated_values = der_elements(encapsulated.content)?;
    if encapsulated_values.len() < 2
        || der_oid_text(encapsulated_values[0])? != "1.2.840.113549.1.9.16.1.4"
        || encapsulated_values[1].tag != 0xa0
    {
        return Err("RFC 3161 CMS payload is not TSTInfo.".into());
    }
    let (tst_octet, remaining) = der_element(encapsulated_values[1].content)?;
    if tst_octet.tag != 0x04 || !remaining.is_empty() {
        return Err("RFC 3161 TSTInfo payload is invalid.".into());
    }
    parse_tst_info(tst_octet.content, expected_digest)
}

fn parse_tst_info(
    bytes: &[u8],
    expected_digest: &str,
) -> std::result::Result<ParsedRfc3161Response, String> {
    let values = der_elements(der_sequence_content(bytes)?)?;
    if values.len() < 5 || values[0].tag != 0x02 || values[1].tag != 0x06 || values[2].tag != 0x30 {
        return Err("RFC 3161 TSTInfo structure is invalid.".into());
    }
    let policy_oid = der_oid_text(values[1])?;
    let imprint_values = der_elements(values[2].content)?;
    if imprint_values.len() != 2 || imprint_values[0].tag != 0x30 || imprint_values[1].tag != 0x04 {
        return Err("RFC 3161 message imprint is invalid.".into());
    }
    let algorithm = der_elements(imprint_values[0].content)?
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 message imprint algorithm is missing.".to_owned())?;
    if der_oid_text(algorithm)? != "2.16.840.1.101.3.4.2.1" {
        return Err("RFC 3161 response does not use SHA-256 message imprint.".into());
    }
    let returned_digest = imprint_values[1]
        .content
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let serial_number = der_integer_hex(values[3])?;
    if values[4].tag != 0x18 {
        return Err("RFC 3161 generation time is missing.".into());
    }
    let timestamp_value = generalized_time_to_rfc3339(values[4].content)?;
    Ok(ParsedRfc3161Response {
        timestamp_value,
        serial_number,
        policy_oid,
        digest_match: Some(returned_digest.eq_ignore_ascii_case(expected_digest)),
        error: None,
    })
}

fn generalized_time_to_rfc3339(bytes: &[u8]) -> std::result::Result<String, String> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| "RFC 3161 generation time is not UTF-8.".to_owned())?;
    if let Ok(value) = chrono::DateTime::parse_from_str(value, "%Y%m%d%H%M%SZ") {
        return Ok(value.to_rfc3339());
    }
    // GeneralizedTime may include fractional seconds. Preserve a valid UTC
    // timestamp in RFC 3339 form without accepting a local/ambiguous zone.
    if let Some(value) = value.strip_suffix('Z') {
        let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
        let datetime = chrono::NaiveDateTime::parse_from_str(whole, "%Y%m%d%H%M%S")
            .map_err(|_| "RFC 3161 generation time is invalid.".to_owned())?;
        let fraction = fraction.trim_end_matches('0');
        let base = datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        return Ok(if fraction.is_empty() {
            format!("{base}Z")
        } else {
            format!("{base}.{fraction}Z")
        });
    }
    Err("RFC 3161 generation time must use UTC (Z).".into())
}

#[derive(Debug)]
pub struct StagedExternalTimestamp {
    pub record: ExternalTimestampRecord,
    stage_relative: PathBuf,
    live_relative: PathBuf,
}

/// Build and durably stage a complete timestamp sidecar. The caller must
/// register `record` in SQLite before calling [`publish`], so a process exit can
/// never leave a new live sidecar that is invisible to the database.
pub fn stage(
    track_root: &Path,
    certificate_id: &str,
    source: &Path,
    input: ExternalTimestampInput,
) -> Result<StagedExternalTimestamp> {
    stage_with_provider_metadata(track_root, certificate_id, source, input, None, None)
}

/// Stage an immutable provider response that SunoDM itself requested. All
/// values are derived from the configured adapter and its raw response; the
/// caller supplies only the already-selected finalized anchor identity.
pub fn stage_provider_response(
    track_root: &Path,
    certificate_id: &str,
    referenced_revision_id: &str,
    referenced_sha256: &str,
    source: &Path,
    response: ProviderTimestampResponse,
) -> Result<StagedExternalTimestamp> {
    let mut metadata = response.metadata;
    metadata.referenced_revision_id = referenced_revision_id.to_owned();
    let raw_provider_response = response.raw_provider_response;
    let input = ExternalTimestampInput {
        provider: response.provider,
        timestamp_type: TimestampType::ExternalIntegrityTimestamp,
        timestamp_value: response.timestamp_value,
        referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
        other_referenced_artifact: String::new(),
        referenced_sha256: referenced_sha256.to_owned(),
        external_reference_id: response.external_reference_id,
        provider_verification_url: response.provider_verification_url,
        note: response.note,
    };
    stage_with_provider_metadata(
        track_root,
        certificate_id,
        source,
        input,
        Some(metadata),
        raw_provider_response.as_ref(),
    )
}

fn stage_with_provider_metadata(
    track_root: &Path,
    certificate_id: &str,
    source: &Path,
    input: ExternalTimestampInput,
    provider_metadata: Option<TimestampProviderMetadata>,
    raw_provider_response: Option<&ProviderRawResponse>,
) -> Result<StagedExternalTimestamp> {
    validate_input(&input)?;
    if raw_provider_response.is_some() && provider_metadata.is_none() {
        return Err(AppError::Validation(
            "A raw provider response archive requires provider-derived metadata.".into(),
        ));
    }
    evidence::validate_type(&crate::model::EvidenceRole::ExternalTimestamp, source)?;
    let evidence_file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Timestamp evidence file name is invalid.".into()))?
        .to_owned();
    if evidence_file_name
        .chars()
        .any(|value| value.is_control() || value == '/' || value == '\\')
    {
        return Err(AppError::Validation(
            "Timestamp evidence file name contains unsafe characters.".into(),
        ));
    }

    let referenced_relative = referenced_artifact_path(&input)?;
    let referenced_path = contained_path(track_root, &referenced_relative, true)?;
    let metadata = fs::symlink_metadata(&referenced_path)
        .map_err(|error| AppError::io(&referenced_path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Validation(
            "The referenced timestamp artifact must be a regular managed file.".into(),
        ));
    }
    let actual_sha256 = sha256_file(&referenced_path)?;
    if input.referenced_artifact == TimestampReferencedArtifact::Other
        && integrity::listed_hash(track_root, &referenced_relative)?.as_deref()
            != Some(actual_sha256.as_str())
    {
        return Err(AppError::Validation(
            "Other timestamp artifacts must be an unchanged entry in the verified phase-one SHA256SUMS.txt file."
                .into(),
        ));
    }
    let referenced_sha256 = input.referenced_sha256.trim().to_ascii_lowercase();
    let referenced_hash_match = Some(actual_sha256 == referenced_sha256);

    let id = Uuid::new_v4().to_string();
    let live_relative = PathBuf::from(EXTERNAL_TIMESTAMPS_DIR).join(&id);
    let live_directory = contained_path(track_root, &live_relative, false)?;
    if live_directory.exists() {
        return Err(AppError::Collision(live_directory.display().to_string()));
    }
    let staging_parent = ensure_contained_directory(track_root, Path::new(STAGING_DIR))?;
    sync_directory(&staging_parent)?;
    sync_directory(
        staging_parent
            .parent()
            .ok_or_else(|| AppError::PathEscape)?,
    )?;
    let live_parent = ensure_contained_directory(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR))?;
    sync_directory(&live_parent)?;
    sync_directory(live_parent.parent().ok_or_else(|| AppError::PathEscape)?)?;
    let stage_relative = PathBuf::from(STAGING_DIR).join(&id);
    let stage_directory = ensure_contained_directory(track_root, &stage_relative)?;

    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let managed_evidence_name = format!("TIMESTAMP_EVIDENCE.{extension}");
    let live_record_relative = live_relative.join(RECORD_FILE);
    let live_markdown_relative = live_relative.join(MARKDOWN_FILE);
    let live_pdf_relative = live_relative.join(PDF_FILE);
    let live_hash_list_relative = live_relative.join(HASH_LIST_FILE);
    let imported_at = Utc::now().to_rfc3339();
    let raw_provider_response_name = raw_provider_response
        .map(provider_response_artifact_name)
        .transpose()?;
    let mut provider_metadata = provider_metadata;

    let staging = (|| -> Result<StagedExternalTimestamp> {
        let evidence_path = stage_directory.join(&managed_evidence_name);
        let (evidence_sha256, _) = copy_new_hashed(source, &evidence_path)?;
        let provider_response_sha256 = if let Some(raw_provider_response) = raw_provider_response {
            let raw_name = raw_provider_response_name.as_deref().ok_or_else(|| {
                AppError::Data("Raw provider response archive name is missing.".into())
            })?;
            let raw_path = stage_directory.join(raw_name);
            atomic_write_new(&raw_path, &raw_provider_response.bytes)?;
            sha256_file(&raw_path)?
        } else {
            evidence_sha256.clone()
        };
        if let Some(metadata) = provider_metadata.as_mut() {
            metadata.provider_response_file_name = raw_provider_response_name
                .clone()
                .unwrap_or_else(|| managed_evidence_name.clone());
            metadata.provider_response_sha256 = provider_response_sha256;
        }
        let automatic_record = provider_metadata.is_some();
        let mut record = ExternalTimestampRecord {
            id: id.clone(),
            certificate_id: certificate_id.to_owned(),
            sidecar_format_version: SIDECAR_FORMAT_VERSION,
            provider: input.provider.trim().to_owned(),
            timestamp_type: input.timestamp_type,
            timestamp_value: input.timestamp_value.trim().to_owned(),
            referenced_artifact: input.referenced_artifact,
            referenced_artifact_path: portable_relative(&referenced_relative),
            referenced_sha256,
            actual_sha256,
            referenced_hash_match,
            external_reference_id: input.external_reference_id.trim().to_owned(),
            provider_verification_url: input.provider_verification_url.trim().to_owned(),
            note: input.note.trim().to_owned(),
            evidence_file_name,
            evidence_sha256,
            markdown_sha256: String::new(),
            pdf_sha256: String::new(),
            imported_at,
            provenance: if automatic_record {
                "Provider-derived metadata; managed provider response; system-verified finalized anchor comparison"
                    .into()
            } else {
                "Managed copy; user-confirmed metadata; system-verified SHA-256 comparison".into()
            },
            provider_metadata,
            record_relative_path: portable_relative(&live_record_relative),
            markdown_relative_path: portable_relative(&live_markdown_relative),
            pdf_relative_path: portable_relative(&live_pdf_relative),
            hash_list_relative_path: portable_relative(&live_hash_list_relative),
            integrity_verified_at_publication: true,
            integrity_verified: true,
            integrity_issues: Vec::new(),
        };

        let markdown = render_markdown(&record);
        atomic_write_new(&stage_directory.join(MARKDOWN_FILE), markdown.as_bytes())?;
        let pdf = render_pdf(&record)?;
        atomic_write_new(&stage_directory.join(PDF_FILE), &pdf)?;
        record.markdown_sha256 = sha256_file(&stage_directory.join(MARKDOWN_FILE))?;
        record.pdf_sha256 = sha256_file(&stage_directory.join(PDF_FILE))?;
        let record_bytes = immutable_record_bytes(&record)?;
        atomic_write_new(&stage_directory.join(RECORD_FILE), &record_bytes)?;

        let hashes = artifact_hashes_with_provider_response(
            &stage_directory,
            &managed_evidence_name,
            raw_provider_response_name.as_deref(),
        )?;
        let hash_list = render_hash_list(record.sidecar_format_version, &hashes)?;
        atomic_write_new(&stage_directory.join(HASH_LIST_FILE), hash_list.as_bytes())?;
        verify_staged_hashes(&stage_directory, &hashes)?;
        verify_record_in_directory(track_root, &stage_directory, &record, None)?;
        // The database row is written only after `stage` returns. Sync both the
        // completed directory contents and its parent entry so that a crash can
        // leave either a recoverable complete stage or no registered record.
        sync_directory(&stage_directory)?;
        if let Some(parent) = stage_directory.parent() {
            sync_directory(parent)?;
        }

        Ok(StagedExternalTimestamp {
            record,
            stage_relative,
            live_relative,
        })
    })();

    if staging.is_err() && stage_directory.exists() {
        let _ = fs::remove_dir_all(&stage_directory);
    }
    staging
}

/// Publish a staged record after its database row exists. Both directories are
/// synced around the rename so startup recovery sees either the complete stage
/// or the complete live sidecar.
pub fn publish(
    track_root: &Path,
    staged: &StagedExternalTimestamp,
) -> Result<ExternalTimestampRecord> {
    let stage_directory = contained_path(track_root, &staged.stage_relative, true)?;
    let live_directory = contained_path(track_root, &staged.live_relative, false)?;
    if live_directory.exists() {
        return Err(AppError::Collision(live_directory.display().to_string()));
    }
    verify_record_in_directory(track_root, &stage_directory, &staged.record, None)?;
    fs::rename(&stage_directory, &live_directory)
        .map_err(|error| AppError::io(&live_directory, error))?;
    sync_directory(
        live_directory
            .parent()
            .ok_or_else(|| AppError::PathEscape)?,
    )?;
    if let Some(stage_parent) = stage_directory.parent() {
        sync_directory(stage_parent)?;
    }
    verify_published_record(track_root, &staged.record)?;
    Ok(staged.record.clone())
}

pub fn discard_staged(track_root: &Path, staged: &StagedExternalTimestamp) -> Result<()> {
    let directory = contained_path(track_root, &staged.stage_relative, false)?;
    if directory.exists() {
        fs::remove_dir_all(&directory).map_err(|error| AppError::io(&directory, error))?;
        if let Some(parent) = directory.parent() {
            sync_directory(parent)?;
        }
    }
    Ok(())
}

pub fn remove_published_record(track_root: &Path, record: &ExternalTimestampRecord) -> Result<()> {
    let record_path = Path::new(&record.record_relative_path);
    let Some(relative_directory) = record_path.parent() else {
        return Err(AppError::PathEscape);
    };
    if relative_directory.parent() != Some(Path::new(EXTERNAL_TIMESTAMPS_DIR)) {
        return Err(AppError::PathEscape);
    }
    let directory = contained_path(track_root, relative_directory, false)?;
    if directory.exists() {
        let parent = directory.parent().ok_or_else(|| AppError::PathEscape)?;
        fs::remove_dir_all(&directory).map_err(|error| AppError::io(&directory, error))?;
        // Keep the database registration until the directory removal is
        // durably visible. If this sync fails, the caller deliberately retains
        // the row so startup recovery can never encounter an unregistered live
        // sidecar after a power loss.
        sync_directory(parent)?;
    }
    Ok(())
}

/// Re-verify every published sidecar artifact against the certificate-bound
/// database record. This deliberately does not fold the addendum into the
/// phase-one integrity set, so a damaged addendum is reported independently
/// without changing the finalized certificate bytes or their validity.
pub fn verify_published_record(track_root: &Path, record: &ExternalTimestampRecord) -> Result<()> {
    let location = resolve_published_record(track_root, record)?;
    verify_record_in_directory(
        track_root,
        &location.directory,
        record,
        location.revision_root.as_deref(),
    )
}

fn verify_record_in_directory(
    track_root: &Path,
    directory: &Path,
    record: &ExternalTimestampRecord,
    revision_root: Option<&Path>,
) -> Result<()> {
    record_directory(record)?;
    let extension = Path::new(&record.evidence_file_name)
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::Validation("Timestamp evidence record has no valid extension.".into())
        })?
        .to_ascii_lowercase();
    if !crate::model::EvidenceRole::ExternalTimestamp
        .allowed_extensions()
        .contains(&extension.as_str())
    {
        return Err(AppError::Validation(
            "Timestamp evidence record has an unsupported extension.".into(),
        ));
    }
    let managed_evidence_name = format!("TIMESTAMP_EVIDENCE.{extension}");
    let provider_response_name = provider_response_name_for_record(record, &managed_evidence_name)?;
    let mut expected_names = BTreeSet::from([
        RECORD_FILE.to_owned(),
        managed_evidence_name.clone(),
        MARKDOWN_FILE.to_owned(),
        PDF_FILE.to_owned(),
        HASH_LIST_FILE.to_owned(),
    ]);
    if let Some(provider_response_name) = &provider_response_name {
        if provider_response_name != &managed_evidence_name {
            expected_names.insert(provider_response_name.clone());
        }
    }
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(&directory).map_err(|error| AppError::io(&directory, error))? {
        let entry = entry.map_err(|error| AppError::io(&directory, error))?;
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| AppError::Validation("Timestamp sidecar filename is invalid.".into()))?
            .to_owned();
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::Validation(format!(
                "Timestamp sidecar contains a non-regular file: {name}"
            )));
        }
        actual_names.insert(name);
    }
    if actual_names != expected_names {
        return Err(AppError::Validation(
            "Timestamp sidecar file set does not match its managed record.".into(),
        ));
    }

    let record_path = directory.join(RECORD_FILE);
    let record_bytes = fs::read(&record_path).map_err(|error| AppError::io(&record_path, error))?;
    let stored_record: ExternalTimestampRecord = serde_json::from_slice(&record_bytes)?;
    match stored_record.sidecar_format_version {
        0 if !stored_record.integrity_verified => {
            return Err(AppError::Validation(
                "Legacy timestamp sidecar has no publication-time integrity assertion.".into(),
            ));
        }
        0 if !immutable_records_match(&stored_record, record) => {
            return Err(AppError::Validation(
                "Legacy TIMESTAMP_RECORD.json differs from the registered timestamp record.".into(),
            ));
        }
        0 => {}
        SIDECAR_FORMAT_VERSION if !stored_record.integrity_verified_at_publication => {
            return Err(AppError::Validation(
                "Timestamp sidecar does not record successful publication-time integrity verification."
                    .into(),
            ));
        }
        SIDECAR_FORMAT_VERSION => {
            // Current sidecars have one canonical immutable JSON
            // representation. A semantic deserialize/compare would accept
            // unknown or runtime-only claims after an attacker regenerated the
            // self-contained hash list; exact bytes reject that ambiguity.
            if record_bytes != immutable_record_bytes(record)? {
                return Err(AppError::Validation(
                    "TIMESTAMP_RECORD.json is not the exact immutable registered record.".into(),
                ));
            }
        }
        version => {
            return Err(AppError::Validation(format!(
                "Unsupported external timestamp sidecar format version: {version}."
            )));
        }
    }

    // Verify the exact immutable bytes that were published. Do not re-render
    // Markdown or PDF: renderer changes must not invalidate historical records.
    let hashes = artifact_hashes_with_provider_response(
        directory,
        &managed_evidence_name,
        provider_response_name.as_deref(),
    )?;
    let evidence_sha256 = hashes
        .get(&managed_evidence_name)
        .ok_or_else(|| AppError::Data("External timestamp evidence hash is missing.".into()))?;
    if evidence_sha256 != &record.evidence_sha256 {
        return Err(AppError::Validation(
            "External timestamp evidence SHA-256 no longer matches its registered value.".into(),
        ));
    }
    if let (Some(metadata), Some(provider_response_name)) =
        (&record.provider_metadata, provider_response_name.as_deref())
    {
        let provider_response_sha256 = hashes
            .get(provider_response_name)
            .ok_or_else(|| AppError::Data("Provider response archive hash is missing.".into()))?;
        if provider_response_sha256 != &metadata.provider_response_sha256 {
            return Err(AppError::Validation(
                "Provider response archive SHA-256 no longer matches its immutable metadata."
                    .into(),
            ));
        }
    }
    let expected_hash_list = render_hash_list(stored_record.sidecar_format_version, &hashes)?;
    let hash_list_path = directory.join(HASH_LIST_FILE);
    let hash_list =
        fs::read(&hash_list_path).map_err(|error| AppError::io(&hash_list_path, error))?;
    if hash_list != expected_hash_list.as_bytes() {
        return Err(AppError::Validation(
            "Timestamp sidecar SHA-256 list is incomplete or no longer matches.".into(),
        ));
    }
    if stored_record.sidecar_format_version == SIDECAR_FORMAT_VERSION {
        let markdown_sha256 = hashes
            .get(MARKDOWN_FILE)
            .ok_or_else(|| AppError::Data("Timestamp Markdown hash is missing.".into()))?;
        let pdf_sha256 = hashes
            .get(PDF_FILE)
            .ok_or_else(|| AppError::Data("Timestamp PDF hash is missing.".into()))?;
        if &stored_record.markdown_sha256 != markdown_sha256 {
            return Err(AppError::Validation(
                "External timestamp Markdown bytes no longer match their publication hash.".into(),
            ));
        }
        if &stored_record.pdf_sha256 != pdf_sha256 {
            return Err(AppError::Validation(
                "External timestamp PDF bytes no longer match their publication hash.".into(),
            ));
        }
    }

    let referenced_relative = PathBuf::from(&record.referenced_artifact_path);
    validate_stable_artifact_relative(&referenced_relative)?;
    let actual_referenced_sha256 =
        verify_referenced_artifact(track_root, revision_root, &referenced_relative, record)?;
    if actual_referenced_sha256 != record.actual_sha256
        || record.referenced_hash_match
            != Some(actual_referenced_sha256 == record.referenced_sha256)
    {
        return Err(AppError::Validation(
            "Timestamp record no longer matches the selected finalized artifact.".into(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct PublishedRecordLocation {
    directory: PathBuf,
    revision_root: Option<PathBuf>,
}

fn resolve_published_record(
    track_root: &Path,
    record: &ExternalTimestampRecord,
) -> Result<PublishedRecordLocation> {
    let live_relative = record_directory(record)?;
    let live = contained_path(track_root, &live_relative, false)?;
    if live.is_dir() {
        return Ok(PublishedRecordLocation {
            directory: live,
            revision_root: None,
        });
    }

    let revisions_relative = Path::new(".archive/revisions");
    let revisions = contained_path(track_root, revisions_relative, false)?;
    let mut matches = Vec::new();
    if revisions.is_dir() {
        let nested_timestamp_directory = Path::new(EXTERNAL_TIMESTAMPS_DIR)
            .strip_prefix(certificate::CERTIFICATE_DIR)
            .map_err(|_| AppError::PathEscape)?;
        for entry in fs::read_dir(&revisions).map_err(|error| AppError::io(&revisions, error))? {
            let entry = entry.map_err(|error| AppError::io(&revisions, error))?;
            let entry_path = entry.path();
            let metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| AppError::io(&entry_path, error))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            let candidate = entry_path
                .join("certificate")
                .join(nested_timestamp_directory)
                .join(&record.id);
            if candidate.is_dir() {
                let revision_metadata = entry_path.join("revision.json");
                let revision_metadata_relative = revision_metadata
                    .strip_prefix(track_root)
                    .map_err(|_| AppError::PathEscape)?;
                let revision_metadata =
                    contained_path(track_root, revision_metadata_relative, true)?;
                let metadata = fs::symlink_metadata(&revision_metadata)
                    .map_err(|error| AppError::io(&revision_metadata, error))?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(AppError::Validation(format!(
                        "Revision metadata for external timestamp {} is not a regular file.",
                        record.id
                    )));
                }
                let revision: serde_json::Value = serde_json::from_slice(
                    &fs::read(&revision_metadata)
                        .map_err(|error| AppError::io(&revision_metadata, error))?,
                )?;
                let archived_certificate_id = revision
                    .get("previous_certificate")
                    .and_then(|value| value.get("certificateId"))
                    .and_then(|value| value.as_str());
                if archived_certificate_id != Some(record.certificate_id.as_str()) {
                    return Err(AppError::Validation(format!(
                        "Revision metadata certificate ID does not match external timestamp {}.",
                        record.id
                    )));
                }
                let candidate_relative = candidate
                    .strip_prefix(track_root)
                    .map_err(|_| AppError::PathEscape)?;
                let candidate = contained_path(track_root, candidate_relative, true)?;
                matches.push(PublishedRecordLocation {
                    directory: candidate,
                    revision_root: Some(entry_path),
                });
            }
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => {
            let stage = contained_path(
                track_root,
                &PathBuf::from(STAGING_DIR).join(&record.id),
                false,
            )?;
            if stage.is_dir() {
                Err(AppError::Validation(format!(
                    "External timestamp publication {} is still staged and requires recovery.",
                    record.id
                )))
            } else {
                Err(AppError::Validation(format!(
                    "External timestamp sidecar {} is missing.",
                    record.id
                )))
            }
        }
        _ => Err(AppError::Validation(format!(
            "External timestamp sidecar {} exists in multiple revision archives.",
            record.id
        ))),
    }
}

/// Reconcile the two durable states used by phase-two publication. A registered
/// database row makes its matching stage recoverable; an unregistered stage is
/// an uncommitted operation and is removed. An unexpected live sidecar is never
/// silently adopted as a user-confirmed database fact.
pub fn reconcile_publications(
    track_root: &Path,
    registered: &[ExternalTimestampRecord],
) -> Result<bool> {
    let mut recovered = false;
    let registered_ids = registered
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();

    for record in registered {
        let live = contained_path(track_root, &record_directory(record)?, false)?;
        if live.is_dir() {
            continue;
        }
        // An archived record is already durably published and must not be
        // restored into the current certificate revision.
        if resolve_published_record(track_root, record).is_ok() {
            continue;
        }
        let stage_relative = PathBuf::from(STAGING_DIR).join(&record.id);
        let stage = contained_path(track_root, &stage_relative, false)?;
        if !stage.is_dir() {
            continue;
        }
        verify_record_in_directory(track_root, &stage, record, None)?;
        ensure_contained_directory(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR))?;
        fs::rename(&stage, &live).map_err(|error| AppError::io(&live, error))?;
        if let Some(parent) = live.parent() {
            sync_directory(parent)?;
        }
        if let Some(parent) = stage.parent() {
            sync_directory(parent)?;
        }
        verify_published_record(track_root, record)?;
        recovered = true;
    }

    let live_parent = contained_path(track_root, Path::new(EXTERNAL_TIMESTAMPS_DIR), false)?;
    if live_parent.is_dir() {
        for entry in
            fs::read_dir(&live_parent).map_err(|error| AppError::io(&live_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&live_parent, error))?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
            let file_name = entry.file_name();
            let id = file_name.to_str().ok_or_else(|| {
                AppError::Data("External timestamp directory name is not UTF-8.".into())
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || Uuid::parse_str(id).is_err()
            {
                return Err(AppError::Data(format!(
                    "Unexpected entry in the external timestamp publication directory: {}.",
                    path.display()
                )));
            }
            if !registered_ids.contains(id) {
                return Err(AppError::Data(format!(
                    "Unregistered external timestamp sidecar detected: {id}. It was not adopted automatically."
                )));
            }
        }
    }

    let staging_parent = contained_path(track_root, Path::new(STAGING_DIR), false)?;
    if staging_parent.is_dir() {
        for entry in
            fs::read_dir(&staging_parent).map_err(|error| AppError::io(&staging_parent, error))?
        {
            let entry = entry.map_err(|error| AppError::io(&staging_parent, error))?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
            let file_name = entry.file_name();
            let id = file_name.to_str().ok_or_else(|| {
                AppError::Data("Timestamp staging directory name is not UTF-8.".into())
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || Uuid::parse_str(id).is_err()
            {
                return Err(AppError::Data(format!(
                    "Unexpected entry in the timestamp staging directory: {}.",
                    path.display()
                )));
            }
            if registered_ids.contains(id) {
                // A registered stage that could not be recovered above remains
                // visible through its database record and must not be discarded.
                continue;
            }
            fs::remove_dir_all(&path).map_err(|error| AppError::io(&path, error))?;
            recovered = true;
        }
        if recovered {
            sync_directory(&staging_parent)?;
        }
    }
    Ok(recovered)
}

fn immutable_record_bytes(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    let mut value = serde_json::to_value(record)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::Data("Timestamp record serialization is not an object.".into()))?;
    object.remove("integrityVerified");
    object.remove("integrityIssues");
    Ok(serde_json::to_vec_pretty(&value)?)
}

fn immutable_records_match(
    stored: &ExternalTimestampRecord,
    registered: &ExternalTimestampRecord,
) -> bool {
    let mut stored = stored.clone();
    let mut registered = registered.clone();
    stored.integrity_verified = false;
    stored.integrity_issues.clear();
    registered.integrity_verified = false;
    registered.integrity_issues.clear();
    stored == registered
}

fn provider_response_artifact_name(raw_provider_response: &ProviderRawResponse) -> Result<String> {
    let extension = raw_provider_response.extension.trim().to_ascii_lowercase();
    if extension.is_empty()
        || extension.len() > 16
        || !extension
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "Provider response archive extension is invalid.".into(),
        ));
    }
    Ok(format!("{PROVIDER_RESPONSE_FILE_PREFIX}.{extension}"))
}

/// Resolve the optional extra raw-provider artifact recorded in a modern
/// automatic sidecar. Empty metadata remains valid for older automatic
/// records created before raw-response archive fields existed.
fn provider_response_name_for_record(
    record: &ExternalTimestampRecord,
    managed_evidence_name: &str,
) -> Result<Option<String>> {
    let Some(metadata) = &record.provider_metadata else {
        return Ok(None);
    };
    let name = metadata.provider_response_file_name.trim();
    let digest = metadata.provider_response_sha256.trim();
    if name.is_empty() && digest.is_empty() {
        return Ok(None);
    }
    if name.is_empty() || digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::Validation(
            "Provider response archive metadata is incomplete or invalid.".into(),
        ));
    }
    if name == managed_evidence_name {
        return Ok(Some(name.to_owned()));
    }
    let extension = name
        .strip_prefix(&format!("{PROVIDER_RESPONSE_FILE_PREFIX}."))
        .ok_or_else(|| {
            AppError::Validation(
                "Provider response archive has an unexpected managed filename.".into(),
            )
        })?;
    if extension.is_empty()
        || extension.len() > 16
        || !extension
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "Provider response archive filename is invalid.".into(),
        ));
    }
    Ok(Some(name.to_owned()))
}

fn artifact_hashes(directory: &Path, evidence_name: &str) -> Result<BTreeMap<String, String>> {
    artifact_hashes_with_provider_response(directory, evidence_name, None)
}

fn artifact_hashes_with_provider_response(
    directory: &Path,
    evidence_name: &str,
    provider_response_name: Option<&str>,
) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for name in [RECORD_FILE, evidence_name, MARKDOWN_FILE, PDF_FILE] {
        hashes.insert(name.to_owned(), sha256_file(&directory.join(name))?);
    }
    if let Some(provider_response_name) = provider_response_name {
        if provider_response_name != evidence_name {
            hashes.insert(
                provider_response_name.to_owned(),
                sha256_file(&directory.join(provider_response_name))?,
            );
        }
    }
    Ok(hashes)
}

fn render_hash_list(version: u32, hashes: &BTreeMap<String, String>) -> Result<String> {
    let mut output = match version {
        0 => String::new(),
        SIDECAR_FORMAT_VERSION => HASH_LIST_V1_HEADER.to_owned(),
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported external timestamp sidecar format version: {other}."
            )));
        }
    };
    for (name, digest) in hashes {
        output.push_str(&format!("{digest}  {name}\n"));
    }
    Ok(output)
}

fn verify_referenced_artifact(
    track_root: &Path,
    revision_root: Option<&Path>,
    referenced_relative: &Path,
    record: &ExternalTimestampRecord,
) -> Result<String> {
    if let Some(revision_root) = revision_root {
        let archived_path = if let Ok(certificate_relative) =
            referenced_relative.strip_prefix(certificate::CERTIFICATE_DIR)
        {
            Some(revision_root.join("certificate").join(certificate_relative))
        } else if referenced_relative == Path::new(certificate::PDF_FILE)
            || referenced_relative == Path::new(integrity::HASH_FILE)
        {
            Some(revision_root.join(referenced_relative))
        } else {
            None
        };
        if let Some(path) = archived_path {
            let relative = path
                .strip_prefix(track_root)
                .map_err(|_| AppError::PathEscape)?;
            let path = contained_path(track_root, relative, false)?;
            if path.exists() {
                let metadata =
                    fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(AppError::Validation(
                        "The timestamp's archived referenced artifact is not a regular file."
                            .into(),
                    ));
                }
                return sha256_file(&path);
            }
        }
        if record.referenced_artifact == TimestampReferencedArtifact::Other
            && integrity::listed_hash(revision_root, referenced_relative)?.as_deref()
                == Some(record.actual_sha256.as_str())
        {
            // Revision archives retain the verified phase-one hash list even
            // when an arbitrary `Other` source byte is not duplicated.
            return Ok(record.actual_sha256.clone());
        }
        return Err(AppError::Validation(
            "The timestamp's referenced artifact is missing from its revision archive.".into(),
        ));
    }

    let referenced_path = contained_path(track_root, referenced_relative, true)?;
    let actual = sha256_file(&referenced_path)?;
    if record.referenced_artifact == TimestampReferencedArtifact::Other
        && integrity::listed_hash(track_root, referenced_relative)?.as_deref()
            != Some(actual.as_str())
    {
        return Err(AppError::Validation(
            "The Other timestamp artifact is not an unchanged phase-one SHA256SUMS entry.".into(),
        ));
    }
    Ok(actual)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> Result<()> {
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| AppError::io(directory, error))
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> Result<()> {
    // Opening a directory for fsync is not portable (notably on Windows).
    // The files themselves are still atomically written and synced before the
    // rename; directory durability remains a best-effort platform boundary.
    Ok(())
}

pub fn finalization_anchors(track_root: &Path) -> Result<Vec<FinalizationAnchor>> {
    let definitions = [
        (
            TimestampReferencedArtifact::EvidenceManifest,
            "Evidence manifest (recommended timestamp anchor)",
            certificate::MANIFEST_FILE,
        ),
        (
            TimestampReferencedArtifact::Sha256sums,
            "Track SHA-256 manifest",
            integrity::HASH_FILE,
        ),
        (
            TimestampReferencedArtifact::DocumentationCertificateMarkdown,
            "Documentation certificate (Markdown)",
            certificate::CERTIFICATE_FILE,
        ),
        (
            TimestampReferencedArtifact::CertificatePdf,
            "Documentation certificate (PDF)",
            certificate::PDF_FILE,
        ),
        (
            TimestampReferencedArtifact::FinalEvidencePackage,
            "Final evidence package certificate hash set",
            certificate::CERTIFICATE_HASH_FILE,
        ),
    ];
    definitions
        .into_iter()
        .map(|(artifact, label, relative)| {
            let path = contained_path(track_root, Path::new(relative), true)?;
            Ok(FinalizationAnchor {
                artifact,
                label: label.into(),
                relative_path: relative.into(),
                sha256: sha256_file(&path)?,
            })
        })
        .collect()
}

/// Resolve the one automatic timestamp anchor from the immutable phase-one
/// certificate hash set, then rehash the live manifest before any provider
/// request is made. This deliberately does not accept a UI-selected hash.
pub fn finalized_manifest_anchor(track_root: &Path) -> Result<FinalizationAnchor> {
    let certificate_hashes = contained_path(
        track_root,
        Path::new(certificate::CERTIFICATE_HASH_FILE),
        true,
    )?;
    let content = fs::read_to_string(&certificate_hashes)
        .map_err(|error| AppError::io(&certificate_hashes, error))?;
    let mut expected = None;
    for (index, line) in content.lines().enumerate() {
        let (digest, path) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!(
                "Invalid finalized certificate hash entry on line {}.",
                index + 1
            ))
        })?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::Data(format!(
                "Invalid finalized certificate digest on line {}.",
                index + 1
            )));
        }
        if path == certificate::MANIFEST_FILE {
            if expected.replace(digest.to_ascii_lowercase()).is_some() {
                return Err(AppError::Data(
                    "Finalized certificate hash set contains the evidence manifest more than once."
                        .into(),
                ));
            }
        }
    }
    let expected = expected.ok_or_else(|| {
        AppError::Data(
            "Finalized certificate hash set does not contain EVIDENCE_MANIFEST.json.".into(),
        )
    })?;
    let manifest = contained_path(track_root, Path::new(certificate::MANIFEST_FILE), true)?;
    let actual = sha256_file(&manifest)?;
    if actual != expected {
        return Err(AppError::Validation(
            "INTEGRITY CHECK FAILED: The selected timestamp anchor no longer matches the finalized snapshot."
                .into(),
        ));
    }
    Ok(FinalizationAnchor {
        artifact: TimestampReferencedArtifact::EvidenceManifest,
        label: "Evidence manifest (recommended timestamp anchor)".into(),
        relative_path: certificate::MANIFEST_FILE.into(),
        sha256: expected,
    })
}

fn referenced_artifact_path(input: &ExternalTimestampInput) -> Result<PathBuf> {
    let fixed = match input.referenced_artifact {
        TimestampReferencedArtifact::EvidenceManifest => Some(certificate::MANIFEST_FILE),
        TimestampReferencedArtifact::Sha256sums => Some(integrity::HASH_FILE),
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            Some(certificate::CERTIFICATE_FILE)
        }
        TimestampReferencedArtifact::CertificatePdf => Some(certificate::PDF_FILE),
        TimestampReferencedArtifact::FinalEvidencePackage => {
            Some(certificate::CERTIFICATE_HASH_FILE)
        }
        TimestampReferencedArtifact::Other => None,
    };
    if let Some(relative) = fixed {
        return Ok(PathBuf::from(relative));
    }
    let relative = PathBuf::from(input.other_referenced_artifact.trim());
    validate_stable_artifact_relative(&relative)?;
    Ok(relative)
}

fn validate_stable_artifact_relative(relative: &Path) -> Result<()> {
    validate_relative(&relative)?;
    let portable = portable_relative(&relative);
    if portable.contains('\\')
        || portable.chars().any(char::is_control)
        || portable == ".archive"
        || portable.starts_with(".archive/")
        || portable == EXTERNAL_TIMESTAMPS_DIR
        || portable.starts_with(&format!("{EXTERNAL_TIMESTAMPS_DIR}/"))
    {
        return Err(AppError::Validation(
            "Other timestamp artifacts must identify a stable phase-one track file.".into(),
        ));
    }
    Ok(())
}

fn record_directory(record: &ExternalTimestampRecord) -> Result<PathBuf> {
    Uuid::parse_str(&record.id)
        .map_err(|_| AppError::Validation("Timestamp record ID is invalid.".into()))?;
    let directory = PathBuf::from(EXTERNAL_TIMESTAMPS_DIR).join(&record.id);
    validate_relative(&directory)?;
    for (actual, expected) in [
        (&record.record_relative_path, directory.join(RECORD_FILE)),
        (
            &record.markdown_relative_path,
            directory.join(MARKDOWN_FILE),
        ),
        (&record.pdf_relative_path, directory.join(PDF_FILE)),
        (
            &record.hash_list_relative_path,
            directory.join(HASH_LIST_FILE),
        ),
    ] {
        if actual != &portable_relative(&expected) {
            return Err(AppError::Validation(
                "Timestamp record contains an inconsistent managed path.".into(),
            ));
        }
    }
    Ok(directory)
}

fn validate_input(input: &ExternalTimestampInput) -> Result<()> {
    for (name, value, max, required) in [
        (
            "Timestamp provider / issuer",
            input.provider.as_str(),
            1000,
            true,
        ),
        (
            "Timestamp value",
            input.timestamp_value.as_str(),
            500,
            false,
        ),
        (
            "Other referenced artifact",
            input.other_referenced_artifact.as_str(),
            4000,
            input.referenced_artifact == TimestampReferencedArtifact::Other,
        ),
        (
            "External reference ID",
            input.external_reference_id.as_str(),
            1000,
            false,
        ),
        (
            "Provider verification URL",
            input.provider_verification_url.as_str(),
            4000,
            false,
        ),
        ("Timestamp note", input.note.as_str(), 20_000, false),
    ] {
        if required && value.trim().is_empty() {
            return Err(AppError::Validation(format!("{name} is required.")));
        }
        if value.len() > max || value.chars().any(|character| character == '\0') {
            return Err(AppError::Validation(format!(
                "{name} is invalid or too long."
            )));
        }
    }
    let digest = input.referenced_sha256.trim();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Validation(
            "Referenced hash must be a SHA-256 value.".into(),
        ));
    }
    if !input.provider_verification_url.trim().is_empty() {
        let parsed = Url::parse(input.provider_verification_url.trim())
            .map_err(|_| AppError::Validation("Provider verification URL is invalid.".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(AppError::Validation(
                "Provider verification URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

fn verify_staged_hashes(directory: &Path, hashes: &BTreeMap<String, String>) -> Result<()> {
    for (name, expected) in hashes {
        let actual = sha256_file(&directory.join(name))?;
        if &actual != expected {
            return Err(AppError::Validation(format!(
                "External timestamp addendum integrity mismatch: {name}"
            )));
        }
    }
    Ok(())
}

fn render_pdf(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    certificate_pdf::generate_external_timestamp_addendum_pdf(&ExternalTimestampPdfSnapshot {
        certificate_id: &record.certificate_id,
        provider: &record.provider,
        timestamp_type: timestamp_type_label(record.timestamp_type),
        timestamp_value: &record.timestamp_value,
        referenced_artifact: referenced_artifact_label(record.referenced_artifact),
        referenced_artifact_path: &record.referenced_artifact_path,
        referenced_sha256: &record.referenced_sha256,
        actual_sha256: &record.actual_sha256,
        referenced_hash_match: record.referenced_hash_match,
        evidence_file_name: &record.evidence_file_name,
        evidence_sha256: &record.evidence_sha256,
        imported_at: &record.imported_at,
        provenance: &record.provenance,
        external_reference_id: &record.external_reference_id,
        provider_verification_url: &record.provider_verification_url,
        note: &record.note,
        provider_metadata: record.provider_metadata.as_ref(),
    })
}

fn render_markdown(record: &ExternalTimestampRecord) -> String {
    let automatic = record.provider_metadata.is_some();
    let provider_origin = if automatic {
        "Provider-derived metadata"
    } else {
        "Legacy user-recorded fact"
    };
    let provider_metadata_md = provider_metadata_markdown(record);
    format!(
        "# SunoDM External Timestamp Evidence Addendum\n\n> Post-finalization technical evidence record — no legal qualification asserted.\n\n## Certificate association\n\n- Certificate ID: `{}`\n- Timestamp record ID: `{}`\n- Imported at [System value]: {}\n\n## External Timestamp Evidence\n\n- Provider / issuer [{provider_origin}]: {}\n- Timestamp type [{provider_origin}]: {}\n- Timestamp value [{provider_origin}]: {}\n- Referenced artifact [System value]: {}\n- Referenced artifact path [System value]: `{}`\n- Referenced SHA-256 [System verification]: `{}`\n- Actual artifact SHA-256 [System verification]: `{}`\n- Referenced hash match [System verification]: **{}**\n- Timestamp evidence filename [Evidence-derived metadata]: {}\n- Timestamp evidence SHA-256 [System verification]: `{}`\n- External reference ID [{provider_origin}]: {}\n- Provider verification URL [{provider_origin}]: {}\n- Note [{provider_origin}]: {}\n- Provenance [System value]: {}\n{provider_metadata_md}\n{}\n",
        md(&record.certificate_id),
        md(&record.id),
        md(&record.imported_at),
        documented_md(&record.provider),
        timestamp_type_label(record.timestamp_type),
        documented_md(&record.timestamp_value),
        referenced_artifact_label(record.referenced_artifact),
        md(&record.referenced_artifact_path),
        record.referenced_sha256,
        record.actual_sha256,
        match record.referenced_hash_match {
            Some(true) => "YES",
            Some(false) => "NO",
            None => "NOT VERIFIED",
        },
        documented_md(&record.evidence_file_name),
        record.evidence_sha256,
        documented_md(&record.external_reference_id),
        documented_md(&record.provider_verification_url),
        documented_md(&record.note),
        documented_md(&record.provenance),
        DISCLAIMER,
    )
}

fn provider_metadata_markdown(record: &ExternalTimestampRecord) -> String {
    let Some(metadata) = &record.provider_metadata else {
        return "\n- Record source [System value]: Legacy manually recorded timestamp evidence\n- Provider response verification [System verification]: NOT RECORDED (legacy manually recorded timestamp evidence)\n".into();
    };
    format!(
        "\n### Provider response metadata\n\n- Record source [System value]: Automatically attached provider response\n- Referenced finalization snapshot ID [System value]: `{}`\n- Provider adapter [Provider-derived metadata]: {}\n- Protocol [Provider-derived metadata]: {}\n- Request algorithm [System value]: {}\n- Response format [Provider-derived metadata]: {}\n- Provider endpoint identifier [Provider-derived metadata]: {}\n- Archived raw provider response [System value]: {}\n- Archived raw provider response SHA-256 [System verification]: `{}`\n- Provider response structure valid [System verification]: {}\n- Provider digest match [System verification]: {}\n- CMS signature verified [System verification]: {}\n- Trust chain verified [System verification]: {}\n- Provider verification result [System verification]: {}\n- Provider verification message [System verification]: {}\n- Provider verification timestamp [System verification]: {}\n- Timestamp issuer [Provider-derived metadata]: {}\n- Timestamp certificate subject [Provider-derived metadata]: {}\n- Timestamp certificate serial number [Provider-derived metadata]: {}\n- Policy OID [Provider-derived metadata]: {}\n",
        documented_md(&metadata.referenced_revision_id),
        documented_md(&metadata.adapter),
        documented_md(&metadata.protocol),
        documented_md(&metadata.request_algorithm),
        documented_md(&metadata.response_format),
        documented_md(&metadata.provider_endpoint_identifier),
        documented_md(&metadata.provider_response_file_name),
        documented_md(&metadata.provider_response_sha256),
        optional_bool_label(metadata.response_structure_valid),
        optional_bool_label(metadata.provider_digest_match),
        optional_bool_label(metadata.signature_verified),
        optional_bool_label(metadata.trust_chain_verified),
        timestamp_status_label(metadata.verification_result),
        documented_md(&metadata.verification_message),
        documented_md(&metadata.verification_timestamp),
        documented_md(&metadata.issuer),
        documented_md(&metadata.certificate_subject),
        documented_md(&metadata.certificate_serial_number),
        documented_md(&metadata.policy_oid),
    )
}

fn optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT VERIFIED",
    }
}

pub fn timestamp_status_label(value: ExternalTimestampStatus) -> &'static str {
    match value {
        ExternalTimestampStatus::NotRecorded => "NOT RECORDED",
        ExternalTimestampStatus::Requesting => "REQUESTING",
        ExternalTimestampStatus::Attached => "ATTACHED",
        ExternalTimestampStatus::Verified => "VERIFIED",
        ExternalTimestampStatus::VerificationFailed => "VERIFICATION FAILED",
        ExternalTimestampStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        ExternalTimestampStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        ExternalTimestampStatus::AnchorMismatch => "ANCHOR MISMATCH",
        ExternalTimestampStatus::Disabled => "DISABLED",
        ExternalTimestampStatus::Ready => "READY",
        ExternalTimestampStatus::ConfigurationIncomplete => "CONFIGURATION INCOMPLETE",
        ExternalTimestampStatus::AuthenticationRequired => "AUTHENTICATION REQUIRED",
        ExternalTimestampStatus::ConnectionFailed => "CONNECTION FAILED",
        ExternalTimestampStatus::UnsupportedResponse => "UNSUPPORTED RESPONSE",
        ExternalTimestampStatus::VerificationConfigurationIncomplete => {
            "VERIFICATION CONFIGURATION INCOMPLETE"
        }
    }
}

fn md(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('\r', "")
        .replace('\n', "<br>")
}

fn documented_md(value: &str) -> String {
    if value.trim().is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        md(value)
    }
}

pub fn timestamp_type_label(value: TimestampType) -> &'static str {
    match value {
        TimestampType::QualifiedElectronicTimestampUserDeclared => {
            "Qualified electronic timestamp — user declared"
        }
        TimestampType::ElectronicTimestamp => "Electronic timestamp",
        TimestampType::ExternalIntegrityTimestamp => "External integrity timestamp",
        TimestampType::Other => "Other",
        TimestampType::NotDocumented => "NOT DOCUMENTED",
    }
}

pub fn referenced_artifact_label(value: TimestampReferencedArtifact) -> &'static str {
    match value {
        TimestampReferencedArtifact::EvidenceManifest => "EVIDENCE_MANIFEST.json",
        TimestampReferencedArtifact::Sha256sums => "SHA256SUMS.txt",
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            "DOCUMENTATION_CERTIFICATE.md"
        }
        TimestampReferencedArtifact::CertificatePdf => "Certificate PDF",
        TimestampReferencedArtifact::FinalEvidencePackage => "Final Evidence Package",
        TimestampReferencedArtifact::Other => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use tempfile::tempdir;

    struct MockTransport {
        calls: Cell<u32>,
        requests: RefCell<Vec<HttpRequest>>,
        response: std::result::Result<HttpResponse, ProviderFailure>,
    }

    impl MockTransport {
        fn successful(body: Vec<u8>) -> Self {
            Self {
                calls: Cell::new(0),
                requests: RefCell::new(Vec::new()),
                response: Ok(HttpResponse { status: 200, body }),
            }
        }
    }

    impl TimestampHttpTransport for MockTransport {
        fn post(&self, request: HttpRequest) -> std::result::Result<HttpResponse, ProviderFailure> {
            self.calls.set(self.calls.get() + 1);
            self.requests.borrow_mut().push(request);
            self.response.clone()
        }
    }

    fn free_tsa_settings() -> TimestampSettings {
        TimestampSettings {
            enabled: true,
            provider: TimestampProviderKind::FreeTsa,
            ..Default::default()
        }
    }

    fn rfc3161_response_for_digest(digest: &str) -> Vec<u8> {
        let digest = decode_sha256_hex(digest).expect("digest bytes");
        let algorithm = der_sequence(&[
            der_oid("2.16.840.1.101.3.4.2.1").expect("SHA-256 OID"),
            der_tlv(0x05, &[]),
        ]);
        let imprint = der_sequence(&[algorithm, der_tlv(0x04, &digest)]);
        let tst_info = der_sequence(&[
            der_integer_unsigned(&[1]),
            der_oid("1.2.3.4").expect("policy OID"),
            imprint,
            der_integer_unsigned(&[42]),
            der_tlv(0x18, b"20260818120000Z"),
        ]);
        let encapsulated = der_sequence(&[
            der_oid("1.2.840.113549.1.9.16.1.4").expect("TSTInfo OID"),
            der_tlv(0xa0, &der_tlv(0x04, &tst_info)),
        ]);
        let signed_data = der_sequence(&[
            der_integer_unsigned(&[1]),
            der_tlv(0x31, &[]),
            encapsulated,
            der_tlv(0x31, &[]),
        ]);
        let token = der_sequence(&[
            der_oid("1.2.840.113549.1.7.2").expect("SignedData OID"),
            der_tlv(0xa0, &signed_data),
        ]);
        der_sequence(&[der_sequence(&[der_integer_unsigned(&[0])]), token])
    }

    #[test]
    fn qualified_type_is_explicitly_user_declared() {
        assert!(
            timestamp_type_label(TimestampType::QualifiedElectronicTimestampUserDeclared)
                .contains("user declared")
        );
    }

    #[test]
    fn markdown_keeps_no_and_not_documented_distinct() {
        assert_eq!(documented_md(""), "NOT DOCUMENTED");
        assert_eq!(documented_md("NO"), "NO");
    }

    #[test]
    fn rfc3161_mock_response_with_wrong_digest_is_archived_as_verification_failed() {
        let requested_digest = "11".repeat(32);
        let returned_digest = "22".repeat(32);
        let mock = MockTransport::successful(rfc3161_response_for_digest(&returned_digest));

        let response =
            request_timestamp_with_transport(&free_tsa_settings(), None, &requested_digest, &mock)
                .expect("provider response is retained for diagnosis");

        assert_eq!(mock.calls.get(), 1);
        assert_eq!(response.status, ExternalTimestampStatus::VerificationFailed);
        assert_eq!(
            response.metadata.provider_digest_match,
            Some(false),
            "the returned TSTInfo digest must not be accepted"
        );
        assert_eq!(
            response.metadata.verification_result,
            ExternalTimestampStatus::VerificationFailed
        );
        assert!(mock.requests.borrow()[0]
            .body
            .windows(32)
            .any(|window| window
                == decode_sha256_hex(&requested_digest)
                    .expect("digest")
                    .as_slice()));
    }

    #[test]
    fn provider_test_reports_an_unusable_rfc3161_response() {
        let mock = MockTransport::successful(b"not an RFC 3161 response".to_vec());

        let result = test_provider_with_transport(&free_tsa_settings(), None, &mock);

        assert_eq!(mock.calls.get(), 1);
        assert_eq!(result.status, ExternalTimestampStatus::UnsupportedResponse);
        assert!(result.message.contains("could not be technically verified"));
    }

    #[test]
    fn open_timestamps_uses_native_ots_proof_and_remains_attached() {
        let digest = "ab".repeat(32);
        // The real calendar response is a serialized Timestamp, not an `.ots`
        // file. Its bytes must survive unchanged both inside the wrapper and
        // in the separate raw provider-response archive.
        let raw_calendar_response = b"OpenTimestamps fixture Timestamp\0".to_vec();
        let mock = MockTransport::successful(raw_calendar_response.clone());
        let settings = TimestampSettings {
            enabled: true,
            provider: TimestampProviderKind::OpenTimestamps,
            ..Default::default()
        };

        let response = request_timestamp_with_transport(&settings, None, &digest, &mock)
            .expect("OTS proof response");

        assert_eq!(response.evidence_extension, "ots");
        assert_eq!(
            response.evidence_bytes,
            open_timestamps_detached_proof(&digest, &raw_calendar_response)
                .expect("detached proof")
        );
        assert!(response
            .evidence_bytes
            .starts_with(OPEN_TIMESTAMPS_DETACHED_MAGIC));
        let prefix_length = OPEN_TIMESTAMPS_DETACHED_MAGIC.len();
        assert_eq!(
            response.evidence_bytes[prefix_length],
            OPEN_TIMESTAMPS_DETACHED_VERSION
        );
        assert_eq!(
            response.evidence_bytes[prefix_length + 1],
            OPEN_TIMESTAMPS_SHA256_FILE_HASH_OP
        );
        assert_eq!(
            &response.evidence_bytes[prefix_length + 2..prefix_length + 34],
            decode_sha256_hex(&digest).expect("digest").as_slice()
        );
        assert_eq!(
            &response.evidence_bytes[prefix_length + 34..],
            raw_calendar_response.as_slice()
        );
        let raw_archive = response
            .raw_provider_response
            .as_ref()
            .expect("raw provider archive");
        assert_eq!(raw_archive.extension, "bin");
        assert_eq!(raw_archive.bytes, raw_calendar_response);
        assert_eq!(response.status, ExternalTimestampStatus::Attached);
        assert!(response.metadata.protocol.contains("OpenTimestamps"));
        assert!(!response.metadata.protocol.contains("RFC 3161"));
        assert_eq!(response.metadata.provider_digest_match, Some(true));
    }

    #[test]
    fn open_timestamps_sidecar_keeps_detached_proof_and_raw_response_integrity_bound() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let manifest = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&manifest, b"{\"finalized\":true}\n").expect("manifest");
        let manifest_digest = sha256_file(&manifest).expect("manifest digest");
        fs::write(
            track_root.join(certificate::CERTIFICATE_HASH_FILE),
            format!("{manifest_digest}  {}\n", certificate::MANIFEST_FILE),
        )
        .expect("certificate hash set");

        let digest = manifest_digest.clone();
        let raw_calendar_response = b"serialized-calendar-timestamp".to_vec();
        let settings = TimestampSettings {
            enabled: true,
            provider: TimestampProviderKind::OpenTimestamps,
            ..Default::default()
        };
        let response = request_timestamp_with_transport(
            &settings,
            None,
            &digest,
            &MockTransport::successful(raw_calendar_response.clone()),
        )
        .expect("OTS response");
        let proof_bytes = response.evidence_bytes.clone();
        let proof_source = track_root.join("provider-proof.ots");
        fs::write(&proof_source, &proof_bytes).expect("proof source");

        let staged = stage_provider_response(
            track_root,
            "certificate-fixture",
            "finalization-snapshot-fixture",
            &manifest_digest,
            &proof_source,
            response,
        )
        .expect("stage automatic OTS proof");
        let metadata = staged
            .record
            .provider_metadata
            .as_ref()
            .expect("provider metadata");
        assert_eq!(
            metadata.provider_response_file_name,
            "PROVIDER_RESPONSE.bin"
        );
        let stage_directory = track_root.join(STAGING_DIR).join(&staged.record.id);
        assert_eq!(
            fs::read(stage_directory.join("TIMESTAMP_EVIDENCE.ots")).expect("detached proof"),
            proof_bytes
        );
        assert_eq!(
            fs::read(stage_directory.join("PROVIDER_RESPONSE.bin")).expect("raw response"),
            raw_calendar_response
        );
        assert_eq!(
            metadata.provider_response_sha256,
            sha256_file(&stage_directory.join("PROVIDER_RESPONSE.bin")).expect("raw hash")
        );
        let hash_list =
            fs::read_to_string(stage_directory.join(HASH_LIST_FILE)).expect("hash list");
        assert!(hash_list.contains("PROVIDER_RESPONSE.bin"));
        verify_record_in_directory(track_root, &stage_directory, &staged.record, None)
            .expect("sidecar verifies including raw response");
        discard_staged(track_root, &staged).expect("discard test stage");
    }

    #[test]
    fn tampered_finalized_manifest_prevents_provider_request() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let manifest = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&manifest, b"{\"finalized\":true}\n").expect("manifest");
        let expected = sha256_file(&manifest).expect("finalized hash");
        fs::write(
            track_root.join(certificate::CERTIFICATE_HASH_FILE),
            format!("{expected}  {}\n", certificate::MANIFEST_FILE),
        )
        .expect("certificate hash set");
        fs::write(&manifest, b"{\"tampered\":true}\n").expect("tampered manifest");
        let mock = MockTransport::successful(rfc3161_response_for_digest(&"00".repeat(32)));

        if let Ok(anchor) = finalized_manifest_anchor(track_root) {
            let _ =
                request_timestamp_with_transport(&free_tsa_settings(), None, &anchor.sha256, &mock);
        }
        assert_eq!(mock.calls.get(), 0, "no digest request may leave the app");
        assert!(finalized_manifest_anchor(track_root)
            .expect_err("tampered anchor is rejected")
            .to_string()
            .contains("INTEGRITY CHECK FAILED"));
    }

    #[test]
    fn verification_pins_published_bytes_and_never_requires_current_renderer_output() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let anchor = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&anchor, b"{\"historical\":true}\n").expect("manifest anchor");
        let source = track_root.join("timestamp.json");
        fs::write(&source, b"{\"provider\":\"fixture\"}\n").expect("timestamp source");
        let anchor_sha256 = sha256_file(&anchor).expect("anchor hash");
        let staged = stage(
            track_root,
            "CERT-RENDERER-INDEPENDENCE",
            &source,
            ExternalTimestampInput {
                provider: "Fixture Provider".into(),
                timestamp_type: TimestampType::ElectronicTimestamp,
                timestamp_value: "2026-08-17T16:00:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: anchor_sha256,
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: "renderer independence".into(),
            },
        )
        .expect("stage timestamp");
        let mut record = publish(track_root, &staged).expect("publish timestamp");
        let record_directory = track_root
            .join(&record.record_relative_path)
            .parent()
            .expect("record directory")
            .to_path_buf();

        let historical_markdown = b"# Historical addendum bytes\n\nThese bytes intentionally do not come from the current renderer.\n";
        assert_ne!(
            historical_markdown.as_slice(),
            render_markdown(&record).as_bytes()
        );
        fs::write(record_directory.join(MARKDOWN_FILE), historical_markdown)
            .expect("historical markdown bytes");
        record.markdown_sha256 =
            sha256_file(&record_directory.join(MARKDOWN_FILE)).expect("markdown hash");
        fs::write(
            record_directory.join(RECORD_FILE),
            immutable_record_bytes(&record).expect("immutable record bytes"),
        )
        .expect("updated immutable record fixture");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("artifact hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).expect("versioned hash list"),
        )
        .expect("updated hash list fixture");

        verify_published_record(track_root, &record)
            .expect("persisted historical bytes verify without re-rendering");
        let immutable: serde_json::Value = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("record bytes"),
        )
        .expect("record JSON");
        assert_eq!(
            immutable["integrityVerifiedAtPublication"].as_bool(),
            Some(true)
        );
        assert!(immutable.get("integrityVerified").is_none());
        assert!(fs::read_to_string(record_directory.join(HASH_LIST_FILE))
            .expect("hash list")
            .starts_with(HASH_LIST_V1_HEADER));

        // Even a self-consistent rewritten hash list cannot authorize extra
        // runtime/trust claims in the immutable v1 JSON record.
        let mut injected: serde_json::Value = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("immutable record bytes"),
        )
        .expect("immutable record JSON");
        let object = injected.as_object_mut().expect("record object");
        object.insert("integrityVerified".into(), serde_json::Value::Bool(true));
        object.insert(
            "providerQualificationVerifiedBySunoDM".into(),
            serde_json::Value::Bool(true),
        );
        fs::write(
            record_directory.join(RECORD_FILE),
            serde_json::to_vec_pretty(&injected).expect("injected JSON bytes"),
        )
        .expect("injected record");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("injected hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).expect("injected hash list"),
        )
        .expect("self-consistent injected hash list");
        let error = verify_published_record(track_root, &record)
            .expect_err("injected immutable claims must fail verification");
        assert!(error.to_string().contains("exact immutable"));
    }

    #[test]
    fn legacy_v0_sidecars_remain_self_consistently_verifiable_without_rendering() {
        let directory = tempdir().expect("temporary track root");
        let track_root = directory.path();
        fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR))
            .expect("certificate directory");
        let anchor = track_root.join(certificate::MANIFEST_FILE);
        fs::write(&anchor, b"legacy anchor").expect("manifest anchor");
        let source = track_root.join("legacy-timestamp.json");
        fs::write(&source, b"legacy timestamp evidence").expect("timestamp source");
        let staged = stage(
            track_root,
            "CERT-LEGACY-V0",
            &source,
            ExternalTimestampInput {
                provider: "Legacy Provider".into(),
                timestamp_type: TimestampType::ElectronicTimestamp,
                timestamp_value: String::new(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: sha256_file(&anchor).expect("anchor hash"),
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: String::new(),
            },
        )
        .expect("stage timestamp");
        let current = publish(track_root, &staged).expect("publish timestamp");
        let record_directory = track_root
            .join(&current.record_relative_path)
            .parent()
            .expect("record directory")
            .to_path_buf();

        let mut legacy_value = serde_json::to_value(&current).expect("legacy record value");
        let legacy_object = legacy_value.as_object_mut().expect("record object");
        legacy_object.remove("sidecarFormatVersion");
        legacy_object.remove("markdownSha256");
        legacy_object.remove("pdfSha256");
        legacy_object.remove("integrityVerifiedAtPublication");
        let legacy_bytes = serde_json::to_vec_pretty(&legacy_value).expect("legacy bytes");
        fs::write(record_directory.join(RECORD_FILE), legacy_bytes).expect("legacy record");
        let hashes =
            artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").expect("legacy hashes");
        fs::write(
            record_directory.join(HASH_LIST_FILE),
            render_hash_list(0, &hashes).expect("legacy hash list"),
        )
        .expect("legacy hash list fixture");
        let registered: ExternalTimestampRecord = serde_json::from_slice(
            &fs::read(record_directory.join(RECORD_FILE)).expect("legacy record bytes"),
        )
        .expect("deserialize legacy record");

        verify_published_record(track_root, &registered)
            .expect("legacy sidecar self-consistency verifies without renderer equality");
    }
}
