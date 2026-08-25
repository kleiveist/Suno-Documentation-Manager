use super::*;

#[test]
fn identical_snapshot_produces_identical_pdf_bytes() {
    let fixture = Fixture::new(4);
    assert_eq!(fixture.generate(), fixture.generate());
}

#[test]
fn renders_vocal_track_from_explicit_new_fields() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.instrumental_track = Some(false);
    fixture.track.fields.vocal_lyrics_present = Some(true);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Vocal);
    fixture.track.fields.suno_content_classification =
        Some(SunoContentClassification::VocalLyricsOnly);
    fixture.track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
    fixture.track.fields.suno_lyrics_field_text = "Sing this exact line".into();
    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("F. Suno Generation Text Field"));
    assert!(text.contains("Suno Instrumental Mode Selected [User-confirmed fact]: NO"));
    assert!(text.contains("Vocal Lyrics Present [Classification-derived presentation]: YES"));
    assert!(text.contains("Final Audio Contains Vocals [User-confirmed fact]: YES"));
    assert!(text.contains("Sing this exact line"));
}

#[test]
fn mixed_content_and_unspecified_vocal_intent_remain_independent_from_audio() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.vocal_lyrics_present = Some(false);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Unspecified);
    fixture.track.fields.suno_content_classification = Some(SunoContentClassification::Mixed);
    fixture.track.fields.suno_lyrics_field_text = "[Intro]\nSing this line".into();

    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("Content Classification [User-confirmed fact]: MIXED"));
    assert!(text.contains("Vocal Lyrics Present [Classification-derived presentation]: YES"));
    assert!(
        text.contains("Structure Instructions Present [Classification-derived presentation]: YES")
    );
    assert!(text.contains("Vocal Intent [User-confirmed fact]: UNSPECIFIED"));
    assert!(text.contains("Final Audio Contains Vocals [User-confirmed fact]: NO"));
}

#[test]
fn legacy_lyrics_remain_unclassified_without_vocal_claim() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.instrumental_track = None;
    fixture.track.fields.vocal_lyrics_present = None;
    fixture.track.fields.vocal_intent = None;
    fixture.track.fields.suno_content_classification = None;
    fixture.track.fields.suno_lyrics_content_source = None;
    fixture.track.fields.suno_lyrics_field_text.clear();
    fixture.track.fields.lyrics_source = "legacy-mixed".into();
    fixture.track.fields.lyrics_text = "[Intro] historical value".into();
    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("Unclassified legacy lyrics data"));
    assert!(text.contains("not a Vocal Lyrics claim"));
    assert!(!text.contains("F. Suno Lyrics / Structure Field – Vocal Lyrics"));
}

#[test]
fn renders_plan_at_generation_and_technical_subscription_coverage() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_final_generation_date = "2026-08-15".into();
    fixture.track.fields.suno_plan_at_generation = "Premier".into();
    let mut subscription = evidence_item(21);
    subscription.role = EvidenceRole::SubscriptionPayment;
    subscription.relative_path = "04_LICENSES/subscription.pdf".into();
    subscription.coverage_start = Some("2026-08-14".into());
    subscription.coverage_end = Some("2026-09-13".into());
    fixture.evidence.push(subscription);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("Suno plan at generation [User-confirmed fact]\nPremier"));
    assert!(text.contains("Final-generation date covered [System verification]\nYES"));
    assert!(!text.contains("Commercial rights confirmed"));
}

#[test]
fn legacy_plan_value_does_not_become_plan_at_generation() {
    let historical: TrackFields = serde_json::from_value(serde_json::json!({
        "sunoPlanAtCreation": "Pro"
    }))
    .expect("legacy plan fixture");
    let mut fixture = Fixture::new(1);
    fixture.track.fields.suno_plan_at_generation = historical.suno_plan_at_generation;
    fixture.track.fields.legacy_suno_plan_at_creation = historical.legacy_suno_plan_at_creation;

    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    assert!(normalized.contains("Suno plan at generation [User-confirmed fact] NOT DOCUMENTED"));
    assert!(normalized.contains(
        "Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: Pro"
    ));
    assert!(!normalized.contains("Suno plan at generation [User-confirmed fact] Pro"));
}

