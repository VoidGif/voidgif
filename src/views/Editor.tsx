import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { useT } from "../stores/settingsStore";
import {
  cropSession,
  deleteFrames,
  duplicateFrames,
  frameUrl,
  groupFrames,
  makePingpong,
  mergeDuplicates,
  redoEdit,
  scaleDelays,
  setFrameDelays,
  trimStaticEdges,
  undoEdit,
  ungroup,
} from "../lib/ipc";
import Filmstrip from "../components/Filmstrip";
import ExportDialog from "../components/ExportDialog";
import ResizeDialog from "../components/ResizeDialog";
import SaveProjectButton from "../components/SaveProjectButton";
import ContinueRecordButton from "../components/ContinueRecordButton";
import SpeedButton from "../components/SpeedButton";
import LoopButton from "../components/LoopButton";
import ToolbarButton from "../components/ToolbarButton";
import {
  IconBack,
  IconCheck,
  IconClock,
  IconCopy,
  IconCrop,
  IconExport,
  IconGroup,
  IconLoop,
  IconMerge,
  IconNext,
  IconPause,
  IconPlay,
  IconPrev,
  IconRedo,
  IconResize,
  IconTrash,
  IconTrim,
  IconUndo,
  IconUngroup,
} from "../components/icons";
import type { Region, SessionInfo } from "../types";

/** Drag rectangle in preview-container coordinates. */
interface CropDrag {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export default function Editor() {
  const t = useT();
  const session = useAppStore((s) => s.session);
  const sessionRev = useAppStore((s) => s.sessionRev);
  const setSession = useAppStore((s) => s.setSession);
  const setView = useAppStore((s) => s.setView);

