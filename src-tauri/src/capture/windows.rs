//! Windows capture backend — Windows Graphics Capture via `windows-capture`.
//!
//! WGC only captures whole monitors, so we find the monitor containing the
//! requested region, capture it, and crop each frame GPU-side with
//! `Frame::buffer_crop`. Frames arrive VSync-paced at the monitor refresh
//! rate; we throttle down to the requested fps before copying any pixels.

use super::{CaptureConfig, CaptureFlags, CapturedFrame, CaptureSession, Region};
use crossbeam_channel::{Sender, TrySendError};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

struct MonitorHit {
    hmonitor: isize,
    rect: RECT,
}

/// Enumerates physical monitors with their virtual-desktop rects.
fn enumerate_monitor_rects() -> Vec<MonitorHit> {
    unsafe extern "system" fn callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let hits = unsafe { &mut *(lparam.0 as *mut Vec<MonitorHit>) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool() {
            hits.push(MonitorHit {
                hmonitor: hmonitor.0 as isize,
                rect: info.rcMonitor,
            });
        }
        true.into()
    }

    let mut hits: Vec<MonitorHit> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM(&mut hits as *mut _ as isize),
        );
    }
    hits
}

/// Finds the monitor containing the region's center point.
fn monitor_for_region(region: &Region) -> Result<MonitorHit, String> {
    let cx = region.x + region.width as i32 / 2;
    let cy = region.y + region.height as i32 / 2;
    enumerate_monitor_rects()
        .into_iter()
        .find(|m| {
            cx >= m.rect.left && cx < m.rect.right && cy >= m.rect.top && cy < m.rect.bottom
        })
        .ok_or_else(|| "selected region is not on any monitor".to_string())
}

/// Everything the capture handler needs, passed through Settings flags.
struct HandlerFlags {
    sink: Sender<CapturedFrame>,
    flags: Arc<CaptureFlags>,
    /// Crop rect in monitor-local physical pixels.
    crop: (u32, u32, u32, u32),
    frame_interval: Duration,
}

struct Handler {
    cfg: HandlerFlags,
    next_due: Instant,
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = HandlerFlags;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            cfg: ctx.flags,
            next_due: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let now = Instant::now();
        if self.cfg.flags.paused.load(Ordering::Relaxed) {
            self.next_due = now;
            return Ok(());
        }
        if now < self.next_due {
            return Ok(()); // throttle down to target fps
        }
        // Schedule from the ideal grid to avoid drift, but never fall behind
        // more than one interval.
        self.next_due = (self.next_due + self.cfg.frame_interval).max(now);

        let (x0, y0, x1, y1) = self.cfg.crop;
        let frame_w = frame.width();
        let frame_h = frame.height();
        // Guard against monitor mode changes mid-recording.
        let x1 = x1.min(frame_w);
        let y1 = y1.min(frame_h);
        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }
        let w = x1 - x0;
        let h = y1 - y0;

        let mut buffer = frame.buffer_crop(x0, y0, x1, y1)?;
        let row_pitch = buffer.row_pitch() as usize;
        let raw = buffer.as_raw_buffer();

        let mut packed = vec![0u8; (w * h * 4) as usize];
        let tight_row = (w * 4) as usize;
        for y in 0..h as usize {
            let src = &raw[y * row_pitch..y * row_pitch + tight_row];
            packed[y * tight_row..(y + 1) * tight_row].copy_from_slice(src);
        }

        let captured = CapturedFrame {
            bgra: packed,
            width: w,
            height: h,
            captured_at: now,
        };
        match self.cfg.sink.try_send(captured) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.cfg.flags.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {
                // Receiver is gone (stop under way); nothing to do.
            }
        }
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct WindowsSession {
    control: windows_capture::capture::CaptureControl<Handler, <Handler as GraphicsCaptureApiHandler>::Error>,
}

impl CaptureSession for WindowsSession {
    fn stop(self: Box<Self>) -> Result<(), String> {
        self.control.stop().map_err(|e| e.to_string())
    }
}

pub fn start(
    config: CaptureConfig,
    sink: Sender<CapturedFrame>,
    flags: Arc<CaptureFlags>,
) -> Result<Box<dyn CaptureSession>, String> {
    let hit = monitor_for_region(&config.region)?;
    let monitor = Monitor::from_raw_hmonitor(hit.hmonitor as *mut std::ffi::c_void);

    // True rectangle intersection of region and monitor in virtual-desktop
    // coordinates — clamping each edge independently must not shift the crop.
    let ix0 = config.region.x.max(hit.rect.left);
    let iy0 = config.region.y.max(hit.rect.top);
    let ix1 = (config.region.x + config.region.width as i32).min(hit.rect.right);
    let iy1 = (config.region.y + config.region.height as i32).min(hit.rect.bottom);
    if ix0 >= ix1 || iy0 >= iy1 {
        return Err("selected region lies outside the monitor".into());
    }
    let x0 = (ix0 - hit.rect.left) as u32;
    let y0 = (iy0 - hit.rect.top) as u32;
    let x1 = (ix1 - hit.rect.left) as u32;
    let y1 = (iy1 - hit.rect.top) as u32;

    let handler_flags = HandlerFlags {
        sink,
        flags,
        crop: (x0, y0, x1, y1),
        frame_interval: Duration::from_secs_f64(1.0 / config.fps.max(1) as f64),
    };

    let cursor = if config.show_cursor {
        CursorCaptureSettings::WithCursor
    } else {
        CursorCaptureSettings::WithoutCursor
    };

    let settings = Settings::new(
        monitor,
        cursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        handler_flags,
    );

    let control = Handler::start_free_threaded(settings).map_err(|e| e.to_string())?;
    Ok(Box::new(WindowsSession { control }))
}
