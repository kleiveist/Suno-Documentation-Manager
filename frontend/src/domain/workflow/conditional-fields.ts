import type { GlobalProfile, TrackFields } from "../types";

export const isAiArtwork = (fields: TrackFields): boolean =>
  fields.artworkOrigin === "ai_generated" || fields.artworkOrigin === "ai_assisted";

function addSourceFields(fields: TrackFields, visible: Set<string>): void {
  if (fields.externalAudioUploaded === true) {
    visible.add("externalAudioSource");
    visible.add("externalAudioOwnership");
    visible.add("externalAudioLicense");
    visible.add("externalAudioFile");
  }
  if (fields.ownAudioUploaded === true) {
    visible.add("ownAudioSource");
    visible.add("ownAudioOwnership");
    visible.add("ownAudioFile");
  }
  if (fields.codeBasedGeneration === true) {
    visible.add("sourceCodeFile");
    visible.add("codeAudioPostProcessed");
    visible.add("codeGeneratedAudioFile");
    if (fields.codeAudioPostProcessed === true) {
      visible.add("codeAudioPostProcessingOperations");
      if (fields.codeAudioPostProcessingOperations.includes("Other post-processing")) {
        visible.add("codeAudioPostProcessingNote");
      }
    }
  }
  if (fields.thirdPartySamplesUploaded === true) {
    visible.add("thirdPartySampleSource");
    visible.add("thirdPartySampleOwnership");
    visible.add("thirdPartySampleFile");
    visible.add("thirdPartySampleLicense");
  }
}

function addEditingAndContentFields(fields: TrackFields, visible: Set<string>): void {
  if (fields.humanEditingPerformed === true) visible.add("humanEditingDetails");
  if (fields.postExportEditingPerformed === true) visible.add("postExportEditingDetails");
  if (fields.sunoContentClassification !== null && fields.sunoContentClassification !== "EMPTY") {
    visible.add("sunoLyricsContentSource");
    visible.add("sunoLyricsFieldText");
    if (fields.sunoContentClassification === "OTHER") visible.add("sunoLyricsOtherContentType");
  }
}

function addAudioTransparencyFields(fields: TrackFields, visible: Set<string>): void {
  if (fields.generativeAiUsed !== true) return;
  visible.add("audioAiSystem");
  visible.add("audioAiAssessment");
  visible.add("audioDisclosure");
  if (fields.audioDisclosureApplied === "yes") {
    visible.add("audioDisclosureLocations");
    visible.add("audioDisclosureText");
  }
  if (fields.audioDisclosureApplied === "no") visible.add("audioDisclosureReason");
}

function addArtworkFields(fields: TrackFields, visible: Set<string>): void {
  if (isAiArtwork(fields)) {
    visible.add("aiImageService");
    visible.add("aiArtworkOriginal");
    visible.add("disclosure");
  }
  if (fields.artworkOrigin === "human") visible.add("humanArtworkProcessOperations");
  if (fields.artworkOrigin === "ai_assisted") {
    visible.add("humanArtworkModifications");
    if (fields.humanArtworkModifications.includes("Other human editing")) visible.add("customArtworkChange");
  }
  if (fields.depictsRealPerson === true) visible.add("realPersonNotes");
  if (fields.depictsRealEvent === true) visible.add("realEventNotes");
  if (fields.containsTrademark === true) visible.add("trademarkNotes");
}

export function visibleConditionalFields(fields: TrackFields, _profile: GlobalProfile): Set<string> {
  void _profile;
  const visible = new Set<string>();
  addSourceFields(fields, visible);
  addEditingAndContentFields(fields, visible);
  addAudioTransparencyFields(fields, visible);
  addArtworkFields(fields, visible);
  return visible;
}

export function contentCheckAllNegative(fields: TrackFields): boolean {
  return fields.depictsRealPerson === false && fields.depictsRealEvent === false && fields.containsTrademark === false;
}
