//! Shared application state managed by Tauri.

use crate::recorder::ActiveRecording;
use crate::session::Session;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

/// Where a continue-recording's frames are spliced into the existing session.
/// `None` on an ArmedSetup means a normal (replace) recording.
#[derive(Debug, Clone, Copy)]
pub enum ContinueInsert {
    /// Before the first frame.
    Start,
    /// Right after the frame with this id (falls back to End if it's gone).
    AfterFrame(u64),
    /// Appended at the end.
    End,
}

/// Capture settings staged while the recorder frame window is open but not
/// yet recording (the ScreenToGif-style "armed" state). All of them may be
/// tweaked from the panel before the user hits record.
pub struct ArmedSetup {
    pub fps: u32,
    pub show_cursor: bool,
    pub full_screen: bool,
    /// Set when this is a continue-recording; the capture hole is then locked
    /// to the existing session's dimensions and the result is merged in.
    pub continue_insert: Option<ContinueInsert>,
}

/// The transparent capture hole of the recorder frame window, in CSS pixels
/// relative to the window's inner top-left. Reported by the frontend whenever
/// its layout changes; the physical rect is derived live from the window.
#[derive(Debug, Clone, Copy)]
pub struct HoleRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
    pub recording: Mutex<Option<ActiveRecording>>,
    /// Staged capture settings while the recorder frame window is open.
    pub armed: Mutex<Option<ArmedSetup>>,
    /// Latest hole rect reported by the recorder frame window.
    pub hole_rect: Mutex<Option<HoleRect>>,
    /// Bumped on every recorder-window open so stale cursor pollers exit.
    pub poller_gen: AtomicU64,
    pub export_running: Arc<AtomicBool>,
    pub export_cancel: Arc<AtomicBool>,
    /// Serializes size-estimation calls: a second concurrent estimate returns
    /// `busy` (the UI ignores it) rather than piling encode work on the disk.
    pub estimating: AtomicBool,
}

impl AppState {
    /// Replaces the current session, cleaning up the old spool file.
    pub fn replace_session(&self, new: Option<Session>) {
        let mut guard = self.session.lock().expect("session lock poisoned");
        if let Some(old) = guard.take() {
            old.cleanup();
        }
        *guard = new;
    }
}
