use super::*;

// These sentinels exist only while the Markdown template is localized. Input
// validation rejects them, and they are removed before either language is
// published. They let the localizer distinguish historical values from
// certificate-owned text even when a multiline value begins with Markdown
// syntax or is literally named like a system status.
pub(super) const MARKDOWN_VALUE_START: char = '\u{1e}';
pub(super) const MARKDOWN_VALUE_END: char = '\u{1f}';

/// Return a certificate label in the configured output language. The
/// compatibility bilingual mode is used only for the Markdown presentation;
/// PDF generation passes one language per file.
pub(crate) fn localized_certificate_label(
    options: CertificateRenderOptions,
    english: &str,
) -> String {
    let german = german_certificate_label(english);
    localized_certificate_variant(options, &german, english, " / ")
}

/// Return a prose paragraph in the configured output language. Newlines keep
/// compatibility bilingual Markdown versions visually separate.
pub(crate) fn localized_certificate_paragraph(
    options: CertificateRenderOptions,
    english: &str,
) -> String {
    let german = german_certificate_paragraph(english);
    localized_certificate_variant(options, &german, english, "\n")
}

/// Translate only certificate-owned Markdown labels and prose. Values supplied
/// by the user or captured from evidence retain their original characters and
/// language; multiline continuations are indented so their Markdown syntax
/// cannot become certificate structure.
pub(super) fn localized_markdown_certificate(
    english_certificate: &str,
    options: CertificateRenderOptions,
) -> String {
    let clean_english = materialize_markdown_values(english_certificate);
    if !matches!(options.language, CertificateLanguage::De) && !options.bilingual {
        return clean_english;
    }

    let mut fence_length = None;
    let mut protected_value = false;
    let german = english_certificate
        .split('\n')
        .map(|line| {
            // Protected historical content must never open/close a template
            // code fence. Scan the transient origin markers before looking at
            // Markdown syntax, including when a protected line is literally
            // made only of backticks.
            let protected_at_line_start = protected_value || line.starts_with(MARKDOWN_VALUE_START);
            let contains_value_start = line.contains(MARKDOWN_VALUE_START);
            for character in line.chars() {
                if character == MARKDOWN_VALUE_START {
                    protected_value = true;
                } else if character == MARKDOWN_VALUE_END {
                    protected_value = false;
                }
            }
            if protected_at_line_start {
                return line.to_owned();
            }
            if contains_value_start {
                // A normal certificate label may precede a protected first
                // value line. Translate that label while the sentinel keeps
                // the field value outside the system-value allowlist.
                return german_markdown_line(line);
            }

            let trimmed = line.trim();
            if let Some(required) = fence_length {
                if trimmed.len() >= required && trimmed.chars().all(|character| character == '`') {
                    fence_length = None;
                }
                return line.to_owned();
            }
            let opening_length = trimmed
                .chars()
                .take_while(|character| *character == '`')
                .count();
            if opening_length >= 3 {
                fence_length = Some(opening_length);
                return line.to_owned();
            }
            german_markdown_line(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let german = materialize_markdown_values(&german);
    if !options.bilingual {
        return german;
    }

    match options.language {
        CertificateLanguage::De => {
            format!("{german}\n\n---\n\n# English certificate\n\n{clean_english}")
        }
        CertificateLanguage::En => {
            format!("{clean_english}\n\n---\n\n# Deutsche Fassung\n\n{german}")
        }
    }
}

pub(super) fn materialize_markdown_values(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut protected = false;
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            MARKDOWN_VALUE_START => protected = true,
            MARKDOWN_VALUE_END => protected = false,
            '\r' if protected && characters.peek() == Some(&'\n') => {
                output.push('\r');
                output.push('\n');
                characters.next();
                output.push_str("    ");
            }
            '\r' | '\n' if protected => {
                output.push(character);
                output.push_str("    ");
            }
            _ => output.push(character),
        }
    }
    output
}

pub(super) fn german_markdown_line(line: &str) -> String {
    for prefix in ["#### ", "### ", "## ", "# ", "> "] {
        if let Some(value) = line.strip_prefix(prefix) {
            return format!("{prefix}{}", german_certificate_paragraph(value));
        }
    }
    if let Some(value) = line.strip_prefix("- ") {
        if let Some((label, field_value)) = value.split_once(": ") {
            return format!(
                "- {}: {}",
                german_certificate_label(label),
                german_markdown_system_value(field_value)
            );
        }
        if is_certificate_owned_markdown_prose(value) {
            return format!("- {}", german_certificate_paragraph(value));
        }
        return line.to_owned();
    }
    if is_certificate_owned_markdown_prose(line) {
        return german_certificate_paragraph(line);
    }
    // Unprefixed lines can be multiline continuations of user-entered prompts,
    // notes, or evidence metadata. Preserve them byte-for-byte instead of
    // applying template substring replacements to historical values.
    german_markdown_system_value(line)
}

pub(super) fn german_markdown_system_value(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut segment = String::new();
    let mut protected = false;
    for character in value.chars() {
        match character {
            MARKDOWN_VALUE_START => {
                if !segment.is_empty() {
                    output.push_str(&german_certificate_label(&segment));
                    segment.clear();
                }
                output.push(character);
                protected = true;
            }
            MARKDOWN_VALUE_END => {
                output.push_str(&segment);
                segment.clear();
                output.push(character);
                protected = false;
            }
            _ => segment.push(character),
        }
    }
    if protected {
        output.push_str(&segment);
    } else {
        output.push_str(&german_certificate_label(&segment));
    }
    output
}

pub(super) fn is_certificate_owned_markdown_prose(line: &str) -> bool {
    matches!(
        line,
        "This retained historical data is unclassified legacy data and is not a Vocal Lyrics claim."
            | "No AI Act compliance, legal necessity, or legal safety determination is made."
            | "No AI Act compliance, legal necessity, legal safety, or other legal determination is made."
            | "No archived terms evidence recorded."
            | "No external timestamp evidence recorded."
            | "Factual archive and coverage status only. No rights ownership, license validity, legality, or non-infringement conclusion is made."
            | "This is a factual coverage and archive status only; it is not a rights determination."
            | "Post-finalization timestamp evidence, if later attached, is recorded in a separate addendum and does not change this technical-finalization snapshot."
            | "For long-term evidentiary preservation, an external timestamp can be added after technical finalization."
            | "Technical timestamp verification and provider qualification answer different questions. No legal effect is inferred. A regulatory qualification is reported only when independently verified."
            | "For long-term evidentiary preservation, an external timestamp can be requested in a later immutable addendum."
            | "Audio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion."
            | "This certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks."
            | "It does **not** confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification."
            | "Origin labels used: **User-confirmed fact**, **Evidence-derived metadata**, **System verification**, and **System value**."
    ) || line == crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE
}

pub(super) fn localized_certificate_variant(
    options: CertificateRenderOptions,
    german: &str,
    english: &str,
    separator: &str,
) -> String {
    if options.bilingual && german != english {
        return match options.language {
            CertificateLanguage::De => format!("{german}{separator}{english}"),
            CertificateLanguage::En => format!("{english}{separator}{german}"),
        };
    }
    match options.language {
        CertificateLanguage::De => german.to_owned(),
        CertificateLanguage::En => english.to_owned(),
    }
}

pub(super) fn german_certificate_paragraph(english: &str) -> String {
    german_certificate_paragraph_a(english)
        .or_else(|| german_certificate_paragraph_b(english))
        .or_else(|| german_certificate_paragraph_c(english))
        .map(str::to_owned)
        .unwrap_or_else(|| german_certificate_label(english))
}

fn german_certificate_paragraph_a(english: &str) -> Option<&'static str> {
    match english {
        "Technical documentation only — not a legal or governmental certification." => Some({
            "Ausschließlich technische Dokumentation — keine rechtliche oder behördliche Zertifizierung."
        }),
        "Finalized technical snapshot – not a legal certification" => Some({
            "Finalisierter technischer Snapshot – keine rechtliche Zertifizierung"
        }),
        "Finalized technical documentation, evidence, and integrity snapshot" => Some({
            "Finalisierter technischer Snapshot für Dokumentation, Evidence und Integrität"
        }),
        "Technical evidence report — not a legal or governmental certification" => Some({
            "Technischer Evidenzbericht — keine rechtliche oder behördliche Zertifizierung"
        }),
        "Configured documentation requirements completed" => Some({
            "Konfigurierte Dokumentationsanforderungen abgeschlossen"
        }),
        "Status values describe documentation and technical checks only; they are not a rights clearance or legal approval." => Some({
            "Statuswerte beschreiben ausschließlich Dokumentation und technische Prüfungen; sie stellen keine Rechtefreigabe oder rechtliche Genehmigung dar."
        }),
        "All sections are part of the same immutable technical certificate rendition." => Some({
            "Alle Abschnitte sind Bestandteil derselben unveränderlichen technischen Zertifikatsfassung."
        }),
        "Timeline not available — no documented dates" => Some({
            "Timeline nicht verfügbar — keine dokumentierten Datumswerte"
        }),
        "Artwork preview not available" => Some("Artwork-Vorschau nicht verfügbar"),
        "Artwork images are visual previews only. Evidence ID, registered path, and full SHA-256 remain authoritative in the Evidence Register." => Some({
            "Artwork-Bilder sind ausschließlich visuelle Vorschauen. Evidence-ID, registrierter Pfad und vollständiger SHA-256 bleiben im Evidenzregister maßgeblich."
        }),
        "External screening: NOT RUN" => Some("Externes Screening: NICHT AUSGEFÜHRT"),
        "Proportional visualization unavailable — source duration not documented." => Some({
            "Proportionale Visualisierung nicht verfügbar — Quelldauer nicht dokumentiert."
        }),
        "The following A–L sections retain the complete technical certificate detail for audit and verification." => Some({
            "Die folgenden Abschnitte A–L enthalten weiterhin sämtliche technischen Zertifikatsdetails für Audit und Verifikation."
        }),
        _ => None,
    }
}

