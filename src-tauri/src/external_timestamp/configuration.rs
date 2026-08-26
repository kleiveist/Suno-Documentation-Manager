use super::*;

pub(super) fn provider_adapter(
    kind: TimestampProviderKind,
) -> Option<Box<dyn TimestampProviderAdapter>> {
    match kind {
        TimestampProviderKind::FreeTsa => Some(Box::new(Rfc3161TimestampAdapter {
            provider: "FreeTSA",
            endpoint: FREETSA_ENDPOINT,
            adapter: "freetsa_rfc3161",
            trust_root_available: false,
        })),
        TimestampProviderKind::SigstorePublicTsa => {
            let _ = SIGSTORE_PUBLIC_TSA_CERTCHAIN_ENDPOINT;
            Some(Box::new(Rfc3161TimestampAdapter {
                provider: "Sigstore Public TSA",
                endpoint: SIGSTORE_PUBLIC_TSA_ENDPOINT,
                adapter: "sigstore_public_tsa_rfc3161",
                // A public cert-chain endpoint is not itself a pinned trust root.
                // VERIFIED therefore still requires an explicit local CA file.
                trust_root_available: false,
            }))
        }
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
) -> (TimestampProviderConfigurationStatus, String) {
    if !settings.enabled || settings.provider == TimestampProviderKind::Disabled {
        return (
            TimestampProviderConfigurationStatus::Disabled,
            "External timestamp service is disabled.".into(),
        );
    }
    if settings.provider == TimestampProviderKind::OpenTimestamps {
        return (
            TimestampProviderConfigurationStatus::Ready,
            "OpenTimestamps calendar service is ready. This is not RFC 3161; an initial detached proof remains ATTACHED until OpenTimestamps verification or upgrade confirms its Bitcoin anchoring."
                .into(),
        );
    }
    if settings.provider != TimestampProviderKind::CustomRfc3161 {
        if settings.custom.ca_certificate_path.trim().is_empty() {
            return (
                TimestampProviderConfigurationStatus::VerificationConfigurationIncomplete,
                "An explicit TSA CA trust-anchor file is required before RFC 3161 responses can be marked VERIFIED."
                    .into(),
            );
        }
        return (
            TimestampProviderConfigurationStatus::Ready,
            "RFC 3161 timestamp service and explicit TSA trust anchor are ready.".into(),
        );
    }
    match validate_custom_settings(&settings.custom, secret_available) {
        Ok(()) if settings.custom.ca_certificate_path.trim().is_empty() => (
            TimestampProviderConfigurationStatus::VerificationConfigurationIncomplete,
            "An explicit TSA CA trust-anchor file is required before RFC 3161 responses can be marked VERIFIED."
                .into(),
        ),
        Ok(()) => (
            TimestampProviderConfigurationStatus::Ready,
            "Custom RFC 3161 timestamp service and explicit TSA trust anchor are configured."
                .into(),
        ),
        Err(failure) => (provider_configuration_status(failure.status), failure.message),
    }
}

pub(super) fn provider_configuration_status(
    status: ExternalTimestampStatus,
) -> TimestampProviderConfigurationStatus {
    match status {
        ExternalTimestampStatus::Disabled => TimestampProviderConfigurationStatus::Disabled,
        ExternalTimestampStatus::ConfigurationIncomplete => {
            TimestampProviderConfigurationStatus::NotConfigured
        }
        ExternalTimestampStatus::AuthenticationRequired => {
            TimestampProviderConfigurationStatus::AuthenticationRequired
        }
        ExternalTimestampStatus::AuthenticationFailed => {
            TimestampProviderConfigurationStatus::AuthenticationFailed
        }
        ExternalTimestampStatus::ConnectionFailed
        | ExternalTimestampStatus::ProviderUnavailable => {
            TimestampProviderConfigurationStatus::ConnectionFailed
        }
        ExternalTimestampStatus::VerificationConfigurationIncomplete => {
            TimestampProviderConfigurationStatus::VerificationConfigurationIncomplete
        }
        _ => TimestampProviderConfigurationStatus::ProviderError,
    }
}

pub(crate) fn timestamp_status_for_configuration(
    status: TimestampProviderConfigurationStatus,
) -> ExternalTimestampStatus {
    match status {
        TimestampProviderConfigurationStatus::Disabled => ExternalTimestampStatus::Disabled,
        TimestampProviderConfigurationStatus::NotConfigured => {
            ExternalTimestampStatus::ConfigurationIncomplete
        }
        TimestampProviderConfigurationStatus::Ready => ExternalTimestampStatus::Ready,
        TimestampProviderConfigurationStatus::AuthenticationRequired => {
            ExternalTimestampStatus::AuthenticationRequired
        }
        TimestampProviderConfigurationStatus::AuthenticationFailed => {
            ExternalTimestampStatus::AuthenticationFailed
        }
        TimestampProviderConfigurationStatus::ConnectionFailed => {
            ExternalTimestampStatus::ConnectionFailed
        }
        TimestampProviderConfigurationStatus::VerificationConfigurationIncomplete => {
            ExternalTimestampStatus::VerificationConfigurationIncomplete
        }
        TimestampProviderConfigurationStatus::ProviderError => {
            ExternalTimestampStatus::ProviderUnavailable
        }
    }
}

pub(super) fn validate_custom_settings(
    custom: &CustomRfc3161Settings,
    secret_available: bool,
) -> std::result::Result<(), ProviderFailure> {
    validate_custom_endpoint(custom)?;
    validate_custom_protocol_settings(custom)?;
    validate_custom_authentication(custom, secret_available)
}

pub(super) fn validate_custom_endpoint(
    custom: &CustomRfc3161Settings,
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
    Ok(())
}

pub(super) fn validate_custom_protocol_settings(
    custom: &CustomRfc3161Settings,
) -> std::result::Result<(), ProviderFailure> {
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
    Ok(())
}

pub(super) fn validate_custom_authentication(
    custom: &CustomRfc3161Settings,
    secret_available: bool,
) -> std::result::Result<(), ProviderFailure> {
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
