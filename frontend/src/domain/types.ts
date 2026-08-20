export type TrackStatus = "DRAFT" | "ACTIVE" | "READY" | "FINALIZED" | "SUPERSEDED";

export type StepStatus = "NOT_RUN" | "PASS" | "FAIL" | "BLOCKED" | "N_A" | "NOT_VERIFIED";

export type StepId =
  | "track"
  | "source"
  | "suno"
  | "human_work"
  | "artwork"
  | "ai_transparency"
  | "release"
  | "evidence_licenses"
  | "integrity"
  | "finalize";

export type EvidenceRole =
  | "suno_final_export"
  | "suno_project_zip"
  | "suno_screenshot"
  | "subscription_payment"
  | "release_wav"
  | "release_mp3"
  | "release_mp4"
  | "release_artwork"
  | "artwork_suno_original"
  | "ai_artwork_original"
  | "ai_artwork_edited"
  | "human_edited_artwork"
  | "final_artwork"
  | "external_audio_license"
  | "external_audio_file"
  | "own_audio_file"
  | "source_code_file"
  | "code_generated_audio_file"
  | "third_party_sample_file"
  | "third_party_sample_license"
  | "suno_terms_rights"
  | "external_timestamp"
  | "lyrics"
  | "style"
  | "other";

export type ArtworkOrigin = "none" | "human" | "ai_generated" | "ai_assisted";
export type LyricsSource = "instrumental" | "human" | "suno" | "mixed";
export type DocumentationAnswer = "yes" | "no" | "not_documented";
export type SunoContentClassification =
  | "STRUCTURE_ONLY"
  | "VOCAL_LYRICS_ONLY"
  | "MIXED"
  | "EMPTY"
  | "OTHER";
export type VocalIntent = "VOCAL" | "INSTRUMENTAL" | "UNSPECIFIED";
/** @deprecated Read compatibility for historical multi-value track records only. */
export type SunoLyricsContentType =
  | "vocal_lyrics"
  | "structure_instructions"
  | "sound_instructions"
  | "arrangement_instructions"
  | "mixed"
  | "other";
export type SunoLyricsContentSource = "human" | "ai" | "mixed";
export type DisclosurePolicy = "always" | "per_artwork" | "none";
export type SubscriptionBillingCycle = "monthly" | "annual";
export type TrackLibrarySection = "single" | "album";
export type CertificateLanguage = "de" | "en";

/**
 * A configured provider is deliberately kept separate from track data.  The
 * provider only ever receives the selected snapshot digest; secrets are held
 * by the native secure configuration mechanism and are never represented
 * here.
 */
export type TimestampProviderKind =
  | "disabled"
  | "free_tsa"
  | "open_timestamps"
  | "sigstore_public_tsa"
  | "custom_rfc3161";

export type TimestampAuthenticationMode = "none" | "basic" | "bearer_token" | "api_key" | "client_certificate";

export type ExternalTimestampStatus =
  | "not_recorded"
  | "requesting"
  | "attached"
  | "verified"
  | "verification_failed"
  | "provider_unavailable"
  | "authentication_failed"
  | "anchor_mismatch"
  | "disabled"
  | "ready"
  | "configuration_incomplete"
  | "authentication_required"
  | "connection_failed"
  | "unsupported_response"
  | "verification_configuration_incomplete";

/** The native model uses one factual status set for provider and attachment state. */
export type TimestampProviderStatus = ExternalTimestampStatus;

export interface TimestampProviderCapabilities {
  rfc3161: boolean;
  openTimestamps: boolean;
  requiresAuthentication: boolean;
  supportsSha256: boolean;
  supportsOfflineVerification: boolean;
  returnsSignedTimestamp: boolean;
  externalTrustRootAvailable: boolean;
  /** Informational only; never a legal qualification. */
  qualificationStatus: string;
}

export interface TimestampCustomSettings {
  providerName: string;
  endpoint: string;
  authenticationMode: TimestampAuthenticationMode;
  /** A non-secret account name, if the selected authentication method needs one. */
  username: string;
  /** Local configuration paths only. Private key material is never exposed here. */
  clientCertificatePath: string;
  caCertificatePath: string;
  policyOid: string;
  timeoutSeconds: number;
}

export interface TimestampSettings {
  enabled: boolean;
  provider: TimestampProviderKind;
  autoAfterFinalization: boolean;
  custom: TimestampCustomSettings;
  /** Read-only provider state calculated by the native adapter. */
  status: TimestampProviderStatus;
  statusMessage: string;
  lastTestedAt?: string;
}

