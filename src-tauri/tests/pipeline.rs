//! Headless integration tests for the session → editor → export pipeline,
//! plus a live smoke test of the Windows capture backend.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use voidgif_lib::capture::Region;
use voidgif_lib::export::{ExportFormat, ExportPlan, ExportSettings, ProgressFn};
use voidgif_lib::session::{self, Session};
use voidgif_lib::{editor, export, gif_import, project};

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("voidgif-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Builds a session of `n` solid-color BGRA frames with a moving gradient so
/// gifski has real work to do.
fn synthetic_session(dir: &std::path::Path, n: u32, w: u32, h: u32) -> Session {
    let mut session = Session::create(dir, format!("test-{n}x{w}x{h}"), w, h, 30).unwrap();
    for i in 0..n {
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let o = ((y * w + x) * 4) as usize;
                frame[o] = ((x * 255 / w) as u8).wrapping_add((i * 16) as u8); // B
                frame[o + 1] = (y * 255 / h) as u8; // G
                frame[o + 2] = (i * 32 % 255) as u8; // R
                frame[o + 3] = 255;
            }
        }
        session.append_frame(&frame, w, h, 33).unwrap();
    }
    session
}

fn no_progress() -> ProgressFn {
    Arc::new(|_| {})
}

#[test]
fn spool_roundtrip_preserves_pixels() {
    let dir = test_dir("spool");
    let mut session = synthetic_session(&dir, 3, 32, 24);
    let frame = session.frames[1];
    let pixels = session.read_pixels(frame.pixels).unwrap();
    assert_eq!(pixels.len(), 32 * 24 * 4);
    // First pixel of frame 1: B = 0 + 16, G = 0, R = 32, A = 255
    assert_eq!(&pixels[0..4], &[16, 0, 32, 255]);
}

#[test]
fn editor_ops_and_undo() {
    let dir = test_dir("editor");
    let mut s = synthetic_session(&dir, 5, 32, 24);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();

    editor::delete_frames(&mut s, &[ids[0]]).unwrap();
    assert_eq!(s.frames.len(), 4);

    editor::duplicate_frames(&mut s, &[ids[1]]).unwrap();
    assert_eq!(s.frames.len(), 5);
    assert_eq!(s.frames[0].id, ids[1]);
    assert_eq!(s.frames[1].pixels.offset, s.frames[0].pixels.offset);

    let reversed: Vec<u64> = s.frames.iter().rev().map(|f| f.id).collect();
    editor::reorder_frames(&mut s, &reversed).unwrap();
    assert_eq!(s.frames[0].id, reversed[0]);

    editor::set_frame_delays(&mut s, &[reversed[0]], 250).unwrap();
    assert_eq!(s.frames[0].delay_ms, 250);

    // Undo the delay change
    editor::undo(&mut s).unwrap();
    assert_ne!(s.frames[0].delay_ms, 250);
    editor::redo(&mut s).unwrap();
    assert_eq!(s.frames[0].delay_ms, 250);

    // Deleting every frame must fail
    let all: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    assert!(editor::delete_frames(&mut s, &all).is_err());
    session_cleanup(s);
}

#[test]
fn crop_and_resize_change_dimensions_and_pixels() {
    let dir = test_dir("croprs");
    let mut s = synthetic_session(&dir, 3, 64, 48);

    editor::crop(&mut s, Region { x: 8, y: 8, width: 32, height: 16 }).unwrap();
    assert_eq!((s.width, s.height), (32, 16));
    let px = s.read_pixels(s.frames[0].pixels).unwrap();
    assert_eq!(px.len(), 32 * 16 * 4);

    editor::resize(&mut s, 16, 8).unwrap();
    assert_eq!((s.width, s.height), (16, 8));
    let px = s.read_pixels(s.frames[0].pixels).unwrap();
    assert_eq!(px.len(), 16 * 8 * 4);

    // Undo twice returns to the original geometry
    editor::undo(&mut s).unwrap();
    editor::undo(&mut s).unwrap();
    assert_eq!((s.width, s.height), (64, 48));
    let px = s.read_pixels(s.frames[0].pixels).unwrap();
    assert_eq!(px.len(), 64 * 48 * 4);
    session_cleanup(s);
}

#[test]
fn project_save_load_roundtrip() {
    let dir = test_dir("project");
    let mut s = synthetic_session(&dir, 4, 40, 30);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    editor::duplicate_frames(&mut s, &[ids[2]]).unwrap(); // shared pixels on disk
    let original_first = s.read_pixels(s.frames[0].pixels).unwrap();

    let path = dir.join("roundtrip.voidgif");
    project::save(&mut s, path.to_str().unwrap()).unwrap();

    let mut loaded = project::load(path.to_str().unwrap(), &dir, "loaded".into()).unwrap();
    assert_eq!(loaded.frames.len(), 5);
    assert_eq!((loaded.width, loaded.height), (40, 30));
    assert_eq!(
        loaded.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>(),
        s.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>()
    );
    let loaded_first = loaded.read_pixels(loaded.frames[0].pixels).unwrap();
    assert_eq!(loaded_first, original_first);
    session_cleanup(s);
    session_cleanup(loaded);
}

