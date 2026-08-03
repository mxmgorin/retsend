#!/usr/bin/env bash
# Cross-build an ARM binary the way CI does, and print its path.
#
#   tools/arm/build.sh armhf|aarch64 [cargo args...]
#
# Caches live outside the repo so container-root files never mix with host
# builds. The repo's parent is mounted, so a sibling path dependency resolves.
set -euo pipefail

arch=${1:?usage: build.sh armhf|aarch64 [cargo args...]}
shift || true

case "$arch" in
armhf)
  target=armv7-unknown-linux-gnueabihf
  libdir=/opt/sdl2/armhf/usr/lib/arm-linux-gnueabihf
  ;;
aarch64)
  target=aarch64-unknown-linux-gnu
  libdir=/opt/sdl2/arm64/usr/lib/aarch64-linux-gnu
  ;;
*)
  echo "unknown arch: $arch (expected armhf or aarch64)" >&2
  exit 2
  ;;
esac

image=retsend-arm-cross
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
cache=${RETSEND_ARM_CACHE:-$HOME/.cache/retsend-arm}
# The glibc floor, from the oldest userland we target (OnionOS, ArkOS).
floor=$(cat "$here/glibc-floor")

mkdir -p "$cache/target" "$cache/cargo" "$cache/zig"
# Cached after the first run.
docker build -q -t "$image" -f "$here/Dockerfile" "$here" >/dev/null

docker run --rm --network host \
  -v "$(dirname "$repo")":/repos \
  -v "$cache/target":/target \
  -v "$cache/cargo":/cargo \
  -v "$cache/zig":/zig-cache \
  -e CARGO_TARGET_DIR=/target -e CARGO_HOME=/cargo -e RUSTUP_HOME=/cargo \
  -e ZIG_GLOBAL_CACHE_DIR=/zig-cache \
  -e "REPO_NAME=$(basename "$repo")" \
  -e "TARGET=$target" -e "LIBDIR=$libdir" -e "FLOOR=$floor" \
  -e "HOST_UID=$(id -u)" -e "HOST_GID=$(id -g)" \
  -e "CARGO_ARGS=$*" \
  "$image" bash -euxc '
    export PATH="/cargo/bin:$PATH"
    command -v cargo >/dev/null || curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --default-toolchain none
    cd "/repos/$REPO_NAME"
    rustup target add "$TARGET"
    # No linker override: cargo-zigbuild points the linker and cc at zig itself,
    # and a -C linker of our own would opt out of it.
    export RUSTFLAGS="-L $LIBDIR -C link-arg=-Wl,--allow-shlib-undefined"
    export CARGO_PROFILE_RELEASE_LTO=fat
    export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
    cargo zigbuild --release --target "$TARGET.$FLOOR" $CARGO_ARGS
    out="/target/$TARGET/release/retsend"
    readelf -V "$out" | grep -o "GLIBC_2\.[0-9]*" | sort -uV | tr "\n" " "
    echo
    # Above the floor the loader on the device refuses it, so fail here instead.
    major_floor=${FLOOR#2.}
    newer=$(readelf -V "$out" | grep -o "GLIBC_2\.[0-9]*" | sort -uV \
      | awk -F. -v f="$major_floor" "\$2 > f" | tr "\n" " ")
    [ -z "$newer" ] || { echo "binary requires $newer; the floor is GLIBC_$FLOOR" >&2; exit 1; }
    chown -R "$HOST_UID:$HOST_GID" /target /cargo /zig-cache
  '

echo "$cache/target/$target/release/retsend"