#[test]
fn labels_source_paths_as_system_values_and_human_details_as_user_facts() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.external_audio_uploaded = Some(true);
    fixture.track.fields.external_audio_source = "External recorder".into();
    fixture.track.fields.external_audio_ownership = "User-owned recording".into();
    fixture.track.fields.own_audio_uploaded = Some(true);
    fixture.track.fields.own_audio_source = "Own stem".into();
    fixture.track.fields.own_audio_ownership = "Created by user".into();
    fixture.track.fields.third_party_samples_uploaded = Some(true);
    fixture.track.fields.third_party_sample_source = "Sample archive".into();
    fixture.track.fields.third_party_sample_ownership = "Licensed sample".into();
    fixture.track.fields.code_based_generation = Some(true);
    fixture.track.fields.code_audio_post_processed = Some(true);
    fixture.track.fields.code_audio_post_processing_operations =
        vec!["Normalize".into(), "Other post-processing".into()];
    fixture.track.fields.code_audio_post_processing_note = "Manual limiter".into();
    fixture.track.fields.human_editing_performed = Some(true);
    fixture.track.fields.human_editing_details = "Manual timing edit".into();
    fixture.track.fields.post_export_editing_performed = Some(true);
    fixture.track.fields.post_export_editing_details = "Manual mastering".into();
    fixture.track.fields.artwork_origin = "ai_assisted".into();
    fixture.track.fields.human_artwork_modifications =
        vec!["Crop".into(), "Other human editing".into()];
    fixture.track.fields.custom_artwork_change = "Manual typography".into();
    let mut source = evidence_item(50);
    source.role = EvidenceRole::SourceCodeFile;
    source.relative_path = "03_DOCUMENTATION/source-code.rs".into();
    let mut generated = evidence_item(51);
    generated.role = EvidenceRole::CodeGeneratedAudioFile;
    generated.relative_path = "03_DOCUMENTATION/code-audio.wav".into();
    fixture.evidence.extend([source, generated]);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let (_, text) = parse_text(&fixture.generate());
    let normalized = normalized_text(&text);
    for expected in [
        "External audio uploaded [User-confirmed fact]",
        "External audio source [User-confirmed fact]",
        "External audio provenance statement [User-confirmed fact]",
        "Own audio source [User-confirmed fact]",
        "Own audio provenance statement [User-confirmed fact]",
        "Third-party sample source [User-confirmed fact]",
        "Third-party sample provenance statement [User-confirmed fact]",
        "Source-code evidence [System value]",
        "Code-generated audio evidence [System value]",
        "Code-audio post-processing [User-confirmed fact]",
        "Code-audio post-processing operations [User-confirmed fact]",
        "Other code-audio post-processing [User-confirmed fact]",
        "Documented human contribution [User-confirmed fact]",
        "Documented desktop/post-export editing [User-confirmed fact]",
        "Confirmed human artwork modifications [User-confirmed fact]",
        "Other human artwork change [User-confirmed fact]",
    ] {
        assert!(
            normalized.contains(expected),
            "missing D/E provenance label: {expected}"
        );
    }
    assert!(normalized.contains("03_DOCUMENTATION/source-code.rs"));
    assert!(normalized.contains("03_DOCUMENTATION/code-audio.wav"));
    assert!(normalized.contains("Manual timing edit"));
    assert!(normalized.contains("Manual typography"));
}

#[test]
fn artwork_pdf_records_explicit_no_hash_match_and_import_timestamp_limit() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.artwork_origin = "ai_assisted".into();
    fixture.track.fields.ai_image_service = "Documented image service".into();
    fixture.track.fields.disclosure_applied = Some(false);
    let mut human = evidence_item(90);
    human.role = EvidenceRole::HumanEditedArtwork;
    human.sha256 = Some(DIGEST_A.into());
    let mut final_artwork = evidence_item(91);
    final_artwork.role = EvidenceRole::FinalArtwork;
    final_artwork.sha256 = Some(DIGEST_A.into());
    fixture.evidence.extend([human, final_artwork]);
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let (_, english) = parse_text(&fixture.generate());
    let english_normalized = normalized_text(&english);
    assert!(english_normalized.contains("Artwork disclosure applied [User-confirmed fact] NO"));
    assert!(english_normalized
        .contains("Artwork disclosure deliberately not applied [User-confirmed fact] YES"));
    assert!(english_normalized.contains("BYTE-IDENTICAL / SHA-256 MATCH"));
    assert!(english_normalized.contains(crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE));

    fixture.render_options = CertificateRenderOptions {
        language: CertificateLanguage::De,
        bilingual: false,
    };
    let (_, german) = parse_text(&fixture.generate());
    assert!(german.contains("Artwork-Hinweis bewusst nicht angewendet"));
    assert!(german.contains("Import-Zeitstempel dokumentieren nur den Import in SunoDM"));
}

