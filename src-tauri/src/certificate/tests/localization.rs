use super::*;

#[test]
fn dynamic_pdf_labels_localize_numbered_terms_and_subscription_suffixes() {
    let german = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    for (english, expected) in [
        ("Coverage", "Abdeckung"),
        ("A. Technical timestamp", "A. Technischer Zeitstempel"),
        (
            "B. Provider trust and qualification",
            "B. Providervertrauen und Qualifikation",
        ),
        (
            "Qualification at timestamp [Independent trust verification]",
            "Qualifikation zum Zeitstempelzeitpunkt [Unabhängige Vertrauensprüfung]",
        ),
        (
            "Trusted List validation [Independent trust verification]",
            "Validierung der Vertrauensliste [Unabhängige Vertrauensprüfung]",
        ),
        (
            "eIDAS QUALIFIED TRUST SERVICE – VERIFIED",
            "eIDAS-QUALIFIZIERTER VERTRAUENSDIENST – VERIFIZIERT",
        ),
        (
            "Subscription evidence 1 path [System value]",
            "Abo-Evidence 1 Pfad [Systemwert]",
        ),
        (
            "Subscription evidence 1 coverage start",
            "Abo-Evidence 1 Abdeckungsbeginn",
        ),
        (
            "Terms document 1 provider/source [User-confirmed fact]",
            "Dokument zu Nutzungsbedingungen 1 Anbieter/Quelle [Vom Nutzer bestätigte Angabe]",
        ),
        (
            "Terms document 1 provenance [System value]",
            "Dokument zu Nutzungsbedingungen 1 Herkunft [Systemwert]",
        ),
    ] {
        assert_eq!(localized_certificate_label(german, english), expected);
    }
}

#[test]
fn audio_screening_manifest_and_markdown_omit_sensitive_raw_values() {
    let mut state = AudioScreeningState::default();
    state.local.status = AudioScreeningStatus::FingerprintGenerated;
    state.local.fingerprint = "RAW_CHROMAPRINT_MUST_NOT_APPEAR".into();
    state.local.message = "ACCESS_SECRET_MUST_NOT_APPEAR".into();
    state.local.artifact_relative_path =
        "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json".into();
    state.local.artifact_sha256 = DIGEST.into();
    state.external.status = AudioScreeningStatus::MatchDetected;
    state.external.message = "RAW_PROVIDER_RESPONSE_MUST_NOT_APPEAR".into();
    state
        .external
        .matches
        .extend((0..6).map(|index| crate::model::AudioScreeningMatch {
            title: format!("Provider title {index}"),
            artists: vec![format!("Provider artist {index}")],
            ..Default::default()
        }));

    let manifest = audio_screening_manifest(&state).to_string();
    let markdown = materialize_markdown_values(&audio_screening_markdown(&state));
    for forbidden in [
        "RAW_CHROMAPRINT_MUST_NOT_APPEAR",
        "ACCESS_SECRET_MUST_NOT_APPEAR",
        "RAW_PROVIDER_RESPONSE_MUST_NOT_APPEAR",
    ] {
        assert!(!manifest.contains(forbidden), "manifest leaked {forbidden}");
        assert!(!markdown.contains(forbidden), "markdown leaked {forbidden}");
    }
    assert!(manifest.contains("Provider title 5"));
    assert!(markdown.contains("Provider artist 5"));
}

