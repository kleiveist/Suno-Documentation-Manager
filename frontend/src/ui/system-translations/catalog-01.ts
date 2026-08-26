import type { SystemTranslation } from "./catalog-types";

export const SYSTEM_TRANSLATIONS_01: readonly SystemTranslation[] = [
  // workflows/suno-track.toml — requirement missing_message values
  ["Tracktitel fehlt.", "Track title is missing."],
  ["Produktionsbeginn fehlt.", "Production start date is missing."],
  ["Produktionsende fehlt.", "Production end date is missing."],
  ["Die beabsichtigte kommerzielle Nutzung ist nicht bestätigt.", "Intended commercial use has not been confirmed."],
  ["Der globale Künstlername fehlt.", "Global artist name is missing."],
  ["Der globale Suno-Profilname fehlt.", "Global Suno profile name is missing."],
  ["Der globale Suno-Handle fehlt.", "Global Suno handle is missing."],
  ["Der globale Suno-Tarif fehlt.", "Global Suno plan is missing."],
  ["Das Startdatum des globalen Suno-Abonnements fehlt.", "The start date of the global Suno subscription is missing."],
  [
    "Die globale Transparenzrichtlinie für AI-Artwork fehlt.",
    "The global transparency policy for AI artwork is missing."
  ],
  ["Der globale Standarddienst für AI-Bilder fehlt.", "The global default service for AI images is missing."],
  ["Die Angabe zu extern hochgeladenem Audio fehlt.", "Information about uploaded external audio is missing."],
  [
    "Quelle und Rechteangaben für externes Audio fehlen.",
    "Source and rights information for external audio are missing."
  ],
  ["Die hochgeladene externe Audiodatei fehlt.", "The uploaded external audio file is missing."],
  ["Der Lizenznachweis für externes Audio fehlt.", "License evidence for external audio is missing."],
  ["Die Angabe zu eigenem hochgeladenem Audio fehlt.", "Information about audio uploaded by you is missing."],
  [
    "Quelle und Bestätigung für eigenes Audio fehlen.",
    "Source and confirmation details for audio uploaded by you are missing."
  ],
  ["Die eigene hochgeladene Audiodatei fehlt.", "The audio file uploaded by you is missing."],
  ["Die Angabe zur codebasierten Erzeugung fehlt.", "Information about code-based generation is missing."],
  [
    "Der Quellcode oder die Quelldatei der codebasierten Erzeugung fehlt.",
    "The source code or source file for code-based generation is missing."
  ],
  [
    "Die Angabe zur Nachbearbeitung des codebasiert erzeugten Audios fehlt.",
    "Information about post-processing of the code-generated audio is missing."
  ],
  [
    "Mindestens eine tatsächlich durchgeführte Audio-Nachbearbeitung fehlt.",
    "At least one actually performed audio post-processing step is missing."
  ],
  [
    "Die mit dem Quellcode erzeugte WAV- oder MP3-Datei fehlt.",
    "The WAV or MP3 file generated from the source code is missing."
  ],
  ["Die Angabe zu Samples Dritter fehlt.", "Information about third-party samples is missing."],
  [
    "Quelle und Rechteangaben für Samples Dritter fehlen.",
    "Source and rights information for third-party samples are missing."
  ],
  ["Der Lizenznachweis für Samples Dritter fehlt.", "License evidence for third-party samples is missing."],
  [
    "Die hochgeladene Audiodatei mit Samples Dritter fehlt.",
    "The uploaded audio file containing third-party samples is missing."
  ],
  ["Das verwendete Suno-Modell fehlt.", "The Suno model used is missing."],
  ["Die Suno-Projekt-URL fehlt.", "The Suno project URL is missing."],
  ["Das Datum der finalen Suno-Generation fehlt.", "The date of the final Suno generation is missing."],
  ["Der Suno-Tarif bei der finalen Generation fehlt.", "The Suno plan at the time of final generation is missing."],
  ["Das Datum der letzten Bearbeitung fehlt.", "The date of the last edit is missing."],
  ["Der finale Suno-Export fehlt.", "The final Suno export is missing."],
  [
    "Der tatsächliche Suno-Exportdateiname fehlt, stimmt nicht mit dem dokumentierten Titel überein oder die Abweichung wurde nicht ausdrücklich bestätigt.",
    "The actual Suno export filename is missing, does not match the documented title, or the discrepancy has not been explicitly confirmed."
  ],
  [
    "Die ausdrückliche Angabe, ob der Track instrumental ist, fehlt.",
    "The explicit indication of whether the track is instrumental is missing."
  ],
  [
    "Die ausdrückliche Angabe, ob das finale Audio Gesang enthält, fehlt.",
    "The explicit indication of whether the final audio contains vocals is missing."
  ],
  [
    "Die beabsichtigte Gesangsnutzung (VOCAL, INSTRUMENTAL oder UNSPECIFIED) wurde nicht ausdrücklich ausgewählt.",
    "The intended vocal use (VOCAL, INSTRUMENTAL, or UNSPECIFIED) has not been explicitly selected."
  ],
  [
    "Die eindeutige Inhaltsklassifizierung des Suno Generation Text Field fehlt.",
    "The unambiguous content classification of the Suno Generation Text Field is missing."
  ],
  [
    "Die Quelle des Inhalts im Suno-Lyrics-/Structure-Feld fehlt.",
    "The source of the content in the Suno lyrics/structure field is missing."
  ],
  ["Der Inhalt des Suno-Lyrics-/Structure-Felds fehlt.", "The content of the Suno lyrics/structure field is missing."],
  ["Die Beschreibung für den Inhaltstyp 'Other' fehlt.", "The description for the content type 'Other' is missing."],
  ["Der in Suno verwendete Style-Prompt fehlt.", "The style prompt used in Suno is missing."],
  ["Die Angabe zur menschlichen Bearbeitung fehlt.", "Information about human editing is missing."],
  [
    "Die tatsächlich ausgeführten menschlichen Bearbeitungsschritte fehlen.",
    "The human editing steps actually performed are missing."
  ],
  [
    "Die Angabe, ob die Datei auf dem Desktop-PC bearbeitet wurde, fehlt.",
    "The information about whether the file was edited on a desktop computer is missing."
  ],
  [
    "Die Bearbeitungsschritte auf dem Desktop-PC fehlen.",
    "The editing steps performed on the desktop computer are missing."
  ],
  ["Die Herkunft des Artworks fehlt.", "The origin of the artwork is missing."],
  ["Das unveränderte AI-Artwork-Original fehlt.", "The unmodified original AI artwork is missing."],
  [
    "Mindestens eine menschliche Änderung am KI-assistierten Artwork fehlt.",
    "At least one human change to the AI-assisted artwork is missing."
  ],
  ["Das finale Artwork fehlt.", "The final artwork is missing."],
  [
    "Die Angabe zur Darstellung einer realen Person fehlt.",
    "Information about the depiction of a real person is missing."
  ],
  [
    "Die sachliche Notiz zur dargestellten realen Person fehlt.",
    "The factual note about the depicted real person is missing."
  ],
  [
    "Die Angabe zur Darstellung eines realen Ereignisses fehlt.",
    "Information about the depiction of a real event is missing."
  ],
  [
    "Die sachliche Notiz zum dargestellten realen Ereignis fehlt.",
    "The factual note about the depicted real event is missing."
  ],
  [
    "Die Angabe zu Marken oder Logos im Artwork fehlt.",
    "Information about trademarks or logos in the artwork is missing."
  ],
  ["Die sachliche Notiz zur Marke oder zum Logo fehlt.", "The factual note about the trademark or logo is missing."],
  [
    "Die Angabe zur Verwendung generativer AI im Audio fehlt.",
    "Information about the use of generative AI in the audio is missing."
  ],
  ["Das für das Audio verwendete AI-System fehlt.", "The AI system used for the audio is missing."],
  ["Die Angabe zu AI-assistierten Audioelementen fehlt.", "Information about AI-assisted audio elements is missing."],
  ["Die Angabe zu AI-generierten Audioelementen fehlt.", "Information about AI-generated audio elements is missing."],
  [
    "Die Angabe zur absichtlichen Imitation der Stimme einer realen Person fehlt.",
    "Information about intentional imitation of a real person's voice is missing."
  ],
  [
    "Die Angabe zur absichtlichen Darstellung der Identität einer realen Person fehlt.",
    "Information about intentional representation of the identity of a real person is missing."
  ],
  [
    "Die Angabe zur Darstellung eines realen Ereignisses als authentische Aufnahme fehlt.",
    "Information about representing a real event as an authentic recording is missing."
  ],
  [
    "Die Angabe zur authentisch wirkenden AI-Aufnahme eines realen Orts, einer Institution oder eines Ereignisses fehlt.",
    "Information about an AI recording of a real location, institution, or event presented as authentic is missing."
  ],
  [
    "Die Entscheidung zur Audio-AI-Kennzeichnung fehlt oder ist für die beabsichtigte kommerzielle Nutzung nicht dokumentiert.",
    "The decision on audio AI disclosure is missing or has not been documented for the intended commercial use."
  ],
  ["Mindestens ein Ort der Audio-AI-Kennzeichnung fehlt.", "At least one audio AI disclosure location is missing."],
  ["Der verwendete Text der Audio-AI-Kennzeichnung fehlt.", "The text used for the audio AI disclosure is missing."],
  ["Der verwendete AI-Bilddienst fehlt.", "The AI image service used is missing."],
  ["Die Transparenzrichtlinie für AI-Artwork fehlt.", "The transparency policy for AI artwork is missing."],
  [
    "Die explizite YES-/NO-Entscheidung zur sichtbaren AI-Kennzeichnung fehlt oder ist bei YES nicht vollständig belegt.",
    "The explicit YES/NO decision on visible AI disclosure is missing or, if YES, is not fully evidenced."
  ],
  ["Die finale Release-Audiodatei fehlt.", "The final release audio file is missing."],
  [
    "Der tatsächliche Release-Dateiname fehlt, stimmt nicht mit dem dokumentierten Titel überein oder die Abweichung wurde nicht ausdrücklich bestätigt.",
    "The actual release filename is missing, does not match the documented title, or the discrepancy has not been explicitly confirmed."
  ],
  [
    "Der lokale Chromaprint-Fingerprint für das autoritative Release-Audio fehlt, ist veraltet oder ist nicht an dessen SHA-256 gebunden.",
    "The local Chromaprint fingerprint for the authoritative release audio is missing, outdated, or is not bound to its SHA-256."
  ],
  [
    "Die zugeordneten Abo-Nachweise decken den Produktionszeitraum nicht lückenlos ab.",
    "The assigned subscription evidence does not cover the production period without gaps."
  ],
  [
    "Die zugeordneten Abo-Nachweise decken das Datum der finalen Suno-Generation nicht nachweisbar ab.",
    "The assigned subscription evidence does not demonstrably cover the date of the final Suno generation."
  ],
  [
    "Ein verifizierter lokaler Nachweis der Suno-Nutzungsbedingungen mit Titel, Anbieter und Abrufdatum fehlt.",
    "A verified local record of the Suno terms of service, including title, provider, and retrieval date, is missing."
  ],
  [
    "Mindestens ein Pflichtdokument fehlt oder ist nicht aktuell.",
    "At least one required document is missing or is not current."
  ],
  ["Die SHA-256-Prüfung fehlt oder ist fehlgeschlagen.", "SHA-256 verification is missing or has failed."],
  ["Mindestens eine blockierende Abweichung ist offen.", "At least one blocking deviation remains unresolved."],
  // Native workflow consistency and evidence findings
  ["Evidence fehlt oder ist nicht verifiziert: {path}", "Evidence is missing or not verified: {path}"],
  [
    "Mehrere widersprüchliche Suno-Metadatensätze wurden erkannt; kein Datum wurde automatisch ausgewählt.",
    "Multiple conflicting Suno metadata sets were detected; no date was selected automatically."
  ],
  [
    "Gespeicherte Suno-Metadaten stimmen nicht mit dem erhaltenen eingebetteten Wert überein.",
    "Stored Suno metadata does not match the received embedded value."
  ],
  [
    "Die gespeicherte Evidence-Herkunft verweist nicht mehr auf den aktuellen Suno-Export.",
    "The stored evidence origin no longer refers to the current Suno export."
  ],
  [
    "Menschlich bearbeitetes Artwork ist vorhanden, aber die Bearbeitung ist nicht dokumentiert.",
    "Human-edited artwork is present, but the editing is not documented."
  ],
  [
    "Verifizierte Terms-Evidence ist vorhanden, widerspricht aber der Angabe, dass sie nicht verfügbar sei.",
    "Verified terms evidence is present, but conflicts with the statement that it is unavailable."
  ],
  ["Eine automatisch referenzierte Evidence-Datei fehlt.", "An automatically referenced evidence file is missing."],
  // System status labels from Rust and frontend/domain types
  ["NICHT ERFASST", "NOT RECORDED"],
  ["WIRD ANGEFORDERT", "REQUESTING"],
  ["ANGEHÄNGT", "ATTACHED"],
  ["VERIFIZIERT", "VERIFIED"],
  ["VERIFIKATION FEHLGESCHLAGEN", "VERIFICATION FAILED"],
  ["PROVIDER NICHT VERFÜGBAR", "PROVIDER UNAVAILABLE"],
  ["AUTHENTIFIZIERUNG FEHLGESCHLAGEN", "AUTHENTICATION FAILED"],
  ["ANCHOR-ABWEICHUNG", "ANCHOR MISMATCH"],
  ["DEAKTIVIERT", "DISABLED"],
  ["BEREIT", "READY"],
  ["KONFIGURATION UNVOLLSTÄNDIG", "CONFIGURATION INCOMPLETE"],
  ["AUTHENTIFIZIERUNG ERFORDERLICH", "AUTHENTICATION REQUIRED"],
  ["VERBINDUNG FEHLGESCHLAGEN", "CONNECTION FAILED"],
  ["NICHT UNTERSTÜTZTE ANTWORT", "UNSUPPORTED RESPONSE"],
  ["VERIFIKATIONSKONFIGURATION UNVOLLSTÄNDIG", "VERIFICATION CONFIGURATION INCOMPLETE"],
  ["NICHT AUSGEFÜHRT", "NOT RUN"],
  ["FINGERPRINT ERZEUGT", "FINGERPRINT GENERATED"],
  ["KEINE ÜBEREINSTIMMUNG ERKANNT", "NO MATCH DETECTED"],
  ["ÜBEREINSTIMMUNG ERKANNT", "MATCH DETECTED"],
  ["ÜBERSPRUNGEN – NICHT KONFIGURIERT", "SKIPPED – NOT CONFIGURED"],
  ["ÜBERSPRUNGEN (NICHT KONFIGURIERT)", "SKIPPED NOT CONFIGURED"],
  ["KONFIGURATION UNGÜLTIG", "CONFIGURATION INVALID"],
  ["ENGINE NICHT VERFÜGBAR", "ENGINE UNAVAILABLE"],
  ["NICHT UNTERSTÜTZTES FORMAT", "UNSUPPORTED FORMAT"],
  ["VERARBEITUNG FEHLGESCHLAGEN", "PROCESSING FAILED"],
  ["VERALTET", "STALE"],
  // Frontend domain defaults
  ["Der externe Zeitstempeldienst ist deaktiviert.", "External timestamp service is disabled."],
  [
    "Die lokale Chromaprint-Prüfung wurde für die aktuelle Release-Datei noch nicht ausgeführt.",
    "The local Chromaprint screening has not run for the current release file."
  ],
  [
    "Die optionale ACRCloud-Prüfung wurde noch nicht angefordert.",
    "The optional ACRCloud check has not been requested."
  ],
  ["Die optionale ACRCloud-Prüfung ist nicht konfiguriert.", "Optional ACRCloud screening is not configured."],
  // Browser-demo status, provider, and screening messages
  [
    "Nur Browser-Demo: Es wurde keine lokale Audiodatei analysiert und keine Provider-Anfrage gesendet.",
    "Browser demo presentation only: no local audio file was analysed and no provider request was made."
  ],
  [
    "Browser-Demo: Die optionale ACRCloud-Prüfung ist nicht konfiguriert und es wurde kein Provider kontaktiert.",
    "Browser demo: optional ACRCloud screening is not configured and no provider was contacted."
  ],
  [
    "Ein externer Zeitstempelnachweis ist optional und für die technische Finalisierung nicht erforderlich.",
    "External timestamp evidence is optional and is not required for technical finalization."
  ],
  [
    "Der OpenTimestamps-Kalenderdienst ist bereit. Dies ist kein RFC 3161; ein erster Nachweis bleibt ATTACHED, bis die OpenTimestamps-Verifikation oder ein Upgrade erfolgt.",
    "OpenTimestamps calendar service is ready. This is not RFC 3161; an initial proof remains ATTACHED pending OpenTimestamps verification or upgrade."
  ],
  [
    "Wähle eine explizite TSA-CA-Trust-Anchor-Datei aus, bevor RFC-3161-Antworten als VERIFIED markiert werden können.",
    "Select an explicit TSA CA trust-anchor file before RFC 3161 responses can be marked VERIFIED."
  ],
  [
    "{provider} RFC-3161-Dienst und sein expliziter TSA-Trust-Anchor sind bereit.",
    "{provider} RFC 3161 service and its explicit TSA trust anchor are ready."
  ],
  [
    "Gib einen Providernamen und einen TSA-Endpunkt für Custom RFC 3161 ein.",
    "Enter a provider name and TSA endpoint for Custom RFC 3161."
  ],
  [
    "Gib den Kontonamen ein und konfiguriere das Secret getrennt.",
    "Enter the account name and configure its secret separately."
  ],
  [
    "Konfiguriere das Provider-Token in den sicheren lokalen Einstellungen.",
    "Configure the provider token in secure local settings."
  ],
  [
    "Wähle ein konfiguriertes Client-Zertifikat für diesen Provider aus.",
    "Select a configured client certificate for this provider."
  ],
  ["{provider} und sein expliziter Trust Anchor sind bereit.", "{provider} and its explicit trust anchor are ready."],
  ["Die optionale ACRCloud-Prüfung ist deaktiviert.", "Optional ACRCloud screening is disabled."],
  [
    "Gib den ACRCloud-Projekthost ein, bevor du die optionale Providerprüfung nutzt.",
    "Enter the ACRCloud project host before using the optional provider check."
  ],
  [
    "Speichere beide ACRCloud-Zugangswerte in den sicheren lokalen Einstellungen, bevor du die optionale Providerprüfung nutzt.",
    "Store both ACRCloud access values in local secure settings before using the optional provider check."
  ],
  [
    "ACRCloud ist konfiguriert. Nutze den expliziten Test oder die Prüfung pro Track; es wird keine Anfrage automatisch ausgeführt.",
    "ACRCloud is configured. Use the explicit test or per-track check; no request runs automatically."
  ],
  [
    "Der finalisierte Evidence-Manifest-Anchor ist nicht verfügbar.",
    "The finalized evidence-manifest anchor is not available."
  ],
  [
    "Der Detached Proof ist lokal an den angeforderten SHA-256-Wert gebunden; die explizite OpenTimestamps-Verifikation oder ein Upgrade steht noch aus.",
    "Detached proof is locally bound to the requested SHA-256; explicit OpenTimestamps verification or upgrade is pending."
  ],
  [
    "Struktur- und Digest-Prüfungen sind abgeschlossen; Provider-Signatur und Vertrauensprüfung werden nicht behauptet.",
    "Structural and digest checks completed; provider signature and trust verification are not asserted."
  ],
  [
    "Die Anfrage für einen externen Zeitstempelnachweis wird vorbereitet.",
    "External timestamp request is being prepared."
  ],
  [
    "OpenTimestamps-Detached-Proof archiviert; die Bitcoin-Verankerung steht noch zur Verifikation oder zum Upgrade aus.",
    "OpenTimestamps detached proof archived; Bitcoin anchoring remains pending verification or upgrade."
  ],
  [
    "Automatische Providerantwort; Struktur- und Digest-Prüfungen",
    "Automatic provider response; structural and digest checks"
  ],
  [
    "OpenTimestamps-Detached-Proof; Bitcoin-Verankerung wartet auf Verifikation/Upgrade",
    "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade"
  ],
  [
    "OpenTimestamps-Detached-Proof angehängt; eine spätere Verifikation oder ein Upgrade ist erforderlich.",
    "OpenTimestamps detached proof attached; later verification or upgrade is required."
  ],
  [
    "Externe Zeitstempelantwort angehängt; Struktur- und Digest-Prüfungen abgeschlossen.",
    "External timestamp response attached; structural and digest checks completed."
  ],
  [
    "Das autoritative Release-Audio hat sich geändert; erstelle einen neuen lokalen Fingerprint für die aktuelle Datei.",
    "The authoritative release audio changed; generate a new local fingerprint for the current file."
  ],
  [
    "Das autoritative Release-Audio ist nicht mehr verfügbar; der bisherige lokale Fingerprint ist veraltet.",
    "The authoritative release audio is no longer available; the prior local fingerprint is stale."
  ],
  [
    "Das autoritative Release-Audio hat sich geändert; das bisherige externe Ergebnis ist nicht mehr aktuell.",
    "The authoritative release audio changed; the prior external result is no longer current."
  ],
  [
    "OpenTimestamps-Kalender erreichbar. Dies ist kein RFC 3161; die Proof-Verifikation oder ein Upgrade steht noch aus.",
    "OpenTimestamps calendar reachable. This is not RFC 3161; proof verification or upgrade remains pending."
  ],
  [
    "Provider erreichbar. RFC-3161-Zeitstempeldienst ist bereit.",
    "Provider reachable. RFC 3161 timestamp service ready."
  ],
  [
    "Browser-Demo: Es wird keine Verbindung zu ACRCloud hergestellt. Teste den konfigurierten Provider in der Desktop-App.",
    "Browser demo: no ACRCloud connection is made. Test the configured provider in the desktop app."
  ],
  [
    "Nur Browser-Demo: Es wurde keine lokale Audiodatei analysiert. Führe die Prüfung in der Desktop-App für ein autoritatives Ergebnis aus.",
    "Browser demo presentation only: no local audio file was analysed. Run this check in the desktop app for an authoritative result."
  ],
  [
    "Browser-Demo: Es wurde keine ACRCloud-Anfrage gesendet. Die Desktop-App ist für eine autoritative Providerantwort erforderlich.",
    "Browser demo: no ACRCloud request was sent. The desktop app is required for an authoritative provider response."
  ],
  [
    "Browser-Demo: Der lokale Screening-Status wurde nur zur Oberflächenvorschau simuliert. Die Desktop-App erzeugt den echten Chromaprint-Fingerprint.",
    "Browser demo: The local screening status was simulated only for UI preview. The desktop app creates the actual Chromaprint fingerprint."
  ],
  [
    "Die optionale externe Prüfung wurde übersprungen; die ACRCloud-Konfiguration ist nicht vollständig.",
    "The optional external check was skipped because the ACRCloud configuration is incomplete."
  ],
  [
    "Browser-Demo: Es wurde keine ACRCloud-Anfrage ausgeführt und kein Providerergebnis erzeugt.",
    "Browser demo: No ACRCloud request was made and no provider result was generated."
  ],
  ["{count} Dokumente wurden deterministisch erzeugt.", "{count} documents were generated deterministically."],
  [
    "Der sichtbare KI-Hinweis wurde lokal auf einer neuen Artwork-Version angewendet.",
    "The visible AI disclosure was applied locally to a new artwork version."
  ],
  ["{count} Dateien wurden gehasht.", "{count} files were hashed."],
  [
    "{verified} von {total} Dateien erfolgreich verifiziert.",
    "{verified} of {total} files were verified successfully."
  ],
  ["Dokumentation finalisiert und Zertifikat erzeugt.", "Documentation finalized and certificate generated."],
  [
    "Der bisherige Snapshot wurde archiviert und eine neue Revision angelegt.",
    "The previous snapshot was archived and a new revision was created."
  ],
  ["Der Track wurde im aktuellen Workspace nicht gefunden.", "The track was not found in the current workspace."],
  ["Der globale Terms-Nachweis wurde nicht gefunden.", "The global terms evidence was not found."],
  ["Der globale Nachweis wurde nicht gefunden.", "The global evidence was not found."],
  [
    "Ein Albumordner mit diesem Namen existiert bereits: {title}",
    "An album folder with this name already exists: {title}"
  ],
  ["Album nicht gefunden: {title}", "Album not found: {title}"],
  [
    "Die Rolle {role} ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum Ersetzen.",
    "The {role} role is already assigned. Use the upload button on the existing evidence to replace it."
  ],
  ["Die zu ersetzende Evidence wurde nicht gefunden.", "The evidence to be replaced was not found."],
  ["Die Evidence wurde nicht gefunden.", "The evidence was not found."],
  ["Erzeuge zuerst die aktuellen Dokumente.", "Generate the current documents first."],
  [
    "Importiere zuerst die autoritative finale Release-Audiodatei.",
    "Import the authoritative final release audio file first."
  ],
  ["Finalisierung blockiert: {0}", "Finalization blocked: {0}"],
  ["Projektinterne Disclosure-Policy", "Project-internal disclosure policy"],
  ["Evidence-Manifest (empfohlener Zeitstempel-Anchor)", "Evidence manifest (recommended timestamp anchor)"],
  ["Track-SHA-256-Manifest", "Track SHA-256 manifest"],
  ["Dokumentationszertifikat (Markdown)", "Documentation certificate (Markdown)"],
  ["Dokumentationszertifikat (englisches PDF)", "Documentation certificate (English PDF)"],
  ["Prüfsummensatz des finalen Evidence-Paketzertifikats", "Final evidence package certificate hash set"],
  ["Deaktiviert", "Disabled"],
  // Native AppError variants
  ["Kein Workspace ist geöffnet.", "No workspace is open."],
  ["Der Workspace-Sperrzustand ist nicht verfügbar.", "Workspace state lock is unavailable."],
  ["Es wurde kein Workspace-Pfad angegeben.", "No workspace path was provided."],
  ["Zeitstempel-Provider-Testaufgabe fehlgeschlagen: {error}", "Timestamp provider test failed: {error}"],
  ["Audio-Prüfprovider-Testaufgabe fehlgeschlagen: {error}", "Audio-screening provider test failed: {error}"],
  ["Zeitstempel-Anhangsaufgabe fehlgeschlagen: {error}", "Timestamp attachment task failed: {error}"],
  ["Ordnerimportaufgabe fehlgeschlagen: {error}", "Folder import task failed: {error}"],
  ["Track-Cover-Aufgabe fehlgeschlagen: {error}", "Track cover task failed: {error}"],
  ["Evidence-Importaufgabe fehlgeschlagen: {error}", "Evidence import task failed: {error}"],
  ["Evidence-Vorschauaufgabe fehlgeschlagen: {error}", "Evidence preview task failed: {error}"],
  ["Dokumenterzeugungsaufgabe fehlgeschlagen: {error}", "Document generation task failed: {error}"],
  ["SHA-256-Berechnungsaufgabe fehlgeschlagen: {error}", "SHA-256 calculation task failed: {error}"],
  ["SHA-256-Verifikationsaufgabe fehlgeschlagen: {error}", "SHA-256 verification task failed: {error}"],
  ["Lokale Audio-Prüfaufgabe fehlgeschlagen: {error}", "Local audio-screening task failed: {error}"],
  ["Externe Audio-Prüfaufgabe fehlgeschlagen: {error}", "External audio-screening task failed: {error}"],
  ["Finalisierungsaufgabe fehlgeschlagen: {error}", "Finalization task failed: {error}"],
  ["Der ausgewählte Pfad ist kein gültiger Workspace: {0}", "The selected path is not a valid workspace: {0}"],
  ["Der angeforderte Pfad liegt außerhalb des Workspaces.", "The requested path is outside the workspace."],
  [
    "Symbolische Links werden für verwaltete Pfade nicht akzeptiert: {0}",
    "Symbolic links are not accepted for managed paths: {0}"
  ],
  ["Am verwalteten Ziel existiert bereits eine Datei: {0}", "A file already exists at the managed destination: {0}"],
  [
    "Der Dateityp ist für die Evidence-Rolle {role} nicht zulässig: {extension}",
    "The file type is not allowed for evidence role {role}: {extension}"
  ],
  ["Track nicht gefunden: {0}", "Track not found: {0}"],
  ["Evidence nicht gefunden: {0}", "Evidence not found: {0}"],
  [
    "Der Track ist finalisiert. Erstelle vor Änderungen eine Revision.",
    "The track is finalized. Create a revision before changing it."
  ],
  [
    "Vorhandene nicht verwaltete Dokumente erfordern eine explizite Übernahme und Archivierung: {0}",
    "Existing unmanaged documents require explicit adoption and archival: {0}"
  ],
  ["Der Vorgang ist blockiert: {0}", "The operation is blocked: {0}"],
  ["Datenbankfehler: {0}", "Database error: {0}"],
  ["Dateioperation für {path} fehlgeschlagen: {source}", "File operation failed for {path}: {source}"],
  ["Ungültige gespeicherte Daten: {0}", "Invalid stored data: {0}"],
  ["Bildverarbeitung fehlgeschlagen: {0}", "Image processing failed: {0}"],
  // Native validation templates and their field labels
  ["{field} ist erforderlich.", "{field} is required."],
  ["{field} ist ungültig oder zu lang.", "{field} is invalid or too long."],
  ["{field} enthält zu viele Einträge.", "{field} contains too many entries."],
  ["{field} muss YYYY-MM-DD verwenden.", "{field} must use YYYY-MM-DD."],
  ["{field}-Ende darf nicht vor dem Start liegen.", "{field} end cannot be before its start."],
  [
    "Das Datum des Abozeitraums liegt außerhalb des unterstützten Bereichs.",
    "Subscription coverage date is outside the supported range."
  ],
  ["Die Artwork-Transparenzrichtlinie ist ungültig.", "Artwork transparency policy is invalid."],
  [
    "Das Datum der letzten Bearbeitung darf nicht vor dem Produktionsbeginn liegen.",
    "Last editing date cannot be before production start."
  ],
  ["Die Lyrics-Quelle ist ungültig.", "Lyrics source is invalid."],
  ["Die Artwork-Herkunft ist ungültig.", "Artwork origin is invalid."]
];
