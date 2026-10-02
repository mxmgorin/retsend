# retsend for spruceOS

Transfer files between a Miyoo Mini Plus or Mini Flip and your phone or PC over
wifi, using [LocalSend](https://localsend.org).

**Miyoo Mini Plus and Mini Flip only.** The original Mini has no wifi, so there is
nothing for this to talk over, and it is left out of the device list in
`config.json`. spruceOS also runs on the Miyoo Flip, the TrimUI handhelds and the
Anbernic RG-XX series; those are aarch64, and `retsend-spruceos-aarch64.zip` is the
package for them.

## Install

Unzip `retsend-spruceos.zip` into the root of the SD card, so that the app lands in
`App/Retsend/`. It shows up under **Apps**, sorted by name, and **MENU quits** it.

## Setup

1. Install LocalSend on your phone or PC (localsend.org).
2. Turn wifi on in spruce's settings and connect both devices to the same network.
   spruce brings the radio up at boot and only when that setting is on — the
   launcher writes a line into `Saves/spruce/spruce.log` if it finds it off, since
   an empty radar otherwise looks like a bug.
3. Launch Retsend — nearby devices appear on the radar.

Received files land in `/mnt/SDCARD/Roms` by default; change the folder in Settings.
The config lives in `App/Retsend/data/config.toml`, and the last run's log in
`App/Retsend/log.txt`.

## Controls

| Button  | Action                                                  |
|---------|---------------------------------------------------------|
| D-pad   | Navigate                                                |
| A       | Send to device / select file / accept / type (keyboard) |
| B       | Back / decline / cancel / erase (keyboard)              |
| Start   | Settings · confirm send · OK (keyboard)                 |
| Select  | Refresh radar · switch roots · layer (keyboard)         |
| L1/R1   | Page through lists                                      |
| MENU    | Quit                                                    |

## Notes for this device

MENU is the way out, and here it is genuinely ours. spruce's own MENU handler runs
`prepare_game_switcher`, which returns early for anything whose command does not
come from `Emu/` — an app in `App/` is left alone, killed by nothing. So the app
answers the key itself and shuts the transfers down on its way out rather than
being cut off where the frame stands. Powering off arrives as a signal: the
launcher `exec`s the app so it lands there rather than on a shell holding it, and
`SIGTERM` stops announcing and closes the sockets first, so a peer sees a transfer
end rather than a connection that stopped answering.

The panel driver has no GPU behind it, so the launcher asks for the software
renderer and for the driver's one presentation path (`RETSEND_SOFTWARE`,
`RETSEND_BLIT`); the app bundles an SDL2 carrying that driver in `lib/`, as every
SDL2 port here does. It must be the bundled one: spruce ships its own Miyoo SDL2,
a different build whose video and render drivers answer to `mmiyoo` and `MMIYOO`
rather than `Mini` and `Miyoo Mini`.

`devices` in `config.json` names the four spellings PyUI uses for the Plus and the
Flip, so the entry stays hidden on the original Mini and on the aarch64 handhelds
spruce also supports. The launcher checks `PLATFORM` as well, in case that list
ever falls behind a firmware rename.

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin/)
- Implements the [LocalSend](https://localsend.org) protocol
- Bundled SDL2 for the Miyoo Mini by
  [Steward Fu](https://github.com/steward-fu/sdl2) (zlib, with LGPL-2.1 drivers),
  built here — provenance and licences in `lib/README.md`
- [spruceOS](https://github.com/spruceUI/spruceOS) is the spruceUI team's; this
  package only follows its `App/` layout
- Source and issues: https://github.com/mxmgorin/retsend
