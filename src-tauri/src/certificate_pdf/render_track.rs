use super::*;

pub(super) fn render_document_title(layout: &mut PdfLayout) {
    layout.write_localized(
        DOCUMENT_TITLE,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        14.0,
        BuiltinFont::HelveticaBold,
        1.28,
    );
    layout.write_localized(
        "Finalized technical snapshot – not a legal certification",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.5,
        BuiltinFont::Helvetica,
        1.2,
    );
    layout.y_mm -= 1.0;
    layout.rule(LEFT_MM, RIGHT_MM, layout.y_mm, 0.6);
    layout.y_mm -= 3.0;
}

pub(super) fn render_header_metadata(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("A. Certificate / Snapshot Identity");
    let rows = vec![
        TableRow::documented_plain("Track", &snapshot.track.fields.title),
        TableRow::documented_plain("Artist", &snapshot.profile.artist_name),
        TableRow::mono("Certificate ID", snapshot.certificate_id),
        TableRow::mono("Finalization timestamp", snapshot.finalized_at),
        TableRow::system_plain("Documentation status", "DOCUMENTATION COMPLETE"),
        TableRow::system_plain("Meaning", "configured documentation requirements completed"),
        TableRow::system_plain(
            "PASS definition",
            "Configured documentation requirements for this step were satisfied.",
        ),
        TableRow::mono("Application version", env!("CARGO_PKG_VERSION")),
        TableRow::mono("Workflow ID", &snapshot.track.workflow_id),
        TableRow::mono("Workflow version", &snapshot.track.workflow_version),
        TableRow::mono("Certificate version", snapshot.certificate_version),
    ];
    layout.table_rows(&rows, 45.0);
}

pub(super) fn render_track_data(layout: &mut PdfLayout, snapshot: &CertificatePdfSnapshot<'_>) {
    layout.section_title("B. Track identity");
    let fields = &snapshot.track.fields;
    let rows = vec![
        TableRow::documented_plain("Documented title [User-confirmed fact]", &fields.title),
        TableRow::documented_plain(
            "Artist [User-confirmed fact]",
            &snapshot.profile.artist_name,
        ),
        TableRow::optional_plain(
            "Actual release filename [Evidence-derived metadata]",
            evidence_original_file_name(snapshot.evidence, EvidenceRole::ReleaseWav),
            "NOT RECORDED",
        ),
        TableRow::optional_plain(
            "Actual Suno export filename [Evidence-derived metadata]",
            evidence_original_file_name(snapshot.evidence, EvidenceRole::SunoFinalExport),
            "NOT RECORDED",
        ),
        TableRow::documented_plain("Production start", &fields.production_start_date),
        TableRow::documented_plain("Production end", &fields.production_end_date),
        TableRow::documented_plain(
            format!(
                "Last editing date [{}]",
                fact_origin_label(snapshot.automation.final_export_origin)
            ),
            &fields.final_export_date,
        ),
        TableRow::system_plain(
            "Commercial use intended",
            yes_no(Some(fields.commercial_use_intended)),
        ),
        TableRow::documented_plain("Suno profile", &snapshot.profile.suno_profile_name),
        TableRow::documented_mono("Suno handle", &snapshot.profile.suno_handle),
    ];
    layout.table_rows(&rows, 45.0);
}

