import { describe, expect, it } from "vitest";

import { audioScreeningIntensityBand, audioScreeningIntensityEstimate } from "./app";
import { resolveTheme, storedTheme, toggledTheme } from "./ui/theme";

describe("theme", () => {
  it("uses a valid saved choice before the operating-system preference", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("falls back to the operating-system preference for missing or invalid storage", () => {
    expect(resolveTheme(null, true)).toBe("dark");
    expect(resolveTheme("unsupported", false)).toBe("light");
    expect(storedTheme("unsupported")).toBeNull();
  });

  it("toggles between both supported themes", () => {
    expect(toggledTheme("light")).toBe("dark");
    expect(toggledTheme("dark")).toBe("light");
  });
});

describe("ACRCloud screening intensity estimates", () => {
  it("caps a fixed-reference target at the actual verified track duration", () => {
    const estimate = audioScreeningIntensityEstimate(
      {
        intensityPercent: 100,
        dynamicByTrackDuration: false,
        referenceDurationSeconds: 600
      },
      30
    );

    expect(estimate).toEqual({
      calculationDurationSeconds: 600,
      targetDurationSeconds: 30,
      requestedRequestCount: 3,
      actualRequestCount: 2,
      maxUniqueDurationSeconds: 24,
      capped: true
    });
  });

  it("keeps a single bounded sample available for tracks shorter than twelve seconds", () => {
    const estimate = audioScreeningIntensityEstimate(
      {
        intensityPercent: 100,
        dynamicByTrackDuration: true,
        referenceDurationSeconds: 300
      },
      5
    );

    expect(estimate.actualRequestCount).toBe(1);
    expect(estimate.maxUniqueDurationSeconds).toBe(5);
  });

  it("uses a shortened final sample to preserve uncapped requested coverage", () => {
    const estimate = audioScreeningIntensityEstimate(
      {
        intensityPercent: 25,
        dynamicByTrackDuration: false,
        referenceDurationSeconds: 300
      },
      600
    );

    expect(estimate.targetDurationSeconds).toBe(75);
    expect(estimate.actualRequestCount).toBe(7);
    expect(estimate.maxUniqueDurationSeconds).toBe(75);
    expect(estimate.capped).toBe(false);
  });

  it("never promises more than 25 requests or 300 seconds of unique audio", () => {
    const estimate = audioScreeningIntensityEstimate(
      {
        intensityPercent: 100,
        dynamicByTrackDuration: true,
        referenceDurationSeconds: 300
      },
      10_000
    );

    expect(estimate.actualRequestCount).toBe(25);
    expect(estimate.maxUniqueDurationSeconds).toBe(300);
    expect(estimate.capped).toBe(true);
    expect(audioScreeningIntensityBand(16)).toBe("high");
    expect(audioScreeningIntensityBand(25)).toBe("very_high");
  });
});
