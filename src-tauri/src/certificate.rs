use crate::certificate_pdf::{self, CertificatePdfSnapshot};
use crate::error::{AppError, Result};
use crate::integrity::HASH_FILE;
use crate::model::{
    AudioScreeningState, AudioScreeningStatus, BlockingDeviation, CertificateLanguage,
    CertificateRenderOptions, DocumentationAnswer, EvidenceItem, EvidenceMetadata,
    EvidenceProvenance, EvidenceRole, FactOrigin, Profile, StepState, StepStatus,
    SunoLyricsContentSource, SunoLyricsContentType, TrackFields, TrackRecord,
};
use crate::security::{
    atomic_write_new, contained_path, copy_new, ensure_contained_directory, portable_relative,
    sha256_bytes, sha256_file,
};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const CERTIFICATE_DIR: &str = "06_CERTIFICATE";
pub const CERTIFICATE_FILE: &str = "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md";
pub const MANIFEST_FILE: &str = "06_CERTIFICATE/EVIDENCE_MANIFEST.json";
pub const CERTIFICATE_HASH_FILE: &str = "06_CERTIFICATE/CERTIFICATE_SHA256.txt";
/// Stable English PDF filename. Older certificate sets use this filename for
/// their single language-selected PDF; new sets keep it as the English PDF.
pub const PDF_FILE: &str = "SunoDM_DOCUMENTATION_CERTIFICATE.pdf";
pub const PDF_FILE_EN: &str = PDF_FILE;
pub const PDF_FILE_DE: &str = "SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf";
pub const CERTIFICATE_FORMAT_VERSION: &str = "5.2";

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
/// by the user or captured from evidence remain byte-for-byte unchanged.
fn localized_markdown_certificate(
    english_certificate: &str,
    options: CertificateRenderOptions,
) -> String {
    if !matches!(options.language, CertificateLanguage::De) && !options.bilingual {
        return english_certificate.to_owned();
    }

    let german = english_certificate
        .lines()
        .map(german_markdown_line)
        .collect::<Vec<_>>()
        .join("\n");
    if !options.bilingual {
        return german;
    }

    match options.language {
        CertificateLanguage::De => {
            format!("{german}\n\n---\n\n# English certificate\n\n{english_certificate}")
        }
        CertificateLanguage::En => {
            format!("{english_certificate}\n\n---\n\n# Deutsche Fassung\n\n{german}")
        }
    }
}

fn german_markdown_line(line: &str) -> String {
    for prefix in ["#### ", "### ", "## ", "# ", "> "] {
        if let Some(value) = line.strip_prefix(prefix) {
            return format!("{prefix}{}", german_certificate_paragraph(value));
        }
    }
    if let Some(value) = line.strip_prefix("- ") {
        if let Some((label, field_value)) = value.split_once(": ") {
            return format!("- {}: {field_value}", german_certificate_label(label));
        }
        return format!("- {}", german_certificate_label(value));
    }
    german_certificate_paragraph(line)
}

