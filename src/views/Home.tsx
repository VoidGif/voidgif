import { useEffect, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore, useT } from "../stores/settingsStore";
import { importGif, isTauri, loadProject, openRecorder } from "../lib/ipc";
import { open } from "@tauri-apps/plugin-dialog";
import { promptSaveProject } from "../lib/saveFlow";
import SettingsDialog from "../components/SettingsDialog";
import { IconGear } from "../components/icons";

const FPS_CHOICES = [15, 24, 30, 60];

export default function Home() {
  const t = useT();
  const setView = useAppStore((s) => s.setView);
  const setSession = useAppStore((s) => s.setSession);
  const session = useAppStore((s) => s.session);
  const unsaved = useAppStore((s) => s.unsaved);
  const markSaved = useAppStore((s) => s.markSaved);
  const defaultFps = useSettingsStore((s) => s.defaultFps);
  const defaultCursor = useSettingsStore((s) => s.defaultCursor);

  const [fps, setFps] = useState(defaultFps);
  const [cursor, setCursor] = useState(defaultCursor);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  // Set when the record / open action needs the unsaved-work confirmation.
  const [pending, setPending] = useState<"record" | "open" | null>(null);

  // Follow the persisted defaults if they change in Settings (still adjustable
  // per-recording afterwards).
  useEffect(() => setFps(defaultFps), [defaultFps]);
  useEffect(() => setCursor(defaultCursor), [defaultCursor]);

  useEffect(() => {
    if (!pending) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPending(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pending]);

  const beginSelection = async () => {
    setError(null);
    try {
      await openRecorder(fps, cursor);
    } catch (e) {
      setError(String(e));
    }
  };

  const openProject = async () => {
    setError(null);
    setBusy(true);
    try {
      const path = await open({
        multiple: false,
        filters: [
          { name: "Project or GIF", extensions: ["voidgif", "gif"] },
          { name: "VoidGif project", extensions: ["voidgif"] },
          { name: "GIF image", extensions: ["gif"] },
        ],
      });
      if (typeof path === "string") {
        // Route by extension: GIFs are composited into a new session, .voidgif
        // projects load directly.
        const session = /\.gif$/i.test(path)
          ? await importGif(path)
          : await loadProject(path);
        setSession(session, { pixelsChanged: true });
        // Freshly loaded content matches its file on disk.
        markSaved();
        setView("editor");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runAction = (action: "record" | "open") =>
    action === "record" ? void beginSelection() : void openProject();

  // Both actions end up replacing the current session — ask before losing
  // work that was never written to a .voidgif file.
  const guarded = (action: "record" | "open") => {
    if (session && unsaved) setPending(action);
    else runAction(action);
  };

  const confirmContinue = () => {
    const action = pending;
    setPending(null);
    if (action) runAction(action);
  };

  const confirmSaveThenContinue = async () => {
    const result = await promptSaveProject();
    if (result === "saved") confirmContinue();
    else if (result === "error") {
      setPending(null);
      setError(t("saveFailed"));
    }
    // "cancelled": keep the confirmation open so nothing is lost silently.
  };

  return (
    <div className="relative flex h-full flex-col items-center justify-center gap-8 p-8">
      <button
        onClick={() => setShowSettings(true)}
        title={t("settingsTitle")}
        aria-label={t("settingsTitle")}
        className="absolute right-4 top-4 rounded-lg p-2 text-ink-3 vg-transition hover:bg-void-800 hover:text-ink-1"
      >
        <IconGear size={20} />
      </button>

      <div className="flex flex-col items-center gap-2.5">
        <div className="text-5xl font-semibold tracking-tight text-ink-1">
          <span className="bg-gradient-to-r from-accent-400 to-fuchsia-400 bg-clip-text text-transparent">
            Void
          </span>
          Gif
        </div>
        <p className="text-sm text-ink-2">{t("appTagline")}</p>
      </div>

      <button
        onClick={() => guarded("record")}
        disabled={!isTauri || busy}
        className="group relative rounded-2xl bg-gradient-to-b from-accent-500 to-accent-600 px-10 py-4 text-lg font-medium text-white shadow-lg shadow-accent-600/30 ring-1 ring-inset ring-white/15 vg-transition hover:from-accent-400 hover:to-accent-500 hover:shadow-xl hover:shadow-accent-600/40 active:scale-[0.98] disabled:opacity-40"
      >
        <span className="mr-2.5 inline-block size-2.5 animate-pulse rounded-full bg-red-400 align-middle shadow-[0_0_8px_rgba(248,113,113,0.9)] motion-reduce:animate-none" />
        {t("selectRegionRecord")}
      </button>

      <div className="flex items-center gap-5 rounded-2xl border border-line bg-void-900 px-6 py-4 shadow-sm shadow-black/10">
        <label className="flex items-center gap-2.5 text-sm text-ink-2">
          {t("fps")}
          <div className="flex overflow-hidden rounded-lg border border-line-strong">
            {FPS_CHOICES.map((f) => (
              <button
                key={f}
                onClick={() => setFps(f)}
                className={`px-3 py-1.5 text-xs font-medium vg-transition ${
                  fps === f
                    ? "bg-accent-600 text-white"
                    : "bg-void-800 text-ink-2 hover:bg-void-700 hover:text-ink-1"
                }`}
              >
                {f}
              </button>
            ))}
          </div>
        </label>

        <div className="h-6 w-px bg-line-strong" />

        <label className="flex cursor-pointer items-center gap-2 text-sm text-ink-2">
          <input
            type="checkbox"
            checked={cursor}
            onChange={(e) => setCursor(e.target.checked)}
            className="size-4 accent-violet-600"
          />
          {t("captureCursor")}
        </label>
      </div>

      <div className="flex items-center gap-4 text-sm">
        {session && (
          <button
            onClick={() => setView("editor")}
            className="rounded-lg border border-accent-500/40 bg-accent-600/10 px-4 py-2 text-accent-400 vg-transition hover:bg-accent-600/20"
          >
            {t("continueEditing", { count: session.frames.length })}
          </button>
        )}
        <button
          onClick={() => guarded("open")}
          disabled={!isTauri || busy}
          className="rounded-lg border border-line-strong px-4 py-2 text-ink-2 vg-transition hover:bg-void-800 hover:text-ink-1 disabled:opacity-40"
        >
          {t("openFile")}
        </button>
        <span className="text-xs text-ink-2">{t("hotkeysHint")}</span>
      </div>

      {!isTauri && (
        <p className="text-xs text-amber-400/80">{t("browserPreviewNotice")}</p>
      )}
      {error && <p className="max-w-md text-xs text-rose-400">{error}</p>}

      {showSettings && <SettingsDialog onClose={() => setShowSettings(false)} />}

      {pending && session && (
        <div className="vg-scrim fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4">
          <div className="vg-modal w-[400px] rounded-2xl border border-line-strong bg-void-900 p-6 shadow-2xl shadow-black/40">
            <h2 className="mb-2 text-lg font-semibold text-ink-1">
              {t("unsavedTitle")}
            </h2>
            <p className="mb-5 text-sm text-ink-2">
              {t(pending === "record" ? "unsavedBodyRecord" : "unsavedBodyOpen", {
                count: session.frames.length,
              })}
            </p>
            <div className="flex flex-wrap justify-end gap-2">
              <button
                onClick={() => setPending(null)}
                className="rounded-lg px-4 py-2 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
              >
                {t("cancel")}
              </button>
              <button
                onClick={confirmContinue}
                className="rounded-lg border border-line-strong px-4 py-2 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
              >
                {t("unsavedContinue")}
              </button>
              <button
                onClick={() => void confirmSaveThenContinue()}
                className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.98]"
              >
                {t("unsavedSaveContinue")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
