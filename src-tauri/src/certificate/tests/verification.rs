use super::*;

#[test]
fn current_and_previous_pdf_certificate_formats_remain_recognizable() {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let manifest_path = workspace.path().join("manifest.json");

    for version in ["2.0", "3.0", "4.0", "4.1", "5.0", "5.1"] {
        let hashes = BTreeMap::from([(PDF_FILE.into(), DIGEST.into())]);
        fs::write(
            &manifest_path,
            format!("{{\"certificate\":{{\"format_version\":\"{version}\"}}}}\n"),
        )
        .expect("certificate manifest fixture");
        assert!(certificate_format_requires_pdf(&manifest_path, &hashes)
            .expect("supported certificate format"));
    }
    let historical_bilingual_hashes = BTreeMap::from([
        (PDF_FILE.into(), DIGEST.into()),
        (PDF_FILE_DE.into(), DIGEST.into()),
    ]);
    for version in ["5.2", "6.0"] {
        fs::write(
            &manifest_path,
            format!("{{\"certificate\":{{\"format_version\":\"{version}\"}}}}\n"),
        )
        .expect("historical bilingual certificate manifest fixture");
        assert!(
            certificate_format_requires_pdf(&manifest_path, &historical_bilingual_hashes)
                .expect("historical bilingual certificate format")
        );
        assert_eq!(
            certificate_format_requires_pdfa_2b(&manifest_path)
                .expect("historical archive profile requirement"),
            version == "6.0"
        );
    }
    fs::write(
        &manifest_path,
        format!("{{\"certificate\":{{\"format_version\":\"{CERTIFICATE_FORMAT_VERSION}\"}}}}\n"),
    )
    .expect("current certificate manifest fixture");
    let hashes = BTreeMap::from([
        (PDF_FILE.into(), DIGEST.into()),
        (PDF_FILE_DE.into(), DIGEST.into()),
    ]);
    assert!(certificate_format_requires_pdf(&manifest_path, &hashes)
        .expect("current supported certificate format"));
}

struct PublicationFixture {
    manifest: Vec<u8>,
    certificate: Vec<u8>,
    pdf_en: Vec<u8>,
    pdf_de: Vec<u8>,
    certificate_hashes: Vec<u8>,
}

fn publication_fixture(track_root: &Path) -> PublicationFixture {
    let main_hashes = b"fixture main hash manifest\n";
    let main_hash_path = track_root.join(HASH_FILE);
    fs::create_dir_all(
        main_hash_path
            .parent()
            .expect("main hash manifest parent directory"),
    )
    .expect("create documentation fixture directory");
    fs::write(&main_hash_path, main_hashes).expect("write main hash manifest fixture");
    // Publication rollback tests exercise file-transaction mechanics, not
    // the archive renderer. Use the last historical dual-PDF format so the
    // intentionally minimal parseable fixture is not misrepresented as
    // current PDF/A-2b output.
    let manifest = b"{\"certificate\":{\"format_version\":\"5.2\"},\"fixture\":true}\n".to_vec();
    let certificate = b"# Fixture certificate\n".to_vec();
    let mut pdf_document = printpdf::PdfDocument::new("Fixture certificate");
    let pdf = pdf_document
        .with_pages(vec![printpdf::PdfPage::new(
            printpdf::Mm(210.0),
            printpdf::Mm(297.0),
            Vec::new(),
        )])
        .save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
        sha256_bytes(main_hashes),
        HASH_FILE,
        sha256_bytes(&manifest),
        MANIFEST_FILE,
        sha256_bytes(&certificate),
        CERTIFICATE_FILE,
        sha256_bytes(&pdf),
        PDF_FILE,
        sha256_bytes(&pdf),
        PDF_FILE_DE,
    )
    .into_bytes();
    PublicationFixture {
        manifest,
        certificate,
        pdf_en: pdf.clone(),
        pdf_de: pdf,
        certificate_hashes,
    }
}

#[test]
fn historical_5_2_dual_non_pdfa_certificate_remains_verifiable() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let track_root = workspace.path();
    let fixture = publication_fixture(track_root);
    fs::create_dir_all(track_root.join(CERTIFICATE_DIR))
        .expect("create historical certificate directory");
    fs::write(track_root.join(MANIFEST_FILE), fixture.manifest)
        .expect("write historical evidence manifest");
    fs::write(track_root.join(CERTIFICATE_FILE), fixture.certificate)
        .expect("write historical Markdown certificate");
    fs::write(track_root.join(PDF_FILE), fixture.pdf_en)
        .expect("write historical English certificate PDF");
    fs::write(track_root.join(PDF_FILE_DE), fixture.pdf_de)
        .expect("write historical German certificate PDF");
    fs::write(
        track_root.join(CERTIFICATE_HASH_FILE),
        fixture.certificate_hashes,
    )
    .expect("write historical certificate hash list");

    verify(track_root).expect("historical 5.2 dual non-PDF/A certificate must verify");
}

