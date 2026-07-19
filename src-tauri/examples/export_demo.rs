//! Exports a .voidgif project to GIF for quality inspection:
//! `cargo run --example export_demo -- in.voidgif out.gif [quality]`

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use voidgif_lib::export::{execute, ExportFormat, ExportPlan, ExportSettings, ProgressFn};
use voidgif_lib::project;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: export_demo in.voidgif out.gif [quality]");
    let output = args.next().expect("missing output path");
    let quality: u8 = args.next().and_then(|a| a.parse().ok()).unwrap_or(95);

    let dir = std::env::temp_dir().join("voidgif-example");
    let session = project::load(&input, &dir, "export-demo".into()).expect("load project");

    let settings = ExportSettings {
        format: if output.ends_with(".png") {
            ExportFormat::Apng
        } else if output.ends_with(".mp4") {
            ExportFormat::Mp4
        } else {
            ExportFormat::Gif
        },
        path: output.clone(),
        quality,
        width: None,
        loop_: Some(true),
        fast: false,
    };
    let progress: ProgressFn = Arc::new(|_| {});
    execute(
        &mut ExportPlan::from_session(&session).unwrap(),
        &settings,
        &Arc::new(AtomicBool::new(false)),
        &progress,
    )
    .expect("export");
    session.cleanup();

    let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
    println!("wrote {output} ({size} bytes)");
}
