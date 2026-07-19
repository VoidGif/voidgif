//! VoidGif — record your screen, edit every frame, export beautiful GIFs.

pub mod capture;
pub mod editor;
pub mod export;
mod frameserver;
pub mod gif_import;
pub mod project;
mod recorder;
pub mod session;
pub mod settings;
pub mod share;
mod state;

use capture::{CaptureConfig, Region};
use session::SessionInfo;
use state::{AppState, ArmedSetup, ContinueInsert, HoleRect};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

type CmdResult<T> = Result<T, String>;

// ---------------------------------------------------------------- monitors

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorInfo {
    id: usize,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
    is_primary: bool,
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> CmdResult<Vec<MonitorInfo>> {
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;
    let primary_pos = primary.as_ref().map(|m| *m.position());
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            id: i,
            name: m.name().cloned().unwrap_or_else(|| format!("Display {}", i + 1)),
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
            scale_factor: m.scale_factor(),
            is_primary: primary_pos.is_some_and(|p| p == *m.position()),
        })
        .collect())
}

// -------------------------------------------------- recorder frame window

/// The monitor containing the region's center.
fn monitor_for_region(app: &AppHandle, region: &Region) -> Option<tauri::Monitor> {
    let cx = region.x + region.width as i32 / 2;
    let cy = region.y + region.height as i32 / 2;
    app.available_monitors().ok().and_then(|monitors| {
        monitors.into_iter().find(|m| {
            let p = m.position();
            let s = m.size();
            cx >= p.x && cx < p.x + s.width as i32 && cy >= p.y && cy < p.y + s.height as i32
        })
    })
}

/// Clamp to the containing monitor: the frame window can hang off a screen
/// edge, and the session dimensions must match what the capture backend can
/// actually deliver.
fn clamp_region_to_monitor(app: &AppHandle, region: Region) -> CmdResult<Region> {
    let monitor =
        monitor_for_region(app, &region).ok_or("capture region is not on any monitor")?;
    let (mp, ms) = (*monitor.position(), *monitor.size());
    let ix0 = region.x.max(mp.x);
    let iy0 = region.y.max(mp.y);
    let ix1 = (region.x + region.width as i32).min(mp.x + ms.width as i32);
    let iy1 = (region.y + region.height as i32).min(mp.y + ms.height as i32);
    if ix1 - ix0 < 4 || iy1 - iy0 < 4 {
        return Err("capture region is too small".into());
    }
    Ok(Region {
        x: ix0,
        y: iy0,
        width: (ix1 - ix0) as u32,
        height: (iy1 - iy0) as u32,
    })
}

/// A FIXED-size capture region anchored at (x, y), used by continue-recording:
/// the size is locked to the existing session's dimensions so every captured
/// frame matches (and the merge can never be rejected for a size mismatch).
/// The position is nudged so the region stays fully on its monitor; if the
/// monitor is smaller than the region it errors.
fn locked_region_at(app: &AppHandle, x: i32, y: i32, width: u32, height: u32) -> CmdResult<Region> {
    let probe = Region { x, y, width, height };
    let monitor =
        monitor_for_region(app, &probe).ok_or("capture region is not on any monitor")?;
    let (mp, ms) = (*monitor.position(), *monitor.size());
    if width > ms.width || height > ms.height {
        return Err("recording is larger than the screen".into());
    }
    let mut nx = x;
    let mut ny = y;
    if nx + width as i32 > mp.x + ms.width as i32 {
        nx = mp.x + ms.width as i32 - width as i32;
    }
    if ny + height as i32 > mp.y + ms.height as i32 {
        ny = mp.y + ms.height as i32 - height as i32;
    }
    nx = nx.max(mp.x);
    ny = ny.max(mp.y);
    Ok(Region { x: nx, y: ny, width, height })
}

// Last placement of the recorder frame window, persisted across runs.
// (A settings module will absorb this later — keep it a tiny fn pair.)
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct RecorderGeom {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn recorder_geom_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("recorder-window.json"))
}

/// Restores the last frame-window placement. None unless the saved rect still
/// lands on a live monitor — monitor layouts change between sessions.
fn load_recorder_geom(app: &AppHandle) -> Option<RecorderGeom> {
    let raw = std::fs::read_to_string(recorder_geom_path(app)?).ok()?;
    let g: RecorderGeom = serde_json::from_str(&raw).ok()?;
    if g.width < 100 || g.height < 100 {
        return None;
    }
    let center = Region { x: g.x, y: g.y, width: g.width, height: g.height };
    monitor_for_region(app, &center).is_some().then_some(g)
}

fn save_recorder_geom(app: &AppHandle) {
    let Some(win) = app.get_webview_window("recorder") else { return };
    let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else { return };
    let Some(path) = recorder_geom_path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let g = RecorderGeom { x: pos.x, y: pos.y, width: size.width, height: size.height };
    if let Ok(json) = serde_json::to_string(&g) {
        let _ = std::fs::write(path, json);
    }
}

