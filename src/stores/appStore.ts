import { create } from "zustand";
import type {
  ExportProgress,
  RecorderPhase,
  RecordingStats,
  SessionInfo,
} from "../types";

export type View = "home" | "editor";

interface AppState {
  view: View;
  recorderPhase: RecorderPhase;
  stats: RecordingStats;
  session: SessionInfo | null;
  /** Bumped whenever pixels change (crop/resize/load) to bust image caches. */
  sessionRev: number;
  exportProgress: ExportProgress | null;
  /** True when the session has unsaved edits since the last autosave. */
  dirty: boolean;
  /** True when the session has work not written to a .voidgif file. Unlike
   *  `dirty` (autosave-scoped), this only clears on an explicit save or a
   *  fresh project load — it gates replacing the session with a new
   *  recording or another project. */
  unsaved: boolean;

  setView: (v: View) => void;
  setRecorderPhase: (p: RecorderPhase) => void;
  setStats: (s: RecordingStats) => void;
  setSession: (s: SessionInfo | null, opts?: { pixelsChanged?: boolean }) => void;
  setExportProgress: (p: ExportProgress | null) => void;
  /** Clear the dirty flag after a successful autosave. */
  clearDirty: () => void;
  /** Clear the unsaved flag after an explicit save or fresh project load. */
  markSaved: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  view: "home",
  recorderPhase: "idle",
  stats: { frameCount: 0, elapsedMs: 0, droppedFrames: 0 },
  session: null,
  sessionRev: 0,
  exportProgress: null,
  dirty: false,
  unsaved: false,

  setView: (view) => set({ view }),
  setRecorderPhase: (recorderPhase) => set({ recorderPhase }),
  setStats: (stats) => set({ stats }),
  // Every session mutation (record/edit/load) marks the state dirty so the
  // autosave timer knows there's something new to snapshot.
  setSession: (session, opts) =>
    set((prev) => ({
      session,
      sessionRev: opts?.pixelsChanged ? prev.sessionRev + 1 : prev.sessionRev,
      dirty: true,
      unsaved: session !== null,
    })),
  setExportProgress: (exportProgress) => set({ exportProgress }),
  clearDirty: () => set({ dirty: false }),
  markSaved: () => set({ unsaved: false }),
}));
