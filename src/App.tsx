import { useEffect, useState } from "react";
import { useAppStore } from "./stores/appStore";
import { useSettingsStore, useT } from "./stores/settingsStore";
import {
  autosaveNow,
  checkAutosave,
  discardAutosave,
  getSession,
  getSettings,
  isTauri,
  onExportProgress,
  onRecordingDiscarded,
  onRecordingStarted,
  onRecordingStats,
  onRecordingStopped,
  restoreAutosave,
  setTrayLanguage,
} from "./lib/ipc";
import type { AutosaveInfo } from "./types";
import Home from "./views/Home";
import Editor from "./views/Editor";
import Onboarding from "./views/Onboarding";

export default function App() {
  const view = useAppStore((s) => s.view);
  const setView = useAppStore((s) => s.setView);
  const setSession = useAppStore((s) => s.setSession);
  const setStats = useAppStore((s) => s.setStats);
  const setRecorderPhase = useAppStore((s) => s.setRecorderPhase);
  const setExportProgress = useAppStore((s) => s.setExportProgress);
  const hydrate = useSettingsStore((s) => s.hydrate);

  // `booted` gates the first paint until settings load, so Home never flashes
  // before onboarding. `onboarding` is set when no settings file exists yet.
  const [booted, setBooted] = useState(false);
  const [onboarding, setOnboarding] = useState(false);
  // Pending crash-recovery snapshot (from a previous kill/crash), if any.
  const [recovery, setRecovery] = useState<AutosaveInfo | null>(null);

  useEffect(() => {
    if (!isTauri) {
      setBooted(true);
      return;
    }
    void getSettings()
      .then((settings) => {
        if (settings) hydrate(settings);
        else {
          // First run: no saved language yet, but the tray (built with its
          // English fallback) should already follow the OS language.
          setOnboarding(true);
          void setTrayLanguage(
            useSettingsStore.getState().resolvedLanguage,
          ).catch((e) => console.warn("tray relabel failed", e));
        }
      })
      .catch(() => {})
      .finally(() => setBooted(true));
  }, [hydrate]);

  useEffect(() => {
    if (!isTauri) return;
    // A session may already exist (opened via CLI / file association).
    void getSession().then((s) => {
      if (s) {
        setSession(s, { pixelsChanged: true });
        setView("editor");
      }
    });
    const subs = [
      onRecordingStarted(() => setRecorderPhase("recording")),
      onRecordingStats(setStats),
      onRecordingStopped((session) => {
        setRecorderPhase("idle");
        setSession(session, { pixelsChanged: true });
        setView("editor");
      }),
      onRecordingDiscarded(() => {
        setRecorderPhase("idle");
      }),
      onExportProgress(setExportProgress),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [setExportProgress, setRecorderPhase, setSession, setStats, setView]);

  // Offer crash recovery once settings are up (this component only ever renders
  // in the main window). First run has no autosave, so skip during onboarding.
  useEffect(() => {
    if (!isTauri || !booted || onboarding) return;
    let cancelled = false;
    void checkAutosave()
      .then((info) => {
        if (!cancelled && info) setRecovery(info);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [booted, onboarding]);

  // Autosave loop: every 45s, snapshot the session if it's dirty and idle.
  // Recording is skipped (the recorder's session isn't in state.session anyway).
  useEffect(() => {
    if (!isTauri) return;
    const id = window.setInterval(() => {
      const st = useAppStore.getState();
      if (!st.dirty || !st.session) return;
      if (st.recorderPhase === "recording" || st.recorderPhase === "paused") return;
      void autosaveNow()
        .then((written) => {
          if (written) useAppStore.getState().clearDirty();
        })
        .catch(() => {});
    }, 45_000);
    return () => window.clearInterval(id);
  }, []);

  const doRecover = async () => {
    try {
      const s = await restoreAutosave();
      setSession(s, { pixelsChanged: true });
      setView("editor");
    } catch {
      // The snapshot was unreadable; nothing to recover into.
    }
    setRecovery(null);
  };

  const doDiscardRecovery = async () => {
    try {
      await discardAutosave();
    } catch {
      // Best-effort — the file may already be gone.
    }
    setRecovery(null);
  };

  if (!booted) return null;
  if (onboarding) return <Onboarding onDone={() => setOnboarding(false)} />;
  return (
    <>
      {view === "editor" ? <Editor /> : <Home />}
      {recovery && (
        <RecoveryModal
          info={recovery}
          onRecover={() => void doRecover()}
          onDiscard={() => void doDiscardRecovery()}
        />
      )}
    </>
  );
}

/** Crash-recovery prompt shown when an autosave snapshot survived a hard exit. */
function RecoveryModal({
  info,
  onRecover,
  onDiscard,
}: {
  info: AutosaveInfo;
  onRecover: () => void;
  onDiscard: () => void;
}) {
  const t = useT();
  const lang = useSettingsStore((s) => s.resolvedLanguage);
  const when = (() => {
    const d = new Date(info.savedAt);
    return Number.isNaN(d.getTime()) ? info.savedAt : d.toLocaleString(lang);
  })();
  return (
    <div className="vg-scrim fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4">
      <div className="vg-modal w-[380px] rounded-2xl border border-line-strong bg-void-900 p-6 shadow-2xl shadow-black/40">
        <h2 className="mb-2 text-lg font-semibold text-ink-1">{t("recoverTitle")}</h2>
        <p className="mb-5 text-sm text-ink-2">
          {t("recoverBody", { count: info.frames, time: when })}
        </p>
        <div className="flex justify-end gap-2">
          <button
            onClick={onDiscard}
            className="rounded-lg px-4 py-2 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
          >
            {t("recoverDiscard")}
          </button>
          <button
            onClick={onRecover}
            className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.98]"
          >
            {t("recoverAction")}
          </button>
        </div>
      </div>
    </div>
  );
}
