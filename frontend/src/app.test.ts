import { describe, expect, it } from "vitest";

import {
  MAIN_NAVIGATION,
  canonicalGuidedChoiceArray,
  canonicalGuidedChoiceList,
  canonicalGuidedChoiceValue,
  canonicalTrackFieldPatch,
  canCreateTrackRevision,
  documentationAnswerLabel,
  externalAudioScreeningIsCurrent,
  externalTimestampAttachmentIsTerminal,
  externalTimestampRecordPresentation,
  externalTimestampSummaryFor,
  externalTimestampIntegrityPresentation,
  externalTimestampMatchLabel,
  externalTimestampTypeLabel,
  factOriginLabel,
  finalizedTrackPresentation,
  generativeAiAudioDetailState,
  isAutomaticDateReadonly,
  isTrackContentLocked,
  legacySunoPlanNoticeMarkup,
  localAudioScreeningIsCurrent,
  missingProfileFields,
  normalizeGuidedTrackFields,
  operationProgressPercent,
  operationStageLabel,
  parseMultiChoiceValue,
  resetWorkspaceScopedUiState,
  SETTINGS_CATEGORY_DEFINITIONS,
  SUNO_CONTENT_CLASSIFICATION_CHOICES,
  settingsCategoryNavigationMarkup,
  shouldDiscardLockedDraft,
  shouldIgnoreModalBackdropClick,
  singleChoiceFieldMarkup,
  termsMetadataComplete,
  timestampArtifactLabel,
  timestampProviderIsReady,
  timestampProviderProtocolPresentation,
  timestampProviderStatusLabel,
  VOCAL_INTENT_CHOICES,
  visibleExternalAudioScreening,
  visibleLocalAudioScreening,
  externalTimestampStatusLabel,
  serializeMultiChoiceValue,
  trackSummaryFromDetail,
  trackCheckSummary,
  type GuidedChoice,
  type WorkspaceScopedUiState,
  workflowUpgradeFinalizationBlocker,
  workflowUpgradePresentation
} from "./app";
import { evidenceRoleFileTypes, WORKFLOW_STEPS } from "./domain/workflow";
import { emptyAudioScreeningSettings, emptyAudioScreeningSummary, emptyEvidenceMetadata, emptyProfile, emptyTimestampSettings, emptyTrackAutomation, emptyTrackFields, type EvidenceItem, type ExternalTimestampRecord, type TrackDetail } from "./domain/types";
import { resolveTheme, storedTheme, toggledTheme } from "./ui/theme";
import { translateUiText } from "./ui/i18n";