#[test]
fn gif_export_produces_valid_gif() {
    let dir = test_dir("gif");
    let s = synthetic_session(&dir, 8, 64, 48);
    let out = dir.join("out.gif");
    let settings = ExportSettings {
        format: ExportFormat::Gif,
        path: out.to_str().unwrap().into(),
        quality: 90,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::execute(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"GIF89a"), "not a GIF89a file");
    assert!(bytes.len() > 500, "suspiciously small GIF");
    session_cleanup(s);
}

/// The MIT `gif`-crate fallback is the only GIF encoder on macOS builds —
/// it must produce a valid animated GIF regardless of the gifski feature.
#[test]
fn compat_gif_export_produces_valid_gif() {
    let dir = test_dir("gifcompat");
    let s = synthetic_session(&dir, 6, 64, 48);
    let out = dir.join("compat.gif");
    let settings = ExportSettings {
        format: ExportFormat::Gif,
        path: out.to_str().unwrap().into(),
        quality: 80,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::export_gif_compat(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"GIF89a"), "not a GIF89a file");
    // NETSCAPE2.0 extension marks the infinite loop.
    assert!(
        bytes.windows(11).any(|w| w == b"NETSCAPE2.0"),
        "loop extension missing"
    );
    session_cleanup(s);
}

#[test]
fn apng_export_produces_valid_png() {
    let dir = test_dir("apng");
    let s = synthetic_session(&dir, 5, 48, 32);
    let out = dir.join("out.png");
    let settings = ExportSettings {
        format: ExportFormat::Apng,
        path: out.to_str().unwrap().into(),
        quality: 100,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::execute(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']), "not a PNG");
    // acTL chunk marks an animated PNG
    assert!(
        bytes.windows(4).any(|w| w == b"acTL"),
        "APNG is missing the acTL animation chunk"
    );
    session_cleanup(s);
}

#[test]
fn png_sequence_export_writes_all_frames() {
    let dir = test_dir("pngseq");
    let s = synthetic_session(&dir, 4, 32, 32);
    let out = dir.join("seq.png");
    let settings = ExportSettings {
        format: ExportFormat::PngSeq,
        path: out.to_str().unwrap().into(),
        quality: 100,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::execute(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    for i in 1..=4 {
        assert!(dir.join(format!("seq_{i:04}.png")).exists(), "missing frame {i}");
    }
    session_cleanup(s);
}

/// MP4 export via Windows Media Foundation — validates container + non-empty
/// encode. (Orientation is covered by the browser-based visual check.)
#[test]
#[cfg(windows)]
fn mp4_export_produces_valid_mp4() {
    let dir = test_dir("mp4");
    let s = synthetic_session(&dir, 12, 320, 240);
    let out = dir.join("out.mp4");
    let settings = ExportSettings {
        format: ExportFormat::Mp4,
        path: out.to_str().unwrap().into(),
        quality: 90,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::execute(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.len() > 1000, "suspiciously small MP4 ({} bytes)", bytes.len());
    assert_eq!(&bytes[4..8], b"ftyp", "missing MP4 ftyp box");
    session_cleanup(s);
}

/// Regression test for "every captured frame is identical": animate a color-
/// cycling window inside the capture region (no input injection) and assert
/// the captured frames actually differ.
#[test]
#[cfg(windows)]
fn capture_sees_screen_changes() {
    use std::time::Duration;
    use voidgif_lib::capture::{self, CaptureConfig, CaptureFlags};

    const ANIMATOR: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$f = New-Object System.Windows.Forms.Form
$f.FormBorderStyle = 'None'
$f.StartPosition = 'Manual'
$f.Location = New-Object System.Drawing.Point(0, 0)
$f.Size = New-Object System.Drawing.Size(500, 400)
$f.TopMost = $true
$colors = @([System.Drawing.Color]::Red, [System.Drawing.Color]::Lime,
            [System.Drawing.Color]::Blue, [System.Drawing.Color]::Yellow)
$i = 0
$t = New-Object System.Windows.Forms.Timer
$t.Interval = 60
$t.Add_Tick({ $script:i++; $f.BackColor = $colors[$script:i % 4] })
$t.Start()
[System.Windows.Forms.Application]::Run($f)
"#;

    let mut animator = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", ANIMATOR])
        .spawn()
        .expect("spawn animator window");
    std::thread::sleep(Duration::from_millis(1500)); // let the form paint

    let (tx, rx) = crossbeam_channel::bounded(8);
    let flags = Arc::new(CaptureFlags::default());
    let config = CaptureConfig {
        region: Region { x: 40, y: 40, width: 240, height: 180 },
        fps: 30,
        show_cursor: false,
    };
    let backend = capture::start_backend(config, tx, Arc::clone(&flags))
        .expect("failed to start capture backend");

    let deadline = std::time::Instant::now() + Duration::from_millis(3000);
    let mut hashes: Vec<u64> = Vec::new();
    while std::time::Instant::now() < deadline && hashes.len() < 12 {
        if let Ok(frame) = rx.recv_timeout(Duration::from_millis(300)) {
            // Cheap content fingerprint: sample every 977th byte.
            let mut h: u64 = frame.bgra.len() as u64;
            for (i, b) in frame.bgra.iter().enumerate().step_by(977) {
                h = h.wrapping_mul(31).wrapping_add(*b as u64).wrapping_add(i as u64);
            }
            hashes.push(h);
        }
    }
    backend.stop().expect("failed to stop capture");
    let _ = animator.kill();

    let mut distinct = hashes.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(
        hashes.len() >= 5,
        "expected at least 5 frames, got {}",
        hashes.len()
    );
    assert!(
        distinct.len() >= 3,
        "captured {} frames but only {} distinct contents — capture is frozen on one frame",
        hashes.len(),
        distinct.len()
    );
}

/// Live smoke test: captures a small region of the primary monitor for ~1.5s
/// and asserts that real frames flow through the pipeline. Requires an
/// interactive Windows session.
#[test]
#[cfg(windows)]
fn live_capture_smoke() {
    use voidgif_lib::capture::{self, CaptureConfig, CaptureFlags};

    let (tx, rx) = crossbeam_channel::bounded(8);
    let flags = Arc::new(CaptureFlags::default());
    let config = CaptureConfig {
        region: Region { x: 0, y: 0, width: 320, height: 240 },
        fps: 30,
        show_cursor: false,
    };
    let backend = capture::start_backend(config, tx, Arc::clone(&flags))
        .expect("failed to start capture backend");

    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2500);
    let mut frames = 0usize;
    while std::time::Instant::now() < deadline && frames < 10 {
        if let Ok(frame) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
            assert_eq!(frame.width, 320);
            assert_eq!(frame.height, 240);
            assert_eq!(frame.bgra.len(), 320 * 240 * 4);
            frames += 1;
        }
    }
    backend.stop().expect("failed to stop capture");
    assert!(
        frames >= 5,
        "expected at least 5 frames in 2.5s, got {frames}"
    );
}

#[test]
fn group_ungroup_move_and_undo() {
    let dir = test_dir("groups");
    let mut s = synthetic_session(&dir, 6, 16, 16);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();

    // Non-contiguous selection is rejected and creates no group.
    assert!(editor::group_frames(&mut s, &[ids[0], ids[2]]).is_err());
    assert!(s.groups.is_empty());

    // Group a contiguous run [1,2,3].
    editor::group_frames(&mut s, &[ids[1], ids[2], ids[3]]).unwrap();
    assert_eq!(s.groups.len(), 1);
    let gid = s.groups[0].id;
    assert_eq!(s.groups[0].number, 1);
    let members: Vec<u64> = s
        .frames
        .iter()
        .filter(|f| f.group_id == Some(gid))
        .map(|f| f.id)
        .collect();
    assert_eq!(members, vec![ids[1], ids[2], ids[3]]);

    // Move the whole block to the front (rest-space index 0).
    editor::move_group(&mut s, gid, 0).unwrap();
    let order: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    assert_eq!(&order[0..3], &[ids[1], ids[2], ids[3]][..]);
    let positions: Vec<usize> = s
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.group_id == Some(gid))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(positions, vec![0, 1, 2], "block must stay contiguous");

    // Undo the move restores the original order (group intact).
    editor::undo(&mut s).unwrap();
    assert_eq!(s.frames.iter().map(|f| f.id).collect::<Vec<_>>(), ids);
    assert_eq!(s.frames.iter().filter(|f| f.group_id == Some(gid)).count(), 3);

    // Ungroup clears membership + meta; undo brings the group back.
    editor::ungroup(&mut s, gid).unwrap();
    assert!(s.groups.is_empty());
    assert!(s.frames.iter().all(|f| f.group_id.is_none()));
    editor::undo(&mut s).unwrap();
    assert_eq!(s.groups.len(), 1);
    assert_eq!(s.frames.iter().filter(|f| f.group_id == Some(gid)).count(), 3);
    session_cleanup(s);
}

#[test]
fn reorder_scatters_group_and_normalizes() {
    let dir = test_dir("regroup");
    let mut s = synthetic_session(&dir, 5, 16, 16);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    editor::group_frames(&mut s, &[ids[1], ids[2], ids[3]]).unwrap();
    let gid = s.groups[0].id;

    // Splice a non-member into the block: [1,2,0,3,4]. Longest run is [1,2].
    let new_order = vec![ids[1], ids[2], ids[0], ids[3], ids[4]];
    editor::reorder_frames(&mut s, &new_order).unwrap();

    let members: Vec<u64> = s
        .frames
        .iter()
        .filter(|f| f.group_id == Some(gid))
        .map(|f| f.id)
        .collect();
    assert_eq!(members, vec![ids[1], ids[2]], "stragglers stripped");
    assert_eq!(
        s.frames.iter().find(|f| f.id == ids[3]).unwrap().group_id,
        None
    );
    assert_eq!(s.groups.len(), 1);
    session_cleanup(s);
}

#[test]
fn merge_session_middle_preserves_order_delays_and_undo() {
    let dir = test_dir("merge");
    let mut target = synthetic_session(&dir, 4, 20, 20);
    let orig_ids: Vec<u64> = target.frames.iter().map(|f| f.id).collect();

    let mut source = synthetic_session(&dir, 3, 20, 20);
    for f in source.frames.iter_mut() {
        f.delay_ms = 77;
    }

    session::merge_session(&mut target, &mut source, 2).unwrap();
    assert_eq!(target.frames.len(), 7);
    assert_eq!(target.groups.len(), 1);
    let gid = target.groups[0].id;
    let block: Vec<usize> = target
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.group_id == Some(gid))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(block, vec![2, 3, 4], "inserted block is contiguous in the middle");
    assert!(target.frames[2..5].iter().all(|f| f.delay_ms == 77), "delays preserved");
    assert_eq!(target.frames[0].id, orig_ids[0]);
    assert_eq!(target.frames[1].id, orig_ids[1]);
    assert_eq!(target.frames[5].id, orig_ids[2]);
    assert_eq!(target.frames[6].id, orig_ids[3]);
    // Inserted pixels were copied into the target spool and are readable.
    let px = target.read_pixels(target.frames[2].pixels).unwrap();
    assert_eq!(px.len(), 20 * 20 * 4);

    // One undo reverts the whole merge (frames + group).
    editor::undo(&mut target).unwrap();
    assert_eq!(target.frames.len(), 4);
    assert!(target.groups.is_empty());
    assert_eq!(target.frames.iter().map(|f| f.id).collect::<Vec<_>>(), orig_ids);

    session_cleanup(target);
    session_cleanup(source);
}

#[test]
fn merge_session_start_end_and_dim_mismatch() {
    let dir = test_dir("merge2");
    let mut target = synthetic_session(&dir, 2, 20, 20);

    let mut src_start = synthetic_session(&dir, 2, 20, 20);
    session::merge_session(&mut target, &mut src_start, 0).unwrap();
    let gid0 = target.groups[0].id;
    assert_eq!(target.frames[0].group_id, Some(gid0));
    assert_eq!(target.frames[1].group_id, Some(gid0));
    assert_eq!(target.frames.len(), 4);

    let mut src_end = synthetic_session(&dir, 1, 20, 20);
    let before = target.frames.len();
    session::merge_session(&mut target, &mut src_end, before).unwrap();
    assert_eq!(target.frames.len(), before + 1);
    assert!(target.frames.last().unwrap().group_id.is_some());
    assert_eq!(target.groups.len(), 2);
    let mut nums: Vec<u32> = target.groups.iter().map(|g| g.number).collect();
    nums.sort_unstable();
    assert_eq!(nums, vec![1, 2], "group numbers increment");

    // A dimension mismatch is rejected and leaves the target untouched.
    let mut bad = synthetic_session(&dir, 2, 21, 20);
    let len_before = target.frames.len();
    assert!(session::merge_session(&mut target, &mut bad, 0).is_err());
    assert_eq!(target.frames.len(), len_before);
    assert_eq!(target.groups.len(), 2);

    session_cleanup(target);
    session_cleanup(src_start);
    session_cleanup(src_end);
    session_cleanup(bad);
}

#[test]
fn project_v2_roundtrip_with_groups() {
    let dir = test_dir("projv2");
    let mut s = synthetic_session(&dir, 4, 24, 18);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    editor::group_frames(&mut s, &[ids[1], ids[2]]).unwrap();
    let gid = s.groups[0].id;
    let color = s.groups[0].color;
    let number = s.groups[0].number;

    let path = dir.join("groups.voidgif");
    project::save(&mut s, path.to_str().unwrap()).unwrap();

    let loaded = project::load(path.to_str().unwrap(), &dir, "loadedv2".into()).unwrap();
    assert_eq!(loaded.frames.len(), 4);
    assert_eq!(loaded.groups.len(), 1);
    assert_eq!(loaded.groups[0].number, number);
    assert_eq!(loaded.groups[0].color, color);
    let members: Vec<u64> = loaded
        .frames
        .iter()
        .filter(|f| f.group_id == Some(loaded.groups[0].id))
        .map(|f| f.id)
        .collect();
    assert_eq!(members, vec![ids[1], ids[2]]);
    assert!(loaded.next_group_id > gid, "next_group_id advanced past the group");
    session_cleanup(s);
    session_cleanup(loaded);
}

/// A version-1 file (no group fields) must still load, with no groups. We reuse
/// the zstd pixel tail of a v2 save and prepend a hand-built v1 JSON header, so
/// the test needs no direct zstd dependency.
#[test]
fn project_v1_without_groups_still_loads() {
    let dir = test_dir("projv1");
    let mut s = synthetic_session(&dir, 2, 4, 4);
    let v2_path = dir.join("v2.voidgif");
    project::save(&mut s, v2_path.to_str().unwrap()).unwrap();
    let raw = std::fs::read(&v2_path).unwrap();

    // Layout: 8-byte magic · u64 LE json length · json · zstd pixel tail.
    let json_len = u64::from_le_bytes(raw[8..16].try_into().unwrap()) as usize;
    let tail = &raw[16 + json_len..];

    // Two distinct 4×4 frames → two 64-byte unique pixel entries in order.
    let v1_json = br#"{"version":1,"width":4,"height":4,"fps":30,"pixels":[{"len":64,"width":4,"height":4},{"len":64,"width":4,"height":4}],"frames":[{"id":0,"delay_ms":33,"pixel":0},{"id":1,"delay_ms":33,"pixel":1}]}"#;
    let v1_path = dir.join("v1.voidgif");
    let mut out = Vec::new();
    out.extend_from_slice(&raw[0..8]);
    out.extend_from_slice(&(v1_json.len() as u64).to_le_bytes());
    out.extend_from_slice(v1_json);
    out.extend_from_slice(tail);
    std::fs::write(&v1_path, &out).unwrap();

    let loaded = project::load(v1_path.to_str().unwrap(), &dir, "loadedv1".into()).unwrap();
    assert_eq!(loaded.frames.len(), 2);
    assert!(loaded.groups.is_empty());
    assert!(loaded.frames.iter().all(|f| f.group_id.is_none()));
    session_cleanup(s);
    session_cleanup(loaded);
}

/// The autosave/recovery header peek reads version + dimensions + frame count
/// straight from the JSON metadata, without touching the zstd pixel tail.
#[test]
fn project_peek_reads_metadata_without_pixels() {
    let dir = test_dir("peek");
    let mut s = synthetic_session(&dir, 7, 48, 36);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    editor::group_frames(&mut s, &[ids[1], ids[2]]).unwrap(); // exercise v2 header

    let path = dir.join("peek.voidgif");
    project::save(&mut s, path.to_str().unwrap()).unwrap();

    let summary = project::peek(path.to_str().unwrap()).unwrap();
    assert_eq!(summary.version, 2);
    assert_eq!(summary.frames, 7);
    assert_eq!((summary.width, summary.height), (48, 36));

    // A non-project file is rejected, not silently summarized.
    let junk = dir.join("junk.voidgif");
    std::fs::write(&junk, b"not a voidgif at all").unwrap();
    assert!(project::peek(junk.to_str().unwrap()).is_err());
    session_cleanup(s);
}

/// Autosave is just project::save to a fixed path; recovery peeks then loads it,
/// and discard removes it. Exercise that save → peek → load → discard round trip
/// on the project helpers directly (the Tauri path logic needs no coverage here).
#[test]
fn autosave_save_peek_load_discard_roundtrip() {
    let dir = test_dir("autosave");
    let mut s = synthetic_session(&dir, 5, 32, 24);
    let path = dir.join("autosave.voidgif");
    let path_str = path.to_str().unwrap();

    // "autosave_now"
    project::save(&mut s, path_str).unwrap();
    assert!(path.exists());

    // "check_autosave" header peek
    let summary = project::peek(path_str).unwrap();
    assert_eq!(summary.frames, 5);

    // "restore_autosave" loads the snapshot back
    let loaded = project::load(path_str, &dir, "restored".into()).unwrap();
    assert_eq!(loaded.frames.len(), 5);
    assert_eq!((loaded.width, loaded.height), (32, 24));

    // "discard_autosave"
    std::fs::remove_file(&path).unwrap();
    assert!(!path.exists());
    assert!(project::peek(path_str).is_err(), "peek fails once discarded");

    session_cleanup(s);
    session_cleanup(loaded);
}

/// Size estimation encodes a small frame sample and extrapolates; the estimate
/// must land near a real full export at the same settings (the whole point).
#[test]
fn estimate_gif_size_tracks_real_export() {
    let dir = test_dir("estimate");
    let s = synthetic_session(&dir, 60, 240, 160);
    let settings = ExportSettings {
        format: ExportFormat::Gif,
        path: dir.join("real.gif").to_str().unwrap().into(),
        quality: 80,
        width: None,
        loop_: Some(true),
        fast: false,
    };

    let estimated =
        export::estimate_gif_size(&mut ExportPlan::from_session(&s).unwrap(), &settings).unwrap();

    export::execute(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();
    let actual = std::fs::metadata(&settings.path).unwrap().len();

    // Sampled extrapolation is approximate, but must be in the right ballpark.
    let ratio = estimated as f64 / actual as f64;
    assert!(
        (0.6..=1.4).contains(&ratio),
        "estimate {estimated} vs actual {actual} (ratio {ratio:.2}) is out of range"
    );
    session_cleanup(s);
}

/// fit_to_target lowers quality (then width) toward a byte budget. Estimation is
/// approximate, so the contract isn't "hit the byte exactly" — it's "keep full
/// quality when the target is generous, and drop to the floor when it's
/// impossible" (the run_fit caller then reports the actual size + warns). Both
/// ends of that contract are exercised here.
#[test]
fn fit_to_target_picks_full_when_generous_and_floor_when_impossible() {
    let dir = test_dir("fit");
    let s = synthetic_session(&dir, 40, 200, 140);
    let base = ExportSettings {
        format: ExportFormat::Gif,
        path: dir.join("fit.gif").to_str().unwrap().into(),
        quality: 100,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let export_at = |settings: &ExportSettings| {
        export::execute(
            &mut ExportPlan::from_session(&s).unwrap(),
            settings,
            &Arc::new(AtomicBool::new(false)),
            &no_progress(),
        )
        .unwrap();
        std::fs::metadata(&settings.path).unwrap().len()
    };
    let full = export_at(&base);

    // Generous target (2× the full-quality size): keep the best quality, no
    // downscale, and the export comfortably fits.
    let generous = export::fit_to_target(
        &mut ExportPlan::from_session(&s).unwrap(),
        &base,
        full * 2,
        &cancel,
        &no_progress(),
    )
    .unwrap();
    // The ≤6-iteration binary search converges near the top of 30..=100 (e.g.
    // 99), not exactly 100 — near-full is the contract, not the endpoint.
    assert!(generous.quality >= 96, "generous target keeps near-full quality (got {})", generous.quality);
    assert_eq!(generous.width, None, "generous target does not downscale");
    assert!(export_at(&generous) <= full * 2, "generous fit stays under target");

    // Impossible target (a tiny fraction of full): fall to the floor — 50%
    // width, quality 30 — and shrink the file dramatically even though it can't
    // physically reach the byte budget.
    let floor = export::fit_to_target(
        &mut ExportPlan::from_session(&s).unwrap(),
        &base,
        (full / 100).max(1),
        &cancel,
        &no_progress(),
    )
    .unwrap();
    assert_eq!(floor.quality, 30, "impossible target drives quality to the floor");
    assert_eq!(floor.width, Some(200 / 2), "impossible target drives width to 50%");
    assert!(export_at(&floor) < full / 2, "floor settings shrink the file substantially");

    session_cleanup(s);
}

/// Builds a session whose frames are exact copies of a small set of distinct
/// images, so frame-difference ops have unambiguous duplicates to fold. `specs`
/// is a list of (color_seed, repeat_count): each seed produces one solid-ish
/// frame repeated `repeat_count` times. Delays default to 33ms.
fn patterned_session(dir: &std::path::Path, w: u32, h: u32, specs: &[(u8, u32)]) -> Session {
    let mut session = Session::create(dir, "patterned".into(), w, h, 30).unwrap();
    for &(seed, count) in specs {
        for _ in 0..count {
            let mut frame = vec![0u8; (w * h * 4) as usize];
            for p in frame.chunks_exact_mut(4) {
                p[0] = seed; // B
                p[1] = seed.wrapping_mul(3); // G
                p[2] = seed.wrapping_mul(7); // R
                p[3] = 255;
            }
            session.append_frame(&frame, w, h, 33).unwrap();
        }
    }
    session
}

#[test]
fn frame_diff_detects_identical_and_different() {
    let dir = test_dir("framediff");
    let mut s = patterned_session(&dir, 16, 16, &[(10, 1), (10, 1), (200, 1)]);
    let (a, b, c) = (s.frames[0].pixels, s.frames[1].pixels, s.frames[2].pixels);
    assert_eq!(session::frame_diff(&mut s, a, b), 0.0, "identical frames diff = 0");
    assert!(session::frame_diff(&mut s, a, c) > 0.1, "distinct frames diff clearly > 0");
    session_cleanup(s);
}

#[test]
fn trim_static_edges_removes_only_static_ends() {
    let dir = test_dir("trim");
    // 5 identical · 6 distinct (animated) · 5 identical  →  16 frames.
    let mut specs: Vec<(u8, u32)> = vec![(40, 5)];
    for k in 0..6 {
        specs.push((80 + k * 20, 1));
    }
    specs.push((220, 5));
    let mut s = patterned_session(&dir, 24, 18, &specs);
    assert_eq!(s.frames.len(), 16);

    let removed = editor::trim_static_edges(&mut s, 0.004).unwrap();
    // Leading run of 5 identical drops 4 (keeps the last still frame); same at
    // the end → 8 removed, 8 kept.
    assert_eq!(removed, 8);
    assert_eq!(s.frames.len(), 8);
    // The surviving first/last frames now differ (no static edges left).
    let (first, last) = (s.frames[0].pixels, s.frames[s.frames.len() - 1].pixels);
    assert!(session::frame_diff(&mut s, first, last) > 0.1, "trimmed ends differ");

    // Undo restores all 16 frames in one step.
    editor::undo(&mut s).unwrap();
    assert_eq!(s.frames.len(), 16);

    // A fully-static clip keeps exactly 2 frames.
    let mut flat = patterned_session(&dir, 8, 8, &[(50, 6)]);
    let r2 = editor::trim_static_edges(&mut flat, 0.004).unwrap();
    assert_eq!(flat.frames.len(), 2);
    assert_eq!(r2, 4);

    // An all-distinct clip is a no-op (no undo pushed).
    let mut moving = patterned_session(&dir, 8, 8, &[(10, 1), (60, 1), (120, 1), (200, 1)]);
    assert_eq!(editor::trim_static_edges(&mut moving, 0.004).unwrap(), 0);
    assert!(!moving.info().can_undo);

    session_cleanup(s);
    session_cleanup(flat);
    session_cleanup(moving);
}

#[test]
fn merge_duplicates_sums_delays_and_respects_cap() {
    let dir = test_dir("merge_dup");
    // Runs: 3×A · 1×B · 2×C  →  merges to A, B, C (3 frames), folding 3 away.
    let mut s = patterned_session(&dir, 20, 16, &[(30, 3), (90, 1), (150, 2)]);
    assert_eq!(s.frames.len(), 6);

    let merged = editor::merge_duplicates(&mut s, 0.002).unwrap();
    assert_eq!(merged, 3, "3 duplicate frames folded away");
    assert_eq!(s.frames.len(), 3);
    // Leader delays are the summed run delays (33ms each).
    assert_eq!(s.frames[0].delay_ms, 99, "run of 3 → 3×33ms");
    assert_eq!(s.frames[1].delay_ms, 33);
    assert_eq!(s.frames[2].delay_ms, 66, "run of 2 → 2×33ms");

    editor::undo(&mut s).unwrap();
    assert_eq!(s.frames.len(), 6);

    // Cap: a duplicate run whose summed delay exceeds 60000ms splits so no
    // single frame exceeds the ceiling.
    let mut capped = patterned_session(&dir, 8, 8, &[(70, 2)]);
    for f in capped.frames.iter_mut() {
        f.delay_ms = 40_000; // 2×40000 = 80000 > 60000
    }
    editor::merge_duplicates(&mut capped, 0.002).unwrap();
    assert!(capped.frames.iter().all(|f| f.delay_ms <= 60_000), "no delay over the cap");
    assert_eq!(
        capped.frames.iter().map(|f| f.delay_ms as u64).sum::<u64>(),
        80_000,
        "total delay is preserved across the split"
    );
    assert_eq!(capped.frames.len(), 2, "80000ms split into 60000 + 20000");

    session_cleanup(s);
    session_cleanup(capped);
}

#[test]
fn merge_duplicates_group_survives_and_normalizes() {
    let dir = test_dir("merge_grp");
    // 1×A · 3×B(grouped) · 1×C. Merging folds the B-run into its first (grouped)
    // frame; the group keeps a single contiguous member.
    let mut s = patterned_session(&dir, 16, 16, &[(20, 1), (120, 3), (220, 1)]);
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    editor::group_frames(&mut s, &[ids[1], ids[2], ids[3]]).unwrap();
    let gid = s.groups[0].id;

    let merged = editor::merge_duplicates(&mut s, 0.002).unwrap();
    assert_eq!(merged, 2, "two of the three B frames folded away");
    assert_eq!(s.frames.len(), 3);
    // The surviving grouped leader is frame B (the run's first).
    let members: Vec<u64> = s.frames.iter().filter(|f| f.group_id == Some(gid)).map(|f| f.id).collect();
    assert_eq!(members, vec![ids[1]], "group keeps its leading member");
    assert_eq!(s.groups.len(), 1, "single-member group survives normalization");

    session_cleanup(s);
}

#[test]
fn scale_delays_clamps_and_targets() {
    let dir = test_dir("scale");
    let mut s = synthetic_session(&dir, 4, 16, 16); // delays all 33ms
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();

    // 2× faster on all frames → half the delay (rounded).
    editor::scale_delays(&mut s, &[], 2.0).unwrap();
    assert!(s.frames.iter().all(|f| f.delay_ms == 17), "33/2 rounds to 17");

    // Selection-only: slow the first frame 4×.
    editor::scale_delays(&mut s, &[ids[0]], 0.25).unwrap();
    assert_eq!(s.frames[0].delay_ms, 68, "17/0.25 = 68");
    assert_eq!(s.frames[1].delay_ms, 17, "unselected frames untouched");

    // Clamp: a huge speed-up floors at 10ms.
    editor::scale_delays(&mut s, &[], 10.0).unwrap();
    assert!(s.frames.iter().all(|f| f.delay_ms >= 10), "delays clamp to >= 10ms");

    // Invalid factor rejected.
    assert!(editor::scale_delays(&mut s, &[], 0.0).is_err());
    assert!(editor::scale_delays(&mut s, &[], 11.0).is_err());

    session_cleanup(s);
}

#[test]
fn make_pingpong_excludes_endpoints_and_shares_pixels() {
    let dir = test_dir("pingpong");
    let mut s = synthetic_session(&dir, 4, 16, 16); // A B C D
    let ids: Vec<u64> = s.frames.iter().map(|f| f.id).collect();
    let offsets: Vec<u64> = s.frames.iter().map(|f| f.pixels.offset).collect();

    editor::make_pingpong(&mut s).unwrap();
    // A B C D + C B  → 2n-2 = 6 frames.
    assert_eq!(s.frames.len(), 6);
    // Appended frames mirror the interior, excluding first and last.
    assert_eq!(s.frames[4].pixels.offset, offsets[2], "5th frame reuses C's pixels");
    assert_eq!(s.frames[5].pixels.offset, offsets[1], "6th frame reuses B's pixels");
    // New ids, no pixel duplication on disk, no group.
    assert!(s.frames[4].id != ids[2] && s.frames[5].id != ids[1], "appended frames get fresh ids");
    assert!(s.frames[4].group_id.is_none() && s.frames[5].group_id.is_none());

    editor::undo(&mut s).unwrap();
    assert_eq!(s.frames.len(), 4);

    // Guard: fewer than 3 frames is rejected.
    let mut two = synthetic_session(&dir, 2, 8, 8);
    assert!(editor::make_pingpong(&mut two).is_err());

    session_cleanup(s);
    session_cleanup(two);
}

/// Mean absolute per-byte difference between two BGRA buffers, normalized to
/// 0.0..=1.0. Used to assert GIF palette quantization stays "close" without
/// demanding byte-exact equality.
fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 1.0;
    }
    let mut sum = 0u64;
    for i in 0..n {
        sum += (a[i] as i32 - b[i] as i32).unsigned_abs() as u64;
    }
    sum as f64 / (n as f64 * 255.0)
}

/// A GIF exported by the MIT compat encoder must reopen through `gif_import`
/// with its frame count, canvas size, delays (within GIF's 10 ms quantization),
/// and pixel content (within palette quantization) intact.
#[test]
fn gif_import_roundtrips_a_compat_export() {
    let dir = test_dir("gifimport");
    // Solid-color frames quantize near-losslessly, so the pixel assertion is
    // unambiguous. Distinct seeds keep the four frames apart.
    let mut s = patterned_session(&dir, 32, 24, &[(30, 1), (90, 1), (150, 1), (210, 1)]);
    let refs: Vec<_> = s.frames.iter().map(|f| f.pixels).collect();
    let originals: Vec<Vec<u8>> = refs.iter().map(|&p| s.read_pixels(p).unwrap()).collect();

    let out = dir.join("roundtrip.gif");
    let settings = ExportSettings {
        format: ExportFormat::Gif,
        path: out.to_str().unwrap().into(),
        quality: 90,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    export::export_gif_compat(
        &mut ExportPlan::from_session(&s).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &no_progress(),
    )
    .unwrap();

    let mut imported = gif_import::load(out.to_str().unwrap(), &dir, "gifin".into()).unwrap();
    assert_eq!(imported.frames.len(), 4, "frame count survives the roundtrip");
    assert_eq!((imported.width, imported.height), (32, 24), "canvas dims match");
    // 33 ms is stored as 3 centiseconds → decodes back to 30 ms.
    for f in &imported.frames {
        assert!(
            (f.delay_ms as i32 - 33).abs() <= 10,
            "delay {} is within GIF's 10 ms quantization of 33",
            f.delay_ms
        );
    }
    for (i, orig) in originals.iter().enumerate() {
        let got = imported.read_pixels(imported.frames[i].pixels).unwrap();
        assert_eq!(got.len(), orig.len(), "frame {i} size matches");
        assert!(
            mean_abs_diff(orig, &got) < 0.05,
            "frame {i} content survives palette quantization"
        );
    }
    session_cleanup(s);
    session_cleanup(imported);
}

/// Sub-rect blitting + Background disposal: frame 1 fills the canvas with A and
/// disposes to background; frame 2 paints only a small offset rect with B. The
/// composited result must be all-A for frame 1, and for frame 2: B inside the
/// rect, transparent (not A) outside — proving the background clear ran.
#[test]
fn gif_import_composites_subrects_and_background_disposal() {
    use std::borrow::Cow;

    let dir = test_dir("gifdispose");
    let (w, h) = (8u16, 8u16);
    // Global palette: index 0 = A (R,G,B), index 1 = B.
    let (a_rgb, b_rgb) = ([200u8, 40, 40], [40u8, 200, 40]);
    let palette = vec![a_rgb[0], a_rgb[1], a_rgb[2], b_rgb[0], b_rgb[1], b_rgb[2]];

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut buf, w, h, &palette).unwrap();
        // Frame 1: full canvas of A, disposed back to background afterwards.
        let f1 = gif::Frame {
            width: w,
            height: h,
            delay: 10,
            dispose: gif::DisposalMethod::Background,
            buffer: Cow::Owned(vec![0u8; (w as usize) * (h as usize)]),
            ..Default::default()
        };
        enc.write_frame(&f1).unwrap();
        // Frame 2: a 2×2 rect of B at (2,2); keep it on the (now cleared) canvas.
        let f2 = gif::Frame {
            left: 2,
            top: 2,
            width: 2,
            height: 2,
            delay: 10,
            dispose: gif::DisposalMethod::Keep,
            buffer: Cow::Owned(vec![1u8; 4]),
            ..Default::default()
        };
        enc.write_frame(&f2).unwrap();
    }
    let gif_path = dir.join("dispose.gif");
    std::fs::write(&gif_path, &buf).unwrap();

    let mut imported = gif_import::load(gif_path.to_str().unwrap(), &dir, "disp".into()).unwrap();
    assert_eq!(imported.frames.len(), 2);
    assert_eq!((imported.width, imported.height), (8, 8));

    // Session frames are BGRA; A/B stored with the byte order swapped.
    let a_bgra = [a_rgb[2], a_rgb[1], a_rgb[0], 255];
    let b_bgra = [b_rgb[2], b_rgb[1], b_rgb[0], 255];

    let f0 = imported.read_pixels(imported.frames[0].pixels).unwrap();
    assert_eq!(&f0[0..4], &a_bgra, "frame 0 is fully painted with A");

    let f1 = imported.read_pixels(imported.frames[1].pixels).unwrap();
    let inside = ((2 * 8 + 2) * 4) as usize; // pixel (2,2)
    assert_eq!(&f1[inside..inside + 4], &b_bgra, "sub-rect painted with B");
    assert_eq!(
        &f1[0..4],
        &[0, 0, 0, 0],
        "outside the sub-rect is transparent — Background disposal cleared A"
    );

    session_cleanup(imported);
}

fn session_cleanup(s: Session) {
    s.cleanup();
}
