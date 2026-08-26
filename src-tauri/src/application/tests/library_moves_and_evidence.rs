use super::*;

#[test]
fn library_reclassification_preserves_active_track_state_and_files() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Immutable Library Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let track_root = app.root().join(&created.relative_path);
    fs::write(
        track_root.join("03_DOCUMENTATION/library-sentinel.bin"),
        b"library placement must not touch track files",
    )
    .expect("track sentinel");

    let mut record = app.persistence.track(&created.id).expect("stored track");
    record.status = TrackStatus::Active;
    record.documents = DocumentState {
        generated: true,
        current: true,
        generated_at: Some("2026-08-01T12:00:00Z".into()),
        template_version: "preserved-template".into(),
        files: vec!["03_DOCUMENTATION/library-sentinel.bin".into()],
        input_fingerprint: "preserved-fingerprint".into(),
    };
    record.integrity = IntegrityState {
        generated: true,
        verified: true,
        file_count: 7,
        verified_count: 7,
        generated_at: Some("2026-08-01T12:01:00Z".into()),
        verified_at: Some("2026-08-01T12:02:00Z".into()),
        mismatch_files: Vec::new(),
    };
    record.certificate = CertificateState {
        valid: true,
        certificate_id: Some("SDM-library-preservation".into()),
        finalization_snapshot_id: None,
        finalized_at: Some("2026-08-01T12:03:00Z".into()),
        workflow_version: Some(record.workflow_version.clone()),
        certificate_language: CertificateLanguage::En,
        bilingual: false,
        invalidated_at: None,
        invalidation_reason: None,
    };
    record.updated_at = "2026-08-01T12:04:00Z".into();
    app.persistence
        .save_track(&record)
        .expect("finalized fixture");
    let protected_before = track_record_without_library_path(&record);
    let tree_before = track_tree_snapshot(&track_root);

    let album = app
        .update_track_library(
            &created.id,
            TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("  Preserved Album  ".into()),
            },
        )
        .expect("active album reassignment");
    assert_eq!(album.status, TrackStatus::Active);
    assert_eq!(album.updated_at, "2026-08-01T12:04:00Z");
    assert_eq!(album.library.section, TrackLibrarySection::Album);
    assert_eq!(
        album.library.album_title.as_deref(),
        Some("Preserved Album")
    );
    let album_record = app.persistence.track(&created.id).expect("stored album");
    assert_eq!(
        track_record_without_library_path(&album_record),
        protected_before
    );
    let album_root = app.root().join(&album.relative_path);
    assert_eq!(track_tree_snapshot(&album_root), tree_before);
    assert!(!track_root.exists());

    let single = app
        .update_track_library(
            &created.id,
            TrackLibraryPlacement {
                section: TrackLibrarySection::Single,
                album_title: Some("must be cleared".into()),
            },
        )
        .expect("active single reassignment");
    assert_eq!(single.library, TrackLibraryPlacement::default());
    let single_record = app.persistence.track(&created.id).expect("stored single");
    assert_eq!(
        track_record_without_library_path(&single_record),
        protected_before
    );
    let single_root = app.root().join(&single.relative_path);
    assert_eq!(track_tree_snapshot(&single_root), tree_before);
    assert!(!album_root.exists(), "the track left its former album path");
    assert!(
        app.root().join("Preserved Album").is_dir(),
        "empty album folders remain reusable"
    );
}

