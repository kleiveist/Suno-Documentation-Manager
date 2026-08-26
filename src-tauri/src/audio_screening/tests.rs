use super::*;
use crate::model::{EvidenceMetadata, EvidenceRole};
use std::cell::RefCell;
use std::fs;
use tempfile::tempdir;

struct FixedGetTransport {
    status: u16,
}

impl AcrCloudHttpTransport for FixedGetTransport {
    fn post(
        &self,
        _request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        panic!("connection test must not send a POST request")
    }

    fn get(
        &self,
        _request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        Ok(AcrCloudResponse {
            status: self.status,
            body: Vec::new(),
        })
    }
}

struct NoUploadTransport;

impl AcrCloudHttpTransport for NoUploadTransport {
    fn post(
        &self,
        _request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        panic!("external request must not be sent without a current local fingerprint")
    }

    fn get(
        &self,
        _request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        panic!("external screening must not run a connection test")
    }
}

struct RecordingTransport {
    posts: RefCell<Vec<AcrCloudRequest>>,
    responses: RefCell<Vec<AcrCloudResponse>>,
}

impl RecordingTransport {
    fn no_match(count: usize) -> Self {
        Self {
            posts: RefCell::new(Vec::new()),
            responses: RefCell::new(
                (0..count)
                    .map(|_| AcrCloudResponse {
                        status: 200,
                        body: br#"{"status":{"code":1001,"msg":"No result","version":"1.0"}}"#
                            .to_vec(),
                    })
                    .collect(),
            ),
        }
    }
}

impl AcrCloudHttpTransport for RecordingTransport {
    fn post(
        &self,
        request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        self.posts.borrow_mut().push(request);
        self.responses
            .borrow_mut()
            .drain(..1)
            .next()
            .ok_or(ProviderFailure {
                status: AudioScreeningStatus::ProviderUnavailable,
                message: "ACRCloud could not be reached.",
            })
    }

    fn get(
        &self,
        _request: AcrCloudRequest,
    ) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
        panic!("screening run must not issue a GET request")
    }
}

#[test]
fn acrcloud_signature_uses_the_official_canonical_string() {
    let signature = acrcloud_signature("test_key", "test_secret", "1234567890").expect("signature");
    assert_eq!(signature, "ECjW5VypntRDhMHRNLGVSuzqpCg=");
}

#[test]
fn multipart_contains_required_fields_but_never_the_access_secret() {
    let signature = acrcloud_signature("key", "secret-value", "1").expect("signature");
    let body = build_acrcloud_multipart("boundary", "key", "1", &signature, b"wav");
    let text = String::from_utf8_lossy(&body);
    for field in [
        "access_key",
        "data_type",
        "signature_version",
        "signature",
        "timestamp",
        "sample_bytes",
        "name=\"sample\"",
    ] {
        assert!(text.contains(field), "missing {field}");
    }
    assert!(!text.contains("secret-value"));
    assert!(text.contains("\r\n\r\nwav\r\n--boundary--"));
}

#[test]
fn provider_response_maps_no_match_and_match_without_legal_statuses() {
    let request_sensitive_values =
        RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
    let no_match = br#"{"status":{"code":1001,"msg":"No result","version":"1.0"}}"#;
    let parsed =
        parse_acrcloud_response(200, no_match, &request_sensitive_values).expect("no match parse");
    assert_eq!(parsed.status, AudioScreeningStatus::NoMatchDetected);
    assert_eq!(parsed.provider_status_code, Some(1001));
    assert_eq!(parsed.provider_status_message.as_deref(), Some("No result"));
    assert_eq!(parsed.provider_api_version.as_deref(), Some("1.0"));
    assert!(parsed.matches.is_empty());

    let matched = br#"{
          "status":{"code":0,"msg":"Success","version":"1.0"},
          "metadata":{"music":[{
             "title":"Track", "artists":[{"name":"Artist"}],
             "album":{"name":"Album"}, "external_ids":{"isrc":"ISRC"},
             "acrid":"acrid-1", "score":88
          }]}
        }"#;
    let parsed =
        parse_acrcloud_response(200, matched, &request_sensitive_values).expect("match parse");
    assert_eq!(parsed.status, AudioScreeningStatus::MatchDetected);
    assert_eq!(parsed.provider_status_code, Some(0));
    assert_eq!(parsed.provider_status_message.as_deref(), Some("Success"));
    assert_eq!(parsed.provider_api_version.as_deref(), Some("1.0"));
    assert_eq!(parsed.matches.len(), 1);
    assert_eq!(parsed.matches[0].title, "Track");
    assert_eq!(parsed.matches[0].score, Some(88.0));
}

