# Research Brief: RetSend Knulli Support

**Depth:** wide, read-only research  
**Status:** complete — implementation is blocked pending remediation and real-device evidence  
**Scope:** Evaluate RetSend v0.9.0 for [Knulli App Store issue #2](https://github.com/jellydn/knulli-app-store/issues/2). This is not an implementation or a catalogue admission decision.

## Executive Summary

RetSend is a plausible product fit, but it is not ready for the Knulli App Store. The current v0.9.0 ARM64 artifact has been inspected and is structurally safe as a ZIP, but the GitHub release is mutable, the PortMaster launcher is not reusable, and Knulli runtime support is not proven.

RetSend also has application-level blockers: TLS peer certificates are not checked against the advertised device fingerprint, received-file overwrite defaults to enabled, receive-path containment is not symlink-safe, and transfers have no free-space or aggregate-session quota. The App Store does not sandbox a launched package. These issues need upstream changes or a deliberately constrained Knulli integration before any installable status.

**Decision:** Keep the prospective package as a non-installable **candidate**. TrimUI Smart Pro may move to controlled real-device testing after security and release blockers are addressed. MagicX Zero 28 needs a confirmed Knulli installation source and its own complete evidence matrix.

## Evidence Status

| Area | Result | Confidence |
| --- | --- | --- |
| RetSend v0.9.0 artifact and dynamic requirements | Directly inspected | High |
| RetSend runtime, input, paths, and transfer behaviour | Source reviewed at `1775a07` | High for source facts |
| Knulli App Store lifecycle and package controls | Source and guide reviewed at `a678911` | High |
| TrimUI Smart Pro architecture, Knulli source support, display and input integration | Source and vendor evidence reviewed | High for platform facts; medium for application compatibility |
| MagicX Zero 28 target and display support | Source/project evidence reviewed | Medium; no stable-release evidence |
| Real-device RetSend launch, controls, network, lifecycle and transfer tests | Not performed | Unknown |
| GPL/source correspondence and complete dependency notice inventory | Incomplete | Blocker |

## Verified Release Evidence

The official v0.9.0 assets were downloaded from `mxmgorin/retsend` and inspected locally on 2026-09-16.

| Asset | Compressed bytes | Extracted bytes | SHA-256 | Verified contents/runtime |
| --- | ---: | ---: | --- | --- |
| `retsend-linux-aarch64.zip` | 4,103,397 | 7,958,728 | `f545a7a07e5620f890d3a831d8a018f49a3d44d34b9de5e5747b861751d2c412` | One AArch64 PIE executable; interpreter `/lib/ld-linux-aarch64.so.1` |
| `retsend-portmaster.zip` | 8,263,525 | 15,961,621 | `9f6bcaf2fc27282e8e211236834b3e871e251cc1a8ac17de95398d73f4eb3318` | AArch64 and armhf binaries, PortMaster launcher, metadata, README, screenshot, licence file |

Neither archive contains absolute or parent-traversal paths or symlink members. The ARM64 binary depends on `libSDL2-2.0.so.0`, `libm.so.6`, `libpthread.so.0`, `libc.so.6`, and `libdl.so.2`; its highest observed glibc symbol requirement is `GLIBC_2.28`. This verifies an upstream build target only. It does not prove that either Knulli device can load or render it.

The release API reports `immutable: false`. Thus, the v0.9.0 URL is not immutable release evidence. A Knulli manifest must not claim `immutable: true` for this release; mirror or publish a cryptographically reviewed, immutable package release first.

## RetSend Runtime Findings

### Platform and launcher

- No Knulli launcher, package definition, or release job exists. Current handheld targets are PortMaster, OnionOS, spruceOS, Allium, and muOS (`.github/workflows/build-linux-arm.yml`).
- The ARM workflow builds AArch64 and armhf with a glibc 2.28 symbol ceiling. It links with SDL 2.26.5 but expects the device to provide SDL2 at runtime. RetSend uses `SDL_RenderGeometry`, requiring SDL 2.0.18 or newer.
- The PortMaster launcher cannot be reused. It sources PortMaster control files, invokes PortMaster helpers, and places data relative to its game directory (`portmaster/Retsend.sh`).
- `RETSEND_DATA_DIR` persists `config.toml`, TLS identity, and transfer history; `RETSEND_SAVE_DIR` chooses the default receive root; `RETSEND_BROWSER_ROOTS` exposes additional roots (`src/config/paths.rs`). A Knulli launcher must constrain all of these.

### Input and display

- RetSend uses SDL GameController semantics: A=confirm, B=back, X=alternate action, Y=pin, Start=settings/confirm, Back=refresh discovery, L1/R1=page navigation (`src/event/gamepad.rs`). Semantic mappings are not proof of the printed physical buttons on a Knulli device.
- RetSend defaults to a 640×480-oriented UI with relative scaling. Both 640×480 and 1280×720 need direct layout, orientation, text, and clipping tests.

### Receive and storage safety

- Received names are lexically sanitised. Traversal components, separators, NUL, control characters, and FAT-illegal characters are handled, and directory depth is capped (`src/transfer/files.rs`).
- Inbound writes use random sibling `.part` files, validate the declared byte count, call `sync_all`, and rename only on completion. Stale partial files are cleaned after 24 hours (`src/transfer/inbound.rs`).
- There is no preflight free-space check, preallocation, aggregate-session quota, or inbound content-hash validation. Outbound metadata sets `sha256` to `None` (`src/transfer/outbound.rs`).
- Default `overwrite` is `true` (`src/config/transfer.rs`). Collision-safe naming must be the Knulli default; overwriting should need an explicit per-file confirmation.
- Path safety is lexical, not filesystem-enforced. Existing symlink components can be followed by `create_dir_all` and ordinary file operations. Absolute user routes and selected directories can also bypass the default receive root. This is an implementation-derived risk, not an exploit reproduction.

### Network and identity security

- RetSend uses IPv4 multicast discovery at `224.0.0.167:53317`; TLS TCP begins at 53317 and tries later ports when necessary (`src/net/protocol.rs`, `src/config/network.rs`). It needs LAN multicast and inbound TCP testing, with manual-IP fallback tested separately.
- RetSend persists a self-signed certificate and private key in its data directory. Regeneration changes its advertised identity (`src/net/tls.rs`). The key is written through `std::fs::write` without an explicit restrictive Unix mode or atomic paired-file update.
- Outbound clients set `disable_verification(true)` (`src/net/client.rs:36-45`). The code documents the discovered fingerprint as the trust model, but the live peer certificate is not bound to that fingerprint. A malicious LAN peer can therefore impersonate a discovered/manual peer. Implement certificate pinning, or safe TOFU with a visible new/changed-identity confirmation, before distribution.

## Knulli App Store Controls

At [Knulli App Store commit `a678911`](https://github.com/jellydn/knulli-app-store/tree/a678911010878198fd117d33e504d22dccf15266), the [package guide](https://github.com/jellydn/knulli-app-store/blob/a678911010878198fd117d33e504d22dccf15266/docs/how-to-add-a-package.md) requires immutable versioned release evidence, exact archive metadata, ABI/dependency evidence, narrow `/userdata` writes, and real-device lifecycle proof.

- A candidate may not contain actionable `release`, `compatibility`, or `install` fields. It cannot become experimental, installable, or verified until those controls pass.
- An installable package requires Knulli/AArch64 compatibility, firmware/device/resolution scope, a version-pinned HTTPS release, SHA-256, compressed and installed sizes, and ZIP or `tar.gz` format.
- The installer rejects traversal, duplicate entries, symlinks, devices, pipes, and other non-regular archive members. It stages, journals, rolls back, and tracks package ownership for install, repair, update, and uninstall.
- `network: true` is disclosure metadata, not an operating-system permission. The installer does not sandbox RetSend at runtime ([security model](https://github.com/jellydn/knulli-app-store/blob/a678911010878198fd117d33e504d22dccf15266/docs/security-model.md)).
- Menu integration is optional and must use an evidenced `/userdata` gamelist path. PortMaster and generic Batocera locations are not evidence. Preserve paths only protect archive-relative paths, not arbitrary runtime files elsewhere.

## Device Compatibility Matrix

| Requirement | TrimUI Smart Pro | MagicX Zero 28 |
| --- | --- | --- |
| CPU / ABI | A133/Cortex-A53 Knulli configuration is AArch64. | A133 configuration is AArch64; vendor identifies an A133P Cortex-A53. |
| Knulli support | Official Knulli stable device image exists. | Source-level/experimental support exists, but the reviewed latest stable asset list has no Zero 28 image. |
| Display evidence | Knulli source has a 1280×720 SDL2 EGL/PowerVR patch; vendor panel is 1280×720. | Source/project evidence indicates a rotated 640×480 framebuffer; vendor panel is 640×480. |
| SDL2 / glibc runtime | Buildroot glibc source configuration and SDL path exist; installed versions and `SDL_RenderGeometry` remain unverified. | Same family evidence, but exact installed library, backend, and glibc are unverified. |
| Input evidence | Knulli starts TrimUI input support and exports `SDL_GAMECONTROLLERCONFIG`; physical labels to SDL semantics remain unverified. | Runtime diagnostic reports `magicx-input` and a nonzero GUID; checked-in physical button indices are not established. |
| Wi-Fi / LAN | Built-in Wi-Fi support is documented; multicast, inbound TCP, reconnect, and AP isolation remain untested. | Vendor specifies 2.4 GHz Wi-Fi; Knulli driver and RetSend LAN behaviour remain untested. |
| Admission verdict | **Likely compatible, but only for controlled real-device testing after blockers close.** | **Insufficient evidence for App Store admission.** Confirm a Knulli installation source first. |

Shared A133 hardware is not compatibility proof. Treat the device, Knulli version, display mode, controller GUID, and resolution as separate evidence matrices.

## Licensing and Supply Chain

RetSend declares GPL-3.0 and identifies `mxmgorin` as its upstream author in `Cargo.toml`. The declared dependency set includes system SDL2 and Rust crates. The following must be demonstrated before Knulli redistribution:

The `libEGL.so` provenance concern found in other handheld bundle paths does not apply to the inspected generic ARM64 asset, which contains only `retsend`. It becomes a Knulli blocker only if a future Knulli bundle reuses or adds that library.

1. Produce an exact corresponding-source offer/archive for the reviewed commit and every bundled or modified native component.
2. Include the GPL text and all applicable third-party notices in the Knulli package/release materials.
3. Resolve the provenance, source availability, copyright, and licence of every distributed native library. Do not redistribute a component with unknown licence/provenance.
4. Generate a dependency licence inventory/SBOM and add a licence policy check.
5. Pin build actions by reviewed commit, retrieve build inputs over HTTPS with verified checksums/signatures, and publish provenance/attestation. The current ARM workflow downloads some build inputs without that level of integrity control.
6. Publish a signed/attested, immutable Knulli asset. A checksum that is stored beside a mutable upstream asset is insufficient.

## Proposed Knulli Package Boundary (future design)

This is a design proposal, not a manifest.

- Publish a dedicated versioned Knulli ZIP; do not repurpose the generic binary or PortMaster archive.
- Install under a narrow owned destination, for example `/userdata/roms/tools/RetSend`.
- Ship a Knulli-specific `RetSend.sh` plus `retsend` as declared executables.
- Set `network: true` and document multicast/inbound-TCP use.
- Set `RETSEND_DATA_DIR` to a preserved package-relative `data/` directory, with secure TLS-key creation and migration.
- Set `RETSEND_SAVE_DIR` to a separate reviewed inbox, for example `/userdata/roms/retsend-inbox`. Do not default to the full ROM root.
- Set only reviewed browser roots. Disable or constrain absolute routes, arbitrary one-transfer destination selection, and symlink traversal.
- Add menu metadata only after verifying the target Knulli gamelist path and launcher operation on each device.

## Go / No-Go Blockers

| Blocker | Required outcome |
| --- | --- |
| Mutable v0.9.0 release | Publish or mirror an immutable, version-pinned artifact with reviewed SHA-256 and provenance. |
| TLS peer impersonation | Pin the presented certificate to the discovered fingerprint, or implement safe explicit TOFU. |
| Receive containment and overwrite | Use symlink-safe rooted file operations; constrain routes; default to collision-safe saves; make overwrite explicit. |
| Space and transfer limits | Preflight available space with a safety margin; add configurable per-file and per-session ceilings; test ENOSPC and cleanup. |
| Licence/source correspondence | Complete GPL/LGPL and dependency notices/source review, including all bundled native components. |
| Knulli runtime proof | Test ELF loader, glibc, SDL2 symbols/backend, rendering, controller semantics, launcher exit, and network behaviour per device. |
| MagicX Zero 28 platform proof | Establish a supported Knulli installation source and independent device evidence. |
| Lifecycle proof | Pass manifest, archive, install, repair, update/rollback, uninstall, preserved-data, and ownership tests. |

## Real-Device Test Plan

Run separately on each device, recording device revision, exact Knulli version, RetSend artifact hash, and tester/date.

1. **Runtime baseline:** record board, firmware, `uname -m`, glibc, ELF interpreter/symbols, `ldd` output, SDL2 version, `SDL_RenderGeometry`, renderer, display geometry, and rotation.
2. **Input:** map every printed physical button through evdev to SDL semantic action. Test navigation, Select, Back, Settings, refresh, paging, clean exit, hotkeys, and reconnect.
3. **UI:** capture 640×480 and 1280×720 screens. Check scaling, focus, text size, clipping, keyboard, and orientation.
4. **Network:** test offline start, DHCP/DNS, multicast discovery, manual IP, inbound/outbound TCP, AP/client isolation, Wi-Fi reconnect, and sleep/resume.
5. **Transfers:** test files, folders, large files, duplicates, hostile filenames, interrupted transfers, restart cleanup, low-space conditions, hash verification, and collisions.
6. **Containment and identity:** verify no writes leave approved paths; test symlinked path components; test TLS identity persistence, changed-peer confirmation, and config preservation.
7. **Package lifecycle:** test fresh install, repeated install, repair, update, rollback, uninstall, menu refresh, and preserved data. Inspect ownership state and backups after each failure case.

## Suggested Delivery Phases

1. **Security and provenance:** resolve TLS pinning, rooted receive writes, collision default, quotas, licence/source correspondence, reproducible immutable release, and SBOM.
2. **Knulli launcher and package:** add a dedicated launcher, package assembly CI, constrained data/inbox configuration, and candidate manifest/documentation.
3. **TrimUI Smart Pro evidence:** run the full real-device plan; publish logs, dependency output, screenshots, exact lifecycle results, and archive inventory. Consider experimental status only if every mandatory control passes.
4. **MagicX Zero 28 evidence:** first establish Knulli image support; then repeat the complete independent matrix. Do not inherit TrimUI evidence.
5. **Catalogue admission:** seek verified status only after the exact device/resolution/version matrix has real-device evidence and all blockers close.

## Primary Sources

- [Knulli App Store issue #2](https://github.com/jellydn/knulli-app-store/issues/2)
- [Knulli package guide, `a678911`](https://github.com/jellydn/knulli-app-store/blob/a678911010878198fd117d33e504d22dccf15266/docs/how-to-add-a-package.md)
- [Knulli security model, `a678911`](https://github.com/jellydn/knulli-app-store/blob/a678911010878198fd117d33e504d22dccf15266/docs/security-model.md)
- [RetSend v0.9.0 source commit `1775a07`](https://github.com/mxmgorin/retsend/tree/1775a07bcc378b5cd83f792d5c14c5362e77fbed)
- [RetSend v0.9.0 release](https://github.com/mxmgorin/retsend/releases/tag/v0.9.0)
- [RetSend release API metadata](https://api.github.com/repos/mxmgorin/retsend/releases/382021042)
- [Knulli A133 board configuration](https://github.com/knulli-cfw/knulli-linux/blob/knulli-main/configs/knulli-a133.board)
- [Knulli MagicX Zero 28 experimental guide](https://github.com/jellydn/knulli-app-store/blob/main/docs/magicx-zero-28.md)
- [Knulli latest releases](https://github.com/knulli-cfw/knulli-linux/releases/latest)
