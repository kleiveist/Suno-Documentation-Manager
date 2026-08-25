import type { TrackLibraryStatusFilter } from "../domain/track-library";
import { emptyAudioScreeningSettings, emptyTimestampSettings } from "../domain/types";
import type {
  AudioScreeningProviderTestResult,
  AudioScreeningSettings,
  EvidenceMetadata,
  EvidencePreview,
  FolderImportProposal,
  GlobalEvidenceItem,
  GlobalProfile,
  OperationProgress,
  ScanResult,
  StepId,
  TimestampProviderTestResult,
  TimestampSettings,
  TrackDetail,
  TrackFields,
  TrackSummary,
  WorkflowDefinitionDto,
  WorkspaceSummary
} from "../domain/types";
import type { ColorTheme } from "../ui/theme";
import type { LongOperationKind } from "./progress";

export const WORKSPACE_PATH_STORAGE_KEY = "suno-doc-manager.last-workspace-path";

export type MainView = "dashboard" | "tracks" | "current" | "workspace" | "settings";

export type SettingsCategory = "global" | "external" | "files";

export type TrackTab = "overview" | "suno" | "artwork" | "release" | "evidence" | "certificate";

export type ToastKind = "success" | "error" | "info";

export interface ToastState {
  kind: ToastKind;
  title: string;
  message: string;
}

export interface ActiveOperationProgress {
  kind: LongOperationKind;
  progress: OperationProgress;
  elapsedSeconds: number;
}

export interface AppState {
  workspace: WorkspaceSummary | null;
  profile: GlobalProfile;
  timestampSettings: TimestampSettings;
  timestampProviderTest: TimestampProviderTestResult | null;
  audioScreeningSettings: AudioScreeningSettings;
  audioScreeningProviderTest: AudioScreeningProviderTestResult | null;
  tracks: TrackSummary[];
  albums: string[];
  track: TrackDetail | null;
  workflow: WorkflowDefinitionDto | null;
  globalEvidence: GlobalEvidenceItem[];
  view: MainView;
  settingsCategory: SettingsCategory;
  trackTab: TrackTab;
  activeStep: StepId | null;
  trackDraft: TrackFields | null;
  scanResult: ScanResult | null;
  query: string;
  trackFilter: TrackLibraryStatusFilter;
  busy: boolean;
  busyLabel: string;
  operationProgress: ActiveOperationProgress | null;
  sidebarOpen: boolean;
  showNewTrack: boolean;
  folderImport: FolderImportProposal | null;
  showTrackLibrary: boolean;
  showSubscriptionEvidence: boolean;
  evidencePreview: EvidencePreview | null;
  showCertificatePopup: boolean;
  termsMetadataDialog: { evidenceId: string | null; metadata: EvidenceMetadata } | null;
  theme: ColorTheme;
  toast: ToastState | null;
}

export function collectUserText(target: Set<string>, value: unknown, seen = new Set<object>()): void {
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed) target.add(trimmed);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item) => collectUserText(target, item, seen));
    return;
  }
  if (!value || typeof value !== "object" || seen.has(value)) return;
  seen.add(value);
  Object.values(value).forEach((item) => collectUserText(target, item, seen));
}

export type WorkspaceScopedUiState = Pick<
  AppState,
  | "track"
  | "trackDraft"
  | "activeStep"
  | "trackTab"
  | "scanResult"
  | "albums"
  | "showNewTrack"
  | "showTrackLibrary"
  | "showSubscriptionEvidence"
  | "evidencePreview"
  | "query"
  | "trackFilter"
  | "showCertificatePopup"
  | "folderImport"
  | "termsMetadataDialog"
  | "timestampSettings"
  | "timestampProviderTest"
  | "audioScreeningSettings"
  | "audioScreeningProviderTest"
> & { draftDirty: boolean };

export function resetWorkspaceScopedUiState(state: WorkspaceScopedUiState): WorkspaceScopedUiState {
  return {
    ...state,
    track: null,
    trackDraft: null,
    activeStep: null,
    trackTab: "overview",
    scanResult: null,
    albums: [],
    showNewTrack: false,
    folderImport: null,
    showTrackLibrary: false,
    showSubscriptionEvidence: false,
    evidencePreview: null,
    showCertificatePopup: false,
    termsMetadataDialog: null,
    timestampSettings: { ...emptyTimestampSettings, custom: { ...emptyTimestampSettings.custom } },
    timestampProviderTest: null,
    audioScreeningSettings: { ...emptyAudioScreeningSettings },
    audioScreeningProviderTest: null,
    query: "",
    trackFilter: "all",
    draftDirty: false
  };
}

export function shouldIgnoreModalBackdropClick(isBackdrop: boolean, isDirectClick: boolean): boolean {
  return isBackdrop && !isDirectClick;
}

export interface FinalizedTrackPresentation {
  title: string;
  message: string;
  actionLabel?: string;
  invalid: boolean;
}

export function isTrackContentLocked(status: TrackDetail["status"]): boolean {
  return status === "FINALIZED" || status === "SUPERSEDED";
}

export function canCreateTrackRevision(status: TrackDetail["status"]): boolean {
  return status === "FINALIZED";
}

export function finalizedTrackPresentation(
  track: Pick<TrackDetail, "status" | "certificate">
): FinalizedTrackPresentation | null {
  if (!isTrackContentLocked(track.status)) return null;
  if (track.status === "SUPERSEDED") {
    return {
      title: "Ersetzter Snapshot – nur lesbar",
      message:
        "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt. Navigation und reine Prüfungen bleiben verfügbar; der Snapshot selbst kann nicht erneut bearbeitet werden.",
      invalid: false
    };
  }
  return track.certificate.valid
    ? {
        title: "Finalisierter Snapshot – nur lesbar",
        message:
          "Lege eine neue Revision an, bevor du Angaben, Nachweise oder erzeugte Dokumente änderst. Die Navigation und reine Prüfungen bleiben verfügbar.",
        actionLabel: "Neue Revision anlegen und bearbeiten",
        invalid: false
      }
    : {
        title: "Finalisierter Snapshot mit ungültigem Zertifikat",
        message:
          "Der bisherige Snapshot bleibt erhalten. Lege eine neue Revision an, um Abweichungen zu bearbeiten und anschließend neu zu finalisieren.",
        actionLabel: "Neue Revision anlegen und bearbeiten",
        invalid: true
      };
}

export function shouldDiscardLockedDraft(status: TrackDetail["status"], draftDirty: boolean): boolean {
  return isTrackContentLocked(status) && draftDirty;
}
