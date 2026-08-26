use super::*;

/// Evaluate public ACRCloud configuration without a network operation.
/// `credentials_available` must be derived from the private secret store, not
/// trusted from the public `credentials_configured` display field.
pub fn provider_configuration_status(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
) -> (AudioScreeningProviderStatus, String) {
    if !settings.enabled {
        return (
            AudioScreeningProviderStatus::Disabled,
            "External ACRCloud screening is disabled.".into(),
        );
    }
    if normalize_acrcloud_host(&settings.host).is_err()
        || settings.timeout_seconds == 0
        || settings.timeout_seconds > 120
    {
        return (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "ACRCloud host or timeout is invalid.".into(),
        );
    }
    if !credentials_available {
        return (
            AudioScreeningProviderStatus::NotConfigured,
            "ACRCloud access key and access secret are not configured.".into(),
        );
    }
    (
        AudioScreeningProviderStatus::Ready,
        "ACRCloud is configured for explicitly started audio screening.".into(),
    )
}

pub fn apply_provider_configuration_status(
    settings: &mut AudioScreeningSettings,
    credentials_available: bool,
) {
    let (status, message) = provider_configuration_status(settings, credentials_available);
    settings.credentials_configured = credentials_available;
    settings.status = status;
    settings.status_message = message;
    refresh_local_engine_status(settings);
}

/// Normalizes only a dedicated public ACRCloud project host.  Embedded paths,
/// credentials, ports, IP addresses, redirects, and arbitrary destinations are
/// rejected before an HTTP request can be constructed.
pub fn normalize_acrcloud_host(value: &str) -> std::result::Result<String, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 253 || trimmed.contains(char::is_whitespace) {
        return Err(());
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let parsed = Url::parse(&candidate).map_err(|_| ())?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !(parsed.path().is_empty() || parsed.path() == "/")
    {
        return Err(());
    }
    let host = parsed.host_str().ok_or(())?.to_ascii_lowercase();
    if !host.is_ascii()
        || host == "acrcloud.com"
        || !host.ends_with(".acrcloud.com")
        || host.split('.').any(|part| part.is_empty())
    {
        return Err(());
    }
    Ok(host)
}

pub(super) fn acrcloud_identify_url(
    settings: &AudioScreeningSettings,
) -> std::result::Result<String, ProviderFailure> {
    let host = normalize_acrcloud_host(&settings.host).map_err(|_| ProviderFailure {
        status: AudioScreeningStatus::ConfigurationInvalid,
        message: "ACRCloud host or timeout is invalid.",
    })?;
    if settings.timeout_seconds == 0 || settings.timeout_seconds > 120 {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud host or timeout is invalid.",
        });
    }
    Ok(format!("https://{host}/v1/identify"))
}

/// Tests only configuration and HTTPS reachability.  It intentionally sends no
/// audio, no multipart data, and no credentials.
pub fn test_acrcloud_provider(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
) -> AudioScreeningProviderTestResult {
    test_acrcloud_provider_with_transport(
        settings,
        credentials_available,
        &UreqAcrCloudHttpTransport,
    )
}

pub(super) fn test_acrcloud_provider_with_transport(
    settings: &AudioScreeningSettings,
    credentials_available: bool,
    transport: &dyn AcrCloudHttpTransport,
) -> AudioScreeningProviderTestResult {
    let tested_at = Utc::now().to_rfc3339();
    let (configuration_status, configuration_message) =
        provider_configuration_status(settings, credentials_available);
    if configuration_status != AudioScreeningProviderStatus::Ready {
        return AudioScreeningProviderTestResult {
            status: configuration_status,
            message: configuration_message,
            tested_at,
        };
    }
    let request = match acrcloud_identify_url(settings) {
        Ok(url) => AcrCloudRequest {
            url,
            headers: Vec::new(),
            body: Vec::new(),
            timeout_seconds: settings.timeout_seconds,
        },
        Err(failure) => {
            return AudioScreeningProviderTestResult {
                status: provider_status_from_screening(failure.status),
                message: failure.message.into(),
                tested_at,
            };
        }
    };
    match transport.get(request) {
        Ok(response) => {
            let (status, message) = provider_test_status_for_http(response.status);
            AudioScreeningProviderTestResult {
                status,
                message: message.into(),
                tested_at,
            }
        }
        Err(failure) => AudioScreeningProviderTestResult {
            status: provider_status_from_screening(failure.status),
            message: failure.message.into(),
            tested_at,
        },
    }
}

/// A connection test intentionally sends no credentials and uses GET, while
/// the provider's identify endpoint is normally POST.  Therefore 401/403 and
/// 405 still prove the configured HTTPS endpoint is reachable; redirects and
/// other unexpected responses never become a misleading ready state.
pub(super) fn provider_test_status_for_http(
    status: u16,
) -> (AudioScreeningProviderStatus, &'static str) {
    match status {
        200..=299 | 401 | 403 | 405 => (
            AudioScreeningProviderStatus::Ready,
            "ACRCloud host responded. No audio or credentials were sent by this test.",
        ),
        400 | 404 | 300..=399 => (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "The ACRCloud host did not expose the expected identification endpoint.",
        ),
        408 | 425 | 429 | 500..=599 => (
            AudioScreeningProviderStatus::ProviderUnavailable,
            "The ACRCloud host is temporarily unavailable for a connection test.",
        ),
        _ => (
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "The ACRCloud host returned an unexpected response to the connection test.",
        ),
    }
}

pub(super) fn provider_status_from_screening(
    status: AudioScreeningStatus,
) -> AudioScreeningProviderStatus {
    match status {
        AudioScreeningStatus::AuthenticationFailed => {
            AudioScreeningProviderStatus::AuthenticationFailed
        }
        AudioScreeningStatus::ProviderUnavailable => {
            AudioScreeningProviderStatus::ProviderUnavailable
        }
        AudioScreeningStatus::ConfigurationInvalid => {
            AudioScreeningProviderStatus::ConfigurationInvalid
        }
        _ => AudioScreeningProviderStatus::ProviderUnavailable,
    }
}