fn german_certificate_paragraph_b(english: &str) -> Option<&'static str> {
    match english {
        "Full millisecond-bound ACRCloud sample records are retained in the Technical Appendix." => Some({
            "Vollständige, millisekundengenau gebundene ACRCloud-Probendatensätze sind im technischen Anhang enthalten."
        }),
        "This appendix retains long technical records that are summarized in the preceding certificate sections." => Some({
            "Dieser Anhang enthält lange technische Datensätze, die in den vorangehenden Zertifikatsabschnitten zusammengefasst sind."
        }),
        "No separate appendix records were required for this snapshot." => Some({
            "Für diesen Snapshot waren keine separaten Anhangsdatensätze erforderlich."
        }),
        "This is a factual coverage and archive status only; it is not a rights determination." => Some({
            "Dies ist ausschließlich ein sachlicher Abdeckungs- und Archivstatus; er stellt keine Rechtefeststellung dar."
        }),
        "Post-finalization timestamp evidence, if later attached, is recorded in a separate addendum and does not change this technical-finalization snapshot." => Some({
            "Nach der Finalisierung angehängte Zeitstempelnachweise werden in einem separaten Nachtrag dokumentiert und verändern diesen technischen Finalisierungssnapshot nicht."
        }),
        "For long-term evidentiary preservation, an external timestamp can be added after technical finalization." => Some({
            "Zur langfristigen Beweissicherung kann nach der technischen Finalisierung ein externer Zeitstempel angehängt werden."
        }),
        "No archived terms evidence recorded." => Some({
            "Keine archivierte Evidence zu Nutzungsbedingungen dokumentiert."
        }),
        "No external timestamp evidence recorded." => Some({
            "Kein externer Zeitstempelnachweis dokumentiert."
        }),
        "Audio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion." => Some({
            "Audio-Screening-Ergebnisse sind ausschließlich technische Vergleichsdatensätze. Sie begründen keine Aussage zu Urheberschaft, Rechteinhaberschaft, Erlaubnis, Verletzung, Rechtmäßigkeit, Release-Freigabe oder einer sonstigen rechtlichen Schlussfolgerung."
        }),
        "This certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks." => Some({
            "Dieses Zertifikat bestätigt die erfassten Eingaben, den finalisierten Snapshot, registrierte Evidence, dokumentierte Herkunft, SHA-256-Werte und konfigurierte Workflow-Prüfungen."
        }),
        "It does **not** confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification." => Some({
            "Es bestätigt **nicht** Urheberschaft, Rechteinhaberschaft, Nichtverletzung, Rechtmäßigkeit, Lizenzgültigkeit, gerichtlichen Beweiswert, gesetzliche Konformität oder eine behördliche Zertifizierung."
        }),
        "Origin labels used: **User-confirmed fact**, **Evidence-derived metadata**, **System verification**, and **System value**." => Some({
            "Verwendete Herkunftskennzeichnungen: **Vom Nutzer bestätigte Angabe**, **Aus Evidenzmetadaten**, **Systemprüfung** und **Systemwert**."
        }),
        "No AI Act compliance, legal necessity, or legal safety determination is made." => Some({
            "Es wird keine Feststellung zur AI-Act-Konformität, rechtlichen Erforderlichkeit oder rechtlichen Sicherheit getroffen."
        }),
        _ => None,
    }
}

