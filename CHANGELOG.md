# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Devices can be added to the radar by IP: X on the Send tab types `ip[:port]`
  (port defaults to 53317) and registers with whatever answers there — the way
  in on networks where multicast discovery never arrives, like guest Wi-Fi and
  AP-isolated access points. HTTPS is tried before plain HTTP, `/info` before
  giving up, and a hand-added device stays on the radar instead of expiring
  after two minutes of silence. It still has to be routable: this replaces
  discovery, not the network path.

## [0.2.1] - 2026-07-30

### Fixed

- PortMaster: received files default to the ROMs folder on muOS too. The launcher
  took PortMaster's `$directory` for the ROMs root, which on muOS is the card root
  (`/mnt/mmc`) one level above `ROMS`. Installs that already have a config keep
  their folder — change it in Settings.

## [0.2.0] - 2026-07-30

### Changed

- Every footer now ends with A, so the confirm hint sits under the same thumb on
  all screens: the Send tab and the on-screen keyboard used to lead with it.
- The on-screen keyboard erases with X; B now just leaves it, instead of erasing
  and then closing once the buffer ran empty.
- The routes editor scrolls through the auto save routes: the cursor carries on
  past the add row into them, so a list longer than the screen no longer hides
  its last rows. They stay read-only — A does nothing there.

### Added

- History rows show the folder the files landed in (`→`) or were sent from (`←`),
  so a save route that moved a ROM somewhere unexpected is visible without
  hunting for it. A session split across routes shows the first folder and how
  many more it used. Entries logged before this stay two-line.
- The incoming-request modal names the routed destinations instead of the plain
  save directory, so the folders the files will actually land in are visible
  before pressing A — one line per folder when the save routes spread a request
  over several, up to four before the rest collapse into a count.
- X in the send browser selects every file of the current folder (and clears them
  again), so a folder full of saves goes out without walking the list. Subfolders
  stay out: the protocol only carries a flat list of files.
- Pinned paths: Y in the file browser pins the folder or file under the cursor,
  and pinned rows lead every listing — a pinned folder is one press away from
  anywhere in the tree, a pinned file can be sent without navigating
  (`[transfer] pinned_paths`). The send browser also reopens where the last send
  came from (`[transfer] last_send_dir`).
- Automatic save routes: received ROMs are sorted into the console folders that
  already exist in the save directory, using the folder names KNULLI, ROCKNIX,
  and muOS each actually use. Nothing is created, and only extensions that name
  exactly one console take part. Toggled from Settings → Auto save routes
  (`[transfer] auto_routes`, on by default); explicit `[transfer.routes]` entries
  still win.

## [0.1.0] - 2026-07-28

Initial release. A Rust LocalSend client built for handheld and gamepad-first
use (Knulli, muOS, ROCKNIX), running on desktop Linux too.

### Added

- LocalSend protocol v2.1: multicast announce and `/register` discovery, shown
  as a live radar of nearby devices.
- Receive flow: accept/decline dialog with countdown, progress with speed and
  ETA, and cancel from either side; quick-save mode auto-accepts.
- Send flow: gamepad file browser with multi-select and switchable roots, staged
  files, a waiting phase, and per-file progress.
- Encryption: the protocol's HTTPS mode, on by default — a self-signed identity
  generated once and served via rustls, announced as the certificate's SHA-256
  fingerprint. Works with the official app's defaults both ways.
- Save routes: received files sorted into folders by extension, editable on the
  device from Settings → Save routes.
- Settings on device: alias, save folder, and port, applied live — the network
  layer restarts on a port change.
- History tab: persisted transfer log with a configurable limit.
- On-screen keyboard (3-layer grid) for text entry without a keyboard.
- Tabbed navigation (Send, Receive, History, Settings) and an about overlay.
- Headless `--receive` mode for SSH and scripting.
- Streaming writes with sanitized names, one transfer at a time, and a sweep of
  stale `.part` files at startup.
- Brand wordmark and window icon.
- Builds for Linux x86_64 and aarch64, with PortMaster packaging.

[Unreleased]: https://github.com/mxmgorin/retsend/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/mxmgorin/retsend/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/mxmgorin/retsend/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mxmgorin/retsend/releases/tag/v0.1.0
