import { useCallback } from "react";
import { create } from "zustand";
import type { Language, Settings, ThemeName } from "../types";
import {
  resolveLanguage,
  translate,
  type TransVars,
  type TranslationKey,
} from "../i18n";
import { isTauri, setSettings, setTrayLanguage } from "../lib/ipc";
import { applyTheme } from "../lib/theme";

interface SettingsState {
  theme: ThemeName;
  /** Stored language; `null` = follow the OS. */
  language: Language | null;
  /** Concrete language actually used for rendering. */
  resolvedLanguage: Language;
  defaultFps: number;
  defaultCursor: boolean;
  /** Folders the last export / save used; dialogs reopen there (null = unset). */
  lastExportDir: string | null;
  lastProjectDir: string | null;
  /** True once settings have been loaded (from disk or onboarding). */
  hydrated: boolean;

  /** Apply persisted settings + theme without writing back to disk. */
  hydrate: (s: Settings) => void;
  /** Merge a change, apply the theme, and persist the full settings to disk. */
  update: (patch: Partial<Settings>) => void;
}

function toSettings(s: SettingsState): Settings {
  return {
    theme: s.theme,
    language: s.language,
    defaultFps: s.defaultFps,
    defaultCursor: s.defaultCursor,
    lastExportDir: s.lastExportDir,
    lastProjectDir: s.lastProjectDir,
  };
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  theme: "dark",
  language: null,
  resolvedLanguage: resolveLanguage(null),
  defaultFps: 30,
  defaultCursor: true,
  lastExportDir: null,
  lastProjectDir: null,
  hydrated: false,

  hydrate: (s) => {
    applyTheme(s.theme);
    const resolvedLanguage = resolveLanguage(s.language);
    set({
      theme: s.theme,
      language: s.language,
      resolvedLanguage,
      defaultFps: s.defaultFps,
      defaultCursor: s.defaultCursor,
      lastExportDir: s.lastExportDir,
      lastProjectDir: s.lastProjectDir,
      hydrated: true,
    });
    if (isTauri) {
      void setTrayLanguage(resolvedLanguage).catch((e) =>
        console.warn("tray relabel failed", e),
      );
    }
  },

  update: (patch) => {
    if (patch.theme) applyTheme(patch.theme);
    const resolvedLanguage =
      "language" in patch
        ? resolveLanguage(patch.language ?? null)
        : get().resolvedLanguage;
    set({ ...patch, resolvedLanguage });
    if (isTauri) {
      void setSettings(toSettings(get())).catch(() => {
        // Persisting is best-effort; the in-memory state already updated.
      });
      if ("language" in patch) {
        void setTrayLanguage(resolvedLanguage).catch((e) =>
          console.warn("tray relabel failed", e),
        );
      }
    }
  },
}));

/**
 * Translation hook. Subscribes to `resolvedLanguage` so every component using
 * it re-renders instantly when the language changes.
 */
export function useT() {
  const lang = useSettingsStore((s) => s.resolvedLanguage);
  return useCallback(
    (key: TranslationKey, vars?: TransVars) => translate(lang, key, vars),
    [lang],
  );
}