#[test]
fn album_rename_moves_the_folder_and_updates_every_member_path() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let library = TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some("Gravity Drift".into()),
    };
    let first = app
        .create_track(CreateTrackInput {
            title: "Gravaty".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library,
        })
        .expect("first album track");
    let second = app
        .create_track(CreateTrackInput {
            title: "Orbit".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("gravity drift".into()),
            },
        })
        .expect("second album track");
    assert_eq!(second.relative_path, "Gravity Drift/Orbit");
    fs::write(
        app.root()
            .join(&first.relative_path)
            .join("01_RELEASE/master.wav"),
        b"album rename sentinel",
    )
    .expect("sentinel");

    let renamed = app
        .rename_album("Gravity Drift", "Gravity Drive")
        .expect("rename album");

    assert!(!app.root().join("Gravity Drift").exists());
    assert!(app.root().join("Gravity Drive/Gravaty").is_dir());
    assert!(app.root().join("Gravity Drive/Orbit").is_dir());
    assert_eq!(
        fs::read(
            app.root()
                .join("Gravity Drive/Gravaty/01_RELEASE/master.wav")
        )
        .expect("preserved sentinel"),
        b"album rename sentinel"
    );
    for id in [&first.id, &second.id] {
        let summary = renamed
            .iter()
            .find(|track| &track.id == id)
            .expect("renamed member");
        assert!(summary.relative_path.starts_with("Gravity Drive/"));
        assert_eq!(
            summary.library.album_title.as_deref(),
            Some("Gravity Drive")
        );
    }
}

#[test]
fn changing_a_track_title_renames_its_managed_folder() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Old Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let old_root = app.root().join(&created.relative_path);
    fs::write(old_root.join("02_SUNO/source.wav"), b"rename sentinel").expect("sentinel");
    let release_source = directory.path().join("original-master.wav");
    fs::write(&release_source, b"RIFF\x08\0\0\0WAVErename release").expect("release fixture");
    let imported = app
        .import_evidence_from(&created.id, EvidenceRole::ReleaseWav, &release_source)
        .expect("release import before title rename");
    let old_release = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("old release evidence");
    assert_eq!(old_release.relative_path, "01_RELEASE/Old Track.wav");
    let artwork_source = directory.path().join("human-edit.png");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([10, 20, 30, 255]))
        .save(&artwork_source)
        .expect("artwork fixture");
    let imported = app
        .import_evidence_from(
            &created.id,
            EvidenceRole::HumanEditedArtwork,
            &artwork_source,
        )
        .expect("human-edited artwork before title rename");
    assert!(imported
        .evidence
        .iter()
        .any(|item| item.relative_path == "05_ARTWORK/OLD_TRACK_HUMAN_EDITED.png"));

    let renamed = app
        .update_track(
            &created.id,
            TrackPatch {
                title: Some("New Track".into()),
                ..TrackPatch::default()
            },
        )
        .expect("rename track");

    assert_eq!(renamed.relative_path, "Singles/New Track");
    assert!(!old_root.exists());
    assert_eq!(
        fs::read(app.root().join("Singles/New Track/02_SUNO/source.wav"))
            .expect("preserved sentinel"),
        b"rename sentinel"
    );
    let renamed_release = renamed
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("renamed release evidence");
    assert_eq!(renamed_release.file_name, "New Track.wav");
    assert_eq!(renamed_release.relative_path, "01_RELEASE/New Track.wav");
    assert!(app
        .root()
        .join("Singles/New Track/01_RELEASE/New Track.wav")
        .is_file());
    assert!(!app
        .root()
        .join("Singles/New Track/01_RELEASE/Old Track.wav")
        .exists());
    let retained_artwork = renamed
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::HumanEditedArtwork)
        .expect("retained artwork evidence");
    assert_eq!(
        retained_artwork.relative_path,
        "05_ARTWORK/OLD_TRACK_HUMAN_EDITED.png"
    );
    assert!(app
        .root()
        .join("Singles/New Track/05_ARTWORK/OLD_TRACK_HUMAN_EDITED.png")
        .is_file());
}

