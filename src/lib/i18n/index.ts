import { settingsStore } from "$lib/stores/settings.svelte";
import { en } from "./locales/en";
import { fr } from "./locales/fr";
import { es } from "./locales/es";
import { de } from "./locales/de";
import { pt } from "./locales/pt";
import { zh } from "./locales/zh";

export type LocaleCode = "en" | "fr" | "es" | "de" | "pt" | "zh";

/** The English catalogue is the source of truth; other locales mirror its keys. */
export type MessageKey = keyof typeof en;

const CATALOGUES: Record<LocaleCode, Partial<Record<MessageKey, string>>> = {
  en,
  fr,
  es,
  de,
  pt,
  zh,
};

/** Selectable languages, in the order shown in Settings. */
export const LOCALES: { code: LocaleCode; label: string }[] = [
  { code: "en", label: "English" },
  { code: "fr", label: "Français" },
  { code: "es", label: "Español" },
  { code: "de", label: "Deutsch" },
  { code: "pt", label: "Português" },
  { code: "zh", label: "中文" },
];

function normalize(code: string | undefined | null): LocaleCode {
  if (code && code in CATALOGUES) return code as LocaleCode;
  // Fall back to a base-language match (e.g. "fr-FR" -> "fr").
  const base = (code ?? "").slice(0, 2).toLowerCase();
  return base in CATALOGUES ? (base as LocaleCode) : "en";
}

/**
 * The active language. Reads the reactive settings store, so any `t()` call in a
 * component template re-runs when the user changes languages.
 */
export function currentLocale(): LocaleCode {
  return normalize(settingsStore.settings.language);
}

function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name) =>
    name in params ? String(params[name]) : match
  );
}

/**
 * Translate a key. Falls back to English, then to the raw key, so a missing
 * translation degrades gracefully instead of showing nothing.
 */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const locale = currentLocale();
  const message = CATALOGUES[locale][key] ?? en[key] ?? key;
  return interpolate(message, params);
}
