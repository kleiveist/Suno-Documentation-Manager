use super::*;

#[test]
fn verified_timestamp_requires_the_current_complete_verifier_profile() {
    let digest = "a".repeat(64);
    let mut metadata = TimestampProviderMetadata {
        protocol: "RFC 3161".into(),
        request_nonce: "01".into(),
        response_nonce: "01".into(),
        nonce_match: Some(true),
        policy_oid: "1.2.3.4".into(),
        cryptographic_verifier: crate::external_timestamp::RFC3161_CRYPTOGRAPHIC_VERIFIER.into(),
        trust_anchor_sha256: vec!["b".repeat(64)],
        response_structure_valid: Some(true),
        provider_digest_match: Some(true),
        signature_verified: Some(true),
        trust_chain_verified: Some(true),
        verification_result: ExternalTimestampStatus::Verified,
        ..TimestampProviderMetadata::default()
    };
    let record = |metadata: TimestampProviderMetadata| ExternalTimestampRecord {
        id: "timestamp-record".into(),
        certificate_id: "certificate".into(),
        sidecar_format_version: 2,
        provider: "fixture TSA".into(),
        timestamp_type: TimestampType::ElectronicTimestamp,
        timestamp_value: "2026-08-20T07:38:27Z".into(),
        referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
        referenced_artifact_path: certificate::MANIFEST_FILE.into(),
        referenced_sha256: digest.clone(),
        actual_sha256: digest.clone(),
        referenced_hash_match: Some(true),
        external_reference_id: String::new(),
        provider_verification_url: String::new(),
        note: String::new(),
        evidence_file_name: "TIMESTAMP_EVIDENCE.tsr".into(),
        evidence_sha256: "c".repeat(64),
        markdown_sha256: "d".repeat(64),
        pdf_sha256: "e".repeat(64),
        imported_at: "2026-08-20T07:38:27Z".into(),
        provenance: "provider_automatic".into(),
        provider_metadata: Some(metadata),
        record_relative_path: "record/TIMESTAMP_RECORD.json".into(),
        markdown_relative_path: "record/EXTERNAL_TIMESTAMP_ADDENDUM.md".into(),
        pdf_relative_path: "record/EXTERNAL_TIMESTAMP_ADDENDUM.pdf".into(),
        hash_list_relative_path: "record/TIMESTAMP_RECORD_SHA256.txt".into(),
        integrity_verified_at_publication: true,
        integrity_verified: true,
        integrity_issues: Vec::new(),
    };

    assert!(currently_verified_provider_timestamp(&record(
        metadata.clone()
    )));

    metadata.cryptographic_verifier = "sigstore-tsa 0.10.0".into();
    assert!(!currently_verified_provider_timestamp(&record(
        metadata.clone()
    )));

    metadata.cryptographic_verifier =
        crate::external_timestamp::RFC3161_CRYPTOGRAPHIC_VERIFIER.into();
    metadata.policy_oid.clear();
    assert!(!currently_verified_provider_timestamp(&record(metadata)));
}

