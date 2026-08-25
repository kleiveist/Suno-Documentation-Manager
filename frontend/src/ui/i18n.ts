import { SYSTEM_TRANSLATIONS } from "./system-translations";
import { PRIMARY_TRANSLATIONS_01 } from "./i18n/primary-01";
import { PRIMARY_TRANSLATIONS_02 } from "./i18n/primary-02";
import { SUPPLEMENTAL_TRANSLATIONS_01 } from "./i18n/supplemental-01";
import { SUPPLEMENTAL_TRANSLATIONS_02 } from "./i18n/supplemental-02";
import type { Translation } from "./i18n/catalog-types";

export type AppLanguage = "de" | "en";

/*
 * The app deliberately keeps domain values and persisted document content
 * language-neutral. This catalog is only for UI copy. Keeping it here gives
 * all renderers one translation boundary without changing track snapshots.
 */
const TRANSLATIONS: readonly Translation[] = [...PRIMARY_TRANSLATIONS_01, ...PRIMARY_TRANSLATIONS_02];

/*
 * These are application-owned labels which are emitted by presentation
 * helpers, native dialogs, or dynamically assembled status cards.  Keeping
 * them here makes the boundary explicit: arbitrary user content is never
 * translated, while known system copy is available in both directions.
 */
const SUPPLEMENTAL_TRANSLATIONS: readonly Translation[] = [
  ...SUPPLEMENTAL_TRANSLATIONS_01,
  ...SUPPLEMENTAL_TRANSLATIONS_02
];

/*
 * UI copy can originate in either layer: most HTML is authored in German,
 * while technical status values and native command results are commonly
 * English. Keep both lookup directions so a German UI never leaks an
 * otherwise known English system message (and vice versa).
 */
const ALL_TRANSLATIONS: readonly Translation[] = [
  ...TRANSLATIONS,
  ...SUPPLEMENTAL_TRANSLATIONS,
  ...SYSTEM_TRANSLATIONS
];
const germanToEnglish = new Map(ALL_TRANSLATIONS);
const englishToGerman = new Map<string, string>();
for (const [german, english] of ALL_TRANSLATIONS) {
  // A few short technical terms deliberately have several German usages
  // (for example "Open" as a state or an action). The first catalog entry is
  // the stable fallback; explicit source text remains unmodified in its own
  // language.
  if (!englishToGerman.has(english)) englishToGerman.set(english, german);
}

// Only system messages may be composed from several catalogued requirement
// texts (for example the finalization gate). Keep this deliberately separate
// from ordinary UI copy so titles, paths, and other user values are never
// scanned for translatable fragments.
const germanToEnglishSystemFragments = SYSTEM_TRANSLATIONS.filter(
  ([german, english]) => german.length >= 12 && !/\{[^}]*\}/.test(german) && german !== english
)
  .slice()
  .sort(([left], [right]) => right.length - left.length);
const englishToGermanSystemFragments = SYSTEM_TRANSLATIONS.filter(
  ([german, english]) => english.length >= 12 && !/\{[^}]*\}/.test(english) && german !== english
)
  .slice()
  .sort(([, left], [, right]) => right.length - left.length);

interface TemplateTranslation {
  readonly matcher: RegExp;
  readonly replacement: string;
  readonly captureNames: readonly string[];
  readonly translateCaptures: readonly boolean[];
}

function escapeRegularExpression(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function templateTranslation(source: string, replacement: string): TemplateTranslation | null {
  if (!/\{[^}]*\}/.test(source)) return null;
  const placeholders = [...source.matchAll(/\{([^}]*)\}/g)].map((match) => match[1]);
  const matcher = source
    .split(/(\{[^}]*\})/g)
    .map((part) => (/^\{[^}]*\}$/.test(part) ? "(.+?)" : escapeRegularExpression(part)))
    .join("");
  return {
    matcher: new RegExp(`^${matcher}$`),
    replacement,
    captureNames: placeholders,
    // Native AppError wraps a known system message in {0}; translate that
    // nested system message, but never reinterpret paths, titles, or any
    // other user-provided placeholder value.
    translateCaptures: placeholders.map((name) =>
      [
        "0",
        "label",
        "role",
        "kind",
        "status",
        "field",
        "error",
        "reason",
        "message",
        "source",
        "items",
        "types"
      ].includes(name)
    )
  };
}

