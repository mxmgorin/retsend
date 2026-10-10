#!/bin/sh
# Allium (Miyoo Mini Plus / Flip) launcher.
gamedir=$(cd "$(dirname "$0")" && pwd)
cd "$gamedir" || exit 1

# Bundled SDL2 first (lib/README.md); lib/fallback last, for libraries the card lacks.
export LD_LIBRARY_PATH="$gamedir/lib:/mnt/SDCARD/miyoo/lib:/lib:/config/lib:/customer/lib:$gamedir/lib/fallback"
export LD_PRELOAD="$gamedir/lib/libSDL2-2.0.so.0"
export SDL_VIDEODRIVER=Mini
export EGL_VIDEODRIVER=Mini
# SDL's `software` driver comes first otherwise and never reaches the panel.
export SDL_RENDER_DRIVER="Miyoo Mini"

# The stock HOME is read-only rootfs; keep writable paths on the card.
export HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR=/mnt/SDCARD/Roms
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
export RETSEND_SOFTWARE=1 # no GPU on the SSD202
export RETSEND_BLIT=1     # the panel driver shows texture copies and nothing else
# Layout detection expects the driver name `mmiyoo`; this build says `Mini`.
export RETSEND_KEYMAP=miyoo
# Allium has no kill helper and ignores a bare MENU.
export RETSEND_MENU_QUIT=1
#export RETSEND_LOG_LEVEL=debug

# `exec`, so alliumd's stop and terminate signals reach the app, not a shell.
exec ./retsend >log.txt 2>&1
