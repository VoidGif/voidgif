import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useSettingsStore, useT } from "../stores/settingsStore";
import { isTauri, openExternal } from "../lib/ipc";
import { ThemePicker, LanguagePicker } from "./pickers";

const FPS_CHOICES = [15, 24, 30, 60];

// Fallback when getVersion() is unavailable (browser preview). Keep in sync
// with the version in tauri.conf.json / Cargo.toml.
const FALLBACK_VERSION = "0.1.0";

// About-section links: [open_external page key, i18n label key].
const ABOUT_LINKS = [
  ["source", "aboutSource"],
  ["notices", "aboutNotices"],
  ["privacy", "aboutPrivacy"],
  ["website", "aboutWebsite"],
] as const;

/**
 * Settings modal reachable from the Home gear button. Every control saves
 * immediately via the store's `update` (which persists with set_settings) —
 * there is no OK button, just Close.
 */
export default function SettingsDialog({ onClose }: { onClose: () => void }) {
  const t = useT();
  const theme = useSettingsStore((s) => s.theme);
  const language = useSettingsStore((s) => s.language);
  const defaultFps = useSettingsStore((s) => s.defaultFps);
  const defaultCursor = useSettingsStore((s) => s.defaultCursor);
  const update = useSettingsStore((s) => s.update);

  // getVersion() is permitted by core:default in capabilities/default.json;
  // browser preview has no Tauri, so it keeps the hardcoded fallback.
  const [version, setVersion] = useState(FALLBACK_VERSION);
  useEffect(() => {
    if (!isTauri) return;
    getVersion()
      .then(setVersion)
      .catch(() => {});
  }, []);

  return (
    <div
      className="vg-scrim fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onClose}
    >
      <div
        className="vg-modal max-h-full w-[420px] overflow-y-auto rounded-2xl border border-line-strong bg-void-900 p-6 shadow-2xl shadow-black/40"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-lg font-semibold text-ink-1">{t("settingsTitle")}</h2>
          <button
            onClick={onClose}
            className="rounded-lg px-3 py-1.5 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
          >
            {t("close")}
          </button>
        </div>

        <div className="mb-5">
          <div className="mb-2 text-sm font-medium text-ink-2">{t("theme")}</div>
          <ThemePicker value={theme} onChange={(th) => update({ theme: th })} t={t} />
        </div>

        <div className="mb-5">
          <div className="mb-2 text-sm font-medium text-ink-2">{t("language")}</div>
          <LanguagePicker
            value={language}
            onChange={(l) => update({ language: l })}
            t={t}
          />
        </div>

        <div className="mb-5">
          <div className="mb-2 text-sm font-medium text-ink-2">{t("defaultFps")}</div>
          <div className="flex w-max overflow-hidden rounded-lg border border-line-strong">
            {FPS_CHOICES.map((f) => (
              <button
                key={f}
                onClick={() => update({ defaultFps: f })}
                className={`px-4 py-1.5 text-sm font-medium vg-transition ${
                  defaultFps === f
                    ? "bg-accent-600 text-white"
                    : "bg-void-800 text-ink-2 hover:bg-void-700 hover:text-ink-1"
                }`}
              >
                {f}
              </button>
            ))}
          </div>
        </div>

        <label className="flex cursor-pointer items-center gap-2 text-sm text-ink-2">
          <input
            type="checkbox"
            checked={defaultCursor}
            onChange={(e) => update({ defaultCursor: e.target.checked })}
            className="size-4 accent-violet-600"
          />
          {t("captureCursorDefault")}
        </label>

        <div className="my-5 h-px bg-line-strong" />

        <div className="mb-2 text-sm font-medium text-ink-2">{t("aboutTitle")}</div>
        <p className="text-sm text-ink-1">VoidGif {version} · © 2026 VoidGif</p>
        <p className="mt-1 text-xs leading-relaxed text-ink-3">{t("aboutLicense")}</p>
        <div className="mt-3 flex flex-wrap gap-2">
          {ABOUT_LINKS.map(([page, key]) => (
            <button
              key={page}
              onClick={() => void openExternal(page).catch(() => {})}
              className="rounded-lg border border-line-strong bg-void-800 px-3 py-1.5 text-xs font-medium text-ink-1 vg-transition hover:border-ink-3"
            >
              {t(key)}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
