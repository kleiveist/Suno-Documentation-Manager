import { describe, expect, it } from "vitest";

import {
  calculateMissingRequirements,
  calculateProgress,
  contentCheckAllNegative,
  deriveStepStatus,
  evaluateRequirements,
  finalizationGate,
  subscriptionEvidenceRelevance,
  subscriptionProductionCoverageStatus,
  statusLabel,
  stepStatuses,
  visibleConditionalFields,
  evidenceRoleFileTypes
} from "./workflow";
import {
  emptyEvidenceMetadata,
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
  const fields = {
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
    sunoPlanAtCreation: "Premier",
    finalExportDate: "2026-07-03",
    lyricsSource: "instrumental" as const,
    instrumentalTrack: true,
    sunoStylePrompt: "cinematic synthwave, driving bass",
    externalAudioUploaded: false,
    ownAudioUploaded: false,
    codeBasedGeneration: false,
    thirdPartySamplesUploaded: false,
    humanEditingPerformed: false,
    postExportEditingPerformed: false,
    commercialUseIntended: false,
    artworkOrigin: "none" as const
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
    workflowVersion: "1.6",
    profileSnapshot: structuredClone(profile),
    automation: emptyTrackAutomation(),
    fields,
    steps: [],
    evidence: [evidence("suno_final_export"), evidence("release_wav")],
    documents: { generated: true, current: true, templateVersion: "1.7", files: ["README.md"] },
    integrity: { generated: true, verified: true, fileCount: 3, verifiedCount: 3, mismatchFiles: [] },
    certificate: { valid: false },
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

  it("blocks contradictory instrumental, lyrics and confirmed human-work facts", () => {
    const track = completeTrack();
    track.fields.instrumentalTrack = true;
    track.fields.lyricsSource = "mixed";
    track.fields.lyricsText = "Contradictory lyrics";
    track.fields.humanEditingPerformed = true;
    track.fields.humanEditingDetails = "Arrangement, Lyrics";
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("instrumental-consistency");
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

  it("checks commercial final-generation coverage and the explicit terms alternative", () => {
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
    expect(ids).not.toContain("terms-evidence");
    track.evidence.push(evidence("suno_terms_rights"));
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
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("ai-disclosure");
    const original = track.evidence.find((item) => item.role === "ai_artwork_original")!;
    const disclosed = {
      ...evidence("ai_artwork_edited"),
      provenance: "generated_disclosure" as const,
      derivedFromEvidenceId: original.id,
      generatorVersion: "local-disclosure-v1",
      generatedDisclosureText: "AI-assisted"
    };
    track.evidence.push(disclosed);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("ai-disclosure");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
    const final = track.evidence.find((item) => item.role === "final_artwork")!;
    final.sha256 = disclosed.sha256;
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("release-final-artwork");
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
    track.fields.sunoPlanAtCreation = "Historical Studio Plan";
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

  it("deactivates AI Transparency after three explicit No answers", () => {
    const track = completeTrack();
    track.fields.artworkOrigin = "ai_generated";
    track.fields.aiImageService = "Local Image Tool";
    track.fields.depictsRealPerson = false;
    track.fields.depictsRealEvent = false;
    track.fields.containsTrademark = false;
    track.evidence.push(evidence("ai_artwork_original"), evidence("final_artwork"));

    expect(contentCheckAllNegative(track.fields)).toBe(true);
    expect(visibleConditionalFields(track.fields, profile)).not.toContain("disclosure");
    expect(calculateMissingRequirements(track, profile).filter((item) => item.stepId === "ai_transparency")).toEqual([]);
    expect(calculateMissingRequirements(track, profile).map((item) => item.id)).not.toContain("artwork-final");
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
