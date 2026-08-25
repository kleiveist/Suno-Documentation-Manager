use super::*;

fn generate_current_pdfa_fixture(track_root: &Path) -> serde_json::Value {
    let mut resolver = |_digest: &str, _bytes: &[u8]| FinalizationTimestampSnapshot::default();
    generate_current_pdfa_fixture_with_resolver(track_root, &mut resolver)
}

fn generate_current_pdfa_fixture_with_resolver(
    track_root: &Path,
    timestamp_resolver: &mut dyn FnMut(&str, &[u8]) -> FinalizationTimestampSnapshot,
) -> serde_json::Value {
    let hash_manifest = track_root.join(HASH_FILE);
    fs::create_dir_all(hash_manifest.parent().expect("hash manifest parent"))
        .expect("create hash manifest parent");
    fs::write(
        &hash_manifest,
        format!("{DIGEST}  02_SUNO/semantic-fixture.txt\n"),
    )
    .expect("write hash manifest fixture");

    let config = crate::workflow::config().expect("workflow config");
    let steps = config
        .steps
        .iter()
        .map(|step| StepState {
            id: step.id.clone(),
            status: StepStatus::Pass,
            na_reason: None,
            updated_at: Some("2026-08-20T10:00:00Z".into()),
        })
        .collect::<Vec<_>>();
    let profile = Profile {
        artist_name: "Semantic Fixture Artist".into(),
        ..Profile::default()
    };
    let track = TrackRecord {
        id: "semantic-manifest-fixture".into(),
        relative_path: "semantic-manifest-fixture".into(),
        status: crate::model::TrackStatus::Finalized,
        workflow_id: config.id,
        workflow_version: config.version,
        profile_snapshot: profile.clone(),
        library: Default::default(),
        field_origins: Default::default(),
        fields: TrackFields {
            title: "Semantic Manifest Fixture".into(),
            vocal_lyrics_present: Some(false),
            vocal_intent: Some(VocalIntent::Vocal),
            suno_content_classification: Some(SunoContentClassification::Mixed),
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: "[Verse]\nSing this line\n[Drop]".into(),
            ..TrackFields::default()
        },
        audio_screening: Default::default(),
        documents: Default::default(),
        integrity: Default::default(),
        certificate: Default::default(),
        created_at: "2026-08-20T09:00:00Z".into(),
        updated_at: "2026-08-20T10:00:00Z".into(),
        legacy: false,
    };

    generate_with_finalization_timestamp(
        GenerationInput {
            track_root,
            track: &track,
            profile: &profile,
            steps: &steps,
            evidence: &[],
            deviations: &[],
            certificate_id: "semantic-manifest-certificate",
            finalized_at: "2026-08-20T10:00:00Z",
            transaction_id: "semantic-manifest-transaction",
            render_options: CertificateRenderOptions::default(),
        },
        timestamp_resolver,
    )
    .expect("generate semantic manifest fixture");

    serde_json::from_slice(
        &fs::read(track_root.join(MANIFEST_FILE)).expect("read evidence manifest"),
    )
    .expect("parse evidence manifest")
}

