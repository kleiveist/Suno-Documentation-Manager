use super::*;

#[test]
fn renders_complete_terms_record_and_finalization_timestamp_status() {
    let mut fixture = Fixture::new(1);
    let mut terms = evidence_item(10);
    terms.role = EvidenceRole::SunoTermsRights;
    terms.relative_path = "04_LICENSES/suno-terms.pdf".into();
    terms.metadata.document_title = "Archived Suno Terms".into();
    terms.metadata.provider = "Suno".into();
    terms.metadata.source_url = "https://suno.example/terms".into();
    terms.metadata.retrieval_date = "2026-01-03".into();
    terms.metadata.effective_date = "2026-01-01".into();
    terms.metadata.applicable_production_period = "2026-01-01 to 2026-01-31".into();
    terms.metadata.factual_note = "Locally archived copy".into();
    terms.metadata.original_file_name = "Suno Terms.pdf".into();
    let terms_id = terms.id.clone();
    fixture.evidence.push(terms);
    let mut future_terms = evidence_item(11);
    future_terms.role = EvidenceRole::SunoTermsRights;
    future_terms.relative_path = "04_LICENSES/suno-terms-future.pdf".into();
    future_terms.metadata.document_title = "Future archived Suno Terms".into();
    future_terms.metadata.effective_date = "2026-02-01".into();
    future_terms.metadata.applicable_production_period = "From 2026-02-01".into();
    fixture.evidence.push(future_terms);
    fixture
        .evidence
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    assert!(text.contains("Archived Suno Terms"));
    assert!(text.contains(&terms_id));
    assert!(text.contains("2026-01-01 to 2026-01-31"));
    assert!(text.contains("Suno Terms.pdf"));
    assert!(text.contains("Terms applicable to documented production period"));
    assert!(text.contains("Future archived Terms"));
    assert!(text.contains("Not applicable to production before"));
    assert!(text.contains("2026-02-01"));
    for expected in [
        "Terms document 1 evidence ID [System value]",
        "Terms document 1 title [User-confirmed fact]",
        "Terms document 1 provider/source [User-confirmed fact]",
        "Terms document 1 source URL [User-confirmed fact]",
        "Terms document 1 retrieval date [User-confirmed fact]",
        "Terms document 1 effective date [User-confirmed fact]",
        "Terms document 1 applicable production period [User-confirmed fact]",
        "Terms document 1 factual note [User-confirmed fact]",
        "Terms document 1 path [System value]",
        "Terms document 1 original filename [Evidence-derived metadata]",
        "Terms document 1 SHA-256 [System verification]",
        "Terms document 1 imported at [System value]",
        "Terms document 1 provenance [System value]",
    ] {
        assert!(
            normalized.contains(expected),
            "missing Terms provenance label: {expected}"
        );
    }
    assert!(text.contains("A. Technical timestamp"));
    assert!(normalized.contains("Provider configuration DISABLED"));
    assert!(normalized.contains("Concrete timestamp status NOT RECORDED"));
    assert!(normalized.contains("Provider identity NOT CHECKED"));
    assert!(normalized.contains("eIDAS qualification NOT CHECKED"));

    fixture.render_options = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    let (_, german) = parse_text(&fixture.generate());
    let german = normalized_text(&german);
    assert!(
        german.contains("Für den dokumentierten Produktionszeitraum geltende Nutzungsbedingungen")
    );
    assert!(german.contains("Künftig geltende archivierte Nutzungsbedingungen"));
    // PDF text extraction interleaves the adjacent value between wrapped
    // label lines, so assert both label fragments independently.
    assert!(german.contains("Nicht auf Produktionen vor diesem Datum"));
    assert!(german.contains("anwendbar:"));
    assert!(!german.contains("Not applicable to production before"));
    for expected in [
        "Evidence-ID",
        "Nutzungsbedingungen 1 Titel",
        "Anbieter/Quelle",
        "Quell-URL",
        "Abrufdatum",
        "anwendbarer Produktionszeitraum",
        "sachliche Anmerkung",
        "Nutzungsbedingungen 1 Pfad",
        "ursprünglicher Dateiname",
        "importiert am",
        "Nutzungsbedingungen 1 Herkunft",
    ] {
        assert!(
            german.contains(expected),
            "German Terms detail omitted localized label fragment: {expected}"
        );
    }
    for forbidden in [
        " evidence ID [",
        " title [",
        " provider/source [",
        " source URL [",
        " retrieval date [",
        " applicable production period [",
        " factual note [",
        " path [",
        " original filename [",
        " imported at [",
        " provenance [",
    ] {
        assert!(
            !german.contains(forbidden),
            "German Terms detail retained English template fragment: {forbidden}"
        );
    }
}

