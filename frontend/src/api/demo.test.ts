import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "./demo";

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

afterEach(() => vi.useRealTimers());

describe("demo track library", () => {
  it("reclassifies a track without changing its documentation state or timestamp", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));

    const updated = await settle(api.updateTrackLibrary("gravity", {
      section: "album",
      albumTitle: "  Northern Lights  "
    }));
    const { library: _beforeLibrary, ...beforeState } = before;
    const { library: _updatedLibrary, ...updatedState } = updated;

    expect(updated.library).toEqual({ section: "album", albumTitle: "Northern Lights" });
    expect(updatedState).toEqual(beforeState);
  });

  it("uses the same album-title validation boundary as the native adapter", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const update = api.updateTrackLibrary("gravity", {
      section: "album",
      albumTitle: "x".repeat(201)
    });
    const rejected = expect(update).rejects.toThrow("Albumtitel");
    await vi.runAllTimersAsync();
    await rejected;
  });
});
