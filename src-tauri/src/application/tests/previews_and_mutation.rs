use super::*;

#[test]
fn evidence_preview_embeds_images_and_source_text_but_does_not_load_zip_archives() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Evidence Preview".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let fixtures = directory.path().join("fixtures");
    fs::create_dir(&fixtures).expect("fixtures");
    let screenshot = fixtures.join("screenshot.png");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([12, 24, 48, 255]))
        .save(&screenshot)
        .expect("screenshot fixture");
    let project = fixtures.join("project.zip");
    fs::write(&project, b"PK\x03\x04project fixture").expect("ZIP fixture");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoScreenshot, &screenshot)
        .expect("screenshot import");
    let screenshot_item = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoScreenshot)
        .expect("screenshot evidence");
    let image_preview = app
        .preview_evidence(&track.id, &screenshot_item.id)
        .expect("image preview");
    assert!(image_preview
        .data_url
        .as_deref()
        .is_some_and(|value| value.starts_with("data:image/png;base64,")));

    let source_code = fixtures.join("generator.py");
    fs::write(&source_code, b"def generate():\n    return 'sound'\n").expect("source-code fixture");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SourceCodeFile, &source_code)
        .expect("source-code import");
    let source_code_item = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SourceCodeFile)
        .expect("source-code evidence");
    let source_code_preview = app
        .preview_evidence(&track.id, &source_code_item.id)
        .expect("source-code preview");
    assert_eq!(
        source_code_preview.text_content.as_deref(),
        Some("def generate():\n    return 'sound'\n")
    );

    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoProjectZip, &project)
        .expect("ZIP import");
    let project_item = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoProjectZip)
        .expect("ZIP evidence");
    let zip_preview = app
        .preview_evidence(&track.id, &project_item.id)
        .expect("ZIP preview metadata");
    assert!(zip_preview.data_url.is_none());
    assert!(zip_preview.text_content.is_none());
    assert!(zip_preview
        .message
        .as_deref()
        .is_some_and(|message| message.contains("nicht entpackt")));

    let replacement_root = directory.path().join("replacement");
    fs::create_dir(&replacement_root).expect("replacement root");
    let replacement_project = replacement_root.join("project.zip");
    fs::write(&replacement_project, b"PK\x03\x04replacement project")
        .expect("replacement ZIP fixture");
    let replaced = app
        .replace_evidence_from(
            &track.id,
            &project_item.id,
            EvidenceRole::SunoProjectZip,
            &replacement_project,
        )
        .expect("same-path database replacement");
    let projects = replaced
        .evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SunoProjectZip)
        .collect::<Vec<_>>();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project_item.id);
    assert_eq!(projects[0].relative_path, project_item.relative_path);
}

#[test]
fn track_cover_uses_a_bounded_centered_final_artwork_thumbnail() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Centered Cover".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    assert!(app.track_cover(&track.id).expect("empty cover").is_none());

    let fixture = directory.path().join("wide-final.png");
    image::RgbaImage::from_fn(600, 200, |x, _| {
        if x < 200 {
            image::Rgba([180, 30, 40, 255])
        } else if x < 400 {
            image::Rgba([30, 180, 70, 255])
        } else {
            image::Rgba([30, 60, 180, 255])
        }
    })
    .save(&fixture)
    .expect("wide final-artwork fixture");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &fixture)
        .expect("final artwork import");
    let evidence_id = imported.cover_evidence_id.expect("cover evidence ID");
    assert_eq!(
        app.list_tracks()
            .expect("track summaries")
            .into_iter()
            .find(|item| item.id == track.id)
            .and_then(|item| item.cover_evidence_id),
        Some(evidence_id.clone())
    );

    let preview = app
        .track_cover(&track.id)
        .expect("track cover")
        .expect("present track cover");
    assert_eq!(preview.evidence_id, evidence_id);
    let encoded = preview
        .data_url
        .strip_prefix("data:image/png;base64,")
        .expect("PNG data URL");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("base64 thumbnail");
    let thumbnail = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .expect("decode thumbnail")
        .to_rgba8();
    assert_eq!(
        thumbnail.dimensions(),
        (
            crate::artwork::COVER_PREVIEW_SIZE,
            crate::artwork::COVER_PREVIEW_SIZE
        )
    );
    assert_eq!(
        *thumbnail.get_pixel(96, 96),
        image::Rgba([30, 180, 70, 255])
    );
}

