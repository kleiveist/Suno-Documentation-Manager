import type { SystemTranslation } from "./catalog-types";

export const SYSTEM_TRANSLATIONS_04: readonly SystemTranslation[] = [
  ["Eine nicht negative INTEGER wurde erwartet.", "Expected a non-negative INTEGER."],
  ["Der Status der Zeitstempelantwort ist zu groß.", "Timestamp response status is too large."],
  ["Die RFC-3161-Antwort enthält keine Statusinformationen.", "RFC 3161 response has no status information."],
  ["Die RFC-3161-Statusinformationen sind ungültig.", "RFC 3161 status information is invalid."],
  ["Der RFC-3161-Antwortstatus fehlt.", "RFC 3161 response status is missing."],
  ["Das RFC-3161-Timestamp-Token ist nicht CMS SignedData.", "RFC 3161 timestamp token is not CMS SignedData."],
  ["Der RFC-3161-CMS-SignedData-Wrapper fehlt.", "RFC 3161 CMS SignedData wrapper is missing."],
  ["Der eingekapselte Inhalt von RFC-3161-CMS fehlt.", "RFC 3161 CMS encapsulated content is missing."],
  ["Der eingekapselte Inhalt von RFC-3161-CMS ist ungültig.", "RFC 3161 CMS encapsulated content is invalid."],
  ["Die RFC-3161-CMS-Nutzlast ist nicht TSTInfo.", "RFC 3161 CMS payload is not TSTInfo."],
  ["Die RFC-3161-TSTInfo-Nutzlast ist ungültig.", "RFC 3161 TSTInfo payload is invalid."],
  ["Die RFC-3161-TSTInfo-Struktur ist ungültig.", "RFC 3161 TSTInfo structure is invalid."],
  ["Die RFC-3161-TSTInfo-Version muss 1 sein.", "RFC 3161 TSTInfo version must be 1."],
  ["Der RFC-3161-Message-Imprint ist ungültig.", "RFC 3161 message imprint is invalid."],
  ["Der Algorithmus des RFC-3161-Message-Imprints fehlt.", "RFC 3161 message imprint algorithm is missing."],
  [
    "Die RFC-3161-Antwort verwendet keinen SHA-256-Message-Imprint.",
    "RFC 3161 response does not use SHA-256 message imprint."
  ],
  ["Die RFC-3161-Erstellungszeit fehlt.", "RFC 3161 generation time is missing."],
  ["RFC-3161-TSTInfo enthält mehr als eine Nonce.", "RFC 3161 TSTInfo contains more than one nonce."],
  ["Die RFC-3161-Erstellungszeit ist nicht UTF-8.", "RFC 3161 generation time is not UTF-8."],
  ["Die RFC-3161-Erstellungszeit muss UTC (Z) verwenden.", "RFC 3161 generation time must use UTC (Z)."],
  [
    "Andere Zeitstempelartefakte müssen ein unveränderter Eintrag in der verifizierten SHA256SUMS.txt der ersten Phase sein.",
    "Other timestamp artifacts must be an unchanged entry in the verified phase-one SHA256SUMS.txt file."
  ],
  ["Der Name des Roharchivs der Providerantwort fehlt.", "Raw provider response archive name is missing."],
  [
    "Die Revisionsmetadaten für den externen Zeitstempel {id} sind keine reguläre Datei.",
    "Revision metadata for external timestamp {id} is not a regular file."
  ],
  [
    "Die Zertifikats-ID in den Revisionsmetadaten stimmt nicht mit dem externen Zeitstempel {id} überein.",
    "Revision metadata certificate ID does not match external timestamp {id}."
  ],
  [
    "Die Veröffentlichung des externen Zeitstempels {id} befindet sich noch im Staging und benötigt eine Wiederherstellung.",
    "External timestamp publication {id} is still staged and requires recovery."
  ],
  ["Das Sidecar des externen Zeitstempels {id} fehlt.", "External timestamp sidecar {id} is missing."],
  [
    "Das Sidecar des externen Zeitstempels {id} existiert in mehreren Revisionsarchiven.",
    "External timestamp sidecar {id} exists in multiple revision archives."
  ],
  [
    "Der Name des Verzeichnisses für externe Zeitstempel ist nicht UTF-8.",
    "External timestamp directory name is not UTF-8."
  ],
  [
    "Unerwarteter Eintrag im Veröffentlichungsverzeichnis für externe Zeitstempel: {path}.",
    "Unexpected entry in the external timestamp publication directory: {path}."
  ],
  [
    "Nicht registriertes Zeitstempel-Sidecar erkannt: {id}. Es wurde nicht automatisch übernommen.",
    "Unregistered external timestamp sidecar detected: {id}. It was not adopted automatically."
  ],
  [
    "Der Name des Zeitstempel-Staging-Verzeichnisses ist nicht UTF-8.",
    "Timestamp staging directory name is not UTF-8."
  ],
  [
    "Unerwarteter Eintrag im Zeitstempel-Staging-Verzeichnis: {path}.",
    "Unexpected entry in the timestamp staging directory: {path}."
  ],
  [
    "Die Serialisierung des Zeitstempeldatensatzes ist kein Objekt.",
    "Timestamp record serialization is not an object."
  ],
  ["Die Erweiterung des Providerantwort-Archivs ist ungültig.", "Provider response archive extension is invalid."],
  [
    "Die Metadaten des Providerantwort-Archivs sind unvollständig oder ungültig.",
    "Provider response archive metadata is incomplete or invalid."
  ],
  [
    "Das Providerantwort-Archiv hat einen unerwarteten verwalteten Dateinamen.",
    "Provider response archive has an unexpected managed filename."
  ],
  ["Der Dateiname des Providerantwort-Archivs ist ungültig.", "Provider response archive filename is invalid."],
  [
    "Das archivierte referenzierte Artefakt des Zeitstempels ist keine reguläre Datei.",
    "The timestamp's archived referenced artifact is not a regular file."
  ],
  [
    "Das referenzierte Artefakt des Zeitstempels fehlt im Revisionsarchiv.",
    "The timestamp's referenced artifact is missing from its revision archive."
  ],
  [
    "Das andere Zeitstempelartefakt ist kein unveränderter SHA256SUMS-Eintrag der ersten Phase.",
    "The Other timestamp artifact is not an unchanged phase-one SHA256SUMS entry."
  ],
  [
    "Ungültiger finalisierter Zertifikatshash-Eintrag in Zeile {line}.",
    "Invalid finalized certificate hash entry on line {line}."
  ],
  [
    "Ungültiger finalisierter Zertifikatdigest in Zeile {line}.",
    "Invalid finalized certificate digest on line {line}."
  ],
  [
    "Der finalisierte Zertifikatshashsatz enthält das Evidence-Manifest mehr als einmal.",
    "Finalized certificate hash set contains the evidence manifest more than once."
  ],
  [
    "Der finalisierte Zertifikatshashsatz enthält kein EVIDENCE_MANIFEST.json.",
    "Finalized certificate hash set does not contain EVIDENCE_MANIFEST.json."
  ],
  [
    "INTEGRITÄTSPRÜFUNG FEHLGESCHLAGEN: Der ausgewählte Zeitstempel-Anchor stimmt nicht mehr mit dem finalisierten Snapshot überein.",
    "INTEGRITY CHECK FAILED: The selected timestamp anchor no longer matches the finalized snapshot."
  ],
  [
    "Andere Zeitstempelartefakte müssen eine stabile Track-Datei der ersten Phase identifizieren.",
    "Other timestamp artifacts must identify a stable phase-one track file."
  ],
  [
    "Der Zeitstempeldatensatz enthält einen inkonsistenten verwalteten Pfad.",
    "Timestamp record contains an inconsistent managed path."
  ],
  ["Zeitstempel-Provider / Aussteller", "Timestamp provider / issuer"],
  ["Anderes referenziertes Artefakt", "Other referenced artifact"],
  ["Zeitstempelnotiz", "Timestamp note"],
  // Presented workflow steps, states, and automatic-consistency notices
  ["Quelle", "Source"],
  ["Menschliche Arbeit", "Human work"],
  ["KI-Transparenz", "AI transparency"],
  ["Evidence & Lizenzen", "Evidence & licenses"],
  ["Integrität", "Integrity"],
  ["Finalisieren", "Finalize"],
  ["Titel und Produktionszeitraum", "Title and production period"],
  ["Audioquellen und Rechtezuordnung", "Audio sources and rights assignment"],
  ["Projekt, Modell und Erstellungstarif", "Project, model, and generation plan"],
  [
    "Vocal Lyrics, Suno-Feldinhalt und bestätigte Bearbeitungen",
    "Vocal lyrics, Suno field content, and confirmed edits"
  ],
  ["Entstehung und Content-Check", "Creation and content check"],
  ["Audio-Assessment und Artwork-Disclosure", "Audio assessment and artwork disclosure"],
  ["Letzte Bearbeitung und Release-Dateien", "Last edit and release files"],
  ["Nachweise vollständig zuordnen", "Assign all evidence completely"],
  ["Dokumente, SHA-256 und Verifikation", "Documents, SHA-256, and verification"],
  ["Gate prüfen und Zertifikat erzeugen", "Check gate and generate certificate"],
  ["Entwurf", "Draft"],
  ["In Arbeit", "In progress"],
  ["Bereit", "Ready"],
  ["Finalisiert", "Finalized"],
  ["Ersetzt", "Superseded"],
  ["Offen", "Open"],
  ["Erfüllt", "Passed"],
  ["Fehlgeschlagen", "Failed"],
  ["Blockiert", "Blocked"],
  ["BLOCKIERT", "BLOCKED"],
  ["N/A", "N/A"],
  ["Nicht verifiziert", "Not verified"],
  ["BLOCKIEREND", "BLOCKING"],
  ["WARNUNG", "WARNING"],
  ["HINWEIS", "INFO"],
  ["BESTANDEN MIT WARNUNGEN", "PASS WITH WARNINGS"],
  ["BESTANDEN", "PASS"],
  [
    "Ein externer Zeitstempelnachweis ist angehängt, seine Verifikation steht jedoch noch aus.",
    "External timestamp evidence is attached, but its verification is still pending."
  ],
  [
    "Der externe Zeitstempelnachweis meldet den Status {status}.",
    "External timestamp evidence reports status {status}."
  ],
  [
    "Für den finalisierten Snapshot ist kein externer Zeitstempelnachweis erfasst.",
    "No external timestamp evidence is recorded for the finalized snapshot."
  ],
  [
    "Im Suno-Final-Export wurden keine Suno-Studio-Metadaten erkannt.",
    "No Suno Studio metadata was detected in the final Suno export."
  ],
  [
    "Menschlich bearbeitetes und finales Artwork sind byte-identisch (SHA-256-Match).",
    "Human-edited and final artwork are byte-identical (SHA-256 match)."
  ],
  ["BYTE-IDENTISCH / SHA-256-ÜBEREINSTIMMUNG", "BYTE-IDENTICAL / SHA-256 MATCH"],
  ["KEINE SHA-256-ÜBEREINSTIMMUNG", "NO SHA-256 MATCH"],
  ["NICHT VERIFIZIERT", "NOT VERIFIED"],
  ["Integritätsabweichung: {file}", "Integrity mismatch: {file}"],
  ["Abweichung: {description}", "Deviation: {description}"],
  // evaluateRequirements labels and field names
  ["Track-Titel", "Track title"],
  ["Produktionsstart", "Production start"],
  ["Produktionsende", "Production end"],
  ["Angabe zur kommerziellen Nutzung", "Commercial-use indication"],
  ["Globaler Künstlername", "Global artist name"],
  ["Globale KI-Artwork-Transparenzrichtlinie", "Global AI artwork transparency policy"],
  ["Angabe zu externem Audio", "External-audio indication"],
  ["Angabe zu eigenem Audio", "Own-audio indication"],
  ["Angabe zur codebasierten Erzeugung", "Code-based generation indication"],
  ["Angabe zu fremden Samples", "Third-party samples indication"],
  ["Quelle des externen Audios", "Source of the external audio"],
  ["Rechtezuordnung des externen Audios", "Rights assignment for the external audio"],
  ["Lizenznachweis für externes Audio", "License evidence for external audio"],
  ["Importierte externe Audiodatei", "Imported external audio file"],
  ["Quelle des eigenen Audios", "Source of the own audio"],
  ["Rechtezuordnung des eigenen Audios", "Rights assignment for the own audio"],
  ["Importierte eigene Audiodatei", "Imported own audio file"],
  ["Quellcode oder Quelldatei der codebasierten Erzeugung", "Source code or source file for code-based generation"],
  [
    "Angabe zur Nachbearbeitung des codebasiert erzeugten Audios",
    "Post-processing indication for the code-generated audio"
  ],
  [
    "Bestätigte Nachbearbeitungsschritte des codebasiert erzeugten Audios",
    "Confirmed post-processing steps for the code-generated audio"
  ],
  ["Mit dem Quellcode erzeugte WAV- oder MP3-Datei", "WAV or MP3 file generated from the source code"],
  ["Quelle der fremden Samples", "Source of the third-party samples"],
  ["Rechtezuordnung der fremden Samples", "Rights assignment for the third-party samples"],
  ["Importierte Sample-Datei", "Imported sample file"],
  ["Lizenznachweis der fremden Samples", "License evidence for the third-party samples"],
  ["Suno-Modell", "Suno model"],
  ["Globaler Suno-Profilname", "Global Suno profile name"],
  ["Globaler Suno-Benutzername", "Global Suno username"],
  ["Globaler Suno-Tarif", "Global Suno plan"],
  ["Startdatum des globalen Suno-Abonnements", "Start date of the global Suno subscription"],
  ["Suno-Projekt-URL", "Suno project URL"],
  ["Datum der finalen Suno-Generation", "Date of the final Suno generation"],
  ["Suno-Tarif bei der finalen Generation", "Suno plan at final generation"],
  ["Finaler Suno-Export", "Final Suno export"],
  [
    "Suno-Exportdateiname stimmt mit Titel überein oder Abweichung ist bestätigt",
    "Suno export filename matches the title or the discrepancy is confirmed"
  ],
  ["Angabe: Instrumentaltrack", "Indication: instrumental track"],
  ["Angabe: Finales Audio enthält Gesang", "Indication: final audio contains vocals"],
  ["Beabsichtigte Gesangsnutzung", "Intended vocal use"],
  [
    "Eindeutige Inhaltsklassifizierung des Suno Generation Text Field",
    "Unambiguous content classification of the Suno Generation Text Field"
  ],
  ["Quelle des Inhalts im Suno-Lyrics-/Structure-Feld", "Source of the content in the Suno lyrics/structure field"],
  ["Exakter Inhalt des Suno-Lyrics-/Structure-Felds", "Exact content of the Suno lyrics/structure field"],
  ["Beschreibung des sonstigen Suno-Feldinhalts", "Description of the other Suno field content"],
  ["In Suno verwendeter Style-Prompt", "Style prompt used in Suno"],
  ["Angabe zu menschlicher Bearbeitung", "Human-editing indication"],
  ["Bestätigte menschliche Bearbeitungsschritte", "Confirmed human editing steps"],
  ["Angabe zur Bearbeitung auf dem Desktop-PC", "Indication of editing on the desktop computer"],
  ["Bearbeitungsschritte auf dem Desktop-PC", "Editing steps on the desktop computer"],
  ["Entstehungsart des Artworks", "Artwork creation type"],
  [
    "Mindestens eine menschliche Änderung am KI-assistierten Artwork",
    "At least one human change to the AI-assisted artwork"
  ],
  ["Content-Check: reale Person", "Content check: real person"],
  ["Content-Check: reales Ereignis", "Content check: real event"],
  ["Content-Check: Marke oder Logo", "Content check: trademark or logo"],
  ["Notiz zur dargestellten realen Person", "Note on the depicted real person"],
  ["Notiz zum dargestellten realen Ereignis", "Note on the depicted real event"],
  ["Notiz zur dargestellten Marke oder zum Logo", "Note on the depicted trademark or logo"],
  [
    "Finales Artwork muss exakt die lokal gekennzeichnete Fassung sein",
    "Final artwork must exactly match the locally disclosed version"
  ],
  ["Finales, aus Suno heruntergeladenes Artwork", "Final artwork downloaded from Suno"],
  ["Unverändertes KI-Artwork", "Unmodified AI artwork"],
  ["Verwendeter KI-Bilddienst", "AI image service used"],
  ["Globaler Standarddienst für KI-Bilder", "Global default service for AI images"],
  ["KI-Transparenzrichtlinie", "AI transparency policy"],
  ["Artwork-Hinweistext und lokal erzeugte Fassung", "Artwork disclosure text and locally generated version"],
  ["Explizite YES-/NO-Entscheidung zum Artwork-Hinweis", "Explicit YES/NO decision on the artwork disclosure"],
  ["Angabe zur Verwendung generativer KI im Audio", "Indication of generative AI use in the audio"],
  ["Für das Audio verwendetes KI-System", "AI system used for the audio"],
  ["Angabe zu KI-assistierten Audioelementen", "Indication of AI-assisted audio elements"],
  ["Angabe zu KI-generierten Audioelementen", "Indication of AI-generated audio elements"],
  ["Angabe zur absichtlichen Imitation einer realen Stimme", "Indication of intentional imitation of a real voice"],
  [
    "Angabe zur absichtlichen Darstellung der Identität einer realen Person",
    "Indication of intentional representation of a real person's identity"
  ],
  ["Angabe zu einem realen Ereignis als authentische Aufnahme", "Indication of a real event as an authentic recording"],
  [
    "Angabe zu realem Ort, Institution oder Ereignis als authentische KI-Aufnahme",
    "Indication of a real location, institution, or event as an authentic AI recording"
  ],
  [
    "Audio-Disclosure-Entscheidung dokumentiert; bei kommerzieller Nutzung nicht 'Not documented'",
    "Audio disclosure decision documented; not 'Not documented' for commercial use"
  ],
  ["Ort des Audio-Disclosures", "Audio disclosure location"],
  ["Text des Audio-Disclosures", "Audio disclosure text"],
  ["Datum der letzten Bearbeitung", "Last editing date"],
  ["Finale Release-Audiodatei", "Final release audio file"],
  [
    "Release-Dateiname stimmt mit Titel überein oder Abweichung ist bestätigt",
    "Release filename matches the title or the discrepancy is confirmed"
  ],
  [
    "Lokaler Chromaprint-Fingerprint für die aktuelle finale Release-Audiodatei",
    "Local Chromaprint fingerprint for the current final release audio file"
  ],
  ["Abo-/Zahlungsnachweis für den Produktionszeitraum", "Subscription/payment evidence for the production period"],
  [
    "Abo-Nachweis deckt das Datum der finalen Generation ab",
    "Subscription evidence covers the date of the final generation"
  ],
  [
    "Terms-Evidence ist vorhanden, aber die beschreibenden Metadaten sind unvollständig: Dokumenttitel, Provider/Quelle und Abrufdatum sind erforderlich.",
    "Terms evidence exists, but descriptive metadata is incomplete: document title, provider/source, and retrieval date are required."
  ],
  [
    "Verifizierter lokaler Suno-Terms-/Rights-Nachweis mit Titel, Provider und Abrufdatum",
    "Verified local Suno terms/rights evidence with title, provider, and retrieval date"
  ],
  ["Aktuelle generierte Dokumente", "Current generated documents"],
  ["Vollständige SHA-256-Verifikation", "Complete SHA-256 verification"],
  ["Alle blockierenden Abweichungen gelöst", "All blocking deviations resolved"],
  // evidenceRoleLabel values
  ["Suno Final-Export", "Suno final export"],
  ["Suno-Projekt-ZIP", "Suno project ZIP"],
  ["Suno-Screenshot", "Suno screenshot"],
  ["Abo-/Zahlungsnachweis", "Subscription/payment evidence"],
  ["Release-MP3", "Release MP3"],
  ["Release-MP4", "Release MP4"],
  ["Release-Artwork", "Release artwork"],
  ["Suno-Original-Artwork", "Suno original artwork"],
  ["KI-Artwork Original", "Original AI artwork"],
  ["KI-Artwork bearbeitet", "Edited AI artwork"],
  ["Menschlich bearbeitetes Artwork", "Human-edited artwork"],
  ["Finales Artwork", "Final artwork"],
  ["Lizenz für externes Audio", "License for external audio"],
  ["Externe Audiodatei", "External audio file"],
  ["Eigene Audiodatei", "Own audio file"],
  ["Quellcode / Quelldatei", "Source code / source file"],
  ["Codebasiert erzeugte Audiodatei", "Code-generated audio file"],
  ["Fremde Sample-Datei", "Third-party sample file"],
  ["Lizenz für fremde Samples", "License for third-party samples"],
  ["Suno-Nutzungsbedingungen / Rechteinformationen", "Suno terms of service / rights information"],
  ["Externer Zeitstempelnachweis", "External timestamp evidence"],
  ["Lyrics-Datei", "Lyrics file"],
  ["Style-/Prompt-Datei", "Style/prompt file"],
  ["Sonstiger Nachweis", "Other evidence"],
  // evidenceRoleFileTypes values
  ["WAV, MP3, FLAC, M4A, AIFF oder OGG", "WAV, MP3, FLAC, M4A, AIFF or OGG"],
  ["ZIP", "ZIP"],
  ["PNG, JPG, WebP oder PDF", "PNG, JPG, WebP or PDF"],
  ["PDF, PNG, JPG, TXT oder Markdown", "PDF, PNG, JPG, TXT or Markdown"],
  ["MP3", "MP3"],
  ["MP4 oder M4V", "MP4 or M4V"],
  ["PNG oder JPG", "PNG or JPG"],
  [
    "Ruby, Python, JavaScript, TypeScript, Text, Markdown und weitere Text-/Quellcodeformate",
    "Ruby, Python, JavaScript, TypeScript, text, Markdown, and other text/source-code formats"
  ],
  ["WAV oder MP3", "WAV or MP3"],
  ["PDF", "PDF"],
  [
    "TSR, TST, P7S, PDF, TXT, Markdown, JSON, HTML, PNG oder JPG",
    "TSR, TST, P7S, PDF, TXT, Markdown, JSON, HTML, PNG, or JPG"
  ],
  ["TXT oder Markdown", "TXT or Markdown"],
  ["PDF, Bild, Text, ZIP, WAV, MP3 oder MP4", "PDF, image, text, ZIP, WAV, MP3, or MP4"]
];