#[test]
fn final_pdf_separates_concrete_timestamp_from_provider_qualification() {
    let mut fixture = timestamp_qualification_fixture();

    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    for expected in [
        "Custom label that cannot grant qualification",
        "RFC 3161 Timestamp Protocol",
        "Concrete timestamp status VERIFIED",
        "Manifest hash binding VERIFIED",
        "Provider identity PROVIDER IDENTITY VERIFIED",
        "eIDAS qualification NOT VERIFIED",
    ] {
        assert!(
            normalized.contains(expected),
            "final PDF omitted {expected}"
        );
    }
    assert!(!text.contains("eIDAS QUALIFIED TRUST SERVICE – VERIFIED"));

    {
        let qualification = fixture
            .finalization_timestamp
            .provider_metadata
            .as_mut()
            .and_then(|metadata| metadata.qualification.as_mut())
            .expect("qualification fixture");
        qualification.status = TimestampQualificationStatus::QualifiedServiceVerified;
        qualification.trust_service_status = TimestampQualificationStatus::TrustServiceVerified;
        qualification.eidas_qualification_status =
            TimestampQualificationStatus::QualifiedServiceVerified;
        qualification.current_qualification_status =
            TimestampQualificationStatus::QualifiedServiceVerified;
        qualification.qualification_at_timestamp =
            TimestampQualificationStatus::QualifiedServiceVerified;
    }
    fixture.finalization_timestamp.technical_status = ExternalTimestampStatus::VerificationFailed;

    let error = fixture
        .generate_result()
        .expect_err("qualified badge requires validated Trusted List evidence");
    assert!(error.to_string().contains("without Trusted List evidence"));

    {
        let qualification = fixture
            .finalization_timestamp
            .provider_metadata
            .as_mut()
            .and_then(|metadata| metadata.qualification.as_mut())
            .expect("qualification fixture");
        qualification.trusted_list = Some(crate::model::TrustedListEvidence {
            source: "https://official.example.test/member-state-tl.xml".into(),
            territory: "DE".into(),
            version: "6".into(),
            sequence_number: "42".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            next_update: "2026-07-01T00:00:00Z".into(),
            sha256: DIGEST_B.into(),
            validation_status: TrustedListValidationStatus::Verified,
            validated_at: "2026-01-04T12:31:30Z".into(),
        });
        qualification.trust_service_provider = "Official TSP identity".into();
        qualification.trust_service_name = "Qualified timestamp service".into();
        qualification.service_type = "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST".into();
        qualification.service_status =
            "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted".into();
        qualification.current_service_status =
            "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted".into();
        qualification.qualification_type = "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST".into();
        qualification.status_valid_from = "2025-01-01T00:00:00Z".into();
        qualification.current_status_valid_from = "2025-01-01T00:00:00Z".into();
    }

    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    assert!(normalized.contains("Concrete timestamp status VERIFICATION FAILED"));
    assert!(text.contains("eIDAS QUALIFIED TRUST SERVICE – VERIFIED"));

    {
        let qualification = fixture
            .finalization_timestamp
            .provider_metadata
            .as_mut()
            .and_then(|metadata| metadata.qualification.as_mut())
            .expect("qualification fixture");
        qualification.current_service_status =
            "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn".into();
    }
    let error = fixture
        .generate_result()
        .expect_err("current qualification requires its own granted status period");
    assert!(error
        .to_string()
        .contains("current qualified-service status"));
    fixture
        .finalization_timestamp
        .provider_metadata
        .as_mut()
        .and_then(|metadata| metadata.qualification.as_mut())
        .expect("qualification fixture")
        .current_service_status =
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted".into();

    assert_german_timestamp_qualification(&mut fixture);
}

