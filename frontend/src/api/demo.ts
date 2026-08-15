import {
  calculateMissingRequirements,
  calculateProgress,
  evaluateRequirements,
  finalizationGate,
  stepStatuses,
  WORKFLOW_ID,
  WORKFLOW_VERSION
} from "../domain/workflow";
import { subscriptionCoverageEnd } from "../domain/subscription";
import { trackLibraryAssignment } from "../domain/track-library";
import {
  emptyProfile,
  emptyTrackFields,
  type ActionResult,
  type EvidenceItem,
  type EvidenceRole,
  type GlobalProfile,
  type GlobalEvidenceItem,
  type ScanResult,
  type StepId,
  type StepStatus,
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

function trackFolderName(title: string): string {
  return title.trim();
}

function trackRelativePath(library: TrackLibraryAssignment, title: string): string {
  const parent = library.section === "album" ? library.albumTitle!.trim() : "Singles";
  return `${parent}/${trackFolderName(title)}`;
}

function evidence(role: EvidenceRole, fileName: string): EvidenceItem {
  return {
    id: crypto.randomUUID(),
    role,
    fileName,
    relativePath: `evidence/${fileName}`,
    sha256: "7d8f35b868a8f3d4e5b4a2f331f4b0ddc6be8365d932b838b42ec86e09721690",
    sizeBytes: 8_476_231,
    importedAt: now(),
    verified: true,
    provenance: "managed_copy"
  };
}

function makeTrack(
  id: string,
  title: string,
  profile: GlobalProfile,
  complete = false,
  library: TrackLibraryAssignment = { section: "single" }
): TrackDetail {
  const fields = {
    ...emptyTrackFields(profile),
    title,
    productionStartDate: "2026-07-18",
    productionEndDate: complete ? "2026-07-24" : "",
    sunoModel: complete ? "v4.5" : "",
    sunoProjectUrl: complete ? "https://suno.com/song/demo-project" : "",
    sunoPlanAtCreation: "Premier",
    finalExportDate: complete ? "2026-07-24" : "",
    lyricsSource: complete ? ("human" as const) : ("" as const),
    lyricsText: complete ? "Eigene Lyrics – im Track-Dokument vollständig gespeichert." : "",
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
    humanArtworkModifications: complete ? "Typografie und Farbkorrektur" : "",
    depictsRealPerson: complete ? false : null,
    depictsRealEvent: complete ? false : null,
    containsTrademark: complete ? false : null,
    disclosureApplied: complete,
    disclosureText: "AI-assisted"
  };
  const originalArtwork = evidence("ai_artwork_original", `${title}_AI_ORIGINAL.png`);
  const disclosedArtwork: EvidenceItem = {
    ...evidence("ai_artwork_edited", `${title}_AI_EDITED.png`),
    provenance: "generated_disclosure",
    derivedFromEvidenceId: originalArtwork.id,
    generatorVersion: "local-disclosure-v1",
    generatedDisclosureText: fields.disclosureText
  };
  const items = complete
    ? [
        evidence("release_wav", `${title}_FINAL.wav`),
        evidence("suno_final_export", `${title}_SUNO_FINAL.wav`),
        { ...evidence("subscription_payment", "subscription_2026-07.pdf"), provenance: "global_copy" as const, sourceGlobalEvidenceId: "demo-global-subscription", coverageStart: "2026-07-01", coverageEnd: "2026-07-31" },
        originalArtwork,
        disclosedArtwork,
        evidence("final_artwork", `${title}_FINAL.jpeg`)
      ]
    : [evidence("suno_screenshot", `${title}_SUNO.png`)];
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
    fields,
    steps: [],
    evidence: items,
    documents: {
      generated: complete,
      current: complete,
      generatedAt: complete ? now() : undefined,
      templateVersion: "1.2",
      files: complete ? ["02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md"] : []
    },
    integrity: {
      generated: complete,
      verified: complete,
      fileCount: complete ? 17 : 0,
      verifiedCount: complete ? 17 : 0,
      mismatchFiles: []
    },
    certificate: { valid: false }
  };
  refresh(track);
  return track;
}

