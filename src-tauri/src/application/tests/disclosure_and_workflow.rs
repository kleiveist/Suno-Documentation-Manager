use super::*;

#[test]
fn always_disclosure_policy_requires_final_artwork_to_match_local_disclosure_output() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let fixtures = directory.path().join("fixtures");
    let base = prepare_ready_track(&app, &fixtures, "Disclosure Lineage");
    let (imported_edited, imported_final, track_root) =
        assert_imported_disclosure_lineage_is_rejected(&app, &fixtures, &base);
    assert_generated_disclosure_lineage_is_accepted(
        &app,
        &base,
        &track_root,
        &imported_edited,
        &imported_final,
    );
}

fn assert_imported_disclosure_lineage_is_rejected(
    app: &WorkspaceApp,
    fixtures: &Path,
    base: &TrackDetail,
) -> (EvidenceItem, EvidenceItem, PathBuf) {
    app.update_track(
        &base.id,
        TrackPatch {
            artwork_origin: Some("ai_assisted".into()),
            ai_image_service: Some("Local Tool".into()),
            human_artwork_modifications: Some(vec!["Visible disclosure added locally".into()]),
            depicts_real_person: Some(true),
            real_person_notes: Some("Documented real-person depiction".into()),
            depicts_real_event: Some(false),
            contains_trademark: Some(false),
            disclosure_applied: Some(true),
            disclosure_text: Some("AI-assisted".into()),
            ..TrackPatch::default()
        },
    )
    .expect("AI artwork facts");
    let ai_original = fixtures.join("lineage-original.png");
    let manually_edited = fixtures.join("manually-imported-edited.png");
    image::RgbaImage::from_pixel(640, 640, image::Rgba([20, 40, 80, 255]))
        .save(&ai_original)
        .expect("AI original fixture");
    image::RgbaImage::from_pixel(640, 640, image::Rgba([180, 30, 60, 255]))
        .save(&manually_edited)
        .expect("manually edited artwork fixture");
    app.import_evidence_from(&base.id, EvidenceRole::AiArtworkOriginal, &ai_original)
        .expect("AI original import");
    let imported = app
        .import_evidence_from(&base.id, EvidenceRole::AiArtworkEdited, &manually_edited)
        .expect("manually edited artwork import");
    let imported_edited = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::AiArtworkEdited)
        .expect("imported AI-edited evidence")
        .clone();
    assert_eq!(imported_edited.provenance, EvidenceProvenance::ManagedCopy);
    assert_eq!(imported.fields.disclosure_applied, Some(true));
    let track_root = app.root().join(&base.relative_path);
    let with_matching_import = app
        .import_evidence_from(
            &base.id,
            EvidenceRole::FinalArtwork,
            &track_root.join(&imported_edited.relative_path),
        )
        .expect("hash-equal final artwork import");
    let imported_final = with_matching_import
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .expect("hash-equal imported final evidence")
        .clone();
    assert_eq!(imported_final.sha256, imported_edited.sha256);
    app.generate_documents(&base.id, false)
        .expect("documents with imported artwork pair");
    app.calculate_hashes(&base.id)
        .expect("hashes with imported artwork pair");
    let rejected = app
        .validate_track(&base.id)
        .expect("imported-lineage gate evaluation");
    assert!(!rejected.valid);
    assert!(rejected
        .missing_items
        .iter()
        .any(|item| item.contains("AI-Kennzeichnung")));
    assert_eq!(
        rejected
            .missing_items
            .iter()
            .filter(|item| item.contains("finale Artwork"))
            .count(),
        1,
        "final artwork must be required exactly once"
    );
    assert!(matches!(
        app.finalize_track(&base.id),
        Err(AppError::Validation(_))
    ));
    (imported_edited, imported_final, track_root)
}

