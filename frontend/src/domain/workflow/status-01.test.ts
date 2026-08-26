import { describe, expect, it } from "vitest";

import { automaticConsistencyPresentation, finalizationGate } from "../workflow";

import { profile, completeTrack } from "./test-fixtures";

describe("statuses and finalization", () => {
  it("keeps informational findings presentation-only and leaves the gate valid", () => {
    const track = completeTrack();

    const presentation = automaticConsistencyPresentation(track);

    expect(presentation.outcome).toBe("PASS");
    expect(presentation.findings).toContainEqual(
      expect.objectContaining({
        code: "suno_metadata_not_detected",
        level: "INFO",
        stepId: "suno"
      })
    );
    expect(presentation.infoCount).toBeGreaterThan(0);
    expect(presentation.warningCount).toBe(0);
    expect(presentation.blockingCount).toBe(0);
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("reports non-blocking notes as PASS WITH WARNINGS without changing completion semantics", () => {
    const track = completeTrack();
    track.blockingDeviations = [
      {
        id: "note-1",
        title: "Note",
        description: "Optionale technische Nachprüfung empfohlen.",
        blocking: false,
        resolved: false,
        createdAt: "2026-08-01T10:00:00Z"
      }
    ];

    const presentation = automaticConsistencyPresentation(track);

    expect(presentation.outcome).toBe("PASS WITH WARNINGS");
    expect(presentation.findings).toContainEqual(
      expect.objectContaining({
        code: "deviation:note-1",
        level: "WARNING",
        stepId: "finalize",
        userProvided: true
      })
    );
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("keeps native blocking consistency issues visibly BLOCKED and gate-authoritative", () => {
    const track = completeTrack();
    track.automation.consistencyIssues = [
      {
        code: "suno_stored_metadata_mismatch",
        message: "Gespeicherte und eingebettete Metadaten widersprechen sich.",
        stepId: "suno",
        blocking: true
      }
    ];

    const presentation = automaticConsistencyPresentation(track);

    expect(presentation.outcome).toBe("BLOCKED");
    expect(presentation.findings[0]).toEqual(
      expect.objectContaining({
        code: "consistency:suno_stored_metadata_mismatch",
        level: "BLOCKING"
      })
    );
    expect(finalizationGate(track, profile).valid).toBe(false);
  });
});