fn assert_injected_publication_failure(failure: CertificatePublicationFailure) {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let track_root = workspace.path();
    let fixture = publication_fixture(track_root);
    let live = track_root.join(CERTIFICATE_DIR);
    fs::create_dir(&live).expect("create empty live certificate directory");
    let correlated_stage = track_root
        .join(".archive")
        .join("certificate-staging")
        .join(failure.stage_id());

    let error = publish_certificate_set_impl(
        track_root,
        CertificateSetBytes {
            manifest: &fixture.manifest,
            certificate: &fixture.certificate,
            pdf_en: &fixture.pdf_en,
            pdf_de: &fixture.pdf_de,
            certificate_hashes: &fixture.certificate_hashes,
        },
        &failure.stage_id(),
        Some(failure),
    )
    .expect_err("injected publication failure");

    assert_eq!(
        error.to_string(),
        format!(
            "Invalid stored data: Injected certificate publication failure at {}.",
            failure.label()
        )
    );
    assert!(
        verify(track_root).is_err(),
        "incomplete live certificate unexpectedly verified after {} failure",
        failure.label()
    );
    assert!(
        live.is_dir(),
        "empty live certificate directory was removed"
    );
    assert!(
        fs::read_dir(&live)
            .expect("read restored certificate directory")
            .next()
            .is_none(),
        "live certificate directory is not empty after {} failure",
        failure.label()
    );
    assert!(
        !track_root.join(PDF_FILE).exists(),
        "live PDF remains after {} failure",
        failure.label()
    );
    assert!(
        !track_root.join(PDF_FILE_DE).exists(),
        "German live PDF remains after {} failure",
        failure.label()
    );
    assert!(
        !correlated_stage.exists(),
        "correlated staging directory was not cleaned after {} failure",
        failure.label()
    );
    let staging_parent = track_root.join(".archive/certificate-staging");
    assert!(
        !staging_parent.exists()
            || fs::read_dir(&staging_parent)
                .expect("read certificate staging directory")
                .next()
                .is_none(),
        "certificate staging contains residue after {} failure",
        failure.label()
    );
}

#[test]
fn staging_directory_creation_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::StagingDirectoryCreate);
}

#[test]
fn manifest_write_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::ManifestWrite);
}

#[test]
fn certificate_write_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::CertificateWrite);
}

#[test]
fn pdf_write_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::PdfWrite);
}

#[test]
fn certificate_hash_write_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::CertificateHashWrite);
}

#[test]
fn certificate_publish_failure_is_controlled_and_cleaned() {
    assert_injected_publication_failure(CertificatePublicationFailure::CertificatePublishRename);
}

#[test]
fn pdf_publish_failure_rolls_back_certificate_and_cleans_staging() {
    assert_injected_publication_failure(CertificatePublicationFailure::PdfPublish);
}

#[test]
fn post_publish_verification_failure_rolls_back_and_cleans_staging() {
    assert_injected_publication_failure(CertificatePublicationFailure::PostPublishVerification);
}

#[test]
fn certificate_hash_parser_requires_exact_complete_unique_set() {
    let legacy =
        format!("{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n");
    assert_eq!(
        parse_certificate_hashes(&legacy)
            .expect("valid legacy set")
            .len(),
        3
    );

    let single_pdf = format!(
        "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
    );
    assert_eq!(
        parse_certificate_hashes(&single_pdf)
            .expect("valid single-PDF set")
            .len(),
        4
    );

    let valid = format!(
        "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n{DIGEST}  {PDF_FILE_DE}\n"
    );
    assert_eq!(
        parse_certificate_hashes(&valid)
            .expect("valid dual-PDF set")
            .len(),
        5
    );

    let duplicate = format!(
        "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n{DIGEST}  {PDF_FILE_DE}\n{DIGEST}  {PDF_FILE_DE}\n"
    );
    assert!(parse_certificate_hashes(&duplicate).is_err());

    let invalid_digest = format!(
        "short  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
    );
    assert!(parse_certificate_hashes(&invalid_digest).is_err());
}

