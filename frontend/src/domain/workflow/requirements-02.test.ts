import { describe, expect, it } from "vitest";

import { calculateMissingRequirements, finalizationGate } from "../workflow";

import { completeTrack, profile } from "./test-fixtures";

describe("missing requirements", () => {
  it("keeps Vocal Intent, classification, instrumental mode, and final audio independent", () => {
    const track = completeTrack();
    track.fields.instrumentalTrack = true;
    track.fields.vocalLyricsPresent = true;
    track.fields.vocalIntent = "INSTRUMENTAL";
    track.fields.sunoContentClassification = "MIXED";
    track.fields.humanEditingPerformed = true;
    track.fields.humanEditingDetails = "Arrangement, Lyrics";
    track.fields.legacyLyricsText = "[Intro]\nlegacy value";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "instrumental-vocal-consistency"
    );
    expect(finalizationGate(track, profile).valid).toBe(true);

    track.fields.vocalIntent = "VOCAL";
    track.fields.vocalLyricsPresent = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "instrumental-vocal-consistency"
    );
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("uses EMPTY as the only N/A content branch and requires an OTHER label", () => {
    const track = completeTrack();
    track.fields.sunoContentClassification = "EMPTY";
    track.fields.vocalIntent = "UNSPECIFIED";
    track.fields.sunoLyricsContentSource = null;
    track.fields.sunoLyricsFieldText = "";
    track.fields.sunoLyricsOtherContentType = "";
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("suno-lyrics-content-source");
    expect(ids).not.toContain("suno-lyrics-field-text");
    expect(ids).not.toContain("suno-lyrics-other-content-type");

    track.fields.sunoContentClassification = "OTHER";
    track.fields.sunoLyricsContentSource = "human";
    track.fields.sunoLyricsFieldText = "[Spoken direction]";
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("suno-lyrics-other-content-type");
    track.fields.sunoLyricsOtherContentType = "Spoken performance direction";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "suno-lyrics-other-content-type"
    );
  });

  it("requires explicit scalar semantics and accepts MIXED as one classification", () => {
    const track = completeTrack();
    track.fields.sunoContentClassification = null;
    track.fields.vocalIntent = null;
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toEqual(expect.arrayContaining(["suno-content-classification", "vocal-intent"]));

    track.fields.sunoContentClassification = "MIXED";
    track.fields.vocalIntent = "UNSPECIFIED";
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("suno-content-classification");
    expect(ids).not.toContain("vocal-intent");
  });

  it("does not let a historical plan-at-creation value satisfy the current generation-plan gate", () => {
    const track = completeTrack();
    track.fields.sunoPlanAtGeneration = "";
    track.fields.legacySunoPlanAtCreation = "Historical Pro";

    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("suno-plan");
  });

  it("TEST 03 accepts a vocal track with vocal lyrics", () => {
    const track = completeTrack();
    track.fields.instrumentalTrack = false;
    track.fields.vocalLyricsPresent = true;
    track.fields.vocalIntent = "VOCAL";
    track.fields.sunoContentClassification = "VOCAL_LYRICS_ONLY";
    track.fields.sunoLyricsFieldText = "Original vocal lyrics";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "instrumental-vocal-consistency"
    );
  });

  it("requires explicit confirmation for an intentional filename deviation without changing the title", () => {
    const track = completeTrack();
    track.fields.title = "Gravaty";
    track.evidence.find((item) => item.role === "release_wav")!.metadata!.originalFileName = "GRAVITY.wav";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("release-filename");
    track.fields.releaseFilenameDifferenceConfirmed = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-filename");
    expect(track.fields.title).toBe("Gravaty");
  });
});
