# Android port

The same app: the Rust code builds as a cdylib (`libretsend.so`) that SDL's
`SDLActivity` loads and enters through `SDL_main` in `src/lib.rs`. Android-only
code sits behind `#[cfg(target_os = "android")]`.

One APK per ABI, API 24 (Android 7) and up: `arm64-v8a` and `armeabi-v7a`.

## Install

Sideload `retsend-android-arm64.apk` from
[Releases](https://github.com/mxmgorin/retsend/releases), or
`retsend-android-armv7.apk` on a 32-bit device.

| All files access | Received files | File browser |
| --- | --- | --- |
| Granted | `Download/` | the whole storage |
| Denied | `Android/data/com.retsend/files/` (hidden from file managers on Android 11+) | that folder |

The prompt shows once; later it is **Settings → Apps → retsend → All files
access**, then **Settings → Save folder** in the app.

## Building

One-time setup:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-ndk --locked
# Android Studio (SDK + NDK + CMake), or:
sdkmanager --install "ndk;27.2.12479018" "platforms;android-34" "build-tools;34.0.0"
```

Build (`libSDL2.so` on the first run, then the cdylib and the APK):

```sh
./android/scripts/build.sh                       # debug   -> app/build/outputs/apk/debug/app-debug.apk
./android/scripts/build.sh release               # release -> app/build/outputs/apk/release/app-release.apk
./android/scripts/build.sh release armeabi-v7a   # 32-bit, same path
adb install -r android/app/build/outputs/apk/release/app-release.apk
```

- SDK and the newest NDK are auto-detected; override with `ANDROID_SDK_ROOT` /
  `ANDROID_NDK_HOME`.
- Debug and release sign with `app/debug.keystore`, so `-r` updates in place.
- Android Studio: run `build.sh` once, then open `android/`. Gradle only packages
  the `.so` files in `app/src/main/jniLibs/`; rebuild them after Rust changes.
- Logs: `adb logcat -s retsend`.

## How it fits

- **Entry:** `RetsendActivity.getLibraries()` returns `{"SDL2", "retsend"}`; SDL
  calls `SDL_main`, which runs the desktop's `run_app()`.
- **Paths:** `RetsendActivity.onCreate` sets the launchers' `RETSEND_*` env vars
  before SDL starts: `RETSEND_DATA_DIR` (`getFilesDir()`), `RETSEND_SAVE_DIR`,
  `RETSEND_BROWSER_ROOTS` (reachable storage volumes), `RETSEND_ALIAS`
  (`Build.MODEL`), `RETSEND_PANIC_FILE`.
- **Permissions:** `RetsendLauncherActivity` asks for all-files access before SDL
  starts, since the paths are read once at startup.
- **Discovery:** the activity holds a `WifiManager.MulticastLock`; without it the
  driver drops multicast announces.
- **Input:** touch or gamepad. SDL's touch-to-mouse is off: egui-sdl2 builds the
  pointer from fingers. A footer hint tap presses the button it names.
- **Back:** `SDL_ANDROID_TRAP_BACK_BUTTON` turns it into the Back command; on the
  home screen it quits.
- **Orientation:** locked to `sensorLandscape`, the handhelds' 640x480 layout.

## SDL version

`sdl2-sys 0.38` vendors SDL 2.26.4. `scripts/sync-sdl.sh` copies the
`org.libsdl.app` Java glue and builds `libSDL2.so` from that source, so glue,
library and bindings match. Its outputs (glue, wrapper jar, `jniLibs/`,
`res/mipmap-*`) are git-ignored.

## Not done yet

- **GL surface loss on background:** egui's textures may go stale; transfers are
  unaffected. Untested on a device.
- **Share target:** no `ACTION_SEND` intent filter; files are picked in the app.
- **Storage Access Framework:** all-files access is used instead.
- **Portrait:** needs a per-frame window-size sync `EguiWindow` lacks.

## Signing

CI decodes the `RETSEND_KEYSTORE_BASE64` secret to `app/release.keystore`, so
every release APK has the same signature; without it (forks) a throwaway key is
used. One-time setup:

```sh
keytool -genkeypair -keystore release.keystore -storepass android -keypass android \
  -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=retsend,O=retsend,C=US"
base64 -w0 release.keystore   # save as the repo secret RETSEND_KEYSTORE_BASE64
```

Other passwords or alias: `RETSEND_KEYSTORE_PASS`, `RETSEND_KEY_ALIAS`,
`RETSEND_KEY_PASS`.