fn multi_sample_screening_state() -> AudioScreeningState {
    let mut state = AudioScreeningState::default();
    let external = &mut state.external;
    external.screening_mode = AudioScreeningMode::MultiSample;
    external.status = AudioScreeningStatus::NoMatchDetected;
    external.provider_status = AudioScreeningProviderStatus::Ready;
    external.requested_intensity_percent = 25;
    external.dynamic_by_track_duration = true;
    external.reference_duration_seconds = Some(300);
    external.source_duration_milliseconds = Some(646_000);
    external.target_duration_milliseconds = 161_500;
    external.planned_request_count = 14;
    external.executed_request_count = 2;
    external.request_count = 2;
    external.unique_sample_count = 2;
    external.duplicate_sample_count = 0;
    external.overlapping_sample_count = 0;
    external.unique_sample_duration_milliseconds = 24_000;
    external.track_coverage_percent = 3.72;
    external.message = "RAW_EXTERNAL_MESSAGE_MUST_NOT_RENDER".into();
    external.samples = vec![
        crate::model::AudioScreeningSampleRecord {
            sequence: 1,
            offset_milliseconds: 30_000,
            end_offset_milliseconds: 42_000,
            duration_milliseconds: 12_000,
            status: AudioScreeningStatus::NoMatchDetected,
            message: "RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER".into(),
            provider_status_code: Some(1001),
            provider_status_message: Some("No result".into()),
            provider_api_version: Some("1.0".into()),
            matches: Vec::new(),
            response_relative_path: Some(
                "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE_01.json".into(),
            ),
            response_sha256: Some(DIGEST.into()),
        },
        crate::model::AudioScreeningSampleRecord {
            sequence: 2,
            offset_milliseconds: 95_000,
            end_offset_milliseconds: 107_000,
            duration_milliseconds: 12_000,
            status: AudioScreeningStatus::MatchDetected,
            message: "SECOND_RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER".into(),
            provider_status_code: Some(0),
            provider_status_message: Some("Success".into()),
            provider_api_version: Some("1.0".into()),
            matches: vec![crate::model::AudioScreeningMatch {
                title: "Sample match title".into(),
                artists: vec!["Sample match artist".into()],
                ..Default::default()
            }],
            response_relative_path: Some(
                "03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE_02.json".into(),
            ),
            response_sha256: Some("b".repeat(64)),
        },
    ];
    state
}

