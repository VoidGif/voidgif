//! Recording session: frame metadata in memory, pixels spooled to disk.
//!
//! The spool is a single append-only file of raw BGRA frames. Every edit that
//! changes pixels (crop/resize) appends new frame data and repoints the
//! metadata — old bytes stay in the file until the session is dropped, which
//! keeps undo cheap and safe at the cost of temp-disk space.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const MAX_UNDO: usize = 64;

/// Where a frame's pixels live inside the spool file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PixelRef {
    pub offset: u64,
    pub len: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Frame {
    /// Stable id — survives reorder/delete; used by the frontend.
    pub id: u64,
    pub delay_ms: u32,
    pub pixels: PixelRef,
    /// Group membership; `None` when the frame is ungrouped. Members of a group
    /// are always CONTIGUOUS in display order — the editor normalizes after any
    /// mutation that could break that.
    #[serde(default)]
    pub group_id: Option<u64>,
}

/// A frame group: an auto-numbered, colored run of contiguous frames. Groups
/// have no user-editable name — only a number (for the "녹화 N" style label)
/// and a palette color index.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GroupMeta {
    pub id: u64,
    pub number: u32,
    /// Palette index 0..=5.
    pub color: u8,
}

/// Metadata snapshot pushed to the undo stack before each mutation.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub frames: Vec<Frame>,
    pub width: u32,
    pub height: u32,
    pub groups: Vec<GroupMeta>,
}

pub struct Session {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frames: Vec<Frame>,
    pub next_frame_id: u64,
    /// Group metadata; membership lives on each frame (`Frame::group_id`).
    pub groups: Vec<GroupMeta>,
    pub next_group_id: u64,
    spool_path: PathBuf,
    spool: File,
    spool_len: u64,
    pub undo_stack: Vec<Snapshot>,
    pub redo_stack: Vec<Snapshot>,
}

/// What the frontend sees (serde camelCase to match src/types.ts).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub can_undo: bool,
    pub can_redo: bool,
    pub frames: Vec<FrameInfo>,
    pub groups: Vec<GroupInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameInfo {
    pub id: u64,
    pub delay_ms: u32,
    pub group_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub id: u64,
    pub number: u32,
    pub color: u8,
}

impl Session {
    pub fn create(dir: &Path, id: String, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let spool_path = dir.join(format!("{id}.spool"));
        let spool = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&spool_path)?;
        Ok(Self {
            id,
            width,
            height,
            fps,
            frames: Vec::new(),
            next_frame_id: 0,
            groups: Vec::new(),
            next_group_id: 0,
            spool_path,
            spool,
            spool_len: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            width: self.width,
            height: self.height,
            fps: self.fps,
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            frames: self
                .frames
                .iter()
                .map(|f| FrameInfo { id: f.id, delay_ms: f.delay_ms, group_id: f.group_id })
                .collect(),
            groups: self
                .groups
                .iter()
                .map(|g| GroupInfo { id: g.id, number: g.number, color: g.color })
                .collect(),
        }
    }

    /// Appends raw BGRA pixels, returns the new frame's stable id.
    pub fn append_frame(
        &mut self,
        bgra: &[u8],
        width: u32,
        height: u32,
        delay_ms: u32,
    ) -> std::io::Result<u64> {
        let pixels = self.append_pixels(bgra, width, height)?;
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.frames.push(Frame { id, delay_ms, pixels, group_id: None });
        Ok(id)
    }

    /// Appends a pixel buffer without creating a frame (used by crop/resize).
    pub fn append_pixels(
        &mut self,
        bgra: &[u8],
        width: u32,
        height: u32,
    ) -> std::io::Result<PixelRef> {
        debug_assert_eq!(bgra.len() as u64, width as u64 * height as u64 * 4);
        self.spool.seek(SeekFrom::Start(self.spool_len))?;
        self.spool.write_all(bgra)?;
        let r = PixelRef {
            offset: self.spool_len,
            len: bgra.len() as u64,
            width,
            height,
        };
        self.spool_len += bgra.len() as u64;
        Ok(r)
    }

