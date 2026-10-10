# Building from source

retsend builds with plain `cargo`; rustup installs the Rust version pinned in
[`rust-toolchain.toml`](../rust-toolchain.toml).

## Linux

SDL2 is the only native dependency:

```sh
sudo apt-get install -y build-essential pkg-config libsdl2-dev
cargo run
```

Tests are headless, with no SDL or network setup:

```sh
cargo test
```

## Handhelds (ARM Linux)

Cross-builds run in Docker, the way CI does them:

```sh
tools/arm/build.sh armhf     # Miyoo Mini
tools/arm/build.sh aarch64   # PortMaster, muOS, spruceOS aarch64
```

The packages are assembled in
[`build-linux-arm.yml`](../.github/workflows/build-linux-arm.yml).

## Android

With the Android SDK and an NDK:

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
./android/scripts/build.sh release   # android/app/build/outputs/apk/release/app-release.apk
```

See [android/README.md](../android/README.md).

## PS Vita

With [VitaSDK](https://vitasdk.org) and its `sdl2` package, a nightly toolchain
with `rust-src`, and `cargo install cargo-vita`:

```sh
VITASDK=/usr/local/vitasdk packaging/vita/build.sh   # target/armv7-sony-vita-newlibeabihf/vita/retsend.vpk
```

The script patches a copy of ring until a release builds its assembly for the
Vita; see its header.