/// Physical screen rect of the hole, derived live from the window placement.
fn hole_screen_rect(win: &tauri::WebviewWindow, hole: &HoleRect) -> Option<Region> {
    let pos = win.inner_position().ok()?;
    let scale = win.scale_factor().ok()?;
    Some(Region {
        x: pos.x + (hole.x * scale).round() as i32,
        y: pos.y + (hole.y * scale).round() as i32,
        width: (hole.width * scale).round().max(0.0) as u32,
        height: (hole.height * scale).round().max(0.0) as u32,
    })
}

/// ~30Hz cursor poller: the transparent hole must pass clicks to the apps
/// underneath while the frame and control bar stay interactive, but
/// set_ignore_cursor_events is all-or-nothing — so toggle it by whether the
/// cursor is currently over the hole. Exits when the window closes or a newer
/// poller generation supersedes this one.
#[cfg(windows)]
fn spawn_hole_poller(app: AppHandle, gen: u64) {
    std::thread::spawn(move || {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut last: Option<bool> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(33));
            let state = app.state::<AppState>();
            if state.poller_gen.load(Ordering::Relaxed) != gen {
                break;
            }
            let Some(win) = app.get_webview_window("recorder") else { break };
            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt) }.is_err() {
                continue;
            }
            let hole = *state.hole_rect.lock().unwrap();
            let inside = hole
                .and_then(|h| hole_screen_rect(&win, &h))
                .is_some_and(|r| {
                    pt.x >= r.x
                        && pt.x < r.x + r.width as i32
                        && pt.y >= r.y
                        && pt.y < r.y + r.height as i32
                });
            if last != Some(inside) && win.set_ignore_cursor_events(inside).is_ok() {
                last = Some(inside);
            }
        }
    });
}

#[cfg(not(windows))]
fn spawn_hole_poller(_app: AppHandle, _gen: u64) {
    // TODO(mac): NSEvent-based cursor tracking when the macOS port lands.
}

// Async so window creation runs off the main thread: a sync #[tauri::command]
// that builds a webview window deadlocks on Windows (documented Tauri caveat).
#[tauri::command]
async fn open_recorder(
    app: AppHandle,
    state: State<'_, AppState>,
    fps: u32,
    show_cursor: bool,
) -> CmdResult<()> {
    if state.recording.lock().unwrap().is_some() {
        return Err("already recording".into());
    }
    if let Some(win) = app.get_webview_window("recorder") {
        let _ = win.set_focus();
        return Ok(());
    }
    *state.armed.lock().unwrap() = Some(ArmedSetup {
        fps: fps.clamp(1, 60),
        show_cursor,
        full_screen: false,
        continue_insert: None,
    });
    *state.hole_rect.lock().unwrap() = None;

    let saved = load_recorder_geom(&app);
    let built = WebviewWindowBuilder::new(&app, "recorder", WebviewUrl::App("index.html".into()))
        .title("VoidGif recorder")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(false)
        .resizable(true)
        .inner_size(520.0, 420.0)
        .min_inner_size(240.0, 200.0)
        .center()
        .visible(false)
        .build();
    let win = match built {
        Ok(w) => w,
        Err(e) => {
            *state.armed.lock().unwrap() = None;
            return Err(format!("failed to open recorder window: {e}"));
        }
    };
    // Restore last placement with PHYSICAL coordinates: logical builder
    // positions are resolved against whichever monitor tao matches first,
    // which lands on the wrong monitor in mixed-DPI setups.
    if let Some(g) = saved {
        let _ = win.set_position(tauri::PhysicalPosition::new(g.x, g.y));
        let _ = win.set_size(tauri::PhysicalSize::new(g.width, g.height));
    }
    let _ = win.show();
    let _ = win.set_focus();
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    let gen = state.poller_gen.fetch_add(1, Ordering::Relaxed) + 1;
    spawn_hole_poller(app.clone(), gen);
    Ok(())
}

// Chrome around the transparent hole, in CSS pixels, mirroring RecorderPanel's
// fixed layout: horizontally 10px grab edge + 2px border on each side (24);
// vertically 10px top edge + 4px border (top+bottom) + 56px control bar (70).
// Keep in sync with RecorderPanel.tsx.
const HOLE_CHROME_W: f64 = 24.0;
const HOLE_CHROME_H: f64 = 70.0;

