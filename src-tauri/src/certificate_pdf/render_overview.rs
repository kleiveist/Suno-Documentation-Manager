use super::*;

pub(super) fn render_certificate_summary(layout: &mut PdfLayout, view: &CertificateViewModel<'_>) {
    let snapshot = view.snapshot;
    layout.mark_bookmark("Summary", layout.current_page_index());
    render_summary_heading(layout, snapshot);
    render_summary_artwork(layout, view);
    render_summary_completion(layout, view);
    render_summary_groups(layout, view);
    layout.text_at(
        LEFT_MM,
        31.0,
        &layout.paragraph(
            "Status values describe documentation and technical checks only; they are not a rights clearance or legal approval.",
        ),
        6.4,
        BuiltinFont::Helvetica,
    );
}

fn render_summary_heading(layout: &mut PdfLayout, snapshot: &CertificatePdfSnapshot<'_>) {
    let document_title = layout.label(DOCUMENT_TITLE);
    layout.text_at(
        LEFT_MM,
        278.0,
        &document_title,
        fit_text_size(
            &document_title,
            RIGHT_MM - LEFT_MM,
            15.5,
            11.0,
            BuiltinFont::HelveticaBold,
        ),
        BuiltinFont::HelveticaBold,
    );
    let boundary =
        layout.paragraph("Technical evidence report — not a legal or governmental certification");
    layout.text_at(LEFT_MM, 269.5, &boundary, 7.2, BuiltinFont::Helvetica);
    layout.rule(LEFT_MM, RIGHT_MM, 265.0, 0.65);

    let title = documented(&snapshot.track.fields.title);
    for (index, line) in bounded_wrapped_lines(title, 112.0, 15.0, BuiltinFont::HelveticaBold, 2)
        .iter()
        .enumerate()
    {
        layout.text_at(
            LEFT_MM,
            252.0 - index as f32 * 6.5,
            line,
            15.0,
            BuiltinFont::HelveticaBold,
        );
    }
    for (index, line) in bounded_wrapped_lines(
        documented(&snapshot.profile.artist_name),
        112.0,
        9.5,
        BuiltinFont::Helvetica,
        2,
    )
    .iter()
    .enumerate()
    {
        layout.text_at(
            LEFT_MM,
            238.0 - index as f32 * 5.5,
            line,
            9.5,
            BuiltinFont::Helvetica,
        );
    }

    let metadata = [
        ("Certificate ID", snapshot.certificate_id, true),
        ("Finalization timestamp", snapshot.finalized_at, true),
        ("Application version", env!("CARGO_PKG_VERSION"), false),
        (
            "Workflow version",
            snapshot.track.workflow_version.as_str(),
            false,
        ),
        ("Certificate version", snapshot.certificate_version, false),
    ];
    let mut metadata_y = 224.0;
    for (label, value, mono) in metadata {
        let label = layout.label(label);
        layout.text_at(LEFT_MM, metadata_y, &label, 6.1, BuiltinFont::HelveticaBold);
        let font = if mono {
            BuiltinFont::Courier
        } else {
            BuiltinFont::Helvetica
        };
        let size = fit_text_size(value, 74.0, 6.8, 5.5, font);
        let displayed = ellipsize_text_to_width(value, 74.0, size, font);
        layout.text_at(LEFT_MM + 38.0, metadata_y, &displayed, size, font);
        metadata_y -= 7.0;
    }
}

