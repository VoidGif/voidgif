# Third-party notices

VoidGif is licensed under the GNU AGPL-3.0-or-later (see [LICENSE](LICENSE)).
It is built on the open-source components below. Each component remains under
its own license; the full text of every license, together with per-project
copyright notices, is available in each project's repository (crates on
https://crates.io, npm packages on https://www.npmjs.com). Exact versions are
pinned in `src-tauri/Cargo.lock` and `package-lock.json`.

## Copyleft components

| Component | License | Notes |
| --- | --- | --- |
| gifski | AGPL-3.0-or-later | GIF encoder (https://gif.ski). Windows builds only; excluded from macOS builds via Cargo target gating. VoidGif's complete corresponding source is published in this repository. |
| imagequant | GPL-3.0-or-later | Palette quantization used by gifski; same target gating. Compatible with this project's AGPL-3.0 license. |
| cssparser, cssparser-macros, dtoa-short, option-ext, selectors | MPL-2.0 | Used unmodified. Source for these files is available from their repositories. |

## JavaScript runtime dependencies

| Package | License |
| --- | --- |
| @tanstack/react-virtual | MIT |
| @tauri-apps/api | MIT OR Apache-2.0 |
| @tauri-apps/plugin-dialog | MIT OR Apache-2.0 |
| react | MIT |
| react-dom | MIT |
| zustand | MIT |

(Build-time tooling — Vite, TypeScript, Tailwind CSS — is MIT-licensed and not
distributed with the application.)

## Rust dependencies by license

### MIT OR Apache-2.0 (147)

anstream v1.0.0, anstyle v1.0.14, anstyle-parse v1.0.0, anstyle-query v1.1.5, anstyle-wincon v3.0.11, anyhow v1.0.103, arrayvec v0.7.8, base64 v0.22.1, bitflags v2.13.0, block-buffer v0.10.4, camino v1.2.4, cargo-platform v0.1.9, cfg-if v1.0.4, colorchoice v1.0.5, cookie v0.18.1, cpufeatures v0.2.17, crc32fast v1.5.0, crossbeam-channel v0.5.16, crossbeam-deque v0.8.7, crossbeam-epoch v0.9.20, crossbeam-utils v0.8.22, crypto-common v0.1.7, deranged v0.5.8, digest v0.10.7, dirs v6.0.0, dirs-sys v0.5.0, displaydoc v0.2.6, document-features v0.2.12, dtoa v1.0.11, dyn-clone v1.0.20, either v1.16.0, env_filter v2.0.0, env_logger v0.11.11, erased-serde v0.4.10, fast_image_resize v6.0.0, fdeflate v0.3.7, flate2 v1.1.9, form_urlencoded v1.2.2, getrandom v0.3.4, getrandom v0.4.3, gif v0.13.3, gif-dispose v5.0.1, glob v0.3.3, hashbrown v0.12.3, hashbrown v0.17.1, heck v0.5.0, html5ever v0.38.0, http v1.4.2, idna v1.1.0, is_terminal_polyfill v1.70.2, itoa v1.0.18, jsonptr v0.6.3, keyboard-types v0.7.0, libc v0.2.186, litrs v1.0.0, lock_api v0.4.14, log v0.4.33, markup5ever v0.38.0, mime v0.3.17, num-conv v0.2.2, num-traits v0.2.19, once_cell v1.21.4, once_cell_polyfill v1.70.2, ordered-channel v1.2.0, parking_lot v0.12.5, parking_lot_core v0.9.12, percent-encoding v2.3.2, png v0.17.16, png v0.18.1, powerfmt v0.2.0, proc-macro2 v1.0.106, quote v1.0.46, rayon v1.12.0, rayon-core v1.13.0, regex v1.13.0, regex-automata v0.4.15, regex-syntax v0.8.11, scopeguard v1.2.0, semver v1.0.28, serde v1.0.228, serde_core v1.0.228, serde_derive v1.0.228, serde_derive_internals v0.29.1, serde_json v1.0.150, serde_repr v0.1.20, serde_spanned v1.1.1, serde_with v3.21.0, serde_with_macros v3.21.0, serde-untagged v0.1.9, serialize-to-javascript v0.1.2, serialize-to-javascript-impl v0.1.2, servo_arc v0.4.3, sha2 v0.10.9, smallvec v1.15.2, softbuffer v0.4.8, stable_deref_trait v1.2.1, string_cache v0.9.0, syn v2.0.118, tendril v0.5.1, thiserror v1.0.69, thiserror v2.0.18, thiserror-impl v1.0.69, thiserror-impl v2.0.18, thread_local v1.1.9, time v0.3.53, time-core v0.1.9, time-macros v0.2.31, toml v1.1.2+spec-1.1.0, toml_datetime v1.1.1+spec-1.1.0, toml_parser v1.1.2+spec-1.1.0, toml_writer v1.1.1+spec-1.1.0, tray-icon v0.24.1, typeid v1.0.3, typenum v1.20.1, unicode-segmentation v1.13.3, url v2.5.8, web_atoms v0.2.5, weezl v0.1.12, windows v0.61.3, windows v0.62.2, windows_x86_64_msvc v0.52.6, windows_x86_64_msvc v0.53.1, windows-collections v0.2.0, windows-collections v0.3.2, windows-core v0.61.2, windows-core v0.62.2, windows-future v0.2.1, windows-future v0.3.2, windows-implement v0.60.2, windows-interface v0.59.3, windows-link v0.1.3, windows-link v0.2.1, windows-numerics v0.2.0, windows-numerics v0.3.1, windows-result v0.3.4, windows-result v0.4.1, windows-strings v0.4.2, windows-strings v0.5.1, windows-sys v0.59.0, windows-sys v0.60.2, windows-sys v0.61.2, windows-targets v0.52.6, windows-targets v0.53.5, windows-threading v0.1.0, windows-threading v0.2.1, windows-version v0.1.7, zstd-safe v7.2.4

### MIT (41)

bytes v1.12.1, cargo_metadata v0.19.2, cfb v0.7.3, color_quant v1.1.0, darling v0.23.0, darling_core v0.23.0, darling_macro v0.23.0, derive_more v2.1.1, derive_more-impl v2.1.1, dom_query v0.27.0, generic-array v0.14.7, ico v0.5.0, infer v0.19.0, loop9 v0.1.5, new_debug_unreachable v1.0.6, phf v0.13.1, phf_generator v0.13.1, phf_macros v0.13.1, phf_shared v0.13.1, plist v1.10.0, precomputed-hash v0.1.1, quick-xml v0.41.0, resize v0.8.9, rfd v0.16.0, rgb v0.8.53, schemars v0.8.22, schemars_derive v0.8.22, simd-adler32 v0.3.9, strsim v0.11.1, synstructure v0.13.2, tokio v1.52.3, tracing v0.1.44, tracing-core v0.1.36, urlpattern v0.3.0, webview2-com v0.38.2, webview2-com-macros v0.8.1, webview2-com-sys v0.38.2, windows-capture v2.0.0, winnow v1.0.3, zmij v1.0.21, zstd v0.13.3

### Apache-2.0 OR MIT (27)

bit-set v0.8.0, bit-vec v0.8.0, ctor v0.8.0, ctor-proc-macro v0.0.7, equivalent v1.0.2, fastrand v2.4.1, global-hotkey v0.8.0, idna_adapter v1.2.2, indexmap v1.9.3, indexmap v2.14.0, muda v0.19.3, pin-project-lite v0.2.17, rustc-hash v2.1.3, tauri v2.11.5, tauri-codegen v2.6.3, tauri-macros v2.6.3, tauri-plugin-dialog v2.7.1, tauri-plugin-fs v2.5.1, tauri-plugin-global-shortcut v2.3.2, tauri-runtime v2.11.3, tauri-runtime-wry v2.11.4, tauri-utils v2.9.3, utf8_iter v1.0.4, utf8parse v0.2.2, uuid v1.23.4, window-vibrancy v0.6.0, wry v0.55.1

### Unicode-3.0 (18)

icu_collections v2.2.0, icu_locale_core v2.2.0, icu_normalizer v2.2.0, icu_normalizer_data v2.2.0, icu_properties v2.2.0, icu_properties_data v2.2.0, icu_provider v2.2.0, litemap v0.8.2, potential_utf v0.1.5, tinystr v0.8.3, writeable v0.6.3, yoke v0.8.3, yoke-derive v0.8.2, zerofrom v0.1.8, zerofrom-derive v0.1.7, zerotrie v0.2.4, zerovec v0.11.6, zerovec-derive v0.11.3

### MIT/Apache-2.0 (11)

bitflags v1.3.2, ident_case v1.0.1, json-patch v3.0.1, quick-error v2.0.1, siphasher v1.0.3, unic-char-property v0.9.0, unic-char-range v0.9.0, unic-common v0.9.0, unic-ucd-ident v0.9.0, unic-ucd-version v0.9.0, zstd-sys v2.0.16+zstd.1.5.7

### Unlicense OR MIT (5)

aho-corasick v1.1.4, byteorder v1.5.0, jiff v0.2.32, memchr v2.8.3, winapi-util v0.1.11

### MPL-2.0 (5)

cssparser v0.36.0, cssparser-macros v0.6.1, dtoa-short v0.3.5, option-ext v0.2.0, selectors v0.36.1

### BSD-3-Clause (2)

alloc-no-stdlib v2.0.4, alloc-stdlib v0.2.4

### Unlicense/MIT (2)

same-file v1.0.6, walkdir v2.5.0

### 0BSD OR MIT OR Apache-2.0 (1)

adler2 v2.0.1

### BSD-3-Clause AND MIT (1)

brotli v8.0.4

### BSD-3-Clause/MIT (1)

brotli-decompressor v5.0.3

### Zlib OR Apache-2.0 OR MIT (1)

bytemuck v1.25.0

### Apache-2.0 AND MIT (1)

dpi v0.1.2

### CC0-1.0 OR MIT-0 OR Apache-2.0 (1)

dunce v1.0.5

### Apache-2.0 / MIT (1)

fnv v1.0.7

### Zlib (1)

foldhash v0.2.0

### AGPL-3.0-or-later (1)

gifski v1.34.0

### GPL-3.0-or-later (1)

imagequant v4.4.1

### CC0-1.0 OR Apache-2.0 (1)

imgref v1.12.2

### MIT OR Zlib OR Apache-2.0 (1)

miniz_oxide v0.8.9

### MIT OR Apache-2.0 OR Zlib (1)

raw-window-handle v0.6.2

### Apache-2.0 (1)

tao v0.35.3

### (MIT OR Apache-2.0) AND Unicode-3.0 (1)

unicode-ident v1.0.24

## License texts

- **MIT**: https://opensource.org/license/mit — "Permission is hereby granted,
  free of charge, to any person obtaining a copy of this software and
  associated documentation files (the 'Software'), to deal in the Software
  without restriction…" (full text at the link; each project's own copy
  carries its copyright line).
- **Apache-2.0**: https://www.apache.org/licenses/LICENSE-2.0
- **BSD-2-Clause / BSD-3-Clause**: https://opensource.org/license/bsd-2-clause /
  https://opensource.org/license/bsd-3-clause
- **ISC**: https://opensource.org/license/isc-license-txt
- **Zlib**: https://opensource.org/license/zlib
- **Unlicense**: https://unlicense.org
- **MPL-2.0**: https://www.mozilla.org/MPL/2.0/
- **GPL-3.0-or-later**: https://www.gnu.org/licenses/gpl-3.0.html
- **AGPL-3.0-or-later**: [LICENSE](LICENSE) in this repository.
