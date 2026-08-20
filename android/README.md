# Android port

retsend on Android is the same app: SDL2 has a mature Android port, so the Rust
code ships as a cdylib (`libretsend.so`) that SDL's Java `SDLActivity` loads and
enters through the C `SDL_main` in `src/lib.rs`. Windowing, the GLES context,
gamepad input and the whole net stack carry over. What is Android's alone is
packaging, storage paths, touch, and the Back button — all of it behind
`#[cfg(target_os = "android")]` or additive Cargo entries, so the Linux and
handheld builds are untouched.

Targets `arm64-v8a` only, which is every Android handheld and phone worth
running this on, from API 24 (Android 7).

## Install

Grab `retsend-android-arm64.apk` from
[Releases](https://github.com/mxmgorin/retsend/releases) and sideload it.

On first launch it asks for **All files access**. Grant it and received files
land in `Download/`, and the file browser can reach the whole card — which is
the point on a device where an emulator has to find what you sent. Denied, the
app still works, but it is confined to its own external directory
(`Android/data/com.retsend/files/`), which file managers can't open on Android
11+. The grant screen is only offered once; afterwards it's
**Settings → Apps → retsend → All files access**, and the save folder is then
changed in-app under **Settings → Save folder**.

## Building

One-time setup:

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
# Android Studio (SDK + an NDK + CMake), or the sdkmanager equivalents:
sdkmanager --install "ndk;27.2.12479018" "platforms;android-34" "build-tools;34.0.0"
```

Then one command builds `libSDL2.so` (first run only), cross-compiles the cdylib
and assembles the APK:

```sh
./android/scripts/build.sh           # debug   -> android/app/build/outputs/apk/debug/app-debug.apk
./android/scripts/build.sh release   # release -> android/app/build/outputs/apk/release/app-release.apk
adb install -r android/app/build/outputs/apk/release/app-release.apk
```

It auto-detects the SDK and the newest installed NDK (override with
`ANDROID_SDK_ROOT` / `ANDROID_NDK_HOME`). Debug and release sign with the same
`app/debug.keystore`, so `-r` updates in place.

In Android Studio: run `build.sh` once so the `.so` files exist under
`app/src/main/jniLibs/`, then open `android/` and use Run — Gradle only packages
what is already there, it never builds Rust. Re-run the `cargo ndk` step
whenever the Rust code changes.

Logs go to logcat: `adb logcat -s retsend`.

## How the pieces fit

- **Entry point** — `RetsendActivity.getLibraries()` returns `{"SDL2",
  "retsend"}`, so SDL loads `libretsend.so` and calls its `SDL_main`, which hands
  off to the same `run_app()` the desktop binary uses.
- **Paths** — `RetsendActivity.onCreate` sets the same `RETSEND_*` env vars the
  PortMaster and OnionOS launchers set, before SDL starts: `RETSEND_DATA_DIR`
  (internal `getFilesDir()`: config, TLS identity, history), `RETSEND_SAVE_DIR`,
  `RETSEND_BROWSER_ROOTS` (the storage volumes it can reach), `RETSEND_ALIAS`
  (`Build.MODEL`), `RETSEND_SCALE` (display density) and `RETSEND_PANIC_FILE`.
- **Permissions** — `RetsendLauncherActivity` is the launcher entry and asks for
  all-files access *before* starting SDL, because the paths above are read once
  at startup and the save folder is persisted on first run.
- **Discovery** — the activity holds a `WifiManager.MulticastLock` for its
  lifetime; without one the Wi-Fi driver filters out the multicast announces.
- **Input** — touch or a gamepad. SDL's own touch→mouse synthesis is off, since
  egui-sdl2 builds a pointer stream from the finger events itself. A tap becomes
  the same `AppCommand` a button would: on a row it places the cursor and
  confirms, and each footer hint slot *is* the button it names, which is what
  makes Start/Select/X/Y reachable without a pad.
- **Back** — `SDL_ANDROID_TRAP_BACK_BUTTON` makes it an `AC_BACK` key instead of
  backgrounding the activity, mapped to the Back command; at the top level it
  quits, since on Android Back has to lead somewhere.
- **Orientation** — locked to `sensorLandscape`. The UI is the handhelds'
  640x480 one, and a fixed aspect also avoids relayout on rotation, which
  egui-sdl2 drives off a cached window size that an Android rotation can outrun.

## SDL version coupling

`sdl2-sys 0.38` vendors SDL 2.26.4. `scripts/sync-sdl.sh` copies the
`org.libsdl.app` Java glue and builds `libSDL2.so` from that same source, so the
glue, the runtime `.so` and the Rust bindings all match. Everything it produces
(Java glue, wrapper jar, `jniLibs/`, `res/mipmap-*`) is git-ignored and
regenerated.

## Not done yet

- **GL surface loss on background.** Android destroys the EGL surface when the
  app leaves the foreground; SDL blocks the app thread across that, but a
  context loss would leave egui's textures stale. Transfers run on their own
  threads and are unaffected. Untested on a device.
- **Sending files the system hands us.** No `ACTION_SEND` intent filter yet, so
  retsend can't be a share target; files are picked in its own browser.
- **Save folder via SAF.** All-files access is used instead, which is what plain
  file I/O needs and what sideloading allows.
- **Portrait.** Would need a per-frame window-size sync that `EguiWindow` doesn't
  expose yet, hence the orientation lock.

## Signing

CI restores a stable key from the `RETSEND_KEYSTORE_BASE64` secret (decoded to
`app/release.keystore`, passed via `RETSEND_KEYSTORE`) so every release APK has
the same signature; without the secret it falls back to an ephemeral key so
forks still build. One-time setup:

```sh
keytool -genkeypair -keystore release.keystore -storepass android -keypass android \
  -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=retsend,O=retsend,C=US"
base64 -w0 release.keystore   # save as the repo secret RETSEND_KEYSTORE_BASE64
```

Non-default passwords or alias also need `RETSEND_KEYSTORE_PASS`,
`RETSEND_KEY_ALIAS` and `RETSEND_KEY_PASS`. Play distribution would need a real
upload key through the same env — and a different storage story, since
`MANAGE_EXTERNAL_STORAGE` is not something Play grants a file-transfer app
lightly.
