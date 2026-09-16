# Knulli Build Facts

- RetSend produces a dedicated versioned Knulli AArch64 release ZIP from its CI workflow.
- The Knulli ZIP contains a Knulli-specific launcher, the AArch64 RetSend executable, GPL-3.0 text, third-party notices, source metadata, and release checksum metadata.
- The archive holds a single `RetSend/` folder at its root (launcher, binary, `LICENSE`, `NOTICES`, `README.md`) so manual and App Store installs land cleanly; no catalogue manifest or ES menu integration ships.
- The Knulli launcher does not source or invoke PortMaster helpers.
- The launcher stores RetSend configuration, history, and TLS identity at `/userdata/system/configs/retsend` — on the persistent `/userdata` partition, outside the package tree, so install, repair, update, and uninstall cannot destroy or silently regenerate the TLS identity or transfer history. Uninstall intentionally leaves it behind; a future manifest must disclose that.
- The launcher creates and uses `/userdata/roms/retsend-inbox` as its default receive directory.
- The launcher exposes only reviewed Knulli storage roots to RetSend's file browser: exactly `/userdata/roms` and `/userdata/roms/retsend-inbox`, set explicitly via `RETSEND_BROWSER_ROOTS` rather than relying on auto-detection. `HOME` points at the package directory so the browser's home-root rule adds no extra root and the data dir is unreachable from the browser.
- The launcher creates its directories and verifies the binary before starting, failing closed with a non-zero exit if either step fails; it starts the binary with `exec` and sets no display or input overrides absent device evidence.
- The Knulli artifact is labelled generic AArch64 and does not claim support for a named handheld before real-device evidence exists.
- CI records the archive inventory, compressed size, extracted size, and SHA-256 for each Knulli artifact.
- CI downloads build inputs over HTTPS and verifies pinned external inputs before use: every external input (zig, cargo-zigbuild, SDL2 debs) is version-pinned and SHA-256-verified via `sha256sum -c -` before use, and GitHub Actions are pinned by commit. Provenance attestation and signed releases remain out of scope.
- Knulli release build tests verify launcher shell syntax, archive contents, ZIP safety, executable mode, ELF architecture, dynamic requirements, and required licence/source files. The dynamic-requirement check enforces an exact NEEDED allowlist (SDL2 plus the libc family); ELF checks require `readelf` and degrade to a warning where it is absent.
- Dependencies are disclosed by family in `NOTICES` with `Cargo.lock` as the exact version record; a per-crate SBOM is planned, not done.
- This goal does not add a Knulli App Store catalogue manifest, publish an immutable release, or claim device compatibility.
- This goal does not change RetSend TLS, receive-path, overwrite, quota, or transfer security behaviour.
- Deferred catalogue-admission blockers (TLS pinning, TLS key file mode, receive-path containment, overwrite default, quotas, immutable signed release, attestation, per-crate SBOM) are tracked in `goals/knulli-build/deferred-blockers.md`.
