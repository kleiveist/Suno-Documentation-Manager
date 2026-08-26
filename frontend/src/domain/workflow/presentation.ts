import type {
  AutomaticConsistencyFinding,
  AutomaticConsistencyFindingLevel,
  AutomaticConsistencyPresentation,
  ExternalTimestampStatus,
  TrackDetail
} from "../types";
import { humanEditedFinalArtworkStatus } from "./evidence";

const TIMESTAMP_WARNING_STATUSES: ReadonlySet<ExternalTimestampStatus> = new Set([
  "verification_failed",
  "provider_unavailable",
  "authentication_failed",
  "anchor_mismatch",
  "configuration_incomplete",
  "authentication_required",
  "connection_failed",
  "unsupported_response",
  "verification_configuration_incomplete"
]);

const automaticFindingOrder: Record<AutomaticConsistencyFindingLevel, number> = {
  BLOCKING: 0,
  WARNING: 1,
  INFO: 2
};

function initialFindings(track: TrackDetail): AutomaticConsistencyFinding[] {
  return track.automation.consistencyIssues.map((issue) => ({
    code: `consistency:${issue.code}`,
    level: issue.blocking ? "BLOCKING" : "WARNING",
    message: issue.message,
    stepId: issue.stepId
  }));
}

function appendDeviationFindings(findings: AutomaticConsistencyFinding[], track: TrackDetail): void {
  for (const deviation of track.blockingDeviations ?? []) {
    if (deviation.resolved) continue;
    findings.push({
      code: `deviation:${deviation.id}`,
      level: deviation.blocking ? "BLOCKING" : "WARNING",
      message: deviation.description,
      stepId: "finalize",
      userProvided: true
    });
  }
}

function finalizedTimestampFinding(track: TrackDetail): AutomaticConsistencyFinding | null {
  if (track.status !== "FINALIZED" && track.status !== "SUPERSEDED") return null;
  const latestTimestamp = track.externalTimestamps.at(-1);
  const timestampStatus =
    track.externalTimestampSummary?.status ??
    latestTimestamp?.providerMetadata?.verificationResult ??
    (latestTimestamp ? "attached" : "not_recorded");
  const timestampMessage = track.externalTimestampSummary?.message.trim();

  if (timestampStatus === "attached") {
    return {
      code: "external_timestamp_attached_unverified",
      level: "WARNING",
      message:
        timestampMessage || "Ein externer Zeitstempelnachweis ist angehängt, seine Verifikation steht jedoch noch aus.",
      stepId: "finalize"
    };
  }
  if (TIMESTAMP_WARNING_STATUSES.has(timestampStatus)) {
    return {
      code: `external_timestamp_${timestampStatus}`,
      level: "WARNING",
      message:
        timestampMessage ||
        `Der externe Zeitstempelnachweis meldet den Status ${timestampStatus.toUpperCase().replaceAll("_", " ")}.`,
      stepId: "finalize"
    };
  }
  if (timestampStatus === "not_recorded") {
    return {
      code: "external_timestamp_not_recorded",
      level: "INFO",
      message: timestampMessage || "Für den finalisierten Snapshot ist kein externer Zeitstempelnachweis erfasst.",
      stepId: "finalize"
    };
  }
  return null;
}

function appendInformationalFindings(findings: AutomaticConsistencyFinding[], track: TrackDetail): void {
  if (!track.automation.sunoMetadataDetected) {
    findings.push({
      code: "suno_metadata_not_detected",
      level: "INFO",
      message: "Im Suno-Final-Export wurden keine Suno-Studio-Metadaten erkannt.",
      stepId: "suno"
    });
  }
  if (humanEditedFinalArtworkStatus(track.evidence) === "BYTE-IDENTICAL / SHA-256 MATCH") {
    findings.push({
      code: "human_edited_final_artwork_sha256_match",
      level: "INFO",
      message: "Menschlich bearbeitetes und finales Artwork sind byte-identisch (SHA-256-Match).",
      stepId: "artwork"
    });
  }
}

function presentationFrom(findings: AutomaticConsistencyFinding[]): AutomaticConsistencyPresentation {
  findings.sort(
    (left, right) =>
      automaticFindingOrder[left.level] - automaticFindingOrder[right.level] || left.code.localeCompare(right.code)
  );
  const infoCount = findings.filter((finding) => finding.level === "INFO").length;
  const warningCount = findings.filter((finding) => finding.level === "WARNING").length;
  const blockingCount = findings.filter((finding) => finding.level === "BLOCKING").length;
  return {
    outcome: blockingCount > 0 ? "BLOCKED" : warningCount > 0 ? "PASS WITH WARNINGS" : "PASS",
    findings,
    infoCount,
    warningCount,
    blockingCount
  };
}

/**
 * Derives user-facing INFO/WARNING/BLOCKING findings without introducing a
 * second workflow gate. StepStatus, requirement evaluation and finalization
 * continue to use only their existing authoritative inputs.
 */
export function automaticConsistencyPresentation(track: TrackDetail): AutomaticConsistencyPresentation {
  const findings = initialFindings(track);
  appendDeviationFindings(findings, track);
  const timestampFinding = finalizedTimestampFinding(track);
  if (timestampFinding) findings.push(timestampFinding);
  appendInformationalFindings(findings, track);
  return presentationFrom(findings);
}
