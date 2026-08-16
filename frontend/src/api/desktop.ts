import { Channel, invoke } from "@tauri-apps/api/core";

import { createDemoApi } from "./demo";
import type {
  ActionResult,
  EvidencePreview,
  EvidenceMetadata,
  EvidenceRole,
  DocumentPreview,
  GlobalProfile,
  GlobalEvidenceItem,
  OperationProgress,
  ScanResult,
  StepId,
  StepStatus,
  SubscriptionBillingCycle,
  TrackCreateInput,
  TrackCoverPreview,
  TrackDetail,
  TrackLibraryAssignment,
  TrackSummary,
  ValidationResult,
  WorkflowDefinitionDto,
  WorkspaceSummary
} from "../domain/types";

export type OperationProgressHandler = (progress: OperationProgress) => void;

export interface DesktopApi {
  readonly mode: "tauri" | "demo";
  getWorkflow(): Promise<WorkflowDefinitionDto>;
  openWorkspace(): Promise<WorkspaceSummary | null>;
  createWorkspace(): Promise<WorkspaceSummary | null>;
  scanWorkspace(): Promise<ScanResult>;
  getProfile(): Promise<GlobalProfile>;
  updateProfile(profile: GlobalProfile): Promise<GlobalProfile>;
  listGlobalEvidence(): Promise<GlobalEvidenceItem[]>;
  importGlobalEvidence(role: EvidenceRole, coverageStart: string, billingCycle: SubscriptionBillingCycle): Promise<GlobalEvidenceItem | null>;
  importGlobalTermsEvidence(): Promise<GlobalEvidenceItem | null>;
  removeGlobalEvidence(evidenceId: string): Promise<void>;
  attachGlobalEvidence(trackId: string, evidenceId: string): Promise<TrackDetail>;
  listTracks(): Promise<TrackSummary[]>;
  listAlbums(): Promise<string[]>;
  createAlbum(title: string): Promise<string[]>;
  createTrack(input: TrackCreateInput): Promise<TrackDetail>;
  loadTrack(trackId: string): Promise<TrackDetail>;
  loadTrackCover(trackId: string): Promise<TrackCoverPreview | null>;
  updateTrackLibrary(trackId: string, library: TrackLibraryAssignment): Promise<TrackDetail>;
  renameAlbum(oldTitle: string, newTitle: string): Promise<TrackSummary[]>;
  updateTrack(trackId: string, patch: Partial<TrackDetail["fields"]>): Promise<TrackDetail>;
  adoptLegacyProfile(trackId: string): Promise<TrackDetail>;
  addDeviation(trackId: string, description: string, blocking: boolean): Promise<TrackDetail>;
  resolveDeviation(trackId: string, deviationId: string): Promise<TrackDetail>;
  removeDeviation(trackId: string, deviationId: string): Promise<TrackDetail>;
  setStepStatus(trackId: string, stepId: StepId, status: StepStatus, naReason?: string): Promise<TrackDetail>;
  importEvidence(trackId: string, role: EvidenceRole, replaceEvidenceId?: string, metadata?: Partial<EvidenceMetadata>): Promise<TrackDetail | null>;
  removeEvidence(trackId: string, evidenceId: string): Promise<TrackDetail>;
  previewEvidence(trackId: string, evidenceId: string): Promise<EvidencePreview>;
  verifyEvidence(trackId: string, evidenceId?: string): Promise<TrackDetail>;
  previewDocumentGeneration(trackId: string): Promise<DocumentPreview>;
  generateDocuments(trackId: string, adoptExisting?: boolean, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  generateArtworkDisclosure(trackId: string, disclosureText?: string): Promise<ActionResult>;
  calculateHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  verifyHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  validateTrack(trackId: string): Promise<ValidationResult>;
  finalizeTrack(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  invalidateCertificate(trackId: string): Promise<ActionResult>;
  createRevision(trackId: string): Promise<ActionResult>;
  reEvaluateTrack(trackId: string): Promise<ActionResult>;
}

interface TauriWindow extends Window {
  __TAURI_INTERNALS__?: unknown;
}

export function isTauriRuntime(target: Window = window): boolean {
  return Boolean((target as TauriWindow).__TAURI_INTERNALS__);
}

function isCancel(error: unknown): boolean {
  if (error === null || error === undefined) return true;
  const message = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  return /cancel|abgebrochen/i.test(message);
}

function progressChannel(onProgress?: OperationProgressHandler): Channel<OperationProgress> {
  return new Channel<OperationProgress>(onProgress ?? (() => undefined));
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    throw new DesktopCommandError(name, error);
  }
}

class TauriDesktopApi implements DesktopApi {
  readonly mode = "tauri" as const;

  getWorkflow(): Promise<WorkflowDefinitionDto> {
    return command("get_workflow");
  }