pub(super) fn render_final_suno_generation(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("C. Final Suno Generation");
    let fields = &snapshot.track.fields;
    let rows = vec![
        TableRow::documented_plain(
            format!(
                "Final generation date [{}]",
                fact_origin_label(snapshot.automation.final_generation_origin)
            ),
            &fields.suno_final_generation_date,
        ),
        TableRow::system_plain(
            "Final generation date origin",
            fact_origin_label(snapshot.automation.final_generation_origin),
        ),
        TableRow::documented_mono(
            format!(
                "Final generation ID [{}]",
                fact_origin_label(snapshot.automation.final_generation_id_origin)
            ),
            &fields.suno_final_generation_id,
        ),
        TableRow::documented_mono(
            "Suno project URL [User-confirmed fact]",
            &fields.suno_project_url,
        ),
        TableRow::documented_plain(
            format!(
                "Download/export date [{}]",
                fact_origin_label(snapshot.automation.download_export_origin)
            ),
            &fields.suno_download_export_date,
        ),
        TableRow::system_plain(
            "Download/export date origin",
            fact_origin_label(snapshot.automation.download_export_origin),
        ),
        TableRow::system_plain(
            "Suno Studio metadata detected [System verification]",
            yes_no_bool(snapshot.automation.suno_metadata_detected),
        ),
        TableRow::system_plain(
            "Metadata origin",
            if snapshot.automation.suno_metadata_detected {
                "Evidence-derived metadata"
            } else {
                "NOT DOCUMENTED"
            },
        ),
        TableRow::documented_plain("Suno model [User-confirmed fact]", &fields.suno_model),
        TableRow::documented_plain(
            "Suno plan at generation [User-confirmed fact]",
            &fields.suno_plan_at_generation,
        ),
        TableRow::system_plain(
            "Release identical to Suno final export [System verification]",
            yes_no_bool(snapshot.automation.release_identical_to_suno_export),
        ),
    ];
    // Keep long provenance-bearing labels together in extracted/searchable
    // text; IDs and URLs are allowed to wrap in the value column.
    layout.table_rows(&rows, 106.0);
    let legacy_plan_label = layout.label(
        "Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]",
    );
    let legacy_plan_value = if fields.legacy_suno_plan_at_creation.trim().is_empty() {
        layout.label("NOT DOCUMENTED")
    } else {
        fields.legacy_suno_plan_at_creation.clone()
    };
    layout.write_wrapped(
        &format!("{legacy_plan_label}: {legacy_plan_value}"),
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::HelveticaBold,
        1.3,
    );
}

pub(super) fn render_sources_and_generation(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("D. Source provenance");
    let fields = snapshot.track.fields.normalized_conditionals();
    let mut rows = vec![
        TableRow::system_plain(
            "External audio uploaded [User-confirmed fact]",
            yes_no(fields.external_audio_uploaded),
        ),
        TableRow::system_plain(
            "Own audio uploaded [User-confirmed fact]",
            yes_no(fields.own_audio_uploaded),
        ),
        TableRow::system_plain(
            "Third-party samples [User-confirmed fact]",
            yes_no(fields.third_party_samples_uploaded),
        ),
        TableRow::system_plain(
            "Code-based generation [User-confirmed fact]",
            yes_no(fields.code_based_generation),
        ),
        TableRow::conditional_optional_plain(
            "Source-code evidence [System value]",
            fields.code_based_generation,
            evidence_relative_path(snapshot.evidence, EvidenceRole::SourceCodeFile),
            "NOT RECORDED",
        ),
        TableRow::conditional_optional_plain(
            "Code-generated audio evidence [System value]",
            fields.code_based_generation,
            evidence_relative_path(snapshot.evidence, EvidenceRole::CodeGeneratedAudioFile),
            "NOT RECORDED",
        ),
        TableRow::system_plain(
            "Code-audio post-processing [User-confirmed fact]",
            branch_value(
                fields.code_based_generation,
                yes_no(fields.code_audio_post_processed),
            ),
        ),
        if fields.code_based_generation == Some(true)
            && fields.code_audio_post_processed == Some(true)
        {
            TableRow::documented_list_plain(
                "Code-audio post-processing operations [User-confirmed fact]",
                &fields.code_audio_post_processing_operations,
            )
        } else {
            TableRow::system_plain(
                "Code-audio post-processing operations [User-confirmed fact]",
                if fields.code_based_generation.is_none() {
                    "NOT DOCUMENTED"
                } else {
                    "N/A"
                },
            )
        },
        TableRow::conditional_plain(
            "External audio source [User-confirmed fact]",
            fields.external_audio_uploaded,
            &fields.external_audio_source,
        ),
        TableRow::conditional_plain(
            "External audio provenance statement [User-confirmed fact]",
            fields.external_audio_uploaded,
            &fields.external_audio_ownership,
        ),
        TableRow::conditional_plain(
            "Own audio source [User-confirmed fact]",
            fields.own_audio_uploaded,
            &fields.own_audio_source,
        ),
        TableRow::conditional_plain(
            "Own audio provenance statement [User-confirmed fact]",
            fields.own_audio_uploaded,
            &fields.own_audio_ownership,
        ),
        TableRow::conditional_plain(
            "Third-party sample source [User-confirmed fact]",
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_source,
        ),
        TableRow::conditional_plain(
            "Third-party sample provenance statement [User-confirmed fact]",
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_ownership,
        ),
    ];

    if fields.code_based_generation == Some(true) && fields.code_audio_post_processed == Some(true)
    {
        push_documented_detail(
            &mut rows,
            "Other code-audio post-processing [User-confirmed fact]",
            &fields.code_audio_post_processing_note,
        );
    }

    // Provenance-bearing labels must remain contiguous in extracted text.
    layout.table_rows(&rows, 106.0);
}

