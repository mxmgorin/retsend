#!/bin/sh
# Hands the port to spruce's PORTS launcher, which sets up PortMaster and
# takes the system from the Emu/PORTS/ segment of the script path.
appdir=$(cd "$(dirname "$0")" && pwd)
export RETSEND_GAMEDIR="$appdir/retsend/"
export RETSEND_THEME=spruce
exec /mnt/SDCARD/Emu/PORTS/../../spruce/scripts/emu/standard_launch.sh "$appdir/Retsend.sh"
