use super::*;

#[test]
fn legacy_finalized_snapshot_uses_stable_fallback_for_automatic_timestamp_attachment() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Legacy Timestamp Snapshot",
    );
    let certificate_id = finalized
        .certificate
        .certificate_id
        .clone()
        .expect("certificate ID");
    let mut legacy = app.persistence.track(&finalized.id).expect("stored track");
    legacy.certificate.finalization_snapshot_id = None;
    app.persistence.save_track(&legacy).expect("legacy state");
    let (endpoint, server) = one_shot_timestamp_server(b"malformed TSA response".to_vec());
    app.update_timestamp_settings(custom_timestamp_settings(endpoint, false))
        .expect("custom timestamp settings");

    let attached = app
        .attach_configured_external_timestamp(&finalized.id)
        .expect("provider response is archived even when verification fails");
    server.join().expect("timestamp server");

    assert_eq!(attached.status, TrackStatus::Finalized);
    assert_eq!(
        attached.external_timestamp_summary.status,
        ExternalTimestampStatus::VerificationFailed
    );
    let record = attached
        .external_timestamps
        .first()
        .expect("automatic timestamp record");
    assert_eq!(
        record
            .provider_metadata
            .as_ref()
            .expect("provider metadata")
            .referenced_revision_id,
        format!("legacy-finalization-snapshot:{certificate_id}")
    );
}

#[test]
fn automatic_provider_failure_keeps_phase_one_finalized() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserved timestamp port");
    let endpoint = format!(
        "http://{}/rfc3161",
        listener.local_addr().expect("listener address")
    );
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("automatic timestamp request");
        drop(stream); // deterministic connection failure after phase one
    });
    app.update_timestamp_settings(custom_timestamp_settings(endpoint, true))
        .expect("auto timestamp settings");

    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Auto Timestamp Failure",
    );
    server.join().expect("timestamp server");

    assert_eq!(finalized.status, TrackStatus::Finalized);
    assert!(finalized.certificate.valid);
    assert_eq!(
        finalized.external_timestamp_summary.status,
        ExternalTimestampStatus::VerificationFailed
    );
    assert!(finalized.external_timestamps.is_empty());
}

#[test]
fn manifest_tamper_records_anchor_mismatch_without_contacting_provider() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Timestamp Anchor Tamper",
    );
    let (endpoint, calls, server) = observing_timestamp_server();
    app.update_timestamp_settings(custom_timestamp_settings(endpoint, false))
        .expect("timestamp settings");
    let root = app.root().join(&finalized.relative_path);
    fs::write(
        root.join(certificate::MANIFEST_FILE),
        b"{\"tampered\":true}\n",
    )
    .expect("tamper manifest");

    let detail = app
        .attach_configured_external_timestamp(&finalized.id)
        .expect("anchor mismatch returns track state");
    server.join().expect("observer server");

    assert_eq!(
        detail.external_timestamp_summary.status,
        ExternalTimestampStatus::AnchorMismatch
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(detail.external_timestamps.is_empty());
}

#[test]
fn one_changed_audio_byte_is_reported_as_not_identical_in_every_certificate_format() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "One Byte Identity Check",
    );
    assert!(!finalized.automation.release_identical_to_suno_export);
    let suno_hash = p0_evidence(&finalized, EvidenceRole::SunoFinalExport)
        .sha256
        .as_deref()
        .expect("Suno SHA-256");
    let release_hash = p0_evidence(&finalized, EvidenceRole::ReleaseWav)
        .sha256
        .as_deref()
        .expect("release SHA-256");
    assert_ne!(suno_hash, release_hash);

    let track_root = app.root().join(&finalized.relative_path);
    let managed = fs::read_to_string(track_root.join("02_SUNO/suno_project.txt"))
        .expect("managed Suno document");
    assert!(managed.contains("Release identical to Suno final export [System verification]: NO"));
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("manifest bytes"),
    )
    .expect("manifest JSON");
    assert_eq!(
        manifest["system_verification"]["release_identical_to_suno_export"].as_bool(),
        Some(false)
    );
    let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
        .expect("Markdown certificate");
    assert!(markdown.contains("Release identical to Suno final export: **NO**"));
    let pdf = fs::read(track_root.join(certificate::PDF_FILE)).expect("certificate PDF");
    let mut warnings = Vec::new();
    let pdf =
        printpdf::PdfDocument::parse(&pdf, &printpdf::PdfParseOptions::default(), &mut warnings)
            .expect("parse certificate PDF");
    let compact_pdf_text = pdf
        .extract_text()
        .into_iter()
        .flatten()
        .collect::<String>()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    assert!(compact_pdf_text.contains("ReleaseidenticaltoSunofinalexport[Systemverification]NO"));
}

