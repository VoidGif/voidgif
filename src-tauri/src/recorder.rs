//! Recording orchestration: wires a platform capture backend to the disk
//! spool, computes per-frame delays, and reports stats to the UI.

use crate::capture::{self, CaptureConfig, CaptureFlags, CaptureSession};
use crate::session::Session;
use crate::state::ContinueInsert;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStats {
    pub frame_count: usize,
    pub elapsed_ms: u64,
    pub dropped_frames: u64,
}

pub struct ActiveRecording {
    backend: Box<dyn CaptureSession>,
    writer: JoinHandle<Result<Session, String>>,
    pub flags: Arc<CaptureFlags>,
    /// Retained so the recorder panel can report them via get_recorder_state.
    pub fps: u32,
    pub show_cursor: bool,
    /// Set for continue-recordings: the stop path merges into the existing
    /// session at this position instead of replacing it. `None` = normal.
    pub continue_insert: Option<ContinueInsert>,
}

fn session_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("voidgif")
}

fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("rec-{nanos:x}")
}

pub fn start(app: AppHandle, config: CaptureConfig) -> Result<ActiveRecording, String> {
    let session = Session::create(
        &session_dir(),
        new_session_id(),
        config.region.width,
        config.region.height,
        config.fps,
    )
    .map_err(|e| format!("failed to create session spool: {e}"))?;

    let (tx, rx) = crossbeam_channel::bounded::<capture::CapturedFrame>(8);
    let flags = Arc::new(CaptureFlags::default());

    let backend = capture::start_backend(config, tx, Arc::clone(&flags))?;

    let nominal_ms = Session::nominal_delay_ms(config.fps);
    let writer_flags = Arc::clone(&flags);
    let writer = std::thread::spawn(move || -> Result<Session, String> {
        let mut session = session;
        let mut prev_at: Option<Instant> = None;
        let mut elapsed_ms: u64 = 0;
        let mut last_stats = Instant::now();

        // Exits when the capture backend stops and drops the sender.
        while let Ok(frame) = rx.recv() {
            // Invariant: every spooled frame matches the session dimensions.
            // A display-mode change mid-recording can shrink the capture; a
            // mismatched frame would corrupt crop/APNG later, so drop it.
            if frame.width != session.width || frame.height != session.height {
                writer_flags.dropped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let gap_ms = prev_at
                .map(|p| frame.captured_at.duration_since(p).as_millis() as u64)
                .unwrap_or(nominal_ms as u64);
            // A gap much larger than nominal means a pause (or a stall):
            // splice it out of the timeline instead of freezing the GIF.
            let delay_ms = if gap_ms > 3 * nominal_ms as u64 {
                nominal_ms
            } else {
                gap_ms.clamp(10, 1000) as u32
            };
            prev_at = Some(frame.captured_at);
            elapsed_ms += delay_ms as u64;

            session
                .append_frame(&frame.bgra, frame.width, frame.height, delay_ms)
                .map_err(|e| format!("failed to spool frame: {e}"))?;

            if last_stats.elapsed() >= Duration::from_millis(250) {
                last_stats = Instant::now();
                let _ = app.emit(
                    "recorder://stats",
                    RecordingStats {
                        frame_count: session.frames.len(),
                        elapsed_ms,
                        dropped_frames: writer_flags.dropped.load(Ordering::Relaxed),
                    },
                );
            }
        }
        Ok(session)
    });

    Ok(ActiveRecording {
        backend,
        writer,
        flags,
        fps: config.fps,
        show_cursor: config.show_cursor,
        continue_insert: None,
    })
}

/// Stops capture, drains the pipeline, and returns the finished session.
pub fn stop(rec: ActiveRecording) -> Result<Session, String> {
    if let Err(e) = rec.backend.stop() {
        // The capture thread may still be alive and feeding the writer.
        // Don't leak the writer thread or its spool file — reap them in the
        // background once the capture side eventually winds down.
        std::thread::spawn(move || {
            if let Ok(Ok(session)) = rec.writer.join() {
                session.cleanup();
            }
        });
        return Err(format!("failed to stop capture: {e}"));
    }
    rec.writer
        .join()
        .map_err(|_| "recording writer thread panicked".to_string())?
}