fn localized_certificate_variant(
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

fn german_certificate_paragraph(english: &str) -> String {
    let translated = match english {
        "Technical documentation only — not a legal or governmental certification." => {
            "Ausschließlich technische Dokumentation — keine rechtliche oder behördliche Zertifizierung."
        }
        "Finalized technical snapshot – not a legal certification" => {
            "Finalisierter technischer Snapshot – keine rechtliche Zertifizierung"
        }
        "Finalized technical documentation, evidence, and integrity snapshot" => {
            "Finalisierter technischer Snapshot für Dokumentation, Evidence und Integrität"
        }
        "This is a factual coverage and archive status only; it is not a rights determination." => {
            "Dies ist ausschließlich ein sachlicher Abdeckungs- und Archivstatus; er stellt keine Rechtefeststellung dar."
        }
        "Post-finalization timestamp evidence, if later attached, is recorded in a separate addendum and does not change this technical-finalization snapshot." => {
            "Nach der Finalisierung angehängte Zeitstempelnachweise werden in einem separaten Nachtrag dokumentiert und verändern diesen technischen Finalisierungssnapshot nicht."
        }
        "For long-term evidentiary preservation, an external timestamp can be added after technical finalization." => {
            "Zur langfristigen Beweissicherung kann nach der technischen Finalisierung ein externer Zeitstempel angehängt werden."
        }
        "No archived terms evidence recorded." => {
            "Keine archivierte Evidence zu Nutzungsbedingungen dokumentiert."
        }
        "No external timestamp evidence recorded." => {
            "Kein externer Zeitstempelnachweis dokumentiert."
        }
        "Audio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion." => {
            "Audio-Screening-Ergebnisse sind ausschließlich technische Vergleichsdatensätze. Sie begründen keine Aussage zu Urheberschaft, Rechteinhaberschaft, Erlaubnis, Verletzung, Rechtmäßigkeit, Release-Freigabe oder einer sonstigen rechtlichen Schlussfolgerung."
        }
        "This certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks." => {
            "Dieses Zertifikat bestätigt die erfassten Eingaben, den finalisierten Snapshot, registrierte Evidence, dokumentierte Herkunft, SHA-256-Werte und konfigurierte Workflow-Prüfungen."
        }
        "It does **not** confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification." => {
            "Es bestätigt **nicht** Urheberschaft, Rechteinhaberschaft, Nichtverletzung, Rechtmäßigkeit, Lizenzgültigkeit, gerichtlichen Beweiswert, gesetzliche Konformität oder eine behördliche Zertifizierung."
        }
        "Origin labels used: **User-confirmed fact**, **Evidence-derived metadata**, **System verification**, and **System value**." => {
            "Verwendete Herkunftskennzeichnungen: **Vom Nutzer bestätigte Angabe**, **Aus Evidenzmetadaten**, **Systemprüfung** und **Systemwert**."
        }
        "No AI Act compliance, legal necessity, or legal safety determination is made." => {
            "Es wird keine Feststellung zur AI-Act-Konformität, rechtlichen Erforderlichkeit oder rechtlichen Sicherheit getroffen."
        }
        "No AI Act compliance, legal necessity, legal safety, or other legal determination is made." => {
            "Es wird keine Feststellung zur AI-Act-Konformität, rechtlichen Erforderlichkeit, rechtlichen Sicherheit oder einer sonstigen rechtlichen Bewertung getroffen."
        }
        "External timestamp evidence at technical finalization: NOT RECORDED" => {
            "Externer Zeitstempelnachweis bei technischer Finalisierung: NICHT ERFASST"
        }
        "Origin labels: User-confirmed fact / Evidence-derived metadata / System verification / System value." => {
            "Herkunftskennzeichnungen: Vom Nutzer bestätigte Angabe / Aus Evidenzmetadaten / Systemprüfung / Systemwert."
        }
        "This technical certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks. It does not confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification." => {
            "Dieses technische Zertifikat bestätigt die erfassten Eingaben, den finalisierten Snapshot, registrierte Evidence, dokumentierte Herkunft, SHA-256-Werte und konfigurierte Workflow-Prüfungen. Es bestätigt weder Urheberschaft noch Rechteinhaberschaft, Nichtverletzung, Rechtmäßigkeit, Lizenzgültigkeit, gerichtlichen Beweiswert, gesetzliche Konformität oder eine behördliche Zertifizierung."
        }
        "This retained historical data is unclassified legacy data and is not a Vocal Lyrics claim." => {
            "Diese erhaltenen historischen Daten sind nicht klassifizierte Legacy-Daten und keine Aussage zu Vocal Lyrics."
        }
        "Factual archive and coverage status only. No rights ownership, license validity, legality, or non-infringement conclusion is made." => {
            "Ausschließlich sachlicher Archiv- und Abdeckungsstatus. Es wird keine Feststellung zu Rechteinhaberschaft, Lizenzgültigkeit, Rechtmäßigkeit oder Nichtverletzung getroffen."
        }
        "Post-finalization technical evidence record – no legal qualification asserted" => {
            "Technischer Evidenzdatensatz nach der Finalisierung – keine rechtliche Qualifizierung behauptet"
        }
        "The application records the external timestamp evidence and its referenced hash. It does not determine any legal qualification of the timestamp." => {
            "Die Anwendung dokumentiert den externen Zeitstempelnachweis und seinen referenzierten Hash. Sie trifft keine Aussage über eine rechtliche Qualifizierung des Zeitstempels."
        }
        _ => return german_certificate_label(english),
    };
    translated.to_owned()
}

fn german_certificate_label(english: &str) -> String {
    let mut translated = english.to_owned();
    for (source, target) in [
        (
            "SunoDM – Technical Documentation and Evidence Certificate",
            "SunoDM – Technisches Dokumentations- und Evidenzzertifikat",
        ),
        (
            "SunoDM Technical Documentation and Evidence Certificate",
            "SunoDM Technisches Dokumentations- und Evidenzzertifikat",
        ),
        (
            "A. Certificate / Snapshot Identity",
            "A. Zertifikats- / Snapshot-Identität",
        ),
        ("B. Track identity", "B. Track-Identität"),
        ("C. Final Suno Generation", "C. Finale Suno-Erzeugung"),
        ("D. Source provenance", "D. Herkunft der Quellen"),
        ("E. Human contribution", "E. Menschlicher Beitrag"),
        (
            "F. Suno Generation Text Field",
            "F. Suno Generierungs-Textfeld",
        ),
        (
            "F.1 Unclassified legacy lyrics data",
            "F.1 Nicht klassifizierte historische Lyrics-Daten",
        ),
        (
            "G.1 AI Transparency Assessment – Audio",
            "G.1 KI-Transparenzbewertung – Audio",
        ),
        (
            "G.2 AI Transparency Assessment – Artwork",
            "G.2 KI-Transparenzbewertung – Artwork",
        ),
        (
            "G. AI Transparency Assessment",
            "G. KI-Transparenzbewertung",
        ),
        (
            "H. License and rights evidence",
            "H. Lizenz- und Rechte-Evidence",
        ),
        (
            "I. External Timestamp Evidence",
            "I. Externe Zeitstempel-Evidence",
        ),
        ("J. Evidence register", "J. Evidenzregister"),
        (
            "K. Integrity anchors and workflow",
            "K. Integritätsanker und Workflow",
        ),
        ("K. Integrity anchors", "K. Integritätsanker"),
        (
            "K.1 Configured workflow checks",
            "K.1 Konfigurierte Workflow-Prüfungen",
        ),
        (
            "K.2 Pre-release audio screening",
            "K.2 Audio-Screening vor Veröffentlichung",
        ),
        (
            "Pre-release audio screening",
            "Audio-Screening vor Veröffentlichung",
        ),
        ("Local screening status", "Status des lokalen Screenings"),
        ("Local engine", "Lokale Engine"),
        ("Local engine version", "Version der lokalen Engine"),
        ("Fingerprint algorithm", "Fingerprint-Algorithmus"),
        ("Local source Evidence ID", "Lokale Quell-Evidence-ID"),
        ("Local source path", "Lokaler Quellpfad"),
        ("Local source SHA-256", "Lokaler Quell-SHA-256"),
        ("Local source size (bytes)", "Lokale Quellgröße (Bytes)"),
        ("Local measured duration (ms)", "Lokal gemessene Dauer (ms)"),
        ("Local record path", "Pfad des lokalen Datensatzes"),
        ("Local record SHA-256", "SHA-256 des lokalen Datensatzes"),
        ("Local generated at", "Lokal erzeugt am"),
        (
            "External screening provider",
            "Anbieter des externen Screenings",
        ),
        (
            "External screening status",
            "Status des externen Screenings",
        ),
        (
            "External provider configured at snapshot",
            "Externer Anbieter im Snapshot konfiguriert",
        ),
        ("External source Evidence ID", "Externe Quell-Evidence-ID"),
        ("External source path", "Externer Quellpfad"),
        ("External source SHA-256", "Externer Quell-SHA-256"),
        ("External checked at", "Extern geprüft am"),
        (
            "External sample offset (ms)",
            "Offset der externen Probe (ms)",
        ),
        (
            "External sample duration (ms)",
            "Dauer der externen Probe (ms)",
        ),
        (
            "External source duration (ms)",
            "Dauer der externen Quelle (ms)",
        ),
        ("External request count", "Anzahl externer Anfragen"),
        ("External response archive", "Archiv der externen Antwort"),
        ("External response SHA-256", "SHA-256 der externen Antwort"),
        ("Provider matches", "Anbietertreffer"),
        ("Provider match", "Anbietertreffer"),
        (
            "Provider-derived metadata",
            "Vom Anbieter abgeleitete Metadaten",
        ),
        (
            "L. Technical certificate statement",
            "L. Erklärung zum technischen Zertifikat",
        ),
        (
            "Configured documentation requirements for this step were satisfied.",
            "Die konfigurierten Dokumentationsanforderungen für diesen Schritt wurden erfüllt.",
        ),
        (
            "configured documentation requirements completed",
            "konfigurierte Dokumentationsanforderungen abgeschlossen",
        ),
        (
            "Historical user data; not a plan-at-generation claim",
            "Historische Nutzerdaten; keine Aussage zum Tarif bei der Erzeugung",
        ),
        ("Evidence-derived metadata", "Aus Evidenzmetadaten"),
        ("User-confirmed fact", "Vom Nutzer bestätigte Angabe"),
        ("System verification", "Systemprüfung"),
        ("System value", "Systemwert"),
        (
            "Actual Suno export filename",
            "Tatsächlicher Dateiname des Suno-Exports",
        ),
        (
            "Actual release filename",
            "Tatsächlicher Dateiname der Veröffentlichung",
        ),
        (
            "Final generation date origin",
            "Herkunft des Datums der finalen Erzeugung",
        ),
        ("Final generation date", "Datum der finalen Erzeugung"),
        ("Final generation ID", "ID der finalen Erzeugung"),
        (
            "Final-generation date covered",
            "Datum der finalen Erzeugung abgedeckt",
        ),
        ("Finalized at", "Finalisiert am"),
        ("Finalization timestamp", "Zeitpunkt der Finalisierung"),
        ("Final result", "Endergebnis"),
        ("Suno project URL", "Suno-Projekt-URL"),
        (
            "Suno Studio metadata detected",
            "Suno-Studio-Metadaten erkannt",
        ),
        (
            "Metadata detection origin",
            "Herkunft der Metadatenerkennung",
        ),
        ("Metadata origin", "Metadatenherkunft"),
        ("Suno plan at generation", "Suno-Tarif bei der Erzeugung"),
        ("Suno model", "Suno-Modell"),
        (
            "Download/export date origin",
            "Herkunft des Download-/Exportdatums",
        ),
        ("Download/export date", "Download-/Exportdatum"),
        (
            "Release identical to Suno final export",
            "Release identisch mit finalem Suno-Export",
        ),
        ("Release identity origin", "Herkunft der Release-Identität"),
        (
            "Assigned subscription evidence jointly covers the production period",
            "Zugeordnete Abo-Evidence deckt den Produktionszeitraum gemeinsam ab",
        ),
        (
            "Terms evidence not available",
            "Evidence zu Nutzungsbedingungen nicht verfügbar",
        ),
        (
            "Terms evidence exists",
            "Evidence zu Nutzungsbedingungen vorhanden",
        ),
        (
            "Terms evidence IDs",
            "IDs der Evidence zu Nutzungsbedingungen",
        ),
        (
            "Archived service-terms evidence",
            "Archivierte Evidence zu Nutzungsbedingungen",
        ),
        (
            "External timestamp evidence at technical finalization",
            "Externer Zeitstempelnachweis bei technischer Finalisierung",
        ),
        ("Evidence file count", "Anzahl der Evidence-Dateien"),
        ("Previous revision archives", "Archive früherer Revisionen"),
        ("Blocking deviations", "Blockierende Abweichungen"),
        (
            "Mandatory steps completed",
            "Abgeschlossene Pflichtschritte",
        ),
        ("N/A steps with reasons", "N/A-Schritte mit Begründungen"),
        ("Application version", "Anwendungsversion"),
        ("Certificate schema", "Zertifikatsschema"),
        ("Certificate version", "Zertifikatsversion"),
        ("Certificate ID", "Zertifikats-ID"),
        ("Page", "Seite"),
        ("Documentation status", "Dokumentationsstatus"),
        ("Documented title", "Dokumentierter Titel"),
        ("Track identity", "Track-Identität"),
        ("Track", "Track"),
        ("Artist", "Künstler/in"),
        ("Production start", "Produktionsbeginn"),
        ("Production end", "Produktionsende"),
        ("Last editing date", "Datum der letzten Bearbeitung"),
        ("Commercial use intended", "Kommerzielle Nutzung vorgesehen"),
        ("Suno profile", "Suno-Profil"),
        ("Suno handle", "Suno-Benutzername"),
        ("External audio uploaded", "Externes Audio hochgeladen"),
        ("Own audio uploaded", "Eigenes Audio hochgeladen"),
        (
            "Third-party samples uploaded",
            "Samples Dritter hochgeladen",
        ),
        ("Third-party samples", "Samples Dritter"),
        ("Code-based generation", "Codebasierte Erzeugung"),
        ("Source-code evidence", "Evidence zum Quellcode"),
        (
            "Code-generated audio evidence",
            "Evidence zu codegeneriertem Audio",
        ),
        (
            "Code-audio post-processing operations",
            "Nachbearbeitungsschritte für codegeneriertes Audio",
        ),
        (
            "Code-audio post-processing",
            "Nachbearbeitung für codegeneriertes Audio",
        ),
        (
            "External audio provenance statement",
            "Herkunftsangabe zu externem Audio",
        ),
        ("External audio source", "Quelle des externen Audios"),
        (
            "Own audio provenance statement",
            "Herkunftsangabe zu eigenem Audio",
        ),
        ("Own audio source", "Quelle des eigenen Audios"),
        (
            "Third-party sample provenance statement",
            "Herkunftsangabe zu Samples Dritter",
        ),
        ("Third-party sample source", "Quelle der Samples Dritter"),
        (
            "Other code-audio post-processing note",
            "Sonstige Anmerkung zur Nachbearbeitung des codegenerierten Audios",
        ),
        (
            "Human editing performed",
            "Menschliche Bearbeitung durchgeführt",
        ),
        (
            "Confirmed human editing",
            "Bestätigte menschliche Bearbeitung",
        ),
        (
            "Desktop-PC editing after the Suno WAV",
            "Desktop-PC-Bearbeitung nach dem Suno-WAV",
        ),
        (
            "Confirmed desktop-PC editing",
            "Bestätigte Desktop-PC-Bearbeitung",
        ),
        (
            "Confirmed human artwork process",
            "Bestätigter menschlicher Artwork-Prozess",
        ),
        (
            "Human artwork process notes",
            "Anmerkungen zum menschlichen Artwork-Prozess",
        ),
        (
            "Confirmed human artwork modifications",
            "Bestätigte menschliche Artwork-Änderungen",
        ),
        (
            "Other human artwork change",
            "Sonstige menschliche Artwork-Änderung",
        ),
        (
            "Suno Instrumental Mode Selected",
            "Ausgewählter Suno-Instrumentalmodus",
        ),
        (
            "Generation Text Field Available",
            "Generierungstextfeld verfügbar",
        ),
        (
            "Generation Text Field Used",
            "Generierungstextfeld verwendet",
        ),
        ("Content Classification", "Inhaltsklassifizierung"),
        ("Vocal Lyrics Present", "Vocal Lyrics vorhanden"),
        (
            "Structure Instructions Present",
            "Strukturanweisungen vorhanden",
        ),
        ("Vocal Intent", "Vokale Intention"),
        (
            "Final Audio Contains Vocals",
            "Finaler Audioinhalt enthält Gesang",
        ),
        ("Instrumental track", "Instrumentaltrack"),
        ("Vocal lyrics present", "Vocal Lyrics vorhanden"),
        (
            "Structure instructions present",
            "Strukturanweisungen vorhanden",
        ),
        (
            "Exact Generation Text Field Content",
            "Exakter Text des Suno-Textfelds",
        ),
        ("Legacy source value", "Historischer Quellenwert"),
        ("Legacy text", "Historischer Text"),
        ("Classification", "Klassifizierung"),
        ("Generative AI used", "Generative KI verwendet"),
        ("AI system", "KI-System"),
        ("AI-assisted audio elements", "KI-assistierte Audioelemente"),
        ("AI-generated audio elements", "KI-generierte Audioelemente"),
        (
            "Real person voice intentionally imitated",
            "Stimme einer realen Person absichtlich imitiert",
        ),
        (
            "Real person's identity intentionally represented",
            "Identität einer realen Person absichtlich dargestellt",
        ),
        (
            "Real event represented as authentic recording",
            "Reales Ereignis als authentische Aufnahme dargestellt",
        ),
        (
            "Real location / institution / event presented as authentic AI recording",
            "Realer Ort / Institution / Ereignis als authentische KI-Aufnahme dargestellt",
        ),
        ("Disclosure applied", "Hinweis angewendet"),
        ("Disclosure locations", "Orte des Hinweises"),
        ("Disclosure text", "Hinweistext"),
        ("Disclosure reason / note", "Grund / Anmerkung zum Hinweis"),
        (
            "Deepfake-related indicator summary",
            "Zusammenfassung zu Deepfake-bezogenen Indikatoren",
        ),
        ("Suno style prompt", "Suno-Style-Prompt"),
        ("Artwork origin", "Herkunft des Artworks"),
        ("AI image service", "KI-Bilddienst"),
        ("Human artwork process", "Menschlicher Artwork-Prozess"),
        ("Human artwork changes", "Menschliche Artwork-Änderungen"),
        ("Depicts real person", "Reale Person dargestellt"),
        ("Real-person note", "Anmerkung zur realen Person"),
        ("Depicts real event", "Reales Ereignis dargestellt"),
        ("Real-event note", "Anmerkung zum realen Ereignis"),
        ("Trademark/logo note", "Anmerkung zu Marke/Logo"),
        ("Trademark/logo", "Marke/Logo"),
        ("Artwork disclosure applied", "Artwork-Hinweis angewendet"),
        ("Artwork disclosure text", "Artwork-Hinweistext"),
        ("Subscription evidence", "Abo-Evidence"),
        ("Terms document", "Dokument zu Nutzungsbedingungen"),
        ("Evidence ID", "Evidence-ID"),
        ("Original filename", "Ursprünglicher Dateiname"),
        ("Original file name", "Ursprünglicher Dateiname"),
        ("Managed filename", "Verwalteter Dateiname"),
        ("File name", "Dateiname"),
        ("Relative path", "Relativer Pfad"),
        ("Document title", "Dokumenttitel"),
        ("Provider/source", "Anbieter/Quelle"),
        ("Source URL", "Quell-URL"),
        ("Retrieval date", "Abrufdatum"),
        ("Effective date", "Gültigkeitsdatum"),
        (
            "Applicable production period",
            "Anwendbarer Produktionszeitraum",
        ),
        ("Factual note", "Sachliche Anmerkung"),
        ("Imported at", "Importiert am"),
        ("Provenance", "Herkunft"),
        (
            "Source global evidence ID",
            "ID der globalen Ausgangs-Evidence",
        ),
        ("Derived from evidence ID", "Abgeleitet von Evidence-ID"),
        ("Generator version", "Generatorversion"),
        ("Generated disclosure text", "Generierter Hinweistext"),
        ("Coverage start", "Abdeckungsbeginn"),
        ("Coverage end", "Abdeckungsende"),
        (
            "Phase-two attachment policy",
            "Richtlinie für Phase-zwei-Anhänge",
        ),
        (
            "Post-finalization addendum; phase-one snapshot remains unchanged",
            "Nachtrag nach Finalisierung; Snapshot der ersten Phase bleibt unverändert",
        ),
        ("Release audio SHA-256", "SHA-256 des Release-Audios"),
        ("Final artwork SHA-256", "SHA-256 des finalen Artworks"),
        ("Previous revision count", "Anzahl früherer Revisionen"),
        ("Generated by", "Erzeugt von"),
        ("Workflow version", "Workflow-Version"),
        ("Workflow ID", "Workflow-ID"),
        ("Workflow", "Workflow"),
        ("Meaning", "Bedeutung"),
        ("PASS definition", "PASS-Definition"),
        ("Role", "Rolle"),
        ("Size (bytes)", "Größe (Bytes)"),
        (
            "Evidence register (continuation)",
            "Evidenzregister (Fortsetzung)",
        ),
        ("(continuation)", "(Fortsetzung)"),
        ("NOT RECORDED", "NICHT ERFASST"),
        ("NOT DOCUMENTED", "NICHT DOKUMENTIERT"),
        ("NOT VERIFIED", "NICHT VERIFIZIERT"),
        ("DOCUMENTATION COMPLETE", "DOKUMENTATION ABGESCHLOSSEN"),
    ] {
        translated = translated.replace(source, target);
    }
    translated
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEvidence<'a> {
    id: &'a str,
    role: &'a str,
    file_name: &'a str,
    relative_path: &'a str,
    sha256: Option<&'a str>,
    size_bytes: u64,
    imported_at: &'a str,
    source_global_evidence_id: Option<&'a str>,
    coverage_start: Option<&'a str>,
    coverage_end: Option<&'a str>,
    provenance: &'a EvidenceProvenance,
    derived_from_evidence_id: Option<&'a str>,
    generator_version: Option<&'a str>,
    generated_disclosure_text: Option<&'a str>,
    metadata: serde_json::Value,
}

/// Sanitized portable screening snapshot for a new certificate manifest.
///
/// The full Chromaprint fingerprint is retained only in the dedicated local
/// screening artifact; it is deliberately not copied into a certificate
/// manifest. Raw provider response bytes, request signatures, and credentials
/// are likewise excluded. This keeps the manifest reviewable while preserving
/// the source/artifact binding needed for an integrity audit.
fn audio_screening_manifest(state: &AudioScreeningState) -> serde_json::Value {
    let local = &state.local;
    let external = &state.external;
    let matches = external.matches.iter().take(5).collect::<Vec<_>>();
    json!({
        "local": {
            "schemaVersion": local.schema_version,
            "status": local.status,
            "engine": local.engine,
            "engineVersion": local.engine_version,
            "fingerprintAlgorithm": local.fingerprint_algorithm,
            "sourceEvidenceId": local.source_evidence_id,
            "sourceRelativePath": local.source_relative_path,
            "sourceSha256": local.source_sha256,
            "sourceSizeBytes": local.source_size_bytes,
            "durationMilliseconds": local.duration_milliseconds,
            "generatedAt": local.generated_at,
            "artifactRelativePath": local.artifact_relative_path,
            "artifactSha256": local.artifact_sha256,
        },
        "external": {
            "schemaVersion": external.schema_version,
            "provider": external.provider,
            "status": external.status,
            "sourceEvidenceId": external.source_evidence_id,
            "sourceRelativePath": external.source_relative_path,
            "sourceSha256": external.source_sha256,
            "sourceSizeBytes": external.source_size_bytes,
            "checkedAt": external.checked_at,
            "sampleOffsetMilliseconds": external.sample_offset_milliseconds,
            "sampleDurationMilliseconds": external.sample_duration_milliseconds,
            "sourceDurationMilliseconds": external.source_duration_milliseconds,
            "requestCount": external.request_count,
            "responseRelativePath": external.response_relative_path,
            "responseSha256": external.response_sha256,
            "configuredAtSnapshot": external.configured_at_snapshot,
            "matches": matches,
        },
        "statementScope": "technical comparison record only; no authorship, ownership, permission, infringement, legality, release-clearance, or legal conclusion",
    })
}

/// The certificate's K.2 section intentionally contains a concise, factual
/// screening summary. It never displays a raw local fingerprint, raw provider
/// response, request signature, or credential.
fn audio_screening_markdown(state: &AudioScreeningState) -> String {
    let local = &state.local;
    let external = &state.external;
    let mut output = format!(
        "- Local screening status [System verification]: **{}**\n- Local engine [System verification]: {}\n- Local engine version [System verification]: {}\n- Fingerprint algorithm [System verification]: {}\n- Local source Evidence ID [System verification]: {}\n- Local source path [System verification]: `{}`\n- Local source SHA-256 [System verification]: `{}`\n- Local source size (bytes) [System verification]: {}\n- Local measured duration (ms) [System verification]: {}\n- Local record path [System verification]: `{}`\n- Local record SHA-256 [System verification]: `{}`\n- Local generated at [System value]: {}\n\n- External screening provider [System value]: {}\n- External screening status [System verification]: **{}**\n- External provider configured at snapshot [System value]: {}\n- External source Evidence ID [System verification]: {}\n- External source path [System verification]: `{}`\n- External source SHA-256 [System verification]: `{}`\n- External checked at [System value]: {}\n- External sample offset (ms) [System value]: {}\n- External sample duration (ms) [System value]: {}\n- External source duration (ms) [System value]: {}\n- External request count [System value]: {}\n- External response archive [System verification]: `{}`\n- External response SHA-256 [System verification]: `{}`\n",
        audio_screening_status_label(local.status),
        documented(&local.engine),
        documented(&local.engine_version),
        documented(&local.fingerprint_algorithm),
        documented(&local.source_evidence_id),
        documented(&local.source_relative_path),
        documented(&local.source_sha256),
        local.source_size_bytes,
        local
            .duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        documented(&local.artifact_relative_path),
        documented(&local.artifact_sha256),
        local.generated_at.as_deref().unwrap_or("NOT DOCUMENTED"),
        documented(&external.provider),
        audio_screening_status_label(external.status),
        recorded_bool(external.configured_at_snapshot),
        documented(&external.source_evidence_id),
        documented(&external.source_relative_path),
        documented(&external.source_sha256),
        external.checked_at.as_deref().unwrap_or("NOT DOCUMENTED"),
        external
            .sample_offset_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external
            .sample_duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external
            .source_duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external.request_count,
        external
            .response_relative_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("NOT RECORDED"),
        external
            .response_sha256
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("NOT RECORDED"),
    );

    if external.matches.is_empty() {
        output.push_str("- Provider matches [Provider-derived metadata]: NONE RECORDED\n");
    } else {
        for (index, item) in external.matches.iter().take(5).enumerate() {
            let artists = if item.artists.is_empty() {
                "NOT DOCUMENTED".to_owned()
            } else {
                item.artists.join(", ")
            };
            let mut value = format!("{} — {artists}", documented(&item.title));
            if let Some(album) = item
                .album
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; album {album}"));
            }
            if let Some(isrc) = item
                .isrc
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; ISRC {isrc}"));
            }
            if let Some(acrid) = item
                .acrid
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                value.push_str(&format!("; ACRID {acrid}"));
            }
            if let Some(score) = item.score {
                value.push_str(&format!("; score {score}"));
            }
            output.push_str(&format!(
                "- Provider match {} [Provider-derived metadata]: {value}\n",
                index + 1
            ));
        }
    }
    output.push_str("\nAudio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion.\n");
    output
}

