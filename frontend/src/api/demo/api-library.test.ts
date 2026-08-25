import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "../demo";

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

describe("demo library discovery", () => {
  it("lists summary data, albums, cover availability, and browser import capabilities", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const tracks = await settle(api.listTracks());
    expect(tracks).toHaveLength(2);
    expect(tracks[0]).not.toHaveProperty("fields");
    expect(await settle(api.listAlbums())).toEqual(["Event Horizon"]);
    expect(await settle(api.loadTrackCover("gravity"))).toEqual(
      expect.objectContaining({
        evidenceId: expect.any(String),
        dataUrl: expect.stringMatching(/^data:image\/png;base64,/)
      })
    );
    expect(await settle(api.loadTrackCover("cosmic-pulse"))).toBeNull();
    expect(await settle(api.scanImportFolder())).toBeNull();
    await expect(
      api.executeFolderImport({
        sourcePath: "/demo/import",
        expectedKind: "single",
        productionStartDate: "2026-08-20"
      })
    ).rejects.toThrow("nur in der Desktop-App");
  });

  it("rejects invalid and duplicate album operations", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.createAlbum("x".repeat(201)), "Albumtitel ist ungültig");
    await expectTimedRejection(api.createAlbum(" event horizon "), "existiert bereits");
    await expectTimedRejection(api.renameAlbum("Missing", "Future"), "Album nicht gefunden");
    await settle(api.createAlbum("Occupied"));
    await expectTimedRejection(api.renameAlbum("Event Horizon", "Occupied"), "existiert bereits");
    await expectTimedRejection(
      api.createTrack({
        title: "Invalid album track",
        productionStartDate: "2026-08-20",
        commercialUseIntended: false,
        library: { section: "album" }
      }),
      "Albumtitel erforderlich"
    );
  });
});

describe("demo track maintenance", () => {
  it("updates titles, profile snapshots, deviations, and workflow step state", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const renamed = await settle(api.updateTrack("gravity", { title: "Gravity Prime" }));
    expect(renamed.relativePath).toBe("Event Horizon/Gravity Prime");
    expect(renamed.evidence.find((item) => item.role === "release_wav")?.fileName).toBe("Gravity Prime.wav");

    const profile = await settle(api.getProfile());
    await settle(api.updateProfile({ ...profile, artistName: "New Artist" }));
    expect((await settle(api.adoptLegacyProfile("cosmic-pulse"))).profileSnapshot.artistName).toBe("New Artist");

    const withDeviation = await settle(api.addDeviation("cosmic-pulse", "Needs review", true));
    const deviationId = withDeviation.blockingDeviations?.[0].id ?? "";
    const resolved = await settle(api.resolveDeviation("cosmic-pulse", deviationId));
    expect(resolved.blockingDeviations?.[0].resolved).toBe(true);
    await settle(api.resolveDeviation("cosmic-pulse", "missing"));
    expect((await settle(api.removeDeviation("cosmic-pulse", deviationId))).blockingDeviations).toEqual([]);

    const inserted = await settle(api.setStepStatus("cosmic-pulse", "track", "PASS"));
    expect(inserted.steps.map((step) => step.id)).toContain("track");
    const updated = await settle(api.setStepStatus("cosmic-pulse", "track", "N_A", "Test reason"));
    expect(updated.steps).toHaveLength(10);
  });

  it("enforces track lookup and album assignment validation", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.loadTrack("missing"), "nicht gefunden");
    await expectTimedRejection(
      api.updateTrackLibrary("gravity", { section: "album", albumTitle: "x".repeat(201) }),
      "Albumtitel erforderlich"
    );
  });
});

describe("demo evidence maintenance", () => {
  it("rejects duplicates and invalid replacements", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.importEvidence("gravity", "release_wav"), "bereits belegt");
    await expectTimedRejection(api.importEvidence("gravity", "source_code_file", "missing"), "nicht gefunden");
    await expectTimedRejection(api.previewEvidence("gravity", "missing"), "nicht gefunden");
  });

  it("imports, previews, verifies, removes evidence, and previews document generation", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const imported = await settle(api.importEvidence("cosmic-pulse", "source_code_file"));
    const source = imported!.evidence.find((item) => item.role === "source_code_file")!;
    const preview = await settle(api.previewEvidence("cosmic-pulse", source.id));
    expect(preview).toEqual(
      expect.objectContaining({
        mimeType: undefined,
        dataUrl: undefined,
        message: expect.stringContaining("keine Vorschau")
      })
    );

    const verified = await settle(api.verifyEvidence("cosmic-pulse", ""));
    expect(verified.evidence.every((item) => item.verified)).toBe(true);
    const removed = await settle(api.removeEvidence("cosmic-pulse", source.id));
    expect(removed.evidence.some((item) => item.id === source.id)).toBe(false);

    expect(await settle(api.previewDocumentGeneration("cosmic-pulse"))).toEqual(
      expect.objectContaining({ files: expect.arrayContaining(["03_DOCUMENTATION/README.md"]), collisions: [] })
    );
  });
});
