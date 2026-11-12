# 21-5 creation — measurements (2026-11-12, HEAD `0baa76fb`)

## Instrument

Prebuilt gate binary `/mnt/build/cargo-maos/debug/xtask` (built 2026-11-11 23:06; `xtask/src` last changed at
`a2bbd687`, 2026-11-11 15:52, and is clean in the working tree, so the binary is HEAD's gate code), run from each copy's
root as `xtask {check-kernel-baseline,kloc-check,check-service-boundary,check-workspace-count,check-loom,check-empty-kernel,check-serde-error-handling} --json`
and `xtask stability-matrix --check`, on `git archive 0baa76fb` copies under `/tmp/c215/` (files touched after
extraction; never a worktree; deleted after measurement). Changed `.rs` files were formatted with
`rustfmt 1.9.0 --edition 2021`; `diff` against the untouched copy shows only the intended hunks. `tokei 14.0.0`
(the CI pin). Prototypes were never applied to the working tree and never compiled ([INFERENCE] that they build;
"Not measured here" 1). Every `.diff` here passes `git apply --check -p1` against the working tree. Raw figures:
`kloc-kernel-0baa76fb-and-prototypes.json`; surface triples: `surface-delta-0baa76fb.json`.

One `cargo tree --offline` (metadata only, no build, no target directory) ran in the KP copy to read the dependency
closure (below). **Incident, disclosed:** `check-cohort-mesh` was invoked once on the untouched copy before reading its
source; it shells out to `cargo build -p maos-bin` (`xtask/src/check_cohort_mesh.rs:26,48,157`). The build ran in
`/tmp/c215/head/target` (not the shared target, not the repo) for about 40 s, was killed, produced no result, and its
directory was deleted. No other cargo process was started by this creation.

## Baseline (untouched copy) — reproduces the pins

`check-kernel-baseline` rc 0: **27040** physical, **95** files, set hash `5551f890…`, changed/added/removed `[]`.
`kloc-check` rc 0: kernel **21201** tokei, aggregate **202855** (hardfail 202856), `alarm=true`, `over_budget=[]`.
Touched crates (measured = ceiling, zero headroom): maos-domain 10217 · maos-bin 38549 · xtask 45379;
`maos-orchestrator-buffer` row 917, measured 0 (forward-declared, `kloc.toml:682-684`).
`check-service-boundary` rc 0 (401 baseline items, 0 violations). `check-workspace-count` rc 0 (58/58).
`check-loom` rc 0. `check-empty-kernel` rc 0. `check-serde-error-handling` rc 0. `stability-matrix --check` PASS.
`tokei crates/maos-kernel-core/src/orchestrator/`: 1028 lines, **817 code** — per file (code): `buffer.rs` 170,
`registry.rs` 65, `mod.rs` 67, `gateway_dispatcher.rs` 441, `echo_gateway.rs` 74.

## What the module is and who calls it (read at HEAD)

| Part | File (physical) | Kernel-internal caller | Outside callers |
|---|---|---|---|
| FR20 buffer | `buffer.rs` (220; `:103-220` is the `#[cfg(test)]` module) | none directly | door queue/recall/status (`operator_door.rs:2040-2095,2852`), root drain (`orchestrator_root.rs:708`), `spirits/orchestrator/tests` |
| Registry | `registry.rs` (90) | **`scheduler_loop.rs:89,113,141,490-505`** — the resume replay | `main.rs:2355-2356,3037,3617,5491,6091`; `operator_door.rs:518,573`; bin test support ×2 |
| Journal fns ×3 | `mod.rs:38-97` (97) | none | `operator_door.rs:1031,1142,2028,2178` only |
| FR54 gateway host | `gateway_dispatcher.rs` (531), `echo_gateway.rs` (90) | none | **no production caller**; kernel tests `gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs` only |

The resume replay drains the buffer and re-enqueues every item into the same buffer
(`scheduler_loop.rs:494-503`, over `buffer.rs:46-80`). With no concurrent caller this is an identity on the queue; a
door `enqueue` or a root `dequeue_at_safe_point` between the drain and the re-enqueue sees an empty queue or lands ahead
of the older items ([INFERENCE] from the lock scopes: each call takes the mutex separately). No test passes a registry
into `SpiritSchedulerAdapter::new` (K26): the replay has no direct witness at HEAD.

## Kernel shapes (◆ FLAG-Winston, negative; D-21-5-01, -02)

Every shape moves the files into `crates/maos-orchestrator-buffer/` (lib = old `mod.rs`, `crate::iac::transparency_log`
→ `maos_iac::transparency_log`, `crate::orchestrator::gateway_dispatcher` → `crate::gateway_dispatcher`), re-points
maos-bin (`maos_kernel_core::orchestrator::` → `maos_orchestrator_buffer::`, 5 files) and `spirits/orchestrator/tests`
(2 files + dev-dependency), moves the kernel tests (K24, K25) into the new crate's `tests/`, adds the workspace member and
the post-21-5 workspace-count sentence.

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Kernel files | New crate (tokei / src physical) | Other crates (tokei) | Aggregate |
|---|---|---|---:|---:|---|---:|---|---:|
| **KP** | `maos_domain::orchestrator::OrchestratorResumePort { fn replay_pending(&self, spirit_id: &str) }`; scheduler field/param `Option<Arc<dyn OrchestratorResumePort>>`, resume calls `port.replay_pending(&scb.spirit_id)`; the leaf's registry implements the port with the moved replay body (D-02 A, **recommended**) | `kp-domain-port-leaf-move.diff` | **−1041 (25999)** | **−827 (20374)** | 95 → 90; changed `lib.rs` (−1), `scheduler/scheduler_loop.rs` (−12 / −9), `isolation/mod.rs` (doc, 0) | **829 / 1046** | domain **+3**, bin 0, xtask +1 (control, D-06) | 202855 → 202860 (**+5**); +6 with the control |
| KB | kernel depends on the leaf crate; field/param re-typed to `maos_orchestrator_buffer::OrchestratorBufferRegistry`, body unchanged (D-02 B) | `kb-kernel-depends-on-leaf.diff` | −1029 (26011) | −818 (20383) | 95 → 90; `lib.rs`, `scheduler_loop.rs` | 817 / 1028 | 0 | −1 (202854) |
| KD | kernel drops the replay: field, ctor param and block removed (D-02 C) | `kd-drop-kernel-replay.diff` | −1049 (25991) | −833 (20368) | 95 → 90; `lib.rs`, `scheduler_loop.rs` | 817 / 1028 | bin −1 (the `Some(registry)` arg); 37 more callers (35 tests, 2 benches) lose one positional `None` (uncharged; not in the diff) | −17 (202838) |
| KH | KP port, **buffer half only**: `buffer.rs`, `registry.rs`, the three journal fns move; `gateway_dispatcher.rs`, `echo_gateway.rs` stay at `maos_kernel_core::orchestrator::` with an 11-line `mod.rs` (D-01 B) | `kh-buffer-half-only.diff` | −407 (26633) | −304 (20897) | 95 → 93; `lib.rs` (comment), `orchestrator/mod.rs`, `scheduler_loop.rs` | 308 / 419 | domain +3 | +7 (202862) |

Gate results on the prototypes (pins and baselines deliberately NOT refreshed, so each red is the work the landing owes):

| Gate | KP | KB | KD | KH |
|---|---|---|---|---|
| `check-kernel-baseline` | rc 1: 25999 ≠ 27040; removed 5 files; changed `isolation/mod.rs`, `lib.rs`, `scheduler_loop.rs`; set hash `bb22880f…` | rc 1: 26011; set hash `4c39e1a7…` | rc 1: 25991; set hash `9eef3291…` | rc 1: 26633; set hash `a8a86451…` |
| `kloc-check` | rc 1: `maos-domain 10220 > 10217`, aggregate 202860 ≥ 202856 (202861 with the xtask control, and `xtask 45380 > 45379`) | rc 0 | rc 0 | rc 1: domain, aggregate |
| `check-service-boundary` | rc 1: 22 removed triples, 1 added (below) | rc 1: 22 / 1 | rc 1: 22 / 1 | rc 1: 10 / 1 |
| `check-workspace-count` | rc 0 (59/59 with the new sentence) | same | same | same |
| `stability-matrix --check` | FAIL — `STABILITY.md:30` "workspace crates 58" | same | same | same |
| `check-loom`, `check-empty-kernel`, `check-serde-error-handling` | rc 0 | — | — | — |

Dependency closure (KP, `cargo tree --offline -p maos-kernel-core --edges all`): `maos-orchestrator-buffer` occurs
**0** times; the leaf's normal dependencies are `dashmap`, `maos-attrs`, `maos-domain`, `maos-iac`, `maos-manifest`,
`maos-spirit-abi`, `thiserror`. `Cargo.lock` gains one package entry and two dependents (maos-bin, orchestrator).
Under KB the kernel depends on the leaf, so the D-06 control cannot be written.

## Public surface (`check-service-boundary`, K11/K12)

KP, KB and KD remove the same **21** orchestrator triples (12 items + 9 `pub use` leaves) and change the
`SpiritSchedulerAdapter` struct hash (its private field type is hashed): **22 removed, 1 added**. KH removes 9 (the
buffer half: 6 items, 3 leaves) + the struct hash. Full 64-hex hashes: `surface-delta-0baa76fb.json`.

| Kind | Path (`maos_kernel_core::…`) | HEAD hash | KP | KH |
|---|---|---|---|---|
| struct | `scheduler::scheduler_loop::SpiritSchedulerAdapter` | `2daf34b24f2563ac…` | **refresh → `d2170d389c72d692…`** (KB `77bfe43017690a92…`, KD `891bd15e7a12c6d4…`) | refresh → `d2170d389c72d692…` |
| enum | `orchestrator::buffer::OrchestratorBufferError` | `c8b994d079168735…` | removed | removed |
| struct | `orchestrator::buffer::OrchestratorBuffer` | `c2f19b8a5a4d608a…` | removed | removed |
| struct | `orchestrator::registry::OrchestratorBufferRegistry` | `f07938833d9959b1…` | removed | removed |
| fn | `orchestrator::journal_orchestrator_queue` | `a430e2f986fac353…` | removed | removed |
| fn | `orchestrator::journal_director_lifecycle_action` | `29f7bf96e33dd41b…` | removed | removed |
| fn | `orchestrator::journal_token_revocation` | `bb5d648b449b75a4…` | removed | removed |
| struct | `orchestrator::echo_gateway::EchoGatewayFactory` | `7f04203c6946f7b6…` | removed | kept |
| struct | `orchestrator::echo_gateway::EchoGatewaySubmodule` | `ff590b62d552c059…` | removed | kept |
| struct | `orchestrator::gateway_dispatcher::GatewayDispatcher` | `e687b4485353b935…` | removed | kept |
| struct | `orchestrator::gateway_dispatcher::GatewayInstance` | `6e9f58998b68d91d…` | removed | kept |
| struct | `orchestrator::gateway_dispatcher::GatewaySubmoduleRegistry` | `93820db9d6428033…` | removed | kept |
| trait | `orchestrator::gateway_dispatcher::GatewaySubmoduleFactory` | `5a5e01e8f2fb5731…` | removed | kept |
| use | `orchestrator::buffer::OrchestratorBuffer` | `fab11b646d107620…` | removed | removed |
| use | `orchestrator::buffer::OrchestratorBufferError` | `b6b819a36eae6447…` | removed | removed |
| use | `orchestrator::registry::OrchestratorBufferRegistry` | `a330507117b40627…` | removed | removed |
| use | `orchestrator::echo_gateway::EchoGatewayFactory` | `ead79c101ed4c354…` | removed | kept |
| use | `orchestrator::echo_gateway::EchoGatewaySubmodule` | `998c737b27d3ee5d…` | removed | kept |
| use | `orchestrator::gateway_dispatcher::GatewayDispatcher` | `9f8b7959d07f42e3…` | removed | kept |
| use | `orchestrator::gateway_dispatcher::GatewayInstance` | `f93095489864c794…` | removed | kept |
| use | `orchestrator::gateway_dispatcher::GatewaySubmoduleFactory` | `610108159a9698d4…` | removed | kept |
| use | `orchestrator::gateway_dispatcher::GatewaySubmoduleRegistry` | `6e48c02949956a07…` | removed | kept |

Unchanged in every shape: `use scheduler::scheduler_loop::SpiritSchedulerAdapter` (`bd25e470…`) and
`use api::crate::scheduler::SpiritSchedulerAdapter` (`a0572416…`) — a `use` leaf hashes its path text, not the type.
The new domain trait is outside the kernel surface (the gate walks `maos-kernel-core/src` only). `abi-diff` reads
`maos-spirit-abi` only (`abi_diff.rs:8`) and no shape touches it (`diff -rq` clean); the abi baselines
`xtask/abi-baseline/*.txt` are unaffected. `xtask/kernel-api-classes.toml:367-375` holds nine rows for removed paths.

## ADR-069 and the STOP (kernel tokei; `docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46,56`)

| Point | Kernel tokei | To the 22 000 alarm / STOP | To the 23 000 envelope |
|---|---:|---:|---:|
| HEAD `0baa76fb` | 21201 | 799 | 1799 |
| HEAD + owed 20-9 R (+21, priced at `8c809f9c`) and 20-13 KT3 (±1) [INFERENCE: not yet landed] | ≈ 21221–21223 | ≈ 777–779 | ≈ 1777–1779 |
| KP on HEAD | **20374** | **1626** | **2626** |
| KP after the owed rows [INFERENCE: no file overlap — 20-9 R is `schedule_watchdog.rs`, 20-13 KT3 is the security manager] | ≈ 20394–20396 | ≈ 1604–1606 | ≈ 2604–2606 |
| KB / KD / KH on HEAD | 20383 / 20368 / 20897 | 1617 / 1632 / 1103 | 2617 / 2632 / 2103 |

No shape crosses the alarm, so no row needs `ENVELOPE ALARM`; the HISTORY text records the relief (D-21-5-03).

## Not measured here (owed at T0 of the dev story)

1. That KP (or the ruled shape) compiles and passes clippy: `cargo check --workspace --all-targets`, focused suites.
2. `check-cohort-mesh` (needs a maos-bin build and its legs) and `check-dependency-closure` through the gate (the
   closure itself was read with `cargo tree`, above).
3. Formatted figures after `cargo fmt --all` on the real tree (the prototype used `rustfmt --edition 2021` per file).
4. The maos-bin focused suites that construct the door with a registry: `shell_host_16_2`, `erasure_million_20_11`,
   `worker_freeze_19_2`, `orchestrator_queue_19_2`; maos-cli `orchestrator_queue_test`, `orchestrator_recall_pick_19_2`,
   `pause_resume_test`; maos-control `post_surface_16_1`; `cargo test -p orchestrator`.
5. The D-21-5-09 FR21 exemption (maos-iac) was not prototyped: it is not kernel-touching and its design is open.
6. Re-base onto whatever 20-9 L2, 20-13 KT3 and the 20-14 corrections land first (none touches this story's kernel files
   per their records; the set hash and `src_lines` still move).
