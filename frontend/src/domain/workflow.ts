import type {
  EvidenceItem,
  EvidenceRole,
  GlobalProfile,
  StepId,
  StepStatus,
  TrackDetail,
  TrackFields,
  TrackStatus,
  ValidationResult,
  WorkflowStepState
} from "./types";

export interface WorkflowStepDefinition {
  id: StepId;
  number: string;
  shortLabel: string;
  title: string;
  description: string;
  required: boolean;
}

export const WORKFLOW_ID = "suno-track";
export const WORKFLOW_VERSION = "1.6";

export const WORKFLOW_STEPS: readonly WorkflowStepDefinition[] = [
  { id: "track", number: "01", shortLabel: "Track", title: "Track", description: "Titel und Produktionszeitraum", required: true },
  { id: "source", number: "02", shortLabel: "Quelle", title: "Source", description: "Audioquellen und Rechtezuordnung", required: true },
  { id: "suno", number: "03", shortLabel: "Suno", title: "Suno", description: "Projekt, Modell und Erstellungstarif", required: true },
  { id: "human_work", number: "04", shortLabel: "Human Work", title: "Menschliche Arbeit", description: "Lyrics und bestätigte Bearbeitungen", required: true },
  { id: "artwork", number: "05", shortLabel: "Artwork", title: "Artwork", description: "Entstehung und Content-Check", required: true },
  { id: "ai_transparency", number: "06", shortLabel: "AI-Hinweis", title: "KI-Transparenz", description: "Projektinterne Disclosure-Policy", required: true },
  { id: "release", number: "07", shortLabel: "Release", title: "Release", description: "Letzte Bearbeitung und Release-Dateien", required: true },
  { id: "evidence_licenses", number: "08", shortLabel: "Evidence", title: "Evidence & Lizenzen", description: "Nachweise vollständig zuordnen", required: true },
  { id: "integrity", number: "09", shortLabel: "Integrität", title: "Integrität", description: "Dokumente, SHA-256 und Verifikation", required: true },
  { id: "finalize", number: "10", shortLabel: "Abschluss", title: "Finalisieren", description: "Gate prüfen und Zertifikat erzeugen", required: true }
];

export interface MissingRequirement {
  id: string;
  stepId: StepId;
  label: string;
  evidenceRole?: EvidenceRole;
}

export interface RequirementEvaluation extends MissingRequirement {
  completed: boolean;
}

const hasText = (value: string): boolean => value.trim().length > 0;
const hasSelections = (value: readonly string[]): boolean => value.some((item) => item.trim().length > 0);
const hasEvidence = (evidence: EvidenceItem[], role: EvidenceRole): boolean =>
  evidence.some((item) => item.role === role && item.verified && Boolean(item.sha256) && !item.verificationError);
const originalFileName = (evidence: EvidenceItem[], role: EvidenceRole): string =>
  evidence.find((item) => item.role === role && item.verified && Boolean(item.sha256) && !item.verificationError)
    ?.metadata?.originalFileName?.trim() ?? "";
const filenameIdentity = (value: string): string => value
  .normalize("NFKD")
  .replace(/[^\p{L}\p{N}]/gu, "")
  .toLocaleLowerCase();
export const filenameMatchesDocumentedTitle = (title: string, fileName: string): boolean =>
  Boolean(fileName.trim()) && filenameIdentity(title) === filenameIdentity(fileName.replace(/\.[^.]+$/, ""));
const filenameRequirementMet = (
  evidence: EvidenceItem[],
  role: EvidenceRole,
  title: string,
  confirmed: boolean | null
): boolean => {
  const fileName = originalFileName(evidence, role);
  return Boolean(fileName) && (filenameMatchesDocumentedTitle(title, fileName) || confirmed === true);
};
export type SubscriptionCoverageStatus = "YES" | "NO" | "NOT_VERIFIED";

const subscriptionItems = (evidence: EvidenceItem[], portableOnly = false): EvidenceItem[] => evidence.filter((item) =>
  hasEvidence([item], "subscription_payment") && (!portableOnly || Boolean(item.sourceGlobalEvidenceId))
);

