# retsend for muOS

Transfer files between a muOS handheld and your phone or PC over Wi-Fi,
using [LocalSend](https://localsend.org).

The binary and a launcher, no bundled libraries: muOS carries its own SDL2. It
installs under **Applications**; `retsend-portmaster.zip` puts the same app under
Ports instead.

## Install

Copy `retsend-muos.muxapp` to `ARCHIVE/` on the SD card and install it from
**Applications → Archive Manager**. It unpacks into `MUOS/application/Retsend/`.

## Setup

1. Install [LocalSend](https://localsend.org) on your phone or PC.
2. Connect both devices to the same Wi-Fi network (in muOS's settings).
3. Launch Retsend: nearby devices appear on the radar.

| What | Where |
| --- | --- |
| Received files | `ROMS/` on the boot card (change in Settings) |
| Config | `MUOS/application/Retsend/data/config.toml` |
| Log | `MUOS/application/Retsend/log.txt` |

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
| B (home screen) | Quit |

## Notes

- muOS expects an app to quit itself; its own `R2+SELECT+B` kills the process
  mid-transfer. The launcher sets `RETSEND_BACK_QUIT`, so B on the home screen
  closes the transfers first.
- The launcher holds `CAFFEINE` on, so the device does not sleep mid-transfer.
- `SETUP_APP` sets up the governor and the controller mapping; its `HOME` is
  read-only, so the launcher moves it to the app folder. Older muOS without
  `SETUP_APP` gets the same steps spelled out.
- The app glyph is `glyph/retsend.png`, 24×24 raster: muOS draws it unscaled, and
  the SVG path renders nothing on device. A theme's own `retsend` glyph wins.

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- [muOS](https://muos.dev) by the MustardOS team; this package follows its
  application layout
- Source and issues: https://github.com/mxmgorin/retsend
