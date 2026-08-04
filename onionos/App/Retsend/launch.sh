#!/bin/sh
# OnionOS (Miyoo Mini Plus / Flip) launcher.
sysdir=/mnt/SDCARD/.tmp_update
miyoodir=/mnt/SDCARD/miyoo
gamedir=$(cd "$(dirname "$0")" && pwd)
cd "$gamedir" || exit 1

# Our SDL2 first, preloaded like every SDL2 port here; parasyte and miyoo carry
# what it links against (libGLESv2, libpng16, libz, libfreetype, libbz2).
export LD_LIBRARY_PATH="$gamedir/lib:$sysdir/lib/parasyte:$sysdir/lib:$miyoodir/lib:/lib:/config/lib:/customer/lib"
export LD_PRELOAD="$gamedir/lib/libSDL2-2.0.so.0"
export SDL_VIDEODRIVER=mmiyoo
export EGL_VIDEODRIVER=mmiyoo
export SDL_AUDIODRIVER=dsp

# The stock HOME is read-only rootfs; keep writable paths on the card.
export HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR=/mnt/SDCARD/Roms
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
export RETSEND_SOFTWARE=1 # no GPU on the SSD202
export RETSEND_BLIT=1     # mmiyoo shows texture copies and nothing else
#export RETSEND_LOG_LEVEL=debug

# Nothing here raises an SDL quit event, so MENU is the way out.
"$sysdir/bin/pressMenu2Kill" retsend &
./retsend >log.txt 2>&1
pkill -9 pressMenu2Kill
