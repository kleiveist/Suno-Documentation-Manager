import type { EvidenceItem, EvidenceRole, TrackDetail, TrackFields } from "../types";

export const hasText = (value: string): boolean => value.trim().length > 0;
export const hasSelections = (value: readonly string[]): boolean => value.some((item) => item.trim().length > 0);
export const hasEvidence = (evidence: EvidenceItem[], role: EvidenceRole): boolean =>
  evidence.some((item) => item.role === role && item.verified && Boolean(item.sha256) && !item.verificationError);
const releaseEvidence = (evidence: EvidenceItem[]): EvidenceItem | undefined =>
  evidence.find(
    (item) => item.role === "release_wav" && item.verified && Boolean(item.sha256) && !item.verificationError
  );
export const hasCurrentLocalAudioScreening = (track: Pick<TrackDetail, "evidence" | "audioScreening">): boolean => {
  const source = releaseEvidence(track.evidence);
  const local = track.audioScreening?.local;
  return (
    local?.status === "fingerprint_generated" &&
    Boolean(source) &&
    local.sourceEvidenceId === source!.id &&
    local.sourceRelativePath === source!.relativePath &&
    local.sourceSha256 === source!.sha256 &&
    local.sourceSizeBytes === source!.sizeBytes &&
    Boolean(local.artifactRelativePath?.trim()) &&
    /^[a-f\d]{64}$/iu.test(local.artifactSha256 ?? "")
  );
};
export const documentationAnswerProvided = (value: TrackFields["aiAssistedAudioElements"]): boolean => value !== null;
const originalFileName = (evidence: EvidenceItem[], role: EvidenceRole): string =>
  evidence
    .find((item) => item.role === role && item.verified && Boolean(item.sha256) && !item.verificationError)
    ?.metadata?.originalFileName?.trim() ?? "";
const filenameIdentity = (value: string): string =>
  value
    .normalize("NFKD")
    .replace(/[^\p{L}\p{N}]/gu, "")
    .toLocaleLowerCase();
export const filenameMatchesDocumentedTitle = (title: string, fileName: string): boolean =>
  Boolean(fileName.trim()) && filenameIdentity(title) === filenameIdentity(fileName.replace(/\.[^.]+$/, ""));
export const filenameRequirementMet = (
  evidence: EvidenceItem[],
  role: EvidenceRole,
  title: string,
  confirmed: boolean | null
): boolean => {
  const fileName = originalFileName(evidence, role);
  return Boolean(fileName) && (filenameMatchesDocumentedTitle(title, fileName) || confirmed === true);
};

export type SubscriptionCoverageStatus = "YES" | "NO" | "NOT_VERIFIED";

const subscriptionItems = (evidence: EvidenceItem[], portableOnly = false): EvidenceItem[] =>
  evidence.filter(
    (item) => hasEvidence([item], "subscription_payment") && (!portableOnly || Boolean(item.sourceGlobalEvidenceId))
  );

export const subscriptionGenerationCoverageStatus = (
  evidence: EvidenceItem[],
  fields: TrackFields
): SubscriptionCoverageStatus => {
  if (!fields.sunoFinalGenerationDate) return "NOT_VERIFIED";
  const items = subscriptionItems(evidence);
  if (!items.length || items.some((item) => !item.coverageStart || !item.coverageEnd)) return "NOT_VERIFIED";
  return items.some(
    (item) =>
      item.coverageStart! <= fields.sunoFinalGenerationDate && item.coverageEnd! >= fields.sunoFinalGenerationDate
  )
    ? "YES"
    : "NO";
};

export const isoDay = (value: string | undefined): number | null => {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const milliseconds = Date.parse(`${value}T00:00:00Z`);
  if (!Number.isFinite(milliseconds) || new Date(milliseconds).toISOString().slice(0, 10) !== value) return null;
  return Math.floor(milliseconds / 86_400_000);
};