/// Opens the recorder in CONTINUE mode: the capture hole is locked to the
/// current session's dimensions (movable but not resizable, no full-screen),
/// and on stop the new frames are merged into the session at `insert`.
/// `insert` is "start" | "after" | "end"; `after_frame_id` is required for
/// "after". Async per the window-command caveat.
#[tauri::command]
async fn open_recorder_continue(
    app: AppHandle,
    state: State<'_, AppState>,
    insert: String,
    after_frame_id: Option<u64>,
) -> CmdResult<()> {
    if state.recording.lock().unwrap().is_some() {
        return Err("already recording".into());
    }
    let (sw, sh, fps) = {
        let guard = state.session.lock().unwrap();
        let s = guard.as_ref().ok_or("no session to continue")?;
        (s.width, s.height, s.fps)
    };
    let continue_insert = match insert.as_str() {
        "start" => ContinueInsert::Start,
        "end" => ContinueInsert::End,
        "after" => ContinueInsert::AfterFrame(
            after_frame_id.ok_or("after_frame_id is required for 'after'")?,
        ),
        other => return Err(format!("unknown insert position: {other}")),
    };
    if let Some(win) = app.get_webview_window("recorder") {
        let _ = win.set_focus();
        return Ok(());
    }

    let show_cursor = settings::load(&app).map(|s| s.default_cursor).unwrap_or(true);
    *state.armed.lock().unwrap() = Some(ArmedSetup {
        fps: fps.clamp(1, 60),
        show_cursor,
        full_screen: false,
        continue_insert: Some(continue_insert),
    });
    *state.hole_rect.lock().unwrap() = None;

    let saved = load_recorder_geom(&app);
    let built = WebviewWindowBuilder::new(&app, "recorder", WebviewUrl::App("index.html".into()))
        .title("VoidGif recorder")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(false)
        .resizable(false)
        .inner_size(520.0, 420.0)
        .visible(false)
        .build();
    let win = match built {
        Ok(w) => w,
        Err(e) => {
            *state.armed.lock().unwrap() = None;
            return Err(format!("failed to open recorder window: {e}"));
        }
    };
    // Position first (remembered placement or centered), then size the window
    // so the hole lands at ~session dims. Capture forces the EXACT dims later,
    // so sub-pixel chrome rounding here is only cosmetic.
    if let Some(g) = saved {
        let _ = win.set_position(tauri::PhysicalPosition::new(g.x, g.y));
    } else {
        let _ = win.center();
    }
    let scale = win.scale_factor().unwrap_or(1.0);
    let chrome_w = (HOLE_CHROME_W * scale).round() as u32;
    let chrome_h = (HOLE_CHROME_H * scale).round() as u32;
    if let Some(mon) = win.current_monitor().ok().flatten() {
        let ms = mon.size();
        if sw + chrome_w > ms.width || sh + chrome_h > ms.height {
            *state.armed.lock().unwrap() = None;
            let _ = win.close();
            return Err("recording is larger than the screen".into());
        }
    }
    let _ = win.set_size(tauri::PhysicalSize::new(sw + chrome_w, sh + chrome_h));
    let _ = win.show();
    let _ = win.set_focus();
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    let gen = state.poller_gen.fetch_add(1, Ordering::Relaxed) + 1;
    spawn_hole_poller(app.clone(), gen);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameInfo {
    /// Hole size in physical pixels (the region that would be captured).
    hole_width: u32,
    hole_height: u32,
    /// Size of the monitor the frame window currently sits on.
    monitor_width: u32,
    monitor_height: u32,
}

/// The panel reports its transparent hole (CSS px, window-relative) whenever
/// layout changes; the click-through poller and capture both derive the
/// region from it. Returns physical sizes for the panel's readout.
#[tauri::command]
fn report_hole_rect(
    app: AppHandle,
    state: State<AppState>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> CmdResult<FrameInfo> {
    *state.hole_rect.lock().unwrap() = Some(HoleRect { x, y, width, height });
    let win = app
        .get_webview_window("recorder")
        .ok_or("recorder window not open")?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let (mw, mh) = win
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| (m.size().width, m.size().height))
        .unwrap_or((0, 0));
    Ok(FrameInfo {
        hole_width: (width * scale).round() as u32,
        hole_height: (height * scale).round() as u32,
        monitor_width: mw,
        monitor_height: mh,
    })
}

fn close_recording_windows(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("recorder") {
        let _ = w.close();
    }
}

/// Adjusts capture options of the armed setup from the recorder panel. Sync is
/// fine: no window work, just a field update behind the mutex. full_screen
/// lives here (not in the frontend) so the F7 hotkey honors the checkbox.
#[tauri::command]
fn update_recorder_options(
    state: State<AppState>,
    fps: u32,
    show_cursor: bool,
    full_screen: bool,
) -> CmdResult<()> {
    let mut guard = state.armed.lock().unwrap();
    let armed = guard.as_mut().ok_or("not armed")?;
    armed.fps = fps.clamp(1, 60);
    armed.show_cursor = show_cursor;
    armed.full_screen = full_screen;
    Ok(())
}

/// Shared arm→record path used by the command and the F7 hotkey. The region
/// comes from the frame window's hole (or the whole monitor in full-screen
/// mode). Armed setup is consumed only on a successful start so the panel
/// stays usable on failure.
fn start_from_frame_inner(app: &AppHandle) -> CmdResult<()> {
    let state = app.state::<AppState>();
    if state.recording.lock().unwrap().is_some() {
        return Err("already recording".into());
    }
    let (fps, show_cursor, full_screen, continue_insert) = {
        let guard = state.armed.lock().unwrap();
        let s = guard.as_ref().ok_or("not armed")?;
        (s.fps, s.show_cursor, s.full_screen, s.continue_insert)
    };
    let win = app
        .get_webview_window("recorder")
        .ok_or("recorder window not open")?;
    save_recorder_geom(app);

    let region = if continue_insert.is_some() {
        // Continue mode: lock the capture to the session's exact dimensions,
        // positioned at the hole's top-left. full_screen is ignored here.
        let (sw, sh) = {
            let guard = state.session.lock().unwrap();
            let s = guard.as_ref().ok_or("session to continue is gone")?;
            (s.width, s.height)
        };
        let hole = state
            .hole_rect
            .lock()
            .unwrap()
            .ok_or("frame layout not reported yet")?;
        let raw = hole_screen_rect(&win, &hole).ok_or("cannot resolve frame position")?;
        locked_region_at(app, raw.x, raw.y, sw, sh)?
    } else if full_screen {
        let m = win
            .current_monitor()
            .map_err(|e| e.to_string())?
            .ok_or("recorder window is not on any monitor")?;
        Region {
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
        }
    } else {
        let hole = state
            .hole_rect
            .lock()
            .unwrap()
            .ok_or("frame layout not reported yet")?;
        let raw = hole_screen_rect(&win, &hole).ok_or("cannot resolve frame position")?;
        clamp_region_to_monitor(app, raw)?
    };

    let config = CaptureConfig { region, fps, show_cursor };
    let mut recording = recorder::start(app.clone(), config)?;
    recording.continue_insert = continue_insert;
    *state.recording.lock().unwrap() = Some(recording);
    *state.armed.lock().unwrap() = None;
    if full_screen {
        // The frame window must not appear in its own capture; user drives
        // the recording via F7 (pause) / F8 (stop) hotkeys while hidden.
        let _ = win.hide();
    } else {
        // Region is locked while recording.
        let _ = win.set_resizable(false);
    }
    let _ = app.emit("recorder://started", ());
    Ok(())
}

// Async per the window-command caveat (hides/locks the frame window).
#[tauri::command]
async fn start_recording_from_frame(app: AppHandle) -> CmdResult<()> {
    start_from_frame_inner(&app)
}

/// Shared cancel path (armed but not recording) used by the command and F8.
fn cancel_armed_inner(app: &AppHandle) -> CmdResult<()> {
    let state = app.state::<AppState>();
    if state.recording.lock().unwrap().is_some() {
        return Err("cannot cancel while recording".into());
    }
    if state.armed.lock().unwrap().take().is_none() {
        return Err("not armed".into());
    }
    save_recorder_geom(app);
    close_recording_windows(app);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
    let _ = app.emit_to("main", "recorder://discarded", ());
    Ok(())
}

#[tauri::command]
async fn cancel_armed(app: AppHandle) -> CmdResult<()> {
    cancel_armed_inner(&app)
}

fn set_paused(app: &AppHandle, paused: bool) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let guard = state.recording.lock().unwrap();
    let rec = guard.as_ref().ok_or("not recording")?;
    rec.flags.paused.store(paused, Ordering::Relaxed);
    let _ = app.emit("recorder://pausestate", paused);
    Ok(())
}

