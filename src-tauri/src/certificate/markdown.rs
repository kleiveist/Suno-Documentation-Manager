use super::*;

pub(super) fn evidence_register_markdown(evidence: &[&EvidenceItem]) -> String {
    evidence
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "### {}. {}\n\n- Evidence ID [System value]: `{}`\n- Original filename [Evidence-derived metadata]: `{}`\n- Managed filename [System value]: `{}`\n- Role [System value]: `{}`\n- Provenance [System value]: `{}`\n- Relative path [System value]: `{}`\n- Size [System value]: {} bytes\n- SHA-256 [System verification]: `{}`\n- Imported at [System value]: `{}`\n- Document title [User-confirmed fact]: {}\n- Provider/source [User-confirmed fact]: {}\n- Source URL [User-confirmed fact]: {}\n- Retrieval date [User-confirmed fact]: {}\n- Effective date [User-confirmed fact]: {}\n- Applicable production period [User-confirmed fact]: {}\n- Factual note [User-confirmed fact]: {}\n- Source global evidence ID [System value]: `{}`\n- Derived from evidence ID [System value]: `{}`\n- Generator version [System value]: `{}`\n- Generated disclosure text [System value]: {}\n\n",
                index + 1,
                item.role.as_str(),
                markdown_raw_value(&item.id),
                markdown_documented(&item.metadata.original_file_name),
                markdown_raw_value(&item.file_name),
                item.role.as_str(),
                item.provenance.as_str(),
                markdown_raw_value(&item.relative_path),
                item.size_bytes,
                markdown_optional_value(item.sha256.as_deref(), "NOT RECORDED"),
                markdown_documented(&item.imported_at),
                markdown_documented(&item.metadata.document_title),
                markdown_documented(&item.metadata.provider),
                markdown_documented(&item.metadata.source_url),
                markdown_documented(&item.metadata.retrieval_date),
                markdown_documented(&item.metadata.effective_date),
                markdown_documented(&item.metadata.applicable_production_period),
                markdown_documented(&item.metadata.factual_note),
                markdown_optional_value(item.source_global_evidence_id.as_deref(), "N/A"),
                markdown_optional_value(item.derived_from_evidence_id.as_deref(), "N/A"),
                markdown_optional_value(item.generator_version.as_deref(), "N/A"),
                item.generated_disclosure_text
                    .as_deref()
                    .map(markdown_documented)
                    .unwrap_or_else(|| "N/A".into()),
            )
        })
        .collect()
}

pub(super) fn terms_evidence_markdown(terms: &[&EvidenceItem]) -> String {
    if terms.is_empty() {
        return "No archived terms evidence recorded.\n".into();
    }
    terms
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "#### Terms evidence {} — `{}`\n\n- Evidence ID [System value]: `{}`\n- Document title [User-confirmed fact]: {}\n- Provider/source [User-confirmed fact]: {}\n- Source URL [User-confirmed fact]: {}\n- Retrieval date [User-confirmed fact]: {}\n- Effective date [User-confirmed fact]: {}\n- Applicable production period [User-confirmed fact]: {}\n- Factual note [User-confirmed fact]: {}\n- Relative path [System value]: `{}`\n- Original filename [Evidence-derived metadata]: `{}`\n- SHA-256 [System verification]: `{}`\n- Imported at [System value]: `{}`\n- Provenance [System value]: `{}`\n\n",
                index + 1,
                markdown_raw_value(&item.id),
                markdown_raw_value(&item.id),
                markdown_documented(&item.metadata.document_title),
                markdown_documented(&item.metadata.provider),
                markdown_documented(&item.metadata.source_url),
                markdown_documented(&item.metadata.retrieval_date),
                markdown_documented(&item.metadata.effective_date),
                markdown_documented(&item.metadata.applicable_production_period),
                markdown_documented(&item.metadata.factual_note),
                markdown_raw_value(&item.relative_path),
                markdown_documented(&item.metadata.original_file_name),
                markdown_optional_value(item.sha256.as_deref(), "NOT RECORDED"),
                markdown_documented(&item.imported_at),
                item.provenance.as_str(),
            )
        })
        .collect()
}

pub(super) fn suno_plan_context_markdown(fields: &TrackFields) -> String {
    format!(
        "- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n",
        markdown_documented(&fields.suno_plan_at_generation),
        markdown_documented(&fields.legacy_suno_plan_at_creation),
    )
}

