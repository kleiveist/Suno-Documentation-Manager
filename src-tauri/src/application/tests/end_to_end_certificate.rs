use super::*;

struct EndToEndFixtures {
    fixture_root: PathBuf,
    suno_export: PathBuf,
    release_master: PathBuf,
    ai_original: PathBuf,
    release_mp3: PathBuf,
    source_code: PathBuf,
    code_audio: PathBuf,
    subscription_one: PathBuf,
    subscription_two: PathBuf,
    terms_source: PathBuf,
}

#[test]
fn end_to_end_documentation_workflow_creates_portable_certificate() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let updated = create_end_to_end_track(&app);
    let fixtures = write_end_to_end_fixtures(directory.path());
    import_end_to_end_track_evidence(&app, &updated, &fixtures);
    attach_end_to_end_global_evidence(&app, &updated, &fixtures);
    let (final_detail, track_root) = finalize_end_to_end_track(&app, &updated);
    let manifest_bytes =
        reproduce_end_to_end_certificate(&app, directory.path(), &updated, &track_root);
    let manifest_json = assert_end_to_end_manifest(&app, manifest_bytes);
    assert_end_to_end_markdown_and_counts(&track_root, &manifest_json);
    attach_end_to_end_timestamp_and_revision(&app, &updated, &final_detail, &fixtures, &track_root);
    if std::env::var_os("SUNODM_KEEP_ACCEPTANCE_FIXTURE").is_some() {
        let retained = directory.keep();
        eprintln!("retained acceptance fixture: {}", retained.display());
    }
}