#[tauri::command]
fn pause_recording(app: AppHandle) -> CmdResult<()> {
    set_paused(&app, true)
}

#[tauri::command]
fn resume_recording(app: AppHandle) -> CmdResult<()> {
    set_paused(&app, false)
}

/// Shared stop flow used by the command, hotkey, and HUD.
fn finish_recording(app: &AppHandle, discard: bool) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let rec = state
        .recording
        .lock()
        .unwrap()
        .take()
        .ok_or("not recording")?;
    // Capture the continue-insert target before `stop` consumes the recording.
    let continue_insert = rec.continue_insert;

    close_recording_windows(app);
    let result = recorder::stop(rec);

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }

    match result {
        Ok(mut source) if !discard && !source.frames.is_empty() => {
            match continue_insert {
                // Continue-recording: merge into the existing session in place.
                Some(ci) => {
                    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
                    let Some(target) = guard.as_mut() else {
                        // The session vanished mid-recording — fall back to
                        // adopting the new frames as a fresh session.
                        drop(guard);
                        let info = source.info();
                        state.replace_session(Some(source));
                        let _ = app.emit_to("main", "recorder://stopped", info);
                        return Ok(());
                    };
                    let insert_at = match ci {
                        ContinueInsert::Start => 0,
                        ContinueInsert::End => target.frames.len(),
                        ContinueInsert::AfterFrame(id) => target
                            .frames
                            .iter()
                            .position(|f| f.id == id)
                            .map(|i| i + 1)
                            .unwrap_or(target.frames.len()),
                    };
                    let merged = session::merge_session(target, &mut source, insert_at);
                    let info = target.info();
                    drop(guard);
                    source.cleanup();
                    match merged {
                        Ok(()) => {
                            let _ = app.emit_to("main", "recorder://stopped", info);
                            Ok(())
                        }
                        Err(e) => {
                            let _ = app.emit_to("main", "recorder://discarded", ());
                            Err(e)
                        }
                    }
                }
                // Normal recording: replace the session.
                None => {
                    let info = source.info();
                    state.replace_session(Some(source));
                    let _ = app.emit_to("main", "recorder://stopped", info);
                    Ok(())
                }
            }
        }
        Ok(source) => {
            source.cleanup();
            let _ = app.emit_to("main", "recorder://discarded", ());
            Ok(())
        }
        Err(e) => {
            let _ = app.emit_to("main", "recorder://discarded", ());
            Err(e)
        }
    }
}

