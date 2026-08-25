use super::*;

#[test]
fn missing_reference_duration_is_na_while_a_recorded_value_is_exact() {
    let mut external = AudioScreeningExternalRecord {
        screening_mode: AudioScreeningMode::MultiSample,
        reference_duration_seconds: None,
        ..AudioScreeningExternalRecord::default()
    };
    let value = |record: &AudioScreeningExternalRecord| {
        multi_sample_pdf_rows(record)
            .into_iter()
            .find(|row| row.label.starts_with("Reference duration (seconds)"))
            .map(|row| row.value)
            .expect("reference-duration row")
    };
    assert_eq!(value(&external), "N/A");
    external.reference_duration_seconds = Some(300);
    assert_eq!(value(&external), "300");
}

#[test]
fn consolidated_response_archive_requires_complete_consistent_sample_references() {
    let mut external = AudioScreeningExternalRecord {
        response_relative_path: Some("03_DOCUMENTATION/ACRCLOUD_RESPONSE.json".into()),
        response_sha256: Some(DIGEST_A.into()),
        samples: vec![
            crate::model::AudioScreeningSampleRecord::default(),
            crate::model::AudioScreeningSampleRecord::default(),
        ],
        ..AudioScreeningExternalRecord::default()
    };

    // A top-level archive alone does not prove that it consolidates every
    // sample response; explicit, consistent per-sample bindings are
    // required before individual archive rows may be suppressed.
    assert!(!response_archive_is_consolidated(&external));
    assert_eq!(
        response_archive_description(&external),
        "Top-level response archive recorded; per-sample bindings not documented"
    );

    external.samples[0].response_relative_path = external.response_relative_path.clone();
    external.samples[0].response_sha256 = external.response_sha256.clone();
    assert!(!response_archive_is_consolidated(&external));
    assert_eq!(
        response_archive_description(&external),
        "Partial consolidated response archive: 1 of 2 ACRCloud sample responses"
    );

    external.samples[1].response_relative_path = external.response_relative_path.clone();
    external.samples[1].response_sha256 = external.response_sha256.clone();
    assert!(response_archive_is_consolidated(&external));
    assert_eq!(
        response_archive_description(&external),
        "Consolidated response archive: 2 ACRCloud sample responses"
    );

    external.samples[1].response_sha256 = Some(DIGEST_B.into());
    assert!(!response_archive_is_consolidated(&external));
    assert_eq!(
        response_archive_description(&external),
        "Multiple response archives retained in the sample records"
    );

    external.response_sha256 = None;
    assert!(!response_archive_is_consolidated(&external));
}

#[test]
fn provider_summary_reports_partial_sample_metadata_instead_of_hiding_it() {
    let documented = crate::model::AudioScreeningSampleRecord {
        provider_status_code: Some(1001),
        provider_status_message: Some("No result".into()),
        provider_api_version: Some("1.0".into()),
        ..Default::default()
    };
    let external = AudioScreeningExternalRecord {
        samples: vec![
            documented.clone(),
            crate::model::AudioScreeningSampleRecord::default(),
        ],
        ..AudioScreeningExternalRecord::default()
    };

    let summary = consolidated_provider_response(&external);
    assert_eq!(summary.response, "MULTIPLE — see Technical Appendix");
    assert_eq!(summary.api_version, "MULTIPLE — see Technical Appendix");
    assert!(summary.response_is_system);
    assert!(summary.api_version_is_system);
    let german = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    assert_eq!(
        localized_table_value(german, &summary.response),
        "MEHRERE — siehe Technischer Anhang"
    );

    let same_provider = AudioScreeningExternalRecord {
        samples: vec![documented.clone(), documented.clone()],
        ..AudioScreeningExternalRecord::default()
    };
    let raw_summary = consolidated_provider_response(&same_provider);
    assert_eq!(raw_summary.response, "1001 · No result");
    assert_eq!(raw_summary.api_version, "1.0");
    assert!(!raw_summary.response_is_system);
    assert!(!raw_summary.api_version_is_system);
    let raw_row = TableRow::provider_summary(
        "Provider response",
        &raw_summary.response,
        raw_summary.response_is_system,
    );
    assert!(!raw_row.localize_value);

    let provider_match = crate::model::AudioScreeningMatch {
        title: "Catalog title".into(),
        ..Default::default()
    };
    let mut repeated = external;
    repeated.samples[0].matches.push(provider_match.clone());
    repeated.matches.push(provider_match);
    assert!(aggregate_matches_are_repeated_in_samples(&repeated));
    repeated.matches.push(crate::model::AudioScreeningMatch {
        title: "Aggregate-only title".into(),
        ..Default::default()
    });
    assert!(!aggregate_matches_are_repeated_in_samples(&repeated));
}

