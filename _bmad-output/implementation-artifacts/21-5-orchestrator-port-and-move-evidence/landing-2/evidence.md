# 21-5 L2 (kernel move) — evidence (2026-11-12, Dev215L2, base `257a0fd7`, landed UNCOMMITTED until Main commits)

All commands `env -u CI -u GITHUB_ACTIONS`, one cargo process, the repo `target` symlink, xtask `/mnt/build/cargo-maos/debug/xtask`
(rebuilt with `cargo build --locked -p xtask` after the `FORBIDDEN_CRATES` edit).

## What moved

`crates/maos-kernel-core/src/orchestrator/{mod,buffer,registry,gateway_dispatcher,echo_gateway}.rs` → NEW workspace member
`crates/maos-orchestrator-buffer/` (`lib.rs` = old `mod.rs`; `crate::iac::transparency_log` → `maos_iac`). The KP prototype diff applied
to the live tree with `git apply -p1 --exclude='crates/maos-domain/*'` (rc 0; the domain hunk landed at L1). Kernel files changed:
`lib.rs`, `scheduler/scheduler_loop.rs`, `isolation/mod.rs` (doc) + the five deleted files; nothing else. The three kernel tests
(`gateway_dispatcher_fr54`, `gateway_uninstall_fr65_v05`, `lifecycle_journal_write_latency`) moved to the leaf's `tests/`.
`SpiritSchedulerAdapter` holds `Option<Arc<dyn maos_domain::orchestrator::OrchestratorResumePort>>`; `resume` calls
`port.replay_pending(&scb.spirit_id)` (before `fire_on_resume`, as before); the leaf's `OrchestratorBufferRegistry` implements the port;
`main.rs` passes `Some(orchestrator_registry.clone())` (coerces). No shim re-export in the kernel.

**Prototype corrections (KP never compiled):** (1) the leaf needs `maos-capability` and `tokio` (`features rt, time`) as normal dependencies
(`gateway_dispatcher.rs` uses `tokio::spawn`/`time`, `crate::capability::cap_tokens::monotonic_now_ns` → `maos_capability::cap_tokens::...`)
and `serde_json` as a dev-dependency (FR65 test); the creation closure listed only `dashmap, maos-attrs, maos-domain, maos-iac,
maos-manifest, maos-spirit-abi, thiserror`. `Cargo.lock` gained `maos-orchestrator-buffer` with those edges. (2) The moved
`gateway_dispatcher.rs` carries three pre-existing rustc warnings (unreachable pattern, unused `e`, unused `ctx`) unchanged: moved verbatim,
not fixed (no behaviour or token drift). (3) `cargo check --locked` needed one `Cargo.lock` refresh (`--offline` without `--locked`) for the
added dependencies; afterwards `--locked` everywhere.

**D-21-5-07 B (one-lock replay).** NEW `OrchestratorBuffer::replay_pending(&self)`: under ONE guard it pops the front and pushes it back
`len` times (rotation; no allocation; capacity cannot be exceeded; poisoned mutex left alone as `recall_all_pending` does). The registry's
`replay_pending` looks up the buffer (never creates one) and calls it. Observably an identity in sequential use (the goldens hold); what it
removes is the lost-instruction window of the two-lock drain + re-enqueue (and the `eprintln!`). The AC2.5 plant "drop the last item" is
meaningful on this body only as "drop items" (P2a: the re-push becomes `drop(instruction)`) and "off by one" (P2b: loop bound `len-1`,
reorders); a literal `take(len-1)` plant would not mutate (a rotation never empties the queue before the last pop). Said here as the
priced-vs-built difference of the plant, not hidden.

## Witness table

| Witness | Before (observed) | After (observed) |
|---|---|---|
| race `a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue` (`crates/maos-bin/tests/orchestrator_equivalence_21_5.rs`, `#[ignore]` removed) | RED 11/11 on the unmoved two-lock replay (L1, `landing-1/red/`, one `resume re-enqueue failed … at capacity (32)` line per accepted producer enqueue) | GREEN 10/10 on the one-lock replay, 0 `re-enqueue failed` lines (`green/race-run-*.log`, `green/race-ten-runs-summary.txt`) |
| goldens (a)(b)(c) of AC5, same file; the diff of the file = the `use` lines (now `maos_orchestrator_buffer`), the header paragraph (it named the retired path, which the AC3.2 structural test forbids), the removed `#[ignore]` + its doc line | green at HEAD (L1) | 6/6 green on the moved code (`gates/nextest-bin.log`) |
| leaf `crates/maos-orchestrator-buffer/tests/resume_replay_21_5.rs` (3 queued ⇒ same FIFO, count 3; unknown Spirit ⇒ no buffer; producer racing a full-queue replay ⇒ no accepted enqueue, queue unchanged) | no leaf at HEAD (does not compile) | 3/3 green ×6 |
| kernel `crates/maos-kernel-core/tests/orchestrator_resume_port_21_5.rs` (recording port: pause no call; resume ⇒ one call with the Spirit id; a resume of a non-paused Spirit ⇒ no call) | at HEAD the ctor takes the concrete registry, so a recording port does not type-check (compile-level; not re-run at HEAD: it needs a second build tree) | 2/2 green |
| `xtask/tests/orchestrator_out_of_kernel_21_5.rs` (no kernel `orchestrator/` dir or `mod`; no `.rs` under `crates/ spirits/ xtask/src` names `maos_kernel_core::orchestrator`) | creation measured at HEAD: directory present, 15 occurrences in 10 files | 2/2 green |