fn create_end_to_end_track(app: &WorkspaceApp) -> TrackDetail {
    let created = app
        .create_track(CreateTrackInput {
            title: "End To End".into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: true,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track creation");
    app.update_track(
        &created.id,
        TrackPatch {
            production_end_date: Some("2026-08-02".into()),
            suno_model: Some("v4.5".into()),
            suno_project_url: Some("https://suno.com/song/test-track".into()),
            suno_project_version_id: Some("project-end-to-end-v1".into()),
            suno_final_generation_id: Some("generation-end-to-end".into()),
            suno_final_generation_time: Some("14:35".into()),
            suno_plan_at_generation: Some("Pro".into()),
            final_export_date: Some("2026-08-03".into()),
            instrumental_track: Some(true),
            vocal_lyrics_present: Some(false),
            vocal_intent: Some(VocalIntent::Instrumental),
            suno_content_classification: Some(SunoContentClassification::StructureOnly),
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: Some(
                "[Intro]\n[sidechained synth pad]\n[Drop]\n[Outro]".into(),
            ),
            suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
            external_audio_uploaded: Some(false),
            own_audio_uploaded: Some(false),
            code_based_generation: Some(true),
            code_audio_post_processed: Some(true),
            code_audio_post_processing_operations: Some(vec![
                "EQ".into(),
                "Other post-processing".into(),
            ]),
            code_audio_post_processing_note: Some(
                "Rendered Sonic Pi layer was level-adjusted before use.".into(),
            ),
            third_party_samples_uploaded: Some(false),
            human_editing_performed: Some(false),
            post_export_editing_performed: Some(true),
            post_export_editing_details: Some(
                "Final level adjustment and metadata preparation.".into(),
            ),
            commercial_use_intended: Some(true),
            generative_ai_used: Some(true),
            audio_ai_system: Some("Suno".into()),
            ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
            ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
            real_person_voice_intentionally_imitated: Some(DocumentationAnswer::No),
            real_person_identity_intentionally_represented: Some(DocumentationAnswer::No),
            real_event_represented_as_authentic_recording: Some(DocumentationAnswer::No),
            real_location_institution_event_presented_as_authentic_ai_recording: Some(
                DocumentationAnswer::NotDocumented,
            ),
            audio_disclosure_applied: Some(DocumentationAnswer::Yes),
            audio_disclosure_locations: Some(vec!["release metadata".into(), "description".into()]),
            audio_disclosure_text: Some(
                "AI-generated and AI-assisted audio elements documented with SunoDM.".into(),
            ),
            artwork_origin: Some("ai_assisted".into()),
            ai_image_service: Some("Local Tool".into()),
            human_artwork_modifications: Some(vec!["Visible disclosure added locally".into()]),
            depicts_real_person: Some(true),
            real_person_notes: Some("Documented real-person depiction".into()),
            depicts_real_event: Some(false),
            contains_trademark: Some(false),
            ..TrackPatch::default()
        },
    )
    .expect("track facts")
}

fn write_end_to_end_fixtures(directory: &Path) -> EndToEndFixtures {
    let fixture_root = directory.join("fixtures");
    fs::create_dir(&fixture_root).expect("fixture directory");
    let suno_export = fixture_root.join("suno-export.wav");
    let release_master = fixture_root.join("release-master.wav");
    let ai_original = fixture_root.join("ai-original.png");
    let release_mp3 = fixture_root.join("release-master.mp3");
    let source_code = fixture_root.join("sonic-pi-generator.rb");
    let code_audio = fixture_root.join("sonic-pi-render.wav");
    let subscription_one = fixture_root.join("subscription-one.pdf");
    let subscription_two = fixture_root.join("subscription-two.pdf");
    let terms_source = fixture_root.join("suno-terms.pdf");
    let final_audio = p0_screening_wav(Some(&p0_suno_comment("2026-08-02T06:38:06Z")), 31);
    fs::write(&suno_export, &final_audio).expect("Suno fixture");
    fs::write(&release_master, &final_audio).expect("byte-identical release fixture");
    fs::write(&release_mp3, b"ID3\x04\0\0release mp3 fixture").expect("release MP3 fixture");
    fs::write(
        &source_code,
        b"use_bpm 110\nlive_loop :documented_layer do\n  play 48\n  sleep 1\nend\n",
    )
    .expect("Sonic Pi source fixture");
    fs::write(&code_audio, b"RIFF\x08\0\0\0WAVEsonic pi rendered layer")
        .expect("code-generated audio fixture");
    image::RgbaImage::from_pixel(640, 640, image::Rgba([24, 48, 96, 255]))
        .save(&ai_original)
        .expect("AI artwork fixture");
    fs::write(
        &subscription_one,
        b"%PDF-1.7\n1 0 obj\n<</Type /Receipt>>\nendobj\n%%EOF\n",
    )
    .expect("first subscription fixture");
    fs::write(
        &subscription_two,
        b"%PDF-1.7\n1 0 obj\n<</Type /Receipt /Period 2>>\nendobj\n%%EOF\n",
    )
    .expect("second subscription fixture");
    fs::write(
        &terms_source,
        b"%PDF-1.7\n1 0 obj\n<</Type /Terms>>\nendobj\n%%EOF\n",
    )
    .expect("terms fixture");
    EndToEndFixtures {
        fixture_root,
        suno_export,
        release_master,
        ai_original,
        release_mp3,
        source_code,
        code_audio,
        subscription_one,
        subscription_two,
        terms_source,
    }
}

fn import_end_to_end_track_evidence(
    app: &WorkspaceApp,
    updated: &TrackDetail,
    fixtures: &EndToEndFixtures,
) {
    let suno_export = &fixtures.suno_export;
    let release_master = &fixtures.release_master;
    let ai_original = &fixtures.ai_original;
    let release_mp3 = &fixtures.release_mp3;
    let source_code = &fixtures.source_code;
    let code_audio = &fixtures.code_audio;
    app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, suno_export)
        .expect("Suno evidence import");
    let automated = app
        .import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, release_master)
        .expect("release evidence import");
    assert_eq!(automated.fields.suno_final_generation_date, "2026-08-02");
    assert_eq!(
        automated.fields.suno_final_generation_id,
        "generation-end-to-end"
    );
    assert_eq!(automated.fields.production_end_date, "2026-08-02");
    assert_eq!(automated.fields.suno_download_export_date, "2026-08-02");
    assert_eq!(
        automated.automation.final_generation_origin,
        FactOrigin::EvidenceDerivedMetadata
    );
    assert_eq!(
        automated.automation.final_generation_id_origin,
        FactOrigin::UserConfirmedFact
    );
    assert!(automated.automation.release_identical_to_suno_export);
    app.update_track(
        &updated.id,
        TrackPatch {
            release_filename_difference_confirmed: Some(true),
            suno_export_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("explicit filename difference confirmations");
    app.import_evidence_from(&updated.id, EvidenceRole::AiArtworkOriginal, ai_original)
        .expect("AI original import");
    app.import_evidence_from(&updated.id, EvidenceRole::ReleaseMp3, release_mp3)
        .expect("release MP3 import");
    app.import_evidence_from(&updated.id, EvidenceRole::SourceCodeFile, source_code)
        .expect("Sonic Pi source-code import");
    app.import_evidence_from(
        &updated.id,
        EvidenceRole::CodeGeneratedAudioFile,
        code_audio,
    )
    .expect("code-generated audio import");
    let disclosed = app
        .generate_artwork_disclosure(&updated.id, Some("AI-assisted".into()))
        .expect("local artwork disclosure")
        .track
        .expect("disclosed track");
    assert!(disclosed
        .evidence
        .iter()
        .any(|item| item.role == EvidenceRole::AiArtworkEdited && item.verified));
    let mut reordered_timestamp = disclosed
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::AiArtworkEdited)
        .expect("generated disclosure evidence")
        .clone();
    reordered_timestamp.imported_at = "2000-01-01T00:00:00Z".into();
    app.persistence
        .save_evidence(&updated.id, &reordered_timestamp)
        .expect("simulate a non-chronological import timestamp");
    let repeated = app
        .generate_artwork_disclosure(&updated.id, Some("AI-assisted".into()))
        .expect("idempotent disclosure request");
    assert!(repeated.message.contains("bereits vorhanden"));
    assert_eq!(
        repeated
            .track
            .expect("repeated disclosure track")
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::AiArtworkEdited)
            .count(),
        1
    );
    let disclosed_artwork = disclosed
        .evidence
        .iter()
        .find(|item| item.role == EvidenceRole::AiArtworkEdited)
        .expect("locally disclosed artwork");
    app.import_evidence_from(
        &updated.id,
        EvidenceRole::FinalArtwork,
        &app.root()
            .join(&disclosed.relative_path)
            .join(&disclosed_artwork.relative_path),
    )
    .expect("final artwork import from disclosed bytes");
}

