use super::*;

struct CertificateCrossCheckContext {
    finalized: TrackDetail,
    stored_track: TrackRecord,
    stored_evidence: Vec<EvidenceItem>,
    stored_steps: Vec<StepState>,
    stored_deviations: Vec<BlockingDeviation>,
    track_root: PathBuf,
    sums: BTreeMap<String, String>,
    manifest: serde_json::Value,
    certificate: ParsedCertificate,
    pdf_text: String,
    compact_pdf_text: String,
    certificate_hashes: BTreeMap<String, String>,
}

#[test]
fn finalized_certificate_fields_cross_check_sqlite_track_evidence_hashes_and_manifest() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = prepare_certificate_cross_check_track(&app, directory.path());
    let context = load_certificate_cross_check_context(&app, finalized);
    assert_certificate_identity_and_profile(&context);
    let release = assert_certificate_evidence_and_artwork_hashes(&context);
    assert_certificate_file_hashes_and_deviations(&context, release);
    assert_certificate_workflow_steps(&context);
}

fn prepare_certificate_cross_check_track(app: &WorkspaceApp, directory: &Path) -> TrackDetail {
    let created = app
        .create_track(CreateTrackInput {
            title: "Certificate Field Cross Check".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track creation");
    let updated = app
        .update_track(
            &created.id,
            TrackPatch {
                production_end_date: Some("2026-08-02".into()),
                suno_model: Some("v4.5".into()),
                suno_project_url: Some("https://suno.com/song/certificate-cross-check".into()),
                suno_project_version_id: Some("project-cross-check-v1".into()),
                suno_final_generation_id: Some("generation-cross-check".into()),
                suno_final_generation_date: Some("2026-08-02".into()),
                suno_final_generation_time: Some("09:10".into()),
                suno_download_export_date: Some("2026-08-03".into()),
                suno_plan_at_generation: Some("Pro".into()),
                final_export_date: Some("2026-08-03".into()),
                instrumental_track: Some(true),
                vocal_lyrics_present: Some(false),
                vocal_intent: Some(VocalIntent::Instrumental),
                suno_content_classification: Some(SunoContentClassification::Empty),
                suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                external_audio_uploaded: Some(false),
                own_audio_uploaded: Some(false),
                code_based_generation: Some(false),
                third_party_samples_uploaded: Some(false),
                human_editing_performed: Some(false),
                post_export_editing_performed: Some(false),
                commercial_use_intended: Some(false),
                generative_ai_used: Some(false),
                artwork_origin: Some("human".into()),
                depicts_real_person: Some(false),
                depicts_real_event: Some(false),
                contains_trademark: Some(false),
                ..TrackPatch::default()
            },
        )
        .expect("track facts");

    let fixture_root = directory.join("fixtures");
    fs::create_dir(&fixture_root).expect("fixture directory");
    let suno_export = fixture_root.join("cross-check-suno.wav");
    let release_master = fixture_root.join("cross-check-release.wav");
    let final_artwork = fixture_root.join("cross-check-final.png");
    fs::write(&suno_export, p0_screening_wav(None, 41)).expect("Suno export fixture");
    fs::write(&release_master, p0_screening_wav(None, 47)).expect("release fixture");
    image::RgbaImage::from_pixel(64, 64, image::Rgba([32, 64, 96, 255]))
        .save(&final_artwork)
        .expect("final artwork fixture");
    app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
        .expect("Suno evidence import");
    app.import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
        .expect("release evidence import");
    app.import_evidence_from(&updated.id, EvidenceRole::FinalArtwork, &final_artwork)
        .expect("final artwork import");
    app.update_track(
        &updated.id,
        TrackPatch {
            release_filename_difference_confirmed: Some(true),
            suno_export_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("explicit filename difference confirmations");

    let deviation_detail = app
        .add_deviation(
            &updated.id,
            DeviationInput {
                description: "Resolved certificate cross-check note".into(),
                blocking: true,
            },
        )
        .expect("blocking deviation fixture");
    let deviation_id = deviation_detail
        .blocking_deviations
        .iter()
        .find(|deviation| deviation.blocking && !deviation.resolved)
        .expect("unresolved fixture deviation")
        .id
        .clone();
    app.resolve_deviation(&updated.id, &deviation_id)
        .expect("resolved deviation fixture");

    app.generate_documents(&updated.id, false)
        .expect("document generation");
    app.calculate_hashes(&updated.id)
        .expect("SHA256SUMS generation");
    let validation = app.validate_track(&updated.id).expect("native gate");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
    let finalized = app
        .finalize_track(&updated.id)
        .expect("real finalization")
        .track
        .expect("finalized detail");
    assert_eq!(finalized.status, TrackStatus::Finalized);
    finalized
}

fn load_certificate_cross_check_context(
    app: &WorkspaceApp,
    finalized: TrackDetail,
) -> CertificateCrossCheckContext {
    let updated = &finalized;
    let stored_track = app
        .persistence
        .track(&updated.id)
        .expect("SQLite track record");
    let stored_evidence = app
        .persistence
        .evidence(&updated.id)
        .expect("SQLite evidence records");
    let stored_steps = app
        .persistence
        .stored_steps(&updated.id)
        .expect("SQLite step states");
    let stored_deviations = app
        .persistence
        .deviations(&updated.id)
        .expect("SQLite deviations");
    let track_root = app.root().join(&stored_track.relative_path);
    let sums_bytes = fs::read(track_root.join(integrity::HASH_FILE)).expect("SHA256SUMS bytes");
    let sums =
        parse_sha256sums(std::str::from_utf8(&sums_bytes).expect("SHA256SUMS must be UTF-8"));
    let manifest_bytes =
        fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("evidence manifest bytes");
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).expect("evidence manifest JSON");
    let certificate_text = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
        .expect("certificate document");
    let certificate = parse_certificate_document(&certificate_text);
    let pdf_bytes = fs::read(track_root.join(certificate::PDF_FILE)).expect("technical PDF bytes");
    let mut pdf_warnings = Vec::new();
    let parsed_pdf = printpdf::PdfDocument::parse(
        &pdf_bytes,
        &printpdf::PdfParseOptions::default(),
        &mut pdf_warnings,
    )
    .expect("parse technical PDF");
    let pdf_text = parsed_pdf
        .extract_text()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let compact_pdf_text = pdf_text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let certificate_hashes = parse_sha256sums(
        &fs::read_to_string(track_root.join(certificate::CERTIFICATE_HASH_FILE))
            .expect("certificate hash set"),
    );
    CertificateCrossCheckContext {
        finalized,
        stored_track,
        stored_evidence,
        stored_steps,
        stored_deviations,
        track_root,
        sums,
        manifest,
        certificate,
        pdf_text,
        compact_pdf_text,
        certificate_hashes,
    }
}

fn assert_certificate_identity_and_profile(context: &CertificateCrossCheckContext) {
    let finalized = &context.finalized;
    let stored_track = &context.stored_track;
    let manifest = &context.manifest;
    let certificate = &context.certificate;
    let pdf_text = &context.pdf_text;
    let compact_pdf_text = &context.compact_pdf_text;
    let certificate_id = stored_track
        .certificate
        .certificate_id
        .as_deref()
        .expect("stored certificate ID");
    let finalized_at = stored_track
        .certificate
        .finalized_at
        .as_deref()
        .expect("stored finalization timestamp");
    assert_eq!(
        finalized.certificate.certificate_id.as_deref(),
        Some(certificate_id)
    );
    assert_eq!(
        finalized.certificate.finalized_at.as_deref(),
        Some(finalized_at)
    );
    assert_eq!(manifest_string(manifest, "/certificate/id"), certificate_id);
    assert_eq!(certificate.fields["Certificate ID"], certificate_id);
    assert!(compact_pdf_text.contains(certificate_id));
    assert_eq!(
        manifest_string(manifest, "/certificate/format_version"),
        certificate::CERTIFICATE_FORMAT_VERSION
    );
    assert_eq!(
        certificate.fields["Certificate schema"],
        certificate::CERTIFICATE_FORMAT_VERSION
    );

    assert_eq!(finalized.title, stored_track.fields.title);
    assert_eq!(
        manifest_string(manifest, "/track/title"),
        stored_track.fields.title
    );
    assert_eq!(
        certificate.fields["Documented title [User-confirmed fact]"],
        stored_track.fields.title
    );
    assert!(pdf_text.contains(&stored_track.fields.title));
    assert_eq!(
        manifest_string(manifest, "/artist/name"),
        stored_track.profile_snapshot.artist_name
    );
    assert_eq!(
        certificate.fields["Artist [User-confirmed fact]"],
        stored_track.profile_snapshot.artist_name
    );
    assert_eq!(
        manifest_string(manifest, "/workflow/id"),
        stored_track.workflow_id
    );
    assert_eq!(
        manifest_string(manifest, "/workflow/version"),
        stored_track.workflow_version
    );
    assert_eq!(
        certificate.fields["Workflow"],
        format!(
            "{} / {}",
            stored_track.workflow_id, stored_track.workflow_version
        )
    );
    assert_eq!(
        manifest_string(manifest, "/workflow/application_version"),
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        certificate.fields["Application version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        manifest_string(manifest, "/finalization/timestamp"),
        finalized_at
    );
    assert_eq!(certificate.fields["Finalized at"], finalized_at);
}

fn assert_certificate_evidence_and_artwork_hashes(
    context: &CertificateCrossCheckContext,
) -> &EvidenceItem {
    let stored_evidence = &context.stored_evidence;
    let sums = &context.sums;
    let manifest = &context.manifest;
    let certificate = &context.certificate;
    let manifest_evidence = manifest["evidence"]
        .as_array()
        .expect("manifest evidence array");
    let verified_evidence = stored_evidence
        .iter()
        .filter(|item| item.verified && item.sha256.is_some() && item.verification_error.is_none())
        .collect::<Vec<_>>();
    assert_eq!(manifest_evidence.len(), verified_evidence.len());
    assert_eq!(
        certificate.fields["Evidence file count"],
        verified_evidence.len().to_string()
    );
    let manifest_evidence_by_id = manifest_evidence
        .iter()
        .map(|item| {
            let id = item["id"].as_str().expect("manifest evidence ID");
            (id, item)
        })
        .collect::<BTreeMap<_, _>>();
    for evidence in &verified_evidence {
        let manifest_item = manifest_evidence_by_id
            .get(evidence.id.as_str())
            .unwrap_or_else(|| panic!("manifest evidence missing: {}", evidence.id));
        assert_eq!(manifest_item["role"].as_str(), Some(evidence.role.as_str()));
        assert_eq!(
            manifest_item["relativePath"].as_str(),
            Some(evidence.relative_path.as_str())
        );
        assert_eq!(manifest_item["sha256"].as_str(), evidence.sha256.as_deref());
    }

    let release = verified_evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("stored release WAV");
    let artwork = verified_evidence
        .iter()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .expect("stored final artwork");
    assert_eq!(
        sums.get(&release.relative_path),
        release.sha256.as_ref(),
        "release WAV hash must agree between SHA256SUMS and SQLite"
    );
    assert_eq!(
        sums.get(&artwork.relative_path),
        artwork.sha256.as_ref(),
        "final artwork hash must agree between SHA256SUMS and SQLite"
    );
    assert_eq!(
        manifest["hashes"]
            .get(&release.relative_path)
            .and_then(serde_json::Value::as_str),
        release.sha256.as_deref()
    );
    assert_eq!(
        manifest["hashes"]
            .get(&artwork.relative_path)
            .and_then(serde_json::Value::as_str),
        artwork.sha256.as_deref()
    );
    assert_eq!(
        certificate.fields["Release audio SHA-256"],
        release.sha256.as_deref().unwrap()
    );
    assert_eq!(
        certificate.fields["Final artwork SHA-256"],
        artwork.sha256.as_deref().unwrap()
    );
    release
}

fn assert_certificate_file_hashes_and_deviations(
    context: &CertificateCrossCheckContext,
    release: &EvidenceItem,
) {
    let track_root = &context.track_root;
    let sums = &context.sums;
    let manifest = &context.manifest;
    let certificate = &context.certificate;
    let compact_pdf_text = &context.compact_pdf_text;
    let certificate_hashes = &context.certificate_hashes;
    let stored_deviations = &context.stored_deviations;
    let sums_sha =
        sha256_file(&track_root.join(integrity::HASH_FILE)).expect("SHA256SUMS file hash");
    let manifest_sha = sha256_file(&track_root.join(certificate::MANIFEST_FILE))
        .expect("evidence manifest file hash");
    assert_eq!(
        manifest_string(manifest, "/certificate/sha256sums_sha256"),
        sums_sha
    );
    assert_eq!(certificate.fields["SHA256SUMS.txt SHA-256"], sums_sha);
    assert_eq!(
        certificate.fields["Evidence manifest SHA-256"],
        manifest_sha
    );
    assert_eq!(certificate_hashes.len(), 5);
    assert_eq!(
        certificate_hashes.get(certificate::PDF_FILE),
        Some(&sha256_file(&track_root.join(certificate::PDF_FILE)).expect("technical PDF hash"))
    );
    assert_eq!(
        certificate_hashes.get(certificate::PDF_FILE_DE),
        Some(
            &sha256_file(&track_root.join(certificate::PDF_FILE_DE))
                .expect("German technical PDF hash")
        )
    );
    assert!(compact_pdf_text.contains(release.sha256.as_deref().unwrap()));
    assert_eq!(
        manifest["hashes"],
        serde_json::to_value(sums).expect("serialize parsed SHA256SUMS")
    );

    assert_eq!(
        manifest["deviations"],
        serde_json::to_value(stored_deviations).unwrap()
    );
    let open_blocking = stored_deviations
        .iter()
        .filter(|deviation| deviation.blocking && !deviation.resolved)
        .count();
    assert_eq!(
        certificate.fields["Blocking deviations"],
        open_blocking.to_string()
    );
}

fn assert_certificate_workflow_steps(context: &CertificateCrossCheckContext) {
    let manifest = &context.manifest;
    let certificate = &context.certificate;
    let stored_steps = &context.stored_steps;
    let manifest_steps = manifest["steps"].as_array().expect("manifest steps");
    let manifest_na_reasons = manifest_steps
        .iter()
        .filter(|step| step["status"].as_str() == Some("N_A"))
        .map(|step| {
            (
                step["id"].as_str().expect("N/A step ID").to_owned(),
                step["naReason"].as_str().expect("N/A reason").to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(manifest_na_reasons, certificate.na_reasons);
    assert!(certificate.na_reasons.is_empty());
    assert!(stored_steps
        .iter()
        .all(|step| step.status != StepStatus::NotApplicable));
    let manifest_completed = manifest_steps
        .iter()
        .filter(|step| matches!(step["status"].as_str(), Some("PASS" | "N_A")))
        .map(|step| {
            let id = step["id"].as_str().expect("completed step ID").to_owned();
            let status = match step["status"].as_str().expect("completed status") {
                "PASS" => "PASS",
                "N_A" => "N/A",
                status => panic!("unexpected completed status: {status}"),
            };
            (id, status.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(certificate.completed_steps, manifest_completed);
}
#[test]
fn global_subscription_evidence_requires_pdf_signature_and_relevant_dates() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Commercial Coverage".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: true,
            library: TrackLibraryPlacement::default(),
        })
        .expect("commercial track");
    app.update_track(
        &track.id,
        TrackPatch {
            production_end_date: Some("2026-08-03".into()),
            ..TrackPatch::default()
        },
    )
    .expect("production range");

    let fixtures = directory.path().join("fixtures");
    fs::create_dir(&fixtures).expect("fixture directory");
    let disguised_pdf = fixtures.join("disguised.pdf");
    fs::write(&disguised_pdf, b"plain text with a PDF extension").expect("disguised PDF fixture");
    assert!(matches!(
        app.register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            &disguised_pdf,
            Some("2026-08-01".into()),
            Some("2026-08-31".into()),
        ),
        Err(AppError::Validation(_))
    ));

    let narrow_pdf = fixtures.join("narrow-subscription.pdf");
    fs::write(&narrow_pdf, b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n")
        .expect("narrow subscription PDF");
    let narrow = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            &narrow_pdf,
            Some("2026-08-02".into()),
            Some("2026-08-31".into()),
        )
        .expect("valid global PDF with narrow coverage");
    app.attach_global_evidence(&track.id, &narrow.evidence.id)
        .expect("partially overlapping subscription may be combined with another receipt");

    let irrelevant_pdf = fixtures.join("irrelevant-subscription.pdf");
    fs::write(&irrelevant_pdf, b"%PDF-1.7\nirrelevant\n%%EOF\n")
        .expect("irrelevant subscription PDF");
    let irrelevant = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            &irrelevant_pdf,
            Some("2026-09-01".into()),
            Some("2026-09-30".into()),
        )
        .expect("valid but irrelevant global evidence");
    assert!(matches!(
        app.attach_global_evidence(&track.id, &irrelevant.evidence.id),
        Err(AppError::Validation(_))
    ));

    let covering_pdf = fixtures.join("covering-subscription.pdf");
    let covering_bytes = b"%PDF-1.7\n1 0 obj\n<</Type /Receipt>>\nendobj\n%%EOF\n";
    fs::write(&covering_pdf, covering_bytes).expect("covering subscription PDF");
    let covering = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            &covering_pdf,
            Some("2026-07-01".into()),
            Some("2026-08-31".into()),
        )
        .expect("covering global evidence");
    let attached = app
        .attach_global_evidence(&track.id, &covering.evidence.id)
        .expect("portable track copy");
    let portable = attached
        .evidence
        .iter()
        .find(|item| {
            item.source_global_evidence_id.as_deref() == Some(covering.evidence.id.as_str())
        })
        .expect("subscription evidence attached");
    assert!(portable.verified);
    assert_eq!(
        portable.source_global_evidence_id.as_deref(),
        Some(covering.evidence.id.as_str())
    );
    assert_eq!(portable.coverage_start.as_deref(), Some("2026-07-01"));
    assert_eq!(portable.coverage_end.as_deref(), Some("2026-08-31"));
    assert!(portable
        .relative_path
        .starts_with("04_LICENSES/subscription_"));
    assert_eq!(
        fs::read(
            app.root()
                .join(&attached.relative_path)
                .join(&portable.relative_path)
        )
        .expect("portable evidence bytes"),
        covering_bytes
    );
    assert_eq!(portable.sha256, covering.evidence.sha256);
}
