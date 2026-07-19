/** Shared types mirrored by the Rust backend (serde camelCase). */

export interface MonitorInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  isPrimary: boolean;
}

/** Physical pixels, virtual-desktop coordinates. */
export interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface FrameMeta {
  /** Stable id — survives reordering. */
  id: number;
  /** Delay shown for this frame, in milliseconds. */
  delayMs: number;
  /** Owning group id, or null/undefined when ungrouped. */
  groupId?: number | null;
}

/** A frame group: auto number + palette color (index 0..5). No custom name. */
export interface GroupInfo {
  id: number;
  number: number;
  color: number;
}

export interface SessionInfo {
  id: string;
  width: number;
  height: number;
  frames: FrameMeta[];
  groups: GroupInfo[];
  /** Nominal capture FPS the session was recorded at. */
  fps: number;
  canUndo: boolean;
  canRedo: boolean;
}

/** Result of trim-static-edges: the updated session plus how many frames went. */
export interface TrimResult {
  session: SessionInfo;
  removed: number;
}

/** Result of merge-duplicates: the updated session plus how many frames folded. */
export interface MergeResult {
  session: SessionInfo;
  merged: number;
}

export type RecorderPhase =
  | "idle"
  | "selecting"
  | "countdown"
  | "recording"
  | "paused";

/** Backend snapshot of the recorder, queried on recorder-panel mount. */
export interface RecorderStateInfo {
  phase: "idle" | "armed" | "recording" | "paused";
  fps: number;
  showCursor: boolean;
  fullScreen: boolean;
  /** True for a continue-recording: size is locked, full-screen hidden. */
  continueMode: boolean;
}

/** Physical sizes returned when the panel reports its hole layout. */
export interface FrameInfo {
  /** Hole size in physical pixels (the region that would be captured). */
  holeWidth: number;
  holeHeight: number;
  /** Size of the monitor the frame window currently sits on. */
  monitorWidth: number;
  monitorHeight: number;
}

export interface RecordingStats {
  frameCount: number;
  elapsedMs: number;
  droppedFrames: number;
}

export type ExportFormat = "gif" | "apng" | "png-seq" | "mp4" | "webm";

export interface GifExportSettings {
  format: ExportFormat;
  path: string;
  /** gifski quality 1–100. */
  quality: number;
  /** Downscale target width; null = source size. */
  width: number | null;
  /** true = infinite loop. */
  loop: boolean;
  fast: boolean;
}

export interface ExportProgress {
  current: number;
  total: number;
  stage: "collecting" | "estimating" | "encoding" | "writing" | "done" | "error";
  message?: string;
}

/** Outcome of a target-size auto-fit export (event `export://fit-result`). */
export interface FitResult {
  path: string;
  /** Chosen gifski quality (30–100). */
  quality: number;
  /** Chosen output width in px. */
  width: number;
  /** Final exported file size in bytes. */
  bytes: number;
  targetBytes: number;
  /** True when even the floor settings overshot the target. */
  over: boolean;
}

/** Crash-recovery snapshot header, from `check_autosave`. */
export interface AutosaveInfo {
  /** ISO-8601 UTC timestamp of the last autosave. */
  savedAt: string;
  frames: number;
}

export type ThemeName = "dark" | "light";

/** UI languages. `null` for a stored language means "follow the OS". */
export type Language = "en" | "ko" | "ja";

/** Persisted user settings, mirrored by the Rust `settings::Settings`. */
export interface Settings {
  theme: ThemeName;
  language: Language | null;
  defaultFps: number;
  defaultCursor: boolean;
  /** Folder the last export wrote to; the export dialog reopens here. */
  lastExportDir: string | null;
  /** Folder the last `.voidgif` saved to; the save dialog reopens here. */
  lastProjectDir: string | null;
}