pub(super) fn source_provenance_markdown(
    fields: &TrackFields,
    evidence: &[&EvidenceItem],
) -> String {
    let fields = fields.normalized_conditionals();
    let source_code = evidence_path(evidence, EvidenceRole::SourceCodeFile);
    let code_audio = evidence_path(evidence, EvidenceRole::CodeGeneratedAudioFile);
    let mut output = format!(
        "- External audio uploaded [User-confirmed fact]: {}\n- External audio source [User-confirmed fact]: {}\n- External audio provenance statement [User-confirmed fact]: {}\n- Own audio uploaded [User-confirmed fact]: {}\n- Own audio source [User-confirmed fact]: {}\n- Own audio provenance statement [User-confirmed fact]: {}\n- Third-party samples uploaded [User-confirmed fact]: {}\n- Third-party sample source [User-confirmed fact]: {}\n- Third-party sample provenance statement [User-confirmed fact]: {}\n- Code-based generation [User-confirmed fact]: {}\n- Source-code evidence [System value]: `{}`\n- Code-generated audio evidence [System value]: `{}`\n- Code-audio post-processing [User-confirmed fact]: {}\n- Code-audio post-processing operations [User-confirmed fact]: {}\n",
        recorded_bool(fields.external_audio_uploaded),
        markdown_conditional_text(
            fields.external_audio_uploaded,
            &fields.external_audio_source
        ),
        markdown_conditional_text(
            fields.external_audio_uploaded,
            &fields.external_audio_ownership
        ),
        recorded_bool(fields.own_audio_uploaded),
        markdown_conditional_text(fields.own_audio_uploaded, &fields.own_audio_source),
        markdown_conditional_text(fields.own_audio_uploaded, &fields.own_audio_ownership),
        recorded_bool(fields.third_party_samples_uploaded),
        markdown_conditional_text(
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_source,
        ),
        markdown_conditional_text(
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_ownership,
        ),
        recorded_bool(fields.code_based_generation),
        markdown_conditional_recorded_value(
            fields.code_based_generation,
            source_code,
            "NOT RECORDED",
        ),
        markdown_conditional_recorded_value(
            fields.code_based_generation,
            code_audio,
            "NOT RECORDED",
        ),
        conditional_answer(
            fields.code_based_generation,
            recorded_bool(fields.code_audio_post_processed),
        ),
        if fields.code_based_generation == Some(true)
            && fields.code_audio_post_processed == Some(true)
        {
            markdown_documented_string_list(&fields.code_audio_post_processing_operations)
        } else {
            "N/A".into()
        },
    );
    if fields.code_based_generation == Some(true) && fields.code_audio_post_processed == Some(true)
    {
        output.push_str(&format!(
            "- Other code-audio post-processing note [User-confirmed fact]: {}\n",
            markdown_documented(&fields.code_audio_post_processing_note)
        ));
    }
    output
}

pub(super) fn evidence_path<'a>(evidence: &'a [&EvidenceItem], role: EvidenceRole) -> &'a str {
    evidence
        .iter()
        .copied()
        .find(|item| item.role == role)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT RECORDED")
}

pub(super) fn human_contribution_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let mut output = format!(
        "- Human editing performed [User-confirmed fact]: {}\n- Confirmed human editing [User-confirmed fact]: {}\n- Desktop-PC editing after the Suno WAV [User-confirmed fact]: {}\n- Confirmed desktop-PC editing [User-confirmed fact]: {}\n",
        recorded_bool(fields.human_editing_performed),
        markdown_conditional_text(
            fields.human_editing_performed,
            &fields.human_editing_details
        ),
        recorded_bool(fields.post_export_editing_performed),
        markdown_conditional_text(
            fields.post_export_editing_performed,
            &fields.post_export_editing_details,
        ),
    );
    if fields.artwork_origin == "human" {
        output.push_str(&format!(
            "- Confirmed human artwork process [User-confirmed fact]: {}\n- Human artwork process notes [User-confirmed fact]: {}\n",
            markdown_documented_string_list(&fields.human_artwork_process_operations),
            markdown_documented(&fields.human_artwork_process_notes),
        ));
    } else if fields.artwork_origin == "ai_assisted" {
        output.push_str(&format!(
            "- Confirmed human artwork modifications [User-confirmed fact]: {}\n- Other human artwork change [User-confirmed fact]: {}\n",
            markdown_documented_string_list(&fields.human_artwork_modifications),
            markdown_documented(&fields.custom_artwork_change),
        ));
    }
    output
}

