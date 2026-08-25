import { SYSTEM_TRANSLATIONS_01 } from "./system-translations/catalog-01";
import { SYSTEM_TRANSLATIONS_02 } from "./system-translations/catalog-02";
import { SYSTEM_TRANSLATIONS_03 } from "./system-translations/catalog-03";
import { SYSTEM_TRANSLATIONS_04 } from "./system-translations/catalog-04";
import type { SystemTranslation } from "./system-translations/catalog-types";

/**
 * User-facing messages that originate outside the handwritten UI copy:
 * workflow definitions, native commands, provider adapters, and browser demo
 * responses. Tuple order is always [German, English].
 *
 * Dynamic values use the placeholder shape used by their producer. A consumer
 * should resolve the catalog entry before interpolating a path, count, or ID.
 */
export type { SystemTranslation } from "./system-translations/catalog-types";

export const SYSTEM_TRANSLATIONS: readonly SystemTranslation[] = [
  ...SYSTEM_TRANSLATIONS_01,
  ...SYSTEM_TRANSLATIONS_02,
  ...SYSTEM_TRANSLATIONS_03,
  ...SYSTEM_TRANSLATIONS_04
];
