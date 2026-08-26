import {
  calculateMissingRequirements,
  calculateProgress,
  evaluateRequirements,
  finalizationGate,
  stepStatuses,
  WORKFLOW_ID,
  WORKFLOW_VERSION
} from "../../domain/workflow";
import {
  emptyAudioScreeningSummary,
  emptyEvidenceMetadata,
  emptyTrackAutomation,
  emptyTrackFields
} from "../../domain/types";
import type {
  AudioScreeningSummary,
  EvidenceItem,
  GlobalProfile,
  TrackDetail,
  TrackFields,
  TrackLibraryAssignment
} from "../../domain/types";
import { refreshAutomation } from "./automation";
import { clone, evidence, managedArtworkName, now, sunoEvidence, trackRelativePath } from "./helpers";
import { notRecordedExternalTimestampSummary } from "./timestamp";

function demoProductionFields(title: string, complete: boolean): Partial<TrackFields> {
  return {
    title,
    productionStartDate: "2026-07-18",
    productionEndDate: "",
    sunoModel: complete ? "v4.5" : "",
    sunoProjectUrl: complete ? "https://suno.com/song/demo-project" : "",
    sunoFinalGenerationDate: "",
    sunoDownloadExportDate: complete ? "2026-07-24" : "",
    sunoPlanAtGeneration: complete ? "Premier" : "",
    finalExportDate: complete ? "2026-07-24" : "",
    instrumentalTrack: complete ? false : null,
    vocalLyricsPresent: complete ? true : null,
    vocalIntent: complete ? "VOCAL" : null,
    sunoLyricsFieldContent: null,
    sunoContentClassification: complete ? "VOCAL_LYRICS_ONLY" : null,
    sunoLyricsContentTypes: [],
    sunoLyricsContentSource: complete ? "human" : null,
    sunoLyricsFieldText: complete ? "Eigene Lyrics – im Track-Dokument vollständig gespeichert." : "",
    sunoStylePrompt: complete ? "cinematic synthwave, driving bass, wide vocal" : ""
  };
}

function demoTransparencyFields(complete: boolean): Partial<TrackFields> {
  return {
    externalAudioUploaded: false,
    ownAudioUploaded: false,
    codeBasedGeneration: false,
    thirdPartySamplesUploaded: false,
    humanEditingPerformed: complete,
    humanEditingDetails: complete ? "Timing and cuts | EQ | Loudness adjustment" : "",
    postExportEditingPerformed: false,
    artworkOrigin: complete ? "ai_assisted" : "",
    aiImageService: complete ? "OpenAI" : "",
    humanArtworkModifications: complete ? ["Typography added", "Color correction"] : [],
    depictsRealPerson: complete ? false : null,
    depictsRealEvent: complete ? false : null,
    containsTrademark: complete ? false : null,
    disclosureApplied: complete,
    disclosureText: "AI-assisted",
    generativeAiUsed: complete ? true : null,
    audioAiSystem: complete ? "Suno" : "",
    aiAssistedAudioElements: complete ? "yes" : null,
    aiGeneratedAudioElements: complete ? "yes" : null,
    realPersonVoiceIntentionallyImitated: complete ? "no" : null,
    realPersonIdentityIntentionallyRepresented: complete ? "no" : null,
    realEventRepresentedAsAuthenticRecording: complete ? "no" : null,
    realLocationInstitutionEventPresentedAsAuthenticAiRecording: complete ? "no" : null,
    audioDisclosureApplied: complete ? "yes" : null,
    audioDisclosureLocations: complete ? ["Release-Metadaten"] : [],
    audioDisclosureText: complete ? "AI-generated audio" : ""
  };
}

function demoTrackFields(profile: GlobalProfile, title: string, complete: boolean): TrackDetail["fields"] {
  return {
    ...emptyTrackFields(profile),
    ...demoProductionFields(title, complete),
    ...demoTransparencyFields(complete)
  };
}

interface DemoArtworkEvidence {
  original: EvidenceItem;
  disclosed: EvidenceItem;
  final: EvidenceItem;
}

function demoArtworkEvidence(title: string, disclosureText: string): DemoArtworkEvidence {
  const original = evidence("ai_artwork_original", managedArtworkName(title, "ai_artwork_original", "png")!);
  const disclosed: EvidenceItem = {
    ...evidence("ai_artwork_edited", managedArtworkName(title, "ai_artwork_edited", "png")!),
    provenance: "generated_disclosure",
    derivedFromEvidenceId: original.id,
    generatorVersion: "local-disclosure-v1",
    generatedDisclosureText: disclosureText
  };
  const final: EvidenceItem = {
    ...evidence("final_artwork", managedArtworkName(title, "final_artwork", "jpeg")!),
    sha256: disclosed.sha256
  };
  return { original, disclosed, final };
}

function completeTrackEvidence(title: string, artwork: DemoArtworkEvidence): EvidenceItem[] {
  return [
    evidence("release_wav", `${title}.wav`),
    sunoEvidence(`${title}_SUNO_FINAL.wav`),
    {
      ...evidence("subscription_payment", "subscription_2026-07.pdf"),
      provenance: "global_copy",
      sourceGlobalEvidenceId: "demo-global-subscription",
      coverageStart: "2026-07-01",
      coverageEnd: "2026-07-31"
    },
    artwork.original,
    artwork.disclosed,
    artwork.final,
    {
      ...evidence("suno_terms_rights", "suno_terms.pdf"),
      provenance: "global_copy",
      sourceGlobalEvidenceId: "demo-global-terms",
      metadata: {
        ...emptyEvidenceMetadata(),
        originalFileName: "suno_terms.pdf",
        documentTitle: "Suno Terms of Service",
        provider: "Suno, Inc.",
        retrievalDate: "2026-07-18"
      }
    }
  ];
}

