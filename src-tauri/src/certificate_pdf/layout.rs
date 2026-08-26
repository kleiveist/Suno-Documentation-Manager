use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BookmarkTarget {
    pub(super) title: String,
    pub(super) page_index: usize,
    pub(super) include_in_contents: bool,
}

pub(super) struct PdfLayout {
    pub(super) pages: Vec<Vec<Op>>,
    pub(super) y_mm: f32,
    pub(super) render_options: CertificateRenderOptions,
    pub(super) running_header_primary: String,
    pub(super) running_header_secondary: String,
    pub(super) bookmarks: Vec<BookmarkTarget>,
}

impl PdfLayout {
    pub(super) fn new(render_options: CertificateRenderOptions) -> Self {
        Self {
            pages: vec![Vec::new()],
            y_mm: FIRST_PAGE_TOP_MM,
            render_options,
            running_header_primary: String::new(),
            running_header_secondary: String::new(),
            bookmarks: Vec::new(),
        }
    }

    pub(super) fn set_running_header(&mut self, track: &str, artist: &str) {
        self.running_header_primary = format!("{} · {}", documented(track), documented(artist));
        self.running_header_secondary = self.label("Technical Evidence Certificate");
    }

    pub(super) fn current_page_index(&self) -> usize {
        self.pages.len().saturating_sub(1)
    }

    pub(super) fn mark_bookmark(&mut self, title: &str, page_index: usize) {
        let include_in_contents = matches!(
            title,
            "Summary"
                | "Evidence Overview"
                | "Full Technical Certificate"
                | "A. Certificate / Snapshot Identity"
                | "B. Track identity"
                | "C. Final Suno Generation"
                | "D. Source provenance"
                | "E. Human contribution"
                | "F. Suno Generation Text Field"
                | "G.1 AI Transparency Assessment – Audio"
                | "G.2 AI Transparency Assessment – Artwork"
                | "H. License and rights evidence"
                | "I. External Timestamp Evidence"
                | "J. Evidence register"
                | "K. Integrity anchors and workflow"
                | "K. Integrity anchors"
                | "K.1 Configured workflow checks"
                | "K.2 Pre-release audio screening"
                | "L. Technical certificate statement"
                | "Technical Appendix"
        );
        let title = self.label(title);
        if self
            .bookmarks
            .iter()
            .any(|bookmark| bookmark.title == title && bookmark.page_index == page_index)
        {
            return;
        }
        self.bookmarks.push(BookmarkTarget {
            title,
            page_index,
            include_in_contents,
        });
    }

    pub(super) fn label(&self, english: &str) -> String {
        localized_certificate_label(self.render_options, english)
    }

    pub(super) fn paragraph(&self, english: &str) -> String {
        localized_certificate_paragraph(self.render_options, english)
    }

    pub(super) fn write_localized(
        &mut self,
        english: &str,
        x_mm: f32,
        width_mm: f32,
        size_pt: f32,
        font: BuiltinFont,
        line_factor: f32,
    ) {
        let text = self.paragraph(english);
        self.write_wrapped(&text, x_mm, width_mm, size_pt, font, line_factor);
    }

    pub(super) fn current_ops(&mut self) -> &mut Vec<Op> {
        self.pages
            .last_mut()
            .expect("PDF layout always contains a page")
    }

    pub(super) fn fits(&self, height_mm: f32) -> bool {
        self.y_mm - height_mm >= CONTENT_BOTTOM_MM
    }

    pub(super) fn full_content_height(&self) -> f32 {
        CONTINUATION_TOP_MM - CONTENT_BOTTOM_MM
    }

    pub(super) fn ensure_space(&mut self, height_mm: f32) {
        if !self.fits(height_mm) {
            self.new_page();
        }
    }

