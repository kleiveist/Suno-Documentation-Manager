import type { AppLanguage } from "./i18n";

function localeFor(language: AppLanguage): string {
  return language === "en" ? "en-US" : "de-DE";
}

export function escapeHtml(value: unknown): string {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

export function formatDate(value?: string, includeTime = false, language: AppLanguage = "de"): string {
  if (!value) return language === "en" ? "Not yet" : "Noch nicht";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(localeFor(language), {
    dateStyle: "medium",
    ...(includeTime ? { timeStyle: "short" as const } : {})
  }).format(date);
}

export function formatBytes(bytes: number, language: AppLanguage = "de"): string {
  if (!Number.isFinite(bytes) || bytes < 1) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toLocaleString(localeFor(language), { maximumFractionDigits: 1 })} ${units[index]}`;
}

export function titleInitials(title: string): string {
  return title.split(/\s+/).slice(0, 2).map((word) => word[0]?.toUpperCase()).join("") || "ST";
}
