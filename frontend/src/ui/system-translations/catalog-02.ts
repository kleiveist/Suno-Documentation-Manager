import type { SystemTranslation } from "./catalog-types";

export const SYSTEM_TRANSLATIONS_02: readonly SystemTranslation[] = [
  ["Die Suno-Projekt-URL ist ungültig.", "Suno project URL is invalid."],
  ["Die Suno-Projekt-URL muss eine HTTP(S)-URL mit Host sein.", "Suno project URL must be an HTTP(S) URL with a host."],
  ["Zu viele eingebettete Evidence-Metadateneinträge.", "Evidence contains too many embedded metadata entries."],
  ["Die Evidence-Quell-URL ist ungültig.", "Evidence source URL is invalid."],
  [
    "Die Evidence-Quell-URL muss eine HTTP(S)-URL mit Host sein.",
    "Evidence source URL must be an HTTP(S) URL with a host."
  ],
  ["Die Provider-Verifizierungs-URL ist ungültig.", "Provider verification URL is invalid."],
  [
    "Die Provider-Verifizierungs-URL muss eine HTTP(S)-URL mit Host sein.",
    "Provider verification URL must be an HTTP(S) URL with a host."
  ],
  [
    "Ein externer Zeitstempelnachweis benötigt Provider/Aussteller, Zeitstempel, referenzierten Hash und referenziertes Artefakt.",
    "External timestamp evidence requires provider/issuer, timestamp, referenced hash, and referenced artifact."
  ],
  [
    "Der referenzierte Hash eines externen Zeitstempelnachweises muss ein SHA-256-Wert sein.",
    "External timestamp evidence referenced hash must be a SHA-256 value."
  ],
  ["Der Albumtitel ist ungültig oder zu lang.", "Album title is invalid or too long."],
  ["Der Albumtitel ergibt keinen sicheren Ordnernamen.", "Album title does not form a safe folder name."],
  ["Der Tracktitel ergibt keinen sicheren Ordnernamen.", "Track title does not form a safe folder name."],
  ["Der Ordnername der Track-Bibliothek muss UTF-8 verwenden.", "Track library folder name must use UTF-8."],
  [
    "Der Tracktitel darf keine Pfadtrenner oder Traversal-Bestandteile enthalten.",
    "Track title must not contain path separators or traversal components."
  ],
  [
    "Für diesen Vorgang sind Produktionsbeginn und Produktionsende erforderlich.",
    "Production start and end dates are required for this operation."
  ],
  ["Künstlername", "Artist name"],
  ["Suno-Profilname", "Suno profile name"],
  ["Suno-Handle", "Suno handle"],
  ["Suno-Tarif", "Suno plan"],
  ["Standarddienst für KI-Bilder", "Default AI image service"],
  ["Hinweistext", "Disclosure text"],
  ["Abo-Startdatum", "Subscription start date"],
  ["Suno-Modell", "Suno model"],
  ["Suno-Tarif bei der Generation", "Suno plan at generation"],
  ["Historischer Suno-Tarif bei Erstellung", "Legacy Suno plan at creation"],
  ["KI-System für Audio", "Audio AI system"],
  ["Sonstiger Inhaltstyp im Suno-Lyrics-/Structure-Feld", "Other Suno lyrics/structure content type"],
  ["KI-Bilddienst", "AI image service"],
  ["Lyrics", "Lyrics"],
  ["Inhalt des Suno-Lyrics-/Structure-Felds", "Suno lyrics/structure field content"],
  ["Suno-Style-Prompt", "Suno style prompt"],
  ["Quelle für externes Audio", "External audio source"],
  ["Rechte für externes Audio", "External audio ownership"],
  ["Quelle für eigenes Audio", "Own audio source"],
  ["Rechte für eigenes Audio", "Own audio ownership"],
  ["Sample-Quelle", "Sample source"],
  ["Sample-Rechte", "Sample ownership"],
  ["Details zur menschlichen Bearbeitung", "Human editing details"],
  ["Details zur Bearbeitung nach dem Export", "Post-export editing details"],
  ["Notiz zur codebasierten Audio-Nachbearbeitung", "Code-audio post-processing note"],
  ["Notizen zum menschlichen Artwork-Prozess", "Human artwork process notes"],
  ["Individuelle Artwork-Änderung", "Custom artwork change"],
  ["Notiz zu realer Person", "Real-person note"],
  ["Notiz zu realem Ereignis", "Real-event note"],
  ["Notiz zu Marke", "Trademark note"],
  ["Text des Audio-Hinweises", "Audio disclosure text"],
  ["Begründung für Audio-Hinweis", "Audio disclosure reason"],
  ["Release-Notizen", "Release notes"],
  ["Operationen der codebasierten Audio-Nachbearbeitung", "Code-audio post-processing operations"],
  ["Operationen des menschlichen Artwork-Prozesses", "Human artwork process operations"],
  ["Menschliche Artwork-Änderungen", "Human artwork modifications"],
  ["Orte des Audio-Hinweises", "Audio disclosure locations"],
  ["Produktionsbeginn", "Production start"],
  ["Produktionsende", "Production end"],
  ["Produktionszeitraum", "Production period"],
  ["Datum der letzten Bearbeitung", "Last editing date"],
  ["Datum der finalen Generation", "Final generation date"],
  ["Suno-Download-/Exportdatum", "Suno download/export date"],
  ["Evidence-Dokumenttitel", "Evidence document title"],
  ["Evidence-Provider", "Evidence provider"],
  ["Evidence-Quell-URL", "Evidence source URL"],
  ["Sachliche Evidence-Notiz", "Evidence factual note"],
  ["Geltender Produktionszeitraum", "Applicable production period"],
  ["Zeitstempeltyp", "Timestamp type"],
  ["Zeitstempelwert", "Timestamp value"],
  ["Referenz-Hash", "Referenced hash"],
  ["Referenziertes Artefakt", "Referenced artifact"],
  ["Externe Referenz-ID", "External reference ID"],
  ["Provider-Verifizierungs-URL", "Provider verification URL"],
  ["Evidence-Dateierweiterung", "Evidence file extension"],
  ["Evidence-MIME-Typ", "Evidence MIME type"],
  ["Evidence-Audioformat", "Evidence audio format"],
  ["Suno-Erstellzeitstempel", "Suno created timestamp"],
  ["Suno-Erstellungsdatum", "Suno created date"],
  ["Technische Suno-ID", "Suno technical ID"],
  ["Rohe eingebettete Suno-Metadaten", "Raw embedded Suno metadata"],
  ["Schlüssel eingebetteter Metadaten", "Embedded metadata key"],
  ["Wert eingebetteter Metadaten", "Embedded metadata value"],
  ["Evidence-Abrufdatum", "Evidence retrieval date"],
  ["Evidence-Gültigkeitsdatum", "Evidence effective date"],
  ["Albumtitel", "Album title"],
  // Native evidence preview warnings
  [
    "Das Bild ist größer als 16 MB und wird deshalb nicht in den Arbeitsspeicher geladen.",
    "The image is larger than 16 MB and is therefore not loaded into memory."
  ],
  [
    "Die Textdatei ist größer als 512 KB und wird deshalb nicht vollständig geladen.",
    "The text file is larger than 512 KB and is therefore not loaded completely."
  ],
  [
    "ZIP-Dateien werden für die Vorschau nicht entpackt oder in den Arbeitsspeicher geladen.",
    "ZIP files are not unpacked or loaded into memory for preview."
  ],
  [
    "Für diesen Dateityp ist keine sichere Vorschau innerhalb der App verfügbar.",
    "No safe in-app preview is available for this file type."
  ],
  [
    "Für die Evidence-Vorschau ist eine reguläre verwaltete Datei erforderlich.",
    "Evidence preview requires a regular managed file."
  ],
  // Native application action results and validation messages
  ["{count} Dokumente wurden erzeugt.", "{count} documents generated."],
  [
    "Der aktuelle sichtbare KI-Hinweis ist bereits vorhanden; es wurde keine doppelte Datei erzeugt.",
    "The current visible AI disclosure already exists; no duplicate file was generated."
  ],
  [
    "Der sichtbare AI-Hinweis wurde lokal erzeugt; wähle das finale Artwork separat aus.",
    "Visible AI disclosure generated locally; select a final artwork separately."
  ],
  [
    "Ein sichtbarer Hinweis ist nur für AI-generiertes oder AI-assistiertes Artwork verfügbar.",
    "Visible disclosure is available only for AI-generated or AI-assisted artwork."
  ],
  ["Importiere zuerst das unveränderte AI-Artwork.", "Import the AI artwork original first."],
  ["Der globale Nachweis enthält keinen Beginn des Abdeckungszeitraums.", "Global evidence has no coverage start."],
  ["Der globale Nachweis enthält kein Ende des Abdeckungszeitraums.", "Global evidence has no coverage end."],
  [
    "Der ausgewählte Abo-Nachweis überschneidet sich weder mit dem dokumentierten Produktionszeitraum noch deckt er das dokumentierte Datum der finalen Generation ab.",
    "The selected subscription evidence neither overlaps the recorded production period nor covers the recorded final-generation date."
  ],
  [
    "Nur Abo-/Zahlungsnachweise oder Suno-Terms-/Rights-Evidence können einem Track zugeordnet werden.",
    "Only subscription/payment or Suno terms/rights evidence can be attached to a track."
  ],
  [
    "Ein externer Zeitstempelnachweis kann erst nach der technischen Finalisierung angehängt werden.",
    "External timestamp evidence can only be attached after technical finalization."
  ],
  [
    "Erzeuge die aktuellen verwalteten Dokumente, bevor du Prüfsummen berechnest.",
    "Generate the current managed documents before calculating hashes."
  ],
  ["{count} Dateien wurden gehasht und erneut verifiziert.", "{count} files hashed and re-verified."],
  ["{verified} von {total} Dateien verifiziert.", "{verified} of {total} files verified."],
  ["Lokale Chromaprint-Prüfung erfasst: {status}.", "Local Chromaprint screening recorded: {status}."],
  ["Externe ACRCloud-Prüfung erfasst: {status}.", "External ACRCloud screening recorded: {status}."],
  [
    "Dokumentation finalisiert und Zertifikatssatz verifiziert. Deutsche und englische technische Dokumentations-PDFs erzeugt: {germanFile}, {englishFile}",
    "Documentation finalized and certificate set verified. German and English technical documentation PDFs created: {germanFile}, {englishFile}"
  ],
  [
    "Ein externer Zeitstempelnachweis kann nur an einen gültigen technisch finalisierten Snapshot angehängt werden.",
    "External timestamp evidence can only be attached to a valid technically finalized snapshot."
  ],
  [
    "Die Integritätsprüfung des finalisierten Tracks ist fehlgeschlagen: {error}",
    "The finalized track integrity check failed: {error}"
  ],
  ["Finalisierter Track hat keine Zertifikats-ID.", "Finalized track has no certificate ID."],
  [
    "Die Integritätsprüfung des finalisierten Tracks ist vor dem Anhängen des Zeitstempels fehlgeschlagen.",
    "The finalized track integrity check failed before timestamp attachment."
  ],
  ["Der Track ist nicht finalisiert.", "The track is not finalized."],
  [
    "Zertifikat als ungültig markiert; Zertifikatsdateien wurden nicht überschrieben.",
    "Certificate marked invalid; certificate files were not overwritten."
  ],
  ["Nur ein finalisierter Track kann eine neue Revision beginnen.", "Only a finalized track can start a new revision."],
  [
    "Vorheriges Zertifikat als Revision {revision_id} archiviert.",
    "Previous certificate archived as revision {revision_id}."
  ],
  [
    "Der Track verwendet bereits die aktuelle Workflow-Version.",
    "The track already uses the current workflow version."
  ],
  ["Der aktuelle SHA256SUMS-Pfad ist keine reguläre Datei.", "The current SHA256SUMS path is not a regular file."],
  [
    "Vorheriges Zertifikat archiviert; der Track ist bereit für die Neubewertung mit Workflow {workflowId} {workflowVersion}.",
    "Previous certificate archived; track is ready for reevaluation with workflow {workflowId} {workflowVersion}."
  ],
  [
    "Der Track ist bereit für die Neubewertung mit Workflow {workflowId} {workflowVersion}.",
    "Track is ready for reevaluation with workflow {workflowId} {workflowVersion}."
  ],
  ["N/A benötigt eine Begründung.", "N/A requires a reason."],
  ["Abweichung nicht gefunden.", "Deviation not found."],
  ["Der Beginn des Abozeitraums ist erforderlich.", "Subscription coverage start is required."],
  ["Das Ende des Abozeitraums ist erforderlich.", "Subscription coverage end is required."],
  ["Kandidat mit symbolischem Link übersprungen: {fileName}", "Skipped symbolic-link candidate: {fileName}"],
  ["Ungültiger Albumordner {folderName} übersprungen: {error}", "Skipped invalid album folder {folderName}: {error}"],
  ["Track-Kandidat mit symbolischem Link übersprungen: {path}", "Skipped symbolic-link track candidate: {path}"],
  // Native action guards for current audio-screening state
  [
    "Importiere und verifiziere vor der Audio-Prüfung die maßgebliche finale Release-Audiodatei.",
    "Import and verify the authoritative final release audio before audio screening."
  ],
  [
    "Das maßgebliche Release-Audio hat sich während der Audio-Prüfung geändert. Das Ergebnis wurde verworfen; verifiziere oder ersetze die Release-Datei und führe die Prüfung erneut aus.",
    "The authoritative release audio changed while audio screening was running. The result was discarded; verify or replace the release file and run screening again."
  ],
  [
    "Veraltete Audio-Prüfungsartefakte sind noch im aktiven Track-Ordner vorhanden. Führe die lokale Prüfung nach dem Wiederherstellen oder Ersetzen der Release-Audiodatei erneut aus.",
    "Stale audio-screening artifacts are still present in the live track directory. Run the local screening again after restoring or replacing the release audio."
  ],
  [
    "Die Audio-Prüfung ist veraltet, weil sich das maßgebliche Release geändert hat. Verifiziere oder ersetze die Release-Audiodatei und führe die lokale Prüfung erneut aus, bevor du Dokumente oder Prüfsummen erzeugst.",
    "Audio screening is stale because the authoritative release changed. Verify or replace the release audio and run local screening again before generating documents or hashes."
  ],
  [
    "Der lokale Audio-Prüfungsdatensatz stimmt nicht mehr mit seinem portablen Artefakt überein. Führe die lokale Chromaprint-Prüfung erneut aus, bevor du fortfährst.",
    "The local audio-screening record no longer matches its portable artifact. Run the local Chromaprint screening again before continuing."
  ],
  [
    "Die externe Audio-Prüfungsantwort stimmt nicht mehr mit ihrem portablen Artefakt überein. Führe die explizite ACRCloud-Prüfung erneut aus, bevor du fortfährst.",
    "The external audio-screening response no longer matches its portable artifact. Run the explicit ACRCloud screening again before continuing."
  ],
  [
    "Importiere und verifiziere vor der externen Prüfung die maßgebliche finale Release-Audiodatei.",
    "Import and verify the authoritative final release audio before external screening."
  ],
  [
    "Erzeuge vor dem Start der externen Prüfung einen aktuellen lokalen Chromaprint-Fingerprint für die maßgebliche Release-Audiodatei.",
    "Generate a current local Chromaprint fingerprint for the authoritative release audio before starting external screening."
  ],
  [
    "Der aktuelle lokale Chromaprint-Datensatz fehlt oder hat sich geändert. Führe die lokale Prüfung erneut aus, bevor du die externe Prüfung startest.",
    "The current local Chromaprint record is missing or has changed. Run the local screening again before starting external screening."
  ],
  // Native local and external audio-screening messages
  ["ACRCloud konnte nicht erreicht werden.", "ACRCloud could not be reached."],
  ["Die ACRCloud-Antwort konnte nicht gelesen werden.", "The ACRCloud response could not be read."],
  [
    "Die ACRCloud-Antwort überschreitet die unterstützte Größenbegrenzung.",
    "The ACRCloud response exceeds the supported size limit."
  ],
  ["Die externe ACRCloud-Prüfung ist deaktiviert.", "External ACRCloud screening is disabled."],
  ["ACRCloud-Host oder Timeout ist ungültig.", "ACRCloud host or timeout is invalid."],
  [
    "ACRCloud Access Key und Access Secret sind nicht konfiguriert.",
    "ACRCloud access key and access secret are not configured."
  ],
  [
    "ACRCloud ist für ausdrücklich gestartete Audio-Prüfungen konfiguriert.",
    "ACRCloud is configured for explicitly started audio screening."
  ],
  [
    "Der ACRCloud-Host hat geantwortet. Dieser Test hat weder Audio noch Zugangsdaten gesendet.",
    "ACRCloud host responded. No audio or credentials were sent by this test."
  ],
  [
    "Der ACRCloud-Host stellt den erwarteten Identifikationsendpunkt nicht bereit.",
    "The ACRCloud host did not expose the expected identification endpoint."
  ],
  [
    "Der ACRCloud-Host ist für einen Verbindungstest vorübergehend nicht verfügbar.",
    "The ACRCloud host is temporarily unavailable for a connection test."
  ],
  [
    "Der ACRCloud-Host hat auf den Verbindungstest unerwartet geantwortet.",
    "The ACRCloud host returned an unexpected response to the connection test."
  ],
  [
    "Es wurde noch kein lokaler Chromaprint-Fingerprint erzeugt.",
    "No local Chromaprint fingerprint has been generated yet."
  ],
  ["Es wurde noch keine externe Katalogprüfung ausgeführt.", "No external catalog screening has been run."],
  [
    "Ein lokaler Chromaprint-Fingerprint wurde aus dem autoritativen Release-Audio erzeugt.",
    "A local Chromaprint fingerprint was generated from the authoritative release audio."
  ],
  [
    "Für diese Plattform ist keine gebündelte Chromaprint-Engine verfügbar.",
    "No bundled Chromaprint engine is available for this platform."
  ],
  ["Die gebündelte Chromaprint-Engine fehlt.", "Bundled Chromaprint engine is missing."],
  ["Die gebündelte Chromaprint-Engine ist keine reguläre Datei.", "Bundled Chromaprint engine is not a regular file."],
  [
    "Die gebündelte Chromaprint-Engine kann nicht verifiziert werden.",
    "Bundled Chromaprint engine cannot be verified."
  ],
  [
    "Die Verifikation der gebündelten Chromaprint-Engine ist fehlgeschlagen.",
    "Bundled Chromaprint engine verification failed."
  ],
  ["Die gebündelte Chromaprint-Engine ist nicht verfügbar.", "The bundled Chromaprint engine is unavailable."],
  [
    "Das autoritative Release-Audioformat wird vom gebündelten Chromaprint-Decoder nicht unterstützt.",
    "The authoritative release audio format is not supported by the bundled Chromaprint decoder."
  ],
  [
    "Chromaprint konnte für das autoritative Release-Audio keinen Fingerprint erzeugen.",
    "Chromaprint could not generate a fingerprint for the authoritative release audio."
  ],
  [
    "Ein aktueller lokaler Chromaprint-Fingerprint ist vor der externen Prüfung erforderlich.",
    "A current local Chromaprint fingerprint is required before external screening."
  ],
  [
    "Das autoritative Release-Audio konnte für die externe Prüfung nicht verifiziert werden.",
    "The authoritative release audio could not be verified for external screening."
  ],
  [
    "Die externe ACRCloud-Prüfung unterstützt derzeit PCM-WAV-Release-Audio.",
    "External ACRCloud screening currently supports PCM WAV release audio."
  ],
  [
    "Für ACRCloud konnte kein begrenztes WAV-Sample vorbereitet werden.",
    "A bounded WAV sample could not be prepared for ACRCloud."
  ],
  [
    "Das begrenzte ACRCloud-Sample liegt außerhalb der unterstützten Größenbegrenzung.",
    "The bounded ACRCloud sample is outside the supported size limit."
  ],
  ["Die ACRCloud-Zugangsdaten sind ungültig.", "ACRCloud credentials are invalid."],
  [
    "Die Providerantwort enthielt unsichere zugangsdatenähnliche Felder und wurde nicht dokumentiert.",
    "The provider response contained unsafe credential-like fields and was not documented."
  ],
  ["ACRCloud hat unerwartet geantwortet.", "ACRCloud returned an unexpected response."],
  [
    "ACRCloud hat für das übermittelte Audio-Sample keinen Katalogtreffer zurückgegeben.",
    "ACRCloud returned no catalog match for the submitted audio sample."
  ],
  [
    "ACRCloud hat für das übermittelte Audio-Sample einen oder mehrere Katalogtreffer zurückgegeben.",
    "ACRCloud returned one or more catalog matches for the submitted audio sample."
  ],
  [
    "ACRCloud hat die konfigurierten Zugangsdaten nicht akzeptiert.",
    "ACRCloud did not accept the configured credentials."
  ],
  [
    "ACRCloud ist für diese Prüfungsanfrage vorübergehend nicht verfügbar.",
    "ACRCloud is temporarily unavailable for this screening request."
  ],
  ["ACRCloud hat eine unerwartete HTTP-Antwort zurückgegeben.", "ACRCloud returned an unexpected HTTP response."],
  [
    "Der externe Audio-Prüfdatensatz enthält einen nicht endlichen Provider-Score.",
    "The external audio-screening record contains a non-finite provider score."
  ],
  ["Die ACRCloud-Antwort ist kein gültiges JSON.", "The ACRCloud response is not valid JSON."],
  ["Ungültiger Name des Audio-Prüfartefakts.", "Invalid audio-screening artifact name."],
  ["Der aktuelle Audio-Prüfpfad ist kein Verzeichnis.", "The current audio-screening path is not a directory."],
  [
    "Das aktuelle Audio-Prüfverzeichnis enthält einen nicht unterstützten Dateityp.",
    "The current audio-screening directory contains an unsupported file type."
  ],
  [
    "Die autoritative Release-Audiodatei konnte für die Fingerprinterstellung nicht verifiziert werden.",
    "The authoritative release audio could not be verified for fingerprinting."
  ],
  [
    "Die gebündelte Chromaprint-Engine ist für diese Installation nicht verfügbar.",
    "The bundled Chromaprint engine is unavailable for this installation."
  ],
  [
    "ACRCloud hat einen nicht endlichen Provider-Score zurückgegeben.",
    "ACRCloud returned a non-finite provider score."
  ],
  [
    "Das autoritative Release-Audio hat sich geändert; der lokale Fingerprint ist veraltet.",
    "The authoritative release audio changed; the local fingerprint is stale."
  ],
  [
    "Das autoritative Release-Audio hat sich geändert; das externe Katalogergebnis ist veraltet.",
    "The authoritative release audio changed; the external catalog result is stale."
  ],
  // Configurable ACRCloud coverage UI and archived multi-sample summary
  ["ACRCloud-Prüfintensität", "ACRCloud screening intensity"],
  [
    "Lege fest, wie viel eines Tracks bei einer bewusst gestarteten externen Prüfung repräsentativ und ohne überlappende Samples geprüft wird.",
    "Choose how much of a track is checked representatively and without overlapping samples during an explicitly started external screening."
  ],
  ["Gewünschte Prüfintensität", "Requested screening intensity"],
  ["Gewünschte ACRCloud-Prüfintensität", "Requested ACRCloud screening intensity"],
  ["Dynamische Berechnung nach tatsächlicher Tracklänge", "Dynamic calculation by actual track duration"],
  [
    "Ist der Schalter aktiv, wird die Zielprüfzeit beim Lauf aus der verifizierten Dauer der aktuellen Release-Datei bestimmt.",
    "When enabled, the target screening time is calculated from the verified duration of the current release file at run time."
  ],
  ["Referenzlänge (Minuten)", "Reference length (minutes)"],
  ["Referenzlänge in Minuten", "Reference length in minutes"],
  [
    "Die tatsächliche Trackdauer wird beim Lauf nie überschritten.",
    "The actual track duration is never exceeded at run time."
  ],
  [
    "Das Audio-Screening ist ausschließlich ein technischer Vergleichsdatensatz und begründet keine Aussage zu Urheberschaft, Rechteinhaberschaft, Erlaubnis, Nichtverletzung, Rechtmäßigkeit oder Release-Freigabe.",
    "Audio screening is solely a technical comparison record and makes no statement about authorship, ownership, permission, non-infringement, lawfulness, or release approval."
  ],
  ["Antwortarchiv nicht dokumentiert", "Response archive not documented"],
  // Native external timestamp provider and attachment messages
  ["Zeitstempel-Provider konnte nicht erreicht werden.", "Timestamp provider could not be reached."],
  [
    "Die Antwort des Zeitstempel-Providers konnte nicht gelesen werden.",
    "Timestamp provider response could not be read."
  ],
  [
    "Die Antwort des Zeitstempel-Providers überschreitet die unterstützte Größenbegrenzung.",
    "Timestamp provider response exceeds the supported size limit."
  ],
  [
    "OpenTimestamps-Kalenderdienst ist bereit. Dies ist kein RFC 3161; ein erster Detached Proof bleibt ATTACHED, bis die OpenTimestamps-Verifikation oder ein Upgrade seine Bitcoin-Verankerung bestätigt.",
    "OpenTimestamps calendar service is ready. This is not RFC 3161; an initial detached proof remains ATTACHED until OpenTimestamps verification or upgrade confirms its Bitcoin anchoring."
  ],
  [
    "Eine explizite TSA-CA-Trust-Anchor-Datei ist erforderlich, bevor RFC-3161-Antworten als VERIFIED markiert werden können.",
    "An explicit TSA CA trust-anchor file is required before RFC 3161 responses can be marked VERIFIED."
  ],
  [
    "RFC-3161-Zeitstempeldienst und expliziter TSA-Trust-Anchor sind bereit.",
    "RFC 3161 timestamp service and explicit TSA trust anchor are ready."
  ],
  [
    "Custom-RFC-3161-Zeitstempeldienst und expliziter TSA-Trust-Anchor sind konfiguriert.",
    "Custom RFC 3161 timestamp service and explicit TSA trust anchor are configured."
  ],
  ["Ein Custom-RFC-3161-TSA-Endpunkt ist erforderlich.", "Custom RFC 3161 TSA endpoint is required."],
  [
    "Der Custom-RFC-3161-TSA-Endpunkt ist keine gültige HTTP(S)-URL.",
    "Custom RFC 3161 TSA endpoint is not a valid HTTP(S) URL."
  ],
  [
    "Der Custom-RFC-3161-TSA-Endpunkt muss eine reine HTTP(S)-URL ohne eingebettete Zugangsdaten oder Query-Werte sein.",
    "Custom RFC 3161 TSA endpoint must be a plain HTTP(S) URL without embedded credentials or query values."
  ],
  [
    "Das Timeout für Custom RFC 3161 muss zwischen 1 und 120 Sekunden liegen.",
    "Custom RFC 3161 timeout must be between 1 and 120 seconds."
  ],
  ["Die Custom-RFC-3161-Policy-OID ist ungültig.", "Custom RFC 3161 policy OID is invalid."],
  [
    "Für die Client-Zertifikat-Authentifizierung ist ein Client-Zertifikatspfad erforderlich.",
    "A client certificate path is required for client-certificate authentication."
  ],
  [
    "Die Client-Zertifikat-Authentifizierung ist vorbereitet, wird von diesem Provider-Adapter aber noch nicht unterstützt.",
    "Client-certificate authentication is prepared but is not enabled by this provider adapter yet."
  ],
  [
    "Für die Basic-Authentifizierung ist ein Benutzername erforderlich.",
    "A username is required for Basic authentication."
  ],
  [
    "Für den konfigurierten Zeitstempeldienst ist ein Passwort oder Token erforderlich.",
    "A password or token is required for the configured timestamp service."
  ],
  [
    "Der Provider hat geantwortet, aber seine Testantwort konnte technisch nicht verifiziert werden.",
    "Provider responded, but its test response could not be technically verified."
  ],
  [
    "OpenTimestamps-Kalenderdienst erreichbar. Dies ist kein RFC 3161; ein erster Proof bleibt ATTACHED, bis die OpenTimestamps-Verifikation oder ein Upgrade erfolgt.",
    "OpenTimestamps calendar service reachable. This is not RFC 3161; an initial proof remains ATTACHED pending OpenTimestamps verification or upgrade."
  ],
  ["RFC-3161-Zeitstempeldienst bereit.", "RFC 3161 timestamp service ready."],
  ["Der Digest des Zeitstempel-Anchors ist kein SHA-256-Wert.", "Timestamp anchor digest is not a SHA-256 value."],
  ["OpenTimestamps hat einen leeren Proof zurückgegeben.", "OpenTimestamps returned an empty proof."],
  [
    "OpenTimestamps-Detached-Proof angehängt; spätere Verifikation oder ein Upgrade ist verfügbar.",
    "OpenTimestamps detached proof attached; later verification or upgrade is available."
  ],
  ["Der Zeitstempel-Provider hat eine leere Antwort zurückgegeben.", "Timestamp provider returned an empty response."],
  [
    "Die Zeitstempelantwort wurde archiviert, aber die RFC-3161-Antwortverifikation ist fehlgeschlagen: {reason}.",
    "Timestamp response was archived, but RFC 3161 response verification failed: {reason}."
  ],
  [
    "Der Zeitstempel-Provider hat die konfigurierte Authentifizierung abgelehnt.",
    "Timestamp provider rejected the configured authentication."
  ],
  [
    "Der Zeitstempel-Provider hat den HTTP-Status {status} zurückgegeben.",
    "Timestamp provider returned HTTP status {status}."
  ],
  [
    "Eine Roharchivdatei der Providerantwort erfordert vom Provider abgeleitete Metadaten.",
    "A raw provider response archive requires provider-derived metadata."
  ],
  ["Der Dateiname des Zeitstempel-Evidence ist ungültig.", "Timestamp evidence file name is invalid."],
  [
    "Der Dateiname des Zeitstempel-Evidence enthält unsichere Zeichen.",
    "Timestamp evidence file name contains unsafe characters."
  ]
];