fn render_summary_artwork(layout: &mut PdfLayout, view: &CertificateViewModel<'_>) {
    let artwork_x = 136.0;
    let artwork_top = 255.0;
    let artwork_size = 54.0;
    if let Some(preview) = view.final_artwork_preview {
        let (_, rendered_height) =
            layout.image_at(preview, artwork_x, artwork_top, artwork_size, artwork_size);
        let caption_y = artwork_top - rendered_height - 5.0;
        layout.text_at(
            artwork_x,
            caption_y,
            &layout.label("Final artwork preview"),
            6.1,
            BuiltinFont::HelveticaBold,
        );
        let caption_size = fit_text_size(
            &preview.file_name,
            artwork_size,
            5.8,
            5.5,
            BuiltinFont::Helvetica,
        );
        let caption = ellipsize_text_to_width(
            &preview.file_name,
            artwork_size,
            caption_size,
            BuiltinFont::Helvetica,
        );
        layout.text_at(
            artwork_x,
            caption_y - 5.0,
            &caption,
            caption_size,
            BuiltinFont::Helvetica,
        );
        layout.text_at(
            artwork_x,
            caption_y - 10.0,
            &format!("SHA-256 {}", abbreviated_sha256(&preview.sha256)),
            5.5,
            BuiltinFont::Courier,
        );
    } else {
        layout.box_outline(artwork_x, artwork_top, artwork_size, artwork_size, 0.4);
        let fallback = layout.paragraph("Artwork preview not available");
        let lines = wrap_text(
            &fallback,
            artwork_size - 8.0,
            7.0,
            BuiltinFont::HelveticaBold,
        );
        for (index, line) in lines.iter().enumerate() {
            layout.text_at(
                artwork_x + 4.0,
                artwork_top - 23.0 - index as f32 * 5.0,
                line,
                7.0,
                BuiltinFont::HelveticaBold,
            );
        }
        if let Some(item) = view.final_artwork {
            let caption_size = fit_text_size(
                &item.file_name,
                artwork_size,
                5.8,
                5.5,
                BuiltinFont::Helvetica,
            );
            let caption = ellipsize_text_to_width(
                &item.file_name,
                artwork_size,
                caption_size,
                BuiltinFont::Helvetica,
            );
            layout.text_at(
                artwork_x,
                artwork_top - artwork_size - 5.0,
                &caption,
                caption_size,
                BuiltinFont::Helvetica,
            );
            if let Some(digest) = item.sha256.as_deref() {
                layout.text_at(
                    artwork_x,
                    artwork_top - artwork_size - 10.0,
                    &format!("SHA-256 {}", abbreviated_sha256(digest)),
                    5.5,
                    BuiltinFont::Courier,
                );
            }
        }
    }
}

fn render_summary_completion(layout: &mut PdfLayout, view: &CertificateViewModel<'_>) {
    layout.box_outline(LEFT_MM, 188.0, RIGHT_MM - LEFT_MM, 24.0, 0.65);
    layout.status_icon_at(LEFT_MM + 5.0, 177.8, StatusTone::Success);
    layout.status_text_at(
        LEFT_MM + 11.0,
        177.0,
        &format!("PASS  {}", layout.label("DOCUMENTATION COMPLETE")),
        12.0,
        BuiltinFont::HelveticaBold,
        StatusTone::Success,
    );
    layout.text_at(
        LEFT_MM + 5.0,
        169.5,
        &layout.paragraph("Configured documentation requirements completed"),
        7.0,
        BuiltinFont::Helvetica,
    );
    layout.text_at(
        139.0,
        169.5,
        &format!(
            "{}: {}",
            layout.label("Blocking deviations"),
            view.open_blocking_deviations
        ),
        7.0,
        BuiltinFont::HelveticaBold,
    );
}

fn render_summary_groups(layout: &mut PdfLayout, view: &CertificateViewModel<'_>) {
    let positions = [
        (LEFT_MM, 157.0),
        (107.0, 157.0),
        (LEFT_MM, 118.0),
        (107.0, 118.0),
        (LEFT_MM, 79.0),
        (107.0, 79.0),
    ];
    for (group, (x_mm, top_mm)) in view.groups.iter().zip(positions) {
        render_summary_group(layout, group, x_mm, top_mm, 87.0, 34.0);
    }
}

