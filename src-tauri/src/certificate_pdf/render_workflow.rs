use super::*;

pub(super) fn render_workflow(
    layout: &mut PdfLayout,
    steps: &[StepState],
    labels: &HashMap<&str, &str>,
) {
    layout.section_title("K.1 Configured workflow checks");
    layout.column_heading("Step", "Authoritative status / N/A reason", 63.0);
    for step in steps {
        let mut value = step_status(&step.status).to_owned();
        if step.status == StepStatus::NotApplicable {
            value.push_str(" | ");
            value.push_str(&layout.label("N/A reason"));
            value.push_str(": ");
            value.push_str(
                step.na_reason
                    .as_deref()
                    .filter(|reason| !reason.trim().is_empty())
                    .unwrap_or("NOT DOCUMENTED"),
            );
        }
        layout.table_row(
            &TableRow::system_plain(
                labels
                    .get(step.id.as_str())
                    .copied()
                    .expect("validated workflow step label"),
                value,
            ),
            63.0,
        );
    }
}

/// K.2 is deliberately a compact factual rendering of the state frozen for
/// this certificate. It never prints the raw Chromaprint fingerprint, raw
/// provider response, request signature, or credentials.
pub(super) fn render_audio_screening(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("K.2 Pre-release audio screening");
    let external = &snapshot.track.audio_screening.external;
    let multi_sample = has_multi_sample_data(external);
    let rows = audio_screening_base_rows(snapshot, multi_sample);
    layout.table_rows(&rows, 78.0);

    let multi_sample_rows = multi_sample_pdf_rows(external);
    if !multi_sample_rows.is_empty() {
        layout.table_rows(&multi_sample_rows, 78.0);
    }

    if !external.samples.is_empty() {
        render_audio_sampling_visualization(layout, external);
        layout.write_localized(
            "Full millisecond-bound ACRCloud sample records are retained in the Technical Appendix.",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.0,
            BuiltinFont::HelveticaBold,
            1.3,
        );
    }

    render_provider_matches(layout, external, multi_sample);
    layout.write_localized(
        "Audio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion.",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::Helvetica,
        1.3,
    );
}

pub(super) fn aggregate_matches_are_repeated_in_samples(
    external: &AudioScreeningExternalRecord,
) -> bool {
    let sample_matches = external
        .samples
        .iter()
        .flat_map(|sample| sample.matches.iter())
        .collect::<Vec<_>>();
    sample_matches.len() == external.matches.len()
        && sample_matches
            .iter()
            .zip(&external.matches)
            .all(|(sample, aggregate)| *sample == aggregate)
}

/// K.2 reports the configuration-derived target separately from what was
/// actually executed. This makes a capped or shortened run reviewable without
/// claiming it is a complete rights assessment.
pub(super) fn multi_sample_pdf_rows(external: &AudioScreeningExternalRecord) -> Vec<TableRow> {
    if !has_multi_sample_data(external) {
        return Vec::new();
    }

    let calculation_mode = if external.dynamic_by_track_duration {
        "DYNAMIC BY TRACK DURATION"
    } else {
        "FIXED REFERENCE DURATION"
    };
    let reference_duration = external
        .reference_duration_seconds
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".into());
    let mut rows = vec![
        TableRow::system_plain(
            "External screening mode [System value]",
            external.screening_mode.as_str(),
        ),
        TableRow::plain(
            "Requested coverage [System value]",
            format!("{} %", external.requested_intensity_percent),
        ),
        TableRow::system_plain("Calculation mode [System value]", calculation_mode),
        TableRow::plain(
            "Reference duration (seconds) [System value]",
            reference_duration,
        ),
        TableRow::plain(
            "Target screening duration (ms) [System value]",
            external.target_duration_milliseconds.to_string(),
        ),
        TableRow::plain(
            "Planned requests [System value]",
            external.planned_request_count.to_string(),
        ),
        TableRow::plain(
            "Executed requests [System verification]",
            external.executed_request_count.to_string(),
        ),
        TableRow::plain(
            "Unique samples [System verification]",
            external.unique_sample_count.to_string(),
        ),
        TableRow::plain(
            "Duplicate samples [System verification]",
            external.duplicate_sample_count.to_string(),
        ),
        TableRow::plain(
            "Overlapping samples [System verification]",
            external.overlapping_sample_count.to_string(),
        ),
        TableRow::plain(
            "Unique sampled duration (ms) [System verification]",
            external.unique_sample_duration_milliseconds.to_string(),
        ),
        TableRow::plain(
            "Track coverage (%) [System verification]",
            format!("{:.2}", external.track_coverage_percent),
        ),
        TableRow::system_plain(
            "Provider configuration status [System verification]",
            audio_screening_provider_status_label(external.provider_status),
        ),
        TableRow::system_plain(
            "Overall result [System verification]",
            audio_screening_status_label(external.status),
        ),
    ];

    let provider_response = consolidated_provider_response(external);
    rows.push(TableRow::provider_summary(
        "Provider response [Provider-derived metadata]",
        provider_response.response,
        provider_response.response_is_system,
    ));
    rows.push(TableRow::provider_summary(
        "API version [Provider-derived metadata]",
        provider_response.api_version,
        provider_response.api_version_is_system,
    ));

    if external.samples.is_empty() {
        rows.push(TableRow::system_plain(
            "Sample results [System verification]",
            "NONE RECORDED",
        ));
        return rows;
    }

    rows
}

