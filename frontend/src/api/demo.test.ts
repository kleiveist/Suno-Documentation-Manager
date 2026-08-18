import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "./demo";

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

async function finalizeGravity(api: ReturnType<typeof createDemoApi>, bilingual = false) {
  await settle(api.updateTrack("gravity", {
    sunoExportFilenameDifferenceConfirmed: true
  }));
  await settle(api.generateDocuments("gravity", false));
  await settle(api.calculateHashes("gravity"));
  await settle(api.verifyHashes("gravity"));
  return settle(api.finalizeTrack("gravity", { bilingual }));
}

async function configureFreeTsa(api: ReturnType<typeof createDemoApi>, autoAfterFinalization = false) {
  const settings = await settle(api.getTimestampSettings());
  return settle(api.updateTimestampSettings({
    ...settings,
    custom: { ...settings.custom },
    enabled: true,
    provider: "free_tsa",
    autoAfterFinalization
  }));
}

afterEach(() => vi.useRealTimers());

describe("demo track library", () => {
  it("keeps editable track documents and hashes current when only certificate language changes", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));
    const profile = await settle(api.getProfile());

    await settle(api.updateProfile({ ...profile, certificateLanguage: "de" }));

    const after = await settle(api.loadTrack("gravity"));
    expect(after.profileSnapshot.certificateLanguage).toBe("en");
    expect(after.documents).toEqual(before.documents);
    expect(after.integrity).toEqual(before.integrity);
  });

  it("records the selected primary language and bilingual switch at finalization", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const profile = await settle(api.getProfile());
    await settle(api.updateProfile({ ...profile, certificateLanguage: "de" }));

    const finalized = await finalizeGravity(api, true);

    expect(finalized.track?.certificate).toEqual(expect.objectContaining({
      certificateLanguage: "de",
      bilingual: true
    }));
  });

  it("does not prefill a new track's generation plan from the global profile", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const created = await settle(api.createTrack({
      title: "Fresh Plan",
      productionStartDate: "2026-08-17",
      commercialUseIntended: false,
      library: { section: "single" }
    }));

    expect(created.profileSnapshot.sunoPlan).toBe("Premier");
    expect(created.fields.sunoPlanAtGeneration).toBe("");
    expect(created.fields.legacySunoPlanAtCreation).toBe("");
  });

  it("creates and renames an album before it contains a track", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    expect(await settle(api.createAlbum("  Empty Album  "))).toContain("Empty Album");
    await settle(api.renameAlbum("Empty Album", "Future Album"));

    expect(await settle(api.listAlbums())).toContain("Future Album");
    expect(await settle(api.listAlbums())).not.toContain("Empty Album");
  });

  it("reclassifies a track by moving its folder without changing documentation state", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));

    const updated = await settle(api.updateTrackLibrary("gravity", {
      section: "album",
      albumTitle: "  Northern Lights  "
    }));
    const { library: _beforeLibrary, relativePath: _beforePath, ...beforeState } = before;
    const { library: _updatedLibrary, relativePath: _updatedPath, ...updatedState } = updated;

    expect(updated.library).toEqual({ section: "album", albumTitle: "Northern Lights" });
    expect(updated.relativePath).toBe("Northern Lights/Gravity");
    expect(updatedState).toEqual(beforeState);
  });

  it("renames an album folder and every contained track path", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const tracks = await settle(api.renameAlbum("Event Horizon", "Gravity Drift"));
    const gravity = tracks.find((track) => track.id === "gravity");

    expect(gravity?.library).toEqual({ section: "album", albumTitle: "Gravity Drift" });
    expect(gravity?.relativePath).toBe("Gravity Drift/Gravity");
  });

  it("uses the same album-title validation boundary as the native adapter", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const update = api.updateTrackLibrary("gravity", {
      section: "album",
      albumTitle: "x".repeat(201)
    });
    const rejected = expect(update).rejects.toThrow("Albumtitel");
    await vi.runAllTimersAsync();
    await rejected;
  });
});