export interface TimestampProviderTestResult {
  provider: TimestampProviderKind;
  status: TimestampProviderStatus;
  message: string;
  testedAt: string;
  capabilities: TimestampProviderCapabilities;
}

/**
 * Audio-screening results are technical observations, not a legal, rights or
 * infringement assessment. The full local fingerprint and raw provider
 * response deliberately remain in the portable track documentation and are
 * never returned to the browser UI.
 */
export type AudioScreeningStatus =
  | "not_run"
  | "fingerprint_generated"
  | "no_match_detected"
  | "match_detected"
  | "skipped_not_configured"
  | "provider_unavailable"
  | "authentication_failed"
  | "configuration_invalid"
  | "engine_unavailable"
  | "unsupported_format"
  | "processing_failed"
  | "stale";

/** State of the optional ACRCloud configuration itself, separate from a track result. */
export type AudioScreeningProviderStatus =
  | "disabled"
  | "not_configured"
  | "ready"
  | "authentication_failed"
  | "provider_unavailable"
  | "configuration_invalid";

export interface AudioScreeningMatch {
  title?: string;
  artists?: string[];
  album?: string;
  isrc?: string;
  acrid?: string;
  score?: number;
}

export interface AudioScreeningLocalSummary {
  status: AudioScreeningStatus;
  message: string;
  engine?: string;
  engineVersion?: string;
  sourceEvidenceId?: string;
  sourceRelativePath?: string;
  sourceSha256?: string;
  sourceSizeBytes?: number;
  durationMilliseconds?: number;
  fingerprintAlgorithm?: string;
  generatedAt?: string;
  artifactRelativePath?: string;
  artifactSha256?: string;
}

export interface AudioScreeningExternalSummary {
  provider: "ACRCloud";
  status: AudioScreeningStatus;
  message: string;
  sourceEvidenceId?: string;
  sourceRelativePath?: string;
  sourceSha256?: string;
  sourceSizeBytes?: number;
  checkedAt?: string;
  sampleOffsetMilliseconds?: number;
  sampleDurationMilliseconds?: number;
  sourceDurationMilliseconds?: number;
  requestCount?: number;
  responseRelativePath?: string;
  responseSha256?: string;
  matches: AudioScreeningMatch[];
}

export interface AudioScreeningSummary {
  local: AudioScreeningLocalSummary;
  external: AudioScreeningExternalSummary;
}

export interface AudioScreeningSettings {
  enabled: boolean;
  host: string;
  timeoutSeconds: number;
  status: AudioScreeningProviderStatus;
  statusMessage: string;
  credentialsConfigured: boolean;
  lastTestedAt?: string;
  localEngineAvailable: boolean;
  localEngineVersion?: string;
}

/** Write-only inputs; native secure storage never returns either field. */
export interface AudioScreeningSecretInput {
  accessKey?: string;
  accessSecret?: string;
}

export interface AudioScreeningProviderTestResult {
  status: AudioScreeningProviderStatus;
  message: string;
  testedAt: string;
}

export type ExternalTimestampType =
  | "qualified_electronic_timestamp_user_declared"
  | "electronic_timestamp"
  | "external_integrity_timestamp"
  | "other"
  | "not_documented";

export type TimestampReferencedArtifact =
  | "evidence_manifest"
  | "sha256sums"
  | "documentation_certificate_markdown"
  | "certificate_pdf"
  | "final_evidence_package"
  | "other";

export interface TrackLibraryAssignment {
  section: TrackLibrarySection;
  albumTitle?: string;
}

export type FolderImportKind = "single" | "album";

export interface FolderImportFile {
  fileName: string;
  roles: string[];
  selected: boolean;
}

export interface FolderImportTrackProposal {
  title: string;
  sourcePath: string;
  files: FolderImportFile[];
  ambiguities: string[];
  unassignedFiles: string[];
}

export interface FolderImportProposal {
  sourcePath: string;
  kind: FolderImportKind;
  albumTitle?: string;
  tracks: FolderImportTrackProposal[];
  unassignedFiles: string[];
}

export interface FolderImportExecutionInput {
  sourcePath: string;
  expectedKind: FolderImportKind;
  singleTrackTitle?: string;
  singleTrackLibrary?: TrackLibraryAssignment;
  productionStartDate: string;
  commercialUseIntended?: boolean;
}

export interface WorkspaceSummary {
  id: string;
  name: string;
  path: string;
  trackCount: number;
  lastScannedAt?: string;
}

