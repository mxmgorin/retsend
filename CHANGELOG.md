# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Receiving from LocalSend 1.18+. It streams uploads through reqwest, which
  frames the body `Transfer-Encoding: chunked` instead of sending a
  Content-Length, and the server answered every chunked request with 411 — the
  sender saw `statusCode(status: 411)`. Chunked bodies are now decoded;
  extensions and trailers are dropped, and a Content-Length sent beside chunked
  framing is ignored rather than trusted. Transfer codings we can't decode moved
  from 411 to 501.

## [0.5.3] - 2026-08-13

### Fixed

- Sending to a receiver that requires client authentication. LocalSend's newer
  core makes a client certificate mandatory whenever it isn't also serving its
  web UI, and we presented none, so the handshake died before the first request
  and the transfer read **Send failed: certificate required**. Outbound requests
  now carry our persisted identity — the same certificate the announced
  fingerprint hashes. It is loaded even with `https = false`, since a peer
  announces its own scheme and we can be the client of an HTTPS receiver while
  serving plain http.

## [0.5.2] - 2026-08-08

### Fixed

- The settings list scrolls. Its nine rows are taller than a 640x480 screen has
  room for, and the list had no scroll area, so the last row (About) was cut off
  at the panel edge — the cursor still moved onto it, off-screen. A smaller panel
  or a larger `RETSEND_SCALE` cut it sooner.

## [0.5.1] - 2026-08-08

### Changed

- The tab bar leads with Receive, the tab the app opens on. Landing mid-bar meant
  left and right stepped off the landing screen in two unrelated directions.
- Button hints name what the button does. A on the radar reads **Choose files**,
  not "Send" — it opens the file browser, and Start is what sends. Select on
  Receive reads **Announce** rather than the radar's "Refresh": there is nothing
  to refresh there, the button re-multicasts our announce. X on an incoming
  request reads **Save to…** rather than "Folder".
- Settings rows are grouped: the settings that decide what happens to an incoming
  file now run unbroken, and Port — set once, if ever — moved down next to About.
  `Alias` is **Device name**, `Save to` is **Save folder**, and `Existing files`
  asks **If a file exists**.
- A settings row's action moved out of the row and into the footer, where every
  other screen keeps its buttons.
- The radar badges only peers announcing plain HTTP, in red. Encryption is the
  norm now, so an HTTPS badge on every row was noise and the exception was not
  visible as an exception.
- History rows use the tab bar's arrows for direction — ↓ came in, ↑ went out —
  instead of a second, unrelated arrow language.
- A transfer's ETA reads in minutes and hours once it passes a minute ("5m 05s"),
  rather than counting seconds to "~305s".

### Added

- The Receive screen shows the folder incoming files land in, under the address.
- The destination picker's countdown turns red for its last ten seconds.

## [0.5.0] - 2026-08-08

### Added

- Folders are received as folders. Protocol v2 carries a folder transfer as
  files whose `fileName` holds the relative path, which the receiver flattened
  to a basename; it now rebuilds the sender's tree under the save folder, up to
  eight levels deep, with every component sanitized on its own so a path still
  cannot leave the save folder. Files that arrive inside a folder skip the save
  routes — a folder lands whole instead of split across console folders — and
  loose files route as before. **Settings → Received folders** switches it off
  (`[transfer] keep_folders = false`) for the old flattening.

## [0.4.1] - 2026-08-05

### Changed

- A held direction wakes the event loop once per auto-repeat instead of spinning
  it. The loop stopped blocking on the event pump at all while a direction was
  down, so it ran a core flat out to fire repeats at the configured interval; the
  block is now bounded by the earlier of egui's repaint request and the moment
  the next repeat comes due.
- The browser and history lists lay out and paint only the rows in view. A ROM
  folder can hold thousands of entries, and every one of them was a laid-out,
  painted row on every frame. History rows differ in height depending on whether
  they carry a path line, so their tops are prefix sums rather than a fixed step.
- A frame builds the data for the one screen it draws. Peers, history rows and
  save routes were all assembled every frame no matter which screen was on top,
  so covered screens still paid for their allocations; the base-screen precedence
  the render pass already followed now picks what to build. The peer and history
  counts the command router clamps its cursors against are from the last frame
  that showed those tabs.
- Directory listings sort on a cached lowercase key — one allocation per entry
  rather than two per comparison.

### Added

- The About screen names the author, next to the version, build date, commit and
  project URL.

