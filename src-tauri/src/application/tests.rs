use super::*;
use crate::model::{
    CertificateLanguage, CustomRfc3161Settings, DocumentationAnswer, EmbeddedMetadata, FactOrigin,
    FinalizeOptions, SunoLyricsContentSource, SunoLyricsContentType, TimestampProviderKind,
    TimestampProviderMetadata, TimestampReferencedArtifact, TimestampSettings, TimestampType,
    VocalIntent,
};
use crate::workflow::CoverageStatus;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[derive(Debug)]
struct ParsedCertificate {
    fields: BTreeMap<String, String>,
    completed_steps: BTreeMap<String, String>,
    na_reasons: BTreeMap<String, String>,
}
fn complete_profile() -> Profile {
    Profile {
        artist_name: "Acceptance Artist".into(),
        suno_profile_name: "acceptance-profile".into(),
        suno_handle: "@acceptance".into(),
        suno_plan: "Pro".into(),
        subscription_start_date: "2026-01-01".into(),
        default_commercial_use: false,
        default_ai_image_service: "Local Tool".into(),
        artwork_transparency_policy: "always".into(),
        disclosure_text: "AI-assisted".into(),
        certificate_language: CertificateLanguage::En,
    }
}

fn source_file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .expect("relative source file")
                .to_string_lossy()
                .replace('\\', "/");
            (
                relative,
                fs::read(entry.path()).expect("source fixture bytes"),
            )
        })
        .collect()
}
fn prepare_ready_track(app: &WorkspaceApp, fixture_root: &Path, title: &str) -> TrackDetail {
    let created = app
        .create_track(CreateTrackInput {
            title: title.into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended: false,
            library: TrackLibraryPlacement::default(),
        })
        .expect("track creation");
    let updated = app
        .update_track(
            &created.id,
            TrackPatch {
                suno_model: Some("v4.5".into()),
                suno_project_url: Some("https://suno.com/song/acceptance-track".into()),
                suno_project_version_id: Some("acceptance-project-version".into()),
                suno_final_generation_id: Some("acceptance-generation".into()),
                suno_final_generation_date: Some("2026-08-02".into()),
                suno_download_export_date: Some("2026-08-03".into()),
                suno_plan_at_generation: Some("Pro".into()),
                production_end_date: Some("2026-08-03".into()),
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
                generative_ai_used: Some(true),
                audio_ai_system: Some("Suno".into()),
                ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
                ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
                real_person_voice_intentionally_imitated: Some(DocumentationAnswer::No),
                real_person_identity_intentionally_represented: Some(DocumentationAnswer::No),
                real_event_represented_as_authentic_recording: Some(DocumentationAnswer::No),
                real_location_institution_event_presented_as_authentic_ai_recording: Some(
                    DocumentationAnswer::No,
                ),
                audio_disclosure_applied: Some(DocumentationAnswer::No),
                audio_disclosure_reason: Some(
                    "User deliberately recorded that no disclosure was applied.".into(),
                ),
                artwork_origin: Some("none".into()),
                ..TrackPatch::default()
            },
        )
        .expect("complete track facts");
    fs::create_dir_all(fixture_root).expect("fixture directory");
    let suno_export = fixture_root.join("suno-export.wav");
    let release_master = fixture_root.join("release-master.wav");
    // The current workflow requires a genuine local Chromaprint result.
    // Use non-trivial PCM fixtures so the bundled engine can produce one;
    // the two deterministic signals intentionally differ.
    let suno_bytes = p0_screening_wav(None, 17);
    let release_bytes = p0_screening_wav(None, 23);
    fs::write(&suno_export, &suno_bytes).expect("Suno fixture");
    fs::write(&release_master, &release_bytes).expect("one-byte-different release fixture");
    app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
        .expect("Suno evidence import");
    app.import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
        .expect("release evidence import");
    app.update_track(
        &updated.id,
        TrackPatch {
            suno_export_filename_difference_confirmed: Some(true),
            release_filename_difference_confirmed: Some(true),
            ..TrackPatch::default()
        },
    )
    .expect("confirm intentional fixture filename deviations");
    app.generate_documents(&updated.id, false)
        .expect("document generation");
    let ready = app
        .calculate_hashes(&updated.id)
        .expect("SHA-256 generation")
        .track
        .expect("ready track detail");
    let validation = app.validate_track(&updated.id).expect("native gate");
    assert!(
        validation.valid,
        "missing={:?}; blocking={:?}",
        validation.missing_items, validation.blocking_items
    );
    ready
}