#[test]
fn track_title_release_collision_rolls_back_folder_file_and_metadata() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Old Conflict".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let source = directory.path().join("release.wav");
    fs::write(&source, b"RIFF\x08\0\0\0WAVEmanaged release").expect("release source");
    app.import_evidence_from(&created.id, EvidenceRole::ReleaseWav, &source)
        .expect("release import");
    let old_root = app.root().join(&created.relative_path);
    let occupied = old_root.join("01_RELEASE/New Conflict.wav");
    fs::write(&occupied, b"unmanaged collision sentinel").expect("collision sentinel");

    let error = app
        .update_track(
            &created.id,
            TrackPatch {
                title: Some("New Conflict".into()),
                ..TrackPatch::default()
            },
        )
        .expect_err("occupied release target must reject title change");
    assert!(matches!(error, AppError::Collision(_)));
    assert!(old_root.is_dir());
    assert!(!app.root().join("Singles/New Conflict").exists());
    assert_eq!(
        fs::read(old_root.join("01_RELEASE/Old Conflict.wav"))
            .expect("managed release after rollback"),
        b"RIFF\x08\0\0\0WAVEmanaged release"
    );
    assert_eq!(
        fs::read(&occupied).expect("collision sentinel after rollback"),
        b"unmanaged collision sentinel"
    );
    let unchanged = app.load_track(&created.id).expect("unchanged track");
    assert_eq!(unchanged.title, "Old Conflict");
    let release = unchanged
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");
    assert_eq!(release.file_name, "Old Conflict.wav");
    assert_eq!(release.relative_path, "01_RELEASE/Old Conflict.wav");
}

#[test]
fn library_move_rolls_back_when_the_database_rejects_the_new_path() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let moving = app
        .create_track(CreateTrackInput {
            title: "First".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("moving track");
    let collision = app
        .create_track(CreateTrackInput {
            title: "Second".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("collision track");
    let collision_root = app.root().join(&collision.relative_path);
    fs::remove_file(collision_root.join(TRACK_IDENTITY_FILE)).expect("remove test identity");
    let mut collision_record = app
        .persistence
        .track(&collision.id)
        .expect("collision record");
    collision_record.relative_path = "Rollback Album/First".into();
    collision_record.library = TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some("Rollback Album".into()),
    };
    app.persistence
        .save_track(&collision_record)
        .expect("stale collision record");

    let error = app
        .update_track_library(
            &moving.id,
            TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("Rollback Album".into()),
            },
        )
        .expect_err("database uniqueness must reject target path");

    assert!(matches!(error, AppError::Database(_)));
    assert!(app.root().join("Singles/First").is_dir());
    assert!(!app.root().join("Rollback Album/First").exists());
    assert!(!app.root().join("Rollback Album").exists());
    let unchanged = app.persistence.track(&moving.id).expect("unchanged record");
    assert_eq!(unchanged.relative_path, moving.relative_path);
    assert_eq!(unchanged.library, TrackLibraryPlacement::default());
}

#[test]
fn reopen_recovers_an_externally_renamed_album_folder_from_track_identity() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Orbit".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("Old Album".into()),
            },
        })
        .expect("track");
    drop(app);
    fs::rename(workspace.join("Old Album"), workspace.join("Renamed Album"))
        .expect("external album rename");

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopen renamed workspace");
    let recovered = reopened.load_track(&created.id).expect("recovered track");
    assert_eq!(recovered.relative_path, "Renamed Album/Orbit");
    assert_eq!(
        recovered.library.album_title.as_deref(),
        Some("Renamed Album")
    );
}

#[test]
fn reopen_repairs_the_reported_legacy_missing_path_from_its_album_folder() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let old_root = workspace.join("Neuer Ordner");
    fs::create_dir_all(old_root.join("01_RELEASE")).expect("legacy track");
    app.scan_workspace().expect("index legacy track");
    let mut record = app
        .persistence
        .track_by_relative_path("Neuer Ordner")
        .expect("lookup")
        .expect("legacy record");
    record.library = TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some("Gravity Drift".into()),
    };
    app.persistence
        .save_track(&record)
        .expect("album assignment");
    drop(app);
    fs::create_dir(workspace.join("Gravity Drift")).expect("album folder");
    fs::rename(&old_root, workspace.join("Gravity Drift/Gravaty")).expect("external track move");

    let reopened = WorkspaceApp::open(&workspace, false).expect("repaired workspace");
    let recovered = reopened
        .load_track(&record.id)
        .expect("recovered legacy track");
    assert_eq!(recovered.relative_path, "Gravity Drift/Gravaty");
    assert_eq!(recovered.title, "Gravaty");
}