pub(super) fn suno_field_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let mut output = format!(
        "## F. Suno Generation Text Field\n\n- Suno Instrumental Mode Selected [User-confirmed fact]: {}\n- Generation Text Field Used [User-confirmed fact]: {}\n- Content Classification [User-confirmed fact]: {}\n- Vocal Lyrics Present [Classification-derived presentation]: {}\n- Structure Instructions Present [Classification-derived presentation]: {}\n- Vocal Intent [User-confirmed fact]: {}\n- Final Audio Contains Vocals [User-confirmed fact]: {}\n",
        recorded_bool(fields.instrumental_track),
        generation_text_field_used(&fields),
        content_classification(&fields),
        generation_field_vocal_lyrics_present(&fields),
        structure_instructions_present(&fields),
        suno_vocal_intent(&fields),
        final_audio_contains_vocals(&fields),
    );
    output.push_str("\n### Exact Generation Text Field Content\n\n");
    match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => output.push_str("N/A\n\n"),
        Some(_) if fields.suno_lyrics_field_text.trim().is_empty() => {
            output.push_str("NOT DOCUMENTED\n\n");
        }
        Some(_) => output.push_str(&markdown_fenced_text(&fields.suno_lyrics_field_text)),
        None => output.push_str("NOT DOCUMENTED\n\n"),
    }
    if !fields.lyrics_source.trim().is_empty() || !fields.lyrics_text.trim().is_empty() {
        output.push_str(
            "### F.1 Unclassified legacy lyrics data\n\n- Classification: **NOT DOCUMENTED**\n",
        );
        output.push_str(&format!(
            "- Legacy source value: {}\n- Legacy text: {}\n\nThis retained historical data is unclassified legacy data and is not a Vocal Lyrics claim.\n\n",
            markdown_documented(&fields.lyrics_source),
            markdown_documented(&fields.lyrics_text),
        ));
    }
    output
}

pub(super) fn markdown_fenced_text(value: &str) -> String {
    let maximum_backtick_run = value
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((maximum_backtick_run + 1).max(3));
    format!("{fence}text\n{value}\n{fence}\n\n")
}

pub(super) fn ai_audio_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let active = fields.generative_ai_used;
    format!(
        "- Generative AI used [User-confirmed fact]: {}\n- AI system [User-confirmed fact]: {}\n- AI-assisted audio elements [User-confirmed fact]: {}\n- AI-generated audio elements [User-confirmed fact]: {}\n- Real person voice intentionally imitated [User-confirmed fact]: {}\n- Real person's identity intentionally represented [User-confirmed fact]: {}\n- Real event represented as authentic recording [User-confirmed fact]: {}\n- Real location / institution / event presented as authentic AI recording [User-confirmed fact]: {}\n- Disclosure applied [User-confirmed fact]: {}\n- Disclosure locations [User-confirmed fact]: {}\n- Disclosure text [User-confirmed fact]: {}\n- Disclosure reason / note [User-confirmed fact]: {}\n- Deepfake-related indicator summary: {}\n- Suno style prompt [User-confirmed fact]: {}\n\nNo AI Act compliance, legal necessity, or legal safety determination is made.\n",
        recorded_bool(active),
        markdown_conditional_text(active, &fields.audio_ai_system),
        conditional_documentation_answer(active, fields.ai_assisted_audio_elements),
        conditional_documentation_answer(active, fields.ai_generated_audio_elements),
        conditional_documentation_answer(active, fields.real_person_voice_intentionally_imitated,),
        conditional_documentation_answer(
            active,
            fields.real_person_identity_intentionally_represented,
        ),
        conditional_documentation_answer(
            active,
            fields.real_event_represented_as_authentic_recording,
        ),
        conditional_documentation_answer(
            active,
            fields.real_location_institution_event_presented_as_authentic_ai_recording,
        ),
        conditional_documentation_answer(active, fields.audio_disclosure_applied),
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                markdown_documented_string_list(&fields.audio_disclosure_locations)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                markdown_documented(&fields.audio_disclosure_text)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::No)) => {
                markdown_documented(&fields.audio_disclosure_reason)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        deepfake_indicator_summary(&fields),
        markdown_documented(&fields.suno_style_prompt),
    )
}