const englishToGermanTemplates = ALL_TRANSLATIONS.map(([german, english]) =>
  templateTranslation(english, german)
).filter((entry): entry is TemplateTranslation => entry !== null);
const germanToEnglishTemplates = ALL_TRANSLATIONS.map(([german, english]) =>
  templateTranslation(german, english)
).filter((entry): entry is TemplateTranslation => entry !== null);

function translateTemplateUiText(value: string, language: AppLanguage): string | null {
  const templates = language === "en" ? germanToEnglishTemplates : englishToGermanTemplates;
  for (const { matcher, replacement, captureNames, translateCaptures } of templates) {
    const match = value.match(matcher);
    if (!match) continue;
    let index = 0;
    return replacement.replace(/\{[^}]*\}/g, () => {
      const captured = match[index + 1] ?? "";
      const translated = translateCaptures[index]
        ? translateSystemMessageCapture(
            captured,
            language,
            ["0", "error", "reason", "message", "source"].includes(captureNames[index] ?? "")
          )
        : captured;
      index += 1;
      return translated;
    });
  }
  return null;
}

function translateSystemMessageCapture(value: string, language: AppLanguage, hideUnknownDiagnostic = false): string {
  const direct = translateUiText(value, language);
  if (direct !== value || hasUiTranslation(value, language)) return direct;
  const fragments = language === "en" ? germanToEnglishSystemFragments : englishToGermanSystemFragments;
  const fragmentTranslation = fragments.reduce(
    (result, [german, english]) =>
      result.replaceAll(language === "en" ? german : english, language === "en" ? english : german),
    value
  );
  if (fragmentTranslation !== value || !hideUnknownDiagnostic) return fragmentTranslation;
  // Paths are evidence data, not diagnostics. They must remain visible even
  // when an enclosing native validation message is localized.
  if (/[\\/]/.test(value)) return value;
  return language === "en" ? "Technical detail is unavailable." : "Technisches Detail ist nicht verfügbar.";
}

function replaceTrimmed(value: string, replacement: string): string {
  const trimmed = value.trim();
  return trimmed ? value.replace(trimmed, replacement) : value;
}

function translateDynamicUiText(value: string, language: AppLanguage): string | null {
  return language === "en" ? translateDynamicGermanToEnglish(value) : translateDynamicEnglishToGerman(value);
}

