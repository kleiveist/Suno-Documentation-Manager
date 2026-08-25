import { describe, expect, it } from "vitest";

import { calculateMissingRequirements, subscriptionGenerationCoverageStatus, WORKFLOW_VERSION } from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

import { type TrackDetail } from "../types";

describe("missing requirements", () => {
  it("TEST 06/07 records the plan at generation and evaluates date coverage technically", () => {
    const track = completeTrack();
    track.fields.sunoPlanAtGeneration = "Premier";
    track.fields.sunoFinalGenerationDate = "2026-08-15";
    const covered = evidence("subscription_payment");
    covered.coverageStart = "2026-08-14";
    covered.coverageEnd = "2026-09-13";
    expect(subscriptionGenerationCoverageStatus([covered], track.fields)).toBe("YES");
    covered.coverageEnd = "2026-08-13";
    expect(subscriptionGenerationCoverageStatus([covered], track.fields)).toBe("NO");
    expect(WORKFLOW_VERSION).toBe("1.9");
  });

  it("requires a verified generated disclosure artifact for AI artwork", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_assisted";
    track.fields.humanArtworkModifications = ["Prompt written manually"];
    track.fields.aiImageService = "Local Image Tool";
    track.fields.disclosureApplied = true;
    track.fields.disclosureText = "AI-assisted";
    track.fields.depictsRealPerson = true;
    track.fields.realPersonNotes = "Fiktive Bearbeitung einer realen Person";
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    const independentFinal = evidence("final_artwork");
    independentFinal.sha256 = "b".repeat(64);
    track.evidence.push(evidence("ai_artwork_original"), independentFinal);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("ai-disclosure-decision");
    const original = track.evidence.find((item) => item.role === "ai_artwork_original")!;
    const disclosed = {
      ...evidence("ai_artwork_edited"),
      provenance: "generated_disclosure" as const,
      derivedFromEvidenceId: original.id,
      generatorVersion: "local-disclosure-v1",
      generatedDisclosureText: "AI-assisted"
    };
    track.evidence.push(disclosed);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("ai-disclosure-decision");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
    const final = track.evidence.find((item) => item.role === "final_artwork")!;
    final.sha256 = disclosed.sha256;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
  });

  it("TEST 08 completes the factual audio AI transparency questionnaire without legal conclusions", () => {
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
      audioDisclosureApplied: "yes",
      audioDisclosureLocations: ["release metadata"],
      audioDisclosureText: "AI-generated audio"
    } satisfies Partial<TrackDetail["fields"]>);

    expect(calculateMissingRequirements(track, profile).filter((item) => item.stepId === "ai_transparency")).toEqual(
      []
    );
  });

  it("TEST 09 keeps commercial generative-AI disclosure NOT DOCUMENTED visibly incomplete", () => {
    const track = completeTrack();
    Object.assign(track.fields, {
      commercialUseIntended: true,
      generativeAiUsed: true,
      audioAiSystem: "Suno",
      aiAssistedAudioElements: "yes",
      aiGeneratedAudioElements: "yes",
      realPersonVoiceIntentionallyImitated: "no",
      realPersonIdentityIntentionallyRepresented: "no",
      realEventRepresentedAsAuthenticRecording: "no",
      realLocationInstitutionEventPresentedAsAuthenticAiRecording: "no",
      audioDisclosureApplied: "not_documented"
    } satisfies Partial<TrackDetail["fields"]>);

    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("audio-disclosure-status");
  });
});
