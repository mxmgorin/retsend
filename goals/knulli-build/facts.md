# Knulli Build Facts

- RetSend produces a dedicated versioned Knulli AArch64 release ZIP from its CI workflow.
- The Knulli ZIP contains a Knulli-specific launcher, the AArch64 RetSend executable, GPL-3.0 text, third-party notices, source metadata, and release checksum metadata.
- The Knulli launcher does not source or invoke PortMaster helpers.
- The launcher stores RetSend configuration, history, and TLS identity at `/userdata/system/configs/retsend`.
- The launcher creates and uses `/userdata/roms/retsend-inbox` as its default receive directory.
- The launcher exposes only reviewed Knulli storage roots to RetSend's file browser.
- The Knulli artifact is labelled generic AArch64 and does not claim support for a named handheld before real-device evidence exists.
- CI records the archive inventory, compressed size, extracted size, and SHA-256 for each Knulli artifact.
- CI downloads build inputs over HTTPS and verifies pinned external inputs before use.
- Knulli release build tests verify launcher shell syntax, archive contents, ZIP safety, executable mode, ELF architecture, dynamic requirements, and required licence/source files.
- This goal does not add a Knulli App Store catalogue manifest, publish an immutable release, or claim device compatibility.
- This goal does not change RetSend TLS, receive-path, overwrite, quota, or transfer security behaviour.
