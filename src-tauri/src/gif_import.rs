//! Opening an existing GIF for editing (ScreenToGif-style "Open").
//!
//! A GIF frame is a sub-rectangle plus a disposal method; our sessions store
//! full-canvas BGRA frames. We composite each GIF frame onto a persistent RGBA
//! canvas the size of the logical screen, honoring disposal, then append the
//! whole canvas as one session frame. Unpainted pixels stay transparent (alpha
//! 0) — the rest of the pipeline renders that alpha as black, matching how it
//! treats transparency everywhere else.

use crate::session::{Frame, Session};
use gif::{DecodeOptions, DisposalMethod};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn load(path: &str, session_dir: &Path, new_id: String) -> Result<Session, String> {
    // Hard caps — a crafted GIF must not allocate huge buffers, fill the temp
    // disk, or overflow the byte arithmetic below. Same rationale/values as
    // `project::load`; every session frame here is a full canvas, so the total
    // is simply `frame_count * frame_bytes`.
    const MAX_DIM: u32 = 8192;
    const MAX_FRAMES: usize = 20_000;
    const MAX_TOTAL_PIXEL_BYTES: u64 = 8 << 30; // 8 GiB spool budget

    let file = File::open(path).map_err(|e| format!("open GIF file: {e}"))?;
    let mut options = DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(BufReader::new(file))
        .map_err(|_| "not a GIF file".to_string())?;

    let width = decoder.width() as u32;
    let height = decoder.height() as u32;
    if width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM {
        return Err("GIF dimensions out of range".into());
    }
    let frame_bytes = width as u64 * height as u64 * 4;

    // fps is refined from the median delay once every frame is seen (below);
    // it only seeds continue-recording defaults, so the initial value is moot.
    let mut session = Session::create(session_dir, new_id, width, height, 30)
        .map_err(|e| format!("create session spool: {e}"))?;

    // Persistent RGBA canvas of the logical screen. Snapshot holds the pre-frame
    // canvas for DisposalMethod::Previous restores.
    let mut canvas = vec![0u8; frame_bytes as usize];
    let mut prev_snapshot: Option<Vec<u8>> = None;
    let mut delays: Vec<u32> = Vec::new();

    loop {
        let frame = match decoder
            .read_next_frame()
            .map_err(|e| format!("decode GIF frame: {e}"))?
        {
            Some(f) => f,
            None => break,
        };

        if delays.len() >= MAX_FRAMES {
            session.cleanup();
            return Err("GIF too large (too many frames)".into());
        }
        if (delays.len() as u64 + 1) * frame_bytes > MAX_TOTAL_PIXEL_BYTES {
            session.cleanup();
            return Err("GIF too large (pixel data exceeds size budget)".into());
        }

        let (fx, fy) = (frame.left as u32, frame.top as u32);
        let (fw, fh) = (frame.width as u32, frame.height as u32);
        let dispose = frame.dispose;

        // (a) DisposalMethod::Previous means "undo this frame afterwards" —
        //     capture the canvas before painting so it can be restored.
        if dispose == DisposalMethod::Previous {
            prev_snapshot = Some(canvas.clone());
        }

        // (b) Blit the sub-rect, copying only opaque pixels (GIF transparency is
        //     binary: a pixel is either the transparent index or fully opaque).
        for row in 0..fh {
            let cy = fy + row;
            if cy >= height {
                break;
            }
            for col in 0..fw {
                let cx = fx + col;
                if cx >= width {
                    break;
                }
                let src = ((row * fw + col) * 4) as usize;
                if frame.buffer[src + 3] == 0 {
                    continue;
                }
                let dst = ((cy * width + cx) * 4) as usize;
                canvas[dst..dst + 4].copy_from_slice(&frame.buffer[src..src + 4]);
            }
        }

        // (c) Append the whole canvas as a BGRA frame (RGBA -> BGRA, alpha kept).
        let mut bgra = canvas.clone();
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let pixels = session
            .append_pixels(&bgra, width, height)
            .map_err(|e| format!("write spool: {e}"))?;

        // gif delays are in 10 ms units; a 0/1-unit delay means "as fast as
        // possible", which browsers and editors universally render at ~100 ms.
        let raw = frame.delay as u32;
        let delay_ms = (if raw <= 1 { 100 } else { raw * 10 }).clamp(10, 60_000);
        session.frames.push(Frame {
            id: delays.len() as u64,
            delay_ms,
            pixels,
            group_id: None,
        });
        delays.push(delay_ms);

        // (d) Apply disposal to ready the canvas for the next frame.
        match dispose {
            DisposalMethod::Background => {
                // Clear only this frame's rect back to transparent.
                for row in 0..fh {
                    let cy = fy + row;
                    if cy >= height {
                        break;
                    }
                    for col in 0..fw {
                        let cx = fx + col;
                        if cx >= width {
                            break;
                        }
                        let dst = ((cy * width + cx) * 4) as usize;
                        canvas[dst..dst + 4].copy_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
            DisposalMethod::Previous => {
                if let Some(snap) = prev_snapshot.take() {
                    canvas = snap;
                }
            }
            // Keep / Any: leave the composited canvas untouched.
            _ => {}
        }
    }

    if delays.is_empty() {
        session.cleanup();
        return Err("GIF has no frames".into());
    }

    session.next_frame_id = delays.len() as u64;

    // Seed fps from the median frame delay (a robust central tendency for the
    // mixed delays real GIFs carry). Only affects continue-recording defaults.
    delays.sort_unstable();
    let median_ms = delays[delays.len() / 2].max(1);
    session.fps = (1000 / median_ms).clamp(1, 60);

    Ok(session)
}
