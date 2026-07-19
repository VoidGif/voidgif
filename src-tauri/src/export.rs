//! Export pipeline. Runs on background threads; reports progress through a
//! callback (wired to `export://progress` events by `run`); cancellable via a
//! shared flag.
//!
//! GIF uses gifski (collector + writer on separate threads — required by its
//! API). Export reads pixels through its own read-only spool handle, so the
//! editor stays responsive while encoding: spool data is append-only and
//! existing offsets never move.

use crate::session::{Frame, Session};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    Gif,
    Apng,
    PngSeq,
    Mp4,
    Webm,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub path: String,
    pub quality: u8,
    pub width: Option<u32>,
    /// `loop` is a Rust keyword; the frontend sends "loop".
    #[serde(rename = "loop")]
    pub loop_: Option<bool>,
    pub fast: bool,
}

impl ExportSettings {
    fn loops(&self) -> bool {
        self.loop_.unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub current: usize,
    pub total: usize,
    pub stage: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub type ProgressFn = Arc<dyn Fn(ExportProgress) + Send + Sync>;

/// Immutable copy of everything the export threads need; taken under the
/// session lock, used without it. Holds its own open spool handle so the
/// export keeps working even if the session is replaced (and its spool
/// deleted) mid-export — Windows share-delete semantics keep reads valid.
pub struct ExportPlan {
    pub frames: Vec<Frame>,
    pub width: u32,
    pub height: u32,
    pub spool_path: PathBuf,
    spool: File,
}

impl ExportPlan {
    pub fn from_session(session: &Session) -> Result<Self, String> {
        let spool_path = session.spool_path().to_path_buf();
        let spool =
            File::open(&spool_path).map_err(|e| format!("open spool: {e}"))?;
        Ok(Self {
            frames: session.frames.clone(),
            width: session.width,
            height: session.height,
            spool_path,
            spool,
        })
    }
}

fn read_frame_rgba(spool: &mut File, frame: &Frame) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; frame.pixels.len as usize];
    spool
        .seek(SeekFrom::Start(frame.pixels.offset))
        .and_then(|_| spool.read_exact(&mut buf))
        .map_err(|e| format!("spool read: {e}"))?;
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2); // BGRA -> RGBA
    }
    Ok(buf)
}

#[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
struct Reporter {
    progress: ProgressFn,
    total: usize,
    written: usize,
    cancel: Arc<AtomicBool>,
}

#[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
impl gifski::progress::ProgressReporter for Reporter {
    fn increase(&mut self) -> bool {
        self.written += 1;
        (self.progress)(ExportProgress {
            current: self.written,
            total: self.total,
            stage: "encoding",
            message: None,
        });
        !self.cancel.load(Ordering::Relaxed)
    }
}

/// gifski core: encodes `frames` to any `Write` sink. No file management — the
/// caller owns the sink's lifecycle (real export wraps a file; size estimation
/// wraps a byte counter). Runs the writer on its own thread per gifski's API.
#[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
fn gif_encode_gifski<W: Write + Send + 'static>(
    spool: &mut File,
    frames: &[Frame],
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
    out: W,
) -> Result<(), String> {
    let gif_settings = gifski::Settings {
        width: settings.width,
        height: None,
        quality: settings.quality.clamp(1, 100),
        fast: settings.fast,
        repeat: if settings.loops() {
            gifski::Repeat::Infinite
        } else {
            gifski::Repeat::Finite(0)
        },
    };
    let (collector, writer) =
        gifski::new(gif_settings).map_err(|e| format!("gifski init: {e}"))?;

    let total = frames.len();
    let mut reporter = Reporter {
        progress: Arc::clone(progress),
        total,
        written: 0,
        cancel: Arc::clone(cancel),
    };
    let writer_thread = std::thread::spawn(move || writer.write(out, &mut reporter));

    let mut pts = 0.0_f64;
    let mut collect_err = None;
    for (i, frame) in frames.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let rgba = match read_frame_rgba(spool, frame) {
            Ok(b) => b,
            Err(e) => {
                collect_err = Some(e);
                break;
            }
        };
        use rgb::FromSlice;
        let img = imgref::ImgVec::new(
            rgba.as_rgba().to_vec(),
            frame.pixels.width as usize,
            frame.pixels.height as usize,
        );
        if let Err(e) = collector.add_frame_rgba(i, img, pts) {
            collect_err = Some(format!("gifski frame: {e}"));
            break;
        }
        pts += frame.delay_ms as f64 / 1000.0;
    }
    drop(collector); // end of stream — lets the writer finish

    let write_result = writer_thread
        .join()
        .map_err(|_| "gifski writer thread panicked".to_string())?;

    if cancel.load(Ordering::Relaxed) {
        return Err("export cancelled".into());
    }
    if let Some(e) = collect_err {
        return Err(e);
    }
    write_result.map_err(|e| format!("gifski write: {e}"))
}