// Async per the window-command caveat (finish_recording closes windows and
// shows the main window).
#[tauri::command]
async fn stop_recording(app: AppHandle) -> CmdResult<()> {
    finish_recording(&app, false)
}

#[tauri::command]
async fn discard_recording(app: AppHandle) -> CmdResult<()> {
    finish_recording(&app, true)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RecorderStateInfo {
    /// "idle" | "armed" | "recording" | "paused"
    phase: String,
    fps: u32,
    show_cursor: bool,
    full_screen: bool,
    /// True for a continue-recording: the panel hides the full-screen checkbox
    /// and shows a "size locked" hint instead of an adjustable size readout.
    continue_mode: bool,
}

/// Lets a freshly-mounted recorder panel recover its state (the panel mounts
/// after open_recorder seeds the armed setup; also covers webview remounts).
#[tauri::command]
fn get_recorder_state(state: State<AppState>) -> CmdResult<RecorderStateInfo> {
    if let Some(rec) = state.recording.lock().unwrap().as_ref() {
        let paused = rec.flags.paused.load(Ordering::Relaxed);
        return Ok(RecorderStateInfo {
            phase: if paused { "paused" } else { "recording" }.into(),
            fps: rec.fps,
            show_cursor: rec.show_cursor,
            full_screen: false,
            continue_mode: rec.continue_insert.is_some(),
        });
    }
    if let Some(a) = state.armed.lock().unwrap().as_ref() {
        return Ok(RecorderStateInfo {
            phase: "armed".into(),
            fps: a.fps,
            show_cursor: a.show_cursor,
            full_screen: a.full_screen,
            continue_mode: a.continue_insert.is_some(),
        });
    }
    Ok(RecorderStateInfo {
        phase: "idle".into(),
        fps: 30,
        show_cursor: true,
        full_screen: false,
        continue_mode: false,
    })
}

// ------------------------------------------------------------------ editor

fn with_session<T>(
    state: &State<AppState>,
    f: impl FnOnce(&mut session::Session) -> Result<T, String>,
) -> CmdResult<T> {
    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let session = guard.as_mut().ok_or("no session loaded")?;
    f(session)
}

#[tauri::command]
fn get_session(state: State<AppState>) -> CmdResult<Option<SessionInfo>> {
    Ok(state.session.lock().unwrap().as_ref().map(|s| s.info()))
}

#[tauri::command]
fn delete_frames(state: State<AppState>, ids: Vec<u64>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::delete_frames(s, &ids)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn duplicate_frames(state: State<AppState>, ids: Vec<u64>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::duplicate_frames(s, &ids)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn reorder_frames(state: State<AppState>, order: Vec<u64>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::reorder_frames(s, &order)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn group_frames(state: State<AppState>, ids: Vec<u64>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::group_frames(s, &ids)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn ungroup(state: State<AppState>, group_id: u64) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::ungroup(s, group_id)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn move_group(state: State<AppState>, group_id: u64, to_index: usize) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::move_group(s, group_id, to_index)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn set_frame_delays(state: State<AppState>, ids: Vec<u64>, delay_ms: u32) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::set_frame_delays(s, &ids, delay_ms)?;
        Ok(s.info())
    })
}

// Async so these pixel-rewriting ops don't block the main thread; the session
// mutex serializes them safely against other editor commands.
#[tauri::command]
async fn crop_session(state: State<'_, AppState>, rect: Region) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::crop(s, rect)?;
        Ok(s.info())
    })
}