function refresh(track: TrackDetail): void {
  track.title = track.fields.title;
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
  const result = (track: TrackDetail, message: string): ActionResult => ({ message, track: clone(track) });

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
          { id: "release", number: "07", label: "Release", description: "Finaler Export und Release-Dateien", required: true },
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
        track.profileSnapshot = clone(profile);
        track.documents.current = false;
        refresh(track);
      }
      return clone(profile);
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
    async removeGlobalEvidence(evidenceId) {
      await wait();
      globalEvidence = globalEvidence.filter((item) => item.id !== evidenceId);
    },
    async attachGlobalEvidence(trackId, evidenceId) {
      await wait();
      const track = get(trackId);
      const item = globalEvidence.find((entry) => entry.id === evidenceId);
      if (!item) throw new Error("Der globale Nachweis wurde nicht gefunden.");
      track.evidence.push({ ...clone(item), id: crypto.randomUUID(), sourceGlobalEvidenceId: item.id, relativePath: `04_LICENSES/${item.fileName}` });
      refresh(track);
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
      refresh(track);
      tracks.set(id, track);
      if (workspace) workspace.trackCount = tracks.size;
      return clone(track);
    },
    async loadTrack(trackId: string) {
      await wait();
      return clone(get(trackId));
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
      const track = get(trackId);
      track.fields = { ...track.fields, ...clone(patch) };
      if (patch.title !== undefined) {
        track.relativePath = trackRelativePath(track.library, patch.title);
      }
      track.documents.current = false;
      if (track.status === "FINALIZED") {
        track.certificate.valid = false;
        track.certificate.invalidationReason = "Dokumentation nach Finalisierung geändert";
        track.integrity.mismatchFiles = ["03_DOCUMENTATION/README.md"];
      }
      refresh(track);
      return clone(track);
    },
    async adoptLegacyProfile(trackId) {
      await wait();
      const track = get(trackId);
      track.profileSnapshot = clone(profile);
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async addDeviation(trackId, description, blocking) {
      await wait();
      const track = get(trackId);
      track.blockingDeviations ??= [];
      track.blockingDeviations.push({ id: crypto.randomUUID(), title: blocking ? "Blockierende Abweichung" : "Hinweis", description, blocking, resolved: false, createdAt: now() });
      refresh(track);
      return clone(track);
    },
    async resolveDeviation(trackId, deviationId) {
      await wait();
      const track = get(trackId);
      const deviation = track.blockingDeviations?.find((item) => item.id === deviationId);
      if (deviation) Object.assign(deviation, { resolved: true, resolvedAt: now() });
      refresh(track);
      return clone(track);
    },
    async removeDeviation(trackId, deviationId) {
      await wait();
      const track = get(trackId);
      track.blockingDeviations = track.blockingDeviations?.filter((item) => item.id !== deviationId);
      refresh(track);
      return clone(track);
    },
    async setStepStatus(trackId, stepId: StepId, status: StepStatus, naReason?: string) {
      await wait();
      const track = get(trackId);
      const current = track.steps.find((step) => step.id === stepId);
      if (current) Object.assign(current, { status, naReason, updatedAt: now() });
      else track.steps.push({ id: stepId, status, naReason, updatedAt: now() });
      refresh(track);
      return clone(track);
    },
    async importEvidence(trackId, role, replaceEvidenceId) {
      await wait();
      const track = get(trackId);
      if (!replaceEvidenceId && ["release_wav", "final_artwork"].includes(role) && track.evidence.some((item) => item.role === role)) {
        throw new Error(`Die Rolle ${role} ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum Ersetzen.`);
      }
      const extension = role.includes("artwork") || role === "suno_screenshot" || role === "final_artwork"
        ? "png"
        : role === "release_wav" || role === "suno_final_export"
          ? "wav"
          : role === "source_code_file"
            ? "py"
          : role.includes("subscription")
            ? "pdf"
            : "zip";
      const next = evidence(role, `${role}.${extension}`);
      const replaceIndex = replaceEvidenceId
        ? track.evidence.findIndex((item) => item.id === replaceEvidenceId && item.role === role)
        : -1;
      if (replaceEvidenceId && replaceIndex < 0) throw new Error("Die zu ersetzende Evidence wurde nicht gefunden.");
      if (replaceIndex >= 0) track.evidence[replaceIndex] = { ...next, id: replaceEvidenceId! };
      else track.evidence.push(next);
      track.documents.current = false;
      refresh(track);
      return clone(track);
    },
    async removeEvidence(trackId, evidenceId) {
      await wait();
      const track = get(trackId);
      track.evidence = track.evidence.filter((item) => item.id !== evidenceId);
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
    async generateDocuments(trackId) {
      await wait();
      const track = get(trackId);
      track.documents = {
        generated: true,
        current: true,
        generatedAt: now(),
        templateVersion: "1.2",
        files: ["02_SUNO/suno_project.txt", "02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md", "04_LICENSES/suno_account_and_license.md", "04_LICENSES/openai_image_generation.md", "05_ARTWORK/artwork_process.md"]
      };
      track.integrity.generated = false;
      track.integrity.verified = false;
      refresh(track);
      return result(track, "8 Dokumente wurden deterministisch erzeugt.");
    },
    async generateArtworkDisclosure(trackId, disclosureText) {
      await wait();
      const track = get(trackId);
      track.fields.disclosureApplied = true;
      if (disclosureText) track.fields.disclosureText = disclosureText;
      const source = [...track.evidence].reverse().find((item) => item.role === "ai_artwork_original" && item.verified);
      if (!source) throw new Error("Importiere zuerst das unveränderte KI-Artwork.");
      if (!track.evidence.some((item) => item.role === "ai_artwork_edited")) {
        track.evidence.push({
          ...evidence("ai_artwork_edited", `${track.title}_AI_EDITED.png`),
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
    async calculateHashes(trackId) {
      await wait();
      const track = get(trackId);
      if (!track.documents.current) throw new Error("Erzeuge zuerst die aktuellen Dokumente.");
      track.integrity = { generated: true, verified: false, fileCount: track.evidence.length + track.documents.files.length, verifiedCount: 0, generatedAt: now(), mismatchFiles: [] };
      refresh(track);
      return result(track, `${track.integrity.fileCount} Dateien wurden gehasht.`);
    },
    async verifyHashes(trackId) {
      await wait();
      const track = get(trackId);
      if (!track.integrity.generated) throw new Error("Erzeuge zuerst SHA-256-Prüfsummen.");
      track.integrity.verified = true;
      track.integrity.verifiedCount = track.integrity.fileCount;
      track.integrity.verifiedAt = now();
      track.integrity.mismatchFiles = [];
      refresh(track);
      return result(track, `${track.integrity.verifiedCount} von ${track.integrity.fileCount} Dateien erfolgreich verifiziert.`);
    },
    async validateTrack(trackId): Promise<ValidationResult> {
      await wait();
      const track = get(trackId);
      return finalizationGate(track, track.profileSnapshot);
    },
    async finalizeTrack(trackId) {
      await wait();
      const track = get(trackId);
      const gate = finalizationGate(track, track.profileSnapshot);
      if (!gate.valid) throw new Error(`Finalisierung blockiert: ${[...gate.missingItems, ...gate.blockingItems].join(", ")}`);
      track.status = "FINALIZED";
      track.certificate = { valid: true, certificateId: `SDM-${new Date().getFullYear()}-${track.id.slice(0, 8).toUpperCase()}`, finalizedAt: now(), workflowVersion: WORKFLOW_VERSION };
      refresh(track);
      track.status = "FINALIZED";
      return result(track, "Dokumentation finalisiert und Zertifikat erzeugt.");
    },
    async invalidateCertificate(trackId) {
      await wait();
      const track = get(trackId);
      track.certificate.valid = false;
      track.certificate.invalidatedAt = now();
      track.certificate.invalidationReason = "Manuell invalidiert";
      return result(track, "Das Zertifikat wurde als ungültig markiert.");
    },
    async createRevision(trackId) {
      await wait();
      const track = get(trackId);
      track.status = "ACTIVE";
      track.certificate = { valid: false };
      track.integrity.generated = false;
      track.integrity.verified = false;
      track.integrity.mismatchFiles = [];
      track.documents.current = false;
      refresh(track);
      return result(track, "Der bisherige Snapshot wurde archiviert und eine neue Revision angelegt.");
    },
    async reEvaluateTrack(trackId) {
      await wait();
      const track = get(trackId);
      if (track.workflowId === WORKFLOW_ID && track.workflowVersion === WORKFLOW_VERSION) {
        throw new Error("Der Track verwendet bereits die aktuelle Workflow-Version.");
      }
      const archived = track.status === "FINALIZED";
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