#[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
fn export_gif(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    let out =
        BufWriter::new(File::create(&settings.path).map_err(|e| format!("create output: {e}"))?);
    // Move frames out so the spool (mut) and the frame slice (shared) can be
    // borrowed at once, then put them back.
    let frames = std::mem::take(&mut plan.frames);
    let result = gif_encode_gifski(&mut plan.spool, &frames, settings, cancel, progress, out);
    plan.frames = frames;
    if result.is_err() {
        let _ = std::fs::remove_file(&settings.path);
    }
    result
}

/// Compat core: encodes `frames` to any `Write` sink (see `gif_encode_gifski`).
fn gif_encode_compat<W: Write>(
    spool: &mut File,
    frames: &[Frame],
    width: u32,
    height: u32,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
    out: W,
) -> Result<(), String> {
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return Err("frame dimensions exceed the GIF format limit".into());
    }
    let total = frames.len();
    let mut out = out;
    let mut encoder = gif::Encoder::new(&mut out, width as u16, height as u16, &[])
        .map_err(|e| format!("gif init: {e}"))?;
    encoder
        .set_repeat(if settings.loops() {
            gif::Repeat::Infinite
        } else {
            gif::Repeat::Finite(0)
        })
        .map_err(|e| e.to_string())?;

    // quality 1..=100 -> NeuQuant speed 30..=1 (1 = best); fast mode floors it.
    let quality = settings.quality.clamp(1, 100) as u32;
    let mut speed = (31 - (quality * 30).div_ceil(100)).clamp(1, 30) as i32;
    if settings.fast {
        speed = speed.max(10);
    }

    for (i, frame) in frames.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("export cancelled".into());
        }
        let mut rgba = read_frame_rgba(spool, frame)?;
        let mut gif_frame = gif::Frame::from_rgba_speed(
            frame.pixels.width as u16,
            frame.pixels.height as u16,
            &mut rgba,
            speed,
        );
        // GIF delays are centiseconds.
        gif_frame.delay = (frame.delay_ms / 10).clamp(1, u16::MAX as u32) as u16;
        encoder
            .write_frame(&gif_frame)
            .map_err(|e| format!("gif frame: {e}"))?;
        progress(ExportProgress { current: i + 1, total, stage: "encoding", message: None });
    }
    Ok(())
}

/// Fallback GIF encoder built on the MIT-licensed `gif` crate (NeuQuant
/// per-frame palettes). Lower fidelity than gifski, but license-clean for the
/// Mac App Store — it is the only GIF path compiled into macOS builds.
pub fn export_gif_compat(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    let (w, h) = (plan.width, plan.height);
    let out = BufWriter::new(File::create(&settings.path).map_err(|e| format!("create output: {e}"))?);
    let frames = std::mem::take(&mut plan.frames);
    let result = gif_encode_compat(&mut plan.spool, &frames, w, h, settings, cancel, progress, out);
    plan.frames = frames;
    if result.is_err() {
        let _ = std::fs::remove_file(&settings.path);
    }
    result
}

