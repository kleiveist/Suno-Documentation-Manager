import { describe, expect, it } from "vitest";

import {
  MAIN_NAVIGATION,
  canonicalGuidedChoiceList,
  canonicalGuidedChoiceValue,
  missingProfileFields,
  normalizeGuidedTrackFields,
  parseMultiChoiceValue,
  resetWorkspaceScopedUiState,
  shouldIgnoreModalBackdropClick,
  serializeMultiChoiceValue,
  trackSummaryFromDetail,
  type GuidedChoice,
  type WorkspaceScopedUiState,
  workflowUpgradePresentation
} from "./app";
import { WORKFLOW_STEPS } from "./domain/workflow";
import { emptyProfile, emptyTrackFields } from "./domain/types";
import { resolveTheme, storedTheme, toggledTheme } from "./ui/theme";

describe("theme", () => {
  it("uses a valid saved choice before the operating-system preference", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("falls back to the operating-system preference for missing or invalid storage", () => {
    expect(resolveTheme(null, true)).toBe("dark");
    expect(resolveTheme("unsupported", false)).toBe("light");
    expect(storedTheme("unsupported")).toBeNull();
  });

  it("toggles between both supported themes", () => {
    expect(toggledTheme("light")).toBe("dark");
    expect(toggledTheme("dark")).toBe("light");
  });
});

describe("navigation", () => {
  it("stores multiple guided choices deterministically", () => {
    expect(serializeMultiChoiceValue(["Mixing", "Mastering", "Mixing"])).toBe("Mixing | Mastering");
    expect(parseMultiChoiceValue("Mixing | Mastering")).toEqual(["Mixing", "Mastering"]);
  });

  it("stores English values while accepting localized labels and retaining unknown legacy data", () => {
    const choices: readonly GuidedChoice[] = [
      ["Timing and cuts", "Timing und Cuts"],
      ["Loudness adjustment", "Lautheitsanpassung"]
    ];
    expect(canonicalGuidedChoiceValue("Timing und Cuts", choices)).toBe("Timing and cuts");
    expect(canonicalGuidedChoiceList("Timing und Cuts | Lautheitsanpassung", choices)).toBe(
      "Timing and cuts | Loudness adjustment"
    );
    expect(canonicalGuidedChoiceValue("Historischer Freitext", choices)).toBe("Historischer Freitext");
  });

  it("normalizes every guided track value before saving", () => {
    const normalized = normalizeGuidedTrackFields({
      ...emptyTrackFields(),
      ownAudioSource: "Eigene Aufnahme",
      ownAudioOwnership: "Eigene Produktion",
      humanEditingDetails: "Timing und Cuts | Lautheitsanpassung",
      postExportEditingDetails: "Schnitt | Mixing",
      releaseNotes: "Originale Suno-Fassung | Radio Edit"
    });

    expect(normalized.ownAudioSource).toBe("Original field recording");
    expect(normalized.ownAudioOwnership).toBe("Solely owned by the artist");
    expect(normalized.humanEditingDetails).toBe("Timing and cuts | Loudness adjustment");
    expect(normalized.postExportEditingDetails).toBe("Editing and cuts | Mixing");
    expect(normalized.releaseNotes).toBe("Original Suno version | Radio edit");
  });

  it("exposes every required German main view", () => {
    expect(MAIN_NAVIGATION.map((item) => [item.id, item.label])).toEqual([
      ["dashboard", "Dashboard"], ["tracks", "Tracks"], ["current", "Aktueller Track"],
      ["workspace", "Workspace"], ["settings", "Einstellungen"]
    ]);
  });

  it("prevents creation of an immutable track snapshot from incomplete global data", () => {
    expect(missingProfileFields(emptyProfile)).toEqual(expect.arrayContaining([
      "Künstlername", "Suno-Profilname", "Suno-Benutzername", "Suno-Tarif", "Abo-Startdatum", "Standard-KI-Bilddienst"
    ]));
    expect(missingProfileFields({
      ...emptyProfile,
      artistName: "Artist", sunoProfileName: "Profile", sunoHandle: "@artist", sunoPlan: "Premier",
      subscriptionStartDate: "2026-01-01", defaultAiImageService: "Local Tool"
    })).toEqual([]);
  });

  it("makes all ten workflow steps reachable in their declared order", () => {
    expect(WORKFLOW_STEPS.map((step) => step.id)).toEqual([
      "track", "source", "suno", "human_work", "artwork", "ai_transparency",
      "release", "evidence_licenses", "integrity", "finalize"
    ]);
  });

  it("clears every workspace-scoped selection before entering another workspace", () => {
    const previous: WorkspaceScopedUiState = {
      track: { id: "old-track" } as WorkspaceScopedUiState["track"],
      trackDraft: { title: "Old track draft" } as WorkspaceScopedUiState["trackDraft"],
      activeStep: "release",
      trackTab: "certificate",
      scanResult: { discovered: 1, indexed: 1, unchanged: 0, warnings: [] },
      albums: ["Old Album"],
      showNewTrack: true,
      showTrackLibrary: true,
      showSubscriptionEvidence: true,
      evidencePreview: {
        evidenceId: "preview",
        role: "suno_screenshot",
        fileName: "preview.png",
        relativePath: "02_SUNO/preview.png",
        sizeBytes: 42
      },
      query: "old workspace query",
      trackFilter: "finalized",
      draftDirty: true
    };

    const reset = resetWorkspaceScopedUiState(previous);

    expect(reset).toEqual({
      track: null,
      trackDraft: null,
      activeStep: null,
      trackTab: "overview",
      scanResult: null,
      albums: [],
      showNewTrack: false,
      showTrackLibrary: false,
      showSubscriptionEvidence: false,
      evidencePreview: null,
      query: "",
      trackFilter: "all",
      draftDirty: false
    });
    expect(previous.track).not.toBeNull();
    expect(previous.trackDraft).not.toBeNull();
    expect(previous.activeStep).toBe("release");
    expect(previous.trackTab).toBe("certificate");
    expect(previous.scanResult).not.toBeNull();
    expect(previous.draftDirty).toBe(true);
  });

  it("ignores delegated backdrop actions for clicks inside a modal", () => {
    expect(shouldIgnoreModalBackdropClick(true, false)).toBe(true);
    expect(shouldIgnoreModalBackdropClick(true, true)).toBe(false);
    expect(shouldIgnoreModalBackdropClick(false, false)).toBe(false);
  });

  it("keeps a changed library assignment when applying a track detail", () => {
    const summary = trackSummaryFromDetail({
      id: "track-1",
      title: "Album Track",
      relativePath: "album-track",
      library: { section: "album", albumTitle: "Northern Lights" },
      status: "FINALIZED",
      updatedAt: "2026-08-14T10:00:00Z",
      progress: 100,
      missingCount: 0,
      certificate: { valid: true }
    } as Parameters<typeof trackSummaryFromDetail>[0]);

    expect(summary.library).toEqual({ section: "album", albumTitle: "Northern Lights" });
    expect(summary.certificateValid).toBe(true);
  });

  it("presents an explicit action without rewriting a finalized older workflow", () => {
    const presentation = workflowUpgradePresentation(
      {
        status: "FINALIZED",
        workflowId: "suno-track",
        workflowVersion: "1.0",
        certificate: { valid: true, workflowVersion: "1.0" }
      },
      { id: "suno-track", version: "1.1" }
    );

    expect(presentation).toEqual({
      message: "Finalized with workflow suno-track 1.0 / Current workflow suno-track 1.1",
      action: "re-evaluate-track"
    });
    expect(workflowUpgradePresentation(
      {
        status: "FINALIZED",
        workflowId: "suno-track",
        workflowVersion: "1.1",
        certificate: { valid: true, workflowVersion: "1.1" }
      },
      { id: "suno-track", version: "1.1" }
    )).toBeNull();
  });
});
