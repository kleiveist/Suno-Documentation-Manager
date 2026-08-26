import type { SystemTranslation } from "./catalog-types";

export const SYSTEM_TRANSLATIONS_03: readonly SystemTranslation[] = [
  [
    "Das referenzierte Zeitstempelartefakt muss eine reguläre verwaltete Datei sein.",
    "The referenced timestamp artifact must be a regular managed file."
  ],
  [
    "Der finalisierte Manifest-Anchor hat sich geändert, bevor die Providerantwort vorbereitet werden konnte.",
    "The finalized manifest anchor changed before the provider response could be staged."
  ],
  ["Die Referenz-ID des Zeitstempeldatensatzes ist ungültig.", "Timestamp record ID is invalid."],
  ["Der referenzierte Hash muss ein SHA-256-Wert sein.", "Referenced hash must be a SHA-256 value."],
  ["Die Verifikations-URL des Providers ist ungültig.", "Provider verification URL is invalid."],
  [
    "Die Verifikations-URL des Providers muss eine HTTP(S)-URL mit Host sein.",
    "Provider verification URL must be an HTTP(S) URL with a host."
  ],
  [
    "Integritätsabweichung im Zeitstempel-Addendum: {fileName}",
    "External timestamp addendum integrity mismatch: {fileName}"
  ],
  [
    "Ein technisch verifizierter externer Zeitstempel ist bereits an diesen finalisierten Snapshot angehängt.",
    "A technically verified external timestamp is already attached to this finalized snapshot."
  ],
  [
    "Der ausgewählte Zeitstempel-Anchor stimmt nicht mehr mit dem finalisierten Snapshot überein.",
    "The selected timestamp anchor no longer matches the finalized snapshot."
  ],
  [
    "Ein externer Zeitstempelnachweis wird für den finalisierten Manifest-Anchor angefordert.",
    "Requesting external timestamp evidence for the finalized manifest anchor."
  ],
  [
    "Der ausgewählte Zeitstempel-Anchor hat sich vor Beginn der Provideranfrage geändert.",
    "The selected timestamp anchor changed before the provider request started."
  ],
  [
    "Der ausgewählte Zeitstempel-Anchor hat sich während der Provideranfrage geändert.",
    "The selected timestamp anchor changed while the provider request was in progress."
  ],
  [
    "Die Antwort des Zeitstempel-Providers konnte nicht sicher archiviert werden.",
    "Timestamp provider response could not be archived safely."
  ],
  [
    "Das Anhängen des Zeitstempelnachweises hat die Integritätsmenge der ersten Phase verändert.",
    "Attaching timestamp evidence changed the phase-one integrity set."
  ],
  [
    "Das Anhängen des Zeitstempelnachweises hat einen finalisierten Anchor verändert.",
    "Attaching timestamp evidence changed a finalized anchor."
  ],
  [
    "Die Antwort des Zeitstempel-Providers konnte nicht sicher veröffentlicht werden.",
    "Timestamp provider response could not be published safely."
  ],
  [
    "Die RFC-3161-Antwort und ihre aktuelle Sidecar-Integrität sind technisch verifiziert.",
    "The RFC 3161 response and its current sidecar integrity are technically verified."
  ],
  [
    "Eine gespeicherte VERIFIED-Zeitstempelzusammenfassung hat kein aktuell verifizierbares veröffentlichtes Sidecar.",
    "A stored VERIFIED timestamp summary has no currently verifiable published sidecar."
  ],
  [
    "Ein externer Zeitstempelnachweis ist vorhanden, aber die Integritätsprüfung seines aktuellen Sidecars oder referenzierten Anchors ist fehlgeschlagen.",
    "External timestamp evidence is present, but its current sidecar or referenced-anchor integrity verification failed."
  ],
  [
    "Der unveränderliche Datensatz behauptet VERIFIED, aber das vollständige aktuelle RFC-3161-Verifikationsprädikat ist nicht erfüllt.",
    "The immutable record claims VERIFIED, but the complete current RFC 3161 verification predicate is not satisfied."
  ],
  [
    "Ein externer Zeitstempelnachweis ist angehängt; sein unveränderlicher Provider-Verifikationsstatus wird ohne Hochstufung angezeigt.",
    "External timestamp evidence is attached; its immutable provider verification status is shown without promotion."
  ],
  [
    "Manuell erfasster historischer Zeitstempelnachweis ist angehängt; er wurde nicht automatisch zu einem providerverifizierten Nachweis hochgestuft.",
    "Legacy manually recorded timestamp evidence is attached; it has not been automatically promoted to provider-verified evidence."
  ],
  // Fixed text sent through OperationProgress.currentFile by native audio checks
  ["Autoritatives Release-Audio wird vorbereitet", "Preparing authoritative release audio"],
  ["Lokaler Chromaprint-Fingerprint wird erzeugt", "Generating local Chromaprint fingerprint"],
  ["Lokale Fingerprint-Operation abgeschlossen", "Local fingerprint operation completed"],
  ["Audio-Prüfdatensatz wird gespeichert", "Saving audio-screening record"],
  ["Audio-Prüfung abgeschlossen", "Audio screening completed"],
  ["Externe Katalogprüfung wird vorbereitet", "Preparing external catalog screening"],
  ["Begrenztes Audio-Sample wird an ACRCloud gesendet", "Sending bounded audio sample to ACRCloud"],
  ["ACRCloud-Antwort wird abgewartet", "Waiting for ACRCloud response"],
  ["ACRCloud-Antwort wird verarbeitet", "Processing ACRCloud response"],
  ["Audio-Prüfergebnis wird gespeichert", "Saving audio-screening result"],
  // Native timestamp integrity, evidence, artwork, and hash diagnostics
  [
    "Der Zeitstempel-Evidence-Datensatz hat keine gültige Dateierweiterung.",
    "Timestamp evidence record has no valid extension."
  ],
  [
    "Der Zeitstempel-Evidence-Datensatz hat eine nicht unterstützte Dateierweiterung.",
    "Timestamp evidence record has an unsupported extension."
  ],
  ["Der Dateiname des Zeitstempel-Sidecars ist ungültig.", "Timestamp sidecar filename is invalid."],
  [
    "Das Zeitstempel-Sidecar enthält keine reguläre Datei: {fileName}",
    "Timestamp sidecar contains a non-regular file: {fileName}"
  ],
  [
    "Der Dateisatz des Zeitstempel-Sidecars stimmt nicht mit seinem verwalteten Datensatz überein.",
    "Timestamp sidecar file set does not match its managed record."
  ],
  [
    "Das Legacy-Zeitstempel-Sidecar enthält keine Integritätsaussage zum Veröffentlichungszeitpunkt.",
    "Legacy timestamp sidecar has no publication-time integrity assertion."
  ],
  [
    "Die Legacy-TIMESTAMP_RECORD.json weicht vom registrierten Zeitstempeldatensatz ab.",
    "Legacy TIMESTAMP_RECORD.json differs from the registered timestamp record."
  ],
  [
    "Das Zeitstempel-Sidecar dokumentiert keine erfolgreiche Integritätsprüfung zum Veröffentlichungszeitpunkt.",
    "Timestamp sidecar does not record successful publication-time integrity verification."
  ],
  [
    "TIMESTAMP_RECORD.json ist nicht der exakt registrierte unveränderliche Datensatz.",
    "TIMESTAMP_RECORD.json is not the exact immutable registered record."
  ],
  [
    "Nicht unterstützte Formatversion des externen Zeitstempel-Sidecars: {version}.",
    "Unsupported external timestamp sidecar format version: {version}."
  ],
  ["Der Hash des externen Zeitstempel-Evidence fehlt.", "External timestamp evidence hash is missing."],
  [
    "Der SHA-256-Wert des externen Zeitstempel-Evidence stimmt nicht mehr mit dem registrierten Wert überein.",
    "External timestamp evidence SHA-256 no longer matches its registered value."
  ],
  ["Der Hash des Providerantwort-Archivs fehlt.", "Provider response archive hash is missing."],
  [
    "Der SHA-256-Wert des Providerantwort-Archivs stimmt nicht mehr mit seinen unveränderlichen Metadaten überein.",
    "Provider response archive SHA-256 no longer matches its immutable metadata."
  ],
  [
    "Die SHA-256-Liste des Zeitstempel-Sidecars ist unvollständig oder stimmt nicht mehr überein.",
    "Timestamp sidecar SHA-256 list is incomplete or no longer matches."
  ],
  ["Der Hash des Zeitstempel-Markdown-Dokuments fehlt.", "Timestamp Markdown hash is missing."],
  ["Der Hash des Zeitstempel-PDFs fehlt.", "Timestamp PDF hash is missing."],
  [
    "Die Bytes des externen Zeitstempel-Markdowns stimmen nicht mehr mit ihrem Veröffentlichungshash überein.",
    "External timestamp Markdown bytes no longer match their publication hash."
  ],
  [
    "Die Bytes des externen Zeitstempel-PDFs stimmen nicht mehr mit ihrem Veröffentlichungshash überein.",
    "External timestamp PDF bytes no longer match their publication hash."
  ],
  [
    "Der Zeitstempeldatensatz stimmt nicht mehr mit dem ausgewählten finalisierten Artefakt überein.",
    "Timestamp record no longer matches the selected finalized artifact."
  ],
  [
    "Die RFC-3161-Antwortstruktur, der SHA-256-Message-Imprint, Nonce und die vom Provider zurückgegebene Policy-OID (einschließlich einer angeforderten Policy-Übereinstimmung, falls konfiguriert) wurden geprüft. Dieser Endpunkt-Verbindungstest hat keine finalisierten Artefaktbytes bereitgestellt; CMS- und Vertrauensketteprüfung wurden daher nicht behauptet.",
    "RFC 3161 response structure, SHA-256 message imprint, nonce, and the provider-returned policy OID (including a requested-policy match when configured) were checked. This endpoint connection test did not supply finalized artifact bytes, so CMS and trust-chain verification were not asserted."
  ],
  [
    "Die dem RFC-3161-Verifizierer bereitgestellten finalisierten Artefaktbytes stimmen nicht mehr mit dem ausgewählten SHA-256-Anchor überein.",
    "The finalized artifact bytes supplied to the RFC 3161 verifier no longer match the selected SHA-256 anchor."
  ],
  [
    "Die RFC-3161-Antwortstruktur, der SHA-256-Message-Imprint, Nonce und die vom Provider zurückgegebene Policy-OID (einschließlich einer angeforderten Policy-Übereinstimmung, falls konfiguriert) wurden geprüft, aber keine explizite TSA-CA-Trust-Anchor-Datei ist konfiguriert. Die Antwort bleibt ohne VERIFIED-Behauptung archiviert.",
    "RFC 3161 response structure, SHA-256 message imprint, nonce, and the provider-returned policy OID (including a requested-policy match when configured) were checked, but no explicit TSA CA trust-anchor file is configured. The response remains archived without a VERIFIED claim."
  ],
  [
    "RFC-3161-Antwort mit {RFC3161_CRYPTOGRAPHIC_VERIFIER} verifiziert: SHA-256-Message-Imprint, Request-Nonce, vom Provider zurückgegebene Policy-OID (und angeforderte Policy-Übereinstimmung, falls konfiguriert), CMS-Signatur, kritische und alleinige timeStamping-EKU, Zertifikatsgültigkeit zu genTime und die Kette zum konfigurierten Trust Anchor stimmen überein. SHA-256 des Trust Anchors: {hash}.",
    "RFC 3161 response verified with {RFC3161_CRYPTOGRAPHIC_VERIFIER}: SHA-256 message imprint, request nonce, provider-returned policy OID (and requested-policy match when configured), CMS signature, critical and sole timeStamping EKU, certificate validity at genTime, and chain to the configured trust anchor all match. Trust anchor SHA-256: {hash}."
  ],
  [
    "RFC-3161-Antwort wurde archiviert, aber die CMS-/X.509-Verifikation ist fehlgeschlagen: {error}.",
    "RFC 3161 response was archived, but CMS/X.509 verification failed: {error}."
  ],
  [
    "Die konfigurierte TSA-Trust-Anchor-Datei kann nicht gelesen werden.",
    "The configured TSA trust-anchor file cannot be read."
  ],
  [
    "Der konfigurierte TSA-Trust-Anchor muss eine reguläre Datei ohne symbolischen Link sein.",
    "The configured TSA trust anchor must be a regular, non-symlink file."
  ],
  [
    "Die konfigurierte TSA-Trust-Anchor-Datei muss 1 bis {MAX_TRUST_ANCHOR_BYTES} Bytes enthalten.",
    "The configured TSA trust-anchor file must contain 1 to {MAX_TRUST_ANCHOR_BYTES} bytes."
  ],
  ["Das konfigurierte TSA-Trust-Anchor-PEM ist ungültig.", "The configured TSA trust-anchor PEM is invalid."],
  [
    "Die konfigurierte TSA-Trust-Anchor-Datei enthält keine Zertifikate.",
    "The configured TSA trust-anchor file contains no certificates."
  ],
  ["Ungültiger Objektbezeichner.", "Invalid object identifier."],
  ["DER-Element ist abgeschnitten.", "DER element is truncated."],
  ["DER-Länge ist ungültig.", "DER length is invalid."],
  ["Genau eine DER-SEQUENCE wurde erwartet.", "Expected one DER SEQUENCE."],
  ["Die RFC-3161-Antwort enthält kein Timestamp-Token.", "RFC 3161 response has no timestamp token."],
  ["Das RFC-3161-Timestamp-Token ist ungültig.", "RFC 3161 timestamp token is invalid."],
  ["RFC-3161-CMS-SignedData ist ungültig.", "RFC 3161 CMS SignedData is invalid."],
  ["Die RFC-3161-Erstellungszeit ist ungültig.", "RFC 3161 generation time is invalid."],
  [
    "Die Zeitstempelstelle hat die Anfrage abgelehnt (RFC-3161-Status {status}).",
    "Timestamp authority rejected the request (RFC 3161 status {status})."
  ],
  [
    "Die Evidence-Quelle muss eine reguläre Datei ohne symbolischen Link sein.",
    "Evidence source must be a regular, non-symbolic-link file."
  ],
  [
    "Der Inhalt der Evidence-Datei stimmt nicht mit dem Dateityp .{extension} überein.",
    "Evidence file contents do not match the .{extension} file type."
  ],
  ["Die Evidence-Datei fehlt.", "Evidence file is missing."],
  ["SHA-256-Abweichung.", "SHA-256 mismatch."],
  [
    "Die Legacy-Evidence-Typverifikation ist fehlgeschlagen: {error}",
    "Legacy evidence type verification failed: {error}"
  ],
  [
    "Die Track-Cover-Vorschau benötigt ein reguläres verwaltetes Bild.",
    "Track cover preview requires a regular managed image."
  ],
  [
    "Das finale Artwork ist für die Track-Cover-Vorschau zu groß.",
    "The final artwork is too large for the track cover preview."
  ],
  [
    "Der sichtbare Hinweis benötigt das verifizierte KI-Artwork-Original.",
    "Visible disclosure requires the verified AI artwork original."
  ],
  [
    "Der Hinweistext muss 1 bis 80 sichtbare Zeichen enthalten.",
    "Disclosure text must contain 1 to 80 visible characters."
  ],
  ["Es gibt keine Track-Dateien zum Hashen.", "There are no track files to hash."],
  ["Track-Pfade ohne UTF-8 können nicht gehasht werden.", "Non-UTF-8 track paths cannot be hashed."],
  [
    "Der Track-Pfad enthält Zeichen, die von SHA256SUMS nicht unterstützt werden.",
    "Track path contains characters unsupported by SHA256SUMS."
  ],
  ["Ungültige SHA256SUMS-Zeile {line}.", "Invalid SHA256SUMS line {line}."],
  ["Ungültiger SHA-256-Digest in Zeile {line}.", "Invalid SHA-256 digest on line {line}."],
  ["Doppelter SHA256SUMS-Pfad: {path}", "Duplicate SHA256SUMS path: {path}"],
  ["Der Importpfad muss ein normaler Ordner sein.", "The import path must be a regular directory."],
  ["Unbenannter Track", "Untitled track"],
  ["{role}: {count} Kandidaten", "{role}: {count} candidates"],
  ["suno_final_export: {wav_count} WAV-Kandidaten", "suno_final_export: {wav_count} WAV candidates"],
  ["Ungültige WAV-Datei '{fileName}': {message}.", "Invalid WAV file '{fileName}': {message}."],
  // Remaining native application recovery, import, and validation diagnostics
  [
    "Der Finalisierungs-Wiederherstellungsmarker stimmt nicht mit seinem Track überein.",
    "Finalization recovery marker does not match its track."
  ],
  [
    "Der Finalisierungs-Wiederherstellungsmarker enthält keine Transaktions-ID.",
    "Finalization recovery marker has no transaction ID."
  ],
  ["Ein Track-Ordner benötigt einen übergeordneten Ordner.", "A track folder needs a parent."],
  [
    "Der Quellordner hat sich seit der Vorschau geändert. Bitte erneut analysieren.",
    "The source folder changed since the preview. Please analyse it again."
  ],
  [
    "Der Ziel-Track würde innerhalb des ausgewählten Quellordners liegen. Wähle einen getrennten Quellordner, damit die Quelle unverändert bleibt.",
    "The target track would be located inside the selected source folder. Choose a separate source folder so the source remains unchanged."
  ],
  [
    "Terms-Evidence kann nicht als nicht verfügbar markiert werden, solange eine verifizierte lokale Terms-Evidence-Datei angehängt ist.",
    "Terms evidence cannot be marked unavailable while a verified local Terms evidence file is attached."
  ],
  [
    "Umbenennen des Release fehlgeschlagen ({error}); Zurücksetzen des Ordners fehlgeschlagen: {error}",
    "Release rename failed ({error}); folder rollback failed: {error}"
  ],
  [
    "Aktualisierung des Tracks fehlgeschlagen ({error}); Zurücksetzen des Ordners fehlgeschlagen: {error}",
    "Track update failed ({error}); folder rollback failed: {error}"
  ],
  [
    "Aktualisierung des Tracks fehlgeschlagen ({error}); Zurücksetzen des Release fehlgeschlagen: {error}",
    "Track update failed ({error}); release rollback failed: {error}"
  ],
  [
    "Aktualisierung der Bibliothek fehlgeschlagen ({error}); Zurücksetzen des Ordners fehlgeschlagen: {error}",
    "Library update failed ({error}); folder rollback failed: {error}"
  ],
  [
    "Umbenennen des Albums fehlgeschlagen ({error}); Zurücksetzen des Ordners fehlgeschlagen: {error}",
    "Album rename failed ({error}); folder rollback failed: {error}"
  ],
  [
    "Nur ein importierter Legacy-Track benötigt die Übernahme des Profil-Snapshots.",
    "Only an imported legacy track needs profile-snapshot adoption."
  ],
  [
    "Dieser Schritt hat anwendbare Pflichtanforderungen und kann nicht N/A sein.",
    "This step has applicable mandatory requirements and cannot be N/A."
  ],
  ["N/A-Begründung", "N/A reason"],
  ["Blockierende Abweichung", "Blocking deviation"],
  [
    "Registriere das Suno-Terms-PDF mit dem dafür vorgesehenen globalen Import.",
    "Register the Suno terms PDF with the dedicated global importer."
  ],
  ["Abo-Abdeckung", "Subscription coverage"],
  ["Beginn des Evidence-Abdeckungszeitraums", "Evidence coverage start"],
  ["Ende des Evidence-Abdeckungszeitraums", "Evidence coverage end"],
  [
    "Nur Suno-Terms-/Rights-Evidence hat hier bearbeitbare beschreibende Metadaten.",
    "Only Suno terms/rights evidence has editable descriptive metadata here."
  ],
  [
    "Ein Abrechnungsrhythmus kann nur für Abo-/Zahlungs-Evidence verwendet werden.",
    "A billing cycle can only be used for subscription/payment evidence."
  ],
  ["Der Evidence-Pfad enthält keinen Dateinamen.", "Evidence path has no file name."],
  [
    "Registriere Abo- und Suno-Terms-/Rights-Evidence global in den Einstellungen und hänge eine portable Kopie an.",
    "Register subscription and Suno terms/rights evidence globally in Settings and attach a portable copy."
  ],
  [
    "Die Evidence-Rolle '{role}' ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen.",
    "The evidence role '{role}' is already assigned. Use the upload button on the existing evidence to replace it safely."
  ],
  [
    "Unter {path} ist bereits Evidence registriert. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen.",
    "Evidence is already registered at {path}. Use the upload button on the existing evidence to replace it safely."
  ],
  [
    "Ersetze Abo- und Suno-Terms-/Rights-Evidence im globalen Evidence-Register.",
    "Replace subscription and Suno terms/rights evidence in the global evidence register."
  ],
  [
    "Die ausgewählte Ersatzrolle stimmt nicht mit der vorhandenen Evidence überein.",
    "The selected replacement role does not match the existing evidence."
  ],
  [
    "Ein anderer Evidence-Datensatz verwendet bereits {path}. Entferne diesen Datensatz, bevor du die Datei ersetzt.",
    "Another evidence record already uses {path}. Remove that record before replacing this file."
  ],
  ["Der Legacy-Evidence-Pfad enthält keinen Dateinamen.", "Legacy evidence path has no file name."],
  ["Die indexierte Legacy-Evidence ist keine reguläre Datei.", "Indexed legacy evidence is not a regular file."],
  [
    "Entfernen der Legacy-Evidence fehlgeschlagen ({error}); das Zurücksetzen war unvollständig.",
    "Legacy evidence removal failed ({error}); rollback was incomplete."
  ],
  [
    "Entfernen der Evidence fehlgeschlagen ({error}); das Zurücksetzen war unvollständig.",
    "Evidence removal failed ({error}); rollback was incomplete."
  ],
  [
    "Evidence-Integritätsabweichung nach der Finalisierung erkannt",
    "Evidence integrity mismatch detected after finalization"
  ],
  ["Track-Integrität nach der Finalisierung geändert", "Track integrity changed after finalization"],
  [
    "Das Zertifikatsverzeichnis enthält bereits Dateien. Bewahre sie auf oder archiviere sie, bevor du finalisierst.",
    "The certificate directory already contains files. Preserve or archive them before finalizing."
  ],
  [
    "Das technische Dokumentations-PDF existiert bereits: {path}",
    "The technical documentation PDF already exists: {path}"
  ],
  [
    "Track-Dateien haben sich während der Finalisierung geändert: {items}",
    "Track files changed during finalization: {items}"
  ],
  [
    "Registrierung des externen Zeitstempels in der Datenbank fehlgeschlagen ({error}); auch das Bereinigen der Staging-Daten ist fehlgeschlagen: {error}.",
    "External timestamp database registration failed ({error}); staging cleanup also failed ({error})."
  ],
  [
    "Veröffentlichung des externen Zeitstempels fehlgeschlagen ({error}); auch das Bereinigen des Dateisystems ist fehlgeschlagen: {error}. Der registrierte Datensatz wurde zur Wiederherstellung beibehalten.",
    "External timestamp publication failed ({error}); filesystem cleanup also failed ({error}). The registered record was retained for recovery."
  ],
  [
    "Veröffentlichung des externen Zeitstempels fehlgeschlagen ({error}); auch das Zurücksetzen der Datenbank ist fehlgeschlagen: {error}. Der registrierte fehlgeschlagene Datensatz bleibt sichtbar.",
    "External timestamp publication failed ({error}); database rollback also failed ({error}). The registered failed record remains visible."
  ],
  ["Zertifikat vom Benutzer als ungültig markiert", "Certificate invalidated by the user"],
  [
    "Das Archiv der SHA256SUMS-Datei für die Revision konnte nicht verifiziert werden.",
    "The revision SHA256SUMS archive copy could not be verified."
  ],
  [
    "Revision fehlgeschlagen ({error}); Wiederherstellung der Audio-Prüfung fehlgeschlagen: {error}",
    "Revision failed ({error}); audio-screening rollback failed: {error}"
  ],
  [
    "Veröffentlichung des Revisionsarchivs fehlgeschlagen ({error}); Wiederherstellung der Audio-Prüfung fehlgeschlagen: {error}",
    "Revision archive publication failed ({error}); audio-screening rollback failed: {error}"
  ],
  [
    "Speichern der Revision in der Datenbank fehlgeschlagen ({error}); Wiederherstellung der Audio-Prüfung fehlgeschlagen: {error}",
    "Revision database save failed ({error}); audio-screening rollback failed: {error}"
  ],
  [
    "Importierte Legacy-Evidence ist ein mehrdeutiger doppelter Final-/Release-Kandidat; klassifiziere oder entferne sie ausdrücklich.",
    "Imported legacy evidence is an ambiguous duplicate final/release candidate; classify or remove it explicitly."
  ],
  [
    "Importierte Legacy-Evidence wurde nicht unabhängig verifiziert.",
    "Imported legacy evidence has not been independently verified."
  ],
  [
    "Wiederhergestellte nicht indexierte Track-Evidence wurde nicht unabhängig verifiziert.",
    "Recovered unindexed track evidence has not been independently verified."
  ],
  ["Der verwaltete Track-Pfad ist kein Verzeichnis: {path}", "Managed track path is not a directory: {path}"],
  [
    "Der Track {path} ist nicht im Albumordner {albumPath} gespeichert.",
    "Track {path} is not stored inside album folder {albumPath}."
  ],
  ["Der gespeicherte Track-Pfad enthält keinen Ordnernamen.", "Stored track path has no folder name."],
  ["Der verwaltete Release-Dateiname ist ungültig.", "Managed release file name is invalid."],
  [
    "Migration der Bibliothek fehlgeschlagen ({error}); Zurücksetzen des Ordners fehlgeschlagen: {error}",
    "Library migration failed ({error}); folder rollback failed: {error}"
  ],
  [
    "Aktualisierung der Release-Metadaten fehlgeschlagen ({error}); Zurücksetzen der Datei fehlgeschlagen: {error}",
    "Release metadata update failed ({error}); file rollback failed: {error}"
  ],
  [
    "Umbenennen des Release fehlgeschlagen ({error}); Zurücksetzen eines früheren Release fehlgeschlagen: {error}",
    "Release rename failed ({error}); earlier release rollback failed: {error}"
  ],
  ["{field}: {error}", "{field}: {error}"],
  ["{field}-Metadaten: {error}", "{field} metadata: {error}"],
  ["Audio-Prüfung vor dem Release: {error}", "Pre-release audio screening: {error}"],
  ["{id}: {status}", "{id}: {status}"],
  ["{field}: N/A benötigt eine Begründung", "{field}: N/A requires a reason"],
  [
    "03_DOCUMENTATION/SHA256SUMS.txt (ungültig oder nicht lesbar)",
    "03_DOCUMENTATION/SHA256SUMS.txt (invalid or unreadable)"
  ],
  ["Dokumentation nach der Finalisierung geändert", "Documentation changed after finalization"],
  [
    "Der Zeitstempel-Provider hat ein nicht unterstütztes Evidence-Format zurückgegeben.",
    "Timestamp provider returned an unsupported evidence format."
  ],
  [
    "Für diesen Track ist Workflow {id} {version} gespeichert, aber {currentId} {currentVersion} ist aktuell. Bewerte den Track ausdrücklich neu, bevor du Dokumente, Hashes oder ein Zertifikat erzeugst.",
    "Workflow {id} {version} is stored for this track, but {currentId} {currentVersion} is current. Re-evaluate the track explicitly before generating documents, hashes, or a certificate."
  ],
  ["Das Revisionszertifikat hat kein Archivverzeichnis.", "Revision certificate has no archive directory."],
  ["Das Live-Zertifikat hat keinen Track-Root.", "Live certificate has no track root."],
  [
    "Die Live- und archivierten technischen Dokumentations-PDFs stimmen nicht überein: {fileName}.",
    "The live and archived technical documentation PDFs do not match: {fileName}."
  ],
  ["PDF-Zurücksetzen fehlgeschlagen: {error}", "PDF rollback failed: {error}"],
  ["Zurücksetzen des Zertifikats fehlgeschlagen: {error}", "certificate rollback failed: {error}"],
  ["Wiederherstellung des Live-Verzeichnisses fehlgeschlagen: {error}", "live directory recovery failed: {error}"],
  ["Wiederherstellung der Revision fehlgeschlagen ({error}); {error}", "Revision recovery failed ({error}); {error}"],
  ["{field} ist keine reguläre Datei.", "{field} is not a regular file."],
  [
    "Entfernen fehlgeschlagen ({error}); Zurücksetzen der Datei fehlgeschlagen: {error}",
    "Removal failed ({error}); file rollback failed: {error}"
  ],
  [
    "Finalisierung fehlgeschlagen ({error}); Zurücksetzen des Zertifikatspfads fehlgeschlagen: {error}",
    "Finalization failed ({error}); certificate rollback path failed: {error}"
  ],
  [
    "Finalisierung fehlgeschlagen ({error}); Zurücksetzen des PDF-Pfads fehlgeschlagen: {error}",
    "Finalization failed ({error}); PDF rollback path failed: {error}"
  ],
  [
    "Finalisierung fehlgeschlagen ({error}); Bereinigen des PDFs fehlgeschlagen: {error}",
    "Finalization failed ({error}); PDF cleanup failed: {error}"
  ],
  [
    "Finalisierung fehlgeschlagen ({error}); Bereinigen des Zertifikats fehlgeschlagen: {error}",
    "Finalization failed ({error}); certificate cleanup failed: {error}"
  ],
  [
    "Finalisierung fehlgeschlagen ({error}); Wiederherstellung des leeren Zertifikatsverzeichnisses fehlgeschlagen: {error}",
    "Finalization failed ({error}); empty certificate directory recovery failed: {error}"
  ],
  ["Das Zurücksetzen des PDFs würde {path} überschreiben", "PDF rollback would overwrite {path}"],
  ["Bereinigen des Live-Zertifikats fehlgeschlagen: {error}", "live certificate cleanup failed: {error}"],
  ["Das Zurücksetzen des Zertifikats würde {path} überschreiben", "certificate rollback would overwrite {path}"],
  ["Validierung des Live-Zertifikats fehlgeschlagen: {error}", "live certificate validation failed: {error}"],
  ["Bereinigen des archivierten PDFs fehlgeschlagen: {error}", "archived PDF cleanup failed: {error}"],
  ["Bereinigen des bereitgestellten Zertifikats fehlgeschlagen: {error}", "staged certificate cleanup failed: {error}"],
  ["Bereinigen der Staging-Daten fehlgeschlagen: {error}", "staging cleanup failed: {error}"],
  ["Revision fehlgeschlagen ({error}); {error}", "Revision failed ({error}); {error}"],
  ["Das Audio-Prüfverzeichnis hat keinen Live-Parent.", "Audio-screening directory has no live parent."],
  ["{field} ist kein Verzeichnis.", "{field} is not a directory."],
  [
    "Staging der Revision fehlgeschlagen ({error}); Bereinigung fehlgeschlagen: {error}",
    "Revision staging failed ({error}); cleanup failed: {error}"
  ],
  ["Das technische Dokumentations-PDF", "The technical documentation PDF"],
  ["Das Audio-Prüfverzeichnis vor dem Release", "The pre-release audio-screening directory"],
  ["Das archivierte technische Dokumentations-PDF", "The archived technical documentation PDF"],
  ["Das Live-technische-Dokumentations-PDF", "The live technical documentation PDF"],
  ["Beginn des Abo-Abdeckungszeitraums", "Subscription coverage start"],
  ["{field}-Beginn", "{field} start"],
  ["{field}-Ende", "{field} end"],
  // Native evidence and artwork diagnostics not covered by the action wrappers
  ["Der Evidence-Dateiname ist ungültig.", "Evidence file name is invalid."],
  [
    "Der ausgewählte Evidence-Ersatz stimmt nicht mit der vorhandenen Rolle überein.",
    "The selected evidence replacement does not match the existing role."
  ],
  ["Der vorhandene Evidence-Pfad enthält keinen Dateinamen.", "The existing evidence path has no file name."],
  [
    "Nur Abo-/Zahlungsnachweise, Suno-Terms-/Rights-Evidence oder andere wiederverwendbare Evidence dürfen global registriert werden.",
    "Only subscription/payment, Suno terms/rights, or other reusable evidence may be registered globally."
  ],
  [
    "Die registrierte globale Evidence hat sich geändert und kann nicht angehängt werden.",
    "The registered global evidence has changed and cannot be attached."
  ],
  ["Der Dateiname der globalen Evidence ist ungültig.", "Global evidence file name is invalid."],
  ["Die globale Evidence-ID ist ungültig.", "Global evidence ID is invalid."],
  ["Der Evidence-Pfad ist keine reguläre Datei.", "Evidence path is not a regular file."],
  [
    "Die Größe der Evidence-Datei hat sich geändert; führe die Verifikation erneut aus.",
    "Evidence file size changed; run verification again."
  ],
  [
    "Indexierte Track-Evidence erfordert eine ausdrückliche Verifikation.",
    "Indexed track evidence requires explicit verification."
  ],
  ["Der Evidence-Dateiname enthält unsichere Zeichen.", "Evidence file name contains unsafe characters."],
  [
    "Die Abmessungen des finalen Artworks sind für die Track-Cover-Vorschau nicht sicher.",
    "The final artwork dimensions are not safe for the track cover preview."
  ],
  ["Die Quelle des KI-Artworks hat sich nach dem Import geändert.", "The AI artwork source changed after import."],
  ["Das Artwork ist für den Hinweistext zu klein.", "Artwork is too small for the disclosure text."],
  [
    "Das Zeichen '{character}' im Hinweis wird vom lokalen Renderer nicht unterstützt.",
    "Disclosure character '{character}' is not supported by the local renderer."
  ],
  ["Leere SHA256SUMS-Zeile {line}.", "Empty SHA256SUMS line {line}."],
  [
    "Ungültiger oder ausgeschlossener SHA256SUMS-Pfad in Zeile {line}.",
    "Invalid or excluded SHA256SUMS path on line {line}."
  ],
  // Native RFC-3161 parser, publication, and sidecar-integrity diagnostics
  ["Ungültiger SHA-256-Digest.", "Invalid SHA-256 digest."],
  [
    "OpenTimestamps-Detached-Proofs benötigen einen SHA-256-Digest.",
    "OpenTimestamps detached proofs require a SHA-256 digest."
  ],
  [
    "Der zurückgegebene Message Imprint stimmt nicht mit dem angeforderten SHA-256-Digest überein",
    "the returned message imprint does not match the requested SHA-256 digest"
  ],
  [
    "Die zurückgegebene Nonce stimmt nicht mit der Anfrage-Nonce überein",
    "the returned nonce does not match the request nonce"
  ],
  [
    "Die zurückgegebene Policy-OID stimmt nicht mit der angeforderten Policy überein",
    "the returned policy OID does not match the requested policy"
  ],
  ["DER-Länge läuft über.", "DER length overflows."],
  ["DER-Länge ist nicht kanonisch.", "DER length is not canonical."],
  ["DER-Elementlänge läuft über.", "DER element length overflows."],
  ["Ein Objektbezeichner wurde erwartet.", "Expected an object identifier."],
  ["Objektbezeichner ist abgeschnitten.", "Object identifier is truncated."],
  ["Objektbezeichner ist zu groß.", "Object identifier is too large."]
];