pub(super) fn render_summary_group(
    layout: &mut PdfLayout,
    group: &OverviewGroup,
    x_mm: f32,
    top_mm: f32,
    width_mm: f32,
    height_mm: f32,
) {
    layout.box_outline(x_mm, top_mm, width_mm, height_mm, 0.3);
    layout.text_at(
        x_mm + 3.0,
        top_mm - 6.0,
        &layout.label(group.title),
        6.2,
        BuiltinFont::HelveticaBold,
    );
    let mut y_mm = top_mm - 13.0;
    for fact in group.facts.iter().take(4) {
        let label = layout.label(fact.label);
        let label_size = fit_text_size(&label, width_mm * 0.50, 5.8, 5.5, BuiltinFont::Helvetica);
        let label =
            ellipsize_text_to_width(&label, width_mm * 0.50, label_size, BuiltinFont::Helvetica);
        layout.text_at(x_mm + 3.0, y_mm, &label, label_size, BuiltinFont::Helvetica);
        let value = if fact.localize_value {
            layout.label(&fact.value)
        } else {
            fact.value.clone()
        };
        // The tone is already conveyed by the adjacent shape/color marker.
        // Repeating PASS/NOTICE/ERROR consumed the small summary value column
        // and forced otherwise readable German values into tiny type.
        let displayed = value;
        let value_x = x_mm + width_mm * 0.52;
        layout.status_icon_at(value_x, y_mm + 0.6, fact.tone);
        let value_width = width_mm * 0.47 - 4.8;
        let value_size = fit_text_size(
            &displayed,
            value_width,
            5.9,
            5.5,
            BuiltinFont::HelveticaBold,
        );
        let displayed = ellipsize_text_to_width(
            &displayed,
            value_width,
            value_size,
            BuiltinFont::HelveticaBold,
        );
        layout.status_text_at(
            value_x + 4.8,
            y_mm,
            &displayed,
            value_size,
            BuiltinFont::HelveticaBold,
            fact.tone,
        );
        y_mm -= 5.8;
    }
}

pub(super) fn render_evidence_overview(layout: &mut PdfLayout, view: &CertificateViewModel<'_>) {
    layout.mark_bookmark("Evidence Overview", layout.current_page_index());
    layout.text_at(
        LEFT_MM,
        265.0,
        &layout.label("Evidence Overview"),
        14.0,
        BuiltinFont::HelveticaBold,
    );
    layout.rule(LEFT_MM, RIGHT_MM, 259.0, 0.6);
    layout.y_mm = 252.0;
    render_production_timeline(layout, view.snapshot);
    render_artwork_process_overview(layout, view.snapshot);
    render_audio_sampling_visualization(layout, &view.snapshot.track.audio_screening.external);

    layout.section_title("Screening and workflow snapshot");
    let local = &view.snapshot.track.audio_screening.local;
    let external = &view.snapshot.track.audio_screening.external;
    layout.table_rows(
        &[
            TableRow::system_plain("Chromaprint", audio_screening_status_label(local.status)),
            TableRow::system_plain("ACRCloud", audio_screening_status_label(external.status)),
            TableRow::provider_summary(
                "Provider response",
                &view.provider_response.response,
                view.provider_response.response_is_system,
            ),
            TableRow::provider_summary(
                "API version",
                &view.provider_response.api_version,
                view.provider_response.api_version_is_system,
            ),
            TableRow::plain(
                "Workflow checks",
                format!(
                    "{} PASS · {} N/A",
                    view.workflow_pass_count, view.workflow_na_count
                ),
            ),
        ],
        52.0,
    );
}

