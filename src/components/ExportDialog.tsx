import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore, useT } from "../stores/settingsStore";
import { dirOf, joinDir } from "../lib/lastDir";
import {
  cancelExport,
  copyFileToClipboard,
  estimateGifSize,
  isTauri,
  onExportFitResult,
  revealInExplorer,
  startExport,
  startExportFit,
} from "../lib/ipc";
import type { ExportFormat, ExportProgress, FitResult, GifExportSettings } from "../types";
import type { TranslationKey } from "../i18n";

const IS_MAC = /mac/i.test(navigator.userAgent);

const FORMATS: {
  id: ExportFormat;
  label: string;
  ext: string;
  noteKey: TranslationKey;
}[] = [
  { id: "gif", label: "GIF", ext: "gif", noteKey: "noteGifski" },
  { id: "apng", label: "APNG", ext: "png", noteKey: "noteApng" },
  { id: "png-seq", label: "PNG seq", ext: "png", noteKey: "notePngSeq" },
  // MP4 rides on Windows Media Foundation; macOS gets AVFoundation later.
  ...(IS_MAC
    ? []
    : [{ id: "mp4" as ExportFormat, label: "MP4", ext: "mp4", noteKey: "noteMp4" as TranslationKey }]),
];

const MAX_WIDTH = 7680;
const MB = 1024 * 1024;

// Platform share-size budgets (brand labels stay untranslated).
const PRESETS: { id: string; label: string; bytes: number }[] = [
  { id: "github", label: "GitHub", bytes: 10 * MB },
  { id: "discord", label: "Discord", bytes: 8 * MB },
  { id: "slack", label: "Slack", bytes: 5 * MB },
  { id: "x", label: "X", bytes: 15 * MB },
];

const STAGE_KEY: Record<ExportProgress["stage"], TranslationKey> = {
  collecting: "stageCollecting",
  estimating: "stageEstimating",
  encoding: "stageEncoding",
  writing: "stageWriting",
  done: "stageDone",
  error: "stageError",
};

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < MB) return `${Math.round(n / 1024)} KB`;
  return `${(n / MB).toFixed(1)} MB`;
}

