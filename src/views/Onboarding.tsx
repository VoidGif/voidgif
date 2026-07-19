import { useCallback, useEffect, useState } from "react";
import type { Language, ThemeName } from "../types";
import { useSettingsStore } from "../stores/settingsStore";
import { applyTheme } from "../lib/theme";
import { resolveLanguage, translate, type TransVars, type TranslationKey } from "../i18n";
import { ThemePicker, LanguagePicker } from "../components/pickers";

/**
 * First-run setup: pick a theme (applied live) and a language (previewed live),
 * then Continue persists via set_settings and hands off to Home. Uses a
 * self-contained `t` so the picker copy re-renders as the language changes,
 * without writing anything to disk before Continue.
 */
export default function Onboarding({ onDone }: { onDone: () => void }) {
  const update = useSettingsStore((s) => s.update);
  const [theme, setTheme] = useState<ThemeName>("dark");
  const [language, setLanguage] = useState<Language | null>(null);

  const t = useCallback(
    (key: TranslationKey, vars?: TransVars) =>
      translate(resolveLanguage(language), key, vars),
    [language],
  );

  // A stale localStorage theme cache (from before the settings file was
  // removed) may have painted the app light while "Dark" is selected here —
  // sync the visible theme to the actual selection on mount.
  useEffect(() => {
    applyTheme(theme);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pickTheme = (next: ThemeName) => {
    setTheme(next);
    applyTheme(next); // live preview
  };

  const finish = () => {
    update({ theme, language });
    onDone();
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-8 overflow-y-auto p-8">
      <div className="vg-modal flex flex-col items-center gap-2.5">
        <div className="text-4xl font-semibold tracking-tight text-ink-1">
          <span className="bg-gradient-to-r from-accent-400 to-fuchsia-400 bg-clip-text text-transparent">
            Void
          </span>
          Gif
        </div>
        <p className="text-sm text-ink-2">{t("onboardingSubtitle")}</p>
      </div>

      <div className="vg-modal w-full max-w-md rounded-2xl border border-line bg-void-900 p-6 shadow-xl shadow-black/20">
        <div className="mb-6">
          <div className="mb-2.5 text-sm font-medium text-ink-2">{t("theme")}</div>
          <ThemePicker value={theme} onChange={pickTheme} t={t} />
        </div>

        <div>
          <div className="mb-2.5 text-sm font-medium text-ink-2">{t("language")}</div>
          <LanguagePicker value={language} onChange={setLanguage} t={t} />
        </div>
      </div>

      <button
        onClick={finish}
        className="vg-modal rounded-2xl bg-gradient-to-b from-accent-500 to-accent-600 px-10 py-3 text-lg font-medium text-white shadow-lg shadow-accent-600/30 ring-1 ring-inset ring-white/15 vg-transition hover:from-accent-400 hover:to-accent-500 active:scale-[0.98]"
      >
        {t("continueButton")}
      </button>
    </div>
  );
}