fn audio_screening_status_label(status: AudioScreeningStatus) -> &'static str {
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

fn phase_one_metadata(metadata: &EvidenceMetadata) -> Result<serde_json::Value> {
    let mut sanitized = serde_json::to_value(metadata)?;
    if let Some(object) = sanitized.as_object_mut() {
        for field in [
            "timestampType",
            "externalTimestamp",
            "referencedHash",
            "referencedArtifact",
            "externalReferenceId",
            "providerVerificationUrl",
        ] {
            object.remove(field);
        }
    }
    Ok(sanitized)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AutomaticEvidenceRelationship {
    kind: &'static str,
    source_evidence_id: String,
    source_role: &'static str,
    target_evidence_id: String,
    target_role: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AutomaticGlobalTrackRelationship {
    kind: &'static str,
    source_global_evidence_id: String,
    materialized_evidence_id: String,
    role: &'static str,
    target_track_id: String,
}

fn automatic_role_relationships(evidence: &[&EvidenceItem]) -> Vec<AutomaticEvidenceRelationship> {
    let mut relationships = Vec::new();

    append_role_relationships(
        evidence,
        EvidenceRole::SourceCodeFile,
        EvidenceRole::CodeGeneratedAudioFile,
        "source_to_generated_audio",
        &mut relationships,
    );
    for (source_role, target_role) in [
        (
            EvidenceRole::AiArtworkOriginal,
            EvidenceRole::AiArtworkEdited,
        ),
        (
            EvidenceRole::AiArtworkEdited,
            EvidenceRole::HumanEditedArtwork,
        ),
        (EvidenceRole::HumanEditedArtwork, EvidenceRole::FinalArtwork),
    ] {
        append_role_relationships(
            evidence,
            source_role,
            target_role,
            "artwork_stage",
            &mut relationships,
        );
    }

    relationships.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.source_evidence_id.cmp(&right.source_evidence_id))
            .then_with(|| left.target_evidence_id.cmp(&right.target_evidence_id))
    });
    relationships
}

fn automatic_global_track_relationships(
    track_id: &str,
    evidence: &[&EvidenceItem],
) -> Vec<AutomaticGlobalTrackRelationship> {
    let mut relationships = evidence
        .iter()
        .copied()
        .filter(|item| item.provenance == EvidenceProvenance::GlobalCopy)
        .filter_map(|item| {
            item.source_global_evidence_id
                .as_deref()
                .filter(|source_id| !source_id.trim().is_empty())
                .map(|source_id| AutomaticGlobalTrackRelationship {
                    kind: "global_evidence_to_track",
                    source_global_evidence_id: source_id.to_owned(),
                    materialized_evidence_id: item.id.clone(),
                    role: item.role.as_str(),
                    target_track_id: track_id.to_owned(),
                })
        })
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| {
        left.source_global_evidence_id
            .cmp(&right.source_global_evidence_id)
            .then_with(|| {
                left.materialized_evidence_id
                    .cmp(&right.materialized_evidence_id)
            })
    });
    relationships
}

fn append_role_relationships(
    evidence: &[&EvidenceItem],
    source_role: EvidenceRole,
    target_role: EvidenceRole,
    kind: &'static str,
    relationships: &mut Vec<AutomaticEvidenceRelationship>,
) {
    let sources = evidence
        .iter()
        .copied()
        .filter(|item| item.role == source_role)
        .collect::<Vec<_>>();
    let targets = evidence
        .iter()
        .copied()
        .filter(|item| item.role == target_role)
        .collect::<Vec<_>>();

    for target in &targets {
        let Some(source_id) = target
            .derived_from_evidence_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        if let Some(source) = sources
            .iter()
            .copied()
            .find(|source| source.id == source_id)
        {
            relationships.push(evidence_relationship(kind, source, target));
        }
    }

    // Role inference is only a safe ID-level statement when both concrete
    // stages are singletons and the target has no explicit lineage claim.
    if sources.len() == 1 && targets.len() == 1 && targets[0].derived_from_evidence_id.is_none() {
        relationships.push(evidence_relationship(kind, sources[0], targets[0]));
    }
}

