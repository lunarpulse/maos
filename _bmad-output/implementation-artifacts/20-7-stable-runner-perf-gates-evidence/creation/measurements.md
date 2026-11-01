# 20-7 creation — measurements

## Instrument

Prebuilt `/mnt/build/cargo-maos/debug/xtask` (`$CARGO_TARGET_DIR`, inherited; no cargo ran), invoked as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=<copy>/xtask <xtask> {check-kernel-baseline,kloc-check} --json` from a
`git archive a4c00897` copy per prototype under `/tmp/proto207/` (deleted after measurement). Prototypes were
formatted with `rustfmt 1.9.0` (edition 2021, default config — the repo has no `rustfmt.toml`), never applied to the tree, never compiled.
Raw figures: `kloc-kernel-a4c00897-and-prototypes.json`.

## Baseline (untouched copy)

`check-kernel-baseline` passed: 26801 physical, 95 files, set hash `c023d0d7…`; `kloc-check` passed: kernel 21011
tokei, aggregate 188816, `alarm = true`, `over_budget = []`. Matches the pins (`xtask/kernel-core-baseline.toml:505`,
`xtask/kloc.toml:264,718`).

## Kernel prototypes (all touch `crates/maos-kernel-core/src/security/sandbox/`)

| Shape | Files | Physical | Tokei | Diff |
|---|---|---:|---:|---|
| **K1w** — before removal, `run_teardown` waits (bounded by its own deadline) for an in-flight snapshot owner to publish instead of returning `TerminationUnconfirmed`; snapshot-before-removal order kept; deterministic (D-20-7-01 A) | `t3/child.rs` | +18 (26819) | +15 (21026) | `k1w-wait-snapshot-then-remove.diff` |
| **K1** — `run_teardown` proceeds to removal with the snapshot still in flight when `termination_confirmed()` holds; still refuses when nothing confirmed the exit (D-20-7-01 B) | `t3/child.rs` | +4 (26805) | +1 (21012) | `k1-remove-confirmed-dead-payload.diff` |
| **K2r** — SIGKILL the verified payload pidfd before the runtime CLI kill, via `rustix::process::pidfd_send_signal` (adds `"process"` to kernel-core's existing `rustix` features; maos-bin already enables it, `crates/maos-bin/Cargo.toml:123`) (D-20-7-02 A) | `t3/child.rs`, `Cargo.toml` | +22 (26823) | +16 (21027) | `k2r-pidfd-kill-rustix.diff` |
| **K2** — same through a new `libc::syscall(SYS_pidfd_send_signal)` helper with an `unsafe` block in `sandbox/linux.rs` (D-20-7-02 B) | `t3/child.rs`, `linux.rs` | +44 (26845) | +33 (21044) | `k2-pidfd-kill-libc.diff` |
| **R = K1w + K2r (recommended)** | both | **+40 (26841)** | **+31 (21042)** | `k1wr-k1w-k2r-bundle.diff` |
| R′ = K1 + K2r | both | +26 (26827) | +17 (21028) | `k12r-k1-k2r-bundle.diff` |
| R″ = K1 + K2 | three | +48 (26849) | +34 (21045) | `k12-k1-k2-libc-bundle.diff` |

Every shape moves both kernel pins and the content hashes of the touched files. ADR-069
(`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): R reaches 21042 tokei, 958 under the 22000 alarm;
its ×2 ceiling (21073) is 927 under; no ENVELOPE ALARM row.

Why K1w over K1: K1's failing-before race and its fix both hinge on `termination_confirmed()`; K1w removes the race
itself (the observer's in-flight `inspect` is bounded by the same snapshot deadline and notifies `changed` on publish,
`child.rs:236-241`), so teardown always reaches `rm` once the owner publishes, pidfd or not. At HEAD the hosted logs
show the race both ways: 4/6 S3 failures carry the `TerminationUnconfirmed` residue, 2/6 do not
(`flake-diagnosis.md`).

## Non-kernel forecasts (not measured — [INFERENCE] from analogues)

| Crate · component | Forecast (tokei) | Analogue |
|---|---:|---|
| xtask · `perf-budgets.toml` loader + schema | +60…+100 | — |
| xtask · `check-perf-verdict` (ledger read, calibration, 5-run median, MAD regression, fingerprint, 4 verdicts, 48 h staleness) | +250…+400 | `check_rto_gate.rs` 292 + `check_rto.rs` 303 code lines |
| xtask · `check-perf-runner-policy` (workflow YAML, ≥4 refusals) | +60…+120 | `serde_yaml` already an xtask dependency |
| xtask · perf-ledger append (hosted follow-up job) | +80…+150 | `check_rto.rs` collector half |
| xtask · release precondition reads the perf ledger | +30…+60 | `check_release_precondition.rs` 81 code lines |
| xtask · Tier C count comparison vs committed baseline | +80…+160 | — |
| **xtask total** | **+560…+990** | epic said +250–450 (two gates + loader only) |
| maos-bench `src/` · budget loader use, NFR-Perf-2/4/5 and Tier C harness helpers (counting allocator) | +60…+200 | `harness/j1.rs` spawn helper precedent; `benches/` uncharged (`kloc_check.rs:305`) |
| maos-bin `src/` · `WorkerBinding` drop never panics (D-20-7-03 A), real-spawn seam if one is needed (D-20-7-06 A) | 0…+40 | `supervision.rs:1453-1470,2503-2504` |
| maos-kernel-core `tests/`, `benches/` | uncharged | — |

## Not measured here (owed at T0; a measured figure above the ruled envelope is STOP + D-row)

1. Compilation of R / R′ (`cargo check -p maos-kernel-core`, `cargo clippy`), and whether any kernel-core-only
   build reds on the added `rustix` feature (K2r).
2. The pidfd SIGKILL against a rootless podman 4.9.3 and 5.7 container's pid 1 from the root's uid (expected
   permitted: the test itself `kill -KILL`s the same pid, `worker_terminal_facts_17_7.rs:269-273`).
3. On a stalled-inspect run, which exit cause S3's journal records: client `Exited{125}` or pidfd `signaled(9)`
   (`supervision.rs:933-950` → `classify_exit_with_container`).
4. The ac4 hosted diagnostic: root stderr, payload `/proc/<pid>/status` `State`/`PPid`, `podman ps -a`/`inspect`
   of the CID at the moment the 5 s wait fails — decides D-20-7-02 between K2r (live payload) and the oracle
   (zombie awaiting reap).
5. A loaded-host reproduction attempt of the supervisor barrier (≥ 50 runs under the whole-package parallel
   nextest), with the stub calls log printed.
6. Tier C instrument availability on `ubuntu-24.04`/`26.04` hosted images (`valgrind --tool=callgrind`), and its
   wall-clock cost per PR.
