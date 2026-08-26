import type { EvidenceRole, TrackDetail, TrackFieldPatch, TrackFields } from "../domain/types";
import { escapeHtml } from "../ui/format";

export function parseMultiChoiceValue(value: string): string[] {
  return value
    .split(" | ")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function serializeMultiChoiceValue(values: string[]): string {
  return [...new Set(values.map((item) => item.trim()).filter(Boolean))].join(" | ");
}

export type GuidedChoice = readonly [value: string, label: string, aliases?: readonly string[]];

export type SingleChoiceOption = readonly [value: string, label: string];

export const normalizedChoiceText = (value: string): string =>
  value.trim().normalize("NFKC").toLocaleLowerCase("de-DE");

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

export const evidenceRoles: EvidenceRole[] = [
  "suno_final_export",
  "suno_project_zip",
  "suno_screenshot",
  "release_wav",
  "release_mp3",
  "release_mp4",
  "release_artwork",
  "artwork_suno_original",
  "ai_artwork_original",
  "ai_artwork_edited",
  "human_edited_artwork",
  "final_artwork",
  "external_audio_file",
  "external_audio_license",
  "own_audio_file",
  "source_code_file",
  "code_generated_audio_file",
  "third_party_sample_file",
  "third_party_sample_license",
  "lyrics",
  "style",
  "other"
];

export const externalAudioSourceChoices: readonly GuidedChoice[] = [
  ["Audio from a licensed sample library", "Lizenzierte Sample-Bibliothek"],
  ["Licensed beat or instrumental", "Lizenzierter Beat oder Instrumentaltrack"],
  ["Audio supplied by a collaborator", "Von Mitwirkenden bereitgestelltes Audio"],
  ["Commissioned recording", "Beauftragte Aufnahme"],
  ["Public-domain recording", "Gemeinfreie Aufnahme"],
  ["Creative Commons recording", "Aufnahme unter Creative-Commons-Lizenz"]
];

export const externalAudioRightsChoices: readonly GuidedChoice[] = [
  ["Commercial-use license", "Lizenz für kommerzielle Nutzung"],
  ["Direct permission from the rights holder", "Direkte Erlaubnis des Rechteinhabers"],
  ["Joint rights agreement", "Gemeinsame Rechtevereinbarung"],
  ["Public domain", "Gemeinfreiheit"],
  ["Creative Commons license", "Creative-Commons-Lizenz"]
];

export const ownAudioSourceChoices: readonly GuidedChoice[] = [
  ["Original vocal recording", "Eigene Gesangsaufnahme"],
  ["Original instrument recording", "Eigene Instrumentalaufnahme"],
  ["Original field recording", "Eigene Feldaufnahme", ["Eigene Aufnahme"]],
  ["Original MIDI or software render", "Eigener MIDI- oder Software-Render"],
  ["Original sound design", "Eigenes Sounddesign"]
];

export const ownAudioRightsChoices: readonly GuidedChoice[] = [
  ["Solely owned by the artist", "Ausschließlich eigene Rechte", ["Eigene Produktion"]],
  ["Jointly owned with collaborators", "Gemeinsame Rechte mit Mitwirkenden"],
  ["Participant permissions documented", "Einwilligungen der Beteiligten dokumentiert"]
];

export const sampleSourceChoices: readonly GuidedChoice[] = [
  ["Commercial sample library", "Kommerzielle Sample-Bibliothek"],
  ["Royalty-free sample pack", "Royalty-free Sample-Pack"],
  ["Directly licensed from the sample creator", "Direkt vom Sample-Urheber lizenziert"],
  ["Public-domain archive", "Gemeinfreies Archiv"],
  ["Creative Commons source", "Creative-Commons-Quelle"]
];

export const sampleRightsChoices: readonly GuidedChoice[] = [
  ["Commercial sample license", "Kommerzielle Sample-Lizenz"],
  ["Royalty-free license", "Royalty-free Lizenz"],
  ["Direct permission from the rights holder", "Direkte Erlaubnis des Rechteinhabers"],
  ["Public domain", "Gemeinfreiheit"],
  ["Creative Commons license", "Creative-Commons-Lizenz"]
];

export const humanWorkChoices: readonly GuidedChoice[] = [
  ["Arrangement", "Arrangement"],
  ["Lyrics", "Lyrics"],
  ["Timing and cuts", "Timing und Cuts"],
  ["Sound design", "Sounddesign"],
  ["EQ", "EQ"],
  ["Mixing", "Mixing"],
  ["Mastering", "Mastering"],
  ["Loudness adjustment", "Lautheitsanpassung"]
];

export const postExportWorkChoices: readonly GuidedChoice[] = [
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

export const codeAudioPostProcessingChoices: readonly GuidedChoice[] = [
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

export const humanArtworkProcessChoices: readonly GuidedChoice[] = [
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

export const aiArtworkHumanChangeChoices: readonly GuidedChoice[] = [
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

export const sunoModelSuggestions = [
  "v5.5",
  "v5",
  "v4.5-all",
  "v4.5+",
  "v4.5",
  "v4",
  "v3.5",
  "v3",
  "Custom Model / v5.5 Custom Model"
] as const;

export const sunoPlanSuggestions = ["Free", "Pro", "Premier"] as const;

export const aiSystemSuggestions = ["Suno", "ChatGPT / OpenAI", "Udio", "Stable Audio", "ElevenLabs"] as const;

export const SUNO_CONTENT_CLASSIFICATION_CHOICES: readonly SingleChoiceOption[] = [
  ["", "Bitte auswählen"],
  ["STRUCTURE_ONLY", "STRUCTURE_ONLY – nur Struktur-, Sound- oder Arrangement-Anweisungen"],
  ["VOCAL_LYRICS_ONLY", "VOCAL_LYRICS_ONLY – nur Vocal Lyrics"],
  ["MIXED", "MIXED – Vocal Lyrics und Struktur-Anweisungen"],
  ["EMPTY", "EMPTY – Textfeld leer"],
  ["OTHER", "OTHER – sonstiger Inhalt"]
];

export const VOCAL_INTENT_CHOICES: readonly SingleChoiceOption[] = [
  ["", "Bitte auswählen"],
  ["VOCAL", "VOCAL – Gesang beabsichtigt"],
  ["INSTRUMENTAL", "INSTRUMENTAL – kein Gesang beabsichtigt"],
  ["UNSPECIFIED", "UNSPECIFIED – bewusst nicht spezifiziert"]
];

export const audioDisclosureLocationChoices: readonly GuidedChoice[] = [
  ["release metadata", "Release-Metadaten"],
  ["distributor metadata", "Distributor-Metadaten"],
  ["description", "Beschreibung"],
  ["artwork", "Artwork"],
  ["credits", "Credits"],
  ["website", "Website"],
  ["other", "Sonstiger Ort"]
];

export const releaseNoteChoices: readonly GuidedChoice[] = [
  ["Original Suno version", "Originale Suno-Fassung"],
  ["Streaming master", "Streaming-Master"],
  ["Radio edit", "Radio Edit"],
  ["Extended mix", "Extended Mix"],
  ["Instrumental version", "Instrumental"],
  ["Clean version", "Clean Version"],
  ["Explicit version", "Explicit Version"],
  ["Social-media version", "Social-Media-Version"]
];

export function normalizeGuidedTrackFields(fields: TrackFields): TrackFields {
  const normalized = structuredClone(fields);
  if (normalized.sunoContentClassification !== null) {
    normalized.sunoLyricsFieldContent = null;
    normalized.sunoLyricsContentTypes = [];
    if (normalized.sunoContentClassification === "EMPTY") {
      normalized.sunoLyricsContentSource = null;
      normalized.sunoLyricsFieldText = "";
      normalized.sunoLyricsOtherContentType = "";
    } else if (normalized.sunoContentClassification !== "OTHER") {
      normalized.sunoLyricsOtherContentType = "";
    }
  }
  normalized.externalAudioSource = canonicalGuidedChoiceValue(
    normalized.externalAudioSource,
    externalAudioSourceChoices
  );
  normalized.externalAudioOwnership = canonicalGuidedChoiceValue(
    normalized.externalAudioOwnership,
    externalAudioRightsChoices
  );
  normalized.ownAudioSource = canonicalGuidedChoiceValue(normalized.ownAudioSource, ownAudioSourceChoices);
  normalized.ownAudioOwnership = canonicalGuidedChoiceValue(normalized.ownAudioOwnership, ownAudioRightsChoices);
  normalized.thirdPartySampleSource = canonicalGuidedChoiceValue(
    normalized.thirdPartySampleSource,
    sampleSourceChoices
  );
  normalized.thirdPartySampleOwnership = canonicalGuidedChoiceValue(
    normalized.thirdPartySampleOwnership,
    sampleRightsChoices
  );
  normalized.humanEditingDetails = canonicalGuidedChoiceList(normalized.humanEditingDetails, humanWorkChoices);
  normalized.postExportEditingDetails = canonicalGuidedChoiceList(
    normalized.postExportEditingDetails,
    postExportWorkChoices
  );
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

export function canonicalTrackFieldPatch(fields: TrackFields): TrackFieldPatch {
  const normalized = normalizeGuidedTrackFields(fields);
  const {
    sunoLyricsFieldContent: _legacyContentAnswer,
    sunoLyricsContentTypes: _legacyContentTypes,
    ...patch
  } = normalized;
  void _legacyContentAnswer;
  void _legacyContentTypes;
  return patch;
}

export const evidenceProvenanceLabel = (value: TrackDetail["evidence"][number]["provenance"]): string =>
  ({
    managed_copy: "Verwaltete Kopie",
    global_copy: "Globale portable Kopie",
    generated_disclosure: "Lokal erzeugter Disclosure-Nachweis",
    indexed_legacy: "Historisch indexiert"
  })[value ?? "managed_copy"];