    /// Reads a frame's raw BGRA pixels back from the spool.
    pub fn read_pixels(&mut self, pixels: PixelRef) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; pixels.len as usize];
        self.spool.seek(SeekFrom::Start(pixels.offset))?;
        self.spool.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn frame_by_id(&self, id: u64) -> Option<Frame> {
        self.frames.iter().copied().find(|f| f.id == id)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            frames: self.frames.clone(),
            width: self.width,
            height: self.height,
            groups: self.groups.clone(),
        }
    }

    pub fn push_undo(&mut self) {
        let snap = self.snapshot();
        self.push_undo_snapshot(snap);
    }

    /// Pushes a pre-taken snapshot — used by ops that only commit on success,
    /// so a failed edit never pollutes the undo/redo stacks.
    pub fn push_undo_snapshot(&mut self, snap: Snapshot) {
        self.undo_stack.push(snap);
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn restore(&mut self, snap: Snapshot) {
        self.frames = snap.frames;
        self.width = snap.width;
        self.height = snap.height;
        self.groups = snap.groups;
    }

    /// Enforces the group invariant after any mutation that can reorder or
    /// remove frames: each surviving group keeps only the LONGEST contiguous
    /// run of its members (stragglers lose their `group_id`), and group metas
    /// with no remaining members are dropped. Idempotent.
    pub fn normalize_groups(&mut self) {
        use std::collections::HashSet;
        let mut live: HashSet<u64> = HashSet::new();
        let group_ids: Vec<u64> = self.groups.iter().map(|g| g.id).collect();
        for gid in group_ids {
            // Longest contiguous run of this group's members in display order.
            let (mut best_start, mut best_len) = (0usize, 0usize);
            let (mut cur_start, mut cur_len) = (0usize, 0usize);
            for (i, f) in self.frames.iter().enumerate() {
                if f.group_id == Some(gid) {
                    if cur_len == 0 {
                        cur_start = i;
                    }
                    cur_len += 1;
                    if cur_len > best_len {
                        best_len = cur_len;
                        best_start = cur_start;
                    }
                } else {
                    cur_len = 0;
                }
            }
            if best_len == 0 {
                continue; // no members left → meta dropped below
            }
            live.insert(gid);
            for (i, f) in self.frames.iter_mut().enumerate() {
                if f.group_id == Some(gid) && !(best_start..best_start + best_len).contains(&i) {
                    f.group_id = None;
                }
            }
        }
        self.groups.retain(|g| live.contains(&g.id));
    }

    /// Deletes the spool file; call when the session is discarded.
    pub fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.spool_path);
    }

    pub fn spool_path(&self) -> &Path {
        &self.spool_path
    }

    /// Nominal per-frame delay for the capture fps.
    pub fn nominal_delay_ms(fps: u32) -> u32 {
        (1000 / fps.max(1)).max(10)
    }
}

/// Mean absolute BGRA difference between two frames, sampled every 13th pixel
/// and normalized to 0.0..=1.0 (0 = identical, 1 = maximal). Same dimensions
/// are assumed (a session invariant). A read failure yields 1.0 so callers
/// never fold frames across an unreadable buffer. Used by trim/merge.
pub fn frame_diff(session: &mut Session, a: PixelRef, b: PixelRef) -> f64 {
    let (pa, pb) = match (session.read_pixels(a), session.read_pixels(b)) {
        (Ok(pa), Ok(pb)) => (pa, pb),
        _ => return 1.0,
    };
    let n = pa.len().min(pb.len());
    if n < 4 {
        return if pa.len() == pb.len() { 0.0 } else { 1.0 };
    }
    // Step 13 whole pixels (52 bytes) so every channel (B/G/R/A) stays aligned
    // and contributes evenly to the sample.
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    let mut i = 0;
    while i + 4 <= n {
        for c in 0..4 {
            sum += (pa[i + c] as i32 - pb[i + c] as i32).unsigned_abs() as u64;
        }
        count += 4;
        i += 13 * 4;
    }
    if count == 0 {
        return 0.0;
    }
    sum as f64 / (count as f64 * 255.0)
}

/// Merges `source`'s frames into `target` at `insert_at` (a frame index),
/// copying the source pixels into the target spool. All inserted frames become
/// ONE new group (auto number/color). Records a single undo snapshot so the
/// whole merge reverts as one step. `source` is left untouched (the caller
/// cleans it up); on error `target` keeps its frames/groups unchanged.
pub fn merge_session(
    target: &mut Session,
    source: &mut Session,
    insert_at: usize,
) -> Result<(), String> {
    if source.width != target.width || source.height != target.height {
        return Err(format!(
            "recording size {}×{} does not match the session ({}×{})",
            source.width, source.height, target.width, target.height
        ));
    }
    if source.frames.is_empty() {
        return Ok(()); // nothing to merge — leave the undo stack clean
    }
    let insert_at = insert_at.min(target.frames.len());
    // Snapshot BEFORE any change to frames/groups so undo reverts fully.
    let snapshot = target.snapshot();

    let number = target.groups.iter().map(|g| g.number).max().unwrap_or(0) + 1;
    let color = (target.groups.len() % 6) as u8;
    let group_id = target.next_group_id;

    // Copy each UNIQUE source pixel buffer once (dedup by spool offset), remap
    // PixelRefs into the target spool, and mint fresh frame ids.
    use std::collections::HashMap;
    let mut ref_map: HashMap<u64, PixelRef> = HashMap::new();
    let mut inserted: Vec<Frame> = Vec::with_capacity(source.frames.len());
    for sframe in source.frames.clone() {
        let pixels = match ref_map.get(&sframe.pixels.offset) {
            Some(r) => *r,
            None => {
                let buf = source
                    .read_pixels(sframe.pixels)
                    .map_err(|e| format!("read source frame: {e}"))?;
                let r = target
                    .append_pixels(&buf, sframe.pixels.width, sframe.pixels.height)
                    .map_err(|e| format!("write frame: {e}"))?;
                ref_map.insert(sframe.pixels.offset, r);
                r
            }
        };
        let id = target.next_frame_id;
        target.next_frame_id += 1;
        inserted.push(Frame {
            id,
            delay_ms: sframe.delay_ms,
            pixels,
            group_id: Some(group_id),
        });
    }

    // Commit: everything below is infallible, so the snapshot/undo state is
    // only touched once the merge is guaranteed to complete.
    target.push_undo_snapshot(snapshot);
    target.next_group_id += 1;
    target.groups.push(GroupMeta { id: group_id, number, color });
    let tail = target.frames.split_off(insert_at);
    target.frames.extend(inserted);
    target.frames.extend(tail);
    Ok(())
}