#[test]
fn chromaprint_integrity_fields_remain_complete_but_the_raw_fingerprint_is_absent() {
    let mut fixture = Fixture::new(1);
    let local = &mut fixture.track.audio_screening.local;
    local.status = AudioScreeningStatus::FingerprintGenerated;
    local.engine = "chromaprint".into();
    local.engine_version = "1.6.1".into();
    local.fingerprint_algorithm = "2".into();
    local.source_evidence_id = "release-audio-evidence".into();
    local.source_relative_path = "01_RELEASE/GRAVITY.wav".into();
    local.source_sha256 = DIGEST_A.into();
    local.source_size_bytes = 123_456;
    local.duration_milliseconds = Some(646_160);
    local.artifact_relative_path = "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json".into();
    local.artifact_sha256 = DIGEST_B.into();
    local.generated_at = Some("2026-01-04T10:11:12Z".into());
    local.fingerprint = "RAW_CHROMAPRINT_FINGERPRINT_MUST_NOT_RENDER".into();

    let (_, text) = parse_text(&fixture.generate());
    for expected in [
        "FINGERPRINT GENERATED",
        "chromaprint",
        "1.6.1",
        "Fingerprint algorithm [System verification]",
        "646160",
        "01_RELEASE/GRAVITY.wav",
        DIGEST_A,
        "LOCAL_FINGERPRINT.json",
        DIGEST_B,
    ] {
        assert!(
            text.contains(expected),
            "Chromaprint block omitted {expected}"
        );
    }
    assert!(!text.contains("RAW_CHROMAPRINT_FINGERPRINT_MUST_NOT_RENDER"));
}

#[test]
fn german_and_english_pdfs_share_technical_values_without_template_leakage() {
    let mut fixture = Fixture::new(6);
    fixture.revision_references =
        vec![".archive/revisions/123e4567-e89b-12d3-a456-426614174000/revision.json".into()];
    let external = &mut fixture.track.audio_screening.external;
    external.screening_mode = AudioScreeningMode::MultiSample;
    external.status = AudioScreeningStatus::NoMatchDetected;
    external.reference_duration_seconds = Some(300);
    external.planned_request_count = 11;
    external.executed_request_count = 11;
    external.request_count = 11;
    external.unique_sample_count = 11;
    external.duplicate_sample_count = 0;
    external.overlapping_sample_count = 0;
    external.unique_sample_duration_milliseconds = 129_232;
    external.track_coverage_percent = 20.00;

    let (_, english_text) = parse_text(&fixture.generate());
    fixture.render_options = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    let (_, german_text) = parse_text(&fixture.generate());
    for technical_value in [
        CERTIFICATE_ID,
        "2026-01-04T12:34:56Z",
        "Größe & Präzision",
        "Künstlerin Änne Öster",
        DIGEST_A,
        "300",
        "129232",
        "20.00",
        "123e4567-e89b-12d3-a456-426614174000",
    ] {
        assert!(english_text.contains(technical_value));
        assert!(german_text.contains(technical_value));
    }
    for german_template in ["Evidenzregister (Fortsetzung)", "Schritt", "N/A-Grund"] {
        assert!(
            !english_text.contains(german_template),
            "English PDF leaked {german_template}"
        );
    }
    for english_template in [
        "E. Human contribution",
        "Authoritative status / N/A reason",
        "Evidence Register (continued)",
    ] {
        assert!(
            !german_text.contains(english_template),
            "German PDF leaked {english_template}"
        );
    }
}

#[test]
fn rejects_contradictory_terms_availability_statements() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_terms_evidence_not_available = Some(true);
    let mut terms = evidence_item(200);
    terms.role = EvidenceRole::SunoTermsRights;
    terms.relative_path = "04_LICENSES/terms.pdf".into();
    fixture.evidence.push(terms);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let error = fixture
        .generate_result()
        .expect_err("contradictory Terms availability must not render");
    assert!(error.to_string().contains(
        "Certificate PDF cannot combine verified Terms evidence with an unavailable claim."
    ));
}

