import type { DesktopApi } from "./api/desktop";
import { toUserMessage } from "./api/desktop";
import {
  calculateMissingRequirements,
  contentCheckAllNegative,
  evaluateRequirements,
  evidenceRoleFileTypes,
  evidenceRoleLabel,
  filenameMatchesDocumentedTitle,
  finalizationGate,
  statusLabel,
  stepStatuses,
  subscriptionEvidenceRelevance,
  subscriptionGenerationCoverageStatus,
  subscriptionProductionCoverageStatus,
  visibleConditionalFields,
  WORKFLOW_STEPS
} from "./domain/workflow";
import {
  emptyProfile,
  type EvidenceItem,
  type EvidencePreview,
  type EvidenceMetadata,
  type FolderImportProposal,
  type GlobalEvidenceItem,
  type EvidenceRole,
  type FactOrigin,
  type GlobalProfile,
  type OperationProgress,
  type ScanResult,
  type StepId,
  type SubscriptionBillingCycle,
  type TrackCoverPreview,
  type TrackDetail,
  type TrackFields,
  type TrackLibraryAssignment,
  type TrackSummary,
  type WorkflowDefinitionDto,
  type WorkspaceSummary
} from "./domain/types";
import { subscriptionCoverageEnd } from "./domain/subscription";
import {
  groupTrackLibrary,
  trackLibraryAssignment,
  type AlbumTrackGroup,
  type TrackLibraryStatusFilter
} from "./domain/track-library";
import { escapeHtml, formatBytes, formatDate, titleInitials } from "./ui/format";
import { icon } from "./ui/icons";
import {
  resolveTheme,
  THEME_STORAGE_KEY,
  toggledTheme,
  type ColorTheme
} from "./ui/theme";

type MainView = "dashboard" | "tracks" | "current" | "workspace" | "settings";
type TrackTab = "overview" | "suno" | "artwork" | "release" | "evidence" | "certificate";
type ToastKind = "success" | "error" | "info";
export type LongOperationKind = "documents" | "hashes" | "verification" | "finalization";

interface ToastState {
  kind: ToastKind;
  title: string;
  message: string;
}

interface ActiveOperationProgress {
  kind: LongOperationKind;
  progress: OperationProgress;
  elapsedSeconds: number;
}

interface AppState {
  workspace: WorkspaceSummary | null;
  profile: GlobalProfile;
  tracks: TrackSummary[];
  albums: string[];
  track: TrackDetail | null;
  workflow: WorkflowDefinitionDto | null;
  globalEvidence: GlobalEvidenceItem[];
  view: MainView;
  trackTab: TrackTab;
  activeStep: StepId | null;
  trackDraft: TrackFields | null;
  scanResult: ScanResult | null;
  query: string;
  trackFilter: TrackLibraryStatusFilter;
  busy: boolean;
  busyLabel: string;
  operationProgress: ActiveOperationProgress | null;
  sidebarOpen: boolean;
  showNewTrack: boolean;
  folderImport: FolderImportProposal | null;
  showTrackLibrary: boolean;
  showSubscriptionEvidence: boolean;
  evidencePreview: EvidencePreview | null;
  showCertificatePopup: boolean;
  theme: ColorTheme;
  toast: ToastState | null;
}

export type WorkspaceScopedUiState = Pick<
  AppState,
  "track" | "trackDraft" | "activeStep" | "trackTab" | "scanResult" | "albums" |
  "showNewTrack" | "showTrackLibrary" | "showSubscriptionEvidence" | "evidencePreview" | "query" | "trackFilter"
  | "showCertificatePopup" | "folderImport"
> & { draftDirty: boolean };

export function resetWorkspaceScopedUiState(state: WorkspaceScopedUiState): WorkspaceScopedUiState {
  return {
    ...state,
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
    query: "",
    trackFilter: "all",
    draftDirty: false
  };
}

export function shouldIgnoreModalBackdropClick(isBackdrop: boolean, isDirectClick: boolean): boolean {
  return isBackdrop && !isDirectClick;
}

export interface FinalizedTrackPresentation {
  title: string;
  message: string;
  actionLabel?: string;
  invalid: boolean;
}

export function isTrackContentLocked(status: TrackDetail["status"]): boolean {
  return status === "FINALIZED" || status === "SUPERSEDED";
}

export function canCreateTrackRevision(status: TrackDetail["status"]): boolean {
  return status === "FINALIZED";
}

export function finalizedTrackPresentation(
  track: Pick<TrackDetail, "status" | "certificate">
): FinalizedTrackPresentation | null {
  if (!isTrackContentLocked(track.status)) return null;
  if (track.status === "SUPERSEDED") {
    return {
      title: "Ersetzter Snapshot – nur lesbar",
      message: "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt. Navigation und reine Prüfungen bleiben verfügbar; der Snapshot selbst kann nicht erneut bearbeitet werden.",
      invalid: false
    };
  }
  return track.certificate.valid
    ? {
        title: "Finalisierter Snapshot – nur lesbar",
        message: "Lege eine neue Revision an, bevor du Angaben, Nachweise oder erzeugte Dokumente änderst. Die Navigation und reine Prüfungen bleiben verfügbar.",
        actionLabel: "Neue Revision anlegen und bearbeiten",
        invalid: false
      }
    : {
        title: "Finalisierter Snapshot mit ungültigem Zertifikat",
        message: "Der bisherige Snapshot bleibt erhalten. Lege eine neue Revision an, um Abweichungen zu bearbeiten und anschließend neu zu finalisieren.",
        actionLabel: "Neue Revision anlegen und bearbeiten",
        invalid: true
      };
}

export function shouldDiscardLockedDraft(status: TrackDetail["status"], draftDirty: boolean): boolean {
  return isTrackContentLocked(status) && draftDirty;
}

function progressRatio(progress: OperationProgress): number {
  if (progress.totalBytes > 0) return Math.min(progress.processedBytes / progress.totalBytes, 1);
  if (progress.totalFiles > 0) return Math.min(progress.processedFiles / progress.totalFiles, 1);
  return 0;
}

export function operationProgressPercent(kind: LongOperationKind, progress: OperationProgress): number {
  const ratio = progressRatio(progress);
  if (progress.stage === "complete") return 100;
  if (progress.stage === "saving_result") return 98;
  if (kind === "documents") {
    if (progress.stage === "preparing_documents") return 5;
    if (progress.stage === "rendering_documents") return 18;
    if (progress.stage === "writing_documents") return Math.round(22 + ratio * 68);
    if (progress.stage === "finalizing_documents") return 94;
    return 2;
  }
  if (kind === "hashes") {
    if (progress.stage === "discovering_files") return 4;
    if (progress.stage === "hashing") return Math.round(7 + ratio * 43);
    if (progress.stage === "writing_hash_list") return 53;
    if (progress.stage === "preparing_verification") return 57;
    if (progress.stage === "reading_hash_list") return 60;
    if (progress.stage === "verifying") return Math.round(62 + ratio * 31);
    if (progress.stage === "comparing_hashes") return 96;
    return 2;
  }
  if (kind === "finalization") {
    if (progress.stage === "validating_finalization_gate") return 5;
    if (progress.stage === "collecting_final_snapshot") return 13;
    if (progress.stage === "writing_finalization_marker") return 22;
    if (progress.stage === "generating_certificate") return 35;
    if (progress.stage === "verifying_certificate") return 50;
    if (progress.stage === "verifying_final_snapshot" || progress.stage === "reading_hash_list") return 58;
    if (progress.stage === "verifying") return Math.round(62 + ratio * 28);
    if (progress.stage === "comparing_hashes") return 92;
    if (progress.stage === "saving_final_snapshot") return 97;
    return 2;
  }
  if (progress.stage === "reading_hash_list") return 6;
  if (progress.stage === "verifying") return Math.round(10 + ratio * 82);
  if (progress.stage === "comparing_hashes") return 96;
  return 2;
}

export function operationStageLabel(stage: string): string {
  return ({
    discovering_files: "Dateien werden erfasst",
    hashing: "Digitale Fingerabdrücke entstehen",
    writing_hash_list: "Hashliste wird geschrieben",
    preparing_verification: "Gegenprüfung wird vorbereitet",
    reading_hash_list: "Gespeicherte Hashliste wird gelesen",
    verifying: "Dateien werden erneut geprüft",
    comparing_hashes: "Ergebnisse werden verglichen",
    preparing_documents: "Dokumentdaten werden gesammelt",
    rendering_documents: "Dokumente werden zusammengesetzt",
    writing_documents: "Dokumente werden sicher geschrieben",
    finalizing_documents: "Dokumentsatz wird aufgeräumt",
    saving_result: "Ergebnis wird lokal gespeichert",
    complete: "Vorgang abgeschlossen",
    validating_finalization_gate: "Finalisierungs-Gate wird geprüft",
    collecting_final_snapshot: "Unveränderlicher Snapshot wird vorbereitet",
    writing_finalization_marker: "Transaktion wird abgesichert",
    generating_certificate: "Zertifikat und Manifest entstehen",
    verifying_certificate: "Zertifikatssatz wird geprüft",
    verifying_final_snapshot: "Finaler Snapshot wird gegengeprüft",
    saving_final_snapshot: "Finalisierung wird verbindlich gespeichert"
  } as Record<string, string>)[stage] ?? "Lokaler Vorgang läuft";
}

const OPERATION_PROGRESS_CONFIGURATION = {
  documents: {
    eyebrow: "Dokument-Manufaktur",
    title: "Dein Dokumentsatz entsteht",
    iconName: "file" as const,
    steps: ["Daten sammeln", "Inhalte rendern", "Dateien schreiben", "Sicher abschließen"],
    thresholds: [0, 18, 22, 94],
    tips: [
      "Jede Datei wird zuerst vollständig aufgebaut und anschließend atomar veröffentlicht.",
      "Lyrics und Style-Prompt werden im Suno-Ordner als portable Markdown-Dateien abgelegt.",
      "Vorhandene verwaltete Dokumente werden nicht mit halbfertigen Inhalten überschrieben."
    ]
  },
  hashes: {
    eyebrow: "SHA-256-Werkstatt",
    title: "Digitale Fingerabdrücke entstehen",
    iconName: "hash" as const,
    steps: ["Dateien finden", "Bytes hashen", "Hashliste schreiben", "Gegenprüfung"],
    thresholds: [0, 7, 53, 57],
    tips: [
      "Große Dateien werden blockweise gelesen – sie müssen dafür nicht komplett in den Arbeitsspeicher.",
      "Schon ein einziges geändertes Byte erzeugt einen anderen SHA-256-Fingerabdruck.",
      "Nach dem Schreiben liest die App alle Dateien erneut und prüft die neue Hashliste."
    ]
  },
  verification: {
    eyebrow: "Integritätsradar",
    title: "Prüfsummen werden verifiziert",
    iconName: "shield" as const,
    steps: ["Hashliste lesen", "Dateien prüfen", "Werte vergleichen", "Ergebnis sichern"],
    thresholds: [0, 10, 96, 98],
    tips: [
      "Die Prüfung berechnet jeden Fingerabdruck erneut und vertraut keinem gespeicherten Dateistatus.",
      "Zusätzliche, fehlende und veränderte Dateien werden getrennt als Abweichung erkannt.",
      "Die Verifikation bleibt lokal; keine Datei und kein Hash verlässt den Workspace."
    ]
  },
  finalization: {
    eyebrow: "Zertifikats-Tresor",
    title: "Dein finaler Snapshot wird versiegelt",
    iconName: "certificate" as const,
    steps: ["Gate bestätigen", "Zertifikat erzeugen", "Snapshot gegenprüfen", "Sicher versiegeln"],
    thresholds: [0, 13, 58, 92],
    tips: [
      "Zertifikat, Evidence-Manifest und Zertifikats-Hashliste werden als zusammengehöriger Satz veröffentlicht.",
      "Vor dem Abschluss prüft die App die komplette SHA-256-Liste noch einmal von der Festplatte.",
      "Der finalisierte Snapshot bleibt unverändert; spätere Änderungen beginnen als ausdrücklich angelegte Revision."
    ]
  }
} as const;

export function parseMultiChoiceValue(value: string): string[] {
  return value.split(" | ").map((item) => item.trim()).filter(Boolean);
}

export function serializeMultiChoiceValue(values: string[]): string {
  return [...new Set(values.map((item) => item.trim()).filter(Boolean))].join(" | ");
}

export type GuidedChoice = readonly [value: string, label: string, aliases?: readonly string[]];
export type SingleChoiceOption = readonly [value: string, label: string];

const normalizedChoiceText = (value: string): string => value.trim().normalize("NFKC").toLocaleLowerCase("de-DE");

export function canonicalGuidedChoiceValue(value: string, choices: readonly GuidedChoice[]): string {
  const normalized = normalizedChoiceText(value);
  if (!normalized) return "";
  const choice = choices.find(([candidate, label, aliases = []]) =>
    [candidate, label, ...aliases].some((item) => normalizedChoiceText(item) === normalized)
  );
  return choice?.[0] ?? value.trim();
}

export function canonicalGuidedChoiceList(value: string, choices: readonly GuidedChoice[]): string {
  return serializeMultiChoiceValue(
    parseMultiChoiceValue(value).map((item) => canonicalGuidedChoiceValue(item, choices))
  );
}

export function canonicalGuidedChoiceArray(values: readonly string[], choices: readonly GuidedChoice[]): string[] {
  return [...new Set(values.map((item) => canonicalGuidedChoiceValue(item, choices)).filter(Boolean))];
}

export function singleChoiceFieldMarkup(
  name: string,
  label: string,
  value: string,
  options: readonly SingleChoiceOption[],
  required = false
): string {
  return `<fieldset class="multi-choice-field single-choice-field field--wide" data-single-choice-group ${required ? `aria-required="true"` : ""}><legend>${escapeHtml(label)}${required ? " *" : ""}</legend><div>${options.map(([option, optionLabel]) => `<label><input type="radio" name="${escapeHtml(name)}" value="${escapeHtml(option)}" data-single-choice ${value === option ? "checked" : ""} ${required ? "required" : ""}><span>${escapeHtml(optionLabel)}</span></label>`).join("")}</div><p class="field-help">Wähle genau eine passende Option aus.</p></fieldset>`;
}

export function trackSummaryFromDetail(track: TrackDetail): TrackSummary {
  return {
    id: track.id,
    title: track.title,
    relativePath: track.relativePath,
    library: structuredClone(track.library),
    status: track.status,
    updatedAt: track.updatedAt,
    progress: track.progress,
    missingCount: track.missingCount,
    certificateValid: track.certificate.valid,
    legacy: track.legacy,
    coverEvidenceId: track.coverEvidenceId
  };
}

export interface TrackCheckSummary {
  documentation: "vollständig" | "unvollständig";
  fileIntegrity: "geprüft" | "offen";
  sunoMetadata: "erkannt" | "nicht erkannt";
  subscriptionCoverage: "passend" | "nicht passend" | "nicht geprüft" | "nicht erforderlich";
  warningCount: number;
}

export function trackCheckSummary(track: TrackDetail): TrackCheckSummary {
  const documentationMissing = calculateMissingRequirements(track, track.profileSnapshot)
    .filter((item) => item.stepId !== "integrity" && item.stepId !== "finalize");
  const subscriptions = track.evidence.filter((item) =>
    item.role === "subscription_payment"
    && item.verified
    && Boolean(item.sha256)
    && !item.verificationError
    && Boolean(item.coverageStart)
    && Boolean(item.coverageEnd)
  );
  const generationDate = track.fields.sunoFinalGenerationDate;
  const subscriptionCoverage = !track.fields.commercialUseIntended
    ? "nicht erforderlich" as const
    : !generationDate || subscriptions.length === 0
      ? "nicht geprüft" as const
      : subscriptions.some((item) => item.coverageStart! <= generationDate && item.coverageEnd! >= generationDate)
        ? "passend" as const
        : "nicht passend" as const;
  const warnings = new Set<string>();
  track.automation.consistencyIssues.forEach((item) => warnings.add(`consistency:${item.code}`));
  track.integrity.mismatchFiles.forEach((item) => warnings.add(`integrity:${item}`));
  track.evidence.filter((item) => !item.verified || Boolean(item.verificationError))
    .forEach((item) => warnings.add(`evidence:${item.id}`));
  (track.blockingDeviations ?? []).filter((item) => !item.resolved)
    .forEach((item) => warnings.add(`deviation:${item.id}`));
  return {
    documentation: documentationMissing.length === 0 ? "vollständig" : "unvollständig",
    fileIntegrity: track.integrity.verified && track.integrity.mismatchFiles.length === 0 ? "geprüft" : "offen",
    sunoMetadata: track.automation.sunoMetadataDetected ? "erkannt" : "nicht erkannt",
    subscriptionCoverage,
    warningCount: warnings.size
  };
}

export function factOriginLabel(origin: FactOrigin): string {
  if (origin === "evidence_derived_metadata") return "Automatisch aus Suno-WAV erkannt";
  if (origin === "user_confirmed_fact") return "Nutzerangabe";
  return "Noch nicht dokumentiert";
}

export function isAutomaticDateReadonly(origin: FactOrigin): boolean {
  return origin === "evidence_derived_metadata";
}

export const MAIN_NAVIGATION: ReadonlyArray<{ id: MainView; label: string; iconName: "dashboard" | "tracks" | "current" | "workspace" | "settings" }> = [
  { id: "dashboard", label: "Dashboard", iconName: "dashboard" },
  { id: "tracks", label: "Tracks", iconName: "tracks" },
  { id: "current", label: "Aktueller Track", iconName: "current" },
  { id: "workspace", label: "Workspace", iconName: "workspace" },
  { id: "settings", label: "Einstellungen", iconName: "settings" }
];

export function missingProfileFields(profile: GlobalProfile): string[] {
  const fields: Array<[keyof GlobalProfile, string]> = [
    ["artistName", "Künstlername"], ["sunoProfileName", "Suno-Profilname"],
    ["sunoHandle", "Suno-Benutzername"], ["sunoPlan", "Suno-Tarif"],
    ["subscriptionStartDate", "Abo-Startdatum"], ["defaultAiImageService", "Standard-KI-Bilddienst"],
    ["artworkTransparencyPolicy", "Artwork-Transparenzrichtlinie"], ["disclosureText", "Standard-Hinweistext"]
  ];
  return fields.filter(([key]) => typeof profile[key] !== "string" || String(profile[key]).trim() === "").map(([, label]) => label);
}

export interface WorkflowUpgradePresentation {
  message: string;
  action?: "re-evaluate-track";
}

export function workflowUpgradePresentation(
  track: Pick<TrackDetail, "status" | "workflowId" | "workflowVersion" | "certificate">,
  current: Pick<WorkflowDefinitionDto, "id" | "version"> | null
): WorkflowUpgradePresentation | null {
  if (!current || (track.workflowId === current.id && track.workflowVersion === current.version)) return null;
  const previous = `${track.workflowId} ${track.certificate.workflowVersion ?? track.workflowVersion}`;
  const next = `${current.id} ${current.version}`;
  return {
    message: track.status === "FINALIZED"
      ? `Finalized with workflow ${previous} / Current workflow ${next}`
      : track.status === "SUPERSEDED"
        ? `Superseded snapshot uses workflow ${previous} / Current workflow ${next}`
        : `Track uses workflow ${previous} / Current workflow ${next}`,
    ...(track.status === "SUPERSEDED" ? {} : { action: "re-evaluate-track" as const })
  };
}

export function workflowUpgradeFinalizationBlocker(
  track: Pick<TrackDetail, "status" | "workflowId" | "workflowVersion" | "certificate">,
  current: Pick<WorkflowDefinitionDto, "id" | "version"> | null
): string | null {
  return workflowUpgradePresentation(track, current)
    ? "Vor der Finalisierung muss der Track ausdrücklich mit dem aktuellen Workflow neu bewertet werden."
    : null;
}

const evidenceRoles: EvidenceRole[] = [
  "suno_final_export", "suno_project_zip", "suno_screenshot",
  "release_wav", "release_mp3", "release_mp4", "release_artwork", "artwork_suno_original", "ai_artwork_original",
  "ai_artwork_edited", "human_edited_artwork", "final_artwork", "external_audio_file",
  "external_audio_license", "own_audio_file", "source_code_file", "code_generated_audio_file", "third_party_sample_file",
  "third_party_sample_license", "external_timestamp", "lyrics", "style", "other"
];

const externalAudioSourceChoices: readonly GuidedChoice[] = [
  ["Audio from a licensed sample library", "Lizenzierte Sample-Bibliothek"],
  ["Licensed beat or instrumental", "Lizenzierter Beat oder Instrumentaltrack"],
  ["Audio supplied by a collaborator", "Von Mitwirkenden bereitgestelltes Audio"],
  ["Commissioned recording", "Beauftragte Aufnahme"],
  ["Public-domain recording", "Gemeinfreie Aufnahme"],
  ["Creative Commons recording", "Aufnahme unter Creative-Commons-Lizenz"]
];

const externalAudioRightsChoices: readonly GuidedChoice[] = [
  ["Commercial-use license", "Lizenz für kommerzielle Nutzung"],
  ["Direct permission from the rights holder", "Direkte Erlaubnis des Rechteinhabers"],
  ["Joint rights agreement", "Gemeinsame Rechtevereinbarung"],
  ["Public domain", "Gemeinfreiheit"],
  ["Creative Commons license", "Creative-Commons-Lizenz"]
];

const ownAudioSourceChoices: readonly GuidedChoice[] = [
  ["Original vocal recording", "Eigene Gesangsaufnahme"],
  ["Original instrument recording", "Eigene Instrumentalaufnahme"],
  ["Original field recording", "Eigene Feldaufnahme", ["Eigene Aufnahme"]],
  ["Original MIDI or software render", "Eigener MIDI- oder Software-Render"],
  ["Original sound design", "Eigenes Sounddesign"]
];

const ownAudioRightsChoices: readonly GuidedChoice[] = [
  ["Solely owned by the artist", "Ausschließlich eigene Rechte", ["Eigene Produktion"]],
  ["Jointly owned with collaborators", "Gemeinsame Rechte mit Mitwirkenden"],
  ["Participant permissions documented", "Einwilligungen der Beteiligten dokumentiert"]
];

const sampleSourceChoices: readonly GuidedChoice[] = [
  ["Commercial sample library", "Kommerzielle Sample-Bibliothek"],
  ["Royalty-free sample pack", "Royalty-free Sample-Pack"],
  ["Directly licensed from the sample creator", "Direkt vom Sample-Urheber lizenziert"],
  ["Public-domain archive", "Gemeinfreies Archiv"],
  ["Creative Commons source", "Creative-Commons-Quelle"]
];

const sampleRightsChoices: readonly GuidedChoice[] = [
  ["Commercial sample license", "Kommerzielle Sample-Lizenz"],
  ["Royalty-free license", "Royalty-free Lizenz"],
  ["Direct permission from the rights holder", "Direkte Erlaubnis des Rechteinhabers"],
  ["Public domain", "Gemeinfreiheit"],
  ["Creative Commons license", "Creative-Commons-Lizenz"]
];

const humanWorkChoices: readonly GuidedChoice[] = [
  ["Arrangement", "Arrangement"],
  ["Lyrics", "Lyrics"],
  ["Timing and cuts", "Timing und Cuts"],
  ["Sound design", "Sounddesign"],
  ["EQ", "EQ"],
  ["Mixing", "Mixing"],
  ["Mastering", "Mastering"],
  ["Loudness adjustment", "Lautheitsanpassung"]
];

