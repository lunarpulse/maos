# 21-5 L1 (kernel-free) — evidence (2026-11-12, Dev215L1, base `d00802aa`, landed UNCOMMITTED until Main commits)

## What L1 contains

| Item | Where | AC |
|---|---|---|
| `pub trait OrchestratorResumePort: Send + Sync { fn replay_pending(&self, spirit_id: &str); }` (no dependency added; no implementor until L2) | `crates/maos-domain/src/orchestrator.rs` (after `OrchestratorInstruction`) | AC4.1 |
| Equivalence goldens (a) journal rows, (b) buffer/registry semantics, (c) resume replay through the real `SpiritSchedulerAdapter` + never-queued resume — against the UNMOVED `maos_kernel_core::orchestrator` | NEW `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs`, `crates/maos-bin/tests/fixtures/orchestrator_equivalence_21_5.json` | AC5, D-21-5-06 B |
| Replay-race witness `a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue` — RED on the unmoved code, committed `#[ignore]`d | same file | D-21-5-07 B (AC2.2 witness, recorded early) |
| Dated notes, no text deleted: FR20 (`functional-requirements.md:54`), FR54 (`:68`), §4.0.7 (`4-kernel-design.md:186`) | planning-artifacts | AC4.2, D-21-5-01 A |
| Exact pins: `kloc.toml` `maos-domain` 10274 → 10277, `_aggregate_hardfail` 203241 → 203244 (`scripts/repin.py --apply --story 21-5-orchestrator-port-and-move … --adjudicated maos-domain`) | `xtask/kloc.toml` | AC7.1 (L1 part) |

No file under `crates/maos-kernel-core/src` changed (plants were restored by copy-back + `cmp` + `touch`; `git status` shows no kernel path). No ADR-070 (L2). Deferred-work re-route was already done by Main.

## Witness table

| Witness | Red-before (observed) | Plant (observed) | Final |
|---|---|---|---|
| (a) `a_journal_rows_equal_the_fixture` | n/a: golden, green at HEAD (baseline) | P1 reasoning format → FAIL | pass |
| (b) `b_buffer_is_fifo_bounded_and_drains_in_order` | golden, green | P2 `push_front` → FAIL | pass |
| (b) `b_registry_get_never_creates_and_get_or_create_is_idempotent` | golden, green | P4 `get` creates → FAIL | pass |
| (c) `c_resume_replay_keeps_order_and_count` | golden, green | P3 replay drops last → FAIL | pass |
| (c) `c_resume_of_a_spirit_never_queued_to_creates_no_buffer` | golden, green | P4 → FAIL | pass |
| **race** `a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue` | **RED on the unmoved replay: 11 of 11 runs**, accepted producer enqueues 573…1788, one `resume re-enqueue failed` line each (`red/`) | the unmoved code is the plant | `#[ignore]`d at L1 (hosted stays green); L2 un-ignores, must be green |
| domain trait | no witness for a type (AC4.3); AC2.4 is its witness | — | compiles; maos-domain suite 310-test run green |

Plants: `plants.md`, raw logs `plants/`. Red raw logs: `red/first-run-5-green-1-red.log` (5 pass + 1 red together), `red/run-01.log`, `red/run-08.log`, `red/ten-runs-summary.txt`.

## Pins (live tree, `scripts/repin.py --check` = PINS IN SYNC after the apply)

kernel **27103 physical / 21256 tokei / 95 files**, set hash `de4fdc82c636…` — unchanged (`check-kernel-baseline` rc 0); `maos-domain` **10277** (ceiling = measured; +3 priced, D-21-5-12 A ceiling +6 → inside); `maos-bin` 38602 (tests uncharged, unchanged); `xtask` 45461 (unchanged); aggregate **203243**, hardfail **203244**. The script reports a `maos-domain` "OVERDRAFT +3" for any growth over the live ceiling and needs `--adjudicated maos-domain`; the +3 is exactly the priced D-21-5-12 A figure, so it was applied on that basis and is reported to Main.

## Verification (all observed; `env -u CI -u GITHUB_ACTIONS`)

