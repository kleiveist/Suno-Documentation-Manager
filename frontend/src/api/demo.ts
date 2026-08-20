import {
  calculateMissingRequirements,
  calculateProgress,
  evaluateRequirements,
  finalizationGate,
  stepStatuses,
  subscriptionEvidenceRelevance,
  WORKFLOW_ID,
  WORKFLOW_VERSION
} from "../domain/workflow";
import { subscriptionCoverageEnd } from "../domain/subscription";
import { trackLibraryAssignment } from "../domain/track-library";
import {
  emptyEvidenceMetadata,
  emptyAudioScreeningSettings,
  emptyAudioScreeningSummary,
  emptyProfile,
  emptyTimestampSettings,
  emptyTrackAutomation,
  emptyTrackFields,
  type ActionResult,
  type AudioScreeningProviderTestResult,
  type AudioScreeningSecretInput,
  type AudioScreeningSettings,
  type ByteIdenticalPair,
  type ConsistencyIssue,
  type EvidenceItem,
  type EvidenceMetadata,
  type FolderImportExecutionInput,
  type FolderImportProposal,
  type EvidenceRole,
  type ExternalTimestampRecord,
  type ExternalTimestampSummary,
  type FactOrigin,
  type FinalizeOptions,
  type GlobalProfile,
  type GlobalEvidenceItem,
  type OperationProgress,
  type ScanResult,
  type StepId,
  type StepStatus,
  type TimestampProviderKind,
  type TimestampProviderCapabilities,
  type TimestampSettings,
  type TrackCreateInput,
  type TrackDetail,
  type TrackLibraryAssignment,
  type TrackSummary,
  type ValidationResult,
  type WorkflowDefinitionDto,
  type WorkspaceSummary
} from "../domain/types";
import type { DesktopApi } from "./desktop";

const now = (): string => new Date().toISOString();
const clone = <T>(value: T): T => structuredClone(value);
const wait = async (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 140));

async function demoProgress(
  onProgress: ((progress: OperationProgress) => void) | undefined,
  values: OperationProgress[]
): Promise<void> {
  for (const value of values) {
    onProgress?.(value);
    await new Promise((resolve) => setTimeout(resolve, 55));
  }
}

function trackFolderName(title: string): string {
  return title.trim();
}

function canonicalArtworkStem(title: string): string {
  const stem = title.trim().replace(/[^a-z\d]+/giu, "_").replace(/^_+|_+$/gu, "").toLocaleUpperCase("en-US");
  return stem || "TRACK";
}

function managedArtworkName(title: string, role: EvidenceRole, extension: string): string | null {
  const suffix: Partial<Record<EvidenceRole, string>> = {
    artwork_suno_original: "SUNO_ORIGINAL",
    ai_artwork_original: "AI_ORIGINAL",
    ai_artwork_edited: "AI_EDITED",
    human_edited_artwork: "HUMAN_EDITED",
    final_artwork: "FINAL"
  };
  return suffix[role] ? `${canonicalArtworkStem(title)}_${suffix[role]}.${extension}` : null;
}

function trackRelativePath(library: TrackLibraryAssignment, title: string): string {
  const parent = library.section === "album" ? library.albumTitle!.trim() : "Singles";
  return `${parent}/${trackFolderName(title)}`;
}

function evidence(role: EvidenceRole, fileName: string): EvidenceItem {
  const extension = fileName.includes(".") ? fileName.slice(fileName.lastIndexOf(".") + 1).toLocaleLowerCase() : "";
  const audioPair = role === "release_wav" || role === "suno_final_export";
  const hashCharacter = audioPair ? "7" : ((role.length % 15) + 1).toString(16);
  return {
    id: crypto.randomUUID(),
    role,
    fileName,
    relativePath: `evidence/${fileName}`,
    sha256: hashCharacter.repeat(64),
    sizeBytes: 8_476_231,
    importedAt: now(),
    verified: true,
    provenance: "managed_copy",
    metadata: {
      ...emptyEvidenceMetadata(),
      originalFileName: fileName,
      fileExtension: extension,
      mimeType: extension === "wav" ? "audio/wav" : extension === "png" ? "image/png" : "application/octet-stream",
      audioFormat: extension === "wav" ? "WAV" : "",
      audioChannels: extension === "wav" ? 2 : null,
      audioSampleRateHz: extension === "wav" ? 48_000 : null,
      audioDurationMilliseconds: extension === "wav" ? 213_450 : null,
      audioBitDepth: extension === "wav" ? 24 : null
    }
  };
}

function sunoEvidence(fileName: string, createdTimestamp = "2026-07-24T10:12:13Z"): EvidenceItem {
  const item = evidence("suno_final_export", fileName);
  const createdDate = createdTimestamp.slice(0, 10);
  const technicalId = "6c8a40fd-32bf-4c7b-ab59-23579ff95828";
  const raw = `made with suno studio; created=${createdTimestamp}; id=${technicalId}`;
  item.metadata = {
    ...item.metadata!,
    embeddedMetadata: [{ key: "comment", value: raw }],
    sunoStudioDetected: true,
    sunoCreatedTimestamp: createdTimestamp,
    sunoCreatedDate: createdDate,
    sunoId: technicalId,
    sunoRawMetadata: raw
  };
  return item;
}

function makeTrack(
  id: string,
  title: string,
  profile: GlobalProfile,
  complete = false,
  library: TrackLibraryAssignment = { section: "single" }
): TrackDetail {
  const fields: TrackDetail["fields"] = {
    ...emptyTrackFields(profile),
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
    sunoLyricsContentSource: complete ? ("human" as const) : null,
    sunoLyricsFieldText: complete ? "Eigene Lyrics – im Track-Dokument vollständig gespeichert." : "",
    sunoStylePrompt: complete ? "cinematic synthwave, driving bass, wide vocal" : "",
    externalAudioUploaded: false,
    ownAudioUploaded: false,
    codeBasedGeneration: false,
    thirdPartySamplesUploaded: false,
    humanEditingPerformed: complete,
    humanEditingDetails: complete ? "Timing and cuts | EQ | Loudness adjustment" : "",
    postExportEditingPerformed: false,
    artworkOrigin: complete ? ("ai_assisted" as const) : ("" as const),
    aiImageService: complete ? "OpenAI" : "",
    humanArtworkModifications: complete ? ["Typography added", "Color correction"] : [],
    depictsRealPerson: complete ? false : null,
    depictsRealEvent: complete ? false : null,
    containsTrademark: complete ? false : null,
    disclosureApplied: complete,
    disclosureText: "AI-assisted",
    generativeAiUsed: complete ? true : null,
    audioAiSystem: complete ? "Suno" : "",
    aiAssistedAudioElements: complete ? ("yes" as const) : null,
    aiGeneratedAudioElements: complete ? ("yes" as const) : null,
    realPersonVoiceIntentionallyImitated: complete ? ("no" as const) : null,
    realPersonIdentityIntentionallyRepresented: complete ? ("no" as const) : null,
    realEventRepresentedAsAuthenticRecording: complete ? ("no" as const) : null,
    realLocationInstitutionEventPresentedAsAuthenticAiRecording: complete ? ("no" as const) : null,
    audioDisclosureApplied: complete ? ("yes" as const) : null,
    audioDisclosureLocations: complete ? ["Release-Metadaten"] : [],
    audioDisclosureText: complete ? "AI-generated audio" : ""
  };
  const originalArtwork = evidence("ai_artwork_original", managedArtworkName(title, "ai_artwork_original", "png")!);
  const disclosedArtwork: EvidenceItem = {
    ...evidence("ai_artwork_edited", managedArtworkName(title, "ai_artwork_edited", "png")!),
    provenance: "generated_disclosure",
    derivedFromEvidenceId: originalArtwork.id,
    generatorVersion: "local-disclosure-v1",
    generatedDisclosureText: fields.disclosureText
  };
  const finalArtwork: EvidenceItem = {
    ...evidence("final_artwork", managedArtworkName(title, "final_artwork", "jpeg")!),
    sha256: disclosedArtwork.sha256
  };
  const items = complete
    ? [
        evidence("release_wav", `${title}.wav`),
        sunoEvidence(`${title}_SUNO_FINAL.wav`),
        { ...evidence("subscription_payment", "subscription_2026-07.pdf"), provenance: "global_copy" as const, sourceGlobalEvidenceId: "demo-global-subscription", coverageStart: "2026-07-01", coverageEnd: "2026-07-31" },
        originalArtwork,
        disclosedArtwork,
        finalArtwork,
        {
          ...evidence("suno_terms_rights", "suno_terms.pdf"),
          provenance: "global_copy" as const,
          sourceGlobalEvidenceId: "demo-global-terms",
          metadata: {
            ...emptyEvidenceMetadata(),
            originalFileName: "suno_terms.pdf",
            documentTitle: "Suno Terms of Service",
            provider: "Suno, Inc.",
            retrievalDate: "2026-07-18"
          }
        }
      ]
    : [evidence("suno_screenshot", `${title}_SUNO.png`)];
  const release = items.find((item) => item.role === "release_wav");
  const audioScreening = clone(emptyAudioScreeningSummary);
  if (complete && release?.sha256) {
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
      provider: "ACRCloud",
      status: "skipped_not_configured",
      message: "Browser demo: optional ACRCloud screening is not configured and no provider was contacted.",
      sourceEvidenceId: release.id,
      sourceRelativePath: release.relativePath,
      sourceSha256: release.sha256,
      sourceSizeBytes: release.sizeBytes,
      matches: []
    };
  }
  const track: TrackDetail = {
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
      templateVersion: "1.10",
      files: complete ? ["02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md"] : []
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
  refresh(track);
  return track;
}