#[test]
fn renders_complete_audio_transparency_without_legal_conclusion() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.generative_ai_used = Some(true);
    fixture.track.fields.audio_ai_system = "Suno".into();
    fixture.track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::No);
    fixture.track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::Yes);
    fixture
        .track
        .fields
        .real_person_voice_intentionally_imitated = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_person_identity_intentionally_represented = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_event_represented_as_authentic_recording = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_location_institution_event_presented_as_authentic_ai_recording =
        Some(DocumentationAnswer::No);
    fixture.track.fields.audio_disclosure_applied = Some(DocumentationAnswer::Yes);
    fixture.track.fields.audio_disclosure_locations = vec!["release metadata".into()];
    fixture.track.fields.audio_disclosure_text = "AI-assisted audio".into();
    if let Some(step) = fixture
        .steps
        .iter_mut()
        .find(|step| step.id == "ai_transparency")
    {
        step.status = StepStatus::Pass;
        step.na_reason = None;
    }
    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("AI Transparency Assessment – Audio"));
    assert!(text.contains("Documented deepfake-related indicators: none recorded"));
    assert!(text.contains("Disclosure applied [User-confirmed fact]\nYES"));
    assert!(!text.contains("AI Act compliant"));
    assert!(!text.contains("No deepfake"));
}

#[test]
fn renders_conscious_no_disclosure_with_reason() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.generative_ai_used = Some(true);
    fixture.track.fields.audio_ai_system = "Suno".into();
    fixture.track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::No);
    fixture.track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::Yes);
    fixture
        .track
        .fields
        .real_person_voice_intentionally_imitated = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_person_identity_intentionally_represented = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_event_represented_as_authentic_recording = Some(DocumentationAnswer::No);
    fixture
        .track
        .fields
        .real_location_institution_event_presented_as_authentic_ai_recording =
        Some(DocumentationAnswer::No);
    fixture.track.fields.audio_disclosure_applied = Some(DocumentationAnswer::No);
    fixture.track.fields.audio_disclosure_reason = "User documented a factual reason".into();
    if let Some(step) = fixture
        .steps
        .iter_mut()
        .find(|step| step.id == "ai_transparency")
    {
        step.status = StepStatus::Pass;
        step.na_reason = None;
    }
    let (_, text) = parse_text(&fixture.generate());
    assert!(text.contains("Disclosure applied [User-confirmed fact]\nNO"));
    assert!(text.contains("User documented a factual reason"));
    assert!(!text.contains("Disclosure legally unnecessary"));
}

#[test]
fn preserves_no_na_and_not_documented_as_distinct_states() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.external_audio_uploaded = Some(false);
    fixture.track.fields.code_based_generation = Some(false);
    fixture.track.fields.generative_ai_used = Some(true);
    fixture.track.fields.audio_ai_system = "Suno".into();
    fixture.track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::No);
    fixture.track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::NotDocumented);
    let bytes = fixture.generate();
    write_review_pdf("case-d-not-documented-na.pdf", &bytes);
    let (_, text) = parse_text(&bytes);
    assert!(text.contains("External audio uploaded [User-confirmed fact]\nNO"));
    assert!(text.contains("Source-code evidence [System value]\nN/A"));
    assert!(text.contains("AI-generated audio elements [User-confirmed fact]\nNOT DOCUMENTED"));
}

#[test]
fn renders_multiple_evidence_items_and_optional_manifest_fields() {
    let mut fixture = Fixture::new(4);
    fixture
        .revision_references
        .push(".archive/revisions/revision-fixture".into());
    let bytes = fixture.generate();
    let (_, text) = parse_text(&bytes);
    assert!(text.contains("02_SUNO/evidence-0000.dat"));
    assert!(text.contains("02_SUNO/evidence-0001.dat"));
    assert!(text.contains("02_SUNO/evidence-0002.dat"));
    assert!(text.contains("release_wav"));
    assert!(text.contains("source_code_file"));
    assert!(text.contains("global-0000"));
    assert!(text.contains("generator-1.0"));
    assert!(text.contains(".archive/revisions/revision-fixture"));
}

