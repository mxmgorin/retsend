#!/usr/bin/env bash
# One-command Android build: cross-compiles the Rust cdylib and assembles the APK.
#
#   ./android/scripts/build.sh                        # debug APK, arm64-v8a
#   ./android/scripts/build.sh release                # release APK
#   ./android/scripts/build.sh release armeabi-v7a    # 32-bit ARM
#
# Prereqs (one-time):
#   rustup target add aarch64-linux-android armv7-linux-androideabi
#   cargo install cargo-ndk --locked
#   Android SDK + an NDK + CMake (Android Studio installs these).
#
# Auto-detects the SDK and the newest installed NDK; override with
# ANDROID_SDK_ROOT / ANDROID_NDK_HOME.
set -euo pipefail

profile="${1:-debug}"
abi="${2:-arm64-v8a}"
api="24"
case "$abi" in
    arm64-v8a) rust_target=aarch64-linux-android ;;
    armeabi-v7a) rust_target=armv7-linux-androideabi ;;
    *) echo "unsupported ABI: $abi"; exit 1 ;;
esac
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
rustup target list --installed | grep -q "$rust_target" \
    || { echo "run: rustup target add $rust_target"; exit 1; }
command -v cargo-ndk >/dev/null \
    || { echo "run: cargo install cargo-ndk --locked"; exit 1; }

# --- SDL libs + glue (only if missing) ----------------------------------------
if [ ! -f "$here/app/src/main/jniLibs/$abi/libSDL2.so" ]; then
    echo "==> building libSDL2.so + syncing SDL glue"
    bash "$here/scripts/sync-sdl.sh" "$abi"
fi

# --- link workaround ----------------------------------------------------------
# NDK r23+ dropped libgcc, but rustc's Android target specs still emit `-lgcc`. Redirect it to libunwind; the `-L` for this stub and for the
# bundled libSDL2.so live in .cargo/config.toml.
mkdir -p "$repo/target/ndk-libgcc-stub"
echo "INPUT(-lunwind)" > "$repo/target/ndk-libgcc-stub/libgcc.a"

# --- build cdylib -------------------------------------------------------------
echo "==> cross-compiling libretsend.so ($profile, $abi)"
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
    ./gradlew --no-daemon -Pabi="$abi" assembleRelease
    echo "APK: android/app/build/outputs/apk/release/app-release.apk"
else
    ./gradlew --no-daemon -Pabi="$abi" assembleDebug
    echo "APK: android/app/build/outputs/apk/debug/app-debug.apk"
fi