export const subscriptionGenerationCoverageStatus = (evidence: EvidenceItem[], fields: TrackFields): SubscriptionCoverageStatus => {
  if (!fields.sunoFinalGenerationDate) return "NOT_VERIFIED";
  const items = subscriptionItems(evidence);
  if (!items.length || items.some((item) => !item.coverageStart || !item.coverageEnd)) return "NOT_VERIFIED";
  return items.some((item) => item.coverageStart! <= fields.sunoFinalGenerationDate && item.coverageEnd! >= fields.sunoFinalGenerationDate)
    ? "YES"
    : "NO";
};

const isoDay = (value: string | undefined): number | null => {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const milliseconds = Date.parse(`${value}T00:00:00Z`);
  if (!Number.isFinite(milliseconds) || new Date(milliseconds).toISOString().slice(0, 10) !== value) return null;
  return Math.floor(milliseconds / 86_400_000);
};

export const subscriptionProductionCoverageStatus = (evidence: EvidenceItem[], fields: TrackFields): SubscriptionCoverageStatus => {
  const productionStart = isoDay(fields.productionStartDate);
  const productionEnd = isoDay(fields.productionEndDate);
  if (productionStart === null || productionEnd === null || productionEnd < productionStart) return "NOT_VERIFIED";
  const items = subscriptionItems(evidence, true);
  if (!items.length || items.some((item) => !item.coverageStart || !item.coverageEnd)) return "NOT_VERIFIED";
  const ranges = items
    .map((item) => [isoDay(item.coverageStart), isoDay(item.coverageEnd)] as const)
    .filter((range): range is readonly [number, number] => range[0] !== null && range[1] !== null && range[1] >= range[0])
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
  const coversProduction = Boolean(start && end && fields.productionStartDate && fields.productionEndDate)
    && start <= fields.productionStartDate && end >= fields.productionEndDate;
  const overlapsProduction = Boolean(start && end && fields.productionStartDate && fields.productionEndDate)
    && start <= fields.productionEndDate && end >= fields.productionStartDate;
  const coversGeneration = Boolean(start && end && fields.sunoFinalGenerationDate)
    && start <= fields.sunoFinalGenerationDate && end >= fields.sunoFinalGenerationDate;
  return {
    relevant: overlapsProduction || coversGeneration,
    coversProduction,
    overlapsProduction,
    coversGeneration
  };
}
const localDisclosureArtifacts = (evidence: EvidenceItem[], fields: TrackFields): EvidenceItem[] =>
  evidence.filter((item) =>
    item.role === "ai_artwork_edited"
    && item.verified
    && Boolean(item.sha256)
    && !item.verificationError
    && item.provenance === "generated_disclosure"
    && item.generatorVersion === "local-disclosure-v1"
    && item.generatedDisclosureText === fields.disclosureText.trim()
    && Boolean(item.derivedFromEvidenceId)
    && item.derivedFromEvidenceId !== item.id
    && evidence.some((source) =>
      source.id === item.derivedFromEvidenceId
      && source.role === "ai_artwork_original"
      && source.verified
      && Boolean(source.sha256)
      && !source.verificationError
    )
  );
const hasDisclosedFinalArtwork = (evidence: EvidenceItem[], fields: TrackFields): boolean => {
  const disclosedHashes = new Set(
    localDisclosureArtifacts(evidence, fields)
      .map((item) => item.sha256!)
  );
  return evidence.some((item) =>
    item.role === "final_artwork"
    && item.verified
    && Boolean(item.sha256)
    && !item.verificationError
    && disclosedHashes.has(item.sha256!)
  );
};
const isAiArtwork = (fields: TrackFields): boolean =>
  fields.artworkOrigin === "ai_generated" || fields.artworkOrigin === "ai_assisted";
const hasCoveringSubscriptionEvidence = (evidence: EvidenceItem[], fields: TrackFields): boolean =>
  subscriptionProductionCoverageStatus(evidence, fields) === "YES";