fn evidence_relationship(
    kind: &'static str,
    source: &EvidenceItem,
    target: &EvidenceItem,
) -> AutomaticEvidenceRelationship {
    AutomaticEvidenceRelationship {
        kind,
        source_evidence_id: source.id.clone(),
        source_role: source.role.as_str(),
        target_evidence_id: target.id.clone(),
        target_role: target.role.as_str(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
    render_options: CertificateRenderOptions,
) -> Result<()> {
    generate_impl(
        track_root,
        track,
        profile,
        steps,
        evidence,
        deviations,
        certificate_id,
        finalized_at,
        transaction_id,
        render_options,
        #[cfg(test)]
        None,
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CertificateGenerationFailure {
    PdfGeneration,
    PdfStaging,
    PdfPublication,
    PostPublishVerification,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_with_failure(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
    render_options: CertificateRenderOptions,
    failure: CertificateGenerationFailure,
) -> Result<()> {
    generate_impl(
        track_root,
        track,
        profile,
        steps,
        evidence,
        deviations,
        certificate_id,
        finalized_at,
        transaction_id,
        render_options,
        Some(failure),
    )
}

#[allow(clippy::too_many_arguments)]
fn generate_impl(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
    render_options: CertificateRenderOptions,
    #[cfg(test)] failure: Option<CertificateGenerationFailure>,
) -> Result<()> {
    let hash_manifest = contained_path(track_root, Path::new(HASH_FILE), true)?;
    let hash_manifest_sha = sha256_file(&hash_manifest)?;
    let hashes = parse_hashes(&hash_manifest)?;
    let mut evidence_values = evidence
        .iter()
        .filter(|item| {
            item.role != EvidenceRole::ExternalTimestamp
                && item.verified
                && item.sha256.is_some()
                && item.verification_error.is_none()
        })
        .collect::<Vec<_>>();
    evidence_values.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if track.fields.suno_terms_evidence_not_available == Some(true)
        && evidence_values
            .iter()
            .any(|item| item.role == EvidenceRole::SunoTermsRights)
    {
        return Err(AppError::Validation(
            "Certificate generation cannot combine verified Terms evidence with an unavailable claim."
                .into(),
        ));
    }
    let archived_revisions = archived_revision_references(track_root)?;
    let evidence_manifest = evidence_values
        .iter()
        .map(|item| -> Result<ManifestEvidence<'_>> {
            Ok(ManifestEvidence {
                id: &item.id,
                role: item.role.as_str(),
                file_name: &item.file_name,
                relative_path: &item.relative_path,
                sha256: item.sha256.as_deref(),
                size_bytes: item.size_bytes,
                imported_at: &item.imported_at,
                source_global_evidence_id: item.source_global_evidence_id.as_deref(),
                coverage_start: item.coverage_start.as_deref(),
                coverage_end: item.coverage_end.as_deref(),
                provenance: &item.provenance,
                derived_from_evidence_id: item.derived_from_evidence_id.as_deref(),
                generator_version: item.generator_version.as_deref(),
                generated_disclosure_text: item.generated_disclosure_text.as_deref(),
                metadata: phase_one_metadata(&item.metadata)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let automation = crate::workflow::automation_summary(track, evidence);
    let byte_identical_pairs = crate::workflow::byte_identical_pairs(evidence);
    let automatic_relationships = automatic_role_relationships(&evidence_values);
    let automatic_global_relationships =
        automatic_global_track_relationships(&track.id, &evidence_values);
    let manifest = json!({
        "schema_version": 6,
        "track": {
            "id": track.id,
            "title": track.fields.title,
            "relative_path": ".",
            "production_start_date": track.fields.production_start_date,
            "production_end_date": track.fields.production_end_date,
            "final_export_date": track.fields.final_export_date,
        },
        "artist": {
            "name": profile.artist_name,
            "suno_profile_name": profile.suno_profile_name,
            "suno_handle": profile.suno_handle,
        },
        "documented_facts": track.fields,
        "semantic_snapshot": {
            "suno_lyrics_structure": {
                "instrumental_track": recorded_bool(track.fields.instrumental_track),
                "vocal_lyrics_present": recorded_bool(track.fields.vocal_lyrics_present),
                "suno_lyrics_field_content": recorded_bool(track.fields.suno_lyrics_field_content),
                "content_types": suno_content_types(&track.fields),
                "structure_instructions_present": structure_instructions_present(&track.fields),
                "content_source": suno_content_source(&track.fields),
                "exact_field_text": match track.fields.suno_lyrics_field_content {
                    Some(true) => documented(&track.fields.suno_lyrics_field_text),
                    Some(false) => "N/A",
                    None => "NOT DOCUMENTED",
                },
                "legacy_data_classification": if track.fields.lyrics_source.trim().is_empty()
                    && track.fields.lyrics_text.trim().is_empty()
                {
                    "N/A"
                } else {
                    "NOT DOCUMENTED"
                },
            },
            "ai_transparency_audio": {
                "generative_ai_used": recorded_bool(track.fields.generative_ai_used),
                "ai_assisted_audio_elements": conditional_documentation_answer(
                    track.fields.generative_ai_used,
                    track.fields.ai_assisted_audio_elements,
                ),
                "ai_generated_audio_elements": conditional_documentation_answer(
                    track.fields.generative_ai_used,
                    track.fields.ai_generated_audio_elements,
                ),
                "audio_disclosure_applied": conditional_documentation_answer(
                    track.fields.generative_ai_used,
                    track.fields.audio_disclosure_applied,
                ),
                "deepfake_related_indicator_summary": deepfake_indicator_summary(&track.fields),
            },
        },
        "profile_snapshot": profile,
        "workflow": {
            "id": track.workflow_id,
            "version": track.workflow_version,
            "application_version": env!("CARGO_PKG_VERSION"),
        },
        "finalization": {
            "timestamp": finalized_at,
            "result": "DOCUMENTATION COMPLETE",
            "meaning": "configured documentation requirements completed",
        },
        "steps": steps,
        "evidence": evidence_manifest,
        "audio_screening": audio_screening_manifest(&track.audio_screening),
        "hashes": hashes,
        "certificate": {
            "id": certificate_id,
            "format_version": CERTIFICATE_FORMAT_VERSION,
            "rendering": render_options,
            "pdf_languages": ["en", "de"],
            "pdf_files": [PDF_FILE, PDF_FILE_DE],
            "status": "DOCUMENTATION COMPLETE",
            "status_meaning": "configured documentation requirements completed",
            "workflow_pass_meaning": "Configured documentation requirements for this step were satisfied.",
            "sha256sums_sha256": hash_manifest_sha,
            "statement_scope": {
                "confirms": [
                    "recorded user inputs", "finalized snapshot", "registered local evidence",
                    "recorded provenance", "SHA-256 values", "configured workflow checks"
                ],
                "does_not_confirm": [
                    "authorship", "rights ownership", "non-infringement", "legality",
                    "license validity", "judicial evidentiary weight", "statutory compliance",
                    "governmental certification"
                ]
            },
        },
        "origin_labels": {
            "user_confirmed_fact": "Values explicitly entered or confirmed by the user",
            "evidence_derived_metadata": "Metadata read from or captured with a local evidence import",
            "system_verification": "Local structural, hash, consistency, and configured workflow checks"
        },
        "evidence_derived_metadata": {
            "suno_created_timestamp": automation.suno_created_timestamp,
            "suno_id": automation.suno_id,
        },
        "system_verification": {
            "subscription_final_generation_coverage": crate::workflow::subscription_generation_coverage(track, evidence),
            "subscription_production_coverage": crate::workflow::subscription_production_coverage(track, evidence),
            "release_original_file_name": crate::workflow::original_evidence_file_name(evidence, EvidenceRole::ReleaseWav),
            "suno_export_original_file_name": crate::workflow::original_evidence_file_name(evidence, EvidenceRole::SunoFinalExport),
            "external_timestamp_at_technical_finalization": "NOT RECORDED",
            "fact_origins": {
                "final_suno_generation_id": automation.final_generation_id_origin,
                "final_suno_generation_date": automation.final_generation_origin,
                "production_end_date": automation.production_end_origin,
                "download_export_date": automation.download_export_origin,
                "last_editing_date": automation.final_export_origin,
            },
            "suno_metadata_detected": automation.suno_metadata_detected,
            "release_identical_to_suno_export": automation.release_identical_to_suno_export,
            "byte_identical_pairs": &byte_identical_pairs,
            "automatic_role_relationships": &automatic_relationships,
            "automatic_global_track_relationships": &automatic_global_relationships,
            "consistency_issues": &automation.consistency_issues,
        },
        "external_timestamp": {
            "status_at_technical_finalization": "NOT RECORDED",
            "attachment_phase": "post_finalization_addendum",
            "changes_phase_one_snapshot": false,
        },
        "deviations": deviations,
        "revision_archives": &archived_revisions,
    });
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    manifest_bytes.push(b'\n');
    let manifest_sha = sha256_bytes(&manifest_bytes);

    let release_wav = evidence_values
        .iter()
        .copied()
        .find(|item| item.role == EvidenceRole::ReleaseWav)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("NOT DOCUMENTED");
    let final_artwork = evidence_values
        .iter()
        .copied()
        .find(|item| item.role == EvidenceRole::FinalArtwork)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("N/A");
    let na_steps = steps
        .iter()
        .filter(|step| step.status == StepStatus::NotApplicable)
        .map(|step| {
            format!(
                "- {} — {}\n",
                step.id,
                step.na_reason.as_deref().unwrap_or("NOT DOCUMENTED")
            )
        })
        .collect::<String>();
    let completed_steps = steps
        .iter()
        .filter(|step| matches!(step.status, StepStatus::Pass | StepStatus::NotApplicable))
        .map(|step| format!("- {}: {}\n", step.id, step_status_label(&step.status)))
        .collect::<String>();
    let open_blocking = deviations
        .iter()
        .filter(|d| d.blocking && !d.resolved)
        .count();
    let release_file_name =
        crate::workflow::original_evidence_file_name(evidence, EvidenceRole::ReleaseWav)
            .unwrap_or("NOT RECORDED");
    let suno_export_file_name =
        crate::workflow::original_evidence_file_name(evidence, EvidenceRole::SunoFinalExport)
            .unwrap_or("NOT RECORDED");
    let generation_coverage =
        match crate::workflow::subscription_generation_coverage(track, evidence) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        };
    let production_coverage =
        match crate::workflow::subscription_production_coverage(track, evidence) {
            crate::workflow::CoverageStatus::Yes => "YES",
            crate::workflow::CoverageStatus::No => "NO",
            crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
        };
    let final_generation_origin = fact_origin_label(automation.final_generation_origin);
    let final_generation_id_origin = fact_origin_label(automation.final_generation_id_origin);
    let download_export_origin = fact_origin_label(automation.download_export_origin);
    let last_editing_origin = fact_origin_label(automation.final_export_origin);
    let last_editing_date = documented(&track.fields.final_export_date);
    let suno_metadata_detected = yes_no(automation.suno_metadata_detected);
    let release_identical_to_suno_export = yes_no(automation.release_identical_to_suno_export);
    let evidence_register_md = evidence_register_markdown(&evidence_values);
    let terms = evidence_values
        .iter()
        .copied()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .collect::<Vec<_>>();
    let terms_ids = if terms.is_empty() {
        "N/A".to_owned()
    } else {
        terms
            .iter()
            .map(|item| format!("`{}`", item.id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let terms_details_md = terms_evidence_markdown(&terms);
    let source_provenance_md = source_provenance_markdown(&track.fields, &evidence_values);
    let suno_plan_md = suno_plan_context_markdown(&track.fields);
    let suno_field_md = suno_field_markdown(&track.fields);
    let human_contribution_md = human_contribution_markdown(&track.fields);
    let ai_audio_md = ai_audio_markdown(&track.fields);
    let ai_artwork_md = ai_artwork_markdown(&track.fields);
    let audio_screening_md = audio_screening_markdown(&track.audio_screening);
    let revision_archives = if archived_revisions.is_empty() {
        "NONE RECORDED".to_owned()
    } else {
        archived_revisions.join(", ")
    };
    let english_certificate = format!(
        "# SunoDM Technical Documentation and Evidence Certificate\n\n> Technical documentation only — not a legal or governmental certification.\n\n## A. Certificate / Snapshot Identity\n\n- Certificate ID: `{certificate_id}`\n- Application version: `{}`\n- Workflow: `{}` / `{}`\n- Certificate schema: `{CERTIFICATE_FORMAT_VERSION}`\n- Finalized at: `{finalized_at}`\n- Documentation status: **DOCUMENTATION COMPLETE**\n- Meaning: configured documentation requirements completed\n- PASS definition: Configured documentation requirements for this step were satisfied.\n\n## B. Track identity\n\n- Documented title [User-confirmed fact]: {}\n- Artist [User-confirmed fact]: {}\n- Actual release filename [Evidence-derived metadata]: `{release_file_name}`\n- Actual Suno export filename [Evidence-derived metadata]: `{suno_export_file_name}`\n- Last editing date [{last_editing_origin}]: {last_editing_date}\n\n## C. Final Suno Generation\n\n- Final generation date [{final_generation_origin}]: {}\n- Final generation date origin: **{final_generation_origin}**\n- Final generation ID [{final_generation_id_origin}]: {}\n- Suno project URL [User-confirmed fact]: {}\n- Download/export date [{download_export_origin}]: {}\n- Download/export date origin: **{download_export_origin}**\n- Suno Studio metadata detected: **{suno_metadata_detected}**\n- Metadata detection origin: **System verification**\n- Metadata origin: {}\n- Suno model [User-confirmed fact]: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Release identical to Suno final export: **{release_identical_to_suno_export}**\n- Release identity origin: **System verification**\n\n## D. Source provenance\n\n{source_provenance_md}\n## E. Human contribution\n\n{human_contribution_md}\n{suno_field_md}\n## G. AI Transparency Assessment\n\n### G.1 Audio\n\n{ai_audio_md}\n### G.2 Artwork\n\n{ai_artwork_md}\n## H. License and rights evidence\n\n- Assigned subscription evidence jointly covers the production period [System verification]: **{production_coverage}**\n- Final-generation date covered [System verification]: **{generation_coverage}**\n- Terms evidence exists [System verification]: **{}**\n- Terms evidence IDs [System value]: {terms_ids}\n- Terms evidence not available [User-confirmed fact]: {}\n\n### Archived service-terms evidence\n\n{terms_details_md}\nThis is a factual coverage and archive status only; it is not a rights determination.\n\n## I. External Timestamp Evidence\n\n- External timestamp evidence at technical finalization: **NOT RECORDED**\n- No external timestamp evidence recorded.\n{}\nPost-finalization timestamp evidence, if later attached, is recorded in a separate addendum and does not change this technical-finalization snapshot.\n\n## J. Evidence register\n\n- Evidence file count: {}\n\n{evidence_register_md}\n## K. Integrity anchors and workflow\n\n- Release audio SHA-256: `{release_wav}`\n- Final artwork SHA-256: `{final_artwork}`\n- SHA256SUMS.txt SHA-256: `{hash_manifest_sha}`\n- Evidence manifest SHA-256: `{manifest_sha}`\n- Blocking deviations: {open_blocking}\n- Previous revision archives [System verification]: `{revision_archives}`\n- Final result: **DOCUMENTATION COMPLETE**\n\n### K.1 Configured workflow checks\n\n{completed_steps}\n### N/A steps with reasons\n\n{}\n### K.2 Pre-release audio screening\n\n{audio_screening_md}\n## L. Technical certificate statement\n\nThis certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks.\n\nIt does **not** confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification.\n\nOrigin labels used: **User-confirmed fact**, **Evidence-derived metadata**, **System verification**, and **System value**.\n",
        env!("CARGO_PKG_VERSION"),
        track.workflow_id,
        track.workflow_version,
        track.fields.title,
        profile.artist_name,
        documented(&track.fields.suno_final_generation_date),
        documented(&track.fields.suno_final_generation_id),
        documented(&track.fields.suno_project_url),
        documented(&track.fields.suno_download_export_date),
        if automation.suno_metadata_detected {
            "Evidence-derived metadata"
        } else {
            "NOT DOCUMENTED"
        },
        documented(&track.fields.suno_model),
        documented(&track.fields.suno_plan_at_generation),
        if terms.is_empty() { "NO" } else { "YES" },
        recorded_bool(track.fields.suno_terms_evidence_not_available),
        if track.fields.commercial_use_intended {
            "\nFor long-term evidentiary preservation, an external timestamp can be added after technical finalization.\n"
        } else {
            ""
        },
        evidence_values.len(),
        if na_steps.is_empty() {
            "- NONE\n"
        } else {
            &na_steps
        }
    );
    let english_certificate = english_certificate.replacen(
        &format!(
            "- Suno plan at generation [User-confirmed fact]: {}\n",
            documented(&track.fields.suno_plan_at_generation)
        ),
        &suno_plan_md,
        1,
    );
    let certificate = localized_markdown_certificate(&english_certificate, render_options);
    let certificate_sha = sha256_bytes(certificate.as_bytes());

    #[cfg(test)]
    if failure == Some(CertificateGenerationFailure::PdfGeneration) {
        return Err(AppError::Data(
            "Injected technical PDF generation failure.".into(),
        ));
    }
    let pdf_en = certificate_pdf::generate_pdf(&CertificatePdfSnapshot {
        track,
        automation: &automation,
        profile,
        steps,
        evidence: &evidence_values,
        deviations,
        revision_references: &archived_revisions,
        certificate_id,
        finalized_at,
        certificate_version: CERTIFICATE_FORMAT_VERSION,
        sha256sums_sha256: &hash_manifest_sha,
        evidence_manifest_sha256: &manifest_sha,
        markdown_certificate_sha256: &certificate_sha,
        render_options: CertificateRenderOptions {
            language: CertificateLanguage::En,
            bilingual: false,
        },
    })?;
    let pdf_de = certificate_pdf::generate_pdf(&CertificatePdfSnapshot {
        track,
        automation: &automation,
        profile,
        steps,
        evidence: &evidence_values,
        deviations,
        revision_references: &archived_revisions,
        certificate_id,
        finalized_at,
        certificate_version: CERTIFICATE_FORMAT_VERSION,
        sha256sums_sha256: &hash_manifest_sha,
        evidence_manifest_sha256: &manifest_sha,
        markdown_certificate_sha256: &certificate_sha,
        render_options: CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    })?;
    let pdf_en_sha = sha256_bytes(&pdf_en);
    let pdf_de_sha = sha256_bytes(&pdf_de);
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
        hash_manifest_sha,
        HASH_FILE,
        manifest_sha,
        MANIFEST_FILE,
        certificate_sha,
        CERTIFICATE_FILE,
        pdf_en_sha,
        PDF_FILE,
        pdf_de_sha,
        PDF_FILE_DE,
    );
    publish_certificate_set_impl(
        track_root,
        &manifest_bytes,
        certificate.as_bytes(),
        &pdf_en,
        &pdf_de,
        certificate_hashes.as_bytes(),
        transaction_id,
        #[cfg(test)]
        failure.and_then(CertificateGenerationFailure::publication_failure),
    )?;
    Ok(())
}

fn archived_revision_references(track_root: &Path) -> Result<Vec<String>> {
    let relative_root = Path::new(".archive/revisions");
    let revisions_root = contained_path(track_root, relative_root, false)?;
    if !revisions_root.exists() {
        return Ok(Vec::new());
    }
    if !revisions_root.is_dir() {
        return Err(AppError::Data(
            "The managed revision archive path is not a directory.".into(),
        ));
    }

    let mut references = Vec::new();
    for entry in
        fs::read_dir(&revisions_root).map_err(|error| AppError::io(&revisions_root, error))?
    {
        let entry = entry.map_err(|error| AppError::io(&revisions_root, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::io(entry.path(), error))?;
        if file_type.is_symlink() {
            return Err(AppError::Symlink(entry.path().display().to_string()));
        }
        if !file_type.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| AppError::Data("A revision archive ID is not valid UTF-8.".into()))?;
        let relative = relative_root.join(&name);
        let metadata = contained_path(track_root, &relative.join("revision.json"), false)?;
        if metadata.is_file() {
            references.push(portable_relative(&relative));
        }
    }
    references.sort();
    Ok(references)
}

pub fn verify(track_root: &Path) -> Result<()> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|e| AppError::io(&sums, e))?;
    let hashes = parse_certificate_hashes(&content)?;
    let mut verified_pdfs = BTreeMap::new();
    for (relative, expected) in &hashes {
        let path = contained_path(track_root, Path::new(relative), true)?;
        let actual = if is_certificate_pdf_path(relative) {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdfs.insert(relative.as_str(), bytes);
            digest
        } else {
            sha256_file(&path)?
        };
        if actual != *expected {
            return Err(AppError::Validation(format!(
                "Certificate integrity mismatch: {relative}"
            )));
        }
    }
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    for pdf_path in required_certificate_pdf_paths(&manifest, &hashes)? {
        let bytes = verified_pdfs.get(pdf_path).ok_or_else(|| {
            AppError::Validation(format!(
                "Certificate PDF hash entry was not verified: {pdf_path}"
            ))
        })?;
        certificate_pdf::validate_pdf_bytes(bytes)?;
    }
    Ok(())
}

/// Returns whether the live certificate format requires the root-level PDF.
///
/// This intentionally performs only the format/hash-set inspection needed by
/// interrupted-revision recovery. Full integrity validation remains in
/// [`verify`]. Legacy certificates without a format version use the historical
/// three-entry set and do not trigger PDF recovery.
pub(crate) fn expects_pdf(track_root: &Path) -> Result<bool> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|error| AppError::io(&sums, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    certificate_format_requires_pdf(&manifest, &hashes)
}

pub(crate) fn required_pdf_files(track_root: &Path) -> Result<Vec<&'static str>> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|error| AppError::io(&sums, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    Ok(required_certificate_pdf_paths(&manifest, &hashes)?.to_vec())
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CertificatePublicationFailure {
    StagingDirectoryCreate,
    ManifestWrite,
    CertificateWrite,
    PdfWrite,
    CertificateHashWrite,
    CertificatePublishRename,
    PdfPublish,
    PostPublishVerification,
}

#[cfg(test)]
impl CertificatePublicationFailure {
    fn label(self) -> &'static str {
        match self {
            Self::StagingDirectoryCreate => "staging-directory-create",
            Self::ManifestWrite => "manifest-write",
            Self::CertificateWrite => "certificate-write",
            Self::PdfWrite => "pdf-write",
            Self::CertificateHashWrite => "certificate-hash-write",
            Self::CertificatePublishRename => "certificate-publish-rename",
            Self::PdfPublish => "pdf-publish",
            Self::PostPublishVerification => "post-publish-verification",
        }
    }

    fn stage_id(self) -> String {
        format!("failure-injection-{}", self.label())
    }
}

#[cfg(test)]
impl CertificateGenerationFailure {
    fn publication_failure(self) -> Option<CertificatePublicationFailure> {
        match self {
            Self::PdfGeneration => None,
            Self::PdfStaging => Some(CertificatePublicationFailure::PdfWrite),
            Self::PdfPublication => Some(CertificatePublicationFailure::PdfPublish),
            Self::PostPublishVerification => {
                Some(CertificatePublicationFailure::PostPublishVerification)
            }
        }
    }
}

#[cfg(test)]
fn inject_certificate_publication_failure(
    configured: Option<CertificatePublicationFailure>,
    phase: CertificatePublicationFailure,
) -> Result<()> {
    if configured == Some(phase) {
        return Err(AppError::Data(format!(
            "Injected certificate publication failure at {}.",
            phase.label()
        )));
    }
    Ok(())
}

fn publish_certificate_set_impl(
    track_root: &Path,
    manifest: &[u8],
    certificate: &[u8],
    pdf_en: &[u8],
    pdf_de: &[u8],
    certificate_hashes: &[u8],
    transaction_id: &str,
    #[cfg(test)] failure: Option<CertificatePublicationFailure>,
) -> Result<()> {
    #[cfg(test)]
    let stage_id = failure
        .map(CertificatePublicationFailure::stage_id)
        .unwrap_or_else(|| transaction_id.to_owned());
    #[cfg(not(test))]
    let stage_id = transaction_id.to_owned();
    let stage_relative = PathBuf::from(".archive")
        .join("certificate-staging")
        .join(stage_id);
    #[cfg(test)]
    inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::StagingDirectoryCreate,
    )?;
    let stage = ensure_contained_directory(track_root, &stage_relative)?;
    let staged_certificate_dir = stage.join("certificate");
    let destination = contained_path(track_root, Path::new(CERTIFICATE_DIR), false)?;
    let pdf_destinations = [
        contained_path(track_root, Path::new(PDF_FILE), false)?,
        contained_path(track_root, Path::new(PDF_FILE_DE), false)?,
    ];
    let mut destination_started_empty = false;
    let mut certificate_published = false;
    let mut pdf_published = Vec::new();
    let publish_result = (|| -> Result<()> {
        fs::create_dir(&staged_certificate_dir)
            .map_err(|error| AppError::io(&staged_certificate_dir, error))?;
        let staged_manifest = staged_certificate_dir.join("EVIDENCE_MANIFEST.json");
        let staged_certificate = staged_certificate_dir.join("DOCUMENTATION_CERTIFICATE.md");
        let staged_hashes = staged_certificate_dir.join("CERTIFICATE_SHA256.txt");
        let staged_pdf = stage.join(PDF_FILE);
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::ManifestWrite,
        )?;
        atomic_write_new(&staged_manifest, manifest)?;
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificateWrite,
        )?;
        atomic_write_new(&staged_certificate, certificate)?;
        #[cfg(test)]
        inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfWrite)?;
        atomic_write_new(&staged_pdf, pdf_en)?;
        atomic_write_new(&stage.join(PDF_FILE_DE), pdf_de)?;
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificateHashWrite,
        )?;
        atomic_write_new(&staged_hashes, certificate_hashes)?;
        verify_staged_set(track_root, &stage)?;

        for pdf_destination in &pdf_destinations {
            if pdf_destination.exists() {
                return Err(AppError::Collision(pdf_destination.display().to_string()));
            }
        }
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificatePublishRename,
        )?;
        if destination.exists() {
            if !destination.is_dir() {
                return Err(AppError::Collision(destination.display().to_string()));
            }
            if fs::read_dir(&destination)
                .map_err(|error| AppError::io(&destination, error))?
                .next()
                .is_some()
            {
                return Err(AppError::Collision(destination.display().to_string()));
            }
            destination_started_empty = true;
            fs::remove_dir(&destination).map_err(|error| AppError::io(&destination, error))?;
        }
        fs::rename(&staged_certificate_dir, &destination)
            .map_err(|error| AppError::io(&destination, error))?;
        certificate_published = true;
        #[cfg(test)]
        inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfPublish)?;
        for (pdf_destination, staged_pdf) in pdf_destinations
            .iter()
            .zip([stage.join(PDF_FILE), stage.join(PDF_FILE_DE)])
        {
            copy_new(&staged_pdf, pdf_destination)?;
            pdf_published.push(pdf_destination.clone());
        }
        #[cfg(test)]
        let post_publish_verification = inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::PostPublishVerification,
        )
        .and_then(|()| verify(track_root));
        #[cfg(not(test))]
        let post_publish_verification = verify(track_root);
        post_publish_verification?;
        fs::remove_dir_all(&stage).map_err(|error| AppError::io(&stage, error))?;
        Ok(())
    })();

    if let Err(cause) = publish_result {
        let mut rollback_errors = Vec::new();
        for pdf_destination in pdf_published.iter().rev() {
            if let Err(error) = fs::remove_file(pdf_destination) {
                rollback_errors.push(format!("PDF cleanup failed: {error}"));
            }
        }
        if certificate_published && destination.exists() {
            if let Err(error) = fs::rename(&destination, &staged_certificate_dir) {
                rollback_errors.push(format!("certificate rollback failed: {error}"));
            }
        }
        if rollback_errors.is_empty() && stage.exists() {
            if let Err(error) = fs::remove_dir_all(&stage) {
                rollback_errors.push(format!("staging cleanup failed: {error}"));
            }
        }
        if rollback_errors.is_empty() && destination_started_empty && !destination.exists() {
            if let Err(error) = fs::create_dir(&destination) {
                rollback_errors.push(format!(
                    "empty certificate directory recovery failed: {error}"
                ));
            }
        }
        if rollback_errors.is_empty() {
            return Err(cause);
        }
        return Err(AppError::Data(format!(
            "Certificate publication failed ({cause}); {}",
            rollback_errors.join("; ")
        )));
    }

    Ok(())
}