    pub(super) fn new_page(&mut self) {
        self.pages.push(Vec::new());
        self.y_mm = CONTINUATION_TOP_MM;
        let primary = if self.running_header_primary.is_empty() {
            self.label(DOCUMENT_TITLE)
        } else {
            self.running_header_primary.clone()
        };
        let secondary = if self.running_header_secondary.is_empty() {
            self.label("Technical Evidence Certificate")
        } else {
            self.running_header_secondary.clone()
        };
        let primary_size = fit_text_size(
            &primary,
            RIGHT_MM - LEFT_MM,
            6.8,
            5.5,
            BuiltinFont::HelveticaBold,
        );
        let primary = ellipsize_text_to_width(
            &primary,
            RIGHT_MM - LEFT_MM,
            primary_size,
            BuiltinFont::HelveticaBold,
        );
        self.text_at(
            LEFT_MM,
            286.0,
            &primary,
            primary_size,
            BuiltinFont::HelveticaBold,
        );
        self.text_at(LEFT_MM, 281.4, &secondary, 6.2, BuiltinFont::Helvetica);
        self.rule(LEFT_MM, RIGHT_MM, 278.0, 0.35);
    }

    pub(super) fn continuation_title(&mut self, title: &str) {
        self.ensure_space(8.0);
        let title = self.label(title);
        self.text_at(
            LEFT_MM,
            self.y_mm - 3.2,
            &title,
            9.5,
            BuiltinFont::HelveticaBold,
        );
        self.y_mm -= 6.5;
        self.rule(LEFT_MM, RIGHT_MM, self.y_mm, 0.5);
        self.y_mm -= 1.5;
    }

    pub(super) fn section_title(&mut self, title: &str) {
        self.ensure_space(13.0);
        let page_index = self.current_page_index();
        self.mark_bookmark(title, page_index);
        self.y_mm -= 2.0;
        let title = self.label(title);
        self.text_at(
            LEFT_MM,
            self.y_mm - 3.5,
            &title,
            10.0,
            BuiltinFont::HelveticaBold,
        );
        self.y_mm -= 7.0;
        self.rule(LEFT_MM, RIGHT_MM, self.y_mm, 0.5);
        self.y_mm -= 1.5;
    }

    pub(super) fn write_wrapped(
        &mut self,
        text: &str,
        x_mm: f32,
        width_mm: f32,
        size_pt: f32,
        font: BuiltinFont,
        line_factor: f32,
    ) {
        let lines = wrap_text(text, width_mm, size_pt, font);
        let line_height_mm = size_pt * 0.352_778 * line_factor;
        for line in lines {
            self.ensure_space(line_height_mm);
            let baseline = self.y_mm - size_pt * 0.352_778 * 0.82;
            self.text_at(x_mm, baseline, &line, size_pt, font);
            self.y_mm -= line_height_mm;
        }
    }

    pub(super) fn table_rows(&mut self, rows: &[TableRow], label_width_mm: f32) {
        for row in rows {
            self.table_row(row, label_width_mm);
        }
    }

    pub(super) fn table_row(&mut self, row: &TableRow, label_width_mm: f32) {
        let label_x = LEFT_MM + 1.0;
        let value_x = LEFT_MM + label_width_mm + 2.0;
        let value_width = RIGHT_MM - value_x - 1.0;
        let label = self.label(&row.label);
        let label_lines = wrap_text(
            &label,
            label_width_mm - 2.0,
            6.7,
            BuiltinFont::HelveticaBold,
        );
        let value = if row.localize_value {
            localized_table_value(self.render_options, &row.value)
        } else {
            row.value.clone()
        };
        let value_lines = wrap_text(&value, value_width, row.value_size_pt, row.value_font);
        let line_count = label_lines.len().max(value_lines.len()).max(1);
        let line_height_mm = 3.35;
        let total_height = line_count as f32 * line_height_mm + 2.0;
        if total_height <= self.full_content_height() && !self.fits(total_height) {
            self.new_page();
        }

        self.y_mm -= 0.8;
        for line_index in 0..line_count {
            let continued = if !self.fits(line_height_mm + 0.8) {
                self.new_page();
                let continuation_label = self.label(&format!("{} (continuation)", row.label));
                self.text_at(
                    label_x,
                    self.y_mm - 2.45,
                    &continuation_label,
                    6.7,
                    BuiltinFont::HelveticaBold,
                );
                true
            } else {
                false
            };
            let baseline = self.y_mm - 2.45;
            if !continued {
                if let Some(line) = label_lines.get(line_index) {
                    self.text_at(label_x, baseline, line, 6.7, BuiltinFont::HelveticaBold);
                }
            }
            if let Some(line) = value_lines.get(line_index) {
                self.text_at(value_x, baseline, line, row.value_size_pt, row.value_font);
            }
            self.y_mm -= line_height_mm;
        }
        self.y_mm -= 0.4;
        self.rule(LEFT_MM, RIGHT_MM, self.y_mm, 0.2);
        self.y_mm -= 0.8;
    }