pub(super) fn render_human_contribution(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("E. Human contribution");
    let fields = snapshot.track.fields.normalized_conditionals();
    let mut rows = vec![
        TableRow::system_plain(
            "Human contribution documented [User-confirmed fact]",
            yes_no(fields.human_editing_performed),
        ),
        TableRow::conditional_plain(
            "Documented human contribution [User-confirmed fact]",
            fields.human_editing_performed,
            &fields.human_editing_details,
        ),
        TableRow::system_plain(
            "External desktop/post-export editing documented [User-confirmed fact]",
            yes_no(fields.post_export_editing_performed),
        ),
        TableRow::conditional_plain(
            "Documented desktop/post-export editing [User-confirmed fact]",
            fields.post_export_editing_performed,
            &fields.post_export_editing_details,
        ),
    ];
    if fields.artwork_origin == "human" {
        rows.push(TableRow::documented_list_plain(
            "Confirmed human artwork process [User-confirmed fact]",
            &fields.human_artwork_process_operations,
        ));
        rows.push(TableRow::documented_plain(
            "Human artwork process notes [User-confirmed fact]",
            &fields.human_artwork_process_notes,
        ));
    } else if fields.artwork_origin == "ai_assisted" {
        rows.push(TableRow::documented_list_plain(
            "Confirmed human artwork modifications [User-confirmed fact]",
            &fields.human_artwork_modifications,
        ));
        rows.push(TableRow::documented_plain(
            "Other human artwork change [User-confirmed fact]",
            &fields.custom_artwork_change,
        ));
    }
    // Preserve the full provenance label as a contiguous accessibility/search
    // token; the human-artwork labels are longer than the ordinary field names.
    layout.table_rows(&rows, 106.0);
}