pub(super) fn render_production_timeline(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("Track / production timeline");
    let fields = &snapshot.track.fields;
    let points = [
        ("Production start", fields.production_start_date.as_str()),
        (
            "Final generation",
            fields.suno_final_generation_date.as_str(),
        ),
        (
            "Technical finalization",
            snapshot
                .finalized_at
                .get(..10)
                .unwrap_or(snapshot.finalized_at),
        ),
    ]
    .into_iter()
    .filter_map(|(label, value)| parse_iso_date(value).map(|date| (label, value, date)))
    .collect::<Vec<_>>();
    let subscriptions = snapshot
        .evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SubscriptionPayment)
        .filter_map(|item| {
            let start_value = nonempty(item.coverage_start.as_deref())?;
            let end_value = nonempty(item.coverage_end.as_deref())?;
            Some((
                start_value,
                end_value,
                parse_iso_date(start_value)?,
                parse_iso_date(end_value)?,
            ))
        })
        .collect::<Vec<_>>();

    let mut all_dates = points.iter().map(|point| point.2).collect::<Vec<_>>();
    for range in &subscriptions {
        all_dates.extend([range.2, range.3]);
    }
    let Some(minimum) = all_dates.iter().min().copied() else {
        layout.write_localized(
            "Timeline not available — no documented dates",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.0,
            BuiltinFont::Helvetica,
            1.3,
        );
        return;
    };
    let maximum = all_dates.iter().max().copied().unwrap_or(minimum);
    let axis_start = LEFT_MM + 8.0;
    let axis_width = RIGHT_MM - LEFT_MM - 16.0;
    // Keep the event ticks strictly proportional while moving only colliding
    // callouts onto separate vertical lanes. Closely spaced production dates
    // otherwise make both the date and event labels unreadable.
    let axis_y = layout.y_mm - 22.0;
    layout.rule(axis_start, axis_start + axis_width, axis_y, 0.55);
    let point_positions = points
        .iter()
        .map(|point| timeline_x(point.2, minimum, maximum, axis_start, axis_width))
        .collect::<Vec<_>>();
    let label_lanes = timeline_label_lanes(&point_positions);
    for (index, (label, value, _date)) in points.iter().enumerate() {
        let x = point_positions[index];
        layout.vertical_rule(x, axis_y + 2.0, axis_y - 2.0, 0.7);
        let label = layout.label(label);
        let label_size = fit_text_size(&label, 36.0, 5.7, 5.5, BuiltinFont::Helvetica);
        let label = ellipsize_text_to_width(&label, 36.0, label_size, BuiltinFont::Helvetica);
        let text_x = (x - 18.0).clamp(LEFT_MM, RIGHT_MM - 36.0);
        let date_y = axis_y + 19.0 - label_lanes[index] as f32 * 6.5;
        layout.text_at(text_x, date_y, value, 5.7, BuiltinFont::Courier);
        layout.text_at(
            text_x,
            date_y - 4.3,
            &label,
            label_size,
            BuiltinFont::Helvetica,
        );
    }
    let mut range_y = axis_y - 7.0;
    for (index, (start_value, end_value, start, end)) in subscriptions.iter().take(4).enumerate() {
        let from_x = timeline_x(*start, minimum, maximum, axis_start, axis_width);
        let to_x = timeline_x(*end, minimum, maximum, axis_start, axis_width);
        layout.rule(from_x, to_x.max(from_x + 0.8), range_y, 1.2);
        layout.text_at(
            axis_start,
            range_y - 4.0,
            &format!(
                "{} {}: {} – {}",
                layout.label("Subscription evidence"),
                index + 1,
                start_value,
                end_value
            ),
            5.5,
            BuiltinFont::Helvetica,
        );
        range_y -= 7.0;
    }
    layout.y_mm = if subscriptions.is_empty() {
        axis_y - 6.0
    } else {
        range_y - 1.0
    };
}

pub(super) fn timeline_label_lanes(positions: &[f32]) -> Vec<usize> {
    const CALLOUT_WIDTH_MM: f32 = 38.0;
    const LANE_COUNT: usize = 3;

    let mut sorted = positions.iter().copied().enumerate().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.1.total_cmp(&right.1));
    let mut lanes = vec![0; positions.len()];
    let mut last_position_by_lane = [None; LANE_COUNT];
    for (original_index, position) in sorted {
        let lane = last_position_by_lane
            .iter()
            .position(|previous| {
                previous.is_none_or(|previous| position - previous >= CALLOUT_WIDTH_MM)
            })
            .unwrap_or(LANE_COUNT - 1);
        lanes[original_index] = lane;
        last_position_by_lane[lane] = Some(position);
    }
    lanes
}