describe("demo evidence controls", () => {
  it("models Suno-WAV metadata as automatic generation, production, download and no-editing facts", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", { postExportEditingPerformed: false }));
    await settle(api.importEvidence("cosmic-pulse", "release_wav"));

    const updated = await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));
    const suno = updated!.evidence.find((item) => item.role === "suno_final_export")!;

    expect(suno.metadata).toEqual(expect.objectContaining({
      sunoStudioDetected: true,
      sunoCreatedTimestamp: "2026-08-17T06:38:06Z",
      sunoCreatedDate: "2026-08-17",
      sunoId: "6c8a40fd-32bf-4c7b-ab59-23579ff95828"
    }));
    expect(updated!.fields.sunoFinalGenerationDate).toBe("2026-08-17");
    expect(updated!.fields.sunoFinalGenerationId).toBe("6c8a40fd-32bf-4c7b-ab59-23579ff95828");
    expect(updated!.fields.productionEndDate).toBe("2026-08-17");
    expect(updated!.fields.sunoDownloadExportDate).toBe("2026-08-17");
    expect(updated!.fields.finalExportDate).toBe("2026-08-17");
    expect(updated!.automation).toEqual(expect.objectContaining({
      finalGenerationIdOrigin: "evidence_derived_metadata",
      finalGenerationOrigin: "evidence_derived_metadata",
      productionEndOrigin: "evidence_derived_metadata",
      downloadExportOrigin: "evidence_derived_metadata",
      finalExportOrigin: "evidence_derived_metadata",
      sunoMetadataDetected: true,
      releaseIdenticalToSunoExport: true
    }));
  });

  it("never replaces a pre-existing manual final generation ID", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", {
      sunoFinalGenerationId: "manual-generation-id"
    }));

    const imported = await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));

    expect(imported!.fields.sunoFinalGenerationId).toBe("manual-generation-id");
    expect(imported!.automation.finalGenerationIdOrigin).toBe("user_confirmed_fact");
  });

  it("uses a manual last-editing date after confirmed desktop editing", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", { postExportEditingPerformed: false }));
    await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));

    const edited = await settle(api.updateTrack("cosmic-pulse", {
      postExportEditingPerformed: true,
      postExportEditingDetails: "Mastering",
      finalExportDate: "2026-08-19"
    }));

    expect(edited.fields.finalExportDate).toBe("2026-08-19");
    expect(edited.automation.finalExportOrigin).toBe("user_confirmed_fact");
    expect(edited.fields.sunoDownloadExportDate).toBe("2026-08-17");
    expect(edited.automation.downloadExportOrigin).toBe("evidence_derived_metadata");
  });

  it("replaces fallback dates with metadata and keeps both dates automatic after editing", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", {
      sunoFinalGenerationDate: "2026-08-16",
      postExportEditingPerformed: false
    }));
    const imported = await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));

    expect(imported!.fields.sunoFinalGenerationDate).toBe("2026-08-17");
    expect(imported!.fields.productionEndDate).toBe("2026-08-17");
    expect(imported!.automation.consistencyIssues).toEqual([]);

    const edited = await settle(api.updateTrack("cosmic-pulse", {
      postExportEditingPerformed: true,
      postExportEditingDetails: "Mastering"
    }));
    expect(edited.fields.productionEndDate).toBe("2026-08-17");
    expect(edited.automation.productionEndOrigin).toBe("evidence_derived_metadata");
  });

  it("rejects submitted date overrides while valid Suno metadata exists", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", { postExportEditingPerformed: false }));
    await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));

    const patched = await settle(api.updateTrack("cosmic-pulse", {
      sunoFinalGenerationDate: "2026-08-16",
      productionEndDate: "2026-08-18"
    }));

    expect(patched.fields.sunoFinalGenerationDate).toBe("2026-08-17");
    expect(patched.fields.productionEndDate).toBe("2026-08-17");
    expect(patched.automation.finalGenerationOrigin).toBe("evidence_derived_metadata");
    expect(patched.automation.productionEndOrigin).toBe("evidence_derived_metadata");
    expect(patched.automation.consistencyIssues).toEqual([]);

    const edited = await settle(api.updateTrack("cosmic-pulse", {
      postExportEditingPerformed: true,
      postExportEditingDetails: "Mastering",
      productionEndDate: "2026-08-19"
    }));
    expect(edited.fields.productionEndDate).toBe("2026-08-17");
    expect(edited.automation.productionEndOrigin).toBe("evidence_derived_metadata");
  });

  it("keeps the metadata date authoritative when other track dates change", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await settle(api.updateTrack("cosmic-pulse", { postExportEditingPerformed: false }));
    const imported = await settle(api.importEvidence("cosmic-pulse", "suno_final_export"));
    expect(imported!.automation.productionEndOrigin).toBe("evidence_derived_metadata");

    const updated = await settle(api.updateTrack("cosmic-pulse", {
      productionStartDate: "2026-08-10"
    }));

    expect(updated.fields.productionEndDate).toBe("2026-08-17");
    expect(updated.automation.productionEndOrigin).toBe("evidence_derived_metadata");
  });

  it("accepts manual date fallbacks only when no metadata date exists", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const updated = await settle(api.updateTrack("cosmic-pulse", {
      sunoFinalGenerationDate: "2026-08-16",
      productionEndDate: "2026-08-18"
    }));

    expect(updated.fields.sunoFinalGenerationDate).toBe("2026-08-16");
    expect(updated.fields.productionEndDate).toBe("2026-08-18");
    expect(updated.automation.finalGenerationOrigin).toBe("user_confirmed_fact");
    expect(updated.automation.productionEndOrigin).toBe("user_confirmed_fact");
  });

  it("replaces one selected record and previews present image evidence", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));
    const original = before.evidence.find((item) => item.role === "ai_artwork_original")!;

    const updated = await settle(api.importEvidence("gravity", "ai_artwork_original", original.id));
    const replacement = updated!.evidence.find((item) => item.id === original.id)!;
    const preview = await settle(api.previewEvidence("gravity", replacement.id));

    expect(updated!.evidence.filter((item) => item.id === original.id)).toHaveLength(1);
    expect(preview.dataUrl).toMatch(/^data:image\/png;base64,/);
  });

  it("registers Suno terms globally and copies them into existing and new projects", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const global = await settle(api.importGlobalTermsEvidence({
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17",
      sourceUrl: "https://suno.com/terms"
    }));
    const existing = await settle(api.loadTrack("gravity"));
    const created = await settle(api.createTrack({
      title: "Later Project",
      productionStartDate: "2026-08-16",
      commercialUseIntended: false,
      library: { section: "single" }
    }));

    for (const track of [existing, created]) {
      expect(track.evidence).toEqual(expect.arrayContaining([
        expect.objectContaining({
          role: "suno_terms_rights",
          sourceGlobalEvidenceId: global!.id,
          provenance: "global_copy",
          metadata: expect.objectContaining({
            originalFileName: "suno_terms.pdf",
            documentTitle: "Suno Terms of Service",
            provider: "Suno, Inc.",
            retrievalDate: "2026-08-17"
          })
        })
      ]));
    }
  });

  it("rejects incomplete Terms metadata and propagates later metadata edits only to editable copies", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const rejected = expect(api.importGlobalTermsEvidence({ documentTitle: "", provider: "", retrievalDate: "" }))
      .rejects.toThrow("Dokumenttitel");
    await vi.runAllTimersAsync();
    await rejected;

    const global = await settle(api.importGlobalTermsEvidence({
      documentTitle: "Suno Terms",
      provider: "Suno",
      retrievalDate: "2026-08-17"
    }));
    await settle(api.updateGlobalTermsEvidenceMetadata(global!.id, {
      documentTitle: "Suno Terms of Service",
      provider: "Suno, Inc.",
      retrievalDate: "2026-08-17",
      applicableProductionPeriod: "2026"
    }));

    const updated = await settle(api.loadTrack("gravity"));
    expect(updated.evidence.find((item) => item.sourceGlobalEvidenceId === global!.id)?.metadata)
      .toEqual(expect.objectContaining({ documentTitle: "Suno Terms of Service", applicableProductionPeriod: "2026" }));
  });
});

