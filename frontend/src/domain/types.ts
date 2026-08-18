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

export interface TrackAutomation {
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
  invalidatedAt?: string;
  invalidationReason?: string;
}

export interface ExternalTimestampInput {
  provider: string;
  timestampType: ExternalTimestampType;
  timestampValue: string;
  referencedArtifact: TimestampReferencedArtifact;
  otherReferencedArtifact: string;
  referencedSha256: string;
  externalReferenceId: string;
  providerVerificationUrl: string;
  note: string;
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
  recordRelativePath: string;
  markdownRelativePath: string;
  pdfRelativePath: string;
  hashListRelativePath: string;
  integrityVerified: boolean;
  integrityIssues: string[];
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
  sunoLyricsFieldContent: boolean | null;
  sunoLyricsContentTypes: SunoLyricsContentType[];
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
  disclosureText: "AI-assisted"
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
    sunoLyricsFieldContent: null,
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