fn timestamp_qualification_fixture() -> Fixture {
    let mut fixture = Fixture::new(1);
    fixture.finalization_timestamp = FinalizationTimestampSnapshot {
        provider: "Custom label that cannot grant qualification".into(),
        provider_configuration_status: TimestampProviderConfigurationStatus::Ready,
        provider_configuration_message: "Provider configuration ready.".into(),
        automatic_request_enabled: true,
        technical_status: ExternalTimestampStatus::Verified,
        technical_message: "RFC 3161 timestamp cryptographically verified.".into(),
        timestamp_value: "2026-01-04T12:30:00Z".into(),
        external_reference_id: "serial-42".into(),
        provider_verification_url: "https://timestamp.example.invalid".into(),
        provider_metadata: Some(TimestampProviderMetadata {
            adapter: "custom_rfc3161".into(),
            protocol: "RFC 3161 Timestamp Protocol".into(),
            request_algorithm: "SHA-256".into(),
            response_format: "RFC 3161 TimeStampResp (.tsr)".into(),
            response_structure_valid: Some(true),
            provider_digest_match: Some(true),
            signature_verification_applicable: Some(true),
            signature_verified: Some(true),
            trust_chain_verification_applicable: Some(true),
            trust_chain_verified: Some(true),
            provider_identity_verified: Some(true),
            certificate_subject: "CN=Fixture TSA".into(),
            issuer: "CN=Fixture Root".into(),
            certificate_serial_number: "42".into(),
            certificate_sha256: DIGEST_A.into(),
            verification_result: ExternalTimestampStatus::Verified,
            verification_message: "Technically verified.".into(),
            verification_timestamp: "2026-01-04T12:31:00Z".into(),
            qualification: Some(crate::model::TimestampQualificationRecord {
                status: TimestampQualificationStatus::ProviderIdentityVerified,
                provider_identity_status: TimestampQualificationStatus::ProviderIdentityVerified,
                trust_service_status: TimestampQualificationStatus::NotVerified,
                eidas_qualification_status: TimestampQualificationStatus::NotVerified,
                current_qualification_status: TimestampQualificationStatus::NotVerified,
                qualification_at_timestamp: TimestampQualificationStatus::NotVerified,
                checked_at: "2026-01-04T12:31:30Z".into(),
                message: "No qualified-service evidence was verified.".into(),
                identity: crate::model::TimestampServiceIdentity {
                    certificate_sha256: DIGEST_A.into(),
                    certificate_subject: "CN=Fixture TSA".into(),
                    certificate_issuer: "CN=Fixture Root".into(),
                    certificate_serial_number: "42".into(),
                    policy_oid: "1.2.3.4".into(),
                    service_identifier: String::new(),
                },
                ..Default::default()
            }),
            ..Default::default()
        }),
    };
    fixture
}

fn assert_german_timestamp_qualification(fixture: &mut Fixture) {
    fixture.render_options = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    let (_, german_text) = parse_text(&fixture.generate());
    let german = normalized_text(&german_text);
    for expected in [
        "A. Technischer Zeitstempel",
        "B. Providervertrauen und Qualifikation",
        "Status des konkreten Zeitstempels VERIFIKATION FEHLGESCHLAGEN",
        "Provideridentität PROVIDERIDENTITÄT VERIFIZIERT",
        "eIDAS-Qualifikation QUALIFIZIERTER DIENST VERIFIZIERT",
        "Dienststatus zum Zeitstempelzeitpunkt",
        "Aktueller Dienststatus",
        "Validierung der Vertrauensliste VERIFIZIERT",
        "eIDAS-QUALIFIZIERTER VERTRAUENSDIENST – VERIFIZIERT",
    ] {
        assert!(
            german.contains(expected),
            "German timestamp PDF omitted {expected}"
        );
    }
    for forbidden in [
        "A. Technical timestamp",
        "B. Provider trust and qualification",
        "eIDAS QUALIFIED TRUST SERVICE – VERIFIED",
    ] {
        assert!(
            !german.contains(forbidden),
            "German timestamp PDF retained {forbidden}"
        );
    }
}

