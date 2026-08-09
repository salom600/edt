# Third-Party Notices

This file lists all third-party software used by the edt project, along
with their licenses. It is maintained manually; if you spot something
missing, please open an issue or PR.

## System dependencies

### FFmpeg — LGPL 2.1+ (or GPL, depending on build)

edt shells out to the `ffmpeg` and `ffprobe` binaries via `std::process::Command`.
edt does **not** link FFmpeg's libraries, so the LGPL/GPL terms of FFmpeg
do not extend to edt itself. Users must install FFmpeg separately.

- Homepage: <https://ffmpeg.org/>
- License: <https://ffmpeg.org/legal.html>
- Source: <https://git.ffmpeg.org/ffmpeg.git>

If you redistribute edt together with a bundled FFmpeg binary, you must
also redistribute FFmpeg's source code and respect its license terms.

## Rust dependencies

All dependencies are pulled from <https://crates.io> and are listed with
their exact versions in [`Cargo.lock`](Cargo.lock). Their licenses are
summarized below; the authoritative source is each crate's
`Cargo.toml` on crates.io.

### Direct dependencies

| Crate | License | Purpose |
|---|---|---|
| `egui` / `eframe` / `egui_extras` | MIT OR Apache-2.0 | Immediate-mode GUI |
| `rfd` | MIT OR Apache-2.0 | Native file dialogs |
| `serde` / `serde_json` | MIT OR Apache-2.0 | Serialization |
| `thiserror` | MIT OR Apache-2.0 | Ergonomic error enums |
| `anyhow` | MIT OR Apache-2.0 | Application error handling |
| `tracing` / `tracing-subscriber` | MIT | Structured logging |
| `image` | MIT OR Apache-2.0 | Image decoding & resizing |
| `directories` | MIT OR Apache-2.0 | OS-standard directories |
| `parking_lot` | MIT OR Apache-2.0 | Fast synchronization primitives |
| `crossbeam-channel` | MIT OR Apache-2.0 | Multi-producer multi-consumer channels |
| `once_cell` | MIT OR Apache-2.0 | Lazy static initialization |

### Transitive dependencies

Transitive dependencies include (non-exhaustive):

- `ab_glyph`, `ahash`, `allocator-api2`, `android-activity`,
  `arboard`, `atomic-waker`, `autocfg`, `base64`, `bitflags`,
  `block2`, `block-buffer`, `bytemuck`, `byteorder`, `bytes`,
  `cc`, `cfg-if`, `cfg_aliases`, `cgl`, `clipboard-win`, `cocoa`,
  `color_quant`, `core-foundation`, `core-graphics`, `core-graphics-types`,
  `core_maths`, `cpufeatures`, `crc32fast`, `crossbeam-deque`,
  `crossbeam-epoch`, `crossbeam-utils`, `crypto-common`, `cursor-icon`,
  `darling`, `derivative`, `digest`, `directories-sys`, `dispatch`,
  `dlib`, `downcast-rs`, `ecolor`, `egui-wgpu`, `egui_glow`, `either`,
  `emath`, `encase`, `encoding_rs`, `epaint`, `equivalent`,
  `errno`, `error-code`, `exr`, `fastrand`, `fdeflate`, `flate2`,
  `flume`, `fnv`, `foreign-types`, `foreign-types-shared`, `form_urlencoded`,
  `futures-core`, `futures-sink`, `futures-task`, `futures-util`,
  `gethostname`, `getrandom`, `gif`, `gl_generator`, `glam`, `glob`,
  `glutin`, `glutin-winit`, `gloo-timers`, `half`, `hashbrown`,
  `hermit-abi`, `home`, `icrate`, `idna`, `image`, `indexmap`,
  `iovec`, `itoa`, `jni`, `jobserver`, `jpeg-decoder`, `js-sys`,
  `khronos_api`, `lazy_static`, `leb128`, `lewton`, `libc`,
  `libloading`, `libredox`, `linux-raw-sys`, `lock_api`, `log`,
  `malloc_buf`, `memchr`, `memmap2`, `metal`, `miniz_oxide`,
  `mio`, `ndk`, `ndk-context`, `ndk-sys`, `nix`, `num-traits`,
  `num_cpus`, `objc`, `objc2`, `objc2-app-kit`, `objc2-foundation`,
  `object`, `once_cell`, `orbclient`, `percent-encoding`, `pin-project-lite`,
  `pkg-config`, `png`, `polling`, `proc-macro2`, `quick-xml`, `quote`,
  `raw-window-handle`, `raw-window-handle0-5`, `rayon`, `rayon-core`,
  `redox_syscall`, `redox_users`, `renderdoc-sys`, `rfd`, `rustix`,
  `rustversion`, `same-file`, `scoped-tls`, `scopeguard`, `sctk-adwaita`,
  `serde`, `serde_derive`, `serde_repr`, `sha1`, `sha2`, `simba`,
  `slotmap`, `smallvec`, `smithay-client-toolkit`, `smithay-clipboard`,
  `smol_str`, `socket2`, `spin`, `static_assertions`, `strict-num`,
  `syn`, `sysinfo`, `thiserror`, `thiserror-impl`, `thread_local`,
  `tiff`, `tiny-skia`, `tiny-skia-path`, `toml_datetime`,
  `toml_edit`, `tracing`, `tracing-attributes`, `tracing-core`,
  `tracing-log`, `tracing-subscriber`, `ttf-parser`, `unicode-bidi`,
  `unicode-ident`, `unicode-normalization`, `unicode-segmentation`,
  `url`, `urlencoding`, `usvg`, `vec_map`, `version_check`, `wasi`,
  `wasm-bindgen`, `wasm-bindgen-futures`, `wasm-bindgen-macro`,
  `wasm-bindgen-macro-support`, `wasm-bindgen-shared`, `wayland-backend`,
  `wayland-client`, `wayland-csd-frame`, `wayland-protocols`,
  `wayland-scanner`, `wayland-sys`, `web-sys`, `web-time`,
  `webbrowser`, `wgpu`, `wgpu-core`, `wgpu-hal`, `wgpu-types`,
  `widestring`, `winapi`, `winapi-util`, `winit`, `winres`,
  `x11-dl`, `x11rb`, `x11rb-protocol`, `xdg-home`, `zune-core`,
  `zune-inflate`, `zune-jpeg`

All of these are MIT, Apache-2.0, BSD, ISC, MPL-2.0, Zlib, or
Unicode-DFS-2016 licensed — all permissive and compatible with edt's
dual MIT/Apache-2.0 license.

If you need the exact license text for any transitive dependency,
check the `Cargo.toml` of that crate on crates.io or run:

```sh
cargo license --workspace --avoid-dev-deps
```

(after `cargo install cargo-license`).

## edt source

edt itself is dual-licensed under MIT OR Apache-2.0. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