#[test]
fn provider_status_text_is_bounded_and_control_safe_at_the_parse_boundary() {
    let request_sensitive_values =
        RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
    let body = serde_json::to_vec(&serde_json::json!({
        "status": {
            "code": 1001,
            "msg": format!("No\nresult\u{0} {}", "x".repeat(MAX_PROVIDER_TEXT_BYTES * 2)),
            "version": format!("1.0\t{}", "v".repeat(MAX_PROVIDER_TEXT_BYTES * 2)),
        }
    }))
    .expect("provider response JSON");

    let parsed = parse_acrcloud_response(200, &body, &request_sensitive_values)
        .expect("bounded provider status parse");
    let message = parsed.provider_status_message.expect("provider message");
    let version = parsed.provider_api_version.expect("provider API version");
    assert!(message.len() <= MAX_PROVIDER_TEXT_BYTES);
    assert!(version.len() <= MAX_PROVIDER_TEXT_BYTES);
    assert!(message.starts_with("No result "));
    assert!(version.starts_with("1.0 "));
    assert!(!message.chars().any(char::is_control));
    assert!(!version.chars().any(char::is_control));
}

#[test]
fn non_success_http_response_retains_safe_bounded_provider_status_metadata() {
    let request_sensitive_values =
        RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
    let body = br#"{"status":{"code":3003,"msg":"Request limit exceeded","version":"1.0"}}"#;
    let parsed = parse_acrcloud_response(429, body, &request_sensitive_values)
        .expect("non-success provider response");

    assert_eq!(parsed.status, status_for_http_error(429));
    assert_eq!(parsed.provider_status_code, Some(3003));
    assert_eq!(
        parsed.provider_status_message.as_deref(),
        Some("Request limit exceeded")
    );
    assert_eq!(parsed.provider_api_version.as_deref(), Some("1.0"));
    assert!(parsed.raw_response.is_some());
    assert!(parsed.matches.is_empty());
}

