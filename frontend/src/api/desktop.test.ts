import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

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
});