fn verify_staged_set(track_root: &Path, stage: &Path) -> Result<()> {
    let certificate_stage = stage.join("certificate");
    let hashes_path = certificate_stage.join("CERTIFICATE_SHA256.txt");
    let content =
        fs::read_to_string(&hashes_path).map_err(|error| AppError::io(&hashes_path, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let mut verified_pdfs = BTreeMap::new();
    for (relative, expected) in &hashes {
        let path = match relative.as_str() {
            HASH_FILE => contained_path(track_root, Path::new(HASH_FILE), true)?,
            MANIFEST_FILE => certificate_stage.join("EVIDENCE_MANIFEST.json"),
            CERTIFICATE_FILE => certificate_stage.join("DOCUMENTATION_CERTIFICATE.md"),
            PDF_FILE => stage.join(PDF_FILE),
            PDF_FILE_DE => stage.join(PDF_FILE_DE),
            _ => return Err(AppError::Data("Unexpected certificate hash entry.".into())),
        };
        let actual = if is_certificate_pdf_path(relative) {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdfs.insert(relative.as_str(), bytes);
            digest
        } else {
            sha256_file(&path)?
        };
        if actual != *expected {
            return Err(AppError::Validation(format!(
                "Staged certificate integrity mismatch: {relative}"
            )));
        }
    }
    let manifest_path = certificate_stage.join("EVIDENCE_MANIFEST.json");
    let required_pdfs = required_certificate_pdf_paths(&manifest_path, &hashes)?;
    if required_pdfs.is_empty() {
        return Err(AppError::Validation(
            "A newly generated certificate must use the PDF certificate format.".into(),
        ));
    }
    for pdf_path in required_pdfs {
        let bytes = verified_pdfs.get(pdf_path).ok_or_else(|| {
            AppError::Validation(format!(
                "Staged certificate PDF hash entry was not verified: {pdf_path}"
            ))
        })?;
        certificate_pdf::validate_pdf_bytes(bytes)?;
    }
    Ok(())
}

fn parse_certificate_hashes(content: &str) -> Result<BTreeMap<String, String>> {
    let legacy_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE];
    let single_pdf_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE, PDF_FILE];
    let dual_pdf_paths = [
        HASH_FILE,
        MANIFEST_FILE,
        CERTIFICATE_FILE,
        PDF_FILE,
        PDF_FILE_DE,
    ];
    let expected_paths = [
        HASH_FILE,
        MANIFEST_FILE,
        CERTIFICATE_FILE,
        PDF_FILE,
        PDF_FILE_DE,
    ];
    let mut result = BTreeMap::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.is_empty() {
            return Err(AppError::Data(format!(
                "Empty certificate hash line {}.",
                line_number + 1
            )));
        }
        let (digest, relative) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!(
                "Invalid certificate hash line {}.",
                line_number + 1
            ))
        })?;
        validate_digest(digest, line_number + 1)?;
        if !expected_paths.contains(&relative) {
            return Err(AppError::Data(format!(
                "Unexpected certificate hash path on line {}.",
                line_number + 1
            )));
        }
        if result
            .insert(relative.to_owned(), digest.to_ascii_lowercase())
            .is_some()
        {
            return Err(AppError::Data(format!(
                "Duplicate certificate hash path: {relative}"
            )));
        }
    }
    let is_legacy_set = result.len() == legacy_paths.len()
        && legacy_paths.iter().all(|path| result.contains_key(*path));
    let is_single_pdf_set = result.len() == single_pdf_paths.len()
        && single_pdf_paths
            .iter()
            .all(|path| result.contains_key(*path));
    let is_dual_pdf_set = result.len() == dual_pdf_paths.len()
        && dual_pdf_paths.iter().all(|path| result.contains_key(*path));
    if !is_legacy_set && !is_single_pdf_set && !is_dual_pdf_set {
        return Err(AppError::Validation(
            "Certificate hash set is incomplete.".into(),
        ));
    }
    Ok(result)
}

fn evidence_register_markdown(evidence: &[&EvidenceItem]) -> String {
    evidence
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "### {}. {}\n\n- Evidence ID [System value]: `{}`\n- Original filename [Evidence-derived metadata]: `{}`\n- Managed filename [System value]: `{}`\n- Role [System value]: `{}`\n- Provenance [System value]: `{}`\n- Relative path [System value]: `{}`\n- Size [System value]: {} bytes\n- SHA-256 [System verification]: `{}`\n- Imported at [System value]: `{}`\n- Document title [User-confirmed fact]: {}\n- Provider/source [User-confirmed fact]: {}\n- Source URL [User-confirmed fact]: {}\n- Retrieval date [User-confirmed fact]: {}\n- Effective date [User-confirmed fact]: {}\n- Applicable production period [User-confirmed fact]: {}\n- Factual note [User-confirmed fact]: {}\n- Source global evidence ID [System value]: `{}`\n- Derived from evidence ID [System value]: `{}`\n- Generator version [System value]: `{}`\n- Generated disclosure text [System value]: {}\n\n",
                index + 1,
                item.role.as_str(),
                item.id,
                documented(&item.metadata.original_file_name),
                item.file_name,
                item.role.as_str(),
                item.provenance.as_str(),
                item.relative_path,
                item.size_bytes,
                item.sha256.as_deref().unwrap_or("NOT RECORDED"),
                item.imported_at,
                documented(&item.metadata.document_title),
                documented(&item.metadata.provider),
                documented(&item.metadata.source_url),
                documented(&item.metadata.retrieval_date),
                documented(&item.metadata.effective_date),
                documented(&item.metadata.applicable_production_period),
                documented(&item.metadata.factual_note),
                item.source_global_evidence_id.as_deref().unwrap_or("N/A"),
                item.derived_from_evidence_id.as_deref().unwrap_or("N/A"),
                item.generator_version.as_deref().unwrap_or("N/A"),
                item.generated_disclosure_text
                    .as_deref()
                    .map(documented)
                    .unwrap_or("N/A"),
            )
        })
        .collect()
}

