# 20-1 creation measurements (2026-11-01, HEAD `a4c00897`)

No cargo ran (parent rule: inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos`, no build, no test, no override). No
crate source in the tree was modified: every prototype was made on a `git archive HEAD` copy under
`/tmp/proto201/<name>/` (deleted after measurement), `rustfmt --edition 2021` run on every changed file, then measured
with the prebuilt gate binary **from that copy's root**:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto201/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
(tokei inside `kloc-check`). Every `.diff` here passes `git apply --check` against the working tree. Raw results:
`kloc-kernel-a4c00897-and-prototypes.json`. Prototypes were not compiled ([INFERENCE] that they build).

## Instrument replication

| Figure | Method | In-tree and `/tmp` copy `base` | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26801**, 95 files, set hash `c023d0d7…`, changed/added/removed `[]`, rc 0 | `xtask/kernel-core-baseline.toml:505` |
| Kernel tokei | `kloc-check --json` | **21011** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed=true`, `alarm=true`, `over_budget=[]`, rc 0) | maos-bin **33195** · maos-cli **6396** · maos-registry **3452** · maos-spirit-cli **753** · maos-compliance **2032** · maos-mcp **1174** · maos-capability **1433** · maos-domain **10093** · maos-iac **8270** · maos-shell **608** · xtask **42860** · aggregate **188816** | `:517` · `:489` (6856 reserved; E19 STOP 6382, exceeded under grant203c) · `:688` (4282) · `:696` (1273) · `:481` (2117) · `:430` (1194) · `:265` · `:425` · `:406` · `:691` · `:339` · `_aggregate_hardfail` `:718` = 188817 |

Aggregate room: **1 line**. maos-registry (830), maos-spirit-cli (520), maos-compliance (85) and maos-mcp (20) have
numeric pin room, but under D02 A no numeric E20 room is free: every add needs the aggregate grant.

## Kernel-touching options, priced by throwaway prototype

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Also |
|---|---|---|---:|---:|---|
| fmt0 | **CONTROL, no semantic change**: split the one >100-column `json!` line (`applier.rs:163`) that makes rustfmt skip the `apply_crl` body | `fmt0-applier-format-control.diff` | **+40** (26841) | **+40** (21051) | the formatting any applier edit pays |
| KQ0 | Quarantine arm fences via the existing pause fence (`set_paused`), keeps the Spirit loaded, no drain/terminate; non-sticky (D-20-1-02 B) | `kq0-fence-only.diff` | **+43** (26844) | **+40** (21051) | net of fmt0: +3 / 0 |
| KQ1 | Dedicated quarantine set in `CapabilityRegistryAdapter` (`set_quarantined`/`is_quarantined`; `is_paused` = paused ∪ quarantined so every existing fence site honours it); arm sets it; unload clears it; a director resume cannot lift it (D-20-1-02 A) | `kq1-sticky-quarantine.diff` | **+68** (26869) | **+59** (21070) | net of fmt0: +28 / +19 |
| KQ2 | KQ1 + outbound IAC fence through the applier's held-but-unused `_iac` handle (`applier.rs:33`) (D-20-1-02 C) | `kq2-sticky-quarantine-plus-iac-egress.diff` | **+69** (26870) | **+60** (21071) | **maos-iac +24**, **maos-domain +2** (`IacBusError::SenderQuarantined`; exhaustive matches not searched) |
| P | Revocation poller edge-trigger: first failure + recovery with count (D-20-1-03 A; D-18-0-12 B) | `p-poller-edge-trigger.diff` | **+10** (26811) | **+9** (21020) | — |
| **R = KQ1 + P (recommended)** | | `krec-kq1-p-bundle.diff` | **+78 (26879)** | **+68 (21079)** | additive |
| R′ = KQ2 + P | | `kalt-kq2-p-bundle.diff` | +79 (26880) | +69 (21080) | maos-iac +24, maos-domain +2; aggregate +95 |
| Rmin = KQ0 + P | | `kmin-kq0-p-bundle.diff` | +53 (26854) | +49 (21060) | — |