fn export_apng(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    // The APNG header carries one canvas size; refuse mismatched frames
    // up front instead of failing mid-encode.
    if let Some(f) = plan
        .frames
        .iter()
        .find(|f| f.pixels.width != plan.width || f.pixels.height != plan.height)
    {
        return Err(format!(
            "frame {} ({}×{}) does not match the session size {}×{} — APNG needs uniform frames",
            f.id, f.pixels.width, f.pixels.height, plan.width, plan.height
        ));
    }
    let total = plan.frames.len();
    let file = File::create(&settings.path).map_err(|e| format!("create output: {e}"))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), plan.width, plan.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Balanced);
    encoder
        .set_animated(total as u32, if settings.loops() { 0 } else { 1 })
        .map_err(|e| e.to_string())?;
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;

    for (i, frame) in plan.frames.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&settings.path);
            return Err("export cancelled".into());
        }
        let rgba = read_frame_rgba(&mut plan.spool, frame)?;
        writer
            .set_frame_delay(frame.delay_ms.min(u16::MAX as u32) as u16, 1000)
            .map_err(|e| e.to_string())?;
        writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
        progress(ExportProgress { current: i + 1, total, stage: "encoding", message: None });
    }
    writer.finish().map_err(|e| e.to_string())
}

fn export_png_seq(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    let base = Path::new(&settings.path);
    let dir = base.parent().ok_or("invalid output path")?;
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("frame");

    let total = plan.frames.len();
    // The save dialog only confirmed overwriting `stem.png` — never silently
    // clobber the sibling numbered files this export is about to write.
    for i in 1..=total {
        let path = dir.join(format!("{stem}_{i:04}.png"));
        if path.exists() {
            return Err(format!(
                "{} already exists — choose a different name or an empty folder",
                path.display()
            ));
        }
    }
    for (i, frame) in plan.frames.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("export cancelled".into());
        }
        let rgba = read_frame_rgba(&mut plan.spool, frame)?;
        let path = dir.join(format!("{stem}_{:04}.png", i + 1));
        let file = File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
        let mut encoder =
            png::Encoder::new(BufWriter::new(file), frame.pixels.width, frame.pixels.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Balanced);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        progress(ExportProgress { current: i + 1, total, stage: "writing", message: None });
    }
    Ok(())
}

