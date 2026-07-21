<h1 align="center">LocalSend Retro</h1>

<p align="center">Wireless file transfer for retro handhelds.</p>

<div align="center">
  <a href="https://github.com/mxmgorin/localsend-retro/actions/workflows/ci.yml"><img src="https://github.com/mxmgorin/localsend-retro/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/mxmgorin/localsend-retro/actions/workflows/build-linux-arm.yml"><img src="https://github.com/mxmgorin/localsend-retro/actions/workflows/build-linux-arm.yml/badge.svg" alt="Linux ARM"></a>
  <a href="https://github.com/mxmgorin/localsend-retro/actions/workflows/build-linux.yml"><img src="https://github.com/mxmgorin/localsend-retro/actions/workflows/build-linux.yml/badge.svg" alt="Linux"></a>
  <a href="https://deps.rs/repo/github/mxmgorin/localsend-retro"><img src="https://deps.rs/repo/github/mxmgorin/localsend-retro/status.svg" alt="Dependencies"></a>
</div>

localsend-retro is a [LocalSend](https://localsend.org)-protocol file-transfer
client written in **Rust**, using **SDL2** for windowing and input and
[**egui**](https://github.com/emilk/egui) for the UI. Push ROMs from your phone
or PC to the device and pull saves or screenshots back over wifi — no cable, no
SSH. Fully compatible with the official LocalSend apps.

It targets PortMaster-compatible Linux handhelds (**Knulli, muOS, ROCKNIX** —
Anbernic/TrimUI/Miyoo class devices, gamepad-only, no compositor) and runs on
regular desktop Linux too.

> 🛠️ **Work in progress.** Early development — experimental and bugs are expected.

## Why?

Getting files onto a handheld usually means pulling the SD card or setting up
SSH/SMB. Meanwhile every phone and desktop can run LocalSend. What's missing is
the other end: a client built for a 640×480 screen and a D-pad. This is that
client — the device shows up next to your other LocalSend devices, and files
move in both directions.

## Features

- **Discovery** (LocalSend protocol v2.1) — UDP multicast announce/listen plus
  the TCP `/register` exchange, so devices find each other even on networks
  that drop multicast. A radar screen lists nearby devices live.
- **Receive** — accept/decline dialog with file list, sizes, and a countdown;
  streaming to `.part` files with an atomic rename (an SD yank can't leave a
  truncated file that looks complete); hostile file names sanitized; progress
  with speed and ETA; cancel from either side. A quick-save mode auto-accepts
  into the save folder.
- **Send** — pick a device on the radar, multi-select files in a gamepad file
  browser (selection survives navigating between folders, mount roots one
  button away), watch per-file progress; the receiver declining or picking a
  subset is handled per the protocol.
- **Encryption** — the protocol's https mode, on by default: a self-signed TLS
  identity generated once and persisted, announce fingerprint = its SHA-256.
  Works with the official app's default settings in both directions.
- **Settings on device** — alias via an on-screen keyboard, save folder via a
  directory picker, port stepper, quick-save toggle; the network stack restarts
  in place when needed.
- **Headless mode** — `localsend-retro --receive` runs without a screen:
  auto-accept into the save folder, progress on stdout. For ssh sessions and
  scripting.
- **Small and simple** — blocking networking (threads + channels, no async
  runtime), a hand-rolled minimal HTTP/1.1 server, system SDL2. The protocol
  stack is SDL-free and covered by headless integration tests over real TCP.

## Install (PortMaster devices)

Grab `localsend-retro-portmaster.zip` from
[Releases](https://github.com/mxmgorin/localsend-retro/releases) and unpack it
into your ports folder (e.g. `/roms/ports/`). One ARMv8.0 baseline binary
covers every supported device.

Received files land in the ROMs root by default — change the folder in
Settings.

## Building & running (desktop)

System SDL2 is the only native dependency. On Debian/Ubuntu:

```sh
sudo apt-get install -y build-essential pkg-config libsdl2-dev
cargo run
```

Files passed as arguments (`cargo run -- file1 file2`) come pre-selected in the
send browser.

Two instances on one machine discover each other (multicast loopback) — handy
for trying both sides of a transfer without a second device:

```sh
LSRETRO_DATA_DIR=/tmp/ls-a cargo run &
LSRETRO_DATA_DIR=/tmp/ls-b cargo run
```

Tests are headless (no SDL, no network setup needed):

```sh
cargo test
```

## Controls

| Pad          | Keyboard   | Action                                        |
|--------------|------------|-----------------------------------------------|
| D-pad / stick| Arrows     | Navigate                                      |
| A            | Enter      | Send to device · select file · accept · type  |
| B            | Esc        | Back · decline · cancel · erase (keyboard)    |
| Start        | F1         | Settings · confirm send · OK (keyboard)       |
| Select       | Tab / F5   | Refresh radar · switch roots · layer (keyboard)|
| L1 / R1      | PgUp / PgDn| Page through lists · port ±100 (settings)     |

## Configuration

`config.toml` lives in the data dir (created with defaults on first run) and
everything in it is also editable from the Settings screen, except:

- `[network] https = false` — fall back to the protocol's plain-http mode
- `[network] announce_interval_secs` — multicast announce cadence
- `[transfer] browser_roots` — extra mount points for the file browser

Environment variables override paths and control logging at launch:
`LSRETRO_DATA_DIR`, `LSRETRO_CONFIG`, `LSRETRO_SAVE_DIR`, `LSRETRO_SCALE`,
`LSRETRO_GLES=0|1`, `LSRETRO_LOG_LEVEL`, `LSRETRO_LOG_FILE`,
`LSRETRO_PANIC_FILE`.

The TCP port defaults to 53317 and falls back to the next free one when
something else (say, the official LocalSend app on a dev machine) already
holds it — the announce carries the real port, so discovery keeps working.

## Roadmap

PIN support, favorites, manual IP entry, in-app self-update.

## License

GPL-3.0
