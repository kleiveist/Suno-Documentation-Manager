export const activeProfileId = "desktop-local";
export const activeProfileName = "Desktop local";
export const enabledFeatures = ["frontend", "tauri"] as const;
export type ProjectFeature = (typeof enabledFeatures)[number];

const featureSet = new Set<string>(enabledFeatures);

export function hasFeature(feature: string): boolean {
  return featureSet.has(feature);
}