export const subscriptionProductionCoverageStatus = (
  evidence: EvidenceItem[],
  fields: TrackFields
): SubscriptionCoverageStatus => {
  const productionStart = isoDay(fields.productionStartDate);
  const productionEnd = isoDay(fields.productionEndDate);
  if (productionStart === null || productionEnd === null || productionEnd < productionStart) return "NOT_VERIFIED";
  const items = subscriptionItems(evidence, true);
  if (!items.length || items.some((item) => !item.coverageStart || !item.coverageEnd)) return "NOT_VERIFIED";
  const ranges = items
    .map((item) => [isoDay(item.coverageStart), isoDay(item.coverageEnd)] as const)
    .filter(
      (range): range is readonly [number, number] => range[0] !== null && range[1] !== null && range[1] >= range[0]
    )
    .sort((left, right) => left[0] - right[0]);
  if (ranges.length !== items.length) return "NOT_VERIFIED";
  let nextUncovered = productionStart;
  for (const [start, end] of ranges) {
    if (end < productionStart || start > productionEnd) continue;
    if (start > nextUncovered) break;
    nextUncovered = Math.max(nextUncovered, end + 1);
    if (nextUncovered > productionEnd) return "YES";
  }
  return "NO";
};

export interface SubscriptionEvidenceRelevance {
  relevant: boolean;
  coversProduction: boolean;
  overlapsProduction: boolean;
  coversGeneration: boolean;
}

export function subscriptionEvidenceRelevance(
  item: Pick<EvidenceItem, "coverageStart" | "coverageEnd">,
  fields: TrackFields
): SubscriptionEvidenceRelevance {
  const start = item.coverageStart ?? "";
  const end = item.coverageEnd ?? "";
  const coversProduction =
    Boolean(start && end && fields.productionStartDate && fields.productionEndDate) &&
    start <= fields.productionStartDate &&
    end >= fields.productionEndDate;
  const overlapsProduction =
    Boolean(start && end && fields.productionStartDate && fields.productionEndDate) &&
    start <= fields.productionEndDate &&
    end >= fields.productionStartDate;
  const coversGeneration =
    Boolean(start && end && fields.sunoFinalGenerationDate) &&
    start <= fields.sunoFinalGenerationDate &&
    end >= fields.sunoFinalGenerationDate;
  return {
    relevant: overlapsProduction || coversGeneration,
    coversProduction,
    overlapsProduction,
    coversGeneration
  };
}

export const localDisclosureArtifacts = (evidence: EvidenceItem[], fields: TrackFields): EvidenceItem[] =>
  evidence.filter(
    (item) =>
      item.role === "ai_artwork_edited" &&
      item.verified &&
      Boolean(item.sha256) &&
      !item.verificationError &&
      item.provenance === "generated_disclosure" &&
      item.generatorVersion === "local-disclosure-v1" &&
      item.generatedDisclosureText === fields.disclosureText.trim() &&
      Boolean(item.derivedFromEvidenceId) &&
      item.derivedFromEvidenceId !== item.id &&
      evidence.some(
        (source) =>
          source.id === item.derivedFromEvidenceId &&
          source.role === "ai_artwork_original" &&
          source.verified &&
          Boolean(source.sha256) &&
          !source.verificationError
      )
  );
export const hasDisclosedFinalArtwork = (evidence: EvidenceItem[], fields: TrackFields): boolean => {
  const disclosedHashes = new Set(localDisclosureArtifacts(evidence, fields).map((item) => item.sha256!));
  return evidence.some(
    (item) =>
      item.role === "final_artwork" &&
      item.verified &&
      Boolean(item.sha256) &&
      !item.verificationError &&
      disclosedHashes.has(item.sha256!)
  );
};

export const humanEditedFinalArtworkStatus = (
  evidence: EvidenceItem[]
): "BYTE-IDENTICAL / SHA-256 MATCH" | "NO SHA-256 MATCH" | "NOT VERIFIED" => {
  const verifiedForRole = (role: EvidenceRole): EvidenceItem[] =>
    evidence.filter((item) => item.role === role && item.verified && Boolean(item.sha256) && !item.verificationError);
  const humanEdited = verifiedForRole("human_edited_artwork");
  const finalArtwork = verifiedForRole("final_artwork");
  if (!humanEdited.length || finalArtwork.length !== 1) return "NOT VERIFIED";
  return humanEdited.some((item) => item.sha256 === finalArtwork[0].sha256)
    ? "BYTE-IDENTICAL / SHA-256 MATCH"
    : "NO SHA-256 MATCH";
};