export default function ExportDialog({ onClose }: { onClose: () => void }) {
  const t = useT();
  const session = useAppStore((s) => s.session);
  const progress = useAppStore((s) => s.exportProgress);
  const setExportProgress = useAppStore((s) => s.setExportProgress);

  const [format, setFormat] = useState<ExportFormat>("gif");
  const [quality, setQuality] = useState(90);
  const [width, setWidth] = useState<string>("");
  const [fast, setFast] = useState(false);
  const [preset, setPreset] = useState<string | null>(null);
  const [estimate, setEstimate] = useState<number | null>(null);
  const [estimating, setEstimating] = useState(false);
  const [fitResult, setFitResult] = useState<FitResult | null>(null);
  const [copied, setCopied] = useState(false);
  const [shareError, setShareError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Synchronous re-entry guard: `exporting` only flips after the async save
  // dialog resolves, which is far too late to stop a double click.
  const launching = useRef(false);
  const [busy, setBusy] = useState(false);
  const copyTimer = useRef(0);

  const exporting =
    busy ||
    (progress != null && progress.stage !== "done" && progress.stage !== "error");

  const activePreset = PRESETS.find((p) => p.id === preset) ?? null;
  const useFit = format === "gif" && activePreset != null;

  const gifSettings = (): GifExportSettings => ({
    format: "gif",
    path: "",
    quality,
    width: width ? Math.min(parseInt(width, 10), MAX_WIDTH) : null,
    loop: true,
    fast,
  });

  // Live size estimate: debounced 500ms after any GIF setting changes (and on
  // open). A concurrent-estimate "busy" rejection keeps the last value.
  useEffect(() => {
    if (format !== "gif" || !session || !isTauri || exporting) {
      setEstimate(null);
      setEstimating(false);
      return;
    }
    let cancelled = false;
    setEstimating(true);
    const handle = window.setTimeout(() => {
      void estimateGifSize(gifSettings())
        .then((bytes) => {
          if (!cancelled) {
            setEstimate(bytes);
            setEstimating(false);
          }
        })
        .catch(() => {
          if (!cancelled) setEstimating(false);
        });
    }, 500);
    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [format, quality, width, fast, session, exporting]);

  // The chosen settings from a target-fit export arrive out of band.
  useEffect(() => {
    const un = onExportFitResult((r) => setFitResult(r));
    return () => {
      void un.then((f) => f());
    };
  }, []);

  useEffect(() => () => window.clearTimeout(copyTimer.current), []);

  const begin = async () => {
    if (launching.current || exporting) return;
    launching.current = true;
    setBusy(true);
    setError(null);
    setShareError(null);
    setFitResult(null);
    try {
      const meta = FORMATS.find((f) => f.id === format)!;
      const lastDir = useSettingsStore.getState().lastExportDir;
      const path = await save({
        defaultPath: lastDir
          ? joinDir(lastDir, `recording.${meta.ext}`)
          : `recording.${meta.ext}`,
        filters: [{ name: meta.label, extensions: [meta.ext] }],
      });
      if (!path) return;
      // Remember the folder as soon as it's chosen — the choice is real even if
      // the export below fails.
      const dir = dirOf(path);
      if (dir) useSettingsStore.getState().update({ lastExportDir: dir });
      setExportProgress({ current: 0, total: 1, stage: "collecting" });
      if (useFit && activePreset) {
        await startExportFit({ ...gifSettings(), path }, activePreset.bytes);
      } else if (format === "gif") {
        await startExport({ ...gifSettings(), path });
      } else {
        await startExport({
          format,
          path,
          quality,
          width: width ? Math.min(parseInt(width, 10), MAX_WIDTH) : null,
          loop: true,
          fast,
        });
      }
    } catch (e) {
      // Leave any in-flight export's progress alone — only surface the error.
      setError(String(e));
    } finally {
      launching.current = false;
      setBusy(false);
    }
  };

  const doCopy = async (path?: string) => {
    if (!path) return;
    setShareError(null);
    try {
      await copyFileToClipboard(path);
      setCopied(true);
      window.clearTimeout(copyTimer.current);
      copyTimer.current = window.setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      setShareError(String(e));
    }
  };

  const doReveal = async (path?: string) => {
    if (!path) return;
    setShareError(null);
    try {
      await revealInExplorer(path);
    } catch (e) {
      setShareError(String(e));
    }
  };

  const pct =
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : 0;

  const donePath = progress?.stage === "done" ? progress.message : undefined;

  return (
    <div className="vg-scrim fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="vg-modal w-[420px] rounded-2xl border border-line-strong bg-void-900 p-6 shadow-2xl shadow-black/40">
        <h2 className="mb-5 text-lg font-semibold text-ink-1">{t("export")}</h2>

        <div className="mb-4 grid grid-cols-2 gap-2">
          {FORMATS.map((f) => (
            <button
              key={f.id}
              onClick={() => setFormat(f.id)}
              disabled={exporting}
              className={`rounded-xl border px-3 py-2 text-left vg-transition ${
                format === f.id
                  ? "border-accent-500 bg-accent-600/15 shadow-sm shadow-accent-600/10"
                  : "border-line-strong bg-void-800 hover:border-ink-3"
              }`}
            >
              <div className="text-sm font-medium text-ink-1">{f.label}</div>
              <div className="text-[11px] text-ink-2">{t(f.noteKey)}</div>
            </button>
          ))}
        </div>

        {format === "gif" && (
          <>
            {/* Platform target-size presets */}
            <div className="mb-3 flex flex-wrap gap-1.5">
              {PRESETS.map((p) => {
                const on = preset === p.id;
                return (
                  <button
                    key={p.id}
                    disabled={exporting}
                    onClick={() => setPreset(on ? null : p.id)}
                    className={`rounded-full border px-2.5 py-1 text-xs font-medium vg-transition disabled:opacity-40 ${
                      on
                        ? "border-accent-500 bg-accent-600/20 text-accent-200"
                        : "border-line-strong bg-void-800 text-ink-2 hover:border-ink-3"
                    }`}
                  >
                    {p.label} <span className="text-ink-3">{p.bytes / MB}MB</span>
                  </button>
                );
              })}
            </div>

            <label className="mb-3 block text-sm text-ink-2">
              {t("quality")} <span className="font-mono text-accent-400">{quality}</span>
              <input
                type="range"
                min={1}
                max={100}
                value={quality}
                disabled={exporting}
                onChange={(e) => setQuality(Number(e.target.value))}
                className="mt-1 w-full accent-violet-500"
              />
            </label>
            <label className="mb-3 flex items-center gap-2 text-sm text-ink-2">
              <input
                type="checkbox"
                checked={fast}
                disabled={exporting}
                onChange={(e) => setFast(e.target.checked)}
                className="size-4 accent-violet-600"
              />
              {t("fastMode")}
            </label>
            <label className="mb-3 block text-sm text-ink-2">
              {t("widthLabel", { size: session?.width ?? "source" })}
              <input
                value={width}
                disabled={exporting}
                maxLength={4}
                onChange={(e) => setWidth(e.target.value.replace(/\D/g, ""))}
                placeholder={t("widthPlaceholder")}
                className="mt-1 w-full rounded-lg border border-line-strong bg-void-800 px-3 py-2 text-sm text-ink-1 outline-none focus:border-accent-500"
              />
            </label>

            {/* Live estimated size (+ target budget when a preset is picked) */}
            <div className="mb-4 flex items-center gap-2 text-sm">
              <span className="text-ink-2">
                {t("estSize")}:{" "}
                {estimate != null ? (
                  <span className="font-mono text-ink-1">~{formatBytes(estimate)}</span>
                ) : (
                  <span className="text-ink-3">—</span>
                )}
              </span>
              {activePreset && estimate != null && (
                <span
                  className={`font-mono ${
                    estimate <= activePreset.bytes ? "text-emerald-400" : "text-rose-400"
                  }`}
                >
                  / {activePreset.bytes / MB} MB {estimate <= activePreset.bytes ? "✓" : "✕"}
                </span>
              )}
              {estimating && (
                <span
                  aria-label={t("estimatingSize")}
                  className="inline-block size-3 animate-spin rounded-full border-2 border-ink-3 border-t-transparent motion-reduce:animate-none"
                />
              )}
            </div>
          </>
        )}
        {format !== "gif" && (
          <p className="mb-4 text-xs text-ink-3">
            {t("exportSourceSizeNote", {
              w: session?.width ?? 0,
              h: session?.height ?? 0,
            })}
          </p>
        )}

        {progress && (
          <div className="mb-4">
            <div className="mb-1 flex justify-between text-xs text-ink-2">
              <span>
                {progress.stage === "error"
                  ? (progress.message ?? t("exportFailed"))
                  : t(STAGE_KEY[progress.stage])}
              </span>
              <span className="font-mono">{pct}%</span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-void-700">
              <div
                className={`h-full transition-all ${progress.stage === "error" ? "bg-rose-500" : "bg-accent-500"}`}
                style={{ width: `${pct}%` }}
              />
            </div>
            {progress.stage === "done" && (
              <div className="mt-2 space-y-2">
                <p className="text-xs text-emerald-400">
                  {t("savedMsg", { message: progress.message ?? "" })}
                </p>
                {fitResult && (
                  <p className={`text-xs ${fitResult.over ? "text-amber-400" : "text-ink-2"}`}>
                    {fitResult.over
                      ? t("fitOverTarget", {
                          target: `${Math.round(fitResult.targetBytes / MB)} MB`,
                          size: formatBytes(fitResult.bytes),
                        })
                      : t("fitChosen", {
                          quality: fitResult.quality,
                          width: fitResult.width,
                          size: formatBytes(fitResult.bytes),
                        })}
                  </p>
                )}
                {/* PNG-sequence writes numbered siblings, so its "path" isn't a
                    single file to share — only offer sharing for one-file formats. */}
                {format !== "png-seq" && (
                  <div className="flex gap-2">
                    <button
                      onClick={() => void doCopy(donePath)}
                      className="rounded-lg border border-line-strong bg-void-800 px-3 py-1.5 text-xs font-medium text-ink-1 vg-transition hover:border-ink-3"
                    >
                      {copied ? t("copiedToClipboard") : t("copyToClipboard")}
                    </button>
                    <button
                      onClick={() => void doReveal(donePath)}
                      className="rounded-lg border border-line-strong bg-void-800 px-3 py-1.5 text-xs font-medium text-ink-1 vg-transition hover:border-ink-3"
                    >
                      {t("revealInFolder")}
                    </button>
                  </div>
                )}
                {shareError && <p className="text-xs text-rose-400">{shareError}</p>}
              </div>
            )}
          </div>
        )}
        {error && <p className="mb-3 text-xs text-rose-400">{error}</p>}

        <div className="mt-6 flex justify-end gap-2">
          <button
            onClick={() => {
              if (exporting) void cancelExport();
              setExportProgress(null);
              onClose();
            }}
            className="rounded-lg px-4 py-2 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
          >
            {exporting ? t("cancelExport") : t("close")}
          </button>
          <button
            onClick={() => void begin()}
            disabled={exporting || !session}
            className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.98] disabled:opacity-40"
          >
            {exporting ? t("exporting") : useFit ? t("fitExport") : t("export")}
          </button>
        </div>
      </div>
    </div>
  );
}