fn terms_evidence_markdown(terms: &[&EvidenceItem]) -> String {
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
                item.id,
                item.id,
                documented(&item.metadata.document_title),
                documented(&item.metadata.provider),
                documented(&item.metadata.source_url),
                documented(&item.metadata.retrieval_date),
                documented(&item.metadata.effective_date),
                documented(&item.metadata.applicable_production_period),
                documented(&item.metadata.factual_note),
                item.relative_path,
                documented(&item.metadata.original_file_name),
                item.sha256.as_deref().unwrap_or("NOT RECORDED"),
                item.imported_at,
                item.provenance.as_str(),
            )
        })
        .collect()
}

fn suno_plan_context_markdown(fields: &TrackFields) -> String {
    format!(
        "- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n",
        documented(&fields.suno_plan_at_generation),
        documented(&fields.legacy_suno_plan_at_creation),
    )
}

fn source_provenance_markdown(fields: &TrackFields, evidence: &[&EvidenceItem]) -> String {
    let fields = fields.normalized_conditionals();
    let source_code = evidence_path(evidence, EvidenceRole::SourceCodeFile);
    let code_audio = evidence_path(evidence, EvidenceRole::CodeGeneratedAudioFile);
    let mut output = format!(
        "- External audio uploaded [User-confirmed fact]: {}\n- External audio source [User-confirmed fact]: {}\n- External audio provenance statement [User-confirmed fact]: {}\n- Own audio uploaded [User-confirmed fact]: {}\n- Own audio source [User-confirmed fact]: {}\n- Own audio provenance statement [User-confirmed fact]: {}\n- Third-party samples uploaded [User-confirmed fact]: {}\n- Third-party sample source [User-confirmed fact]: {}\n- Third-party sample provenance statement [User-confirmed fact]: {}\n- Code-based generation [User-confirmed fact]: {}\n- Source-code evidence [System value]: `{}`\n- Code-generated audio evidence [System value]: `{}`\n- Code-audio post-processing [User-confirmed fact]: {}\n- Code-audio post-processing operations [User-confirmed fact]: {}\n",
        recorded_bool(fields.external_audio_uploaded),
        conditional_text(
            fields.external_audio_uploaded,
            &fields.external_audio_source
        ),
        conditional_text(
            fields.external_audio_uploaded,
            &fields.external_audio_ownership
        ),
        recorded_bool(fields.own_audio_uploaded),
        conditional_text(fields.own_audio_uploaded, &fields.own_audio_source),
        conditional_text(fields.own_audio_uploaded, &fields.own_audio_ownership),
        recorded_bool(fields.third_party_samples_uploaded),
        conditional_text(
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_source,
        ),
        conditional_text(
            fields.third_party_samples_uploaded,
            &fields.third_party_sample_ownership,
        ),
        recorded_bool(fields.code_based_generation),
        conditional_text(fields.code_based_generation, source_code),
        conditional_text(fields.code_based_generation, code_audio),
        conditional_answer(
            fields.code_based_generation,
            recorded_bool(fields.code_audio_post_processed),
        ),
        if fields.code_based_generation == Some(true)
            && fields.code_audio_post_processed == Some(true)
        {
            documented_string_list(&fields.code_audio_post_processing_operations)
        } else {
            "N/A".into()
        },
    );
    if fields.code_based_generation == Some(true) && fields.code_audio_post_processed == Some(true)
    {
        output.push_str(&format!(
            "- Other code-audio post-processing note [User-confirmed fact]: {}\n",
            documented(&fields.code_audio_post_processing_note)
        ));
    }
    output
}

fn evidence_path<'a>(evidence: &'a [&EvidenceItem], role: EvidenceRole) -> &'a str {
    evidence
        .iter()
        .copied()
        .find(|item| item.role == role)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT RECORDED")
}

fn human_contribution_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let mut output = format!(
        "- Human editing performed [User-confirmed fact]: {}\n- Confirmed human editing [User-confirmed fact]: {}\n- Desktop-PC editing after the Suno WAV [User-confirmed fact]: {}\n- Confirmed desktop-PC editing [User-confirmed fact]: {}\n",
        recorded_bool(fields.human_editing_performed),
        conditional_text(
            fields.human_editing_performed,
            &fields.human_editing_details
        ),
        recorded_bool(fields.post_export_editing_performed),
        conditional_text(
            fields.post_export_editing_performed,
            &fields.post_export_editing_details,
        ),
    );
    if fields.artwork_origin == "human" {
        output.push_str(&format!(
            "- Confirmed human artwork process [User-confirmed fact]: {}\n- Human artwork process notes [User-confirmed fact]: {}\n",
            documented_string_list(&fields.human_artwork_process_operations),
            documented(&fields.human_artwork_process_notes),
        ));
    } else if fields.artwork_origin == "ai_assisted" {
        output.push_str(&format!(
            "- Confirmed human artwork modifications [User-confirmed fact]: {}\n- Other human artwork change [User-confirmed fact]: {}\n",
            documented_string_list(&fields.human_artwork_modifications),
            documented(&fields.custom_artwork_change),
        ));
    }
    output
}

fn suno_field_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let mut output = format!(
        "## F. Suno Generation Text Field\n\n- Suno Instrumental Mode Selected [User-confirmed fact]: {}\n- Generation Text Field Available [User-confirmed fact]: {}\n- Generation Text Field Used [User-confirmed fact]: {}\n- Content Classification [User-confirmed fact]: {}\n- Vocal Lyrics Present [User-confirmed fact]: {}\n- Structure Instructions Present [User-confirmed fact]: {}\n- Vocal Intent [User-confirmed fact]: {}\n- Final Audio Contains Vocals [User-confirmed fact]: {}\n",
        recorded_bool(fields.instrumental_track),
        recorded_bool(fields.suno_lyrics_field_content),
        generation_text_field_used(&fields),
        content_classification(&fields),
        generation_field_vocal_lyrics_present(&fields),
        structure_instructions_present(&fields),
        suno_vocal_intent(&fields),
        final_audio_contains_vocals(&fields),
    );
    output.push_str("\n### Exact Generation Text Field Content\n\n");
    output.push_str(match fields.suno_lyrics_field_content {
        Some(true) => documented(&fields.suno_lyrics_field_text),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    });
    output.push_str("\n\n");
    if !fields.lyrics_source.trim().is_empty() || !fields.lyrics_text.trim().is_empty() {
        output.push_str(
            "### F.1 Unclassified legacy lyrics data\n\n- Classification: **NOT DOCUMENTED**\n",
        );
        output.push_str(&format!(
            "- Legacy source value: {}\n- Legacy text: {}\n\nThis retained historical data is unclassified legacy data and is not a Vocal Lyrics claim.\n\n",
            documented(&fields.lyrics_source),
            documented(&fields.lyrics_text),
        ));
    }
    output
}

fn ai_audio_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let active = fields.generative_ai_used;
    format!(
        "- Generative AI used [User-confirmed fact]: {}\n- AI system [User-confirmed fact]: {}\n- AI-assisted audio elements [User-confirmed fact]: {}\n- AI-generated audio elements [User-confirmed fact]: {}\n- Real person voice intentionally imitated [User-confirmed fact]: {}\n- Real person's identity intentionally represented [User-confirmed fact]: {}\n- Real event represented as authentic recording [User-confirmed fact]: {}\n- Real location / institution / event presented as authentic AI recording [User-confirmed fact]: {}\n- Disclosure applied [User-confirmed fact]: {}\n- Disclosure locations [User-confirmed fact]: {}\n- Disclosure text [User-confirmed fact]: {}\n- Disclosure reason / note [User-confirmed fact]: {}\n- Deepfake-related indicator summary: {}\n- Suno style prompt [User-confirmed fact]: {}\n\nNo AI Act compliance, legal necessity, or legal safety determination is made.\n",
        recorded_bool(active),
        conditional_text(active, &fields.audio_ai_system),
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
                documented_string_list(&fields.audio_disclosure_locations)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                documented(&fields.audio_disclosure_text)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::No)) => {
                documented(&fields.audio_disclosure_reason)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        deepfake_indicator_summary(&fields),
        documented(&fields.suno_style_prompt),
    )
}

fn ai_artwork_markdown(fields: &TrackFields) -> String {
    let fields = fields.normalized_conditionals();
    let artwork_present = !matches!(fields.artwork_origin.as_str(), "" | "none");
    let ai_artwork = matches!(
        fields.artwork_origin.as_str(),
        "ai_generated" | "ai_assisted"
    );
    format!(
        "- Artwork origin [User-confirmed fact]: {}\n- AI image service [User-confirmed fact]: {}\n- Human artwork process [User-confirmed fact]: {}\n- Human artwork modifications [User-confirmed fact]: {}\n- Depicts real person [User-confirmed fact]: {}\n- Real-person note [User-confirmed fact]: {}\n- Depicts real event [User-confirmed fact]: {}\n- Real-event note [User-confirmed fact]: {}\n- Trademark/logo [User-confirmed fact]: {}\n- Trademark/logo note [User-confirmed fact]: {}\n- Artwork disclosure applied [User-confirmed fact]: {}\n- Artwork disclosure text [User-confirmed fact]: {}\n",
        documented(&fields.artwork_origin),
        if ai_artwork {
            documented(&fields.ai_image_service)
        } else {
            "N/A"
        },
        if fields.artwork_origin == "human" {
            documented_string_list(&fields.human_artwork_process_operations)
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
            documented_string_list(&fields.human_artwork_modifications)
        } else {
            "N/A".into()
        },
        if artwork_present {
            recorded_bool(fields.depicts_real_person)
        } else {
            "N/A"
        },
        if artwork_present {
            applicable_note(fields.depicts_real_person, &fields.real_person_notes)
        } else {
            "N/A"
        },
        if artwork_present {
            recorded_bool(fields.depicts_real_event)
        } else {
            "N/A"
        },
        if artwork_present {
            applicable_note(fields.depicts_real_event, &fields.real_event_notes)
        } else {
            "N/A"
        },
        if artwork_present {
            recorded_bool(fields.contains_trademark)
        } else {
            "N/A"
        },
        if artwork_present {
            applicable_note(fields.contains_trademark, &fields.trademark_notes)
        } else {
            "N/A"
        },
        if ai_artwork {
            recorded_bool(fields.disclosure_applied)
        } else {
            "N/A"
        },
        if ai_artwork && fields.disclosure_applied == Some(true) {
            documented(&fields.disclosure_text)
        } else {
            "N/A"
        },
    )
}

fn suno_content_types(fields: &TrackFields) -> String {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A".into(),
        None => "NOT DOCUMENTED".into(),
        Some(true) if fields.suno_lyrics_content_types.is_empty() => "NOT DOCUMENTED".into(),
        Some(true) => fields
            .suno_lyrics_content_types
            .iter()
            .map(|value| match value {
                SunoLyricsContentType::VocalLyrics => "Vocal lyrics",
                SunoLyricsContentType::StructureInstructions => "Structure instructions",
                SunoLyricsContentType::SoundInstructions => "Sound instructions",
                SunoLyricsContentType::ArrangementInstructions => "Arrangement instructions",
                SunoLyricsContentType::Mixed => "Mixed",
                SunoLyricsContentType::Other => "Other",
            })
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn generation_text_field_used(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(true) if fields.suno_lyrics_field_text.trim().is_empty() => "NOT DOCUMENTED",
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT DOCUMENTED",
    }
}

fn content_classification(fields: &TrackFields) -> String {
    suno_content_types(fields)
}

fn generation_field_vocal_lyrics_present(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
        Some(true) if fields.suno_lyrics_content_types.is_empty() => "NOT DOCUMENTED",
        Some(true) => yes_no(
            fields
                .suno_lyrics_content_types
                .contains(&SunoLyricsContentType::VocalLyrics),
        ),
    }
}

fn suno_vocal_intent(fields: &TrackFields) -> &'static str {
    generation_field_vocal_lyrics_present(fields)
}

fn final_audio_contains_vocals(fields: &TrackFields) -> &'static str {
    recorded_bool(fields.vocal_lyrics_present)
}

fn structure_instructions_present(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
        Some(true) if fields.suno_lyrics_content_types.is_empty() => "NOT DOCUMENTED",
        Some(true) => yes_no(
            fields
                .suno_lyrics_content_types
                .contains(&SunoLyricsContentType::StructureInstructions),
        ),
    }
}

fn suno_content_source(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
        Some(true) => match fields.suno_lyrics_content_source {
            Some(SunoLyricsContentSource::Human) => "human",
            Some(SunoLyricsContentSource::Ai) => "AI",
            Some(SunoLyricsContentSource::Mixed) => "mixed",
            None => "NOT DOCUMENTED",
        },
    }
}

fn documentation_answer(value: Option<DocumentationAnswer>) -> &'static str {
    match value {
        Some(DocumentationAnswer::Yes) => "YES",
        Some(DocumentationAnswer::No) => "NO",
        Some(DocumentationAnswer::NotDocumented) | None => "NOT DOCUMENTED",
    }
}