#[test]
fn rejects_legacy_external_timestamp_from_phase_one_pdf() {
    let mut fixture = Fixture::new(1);
    let mut timestamp = evidence_item(11);
    timestamp.role = EvidenceRole::ExternalTimestamp;
    timestamp.relative_path = "03_DOCUMENTATION/timestamp.pdf".into();
    fixture.evidence.push(timestamp);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let error = fixture
        .generate_result()
        .expect_err("phase-one PDF rejects timestamp evidence");
    assert!(error
        .to_string()
        .contains("cannot contain legacy external timestamp"));
}

#[test]
fn external_timestamp_addendum_is_complete_factual_and_deterministic() {
    let snapshot = ExternalTimestampPdfSnapshot {
        certificate_id: CERTIFICATE_ID,
        provider: "Example Timestamp Issuer",
        timestamp_type: "Qualified electronic timestamp – user declared",
        timestamp_value: "2026-01-04T12:00:00Z",
        referenced_artifact: "EVIDENCE_MANIFEST.json",
        referenced_artifact_path: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referenced_sha256: DIGEST_A,
        actual_sha256: DIGEST_B,
        referenced_hash_match: Some(false),
        evidence_file_name: "timestamp-evidence.tsr",
        evidence_sha256: DIGEST_C,
        imported_at: "2026-01-04T12:05:00Z",
        provenance: "managed_copy",
        external_reference_id: "reference-1234",
        provider_verification_url: "https://timestamp.example/verify/reference-1234",
        note: "User-supplied timestamp record",
        provider_metadata: None,
    };

    let first =
        generate_external_timestamp_addendum_pdf(&snapshot).expect("generate timestamp addendum");
    let second =
        generate_external_timestamp_addendum_pdf(&snapshot).expect("regenerate timestamp addendum");
    assert_eq!(first, second);
    validate_pdfa_2b_bytes(&first).expect("timestamp addendum PDF/A-2b structure");
    let (document, text) = parse_text(&first);
    assert!(!document.pages.is_empty());
    let normalized = normalized_text(&text);
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    for expected in [
        "Example Timestamp Issuer",
        "Qualified electronic timestamp – user declared",
        "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        DIGEST_A,
        DIGEST_B,
        DIGEST_C,
        EXTERNAL_TIMESTAMP_DISCLAIMER,
        CERTIFICATE_ID,
        "Page 1 /",
    ] {
        let expected_compact = expected
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            compact.contains(&expected_compact),
            "missing addendum value: {expected}"
        );
    }
    assert!(normalized.contains("Referenced hash match [System verification] NO"));
    for provenance_label in [
        "Original Certificate ID [System value]",
        "Record source [System value]",
        "Provider / issuer [Legacy user-recorded fact]",
        "Evidence filename [Evidence-derived metadata]",
        "External reference ID [Legacy user-recorded fact]",
        "Provider verification URL [Legacy user-recorded fact]",
        "Imported at [System value]",
        "Provenance [System value]",
    ] {
        assert!(
            normalized.contains(provenance_label),
            "missing timestamp provenance label: {provenance_label}"
        );
    }
    assert!(!text.contains("System verified qualified timestamp"));
    assert!(text.contains("Legacy manually recorded timestamp evidence"));
    assert!(!text.contains("[User-confirmed fact]"));
    assert!(normalized.contains(
        "Provider verification result [System verification] ATTACHED – legacy manually recorded; no provider verification recorded"
    ));
    assert!(text.contains(&format!(
        "Page {} / {}",
        document.pages.len(),
        document.pages.len()
    )));
    for (index, page) in document.extract_text().into_iter().enumerate() {
        let page_text = page.join("\n");
        assert!(page_text.contains(CERTIFICATE_ID));
        if index > 0 {
            assert!(
                page_text.contains("Technical Evidence Certificate"),
                "missing document header on continuation page {}",
                index + 1
            );
        }
        assert!(page_text.contains(&format!("Page {} / {}", index + 1, document.pages.len())));
    }

    let matching_snapshot = ExternalTimestampPdfSnapshot {
        actual_sha256: DIGEST_A,
        referenced_hash_match: Some(true),
        ..snapshot
    };
    let (_, matching_text) = parse_text(
        &generate_external_timestamp_addendum_pdf(&matching_snapshot)
            .expect("generate matching timestamp addendum"),
    );
    assert!(
        normalized_text(&matching_text).contains("Referenced hash match [System verification] YES")
    );
}

