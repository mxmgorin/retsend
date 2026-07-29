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

- **Discovery** — a live radar of nearby devices (LocalSend protocol v2.1).
- **Receive** — accept/decline dialog with countdown, speed/ETA, and cancel from either side; quick-save mode auto-accepts.
- **Send** — gamepad file browser with multi-select and per-file progress; pin the folders and files you send often and they lead every listing.
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
| B            | Esc        | Back · decline · cancel · erase (keyboard)    |
| Y            | Y          | Pin / unpin the row under the cursor          |
| Start        | F1         | Confirm send · OK (keyboard)                  |
| Select       | Tab / F5   | Refresh radar · switch roots · layer (keyboard)|
| L1 / R1      | PgUp / PgDn| Switch tabs · page the file browser           |

## Configuration

`config.toml` lives in the data dir (created with defaults on first run) and
everything in it is also editable from the Settings screen, except:

- `[network] https = false` — fall back to the protocol's plain-http mode
- `[network] announce_interval_secs` — multicast announce cadence
- `[transfer] browser_roots` — extra mount points for the file browser
- `[transfer] history_limit` — max transfers kept in the History tab (default 200)

Received files land in `save_dir` by default, but ROMs are sorted on their own:
retsend looks for the console folders already present in `save_dir` and routes by
extension — `.gba` into `gba`, `.sfc` into `snes`, `.ws` into `wswan`. The folder
names come from KNULLI, ROCKNIX, and muOS themselves, which disagree (Mega Drive
is `megadrive`, `genesis`, and `md` respectively — all three are recognized), and
SD-card templates that rename folders to `Nintendo Game Boy Advance (GBA)` are
matched through the code in parentheses. Nothing is created for you: a console
folder that isn't there gets no route, so on desktop this does nothing. Only
extensions that name exactly one console take part — `.zip`, `.iso`, `.bin`, and
`.md` never do. Turn it off with **Settings → Auto save routes** or
`[transfer] auto_routes = false`.

`[transfer.routes]` overrides all of that per file extension, and wins over the
detected routes. Edit it on the device from **Settings → Save routes** (type the
extension, pick the folder), or in the config:

```toml
[transfer.routes]
gbc = "gb"                    # relative → <save_dir>/gb
gba = "/roms/gba"             # absolute → used as-is
png = "/roms/screenshots"
```

Extensions match case-insensitively, folders are created on demand, and
anything without a matching route still lands in `save_dir`.

**Y** pins whatever the cursor is on — a folder you send from often, or a single
file. Pinned rows lead every listing, so a pinned folder is one press away from
anywhere in the tree, and a pinned file can be selected and sent without
navigating at all. Pins live in `[transfer] pinned_paths`; the folder pickers list
only the pinned folders, and a pin on a card that isn't in the slot is skipped
rather than shown dead. The send browser also reopens in the folder your last send
came from (`[transfer] last_send_dir`).

Environment variables override paths and control logging at launch:
`RETSEND_DATA_DIR`, `RETSEND_CONFIG`, `RETSEND_SAVE_DIR`, `RETSEND_SCALE`,
`RETSEND_GLES=0|1`, `RETSEND_LOG_LEVEL`, `RETSEND_LOG_FILE`,
`RETSEND_PANIC_FILE`.