#[test]
fn large_evidence_register_and_long_timestamp_addendum_paginate_without_truncation() {
    let bytes = Fixture::new(80).generate();
    let (document, text) = parse_text(&bytes);
    assert!(document.pages.len() > 1);
    assert!(text.contains("02_SUNO/evidence-0079.dat"));
    assert!(text.contains(&format!("Page {} / {}", 1, document.pages.len())));
    assert!(text.contains(&format!(
        "Page {} / {}",
        document.pages.len(),
        document.pages.len()
    )));

    for (index, page) in document.extract_text().into_iter().enumerate() {
        let page_text = page.join("\n");
        assert!(page_text.contains(CERTIFICATE_ID));
        if index > 0 {
            assert!(
                page_text.contains("Technical Evidence Certificate"),
                "missing document header on evidence-register continuation page {}",
                index + 1
            );
        }
        assert!(page_text.contains(&format!("Page {} / {}", index + 1, document.pages.len())));
    }

    // Timestamp evidence is intentionally a separate phase-two addendum.
    // Exercise it alongside the large phase-one certificate so TEST 17
    // covers both PDFs without introducing a cyclic certificate hash.
    let long_timestamp_url = format!(
        "https://timestamp.example.invalid/verify/{}/record?proof={}",
        "nested-segment/".repeat(20),
        "abcdef0123456789".repeat(10)
    );
    let long_external_reference = format!(
        "urn:example:timestamp:{}",
        "123e4567-e89b-12d3-a456-426614174000-".repeat(12)
    );
    let long_timestamp_note =
        "Long factual timestamp note retained without truncation. ".repeat(80);
    let timestamp_snapshot = ExternalTimestampPdfSnapshot {
        certificate_id: CERTIFICATE_ID,
        provider: "Example External Timestamp Provider",
        timestamp_type: "External integrity timestamp",
        timestamp_value: "2026-08-17T12:34:56Z",
        referenced_artifact: "EVIDENCE_MANIFEST.json",
        referenced_artifact_path: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referenced_sha256: DIGEST_A,
        actual_sha256: DIGEST_A,
        referenced_hash_match: Some(true),
        evidence_file_name: "timestamp-evidence-123e4567-e89b-12d3-a456-426614174000.tsr",
        evidence_sha256: DIGEST_C,
        imported_at: "2026-08-17T12:35:00Z",
        provenance: "managed_copy",
        external_reference_id: &long_external_reference,
        provider_verification_url: &long_timestamp_url,
        note: &long_timestamp_note,
        provider_metadata: None,
    };
    let timestamp_bytes = generate_external_timestamp_addendum_pdf(&timestamp_snapshot)
        .expect("long timestamp addendum");
    write_review_pdf("external-timestamp-long-values.pdf", &timestamp_bytes);
    assert_eq!(
        timestamp_bytes,
        generate_external_timestamp_addendum_pdf(&timestamp_snapshot)
            .expect("deterministic long timestamp addendum")
    );
    let (timestamp_document, timestamp_text) = parse_text(&timestamp_bytes);
    assert!(timestamp_document.pages.len() > 1);
    let timestamp_compact = timestamp_text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    for expected in [
        long_timestamp_url.as_str(),
        long_external_reference.as_str(),
        DIGEST_A,
        DIGEST_C,
        "timestamp-evidence-123e4567-e89b-12d3-a456-426614174000.tsr",
    ] {
        let expected_compact = expected
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            timestamp_compact.contains(&expected_compact),
            "truncated long timestamp value: {expected}"
        );
    }
    assert!(normalized_text(&timestamp_text)
        .contains("Long factual timestamp note retained without truncation."));
    for (index, page) in timestamp_document.extract_text().into_iter().enumerate() {
        let page_text = page.join("\n");
        assert!(page_text.contains(CERTIFICATE_ID));
        if index > 0 {
            assert!(
                page_text.contains("Technical Evidence Certificate"),
                "missing document header on timestamp continuation page {}",
                index + 1
            );
        }
        assert!(page_text.contains(&format!(
            "Page {} / {}",
            index + 1,
            timestamp_document.pages.len()
        )));
    }
}