pub(super) fn render_suno_field(layout: &mut PdfLayout, snapshot: &CertificatePdfSnapshot<'_>) {
    let fields = snapshot.track.fields.normalized_conditionals();
    let classification = content_classification(&fields);
    layout.section_title("F. Suno Generation Text Field");
    for (label, value) in [
        (
            "Suno Instrumental Mode Selected [User-confirmed fact]",
            yes_no(fields.instrumental_track),
        ),
        (
            "Generation Text Field Used [User-confirmed fact]",
            generation_text_field_used(&fields),
        ),
        (
            "Content Classification [User-confirmed fact]",
            classification,
        ),
        (
            "Vocal Lyrics Present [Classification-derived presentation]",
            generation_field_vocal_lyrics_present(&fields),
        ),
        (
            "Structure Instructions Present [Classification-derived presentation]",
            structure_instructions_present(&fields),
        ),
        (
            "Vocal Intent [User-confirmed fact]",
            suno_vocal_intent(&fields),
        ),
        (
            "Final Audio Contains Vocals [User-confirmed fact]",
            final_audio_contains_vocals(&fields),
        ),
    ] {
        let status_line = format!("{}: {}", layout.label(label), layout.label(value));
        layout.write_wrapped(
            &status_line,
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.2,
            BuiltinFont::HelveticaBold,
            1.3,
        );
    }
    layout.write_localized(
        "Exact Generation Text Field Content",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::HelveticaBold,
        1.3,
    );
    let exact_generation_text = match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => layout.label("N/A"),
        Some(_) if fields.suno_lyrics_field_text.trim().is_empty() => {
            layout.label("NOT DOCUMENTED")
        }
        Some(_) => fields.suno_lyrics_field_text.clone(),
        None => layout.label("NOT DOCUMENTED"),
    };
    layout.write_wrapped(
        &exact_generation_text,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.1,
        BuiltinFont::Courier,
        1.3,
    );
    if !fields.lyrics_source.trim().is_empty() || !fields.lyrics_text.trim().is_empty() {
        layout.section_title("F.1 Unclassified legacy lyrics data");
        let legacy_source = if fields.lyrics_source.trim().is_empty() {
            layout.label("NOT DOCUMENTED")
        } else {
            fields.lyrics_source.clone()
        };
        let legacy_text = if fields.lyrics_text.trim().is_empty() {
            layout.label("NOT DOCUMENTED")
        } else {
            fields.lyrics_text.clone()
        };
        layout.table_rows(
            &[
                TableRow::system_plain("Classification", "NOT DOCUMENTED"),
                TableRow::plain("Legacy source value", legacy_source),
            ],
            48.0,
        );
        let legacy_text_label = layout.label("Legacy text");
        layout.write_wrapped(
            &format!("{legacy_text_label}: {legacy_text}"),
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.1,
            BuiltinFont::Helvetica,
            1.3,
        );
        layout.write_localized(
            "This retained historical data is unclassified legacy data and is not a Vocal Lyrics claim.",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.1,
            BuiltinFont::HelveticaBold,
            1.3,
        );
    }
}

pub(super) fn render_ai_usage(layout: &mut PdfLayout, snapshot: &CertificatePdfSnapshot<'_>) {
    layout.section_title("G.1 AI Transparency Assessment – Audio");
    let fields = snapshot.track.fields.normalized_conditionals();
    let rows = ai_usage_rows(&fields);
    layout.table_rows(&rows, 76.0);
    render_style_prompt(layout, &fields);
}

fn ai_usage_rows(fields: &TrackFields) -> Vec<TableRow> {
    let active = fields.generative_ai_used;
    vec![
        TableRow::system_plain("Generative AI used [User-confirmed fact]", yes_no(active)),
        TableRow::conditional_plain(
            "AI system [User-confirmed fact]",
            active,
            &fields.audio_ai_system,
        ),
        TableRow::system_plain(
            "AI-assisted audio elements [User-confirmed fact]",
            controlled_documentation_answer(active, fields.ai_assisted_audio_elements),
        ),
        TableRow::system_plain(
            "AI-generated audio elements [User-confirmed fact]",
            controlled_documentation_answer(active, fields.ai_generated_audio_elements),
        ),
        TableRow::system_plain(
            "Real person voice intentionally imitated [User-confirmed fact]",
            controlled_documentation_answer(
                active,
                fields.real_person_voice_intentionally_imitated,
            ),
        ),
        TableRow::system_plain(
            "Real person's identity intentionally represented [User-confirmed fact]",
            controlled_documentation_answer(
                active,
                fields.real_person_identity_intentionally_represented,
            ),
        ),
        TableRow::system_plain(
            "Real event represented as authentic recording [User-confirmed fact]",
            controlled_documentation_answer(
                active,
                fields.real_event_represented_as_authentic_recording,
            ),
        ),
        TableRow::system_plain(
            "Real location / institution / event presented as authentic AI recording [User-confirmed fact]",
            controlled_documentation_answer(
                active,
                fields.real_location_institution_event_presented_as_authentic_ai_recording,
            ),
        ),
        TableRow::system_plain(
            "Disclosure applied [User-confirmed fact]",
            controlled_documentation_answer(active, fields.audio_disclosure_applied),
        ),
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => TableRow::documented_list_plain(
                "Disclosure locations [User-confirmed fact]",
                &fields.audio_disclosure_locations,
            ),
            (Some(true), _) | (Some(false), _) => TableRow::system_plain(
                "Disclosure locations [User-confirmed fact]",
                "N/A",
            ),
            (None, _) => TableRow::system_plain(
                "Disclosure locations [User-confirmed fact]",
                "NOT DOCUMENTED",
            ),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => TableRow::documented_plain(
                "Disclosure text [User-confirmed fact]",
                &fields.audio_disclosure_text,
            ),
            (Some(true), _) | (Some(false), _) => {
                TableRow::system_plain("Disclosure text [User-confirmed fact]", "N/A")
            }
            (None, _) => TableRow::system_plain(
                "Disclosure text [User-confirmed fact]",
                "NOT DOCUMENTED",
            ),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::No)) => TableRow::documented_plain(
                "Disclosure reason / note [User-confirmed fact]",
                &fields.audio_disclosure_reason,
            ),
            (Some(true), _) | (Some(false), _) => TableRow::system_plain(
                "Disclosure reason / note [User-confirmed fact]",
                "N/A",
            ),
            (None, _) => TableRow::system_plain(
                "Disclosure reason / note [User-confirmed fact]",
                "NOT DOCUMENTED",
            ),
        },
        TableRow::system_plain(
            "Deepfake-related indicator summary",
            deepfake_indicator_summary(fields),
        ),
    ]
}

