#!/bin/sh
# NextUI / NX Redux (TrimUI) launcher. No PortMaster here, so the pad mapping
# is done here.
gamedir=$(cd "$(dirname "$0")" && pwd)
card=$(cd "$gamedir/../../.." && pwd)
cd "$gamedir" || exit 1

: > "$gamedir/log.txt"

# The menu leaves the clusters capped and each pak raises its own.
for policy in /sys/devices/system/cpu/cpufreq/policy*; do
  cat "$policy/cpuinfo_max_freq" > "$policy/scaling_max_freq" 2>/dev/null
done

# trimui_inputd's pad reports Xbox codes under Nintendo labels; map by label.
export SDL_GAMECONTROLLERCONFIG="030000005e0400008e02000014010000,TRIMUI Player1,a:b1,b:b0,back:b6,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b8,leftshoulder:b4,leftstick:b9,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b10,righttrigger:a5,rightx:a3,righty:a4,start:b7,x:b3,y:b2,platform:Linux,"

export HOME="$gamedir"
export XDG_DATA_HOME="$gamedir"
export RETSEND_DATA_DIR="$gamedir/data"
export RETSEND_SAVE_DIR="$card/Roms"
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
# NextUI expects a pak to end itself.
export RETSEND_BACK_QUIT=1
#export RETSEND_LOG_LEVEL=debug

exec "$gamedir/retsend" "$@" >> "$gamedir/log.txt" 2>&1