#[test]
fn native_validation_rejects_invalid_enums_dates_and_urls() {
    let mut fields = crate::model::TrackFields {
        artwork_origin: "adversarial".into(),
        ..Default::default()
    };
    assert!(validate_track_fields(&fields).is_err());
    fields.artwork_origin = "none".into();
    fields.production_start_date = "2026-08-31".into();
    fields.production_end_date = "2026-08-01".into();
    assert!(validate_track_fields(&fields).is_err());
    fields.production_end_date = "2026-09-01".into();
    fields.suno_project_url = "javascript:alert(1)".into();
    assert!(validate_track_fields(&fields).is_err());
}

#[test]
fn authoritative_release_suno_and_artwork_roles_are_singular() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Singular Assets".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let fixtures = directory.path().join("fixtures");
    fs::create_dir(&fixtures).expect("fixtures");
    let first_wav = fixtures.join("first.wav");
    let second_wav = fixtures.join("second.wav");
    fs::write(&first_wav, b"RIFF\x08\0\0\0WAVEfirst release").expect("first wav");
    fs::write(&second_wav, b"RIFF\x08\0\0\0WAVEsecond release").expect("second wav");
    app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &first_wav)
        .expect("first release import");
    assert!(matches!(
        app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &second_wav),
        Err(AppError::Validation(_))
    ));
    let current_release = app
        .load_track(&track.id)
        .expect("current release")
        .evidence
        .into_iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence");
    let replaced_release = app
        .replace_evidence_from(
            &track.id,
            &current_release.id,
            EvidenceRole::ReleaseWav,
            &second_wav,
        )
        .expect("explicit release replacement");
    let active_release = replaced_release
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("active replacement");
    assert_eq!(active_release.id, current_release.id);
    assert_eq!(active_release.file_name, "Singular Assets.wav");
    assert!(app
        .root()
        .join(&replaced_release.relative_path)
        .join(".archive/evidence-replacements")
        .is_dir());

    app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_wav)
        .expect("first Suno export import");
    assert!(matches!(
        app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &second_wav),
        Err(AppError::Validation(_))
    ));

    let first_art = fixtures.join("first.png");
    let second_art = fixtures.join("second.jpeg");
    image::RgbaImage::from_pixel(64, 64, image::Rgba([20, 30, 40, 255]))
        .save(&first_art)
        .expect("first art");
    image::RgbImage::from_pixel(64, 64, image::Rgb([40, 30, 20]))
        .save(&second_art)
        .expect("second art");
    app.import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &first_art)
        .expect("first artwork import");
    assert!(matches!(
        app.import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &second_art),
        Err(AppError::Validation(_))
    ));
    let current = app.load_track(&track.id).expect("current track");
    assert_eq!(
        current
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::ReleaseWav)
            .count(),
        1
    );
    assert_eq!(
        current
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::SunoFinalExport)
            .count(),
        1
    );
    assert_eq!(
        current
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::FinalArtwork)
            .count(),
        1
    );
}

#[test]
fn release_import_never_overwrites_an_existing_track_title_target() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Collision Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let target = app
        .root()
        .join(&track.relative_path)
        .join("01_RELEASE/Collision Track.wav");
    fs::write(&target, b"existing bytes").expect("collision sentinel");
    let source = directory.path().join("incoming.wav");
    fs::write(&source, b"RIFF\x08\0\0\0WAVEincoming").expect("release source");

    assert!(matches!(
        app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source),
        Err(AppError::Collision(_))
    ));
    assert_eq!(
        fs::read(&target).expect("preserved target"),
        b"existing bytes"
    );
    assert!(app
        .persistence
        .evidence(&track.id)
        .expect("evidence list")
        .is_empty());
}