fn conditional_documentation_answer(
    controlling: Option<bool>,
    value: Option<DocumentationAnswer>,
) -> &'static str {
    match controlling {
        Some(true) => documentation_answer(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn deepfake_indicator_summary(fields: &TrackFields) -> &'static str {
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
    if answers
        .iter()
        .any(|answer| *answer == Some(DocumentationAnswer::Yes))
    {
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

fn documented_string_list(values: &[String]) -> String {
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

fn conditional_text<'a>(controlling: Option<bool>, value: &'a str) -> &'a str {
    match controlling {
        Some(true) => documented(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn conditional_answer<'a>(controlling: Option<bool>, value: &'a str) -> &'a str {
    match controlling {
        Some(true) => value,
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn applicable_note<'a>(answer: Option<bool>, value: &'a str) -> &'a str {
    match answer {
        Some(true) => documented(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn step_status_label(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::NotRun => "NOT_RUN",
        StepStatus::Pass => "PASS",
        StepStatus::Fail => "FAIL",
        StepStatus::Blocked => "BLOCKED",
        StepStatus::NotApplicable => "N/A",
        StepStatus::NotVerified => "NOT_VERIFIED",
    }
}

fn documented(value: &str) -> &str {
    if value.trim().is_empty() {
        "NOT DOCUMENTED"
    } else {
        value
    }
}

fn fact_origin_label(origin: FactOrigin) -> &'static str {
    match origin {
        FactOrigin::UserConfirmedFact => "User-confirmed fact",
        FactOrigin::EvidenceDerivedMetadata => "Evidence-derived metadata",
        FactOrigin::NotDocumented => "NOT DOCUMENTED",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

fn recorded_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT DOCUMENTED",
    }
}

fn certificate_format_requires_pdf(
    manifest_path: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<bool> {
    Ok(!required_certificate_pdf_paths(manifest_path, hashes)?.is_empty())
}

fn required_certificate_pdf_paths(
    manifest_path: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<&'static [&'static str]> {
    let bytes = fs::read(manifest_path).map_err(|error| AppError::io(manifest_path, error))?;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)?;
    let format_version = manifest
        .get("certificate")
        .and_then(|certificate| certificate.get("format_version"))
        .and_then(serde_json::Value::as_str);
    match format_version {
        Some(CERTIFICATE_FORMAT_VERSION) => {
            if !hashes.contains_key(PDF_FILE) || !hashes.contains_key(PDF_FILE_DE) {
                return Err(AppError::Validation(
                    "Certificate format 5.2 requires German and English PDF hash entries.".into(),
                ));
            }
            Ok(&[PDF_FILE, PDF_FILE_DE])
        }
        Some("5.1" | "5.0" | "4.1" | "4.0" | "3.0" | "2.0") => {
            if !hashes.contains_key(PDF_FILE) {
                return Err(AppError::Validation(
                    "This certificate format requires the root-level technical PDF hash.".into(),
                ));
            }
            Ok(&[PDF_FILE])
        }
        Some(version) => Err(AppError::Validation(format!(
            "Unsupported certificate format version: {version}"
        ))),
        None => {
            if is_certificate_pdf_path_in_hashes(hashes) {
                return Err(AppError::Validation(
                    "A legacy certificate cannot contain an unversioned PDF hash entry.".into(),
                ));
            }
            Ok(&[])
        }
    }
}

fn is_certificate_pdf_path(relative: &str) -> bool {
    matches!(relative, PDF_FILE | PDF_FILE_DE)
}

fn is_certificate_pdf_path_in_hashes(hashes: &BTreeMap<String, String>) -> bool {
    hashes.keys().any(|path| is_certificate_pdf_path(path))
}

fn parse_hashes(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    let mut result = BTreeMap::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.is_empty() {
            return Err(AppError::Data(format!(
                "Empty SHA256SUMS line {}.",
                line_number + 1
            )));
        }
        let (hash, relative) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!("Invalid SHA256SUMS line {}.", line_number + 1))
        })?;
        validate_digest(hash, line_number + 1)?;
        let relative_path = Path::new(relative);
        crate::security::validate_relative(relative_path)?;
        if relative.contains('\\') || relative.chars().any(char::is_control) {
            return Err(AppError::Data(format!(
                "Invalid SHA256SUMS path on line {}.",
                line_number + 1
            )));
        }
        let portable = portable_relative(relative_path);
        if !hash_manifest_path_allowed(relative_path) {
            return Err(AppError::Data(format!(
                "Excluded SHA256SUMS path on line {}.",
                line_number + 1
            )));
        }
        if result
            .insert(portable.clone(), hash.to_ascii_lowercase())
            .is_some()
        {
            return Err(AppError::Data(format!(
                "Duplicate SHA256SUMS path: {portable}"
            )));
        }
    }
    if result.is_empty() {
        return Err(AppError::Validation("SHA256SUMS is empty.".into()));
    }
    Ok(result)
}

fn validate_digest(digest: &str, line_number: usize) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Data(format!(
            "Invalid SHA-256 digest on line {line_number}."
        )));
    }
    Ok(())
}