/// MP4 (H.264) export through Windows Media Foundation — no ffmpeg needed.
/// WebM stays unsupported: stock Windows ships no VP9 encoder MFT.
#[cfg(windows)]
pub fn export_mp4(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    use windows_capture::encoder::{
        AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
        VideoSettingsSubType,
    };

    // The encoder rides on WinRT MediaTranscoder; make sure COM/WinRT is up
    // on this worker thread (RPC_E_CHANGED_MODE just means it already is).
    unsafe {
        let _ = windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        );
    }

    // H.264 wants even dimensions — crop a trailing row/column if needed.
    let src_w = plan.width;
    let src_h = plan.height;
    let w = src_w & !1;
    let h = src_h & !1;
    if w < 2 || h < 2 {
        return Err("frames are too small for video export".into());
    }

    let fps = {
        // Median-ish frame rate from delays; the container just needs a hint.
        let avg_delay = plan
            .frames
            .iter()
            .map(|f| f.delay_ms as u64)
            .sum::<u64>()
            .max(1)
            / plan.frames.len().max(1) as u64;
        (1000 / avg_delay.clamp(10, 1000)).clamp(1, 60) as u32
    };
    let bitrate = ((w as u64 * h as u64 * fps as u64) / 10).clamp(2_000_000, 40_000_000) as u32;

    let mut encoder = VideoEncoder::new(
        VideoSettingsBuilder::new(w, h)
            .sub_type(VideoSettingsSubType::H264) // HEVC default can be unlicensed
            .frame_rate(fps)
            .bitrate(bitrate),
        AudioSettingsBuilder::new().disabled(true),
        ContainerSettingsBuilder::new(), // MPEG4
        &settings.path,
    )
    .map_err(|e| {
        let mut msg = format!("video encoder init: {e}");
        if msg.contains("C00D6D60") || msg.contains("c00d6d60") {
            msg.push_str(
                " — the H.264 encoder rejected this frame size; very small recordings \
                 may need Resize to roughly 128×96 or larger first",
            );
        }
        msg
    })?;

    let total = plan.frames.len();
    let mut ts_100ns: i64 = 0;
    for (i, frame) in plan.frames.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            drop(encoder); // best-effort teardown
            let _ = std::fs::remove_file(&settings.path);
            return Err("export cancelled".into());
        }
        let bgra = {
            let mut buf = vec![0u8; frame.pixels.len as usize];
            plan.spool
                .seek(SeekFrom::Start(frame.pixels.offset))
                .and_then(|_| plan.spool.read_exact(&mut buf))
                .map_err(|e| format!("spool read: {e}"))?;
            buf // MF wants BGRA — exactly what the spool stores
        };
        // MF's uncompressed-RGB path expects bottom-up rows; ours are top-down.
        let src_row = src_w as usize * 4;
        let dst_row = w as usize * 4;
        let mut flipped = vec![0u8; dst_row * h as usize];
        for y in 0..h as usize {
            let src_y = h as usize - 1 - y;
            flipped[y * dst_row..(y + 1) * dst_row]
                .copy_from_slice(&bgra[src_y * src_row..src_y * src_row + dst_row]);
        }
        encoder
            .send_frame_buffer(&flipped, ts_100ns)
            .map_err(|e| format!("video frame {i}: {e}"))?;
        ts_100ns += frame.delay_ms as i64 * 10_000;

        // send_frame_buffer queues on an unbounded channel — pace disk reads
        // so long clips don't balloon RAM while the transcoder catches up.
        if i % 24 == 23 {
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        progress(ExportProgress { current: i + 1, total, stage: "encoding", message: None });
    }
    progress(ExportProgress { current: total, total, stage: "writing", message: None });
    encoder
        .finish()
        .map_err(|e| format!("video finalize: {e}"))
}

/// Synchronous export used by tests and the background thread in `run`.
pub fn execute(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<(), String> {
    match settings.format {
        #[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
        ExportFormat::Gif => export_gif(plan, settings, cancel, progress),
        #[cfg(not(all(feature = "gifski-encoder", not(target_os = "macos"))))]
        ExportFormat::Gif => export_gif_compat(plan, settings, cancel, progress),
        ExportFormat::Apng => export_apng(plan, settings, cancel, progress),
        ExportFormat::PngSeq => export_png_seq(plan, settings, cancel, progress),
        #[cfg(windows)]
        ExportFormat::Mp4 => export_mp4(plan, settings, cancel, progress),
        #[cfg(not(windows))]
        ExportFormat::Mp4 => Err("MP4 export is not available on this platform yet".into()),
        ExportFormat::Webm => {
            Err("WebM export is not supported (no VP9 encoder in Windows Media Foundation)".into())
        }
    }
}

// ------------------------------------------------------------- size estimation

/// A `Write` sink that discards bytes and only tallies their count, so a GIF
/// can be encoded "to nowhere" purely to measure its size.
struct ByteCounter {
    count: Arc<AtomicU64>,
}

impl Write for ByteCounter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.count.fetch_add(buf.len() as u64, Ordering::Relaxed);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Encodes `frames` as GIF to `out` through whichever encoder this build ships
/// (gifski when the feature is on and off macOS, else the compat encoder) —
/// mirrors `execute`'s Gif gating exactly so estimates track real exports.
fn gif_encode_active<W: Write + Send + 'static>(
    spool: &mut File,
    frames: &[Frame],
    width: u32,
    height: u32,
    settings: &ExportSettings,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
    out: W,
) -> Result<(), String> {
    #[cfg(all(feature = "gifski-encoder", not(target_os = "macos")))]
    {
        let _ = (width, height);
        gif_encode_gifski(spool, frames, settings, cancel, progress, out)
    }
    #[cfg(not(all(feature = "gifski-encoder", not(target_os = "macos"))))]
    {
        gif_encode_compat(spool, frames, width, height, settings, cancel, progress, out)
    }
}