export const hasCoveringSubscriptionEvidence = (evidence: EvidenceItem[], fields: TrackFields): boolean =>
  subscriptionProductionCoverageStatus(evidence, fields) === "YES";

export function evidenceRoleLabel(role: EvidenceRole): string {
  const labels: Record<EvidenceRole, string> = {
    suno_final_export: "Suno Final-Export",
    suno_project_zip: "Suno-Projekt-ZIP",
    suno_screenshot: "Suno-Screenshot",
    subscription_payment: "Abo-/Zahlungsnachweis",
    release_wav: "Finale Release-Audiodatei",
    release_mp3: "Release-MP3",
    release_mp4: "Release-MP4",
    release_artwork: "Release-Artwork",
    artwork_suno_original: "Suno-Original-Artwork",
    ai_artwork_original: "KI-Artwork Original",
    ai_artwork_edited: "KI-Artwork bearbeitet",
    human_edited_artwork: "Menschlich bearbeitetes Artwork",
    final_artwork: "Finales Artwork",
    external_audio_license: "Lizenz für externes Audio",
    external_audio_file: "Externe Audiodatei",
    own_audio_file: "Eigene Audiodatei",
    source_code_file: "Quellcode / Quelldatei",
    code_generated_audio_file: "Codebasiert erzeugte Audiodatei",
    third_party_sample_file: "Fremde Sample-Datei",
    third_party_sample_license: "Lizenz für fremde Samples",
    suno_terms_rights: "Suno-Nutzungsbedingungen / Rechteinformationen",
    external_timestamp: "Externer Zeitstempelnachweis",
    lyrics: "Lyrics-Datei",
    style: "Style-/Prompt-Datei",
    other: "Sonstiger Nachweis"
  };
  return labels[role];
}

export function evidenceRoleFileTypes(role: EvidenceRole): string {
  const types: Record<EvidenceRole, string> = {
    suno_final_export: "WAV, MP3, FLAC, M4A, AIFF oder OGG",
    suno_project_zip: "ZIP",
    suno_screenshot: "PNG, JPG, WebP oder PDF",
    subscription_payment: "PDF, PNG, JPG, TXT oder Markdown",
    release_wav: "WAV, MP3, FLAC, M4A, AIFF oder OGG",
    release_mp3: "MP3",
    release_mp4: "MP4 oder M4V",
    release_artwork: "PNG oder JPG",
    artwork_suno_original: "PNG oder JPG",
    ai_artwork_original: "PNG oder JPG",
    ai_artwork_edited: "PNG oder JPG",
    human_edited_artwork: "PNG oder JPG",
    final_artwork: "PNG oder JPG",
    external_audio_license: "PDF, PNG, JPG, TXT oder Markdown",
    external_audio_file: "WAV, MP3, FLAC, M4A, AIFF oder OGG",
    own_audio_file: "WAV, MP3, FLAC, M4A, AIFF oder OGG",
    source_code_file: "Ruby, Python, JavaScript, TypeScript, Text, Markdown und weitere Text-/Quellcodeformate",
    code_generated_audio_file: "WAV oder MP3",
    third_party_sample_file: "WAV, MP3, FLAC, M4A, AIFF oder OGG",
    third_party_sample_license: "PDF, PNG, JPG, TXT oder Markdown",
    suno_terms_rights: "PDF",
    external_timestamp: "TSR, TST, P7S, PDF, TXT, Markdown, JSON, HTML, PNG oder JPG",
    lyrics: "TXT oder Markdown",
    style: "TXT oder Markdown",
    other: "PDF, Bild, Text, ZIP, WAV, MP3 oder MP4"
  };
  return types[role];
}