#[test]
fn provider_response_echoing_request_sensitive_values_is_not_retained() {
    let access_key = ["test", "access", "key"].join("-");
    let access_secret = ["test", "access", "secret"].join("-");
    let signature = acrcloud_signature(&access_key, &access_secret, "123").expect("test signature");
    let request_sensitive_values =
        RequestSensitiveValues::new(&access_key, &access_secret, &signature);
    let escaped_secret = access_secret
        .bytes()
        .map(|byte| format!("\\u{byte:04x}"))
        .collect::<String>();
    let responses = [
        serde_json::json!({
            "status": {"code": 0},
            "metadata": {"music": [{"title": format!("echo: {access_key}")}]}
        })
        .to_string(),
        format!(
            r#"{{"status":{{"code":0}},"metadata":{{"music":[{{"title":"{escaped_secret}"}}]}}}}"#
        ),
        format!(r#"{{"{signature}":"ordinary value","status":{{"code":1001,"msg":"No result"}}}}"#),
        r#"{"secret":"unrelated value","status":{"code":1001,"msg":"No result"}}"#.into(),
    ];
    for response in responses {
        let parsed = parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
            .expect("controlled response result");
        assert_eq!(parsed.status, AudioScreeningStatus::ProcessingFailed);
        assert!(parsed.matches.is_empty());
        assert!(parsed.raw_response.is_none());
    }

    // The controlled result is what the application persists.  It has no
    // provider response or parsed match text to forward to Track JSON,
    // the WebView, Markdown, a certificate, or a manifest.
    let response = serde_json::json!({
        "status": {"code": 0},
        "metadata": {"music": [{"title": format!("echo: {access_secret}")}]}
    })
    .to_string();
    let parsed = parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
        .expect("controlled response result");
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    let evidence = evidence_item();
    let mut record = external_record_base("track-1", &evidence);
    record.status = parsed.status;
    record.message = parsed.message.into();
    record.matches = parsed.matches;
    let persisted = finish_external_record(&root, record, None, Vec::new(), &mut |_, _| {})
        .expect("persist controlled response");
    assert_eq!(persisted.status, AudioScreeningStatus::ProcessingFailed);
    assert!(persisted.matches.is_empty());
    assert!(persisted.response_relative_path.is_none());
    assert!(!root.join(ACRCLOUD_RESPONSE_FILE).exists());
    let stored =
        fs::read_to_string(root.join(EXTERNAL_SCREENING_FILE)).expect("stored controlled result");
    let markdown = fs::read_to_string(root.join(AUDIO_SCREENING_MARKDOWN_FILE))
        .expect("controlled screening summary");
    for sensitive in [&access_key, &access_secret, &signature] {
        assert!(!stored.contains(sensitive));
        assert!(!markdown.contains(sensitive));
    }
}

#[test]
fn provider_response_recursively_rejects_common_credential_field_names() {
    let request_sensitive_values =
        RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
    for field in [
        "secret",
        "session_id",
        "set_cookie",
        "refresh_token",
        "id_token",
        "csrf_token",
        "jwt_claim",
    ] {
        let response = format!(
            r#"{{"status":{{"code":1001,"msg":"No result"}},"nested":{{"{field}":"unrelated value"}}}}"#
        );
        let parsed = parse_acrcloud_response(200, response.as_bytes(), &request_sensitive_values)
            .expect("controlled credential-like response");
        assert_eq!(
            parsed.status,
            AudioScreeningStatus::ProcessingFailed,
            "{field}"
        );
        assert!(parsed.matches.is_empty(), "{field}");
        assert!(parsed.raw_response.is_none(), "{field}");
    }
}

#[test]
fn non_finite_provider_score_becomes_controlled_failure_before_publication() {
    let request_sensitive_values =
        RequestSensitiveValues::new("request-key", "request-secret", "request-signature");
    for response in [
        br#"{
              "status":{"code":0},
              "metadata":{"music":[{"title":"Track","score":"NaN"}]}
            }"# as &[u8],
        br#"{
              "status":{"code":0},
              "metadata":{"music":[{"score":"NaN"}]}
            }"#,
    ] {
        let parsed = parse_acrcloud_response(200, response, &request_sensitive_values)
            .expect("controlled score result");
        assert_eq!(parsed.status, AudioScreeningStatus::ProcessingFailed);
        assert!(parsed.matches.is_empty());
        assert!(parsed.raw_response.is_none());
    }

    // A defensive direct-publication check happens before archival, so a
    // malformed in-memory record cannot partially replace current output.
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    ensure_screening_directory(&root).expect("screening directory");
    fs::write(root.join(EXTERNAL_SCREENING_FILE), b"previous result")
        .expect("previous external result");
    let evidence = evidence_item();
    let mut record = external_record_base("track-1", &evidence);
    record.matches.push(AudioScreeningMatch {
        title: "Track".into(),
        artists: Vec::new(),
        album: None,
        isrc: None,
        acrid: None,
        score: Some(f64::NAN),
    });
    assert!(publish_external_screening_artifacts(&root, None, &mut record, Vec::new()).is_err());
    assert_eq!(
        fs::read(root.join(EXTERNAL_SCREENING_FILE)).expect("unchanged previous result"),
        b"previous result"
    );
    assert!(!root.join(".archive/audio-screening").exists());
}

#[test]
fn configuration_status_distinguishes_disabled_not_configured_and_ready() {
    let mut settings = AudioScreeningSettings::default();
    assert_eq!(
        provider_configuration_status(&settings, false).0,
        AudioScreeningProviderStatus::Disabled
    );
    settings.enabled = true;
    settings.host = "identify-eu-west-1.acrcloud.com".into();
    assert_eq!(
        provider_configuration_status(&settings, false).0,
        AudioScreeningProviderStatus::NotConfigured
    );
    assert_eq!(
        provider_configuration_status(&settings, true).0,
        AudioScreeningProviderStatus::Ready
    );
    settings.host = "http://127.0.0.1:8080".into();
    assert_eq!(
        provider_configuration_status(&settings, true).0,
        AudioScreeningProviderStatus::ConfigurationInvalid
    );
}