/// Empirical correction. A sparse frame sample loses the inter-frame
/// compression the full clip enjoys (sampled frames differ more), so the naive
/// extrapolation runs high — most at the default quality, worse as quality
/// drops. Tuned against the make_test_project demo (60f, 480×300): at the
/// dialog's default quality this lands the estimate within ±20% of a real
/// export. Content-dependent, so it stays a rough figure by design.
const ESTIMATE_CORRECTION: f64 = 0.85;

/// Estimates the byte size of a GIF export without writing the file: encodes a
/// sample of ~12–16 evenly-spaced frames at the exact quality/width/fast
/// settings, then extrapolates to the full frame count. Opens nothing new — it
/// reuses the plan's own read-only spool handle, exactly like a real export.
pub fn estimate_gif_size(
    plan: &mut ExportPlan,
    settings: &ExportSettings,
) -> Result<u64, String> {
    let n = plan.frames.len();
    if n == 0 {
        return Err("no frames to estimate".into());
    }
    // Encode every k-th frame so ~14 frames get sampled regardless of length.
    let k = (n / 14).max(1);
    let sampled: Vec<Frame> = plan.frames.iter().copied().step_by(k).collect();
    let sampled_count = sampled.len().max(1);

    let count = Arc::new(AtomicU64::new(0));
    let sink = ByteCounter { count: Arc::clone(&count) };
    let cancel = Arc::new(AtomicBool::new(false));
    let progress: ProgressFn = Arc::new(|_| {});
    let (w, h) = (plan.width, plan.height);
    gif_encode_active(&mut plan.spool, &sampled, w, h, settings, &cancel, &progress, sink)?;

    let sample_bytes = count.load(Ordering::Relaxed);
    let estimated =
        sample_bytes as f64 * (n as f64 / sampled_count as f64) * ESTIMATE_CORRECTION;
    Ok(estimated.round() as u64)
}

/// Searches for export settings whose GIF lands under `target_bytes`: binary-
/// searches quality in 30..=100 for the highest that fits; if even quality 30
/// overshoots it steps the width down in 10% increments (floor: 50% of source)
/// and re-searches. Returns the chosen settings — which may still exceed the
/// target if it is impossible even at the floor, in which case the caller
/// exports at the floor and warns. Emits "estimating" progress ticks.
pub fn fit_to_target(
    plan: &mut ExportPlan,
    base_settings: &ExportSettings,
    target_bytes: u64,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
) -> Result<ExportSettings, String> {
    let source_w = plan.width;
    let min_w = (source_w / 2).max(1);

    // Width candidates: the user's chosen width first, then 10% steps down to
    // the 50%-of-source floor.
    let mut widths: Vec<Option<u32>> = vec![base_settings.width];
    let mut w = base_settings.width.unwrap_or(source_w).min(source_w);
    while w > min_w {
        let next = ((w * 9) / 10).max(min_w);
        if next >= w {
            break;
        }
        widths.push(Some(next));
        w = next;
    }

    let total_ticks = widths.len() * 6;
    let mut tick = 0usize;
    let mut floor_width: Option<u32> = base_settings.width;

    for width in widths {
        if cancel.load(Ordering::Relaxed) {
            return Err("export cancelled".into());
        }
        floor_width = width;
        // Highest quality in [30, 100] whose estimate fits the target.
        let mut lo = 30u8;
        let mut hi = 100u8;
        let mut best: Option<u8> = None;
        for _ in 0..6 {
            if lo > hi {
                break;
            }
            let mid = lo + (hi - lo) / 2;
            let mut probe = base_settings.clone();
            probe.format = ExportFormat::Gif;
            probe.width = width;
            probe.quality = mid;
            if cancel.load(Ordering::Relaxed) {
                return Err("export cancelled".into());
            }
            let est = estimate_gif_size(plan, &probe)?;
            tick += 1;
            progress(ExportProgress {
                current: tick,
                total: total_ticks,
                stage: "estimating",
                message: None,
            });
            if est <= target_bytes {
                best = Some(mid);
                lo = mid + 1;
            } else if mid == 0 {
                break;
            } else {
                hi = mid - 1;
            }
        }
        if let Some(q) = best {
            let mut chosen = base_settings.clone();
            chosen.format = ExportFormat::Gif;
            chosen.width = width;
            chosen.quality = q;
            return Ok(chosen);
        }
    }

    // Nothing fit even at the smallest width — return the floor and let the
    // caller export + warn on the real size.
    let mut chosen = base_settings.clone();
    chosen.format = ExportFormat::Gif;
    chosen.width = floor_width;
    chosen.quality = 30;
    Ok(chosen)
}

