# retsend for spruceOS (Miyoo Mini)

Send and receive files between a Miyoo Mini Plus or Mini Flip and your phone or PC
over Wi-Fi, using [LocalSend](https://localsend.org).

**Miyoo Mini Plus and Mini Flip only**: the original Mini has no Wi-Fi.
For spruce's aarch64 handhelds (Miyoo Flip, TrimUI, Anbernic RG-XX and others)
use `retsend-spruceos-aarch64.zip`.

## Install

Unzip `retsend-spruceos-armhf.zip` at the root of the SD card. The app lands in
`App/Retsend/` and shows up under Apps.

## Setup

1. Install [LocalSend](https://localsend.org) on your phone or PC.
2. Connect both devices to the same Wi-Fi network (turn Wi-Fi on in spruce's settings: it only comes up at boot).
3. Launch Retsend: nearby devices appear on the radar.

With Wi-Fi off, the launcher writes a warning to `Saves/spruce/spruce.log`.

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

## Notes

- spruce's MENU handler ignores apps launched from `App/`, so the app handles
  MENU itself and closes its transfers before it exits.
- The launcher `exec`s the app, so the power-off signal reaches it and peers see
  transfers end cleanly.
- No GPU: the launcher sets `RETSEND_SOFTWARE` and `RETSEND_BLIT`. The bundled
  SDL2 in `lib/` is required: spruce's own build names its drivers `mmiyoo` and
  `MMIYOO`, not `Mini` and `Miyoo Mini`.
- `devices` in `config.json` lists the Plus and Flip names PyUI uses; the launcher
  also checks `PLATFORM`.

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- SDL2 for the Miyoo Mini by [Steward Fu](https://github.com/steward-fu/sdl2)
  (zlib, LGPL-2.1 drivers); provenance and licences in `lib/README.md`
- [spruceOS](https://github.com/spruceUI/spruceOS) by the spruceUI team; this package
  follows its `App/` layout
- Source and issues: https://github.com/mxmgorin/retsend