#[test]
fn automatic_timestamp_addendum_renders_provider_metadata_without_user_or_legal_claims() {
    let metadata = TimestampProviderMetadata {
        adapter: "sigstore_public_tsa_rfc3161".into(),
        protocol: "RFC 3161 Timestamp Protocol".into(),
        request_algorithm: "SHA-256".into(),
        response_format: "RFC 3161 TimeStampResp (.tsr)".into(),
        provider_endpoint_identifier: "https://timestamp.sigstore.dev/api/v1/timestamp".into(),
        provider_response_file_name: "PROVIDER_RESPONSE.bin".into(),
        provider_response_sha256: DIGEST_B.into(),
        referenced_revision_id: "finalization-snapshot-2026-0001".into(),
        issuer: "Example TSA Issuer".into(),
        certificate_subject: "CN=Example TSA".into(),
        certificate_serial_number: "01:23:45:67".into(),
        policy_oid: "1.2.3.4.5".into(),
        request_nonce: "0123456789abcdef".into(),
        response_nonce: "0123456789abcdef".into(),
        nonce_match: Some(true),
        requested_policy_oid: "1.2.3.4.5".into(),
        policy_match: Some(true),
        cryptographic_verifier: crate::external_timestamp::RFC3161_CRYPTOGRAPHIC_VERIFIER.into(),
        trust_anchor_sha256: vec![DIGEST_A.into()],
        response_structure_valid: Some(true),
        provider_digest_match: Some(true),
        signature_verified: Some(true),
        trust_chain_verified: Some(true),
        verification_result: ExternalTimestampStatus::Verified,
        verification_message:
            "RFC 3161 response, request binding, CMS signature, and trust chain were verified."
                .into(),
        verification_timestamp: "2026-01-04T12:05:00Z".into(),
        ..TimestampProviderMetadata::default()
    };
    let snapshot = ExternalTimestampPdfSnapshot {
        certificate_id: CERTIFICATE_ID,
        provider: "Sigstore Public TSA",
        timestamp_type: "External integrity timestamp",
        timestamp_value: "2026-01-04T12:00:00Z",
        referenced_artifact: "EVIDENCE_MANIFEST.json",
        referenced_artifact_path: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referenced_sha256: DIGEST_A,
        actual_sha256: DIGEST_A,
        referenced_hash_match: Some(true),
        evidence_file_name: "provider-response.tsr",
        evidence_sha256: DIGEST_C,
        imported_at: "2026-01-04T12:05:01Z",
        provenance: "Managed provider response; system-verified SHA-256 comparison",
        external_reference_id: "provider-reference-1234",
        provider_verification_url: "https://timestamp.sigstore.dev/api/v1/timestamp",
        note: "Provider response archived by the configured adapter.",
        provider_metadata: Some(&metadata),
    };

    let bytes = generate_external_timestamp_addendum_pdf(&snapshot)
        .expect("generate automatic timestamp addendum");
    let (_, text) = parse_text(&bytes);
    let normalized = normalized_text(&text);
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    for expected in [
        "Original Certificate ID [System value]",
        CERTIFICATE_ID,
        "Finalization snapshot / revision ID [System verification]",
        "finalization-snapshot-2026-0001",
        "Timestamp protocol [Provider-derived metadata]",
        "RFC 3161 Timestamp Protocol",
        "Timestamp adapter [Provider-derived metadata]",
        "sigstore_public_tsa_rfc3161",
        "Request algorithm [Provider-derived metadata]",
        "Provider response archive [System value]",
        "PROVIDER_RESPONSE.bin",
        "Provider response SHA-256 [System verification]",
        DIGEST_B,
        "Timestamp evidence SHA-256 [System verification]",
        DIGEST_C,
        "Provider verification result [System verification] VERIFIED",
        "Verification timestamp [System verification]",
        "2026-01-04T12:05:00Z",
        "Provider response structure valid [System verification] YES",
        "Provider digest match [System verification] YES",
        "RFC 3161 request nonce [System value]",
        "0123456789abcdef",
        "RFC 3161 nonce match [System verification] YES",
        "Requested timestamp policy OID [System value]",
        "Requested policy match [System verification] YES",
        "Cryptographic verifier [System verification]",
        crate::external_timestamp::RFC3161_CRYPTOGRAPHIC_VERIFIER,
        "Trust-anchor SHA-256 [System verification]",
        "Timestamp signature verified [System verification] YES",
        "Timestamp trust chain verified [System verification] YES",
    ] {
        let expected_compact = expected
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            compact.contains(&expected_compact),
            "missing automatic provider value: {expected}"
        );
    }
    assert!(!text.contains("[User-confirmed fact]"));
    assert!(!text.contains("Legacy manually recorded timestamp evidence"));
    assert!(!text.contains("Qualified electronic timestamp"));
    assert!(normalized.contains("do not establish a qualified timestamp, legal effect"));
}

