import type { OperationProgressHandler } from "../desktop";
import { demoProgress, now } from "./helpers";
import { refresh } from "./fixtures";
import { configuredAudioScreeningStatus, demoExternalScreeningPlan } from "./audio-screening";
import { DemoDocumentsApi } from "./api-documents";

export abstract class DemoScreeningApi extends DemoDocumentsApi {
  async runLocalAudioScreening(trackId: string, onProgress: OperationProgressHandler) {
    const track = this.mutableTrack(trackId);
    const source = this.releaseAudio(track);
    if (!source?.sha256) throw new Error("Importiere zuerst die autoritative finale Release-Audiodatei.");
    await demoProgress(onProgress, [
      {
        stage: "preparing_audio",
        processedBytes: 0,
        totalBytes: source.sizeBytes,
        processedFiles: 0,
        totalFiles: 1,
        currentFile: source.relativePath
      },
      {
        stage: "fingerprinting_audio",
        processedBytes: Math.round(source.sizeBytes * 0.65),
        totalBytes: source.sizeBytes,
        processedFiles: 0,
        totalFiles: 1,
        currentFile: source.relativePath
      },
      {
        stage: "fingerprint_complete",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1,
        currentFile: source.relativePath
      },
      {
        stage: "saving_screening_result",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1,
        currentFile: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json"
      },
      {
        stage: "complete",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1
      }
    ]);
    track.audioScreening.local = {
      status: "fingerprint_generated",
      message:
        "Browser demo presentation only: no local audio file was analysed. Run this check in the desktop app for an authoritative result.",
      engine: "Chromaprint",
      engineVersion: "demo",
      sourceEvidenceId: source.id,
      sourceRelativePath: source.relativePath,
      sourceSha256: source.sha256,
      sourceSizeBytes: source.sizeBytes,
      durationMilliseconds: source.metadata?.audioDurationMilliseconds ?? undefined,
      fingerprintAlgorithm: "2",
      generatedAt: now(),
      artifactRelativePath: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json",
      artifactSha256: "d".repeat(64)
    };
    track.documents.current = false;
    track.integrity.generated = false;
    track.integrity.verified = false;
    refresh(track);
    return this.result(
      track,
      "Browser-Demo: Der lokale Screening-Status wurde nur zur Oberflächenvorschau simuliert. Die Desktop-App erzeugt den echten Chromaprint-Fingerprint."
    );
  }

  async runExternalAudioScreening(trackId: string, onProgress: OperationProgressHandler) {
    const track = this.mutableTrack(trackId);
    const source = this.releaseAudio(track);
    if (!source?.sha256) throw new Error("Importiere zuerst die autoritative finale Release-Audiodatei.");
    const configured = configuredAudioScreeningStatus(
      this.state.audioScreeningSettings,
      this.state.audioAccessKeyConfigured && this.state.audioAccessSecretConfigured
    );
    const plan = demoExternalScreeningPlan(
      this.state.audioScreeningSettings,
      source.metadata?.audioDurationMilliseconds ?? undefined
    );
    if (configured.status !== "ready") {
      const externalStatus =
        configured.status === "configuration_invalid"
          ? "configuration_invalid"
          : configured.status === "authentication_failed"
            ? "authentication_failed"
            : configured.status === "provider_unavailable"
              ? "provider_unavailable"
              : "skipped_not_configured";
      track.audioScreening.external = {
        provider: "ACRCloud",
        status: externalStatus,
        message: configured.statusMessage,
        sourceEvidenceId: source.id,
        sourceRelativePath: source.relativePath,
        sourceSha256: source.sha256,
        sourceSizeBytes: source.sizeBytes,
        sourceDurationMilliseconds: source.metadata?.audioDurationMilliseconds ?? undefined,
        ...plan,
        providerStatus: configured.status,
        matches: []
      };
      track.documents.current = false;
      track.integrity.generated = false;
      track.integrity.verified = false;
      refresh(track);
      return this.result(
        track,
        "Die optionale externe Prüfung wurde übersprungen; die ACRCloud-Konfiguration ist nicht vollständig."
      );
    }
    await demoProgress(onProgress, [
      {
        stage: "preparing_external_check",
        processedBytes: 0,
        totalBytes: source.sizeBytes,
        processedFiles: 0,
        totalFiles: 1,
        currentFile: source.relativePath
      },
      {
        stage: "sending_provider_request",
        processedBytes: Math.round(source.sizeBytes * 0.3),
        totalBytes: source.sizeBytes,
        processedFiles: 0,
        totalFiles: 1,
        currentFile: source.relativePath
      },
      {
        stage: "waiting_provider_response",
        processedBytes: Math.round(source.sizeBytes * 0.7),
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1
      },
      {
        stage: "processing_provider_response",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1
      },
      {
        stage: "saving_screening_result",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1
      },
      {
        stage: "complete",
        processedBytes: source.sizeBytes,
        totalBytes: source.sizeBytes,
        processedFiles: 1,
        totalFiles: 1
      }
    ]);
    track.audioScreening.external = {
      provider: "ACRCloud",
      status: "provider_unavailable",
      message:
        "Browser demo: no ACRCloud request was sent. The desktop app is required for an authoritative provider response.",
      sourceEvidenceId: source.id,
      sourceRelativePath: source.relativePath,
      sourceSha256: source.sha256,
      sourceSizeBytes: source.sizeBytes,
      sourceDurationMilliseconds: source.metadata?.audioDurationMilliseconds ?? undefined,
      checkedAt: now(),
      ...plan,
      providerStatus: "provider_unavailable",
      matches: []
    };
    track.documents.current = false;
    track.integrity.generated = false;
    track.integrity.verified = false;
    refresh(track);
    return this.result(
      track,
      "Browser-Demo: Es wurde keine ACRCloud-Anfrage ausgeführt und kein Providerergebnis erzeugt."
    );
  }
}