#[tauri::command]
async fn resize_session(state: State<'_, AppState>, width: u32, height: u32) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::resize(s, width, height)?;
        Ok(s.info())
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TrimResult {
    session: SessionInfo,
    removed: usize,
}

// Async: frame_diff reads full frame buffers off the spool, so keep the disk
// IO off the main thread (matching crop/resize).
#[tauri::command]
async fn trim_static_edges(state: State<'_, AppState>) -> CmdResult<TrimResult> {
    with_session(&state, |s| {
        let removed = editor::trim_static_edges(s, 0.004)?;
        Ok(TrimResult { session: s.info(), removed })
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MergeResult {
    session: SessionInfo,
    merged: usize,
}

#[tauri::command]
async fn merge_duplicates(state: State<'_, AppState>) -> CmdResult<MergeResult> {
    with_session(&state, |s| {
        let merged = editor::merge_duplicates(s, 0.002)?;
        Ok(MergeResult { session: s.info(), merged })
    })
}

#[tauri::command]
fn scale_delays(state: State<AppState>, ids: Vec<u64>, factor: f64) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::scale_delays(s, &ids, factor)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn make_pingpong(state: State<AppState>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::make_pingpong(s)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn undo_edit(state: State<AppState>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::undo(s)?;
        Ok(s.info())
    })
}

#[tauri::command]
fn redo_edit(state: State<AppState>) -> CmdResult<SessionInfo> {
    with_session(&state, |s| {
        editor::redo(s)?;
        Ok(s.info())
    })
}

// ------------------------------------------------------------------ export

#[tauri::command]
fn start_export(app: AppHandle, state: State<AppState>, settings: export::ExportSettings) -> CmdResult<()> {
    if state
        .export_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("an export is already running".into());
    }
    state.export_cancel.store(false, Ordering::SeqCst);

    let guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let Some(session) = guard.as_ref() else {
        state.export_running.store(false, Ordering::SeqCst);
        return Err("no session loaded".into());
    };
    export::run(
        app,
        session,
        settings,
        Arc::clone(&state.export_cancel),
        Arc::clone(&state.export_running),
    )
}

#[tauri::command]
fn cancel_export(state: State<AppState>) -> CmdResult<()> {
    state.export_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Estimates the byte size of a GIF export at the given settings. Uses its own
/// spool handle (like ExportPlan) so it never disturbs a running export;
/// concurrent estimates are serialized — the second returns `busy` and the UI
/// ignores it. The heavy encode runs on a blocking thread.
#[tauri::command]
async fn estimate_gif_size(
    state: State<'_, AppState>,
    settings: export::ExportSettings,
) -> CmdResult<u64> {
    if state
        .estimating
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("busy".into());
    }
    let plan = {
        match state.session.lock() {
            Ok(guard) => match guard.as_ref() {
                Some(s) => export::ExportPlan::from_session(s),
                None => Err("no session loaded".into()),
            },
            Err(_) => Err("session lock poisoned".into()),
        }
    };
    let result = match plan {
        Ok(mut plan) => tauri::async_runtime::spawn_blocking(move || {
            export::estimate_gif_size(&mut plan, &settings)
        })
        .await
        .unwrap_or_else(|e| Err(format!("estimate task failed: {e}"))),
        Err(e) => Err(e),
    };
    state.estimating.store(false, Ordering::SeqCst);
    result
}

/// Target-size auto-fit export: picks settings that land under `target_bytes`,
/// then runs the real export with them. Mirrors `start_export`'s running guard.
#[tauri::command]
fn start_export_fit(
    app: AppHandle,
    state: State<AppState>,
    settings: export::ExportSettings,
    target_bytes: u64,
) -> CmdResult<()> {
    if state
        .export_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("an export is already running".into());
    }
    state.export_cancel.store(false, Ordering::SeqCst);

    let guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let Some(session) = guard.as_ref() else {
        state.export_running.store(false, Ordering::SeqCst);
        return Err("no session loaded".into());
    };
    export::run_fit(
        app,
        session,
        settings,
        target_bytes,
        Arc::clone(&state.export_cancel),
        Arc::clone(&state.export_running),
    )
}

// ----------------------------------------------------------------- project

/// Session ids must be unique per load — a repeated id would truncate and
/// then delete the spool file still owned by the active session.
fn unique_session_id(prefix: &str) -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id())
}