#[test]
fn coverage_settings_reject_out_of_range_values() {
    let mut settings = AudioScreeningSettings {
        intensity_percent: 0,
        ..AudioScreeningSettings::default()
    };
    assert!(validate_acrcloud_sampling_settings(&settings).is_err());
    settings.intensity_percent = 101;
    assert!(validate_acrcloud_sampling_settings(&settings).is_err());
    settings.intensity_percent = 25;
    settings.reference_duration_seconds = 0;
    assert!(validate_acrcloud_sampling_settings(&settings).is_err());
    settings.reference_duration_seconds = 86_401;
    assert!(validate_acrcloud_sampling_settings(&settings).is_err());
    settings.reference_duration_seconds = 300;
    assert!(validate_acrcloud_sampling_settings(&settings).is_ok());
}

#[test]
fn missing_historical_reference_duration_is_documented_as_not_available() {
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    ensure_screening_directory(&root).expect("screening directory");
    let evidence = evidence_item();
    let mut external = external_record_base("track-1", &evidence);

    assert_eq!(external.reference_duration_seconds, None);
    publish_screening_markdown(&root, None, Some(&external)).expect("legacy markdown");
    let legacy = fs::read_to_string(root.join(AUDIO_SCREENING_MARKDOWN_FILE))
        .expect("legacy screening markdown");
    assert!(legacy.contains("- Reference duration: N/A"));

    external.reference_duration_seconds = Some(300);
    publish_screening_markdown(&root, None, Some(&external)).expect("current markdown");
    let current = fs::read_to_string(root.join(AUDIO_SCREENING_MARKDOWN_FILE))
        .expect("current screening markdown");
    assert!(current.contains("- Reference duration: 300 seconds"));
}

#[test]
fn connection_test_only_marks_expected_http_statuses_as_reachable() {
    let settings = configured_provider_settings();
    for status in [200, 204, 401, 403, 405] {
        let result =
            test_acrcloud_provider_with_transport(&settings, true, &FixedGetTransport { status });
        assert_eq!(
            result.status,
            AudioScreeningProviderStatus::Ready,
            "{status}"
        );
    }
    for status in [400, 404, 301, 302, 418] {
        let result =
            test_acrcloud_provider_with_transport(&settings, true, &FixedGetTransport { status });
        assert_eq!(
            result.status,
            AudioScreeningProviderStatus::ConfigurationInvalid,
            "{status}"
        );
    }
    for status in [429, 500, 503] {
        let result =
            test_acrcloud_provider_with_transport(&settings, true, &FixedGetTransport { status });
        assert_eq!(
            result.status,
            AudioScreeningProviderStatus::ProviderUnavailable,
            "{status}"
        );
    }
}

#[test]
fn adapter_does_not_upload_without_a_current_local_fingerprint() {
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    let evidence = evidence_item();
    let result = run_external_audio_screening_with_transport(
        ExternalAudioScreeningRequest {
            settings: &configured_provider_settings(),
            credentials: Some(("access-key", "access-secret")),
            source_path: &root.join("01_RELEASE/release.wav"),
            track_id: "track-1",
            evidence: &evidence,
            track_root: &root,
            local: None,
        },
        no_progress,
        &NoUploadTransport,
    )
    .expect("controlled no-upload result");
    assert_eq!(result.status, AudioScreeningStatus::ProcessingFailed);
    assert_eq!(result.request_count, 0);
    assert!(result.matches.is_empty());
    assert!(result.response_relative_path.is_none());
    assert!(!root.join(ACRCLOUD_RESPONSE_FILE).exists());
}