export interface GlobalProfile {
  artistName: string;
  sunoProfileName: string;
  sunoHandle: string;
  sunoPlan: string;
  subscriptionStartDate: string;
  defaultCommercialUse: boolean;
  defaultAiImageService: string;
  artworkTransparencyPolicy: DisclosurePolicy;
  disclosureText: string;
  certificateLanguage: CertificateLanguage;
}

export interface EvidenceItem {
  id: string;
  role: EvidenceRole;
  fileName: string;
  relativePath: string;
  sha256?: string;
  sizeBytes: number;
  importedAt: string;
  verified: boolean;
  verificationError?: string;
  sourceGlobalEvidenceId?: string;
  coverageStart?: string;
  coverageEnd?: string;
  provenance?: "managed_copy" | "indexed_legacy" | "generated_disclosure" | "global_copy";
  derivedFromEvidenceId?: string;
  generatorVersion?: string;
  generatedDisclosureText?: string;
  metadata?: EvidenceMetadata;
}

export interface EvidenceMetadata {
  originalFileName: string;
  documentTitle: string;
  provider: string;
  sourceUrl: string;
  retrievalDate: string;
  effectiveDate: string;
  applicableProductionPeriod: string;
  factualNote: string;
  externalTimestamp: string;
  timestampType: string;
  externalReferenceId: string;
  providerVerificationUrl: string;
  referencedHash: string;
  referencedArtifact: string;
  fileExtension: string;
  mimeType: string;
  audioFormat: string;
  audioChannels: number | null;
  audioSampleRateHz: number | null;
  audioDurationMilliseconds: number | null;
  audioBitDepth: number | null;
  embeddedMetadata: EmbeddedMetadata[];
  sunoStudioDetected: boolean;
  sunoCreatedTimestamp: string;
  sunoCreatedDate: string;
  sunoId: string;
  sunoRawMetadata: string;
}

export interface EmbeddedMetadata {
  key: string;
  value: string;
}

export type FactOrigin = "user_confirmed_fact" | "evidence_derived_metadata" | "not_documented";

export interface ByteIdenticalPair {
  leftEvidenceId: string;
  leftRole: EvidenceRole;
  rightEvidenceId: string;
  rightRole: EvidenceRole;
  sha256: string;
}

export interface ConsistencyIssue {
  code: string;
  message: string;
  stepId: StepId;
  blocking: boolean;
}

/**
 * A display-only classification of automatic findings. These values do not
 * extend StepStatus and must never be used as workflow or finalization gates.
 */
export type AutomaticConsistencyFindingLevel = "INFO" | "WARNING" | "BLOCKING";

/** A display-only aggregate derived from the authoritative track state. */
export type AutomaticConsistencyOutcome = "PASS" | "PASS WITH WARNINGS" | "BLOCKED";

export interface AutomaticConsistencyFinding {
  code: string;
  level: AutomaticConsistencyFindingLevel;
  message: string;
  stepId: StepId;
}

export interface AutomaticConsistencyPresentation {
  outcome: AutomaticConsistencyOutcome;
  findings: AutomaticConsistencyFinding[];
  infoCount: number;
  warningCount: number;
  blockingCount: number;
}

export interface TrackAutomation {
  finalGenerationIdOrigin: FactOrigin;
  finalGenerationOrigin: FactOrigin;
  productionEndOrigin: FactOrigin;
  downloadExportOrigin: FactOrigin;
  finalExportOrigin: FactOrigin;
  sunoMetadataDetected: boolean;
  sunoCreatedTimestamp?: string;
  sunoId?: string;
  releaseIdenticalToSunoExport: boolean;
  byteIdenticalPairs: ByteIdenticalPair[];
  consistencyIssues: ConsistencyIssue[];
}

export interface EvidencePreview {
  evidenceId: string;
  role: EvidenceRole;
  fileName: string;
  relativePath: string;
  sizeBytes: number;
  mimeType?: string;
  dataUrl?: string;
  textContent?: string;
  message?: string;
}

export interface TrackCoverPreview {
  evidenceId: string;
  dataUrl: string;
}

export interface GlobalEvidenceItem extends EvidenceItem {
  notes?: string;
}

export interface WorkflowStepState {
  id: StepId;
  status: StepStatus;
  naReason?: string;
  updatedAt?: string;
}

export interface IntegrityState {
  generated: boolean;
  verified: boolean;
  fileCount: number;
  verifiedCount: number;
  generatedAt?: string;
  verifiedAt?: string;
  mismatchFiles: string[];
}

