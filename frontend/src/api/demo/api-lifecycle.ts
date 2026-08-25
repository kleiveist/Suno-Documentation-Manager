import type { FinalizeOptions, ValidationResult } from "../../domain/types";
import type { OperationProgressHandler } from "../desktop";
import { finalizationGate, WORKFLOW_ID, WORKFLOW_VERSION } from "../../domain/workflow";
import { emptyAudioScreeningSummary } from "../../domain/types";
import { clone, demoProgress, now, wait } from "./helpers";
import { refresh } from "./fixtures";
import { notRecordedExternalTimestampSummary } from "./timestamp";
import { migrateLegacySunoSemantics } from "./suno-semantics";
import { DemoScreeningApi } from "./api-screening";

export abstract class DemoLifecycleApi extends DemoScreeningApi {
  async validateTrack(trackId: string): Promise<ValidationResult> {
    await wait();
    const track = this.get(trackId);
    return finalizationGate(track, track.profileSnapshot);
  }

  async finalizeTrack(trackId: string, _options?: FinalizeOptions, onProgress?: OperationProgressHandler) {
    const track = this.mutableTrack(trackId);
    const gate = finalizationGate(track, track.profileSnapshot);
    if (!gate.valid)
      throw new Error(`Finalisierung blockiert: ${[...gate.missingItems, ...gate.blockingItems].join(", ")}`);
    const totalFiles = track.integrity.fileCount;
    const totalBytes = Math.max(totalFiles, 1) * 8476231;
    await demoProgress(onProgress, [
      { stage: "validating_finalization_gate", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
      { stage: "collecting_final_snapshot", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
      { stage: "writing_finalization_marker", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
      {
        stage: "generating_certificate",
        processedBytes: 0,
        totalBytes: 0,
        processedFiles: track.evidence.length,
        totalFiles: track.evidence.length
      },
      { stage: "verifying_certificate", processedBytes: 0, totalBytes: 0, processedFiles: 3, totalFiles: 3 },
      {
        stage: "verifying",
        processedBytes: Math.round(totalBytes * 0.45),
        totalBytes,
        processedFiles: Math.floor(totalFiles * 0.45),
        totalFiles,
        currentFile: "03_DOCUMENTATION/README.md"
      },
      {
        stage: "verifying",
        processedBytes: totalBytes,
        totalBytes,
        processedFiles: totalFiles,
        totalFiles,
        currentFile: "01_RELEASE/demo.wav"
      },
      {
        stage: "saving_final_snapshot",
        processedBytes: totalBytes,
        totalBytes,
        processedFiles: totalFiles,
        totalFiles
      },
      { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
    ]);
    track.status = "FINALIZED";
    const certificateId = `SDM-${new Date().getFullYear()}-${track.id.slice(0, 8).toUpperCase()}`;
    track.certificate = {
      valid: true,
      certificateId,
      finalizedAt: now(),
      workflowVersion: WORKFLOW_VERSION,
      certificateLanguage: this.state.profile.certificateLanguage,
      bilingual: true
    };
    track.finalizationAnchors = [
      {
        artifact: "evidence_manifest",
        label: "Evidence manifest (recommended timestamp anchor)",
        relativePath: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        sha256: "a".repeat(64)
      },
      {
        artifact: "sha256sums",
        label: "Track SHA-256 manifest",
        relativePath: "03_DOCUMENTATION/SHA256SUMS.txt",
        sha256: "b".repeat(64)
      },
      {
        artifact: "documentation_certificate_markdown",
        label: "Documentation certificate (Markdown)",
        relativePath: "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md",
        sha256: "c".repeat(64)
      },
      {
        artifact: "certificate_pdf",
        label: "Documentation certificate (English PDF)",
        relativePath: "SunoDM_DOCUMENTATION_CERTIFICATE.pdf",
        sha256: "d".repeat(64)
      },
      {
        artifact: "final_evidence_package",
        label: "Final evidence package certificate hash set",
        relativePath: "06_CERTIFICATE/CERTIFICATE_SHA256.txt",
        sha256: "e".repeat(64)
      }
    ];
    refresh(track);
    track.status = "FINALIZED";
    if (this.state.timestampSettings.enabled && this.state.timestampSettings.autoAfterFinalization) {
      this.attachConfiguredTimestamp(track);
    }
    return this.result(track, "Dokumentation finalisiert und Zertifikat erzeugt.");
  }

  async attachExternalTimestamp(trackId: string) {
    await wait();
    const track = this.get(trackId);
    return clone(this.attachConfiguredTimestamp(track));
  }

  async invalidateCertificate(trackId: string) {
    await wait();
    const track = this.get(trackId);
    if (track.status !== "FINALIZED") throw new Error("Der Track ist nicht finalisiert.");
    track.certificate.valid = false;
    track.certificate.invalidatedAt = now();
    track.certificate.invalidationReason = "Manuell invalidiert";
    return this.result(track, "Das Zertifikat wurde als ungültig markiert.");
  }

  async createRevision(trackId: string) {
    await wait();
    const track = this.get(trackId);
    if (track.status !== "FINALIZED") {
      throw new Error("Nur ein finalisierter Track kann eine neue Revision beginnen.");
    }
    migrateLegacySunoSemantics(track.fields);
    track.status = "ACTIVE";
    track.certificate = { valid: false };
    track.integrity.generated = false;
    track.integrity.verified = false;
    track.integrity.mismatchFiles = [];
    track.documents.current = false;
    track.externalTimestamps = [];
    track.externalTimestampSummary = notRecordedExternalTimestampSummary();
    track.audioScreening = clone(emptyAudioScreeningSummary);
    track.finalizationAnchors = [];
    refresh(track);
    return this.result(track, "Der bisherige Snapshot wurde archiviert und eine neue Revision angelegt.");
  }

  async reEvaluateTrack(trackId: string) {
    await wait();
    const track = this.get(trackId);
    if (track.workflowId === WORKFLOW_ID && track.workflowVersion === WORKFLOW_VERSION) {
      throw new Error("Der Track verwendet bereits die aktuelle Workflow-Version.");
    }
    if (track.status === "SUPERSEDED") {
      throw new Error("Der Track wurde durch eine neuere Revision ersetzt und kann nicht mehr geändert werden.");
    }
    const archived = track.status === "FINALIZED";
    migrateLegacySunoSemantics(track.fields);
    track.workflowId = WORKFLOW_ID;
    track.workflowVersion = WORKFLOW_VERSION;
    track.status = "ACTIVE";
    track.certificate = { valid: false };
    track.documents.current = false;
    track.integrity = {
      generated: false,
      verified: false,
      fileCount: 0,
      verifiedCount: 0,
      mismatchFiles: []
    };
    track.steps = [];
    track.externalTimestamps = [];
    track.externalTimestampSummary = notRecordedExternalTimestampSummary();
    track.audioScreening = clone(emptyAudioScreeningSummary);
    track.finalizationAnchors = [];
    refresh(track);
    return this.result(
      track,
      archived
        ? "Der bisherige Snapshot wurde archiviert; die Neubewertung verwendet den aktuellen Workflow."
        : "Die Neubewertung verwendet jetzt den aktuellen Workflow."
    );
  }
}
