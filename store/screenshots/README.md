# VoidGif — Store screenshots

Four ready-to-upload screenshots, **1600×900 PNG** (above the Microsoft Store
desktop minimum of 1366×768). Upload them on each language's **Store listing**
page (Step 8 of `../README-submission.md`). Partner Center lets you reuse the
same images across languages.

| File | Screen | Theme |
| --- | --- | --- |
| `editor-takes-dark.png` | **Hero.** Editor — 48-frame filmstrip with two multi-take groups (colored bands + "●N" chips), current-frame ring on a grouped tile | Dark |
| `export-fit-dark.png` | Export dialog — GIF, Discord platform preset selected, live size estimate ("~208 KB / 8 MB ✓") | Dark |
| `home-dark.png` | Home — record button, FPS picker, capture-cursor | Dark |
| `editor-light.png` | Editor — same grouped session, light palette | Light |

All four are uniform **English** UI, captured against the current (icon-only
toolbar, group/take) build.

## A note on the UI language in these shots

The app resolves its config dir via the Windows Known-Folder API, so an
`APPDATA` env-var override is ignored — it always reads the machine's real
`settings.json`. To force a uniform language/theme, back up that file, write
the desired `{"theme":...,"language":"en",...}`, capture, then restore the
backup (this is what the regenerate steps below do).

## Regenerate

```powershell
# 1) a demo project to load into the editor (once — 48 frames so two 12-frame
#    take-groups fit with a clean gap between them):
cargo run --release --example make_test_project --manifest-path src-tauri\Cargo.toml -- "$env:TEMP\vg-demo.voidgif" 48 480 300

# 2) home + both editor shots (PrintWindow, window-only, no input injection):
.\capture-screenshots.ps1 -DemoProject "$env:TEMP\vg-demo.voidgif"

# 3) light-theme editor + export dialog (drives the page over CDP; needs Node 22+):
.\capture-cdp.ps1 -DemoProject "$env:TEMP\vg-demo.voidgif"
```

Both scripts launch throwaway instances with an isolated WebView2 profile,
capture only the app window via `PrintWindow(PW_RENDERFULLCONTENT)`, and kill
every instance they start. `capture-cdp.ps1` uses `cdp-eval.mjs` to toggle the
theme / open the export dialog without injecting OS mouse or keyboard input.

The **2026-07 refresh** (`editor-takes-dark.png` hero + `export-fit-dark.png`)
additionally drove the app over CDP to: select frames 1–12 and 25–36 in the
filmstrip and click the toolbar's "Group" button (`button[aria-label="Group"]`)
to create two take-groups, click a frame inside the second group to move the
current-frame ring onto a grouped tile, set the filmstrip's `scrollLeft` so
both group color bands are in frame, then open Export and click the "Discord"
preset chip. All of that is DOM clicks / scroll dispatched through
`Runtime.evaluate` — still no OS input injection and no product-code changes.
That one-off driver script isn't checked in; the two commands above remain the
supported way to regenerate the base set, and the same group-then-navigate
recipe can be replayed by hand in the running app if you want to refresh the
hero shot again.

Recorder-armed capture-frame shot was skipped this round (not covered by
either script and non-trivial to add safely — the frame window is a separate,
positioned overlay). Add it as a follow-up if it's needed for the listing.