struct TimestampRevisionContext {
    finalized: TrackDetail,
    certificate_id: String,
    manifest_anchor: FinalizationAnchor,
    track_root: PathBuf,
    stable_files: BTreeMap<String, Vec<u8>>,
}

#[test]
fn external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let context = prepare_timestamp_revision_context(&app, directory.path());
    let (matching, mismatch) =
        attach_and_verify_timestamp_addenda(&app, directory.path(), &context);
    let archived_mismatch_evidence =
        assert_timestamp_revision_history(&app, &context, &matching, &mismatch);
    assert_archived_timestamp_binding(&app, &context, &mismatch, &archived_mismatch_evidence);
}

fn prepare_timestamp_revision_context(
    app: &WorkspaceApp,
    directory: &Path,
) -> TimestampRevisionContext {
    let finalized = finalize_acceptance_track(
        app,
        &directory.join("fixtures"),
        "Timestamp Revision Binding",
    );
    let certificate_id = finalized
        .certificate
        .certificate_id
        .clone()
        .expect("certificate ID");
    let manifest_anchor = finalized
        .finalization_anchors
        .iter()
        .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
        .expect("manifest anchor")
        .clone();
    let track_root = app.root().join(&finalized.relative_path);
    let excluded_anchor = track_root.join(".summary/track.json");
    let excluded_source = directory.join("timestamp-excluded.json");
    fs::write(&excluded_source, b"{\"providerRecord\":\"excluded\"}\n")
        .expect("excluded timestamp fixture");
    let excluded_error = app
        .attach_external_timestamp_from(
            &finalized.id,
            &excluded_source,
            ExternalTimestampInput {
                provider: "Example Timestamp Provider".into(),
                timestamp_type: TimestampType::ExternalIntegrityTimestamp,
                timestamp_value: "2026-08-17T11:55:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::Other,
                other_referenced_artifact: ".summary/track.json".into(),
                referenced_sha256: sha256_file(&excluded_anchor).expect("excluded anchor digest"),
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: String::new(),
            },
        )
        .expect_err("excluded post-finalization file cannot become an Other anchor");
    assert!(excluded_error.to_string().contains("phase-one SHA256SUMS"));
    assert!(app
        .persistence
        .external_timestamps(&finalized.id)
        .expect("no rejected timestamp record")
        .is_empty());
    let stable_files = [
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_FILE,
        certificate::CERTIFICATE_HASH_FILE,
        certificate::PDF_FILE,
        integrity::HASH_FILE,
    ]
    .into_iter()
    .map(|relative| {
        (
            relative.to_owned(),
            fs::read(track_root.join(relative)).expect("stable phase-one artifact"),
        )
    })
    .collect::<BTreeMap<_, _>>();
    TimestampRevisionContext {
        finalized,
        certificate_id,
        manifest_anchor,
        track_root,
        stable_files,
    }
}

