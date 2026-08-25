use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct VerifiedTimestampIdentity {
    pub(super) certificate_sha256: String,
    pub(super) certificate_subject: String,
    pub(super) certificate_issuer: String,
    pub(super) certificate_serial_number: String,
}

pub(super) struct Rfc3161CryptographicVerification {
    pub(super) status: ExternalTimestampStatus,
    pub(super) signature_verified: Option<bool>,
    pub(super) trust_chain_verified: Option<bool>,
    pub(super) cryptographic_verifier: String,
    pub(super) trust_anchor_sha256: Vec<String>,
    pub(super) identity: Option<VerifiedTimestampIdentity>,
    pub(super) message: String,
}

pub(super) fn provider_identity_qualification(
    identity: &VerifiedTimestampIdentity,
    policy_oid: &str,
) -> TimestampQualificationRecord {
    TimestampQualificationRecord {
        status: TimestampQualificationStatus::ProviderIdentityVerified,
        provider_identity_status: TimestampQualificationStatus::ProviderIdentityVerified,
        trust_service_status: TimestampQualificationStatus::NotChecked,
        eidas_qualification_status: TimestampQualificationStatus::NotChecked,
        current_qualification_status: TimestampQualificationStatus::NotChecked,
        qualification_at_timestamp: TimestampQualificationStatus::NotChecked,
        message: "The timestamp signer identity was cryptographically verified. No validated official Trusted List source was available for a regulatory qualification lookup.".into(),
        identity: TimestampServiceIdentity {
            certificate_sha256: identity.certificate_sha256.clone(),
            certificate_subject: identity.certificate_subject.clone(),
            certificate_issuer: identity.certificate_issuer.clone(),
            certificate_serial_number: identity.certificate_serial_number.clone(),
            policy_oid: policy_oid.into(),
            service_identifier: String::new(),
        },
        ..Default::default()
    }
}

pub(super) fn verify_rfc3161_cryptography(
    response: &[u8],
    expected_digest: &str,
    artifact_bytes: Option<&[u8]>,
    settings: &TimestampSettings,
) -> Rfc3161CryptographicVerification {
    let Some(artifact_bytes) = artifact_bytes else {
        return Rfc3161CryptographicVerification {
            status: ExternalTimestampStatus::Attached,
            signature_verified: None,
            trust_chain_verified: None,
            cryptographic_verifier: String::new(),
            trust_anchor_sha256: Vec::new(),
            identity: None,
            message: "RFC 3161 response structure, SHA-256 message imprint, nonce, and the provider-returned policy OID (including a requested-policy match when configured) were checked. This endpoint connection test did not supply finalized artifact bytes, so CMS and trust-chain verification were not asserted."
                .into(),
        };
    };
    let actual_digest = sha256_bytes(artifact_bytes);
    if !actual_digest.eq_ignore_ascii_case(expected_digest) {
        return Rfc3161CryptographicVerification {
            status: ExternalTimestampStatus::AnchorMismatch,
            signature_verified: None,
            trust_chain_verified: None,
            cryptographic_verifier: String::new(),
            trust_anchor_sha256: Vec::new(),
            identity: None,
            message: "The finalized artifact bytes supplied to the RFC 3161 verifier no longer match the selected SHA-256 anchor."
                .into(),
        };
    }
    // The existing CA path is global timestamp configuration even though it is
    // grouped with the custom-provider fields in the persisted settings. It is
    // therefore also usable to pin a public RFC 3161 preset explicitly.
    let trust_path = settings.custom.ca_certificate_path.trim();
    if trust_path.is_empty() {
        return Rfc3161CryptographicVerification {
            status: ExternalTimestampStatus::VerificationConfigurationIncomplete,
            signature_verified: None,
            trust_chain_verified: None,
            cryptographic_verifier: String::new(),
            trust_anchor_sha256: Vec::new(),
            identity: None,
            message: "RFC 3161 response structure, SHA-256 message imprint, nonce, and the provider-returned policy OID (including a requested-policy match when configured) were checked, but no explicit TSA CA trust-anchor file is configured. The response remains archived without a VERIFIED claim."
                .into(),
        };
    }
    let (roots, fingerprints) = match load_trust_anchors(Path::new(trust_path)) {
        Ok(value) => value,
        Err(message) => {
            return Rfc3161CryptographicVerification {
                status: ExternalTimestampStatus::VerificationConfigurationIncomplete,
                signature_verified: None,
                trust_chain_verified: None,
                cryptographic_verifier: String::new(),
                trust_anchor_sha256: Vec::new(),
                identity: None,
                message,
            };
        }
    };
    let opts = sigstore_tsa::VerifyOpts::new().with_roots(roots);
    match sigstore_tsa::verify_timestamp_response(response, artifact_bytes, opts) {
        Ok(result) => Rfc3161CryptographicVerification {
            status: ExternalTimestampStatus::Verified,
            signature_verified: Some(true),
            trust_chain_verified: Some(true),
            cryptographic_verifier: RFC3161_CRYPTOGRAPHIC_VERIFIER.into(),
            trust_anchor_sha256: fingerprints.clone(),
            identity: Some(VerifiedTimestampIdentity {
                certificate_sha256: sha256_bytes(&result.signer_certificate_der),
                certificate_subject: result.signer_subject,
                certificate_issuer: result.signer_issuer,
                certificate_serial_number: result.signer_serial_number,
            }),
            message: format!(
                "RFC 3161 response verified with {RFC3161_CRYPTOGRAPHIC_VERIFIER}: SHA-256 message imprint, request nonce, provider-returned policy OID (and requested-policy match when configured), CMS signature, critical and sole timeStamping EKU, certificate validity at genTime, and chain to the configured trust anchor all match. Trust anchor SHA-256: {}.",
                fingerprints.join(", ")
            ),
        },
        Err(error) => {
            let (signature_verified, trust_chain_verified) = match &error {
                sigstore_tsa::Error::CertificateValidationError(_)
                | sigstore_tsa::Error::InvalidEKU => (Some(true), Some(false)),
                sigstore_tsa::Error::SignatureVerificationError(_) => (Some(false), None),
                _ => (Some(false), None),
            };
            Rfc3161CryptographicVerification {
                status: ExternalTimestampStatus::VerificationFailed,
                signature_verified,
                trust_chain_verified,
                cryptographic_verifier: RFC3161_CRYPTOGRAPHIC_VERIFIER.into(),
                trust_anchor_sha256: fingerprints,
                identity: None,
                message: format!(
                    "RFC 3161 response was archived, but CMS/X.509 verification failed: {error}."
                ),
            }
        }
    }
}