fn finalize_acceptance_track(app: &WorkspaceApp, fixture_root: &Path, title: &str) -> TrackDetail {
    let ready = prepare_ready_track(app, fixture_root, title);
    app.finalize_track(&ready.id)
        .expect("finalization")
        .track
        .expect("finalized track detail")
}
fn custom_timestamp_settings(endpoint: String, auto_after_finalization: bool) -> TimestampSettings {
    TimestampSettings {
        enabled: true,
        provider: TimestampProviderKind::CustomRfc3161,
        auto_after_finalization,
        custom: CustomRfc3161Settings {
            provider_name: "Deterministic test TSA".into(),
            endpoint,
            ca_certificate_path: "unused-test-trust-anchor.der".into(),
            timeout_seconds: 2,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn one_shot_timestamp_server(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("timestamp test listener");
    let endpoint = format!(
        "http://{}/rfc3161",
        listener.local_addr().expect("listener address")
    );
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("timestamp request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request);
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/timestamp-reply\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .expect("response header");
        stream.write_all(&body).expect("response body");
    });
    (endpoint, handle)
}

fn observing_timestamp_server() -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("timestamp test listener");
    listener
        .set_nonblocking(true)
        .expect("non-blocking listener");
    let endpoint = format!(
        "http://{}/rfc3161",
        listener.local_addr().expect("listener address")
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = calls.clone();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(400);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((_stream, _)) => {
                    observed_calls.fetch_add(1, Ordering::SeqCst);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("timestamp test listener failed: {error}"),
            }
        }
    });
    (endpoint, calls, handle)
}
fn track_tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut result = BTreeMap::new();
    for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
        let entry = entry.expect("read track tree");
        let relative = entry.path().strip_prefix(root).expect("relative path");
        let key = portable_relative(relative);
        let value = if entry.file_type().is_dir() {
            b"<directory>".to_vec()
        } else {
            fs::read(entry.path()).expect("read track fixture")
        };
        result.insert(key, value);
    }
    result
}
fn certificate_file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    [
        certificate::CERTIFICATE_FILE,
        certificate::MANIFEST_FILE,
        certificate::CERTIFICATE_HASH_FILE,
        certificate::PDF_FILE,
    ]
    .into_iter()
    .map(|relative| {
        (
            relative.to_owned(),
            fs::read(root.join(relative)).expect("read certificate file"),
        )
    })
    .collect()
}

fn parse_certificate_document(content: &str) -> ParsedCertificate {
    enum Section {
        Fields,
        CompletedSteps,
        NaReasons,
        Other,
    }

    let mut fields = BTreeMap::new();
    let mut completed_steps = BTreeMap::new();
    let mut na_reasons = BTreeMap::new();
    let mut section = Section::Fields;
    for line in content.lines() {
        section = match line {
            "## J. Evidence register" => Section::Other,
            "## K. Integrity anchors and workflow" => Section::Fields,
            "### Mandatory steps completed" | "### K.1 Configured workflow checks" => {
                Section::CompletedSteps
            }
            "### N/A steps with reasons" => Section::NaReasons,
            "### K.2 Pre-release audio screening" => Section::Other,
            "## L. Technical certificate statement" => Section::Other,
            _ => section,
        };
        if !line.starts_with("- ") {
            continue;
        }
        let entry = &line[2..];
        match section {
            Section::Fields => {
                let Some((key, value)) = entry.split_once(": ") else {
                    continue;
                };
                assert!(
                    fields
                        .insert(key.into(), unquote_certificate_value(value))
                        .is_none(),
                    "duplicate certificate field: {key}"
                );
            }
            Section::CompletedSteps => {
                let (step_id, status) = entry
                    .split_once(": ")
                    .unwrap_or_else(|| panic!("malformed completed step: {line}"));
                assert!(
                    completed_steps
                        .insert(step_id.into(), status.into())
                        .is_none(),
                    "duplicate completed step: {step_id}"
                );
            }
            Section::NaReasons if !entry.eq_ignore_ascii_case("none") => {
                let (step_id, reason) = entry
                    .split_once(" — ")
                    .unwrap_or_else(|| panic!("malformed N/A reason: {line}"));
                assert!(
                    na_reasons.insert(step_id.into(), reason.into()).is_none(),
                    "duplicate N/A step: {step_id}"
                );
            }
            Section::Other if entry.starts_with("Evidence file count: ") => {
                let (key, value) = entry
                    .split_once(": ")
                    .expect("evidence count certificate field");
                fields.insert(key.into(), unquote_certificate_value(value));
            }
            Section::NaReasons | Section::Other => {}
        }
    }
    ParsedCertificate {
        fields,
        completed_steps,
        na_reasons,
    }
}