#[test]
fn rejects_contradictory_overall_screening_result_and_retains_sample_only_matches() {
    let mut fixture = Fixture::new(1);
    let provider_match = crate::model::AudioScreeningMatch {
        title: "Sample-only provider match".into(),
        artists: vec!["Sample-only artist".into()],
        ..Default::default()
    };
    fixture.track.audio_screening.external.status = AudioScreeningStatus::NoMatchDetected;
    fixture.track.audio_screening.external.screening_mode = AudioScreeningMode::MultiSample;
    fixture.track.audio_screening.external.samples =
        vec![crate::model::AudioScreeningSampleRecord {
            sequence: 1,
            status: AudioScreeningStatus::MatchDetected,
            matches: vec![provider_match],
            ..Default::default()
        }];

    let error = fixture
        .generate_result()
        .expect_err("contradictory overall/sample result must be rejected");
    assert!(error
        .to_string()
        .contains("overall result contradicts its provider match records"));

    // Historical sample records may contain the complete match while the
    // redundant aggregate list is absent. Keep that factual record rather
    // than incorrectly printing NONE RECORDED.
    fixture.track.audio_screening.external.status = AudioScreeningStatus::MatchDetected;
    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    assert!(normalized.contains("1 — recorded; complete per-sample records"));
    assert!(normalized.contains("Sample-only provider match — Sample-only artist"));
    assert!(!normalized.contains("Provider matches [Provider-derived metadata] NONE RECORDED"));
}

#[test]
fn rejects_screening_coverage_that_contradicts_documented_durations() {
    let mut fixture = Fixture::new(1);
    let external = &mut fixture.track.audio_screening.external;
    external.status = AudioScreeningStatus::NoMatchDetected;
    external.screening_mode = AudioScreeningMode::MultiSample;
    external.source_duration_milliseconds = Some(1_000);
    external.unique_sample_duration_milliseconds = 500;
    external.track_coverage_percent = 49.99;
    external.planned_request_count = 1;
    external.executed_request_count = 1;
    external.request_count = 1;
    external.unique_sample_count = 1;
    external.samples = vec![crate::model::AudioScreeningSampleRecord {
        sequence: 1,
        offset_milliseconds: 0,
        end_offset_milliseconds: 500,
        duration_milliseconds: 500,
        status: AudioScreeningStatus::NoMatchDetected,
        ..Default::default()
    }];

    let error = fixture
        .generate_result()
        .expect_err("contradictory track coverage must not render");
    assert!(error
        .to_string()
        .contains("track coverage contradicts the documented durations"));

    fixture
        .track
        .audio_screening
        .external
        .track_coverage_percent = 50.0;
    let (_, text) = parse_text(&fixture.generate());
    assert!(normalized_text(&text).contains("Track coverage (%) [System verification] 50.00"));
}

#[test]
fn renders_clean_instrumental_lyrics_statement_and_scope_boundary() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.instrumental_track = Some(true);
    fixture.track.fields.vocal_lyrics_present = Some(false);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Instrumental);
    fixture.track.fields.suno_content_classification =
        Some(SunoContentClassification::StructureOnly);
    fixture.track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Mixed);
    fixture.track.fields.suno_lyrics_field_text = "[Intro]\n[Drop]\n[Outro]".into();
    fixture.track.fields.human_editing_performed = Some(false);
    fixture.track.fields.human_editing_details.clear();
    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    let human_section = normalized
        .rfind("E. Human contribution")
        .expect("human-contribution section");
    let suno_field_section = normalized
        .rfind("F. Suno Generation Text Field")
        .expect("independent Suno-field section");
    let used_field_line = normalized
        .find("Generation Text Field Used [User-confirmed fact]: YES")
        .expect("used text field");
    let ai_section = normalized
        .rfind("G.1 AI Transparency Assessment – Audio")
        .expect("AI-transparency section");
    assert!(human_section < suno_field_section);
    assert!(suno_field_section < used_field_line);
    assert!(used_field_line < ai_section);
    assert!(!normalized[human_section..suno_field_section].contains("Content source"));
    assert!(!normalized.contains("E.1 Suno Structure / Generation Instructions"));
    assert!(text.contains("Suno Instrumental Mode Selected [User-confirmed fact]: YES"));
    assert!(text.contains("Vocal Lyrics Present [Classification-derived presentation]: NO"));
    assert!(text.contains("Final Audio Contains Vocals [User-confirmed fact]: NO"));
    assert!(
        text.contains("Structure Instructions Present [Classification-derived presentation]: YES")
    );
    assert!(text.contains("Vocal Intent [User-confirmed fact]: INSTRUMENTAL"));
    assert!(text.contains("[Intro]"));
    assert!(text.contains("does not confirm authorship"));
    assert!(!text.contains("Commercial rights confirmed"));
}

