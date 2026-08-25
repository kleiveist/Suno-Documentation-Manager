use super::*;

pub fn test_provider(
    settings: &TimestampSettings,
    secret: Option<&str>,
) -> TimestampProviderTestResult {
    test_provider_with_transport(settings, secret, &UreqTimestampHttpTransport)
}

pub(super) fn test_provider_with_transport(
    settings: &TimestampSettings,
    secret: Option<&str>,
    transport: &dyn TimestampHttpTransport,
) -> TimestampProviderTestResult {
    let tested_at = Utc::now().to_rfc3339();
    let secret_available = secret.is_some_and(|value| !value.trim().is_empty());
    let (configuration_status, configuration_message) = settings_status(settings, secret_available);
    let capabilities = provider_capabilities(settings.provider);
    if configuration_status != TimestampProviderConfigurationStatus::Ready {
        return TimestampProviderTestResult {
            provider: settings.provider,
            status: configuration_status,
            message: configuration_message,
            tested_at,
            capabilities,
        };
    }
    match request_timestamp_with_transport(settings, secret, &"00".repeat(32), None, transport) {
        Ok(response) => {
            let (status, message) = if response.status
                == ExternalTimestampStatus::VerificationFailed
            {
                (
                    TimestampProviderConfigurationStatus::ProviderError,
                    "Provider responded, but its test response could not be technically verified."
                        .into(),
                )
            } else if settings.provider == TimestampProviderKind::OpenTimestamps {
                (
                        TimestampProviderConfigurationStatus::Ready,
                        "OpenTimestamps calendar service reachable. This is not RFC 3161; an initial proof remains ATTACHED pending OpenTimestamps verification or upgrade."
                            .into(),
                    )
            } else {
                (
                    TimestampProviderConfigurationStatus::Ready,
                    "RFC 3161 timestamp service ready.".into(),
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
            status: provider_configuration_status(failure.status),
            message: failure.message,
            tested_at,
            capabilities,
        },
    }
}

/// Request and cryptographically verify a timestamp for the exact finalized
/// artifact bytes. The provider connection test uses the internal request path
/// without artifact bytes, so it cannot be mistaken for a verified snapshot.
pub fn request_timestamp_for_artifact(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    artifact_bytes: &[u8],
) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
    request_timestamp_with_transport(
        settings,
        secret,
        digest,
        Some(artifact_bytes),
        &UreqTimestampHttpTransport,
    )
}

/// Capture all four independent finalization layers after the manifest anchor
/// exists and before the certificate PDF is rendered. Provider and trust
/// lookup failures are represented as data and never returned as a
/// finalization error.
pub fn attempt_finalization_timestamp(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    artifact_bytes: &[u8],
) -> FinalizationTimestampAttempt {
    attempt_finalization_timestamp_with_transport(
        settings,
        secret,
        digest,
        artifact_bytes,
        &UreqTimestampHttpTransport,
    )
}

pub(super) fn attempt_finalization_timestamp_with_transport(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    artifact_bytes: &[u8],
    transport: &dyn TimestampHttpTransport,
) -> FinalizationTimestampAttempt {
    let provider = provider_display_name(settings);
    let secret_available = secret.is_some_and(|value| !value.trim().is_empty());
    let (configuration_status, configuration_message) = settings_status(settings, secret_available);
    let automatic_request_enabled = settings.enabled
        && settings.auto_after_finalization
        && settings.provider != TimestampProviderKind::Disabled;

    if !automatic_request_enabled {
        return FinalizationTimestampAttempt {
            snapshot: FinalizationTimestampSnapshot {
                provider,
                provider_configuration_status: configuration_status,
                provider_configuration_message: configuration_message,
                automatic_request_enabled: false,
                technical_status: ExternalTimestampStatus::NotRecorded,
                technical_message:
                    "No automatic external timestamp was requested for this finalization.".into(),
                ..Default::default()
            },
            response: None,
        };
    }

    if configuration_status != TimestampProviderConfigurationStatus::Ready {
        return FinalizationTimestampAttempt {
            snapshot: FinalizationTimestampSnapshot {
                provider,
                provider_configuration_status: configuration_status,
                provider_configuration_message: configuration_message,
                automatic_request_enabled: true,
                technical_status: ExternalTimestampStatus::NotRecorded,
                technical_message: "No timestamp evidence was created because the provider configuration was not ready.".into(),
                ..Default::default()
            },
            response: None,
        };
    }

    match request_timestamp_with_transport(
        settings,
        secret,
        digest,
        Some(artifact_bytes),
        transport,
    ) {
        Ok(response) => {
            let snapshot = FinalizationTimestampSnapshot {
                provider: response.provider.clone(),
                provider_configuration_status: TimestampProviderConfigurationStatus::Ready,
                provider_configuration_message: configuration_message,
                automatic_request_enabled: true,
                technical_status: response.status,
                technical_message: response.message.clone(),
                timestamp_value: response.timestamp_value.clone(),
                external_reference_id: response.external_reference_id.clone(),
                provider_verification_url: response.provider_verification_url.clone(),
                provider_metadata: Some(response.metadata.clone()),
            };
            FinalizationTimestampAttempt {
                snapshot,
                response: Some(response),
            }
        }
        Err(failure) => FinalizationTimestampAttempt {
            snapshot: FinalizationTimestampSnapshot {
                provider,
                provider_configuration_status: provider_configuration_status(failure.status),
                provider_configuration_message: failure.message.clone(),
                automatic_request_enabled: true,
                technical_status: ExternalTimestampStatus::VerificationFailed,
                technical_message: failure.message,
                ..Default::default()
            },
            response: None,
        },
    }
}

pub(super) fn request_timestamp_with_transport(
    settings: &TimestampSettings,
    secret: Option<&str>,
    digest: &str,
    artifact_bytes: Option<&[u8]>,
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
    if status != TimestampProviderConfigurationStatus::Ready {
        return Err(ProviderFailure {
            status: timestamp_status_for_configuration(status),
            message,
        });
    }
    let adapter = provider_adapter(settings.provider).ok_or_else(|| ProviderFailure {
        status: ExternalTimestampStatus::Disabled,
        message: "External timestamp service is disabled.".into(),
    })?;
    adapter.request(settings, secret, digest, artifact_bytes, transport)
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
            qualification_status: "not_checked".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        artifact_bytes: Option<&[u8]>,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
        request_rfc3161(Rfc3161RequestContext {
            provider: self.provider,
            endpoint: self.endpoint,
            adapter: self.adapter,
            custom: None,
            settings,
            secret,
            digest,
            artifact_bytes,
            transport,
        })
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
            qualification_status: "not_checked".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        artifact_bytes: Option<&[u8]>,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
        let provider = self.display_name(settings);
        request_rfc3161(Rfc3161RequestContext {
            provider: &provider,
            endpoint: settings.custom.endpoint.trim(),
            adapter: "custom_rfc3161",
            custom: Some(&settings.custom),
            settings,
            secret,
            digest,
            artifact_bytes,
            transport,
        })
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
            qualification_status: "not_checked".into(),
        }
    }

    fn request(
        &self,
        settings: &TimestampSettings,
        _secret: Option<&str>,
        digest: &str,
        _artifact_bytes: Option<&[u8]>,
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
                protocol: "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade".into(),
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
