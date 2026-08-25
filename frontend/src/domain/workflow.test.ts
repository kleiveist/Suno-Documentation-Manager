import { describe, expect, it } from "vitest";

import { calculateProgress, evaluateRequirements, visibleConditionalFields } from "./workflow";

import { profile, completeTrack } from "./workflow/test-fixtures";

import { emptyTrackFields } from "./types";

describe("conditional fields", () => {
  it("hides external-audio details until yes", () => {
    const fields = emptyTrackFields(profile);
    expect(visibleConditionalFields(fields, profile)).not.toContain("externalAudioSource");
    fields.externalAudioUploaded = true;
    expect(visibleConditionalFields(fields, profile)).toEqual(expect.objectContaining(new Set()));
    expect([...visibleConditionalFields(fields, profile)]).toEqual(
      expect.arrayContaining([
        "externalAudioSource",
        "externalAudioOwnership",
        "externalAudioFile",
        "externalAudioLicense"
      ])
    );
  });

  it("shows own-audio and sample details only when applicable", () => {
    const fields = emptyTrackFields(profile);
    fields.ownAudioUploaded = true;
    fields.thirdPartySamplesUploaded = true;
    const visible = [...visibleConditionalFields(fields, profile)];
    expect(visible).toEqual(
      expect.arrayContaining([
        "ownAudioSource",
        "ownAudioOwnership",
        "ownAudioFile",
        "thirdPartySampleSource",
        "thirdPartySampleOwnership",
        "thirdPartySampleFile",
        "thirdPartySampleLicense"
      ])
    );
  });

  it("shows source-code and generated-audio uploads only for code-based generation", () => {
    const fields = emptyTrackFields(profile);
    expect(visibleConditionalFields(fields, profile)).not.toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).not.toContain("codeGeneratedAudioFile");
    fields.codeBasedGeneration = false;
    expect(visibleConditionalFields(fields, profile)).not.toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).not.toContain("codeGeneratedAudioFile");
    fields.codeBasedGeneration = true;
    expect(visibleConditionalFields(fields, profile)).toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).toContain("codeGeneratedAudioFile");
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessed");
    fields.codeAudioPostProcessed = true;
    fields.codeAudioPostProcessingOperations = ["Mixing", "Other post-processing"];
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessingOperations");
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessingNote");
  });

  it("shows AI and content-check follow-ups conditionally", () => {
    const fields = emptyTrackFields(profile);
    fields.artworkOrigin = "ai_assisted";
    fields.depictsRealPerson = true;
    fields.depictsRealEvent = true;
    fields.containsTrademark = true;
    const visible = [...visibleConditionalFields(fields, profile)];
    expect(visible).toEqual(
      expect.arrayContaining([
        "aiImageService",
        "aiArtworkOriginal",
        "disclosure",
        "humanArtworkModifications",
        "realPersonNotes",
        "realEventNotes",
        "trademarkNotes"
      ])
    );
  });
});

describe("progress", () => {
  it("uses completed applicable requirements as the denominator", () => {
    const track = completeTrack();
    expect(calculateProgress(evaluateRequirements(track, profile))).toBe(100);
    track.fields.externalAudioUploaded = true;
    expect(calculateProgress(evaluateRequirements(track, profile))).toBeLessThan(100);
  });

  it("excludes non-applicable branches from the denominator", () => {
    const base = completeTrack();
    const baseCount = evaluateRequirements(base, profile).length;
    base.fields.externalAudioUploaded = true;
    expect(evaluateRequirements(base, profile).length).toBe(baseCount + 4);
  });
});
