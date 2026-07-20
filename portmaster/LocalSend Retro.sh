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

# Pick the build matching this device's CPU (same scheme as retsurf). Anything
# unknown falls through to the ARMv8.0 baseline, which runs on every core.
#   0xd05 = Cortex-A55 (RK3566, Allwinner A523)
#   0xd04 = Cortex-A35 (RK3326)
#   0xd03 = Cortex-A53 (H700, Allwinner A133 Plus) — and the sane default.
CPU_PART="$(grep -m1 -i 'CPU part' /proc/cpuinfo | grep -oiE '0x[0-9a-f]+' | head -1 | tr 'A-Z' 'a-z')"
select_binary() {
  case "$CPU_PART" in
    0xd05) grep -qw atomics /proc/cpuinfo && echo "localsend-retro.a55" || echo "localsend-retro.a53" ;;
    0xd04) echo "localsend-retro.a35" ;;
    *)     echo "localsend-retro.a53" ;;
  esac
}

BINNAME="$(select_binary)"
if [ ! -x "$GAMEDIR/$BINNAME" ]; then
  BINNAME="localsend-retro.a53"
fi
if [ ! -x "$GAMEDIR/$BINNAME" ]; then
  echo "ERROR: no runnable localsend-retro binary found in $GAMEDIR" >&2
  exit 1
fi
BIN="$GAMEDIR/$BINNAME"

cd "$GAMEDIR"

> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1

echo "localsend-retro: CPU part ${CPU_PART:-unknown}, selected $BINNAME"

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
