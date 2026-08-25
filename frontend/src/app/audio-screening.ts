import type {
  AudioScreeningExternalSummary,
  AudioScreeningLocalSummary,
  AudioScreeningProviderStatus,
  AudioScreeningSettings,
  AudioScreeningStatus,
  EvidenceItem
} from "../domain/types";
import { translateUiText } from "../ui/i18n";
import type { AppLanguage } from "../ui/i18n";

export function audioScreeningStatusLabel(value: AudioScreeningStatus): string {
  return (
    {
      not_run: "NOT RUN",
      fingerprint_generated: "FINGERPRINT GENERATED",
      no_match_detected: "NO MATCH DETECTED",
      match_detected: "MATCH DETECTED",
      skipped_not_configured: "SKIPPED – NOT CONFIGURED",
      provider_unavailable: "PROVIDER UNAVAILABLE",
      authentication_failed: "AUTHENTICATION FAILED",
      configuration_invalid: "CONFIGURATION INVALID",
      engine_unavailable: "ENGINE UNAVAILABLE",
      unsupported_format: "UNSUPPORTED FORMAT",
      processing_failed: "PROCESSING FAILED",
      stale: "STALE"
    } as const
  )[value];
}

export function audioScreeningProviderStatusLabel(value: AudioScreeningProviderStatus): string {
  return (
    {
      disabled: "DISABLED",
      not_configured: "NOT CONFIGURED",
      ready: "READY",
      authentication_failed: "AUTHENTICATION FAILED",
      provider_unavailable: "PROVIDER UNAVAILABLE",
      configuration_invalid: "CONFIGURATION INVALID"
    } as const
  )[value];
}

export function audioScreeningProviderIsReady(settings: AudioScreeningSettings): boolean {
  return settings.enabled && settings.credentialsConfigured && settings.status === "ready";
}

export const ACRCLOUD_SAMPLE_MAX_SECONDS = 12;

export const ACRCLOUD_MAX_REQUESTS = 25;

export const ACRCLOUD_MAX_UNIQUE_SECONDS = ACRCLOUD_SAMPLE_MAX_SECONDS * ACRCLOUD_MAX_REQUESTS;

export interface AudioScreeningIntensityEstimate {
  /** The duration used for this preview. A dynamic setting uses the actual track when known. */
  calculationDurationSeconds: number;
  targetDurationSeconds: number;
  requestedRequestCount: number;
  actualRequestCount: number;
  maxUniqueDurationSeconds: number;
  capped: boolean;
}

export function audioScreeningIntensityEstimate(
  settings: Pick<AudioScreeningSettings, "intensityPercent" | "dynamicByTrackDuration" | "referenceDurationSeconds">,
  actualTrackDurationSeconds?: number
): AudioScreeningIntensityEstimate {
  const intensityPercent = Number.isFinite(settings.intensityPercent)
    ? Math.min(Math.max(Math.trunc(settings.intensityPercent), 1), 100)
    : 5;
  const referenceDurationSeconds = Number.isFinite(settings.referenceDurationSeconds)
    ? Math.min(Math.max(Math.trunc(settings.referenceDurationSeconds), 1), 3_600)
    : 300;
  const knownTrackDuration =
    Number.isFinite(actualTrackDurationSeconds) && actualTrackDurationSeconds! > 0
      ? actualTrackDurationSeconds!
      : undefined;
  const calculationDurationSeconds =
    settings.dynamicByTrackDuration && knownTrackDuration ? knownTrackDuration : referenceDurationSeconds;
  const availableDurationSeconds = knownTrackDuration ?? calculationDurationSeconds;
  // In fixed-reference mode the reference only determines the requested
  // coverage. A known shorter release still cannot be sampled beyond its end.
  const targetDurationSeconds = Math.min(
    (calculationDurationSeconds * intensityPercent) / 100,
    availableDurationSeconds
  );
  const requestedRequestCount =
    targetDurationSeconds > 0 ? Math.ceil(targetDurationSeconds / ACRCLOUD_SAMPLE_MAX_SECONDS) : 0;
  // A short track can still use one bounded sample; otherwise every additional
  // sample needs a fully non-overlapping 12-second interval.
  const maxNonOverlappingRequests =
    availableDurationSeconds > 0 && availableDurationSeconds < ACRCLOUD_SAMPLE_MAX_SECONDS
      ? 1
      : Math.floor(availableDurationSeconds / ACRCLOUD_SAMPLE_MAX_SECONDS);
  const actualRequestCount = Math.min(requestedRequestCount, maxNonOverlappingRequests, ACRCLOUD_MAX_REQUESTS);
  // When the requested count fits, the final sample is shortened so that the
  // planned unique duration remains proportional to the requested target. If
  // a non-overlap/cap boundary removes requests, all remaining slots are full
  // twelve-second samples and the reduced maximum is reported honestly.
  const maxUniqueDurationSeconds =
    actualRequestCount === requestedRequestCount
      ? targetDurationSeconds
      : Math.min(
          availableDurationSeconds,
          actualRequestCount * ACRCLOUD_SAMPLE_MAX_SECONDS,
          ACRCLOUD_MAX_UNIQUE_SECONDS
        );
  return {
    calculationDurationSeconds,
    targetDurationSeconds,
    requestedRequestCount,
    actualRequestCount,
    maxUniqueDurationSeconds,
    capped: actualRequestCount < requestedRequestCount
  };
}

