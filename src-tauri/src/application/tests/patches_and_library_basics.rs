use super::*;

#[test]
fn track_patches_clear_values_from_inactive_conditional_branches() {
    let mut fields = crate::model::TrackFields {
        lyrics_source: "human".into(),
        lyrics_text: "stale human lyrics".into(),
        external_audio_uploaded: Some(true),
        external_audio_source: "stale external source".into(),
        external_audio_ownership: "stale external rights".into(),
        own_audio_uploaded: Some(true),
        own_audio_source: "stale own source".into(),
        own_audio_ownership: "stale own rights".into(),
        third_party_samples_uploaded: Some(true),
        third_party_sample_source: "stale sample source".into(),
        third_party_sample_ownership: "stale sample rights".into(),
        human_editing_performed: Some(true),
        human_editing_details: "stale human edit".into(),
        post_export_editing_performed: Some(true),
        post_export_editing_details: "stale post edit".into(),
        artwork_origin: "ai_assisted".into(),
        ai_image_service: "stale AI service".into(),
        human_artwork_modifications: vec!["stale artwork modifications".into()],
        depicts_real_person: Some(true),
        real_person_notes: "stale person note".into(),
        depicts_real_event: Some(true),
        real_event_notes: "stale event note".into(),
        contains_trademark: Some(true),
        trademark_notes: "stale trademark note".into(),
        disclosure_applied: Some(true),
        disclosure_text: "stale disclosure".into(),
        ..crate::model::TrackFields::default()
    };

    apply_patch(
        &mut fields,
        TrackPatch {
            lyrics_source: Some("instrumental".into()),
            external_audio_uploaded: Some(false),
            own_audio_uploaded: Some(false),
            code_based_generation: Some(false),
            third_party_samples_uploaded: Some(false),
            human_editing_performed: Some(false),
            post_export_editing_performed: Some(false),
            artwork_origin: Some("human".into()),
            depicts_real_person: Some(false),
            depicts_real_event: Some(false),
            contains_trademark: Some(false),
            ..TrackPatch::default()
        },
    );

    assert_eq!(fields.lyrics_text, "stale human lyrics");
    assert_eq!(fields.lyrics_source, "instrumental");
    for value in [
        &fields.external_audio_source,
        &fields.external_audio_ownership,
        &fields.own_audio_source,
        &fields.own_audio_ownership,
        &fields.third_party_sample_source,
        &fields.third_party_sample_ownership,
        &fields.human_editing_details,
        &fields.post_export_editing_details,
        &fields.ai_image_service,
        &fields.real_person_notes,
        &fields.real_event_notes,
        &fields.trademark_notes,
        &fields.disclosure_text,
    ] {
        assert!(value.is_empty(), "inactive value survived: {value}");
    }
    assert!(fields.human_artwork_modifications.is_empty());
    assert_eq!(fields.disclosure_applied, None);
    assert_eq!(fields.depicts_real_person, Some(false));

    apply_patch(
        &mut fields,
        TrackPatch {
            artwork_origin: Some("none".into()),
            ..TrackPatch::default()
        },
    );
    assert_eq!(fields.depicts_real_person, None);
    assert_eq!(fields.depicts_real_event, None);
    assert_eq!(fields.contains_trademark, None);

    let mut expected = fields.clone();
    expected.lyrics_text = "updated legacy value".into();
    apply_patch(
        &mut fields,
        TrackPatch {
            lyrics_text: Some("updated legacy value".into()),
            real_person_notes: Some("ignored hidden note".into()),
            ..TrackPatch::default()
        },
    );
    assert_eq!(
        fields, expected,
        "legacy lyrics remain editable, while inactive artwork details stay ignored"
    );
}

