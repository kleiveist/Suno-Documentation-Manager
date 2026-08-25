import type {
  FolderImportExecutionInput,
  FolderImportProposal,
  EvidenceRole,
  StepId,
  StepStatus,
  TrackCreateInput,
  TrackDetail,
  TrackFieldPatch,
  TrackLibraryAssignment,
  TrackSummary
} from "../../domain/types";
import { trackLibraryAssignment } from "../../domain/track-library";
import { clone, evidence, managedArtworkName, now, sunoEvidence, trackRelativePath, wait } from "./helpers";
import { makeTrack, refresh } from "./fixtures";
import { normalizeCanonicalSunoSemantics } from "./suno-semantics";
import { DemoWorkspaceApi } from "./api-workspace";

export abstract class DemoLibraryApi extends DemoWorkspaceApi {
  async listTracks(): Promise<TrackSummary[]> {
    await wait();
    return [...this.state.tracks.values()].map(
      ({
        fields: _fields,
        steps: _steps,
        evidence: _evidence,
        documents: _documents,
        integrity: _integrity,
        certificate: _certificate,
        ...summary
      }) => {
        void _fields;
        void _steps;
        void _evidence;
        void _documents;
        void _integrity;
        void _certificate;
        return clone(summary);
      }
    );
  }

  async listAlbums(): Promise<string[]> {
    await wait();
    return clone(this.albumList());
  }

  async createAlbum(title: string): Promise<string[]> {
    await wait();
    const normalized = trackLibraryAssignment("album", title);
    if (!normalized?.albumTitle) throw new Error("Der Albumtitel ist ungültig.");
    const key = this.albumKey(normalized.albumTitle);
    if (this.state.albums.has(key))
      throw new Error(`Ein Albumordner mit diesem Namen existiert bereits: ${this.state.albums.get(key)}`);
    this.rememberAlbum(normalized);
    return clone(this.albumList());
  }

  async createTrack(input: TrackCreateInput) {
    await wait();
    const id = crypto.randomUUID();
    const library = trackLibraryAssignment(input.library.section, input.library.albumTitle ?? "");
    if (!library) throw new Error("Für einen Album-Track ist ein Albumtitel erforderlich.");
    this.rememberAlbum(library);
    const track = makeTrack(id, input.title.trim(), this.state.profile, false, library);
    track.fields.productionStartDate = input.productionStartDate;
    track.fields.commercialUseIntended = input.commercialUseIntended;
    for (const item of this.state.globalEvidence.filter((entry) => entry.role === "suno_terms_rights")) {
      this.attachGlobalToTrack(track, item);
    }
    refresh(track);
    this.state.tracks.set(id, track);
    if (this.state.workspace) this.state.workspace.trackCount = this.state.tracks.size;
    return clone(track);
  }

  async scanImportFolder(): Promise<FolderImportProposal | null> {
    await wait();
    return null;
  }

  async executeFolderImport(_input: FolderImportExecutionInput): Promise<TrackDetail[]> {
    void _input;
    throw new Error("Der Ordner-Import ist nur in der Desktop-App verfügbar.");
  }

  async loadTrack(trackId: string) {
    await wait();
    return clone(this.get(trackId));
  }

  async loadTrackCover(trackId: string) {
    await wait();
    const track = this.get(trackId);
    const item = track.evidence.find(
      (entry) => entry.role === "final_artwork" && entry.verified && Boolean(entry.sha256) && !entry.verificationError
    );
    if (!item) return null;
    return {
      evidenceId: item.id,
      dataUrl:
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAGUlEQVR42mNkYPj/n4GBgYGJAQoAHgQCAWNA+rMAAAAASUVORK5CYII="
    };
  }

  async updateTrackLibrary(trackId: string, input: TrackLibraryAssignment) {
    await wait();
    const track = this.get(trackId);
    const library = trackLibraryAssignment(input.section, input.albumTitle ?? "");
    if (!library) throw new Error("Für einen Album-Track ist ein Albumtitel erforderlich.");
    this.rememberAlbum(library);
    track.library = library;
    track.relativePath = trackRelativePath(library, track.title);
    return clone(track);
  }

  async renameAlbum(oldTitle: string, newTitle: string) {
    await wait();
    const normalized = trackLibraryAssignment("album", newTitle);
    if (!normalized?.albumTitle) throw new Error("Der neue Albumtitel ist ungültig.");
    const oldKey = this.albumKey(oldTitle);
    const newKey = this.albumKey(normalized.albumTitle);
    const existingTitle = this.state.albums.get(oldKey);
    if (!existingTitle) throw new Error(`Album nicht gefunden: ${oldTitle}`);
    if (oldKey !== newKey && this.state.albums.has(newKey)) {
      throw new Error(`Ein Albumordner mit diesem Namen existiert bereits: ${this.state.albums.get(newKey)}`);
    }
    const matching = [...this.state.tracks.values()].filter(
      (track) => track.library.section === "album" && this.albumKey(track.library.albumTitle ?? "") === oldKey
    );
    for (const track of matching) {
      track.library = clone(normalized);
      track.relativePath = trackRelativePath(normalized, track.title);
    }
    this.state.albums.delete(oldKey);
    this.rememberAlbum(normalized);
    return [...this.state.tracks.values()].map(
      ({
        fields: _fields,
        steps: _steps,
        evidence: _evidence,
        documents: _documents,
        integrity: _integrity,
        certificate: _certificate,
        ...summary
      }) => {
        void _fields;
        void _steps;
        void _evidence;
        void _documents;
        void _integrity;
        void _certificate;
        return clone(summary);
      }
    );
  }