fn german_certificate_paragraph_c(english: &str) -> Option<&'static str> {
    match english {
        "No AI Act compliance, legal necessity, legal safety, or other legal determination is made." => Some({
            "Es wird keine Feststellung zur AI-Act-Konformität, rechtlichen Erforderlichkeit, rechtlichen Sicherheit oder einer sonstigen rechtlichen Bewertung getroffen."
        }),
        "Documented input – this field records the submitted prompt and does not describe or verify the resulting audio content." => Some({
            "Dokumentierte Eingabe – dieses Feld dokumentiert den eingegebenen Prompt und stellt keine Feststellung über den tatsächlichen finalen Audioinhalt dar."
        }),
        "External timestamp evidence at technical finalization: NOT RECORDED" => Some({
            "Externer Zeitstempelnachweis bei technischer Finalisierung: NICHT ERFASST"
        }),
        "Origin labels: User-confirmed fact / Evidence-derived metadata / System verification / System value." => Some({
            "Herkunftskennzeichnungen: Vom Nutzer bestätigte Angabe / Aus Evidenzmetadaten / Systemprüfung / Systemwert."
        }),
        "This technical certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks. It does not confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification." => Some({
            "Dieses technische Zertifikat bestätigt die erfassten Eingaben, den finalisierten Snapshot, registrierte Evidence, dokumentierte Herkunft, SHA-256-Werte und konfigurierte Workflow-Prüfungen. Es bestätigt weder Urheberschaft noch Rechteinhaberschaft, Nichtverletzung, Rechtmäßigkeit, Lizenzgültigkeit, gerichtlichen Beweiswert, gesetzliche Konformität oder eine behördliche Zertifizierung."
        }),
        "This retained historical data is unclassified legacy data and is not a Vocal Lyrics claim." => Some({
            "Diese erhaltenen historischen Daten sind nicht klassifizierte Legacy-Daten und keine Aussage zu Vocal Lyrics."
        }),
        "Factual archive and coverage status only. No rights ownership, license validity, legality, or non-infringement conclusion is made." => Some({
            "Ausschließlich sachlicher Archiv- und Abdeckungsstatus. Es wird keine Feststellung zu Rechteinhaberschaft, Lizenzgültigkeit, Rechtmäßigkeit oder Nichtverletzung getroffen."
        }),
        "The application records technical timestamp evidence separately from provider qualification. It does not infer legal effect; a regulatory qualification is reported only when independently verified." => Some({
            "Die Anwendung dokumentiert technische Zeitstempel-Evidence getrennt von der Providerqualifikation. Sie leitet daraus keine Rechtswirkung ab; eine regulatorische Qualifikation wird nur ausgewiesen, wenn sie unabhängig verifiziert wurde."
        }),
        "Technical timestamp verification and provider qualification answer different questions. No legal effect is inferred. A regulatory qualification is reported only when independently verified." => Some({
            "Die technische Zeitstempelverifikation und die Providerqualifikation beantworten unterschiedliche Fragen. Es wird keine Rechtswirkung abgeleitet. Eine regulatorische Qualifikation wird nur ausgewiesen, wenn sie unabhängig verifiziert wurde."
        }),
        "For long-term evidentiary preservation, an external timestamp can be requested in a later immutable addendum." => Some({
            "Zur langfristigen Beweissicherung kann ein externer Zeitstempel in einem späteren unveränderlichen Nachtrag angefordert werden."
        }),
        "Post-finalization technical evidence record – no legal qualification asserted" => Some({
            "Technischer Evidenzdatensatz nach der Finalisierung – keine rechtliche Qualifizierung behauptet"
        }),
        "The application records the external timestamp evidence and its referenced hash. It does not determine any legal qualification of the timestamp." => Some({
            "Die Anwendung dokumentiert den externen Zeitstempelnachweis und seinen referenzierten Hash. Sie trifft keine Aussage über eine rechtliche Qualifizierung des Zeitstempels."
        }),
        crate::documents::ARTWORK_IMPORT_TIMESTAMP_NOTICE => Some({
            "Import-Zeitstempel dokumentieren nur den Import in SunoDM und nicht die tatsächliche Erstellungs- oder Bearbeitungsreihenfolge der Artwork-Dateien."
        }),
        _ => None,
    }
}

pub(super) fn german_certificate_label(english: &str) -> String {
    let mut translated = english.to_owned();
    let mut replacements = LABEL_REPLACEMENTS_A
        .iter()
        .chain(LABEL_REPLACEMENTS_B)
        .chain(LABEL_REPLACEMENTS_C)
        .copied()
        .collect::<Vec<_>>();
    // Translate the most specific phrase first. This prevents a short summary
    // label such as "Final generation" or "DOCUMENTED" from consuming the
    // prefix of a longer technical label/status before it can be localized.
    replacements.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    for (source, target) in replacements {
        translated = translated.replace(source, target);
    }
    translated
}