#[test]
fn legacy_certificate_without_pdf_remains_verifiable() {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let track_root = workspace.path();
    let main_hash_path = track_root.join(HASH_FILE);
    fs::create_dir_all(main_hash_path.parent().expect("main hash parent"))
        .expect("documentation directory");
    fs::write(&main_hash_path, b"legacy main hash list\n").expect("main hash fixture");
    let certificate_path = track_root.join(CERTIFICATE_FILE);
    fs::create_dir_all(certificate_path.parent().expect("certificate parent"))
        .expect("certificate directory");
    let manifest_path = track_root.join(MANIFEST_FILE);
    fs::write(&manifest_path, b"{\"certificate\":{}}\n").expect("legacy manifest");
    fs::write(&certificate_path, b"# Legacy certificate\n").expect("legacy certificate");
    let hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n",
        sha256_file(&main_hash_path).expect("main hash digest"),
        HASH_FILE,
        sha256_file(&manifest_path).expect("manifest digest"),
        MANIFEST_FILE,
        sha256_file(&certificate_path).expect("certificate digest"),
        CERTIFICATE_FILE,
    );
    fs::write(track_root.join(CERTIFICATE_HASH_FILE), hashes).expect("legacy hash set");

    verify(track_root).expect("legacy certificate remains valid");
    assert!(!expects_pdf(track_root).expect("legacy format detection"));
}

#[test]
fn evidence_manifest_hash_parser_rejects_duplicates_and_exclusions() {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let sums = workspace.path().join("SHA256SUMS.txt");
    fs::write(
        &sums,
        format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
    )
    .expect("write duplicate sums");
    assert!(parse_hashes(&sums).is_err());

    fs::write(&sums, format!("{DIGEST}  06_CERTIFICATE/hidden.txt\n"))
        .expect("write excluded sums");
    assert!(parse_hashes(&sums).is_err());

    fs::write(&sums, format!("{DIGEST}  {PDF_FILE}\n")).expect("write excluded root PDF");
    assert!(parse_hashes(&sums).is_err());
}

#[test]
fn certificate_hash_parser_rejects_empty_missing_extra_and_unsafe_entries() {
    let invalid_sets = [
        String::new(),
        format!("{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n"),
        format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {CERTIFICATE_HASH_FILE}\n"
        ),
        format!(
            "{DIGEST}  {HASH_FILE}\n\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
        ),
        format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  ../EVIDENCE_MANIFEST.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
        ),
        format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  /absolute.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
        ),
        format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  06_CERTIFICATE/control\tmanifest.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
        ),
    ];

    for content in invalid_sets {
        assert!(
            parse_certificate_hashes(&content).is_err(),
            "invalid certificate set was accepted: {content:?}"
        );
    }
}

#[test]
fn evidence_manifest_hash_parser_covers_format_and_path_edge_cases() {
    let invalid_entries = [
        String::new(),
        "not-a-digest  01_RELEASE/song.wav\n".into(),
        format!("{DIGEST}  /absolute.wav\n"),
        format!("{DIGEST}  ../escape.wav\n"),
        format!("{DIGEST}  01_RELEASE\\windows.wav\n"),
        format!("{DIGEST}  01_RELEASE/control\tname.wav\n"),
        format!("{DIGEST}  {HASH_FILE}\n"),
        format!("{DIGEST}  .archive/hidden.wav\n"),
        format!("{DIGEST}  .summary/hidden.wav\n"),
        format!("{DIGEST}  .suno-doc/workspace.sqlite\n"),
        format!("{DIGEST}  {CERTIFICATE_FILE}\n"),
        format!("{DIGEST}  01_RELEASE/song.wav\n\n{DIGEST}  02_SUNO/song.wav\n"),
        format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
    ];

    for content in invalid_entries {
        assert!(
            parse_main_hash_fixture(&content).is_err(),
            "invalid main hash entry was accepted: {content:?}"
        );
    }

    let uppercase = DIGEST.to_ascii_uppercase();
    let parsed = parse_main_hash_fixture(&format!(
        "{uppercase}  01_RELEASE/song.wav\n{DIGEST}  02_SUNO/source.wav\n"
    ))
    .expect("portable valid hash list");
    assert_eq!(parsed["01_RELEASE/song.wav"], DIGEST);
    assert_eq!(parsed.len(), 2);
}