## Plants (AC1.4, AC2.5, AC3.4; each restored by copy-back + `cmp` + `touch`; raw logs `plants/`)

| # | Plant | Expected red | Observed |
|---|---|---|---|
| P1 | `scheduler_loop.rs`: the `port.replay_pending(&scb.spirit_id);` line commented out | kernel recording vector | FAIL `resume_calls_the_port_once_with_the_spirit_id` (1 pass 1 fail) — `p1-kernel-port-call-deleted.log` |
| P2a | leaf `buffer.rs` replay re-push → `drop(instruction);` | leaf vector, AC5(c), race | FAIL leaf `three_queued…`, `a_producer_racing…`; bin `c_resume_replay_keeps_order_and_count`, race (2+2 fail) — `p2a-leaf-replay-drops-items.log` |
| P2b | leaf `buffer.rs` loop bound `len` → `len.saturating_sub(1)` (off by one, reorders) | same | the same four FAIL — `p2b-leaf-replay-off-by-one.log` |
| P6 | leaf `registry.rs` replay body → the old two-lock `recall_all_pending` + `enqueue` | race witnesses | FAIL leaf `a_producer_racing_a_replay_of_a_full_queue_changes_nothing`, bin race witness; sequential vectors stay green — `p6-two-lock-replay.log` |
| P3 | `maos-kernel-core/Cargo.toml` gains `maos-orchestrator-buffer` | `check-dependency-closure` | rc 1, violations `["maos-orchestrator-buffer"]` in `maos-kernel-core`; `Cargo.lock` restored — `p3-kernel-depends-on-leaf.log` |
| P4 | `pub mod orchestrator;` + stub `orchestrator/mod.rs` in the kernel | structural test | FAIL `the_kernel_has_no_orchestrator_module` — `p4-module-restored.log` |
| P4b | a `.rs` file under `spirits/` naming the retired path | structural test | FAIL `no_rust_file_names_the_retired_kernel_path` — `p4b-retired-path-named.log` |
| P5 | a stub `orchestrator/registry.rs` left in the kernel | `check-kernel-baseline` | rc 1: 26063 ≠ 26062, `added: ["orchestrator/registry.rs"]`, 91 files — `p5-registry-left-in-kernel.log` |

After the plants `git status` shows no kernel path beyond the intended ones; the stub directories were removed.

## Pins (`python3 scripts/repin.py --check` → `PINS IN SYNC`)

| Pin | Before (`257a0fd7`) | After | Notes |
|---|---:|---:|---|
| kernel physical / files | 27103 / 95 | **26062 / 90** | `check-kernel-baseline` rc 0; set hash `6eaa5c6a…` |
| kernel tokei (`kloc.toml:264`, lowered) | 21256 | **20429** | −827; 1571 to the 22000 alarm, 2571 to 23000; HISTORY first segment cites `21-5-orchestrator-port-and-move ->` and carries `ADR-069 RELIEF VALVE (21-5, ruling s21)` |
| `maos-orchestrator-buffer` | 917 (forward row, measured 0) | **833** | exact (D-21-5-10 A; priced 829, +4 replay; ≤ 835 ceiling) — `repin.py --lower` |
| `maos-domain` | 10277 | 10277 | unchanged (L1 landed the +3) |
| `maos-bin` | 38602 | 38602 | 0 |
| `xtask` | 45461 | **45462** | +1 `FORBIDDEN_CRATES`; the script flagged OVERDRAFT +1, adjudicated as the priced +1 (D-21-5-12 A ceiling +2) via `--adjudicated xtask` |
| aggregate / hardfail | 203243 / 203244 | **203250 / 203251** | +7 |
| constants | 27103 / 21256 / 95 | 26062 / 20429 / 90 | `kernel_pin_content_hash_16_0.rs` (4 `src_lines` + `PINNED_ENTRIES` + comment/message), `recovery_lane_ceiling_rule.rs` `RATIFIED_AT_EASING` |

`repin.py` refuses a removed kernel file, so `kernel-core-baseline.toml`'s `[kernel_src]` block (set hash, three changed digests, five rows
deleted: 95 → 90 rows) was adopted from `check-kernel-baseline --emit-pin` by splice before `--apply`; the script then wrote `src_lines`,
the HISTORY block (its boilerplate said "none added or removed; changed files: none" — corrected by hand to name the three changed files and
the five removed rows), the kloc rows and the test constants.

## Surface

