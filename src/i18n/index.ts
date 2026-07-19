import type { Language } from "../types";
import { en, type TranslationKey } from "./en";
import { ko } from "./ko";
import { ja } from "./ja";

export type { TranslationKey };

const DICTS: Record<Language, Record<TranslationKey, string>> = { en, ko, ja };

/** Values allowed in `{token}` interpolation. */
export type TransVars = Record<string, string | number>;

/**
 * Resolves a stored language (`null` = follow OS) to a concrete UI language.
 * Only Korean and Japanese are special-cased; everything else falls back to
 * English.
 */
export function resolveLanguage(language: Language | null): Language {
  if (language) return language;
  const nav = (typeof navigator !== "undefined" && navigator.language) || "en";
  if (nav.startsWith("ko")) return "ko";
  if (nav.startsWith("ja")) return "ja";
  return "en";
}

/**
 * Looks up `key` in `lang`'s dictionary (falling back to English, then the raw
 * key) and fills any `{name}` tokens from `vars`.
 */
export function translate(
  lang: Language,
  key: TranslationKey,
  vars?: TransVars,
): string {
  let s: string = DICTS[lang][key] ?? en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