fn unquote_certificate_value(value: &str) -> String {
    value
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .or_else(|| {
            value
                .strip_prefix("**")
                .and_then(|value| value.strip_suffix("**"))
        })
        .unwrap_or(value)
        .replace('`', "")
}

fn parse_sha256sums(content: &str) -> BTreeMap<String, String> {
    content
        .lines()
        .map(|line| {
            let (digest, relative) = line
                .split_once("  ")
                .unwrap_or_else(|| panic!("malformed SHA256SUMS entry: {line}"));
            assert_eq!(digest.len(), 64, "SHA-256 digest length for {relative}");
            assert!(
                digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "non-hex SHA-256 digest for {relative}"
            );
            (relative.into(), digest.to_ascii_lowercase())
        })
        .collect()
}

fn track_record_without_library_path(track: &TrackRecord) -> serde_json::Value {
    let mut value = serde_json::to_value(track).expect("serializable track record");
    let object = value.as_object_mut().expect("track record object");
    object.remove("library");
    object.remove("relativePath");
    value
}

const P0_SUNO_ID: &str = "6c8a40fd-32bf-4c7b-ab59-23579ff95828";
const P0_SECOND_SUNO_ID: &str = "7d9b51ae-43cf-4d8c-bc6a-3468a00a6929";
const P0_THIRD_SUNO_ID: &str = "8ea062bf-54d0-4e9d-cd7b-4579b11b7a3a";
const P0_SUNO_MARKER_ALIAS_ID: &str = "c18284d0-9b50-40ca-bece-0362fe7c82dd";

fn manifest_string<'a>(manifest: &'a serde_json::Value, pointer: &str) -> &'a str {
    manifest
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("manifest field {pointer} must be a string"))
}
fn p0_suno_comment(timestamp: &str) -> String {
    p0_suno_comment_with_id(timestamp, P0_SUNO_ID)
}

fn p0_suno_comment_with_id(timestamp: &str, id: &str) -> String {
    format!("made with suno studio; created={timestamp}; id={id}")
}

fn p0_suno_marker_alias_comment(timestamp: &str, id: &str) -> String {
    format!("made with suno; created={timestamp}; id={id}")
}

/// Test-only RIFF encoder. It intentionally does not call any production
/// parser or fixture utility, so malformed parser offsets cannot be masked
/// by an encoder sharing the same implementation.
fn p0_pcm_wav(comment: Option<&str>) -> Vec<u8> {
    let entries = comment
        .map(|value| vec![(*b"ICMT", value.as_bytes().to_vec())])
        .unwrap_or_default();
    p0_pcm_wav_with_info_entries(&entries)
}

/// A longer, non-silent PCM fixture for tests that deliberately exercise
/// the real bundled Chromaprint sidecar. Keep the regular P0 WAV tiny so
/// metadata-parser tests retain their existing 10 ms assertions.
fn p0_screening_wav(comment: Option<&str>, seed: u8) -> Vec<u8> {
    let entries = comment
        .map(|value| vec![(*b"ICMT", value.as_bytes().to_vec())])
        .unwrap_or_default();
    let frames = 48_000_usize * 4;
    let mut audio = Vec::with_capacity(frames * 4);
    for frame in 0..frames {
        let phase = (frame * (17 + usize::from(seed))) % 109;
        let sample = ((phase as i32 * 2 - 108) * 220) as i16;
        for _ in 0..2 {
            audio.extend_from_slice(&sample.to_le_bytes());
        }
    }
    p0_pcm_wav_with_info_entries_and_audio(&entries, &audio)
}

fn p0_pcm_wav_with_info_entries(entries: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    p0_pcm_wav_with_info_entries_and_audio(entries, &vec![0; 1_920])
}

