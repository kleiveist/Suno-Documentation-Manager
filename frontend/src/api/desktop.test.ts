import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  Channel: class<T> {
    constructor(readonly onmessage: (message: T) => void) {}
  }
}));

import { createDesktopApi, DesktopCommandError, isTauriRuntime, toUserMessage } from "./desktop";

beforeEach(() => invokeMock.mockReset());

describe("error presentation", () => {
  it("presents structured, string and unknown errors in German UI-safe text", () => {
    expect(toUserMessage({ message: "Dateikonflikt erkannt" })).toBe("Dateikonflikt erkannt");
    expect(toUserMessage("Pfad liegt außerhalb des Workspace")).toBe("Pfad liegt außerhalb des Workspace");
    expect(toUserMessage({ unexpected: true })).toBe("Die lokale Aktion konnte nicht abgeschlossen werden.");
  });

  it("preserves controlled native command messages", () => {
    const error = new DesktopCommandError("import_evidence", { detail: "Dateityp nicht zulässig" });
    expect(error.message).toBe("Dateityp nicht zulässig");
    expect(error.commandName).toBe("import_evidence");
  });
});

describe("runtime selection", () => {
  it("distinguishes Tauri from the clearly labeled browser demo", () => {
    expect(isTauriRuntime({} as Window)).toBe(false);
    expect(isTauriRuntime({ __TAURI_INTERNALS__: {} } as unknown as Window)).toBe(true);
  });

  it("maps workflow reevaluation to its narrow native command", async () => {
    invokeMock.mockResolvedValue({ message: "ready for reevaluation" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.reEvaluateTrack("track-1")).resolves.toEqual({
      message: "ready for reevaluation"
    });
    expect(invokeMock).toHaveBeenCalledWith("re_evaluate_track", { trackId: "track-1" });
  });

  it("passes the selected subscription billing cycle to the native importer", async () => {
    invokeMock.mockResolvedValue(null);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.importGlobalEvidence("subscription_payment", "2026-08-01", "annual")).resolves.toBeNull();
    expect(invokeMock).toHaveBeenCalledWith("import_global_evidence", {
      role: "subscription_payment",
      coverageStart: "2026-08-01",
      billingCycle: "annual"
    });
  });

  it("opens the dedicated global Suno terms PDF picker without metadata", async () => {
    invokeMock.mockResolvedValue(null);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.importGlobalTermsEvidence()).resolves.toBeNull();
    expect(invokeMock).toHaveBeenCalledWith("import_global_terms_evidence", undefined);
  });

  it("passes a library assignment when creating a track", async () => {
    invokeMock.mockResolvedValue({ id: "track-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    const input = {
      title: "Album Track",
      productionStartDate: "2026-08-14",
      commercialUseIntended: true,
      library: { section: "album" as const, albumTitle: "Northern Lights" }
    };

    await api.createTrack(input);

    expect(invokeMock).toHaveBeenCalledWith("create_track", { input });
  });

  it("maps physical album listing and creation to narrow native commands", async () => {
    invokeMock.mockResolvedValue(["Gravity Drift"]);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.listAlbums()).resolves.toEqual(["Gravity Drift"]);
    await expect(api.createAlbum("Gravity Drift")).resolves.toEqual(["Gravity Drift"]);

    expect(invokeMock).toHaveBeenNthCalledWith(1, "list_albums", undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "create_album", { title: "Gravity Drift" });
  });

  it("maps physical reclassification to the narrow native command", async () => {
    invokeMock.mockResolvedValue({ id: "track-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.updateTrackLibrary("track-1", { section: "single" });

    expect(invokeMock).toHaveBeenCalledWith("update_track_library", {
      trackId: "track-1",
      input: { section: "single" }
    });
  });

  it("maps an album-folder rename to the native command", async () => {
    invokeMock.mockResolvedValue([]);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.renameAlbum("Old Album", "New Album");

    expect(invokeMock).toHaveBeenCalledWith("rename_album", {
      oldTitle: "Old Album",
      newTitle: "New Album"
    });
  });

  it("maps explicit evidence replacement and preview to narrow native commands", async () => {
    invokeMock.mockResolvedValueOnce({ id: "track-1" }).mockResolvedValueOnce({ evidenceId: "evidence-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.importEvidence("track-1", "suno_project_zip", "evidence-1");
    await api.previewEvidence("track-1", "evidence-1");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "import_evidence", {
      trackId: "track-1",
      role: "suno_project_zip",
      replaceEvidenceId: "evidence-1"
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "preview_evidence", {
      trackId: "track-1",
      evidenceId: "evidence-1"
    });
  });

  it("lets the native Suno importer derive technical metadata without user-supplied fields", async () => {
    invokeMock.mockResolvedValue({ id: "track-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.importEvidence("track-1", "suno_final_export");

    expect(invokeMock).toHaveBeenCalledWith("import_evidence", {
      trackId: "track-1",
      role: "suno_final_export",
      replaceEvidenceId: undefined
    });
  });

  it("loads the bounded final-artwork cover through its dedicated command", async () => {
    invokeMock.mockResolvedValue({ evidenceId: "artwork-1", dataUrl: "data:image/png;base64,AA==" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.loadTrackCover("track-1")).resolves.toEqual({
      evidenceId: "artwork-1",
      dataUrl: "data:image/png;base64,AA=="
    });
    expect(invokeMock).toHaveBeenCalledWith("load_track_cover", { trackId: "track-1" });
  });

  it("streams native integrity progress through a scoped IPC channel", async () => {
    const progress = vi.fn();
    invokeMock.mockImplementationOnce(async (_command, args) => {
      args.onProgress.onmessage({ stage: "hashing", processedBytes: 50, totalBytes: 100, processedFiles: 1, totalFiles: 2 });
      return { message: "done" };
    });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.calculateHashes("track-1", progress);

    expect(progress).toHaveBeenCalledWith(expect.objectContaining({ stage: "hashing", processedBytes: 50 }));
    expect(invokeMock).toHaveBeenCalledWith("calculate_hashes", {
      trackId: "track-1",
      onProgress: expect.anything()
    });
  });

  it("streams finalization progress through the certificate command", async () => {
    const progress = vi.fn();
    invokeMock.mockImplementationOnce(async (_command, args) => {
      args.onProgress.onmessage({ stage: "generating_certificate", processedBytes: 0, totalBytes: 0, processedFiles: 4, totalFiles: 4 });
      return { message: "finalized" };
    });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.finalizeTrack("track-1", progress);

    expect(progress).toHaveBeenCalledWith(expect.objectContaining({ stage: "generating_certificate" }));
    expect(invokeMock).toHaveBeenCalledWith("finalize_track", {
      trackId: "track-1",
      onProgress: expect.anything()
    });
  });
});
