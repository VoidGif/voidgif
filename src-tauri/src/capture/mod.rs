//! Platform-agnostic capture interface.
//!
//! A platform backend pushes tightly-packed BGRA frames into a bounded
//! channel; the recorder's writer thread spools them to disk. Backends must
//! respect `CaptureFlags::paused` and never block on a full channel (drop the
//! frame and bump `dropped` instead — disk hiccups must not stall capture).

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::time::Instant;

/// Physical pixels in virtual-desktop coordinates (can be negative on
/// multi-monitor setups).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureConfig {
    pub region: Region,
    pub fps: u32,
    pub show_cursor: bool,
}

/// One captured frame, tightly packed BGRA8, exactly region-sized.
pub struct CapturedFrame {
    pub bgra: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub captured_at: Instant,
}

#[derive(Default)]
pub struct CaptureFlags {
    pub paused: AtomicBool,
    pub dropped: AtomicU64,
}

/// Handle to a running platform capture; stopping joins the capture thread.
pub trait CaptureSession: Send {
    fn stop(self: Box<Self>) -> Result<(), String>;
}

/// Starts the platform backend for `config`, delivering frames into `sink`.
pub fn start_backend(
    config: CaptureConfig,
    sink: Sender<CapturedFrame>,
    flags: Arc<CaptureFlags>,
) -> Result<Box<dyn CaptureSession>, String> {
    #[cfg(windows)]
    {
        windows::start(config, sink, flags)
    }
    #[cfg(target_os = "macos")]
    {
        macos::start(config, sink, flags)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (config, sink, flags);
        Err("screen capture is not supported on this platform".into())
    }
}