export interface DocumentState {
  generated: boolean;
  current: boolean;
  generatedAt?: string;
  templateVersion: string;
  files: string[];
}

export interface CertificateState {
  valid: boolean;
  certificateId?: string;
  finalizedAt?: string;
  workflowVersion?: string;
  certificateLanguage?: CertificateLanguage;
  bilingual?: boolean;
  invalidatedAt?: string;
  invalidationReason?: string;
}

/** Deprecated compatibility payload; native finalization always creates both PDF languages. */
export interface FinalizeOptions {
  bilingual: boolean;
}

export interface ExternalTimestampRecord {
  id: string;
  certificateId: string;
  provider: string;
  timestampType: ExternalTimestampType;
  timestampValue: string;
  referencedArtifact: TimestampReferencedArtifact;
  referencedArtifactPath: string;
  referencedSha256: string;
  actualSha256: string;
  referencedHashMatch: boolean | null;
  externalReferenceId: string;
  providerVerificationUrl: string;
  note: string;
  evidenceFileName: string;
  evidenceSha256: string;
  importedAt: string;
  provenance: string;
  /** Absent for legacy manually recorded timestamp evidence. */
  providerMetadata?: TimestampProviderMetadata;
  recordRelativePath: string;
  markdownRelativePath: string;
  pdfRelativePath: string;
  hashListRelativePath: string;
  integrityVerified: boolean;
  integrityIssues: string[];
}

export interface TimestampProviderMetadata {
  adapter: string;
  protocol: string;
  requestAlgorithm: string;
  responseFormat: string;
  providerEndpointIdentifier: string;
  /** Untouched provider response bytes retained alongside the usable proof. */
  providerResponseFileName: string;
  providerResponseSha256: string;
  referencedRevisionId: string;
  issuer: string;
  certificateSubject: string;
  certificateSerialNumber: string;
  requestNonce?: string;
  responseNonce?: string;
  nonceMatch?: boolean | null;
  requestedPolicyOid?: string;
  policyOid: string;
  policyMatch?: boolean | null;
  cryptographicVerifier?: string;
  trustAnchorSha256?: string[];
  responseStructureValid: boolean | null;
  providerDigestMatch: boolean | null;
  signatureVerified: boolean | null;
  trustChainVerified: boolean | null;
  verificationResult: ExternalTimestampStatus;
  verificationMessage: string;
  verificationTimestamp: string;
}

/** A compact, current-snapshot status for the normal finalization UI. */
export interface ExternalTimestampSummary {
  status: ExternalTimestampStatus;
  message: string;
  provider: string;
  recordId?: string;
  updatedAt?: string;
}

export interface FinalizationAnchor {
  artifact: TimestampReferencedArtifact;
  label: string;
  relativePath: string;
  sha256: string;
}

