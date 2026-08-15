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
  | "ai_artwork_original"
  | "ai_artwork_edited"
  | "human_edited_artwork"
  | "final_artwork"
  | "external_audio_license"
  | "external_audio_file"
  | "own_audio_file"
  | "source_code_file"
  | "third_party_sample_file"
  | "third_party_sample_license"
  | "other";

export type ArtworkOrigin = "none" | "human" | "ai_generated" | "ai_assisted";
export type LyricsSource = "instrumental" | "human" | "suno" | "mixed";
export type DisclosurePolicy = "always" | "per_artwork" | "none";
export type SubscriptionBillingCycle = "monthly" | "annual";
export type TrackLibrarySection = "single" | "album";

export interface TrackLibraryAssignment {
  section: TrackLibrarySection;
  albumTitle?: string;
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

export interface TrackFields {
  title: string;
  productionStartDate: string;
  productionEndDate: string;
  sunoModel: string;
  sunoProjectUrl: string;
  sunoPlanAtCreation: string;
  finalExportDate: string;
  lyricsSource: LyricsSource | "";
  lyricsText: string;
  sunoStylePrompt: string;
  externalAudioUploaded: boolean | null;
  externalAudioSource: string;
  externalAudioOwnership: string;
  ownAudioUploaded: boolean | null;
  ownAudioSource: string;
  ownAudioOwnership: string;
  codeBasedGeneration: boolean | null;
  thirdPartySamplesUploaded: boolean | null;
  thirdPartySampleSource: string;
  thirdPartySampleOwnership: string;
  humanEditingPerformed: boolean | null;
  humanEditingDetails: string;
  postExportEditingPerformed: boolean | null;
  postExportEditingDetails: string;
  commercialUseIntended: boolean;
  artworkOrigin: ArtworkOrigin | "";
  aiImageService: string;
  humanArtworkModifications: string;
  depictsRealPerson: boolean | null;
  realPersonNotes: string;
  depictsRealEvent: boolean | null;
  realEventNotes: string;
  containsTrademark: boolean | null;
  trademarkNotes: string;
  disclosureApplied: boolean | null;
  disclosureText: string;
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
}

export interface TrackDetail extends TrackSummary {
  workflowId: string;
  workflowVersion: string;
  profileSnapshot: GlobalProfile;
  fields: TrackFields;
  steps: WorkflowStepState[];
  evidence: EvidenceItem[];
  documents: DocumentState;
  integrity: IntegrityState;
  certificate: CertificateState;
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

export function emptyTrackFields(profile: GlobalProfile = emptyProfile): TrackFields {
  return {
    title: "",
    productionStartDate: "",
    productionEndDate: "",
    sunoModel: "",
    sunoProjectUrl: "",
    sunoPlanAtCreation: profile.sunoPlan,
    finalExportDate: "",
    lyricsSource: "",
    lyricsText: "",
    sunoStylePrompt: "",
    externalAudioUploaded: null,
    externalAudioSource: "",
    externalAudioOwnership: "",
    ownAudioUploaded: null,
    ownAudioSource: "",
    ownAudioOwnership: "",
    codeBasedGeneration: null,
    thirdPartySamplesUploaded: null,
    thirdPartySampleSource: "",
    thirdPartySampleOwnership: "",
    humanEditingPerformed: null,
    humanEditingDetails: "",
    postExportEditingPerformed: null,
    postExportEditingDetails: "",
    commercialUseIntended: profile.defaultCommercialUse,
    artworkOrigin: "",
    aiImageService: profile.defaultAiImageService,
    humanArtworkModifications: "",
    depictsRealPerson: null,
    realPersonNotes: "",
    depictsRealEvent: null,
    realEventNotes: "",
    containsTrademark: null,
    trademarkNotes: "",
    disclosureApplied: null,
    disclosureText: profile.disclosureText,
    releaseNotes: ""
  };
}