pub(super) fn has_multi_sample_data(external: &AudioScreeningExternalRecord) -> bool {
    external.screening_mode == AudioScreeningMode::MultiSample || !external.samples.is_empty()
}

pub(super) fn sample_result_label(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::NoMatchDetected => "NO MATCH",
        _ => audio_screening_status_label(status),
    }
}

pub(super) fn audio_screening_provider_status_label(
    status: AudioScreeningProviderStatus,
) -> &'static str {
    match status {
        AudioScreeningProviderStatus::Disabled => "DISABLED",
        AudioScreeningProviderStatus::NotConfigured => "NOT CONFIGURED",
        AudioScreeningProviderStatus::Ready => "READY",
        AudioScreeningProviderStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        AudioScreeningProviderStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        AudioScreeningProviderStatus::ConfigurationInvalid => "CONFIGURATION INVALID",
    }
}

pub(super) fn audio_screening_match_value(item: &crate::model::AudioScreeningMatch) -> String {
    let artists = if item.artists.is_empty() {
        "NOT DOCUMENTED".to_owned()
    } else {
        item.artists.join(", ")
    };
    let mut value = format!("{} — {artists}", documented(&item.title));
    if let Some(album) = nonempty(item.album.as_deref()) {
        value.push_str(&format!("; album {album}"));
    }
    if let Some(isrc) = nonempty(item.isrc.as_deref()) {
        value.push_str(&format!("; ISRC {isrc}"));
    }
    if let Some(acrid) = nonempty(item.acrid.as_deref()) {
        value.push_str(&format!("; ACRID {acrid}"));
    }
    if let Some(score) = item.score {
        value.push_str(&format!("; score {score}"));
    }
    value
}

pub(super) fn audio_screening_status_label(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::NotRun => "NOT RUN",
        AudioScreeningStatus::FingerprintGenerated => "FINGERPRINT GENERATED",
        AudioScreeningStatus::NoMatchDetected => "NO MATCH DETECTED",
        AudioScreeningStatus::MatchDetected => "MATCH DETECTED",
        AudioScreeningStatus::SkippedNotConfigured => "SKIPPED NOT CONFIGURED",
        AudioScreeningStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        AudioScreeningStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        AudioScreeningStatus::ConfigurationInvalid => "CONFIGURATION INVALID",
        AudioScreeningStatus::EngineUnavailable => "ENGINE UNAVAILABLE",
        AudioScreeningStatus::UnsupportedFormat => "UNSUPPORTED FORMAT",
        AudioScreeningStatus::ProcessingFailed => "PROCESSING FAILED",
        AudioScreeningStatus::Stale => "STALE",
    }
}

pub(super) fn render_evidence_register(layout: &mut PdfLayout, evidence: &[&EvidenceItem]) {
    layout.section_title("J. Evidence register");
    render_evidence_column_heading(layout);
    for (index, item) in evidence.iter().enumerate() {
        let digest = item
            .sha256
            .as_deref()
            .expect("validated certificate PDF evidence digest");
        prepare_evidence_item_page(layout, item, digest);
        render_evidence_identity(layout, index, item, digest);
        render_evidence_document_metadata(layout, item);
        render_evidence_role_metadata(layout, item);
        render_evidence_embedded_metadata(layout, item);
        layout.y_mm -= 1.8;
    }
}

