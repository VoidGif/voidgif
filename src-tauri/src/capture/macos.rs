//! macOS capture backend — ScreenCaptureKit via the `screencapturekit` crate.
//!
//! NOTE: written on Windows against docs.rs/screencapturekit/8.0.0; it is
//! cfg-gated out of Windows builds. Compile + runtime verification happens in
//! a macOS session (see project page). Verify items are marked TODO(mac).

use super::{CaptureConfig, CaptureFlags, CapturedFrame, CaptureSession};
use crossbeam_channel::{Sender, TrySendError};
use screencapturekit::cg::{CGPoint, CGRect, CGSize};
use screencapturekit::cm::CMSampleBufferExt;
use screencapturekit::cv::PixelBufferCursorExt;
use screencapturekit::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Checks/requests the Screen Recording TCC permission.
/// Returns Ok(()) when granted; Err with user guidance otherwise.
pub fn ensure_permission() -> Result<(), String> {
    let access = core_graphics::access::ScreenCaptureAccess::default();
    if access.preflight() {
        return Ok(());
    }
    // Shows the system prompt only once per app install; afterwards the user
    // must enable it in System Settings and relaunch.
    access.request();
    Err(
        "Screen Recording permission is required. Enable VoidGif under System Settings \
         → Privacy & Security → Screen Recording, then relaunch the app."
            .to_string(),
    )
}

struct FrameHandler {
    sink: Sender<CapturedFrame>,
    flags: Arc<CaptureFlags>,
    next_due: Mutex<Instant>,
    frame_interval: Duration,
}

impl SCStreamOutputTrait for FrameHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        let now = Instant::now();
        if self.flags.paused.load(Ordering::Relaxed) {
            *self.next_due.lock().unwrap() = now;
            return;
        }
        {
            let mut due = self.next_due.lock().unwrap();
            if now < *due {
                return;
            }
            *due = (*due + self.frame_interval).max(now);
        }

        let Some(pixel_buffer) = sample.image_buffer() else {
            return;
        };
        let Ok(guard) = pixel_buffer.lock_read_only() else {
            return;
        };
        let stride = pixel_buffer.bytes_per_row();
        let w = pixel_buffer.width();
        let h = pixel_buffer.height();
        let bytes: &[u8] = guard.as_slice();
        let tight_row = w * 4;
        let mut packed = vec![0u8; tight_row * h];
        for y in 0..h {
            let src = &bytes[y * stride..y * stride + tight_row];
            packed[y * tight_row..(y + 1) * tight_row].copy_from_slice(src);
        }
        drop(guard);

        let frame = CapturedFrame {
            bgra: packed,
            width: w as u32,
            height: h as u32,
            captured_at: now,
        };
        match self.sink.try_send(frame) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                self.flags.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

struct MacSession {
    stream: SCStream,
}

// TODO(mac): confirm SCStream is Send; frames flow via the channel so we only
// move the handle across threads for stop().
unsafe impl Send for MacSession {}

impl CaptureSession for MacSession {
    fn stop(self: Box<Self>) -> Result<(), String> {
        self.stream.stop_capture().map_err(|e| format!("{e:?}"))
    }
}

pub fn start(
    config: CaptureConfig,
    sink: Sender<CapturedFrame>,
    flags: Arc<CaptureFlags>,
) -> Result<Box<dyn CaptureSession>, String> {
    ensure_permission()?;

    let content = SCShareableContent::get().map_err(|e| format!("{e:?}"))?;
    let displays = content.displays();
    if displays.is_empty() {
        return Err("no displays available for capture".into());
    }

    // Map the region (physical px, virtual desktop) onto a display.
    // TODO(mac): verify SCDisplay frame/scale accessors against 8.0.0 and
    // multi-monitor coordinate spaces; the primary-display fallback below is
    // correct for single-monitor setups.
    let display = &displays[0];
    let scale = 2.0_f64; // TODO(mac): read backing scale factor from the display
    let region_points = CGRect {
        origin: CGPoint {
            x: config.region.x as f64 / scale,
            y: config.region.y as f64 / scale,
        },
        size: CGSize {
            width: config.region.width as f64 / scale,
            height: config.region.height as f64 / scale,
        },
    };

    let sc_config = SCStreamConfiguration::new()
        .with_source_rect(region_points)
        .with_width(config.region.width)
        .with_height(config.region.height)
        .with_pixel_format(PixelFormat::BGRA)
        .with_shows_cursor(config.show_cursor)
        .with_queue_depth(5);

    let filter = SCContentFilter::create()
        .with_display(display)
        .with_excluding_windows(&[])
        .build();

    let handler = FrameHandler {
        sink,
        flags,
        next_due: Mutex::new(Instant::now()),
        frame_interval: Duration::from_secs_f64(1.0 / config.fps.max(1) as f64),
    };

    let mut stream = SCStream::new(&filter, &sc_config);
    stream.add_output_handler(handler, SCStreamOutputType::Screen);
    stream.start_capture().map_err(|e| format!("{e:?}"))?;

    Ok(Box::new(MacSession { stream }))
}
