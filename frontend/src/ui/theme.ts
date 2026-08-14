export const THEME_STORAGE_KEY = "suno-documentation-theme";

export type ColorTheme = "light" | "dark";

export function storedTheme(value: string | null): ColorTheme | null {
  return value === "light" || value === "dark" ? value : null;
}

export function resolveTheme(value: string | null, systemPrefersDark: boolean): ColorTheme {
  return storedTheme(value) ?? (systemPrefersDark ? "dark" : "light");
}

export function toggledTheme(theme: ColorTheme): ColorTheme {
  return theme === "dark" ? "light" : "dark";
}
