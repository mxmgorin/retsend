# retsend for OnionOS

Transfer files between a Miyoo Mini Plus or Flip and your phone or PC over wifi, using
the [LocalSend](https://localsend.org).

**Miyoo Mini Plus and Flip only.** The original Mini has no wifi, so there is nothing for
this to talk over.

## Install

Unzip `retsend-onionos.zip` into the root of the SD card, so that the app lands
in `App/Retsend/`. It shows up under Apps.

## Setup

1. Install LocalSend on your phone or PC (localsend.org).
2. Connect both devices to the same wifi network — turn wifi on in Onion's
   settings first; the Plus keeps it off by default.
3. Launch Retsend — nearby devices appear on the radar.

Received files land in `/mnt/SDCARD/Roms` by default; change the folder in
Settings. The config lives in `App/Retsend/data/config.toml`, and the last run's
log in `App/Retsend/log.txt`.

## Controls

| Button  | Action                                                 |
|---------|--------------------------------------------------------|
| D-pad   | Navigate                                               |
| A       | Send to device / select file / accept / type (keyboard) |
| B       | Back / decline / cancel / erase (keyboard)              |
| Start   | Settings · confirm send · OK (keyboard)                 |
| Select  | Refresh radar · switch roots · layer (keyboard)         |
| L1/R1   | Page through lists                                     |
| MENU    | Quit                                                   |

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- Bundled SDL2 for the Miyoo Mini by
  [Steward Fu](https://github.com/steward-fu/sdl2) (zlib, with LGPL-2.1 drivers),
  built here — provenance and licences in `lib/README.md`
- Source and issues: https://github.com/mxmgorin/retsend