export function visibleConditionalFields(fields: TrackFields, profile: GlobalProfile): Set<string> {
  const visible = new Set<string>();
  if (fields.externalAudioUploaded === true) {
    visible.add("externalAudioSource");
    visible.add("externalAudioOwnership");
    visible.add("externalAudioLicense");
    visible.add("externalAudioFile");
  }
  if (fields.ownAudioUploaded === true) {
    visible.add("ownAudioSource");
    visible.add("ownAudioOwnership");
    visible.add("ownAudioFile");
  }
  if (fields.codeBasedGeneration === true) {
    visible.add("sourceCodeFile");
    visible.add("codeAudioPostProcessed");
    visible.add("codeGeneratedAudioFile");
    if (fields.codeAudioPostProcessed === true) {
      visible.add("codeAudioPostProcessingOperations");
      if (fields.codeAudioPostProcessingOperations.includes("Other post-processing")) {
        visible.add("codeAudioPostProcessingNote");
      }
    }
  }
  if (fields.thirdPartySamplesUploaded === true) {
    visible.add("thirdPartySampleSource");
    visible.add("thirdPartySampleOwnership");
    visible.add("thirdPartySampleFile");
    visible.add("thirdPartySampleLicense");
  }
  if (fields.humanEditingPerformed === true) visible.add("humanEditingDetails");
  if (fields.postExportEditingPerformed === true) visible.add("postExportEditingDetails");
  if (fields.lyricsSource !== "" && fields.lyricsSource !== "instrumental") visible.add("lyricsText");
  if (isAiArtwork(fields)) {
    visible.add("aiImageService");
    visible.add("aiArtworkOriginal");
    if (profile.artworkTransparencyPolicy !== "none" && !contentCheckAllNegative(fields)) visible.add("disclosure");
  }
  if (fields.artworkOrigin === "human") visible.add("humanArtworkProcessOperations");
  if (fields.artworkOrigin === "ai_assisted") {
    visible.add("humanArtworkModifications");
    if (fields.humanArtworkModifications.includes("Other human editing")) visible.add("customArtworkChange");
  }
  if (fields.depictsRealPerson === true) visible.add("realPersonNotes");
  if (fields.depictsRealEvent === true) visible.add("realEventNotes");
  if (fields.containsTrademark === true) visible.add("trademarkNotes");
  return visible;
}

export function contentCheckAllNegative(fields: TrackFields): boolean {
  return fields.depictsRealPerson === false
    && fields.depictsRealEvent === false
    && fields.containsTrademark === false;
}