export interface TrackFields {
  title: string;
  productionStartDate: string;
  productionEndDate: string;
  sunoModel: string;
  sunoProjectUrl: string;
  sunoProjectVersionId: string;
  sunoFinalGenerationId: string;
  sunoFinalGenerationDate: string;
  sunoFinalGenerationTime: string;
  sunoDownloadExportDate: string;
  sunoPlanAtGeneration: string;
  legacySunoPlanAtCreation: string;
  finalExportDate: string;
  instrumentalTrack: boolean | null;
  legacyLyricsSource: LyricsSource | "";
  legacyLyricsText: string;
  vocalLyricsPresent: boolean | null;
  vocalIntent: VocalIntent | null;
  /** @deprecated Historical controller retained only while reading old records. */
  sunoLyricsFieldContent?: boolean | null;
  sunoContentClassification: SunoContentClassification | null;
  /** @deprecated Read compatibility for historical multi-value track records only. */
  sunoLyricsContentTypes?: SunoLyricsContentType[];
  sunoLyricsContentSource: SunoLyricsContentSource | null;
  sunoLyricsFieldText: string;
  sunoLyricsOtherContentType: string;
  sunoStylePrompt: string;
  externalAudioUploaded: boolean | null;
  externalAudioSource: string;
  externalAudioOwnership: string;
  ownAudioUploaded: boolean | null;
  ownAudioSource: string;
  ownAudioOwnership: string;
  codeBasedGeneration: boolean | null;
  codeAudioPostProcessed: boolean | null;
  codeAudioPostProcessingOperations: string[];
  codeAudioPostProcessingNote: string;
  thirdPartySamplesUploaded: boolean | null;
  thirdPartySampleSource: string;
  thirdPartySampleOwnership: string;
  humanEditingPerformed: boolean | null;
  humanEditingDetails: string;
  postExportEditingPerformed: boolean | null;
  postExportEditingDetails: string;
  commercialUseIntended: boolean;
  releaseFilenameDifferenceConfirmed: boolean | null;
  sunoExportFilenameDifferenceConfirmed: boolean | null;
  sunoTermsEvidenceNotAvailable: boolean | null;
  artworkOrigin: ArtworkOrigin | "";
  aiImageService: string;
  humanArtworkProcessOperations: string[];
  humanArtworkProcessNotes: string;
  humanArtworkModifications: string[];
  customArtworkChange: string;
  depictsRealPerson: boolean | null;
  realPersonNotes: string;
  depictsRealEvent: boolean | null;
  realEventNotes: string;
  containsTrademark: boolean | null;
  trademarkNotes: string;
  disclosureApplied: boolean | null;
  disclosureText: string;
  generativeAiUsed: boolean | null;
  audioAiSystem: string;
  aiAssistedAudioElements: DocumentationAnswer | null;
  aiGeneratedAudioElements: DocumentationAnswer | null;
  realPersonVoiceIntentionallyImitated: DocumentationAnswer | null;
  realPersonIdentityIntentionallyRepresented: DocumentationAnswer | null;
  realEventRepresentedAsAuthenticRecording: DocumentationAnswer | null;
  realLocationInstitutionEventPresentedAsAuthenticAiRecording: DocumentationAnswer | null;
  audioDisclosureApplied: DocumentationAnswer | null;
  audioDisclosureLocations: string[];
  audioDisclosureText: string;
  audioDisclosureReason: string;
  releaseNotes: string;
}

export type TrackFieldPatch = Partial<Omit<
  TrackFields,
  "sunoLyricsFieldContent" | "sunoLyricsContentTypes"
>>;

export interface TrackSummary {
  id: string;
  title: string;
  relativePath: string;
  library: TrackLibraryAssignment;
  status: TrackStatus;
  updatedAt: string;
  progress: number;
  missingCount: number;
  certificateValid?: boolean;
  legacy?: boolean;
  coverEvidenceId?: string;
}

export interface TrackDetail extends TrackSummary {
  workflowId: string;
  workflowVersion: string;
  profileSnapshot: GlobalProfile;
  automation: TrackAutomation;
  fields: TrackFields;
  steps: WorkflowStepState[];
  evidence: EvidenceItem[];
  documents: DocumentState;
  integrity: IntegrityState;
  certificate: CertificateState;
  externalTimestamps: ExternalTimestampRecord[];
  externalTimestampSummary?: ExternalTimestampSummary;
  audioScreening: AudioScreeningSummary;
  finalizationAnchors: FinalizationAnchor[];
  blockingDeviations?: BlockingDeviation[];
  missingItems?: string[];
}

export interface TrackCreateInput {
  title: string;
  productionStartDate: string;
  commercialUseIntended: boolean;
  library: TrackLibraryAssignment;
}

export interface ValidationResult {
  valid: boolean;
  missingItems: string[];
  blockingItems: string[];
}

export interface ScanResult {
  discovered: number;
  indexed: number;
  unchanged: number;
  warnings: string[];
  candidates?: Array<{
    name: string;
    relativePath: string;
    status: "INCOMPLETE" | "NOT_VERIFIED" | "INDEXED";
    missingItems: string[];
    hasManagedDocumentCollision: boolean;
  }>;
}

export interface BlockingDeviation {
  id: string;
  title: string;
  description: string;
  blocking: boolean;
  resolved: boolean;
  createdAt: string;
  resolvedAt?: string;
}

export interface ActionResult {
  message: string;
  track?: TrackDetail;
}

export interface OperationProgress {
  stage: string;
  processedBytes: number;
  totalBytes: number;
  processedFiles: number;
  totalFiles: number;
  currentFile?: string;
}

export interface DocumentPreview {
  files: string[];
  collisions: string[];
  adoptionRequired: boolean;
}

export interface WorkflowDefinitionDto {
  schemaVersion: number;
  id: string;
  version: string;
  name: string;
  steps: Array<{
    id: StepId;
    number: string;
    label: string;
    title?: string;
    description: string;
    required: boolean;
  }>;
}

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
    matches: []
  }
};

export const emptyAudioScreeningSettings: AudioScreeningSettings = {
  enabled: false,
  host: "",
  timeoutSeconds: 30,
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
