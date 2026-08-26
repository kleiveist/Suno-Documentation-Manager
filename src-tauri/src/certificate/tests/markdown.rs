use super::*;

#[test]
fn terms_markdown_labels_each_context_value_with_its_origin() {
    let mut terms = relationship_evidence("terms-origin-labels", EvidenceRole::SunoTermsRights);
    terms.metadata.document_title = "Archived Terms".into();
    terms.metadata.provider = "Suno".into();
    terms.metadata.source_url = "https://suno.example/terms".into();
    terms.metadata.retrieval_date = "2026-08-17".into();
    terms.metadata.effective_date = "2026-08-01".into();
    terms.metadata.applicable_production_period = "2026-08-01 to 2026-08-31".into();
    terms.metadata.factual_note = "User-recorded archive context".into();
    terms.metadata.original_file_name = "Suno Terms.pdf".into();

    let markdown = terms_evidence_markdown(&[&terms]);
    for expected in [
        "Evidence ID [System value]",
        "Document title [User-confirmed fact]",
        "Provider/source [User-confirmed fact]",
        "Source URL [User-confirmed fact]",
        "Retrieval date [User-confirmed fact]",
        "Effective date [User-confirmed fact]",
        "Applicable production period [User-confirmed fact]",
        "Factual note [User-confirmed fact]",
        "Relative path [System value]",
        "Original filename [Evidence-derived metadata]",
        "SHA-256 [System verification]",
        "Imported at [System value]",
        "Provenance [System value]",
    ] {
        assert!(
            markdown.contains(expected),
            "missing Terms label: {expected}"
        );
    }
    assert!(markdown.contains("Archived Terms"));
    assert!(markdown.contains("Suno Terms.pdf"));
    assert!(markdown.contains(DIGEST));
}

#[test]
fn historical_plan_value_is_not_rendered_as_plan_at_generation() {
    let fields: TrackFields = serde_json::from_value(serde_json::json!({
        "sunoPlanAtCreation": "Pro"
    }))
    .expect("legacy plan fixture");

    let markdown = materialize_markdown_values(&suno_plan_context_markdown(&fields));
    assert!(markdown.contains("Suno plan at generation [User-confirmed fact]: NOT DOCUMENTED"));
    assert!(markdown.contains(
        "Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: Pro"
    ));
    assert!(!markdown.contains("Suno plan at generation [User-confirmed fact]: Pro"));
}

#[test]
fn source_and_human_detail_markdown_labels_user_facts_and_system_paths() {
    let mut fields = TrackFields {
        external_audio_uploaded: Some(true),
        external_audio_source: "External recorder".into(),
        external_audio_ownership: "User-owned recording".into(),
        own_audio_uploaded: Some(true),
        own_audio_source: "Own stem".into(),
        own_audio_ownership: "Created by user".into(),
        third_party_samples_uploaded: Some(true),
        third_party_sample_source: "Sample archive".into(),
        third_party_sample_ownership: "Licensed sample".into(),
        code_based_generation: Some(true),
        code_audio_post_processed: Some(true),
        code_audio_post_processing_operations: vec!["Normalize".into()],
        code_audio_post_processing_note: "Manual limiter".into(),
        human_editing_performed: Some(true),
        human_editing_details: "Manual timing edit".into(),
        post_export_editing_performed: Some(true),
        post_export_editing_details: "Manual mastering".into(),
        artwork_origin: "ai_assisted".into(),
        human_artwork_modifications: vec!["Crop".into()],
        custom_artwork_change: "Manual typography".into(),
        ..TrackFields::default()
    };

    let source = relationship_evidence("source-code", EvidenceRole::SourceCodeFile);
    let generated = relationship_evidence("code-audio", EvidenceRole::CodeGeneratedAudioFile);
    let source_markdown = source_provenance_markdown(&fields, &[&source, &generated]);
    for expected in [
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
        "Other code-audio post-processing note [User-confirmed fact]",
    ] {
        assert!(
            source_markdown.contains(expected),
            "missing source provenance label: {expected}"
        );
    }
    assert!(source_markdown.contains("03_DOCUMENTATION/source-code.dat"));
    assert!(source_markdown.contains("03_DOCUMENTATION/code-audio.dat"));

    let assisted_artwork = human_contribution_markdown(&fields);
    for expected in [
        "Confirmed human editing [User-confirmed fact]",
        "Confirmed desktop-PC editing [User-confirmed fact]",
        "Confirmed human artwork modifications [User-confirmed fact]",
        "Other human artwork change [User-confirmed fact]",
    ] {
        assert!(
            assisted_artwork.contains(expected),
            "missing human contribution label: {expected}"
        );
    }

    fields.artwork_origin = "human".into();
    fields.human_artwork_process_operations = vec!["Paint".into()];
    fields.human_artwork_process_notes = "Hand-painted cover".into();
    let human_artwork = human_contribution_markdown(&fields);
    assert!(human_artwork.contains("Confirmed human artwork process [User-confirmed fact]"));
    assert!(human_artwork.contains("Human artwork process notes [User-confirmed fact]"));
}
