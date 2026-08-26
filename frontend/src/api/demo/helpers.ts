import { emptyEvidenceMetadata } from "../../domain/types";
import type { EvidenceItem, EvidenceRole, OperationProgress, TrackLibraryAssignment } from "../../domain/types";

export const now = (): string => new Date().toISOString();
export const clone = <T>(value: T): T => structuredClone(value);
export const wait = async (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 140));

export async function demoProgress(
  onProgress: ((progress: OperationProgress) => void) | undefined,
  values: OperationProgress[]
): Promise<void> {
  for (const value of values) {
    onProgress?.(value);
    await new Promise((resolve) => setTimeout(resolve, 55));
  }
}

export function trackFolderName(title: string): string {
  return title.trim();
}

export function canonicalArtworkStem(title: string): string {
  const stem = title
    .trim()
    .replace(/[^a-z\d]+/giu, "_")
    .replace(/^_+|_+$/gu, "")
    .toLocaleUpperCase("en-US");
  return stem || "TRACK";
}

export function managedArtworkName(title: string, role: EvidenceRole, extension: string): string | null {
  const suffix: Partial<Record<EvidenceRole, string>> = {
    artwork_suno_original: "SUNO_ORIGINAL",
    ai_artwork_original: "AI_ORIGINAL",
    ai_artwork_edited: "AI_EDITED",
    human_edited_artwork: "HUMAN_EDITED",
    final_artwork: "FINAL"
  };
  return suffix[role] ? `${canonicalArtworkStem(title)}_${suffix[role]}.${extension}` : null;
}

export function trackRelativePath(library: TrackLibraryAssignment, title: string): string {
  const parent = library.section === "album" ? library.albumTitle!.trim() : "Singles";
  return `${parent}/${trackFolderName(title)}`;
}

export function evidence(role: EvidenceRole, fileName: string): EvidenceItem {
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

export function sunoEvidence(fileName: string, createdTimestamp = "2026-07-24T10:12:13Z"): EvidenceItem {
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
