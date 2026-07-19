//! Generates a .voidgif that exercises the Wave 1 editor tools:
//!   `cargo run --example wave1_demo -- out.voidgif`
//!
//! Layout (32 frames): 5 identical idle frames · 20 "animated" frames arranged
//! as 10 distinct images each duplicated once (10 runs of 2) · 5 identical idle
//! frames of a DIFFERENT color. Trimming removes the static ends; merging folds
//! the duplicate pairs; scale/pingpong/seam then act on the result.

use voidgif_lib::project;
use voidgif_lib::session::Session;

fn solid(w: u32, h: u32, b: u8, g: u8, r: u8) -> Vec<u8> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for p in buf.chunks_exact_mut(4) {
        p[0] = b;
        p[1] = g;
        p[2] = r;
        p[3] = 255;
    }
    buf
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "wave1_demo.voidgif".into());
    let (w, h) = (360u32, 240u32);

    let dir = std::env::temp_dir().join("voidgif-wave1");
    let mut s = Session::create(&dir, "wave1".into(), w, h, 24).expect("session");

    // 5 identical idle frames (dark teal).
    let idle_a = solid(w, h, 60, 40, 30);
    for _ in 0..5 {
        s.append_frame(&idle_a, w, h, 42).expect("append");
    }

    // 20 animated frames = 10 distinct colors, each duplicated once.
    for k in 0..10u32 {
        let frame = solid(
            w,
            h,
            (30 + k * 22) as u8,
            (200 - k * 15) as u8,
            (40 + k * 20) as u8,
        );
        s.append_frame(&frame, w, h, 42).expect("append");
        s.append_frame(&frame, w, h, 42).expect("append"); // duplicate
    }

    // 5 identical idle frames (warm — distinct from the leading idle color).
    let idle_b = solid(w, h, 40, 60, 210);
    for _ in 0..5 {
        s.append_frame(&idle_b, w, h, 42).expect("append");
    }

    project::save(&mut s, &out).expect("save");
    s.cleanup();
    println!("wrote {out}: {} frames ({w}x{h})", 30);
}