#[test]
fn desktop_patch_null_clears_documented_nullable_facts_but_omission_preserves_them() {
    let mut fields = crate::model::TrackFields {
        instrumental_track: Some(true),
        vocal_lyrics_present: Some(false),
        vocal_intent: Some(VocalIntent::Vocal),
        suno_content_classification: Some(SunoContentClassification::Mixed),
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
        suno_lyrics_field_text: "Lyrics\n[Bridge]".into(),
        generative_ai_used: Some(true),
        audio_ai_system: "Suno".into(),
        ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
        ..crate::model::TrackFields::default()
    };
    let request: TrackPatchRequest = serde_json::from_value(serde_json::json!({
        "instrumentalTrack": null,
        "vocalIntent": null,
        "sunoContentClassification": null,
        "generativeAiUsed": null,
        "aiGeneratedAudioElements": null
    }))
    .expect("desktop track patch");

    apply_patch_with_explicit_nulls(&mut fields, request.patch, &request.explicit_null_fields);

    assert_eq!(fields.instrumental_track, None);
    assert_eq!(fields.vocal_intent, None);
    assert_eq!(fields.suno_content_classification, None);
    assert_eq!(fields.generative_ai_used, None);
    assert_eq!(fields.ai_generated_audio_elements, None);
    assert_eq!(fields.vocal_lyrics_present, Some(false));
    assert_eq!(fields.audio_ai_system, "Suno");
}

#[test]
fn legacy_suno_classification_migration_is_conservative_and_never_infers_intent() {
    let cases = [
        (
            Some(false),
            vec![SunoLyricsContentType::Mixed],
            Some(SunoContentClassification::Empty),
        ),
        (
            Some(true),
            vec![SunoLyricsContentType::VocalLyrics],
            Some(SunoContentClassification::VocalLyricsOnly),
        ),
        (
            Some(true),
            vec![
                SunoLyricsContentType::StructureInstructions,
                SunoLyricsContentType::SoundInstructions,
                SunoLyricsContentType::ArrangementInstructions,
            ],
            Some(SunoContentClassification::StructureOnly),
        ),
        (
            Some(true),
            vec![
                SunoLyricsContentType::VocalLyrics,
                SunoLyricsContentType::StructureInstructions,
            ],
            Some(SunoContentClassification::Mixed),
        ),
        (
            Some(true),
            vec![SunoLyricsContentType::Other],
            Some(SunoContentClassification::Other),
        ),
    ];

    for (legacy_answer, legacy_types, expected) in cases {
        let mut fields = crate::model::TrackFields {
            suno_lyrics_field_content: legacy_answer,
            suno_lyrics_content_types: legacy_types,
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
            suno_lyrics_field_text: "legacy exact text".into(),
            ..Default::default()
        };
        assert!(migrate_legacy_suno_semantics(&mut fields));
        assert_eq!(fields.suno_content_classification, expected);
        assert_eq!(fields.vocal_intent, None);
        assert_eq!(fields.suno_lyrics_field_content, None);
        assert!(fields.suno_lyrics_content_types.is_empty());
        if expected == Some(SunoContentClassification::Empty) {
            assert_eq!(fields.suno_lyrics_content_source, None);
            assert!(fields.suno_lyrics_field_text.is_empty());
        }
    }

    for ambiguous in [
        vec![SunoLyricsContentType::Mixed],
        vec![
            SunoLyricsContentType::Other,
            SunoLyricsContentType::VocalLyrics,
        ],
        vec![
            SunoLyricsContentType::Other,
            SunoLyricsContentType::StructureInstructions,
        ],
    ] {
        let mut fields = crate::model::TrackFields {
            suno_lyrics_field_content: Some(true),
            suno_lyrics_content_types: ambiguous.clone(),
            ..Default::default()
        };
        assert!(!migrate_legacy_suno_semantics(&mut fields));
        assert_eq!(fields.suno_content_classification, None);
        assert_eq!(fields.vocal_intent, None);
        assert_eq!(fields.suno_lyrics_content_types, ambiguous);
    }
}