fn audio_screening_base_rows(
    snapshot: &CertificatePdfSnapshot<'_>,
    multi_sample: bool,
) -> Vec<TableRow> {
    let local = &snapshot.track.audio_screening.local;
    let external = &snapshot.track.audio_screening.external;
    let mut rows = vec![
        TableRow::system_plain(
            "Local screening status [System verification]",
            audio_screening_status_label(local.status),
        ),
        TableRow::documented_plain("Local engine [System verification]", &local.engine),
        TableRow::documented_plain(
            "Local engine version [System verification]",
            &local.engine_version,
        ),
        TableRow::documented_plain(
            "Fingerprint algorithm [System verification]",
            &local.fingerprint_algorithm,
        ),
        TableRow::documented_mono(
            "Local source Evidence ID [System verification]",
            &local.source_evidence_id,
        ),
        TableRow::documented_mono(
            "Local source path [System verification]",
            &local.source_relative_path,
        ),
        TableRow::documented_mono(
            "Local source SHA-256 [System verification]",
            &local.source_sha256,
        ),
        TableRow::plain(
            "Local source size (bytes) [System verification]",
            local.source_size_bytes.to_string(),
        ),
        TableRow::optional_u64_plain(
            "Local measured duration (ms) [System verification]",
            local.duration_milliseconds,
            "NOT DOCUMENTED",
        ),
        TableRow::documented_mono(
            "Local record path [System verification]",
            &local.artifact_relative_path,
        ),
        TableRow::documented_mono(
            "Local record SHA-256 [System verification]",
            &local.artifact_sha256,
        ),
        TableRow::optional_plain(
            "Local generated at [System value]",
            local.generated_at.as_deref(),
            "NOT RECORDED",
        ),
        TableRow::documented_plain(
            "External screening provider [System value]",
            &external.provider,
        ),
        TableRow::system_plain(
            "External screening status [System verification]",
            audio_screening_status_label(external.status),
        ),
        TableRow::system_plain(
            "External provider configured at snapshot [System value]",
            yes_no(external.configured_at_snapshot),
        ),
        TableRow::documented_mono(
            "External source Evidence ID [System verification]",
            &external.source_evidence_id,
        ),
        TableRow::documented_mono(
            "External source path [System verification]",
            &external.source_relative_path,
        ),
        TableRow::documented_mono(
            "External source SHA-256 [System verification]",
            &external.source_sha256,
        ),
        TableRow::optional_plain(
            "External checked at [System value]",
            external.checked_at.as_deref(),
            "NOT RECORDED",
        ),
        TableRow::optional_u64_plain(
            "External source duration (ms) [System value]",
            external.source_duration_milliseconds,
            "NOT DOCUMENTED",
        ),
        TableRow::plain(
            "External request count [System value]",
            external.request_count.to_string(),
        ),
        TableRow::optional_mono(
            "External response archive [System verification]",
            external.response_relative_path.as_deref(),
            "NOT RECORDED",
        ),
        TableRow::optional_mono(
            "External response SHA-256 [System verification]",
            external.response_sha256.as_deref(),
            "NOT RECORDED",
        ),
    ];
    insert_legacy_sample_rows(&mut rows, external, multi_sample);
    rows
}

fn insert_legacy_sample_rows(
    rows: &mut Vec<TableRow>,
    external: &AudioScreeningExternalRecord,
    multi_sample: bool,
) {
    if multi_sample {
        return;
    }
    // Keep the historical one-sample labels in their original position;
    // multi-sample records render the per-sample offset and duration rows
    // below instead of showing a misleading legacy placeholder.
    rows.insert(
        19,
        TableRow::optional_u64_plain(
            "External sample offset (ms) [System value]",
            external.sample_offset_milliseconds,
            "NOT DOCUMENTED",
        ),
    );
    rows.insert(
        20,
        TableRow::optional_u64_plain(
            "External sample duration (ms) [System value]",
            external.sample_duration_milliseconds,
            "NOT DOCUMENTED",
        ),
    );
}

