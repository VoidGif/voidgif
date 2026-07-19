//! Frame editing operations. Metadata-only ops (delete/duplicate/reorder/
//! delay) never touch pixels; crop/resize append new pixel data to the spool
//! (copy-on-write), which keeps undo snapshots valid forever.
//!
//! Fallible ops take their undo snapshot up front but only push it after the
//! whole operation succeeds, so a failed edit never pollutes undo/redo.

use crate::capture::Region;
use crate::session::{self, Frame, GroupMeta, Session};
use fast_image_resize as fir;

/// Frame count ceiling shared by growth ops (mirrors the project-load cap).
const MAX_FRAMES: usize = 20_000;
/// GIF delay ceiling — no single frame may exceed this after a merge.
const MAX_DELAY_MS: u32 = 60_000;

pub fn delete_frames(session: &mut Session, ids: &[u64]) -> Result<(), String> {
    let matched = session.frames.iter().filter(|f| ids.contains(&f.id)).count();
    if matched == 0 {
        return Ok(());
    }
    if matched >= session.frames.len() {
        return Err("cannot delete every frame".into());
    }
    session.push_undo();
    session.frames.retain(|f| !ids.contains(&f.id));
    // A delete can empty a group → drop its meta (membership stays contiguous).
    session.normalize_groups();
    Ok(())
}

/// Groups a contiguous run of frames into a new auto-numbered, colored group.
/// `ids` must map to frames that are currently adjacent in display order.
/// Frames already in another group are reassigned to the new one.
pub fn group_frames(session: &mut Session, ids: &[u64]) -> Result<(), String> {
    use std::collections::HashSet;
    let id_set: HashSet<u64> = ids.iter().copied().collect();
    if id_set.len() < 2 {
        return Err("select at least two frames to group".into());
    }
    let positions: Vec<usize> = session
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| id_set.contains(&f.id))
        .map(|(i, _)| i)
        .collect();
    if positions.len() != id_set.len() {
        return Err("some frames to group were not found".into());
    }
    // Contiguity: the matched positions must form one unbroken run.
    let first = positions[0];
    let last = positions[positions.len() - 1];
    if last - first + 1 != positions.len() {
        return Err("frames to group must be contiguous".into());
    }
    // No-op guard: already exactly one group covering just these frames.
    if let Some(gid) = session.frames[first].group_id {
        let same_group = positions.iter().all(|&i| session.frames[i].group_id == Some(gid));
        let members = session.frames.iter().filter(|f| f.group_id == Some(gid)).count();
        if same_group && members == positions.len() {
            return Ok(());
        }
    }
    session.push_undo();
    let number = session.groups.iter().map(|g| g.number).max().unwrap_or(0) + 1;
    let color = (session.groups.len() % 6) as u8;
    let id = session.next_group_id;
    session.next_group_id += 1;
    session.groups.push(GroupMeta { id, number, color });
    for f in session.frames.iter_mut() {
        if id_set.contains(&f.id) {
            f.group_id = Some(id);
        }
    }
    // Reassignment may have emptied a source group.
    session.normalize_groups();
    Ok(())
}

/// Dissolves a group: clears its members' `group_id` and removes the meta.
pub fn ungroup(session: &mut Session, group_id: u64) -> Result<(), String> {
    if !session.groups.iter().any(|g| g.id == group_id) {
        return Ok(()); // no-op — unknown/already-gone group
    }
    session.push_undo();
    for f in session.frames.iter_mut() {
        if f.group_id == Some(group_id) {
            f.group_id = None;
        }
    }
    session.groups.retain(|g| g.id != group_id);
    Ok(())
}