function translateDynamicGermanToEnglish(value: string): string | null {
  const screeningSummary = value.match(/^(\d+) % · Ziel: ca\. (\d+) Sekunden · (\d+) ACRCloud-Requests$/);
  if (screeningSummary) {
    return `${screeningSummary[1]}% · target: approx. ${screeningSummary[2]} seconds · ${screeningSummary[3]} ACRCloud requests`;
  }
  const screeningReferencePreview = value.match(/^Dynamisch · Vorschau mit Referenzlänge (.+)$/);
  if (screeningReferencePreview) return `Dynamic · preview with ${screeningReferencePreview[1]} reference length`;
  const screeningFixedPreview = value.match(/^Feste Referenzlänge: (.+)$/);
  if (screeningFixedPreview) return `Fixed reference length: ${screeningFixedPreview[1]}`;
  const screeningRequestBand = value.match(/^(\d+) erwartete Requests · (niedrig|normal|erhöht|hoch|sehr hoch)$/);
  if (screeningRequestBand) {
    const band = (
      { niedrig: "low", normal: "normal", erhöht: "elevated", hoch: "high", "sehr hoch": "very high" } as Record<
        string,
        string
      >
    )[screeningRequestBand[2]];
    return `${screeningRequestBand[1]} expected requests · ${band}`;
  }
  if (
    value ===
    "Feste Obergrenze: maximal 25 ACRCloud-Requests bzw. 300 Sekunden eindeutiges Audio pro Track; jeder Request enthält höchstens 12 Sekunden."
  ) {
    return "Hard limit: at most 25 ACRCloud requests and 300 seconds of unique audio per track; each request contains at most 12 seconds.";
  }
  const greeting = value.match(/^Guten Tag, (.+)\.$/);
  if (greeting) return `Good day, ${greeting[1]}.`;
  const missing = value.match(/^Bei (.+) sind noch (\d+) Pflichtpunkte offen\.$/);
  if (missing) return `There are still ${missing[2]} required items open for ${missing[1]}.`;
  const missingCount = value.match(/^(\d+) erforderliche Angaben oder Nachweise fehlen noch\.$/);
  if (missingCount) return `${missingCount[1]} required details or evidence items are still missing.`;
  const openCount = value.match(/^(\d+) offen$/);
  if (openCount) return `${openCount[1]} open`;
  const detected = value.match(/^(\d+) erkannt$/);
  if (detected) return `${detected[1]} detected`;
  const indexed = value.match(/^(\d+) indexiert$/);
  if (indexed) return `${indexed[1]} indexed`;
  const unchanged = value.match(/^(\d+) unverändert$/);
  if (unchanged) return `${unchanged[1]} unchanged`;
  const fileCount = value.match(/^(\d+) Dateien$/);
  if (fileCount) return `${fileCount[1]} files`;
  const trackCount = value.match(/^(\d+) Tracks$/);
  if (trackCount) return `${trackCount[1]} tracks`;
  const albumCount = value.match(/^(\d+) Alben$/);
  if (albumCount) return `${albumCount[1]} albums`;
  const coverage = value.match(/^Abgedeckter Zeitraum: (.+)$/);
  if (coverage) return `Covered period: ${coverage[1]}`;
  const preview = value.match(/^Vorschau von (.+)$/);
  if (preview) return `Preview of ${preview[1]}`;
  const scan = value.match(/^(\d+) Track-Ordner erkannt\. Es wurden keine bestehenden Dateien überschrieben\.$/);
  if (scan) return `${scan[1]} track folders detected. No existing files were overwritten.`;
  return null;
}

function translateDynamicEnglishToGerman(value: string): string | null {
  const screeningSummary = value.match(/^(\d+)% · target: approx\. (\d+) seconds · (\d+) ACRCloud requests$/);
  if (screeningSummary) {
    return `${screeningSummary[1]} % · Ziel: ca. ${screeningSummary[2]} Sekunden · ${screeningSummary[3]} ACRCloud-Requests`;
  }
  const screeningReferencePreview = value.match(/^Dynamic · preview with (.+) reference length$/);
  if (screeningReferencePreview) return `Dynamisch · Vorschau mit Referenzlänge ${screeningReferencePreview[1]}`;
  const screeningFixedPreview = value.match(/^Fixed reference length: (.+)$/);
  if (screeningFixedPreview) return `Feste Referenzlänge: ${screeningFixedPreview[1]}`;
  const screeningRequestBand = value.match(/^(\d+) expected requests · (low|normal|elevated|high|very high)$/);
  if (screeningRequestBand) {
    const band = (
      { low: "niedrig", normal: "normal", elevated: "erhöht", high: "hoch", "very high": "sehr hoch" } as Record<
        string,
        string
      >
    )[screeningRequestBand[2]];
    return `${screeningRequestBand[1]} erwartete Requests · ${band}`;
  }
  if (
    value ===
    "Hard limit: at most 25 ACRCloud requests and 300 seconds of unique audio per track; each request contains at most 12 seconds."
  ) {
    return "Feste Obergrenze: maximal 25 ACRCloud-Requests bzw. 300 Sekunden eindeutiges Audio pro Track; jeder Request enthält höchstens 12 Sekunden.";
  }
  const greeting = value.match(/^Good day, (.+)\.$/);
  if (greeting) return `Guten Tag, ${greeting[1]}.`;
  const missing = value.match(/^There are still (\d+) required items open for (.+)\.$/);
  if (missing) return `Bei ${missing[2]} sind noch ${missing[1]} Pflichtpunkte offen.`;
  const missingCount = value.match(/^(\d+) required details or evidence items are still missing\.$/);
  if (missingCount) return `${missingCount[1]} erforderliche Angaben oder Nachweise fehlen noch.`;
  const openCount = value.match(/^(\d+) open$/);
  if (openCount) return `${openCount[1]} offen`;
  const detected = value.match(/^(\d+) detected$/);
  if (detected) return `${detected[1]} erkannt`;
  const indexed = value.match(/^(\d+) indexed$/);
  if (indexed) return `${indexed[1]} indexiert`;
  const unchanged = value.match(/^(\d+) unchanged$/);
  if (unchanged) return `${unchanged[1]} unverändert`;
  const fileCount = value.match(/^(\d+) files$/);
  if (fileCount) return `${fileCount[1]} Dateien`;
  const trackCount = value.match(/^(\d+) tracks$/);
  if (trackCount) return `${trackCount[1]} Tracks`;
  const albumCount = value.match(/^(\d+) albums$/);
  if (albumCount) return `${albumCount[1]} Alben`;
  const coverage = value.match(/^Covered period: (.+)$/);
  if (coverage) return `Abgedeckter Zeitraum: ${coverage[1]}`;
  const preview = value.match(/^Preview of (.+)$/);
  if (preview) return `Vorschau von ${preview[1]}`;
  const scan = value.match(/^(\d+) track folders detected\. No existing files were overwritten\.$/);
  if (scan) return `${scan[1]} Track-Ordner erkannt. Es wurden keine bestehenden Dateien überschrieben.`;
  return null;
}

