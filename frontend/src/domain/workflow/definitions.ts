import type { StepId, StepStatus, TrackStatus } from "../types";

export interface WorkflowStepDefinition {
  id: StepId;
  number: string;
  shortLabel: string;
  title: string;
  description: string;
  required: boolean;
}

export const WORKFLOW_ID = "suno-track";
export const WORKFLOW_VERSION = "1.9";

export const WORKFLOW_STEPS: readonly WorkflowStepDefinition[] = [
  {
    id: "track",
    number: "01",
    shortLabel: "Track",
    title: "Track",
    description: "Titel und Produktionszeitraum",
    required: true
  },
  {
    id: "source",
    number: "02",
    shortLabel: "Quelle",
    title: "Source",
    description: "Audioquellen und Rechtezuordnung",
    required: true
  },
  {
    id: "suno",
    number: "03",
    shortLabel: "Suno",
    title: "Suno",
    description: "Projekt, Modell und Erstellungstarif",
    required: true
  },
  {
    id: "human_work",
    number: "04",
    shortLabel: "Human work",
    title: "Menschliche Arbeit",
    description: "Vocal Lyrics, Suno-Feldinhalt und bestätigte Bearbeitungen",
    required: true
  },
  {
    id: "artwork",
    number: "05",
    shortLabel: "Artwork",
    title: "Artwork",
    description: "Entstehung und Content-Check",
    required: true
  },
  {
    id: "ai_transparency",
    number: "06",
    shortLabel: "AI Disclosure",
    title: "KI-Transparenz",
    description: "Audio-Assessment und Artwork-Disclosure",
    required: true
  },
  {
    id: "release",
    number: "07",
    shortLabel: "Release",
    title: "Release",
    description: "Letzte Bearbeitung und Release-Dateien",
    required: true
  },
  {
    id: "evidence_licenses",
    number: "08",
    shortLabel: "Evidence",
    title: "Evidence & Lizenzen",
    description: "Nachweise vollständig zuordnen",
    required: true
  },
  {
    id: "integrity",
    number: "09",
    shortLabel: "Integrität",
    title: "Integrität",
    description: "Dokumente, SHA-256 und Verifikation",
    required: true
  },
  {
    id: "finalize",
    number: "10",
    shortLabel: "Abschluss",
    title: "Finalisieren",
    description: "Gate prüfen und Zertifikat erzeugen",
    required: true
  }
];

export function statusLabel(status: StepStatus | TrackStatus): string {
  const labels: Record<StepStatus | TrackStatus, string> = {
    DRAFT: "Entwurf",
    ACTIVE: "In Arbeit",
    READY: "Bereit",
    FINALIZED: "Finalisiert",
    SUPERSEDED: "Ersetzt",
    NOT_RUN: "Offen",
    PASS: "Erfüllt",
    FAIL: "Fehlgeschlagen",
    BLOCKED: "Blockiert",
    N_A: "N/A",
    NOT_VERIFIED: "Nicht verifiziert"
  };
  return labels[status];
}

export function stepLabel(stepId: StepId): string {
  return WORKFLOW_STEPS.find((step) => step.id === stepId)?.title ?? stepId;
}