fn attach_and_verify_timestamp_addenda(
    app: &WorkspaceApp,
    directory: &Path,
    context: &TimestampRevisionContext,
) -> (ExternalTimestampRecord, ExternalTimestampRecord) {
    let finalized = &context.finalized;
    let certificate_id = context.certificate_id.clone();
    let manifest_anchor = context.manifest_anchor.clone();
    let track_root = &context.track_root;
    let stable_files = &context.stable_files;
    let matching_source = directory.join("timestamp-match.json");
    fs::write(
        &matching_source,
        b"{\"providerRecord\":\"timestamp-match\"}\n",
    )
    .expect("matching timestamp fixture");
    let attached = app
        .attach_external_timestamp_from(
            &finalized.id,
            &matching_source,
            ExternalTimestampInput {
                provider: "Example Timestamp Provider".into(),
                timestamp_type: TimestampType::QualifiedElectronicTimestampUserDeclared,
                timestamp_value: "2026-08-17T12:00:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: manifest_anchor.sha256.clone(),
                external_reference_id: "EXT-REF-001".into(),
                provider_verification_url: "https://timestamp.example/verify/EXT-REF-001".into(),
                note: "Provider qualification is user-declared, not system-verified.".into(),
            },
        )
        .expect("matching timestamp attachment");
    assert_eq!(attached.external_timestamps.len(), 1);
    let matching = &attached.external_timestamps[0];
    assert_eq!(matching.certificate_id, certificate_id);
    assert_eq!(matching.referenced_hash_match, Some(true));
    assert_eq!(matching.actual_sha256, manifest_anchor.sha256);
    for relative in [
        &matching.record_relative_path,
        &matching.markdown_relative_path,
        &matching.pdf_relative_path,
        &matching.hash_list_relative_path,
    ] {
        assert!(
            track_root.join(relative).is_file(),
            "missing addendum {relative}"
        );
    }
    let addendum = fs::read_to_string(track_root.join(&matching.markdown_relative_path))
        .expect("timestamp addendum markdown");
    assert!(addendum.contains("Referenced hash match [System verification]: **YES**"));
    assert!(addendum.contains("user declared"));
    assert!(addendum.contains("does not determine any legal qualification"));

    let mismatch_source = directory.join("timestamp-mismatch.tsr");
    fs::write(&mismatch_source, b"opaque non-empty RFC3161-like fixture")
        .expect("mismatch timestamp fixture");
    let mismatched = app
        .attach_external_timestamp_from(
            &finalized.id,
            &mismatch_source,
            ExternalTimestampInput {
                provider: "Example Timestamp Provider".into(),
                timestamp_type: TimestampType::ExternalIntegrityTimestamp,
                timestamp_value: "2026-08-17T12:05:00Z".into(),
                referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                other_referenced_artifact: String::new(),
                referenced_sha256: "0".repeat(64),
                external_reference_id: String::new(),
                provider_verification_url: String::new(),
                note: String::new(),
            },
        )
        .expect("mismatching timestamp remains documented");
    assert_eq!(mismatched.external_timestamps.len(), 2);
    let mismatch = mismatched
        .external_timestamps
        .iter()
        .find(|record| record.referenced_hash_match == Some(false))
        .expect("negative hash comparison");
    let mismatch_addendum = fs::read_to_string(track_root.join(&mismatch.markdown_relative_path))
        .expect("mismatch addendum markdown");
    assert!(mismatch_addendum.contains("Referenced hash match [System verification]: **NO**"));

    for (relative, expected) in stable_files {
        assert_eq!(
            &fs::read(track_root.join(relative)).expect("phase-one artifact after addendum"),
            expected,
            "timestamp attachment changed {relative}"
        );
    }
    certificate::verify(track_root).expect("primary certificate remains valid");
    assert!(
        integrity::verify(track_root)
            .expect("primary integrity remains readable")
            .verified
    );
    (matching.clone(), mismatch.clone())
}

fn assert_timestamp_revision_history(
    app: &WorkspaceApp,
    context: &TimestampRevisionContext,
    matching: &ExternalTimestampRecord,
    mismatch: &ExternalTimestampRecord,
) -> PathBuf {
    let finalized = &context.finalized;
    let certificate_id = context.certificate_id.clone();
    let track_root = &context.track_root;
    let matching_directory = track_root
        .join(&matching.record_relative_path)
        .parent()
        .expect("timestamp record directory")
        .to_path_buf();
    let matching_evidence = matching_directory.join("TIMESTAMP_EVIDENCE.json");
    fs::write(&matching_evidence, b"tampered timestamp evidence")
        .expect("tamper timestamp sidecar only");
    let after_sidecar_tamper = app
        .load_track(&finalized.id)
        .expect("phase-one snapshot remains loadable");
    assert!(after_sidecar_tamper.certificate.valid);
    let damaged = after_sidecar_tamper
        .external_timestamps
        .iter()
        .find(|record| record.id == matching.id)
        .expect("damaged timestamp record remains visible");
    assert!(!damaged.integrity_verified);
    assert!(damaged
        .integrity_issues
        .iter()
        .any(|issue| issue.contains("evidence SHA-256")));
    assert!(after_sidecar_tamper
        .external_timestamps
        .iter()
        .find(|record| record.id == mismatch.id)
        .is_some_and(|record| record.integrity_verified));
    assert!(
        integrity::verify(track_root)
            .expect("phase-one integrity after sidecar tamper")
            .verified
    );

    let revision = app
        .create_revision(&finalized.id)
        .expect("new revision after timestamp")
        .track
        .expect("revision detail");
    assert_eq!(revision.external_timestamps.len(), 2);
    assert!(revision
        .external_timestamps
        .iter()
        .find(|record| record.id == matching.id)
        .is_some_and(|record| !record.integrity_verified));
    assert!(revision
        .external_timestamps
        .iter()
        .find(|record| record.id == mismatch.id)
        .is_some_and(|record| record.integrity_verified));
    assert!(revision.finalization_anchors.is_empty());
    let persisted = app
        .persistence
        .external_timestamps(&finalized.id)
        .expect("historical timestamp records");
    assert_eq!(persisted.len(), 2);
    assert!(persisted
        .iter()
        .all(|record| record.certificate_id == certificate_id));
    let archived_records = WalkDir::new(track_root.join(".archive/revisions"))
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_name() == "TIMESTAMP_RECORD.json")
        .count();
    assert_eq!(archived_records, 2);

    let archived_mismatch_evidence = WalkDir::new(track_root.join(".archive/revisions"))
        .into_iter()
        .filter_map(std::result::Result::ok)
        .find(|entry| {
            entry.file_name() == "TIMESTAMP_EVIDENCE.tsr"
                && entry.path().parent().and_then(Path::file_name)
                    == Some(std::ffi::OsStr::new(&mismatch.id))
        })
        .expect("archived mismatch evidence")
        .into_path();
    archived_mismatch_evidence
}

