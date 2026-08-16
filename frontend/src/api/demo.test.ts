import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "./demo";

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

afterEach(() => vi.useRealTimers());

describe("demo track library", () => {
  it("creates and renames an album before it contains a track", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    expect(await settle(api.createAlbum("  Empty Album  "))).toContain("Empty Album");
    await settle(api.renameAlbum("Empty Album", "Future Album"));

    expect(await settle(api.listAlbums())).toContain("Future Album");
    expect(await settle(api.listAlbums())).not.toContain("Empty Album");
  });

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

describe("demo evidence controls", () => {
  it("replaces one selected record and previews present image evidence", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));
    const original = before.evidence.find((item) => item.role === "ai_artwork_original")!;

    const updated = await settle(api.importEvidence("gravity", "ai_artwork_original", original.id));
    const replacement = updated!.evidence.find((item) => item.id === original.id)!;
    const preview = await settle(api.previewEvidence("gravity", replacement.id));

    expect(updated!.evidence.filter((item) => item.id === original.id)).toHaveLength(1);
    expect(preview.dataUrl).toMatch(/^data:image\/png;base64,/);
  });

  it("registers Suno terms globally and copies them into existing and new projects", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const global = await settle(api.importGlobalTermsEvidence({
      documentTitle: "Suno Terms of Service",
      provider: "Suno",
      sourceUrl: "https://suno.example/terms",
      retrievalDate: "2026-08-16"
    }));
    const existing = await settle(api.loadTrack("gravity"));
    const created = await settle(api.createTrack({
      title: "Later Project",
      productionStartDate: "2026-08-16",
      commercialUseIntended: false,
      library: { section: "single" }
    }));

    for (const track of [existing, created]) {
      expect(track.evidence).toEqual(expect.arrayContaining([
        expect.objectContaining({
          role: "suno_terms_rights",
          sourceGlobalEvidenceId: global!.id,
          provenance: "global_copy",
          metadata: expect.objectContaining({ documentTitle: "Suno Terms of Service" })
        })
      ]));
    }
  });
});
