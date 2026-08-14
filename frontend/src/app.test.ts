import { describe, expect, it } from "vitest";

import {
  MAIN_NAVIGATION,
  missingProfileFields,
  resetWorkspaceScopedUiState,
  shouldIgnoreModalBackdropClick,
  trackSummaryFromDetail,
  type WorkspaceScopedUiState,
  workflowUpgradePresentation
} from "./app";
import { WORKFLOW_STEPS } from "./domain/workflow";
import { emptyProfile } from "./domain/types";
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
      showNewTrack: true,
      showTrackLibrary: true,
      showSubscriptionEvidence: true,
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
      showNewTrack: false,
      showTrackLibrary: false,
      showSubscriptionEvidence: false,
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
