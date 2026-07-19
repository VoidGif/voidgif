import type { Language, ThemeName } from "../types";
import type { TransVars, TranslationKey } from "../i18n";

/** Translation function shape, so pickers work with either a live-preview `t`
 *  (onboarding) or the store-bound `t` (settings dialog). */
type TFn = (key: TranslationKey, vars?: TransVars) => string;

const LANGUAGES: { value: Language | null; native: string }[] = [
  { value: null, native: "" }, // label comes from t("languageSystem")
  { value: "ko", native: "한국어" },
  { value: "ja", native: "日本語" },
  { value: "en", native: "English" },
];

/** Fixed-color mini app mockups so each card previews its own theme regardless
 *  of the currently active theme — a tiny VoidGif: titlebar, hero wordmark,
 *  record CTA, option pills and a filmstrip with the current-frame ring. */
function ThemeSwatch({ theme }: { theme: ThemeName }) {
  const c =
    theme === "dark"
      ? {
          bg: "#0e0e14",
          panel: "#191923",
          ink: "#e7e7ee",
          dim: "#4b4b58",
          line: "rgba(255,255,255,0.08)",
        }
      : {
          bg: "#f6f6f8",
          panel: "#ffffff",
          ink: "#1a1a22",
          dim: "#c6c6d0",
          line: "rgba(0,0,0,0.08)",
        };
  return (
    <div
      className="overflow-hidden rounded-lg border"
      style={{ background: c.bg, borderColor: c.line }}
    >
      {/* titlebar */}
      <div
        className="flex items-center gap-1 border-b px-2 py-1.5"
        style={{ borderColor: c.line }}
      >
        <div className="size-1 rounded-full" style={{ background: c.dim }} />
        <div className="size-1 rounded-full" style={{ background: c.dim }} />
        <div className="size-1 rounded-full" style={{ background: c.dim }} />
        <div className="ml-auto size-1.5 rounded-full" style={{ background: c.dim }} />
      </div>
      {/* hero: wordmark + record CTA */}
      <div className="flex flex-col items-center gap-1.5 px-2 pb-1.5 pt-2">
        <div
          className="h-1.5 w-10 rounded-full"
          style={{ background: "linear-gradient(90deg,#a78bfa,#e879f9)" }}
        />
        <div
          className="flex h-3.5 w-16 items-center justify-center gap-1 rounded-full"
          style={{ background: "#7c3aed" }}
        >
          <div className="size-1 rounded-full" style={{ background: "#fda4af" }} />
          <div className="h-1 w-8 rounded-full" style={{ background: "rgba(255,255,255,0.85)" }} />
        </div>
      </div>
      {/* option card: fps pills */}
      <div className="mx-2 mb-1.5 flex items-center gap-1 rounded-md p-1.5" style={{ background: c.panel }}>
        <div className="h-1.5 w-4 rounded-sm" style={{ background: "#7c3aed" }} />
        <div className="h-1.5 w-4 rounded-sm" style={{ background: c.dim }} />
        <div className="h-1.5 w-4 rounded-sm" style={{ background: c.dim }} />
        <div className="ml-auto h-1.5 w-6 rounded-sm" style={{ background: c.ink, opacity: 0.6 }} />
      </div>
      {/* filmstrip with current-frame ring */}
      <div className="flex gap-1 px-2 pb-2">
        {[0, 1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className="h-2.5 flex-1 rounded-sm"
            style={
              i === 1
                ? { background: c.panel, boxShadow: "0 0 0 1px #a78bfa" }
                : { background: c.panel, border: `1px solid ${c.line}` }
            }
          />
        ))}
      </div>
    </div>
  );
}

export function ThemePicker({
  value,
  onChange,
  t,
}: {
  value: ThemeName;
  onChange: (theme: ThemeName) => void;
  t: TFn;
}) {
  return (
    <div className="grid grid-cols-2 gap-3">
      {(["dark", "light"] as ThemeName[]).map((theme) => (
        <button
          key={theme}
          type="button"
          onClick={() => onChange(theme)}
          className={`rounded-xl border-2 p-1 text-left vg-transition ${
            value === theme
              ? "border-accent-500 shadow-md shadow-accent-600/15"
              : "border-line-strong hover:border-ink-3"
          }`}
        >
          <ThemeSwatch theme={theme} />
          <div className="px-1 py-1.5 text-sm font-medium text-ink-1">
            {t(theme === "dark" ? "themeDark" : "themeLight")}
          </div>
        </button>
      ))}
    </div>
  );
}

export function LanguagePicker({
  value,
  onChange,
  t,
}: {
  value: Language | null;
  onChange: (language: Language | null) => void;
  t: TFn;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      {LANGUAGES.map((lang) => {
        const selected = value === lang.value;
        return (
          <button
            key={lang.value ?? "system"}
            type="button"
            onClick={() => onChange(lang.value)}
            className={`flex items-center justify-between rounded-lg border px-3 py-2 text-sm vg-transition ${
              selected
                ? "border-accent-500 bg-accent-600/15 text-ink-1"
                : "border-line-strong text-ink-2 hover:bg-void-800 hover:text-ink-1"
            }`}
          >
            <span>{lang.value === null ? t("languageSystem") : lang.native}</span>
            {selected && <span className="text-accent-400">✓</span>}
          </button>
        );
      })}
    </div>
  );
}