fn render_provider_matches(
    layout: &mut PdfLayout,
    external: &AudioScreeningExternalRecord,
    multi_sample: bool,
) {
    let sample_match_count = external
        .samples
        .iter()
        .map(|sample| sample.matches.len())
        .sum::<usize>();
    if external.matches.is_empty() && sample_match_count == 0 {
        layout.table_row(
            &TableRow::system_plain(
                "Provider matches [Provider-derived metadata]",
                "NONE RECORDED",
            ),
            78.0,
        );
    } else if external.matches.is_empty() {
        layout.table_row(
            &TableRow::plain(
                "Provider matches [Provider-derived metadata]",
                format!(
                    "{} — {}",
                    sample_match_count,
                    layout.label("recorded; complete per-sample records in Technical Appendix")
                ),
            ),
            78.0,
        );
    } else if multi_sample && aggregate_matches_are_repeated_in_samples(external) {
        layout.table_row(
            &TableRow::plain(
                "Provider matches [Provider-derived metadata]",
                format!(
                    "{} — {}",
                    external.matches.len(),
                    layout.label("recorded; complete per-sample records in Technical Appendix")
                ),
            ),
            78.0,
        );
    } else {
        for (index, item) in external.matches.iter().enumerate() {
            layout.table_row(
                &TableRow::plain(
                    format!("Provider match {} [Provider-derived metadata]", index + 1),
                    audio_screening_match_value(item),
                ),
                78.0,
            );
        }
    }
}

fn prepare_evidence_item_page(layout: &mut PdfLayout, item: &EvidenceItem, digest: &str) {
    let minimum_height = 16.0
        + estimated_row_height(&item.relative_path, 144.0, 6.4, BuiltinFont::Courier)
        + estimated_row_height(digest, 144.0, 6.4, BuiltinFont::Courier);
    if !layout.fits(minimum_height) && minimum_height <= layout.full_content_height() {
        layout.new_page();
        layout.continuation_title("Evidence Register (continued)");
        render_evidence_column_heading(layout);
    }
}

fn render_evidence_identity(
    layout: &mut PdfLayout,
    index: usize,
    item: &EvidenceItem,
    digest: &str,
) {
    layout.evidence_summary_row(
        index + 1,
        item.role.as_str(),
        item.provenance.as_str(),
        item.size_bytes,
    );
    layout.table_row(
        &TableRow::mono("Evidence ID [System value]", &item.id),
        48.0,
    );
    layout.table_row(
        &TableRow::plain("File name [System value]", &item.file_name),
        48.0,
    );
    layout.table_row(
        &TableRow::documented_plain(
            "Original file name [Evidence-derived metadata]",
            &item.metadata.original_file_name,
        ),
        56.0,
    );
    layout.table_row(
        &TableRow::mono("Relative path [System value]", &item.relative_path),
        48.0,
    );
    layout.table_row(
        &TableRow::mono("SHA-256 [System verification]", digest),
        58.0,
    );

    let imported_at = if item.imported_at.trim().is_empty() {
        TableRow::system_mono("Imported at [System value]", "NOT DOCUMENTED")
    } else {
        TableRow::mono("Imported at [System value]", &item.imported_at)
    };
    layout.table_row(&imported_at, 48.0);
    if let Some(value) = nonempty(item.coverage_start.as_deref()) {
        layout.table_row(&TableRow::mono("Coverage start", value), 34.0);
    }
    if let Some(value) = nonempty(item.coverage_end.as_deref()) {
        layout.table_row(&TableRow::mono("Coverage end", value), 34.0);
    }
    if let Some(value) = nonempty(item.source_global_evidence_id.as_deref()) {
        layout.table_row(&TableRow::mono("Source global evidence ID", value), 45.0);
    }
    if let Some(value) = nonempty(item.derived_from_evidence_id.as_deref()) {
        layout.table_row(&TableRow::mono("Derived from evidence ID", value), 45.0);
    }
    if let Some(value) = nonempty(item.generator_version.as_deref()) {
        layout.table_row(&TableRow::mono("Generator version", value), 38.0);
    }
    if let Some(value) = nonempty(item.generated_disclosure_text.as_deref()) {
        layout.table_row(&TableRow::plain("Generated disclosure text", value), 45.0);
    }
}

