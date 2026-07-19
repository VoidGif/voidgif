# VoidGif

**Record your screen. Edit every frame. Export beautiful GIFs.**

VoidGif is a free, lightweight screen-to-GIF recorder and frame editor for
Windows. Truly free — no watermark, no time limit, no subscription, no ads,
and fully offline: it makes no network connections and collects no data.

🌐 Website: https://voidgif.github.io · 🔒 [Privacy policy](https://voidgif.github.io/privacy/)

## Highlights

- **Capture frame recording** — a movable, resizable window whose transparent
  center defines the region; remembers its size and position. Full-screen mode
  via a checkbox. Up to 60fps (Windows Graphics Capture).
- **Multi-take editing** — continue recording and insert the new take at the
  start, after the current frame, or at the end; each take is auto-grouped as a
  colored block you can move as a unit.
- **Frame editor** — delete, duplicate, reorder, per-frame delay, crop, resize,
  playback speed, undo/redo; one-click static-edge trim and duplicate-frame
  merge; loop finishing (ping-pong, seam preview). Virtualized filmstrip stays
  fast with hundreds of frames.
- **Best-in-class GIF quality** — encoded with [gifski] (thousands of colors
  per frame, temporal dithering). Also exports lossless APNG, PNG sequences,
  and H.264 MP4 (via Windows Media Foundation — no ffmpeg).
- **Size-aware export** — live file-size estimate before export, plus
  GitHub / Discord / Slack / X presets that auto-fit your GIF under the limit.
- **Safety** — auto-save with crash recovery; copy the result straight to the
  clipboard.
- Dark & light themes · English / 한국어 / 日本語 · global hotkeys (F7/F8) ·
  ~15 MB, no runtime dependencies.

## Building

Prerequisites: Rust (stable, MSVC), Node.js 20+, and the Visual Studio C++
Build Tools with the Windows SDK.

```
npm install
npx tauri dev      # run in development
npx tauri build    # produce installers (msi / nsis)
cd src-tauri && cargo test
```

The macOS backend (ScreenCaptureKit) is scaffolded but not yet verified;
macOS builds exclude AGPL components and use an MIT GIF encoder
(see `src-tauri/Cargo.toml`).

## License

VoidGif is licensed under the **GNU AGPL-3.0-or-later** (see [LICENSE](LICENSE)).
GIF encoding on Windows uses [gifski] (AGPL-3.0). macOS builds are configured
so that no AGPL components are compiled in.

[gifski]: https://gif.ski