describe("demo external timestamp attachment", () => {
  it("uses the configured provider and the evidence-manifest anchor automatically after finalization", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const finalized = await finalizeGravity(api);
    expect(finalized.track!.externalTimestampSummary).toEqual(expect.objectContaining({ status: "not_recorded" }));
    expect(finalized.track!.finalizationAnchors).toEqual([
      {
        artifact: "evidence_manifest",
        label: "Evidence manifest (recommended timestamp anchor)",
        relativePath: "06_CERTIFICATE/EVIDENCE_MANIFEST.json",
        sha256: "a".repeat(64)
      },
      {
        artifact: "sha256sums",
        label: "Track SHA-256 manifest",
        relativePath: "03_DOCUMENTATION/SHA256SUMS.txt",
        sha256: "b".repeat(64)
      },
      {
        artifact: "documentation_certificate_markdown",
        label: "Documentation certificate (Markdown)",
        relativePath: "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md",
        sha256: "c".repeat(64)
      },
      {
        artifact: "certificate_pdf",
        label: "Documentation certificate (PDF)",
        relativePath: "SunoDM_DOCUMENTATION_CERTIFICATE.pdf",
        sha256: "d".repeat(64)
      },
      {
        artifact: "final_evidence_package",
        label: "Final evidence package certificate hash set",
        relativePath: "06_CERTIFICATE/CERTIFICATE_SHA256.txt",
        sha256: "e".repeat(64)
      }
    ]);
    const anchor = finalized.track!.finalizationAnchors.find((item) => item.artifact === "evidence_manifest")!;

    await configureFreeTsa(api);
    const timestamped = await settle(api.attachExternalTimestamp("gravity"));

    const record = timestamped.externalTimestamps[0];
    const sidecarDirectory = `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${record.id}`;
    expect(record).toEqual(expect.objectContaining({
      certificateId: finalized.track!.certificate.certificateId,
      provider: "FreeTSA",
      timestampType: "external_integrity_timestamp",
      referencedArtifact: "evidence_manifest",
      referencedHashMatch: true,
      actualSha256: anchor.sha256,
      provenance: "Automatic provider response; structural and digest checks",
      recordRelativePath: `${sidecarDirectory}/TIMESTAMP_RECORD.json`,
      markdownRelativePath: `${sidecarDirectory}/EXTERNAL_TIMESTAMP_ADDENDUM.md`,
      pdfRelativePath: `${sidecarDirectory}/EXTERNAL_TIMESTAMP_ADDENDUM.pdf`,
      hashListRelativePath: `${sidecarDirectory}/TIMESTAMP_RECORD_SHA256.txt`,
      integrityVerified: true,
      integrityIssues: [],
      providerMetadata: expect.objectContaining({
        protocol: "RFC 3161",
        providerResponseFileName: "TIMESTAMP_RESPONSE.tsr",
        providerResponseSha256: "f".repeat(64),
        providerDigestMatch: true,
        verificationResult: "attached",
        signatureVerified: null,
        trustChainVerified: null
      })
    }));
    expect(timestamped.externalTimestampSummary).toEqual(expect.objectContaining({
      status: "attached",
      provider: "FreeTSA",
      recordId: record.id
    }));

    const revision = await settle(api.createRevision("gravity"));
    expect(revision.track!.externalTimestamps).toEqual([]);
    expect(revision.track!.externalTimestampSummary).toEqual(expect.objectContaining({ status: "not_recorded" }));
    expect(revision.track!.finalizationAnchors).toEqual([]);
  });

  it("keeps technical finalization complete when no provider is configured", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const finalized = await finalizeGravity(api);

    expect(finalized.track).toEqual(expect.objectContaining({ status: "FINALIZED" }));
    expect(finalized.track!.certificate.valid).toBe(true);
    expect(finalized.track!.externalTimestampSummary).toEqual(expect.objectContaining({ status: "not_recorded" }));

    const attempted = await settle(api.attachExternalTimestamp("gravity"));
    expect(attempted.externalTimestamps).toEqual([]);
    expect(attempted.externalTimestampSummary).toEqual(expect.objectContaining({ status: "provider_unavailable" }));
  });

  it("can attach automatically after phase-one finalization without creating a duplicate", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await configureFreeTsa(api, true);

    const finalized = await finalizeGravity(api);
    expect(finalized.track!.status).toBe("FINALIZED");
    expect(finalized.track!.externalTimestampSummary).toEqual(expect.objectContaining({ status: "attached" }));
    expect(finalized.track!.externalTimestamps).toHaveLength(1);

    const retry = await settle(api.attachExternalTimestamp("gravity"));
    expect(retry.externalTimestamps).toHaveLength(1);
    expect(retry.externalTimestampSummary).toEqual(expect.objectContaining({ status: "attached" }));
  });
});