fn assert_generated_disclosure_lineage_is_accepted(
    app: &WorkspaceApp,
    base: &TrackDetail,
    track_root: &Path,
    imported_edited: &EvidenceItem,
    imported_final: &EvidenceItem,
) {
    app.remove_evidence(&base.id, &imported_final.id)
        .expect("remove untrusted final artwork");
    let after_imported_edit_removal = app
        .remove_evidence(&base.id, &imported_edited.id)
        .expect("remove manually imported AI-edited artwork");
    assert_eq!(
        after_imported_edit_removal.fields.disclosure_applied,
        Some(true),
        "a manually asserted flag alone must not become trusted provenance"
    );
    let disclosed = app
        .generate_artwork_disclosure(&base.id, Some("AI-assisted".into()))
        .expect("local disclosure output")
        .track
        .expect("disclosed track");
    let generated = disclosed
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::AiArtworkEdited)
        .expect("locally generated AI-edited evidence")
        .clone();
    assert_eq!(
        generated.provenance,
        EvidenceProvenance::GeneratedDisclosure
    );
    assert_eq!(
        generated.derived_from_evidence_id.as_deref(),
        disclosed
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::AiArtworkOriginal)
            .map(|item| item.id.as_str())
    );
    assert_eq!(
        generated.generator_version.as_deref(),
        Some(crate::artwork::DISCLOSURE_GENERATOR_VERSION)
    );
    let with_linked_final = app
        .import_evidence_from(
            &base.id,
            EvidenceRole::FinalArtwork,
            &track_root.join(&generated.relative_path),
        )
        .expect("import final artwork from disclosed bytes");
    let linked = with_linked_final
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .expect("linked final evidence");
    assert_eq!(linked.sha256, generated.sha256);
    assert_eq!(
        fs::read(track_root.join(&linked.relative_path)).expect("linked final bytes"),
        fs::read(track_root.join(&generated.relative_path)).expect("disclosure output bytes")
    );
    app.generate_documents(&base.id, false)
        .expect("current linked documents");
    app.calculate_hashes(&base.id)
        .expect("current linked hashes");
    let accepted = app
        .validate_track(&base.id)
        .expect("linked gate evaluation");
    assert!(
        accepted.valid,
        "missing={:?}; blocking={:?}",
        accepted.missing_items, accepted.blocking_items
    );
    let finalized = app
        .finalize_track(&base.id)
        .expect("linked artwork finalization")
        .track
        .expect("finalized linked track");
    assert_eq!(finalized.status, TrackStatus::Finalized);
    assert!(finalized.certificate.valid);
}
#[test]
fn missing_evidence_can_be_removed_and_reimported_during_recovery_revision() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let fixtures = directory.path().join("fixtures");
    let finalized = finalize_acceptance_track(&app, &fixtures, "Missing Evidence Recovery");
    let release = finalized
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence")
        .clone();
    let track_root = app.root().join(&finalized.relative_path);
    let managed_path = track_root.join(&release.relative_path);
    fs::remove_file(&managed_path).expect("external evidence deletion");
    let invalid = app
        .load_track(&finalized.id)
        .expect("load invalidated finalized track");
    assert!(!invalid.certificate.valid);
    assert!(invalid
        .evidence
        .iter()
        .any(|item| item.id == release.id && !item.verified));

    let revision = app
        .create_revision(&finalized.id)
        .expect("start recovery revision")
        .track
        .expect("active recovery track");
    assert_eq!(revision.status, TrackStatus::Active);
    let removed = app
        .remove_evidence(&finalized.id, &release.id)
        .expect("remove metadata for externally missing evidence");
    assert!(!removed.evidence.iter().any(|item| item.id == release.id));
    assert!(!managed_path.exists());

    let reimported = app
        .import_evidence_from(
            &finalized.id,
            EvidenceRole::ReleaseWav,
            &fixtures.join("release-master.wav"),
        )
        .expect("reimport same managed target name");
    let replacement = reimported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("replacement release evidence");
    assert_ne!(replacement.id, release.id);
    assert_eq!(replacement.relative_path, release.relative_path);
    assert!(replacement.verified);
    assert!(managed_path.is_file());

    app.update_track(
        &finalized.id,
        TrackPatch {
            release_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("reconfirm the reimported release filename difference");

    app.generate_documents(&finalized.id, false)
        .expect("regenerate recovery documents");
    app.calculate_hashes(&finalized.id)
        .expect("regenerate recovery hashes");
    let recovered_gate = app
        .validate_track(&finalized.id)
        .expect("validate recovered revision");
    assert!(
        recovered_gate.valid,
        "missing={:?}; blocking={:?}",
        recovered_gate.missing_items, recovered_gate.blocking_items
    );
    let recovered = app
        .finalize_track(&finalized.id)
        .expect("finalize recovered revision")
        .track
        .expect("recovered finalized track");
    assert_eq!(recovered.status, TrackStatus::Finalized);
    assert!(recovered.certificate.valid);
    let manifest = fs::read_to_string(track_root.join(certificate::MANIFEST_FILE))
        .expect("recovered manifest");
    let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
        .expect("recovered Markdown certificate");
    assert!(manifest.contains(".archive/revisions/"));
    assert!(markdown.contains(".archive/revisions/"));
    let pdf = fs::read(track_root.join(certificate::PDF_FILE)).expect("recovered PDF");
    let mut warnings = Vec::new();
    let pdf_text =
        printpdf::PdfDocument::parse(&pdf, &printpdf::PdfParseOptions::default(), &mut warnings)
            .expect("parse recovered PDF")
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
    assert!(pdf_text.contains(".archive/revisions/"));
}

#[test]
fn workflow_upgrade_archives_finalized_v19_and_requires_fresh_v110_outputs() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized =
        finalize_acceptance_track(&app, &directory.path().join("fixtures"), "Workflow Upgrade");
    assert_eq!(finalized.workflow_version, "1.9");
    let track_root = app.root().join(&finalized.relative_path);
    let certificate_before = certificate_file_snapshot(&track_root);
    let hashes_before =
        fs::read(track_root.join(integrity::HASH_FILE)).expect("read finalized SHA256SUMS");
    app.persistence
        .save_step(
            &finalized.id,
            &StepState {
                id: "source".into(),
                status: StepStatus::Fail,
                na_reason: None,
                updated_at: Some("2026-08-13T12:00:00Z".into()),
            },
        )
        .expect("stored old-workflow override");

    let workflow_v110 = workflow::config_with_version_for_test("1.10")
        .expect("test-only workflow 1.10 configuration");
    let upgraded = app
        .re_evaluate_track_with_workflow(&finalized.id, &workflow_v110)
        .expect("explicit workflow reevaluation")
        .track
        .expect("upgraded track detail");

    assert_eq!(upgraded.status, TrackStatus::Active);
    assert_eq!(upgraded.workflow_id, "suno-track");
    assert_eq!(upgraded.workflow_version, "1.10");
    assert!(!upgraded.documents.current);
    assert!(!upgraded.integrity.generated);
    assert!(!upgraded.integrity.verified);
    assert!(!upgraded.certificate.valid);
    assert!(upgraded.certificate.certificate_id.is_none());
    assert!(!track_root.join(integrity::HASH_FILE).exists());
    assert!(app
        .persistence
        .stored_steps(&finalized.id)
        .expect("stored steps after upgrade")
        .is_empty());

    let archives = fs::read_dir(track_root.join(".archive/revisions"))
        .expect("revision archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("revision entries");
    assert_eq!(archives.len(), 1);
    let archive = archives[0].path();
    assert_eq!(
        fs::read(archive.join(integrity::HASH_FILE)).expect("archived SHA256SUMS"),
        hashes_before
    );
    for (relative, bytes) in certificate_before {
        let archived = if relative == certificate::PDF_FILE {
            archive.join(certificate::PDF_FILE)
        } else {
            let file_name = Path::new(&relative)
                .file_name()
                .expect("certificate file name");
            archive.join("certificate").join(file_name)
        };
        assert_eq!(
            fs::read(archived).expect("archived certificate byte snapshot"),
            bytes
        );
    }

    assert!(matches!(
        app.re_evaluate_track_with_workflow(&finalized.id, &workflow_v110),
        Err(AppError::Validation(_))
    ));
    assert_eq!(
        fs::read_dir(track_root.join(".archive/revisions"))
            .expect("unchanged revision archive")
            .count(),
        1
    );
}