describe("theme", () => {
  it("uses a valid saved choice before the operating-system preference", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("falls back to the operating-system preference for missing or invalid storage", () => {
    expect(resolveTheme(null, true)).toBe("dark");
    expect(resolveTheme("unsupported", false)).toBe("light");
    expect(storedTheme("unsupported")).toBeNull();
  });

  it("toggles between both supported themes", () => {
    expect(toggledTheme("light")).toBe("dark");
    expect(toggledTheme("dark")).toBe("light");
  });
});

describe("navigation", () => {
  it("keeps the three settings categories and their local navigation labels stable", () => {
    expect(SETTINGS_CATEGORY_DEFINITIONS.map(({ id, label }) => [id, label])).toEqual([
      ["global", "Globale Angaben"],
      ["external", "Externe Dienste"],
      ["files", "Globale Datei-Führung"]
    ]);

    const markup = settingsCategoryNavigationMarkup();
    expect(markup).toContain('data-settings-category="global"');
    expect(markup).toContain('data-settings-category="external"');
    expect(markup).toContain('data-settings-category="files"');
    expect(markup).not.toContain('data-action=');
    expect(settingsCategoryNavigationMarkup("external")).toContain('data-settings-category="external" aria-controls="settings-external" aria-selected="true"');
    for (const { label } of SETTINGS_CATEGORY_DEFINITIONS) expect(markup).toContain(label);
  });

  it("uses the certificate language as the app language for shared UI copy", () => {
    expect(translateUiText("Einstellungen", "en")).toBe("Settings");
    expect(translateUiText("Einstellungen", "de")).toBe("Einstellungen");
    expect(translateUiText("Die Nichtanwendung wurde bewusst dokumentiert.", "en"))
      .toBe("The deliberate non-application has been documented.");
    expect(translateUiText("Import-Zeitstempel dokumentieren nur den Import in SunoDM und nicht die tatsächliche Erstellungs- oder Bearbeitungsreihenfolge der Artwork-Dateien.", "en"))
      .toContain("Import timestamps document only the import into SunoDM");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("Global details");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("External services");
    expect(settingsCategoryNavigationMarkup("external", "en")).toContain("Settings sections");
  });

  it("stores multiple guided choices deterministically", () => {
    expect(serializeMultiChoiceValue(["Mixing", "Mastering", "Mixing"])).toBe("Mixing | Mastering");
    expect(parseMultiChoiceValue("Mixing | Mastering")).toEqual(["Mixing", "Mastering"]);
  });

  it("stores English values while accepting localized labels and retaining unknown legacy data", () => {
    const choices: readonly GuidedChoice[] = [
      ["Timing and cuts", "Timing und Cuts"],
      ["Loudness adjustment", "Lautheitsanpassung"]
    ];
    expect(canonicalGuidedChoiceValue("Timing und Cuts", choices)).toBe("Timing and cuts");
    expect(canonicalGuidedChoiceList("Timing und Cuts | Lautheitsanpassung", choices)).toBe(
      "Timing and cuts | Loudness adjustment"
    );
    expect(canonicalGuidedChoiceValue("Historischer Freitext", choices)).toBe("Historischer Freitext");
    expect(canonicalGuidedChoiceArray(["Timing und Cuts", "Historischer Freitext"], choices)).toEqual([
      "Timing and cuts", "Historischer Freitext"
    ]);
  });

  it("renders required single choices as mutually exclusive buttons", () => {
    const markup = singleChoiceFieldMarkup(
      "sunoLyricsContentSource",
      "Content source",
      "human",
      [["instrumental", "Instrumental"], ["human", "Menschlich geschrieben"]],
      true
    );

    expect(markup).not.toContain("<select");
    expect(markup.match(/type="radio"/g)).toHaveLength(2);
    expect(markup.match(/name="sunoLyricsContentSource"/g)).toHaveLength(2);
    expect(markup).toContain('value="human" data-single-choice checked required');
    expect(markup).toContain("Wähle genau eine passende Option aus.");
  });

  it("normalizes every guided track value before saving", () => {
    const normalized = normalizeGuidedTrackFields({
      ...emptyTrackFields(),
      ownAudioSource: "Eigene Aufnahme",
      ownAudioOwnership: "Eigene Produktion",
      humanEditingDetails: "Timing und Cuts | Lautheitsanpassung",
      postExportEditingDetails: "Schnitt | Mixing",
      codeAudioPostProcessingOperations: ["Schnitt", "Mastering"],
      humanArtworkProcessOperations: ["Eigenständig gezeichnet"],
      humanArtworkModifications: ["Farbkorrektur", "Historischer Artwork-Freitext"],
      audioAiSystem: "Historisches unbekanntes KI-System",
      legacySunoPlanAtCreation: "Historical Pro",
      legacyLyricsSource: "mixed",
      legacyLyricsText: "[Intro]\n[Drop]",
      releaseNotes: "Originale Suno-Fassung | Radio Edit"
    });

    expect(normalized.ownAudioSource).toBe("Original field recording");
    expect(normalized.ownAudioOwnership).toBe("Solely owned by the artist");
    expect(normalized.humanEditingDetails).toBe("Timing and cuts | Loudness adjustment");
    expect(normalized.postExportEditingDetails).toBe("Editing and cuts | Mixing");
    expect(normalized.codeAudioPostProcessingOperations).toEqual(["Editing and cuts", "Mastering"]);
    expect(normalized.humanArtworkProcessOperations).toEqual(["Independently drawn"]);
    expect(normalized.humanArtworkModifications).toEqual(["Color correction", "Historischer Artwork-Freitext"]);
    expect(normalized.audioAiSystem).toBe("Historisches unbekanntes KI-System");
    expect(normalized.legacySunoPlanAtCreation).toBe("Historical Pro");
    expect(normalized.legacyLyricsSource).toBe("mixed");
    expect(normalized.legacyLyricsText).toBe("[Intro]\n[Drop]");
    expect(normalized.vocalLyricsPresent).toBeNull();
    expect(normalized.sunoLyricsFieldContent).toBeNull();
    expect(normalized.sunoLyricsContentTypes).toEqual([]);
    expect(normalized.releaseNotes).toBe("Original Suno version | Radio edit");
  });

  it("keeps the canonical Suno classification and Vocal Intent choices exact and singular", () => {
    expect(SUNO_CONTENT_CLASSIFICATION_CHOICES.map(([value]) => value)).toEqual([
      "",
      "STRUCTURE_ONLY",
      "VOCAL_LYRICS_ONLY",
      "MIXED",
      "EMPTY",
      "OTHER"
    ]);
    expect(VOCAL_INTENT_CHOICES.map(([value]) => value)).toEqual([
      "",
      "VOCAL",
      "INSTRUMENTAL",
      "UNSPECIFIED"
    ]);
  });

  it("normalizes EMPTY details without deriving Vocal Intent or classification", () => {
    const empty = normalizeGuidedTrackFields({
      ...emptyTrackFields(),
      sunoContentClassification: "EMPTY",
      vocalIntent: "UNSPECIFIED",
      sunoLyricsFieldContent: true,
      sunoLyricsContentTypes: ["vocal_lyrics", "structure_instructions"],
      sunoLyricsContentSource: "mixed",
      sunoLyricsFieldText: "stale text",
      sunoLyricsOtherContentType: "stale label"
    });
    expect(empty.sunoContentClassification).toBe("EMPTY");
    expect(empty.vocalIntent).toBe("UNSPECIFIED");
    expect(empty.sunoLyricsFieldContent).toBeNull();
    expect(empty.sunoLyricsContentTypes).toEqual([]);
    expect(empty.sunoLyricsContentSource).toBeNull();
    expect(empty.sunoLyricsFieldText).toBe("");
    expect(empty.sunoLyricsOtherContentType).toBe("");

    const legacyOnly = normalizeGuidedTrackFields({
      ...emptyTrackFields(),
      instrumentalTrack: true,
      vocalLyricsPresent: true,
      sunoLyricsFieldContent: true,
      sunoLyricsContentTypes: ["vocal_lyrics"],
      sunoLyricsFieldText: "legacy vocal text"
    });
    expect(legacyOnly.sunoContentClassification).toBeNull();
    expect(legacyOnly.vocalIntent).toBeNull();
  });

  it("omits both legacy classification controllers from new API patches", () => {
    const patch = canonicalTrackFieldPatch({
      ...emptyTrackFields(),
      sunoContentClassification: "MIXED",
      vocalIntent: "VOCAL",
      sunoLyricsFieldContent: true,
      sunoLyricsContentTypes: ["vocal_lyrics", "structure_instructions"]
    });
    expect(patch.sunoContentClassification).toBe("MIXED");
    expect(patch.vocalIntent).toBe("VOCAL");
    expect(patch).not.toHaveProperty("sunoLyricsFieldContent");
    expect(patch).not.toHaveProperty("sunoLyricsContentTypes");
  });

  it("labels automatic dates as evidence-derived without adding another input fact", () => {
    expect(factOriginLabel("evidence_derived_metadata")).toBe("Automatisch aus Suno-WAV erkannt");
    expect(factOriginLabel("user_confirmed_fact")).toBe("Nutzerangabe");
    expect(factOriginLabel("not_documented")).toBe("Noch nicht dokumentiert");
    expect(isAutomaticDateReadonly("evidence_derived_metadata")).toBe(true);
    expect(isAutomaticDateReadonly("user_confirmed_fact")).toBe(false);
    expect(isAutomaticDateReadonly("not_documented")).toBe(false);
  });

  it("TEST 16 keeps YES, NO, NOT DOCUMENTED and timestamp verification states distinct", () => {
    expect(documentationAnswerLabel("yes")).toBe("YES");
    expect(documentationAnswerLabel("no")).toBe("NO");
    expect(documentationAnswerLabel("not_documented")).toBe("NOT DOCUMENTED");
    expect(documentationAnswerLabel(null)).toBe("NOT ANSWERED");
    expect(externalTimestampMatchLabel(true)).toBe("YES");
    expect(externalTimestampMatchLabel(false)).toBe("NO");
    expect(externalTimestampMatchLabel(null)).toBe("NOT VERIFIED");
    expect(generativeAiAudioDetailState(true)).toBe("details_required");
    expect(generativeAiAudioDetailState(false)).toBe("not_applicable");
    expect(generativeAiAudioDetailState(null)).toBe("not_documented");
  });

  it("keeps native timestamp formats visible while the generic importer stays separate", () => {
    expect(evidenceRoleFileTypes("external_timestamp")).toContain("TSR");
    expect(evidenceRoleFileTypes("external_timestamp")).toContain("TST");
    expect(evidenceRoleFileTypes("external_timestamp")).toContain("P7S");
  });

  it("TEST 04/05 identifies complete Terms core metadata without inventing optional facts", () => {
    expect(termsMetadataComplete({
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17"
    })).toBe(true);
    expect(termsMetadataComplete({ documentTitle: "Suno Terms", provider: "", retrievalDate: "" })).toBe(false);
  });

  it("TEST 11/12 uses factual external timestamp labels without claiming qualification", () => {
    expect(externalTimestampTypeLabel("qualified_electronic_timestamp_user_declared"))
      .toBe("Qualified electronic timestamp – user declared");
    expect(timestampArtifactLabel("evidence_manifest")).toBe("EVIDENCE_MANIFEST.json");
    expect(timestampArtifactLabel("certificate_pdf")).toBe("Certificate PDF (English)");
  });

  it("keeps global provider readiness separate from optional per-track timestamp evidence", () => {
    expect(timestampProviderStatusLabel("ready")).toBe("Ready");
    expect(externalTimestampStatusLabel("not_recorded")).toBe("NOT RECORDED");
    expect(timestampProviderIsReady({
      ...emptyTimestampSettings,
      custom: { ...emptyTimestampSettings.custom },
      enabled: true,
      provider: "free_tsa",
      status: "ready"
    })).toBe(true);
    expect(timestampProviderIsReady({
      ...emptyTimestampSettings,
      custom: { ...emptyTimestampSettings.custom },
      enabled: false,
      provider: "free_tsa",
      status: "ready"
    })).toBe(false);
  });

  it("distinguishes OpenTimestamps from RFC 3161 in provider settings", () => {
    expect(timestampProviderProtocolPresentation("open_timestamps")).toEqual({
      label: "OpenTimestamps — nicht RFC 3161",
      detail: expect.stringContaining("ATTACHED")
    });
    expect(timestampProviderProtocolPresentation("free_tsa")).toEqual({
      label: "RFC 3161",
      detail: expect.stringContaining("TSA Trust Anchor")
    });
  });

  it("presents an initial OpenTimestamps proof without RFC-specific claims", () => {
    const record = {
      provider: "OpenTimestamps",
      timestampValue: "",
      providerMetadata: {
        adapter: "open_timestamps",
        protocol: "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade"
      } as NonNullable<ExternalTimestampRecord["providerMetadata"]>
    };

    expect(externalTimestampRecordPresentation(record)).toEqual({
      openTimestamps: true,
      timestampLabel: "Confirmed timestamp",
      timestampValue: "PENDING — OpenTimestamps verification / upgrade required",
      bindingLabel: "Local manifest / proof binding",
      cmsTrustValue: "N/A — not RFC 3161",
      endpointLabel: "Calendar endpoint"
    });
  });

  it("allows a ready RFC 3161 sidecar after OTS while keeping terminal records terminal", () => {
    const ots = {
      provider: "OpenTimestamps",
      provenance: "Automatic provider response",
      providerMetadata: {
        adapter: "open_timestamps",
        protocol: "OpenTimestamps detached proof"
      } as NonNullable<ExternalTimestampRecord["providerMetadata"]>
    };

    expect(externalTimestampAttachmentIsTerminal("attached", ots, "open_timestamps")).toBe(true);
    expect(externalTimestampAttachmentIsTerminal("attached", ots, "free_tsa")).toBe(false);
    expect(externalTimestampAttachmentIsTerminal("verified", ots, "free_tsa")).toBe(true);
    expect(externalTimestampAttachmentIsTerminal("attached", {
      ...ots,
      provenance: "Legacy manually recorded timestamp evidence"
    }, "free_tsa")).toBe(false);
  });

  it("presents an unrequested ACRCloud check as skipped when global credentials are absent", () => {
    const external = { status: "not_run" as const, message: "No external catalog screening has been run." };
    expect(visibleExternalAudioScreening(external, emptyAudioScreeningSettings)).toEqual({
      status: "skipped_not_configured",
      message: "ACRCloud ist nicht aktiviert; es wurde keine externe Katalogprüfung gestartet."
    });
    expect(visibleExternalAudioScreening(external, { status: "not_configured" })).toEqual({
      status: "skipped_not_configured",
      message: "ACRCloud-Zugangsdaten fehlen; es wurde keine externe Katalogprüfung gestartet."
    });
    expect(visibleExternalAudioScreening(external, { status: "ready" })).toEqual(external);
  });

  it("never presents a renamed release file as covered by its earlier local fingerprint", () => {
    const release = {
      id: "release-1",
      role: "release_wav",
      fileName: "current-release.wav",
      relativePath: "01_RELEASE/current-release.wav",
      sha256: "a".repeat(64),
      sizeBytes: 1_024,
      importedAt: "2026-08-18T12:00:00Z",
      verified: true,
      provenance: "managed_copy",
      metadata: emptyEvidenceMetadata()
    } satisfies EvidenceItem;
    const local = {
      ...emptyAudioScreeningSummary.local,
      status: "fingerprint_generated" as const,
      sourceEvidenceId: release.id,
      sourceRelativePath: release.relativePath,
      sourceSha256: release.sha256,
      sourceSizeBytes: release.sizeBytes
    };

    expect(localAudioScreeningIsCurrent(local, [release])).toBe(true);
    const renamedRelease = { ...release, relativePath: "01_RELEASE/renamed-release.wav" };
    expect(localAudioScreeningIsCurrent(local, [renamedRelease])).toBe(false);
    expect(visibleLocalAudioScreening(local, [renamedRelease])).toEqual(expect.objectContaining({
      status: "stale",
      message: expect.stringContaining("nicht mehr an die aktuelle finale Release-Datei gebunden")
    }));
    expect(local.status).toBe("fingerprint_generated");
  });

  it("masks an external catalog result when its release binding is stale", () => {
    const release = {
      id: "release-1",
      role: "release_wav",
      fileName: "current-release.wav",
      relativePath: "01_RELEASE/current-release.wav",
      sha256: "b".repeat(64),
      sizeBytes: 1_024,
      importedAt: "2026-08-18T12:00:00Z",
      verified: true,
      provenance: "managed_copy",
      metadata: emptyEvidenceMetadata()
    } satisfies EvidenceItem;
    const external = {
      ...emptyAudioScreeningSummary.external,
      status: "match_detected" as const,
      message: "Provider reported a match.",
      sourceEvidenceId: release.id,
      sourceRelativePath: "01_RELEASE/earlier-release.wav",
      sourceSha256: release.sha256,
      sourceSizeBytes: release.sizeBytes
    };

    expect(externalAudioScreeningIsCurrent(external, [release])).toBe(false);
    expect(visibleExternalAudioScreening(external, { status: "ready" }, [release])).toEqual({
      status: "stale",
      message: expect.stringContaining("nicht mehr an die aktuelle finale Release-Datei gebunden")
    });
  });

  it("keeps referenced-hash and addendum-integrity results visibly separate", () => {
    expect(externalTimestampMatchLabel(true)).toBe("YES");
    expect(externalTimestampIntegrityPresentation({
      integrityVerified: false,
      integrityIssues: [" Timestamp evidence SHA-256 changed. ", "PDF addendum differs."]
    })).toEqual({
      label: "FAILED",
      issues: ["Timestamp evidence SHA-256 changed.", "PDF addendum differs."]
    });
    expect(externalTimestampIntegrityPresentation({
      integrityVerified: true,
      integrityIssues: []
    })).toEqual({ label: "VERIFIED", issues: [] });
  });

  it("does not promote legacy manually recorded timestamp evidence to verified", () => {
    const summary = externalTimestampSummaryFor({
      externalTimestamps: [{
        id: "legacy-timestamp",
        certificateId: "SDM-legacy",
        provider: "Legacy TSA",
        timestampType: "electronic_timestamp",
        timestampValue: "2026-08-17T12:00:00Z",
        referencedArtifact: "evidence_manifest",
        referencedArtifactPath: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        referencedSha256: "a".repeat(64),
        actualSha256: "a".repeat(64),
        referencedHashMatch: true,
        externalReferenceId: "",
        providerVerificationUrl: "",
        note: "",
        evidenceFileName: "legacy.tsr",
        evidenceSha256: "b".repeat(64),
        importedAt: "2026-08-17T12:00:00Z",
        provenance: "Managed copy; user-confirmed metadata; system-verified SHA-256 comparison",
        recordRelativePath: "06_CERTIFICATE/EXTERNAL_TIMESTAMPS/legacy/TIMESTAMP_RECORD.json",
        markdownRelativePath: "06_CERTIFICATE/EXTERNAL_TIMESTAMPS/legacy/EXTERNAL_TIMESTAMP_ADDENDUM.md",
        pdfRelativePath: "06_CERTIFICATE/EXTERNAL_TIMESTAMPS/legacy/EXTERNAL_TIMESTAMP_ADDENDUM.pdf",
        hashListRelativePath: "06_CERTIFICATE/EXTERNAL_TIMESTAMPS/legacy/TIMESTAMP_RECORD_SHA256.txt",
        integrityVerified: true,
        integrityIssues: []
      }]
    });

    expect(summary.status).toBe("attached");
    expect(summary.message).toContain("Legacy manually recorded");
  });

  it("starts the generation plan empty and presents a legacy plan only as historical data", () => {
    const fields = emptyTrackFields({ ...emptyProfile, sunoPlan: "Premier" });
    expect(fields.sunoPlanAtGeneration).toBe("");
    expect(fields.legacySunoPlanAtCreation).toBe("");

    const markup = legacySunoPlanNoticeMarkup("Historical <Pro>");
    expect(markup).toContain("Historischer Tarif bei Erstellung");
    expect(markup).toContain("keine Aussage zum Tarif bei der finalen Generation");
    expect(markup).toContain("Historical &lt;Pro&gt;");
    expect(markup).toContain("erfüllt die aktuelle Workflow-Anforderung nicht");
    expect(legacySunoPlanNoticeMarkup("   ")).toBe("");
  });

  it("summarizes metadata, integrity, coverage and warnings compactly", () => {
    const track = {
      id: "summary-track",
      title: "Summary Track",
      relativePath: "Singles/Summary Track",
      library: { section: "single" },
      status: "ACTIVE",
      updatedAt: "2026-08-17T10:00:00Z",
      progress: 0,
      missingCount: 1,
      workflowId: "suno-track",
      workflowVersion: "1.7",
      fields: { ...emptyTrackFields(), commercialUseIntended: false },
      profileSnapshot: emptyProfile,
      automation: {
        ...emptyTrackAutomation(),
        sunoMetadataDetected: true,
        consistencyIssues: [{ code: "conflict", message: "Konflikt", stepId: "suno", blocking: true }]
      },
      evidence: [],
      steps: [],
      documents: { generated: false, current: false, templateVersion: "1.8", files: [] },
      integrity: { generated: true, verified: true, fileCount: 0, verifiedCount: 0, mismatchFiles: [] },
      blockingDeviations: [],
      certificate: { valid: false },
      externalTimestamps: [],
      audioScreening: structuredClone(emptyAudioScreeningSummary),
      finalizationAnchors: []
    } satisfies TrackDetail;

    expect(trackCheckSummary(track)).toEqual({
      documentation: "unvollständig",
      fileIntegrity: "geprüft",
      sunoMetadata: "erkannt",
      subscriptionCoverage: "nicht erforderlich",
      warningCount: 1
    });
  });

  it("exposes every required German main view", () => {
    expect(MAIN_NAVIGATION.map((item) => [item.id, item.label])).toEqual([
      ["dashboard", "Dashboard"], ["tracks", "Tracks"], ["current", "Aktueller Track"],
      ["workspace", "Workspace"], ["settings", "Einstellungen"]
    ]);
  });

  it("prevents creation of an immutable track snapshot from incomplete global data", () => {
    expect(missingProfileFields(emptyProfile)).toEqual(expect.arrayContaining([
      "Künstlername", "Suno-Profilname", "Suno-Benutzername", "Suno-Tarif", "Abo-Startdatum", "Standard-KI-Bilddienst"
    ]));
    expect(missingProfileFields({
      ...emptyProfile,
      artistName: "Artist", sunoProfileName: "Profile", sunoHandle: "@artist", sunoPlan: "Premier",
      subscriptionStartDate: "2026-01-01", defaultAiImageService: "Local Tool"
    })).toEqual([]);
  });

  it("makes all ten workflow steps reachable in their declared order", () => {
    expect(WORKFLOW_STEPS.map((step) => step.id)).toEqual([
      "track", "source", "suno", "human_work", "artwork", "ai_transparency",
      "release", "evidence_licenses", "integrity", "finalize"
    ]);
  });

  it("clears every workspace-scoped selection before entering another workspace", () => {
    const previous: WorkspaceScopedUiState = {
      track: { id: "old-track" } as WorkspaceScopedUiState["track"],
      trackDraft: { title: "Old track draft" } as WorkspaceScopedUiState["trackDraft"],
      activeStep: "release",
      trackTab: "certificate",
      scanResult: { discovered: 1, indexed: 1, unchanged: 0, warnings: [] },
      albums: ["Old Album"],
      showNewTrack: true,
      folderImport: null,
      showTrackLibrary: true,
      showSubscriptionEvidence: true,
      evidencePreview: {
        evidenceId: "preview",
        role: "suno_screenshot",
        fileName: "preview.png",
        relativePath: "02_SUNO/preview.png",
        sizeBytes: 42
      },
      showCertificatePopup: true,
      termsMetadataDialog: { evidenceId: null, metadata: emptyEvidenceMetadata() },
      timestampSettings: { ...emptyTimestampSettings, custom: { ...emptyTimestampSettings.custom }, enabled: true, provider: "free_tsa", status: "ready" },
      timestampProviderTest: {
        provider: "free_tsa",
        status: "ready",
        message: "Provider reachable",
        testedAt: "2026-08-18T12:00:00Z",
        capabilities: {
          rfc3161: true,
          openTimestamps: false,
          requiresAuthentication: false,
          supportsSha256: true,
          supportsOfflineVerification: false,
          returnsSignedTimestamp: true,
          externalTrustRootAvailable: false,
          qualificationStatus: "unknown"
        }
      },
      audioScreeningSettings: {
        ...emptyAudioScreeningSettings,
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com",
        credentialsConfigured: true,
        status: "ready"
      },
      audioScreeningProviderTest: {
        status: "ready",
        message: "Provider reachable",
        testedAt: "2026-08-18T12:00:00Z"
      },
      query: "old workspace query",
      trackFilter: "finalized",
      draftDirty: true
    };

    const reset = resetWorkspaceScopedUiState(previous);

    expect(reset).toEqual({
      track: null,
      trackDraft: null,
      activeStep: null,
      trackTab: "overview",
      scanResult: null,
      albums: [],
      showNewTrack: false,
      folderImport: null,
      showTrackLibrary: false,
      showSubscriptionEvidence: false,
      evidencePreview: null,
      showCertificatePopup: false,
      termsMetadataDialog: null,
      timestampSettings: { ...emptyTimestampSettings, custom: { ...emptyTimestampSettings.custom } },
      timestampProviderTest: null,
      audioScreeningSettings: { ...emptyAudioScreeningSettings },
      audioScreeningProviderTest: null,
      query: "",
      trackFilter: "all",
      draftDirty: false
    });
    expect(previous.track).not.toBeNull();
    expect(previous.trackDraft).not.toBeNull();
    expect(previous.activeStep).toBe("release");
    expect(previous.trackTab).toBe("certificate");
    expect(previous.scanResult).not.toBeNull();
    expect(previous.draftDirty).toBe(true);
  });

  it("ignores delegated backdrop actions for clicks inside a modal", () => {
    expect(shouldIgnoreModalBackdropClick(true, false)).toBe(true);
    expect(shouldIgnoreModalBackdropClick(true, true)).toBe(false);
    expect(shouldIgnoreModalBackdropClick(false, false)).toBe(false);
  });

  it("keeps finalized snapshots navigable while requiring an explicit revision for edits", () => {
    expect(finalizedTrackPresentation({
      status: "FINALIZED",
      certificate: { valid: true, certificateId: "SDM-2026-TEST" }
    })).toEqual({
      title: "Finalisierter Snapshot – nur lesbar",
      message: expect.stringContaining("Navigation und reine Prüfungen bleiben verfügbar"),
      actionLabel: "Neue Revision anlegen und bearbeiten",
      invalid: false
    });
    expect(finalizedTrackPresentation({
      status: "FINALIZED",
      certificate: { valid: false, certificateId: "SDM-2026-TEST" }
    })).toEqual(expect.objectContaining({
      actionLabel: "Neue Revision anlegen und bearbeiten",
      invalid: true
    }));
    expect(finalizedTrackPresentation({ status: "ACTIVE", certificate: { valid: false } })).toBeNull();

    const superseded = finalizedTrackPresentation({
      status: "SUPERSEDED",
      certificate: { valid: true, certificateId: "SDM-2026-OLD" }
    });
    expect(superseded).toEqual(expect.objectContaining({
      title: "Ersetzter Snapshot – nur lesbar",
      message: expect.stringContaining("Navigation und reine Prüfungen bleiben verfügbar")
    }));
    expect(superseded).not.toHaveProperty("actionLabel");

    expect(isTrackContentLocked("FINALIZED")).toBe(true);
    expect(isTrackContentLocked("SUPERSEDED")).toBe(true);
    expect(isTrackContentLocked("ACTIVE")).toBe(false);
    expect(canCreateTrackRevision("FINALIZED")).toBe(true);
    expect(canCreateTrackRevision("SUPERSEDED")).toBe(false);
    expect(shouldDiscardLockedDraft("FINALIZED", true)).toBe(true);
    expect(shouldDiscardLockedDraft("FINALIZED", false)).toBe(false);
    expect(shouldDiscardLockedDraft("SUPERSEDED", true)).toBe(true);
    expect(shouldDiscardLockedDraft("SUPERSEDED", false)).toBe(false);
    expect(shouldDiscardLockedDraft("ACTIVE", true)).toBe(false);
  });

  it("maps real native work counters into honest operation progress", () => {
    expect(operationProgressPercent("hashes", {
      stage: "hashing", processedBytes: 500, totalBytes: 1_000, processedFiles: 1, totalFiles: 2
    })).toBe(29);
    expect(operationProgressPercent("hashes", {
      stage: "verifying", processedBytes: 500, totalBytes: 1_000, processedFiles: 1, totalFiles: 2
    })).toBe(78);
    expect(operationProgressPercent("verification", {
      stage: "verifying", processedBytes: 750, totalBytes: 1_000, processedFiles: 3, totalFiles: 4
    })).toBe(72);
    expect(operationProgressPercent("documents", {
      stage: "writing_documents", processedBytes: 0, totalBytes: 0, processedFiles: 4, totalFiles: 8
    })).toBe(56);
    expect(operationProgressPercent("documents", {
      stage: "complete", processedBytes: 0, totalBytes: 0, processedFiles: 8, totalFiles: 8
    })).toBe(100);
    expect(operationProgressPercent("finalization", {
      stage: "verifying", processedBytes: 500, totalBytes: 1_000, processedFiles: 5, totalFiles: 10
    })).toBe(76);
    expect(operationProgressPercent("finalization", {
      stage: "saving_final_snapshot", processedBytes: 1_000, totalBytes: 1_000, processedFiles: 10, totalFiles: 10
    })).toBe(97);
    expect(operationProgressPercent("audio_screening", {
      stage: "fingerprinting_audio", processedBytes: 500, totalBytes: 1_000, processedFiles: 0, totalFiles: 1
    })).toBe(30);
    expect(operationProgressPercent("audio_screening", {
      stage: "saving_screening_result", processedBytes: 1_000, totalBytes: 1_000, processedFiles: 1, totalFiles: 1
    })).toBe(96);
    expect(operationStageLabel("comparing_hashes")).toBe("Ergebnisse werden verglichen");
    expect(operationStageLabel("generating_certificate")).toBe("Zertifikat und Manifest entstehen");
    expect(operationStageLabel("preparing_audio", "audio_screening")).toBe("Audio wird vorbereitet");
    expect(operationStageLabel("fingerprinting_audio", "audio_screening")).toBe("Lokaler Audio-Fingerprint wird erzeugt");
    expect(operationStageLabel("fingerprint_complete", "audio_screening")).toBe("Chromaprint-Fingerprint abgeschlossen");
    expect(operationStageLabel("preparing_external_check", "audio_screening")).toBe("Externe Katalogprüfung wird vorbereitet");
    expect(operationStageLabel("sending_provider_request", "audio_screening")).toBe("Audioausschnitt wird übertragen");
    expect(operationStageLabel("waiting_provider_response", "audio_screening")).toBe("ACRCloud-Ergebnis wird erwartet");
    expect(operationStageLabel("processing_provider_response", "audio_screening")).toBe("Provider-Ergebnis wird geprüft");
    expect(operationStageLabel("saving_screening_result", "audio_screening")).toBe("Prüfergebnis wird lokal dokumentiert");
    expect(operationStageLabel("complete", "audio_screening")).toBe("Prüfung abgeschlossen");
  });

  it("keeps a changed library assignment when applying a track detail", () => {
    const summary = trackSummaryFromDetail({
      id: "track-1",
      title: "Album Track",
      relativePath: "album-track",
      library: { section: "album", albumTitle: "Northern Lights" },
      status: "FINALIZED",
      updatedAt: "2026-08-14T10:00:00Z",
      progress: 100,
      missingCount: 0,
      coverEvidenceId: "artwork-1",
      certificate: { valid: true }
    } as Parameters<typeof trackSummaryFromDetail>[0]);

    expect(summary.library).toEqual({ section: "album", albumTitle: "Northern Lights" });
    expect(summary.certificateValid).toBe(true);
    expect(summary.coverEvidenceId).toBe("artwork-1");
  });

  it("presents an explicit action without rewriting a finalized older workflow", () => {
    const presentation = workflowUpgradePresentation(
      {
        status: "FINALIZED",
        workflowId: "suno-track",
        workflowVersion: "1.0",
        certificate: { valid: true, workflowVersion: "1.0" }
      },
      { id: "suno-track", version: "1.7" }
    );

    expect(presentation).toEqual({
      message: "Finalized with workflow suno-track 1.0 / Current workflow suno-track 1.7",
      action: "re-evaluate-track"
    });
    expect(workflowUpgradePresentation(
      {
        status: "FINALIZED",
        workflowId: "suno-track",
        workflowVersion: "1.7",
        certificate: { valid: true, workflowVersion: "1.7" }
      },
      { id: "suno-track", version: "1.7" }
    )).toBeNull();

    expect(workflowUpgradePresentation(
      {
        status: "SUPERSEDED",
        workflowId: "suno-track",
        workflowVersion: "1.0",
        certificate: { valid: true, workflowVersion: "1.0" }
      },
      { id: "suno-track", version: "1.7" }
    )).toEqual({
      message: "Superseded snapshot uses workflow suno-track 1.0 / Current workflow suno-track 1.7"
    });
  });

  it("blocks finalization until an outdated workflow is explicitly re-evaluated", () => {
    const track = {
      status: "ACTIVE" as const,
      workflowId: "suno-track",
      workflowVersion: "1.0",
      certificate: { valid: false }
    };

    expect(workflowUpgradeFinalizationBlocker(track, { id: "suno-track", version: "1.7" }))
      .toContain("ausdrücklich mit dem aktuellen Workflow neu bewertet");
    expect(workflowUpgradeFinalizationBlocker(
      { ...track, workflowVersion: "1.7" },
      { id: "suno-track", version: "1.7" }
    )).toBeNull();
  });
});