#[test]
fn wav_extraction_is_bounded_deterministic_and_does_not_change_source() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("release.wav");
    let original = pcm_wave(48_000, 2, 16, 30);
    fs::write(&source, &original).expect("write WAV");
    let extracted = extract_bounded_pcm_wav_sample(&source).expect("extract sample");
    assert_eq!(fs::read(&source).expect("read original"), original);
    assert_eq!(extracted.duration_milliseconds, 12_000);
    assert_eq!(extracted.offset_milliseconds, 9_000);
    assert_eq!(extracted.source_duration_milliseconds, 30_000);
    assert!(extracted.bytes.starts_with(b"RIFF"));
    assert!(extracted.bytes.len() < original.len());
}

#[test]
fn wav_extraction_reserves_riff_overhead_inside_the_provider_cap() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("release.wav");
    // 8-bit mono permits an odd data payload, exercising the extra RIFF
    // pad byte as well as the fixed header reservation.
    fs::write(&source, pcm_wave(1_000_000, 1, 8, 5)).expect("write large WAV");
    let extracted = extract_bounded_pcm_wav_sample(&source).expect("extract bounded sample");
    assert!(extracted.bytes.len() <= MAX_SAMPLE_AUDIO_BYTES as usize);
    assert_eq!(extracted.bytes.len(), MAX_SAMPLE_AUDIO_BYTES as usize);
}

#[test]
fn sampling_plan_is_deterministic_evenly_distributed_and_hard_capped() {
    let settings = AudioScreeningSettings {
        intensity_percent: 25,
        dynamic_by_track_duration: true,
        ..AudioScreeningSettings::default()
    };
    let first = plan_acrcloud_sample_ranges(600_000, &settings);
    let second = plan_acrcloud_sample_ranges(600_000, &settings);
    assert_eq!(first, second);
    assert_eq!(first.target_duration_milliseconds, 150_000);
    assert_eq!(first.requested_request_count, 13);
    assert_eq!(first.planned_request_count, 13);
    assert_eq!(first.maximum_unique_duration_milliseconds, 150_000);
    assert_eq!(
        first
            .samples
            .first()
            .map(|sample| sample.offset_milliseconds),
        Some(0)
    );
    assert_eq!(
        first
            .samples
            .last()
            .map(|sample| sample.end_offset_milliseconds),
        Some(600_000)
    );
    assert_eq!(
        first
            .samples
            .last()
            .map(|sample| sample.duration_milliseconds),
        Some(6_000)
    );
    assert_non_overlapping_millisecond_ranges(&first.samples, 600_000);

    let maximum = plan_acrcloud_sample_ranges(
        7_200_000,
        &AudioScreeningSettings {
            intensity_percent: 100,
            ..AudioScreeningSettings::default()
        },
    );
    assert_eq!(maximum.planned_request_count, MAX_ACRCLOUD_REQUESTS);
    assert_eq!(
        maximum.maximum_unique_duration_milliseconds,
        MAX_ACRCLOUD_UNIQUE_SAMPLE_SECONDS * 1_000
    );
    assert_non_overlapping_millisecond_ranges(&maximum.samples, 7_200_000);
}

#[test]
fn sampling_plan_caps_fixed_reference_at_track_and_handles_short_tracks() {
    let fixed = AudioScreeningSettings {
        intensity_percent: 25,
        dynamic_by_track_duration: false,
        reference_duration_seconds: 300,
        ..AudioScreeningSettings::default()
    };
    let capped = plan_acrcloud_sample_ranges(60_000, &fixed);
    assert_eq!(capped.target_duration_milliseconds, 60_000);
    assert_eq!(capped.planned_request_count, 5);
    assert_eq!(capped.maximum_unique_duration_milliseconds, 60_000);
    assert_non_overlapping_millisecond_ranges(&capped.samples, 60_000);

    let short = plan_acrcloud_sample_ranges(9_000, &AudioScreeningSettings::default());
    assert_eq!(short.planned_request_count, 1);
    assert_eq!(short.samples[0].offset_milliseconds, 0);
    assert_eq!(short.samples[0].duration_milliseconds, 9_000);
    assert_eq!(short.samples[0].end_offset_milliseconds, 9_000);
}

