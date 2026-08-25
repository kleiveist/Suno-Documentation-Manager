import { emptyAudioScreeningSettings } from "../../domain/types";
import type { AudioScreeningSettings, EvidenceItem, TrackDetail } from "../../domain/types";
import { clone } from "./helpers";

export function configuredAudioScreeningStatus(
  settings: AudioScreeningSettings,
  credentialsConfigured = false
): Pick<AudioScreeningSettings, "status" | "statusMessage"> {
  if (!settings.enabled) {
    return {
      status: "disabled",
      statusMessage: "Optional ACRCloud screening is disabled."
    };
  }
  if (!settings.host.trim()) {
    return {
      status: "configuration_invalid",
      statusMessage: "Enter the ACRCloud project host before using the optional provider check."
    };
  }
  if (!credentialsConfigured) {
    return {
      status: "not_configured",
      statusMessage:
        "Store both ACRCloud access values in local secure settings before using the optional provider check."
    };
  }
  return {
    status: "ready",
    statusMessage: "ACRCloud is configured. Use the explicit test or per-track check; no request runs automatically."
  };
}

export function normalizeAudioScreeningSettings(
  next: AudioScreeningSettings,
  credentialsConfigured = false
): AudioScreeningSettings {
  const timeoutCandidate = Number(next.timeoutSeconds);
  const timeoutSeconds =
    Number.isFinite(timeoutCandidate) && timeoutCandidate > 0
      ? Math.min(Math.trunc(timeoutCandidate), 120)
      : emptyAudioScreeningSettings.timeoutSeconds;
  const intensityCandidate = Number(next.intensityPercent);
  const intensityPercent = Number.isFinite(intensityCandidate)
    ? Math.min(Math.max(Math.trunc(intensityCandidate), 1), 100)
    : emptyAudioScreeningSettings.intensityPercent;
  const referenceCandidate = Number(next.referenceDurationSeconds);
  const referenceDurationSeconds = Number.isFinite(referenceCandidate)
    ? Math.min(Math.max(Math.trunc(referenceCandidate), 1), 3_600)
    : emptyAudioScreeningSettings.referenceDurationSeconds;
  const normalized: AudioScreeningSettings = {
    ...emptyAudioScreeningSettings,
    ...clone(next),
    host: next.host.trim(),
    timeoutSeconds,
    intensityPercent,
    dynamicByTrackDuration: Boolean(next.dynamicByTrackDuration),
    referenceDurationSeconds,
    credentialsConfigured,
    localEngineAvailable: false,
    localEngineVersion: undefined
  };
  return { ...normalized, ...configuredAudioScreeningStatus(normalized, credentialsConfigured) };
}

/**
 * The browser demo never contacts ACRCloud, but it records the same bounded
 * plan metadata that the desktop presentation will receive. This keeps the
 * UI honest: planned requests are not presented as executed samples.
 */
export function demoExternalScreeningPlan(settings: AudioScreeningSettings, sourceDurationMilliseconds?: number) {
  const sourceDurationSeconds = Math.max(0, (sourceDurationMilliseconds ?? 0) / 1_000);
  const calculationDurationSeconds =
    settings.dynamicByTrackDuration && sourceDurationSeconds > 0
      ? sourceDurationSeconds
      : settings.referenceDurationSeconds;
  const targetDurationMilliseconds = Math.ceil(
    Math.min(
      (calculationDurationSeconds * settings.intensityPercent) / 100,
      sourceDurationSeconds || calculationDurationSeconds
    ) * 1_000
  );
  const requestedRequestCount = targetDurationMilliseconds > 0 ? Math.ceil(targetDurationMilliseconds / 12_000) : 0;
  const maxNonOverlappingRequests =
    sourceDurationSeconds > 0 && sourceDurationSeconds < 12 ? 1 : Math.floor(sourceDurationSeconds / 12);
  const plannedRequestCount = Math.min(requestedRequestCount, maxNonOverlappingRequests, 25);
  return {
    screeningMode: plannedRequestCount > 1 ? ("multi_sample" as const) : ("single_sample" as const),
    requestedIntensityPercent: settings.intensityPercent,
    dynamicByTrackDuration: settings.dynamicByTrackDuration,
    referenceDurationSeconds: settings.referenceDurationSeconds,
    targetDurationMilliseconds,
    plannedRequestCount,
    executedRequestCount: 0,
    uniqueSampleCount: 0,
    overlappingSampleCount: 0,
    duplicateSampleCount: 0,
    uniqueSampleDurationMilliseconds: 0,
    trackCoveragePercent: 0,
    samples: []
  };
}

export function releaseAudio(track: TrackDetail): EvidenceItem | undefined {
  return track.evidence.find(
    (item) => item.role === "release_wav" && item.verified && Boolean(item.sha256) && !item.verificationError
  );
}

export function markAudioScreeningStale(track: TrackDetail): void {
  const source = releaseAudio(track);
  const local = track.audioScreening.local;
  if (local.status !== "not_run") {
    track.audioScreening.local = {
      ...local,
      status: "stale",
      message: source
        ? "The authoritative release audio changed; generate a new local fingerprint for the current file."
        : "The authoritative release audio is no longer available; the prior local fingerprint is stale."
    };
  }
  const external = track.audioScreening.external;
  if (external.status !== "not_run") {
    track.audioScreening.external = {
      ...external,
      status: "stale",
      message: "The authoritative release audio changed; the prior external result is no longer current.",
      responseRelativePath: undefined,
      responseSha256: undefined
    };
  }
}