#[test]
fn folder_import_single_uses_selected_library_and_keeps_incomplete_facts_open() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let source = directory.path().join("Awakening");
    fs::create_dir(&source).expect("source directory");
    fs::write(source.join("Awakening.mp3"), b"ID3audio").expect("MP3 fixture");
    fs::write(source.join("Awakening.mp4"), b"\0\0\0\x0cftypisom").expect("MP4 fixture");
    fs::write(source.join("Awakening.wav"), b"RIFF\x04\0\0\0WAVE").expect("WAV fixture");
    fs::write(source.join("Awakening.jpeg"), b"\xff\xd8\xffimage").expect("JPEG fixture");
    fs::write(
        source.join("Awakening_AI_ORIGINAL.png"),
        b"\x89PNG\r\n\x1a\noriginal",
    )
    .expect("AI original fixture");
    fs::write(
        source.join("Awakening_AI_EDITED.png"),
        b"\x89PNG\r\n\x1a\nedited",
    )
    .expect("AI edited fixture");
    fs::write(
        source.join("Bildschirmfoto_20260817_141059.png"),
        b"\x89PNG\r\n\x1a\nscreenshot",
    )
    .expect("screenshot fixture");
    fs::write(source.join("SpaceWideToWide1.rb"), b"play 60\n").expect("source-code fixture");
    fs::write(source.join("Lyrics.txt"), b"First line\nSecond line\n").expect("lyrics fixture");
    fs::write(source.join("Style.txt"), b"dreamy synthwave\n").expect("style fixture");
    let before = source_file_snapshot(&source);

    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let imported = app
        .import_folder(FolderImportExecutionInput {
            source_path: source.display().to_string(),
            expected_kind: folder_import::FolderImportKind::Single,
            single_track_title: Some("Awakening".into()),
            single_track_library: Some(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("Chosen Album".into()),
            }),
            production_start_date: String::new(),
            commercial_use_intended: Some(false),
        })
        .expect("folder import");

    assert_eq!(imported.len(), 1);
    let track = &imported[0];
    assert_eq!(track.relative_path, "Chosen Album/Awakening");
    assert_eq!(track.library.section, TrackLibrarySection::Album);
    assert_eq!(track.fields.production_start_date, "");
    assert_ne!(track.status, TrackStatus::Finalized);
    assert_eq!(track.fields.code_based_generation, Some(true));
    assert_eq!(track.fields.external_audio_uploaded, None);
    assert_eq!(track.fields.own_audio_uploaded, None);
    assert_eq!(track.fields.third_party_samples_uploaded, None);
    assert_eq!(track.fields.human_editing_performed, None);
    assert_eq!(track.fields.lyrics_text, "First line\nSecond line\n");
    assert!(track.fields.lyrics_source.is_empty());
    assert_eq!(track.fields.suno_style_prompt, "dreamy synthwave\n");
    for role in [
        EvidenceRole::ReleaseMp3,
        EvidenceRole::ReleaseMp4,
        EvidenceRole::ReleaseWav,
        EvidenceRole::ArtworkSunoOriginal,
        EvidenceRole::AiArtworkOriginal,
        EvidenceRole::AiArtworkEdited,
        EvidenceRole::SunoScreenshot,
        EvidenceRole::SourceCodeFile,
        EvidenceRole::Lyrics,
        EvidenceRole::Style,
    ] {
        assert!(
            track.evidence.iter().any(|item| item.role == role),
            "missing imported role {role:?}"
        );
    }
    let track_root = workspace.join(&track.relative_path);
    for folder in TRACK_FOLDERS {
        assert!(track_root.join(folder).is_dir(), "missing {folder}");
    }
    assert!(track_root.join(".summary/track.json").is_file());
    assert_eq!(source_file_snapshot(&source), before);
    assert_eq!(track.steps.len(), 10);
    assert!(track.missing_count > 0);
}

#[test]
fn folder_import_album_creates_normal_tracks_and_leaves_root_files_unassigned() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let source = directory.path().join("The Final Protocol");
    fs::create_dir(&source).expect("album source");
    for title in ["Awakening", "Boot Sequence", "LastWarnung"] {
        let track = source.join(title);
        fs::create_dir(&track).expect("track source");
        fs::write(track.join(format!("{title}.mp3")), b"ID3audio").expect("track media");
    }
    fs::write(source.join("signed_contract.pdf"), b"%PDF-contract").expect("root fixture");
    let before = source_file_snapshot(&source);

    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let imported = app
        .import_folder(FolderImportExecutionInput {
            source_path: source.display().to_string(),
            expected_kind: folder_import::FolderImportKind::Album,
            single_track_title: Some("ignored".into()),
            single_track_library: Some(TrackLibraryPlacement::default()),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: Some(false),
        })
        .expect("album folder import");

    assert_eq!(imported.len(), 3);
    for track in &imported {
        assert!(track.relative_path.starts_with("The Final Protocol/"));
        assert_eq!(track.library.section, TrackLibrarySection::Album);
        assert_eq!(
            track.library.album_title.as_deref(),
            Some("The Final Protocol")
        );
        assert!(track.fields.production_start_date.is_empty());
        assert_ne!(track.status, TrackStatus::Finalized);
        assert!(track
            .evidence
            .iter()
            .any(|item| item.role == EvidenceRole::ReleaseMp3));
        assert!(!track
            .evidence
            .iter()
            .any(|item| item.metadata.original_file_name == "signed_contract.pdf"));
        for folder in TRACK_FOLDERS {
            assert!(workspace.join(&track.relative_path).join(folder).is_dir());
        }
        assert!(workspace
            .join(&track.relative_path)
            .join(".summary/track.json")
            .is_file());
    }
    assert_eq!(source_file_snapshot(&source), before);
}