#[test]
fn finalization_resolves_timestamp_once_before_certificate_rendering() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let mut calls = 0_u32;
    let mut resolver = |digest: &str, bytes: &[u8]| {
        calls += 1;
        assert_eq!(sha256_bytes(bytes), digest);
        FinalizationTimestampSnapshot {
            provider: "Certificate identity fixture".into(),
            provider_configuration_status: TimestampProviderConfigurationStatus::Ready,
            provider_configuration_message: "Provider ready.".into(),
            automatic_request_enabled: true,
            technical_status: ExternalTimestampStatus::Verified,
            technical_message: "Concrete RFC 3161 timestamp verified.".into(),
            timestamp_value: "2026-08-20T10:00:01Z".into(),
            external_reference_id: "fixture-serial".into(),
            provider_verification_url: "https://endpoint.example.invalid".into(),
            provider_metadata: Some(crate::model::TimestampProviderMetadata {
                protocol: "RFC 3161 Timestamp Protocol".into(),
                request_algorithm: "SHA-256".into(),
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
                certificate_sha256: DIGEST.into(),
                verification_result: ExternalTimestampStatus::Verified,
                verification_message: "Technically verified.".into(),
                verification_timestamp: "2026-08-20T10:00:02Z".into(),
                qualification: Some(crate::model::TimestampQualificationRecord {
                    status: TimestampQualificationStatus::ProviderIdentityVerified,
                    provider_identity_status:
                        TimestampQualificationStatus::ProviderIdentityVerified,
                    trust_service_status: TimestampQualificationStatus::NotVerified,
                    eidas_qualification_status: TimestampQualificationStatus::NotVerified,
                    current_qualification_status: TimestampQualificationStatus::NotVerified,
                    qualification_at_timestamp: TimestampQualificationStatus::NotVerified,
                    checked_at: "2026-08-20T10:00:03Z".into(),
                    message: "No qualified-service evidence was verified.".into(),
                    identity: crate::model::TimestampServiceIdentity {
                        certificate_sha256: DIGEST.into(),
                        certificate_subject: "CN=Fixture TSA".into(),
                        certificate_issuer: "CN=Fixture Root".into(),
                        certificate_serial_number: "42".into(),
                        policy_oid: "1.2.3.4".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    };

    let manifest = generate_current_pdfa_fixture_with_resolver(workspace.path(), &mut resolver);
    assert_eq!(calls, 1, "timestamp resolver must run exactly once");
    assert_eq!(
        manifest["external_timestamp"]["attempt_timing"],
        "after_manifest_anchor_before_single_certificate_render"
    );
    assert_eq!(
        manifest["external_timestamp"]["status_recorded_in_final_certificate"],
        true
    );
    let markdown = fs::read_to_string(workspace.path().join(CERTIFICATE_FILE))
        .expect("read final Markdown certificate");
    assert!(markdown.contains("Concrete timestamp status [System verification]: **VERIFIED**"));
    assert!(markdown
        .contains("Timestamp protocol [Provider-derived metadata]: RFC 3161 Timestamp Protocol"));
    assert!(
        markdown.contains("eIDAS qualification [Independent trust verification]: **NOT VERIFIED**")
    );
    assert!(!markdown.contains("eIDAS QUALIFIED TRUST SERVICE – VERIFIED"));
    assert!(!markdown
        .contains("External timestamp evidence at technical finalization: **NOT RECORDED**"));
    verify(workspace.path()).expect("single-render certificate set verifies");
}

#[test]
fn manifest_semantic_snapshot_keeps_canonical_suno_tokens_and_audio_outcome_independent() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let manifest = generate_current_pdfa_fixture(workspace.path());
    let semantic = &manifest["semantic_snapshot"]["suno_lyrics_structure"];

    assert_eq!(semantic["content_classification"], "MIXED");
    assert_eq!(semantic["vocal_intent"], "VOCAL");
    assert_eq!(semantic["final_audio_contains_vocals"], "NO");
}

#[test]
fn current_pdfa_certificate_rejects_missing_xmp_after_consistent_rehash() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let track_root = workspace.path();
    let manifest = generate_current_pdfa_fixture(track_root);
    assert_eq!(
        manifest["certificate"]["format_version"],
        CERTIFICATE_FORMAT_VERSION
    );
    assert_eq!(
        manifest["certificate"]["pdf_archive"]["archive_format"],
        "PDF/A-2b"
    );
    verify(track_root).expect("current PDF/A-2b certificate set must verify");

    let pdf_path = track_root.join(PDF_FILE);
    let mut archive = lopdf::Document::load_mem(
        &fs::read(&pdf_path).expect("read current English certificate PDF"),
    )
    .expect("parse current English certificate PDF");
    let root_id = archive
        .trailer
        .get(b"Root")
        .and_then(lopdf::Object::as_reference)
        .expect("catalog reference");
    archive
        .get_dictionary_mut(root_id)
        .expect("catalog")
        .remove(b"Metadata");
    let mut tampered_pdf = Vec::new();
    archive
        .save_to(&mut tampered_pdf)
        .expect("serialize PDF without XMP");
    fs::write(&pdf_path, &tampered_pdf).expect("replace PDF with XMP-free fixture");

    let certificate_hash_path = track_root.join(CERTIFICATE_HASH_FILE);
    let current_hashes =
        fs::read_to_string(&certificate_hash_path).expect("read certificate hash list");
    let tampered_sha256 = sha256_bytes(&tampered_pdf);
    // Re-hash the modified PDF deliberately. Verification must therefore
    // fail on the current-format PDF/A contract, not on the ordinary byte hash.
    let rewritten_hashes = current_hashes
        .lines()
        .map(|line| {
            if line.ends_with(&format!("  {PDF_FILE}")) {
                format!("{tampered_sha256}  {PDF_FILE}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&certificate_hash_path, rewritten_hashes)
        .expect("rewrite consistent certificate hash list");

    let error = verify(track_root)
        .expect_err("current certificate without PDF/A XMP must fail after consistent rehash");
    assert!(
        error.to_string().contains("no XMP metadata"),
        "unexpected verification error: {error}"
    );
}
