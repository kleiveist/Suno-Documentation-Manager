use super::*;
use base64::Engine as _;
use std::cell::{Cell, RefCell};
use tempfile::tempdir;

trait TestResultExt<T, E> {
    fn must(self, message: &'static str) -> T;
    fn must_err(self, message: &'static str) -> E;
}

impl<T, E> TestResultExt<T, E> for std::result::Result<T, E> {
    #[track_caller]
    fn must(self, message: &'static str) -> T {
        match self {
            Ok(value) => value,
            Err(_) => std::panic::panic_any(message),
        }
    }

    #[track_caller]
    fn must_err(self, message: &'static str) -> E {
        match self {
            Ok(_) => std::panic::panic_any(message),
            Err(error) => error,
        }
    }
}

trait TestOptionExt<T> {
    fn must(self, message: &'static str) -> T;
}

impl<T> TestOptionExt<T> for Option<T> {
    #[track_caller]
    fn must(self, message: &'static str) -> T {
        match self {
            Some(value) => value,
            None => std::panic::panic_any(message),
        }
    }
}

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
        custom: CustomRfc3161Settings {
            // Parser/transport fixtures never enter certificate-chain
            // verification, but current RFC configuration requires an
            // explicit pin before a request can be attempted.
            ca_certificate_path: "unused-test-trust-anchor.der".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn rfc3161_response_for_digest(digest: &str) -> Vec<u8> {
    rfc3161_response_for_context(digest, None, "1.2.3.4")
}

fn rfc3161_response_for_context(digest: &str, nonce: Option<&[u8]>, policy_oid: &str) -> Vec<u8> {
    rfc3161_response_for_context_and_version(digest, nonce, policy_oid, &[1])
}

fn rfc3161_response_for_context_and_version(
    digest: &str,
    nonce: Option<&[u8]>,
    policy_oid: &str,
    version: &[u8],
) -> Vec<u8> {
    let digest = decode_sha256_hex(digest).must("digest bytes");
    let algorithm = der_sequence(&[
        der_oid("2.16.840.1.101.3.4.2.1").must("SHA-256 OID"),
        der_tlv(0x05, &[]),
    ]);
    let imprint = der_sequence(&[algorithm, der_tlv(0x04, &digest)]);
    let mut tst_info_values = vec![
        der_integer_unsigned(version),
        der_oid(policy_oid).must("policy OID"),
        imprint,
        der_integer_unsigned(&[42]),
        der_tlv(0x18, b"20260818120000Z"),
    ];
    if let Some(nonce) = nonce {
        tst_info_values.push(der_integer_unsigned(nonce));
    }
    let tst_info = der_sequence(&tst_info_values);
    let encapsulated = der_sequence(&[
        der_oid("1.2.840.113549.1.9.16.1.4").must("TSTInfo OID"),
        der_tlv(0xa0, &der_tlv(0x04, &tst_info)),
    ]);
    let signed_data = der_sequence(&[
        der_integer_unsigned(&[1]),
        der_tlv(0x31, &[]),
        encapsulated,
        der_tlv(0x31, &[]),
    ]);
    let token = der_sequence(&[
        der_oid("1.2.840.113549.1.7.2").must("SignedData OID"),
        der_tlv(0xa0, &signed_data),
    ]);
    der_sequence(&[der_sequence(&[der_integer_unsigned(&[0])]), token])
}

#[test]
fn provider_failure_is_captured_without_failing_finalization() {
    let mut settings = free_tsa_settings();
    settings.auto_after_finalization = true;
    let transport = MockTransport {
        calls: Cell::new(0),
        requests: RefCell::new(Vec::new()),
        response: Err(ProviderFailure {
            status: ExternalTimestampStatus::ProviderUnavailable,
            message: "Trusted timestamp endpoint unavailable.".into(),
        }),
    };

    let outcome = attempt_finalization_timestamp_with_transport(
        &settings,
        None,
        &"ab".repeat(32),
        b"immutable manifest bytes",
        &transport,
    );

    assert_eq!(transport.calls.get(), 1);
    assert!(outcome.response.is_none());
    assert_eq!(
        outcome.snapshot.provider_configuration_status,
        TimestampProviderConfigurationStatus::ConnectionFailed
    );
    assert_eq!(
        outcome.snapshot.technical_status,
        ExternalTimestampStatus::VerificationFailed
    );
    assert_eq!(
        outcome.snapshot.provider_metadata, None,
        "a failed request must not invent technical or qualification evidence"
    );
    assert!(outcome
        .snapshot
        .technical_message
        .contains("endpoint unavailable"));
}

#[test]
fn rfc3161_parser_binds_nonce_and_requested_policy() {
    let digest = "ab".repeat(32);
    let nonce = [0x01, 0x23, 0x45, 0x67];
    let response = rfc3161_response_for_context(&digest, Some(&nonce), "1.2.3.4");

    let parsed = parse_rfc3161_response(&response, &digest, "01234567", Some("1.2.3.4"));
    assert!(parsed.error.is_none(), "{:?}", parsed.error);
    assert_eq!(parsed.digest_match, Some(true));
    assert_eq!(parsed.nonce_match, Some(true));
    assert_eq!(parsed.policy_match, Some(true));

    let wrong_nonce = parse_rfc3161_response(&response, &digest, "89abcdef", Some("1.2.3.4"));
    assert_eq!(wrong_nonce.nonce_match, Some(false));

    let wrong_policy = parse_rfc3161_response(&response, &digest, "01234567", Some("1.2.3.5"));
    assert_eq!(wrong_policy.policy_match, Some(false));
}

#[test]
fn rfc3161_parser_rejects_tst_info_versions_other_than_one() {
    let digest = "ab".repeat(32);
    let response = rfc3161_response_for_context_and_version(&digest, Some(&[1]), "1.2.3.4", &[2]);

    let parsed = parse_rfc3161_response(&response, &digest, "01", Some("1.2.3.4"));

    assert_eq!(
        parsed.error.as_deref(),
        Some("RFC 3161 TSTInfo version must be 1.")
    );
}

fn decode_test_fixture(value: &str) -> Vec<u8> {
    let compact = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();
    base64::engine::general_purpose::STANDARD
        .decode(compact)
        .must("base64 test fixture")
}

#[test]
fn signed_rfc3161_fixture_requires_and_verifies_an_explicit_trust_anchor() {
    // Apache-2.0 sigstore-conformance fixture carried by sigstore-tsa.
    let response = decode_test_fixture(include_str!("../../testdata/rfc3161_valid.tsr.b64"));
    let payload = decode_test_fixture(include_str!("../../testdata/rfc3161_payload.b64"));
    let root = decode_test_fixture(include_str!("../../testdata/rfc3161_root.der.b64"));
    let directory = tempdir().must("temporary trust directory");
    let root_path = directory.path().join("tsa-root.der");
    fs::write(&root_path, root).must("trust root fixture");
    let settings = TimestampSettings {
        custom: CustomRfc3161Settings {
            ca_certificate_path: root_path.display().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let verified = verify_rfc3161_cryptography(
        &response,
        &sha256_bytes(&payload),
        Some(&payload),
        &settings,
    );

    assert_eq!(verified.status, ExternalTimestampStatus::Verified);
    assert_eq!(verified.signature_verified, Some(true));
    assert_eq!(verified.trust_chain_verified, Some(true));
    assert!(verified.message.contains("timeStamping EKU"));
    assert!(verified.message.contains("Trust anchor SHA-256"));

    let without_trust = verify_rfc3161_cryptography(
        &response,
        &sha256_bytes(&payload),
        Some(&payload),
        &TimestampSettings::default(),
    );
    assert_eq!(
        without_trust.status,
        ExternalTimestampStatus::VerificationConfigurationIncomplete
    );
    assert_ne!(without_trust.signature_verified, Some(true));
    assert_ne!(without_trust.trust_chain_verified, Some(true));
}

#[test]
fn signed_rfc3161_fixture_rejects_a_changed_artifact() {
    let response = decode_test_fixture(include_str!("../../testdata/rfc3161_valid.tsr.b64"));
    let mut payload = decode_test_fixture(include_str!("../../testdata/rfc3161_payload.b64"));
    let root = decode_test_fixture(include_str!("../../testdata/rfc3161_root.der.b64"));
    let directory = tempdir().must("temporary trust directory");
    let root_path = directory.path().join("tsa-root.der");
    fs::write(&root_path, root).must("trust root fixture");
    let settings = TimestampSettings {
        custom: CustomRfc3161Settings {
            ca_certificate_path: root_path.display().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    payload[0] ^= 1;

    let verification = verify_rfc3161_cryptography(
        &response,
        &sha256_bytes(&payload),
        Some(&payload),
        &settings,
    );

    assert_eq!(
        verification.status,
        ExternalTimestampStatus::VerificationFailed
    );
    assert_ne!(verification.signature_verified, Some(true));
    assert_ne!(verification.trust_chain_verified, Some(true));
}

#[test]
fn signed_rsa_rfc3161_fixture_verifies_through_the_application_profile() {
    let response = decode_test_fixture(include_str!("../../testdata/rfc3161_rsa_valid.tsr.b64"));
    let payload = decode_test_fixture(include_str!("../../testdata/rfc3161_payload.b64"));
    let root = decode_test_fixture(include_str!("../../testdata/rfc3161_rsa_root.der.b64"));
    let directory = tempdir().must("temporary trust directory");
    let root_path = directory.path().join("rsa-tsa-root.der");
    fs::write(&root_path, root).must("RSA trust root fixture");
    let settings = TimestampSettings {
        custom: CustomRfc3161Settings {
            ca_certificate_path: root_path.display().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let verified = verify_rfc3161_cryptography(
        &response,
        &sha256_bytes(&payload),
        Some(&payload),
        &settings,
    );

    assert_eq!(verified.status, ExternalTimestampStatus::Verified);
    assert_eq!(verified.signature_verified, Some(true));
    assert_eq!(verified.trust_chain_verified, Some(true));
    assert_eq!(
        verified.cryptographic_verifier,
        RFC3161_CRYPTOGRAPHIC_VERIFIER
    );
}

fn trusted_list_fixture(
    certificate_sha256: &str,
    periods: Vec<TrustedServiceStatusPeriod>,
    validation_status: TrustedListValidationStatus,
) -> ValidatedTrustedListSnapshot {
    ValidatedTrustedListSnapshot {
        evidence: TrustedListEvidence {
            source: "https://example.test/official-member-state-tl.xml".into(),
            territory: "DE".into(),
            version: "6".into(),
            sequence_number: "42".into(),
            issued_at: "2026-08-01T00:00:00Z".into(),
            next_update: "2027-02-01T00:00:00Z".into(),
            sha256: "ab".repeat(32),
            validation_status,
            validated_at: "2026-08-21T10:00:00Z".into(),
        },
        services: vec![ValidatedTrustedService {
            certificate_sha256: certificate_sha256.into(),
            policy_oid: "1.2.3.4".into(),
            provider_name: "Official TSP identity".into(),
            service_name: "Qualified timestamp service".into(),
            service_type: QUALIFIED_TIMESTAMP_SERVICE_TYPE.into(),
            service_identifier: "service-42".into(),
            periods,
        }],
    }
}

fn timestamp_identity(certificate_sha256: &str) -> TimestampServiceIdentity {
    TimestampServiceIdentity {
        certificate_sha256: certificate_sha256.into(),
        certificate_subject: "CN=Fixture TSA".into(),
        certificate_issuer: "CN=Fixture Root".into(),
        certificate_serial_number: "01".into(),
        policy_oid: "1.2.3.4".into(),
        service_identifier: String::new(),
    }
}

#[test]
fn technically_verified_rfc3161_identity_does_not_imply_eidas_qualification() {
    let response = decode_test_fixture(include_str!("../../testdata/rfc3161_valid.tsr.b64"));
    let payload = decode_test_fixture(include_str!("../../testdata/rfc3161_payload.b64"));
    let root = decode_test_fixture(include_str!("../../testdata/rfc3161_root.der.b64"));
    let directory = tempdir().must("temporary trust directory");
    let root_path = directory.path().join("tsa-root.der");
    fs::write(&root_path, root).must("trust root fixture");
    let settings = TimestampSettings {
        custom: CustomRfc3161Settings {
            ca_certificate_path: root_path.display().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let verified = verify_rfc3161_cryptography(
        &response,
        &sha256_bytes(&payload),
        Some(&payload),
        &settings,
    );
    let identity = verified.identity.must("verified signer identity");
    let qualification = provider_identity_qualification(&identity, "1.2.3.4");

    assert_eq!(verified.status, ExternalTimestampStatus::Verified);
    assert!(!identity.certificate_sha256.is_empty());
    assert_eq!(
        qualification.provider_identity_status,
        TimestampQualificationStatus::ProviderIdentityVerified
    );
    assert_eq!(
        qualification.eidas_qualification_status,
        TimestampQualificationStatus::NotChecked
    );
    assert!(!qualification.message.contains("not qualified"));
}

#[test]
fn validated_trusted_list_recognizes_qualified_service_at_timestamp() {
    let certificate_sha256 = "11".repeat(32);
    let snapshot = trusted_list_fixture(
        &certificate_sha256,
        vec![TrustedServiceStatusPeriod {
            valid_from: "2025-01-01T00:00:00Z".into(),
            valid_until: String::new(),
            status_uri: QUALIFIED_SERVICE_GRANTED_STATUS.into(),
        }],
        TrustedListValidationStatus::Verified,
    );

    let result = verify_provider_qualification(
        &timestamp_identity(&certificate_sha256),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(
        result.status,
        TimestampQualificationStatus::QualifiedServiceVerified
    );
    assert_eq!(
        result.provider_identity_status,
        TimestampQualificationStatus::ProviderIdentityVerified
    );
    assert_eq!(
        result.trust_service_status,
        TimestampQualificationStatus::TrustServiceVerified
    );
    assert_eq!(
        result.qualification_at_timestamp,
        TimestampQualificationStatus::QualifiedServiceVerified
    );
    assert_eq!(result.trust_service_provider, "Official TSP identity");
    assert_eq!(
        result
            .trusted_list
            .as_ref()
            .must("trusted list evidence")
            .validation_status,
        TrustedListValidationStatus::Verified
    );
}

#[test]
fn custom_provider_label_and_endpoint_cannot_set_qualification() {
    let certificate_sha256 = "22".repeat(32);
    let snapshot = trusted_list_fixture(
        &certificate_sha256,
        vec![TrustedServiceStatusPeriod {
            valid_from: "2025-01-01T00:00:00Z".into(),
            valid_until: String::new(),
            status_uri: QUALIFIED_SERVICE_GRANTED_STATUS.into(),
        }],
        TrustedListValidationStatus::Verified,
    );
    let freely_configured_provider_name = "Meine TSA – eIDAS qualified";
    let freely_configured_endpoint = "https://marketing-label.example.test/tsa";

    let result = verify_provider_qualification(
        &timestamp_identity(&certificate_sha256),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(result.trust_service_provider, "Official TSP identity");
    assert_ne!(
        result.trust_service_provider,
        freely_configured_provider_name
    );
    assert!(!result.message.contains(freely_configured_endpoint));
}

#[test]
fn unknown_identity_is_not_verified_without_negative_provider_finding() {
    let snapshot = trusted_list_fixture(
        &"33".repeat(32),
        Vec::new(),
        TrustedListValidationStatus::Verified,
    );

    let result = verify_provider_qualification(
        &timestamp_identity(&"44".repeat(32)),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(result.status, TimestampQualificationStatus::NotVerified);
    assert_eq!(
        result.provider_identity_status,
        TimestampQualificationStatus::ProviderIdentityVerified
    );
    assert_eq!(
        result.eidas_qualification_status,
        TimestampQualificationStatus::NotVerified
    );
    assert!(result.message.contains("not a finding"));
    assert!(!result.message.contains("NOT QUALIFIED"));
}

#[test]
fn unavailable_trusted_list_does_not_change_technical_timestamp_status() {
    let snapshot = trusted_list_fixture(
        &"55".repeat(32),
        Vec::new(),
        TrustedListValidationStatus::Failed,
    );
    let technical_status = ExternalTimestampStatus::Verified;

    let qualification = verify_provider_qualification(
        &timestamp_identity(&"55".repeat(32)),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(technical_status, ExternalTimestampStatus::Verified);
    assert_eq!(
        qualification.status,
        TimestampQualificationStatus::CheckFailed
    );
}

#[test]
fn historical_qualification_is_evaluated_at_timestamp_time() {
    let certificate_sha256 = "66".repeat(32);
    let snapshot = trusted_list_fixture(
        &certificate_sha256,
        vec![
            TrustedServiceStatusPeriod {
                valid_from: "2025-01-01T00:00:00Z".into(),
                valid_until: "2026-08-20T00:00:00Z".into(),
                status_uri: QUALIFIED_SERVICE_GRANTED_STATUS.into(),
            },
            TrustedServiceStatusPeriod {
                valid_from: "2026-08-20T00:00:00Z".into(),
                valid_until: String::new(),
                status_uri: "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn".into(),
            },
        ],
        TrustedListValidationStatus::Verified,
    );

    let result = verify_provider_qualification(
        &timestamp_identity(&certificate_sha256),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(
        result.qualification_at_timestamp,
        TimestampQualificationStatus::QualifiedServiceVerified
    );
    assert_eq!(
        result.current_qualification_status,
        TimestampQualificationStatus::NotVerified
    );
    assert_eq!(result.status_valid_until, "2026-08-20T00:00:00Z");
    assert_eq!(
        result.current_service_status,
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn"
    );
    assert_eq!(result.current_status_valid_from, "2026-08-20T00:00:00Z");
}

#[test]
fn current_qualification_uses_its_own_service_status_period() {
    let certificate_sha256 = "77".repeat(32);
    let withdrawn = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn".to_owned();
    let snapshot = trusted_list_fixture(
        &certificate_sha256,
        vec![
            TrustedServiceStatusPeriod {
                valid_from: "2025-01-01T00:00:00Z".into(),
                valid_until: "2026-08-20T00:00:00Z".into(),
                status_uri: withdrawn.clone(),
            },
            TrustedServiceStatusPeriod {
                valid_from: "2026-08-20T00:00:00Z".into(),
                valid_until: String::new(),
                status_uri: QUALIFIED_SERVICE_GRANTED_STATUS.into(),
            },
        ],
        TrustedListValidationStatus::Verified,
    );

    let result = verify_provider_qualification(
        &timestamp_identity(&certificate_sha256),
        "2026-08-18T12:00:00Z",
        "2026-08-21T10:00:00Z",
        &snapshot,
    );

    assert_eq!(
        result.qualification_at_timestamp,
        TimestampQualificationStatus::NotVerified
    );
    assert_eq!(
        result.current_qualification_status,
        TimestampQualificationStatus::QualifiedServiceVerified
    );
    assert_eq!(result.service_status, withdrawn);
    assert_eq!(
        result.current_service_status,
        QUALIFIED_SERVICE_GRANTED_STATUS
    );
    assert_eq!(result.status_valid_from, "2025-01-01T00:00:00Z");
    assert_eq!(result.current_status_valid_from, "2026-08-20T00:00:00Z");
}

#[test]
fn qualification_and_concrete_timestamp_failure_are_independent() {
    let metadata = TimestampProviderMetadata {
        verification_result: ExternalTimestampStatus::VerificationFailed,
        qualification: Some(TimestampQualificationRecord {
            status: TimestampQualificationStatus::QualifiedServiceVerified,
            eidas_qualification_status: TimestampQualificationStatus::QualifiedServiceVerified,
            qualification_at_timestamp: TimestampQualificationStatus::QualifiedServiceVerified,
            ..Default::default()
        }),
        ..Default::default()
    };

    assert_eq!(
        metadata.verification_result,
        ExternalTimestampStatus::VerificationFailed
    );
    assert_eq!(
        metadata.qualification.must("qualification record").status,
        TimestampQualificationStatus::QualifiedServiceVerified
    );
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

    let response = request_timestamp_with_transport(
        &free_tsa_settings(),
        None,
        &requested_digest,
        None,
        &mock,
    )
    .must("provider response is retained for diagnosis");

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
                .must("digest")
                .as_slice()));
}

#[test]
fn provider_test_reports_an_unusable_rfc3161_response() {
    let mock = MockTransport::successful(b"not an RFC 3161 response".to_vec());

    let result = test_provider_with_transport(&free_tsa_settings(), None, &mock);

    assert_eq!(mock.calls.get(), 1);
    assert_eq!(
        result.status,
        TimestampProviderConfigurationStatus::ProviderError
    );
    assert!(result.message.contains("could not be technically verified"));
}

#[test]
fn open_timestamps_readiness_never_claims_rfc3161_or_a_tsa_trust_anchor() {
    let settings = TimestampSettings {
        enabled: true,
        provider: TimestampProviderKind::OpenTimestamps,
        ..Default::default()
    };

    let (status, message) = settings_status(&settings, false);

    assert_eq!(status, TimestampProviderConfigurationStatus::Ready);
    assert!(message.contains("not RFC 3161"));
    assert!(message.contains("ATTACHED"));
    assert!(message.contains("Bitcoin anchoring"));
    assert!(!message.contains("trust anchor"));
    assert!(!message.contains("TSA"));
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

    let response = request_timestamp_with_transport(&settings, None, &digest, None, &mock)
        .must("OTS proof response");

    assert_eq!(response.evidence_extension, "ots");
    assert_eq!(
        response.evidence_bytes,
        open_timestamps_detached_proof(&digest, &raw_calendar_response).must("detached proof")
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
        decode_sha256_hex(&digest).must("digest").as_slice()
    );
    assert_eq!(
        &response.evidence_bytes[prefix_length + 34..],
        raw_calendar_response.as_slice()
    );
    let raw_archive = response
        .raw_provider_response
        .as_ref()
        .must("raw provider archive");
    assert_eq!(raw_archive.extension, "bin");
    assert_eq!(raw_archive.bytes, raw_calendar_response);
    assert_eq!(response.status, ExternalTimestampStatus::Attached);
    assert!(response.metadata.protocol.contains("OpenTimestamps"));
    assert!(!response.metadata.protocol.contains("RFC 3161"));
    assert!(response
        .metadata
        .protocol
        .contains("Bitcoin anchoring pending"));
    assert_eq!(response.metadata.provider_digest_match, Some(true));
}

#[test]
fn open_timestamps_sidecar_keeps_detached_proof_and_raw_response_integrity_bound() {
    let directory = tempdir().must("temporary track root");
    let track_root = directory.path();
    fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR)).must("certificate directory");
    let manifest = track_root.join(certificate::MANIFEST_FILE);
    fs::write(&manifest, b"{\"finalized\":true}\n").must("manifest");
    let manifest_digest = sha256_file(&manifest).must("manifest digest");
    fs::write(
        track_root.join(certificate::CERTIFICATE_HASH_FILE),
        format!("{manifest_digest}  {}\n", certificate::MANIFEST_FILE),
    )
    .must("certificate hash set");

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
        None,
        &MockTransport::successful(raw_calendar_response.clone()),
    )
    .must("OTS response");
    let proof_bytes = response.evidence_bytes.clone();
    let proof_source = track_root.join("provider-proof.ots");
    fs::write(&proof_source, &proof_bytes).must("proof source");

    let staged = stage_provider_response(
        track_root,
        "certificate-fixture",
        "finalization-snapshot-fixture",
        &manifest_digest,
        &proof_source,
        response,
    )
    .must("stage automatic OTS proof");
    let metadata = staged
        .record
        .provider_metadata
        .as_ref()
        .must("provider metadata");
    assert_eq!(
        metadata.provider_response_file_name,
        "PROVIDER_RESPONSE.bin"
    );
    let stage_directory = track_root.join(STAGING_DIR).join(&staged.record.id);
    assert_eq!(
        fs::read(stage_directory.join("TIMESTAMP_EVIDENCE.ots")).must("detached proof"),
        proof_bytes
    );
    assert_eq!(
        fs::read(stage_directory.join("PROVIDER_RESPONSE.bin")).must("raw response"),
        raw_calendar_response
    );
    assert_eq!(
        metadata.provider_response_sha256,
        sha256_file(&stage_directory.join("PROVIDER_RESPONSE.bin")).must("raw hash")
    );
    let hash_list = fs::read_to_string(stage_directory.join(HASH_LIST_FILE)).must("hash list");
    assert!(hash_list.contains("PROVIDER_RESPONSE.bin"));
    let markdown =
        fs::read_to_string(stage_directory.join(MARKDOWN_FILE)).must("Markdown addendum");
    assert!(markdown.contains(
            "Timestamp value [Provider-derived metadata]: PENDING — OpenTimestamps proof verification / upgrade required"
        ));
    assert!(markdown.contains("Local manifest / proof binding [System verification]: YES"));
    assert!(
        markdown.contains("CMS signature / trust chain [System verification]: N/A — not RFC 3161")
    );
    assert!(markdown.contains("Calendar endpoint [Provider-derived metadata]"));
    assert!(markdown.contains("Calendar endpoint identifier [Provider-derived metadata]"));
    assert!(!markdown.contains("Provider digest match [System verification]"));
    verify_record_in_directory(track_root, &stage_directory, &staged.record, None)
        .must("sidecar verifies including raw response");
    discard_staged(track_root, &staged).must("discard test stage");
}

#[test]
fn verification_pins_published_bytes_and_never_requires_current_renderer_output() {
    let directory = tempdir().must("temporary track root");
    let track_root = directory.path();
    fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR)).must("certificate directory");
    let anchor = track_root.join(certificate::MANIFEST_FILE);
    fs::write(&anchor, b"{\"historical\":true}\n").must("manifest anchor");
    let source = track_root.join("timestamp.json");
    fs::write(&source, b"{\"provider\":\"fixture\"}\n").must("timestamp source");
    let anchor_sha256 = sha256_file(&anchor).must("anchor hash");
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
    .must("stage timestamp");
    let mut record = publish(track_root, &staged).must("publish timestamp");
    let record_directory = track_root
        .join(&record.record_relative_path)
        .parent()
        .must("record directory")
        .to_path_buf();

    let historical_markdown = b"# Historical addendum bytes\n\nThese bytes intentionally do not come from the current renderer.\n";
    assert_ne!(
        historical_markdown.as_slice(),
        render_markdown(&record).as_bytes()
    );
    fs::write(record_directory.join(MARKDOWN_FILE), historical_markdown)
        .must("historical markdown bytes");
    record.markdown_sha256 =
        sha256_file(&record_directory.join(MARKDOWN_FILE)).must("markdown hash");
    fs::write(
        record_directory.join(RECORD_FILE),
        immutable_record_bytes(&record).must("immutable record bytes"),
    )
    .must("updated immutable record fixture");
    let hashes =
        artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").must("artifact hashes");
    fs::write(
        record_directory.join(HASH_LIST_FILE),
        render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).must("versioned hash list"),
    )
    .must("updated hash list fixture");

    verify_published_record(track_root, &record)
        .must("persisted historical bytes verify without re-rendering");
    let immutable: serde_json::Value =
        serde_json::from_slice(&fs::read(record_directory.join(RECORD_FILE)).must("record bytes"))
            .must("record JSON");
    assert_eq!(
        immutable["integrityVerifiedAtPublication"].as_bool(),
        Some(true)
    );
    assert!(immutable.get("integrityVerified").is_none());
    assert!(fs::read_to_string(record_directory.join(HASH_LIST_FILE))
        .must("hash list")
        .starts_with(HASH_LIST_V1_HEADER));

    // Even a self-consistent rewritten hash list cannot authorize extra
    // runtime/trust claims in the immutable v1 JSON record.
    let mut injected: serde_json::Value = serde_json::from_slice(
        &fs::read(record_directory.join(RECORD_FILE)).must("immutable record bytes"),
    )
    .must("immutable record JSON");
    let object = injected.as_object_mut().must("record object");
    object.insert("integrityVerified".into(), serde_json::Value::Bool(true));
    object.insert(
        "providerQualificationVerifiedBySunoDM".into(),
        serde_json::Value::Bool(true),
    );
    fs::write(
        record_directory.join(RECORD_FILE),
        serde_json::to_vec_pretty(&injected).must("injected JSON bytes"),
    )
    .must("injected record");
    let hashes =
        artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").must("injected hashes");
    fs::write(
        record_directory.join(HASH_LIST_FILE),
        render_hash_list(SIDECAR_FORMAT_VERSION, &hashes).must("injected hash list"),
    )
    .must("self-consistent injected hash list");
    let error = verify_published_record(track_root, &record)
        .must_err("injected immutable claims must fail verification");
    assert!(error.to_string().contains("exact immutable"));
}

mod finalization_tamper;
mod legacy_sidecar;