fn render_style_prompt(layout: &mut PdfLayout, fields: &TrackFields) {
    layout.write_localized(
        "Documented Suno style prompt",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.4,
        BuiltinFont::HelveticaBold,
        1.3,
    );
    layout.write_localized(
        "Documented input – this field records the submitted prompt and does not describe or verify the resulting audio content.",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.0,
        BuiltinFont::Helvetica,
        1.3,
    );
    let style_prompt = if fields.suno_style_prompt.trim().is_empty() {
        layout.label("NOT DOCUMENTED")
    } else {
        fields.suno_style_prompt.clone()
    };
    layout.write_wrapped(
        &style_prompt,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        6.8,
        BuiltinFont::Courier,
        1.35,
    );
    layout.write_localized(
        "No AI Act compliance, legal necessity, legal safety, or other legal determination is made.",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::Helvetica,
        1.3,
    );
}

pub(super) fn render_artwork_checks(layout: &mut PdfLayout, snapshot: &CertificatePdfSnapshot<'_>) {
    layout.section_title("G.2 AI Transparency Assessment – Artwork");
    let fields = &snapshot.track.fields;
    let ai_artwork = matches!(
        fields.artwork_origin.as_str(),
        "ai_generated" | "ai_assisted"
    );
    let artwork_evidence = snapshot
        .evidence
        .iter()
        .filter(|item| {
            matches!(
                item.role,
                EvidenceRole::HumanEditedArtwork | EvidenceRole::FinalArtwork
            )
        })
        .map(|item| (*item).clone())
        .collect::<Vec<_>>();
    let artwork_comparison = workflow::human_edited_final_artwork_status(&artwork_evidence);
    let mut rows = artwork_factual_rows(fields, ai_artwork);
    rows.extend(artwork_disclosure_rows(
        fields,
        ai_artwork,
        artwork_comparison,
    ));
    // The explicit disclosure and comparison labels are part of the evidence
    // contract, so preserve them as contiguous searchable tokens.
    layout.table_rows(&rows, 112.0);
    layout.write_localized(
        crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::Helvetica,
        1.3,
    );
}

