import { describe, expect, it } from "vitest";

import { SYSTEM_TRANSLATIONS } from "./system-translations";
import { hasUiTranslation, translateUiText } from "./i18n";

describe("bidirectional system localization", () => {
  it("translates known UI copy in both directions", () => {
    expect(translateUiText("Einstellungen", "en")).toBe("Settings");
    expect(translateUiText("Settings", "de")).toBe("Einstellungen");
    expect(translateUiText("External timestamp service is disabled.", "de"))
      .toBe("Der externe Zeitstempeldienst ist deaktiviert.");
    expect(translateUiText("Der externe Zeitstempeldienst ist deaktiviert.", "en"))
      .toBe("External timestamp service is disabled.");
  });

  it("recognizes every workflow and system message in either source or target language", () => {
    for (const [german, english] of SYSTEM_TRANSLATIONS) {
      expect(hasUiTranslation(german, "en"), german).toBe(true);
      expect(hasUiTranslation(german, "de"), german).toBe(true);
      expect(hasUiTranslation(english, "en"), english).toBe(true);
      expect(hasUiTranslation(english, "de"), english).toBe(true);
      expect(translateUiText(german, "en"), german).toBe(english);
    }
  });

  it("resolves parameterized native and workflow messages without translating user values", () => {
    expect(translateUiText("Evidence is missing or not verified: releases/My Track.wav", "de"))
      .toBe("Evidence fehlt oder ist nicht verifiziert: releases/My Track.wav");
    expect(translateUiText("4 documents generated.", "de"))
      .toBe("4 Dokumente wurden erzeugt.");
    expect(translateUiText("The operation is blocked: Track title is missing.", "de"))
      .toBe("Der Vorgang ist blockiert: Tracktitel fehlt.");
    expect(translateUiText("The operation is blocked: Lyrics source is invalid.", "de"))
      .toBe("Der Vorgang ist blockiert: Die Lyrics-Quelle ist ungültig.");
    expect(translateUiText("Artist name is required.", "de"))
      .toBe("Künstlername ist erforderlich.");
    expect(translateUiText("Preparing authoritative release audio", "de"))
      .toBe("Autoritatives Release-Audio wird vorbereitet");
    expect(translateUiText("External ACRCloud screening recorded: MATCH DETECTED.", "de"))
      .toBe("Externe ACRCloud-Prüfung erfasst: MATCH ERKANNT.");
    expect(translateUiText("Timestamp sidecar contains a non-regular file: Settings", "de"))
      .toBe("Das Zeitstempel-Sidecar enthält keine reguläre Datei: Settings");
    expect(translateUiText("File operation failed for releases/My Track.wav: Permission denied", "de"))
      .toBe("Dateioperation für releases/My Track.wav fehlgeschlagen: Technisches Detail ist nicht verfügbar.");
    expect(translateUiText("Für einen unveränderlichen Track-Snapshot fehlen: Artist name, Suno profile name.", "en"))
      .toBe("For an immutable track snapshot, the following are missing: Artist name, Suno profile name.");
    expect(translateUiText("Gespeicherter Workspace konnte nicht geladen werden: The local action could not be completed.", "en"))
      .toBe("Saved workspace could not be loaded: The local action could not be completed.");
    expect(translateUiText("Suno Final-Export wurde kopiert, gehasht und dem Track zugeordnet.", "en"))
      .toBe("Suno final export was copied, hashed, and assigned to the track.");
    expect(translateUiText("2 Tracks wurden als unvollständige normale SunoDM-Struktur angelegt.", "en"))
      .toBe("2 tracks were created as incomplete standard SunoDM structures.");
    expect(translateUiText("Invalid stored data: Timestamp provider test failed: Lyrics source is invalid.", "de"))
      .toBe("Ungültige gespeicherte Daten: Zeitstempel-Provider-Testaufgabe fehlgeschlagen: Die Lyrics-Quelle ist ungültig.");
    expect(translateUiText("Der Vorgang ist blockiert: releases/Mein Track.wav", "en"))
      .toBe("The operation is blocked: releases/Mein Track.wav");
    expect(translateUiText("Finalized with workflow suno-track 1.8 / Current workflow suno-track 1.9", "de"))
      .toBe("Finalisiert mit Workflow suno-track 1.8 / Aktueller Workflow suno-track 1.9");
    expect(translateUiText("Finalisierung blockiert: Tracktitel fehlt., Produktionsbeginn fehlt.", "en"))
      .toBe("Finalization blocked: Track title is missing., Production start date is missing.");
    expect(translateUiText("Import and verify the authoritative final release audio before external screening.", "de"))
      .toBe("Importiere und verifiziere vor der externen Prüfung die maßgebliche finale Release-Audiodatei.");
    expect(translateUiText("A user-provided song title", "de")).toBe("A user-provided song title");
  });
});
