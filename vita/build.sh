#!/bin/sh
# Builds the PS Vita package, target/armv7-sony-vita-newlibeabihf/vita/retsend.vpk.
#
# Needs VitaSDK (with its sdl2 package) in $VITASDK, cargo-vita, and a nightly
# toolchain with rust-src, `nightly` unless $VITA_TOOLCHAIN names one: the
# target is tier 3, so std is built from source.
#
# ring 0.17.14 compiles its ARM assembly only for the OSes it lists, and the
# Vita joined that list after the release (briansmith/ring#2749). Until a
# release carries it, a patched copy of the registry crate stands in, wired up
# by vita/.cargo/config.toml, which cargo reads only when run from vita/.
# TODO: drop the copy once a ring release names the Vita among its asm targets.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
: "${VITASDK:?VITASDK must point at the VitaSDK install}"
export PATH="$VITASDK/bin:$PATH"
toolchain=${VITA_TOOLCHAIN:-nightly}

ring_src=$(cargo "+$toolchain" metadata --format-version 1 --manifest-path "$root/Cargo.toml" |
  python3 -c 'import json,sys; print(next(p["manifest_path"] for p in json.load(sys.stdin)["packages"] if p["name"] == "ring").rsplit("/", 1)[0])')
ring_dst="$root/target/vita-ring"
rm -rf "$ring_dst"
mkdir -p "$ring_dst"
cp -R "$ring_src/." "$ring_dst/"
sed -i 's/oss: &\["horizon"\],/oss: \&["horizon", "vita"],/' "$ring_dst/build.rs"
grep -q '"horizon", "vita"' "$ring_dst/build.rs" || {
  echo "ring's build.rs changed shape; update the patch in $0" >&2
  exit 1
}

# The patch rewrites ring's lock entry as a path dependency; the committed
# lockfile stays the crates.io one.
cp "$root/Cargo.lock" "$root/target/Cargo.lock.vita-orig"
trap 'mv "$root/target/Cargo.lock.vita-orig" "$root/Cargo.lock"' EXIT

cd "$root/vita"
cargo "+$toolchain" vita build vpk --profile vita "$@"