/// Structured outcome of a target-fit export, emitted on `export://fit-result`
/// so the UI can show the settings it landed on (and whether the target held).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitResult {
    pub path: String,
    pub quality: u8,
    pub width: u32,
    pub bytes: u64,
    pub target_bytes: u64,
    pub over: bool,
}

/// Spawns a target-fit export: searches for settings that fit `target_bytes`,
/// runs the real export with them, then emits `export://fit-result` plus the
/// usual `export://progress` "done". Mirrors `run`'s threading/ownership.
pub fn run_fit(
    app: AppHandle,
    session: &Session,
    base_settings: ExportSettings,
    target_bytes: u64,
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    let mut plan = match ExportPlan::from_session(session) {
        Ok(p) => p,
        Err(e) => {
            running.store(false, Ordering::Relaxed);
            return Err(e);
        }
    };
    let source_w = plan.width;
    std::thread::spawn(move || {
        let emitter = app.clone();
        let progress: ProgressFn = Arc::new(move |p: ExportProgress| {
            let _ = emitter.emit("export://progress", p);
        });
        let outcome = (|| -> Result<(ExportSettings, u64), String> {
            let chosen = fit_to_target(&mut plan, &base_settings, target_bytes, &cancel, &progress)?;
            execute(&mut plan, &chosen, &cancel, &progress)?;
            let bytes = std::fs::metadata(&chosen.path).map(|m| m.len()).unwrap_or(0);
            Ok((chosen, bytes))
        })();
        running.store(false, Ordering::Relaxed);
        match outcome {
            Ok((chosen, bytes)) => {
                let _ = app.emit(
                    "export://fit-result",
                    FitResult {
                        path: chosen.path.clone(),
                        quality: chosen.quality,
                        width: chosen.width.unwrap_or(source_w),
                        bytes,
                        target_bytes,
                        over: bytes > target_bytes,
                    },
                );
                progress(ExportProgress {
                    current: 1,
                    total: 1,
                    stage: "done",
                    message: Some(chosen.path),
                });
            }
            Err(e) => progress(ExportProgress {
                current: 0,
                total: 1,
                stage: "error",
                message: Some(e),
            }),
        }
    });
    Ok(())
}

/// Spawns the export thread and wires progress to `export://progress` events.
/// The plan must be taken under the session lock by the caller; the export
/// itself never locks the session.
pub fn run(
    app: AppHandle,
    session: &Session,
    settings: ExportSettings,
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    let mut plan = match ExportPlan::from_session(session) {
        Ok(p) => p,
        Err(e) => {
            running.store(false, Ordering::Relaxed);
            return Err(e);
        }
    };
    std::thread::spawn(move || {
        let path = settings.path.clone();
        let emitter = app.clone();
        let progress: ProgressFn = Arc::new(move |p: ExportProgress| {
            let _ = emitter.emit("export://progress", p);
        });
        let result = execute(&mut plan, &settings, &cancel, &progress);
        running.store(false, Ordering::Relaxed);
        match result {
            Ok(()) => progress(ExportProgress {
                current: 1,
                total: 1,
                stage: "done",
                message: Some(path),
            }),
            Err(e) => progress(ExportProgress {
                current: 0,
                total: 1,
                stage: "error",
                message: Some(e),
            }),
        }
    });
    Ok(())
}