  const [current, setCurrent] = useState(0);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [playing, setPlaying] = useState(false);
  const [cropMode, setCropMode] = useState(false);
  const [cropDrag, setCropDrag] = useState<CropDrag | null>(null);
  const [showExport, setShowExport] = useState(false);
  const [showResize, setShowResize] = useState(false);
  const [delayInput, setDelayInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  // Loop-seam preview: playback cycles only the last 8 + first 8 frames so the
  // user can judge how cleanly the loop stitches. Pure frontend state.
  const [seamPreview, setSeamPreview] = useState(false);

  const imgRef = useRef<HTMLImageElement>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const playTimer = useRef<number>(0);
  const infoTimer = useRef<number>(0);

  const frames = session?.frames ?? [];
  const clampedCurrent = Math.min(current, Math.max(0, frames.length - 1));
  const currentFrame = frames[clampedCurrent];
  const modalOpen = showExport || showResize;

  // Group/Ungroup toggle state derived from the current selection:
  //  • Ungroup when every selected frame belongs to one and the same group.
  //  • Group when ≥2 selected frames form a contiguous run.
  const groupSel = useMemo(() => {
    const ids = [...selected];
    if (ids.length === 0) return { canGroup: false, ungroupId: null as number | null };
    const picked = ids
      .map((id) => frames.find((f) => f.id === id))
      .filter((f): f is (typeof frames)[number] => !!f);
    const gid0 = picked[0]?.groupId ?? null;
    const ungroupId =
      gid0 != null && picked.every((f) => f.groupId === gid0) ? gid0 : null;
    const idxs = picked
      .map((f) => frames.indexOf(f))
      .sort((a, b) => a - b);
    const canGroup =
      idxs.length >= 2 && idxs[idxs.length - 1] - idxs[0] + 1 === idxs.length;
    return { canGroup, ungroupId };
  }, [selected, frames]);
  const showUngroup = groupSel.ungroupId != null;

  const applyResult = useCallback(
    (s: SessionInfo, pixelsChanged = false) => {
      setSession(s, { pixelsChanged });
      setSelected(new Set());
      setCurrent((c) => Math.min(c, Math.max(0, s.frames.length - 1)));
      setError(null);
    },
    [setSession],
  );

  const run = useCallback(
    async (op: () => Promise<SessionInfo>, pixelsChanged = false) => {
      try {
        applyResult(await op(), pixelsChanged);
      } catch (e) {
        setError(String(e));
      }
    },
    [applyResult],
  );

  // Transient, non-error status line (auto-dismisses). Clears any error so the
  // two banners never stack.
  const showInfo = useCallback((msg: string) => {
    setError(null);
    setInfo(msg);
    window.clearTimeout(infoTimer.current);
    infoTimer.current = window.setTimeout(() => setInfo(null), 3200);
  }, []);
  useEffect(() => () => window.clearTimeout(infoTimer.current), []);

  // ---- Wave 1 editor tools ----
  const doTrim = useCallback(async () => {
    try {
      const { session: s, removed } = await trimStaticEdges();
      applyResult(s);
      showInfo(removed > 0 ? t("trimmedFrames", { count: removed }) : t("trimNothing"));
    } catch (e) {
      setError(String(e));
    }
  }, [applyResult, showInfo, t]);

  const doMerge = useCallback(async () => {
    try {
      const { session: s, merged } = await mergeDuplicates();
      applyResult(s);
      showInfo(merged > 0 ? t("mergedFrames", { count: merged }) : t("mergeNothing"));
    } catch (e) {
      setError(String(e));
    }
  }, [applyResult, showInfo, t]);

  const doScale = useCallback(
    (factor: number) => {
      const ids = selected.size > 0 ? [...selected] : [];
      void run(() => scaleDelays(ids, factor));
    },
    [run, selected],
  );

  // Frame indices played while seam preview is active: the last 8 then first 8.
  const seamIndices = useMemo(() => {
    const n = frames.length;
    if (n === 0) return [] as number[];
    const span = 8;
    const idxs: number[] = [];
    for (let i = Math.max(0, n - span); i < n; i++) idxs.push(i);
    for (let i = 0; i < Math.min(span, n); i++) idxs.push(i);
    return idxs;
  }, [frames.length]);

  const toggleSeam = useCallback(() => {
    setSeamPreview((on) => {
      const next = !on;
      if (next) {
        setCurrent(seamIndices[0] ?? 0);
        setPlaying(true);
      }
      return next;
    });
  }, [seamIndices]);

  // ---- playback ----
  useEffect(() => {
    if (!playing || frames.length === 0) return;
    const delay = frames[clampedCurrent]?.delayMs ?? 33;
    playTimer.current = window.setTimeout(() => {
      setCurrent((c) => {
        // Seam preview walks the last-8 → first-8 ring instead of the full clip.
        if (seamPreview && seamIndices.length > 0) {
          const pos = seamIndices.indexOf(c);
          return seamIndices[pos < 0 ? 0 : (pos + 1) % seamIndices.length];
        }
        return (c + 1) % frames.length;
      });
    }, Math.max(10, delay));
    return () => window.clearTimeout(playTimer.current);
  }, [playing, clampedCurrent, frames, seamPreview, seamIndices]);

  // ---- frame navigation (arrow keys + the ⏮/⏭ toolbar steppers) ----
  const navigate = useCallback(
    (index: number) => {
      const clamped = Math.max(0, Math.min(index, frames.length - 1));
      setCurrent(clamped);
      // Keep selection in sync with navigation so Delete always removes the
      // frame the user is looking at.
      const frame = frames[clamped];
      if (frame) setSelected(new Set([frame.id]));
    },
    [frames],
  );

  // ---- keyboard ----
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (showExport) return; // ExportDialog owns its close (may be mid-export)
        if (showResize) setShowResize(false);
        else if (cropMode) {
          setCropMode(false);
          setCropDrag(null);
        }
        return;
      }
      // Never mutate the session behind a modal dialog.
      if (modalOpen) return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if (e.key === " ") {
        e.preventDefault();
        setPlaying((p) => !p);
      } else if (e.key === "ArrowRight") {
        navigate(clampedCurrent + 1);
      } else if (e.key === "ArrowLeft") {
        navigate(clampedCurrent - 1);
      } else if (e.key === "Home") {
        navigate(0);
      } else if (e.key === "End") {
        navigate(frames.length - 1);
      } else if (e.key === "Delete" && selected.size > 0) {
        void run(() => deleteFrames([...selected]));
      } else if (e.ctrlKey && e.key.toLowerCase() === "z" && !e.shiftKey) {
        if (session?.canUndo) void run(() => undoEdit(), true);
      } else if (
        (e.ctrlKey && e.key.toLowerCase() === "y") ||
        (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "z")
      ) {
        if (session?.canRedo) void run(() => redoEdit(), true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [clampedCurrent, cropMode, frames, modalOpen, navigate, run, selected, session, showExport, showResize]);

  if (!session) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4">
        <p className="text-ink-2">{t("noRecordingLoaded")}</p>
        <button
          onClick={() => setView("home")}
          className="rounded-lg bg-accent-600 px-4 py-2 text-sm text-white"
        >
          {t("backToHome")}
        </button>
      </div>
    );
  }

