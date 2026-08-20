import { describe, expect, it } from "vitest";

import {
  calculateMissingRequirements,
  calculateProgress,
  contentCheckAllNegative,
  deriveStepStatus,
  evaluateRequirements,
  finalizationGate,
  humanEditedFinalArtworkStatus,
  subscriptionEvidenceRelevance,
  subscriptionGenerationCoverageStatus,
  subscriptionProductionCoverageStatus,
  statusLabel,
  stepStatuses,
  visibleConditionalFields,
  evidenceRoleFileTypes,
  WORKFLOW_VERSION
} from "./workflow";
import {
  emptyEvidenceMetadata,
  emptyAudioScreeningSummary,
  emptyProfile,
  emptyTrackAutomation,
  emptyTrackFields,
  type EvidenceItem,
  type EvidenceRole,
  type GlobalProfile,
  type TrackDetail
} from "./types";

const profile: GlobalProfile = {
  ...emptyProfile,
  artistName: "Test Artist",
  sunoProfileName: "Test Profile",
  sunoHandle: "@test",
  sunoPlan: "Premier",
  subscriptionStartDate: "2026-01-01",
  defaultAiImageService: "Local Image Tool"
};

function evidence(role: EvidenceRole): EvidenceItem {
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
      originalFileName: ["suno_final_export", "release_wav"].includes(role) ? "Complete Track.wav" : `${role}.dat`,
    }
  };
}

function completeTrack(): TrackDetail {
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
    documents: { generated: true, current: true, templateVersion: "1.10", files: ["README.md"] },
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

describe("conditional fields", () => {
  it("hides external-audio details until yes", () => {
    const fields = emptyTrackFields(profile);
    expect(visibleConditionalFields(fields, profile)).not.toContain("externalAudioSource");
    fields.externalAudioUploaded = true;
    expect(visibleConditionalFields(fields, profile)).toEqual(expect.objectContaining(new Set()));
    expect([...visibleConditionalFields(fields, profile)]).toEqual(
      expect.arrayContaining(["externalAudioSource", "externalAudioOwnership", "externalAudioFile", "externalAudioLicense"])
    );
  });

  it("shows own-audio and sample details only when applicable", () => {
    const fields = emptyTrackFields(profile);
    fields.ownAudioUploaded = true;
    fields.thirdPartySamplesUploaded = true;
    const visible = [...visibleConditionalFields(fields, profile)];
    expect(visible).toEqual(expect.arrayContaining([
      "ownAudioSource", "ownAudioOwnership", "ownAudioFile",
      "thirdPartySampleSource", "thirdPartySampleOwnership", "thirdPartySampleFile", "thirdPartySampleLicense"
    ]));
  });

  it("shows source-code and generated-audio uploads only for code-based generation", () => {
    const fields = emptyTrackFields(profile);
    expect(visibleConditionalFields(fields, profile)).not.toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).not.toContain("codeGeneratedAudioFile");
    fields.codeBasedGeneration = false;
    expect(visibleConditionalFields(fields, profile)).not.toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).not.toContain("codeGeneratedAudioFile");
    fields.codeBasedGeneration = true;
    expect(visibleConditionalFields(fields, profile)).toContain("sourceCodeFile");
    expect(visibleConditionalFields(fields, profile)).toContain("codeGeneratedAudioFile");
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessed");
    fields.codeAudioPostProcessed = true;
    fields.codeAudioPostProcessingOperations = ["Mixing", "Other post-processing"];
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessingOperations");
    expect(visibleConditionalFields(fields, profile)).toContain("codeAudioPostProcessingNote");
  });

  it("shows AI and content-check follow-ups conditionally", () => {
    const fields = emptyTrackFields(profile);
    fields.artworkOrigin = "ai_assisted";
    fields.depictsRealPerson = true;
    fields.depictsRealEvent = true;
    fields.containsTrademark = true;
    const visible = [...visibleConditionalFields(fields, profile)];
    expect(visible).toEqual(expect.arrayContaining([
      "aiImageService", "aiArtworkOriginal", "disclosure", "humanArtworkModifications",
      "realPersonNotes", "realEventNotes", "trademarkNotes"
    ]));
  });
});