#[test]
fn external_run_archives_every_non_overlapping_multi_sample_response() {
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    let release_directory = root.join("01_RELEASE");
    fs::create_dir_all(&release_directory).expect("release directory");
    let source = release_directory.join("release.wav");
    fs::write(&source, pcm_wave(1_000, 1, 8, 600)).expect("release WAV");
    let mut evidence = evidence_item();
    evidence.sha256 = Some(sha256_file(&source).expect("release hash"));
    evidence.size_bytes = fs::metadata(&source).expect("release metadata").len();
    let mut local = local_record_base("track-1", &evidence);
    local.status = AudioScreeningStatus::FingerprintGenerated;
    local.fingerprint = "1,2,3".into();
    local.duration_milliseconds = Some(600_000);
    publish_local_screening_artifacts(&root, &mut local, None).expect("local record");
    let settings = AudioScreeningSettings {
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com".into(),
        intensity_percent: 25,
        dynamic_by_track_duration: true,
        ..AudioScreeningSettings::default()
    };
    let transport = RecordingTransport::no_match(13);
    let record = run_external_audio_screening_with_transport(
        ExternalAudioScreeningRequest {
            settings: &settings,
            credentials: Some(("access-key", "access-secret")),
            source_path: &source,
            track_id: "track-1",
            evidence: &evidence,
            track_root: &root,
            local: Some(&local),
        },
        no_progress,
        &transport,
    )
    .expect("multi-sample external screening");

    assert_eq!(record.screening_mode, AudioScreeningMode::MultiSample);
    assert_eq!(record.reference_duration_seconds, Some(300));
    assert_eq!(record.status, AudioScreeningStatus::NoMatchDetected);
    assert_eq!(record.planned_request_count, 13);
    assert_eq!(record.executed_request_count, 13);
    assert_eq!(record.request_count, 13);
    assert_eq!(record.unique_sample_count, 13);
    assert_eq!(record.duplicate_sample_count, 0);
    assert_eq!(record.overlapping_sample_count, 0);
    assert_eq!(record.unique_sample_duration_milliseconds, 150_000);
    assert!((record.track_coverage_percent - 25.0).abs() < f64::EPSILON);
    assert_eq!(record.samples.len(), 13);
    assert_eq!(transport.posts.borrow().len(), 13);
    assert_non_overlapping_sample_records(&record.samples, 600_000);
    assert!(record
        .samples
        .iter()
        .all(|sample| sample.status == AudioScreeningStatus::NoMatchDetected));
    assert!(record.samples.iter().all(|sample| {
        sample.provider_status_code == Some(1001)
            && sample.provider_status_message.as_deref() == Some("No result")
            && sample.provider_api_version.as_deref() == Some("1.0")
    }));
    assert!(record
        .samples
        .iter()
        .all(|sample| sample.duration_milliseconds <= 12_000));
    assert!(external_response_artifact_is_current(&root, &record).expect("response hash"));
    let archive: Value = serde_json::from_slice(
        &fs::read(root.join(ACRCLOUD_RESPONSE_FILE)).expect("response archive"),
    )
    .expect("structured response archive");
    let archived_samples = archive
        .get("samples")
        .and_then(Value::as_array)
        .expect("archived samples");
    assert_eq!(archived_samples.len(), 13);
    assert_eq!(archived_samples[0]["response"]["status"]["code"], 1001);
    assert_eq!(
        archived_samples[0]["response"]["status"]["msg"],
        "No result"
    );
    assert_eq!(archived_samples[0]["response"]["status"]["version"], "1.0");
    for (sample, archived) in record.samples.iter().zip(archived_samples) {
        assert_eq!(
            archived.get("offsetMilliseconds").and_then(Value::as_u64),
            Some(sample.offset_milliseconds)
        );
        assert_eq!(
            archived
                .get("endOffsetMilliseconds")
                .and_then(Value::as_u64),
            Some(sample.end_offset_milliseconds)
        );
    }
    let screening: Value = serde_json::from_slice(
        &fs::read(root.join(EXTERNAL_SCREENING_FILE)).expect("screening record"),
    )
    .expect("structured screening record");
    assert_eq!(screening["referenceDurationSeconds"], 300);
    assert_eq!(screening["samples"][0]["providerStatusCode"], 1001);
    assert_eq!(
        screening["samples"][0]["providerStatusMessage"],
        "No result"
    );
    assert_eq!(screening["samples"][0]["providerApiVersion"], "1.0");
    let screening_markdown =
        fs::read_to_string(root.join(AUDIO_SCREENING_MARKDOWN_FILE)).expect("screening markdown");
    assert!(screening_markdown.contains("Provider Code: 1001"));
    assert!(screening_markdown.contains("Provider Message: No result"));
    assert!(screening_markdown.contains("Provider Version: 1.0"));
    let mut mismatched_record = record.clone();
    mismatched_record.samples[0].offset_milliseconds += 1;
    assert!(
        !external_response_artifact_is_current(&root, &mismatched_record)
            .expect("mismatched archive is rejected")
    );
}