    pub(super) fn column_heading(&mut self, left: &str, right: &str, label_width_mm: f32) {
        self.ensure_space(6.0);
        let y = self.y_mm - 2.5;
        let left = self.label(left);
        let right = self.label(right);
        self.text_at(LEFT_MM + 1.0, y, &left, 6.5, BuiltinFont::HelveticaBold);
        self.text_at(
            LEFT_MM + label_width_mm + 2.0,
            y,
            &right,
            6.5,
            BuiltinFont::HelveticaBold,
        );
        self.y_mm -= 4.0;
        self.rule(LEFT_MM, RIGHT_MM, self.y_mm, 0.45);
        self.y_mm -= 1.0;
    }

    pub(super) fn evidence_summary_row(
        &mut self,
        index: usize,
        role: &str,
        provenance: &str,
        size_bytes: u64,
    ) {
        self.ensure_space(6.0);
        let y = self.y_mm - 2.5;
        self.text_at(
            LEFT_MM + 1.0,
            y,
            &index.to_string(),
            6.4,
            BuiltinFont::CourierBold,
        );
        self.text_at(LEFT_MM + 12.0, y, role, 6.4, BuiltinFont::Courier);
        self.text_at(LEFT_MM + 77.0, y, provenance, 6.4, BuiltinFont::Courier);
        self.text_at(
            LEFT_MM + 130.0,
            y,
            &size_bytes.to_string(),
            6.4,
            BuiltinFont::Courier,
        );
        self.y_mm -= 4.0;
        self.rule(LEFT_MM, RIGHT_MM, self.y_mm, 0.25);
        self.y_mm -= 0.7;
    }

    pub(super) fn image_at(
        &mut self,
        preview: &CertificateArtworkPreview,
        x_mm: f32,
        top_mm: f32,
        maximum_width_mm: f32,
        maximum_height_mm: f32,
    ) -> (f32, f32) {
        let source_ratio = preview.width_pixels as f32 / preview.height_pixels as f32;
        let (width_mm, height_mm) = if source_ratio >= maximum_width_mm / maximum_height_mm {
            (maximum_width_mm, maximum_width_mm / source_ratio)
        } else {
            (maximum_height_mm * source_ratio, maximum_height_mm)
        };
        let desired_width_pt = width_mm * 72.0 / 25.4;
        let desired_height_pt = height_mm * 72.0 / 25.4;
        self.current_ops().push(Op::UseXobject {
            id: preview.xobject_id(),
            transform: XObjectTransform {
                translate_x: Some(Pt(x_mm * 72.0 / 25.4)),
                translate_y: Some(Pt((top_mm - height_mm) * 72.0 / 25.4)),
                scale_x: Some(desired_width_pt),
                scale_y: Some(desired_height_pt),
                no_auto_scale: true,
                ..Default::default()
            },
        });
        self.box_outline(x_mm, top_mm, width_mm, height_mm, 0.35);
        (width_mm, height_mm)
    }

    pub(super) fn box_outline(
        &mut self,
        x_mm: f32,
        top_mm: f32,
        width_mm: f32,
        height_mm: f32,
        thickness_pt: f32,
    ) {
        self.rule(x_mm, x_mm + width_mm, top_mm, thickness_pt);
        self.rule(x_mm, x_mm + width_mm, top_mm - height_mm, thickness_pt);
        self.vertical_rule(x_mm, top_mm, top_mm - height_mm, thickness_pt);
        self.vertical_rule(x_mm + width_mm, top_mm, top_mm - height_mm, thickness_pt);
    }