function demoTrackEvidence(title: string, complete: boolean, artwork: DemoArtworkEvidence): EvidenceItem[] {
  return complete ? completeTrackEvidence(title, artwork) : [evidence("suno_screenshot", `${title}_SUNO.png`)];
}

function demoAudioScreening(items: EvidenceItem[], complete: boolean): AudioScreeningSummary {
  const release = items.find((item) => item.role === "release_wav");
  const audioScreening = clone(emptyAudioScreeningSummary);
  if (!complete || !release?.sha256) return audioScreening;
  audioScreening.local = {
    status: "fingerprint_generated",
    message: "Browser demo presentation only: no local audio file was analysed and no provider request was made.",
    engine: "Chromaprint",
    engineVersion: "demo",
    sourceEvidenceId: release.id,
    sourceRelativePath: release.relativePath,
    sourceSha256: release.sha256,
    sourceSizeBytes: release.sizeBytes,
    durationMilliseconds: 213_450,
    fingerprintAlgorithm: "2",
    generatedAt: now(),
    artifactRelativePath: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json",
    artifactSha256: "d".repeat(64)
  };
  audioScreening.external = {
    ...emptyAudioScreeningSummary.external,
    status: "skipped_not_configured",
    message: "Browser demo: optional ACRCloud screening is not configured and no provider was contacted.",
    sourceEvidenceId: release.id,
    sourceRelativePath: release.relativePath,
    sourceSha256: release.sha256,
    sourceSizeBytes: release.sizeBytes,
    matches: []
  };
  return audioScreening;
}

interface DemoTrackInput {
  id: string;
  title: string;
  profile: GlobalProfile;
  complete: boolean;
  library: TrackLibraryAssignment;
  fields: TrackFields;
  items: EvidenceItem[];
  audioScreening: AudioScreeningSummary;
}

function demoTrack(input: DemoTrackInput): TrackDetail {
  const { id, title, profile, complete, library, fields, items, audioScreening } = input;
  return {
    id,
    title,
    relativePath: trackRelativePath(library, title),
    library: clone(library),
    status: complete ? "READY" : "ACTIVE",
    updatedAt: now(),
    progress: 0,
    missingCount: 0,
    workflowId: WORKFLOW_ID,
    workflowVersion: WORKFLOW_VERSION,
    profileSnapshot: clone(profile),
    automation: emptyTrackAutomation(),
    fields,
    steps: [],
    evidence: items,
    documents: {
      generated: complete,
      current: complete,
      generatedAt: complete ? now() : undefined,
      templateVersion: "1.11",
      files: complete
        ? ["02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md"]
        : []
    },
    integrity: {
      generated: complete,
      verified: complete,
      fileCount: complete ? 17 : 0,
      verifiedCount: complete ? 17 : 0,
      mismatchFiles: []
    },
    certificate: { valid: false },
    externalTimestamps: [],
    externalTimestampSummary: notRecordedExternalTimestampSummary(),
    audioScreening,
    finalizationAnchors: []
  };
}

export function makeTrack(
  id: string,
  title: string,
  profile: GlobalProfile,
  complete = false,
  library: TrackLibraryAssignment = { section: "single" }
): TrackDetail {
  const fields = demoTrackFields(profile, title, complete);
  const artwork = demoArtworkEvidence(title, fields.disclosureText);
  const items = demoTrackEvidence(title, complete, artwork);
  const audioScreening = demoAudioScreening(items, complete);
  const track = demoTrack({ id, title, profile, complete, library, fields, items, audioScreening });
  refresh(track);
  return track;
}

export function refresh(track: TrackDetail): void {
  track.title = track.fields.title;
  track.coverEvidenceId = track.evidence.find(
    (item) => item.role === "final_artwork" && item.verified && Boolean(item.sha256) && !item.verificationError
  )?.id;
  refreshAutomation(track);
  const profile = track.profileSnapshot;
  const missing = calculateMissingRequirements(track, profile);
  track.steps = stepStatuses(track, profile);
  track.progress = calculateProgress(evaluateRequirements(track, profile));
  track.missingCount = missing.length;
  if (track.status !== "FINALIZED" && track.status !== "SUPERSEDED") {
    track.status = finalizationGate(track, profile).valid ? "READY" : track.progress > 0 ? "ACTIVE" : "DRAFT";
  }
  track.updatedAt = now();
}

export function sameTrackDocumentationProfile(left: GlobalProfile, right: GlobalProfile): boolean {
  return (
    left.artistName === right.artistName &&
    left.sunoProfileName === right.sunoProfileName &&
    left.sunoHandle === right.sunoHandle &&
    left.sunoPlan === right.sunoPlan &&
    left.subscriptionStartDate === right.subscriptionStartDate &&
    left.defaultCommercialUse === right.defaultCommercialUse &&
    left.defaultAiImageService === right.defaultAiImageService &&
    left.artworkTransparencyPolicy === right.artworkTransparencyPolicy &&
    left.disclosureText === right.disclosureText
  );
}
