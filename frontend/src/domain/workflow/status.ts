import type {
  GlobalProfile,
  StepId,
  StepStatus,
  TrackDetail,
  TrackStatus,
  ValidationResult,
  WorkflowStepState
} from "../types";
import { WORKFLOW_STEPS, statusLabel, stepLabel } from "./definitions";
import {
  evaluateRequirements,
  type MissingRequirement,
  type RequirementEvaluation,
  type RequirementTrack
} from "./requirements";

type FinalizationTrack = Pick<
  TrackDetail,
  "fields" | "evidence" | "documents" | "integrity" | "steps" | "blockingDeviations" | "automation" | "audioScreening"
>;

export function calculateMissingRequirements(track: RequirementTrack, profile: GlobalProfile): MissingRequirement[] {
  return evaluateRequirements(track, profile)
    .filter((item) => !item.completed)
    .map(({ completed: _completed, ...item }) => {
      void _completed;
      return item;
    });
}

export function deriveStepStatus(
  stepId: StepId,
  missing: MissingRequirement[],
  stored?: WorkflowStepState,
  applicable = true
): StepStatus {
  if (!applicable && stored?.status === "N_A" && stored.naReason?.trim()) return "N_A";
  if (missing.some((item) => item.stepId === stepId)) {
    if (["FAIL", "BLOCKED", "NOT_VERIFIED"].includes(stored?.status ?? "")) return stored!.status;
    return "NOT_RUN";
  }
  return "PASS";
}

export function calculateProgress(requirements: RequirementEvaluation[]): number {
  const total = requirements.length;
  if (total === 0) return 100;
  const complete = requirements.filter((item) => item.completed).length;
  return Math.round((complete / total) * 100);
}

export function finalizationGate(track: FinalizationTrack, profile: GlobalProfile): ValidationResult {
  const missing = calculateMissingRequirements(track, profile);
  const blockingStatuses = track.steps.filter((step) => ["FAIL", "BLOCKED", "NOT_VERIFIED"].includes(step.status));
  const invalidNa = track.steps.filter((step) => step.status === "N_A" && !step.naReason?.trim());
  const blockingItems = [
    ...blockingStatuses.map((step) => `${stepLabel(step.id)}: ${statusLabel(step.status)}`),
    ...invalidNa.map((step) => `${stepLabel(step.id)}: N/A benötigt eine Begründung`),
    ...track.integrity.mismatchFiles.map((file) => `Integritätsabweichung: ${file}`),
    ...(track.blockingDeviations ?? [])
      .filter((item) => item.blocking && !item.resolved)
      .map((item) => `Abweichung: ${item.description}`)
  ];
  return {
    valid: missing.length === 0 && blockingItems.length === 0,
    missingItems: missing.map((item) => item.label),
    blockingItems
  };
}

export function deriveTrackStatus(track: TrackDetail, profile: GlobalProfile): TrackStatus {
  if (track.status === "SUPERSEDED") return "SUPERSEDED";
  if (track.status === "FINALIZED" && track.certificate.valid && track.integrity.mismatchFiles.length === 0)
    return "FINALIZED";
  const gate = finalizationGate(track, profile);
  if (gate.valid) return "READY";
  const hasActivity =
    track.evidence.length > 0 ||
    track.documents.generated ||
    calculateProgress(evaluateRequirements(track, profile)) > 0;
  return hasActivity ? "ACTIVE" : "DRAFT";
}

export function stepStatuses(track: TrackDetail, profile: GlobalProfile): WorkflowStepState[] {
  const requirements = evaluateRequirements(track, profile);
  const missing = requirements.filter((item) => !item.completed);
  const statuses = WORKFLOW_STEPS.map((definition) => {
    const stored = track.steps.find((step) => step.id === definition.id);
    const applicable = requirements.some((item) => item.stepId === definition.id);
    return {
      id: definition.id,
      status: deriveStepStatus(definition.id, missing, stored, applicable),
      naReason: stored?.naReason,
      updatedAt: stored?.updatedAt
    };
  });
  const finalize = statuses.find((step) => step.id === "finalize");
  if (
    finalize?.status === "PASS" &&
    statuses.some((step) => step.id !== "finalize" && !["PASS", "N_A"].includes(step.status))
  ) {
    finalize.status = "BLOCKED";
  }
  return statuses;
}