export function evaluateRequirements(
  track: Pick<TrackDetail, "fields" | "evidence" | "documents" | "integrity" | "blockingDeviations" | "automation">,
  profile: GlobalProfile
): RequirementEvaluation[] {
  const { fields, evidence, documents, integrity } = track;
  const requirements: RequirementEvaluation[] = [];
  const add = (id: string, stepId: StepId, label: string, completed: boolean, evidenceRole?: EvidenceRole): void => {
    requirements.push({ id, stepId, label, evidenceRole, completed });
  };

  add("title", "track", "Track-Titel", hasText(fields.title));
  add("production-start", "track", "Produktionsstart", hasText(fields.productionStartDate));
  add("production-end", "track", "Produktionsende", hasText(fields.productionEndDate));
  add("commercial-intent", "track", "Angabe zur kommerziellen Nutzung", true);
  add("profile-artist", "track", "Globaler Künstlername", hasText(profile.artistName));
  add("profile-policy", "track", "Globale KI-Artwork-Transparenzrichtlinie", hasText(profile.artworkTransparencyPolicy));

  add("external-audio-answer", "source", "Angabe zu externem Audio", fields.externalAudioUploaded !== null);
  add("own-audio-answer", "source", "Angabe zu eigenem Audio", fields.ownAudioUploaded !== null);
  add("code-generation-answer", "source", "Angabe zur codebasierten Erzeugung", fields.codeBasedGeneration !== null);
  add("samples-answer", "source", "Angabe zu fremden Samples", fields.thirdPartySamplesUploaded !== null);
  if (fields.externalAudioUploaded === true) {
    add("external-source", "source", "Quelle des externen Audios", hasText(fields.externalAudioSource));
    add("external-ownership", "source", "Rechtezuordnung des externen Audios", hasText(fields.externalAudioOwnership));
    add("external-license", "evidence_licenses", "Lizenznachweis für externes Audio", hasEvidence(evidence, "external_audio_license"), "external_audio_license");
    add("external-audio-file", "evidence_licenses", "Importierte externe Audiodatei", hasEvidence(evidence, "external_audio_file"), "external_audio_file");
  }
  if (fields.ownAudioUploaded === true) {
    add("own-source", "source", "Quelle des eigenen Audios", hasText(fields.ownAudioSource));
    add("own-ownership", "source", "Rechtezuordnung des eigenen Audios", hasText(fields.ownAudioOwnership));
    add("own-audio-file", "evidence_licenses", "Importierte eigene Audiodatei", hasEvidence(evidence, "own_audio_file"), "own_audio_file");
  }
  if (fields.codeBasedGeneration === true) {
    add("source-code-file", "evidence_licenses", "Quellcode oder Quelldatei der codebasierten Erzeugung", hasEvidence(evidence, "source_code_file"), "source_code_file");
    add("code-audio-post-processed-answer", "source", "Angabe zur Nachbearbeitung des codebasiert erzeugten Audios", fields.codeAudioPostProcessed !== null);
    if (fields.codeAudioPostProcessed === true) {
      add("code-audio-post-processing-operations", "source", "Bestätigte Nachbearbeitungsschritte des codebasiert erzeugten Audios", hasSelections(fields.codeAudioPostProcessingOperations));
    }
    add("code-generated-audio-file", "evidence_licenses", "Mit dem Quellcode erzeugte WAV- oder MP3-Datei", hasEvidence(evidence, "code_generated_audio_file"), "code_generated_audio_file");
  }
  if (fields.thirdPartySamplesUploaded === true) {
    add("sample-source", "source", "Quelle der fremden Samples", hasText(fields.thirdPartySampleSource));
    add("sample-ownership", "source", "Rechtezuordnung der fremden Samples", hasText(fields.thirdPartySampleOwnership));
    add("sample-file", "evidence_licenses", "Importierte Sample-Datei", hasEvidence(evidence, "third_party_sample_file"), "third_party_sample_file");
    add("sample-license", "evidence_licenses", "Lizenznachweis der fremden Samples", hasEvidence(evidence, "third_party_sample_license"), "third_party_sample_license");
  }

  add("suno-model", "suno", "Suno-Modell", hasText(fields.sunoModel));
  add("profile-suno-name", "suno", "Globaler Suno-Profilname", hasText(profile.sunoProfileName));
  add("profile-suno-handle", "suno", "Globaler Suno-Benutzername", hasText(profile.sunoHandle));
  add("profile-suno-plan", "suno", "Globaler Suno-Tarif", hasText(profile.sunoPlan));
  add("profile-subscription-start", "suno", "Startdatum des globalen Suno-Abonnements", hasText(profile.subscriptionStartDate));
  add("suno-url", "suno", "Suno-Projekt-URL", hasText(fields.sunoProjectUrl));
  add("suno-generation-date", "suno", "Datum der finalen Suno-Generation", hasText(fields.sunoFinalGenerationDate));
  add("suno-plan", "suno", "Suno-Tarif bei Erstellung", hasText(fields.sunoPlanAtCreation));
  add("suno-final-export", "suno", "Finaler Suno-Export", hasEvidence(evidence, "suno_final_export"), "suno_final_export");
  add("suno-filename", "suno", "Suno-Exportdateiname stimmt mit Titel überein oder Abweichung ist bestätigt", filenameRequirementMet(evidence, "suno_final_export", fields.title, fields.sunoExportFilenameDifferenceConfirmed), "suno_final_export");

  add("lyrics-source", "human_work", "Quelle der Lyrics", Boolean(fields.lyricsSource));
  add("instrumental-answer", "human_work", "Angabe: Instrumentaltrack", fields.instrumentalTrack !== null);
  const lyricsWorkSelected = fields.humanEditingPerformed === true
    && fields.humanEditingDetails.split(",").some((value) => value.trim() === "Lyrics");
  const instrumentalConsistent = fields.instrumentalTrack === true
    ? fields.lyricsSource === "instrumental" && !hasText(fields.lyricsText) && !lyricsWorkSelected
    : fields.instrumentalTrack === false && fields.lyricsSource !== "instrumental";
  add("instrumental-consistency", "human_work", "Instrumental-, Lyrics- und Human-Work-Angaben sind widerspruchsfrei", instrumentalConsistent);
  if (fields.lyricsSource !== "" && fields.lyricsSource !== "instrumental") add("lyrics-text", "human_work", "Verwendeter Lyrics-Text", hasText(fields.lyricsText));
  add("suno-style-prompt", "human_work", "In Suno verwendeter Style-Prompt", hasText(fields.sunoStylePrompt));
  add("human-editing-answer", "human_work", "Angabe zu menschlicher Bearbeitung", fields.humanEditingPerformed !== null);
  if (fields.humanEditingPerformed === true) add("human-editing-details", "human_work", "Bestätigte menschliche Bearbeitungsschritte", hasText(fields.humanEditingDetails));
  add("post-editing-answer", "release", "Angabe zur Bearbeitung auf dem Desktop-PC", fields.postExportEditingPerformed !== null);
  if (fields.postExportEditingPerformed === true) add("post-editing-details", "release", "Bearbeitungsschritte auf dem Desktop-PC", hasText(fields.postExportEditingDetails));

  add("artwork-origin", "artwork", "Entstehungsart des Artworks", Boolean(fields.artworkOrigin));
  if (fields.artworkOrigin === "ai_assisted") {
    add("artwork-human-changes", "artwork", "Mindestens eine menschliche Änderung am KI-assistierten Artwork", hasSelections(fields.humanArtworkModifications));
  }
  if (fields.artworkOrigin && fields.artworkOrigin !== "none") {
    add("real-person-answer", "artwork", "Content-Check: reale Person", fields.depictsRealPerson !== null);
    add("real-event-answer", "artwork", "Content-Check: reales Ereignis", fields.depictsRealEvent !== null);
    add("trademark-answer", "artwork", "Content-Check: Marke oder Logo", fields.containsTrademark !== null);
    if (fields.depictsRealPerson === true) add("real-person-notes", "artwork", "Notiz zur dargestellten realen Person", hasText(fields.realPersonNotes));
    if (fields.depictsRealEvent === true) add("real-event-notes", "artwork", "Notiz zum dargestellten realen Ereignis", hasText(fields.realEventNotes));
    if (fields.containsTrademark === true) add("trademark-notes", "artwork", "Notiz zur dargestellten Marke oder zum Logo", hasText(fields.trademarkNotes));
    const disclosureRequired = isAiArtwork(fields)
      && !contentCheckAllNegative(fields)
      && (profile.artworkTransparencyPolicy === "always"
        || (profile.artworkTransparencyPolicy === "per_artwork" && fields.disclosureApplied === true));
    const finalArtworkComplete = disclosureRequired
      ? hasDisclosedFinalArtwork(evidence, fields)
      : hasEvidence(evidence, "final_artwork");
    const finalArtworkLabel = disclosureRequired
      ? "Finales Artwork muss exakt die lokal gekennzeichnete Fassung sein"
      : "Finales, aus Suno heruntergeladenes Artwork";
    add("artwork-final", "artwork", finalArtworkLabel, finalArtworkComplete, "final_artwork");
  }

  if (isAiArtwork(fields)) {
    add("ai-original", "artwork", "Unverändertes KI-Artwork", hasEvidence(evidence, "ai_artwork_original"), "ai_artwork_original");
    if (!contentCheckAllNegative(fields)) {
      add("ai-service", "ai_transparency", "Verwendeter KI-Bilddienst", hasText(fields.aiImageService));
      add("profile-ai-service", "ai_transparency", "Globaler Standarddienst für KI-Bilder", hasText(profile.defaultAiImageService));
      add("ai-policy", "ai_transparency", "KI-Transparenzrichtlinie", hasText(profile.artworkTransparencyPolicy));
      const hasDisclosureArtifact = localDisclosureArtifacts(evidence, fields).length > 0;
      if (profile.artworkTransparencyPolicy === "always" && (fields.disclosureApplied !== true || !hasDisclosureArtifact)) {
        add("ai-disclosure", "ai_transparency", "Sichtbarer KI-Hinweis", false);
      } else if (profile.artworkTransparencyPolicy === "always") {
        add("ai-disclosure", "ai_transparency", "Sichtbarer KI-Hinweis", true);
      } else if (profile.artworkTransparencyPolicy === "per_artwork" && fields.disclosureApplied === null) {
        add("ai-disclosure-decision", "ai_transparency", "Entscheidung zum sichtbaren KI-Hinweis", false);
      } else if (profile.artworkTransparencyPolicy === "per_artwork" && fields.disclosureApplied === true && !hasDisclosureArtifact) {
        add("ai-disclosure-decision", "ai_transparency", "Erzeugte Fassung mit sichtbarem KI-Hinweis", false);
      } else if (profile.artworkTransparencyPolicy === "per_artwork") {
        add("ai-disclosure-decision", "ai_transparency", "Entscheidung zum sichtbaren KI-Hinweis", true);
      }
    }
  }

  add("export-date", "release", "Datum der letzten Bearbeitung", hasText(fields.finalExportDate));
  add("release-wav", "release", "Finale Release-Audiodatei", hasEvidence(evidence, "release_wav"), "release_wav");
  add("release-filename", "release", "Release-Dateiname stimmt mit Titel überein oder Abweichung ist bestätigt", filenameRequirementMet(evidence, "release_wav", fields.title, fields.releaseFilenameDifferenceConfirmed), "release_wav");
  if (fields.commercialUseIntended) {
    add("subscription-evidence", "evidence_licenses", "Abo-/Zahlungsnachweis für den Produktionszeitraum", hasCoveringSubscriptionEvidence(evidence, fields), "subscription_payment");
    add("subscription-generation-coverage", "evidence_licenses", "Abo-Nachweis deckt das Datum der finalen Generation ab", subscriptionGenerationCoverageStatus(evidence, fields) === "YES", "subscription_payment");
    const hasTermsEvidence = hasEvidence(evidence, "suno_terms_rights");
    add("terms-evidence", "evidence_licenses", "Suno-Nutzungsbedingungen oder ausdrücklich 'Terms evidence not available' (nicht beides)", hasTermsEvidence ? fields.sunoTermsEvidenceNotAvailable !== true : fields.sunoTermsEvidenceNotAvailable === true, hasTermsEvidence ? undefined : "suno_terms_rights");
  }
  add("documents-current", "integrity", "Aktuelle generierte Dokumente", documents.generated && documents.current);
  add("hashes-verified", "integrity", "Vollständige SHA-256-Verifikation", integrity.verified && integrity.mismatchFiles.length === 0);
  for (const issue of track.automation.consistencyIssues.filter((item) => item.blocking)) {
    add(`consistency-${issue.code}`, issue.stepId, issue.message, false);
  }
  add("blocking-deviations", "finalize", "Alle blockierenden Abweichungen gelöst", (track.blockingDeviations ?? []).every((item) => !item.blocking || item.resolved));
  for (const item of evidence.filter((entry) => !entry.verified || !entry.sha256 || Boolean(entry.verificationError))) {
    add(
      `unverified-evidence-${item.id}`,
      "evidence_licenses",
      `Evidence fehlt oder ist nicht verifiziert: ${item.relativePath}`,
      false,
      item.role
    );
  }

  return requirements;
}

