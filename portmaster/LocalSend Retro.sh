#!/bin/bash

XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi

source "$controlfolder/control.txt"
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

GAMEDIR=/$directory/ports/localsend-retro/

# One ARMv8.0 baseline binary runs on every supported core: unlike retsurf
# (Servo is compute-bound and profits from per-CPU builds), this app is
# bounded by wifi and SD-card speed, and ring's crypto detects NEON/AES
# extensions at runtime anyway.
BINNAME="localsend-retro"
if [ ! -x "$GAMEDIR/$BINNAME" ]; then
  echo "ERROR: no runnable localsend-retro binary found in $GAMEDIR" >&2
  exit 1
fi
BIN="$GAMEDIR/$BINNAME"

cd "$GAMEDIR"

> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1

export HOME="$GAMEDIR"
export XDG_DATA_HOME="$GAMEDIR"
export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"

export LSRETRO_DATA_DIR="$GAMEDIR/data"
# Received files land in the ROMs root by default; change it in Settings.
export LSRETRO_SAVE_DIR="/$directory"
export LSRETRO_PANIC_FILE="$GAMEDIR/localsend-retro-panic.log"
#export LSRETRO_LOG_FILE="$GAMEDIR/localsend-retro.log"
#export LSRETRO_LOG_LEVEL=debug

$GPTOKEYB "$BINNAME" &
pm_platform_helper "$BIN"
"$BIN"

pm_finish
