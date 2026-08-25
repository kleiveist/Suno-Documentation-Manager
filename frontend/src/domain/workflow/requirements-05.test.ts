import { describe, expect, it } from "vitest";

import { calculateMissingRequirements, contentCheckAllNegative, visibleConditionalFields } from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

import { type TrackDetail } from "../types";

describe("missing requirements", () => {
  it("TEST 10 accepts a deliberate NO disclosure with an optional factual reason", () => {
    const track = completeTrack();
    Object.assign(track.fields, {
      generativeAiUsed: true,
      audioAiSystem: "Suno",
      aiAssistedAudioElements: "yes",
      aiGeneratedAudioElements: "yes",
      realPersonVoiceIntentionallyImitated: "no",
      realPersonIdentityIntentionallyRepresented: "no",
      realEventRepresentedAsAuthenticRecording: "no",
      realLocationInstitutionEventPresentedAsAuthenticAiRecording: "no",
      audioDisclosureApplied: "no",
      audioDisclosureReason: "User-confirmed publication decision"
    } satisfies Partial<TrackDetail["fields"]>);

    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("audio-disclosure-status");
    expect(ids).not.toContain("audio-disclosure-locations");
    expect(ids).not.toContain("audio-disclosure-text");
  });

  it("evaluates code-audio post-processing only on the applicable branch", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processed-answer"
    );

    track.fields.codeBasedGeneration = true;
    track.fields.codeAudioPostProcessed = false;
    track.evidence.push(evidence("source_code_file"), evidence("code_generated_audio_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processing-operations"
    );

    track.fields.codeAudioPostProcessed = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain(
      "code-audio-post-processing-operations"
    );
    track.fields.codeAudioPostProcessingOperations = ["Mixing", "EQ", "Mastering"];
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processing-operations"
    );
  });

  it("accepts future and historical free-text Suno model and plan values", () => {
    const track = completeTrack();
    track.fields.sunoModel = "v6";
    track.fields.sunoPlanAtGeneration = "Historical Studio Plan";
    const missing = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(missing).not.toContain("suno-model");
    expect(missing).not.toContain("suno-plan");
  });

  it("requires at least one human change for AI-assisted artwork", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_assisted";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("artwork-human-changes");
    track.fields.humanArtworkModifications = ["Cropping", "Color correction"];
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-human-changes");
  });

  it("requires an explicit artwork disclosure decision even after three negative content checks", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_generated";
    track.fields.aiImageService = "Local Image Tool";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("ai_artwork_original"), evidence("final_artwork"));

    expect(contentCheckAllNegative(track.fields)).toBe(true);
    expect(visibleConditionalFields(track.fields, profile)).toContain("disclosure");
    track.fields.generativeAiUsed = null;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toEqual(
      expect.arrayContaining(["generative-ai-answer", "ai-disclosure-decision"])
    );
    track.fields.generativeAiUsed = false;
    track.fields.disclosureApplied = false;
    expect(calculateMissingRequirements(track, profile).filter((item) => item.stepId === "ai_transparency")).toEqual(
      []
    );
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
  });

  it("requires the YES/NO artwork decision under the none policy and accepts deliberate NO", () => {
    const track = completeTrack();
    const noPolicy = { ...profile, artworkTransparencyPolicy: "none" as const };
    track.fields.artworkOrigin = "ai_generated";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("ai_artwork_original"), evidence("final_artwork"));

    expect(calculateMissingRequirements(track, noPolicy).map((item) => item.id)).toContain("ai-disclosure-decision");
    track.fields.disclosureApplied = false;
    expect(calculateMissingRequirements(track, noPolicy).map((item) => item.id)).not.toContain(
      "ai-disclosure-decision"
    );
  });
});