    pub(super) fn vertical_rule(
        &mut self,
        x_mm: f32,
        from_y_mm: f32,
        to_y_mm: f32,
        thickness_pt: f32,
    ) {
        self.current_ops().extend([
            Op::SaveGraphicsState,
            Op::SetOutlineColor {
                col: Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 0.52,
                    icc_profile: None,
                }),
            },
            Op::SetOutlineThickness {
                pt: Pt(thickness_pt),
            },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point::new(Mm(x_mm), Mm(from_y_mm)),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(Mm(x_mm), Mm(to_y_mm)),
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
            Op::RestoreGraphicsState,
        ]);
    }

    pub(super) fn status_text_at(
        &mut self,
        x_mm: f32,
        y_mm: f32,
        text: &str,
        size_pt: f32,
        font: BuiltinFont,
        tone: StatusTone,
    ) {
        self.current_ops().push(Op::SaveGraphicsState);
        self.current_ops().push(Op::SetFillColor {
            col: status_color(tone),
        });
        self.text_at(x_mm, y_mm, text, size_pt, font);
        self.current_ops().push(Op::RestoreGraphicsState);
    }

    pub(super) fn status_icon_at(&mut self, x_mm: f32, y_mm: f32, tone: StatusTone) {
        let line = |points: &[(f32, f32)], is_closed: bool| Line {
            points: points
                .iter()
                .map(|(x, y)| LinePoint {
                    p: Point::new(Mm(*x), Mm(*y)),
                    bezier: false,
                })
                .collect(),
            is_closed,
        };
        let lines = match tone {
            StatusTone::Success => vec![line(
                &[
                    (x_mm, y_mm),
                    (x_mm + 1.3, y_mm - 1.4),
                    (x_mm + 4.0, y_mm + 1.6),
                ],
                false,
            )],
            StatusTone::Notice => vec![line(
                &[
                    (x_mm + 2.0, y_mm + 2.0),
                    (x_mm, y_mm - 1.9),
                    (x_mm + 4.0, y_mm - 1.9),
                ],
                true,
            )],
            StatusTone::Neutral => {
                vec![line(&[(x_mm, y_mm), (x_mm + 3.8, y_mm)], false)]
            }
            StatusTone::Error => vec![
                line(&[(x_mm, y_mm + 1.7), (x_mm + 3.7, y_mm - 1.8)], false),
                line(&[(x_mm, y_mm - 1.8), (x_mm + 3.7, y_mm + 1.7)], false),
            ],
        };
        self.current_ops().push(Op::SaveGraphicsState);
        self.current_ops().push(Op::SetOutlineColor {
            col: status_color(tone),
        });
        self.current_ops()
            .push(Op::SetOutlineThickness { pt: Pt(0.9) });
        for line in lines {
            self.current_ops().push(Op::DrawLine { line });
        }
        self.current_ops().push(Op::RestoreGraphicsState);
    }

    pub(super) fn render_contents_page(&mut self, page_index: usize) {
        let Some(ops) = self.pages.get_mut(page_index) else {
            return;
        };
        let title = localized_certificate_label(self.render_options, "Contents");
        append_text(
            ops,
            LEFT_MM,
            266.0,
            &title,
            14.0,
            BuiltinFont::HelveticaBold,
        );
        append_rule(ops, LEFT_MM, RIGHT_MM, 260.0, 0.6);
        let entries = self
            .bookmarks
            .iter()
            .filter(|bookmark| bookmark.page_index != page_index && bookmark.include_in_contents)
            .cloned()
            .collect::<Vec<_>>();
        let mut y_mm = 251.0;
        for bookmark in entries {
            append_text(
                ops,
                LEFT_MM + 2.0,
                y_mm,
                &bookmark.title,
                7.4,
                BuiltinFont::Helvetica,
            );
            append_text(
                ops,
                181.0,
                y_mm,
                &(bookmark.page_index + 1).to_string(),
                7.4,
                BuiltinFont::Courier,
            );
            y_mm -= 8.2;
        }
        append_text(
            ops,
            LEFT_MM,
            34.0,
            &localized_certificate_paragraph(
                self.render_options,
                "All sections are part of the same immutable technical certificate rendition.",
            ),
            7.0,
            BuiltinFont::Helvetica,
        );
    }

    pub(super) fn text_at(
        &mut self,
        x_mm: f32,
        y_mm: f32,
        text: &str,
        size_pt: f32,
        font: BuiltinFont,
    ) {
        self.current_ops().extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(x_mm), Mm(y_mm)),
            },
            Op::SetFont {
                font: PdfFontHandle::External(archive_font_id(font)),
                size: Pt(size_pt),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_owned())],
            },
            Op::EndTextSection,
        ]);
    }

    pub(super) fn rule(&mut self, from_x_mm: f32, to_x_mm: f32, y_mm: f32, thickness_pt: f32) {
        self.current_ops().extend([
            Op::SaveGraphicsState,
            Op::SetOutlineColor {
                col: Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 0.52,
                    icc_profile: None,
                }),
            },
            Op::SetOutlineThickness {
                pt: Pt(thickness_pt),
            },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point::new(Mm(from_x_mm), Mm(y_mm)),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(Mm(to_x_mm), Mm(y_mm)),
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
            Op::RestoreGraphicsState,
        ]);
    }

    pub(super) fn into_pages(
        mut self,
        certificate_id: &str,
    ) -> (Vec<PdfPage>, Vec<BookmarkTarget>) {
        let page_count = self.pages.len();
        let certificate_id_label = self.label("Certificate ID");
        let page_label = self.label("Page");
        for (index, ops) in self.pages.iter_mut().enumerate() {
            append_rule(ops, LEFT_MM, RIGHT_MM, FOOTER_RULE_Y_MM, 0.35);
            append_text(
                ops,
                LEFT_MM,
                FOOTER_TEXT_Y_MM,
                &format!("{certificate_id_label}: {certificate_id}"),
                5.8,
                BuiltinFont::Helvetica,
            );
            append_text(
                ops,
                166.0,
                FOOTER_TEXT_Y_MM,
                &format!("{page_label} {} / {page_count}", index + 1),
                6.5,
                BuiltinFont::Helvetica,
            );
        }
        let pages = self
            .pages
            .into_iter()
            .map(|ops| PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops))
            .collect();
        (pages, self.bookmarks)
    }
}