pub(super) fn render_artwork_process_overview(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    let roles = [
        EvidenceRole::ArtworkSunoOriginal,
        EvidenceRole::AiArtworkOriginal,
        EvidenceRole::AiArtworkEdited,
        EvidenceRole::HumanEditedArtwork,
        EvidenceRole::FinalArtwork,
        EvidenceRole::ReleaseArtwork,
    ];
    let stages = roles
        .into_iter()
        .filter_map(|role| {
            snapshot
                .evidence
                .iter()
                .copied()
                .find(|item| item.role == role)
                .map(|item| (role, item))
        })
        .collect::<Vec<_>>();
    if stages.is_empty() {
        layout.section_title("Artwork process overview");
        layout.write_localized(
            "Artwork preview not available",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.0,
            BuiltinFont::Helvetica,
            1.3,
        );
        return;
    }

    let gap_mm = 5.0;
    let stage_width =
        ((RIGHT_MM - LEFT_MM) - gap_mm * (stages.len() - 1) as f32) / stages.len() as f32;
    let maximum_name_lines = stages
        .iter()
        .map(|(_, item)| wrap_text(&item.file_name, stage_width, 5.5, BuiltinFont::Helvetica).len())
        .max()
        .unwrap_or(1);
    let stage_height = (51.0 + maximum_name_lines as f32 * 3.7).max(55.0);
    // Reserve the complete thumbnail/caption band plus its factual note so a
    // long file name cannot collide with the following audio section.
    layout.ensure_space(13.0 + stage_height + 10.0);
    layout.section_title("Artwork process overview");
    let top = layout.y_mm;
    for (index, (role, item)) in stages.iter().enumerate() {
        let preview = snapshot
            .artwork_previews
            .iter()
            .find(|preview| preview.role == *role);
        render_artwork_stage(
            layout,
            preview,
            *role,
            item,
            (index, stages.len()),
            (stage_width, top),
        );
    }
    layout.y_mm = top - stage_height;
    layout.write_localized(
        "Artwork images are visual previews only. Evidence ID, registered path, and full SHA-256 remain authoritative in the Evidence Register.",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        6.2,
        BuiltinFont::Helvetica,
        1.25,
    );
}

fn render_artwork_stage(
    layout: &mut PdfLayout,
    preview: Option<&CertificateArtworkPreview>,
    role: EvidenceRole,
    item: &EvidenceItem,
    position: (usize, usize),
    geometry: (f32, f32),
) {
    let (index, stage_count) = position;
    let (stage_width, top) = geometry;
    let x = LEFT_MM + index as f32 * (stage_width + 5.0);
    let image_height = 28.0;
    if let Some(preview) = preview {
        layout.image_at(preview, x, top, stage_width, image_height);
    } else {
        layout.box_outline(x, top, stage_width, image_height, 0.35);
        let fallback = layout.label("Preview N/A");
        layout.text_at(
            x + 3.0,
            top - 14.0,
            &fallback,
            fit_text_size(
                &fallback,
                stage_width - 6.0,
                6.0,
                5.5,
                BuiltinFont::HelveticaBold,
            ),
            BuiltinFont::HelveticaBold,
        );
    }
    let role_label = layout.label(artwork_role_label(role));
    let role_size = fit_text_size(
        &role_label,
        stage_width,
        6.0,
        5.5,
        BuiltinFont::HelveticaBold,
    );
    let role_label = ellipsize_text_to_width(
        &role_label,
        stage_width,
        role_size,
        BuiltinFont::HelveticaBold,
    );
    layout.text_at(
        x,
        top - image_height - 5.0,
        &role_label,
        role_size,
        BuiltinFont::HelveticaBold,
    );
    let name_lines = wrap_text(&item.file_name, stage_width, 5.5, BuiltinFont::Helvetica);
    for (line_index, line) in name_lines.iter().enumerate() {
        layout.text_at(
            x,
            top - image_height - 10.0 - line_index as f32 * 3.7,
            line,
            5.5,
            BuiltinFont::Helvetica,
        );
    }
    if let Some(digest) = item.sha256.as_deref() {
        layout.text_at(
            x,
            top - image_height - 18.0 - name_lines.len() as f32 * 3.7,
            &abbreviated_sha256(digest),
            5.5,
            BuiltinFont::Courier,
        );
    }
    if index + 1 < stage_count {
        layout.text_at(
            x + stage_width + 1.1,
            top - image_height / 2.0,
            ">",
            7.0,
            BuiltinFont::HelveticaBold,
        );
    }
}

