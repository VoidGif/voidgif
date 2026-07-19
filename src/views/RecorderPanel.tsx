import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelArmed,
  discardRecording,
  getRecorderState,
  getSettings,
  onPauseState,
  onRecordingStarted,
  onRecordingStats,
  pauseRecording,
  reportHoleRect,
  resumeRecording,
  startRecordingFromFrame,
  stopRecording,
  updateRecorderOptions,
} from "../lib/ipc";
import { IconLock, IconPause, IconPlay, IconStop, IconX } from "../components/icons";
import { useSettingsStore, useT } from "../stores/settingsStore";
import { resolveLanguage } from "../i18n";
import type { FrameInfo, RecordingStats } from "../types";

type Phase = "armed" | "recording" | "paused";

const FPS_CHOICES = [15, 24, 30, 60];

const fmt = (ms: number) => {
  const s = Math.floor(ms / 1000);
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
};

/**
 * ScreenToGif-style capture frame: a movable, resizable window whose
 * transparent center hole defines the capture region. Thin grab edges +
 * accent border around the hole, opaque control bar at the bottom.
 * Click-through inside the hole is handled by a Rust cursor poller fed by
 * reportHoleRect.
 */
export default function RecorderPanel() {
  const t = useT();
  const [phase, setPhase] = useState<Phase>("armed");
  const [fps, setFps] = useState(30);
  const [cursor, setCursor] = useState(true);
  const [fullScreen, setFullScreen] = useState(false);
  const [continueMode, setContinueMode] = useState(false);
  const [frameInfo, setFrameInfo] = useState<FrameInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<RecordingStats>({
    frameCount: 0,
    elapsedMs: 0,
    droppedFrames: 0,
  });
  // Discard is destructive — require a second click within 3 seconds.
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const confirmTimer = useRef(0);
  const holeRef = useRef<HTMLDivElement>(null);

  const report = useCallback(() => {
    const el = holeRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    reportHoleRect(r.x, r.y, r.width, r.height)
      .then(setFrameInfo)
      .catch(() => {});
  }, []);

  // The recorder is a separate webview with its own (unhydrated) store. Load
  // just the language so labels match the user's choice — but never the theme:
  // the recorder bar floats over arbitrary screen content and must stay dark.
  useEffect(() => {
    void getSettings()
      .then((s) => {
        if (s) {
          useSettingsStore.setState({
            language: s.language,
            resolvedLanguage: resolveLanguage(s.language),
          });
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    // The panel mounts after open_recorder seeds the armed setup, so the
    // backend snapshot is the source of truth (also covers webview remounts).
    void getRecorderState()
      .then((s) => {
        if (s.phase === "recording" || s.phase === "paused") {
          setPhase(s.phase);
        } else {
          setPhase("armed");
        }
        setFps(s.fps);
        setCursor(s.showCursor);
        setFullScreen(s.fullScreen);
        setContinueMode(s.continueMode);
      })
      .catch(() => {});

    report();
    const subs = [
      onRecordingStarted(() => setPhase("recording")),
      onRecordingStats(setStats),
      onPauseState((p) => setPhase(p ? "paused" : "recording")),
    ];

    // Re-report the hole whenever layout changes: window resize (observer)
    // or a move to another monitor (scale / monitor size may change).
    const ro = new ResizeObserver(() => report());
    if (holeRef.current) ro.observe(holeRef.current);
    let moveTimer = 0;
    let unMoved: (() => void) | undefined;
    void (async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      unMoved = await getCurrentWebviewWindow().onMoved(() => {
        if (moveTimer) return;
        moveTimer = window.setTimeout(() => {
          moveTimer = 0;
          report();
        }, 150);
      });
    })();

    return () => {
      subs.forEach((p) => p.then((un) => un()));
      ro.disconnect();
      unMoved?.();
      window.clearTimeout(moveTimer);
      window.clearTimeout(confirmTimer.current);
    };
  }, [report]);

  const pushOptions = (f: number, c: boolean, fs: boolean) => {
    updateRecorderOptions(f, c, fs).catch((e) => setError(String(e)));
  };

  const onRec = () => {
    setError(null);
    startRecordingFromFrame().catch((e) => setError(String(e)));
  };

  const onDiscardClick = () => {
    if (confirmDiscard) {
      window.clearTimeout(confirmTimer.current);
      void discardRecording();
      return;
    }
    setConfirmDiscard(true);
    confirmTimer.current = window.setTimeout(() => setConfirmDiscard(false), 3000);
  };

  const armed = phase === "armed";
  const paused = phase === "paused";
  const drag = armed ? { "data-tauri-drag-region": true } : {};
  // Drag affordance: every draggable chrome element shows a move cursor while
  // armed (buttons/inputs keep their own cursor via UA/explicit styles).
  const dragCursor = armed ? " cursor-move" : "";
  const sizeText = frameInfo
    ? fullScreen
      ? `${frameInfo.monitorWidth}×${frameInfo.monitorHeight}`
      : `${frameInfo.holeWidth}×${frameInfo.holeHeight}`
    : "—";

  return (
    <div className="flex h-full flex-col">
      {/* Grab edges (10px) around the transparent capture hole — draggable
          chrome while armed, and the OS resize-affordance boundary. */}
      <div {...drag} className={`h-2.5 shrink-0 bg-void-900/70${dragCursor}`} />
      <div className="flex min-h-0 flex-1">
        <div {...drag} className={`w-2.5 shrink-0 bg-void-900/70${dragCursor}`} />
        {/* The visible 2px border must itself carry the drag attribute:
            Tauri only starts a window drag when the mousedown target element
            has it. Clicks inside the hole hit the inner div (no attribute),
            so click-through via the Rust poller can't cause accidental drags. */}
        <div
          {...drag}
          className={`min-w-0 flex-1 border-2 ${
            armed ? "border-accent-500/90" : "border-red-500/90"
          }${dragCursor}`}
        >
          {/* The hole: fully transparent; its rect (css px) drives capture */}
          <div ref={holeRef} className="h-full w-full cursor-default" />
        </div>
        <div {...drag} className={`w-2.5 shrink-0 bg-void-900/70${dragCursor}`} />
      </div>

      {/* Control bar */}
      <div
        {...drag}
        className={`flex h-14 shrink-0 items-center gap-3 border-t border-white/10 bg-void-900 px-3${dragCursor}`}
      >
        {armed ? (
          <>
            <button
              onClick={onRec}
              title={t("startRecordingTitle")}
              aria-label={t("startRecordingTitle")}
              className="vg-rec"
            >
              <span className="vg-rec-core" />
            </button>

            <div className="mx-0.5 h-6 w-px shrink-0 bg-white/10" />

            <div className="flex shrink-0 overflow-hidden rounded-lg border border-white/10">
              {FPS_CHOICES.map((f) => (
                <button
                  key={f}
                  onClick={() => {
                    setFps(f);
                    pushOptions(f, cursor, fullScreen);
                  }}
                  className={`px-2 py-1 text-xs font-medium vg-transition ${
                    fps === f
                      ? "bg-accent-600 text-white"
                      : "bg-void-800 text-zinc-400 hover:bg-void-700 hover:text-zinc-200"
                  }`}
                >
                  {f}
                </button>
              ))}
            </div>

            <label className="flex shrink-0 cursor-pointer items-center gap-1.5 text-xs text-zinc-300">
              <input
                type="checkbox"
                checked={cursor}
                onChange={(e) => {
                  setCursor(e.target.checked);
                  pushOptions(fps, e.target.checked, fullScreen);
                }}
                className="size-3.5 accent-violet-600"
              />
              {t("cursor")}
            </label>

            {/* Full-screen is meaningless in continue mode — the hole is
                locked to the existing session's dimensions. */}
            {!continueMode && (
              <label className="flex shrink-0 cursor-pointer items-center gap-1.5 text-xs text-zinc-300">
                <input
                  type="checkbox"
                  checked={fullScreen}
                  onChange={(e) => {
                    setFullScreen(e.target.checked);
                    pushOptions(fps, cursor, e.target.checked);
                  }}
                  className="size-3.5 accent-violet-600"
                />
                {t("fullScreen")}
              </label>
            )}

            <div className="mx-0.5 h-6 w-px shrink-0 bg-white/10" />

            {continueMode ? (
              <span className="flex shrink-0 items-center gap-1 font-mono text-xs text-accent-400">
                <IconLock size={12} />
                {sizeText} px · {t("sizeLocked")}
              </span>
            ) : (
              <span className="shrink-0 font-mono text-xs text-zinc-400">{sizeText} px</span>
            )}

            {error ? (
              <span className="min-w-0 flex-1 truncate text-[10px] text-rose-400">{error}</span>
            ) : (
              <span className="min-w-0 flex-1 truncate text-right text-[10px] text-zinc-400">
                {t("recorderHint")}
              </span>
            )}

            <button
              onClick={() => void cancelArmed()}
              title={t("closeTitle")}
              className="shrink-0 rounded-lg px-2 py-1.5 text-zinc-400 vg-transition hover:bg-void-700 hover:text-rose-400"
            >
              <IconX size={14} />
            </button>
          </>
        ) : (
          <>
            <span
              className={`size-2.5 shrink-0 rounded-full ${
                paused
                  ? "bg-amber-400"
                  : "animate-pulse bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.9)] motion-reduce:animate-none"
              }`}
            />
            <span className="font-mono text-sm text-zinc-200">{fmt(stats.elapsedMs)}</span>
            <span className="font-mono text-xs text-zinc-400">
              {stats.frameCount}f
              {stats.droppedFrames > 0 && (
                <span className="text-amber-500"> −{stats.droppedFrames}</span>
              )}
            </span>

            <div className="mx-1 h-6 w-px bg-white/10" />

            <button
              onClick={() => void (paused ? resumeRecording() : pauseRecording())}
              title={paused ? t("resumeTitle") : t("pauseTitle")}
              className="rounded-lg px-2 py-1.5 text-zinc-200 vg-transition hover:bg-void-700"
            >
              {paused ? <IconPlay /> : <IconPause />}
            </button>
            <button
              onClick={() => void stopRecording()}
              title={t("stopEditTitle")}
              className="flex items-center gap-1.5 rounded-lg bg-accent-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.97]"
            >
              <IconStop size={14} />
              {t("stop")}
            </button>
            <button
              onClick={onDiscardClick}
              title={t("discardTitle")}
              className={`flex items-center gap-1 rounded-lg px-2 py-1.5 text-sm vg-transition ${
                confirmDiscard
                  ? "bg-rose-600 text-white hover:bg-rose-500"
                  : "text-zinc-400 hover:bg-void-700 hover:text-rose-400"
              }`}
            >
              <IconX size={14} />
              {confirmDiscard && t("discardConfirm")}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