## [0.4.0] - 2026-08-04

### Changed

- Left and right switch tabs, on every tab — the tab bar no longer needs the
  shoulder buttons, which some of these pads place awkwardly. L1/R1 still do it.
  The port is typed on the on-screen keyboard now (A on the row) rather than
  stepped one number at a time with left/right, which was the only thing left
  holding those two directions; every other settings row was already on A.
- The ARM binaries are cross-built with `cargo-zigbuild`, which puts the glibc
  floor in the target triple instead of pinning the build to whichever distro
  ships that glibc — no archived apt suites in the pipeline. One build per arch
  feeds every package, so the armhf binary in the PortMaster zip is the same file
  the OnionOS zip carries. `tools/arm/build.sh` runs the same thing locally.

### Added

- An OnionOS package for the Miyoo Mini Plus and Flip (the plain Mini has no
  wifi): `retsend-onionos.zip` unzips at the card root into `App/Retsend/`,
  built and released by a workflow of its own. It ships the SDL2 the platform's
  own ports use and preloads it the same way, keeps the config and received
  files on the card, skips the GL probes the SSD202 cannot answer, and wires
  MENU to quit through `pressMenu2Kill` — nothing on these devices raises an
  SDL quit event.

### Fixed

- The armhf binary starts on a glibc 2.28 userland, and the aarch64 one now
  loads on older ArkOS. Both were built against whatever glibc the CI runner
  had — 2.31 for armhf, 2.35 for aarch64 — so armhf asked for `log`, `log2`,
  `pow@GLIBC_2.29` and `gettid@GLIBC_2.30` on a 2.28 userland and the loader
  refused it before `main()`. Both are built with `cargo-zigbuild` against a
  pinned glibc 2.28 now, and CI fails the job if a reference above the floor
  comes back. `port.json` states that floor for PortMaster.
- Text is no longer clipped along the bottom of every glyph on the Miyoo Mini
  Plus and Flip, and the UI no longer stutters. Both came from the same place:
  egui's meshes were rasterized as triangles, and the SDL 2.26 those devices'
  SDL2 forks are built on drops the last row of a textured triangle. Glyphs and
  plain rectangles are blitted now instead, which is also cheaper than per-pixel
  triangle work on a CPU with no GPU behind it.
- Buttons work on the Miyoo Mini Plus and Flip. Their SDL2 offers the pad as a
  joystick with no gamepad mapping and sends key presses instead (A is Space, B
  is Left Ctrl, Start is Return), none of which matched the desktop bindings.
  The layout is picked from the `mmiyoo` video driver, or by `RETSEND_KEYMAP`.

## [0.3.0] - 2026-08-02

### Added

- Devices can be added to the radar by IP: X on the Send tab types `ip[:port]`
  (port defaults to 53317) and registers with whatever answers there — the way
  in on networks where multicast discovery never arrives, like guest Wi-Fi and
  AP-isolated access points. HTTPS is tried before plain HTTP, `/info` before
  giving up, and a hand-added device stays on the radar instead of expiring
  after two minutes of silence. It still has to be routable: this replaces
  discovery, not the network path.
- The incoming-request modal takes a folder: X opens the browser, Start drops
  that transfer into the folder on screen. The pick is the whole answer to
  "where" — save routes are skipped, so nothing gets moved out of the folder
  just chosen. The request keeps counting down while you browse, so the header
  shows the seconds left before it auto-declines.
- armhf builds for 32-bit handhelds: a `retsend-linux-armhf.zip` release asset,
  and both binaries in the PortMaster zip, picked by `DEVICE_ARCH` at launch.
- Missing GL is no longer fatal: GLES 3.0, then desktop GL, then SDL's renderer
  (software if that is all there is). `RETSEND_SOFTWARE=1` skips GL outright.
  Every build needs SDL 2.0.18 or newer on the device now — that painter calls
  `SDL_RenderGeometry`.

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

[Unreleased]: https://github.com/mxmgorin/retsend/compare/v0.5.3...HEAD
[0.5.3]: https://github.com/mxmgorin/retsend/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/mxmgorin/retsend/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/mxmgorin/retsend/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/mxmgorin/retsend/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/mxmgorin/retsend/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/mxmgorin/retsend/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/mxmgorin/retsend/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/mxmgorin/retsend/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/mxmgorin/retsend/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mxmgorin/retsend/releases/tag/v0.1.0
