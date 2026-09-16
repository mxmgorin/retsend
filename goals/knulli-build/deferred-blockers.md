# Deferred Catalogue-Admission Blockers (knulli-build)

Durable tracker for the security and provenance blockers that `goals/knulli-build/facts.md` scopes out of the knulli-build goal. Source: the research brief's phase-1 Go/No-Go list, narrowed by facts.md's non-goals.

**None of these block candidate packaging** (this goal's output is a non-installable candidate). **All of them block catalogue admission** per the research brief's phase-5 gate. Each blocker stays OPEN until someone records where/how it closed — closing one is a deliberate edit to this file, not a silent disappearance.

| # | Blocker | Why deferred | Closing it requires |
|---|---------|--------------|---------------------|
| 1 | TLS certificate pinning | facts.md non-goal: no TLS behaviour changes | RetSend-core security work; changes advertised identity behaviour |
| 2 | Restrictive Unix mode on TLS key write | Upstream fix, out of packaging scope — the research brief flags that RetSend creates its key via `std::fs::write` without a restrictive mode | Change key creation to owner-only mode in RetSend core |
| 3 | Symlink-safe rooted receive writes | facts.md non-goal: no receive-path changes | Receive-path hardening in RetSend core |
| 4 | Collision-safe overwrite default | facts.md non-goal: no overwrite changes | Overwrite policy change in receive path |
| 5 | Transfer quotas / size ceilings | facts.md non-goal: no quota changes | Quota enforcement in receive path |
| 6 | Immutable, signed release publication | facts.md non-goal (publishing an immutable release is out of scope) | Signing infrastructure plus an immutable release workflow |
| 7 | Provenance attestation for CI artifacts | This goal's supply-chain bar is version-pinned + SHA-256-verified inputs only | Attestation for build inputs and artifacts, including a second verification channel for cargo-zigbuild (it publishes no checksum file) |
| 8 | Per-crate licence inventory / SBOM | `NOTICES` ships family-level disclosure with `Cargo.lock` as the exact record — a deliberate stopgap | Generate an SBOM from Cargo.lock; verify each crate's declared licence, including egui-sdl2 |

## Notes

- #2 is only partially mitigated by this goal: the data dir lives outside the package tree, so a package operation can't force identity regeneration — but the key is still created with a permissive mode on first run.
- #7's weakest link today: cargo-zigbuild's SHA-256 has a single HTTPS source (see `tools/knulli/README.md`).
- The AArch64 binary is built twice per release (ARM workflow and Knulli workflow). If Knulli graduates, fold the Knulli job into the main workflow and deduplicate.
