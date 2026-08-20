#!/usr/bin/env bash
# One-command Android build: cross-compiles the Rust cdylib and assembles the APK.
#
#   ./android/scripts/build.sh            # debug APK (faster)
#   ./android/scripts/build.sh release    # release APK
#
# Prereqs (one-time):
#   rustup target add aarch64-linux-android
#   cargo install cargo-ndk --locked
#   Android SDK + an NDK + CMake (Android Studio installs these).
#
# Auto-detects the SDK and the newest installed NDK; override with
# ANDROID_SDK_ROOT / ANDROID_NDK_HOME.
set -euo pipefail

profile="${1:-debug}"
abi="arm64-v8a"
api="24"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # android/
repo="$(cd "$here/.." && pwd)"

# --- locate SDK + NDK ---------------------------------------------------------
sdk="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Android/Sdk}}"
[ -d "$sdk" ] || { echo "Android SDK not found (set ANDROID_SDK_ROOT)"; exit 1; }
export ANDROID_SDK_ROOT="$sdk"

ndk="${ANDROID_NDK_HOME:-${ANDROID_NDK:-}}"
if [ -z "$ndk" ]; then
    ndk="$(ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -1 || true)"
fi
[ -n "$ndk" ] && [ -d "$ndk" ] || { echo "NDK not found (set ANDROID_NDK_HOME)"; exit 1; }
export ANDROID_NDK_HOME="$ndk"
echo "SDK: $sdk"
echo "NDK: $ndk"

# --- toolchain checks ---------------------------------------------------------
rustup target list --installed | grep -q aarch64-linux-android \
    || { echo "run: rustup target add aarch64-linux-android"; exit 1; }
command -v cargo-ndk >/dev/null \
    || { echo "run: cargo install cargo-ndk --locked"; exit 1; }

# --- SDL libs + glue (only if missing) ----------------------------------------
if [ ! -f "$here/app/src/main/jniLibs/$abi/libSDL2.so" ]; then
    echo "==> building libSDL2.so + syncing SDL glue"
    bash "$here/scripts/sync-sdl.sh"
fi

# --- link workaround ----------------------------------------------------------
# NDK r23+ dropped libgcc, but rustc's aarch64-linux-android target spec still
# emits `-lgcc`. Redirect it to libunwind; the `-L` for this stub and for the
# bundled libSDL2.so live in .cargo/config.toml.
mkdir -p "$repo/target/ndk-libgcc-stub"
echo "INPUT(-lunwind)" > "$repo/target/ndk-libgcc-stub/libgcc.a"

# --- build cdylib -------------------------------------------------------------
echo "==> cross-compiling libretsend.so ($profile)"
cd "$repo"
ndk_flags=(-t "$abi" -P "$api" -o android/app/src/main/jniLibs build)
[ "$profile" = "release" ] && ndk_flags+=(--release)
cargo ndk "${ndk_flags[@]}"

# --- assemble APK -------------------------------------------------------------
echo "==> assembling APK ($profile)"
cd "$here"
# Both build types sign with this one keystore (see app/build.gradle) so
# reinstalls update in place instead of failing on a signature mismatch.
[ -f app/debug.keystore ] || keytool -genkeypair -v -keystore app/debug.keystore \
    -storepass android -keypass android -alias androiddebugkey \
    -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=retsend,O=retsend,C=US"
if [ "$profile" = "release" ]; then
    ./gradlew --no-daemon assembleRelease
    echo "APK: android/app/build/outputs/apk/release/app-release.apk"
else
    ./gradlew --no-daemon assembleDebug
    echo "APK: android/app/build/outputs/apk/debug/app-debug.apk"
fi
