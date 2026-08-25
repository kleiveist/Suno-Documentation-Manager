import type { GlobalProfile, TrackDetail, WorkflowDefinitionDto } from "../domain/types";
import { translateUiText } from "../ui/i18n";
import type { AppLanguage } from "../ui/i18n";
import type { MainView, SettingsCategory } from "./state";

export const MAIN_NAVIGATION: ReadonlyArray<{
  id: MainView;
  label: string;
  iconName: "dashboard" | "tracks" | "current" | "workspace" | "settings";
}> = [
  { id: "dashboard", label: "Dashboard", iconName: "dashboard" },
  { id: "tracks", label: "Tracks", iconName: "tracks" },
  { id: "current", label: "Aktueller Track", iconName: "current" },
  { id: "workspace", label: "Workspace", iconName: "workspace" },
  { id: "settings", label: "Einstellungen", iconName: "settings" }
];

export const SETTINGS_CATEGORY_DEFINITIONS = [
  { id: "global", label: "Globale Angaben", description: "Workspace- und Produktionsvorgaben" },
  { id: "external", label: "Externe Dienste", description: "Optionale externe Integrationen" },
  { id: "files", label: "Globale Datei-Führung", description: "Workspaceweite Evidence-Dateien" }
] as const;

export function settingsCategoryNavigationMarkup(
  activeCategory: SettingsCategory = "global",
  language: AppLanguage = "de"
): string {
  return `<nav class="settings-category-nav" aria-label="${translateUiText("Einstellungsbereiche", language)}">${SETTINGS_CATEGORY_DEFINITIONS.map((category, index) => `<button type="button" class="settings-category-nav__item ${activeCategory === category.id ? "is-active" : ""}" data-settings-category="${category.id}" aria-controls="settings-${category.id}" aria-selected="${activeCategory === category.id}"><span class="settings-category-nav__index">${index + 1}</span><span><strong>${translateUiText(category.label, language)}</strong><small>${translateUiText(category.description, language)}</small></span></button>`).join("")}</nav>`;
}

export function missingProfileFields(profile: GlobalProfile): string[] {
  const fields: Array<[keyof GlobalProfile, string]> = [
    ["artistName", "Künstlername"],
    ["sunoProfileName", "Suno-Profilname"],
    ["sunoHandle", "Suno-Benutzername"],
    ["sunoPlan", "Suno-Tarif"],
    ["subscriptionStartDate", "Abo-Startdatum"],
    ["defaultAiImageService", "Standard-KI-Bilddienst"],
    ["artworkTransparencyPolicy", "Artwork-Transparenzrichtlinie"],
    ["disclosureText", "Standard-Hinweistext"]
  ];
  return fields
    .filter(([key]) => typeof profile[key] !== "string" || String(profile[key]).trim() === "")
    .map(([, label]) => label);
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
    message:
      track.status === "FINALIZED"
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