pub(super) fn append_text(
    ops: &mut Vec<Op>,
    x_mm: f32,
    y_mm: f32,
    text: &str,
    size_pt: f32,
    font: BuiltinFont,
) {
    ops.extend([
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(x_mm), Mm(y_mm)),
        },
        Op::SetFont {
            font: PdfFontHandle::External(archive_font_id(font)),
            size: Pt(size_pt),
        },
        Op::ShowText {
            items: vec![TextItem::Text(text.to_owned())],
        },
        Op::EndTextSection,
    ]);
}

pub(super) fn append_rule(
    ops: &mut Vec<Op>,
    from_x_mm: f32,
    to_x_mm: f32,
    y_mm: f32,
    thickness_pt: f32,
) {
    ops.extend([
        Op::SaveGraphicsState,
        Op::SetOutlineColor {
            col: Color::Cmyk(Cmyk {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 0.52,
                icc_profile: None,
            }),
        },
        Op::SetOutlineThickness {
            pt: Pt(thickness_pt),
        },
        Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point::new(Mm(from_x_mm), Mm(y_mm)),
                        bezier: false,
                    },
                    LinePoint {
                        p: Point::new(Mm(to_x_mm), Mm(y_mm)),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        },
        Op::RestoreGraphicsState,
    ]);
}

pub(super) fn fit_text_size(
    text: &str,
    width_mm: f32,
    preferred_size_pt: f32,
    minimum_size_pt: f32,
    font: BuiltinFont,
) -> f32 {
    let mut size = preferred_size_pt;
    while size > minimum_size_pt && measure_text_width_mm(text, size, font) > width_mm {
        size -= 0.2;
    }
    size.max(minimum_size_pt)
}

pub(super) fn ellipsize_text_to_width(
    text: &str,
    width_mm: f32,
    size_pt: f32,
    font: BuiltinFont,
) -> String {
    if measure_text_width_mm(text, size_pt, font) <= width_mm {
        return text.to_owned();
    }
    let ellipsis = "…";
    let mut output = String::new();
    for character in text.chars() {
        let candidate = format!("{output}{character}{ellipsis}");
        if measure_text_width_mm(&candidate, size_pt, font) > width_mm {
            break;
        }
        output.push(character);
    }
    output.push('…');
    output
}