fn hash_manifest_path_allowed(relative: &Path) -> bool {
    if relative == Path::new(HASH_FILE)
        || relative == Path::new(PDF_FILE)
        || relative == Path::new(PDF_FILE_DE)
    {
        return false;
    }
    !matches!(
        relative.components().next(),
        Some(std::path::Component::Normal(value))
            if value == ".archive"
                || value == ".summary"
                || value == ".suno-doc"
                || value == CERTIFICATE_DIR
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn relationship_evidence(id: &str, role: EvidenceRole) -> EvidenceItem {
        EvidenceItem {
            id: id.into(),
            role,
            file_name: format!("{id}.dat"),
            relative_path: format!("03_DOCUMENTATION/{id}.dat"),
            sha256: Some(DIGEST.into()),
            size_bytes: 1,
            imported_at: "2026-08-17T10:00:00Z".into(),
            verified: true,
            verification_error: None,
            source_global_evidence_id: None,
            coverage_start: None,
            coverage_end: None,
            provenance: EvidenceProvenance::ManagedCopy,
            derived_from_evidence_id: None,
            generator_version: None,
            generated_disclosure_text: None,
            metadata: EvidenceMetadata::default(),
        }
    }

    fn parse_main_hash_fixture(content: &str) -> Result<BTreeMap<String, String>> {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let sums = workspace.path().join("SHA256SUMS.txt");
        fs::write(&sums, content).expect("write SHA256SUMS fixture");
        parse_hashes(&sums)
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
            .push(crate::model::AudioScreeningMatch {
                title: "Provider title".into(),
                artists: vec!["Provider artist".into()],
                ..Default::default()
            });

        let manifest = audio_screening_manifest(&state).to_string();
        let markdown = audio_screening_markdown(&state);
        for forbidden in [
            "RAW_CHROMAPRINT_MUST_NOT_APPEAR",
            "ACCESS_SECRET_MUST_NOT_APPEAR",
            "RAW_PROVIDER_RESPONSE_MUST_NOT_APPEAR",
        ] {
            assert!(!manifest.contains(forbidden), "manifest leaked {forbidden}");
            assert!(!markdown.contains(forbidden), "markdown leaked {forbidden}");
        }
        assert!(manifest.contains("Provider title"));
        assert!(markdown.contains("Provider artist"));
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
        assert!(
            german.contains("- ID der finalen Erzeugung [Aus Evidenzmetadaten]: `generation-id`")
        );
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
    fn suno_field_markdown_is_separate_from_human_contribution_for_ai_and_mixed_sources() {
        for source in [SunoLyricsContentSource::Ai, SunoLyricsContentSource::Mixed] {
            let mut fields = TrackFields::default();
            fields.human_editing_performed = Some(false);
            fields.suno_lyrics_field_content = Some(true);
            fields.suno_lyrics_content_types = vec![SunoLyricsContentType::StructureInstructions];
            fields.suno_lyrics_content_source = Some(source);
            fields.suno_lyrics_field_text = "[AI-or-mixed structure instruction]".into();

            let human = human_contribution_markdown(&fields);
            let suno_field = suno_field_markdown(&fields);

            assert!(!human.contains("Content source [User-confirmed fact]"));
            assert!(!human.contains("[AI-or-mixed structure instruction]"));
            assert!(suno_field.starts_with("## F. Suno Generation Text Field\n"));
            assert!(suno_field.contains("- Generation Text Field Used [User-confirmed fact]: YES"));
            assert!(suno_field.contains("- Vocal Intent [User-confirmed fact]: NO"));
            assert!(suno_field.contains("[AI-or-mixed structure instruction]"));
            assert!(!suno_field.starts_with("## E."));
        }
    }

    #[test]
    fn structure_only_generation_text_does_not_become_vocal_lyrics_or_override_audio_result() {
        let mut fields = TrackFields::default();
        fields.instrumental_track = Some(false);
        fields.vocal_lyrics_present = Some(true);
        fields.suno_lyrics_field_content = Some(true);
        fields.suno_lyrics_content_types = vec![SunoLyricsContentType::StructureInstructions];
        fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
        fields.suno_lyrics_field_text = "[Intro]\n[Drop]".into();

        let markdown = suno_field_markdown(&fields);
        assert!(markdown.contains("- Vocal Lyrics Present [User-confirmed fact]: NO"));
        assert!(markdown.contains("- Structure Instructions Present [User-confirmed fact]: YES"));
        assert!(markdown.contains("- Final Audio Contains Vocals [User-confirmed fact]: YES"));
    }

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

        let markdown = suno_plan_context_markdown(&fields);
        assert!(markdown.contains("Suno plan at generation [User-confirmed fact]: NOT DOCUMENTED"));
        assert!(markdown.contains(
            "Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: Pro"
        ));
        assert!(!markdown.contains("Suno plan at generation [User-confirmed fact]: Pro"));
    }

    #[test]
    fn source_and_human_detail_markdown_labels_user_facts_and_system_paths() {
        let mut fields = TrackFields::default();
        fields.external_audio_uploaded = Some(true);
        fields.external_audio_source = "External recorder".into();
        fields.external_audio_ownership = "User-owned recording".into();
        fields.own_audio_uploaded = Some(true);
        fields.own_audio_source = "Own stem".into();
        fields.own_audio_ownership = "Created by user".into();
        fields.third_party_samples_uploaded = Some(true);
        fields.third_party_sample_source = "Sample archive".into();
        fields.third_party_sample_ownership = "Licensed sample".into();
        fields.code_based_generation = Some(true);
        fields.code_audio_post_processed = Some(true);
        fields.code_audio_post_processing_operations = vec!["Normalize".into()];
        fields.code_audio_post_processing_note = "Manual limiter".into();
        fields.human_editing_performed = Some(true);
        fields.human_editing_details = "Manual timing edit".into();
        fields.post_export_editing_performed = Some(true);
        fields.post_export_editing_details = "Manual mastering".into();
        fields.artwork_origin = "ai_assisted".into();
        fields.human_artwork_modifications = vec!["Crop".into()];
        fields.custom_artwork_change = "Manual typography".into();

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

    #[test]
    fn automatic_relationships_cover_only_unambiguous_adjacent_role_pairs() {
        let source = relationship_evidence("source", EvidenceRole::SourceCodeFile);
        let generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
        let artwork_original =
            relationship_evidence("artwork-original", EvidenceRole::AiArtworkOriginal);
        let artwork_ai_edited =
            relationship_evidence("artwork-ai-edited", EvidenceRole::AiArtworkEdited);
        let artwork_human_edited =
            relationship_evidence("artwork-human-edited", EvidenceRole::HumanEditedArtwork);
        let artwork_final = relationship_evidence("artwork-final", EvidenceRole::FinalArtwork);
        let mut global = relationship_evidence("terms-copy", EvidenceRole::SunoTermsRights);
        global.provenance = EvidenceProvenance::GlobalCopy;
        global.source_global_evidence_id = Some("global-terms".into());
        let evidence = [
            &source,
            &generated,
            &artwork_original,
            &artwork_ai_edited,
            &artwork_human_edited,
            &artwork_final,
            &global,
        ];

        let relationships = automatic_role_relationships(&evidence);

        assert_eq!(relationships.len(), 4);
        assert!(relationships.iter().any(|relationship| {
            relationship.kind == "source_to_generated_audio"
                && relationship.source_evidence_id == "source"
                && relationship.target_evidence_id == "generated"
        }));
        assert!(relationships.iter().any(|relationship| {
            relationship.kind == "artwork_stage"
                && relationship.source_evidence_id == "artwork-original"
                && relationship.target_evidence_id == "artwork-ai-edited"
        }));
        assert!(relationships.iter().any(|relationship| {
            relationship.kind == "artwork_stage"
                && relationship.source_evidence_id == "artwork-ai-edited"
                && relationship.target_evidence_id == "artwork-human-edited"
        }));
        assert!(relationships.iter().any(|relationship| {
            relationship.kind == "artwork_stage"
                && relationship.source_evidence_id == "artwork-human-edited"
                && relationship.target_evidence_id == "artwork-final"
        }));
        assert!(relationships
            .iter()
            .all(|relationship| relationship.kind != "global_copy"));

        let global_relationships = automatic_global_track_relationships("track-1", &evidence);
        assert_eq!(
            global_relationships,
            vec![AutomaticGlobalTrackRelationship {
                kind: "global_evidence_to_track",
                source_global_evidence_id: "global-terms".into(),
                materialized_evidence_id: "terms-copy".into(),
                role: EvidenceRole::SunoTermsRights.as_str(),
                target_track_id: "track-1".into(),
            }]
        );
    }

    #[test]
    fn explicit_lineage_disambiguates_multiple_sources_without_cartesian_products() {
        let source_one = relationship_evidence("source-one", EvidenceRole::SourceCodeFile);
        let source_two = relationship_evidence("source-two", EvidenceRole::SourceCodeFile);
        let mut generated =
            relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
        generated.derived_from_evidence_id = Some(source_two.id.clone());
        let artwork_one = relationship_evidence("artwork-one", EvidenceRole::AiArtworkOriginal);
        let artwork_two = relationship_evidence("artwork-two", EvidenceRole::AiArtworkOriginal);
        let mut artwork_edited =
            relationship_evidence("artwork-edited", EvidenceRole::AiArtworkEdited);
        artwork_edited.derived_from_evidence_id = Some(artwork_one.id.clone());
        let evidence = [
            &source_one,
            &source_two,
            &generated,
            &artwork_one,
            &artwork_two,
            &artwork_edited,
        ];

        let relationships = automatic_role_relationships(&evidence);

        assert_eq!(relationships.len(), 2);
        assert!(relationships.iter().any(|relationship| {
            relationship.source_evidence_id == "source-two"
                && relationship.target_evidence_id == "generated"
        }));
        assert!(relationships.iter().any(|relationship| {
            relationship.source_evidence_id == "artwork-one"
                && relationship.target_evidence_id == "artwork-edited"
        }));
        assert!(relationships.iter().all(|relationship| {
            relationship.source_evidence_id != "source-one"
                && relationship.source_evidence_id != "artwork-two"
        }));
    }

    #[test]
    fn ambiguous_roles_without_explicit_lineage_emit_no_id_relationship() {
        let source_one = relationship_evidence("source-one", EvidenceRole::SourceCodeFile);
        let source_two = relationship_evidence("source-two", EvidenceRole::SourceCodeFile);
        let generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
        let artwork_one = relationship_evidence("artwork-one", EvidenceRole::AiArtworkOriginal);
        let artwork_two = relationship_evidence("artwork-two", EvidenceRole::AiArtworkOriginal);
        let artwork_edited = relationship_evidence("artwork-edited", EvidenceRole::AiArtworkEdited);
        let evidence = [
            &source_one,
            &source_two,
            &generated,
            &artwork_one,
            &artwork_two,
            &artwork_edited,
        ];

        assert!(automatic_role_relationships(&evidence).is_empty());
    }

    #[test]
    fn artwork_relationships_never_skip_a_concrete_stage() {
        let original = relationship_evidence("original", EvidenceRole::AiArtworkOriginal);
        let human_edited = relationship_evidence("human-edited", EvidenceRole::HumanEditedArtwork);
        let final_artwork = relationship_evidence("final", EvidenceRole::FinalArtwork);
        let evidence = [&original, &human_edited, &final_artwork];

        let relationships = automatic_role_relationships(&evidence);

        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].source_evidence_id, "human-edited");
        assert_eq!(relationships[0].target_evidence_id, "final");
    }

    #[test]
    fn invalid_or_non_adjacent_explicit_lineage_is_not_replaced_by_role_inference() {
        let source = relationship_evidence("source", EvidenceRole::SourceCodeFile);
        let mut generated =
            relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
        generated.derived_from_evidence_id = Some("missing-source".into());
        let original = relationship_evidence("original", EvidenceRole::AiArtworkOriginal);
        let human_edited = relationship_evidence("human-edited", EvidenceRole::HumanEditedArtwork);
        let mut final_artwork = relationship_evidence("final", EvidenceRole::FinalArtwork);
        final_artwork.derived_from_evidence_id = Some(original.id.clone());
        let evidence = [
            &source,
            &generated,
            &original,
            &human_edited,
            &final_artwork,
        ];

        assert!(automatic_role_relationships(&evidence).is_empty());
    }

    #[test]
    fn current_and_previous_pdf_certificate_formats_remain_recognizable() {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let manifest_path = workspace.path().join("manifest.json");

        for version in ["2.0", "3.0", "4.0", "4.1", "5.0", "5.1"] {
            let hashes = BTreeMap::from([(PDF_FILE.into(), DIGEST.into())]);
            fs::write(
                &manifest_path,
                format!("{{\"certificate\":{{\"format_version\":\"{version}\"}}}}\n"),
            )
            .expect("certificate manifest fixture");
            assert!(certificate_format_requires_pdf(&manifest_path, &hashes)
                .expect("supported certificate format"));
        }
        fs::write(
            &manifest_path,
            format!(
                "{{\"certificate\":{{\"format_version\":\"{CERTIFICATE_FORMAT_VERSION}\"}}}}\n"
            ),
        )
        .expect("current certificate manifest fixture");
        let hashes = BTreeMap::from([
            (PDF_FILE.into(), DIGEST.into()),
            (PDF_FILE_DE.into(), DIGEST.into()),
        ]);
        assert!(certificate_format_requires_pdf(&manifest_path, &hashes)
            .expect("current supported certificate format"));
    }

    fn publication_fixture(track_root: &Path) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let main_hashes = b"fixture main hash manifest\n";
        let main_hash_path = track_root.join(HASH_FILE);
        fs::create_dir_all(
            main_hash_path
                .parent()
                .expect("main hash manifest parent directory"),
        )
        .expect("create documentation fixture directory");
        fs::write(&main_hash_path, main_hashes).expect("write main hash manifest fixture");
        let manifest = format!(
            "{{\"certificate\":{{\"format_version\":\"{CERTIFICATE_FORMAT_VERSION}\"}},\"fixture\":true}}\n"
        )
        .into_bytes();
        let certificate = b"# Fixture certificate\n".to_vec();
        let mut pdf_document = printpdf::PdfDocument::new("Fixture certificate");
        let pdf = pdf_document
            .with_pages(vec![printpdf::PdfPage::new(
                printpdf::Mm(210.0),
                printpdf::Mm(297.0),
                Vec::new(),
            )])
            .save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
        let certificate_hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
            sha256_bytes(main_hashes),
            HASH_FILE,
            sha256_bytes(&manifest),
            MANIFEST_FILE,
            sha256_bytes(&certificate),
            CERTIFICATE_FILE,
            sha256_bytes(&pdf),
            PDF_FILE,
            sha256_bytes(&pdf),
            PDF_FILE_DE,
        )
        .into_bytes();
        (manifest, certificate, pdf.clone(), pdf, certificate_hashes)
    }

    fn assert_injected_publication_failure(failure: CertificatePublicationFailure) {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let track_root = workspace.path();
        let (manifest, certificate, pdf_en, pdf_de, certificate_hashes) =
            publication_fixture(track_root);
        let live = track_root.join(CERTIFICATE_DIR);
        fs::create_dir(&live).expect("create empty live certificate directory");
        let correlated_stage = track_root
            .join(".archive")
            .join("certificate-staging")
            .join(failure.stage_id());

        let error = publish_certificate_set_impl(
            track_root,
            &manifest,
            &certificate,
            &pdf_en,
            &pdf_de,
            &certificate_hashes,
            &failure.stage_id(),
            Some(failure),
        )
        .expect_err("injected publication failure");

        assert_eq!(
            error.to_string(),
            format!(
                "Invalid stored data: Injected certificate publication failure at {}.",
                failure.label()
            )
        );
        assert!(
            verify(track_root).is_err(),
            "incomplete live certificate unexpectedly verified after {} failure",
            failure.label()
        );
        assert!(
            live.is_dir(),
            "empty live certificate directory was removed"
        );
        assert!(
            fs::read_dir(&live)
                .expect("read restored certificate directory")
                .next()
                .is_none(),
            "live certificate directory is not empty after {} failure",
            failure.label()
        );
        assert!(
            !track_root.join(PDF_FILE).exists(),
            "live PDF remains after {} failure",
            failure.label()
        );
        assert!(
            !track_root.join(PDF_FILE_DE).exists(),
            "German live PDF remains after {} failure",
            failure.label()
        );
        assert!(
            !correlated_stage.exists(),
            "correlated staging directory was not cleaned after {} failure",
            failure.label()
        );
        let staging_parent = track_root.join(".archive/certificate-staging");
        assert!(
            !staging_parent.exists()
                || fs::read_dir(&staging_parent)
                    .expect("read certificate staging directory")
                    .next()
                    .is_none(),
            "certificate staging contains residue after {} failure",
            failure.label()
        );
    }

    #[test]
    fn staging_directory_creation_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::StagingDirectoryCreate);
    }

    #[test]
    fn manifest_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::ManifestWrite);
    }

    #[test]
    fn certificate_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::CertificateWrite);
    }

    #[test]
    fn pdf_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::PdfWrite);
    }

    #[test]
    fn certificate_hash_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::CertificateHashWrite);
    }

    #[test]
    fn certificate_publish_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(
            CertificatePublicationFailure::CertificatePublishRename,
        );
    }

    #[test]
    fn pdf_publish_failure_rolls_back_certificate_and_cleans_staging() {
        assert_injected_publication_failure(CertificatePublicationFailure::PdfPublish);
    }

    #[test]
    fn post_publish_verification_failure_rolls_back_and_cleans_staging() {
        assert_injected_publication_failure(CertificatePublicationFailure::PostPublishVerification);
    }

    #[test]
    fn certificate_hash_parser_requires_exact_complete_unique_set() {
        let legacy = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&legacy)
                .expect("valid legacy set")
                .len(),
            3
        );

        let single_pdf = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&single_pdf)
                .expect("valid single-PDF set")
                .len(),
            4
        );

        let valid = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n{DIGEST}  {PDF_FILE_DE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&valid)
                .expect("valid dual-PDF set")
                .len(),
            5
        );

        let duplicate = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n{DIGEST}  {PDF_FILE_DE}\n{DIGEST}  {PDF_FILE_DE}\n"
        );
        assert!(parse_certificate_hashes(&duplicate).is_err());

        let invalid_digest = format!(
            "short  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
        );
        assert!(parse_certificate_hashes(&invalid_digest).is_err());
    }

    #[test]
    fn legacy_certificate_without_pdf_remains_verifiable() {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let track_root = workspace.path();
        let main_hash_path = track_root.join(HASH_FILE);
        fs::create_dir_all(main_hash_path.parent().expect("main hash parent"))
            .expect("documentation directory");
        fs::write(&main_hash_path, b"legacy main hash list\n").expect("main hash fixture");
        let certificate_path = track_root.join(CERTIFICATE_FILE);
        fs::create_dir_all(certificate_path.parent().expect("certificate parent"))
            .expect("certificate directory");
        let manifest_path = track_root.join(MANIFEST_FILE);
        fs::write(&manifest_path, b"{\"certificate\":{}}\n").expect("legacy manifest");
        fs::write(&certificate_path, b"# Legacy certificate\n").expect("legacy certificate");
        let hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n",
            sha256_file(&main_hash_path).expect("main hash digest"),
            HASH_FILE,
            sha256_file(&manifest_path).expect("manifest digest"),
            MANIFEST_FILE,
            sha256_file(&certificate_path).expect("certificate digest"),
            CERTIFICATE_FILE,
        );
        fs::write(track_root.join(CERTIFICATE_HASH_FILE), hashes).expect("legacy hash set");

        verify(track_root).expect("legacy certificate remains valid");
        assert!(!expects_pdf(track_root).expect("legacy format detection"));
    }

    #[test]
    fn evidence_manifest_hash_parser_rejects_duplicates_and_exclusions() {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let sums = workspace.path().join("SHA256SUMS.txt");
        fs::write(
            &sums,
            format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
        )
        .expect("write duplicate sums");
        assert!(parse_hashes(&sums).is_err());

        fs::write(&sums, format!("{DIGEST}  06_CERTIFICATE/hidden.txt\n"))
            .expect("write excluded sums");
        assert!(parse_hashes(&sums).is_err());

        fs::write(&sums, format!("{DIGEST}  {PDF_FILE}\n")).expect("write excluded root PDF");
        assert!(parse_hashes(&sums).is_err());
    }

    #[test]
    fn certificate_hash_parser_rejects_empty_missing_extra_and_unsafe_entries() {
        let invalid_sets = [
            String::new(),
            format!("{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n"),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {CERTIFICATE_HASH_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  ../EVIDENCE_MANIFEST.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  /absolute.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  06_CERTIFICATE/control\tmanifest.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
        ];

        for content in invalid_sets {
            assert!(
                parse_certificate_hashes(&content).is_err(),
                "invalid certificate set was accepted: {content:?}"
            );
        }
    }

    #[test]
    fn evidence_manifest_hash_parser_covers_format_and_path_edge_cases() {
        let invalid_entries = [
            String::new(),
            "not-a-digest  01_RELEASE/song.wav\n".into(),
            format!("{DIGEST}  /absolute.wav\n"),
            format!("{DIGEST}  ../escape.wav\n"),
            format!("{DIGEST}  01_RELEASE\\windows.wav\n"),
            format!("{DIGEST}  01_RELEASE/control\tname.wav\n"),
            format!("{DIGEST}  {HASH_FILE}\n"),
            format!("{DIGEST}  .archive/hidden.wav\n"),
            format!("{DIGEST}  .summary/hidden.wav\n"),
            format!("{DIGEST}  .suno-doc/workspace.sqlite\n"),
            format!("{DIGEST}  {CERTIFICATE_FILE}\n"),
            format!("{DIGEST}  01_RELEASE/song.wav\n\n{DIGEST}  02_SUNO/song.wav\n"),
            format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
        ];

        for content in invalid_entries {
            assert!(
                parse_main_hash_fixture(&content).is_err(),
                "invalid main hash entry was accepted: {content:?}"
            );
        }

        let uppercase = DIGEST.to_ascii_uppercase();
        let parsed = parse_main_hash_fixture(&format!(
            "{uppercase}  01_RELEASE/song.wav\n{DIGEST}  02_SUNO/source.wav\n"
        ))
        .expect("portable valid hash list");
        assert_eq!(parsed["01_RELEASE/song.wav"], DIGEST);
        assert_eq!(parsed.len(), 2);
    }
}