#[test]
fn folder_import_suno_wav_derives_timestamp_id_and_byte_identity_without_questions() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let source = directory.path().join("Metadata Track");
    fs::create_dir(&source).expect("source directory");
    let raw = p0_suno_comment("2026-08-17T06:38:06Z");
    fs::write(source.join("Metadata Track.wav"), p0_pcm_wav(Some(&raw))).expect("Suno WAV fixture");

    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let imported = app
        .import_folder(FolderImportExecutionInput {
            source_path: source.display().to_string(),
            expected_kind: folder_import::FolderImportKind::Single,
            single_track_title: Some("Metadata Track".into()),
            single_track_library: Some(TrackLibraryPlacement::default()),
            production_start_date: String::new(),
            commercial_use_intended: Some(false),
        })
        .expect("folder import");
    let track = &imported[0];

    assert_eq!(track.fields.suno_download_export_date, "2026-08-17");
    assert_eq!(track.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(track.fields.production_end_date, "2026-08-17");
    assert_eq!(
        track.automation.download_export_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        track.automation.suno_created_timestamp.as_deref(),
        Some("2026-08-17T06:38:06Z")
    );
    assert_eq!(track.automation.suno_id.as_deref(), Some(P0_SUNO_ID));
    assert!(track.automation.release_identical_to_suno_export);
    let suno = track
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoFinalExport)
        .expect("Suno evidence");
    assert_eq!(suno.metadata.suno_id, P0_SUNO_ID);
    assert_eq!(suno.metadata.suno_created_timestamp, "2026-08-17T06:38:06Z");
    assert!(suno.sha256.as_deref().is_some_and(|hash| hash.len() == 64));
}

#[test]
fn folder_import_rejects_a_target_inside_the_source_before_creating_a_track() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    fs::write(workspace.join("Source.mp3"), b"ID3audio").expect("source media");
    let before = source_file_snapshot(&workspace);

    let result = app.import_folder(FolderImportExecutionInput {
        source_path: workspace.display().to_string(),
        expected_kind: folder_import::FolderImportKind::Single,
        single_track_title: Some("Unsafe Nested Target".into()),
        single_track_library: Some(TrackLibraryPlacement::default()),
        production_start_date: String::new(),
        commercial_use_intended: Some(false),
    });

    assert!(matches!(result, Err(AppError::Validation(_))));
    assert!(app.list_tracks().expect("track list").is_empty());
    assert_eq!(source_file_snapshot(&workspace), before);
}
#[test]
fn removing_release_audio_archives_live_screening_artifacts_and_marks_state_stale() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Remove Release Screening",
    );
    let root = app.root().join(&ready.relative_path);
    let live = root.join(audio_screening::AUDIO_SCREENING_DIR);
    assert!(live.join("LOCAL_FINGERPRINT.json").is_file());
    let release = ready
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");

    let removed = app
        .remove_evidence(&ready.id, &release.id)
        .expect("remove release evidence");

    assert_eq!(
        removed.audio_screening.local.status,
        AudioScreeningStatus::Stale
    );
    assert!(!live.exists(), "old screening must not remain live");
    let archive_entries = fs::read_dir(root.join(".archive/audio-screening"))
        .expect("screening archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("screening archive entries");
    assert!(archive_entries.iter().any(|entry| {
        entry
            .path()
            .join("AUDIO_SCREENING/LOCAL_FINGERPRINT.json")
            .is_file()
    }));
}

#[test]
fn release_byte_mismatch_stales_and_archives_screening_before_new_docs_or_hashes() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Mutated Release Screening",
    );
    let root = app.root().join(&ready.relative_path);
    let release = ready
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");
    fs::write(
        root.join(&release.relative_path),
        p0_screening_wav(None, 91),
    )
    .expect("out-of-band source mutation");

    let verified = app
        .verify_evidence(&ready.id, Some(&release.id))
        .expect("record controlled mismatch");

    assert_eq!(
        verified.audio_screening.local.status,
        AudioScreeningStatus::Stale
    );
    assert!(!root.join(audio_screening::AUDIO_SCREENING_DIR).exists());
    assert!(matches!(
        app.generate_documents(&ready.id, false),
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        app.calculate_hashes(&ready.id),
        Err(AppError::Validation(_))
    ));
}