fn p0_pcm_wav_with_info_entries_and_audio(entries: &[([u8; 4], Vec<u8>)], audio: &[u8]) -> Vec<u8> {
    fn append_chunk(destination: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
        destination.extend_from_slice(id);
        destination.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        destination.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            destination.push(0);
        }
    }

    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1_u16.to_le_bytes());
    fmt.extend_from_slice(&2_u16.to_le_bytes());
    fmt.extend_from_slice(&48_000_u32.to_le_bytes());
    fmt.extend_from_slice(&192_000_u32.to_le_bytes());
    fmt.extend_from_slice(&4_u16.to_le_bytes());
    fmt.extend_from_slice(&16_u16.to_le_bytes());

    let mut chunks = Vec::new();
    append_chunk(&mut chunks, b"fmt ", &fmt);
    append_chunk(&mut chunks, b"data", audio);
    if !entries.is_empty() {
        let mut info = b"INFO".to_vec();
        for (id, entry) in entries {
            let mut value = entry.clone();
            value.push(0);
            append_chunk(&mut info, id, &value);
        }
        append_chunk(&mut chunks, b"LIST", &info);
    }

    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(&chunks);
    wav
}

fn p0_track(
    app: &WorkspaceApp,
    title: &str,
    post_export_editing: Option<bool>,
    commercial_use_intended: bool,
) -> TrackDetail {
    if app
        .profile()
        .expect("P0 profile")
        .artist_name
        .trim()
        .is_empty()
    {
        app.update_profile(complete_profile())
            .expect("P0 profile setup");
    }
    let created = app
        .create_track(CreateTrackInput {
            title: title.into(),
            production_start_date: "2026-08-01".into(),
            commercial_use_intended,
            library: TrackLibraryPlacement::default(),
        })
        .expect("P0 track creation");
    let Some(post_export_editing) = post_export_editing else {
        return created;
    };
    app.update_track(
        &created.id,
        TrackPatch {
            post_export_editing_performed: Some(post_export_editing),
            post_export_editing_details: post_export_editing
                .then(|| "Mastering after the Suno export".into()),
            ..TrackPatch::default()
        },
    )
    .expect("P0 post-export fact")
}

fn p0_evidence(detail: &TrackDetail, role: EvidenceRole) -> &EvidenceItem {
    detail
        .evidence
        .iter()
        .find(|item| item.role == role)
        .expect("P0 evidence role")
}

#[derive(Debug, PartialEq, Eq)]
struct P0RawFinalizedRows {
    track_data_json: String,
    track_updated_at: String,
    track_status: String,
    track_workflow_version: String,
    evidence_metadata_json: String,
    evidence_imported_at: String,
    evidence_verified: i64,
    evidence_verification_error: Option<String>,
    evidence_sha256: Option<String>,
    evidence_size_bytes: i64,
}

fn p0_raw_finalized_rows(
    app: &WorkspaceApp,
    track_id: &str,
    evidence_id: &str,
) -> P0RawFinalizedRows {
    let connection = app.persistence.open().expect("raw workspace database");
    let (track_data_json, track_updated_at, track_status, track_workflow_version) = connection
        .query_row(
            "SELECT data_json,updated_at,status,workflow_version FROM tracks WHERE id=?1",
            [track_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("raw finalized track row");
    let (
        evidence_metadata_json,
        evidence_imported_at,
        evidence_verified,
        evidence_verification_error,
        evidence_sha256,
        evidence_size_bytes,
    ) = connection
        .query_row(
            "SELECT metadata_json,imported_at,verified,verification_error,sha256,size_bytes FROM evidence WHERE track_id=?1 AND id=?2",
            rusqlite::params![track_id, evidence_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("raw finalized evidence row");
    P0RawFinalizedRows {
        track_data_json,
        track_updated_at,
        track_status,
        track_workflow_version,
        evidence_metadata_json,
        evidence_imported_at,
        evidence_verified,
        evidence_verification_error,
        evidence_sha256,
        evidence_size_bytes,
    }
}

mod certificate_cross_checks;
mod disclosure_and_workflow;
mod end_to_end_certificate;
mod finalization_recovery;
mod folder_import_and_screening;
mod legacy_evidence;
mod library_moves_and_evidence;
mod p0_finalization_and_transactions;
mod p0_metadata;
mod patches_and_library_basics;
mod previews_and_mutation;
mod profile_and_global_evidence;
mod timestamps;
