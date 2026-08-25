import type { OperationProgress } from "../domain/types";

export type LongOperationKind = "documents" | "hashes" | "verification" | "finalization" | "audio_screening";

type ProgressCalculator = (ratio: number) => number;

const fixedProgress: Record<LongOperationKind, Readonly<Record<string, number>>> = {
  documents: {
    preparing_documents: 5,
    rendering_documents: 18,
    finalizing_documents: 94
  },
  hashes: {
    discovering_files: 4,
    writing_hash_list: 53,
    preparing_verification: 57,
    reading_hash_list: 60,
    comparing_hashes: 96
  },
  verification: {
    reading_hash_list: 6,
    comparing_hashes: 96
  },
  finalization: {
    validating_finalization_gate: 5,
    collecting_final_snapshot: 13,
    writing_finalization_marker: 22,
    generating_certificate: 35,
    verifying_certificate: 50,
    verifying_final_snapshot: 58,
    reading_hash_list: 58,
    comparing_hashes: 92,
    saving_final_snapshot: 97
  },
  audio_screening: {
    preparing_audio: 5,
    fingerprint_complete: 55,
    preparing_external_check: 62,
    sending_provider_request: 70,
    waiting_provider_response: 80,
    processing_provider_response: 89,
    saving_screening_result: 96
  }
};

const calculatedProgress: Record<LongOperationKind, Readonly<Record<string, ProgressCalculator>>> = {
  documents: {
    writing_documents: (ratio) => Math.round(22 + ratio * 68)
  },
  hashes: {
    hashing: (ratio) => Math.round(7 + ratio * 43),
    verifying: (ratio) => Math.round(62 + ratio * 31)
  },
  verification: {
    verifying: (ratio) => Math.round(10 + ratio * 82)
  },
  finalization: {
    verifying: (ratio) => Math.round(62 + ratio * 28)
  },
  audio_screening: {
    fingerprinting_audio: (ratio) => Math.round(9 + ratio * 42)
  }
};

function progressRatio(progress: OperationProgress): number {
  if (progress.totalBytes > 0) return Math.min(progress.processedBytes / progress.totalBytes, 1);
  if (progress.totalFiles > 0) return Math.min(progress.processedFiles / progress.totalFiles, 1);
  return 0;
}

export function operationProgressPercent(kind: LongOperationKind, progress: OperationProgress): number {
  if (progress.stage === "complete") return 100;
  if (progress.stage === "saving_result") return 98;
  const ratio = progressRatio(progress);
  const calculator = calculatedProgress[kind][progress.stage];
  return calculator?.(ratio) ?? fixedProgress[kind][progress.stage] ?? 2;
}

const stageLabels: Readonly<Record<string, string>> = {
  discovering_files: "Dateien werden erfasst",
  hashing: "Digitale Fingerabdrücke entstehen",
  writing_hash_list: "Hashliste wird geschrieben",
  preparing_verification: "Gegenprüfung wird vorbereitet",
  reading_hash_list: "Gespeicherte Hashliste wird gelesen",
  verifying: "Dateien werden erneut geprüft",
  comparing_hashes: "Ergebnisse werden verglichen",
  preparing_documents: "Dokumentdaten werden gesammelt",
  rendering_documents: "Dokumente werden zusammengesetzt",
  writing_documents: "Dokumente werden sicher geschrieben",
  finalizing_documents: "Dokumentsatz wird aufgeräumt",
  saving_result: "Ergebnis wird lokal gespeichert",
  preparing_audio: "Audio wird vorbereitet",
  fingerprinting_audio: "Lokaler Audio-Fingerprint wird erzeugt",
  fingerprint_complete: "Chromaprint-Fingerprint abgeschlossen",
  preparing_external_check: "Externe Katalogprüfung wird vorbereitet",
  sending_provider_request: "Audioausschnitt wird übertragen",
  waiting_provider_response: "ACRCloud-Ergebnis wird erwartet",
  processing_provider_response: "Provider-Ergebnis wird geprüft",
  saving_screening_result: "Prüfergebnis wird lokal dokumentiert",
  complete: "Vorgang abgeschlossen",
  validating_finalization_gate: "Finalisierungs-Gate wird geprüft",
  collecting_final_snapshot: "Unveränderlicher Snapshot wird vorbereitet",
  writing_finalization_marker: "Transaktion wird abgesichert",
  generating_certificate: "Zertifikat und Manifest entstehen",
  verifying_certificate: "Zertifikatssatz wird geprüft",
  verifying_final_snapshot: "Finaler Snapshot wird gegengeprüft",
  saving_final_snapshot: "Finalisierung wird verbindlich gespeichert"
};

