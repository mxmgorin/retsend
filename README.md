<p align="center">
  <img src="resources/retsend-banner.png" alt="retsend: a LocalSend client for retro handhelds" width="640">
</p>

<div align="center">
  <a href="https://github.com/mxmgorin/retsend/releases/latest"><img src="https://img.shields.io/github/v/release/mxmgorin/retsend?style=flat-square&labelColor=16171a&color=3fb8a0&cacheSeconds=180&label=release" alt="Latest release"></a>
  <!-- The check workflow, not a platform build: it is the one a push and a pull request run, and it carries the tests and clippy. -->
  <a href="https://github.com/mxmgorin/retsend/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/mxmgorin/retsend/ci.yml?branch=main&style=flat-square&labelColor=16171a&color=3fb8a0&cacheSeconds=180&logo=githubactions&logoColor=white&label=ci" alt="CI"></a>
</div>

<p align="center">
  <a href="#install">Install</a> &middot;
  <a href="docs/CONFIGURATION.md">Docs</a> &middot;
  <a href="https://github.com/mxmgorin/retsend/issues">Issues</a>
</p>

retsend is a [LocalSend](https://localsend.org) client for retro handhelds, written in Rust with [SDL2](https://www.libsdl.org/) and [egui](https://github.com/emilk/egui). It sends and receives files with your phone or PC over Wi-Fi, without a cable, SSH or pulling the SD card, and works with the official LocalSend apps. Built for the gamepad; keyboard, touch and mouse work too.

**[Install](#install)** on handhelds through PortMaster, muOS, OnionOS, Allium or spruceOS, on the PS Vita, and on Android or Linux.

## Screenshots

| Receive | Request | Save | Transfer |
|:---:|:---:|:---:|:---:|
| ![Waiting for a sender, showing the device name, address, Wi-Fi network and the folder files land in](resources/retsend_receive.png) | ![An incoming request naming the sender, its files and where they land, counting down to the automatic decline](resources/retsend_request.png) | ![Picking the folder an incoming transfer lands in, the request still counting down](resources/retsend_save.png) | ![A transfer in progress: overall bar with bytes, speed and ETA, and a percentage per file](resources/retsend_transfer.png) |

| Send | History | Settings | Save routes |
|:---:|:---:|:---:|:---:|
| ![Radar of a discovered device with its model and address](resources/retsend_send.png) | ![Sent and received transfers with file counts, sizes, times and the folders they used](resources/retsend_history.png) | ![Settings on the device: name, save folder, quick save, file handling and port](resources/retsend_settings.png) | ![Save routes: received files go to a folder by extension, over the console folders detected on the card](resources/retsend_routes.png) |

## Features

- **Discovery**<br>
  A live radar of nearby devices over LocalSend protocol v2.1, plus a device typed in by IP for networks that block multicast.

- **Receive**<br>
  Accept or decline with a countdown, speed and ETA, cancel from either side. X picks the folder for one transfer; quick save accepts everything.

- **Send**<br>
  A gamepad file browser with multi-select, whole folders and per-file progress. Pinned folders and files lead every listing.

- **Save routes**<br>
  Received ROMs land in the console folder they belong to, detected from the card, with per-extension overrides. See [Configuration](docs/CONFIGURATION.md#save-routes).

- **Folders**<br>
  A folder sent either way arrives as a folder.

- **History**<br>
  Every transfer is logged. A repeats a send, X deletes the row.

- **Encryption**<br>
  The protocol's HTTPS mode, on by default, compatible with the official apps' defaults.

- **Headless mode**<br>
  `retsend --receive` runs without a screen, for SSH and scripting. See [Command line](docs/CLI.md).

- **Small**<br>
  No async runtime, a minimal HTTP server, and the system's own SDL2 wherever it can drive the screen.

## Install

Download the package for your system. All builds are on
[Releases](https://github.com/mxmgorin/retsend/releases).

| OS | Package | Where it goes |
| --- | --- | --- |
| [PortMaster](https://portmaster.games/supported-devices.html) | [`retsend-portmaster.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-portmaster.zip) | ports folder, e.g. `/roms/ports/` |
| [muOS](https://muos.dev) | [`retsend-muos.muxapp`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-muos.muxapp) | `ARCHIVE/` on the SD card, then **Applications → Archive Manager** |
| [OnionOS](https://onionui.github.io/) | [`retsend-onionos.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-onionos.zip) | root of the SD card, lands in `App/Retsend/` |
| [Allium](https://github.com/goweiwen/Allium) | [`retsend-allium.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-allium.zip) | root of the SD card, lands in `Apps/Retsend.pak/` |
| [spruceOS](https://github.com/spruceUI/spruceOS) | [`retsend-spruceos-aarch64.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-spruceos-aarch64.zip), [`retsend-spruceos.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-spruceos.zip) (Miyoo Mini) | root of the SD card, lands in `App/Retsend/` |
| PS Vita ([HENkaku](https://henkaku.xyz/)) | [`retsend-vita.vpk`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-vita.vpk) | install with VitaShell |
| Android | [`retsend-android-arm64.apk`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-android-arm64.apk) | sideload it |
| Linux | [`retsend-linux-x86_64.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-linux-x86_64.zip), [`retsend-linux-aarch64.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-linux-aarch64.zip), [`retsend-linux-armhf.zip`](https://github.com/mxmgorin/retsend/releases/latest/download/retsend-linux-armhf.zip) | unpack and run |

The app shows up in the Apps (or Applications) menu. How to quit on each system is
in [Controls](docs/CONTROLS.md#quitting). The Miyoo Mini build needs the Mini Plus
or Mini Flip: the original Mini has no Wi-Fi.

On Android, grant **All files access** and received files land in `Download/`;
otherwise the app keeps to its own folder. See [android/README.md](android/README.md).

## Building

`cargo run` with SDL2 installed. See **[Building from source](docs/BUILDING.md)**
for handhelds, Android and the PS Vita.

## Configuration

`config.toml` is in the data dir (`SDL_GetPrefPath`, e.g.
`~/.local/share/mxmgorin/retsend/` on Linux) and is written with the defaults on
first run. See **[Configuration](docs/CONFIGURATION.md)** for the options, save
routes and folders, **[Controls](docs/CONTROLS.md)** for buttons and touch, and
**[Command line](docs/CLI.md)** for arguments and environment variables.

## How to help

If you find the project useful, here is how you can help:

- **Tell other people about it.** Sharing the project helps it reach more users.
- **Report bugs and request features** in [Issues](https://github.com/mxmgorin/retsend/issues). Feedback is welcome.
- **Star the repo.** It helps the project get noticed and keeps me motivated.
- **[Buy me a coffee](https://ko-fi.com/mxmgorin)** on Ko-fi.

## Credits

retsend is built on:

- the [LocalSend](https://localsend.org) protocol, so it works with the official apps
- [SDL2](https://libsdl.org) through [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2)
  for the window and input
- [egui](https://github.com/emilk/egui) for the UI
- [rustls](https://github.com/rustls/rustls) over [ring](https://github.com/briansmith/ring)
  for TLS, [rcgen](https://github.com/rustls/rcgen) for certificates and
  [ureq](https://github.com/algesten/ureq) as the HTTP client
- [Steward Fu's SDL2](https://github.com/steward-fu/sdl2) with the Miyoo Mini display driver
- [VitaSDK](https://vitasdk.org) and [vita-rust](https://github.com/vita-rust) for the PS Vita build
- [PortMaster](https://portmaster.games) for running on Linux handhelds