  async updateTrack(trackId: string, patch: TrackFieldPatch) {
    await wait();
    const track = this.mutableTrack(trackId);
    const previousTitle = track.fields.title;
    track.fields = { ...track.fields, ...clone(patch) };
    normalizeCanonicalSunoSemantics(track.fields);
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
  }

  async adoptLegacyProfile(trackId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    track.profileSnapshot = clone(this.state.profile);
    track.documents.current = false;
    refresh(track);
    return clone(track);
  }

  async addDeviation(trackId: string, description: string, blocking: boolean) {
    await wait();
    const track = this.mutableTrack(trackId);
    track.blockingDeviations ??= [];
    track.blockingDeviations.push({
      id: crypto.randomUUID(),
      title: blocking ? "Blockierende Abweichung" : "Hinweis",
      description,
      blocking,
      resolved: false,
      createdAt: now()
    });
    refresh(track);
    return clone(track);
  }

  async resolveDeviation(trackId: string, deviationId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    const deviation = track.blockingDeviations?.find((item) => item.id === deviationId);
    if (deviation) Object.assign(deviation, { resolved: true, resolvedAt: now() });
    refresh(track);
    return clone(track);
  }

  async removeDeviation(trackId: string, deviationId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    track.blockingDeviations = track.blockingDeviations?.filter((item) => item.id !== deviationId);
    refresh(track);
    return clone(track);
  }

  async setStepStatus(trackId: string, stepId: StepId, status: StepStatus, naReason?: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    const current = track.steps.find((step) => step.id === stepId);
    if (current) Object.assign(current, { status, naReason, updatedAt: now() });
    else track.steps.push({ id: stepId, status, naReason, updatedAt: now() });
    refresh(track);
    return clone(track);
  }

  async importEvidence(trackId: string, role: EvidenceRole, replaceEvidenceId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    if (
      !replaceEvidenceId &&
      ["release_wav", "suno_final_export", "final_artwork"].includes(role) &&
      track.evidence.some((item) => item.role === role)
    ) {
      throw new Error(
        `Die Rolle ${role} ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum Ersetzen.`
      );
    }
    const extension =
      role.includes("artwork") || role === "suno_screenshot" || role === "final_artwork"
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
    const next =
      role === "suno_final_export"
        ? sunoEvidence(`${role}.${extension}`, "2026-08-17T06:38:06Z")
        : evidence(role, managedArtworkName(track.title, role, extension) ?? `${role}.${extension}`);
    const replaceIndex = replaceEvidenceId
      ? track.evidence.findIndex((item) => item.id === replaceEvidenceId && item.role === role)
      : -1;
    if (replaceEvidenceId && replaceIndex < 0) throw new Error("Die zu ersetzende Evidence wurde nicht gefunden.");
    if (replaceIndex >= 0) track.evidence[replaceIndex] = { ...next, id: replaceEvidenceId! };
    else track.evidence.push(next);
    if (role === "release_wav") this.markAudioScreeningStale(track);
    track.documents.current = false;
    refresh(track);
    return clone(track);
  }

  async removeEvidence(trackId: string, evidenceId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    const removed = track.evidence.find((item) => item.id === evidenceId);
    track.evidence = track.evidence.filter((item) => item.id !== evidenceId);
    if (removed?.role === "release_wav") this.markAudioScreeningStale(track);
    track.documents.current = false;
    refresh(track);
    return clone(track);
  }

  async previewEvidence(trackId: string, evidenceId: string) {
    await wait();
    const item = this.get(trackId).evidence.find((entry) => entry.id === evidenceId);
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
  }

  async verifyEvidence(trackId: string, evidenceId: string) {
    await wait();
    const track = this.get(trackId);
    for (const item of track.evidence) {
      if (!evidenceId || item.id === evidenceId) item.verified = true;
    }
    refresh(track);
    return clone(track);
  }

  async previewDocumentGeneration(trackId: string) {
    await wait();
    this.get(trackId);
    return {
      files: ["02_SUNO/Lyrics.md", "02_SUNO/Style.md", "03_DOCUMENTATION/README.md", "03_DOCUMENTATION/AI_USAGE.md"],
      collisions: [],
      adoptionRequired: false
    };
  }
}
