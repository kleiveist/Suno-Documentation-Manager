import { describe, expect, it } from "vitest";

import {
  canonicalGuidedChoiceArray,
  canonicalGuidedChoiceList,
  canonicalGuidedChoiceValue,
  parseMultiChoiceValue,
  SETTINGS_CATEGORY_DEFINITIONS,
  serializeMultiChoiceValue,
  settingsCategoryNavigationMarkup,
  singleChoiceFieldMarkup,
  type GuidedChoice
} from "./app";
import { translateUiText } from "./ui/i18n";

describe("navigation", () => {
  it("keeps the three settings categories and their local navigation labels stable", () => {
    expect(SETTINGS_CATEGORY_DEFINITIONS.map(({ id, label }) => [id, label])).toEqual([
      ["global", "Globale Angaben"],
      ["external", "Externe Dienste"],
      ["files", "Globale Datei-Führung"]
    ]);

    const markup = settingsCategoryNavigationMarkup();
    expect(markup).toContain('data-settings-category="global"');
    expect(markup).toContain('data-settings-category="external"');
    expect(markup).toContain('data-settings-category="files"');
    expect(markup).not.toContain("data-action=");
    expect(settingsCategoryNavigationMarkup("external")).toContain(
      'data-settings-category="external" aria-controls="settings-external" aria-selected="true"'
    );
    for (const { label } of SETTINGS_CATEGORY_DEFINITIONS) expect(markup).toContain(label);
  });

  it("uses the certificate language as the app language for shared UI copy", () => {
    expect(translateUiText("Einstellungen", "en")).toBe("Settings");
    expect(translateUiText("Einstellungen", "de")).toBe("Einstellungen");
    expect(translateUiText("Die Nichtanwendung wurde bewusst dokumentiert.", "en")).toBe(
      "The deliberate non-application has been documented."
    );
    expect(
      translateUiText(
        "Import-Zeitstempel dokumentieren nur den Import in SunoDM und nicht die tatsächliche Erstellungs- oder Bearbeitungsreihenfolge der Artwork-Dateien.",
        "en"
      )
    ).toContain("Import timestamps document only the import into SunoDM");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("Global details");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("External services");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("Settings sections");
  });

  it("stores multiple guided choices deterministically", () => {
    expect(serializeMultiChoiceValue(["Mixing", "Mastering", "Mixing"])).toBe("Mixing | Mastering");
    expect(parseMultiChoiceValue("Mixing | Mastering")).toEqual(["Mixing", "Mastering"]);
  });

  it("stores English values while accepting localized labels and retaining unknown legacy data", () => {
    const choices: readonly GuidedChoice[] = [
      ["Timing and cuts", "Timing und Cuts"],
      ["Loudness adjustment", "Lautheitsanpassung"]
    ];
    expect(canonicalGuidedChoiceValue("Timing und Cuts", choices)).toBe("Timing and cuts");
    expect(canonicalGuidedChoiceList("Timing und Cuts | Lautheitsanpassung", choices)).toBe(
      "Timing and cuts | Loudness adjustment"
    );
    expect(canonicalGuidedChoiceValue("Historischer Freitext", choices)).toBe("Historischer Freitext");
    expect(canonicalGuidedChoiceArray(["Timing und Cuts", "Historischer Freitext"], choices)).toEqual([
      "Timing and cuts",
      "Historischer Freitext"
    ]);
  });

  it("renders required single choices as mutually exclusive buttons", () => {
    const markup = singleChoiceFieldMarkup(
      "sunoLyricsContentSource",
      "Content source",
      "human",
      [
        ["instrumental", "Instrumental"],
        ["human", "Menschlich geschrieben"]
      ],
      true
    );

    expect(markup).not.toContain("<select");
    expect(markup.match(/type="radio"/g)).toHaveLength(2);
    expect(markup.match(/name="sunoLyricsContentSource"/g)).toHaveLength(2);
    expect(markup).toContain('value="human" data-single-choice checked required');
    expect(markup).toContain("Wähle genau eine passende Option aus.");
  });
});