**Non-kernel comparison (D04 B, as the operator asked):**

| # | Prototype | Diff | Measured | What it does NOT do |
|---|---|---|---:|---|
| B | maos-bin only: FR59 yank-policy `quarantine` through the pub Story 19-2 fence (`TlYankObserver` resolves the pid, `capability.set_paused(pid, true)`, journals `spirit.quarantined`); the door refuses `pause` on a Running+fenced Spirit so pause→resume cannot lift it | `b-nonkernel-yank-quarantine.diff` | **maos-bin +43**, kernel 0 | FR13's CRL `Quarantine` arm is untouched: a polled CRL (`poller.rs:65`, inside the kernel) still drains then **terminates** (`applier.rs:170-197`) — FR13 quarantine stays false; `scheduler.resume` callers other than the door (`main.rs:7747`, `verb_resolver.rs:105`) still lift the fence |

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): R at 21079 tokei is 921 under the 22000 alarm;
the envelope is not headroom — R needs a FLAG-Winston grant row. Any option re-pins `src_lines`
(`kernel-core-baseline.toml:505`) and the content hashes of the changed files; `check-service-boundary` decides whether
the new pub fns (`set_quarantined`, `is_quarantined`) move the surface baseline (not run; measurement owed 3).

## Out-of-kernel, priced by prototype

| Prototype | Diff | Measured |
|---|---|---:|
| S1 `maos-registry-server --bind --root [--signing-key]` over `SpiritRegistryServer::new` + `LocalFsRegistryStorage::at_path` (std args, no new dependency) | `s1-registry-server-binary.diff` | **maos-registry +38** |
| Measured total under R + S1 | — | kernel +78 / +68, maos-registry +38, **aggregate +106** |

## Runtime measurements at HEAD (prebuilt binaries, no cargo)

`runtime/exit-probe.sh` (isolated `HOME`/`XDG_*` under `/tmp/c201`, deleted) → `runtime/exit-probe-a4c00897.txt`.
Lines 2 and 4 green on a dev-signed two-binary dist; 5 rc 1 (stub); 7 rc 1 (guard); 8 rc 2 (clap); 9 dispatch prints
`unknown spirit`; the daemon one-shot server is functional and yanks any Spirit for an unauthenticated
`registry.deprecate`, stamping a process-relative `yanked_at_ns` (2885). One run leaked a probe server (a subshell
`kill`); it was found by `ss -ltnp`, its environment checked (`HOME=/tmp/c201/home`) and killed; the script now kills
the `env`-exec'd pid directly.

## Not measured here (owed to the dev pass)

1. Compilation of every prototype (`cargo check --locked -p maos-kernel-core -p maos-iac -p maos-domain -p maos-bin -p maos-registry`).
2. Exhaustive matches over `IacBusError` (KQ2) and every reader of `is_paused` that must now distinguish pause from quarantine (`operator_door.rs:963` reports `fenced`).
3. `check-service-boundary` on the new pub fns (KQ1) and on `RegistryOrigin::Remote`.
4. Whether a T3 Worker Spirit under a capability fence can still act inside its container (the fence refuses token issuance/verification; container egress is bounded by the T3 sandbox only) — the door's pause path freezes Workers (`operator_door.rs:870-900`), the applier cannot. Hosted T3 leg (AC11.5).
5. Out-of-kernel adds other than S1 and B (forecast only, story §Kernel and kloc pricing).
6. `release-dry-run` with four artifacts and the `epic-20-exit` job (cargo build + componentize-js; hosted).
7. The ≤5 min propagation end to end across processes (server yank → daemon poller → policy) on a 30 s clamp floor (`yank.rs:200-210`): wall-clock cost against the 20-12 test budget.
