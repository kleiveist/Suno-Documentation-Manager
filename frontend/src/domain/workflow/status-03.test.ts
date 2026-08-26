import { describe, expect, it } from "vitest";

import {
  calculateMissingRequirements,
  deriveStepStatus,
  finalizationGate,
  statusLabel,
  stepStatuses
} from "../workflow";

import { profile, evidence, completeTrack } from "./test-fixtures";

describe("statuses and finalization", () => {
  it("treats three explicit No answers as a completed artwork content check", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "human";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("final_artwork"));

    const artwork = stepStatuses(track, profile).find((step) => step.id === "artwork");
    expect(artwork?.status).toBe("PASS");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toEqual(
      expect.arrayContaining(["real-person-answer", "real-event-answer", "trademark-answer"])
    );
  });

  it("renders and derives supported statuses", () => {
    expect(statusLabel("NOT_RUN")).toBe("Offen");
    expect(statusLabel("PASS")).toBe("Erfüllt");
    expect(statusLabel("FAIL")).toBe("Fehlgeschlagen");
    expect(statusLabel("BLOCKED")).toBe("Blockiert");
    expect(statusLabel("N_A")).toBe("N/A");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A", naReason: "Kein Artwork" }, false)).toBe(
      "N_A"
    );
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A", naReason: "Kein Artwork" }, true)).toBe(
      "PASS"
    );
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A" })).toBe("PASS");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "BLOCKED" })).toBe("PASS");
  });

  it("keeps Finalize blocked until every preceding step is complete", () => {
    const track = completeTrack();
    track.fields.productionEndDate = "";
    const statuses = stepStatuses(track, profile);
    expect(statuses.find((step) => step.id === "track")?.status).not.toBe("PASS");
    expect(statuses.find((step) => step.id === "finalize")?.status).toBe("BLOCKED");
  });

  it("blocks finalization for stale documents, mismatches and unresolved deviations", () => {
    const track = completeTrack();
    track.documents.current = false;
    track.integrity.mismatchFiles = ["01_RELEASE/final.wav"];
    track.blockingDeviations = [
      {
        id: "dev",
        title: "Blocker",
        description: "Quelle ungeklärt",
        blocking: true,
        resolved: false,
        createdAt: "2026-08-01"
      }
    ];
    const gate = finalizationGate(track, profile);
    expect(gate.valid).toBe(false);
    expect(gate.missingItems).toContain("Aktuelle generierte Dokumente");
    expect(gate.blockingItems.join(" ")).toContain("Quelle ungeklärt");
    expect(gate.blockingItems.join(" ")).toContain("final.wav");
  });

  it("allows finalization only when every applicable requirement is complete", () => {
    expect(finalizationGate(completeTrack(), profile)).toEqual({ valid: true, missingItems: [], blockingItems: [] });
  });
});
