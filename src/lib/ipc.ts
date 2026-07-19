import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AutosaveInfo,
  ExportProgress,
  FitResult,
  FrameInfo,
  GifExportSettings,
  MergeResult,
  MonitorInfo,
  RecorderStateInfo,
  RecordingStats,
  Region,
  SessionInfo,
  Settings,
  TrimResult,
} from "../types";

/** True when running inside Tauri (false in plain browser preview). */
export const isTauri = "__TAURI_INTERNALS__" in window;

// ---- Monitors ----------------------------------------------------------------

export const listMonitors = () => invoke<MonitorInfo[]>("list_monitors");

// ---- Recorder frame window ----------------------------------------------------

/**
 * Opens the movable/resizable capture frame window (armed state). fps and
 * showCursor seed the armed setup; the panel can adjust them afterwards.
 */
export const openRecorder = (fps: number, showCursor: boolean) =>
  invoke<void>("open_recorder", { fps, showCursor });

/**
 * Opens the recorder in CONTINUE mode: the capture hole is locked to the
 * current session's dimensions and the new frames are merged in on stop.
 * `insert` is "start" | "after" | "end"; `afterFrameId` is required for "after".
 */
export const openRecorderContinue = (
  insert: "start" | "after" | "end",
  afterFrameId?: number,
) =>
  invoke<void>("open_recorder_continue", {
    insert,
    afterFrameId: afterFrameId ?? null,
  });

/**
 * Reports the transparent hole's CSS rect (window-relative) after layout
 * changes; returns physical sizes for the panel's readout.
 */
export const reportHoleRect = (
  x: number,
  y: number,
  width: number,
  height: number,
) => invoke<FrameInfo>("report_hole_rect", { x, y, width, height });

/** Adjusts the armed capture options from the recorder panel. */
export const updateRecorderOptions = (
  fps: number,
  showCursor: boolean,
  fullScreen: boolean,
) => invoke<void>("update_recorder_options", { fps, showCursor, fullScreen });

/** Starts capture from the frame window's hole (or full monitor). */
export const startRecordingFromFrame = () =>
  invoke<void>("start_recording_from_frame");

/** Cancels an armed (not-yet-recording) setup and returns to Home. */
export const cancelArmed = () => invoke<void>("cancel_armed");

/** Backend snapshot for the recorder panel to recover state on mount. */
export const getRecorderState = () =>
  invoke<RecorderStateInfo>("get_recorder_state");

// ---- Recording ----------------------------------------------------------------

export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");

/** Stops capture; the finished session arrives via the recorder://stopped event. */
export const stopRecording = () => invoke<void>("stop_recording");

export const discardRecording = () => invoke<void>("discard_recording");

// ---- Editor operations (frame metadata lives in Rust) ------------------------

export const getSession = () => invoke<SessionInfo | null>("get_session");

export const deleteFrames = (ids: number[]) =>
  invoke<SessionInfo>("delete_frames", { ids });

export const duplicateFrames = (ids: number[]) =>
  invoke<SessionInfo>("duplicate_frames", { ids });

/** New complete ordering of frame ids. */
export const reorderFrames = (order: number[]) =>
  invoke<SessionInfo>("reorder_frames", { order });

/** Groups a contiguous run of frames into a new auto-numbered group. */
export const groupFrames = (ids: number[]) =>
  invoke<SessionInfo>("group_frames", { ids });

/** Dissolves a group. */
export const ungroup = (groupId: number) =>
  invoke<SessionInfo>("ungroup", { groupId });

/**
 * Moves a group's whole block so it starts at `toIndex`, counted in the order
 * WITHOUT the block (the filmstrip drop math).
 */
export const moveGroup = (groupId: number, toIndex: number) =>
  invoke<SessionInfo>("move_group", { groupId, toIndex });

export const setFrameDelays = (ids: number[], delayMs: number) =>
  invoke<SessionInfo>("set_frame_delays", { ids, delayMs });

/** Auto-trims near-static frames off the start and end. Returns the count removed. */
export const trimStaticEdges = () =>
  invoke<TrimResult>("trim_static_edges");

/** Collapses runs of near-identical frames, summing their delays. Returns the count folded. */
export const mergeDuplicates = () =>
  invoke<MergeResult>("merge_duplicates");

/**
 * Scales the delays of `ids` (empty = all frames) by `factor` — a bigger factor
 * plays faster (halves the delay at 2×). Delays clamp to 10–60000 ms.
 */
export const scaleDelays = (ids: number[], factor: number) =>
  invoke<SessionInfo>("scale_delays", { ids, factor });

/** Appends the interior frames in reverse (A B C D → A B C D C B) for a boomerang loop. */
export const makePingpong = () => invoke<SessionInfo>("make_pingpong");

export const cropSession = (rect: Region) =>
  invoke<SessionInfo>("crop_session", { rect });

