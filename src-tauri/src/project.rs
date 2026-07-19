//! .voidgif project file: save/load a session for later editing.
//!
//! Layout: `VOIDGIF1` magic (8 bytes) · u64 LE json length · JSON metadata ·
//! zstd stream of unique pixel buffers in metadata order. Duplicated frames
//! share pixel entries, so duplicates cost nothing on disk.

use crate::session::{Frame, GroupMeta, PixelRef, Session};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"VOIDGIF1";

#[derive(Serialize, Deserialize)]
struct PixelMeta {
    len: u64,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize)]
struct FrameMeta {
    id: u64,
    delay_ms: u32,
    /// Index into `pixels`.
    pixel: usize,
    /// Owning group id (`None`/absent = ungrouped). Added in version 2.
    #[serde(default)]
    group: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct GroupMetaJson {
    id: u64,
    number: u32,
    color: u8,
}

#[derive(Serialize, Deserialize)]
struct ProjectMeta {
    version: u32,
    width: u32,
    height: u32,
    fps: u32,
    pixels: Vec<PixelMeta>,
    frames: Vec<FrameMeta>,
    /// Frame groups (version 2+). Absent in version 1 files → empty.
    #[serde(default)]
    groups: Vec<GroupMetaJson>,
    #[serde(default)]
    next_group_id: u64,
}

/// Lightweight header for autosave/recovery: what a `.voidgif` contains without
/// decoding any pixels.
#[derive(Debug, Clone, Copy)]
pub struct ProjectSummary {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
}

/// Reads only the magic + JSON metadata header of a `.voidgif` (never the zstd
/// pixel tail), so a recovery prompt can be built cheaply. Used by autosave.
pub fn peek(path: &str) -> Result<ProjectSummary, String> {
    let file = File::open(path).map_err(|e| format!("open project file: {e}"))?;
    let mut input = BufReader::new(file);

    let mut magic = [0u8; 8];
    input.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err("not a VoidGif project file".into());
    }
    let mut len_bytes = [0u8; 8];
    input.read_exact(&mut len_bytes).map_err(|e| e.to_string())?;
    let json_len = u64::from_le_bytes(len_bytes) as usize;
    if json_len > 64 * 1024 * 1024 {
        return Err("corrupt project file (oversized metadata)".into());
    }
    let mut json = vec![0u8; json_len];
    input.read_exact(&mut json).map_err(|e| e.to_string())?;
    let meta: ProjectMeta =
        serde_json::from_slice(&json).map_err(|e| format!("corrupt metadata: {e}"))?;
    Ok(ProjectSummary {
        version: meta.version,
        width: meta.width,
        height: meta.height,
        frames: meta.frames.len() as u32,
    })
}

