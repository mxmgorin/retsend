# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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

[Unreleased]: https://github.com/mxmgorin/retsend/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mxmgorin/retsend/releases/tag/v0.1.0
