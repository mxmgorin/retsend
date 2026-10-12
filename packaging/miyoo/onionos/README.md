# retsend for OnionOS

Transfer files between a Miyoo Mini Plus or Mini Flip and your phone or PC
over Wi-Fi, using [LocalSend](https://localsend.org).

**Miyoo Mini Plus and Mini Flip only**: the original Mini has no Wi-Fi.

## Install

Unzip `retsend-onionos.zip` at the root of the SD card. The app lands in
`App/Retsend/` and shows up under Apps.

## Setup

1. Install [LocalSend](https://localsend.org) on your phone or PC.
2. Connect both devices to the same Wi-Fi network (Wi-Fi is off by default: turn it on in Onion's settings).
3. Launch Retsend: nearby devices appear on the radar.

| What | Where |
| --- | --- |
| Received files | `Roms/` (change in Settings) |
| Config | `App/Retsend/data/config.toml` |
| Log | `App/Retsend/log.txt` |

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

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- SDL2 for the Miyoo Mini by [Steward Fu](https://github.com/steward-fu/sdl2)
  (zlib, LGPL-2.1 drivers); provenance and licences in `lib/README.md`
- Source and issues: https://github.com/mxmgorin/retsend