export function calculateMissingRequirements(
  track: Pick<TrackDetail, "fields" | "evidence" | "documents" | "integrity" | "blockingDeviations" | "automation">,
  profile: GlobalProfile
): MissingRequirement[] {
  return evaluateRequirements(track, profile).filter((item) => !item.completed).map(({ completed: _completed, ...item }) => item);
}

export function deriveStepStatus(
  stepId: StepId,
  missing: MissingRequirement[],
  stored?: WorkflowStepState,
  applicable = true
): StepStatus {
  if (!applicable && stored?.status === "N_A" && stored.naReason?.trim()) return "N_A";
  if (missing.some((item) => item.stepId === stepId)) {
    if (["FAIL", "BLOCKED", "NOT_VERIFIED"].includes(stored?.status ?? "")) return stored!.status;
    return "NOT_RUN";
  }
  return "PASS";
}

export function calculateProgress(
  requirements: RequirementEvaluation[]
): number {
  const total = requirements.length;
  if (total === 0) return 100;
  const complete = requirements.filter((item) => item.completed).length;
  return Math.round((complete / total) * 100);
}

export function finalizationGate(
  track: Pick<TrackDetail, "fields" | "evidence" | "documents" | "integrity" | "steps" | "blockingDeviations" | "automation">,
  profile: GlobalProfile
): ValidationResult {
  const missing = calculateMissingRequirements(track, profile);
  const blockingStatuses = track.steps.filter((step) =>
    ["FAIL", "BLOCKED", "NOT_VERIFIED"].includes(step.status)
  );
  const invalidNa = track.steps.filter((step) => step.status === "N_A" && !step.naReason?.trim());
  const blockingItems = [
    ...blockingStatuses.map((step) => `${stepLabel(step.id)}: ${statusLabel(step.status)}`),
    ...invalidNa.map((step) => `${stepLabel(step.id)}: N/A benötigt eine Begründung`),
    ...track.integrity.mismatchFiles.map((file) => `Integritätsabweichung: ${file}`),
    ...(track.blockingDeviations ?? []).filter((item) => item.blocking && !item.resolved).map((item) => `Abweichung: ${item.description}`)
  ];
  return {
    valid: missing.length === 0 && blockingItems.length === 0,
    missingItems: missing.map((item) => item.label),
    blockingItems
  };
}

