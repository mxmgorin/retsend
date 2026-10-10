#!/bin/sh
# spruceOS (Miyoo Mini Plus / Mini Flip) launcher.
. /mnt/SDCARD/spruce/scripts/helperFunctions.sh
gamedir=$(cd "$(dirname "$0")" && pwd)
cd "$gamedir" || exit 1

# A Miyoo Mini build; guards against `devices` in config.json going stale.
if [ "$PLATFORM" != "MiyooMini" ]; then
  log_message "Retsend: this package is the Miyoo Mini build, but PLATFORM is $PLATFORM"
  exit 1
fi

# spruce brings Wi-Fi up only at boot and only with this set.
if [ "$(jq -r '.wifi // 0' "$SYSTEM_JSON" 2>/dev/null)" != "1" ]; then
  log_message "Retsend: wifi is off in spruce settings; nothing will be discovered"
fi

# Bundled SDL2 first: spruce's own names its drivers differently (lib/README.md).
# lib/fallback last, for libraries the card lacks.
export LD_LIBRARY_PATH="$gamedir/lib:$LD_LIBRARY_PATH:$gamedir/lib/fallback"
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
# spruce's MENU handler ignores apps launched from App/.
export RETSEND_MENU_QUIT=1
#export RETSEND_LOG_LEVEL=debug

# `exec`, so SIGTERM reaches the app, not a shell, and the sockets close cleanly.
exec ./retsend >log.txt 2>&1