fn attach_end_to_end_global_evidence(
    app: &WorkspaceApp,
    updated: &TrackDetail,
    fixtures: &EndToEndFixtures,
) {
    let subscription_one = &fixtures.subscription_one;
    let subscription_two = &fixtures.subscription_two;
    let terms_source = &fixtures.terms_source;
    let global_one = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            subscription_one,
            Some("2026-07-01".into()),
            Some("2026-08-01".into()),
        )
        .expect("first global subscription evidence");
    let global_two = app
        .register_global_evidence(
            EvidenceRole::SubscriptionPayment,
            subscription_two,
            Some("2026-08-02".into()),
            Some("2026-08-31".into()),
        )
        .expect("second global subscription evidence");
    app.attach_global_evidence(&updated.id, &global_one.evidence.id)
        .expect("first portable subscription copy");
    let portable = app
        .attach_global_evidence(&updated.id, &global_two.evidence.id)
        .expect("second portable subscription copy");
    assert_eq!(
        portable
            .evidence
            .iter()
            .filter(|item| { item.role == EvidenceRole::SubscriptionPayment })
            .count(),
        2
    );
    let terms = app
        .register_global_terms_evidence(
            terms_source,
            EvidenceMetadata {
                document_title: "Suno Terms of Service".into(),
                provider: "Suno, Inc.".into(),
                source_url: "https://suno.com/terms".into(),
                retrieval_date: "2026-08-02".into(),
                effective_date: "2026-07-01".into(),
                applicable_production_period: "2026-08-01 to 2026-08-02".into(),
                factual_note: "Offline archived terms fixture.".into(),
                ..EvidenceMetadata::default()
            },
        )
        .expect("terms evidence with core metadata");
    assert!(app
        .load_track(&updated.id)
        .expect("track with terms")
        .evidence
        .iter()
        .any(|item| {
            item.role == EvidenceRole::SunoTermsRights
                && item.source_global_evidence_id.as_deref() == Some(terms.evidence.id.as_str())
        }));
}

