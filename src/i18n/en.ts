/**
 * English dictionary — the source of truth. Every key here must be mirrored by
 * ko.ts and ja.ts (enforced by the `Record<keyof typeof en, string>` type).
 * `{name}` tokens are filled at call time by the `t` helper.
 */
export const en = {
  // ---- Common ----
  cancel: "Cancel",
  close: "Close",
  apply: "Apply",

  // ---- Home ----
  appTagline: "Record your screen. Edit every frame. Export beautiful GIFs.",
  selectRegionRecord: "Select region & record",
  fps: "FPS",
  captureCursor: "Capture cursor",
  continueEditing: "← Continue editing ({count} frames)",
  openProject: "Open project…",
  openFile: "Open (project · GIF)",
  hotkeysHint: "Hotkeys: F7 start/pause · F8 stop",
  browserPreviewNotice: "Browser preview — recording requires the desktop app.",
  settingsTitle: "Settings",

  // ---- Editor: toolbar ----
  home: "Home",
  play: "Play",
  pause: "Pause",
  playPauseTitle: "Play/Pause (Space)",
  delete: "Delete",
  deleteSelectedTitle: "Delete selected (Del)",
  duplicate: "Duplicate",
  duplicateTitle: "Duplicate selected",
  delay: "Delay",
  delayInputTitle: "Milliseconds per frame (10–60000)",
  set: "Set",
  setDelayTitle: "Apply delay to selected frames (min 10 ms)",
  crop: "Crop",
  cropTitle: "Crop (Esc to cancel)",
  resize: "Resize",
  resizeTitle: "Resize",
  undoTitle: "Undo (Ctrl+Z)",
  redoTitle: "Redo (Ctrl+Y)",
  dimensionsFrames: "{w}×{h} · {count}f",
  export: "Export",
  msPlaceholder: "ms",

  // ---- Editor: toolbar tooltips (icon-only buttons) ----
  prevFrame: "Previous frame",
  nextFrame: "Next frame",
  undo: "Undo",
  redo: "Redo",
  tipHomeDesc: "Back to the start screen",
  tipPrevDesc: "Step back one frame",
  tipNextDesc: "Step forward one frame",
  tipPlayDesc: "Preview the frames in sequence",
  tipDeleteDesc: "Remove the selected frames",
  tipDuplicateDesc: "Insert a copy of the selected frames",
  tipSetDelayDesc: "Apply this delay to the selected frames (10–60000 ms)",
  tipCropDesc: "Drag over the preview to crop — Esc cancels",
  tipResizeDesc: "Scale every frame to a new size",
  tipUndoDesc: "Revert the last edit",
  tipRedoDesc: "Reapply the reverted edit",
  tipSaveDesc: "Save everything as a .voidgif project",
  tipExportDesc: "Write the animation as GIF, APNG, PNG or MP4",

  // ---- Editor: continue recording ----
  continueRec: "Continue recording",
  tipContinueRecDesc:
    "Record more frames and insert them at the start, after the current frame, or at the end",
  continueAtStart: "At the start",
  continueAfterCurrent: "After current frame",
  continueAtEnd: "At the end",
  errRecordingTooLarge: "The recording area is larger than the screen.",

  // ---- Editor: grouping ----
  group: "Group",
  ungroup: "Ungroup",
  tipGroupDesc: "Group the selected contiguous frames",
  tipUngroupDesc: "Dissolve the selected group",

  // ---- Editor: trim / merge / speed / loop (Wave 1) ----
  trimStatic: "Trim static",
  tipTrimDesc: "Auto-remove still frames from the start and end",
  mergeDupes: "Merge duplicates",
  tipMergeDesc: "Fuse repeated frames into one, adding their delays together",
  speed: "Speed",
  tipSpeedDesc: "Speed up or slow playback — the selection, or every frame",
  loopTools: "Loop",
  tipLoopDesc: "Ping-pong, hold the last frame, or preview the loop seam",
  trimmedFrames: "{count} still frames trimmed",
  trimNothing: "No still edges to trim",
  mergedFrames: "{count} duplicate frames merged",
  mergeNothing: "No duplicate frames found",
  speedTitle: "Playback speed",
  speedApplySelection: "Applies to {count} selected frame(s)",
  speedApplyAll: "Applies to all frames",
  speedClampNote: "Delays under 20 ms may be clamped by some browsers and viewers.",
  loopPingpong: "Make ping-pong (boomerang)",
  loopEndFreeze: "Hold last frame 1 s",
  loopSeamPreview: "Preview loop seam",
  seamPreviewActive: "Seam preview",

  // ---- Recorder: continue mode ----
  sizeLocked: "Size locked",

  // ---- Editor: crop / preview ----
  applyCrop: "Apply crop",
  dismissError: "Dismiss",

  // ---- Editor: empty / errors ----
  noRecordingLoaded: "No recording loaded.",
  backToHome: "Back to home",
  errCropOutside: "Crop selection is outside the image.",
  errDelayRange: "Delay must be between 10 and 60000 ms.",
  errSelectFramesFirst: "Select frames first, then set their delay.",

  // ---- Export dialog ----
  noteGifski: "gifski — best quality",
  noteApng: "lossless, 24-bit",
  notePngSeq: "one file per frame",
  noteMp4: "H.264 video",
  quality: "Quality",
  fastMode: "Fast mode (lower quality, ~3× faster)",
  widthLabel: "Width (px, empty = {size})",
  widthPlaceholder: "e.g. 800",
  exportSourceSizeNote:
    "Exports at source size ({w}×{h}). Use Resize in the editor to change dimensions first.",
  stageCollecting: "collecting",
  stageEstimating: "fitting size",
  stageEncoding: "encoding",
  stageWriting: "writing",
  stageDone: "done",
  stageError: "error",
  exportFailed: "Export failed",
  savedMsg: "Saved ✓ {message}",
  cancelExport: "Cancel export",
  exporting: "Exporting…",

  // ---- Export: size estimate + platform presets ----
  estSize: "Est. size",
  estimatingSize: "Estimating…",
  fitExport: "Export to target size",
  fitChosen: "Quality {quality} · {width}px · {size}",
  fitOverTarget: "Couldn't reach {target} — smallest is {size}",

  // ---- Export: share the result ----
  copyToClipboard: "Copy to clipboard",
  copiedToClipboard: "Copied ✓",
  revealInFolder: "Show in folder",

  // ---- Crash recovery ----
  recoverTitle: "You have unsaved work",
  recoverBody: "{count} frames · saved {time}",
  recoverAction: "Recover",
  recoverDiscard: "Discard",

  // ---- Unsaved-work guard (new recording / open project) ----
  unsavedTitle: "Unsaved work",
  unsavedBodyRecord:
    "Recording again will replace your current {count} frames. Save them first?",
  unsavedBodyOpen:
    "Opening a project will replace your current {count} frames. Save them first?",
  unsavedSaveContinue: "Save & continue",
  unsavedContinue: "Continue without saving",

  // ---- Resize dialog ----
  width: "Width",
  height: "Height",
  lockAspect: "Lock aspect ratio",
  errInvalidSize: "Enter a valid size.",

  // ---- Save project button ----
  save: "Save",
  saving: "Saving…",
  saved: "Saved ✓",
  saveFailed: "Failed ✕",
  saveProjectTitle: "Save project (.voidgif)",

  // ---- Recorder panel ----
  startRecordingTitle: "Start recording (F7)",
  cursor: "Cursor",
  fullScreen: "Full screen",
  recorderHint: "F7 start · F8 close",
  closeTitle: "Close (F8)",
  resumeTitle: "Resume (F7)",
  pauseTitle: "Pause (F7)",
  stop: "Stop",
  stopEditTitle: "Stop & edit (F8)",
  discardTitle: "Discard recording",
  discardConfirm: "Discard?",

  // ---- Onboarding ----
  onboardingSubtitle: "Let's set things up — you can change these anytime.",
  theme: "Theme",
  themeDark: "Dark",
  themeLight: "Light",
  language: "Language",
  languageSystem: "System default",
  continueButton: "Continue",

  // ---- Settings dialog ----
  defaultFps: "Default FPS",
  captureCursorDefault: "Capture cursor by default",
} as const;

export type TranslationKey = keyof typeof en;