#[test]
fn canonical_suno_classification_wins_over_conflicting_legacy_controllers() {
    let mut fields = crate::model::TrackFields {
        vocal_intent: Some(VocalIntent::Vocal),
        suno_content_classification: Some(SunoContentClassification::Mixed),
        suno_lyrics_field_content: Some(false),
        suno_lyrics_content_types: vec![SunoLyricsContentType::Other],
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
        suno_lyrics_field_text: "Lyrics\n[Drop]".into(),
        suno_lyrics_other_content_type: "stale legacy label".into(),
        ..Default::default()
    };

    assert!(migrate_legacy_suno_semantics(&mut fields));
    assert_eq!(
        fields.suno_content_classification,
        Some(SunoContentClassification::Mixed)
    );
    assert_eq!(fields.vocal_intent, Some(VocalIntent::Vocal));
    assert_eq!(fields.suno_lyrics_field_content, None);
    assert!(fields.suno_lyrics_content_types.is_empty());
    assert_eq!(fields.suno_lyrics_field_text, "Lyrics\n[Drop]");
    assert!(fields.suno_lyrics_other_content_type.is_empty());
}
#[test]
fn workspace_creation_initializes_local_database() {
    let directory = tempdir().expect("tempdir");
    let root = directory.path().join("workspace");
    let app = WorkspaceApp::open(&root, true).expect("create workspace");
    assert!(root.join(".suno-doc/workspace.sqlite").is_file());
    assert!(root.join(SINGLES_DIRECTORY).is_dir());
    assert_eq!(app.summary().expect("summary").track_count, 0);
}

#[test]
fn album_creation_persists_an_empty_folder_and_supports_rename() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");

    assert_eq!(
        app.create_album("  Gravity Drift  ")
            .expect("create empty album"),
        vec!["Gravity Drift"]
    );
    assert!(workspace.join("Gravity Drift").is_dir());
    assert!(workspace.join(SINGLES_DIRECTORY).is_dir());
    assert_eq!(
        app.scan_workspace().expect("scan empty album").discovered,
        0,
        "an empty album must not be indexed as a track"
    );

    app.rename_album("Gravity Drift", "Gravity Drive")
        .expect("rename empty album");
    assert!(!workspace.join("Gravity Drift").exists());
    assert!(workspace.join("Gravity Drive").is_dir());
    assert_eq!(
        app.list_albums().expect("album list"),
        vec!["Gravity Drive"]
    );
    assert!(matches!(
        app.create_album("gravity drive")
            .expect_err("case-insensitive duplicate must fail"),
        AppError::Collision(_)
    ));
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
    assert_eq!(
        reopened.list_albums().expect("reopened album list"),
        vec!["Gravity Drive"]
    );
    assert!(workspace.join(SINGLES_DIRECTORY).is_dir());
}

#[test]
fn hidden_workspace_folders_are_pruned_from_album_and_track_discovery() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");

    fs::create_dir_all(workspace.join(".archive/Archived Track/01_RELEASE"))
        .expect("hidden archive fixture");
    fs::create_dir_all(workspace.join(".draft/01_RELEASE")).expect("hidden direct-track fixture");
    fs::create_dir_all(workspace.join("Visible Album/Visible Track/01_RELEASE"))
        .expect("visible album fixture");

    assert_eq!(
        app.list_albums().expect("album list"),
        vec!["Visible Album"]
    );
    let scan = app.scan_workspace().expect("workspace scan");
    assert_eq!(scan.discovered, 1);
    assert_eq!(scan.indexed, 1);
    assert!(scan.warnings.is_empty());
    assert_eq!(
        scan.candidates[0].relative_path,
        "Visible Album/Visible Track"
    );
    assert!(app
        .persistence
        .track_by_relative_path(".archive/Archived Track")
        .expect("hidden archive lookup")
        .is_none());
    assert!(app
        .persistence
        .track_by_relative_path(".draft")
        .expect("hidden direct-track lookup")
        .is_none());
}