fn artwork_factual_rows(fields: &TrackFields, ai_artwork: bool) -> Vec<TableRow> {
    vec![
        TableRow::documented_plain(
            "Artwork origin [User-confirmed fact]",
            &fields.artwork_origin,
        ),
        if ai_artwork {
            TableRow::documented_plain(
                "AI image service [User-confirmed fact]",
                &fields.ai_image_service,
            )
        } else {
            TableRow::system_plain(
                "AI image service [User-confirmed fact]",
                if fields.artwork_origin.trim().is_empty() {
                    "NOT DOCUMENTED"
                } else {
                    "N/A"
                },
            )
        },
        if fields.artwork_origin == "human" {
            TableRow::documented_list_plain(
                "Human artwork process [User-confirmed fact]",
                &fields.human_artwork_process_operations,
            )
        } else if fields.artwork_origin == "ai_assisted" {
            TableRow::system_plain(
                "Human artwork process [User-confirmed fact]",
                if fields
                    .human_artwork_modifications
                    .iter()
                    .any(|value| !value.trim().is_empty())
                    || !fields.custom_artwork_change.trim().is_empty()
                {
                    "YES"
                } else {
                    "NOT DOCUMENTED"
                },
            )
        } else {
            TableRow::system_plain(
                "Human artwork process [User-confirmed fact]",
                if fields.artwork_origin.trim().is_empty() {
                    "NOT DOCUMENTED"
                } else {
                    "N/A"
                },
            )
        },
        if fields.artwork_origin == "ai_assisted" {
            TableRow::documented_list_plain(
                "Human artwork changes [User-confirmed fact]",
                &fields.human_artwork_modifications,
            )
        } else {
            TableRow::system_plain(
                "Human artwork changes [User-confirmed fact]",
                if fields.artwork_origin.trim().is_empty() {
                    "NOT DOCUMENTED"
                } else {
                    "N/A"
                },
            )
        },
        TableRow::system_plain(
            "Depicts real person [User-confirmed fact]",
            artwork_answer(&fields.artwork_origin, fields.depicts_real_person),
        ),
        artwork_note_table_row(
            "Real-person note [User-confirmed fact]",
            &fields.artwork_origin,
            fields.depicts_real_person,
            &fields.real_person_notes,
        ),
        TableRow::system_plain(
            "Depicts real event [User-confirmed fact]",
            artwork_answer(&fields.artwork_origin, fields.depicts_real_event),
        ),
        artwork_note_table_row(
            "Real-event note [User-confirmed fact]",
            &fields.artwork_origin,
            fields.depicts_real_event,
            &fields.real_event_notes,
        ),
        TableRow::system_plain(
            "Trademark/logo [User-confirmed fact]",
            artwork_answer(&fields.artwork_origin, fields.contains_trademark),
        ),
        artwork_note_table_row(
            "Trademark/logo note [User-confirmed fact]",
            &fields.artwork_origin,
            fields.contains_trademark,
            &fields.trademark_notes,
        ),
    ]
}

fn artwork_disclosure_rows(
    fields: &TrackFields,
    ai_artwork: bool,
    artwork_comparison: &str,
) -> Vec<TableRow> {
    vec![
        TableRow::system_plain(
            "Artwork disclosure applied [User-confirmed fact]",
            if ai_artwork {
                yes_no(fields.disclosure_applied)
            } else if fields.artwork_origin.trim().is_empty() {
                "NOT DOCUMENTED"
            } else {
                "N/A"
            },
        ),
        TableRow::system_plain(
            "Artwork disclosure deliberately not applied [User-confirmed fact]",
            if ai_artwork {
                match fields.disclosure_applied {
                    Some(false) => "YES",
                    Some(true) => "NO",
                    None => "NOT DOCUMENTED",
                }
            } else if fields.artwork_origin.trim().is_empty() {
                "NOT DOCUMENTED"
            } else {
                "N/A"
            },
        ),
        if ai_artwork && fields.disclosure_applied == Some(true) {
            TableRow::documented_plain(
                "Artwork disclosure text [User-confirmed fact]",
                &fields.disclosure_text,
            )
        } else {
            TableRow::system_plain(
                "Artwork disclosure text [User-confirmed fact]",
                if ai_artwork && fields.disclosure_applied.is_none() {
                    "NOT DOCUMENTED"
                } else {
                    "N/A"
                },
            )
        },
        TableRow::system_plain(
            "Human-edited/final artwork comparison [System verification]",
            artwork_comparison,
        ),
    ]
}
