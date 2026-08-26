import { describe, expect, it } from "vitest";

import { activeProfileId, activeProfileName, enabledFeatures, hasFeature } from "./project-profile";

describe("active project profile", () => {
  it("exposes the fixed desktop-local SunoDM profile", () => {
    expect(activeProfileId).toBe("desktop-local");
    expect(activeProfileName).toBe("Desktop local");
    expect(enabledFeatures).toEqual(["frontend", "tauri"]);
  });

  it("answers feature checks from the generated feature set", () => {
    for (const feature of enabledFeatures) {
      expect(hasFeature(feature)).toBe(true);
    }
    expect(hasFeature("not-a-project-feature")).toBe(false);
  });

  it("does not advertise unavailable service or database capabilities", () => {
    expect(hasFeature("backend")).toBe(false);
    expect(hasFeature("cloud")).toBe(false);
    expect(hasFeature("database")).toBe(false);
    expect(hasFeature("postgres")).toBe(false);
  });
});
