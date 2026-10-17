#!/bin/sh
# Hands the port to spruce's PORTS launcher, which sets up PortMaster and
# takes the system from the Emu/PORTS/ segment of the script path.
appdir=$(cd "$(dirname "$0")" && pwd)
export RETSEND_GAMEDIR="$appdir/retsend/"
export RETSEND_THEME=spruce
# The default device name: the model spruce runs on.
case "$PLATFORM" in
A30) model="Miyoo A30" ;;
Flip) model="Miyoo Flip" ;;
Brick) model="TrimUI Brick" ;;
BrickPro) model="TrimUI Brick Pro" ;;
SmartPro) model="TrimUI Smart Pro" ;;
SmartProS) model="TrimUI Smart Pro S" ;;
Pixel2) model="GKD Pixel 2" ;;
RGB30) model="Powkiddy RGB30" ;;
RGB20SX) model="Powkiddy RGB20SX" ;;
Miniloong) model="Miniloong Pocket 1" ;;
A10Mini) model="A10 Mini" ;;
XU20) model="XU20" ;;
Zero28) model="MagicX Mini Zero 28" ;;
Zero40) model="MagicX Zero 40" ;;
*) model="$PLATFORM" ;;
esac
[ -n "$model" ] && export RETSEND_ALIAS="$model"
exec /mnt/SDCARD/Emu/PORTS/../../spruce/scripts/emu/standard_launch.sh "$appdir/Retsend.sh"