#[test]
fn qualified_timestamp_service_is_highlighted_only_from_a_verified_trust_record() {
    let metadata = TimestampProviderMetadata {
        adapter: "custom_rfc3161".into(),
        protocol: "RFC 3161 Timestamp Protocol".into(),
        request_algorithm: "SHA-256".into(),
        response_format: "RFC 3161 TimeStampResp (.tsr)".into(),
        provider_endpoint_identifier: "https://arbitrary-label.example.test/tsa".into(),
        certificate_subject: "CN=Verified TSA".into(),
        certificate_sha256: DIGEST_A.into(),
        provider_identity_verified: Some(true),
        provider_digest_match: Some(true),
        signature_verified: Some(true),
        trust_chain_verified: Some(true),
        verification_result: ExternalTimestampStatus::Verified,
        qualification: Some(crate::model::TimestampQualificationRecord {
            status: TimestampQualificationStatus::QualifiedServiceVerified,
            provider_identity_status: TimestampQualificationStatus::ProviderIdentityVerified,
            trust_service_status: TimestampQualificationStatus::TrustServiceVerified,
            eidas_qualification_status: TimestampQualificationStatus::QualifiedServiceVerified,
            current_qualification_status: TimestampQualificationStatus::QualifiedServiceVerified,
            qualification_at_timestamp: TimestampQualificationStatus::QualifiedServiceVerified,
            checked_at: "2026-08-21T10:00:00Z".into(),
            message: "Qualified service verified for the timestamp time.".into(),
            identity: crate::model::TimestampServiceIdentity {
                certificate_sha256: DIGEST_A.into(),
                certificate_subject: "CN=Verified TSA".into(),
                policy_oid: "1.2.3.4".into(),
                ..Default::default()
            },
            trusted_list: Some(crate::model::TrustedListEvidence {
                source: "https://official.example.test/member-state-tl.xml".into(),
                territory: "DE".into(),
                version: "6".into(),
                sequence_number: "42".into(),
                issued_at: "2026-08-01T00:00:00Z".into(),
                next_update: "2027-02-01T00:00:00Z".into(),
                sha256: DIGEST_B.into(),
                validation_status: TrustedListValidationStatus::Verified,
                validated_at: "2026-08-21T10:00:00Z".into(),
            }),
            trust_service_provider: "Official TSP identity".into(),
            trust_service_name: "Qualified timestamp service".into(),
            service_type: "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST".into(),
            service_status: "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted".into(),
            current_service_status: "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted"
                .into(),
            service_identifier: "service-42".into(),
            qualification_type: "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST".into(),
            status_valid_from: "2025-01-01T00:00:00Z".into(),
            current_status_valid_from: "2025-01-01T00:00:00Z".into(),
            ..Default::default()
        }),
        verification_message: "Technical timestamp verified.".into(),
        verification_timestamp: "2026-08-21T10:00:00Z".into(),
        ..Default::default()
    };
    let snapshot = ExternalTimestampPdfSnapshot {
        certificate_id: CERTIFICATE_ID,
        provider: "Meine TSA",
        timestamp_type: "External integrity timestamp",
        timestamp_value: "2026-08-18T12:00:00Z",
        referenced_artifact: "EVIDENCE_MANIFEST.json",
        referenced_artifact_path: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referenced_sha256: DIGEST_A,
        actual_sha256: DIGEST_A,
        referenced_hash_match: Some(true),
        evidence_file_name: "TIMESTAMP_EVIDENCE.tsr",
        evidence_sha256: DIGEST_C,
        imported_at: "2026-08-21T10:00:00Z",
        provenance: "Managed provider response",
        external_reference_id: "42",
        provider_verification_url: "https://arbitrary-label.example.test/tsa",
        note: "Fixture",
        provider_metadata: Some(&metadata),
    };

    let bytes = generate_external_timestamp_addendum_pdf(&snapshot)
        .expect("generate qualified timestamp addendum");
    let (_, text) = parse_text(&bytes);
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    for expected in [
        "Provider Trust and Qualification",
        "eIDAS QUALIFIED TRUST SERVICE – VERIFIED",
        "Qualification at timestamp [Independent trust verification] QUALIFIED SERVICE VERIFIED",
        "Current qualification [Independent trust verification] QUALIFIED SERVICE VERIFIED",
        "Service status at timestamp [Independent trust verification] http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted",
        "Current service status [Independent trust verification] http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted",
        "Official TSP identity",
        "Qualified timestamp service",
        "Trusted List validation [System verification] VERIFIED",
        DIGEST_B,
    ] {
        let expected = expected
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains(&expected), "missing qualification value");
    }
}

