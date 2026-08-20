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

  it("localizes known native errors and hides unmapped low-level diagnostics", () => {
    expect(toUserMessage("No workspace is open.", "de")).toBe("Kein Workspace ist geöffnet.");
    expect(toUserMessage("No workspace is open.", "en")).toBe("No workspace is open.");
    expect(toUserMessage("Kein Workspace ist geöffnet.", "de")).toBe("Kein Workspace ist geöffnet.");
    expect(toUserMessage("The operation is blocked: Track title is missing.", "de"))
      .toBe("Der Vorgang ist blockiert: Tracktitel fehlt.");
    expect(toUserMessage("The operation is blocked: Internal driver fault 42", "de"))
      .toBe("Der Vorgang ist blockiert: Technisches Detail ist nicht verfügbar.");
    expect(toUserMessage("File operation failed for releases/My Track.wav: Permission denied", "de"))
      .toBe("Dateioperation für releases/My Track.wav fehlgeschlagen: Technisches Detail ist nicht verfügbar.");
    expect(toUserMessage("Internal driver fault 42", "de"))
      .toBe("Die lokale Aktion konnte nicht abgeschlossen werden.");
    expect(toUserMessage("Interner Treiberfehler 42", "en"))
      .toBe("The local action could not be completed.");
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

  it("passes the active UI language to every native file picker", async () => {
    invokeMock.mockResolvedValue(null);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.openWorkspace("de");
    await api.createWorkspace("en");
    await api.importGlobalEvidence("subscription_payment", "2026-08-01", "annual", "de");
    await api.importGlobalTermsEvidence({}, "en");
    await api.scanImportFolder("de");
    await api.importEvidence("track-1", "suno_final_export", undefined, undefined, "en");

    expect(invokeMock.mock.calls).toEqual([
      ["open_workspace", { language: "de" }],
      ["create_workspace", { language: "en" }],
      ["import_global_evidence", {
        role: "subscription_payment",
        coverageStart: "2026-08-01",
        billingCycle: "annual",
        language: "de"
      }],
      ["import_global_terms_evidence", { metadata: {}, language: "en" }],
      ["scan_import_folder", { language: "de" }],
      ["import_evidence", {
        trackId: "track-1",
        role: "suno_final_export",
        replaceEvidenceId: undefined,
        language: "en"
      }]
    ]);
  });

  it("passes required descriptive metadata to the global Suno terms importer", async () => {
    invokeMock.mockResolvedValue(null);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    const metadata = {
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17"
    };

    await expect(api.importGlobalTermsEvidence(metadata)).resolves.toBeNull();
    expect(invokeMock).toHaveBeenCalledWith("import_global_terms_evidence", { metadata });
  });

  it("updates global terms metadata through its narrow native command", async () => {
    invokeMock.mockResolvedValue({ id: "terms-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    const metadata = { provider: "Suno, Inc.", retrievalDate: "2026-08-17" };

    await api.updateGlobalTermsEvidenceMetadata("terms-1", metadata);

    expect(invokeMock).toHaveBeenCalledWith("update_global_terms_evidence_metadata", {
      evidenceId: "terms-1",
      metadata
    });
  });

  it("uses the configured provider to attach an external timestamp to a finalized track", async () => {
    invokeMock.mockResolvedValue({ id: "track-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    await api.attachExternalTimestamp("track-1");

    expect(invokeMock).toHaveBeenCalledWith("attach_configured_external_timestamp", { trackId: "track-1" });
  });

  it("keeps global timestamp settings and write-only credentials on narrow native commands", async () => {
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    const settings = {
      enabled: true,
      provider: "free_tsa" as const,
      autoAfterFinalization: false,
      custom: {
        providerName: "",
        endpoint: "",
        authenticationMode: "none" as const,
        username: "",
        clientCertificatePath: "",
        caCertificatePath: "",
        policyOid: "",
        timeoutSeconds: 20
      },
      status: "ready" as const,
      statusMessage: "Ready"
    };
    invokeMock
      .mockResolvedValueOnce(settings)
      .mockResolvedValueOnce(settings)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({ status: "ready", message: "Provider reachable" });

    await expect(api.getTimestampSettings()).resolves.toEqual(settings);
    await api.updateTimestampSettings(settings);
    await api.updateTimestampSecret("write-only-token");
    await api.testTimestampProvider();

    expect(invokeMock).toHaveBeenNthCalledWith(1, "get_timestamp_settings", undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_timestamp_settings", { settings });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "update_timestamp_secret", { input: { secret: "write-only-token" } });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "test_timestamp_provider", undefined);
  });

  it("keeps audio-screening settings, write-only credentials and track checks on narrow native commands", async () => {
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);
    const settings = {
      enabled: true,
      host: "identify-eu-west-1.acrcloud.com",
      timeoutSeconds: 30,
      status: "ready" as const,
      statusMessage: "Ready",
      credentialsConfigured: true,
      localEngineAvailable: true,
      localEngineVersion: "1.6.1"
    };
    invokeMock.mockResolvedValue({ message: "done" });

    await api.getAudioScreeningSettings();
    await api.updateAudioScreeningSettings(settings);
    await api.updateAudioScreeningSecret({ accessKey: "write-only-key", accessSecret: "write-only-secret" });
    await api.testAudioScreeningProvider();
    await api.runLocalAudioScreening("track-1");
    await api.runExternalAudioScreening("track-1");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "get_audio_screening_settings", undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_audio_screening_settings", { settings });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "update_audio_screening_secret", {
      input: { accessKey: "write-only-key", accessSecret: "write-only-secret" }
    });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "test_audio_screening_provider", undefined);
    expect(invokeMock.mock.calls[4]).toEqual(["run_local_audio_screening", expect.objectContaining({ trackId: "track-1", onProgress: expect.anything() })]);
    expect(invokeMock.mock.calls[5]).toEqual(["run_external_audio_screening", expect.objectContaining({ trackId: "track-1", onProgress: expect.anything() })]);
  });

  it("preserves an explicit null in a track patch sent to the native command", async () => {
    invokeMock.mockResolvedValue({ id: "track-1" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.updateTrack("track-1", {
      generativeAiUsed: null,
      sunoContentClassification: null,
      vocalIntent: null
    });

    expect(invokeMock).toHaveBeenCalledWith("update_track", {
      trackId: "track-1",
      input: {
        generativeAiUsed: null,
        sunoContentClassification: null,
        vocalIntent: null
      }
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

  it("uses narrow native commands for folder scan and confirmed import settings", async () => {
    const proposal = {
      sourcePath: "/source/Awakening",
      kind: "single" as const,
      tracks: [],
      unassignedFiles: []
    };
    const input = {
      sourcePath: proposal.sourcePath,
      expectedKind: proposal.kind,
      singleTrackTitle: "Awakening",
      singleTrackLibrary: { section: "album" as const, albumTitle: "Chosen Album" },
      productionStartDate: "",
      commercialUseIntended: false
    };
    invokeMock.mockResolvedValueOnce(proposal).mockResolvedValueOnce([]);
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await expect(api.scanImportFolder()).resolves.toEqual(proposal);
    await expect(api.executeFolderImport(input)).resolves.toEqual([]);

    expect(invokeMock).toHaveBeenNthCalledWith(1, "scan_import_folder", undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "execute_folder_import", { input });
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

    await api.finalizeTrack("track-1", undefined, progress);

    expect(progress).toHaveBeenCalledWith(expect.objectContaining({ stage: "generating_certificate" }));
    expect(invokeMock).toHaveBeenCalledWith("finalize_track", {
      trackId: "track-1",
      onProgress: expect.anything()
    });
  });

  it("passes transient bilingual finalization options to the certificate command", async () => {
    invokeMock.mockResolvedValue({ message: "finalized" });
    const api = createDesktopApi({ __TAURI_INTERNALS__: {} } as unknown as Window);

    await api.finalizeTrack("track-1", { bilingual: true });

    expect(invokeMock).toHaveBeenCalledWith("finalize_track", {
      trackId: "track-1",
      options: { bilingual: true },
      onProgress: expect.anything()
    });
  });
});