const postExportWorkChoices: readonly GuidedChoice[] = [
  ["Editing and cuts", "Schnitt"],
  ["Arrangement", "Arrangement"],
  ["Timing correction", "Timing-Korrektur"],
  ["Sound design", "Sounddesign"],
  ["EQ", "EQ"],
  ["Mixing", "Mixing"],
  ["Mastering", "Mastering"],
  ["Loudness adjustment", "Lautheitsanpassung"],
  ["Noise reduction", "Rauschreduzierung"],
  ["Dynamics processing", "Dynamikbearbeitung"]
];

const codeAudioPostProcessingChoices: readonly GuidedChoice[] = [
  ["Editing and cuts", "Schnitt"],
  ["Arrangement", "Arrangement"],
  ["Mixing", "Mixing"],
  ["Loudness adjustment", "Lautstärke angepasst"],
  ["Normalization", "Normalisierung"],
  ["EQ", "EQ"],
  ["Compression", "Kompression"],
  ["Limiting", "Limiting"],
  ["Reverb", "Reverb"],
  ["Delay", "Delay"],
  ["Additional effects", "Weitere Effekte"],
  ["Stereo processing", "Stereo-Bearbeitung"],
  ["Panning", "Panorama"],
  ["Fade-in/fade-out", "Fade-In/Fade-Out"],
  ["Noise reduction", "Noise Reduction"],
  ["Mastering", "Mastering"],
  ["Resampling", "Resampling"],
  ["Format conversion", "Formatkonvertierung"],
  ["Other post-processing", "Sonstige Nachbearbeitung"]
];

const humanArtworkProcessChoices: readonly GuidedChoice[] = [
  ["Independently drawn", "Eigenständig gezeichnet"],
  ["Independently illustrated", "Eigenständig illustriert"],
  ["Photographed", "Fotografiert"],
  ["Digitally painted", "Digital gemalt"],
  ["Created in 3D", "3D erstellt"],
  ["Compositing", "Compositing"],
  ["Color correction", "Farbkorrektur"],
  ["Retouching", "Retusche"],
  ["Cropping", "Zuschnitt"],
  ["Typography added", "Typografie hinzugefügt"],
  ["Layers edited", "Ebenen bearbeitet"],
  ["Light/shadow adjusted", "Licht/Schatten angepasst"],
  ["Background edited", "Hintergrund bearbeitet"],
  ["Effects added", "Effekte hinzugefügt"],
  ["Other editing", "Sonstige Bearbeitung"]
];

const aiArtworkHumanChangeChoices: readonly GuidedChoice[] = [
  ["Prompt written manually", "Prompt manuell erstellt"],
  ["Subject selected", "Motiv ausgewählt"],
  ["Variants compared", "Varianten verglichen"],
  ["Framing selected", "Ausschnitt gewählt"],
  ["Cropping", "Zuschnitt"],
  ["Retouching", "Retusche"],
  ["Color correction", "Farbkorrektur"],
  ["Brightness/contrast adjusted", "Helligkeit/Kontrast"],
  ["Layer editing", "Ebenenbearbeitung"],
  ["Compositing", "Compositing"],
  ["Background changed", "Hintergrund verändert"],
  ["Elements removed", "Elemente entfernt"],
  ["Elements added", "Elemente hinzugefügt"],
  ["Typography added", "Typografie hinzugefügt"],
  ["Logo/title added", "Logo/Titel hinzugefügt"],
  ["Effects added", "Effekte hinzugefügt"],
  ["Upscaling", "Upscaling"],
  ["Format adjusted", "Format angepasst"],
  ["Manual tracing", "Manuelle Nachzeichnung"],
  ["Other human editing", "Sonstige menschliche Bearbeitung"]
];

const sunoModelSuggestions = [
  "v5.5", "v5", "v4.5-all", "v4.5+", "v4.5", "v4", "v3.5", "v3",
  "Custom Model / v5.5 Custom Model"
] as const;

const sunoPlanSuggestions = ["Free", "Pro", "Premier"] as const;

const releaseNoteChoices: readonly GuidedChoice[] = [
  ["Original Suno version", "Originale Suno-Fassung"],
  ["Streaming master", "Streaming-Master"],
  ["Radio edit", "Radio Edit"],
  ["Extended mix", "Extended Mix"],
  ["Instrumental version", "Instrumental"],
  ["Clean version", "Clean Version"],
  ["Explicit version", "Explicit Version"],
  ["Social-media version", "Social-Media-Version"]
];

/**
 * Convert known localized labels from older UI versions to the stable English values used by
 * persistence and document generation. Unknown legacy values are intentionally retained so the
 * user can see and reclassify them instead of losing information during an ordinary save.
 */
export function normalizeGuidedTrackFields(fields: TrackFields): TrackFields {
  const normalized = structuredClone(fields);
  normalized.externalAudioSource = canonicalGuidedChoiceValue(normalized.externalAudioSource, externalAudioSourceChoices);
  normalized.externalAudioOwnership = canonicalGuidedChoiceValue(normalized.externalAudioOwnership, externalAudioRightsChoices);
  normalized.ownAudioSource = canonicalGuidedChoiceValue(normalized.ownAudioSource, ownAudioSourceChoices);
  normalized.ownAudioOwnership = canonicalGuidedChoiceValue(normalized.ownAudioOwnership, ownAudioRightsChoices);
  normalized.thirdPartySampleSource = canonicalGuidedChoiceValue(normalized.thirdPartySampleSource, sampleSourceChoices);
  normalized.thirdPartySampleOwnership = canonicalGuidedChoiceValue(normalized.thirdPartySampleOwnership, sampleRightsChoices);
  normalized.humanEditingDetails = canonicalGuidedChoiceList(normalized.humanEditingDetails, humanWorkChoices);
  normalized.postExportEditingDetails = canonicalGuidedChoiceList(normalized.postExportEditingDetails, postExportWorkChoices);
  normalized.codeAudioPostProcessingOperations = canonicalGuidedChoiceArray(
    normalized.codeAudioPostProcessingOperations,
    codeAudioPostProcessingChoices
  );
  normalized.humanArtworkProcessOperations = canonicalGuidedChoiceArray(
    normalized.humanArtworkProcessOperations,
    humanArtworkProcessChoices
  );
  normalized.humanArtworkModifications = canonicalGuidedChoiceArray(
    normalized.humanArtworkModifications,
    aiArtworkHumanChangeChoices
  );
  normalized.releaseNotes = canonicalGuidedChoiceList(normalized.releaseNotes, releaseNoteChoices);
  return normalized;
}

const evidenceProvenanceLabel = (value: TrackDetail["evidence"][number]["provenance"]): string => ({
  managed_copy: "Verwaltete Kopie",
  global_copy: "Globale portable Kopie",
  generated_disclosure: "Lokal erzeugter Disclosure-Nachweis",
  indexed_legacy: "Historisch indexiert"
}[value ?? "managed_copy"]);

export class SunoDocumentationApp {
  private readonly state: AppState = {
    workspace: null,
    profile: { ...emptyProfile },
    tracks: [],
    albums: [],
    track: null,
    workflow: null,
    globalEvidence: [],
    view: "dashboard",
    trackTab: "overview",
    activeStep: null,
    trackDraft: null,
    scanResult: null,
    query: "",
    trackFilter: "all",
    busy: false,
    busyLabel: "",
    operationProgress: null,
    sidebarOpen: false,
    showNewTrack: false,
    folderImport: null,
    showTrackLibrary: false,
    showSubscriptionEvidence: false,
    evidencePreview: null,
    showCertificatePopup: false,
    theme: "light",
    toast: null
  };

  private toastTimer: number | undefined;
  private operationTimer: number | undefined;
  private draftDirty = false;
  private followsSystemTheme = true;
  private systemThemeQuery: MediaQueryList | null = null;
  private readonly trackCoverCache = new Map<string, TrackCoverPreview>();
  private trackCoverGeneration = 0;

  constructor(
    private readonly root: HTMLElement,
    private readonly api: DesktopApi
  ) {}

  start(): void {
    this.initializeTheme();
    this.root.addEventListener("click", (event) => void this.handleClick(event));
    this.root.addEventListener("submit", (event) => void this.handleSubmit(event));
    this.root.addEventListener("change", (event) => this.handleChange(event));
    this.root.addEventListener("input", (event) => this.handleInput(event));
    this.render();
  }