  // ---- crop geometry: container-relative drag → source pixels ----
  const toContainer = (e: React.PointerEvent) => {
    const rect = previewRef.current?.getBoundingClientRect();
    return rect
      ? { x: e.clientX - rect.left, y: e.clientY - rect.top }
      : { x: e.clientX, y: e.clientY };
  };

  const applyCrop = async () => {
    const img = imgRef.current;
    const container = previewRef.current;
    if (!img || !container || !cropDrag) return;
    const imgRect = img.getBoundingClientRect();
    const contRect = container.getBoundingClientRect();
    // Image rect in container coordinates (the drag's coordinate space).
    const imgL = imgRect.left - contRect.left;
    const imgT = imgRect.top - contRect.top;
    const sx = session.width / imgRect.width;
    const sy = session.height / imgRect.height;
    // Clamp the drag to the image area so edge-overshoot gestures work.
    const cx0 = Math.max(0, Math.min(cropDrag.x0, cropDrag.x1) - imgL);
    const cy0 = Math.max(0, Math.min(cropDrag.y0, cropDrag.y1) - imgT);
    const cx1 = Math.min(imgRect.width, Math.max(cropDrag.x0, cropDrag.x1) - imgL);
    const cy1 = Math.min(imgRect.height, Math.max(cropDrag.y0, cropDrag.y1) - imgT);
    if (cx1 - cx0 < 2 || cy1 - cy0 < 2) {
      setError(t("errCropOutside"));
      return;
    }
    const x = Math.max(0, Math.min(session.width - 1, Math.round(cx0 * sx)));
    const y = Math.max(0, Math.min(session.height - 1, Math.round(cy0 * sy)));
    const region: Region = {
      x,
      y,
      width: Math.max(1, Math.min(session.width - x, Math.round((cx1 - cx0) * sx))),
      height: Math.max(1, Math.min(session.height - y, Math.round((cy1 - cy0) * sy))),
    };
    setCropMode(false);
    setCropDrag(null);
    await run(() => cropSession(region), true);
  };

  const setDelays = async () => {
    const ms = parseInt(delayInput, 10);
    if (!Number.isFinite(ms) || ms < 10 || ms > 60000) {
      setError(t("errDelayRange"));
      return;
    }
    if (selected.size === 0) {
      setError(t("errSelectFramesFirst"));
      return;
    }
    await run(() => setFrameDelays([...selected], ms));
  };

  const sep = <div className="h-5 w-px shrink-0 bg-line-strong" />;

