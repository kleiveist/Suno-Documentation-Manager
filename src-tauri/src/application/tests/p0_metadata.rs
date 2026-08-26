use super::*;

#[test]
fn p0_suno_marker_alias_import_persists_metadata_and_derives_authoritative_dates() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Metadata Import", None, false);
    let raw = p0_suno_marker_alias_comment("2026-08-18T07:17:52Z", P0_SUNO_MARKER_ALIAS_ID);
    let source = directory.path().join("suno-metadata.wav");
    let wav = p0_pcm_wav(Some(&raw));
    fs::write(&source, &wav).expect("write Suno WAV");

    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("import Suno WAV");
    let evidence = p0_evidence(&imported, EvidenceRole::SunoFinalExport);

    assert!(evidence.verified);
    assert!(evidence
        .sha256
        .as_deref()
        .is_some_and(|hash| !hash.is_empty()));
    assert_eq!(evidence.size_bytes, wav.len() as u64);
    assert_eq!(evidence.metadata.original_file_name, "suno-metadata.wav");
    assert_eq!(evidence.metadata.file_extension, "wav");
    assert_eq!(evidence.metadata.mime_type, "audio/wav");
    assert_eq!(evidence.metadata.audio_format, "WAV");
    assert_eq!(evidence.metadata.audio_channels, Some(2));
    assert_eq!(evidence.metadata.audio_sample_rate_hz, Some(48_000));
    assert_eq!(evidence.metadata.audio_duration_milliseconds, Some(10));
    assert_eq!(evidence.metadata.audio_bit_depth, Some(16));
    assert_eq!(
        evidence.metadata.embedded_metadata,
        vec![EmbeddedMetadata {
            key: "ICMT".into(),
            value: raw.clone(),
        }]
    );
    assert!(evidence.metadata.suno_studio_detected);
    assert_eq!(
        evidence.metadata.suno_created_timestamp,
        "2026-08-18T07:17:52Z"
    );
    assert_eq!(evidence.metadata.suno_created_date, "2026-08-18");
    assert_eq!(evidence.metadata.suno_id, P0_SUNO_MARKER_ALIAS_ID);
    assert_eq!(evidence.metadata.suno_raw_metadata, raw);
    assert_eq!(
        imported.fields.suno_final_generation_id,
        P0_SUNO_MARKER_ALIAS_ID
    );
    assert_eq!(
        imported.automation.final_generation_id_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(imported.fields.suno_final_generation_date, "2026-08-18");
    assert_eq!(
        imported.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(imported.fields.production_end_date, "2026-08-18");
    assert_eq!(
        imported.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(imported.fields.suno_download_export_date, "2026-08-18");
    assert_eq!(
        imported.automation.download_export_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert!(imported.fields.final_export_date.is_empty());
}

#[test]
fn p0_final_generation_id_follows_wav_only_while_system_owned() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Automatic Generation ID", Some(false), false);
    let first_source = directory.path().join("automatic-id-first.wav");
    fs::write(
        &first_source,
        p0_pcm_wav(Some(&p0_suno_comment_with_id(
            "2026-08-17T06:38:06Z",
            P0_SUNO_ID,
        ))),
    )
    .expect("first Suno WAV");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
        .expect("import automatic ID");
    let evidence_id = p0_evidence(&imported, EvidenceRole::SunoFinalExport)
        .id
        .clone();
    assert_eq!(imported.fields.suno_final_generation_id, P0_SUNO_ID);
    assert_eq!(
        imported.automation.final_generation_id_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let replacement_source = directory.path().join("automatic-id-replacement.wav");
    fs::write(
        &replacement_source,
        p0_pcm_wav(Some(&p0_suno_comment_with_id(
            "2026-08-18T06:38:06Z",
            P0_SECOND_SUNO_ID,
        ))),
    )
    .expect("replacement Suno WAV");
    let replaced = app
        .replace_evidence_from(
            &track.id,
            &evidence_id,
            EvidenceRole::SunoFinalExport,
            &replacement_source,
        )
        .expect("replace automatic ID");
    assert_eq!(replaced.fields.suno_final_generation_id, P0_SECOND_SUNO_ID);
    assert_eq!(
        replaced.automation.final_generation_id_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let overridden = app
        .update_track(
            &track.id,
            TrackPatch {
                suno_final_generation_id: Some("manually-confirmed-generation-id".into()),
                ..TrackPatch::default()
            },
        )
        .expect("record manual ID");
    assert_eq!(
        overridden.fields.suno_final_generation_id,
        "manually-confirmed-generation-id"
    );
    assert_eq!(
        overridden.automation.final_generation_id_origin,
        FactOrigin::UserConfirmedFact
    );

    let third_source = directory.path().join("automatic-id-third.wav");
    fs::write(
        &third_source,
        p0_pcm_wav(Some(&p0_suno_comment_with_id(
            "2026-08-19T06:38:06Z",
            P0_THIRD_SUNO_ID,
        ))),
    )
    .expect("third Suno WAV");
    let preserved = app
        .replace_evidence_from(
            &track.id,
            &evidence_id,
            EvidenceRole::SunoFinalExport,
            &third_source,
        )
        .expect("replace while manual ID exists");
    assert_eq!(
        preserved.fields.suno_final_generation_id,
        "manually-confirmed-generation-id"
    );
    assert_eq!(
        preserved.automation.final_generation_id_origin,
        FactOrigin::UserConfirmedFact
    );

    assert_manual_generation_id_is_preserved(&app, &directory);
}

fn assert_manual_generation_id_is_preserved(app: &WorkspaceApp, directory: &tempfile::TempDir) {
    let manual_track = p0_track(app, "P0 Preexisting Generation ID", Some(false), false);
    let manual_track = app
        .update_track(
            &manual_track.id,
            TrackPatch {
                suno_final_generation_id: Some("preexisting-manual-id".into()),
                ..TrackPatch::default()
            },
        )
        .expect("record preexisting manual ID");
    let manual_source = directory.path().join("manual-id.wav");
    fs::write(
        &manual_source,
        p0_pcm_wav(Some(&p0_suno_comment_with_id(
            "2026-08-17T06:38:06Z",
            P0_SUNO_ID,
        ))),
    )
    .expect("manual-ID Suno WAV");
    let preserved_manual = app
        .import_evidence_from(
            &manual_track.id,
            EvidenceRole::SunoFinalExport,
            &manual_source,
        )
        .expect("import alongside manual ID");
    assert_eq!(
        preserved_manual.fields.suno_final_generation_id,
        "preexisting-manual-id"
    );
    assert_eq!(
        preserved_manual.automation.final_generation_id_origin,
        FactOrigin::UserConfirmedFact
    );
}

#[test]
fn p0_no_post_editing_derives_production_end_and_identical_release_passes() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Identical Audio", Some(false), false);
    let raw = p0_suno_comment("2026-08-17T06:38:06Z");
    let bytes = p0_pcm_wav(Some(&raw));
    let suno_source = directory.path().join("separate-suno.wav");
    let release_source = directory.path().join("separate-release.wav");
    fs::write(&suno_source, &bytes).expect("write Suno source");
    fs::write(&release_source, &bytes).expect("write release source");

    app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &suno_source)
        .expect("import Suno WAV");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &release_source)
        .expect("import byte-identical release WAV");
    let suno = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
    let release = p0_evidence(&imported, EvidenceRole::ReleaseWav);

    assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(imported.fields.production_end_date, "2026-08-17");
    assert_eq!(imported.fields.suno_download_export_date, "2026-08-17");
    assert_eq!(imported.fields.final_export_date, "2026-08-17");
    assert_eq!(
        imported.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(suno.sha256, release.sha256);
    assert!(imported.automation.release_identical_to_suno_export);
    assert!(imported.automation.byte_identical_pairs.iter().any(|pair| {
        pair.sha256 == suno.sha256.clone().unwrap_or_default()
            && matches!(
                (pair.left_role, pair.right_role),
                (EvidenceRole::SunoFinalExport, EvidenceRole::ReleaseWav)
                    | (EvidenceRole::ReleaseWav, EvidenceRole::SunoFinalExport)
            )
    }));
}

#[test]
fn p0_metadata_date_remains_authoritative_when_post_export_editing_changes() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let raw = p0_suno_comment("2026-08-17T06:38:06Z");

    let edited_first = p0_track(&app, "P0 Editing Before Import", Some(true), false);
    let edited_first_source = directory.path().join("edited-first.wav");
    fs::write(&edited_first_source, p0_pcm_wav(Some(&raw))).expect("write first WAV");
    let edited_first = app
        .import_evidence_from(
            &edited_first.id,
            EvidenceRole::SunoFinalExport,
            &edited_first_source,
        )
        .expect("import after editing answer");
    assert_eq!(edited_first.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(edited_first.fields.production_end_date, "2026-08-17");
    assert_eq!(edited_first.fields.suno_download_export_date, "2026-08-17");
    assert!(edited_first.fields.final_export_date.is_empty());
    assert_eq!(
        edited_first.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let automated_first = p0_track(&app, "P0 Editing After Import", Some(false), false);
    let automated_source = directory.path().join("automated-first.wav");
    fs::write(&automated_source, p0_pcm_wav(Some(&raw))).expect("write second WAV");
    let automated_first = app
        .import_evidence_from(
            &automated_first.id,
            EvidenceRole::SunoFinalExport,
            &automated_source,
        )
        .expect("derive production end before editing answer changes");
    assert_eq!(automated_first.fields.production_end_date, "2026-08-17");
    assert_eq!(automated_first.fields.final_export_date, "2026-08-17");

    let editing_enabled = app
        .update_track(
            &automated_first.id,
            TrackPatch {
                post_export_editing_performed: Some(true),
                post_export_editing_details: Some("Mastering after export".into()),
                ..TrackPatch::default()
            },
        )
        .expect("record post-export editing");
    assert_eq!(editing_enabled.fields.production_end_date, "2026-08-17");
    assert!(editing_enabled.fields.final_export_date.is_empty());
    assert_eq!(
        editing_enabled.automation.final_export_origin,
        FactOrigin::NotDocumented
    );

    let editing_date = app
        .update_track(
            &automated_first.id,
            TrackPatch {
                final_export_date: Some("2026-08-19".into()),
                ..TrackPatch::default()
            },
        )
        .expect("record the actual desktop editing date");
    assert_eq!(editing_date.fields.final_export_date, "2026-08-19");
    assert_eq!(
        editing_date.automation.final_export_origin,
        FactOrigin::UserConfirmedFact
    );
    assert_eq!(
        editing_enabled.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let manual_end = app
        .update_track(
            &automated_first.id,
            TrackPatch {
                production_end_date: Some("2026-08-20".into()),
                ..TrackPatch::default()
            },
        )
        .expect("attempt a manual production-end override");
    assert_eq!(manual_end.fields.production_end_date, "2026-08-17");
    assert_eq!(
        manual_end.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
}

#[test]
fn p0_plain_wav_uses_manual_fallback_without_inventing_suno_values() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Plain WAV", Some(false), false);
    let source = directory.path().join("plain.wav");
    fs::write(&source, p0_pcm_wav(None)).expect("write plain WAV");

    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("plain WAV import remains valid");
    let evidence = p0_evidence(&imported, EvidenceRole::SunoFinalExport);

    assert_eq!(evidence.metadata.audio_format, "WAV");
    assert_eq!(evidence.metadata.audio_channels, Some(2));
    assert!(!evidence.metadata.suno_studio_detected);
    assert!(evidence.metadata.suno_created_timestamp.is_empty());
    assert!(evidence.metadata.suno_created_date.is_empty());
    assert!(evidence.metadata.suno_id.is_empty());
    assert!(evidence.metadata.suno_raw_metadata.is_empty());
    assert!(imported.fields.suno_final_generation_id.is_empty());
    assert!(imported.fields.suno_final_generation_date.is_empty());
    assert!(imported.fields.production_end_date.is_empty());
    assert_eq!(
        imported.automation.final_generation_origin,
        FactOrigin::NotDocumented
    );
    assert!(imported.automation.consistency_issues.is_empty());
}

#[test]
fn p0_optional_control_metadata_is_ignored_without_losing_valid_suno_facts() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Optional Control", Some(false), false);
    let raw = p0_suno_comment("2026-08-17T06:38:06Z");
    let source = directory.path().join("optional-control.wav");
    fs::write(
        &source,
        p0_pcm_wav_with_info_entries(&[
            (*b"IART", b"unsafe\x01artist".to_vec()),
            (*b"ICMT", raw.as_bytes().to_vec()),
        ]),
    )
    .expect("WAV with optional control metadata");

    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("safe metadata subset imports");
    let item = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
    assert_eq!(
        item.metadata.embedded_metadata,
        vec![EmbeddedMetadata {
            key: "ICMT".into(),
            value: raw.clone(),
        }]
    );
    assert_eq!(item.metadata.suno_raw_metadata, raw);
    assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(imported.fields.production_end_date, "2026-08-17");
}

#[test]
fn p0_control_in_suno_comment_imports_wav_without_derived_facts() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Unsafe Suno Comment", Some(false), false);
    let unsafe_raw = format!(
        "made with suno studio; note=unsafe\x01text; created=2026-08-17T06:38:06Z; id={P0_SUNO_ID}"
    );
    let source = directory.path().join("unsafe-suno-comment.wav");
    fs::write(
        &source,
        p0_pcm_wav_with_info_entries(&[(*b"ICMT", unsafe_raw.into_bytes())]),
    )
    .expect("WAV with unsafe Suno comment");

    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("unsafe optional metadata does not reject WAV");
    let item = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
    assert!(item.metadata.embedded_metadata.is_empty());
    assert!(!item.metadata.suno_studio_detected);
    assert!(item.metadata.suno_raw_metadata.is_empty());
    assert!(imported.fields.suno_final_generation_date.is_empty());
    assert!(imported.fields.production_end_date.is_empty());
}

#[test]
fn p0_metadata_date_replaces_a_manual_fallback_without_a_conflict() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Manual Conflict", None, false);
    let manual = app
        .update_track(
            &track.id,
            TrackPatch {
                suno_final_generation_date: Some("2026-08-16".into()),
                ..TrackPatch::default()
            },
        )
        .expect("manual generation date");
    let raw = p0_suno_comment("2026-08-17T06:38:06Z");
    let source = directory.path().join("conflicting.wav");
    fs::write(&source, p0_pcm_wav(Some(&raw))).expect("write conflicting WAV");

    let imported = app
        .import_evidence_from(&manual.id, EvidenceRole::SunoFinalExport, &source)
        .expect("import authoritative metadata");

    assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(imported.fields.production_end_date, "2026-08-17");
    assert_eq!(
        imported.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        imported.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert!(!imported
        .automation
        .consistency_issues
        .iter()
        .any(|issue| matches!(
            issue.code.as_str(),
            "suno_generation_date_conflict" | "production_end_date_conflict"
        )));

    let overridden = app
        .update_track(
            &manual.id,
            TrackPatch {
                suno_final_generation_date: Some("2026-08-15".into()),
                production_end_date: Some("2026-08-20".into()),
                ..TrackPatch::default()
            },
        )
        .expect("native reconciliation rejects submitted overrides");
    assert_eq!(overridden.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(overridden.fields.production_end_date, "2026-08-17");
    assert!(!overridden
        .missing_items
        .iter()
        .any(|item| item.contains("Suno-Erzeugungsdatum")));
}

#[test]
fn p0_replacing_current_suno_export_updates_only_system_owned_values() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Replace Suno", Some(false), false);
    let track = app
        .update_track(
            &track.id,
            TrackPatch {
                suno_download_export_date: Some("2026-08-21".into()),
                final_export_date: Some("2026-08-22".into()),
                ..TrackPatch::default()
            },
        )
        .expect("manual export dates");
    let first_source = directory.path().join("replace-first.wav");
    fs::write(
        &first_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("first Suno WAV");
    let first = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
        .expect("first Suno import");
    let evidence_id = p0_evidence(&first, EvidenceRole::SunoFinalExport)
        .id
        .clone();
    assert_eq!(first.fields.suno_final_generation_date, "2026-08-17");
    assert_eq!(first.fields.production_end_date, "2026-08-17");

    let second_source = directory.path().join("replace-second.wav");
    fs::write(
        &second_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-18T06:38:06Z"))),
    )
    .expect("second Suno WAV");
    let second = app
        .replace_evidence_from(
            &track.id,
            &evidence_id,
            EvidenceRole::SunoFinalExport,
            &second_source,
        )
        .expect("replace current Suno export");
    assert_eq!(second.fields.suno_final_generation_date, "2026-08-18");
    assert_eq!(second.fields.production_end_date, "2026-08-18");
    assert_eq!(second.fields.suno_download_export_date, "2026-08-18");
    assert_eq!(second.fields.final_export_date, "2026-08-18");
    assert_eq!(
        second.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        second.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );

    let manual_end = app
        .update_track(
            &track.id,
            TrackPatch {
                production_end_date: Some("2026-08-20".into()),
                ..TrackPatch::default()
            },
        )
        .expect("attempt manual production end override");
    assert_eq!(manual_end.fields.production_end_date, "2026-08-18");
    assert_eq!(
        manual_end.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    let third_source = directory.path().join("replace-third.wav");
    fs::write(
        &third_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-19T06:38:06Z"))),
    )
    .expect("third Suno WAV");
    let third = app
        .replace_evidence_from(
            &track.id,
            &evidence_id,
            EvidenceRole::SunoFinalExport,
            &third_source,
        )
        .expect("replace current Suno export again");

    assert_eq!(third.fields.suno_final_generation_date, "2026-08-19");
    assert_eq!(third.fields.production_end_date, "2026-08-19");
    assert_eq!(
        third.automation.production_end_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(third.fields.suno_download_export_date, "2026-08-19");
    assert_eq!(third.fields.final_export_date, "2026-08-19");
    assert_eq!(
        third
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::SunoFinalExport)
            .count(),
        1
    );
    let current = p0_evidence(&third, EvidenceRole::SunoFinalExport);
    assert_eq!(current.id, evidence_id);
    assert_eq!(
        current.metadata.suno_created_timestamp,
        "2026-08-19T06:38:06Z"
    );
}

#[test]
fn p0_removing_suno_export_clears_only_automatic_values() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Remove Automatic", Some(false), false);
    let track = app
        .update_track(
            &track.id,
            TrackPatch {
                suno_download_export_date: Some("2026-08-21".into()),
                final_export_date: Some("2026-08-22".into()),
                ..TrackPatch::default()
            },
        )
        .expect("manual non-derived dates");
    let source = directory.path().join("remove-automatic.wav");
    fs::write(
        &source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("Suno WAV");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("Suno import");
    let evidence_id = p0_evidence(&imported, EvidenceRole::SunoFinalExport)
        .id
        .clone();

    let removed = app
        .remove_evidence(&track.id, &evidence_id)
        .expect("remove current Suno export");
    assert!(removed.fields.suno_final_generation_id.is_empty());
    assert!(removed.fields.suno_final_generation_date.is_empty());
    assert!(removed.fields.production_end_date.is_empty());
    assert!(removed.fields.suno_download_export_date.is_empty());
    assert!(removed.fields.final_export_date.is_empty());
    assert_eq!(
        removed.automation.final_generation_id_origin,
        FactOrigin::NotDocumented
    );
    assert_eq!(
        removed.automation.final_generation_origin,
        FactOrigin::NotDocumented
    );
    assert_eq!(
        removed.automation.production_end_origin,
        FactOrigin::NotDocumented
    );
    assert_eq!(
        removed.automation.download_export_origin,
        FactOrigin::NotDocumented
    );
    assert_eq!(
        removed.automation.final_export_origin,
        FactOrigin::NotDocumented
    );

    let manual_track = p0_track(&app, "P0 Remove Preserves Manual", Some(false), false);
    let manual_source = directory.path().join("remove-manual.wav");
    fs::write(
        &manual_source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("second Suno WAV");
    let manual_track = app
        .import_evidence_from(
            &manual_track.id,
            EvidenceRole::SunoFinalExport,
            &manual_source,
        )
        .expect("second Suno import");
    let manual_id = p0_evidence(&manual_track, EvidenceRole::SunoFinalExport)
        .id
        .clone();
    let removed_manual = app
        .remove_evidence(&manual_track.id, &manual_id)
        .expect("remove export before recording a fallback");
    assert!(removed_manual.fields.suno_final_generation_date.is_empty());
    assert!(removed_manual.fields.production_end_date.is_empty());
    let manual_fallback = app
        .update_track(
            &manual_track.id,
            TrackPatch {
                suno_final_generation_date: Some("2026-08-17".into()),
                production_end_date: Some("2026-08-20".into()),
                ..TrackPatch::default()
            },
        )
        .expect("record manual fallbacks without metadata");
    assert_eq!(
        manual_fallback.fields.suno_final_generation_date,
        "2026-08-17"
    );
    assert_eq!(manual_fallback.fields.production_end_date, "2026-08-20");
    assert_eq!(
        manual_fallback.automation.final_generation_origin,
        FactOrigin::UserConfirmedFact
    );
    assert_eq!(
        manual_fallback.automation.production_end_origin,
        FactOrigin::UserConfirmedFact
    );
}

#[test]
fn p0_metadata_derived_generation_date_feeds_subscription_coverage() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    let track = p0_track(&app, "P0 Subscription Coverage", Some(false), true);
    let source = directory.path().join("coverage-suno.wav");
    fs::write(
        &source,
        p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
    )
    .expect("coverage Suno WAV");
    let imported = app
        .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
        .expect("metadata-derived generation date");
    assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");

    let receipt = directory.path().join("coverage-subscription.pdf");
    fs::write(&receipt, b"%PDF-1.7\nsubscription receipt\n%%EOF\n").expect("subscription PDF");
    let global = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            &receipt,
            Some("2026-08-01".into()),
            Some("2026-08-31".into()),
        )
        .expect("global subscription evidence");
    let attached = app
        .attach_global_evidence(&track.id, &global.evidence.id)
        .expect("attach covering subscription evidence");
    let stored = app.persistence.track(&track.id).expect("stored track");

    assert_eq!(
        workflow::subscription_generation_coverage(&stored, &attached.evidence),
        CoverageStatus::Yes
    );
    assert_eq!(
        stored.field_origins.suno_final_generation_date,
        Some(EvidenceDerivedField {
            value: "2026-08-17".into(),
            original_value: "2026-08-17T06:38:06Z".into(),
            evidence_id: p0_evidence(&attached, EvidenceRole::SunoFinalExport)
                .id
                .clone(),
            evidence_sha256: p0_evidence(&attached, EvidenceRole::SunoFinalExport)
                .sha256
                .clone()
                .expect("Suno SHA-256"),
        })
    );
}