pub(super) fn bounded_wrapped_lines(
    text: &str,
    width_mm: f32,
    size_pt: f32,
    font: BuiltinFont,
    maximum_lines: usize,
) -> Vec<String> {
    let mut lines = wrap_text(text, width_mm, size_pt, font);
    if lines.len() <= maximum_lines {
        return lines;
    }
    lines.truncate(maximum_lines.max(1));
    let last = lines
        .last_mut()
        .expect("bounded text has at least one line");
    *last = ellipsize_text_to_width(&format!("{}…", last.trim_end()), width_mm, size_pt, font);
    lines
}

pub(super) fn status_color(tone: StatusTone) -> Color {
    let (c, m, y, k) = match tone {
        StatusTone::Success => (0.72, 0.0, 0.58, 0.36),
        StatusTone::Notice => (0.0, 0.36, 0.92, 0.22),
        StatusTone::Neutral => (0.0, 0.0, 0.0, 0.68),
        StatusTone::Error => (0.0, 0.82, 0.62, 0.26),
    };
    Color::Cmyk(Cmyk {
        c,
        m,
        y,
        k,
        icc_profile: None,
    })
}

pub(super) fn estimated_row_height(
    text: &str,
    width_mm: f32,
    size_pt: f32,
    font: BuiltinFont,
) -> f32 {
    wrap_text(text, width_mm, size_pt, font).len() as f32 * 3.35 + 2.0
}