#[test]
fn stale_helpers_never_leave_a_previous_result_current() {
    let mut state = AudioScreeningState::default();
    state.local.status = AudioScreeningStatus::FingerprintGenerated;
    state.local.fingerprint = "1,2,3".into();
    state.external.status = AudioScreeningStatus::NoMatchDetected;
    state.external.response_relative_path = Some(ACRCLOUD_RESPONSE_FILE.into());
    state.external.response_sha256 = Some("b".repeat(64));
    mark_screening_stale(&mut state);
    assert_eq!(state.local.status, AudioScreeningStatus::Stale);
    assert_eq!(state.external.status, AudioScreeningStatus::Stale);
    assert!(state.external.response_relative_path.is_none());
    assert!(state.external.response_sha256.is_none());
}

#[test]
fn positive_local_record_requires_duration_and_expected_track_binding() {
    let evidence = evidence_item();
    let mut record = local_record_base("track-a", &evidence);
    record.status = AudioScreeningStatus::FingerprintGenerated;
    record.fingerprint = "1,2,3".into();
    record.artifact_sha256 = "c".repeat(64);
    assert!(!local_record_matches_source(&record, "track-a", &evidence));
    record.duration_milliseconds = Some(1);
    assert!(local_record_matches_source(&record, "track-a", &evidence));
    assert!(!local_record_matches_source(&record, "track-b", &evidence));

    let mut external = external_record_base("track-a", &evidence);
    external.status = AudioScreeningStatus::NoMatchDetected;
    assert!(external_record_matches_source(
        &external, "track-a", &evidence
    ));
    assert!(!external_record_matches_source(
        &external, "track-b", &evidence
    ));
}

#[test]
fn fpcalc_output_requires_a_positive_finite_duration() {
    for output in [
        &br#"{"fingerprint":"1,2,3"}"#[..],
        &br#"{"fingerprint":"1,2,3","duration":0}"#[..],
        &br#"{"fingerprint":"1,2,3","duration":-1}"#[..],
        &br#"{"fingerprint":"1,2,3","duration":"NaN"}"#[..],
    ] {
        assert!(matches!(
            parse_fpcalc_output(output),
            Err(FpcalcFailure::ProcessingFailed)
        ));
    }
    let parsed = parse_fpcalc_output(br#"{"fingerprint":"1,2,3","duration":1.25}"#)
        .expect("valid fpcalc output");
    assert_eq!(parsed.duration_milliseconds, 1_250);
}

#[test]
fn archive_current_screening_artifacts_moves_the_complete_directory() {
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    let current = root.join(AUDIO_SCREENING_DIR);
    fs::create_dir_all(current.join("nested")).expect("current screening directory");
    fs::write(current.join("LOCAL_FINGERPRINT.json"), b"local").expect("local artifact");
    fs::write(current.join("nested/extra.txt"), b"nested").expect("nested artifact");

    assert!(archive_current_screening_artifacts(&root).expect("archive current directory"));
    assert!(!current.exists());
    let archive_root = root.join(".archive/audio-screening");
    let archive = fs::read_dir(&archive_root)
        .expect("archive entries")
        .next()
        .expect("archive entry")
        .expect("archive path")
        .path()
        .join("AUDIO_SCREENING");
    assert_eq!(
        fs::read(archive.join("LOCAL_FINGERPRINT.json")).expect("local"),
        b"local"
    );
    assert_eq!(
        fs::read(archive.join("nested/extra.txt")).expect("nested"),
        b"nested"
    );
    assert!(!archive_current_screening_artifacts(&root).expect("no current directory"));
}

#[cfg(unix)]
#[test]
fn archive_current_screening_artifacts_rejects_a_symlinked_directory() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().expect("temporary directory");
    let target = tempdir().expect("symlink target");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    fs::create_dir_all(root.join("03_DOCUMENTATION")).expect("documentation parent");
    symlink(target.path(), root.join(AUDIO_SCREENING_DIR)).expect("screening symlink");
    assert!(matches!(
        archive_current_screening_artifacts(&root),
        Err(AppError::Symlink(_))
    ));
}

