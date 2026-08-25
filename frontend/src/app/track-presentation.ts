import type { DocumentationAnswer, FactOrigin, TrackDetail, TrackSummary } from "../domain/types";
import { calculateMissingRequirements } from "../domain/workflow";

export function trackSummaryFromDetail(track: TrackDetail): TrackSummary {
  return {
    id: track.id,
    title: track.title,
    relativePath: track.relativePath,
    library: structuredClone(track.library),
    status: track.status,
    updatedAt: track.updatedAt,
    progress: track.progress,
    missingCount: track.missingCount,
    certificateValid: track.certificate.valid,
    legacy: track.legacy,
    coverEvidenceId: track.coverEvidenceId
  };
}

export interface TrackCheckSummary {
  documentation: "vollständig" | "unvollständig";
  fileIntegrity: "geprüft" | "offen";
  sunoMetadata: "erkannt" | "nicht erkannt";
  subscriptionCoverage: "passend" | "nicht passend" | "nicht geprüft" | "nicht erforderlich";
  warningCount: number;
}

export function trackCheckSummary(track: TrackDetail): TrackCheckSummary {
  const documentationMissing = calculateMissingRequirements(track, track.profileSnapshot).filter(
    (item) => item.stepId !== "integrity" && item.stepId !== "finalize"
  );
  const subscriptions = track.evidence.filter(
    (item) =>
      item.role === "subscription_payment" &&
      item.verified &&
      Boolean(item.sha256) &&
      !item.verificationError &&
      Boolean(item.coverageStart) &&
      Boolean(item.coverageEnd)
  );
  const generationDate = track.fields.sunoFinalGenerationDate;
  const subscriptionCoverage = !track.fields.commercialUseIntended
    ? ("nicht erforderlich" as const)
    : !generationDate || subscriptions.length === 0
      ? ("nicht geprüft" as const)
      : subscriptions.some((item) => item.coverageStart! <= generationDate && item.coverageEnd! >= generationDate)
        ? ("passend" as const)
        : ("nicht passend" as const);
  const warnings = new Set<string>();
  track.automation.consistencyIssues.forEach((item) => warnings.add(`consistency:${item.code}`));
  track.integrity.mismatchFiles.forEach((item) => warnings.add(`integrity:${item}`));
  track.evidence
    .filter((item) => !item.verified || Boolean(item.verificationError))
    .forEach((item) => warnings.add(`evidence:${item.id}`));
  (track.blockingDeviations ?? [])
    .filter((item) => !item.resolved)
    .forEach((item) => warnings.add(`deviation:${item.id}`));
  return {
    documentation: documentationMissing.length === 0 ? "vollständig" : "unvollständig",
    fileIntegrity: track.integrity.verified && track.integrity.mismatchFiles.length === 0 ? "geprüft" : "offen",
    sunoMetadata: track.automation.sunoMetadataDetected ? "erkannt" : "nicht erkannt",
    subscriptionCoverage,
    warningCount: warnings.size
  };
}

export function factOriginLabel(origin: FactOrigin): string {
  if (origin === "evidence_derived_metadata") return "Automatisch aus Suno-WAV erkannt";
  if (origin === "user_confirmed_fact") return "Nutzerangabe";
  return "Noch nicht dokumentiert";
}

export function documentationAnswerLabel(value: DocumentationAnswer | null): string {
  if (value === "yes") return "YES";
  if (value === "no") return "NO";
  if (value === "not_documented") return "NOT DOCUMENTED";
  return "NOT ANSWERED";
}

export function generativeAiAudioDetailState(
  value: boolean | null
): "details_required" | "not_applicable" | "not_documented" {
  if (value === true) return "details_required";
  if (value === false) return "not_applicable";
  return "not_documented";
}
