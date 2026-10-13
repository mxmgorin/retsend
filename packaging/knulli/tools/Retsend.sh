#!/bin/bash
# Knulli (Batocera) Tools launcher. The frontend supplies the pad mapping, so
# none is set here.
gamedir="$(cd "$(dirname "$0")" && pwd)/retsend"
cd "$gamedir" || exit 1

export HOME="$gamedir"
export XDG_DATA_HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR=/userdata/roms
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
export RETSEND_BACK_QUIT=1
#export RETSEND_LOG_LEVEL=debug

exec "$gamedir/retsend" "$@" > "$gamedir/log.txt" 2>&1
