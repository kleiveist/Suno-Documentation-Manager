import { describe, expect, it } from "vitest";

import type { OperationProgress } from "../domain/types";
import { operationProgressPercent, operationStageLabel, type LongOperationKind } from "./progress";

function progress(stage: string, ratio = 0): OperationProgress {
  return {
    stage,
    processedBytes: Math.round(ratio * 100),
    totalBytes: 100,
    processedFiles: 0,
    totalFiles: 0
  };
}

function expectFixed(kind: LongOperationKind, expected: Readonly<Record<string, number>>): void {
  for (const [stage, percent] of Object.entries(expected)) {
    expect(operationProgressPercent(kind, progress(stage))).toBe(percent);
  }
}

describe("operation progress", () => {
  it("keeps the document and verification stage percentages stable", () => {
    expectFixed("documents", {
      preparing_documents: 5,
      rendering_documents: 18,
      finalizing_documents: 94
    });
    expect(operationProgressPercent("documents", progress("writing_documents", 0.5))).toBe(56);
    expectFixed("verification", { reading_hash_list: 6, comparing_hashes: 96 });
    expect(operationProgressPercent("verification", progress("verifying", 0.5))).toBe(51);
  });

  it("keeps the hashing stage percentages stable", () => {
    expectFixed("hashes", {
      discovering_files: 4,
      writing_hash_list: 53,
      preparing_verification: 57,
      reading_hash_list: 60,
      comparing_hashes: 96
    });
    expect(operationProgressPercent("hashes", progress("hashing", 0.5))).toBe(29);
    expect(operationProgressPercent("hashes", progress("verifying", 0.5))).toBe(78);
  });

  it("keeps the finalization stage percentages stable", () => {
    expectFixed("finalization", {
      validating_finalization_gate: 5,
      collecting_final_snapshot: 13,
      writing_finalization_marker: 22,
      generating_certificate: 35,
      verifying_certificate: 50,
      verifying_final_snapshot: 58,
      reading_hash_list: 58,
      comparing_hashes: 92,
      saving_final_snapshot: 97
    });
    expect(operationProgressPercent("finalization", progress("verifying", 0.5))).toBe(76);
  });

  it("keeps the audio-screening stage percentages stable", () => {
    expectFixed("audio_screening", {
      preparing_audio: 5,
      fingerprint_complete: 55,
      preparing_external_check: 62,
      sending_provider_request: 70,
      waiting_provider_response: 80,
      processing_provider_response: 89,
      saving_screening_result: 96
    });
    expect(operationProgressPercent("audio_screening", progress("fingerprinting_audio", 0.5))).toBe(30);
  });

  it("keeps common fallbacks and the audio completion label stable", () => {
    expect(operationProgressPercent("documents", progress("complete"))).toBe(100);
    expect(operationProgressPercent("hashes", progress("saving_result"))).toBe(98);
    expect(operationProgressPercent("verification", progress("unknown"))).toBe(2);
    expect(operationStageLabel("complete", "audio_screening")).toBe("Prüfung abgeschlossen");
    expect(operationStageLabel("unknown")).toBe("Lokaler Vorgang läuft");
  });
});
