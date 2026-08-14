import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "./demo";

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

afterEach(() => vi.useRealTimers());

describe("demo track library", () => {
  it("reclassifies a track by moving its folder without changing documentation state", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));

    const updated = await settle(api.updateTrackLibrary("gravity", {
      section: "album",
      albumTitle: "  Northern Lights  "
    }));
    const { library: _beforeLibrary, relativePath: _beforePath, ...beforeState } = before;
    const { library: _updatedLibrary, relativePath: _updatedPath, ...updatedState } = updated;

    expect(updated.library).toEqual({ section: "album", albumTitle: "Northern Lights" });
    expect(updated.relativePath).toBe("Northern Lights/Gravity");
    expect(updatedState).toEqual(beforeState);
  });

  it("renames an album folder and every contained track path", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const tracks = await settle(api.renameAlbum("Event Horizon", "Gravity Drift"));
    const gravity = tracks.find((track) => track.id === "gravity");

    expect(gravity?.library).toEqual({ section: "album", albumTitle: "Gravity Drift" });
    expect(gravity?.relativePath).toBe("Gravity Drift/Gravity");
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