#[test]
fn structure_only_generation_text_does_not_become_vocal_lyrics_in_pdf() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.instrumental_track = Some(false);
    fixture.track.fields.vocal_lyrics_present = Some(true);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Vocal);
    fixture.track.fields.suno_content_classification =
        Some(SunoContentClassification::StructureOnly);
    fixture.track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
    fixture.track.fields.suno_lyrics_field_text = "[Intro]\n[Drop]".into();

    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("Vocal Lyrics Present [Classification-derived presentation]: NO"));
    assert!(
        text.contains("Structure Instructions Present [Classification-derived presentation]: YES")
    );
    assert!(text.contains("Final Audio Contains Vocals [User-confirmed fact]: YES"));
}

#[test]
fn renders_ai_suno_field_outside_human_contribution() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.instrumental_track = Some(true);
    fixture.track.fields.vocal_lyrics_present = Some(false);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Instrumental);
    fixture.track.fields.suno_content_classification =
        Some(SunoContentClassification::StructureOnly);
    fixture.track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Ai);
    fixture.track.fields.suno_lyrics_field_text = "[AI structure instruction]".into();
    fixture.track.fields.human_editing_performed = Some(true);
    fixture.track.fields.human_editing_details = "Manual mastering edit".into();

    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    let human_section = normalized
        .rfind("E. Human contribution")
        .expect("human-contribution section");
    let suno_field_section = normalized
        .rfind("F. Suno Generation Text Field")
        .expect("independent Suno-field section");
    let used_field_line = normalized
        .find("Generation Text Field Used [User-confirmed fact]: YES")
        .expect("AI Suno-field source");
    let ai_section = normalized
        .rfind("G.1 AI Transparency Assessment – Audio")
        .expect("AI-transparency section");

    assert!(human_section < suno_field_section);
    assert!(suno_field_section < used_field_line);
    assert!(used_field_line < ai_section);
    let human_content = &normalized[human_section..suno_field_section];
    assert!(human_content.contains("Manual mastering edit"));
    assert!(!human_content.contains("[AI structure instruction]"));
    assert!(!human_content.contains("Content source"));
    assert!(normalized[suno_field_section..ai_section].contains("[AI structure instruction]"));
}

#[test]
fn style_prompt_is_a_standalone_historical_input_not_an_audio_claim() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.generative_ai_used = Some(true);
    fixture.track.fields.suno_style_prompt = "instrumental, no vocals, no lyrics".into();
    fixture.track.fields.vocal_lyrics_present = Some(true);
    if let Some(step) = fixture
        .steps
        .iter_mut()
        .find(|step| step.id == "ai_transparency")
    {
        step.status = StepStatus::Pass;
        step.na_reason = None;
    }
    let (_, english_text) = parse_text(&fixture.generate());
    let english = normalized_text(&english_text);
    assert!(english.contains("Documented Suno style prompt"));
    assert!(english.contains("instrumental, no vocals, no lyrics"));
    assert!(english.contains("Documented input – this field records the submitted prompt and does not describe or verify the resulting audio content."));
    assert!(english.contains("Final Audio Contains Vocals [User-confirmed fact]: YES"));

    fixture.render_options = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    let (_, german_text) = parse_text(&fixture.generate());
    let german = normalized_text(&german_text);
    assert!(german.contains("Dokumentierter Suno-Style-Prompt"));
    assert!(german.contains("instrumental, no vocals, no lyrics"));
    assert!(german.contains("Dokumentierte Eingabe – dieses Feld dokumentiert den eingegebenen Prompt und stellt keine Feststellung über den tatsächlichen finalen Audioinhalt dar."));
}

#[test]
fn accepted_crlf_and_tabbed_multiline_values_render_without_mutating_snapshot_text() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_style_prompt = "first line\r\nsecond\tline".into();
    fixture.track.fields.suno_lyrics_field_text = "[Intro]\r\n\t[Drop]".into();
    let original_prompt = fixture.track.fields.suno_style_prompt.clone();
    let original_field = fixture.track.fields.suno_lyrics_field_text.clone();

    let bytes = fixture.generate();
    let (_, text) = parse_text(&bytes);
    assert!(text.contains("first line"));
    assert!(text.contains("second line"));
    assert!(text.contains("[Intro]"));
    assert!(text.contains("[Drop]"));
    assert_eq!(fixture.track.fields.suno_style_prompt, original_prompt);
    assert_eq!(fixture.track.fields.suno_lyrics_field_text, original_field);
}