  private initializeTheme(): void {
    let stored: string | null = null;
    try {
      stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    } catch {
      // The selected theme still works for this session if storage is unavailable.
    }
    this.followsSystemTheme = stored !== "light" && stored !== "dark";
    this.systemThemeQuery = typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-color-scheme: dark)")
      : null;
    this.state.theme = resolveTheme(stored, this.systemThemeQuery?.matches ?? false);
    this.applyTheme();
    this.systemThemeQuery?.addEventListener("change", (event) => {
      if (!this.followsSystemTheme) return;
      this.state.theme = event.matches ? "dark" : "light";
      this.applyTheme();
    });
  }

  private applyTheme(): void {
    document.documentElement.dataset.theme = this.state.theme;
    document.documentElement.style.colorScheme = this.state.theme;
    document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')
      ?.setAttribute("content", this.state.theme === "dark" ? "#111310" : "#f4f2ed");
    const dark = this.state.theme === "dark";
    const label = dark ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren";
    const title = dark ? "Heller Modus" : "Dunkler Modus";
    this.root.querySelectorAll<HTMLElement>('[data-action="toggle-theme"]').forEach((control) => {
      control.setAttribute("aria-label", label);
      control.setAttribute("aria-pressed", String(dark));
      control.setAttribute("title", title);
      control.innerHTML = icon(dark ? "sun" : "moon");
    });
  }

  private toggleTheme(): void {
    this.state.theme = toggledTheme(this.state.theme);
    this.followsSystemTheme = false;
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, this.state.theme);
    } catch {
      // Keep the active session usable even if persistence is blocked.
    }
    this.applyTheme();
  }

  private async withBusy<T>(label: string, action: () => Promise<T>): Promise<T | undefined> {
    this.state.busy = true;
    this.state.busyLabel = label;
    this.state.operationProgress = null;
    this.render();
    try {
      return await action();
    } catch (error) {
      this.showToast("error", "Aktion nicht abgeschlossen", toUserMessage(error));
      return undefined;
    } finally {
      this.state.busy = false;
      this.state.busyLabel = "";
      this.state.operationProgress = null;
      this.render();
    }
  }

  private async withOperationProgress<T>(
    kind: LongOperationKind,
    label: string,
    action: (onProgress: (progress: OperationProgress) => void) => Promise<T>
  ): Promise<T | undefined> {
    const initialStage = kind === "documents"
      ? "preparing_documents"
      : kind === "hashes"
        ? "discovering_files"
        : kind === "finalization"
          ? "validating_finalization_gate"
          : "reading_hash_list";
    this.state.busy = true;
    this.state.busyLabel = label;
    this.state.operationProgress = {
      kind,
      elapsedSeconds: 0,
      progress: { stage: initialStage, processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: 0 }
    };
    this.render();
    let active = true;
    const startedAt = Date.now();
    window.clearInterval(this.operationTimer);
    this.operationTimer = window.setInterval(() => {
      if (!active || !this.state.operationProgress) return;
      this.state.operationProgress.elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
      this.syncBusyLayer();
    }, 1000);
    try {
      return await action((progress) => {
        if (!active || !this.state.operationProgress) return;
        this.state.operationProgress.progress = progress;
        this.state.operationProgress.elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
        this.syncBusyLayer();
      });
    } catch (error) {
      this.showToast("error", "Aktion nicht abgeschlossen", toUserMessage(error));
      return undefined;
    } finally {
      active = false;
      window.clearInterval(this.operationTimer);
      this.operationTimer = undefined;
      this.state.busy = false;
      this.state.busyLabel = "";
      this.state.operationProgress = null;
      this.render();
    }
  }

  private syncBusyLayer(): void {
    const operation = this.state.operationProgress;
    const current = this.root.querySelector<HTMLElement>(".busy-layer--operation");
    if (!operation || !current || current.dataset.operationKind !== operation.kind) {
      this.render();
      return;
    }

    // Keep the layer alive while progress changes. Replacing the complete
    // element here restarts every CSS keyframe and makes the artwork jump.
    const progress = operation.progress;
    const configuration = OPERATION_PROGRESS_CONFIGURATION[operation.kind];
    const percent = operationProgressPercent(operation.kind, progress);
    const detail = progress.totalBytes > 0
      ? `${formatBytes(progress.processedBytes)} von ${formatBytes(progress.totalBytes)} · ${progress.processedFiles}/${progress.totalFiles} Dateien`
      : progress.totalFiles > 0
        ? `${progress.processedFiles} von ${progress.totalFiles} Dateien`
        : "Dateisatz wird vorbereitet";
    const minutes = Math.floor(operation.elapsedSeconds / 60);
    const seconds = String(operation.elapsedSeconds % 60).padStart(2, "0");
    const activeStep = configuration.thresholds.reduce<number>(
      (result, threshold, index) => percent >= threshold ? index : result,
      0
    );
    const tip = configuration.tips[Math.floor(operation.elapsedSeconds / 5) % configuration.tips.length];

    const elapsed = current.querySelector<HTMLTimeElement>('[data-operation-value="elapsed"]');
    if (elapsed) {
      elapsed.textContent = `${minutes}:${seconds}`;
      elapsed.dateTime = `PT${operation.elapsedSeconds}S`;
    }
    const percentLabel = current.querySelector<HTMLElement>('[data-operation-value="percent"]');
    if (percentLabel) percentLabel.textContent = `${percent}%`;
    const stageLabel = current.querySelector<HTMLElement>('[data-operation-value="stage"]');
    if (stageLabel) stageLabel.textContent = operationStageLabel(progress.stage);
    const detailLabel = current.querySelector<HTMLElement>('[data-operation-value="detail"]');
    if (detailLabel) detailLabel.textContent = detail;

    const currentFile = current.querySelector<HTMLElement>('[data-operation-value="file"]');
    if (currentFile) {
      const fileName = progress.currentFile ?? "";
      currentFile.textContent = fileName;
      currentFile.title = fileName;
      currentFile.hidden = fileName.length === 0;
    }

    const meter = current.querySelector<HTMLElement>('[data-operation-value="meter"]');
    meter?.setAttribute("aria-valuenow", String(percent));
    const meterBar = current.querySelector<HTMLElement>('[data-operation-value="meter-bar"]');
    if (meterBar) meterBar.style.width = `${percent}%`;

    current.querySelectorAll<HTMLElement>("[data-operation-step]").forEach((step, index) => {
      const complete = index < activeStep;
      step.classList.toggle("is-complete", complete);
      step.classList.toggle("is-active", index === activeStep);
      const badge = step.querySelector<HTMLElement>("[data-operation-step-badge]");
      const nextState = complete ? "complete" : "pending";
      if (badge && badge.dataset.operationStepBadge !== nextState) {
        badge.dataset.operationStepBadge = nextState;
        badge.innerHTML = complete ? icon("check") : String(index + 1);
      }
    });

    const tipLabel = current.querySelector<HTMLElement>('[data-operation-value="tip"]');
    if (tipLabel) tipLabel.textContent = tip;
  }

  private renderBusyLayer(): string {
    if (!this.state.busy) return "";
    const operation = this.state.operationProgress;
    if (!operation) {
      return `<div class="busy-layer" role="status" aria-live="polite"><span class="spinner"></span><span>${escapeHtml(this.state.busyLabel)}</span></div>`;
    }
    const progress = operation.progress;
    const percent = operationProgressPercent(operation.kind, progress);
    const configuration = OPERATION_PROGRESS_CONFIGURATION[operation.kind];
    const detail = progress.totalBytes > 0
      ? `${formatBytes(progress.processedBytes)} von ${formatBytes(progress.totalBytes)} · ${progress.processedFiles}/${progress.totalFiles} Dateien`
      : progress.totalFiles > 0
        ? `${progress.processedFiles} von ${progress.totalFiles} Dateien`
        : "Dateisatz wird vorbereitet";
    const minutes = Math.floor(operation.elapsedSeconds / 60);
    const seconds = String(operation.elapsedSeconds % 60).padStart(2, "0");
    const activeStep = configuration.thresholds.reduce<number>((result, threshold, index) => percent >= threshold ? index : result, 0);
    const tip = configuration.tips[Math.floor(operation.elapsedSeconds / 5) % configuration.tips.length];
    return `<div class="busy-layer busy-layer--operation" data-operation-kind="${operation.kind}" data-operation-theme="${this.state.theme}" role="status" aria-live="polite" aria-busy="true">
      <section class="operation-progress operation-progress--${operation.kind}" aria-label="${escapeHtml(configuration.title)}">
        <header><div><p class="overline">${escapeHtml(configuration.eyebrow)}</p><h2>${escapeHtml(configuration.title)}</h2></div><time data-operation-value="elapsed" datetime="PT${operation.elapsedSeconds}S">${minutes}:${seconds}</time></header>
        <div class="operation-stage">
          <div class="operation-orbit" aria-hidden="true"><i></i><i></i><i></i><span>${icon(configuration.iconName)}</span><b data-operation-value="percent">${percent}%</b></div>
          <div class="operation-stream" aria-hidden="true"><i>01</i><i>a7</i><i>f3</i><i>9c</i><i>42</i><i>e8</i></div>
        </div>
        <div class="operation-status"><strong data-operation-value="stage">${escapeHtml(operationStageLabel(progress.stage))}</strong><span data-operation-value="detail">${escapeHtml(detail)}</span><code data-operation-value="file" title="${escapeHtml(progress.currentFile ?? "")}"${progress.currentFile ? "" : " hidden"}>${escapeHtml(progress.currentFile ?? "")}</code></div>
        <div class="operation-meter" data-operation-value="meter" role="progressbar" aria-label="Fortschritt" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${percent}"><i data-operation-value="meter-bar" style="width:${percent}%"></i></div>
        <ol class="operation-steps">${configuration.steps.map((step, index) => `<li data-operation-step class="${index < activeStep ? "is-complete" : index === activeStep ? "is-active" : ""}"><span data-operation-step-badge="${index < activeStep ? "complete" : "pending"}">${index < activeStep ? icon("check") : index + 1}</span><strong>${escapeHtml(step)}</strong></li>`).join("")}</ol>
        <div class="operation-tip">${icon("info")}<p><strong>Währenddessen</strong><span data-operation-value="tip">${escapeHtml(tip)}</span></p></div>
        <p class="operation-footnote">${icon("lock")} Lokal und nachvollziehbar · Bitte Workspace und Datenträger verbunden lassen.</p>
      </section>
    </div>`;
  }

  private showToast(kind: ToastKind, title: string, message: string): void {
    this.state.toast = { kind, title, message };
    // Most mutations finish after `withBusy` has already rendered its final
    // frame. Render the result immediately so workflow ticks and messages are
    // never delayed until the next interaction.
    this.render();
    window.clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => {
      this.state.toast = null;
      this.render();
    }, kind === "error" ? 8000 : 4500);
  }

  private async enterWorkspace(workspace: WorkspaceSummary): Promise<void> {
    this.state.workspace = workspace;
    this.trackCoverGeneration += 1;
    this.trackCoverCache.clear();
    // The native command has already switched its authoritative workspace.
    // Clear every workspace-scoped selection before loading the new index so a
    // track from the previous workspace can never be rendered or mutated here.
    const { draftDirty, ...resetState } = resetWorkspaceScopedUiState({
      track: this.state.track,
      trackDraft: this.state.trackDraft,
      activeStep: this.state.activeStep,
      trackTab: this.state.trackTab,
      scanResult: this.state.scanResult,
      albums: this.state.albums,
      showNewTrack: this.state.showNewTrack,
      folderImport: this.state.folderImport,
      showTrackLibrary: this.state.showTrackLibrary,
      showSubscriptionEvidence: this.state.showSubscriptionEvidence,
      evidencePreview: this.state.evidencePreview,
      showCertificatePopup: this.state.showCertificatePopup,
      query: this.state.query,
      trackFilter: this.state.trackFilter,
      draftDirty: this.draftDirty
    });
    Object.assign(this.state, resetState);
    this.draftDirty = draftDirty;
    const loaded = await this.withBusy("Workspace wird eingelesen …", async () => {
      const [profile, tracks, albums, workflow, globalEvidence] = await Promise.all([
        this.api.getProfile(), this.api.listTracks(), this.api.listAlbums(), this.api.getWorkflow(), this.api.listGlobalEvidence()
      ]);
      return { profile, tracks, albums, workflow, globalEvidence };
    });
    if (!loaded) {
      this.state.workspace = null;
      return;
    }
    this.state.profile = loaded.profile;
    this.state.tracks = loaded.tracks;
    this.state.albums = loaded.albums;
    this.state.workflow = loaded.workflow;
    this.state.globalEvidence = loaded.globalEvidence;
    this.state.view = "dashboard";
    void this.hydrateTrackCovers(loaded.tracks);
  }

  private async refreshTracks(): Promise<void> {
    const [tracks, albums] = await Promise.all([this.api.listTracks(), this.api.listAlbums()]);
    this.state.tracks = tracks;
    this.state.albums = albums;
    if (this.state.track) {
      this.state.track = await this.api.loadTrack(this.state.track.id);
      this.state.trackDraft = structuredClone(this.state.track.fields);
    }
    void this.hydrateTrackCovers(tracks);
  }

  private applyTrack(track: TrackDetail): void {
    this.state.track = track;
    this.state.trackDraft = structuredClone(track.fields);
    this.draftDirty = false;
    const albumTitle = track.library.section === "album" ? track.library.albumTitle?.trim() : "";
    if (albumTitle && !this.state.albums.some((title) =>
      title.normalize("NFKC").localeCompare(albumTitle.normalize("NFKC"), "de", { sensitivity: "base" }) === 0
    )) {
      this.state.albums.push(albumTitle);
      this.state.albums.sort((left, right) => left.localeCompare(right, "de", { sensitivity: "base", numeric: true }));
    }
    const summaryIndex = this.state.tracks.findIndex((item) => item.id === track.id);
    const summary = trackSummaryFromDetail(track);
    const cachedCover = this.trackCoverCache.get(track.id);
    if (!summary.coverEvidenceId || cachedCover?.evidenceId !== summary.coverEvidenceId) {
      this.trackCoverCache.delete(track.id);
    }
    if (summaryIndex >= 0) this.state.tracks[summaryIndex] = summary;
    else this.state.tracks.unshift(summary);
    void this.hydrateTrackCovers([summary]);
  }

  private async hydrateTrackCovers(tracks: TrackSummary[]): Promise<void> {
    const generation = this.trackCoverGeneration;
    const workspaceId = this.state.workspace?.id;
    const queue = tracks.filter((track) => {
      if (!track.coverEvidenceId) {
        this.trackCoverCache.delete(track.id);
        return false;
      }
      return this.trackCoverCache.get(track.id)?.evidenceId !== track.coverEvidenceId;
    });
    const worker = async (): Promise<void> => {
      let track: TrackSummary | undefined;
      while ((track = queue.shift())) {
        const expectedEvidenceId = track.coverEvidenceId;
        if (!expectedEvidenceId) continue;
        try {
          const cover = await this.api.loadTrackCover(track.id);
          const current = this.state.tracks.find((item) => item.id === track!.id);
          if (generation !== this.trackCoverGeneration
            || workspaceId !== this.state.workspace?.id
            || !cover
            || cover.evidenceId !== expectedEvidenceId
            || current?.coverEvidenceId !== expectedEvidenceId) {
            continue;
          }
          this.trackCoverCache.set(track.id, cover);
          this.revealTrackCover(track.id, cover.dataUrl);
        } catch {
          // A cover is supplemental presentation. Keep the stable initials fallback
          // when its managed image cannot be decoded without interrupting workspace use.
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(queue.length, 3) }, () => worker()));
  }

  private revealTrackCover(trackId: string, dataUrl: string): void {
    this.root.querySelectorAll<HTMLElement>("[data-track-cover]").forEach((cover) => {
      if (cover.dataset.trackCover !== trackId) return;
      const image = cover.querySelector<HTMLImageElement>(".track-cover__image");
      const fallback = cover.querySelector<HTMLElement>(".track-cover__fallback");
      if (!image || !fallback) return;
      image.src = dataUrl;
      image.hidden = false;
      fallback.hidden = true;
      cover.classList.add("has-artwork");
    });
  }

  private render(): void {
    if (!this.state.workspace) {
      this.root.innerHTML = this.renderWelcome();
      return;
    }
    const view = this.renderCurrentView();
    this.root.innerHTML = `
      <div class="app-shell ${this.state.sidebarOpen ? "sidebar-is-open" : ""}">
        ${this.renderSidebar()}
        <div class="sidebar-scrim" data-action="close-sidebar"></div>
        <main class="main-shell">
          ${this.renderTopbar()}
          <div class="view-shell">${view}</div>
        </main>
        ${this.state.showCertificatePopup
          ? this.renderCertificatePopupDialog()
          : this.state.showNewTrack
          ? this.renderNewTrackDialog()
          : this.state.showTrackLibrary
            ? this.renderTrackLibraryDialog()
            : this.state.showSubscriptionEvidence
              ? this.renderSubscriptionEvidenceDialog()
              : this.state.evidencePreview
                ? this.renderEvidencePreviewDialog()
                : ""}
        ${this.renderToast()}
        ${this.renderBusyLayer()}
      </div>`;
  }

  private renderWelcome(): string {
    return `<main class="welcome-shell">
      <div class="welcome-grid"></div>
      <section class="welcome-card" aria-labelledby="welcome-title">
        <div class="brand-mark brand-mark--large" aria-hidden="true"><span></span><span></span><span></span><span></span></div>
        <p class="overline">Suno Documentation Manager</p>
        <h1 id="welcome-title">Deine Musik.<br><em>Sauber dokumentiert.</em></h1>
        <p class="welcome-copy">Ein lokaler Arbeitsbereich für nachvollziehbare Evidence, klare Integritätsprüfungen und portable Track-Dokumentation.</p>
        <div class="welcome-actions">
          <button class="button button--primary button--large" data-action="open-workspace">${icon("workspace")} Workspace auswählen</button>
          <button class="button button--secondary button--large" data-action="create-workspace">${icon("plus")} Neuen Workspace anlegen</button>
        </div>
        <div class="local-promise">${icon("shield")} <span><strong>Vollständig lokal.</strong> Keine Cloud, kein Login, keine Telemetrie.</span></div>
      </section>
      <footer class="welcome-footer"><span>Version 0.1</span><span>•</span><span>Offline by design</span>${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : ""}<button class="welcome-theme-toggle" data-action="toggle-theme" aria-label="${this.state.theme === "dark" ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren"}" aria-pressed="${this.state.theme === "dark"}" title="${this.state.theme === "dark" ? "Heller Modus" : "Dunkler Modus"}">${icon(this.state.theme === "dark" ? "sun" : "moon")}</button></footer>
      ${this.renderToast()}
      ${this.renderBusyLayer()}
    </main>`;
  }

  private renderSidebar(): string {
    return `<aside class="sidebar">
      <div class="sidebar-brand"><div class="brand-mark"><span></span><span></span><span></span><span></span></div><div><strong>SUNO</strong><small>Documentation Manager</small></div></div>
      <nav class="main-nav" aria-label="Hauptnavigation">
        ${MAIN_NAVIGATION.map((item) => `<button class="nav-item ${this.state.view === item.id ? "is-active" : ""}" data-view="${item.id}" ${item.id === "current" && !this.state.track ? "disabled" : ""}>${icon(item.iconName)}<span>${item.label}</span>${item.id === "tracks" ? `<b>${this.state.tracks.length}</b>` : ""}</button>`).join("")}
      </nav>
      <div class="sidebar-bottom">
        <div class="offline-card">${icon("shield")}<div><strong>Lokaler Modus</strong><span>Keine Daten verlassen dieses Gerät</span></div></div>
        <div class="workspace-mini"><span class="workspace-avatar">${escapeHtml(titleInitials(this.state.workspace?.name ?? "WS"))}</span><div><strong>${escapeHtml(this.state.workspace?.name)}</strong><span title="${escapeHtml(this.state.workspace?.path)}">${escapeHtml(this.state.workspace?.path)}</span></div><button class="icon-button" data-view="workspace" aria-label="Workspace öffnen">${icon("arrow")}</button></div>
      </div>
    </aside>`;
  }

  private renderTopbar(): string {
    const titles: Record<MainView, [string, string]> = {
      dashboard: ["Dashboard", "Dokumentationsstatus auf einen Blick"],
      tracks: ["Tracks", "Alle lokalen Musikprojekte"],
      current: [this.state.track?.title ?? "Aktueller Track", "Geführter Dokumentationsworkflow"],
      workspace: ["Workspace", "Lokaler Projektordner und Import"],
      settings: ["Einstellungen", "Globale Stammdaten und Richtlinien"]
    };
    const [title, subtitle] = titles[this.state.view];
    return `<header class="topbar">
      <button class="icon-button mobile-menu" data-action="open-sidebar" aria-label="Navigation öffnen">${icon("menu")}</button>
      <div class="topbar-title"><h1>${escapeHtml(title)}</h1><p>${escapeHtml(subtitle)}</p></div>
      <div class="topbar-actions">
        <button class="theme-toggle icon-button" data-action="toggle-theme" aria-label="${this.state.theme === "dark" ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren"}" aria-pressed="${this.state.theme === "dark"}" title="${this.state.theme === "dark" ? "Heller Modus" : "Dunkler Modus"}">${icon(this.state.theme === "dark" ? "sun" : "moon")}</button>
        ${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : '<span class="offline-pill"><i></i> Offline</span>'}
        <button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button>
      </div>
    </header>`;
  }

  private renderCurrentView(): string {
    switch (this.state.view) {
      case "dashboard": return this.renderDashboard();
      case "tracks": return this.renderTracks();
      case "current": return this.renderTrack();
      case "workspace": return this.renderWorkspace();
      case "settings": return this.renderSettings();
    }
  }

  private renderToast(): string {
    const toast = this.state.toast;
    if (!toast) return "";
    return `<div class="toast toast--${toast.kind}" role="alert">${icon(toast.kind === "success" ? "check" : toast.kind === "error" ? "alert" : "info")}<div><strong>${escapeHtml(toast.title)}</strong><span>${escapeHtml(toast.message)}</span></div><button data-action="dismiss-toast" aria-label="Hinweis schließen">${icon("close")}</button></div>`;
  }

  private renderNewTrackDialog(): string {
    const proposal = this.state.folderImport;
    const importedTrack = proposal?.tracks[0];
    const library: TrackLibraryAssignment = proposal?.kind === "album"
      ? { section: "album", albumTitle: proposal.albumTitle }
      : { section: "single" };
    const submitLabel = proposal
      ? `${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} importieren`
      : "Track anlegen";
    const creationFields = proposal?.kind === "album"
      ? `<div class="read-only-field"><span>Ziel der Bibliothek</span><strong>${escapeHtml(proposal.albumTitle ?? "Unbenanntes Album")}</strong><small>Alle erkannten Tracks erhalten die normale Album-/Track-Struktur. Der Produktionsstart bleibt je Track offen.</small></div>`
      : `${this.textField("title", "Track-Titel", "z. B. Cosmic Pulse", importedTrack?.title ?? "", true)}
        ${this.dateField("productionStartDate", "Produktionsstart", proposal ? "" : new Date().toISOString().slice(0, 10), !proposal)}
        ${this.renderTrackLibraryFields(library, "new-track")}`;
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal track-library-modal" role="dialog" aria-modal="true" aria-labelledby="new-track-title" data-modal-panel>
      <div class="modal-head"><div><p class="overline">Neues Projekt</p><h2 id="new-track-title">Track anlegen</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
      <form id="new-track-form" class="form-stack">
        ${proposal ? this.renderFolderImportPreview(proposal) : ""}
        ${creationFields}
        <label class="toggle-row"><span><strong>Kommerzielle Nutzung vorgesehen</strong><small>Wird als Track-Snapshot gespeichert.</small></span><input type="checkbox" name="commercialUseIntended" ${this.state.profile.defaultCommercialUse ? "checked" : ""}><i></i></label>
        <div class="modal-actions"><button type="button" class="button button--secondary modal-import-button" data-action="scan-folder-import">${icon("upload")} Ordner importieren</button><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("plus")} ${submitLabel}</button></div>
      </form>
    </section></div>`;
  }

  private renderFolderImportPreview(proposal: FolderImportProposal): string {
    const heading = proposal.kind === "album"
      ? `Album: ${escapeHtml(proposal.albumTitle ?? "Unbenannt")}`
      : "Einzelner Track erkannt";
    return `<section class="folder-import-preview"><p class="overline">Import erkannt</p><strong>${heading}</strong><small>${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} erkannt · Produktionsstart bleibt offen, sofern er nicht dokumentiert ist.</small><div class="folder-import-tracks">${proposal.tracks.map((track) => {
      const recognised = track.files.filter((file) => file.selected).map((file) => `${file.fileName} (${file.roles.join(", ")})`);
      return `<div><strong>${escapeHtml(track.title)}</strong>${recognised.length ? `<small>${escapeHtml(recognised.join(" · "))}</small>` : "<small>Keine eindeutig zuordenbare Evidence erkannt.</small>"}${track.ambiguities.length ? `<small class="warning">⚠ ${escapeHtml(track.ambiguities.join(" · "))} – Auswahl bleibt offen</small>` : ""}${track.unassignedFiles.length ? `<small>Nicht zugeordnet: ${escapeHtml(track.unassignedFiles.join(", "))}</small>` : ""}</div>`;
    }).join("")}</div>${proposal.unassignedFiles.length ? `<small>Nicht zugeordnet: ${escapeHtml(proposal.unassignedFiles.join(", "))}</small>` : ""}</section>`;
  }

  private renderTrackLibraryDialog(): string {
    const track = this.state.track;
    if (!track) return "";
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal track-library-modal" role="dialog" aria-modal="true" aria-labelledby="track-library-title" data-modal-panel>
      <div class="modal-head"><div><p class="overline">Bibliothekszuordnung</p><h2 id="track-library-title">${escapeHtml(track.title)} einordnen</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
      <form id="track-library-form" class="form-stack">
        ${this.renderTrackLibraryFields(track.library, "track-library")}
        <div class="library-safety-note">${icon("shield")}<p>Beim Speichern wird der vollständige Track-Ordner sicher in den gewählten Album- oder Singles-Ordner verschoben. Dateien, interne Prüfsummen und Zertifikat bleiben dabei unverändert.</p></div>
        <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("check")} Zuordnung speichern</button></div>
      </form>
    </section></div>`;
  }

  private renderTrackLibraryFields(library: TrackLibraryAssignment, idPrefix: string): string {
    const albumSelected = library.section === "album";
    const albumFieldId = `${idPrefix}-album-field`;
    const albumListId = `${idPrefix}-album-titles`;
    const albumTitles = this.albumTitles();
    return `<fieldset class="track-library-field"><legend>Bereich der Track-Bibliothek *</legend><div class="track-library-choices">
      <label><input type="radio" name="librarySection" value="single" aria-controls="${albumFieldId}" ${albumSelected ? "" : "checked"} required><span>${icon("tracks")}<strong>Single</strong><small>Unter Singles einordnen</small></span></label>
      <label><input type="radio" name="librarySection" value="album" aria-controls="${albumFieldId}" ${albumSelected ? "checked" : ""} required><span>${icon("workspace")}<strong>Album-Track</strong><small>Einem Album zuordnen</small></span></label>
    </div></fieldset>
    <label class="field library-album-field" id="${albumFieldId}" data-library-album-field ${albumSelected ? "" : "hidden"}><span class="field-label">Albumtitel *</span><input type="text" name="albumTitle" list="${albumListId}" placeholder="Bestehendes oder neues Album" value="${escapeHtml(library.albumTitle ?? "")}" autocomplete="off" ${albumSelected ? "required" : "disabled"}><small>Wird als echter Ordnername verwendet; maximal 200 Zeichen, keine Pfadtrenner oder reservierten Namen.</small></label>
    <datalist id="${albumListId}">${albumTitles.map((title) => `<option value="${escapeHtml(title)}"></option>`).join("")}</datalist>`;
  }

  private albumTitles(): string[] {
    const titles = new Map<string, string>();
    for (const title of this.state.albums) {
      const normalized = title.trim();
      if (normalized) titles.set(normalized.normalize("NFKC").toLocaleLowerCase("de-DE"), normalized);
    }
    for (const track of this.state.tracks) {
      const title = track.library?.section === "album" ? track.library.albumTitle?.trim() : "";
      if (title) titles.set(title.normalize("NFKC").toLocaleLowerCase("de-DE"), title);
    }
    return [...titles.values()].sort((left, right) => left.localeCompare(right, "de", { sensitivity: "base", numeric: true }));
  }

  private renderSubscriptionEvidenceDialog(): string {
    const coverageStart = new Date().toISOString().slice(0, 8) + "01";
    const coverageEnd = subscriptionCoverageEnd(coverageStart, "monthly") ?? "";
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal subscription-evidence-modal" role="dialog" aria-modal="true" aria-labelledby="subscription-evidence-title" data-modal-panel>
      <div class="modal-head"><div><p class="overline">Wiederverwendbarer Nachweis</p><h2 id="subscription-evidence-title">Suno-Abo-Nachweis registrieren</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
      <form id="subscription-evidence-form" class="form-stack">
        <fieldset class="billing-cycle-field"><legend>Bezahlrhythmus *</legend><div>
          <label><input type="radio" name="billingCycle" value="monthly" checked><span><strong>Monatlich</strong><small>Ein Kalendermonat ab dem Startdatum</small></span></label>
          <label><input type="radio" name="billingCycle" value="annual"><span><strong>Jährlich</strong><small>Zwölf Kalendermonate ab dem Startdatum</small></span></label>
        </div></fieldset>
        <div class="field-grid two-col">
          ${this.dateField("coverageStart", "Beginn laut Rechnung", coverageStart, true)}
          <label class="field"><span class="field-label">Automatisch abgedeckt bis</span><input type="date" name="coverageEnd" value="${coverageEnd}" readonly aria-readonly="true"></label>
        </div>
        <div class="evidence-guidance">${icon("info")}<p>Übernimm den tatsächlichen Beginn vom Beleg. Das Enddatum wird bis zum Tag vor der nächsten Zahlung berechnet; der Inhalt der Datei wird nicht automatisch ausgelesen. Pro Registrierung wird genau eine Rechnung oder ein Beleg ausgewählt.</p></div>
        <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("upload")} Datei auswählen und registrieren</button></div>
      </form>
    </section></div>`;
  }

  private renderEvidencePreviewDialog(): string {
    const preview = this.state.evidencePreview;
    if (!preview) return "";
    const metadata = this.state.track?.evidence.find((item) => item.id === preview.evidenceId)?.metadata;
    const technicalMetadata = metadata ? [
      metadata.mimeType ? `<div><dt>Medientyp</dt><dd>${escapeHtml(metadata.mimeType)}</dd></div>` : "",
      metadata.audioFormat ? `<div><dt>Audioformat</dt><dd>${escapeHtml(metadata.audioFormat)}</dd></div>` : "",
      typeof metadata.audioChannels === "number" ? `<div><dt>Kanäle</dt><dd>${metadata.audioChannels}</dd></div>` : "",
      typeof metadata.audioSampleRateHz === "number" ? `<div><dt>Sample Rate</dt><dd>${metadata.audioSampleRateHz.toLocaleString("de-DE")} Hz</dd></div>` : "",
      typeof metadata.audioDurationMilliseconds === "number" ? `<div><dt>Dauer</dt><dd>${(metadata.audioDurationMilliseconds / 1000).toLocaleString("de-DE", { maximumFractionDigits: 3 })} s</dd></div>` : "",
      typeof metadata.audioBitDepth === "number" ? `<div><dt>Bit-Tiefe</dt><dd>${metadata.audioBitDepth} Bit</dd></div>` : "",
      metadata.sunoStudioDetected ? `<div><dt>Suno Studio</dt><dd>Erkannt · Evidence-derived metadata</dd></div>` : "",
      metadata.sunoCreatedTimestamp ? `<div><dt>Suno-created</dt><dd>${escapeHtml(metadata.sunoCreatedTimestamp)}</dd></div>` : "",
      metadata.sunoId ? `<div><dt>Technische Suno-ID</dt><dd>${escapeHtml(metadata.sunoId)}</dd></div>` : ""
    ].filter(Boolean).join("") : "";
    const content = preview.dataUrl
      ? `<div class="evidence-preview-stage"><img src="${escapeHtml(preview.dataUrl)}" alt="Vorschau von ${escapeHtml(preview.fileName)}"></div>`
      : preview.textContent !== undefined && preview.textContent !== null
        ? `<pre class="evidence-preview-text">${escapeHtml(preview.textContent)}</pre>`
        : `<div class="evidence-preview-unavailable">${icon("file")}<p>${escapeHtml(preview.message ?? "Für diese Datei ist keine Vorschau verfügbar.")}</p></div>`;
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal evidence-preview-modal" role="dialog" aria-modal="true" aria-labelledby="evidence-preview-title" data-modal-panel>
      <div class="modal-head"><div><p class="overline">Evidence-Vorschau</p><h2 id="evidence-preview-title">${escapeHtml(preview.fileName)}</h2></div><button class="icon-button" data-action="close-modal" aria-label="Vorschau schließen">${icon("close")}</button></div>
      ${content}
      <dl class="evidence-preview-meta"><div><dt>Rolle</dt><dd>${escapeHtml(evidenceRoleLabel(preview.role))}</dd></div><div><dt>Größe</dt><dd>${escapeHtml(formatBytes(preview.sizeBytes))}</dd></div><div><dt>Pfad</dt><dd>${escapeHtml(preview.relativePath)}</dd></div>${technicalMetadata}</dl>
      <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Schließen</button></div>
    </section></div>`;
  }

  private renderCertificatePopupDialog(): string {
    const track = this.state.track;
    if (!track?.certificate.valid || !track.certificate.certificateId) return "";
    const openBlockingDeviations = (track.blockingDeviations ?? [])
      .filter((item) => item.blocking && !item.resolved).length;
    return `<div class="modal-backdrop certificate-popup-backdrop" data-action="close-modal"><section class="modal certificate-popup-modal" role="dialog" aria-modal="true" aria-labelledby="certificate-popup-title" data-modal-panel>
      <button class="icon-button certificate-popup-close" data-action="close-modal" aria-label="Zertifikat schließen">${icon("close")}</button>
      <div class="certificate-popup-celebration" aria-hidden="true"><i></i><i></i><i></i><span>${icon("certificate")}</span></div>
      <p class="overline">Track Documentation Completion Certificate</p>
      <h2 id="certificate-popup-title">Dokumentation erfolgreich finalisiert</h2>
      <span class="certificate-popup-result">${icon("check")} DOCUMENTATION COMPLETE</span>
      <dl class="certificate-popup-facts">
        <div><dt>Certificate ID</dt><dd>${escapeHtml(track.certificate.certificateId)}</dd></div>
        <div><dt>Track</dt><dd>${escapeHtml(track.title)}</dd></div>
        <div><dt>Artist</dt><dd>${escapeHtml(track.profileSnapshot.artistName)}</dd></div>
        <div><dt>Finalisiert</dt><dd>${formatDate(track.certificate.finalizedAt, true)}</dd></div>
        <div><dt>Workflow</dt><dd>${escapeHtml(track.workflowId)} · ${escapeHtml(track.certificate.workflowVersion ?? track.workflowVersion)}</dd></div>
        <div><dt>Integrität</dt><dd>${track.integrity.verifiedCount} / ${track.integrity.fileCount} Dateien verifiziert</dd></div>
        <div><dt>Evidence</dt><dd>${track.evidence.length} Dateien</dd></div>
        <div><dt>Blockierende Abweichungen</dt><dd>${openBlockingDeviations}</dd></div>
      </dl>
      <p class="certificate-popup-note">Der lokale Zertifikatssatz wurde erzeugt und verifiziert. Er bestätigt den Abschluss des konfigurierten Dokumentations- und Integritätsworkflows, ist aber keine behördliche oder rechtliche Zertifizierung.</p>
      <div class="modal-actions certificate-popup-actions"><button type="button" class="button button--secondary" data-action="close-modal">Schließen</button><button type="button" class="button button--primary" data-action="open-certificate-tab">${icon("certificate")} Vollständiges Zertifikat öffnen</button></div>
    </section></div>`;
  }

  private renderTrackCover(
    track: Pick<TrackSummary, "id" | "title" | "coverEvidenceId">,
    modifier = "",
    element: "span" | "div" = "span"
  ): string {
    const cached = this.trackCoverCache.get(track.id);
    const dataUrl = track.coverEvidenceId && cached?.evidenceId === track.coverEvidenceId
      ? cached.dataUrl
      : undefined;
    const classes = `track-cover${modifier ? ` ${modifier}` : ""}${dataUrl ? " has-artwork" : ""}`;
    return `<${element} class="${classes}" data-track-cover="${escapeHtml(track.id)}">
      <img class="track-cover__image" ${dataUrl ? `src="${escapeHtml(dataUrl)}"` : ""} alt="" ${dataUrl ? "" : "hidden"}>
      <span class="track-cover__fallback" ${dataUrl ? "hidden" : ""}>${escapeHtml(titleInitials(track.title))}<i></i></span>
    </${element}>`;
  }

  private renderDashboard(): string {
    const active = this.state.tracks.filter((track) => track.status === "ACTIVE" || track.status === "DRAFT").length;
    const ready = this.state.tracks.filter((track) => track.status === "READY").length;
    const finalized = this.state.tracks.filter((track) => track.status === "FINALIZED" && track.certificateValid !== false).length;
    const recent = [...this.state.tracks].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 4);
    const next = recent.find((track) => track.status !== "FINALIZED" || track.certificateValid === false) ?? recent[0];
    return `<div class="page-content dashboard-page">
      <section class="dashboard-welcome">
        <div><p class="overline">Lokaler Workspace</p><h2>Guten Tag, ${escapeHtml(this.state.profile.artistName || "Artist")}.</h2><p>${next ? `Bei <strong>${escapeHtml(next.title)}</strong> sind noch ${next.missingCount} Pflichtpunkte offen.` : "Lege deinen ersten Track an, um den Workflow zu starten."}</p></div>
        <div class="workspace-seal">${icon("shield")}<span>Workspace geschützt</span></div>
      </section>
      <section class="metric-grid" aria-label="Track-Status">
        ${this.metricCard("tracks", "Tracks gesamt", this.state.tracks.length, "Im aktuellen Workspace", "ink")}
        ${this.metricCard("current", "In Bearbeitung", active, "Dokumentation offen", "amber")}
        ${this.metricCard("check", "Bereit", ready, "Bereit zur Finalisierung", "blue")}
        ${this.metricCard("certificate", "Finalisiert", finalized, "Mit gültigem Snapshot", "green")}
      </section>
      <div class="dashboard-columns">
        <section class="panel recent-panel">
          <div class="panel-heading"><div><p class="overline">Zuletzt bearbeitet</p><h3>Deine Tracks</h3></div><button class="text-button" data-view="tracks">Alle anzeigen ${icon("arrow")}</button></div>
          ${recent.length ? `<div class="track-list compact">${recent.map((track) => this.renderTrackRow(track)).join("")}</div>` : this.emptyState("tracks", "Noch keine Tracks", "Lege deinen ersten Track an und dokumentiere nur, was wirklich relevant ist.", "new-track", "Track anlegen")}
        </section>
        <aside class="panel attention-panel">
          <div class="panel-heading"><div><p class="overline">Nächster Schritt</p><h3>Aufmerksamkeit</h3></div><span class="attention-count">${active}</span></div>
          ${next ? `<div class="attention-track">${this.renderTrackCover(next, "track-cover--large", "div")}<div><span class="status-chip status-chip--${next.status.toLowerCase()}">${statusLabel(next.status)}</span><h4>${escapeHtml(next.title)}</h4><p>${next.missingCount > 0 ? `${next.missingCount} erforderliche Angaben oder Nachweise fehlen noch.` : "Alle Pflichtpunkte sind erfüllt."}</p></div></div>
          <div class="progress-block"><div><span>Dokumentationsfortschritt</span><strong>${next.progress}%</strong></div><progress class="progress-track" max="100" value="${next.progress}" aria-label="Dokumentationsfortschritt ${next.progress} Prozent"></progress></div>
          <button class="button button--dark button--wide" data-track-open="${escapeHtml(next.id)}">Dokumentation fortsetzen ${icon("arrow")}</button>` : `<p class="muted">Keine offenen Tracks.</p>`}
        </aside>
      </div>
      <section class="principle-strip"><div class="principle-icon">${icon("shield")}</div><div><strong>Show what is missing.</strong><span>Ask only what is necessary.</span></div><p>Die App speichert ausschließlich lokal und erzeugt portable Track-Ordner, die ohne diese App verständlich bleiben.</p></section>
    </div>`;
  }

  private metricCard(iconName: "tracks" | "current" | "check" | "certificate", label: string, value: number, note: string, color: string): string {
    return `<article class="metric-card"><span class="metric-icon metric-icon--${color}">${icon(iconName)}</span><div><span>${label}</span><strong>${value.toLocaleString("de-DE")}</strong><small>${note}</small></div></article>`;
  }

  private renderTracks(): string {
    const library = groupTrackLibrary(this.state.tracks, {
      query: this.state.query,
      status: this.state.trackFilter
    }, this.state.albums);
    const albumTrackCount = library.albums.reduce((total, album) => total + album.tracks.length, 0);
    return `<div class="page-content tracks-page">
      <div class="page-lead"><div><p class="overline">Bibliothek</p><h2>Alben & Singles</h2><p>Die Ansicht entspricht der echten Ordnerstruktur im Workspace. Albumordner können direkt angelegt und umbenannt werden.</p></div><button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button></div>
      <section class="panel tracks-panel">
        <div class="tracks-toolbar">
          <label class="search-field"><span class="sr-only">Tracks und Alben durchsuchen</span>${icon("scan")}<input type="search" data-track-search placeholder="Tracks und Alben durchsuchen …" value="${escapeHtml(this.state.query)}"></label>
          <div class="filter-tabs" role="group" aria-label="Statusfilter">
            ${([['all','Alle'],['open','Offen'],['ready','Bereit'],['finalized','Finalisiert']] as const).map(([id, label]) => `<button class="${this.state.trackFilter === id ? "is-active" : ""}" data-track-filter="${id}">${label}</button>`).join("")}
          </div>
          <button class="button button--secondary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button>
        </div>
        <div class="track-library-content">
          <details class="library-section" aria-labelledby="albums-library-title" open>
            <summary class="library-section-head"><span class="library-section-icon">${icon("workspace")}</span><span class="library-section-copy"><strong class="library-section-title" id="albums-library-title" role="heading" aria-level="3">Alben</strong><small>${library.albums.length} ${library.albums.length === 1 ? "Album" : "Alben"} · ${albumTrackCount} ${albumTrackCount === 1 ? "Track" : "Tracks"}</small></span><button type="button" class="album-create-button" data-action="create-album" aria-label="Neuen Albumordner anlegen" title="Neuen Albumordner anlegen">${icon("plus")} Album anlegen</button><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
            <div class="library-section-content">${library.albums.length
              ? `<div class="album-group-list">${library.albums.map((album) => this.renderAlbumGroup(album)).join("")}</div>`
              : this.renderLibraryEmpty("Noch keine Alben", "Lege hier zuerst einen Albumordner an.")}</div>
          </details>
          <details class="library-section" aria-labelledby="singles-library-title" open>
            <summary class="library-section-head"><span class="library-section-icon library-section-icon--single">${icon("tracks")}</span><span class="library-section-copy"><strong class="library-section-title" id="singles-library-title" role="heading" aria-level="3">Singles</strong><small>${library.singles.length} ${library.singles.length === 1 ? "Track" : "Tracks"}</small></span><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
            <div class="library-section-content">${library.singles.length
              ? `${this.renderTrackTableHead()}<div class="track-list">${library.singles.map((track) => this.renderTrackRow(track, true)).join("")}</div>`
              : this.renderLibraryEmpty("Keine passenden Singles", "Lege eine Single an oder passe Suche und Statusfilter an.")}</div>
          </details>
        </div>
      </section>
    </div>`;
  }

  private renderAlbumGroup(album: AlbumTrackGroup<TrackSummary>): string {
    return `<details class="album-group" open>
      <summary class="album-group-head"><span class="album-cover" aria-hidden="true">${escapeHtml(titleInitials(album.title))}<i></i></span><span class="album-group-copy"><strong>${escapeHtml(album.title)}</strong><small>${album.tracks.length} ${album.tracks.length === 1 ? "Track" : "Tracks"}</small></span><button type="button" class="album-rename-button" data-rename-album="${escapeHtml(album.title)}" aria-label="Album ${escapeHtml(album.title)} umbenennen" title="Albumordner umbenennen">Umbenennen</button><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
      <div class="album-group-content">${album.tracks.length
        ? `${this.renderTrackTableHead()}<div class="track-list">${album.tracks.map((track) => this.renderTrackRow(track, true)).join("")}</div>`
        : this.renderLibraryEmpty("Album ist noch leer", "Lege einen Track an und ordne ihn diesem Album zu.")}</div>
    </details>`;
  }

  private renderTrackTableHead(): string {
    return `<div class="track-table-head"><span>Track</span><span>Status</span><span>Fortschritt</span><span>Aktualisiert</span><span></span></div>`;
  }

  private renderLibraryEmpty(title: string, copy: string): string {
    return `<div class="library-empty">${icon("tracks")}<div><strong>${escapeHtml(title)}</strong><span>${escapeHtml(copy)}</span></div></div>`;
  }

  private renderTrackRow(track: TrackSummary, detailed = false): string {
    return `<button class="track-row ${detailed ? "track-row--detailed" : ""}" data-track-open="${escapeHtml(track.id)}">
      ${this.renderTrackCover(track)}
      <span class="track-identity"><strong>${escapeHtml(track.title)}</strong><small>${escapeHtml(track.relativePath)}${track.legacy ? " · Legacy-Import" : ""}</small></span>
      <span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span>
      <span class="row-progress"><progress max="100" value="${track.progress}" aria-label="${track.progress} Prozent"></progress><b>${track.progress}%</b></span>
      ${detailed ? `<time>${formatDate(track.updatedAt)}</time>` : `<span class="missing-hint">${track.missingCount ? `${track.missingCount} offen` : "Vollständig"}</span>`}
      <span class="row-arrow">${icon("arrow")}</span>
    </button>`;
  }

  private renderTrack(): string {
    const track = this.state.track;
    if (!track) return this.emptyState("current", "Kein Track ausgewählt", "Wähle einen Track aus deiner Bibliothek.", "go-tracks", "Tracks öffnen");
    const tabs: Array<[TrackTab, string]> = [["overview", "Übersicht"], ["suno", "Suno"], ["artwork", "Artwork"], ["release", "Release"], ["evidence", "Evidence"], ["certificate", "Zertifikat"]];
    const workflowUpgrade = workflowUpgradePresentation(track, this.state.workflow);
    const libraryLabel = track.library.section === "album" && track.library.albumTitle
      ? `Album · ${track.library.albumTitle}`
      : "Single";
    return `<div class="track-page">
      <section class="track-hero">
        ${this.renderTrackCover(track, "track-cover--hero", "div")}
        <div class="track-hero-copy"><div><span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span><span class="workflow-version">Workflow ${escapeHtml(track.workflowVersion)}</span><button class="library-chip" data-action="edit-track-library" title="${isTrackContentLocked(track.status) ? "Historischer Snapshot – Bibliothekszuordnung ist schreibgeschützt" : "Bibliothekszuordnung ändern"}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>${icon(track.library.section === "album" ? "workspace" : "tracks")} ${escapeHtml(libraryLabel)}</button></div><h2>${escapeHtml(track.title)}</h2><p>${escapeHtml(track.relativePath)}</p></div>
        <div class="hero-progress"><strong>${track.progress}%</strong><span>dokumentiert</span><progress class="progress-track" max="100" value="${track.progress}" aria-label="Dokumentationsfortschritt ${track.progress} Prozent"></progress></div>
      </section>
      <nav class="track-tabs" aria-label="Track-Ansichten">${tabs.map(([id, label]) => `<button class="${this.state.trackTab === id ? "is-active" : ""}" data-track-tab="${id}">${label}</button>`).join("")}</nav>
      <div class="track-content">${workflowUpgrade ? `<div class="policy-card">${icon("info")}<div><p class="overline">Workflow-Upgrade verfügbar</p><h4>${escapeHtml(workflowUpgrade.message)}</h4><p>Der bisherige Zertifikatssnapshot bleibt unverändert. Die Neubewertung verlangt aktuelle Dokumente, Prüfsummen und ein neues Zertifikat.</p></div>${workflowUpgrade.action ? `<button class="button button--secondary" data-action="${workflowUpgrade.action}">Mit aktuellem Workflow neu bewerten</button>` : ""}</div>` : ""}${this.renderTrackTab(track)}</div>
    </div>`;
  }

  private renderTrackTab(track: TrackDetail): string {
    if (this.state.activeStep) return this.renderWorkflowEditor(track, this.state.activeStep);
    switch (this.state.trackTab) {
      case "overview": return this.renderTrackOverview(track);
      case "suno": return this.renderWorkflowEditor(track, "suno");
      case "artwork": return this.renderWorkflowEditor(track, "artwork");
      case "release": return this.renderWorkflowEditor(track, "release");
      case "evidence": return `${this.renderFinalizedSnapshotNotice(track)}${this.renderEvidence(track)}`;
      case "certificate": return this.renderCertificate(track);
    }
  }

  private renderTrackOverview(track: TrackDetail): string {
    const missing = calculateMissingRequirements(track, track.profileSnapshot);
    const evaluatedStatuses = stepStatuses(track, track.profileSnapshot);
    const statuses = this.runtimeSteps().map((step) => evaluatedStatuses.find((state) => state.id === step.id) ?? { id: step.id, status: "NOT_RUN" as const });
    return `<div class="workflow-layout">
      <section class="workflow-main">
        ${this.renderFinalizedSnapshotNotice(track)}
        ${!isTrackContentLocked(track.status) && track.legacy && missingProfileFields(track.profileSnapshot).length ? `<div class="policy-card">${icon("info")}<div><p class="overline">Legacy-Track</p><h4>Historische Stammdaten ausdrücklich bestätigen</h4><p>Der Scan hat keine fehlenden Fakten erfunden. Übernimm die aktuellen Workspace-Stammdaten nur, wenn sie für diesen Track tatsächlich zutreffen; danach kannst du weitere Angaben prüfen und speichern.</p></div><button class="button button--secondary" data-action="adopt-legacy-profile">Stammdaten als Snapshot bestätigen</button></div>` : ""}
        ${this.renderTrackCheckSummary(track)}
        <div class="panel missing-panel ${missing.length === 0 ? "is-complete" : ""}">
          <div class="panel-heading"><div><p class="overline">Finalisierungs-Gate</p><h3>${missing.length ? "Was fehlt noch?" : "Bereit zur Finalisierung"}</h3></div><span class="missing-total">${missing.length}</span></div>
          ${missing.length ? `<ul class="missing-list">${missing.slice(0, 8).map((item) => `<li><span>${icon("alert")}</span><div><strong>${escapeHtml(item.label)}</strong><small>${escapeHtml(this.runtimeSteps().find((step) => step.id === item.stepId)?.title ?? item.stepId)}</small></div><button data-step-open="${item.stepId}">${item.evidenceRole ? "Nachweis" : "Öffnen"} ${icon("arrow")}</button></li>`).join("")}</ul>${missing.length > 8 ? `<button class="text-button" data-track-tab="evidence">Alle ${missing.length} offenen Punkte anzeigen</button>` : ""}` : `<div class="success-message">${icon("check")}<div><strong>Alle lokalen Vorprüfungen sind erfüllt.</strong><span>Rust validiert den Track vor der Finalisierung nochmals vollständig.</span></div></div>`}
        </div>
        <div class="panel workflow-panel"><div class="panel-heading"><div><p class="overline">Geführter Ablauf</p><h3>10 Dokumentationsschritte</h3></div><span class="workflow-id">${escapeHtml(this.state.workflow?.id ?? track.workflowId)} · v${escapeHtml(this.state.workflow?.version ?? track.workflowVersion)}</span></div>
          <div class="step-list">${statuses.map((step, index) => this.renderStepRow(step.id, step.status, index)).join("")}</div>
        </div>
      </section>
      <aside class="workflow-side">
        <div class="panel quick-actions"><p class="overline">Schnellaktionen</p><h3>Dokumentsatz</h3>
          ${this.actionRow("file", "Dokumente", track.documents.current ? "Aktuell" : track.documents.generated ? "Veraltet" : "Nicht erzeugt", "generate-documents", isTrackContentLocked(track.status))}
          ${this.actionRow("hash", "SHA-256", track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Nicht erzeugt", "calculate-hashes", isTrackContentLocked(track.status))}
          ${this.actionRow("shield", "Verifikation", track.integrity.verified ? "Bestanden" : "Offen", "verify-hashes")}
        </div>
        <div class="panel snapshot-card"><p class="overline">Track-Snapshot</p><dl><div><dt>Evidence</dt><dd>${track.evidence.length}</dd></div><div><dt>Dokumente</dt><dd>${track.documents.files.length}</dd></div><div><dt>Verifiziert</dt><dd>${track.integrity.verifiedCount}/${track.integrity.fileCount}</dd></div><div><dt>Abweichungen</dt><dd>${track.blockingDeviations?.filter((item) => item.blocking && !item.resolved).length ?? track.integrity.mismatchFiles.length}</dd></div></dl></div>
      </aside>
    </div>`;
  }

  private renderTrackCheckSummary(track: TrackDetail): string {
    const summary = trackCheckSummary(track);
    const issues = track.automation.consistencyIssues;
    const technicalFacts = [
      track.automation.sunoCreatedTimestamp
        ? `<li><strong>Suno-created</strong><span>${escapeHtml(track.automation.sunoCreatedTimestamp)}</span></li>`
        : "",
      track.automation.sunoId
        ? `<li><strong>Technische Suno-ID</strong><span>${escapeHtml(track.automation.sunoId)}</span></li>`
        : "",
      `<li><strong>Release identisch zum Suno-Export</strong><span>${track.automation.releaseIdenticalToSunoExport ? "PASS" : "Nicht nachgewiesen"}</span></li>`,
      `<li><strong>Byte-identische Paare</strong><span>${track.automation.byteIdenticalPairs.length}</span></li>`
    ].filter(Boolean).join("");
    return `<section class="panel check-summary">
      <div class="panel-heading"><div><p class="overline">Gesamtprüfung</p><h3>Dokumentationsstatus</h3></div><span class="check-warning-count ${summary.warningCount === 0 ? "is-clear" : ""}">${summary.warningCount} Warnungen</span></div>
      <div class="check-summary-grid">
        ${this.checkSummaryItem("Dokumentation", summary.documentation, summary.documentation === "vollständig")}
        ${this.checkSummaryItem("Dateiintegrität", summary.fileIntegrity, summary.fileIntegrity === "geprüft")}
        ${this.checkSummaryItem("Suno-Metadaten", summary.sunoMetadata, summary.sunoMetadata === "erkannt", summary.sunoMetadata === "nicht erkannt")}
        ${this.checkSummaryItem("Subscription-Zeitraum", summary.subscriptionCoverage, summary.subscriptionCoverage === "passend" || summary.subscriptionCoverage === "nicht erforderlich", summary.subscriptionCoverage === "nicht geprüft")}
      </div>
      <details class="check-details"><summary>Technische Details anzeigen</summary><ul>${technicalFacts}</ul>${issues.length ? `<div class="check-issues"><strong>Konsistenzhinweise</strong><ul>${issues.map((item) => `<li>${icon(item.blocking ? "alert" : "info")}<span>${escapeHtml(item.message)}</span></li>`).join("")}</ul></div>` : `<p class="check-details-clear">Keine Konsistenzabweichungen erkannt.</p>`}</details>
    </section>`;
  }

  private checkSummaryItem(label: string, value: string, passed: boolean, neutral = false): string {
    return `<div class="check-summary-item ${passed ? "is-pass" : neutral ? "is-neutral" : "is-open"}">${icon(passed ? "check" : neutral ? "info" : "alert")}<span><strong>${escapeHtml(label)}</strong><small>${escapeHtml(value)}</small></span></div>`;
  }

  private renderStepRow(stepId: StepId, status: string, index: number): string {
    const nativeStep = this.state.workflow?.steps.find((item) => item.id === stepId);
    const fallback = WORKFLOW_STEPS.find((item) => item.id === stepId)!;
    const number = nativeStep?.number ?? fallback.number;
    const label = nativeStep?.title ?? nativeStep?.label ?? fallback.title;
    const description = nativeStep?.description ?? fallback.description;
    return `<button class="step-row" data-step-open="${stepId}"><span class="step-number">${escapeHtml(number)}</span><span class="step-state step-state--${status.toLowerCase().replace("_", "-")}">${status === "PASS" || status === "N_A" ? icon("check") : status === "FAIL" || status === "BLOCKED" ? icon("alert") : index + 1}</span><span class="step-copy"><strong>${escapeHtml(label)}</strong><small>${escapeHtml(description)}</small></span><span class="step-status">${statusLabel(status as Parameters<typeof statusLabel>[0])}</span>${icon("arrow")}</button>`;
  }

  private runtimeSteps(): Array<{ id: StepId; number: string; title: string; description: string }> {
    if (this.state.workflow) {
      return this.state.workflow.steps.map((step) => ({
        id: step.id,
        number: step.number,
        title: step.title ?? step.label,
        description: step.description
      }));
    }
    // Explicit browser-demo/test fallback. Packaged Tauri builds use get_workflow().
    return WORKFLOW_STEPS.map((step) => ({ id: step.id, number: step.number, title: step.title, description: step.description }));
  }

  private actionRow(iconName: "file" | "hash" | "shield", label: string, state: string, action: string, disabled = false): string {
    return `<button class="action-row" data-action="${action}" ${disabled ? "disabled" : ""}><span>${icon(iconName)}</span><div><strong>${label}</strong><small>${state}</small></div>${icon("arrow")}</button>`;
  }

  private renderFinalizedSnapshotNotice(track: TrackDetail): string {
    const presentation = finalizedTrackPresentation(track);
    if (!presentation) return "";
    return `<div class="policy-card finalized-snapshot-notice ${presentation.invalid ? "is-invalid" : ""}">${icon(presentation.invalid ? "alert" : "lock")}<div><p class="overline">Revisionsschutz</p><h4>${escapeHtml(presentation.title)}</h4><p>${escapeHtml(presentation.message)}</p></div>${presentation.actionLabel ? `<button class="button ${presentation.invalid ? "button--danger" : "button--primary"}" data-action="create-revision">${icon("current")} ${escapeHtml(presentation.actionLabel)}</button>` : ""}</div>`;
  }

  private renderWorkflowEditor(track: TrackDetail, stepId: StepId): string {
    const runtimeSteps = this.runtimeSteps();
    const index = runtimeSteps.findIndex((step) => step.id === stepId);
    const definition = this.state.workflow?.steps.find((step) => step.id === stepId);
    const fallback = runtimeSteps[index];
    const label = definition?.title ?? definition?.label ?? fallback.title;
    const description = definition?.description ?? fallback.description;
    const statuses = stepStatuses(track, track.profileSnapshot);
    const currentStatus = statuses.find((step) => step.id === stepId)?.status ?? "NOT_RUN";
    const naEligible = !evaluateRequirements(track, track.profileSnapshot).some((requirement) => requirement.stepId === stepId);
    return `<div class="editor-layout">
      <aside class="workflow-rail">
        <button class="rail-back" data-action="back-overview">${icon("arrow")} Track-Übersicht</button>
        <div class="rail-progress"><span>${index + 1} / 10</span><progress max="10" value="${index + 1}" aria-label="Schritt ${index + 1} von 10"></progress></div>
        <nav aria-label="Workflow-Schritte">${runtimeSteps.map((runtimeStep) => {
          const step = statuses.find((entry) => entry.id === runtimeStep.id) ?? { id: runtimeStep.id, status: "NOT_RUN" as const };
          const item = this.state.workflow?.steps.find((entry) => entry.id === step.id);
          const itemFallback = runtimeStep;
          return `<button class="rail-step ${step.id === stepId ? "is-active" : ""} ${step.status === "PASS" || step.status === "N_A" ? "is-complete" : ""}" data-step-open="${step.id}"><span>${step.status === "PASS" || step.status === "N_A" ? icon("check") : item?.number ?? itemFallback.number}</span><strong>${escapeHtml(item?.title ?? item?.label ?? itemFallback.title)}</strong></button>`;
        }).join("")}</nav>
      </aside>
      <section class="editor-main">
        <header class="editor-head"><div><p class="overline">Schritt ${escapeHtml(definition?.number ?? fallback.number)}</p><h3>${escapeHtml(label)}</h3><p>${escapeHtml(description)}</p></div><span class="status-chip step-status-chip step-status-chip--${currentStatus.toLowerCase().replace("_", "-")}">${statusLabel(currentStatus)}</span></header>
        ${this.renderFinalizedSnapshotNotice(track)}
        ${naEligible ? `<div class="na-control">${icon("info")}<div><strong>Dieser Schritt hat für den aktuellen Track keine anwendbaren Pflichtpunkte.</strong><span>N/A wird nur mit einer gespeicherten sachlichen Begründung akzeptiert.</span></div>${currentStatus === "N_A" ? `<button class="button button--secondary" data-reset-na="${stepId}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>N/A zurücksetzen</button>` : `<button class="button button--secondary" data-mark-na="${stepId}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>Als N/A dokumentieren</button>`}</div>` : ""}
        ${this.renderStepContent(track, stepId)}
        <footer class="editor-footer">
          <button class="button button--secondary" ${index === 0 ? "disabled" : ""} data-step-open="${runtimeSteps[Math.max(index - 1, 0)].id}">${icon("arrow")} Zurück</button>
          ${index < runtimeSteps.length - 1 ? `<button class="button button--dark" data-step-open="${runtimeSteps[index + 1].id}">Weiter: ${escapeHtml(runtimeSteps[index + 1].title)} ${icon("arrow")}</button>` : ""}
        </footer>
      </section>
    </div>`;
  }

  private renderStepContent(track: TrackDetail, stepId: StepId): string {
    const draft = this.state.trackDraft ?? track.fields;
    const conditional = visibleConditionalFields(draft, track.profileSnapshot);
    if (stepId === "evidence_licenses") return this.renderEvidence(track, true);
    if (stepId === "integrity") return this.renderIntegrity(track);
    if (stepId === "finalize") return this.renderFinalization(track);

    let body = "";
    switch (stepId) {
      case "track":
        body = `<div class="field-grid two-col">${this.textField("title", "Track-Titel", "Name des Tracks", draft.title, true)}${this.dateField("productionStartDate", "Produktionsstart", draft.productionStartDate, true)}${this.automatedDateField("productionEndDate", "Produktionsende", draft.productionEndDate, track.automation.productionEndOrigin)}</div>
          <div class="form-section">${this.boolQuestion("commercialUseIntended", "Kommerzielle Nutzung vorgesehen?", "Der tatsächlich für diesen Track verwendete Wert wird im Dokument-Snapshot gespeichert.", draft.commercialUseIntended)}</div>`;
        break;
      case "source":
        body = `${this.boolQuestion("externalAudioUploaded", "Externes Audio hochgeladen?", "Audio außerhalb der eigenen Produktion, das Suno als Quelle erhalten hat.", draft.externalAudioUploaded)}
          ${conditional.has("externalAudioSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("externalAudioSource", "Quelle", draft.externalAudioSource, externalAudioSourceChoices, true)}${this.guidedSingleChoiceField("externalAudioOwnership", "Rechtezuordnung", draft.externalAudioOwnership, externalAudioRightsChoices, true)}</div>${this.inlineEvidenceActions(track, [["external_audio_file", "Audiodatei importieren"], ["external_audio_license", "Lizenznachweis importieren"]])}</div>` : ""}
          ${this.boolQuestion("ownAudioUploaded", "Eigene Audiodatei hochgeladen?", "Eine von dir erstellte Aufnahme oder Instrumentalspur.", draft.ownAudioUploaded)}
          ${conditional.has("ownAudioSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("ownAudioSource", "Quelle", draft.ownAudioSource, ownAudioSourceChoices, true)}${this.guidedSingleChoiceField("ownAudioOwnership", "Rechtezuordnung", draft.ownAudioOwnership, ownAudioRightsChoices, true)}</div>${this.inlineEvidenceActions(track, [["own_audio_file", "Eigene Audiodatei importieren"]])}</div>` : ""}
          ${this.boolQuestion("codeBasedGeneration", "Codebasierte Erzeugung?", "Wurde eine Audiodatei oder ein Ausgangsmaterial mithilfe von Quellcode erzeugt?", draft.codeBasedGeneration)}
          ${conditional.has("sourceCodeFile") ? `<div class="conditional-panel"><div class="conditional-line"></div><p class="field-help">Importiere zuerst den tatsächlich verwendeten Quellcode oder die Quelldatei.</p>${this.inlineEvidenceActions(track, [["source_code_file", "Quellcode oder Quelldatei importieren"]])}${this.boolQuestion("codeAudioPostProcessed", "Wurde das aus dem Quellcode erzeugte Audio nachbearbeitet?", "Es werden nur ausdrücklich bestätigte Bearbeitungen dokumentiert.", draft.codeAudioPostProcessed)}${conditional.has("codeAudioPostProcessingOperations") ? this.multiChoiceArrayField("codeAudioPostProcessingOperations", "Welche Nachbearbeitungen wurden durchgeführt?", draft.codeAudioPostProcessingOperations, codeAudioPostProcessingChoices, true) : ""}${conditional.has("codeAudioPostProcessingNote") ? this.textArea("codeAudioPostProcessingNote", "Sonstige Nachbearbeitung – Details", "Frei beschreibbare zusätzliche Nachbearbeitung", draft.codeAudioPostProcessingNote) : ""}<div class="form-section"><p class="field-label">Erzeugte Audio-Datei</p><p class="field-help">Importiere abschließend die tatsächlich aus dem Quellcode erzeugte WAV- oder MP3-Datei.</p>${this.inlineEvidenceActions(track, [["code_generated_audio_file", "Erzeugte WAV- oder MP3-Datei importieren"]])}</div></div>` : ""}
          ${this.boolQuestion("thirdPartySamplesUploaded", "Fremde Samples hochgeladen?", "Samples oder Loops, die von Dritten stammen.", draft.thirdPartySamplesUploaded)}
          ${conditional.has("thirdPartySampleSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("thirdPartySampleSource", "Sample-Quelle", draft.thirdPartySampleSource, sampleSourceChoices, true)}${this.guidedSingleChoiceField("thirdPartySampleOwnership", "Lizenz / Rechte", draft.thirdPartySampleOwnership, sampleRightsChoices, true)}</div>${this.inlineEvidenceActions(track, [["third_party_sample_file", "Sample-Datei importieren"], ["third_party_sample_license", "Sample-Lizenz importieren"]])}</div>` : ""}`;
        break;
      case "suno": {
        const projectHint = track.evidence.find((item) => item.role === "suno_screenshot");
        body = `<div class="field-grid two-col">${this.suggestedTextField("sunoModel", "Suno-Modell", "z. B. v5.5 oder eigener Wert", draft.sunoModel, sunoModelSuggestions, true)}${this.suggestedTextField("sunoPlanAtCreation", "Tarif bei Erstellung", "z. B. Premier oder historischer Tarif", draft.sunoPlanAtCreation, sunoPlanSuggestions, true)}${this.textField("sunoProjectUrl", "Suno-Projekt-URL", "https://suno.com/song/…", draft.sunoProjectUrl, true, "url")}${this.automatedDateField("sunoFinalGenerationDate", "Datum der finalen Generation", draft.sunoFinalGenerationDate, track.automation.finalGenerationOrigin)}${this.automatedDateField("sunoDownloadExportDate", "Download-/Exportdatum (optional)", draft.sunoDownloadExportDate, track.automation.downloadExportOrigin, false, "Kein gültiges Datum in den WAV-Metadaten erkannt – die manuelle Angabe bleibt optional.")}</div>
          ${this.renderAutomaticSunoMetadata(track)}
          <div class="form-section"><p class="field-label">Suno-Projektnachweis</p>${projectHint ? `<p class="field-help">Sunoprojekthinweis hinterlegt: <strong>${escapeHtml(projectHint.fileName)}</strong></p>` : ""}${this.inlineEvidenceActions(track, [["suno_screenshot", "Screenshot importieren"], ["suno_project_zip", "Projekt-ZIP importieren"], ["suno_final_export", "Suno-Export importieren"]])}</div>`;
        body += this.filenameConfirmation(track, "suno_final_export", "sunoExportFilenameDifferenceConfirmed", draft.sunoExportFilenameDifferenceConfirmed, "Suno-Export");
        break;
      }
      case "human_work":
        body = `${this.boolQuestion("instrumentalTrack", "Ist dies ein Instrumentaltrack?", "Diese ausdrückliche Angabe wird gegen Lyrics-Quelle, Lyrics-Text und Human Work geprüft; Widersprüche werden nicht automatisch korrigiert.", draft.instrumentalTrack)}<div class="field-grid two-col">${singleChoiceFieldMarkup("lyricsSource", "Lyrics-Quelle", draft.lyricsSource, [["instrumental", "Instrumental – keine Lyrics"], ["human", "Menschlich geschrieben"], ["suno", "Von Suno erzeugt"], ["mixed", "Gemischt"]], true)}</div>
          ${conditional.has("lyricsText") ? this.textArea("lyricsText", "Verwendeter Lyrics-Text", "Nur die tatsächlich in Suno verwendete Fassung dokumentieren.", draft.lyricsText, true) : ""}
          ${this.textArea("sunoStylePrompt", "Suno-Style-Prompt", "Den in Suno verwendeten Style-Prompt vollständig dokumentieren.", draft.sunoStylePrompt, true)}
          ${this.boolQuestion("humanEditingPerformed", "Menschliche Bearbeitung durchgeführt?", "Nur bestätigen, wenn sie tatsächlich stattgefunden hat.", draft.humanEditingPerformed)}
          ${conditional.has("humanEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceField("humanEditingDetails", "Bestätigte Schritte", draft.humanEditingDetails, humanWorkChoices, true)}</div>` : ""}`;
        break;
      case "artwork":
        body = `<div class="policy-card artwork-factual-notice">${icon("info")}<div><p class="overline">Nur relevante Angaben</p><h4>Faktische Dokumentation</h4><p>Die App dokumentiert deine Bestätigung und trifft keine rechtliche Entscheidung.</p></div></div><div class="field-grid two-col">${this.selectField("artworkOrigin", "Entstehung des Artworks", draft.artworkOrigin, [["", "Bitte auswählen"], ["none", "Kein Artwork"], ["human", "Menschlich erstellt"], ["ai_generated", "KI-generiert"], ["ai_assisted", "KI-assistiert"]], true)}${conditional.has("aiImageService") ? this.textField("aiImageService", "KI-Bilddienst", "Verwendeter Dienst", draft.aiImageService, true) : ""}</div>
          <div class="form-section"><p class="field-label">Suno-Original-Artwork</p>${this.inlineEvidenceActions(track, [["artwork_suno_original", "Suno-Original importieren"]])}</div>
          ${conditional.has("humanArtworkProcessOperations") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceArrayField("humanArtworkProcessOperations", "Menschlicher Arbeitsprozess", draft.humanArtworkProcessOperations, humanArtworkProcessChoices)}${this.textArea("humanArtworkProcessNotes", "Beschreibung / Ergänzungen", "Arbeitsprozess frei beschreiben oder die Auswahl ergänzen", draft.humanArtworkProcessNotes)}</div>` : ""}
          ${conditional.has("humanArtworkModifications") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceArrayField("humanArtworkModifications", "Menschliche Änderungen", draft.humanArtworkModifications, aiArtworkHumanChangeChoices, true)}${conditional.has("customArtworkChange") ? this.textArea("customArtworkChange", "Sonstige menschliche Bearbeitung – Details", "Frei beschreibbare zusätzliche Änderung", draft.customArtworkChange) : ""}</div>` : ""}
          ${conditional.has("aiArtworkOriginal") ? `<div class="form-section"><p class="field-label">Originale KI-Ausgabe</p>${this.inlineEvidenceActions(track, [["ai_artwork_original", "KI-Original importieren"], ["ai_artwork_edited", "KI-bearbeitete Version importieren"], ["human_edited_artwork", "Menschlich bearbeitete Version importieren"]])}</div>` : ""}
          ${draft.artworkOrigin && draft.artworkOrigin !== "none" ? `<div class="question-group"><div><p class="overline">Artwork Content Check</p><h4>Ja oder Nein auswählen</h4><p>Folgeangaben erscheinen nur bei Ja und bleiben frei beschreibbar.</p></div>${this.boolQuestion("depictsRealPerson", "Zeigt das Artwork absichtlich eine reale Person?", "", draft.depictsRealPerson)}${conditional.has("realPersonNotes") ? this.textArea("realPersonNotes", "Welche reale Person wird dargestellt bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.realPersonNotes, true) : ""}${this.boolQuestion("depictsRealEvent", "Stellt es ein reales Ereignis als authentisch dar?", "", draft.depictsRealEvent)}${conditional.has("realEventNotes") ? this.textArea("realEventNotes", "Welches reale Ereignis wird dargestellt bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.realEventNotes, true) : ""}${this.boolQuestion("containsTrademark", "Reproduziert es eine Marke oder ein Firmenlogo?", "", draft.containsTrademark)}${conditional.has("trademarkNotes") ? this.textArea("trademarkNotes", "Welche Marke oder welches Firmenlogo wird reproduziert bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.trademarkNotes, true) : ""}</div><div class="form-section"><p class="field-label">Finales Artwork</p><p class="field-help">Lade hier die endgültige JPG- oder PNG-Datei hoch, die du aus Suno heruntergeladen hast. Falls ein sichtbarer KI-Hinweis erforderlich ist, ersetzt du sie anschließend durch die lokal gekennzeichnete Fassung.</p>${this.inlineEvidenceActions(track, [["final_artwork", "Finales Suno-Artwork importieren"]])}</div>` : ""}`;
        break;
      case "ai_transparency":
        body = `<div class="policy-card">${icon("info")}<div><p class="overline">Projektinterne Transparenzrichtlinie · Track-Snapshot</p><h4>${this.policyLabel(track.profileSnapshot.artworkTransparencyPolicy)}</h4><p>Dies ist die aktuell für den Track gespeicherte Projektregel – keine pauschale gesetzliche Aussage. Globale Änderungen aktualisieren offene Tracks; finalisierte Snapshots bleiben unverändert.</p></div></div>
          ${conditional.has("disclosure") ? `<div class="field-grid two-col">${this.textField("disclosureText", "Sichtbarer Hinweis", "AI-assisted", draft.disclosureText, true)}${track.profileSnapshot.artworkTransparencyPolicy === "per_artwork" ? this.boolQuestion("disclosureApplied", "Sichtbaren Hinweis anwenden?", "Bei Ja muss die gekennzeichnete Fassung lokal erzeugt werden.", draft.disclosureApplied) : `<div class="read-only-field"><span>Status</span><strong>${draft.disclosureApplied ? "Lokal erzeugt" : "Noch nicht erzeugt"}</strong></div>`}</div><button type="button" class="button button--accent" data-action="generate-disclosure">${icon("certificate")} Sichtbaren Hinweis lokal erzeugen</button>` : `<div class="neutral-message">${icon("check")}<div><strong>AI Transparency ist für diesen Track deaktiviert.</strong><span>${contentCheckAllNegative(draft) ? "Alle drei Content-Checks wurden mit Nein beantwortet." : "Grundlage: Artwork-Angabe und aktive Workspace-Policy."}</span></div></div>`}`;
        break;
      case "release":
        {
          const metadataDate = track.automation.sunoMetadataDetected && track.automation.sunoCreatedTimestamp
            ? track.automation.sunoCreatedTimestamp.slice(0, 10)
            : "";
          const noDesktopEditing = draft.postExportEditingPerformed === false;
          const metadataControlsLastEditing = noDesktopEditing && Boolean(metadataDate);
          const finalDateOrigin: FactOrigin = metadataControlsLastEditing
            ? "evidence_derived_metadata"
            : track.automation.finalExportOrigin === "evidence_derived_metadata"
              ? "not_documented"
              : track.automation.finalExportOrigin;
          const finalDateValue = metadataControlsLastEditing
            ? metadataDate
            : track.automation.finalExportOrigin === "evidence_derived_metadata"
              ? ""
              : draft.finalExportDate;
          body = `${this.boolQuestion("postExportEditingPerformed", "Wurde die Datei noch einmal auf dem Desktop-PC bearbeitet?", "Bei Ja dokumentierst du das Datum der letzten Bearbeitung selbst. Bei Nein übernimmt die App das erkannte Datum aus der Suno-WAV.", draft.postExportEditingPerformed)}
          ${conditional.has("postExportEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceField("postExportEditingDetails", "Bestätigte Bearbeitungsschritte auf dem Desktop-PC", draft.postExportEditingDetails, postExportWorkChoices, true)}</div>` : ""}
          ${draft.postExportEditingPerformed === null
            ? `<div class="neutral-message">${icon("info")}<div><strong>Datum der letzten Bearbeitung</strong><span>Beantworte zuerst die Ja-/Nein-Frage.</span></div></div>`
            : `<div class="field-grid two-col">${this.automatedDateField("finalExportDate", "Datum der letzten Bearbeitung", finalDateValue, finalDateOrigin, true, "Kein gültiges WAV-Metadatum erkannt – bitte Datum manuell dokumentieren.")}${this.multiChoiceField("releaseNotes", "Release-Notizen", draft.releaseNotes, releaseNoteChoices)}</div>`}
          <div class="form-section"><p class="field-label">Finale Release-Dateien</p><p class="field-help">Der ursprüngliche Quelldateiname wird getrennt vom verwalteten Pfad dokumentiert. Das finale Artwork wird einmalig in Schritt 05 verwaltet.</p>${this.inlineEvidenceActions(track, [["release_wav", "Finale Release-Audiodatei importieren"], ["release_mp3", "Zusätzliche MP3 importieren"], ["release_mp4", "MP4 importieren"]])}</div>`;
          body += this.filenameConfirmation(track, "release_wav", "releaseFilenameDifferenceConfirmed", draft.releaseFilenameDifferenceConfirmed, "Release-Datei");
        }
        break;
    }
    const locked = isTrackContentLocked(track.status);
    return `<form id="track-step-form" class="workflow-form ${locked ? "is-read-only" : ""}" data-step="${stepId}" ${locked ? `aria-label="Historischer Snapshot – schreibgeschützt"` : ""}><fieldset class="workflow-form-fields" ${locked ? "disabled" : ""}>${this.renderStepConsistencyIssues(track, stepId)}${body}</fieldset>${locked ? "" : `<div class="form-save"><span>${icon("shield")} Änderungen bleiben lokal im Workspace.</span><button class="button button--primary" type="submit">${icon("check")} Schritt speichern</button></div>`}</form>`;
  }

  private renderEvidence(track: TrackDetail, embedded = false): string {
    const locked = isTrackContentLocked(track.status);
    const hasTermsEvidence = track.evidence.some((item) => item.role === "suno_terms_rights" && item.verified);
    const missing = calculateMissingRequirements(track, track.profileSnapshot).filter((item) => item.evidenceRole);
    return `<div class="${embedded ? "embedded-content" : "evidence-page"}">
      <div class="section-intro"><div><p class="overline">Lokale Nachweise</p><h3>Evidence & Lizenzen</h3><p>Originale bleiben am Quellort. Importierte Kopien werden gehasht und niemals still überschrieben.</p></div><button class="button button--primary" data-action="import-evidence" ${locked ? "disabled" : ""}>${icon("upload")} Evidence importieren</button></div>
      ${this.renderStepConsistencyIssues(track, "evidence_licenses")}
      ${missing.length ? `<div class="evidence-needed"><strong>${missing.length} erforderliche Nachweise fehlen</strong><div>${missing.map((item) => {
        if (item.evidenceRole === "subscription_payment") return `<span class="evidence-reminder">${icon("info")} ${escapeHtml(item.label)} – unten aus globaler Evidence zuordnen</span>`;
        if (item.evidenceRole === "suno_terms_rights") return `<span class="evidence-reminder">${icon("info")} ${escapeHtml(item.label)} – globale Datei unter Einstellungen registrieren</span>`;
        const current = [...track.evidence].reverse().find((evidence) => evidence.role === item.evidenceRole);
        return `<button data-import-role="${item.evidenceRole}" ${current ? `data-replace-evidence="${escapeHtml(current.id)}"` : ""} ${locked ? "disabled" : ""}>${icon(current ? "upload" : "plus")}<span><strong>${escapeHtml(item.label)}</strong><small>${current ? "Vorhandene Datei sicher ersetzen" : escapeHtml(evidenceRoleLabel(item.evidenceRole!))}</small><small>Gefordert: ${escapeHtml(evidenceRoleFileTypes(item.evidenceRole!))}</small></span></button>`;
      }).join("")}</div></div>` : ""}
      ${track.fields.commercialUseIntended ? this.renderGlobalEvidencePicker(track, locked) : ""}
      ${track.fields.commercialUseIntended ? this.renderGlobalTermsEvidencePicker(track, locked, hasTermsEvidence) : ""}
      <div class="form-section"><p class="field-label">Optionaler externer Zeitstempel</p>${this.inlineEvidenceActions(track, [["external_timestamp", "Zeitstempelnachweis importieren"]])}</div>
      <div class="panel evidence-table-panel"><div class="evidence-table-head"><span>Datei</span><span>Rolle</span><span>Integrität</span><span>Größe</span><span></span></div>
        ${track.evidence.length ? `<div class="evidence-list">${track.evidence.map((item) => this.renderEvidenceRow(item, locked)).join("")}</div>` : this.emptyState("file", "Noch keine Evidence", "Importiere echte lokale Dateien über den nativen Dateidialog.")}
      </div>
      <div class="deviation-section"><div class="section-intro compact"><div><p class="overline">Abweichungen</p><h3>Offene Hinweise & Blocker</h3></div><button class="button button--secondary" data-action="add-deviation" ${locked ? "disabled" : ""}>${icon("plus")} Abweichung erfassen</button></div>${this.renderDeviations(track, locked)}</div>
    </div>`;
  }

  private renderEvidenceRow(item: EvidenceItem, locked: boolean): string {
    const sunoTechnical = item.role === "suno_final_export" && item.metadata?.sunoStudioDetected
      ? `<small class="evidence-suno-meta">Suno Studio · ${escapeHtml(item.metadata.sunoCreatedTimestamp || "Zeitstempel nicht dokumentiert")}${item.metadata.sunoId ? ` · ID ${escapeHtml(item.metadata.sunoId)}` : ""}</small>`
      : "";
    return `<div class="evidence-row"><span class="file-icon">${icon("file")}</span><button class="evidence-name" data-preview-evidence="${escapeHtml(item.id)}" title="Evidence-Vorschau öffnen"><strong>${escapeHtml(item.fileName)}</strong><small>${escapeHtml(item.relativePath)} · ${escapeHtml(evidenceProvenanceLabel(item.provenance))}</small>${item.metadata?.documentTitle ? `<small>${escapeHtml(item.metadata.documentTitle)} · ${escapeHtml(item.metadata.provider)}</small>` : ""}${sunoTechnical}</button><span>${escapeHtml(evidenceRoleLabel(item.role))}</span><span class="verification ${item.verified ? "is-valid" : ""}">${item.verified ? icon("check") + " Verifiziert" : "Nicht verifiziert"}</span><span>${formatBytes(item.sizeBytes)}</span><span class="row-actions"><button class="icon-button" data-verify-evidence="${escapeHtml(item.id)}" aria-label="Evidence prüfen">${icon("shield")}</button><button class="icon-button danger" data-remove-evidence="${escapeHtml(item.id)}" aria-label="Evidence entfernen" ${locked ? "disabled" : ""}>${icon("trash")}</button></span></div>`;
  }

  private renderDeviations(track: TrackDetail, locked = false): string {
    const deviations = track.blockingDeviations ?? [];
    if (!deviations.length) return `<div class="neutral-message">${icon("check")}<div><strong>Keine Abweichungen erfasst.</strong><span>Ungeklärte blockierende Abweichungen verhindern die Finalisierung.</span></div></div>`;
    return `<div class="deviation-list">${deviations.map((item) => `<article class="deviation ${item.resolved ? "is-resolved" : ""}">${icon(item.resolved ? "check" : "alert")}<div><strong>${escapeHtml(item.title)}</strong><p>${escapeHtml(item.description)}</p><small>${item.resolved ? `Gelöst ${formatDate(item.resolvedAt, true)}` : `Erfasst ${formatDate(item.createdAt, true)}`}</small></div><div>${!item.resolved ? `<button class="button button--small button--secondary" data-resolve-deviation="${item.id}" ${locked ? "disabled" : ""}>Als gelöst markieren</button>` : ""}<button class="icon-button danger" data-remove-deviation="${item.id}" aria-label="Abweichung entfernen" ${locked ? "disabled" : ""}>${icon("trash")}</button></div></article>`).join("")}</div>`;
  }

  private renderStepConsistencyIssues(track: TrackDetail, stepId: StepId): string {
    const issues = track.automation.consistencyIssues.filter((item) => item.stepId === stepId);
    if (!issues.length) return "";
    return `<div class="consistency-notices">${issues.map((item) => `<div class="danger-banner ${item.blocking ? "" : "is-warning"}">${icon(item.blocking ? "alert" : "info")}<div><strong>${item.blocking ? "Abweichung erkannt" : "Hinweis"}</strong><span>${escapeHtml(item.message)}</span></div></div>`).join("")}</div>`;
  }

  private renderGlobalEvidencePicker(track: TrackDetail, locked = false): string {
    const subscriptions = this.state.globalEvidence.filter((item) => item.role === "subscription_payment");
    const attachedIds = new Set(track.evidence
      .filter((item) => item.role === "subscription_payment")
      .map((item) => item.sourceGlobalEvidenceId)
      .filter((value): value is string => Boolean(value)));
    const productionCoverage = subscriptionProductionCoverageStatus(track.evidence, track.fields);
    const generationCoverage = subscriptionGenerationCoverageStatus(track.evidence, track.fields);
    return `<section class="global-picker"><div><p class="overline">Global registriert</p><h4>Abo-Nachweis für Produktionszeitraum und Finalgeneration</h4><p>Beim Zuordnen kopiert der native Dienst den Nachweis in den Track-Ordner. Produktionszeitraum: <strong>${productionCoverage.replace("_", " ")}</strong> · Finalgeneration: <strong>${generationCoverage.replace("_", " ")}</strong>. Mehrere lückenlos anschließende Abrechnungszeiträume werden gemeinsam gewertet. Dies ist ausschließlich ein Datumsabgleich, keine Rechteaussage.</p></div>${subscriptions.length ? `<div>${subscriptions.map((item) => {
      const coverage = subscriptionEvidenceRelevance(item, track.fields);
      const attached = attachedIds.has(item.id);
      const productionLabel = coverage.coversProduction ? "YES" : coverage.overlapsProduction ? "TEILWEISE" : "NO";
      return `<article class="${coverage.relevant || attached ? "is-covering" : ""}">${icon("file")}<span><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart)} – ${formatDate(item.coverageEnd)} · Produktion: ${productionLabel} · Finalgeneration: ${coverage.coversGeneration ? "YES" : track.fields.sunoFinalGenerationDate ? "NO" : "NOT VERIFIED"}</small></span><button class="button button--small button--secondary" data-attach-global="${item.id}" ${locked || attached || !coverage.relevant ? "disabled" : ""}>${attached ? "Zugeordnet" : coverage.relevant ? "Diesem Track zuordnen" : "Nicht passend"}</button></article>`;
    }).join("")}</div>` : `<p class="empty-inline">Noch keine globale Abo-Evidence. Registriere sie unter Einstellungen.</p>`}</section>`;
  }

  private renderGlobalTermsEvidencePicker(track: TrackDetail, locked: boolean, hasTermsEvidence: boolean): string {
    const terms = this.state.globalEvidence.filter((item) => item.role === "suno_terms_rights");
    const attachedIds = new Set(track.evidence
      .filter((item) => item.role === "suno_terms_rights")
      .map((item) => item.sourceGlobalEvidenceId)
      .filter((value): value is string => Boolean(value)));
    return `<section class="global-picker"><div><p class="overline">Globale Service-Terms-Evidence</p><h4>Archivierte Suno-Nutzungsbedingungen</h4><p>Die Datei wird einmal unter Einstellungen lokal registriert und als gehashte portable Kopie in nicht finalisierte Projekte übernommen. SunoDM trifft keine Rechte- oder Gültigkeitsaussage.</p></div>
      ${terms.length ? `<div>${terms.map((item) => {
        const attached = attachedIds.has(item.id);
        return `<article class="${attached ? "is-covering" : ""}">${icon("file")}<span><strong>${escapeHtml(item.fileName)}</strong><small>Lokale, gehashte PDF</small></span><button class="button button--small button--secondary" data-attach-global="${item.id}" ${locked || attached ? "disabled" : ""}>${attached ? "Im Projekt hinterlegt" : "Diesem Projekt zuordnen"}</button></article>`;
      }).join("")}</div>` : `<p class="empty-inline">Noch keine globalen Suno-Nutzungsbedingungen. Registriere die Datei unter Einstellungen.</p>`}
      <button class="button button--secondary" data-action="terms-unavailable" ${locked || (hasTermsEvidence && track.fields.sunoTermsEvidenceNotAvailable !== true) ? "disabled" : ""}>${hasTermsEvidence && track.fields.sunoTermsEvidenceNotAvailable === true ? "Widerspruch: unavailable-Status zurücknehmen" : hasTermsEvidence ? icon("check") + " Globaler Terms-Nachweis im Projekt hinterlegt" : track.fields.sunoTermsEvidenceNotAvailable === true ? icon("check") + " Terms evidence not available – Status zurücknehmen" : "Terms evidence not available dokumentieren"}</button>
    </section>`;
  }

  private renderIntegrity(track: TrackDetail): string {
    const mismatches = track.integrity.mismatchFiles;
    const locked = isTrackContentLocked(track.status);
    return `<div class="integrity-page">
      <div class="integrity-hero ${track.integrity.verified && !mismatches.length ? "is-valid" : ""}"><span>${icon("shield")}</span><div><p class="overline">SHA-256 Integrität</p><h3>${track.integrity.verified ? "Dateien erfolgreich verifiziert" : "Integritätsprüfung ausstehend"}</h3><p>${track.integrity.fileCount} Dateien gehasht · ${track.integrity.verifiedCount} Dateien verifiziert</p></div><strong>${track.integrity.verified && !mismatches.length ? "PASS" : track.integrity.generated ? "NICHT VERIFIZIERT" : "NICHT ERZEUGT"}</strong></div>
      ${mismatches.length ? `<div class="danger-banner">${icon("alert")}<div><strong>${mismatches.length} Integritätsabweichungen</strong><span>${mismatches.map(escapeHtml).join(", ")}</span></div></div>` : ""}
      <div class="integrity-actions"><article class="action-card">${icon("file")}<div><h4>1. Dokumente erzeugen</h4><p>Versionierte Markdown- und Textdokumente aus den aktuellen Angaben erstellen.</p><span>${track.documents.current ? "Aktuell · " + formatDate(track.documents.generatedAt, true) : "Ausstehend oder veraltet"}</span></div><button class="button button--secondary" data-action="generate-documents" ${locked ? "disabled" : ""}>Erzeugen</button></article>
      <article class="action-card">${icon("hash")}<div><h4>2. SHA-256 berechnen</h4><p>Alle relevanten Dateien in einer extern prüfbaren Hashliste erfassen.</p><span>${track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Ausstehend"}</span></div><button class="button button--secondary" data-action="calculate-hashes" ${locked || !track.documents.current ? "disabled" : ""}>Berechnen</button></article>
      <article class="action-card">${icon("shield")}<div><h4>3. Prüfsummen verifizieren</h4><p>Hashliste erneut lesen und jede erfasste Datei nativ überprüfen.</p><span>${track.integrity.verified ? formatDate(track.integrity.verifiedAt, true) : "Ausstehend"}</span></div><button class="button button--primary" data-action="verify-hashes" ${!track.integrity.generated ? "disabled" : ""}>Verifizieren</button></article></div>
      <div class="technical-note">${icon("info")}<p><strong>Unabhängig prüfbar.</strong> SHA256SUMS.txt bleibt möglichst mit <code>sha256sum -c</code> kompatibel. Zertifikat, Archiv und interne Verwaltungsdaten werden nicht in dieselbe Hashliste aufgenommen.</p></div>
    </div>`;
  }

  private renderFinalization(track: TrackDetail): string {
    const localGate = finalizationGate(track, track.profileSnapshot);
    const workflowBlocker = workflowUpgradeFinalizationBlocker(track, this.state.workflow);
    const blockers = [...localGate.missingItems, ...localGate.blockingItems, ...(workflowBlocker ? [workflowBlocker] : [])];
    const finalized = track.status === "FINALIZED";
    const superseded = track.status === "SUPERSEDED";
    const locked = isTrackContentLocked(track.status);
    const ready = localGate.valid && !workflowBlocker;
    const heading = superseded
      ? "Ersetzter Snapshot – nur lesbar"
      : finalized
      ? track.certificate.valid ? "Dokumentation finalisiert" : "Finalisierter Snapshot nicht mehr gültig"
      : workflowBlocker ? "Workflow-Neubewertung erforderlich"
      : localGate.valid ? "Bereit für den Abschluss" : "Finalisierung noch blockiert";
    const summary = superseded
      ? "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt und bleibt unverändert. Navigation und Integritätsprüfungen sind weiterhin möglich."
      : finalized
      ? track.certificate.valid
        ? "Dieser Snapshot ist abgeschlossen und schreibgeschützt. Verwende die Revisionsaktion oben, um eine bearbeitbare Folgeversion anzulegen."
        : "Das bisherige Zertifikat ist ungültig. Verwende die Revisionsaktion oben, um die Abweichung in einer neuen Folgeversion zu bearbeiten."
      : workflowBlocker
        ? "Wähle zuerst die explizite Neubewertung mit dem aktuellen Workflow. Danach müssen Dokumente und Prüfsummen erneut erzeugt werden."
      : localGate.valid
        ? "Die UI-Vorprüfung ist vollständig. Der native Dienst validiert vor dem Erzeugen des Zertifikats nochmals alle Pflichtschritte, Evidence und Hashes."
        : `${blockers.length} Punkte müssen vor dem Abschluss geklärt werden.`;
    return `<div class="finalize-page">
      <div class="finalize-mark ${ready && !locked ? "is-ready" : ""}">${icon(ready && !locked ? "certificate" : "lock")}</div>
      <p class="overline">Track Documentation Completion Certificate</p><h3>${heading}</h3><p>${summary}</p>
      ${blockers.length ? `<ul class="gate-list">${blockers.map((item) => `<li>${icon("alert")}<span>${escapeHtml(item)}</span></li>`).join("")}</ul>` : `<ul class="gate-list gate-list--success"><li>${icon("check")}<span>Pflichtschritte erfüllt</span></li><li>${icon("check")}<span>Evidence vollständig</span></li><li>${icon("check")}<span>Dokumente aktuell</span></li><li>${icon("check")}<span>SHA-256 vollständig verifiziert</span></li></ul>`}
      ${locked
        ? track.certificate.valid && track.certificate.certificateId
          ? `<button class="button button--finalize" data-action="show-certificate-popup">${icon("certificate")} Zertifikat anzeigen</button>`
          : ""
        : `<button class="button button--finalize" data-action="finalize-track" ${!ready ? "disabled" : ""}>${icon("certificate")} Dokumentation finalisieren</button>`}
      <p class="certificate-disclaimer">Das Zertifikat bestätigt ausschließlich den Abschluss des konfigurierten Dokumentations- und Integritätsworkflows. Es ist keine behördliche Zertifizierung, Rechtsberatung oder unabhängige Feststellung von Urheberschaft oder Rechtskonformität.</p>
    </div>`;
  }

  private renderCertificate(track: TrackDetail): string {
    if (!track.certificate.certificateId) return `<div class="certificate-empty">${icon("certificate")}<p class="overline">Zertifikat</p><h3>Noch kein Completion Certificate</h3><p>Das Zertifikat wird erst nach erfolgreicher nativer Finalisierungsprüfung erzeugt.</p>${canCreateTrackRevision(track.status) ? `<button class="button button--primary" data-action="create-revision">${icon("current")} Neue Revision anlegen und bearbeiten</button>` : isTrackContentLocked(track.status) ? "" : `<button class="button button--dark" data-step-open="finalize">Finalisierungs-Gate öffnen ${icon("arrow")}</button>`}</div>`;
    const deviations = (track.blockingDeviations ?? []).filter((item) => item.blocking && !item.resolved);
    return `<div class="certificate-view ${track.certificate.valid ? "is-valid" : "is-invalid"}">
      <div class="certificate-paper"><header><div class="certificate-seal">${icon("certificate")}</div><div><p>Suno Documentation Manager</p><h3>Track Documentation<br>Completion Certificate</h3></div><span class="certificate-result">${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "CERTIFICATE INVALID"}</span></header>
      <div class="certificate-rule"></div><dl><div><dt>Certificate ID</dt><dd>${escapeHtml(track.certificate.certificateId)}</dd></div><div><dt>Track</dt><dd>${escapeHtml(track.title)}</dd></div><div><dt>Artist</dt><dd>${escapeHtml(track.profileSnapshot.artistName)}</dd></div><div><dt>Workflow</dt><dd>${escapeHtml(track.workflowId)} · ${escapeHtml(track.certificate.workflowVersion ?? track.workflowVersion)}</dd></div><div><dt>Finalisierung</dt><dd>${formatDate(track.certificate.finalizedAt, true)}</dd></div><div><dt>Evidence-Dateien</dt><dd>${track.evidence.length}</dd></div><div><dt>Blockierende Abweichungen</dt><dd>${deviations.length}</dd></div><div><dt>Finales Ergebnis</dt><dd>${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "INVALID"}</dd></div></dl>
      <footer>This certificate confirms completion of the configured documentation workflow and integrity checks. It does not constitute governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.</footer></div>
      <div class="certificate-actions">${track.certificate.valid ? `<button class="button button--secondary" data-action="show-certificate-popup">${icon("certificate")} Zertifikatsübersicht öffnen</button>` : ""}${canCreateTrackRevision(track.status) ? `<button class="button button--primary" data-action="create-revision">${icon("current")} Neue Revision anlegen und bearbeiten</button>` : ""}${track.certificate.valid && canCreateTrackRevision(track.status) ? `<button class="button button--danger-soft" data-action="invalidate-certificate">Zertifikat invalidieren</button>` : ""}</div>
    </div>`;
  }

  private renderWorkspace(): string {
    const scan = this.state.scanResult;
    return `<div class="page-content workspace-page">
      <div class="page-lead"><div><p class="overline">Lokaler Projektordner</p><h2>${escapeHtml(this.state.workspace?.name)}</h2><p class="path-display">${icon("workspace")} ${escapeHtml(this.state.workspace?.path)}</p></div><button class="button button--primary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button></div>
      <section class="workspace-stats"><article><span>${icon("tracks")}</span><div><strong>${this.state.tracks.length}</strong><small>indexierte Tracks</small></div></article><article><span>${icon("scan")}</span><div><strong>${formatDate(this.state.workspace?.lastScannedAt)}</strong><small>zuletzt gescannt</small></div></article><article><span>${icon("shield")}</span><div><strong>Lokal</strong><small>SQLite + Track-Ordner</small></div></article></section>
      <section class="panel workspace-scan"><div class="panel-heading"><div><p class="overline">Bestehende Projekte</p><h3>Legacy-Track-Import</h3><p>Der Scan erkennt bekannte Ordner, Evidence und Hashlisten. Bestehende Dateien werden dabei niemals verändert.</p></div></div>
        ${scan ? `<div class="scan-summary"><span><strong>${scan.discovered}</strong> erkannt</span><span><strong>${scan.indexed}</strong> indexiert</span><span><strong>${scan.unchanged}</strong> unverändert</span></div>${scan.warnings.length ? `<ul class="warning-list">${scan.warnings.map((warning) => `<li>${icon("alert")} ${escapeHtml(warning)}</li>`).join("")}</ul>` : ""}${scan.candidates?.length ? `<div class="candidate-list">${scan.candidates.map((candidate) => `<article><span class="file-icon">${icon("workspace")}</span><div><strong>${escapeHtml(candidate.name)}</strong><small>${escapeHtml(candidate.relativePath)}</small><p>${candidate.missingItems.length ? `Fehlt: ${candidate.missingItems.map(escapeHtml).join(", ")}` : "Bekannte Struktur erkannt"}</p></div><span class="status-chip">${candidate.status === "NOT_VERIFIED" ? "Nicht verifiziert" : candidate.status === "INCOMPLETE" ? "Unvollständig" : "Indexiert"}</span>${candidate.hasManagedDocumentCollision ? `<span class="collision-note">${icon("alert")} Bestehendes Dokument – keine Übernahme ohne Bestätigung und Sicherung</span>` : ""}</article>`).join("")}</div>` : ""}` : `<div class="scan-placeholder">${icon("scan")}<div><strong>Noch kein Scan in dieser Sitzung</strong><span>Legacy-Projekte beginnen als „Nicht verifiziert“. Fehlende historische Angaben werden nie erfunden.</span></div></div>`}
      </section>
      <section class="panel safety-panel"><div>${icon("lock")}<div><h3>Dateisystem-Sicherheit</h3><p>Alle Dateizugriffe laufen über enge native Commands. Pfade werden kanonisiert, Traversal und Symlink-Escapes abgewiesen und Schreibvorgänge atomar ausgeführt.</p></div></div><ul><li>${icon("check")} Kein stilles Überschreiben</li><li>${icon("check")} Relative Track-Pfade</li><li>${icon("check")} Originale bleiben erhalten</li></ul></section>
      <div class="workspace-actions"><button class="button button--secondary" data-action="open-workspace">Anderen Workspace öffnen</button><button class="button button--secondary" data-action="create-workspace">Neuen Workspace anlegen</button></div>
    </div>`;
  }

  private renderSettings(): string {
    const profile = this.state.profile;
    const subscriptions = this.state.globalEvidence.filter((item) => item.role === "subscription_payment");
    const terms = this.state.globalEvidence.filter((item) => item.role === "suno_terms_rights");
    return `<div class="page-content settings-page">
      <div class="page-lead"><div><p class="overline">Workspace-Stammdaten</p><h2>Globale Angaben</h2><p>Diese Werte werden einmal gespeichert und als tatsächlicher Snapshot in jedes Track-Dokument übernommen.</p></div></div>
      <form id="profile-form" class="panel settings-form">
        <div class="settings-section"><div class="settings-section-copy"><span>01</span><div><h3>Artist & Suno</h3><p>Nur produktionsrelevante Profildaten – keine privaten Kontaktdaten.</p></div></div><div class="field-grid two-col">${this.textField("artistName", "Künstlername", "Künstlername", profile.artistName, true)}${this.textField("sunoProfileName", "Suno-Profilname", "Profilname", profile.sunoProfileName, true)}${this.textField("sunoHandle", "Suno-Benutzername", "@handle", profile.sunoHandle, true)}${this.textField("sunoPlan", "Suno-Tarif", "z. B. Premier", profile.sunoPlan, true)}${this.dateField("subscriptionStartDate", "Abo-Startdatum", profile.subscriptionStartDate, true)}</div></div>
        <div class="settings-section"><div class="settings-section-copy"><span>02</span><div><h3>Standards</h3><p>Vorbelegte Werte können pro Track angepasst werden.</p></div></div><div class="field-grid two-col">${this.textField("defaultAiImageService", "Standard-KI-Bilddienst", "z. B. OpenAI", profile.defaultAiImageService)}${this.boolQuestion("defaultCommercialUse", "Kommerzielle Nutzung standardmäßig vorgesehen?", "", profile.defaultCommercialUse)}</div></div>
        <div class="settings-section"><div class="settings-section-copy"><span>03</span><div><h3>Artwork-Transparenz</h3><p>Projektinterne Richtlinie; keine pauschale gesetzliche Kennzeichnungspflicht.</p></div></div><div>${this.radioCards("artworkTransparencyPolicy", profile.artworkTransparencyPolicy, [["always", "Immer sichtbaren KI-Hinweis hinzufügen", "Empfohlener Projektstandard"], ["per_artwork", "Pro Artwork entscheiden", "Entscheidung wird je Track dokumentiert"], ["none", "Kein automatischer sichtbarer Hinweis", "Nur Prozessdokumentation"]])}${this.textField("disclosureText", "Standard-Hinweistext", "AI-assisted", profile.disclosureText, true)}</div></div>
        <div class="form-save settings-save"><span>${icon("shield")} Stammdaten verbleiben in der lokalen Workspace-Datenbank.</span><button class="button button--primary" type="submit">${icon("check")} Einstellungen speichern</button></div>
      </form>
      <section class="panel global-evidence-panel"><div class="panel-heading"><div><p class="overline">Wiederverwendbare Nachweise</p><h3>Suno-Abo-Evidence</h3><p>Registriere jeden Beleg einmal. Bezahlrhythmus und Startdatum bestimmen automatisch den abgedeckten Monat oder das abgedeckte Jahr.</p></div><button class="button button--secondary" data-action="import-global-evidence">${icon("upload")} Abo-Nachweis registrieren</button></div>
        ${subscriptions.length ? `<div class="global-evidence-list">${subscriptions.map((item) => `<article><span class="file-icon">${icon("file")}</span><div><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart)} – ${formatDate(item.coverageEnd)}</small></div><span class="verification is-valid">${icon("check")} Gehasht</span><button class="icon-button danger" data-remove-global-evidence="${item.id}" aria-label="Globalen Nachweis entfernen">${icon("trash")}</button></article>`).join("")}</div>` : `<p class="empty-inline">Noch kein globaler Abo-Nachweis registriert.</p>`}
      </section>
      <section class="panel global-evidence-panel"><div class="panel-heading"><div><p class="overline">Globale Datei für alle Projekte</p><h3>Archivierte Suno-Nutzungsbedingungen</h3><p>Wähle genau eine lokale PDF aus. Sie wird gehasht und in jedes neue sowie jedes noch bearbeitbare Projekt als portable Evidence-Kopie hinterlegt. Finalisierte Snapshots bleiben unverändert.</p><p>Gefordert: PDF. SunoDM trifft keine Rechte- oder Gültigkeitsaussage.</p></div><button class="button button--secondary" data-action="import-global-terms">${icon("upload")} PDF auswählen</button></div>
        ${terms.length ? `<div class="global-evidence-list">${terms.map((item) => `<article><span class="file-icon">${icon("file")}</span><div><strong>${escapeHtml(item.fileName)}</strong><small>Lokale PDF · automatisch gehasht</small></div><span class="verification is-valid">${icon("check")} Gehasht</span><button class="icon-button danger" data-remove-global-evidence="${item.id}" aria-label="Globale Nutzungsbedingungen entfernen">${icon("trash")}</button></article>`).join("")}</div>` : `<p class="empty-inline">Noch keine globale PDF mit Suno-Nutzungsbedingungen registriert.</p>`}
      </section>
    </div>`;
  }

  private inlineEvidenceActions(track: TrackDetail, actions: Array<[EvidenceRole, string]>): string {
    const locked = isTrackContentLocked(track.status);
    return `<div class="inline-evidence">${actions.map(([role, label]) => {
      const present = [...track.evidence].reverse().find((item) => item.role === role);
      const types = evidenceRoleFileTypes(role);
      return `<div class="evidence-control ${present ? "is-present" : ""}">
        <button type="button" class="evidence-button ${present?.verified ? "is-present" : ""}" ${present ? `data-preview-evidence="${escapeHtml(present.id)}"` : `data-import-role="${role}" ${locked ? "disabled" : ""}`}>${present?.verified ? icon("check") : icon("upload")}<span><strong>${escapeHtml(label)}</strong><small>${present ? present.verified ? "Vorhanden – klicken für Vorschau" : "Vorhanden, aber nicht verifiziert – klicken für Vorschau" : evidenceRoleLabel(role)}</small><small class="evidence-types">Gefordert: ${escapeHtml(types)}</small></span></button>
        ${present ? `<button type="button" class="evidence-reupload" data-import-role="${role}" data-replace-evidence="${escapeHtml(present.id)}" aria-label="${escapeHtml(label)} ersetzen" title="Datei ersetzen" ${locked ? "disabled" : ""}>${icon("upload")}</button>` : ""}
      </div>`;
    }).join("")}</div>`;
  }

  private filenameConfirmation(
    track: TrackDetail,
    role: EvidenceRole,
    field: "releaseFilenameDifferenceConfirmed" | "sunoExportFilenameDifferenceConfirmed",
    confirmed: boolean | null,
    label: string
  ): string {
    const item = [...track.evidence].reverse().find((entry) => entry.role === role && entry.verified);
    const actual = item?.metadata?.originalFileName?.trim() ?? "";
    if (!actual) return `<div class="neutral-message">${icon("info")}<div><strong>${escapeHtml(label)}: tatsächlicher Quelldateiname nicht erfasst</strong><span>Importiere oder ersetze die Datei, damit der Name als Evidence-derived metadata gespeichert wird.</span></div></div>`;
    if (filenameMatchesDocumentedTitle(this.state.trackDraft?.title ?? track.fields.title, actual)) {
      return `<div class="neutral-message">${icon("check")}<div><strong>${escapeHtml(label)} passt zum dokumentierten Titel</strong><span>${escapeHtml(actual)}</span></div></div>`;
    }
    return `<div class="conditional-panel"><div class="conditional-line"></div><p><strong>Dateinamenabweichung erkannt</strong><br>Dokumentierter Titel: ${escapeHtml(this.state.trackDraft?.title ?? track.fields.title)}<br>Tatsächlicher Dateiname: ${escapeHtml(actual)}</p>${this.boolQuestion(field, "Ist diese Abweichung beabsichtigt?", "Bestätige ausdrücklich oder korrigiere den dokumentierten Titel. Der Titel wird niemals aus dem Dateinamen abgeleitet.", confirmed)}</div>`;
  }

  private textField(name: string, label: string, placeholder: string, value: string, required = false, type = "text"): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><input type="${type}" name="${name}" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${required ? "required" : ""}></label>`;
  }

  private suggestedTextField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    suggestions: readonly string[],
    required = false
  ): string {
    const listId = `${name}-suggestions`;
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><input type="text" name="${escapeHtml(name)}" list="${escapeHtml(listId)}" autocomplete="off" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${required ? "required" : ""}><datalist id="${escapeHtml(listId)}">${suggestions.map((suggestion) => `<option value="${escapeHtml(suggestion)}"></option>`).join("")}</datalist><small class="field-help">Vorschlag wählen oder einen beliebigen eigenen Wert eingeben.</small></label>`;
  }

  private dateField(name: string, label: string, value: string, required = false): string {
    return this.textField(name, label, "", value, required, "date");
  }

  private automatedDateField(name: string, label: string, value: string, origin: FactOrigin, required = true, fallbackCaption?: string): string {
    const automated = isAutomaticDateReadonly(origin);
    const requirement = !automated && required ? "required" : "";
    const caption = automated ? factOriginLabel(origin) : fallbackCaption ?? factOriginLabel(origin);
    return `<label class="field ${automated ? "field--automated" : ""}"><span class="field-label">${escapeHtml(label)}${!automated && required ? " *" : ""}</span><span class="field-input-wrap"><input type="date" name="${escapeHtml(name)}" value="${escapeHtml(value)}" ${automated ? "readonly aria-readonly=\"true\"" : requirement}>${automated ? `<i title="Evidence-derived metadata">${icon("check")}</i>` : ""}</span><small class="field-caption">${escapeHtml(caption)}</small></label>`;
  }

  private hasManualSunoDateOverriddenByMetadata(previous: TrackDetail, imported: TrackDetail): boolean {
    const dates: Array<[string, string, FactOrigin, FactOrigin]> = [
      [previous.fields.sunoFinalGenerationDate, imported.fields.sunoFinalGenerationDate, previous.automation.finalGenerationOrigin, imported.automation.finalGenerationOrigin],
      [previous.fields.productionEndDate, imported.fields.productionEndDate, previous.automation.productionEndOrigin, imported.automation.productionEndOrigin],
      [previous.fields.sunoDownloadExportDate, imported.fields.sunoDownloadExportDate, previous.automation.downloadExportOrigin, imported.automation.downloadExportOrigin],
      [previous.fields.finalExportDate, imported.fields.finalExportDate, previous.automation.finalExportOrigin, imported.automation.finalExportOrigin]
    ];
    return dates.some(([before, after, previousOrigin, importedOrigin]) =>
      Boolean(before) && before !== after
        && previousOrigin !== "evidence_derived_metadata"
        && importedOrigin === "evidence_derived_metadata"
    );
  }

  private renderAutomaticSunoMetadata(track: TrackDetail): string {
    const timestamp = track.automation.sunoCreatedTimestamp;
    const sunoId = track.automation.sunoId;
    if (!track.automation.sunoMetadataDetected || !timestamp || !sunoId) return "";

    return `<section class="policy-card"><div>${icon("check")}<p class="overline">Automatisch aus Suno-WAV erkannt</p><h4>Aus Dateimetadaten</h4><dl><div><dt>Suno Studio</dt><dd>Ja</dd></div><div><dt>Download/Export</dt><dd>${escapeHtml(formatDate(track.fields.sunoDownloadExportDate))}</dd></div><div><dt>Suno ID</dt><dd><code>${escapeHtml(sunoId)}</code></dd></div></dl><details><summary>Technische Details</summary><p>Embedded Suno export timestamp: <code>${escapeHtml(timestamp)}</code></p></details></div></section>`;
  }

  private textArea(name: string, label: string, placeholder: string, value: string, required = false): string {
    return `<label class="field field--wide"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><textarea name="${name}" placeholder="${escapeHtml(placeholder)}" ${required ? "required" : ""}>${escapeHtml(value)}</textarea></label>`;
  }

  private multiChoiceField(name: string, label: string, value: string, options: readonly GuidedChoice[], required = false): string {
    const selected = parseMultiChoiceValue(canonicalGuidedChoiceList(value, options));
    const known = new Set(options.map(([option]) => option));
    const choices: GuidedChoice[] = [
      ...options,
      ...selected.filter((item) => !known.has(item)).map((item) => [item, `Bisherige Auswahl: ${item} (bitte prüfen)`] as const)
    ];
    return `<fieldset class="multi-choice-field field--wide" data-multi-choice-group ${required ? `data-multi-choice-required aria-required="true"` : ""}><legend>${escapeHtml(label)}${required ? " *" : ""}</legend><div>${choices.map(([option, optionLabel]) => `<label><input type="checkbox" name="${name}" value="${escapeHtml(option)}" data-multi-choice ${selected.includes(option) ? "checked" : ""}><span>${escapeHtml(optionLabel)}</span></label>`).join("")}</div>${required ? `<p class="field-help">Wähle mindestens einen tatsächlich ausgeführten Schritt aus.</p>` : ""}</fieldset>`;
  }

  private multiChoiceArrayField(
    name: string,
    label: string,
    value: readonly string[],
    options: readonly GuidedChoice[],
    required = false
  ): string {
    const selected = canonicalGuidedChoiceArray(value, options);
    const known = new Set(options.map(([option]) => option));
    const choices: GuidedChoice[] = [
      ...options,
      ...selected.filter((item) => !known.has(item)).map((item) => [item, `Bisheriger Freitext: ${item}`] as const)
    ];
    return `<fieldset class="multi-choice-field field--wide" data-multi-choice-group data-choice-array ${required ? `data-multi-choice-required aria-required="true"` : ""}><legend>${escapeHtml(label)}${required ? " *" : ""}</legend><div>${choices.map(([option, optionLabel]) => `<label><input type="checkbox" name="${escapeHtml(name)}" value="${escapeHtml(option)}" data-multi-choice ${selected.includes(option) ? "checked" : ""}><span>${escapeHtml(optionLabel)}</span></label>`).join("")}</div>${required ? `<p class="field-help">Wähle mindestens einen tatsächlich ausgeführten Schritt aus.</p>` : `<p class="field-help">Mehrere Angaben können gleichzeitig ausgewählt und durch Freitext ergänzt werden.</p>`}</fieldset>`;
  }

  private selectField(name: string, label: string, value: string, options: Array<[string, string]>, required = false): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><select name="${name}" ${required ? "required" : ""}>${options.map(([id, text]) => `<option value="${escapeHtml(id)}" ${value === id ? "selected" : ""}>${escapeHtml(text)}</option>`).join("")}</select></label>`;
  }

  private guidedSingleChoiceField(name: string, label: string, value: string, choices: readonly GuidedChoice[], required = false): string {
    const selected = canonicalGuidedChoiceValue(value, choices);
    const known = choices.some(([option]) => option === selected);
    const options: SingleChoiceOption[] = choices.map(([option, optionLabel]) => [option, optionLabel]);
    if (selected && !known) options.unshift([selected, `Bisheriger Wert: ${selected} (bitte prüfen)`]);
    return singleChoiceFieldMarkup(name, label, selected, options, required);
  }

  private boolQuestion(name: string, label: string, help: string, value: boolean | null): string {
    return `<fieldset class="boolean-field"><legend>${escapeHtml(label)}</legend>${help ? `<p>${escapeHtml(help)}</p>` : ""}<div><label><input type="radio" name="${name}" value="true" ${value === true ? "checked" : ""}><span>${icon("check")} Ja</span></label><label><input type="radio" name="${name}" value="false" ${value === false ? "checked" : ""}><span>${icon("close")} Nein</span></label></div></fieldset>`;
  }

  private radioCards(name: string, value: string, options: Array<[string, string, string]>): string {
    return `<div class="radio-cards">${options.map(([id, label, help]) => `<label><input type="radio" name="${name}" value="${id}" ${id === value ? "checked" : ""}><span><i></i><strong>${escapeHtml(label)}</strong><small>${escapeHtml(help)}</small></span></label>`).join("")}</div>`;
  }

  private policyLabel(policy: GlobalProfile["artworkTransparencyPolicy"]): string {
    return policy === "always" ? "Immer sichtbaren KI-Hinweis hinzufügen" : policy === "per_artwork" ? "Pro Artwork entscheiden" : "Kein automatischer sichtbarer Hinweis";
  }

  private emptyState(iconName: "tracks" | "current" | "file", title: string, copy: string, action?: string, actionLabel?: string): string {
    return `<div class="empty-state">${icon(iconName)}<h3>${escapeHtml(title)}</h3><p>${escapeHtml(copy)}</p>${action ? `<button class="button button--secondary" data-action="${action}">${escapeHtml(actionLabel)}</button>` : ""}</div>`;
  }

  private async handleClick(event: Event): Promise<void> {
    const target = event.target as HTMLElement;
    const button = target.closest<HTMLElement>("button, [data-action], [data-view], [data-track-open], [data-step-open], [data-track-tab]");
    if (!button) return;
    if (shouldIgnoreModalBackdropClick(
      button.matches('.modal-backdrop[data-action="close-modal"]'),
      target === button
    )) return;

    if (button.dataset.action === "create-album") {
      event.preventDefault();
      event.stopPropagation();
      const title = window.prompt("Name des neuen Albumordners:")?.trim();
      if (!title) return;
      const albums = await this.withBusy(
        "Albumordner wird angelegt …",
        () => this.api.createAlbum(title)
      );
      if (albums) {
        this.state.albums = albums;
        this.showToast("success", "Albumordner angelegt", `${title} wurde erstellt. Tracks können diesem Album jetzt zugeordnet werden.`);
        this.render();
      }
      return;
    }

    const albumTitle = button.dataset.renameAlbum;
    if (albumTitle) {
      event.preventDefault();
      event.stopPropagation();
      const newTitle = window.prompt("Neuer Name des Albumordners:", albumTitle)?.trim();
      if (!newTitle || newTitle === albumTitle) return;
      const result = await this.withBusy(
        "Albumordner wird umbenannt …",
        async () => {
          const tracks = await this.api.renameAlbum(albumTitle, newTitle);
          const albums = await this.api.listAlbums();
          return { tracks, albums };
        }
      );
      if (result) {
        this.state.tracks = result.tracks;
        this.state.albums = result.albums;
        const currentSummary = this.state.track
          ? result.tracks.find((track) => track.id === this.state.track!.id)
          : undefined;
        if (this.state.track && currentSummary) {
          this.state.track.library = structuredClone(currentSummary.library);
          this.state.track.relativePath = currentSummary.relativePath;
        }
        this.showToast("success", "Albumordner umbenannt", `${albumTitle} wurde in ${newTitle} umbenannt.`);
        this.render();
      }
      return;
    }

    const view = button.dataset.view as MainView | undefined;
    if (view) {
      if (!(await this.flushDraft())) return;
      this.state.view = view;
      this.state.sidebarOpen = false;
      this.state.activeStep = null;
      this.render();
      return;
    }
    const trackId = button.dataset.trackOpen;
    if (trackId) {
      if (!(await this.flushDraft())) return;
      const track = await this.withBusy("Track wird geladen …", () => this.api.loadTrack(trackId));
      if (track) {
        this.applyTrack(track);
        this.state.view = "current";
        this.state.trackTab = "overview";
        this.state.activeStep = null;
      }
      return;
    }
    const stepId = button.dataset.stepOpen as StepId | undefined;
    if (stepId) {
      if (!(await this.flushDraft())) return;
      this.state.activeStep = stepId;
      this.state.view = "current";
      this.render();
      return;
    }
    const tab = button.dataset.trackTab as TrackTab | undefined;
    if (tab) {
      if (!(await this.flushDraft())) return;
      this.state.trackTab = tab;
      this.state.activeStep = null;
      this.render();
      return;
    }
    const filter = button.dataset.trackFilter as AppState["trackFilter"] | undefined;
    if (filter) {
      this.state.trackFilter = filter;
      this.render();
      return;
    }
    const importRole = button.dataset.importRole as EvidenceRole | undefined;
    if (importRole) {
      await this.importEvidence(importRole, button.dataset.replaceEvidence);
      return;
    }
    const previewEvidenceId = button.dataset.previewEvidence;
    if (previewEvidenceId) {
      await this.previewEvidence(previewEvidenceId);
      return;
    }
    if (button.dataset.verifyEvidence) {
      await this.trackMutation("Evidence wird geprüft …", () => this.api.verifyEvidence(this.requireTrack().id, button.dataset.verifyEvidence), "Evidence verifiziert", true);
      return;
    }
    if (button.dataset.removeEvidence) {
      const selected = this.requireTrack().evidence.find((item) => item.id === button.dataset.removeEvidence);
      const prompt = selected?.provenance === "indexed_legacy"
        ? "Historisch indexierte Evidence entfernen? Die Datei wird nachvollziehbar unter .archive/removals gesichert und nicht gelöscht."
        : "Importierte Evidence-Kopie aus dem Track entfernen? Die Originaldatei am Quellort bleibt erhalten.";
      if (window.confirm(prompt)) {
        await this.trackMutation("Evidence wird entfernt …", () => this.api.removeEvidence(this.requireTrack().id, button.dataset.removeEvidence!), "Evidence entfernt");
      }
      return;
    }
    if (button.dataset.removeGlobalEvidence) {
      if (window.confirm("Global registrierten Nachweis entfernen? Bereits in Tracks kopierte Evidence bleibt bestehen.")) {
        await this.withBusy("Nachweis wird entfernt …", () => this.api.removeGlobalEvidence(button.dataset.removeGlobalEvidence!));
        this.state.globalEvidence = await this.api.listGlobalEvidence();
        this.showToast("success", "Nachweis entfernt", "Bereits zugeordnete Track-Kopien wurden nicht verändert.");
      }
      return;
    }
    if (button.dataset.attachGlobal) {
      const global = this.state.globalEvidence.find((item) => item.id === button.dataset.attachGlobal);
      const terms = global?.role === "suno_terms_rights";
      await this.trackMutation(
        terms ? "Nutzungsbedingungen werden in das Projekt kopiert …" : "Abo-Nachweis wird in den Track kopiert …",
        () => this.api.attachGlobalEvidence(this.requireTrack().id, button.dataset.attachGlobal!),
        terms ? "Nutzungsbedingungen im Projekt hinterlegt" : "Abo-Nachweis zugeordnet"
      );
      return;
    }
    if (button.dataset.resolveDeviation) {
      await this.trackMutation("Abweichung wird gelöst …", () => this.api.resolveDeviation(this.requireTrack().id, button.dataset.resolveDeviation!), "Abweichung gelöst");
      return;
    }
    if (button.dataset.markNa) {
      if (!(await this.flushDraft())) return;
      const reason = window.prompt("Warum ist dieser Schritt für den Track nicht anwendbar? Eine konkrete Begründung ist erforderlich.");
      if (!reason?.trim()) return;
      await this.trackMutation("N/A-Begründung wird gespeichert …", () => this.api.setStepStatus(this.requireTrack().id, button.dataset.markNa as StepId, "N_A", reason.trim()), "N/A dokumentiert");
      return;
    }
    if (button.dataset.resetNa) {
      await this.trackMutation("Schrittstatus wird zurückgesetzt …", () => this.api.setStepStatus(this.requireTrack().id, button.dataset.resetNa as StepId, "NOT_RUN"), "Schrittstatus zurückgesetzt");
      return;
    }
    if (button.dataset.removeDeviation) {
      await this.trackMutation("Abweichung wird entfernt …", () => this.api.removeDeviation(this.requireTrack().id, button.dataset.removeDeviation!), "Abweichung entfernt");
      return;
    }

    switch (button.dataset.action) {
      case "toggle-theme": this.toggleTheme(); break;
      case "open-workspace": await this.chooseWorkspace("open"); break;
      case "create-workspace": await this.chooseWorkspace("create"); break;
      case "new-track": {
        if (!(await this.flushDraft())) break;
        const missing = missingProfileFields(this.state.profile);
        if (missing.length) {
          this.state.view = "settings";
          this.showToast("info", "Zuerst Stammdaten vervollständigen", `Für einen unveränderlichen Track-Snapshot fehlen: ${missing.join(", ")}.`);
          this.render();
        } else {
          this.state.showTrackLibrary = false;
          this.state.showSubscriptionEvidence = false;
          this.state.evidencePreview = null;
          this.state.folderImport = null;
          this.state.showNewTrack = true;
          this.render();
        }
        break;
      }
      case "scan-folder-import": {
        const proposal = await this.withBusy("Ordner wird analysiert …", () => this.api.scanImportFolder());
        if (proposal) {
          this.state.folderImport = proposal;
          this.showToast("info", "Ordner analysiert", `${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} erkannt. Nur eindeutige Dateien werden übernommen.`);
          this.render();
        }
        break;
      }
      case "edit-track-library":
        if (this.rejectLockedContentMutation()) break;
        if (!(await this.flushDraft())) break;
        this.state.showNewTrack = false;
        this.state.showSubscriptionEvidence = false;
        this.state.evidencePreview = null;
        this.state.showTrackLibrary = true;
        this.render();
        break;
      case "close-modal": this.state.showNewTrack = false; this.state.folderImport = null; this.state.showTrackLibrary = false; this.state.showSubscriptionEvidence = false; this.state.evidencePreview = null; this.state.showCertificatePopup = false; this.render(); break;
      case "show-certificate-popup":
        if (this.requireTrack().certificate.valid && this.requireTrack().certificate.certificateId) {
          this.state.showCertificatePopup = true;
          this.render();
        }
        break;
      case "open-certificate-tab":
        this.state.showCertificatePopup = false;
        this.state.view = "current";
        this.state.activeStep = null;
        this.state.trackTab = "certificate";
        this.render();
        break;
      case "open-sidebar": this.state.sidebarOpen = true; this.render(); break;
      case "close-sidebar": this.state.sidebarOpen = false; this.render(); break;
      case "dismiss-toast": this.state.toast = null; this.render(); break;
      case "go-tracks": if (await this.flushDraft()) { this.state.view = "tracks"; this.render(); } break;
      case "back-overview": if (await this.flushDraft()) { this.state.activeStep = null; this.state.trackTab = "overview"; this.render(); } break;
      case "scan-workspace": await this.scanWorkspace(); break;
      case "adopt-legacy-profile":
        if (window.confirm("Treffen die aktuellen Workspace-Stammdaten auf diesen historischen Track zu? Sie werden als Track-Snapshot übernommen.")) await this.trackMutation("Stammdaten werden als Legacy-Snapshot übernommen …", () => this.api.adoptLegacyProfile(this.requireTrack().id), "Legacy-Snapshot übernommen");
        break;
      case "import-evidence": await this.chooseEvidenceRole(); break;
      case "import-global-evidence": this.state.showNewTrack = false; this.state.showTrackLibrary = false; this.state.evidencePreview = null; this.state.showSubscriptionEvidence = true; this.render(); break;
      case "import-global-terms": await this.importGlobalTermsEvidence(); break;
      case "add-deviation": await this.addDeviation(); break;
      case "generate-documents": await this.generateDocumentsSafely(); break;
      case "generate-disclosure": await this.runAction("KI-Hinweis wird lokal erzeugt …", () => this.api.generateArtworkDisclosure(this.requireTrack().id, this.state.trackDraft?.disclosureText)); break;
      case "calculate-hashes": await this.runProgressAction("hashes", "SHA-256 wird berechnet …", (onProgress) => this.api.calculateHashes(this.requireTrack().id, onProgress)); break;
      case "verify-hashes": await this.runProgressAction("verification", "Prüfsummen werden verifiziert …", (onProgress) => this.api.verifyHashes(this.requireTrack().id, onProgress), true); break;
      case "finalize-track": await this.finalizeTrack(); break;
      case "invalidate-certificate":
        if (!canCreateTrackRevision(this.requireTrack().status)) {
          this.rejectLockedContentMutation();
          break;
        }
        if (window.confirm("Zertifikat als ungültig markieren? Der finalisierte Snapshot wird nicht still überschrieben.")) await this.runAction("Zertifikat wird invalidiert …", () => this.api.invalidateCertificate(this.requireTrack().id), true);
        break;
      case "create-revision":
        if (!canCreateTrackRevision(this.requireTrack().status)) {
          this.rejectLockedContentMutation();
          break;
        }
        if (window.confirm("Neue Revision anlegen? Der bisherige Certificate-/Manifest-Snapshot wird zuerst unter .archive/revisions gesichert.")) await this.runAction("Neue Revision wird angelegt …", () => this.api.createRevision(this.requireTrack().id), true);
        break;
      case "re-evaluate-track":
        if (this.requireTrack().status === "SUPERSEDED") {
          this.rejectLockedContentMutation();
          break;
        }
        if (window.confirm("Track mit dem aktuellen Workflow neu bewerten? Ein finalisierter Snapshot wird zuerst unverändert als Revision archiviert; Dokumente, Prüfsummen und Zertifikat müssen danach neu erzeugt werden.")) await this.runAction("Workflow wird aktualisiert und neu bewertet …", () => this.api.reEvaluateTrack(this.requireTrack().id), this.requireTrack().status === "FINALIZED");
        break;
      case "terms-unavailable":
        {
          const next = this.requireTrack().fields.sunoTermsEvidenceNotAvailable !== true;
          const prompt = next
            ? "Ausdrücklich dokumentieren, dass kein lokaler Terms-Nachweis verfügbar ist? Dies ist keine Aussage über Rechte oder Gültigkeit."
            : "Den dokumentierten Status 'Terms evidence not available' zurücknehmen?";
          if (!window.confirm(prompt)) break;
          await this.trackMutation(
            "Status wird dokumentiert …",
            () => this.api.updateTrack(this.requireTrack().id, { sunoTermsEvidenceNotAvailable: next }),
            next ? "Terms-Status dokumentiert" : "Terms-Status zurückgenommen"
          );
        }
        break;
    }
  }

  private async handleSubmit(event: SubmitEvent): Promise<void> {
    const form = event.target as HTMLFormElement;
    event.preventDefault();
    if (form.id === "new-track-form") {
      const profileMissing = missingProfileFields(this.state.profile);
      if (profileMissing.length) {
        this.state.showNewTrack = false;
        this.state.view = "settings";
        this.showToast("error", "Track nicht angelegt", `Vervollständige zuerst: ${profileMissing.join(", ")}.`);
        this.render();
        return;
      }
      const proposal = this.state.folderImport;
      if (proposal) {
        const data = new FormData(form);
        const singleTrackLibrary = proposal.kind === "single" ? this.readTrackLibraryAssignment(form) : null;
        if (proposal.kind === "single" && !singleTrackLibrary) return;
        const tracks = await this.withBusy("Ordner wird in normale Track-Strukturen übernommen …", () => this.api.executeFolderImport({
          sourcePath: proposal.sourcePath,
          expectedKind: proposal.kind,
          singleTrackTitle: proposal.kind === "single" ? String(data.get("title") ?? "") : undefined,
          singleTrackLibrary: singleTrackLibrary ?? undefined,
          productionStartDate: proposal.kind === "single" ? String(data.get("productionStartDate") ?? "") : "",
          commercialUseIntended: data.get("commercialUseIntended") === "on"
        }));
        if (tracks?.length) {
          this.applyTrack(tracks[0]);
          this.state.tracks = await this.api.listTracks();
          this.state.albums = await this.api.listAlbums();
          this.state.showNewTrack = false;
          this.state.folderImport = null;
          this.state.view = "current";
          this.state.activeStep = "track";
          this.showToast("success", "Ordner importiert", `${tracks.length} ${tracks.length === 1 ? "Track wurde" : "Tracks wurden"} als unvollständige normale SunoDM-Struktur angelegt.`);
          this.render();
        }
        return;
      }
      const data = new FormData(form);
      const library = this.readTrackLibraryAssignment(form);
      if (!library) return;
      const track = await this.withBusy("Track-Struktur wird angelegt …", () => this.api.createTrack({
        title: String(data.get("title") ?? ""),
        productionStartDate: String(data.get("productionStartDate") ?? ""),
        commercialUseIntended: data.get("commercialUseIntended") === "on",
        library
      }));
      if (track) {
        this.applyTrack(track);
        this.state.showNewTrack = false;
        this.state.view = "current";
        this.state.activeStep = "track";
        this.showToast("success", "Track angelegt", "Die portable Track-Struktur wurde lokal erstellt.");
        this.render();
      }
      return;
    }
    if (form.id === "track-library-form") {
      if (this.rejectLockedContentMutation()) return;
      const library = this.readTrackLibraryAssignment(form);
      if (!library) return;
      const updated = await this.withBusy(
        "Track-Ordner wird verschoben …",
        () => this.api.updateTrackLibrary(this.requireTrack().id, library)
      );
      if (updated) {
        this.applyTrack(updated);
        this.state.showTrackLibrary = false;
        this.showToast("success", "Ordnerstruktur aktualisiert", `Der Track liegt jetzt unter ${updated.relativePath}.`);
        this.render();
      }
      return;
    }
    if (form.id === "subscription-evidence-form") {
      const data = new FormData(form);
      const coverageStart = String(data.get("coverageStart") ?? "");
      const rawBillingCycle = String(data.get("billingCycle") ?? "");
      if (rawBillingCycle !== "monthly" && rawBillingCycle !== "annual") {
        this.showToast("error", "Bezahlrhythmus fehlt", "Wähle monatliche oder jährliche Zahlung aus.");
        this.render();
        return;
      }
      const billingCycle: SubscriptionBillingCycle = rawBillingCycle;
      const coverageEnd = subscriptionCoverageEnd(coverageStart, billingCycle);
      if (!coverageEnd) {
        this.showToast("error", "Startdatum ungültig", "Gib den Beginn des auf der Rechnung abgedeckten Zeitraums an.");
        this.render();
        return;
      }
      const imported = await this.withBusy("Abo-Nachweis wird registriert …", async () => {
        const item = await this.api.importGlobalEvidence("subscription_payment", coverageStart, billingCycle);
        if (!item) return null;
        return { item, globalEvidence: await this.api.listGlobalEvidence() };
      });
      if (imported) {
        this.state.globalEvidence = imported.globalEvidence;
        this.state.showSubscriptionEvidence = false;
        this.showToast("success", "Abo-Nachweis registriert", `Abgedeckter Zeitraum: ${formatDate(coverageStart)} – ${formatDate(imported.item.coverageEnd ?? coverageEnd)}.`);
        this.render();
      }
      return;
    }
    if (form.id === "profile-form") {
      const data = new FormData(form);
      const profile: GlobalProfile = {
        artistName: String(data.get("artistName") ?? ""), sunoProfileName: String(data.get("sunoProfileName") ?? ""),
        sunoHandle: String(data.get("sunoHandle") ?? ""), sunoPlan: String(data.get("sunoPlan") ?? ""),
        subscriptionStartDate: String(data.get("subscriptionStartDate") ?? ""), defaultCommercialUse: data.get("defaultCommercialUse") === "true",
        defaultAiImageService: String(data.get("defaultAiImageService") ?? ""), artworkTransparencyPolicy: String(data.get("artworkTransparencyPolicy")) as GlobalProfile["artworkTransparencyPolicy"],
        disclosureText: String(data.get("disclosureText") ?? "AI-assisted")
      };
      const saved = await this.withBusy("Stammdaten werden gespeichert …", () => this.api.updateProfile(profile));
      if (saved) {
        this.state.profile = saved;
        await this.refreshTracks();
        this.showToast("success", "Einstellungen gespeichert", "Offene Tracks wurden aktualisiert; finalisierte Track-Snapshots bleiben unverändert.");
      }
      return;
    }
    if (form.id === "track-step-form") {
      if (this.rejectLockedContentMutation()) return;
      const emptyRequiredChoice = [...form.querySelectorAll<HTMLElement>("[data-multi-choice-required]")]
        .find((group) => !group.querySelector("input[data-multi-choice]:checked"));
      if (emptyRequiredChoice) {
        this.showToast("error", "Auswahl fehlt", "Wähle mindestens einen tatsächlich ausgeführten Schritt aus.");
        return;
      }
      await this.saveTrackDraft();
    }
  }

  private handleChange(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    const libraryForm = input.closest<HTMLFormElement>("#new-track-form, #track-library-form");
    if (libraryForm && input.name === "librarySection") {
      this.syncTrackLibraryFields(libraryForm);
      return;
    }
    const subscriptionForm = input.closest<HTMLFormElement>("#subscription-evidence-form");
    if (subscriptionForm) {
      this.updateSubscriptionEvidencePreview(subscriptionForm);
      return;
    }
    if (!input.closest("#track-step-form") || !this.state.trackDraft) return;
    if (this.state.track && isTrackContentLocked(this.state.track.status)) {
      this.state.trackDraft = structuredClone(this.state.track.fields);
      this.draftDirty = false;
      return;
    }
    if (input.matches("[data-multi-choice]")) {
      const group = input.closest<HTMLElement>("[data-multi-choice-group]");
      const checked = [...(group?.querySelectorAll<HTMLInputElement>("input[data-multi-choice]:checked") ?? [])]
        .map((item) => item.value);
      const storesArray = group?.hasAttribute("data-choice-array") ?? false;
      (this.state.trackDraft as unknown as Record<string, unknown>)[input.name] = storesArray
        ? checked
        : serializeMultiChoiceValue(checked);
      this.draftDirty = true;
      if (storesArray) this.render();
      return;
    }
    const key = input.name as keyof TrackFields;
    if (!key) return;
    let value: string | boolean | null = input.value;
    if (input instanceof HTMLInputElement && input.type === "radio" && (input.value === "true" || input.value === "false")) value = input.value === "true";
    (this.state.trackDraft as unknown as Record<string, unknown>)[key] = value;
    this.draftDirty = true;
    this.render();
  }

  private handleInput(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLTextAreaElement;
    if (input.name === "albumTitle" && input.closest("#new-track-form, #track-library-form")) {
      input.setCustomValidity("");
      return;
    }
    const subscriptionForm = input.closest<HTMLFormElement>("#subscription-evidence-form");
    if (subscriptionForm) {
      this.updateSubscriptionEvidencePreview(subscriptionForm);
      return;
    }
    if (input.matches("[data-track-search]")) {
      this.state.query = input.value;
      this.render();
      const next = this.root.querySelector<HTMLInputElement>("[data-track-search]");
      next?.focus();
      next?.setSelectionRange(next.value.length, next.value.length);
      return;
    }
    if (input.closest("#track-step-form") && this.state.trackDraft && input.name) {
      if (this.state.track && isTrackContentLocked(this.state.track.status)) {
        this.state.trackDraft = structuredClone(this.state.track.fields);
        this.draftDirty = false;
        return;
      }
      (this.state.trackDraft as unknown as Record<string, unknown>)[input.name] = input.value;
      this.draftDirty = true;
    }
  }

  private syncTrackLibraryFields(form: HTMLFormElement): void {
    const albumSelected = form.querySelector<HTMLInputElement>('input[name="librarySection"]:checked')?.value === "album";
    const field = form.querySelector<HTMLElement>("[data-library-album-field]");
    const input = form.elements.namedItem("albumTitle") as HTMLInputElement | null;
    if (!field || !input) return;
    field.hidden = !albumSelected;
    input.disabled = !albumSelected;
    input.required = albumSelected;
    input.setCustomValidity("");
    if (albumSelected) input.focus();
  }

  private readTrackLibraryAssignment(form: HTMLFormElement): TrackLibraryAssignment | null {
    const section = form.querySelector<HTMLInputElement>('input[name="librarySection"]:checked')?.value ?? "";
    const albumInput = form.elements.namedItem("albumTitle") as HTMLInputElement | null;
    const library = trackLibraryAssignment(section, albumInput?.value ?? "");
    if (library) return library;
    if (albumInput) {
      const field = form.querySelector<HTMLElement>("[data-library-album-field]");
      field?.removeAttribute("hidden");
      albumInput.disabled = false;
      albumInput.required = true;
      albumInput.setCustomValidity(albumInput.value.trim()
        ? "Der Albumtitel darf höchstens 200 Zeichen und keine Pfadtrenner, Steuerzeichen oder reservierten Ordnernamen enthalten."
        : "Gib für einen Album-Track einen Albumtitel an.");
      albumInput.reportValidity();
      albumInput.focus();
    }
    return null;
  }

  private updateSubscriptionEvidencePreview(form: HTMLFormElement): void {
    const coverageStart = form.elements.namedItem("coverageStart") as HTMLInputElement | null;
    const selectedCycle = form.querySelector<HTMLInputElement>('input[name="billingCycle"]:checked');
    const coverageEnd = form.elements.namedItem("coverageEnd") as HTMLInputElement | null;
    if (!coverageStart || !selectedCycle || !coverageEnd) return;
    const cycle = selectedCycle.value as SubscriptionBillingCycle;
    coverageEnd.value = subscriptionCoverageEnd(coverageStart.value, cycle) ?? "";
  }

  private requireTrack(): TrackDetail {
    if (!this.state.track) throw new Error("Wähle zuerst einen Track aus.");
    return this.state.track;
  }

  private rejectLockedContentMutation(): boolean {
    const track = this.state.track;
    if (!track || !isTrackContentLocked(track.status)) return false;
    this.state.trackDraft = structuredClone(track.fields);
    this.draftDirty = false;
    if (track.status === "FINALIZED") {
      this.showToast("info", "Neue Revision erforderlich", "Der finalisierte Snapshot wurde nicht verändert. Lege zuerst eine neue Revision an.");
    } else {
      this.showToast("info", "Ersetzter Snapshot – nur lesbar", "Dieser historische Snapshot bleibt unverändert. Öffne die aktuelle Revision, um Inhalte zu bearbeiten.");
    }
    return true;
  }

  private async chooseWorkspace(kind: "open" | "create"): Promise<void> {
    if (!(await this.flushDraft())) return;
    const workspace = await this.withBusy(kind === "open" ? "Ordnerdialog wird geöffnet …" : "Workspace wird angelegt …", () => kind === "open" ? this.api.openWorkspace() : this.api.createWorkspace());
    if (workspace) await this.enterWorkspace(workspace);
  }

  private async scanWorkspace(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const scan = await this.withBusy("Workspace wird sicher gescannt …", () => this.api.scanWorkspace());
    if (!scan) return;
    this.state.scanResult = scan;
    await this.refreshTracks();
    this.state.view = "workspace";
    this.showToast("success", "Scan abgeschlossen", `${scan.discovered} Track-Ordner erkannt. Es wurden keine bestehenden Dateien überschrieben.`);
  }

  private async importEvidence(role: EvidenceRole, replaceEvidenceId?: string): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    if (replaceEvidenceId && !window.confirm("Vorhandene Evidence durch die neu ausgewählte Datei ersetzen? Die bisherige verwaltete Kopie wird lokal archiviert.")) return;
    const track = this.requireTrack();
    const metadata = this.collectEvidenceMetadata(role);
    if (metadata === null) return;
    const imported = await this.withBusy(
      "Datei auswählen; große Dateien werden im Hintergrund kopiert und gehasht …",
      () => this.api.importEvidence(track.id, role, replaceEvidenceId, metadata)
    );
    if (imported) {
      if (role === "final_artwork") this.trackCoverCache.delete(track.id);
      this.applyTrack(imported);
      const overriddenBySunoMetadata = role === "suno_final_export"
        && imported.automation.sunoMetadataDetected
        && this.hasManualSunoDateOverriddenByMetadata(track, imported);
      const sunoSummary = role === "suno_final_export" && imported.automation.sunoMetadataDetected
        ? ` Suno Studio und der eingebettete Erzeugungszeitpunkt wurden automatisch erkannt.${imported.automation.sunoId ? " Die technische Suno-ID wurde als Evidence erhalten." : ""}`
        : "";
      this.showToast(
        "success",
        replaceEvidenceId ? "Evidence ersetzt" : "Evidence importiert",
        `${evidenceRoleLabel(role)} wurde kopiert, gehasht und dem Track zugeordnet.${replaceEvidenceId ? " Die vorherige Kopie wurde archiviert." : ""}${sunoSummary}`
      );
      if (overriddenBySunoMetadata) {
        this.showToast(
          "info",
          "Suno-Metadaten überschreiben Nutzerangabe",
          "Abweichende Benutzerangabe durch Suno-WAV-Metadaten erkannt. Die technisch aus dem WAV gewonnene Information wird als Evidence-derived metadata verwendet."
        );
      }
    }
  }

  private async importGlobalTermsEvidence(): Promise<void> {
    const imported = await this.withBusy(
      "Nutzungsbedingungen auswählen, global speichern und in Projekte kopieren …",
      async () => {
        const item = await this.api.importGlobalTermsEvidence();
        if (!item) return null;
        return { item, globalEvidence: await this.api.listGlobalEvidence() };
      }
    );
    if (!imported) return;
    this.state.globalEvidence = imported.globalEvidence;
    await this.refreshTracks();
    this.showToast(
      "success",
      "Globale Nutzungsbedingungen registriert",
      "Die Datei wurde lokal gehasht und in jedes noch bearbeitbare Projekt kopiert. Finalisierte Snapshots wurden nicht verändert."
    );
    this.render();
  }

  private collectEvidenceMetadata(role: EvidenceRole): Partial<EvidenceMetadata> | null | undefined {
    if (role === "external_timestamp") {
      const provider = window.prompt("Zeitstempel-Provider / Aussteller:")?.trim();
      if (!provider) return null;
      const externalTimestamp = window.prompt("Zeitstempel (mit Zeitzone, soweit dokumentiert):")?.trim();
      if (!externalTimestamp) return null;
      const referencedHash = window.prompt("Referenzierter Hash:")?.trim();
      if (!referencedHash) return null;
      const referencedArtifact = window.prompt("Referenziertes Artefakt (z. B. Manifest oder Zertifikat):")?.trim();
      if (!referencedArtifact) return null;
      return { provider, externalTimestamp, referencedHash, referencedArtifact };
    }
    return undefined;
  }

  private async previewEvidence(evidenceId: string): Promise<void> {
    const track = this.requireTrack();
    const preview = await this.withBusy("Evidence-Vorschau wird vorbereitet …", () => this.api.previewEvidence(track.id, evidenceId));
    if (!preview) return;
    this.state.showNewTrack = false;
    this.state.showTrackLibrary = false;
    this.state.showSubscriptionEvidence = false;
    this.state.evidencePreview = preview;
    this.render();
  }

  private async chooseEvidenceRole(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    const labels = evidenceRoles.map((role, index) => `${index + 1}: ${evidenceRoleLabel(role)}`).join("\n");
    const choice = window.prompt(`Rolle der Evidence wählen:\n\n${labels}\n\nNummer eingeben:`);
    if (!choice) return;
    const role = evidenceRoles[Number(choice) - 1];
    if (!role) { this.showToast("error", "Ungültige Rolle", "Wähle eine Nummer aus der angezeigten Liste."); return; }
    const existing = ["release_wav", "suno_final_export", "final_artwork"].includes(role)
      ? [...this.requireTrack().evidence].reverse().find((item) => item.role === role)
      : undefined;
    await this.importEvidence(role, existing?.id);
  }

  private async addDeviation(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const description = window.prompt("Abweichung sachlich beschreiben:");
    if (!description?.trim()) return;
    const blocking = window.confirm("Soll diese Abweichung die Finalisierung blockieren?");
    await this.trackMutation("Abweichung wird gespeichert …", () => this.api.addDeviation(this.requireTrack().id, description, blocking), "Abweichung gespeichert");
  }

  private async saveTrackDraft(): Promise<void> {
    if (!this.state.trackDraft) return;
    if (this.rejectLockedContentMutation()) return;
    const normalizedDraft = normalizeGuidedTrackFields(this.state.trackDraft);
    const updated = await this.withBusy("Track-Angaben werden gespeichert …", () => this.api.updateTrack(this.requireTrack().id, normalizedDraft));
    if (updated) { this.applyTrack(updated); this.showToast("success", "Schritt gespeichert", "Der Dokumentationsstatus wurde neu bewertet."); }
  }

  private async flushDraft(): Promise<boolean> {
    if (!this.draftDirty || !this.state.trackDraft || !this.state.track) return true;
    if (shouldDiscardLockedDraft(this.state.track.status, this.draftDirty)) {
      this.state.trackDraft = structuredClone(this.state.track.fields);
      this.draftDirty = false;
      return true;
    }
    const normalizedDraft = normalizeGuidedTrackFields(this.state.trackDraft);
    const updated = await this.withBusy("Ungespeicherte Angaben werden zuerst gesichert …", () => this.api.updateTrack(this.state.track!.id, normalizedDraft));
    if (!updated) return false;
    this.applyTrack(updated);
    return true;
  }

  private async trackMutation(label: string, action: () => Promise<TrackDetail>, success: string, allowLocked = false): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = await this.withBusy(label, action);
    if (track) { this.applyTrack(track); this.showToast("success", success, "Der Track-Status wurde neu bewertet."); }
  }

  private async runAction(label: string, action: () => Promise<{ message: string; track?: TrackDetail }>, allowLocked = false): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const result = await this.withBusy(label, action);
    if (!result) return;
    if (result.track) this.applyTrack(result.track);
    else await this.refreshTracks();
    this.showToast("success", "Aktion abgeschlossen", result.message);
  }

  private async runProgressAction(
    kind: LongOperationKind,
    label: string,
    action: (onProgress: (progress: OperationProgress) => void) => Promise<{ message: string; track?: TrackDetail }>,
    allowLocked = false
  ): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const result = await this.withOperationProgress(kind, label, action);
    if (!result) return;
    if (result.track) this.applyTrack(result.track);
    else await this.refreshTracks();
    this.showToast("success", "Aktion abgeschlossen", result.message);
  }

  private async finalizeTrack(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const workflowBlocker = workflowUpgradeFinalizationBlocker(track, this.state.workflow);
    if (workflowBlocker) {
      this.showToast("error", "Workflow-Neubewertung erforderlich", workflowBlocker);
      return;
    }
    const validation = await this.withBusy("Finalisierungs-Gate wird nativ geprüft …", () => this.api.validateTrack(track.id));
    if (!validation) return;
    if (!validation.valid) { this.showToast("error", "Finalisierung blockiert", [...validation.missingItems, ...validation.blockingItems].join(" · ")); return; }
    const result = await this.withOperationProgress(
      "finalization",
      "Unveränderlicher Snapshot und Zertifikat werden erzeugt …",
      (onProgress) => this.api.finalizeTrack(track.id, onProgress)
    );
    if (result) {
      if (result.track) this.applyTrack(result.track); else await this.refreshTracks();
      this.state.trackTab = "certificate"; this.state.activeStep = null;
      this.state.showCertificatePopup = Boolean(
        this.state.track?.certificate.valid && this.state.track.certificate.certificateId
      );
      this.showToast("success", "Dokumentation finalisiert", result.message);
    }
  }

  private async generateDocumentsSafely(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const preview = await this.withBusy("Dokumentgenerierung wird sicher vorbereitet …", () => this.api.previewDocumentGeneration(track.id));
    if (!preview) return;
    let adoptExisting = false;
    if (preview.adoptionRequired || preview.collisions.length > 0) {
      const collisionList = preview.collisions.join("\n");
      adoptExisting = window.confirm(`Bestehende verwaltete Dokumente erkannt:\n\n${collisionList}\n\nDie native Anwendung sichert den vorhandenen Zustand unter .archive, bevor neue verwaltete Dokumente geschrieben werden. Fortfahren?`);
      if (!adoptExisting) {
        this.showToast("info", "Dokumentgenerierung abgebrochen", "Bestehende Dateien wurden nicht verändert.");
        return;
      }
    }
    await this.runProgressAction(
      "documents",
      "Dokumente werden atomar erzeugt …",
      (onProgress) => this.api.generateDocuments(track.id, adoptExisting, onProgress)
    );
  }
}