  async openWorkspace(): Promise<WorkspaceSummary | null> {
    try {
      return await command<WorkspaceSummary | null>("open_workspace");
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  async createWorkspace(): Promise<WorkspaceSummary | null> {
    try {
      return await command<WorkspaceSummary | null>("create_workspace");
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  scanWorkspace(): Promise<ScanResult> {
    return command("scan_workspace");
  }

  getProfile(): Promise<GlobalProfile> {
    return command("get_profile");
  }

  updateProfile(profile: GlobalProfile): Promise<GlobalProfile> {
    return command("update_profile", { profile });
  }

  listGlobalEvidence(): Promise<GlobalEvidenceItem[]> {
    return command("list_global_evidence");
  }

  async importGlobalEvidence(role: EvidenceRole, coverageStart: string, billingCycle: SubscriptionBillingCycle): Promise<GlobalEvidenceItem | null> {
    try {
      return await command<GlobalEvidenceItem | null>("import_global_evidence", { role, coverageStart, billingCycle });
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  async importGlobalTermsEvidence(): Promise<GlobalEvidenceItem | null> {
    try {
      return await command<GlobalEvidenceItem | null>("import_global_terms_evidence");
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  removeGlobalEvidence(evidenceId: string): Promise<void> {
    return command("remove_global_evidence", { evidenceId });
  }

  attachGlobalEvidence(trackId: string, evidenceId: string): Promise<TrackDetail> {
    return command("attach_global_evidence", { trackId, evidenceId });
  }

  addDeviation(trackId: string, description: string, blocking: boolean): Promise<TrackDetail> {
    return command("add_deviation", { trackId, input: { description, blocking } });
  }

  resolveDeviation(trackId: string, deviationId: string): Promise<TrackDetail> {
    return command("resolve_deviation", { trackId, deviationId });
  }

  removeDeviation(trackId: string, deviationId: string): Promise<TrackDetail> {
    return command("remove_deviation", { trackId, deviationId });
  }

  listTracks(): Promise<TrackSummary[]> {
    return command("list_tracks");
  }

  listAlbums(): Promise<string[]> {
    return command("list_albums");
  }

  createAlbum(title: string): Promise<string[]> {
    return command("create_album", { title });
  }

  createTrack(input: TrackCreateInput): Promise<TrackDetail> {
    return command("create_track", { input });
  }

  loadTrack(trackId: string): Promise<TrackDetail> {
    return command("load_track", { trackId });
  }

  loadTrackCover(trackId: string): Promise<TrackCoverPreview | null> {
    return command("load_track_cover", { trackId });
  }

  updateTrackLibrary(trackId: string, library: TrackLibraryAssignment): Promise<TrackDetail> {
    return command("update_track_library", { trackId, input: library });
  }

  renameAlbum(oldTitle: string, newTitle: string): Promise<TrackSummary[]> {
    return command("rename_album", { oldTitle, newTitle });
  }

  updateTrack(trackId: string, patch: Partial<TrackDetail["fields"]>): Promise<TrackDetail> {
    return command("update_track", { trackId, input: patch });
  }

  adoptLegacyProfile(trackId: string): Promise<TrackDetail> {
    return command("adopt_legacy_profile", { trackId });
  }

  setStepStatus(trackId: string, stepId: StepId, status: StepStatus, naReason?: string): Promise<TrackDetail> {
    return command("set_step_status", { trackId, stepId, status, naReason });
  }

  async importEvidence(trackId: string, role: EvidenceRole, replaceEvidenceId?: string, metadata?: Partial<EvidenceMetadata>): Promise<TrackDetail | null> {
    try {
      const args: Record<string, unknown> = { trackId, role, replaceEvidenceId };
      if (metadata) args.metadata = metadata;
      return await command<TrackDetail | null>("import_evidence", args);
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  removeEvidence(trackId: string, evidenceId: string): Promise<TrackDetail> {
    return command("remove_evidence", { trackId, evidenceId });
  }

  previewEvidence(trackId: string, evidenceId: string): Promise<EvidencePreview> {
    return command("preview_evidence", { trackId, evidenceId });
  }

  verifyEvidence(trackId: string, evidenceId?: string): Promise<TrackDetail> {
    return command("verify_evidence", { trackId, evidenceId });
  }

  previewDocumentGeneration(trackId: string): Promise<DocumentPreview> {
    return command("preview_documents", { trackId });
  }

  generateDocuments(trackId: string, adoptExisting = false, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("generate_documents", { trackId, adoptExisting, onProgress: progressChannel(onProgress) });
  }

  generateArtworkDisclosure(trackId: string, disclosureText?: string): Promise<ActionResult> {
    return command("generate_artwork_disclosure", { trackId, disclosureText });
  }

  calculateHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("calculate_hashes", { trackId, onProgress: progressChannel(onProgress) });
  }

  verifyHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("verify_hashes", { trackId, onProgress: progressChannel(onProgress) });
  }

  validateTrack(trackId: string): Promise<ValidationResult> {
    return command("validate_track", { trackId });
  }

  finalizeTrack(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("finalize_track", { trackId, onProgress: progressChannel(onProgress) });
  }

  invalidateCertificate(trackId: string): Promise<ActionResult> {
    return command("invalidate_certificate", { trackId });
  }

  createRevision(trackId: string): Promise<ActionResult> {
    return command("create_revision", { trackId });
  }

  reEvaluateTrack(trackId: string): Promise<ActionResult> {
    return command("re_evaluate_track", { trackId });
  }
}

export class DesktopCommandError extends Error {
  constructor(
    readonly commandName: string,
    readonly cause: unknown
  ) {
    super(toUserMessage(cause));
    this.name = "DesktopCommandError";
  }
}

export function toUserMessage(error: unknown): string {
  if (error instanceof DesktopCommandError) return error.message;
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    for (const key of ["message", "error", "detail", "reason"]) {
      if (typeof record[key] === "string" && record[key].trim()) return record[key];
    }
  }
  return "Die lokale Aktion konnte nicht abgeschlossen werden.";
}

export function createDesktopApi(target: Window = window): DesktopApi {
  return isTauriRuntime(target) ? new TauriDesktopApi() : createDemoApi();
}