export const resizeSession = (width: number, height: number) =>
  invoke<SessionInfo>("resize_session", { width, height });

export const undoEdit = () => invoke<SessionInfo>("undo_edit");
export const redoEdit = () => invoke<SessionInfo>("redo_edit");

// ---- Frame images -------------------------------------------------------------

/**
 * Frames are served by the custom `voidframe` URI scheme registered in Rust.
 * WebView2 surfaces custom schemes as http://<scheme>.localhost/; WKWebView
 * keeps the raw scheme. (TODO(mac): verify the WKWebView origin on-device.)
 */
const FRAME_ORIGIN = /mac/i.test(navigator.userAgent)
  ? "voidframe://localhost"
  : "http://voidframe.localhost";

export const frameUrl = (frameId: number, maxWidth?: number, rev = 0) => {
  const params = new URLSearchParams();
  if (maxWidth) params.set("w", String(maxWidth));
  if (rev) params.set("r", String(rev));
  const q = params.toString();
  return `${FRAME_ORIGIN}/frame/${frameId}${q ? `?${q}` : ""}`;
};

// ---- Export -------------------------------------------------------------------

export const startExport = (settings: GifExportSettings) =>
  invoke<void>("start_export", { settings });

export const cancelExport = () => invoke<void>("cancel_export");

/** Estimates the GIF export size in bytes. Rejects with "busy" if another
 * estimate is in flight (the caller ignores that). */
export const estimateGifSize = (settings: GifExportSettings) =>
  invoke<number>("estimate_gif_size", { settings });

/** Auto-fits settings to `targetBytes`, then exports. Progress + the final
 * chosen settings arrive via export://progress and export://fit-result. */
export const startExportFit = (settings: GifExportSettings, targetBytes: number) =>
  invoke<void>("start_export_fit", { settings, targetBytes });

// ---- Share exported file ---------------------------------------------------------

/** Puts the exported file on the clipboard as a file drop (Ctrl+V pastes it). */
export const copyFileToClipboard = (path: string) =>
  invoke<void>("copy_file_to_clipboard", { path });

/** Opens the OS file browser with the file selected. */
export const revealInExplorer = (path: string) =>
  invoke<void>("reveal_in_explorer", { path });

/** Opens a fixed project page (source · notices · privacy · website) in the default browser. */
export const openExternal = (page: string) =>
  invoke<void>("open_external", { page });

// ---- Autosave / crash recovery ---------------------------------------------------

/** Saves the live session to the autosave slot; false when there's no session. */
export const autosaveNow = () => invoke<boolean>("autosave_now");

/** Returns the pending crash-recovery snapshot, or null when none exists. */
export const checkAutosave = () => invoke<AutosaveInfo | null>("check_autosave");

/** Deletes the crash-recovery snapshot. */
export const discardAutosave = () => invoke<void>("discard_autosave");

/** Loads the crash-recovery snapshot into the live session (one-shot). */
export const restoreAutosave = () => invoke<SessionInfo>("restore_autosave");

// ---- Project save/load ----------------------------------------------------------

export const saveProject = (path: string) =>
  invoke<void>("save_project", { path });

export const loadProject = (path: string) =>
  invoke<SessionInfo>("load_project", { path });

/** Opens an existing GIF file as a new editable session. */
export const importGif = (path: string) =>
  invoke<SessionInfo>("import_gif", { path });

// ---- Settings -------------------------------------------------------------------

/** Returns null when no settings file exists yet (first run → onboarding). */
export const getSettings = () => invoke<Settings | null>("get_settings");

export const setSettings = (settings: Settings) =>
  invoke<void>("set_settings", { settings });

/**
 * Relabels the native tray menu. The tray can't read the webview dictionaries,
 * so the frontend reports the resolved language (setting or OS fallback).
 */
export const setTrayLanguage = (lang: string) =>
  invoke<void>("set_tray_language", { lang });

// ---- Events ---------------------------------------------------------------------

export const onRecordingStats = (cb: (s: RecordingStats) => void) =>
  listen<RecordingStats>("recorder://stats", (e) => cb(e.payload));

export const onRecordingStarted = (cb: () => void) =>
  listen<void>("recorder://started", () => cb());

export const onRecordingStopped = (cb: (s: SessionInfo) => void) =>
  listen<SessionInfo>("recorder://stopped", (e) => cb(e.payload));

export const onRecordingDiscarded = (cb: () => void) =>
  listen<void>("recorder://discarded", () => cb());

export const onPauseState = (cb: (paused: boolean) => void) =>
  listen<boolean>("recorder://pausestate", (e) => cb(e.payload));

export const onExportProgress = (cb: (p: ExportProgress) => void) =>
  listen<ExportProgress>("export://progress", (e) => cb(e.payload));

export const onExportFitResult = (cb: (r: FitResult) => void) =>
  listen<FitResult>("export://fit-result", (e) => cb(e.payload));

export type { UnlistenFn };
