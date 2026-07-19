//! Generates a synthetic .voidgif project for manual/UI testing:
//! `cargo run --example make_test_project -- out.voidgif [frames] [w] [h]`

use voidgif_lib::project;
use voidgif_lib::session::Session;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "test.voidgif".into());
    let frames: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(24);
    let w: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(320);
    let h: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(200);

    let dir = std::env::temp_dir().join("voidgif-example");
    let mut session = Session::create(&dir, "example".into(), w, h, 30).expect("session");

    for i in 0..frames {
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let cx = (w as f64 / 2.0) + (w as f64 / 3.0) * ((i as f64 / frames as f64) * std::f64::consts::TAU).cos();
        let cy = (h as f64 / 2.0) + (h as f64 / 3.0) * ((i as f64 / frames as f64) * std::f64::consts::TAU).sin();
        for y in 0..h {
            for x in 0..w {
                let o = ((y * w + x) * 4) as usize;
                let d = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt();
                let ball = (1.0 - (d / 28.0).min(1.0)) * 255.0;
                frame[o] = (30 + x * 60 / w) as u8; // B gradient
                frame[o + 1] = (ball * 0.55) as u8 + 20; // G ball
                frame[o + 2] = ball as u8; // R ball
                frame[o + 3] = 255;
            }
        }
        session.append_frame(&frame, w, h, 42).expect("append");
    }

    project::save(&mut session, &out).expect("save");
    session.cleanup();
    println!("wrote {out} ({frames} frames, {w}x{h})");
}