/** Translate known application copy in either direction without touching unknown user content. */
export function translateUiText(value: string, language: AppLanguage): string {
  const trimmed = value.trim();
  if (!trimmed) return value;
  const exact = (language === "en" ? germanToEnglish : englishToGerman).get(trimmed);
  if (exact) return replaceTrimmed(value, exact);
  // Dynamic UI summaries use punctuation such as `:` and must be recognized
  // before generic `{field}: {error}` templates, otherwise their numeric
  // values are mistaken for an unknown native diagnostic.
  const dynamic = translateDynamicUiText(trimmed, language);
  if (dynamic) return replaceTrimmed(value, dynamic);
  const templated = translateTemplateUiText(trimmed, language);
  if (templated) return replaceTrimmed(value, templated);
  return value;
}

/** Allows focused tests and future UI code to distinguish catalogued copy from user-provided data. */
export function hasUiTranslation(value: string, _language: AppLanguage): boolean {
  void _language;
  const trimmed = value.trim();
  if (!trimmed) return true;
  // A system message can already be in the target language. It is still
  // known application copy and must be retained instead of being replaced by
  // the generic safety fallback used for unknown native diagnostics.
  return (
    germanToEnglish.has(trimmed) ||
    englishToGerman.has(trimmed) ||
    germanToEnglishTemplates.some(({ matcher }) => matcher.test(trimmed)) ||
    englishToGermanTemplates.some(({ matcher }) => matcher.test(trimmed)) ||
    translateDynamicUiText(trimmed, "en") !== null ||
    translateDynamicUiText(trimmed, "de") !== null
  );
}

/**
 * Translates rendered application copy while leaving caller-marked data (for
 * example track titles, paths, lyrics, and free-form deviations) unchanged.
 * The renderer still handles static legacy templates, but it must never turn
 * a user title such as "Settings" into a localized navigation label.
 */
export function translateRenderedUi(
  root: HTMLElement,
  language: AppLanguage,
  protectedValues: ReadonlySet<string> = new Set()
): void {
  const document = root.ownerDocument;
  const walker = document.createTreeWalker(root, 4);
  let node: Node | null = walker.nextNode();
  while (node) {
    const value = node.nodeValue ?? "";
    const trimmed = value.trim();
    if (trimmed && !protectedValues.has(trimmed)) {
      const translated = translateUiText(trimmed, language);
      if (translated !== trimmed) node.nodeValue = value.replace(trimmed, translated);
    }
    node = walker.nextNode();
  }
  root.querySelectorAll<HTMLElement>("*").forEach((element) => {
    for (const attribute of ["aria-label", "aria-description", "title", "placeholder", "alt"]) {
      const value = element.getAttribute(attribute);
      if (!value || protectedValues.has(value.trim())) continue;
      const translated = translateUiText(value, language);
      if (translated !== value) element.setAttribute(attribute, translated);
    }
  });
}