// Async so serialization / disk IO stays off the main thread.
#[tauri::command]
async fn save_project(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<()> {
    with_session(&state, |s| project::save(s, &path))?;
    // The user just took ownership of their state — the crash-recovery copy is
    // now redundant.
    discard_autosave_file(&app);
    Ok(())
}

#[tauri::command]
async fn load_project(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<SessionInfo> {
    let dir = std::env::temp_dir().join("voidgif");
    let session = project::load(&path, &dir, unique_session_id("load"))?;
    let info = session.info();
    state.replace_session(Some(session));
    discard_autosave_file(&app);
    Ok(info)
}

/// Opens an existing GIF as a new editable session (composites its frames to
/// full BGRA). Mirrors `load_project`'s session-replace + autosave handling.
#[tauri::command]
async fn import_gif(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<SessionInfo> {
    let dir = std::env::temp_dir().join("voidgif");
    let session = gif_import::load(&path, &dir, unique_session_id("gif"))?;
    let info = session.info();
    state.replace_session(Some(session));
    discard_autosave_file(&app);
    Ok(info)
}

// --------------------------------------------------------------- autosave

/// Where the crash-recovery snapshot lives. `None` only if the OS gives us no
/// config dir at all.
fn autosave_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("autosave.voidgif"))
}

/// Best-effort delete of the autosave file (a no-op when absent).
fn discard_autosave_file(app: &AppHandle) {
    if let Some(path) = autosave_path(app) {
        let _ = std::fs::remove_file(path);
    }
}

/// UTC ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`) from a SystemTime, dependency-free
/// (Howard Hinnant's civil-from-days). The frontend renders it locale-aware.
fn iso8601_utc(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Saves the current session to the autosave path. Cheap `Ok(false)` no-op when
/// no session is loaded; `Ok(true)` when a snapshot was written.
#[tauri::command]
async fn autosave_now(app: AppHandle, state: State<'_, AppState>) -> CmdResult<bool> {
    let Some(path) = autosave_path(&app) else {
        return Err("no app config dir".into());
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let Some(session) = guard.as_mut() else {
        return Ok(false);
    };
    project::save(session, path.to_str().ok_or("invalid autosave path")?)?;
    Ok(true)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AutosaveInfo {
    /// ISO-8601 UTC timestamp of the last autosave (file mtime).
    saved_at: String,
    frames: u32,
}

/// Peeks the autosave file's header (no pixel decode) so the frontend can offer
/// recovery. `None` when there's nothing to recover or the file is unreadable.
#[tauri::command]
fn check_autosave(app: AppHandle) -> Option<AutosaveInfo> {
    let path = autosave_path(&app)?;
    let meta = std::fs::metadata(&path).ok()?;
    let saved_at = iso8601_utc(meta.modified().ok()?);
    let summary = project::peek(path.to_str()?).ok()?;
    Some(AutosaveInfo { saved_at, frames: summary.frames })
}

#[tauri::command]
fn discard_autosave(app: AppHandle) -> CmdResult<()> {
    discard_autosave_file(&app);
    Ok(())
}

/// Loads the autosave snapshot into the live session, then deletes it (recovery
/// is one-shot).
#[tauri::command]
async fn restore_autosave(app: AppHandle, state: State<'_, AppState>) -> CmdResult<SessionInfo> {
    let path = autosave_path(&app).ok_or("no app config dir")?;
    let dir = std::env::temp_dir().join("voidgif");
    let session = project::load(
        path.to_str().ok_or("invalid autosave path")?,
        &dir,
        unique_session_id("restore"),
    )?;
    let info = session.info();
    state.replace_session(Some(session));
    let _ = std::fs::remove_file(&path);
    Ok(info)
}

// ------------------------------------------------------ share (clipboard)

/// Puts the exported file on the clipboard as a CF_HDROP file drop so Ctrl+V
/// pastes the file itself into Explorer, Slack, Discord, etc.
#[tauri::command]
fn copy_file_to_clipboard(path: String) -> CmdResult<()> {
    share::copy_file_to_clipboard(&path)
}

/// Opens the OS file browser with `path` pre-selected.
#[tauri::command]
fn reveal_in_explorer(path: String) -> CmdResult<()> {
    share::reveal_in_explorer(&path)
}

// -------------------------------------------------- share (external pages)

/// Maps a fixed page key to its canonical https URL. The set is hardcoded so
/// the webview can only ever open these pages — never an arbitrary URL or path.
fn external_page_url(page: &str) -> CmdResult<&'static str> {
    match page {
        "source" => Ok("https://github.com/VoidGif/voidgif"),
        "notices" => Ok("https://github.com/VoidGif/voidgif/blob/main/THIRD-PARTY-NOTICES.md"),
        "privacy" => Ok("https://voidgif.github.io/privacy/"),
        "website" => Ok("https://voidgif.github.io"),
        other => Err(format!("unknown page: {other}")),
    }
}

/// Opens one of the fixed About-section pages in the default browser. Async per
/// the project rule that all commands are `async fn`.
#[tauri::command]
async fn open_external(page: String) -> CmdResult<()> {
    share::open_external(external_page_url(&page)?)
}

// ----------------------------------------------------------------- hotkeys

fn register_hotkeys(app: &AppHandle) {
    let shortcuts: [(&str, fn(&AppHandle)); 2] = [
        ("F7", |app| {
            let state = app.state::<AppState>();
            let paused = state
                .recording
                .lock()
                .unwrap()
                .as_ref()
                .map(|r| r.flags.paused.load(Ordering::Relaxed));
            match paused {
                // Recording/paused: toggle pause (no window work — event thread is fine).
                Some(p) => {
                    let _ = set_paused(app, !p);
                }
                // Armed: start recording. Spawn — window work + capture start
                // must stay off the event thread.
                None if state.armed.lock().unwrap().is_some() => {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = start_from_frame_inner(&app) {
                            log::warn!("hotkey start failed: {e}");
                        }
                    });
                }
                None => {}
            }
        }),
        ("F8", |app| {
            let state = app.state::<AppState>();
            let is_recording = state.recording.lock().unwrap().is_some();
            if is_recording {
                let app = app.clone();
                // stop joins capture threads — keep it off the event thread
                std::thread::spawn(move || {
                    if let Err(e) = finish_recording(&app, false) {
                        log::warn!("hotkey stop failed: {e}");
                    }
                });
            } else if state.armed.lock().unwrap().is_some() {
                let app = app.clone();
                // cancel_armed closes windows — keep it off the event thread
                std::thread::spawn(move || {
                    if let Err(e) = cancel_armed_inner(&app) {
                        log::warn!("hotkey cancel failed: {e}");
                    }
                });
            }
        }),
    ];

    for (keys, action) in shortcuts {
        let result = app.global_shortcut().on_shortcut(keys, move |app, _sc, event| {
            if event.state() == ShortcutState::Pressed {
                action(app);
            }
        });
        if let Err(e) = result {
            log::warn!("failed to register global shortcut {keys}: {e}");
        }
    }
}

// --------------------------------------------------------------------- tray

/// Brings the main window back to the foreground (from hidden or minimized).
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// Tray menu labels per app language. The tray is native UI outside the
/// webview, so it can't read the frontend dictionaries — keep these in sync
/// with src/i18n when adding a language.
fn tray_labels(lang: &str) -> (&'static str, &'static str) {
    match lang {
        "ko" => ("VoidGif 열기", "VoidGif 종료"),
        "ja" => ("VoidGif を開く", "VoidGif を終了"),
        _ => ("Open VoidGif", "Quit VoidGif"),
    }
}

fn tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let (open_label, quit_label) = tray_labels(lang);
    let open_item = MenuItem::with_id(app, "tray_open", open_label, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "tray_quit", quit_label, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&open_item, &separator, &quit_item])
}

