import type { TrackDetail } from "../../domain/types";

export function normalizeCanonicalSunoSemantics(fields: TrackDetail["fields"]): void {
  if (fields.sunoContentClassification === null) return;
  fields.sunoLyricsFieldContent = null;
  fields.sunoLyricsContentTypes = [];
  if (fields.sunoContentClassification === "EMPTY") {
    fields.sunoLyricsContentSource = null;
    fields.sunoLyricsFieldText = "";
    fields.sunoLyricsOtherContentType = "";
  } else if (fields.sunoContentClassification !== "OTHER") {
    fields.sunoLyricsOtherContentType = "";
  }
}

export function migrateLegacySunoSemantics(fields: TrackDetail["fields"]): void {
  if (fields.sunoContentClassification !== null) {
    normalizeCanonicalSunoSemantics(fields);
    return;
  }
  const legacy = fields.sunoLyricsContentTypes ?? [];
  let classification: TrackDetail["fields"]["sunoContentClassification"] = null;
  if (fields.sunoLyricsFieldContent === false) {
    classification = "EMPTY";
  } else if (legacy.length > 0) {
    const onlyVocal = legacy.every((value) => value === "vocal_lyrics");
    const isInstruction = (value: (typeof legacy)[number]): boolean =>
      ["structure_instructions", "sound_instructions", "arrangement_instructions"].includes(value);
    const onlyInstructions = legacy.every(isInstruction);
    const onlyOther = legacy.every((value) => value === "other");
    const hasVocal = legacy.includes("vocal_lyrics");
    const hasInstruction = legacy.some(isInstruction);
    const onlyVocalAndInstructions = legacy.every((value) => value === "vocal_lyrics" || isInstruction(value));
    if (onlyVocal) classification = "VOCAL_LYRICS_ONLY";
    else if (onlyInstructions) classification = "STRUCTURE_ONLY";
    else if (onlyOther) classification = "OTHER";
    else if (hasVocal && hasInstruction && onlyVocalAndInstructions) classification = "MIXED";
  }
  if (classification === null) return;
  fields.sunoContentClassification = classification;
  normalizeCanonicalSunoSemantics(fields);
}