#[test]
fn renders_final_generation_id_but_omits_retired_project_version_and_time() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_project_version_id = "legacy-project-version".into();
    fixture.track.fields.suno_final_generation_id = "legacy-generation-id".into();
    fixture.track.fields.suno_final_generation_time = "14:35".into();

    let (_, text) = parse_text(&fixture.generate());
    assert!(!text.contains("Suno project/version ID"));
    assert!(text.contains("Final generation ID [User-confirmed fact]"));
    assert!(!text.contains("Final generation time"));
    assert!(!text.contains("legacy-project-version"));
    assert!(text.contains("legacy-generation-id"));
    assert!(!text.contains("14:35"));
}

#[test]
fn renders_suno_automation_with_evidence_derived_id_but_without_raw_timestamp() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_final_generation_date = "2026-08-17".into();
    fixture.track.fields.suno_final_generation_id = "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into();
    let release_hash = fixture.evidence[0]
        .sha256
        .clone()
        .expect("release fixture digest");
    let mut suno = evidence_item(42);
    suno.role = EvidenceRole::SunoFinalExport;
    suno.relative_path = "02_SUNO/suno-final.wav".into();
    suno.sha256 = Some(release_hash.clone());
    suno.metadata.suno_studio_detected = true;
    suno.metadata.suno_created_timestamp = "2026-08-17T06:38:06Z".into();
    suno.metadata.suno_created_date = "2026-08-17".into();
    suno.metadata.suno_id = "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into();
    suno.metadata.suno_raw_metadata = "made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828".into();
    suno.metadata.file_extension = "wav".into();
    suno.metadata.mime_type = "audio/wav".into();
    suno.metadata.audio_format = "PCM".into();
    suno.metadata.audio_channels = Some(2);
    suno.metadata.audio_sample_rate_hz = Some(44_100);
    suno.metadata.audio_duration_milliseconds = Some(646_160);
    suno.metadata.audio_bit_depth = Some(16);
    suno.metadata.embedded_metadata = vec![crate::model::EmbeddedMetadata {
        key: "software".into(),
        value: "Suno Studio export".into(),
    }];
    fixture.track.field_origins.suno_final_generation_date =
        Some(crate::model::EvidenceDerivedField {
            value: "2026-08-17".into(),
            original_value: "2026-08-17T06:38:06Z".into(),
            evidence_id: suno.id.clone(),
            evidence_sha256: release_hash.clone(),
        });
    fixture.track.field_origins.suno_final_generation_id =
        Some(crate::model::EvidenceDerivedField {
            value: "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into(),
            original_value: "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into(),
            evidence_id: suno.id.clone(),
            evidence_sha256: release_hash,
        });
    fixture.evidence.push(suno);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let (_, text) = parse_text(&fixture.generate());

    assert!(text.contains("Final Suno Generation"));
    assert!(text.contains("2026-08-17"));
    assert!(text.contains("Evidence-derived metadata"));
    assert!(text.contains("Suno Studio metadata detected"));
    assert!(text.contains("Release identical to Suno final export"));
    assert!(text.contains("Final generation ID [Evidence-derived metadata]"));
    let section_c_start = text
        .find("C. Final Suno Generation")
        .expect("section C heading");
    let section_d_start = text[section_c_start..]
        .find("D. Source provenance")
        .map(|offset| section_c_start + offset)
        .expect("section D heading");
    assert!(!text[section_c_start..section_d_start].contains("2026-08-17T06:38:06Z"));
    assert!(text.contains("Suno created timestamp"));
    assert!(text.contains("2026-08-17T06:38:06Z"));
    for expected in [
        "File extension [Evidence-derived metadata]",
        "audio/wav",
        "Audio format [Evidence-derived metadata]",
        "Audio channels [Evidence-derived metadata]",
        "44100",
        "646160",
        "Audio bit depth [Evidence-derived metadata]",
        "Embedded metadata 1 key",
        "software",
        "Suno Studio export",
    ] {
        assert!(
            text.contains(expected),
            "evidence register omitted {expected}"
        );
    }
    assert!(text.contains("6c8a40fd-32bf-4c7b-ab59-23579ff95828"));
}
