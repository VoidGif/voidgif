//! Vertical-orientation marker project: top half RED, bottom half BLUE.
//! `cargo run --example make_flip_test -- out.voidgif`

use voidgif_lib::project;
use voidgif_lib::session::Session;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "flip.voidgif".into());
    let (w, h) = (320u32, 240u32);
    let dir = std::env::temp_dir().join("voidgif-example");
    let mut session = Session::create(&dir, "flip".into(), w, h, 30).expect("session");
    for _ in 0..12 {
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let o = ((y * w + x) * 4) as usize;
                if y < h / 2 {
                    frame[o + 2] = 230; // top = RED (BGRA)
                } else {
                    frame[o] = 230; // bottom = BLUE
                }
                frame[o + 3] = 255;
            }
        }
        session.append_frame(&frame, w, h, 100).expect("append");
    }
    project::save(&mut session, &out).expect("save");
    session.cleanup();
    println!("wrote {out}");
}
