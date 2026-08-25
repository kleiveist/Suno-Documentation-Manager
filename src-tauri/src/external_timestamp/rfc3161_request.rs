use super::*;

pub(super) struct Rfc3161RequestContext<'a> {
    pub(super) provider: &'a str,
    pub(super) endpoint: &'a str,
    pub(super) adapter: &'a str,
    pub(super) custom: Option<&'a CustomRfc3161Settings>,
    pub(super) settings: &'a TimestampSettings,
    pub(super) secret: Option<&'a str>,
    pub(super) digest: &'a str,
    pub(super) artifact_bytes: Option<&'a [u8]>,
    pub(super) transport: &'a dyn TimestampHttpTransport,
}

pub(super) fn request_rfc3161(
    context: Rfc3161RequestContext<'_>,
) -> std::result::Result<ProviderTimestampResponse, ProviderFailure> {
    let Rfc3161RequestContext {
        provider,
        endpoint,
        adapter,
        custom,
        settings,
        secret,
        digest,
        artifact_bytes,
        transport,
    } = context;
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
        body: request.bytes,
        timeout_seconds: configured_timeout_seconds(settings),
    })?;
    ensure_successful_provider_http_response(&response)?;
    if response.body.is_empty() {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::UnsupportedResponse,
            message: "Timestamp provider returned an empty response.".into(),
        });
    }
    let parsed = parse_rfc3161_response(&response.body, digest, &request.nonce_hex, policy_oid);
    let response_context_matches = parsed.error.is_none()
        && parsed.digest_match == Some(true)
        && parsed.nonce_match == Some(true)
        && parsed.policy_match != Some(false);
    let verification = if response_context_matches {
        verify_rfc3161_cryptography(&response.body, digest, artifact_bytes, settings)
    } else {
        let reason = parsed.error.as_deref().unwrap_or_else(|| {
            if parsed.digest_match != Some(true) {
                "the returned message imprint does not match the requested SHA-256 digest"
            } else if parsed.nonce_match != Some(true) {
                "the returned nonce does not match the request nonce"
            } else {
                "the returned policy OID does not match the requested policy"
            }
        });
        Rfc3161CryptographicVerification {
            status: ExternalTimestampStatus::VerificationFailed,
            signature_verified: None,
            trust_chain_verified: None,
            cryptographic_verifier: String::new(),
            trust_anchor_sha256: Vec::new(),
            identity: None,
            message: format!(
                "Timestamp response was archived, but RFC 3161 response verification failed: {reason}."
            ),
        }
    };
    let status = verification.status;
    let verified_identity = verification.identity.clone();
    let qualification = verified_identity
        .as_ref()
        .map(|identity| provider_identity_qualification(identity, &parsed.policy_oid));
    let message = verification.message.clone();
    Ok(ProviderTimestampResponse {
        provider: provider.into(),
        evidence_extension: "tsr".into(),
        evidence_bytes: response.body,
        raw_provider_response: None,
        timestamp_value: parsed.timestamp_value.clone(),
        external_reference_id: parsed.serial_number.clone(),
        provider_verification_url: endpoint.into(),
        note: format!(
            "{message} No legal qualification, eIDAS qualification, or legally binding effect is determined by SunoDM."
        ),
        metadata: rfc3161_metadata(
            Rfc3161MetadataContext {
                adapter,
                endpoint,
                request_nonce: request.nonce_hex,
            },
            parsed,
            policy_oid,
            verification,
            qualification,
            message.clone(),
        ),
        status,
        message,
    })
}

pub(super) struct Rfc3161MetadataContext<'a> {
    pub(super) adapter: &'a str,
    pub(super) endpoint: &'a str,
    pub(super) request_nonce: String,
}

pub(super) fn rfc3161_metadata(
    context: Rfc3161MetadataContext<'_>,
    parsed: ParsedRfc3161Response,
    policy_oid: Option<&str>,
    verification: Rfc3161CryptographicVerification,
    qualification: Option<TimestampQualificationRecord>,
    message: String,
) -> TimestampProviderMetadata {
    let Rfc3161MetadataContext {
        adapter,
        endpoint,
        request_nonce,
    } = context;
    let verified_identity = verification.identity;
    let ParsedRfc3161Response {
        nonce_hex,
        digest_match,
        nonce_match,
        policy_match,
        policy_oid: response_policy_oid,
        error,
        ..
    } = parsed;
    TimestampProviderMetadata {
        adapter: adapter.into(),
        protocol: "RFC 3161 Timestamp Protocol".into(),
        request_algorithm: "SHA-256".into(),
        response_format: "RFC 3161 TimeStampResp (.tsr)".into(),
        provider_endpoint_identifier: endpoint.into(),
        issuer: verified_identity
            .as_ref()
            .map(|value| value.certificate_issuer.clone())
            .unwrap_or_default(),
        certificate_subject: verified_identity
            .as_ref()
            .map(|value| value.certificate_subject.clone())
            .unwrap_or_default(),
        certificate_serial_number: verified_identity
            .as_ref()
            .map(|value| value.certificate_serial_number.clone())
            .unwrap_or_default(),
        certificate_sha256: verified_identity
            .as_ref()
            .map(|value| value.certificate_sha256.clone())
            .unwrap_or_default(),
        provider_identity_verified: verified_identity.as_ref().map(|_| true),
        signature_verification_applicable: Some(true),
        trust_chain_verification_applicable: Some(true),
        request_nonce,
        response_nonce: nonce_hex,
        nonce_match,
        requested_policy_oid: policy_oid.unwrap_or_default().to_owned(),
        policy_oid: response_policy_oid,
        policy_match,
        cryptographic_verifier: verification.cryptographic_verifier,
        trust_anchor_sha256: verification.trust_anchor_sha256,
        response_structure_valid: Some(error.is_none()),
        provider_digest_match: digest_match,
        signature_verified: verification.signature_verified,
        trust_chain_verified: verification.trust_chain_verified,
        verification_result: verification.status,
        verification_message: message,
        verification_timestamp: Utc::now().to_rfc3339(),
        qualification,
        ..Default::default()
    }
}

pub(super) fn ensure_successful_provider_http_response(
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

pub(super) fn configured_timeout_seconds(settings: &TimestampSettings) -> u32 {
    if settings.provider == TimestampProviderKind::CustomRfc3161 {
        settings.custom.timeout_seconds.max(1)
    } else {
        15
    }
}

pub(super) fn authentication_headers(
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

pub(super) fn decode_sha256_hex(value: &str) -> std::result::Result<Vec<u8>, String> {
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

pub(super) fn valid_oid(value: &str) -> bool {
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
pub(super) fn open_timestamps_detached_proof(
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

#[derive(Debug, Clone)]
pub(super) struct Rfc3161Request {
    pub(super) bytes: Vec<u8>,
    pub(super) nonce_hex: String,
}

pub(super) fn rfc3161_request(
    digest: &str,
    policy_oid: Option<&str>,
) -> std::result::Result<Rfc3161Request, String> {
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
    // A nonce binds the provider response to this one request. It is retained
    // in the request context until the response has been checked.
    let nonce = Uuid::new_v4();
    elements.push(der_integer_unsigned(nonce.as_bytes()));
    elements.push(der_tlv(0x01, &[0xff])); // certReq = TRUE
    Ok(Rfc3161Request {
        bytes: der_sequence(&elements),
        nonce_hex: normalized_unsigned_hex(nonce.as_bytes()),
    })
}
