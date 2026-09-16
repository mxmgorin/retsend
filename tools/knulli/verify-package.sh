#!/bin/sh
# Release build tests for the Knulli package (facts.md, "Knulli release build
# tests"): launcher shell syntax, archive contents, ZIP safety, executable
# mode, ELF architecture, dynamic requirements, and required licence/source
# files.
#
# Usage: tools/knulli/verify-package.sh <package-dir> [aarch64|armhf]
# The package dir is the assembled tree (RetSend.sh, retsend, LICENSE,
# NOTICES, README.md). Exits non-zero on the first failed check.

set -eu

if [ $# -lt 1 ]; then
  echo "usage: $0 <package-dir> [aarch64|armhf]" >&2
  exit 2
fi
pkg=$1
arch=${2:-aarch64}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[ -d "$pkg" ] || fail "package dir $pkg is not a directory"

case "$arch" in
  aarch64) machine="AArch64" ;;
  armhf) machine="ARM" ;;
  *) fail "unsupported arch: $arch" ;;
esac

# --- required files ---------------------------------------------------------
for f in RetSend.sh retsend LICENSE NOTICES README.md; do
  [ -f "$pkg/$f" ] || fail "missing required file: $f"
done

# --- launcher shell syntax --------------------------------------------------
sh -n "$pkg/RetSend.sh" || fail "RetSend.sh fails shell syntax check"
# The launcher must not reference PortMaster — in code, that is; the header
# comment explaining that it doesn't is fine.
code=$(grep -v '^[[:space:]]*#' "$pkg/RetSend.sh")
if printf '%s\n' "$code" | grep -qi 'portmaster\|GPTOKEYB\|pm_platform_helper\|pm_finish'; then
  fail "RetSend.sh must not reference PortMaster"
fi

# --- executable mode --------------------------------------------------------
# CI moves artifacts through zips, which drop modes; catch a chmod that was.
[ -x "$pkg/retsend" ] || fail "retsend is not executable"
[ -x "$pkg/RetSend.sh" ] || fail "RetSend.sh is not executable"

# --- ELF architecture -------------------------------------------------------
if command -v readelf >/dev/null 2>&1; then
  readelf -h "$pkg/retsend" | grep -q "Machine:.*$machine" ||
    fail "retsend is not a $arch ELF (expected Machine: $machine)"

  # --- dynamic requirements -------------------------------------------------
  # Only the base system libs; anything else (X11, wayland, …) means the
  # binary linked against the wrong SDL2.
  deps=$(readelf -d "$pkg/retsend" | grep NEEDED | grep -o '\[[^]]*\]')
  echo "dynamic deps: $deps"
  for lib in libSDL2-2.0.so.0 libm.so.6 libpthread.so.0 libc.so.6 libdl.so.2; do
    echo "$deps" | grep -qF "[$lib]" || fail "missing NEEDED entry: $lib"
  done
  echo "$deps" | grep -v 'libSDL2-2.0.so.0\|libm.so.6\|libpthread.so.0\|libc.so.6\|libdl.so.2' &&
    fail "unexpected extra dynamic dependencies (see above)"

  # Interpreter sanity.
  readelf -l "$pkg/retsend" | grep -q 'ld-linux' || fail "no ELF interpreter found"
else
  echo "readelf not available; skipping ELF checks" >&2
fi

# --- licence / source files -------------------------------------------------
head -3 "$pkg/LICENSE" | grep -q "GNU GENERAL PUBLIC LICENSE" ||
  fail "LICENSE does not look like the GPL text"
grep -q "mxmgorin/retsend" "$pkg/NOTICES" || fail "NOTICES missing source URL"
grep -q "SDL" "$pkg/NOTICES" || fail "NOTICES missing the SDL2 notice"
grep -qiE "source|github.com/mxmgorin" "$pkg/README.md" || fail "README missing source reference"

# --- ZIP safety of the assembled tree ---------------------------------------
# No absolute paths, no traversal, no symlinks inside the package.
find "$pkg" -type l | grep -q . && fail "package contains symlinks"
find "$pkg" -name '.*' ! -name '.' | grep -q . && fail "package contains hidden entries"

echo "OK: $pkg ($arch)"