#[test]
fn local_record_artifact_uses_detached_self_hash() {
    let directory = tempdir().expect("temporary directory");
    let root = fs::canonicalize(directory.path()).expect("canonical root");
    let evidence = evidence_item();
    let mut record = local_record_base("track-1", &evidence);
    record.status = AudioScreeningStatus::FingerprintGenerated;
    record.fingerprint = "1,2,3".into();
    publish_local_screening_artifacts(&root, &mut record, None).expect("publish");
    assert!(local_artifact_is_current(&root, &record).expect("validate artifact"));
    let hash_file = root.join(LOCAL_FINGERPRINT_HASH_FILE);
    assert!(fs::read_to_string(hash_file)
        .expect("hash file")
        .contains(&record.artifact_sha256));
}

fn assert_non_overlapping_millisecond_ranges(
    ranges: &[AcrCloudSampleRange],
    track_duration_milliseconds: u64,
) {
    let mut previous_end = 0_u64;
    for range in ranges {
        assert!(range.duration_milliseconds > 0);
        assert!(range.duration_milliseconds <= 12_000);
        assert!(range.offset_milliseconds >= previous_end);
        assert_eq!(
            range.end_offset_milliseconds,
            range.offset_milliseconds + range.duration_milliseconds
        );
        assert!(range.end_offset_milliseconds <= track_duration_milliseconds);
        previous_end = range.end_offset_milliseconds;
    }
}

fn assert_non_overlapping_sample_records(
    samples: &[AudioScreeningSampleRecord],
    track_duration_milliseconds: u64,
) {
    let mut previous_end = 0_u64;
    for sample in samples {
        assert!(sample.duration_milliseconds > 0);
        assert!(sample.duration_milliseconds <= 12_000);
        assert!(sample.offset_milliseconds >= previous_end);
        assert_eq!(
            sample.end_offset_milliseconds,
            sample.offset_milliseconds + sample.duration_milliseconds
        );
        assert!(sample.end_offset_milliseconds <= track_duration_milliseconds);
        previous_end = sample.end_offset_milliseconds;
    }
}

fn evidence_item() -> EvidenceItem {
    EvidenceItem {
        id: "release-1".into(),
        role: EvidenceRole::ReleaseWav,
        file_name: "release.wav".into(),
        relative_path: "01_RELEASE/release.wav".into(),
        sha256: Some("a".repeat(64)),
        size_bytes: 123,
        imported_at: "2026-01-01T00:00:00Z".into(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: Default::default(),
        derived_from_evidence_id: None,
        generator_version: None,
        generated_disclosure_text: None,
        metadata: EvidenceMetadata::default(),
    }
}

fn configured_provider_settings() -> AudioScreeningSettings {
    AudioScreeningSettings {
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com".into(),
        ..AudioScreeningSettings::default()
    }
}

fn no_progress(_: &str, _: &str) {}

fn pcm_wave(sample_rate: u32, channels: u16, bit_depth: u16, seconds: u32) -> Vec<u8> {
    let block_align = channels * (bit_depth / 8);
    let byte_rate = sample_rate * u32::from(block_align);
    let samples = vec![0x7f_u8; (byte_rate * seconds) as usize];
    build_pcm_wav(
        PcmFormat {
            channels,
            sample_rate,
            byte_rate,
            block_align,
            bit_depth,
        },
        &samples,
    )
    .expect("PCM WAV")
}
