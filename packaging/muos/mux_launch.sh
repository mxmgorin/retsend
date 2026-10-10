#!/bin/sh
# HELP: Send and receive files over wifi with LocalSend - no cable, no SSH
# ICON: retsend
# GRID: Retsend

. /opt/muos/script/var/func.sh

APP_BIN="retsend"
# frontend.sh passes the app folder, which is SD1 or SD2 depending on where the
# archive was installed. The fallback is for a run by hand over SSH.
APP_DIR="${1:-$(cd "$(dirname "$0")" && pwd)}"
LOG_FILE="$APP_DIR/log.txt"

# Jacaranda and newer: one call does the action file, the governor, HOME and the
# SDL controller database. Older muOS spells the same out.
if command -v SETUP_APP >/dev/null 2>&1; then
	SETUP_STAGE_OVERLAY
	SETUP_APP "$APP_BIN" ""
else
	echo app >/tmp/act_go
	SETUP_SDL_ENVIRONMENT
	SET_VAR "system" "foreground_process" "$APP_BIN"
fi

# RK3576 (Vita Pro and kin) ships an SDL that finds no EGL driver on its own.
if grep -q "rk3576" /proc/device-tree/compatible 2>/dev/null; then
	for MALI_LIBRARY in /usr/lib/libmali.so /usr/lib/aarch64-linux-gnu/libmali.so; do
		if [ -e "$MALI_LIBRARY" ]; then
			export SDL_VIDEO_EGL_DRIVER="$MALI_LIBRARY"
			break
		fi
	done
	export SDL_OPENGL_ES_DRIVER=1
fi

cd "$APP_DIR" || exit 1

# SETUP_APP points HOME at the board's, which is rootfs; keep writable paths on
# the card next to the app.
export HOME="$APP_DIR"
export XDG_DATA_HOME="$APP_DIR"
export RETSEND_DATA_DIR="$APP_DIR/data"
export RETSEND_PANIC_FILE="$APP_DIR/retsend-panic.log"
ROM_MOUNT="$(GET_VAR "device" "storage/rom/mount")"
export RETSEND_SAVE_DIR="$ROM_MOUNT/ROMS"
# muOS expects an app to end itself — its own way out is the R2+SELECT+B kill —
# so B on the home screen is ours, and the sockets close on the way.
export RETSEND_BACK_QUIT=1
#export RETSEND_LOG_LEVEL=debug

: >"$LOG_FILE"

# An empty radar reads as a broken app rather than an answer, so say which it is.
NET_STATE="$(GET_VAR "device" "network/state")"
if [ -r "$NET_STATE" ] && [ "$(cat "$NET_STATE")" != "up" ]; then
	echo "wifi is down; connect it in muOS settings or nothing will be found" >>"$LOG_FILE"
fi

# A transfer is minutes with no button pressed, which is the device's cue to
# sleep. Absent before Jacaranda, hence the guard.
HAS_CAFFEINE=0
command -v CAFFEINE >/dev/null 2>&1 && HAS_CAFFEINE=1

[ "$HAS_CAFFEINE" -eq 1 ] && CAFFEINE on
./"$APP_BIN" >>"$LOG_FILE" 2>&1
[ "$HAS_CAFFEINE" -eq 1 ] && CAFFEINE off

exit 0
