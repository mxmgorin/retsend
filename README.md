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

`retsend` (**ret**ro + **send**ing) is a Rust [LocalSend](https://localsend.org)
client for retro handhelds: send and receive files with your phone
or PC over Wi-Fi, no cable or SSH. Compatible with the official LocalSend apps.

It targets PortMaster-compatible Linux handhelds (**Knulli, muOS, ROCKNIX** —
Anbernic/TrimUI class devices, gamepad-only, no compositor) and runs on
regular desktop Linux too.

## Why?

Getting files onto a handheld usually means pulling the SD card or
setting up SSH/SMB. Phones and desktops already have LocalSend —
this is the missing end: a client built for a gamepad and screen, moving files both ways.

## Features

- **Discovery** — a live radar of nearby devices (LocalSend protocol v2.1), with
  a manually typed IP address for the networks that block multicast.
- **Receive** — accept/decline dialog with countdown, speed/ETA, and cancel from either side; quick-save mode auto-accepts.
- **Send** — gamepad file browser with multi-select and per-file progress; pin the folders and files you send often and they lead every listing. A send is a flat list of files: the protocol has no folders.
- **Save routes** — received ROMs land in the console folder they belong to, detected from the card; per-extension overrides on top.
- **Encryption** — the protocol's HTTPS mode, on by default; works with the official app's default settings both ways.
- **Settings on device** — alias, save folder, and port, applied live.
- **Headless** — `retsend --receive` runs with no screen, for SSH and scripting.
- **Small and simple** — no async runtime, a minimal HTTP server, system SDL2.

## Install (PortMaster devices)

Grab `retsend-portmaster.zip` from
[Releases](https://github.com/mxmgorin/retsend/releases) and unpack it
into your ports folder (e.g. `/roms/ports/`).

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
| B            | Esc        | Back · decline · cancel · leave the keyboard  |
| X            | X / Bksp   | Add a device by IP · erase a character · take every file in the folder |
| Y            | Y          | Pin / unpin the row under the cursor          |
| Start        | F1         | Confirm send · OK (keyboard)                  |
| Select       | Tab / F5   | Refresh radar · switch roots · layer (keyboard)|
| L1 / R1      | PgUp / PgDn| Switch tabs · page the file browser           |

## Configuration

`config.toml` lives in the data dir (created with defaults on first run). The
Settings screen edits everything in it except:

- `[network] https = false` — fall back to the protocol's plain-http mode
- `[network] announce_interval_secs` — multicast announce cadence
- `[transfer] browser_roots` — extra mount points for the file browser
- `[transfer] history_limit` — max transfers kept in the History tab (default 200)
- `[transfer] pinned_paths`, `last_send_dir` — written by Y and by sending

Environment variables override paths and control logging at launch:
`RETSEND_DATA_DIR`, `RETSEND_CONFIG`, `RETSEND_SAVE_DIR`, `RETSEND_SCALE`,
`RETSEND_GLES=0|1`, `RETSEND_LOG_LEVEL`, `RETSEND_LOG_FILE`,
`RETSEND_PANIC_FILE`.

### Save routes

Received files land in `save_dir`, except ROMs: those are routed by extension
into the console folders already present in `save_dir` — `.gba` into `gba`,
`.sfc` into `snes`. The folder names come from KNULLI, ROCKNIX, and muOS, which
disagree (Mega Drive is `megadrive`, `genesis`, or `md` — all recognized), and
renamed folders like `Nintendo Game Boy Advance (GBA)` are matched by the code in
parentheses. Nothing is created, so a missing folder gets no route, and ambiguous
extensions (`.zip`, `.iso`, `.bin`, `.md`) never take part. Off via
**Settings → Auto save routes** or `[transfer] auto_routes = false`.

Per-extension overrides win over the detected routes — **Settings → Save routes**
on the device, or:

```toml
[transfer.routes]
gbc = "gb"                    # relative → <save_dir>/gb
gba = "/roms/gba"             # absolute → used as-is
png = "/roms/screenshots"
```

Extensions match case-insensitively, folders are created on demand, and anything
unrouted lands in `save_dir`.
