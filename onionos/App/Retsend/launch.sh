#!/bin/sh
# OnionOS (Miyoo Mini Plus / Flip) launcher.
sysdir=/mnt/SDCARD/.tmp_update
miyoodir=/mnt/SDCARD/miyoo
gamedir=$(cd "$(dirname "$0")" && pwd)
cd "$gamedir" || exit 1

# Our SDL2 first, preloaded like every SDL2 port here; the device carries what it
# links against (libGLESv2 and the libmi_* SoC libraries).
export LD_LIBRARY_PATH="$gamedir/lib:$sysdir/lib/parasyte:$sysdir/lib:$miyoodir/lib:/lib:/config/lib:/customer/lib"
export LD_PRELOAD="$gamedir/lib/libSDL2-2.0.so.0"
export SDL_VIDEODRIVER=Mini
export EGL_VIDEODRIVER=Mini
# SDL lists its own `software` driver ahead of the panel's, and what that one
# draws never reaches the screen, so name the panel's outright.
export SDL_RENDER_DRIVER="Miyoo Mini"
# No SDL_AUDIODRIVER: a file transfer never opens the audio subsystem.

# The stock HOME is read-only rootfs; keep writable paths on the card.
export HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR=/mnt/SDCARD/Roms
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
export RETSEND_SOFTWARE=1 # no GPU on the SSD202
export RETSEND_BLIT=1     # the panel driver shows texture copies and nothing else
# The pad layout follows the video driver's name, which this build spells `Mini`.
export RETSEND_KEYMAP=miyoo
#export RETSEND_LOG_LEVEL=debug

# Nothing here raises an SDL quit event, so MENU is the way out.
"$sysdir/bin/pressMenu2Kill" retsend &
./retsend >log.txt 2>&1
pkill -9 pressMenu2Kill