`check-service-boundary` rc 0, 0 violations, 380 items. `docs/ci-baselines/kernel-surface-v0.1-beta.json` hand-edited (a one-off
script removed the 21 blocks by exact text and replaced the single hash, asserting every untouched item equal; `git diff --stat`: 1
insertion, 106 deletions): 401 → 380. Refresh (pre-approved, named in the measured grant): `scheduler::scheduler_loop::SpiritSchedulerAdapter`
`2daf34b2…` → `d2170d389c72d69275cbad202942db89f6d5295bb63104fee3f86c30d7896bf1` (equals the creation-priced hash). Removed: the 21 triples of
`21-5-flag-winston-decision.md` under ADR-070. `kernel-api-classes.toml` nine rows + banner deleted; `serde-error-allowlist.toml` two stale
rows (`gateway_uninstall_fr65_v05.rs:147/148`) deleted (`check-serde-error-handling` rc 0 after). `abi-diff` not run: it cannot run on a dirty
tree and reads `maos-spirit-abi` only (untouched).

## Verification (observed)

- `cargo fmt --all -- --check` rc 0.
- `cargo check --locked --workspace --all-targets` Finished (warnings only; the three moved rustc warnings above).
- `cargo nextest run --locked -p maos-orchestrator-buffer -p orchestrator -p maos-domain` 414 passed, 1 skipped (an existing ignore).
- `cargo nextest run --locked -p maos-kernel-core --test orchestrator_resume_port_21_5` 2/2. `cargo test --locked -p maos-kernel-core --lib` 239 passed (`gates/kernel-lib.log`).
- maos-bin `shell_host_16_2`, `worker_freeze_19_2`, `orchestrator_queue_19_2`, `orchestrator_equivalence_21_5`, `erasure_million_20_11`: 37 passed (`gates/nextest-bin.log`). The first run failed 3 tests with `test fixture stale — maosctl is older than Cargo.toml`
  (an environment fact: the workspace `Cargo.toml` gained a member); `cargo run --locked -p xtask -- build-test-fixtures` (rc 0, `gates/build-test-fixtures.log`) rebuilt the fixtures and the rerun is green.
- maos-cli `orchestrator_queue_test`, `orchestrator_recall_pick_19_2`, `pause_resume_test`: 21 passed. maos-control `post_surface_16_1`: 42 passed.
- xtask `d11_xtask_ceiling_ratchet`, `recovery_lane_ceiling_rule`, `kernel_pin_content_hash_16_0`, `orchestrator_out_of_kernel_21_5`, `decision_adrs_and_provisioning`: 51 passed.
- Gates rc 0 (`gates/`): `check-kernel-baseline`, `kloc-check` (aggregate 203250), `check-service-boundary` (0 violations, 380), `check-dependency-closure`, `check-workspace-count` (59/59), `stability-matrix --check`, `gen-abi-docs --check`, `check-loom`, `check-empty-kernel`, `check-serde-error-handling`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`, `check-test-cost`, `check-unsafe`, `check-pub-field-constructors`, `check-composition-root-completeness`, `check-env-contract`, `error-catalog-check`, `check-fr47`, `check-review-findings-resolved`; **`check-cohort-mesh` rc 0, 39 legs** (run once, from the repo checkout, as the one cargo process: it builds `maos-bin`).
- `invariant-lock --changed-files` for `scheduler_loop.rs` and the leaf `buffer.rs`: rc 0.
- `cargo clippy --workspace --all-targets --locked --message-format=json` rc 0 (10m44s) → `clippy-ratchet` (`gates/clippy-ratchet.txt`): **1331 occurrences / 869 source-distinct** against the hosted baseline 1383 / 897 (the advisory ratchet refuses growth only; passes). The only diagnostics in new-crate files are the eight moved-verbatim ones in `gateway_dispatcher.rs` (`unreachable_patterns` ×2, `unused_variables` ×4, `clone_on_copy` ×2), which the kernel carried before; none of the hosted-lint list (`result_large_err`, `collapsible_if`, `too_many_arguments`, `unused_mut`, `trim_split_whitespace`) in the new code.
- Final run on the final tree (`gates/nextest-xtask-final.log`): `d11_xtask_ceiling_ratchet`, `recovery_lane_ceiling_rule`, `kernel_pin_content_hash_16_0`, `orchestrator_out_of_kernel_21_5`: 48 passed; `repin.py --check` PINS IN SYNC; `check-dev-record-completeness`, `check-epic-close-coherence`, `check-decision-register`, `check-exit-commands` rc 0 after the story/tracker edits.

## Records

Story (tasks, Completion Notes, File List, Change Log; Status stays `in-progress`), `21-5-flag-winston-decision.md` L2 landing record, tracker row comment, ADR-070 + `docs/adr/index.md` row, ADR-069 relief note, `4-kernel-design.md:117` (59), `STABILITY.md` (59), `xtask/kernel-api-classes.toml`. No deferred-work row is closed by this landing (the replay race, routing E1, was fixed at origin and never filed; `deferred-work.md:355` `with_capacity(0)` and `:1465` the FR21 exemption are untouched).

## Not verified / open

Hosted run; the review; the full workspace nextest (ACCEPTED PROCESS: hosted CI authoritative); `prepush`; `abi-diff`; a real second-tree failing-before of the kernel vector at HEAD (compile-level argument only).
