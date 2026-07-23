<h1 align="center">
  <img src="resources/retsend-wordmark.png" alt="retsend" width="320">
</h1>

<p align="center">Wireless file transfer for retro handhelds.</p>

<div align="center">
  <a href="https://github.com/mxmgorin/retsend/actions/workflows/ci.yml"><img src="https://github.com/mxmgorin/retsend/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/mxmgorin/retsend/actions/workflows/build-linux-arm.yml"><img src="https://github.com/mxmgorin/retsend/actions/workflows/build-linux-arm.yml/badge.svg" alt="Linux ARM"></a>
  <a href="https://github.com/mxmgorin/retsend/actions/workflows/build-linux.yml"><img src="https://github.com/mxmgorin/retsend/actions/workflows/build-linux.yml/badge.svg" alt="Linux"></a>
  <a href="https://deps.rs/repo/github/mxmgorin/retsend"><img src="https://deps.rs/repo/github/mxmgorin/retsend/status.svg" alt="Dependencies"></a>
</div>

retsend is a [LocalSend](https://localsend.org)-protocol file-transfer
client written in **Rust**, using **SDL2** for windowing and input and
[**egui**](https://github.com/emilk/egui) for the UI. Push ROMs from your phone
or PC to the device and pull saves or screenshots back over wifi — no cable, no
SSH. Fully compatible with the official LocalSend apps.

It targets PortMaster-compatible Linux handhelds (**Knulli, muOS, ROCKNIX** —
Anbernic/TrimUI/Miyoo class devices, gamepad-only, no compositor) and runs on
regular desktop Linux too.

## Why?

Getting files onto a handheld usually means pulling the SD card, plugging in a
cable, or setting up SSH/SMB. Meanwhile every phone and desktop can run
LocalSend. What's missing is the other end: a client built for a gamepad and a
screen with no desktop behind it. This is that client — files move in both
directions.

## Features

- **Discovery** (LocalSend protocol v2.1) — UDP multicast plus the TCP
  `/register` exchange; a live radar of nearby devices.
- **Receive** — accept/decline dialog with a countdown, streaming to `.part`
  with an atomic rename, sanitized file names, speed/ETA, cancel from either
  side; a quick-save mode auto-accepts.
- **Send** — gamepad file browser with multi-select that survives folder
  navigation; per-file progress, cancel, partial accepts handled.
- **Encryption** — the protocol's https mode, on by default: a persisted
  self-signed identity, announce fingerprint = its SHA-256. Works with the
  official app's default settings both ways.
- **Settings on device** — alias (on-screen keyboard), save folder picker,
  port; applied live.
- **Headless** — `retsend --receive`: no screen, auto-accept,
  progress on stdout. For ssh sessions and scripting.
- **Small and simple** — threads instead of an async runtime, a minimal
  hand-rolled HTTP server, system SDL2; the protocol stack is SDL-free and
  tested headless over real TCP.

## Install (PortMaster devices)

Grab `retsend-portmaster.zip` from
[Releases](https://github.com/mxmgorin/retsend/releases) and unpack it
into your ports folder (e.g. `/roms/ports/`). Received files land in the ROMs root by default — change the folder in Settings.

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
RETSEND_DATA_DIR=/tmp/ls-a cargo run &
RETSEND_DATA_DIR=/tmp/ls-b cargo run
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
| Start        | F1         | Jump to Settings tab · confirm send · OK (keyboard)|
| Select       | Tab / F5   | Refresh radar · switch roots · layer (keyboard)|
| L1 / R1      | PgUp / PgDn| Switch tabs · page the file browser           |

The main screen is three tabs — **Send** (radar of nearby devices → file
browser), **Receive** (your identity and receive status), **Settings** —
cycled with L1/R1.

## Configuration

`config.toml` lives in the data dir (created with defaults on first run) and
everything in it is also editable from the Settings screen, except:

- `[network] https = false` — fall back to the protocol's plain-http mode
- `[network] announce_interval_secs` — multicast announce cadence
- `[transfer] browser_roots` — extra mount points for the file browser

Received files land in `save_dir` by default. `[transfer.routes]` overrides
that per file extension — handy for dropping ROMs straight into each console's
folder. Edit it on the device from **Settings → Save routes** (type the extension,
pick the folder), or in the config:

```toml
[transfer.routes]
gbc = "gb"                    # relative → <save_dir>/gb
gba = "/roms/gba"             # absolute → used as-is
png = "/roms/screenshots"
```

Extensions match case-insensitively, folders are created on demand, and
anything without a matching route still lands in `save_dir`.

Environment variables override paths and control logging at launch:
`RETSEND_DATA_DIR`, `RETSEND_CONFIG`, `RETSEND_SAVE_DIR`, `RETSEND_SCALE`,
`RETSEND_GLES=0|1`, `RETSEND_LOG_LEVEL`, `RETSEND_LOG_FILE`,
`RETSEND_PANIC_FILE`.

The TCP port defaults to 53317 and falls back to the next free one when
something else (say, the official LocalSend app on a dev machine) already
holds it — the announce carries the real port, so discovery keeps working.
