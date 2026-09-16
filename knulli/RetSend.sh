#!/bin/sh
# Knulli (Batocera fork) launcher. Unlike the PortMaster launcher, this sources
# no PortMaster control files and invokes none of its helpers.
#
# Layout on the card (the App Store installs it here):
#   /userdata/roms/tools/RetSend/RetSend.sh   this launcher
#   /userdata/roms/tools/RetSend/retsend      the AArch64 binary
#
# Constrained paths (see goals/knulli-build/facts.md):
#   - Config, history and TLS identity live in /userdata/system/configs/retsend,
#     on the persistent partition, outside the package the installer may replace.
#   - Received files land in /userdata/roms/retsend-inbox, never the ROMs root.
#   - The file browser only roots at the ROMs tree and the inbox.

gamedir=$(cd "$(dirname "$0")" && pwd)
cd "$gamedir" || exit 1

datadir=/userdata/system/configs/retsend
inbox=/userdata/roms/retsend-inbox
mkdir -p "$datadir" "$inbox" || exit 1

# Writable HOME on the persistent card: without it SDL's pref path falls back
# into the read-only rootfs. It sits inside /userdata/roms, so the browser's
# home-root rule (roots are where B cannot leave) adds nothing beyond the two
# reviewed roots below.
export HOME="$gamedir"

export RETSEND_DATA_DIR="$datadir"
export RETSEND_SAVE_DIR="$inbox"
export RETSEND_BROWSER_ROOTS="/userdata/roms:$inbox"
export RETSEND_PANIC_FILE="$gamedir/retsend-panic.log"
#export RETSEND_LOG_LEVEL=debug

[ -x "$gamedir/retsend" ] || {
  echo "retsend: not found or not executable in $gamedir" >&2
  exit 1
}

exec ./retsend