#[test]
fn multi_sample_k2_and_manifest_capture_the_same_sanitized_screening_snapshot() {
    let state = multi_sample_screening_state();
    let manifest = audio_screening_manifest(&state);
    let markdown = materialize_markdown_values(&audio_screening_markdown(&state));
    assert_eq!(
        manifest["external"]["screeningMode"],
        serde_json::Value::String("multi_sample".into())
    );
    assert_eq!(manifest["external"]["plannedRequestCount"], 14);
    assert_eq!(
        manifest["external"]["samples"][0]["offsetMilliseconds"],
        30_000
    );
    assert_eq!(
        manifest["external"]["samples"][1]["endOffsetMilliseconds"],
        107_000
    );
    assert_eq!(
        manifest["external"]["samples"][1]["matches"][0]["title"],
        "Sample match title"
    );
    assert_eq!(manifest["external"]["referenceDurationSeconds"], 300);
    assert_eq!(
        manifest["external"]["samples"][0]["providerStatusCode"],
        1001
    );
    assert_eq!(
        manifest["external"]["samples"][0]["providerStatusMessage"],
        "No result"
    );
    for expected in [
        "External screening mode [System value]: **MULTI-SAMPLE**",
        "Requested coverage [System value]: 25 %",
        "Planned requests [System value]: 14",
        "Reference duration (seconds) [System value]: 300",
        "Executed requests [System verification]: 2",
        "Unique samples [System verification]: 2",
        "Duplicate samples [System verification]: 0",
        "Overlapping samples [System verification]: 0",
        "Unique sampled duration (ms) [System verification]: 24000",
        "Track coverage (%) [System verification]: 3.72",
        "Overall result [System verification]: **NO MATCH DETECTED**",
        "Sample 01 [System verification]: Offset 30000 ms · End offset 42000 ms · Duration 12000 ms · Result NO MATCH",
        "Provider Code: 1001 · Provider Message: No result · Provider Version: 1.0",
        "Sample 02 [System verification]: Offset 95000 ms · End offset 107000 ms · Duration 12000 ms · Result MATCH DETECTED",
        "Sample match title — Sample match artist",
        "Audio-screening results are technical comparison records only.",
    ] {
        assert!(markdown.contains(expected), "K.2 omitted {expected}");
    }
    assert!(!markdown.contains("Reference duration (seconds) [System value]: N/A"));
    let german = localized_markdown_certificate(
        &format!("## K.2 Pre-release audio screening\n\n{markdown}"),
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    for expected in [
        "## K.2 Audio-Screening vor Veröffentlichung",
        "Modus des externen Screenings",
        "Angeforderte Abdeckung",
        "Referenzdauer (Sekunden) [Systemwert]: 300",
        "Doppelte Proben [Systemprüfung]: 0",
        "Überlappende Proben [Systemprüfung]: 0",
        "Probe 01",
        "Provider Code: 1001 · Provider Message: No result · Provider Version: 1.0",
        "Gesamtergebnis",
    ] {
        assert!(german.contains(expected), "German K.2 omitted {expected}");
    }
    for forbidden in [
        "RAW_EXTERNAL_MESSAGE_MUST_NOT_RENDER",
        "RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER",
        "SECOND_RAW_SAMPLE_MESSAGE_MUST_NOT_RENDER",
    ] {
        assert!(!manifest.to_string().contains(forbidden));
        assert!(!markdown.contains(forbidden));
    }
}

#[test]
fn markdown_certificate_uses_selected_language_or_both_languages() {
    let english = "# SunoDM Technical Documentation and Evidence Certificate\n\n## C. Final Suno Generation\n\n- Final generation ID [Evidence-derived metadata]: `generation-id`\n";
    let german = localized_markdown_certificate(
        english,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    assert!(german.contains("# SunoDM Technisches Dokumentations- und Evidenzzertifikat"));
    assert!(german.contains("## C. Finale Suno-Erzeugung"));
    assert!(german.contains("- ID der finalen Erzeugung [Aus Evidenzmetadaten]: `generation-id`"));
    assert!(!german.contains("## C. Final Suno Generation"));

    let bilingual = localized_markdown_certificate(
        english,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: true,
        },
    );
    assert!(bilingual.contains("# English certificate"));
    assert!(bilingual.contains("## C. Finale Suno-Erzeugung"));
    assert!(bilingual.contains("## C. Final Suno Generation"));
}

#[test]
fn markdown_localization_preserves_multiline_historical_values_with_template_words() {
    let english = format!(
        "## F. Suno Generation Text Field\n\n### Exact Generation Text Field Content\n\n{}\n",
        markdown_raw_value(
            "Final generation\nProvider response\nConfigured documentation requirements completed"
        )
    );
    let german = localized_markdown_certificate(
        &english,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );

    assert!(german.contains("## F. Suno Generierungs-Textfeld"));
    assert!(german.contains("### Exakter Text des Suno-Textfelds"));
    for historical_line in [
        "Final generation",
        "Provider response",
        "Configured documentation requirements completed",
    ] {
        assert!(
            german
                .lines()
                .any(|line| line.trim_start() == historical_line),
            "historical line was translated: {historical_line}"
        );
    }

    let fields = TrackFields {
        suno_content_classification: Some(SunoContentClassification::StructureOnly),
        suno_lyrics_field_text: "# Final generation\n- Provider response\n> Evidence Overview"
            .into(),
        ..TrackFields::default()
    };
    let german = localized_markdown_certificate(
        &suno_field_markdown(&fields),
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    for historical_line in [
        "# Final generation",
        "- Provider response",
        "> Evidence Overview",
    ] {
        assert!(german.lines().any(|line| line == historical_line));
    }

    let fields = TrackFields {
        generative_ai_used: Some(true),
        audio_ai_system: "NO".into(),
        suno_style_prompt:
            "first\r\n```\r# Final generation\n> Provider response\n- Provider response: NO\n```\nlast"
                .into(),
        ..TrackFields::default()
    };
    let marked = format!(
        "{}\n## C. Final Suno Generation\n",
        ai_audio_markdown(&fields)
    );
    let german = localized_markdown_certificate(
        &marked,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    assert!(german.contains("KI-System [Vom Nutzer bestätigte Angabe]: NO"));
    assert!(german.contains("\r\n    ```\r    # Final generation"));
    assert!(german.contains("    > Provider response"));
    assert!(german.contains("    - Provider response: NO"));
    assert!(german.contains("## C. Finale Suno-Erzeugung"));
    assert!(!german.contains(MARKDOWN_VALUE_START));
    assert!(!german.contains(MARKDOWN_VALUE_END));

    let bilingual = localized_markdown_certificate(
        &marked,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: true,
        },
    );
    assert!(bilingual.contains("# English certificate"));
    assert!(!bilingual.contains(MARKDOWN_VALUE_START));
    assert!(!bilingual.contains(MARKDOWN_VALUE_END));

    let missing = localized_markdown_certificate(
        &suno_field_markdown(&TrackFields::default()),
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    assert!(missing.contains("Exakter Text des Suno-Textfelds\n\nNICHT DOKUMENTIERT"));
}

#[test]
fn artwork_markdown_records_explicit_no_hash_match_and_timestamp_scope() {
    let fields = TrackFields {
        artwork_origin: "ai_assisted".into(),
        disclosure_applied: Some(false),
        ..TrackFields::default()
    };
    let human_edited = relationship_evidence("human-edited", EvidenceRole::HumanEditedArtwork);
    let final_artwork = relationship_evidence("final", EvidenceRole::FinalArtwork);

    let english = ai_artwork_markdown(&fields, &[human_edited, final_artwork]);
    assert!(
        english.contains("Artwork disclosure deliberately not applied [User-confirmed fact]: YES")
    );
    assert!(english.contains("BYTE-IDENTICAL / SHA-256 MATCH"));
    assert!(english.contains(crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE));

    let german = localized_markdown_certificate(
        &english,
        CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    );
    assert!(german.contains("Artwork-Hinweis bewusst nicht angewendet"));
    assert!(german.contains("BYTE-IDENTISCH / SHA-256-ÜBEREINSTIMMUNG"));
    assert!(german.contains("Import-Zeitstempel dokumentieren nur den Import in SunoDM"));
}

#[test]
fn suno_field_markdown_is_separate_from_human_contribution_for_ai_and_mixed_sources() {
    for source in [SunoLyricsContentSource::Ai, SunoLyricsContentSource::Mixed] {
        let fields = TrackFields {
            human_editing_performed: Some(false),
            suno_content_classification: Some(SunoContentClassification::StructureOnly),
            vocal_intent: Some(VocalIntent::Instrumental),
            suno_lyrics_content_source: Some(source),
            suno_lyrics_field_text: "[AI-or-mixed structure instruction]".into(),
            ..TrackFields::default()
        };

        let human = human_contribution_markdown(&fields);
        let suno_field = suno_field_markdown(&fields);

        assert!(!human.contains("Content source [User-confirmed fact]"));
        assert!(!human.contains("[AI-or-mixed structure instruction]"));
        assert!(suno_field.starts_with("## F. Suno Generation Text Field\n"));
        assert!(suno_field.contains("- Generation Text Field Used [User-confirmed fact]: YES"));
        assert!(suno_field.contains("- Vocal Intent [User-confirmed fact]: INSTRUMENTAL"));
        assert!(suno_field.contains("[AI-or-mixed structure instruction]"));
        assert!(!suno_field.starts_with("## E."));
    }
}

#[test]
fn structure_only_generation_text_does_not_become_vocal_lyrics_or_override_audio_result() {
    let fields = TrackFields {
        instrumental_track: Some(false),
        vocal_lyrics_present: Some(true),
        suno_content_classification: Some(SunoContentClassification::StructureOnly),
        vocal_intent: Some(VocalIntent::Instrumental),
        suno_lyrics_content_source: Some(SunoLyricsContentSource::Human),
        suno_lyrics_field_text: "[Intro]\n[Drop]".into(),
        ..TrackFields::default()
    };

    let markdown = suno_field_markdown(&fields);
    assert!(markdown.contains("- Vocal Lyrics Present [Classification-derived presentation]: NO"));
    assert!(markdown
        .contains("- Structure Instructions Present [Classification-derived presentation]: YES"));
    assert!(markdown.contains("- Vocal Intent [User-confirmed fact]: INSTRUMENTAL"));
    assert!(markdown.contains("- Final Audio Contains Vocals [User-confirmed fact]: YES"));
}