#[test]
fn long_evidence_paths_wrap_without_truncation() {
    let mut fixture = Fixture::new(1);
    let long_path = format!(
        "02_SUNO/{components}evidence.wav",
        components = "nested_component_".repeat(25)
    );
    fixture.evidence[0].relative_path = long_path.clone();
    let bytes = fixture.generate();
    let (_, text) = parse_text(&bytes);
    let compact_text = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    assert!(compact_text.contains(&long_path));
}

#[test]
fn combined_long_value_regression_remains_complete_and_paginated() {
    let mut fixture = Fixture::new(12);
    fixture.track.fields.instrumental_track = Some(true);
    fixture.track.fields.vocal_lyrics_present = Some(false);
    fixture.track.fields.vocal_intent = Some(VocalIntent::Instrumental);
    fixture.track.fields.suno_content_classification =
        Some(SunoContentClassification::StructureOnly);
    fixture.track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Mixed);
    fixture.track.fields.suno_lyrics_field_text =
        "[Intro]\n[sidechained synth pad]\n[white noise riser]\n[Drop]\n".repeat(45);
    fixture.track.fields.suno_final_generation_id = "123e4567-e89b-12d3-a456-426614174000".into();
    let long_url = format!(
        "https://terms.example.invalid/archive/{}/document?revision={}",
        "long-path-component/".repeat(18),
        "abcdef0123456789".repeat(8)
    );
    let mut terms = evidence_item(200);
    terms.role = EvidenceRole::SunoTermsRights;
    terms.relative_path = "04_LICENSES/terms-regression.pdf".into();
    terms.metadata.document_title = "Archived Terms Regression Record".into();
    terms.metadata.provider = "Example Provider".into();
    terms.metadata.source_url = long_url.clone();
    terms.metadata.retrieval_date = "2026-01-03".into();
    terms.metadata.applicable_production_period = "2026-01-01 to 2026-12-31".into();
    fixture.evidence.push(terms);
    for index in 0..4 {
        let mut subscription = evidence_item(210 + index);
        subscription.role = EvidenceRole::SubscriptionPayment;
        subscription.relative_path = format!("04_LICENSES/subscription-{index}.pdf");
        subscription.coverage_start = Some("2026-01-01".into());
        subscription.coverage_end = Some("2026-12-31".into());
        fixture.evidence.push(subscription);
    }
    fixture
        .evidence
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    fixture.revision_references = (0..12)
        .map(|index| {
            format!(".archive/revisions/123e4567-e89b-12d3-a456-{index:012}/revision.json")
        })
        .collect();

    let bytes = fixture.generate();
    write_review_pdf("case-e-long-values.pdf", &bytes);
    let (document, text) = parse_text(&bytes);
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    assert!(document.pages.len() > 3);
    assert!(compact.contains(&long_url));
    assert!(compact.contains("123e4567-e89b-12d3-a456-426614174000"));
    assert!(compact.contains(DIGEST_A));
    assert!(text.contains("Previous revision count [System verification]: 12"));
    assert!(text.contains("[sidechained synth pad]"));
    assert!(text.contains(&format!(
        "Page {} / {}",
        document.pages.len(),
        document.pages.len()
    )));
    for (index, page) in document.extract_text().into_iter().enumerate() {
        let page_text = page.join("\n");
        assert!(page_text.contains(CERTIFICATE_ID));
        if index > 0 {
            assert!(
                page_text.contains("Technical Evidence Certificate"),
                "missing document header on continuation page {}",
                index + 1
            );
        }
        assert!(page_text.contains(&format!("Page {} / {}", index + 1, document.pages.len())));
    }
}

#[test]
fn conditional_details_are_normalized_before_rendering() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.external_audio_uploaded = Some(false);
    fixture.track.fields.external_audio_source = "stale hidden source".into();
    fixture.track.fields.external_audio_ownership = "stale hidden statement".into();
    let bytes = fixture.generate();
    let (_, text) = parse_text(&bytes);
    assert!(!text.contains("stale hidden source"));
    assert!(!text.contains("stale hidden statement"));
}

#[test]
fn rejects_non_win_ansi_snapshot_text_instead_of_replacing_it() {
    let mut fixture = Fixture::new(1);
    fixture.track.fields.title = "Tokyo 東京".into();
    let error = fixture
        .generate_result()
        .expect_err("unsupported built-in-font character");
    assert!(error.to_string().contains("U+6771"));
}

