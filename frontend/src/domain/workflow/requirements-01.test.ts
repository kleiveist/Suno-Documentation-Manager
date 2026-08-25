import { describe, expect, it } from "vitest";

import { calculateMissingRequirements, finalizationGate } from "../workflow";

import { profile, completeTrack } from "./test-fixtures";

describe("missing requirements", () => {
  it("lists only applicable missing items", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile)).toEqual([]);
    track.fields.externalAudioUploaded = true;
    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toEqual(
      expect.arrayContaining(["external-source", "external-ownership", "external-audio-file", "external-license"])
    );
    expect(ids).not.toContain("sample-license");
  });

  it("does not require retired generation identifiers or a generation time", () => {
    const track = completeTrack();
    track.fields.sunoProjectVersionId = "";
    track.fields.sunoFinalGenerationId = "";
    track.fields.sunoFinalGenerationTime = "";

    expect(calculateMissingRequirements(track, profile)).toEqual([]);
    expect(finalizationGate(track, profile)).toEqual({
      valid: true,
      missingItems: [],
      blockingItems: []
    });
  });

  it("allows an unknown download/export date without inventing precision", () => {
    const track = completeTrack();
    track.fields.sunoDownloadExportDate = "";

    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("suno-download-date");
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("assigns the last-editing date only to the release step", () => {
    const track = completeTrack();
    track.fields.finalExportDate = "";

    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "export-date", stepId: "release" })
    );
  });

  it("requires a current local Chromaprint record for the authoritative release audio", () => {
    const track = completeTrack();
    track.audioScreening.local.status = "stale";
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );

    track.audioScreening.local.status = "fingerprint_generated";
    track.audioScreening.local.sourceSha256 = "f".repeat(64);
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );

    track.audioScreening.local.sourceSha256 = "a".repeat(64);
    track.audioScreening.local.sourceRelativePath = "01_RELEASE/old-release.wav";
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );
  });

  it("assigns the desktop-editing answer and details only to the release step", () => {
    const track = completeTrack();
    track.fields.postExportEditingPerformed = null;
    let missing = calculateMissingRequirements(track, profile);
    expect(missing).toContainEqual(expect.objectContaining({ id: "post-editing-answer", stepId: "release" }));
    expect(missing.some((item) => item.id === "post-editing-answer" && item.stepId === "human_work")).toBe(false);

    track.fields.postExportEditingPerformed = true;
    track.fields.postExportEditingDetails = "";
    missing = calculateMissingRequirements(track, profile);
    expect(missing).toContainEqual(expect.objectContaining({ id: "post-editing-details", stepId: "release" }));
  });

  it("mirrors each blocking native consistency issue once in its workflow step", () => {
    const track = completeTrack();
    track.automation.consistencyIssues = [
      {
        code: "suno_stored_metadata_mismatch",
        message: "Gespeicherte und eingebettete Suno-Metadaten stimmen nicht überein.",
        stepId: "suno",
        blocking: true
      }
    ];

    const matches = calculateMissingRequirements(track, profile).filter(
      (item) => item.id === "consistency-suno_stored_metadata_mismatch"
    );
    expect(matches).toEqual([expect.objectContaining({ stepId: "suno" })]);
    expect(finalizationGate(track, profile).valid).toBe(false);
  });

  it("TEST 01 accepts an instrumental with bracketed structure instructions and no vocal lyrics", () => {
    const track = completeTrack();
    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("instrumental-vocal-consistency");
    expect(ids).not.toContain("suno-lyrics-field-text");
    expect(finalizationGate(track, profile).valid).toBe(true);
  });
});