- `cargo nextest run --locked -p maos-domain -p maos-bin --test orchestrator_equivalence_21_5 --lib --no-fail-fast` → rc 0, 310 passed, 1 skipped (the ignored witness) (`gates/nextest-final-equivalence-and-domain.log`).
- `cargo nextest run --locked -p maos-bin --test orchestrator_equivalence_21_5 --run-ignored only` ×10 → rc 100 each, the race witness red (`red/ten-runs-summary.txt`).
- `cargo clippy --locked -p maos-domain -p maos-bin --all-targets --message-format=json` rc 0, **0 diagnostics on the two touched Rust files** (the `-D warnings` form fails on pre-existing `maos-spirit-abi` lints under rustc 1.98.1, which is how the repo's advisory clippy-ratchet leaves them; not ours).
- `cargo fmt --all -- --check` rc 0.
- Gates (prebuilt `/mnt/build/cargo-maos/debug/xtask`, rebuilt by `cargo build --locked -p xtask`, `--json`): check-kernel-baseline, kloc-check, check-service-boundary, check-workspace-count (58/58), check-loom, check-empty-kernel, check-dependency-closure, check-serde-error-handling, check-dev-record-completeness (199 done, 0 violations), check-epic-close-coherence, check-exit-commands, check-decision-register, check-test-cost, `stability-matrix --check` all rc 0; `invariant-lock --changed-files` for the domain file and the new test rc 0 (`gates/`).
- `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule --test kernel_pin_content_hash_16_0` → 46 passed.

Not run: full workspace nextest and `prepush` (workspace tests; ACCEPTED PROCESS: hosted CI authoritative); `check-cohort-mesh` (L2 AC1); a hosted run.

## L2 plan — what L1 learned about the move's real hazards

1. **The race is a data-loss bug, not only an ordering blip.** Each producer enqueue accepted during a resume window is matched one-to-one by a `resume re-enqueue failed: … at capacity (32)` line: the replayed tail is refused and the director's queued instruction is gone. D-21-5-07 B (one lock) therefore fixes a real loss; the `eprintln!` path is reachable today.
2. **Replay "under one lock" is an identity on the queue.** Draining and re-enqueueing the same items inside one guard changes nothing, so the leaf body is observably a lock-and-nothing. The buffer's `queue` is private: the leaf needs a `OrchestratorBuffer` method that holds the single guard (the replay cannot be written in the registry alone). The registry-level plant (AC2.5 "make the leaf replay drop the last item") needs that body to do work (re-push inside the guard) or the plant has nothing to mutate; if L2 prefers the honest no-op, AC2.5's plant moves to the kernel recording-port vector. This is a price/AC wording question for Main (±5 tokei forecast stands).
3. **The L1 witness already runs through the real scheduler**, so at L2 the file's diff is the `use` lines (`maos_orchestrator_buffer::…` for buffer/registry/journal fns; the scheduler/domain imports stay) plus removing `#[ignore]`. `Some(registry)` (an `Arc<OrchestratorBufferRegistry>`) coerces to `Option<Arc<dyn OrchestratorResumePort>>` at the `new` call site, so the tests' constructor call needs no edit; maos-bin already gains the leaf dependency at L2.
4. **The KP prototype still applies to the live tree.** `git apply --check -p1` of `creation/kp-domain-port-leaf-move.diff` with only the domain hunk excluded (that hunk is now in the tree) returns rc 0: `scheduler_loop.rs`, `lib.rs`, `isolation/mod.rs`, the five module files and the bin re-points did not drift through the 20-9/20-13/20-14 landings. The kernel price on the live base is therefore the KP figure on 27103/21256: expect about **−1041 physical / −827 tokei → ≈ 26062 / 20429** [INFERENCE: not re-measured; apply and measure at T0].
5. **Hosted:** the ignored witness is not in the `expensive-tests` filter (`check-test-cost --expensive-filter` selects registered binaries only), so L1's `#[ignore]` runs nowhere until L2; the 5 goldens run in the default lane.
6. The red witness prints one `eprintln!` line per lost instruction (hundreds per run); when green at L2 the test is silent.
