# retsend for Allium

Send and receive files between a Miyoo Mini Plus or Mini Flip and your phone or PC
over Wi-Fi, using [LocalSend](https://localsend.org).

**Miyoo Mini Plus and Mini Flip only**: the original Mini has no Wi-Fi.

## Install

Unzip `retsend-allium.zip` at the root of the SD card. The app lands in
`Apps/Retsend.pak/` and shows up on the Apps tab. Keep the `.pak` suffix: it is
what makes the folder an app.

## Setup

1. Install [LocalSend](https://localsend.org) on your phone or PC.
2. Connect both devices to the same Wi-Fi network (Wi-Fi is off by default: turn it on in Allium's settings).
3. Launch Retsend: nearby devices appear on the radar.

| What | Where |
| --- | --- |
| Received files | `Roms/` (change in Settings) |
| Config | `Apps/Retsend.pak/data/config.toml` |
| Log | `Apps/Retsend.pak/log.txt` |

## Controls

| Button | Action |
| --- | --- |
| D-pad | Navigate; left/right switch tabs |
| A | Send to a device, select a file, accept, repeat a send, type |
| B | Back, decline, cancel |
| X | Add a device by IP, choose where a transfer lands, delete a history row, take a folder, erase (keyboard) |
| Y | Pin or unpin |
| Start | Confirm a send, OK (keyboard) |
| Select | Refresh the radar, switch roots, keyboard layer |
| L1 / R1 | Switch tabs, page the file browser |
| MENU | Quit |

MENU held with another button stays Allium's: brightness, volume, screenshot.

## In the Games tab

Copy `ports/Retsend.port/` to `Roms/PORTS/Retsend.port/` and the app also shows
up among the games. It runs the install under `Apps/`, so there is still one copy
and one identity on the network. Box art: `Roms/PORTS/Imgs/Retsend.png`.

## Notes

- Allium has no kill helper, so the app handles MENU itself and closes its
  transfers before it exits.
- The launcher `exec`s the app, so power-off and lid-close signals reach it and
  peers see transfers end cleanly.
- No GPU: the launcher sets `RETSEND_SOFTWARE` and `RETSEND_BLIT`, and the bundled
  SDL2 in `lib/` carries the panel driver.

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- SDL2 for the Miyoo Mini by [Steward Fu](https://github.com/steward-fu/sdl2)
  (zlib, LGPL-2.1 drivers); provenance and licences in `lib/README.md`
- [Allium](https://github.com/goweiwen/Allium) by goweiwen; this package follows its
  `Apps/*.pak` layout
- Source and issues: https://github.com/mxmgorin/retsend
