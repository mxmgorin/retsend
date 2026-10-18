#!/bin/sh
# Runs under the firmware's PORTS launcher, which sets up the CPU, libraries and input.
appdir=$(cd "$(dirname "$0")" && pwd)
export RETSEND_GAMEDIR="$appdir/retsend/"
# trimui_inputd's pad reports Xbox codes under Nintendo labels; map by label.
export RETSEND_GAMECONTROLLERCONFIG="030000005e0400008e02000014010000,TRIMUI Player1,a:b1,b:b0,back:b6,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b8,leftshoulder:b4,leftstick:b9,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b10,righttrigger:a5,rightx:a3,righty:a4,start:b7,x:b3,y:b2,platform:Linux,"
exec /mnt/SDCARD/Emus/PORTS/launch.sh "$appdir/Retsend.sh"