function refresh(track: TrackDetail): void {
  track.title = track.fields.title;
  track.coverEvidenceId = track.evidence.find((item) =>
    item.role === "final_artwork" && item.verified && Boolean(item.sha256) && !item.verificationError
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

function sameTrackDocumentationProfile(left: GlobalProfile, right: GlobalProfile): boolean {
  return left.artistName === right.artistName
    && left.sunoProfileName === right.sunoProfileName
    && left.sunoHandle === right.sunoHandle
    && left.sunoPlan === right.sunoPlan
    && left.subscriptionStartDate === right.subscriptionStartDate
    && left.defaultCommercialUse === right.defaultCommercialUse
    && left.defaultAiImageService === right.defaultAiImageService
    && left.artworkTransparencyPolicy === right.artworkTransparencyPolicy
    && left.disclosureText === right.disclosureText;
}

function timestampProviderLabel(provider: TimestampProviderKind): string {
  switch (provider) {
    case "free_tsa": return "FreeTSA";
    case "open_timestamps": return "OpenTimestamps";
    case "sigstore_public_tsa": return "Sigstore Public TSA";
    case "custom_rfc3161": return "Custom RFC 3161";
    default: return "Disabled";
  }
}

function timestampProviderUsesRfc3161(provider: TimestampProviderKind): boolean {
  return provider === "free_tsa"
    || provider === "sigstore_public_tsa"
    || provider === "custom_rfc3161";
}

function timestampRecordUsesOpenTimestamps(record: ExternalTimestampRecord | undefined): boolean {
  if (!record) return false;
  return record.providerMetadata?.adapter.toLowerCase().includes("open_timestamps") === true
    || record.providerMetadata?.protocol.toLowerCase().includes("opentimestamps") === true
    || record.provider.toLowerCase() === "opentimestamps";
}

function notRecordedExternalTimestampSummary(): ExternalTimestampSummary {
  return {
    status: "not_recorded",
    message: "External timestamp evidence is optional and is not required for technical finalization.",
    provider: ""
  };
}

function timestampProviderCapabilities(provider: TimestampProviderKind): TimestampProviderCapabilities {
  const rfc3161 = provider === "free_tsa" || provider === "sigstore_public_tsa" || provider === "custom_rfc3161";
  return {
    rfc3161,
    openTimestamps: provider === "open_timestamps",
    requiresAuthentication: provider === "custom_rfc3161",
    supportsSha256: provider !== "disabled",
    supportsOfflineVerification: provider === "open_timestamps" || rfc3161,
    returnsSignedTimestamp: rfc3161,
    externalTrustRootAvailable: false,
    qualificationStatus: "unknown"
  };
}

function configuredTimestampProviderStatus(
  settings: TimestampSettings,
  timestampSecretConfigured = false
): Pick<TimestampSettings, "status" | "statusMessage"> {
  if (!settings.enabled || settings.provider === "disabled") {
    return { status: "disabled", statusMessage: "External timestamp service is disabled." };
  }
  if (settings.provider === "open_timestamps") {
    return {
      status: "ready",
      statusMessage: "OpenTimestamps calendar service is ready. This is not RFC 3161; an initial proof remains ATTACHED pending OpenTimestamps verification or upgrade."
    };
  }
  if (settings.provider !== "custom_rfc3161") {
    if (!settings.custom.caCertificatePath.trim()) {
      return {
        status: "verification_configuration_incomplete",
        statusMessage: "Select an explicit TSA CA trust-anchor file before RFC 3161 responses can be marked VERIFIED."
      };
    }
    return { status: "ready", statusMessage: `${timestampProviderLabel(settings.provider)} RFC 3161 service and its explicit TSA trust anchor are ready.` };
  }
  if (!settings.custom.providerName.trim() || !settings.custom.endpoint.trim()) {
    return {
      status: "configuration_incomplete",
      statusMessage: "Enter a provider name and TSA endpoint for Custom RFC 3161."
    };
  }
  if (settings.custom.authenticationMode === "basic" && !settings.custom.username.trim()) {
    return { status: "authentication_required", statusMessage: "Enter the account name and configure its secret separately." };
  }
  if (["basic", "bearer_token", "api_key"].includes(settings.custom.authenticationMode) && !timestampSecretConfigured) {
    return { status: "authentication_required", statusMessage: "Configure the provider token in secure local settings." };
  }
  if (settings.custom.authenticationMode === "client_certificate" && !settings.custom.clientCertificatePath.trim()) {
    return { status: "authentication_required", statusMessage: "Select a configured client certificate for this provider." };
  }
  if (!settings.custom.caCertificatePath.trim()) {
    return {
      status: "verification_configuration_incomplete",
      statusMessage: "Select an explicit TSA CA trust-anchor file before RFC 3161 responses can be marked VERIFIED."
    };
  }
  return { status: "ready", statusMessage: `${settings.custom.providerName.trim()} and its explicit trust anchor are ready.` };
}

function normalizeTimestampSettings(next: TimestampSettings, timestampSecretConfigured = false): TimestampSettings {
  const custom = { ...emptyTimestampSettings.custom, ...clone(next.custom ?? emptyTimestampSettings.custom) };
  const normalized: TimestampSettings = {
    ...emptyTimestampSettings,
    ...clone(next),
    custom
  };
  const status = configuredTimestampProviderStatus(normalized, timestampSecretConfigured);
  return { ...normalized, ...status };
}

function configuredAudioScreeningStatus(
  settings: AudioScreeningSettings,
  credentialsConfigured = false
): Pick<AudioScreeningSettings, "status" | "statusMessage"> {
  if (!settings.enabled) {
    return {
      status: "disabled",
      statusMessage: "Optional ACRCloud screening is disabled."
    };
  }
  if (!settings.host.trim()) {
    return {
      status: "configuration_invalid",
      statusMessage: "Enter the ACRCloud project host before using the optional provider check."
    };
  }
  if (!credentialsConfigured) {
    return {
      status: "not_configured",
      statusMessage: "Store both ACRCloud access values in local secure settings before using the optional provider check."
    };
  }
  return {
    status: "ready",
    statusMessage: "ACRCloud is configured. Use the explicit test or per-track check; no request runs automatically."
  };
}

function normalizeAudioScreeningSettings(
  next: AudioScreeningSettings,
  credentialsConfigured = false
): AudioScreeningSettings {
  const timeoutCandidate = Number(next.timeoutSeconds);
  const timeoutSeconds = Number.isFinite(timeoutCandidate) && timeoutCandidate > 0
    ? Math.min(Math.trunc(timeoutCandidate), 120)
    : emptyAudioScreeningSettings.timeoutSeconds;
  const normalized: AudioScreeningSettings = {
    ...emptyAudioScreeningSettings,
    ...clone(next),
    host: next.host.trim(),
    timeoutSeconds,
    credentialsConfigured,
    localEngineAvailable: false,
    localEngineVersion: undefined
  };
  return { ...normalized, ...configuredAudioScreeningStatus(normalized, credentialsConfigured) };
}

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
    const isInstruction = (value: (typeof legacy)[number]): boolean => [
      "structure_instructions",
      "sound_instructions",
      "arrangement_instructions"
    ].includes(value);
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

function refreshAutomation(track: TrackDetail): void {
  const previous = track.automation;
  const suno = track.evidence.find((item) =>
    item.role === "suno_final_export" && item.verified && Boolean(item.sha256) && !item.verificationError
      && Boolean(item.metadata?.sunoStudioDetected)
  );
  const createdDate = suno?.metadata?.sunoCreatedDate?.trim() ?? "";
  const sunoId = suno?.metadata?.sunoId?.trim() ?? "";
  const previousCreatedDate = previous.sunoCreatedTimestamp?.slice(0, 10) ?? "";
  const previousSunoId = previous.sunoId ?? "";
  const issues: ConsistencyIssue[] = [];
  const editable = track.status !== "FINALIZED" && track.status !== "SUPERSEDED";
  const finalGeneration = editable
    ? reconcileAutomaticDate(
        track.fields.sunoFinalGenerationDate,
        previous.finalGenerationOrigin,
        createdDate,
        previousCreatedDate
      )
    : { value: track.fields.sunoFinalGenerationDate, origin: previous.finalGenerationOrigin };
  const finalGenerationId = editable
    ? reconcileAutomaticGenerationId(
        track.fields.sunoFinalGenerationId,
        previous.finalGenerationIdOrigin,
        sunoId,
        previousSunoId
      )
    : { value: track.fields.sunoFinalGenerationId, origin: previous.finalGenerationIdOrigin };
  const productionEnd = editable
    ? reconcileAutomaticDate(
        track.fields.productionEndDate,
        previous.productionEndOrigin,
        createdDate,
        previousCreatedDate
      )
    : { value: track.fields.productionEndDate, origin: previous.productionEndOrigin };
  const downloadExport = editable
    ? reconcileAutomaticDate(
        track.fields.sunoDownloadExportDate,
        previous.downloadExportOrigin,
        createdDate,
        previousCreatedDate
      )
    : { value: track.fields.sunoDownloadExportDate, origin: previous.downloadExportOrigin };
  const finalExport = editable
    ? reconcileAutomaticDate(
        track.fields.finalExportDate,
        previous.finalExportOrigin,
        track.fields.postExportEditingPerformed === false ? createdDate : "",
        previousCreatedDate
      )
    : { value: track.fields.finalExportDate, origin: previous.finalExportOrigin };

  track.fields.sunoFinalGenerationDate = finalGeneration.value;
  track.fields.sunoFinalGenerationId = finalGenerationId.value;
  track.fields.productionEndDate = productionEnd.value;
  track.fields.sunoDownloadExportDate = downloadExport.value;
  track.fields.finalExportDate = finalExport.value;

  const pairs: ByteIdenticalPair[] = [];
  const verified = track.evidence.filter((item) => item.verified && Boolean(item.sha256) && !item.verificationError);
  for (let index = 0; index < verified.length; index += 1) {
    for (let rightIndex = index + 1; rightIndex < verified.length; rightIndex += 1) {
      const left = verified[index];
      const right = verified[rightIndex];
      if (left.sha256 !== right.sha256) continue;
      pairs.push({ leftEvidenceId: left.id, leftRole: left.role, rightEvidenceId: right.id, rightRole: right.role, sha256: left.sha256! });
    }
  }
  const releaseIdenticalToSunoExport = pairs.some((pair) =>
    (pair.leftRole === "suno_final_export" && pair.rightRole === "release_wav")
    || (pair.leftRole === "release_wav" && pair.rightRole === "suno_final_export")
  );
  track.automation = {
    finalGenerationIdOrigin: finalGenerationId.origin,
    finalGenerationOrigin: finalGeneration.origin,
    productionEndOrigin: productionEnd.origin,
    downloadExportOrigin: downloadExport.origin,
    finalExportOrigin: finalExport.origin,
    sunoMetadataDetected: Boolean(suno?.metadata?.sunoStudioDetected),
    sunoCreatedTimestamp: suno?.metadata?.sunoCreatedTimestamp || undefined,
    sunoId: suno?.metadata?.sunoId || undefined,
    releaseIdenticalToSunoExport,
    byteIdenticalPairs: pairs,
    consistencyIssues: issues
  };
}

export function createDemoApi(): DesktopApi {
  let workspace: WorkspaceSummary | null = null;
  let profile: GlobalProfile = {
    ...emptyProfile,
    artistName: "GRAV0ID",
    sunoProfileName: "Grav0id Studio",
    sunoHandle: "@grav0id",
    sunoPlan: "Premier",
    subscriptionStartDate: "2026-01-01",
    defaultAiImageService: "OpenAI"
  };
  const tracks = new Map<string, TrackDetail>();
  const albums = new Map<string, string>();
  let globalEvidence: GlobalEvidenceItem[] = [{ ...evidence("subscription_payment", "subscription_2026-07.pdf"), coverageStart: "2026-07-01", coverageEnd: "2026-07-31" }];
  let timestampSettings: TimestampSettings = clone(emptyTimestampSettings);
  // The browser demo deliberately stores only this non-sensitive state bit;
  // native builds keep the actual credential outside workspace data.
  let timestampSecretConfigured = false;
  let audioScreeningSettings: AudioScreeningSettings = clone(emptyAudioScreeningSettings);
  // As with timestamp credentials, the demo retains only configuration state.
  // It never receives, stores, sends or fabricates provider credentials.
  let audioAccessKeyConfigured = false;
  let audioAccessSecretConfigured = false;

  const attachGlobalToTrack = (track: TrackDetail, item: GlobalEvidenceItem): void => {
    if (track.evidence.some((entry) => entry.sourceGlobalEvidenceId === item.id)) return;
    if (item.role === "subscription_payment" && !subscriptionEvidenceRelevance(item, track.fields).relevant) {
      throw new Error("Der ausgewählte Abo-Nachweis überschneidet weder den Produktionszeitraum noch deckt er die Finalgeneration ab.");
    }
    track.evidence.push({
      ...clone(item),
      id: crypto.randomUUID(),
      sourceGlobalEvidenceId: item.id,
      provenance: "global_copy",
      relativePath: `04_LICENSES/${item.role === "suno_terms_rights" ? "suno_terms" : "subscription"}_${item.fileName}`
    });
    if (item.role === "suno_terms_rights") track.fields.sunoTermsEvidenceNotAvailable = false;
    refresh(track);
  };

  const albumKey = (title: string): string => title.trim().normalize("NFKC").toLocaleLowerCase("de-DE");
  const rememberAlbum = (library: TrackLibraryAssignment): void => {
    if (library.section === "album" && library.albumTitle) {
      albums.set(albumKey(library.albumTitle), library.albumTitle.trim());
    }
  };
  const albumList = (): string[] => [...albums.values()]
    .sort((left, right) => left.localeCompare(right, "de", { sensitivity: "base", numeric: true }));

  const get = (trackId: string): TrackDetail => {
    const track = tracks.get(trackId);
    if (!track) throw new Error("Der Track wurde im aktuellen Workspace nicht gefunden.");
    return track;
  };
  const mutableTrack = (trackId: string): TrackDetail => {
    const track = get(trackId);
    if (track.status === "FINALIZED") {
      throw new Error("Der Track ist finalisiert. Lege vor Änderungen eine neue Revision an.");
    }
    if (track.status === "SUPERSEDED") {
      throw new Error("Der Track wurde durch eine neuere Revision ersetzt und kann nicht mehr geändert werden.");
    }
    return track;
  };
  const result = (track: TrackDetail, message: string): ActionResult => ({ message, track: clone(track) });

  const attachConfiguredTimestamp = (track: TrackDetail): TrackDetail => {
    if (track.status !== "FINALIZED" || !track.certificate.valid || !track.certificate.certificateId) {
      throw new Error("Ein externer Zeitstempel kann erst nach der technischen Finalisierung angehängt werden.");
    }
    const summaryStatus = track.externalTimestampSummary?.status ?? "not_recorded";
    const currentRecord = track.externalTimestampSummary?.recordId
      ? track.externalTimestamps.find((record) => record.id === track.externalTimestampSummary?.recordId)
        ?? track.externalTimestamps.at(-1)
      : track.externalTimestamps.at(-1);
    if (summaryStatus === "verified"
      || (summaryStatus === "attached"
        && !(timestampRecordUsesOpenTimestamps(currentRecord)
          && timestampProviderUsesRfc3161(timestampSettings.provider)))) return track;
    if (timestampSettings.status !== "ready") {
      track.externalTimestampSummary = {
        status: timestampSettings.status === "authentication_required" ? "authentication_failed" : "provider_unavailable",
        message: timestampSettings.statusMessage,
        provider: timestampProviderLabel(timestampSettings.provider)
      };
      return track;
    }
    const anchor = track.finalizationAnchors.find((item) => item.artifact === "evidence_manifest");
    if (!anchor) {
      track.externalTimestampSummary = {
        status: "anchor_mismatch",
        message: "The finalized evidence-manifest anchor is not available.",
        provider: timestampProviderLabel(timestampSettings.provider)
      };
      return track;
    }

    const timestampedAt = now();
    const openTimestamps = timestampSettings.provider === "open_timestamps";
    const verificationMessage = openTimestamps
      ? "Detached proof is locally bound to the requested SHA-256; explicit OpenTimestamps verification or upgrade is pending."
      : "Structural and digest checks completed; provider signature and trust verification are not asserted.";
    track.externalTimestampSummary = {
      status: "requesting",
      message: "External timestamp request is being prepared.",
      provider: timestampProviderLabel(timestampSettings.provider)
    };
    const id = crypto.randomUUID();
    const provider = timestampProviderLabel(timestampSettings.provider);
    const evidenceFileName = openTimestamps
      ? "TIMESTAMP_EVIDENCE.ots"
      : timestampSettings.provider === "custom_rfc3161" || timestampSettings.provider === "free_tsa"
        ? "TIMESTAMP_RESPONSE.tsr"
        : "TIMESTAMP_RESPONSE.json";
    track.externalTimestamps.push({
      id,
      certificateId: track.certificate.certificateId,
      provider,
      timestampType: "external_integrity_timestamp",
      timestampValue: openTimestamps ? "" : timestampedAt,
      referencedArtifact: "evidence_manifest",
      referencedArtifactPath: anchor.relativePath,
      referencedSha256: anchor.sha256,
      actualSha256: anchor.sha256,
      referencedHashMatch: true,
      externalReferenceId: `demo-${id.slice(0, 8)}`,
      providerVerificationUrl: openTimestamps ? "https://a.pool.opentimestamps.org/digest" : "",
      note: openTimestamps
        ? "OpenTimestamps detached proof archived; Bitcoin anchoring remains pending verification or upgrade."
        : "",
      evidenceFileName,
      evidenceSha256: "f".repeat(64),
      importedAt: timestampedAt,
      provenance: "Automatic provider response; structural and digest checks",
      providerMetadata: {
        adapter: openTimestamps ? "open_timestamps" : `demo-${timestampSettings.provider}`,
        protocol: openTimestamps
          ? "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade"
          : "RFC 3161",
        requestAlgorithm: "SHA-256",
        responseFormat: timestampSettings.provider === "open_timestamps" ? ".ots proof" : "RFC 3161 TimeStampResp",
        providerEndpointIdentifier: timestampSettings.provider === "custom_rfc3161"
          ? timestampSettings.custom.endpoint
          : provider,
        providerResponseFileName: evidenceFileName,
        providerResponseSha256: "f".repeat(64),
        referencedRevisionId: `demo-finalized-snapshot-${track.id}`,
        issuer: "",
        certificateSubject: "",
        certificateSerialNumber: "",
        policyOid: timestampSettings.provider === "custom_rfc3161" ? timestampSettings.custom.policyOid : "",
        responseStructureValid: openTimestamps ? null : true,
        providerDigestMatch: true,
        signatureVerified: null,
        trustChainVerified: null,
        verificationResult: "attached",
        verificationMessage,
        verificationTimestamp: timestampedAt
      },
      recordRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD.json`,
      markdownRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.md`,
      pdfRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.pdf`,
      hashListRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD_SHA256.txt`,
      integrityVerified: true,
      integrityIssues: []
    });
    track.externalTimestampSummary = {
      // A stored RFC-3161 response with a matching digest is ATTACHED until a
      // provider-specific signature/trust verification is actually available.
      status: "attached",
      message: openTimestamps
        ? "OpenTimestamps detached proof attached; later verification or upgrade is required."
        : "External timestamp response attached; structural and digest checks completed.",
      provider,
      recordId: id,
      updatedAt: timestampedAt
    };
    return track;
  };

  const releaseAudio = (track: TrackDetail): EvidenceItem | undefined => track.evidence.find((item) =>
    item.role === "release_wav" && item.verified && Boolean(item.sha256) && !item.verificationError
  );

  const markAudioScreeningStale = (track: TrackDetail): void => {
    const source = releaseAudio(track);
    const local = track.audioScreening.local;
    if (local.status !== "not_run") {
      track.audioScreening.local = {
        ...local,
        status: "stale",
        message: source
          ? "The authoritative release audio changed; generate a new local fingerprint for the current file."
          : "The authoritative release audio is no longer available; the prior local fingerprint is stale."
      };
    }
    const external = track.audioScreening.external;
    if (external.status !== "not_run") {
      track.audioScreening.external = {
        ...external,
        status: "stale",
        message: "The authoritative release audio changed; the prior external result is no longer current.",
        responseRelativePath: undefined,
        responseSha256: undefined
      };
    }
  };

  return {
    mode: "demo",
    async getWorkflow(): Promise<WorkflowDefinitionDto> {
      return {
        schemaVersion: 1,
        id: WORKFLOW_ID,
        version: WORKFLOW_VERSION,
        name: "Suno Track Documentation",
        steps: [
          { id: "track", number: "01", label: "Track", description: "Titel und Produktionszeitraum", required: true },
          { id: "source", number: "02", label: "Quelle", description: "Audioquellen und Rechtezuordnung", required: true },
          { id: "suno", number: "03", label: "Suno", description: "Projekt, Modell und Erstellungstarif", required: true },
          { id: "human_work", number: "04", label: "Menschliche Arbeit", description: "Lyrics und bestätigte Bearbeitungen", required: true },
          { id: "artwork", number: "05", label: "Artwork", description: "Entstehung und Content-Check", required: true },
          { id: "ai_transparency", number: "06", label: "KI-Transparenz", description: "Projektinterne Disclosure-Policy", required: true },
          { id: "release", number: "07", label: "Release", description: "Letzte Bearbeitung und Release-Dateien", required: true },
          { id: "evidence_licenses", number: "08", label: "Evidence & Lizenzen", description: "Nachweise vollständig zuordnen", required: true },
          { id: "integrity", number: "09", label: "Integrität", description: "Dokumente, SHA-256 und Verifikation", required: true },
          { id: "finalize", number: "10", label: "Finalisieren", description: "Gate prüfen und Zertifikat erzeugen", required: true }
        ]
      };
    },
    async openWorkspace() {
      await wait();
      workspace = { id: "demo-workspace", name: "Music Projects", path: "Beispiel/Workspace", trackCount: 2, lastScannedAt: now() };
      if (tracks.size === 0) {
        const demoAlbum = { section: "album", albumTitle: "Event Horizon" } as const;
        rememberAlbum(demoAlbum);
        tracks.set("gravity", makeTrack("gravity", "Gravity", profile, true, demoAlbum));
        tracks.set("cosmic-pulse", makeTrack("cosmic-pulse", "Cosmic Pulse", profile));
      }
      return clone(workspace);
    },
    async createWorkspace() {
      return this.openWorkspace();
    },
    async restoreWorkspace(_path: string) {
      const opened = await this.openWorkspace();
      if (!opened) throw new Error("Demo-Workspace konnte nicht geöffnet werden.");
      return opened;
    },
    async scanWorkspace(): Promise<ScanResult> {
      await wait();
      if (!workspace) throw new Error("Öffne zuerst einen Workspace.");
      workspace.lastScannedAt = now();
      return { discovered: tracks.size, indexed: tracks.size, unchanged: tracks.size, warnings: [] };
    },
    async getProfile() {
      await wait();
      return clone(profile);
    },
    async updateProfile(next) {
      await wait();
      profile = clone(next);
      for (const track of tracks.values()) {
        if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
        if (sameTrackDocumentationProfile(track.profileSnapshot, profile)) continue;
        track.profileSnapshot = clone(profile);
        track.documents.current = false;
        refresh(track);
      }
      return clone(profile);
    },
    async getTimestampSettings() {
      await wait();
      return clone(timestampSettings);
    },
    async updateTimestampSettings(next) {
      await wait();
      timestampSettings = normalizeTimestampSettings(next, timestampSecretConfigured);
      return clone(timestampSettings);
    },
    async updateTimestampSecret(secret) {
      await wait();
      timestampSecretConfigured = Boolean(secret?.trim());
      const status = configuredTimestampProviderStatus(timestampSettings, timestampSecretConfigured);
      timestampSettings = { ...timestampSettings, ...status };
    },
    async testTimestampProvider() {
      await wait();
      const status = configuredTimestampProviderStatus(timestampSettings, timestampSecretConfigured);
      const testedAt = now();
      timestampSettings = {
        ...timestampSettings,
        ...status,
        lastTestedAt: testedAt
      };
      return {
        provider: timestampSettings.provider,
        status: timestampSettings.status,
        message: timestampSettings.status === "ready"
          ? timestampSettings.provider === "open_timestamps"
            ? "OpenTimestamps calendar reachable. This is not RFC 3161; proof verification or upgrade remains pending."
            : "Provider reachable. RFC 3161 timestamp service ready."
          : timestampSettings.statusMessage,
        testedAt,
        capabilities: timestampProviderCapabilities(timestampSettings.provider)
      };
    },
    async getAudioScreeningSettings() {
      await wait();
      return clone(audioScreeningSettings);
    },
    async updateAudioScreeningSettings(next) {
      await wait();
      const credentialsConfigured = audioAccessKeyConfigured && audioAccessSecretConfigured;
      audioScreeningSettings = normalizeAudioScreeningSettings(next, credentialsConfigured);
      return clone(audioScreeningSettings);
    },
    async updateAudioScreeningSecret(input: AudioScreeningSecretInput) {
      await wait();
      if (typeof input.accessKey === "string" && input.accessKey.trim()) audioAccessKeyConfigured = true;
      if (typeof input.accessSecret === "string" && input.accessSecret.trim()) audioAccessSecretConfigured = true;
      const credentialsConfigured = audioAccessKeyConfigured && audioAccessSecretConfigured;
      audioScreeningSettings = {
        ...audioScreeningSettings,
        credentialsConfigured,
        ...configuredAudioScreeningStatus(audioScreeningSettings, credentialsConfigured)
      };
    },
    async testAudioScreeningProvider(): Promise<AudioScreeningProviderTestResult> {
      await wait();
      const credentialsConfigured = audioAccessKeyConfigured && audioAccessSecretConfigured;
      const configured = configuredAudioScreeningStatus(audioScreeningSettings, credentialsConfigured);
      const testedAt = now();
      const unavailable = configured.status === "ready";
      const status = unavailable ? "provider_unavailable" : configured.status;
      const message = unavailable
        ? "Browser demo: no ACRCloud connection is made. Test the configured provider in the desktop app."
        : configured.statusMessage;
      audioScreeningSettings = {
        ...audioScreeningSettings,
        credentialsConfigured,
        status,
        statusMessage: message,
        lastTestedAt: testedAt
      };
      return { status, message, testedAt };
    },
    async listGlobalEvidence() {
      await wait();
      return clone(globalEvidence);
    },
    async importGlobalEvidence(role, coverageStart, billingCycle) {
      await wait();
      const coverageEnd = coverageStart && billingCycle
        ? subscriptionCoverageEnd(coverageStart, billingCycle) ?? undefined
        : undefined;
      const item = { ...evidence(role, `subscription_${new Date().toISOString().slice(0, 7)}.pdf`), coverageStart, coverageEnd };
      globalEvidence.push(item);
      return clone(item);
    },
    async importGlobalTermsEvidence(metadata) {
      await wait();
      const normalized: EvidenceMetadata = {
        ...emptyEvidenceMetadata(),
        ...clone(metadata),
        originalFileName: "suno_terms.pdf"
      };
      if (!normalized.documentTitle.trim() || !normalized.provider.trim() || !normalized.retrievalDate.trim()) {
        throw new Error("Dokumenttitel, Anbieter und Abrufdatum sind für den Terms-Nachweis erforderlich.");
      }
      const item: GlobalEvidenceItem = {
        ...evidence("suno_terms_rights", "suno_terms.pdf"),
        relativePath: ".suno-doc/global-evidence/suno_terms.pdf",
        metadata: normalized
      };
      globalEvidence.push(item);
      for (const track of tracks.values()) {
        if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
        attachGlobalToTrack(track, item);
      }
      return clone(item);
    },
    async updateGlobalTermsEvidenceMetadata(evidenceId, metadata) {
      await wait();
      const item = globalEvidence.find((entry) => entry.id === evidenceId && entry.role === "suno_terms_rights");
      if (!item) throw new Error("Der globale Terms-Nachweis wurde nicht gefunden.");
      const normalized: EvidenceMetadata = {
        ...emptyEvidenceMetadata(),
        ...item.metadata,
        ...clone(metadata),
        originalFileName: item.metadata?.originalFileName || item.fileName
      };
      if (!normalized.documentTitle.trim() || !normalized.provider.trim() || !normalized.retrievalDate.trim()) {
        throw new Error("Dokumenttitel, Anbieter und Abrufdatum sind für den Terms-Nachweis erforderlich.");
      }
      item.metadata = normalized;
      for (const track of tracks.values()) {
        if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
        let changed = false;
        for (const copy of track.evidence.filter((entry) => entry.sourceGlobalEvidenceId === evidenceId && entry.provenance === "global_copy")) {
          copy.metadata = clone(normalized);
          changed = true;
        }
        if (changed) {
          track.documents.current = false;
          refresh(track);
        }
      }
      return clone(item);
    },
    async removeGlobalEvidence(evidenceId) {
      await wait();
      globalEvidence = globalEvidence.filter((item) => item.id !== evidenceId);
    },
    async attachGlobalEvidence(trackId, evidenceId) {
      await wait();
      const track = mutableTrack(trackId);
      const item = globalEvidence.find((entry) => entry.id === evidenceId);
      if (!item) throw new Error("Der globale Nachweis wurde nicht gefunden.");
      attachGlobalToTrack(track, item);
      return clone(track);
    },
    async listTracks(): Promise<TrackSummary[]> {
      await wait();
      return [...tracks.values()].map(({ fields: _fields, steps: _steps, evidence: _evidence, documents: _documents, integrity: _integrity, certificate: _certificate, ...summary }) => clone(summary));
    },
    async listAlbums(): Promise<string[]> {
      await wait();
      return clone(albumList());
    },
    async createAlbum(title): Promise<string[]> {
      await wait();
      const normalized = trackLibraryAssignment("album", title);
      if (!normalized?.albumTitle) throw new Error("Der Albumtitel ist ungültig.");
      const key = albumKey(normalized.albumTitle);
      if (albums.has(key)) throw new Error(`Ein Albumordner mit diesem Namen existiert bereits: ${albums.get(key)}`);
      rememberAlbum(normalized);
      return clone(albumList());
    },
    async createTrack(input: TrackCreateInput) {
      await wait();
      const id = crypto.randomUUID();
      const library = trackLibraryAssignment(input.library.section, input.library.albumTitle ?? "");
      if (!library) throw new Error("Für einen Album-Track ist ein Albumtitel erforderlich.");
      rememberAlbum(library);
      const track = makeTrack(id, input.title.trim(), profile, false, library);
      track.fields.productionStartDate = input.productionStartDate;
      track.fields.commercialUseIntended = input.commercialUseIntended;
      for (const item of globalEvidence.filter((entry) => entry.role === "suno_terms_rights")) {
        attachGlobalToTrack(track, item);
      }
      refresh(track);
      tracks.set(id, track);
      if (workspace) workspace.trackCount = tracks.size;
      return clone(track);
    },
    async scanImportFolder(): Promise<FolderImportProposal | null> {
      await wait();
      return null;
    },
    async executeFolderImport(_input: FolderImportExecutionInput): Promise<TrackDetail[]> {
      throw new Error("Der Ordner-Import ist nur in der Desktop-App verfügbar.");
    },
    async loadTrack(trackId: string) {
      await wait();
      return clone(get(trackId));
    },
    async loadTrackCover(trackId: string) {
      await wait();
      const track = get(trackId);
      const item = track.evidence.find((entry) =>
        entry.role === "final_artwork" && entry.verified && Boolean(entry.sha256) && !entry.verificationError
      );
      if (!item) return null;
      return {
        evidenceId: item.id,
        dataUrl: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAGUlEQVR42mNkYPj/n4GBgYGJAQoAHgQCAWNA+rMAAAAASUVORK5CYII="
      };
    },
    async updateTrackLibrary(trackId, input) {
      await wait();
      const track = get(trackId);
      const library = trackLibraryAssignment(input.section, input.albumTitle ?? "");
      if (!library) throw new Error("Für einen Album-Track ist ein Albumtitel erforderlich.");
      rememberAlbum(library);
      track.library = library;
      track.relativePath = trackRelativePath(library, track.title);
      return clone(track);
    },
    async renameAlbum(oldTitle, newTitle) {
      await wait();
      const normalized = trackLibraryAssignment("album", newTitle);
      if (!normalized?.albumTitle) throw new Error("Der neue Albumtitel ist ungültig.");
      const oldKey = albumKey(oldTitle);
      const newKey = albumKey(normalized.albumTitle);
      const existingTitle = albums.get(oldKey);
      if (!existingTitle) throw new Error(`Album nicht gefunden: ${oldTitle}`);
      if (oldKey !== newKey && albums.has(newKey)) {
        throw new Error(`Ein Albumordner mit diesem Namen existiert bereits: ${albums.get(newKey)}`);
      }
      const matching = [...tracks.values()].filter((track) =>
        track.library.section === "album" && albumKey(track.library.albumTitle ?? "") === oldKey
      );
      for (const track of matching) {
        track.library = clone(normalized);
        track.relativePath = trackRelativePath(normalized, track.title);
      }
      albums.delete(oldKey);
      rememberAlbum(normalized);
      return [...tracks.values()].map(({ fields: _fields, steps: _steps, evidence: _evidence, documents: _documents, integrity: _integrity, certificate: _certificate, ...summary }) => clone(summary));
    },
    async updateTrack(trackId, patch) {
      await wait();
      const track = mutableTrack(trackId);
      const previousTitle = track.fields.title;
      track.fields = { ...track.fields, ...clone(patch) };
      normalizeCanonicalSunoSemantics(track.fields);
      if (patch.title !== undefined) {
        track.relativePath = trackRelativePath(track.library, patch.title);
        if (patch.title !== previousTitle) {
          for (const item of track.evidence.filter((entry) =>
            ["release_wav", "release_mp3", "release_mp4"].includes(entry.role)
          )) {
            const extension = item.fileName.includes(".") ? item.fileName.slice(item.fileName.lastIndexOf(".")) : "";
            item.fileName = `${patch.title}${extension}`;
            item.relativePath = `01_RELEASE/${item.fileName}`;
          }
        }
      }
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async adoptLegacyProfile(trackId) {
      await wait();
      const track = mutableTrack(trackId);
      track.profileSnapshot = clone(profile);
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async addDeviation(trackId, description, blocking) {
      await wait();
      const track = mutableTrack(trackId);
      track.blockingDeviations ??= [];
      track.blockingDeviations.push({ id: crypto.randomUUID(), title: blocking ? "Blockierende Abweichung" : "Hinweis", description, blocking, resolved: false, createdAt: now() });
      refresh(track);
      return clone(track);
    },
    async resolveDeviation(trackId, deviationId) {
      await wait();
      const track = mutableTrack(trackId);
      const deviation = track.blockingDeviations?.find((item) => item.id === deviationId);
      if (deviation) Object.assign(deviation, { resolved: true, resolvedAt: now() });
      refresh(track);
      return clone(track);
    },
    async removeDeviation(trackId, deviationId) {
      await wait();
      const track = mutableTrack(trackId);
      track.blockingDeviations = track.blockingDeviations?.filter((item) => item.id !== deviationId);
      refresh(track);
      return clone(track);
    },
    async setStepStatus(trackId, stepId: StepId, status: StepStatus, naReason?: string) {
      await wait();
      const track = mutableTrack(trackId);
      const current = track.steps.find((step) => step.id === stepId);
      if (current) Object.assign(current, { status, naReason, updatedAt: now() });
      else track.steps.push({ id: stepId, status, naReason, updatedAt: now() });
      refresh(track);
      return clone(track);
    },
    async importEvidence(trackId, role, replaceEvidenceId) {
      await wait();
      const track = mutableTrack(trackId);
      if (!replaceEvidenceId && ["release_wav", "suno_final_export", "final_artwork"].includes(role) && track.evidence.some((item) => item.role === role)) {
        throw new Error(`Die Rolle ${role} ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum Ersetzen.`);
      }
      const extension = role.includes("artwork") || role === "suno_screenshot" || role === "final_artwork"
        ? "png"
        : role === "release_wav" || role === "suno_final_export"
          ? "wav"
          : role === "source_code_file"
            ? "py"
          : role === "code_generated_audio_file"
            ? "wav"
          : role.includes("subscription")
            ? "pdf"
            : "zip";
      const next = role === "suno_final_export"
        ? sunoEvidence(`${role}.${extension}`, "2026-08-17T06:38:06Z")
        : evidence(role, managedArtworkName(track.title, role, extension) ?? `${role}.${extension}`);
      const replaceIndex = replaceEvidenceId
        ? track.evidence.findIndex((item) => item.id === replaceEvidenceId && item.role === role)
        : -1;
      if (replaceEvidenceId && replaceIndex < 0) throw new Error("Die zu ersetzende Evidence wurde nicht gefunden.");
      if (replaceIndex >= 0) track.evidence[replaceIndex] = { ...next, id: replaceEvidenceId! };
      else track.evidence.push(next);
      if (role === "release_wav") markAudioScreeningStale(track);
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async removeEvidence(trackId, evidenceId) {
      await wait();
      const track = mutableTrack(trackId);
      const removed = track.evidence.find((item) => item.id === evidenceId);
      track.evidence = track.evidence.filter((item) => item.id !== evidenceId);
      if (removed?.role === "release_wav") markAudioScreeningStale(track);
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async previewEvidence(trackId, evidenceId) {
      await wait();
      const item = get(trackId).evidence.find((entry) => entry.id === evidenceId);
      if (!item) throw new Error("Die Evidence wurde nicht gefunden.");
      const isImage = /\.(png|jpe?g|webp)$/i.test(item.fileName);
      return {
        evidenceId: item.id,
        role: item.role,
        fileName: item.fileName,
        relativePath: item.relativePath,
        sizeBytes: item.sizeBytes,
        mimeType: isImage ? "image/png" : undefined,
        dataUrl: isImage
          ? "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
          : undefined,
        message: isImage ? undefined : "Für diesen Dateityp ist in der Browser-Demo keine Vorschau verfügbar."
      };
    },
    async verifyEvidence(trackId, evidenceId) {
      await wait();
      const track = get(trackId);
      for (const item of track.evidence) {
        if (!evidenceId || item.id === evidenceId) item.verified = true;
      }
      refresh(track);
      return clone(track);
    },
    async previewDocumentGeneration(trackId) {
      await wait();
      get(trackId);
      return { files: ["02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md"], collisions: [], adoptionRequired: false };
    },
    async generateDocuments(trackId, _adoptExisting, onProgress) {
      const track = mutableTrack(trackId);
      const documentFiles = ["02_SUNO/suno_project.txt", "02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md", "04_LICENSES/suno_account_and_license.md", "04_LICENSES/openai_image_generation.md", "05_ARTWORK/artwork_process.md"];
      await demoProgress(onProgress, [
        { stage: "preparing_documents", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: documentFiles.length },
        { stage: "rendering_documents", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: documentFiles.length },
        ...documentFiles.map((currentFile, index) => ({ stage: "writing_documents", processedBytes: 0, totalBytes: 0, processedFiles: index + 1, totalFiles: documentFiles.length, currentFile })),
        { stage: "complete", processedBytes: 0, totalBytes: 0, processedFiles: documentFiles.length, totalFiles: documentFiles.length }
      ]);
      track.documents = {
        generated: true,
        current: true,
        generatedAt: now(),
        templateVersion: "1.10",
        files: documentFiles
      };
      track.integrity.generated = false;
      track.integrity.verified = false;
      refresh(track);
      return result(track, "8 Dokumente wurden deterministisch erzeugt.");
    },
    async generateArtworkDisclosure(trackId, disclosureText) {
      await wait();
      const track = mutableTrack(trackId);
      track.fields.disclosureApplied = true;
      if (disclosureText) track.fields.disclosureText = disclosureText;
      const source = [...track.evidence].reverse().find((item) => item.role === "ai_artwork_original" && item.verified);
      if (!source) throw new Error("Importiere zuerst das unveränderte KI-Artwork.");
      if (!track.evidence.some((item) => item.role === "ai_artwork_edited")) {
        track.evidence.push({
          ...evidence("ai_artwork_edited", managedArtworkName(track.title, "ai_artwork_edited", "png")!),
          provenance: "generated_disclosure",
          derivedFromEvidenceId: source.id,
          generatorVersion: "local-disclosure-v1",
          generatedDisclosureText: track.fields.disclosureText.trim()
        });
      }
      track.documents.current = false;
      refresh(track);
      return result(track, "Der sichtbare KI-Hinweis wurde lokal auf einer neuen Artwork-Version angewendet.");
    },
    async calculateHashes(trackId, onProgress) {
      const track = mutableTrack(trackId);
      if (!track.documents.current) throw new Error("Erzeuge zuerst die aktuellen Dokumente.");
      const totalFiles = track.evidence.length + track.documents.files.length;
      const totalBytes = Math.max(totalFiles, 1) * 8_476_231;
      await demoProgress(onProgress, [
        { stage: "discovering_files", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: 0 },
        { stage: "hashing", processedBytes: Math.round(totalBytes * .25), totalBytes, processedFiles: Math.floor(totalFiles * .25), totalFiles, currentFile: "01_RELEASE/demo.wav" },
        { stage: "hashing", processedBytes: Math.round(totalBytes * .7), totalBytes, processedFiles: Math.floor(totalFiles * .7), totalFiles, currentFile: "02_SUNO/demo.zip" },
        { stage: "writing_hash_list", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles, currentFile: "03_DOCUMENTATION/SHA256SUMS.txt" },
        { stage: "verifying", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles },
        { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
      ]);
      track.integrity = { generated: true, verified: false, fileCount: track.evidence.length + track.documents.files.length, verifiedCount: 0, generatedAt: now(), mismatchFiles: [] };
      refresh(track);
      return result(track, `${track.integrity.fileCount} Dateien wurden gehasht.`);
    },
    async verifyHashes(trackId, onProgress) {
      const track = get(trackId);
      if (!track.integrity.generated) throw new Error("Erzeuge zuerst SHA-256-Prüfsummen.");
      const totalFiles = track.integrity.fileCount;
      const totalBytes = Math.max(totalFiles, 1) * 8_476_231;
      await demoProgress(onProgress, [
        { stage: "reading_hash_list", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
        { stage: "verifying", processedBytes: Math.round(totalBytes * .35), totalBytes, processedFiles: Math.floor(totalFiles * .35), totalFiles, currentFile: "01_RELEASE/demo.wav" },
        { stage: "verifying", processedBytes: Math.round(totalBytes * .8), totalBytes, processedFiles: Math.floor(totalFiles * .8), totalFiles, currentFile: "02_SUNO/demo.zip" },
        { stage: "comparing_hashes", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles },
        { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
      ]);
      track.integrity.verified = true;
      track.integrity.verifiedCount = track.integrity.fileCount;
      track.integrity.verifiedAt = now();
      track.integrity.mismatchFiles = [];
      refresh(track);
      return result(track, `${track.integrity.verifiedCount} von ${track.integrity.fileCount} Dateien erfolgreich verifiziert.`);
    },
    async runLocalAudioScreening(trackId, onProgress) {
      const track = mutableTrack(trackId);
      const source = releaseAudio(track);
      if (!source?.sha256) throw new Error("Importiere zuerst die autoritative finale Release-Audiodatei.");
      await demoProgress(onProgress, [
        { stage: "preparing_audio", processedBytes: 0, totalBytes: source.sizeBytes, processedFiles: 0, totalFiles: 1, currentFile: source.relativePath },
        { stage: "fingerprinting_audio", processedBytes: Math.round(source.sizeBytes * .65), totalBytes: source.sizeBytes, processedFiles: 0, totalFiles: 1, currentFile: source.relativePath },
        { stage: "fingerprint_complete", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1, currentFile: source.relativePath },
        { stage: "saving_screening_result", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1, currentFile: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json" },
        { stage: "complete", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1 }
      ]);
      track.audioScreening.local = {
        status: "fingerprint_generated",
        message: "Browser demo presentation only: no local audio file was analysed. Run this check in the desktop app for an authoritative result.",
        engine: "Chromaprint",
        engineVersion: "demo",
        sourceEvidenceId: source.id,
        sourceRelativePath: source.relativePath,
        sourceSha256: source.sha256,
        sourceSizeBytes: source.sizeBytes,
        durationMilliseconds: source.metadata?.audioDurationMilliseconds ?? undefined,
        fingerprintAlgorithm: "2",
        generatedAt: now(),
        artifactRelativePath: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json",
        artifactSha256: "d".repeat(64)
      };
      track.documents.current = false;
      track.integrity.generated = false;
      track.integrity.verified = false;
      refresh(track);
      return result(track, "Browser-Demo: Der lokale Screening-Status wurde nur zur Oberflächenvorschau simuliert. Die Desktop-App erzeugt den echten Chromaprint-Fingerprint.");
    },
    async runExternalAudioScreening(trackId, onProgress) {
      const track = mutableTrack(trackId);
      const source = releaseAudio(track);
      if (!source?.sha256) throw new Error("Importiere zuerst die autoritative finale Release-Audiodatei.");
      const configured = configuredAudioScreeningStatus(
        audioScreeningSettings,
        audioAccessKeyConfigured && audioAccessSecretConfigured
      );
      if (configured.status !== "ready") {
        const externalStatus = configured.status === "configuration_invalid"
          ? "configuration_invalid"
          : configured.status === "authentication_failed"
            ? "authentication_failed"
            : configured.status === "provider_unavailable"
              ? "provider_unavailable"
              : "skipped_not_configured";
        track.audioScreening.external = {
          provider: "ACRCloud",
          status: externalStatus,
          message: configured.statusMessage,
          sourceEvidenceId: source.id,
          sourceRelativePath: source.relativePath,
          sourceSha256: source.sha256,
          sourceSizeBytes: source.sizeBytes,
          matches: []
        };
        track.documents.current = false;
        track.integrity.generated = false;
        track.integrity.verified = false;
        refresh(track);
        return result(track, "Die optionale externe Prüfung wurde übersprungen; die ACRCloud-Konfiguration ist nicht vollständig.");
      }
      await demoProgress(onProgress, [
        { stage: "preparing_external_check", processedBytes: 0, totalBytes: source.sizeBytes, processedFiles: 0, totalFiles: 1, currentFile: source.relativePath },
        { stage: "sending_provider_request", processedBytes: Math.round(source.sizeBytes * .3), totalBytes: source.sizeBytes, processedFiles: 0, totalFiles: 1, currentFile: source.relativePath },
        { stage: "waiting_provider_response", processedBytes: Math.round(source.sizeBytes * .7), totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1 },
        { stage: "processing_provider_response", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1 },
        { stage: "saving_screening_result", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1 },
        { stage: "complete", processedBytes: source.sizeBytes, totalBytes: source.sizeBytes, processedFiles: 1, totalFiles: 1 }
      ]);
      track.audioScreening.external = {
        provider: "ACRCloud",
        status: "provider_unavailable",
        message: "Browser demo: no ACRCloud request was sent. The desktop app is required for an authoritative provider response.",
        sourceEvidenceId: source.id,
        sourceRelativePath: source.relativePath,
        sourceSha256: source.sha256,
        sourceSizeBytes: source.sizeBytes,
        checkedAt: now(),
        matches: []
      };
      track.documents.current = false;
      track.integrity.generated = false;
      track.integrity.verified = false;
      refresh(track);
      return result(track, "Browser-Demo: Es wurde keine ACRCloud-Anfrage ausgeführt und kein Providerergebnis erzeugt.");
    },
    async validateTrack(trackId): Promise<ValidationResult> {
      await wait();
      const track = get(trackId);
      return finalizationGate(track, track.profileSnapshot);
    },
    async finalizeTrack(trackId, _options?: FinalizeOptions, onProgress?) {
      const track = mutableTrack(trackId);
      const gate = finalizationGate(track, track.profileSnapshot);
      if (!gate.valid) throw new Error(`Finalisierung blockiert: ${[...gate.missingItems, ...gate.blockingItems].join(", ")}`);
      const totalFiles = track.integrity.fileCount;
      const totalBytes = Math.max(totalFiles, 1) * 8_476_231;
      await demoProgress(onProgress, [
        { stage: "validating_finalization_gate", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
        { stage: "collecting_final_snapshot", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
        { stage: "writing_finalization_marker", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
        { stage: "generating_certificate", processedBytes: 0, totalBytes: 0, processedFiles: track.evidence.length, totalFiles: track.evidence.length },
        { stage: "verifying_certificate", processedBytes: 0, totalBytes: 0, processedFiles: 3, totalFiles: 3 },
        { stage: "verifying", processedBytes: Math.round(totalBytes * .45), totalBytes, processedFiles: Math.floor(totalFiles * .45), totalFiles, currentFile: "03_DOCUMENTATION/README.md" },
        { stage: "verifying", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles, currentFile: "01_RELEASE/demo.wav" },
        { stage: "saving_final_snapshot", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles },
        { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
      ]);
      track.status = "FINALIZED";
      const certificateId = `SDM-${new Date().getFullYear()}-${track.id.slice(0, 8).toUpperCase()}`;
      track.certificate = {
        valid: true,
        certificateId,
        finalizedAt: now(),
        workflowVersion: WORKFLOW_VERSION,
        certificateLanguage: profile.certificateLanguage,
        bilingual: true
      };
      track.finalizationAnchors = [
        { artifact: "evidence_manifest", label: "Evidence manifest (recommended timestamp anchor)", relativePath: "06_CERTIFICATE/EVIDENCE_MANIFEST.json", sha256: "a".repeat(64) },
        { artifact: "sha256sums", label: "Track SHA-256 manifest", relativePath: "03_DOCUMENTATION/SHA256SUMS.txt", sha256: "b".repeat(64) },
        { artifact: "documentation_certificate_markdown", label: "Documentation certificate (Markdown)", relativePath: "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md", sha256: "c".repeat(64) },
        { artifact: "certificate_pdf", label: "Documentation certificate (English PDF)", relativePath: "SunoDM_DOCUMENTATION_CERTIFICATE.pdf", sha256: "d".repeat(64) },
        { artifact: "final_evidence_package", label: "Final evidence package certificate hash set", relativePath: "06_CERTIFICATE/CERTIFICATE_SHA256.txt", sha256: "e".repeat(64) }
      ];
      refresh(track);
      track.status = "FINALIZED";
      if (timestampSettings.enabled && timestampSettings.autoAfterFinalization) {
        attachConfiguredTimestamp(track);
      }
      return result(track, "Dokumentation finalisiert und Zertifikat erzeugt.");
    },
    async attachExternalTimestamp(trackId) {
      await wait();
      const track = get(trackId);
      return clone(attachConfiguredTimestamp(track));
    },
    async invalidateCertificate(trackId) {
      await wait();
      const track = get(trackId);
      if (track.status !== "FINALIZED") throw new Error("Der Track ist nicht finalisiert.");
      track.certificate.valid = false;
      track.certificate.invalidatedAt = now();
      track.certificate.invalidationReason = "Manuell invalidiert";
      return result(track, "Das Zertifikat wurde als ungültig markiert.");
    },
    async createRevision(trackId) {
      await wait();
      const track = get(trackId);
      if (track.status !== "FINALIZED") {
        throw new Error("Nur ein finalisierter Track kann eine neue Revision beginnen.");
      }
      migrateLegacySunoSemantics(track.fields);
      track.status = "ACTIVE";
      track.certificate = { valid: false };
      track.integrity.generated = false;
      track.integrity.verified = false;
      track.integrity.mismatchFiles = [];
      track.documents.current = false;
      track.externalTimestamps = [];
      track.externalTimestampSummary = notRecordedExternalTimestampSummary();
      track.audioScreening = clone(emptyAudioScreeningSummary);
      track.finalizationAnchors = [];
      refresh(track);
      return result(track, "Der bisherige Snapshot wurde archiviert und eine neue Revision angelegt.");
    },
    async reEvaluateTrack(trackId) {
      await wait();
      const track = get(trackId);
      if (track.workflowId === WORKFLOW_ID && track.workflowVersion === WORKFLOW_VERSION) {
        throw new Error("Der Track verwendet bereits die aktuelle Workflow-Version.");
      }
      if (track.status === "SUPERSEDED") {
        throw new Error("Der Track wurde durch eine neuere Revision ersetzt und kann nicht mehr geändert werden.");
      }
      const archived = track.status === "FINALIZED";
      migrateLegacySunoSemantics(track.fields);
      track.workflowId = WORKFLOW_ID;
      track.workflowVersion = WORKFLOW_VERSION;
      track.status = "ACTIVE";
      track.certificate = { valid: false };
      track.documents.current = false;
      track.integrity = {
        generated: false,
        verified: false,
        fileCount: 0,
        verifiedCount: 0,
        mismatchFiles: []
      };
      track.steps = [];
      track.externalTimestamps = [];
      track.externalTimestampSummary = notRecordedExternalTimestampSummary();
      track.audioScreening = clone(emptyAudioScreeningSummary);
      track.finalizationAnchors = [];
      refresh(track);
      return result(
        track,
        archived
          ? "Der bisherige Snapshot wurde archiviert; die Neubewertung verwendet den aktuellen Workflow."
          : "Die Neubewertung verwendet jetzt den aktuellen Workflow."
      );
    }
  };
}