fn finalize_end_to_end_track(app: &WorkspaceApp, updated: &TrackDetail) -> (TrackDetail, PathBuf) {
    let generated = app
        .generate_documents(&updated.id, false)
        .expect("document generation")
        .track
        .expect("generated track detail");
    assert!(
        generated.documents.current,
        "documents were stale immediately after generation: {:?}",
        generated.documents
    );
    app.calculate_hashes(&updated.id)
        .expect("SHA-256 generation and verification");
    let validation = app.validate_track(&updated.id).expect("native gate");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
    let finalized = app.finalize_track(&updated.id).expect("finalization");
    let final_detail = finalized.track.expect("finalized track detail");
    assert_eq!(final_detail.status, TrackStatus::Finalized);
    assert!(final_detail.certificate.valid);

    let track_root = app.root().join(&final_detail.relative_path);
    for relative in [
        certificate::CERTIFICATE_FILE,
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_HASH_FILE,
        certificate::PDF_FILE,
    ] {
        assert!(track_root.join(relative).is_file(), "generated {relative}");
    }
    certificate::verify(&track_root).expect("certificate integrity");
    (final_detail, track_root)
}

fn reproduce_end_to_end_certificate(
    app: &WorkspaceApp,
    directory: &Path,
    updated: &TrackDetail,
    track_root: &Path,
) -> Vec<u8> {
    // Re-render the exact persisted snapshot in an independent root. This
    // exercises determinism across the manifest, Markdown, PDF, and their
    // certificate hash set rather than checking each renderer in isolation.
    let original_manifest_bytes =
        fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("manifest snapshot");
    let original_manifest_json: serde_json::Value =
        serde_json::from_slice(&original_manifest_bytes).expect("manifest snapshot JSON");
    let certificate_steps: Vec<StepState> =
        serde_json::from_value(original_manifest_json["steps"].clone())
            .expect("certificate workflow snapshot");
    let persisted_snapshot = app
        .persistence
        .track(&updated.id)
        .expect("persisted finalized snapshot");
    let persisted_evidence = app
        .persistence
        .evidence(&updated.id)
        .expect("persisted finalized evidence");
    let persisted_deviations = app
        .persistence
        .deviations(&updated.id)
        .expect("persisted finalized deviations");
    let reproduction_root = directory.join("certificate-reproduction");
    fs::create_dir_all(reproduction_root.join("03_DOCUMENTATION"))
        .expect("reproduction documentation directory");
    fs::copy(
        track_root.join(integrity::HASH_FILE),
        reproduction_root.join(integrity::HASH_FILE),
    )
    .expect("reproduction SHA256SUMS");
    // Artwork thumbnails are derived from the verified evidence bytes, not
    // from mutable database fields. Reproducing the same rendered snapshot
    // therefore requires the same artwork bytes in the independent root.
    for item in persisted_evidence.iter().filter(|item| {
        matches!(
            item.role,
            EvidenceRole::ArtworkSunoOriginal
                | EvidenceRole::AiArtworkOriginal
                | EvidenceRole::AiArtworkEdited
                | EvidenceRole::HumanEditedArtwork
                | EvidenceRole::FinalArtwork
                | EvidenceRole::ReleaseArtwork
        )
    }) {
        let target = reproduction_root.join(&item.relative_path);
        fs::create_dir_all(target.parent().expect("artwork reproduction parent"))
            .expect("artwork reproduction directory");
        fs::copy(track_root.join(&item.relative_path), target)
            .expect("reproduction artwork evidence");
    }
    certificate::generate(certificate::GenerationInput {
        track_root: &reproduction_root,
        track: &persisted_snapshot,
        profile: &persisted_snapshot.profile_snapshot,
        steps: &certificate_steps,
        evidence: &persisted_evidence,
        deviations: &persisted_deviations,
        certificate_id: persisted_snapshot
            .certificate
            .certificate_id
            .as_deref()
            .expect("persisted certificate ID"),
        finalized_at: persisted_snapshot
            .certificate
            .finalized_at
            .as_deref()
            .expect("persisted finalization timestamp"),
        transaction_id: "normalized-snapshot-reproduction",
        render_options: CertificateRenderOptions {
            language: persisted_snapshot.certificate.certificate_language,
            bilingual: persisted_snapshot.certificate.bilingual,
        },
    })
    .expect("re-render identical normalized snapshot");
    for relative in [
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_FILE,
        certificate::PDF_FILE,
        certificate::CERTIFICATE_HASH_FILE,
    ] {
        let original = fs::read(track_root.join(relative)).expect("original certificate artifact");
        let reproduced =
            fs::read(reproduction_root.join(relative)).expect("reproduced certificate artifact");
        let first_difference = original
            .iter()
            .zip(&reproduced)
            .position(|(left, right)| left != right)
            .unwrap_or_else(|| original.len().min(reproduced.len()));
        let textual_context = if relative.ends_with(".json") || relative.ends_with(".md") {
            let original_text = String::from_utf8_lossy(&original);
            let reproduced_text = String::from_utf8_lossy(&reproduced);
            original_text
                .lines()
                .zip(reproduced_text.lines())
                .enumerate()
                .find(|(_, (left, right))| left != right)
                .map(|(index, (left, right))| {
                    format!(
                        "; line {}: original={left:?}; reproduced={right:?}",
                        index + 1
                    )
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        assert!(
            original == reproduced,
            "same normalized snapshot produced different bytes for {relative}; first differing byte {first_difference}{textual_context}"
        );
    }
    original_manifest_bytes
}

fn assert_end_to_end_manifest(
    app: &WorkspaceApp,
    original_manifest_bytes: Vec<u8>,
) -> serde_json::Value {
    let manifest = String::from_utf8(original_manifest_bytes).expect("manifest text");
    assert!(!manifest.contains(app.root().to_string_lossy().as_ref()));
    assert!(manifest.contains("\"relative_path\": \".\""));
    assert!(manifest.contains("01_RELEASE/End To End.wav"));
    assert!(manifest.contains("02_SUNO/suno-export.wav"));
    assert!(manifest.contains("05_ARTWORK/END_TO_END_AI_ORIGINAL.png"));
    assert!(manifest.contains("05_ARTWORK/END_TO_END_AI_EDITED.png"));
    assert!(manifest.contains("05_ARTWORK/END_TO_END_FINAL.png"));
    assert!(manifest.contains("sourceGlobalEvidenceId"));
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest).expect("manifest JSON");
    assert_eq!(
        manifest_json["limitations"]["artwork_import_timestamps"].as_str(),
        Some(crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE)
    );
    assert_eq!(
        manifest_json["system_verification"]["human_edited_final_artwork_status"].as_str(),
        Some("NOT VERIFIED")
    );
    assert_eq!(
        manifest_json["schema_version"].as_u64(),
        Some(u64::from(certificate::EVIDENCE_MANIFEST_SCHEMA_VERSION))
    );
    assert_eq!(
        manifest_json["evidence_derived_metadata"]["suno_created_timestamp"].as_str(),
        Some("2026-08-02T06:38:06Z")
    );
    assert_eq!(
        manifest_json["evidence_derived_metadata"]["suno_id"].as_str(),
        Some(P0_SUNO_ID)
    );
    let manifest_suno = manifest_json["evidence"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["role"].as_str() == Some("suno_final_export"))
        })
        .expect("Suno final export manifest record");
    assert_eq!(
        manifest_suno["metadata"]["sunoId"].as_str(),
        Some(P0_SUNO_ID)
    );
    assert_eq!(
        manifest_suno["metadata"]["sunoCreatedTimestamp"].as_str(),
        Some("2026-08-02T06:38:06Z")
    );
    assert!(manifest_suno["sha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert_eq!(
        manifest_json["system_verification"]["fact_origins"]["final_suno_generation_id"].as_str(),
        Some("user_confirmed_fact")
    );
    assert_eq!(
        manifest_json["system_verification"]["fact_origins"]["final_suno_generation_date"].as_str(),
        Some("evidence_derived_metadata")
    );
    assert_eq!(
        manifest_json["system_verification"]["fact_origins"]["download_export_date"].as_str(),
        Some("evidence_derived_metadata")
    );
    assert_eq!(
        manifest_json["system_verification"]["fact_origins"]["last_editing_date"].as_str(),
        Some("user_confirmed_fact")
    );
    assert_eq!(
        manifest_json["system_verification"]["release_identical_to_suno_export"].as_bool(),
        Some(true)
    );
    assert!(manifest_json["system_verification"]["byte_identical_pairs"]
        .as_array()
        .is_some_and(|pairs| !pairs.is_empty()));
    manifest_json
}

fn assert_end_to_end_markdown_and_counts(track_root: &Path, manifest_json: &serde_json::Value) {
    let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
        .expect("Markdown certificate");
    assert!(markdown.contains("Final generation date [Evidence-derived metadata]: 2026-08-02"));
    assert!(markdown.contains("Final generation ID [User-confirmed fact]: generation-end-to-end"));
    assert!(markdown.contains("Suno Studio metadata detected: **YES**"));
    assert!(!markdown.contains("Suno project/version ID"));
    assert!(!markdown.contains("project-end-to-end-v1"));
    assert!(markdown.contains("Release identical to Suno final export: **YES**"));
    assert!(markdown.contains("Download/export date [Evidence-derived metadata]: 2026-08-02"));
    assert!(markdown.contains("Last editing date [User-confirmed fact]: 2026-08-03"));
    assert!(!markdown.contains("2026-08-02T06:38:06Z"));
    assert!(!markdown.contains(P0_SUNO_ID));
    assert!(markdown.contains("Suno plan at generation [User-confirmed fact]: Pro"));
    assert!(markdown.contains("Suno Instrumental Mode Selected [User-confirmed fact]: YES"));
    assert!(markdown.contains("Vocal Lyrics Present [Classification-derived presentation]: NO"));
    assert!(markdown.contains("Suno Generation Text Field"));
    assert!(markdown.contains("Suno Terms of Service"));
    assert!(markdown.contains("Suno, Inc."));
    assert!(markdown.contains("AI Transparency Assessment"));
    for relative in [
        "03_DOCUMENTATION/README.md",
        "03_DOCUMENTATION/AI_USAGE.md",
        "03_DOCUMENTATION/SHA256SUMS.txt",
    ] {
        assert!(track_root.join(relative).is_file(), "missing {relative}");
    }

    let source_code_records = manifest_json["evidence"]
        .as_array()
        .expect("manifest evidence")
        .iter()
        .filter(|item| item["role"].as_str() == Some("source_code_file"))
        .count();
    let code_audio_records = manifest_json["evidence"]
        .as_array()
        .expect("manifest evidence")
        .iter()
        .filter(|item| item["role"].as_str() == Some("code_generated_audio_file"))
        .count();
    let subscription_records = manifest_json["evidence"]
        .as_array()
        .expect("manifest evidence")
        .iter()
        .filter(|item| item["role"].as_str() == Some("subscription_payment"))
        .count();
    assert_eq!(source_code_records, 1);
    assert_eq!(code_audio_records, 1);
    assert_eq!(subscription_records, 2);
}

fn attach_end_to_end_timestamp_and_revision(
    app: &WorkspaceApp,
    updated: &TrackDetail,
    final_detail: &TrackDetail,
    fixtures: &EndToEndFixtures,
    track_root: &Path,
) {
    let fixture_root = &fixtures.fixture_root;
    let manifest_anchor = final_detail
        .finalization_anchors
        .iter()
        .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
        .expect("final manifest anchor")
        .clone();
    let timestamp_source = fixture_root.join("external-timestamp.json");
    fs::write(
        &timestamp_source,
        b"{\"timestamp\":\"2026-08-04T12:00:00Z\"}\n",
    )
    .expect("external timestamp fixture");
    let timestamped = app
        .attach_external_timestamp_from(
            &updated.id,
            &timestamp_source,
            ExternalTimestampInput {
                provider: "Example Timestamp Provider".into(),
                timestamp_type: TimestampType::ElectronicTimestamp,
                timestamp_value: "2026-08-04T12:00:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: manifest_anchor.sha256,
                external_reference_id: "E2E-TIMESTAMP-1".into(),
                provider_verification_url: "https://timestamp.example/e2e".into(),
                note: "Optional post-finalization evidence fixture.".into(),
            },
        )
        .expect("external timestamp attachment");
    assert_eq!(timestamped.external_timestamps.len(), 1);
    assert_eq!(
        timestamped.external_timestamps[0].referenced_hash_match,
        Some(true)
    );
    let revision = app
        .create_revision(&updated.id)
        .expect("archive complete timestamped snapshot as revision")
        .track
        .expect("new revision detail");
    assert_eq!(revision.external_timestamps.len(), 1);
    assert!(revision.external_timestamps[0].integrity_verified);
    assert!(WalkDir::new(track_root.join(".archive/revisions"))
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|entry| entry.file_name() == "TIMESTAMP_RECORD.json"));
}
