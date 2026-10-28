# 19-4 creation measurements (2026-10-28, HEAD `61af7461`)

No cargo was run (parent rule: a build held the host). Every figure below was produced WITHOUT touching a crate source
file in the tree: HEAD was extracted read-only to `/tmp/h19x4` (`git archive HEAD | tar -x`), the kloc instrument was
replicated with tokei 14.0.0 on that copy and on the working tree, and every prototype was applied to a throwaway
copy under `/tmp/proto19x4/<name>/`, formatted with `rustfmt --edition 2021` (the changed files only), counted, and
saved here as a diff. Nothing to revert; `intent-lineage-coverage-report.md` untouched (no test ran).

## Instrument replication

| Figure | Method | HEAD 61af7461 | Working tree (61af7461 + uncommitted 19-3/19-6) | Pin |
|---|---|---|---|---|
| per-crate tokei code | argv of `xtask/src/kloc_check.rs:294-318`, grouped as `infer_crate_name` `:386-417` (`tokei-61af7461.json`) | maos-bin **30380** · xtask **47451** · maos-bench 2451 · kernel **20619** · maos-control 1442 · maos-cli 6317 · maos-shell 613 · maos-journey-test 946 · maos-corpus-gen 2341 · aggregate **189877** | maos-bin **30847** (+467) · xtask **47454** (+3) · kernel **20628** (+9, 19-3 K2) · others unchanged · aggregate **190356** (+479) | `kloc.toml:517,339,264,718` (= HEAD exactly) |
| kernel physical | `wc -l` over the 94 git-tracked `crates/maos-kernel-core/src/**/*.rs` | **26378** | **26389** (+11, 19-3 K2) | `kernel-core-baseline.toml:505` = 26378 |

The working-tree figures are unformatted, un-reviewed wave-2 code; they are the best available measure of what is
already consumed and are re-measured at T0.

## Kernel-touching options, priced by throwaway prototype (in `/tmp`, never in the tree)

Only one 19-4 option touches the kernel: **D-19-4-05 C** (cross-boot resume through a kernel rehydration API). Every
recommended row is kernel-Δ 0 (the halt is raised by the existing `process_scalar_write` → `invoke_halt`, resolved by
the existing `KernelHaltResolver`, read by the existing `drain_for_spirit_dry_run`).

| Prototype | D-row | Diff | Kernel physical | Kernel tokei | E19 kernel endpoint (≤ 26483 / ≤ 20686) |
|---|---|---|---:|---:|---|
| K1 — `HaltRegistry::rehydrate_pending(tl, boot_nonce, spirit_id, pid, resolved)`: a cold boot re-pends prior boots' `EpistemicHalt` TL rows that no `halt.resolve` decision names, re-keyed to the new pid (`halt/mod.rs` after `lookup_state` `:437-440`) | D-19-4-05 C | `k1-halt-rehydrate-prototype.diff` (44 lines added) | 26378 → **26422 (+44)** | 20619 → **20659 (+40)** | **breached**: 26422 + 19-3 11 + 19-2 74 (replicated) = 26507 > 26483; 20659 + 9 + 46 = 20714 > 20686. Under the ADR-069 alarm (22000). Needs a FLAG-Winston grant. [INFERENCE] +1–3 more for a spirit-id filter on the rows (the TL row carries the prior pid, not the Spirit id). |

Not compiled ([INFERENCE] it type-checks: `query_frames`/`FrameFilter`/`TransparencyLogEntry` are `maos_iac` items re-exported through `crate::iac` (`crates/maos-kernel-core/src/iac.rs:13`); `serde_json` is a kernel dependency (`halt/mod.rs:552`)).

## Non-kernel pricing (maos-bin), by scratch prototype

`p1-maos-bin-recommended-bundle.diff` — the recommended rows' maos-bin side on a `/tmp` copy of HEAD's
`crates/maos-bin/src`, rustfmt'd, counted with tokei: **30380 → 30564 (+184)**. Per-hunk code-line split (blank and
`//` lines excluded; sums to the tokei delta):

| Component (row) | Lines |
|---|---:|
| D-01 A — `orchestrator_scalar_port` factory beside `ButlerOrchestratorAdapter` (35) + `construct`/`construct_at_admitted_pid` take it (11, incl. a rustfmt re-wrap) + successor arm (9) + topology and standalone arms (8 + 8) | 71 |
| D-03 — allowlist: the bundle adds both tags (A); B adds `dispatch_ambiguity` only (1) | 2 |
| D-02 A — `orchestrator_root::judge_section` (≈ 21) + loop call (1) + `let mut goal` (1) | 23 |
| D-05 A — `hold_for_resolution` (≈ 35) + loop hold/resume arm (30) + `MAOS_EPIC_HALT_HOLD_SECS` read (6) + `env_contract.rs` row (5) | 76 |
| D-06 A — topology-drain digest for `--epic` runs | 12 |
| **Σ** | **184** |

Not compiled: the Spirit side (`Orchestrator::with_scalar_port`, `Orchestrator::assess_ambiguity` → `(value, Option<HaltReceipt>)`)
lives in `spirits/orchestrator` (kloc-ungoverned, `kloc_check.rs:311,408`) and is not in the bundle; `memory.read(…)`'s exact
signature is [INFERENCE]. Test flips in `main.rs`'s `#[cfg(test)]` modules (`:14322-14356`, `:14616-14650`) are counted
by tokei (in-file) but line-neutral [INFERENCE ±3].