#[test]
fn unfinalized_legacy_managed_release_name_migrates_but_finalized_snapshot_does_not() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Legacy Managed".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let source = directory.path().join("source.wav");
    fs::write(&source, b"RIFF\x08\0\0\0WAVElegacy managed").expect("release source");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source)
        .expect("release import");
    let mut item = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence")
        .clone();
    let root = app.root().join(&track.relative_path);
    fs::rename(
        root.join(&item.relative_path),
        root.join("01_RELEASE/suno_final_export.wav"),
    )
    .expect("simulate historical managed name");
    item.file_name = "suno_final_export.wav".into();
    item.relative_path = "01_RELEASE/suno_final_export.wav".into();
    app.persistence
        .save_evidence(&track.id, &item)
        .expect("historical evidence metadata");

    let migrated = app.load_track(&track.id).expect("load and migrate");
    let migrated_item = migrated
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("migrated evidence");
    assert_eq!(migrated_item.relative_path, "01_RELEASE/Legacy Managed.wav");
    assert!(root.join("01_RELEASE/Legacy Managed.wav").is_file());

    fs::rename(
        root.join("01_RELEASE/Legacy Managed.wav"),
        root.join("01_RELEASE/suno_final_export.wav"),
    )
    .expect("restore historical name");
    let mut finalized_item = migrated_item.clone();
    finalized_item.file_name = "suno_final_export.wav".into();
    finalized_item.relative_path = "01_RELEASE/suno_final_export.wav".into();
    app.persistence
        .save_evidence(&track.id, &finalized_item)
        .expect("finalized historical metadata");
    let mut record = app.persistence.track(&track.id).expect("track record");
    record.status = TrackStatus::Finalized;
    app.persistence
        .save_track(&record)
        .expect("finalized record");

    assert!(!app
        .migrate_legacy_release_evidence(&record)
        .expect("finalized migration check"));
    assert!(root.join("01_RELEASE/suno_final_export.wav").is_file());
    assert!(!root.join("01_RELEASE/Legacy Managed.wav").exists());
    assert_eq!(
        app.persistence
            .evidence(&track.id)
            .expect("stored finalized evidence")[0]
            .relative_path,
        "01_RELEASE/suno_final_export.wav"
    );
}

#[test]
fn legacy_release_migration_leaves_an_occupied_title_target_unchanged() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Migration Collision".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let source = directory.path().join("source.wav");
    fs::write(&source, b"RIFF\x08\0\0\0WAVElegacy managed").expect("release source");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source)
        .expect("release import");
    let mut item = imported
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("release evidence")
        .clone();
    let root = app.root().join(&track.relative_path);
    fs::rename(
        root.join(&item.relative_path),
        root.join("01_RELEASE/suno_final_export.wav"),
    )
    .expect("simulate historical managed name");
    item.file_name = "suno_final_export.wav".into();
    item.relative_path = "01_RELEASE/suno_final_export.wav".into();
    app.persistence
        .save_evidence(&track.id, &item)
        .expect("historical evidence metadata");
    let occupied = root.join("01_RELEASE/Migration Collision.wav");
    fs::write(&occupied, b"occupied title target").expect("occupied target");

    let loaded = app.load_track(&track.id).expect("load without migration");
    let unchanged = loaded
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .expect("unchanged evidence");
    assert_eq!(unchanged.file_name, "suno_final_export.wav");
    assert_eq!(unchanged.relative_path, "01_RELEASE/suno_final_export.wav");
    assert!(root.join("01_RELEASE/suno_final_export.wav").is_file());
    assert_eq!(
        fs::read(occupied).expect("preserved occupied target"),
        b"occupied title target"
    );
}