#[test]
fn rejects_unsupported_provider_glyphs_instead_of_rendering_notdef() {
    let mut fixture = Fixture::new(1);
    fixture.track.audio_screening.external.status = AudioScreeningStatus::MatchDetected;
    fixture
        .track
        .audio_screening
        .external
        .matches
        .push(crate::model::AudioScreeningMatch {
            title: "Tokyo 東京".into(),
            artists: vec!["Artist".into()],
            ..Default::default()
        });

    let error = fixture
        .generate_result()
        .expect_err("unsupported provider glyph");
    assert!(error.to_string().contains("provider match 1 title"));
    assert!(error.to_string().contains("U+6771"));
}

#[test]
fn rejects_incomplete_statuses_and_open_blocking_deviations() {
    let mut fixture = Fixture::new(1);
    fixture.steps[0].status = StepStatus::Fail;
    assert!(fixture.generate_result().is_err());

    fixture.steps[0].status = StepStatus::Pass;
    fixture.deviations.push(BlockingDeviation {
        id: "deviation-1".into(),
        title: "Open blocker".into(),
        description: "Unresolved technical deviation".into(),
        blocking: true,
        resolved: false,
        created_at: "2026-01-04T00:00:00Z".into(),
        resolved_at: None,
    });
    assert!(fixture.generate_result().is_err());
}

#[test]
fn wrapping_uses_builtin_font_metrics_for_wide_glyphs() {
    let width_mm = 24.0;
    let lines = wrap_text(&"W".repeat(100), width_mm, 8.0, BuiltinFont::HelveticaBold);
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|line| {
        measure_text_width_mm(line, 8.0, BuiltinFont::HelveticaBold) <= width_mm + 0.001
    }));
    assert_eq!(lines.concat(), "W".repeat(100));
}

#[test]
fn wrapping_does_not_split_sha_256_label_at_its_hyphen() {
    let lines = wrap_text(
        "Referenced SHA-256 value",
        30.0,
        8.0,
        BuiltinFont::HelveticaBold,
    );
    assert!(!lines.iter().any(|line| line.ends_with("SHA-")));
    assert_eq!(lines.concat(), "Referenced SHA-256 value");
}

#[test]
fn rejects_non_pdf_staged_bytes() {
    assert!(validate_pdf_bytes(b"not a PDF").is_err());
}

#[test]
fn rejects_unsorted_or_unverified_evidence_without_reordering_or_filtering() {
    let mut fixture = Fixture::new(2);
    fixture.evidence.swap(0, 1);
    {
        let evidence_refs = fixture.evidence.iter().collect::<Vec<_>>();
        let automation = workflow::automation_summary(&fixture.track, &fixture.evidence);
        let snapshot = CertificatePdfSnapshot {
            track: &fixture.track,
            automation: &automation,
            profile: &fixture.profile,
            steps: &fixture.steps,
            evidence: &evidence_refs,
            deviations: &fixture.deviations,
            revision_references: &fixture.revision_references,
            certificate_id: CERTIFICATE_ID,
            finalized_at: "2026-01-04T12:34:56Z",
            certificate_version: "1",
            sha256sums_sha256: DIGEST_A,
            evidence_manifest_sha256: DIGEST_B,
            markdown_certificate_sha256: DIGEST_C,
            finalization_timestamp: FinalizationTimestampSnapshot::default(),
            artwork_previews: &[],
            render_options: CertificateRenderOptions::default(),
        };
        assert!(generate_pdf(&snapshot).is_err());
    }

    fixture
        .evidence
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    fixture.evidence[0].verified = false;
    let evidence_refs = fixture.evidence.iter().collect::<Vec<_>>();
    let automation = workflow::automation_summary(&fixture.track, &fixture.evidence);
    let snapshot = CertificatePdfSnapshot {
        track: &fixture.track,
        automation: &automation,
        profile: &fixture.profile,
        steps: &fixture.steps,
        evidence: &evidence_refs,
        deviations: &fixture.deviations,
        revision_references: &fixture.revision_references,
        certificate_id: CERTIFICATE_ID,
        finalized_at: "2026-01-04T12:34:56Z",
        certificate_version: "1",
        sha256sums_sha256: DIGEST_A,
        evidence_manifest_sha256: DIGEST_B,
        markdown_certificate_sha256: DIGEST_C,
        finalization_timestamp: FinalizationTimestampSnapshot::default(),
        artwork_previews: &[],
        render_options: CertificateRenderOptions::default(),
    };
    assert!(generate_pdf(&snapshot).is_err());
}