pub fn save(session: &mut Session, path: &str) -> Result<(), String> {
    // Deduplicate pixel buffers by spool offset (duplicated frames share them).
    let mut pixel_index: HashMap<u64, usize> = HashMap::new();
    let mut unique: Vec<PixelRef> = Vec::new();
    let mut frames_meta = Vec::with_capacity(session.frames.len());
    for frame in session.frames.clone() {
        let idx = *pixel_index.entry(frame.pixels.offset).or_insert_with(|| {
            unique.push(frame.pixels);
            unique.len() - 1
        });
        frames_meta.push(FrameMeta {
            id: frame.id,
            delay_ms: frame.delay_ms,
            pixel: idx,
            group: frame.group_id,
        });
    }

    let meta = ProjectMeta {
        version: 2,
        width: session.width,
        height: session.height,
        fps: session.fps,
        pixels: unique
            .iter()
            .map(|p| PixelMeta { len: p.len, width: p.width, height: p.height })
            .collect(),
        frames: frames_meta,
        groups: session
            .groups
            .iter()
            .map(|g| GroupMetaJson { id: g.id, number: g.number, color: g.color })
            .collect(),
        next_group_id: session.next_group_id,
    };
    let json = serde_json::to_vec(&meta).map_err(|e| e.to_string())?;

    let file = File::create(path).map_err(|e| format!("create project file: {e}"))?;
    let mut out = BufWriter::new(file);
    out.write_all(MAGIC).map_err(|e| e.to_string())?;
    out.write_all(&(json.len() as u64).to_le_bytes())
        .map_err(|e| e.to_string())?;
    out.write_all(&json).map_err(|e| e.to_string())?;

    let mut z = zstd::stream::write::Encoder::new(out, zstd::DEFAULT_COMPRESSION_LEVEL)
        .map_err(|e| e.to_string())?;
    for pixels in unique {
        let buf = session
            .read_pixels(pixels)
            .map_err(|e| format!("read frame: {e}"))?;
        z.write_all(&buf).map_err(|e| e.to_string())?;
    }
    z.finish()
        .map_err(|e| e.to_string())?
        .flush()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load(path: &str, session_dir: &Path, new_id: String) -> Result<Session, String> {
    let file = File::open(path).map_err(|e| format!("open project file: {e}"))?;
    let mut input = BufReader::new(file);

    let mut magic = [0u8; 8];
    input.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err("not a VoidGif project file".into());
    }
    let mut len_bytes = [0u8; 8];
    input.read_exact(&mut len_bytes).map_err(|e| e.to_string())?;
    let json_len = u64::from_le_bytes(len_bytes) as usize;
    if json_len > 64 * 1024 * 1024 {
        return Err("corrupt project file (oversized metadata)".into());
    }
    let mut json = vec![0u8; json_len];
    input.read_exact(&mut json).map_err(|e| e.to_string())?;
    let meta: ProjectMeta = serde_json::from_slice(&json).map_err(|e| format!("corrupt metadata: {e}"))?;
    if meta.version != 1 && meta.version != 2 {
        return Err(format!("unsupported project version {}", meta.version));
    }
    // A group must own at least one frame, so a file can never claim more.
    if meta.groups.len() > meta.frames.len() {
        return Err("corrupt project file (more groups than frames)".into());
    }

    // Hard caps — a crafted file must not be able to allocate huge buffers,
    // fill the temp disk, or overflow the size arithmetic below.
    const MAX_DIM: u32 = 8192;
    const MAX_ENTRIES: usize = 20_000;
    const MAX_TOTAL_PIXEL_BYTES: u64 = 8 << 30; // 8 GiB spool budget
    if meta.width == 0 || meta.height == 0 || meta.width > MAX_DIM || meta.height > MAX_DIM {
        return Err("corrupt project file (session dimensions out of range)".into());
    }
    if meta.pixels.len() > MAX_ENTRIES || meta.frames.len() > MAX_ENTRIES {
        return Err("corrupt project file (too many frames)".into());
    }
    let total: u64 = meta.pixels.iter().map(|p| p.len).sum();
    if total > MAX_TOTAL_PIXEL_BYTES {
        return Err("corrupt project file (pixel data exceeds size budget)".into());
    }

    let mut session = Session::create(session_dir, new_id, meta.width, meta.height, meta.fps)
        .map_err(|e| format!("create session spool: {e}"))?;

    let mut z = zstd::stream::read::Decoder::new(input).map_err(|e| e.to_string())?;
    let mut refs = Vec::with_capacity(meta.pixels.len());
    for p in &meta.pixels {
        if p.width == 0 || p.height == 0 || p.width > MAX_DIM || p.height > MAX_DIM {
            return Err("corrupt project file (frame dimensions out of range)".into());
        }
        let expected = p.width as u64 * p.height as u64 * 4;
        if p.len != expected {
            return Err("corrupt project file (pixel size mismatch)".into());
        }
        let mut buf = vec![0u8; p.len as usize];
        z.read_exact(&mut buf)
            .map_err(|e| format!("corrupt pixel data: {e}"))?;
        let r = session
            .append_pixels(&buf, p.width, p.height)
            .map_err(|e| format!("write spool: {e}"))?;
        refs.push(r);
    }

    // Only honor frame → group refs that point at a real group meta.
    let valid_groups: HashSet<u64> = meta.groups.iter().map(|g| g.id).collect();
    let mut max_id = 0u64;
    for f in &meta.frames {
        let pixels = *refs
            .get(f.pixel)
            .ok_or("corrupt project file (bad pixel index)")?;
        session.frames.push(Frame {
            id: f.id,
            delay_ms: f.delay_ms.clamp(10, 60_000),
            pixels,
            group_id: f.group.filter(|gid| valid_groups.contains(gid)),
        });
        max_id = max_id.max(f.id);
    }
    session.next_frame_id = max_id + 1;

    session.groups = meta
        .groups
        .iter()
        .map(|g| GroupMeta { id: g.id, number: g.number, color: g.color % 6 })
        .collect();
    let max_group = meta.groups.iter().map(|g| g.id + 1).max().unwrap_or(0);
    session.next_group_id = meta.next_group_id.max(max_group);
    // A hand-crafted/older file may have non-contiguous members — repair.
    session.normalize_groups();
    Ok(session)
}