export function audioScreeningIntensityBand(
  requestCount: number
): "low" | "normal" | "elevated" | "high" | "very_high" {
  if (requestCount <= 5) return "low";
  if (requestCount <= 10) return "normal";
  if (requestCount <= 15) return "elevated";
  if (requestCount <= 20) return "high";
  return "very_high";
}

export function formatScreeningSeconds(seconds: number, language: AppLanguage = "de"): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return language === "de" ? "0 Sekunden" : "0 seconds";
  const rounded = Math.round(seconds);
  return language === "de" ? `${rounded} Sekunden` : `${rounded} seconds`;
}

export function localAudioScreeningIsCurrent(
  local: Pick<
    AudioScreeningLocalSummary,
    "status" | "sourceEvidenceId" | "sourceRelativePath" | "sourceSha256" | "sourceSizeBytes"
  >,
  evidence: readonly Pick<
    EvidenceItem,
    "id" | "role" | "relativePath" | "sha256" | "sizeBytes" | "verified" | "verificationError"
  >[]
): boolean {
  const release = evidence.find(
    (item) => item.role === "release_wav" && item.verified && Boolean(item.sha256) && !item.verificationError
  );
  return (
    local.status === "fingerprint_generated" &&
    Boolean(release) &&
    local.sourceEvidenceId === release!.id &&
    local.sourceRelativePath === release!.relativePath &&
    local.sourceSha256 === release!.sha256 &&
    local.sourceSizeBytes === release!.sizeBytes
  );
}

export function visibleLocalAudioScreening(
  local: AudioScreeningLocalSummary,
  evidence: readonly EvidenceItem[]
): AudioScreeningLocalSummary {
  if (local.status === "fingerprint_generated" && !localAudioScreeningIsCurrent(local, evidence)) {
    return {
      ...local,
      status: "stale",
      message:
        "Der lokale Fingerprint ist nicht mehr an die aktuelle finale Release-Datei gebunden. Führe die lokale Prüfung erneut aus."
    };
  }
  return local;
}

export function externalAudioScreeningIsCurrent(
  external: Pick<
    AudioScreeningExternalSummary,
    "status" | "sourceEvidenceId" | "sourceRelativePath" | "sourceSha256" | "sourceSizeBytes"
  >,
  evidence: readonly Pick<
    EvidenceItem,
    "id" | "role" | "relativePath" | "sha256" | "sizeBytes" | "verified" | "verificationError"
  >[]
): boolean {
  const hasSourceBinding = Boolean(
    external.sourceEvidenceId || external.sourceRelativePath || external.sourceSha256 || external.sourceSizeBytes
  );
  if (!hasSourceBinding) return true;
  const release = evidence.find(
    (item) => item.role === "release_wav" && item.verified && Boolean(item.sha256) && !item.verificationError
  );
  return (
    Boolean(release) &&
    external.sourceEvidenceId === release!.id &&
    external.sourceRelativePath === release!.relativePath &&
    external.sourceSha256 === release!.sha256 &&
    external.sourceSizeBytes === release!.sizeBytes
  );
}

export function visibleExternalAudioScreening(
  external: Pick<
    AudioScreeningExternalSummary,
    "status" | "message" | "sourceEvidenceId" | "sourceRelativePath" | "sourceSha256" | "sourceSizeBytes"
  >,
  settings: Pick<AudioScreeningSettings, "status">,
  evidence: readonly Pick<
    EvidenceItem,
    "id" | "role" | "relativePath" | "sha256" | "sizeBytes" | "verified" | "verificationError"
  >[] = []
): Pick<AudioScreeningExternalSummary, "status" | "message"> {
  if (
    external.status !== "not_run" &&
    external.status !== "skipped_not_configured" &&
    external.status !== "stale" &&
    !externalAudioScreeningIsCurrent(external, evidence)
  ) {
    return {
      status: "stale",
      message:
        "Das externe Katalogergebnis ist nicht mehr an die aktuelle finale Release-Datei gebunden. Starte die Prüfung bei Bedarf erneut."
    };
  }
  if (external.status === "not_run" && (settings.status === "disabled" || settings.status === "not_configured")) {
    return {
      status: "skipped_not_configured",
      message:
        settings.status === "disabled"
          ? "ACRCloud ist nicht aktiviert; es wurde keine externe Katalogprüfung gestartet."
          : "ACRCloud-Zugangsdaten fehlen; es wurde keine externe Katalogprüfung gestartet."
    };
  }
  return external;
}

export function audioScreeningStatusClass(status: AudioScreeningStatus): "is-valid" | "is-warning" | "" {
  if (status === "fingerprint_generated" || status === "no_match_detected") return "is-valid";
  if (
    [
      "match_detected",
      "stale",
      "provider_unavailable",
      "authentication_failed",
      "configuration_invalid",
      "engine_unavailable",
      "unsupported_format",
      "processing_failed"
    ].includes(status)
  )
    return "is-warning";
  return "";
}

export function formatAudioDuration(milliseconds: number | undefined, language: AppLanguage = "de"): string {
  if (!Number.isFinite(milliseconds) || !milliseconds || milliseconds < 0)
    return translateUiText("Nicht dokumentiert", language);
  const totalSeconds = Math.round(milliseconds / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}

export function formatAudioTimestamp(milliseconds: number | undefined, language: AppLanguage = "de"): string {
  return formatAudioDuration(milliseconds, language);
}