pub(super) fn ai_artwork_markdown(fields: &TrackFields, evidence: &[EvidenceItem]) -> String {
    let fields = fields.normalized_conditionals();
    let artwork_present = !matches!(fields.artwork_origin.as_str(), "" | "none");
    let ai_artwork = matches!(
        fields.artwork_origin.as_str(),
        "ai_generated" | "ai_assisted"
    );
    format!(
        "- Artwork origin [User-confirmed fact]: {}\n- AI image service [User-confirmed fact]: {}\n- Human artwork process [User-confirmed fact]: {}\n- Human artwork modifications [User-confirmed fact]: {}\n- Depicts real person [User-confirmed fact]: {}\n- Real-person note [User-confirmed fact]: {}\n- Depicts real event [User-confirmed fact]: {}\n- Real-event note [User-confirmed fact]: {}\n- Trademark/logo [User-confirmed fact]: {}\n- Trademark/logo note [User-confirmed fact]: {}\n- Artwork disclosure applied [User-confirmed fact]: {}\n- Artwork disclosure deliberately not applied [User-confirmed fact]: {}\n- Artwork disclosure text [User-confirmed fact]: {}\n- Human-edited/final artwork comparison [System verification]: {}\n\n{}\n",
        markdown_documented(&fields.artwork_origin),
        if ai_artwork {
            markdown_documented(&fields.ai_image_service)
        } else {
            "N/A".into()
        },
        if fields.artwork_origin == "human" {
            markdown_documented_string_list(&fields.human_artwork_process_operations)
        } else if fields.artwork_origin == "ai_assisted" {
            if fields
                .human_artwork_modifications
                .iter()
                .any(|value| !value.trim().is_empty())
                || !fields.custom_artwork_change.trim().is_empty()
            {
                "YES".into()
            } else {
                "NOT DOCUMENTED".into()
            }
        } else {
            "N/A".into()
        },
        if fields.artwork_origin == "ai_assisted" {
            markdown_documented_string_list(&fields.human_artwork_modifications)
        } else {
            "N/A".into()
        },
        if artwork_present {
            recorded_bool(fields.depicts_real_person)
        } else {
            "N/A"
        },
        if artwork_present {
            markdown_applicable_note(fields.depicts_real_person, &fields.real_person_notes)
        } else {
            "N/A".into()
        },
        if artwork_present {
            recorded_bool(fields.depicts_real_event)
        } else {
            "N/A"
        },
        if artwork_present {
            markdown_applicable_note(fields.depicts_real_event, &fields.real_event_notes)
        } else {
            "N/A".into()
        },
        if artwork_present {
            recorded_bool(fields.contains_trademark)
        } else {
            "N/A"
        },
        if artwork_present {
            markdown_applicable_note(fields.contains_trademark, &fields.trademark_notes)
        } else {
            "N/A".into()
        },
        if ai_artwork {
            recorded_bool(fields.disclosure_applied)
        } else {
            "N/A"
        },
        match (ai_artwork, fields.disclosure_applied) {
            (true, Some(false)) => "YES",
            (true, Some(true)) | (false, _) => "N/A",
            (true, None) => "NOT DOCUMENTED",
        },
        if ai_artwork && fields.disclosure_applied == Some(true) {
            markdown_documented(&fields.disclosure_text)
        } else {
            "N/A".into()
        },
        crate::workflow::human_edited_final_artwork_status(evidence),
        crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE,
    )
}

pub(super) fn generation_text_field_used(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => "NO",
        Some(_) => "YES",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn content_classification(fields: &TrackFields) -> &'static str {
    fields
        .suno_content_classification
        .map(SunoContentClassification::as_str)
        .unwrap_or("NOT DOCUMENTED")
}

pub(super) fn generation_field_vocal_lyrics_present(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::VocalLyricsOnly | SunoContentClassification::Mixed) => {
            "YES"
        }
        Some(SunoContentClassification::StructureOnly) => "NO",
        Some(SunoContentClassification::Empty) => "N/A",
        Some(SunoContentClassification::Other) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn suno_vocal_intent(fields: &TrackFields) -> &'static str {
    fields
        .vocal_intent
        .map(VocalIntent::as_str)
        .unwrap_or("NOT DOCUMENTED")
}

pub(super) fn final_audio_contains_vocals(fields: &TrackFields) -> &'static str {
    recorded_bool(fields.vocal_lyrics_present)
}

pub(super) fn structure_instructions_present(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::StructureOnly | SunoContentClassification::Mixed) => "YES",
        Some(SunoContentClassification::VocalLyricsOnly) => "NO",
        Some(SunoContentClassification::Empty) => "N/A",
        Some(SunoContentClassification::Other) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn suno_content_source(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => "N/A",
        None => "NOT DOCUMENTED",
        Some(_) => match fields.suno_lyrics_content_source {
            Some(SunoLyricsContentSource::Human) => "human",
            Some(SunoLyricsContentSource::Ai) => "AI",
            Some(SunoLyricsContentSource::Mixed) => "mixed",
            None => "NOT DOCUMENTED",
        },
    }
}

