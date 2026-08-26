import { describe, expect, it } from "vitest";

import {
  calculateMissingRequirements,
  subscriptionEvidenceRelevance,
  subscriptionProductionCoverageStatus
} from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

describe("missing requirements", () => {
  it("TEST 04/05 requires complete core metadata for commercial Terms evidence", () => {
    const track = completeTrack();
    track.fields.commercialUseIntended = true;
    const subscription = evidence("subscription_payment");
    subscription.sourceGlobalEvidenceId = "global-subscription";
    subscription.coverageStart = "2026-07-01";
    subscription.coverageEnd = "2026-07-02";
    track.evidence.push(subscription);
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("subscription-generation-coverage");
    expect(ids).toContain("terms-evidence");
    subscription.coverageEnd = "2026-07-31";
    track.fields.sunoTermsEvidenceNotAvailable = true;
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("subscription-generation-coverage");
    expect(ids).toContain("terms-evidence");
    const terms = evidence("suno_terms_rights");
    track.evidence.push(terms);
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("terms-evidence");
    terms.metadata = {
      ...terms.metadata!,
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17"
    };
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("terms-evidence");
    track.fields.sunoTermsEvidenceNotAvailable = false;
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("terms-evidence");
  });

  it("requires source, ownership, file and license on positive source branches", () => {
    const track = completeTrack();
    track.fields.thirdPartySamplesUploaded = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.evidenceRole)).toEqual(
      expect.arrayContaining(["third_party_sample_file", "third_party_sample_license"])
    );
  });

  it("requires source-code and generated WAV/MP3 evidence only after an explicit Yes answer", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("source-code-file");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-generated-audio-file"
    );
    track.fields.codeBasedGeneration = true;
    track.fields.codeAudioPostProcessed = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toEqual(
      expect.arrayContaining(["source-code-file", "code-generated-audio-file"])
    );
    track.evidence.push(evidence("source_code_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("source-code-file");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("code-generated-audio-file");
    track.evidence.push(evidence("code_generated_audio_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-generated-audio-file"
    );
  });

  it("requires reusable subscription evidence only for commercial tracks", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("subscription-evidence");
    track.fields.commercialUseIntended = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("subscription-evidence");
    const subscription = evidence("subscription_payment");
    subscription.sourceGlobalEvidenceId = "global-subscription";
    subscription.coverageStart = "2026-07-01";
    subscription.coverageEnd = "2026-07-31";
    track.evidence.push(subscription);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("subscription-evidence");
    subscription.coverageEnd = "2026-07-02";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("subscription-evidence");
  });

  it("combines adjacent subscription receipts and accepts a generation-only relevant receipt", () => {
    const track = completeTrack();
    track.fields.productionStartDate = "2026-07-18";
    track.fields.productionEndDate = "2026-08-17";
    track.fields.sunoFinalGenerationDate = "2026-08-17";
    const july = evidence("subscription_payment");
    july.sourceGlobalEvidenceId = "july";
    july.coverageStart = "2026-07-14";
    july.coverageEnd = "2026-08-13";
    const august = evidence("subscription_payment");
    august.sourceGlobalEvidenceId = "august";
    august.coverageStart = "2026-08-14";
    august.coverageEnd = "2026-09-13";

    expect(subscriptionEvidenceRelevance(august, track.fields)).toEqual({
      relevant: true,
      coversProduction: false,
      overlapsProduction: true,
      coversGeneration: true
    });
    expect(subscriptionProductionCoverageStatus([july], track.fields)).toBe("NO");
    expect(subscriptionProductionCoverageStatus([july, august], track.fields)).toBe("YES");
  });
});
