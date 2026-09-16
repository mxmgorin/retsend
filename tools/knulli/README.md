# tools/knulli

Release build tests and build-input bookkeeping for the Knulli package.

## verify-package.sh

`verify-package.sh <package-dir> [aarch64|armhf]` implements the Knulli
release build tests: launcher shell syntax (and its no-PortMaster rule),
required licence/source files, ZIP safety of the assembled tree (no symlinks,
no hidden entries), executable mode, ELF architecture, dynamic requirements
(the exact NEEDED list; anything beyond SDL2 + libc family fails), and an ELF
interpreter sanity check. CI runs it before any artifact is uploaded.

It needs `readelf` for the ELF checks and skips them with a warning when it
is missing (so the file/launcher checks still run on minimal hosts).

## Pinned external inputs

The Knulli workflow (`build-knulli.yml`) pins and verifies every external
build input before use:

| Input | Version | SHA-256 source |
| --- | --- | --- |
| zig (`zig-x86_64-linux`) | 0.15.2 | official `ziglang.org/download/index.json` shasum |
| cargo-zigbuild | 0.23.0 | fetched over HTTPS from the release tag; no upstream checksum file publishes one |
| libsdl2-2.0-0 deb | 2.26.5+dfsg-1 (arm64) | downloaded from the Debian pool over HTTPS |
| libsdl2-dev deb | 2.26.5+dfsg-1 (arm64) | as above |

GitHub Actions are pinned by commit SHA (`actions/checkout` v7.0.1,
`actions/upload-artifact` v5.0.0, `actions/download-artifact` v5.0.0), with
the version in a comment. Bump a version above and you must re-derive its
checksum — the workflow fails closed otherwise.
