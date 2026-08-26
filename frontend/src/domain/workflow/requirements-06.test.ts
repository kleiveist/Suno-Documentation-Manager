import { describe, expect, it } from "vitest";

import { calculateMissingRequirements } from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

describe("missing requirements", () => {
  it("TEST 16 distinguishes NO, NOT DOCUMENTED and deterministically non-applicable answers", () => {
    const track = completeTrack();
    track.fields.generativeAiUsed = true;
    track.fields.audioAiSystem = "Suno";
    track.fields.aiAssistedAudioElements = "yes";
    track.fields.aiGeneratedAudioElements = "no";
    track.fields.realPersonVoiceIntentionallyImitated = "not_documented";
    track.fields.realPersonIdentityIntentionallyRepresented = "no";
    track.fields.realEventRepresentedAsAuthenticRecording = "no";
    track.fields.realLocationInstitutionEventPresentedAsAuthenticAiRecording = "no";
    track.fields.audioDisclosureApplied = "no";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("voice-imitation");
    track.fields.realPersonVoiceIntentionallyImitated = null;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("voice-imitation");
    track.fields.generativeAiUsed = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("voice-imitation");
  });

  it("surfaces every attached but missing or unverified evidence item", () => {
    const track = completeTrack();
    const optional = evidence("other");
    optional.verified = false;
    optional.verificationError = "Evidence file is missing.";
    track.evidence.push(optional);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain(
      `unverified-evidence-${optional.id}`
    );
  });
});