#[test]
fn altered_local_fingerprint_artifact_blocks_documents_hashes_and_finalization() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Altered Fingerprint Artifact",
    );
    let root = app.root().join(&ready.relative_path);
    let fingerprint = root.join(audio_screening::LOCAL_FINGERPRINT_FILE);
    let mut bytes = fs::read(&fingerprint).expect("local fingerprint artifact");
    bytes.extend_from_slice(b"\nmutated outside SunoDM\n");
    fs::write(&fingerprint, bytes).expect("mutate local fingerprint artifact");

    assert!(matches!(
        app.generate_documents(&ready.id, false),
        Err(AppError::Validation(message)) if message.contains("local audio-screening record")
    ));
    assert!(matches!(
        app.calculate_hashes(&ready.id),
        Err(AppError::Validation(message)) if message.contains("local audio-screening record")
    ));
    let finalization = app.finalize_track(&ready.id);
    assert!(
        matches!(finalization, Err(AppError::Validation(message)) if message.contains("audio screening"))
    );
    assert!(!root.join(certificate::CERTIFICATE_FILE).exists());
}

#[test]
fn editable_workflow_upgrade_automatically_generates_current_local_screening() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Editable Screening Upgrade",
    );
    let root = app.root().join(&ready.relative_path);
    let mut legacy = app.persistence.track(&ready.id).expect("stored track");
    legacy.workflow_version = "1.7".into();
    legacy.audio_screening = Default::default();
    legacy.fields.suno_content_classification = None;
    legacy.fields.vocal_intent = None;
    legacy.fields.suno_lyrics_field_content = Some(true);
    legacy.fields.suno_lyrics_content_types = vec![
        SunoLyricsContentType::VocalLyrics,
        SunoLyricsContentType::StructureInstructions,
    ];
    legacy.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
    legacy.fields.suno_lyrics_field_text = "Legacy vocal line\n[Drop]".into();
    app.persistence
        .save_track(&legacy)
        .expect("simulate old workflow");
    fs::remove_dir_all(root.join(audio_screening::AUDIO_SCREENING_DIR))
        .expect("remove simulated pre-feature directory");

    let upgraded = app
        .re_evaluate_track(&ready.id)
        .expect("upgrade editable track")
        .track
        .expect("upgraded detail");

    assert_eq!(upgraded.workflow_version, "1.9");
    assert_eq!(
        upgraded.fields.suno_content_classification,
        Some(SunoContentClassification::Mixed)
    );
    assert_eq!(upgraded.fields.vocal_intent, None);
    assert_eq!(upgraded.fields.suno_lyrics_field_content, None);
    assert!(upgraded.fields.suno_lyrics_content_types.is_empty());
    assert_eq!(
        upgraded.audio_screening.local.status,
        AudioScreeningStatus::FingerprintGenerated
    );
    assert!(root.join(audio_screening::LOCAL_FINGERPRINT_FILE).is_file());
}

#[test]
fn release_replacement_resets_optional_external_state_without_blocking_finalization() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Replacement Optional Provider",
    );
    assert_eq!(
        ready.audio_screening.external.status,
        AudioScreeningStatus::SkippedNotConfigured
    );
    let release = ready
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");
    let replacement_source = directory.path().join("replacement-release.wav");
    fs::write(&replacement_source, p0_screening_wav(None, 101))
        .expect("replacement release source");

    let replaced = app
        .replace_evidence_from(
            &ready.id,
            &release.id,
            EvidenceRole::ReleaseWav,
            &replacement_source,
        )
        .expect("replace release and rerun local screening");

    assert_eq!(
        replaced.audio_screening.local.status,
        AudioScreeningStatus::FingerprintGenerated
    );
    assert_eq!(
        replaced.audio_screening.external.status,
        AudioScreeningStatus::SkippedNotConfigured
    );
    app.update_track(
        &ready.id,
        TrackPatch {
            release_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("confirm replacement filename difference");
    app.generate_documents(&ready.id, false)
        .expect("documents after replacement");
    app.calculate_hashes(&ready.id)
        .expect("hashes after replacement");
    let validation = app.validate_track(&ready.id).expect("finalization gate");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
    assert_eq!(
        app.finalize_track(&ready.id)
            .expect("finalization remains allowed")
            .track
            .expect("finalized detail")
            .status,
        TrackStatus::Finalized
    );
}
