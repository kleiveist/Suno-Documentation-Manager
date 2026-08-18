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
  emptyProfile,
  emptyTrackAutomation,
  emptyTrackFields,
  type ActionResult,
  type ByteIdenticalPair,
  type ConsistencyIssue,
  type EvidenceItem,
  type EvidenceMetadata,
  type ExternalTimestampInput,
  type FolderImportExecutionInput,
  type FolderImportProposal,
  type EvidenceRole,
  type FactOrigin,
  type GlobalProfile,
  type GlobalEvidenceItem,
  type OperationProgress,
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
    sunoLyricsFieldContent: complete ? true : null,
    sunoLyricsContentTypes: complete ? ["vocal_lyrics"] : [],
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
        evidence("release_wav", `${title}.wav`),
        sunoEvidence(`${title}_SUNO_FINAL.wav`),
        { ...evidence("subscription_payment", "subscription_2026-07.pdf"), provenance: "global_copy" as const, sourceGlobalEvidenceId: "demo-global-subscription", coverageStart: "2026-07-01", coverageEnd: "2026-07-31" },
        originalArtwork,
        disclosedArtwork,
        evidence("final_artwork", `${title}_FINAL.jpeg`),
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
      templateVersion: "1.8",
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

function refreshAutomation(track: TrackDetail): void {
  const previous = track.automation;
  const suno = track.evidence.find((item) =>
    item.role === "suno_final_export" && item.verified && Boolean(item.sha256) && !item.verificationError
      && Boolean(item.metadata?.sunoStudioDetected)
  );
  const createdDate = suno?.metadata?.sunoCreatedDate?.trim() ?? "";
  const previousCreatedDate = previous.sunoCreatedTimestamp?.slice(0, 10) ?? "";
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
        : evidence(role, `${role}.${extension}`);
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
      const track = mutableTrack(trackId);
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
        templateVersion: "1.8",
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
    async validateTrack(trackId): Promise<ValidationResult> {
      await wait();
      const track = get(trackId);
      return finalizationGate(track, track.profileSnapshot);
    },
    async finalizeTrack(trackId, onProgress) {
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
      track.certificate = { valid: true, certificateId, finalizedAt: now(), workflowVersion: WORKFLOW_VERSION };
      track.finalizationAnchors = [
        { artifact: "evidence_manifest", label: "Evidence manifest (recommended timestamp anchor)", relativePath: "06_CERTIFICATE/EVIDENCE_MANIFEST.json", sha256: "a".repeat(64) },
        { artifact: "sha256sums", label: "Track SHA-256 manifest", relativePath: "03_DOCUMENTATION/SHA256SUMS.txt", sha256: "b".repeat(64) },
        { artifact: "documentation_certificate_markdown", label: "Documentation certificate (Markdown)", relativePath: "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md", sha256: "c".repeat(64) },
        { artifact: "certificate_pdf", label: "Documentation certificate (PDF)", relativePath: "SunoDM_DOCUMENTATION_CERTIFICATE.pdf", sha256: "d".repeat(64) },
        { artifact: "final_evidence_package", label: "Final evidence package certificate hash set", relativePath: "06_CERTIFICATE/CERTIFICATE_SHA256.txt", sha256: "e".repeat(64) }
      ];
      refresh(track);
      track.status = "FINALIZED";
      return result(track, "Dokumentation finalisiert und Zertifikat erzeugt.");
    },
    async attachExternalTimestamp(trackId, input: ExternalTimestampInput) {
      await wait();
      const track = get(trackId);
      if (track.status !== "FINALIZED" || !track.certificate.valid || !track.certificate.certificateId) {
        throw new Error("Ein externer Zeitstempel kann erst nach der technischen Finalisierung angehängt werden.");
      }
      const anchor = track.finalizationAnchors.find((item) => item.artifact === input.referencedArtifact);
      const actualSha256 = anchor?.sha256;
      const id = crypto.randomUUID();
      track.externalTimestamps.push({
        id,
        certificateId: track.certificate.certificateId,
        provider: input.provider,
        timestampType: input.timestampType,
        timestampValue: input.timestampValue,
        referencedArtifact: input.referencedArtifact,
        referencedArtifactPath: anchor?.relativePath || input.otherReferencedArtifact,
        referencedSha256: input.referencedSha256,
        actualSha256: actualSha256 ?? "",
        referencedHashMatch: actualSha256 ? actualSha256.toLocaleLowerCase() === input.referencedSha256.trim().toLocaleLowerCase() : null,
        externalReferenceId: input.externalReferenceId,
        providerVerificationUrl: input.providerVerificationUrl,
        note: input.note,
        evidenceFileName: "external_timestamp_evidence.pdf",
        evidenceSha256: "f".repeat(64),
        importedAt: now(),
        provenance: "Managed copy; user-confirmed metadata; system-verified SHA-256 comparison",
        recordRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD.json`,
        markdownRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.md`,
        pdfRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.pdf`,
        hashListRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD_SHA256.txt`,
        integrityVerified: true,
        integrityIssues: []
      });
      return clone(track);
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
      track.status = "ACTIVE";
      track.certificate = { valid: false };
      track.integrity.generated = false;
      track.integrity.verified = false;
      track.integrity.mismatchFiles = [];
      track.documents.current = false;
      track.externalTimestamps = [];
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