fn render_evidence_document_metadata(layout: &mut PdfLayout, item: &EvidenceItem) {
    for (label, value, mono) in [
        (
            "Document title [User-confirmed fact]",
            item.metadata.document_title.as_str(),
            false,
        ),
        (
            "Provider/source [User-confirmed fact]",
            item.metadata.provider.as_str(),
            false,
        ),
        (
            "Source URL [User-confirmed fact]",
            item.metadata.source_url.as_str(),
            true,
        ),
        (
            "Retrieval date [User-confirmed fact]",
            item.metadata.retrieval_date.as_str(),
            false,
        ),
        (
            "Effective date [User-confirmed fact]",
            item.metadata.effective_date.as_str(),
            false,
        ),
        (
            "Applicable production period [User-confirmed fact]",
            item.metadata.applicable_production_period.as_str(),
            false,
        ),
        (
            "Factual note [User-confirmed fact]",
            item.metadata.factual_note.as_str(),
            false,
        ),
    ] {
        if !value.trim().is_empty() {
            let row = if mono {
                TableRow::mono(label, value)
            } else {
                TableRow::plain(label, value)
            };
            layout.table_row(&row, 66.0);
        }
    }
}

fn render_evidence_role_metadata(layout: &mut PdfLayout, item: &EvidenceItem) {
    // Preserve populated role-specific metadata instead of reducing each
    // evidence item to its common identity fields. Long embedded/raw
    // values use ordinary paginating table rows, so completeness never
    // depends on fitting a single fixed-height block.
    for (label, value, mono) in [
        (
            "File extension [Evidence-derived metadata]",
            item.metadata.file_extension.as_str(),
            true,
        ),
        (
            "MIME type [Evidence-derived metadata]",
            item.metadata.mime_type.as_str(),
            true,
        ),
        (
            "Audio format [Evidence-derived metadata]",
            item.metadata.audio_format.as_str(),
            false,
        ),
        (
            "Suno created timestamp [Evidence-derived metadata]",
            item.metadata.suno_created_timestamp.as_str(),
            true,
        ),
        (
            "Suno created date [Evidence-derived metadata]",
            item.metadata.suno_created_date.as_str(),
            true,
        ),
        (
            "Suno ID [Evidence-derived metadata]",
            item.metadata.suno_id.as_str(),
            true,
        ),
        (
            "Raw Suno metadata [Evidence-derived metadata]",
            item.metadata.suno_raw_metadata.as_str(),
            true,
        ),
    ] {
        if !value.trim().is_empty() {
            let row = if mono {
                TableRow::mono(label, value)
            } else {
                TableRow::plain(label, value)
            };
            layout.table_row(&row, 66.0);
        }
    }
    for (label, value) in [
        (
            "Audio channels [Evidence-derived metadata]",
            item.metadata.audio_channels.map(|value| value.to_string()),
        ),
        (
            "Audio sample rate (Hz) [Evidence-derived metadata]",
            item.metadata
                .audio_sample_rate_hz
                .map(|value| value.to_string()),
        ),
        (
            "Audio duration (ms) [Evidence-derived metadata]",
            item.metadata
                .audio_duration_milliseconds
                .map(|value| value.to_string()),
        ),
        (
            "Audio bit depth [Evidence-derived metadata]",
            item.metadata.audio_bit_depth.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            layout.table_row(&TableRow::mono(label, value), 66.0);
        }
    }
    let has_suno_metadata = item.metadata.suno_studio_detected
        || !item.metadata.suno_created_timestamp.trim().is_empty()
        || !item.metadata.suno_created_date.trim().is_empty()
        || !item.metadata.suno_id.trim().is_empty()
        || !item.metadata.suno_raw_metadata.trim().is_empty();
    if has_suno_metadata {
        layout.table_row(
            &TableRow::system_plain(
                "Suno Studio metadata detected [System verification]",
                yes_no_bool(item.metadata.suno_studio_detected),
            ),
            66.0,
        );
    }
}

fn render_evidence_embedded_metadata(layout: &mut PdfLayout, item: &EvidenceItem) {
    for (metadata_index, metadata) in item.metadata.embedded_metadata.iter().enumerate() {
        if !metadata.key.trim().is_empty() {
            layout.table_row(
                &TableRow::plain(
                    format!("Embedded metadata {} key", metadata_index + 1),
                    &metadata.key,
                ),
                66.0,
            );
        }
        if !metadata.value.trim().is_empty() {
            layout.table_row(
                &TableRow::plain(
                    format!("Embedded metadata {} value", metadata_index + 1),
                    &metadata.value,
                ),
                66.0,
            );
        }
    }
}
