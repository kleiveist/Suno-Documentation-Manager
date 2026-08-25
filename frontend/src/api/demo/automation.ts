import type { ByteIdenticalPair, ConsistencyIssue, FactOrigin, TrackDetail } from "../../domain/types";

function reconcileAutomaticDate(
  currentValue: string,
  previousOrigin: FactOrigin,
  derivedValue: string,
  previousDerivedValue = derivedValue
): { value: string; origin: FactOrigin } {
  if (!derivedValue) {
    if (previousOrigin === "evidence_derived_metadata" && currentValue === previousDerivedValue) currentValue = "";
    return {
      value: currentValue,
      origin: currentValue.trim() ? "user_confirmed_fact" : "not_documented"
    };
  }
  return { value: derivedValue, origin: "evidence_derived_metadata" };
}

function reconcileAutomaticGenerationId(
  currentValue: string,
  previousOrigin: FactOrigin,
  derivedValue: string,
  previousDerivedValue = derivedValue
): { value: string; origin: FactOrigin } {
  if (previousOrigin === "evidence_derived_metadata" && currentValue !== previousDerivedValue) {
    previousOrigin = "user_confirmed_fact";
  }
  if (previousOrigin === "evidence_derived_metadata") {
    return derivedValue
      ? { value: derivedValue, origin: "evidence_derived_metadata" }
      : { value: "", origin: "not_documented" };
  }
  if (!currentValue.trim() && derivedValue) {
    return { value: derivedValue, origin: "evidence_derived_metadata" };
  }
  return {
    value: currentValue,
    origin: currentValue.trim() ? "user_confirmed_fact" : "not_documented"
  };
}

interface AutomaticValue {
  value: string;
  origin: FactOrigin;
}

interface AutomaticFieldValues {
  finalGeneration: AutomaticValue;
  finalGenerationId: AutomaticValue;
  productionEnd: AutomaticValue;
  downloadExport: AutomaticValue;
  finalExport: AutomaticValue;
}

function sunoAutomationEvidence(track: TrackDetail) {
  return track.evidence.find(
    (item) =>
      item.role === "suno_final_export" &&
      item.verified &&
      Boolean(item.sha256) &&
      !item.verificationError &&
      Boolean(item.metadata?.sunoStudioDetected)
  );
}

function automaticFieldValues(
  track: TrackDetail,
  suno: ReturnType<typeof sunoAutomationEvidence>
): AutomaticFieldValues {
  const previous = track.automation;
  const createdDate = suno?.metadata?.sunoCreatedDate?.trim() ?? "";
  const sunoId = suno?.metadata?.sunoId?.trim() ?? "";
  const previousCreatedDate = previous.sunoCreatedTimestamp?.slice(0, 10) ?? "";
  const previousSunoId = previous.sunoId ?? "";
  const editable = track.status !== "FINALIZED" && track.status !== "SUPERSEDED";
  return {
    finalGeneration: editable
      ? reconcileAutomaticDate(
          track.fields.sunoFinalGenerationDate,
          previous.finalGenerationOrigin,
          createdDate,
          previousCreatedDate
        )
      : { value: track.fields.sunoFinalGenerationDate, origin: previous.finalGenerationOrigin },
    finalGenerationId: editable
      ? reconcileAutomaticGenerationId(
          track.fields.sunoFinalGenerationId,
          previous.finalGenerationIdOrigin,
          sunoId,
          previousSunoId
        )
      : { value: track.fields.sunoFinalGenerationId, origin: previous.finalGenerationIdOrigin },
    productionEnd: editable
      ? reconcileAutomaticDate(
          track.fields.productionEndDate,
          previous.productionEndOrigin,
          createdDate,
          previousCreatedDate
        )
      : { value: track.fields.productionEndDate, origin: previous.productionEndOrigin },
    downloadExport: editable
      ? reconcileAutomaticDate(
          track.fields.sunoDownloadExportDate,
          previous.downloadExportOrigin,
          createdDate,
          previousCreatedDate
        )
      : { value: track.fields.sunoDownloadExportDate, origin: previous.downloadExportOrigin },
    finalExport: editable
      ? reconcileAutomaticDate(
          track.fields.finalExportDate,
          previous.finalExportOrigin,
          track.fields.postExportEditingPerformed === false ? createdDate : "",
          previousCreatedDate
        )
      : { value: track.fields.finalExportDate, origin: previous.finalExportOrigin }
  };
}

function applyAutomaticFieldValues(track: TrackDetail, values: AutomaticFieldValues): void {
  track.fields.sunoFinalGenerationDate = values.finalGeneration.value;
  track.fields.sunoFinalGenerationId = values.finalGenerationId.value;
  track.fields.productionEndDate = values.productionEnd.value;
  track.fields.sunoDownloadExportDate = values.downloadExport.value;
  track.fields.finalExportDate = values.finalExport.value;
}

function byteIdenticalPairs(track: TrackDetail): ByteIdenticalPair[] {
  const pairs: ByteIdenticalPair[] = [];
  const verified = track.evidence.filter((item) => item.verified && Boolean(item.sha256) && !item.verificationError);
  for (let index = 0; index < verified.length; index += 1) {
    for (let rightIndex = index + 1; rightIndex < verified.length; rightIndex += 1) {
      const left = verified[index];
      const right = verified[rightIndex];
      if (left.sha256 !== right.sha256) continue;
      pairs.push({
        leftEvidenceId: left.id,
        leftRole: left.role,
        rightEvidenceId: right.id,
        rightRole: right.role,
        sha256: left.sha256!
      });
    }
  }
  return pairs;
}

function releaseIdenticalToSunoExport(pairs: ByteIdenticalPair[]): boolean {
  return pairs.some(
    (pair) =>
      (pair.leftRole === "suno_final_export" && pair.rightRole === "release_wav") ||
      (pair.leftRole === "release_wav" && pair.rightRole === "suno_final_export")
  );
}

export function refreshAutomation(track: TrackDetail): void {
  const suno = sunoAutomationEvidence(track);
  const values = automaticFieldValues(track, suno);
  applyAutomaticFieldValues(track, values);
  const pairs = byteIdenticalPairs(track);
  const issues: ConsistencyIssue[] = [];
  track.automation = {
    finalGenerationIdOrigin: values.finalGenerationId.origin,
    finalGenerationOrigin: values.finalGeneration.origin,
    productionEndOrigin: values.productionEnd.origin,
    downloadExportOrigin: values.downloadExport.origin,
    finalExportOrigin: values.finalExport.origin,
    sunoMetadataDetected: Boolean(suno?.metadata?.sunoStudioDetected),
    sunoCreatedTimestamp: suno?.metadata?.sunoCreatedTimestamp || undefined,
    sunoId: suno?.metadata?.sunoId || undefined,
    releaseIdenticalToSunoExport: releaseIdenticalToSunoExport(pairs),
    byteIdenticalPairs: pairs,
    consistencyIssues: issues
  };
}
