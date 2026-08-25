import { describe, expect, it } from "vitest";

import {
  automaticConsistencyPresentation,
  finalizationGate,
  humanEditedFinalArtworkStatus,
  evidenceRoleFileTypes
} from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

describe("statuses and finalization", () => {
  it("classifies finalized timestamp states without turning them into workflow blockers", () => {
    const track = completeTrack();
    track.status = "FINALIZED";
    track.certificate = { valid: true, certificateId: "certificate-1" };
    track.externalTimestampSummary = {
      status: "attached",
      message: "OpenTimestamps proof attached; verification is still pending.",
      provider: "OpenTimestamps"
    };

    expect(automaticConsistencyPresentation(track)).toEqual(
      expect.objectContaining({
        outcome: "PASS WITH WARNINGS",
        warningCount: 1,
        blockingCount: 0
      })
    );
    expect(automaticConsistencyPresentation(track).findings).toContainEqual(
      expect.objectContaining({
        code: "external_timestamp_attached_unverified",
        level: "WARNING"
      })
    );
    expect(finalizationGate(track, profile).valid).toBe(true);

    for (const status of [
      "verification_failed",
      "provider_unavailable",
      "authentication_failed",
      "anchor_mismatch",
      "configuration_incomplete",
      "authentication_required",
      "connection_failed",
      "unsupported_response",
      "verification_configuration_incomplete"
    ] as const) {
      track.externalTimestampSummary.status = status;
      const failed = automaticConsistencyPresentation(track);
      expect(failed.outcome, status).toBe("PASS WITH WARNINGS");
      expect(failed.findings, status).toContainEqual(
        expect.objectContaining({
          code: `external_timestamp_${status}`,
          level: "WARNING"
        })
      );
    }

    track.externalTimestampSummary = {
      status: "not_recorded",
      message: "No external timestamp evidence recorded.",
      provider: ""
    };
    const notRecorded = automaticConsistencyPresentation(track);
    expect(notRecorded.outcome).toBe("PASS");
    expect(notRecorded.findings).toContainEqual(
      expect.objectContaining({
        code: "external_timestamp_not_recorded",
        level: "INFO"
      })
    );
  });

  it("reports a human-edited/final artwork SHA-256 match as INFO", () => {
    const track = completeTrack();
    track.evidence.push(evidence("human_edited_artwork"), evidence("final_artwork"));

    const presentation = automaticConsistencyPresentation(track);

    expect(presentation.outcome).toBe("PASS");
    expect(presentation.findings).toContainEqual(
      expect.objectContaining({
        code: "human_edited_final_artwork_sha256_match",
        level: "INFO",
        stepId: "artwork"
      })
    );
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("reports the human-edited/final artwork hash comparison by role", () => {
    const humanEdited = evidence("human_edited_artwork");
    const finalArtwork = evidence("final_artwork");
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork])).toBe("BYTE-IDENTICAL / SHA-256 MATCH");
    finalArtwork.sha256 = "b".repeat(64);
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork])).toBe("NO SHA-256 MATCH");
    finalArtwork.verified = false;
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork])).toBe("NOT VERIFIED");
  });

  it("lists the accepted file types directly for every evidence role", () => {
    expect(evidenceRoleFileTypes("suno_project_zip")).toBe("ZIP");
    expect(evidenceRoleFileTypes("suno_screenshot")).toContain("PNG");
    expect(evidenceRoleFileTypes("release_wav")).toContain("FLAC");
    expect(evidenceRoleFileTypes("source_code_file")).toContain("Python");
    expect(evidenceRoleFileTypes("code_generated_audio_file")).toBe("WAV oder MP3");
  });
});