#[test]
fn track_creation_rejects_path_like_titles_without_writing_folders() {
    let directory = tempdir().expect("tempdir");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    for title in ["../escape", "/absolute", r"..\\escape", ".draft"] {
        assert!(app
            .create_track(CreateTrackInput {
                title: title.into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .is_err());
    }
    assert!(!directory.path().join("escape").exists());
    assert!(!workspace.join("absolute").exists());
}

#[test]
fn track_update_rejects_path_like_titles_without_changing_stored_identity() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Stable Track Title".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track creation");
    for title in ["../x", r"a\b", "...", ".draft", "🚀"] {
        assert!(matches!(
            app.update_track(
                &track.id,
                TrackPatch {
                    title: Some(title.into()),
                    ..TrackPatch::default()
                },
            ),
            Err(AppError::Validation(_))
        ));
        let unchanged = app.load_track(&track.id).expect("unchanged track");
        assert_eq!(unchanged.title, "Stable Track Title");
        assert_eq!(unchanged.relative_path, track.relative_path);
    }
}

#[test]
fn finalized_track_rejects_mutation_until_revision_archives_snapshot() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let detail = app
        .create_track(CreateTrackInput {
            title: "Finalized Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let track_root = app.root().join(&detail.relative_path);
    fs::write(
        track_root.join("01_RELEASE/revision-fixture.wav"),
        b"revision fixture",
    )
    .expect("hashable revision fixture");
    integrity::calculate(&track_root).expect("valid main hash fixture");
    fs::write(
        track_root.join(certificate::CERTIFICATE_FILE),
        b"certificate",
    )
    .expect("certificate fixture");
    fs::write(track_root.join(certificate::MANIFEST_FILE), b"{}\n").expect("manifest fixture");
    let mut pdf_document = printpdf::PdfDocument::new("Revision fixture");
    let pdf_bytes = pdf_document
        .with_pages(vec![printpdf::PdfPage::new(
            printpdf::Mm(210.0),
            printpdf::Mm(297.0),
            Vec::new(),
        )])
        .save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
    fs::write(track_root.join(certificate::PDF_FILE), &pdf_bytes).expect("PDF fixture");
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
        sha256_file(&track_root.join(integrity::HASH_FILE)).expect("main hash digest"),
        integrity::HASH_FILE,
        sha256_file(&track_root.join(certificate::MANIFEST_FILE)).expect("manifest digest"),
        certificate::MANIFEST_FILE,
        sha256_file(&track_root.join(certificate::CERTIFICATE_FILE)).expect("certificate digest"),
        certificate::CERTIFICATE_FILE,
        sha256_file(&track_root.join(certificate::PDF_FILE)).expect("PDF digest"),
        certificate::PDF_FILE,
    );
    fs::write(
        track_root.join(certificate::CERTIFICATE_HASH_FILE),
        certificate_hashes,
    )
    .expect("certificate hashes fixture");
    let mut record = app.persistence.track(&detail.id).expect("stored track");
    record.status = TrackStatus::Finalized;
    record.certificate = CertificateState {
        valid: true,
        certificate_id: Some("SDM-test".into()),
        finalization_snapshot_id: None,
        finalized_at: Some("2026-08-13T12:00:00Z".into()),
        workflow_version: Some(record.workflow_version.clone()),
        certificate_language: CertificateLanguage::En,
        bilingual: false,
        invalidated_at: None,
        invalidation_reason: None,
    };
    app.persistence
        .save_track(&record)
        .expect("finalized state");

    let locked = app
        .update_track(
            &detail.id,
            TrackPatch {
                release_notes: Some("must not be written".into()),
                ..TrackPatch::default()
            },
        )
        .expect_err("finalized mutation must be refused");
    assert!(matches!(locked, AppError::Finalized));

    fs::remove_dir(track_root.join(".archive/revisions"))
        .expect("simulate an older track without the managed revision parent");
    let revision = app.create_revision(&detail.id).expect("new revision");
    let revised = revision.track.expect("revised track detail");
    assert_eq!(revised.status, TrackStatus::Active);
    assert!(!revised.certificate.valid);
    for relative in [
        certificate::CERTIFICATE_FILE,
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_HASH_FILE,
        certificate::PDF_FILE,
    ] {
        assert!(!track_root.join(relative).exists(), "live {relative}");
    }
    let revision_directories = fs::read_dir(track_root.join(".archive/revisions"))
        .expect("revision archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("revision entries");
    assert_eq!(revision_directories.len(), 1);
    let archive = revision_directories[0].path();
    assert!(archive.join("revision.json").is_file(), "revision metadata");
    for expected in [
        "DOCUMENTATION_CERTIFICATE.md",
        "EVIDENCE_MANIFEST.json",
        "CERTIFICATE_SHA256.txt",
    ] {
        assert!(
            archive.join("certificate").join(expected).is_file(),
            "archived {expected}"
        );
    }
    assert!(archive.join(certificate::PDF_FILE).is_file());
    assert!(archive.join(integrity::HASH_FILE).is_file());

    let mutable = app
        .update_track(
            &detail.id,
            TrackPatch {
                release_notes: Some("revision change".into()),
                ..TrackPatch::default()
            },
        )
        .expect("mutation after revision");
    assert_eq!(mutable.fields.release_notes, "revision change");
}
