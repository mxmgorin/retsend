#!/bin/bash
# Knulli (Batocera) Tools launcher. The frontend supplies the pad mapping.
gamedir="$(cd "$(dirname "$0")" && pwd)/retsend"

# The frontend's mapping is positional; with ES's "Switch A/B" on, confirm moves east.
if grep -q '<bool name="InvertButtons" value="true"' \
    /userdata/system/configs/emulationstation/es_settings.cfg 2>/dev/null; then
    SDL_GAMECONTROLLERCONFIG="$(printf '%s\n' "$SDL_GAMECONTROLLERCONFIG" | sed \
        -e 's/,a:/,_a:/; s/,b:/,a:/; s/,_a:/,b:/' \
        -e 's/,x:/,_x:/; s/,y:/,x:/; s/,_x:/,y:/')"
    export SDL_GAMECONTROLLERCONFIG
fi

cd "$gamedir" || exit 1

export HOME="$gamedir"
export XDG_DATA_HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR=/userdata/roms
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
export RETSEND_BACK_QUIT=1
#export RETSEND_LOG_LEVEL=debug

exec "$gamedir/retsend" "$@" > "$gamedir/log.txt" 2>&1
