import type { Translation } from "./catalog-types";

export const SUPPLEMENTAL_TRANSLATIONS_01: readonly Translation[] = [
  ["Wähle genau eine passende Option aus.", "Select exactly one suitable option."],
  ["Noch nicht", "Not yet"],
  ["Nicht dokumentiert", "Not documented"],
  ["Nutzerangabe", "User-provided information"],
  ["Automatisch aus Suno-WAV erkannt", "Automatically detected from Suno WAV"],
  ["Evidence-derived metadata", "Evidence-derived metadata"],
  ["Erkannt", "Detected"],
  ["Verifiziert", "Verified"],
  ["Nicht verifiziert", "Not verified"],
  ["Gehasht", "Hashed"],
  ["Vollständig", "Complete"],
  ["Ausstehend", "Pending"],
  ["Ausstehend oder veraltet", "Pending or outdated"],
  ["Gelöst", "Resolved"],
  ["Erfasst", "Recorded"],
  ["Zugeordnet", "Assigned"],
  ["Diesem Track zuordnen", "Assign to this track"],
  ["Diesem Projekt zuordnen", "Assign to this project"],
  ["Nicht passend", "Not applicable"],
  ["Im Projekt hinterlegt", "Stored in project"],
  ["Provider nicht dokumentiert", "Provider not documented"],
  ["Kernmetadaten vollständig", "Core metadata complete"],
  ["Noch nicht getestet.", "Not tested yet."],
  ["Zuletzt geprüft:", "Last checked:"],
  ["Erzeugt:", "Generated:"],
  ["Geprüft:", "Checked:"],
  ["Rolle", "Role"],
  ["Größe", "Size"],
  ["Pfad", "Path"],
  ["Finalisiert", "Finalized"],
  ["Finalisierung", "Finalization"],
  ["Evidence-Dateien", "Evidence files"],
  ["Blockierende Abweichungen", "Blocking deviations"],
  ["Automatische Konsistenz", "Automatic consistency"],
  ["Finales Ergebnis", "Final result"],
  ["Bedeutung", "Meaning"],
  ["Datei", "File"],
  ["Tracks", "Tracks"],
  ["Alben", "Albums"],
  ["Track-Ordner", "Track folders"],
  ["indexierte Tracks", "indexed tracks"],
  ["zuletzt gescannt", "last scanned"],
  ["Lokal", "Local"],
  ["SQLite + Track-Ordner", "SQLite + track folders"],
  ["Status:", "Status:"],
  ["Verbindung testen", "Test connection"],
  ["Nicht beantwortet", "Not answered"],
  ["NOCH NICHT ERFASST", "NOT RECORDED"],
  ["WIRD ANGEFORDERT", "REQUESTING"],
  ["ANGEHÄNGT", "ATTACHED"],
  ["VERIFIZIERT", "VERIFIED"],
  ["Verifizierung fehlgeschlagen", "Verification failed"],
  ["Provider nicht verfügbar", "Provider unavailable"],
  ["Authentifizierung fehlgeschlagen", "Authentication failed"],
  ["Trust Anchor stimmt nicht überein", "Anchor mismatch"],
  ["Deaktiviert", "Disabled"],
  ["Bereit", "Ready"],
  ["Nicht konfiguriert", "Not configured"],
  ["Konfiguration unvollständig", "Configuration incomplete"],
  ["Authentifizierung erforderlich", "Authentication required"],
  ["Verbindung fehlgeschlagen", "Connection failed"],
  ["Providerfehler", "Provider error"],
  ["Nicht unterstützte Antwort", "Unsupported response"],
  ["Verifizierungskonfiguration unvollständig", "Verification configuration incomplete"],
  ["VERIFIKATION FEHLGESCHLAGEN", "VERIFICATION FAILED"],
  ["PROVIDER NICHT VERFÜGBAR", "PROVIDER UNAVAILABLE"],
  ["AUTHENTIFIZIERUNG FEHLGESCHLAGEN", "AUTHENTICATION FAILED"],
  ["ANCHOR-ABWEICHUNG", "ANCHOR MISMATCH"],
  ["DEAKTIVIERT", "DISABLED"],
  ["BEREIT", "READY"],
  ["KONFIGURATION UNVOLLSTÄNDIG", "CONFIGURATION INCOMPLETE"],
  ["AUTHENTIFIZIERUNG ERFORDERLICH", "AUTHENTICATION REQUIRED"],
  ["VERBINDUNG FEHLGESCHLAGEN", "CONNECTION FAILED"],
  ["ANTWORT NICHT UNTERSTÜTZT", "UNSUPPORTED RESPONSE"],
  ["VERIFIKATIONSKONFIGURATION UNVOLLSTÄNDIG", "VERIFICATION CONFIGURATION INCOMPLETE"],
  ["NICHT GEPRÜFT", "NOT CHECKED"],
  ["PROVIDERIDENTITÄT VERIFIZIERT", "PROVIDER IDENTITY VERIFIED"],
  ["VERTRAUENSDIENST VERIFIZIERT", "TRUST SERVICE VERIFIED"],
  ["QUALIFIZIERTER DIENST VERIFIZIERT", "QUALIFIED SERVICE VERIFIED"],
  ["PRÜFUNG FEHLGESCHLAGEN", "CHECK FAILED"],
  ["eIDAS-QUALIFIZIERTER VERTRAUENSDIENST – VERIFIZIERT", "eIDAS QUALIFIED TRUST SERVICE – VERIFIED"],
  ["NICHT AUSGEFÜHRT", "NOT RUN"],
  ["FINGERPRINT ERZEUGT", "FINGERPRINT GENERATED"],
  ["KEIN MATCH ERKANNT", "NO MATCH DETECTED"],
  ["MATCH ERKANNT", "MATCH DETECTED"],
  ["ÜBERSPRUNGEN – NICHT KONFIGURIERT", "SKIPPED – NOT CONFIGURED"],
  ["KONFIGURATION UNGÜLTIG", "CONFIGURATION INVALID"],
  ["ENGINE NICHT VERFÜGBAR", "ENGINE UNAVAILABLE"],
  ["FORMAT NICHT UNTERSTÜTZT", "UNSUPPORTED FORMAT"],
  ["VERARBEITUNG FEHLGESCHLAGEN", "PROCESSING FAILED"],
  ["VERALTET", "STALE"],
  ["NICHT KONFIGURIERT", "NOT CONFIGURED"],
  ["JA", "YES"],
  ["NEIN", "NO"],
  ["NICHT DOKUMENTIERT", "NOT DOCUMENTED"],
  ["NICHT BEANTWORTET", "NOT ANSWERED"],
  ["NICHT VERIFIZIERT", "NOT VERIFIED"],
  ["DOKUMENTATION VOLLSTÄNDIG", "DOCUMENTATION COMPLETE"],
  ["UNGÜLTIG", "INVALID"],
  ["Konfigurierte Dokumentationsanforderungen sind erfüllt", "configured documentation requirements completed"],
  [
    "Nachweis zu Nutzungsbedingungen vorhanden, aber beschreibende Metadaten sind unvollständig.",
    "Terms evidence exists, but descriptive metadata is incomplete."
  ],
  ["Quell-URL: Nicht dokumentiert", "Source URL: Not documented"],
  ["Importiert am", "Imported at"],
  ["Zertifikats-ID", "Certificate ID"],
  ["Bedeutung", "Meaning"],
  ["Eingebetteter Suno-Exportzeitstempel", "Embedded Suno export timestamp"],
  ["OpenTimestamps — nicht RFC 3161", "OpenTimestamps — not RFC 3161"],
  [
    "SHA-256-Detached-Proof; ein erster Nachweis bleibt ATTACHED, bis OpenTimestamps-Upgrade und -Verifikation die Bitcoin-Verankerung bestätigen. CMS- und TSA-Trust-Chain-Prüfungen sind nicht anwendbar.",
    "SHA-256 detached proof; an initial proof remains ATTACHED until OpenTimestamps upgrade and verification confirm the Bitcoin anchoring. CMS and TSA trust-chain checks do not apply."
  ],
  [
    "Signierter TimeStampResp; VERIFIED erfordert CMS-, Nonce-, Policy-, EKU- und Vertrauensketteprüfung gegen den ausdrücklich gewählten TSA Trust Anchor.",
    "Signed TimeStampResp; VERIFIED requires CMS, nonce, policy, EKU, and trust-chain verification against the explicitly selected TSA trust anchor."
  ],
  ["Kein Timestamp-Protokoll aktiv", "No timestamp protocol active"],
  ["Es wird kein externer Zeitstempelnachweis angefordert.", "No external timestamp evidence is requested."],
  ["Bestätigter Zeitstempel", "Confirmed timestamp"],
  ["Zeitstempel", "Timestamp"],
  [
    "AUSSTEHEND — OpenTimestamps-Verifizierung / Upgrade erforderlich",
    "PENDING — OpenTimestamps verification / upgrade required"
  ],
  ["Lokale Manifest-/Proof-Bindung", "Local manifest / proof binding"],
  ["Provider-Digest stimmt überein", "Provider digest match"],
  ["N/V — nicht RFC 3161", "N/A — not RFC 3161"],
  ["Kalender-Endpunkt", "Calendar endpoint"],
  ["Provider-Verifizierungs-URL", "Provider verification URL"],
  ["Workspace auswählen", "Choose workspace"],
  ["Neuen Workspace anlegen", "Create new workspace"],
  ["Globalen Nachweis registrieren", "Register global evidence"],
  ["Unterstützte Nachweise", "Supported evidence"],
  ["Suno-Nutzungsbedingungen als PDF auswählen", "Select Suno terms of use PDF"],
  ["Musikprojekt-Ordner importieren", "Import music-project folder"],
  ["Evidence importieren", "Import evidence"],
  ["Name des neuen Albumordners:", "Name of the new album folder:"],
  ["Neuer Name des Albumordners:", "New album folder name:"],
  [
    "Historisch indexierte Evidence entfernen? Die Datei wird nachvollziehbar unter .archive/removals gesichert und nicht gelöscht.",
    "Remove historically indexed evidence? The file is preserved traceably under .archive/removals and not deleted."
  ],
  [
    "Importierte Evidence-Kopie aus dem Track entfernen? Die Originaldatei am Quellort bleibt erhalten.",
    "Remove the imported evidence copy from the track? The original file at the source remains intact."
  ],
  [
    "Global registrierten Nachweis entfernen? Bereits in Tracks kopierte Evidence bleibt bestehen.",
    "Remove globally registered evidence? Evidence copies already assigned to tracks remain in place."
  ],
  [
    "Warum ist dieser Schritt für den Track nicht anwendbar? Eine konkrete Begründung ist erforderlich.",
    "Why is this step not applicable to the track? A specific reason is required."
  ],
  [
    "Historische Lyrics-Angaben aus diesem bearbeitbaren Track entfernen? Die neuen Lyrics-/Structure-Felder bleiben unverändert.",
    "Remove historical lyrics information from this editable track? The new lyrics and structure fields remain unchanged."
  ],
  [
    "Treffen die aktuellen Workspace-Stammdaten auf diesen historischen Track zu? Sie werden als Track-Snapshot übernommen.",
    "Do the current workspace master data apply to this historical track? They will be adopted as the track snapshot."
  ],
  [
    "Zertifikat als ungültig markieren? Der finalisierte Snapshot wird nicht still überschrieben.",
    "Mark the certificate as invalid? The finalized snapshot will not be silently overwritten."
  ],
  [
    "Neue Revision anlegen? Der bisherige Certificate-/Manifest-Snapshot wird zuerst unter .archive/revisions gesichert.",
    "Create a new revision? The existing certificate and manifest snapshot will first be preserved under .archive/revisions."
  ],
  [
    "Track mit dem aktuellen Workflow neu bewerten? Ein finalisierter Snapshot wird zuerst unverändert als Revision archiviert; Dokumente, Prüfsummen und Zertifikat müssen danach neu erzeugt werden.",
    "Re-evaluate the track with the current workflow? A finalized snapshot will first be archived unchanged as a revision; documents, checksums, and certificate must then be generated again."
  ],
  [
    "Vorhandene Evidence durch die neu ausgewählte Datei ersetzen? Die bisherige verwaltete Kopie wird lokal archiviert.",
    "Replace existing evidence with the newly selected file? The previous managed copy will be archived locally."
  ],
  ["Rolle der Evidence wählen:", "Choose evidence role:"],
  ["Nummer eingeben:", "Enter number:"],
  ["Abweichung sachlich beschreiben:", "Describe the deviation factually:"],
  ["Soll diese Abweichung die Finalisierung blockieren?", "Should this deviation block finalization?"],
  ["Bestehende verwaltete Dokumente erkannt:", "Existing managed documents detected:"],
  [
    "Die native Anwendung sichert den vorhandenen Zustand unter .archive, bevor neue verwaltete Dokumente geschrieben werden. Fortfahren?",
    "The native application preserves the existing state under .archive before writing new managed documents. Continue?"
  ],
  [
    "Der Albumtitel darf höchstens 200 Zeichen und keine Pfadtrenner, Steuerzeichen oder reservierten Ordnernamen enthalten.",
    "The album title may contain at most 200 characters and must not contain path separators, control characters, or reserved folder names."
  ],
  ["Gib für einen Album-Track einen Albumtitel an.", "Enter an album title for an album track."],
  ["Bisherige Auswahl:", "Previous selection:"],
  ["Bisheriger Freitext:", "Previous free text:"],
  ["Bisheriger Wert:", "Previous value:"],
  ["(bitte prüfen)", "(please review)"],
  [
    "Wähle mindestens einen tatsächlich ausgeführten Schritt aus.",
    "Select at least one step that was actually performed."
  ],
  [
    "Mehrere Angaben können gleichzeitig ausgewählt und durch Freitext ergänzt werden.",
    "Several items can be selected at the same time and supplemented with free text."
  ],
  ["Eine Datei ist bereits vorhanden; prüfe ihre Metadaten.", "A file is already present; check its metadata."],
  ["vollständig", "complete"],
  [
    "ACRCloud ist nicht aktiviert; es wurde keine externe Katalogprüfung gestartet.",
    "ACRCloud is not enabled; no external catalog check was started."
  ],
  ["ACRCloud-Konfiguration wird geprüft …", "Checking ACRCloud configuration …"],
  ["ACRCloud-Prüfung wird vorbereitet …", "Preparing ACRCloud check …"],
  [
    "ACRCloud-Zugangsdaten fehlen; es wurde keine externe Katalogprüfung gestartet.",
    "ACRCloud credentials are missing; no external catalog check was started."
  ],
  ["Abweichung gelöst", "Deviation resolved"],
  ["Abweichung wird gelöst …", "Resolving deviation …"],
  ["Alle Pflichtpunkte sind erfüllt.", "All required items are complete."],
  ["Ausschließlich eigene Rechte", "Exclusively own rights"],
  ["Ausschnitt gewählt", "Excerpt selected"],
  [
    "Bearbeitbare Projektkopien wurden aktualisiert; finalisierte Snapshots bleiben unverändert.",
    "Editable project copies were updated; finalized snapshots remain unchanged."
  ],
  [
    "Bereits zugeordnete Track-Kopien wurden nicht verändert.",
    "Track copies that were already assigned were not changed."
  ],
  ["Bestehende Dateien wurden nicht verändert.", "Existing files were not changed."],
  [
    "Bestätige ausdrücklich oder korrigiere den dokumentierten Titel. Der Titel wird niemals aus dem Dateinamen abgeleitet.",
    "Explicitly confirm or correct the documented title. The title is never derived from the file name."
  ],
  [
    "Das bisherige Zertifikat ist ungültig. Verwende die Revisionsaktion oben, um die Abweichung in einer neuen Folgeversion zu bearbeiten.",
    "The existing certificate is invalid. Use the revision action above to address the deviation in a new successor version."
  ],
  [
    "Das externe Katalogergebnis ist nicht mehr an die aktuelle finale Release-Datei gebunden. Starte die Prüfung bei Bedarf erneut.",
    "The external catalog result is no longer bound to the current final release file. Run the check again if needed."
  ],
  [
    "Datei auswählen; große Dateien werden im Hintergrund kopiert und gehasht …",
    "Select file; large files are copied and hashed in the background …"
  ],
  ["Dateien prüfen", "Check files"],
  [
    "Der Legacy-Hinweis wurde gelöscht; aktuelle Lyrics-/Structure-Angaben blieben unverändert.",
    "The legacy note was removed; current lyrics and structure information remained unchanged."
  ],
  [
    "Der Track ist technisch vollständig finalisiert. Ein externer Zeitstempel wurde für diesen Zertifikatssnapshot noch nicht hinterlegt.",
    "The track is technically finalized in full. No external timestamp has yet been recorded for this certificate snapshot."
  ],
  [
    "Der bisherige Snapshot bleibt erhalten. Lege eine neue Revision an, um Abweichungen zu bearbeiten und anschließend neu zu finalisieren.",
    "The existing snapshot is retained. Create a new revision to address deviations and finalize again."
  ],
  [
    "Der finalisierte Snapshot wurde nicht verändert. Lege zuerst eine neue Revision an.",
    "The finalized snapshot was not changed. Create a new revision first."
  ],
  [
    "Der lokale Fingerprint ist nicht mehr an die aktuelle finale Release-Datei gebunden. Führe die lokale Prüfung erneut aus.",
    "The local fingerprint is no longer bound to the current final release file. Run the local check again."
  ],
  [
    "Der zuletzt verwendete Workspace wurde nicht mehr gefunden. Bitte wähle einen anderen Workspace.",
    "The most recently used workspace could no longer be found. Please select another workspace."
  ],
  [
    "Die UI-Vorprüfung ist vollständig. Der native Dienst validiert vor dem Erzeugen des Zertifikats nochmals alle Pflichtschritte, Evidence und Hashes.",
    "The UI pre-check is complete. Before generating the certificate, the native service validates all required steps, evidence, and hashes again."
  ],
  [
    "Dieser Snapshot ist abgeschlossen und schreibgeschützt. Verwende die Revisionsaktion oben, um eine bearbeitbare Folgeversion anzulegen.",
    "This snapshot is complete and read-only. Use the revision action above to create an editable successor version."
  ],
  [
    "Dieser historische Snapshot bleibt unverändert. Öffne die aktuelle Revision, um Inhalte zu bearbeiten.",
    "This historical snapshot remains unchanged. Open the current revision to edit content."
  ],
  [
    "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt und bleibt unverändert. Navigation und Integritätsprüfungen sind weiterhin möglich.",
    "This historical snapshot was superseded by a newer revision and remains unchanged. Navigation and integrity checks remain available."
  ],
  [
    "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt. Navigation und reine Prüfungen bleiben verfügbar; der Snapshot selbst kann nicht erneut bearbeitet werden.",
    "This historical snapshot was superseded by a newer revision. Navigation and read-only checks remain available; the snapshot itself cannot be edited again."
  ],
  ["Effekte hinzugefügt", "Effects added"],
  ["Eigenständig gezeichnet", "Drawn independently"],
  ["Eigenständig illustriert", "Illustrated independently"],
  ["Elemente hinzugefügt", "Elements added"],
  ["Evidence wird geprüft …", "Verifying evidence …"],
  ["Externer Zeitstempel angehängt", "External timestamp attached"],
  [
    "Externer Zeitstempel wird an den finalisierten Manifest-Anchor angehängt …",
    "Attaching external timestamp to the finalized manifest anchor …"
  ],
  ["Finaler Audioinhalt enthält Gesang?", "Does the final audio contain vocals?"],
  ["Finalisierungs-Gate wird nativ geprüft …", "Checking finalization gate natively …"],
  ["Frei beschreibbare zusätzliche Änderung", "Freely describable additional change"],
  ["Für diese Datei ist keine Vorschau verfügbar.", "No preview is available for this file."],
  ["Gespeicherter Workspace nicht verfügbar", "Saved workspace unavailable"],
  ["Gespeicherter Workspace konnte nicht geladen werden: {message}", "Saved workspace could not be loaded: {message}"],
  ["Hintergrund verändert", "Background changed"],
  ["Kein Track ausgewählt", "No track selected"],
  ["Lautstärke angepasst", "Volume adjusted"],
  ["Legacy-Snapshot übernommen", "Legacy snapshot adopted"],
  [
    "Lege eine neue Revision an, bevor du Angaben, Nachweise oder erzeugte Dokumente änderst. Die Navigation und reine Prüfungen bleiben verfügbar.",
    "Create a new revision before changing information, evidence, or generated documents. Navigation and read-only checks remain available."
  ],
  ["Lizenz für kommerzielle Nutzung", "License for commercial use"],
  ["Logo/Titel hinzugefügt", "Logo/title added"],
  ["Lokaler PEM- oder DER-Pfad (erforderlich für VERIFIED)", "Local PEM or DER path (required for VERIFIED)"],
  [
    "Manuelle Angabe; wird nur bei leerem Feld aus gültigen Suno-WAV-Metadaten ergänzt.",
    "Manual entry; only supplemented from valid Suno WAV metadata when the field is empty."
  ],
  ["Motiv ausgewählt", "Motif selected"],
  ["N/A-Begründung wird gespeichert …", "Saving N/A reason …"],
  ["Nutzungsbedingungen auswählen und registrieren …", "Selecting and registering terms of use …"],
  [
    "Offene Tracks wurden aktualisiert; finalisierte Track-Snapshots bleiben unverändert.",
    "Open tracks were updated; finalized track snapshots remain unchanged."
  ],
  ["Ordner wird in normale Track-Strukturen übernommen …", "Adopting folder into standard track structures …"],
  ["Ordnerdialog wird geöffnet …", "Opening folder dialog …"],
  ["Prüfung abgeschlossen", "Check complete"],
  ["RFC-3161-Zeitstempel zusätzlich anhängen", "Attach additional RFC 3161 timestamp"],
  ["Schrittstatus wird zurückgesetzt …", "Resetting step status …"],
  ["Schrittstatus zurückgesetzt", "Step status reset"],
  ["Stammdaten werden als Legacy-Snapshot übernommen …", "Adopting master data as legacy snapshot …"],
  ["Startdatum ungültig", "Invalid start date"],
  ["Suno-Instrumentalmodus ausgewählt?", "Suno instrumental mode selected?"],
  ["Suno-Metadaten überschreiben Nutzerangabe", "Suno metadata overrides user entry"],
  ["Terms-Metadaten unvollständig", "Terms metadata incomplete"],
  ["Timestamp-Provider wird geprüft …", "Checking timestamp provider …"],
  ["Tracks öffnen", "Open tracks"],
  ["Typografie hinzugefügt", "Typography added"],
  ["Ungültige Rolle", "Invalid role"],
  ["Unveränderlicher Snapshot und Zertifikat werden erzeugt …", "Generating immutable snapshot and certificate …"],
  [
    "Vor der Finalisierung muss der Track ausdrücklich mit dem aktuellen Workflow neu bewertet werden.",
    "Before finalization, the track must be explicitly re-evaluated with the current workflow."
  ],
  ["Vorhanden – klicken für Vorschau", "Available – click to preview"],
  ["Vorhanden, aber nicht verifiziert – klicken für Vorschau", "Available but not verified – click to preview"],
  ["Wähle eine Nummer aus der angezeigten Liste.", "Select a number from the displayed list."],
  ["Wähle einen Track aus deiner Bibliothek.", "Select a track from your library."],
  ["Wähle monatliche oder jährliche Zahlung aus.", "Select monthly or annual payment."],
  [
    "Wähle zuerst die explizite Neubewertung mit dem aktuellen Workflow. Danach müssen Dokumente und Prüfsummen erneut erzeugt werden.",
    "First choose explicit re-evaluation with the current workflow. Documents and checksums must then be generated again."
  ],
  ["Wähle zuerst einen Track aus.", "Select a track first."],
  ["Historischer Snapshot – schreibgeschützt", "Historical snapshot – read-only"],
  [
    "Mehrere lückenlos anschließende Abrechnungszeiträume werden gemeinsam gewertet. Dies ist ausschließlich ein Datumsabgleich, keine Rechteaussage.",
    "Multiple contiguous billing periods are evaluated together. This is only a date comparison, not a rights statement."
  ],
  ["3. Prüfsummen verifizieren", "3. Verify checksums"],
  [
    "Alle relevanten Dateien in einer extern prüfbaren Hashliste erfassen.",
    "Record all relevant files in an externally verifiable hash list."
  ],
  [
    "Beim Speichern wird der vollständige Track-Ordner sicher in den gewählten Album- oder Singles-Ordner verschoben. Dateien, interne Prüfsummen und Zertifikat bleiben dabei unverändert.",
    "When saving, the complete track folder is safely moved into the selected album or singles folder. Files, internal checksums, and certificate remain unchanged."
  ],
  [
    "Chromaprint ist ein akustischer Fingerprint und wird getrennt von der SHA-256-Dateiintegrität dargestellt. ACRCloud bleibt eine bewusste, optionale externe Prüfung.",
    "Chromaprint is an acoustic fingerprint and is shown separately from SHA-256 file integrity. ACRCloud remains a deliberate, optional external check."
  ],
  [
    "Das Zertifikat bestätigt ausschließlich den Abschluss des konfigurierten Dokumentations- und Integritätsworkflows. Es ist keine behördliche Zertifizierung, Rechtsberatung oder unabhängige Feststellung von Urheberschaft oder Rechtskonformität.",
    "The certificate confirms only completion of the configured documentation and integrity workflow. It is not governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance."
  ],
  [
    "Der Scan erkennt bekannte Ordner, Evidence und Hashlisten. Bestehende Dateien werden dabei niemals verändert.",
    "The scan detects known folders, evidence, and hash lists. Existing files are never changed."
  ],
  [
    "Der Scan hat keine fehlenden Fakten erfunden. Übernimm die aktuellen Workspace-Stammdaten nur, wenn sie für diesen Track tatsächlich zutreffen; danach kannst du weitere Angaben prüfen und speichern.",
    "The scan did not invent missing facts. Adopt the current workspace master data only if they actually apply to this track; you can then review and save additional information."
  ],
  [
    "Das finale Zertifikat hält den tatsächlichen Timestamp- und Qualification-Zustand seiner Erzeugung fest. Spätere Wiederholungen ergänzen unveränderliche Addenda, ohne das PDF umzuschreiben.",
    "The final certificate records the actual timestamp and qualification state at creation. Later retries add immutable addenda without rewriting the PDF."
  ],
  [
    "Der externe Anbieter hat eine Audio-Übereinstimmung gemeldet. Prüfe den Treffer vor Veröffentlichung.",
    "The external provider reported an audio match. Review the match before release."
  ],
  [
    "Der lokale Zertifikatssatz wurde erzeugt und verifiziert. PASS bedeutet ausschließlich: Configured documentation requirements for this step were satisfied. Dies ist keine behördliche oder rechtliche Zertifizierung.",
    "The local certificate set was generated and verified. PASS means only: Configured documentation requirements for this step were satisfied. This is not governmental or legal certification."
  ],
  [
    "Der Manifest-Anchor wird zuerst festgelegt; anschließend wird der automatische Provider- und Qualification-Versuch ausgewertet und erst danach das finale PDF genau einmal gerendert. Spätere Wiederholungen bleiben separate Addenda.",
    "The manifest anchor is fixed first; the automatic provider and qualification attempt is then evaluated, and only afterward is the final PDF rendered exactly once. Later retries remain separate addenda."
  ],
  [
    "Dieser unverändert erhaltene Altwert füllt „Suno-Tarif bei der finalen Generation“ nicht aus und erfüllt die aktuelle Workflow-Anforderung nicht.",
    "This retained legacy value does not fill in 'Suno plan at final generation' and does not meet the current workflow requirement."
  ],
  [
    "Dokumenttitel, Provider und Abrufdatum sind Kernmetadaten. Die lokale PDF und ihre Metadaten werden in jedes neue sowie jedes noch bearbeitbare Projekt kopiert. Finalisierte Snapshots bleiben unverändert.",
    "Document title, provider, and retrieval date are core metadata. The local PDF and its metadata are copied into every new and still editable project. Finalized snapshots remain unchanged."
  ],
  [
    "Extern: ÜBERSPRUNGEN – kein ACRCloud-Zugang eingerichtet. Die lokale Chromaprint-Prüfung bleibt davon unabhängig.",
    "External: SKIPPED – no ACRCloud credentials configured. The local Chromaprint check remains independent."
  ],
  [
    "Hashliste erneut lesen und jede erfasste Datei nativ überprüfen.",
    "Read the hash list again and verify every recorded file natively."
  ],
  ["Historische Stammdaten ausdrücklich bestätigen", "Explicitly confirm historical master data"],
  ["Integrität", "Integrity"],
  ["Ja oder Nein auswählen", "Select Yes or No"],
  ["Kanäle", "Channels"],
  ["PASS / Erfüllt:", "PASS / Complete:"],
  [
    "Provider-, Authentifizierungs- und Policy-Angaben für den eigenen RFC-3161-Dienst. Der Trust Anchor ist für den Status VERIFIED erforderlich.",
    "Provider, authentication, and policy details for your RFC 3161 service. The trust anchor is required for VERIFIED status."
  ],
  ["SHA256SUMS.txt bleibt möglichst mit", "SHA256SUMS.txt remains compatible where possible with"],
  [
    "Source URL, Effective Date, anwendbarer Produktionszeitraum und sachliche Notiz sind optional. SunoDM trifft keine Rechte- oder Gültigkeitsaussage.",
    "Source URL, effective date, applicable production period, and factual note are optional. SunoDM makes no statement about rights or validity."
  ],
  ["Stammdaten als Snapshot bestätigen", "Confirm master data as snapshot"],
  ["Unabhängig prüfbar.", "Independently verifiable."],
  [
    "VERIFIED wird nur nach CMS-Signatur-, Nonce-, Policy-, EKU-, Gültigkeits- und Vertrauensketteprüfung gegen diesen ausdrücklich gewählten Trust Anchor vergeben.",
    "VERIFIED is assigned only after CMS signature, nonce, policy, EKU, validity, and trust-chain verification against this explicitly selected trust anchor."
  ],
  ["Zertifikat schließen", "Close certificate"],
  ["Zwölf Kalendermonate ab dem Startdatum", "Twelve calendar months from the start date"],
  [
    "Übernimm den tatsächlichen Beginn vom Beleg. Das Enddatum wird bis zum Tag vor der nächsten Zahlung berechnet; der Inhalt der Datei wird nicht automatisch ausgelesen. Pro Registrierung wird genau eine Rechnung oder ein Beleg ausgewählt.",
    "Use the actual start date from the evidence. The end date is calculated through the day before the next payment; the file content is not read automatically. Exactly one invoice or evidence item is selected per registration."
  ],
  [
    "„Generative AI used“ wurde ausdrücklich mit NO dokumentiert.",
    "'Generative AI used' was explicitly documented as NO."
  ],
  ["Historischer Status: Terms evidence not available", "Historical status: Terms evidence not available"],
  ["Kein externer Timestamp-Dienst eingerichtet.", "No external timestamp service configured."],
  [
    "Manuell erfasster historischer Zeitstempelnachweis ist angehängt und wurde nicht automatisch als verifiziert eingestuft.",
    "Legacy manually recorded timestamp evidence is attached and has not been automatically promoted to verified."
  ],
  [
    "Eine Zeitstempelantwort ist angehängt; prüfe ihre technischen Verifizierungsdetails.",
    "A timestamp response is attached; review its technical verification details."
  ],
  [
    "Qualifizierter elektronischer Zeitstempel – vom Nutzer angegeben",
    "Qualified electronic timestamp – user declared"
  ],
  ["Elektronischer Zeitstempel", "Electronic timestamp"],
  ["Externer Integritätszeitstempel", "External integrity timestamp"],
  ["Sonstiges", "Other"],
  ["Zertifikat-PDF (Englisch)", "Certificate PDF (English)"],
  ["Finales Evidence-Paket", "Final Evidence Package"],
  ["Abschlusszertifikat für die Track-Dokumentation", "Track Documentation Completion Certificate"],
  [
    "Die konfigurierten Dokumentationsanforderungen für diesen Schritt wurden erfüllt.",
    "Configured documentation requirements for this step were satisfied."
  ],
  [
    "PASS bedeutet: Die konfigurierten Dokumentationsanforderungen für diesen Schritt wurden erfüllt. Dieses Zertifikat bestätigt den Abschluss des konfigurierten Dokumentationsworkflows und der Integritätsprüfungen. Es ist keine behördliche Zertifizierung, Rechtsberatung oder unabhängige Feststellung von Urheberschaft oder Rechtskonformität.",
    "PASS means: Configured documentation requirements for this step were satisfied. This certificate confirms completion of the configured documentation workflow and integrity checks. It does not constitute governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance."
  ],
  [
    "Dies dokumentiert technische externe Zeitstempelnachweise. Suno Documentation Manager trifft keine rechtliche Einordnung des Zeitstempels.",
    "This records technical external timestamp evidence. No legal qualification of the timestamp is determined by Suno Documentation Manager."
  ],
  [
    "Für einen unveränderlichen Track-Snapshot fehlen: {items}.",
    "For an immutable track snapshot, the following are missing: {items}."
  ],
  ["Vervollständige zuerst: {items}.", "Complete these first: {items}."],
  ["Der Track liegt jetzt unter {path}.", "The track is now located under {path}."],
  [
    "{title} wurde erstellt. Tracks können diesem Album jetzt zugeordnet werden.",
    "{title} was created. Tracks can now be assigned to this album."
  ],
  ["{oldTitle} wurde in {newTitle} umbenannt.", "{oldTitle} was renamed to {newTitle}."],
  [
    "{count} {kind} erkannt. Nur eindeutige Dateien werden übernommen.",
    "{count} {kind} detected. Only unambiguous files will be adopted."
  ],
  [
    "{count} Track wurde als unvollständige normale SunoDM-Struktur angelegt.",
    "{count} track was created as an incomplete standard SunoDM structure."
  ],
  [
    "{count} Tracks wurden als unvollständige normale SunoDM-Struktur angelegt.",
    "{count} tracks were created as incomplete standard SunoDM structures."
  ],
  ["{count} Punkte müssen vor dem Abschluss geklärt werden.", "{count} items must be resolved before completion."],
  ["Gelöst {date}", "Resolved {date}"],
  ["Erfasst {date}", "Recorded {date}"],
  ["Geprüft: {date} · Sample: {sample}", "Checked: {date} · Sample: {sample}"],
  ["Zuletzt geprüft: {date}", "Last checked: {date}"],
  ["Erzeugt: {date}", "Generated: {date}"],
  ["Tatsächlicher Dateiname: {fileName}", "Actual file name: {fileName}"],
  ["{label}: tatsächlicher Quelldateiname nicht erfasst", "{label}: actual source file name not recorded"],
  ["{label} passt zum dokumentierten Titel", "{label} matches the documented title"],
  ["{role} wurde kopiert, gehasht und dem Track zugeordnet.", "{role} was copied, hashed, and assigned to the track."],
  [
    "{count} Track-Ordner erkannt. Es wurden keine bestehenden Dateien überschrieben.",
    "{count} track folders detected. No existing files were overwritten."
  ],
  [
    "Finalisiert mit Workflow {previous} / Aktueller Workflow {current}",
    "Finalized with workflow {previous} / Current workflow {current}"
  ],
  [
    "Ersetzter Snapshot verwendet Workflow {previous} / Aktueller Workflow {current}",
    "Superseded snapshot uses workflow {previous} / Current workflow {current}"
  ],
  [
    "Track verwendet Workflow {previous} / Aktueller Workflow {current}",
    "Track uses workflow {previous} / Current workflow {current}"
  ],
  ["Album {title} umbenennen", "Rename album {title}"]
];