pub(super) fn wrap_text(text: &str, width_mm: f32, size_pt: f32, font: BuiltinFont) -> Vec<String> {
    let normalized = text
        .chars()
        .map(|character| match character {
            '\n' => '\n',
            '\t' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect::<String>();

    let mut result = Vec::new();
    for paragraph in normalized.split('\n') {
        let characters = paragraph.chars().collect::<Vec<_>>();
        if characters.is_empty() {
            result.push(String::new());
            continue;
        }
        let mut start = 0;
        while start < characters.len() {
            let mut used_width_mm = 0.0;
            let mut maximum_fit = start;
            let mut last_wrap_opportunity = None;
            for (offset, character) in characters[start..].iter().copied().enumerate() {
                let character_width_mm =
                    measure_text_width_mm(&character.to_string(), size_pt, font);
                if maximum_fit > start && used_width_mm + character_width_mm > width_mm {
                    break;
                }
                used_width_mm += character_width_mm;
                maximum_fit = start + offset + 1;
                if is_wrap_opportunity(character) {
                    last_wrap_opportunity = Some(maximum_fit);
                }
                if used_width_mm > width_mm {
                    break;
                }
            }
            if maximum_fit >= characters.len() && used_width_mm <= width_mm {
                result.push(characters[start..].iter().collect());
                break;
            }
            let minimum_clean_break = start + (maximum_fit.saturating_sub(start) / 2);
            let break_at = last_wrap_opportunity
                .filter(|index| *index >= minimum_clean_break)
                .unwrap_or(maximum_fit.max(start + 1));
            result.push(characters[start..break_at].iter().collect());
            start = break_at;
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

pub(super) fn measure_text_width_mm(text: &str, size_pt: f32, font: BuiltinFont) -> f32 {
    let metrics = builtin_font_metrics(font);
    let units_per_em = f32::from(metrics.units_per_em.max(1));
    let width_in_em = text
        .chars()
        .map(|character| {
            metrics
                .lookup_glyph_index(character as u32)
                .and_then(|glyph| metrics.get_glyph_width(glyph))
                .map(f32::from)
                .unwrap_or(units_per_em)
                / units_per_em
        })
        .sum::<f32>();
    width_in_em * size_pt * 0.352_778
}

pub(super) fn archive_font_id(font: BuiltinFont) -> FontId {
    FontId(
        match font {
            BuiltinFont::Helvetica | BuiltinFont::HelveticaOblique => ARCHIVE_SANS_REGULAR_ID,
            BuiltinFont::HelveticaBold | BuiltinFont::HelveticaBoldOblique => ARCHIVE_SANS_BOLD_ID,
            BuiltinFont::Courier | BuiltinFont::CourierOblique => ARCHIVE_MONO_REGULAR_ID,
            BuiltinFont::CourierBold | BuiltinFont::CourierBoldOblique => ARCHIVE_MONO_BOLD_ID,
            _ => ARCHIVE_SANS_REGULAR_ID,
        }
        .to_owned(),
    )
}

pub(super) fn install_archive_fonts(document: &mut PdfDocument) -> Result<()> {
    for (id, bytes, expected_sha256) in [
        (
            ARCHIVE_SANS_REGULAR_ID,
            ARCHIVE_SANS_REGULAR_BYTES,
            CERTIFICATE_PDF_FONT_SHA256[0],
        ),
        (
            ARCHIVE_SANS_BOLD_ID,
            ARCHIVE_SANS_BOLD_BYTES,
            CERTIFICATE_PDF_FONT_SHA256[1],
        ),
        (
            ARCHIVE_MONO_REGULAR_ID,
            ARCHIVE_MONO_REGULAR_BYTES,
            CERTIFICATE_PDF_FONT_SHA256[2],
        ),
        (
            ARCHIVE_MONO_BOLD_ID,
            ARCHIVE_MONO_BOLD_BYTES,
            CERTIFICATE_PDF_FONT_SHA256[3],
        ),
    ] {
        if crate::security::sha256_bytes(bytes) != expected_sha256 {
            return Err(AppError::Data(format!(
                "Bundled archive font {id} does not match its recorded SHA-256."
            )));
        }
        let parsed = parse_archive_font(bytes, id)?;
        if !parsed.has_source_bytes() {
            return Err(AppError::Data(format!(
                "Bundled archive font {id} has no embeddable source bytes."
            )));
        }
        document
            .resources
            .fonts
            .map
            .insert(FontId(id.to_owned()), PdfFont::new(parsed));
    }
    Ok(())
}

pub(super) fn parse_archive_font(bytes: &[u8], id: &str) -> Result<ParsedFont> {
    let mut warnings = Vec::new();
    ParsedFont::from_bytes(bytes, 0, &mut warnings).ok_or_else(|| {
        AppError::Data(format!(
            "Bundled archive font {id} could not be parsed for PDF/A embedding."
        ))
    })
}

pub(super) fn builtin_font_metrics(font: BuiltinFont) -> &'static ParsedFont {
    static SANS: OnceLock<ParsedFont> = OnceLock::new();
    static SANS_BOLD: OnceLock<ParsedFont> = OnceLock::new();
    static MONO: OnceLock<ParsedFont> = OnceLock::new();
    static MONO_BOLD: OnceLock<ParsedFont> = OnceLock::new();

    match font {
        BuiltinFont::Helvetica | BuiltinFont::HelveticaOblique => SANS.get_or_init(|| {
            parse_archive_font(ARCHIVE_SANS_REGULAR_BYTES, ARCHIVE_SANS_REGULAR_ID)
                .expect("bundled DejaVu Sans metrics")
        }),
        BuiltinFont::HelveticaBold | BuiltinFont::HelveticaBoldOblique => {
            SANS_BOLD.get_or_init(|| {
                parse_archive_font(ARCHIVE_SANS_BOLD_BYTES, ARCHIVE_SANS_BOLD_ID)
                    .expect("bundled DejaVu Sans Bold metrics")
            })
        }
        BuiltinFont::Courier | BuiltinFont::CourierOblique => MONO.get_or_init(|| {
            parse_archive_font(ARCHIVE_MONO_REGULAR_BYTES, ARCHIVE_MONO_REGULAR_ID)
                .expect("bundled DejaVu Sans Mono metrics")
        }),
        BuiltinFont::CourierBold | BuiltinFont::CourierBoldOblique => MONO_BOLD.get_or_init(|| {
            parse_archive_font(ARCHIVE_MONO_BOLD_BYTES, ARCHIVE_MONO_BOLD_ID)
                .expect("bundled DejaVu Sans Mono Bold metrics")
        }),
        _ => SANS.get_or_init(|| {
            parse_archive_font(ARCHIVE_SANS_REGULAR_BYTES, ARCHIVE_SANS_REGULAR_ID)
                .expect("bundled DejaVu Sans metrics")
        }),
    }
}

pub(super) fn is_wrap_opportunity(character: char) -> bool {
    // A hyphen is deliberately not a preferred break. This keeps technical labels such as
    // `SHA-256` and ordinary UUIDs intact whenever the complete token fits the available width.
    character.is_whitespace() || matches!(character, '/' | '\\' | '_' | '.' | ':' | ';' | ',')
}
