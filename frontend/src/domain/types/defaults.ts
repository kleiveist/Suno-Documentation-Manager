import type {
  AudioScreeningSettings,
  AudioScreeningSummary,
  EvidenceMetadata,
  GlobalProfile,
  TimestampSettings,
  TrackAutomation,
  TrackFields
} from "../types";

export const emptyProfile: GlobalProfile = {
  artistName: "",
  sunoProfileName: "",
  sunoHandle: "",
  sunoPlan: "",
  subscriptionStartDate: "",
  defaultCommercialUse: true,
  defaultAiImageService: "",
  artworkTransparencyPolicy: "always",
  disclosureText: "AI-assisted",
  certificateLanguage: "en"
};

export const emptyTimestampSettings: TimestampSettings = {
  enabled: false,
  provider: "disabled",
  autoAfterFinalization: false,
  custom: {
    providerName: "",
    endpoint: "",
    authenticationMode: "none",
    username: "",
    clientCertificatePath: "",
    caCertificatePath: "",
    policyOid: "",
    timeoutSeconds: 15
  },
  status: "disabled",
  statusMessage: "External timestamp service is disabled."
};

export const emptyAudioScreeningSummary: AudioScreeningSummary = {
  local: {
    status: "not_run",
    message: "The local Chromaprint screening has not run for the current release file."
  },
  external: {
    provider: "ACRCloud",
    status: "not_run",
    message: "The optional ACRCloud check has not been requested.",
    screeningMode: "single_sample",
    requestedIntensityPercent: 5,
    dynamicByTrackDuration: true,
    referenceDurationSeconds: null,
    targetDurationMilliseconds: 0,
    plannedRequestCount: 0,
    executedRequestCount: 0,
    uniqueSampleCount: 0,
    overlappingSampleCount: 0,
    duplicateSampleCount: 0,
    uniqueSampleDurationMilliseconds: 0,
    trackCoveragePercent: 0,
    providerStatus: "disabled",
    samples: [],
    matches: []
  }
};

export const emptyAudioScreeningSettings: AudioScreeningSettings = {
  enabled: false,
  host: "",
  timeoutSeconds: 30,
  intensityPercent: 5,
  dynamicByTrackDuration: true,
  referenceDurationSeconds: 300,
  status: "disabled",
  statusMessage: "Optional ACRCloud screening is not configured.",
  credentialsConfigured: false,
  localEngineAvailable: false
};

export function emptyEvidenceMetadata(): EvidenceMetadata {
  return {
    originalFileName: "",
    documentTitle: "",
    provider: "",
    sourceUrl: "",
    retrievalDate: "",
    effectiveDate: "",
    applicableProductionPeriod: "",
    factualNote: "",
    externalTimestamp: "",
    timestampType: "",
    externalReferenceId: "",
    providerVerificationUrl: "",
    referencedHash: "",
    referencedArtifact: "",
    fileExtension: "",
    mimeType: "",
    audioFormat: "",
    audioChannels: null,
    audioSampleRateHz: null,
    audioDurationMilliseconds: null,
    audioBitDepth: null,
    embeddedMetadata: [],
    sunoStudioDetected: false,
    sunoCreatedTimestamp: "",
    sunoCreatedDate: "",
    sunoId: "",
    sunoRawMetadata: ""
  };
}

export function emptyTrackAutomation(): TrackAutomation {
  return {
    finalGenerationIdOrigin: "not_documented",
    finalGenerationOrigin: "not_documented",
    productionEndOrigin: "not_documented",
    downloadExportOrigin: "not_documented",
    finalExportOrigin: "not_documented",
    sunoMetadataDetected: false,
    releaseIdenticalToSunoExport: false,
    byteIdenticalPairs: [],
    consistencyIssues: []
  };
}

export function emptyTrackFields(profile: GlobalProfile = emptyProfile): TrackFields {
  return {
    title: "",
    productionStartDate: "",
    productionEndDate: "",
    sunoModel: "",
    sunoProjectUrl: "",
    sunoProjectVersionId: "",
    sunoFinalGenerationId: "",
    sunoFinalGenerationDate: "",
    sunoFinalGenerationTime: "",
    sunoDownloadExportDate: "",
    sunoPlanAtGeneration: "",
    legacySunoPlanAtCreation: "",
    finalExportDate: "",
    instrumentalTrack: null,
    legacyLyricsSource: "",
    legacyLyricsText: "",
    vocalLyricsPresent: null,
    vocalIntent: null,
    sunoLyricsFieldContent: null,
    sunoContentClassification: null,
    sunoLyricsContentTypes: [],
    sunoLyricsContentSource: null,
    sunoLyricsFieldText: "",
    sunoLyricsOtherContentType: "",
    sunoStylePrompt: "",
    externalAudioUploaded: null,
    externalAudioSource: "",
    externalAudioOwnership: "",
    ownAudioUploaded: null,
    ownAudioSource: "",
    ownAudioOwnership: "",
    codeBasedGeneration: null,
    codeAudioPostProcessed: null,
    codeAudioPostProcessingOperations: [],
    codeAudioPostProcessingNote: "",
    thirdPartySamplesUploaded: null,
    thirdPartySampleSource: "",
    thirdPartySampleOwnership: "",
    humanEditingPerformed: null,
    humanEditingDetails: "",
    postExportEditingPerformed: null,
    postExportEditingDetails: "",
    commercialUseIntended: profile.defaultCommercialUse,
    releaseFilenameDifferenceConfirmed: null,
    sunoExportFilenameDifferenceConfirmed: null,
    sunoTermsEvidenceNotAvailable: null,
    artworkOrigin: "",
    aiImageService: profile.defaultAiImageService,
    humanArtworkProcessOperations: [],
    humanArtworkProcessNotes: "",
    humanArtworkModifications: [],
    customArtworkChange: "",
    depictsRealPerson: null,
    realPersonNotes: "",
    depictsRealEvent: null,
    realEventNotes: "",
    containsTrademark: null,
    trademarkNotes: "",
    disclosureApplied: null,
    disclosureText: profile.disclosureText,
    generativeAiUsed: null,
    audioAiSystem: "",
    aiAssistedAudioElements: null,
    aiGeneratedAudioElements: null,
    realPersonVoiceIntentionallyImitated: null,
    realPersonIdentityIntentionallyRepresented: null,
    realEventRepresentedAsAuthenticRecording: null,
    realLocationInstitutionEventPresentedAsAuthenticAiRecording: null,
    audioDisclosureApplied: null,
    audioDisclosureLocations: [],
    audioDisclosureText: "",
    audioDisclosureReason: "",
    releaseNotes: ""
  };
}