/// Relabels the tray menu for the language the frontend resolved (explicit
/// setting or OS language — that resolution lives in src/i18n, so the tray
/// follows the webview rather than re-deriving it here).
#[tauri::command]
async fn set_tray_language(app: AppHandle, lang: String) -> CmdResult<()> {
    log::info!("set_tray_language({lang})");
    if !matches!(lang.as_str(), "ko" | "ja" | "en") {
        return Err(format!("invalid language: {lang}"));
    }
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Some(tray) = handle.tray_by_id("voidgif-tray") {
            match tray_menu(&handle, &lang) {
                Ok(menu) => match tray.set_menu(Some(menu)) {
                    Ok(()) => log::info!("tray menu relabeled to {lang}"),
                    Err(e) => log::warn!("failed to relabel tray menu: {e}"),
                },
                Err(e) => log::warn!("failed to build tray menu: {e}"),
            }
        }
    })
    .map_err(|e| e.to_string())
}

/// Builds the always-present system tray. This is a convenience while the app
/// runs — NOT a minimize-to-tray pattern: closing the main window quits the
/// whole process (see the on_window_event handler in `run`).
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    // Explicitly saved language, if any; when the setting is "follow the OS"
    // the frontend reports the resolved language right after boot.
    let lang = settings::load(app)
        .and_then(|s| s.language)
        .unwrap_or_else(|| "en".into());
    let menu = tray_menu(app, &lang)?;

    let mut builder = TrayIconBuilder::with_id("voidgif-tray")
        .tooltip("VoidGif")
        .menu(&menu)
        // Left click shows the window; the menu is right-click only.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray_open" => show_main_window(app),
            "tray_quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    log::info!("system tray created");
    Ok(())
}

// -------------------------------------------------------------------- run

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::default())
        .register_uri_scheme_protocol("voidframe", |ctx, request| frameserver::handle(ctx, request))
        .setup(|app| {
            register_hotkeys(app.handle());
            build_tray(app.handle())?;
            sweep_stale_spools();
            // Support `voidgif something.voidgif` (file association / CLI).
            if let Some(path) = std::env::args().nth(1).filter(|a| a.ends_with(".voidgif")) {
                let dir = std::env::temp_dir().join("voidgif");
                match project::load(&path, &dir, unique_session_id("cli")) {
                    Ok(session) => app.state::<AppState>().replace_session(Some(session)),
                    Err(e) => log::warn!("failed to open {path}: {e}"),
                }
            }
            Ok(())
        })
        // Closing the MAIN window terminates the whole process (owner wants
        // close = quit; the tray is a while-running convenience, not
        // minimize-to-tray). app.exit(0) fires RunEvent::Exit below, which
        // stops any in-flight recording and cleans up spool files.
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_monitors,
            open_recorder,
            open_recorder_continue,
            report_hole_rect,
            update_recorder_options,
            start_recording_from_frame,
            cancel_armed,
            get_recorder_state,
            pause_recording,
            resume_recording,
            stop_recording,
            discard_recording,
            get_session,
            delete_frames,
            duplicate_frames,
            reorder_frames,
            group_frames,
            ungroup,
            move_group,
            set_frame_delays,
            trim_static_edges,
            merge_duplicates,
            scale_delays,
            make_pingpong,
            crop_session,
            resize_session,
            undo_edit,
            redo_edit,
            start_export,
            cancel_export,
            estimate_gif_size,
            start_export_fit,
            save_project,
            load_project,
            import_gif,
            autosave_now,
            check_autosave,
            discard_autosave,
            restore_autosave,
            copy_file_to_clipboard,
            reveal_in_explorer,
            open_external,
            set_tray_language,
            settings::get_settings,
            settings::set_settings
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                // Stop an in-flight recording so its spool isn't orphaned.
                if let Some(rec) = state.recording.lock().unwrap().take() {
                    if let Ok(session) = recorder::stop(rec) {
                        session.cleanup();
                    }
                }
                // Remove the live session's spool from the temp dir.
                state.replace_session(None);
                // A clean exit means the user chose their fate — the crash
                // recovery snapshot is only for kills/crashes.
                discard_autosave_file(app);
            }
        });
}

/// Deletes spool files left behind by crashed/killed runs. Only touches
/// files older than a day so concurrent instances are never disturbed.
fn sweep_stale_spools() {
    let dir = std::env::temp_dir().join("voidgif");
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3600);
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "spool")
            && entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t < cutoff)
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::external_page_url;

    #[test]
    fn external_page_url_maps_known_and_rejects_unknown() {
        assert!(external_page_url("source").unwrap().starts_with("https://"));
        assert!(external_page_url("notices").unwrap().starts_with("https://"));
        assert!(external_page_url("privacy").unwrap().starts_with("https://"));
        assert!(external_page_url("website").unwrap().starts_with("https://"));
        // Anything outside the allowlist is refused before the shell is touched.
        assert!(external_page_url("").is_err());
        assert!(external_page_url("file:///etc/passwd").is_err());
        assert!(external_page_url("https://evil.example").is_err());
    }
}