#[test]
fn previously_indexed_hidden_paths_remain_unloaded_after_reopen() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Archived Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let source = workspace.join(&created.relative_path);
    let hidden_parent = workspace.join(".archive");
    let target = hidden_parent.join("Archived Track");
    fs::create_dir(&hidden_parent).expect("hidden parent");
    fs::rename(&source, &target).expect("move fixture into hidden parent");

    let mut record = app.persistence.track(&created.id).expect("stored track");
    record.relative_path = ".archive/Archived Track".into();
    record.library = TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some(".archive".into()),
    };
    app.persistence
        .save_track(&record)
        .expect("persist pre-fix hidden path fixture");

    assert!(app.list_tracks().expect("visible tracks").is_empty());
    assert_eq!(app.summary().expect("visible summary").track_count, 0);
    assert!(matches!(
        app.load_track(&created.id),
        Err(AppError::TrackNotFound(id)) if id == created.id
    ));
    assert!(target.join(TRACK_IDENTITY_FILE).is_file());
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
    assert!(reopened.list_tracks().expect("reopened tracks").is_empty());
    assert!(reopened.list_albums().expect("reopened albums").is_empty());
    assert_eq!(
        reopened.scan_workspace().expect("reopened scan").discovered,
        0
    );
    assert!(target.join(TRACK_IDENTITY_FILE).is_file());
}

#[test]
fn track_creation_builds_exact_folders() {
    let directory = tempdir().expect("tempdir");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let track = app
        .create_track(CreateTrackInput {
            title: "Acceptance Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");
    let root = app.root().join(&track.relative_path);
    for folder in TRACK_FOLDERS {
        assert!(root.join(folder).is_dir(), "{folder}");
    }
}

#[test]
fn track_creation_persists_album_library_placement_after_reopen() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");

    let created = app
        .create_track(CreateTrackInput {
            title: "Album Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some("  Night Drive  ".into()),
            },
        })
        .expect("album track");
    assert_eq!(created.library.section, TrackLibrarySection::Album);
    assert_eq!(created.library.album_title.as_deref(), Some("Night Drive"));
    assert_eq!(created.relative_path, "Night Drive/Album Track");
    assert!(workspace.join("Night Drive/Album Track").is_dir());
    assert_eq!(crate::persistence::SCHEMA_VERSION, 7);
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
    let detail = reopened.load_track(&created.id).expect("reopened track");
    assert_eq!(detail.library, created.library);
    let summary = reopened
        .list_tracks()
        .expect("track summaries")
        .into_iter()
        .find(|track| track.id == created.id)
        .expect("album summary");
    assert_eq!(summary.library, created.library);
}

#[test]
fn new_guided_and_free_text_track_values_survive_workspace_reopen_exactly() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let assisted_id = create_future_model_track(&app);
    let human_id = create_human_artwork_track(&app);
    drop(app);

    let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
    let assisted = reopened
        .load_track(&assisted_id)
        .expect("reload AI-assisted track");
    assert_future_model_track(&assisted);
    let human = reopened
        .load_track(&human_id)
        .expect("reload human-artwork track");
    assert_human_artwork_track(&human);
}