describe("missing requirements", () => {
  it("lists only applicable missing items", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile)).toEqual([]);
    track.fields.externalAudioUploaded = true;
    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toEqual(expect.arrayContaining(["external-source", "external-ownership", "external-audio-file", "external-license"]));
    expect(ids).not.toContain("sample-license");
  });

  it("does not require retired generation identifiers or a generation time", () => {
    const track = completeTrack();
    track.fields.sunoProjectVersionId = "";
    track.fields.sunoFinalGenerationId = "";
    track.fields.sunoFinalGenerationTime = "";

    expect(calculateMissingRequirements(track, profile)).toEqual([]);
    expect(finalizationGate(track, profile)).toEqual({
      valid: true,
      missingItems: [],
      blockingItems: []
    });
  });

  it("allows an unknown download/export date without inventing precision", () => {
    const track = completeTrack();
    track.fields.sunoDownloadExportDate = "";

    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("suno-download-date");
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("assigns the last-editing date only to the release step", () => {
    const track = completeTrack();
    track.fields.finalExportDate = "";

    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "export-date", stepId: "release" })
    );
  });

  it("requires a current local Chromaprint record for the authoritative release audio", () => {
    const track = completeTrack();
    track.audioScreening.local.status = "stale";
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );

    track.audioScreening.local.status = "fingerprint_generated";
    track.audioScreening.local.sourceSha256 = "f".repeat(64);
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );

    track.audioScreening.local.sourceSha256 = "a".repeat(64);
    track.audioScreening.local.sourceRelativePath = "01_RELEASE/old-release.wav";
    expect(calculateMissingRequirements(track, profile)).toContainEqual(
      expect.objectContaining({ id: "local-audio-screening", stepId: "release" })
    );
  });

  it("assigns the desktop-editing answer and details only to the release step", () => {
    const track = completeTrack();
    track.fields.postExportEditingPerformed = null;
    let missing = calculateMissingRequirements(track, profile);
    expect(missing).toContainEqual(expect.objectContaining({ id: "post-editing-answer", stepId: "release" }));
    expect(missing.some((item) => item.id === "post-editing-answer" && item.stepId === "human_work")).toBe(false);

    track.fields.postExportEditingPerformed = true;
    track.fields.postExportEditingDetails = "";
    missing = calculateMissingRequirements(track, profile);
    expect(missing).toContainEqual(expect.objectContaining({ id: "post-editing-details", stepId: "release" }));
  });

  it("mirrors each blocking native consistency issue once in its workflow step", () => {
    const track = completeTrack();
    track.automation.consistencyIssues = [{
      code: "suno_stored_metadata_mismatch",
      message: "Gespeicherte und eingebettete Suno-Metadaten stimmen nicht überein.",
      stepId: "suno",
      blocking: true
    }];

    const matches = calculateMissingRequirements(track, profile)
      .filter((item) => item.id === "consistency-suno_stored_metadata_mismatch");
    expect(matches).toEqual([expect.objectContaining({ stepId: "suno" })]);
    expect(finalizationGate(track, profile).valid).toBe(false);
  });

  it("TEST 01 accepts an instrumental with bracketed structure instructions and no vocal lyrics", () => {
    const track = completeTrack();
    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("instrumental-vocal-consistency");
    expect(ids).not.toContain("suno-lyrics-field-text");
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("keeps Vocal Intent, classification, instrumental mode, and final audio independent", () => {
    const track = completeTrack();
    track.fields.instrumentalTrack = true;
    track.fields.vocalLyricsPresent = true;
    track.fields.vocalIntent = "INSTRUMENTAL";
    track.fields.sunoContentClassification = "MIXED";
    track.fields.humanEditingPerformed = true;
    track.fields.humanEditingDetails = "Arrangement, Lyrics";
    track.fields.legacyLyricsText = "[Intro]\nlegacy value";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("instrumental-vocal-consistency");
    expect(finalizationGate(track, profile).valid).toBe(true);

    track.fields.vocalIntent = "VOCAL";
    track.fields.vocalLyricsPresent = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("instrumental-vocal-consistency");
    expect(finalizationGate(track, profile).valid).toBe(true);
  });

  it("uses EMPTY as the only N/A content branch and requires an OTHER label", () => {
    const track = completeTrack();
    track.fields.sunoContentClassification = "EMPTY";
    track.fields.vocalIntent = "UNSPECIFIED";
    track.fields.sunoLyricsContentSource = null;
    track.fields.sunoLyricsFieldText = "";
    track.fields.sunoLyricsOtherContentType = "";
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("suno-lyrics-content-source");
    expect(ids).not.toContain("suno-lyrics-field-text");
    expect(ids).not.toContain("suno-lyrics-other-content-type");

    track.fields.sunoContentClassification = "OTHER";
    track.fields.sunoLyricsContentSource = "human";
    track.fields.sunoLyricsFieldText = "[Spoken direction]";
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("suno-lyrics-other-content-type");
    track.fields.sunoLyricsOtherContentType = "Spoken performance direction";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("suno-lyrics-other-content-type");
  });

  it("requires explicit scalar semantics and accepts MIXED as one classification", () => {
    const track = completeTrack();
    track.fields.sunoContentClassification = null;
    track.fields.vocalIntent = null;
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toEqual(expect.arrayContaining(["suno-content-classification", "vocal-intent"]));

    track.fields.sunoContentClassification = "MIXED";
    track.fields.vocalIntent = "UNSPECIFIED";
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("suno-content-classification");
    expect(ids).not.toContain("vocal-intent");
  });

  it("does not let a historical plan-at-creation value satisfy the current generation-plan gate", () => {
    const track = completeTrack();
    track.fields.sunoPlanAtGeneration = "";
    track.fields.legacySunoPlanAtCreation = "Historical Pro";

    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("suno-plan");
  });

  it("TEST 03 accepts a vocal track with vocal lyrics", () => {
    const track = completeTrack();
    track.fields.instrumentalTrack = false;
    track.fields.vocalLyricsPresent = true;
    track.fields.vocalIntent = "VOCAL";
    track.fields.sunoContentClassification = "VOCAL_LYRICS_ONLY";
    track.fields.sunoLyricsFieldText = "Original vocal lyrics";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("instrumental-vocal-consistency");
  });

  it("requires explicit confirmation for an intentional filename deviation without changing the title", () => {
    const track = completeTrack();
    track.fields.title = "Gravaty";
    track.evidence.find((item) => item.role === "release_wav")!.metadata!.originalFileName = "GRAVITY.wav";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("release-filename");
    track.fields.releaseFilenameDifferenceConfirmed = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-filename");
    expect(track.fields.title).toBe("Gravaty");
  });

  it("TEST 04/05 requires complete core metadata for commercial Terms evidence", () => {
    const track = completeTrack();
    track.fields.commercialUseIntended = true;
    const subscription = evidence("subscription_payment");
    subscription.sourceGlobalEvidenceId = "global-subscription";
    subscription.coverageStart = "2026-07-01";
    subscription.coverageEnd = "2026-07-02";
    track.evidence.push(subscription);
    let ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("subscription-generation-coverage");
    expect(ids).toContain("terms-evidence");
    subscription.coverageEnd = "2026-07-31";
    track.fields.sunoTermsEvidenceNotAvailable = true;
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("subscription-generation-coverage");
    expect(ids).toContain("terms-evidence");
    const terms = evidence("suno_terms_rights");
    track.evidence.push(terms);
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("terms-evidence");
    terms.metadata = {
      ...terms.metadata!,
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17"
    };
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("terms-evidence");
    track.fields.sunoTermsEvidenceNotAvailable = false;
    ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("terms-evidence");
  });

  it("requires source, ownership, file and license on positive source branches", () => {
    const track = completeTrack();
    track.fields.thirdPartySamplesUploaded = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.evidenceRole)).toEqual(
      expect.arrayContaining(["third_party_sample_file", "third_party_sample_license"])
    );
  });

  it("requires source-code and generated WAV/MP3 evidence only after an explicit Yes answer", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("source-code-file");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("code-generated-audio-file");
    track.fields.codeBasedGeneration = true;
    track.fields.codeAudioPostProcessed = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toEqual(
      expect.arrayContaining(["source-code-file", "code-generated-audio-file"])
    );
    track.evidence.push(evidence("source_code_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("source-code-file");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("code-generated-audio-file");
    track.evidence.push(evidence("code_generated_audio_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("code-generated-audio-file");
  });

  it("requires reusable subscription evidence only for commercial tracks", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("subscription-evidence");
    track.fields.commercialUseIntended = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("subscription-evidence");
    const subscription = evidence("subscription_payment");
    subscription.sourceGlobalEvidenceId = "global-subscription";
    subscription.coverageStart = "2026-07-01";
    subscription.coverageEnd = "2026-07-31";
    track.evidence.push(subscription);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("subscription-evidence");
    subscription.coverageEnd = "2026-07-02";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("subscription-evidence");
  });

  it("combines adjacent subscription receipts and accepts a generation-only relevant receipt", () => {
    const track = completeTrack();
    track.fields.productionStartDate = "2026-07-18";
    track.fields.productionEndDate = "2026-08-17";
    track.fields.sunoFinalGenerationDate = "2026-08-17";
    const july = evidence("subscription_payment");
    july.sourceGlobalEvidenceId = "july";
    july.coverageStart = "2026-07-14";
    july.coverageEnd = "2026-08-13";
    const august = evidence("subscription_payment");
    august.sourceGlobalEvidenceId = "august";
    august.coverageStart = "2026-08-14";
    august.coverageEnd = "2026-09-13";

    expect(subscriptionEvidenceRelevance(august, track.fields)).toEqual({
      relevant: true,
      coversProduction: false,
      overlapsProduction: true,
      coversGeneration: true
    });
    expect(subscriptionProductionCoverageStatus([july], track.fields)).toBe("NO");
    expect(subscriptionProductionCoverageStatus([july, august], track.fields)).toBe("YES");
  });

  it("TEST 06/07 records the plan at generation and evaluates date coverage technically", () => {
    const track = completeTrack();
    track.fields.sunoPlanAtGeneration = "Premier";
    track.fields.sunoFinalGenerationDate = "2026-08-15";
    const covered = evidence("subscription_payment");
    covered.coverageStart = "2026-08-14";
    covered.coverageEnd = "2026-09-13";
    expect(subscriptionGenerationCoverageStatus([covered], track.fields)).toBe("YES");
    covered.coverageEnd = "2026-08-13";
    expect(subscriptionGenerationCoverageStatus([covered], track.fields)).toBe("NO");
    expect(WORKFLOW_VERSION).toBe("1.9");
  });

  it("requires a verified generated disclosure artifact for AI artwork", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_assisted";
    track.fields.humanArtworkModifications = ["Prompt written manually"];
    track.fields.aiImageService = "Local Image Tool";
    track.fields.disclosureApplied = true;
    track.fields.disclosureText = "AI-assisted";
    track.fields.depictsRealPerson = true;
    track.fields.realPersonNotes = "Fiktive Bearbeitung einer realen Person";
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    const independentFinal = evidence("final_artwork");
    independentFinal.sha256 = "b".repeat(64);
    track.evidence.push(evidence("ai_artwork_original"), independentFinal);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("ai-disclosure-decision");
    const original = track.evidence.find((item) => item.role === "ai_artwork_original")!;
    const disclosed = {
      ...evidence("ai_artwork_edited"),
      provenance: "generated_disclosure" as const,
      derivedFromEvidenceId: original.id,
      generatorVersion: "local-disclosure-v1",
      generatedDisclosureText: "AI-assisted"
    };
    track.evidence.push(disclosed);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("ai-disclosure-decision");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
    const final = track.evidence.find((item) => item.role === "final_artwork")!;
    final.sha256 = disclosed.sha256;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
  });

  it("TEST 08 completes the factual audio AI transparency questionnaire without legal conclusions", () => {
    const track = completeTrack();
    Object.assign(track.fields, {
      generativeAiUsed: true,
      audioAiSystem: "Suno",
      aiAssistedAudioElements: "yes",
      aiGeneratedAudioElements: "yes",
      realPersonVoiceIntentionallyImitated: "no",
      realPersonIdentityIntentionallyRepresented: "no",
      realEventRepresentedAsAuthenticRecording: "no",
      realLocationInstitutionEventPresentedAsAuthenticAiRecording: "no",
      audioDisclosureApplied: "yes",
      audioDisclosureLocations: ["release metadata"],
      audioDisclosureText: "AI-generated audio"
    } satisfies Partial<TrackDetail["fields"]>);

    expect(calculateMissingRequirements(track, profile).filter((item) => item.stepId === "ai_transparency")).toEqual([]);
  });

  it("TEST 09 keeps commercial generative-AI disclosure NOT DOCUMENTED visibly incomplete", () => {
    const track = completeTrack();
    Object.assign(track.fields, {
      commercialUseIntended: true,
      generativeAiUsed: true,
      audioAiSystem: "Suno",
      aiAssistedAudioElements: "yes",
      aiGeneratedAudioElements: "yes",
      realPersonVoiceIntentionallyImitated: "no",
      realPersonIdentityIntentionallyRepresented: "no",
      realEventRepresentedAsAuthenticRecording: "no",
      realLocationInstitutionEventPresentedAsAuthenticAiRecording: "no",
      audioDisclosureApplied: "not_documented"
    } satisfies Partial<TrackDetail["fields"]>);

    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).toContain("audio-disclosure-status");
  });

  it("TEST 10 accepts a deliberate NO disclosure with an optional factual reason", () => {
    const track = completeTrack();
    Object.assign(track.fields, {
      generativeAiUsed: true,
      audioAiSystem: "Suno",
      aiAssistedAudioElements: "yes",
      aiGeneratedAudioElements: "yes",
      realPersonVoiceIntentionallyImitated: "no",
      realPersonIdentityIntentionallyRepresented: "no",
      realEventRepresentedAsAuthenticRecording: "no",
      realLocationInstitutionEventPresentedAsAuthenticAiRecording: "no",
      audioDisclosureApplied: "no",
      audioDisclosureReason: "User-confirmed publication decision"
    } satisfies Partial<TrackDetail["fields"]>);

    const ids = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(ids).not.toContain("audio-disclosure-status");
    expect(ids).not.toContain("audio-disclosure-locations");
    expect(ids).not.toContain("audio-disclosure-text");
  });

  it("evaluates code-audio post-processing only on the applicable branch", () => {
    const track = completeTrack();
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processed-answer"
    );

    track.fields.codeBasedGeneration = true;
    track.fields.codeAudioPostProcessed = false;
    track.evidence.push(evidence("source_code_file"), evidence("code_generated_audio_file"));
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processing-operations"
    );

    track.fields.codeAudioPostProcessed = true;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain(
      "code-audio-post-processing-operations"
    );
    track.fields.codeAudioPostProcessingOperations = ["Mixing", "EQ", "Mastering"];
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "code-audio-post-processing-operations"
    );
  });

  it("accepts future and historical free-text Suno model and plan values", () => {
    const track = completeTrack();
    track.fields.sunoModel = "v6";
    track.fields.sunoPlanAtGeneration = "Historical Studio Plan";
    const missing = calculateMissingRequirements(track, profile).map((item) => item.id);
    expect(missing).not.toContain("suno-model");
    expect(missing).not.toContain("suno-plan");
  });

  it("requires at least one human change for AI-assisted artwork", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_assisted";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain(
      "artwork-human-changes"
    );
    track.fields.humanArtworkModifications = ["Cropping", "Color correction"];
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain(
      "artwork-human-changes"
    );
  });

  it("requires an explicit artwork disclosure decision even after three negative content checks", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_generated";
    track.fields.aiImageService = "Local Image Tool";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("ai_artwork_original"), evidence("final_artwork"));

    expect(contentCheckAllNegative(track.fields)).toBe(true);
    expect(visibleConditionalFields(track.fields, profile)).toContain("disclosure");
    track.fields.generativeAiUsed = null;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toEqual(
      expect.arrayContaining(["generative-ai-answer", "ai-disclosure-decision"])
    );
    track.fields.generativeAiUsed = false;
    track.fields.disclosureApplied = false;
    expect(calculateMissingRequirements(track, profile).filter((item) => item.stepId === "ai_transparency")).toEqual([]);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
  });

  it("requires the YES/NO artwork decision under the none policy and accepts deliberate NO", () => {
    const track = completeTrack();
    const noPolicy = { ...profile, artworkTransparencyPolicy: "none" as const };
    track.fields.artworkOrigin = "ai_generated";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("ai_artwork_original"), evidence("final_artwork"));

    expect(calculateMissingRequirements(track, noPolicy).map((item) => item.id))
      .toContain("ai-disclosure-decision");
    track.fields.disclosureApplied = false;
    expect(calculateMissingRequirements(track, noPolicy).map((item) => item.id))
      .not.toContain("ai-disclosure-decision");
  });

  it("TEST 16 distinguishes NO, NOT DOCUMENTED and deterministically non-applicable answers", () => {
    const track = completeTrack();
    track.fields.generativeAiUsed = true;
    track.fields.audioAiSystem = "Suno";
    track.fields.aiAssistedAudioElements = "yes";
    track.fields.aiGeneratedAudioElements = "no";
    track.fields.realPersonVoiceIntentionallyImitated = "not_documented";
    track.fields.realPersonIdentityIntentionallyRepresented = "no";
    track.fields.realEventRepresentedAsAuthenticRecording = "no";
    track.fields.realLocationInstitutionEventPresentedAsAuthenticAiRecording = "no";
    track.fields.audioDisclosureApplied = "no";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("voice-imitation");
    track.fields.realPersonVoiceIntentionallyImitated = null;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("voice-imitation");
    track.fields.generativeAiUsed = false;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("voice-imitation");
  });

  it("surfaces every attached but missing or unverified evidence item", () => {
    const track = completeTrack();
    const optional = evidence("other");
    optional.verified = false;
    optional.verificationError = "Evidence file is missing.";
    track.evidence.push(optional);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain(
      `unverified-evidence-${optional.id}`
    );
  });
});