#[test]
fn open_timestamps_addendum_uses_protocol_aware_pending_labels() {
    let metadata = TimestampProviderMetadata {
        adapter: "open_timestamps".into(),
        protocol: "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade"
            .into(),
        request_algorithm: "SHA-256".into(),
        response_format: "OpenTimestamps DetachedTimestampFile (.ots)".into(),
        provider_endpoint_identifier: "https://a.pool.opentimestamps.org/digest".into(),
        provider_response_file_name: "PROVIDER_RESPONSE.bin".into(),
        provider_response_sha256: DIGEST_B.into(),
        referenced_revision_id: "finalization-snapshot-ots".into(),
        provider_digest_match: Some(true),
        verification_result: ExternalTimestampStatus::Attached,
        verification_message:
            "Detached proof is locally bound; OpenTimestamps verification is pending.".into(),
        verification_timestamp: "2026-08-20T10:58:00Z".into(),
        ..TimestampProviderMetadata::default()
    };
    let snapshot = ExternalTimestampPdfSnapshot {
        certificate_id: CERTIFICATE_ID,
        provider: "OpenTimestamps",
        timestamp_type: "External integrity timestamp",
        timestamp_value: "",
        referenced_artifact: "EVIDENCE_MANIFEST.json",
        referenced_artifact_path: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referenced_sha256: DIGEST_A,
        actual_sha256: DIGEST_A,
        referenced_hash_match: Some(true),
        evidence_file_name: "TIMESTAMP_EVIDENCE.ots",
        evidence_sha256: DIGEST_C,
        imported_at: "2026-08-20T10:58:00Z",
        provenance: "Provider-derived metadata; managed provider response",
        external_reference_id: "",
        provider_verification_url: "https://a.pool.opentimestamps.org/digest",
        note: "OpenTimestamps proof archived; verification or upgrade remains pending.",
        provider_metadata: Some(&metadata),
    };

    let bytes = generate_external_timestamp_addendum_pdf(&snapshot)
        .expect("generate OpenTimestamps addendum");
    validate_pdfa_2b_bytes(&bytes).expect("OpenTimestamps addendum PDF/A-2b structure");
    let (_, text) = parse_text(&bytes);
    let normalized = normalized_text(&text);

    for expected in [
        "Confirmed timestamp [System verification] PENDING — OpenTimestamps proof verification / upgrade required",
        "Calendar endpoint [Provider-derived metadata]",
        "Calendar endpoint identifier [Provider-derived metadata]",
        "OpenTimestamps proof verification [System verification] PENDING — upgrade/verification required",
        "Local manifest / proof binding [System verification] YES",
        "CMS signature / trust chain [System verification] N/A — not RFC 3161",
        "Provider verification result [System verification] ATTACHED",
    ] {
        assert!(
            normalized.contains(expected),
            "missing protocol-aware OpenTimestamps value: {expected}\n{normalized}"
        );
    }
    assert!(!normalized.contains("Provider digest match [System verification]"));
    assert!(!normalized.contains("Requested timestamp policy OID"));
}