Alternatives (estimates from the bundle's hunks): D-01 B (root-only write, no Spirit port, topology arm only) ≈ 30 instead of 72;
D-05 B (maos-bin cross-boot rehydration via `insert_pending_with_metadata` + a `--resume` path) ≈ 90–130 instead of 74
[INFERENCE: the K1 body ≈ 40 + the boot-time join of `halt.resolve` decisions ≈ 20 + the flag ≈ 15 + re-dispatch-from-section ≈ 20–50];
D-06 B ≈ 10 (no `run.epic` guard); D-07 A is Butler (`spirits/`, 0).

## xtask pricing (analogue, [INFERENCE])

Calibration: `evaluate_beats` (`demo_j1.rs:572-802`) is ≈ 170 code lines for 9 beats (≈ 19/beat); the ordered-stages block `:725-767` ≈ 35.

| Component | Lines |
|---|---:|
| `TOPOLOGY` → epic topology + `--epic` fixture arg + hold env for the child | 5–8 |
| P2–P6 beats generalised from "the first/only event" to "every section, in order" (`ORDERED_STAGES` per section) | 15–30 |
| three new beats (`epic-decomposed-n-frames`, `ambiguity-halt-within-2-frames`, `digest-cites-source-refs` — the last joins `completed[]` refs to `maosctl audit query` rows) | 55–80 |
| resume beat + streaming the child's stdout (spawn + reader thread) + door for the child (`maos init` in the isolated home) + `maosctl halt resolve` | 55–75 |
| `unlanded_beats` −1 entry, "does NOT claim" halt/resume lines | −8…−12 |
| D-11 A — loopback derivation walks `crates/*/tests`, crate-qualified enrolment, `_2c` delegated to leg 10 | 25–40 |
| **Σ** | **≈ 147–221** (central ≈ 180) |

## Envelope arithmetic (`epics/epic-19-founder-loop-w4.md:38-48`)

| Crate | Endpoint | Working tree | Room | Still owed (×1.3 or forecast) | Left for 19-4 | 19-4 need (×1.3) | Fits? |
|---|---:|---:|---:|---|---:|---:|---|
| maos-bin | 31353 | 30847 | 506 | 19-2 196–279 (its ruled forecast) + 19-5 195 (150 × 1.3) | **32–115** | 184 → **239** | **no** — short 124–207 |
| xtask | 47624 | 47454 | 170 | 0 (19-3's xtask is in the tree) | 170 | 147–221 → **191–287** | **no** at ×1.3 (central 180 × 1.3 = 234, short 64) |
| maos-journey-test · maos-corpus-gen · maos-spirit-abi | — | — | — | — | — | 0 (assertions are tests, cassettes data; D-08 A line-neutral) | yes |
| kernel | ≤ 26483 / ≤ 20686 | 26389 / 20628 | — | 19-2 ≤ +93 / +58 | — | 0 (recommended); K1 +44 / +40 | yes / **no** for D-05 C |
| aggregate | 191248 | 190356 | 892 | 19-2 315–415 + 19-5 195 | 282–382 | 239 + 234 = 473 | **no** — short 91–191 |

## Measurements owed (the parent runs these; one cargo process; prefix every command)

`P=env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`

1. `$P cargo run -q --locked -p xtask -- check-kernel-baseline` at the tree that commits 19-3/19-6 — expect `PASSED` at the re-pinned figure (26389 if only 19-3 K2 moved it). Why: the K1 basis and the E19 endpoint arithmetic above.
2. `$P cargo run -q --locked -p xtask -- kloc-check --json` at the same tree — expect maos-bin ≈ 30847 (± rustfmt), xtask ≈ 47454, aggregate ≈ 190356. Why: the room in the envelope table and D-19-4-12.
3. **K1 (D-19-4-05 C):** on the HEAD-61af7461 kernel (`halt/mod.rs` is unmodified in the tree) `git apply _bmad-output/implementation-artifacts/19-4-j1-beats-and-demo-replay-evidence/creation/k1-halt-rehydrate-prototype.diff`; `$P cargo check --locked -p maos-kernel-core --lib` (expect exit 0); `$P cargo run -q --locked -p xtask -- check-kernel-baseline` (expect exit 1: +44 over the then-current pin, changed `halt/mod.rs`); `kloc-check --json` (expect kernel +40); `git checkout -- crates/maos-kernel-core/src/halt/mod.rs`; re-run 1.
4. **P1 bundle (recommended maos-bin rows):** `git apply --check` against HEAD only (the tree's `main.rs`/`orchestrator_root.rs` carry 19-6); the tokei figure (+184) needs no cargo; a compile check is the dev pass's, with the Spirit-side `with_scalar_port`/`assess_ambiguity` added.
5. Topology-arm replay binding (hosted or a T3-capable host): `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/orchestrator-halt-corpus-v0.1.json maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md --once` in an isolated home with the T3 env — expect `orchestrator inference seam wired (provider …)` on stderr (19-1 proved it on the standalone arm only, `crates/maos-bin/tests/epic_args_19_1.rs:406-427`). Why: AC2 judges on the topology arm.
6. `maos_audit::query` row shape for a `halt.resolve` approval decision (kind label, `reasoning` carrying `halt=<id>`): `$P cargo test --locked -p butler --test digest` after adding one throwaway assertion in a scratch copy, or read `crates/maos-audit/src/query.rs`. Why: D-19-4-07 A joins resolutions to halt rows.
7. Wall-clock of `cargo run -p xtask -- demo-j1` on a fresh hosted runner with the T3 substrate (workspace debug build + one three-Worker epic run ≈ 3 × 1.2 s T3 + the hold) — from the first `epic-19-exit` run or a branch-only dispatch. Why: D-19-4-10's job timeout.