pub(super) fn render_audio_sampling_visualization(
    layout: &mut PdfLayout,
    external: &AudioScreeningExternalRecord,
) {
    layout.section_title("Audio sampling visualization");
    if external.samples.is_empty() {
        layout.table_row(
            &TableRow::system_plain(
                "External screening",
                audio_screening_status_label(external.status),
            ),
            44.0,
        );
        return;
    }
    let Some(duration) = external
        .source_duration_milliseconds
        .filter(|value| *value > 0)
    else {
        layout.write_localized(
            "Proportional visualization unavailable — source duration not documented.",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.0,
            BuiltinFont::Helvetica,
            1.3,
        );
        render_audio_sampling_summary(layout, external);
        return;
    };
    let start_x = LEFT_MM + 4.0;
    let width = RIGHT_MM - LEFT_MM - 8.0;
    let axis_y = layout.y_mm - 9.0;
    layout.rule(start_x, start_x + width, axis_y, 0.6);
    layout.text_at(start_x, axis_y + 5.0, "0:00", 5.8, BuiltinFont::Courier);
    let end_label = friendly_milliseconds(duration);
    layout.text_at(
        start_x + width - 16.0,
        axis_y + 5.0,
        &end_label,
        5.8,
        BuiltinFont::Courier,
    );
    for sample in &external.samples {
        let ratio = (sample.offset_milliseconds.min(duration) as f64 / duration as f64) as f32;
        let x = start_x + ratio * width;
        layout.vertical_rule(x, axis_y + 2.7, axis_y - 2.7, 1.1);
    }
    layout.y_mm = axis_y - 7.0;
    render_audio_sampling_summary(layout, external);
}

pub(super) fn render_audio_sampling_summary(
    layout: &mut PdfLayout,
    external: &AudioScreeningExternalRecord,
) {
    let status = audio_screening_status_label(external.status);
    let sample_summary = format!(
        "{} {} · {} {} · {} {}",
        external.unique_sample_count,
        layout.label("Unique samples"),
        external.duplicate_sample_count,
        layout.label("Duplicate samples"),
        external.overlapping_sample_count,
        layout.label("Overlapping samples"),
    );
    let sampled_duration = format!(
        "{} ms · {:.2} % {}",
        external.unique_sample_duration_milliseconds,
        external.track_coverage_percent,
        layout.label("track coverage"),
    );
    layout.table_rows(
        &[
            TableRow::plain("Sample summary", sample_summary),
            TableRow::plain("Sampled duration", sampled_duration),
            TableRow::system_plain("Overall result", status),
        ],
        44.0,
    );
}

pub(super) fn render_full_technical_intro(layout: &mut PdfLayout) {
    layout.mark_bookmark("Full Technical Certificate", layout.current_page_index());
    render_document_title(layout);
    layout.write_localized(
        "The following A–L sections retain the complete technical certificate detail for audit and verification.",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::Helvetica,
        1.3,
    );
    layout.y_mm -= 2.0;
}

pub(super) fn abbreviated_sha256(value: &str) -> String {
    if value.len() <= 18 {
        return value.to_owned();
    }
    format!("{}…{}", &value[..8], &value[value.len() - 8..])
}

pub(super) fn artwork_role_label(role: EvidenceRole) -> &'static str {
    match role {
        EvidenceRole::ArtworkSunoOriginal => "Suno original",
        EvidenceRole::AiArtworkOriginal => "AI original",
        EvidenceRole::AiArtworkEdited => "AI edited",
        EvidenceRole::HumanEditedArtwork => "Human edited",
        EvidenceRole::FinalArtwork => "Final",
        EvidenceRole::ReleaseArtwork => "Release artwork",
        _ => "Artwork",
    }
}

pub(super) fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub(super) fn timeline_x(
    value: NaiveDate,
    minimum: NaiveDate,
    maximum: NaiveDate,
    start_x: f32,
    width: f32,
) -> f32 {
    let total = (maximum - minimum).num_days().max(1) as f32;
    let offset = (value - minimum).num_days().max(0) as f32;
    start_x + (offset / total).clamp(0.0, 1.0) * width
}

pub(super) fn friendly_milliseconds(value: u64) -> String {
    let total_seconds = value / 1_000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02}")
}
