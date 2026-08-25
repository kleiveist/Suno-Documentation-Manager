import { afterEach, describe, expect, it, vi } from "vitest";

import type { TrackStatus } from "../../domain/types";
import { WORKFLOW_VERSION } from "../../domain/workflow";
import { DemoLifecycleApi } from "./api-lifecycle";

class MutableDemoLifecycleApi extends DemoLifecycleApi {
  setLifecycleState(trackId: string, workflowVersion: string, status?: TrackStatus): void {
    const track = this.state.tracks.get(trackId);
    if (!track) throw new Error(`Unknown test track: ${trackId}`);
    track.workflowVersion = workflowVersion;
    if (status) track.status = status;
  }
}

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

async function expectTimedRejection(promise: Promise<unknown>, message: string): Promise<void> {
  const rejection = expect(promise).rejects.toThrow(message);
  await vi.runAllTimersAsync();
  await rejection;
}

afterEach(() => vi.useRealTimers());

describe("demo lifecycle validation", () => {
  it("validates tracks and reports a blocked finalization without mutating the track", async () => {
    vi.useFakeTimers();
    const api = new MutableDemoLifecycleApi();
    await settle(api.openWorkspace());

    const validation = await settle(api.validateTrack("cosmic-pulse"));
    expect(validation.valid).toBe(false);
    expect(validation.missingItems.length + validation.blockingItems.length).toBeGreaterThan(0);

    await expectTimedRejection(api.finalizeTrack("cosmic-pulse"), "Finalisierung blockiert");
    expect((await settle(api.loadTrack("cosmic-pulse"))).status).not.toBe("FINALIZED");
  });

  it("invalidates only finalized certificates", async () => {
    vi.useFakeTimers();
    const api = new MutableDemoLifecycleApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.invalidateCertificate("gravity"), "nicht finalisiert");

    api.setLifecycleState("gravity", WORKFLOW_VERSION, "FINALIZED");
    const invalidated = await settle(api.invalidateCertificate("gravity"));
    expect(invalidated.track?.certificate).toEqual(
      expect.objectContaining({ valid: false, invalidationReason: "Manuell invalidiert" })
    );
    expect(invalidated.track?.certificate.invalidatedAt).toBeTruthy();
  });
});

describe("demo workflow reevaluation", () => {
  it("rejects current and superseded workflows", async () => {
    vi.useFakeTimers();
    const current = new MutableDemoLifecycleApi();
    await settle(current.openWorkspace());
    await expectTimedRejection(current.reEvaluateTrack("gravity"), "bereits die aktuelle Workflow-Version");

    const superseded = new MutableDemoLifecycleApi();
    await settle(superseded.openWorkspace());
    superseded.setLifecycleState("gravity", "legacy", "SUPERSEDED");
    await expectTimedRejection(superseded.reEvaluateTrack("gravity"), "neuere Revision ersetzt");
  });

  it("resets active and archived legacy workflow state", async () => {
    vi.useFakeTimers();
    const active = new MutableDemoLifecycleApi();
    await settle(active.openWorkspace());
    active.setLifecycleState("cosmic-pulse", "legacy", "ACTIVE");
    const activeResult = await settle(active.reEvaluateTrack("cosmic-pulse"));
    expect(activeResult.message).toContain("verwendet jetzt den aktuellen Workflow");
    expect(activeResult.track).toEqual(
      expect.objectContaining({
        workflowVersion: WORKFLOW_VERSION,
        externalTimestamps: [],
        finalizationAnchors: []
      })
    );
    expect(activeResult.track?.certificate.valid).toBe(false);
    expect(activeResult.track?.integrity.generated).toBe(false);

    const archived = new MutableDemoLifecycleApi();
    await settle(archived.openWorkspace());
    archived.setLifecycleState("gravity", "legacy", "FINALIZED");
    const archivedResult = await settle(archived.reEvaluateTrack("gravity"));
    expect(archivedResult.message).toContain("bisherige Snapshot wurde archiviert");
    expect(archivedResult.track?.status).not.toBe("FINALIZED");
  });
});