describe("progress", () => {
  it("uses completed applicable requirements as the denominator", () => {
    const track = completeTrack();
    expect(calculateProgress(evaluateRequirements(track, profile))).toBe(100);
    track.fields.externalAudioUploaded = true;
    expect(calculateProgress(evaluateRequirements(track, profile))).toBeLessThan(100);
  });

  it("excludes non-applicable branches from the denominator", () => {
    const base = completeTrack();
    const baseCount = evaluateRequirements(base, profile).length;
    base.fields.externalAudioUploaded = true;
    expect(evaluateRequirements(base, profile).length).toBe(baseCount + 4);
  });
});

describe("statuses and finalization", () => {
  it("reports the human-edited/final artwork hash comparison by role", () => {
    const humanEdited = evidence("human_edited_artwork");
    const finalArtwork = evidence("final_artwork");
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork]))
      .toBe("BYTE-IDENTICAL / SHA-256 MATCH");
    finalArtwork.sha256 = "b".repeat(64);
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork]))
      .toBe("NO SHA-256 MATCH");
    finalArtwork.verified = false;
    expect(humanEditedFinalArtworkStatus([humanEdited, finalArtwork]))
      .toBe("NOT VERIFIED");
  });

  it("lists the accepted file types directly for every evidence role", () => {
    expect(evidenceRoleFileTypes("suno_project_zip")).toBe("ZIP");
    expect(evidenceRoleFileTypes("suno_screenshot")).toContain("PNG");
    expect(evidenceRoleFileTypes("release_wav")).toContain("FLAC");
    expect(evidenceRoleFileTypes("source_code_file")).toContain("Python");
    expect(evidenceRoleFileTypes("code_generated_audio_file")).toBe("WAV oder MP3");
  });

  it("treats three explicit No answers as a completed artwork content check", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "human";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("final_artwork"));

    const artwork = stepStatuses(track, profile).find((step) => step.id === "artwork");
    expect(artwork?.status).toBe("PASS");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toEqual(
      expect.arrayContaining(["real-person-answer", "real-event-answer", "trademark-answer"])
    );
  });

  it("renders and derives supported statuses", () => {
    expect(statusLabel("NOT_RUN")).toBe("Offen");
    expect(statusLabel("PASS")).toBe("Erfüllt");
    expect(statusLabel("FAIL")).toBe("Fehlgeschlagen");
    expect(statusLabel("BLOCKED")).toBe("Blockiert");
    expect(statusLabel("N_A")).toBe("N/A");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A", naReason: "Kein Artwork" }, false)).toBe("N_A");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A", naReason: "Kein Artwork" }, true)).toBe("PASS");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "N_A" })).toBe("PASS");
    expect(deriveStepStatus("artwork", [], { id: "artwork", status: "BLOCKED" })).toBe("PASS");
  });

  it("keeps Finalize blocked until every preceding step is complete", () => {
    const track = completeTrack();
    track.fields.productionEndDate = "";
    const statuses = stepStatuses(track, profile);
    expect(statuses.find((step) => step.id === "track")?.status).not.toBe("PASS");
    expect(statuses.find((step) => step.id === "finalize")?.status).toBe("BLOCKED");
  });

  it("blocks finalization for stale documents, mismatches and unresolved deviations", () => {
    const track = completeTrack();
    track.documents.current = false;
    track.integrity.mismatchFiles = ["01_RELEASE/final.wav"];
    track.blockingDeviations = [{ id: "dev", title: "Blocker", description: "Quelle ungeklärt", blocking: true, resolved: false, createdAt: "2026-08-01" }];
    const gate = finalizationGate(track, profile);
    expect(gate.valid).toBe(false);
    expect(gate.missingItems).toContain("Aktuelle generierte Dokumente");
    expect(gate.blockingItems.join(" ")).toContain("Quelle ungeklärt");
    expect(gate.blockingItems.join(" ")).toContain("final.wav");
  });

  it("allows finalization only when every applicable requirement is complete", () => {
    expect(finalizationGate(completeTrack(), profile)).toEqual({ valid: true, missingItems: [], blockingItems: [] });
  });
});