  return (
    <div className="flex h-full flex-col">
      {/* ---- Toolbar: compact icon buttons + rich tooltips, fits 760px ---- */}
      <div className="flex items-center gap-2 border-b border-line bg-void-900 px-3 py-2">
        <div className="flex min-w-0 flex-1 items-center gap-0.5">
          <ToolbarButton
            label={t("home")}
            desc={t("tipHomeDesc")}
            align="start"
            onClick={() => setView("home")}
          >
            <IconBack size={17} />
          </ToolbarButton>
          {sep}
          <ToolbarButton
            label={t("prevFrame")}
            desc={t("tipPrevDesc")}
            hotkey="←"
            disabled={frames.length === 0 || clampedCurrent === 0}
            onClick={() => navigate(clampedCurrent - 1)}
          >
            <IconPrev size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={playing ? t("pause") : t("play")}
            desc={t("tipPlayDesc")}
            hotkey="Space"
            onClick={() => setPlaying((p) => !p)}
          >
            {playing ? <IconPause size={17} /> : <IconPlay size={17} />}
          </ToolbarButton>
          <ToolbarButton
            label={t("nextFrame")}
            desc={t("tipNextDesc")}
            hotkey="→"
            disabled={frames.length === 0 || clampedCurrent >= frames.length - 1}
            onClick={() => navigate(clampedCurrent + 1)}
          >
            <IconNext size={16} />
          </ToolbarButton>
          {sep}
          <ToolbarButton
            label={t("delete")}
            desc={t("tipDeleteDesc")}
            hotkey="Del"
            badge={selected.size}
            disabled={selected.size === 0}
            onClick={() => void run(() => deleteFrames([...selected]))}
          >
            <IconTrash size={17} />
          </ToolbarButton>
          <ToolbarButton
            label={t("duplicate")}
            desc={t("tipDuplicateDesc")}
            disabled={selected.size === 0}
            onClick={() => void run(() => duplicateFrames([...selected]))}
          >
            <IconCopy size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={t("trimStatic")}
            desc={t("tipTrimDesc")}
            disabled={frames.length <= 2}
            onClick={() => void doTrim()}
          >
            <IconTrim size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={t("mergeDupes")}
            desc={t("tipMergeDesc")}
            disabled={frames.length < 2}
            onClick={() => void doMerge()}
          >
            <IconMerge size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={showUngroup ? t("ungroup") : t("group")}
            desc={showUngroup ? t("tipUngroupDesc") : t("tipGroupDesc")}
            badge={showUngroup ? undefined : selected.size >= 2 ? selected.size : undefined}
            disabled={!showUngroup && !groupSel.canGroup}
            onClick={() =>
              showUngroup
                ? void run(() => ungroup(groupSel.ungroupId!))
                : void run(() => groupFrames([...selected]))
            }
          >
            {showUngroup ? <IconUngroup size={16} /> : <IconGroup size={16} />}
          </ToolbarButton>
          {sep}
          <ContinueRecordButton
            hasFrames={frames.length > 0}
            currentFrameId={currentFrame?.id}
            onError={setError}
          />
          {sep}
          <div className="flex shrink-0 items-center gap-1 pl-1">
            <IconClock size={15} className="shrink-0 text-ink-3" />
            <input
              value={delayInput}
              onChange={(e) => setDelayInput(e.target.value.replace(/\D/g, ""))}
              onKeyDown={(e) => e.key === "Enter" && void setDelays()}
              placeholder={currentFrame ? `${currentFrame.delayMs}` : t("msPlaceholder")}
              title={t("delayInputTitle")}
              aria-label={t("delay")}
              className="w-12 rounded-md border border-line-strong bg-void-800 px-1.5 py-1 text-center font-mono text-xs text-ink-1 outline-none vg-transition focus:border-accent-500"
            />
            <ToolbarButton
              label={t("set")}
              desc={t("tipSetDelayDesc")}
              disabled={selected.size === 0 || delayInput === ""}
              onClick={() => void setDelays()}
            >
              <IconCheck size={16} />
            </ToolbarButton>
            <SpeedButton
              selectedCount={selected.size}
              disabled={frames.length === 0}
              onApply={doScale}
            />
          </div>
          {sep}
          <ToolbarButton
            label={t("crop")}
            desc={t("tipCropDesc")}
            active={cropMode}
            onClick={() => {
              setCropMode((m) => !m);
              setCropDrag(null);
            }}
          >
            <IconCrop size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={t("resize")}
            desc={t("tipResizeDesc")}
            onClick={() => setShowResize(true)}
          >
            <IconResize size={16} />
          </ToolbarButton>
          <LoopButton
            frameCount={frames.length}
            seamActive={seamPreview}
            onPingpong={() => void run(() => makePingpong())}
            onEndFreeze={() => {
              const last = frames[frames.length - 1];
              if (last) void run(() => setFrameDelays([last.id], 1000));
            }}
            onToggleSeam={toggleSeam}
          />
          {sep}
          <ToolbarButton
            label={t("undo")}
            desc={t("tipUndoDesc")}
            hotkey="Ctrl+Z"
            disabled={!session.canUndo}
            onClick={() => void run(() => undoEdit(), true)}
          >
            <IconUndo size={16} />
          </ToolbarButton>
          <ToolbarButton
            label={t("redo")}
            desc={t("tipRedoDesc")}
            hotkey="Ctrl+Y"
            disabled={!session.canRedo}
            onClick={() => void run(() => redoEdit(), true)}
          >
            <IconRedo size={16} />
          </ToolbarButton>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          {/* At the 760px hard-minimum these two wide, non-icon elements are
              dropped so the icon toolbar never overlaps; both reappear ≥900px
              (the default 960 window is unaffected). */}
          <span className="hidden whitespace-nowrap font-mono text-[11px] text-ink-3 min-[900px]:inline">
            {t("dimensionsFrames", { w: session.width, h: session.height, count: frames.length })}
          </span>
          <SaveProjectButton />
          <button
            onClick={() => setShowExport(true)}
            className="vg-tt relative flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-lg bg-accent-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.98]"
          >
            <IconExport size={15} />
            <span className="hidden min-[900px]:inline">{t("export")}</span>
            <span className="vg-tip vg-tip-end" role="tooltip">
              <span className="vg-tip-name">{t("export")}</span>
              <span className="vg-tip-desc">{t("tipExportDesc")}</span>
            </span>
          </button>
        </div>
      </div>

      {error && (
        <div className="flex items-center justify-between border-b border-rose-900/40 bg-rose-950/40 px-4 py-1.5 text-xs text-rose-300">
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            aria-label={t("dismissError")}
            className="ml-4 text-rose-400 hover:text-rose-200"
          >
            ✕
          </button>
        </div>
      )}

      {info && !error && (
        <div
          role="status"
          className="flex items-center gap-2 border-b border-emerald-500/25 bg-emerald-500/10 px-4 py-1.5 text-xs text-emerald-300"
        >
          <IconCheck size={13} className="shrink-0" />
          <span>{info}</span>
        </div>
      )}

      {/* ---- Preview ---- */}
      <div
        ref={previewRef}
        className="checkerboard relative flex min-h-0 flex-1 items-center justify-center overflow-hidden p-6"
      >
        {currentFrame && (
          <img
            ref={imgRef}
            src={frameUrl(currentFrame.id, undefined, sessionRev)}
            alt={`frame ${clampedCurrent + 1}`}
            draggable={false}
            className="max-h-full max-w-full rounded shadow-2xl shadow-black/40 ring-1 ring-line-strong"
          />
        )}

        {cropMode && (
          <div
            className="absolute inset-0 cursor-crosshair"
            onPointerDown={(e) => {
              const p = toContainer(e);
              setCropDrag({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
              (e.target as HTMLElement).setPointerCapture(e.pointerId);
            }}
            onPointerMove={(e) => {
              if (e.buttons === 1) {
                const p = toContainer(e);
                setCropDrag((d) => (d ? { ...d, x1: p.x, y1: p.y } : d));
              }
            }}
          >
            {cropDrag && (
              <div
                className="absolute border-2 border-accent-400 bg-accent-500/10"
                style={{
                  left: Math.min(cropDrag.x0, cropDrag.x1),
                  top: Math.min(cropDrag.y0, cropDrag.y1),
                  width: Math.abs(cropDrag.x1 - cropDrag.x0),
                  height: Math.abs(cropDrag.y1 - cropDrag.y0),
                }}
              />
            )}
            <div className="vg-modal absolute bottom-4 left-1/2 flex -translate-x-1/2 gap-2 rounded-xl border border-line-strong bg-void-900/95 p-2 shadow-xl shadow-black/30 backdrop-blur-sm">
              <button
                onClick={() => void applyCrop()}
                disabled={!cropDrag}
                className="rounded-lg bg-accent-600 px-4 py-1.5 text-sm font-medium text-white vg-transition hover:bg-accent-500 disabled:opacity-40"
              >
                {t("applyCrop")}
              </button>
              <button
                onClick={() => {
                  setCropMode(false);
                  setCropDrag(null);
                }}
                className="rounded-lg px-3 py-1.5 text-sm text-ink-2 vg-transition hover:bg-void-700"
              >
                {t("cancel")}
              </button>
            </div>
          </div>
        )}

        <span
          data-seam-preview={seamPreview || undefined}
          className={`absolute right-3 top-3 flex items-center gap-1.5 rounded-md border px-2 py-0.5 font-mono text-xs backdrop-blur-sm ${
            seamPreview
              ? "border-accent-500/60 bg-accent-600/20 text-accent-300"
              : "border-line bg-void-900/85 text-ink-2"
          }`}
        >
          {seamPreview && (
            <span className="flex items-center gap-1 text-accent-300">
              <IconLoop size={12} />
              <span className="not-italic">{t("seamPreviewActive")}</span>
            </span>
          )}
          <span>
            {frames.length > 0 ? clampedCurrent + 1 : 0} / {frames.length}
          </span>
        </span>
      </div>

      {/* ---- Filmstrip ---- */}
      <Filmstrip
        current={clampedCurrent}
        selected={selected}
        onNavigate={(i) => {
          setPlaying(false);
          setCurrent(i);
        }}
        onSelect={setSelected}
        onError={setError}
      />

      {showExport && <ExportDialog onClose={() => setShowExport(false)} />}
      {showResize && (
        <ResizeDialog
          onClose={() => setShowResize(false)}
          onApplied={(s) => applyResult(s, true)}
        />
      )}
    </div>
  );
}
