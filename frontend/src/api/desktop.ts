import { Channel, invoke } from "@tauri-apps/api/core";

import { createDemoApi } from "./demo";
import { hasUiTranslation, translateUiText, type AppLanguage } from "../ui/i18n";
import type {
  ActionResult,
  AudioScreeningProviderTestResult,
  AudioScreeningSecretInput,
  AudioScreeningSettings,
  EvidencePreview,
  FolderImportExecutionInput,
  FolderImportProposal,
  EvidenceMetadata,
  EvidenceRole,
  DocumentPreview,
  FinalizeOptions,
  GlobalProfile,
  GlobalEvidenceItem,
  OperationProgress,
  ScanResult,
  StepId,
  StepStatus,
  SubscriptionBillingCycle,
  TimestampProviderTestResult,
  TimestampSettings,
  TrackCreateInput,
  TrackCoverPreview,
  TrackDetail,
  TrackFieldPatch,
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
  openWorkspace(language?: AppLanguage): Promise<WorkspaceSummary | null>;
  createWorkspace(language?: AppLanguage): Promise<WorkspaceSummary | null>;
  restoreWorkspace(path: string): Promise<WorkspaceSummary>;
  scanWorkspace(): Promise<ScanResult>;
  getProfile(): Promise<GlobalProfile>;
  updateProfile(profile: GlobalProfile): Promise<GlobalProfile>;
  getTimestampSettings(): Promise<TimestampSettings>;
  updateTimestampSettings(settings: TimestampSettings): Promise<TimestampSettings>;
  /** Writes a secret to native secure storage; it is intentionally never returned. */
  updateTimestampSecret(secret: string | null): Promise<void>;
  testTimestampProvider(): Promise<TimestampProviderTestResult>;
  getAudioScreeningSettings(): Promise<AudioScreeningSettings>;
  updateAudioScreeningSettings(settings: AudioScreeningSettings): Promise<AudioScreeningSettings>;
  /** Writes access credentials to native secure storage; they are intentionally never returned. */
  updateAudioScreeningSecret(input: AudioScreeningSecretInput): Promise<void>;
  testAudioScreeningProvider(): Promise<AudioScreeningProviderTestResult>;
  listGlobalEvidence(): Promise<GlobalEvidenceItem[]>;
  importGlobalEvidence(role: EvidenceRole, coverageStart: string, billingCycle: SubscriptionBillingCycle, language?: AppLanguage): Promise<GlobalEvidenceItem | null>;
  importGlobalTermsEvidence(metadata: Partial<EvidenceMetadata>, language?: AppLanguage): Promise<GlobalEvidenceItem | null>;
  updateGlobalTermsEvidenceMetadata(evidenceId: string, metadata: Partial<EvidenceMetadata>): Promise<GlobalEvidenceItem>;
  removeGlobalEvidence(evidenceId: string): Promise<void>;
  attachGlobalEvidence(trackId: string, evidenceId: string): Promise<TrackDetail>;
  listTracks(): Promise<TrackSummary[]>;
  listAlbums(): Promise<string[]>;
  createAlbum(title: string): Promise<string[]>;
  createTrack(input: TrackCreateInput): Promise<TrackDetail>;
  scanImportFolder(language?: AppLanguage): Promise<FolderImportProposal | null>;
  executeFolderImport(input: FolderImportExecutionInput): Promise<TrackDetail[]>;
  loadTrack(trackId: string): Promise<TrackDetail>;
  loadTrackCover(trackId: string): Promise<TrackCoverPreview | null>;
  updateTrackLibrary(trackId: string, library: TrackLibraryAssignment): Promise<TrackDetail>;
  renameAlbum(oldTitle: string, newTitle: string): Promise<TrackSummary[]>;
  updateTrack(trackId: string, patch: TrackFieldPatch): Promise<TrackDetail>;
  adoptLegacyProfile(trackId: string): Promise<TrackDetail>;
  addDeviation(trackId: string, description: string, blocking: boolean): Promise<TrackDetail>;
  resolveDeviation(trackId: string, deviationId: string): Promise<TrackDetail>;
  removeDeviation(trackId: string, deviationId: string): Promise<TrackDetail>;
  setStepStatus(trackId: string, stepId: StepId, status: StepStatus, naReason?: string): Promise<TrackDetail>;
  importEvidence(trackId: string, role: EvidenceRole, replaceEvidenceId?: string, metadata?: Partial<EvidenceMetadata>, language?: AppLanguage): Promise<TrackDetail | null>;
  removeEvidence(trackId: string, evidenceId: string): Promise<TrackDetail>;
  previewEvidence(trackId: string, evidenceId: string): Promise<EvidencePreview>;
  verifyEvidence(trackId: string, evidenceId?: string): Promise<TrackDetail>;
  previewDocumentGeneration(trackId: string): Promise<DocumentPreview>;
  generateDocuments(trackId: string, adoptExisting?: boolean, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  generateArtworkDisclosure(trackId: string, disclosureText?: string): Promise<ActionResult>;
  calculateHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  verifyHashes(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  runLocalAudioScreening(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  runExternalAudioScreening(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  validateTrack(trackId: string): Promise<ValidationResult>;
  finalizeTrack(trackId: string, options?: FinalizeOptions, onProgress?: OperationProgressHandler): Promise<ActionResult>;
  attachExternalTimestamp(trackId: string): Promise<TrackDetail>;
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

  async openWorkspace(language?: AppLanguage): Promise<WorkspaceSummary | null> {
    try {
      return await command<WorkspaceSummary | null>("open_workspace", language ? { language } : undefined);
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  async createWorkspace(language?: AppLanguage): Promise<WorkspaceSummary | null> {
    try {
      return await command<WorkspaceSummary | null>("create_workspace", language ? { language } : undefined);
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  restoreWorkspace(path: string): Promise<WorkspaceSummary> {
    return command<WorkspaceSummary>("open_workspace_by_path", { path });
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

  getTimestampSettings(): Promise<TimestampSettings> {
    return command("get_timestamp_settings");
  }

  updateTimestampSettings(settings: TimestampSettings): Promise<TimestampSettings> {
    return command("update_timestamp_settings", { settings });
  }

  updateTimestampSecret(secret: string | null): Promise<void> {
    return command("update_timestamp_secret", { input: { secret } });
  }

  testTimestampProvider(): Promise<TimestampProviderTestResult> {
    return command("test_timestamp_provider");
  }

  getAudioScreeningSettings(): Promise<AudioScreeningSettings> {
    return command("get_audio_screening_settings");
  }

  updateAudioScreeningSettings(settings: AudioScreeningSettings): Promise<AudioScreeningSettings> {
    return command("update_audio_screening_settings", { settings });
  }

  updateAudioScreeningSecret(input: AudioScreeningSecretInput): Promise<void> {
    return command("update_audio_screening_secret", { input });
  }

  testAudioScreeningProvider(): Promise<AudioScreeningProviderTestResult> {
    return command("test_audio_screening_provider");
  }

  listGlobalEvidence(): Promise<GlobalEvidenceItem[]> {
    return command("list_global_evidence");
  }

  async importGlobalEvidence(role: EvidenceRole, coverageStart: string, billingCycle: SubscriptionBillingCycle, language?: AppLanguage): Promise<GlobalEvidenceItem | null> {
    try {
      return await command<GlobalEvidenceItem | null>("import_global_evidence", {
        role,
        coverageStart,
        billingCycle,
        ...(language ? { language } : {})
      });
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  async importGlobalTermsEvidence(metadata: Partial<EvidenceMetadata>, language?: AppLanguage): Promise<GlobalEvidenceItem | null> {
    try {
      return await command<GlobalEvidenceItem | null>("import_global_terms_evidence", {
        metadata,
        ...(language ? { language } : {})
      });
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  updateGlobalTermsEvidenceMetadata(evidenceId: string, metadata: Partial<EvidenceMetadata>): Promise<GlobalEvidenceItem> {
    return command("update_global_terms_evidence_metadata", { evidenceId, metadata });
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

  async scanImportFolder(language?: AppLanguage): Promise<FolderImportProposal | null> {
    try {
      return await command<FolderImportProposal | null>("scan_import_folder", language ? { language } : undefined);
    } catch (error) {
      if (error instanceof DesktopCommandError && isCancel(error.cause)) return null;
      throw error;
    }
  }

  executeFolderImport(input: FolderImportExecutionInput): Promise<TrackDetail[]> {
    return command("execute_folder_import", { input });
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

  updateTrack(trackId: string, patch: TrackFieldPatch): Promise<TrackDetail> {
    return command("update_track", { trackId, input: patch });
  }

  adoptLegacyProfile(trackId: string): Promise<TrackDetail> {
    return command("adopt_legacy_profile", { trackId });
  }

  setStepStatus(trackId: string, stepId: StepId, status: StepStatus, naReason?: string): Promise<TrackDetail> {
    return command("set_step_status", { trackId, stepId, status, naReason });
  }

  async importEvidence(trackId: string, role: EvidenceRole, replaceEvidenceId?: string, metadata?: Partial<EvidenceMetadata>, language?: AppLanguage): Promise<TrackDetail | null> {
    try {
      const args: Record<string, unknown> = { trackId, role, replaceEvidenceId };
      if (metadata) args.metadata = metadata;
      if (language) args.language = language;
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

  runLocalAudioScreening(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("run_local_audio_screening", { trackId, onProgress: progressChannel(onProgress) });
  }

  runExternalAudioScreening(trackId: string, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("run_external_audio_screening", { trackId, onProgress: progressChannel(onProgress) });
  }

  validateTrack(trackId: string): Promise<ValidationResult> {
    return command("validate_track", { trackId });
  }

  finalizeTrack(trackId: string, options?: FinalizeOptions, onProgress?: OperationProgressHandler): Promise<ActionResult> {
    return command("finalize_track", {
      trackId,
      ...(options ? { options } : {}),
      onProgress: progressChannel(onProgress)
    });
  }

  attachExternalTimestamp(trackId: string): Promise<TrackDetail> {
    return command("attach_configured_external_timestamp", { trackId });
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

export function toUserMessage(error: unknown, language?: AppLanguage): string {
  let message = "";
  if (error instanceof DesktopCommandError) message = error.message;
  else if (error instanceof Error && error.message.trim()) message = error.message;
  else if (typeof error === "string" && error.trim()) message = error;
  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    for (const key of ["message", "error", "detail", "reason"]) {
      if (!message && typeof record[key] === "string" && record[key].trim()) message = record[key];
    }
  }
  if (!message) {
    return language === "en"
      ? "The local action could not be completed."
      : "Die lokale Aktion konnte nicht abgeschlossen werden.";
  }
  if (!language) return message;
  const translated = translateUiText(message, language);
  if (hasUiTranslation(message, language)) return translated;
  // Native code still has a few low-level diagnostic messages that are not
  // user copy. Do not surface them in the opposite UI language.
  return language === "en"
    ? "The local action could not be completed."
    : "Die lokale Aktion konnte nicht abgeschlossen werden.";
}

export function createDesktopApi(target: Window = window): DesktopApi {
  return isTauriRuntime(target) ? new TauriDesktopApi() : createDemoApi();
}