describe("demo revision protection", () => {
  it("requires a finalized snapshot before creating a revision", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const before = await settle(api.loadTrack("gravity"));

    const revision = api.createRevision("gravity");
    const rejected = expect(revision).rejects.toThrow("Nur ein finalisierter Track");
    await vi.runAllTimersAsync();
    await rejected;
    expect(await settle(api.loadTrack("gravity"))).toEqual(before);

    const finalized = await finalizeGravity(api);
    expect(finalized.track?.status).toBe("FINALIZED");
    const opened = await settle(api.createRevision("gravity"));
    expect(opened.track?.status).toBe("ACTIVE");
    expect(opened.track?.certificate.valid).toBe(false);
  });

  it("rejects content mutations while a finalized snapshot is locked", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    await finalizeGravity(api);
    const before = await settle(api.loadTrack("gravity"));

    const mutations = [
      () => api.updateTrack("gravity", { title: "Changed" }),
      () => api.addDeviation("gravity", "late change", true),
      () => api.importEvidence("gravity", "source_code_file"),
      () => api.generateDocuments("gravity", false),
      () => api.calculateHashes("gravity")
    ];
    for (const mutate of mutations) {
      const mutation = mutate();
      const rejected = expect(mutation).rejects.toThrow("neue Revision");
      await vi.runAllTimersAsync();
      await rejected;
    }

    const after = await settle(api.loadTrack("gravity"));
    expect(after.fields).toEqual(before.fields);
    expect(after.evidence).toEqual(before.evidence);
    expect(after.documents).toEqual(before.documents);
    expect(after.integrity).toEqual(before.integrity);
    expect(after.certificate).toEqual(before.certificate);
    expect(after.status).toBe("FINALIZED");
  });
});
