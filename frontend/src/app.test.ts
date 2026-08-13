import { describe, expect, it } from "vitest";

import { MAIN_NAVIGATION, missingProfileFields } from "./app";
import { WORKFLOW_STEPS } from "./domain/workflow";
import { emptyProfile } from "./domain/types";

describe("navigation", () => {
  it("exposes every required German main view", () => {
    expect(MAIN_NAVIGATION.map((item) => [item.id, item.label])).toEqual([
      ["dashboard", "Dashboard"], ["tracks", "Tracks"], ["current", "Aktueller Track"],
      ["workspace", "Workspace"], ["settings", "Einstellungen"]
    ]);
  });

  it("prevents creation of an immutable track snapshot from incomplete global data", () => {
    expect(missingProfileFields(emptyProfile)).toEqual(expect.arrayContaining([
      "Künstlername", "Suno-Profilname", "Suno-Benutzername", "Suno-Tarif", "Abo-Startdatum", "Standard-KI-Bilddienst"
    ]));
    expect(missingProfileFields({
      ...emptyProfile,
      artistName: "Artist", sunoProfileName: "Profile", sunoHandle: "@artist", sunoPlan: "Premier",
      subscriptionStartDate: "2026-01-01", defaultAiImageService: "Local Tool"
    })).toEqual([]);
  });

  it("makes all ten workflow steps reachable in their declared order", () => {
    expect(WORKFLOW_STEPS.map((step) => step.id)).toEqual([
      "track", "source", "suno", "human_work", "artwork", "ai_transparency",
      "release", "evidence_licenses", "integrity", "finalize"
    ]);
  });
});