fn assert_archived_timestamp_binding(
    app: &WorkspaceApp,
    context: &TimestampRevisionContext,
    mismatch: &ExternalTimestampRecord,
    archived_mismatch_evidence: &Path,
) {
    let finalized = &context.finalized;
    let revision_root = archived_mismatch_evidence
        .ancestors()
        .find(|path| {
            path.parent().and_then(Path::file_name) == Some(std::ffi::OsStr::new("revisions"))
        })
        .expect("timestamp revision root");
    let revision_metadata = revision_root.join("revision.json");
    let original_revision_metadata =
        fs::read(&revision_metadata).expect("original revision metadata");
    let mut wrong_revision: serde_json::Value =
        serde_json::from_slice(&original_revision_metadata).expect("revision metadata JSON");
    wrong_revision["previous_certificate"]["certificateId"] =
        serde_json::Value::String("SDM-different-certificate".into());
    fs::write(
        &revision_metadata,
        serde_json::to_vec_pretty(&wrong_revision).expect("wrong revision metadata bytes"),
    )
    .expect("tamper revision certificate binding");
    let after_revision_binding_tamper = app
        .load_track(&finalized.id)
        .expect("track remains loadable after revision binding tamper");
    let wrongly_bound = after_revision_binding_tamper
        .external_timestamps
        .iter()
        .find(|record| record.id == mismatch.id)
        .expect("wrongly bound archived timestamp remains visible");
    assert!(!wrongly_bound.integrity_verified);
    assert!(wrongly_bound
        .integrity_issues
        .iter()
        .any(|issue| issue.contains("certificate ID")));
    fs::write(&revision_metadata, original_revision_metadata)
        .expect("restore revision certificate binding");
    let restored_revision_binding = app
        .load_track(&finalized.id)
        .expect("track reload after revision binding restore");
    assert!(restored_revision_binding
        .external_timestamps
        .iter()
        .find(|record| record.id == mismatch.id)
        .is_some_and(|record| record.integrity_verified));

    fs::write(
        archived_mismatch_evidence,
        b"tampered archived timestamp evidence",
    )
    .expect("tamper archived timestamp evidence");
    let after_archive_tamper = app
        .load_track(&finalized.id)
        .expect("track remains loadable after archived sidecar tamper");
    let damaged_archive = after_archive_tamper
        .external_timestamps
        .iter()
        .find(|record| record.id == mismatch.id)
        .expect("archived timestamp remains visible");
    assert!(!damaged_archive.integrity_verified);
    assert!(damaged_archive
        .integrity_issues
        .iter()
        .any(|issue| issue.contains("evidence SHA-256")));
}
#[test]
fn timestamp_publication_recovery_reconciles_pending_stages_without_adopting_orphans() {
    let directory = tempdir().expect("temporary directory");
    let workspace = directory.path().join("workspace");
    let app = WorkspaceApp::open(&workspace, true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let finalized = finalize_acceptance_track(
        &app,
        &directory.path().join("fixtures"),
        "Timestamp Publication Recovery",
    );
    let certificate_id = finalized
        .certificate
        .certificate_id
        .as_deref()
        .expect("certificate ID")
        .to_owned();
    let anchor = finalized
        .finalization_anchors
        .iter()
        .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
        .expect("manifest anchor")
        .clone();
    let track_root = app.root().join(&finalized.relative_path);
    let input = |note: &str| ExternalTimestampInput {
        provider: "Recovery Timestamp Provider".into(),
        timestamp_type: TimestampType::ExternalIntegrityTimestamp,
        timestamp_value: "2026-08-17T15:00:00Z".into(),
        referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
        other_referenced_artifact: String::new(),
        referenced_sha256: anchor.sha256.clone(),
        external_reference_id: String::new(),
        provider_verification_url: String::new(),
        note: note.into(),
    };

    let abandoned_source = directory.path().join("abandoned-timestamp.json");
    fs::write(&abandoned_source, b"{\"state\":\"staged-only\"}\n")
        .expect("abandoned timestamp source");
    let abandoned = external_timestamp::stage(
        &track_root,
        &certificate_id,
        &abandoned_source,
        input("crash before database registration"),
    )
    .expect("unregistered stage");
    let abandoned_stage = track_root
        .join(".archive/timestamp-staging")
        .join(&abandoned.record.id);
    assert!(abandoned_stage.is_dir());
    assert!(!track_root
        .join(&abandoned.record.record_relative_path)
        .exists());
    drop(app);

    let app = WorkspaceApp::open(&workspace, false).expect("clean abandoned stage recovery");
    assert!(!abandoned_stage.exists());
    assert!(app
        .persistence
        .external_timestamps(&finalized.id)
        .expect("timestamp rows after abandoned stage cleanup")
        .is_empty());

    let pending_source = directory.path().join("pending-timestamp.json");
    fs::write(&pending_source, b"{\"state\":\"database-registered\"}\n")
        .expect("pending timestamp source");
    let pending = external_timestamp::stage(
        &track_root,
        &certificate_id,
        &pending_source,
        input("crash after database registration"),
    )
    .expect("registered stage");
    app.persistence
        .save_external_timestamp(&finalized.id, &pending.record)
        .expect("register pending timestamp before simulated crash");
    let pending_stage = track_root
        .join(".archive/timestamp-staging")
        .join(&pending.record.id);
    let pending_live = track_root
        .join(&pending.record.record_relative_path)
        .parent()
        .expect("pending live directory")
        .to_path_buf();
    assert!(pending_stage.is_dir());
    assert!(!pending_live.exists());
    drop(app);

    let app = WorkspaceApp::open(&workspace, false).expect("publish registered pending stage");
    assert!(!pending_stage.exists());
    assert!(pending_live.is_dir());
    let recovered = app
        .load_track(&finalized.id)
        .expect("load recovered timestamp record");
    assert!(recovered
        .external_timestamps
        .iter()
        .find(|record| record.id == pending.record.id)
        .is_some_and(|record| record.integrity_verified));

    // Simulate the old unsafe crash window: a live sidecar without a DB
    // registration is detected explicitly and is never auto-adopted.
    let orphan_source = directory.path().join("orphan-timestamp.json");
    fs::write(&orphan_source, b"{\"state\":\"unregistered-live\"}\n")
        .expect("orphan timestamp source");
    let orphan = external_timestamp::stage(
        &track_root,
        &certificate_id,
        &orphan_source,
        input("legacy unregistered live orphan"),
    )
    .expect("orphan stage");
    external_timestamp::publish(&track_root, &orphan).expect("publish orphan fixture");
    drop(app);
    let error = WorkspaceApp::open(&workspace, false)
        .expect_err("unregistered live sidecar must block silent recovery");
    assert!(error
        .to_string()
        .contains("Unregistered external timestamp sidecar detected"));
}

#[test]
fn finalization_reports_certificate_and_snapshot_verification_progress() {
    let directory = tempdir().expect("temporary directory");
    let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
    app.update_profile(complete_profile()).expect("profile");
    let ready = prepare_ready_track(
        &app,
        &directory.path().join("fixtures"),
        "Progress Certificate",
    );
    let mut events = Vec::new();

    let finalized = app
        .finalize_track_with_progress(&ready.id, &mut |progress| events.push(progress))
        .expect("finalization with progress")
        .track
        .expect("finalized detail");

    assert!(finalized.certificate.valid);
    for stage in [
        "validating_finalization_gate",
        "collecting_final_snapshot",
        "writing_finalization_marker",
        "generating_certificate",
        "verifying_certificate",
        "verifying_final_snapshot",
        "verifying",
        "comparing_hashes",
        "saving_final_snapshot",
        "complete",
    ] {
        assert!(
            events.iter().any(|progress| progress.stage == stage),
            "missing finalization progress stage {stage}"
        );
    }
    assert!(events.iter().any(|progress| {
        progress.stage == "verifying" && progress.total_bytes > 0 && progress.current_file.is_some()
    }));
    assert!(events.last().is_some_and(|progress| {
        progress.stage == "complete"
            && progress.processed_files == finalized.integrity.verified_count
            && progress.total_files == finalized.integrity.file_count
    }));
}
