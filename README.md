# LocalSend Retro

A [LocalSend](https://localsend.org)-protocol file-transfer client for retro
handhelds (Anbernic/TrimUI/Miyoo class devices running muOS/Knulli/ROCKNIX):
push ROMs from your phone or PC to the device, pull saves and screenshots
back — no cable, no SSH. Runs on desktop Linux too.

Built using: Rust,
SDL2 + [egui-sdl2](https://crates.io/crates/egui-sdl2), gamepad-first UI,
blocking networking (threads, no async runtime).

## Status

Early. Working now:

- **Discovery** (LocalSend protocol v2.1): UDP multicast announce/listen on
  `224.0.0.167:53317`, TCP `/register` exchange, radar UI of nearby devices.
- **Receive**: accept/decline modal with a countdown, streaming to `.part`
  with atomic rename, sanitized file names, progress screen with speed/ETA,
  cancel from either side, quick-save (auto-accept) mode.
- **Send**: pick a device on the radar (A), browse and multi-select files
  (D-pad + A, Select cycles mount roots, Start sends); waiting-for-accept
  phase, per-file progress, cancel. CLI arguments pre-select files. HTTP-mode
  peers only for now — turn off encryption in the official app's settings.
- **Settings**: alias (on-screen keyboard), save folder (directory picker),
  port (stepper; the net stack restarts on close), quick save toggle.
- **Headless**: `localsend-retro --receive` — no screen, auto-accept into the
  save dir, progress on stdout. For ssh sessions and scripting.
- App shell: SDL2 window + egui, D-pad/left-stick navigation with hold-repeat,
  keyboard mirror for desktop dev, TOML config, stale `.part` sweep at start.

Next: PortMaster packaging, then HTTPS mode.

## Run (desktop)

```sh
cargo run
```

| Key (pad)          | Action                  |
|--------------------|-------------------------|
| Arrows (D-pad)     | Navigate                |
| Enter (A)          | Confirm / send to peer  |
| Esc (B)            | Back                    |
| F1 (Start)         | Settings                |
| Tab / F5 (Select)  | Re-announce (refresh)   |
| PgUp/PgDn (L1/R1)  | Page through lists      |

Environment: `LSRETRO_DATA_DIR`, `LSRETRO_CONFIG`, `LSRETRO_SAVE_DIR`,
`LSRETRO_SCALE`, `LSRETRO_GLES=0|1`, `LSRETRO_LOG_LEVEL`, `LSRETRO_LOG_FILE`,
`LSRETRO_PANIC_FILE`.

The HTTP port defaults to 53317 and falls back to the next free one when the
official LocalSend app already holds it (the announce carries the real port,
so discovery keeps working).