/// Moves a group's whole member block so it starts at display index
/// `to_index`, where the index is counted in the order WITHOUT the block
/// (matching the filmstrip's drop math). Clamped; single undo entry.
pub fn move_group(session: &mut Session, group_id: u64, to_index: usize) -> Result<(), String> {
    if !session.groups.iter().any(|g| g.id == group_id) {
        return Err("unknown group".into());
    }
    let block: Vec<_> = session
        .frames
        .iter()
        .filter(|f| f.group_id == Some(group_id))
        .copied()
        .collect();
    if block.is_empty() {
        return Err("group has no frames".into());
    }
    let rest: Vec<_> = session
        .frames
        .iter()
        .filter(|f| f.group_id != Some(group_id))
        .copied()
        .collect();
    let to = to_index.min(rest.len());
    // Current block position in rest-space = non-members before the block.
    let cur_start = session
        .frames
        .iter()
        .position(|f| f.group_id == Some(group_id))
        .unwrap();
    let cur_in_rest = session.frames[..cur_start]
        .iter()
        .filter(|f| f.group_id != Some(group_id))
        .count();
    if cur_in_rest == to {
        return Ok(()); // no-op
    }
    session.push_undo();
    let mut new_frames = Vec::with_capacity(session.frames.len());
    new_frames.extend_from_slice(&rest[..to]);
    new_frames.extend_from_slice(&block);
    new_frames.extend_from_slice(&rest[to..]);
    session.frames = new_frames;
    Ok(())
}

pub fn duplicate_frames(session: &mut Session, ids: &[u64]) -> Result<(), String> {
    if !session.frames.iter().any(|f| ids.contains(&f.id)) {
        return Ok(());
    }
    session.push_undo();
    // Insert each duplicate right after its source, preserving order.
    let mut i = 0;
    while i < session.frames.len() {
        if ids.contains(&session.frames[i].id) {
            let mut copy = session.frames[i];
            copy.id = session.next_frame_id;
            session.next_frame_id += 1;
            session.frames.insert(i + 1, copy);
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(())
}

pub fn reorder_frames(session: &mut Session, order: &[u64]) -> Result<(), String> {
    if order.len() != session.frames.len() {
        return Err("reorder list length mismatch".into());
    }
    if session.frames.iter().map(|f| f.id).eq(order.iter().copied()) {
        return Ok(()); // no-op — keep the undo stack clean
    }
    let mut reordered = Vec::with_capacity(order.len());
    for id in order {
        let frame = session
            .frame_by_id(*id)
            .ok_or_else(|| format!("unknown frame id {id}"))?;
        reordered.push(frame);
    }
    session.push_undo();
    session.frames = reordered;
    // Reordering can scatter a group's members: keep the longest contiguous
    // run, strip the stragglers, and drop any group left empty.
    session.normalize_groups();
    Ok(())
}

pub fn set_frame_delays(session: &mut Session, ids: &[u64], delay_ms: u32) -> Result<(), String> {
    if !(10..=60_000).contains(&delay_ms) {
        return Err("delay must be between 10 and 60000 ms".into());
    }
    if !session.frames.iter().any(|f| ids.contains(&f.id)) {
        return Ok(());
    }
    session.push_undo();
    for frame in session.frames.iter_mut() {
        if ids.contains(&frame.id) {
            frame.delay_ms = delay_ms;
        }
    }
    Ok(())
}

pub fn crop(session: &mut Session, rect: Region) -> Result<(), String> {
    if rect.x < 0 || rect.y < 0 || rect.width < 1 || rect.height < 1 {
        return Err("crop rectangle is empty or negative".into());
    }
    let x = rect.x as u64;
    let y = rect.y as u64;
    // u64 math — immune to u32 wrapping on hostile IPC input.
    if x + rect.width as u64 > session.width as u64
        || y + rect.height as u64 > session.height as u64
    {
        return Err("crop rectangle exceeds frame bounds".into());
    }
    let snapshot = session.snapshot();

    let frames = session.frames.clone();
    let mut updated = Vec::with_capacity(frames.len());
    for frame in frames {
        // Frames are session-sized by construction, but never trust that with
        // a slice-indexing loop below — a corrupt project must not panic here.
        if x + rect.width as u64 > frame.pixels.width as u64
            || y + rect.height as u64 > frame.pixels.height as u64
        {
            return Err(format!(
                "frame {} is smaller ({}×{}) than the crop rectangle",
                frame.id, frame.pixels.width, frame.pixels.height
            ));
        }
        let src = session
            .read_pixels(frame.pixels)
            .map_err(|e| format!("read frame: {e}"))?;
        let src_row = frame.pixels.width as usize * 4;
        let dst_row = rect.width as usize * 4;
        let mut dst = vec![0u8; dst_row * rect.height as usize];
        for row in 0..rect.height as usize {
            let s = (y as usize + row) * src_row + x as usize * 4;
            dst[row * dst_row..(row + 1) * dst_row].copy_from_slice(&src[s..s + dst_row]);
        }
        let pixels = session
            .append_pixels(&dst, rect.width, rect.height)
            .map_err(|e| format!("write frame: {e}"))?;
        let mut f = frame;
        f.pixels = pixels;
        updated.push(f);
    }
    session.push_undo_snapshot(snapshot);
    session.frames = updated;
    session.width = rect.width;
    session.height = rect.height;
    Ok(())
}

/// Resizes every frame with a SIMD Lanczos3 filter.
pub fn resize(session: &mut Session, width: u32, height: u32) -> Result<(), String> {
    if width < 1 || height < 1 || width > 8192 || height > 8192 {
        return Err("target size out of range".into());
    }
    if width == session.width && height == session.height {
        return Ok(());
    }
    let snapshot = session.snapshot();

    let mut resizer = fir::Resizer::new();
    let options = fir::ResizeOptions::new()
        .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3))
        .use_alpha(false); // screen captures are opaque

    let frames = session.frames.clone();
    let mut updated = Vec::with_capacity(frames.len());
    for frame in frames {
        let src_bytes = session
            .read_pixels(frame.pixels)
            .map_err(|e| format!("read frame: {e}"))?;
        let src = fir::images::Image::from_vec_u8(
            frame.pixels.width,
            frame.pixels.height,
            src_bytes,
            fir::PixelType::U8x4,
        )
        .map_err(|e| format!("bad frame buffer: {e}"))?;
        let mut dst = fir::images::Image::new(width, height, fir::PixelType::U8x4);
        resizer
            .resize(&src, &mut dst, Some(&options))
            .map_err(|e| format!("resize failed: {e}"))?;
        let pixels = session
            .append_pixels(&dst.into_vec(), width, height)
            .map_err(|e| format!("write frame: {e}"))?;
        let mut f = frame;
        f.pixels = pixels;
        updated.push(f);
    }
    session.push_undo_snapshot(snapshot);
    session.frames = updated;
    session.width = width;
    session.height = height;
    Ok(())
}