pub(super) fn documentation_answer(value: Option<DocumentationAnswer>) -> &'static str {
    match value {
        Some(DocumentationAnswer::Yes) => "YES",
        Some(DocumentationAnswer::No) => "NO",
        Some(DocumentationAnswer::NotDocumented) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn conditional_documentation_answer(
    controlling: Option<bool>,
    value: Option<DocumentationAnswer>,
) -> &'static str {
    match controlling {
        Some(true) => documentation_answer(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn deepfake_indicator_summary(fields: &TrackFields) -> &'static str {
    if fields.generative_ai_used == Some(false) {
        return "N/A";
    }
    if fields.generative_ai_used != Some(true) {
        return "INCOMPLETE – generative AI use NOT DOCUMENTED";
    }
    let answers = [
        fields.real_person_voice_intentionally_imitated,
        fields.real_person_identity_intentionally_represented,
        fields.real_event_represented_as_authentic_recording,
        fields.real_location_institution_event_presented_as_authentic_ai_recording,
    ];
    if answers.contains(&Some(DocumentationAnswer::Yes)) {
        "Potential deepfake-related indicator recorded"
    } else if answers
        .iter()
        .all(|answer| *answer == Some(DocumentationAnswer::No))
    {
        "Documented deepfake-related indicators: none recorded"
    } else {
        "Deepfake-related indicator documentation: INCOMPLETE"
    }
}

pub(super) fn documented_string_list(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        values.join(" | ")
    }
}

pub(super) fn conditional_answer(controlling: Option<bool>, value: &str) -> &str {
    match controlling {
        Some(true) => value,
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn step_status_label(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::NotRun => "NOT_RUN",
        StepStatus::Pass => "PASS",
        StepStatus::Fail => "FAIL",
        StepStatus::Blocked => "BLOCKED",
        StepStatus::NotApplicable => "N/A",
        StepStatus::NotVerified => "NOT_VERIFIED",
    }
}

pub(super) fn documented(value: &str) -> &str {
    if value.trim().is_empty() {
        "NOT DOCUMENTED"
    } else {
        value
    }
}

pub(super) fn markdown_raw_value(value: &str) -> String {
    format!("{MARKDOWN_VALUE_START}{value}{MARKDOWN_VALUE_END}")
}

pub(super) fn markdown_documented(value: &str) -> String {
    if value.trim().is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        markdown_raw_value(value)
    }
}

pub(super) fn markdown_documented_string_list(values: &[String]) -> String {
    if values.iter().all(|value| value.trim().is_empty()) {
        "NOT DOCUMENTED".into()
    } else {
        markdown_raw_value(&documented_string_list(values))
    }
}

pub(super) fn markdown_conditional_text(controlling: Option<bool>, value: &str) -> String {
    match controlling {
        Some(true) => markdown_documented(value),
        Some(false) => "N/A".into(),
        None => "NOT DOCUMENTED".into(),
    }
}

pub(super) fn markdown_conditional_recorded_value(
    controlling: Option<bool>,
    value: &str,
    missing: &str,
) -> String {
    match controlling {
        Some(true) if value == missing || value.trim().is_empty() => missing.into(),
        Some(true) => markdown_raw_value(value),
        Some(false) => "N/A".into(),
        None => "NOT DOCUMENTED".into(),
    }
}

pub(super) fn markdown_applicable_note(answer: Option<bool>, value: &str) -> String {
    match answer {
        Some(true) => markdown_documented(value),
        Some(false) => "N/A".into(),
        None => "NOT DOCUMENTED".into(),
    }
}

pub(super) fn markdown_optional_value(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .map(markdown_raw_value)
        .unwrap_or_else(|| fallback.to_owned())
}

pub(super) fn fact_origin_label(origin: FactOrigin) -> &'static str {
    match origin {
        FactOrigin::UserConfirmedFact => "User-confirmed fact",
        FactOrigin::EvidenceDerivedMetadata => "Evidence-derived metadata",
        FactOrigin::NotDocumented => "NOT DOCUMENTED",
    }
}

pub(super) fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

pub(super) fn recorded_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT DOCUMENTED",
    }
}
