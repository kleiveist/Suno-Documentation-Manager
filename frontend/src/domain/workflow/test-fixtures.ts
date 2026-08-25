import {
  emptyAudioScreeningSummary,
  emptyEvidenceMetadata,
  emptyProfile,
  emptyTrackAutomation,
  emptyTrackFields,
  type EvidenceItem,
  type EvidenceRole,
  type GlobalProfile,
  type TrackDetail
} from "../types";

export const profile: GlobalProfile = {
  ...emptyProfile,
  artistName: "Test Artist",
  sunoProfileName: "Test Profile",
  sunoHandle: "@test",
  sunoPlan: "Premier",
  subscriptionStartDate: "2026-01-01",
  defaultAiImageService: "Local Image Tool"
};

export function evidence(role: EvidenceRole): EvidenceItem {
  return {
    id: role,
    role,
    fileName: `${role}.dat`,
    relativePath: `evidence/${role}.dat`,
    sha256: "a".repeat(64),
    sizeBytes: 42,
    importedAt: "2026-08-01T10:00:00Z",
    verified: true,
    provenance: "managed_copy",
    metadata: {
      ...emptyEvidenceMetadata(),
      originalFileName: ["suno_final_export", "release_wav"].includes(role) ? "Complete Track.wav" : `${role}.dat`
    }
  };
}

export function completeTrack(): TrackDetail {
  const fields: TrackDetail["fields"] = {
    ...emptyTrackFields(profile),
    title: "Complete Track",
    productionStartDate: "2026-07-01",
    productionEndDate: "2026-07-03",
    sunoModel: "v4.5",
    sunoProjectUrl: "https://suno.example.test/project",
    sunoProjectVersionId: "project-version-1",
    sunoFinalGenerationId: "generation-1",
    sunoFinalGenerationDate: "2026-07-03",
    sunoDownloadExportDate: "",
    sunoPlanAtGeneration: "Premier",
    finalExportDate: "2026-07-03",
    instrumentalTrack: true,
    vocalLyricsPresent: false,
    vocalIntent: "INSTRUMENTAL",
    sunoContentClassification: "STRUCTURE_ONLY",
    sunoLyricsContentSource: "human",
    sunoLyricsFieldText: "[Intro]\n[Drop]\n[Outro]",
    sunoStylePrompt: "cinematic synthwave, driving bass",
    externalAudioUploaded: false,
    ownAudioUploaded: false,
    codeBasedGeneration: false,
    thirdPartySamplesUploaded: false,
    humanEditingPerformed: false,
    postExportEditingPerformed: false,
    commercialUseIntended: false,
    artworkOrigin: "none",
    generativeAiUsed: false
  };
  return {
    id: "complete",
    title: fields.title,
    relativePath: "Complete Track",
    library: { section: "single" },
    status: "READY",
    updatedAt: "2026-08-01T10:00:00Z",
    progress: 100,
    missingCount: 0,
    workflowId: "suno-track",
    workflowVersion: "1.9",
    profileSnapshot: structuredClone(profile),
    automation: emptyTrackAutomation(),
    fields,
    steps: [],
    evidence: [evidence("suno_final_export"), evidence("release_wav")],
    documents: { generated: true, current: true, templateVersion: "1.11", files: ["README.md"] },
    integrity: { generated: true, verified: true, fileCount: 3, verifiedCount: 3, mismatchFiles: [] },
    certificate: { valid: false },
    externalTimestamps: [],
    audioScreening: {
      ...structuredClone(emptyAudioScreeningSummary),
      local: {
        status: "fingerprint_generated",
        message: "Local record generated for the current release audio.",
        sourceEvidenceId: "release_wav",
        sourceRelativePath: "evidence/release_wav.dat",
        sourceSha256: "a".repeat(64),
        sourceSizeBytes: 42,
        artifactRelativePath: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json",
        artifactSha256: "b".repeat(64)
      }
    },
    finalizationAnchors: [],
    blockingDeviations: []
  };
}