/// Trims near-static frames off the START and END of the clip. Walking from the
/// start, a leading frame is dropped while it differs from its successor by less
/// than `threshold` (so the last still frame before motion is kept); the same is
/// done from the end backwards. Always leaves >= 2 frames. Returns the number of
/// frames removed; a clip with nothing to trim is a no-op (`Ok(0)`, no undo).
pub fn trim_static_edges(session: &mut Session, threshold: f64) -> Result<usize, String> {
    let n = session.frames.len();
    if n <= 2 {
        return Ok(0);
    }
    // Leading run: frames near-identical to the one after them.
    let mut lead = 0usize;
    while lead + 1 < n {
        let (a, b) = (session.frames[lead].pixels, session.frames[lead + 1].pixels);
        if session::frame_diff(session, a, b) < threshold {
            lead += 1;
        } else {
            break;
        }
    }
    // Trailing run: frames near-identical to the one before them.
    let mut trail = 0usize;
    while n - 1 - trail > 0 {
        let i = n - 1 - trail;
        let (a, b) = (session.frames[i].pixels, session.frames[i - 1].pixels);
        if session::frame_diff(session, a, b) < threshold {
            trail += 1;
        } else {
            break;
        }
    }
    if lead == 0 && trail == 0 {
        return Ok(0);
    }
    // Keep >= 2 frames: cap the total removed, shaving the trailing run first.
    let removable = n - 2;
    if lead + trail > removable {
        let over = lead + trail - removable;
        let cut = over.min(trail);
        trail -= cut;
        lead -= (over - cut).min(lead);
    }
    let removed = lead + trail;
    if removed == 0 {
        return Ok(0);
    }
    session.push_undo();
    session.frames.drain(n - trail..n);
    session.frames.drain(0..lead);
    session.normalize_groups();
    Ok(removed)
}