export function operationStageLabel(stage: string, kind?: LongOperationKind): string {
  if (stage === "complete" && kind === "audio_screening") return "Prüfung abgeschlossen";
  return stageLabels[stage] ?? "Lokaler Vorgang läuft";
}

export const OPERATION_PROGRESS_CONFIGURATION = {
  documents: {
    eyebrow: "Dokument-Manufaktur",
    title: "Dein Dokumentsatz entsteht",
    iconName: "file" as const,
    steps: ["Daten sammeln", "Inhalte rendern", "Dateien schreiben", "Sicher abschließen"],
    thresholds: [0, 18, 22, 94],
    tips: [
      "Jede Datei wird zuerst vollständig aufgebaut und anschließend atomar veröffentlicht.",
      "Lyrics und Style-Prompt werden im Suno-Ordner als portable Markdown-Dateien abgelegt.",
      "Vorhandene verwaltete Dokumente werden nicht mit halbfertigen Inhalten überschrieben."
    ]
  },
  hashes: {
    eyebrow: "SHA-256-Werkstatt",
    title: "Digitale Fingerabdrücke entstehen",
    iconName: "hash" as const,
    steps: ["Dateien finden", "Bytes hashen", "Hashliste schreiben", "Gegenprüfung"],
    thresholds: [0, 7, 53, 57],
    tips: [
      "Große Dateien werden blockweise gelesen – sie müssen dafür nicht komplett in den Arbeitsspeicher.",
      "Schon ein einziges geändertes Byte erzeugt einen anderen SHA-256-Fingerabdruck.",
      "Nach dem Schreiben liest die App alle Dateien erneut und prüft die neue Hashliste."
    ]
  },
  verification: {
    eyebrow: "Integritätsradar",
    title: "Prüfsummen werden verifiziert",
    iconName: "shield" as const,
    steps: ["Hashliste lesen", "Dateien prüfen", "Werte vergleichen", "Ergebnis sichern"],
    thresholds: [0, 10, 96, 98],
    tips: [
      "Die Prüfung berechnet jeden Fingerabdruck erneut und vertraut keinem gespeicherten Dateistatus.",
      "Zusätzliche, fehlende und veränderte Dateien werden getrennt als Abweichung erkannt.",
      "Die Verifikation bleibt lokal; keine Datei und kein Hash verlässt den Workspace."
    ]
  },
  finalization: {
    eyebrow: "Zertifikats-Tresor",
    title: "Dein finaler Snapshot wird versiegelt",
    iconName: "certificate" as const,
    steps: ["Gate bestätigen", "Zertifikat erzeugen", "Snapshot gegenprüfen", "Sicher versiegeln"],
    thresholds: [0, 13, 58, 92],
    tips: [
      "Zertifikat, Evidence-Manifest und Zertifikats-Hashliste werden als zusammengehöriger Satz veröffentlicht.",
      "Vor dem Abschluss prüft die App die komplette SHA-256-Liste noch einmal von der Festplatte.",
      "Der finalisierte Snapshot bleibt unverändert; spätere Änderungen beginnen als ausdrücklich angelegte Revision."
    ]
  },
  audio_screening: {
    eyebrow: "Pre-Release Audio Screening",
    title: "Audio wird technisch geprüft",
    iconName: "tracks" as const,
    steps: ["Audio vorbereiten", "Chromaprint erzeugen", "ACRCloud optional", "Ergebnis sichern"],
    thresholds: [0, 8, 62, 96],
    tips: [
      "Der lokale Chromaprint-Fingerprint bleibt auf diesem Gerät und benötigt keine Netzwerkverbindung.",
      "Eine ACRCloud-Anfrage wird nur nach der ausdrücklichen Aktion für diesen Track vorbereitet.",
      "Ein Ergebnis dokumentiert technische Audio-Erkennung, keine Rechte-, Lizenz- oder Rechtsbewertung."
    ]
  }
} as const;