fn create_future_model_track(app: &WorkspaceApp) -> String {
    let assisted = app
        .create_track(CreateTrackInput {
            title: "Future Model Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("AI-assisted track");
    app.update_track(
        &assisted.id,
        TrackPatch {
            suno_model: Some("v6 private preview".into()),
            suno_plan_at_generation: Some("Historical Founder Plan".into()),
            instrumental_track: Some(true),
            vocal_lyrics_present: Some(false),
            vocal_intent: Some(VocalIntent::Vocal),
            suno_content_classification: Some(SunoContentClassification::Mixed),
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: Some("Vocal line\n[Drop]".into()),
            code_based_generation: Some(true),
            code_audio_post_processed: Some(true),
            code_audio_post_processing_operations: Some(vec![
                "Mixing".into(),
                "EQ".into(),
                "Other post-processing".into(),
            ]),
            code_audio_post_processing_note: Some("Manual spectral repair".into()),
            artwork_origin: Some("ai_assisted".into()),
            ai_image_service: Some("Future Image Tool".into()),
            human_artwork_modifications: Some(vec![
                "Cropping".into(),
                "Typography added".into(),
                "Other human editing".into(),
            ]),
            custom_artwork_change: Some("Hand-painted edge cleanup".into()),
            depicts_real_person: Some(true),
            real_person_notes: Some("A named collaborator in the foreground".into()),
            depicts_real_event: Some(false),
            contains_trademark: Some(true),
            trademark_notes: Some("A supplied sponsor logo in the corner".into()),
            ..TrackPatch::default()
        },
    )
    .expect("persist AI-assisted values");
    assisted.id
}

fn create_human_artwork_track(app: &WorkspaceApp) -> String {
    let human = app
        .create_track(CreateTrackInput {
            title: "Human Artwork Track".into(),
            production_start_date: "2026-08-02".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("human-artwork track");
    app.update_track(
        &human.id,
        TrackPatch {
            artwork_origin: Some("human".into()),
            human_artwork_process_operations: Some(vec![
                "Photographed".into(),
                "Color correction".into(),
                "Typography added".into(),
            ]),
            human_artwork_process_notes: Some("35 mm scan, then manual layout".into()),
            depicts_real_person: Some(false),
            depicts_real_event: Some(false),
            contains_trademark: Some(false),
            ..TrackPatch::default()
        },
    )
    .expect("persist human-artwork values");
    human.id
}

fn assert_future_model_track(assisted: &TrackDetail) {
    assert_eq!(assisted.fields.suno_model, "v6 private preview");
    assert_eq!(
        assisted.fields.suno_plan_at_generation,
        "Historical Founder Plan"
    );
    assert_eq!(assisted.fields.instrumental_track, Some(true));
    assert_eq!(assisted.fields.vocal_lyrics_present, Some(false));
    assert_eq!(assisted.fields.vocal_intent, Some(VocalIntent::Vocal));
    assert_eq!(
        assisted.fields.suno_content_classification,
        Some(SunoContentClassification::Mixed)
    );
    assert_eq!(
        assisted.fields.suno_lyrics_content_source,
        Some(SunoLyricsContentSource::Mixed)
    );
    assert_eq!(assisted.fields.suno_lyrics_field_text, "Vocal line\n[Drop]");
    assert_eq!(assisted.fields.suno_lyrics_field_content, None);
    assert!(assisted.fields.suno_lyrics_content_types.is_empty());
    assert_eq!(assisted.fields.code_audio_post_processed, Some(true));
    assert_eq!(
        assisted.fields.code_audio_post_processing_operations,
        vec!["Mixing", "EQ", "Other post-processing"]
    );
    assert_eq!(
        assisted.fields.code_audio_post_processing_note,
        "Manual spectral repair"
    );
    assert_eq!(
        assisted.fields.human_artwork_modifications,
        vec!["Cropping", "Typography added", "Other human editing"]
    );
    assert_eq!(
        assisted.fields.custom_artwork_change,
        "Hand-painted edge cleanup"
    );
    assert_eq!(assisted.fields.depicts_real_person, Some(true));
    assert_eq!(assisted.fields.depicts_real_event, Some(false));
    assert_eq!(assisted.fields.contains_trademark, Some(true));
    assert_eq!(
        assisted.fields.real_person_notes,
        "A named collaborator in the foreground"
    );
    assert_eq!(
        assisted.fields.trademark_notes,
        "A supplied sponsor logo in the corner"
    );
}

fn assert_human_artwork_track(human: &TrackDetail) {
    assert_eq!(
        human.fields.human_artwork_process_operations,
        vec!["Photographed", "Color correction", "Typography added"]
    );
    assert_eq!(
        human.fields.human_artwork_process_notes,
        "35 mm scan, then manual layout"
    );
    assert_eq!(human.fields.depicts_real_person, Some(false));
    assert_eq!(human.fields.depicts_real_event, Some(false));
    assert_eq!(human.fields.contains_trademark, Some(false));
}
#[test]
fn older_track_json_defaults_to_single_library_section_without_rewriting_it() {
    let legacy_input: CreateTrackInput = serde_json::from_value(serde_json::json!({
        "title": "Legacy API Input",
        "productionStartDate": "2026-08-01",
        "commercialUseIntended": false
    }))
    .expect("legacy create input");
    assert_eq!(legacy_input.library, TrackLibraryPlacement::default());

    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let created = app
        .create_track(CreateTrackInput {
            title: "Pre Library Track".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track");

    let record = app.persistence.track(&created.id).expect("stored track");
    let mut legacy_json = serde_json::to_value(record).expect("track JSON");
    legacy_json
        .as_object_mut()
        .expect("track object")
        .remove("library");
    legacy_json
        .pointer_mut("/fields")
        .and_then(serde_json::Value::as_object_mut)
        .expect("track fields")
        .remove("codeBasedGeneration");
    app.persistence
        .open()
        .expect("database")
        .execute(
            "UPDATE tracks SET data_json=?1 WHERE id=?2",
            rusqlite::params![
                serde_json::to_string(&legacy_json).expect("legacy JSON"),
                created.id
            ],
        )
        .expect("remove library field from stored fixture");

    let defaulted = app.persistence.track(&created.id).expect("defaulted track");
    assert_eq!(defaulted.library, TrackLibraryPlacement::default());
    assert_eq!(defaulted.fields.code_based_generation, None);
    let loaded = app.load_track(&created.id).expect("load defaulted track");
    assert_eq!(loaded.library, TrackLibraryPlacement::default());
    assert_eq!(loaded.fields.code_based_generation, None);
    let materialized_json: String = app
        .persistence
        .open()
        .expect("database")
        .query_row(
            "SELECT data_json FROM tracks WHERE id=?1",
            [&created.id],
            |row| row.get(0),
        )
        .expect("materialized track JSON");
    assert!(
        serde_json::from_str::<serde_json::Value>(&materialized_json)
            .expect("materialized JSON")
            .pointer("/library")
            .is_none(),
        "loading a legacy record must not rewrite its JSON merely to add defaults"
    );
}

#[test]
fn legacy_scan_defaults_library_placement_without_modifying_track_files() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    let legacy_root = workspace.join("Historical Track");
    fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy directory");
    fs::write(
        legacy_root.join("01_RELEASE/history.wav"),
        b"RIFF\x08\0\0\0WAVEhistorical track",
    )
    .expect("legacy file");
    let before = track_tree_snapshot(&legacy_root);

    app.scan_workspace().expect("legacy scan");
    let indexed = app
        .persistence
        .track_by_relative_path("Historical Track")
        .expect("legacy lookup")
        .expect("indexed legacy track");
    assert!(indexed.legacy);
    assert_eq!(indexed.library, TrackLibraryPlacement::default());
    assert_eq!(track_tree_snapshot(&legacy_root), before);
}

#[test]
fn track_library_validation_rejects_invalid_albums_and_normalizes_singles() {
    for album_title in [
        None,
        Some(String::new()),
        Some("   ".into()),
        Some(".archive".into()),
        Some("  .private  ".into()),
        Some("invalid\ncontrol".into()),
        Some("x".repeat(201)),
    ] {
        assert!(matches!(
            normalize_track_library(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title,
            }),
            Err(AppError::Validation(_))
        ));
    }
    assert!(
        serde_json::from_value::<TrackLibraryPlacement>(serde_json::json!({
            "section": "ep"
        }))
        .is_err()
    );
    assert_eq!(
        normalize_track_library(TrackLibraryPlacement {
            section: TrackLibrarySection::Single,
            album_title: Some("Ignored album".into()),
        })
        .expect("single normalization"),
        TrackLibraryPlacement::default()
    );
    assert_eq!(
        serde_json::to_value(TrackLibraryPlacement::default()).expect("single serialization"),
        serde_json::json!({ "section": "single" })
    );

    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let error = app
        .create_track(CreateTrackInput {
            title: "Invalid Album".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: None,
            },
        })
        .expect_err("album without a title must fail");
    assert!(matches!(error, AppError::Validation(_)));
    assert!(!app.root().join("Invalid-Album").exists());
    assert!(app.list_tracks().expect("empty track list").is_empty());
}