/// Collapses RUNS of consecutive near-identical frames (diff < `threshold`) into
/// the run's first frame, summing the run's delays onto it. If a summed delay
/// exceeds the 60s GIF ceiling the leftover is emitted as extra ungrouped frames
/// so no single delay exceeds the cap. The leader keeps the first frame's group;
/// emptied groups are normalized away. Returns the count of frames folded in;
/// a clip with no adjacent duplicates is a no-op (`Ok(0)`, no undo).
pub fn merge_duplicates(session: &mut Session, threshold: f64) -> Result<usize, String> {
    let orig = session.frames.clone();
    let n = orig.len();
    if n < 2 {
        return Ok(0);
    }
    let snapshot = session.snapshot();
    let mut result: Vec<Frame> = Vec::with_capacity(n);
    let mut absorbed = 0usize;
    let mut i = 0;
    while i < n {
        // Extend the run while each next frame matches the previous one.
        let mut sum: u64 = orig[i].delay_ms as u64;
        let mut j = i + 1;
        while j < n && session::frame_diff(session, orig[j - 1].pixels, orig[j].pixels) < threshold {
            sum += orig[j].delay_ms as u64;
            j += 1;
        }
        absorbed += j - i - 1;
        // Emit the run leader with the summed delay, splitting only if it would
        // exceed the GIF ceiling (rare: needs > 60s of duplicates in one run).
        let mut leader = orig[i];
        let first = sum.min(MAX_DELAY_MS as u64) as u32;
        leader.delay_ms = first.max(10);
        result.push(leader);
        let mut remaining = sum - first as u64;
        while remaining > 0 {
            let chunk = remaining.min(MAX_DELAY_MS as u64) as u32;
            remaining -= chunk as u64;
            let mut extra = orig[i];
            extra.id = session.next_frame_id;
            session.next_frame_id += 1;
            extra.group_id = None;
            extra.delay_ms = chunk;
            result.push(extra);
        }
        i = j;
    }
    if absorbed == 0 {
        return Ok(0); // no adjacent duplicates — keep the undo stack clean
    }
    session.push_undo_snapshot(snapshot);
    session.frames = result;
    session.normalize_groups();
    Ok(absorbed)
}

/// Scales the delays of `ids` (empty = every frame) by `factor`: a larger factor
/// plays faster, so `new = clamp(round(delay / factor), 10, 60000)`. Single undo;
/// a selection matching no frames is a no-op.
pub fn scale_delays(session: &mut Session, ids: &[u64], factor: f64) -> Result<(), String> {
    if !factor.is_finite() || !(0.1..=10.0).contains(&factor) {
        return Err("speed factor must be between 0.1 and 10".into());
    }
    let all = ids.is_empty();
    let has_target = session
        .frames
        .iter()
        .any(|f| all || ids.contains(&f.id));
    if !has_target {
        return Ok(());
    }
    session.push_undo();
    for f in session.frames.iter_mut() {
        if all || ids.contains(&f.id) {
            let scaled = (f.delay_ms as f64 / factor).round();
            f.delay_ms = scaled.clamp(10.0, MAX_DELAY_MS as f64) as u32;
        }
    }
    Ok(())
}

/// Turns the clip into a ping-pong (boomerang): appends the interior frames in
/// reverse, excluding the first and last (A B C D -> A B C D C B). Appended
/// frames reuse the source PixelRefs (zero pixel cost), get fresh ids, and carry
/// no group. Needs >= 3 frames and respects the frame-count cap. Single undo.
pub fn make_pingpong(session: &mut Session) -> Result<(), String> {
    let n = session.frames.len();
    if n < 3 {
        return Err("need at least 3 frames to make a ping-pong loop".into());
    }
    let appended = n - 2;
    if n + appended > MAX_FRAMES {
        return Err("ping-pong would exceed the frame limit".into());
    }
    session.push_undo();
    let mut extra: Vec<Frame> = Vec::with_capacity(appended);
    for i in (1..n - 1).rev() {
        let mut f = session.frames[i];
        f.id = session.next_frame_id;
        session.next_frame_id += 1;
        f.group_id = None;
        extra.push(f);
    }
    session.frames.extend(extra);
    Ok(())
}

pub fn undo(session: &mut Session) -> Result<(), String> {
    let snap = session.undo_stack.pop().ok_or("nothing to undo")?;
    session.redo_stack.push(session.snapshot());
    session.restore(snap);
    Ok(())
}

pub fn redo(session: &mut Session) -> Result<(), String> {
    let snap = session.redo_stack.pop().ok_or("nothing to redo")?;
    session.undo_stack.push(session.snapshot());
    session.restore(snap);
    Ok(())
}