export function deriveTrackStatus(track: TrackDetail, profile: GlobalProfile): TrackStatus {
  if (track.status === "SUPERSEDED") return "SUPERSEDED";
  if (track.status === "FINALIZED" && track.certificate.valid && track.integrity.mismatchFiles.length === 0) return "FINALIZED";
  const gate = finalizationGate(track, profile);
  if (gate.valid) return "READY";
  const hasActivity = track.evidence.length > 0 || track.documents.generated || calculateProgress(evaluateRequirements(track, profile)) > 0;
  return hasActivity ? "ACTIVE" : "DRAFT";
}

export function statusLabel(status: StepStatus | TrackStatus): string {
  const labels: Record<StepStatus | TrackStatus, string> = {
    DRAFT: "Entwurf",
    ACTIVE: "In Arbeit",
    READY: "Bereit",
    FINALIZED: "Finalisiert",
    SUPERSEDED: "Ersetzt",
    NOT_RUN: "Offen",
    PASS: "Erfüllt",
    FAIL: "Fehlgeschlagen",
    BLOCKED: "Blockiert",
    N_A: "N/A",
    NOT_VERIFIED: "Nicht verifiziert"
  };
  return labels[status];
}

export function stepLabel(stepId: StepId): string {
  return WORKFLOW_STEPS.find((step) => step.id === stepId)?.title ?? stepId;
}

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
    external_timestamp: "PDF, TXT, Markdown, JSON, HTML, PNG oder JPG",
    other: "PDF, Bild, Text, ZIP, WAV, MP3 oder MP4"
  };
  return types[role];
}

export function stepStatuses(track: TrackDetail, profile: GlobalProfile): WorkflowStepState[] {
  const requirements = evaluateRequirements(track, profile);
  const missing = requirements.filter((item) => !item.completed);
  const statuses = WORKFLOW_STEPS.map((definition) => {
    const stored = track.steps.find((step) => step.id === definition.id);
    const applicable = requirements.some((item) => item.stepId === definition.id);
    return {
      id: definition.id,
      status: deriveStepStatus(definition.id, missing, stored, applicable),
      naReason: stored?.naReason,
      updatedAt: stored?.updatedAt
    };
  });
  const finalize = statuses.find((step) => step.id === "finalize");
  if (finalize?.status === "PASS" && statuses.some((step) => step.id !== "finalize" && !["PASS", "N_A"].includes(step.status))) {
    finalize.status = "BLOCKED";
  }
  return statuses;
}
