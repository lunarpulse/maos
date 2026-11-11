# 20-9 L1 correction round 4 (the last) — evidence (Dev209L1d)

Reviews of the round-3 corrections (`f56959c2`): `../../review/gpt-review-r3.md` (2 P2 + the snapshot-owner closure note) and `../../review/security-r3.md` (GLM-5.3: 1 High, 2 Medium, 1 Low, 2 Info), persisted verbatim.

## Method

- **Red-before:** a scratch `git archive HEAD` (`0e455c4e`, which is round 3 plus the 20-5 L1 candidate; the latter edits no file of this story and its only `maos-bin` source change is `worker_spawn.rs`) in `/tmp/base209r4`, with ONLY the changed test files overlaid, built with its own target dir `/tmp/base209r4-target` (passed per command, never exported) and its own fixtures; deleted afterwards. Raw logs `red/red-0e455c4e.log` (six tests: five red, one green, see below) and `red/red-0e455c4e-f4.log` (the F4 vector, added after the first run: red).
- **Plants:** in the final tree, one mutation per fix (`../plants.sh`, exactly-once replace, `PLANT_DIR=round4/plants`, `@@` separator), the proving test, then copy-back + `cmp` + `touch`. Nine plant runs carrying ten textual replacements (`r4-3a` replaces two lines), raw logs `plants/r4-*.log`, each ending `restored … cmp clean, touched`, none `DID NOT APPLY`.
- **Final tree:** `final-focused.log`, `workspace-nextest.log`, `gate-*.log`, `clippy-ratchet.log`, `verb-table-no-default.log`, `check-no-default.log`, `fmt-check.log` (this directory).

## Witness table

| Finding | Witness (final tree) | Red at `0e455c4e` | Plant → result |
|---|---|---|---|
| GPT r3 #1 busy drop attributed to its own invocation | `spawned_lifecycle_20_9::a_telemetry_drop_during_a_successful_forget_notice_leaves_the_purge_delivered` (a forget notice runs its 4 s export successfully while a telemetry event queued behind it is dropped at 2.5 s) | `purge_undelivered: [1]` (the notice read as busy) | `r4-1a-busy-for-every-clean-run` red; `r4-1b-busy-never-seen` red via the round-3 vector `a_telemetry_event_behind_a_long_turn_is_dropped_without_quarantine_or_a_late_run` |
| GPT r3 #2 the wall must not detach a running hook | `hook_pump_17_4::a_consolidation_past_its_wall_is_still_joined_before_shutdown_returns` (compiled-in hook of 9.75 s under a 10 s budget, wall 9.5 s; shutdown starts once the wall has quarantined it) | `shutdown returned while the hook still ran` | `r4-3a-no-drain-at-shutdown` red (both drains removed), `r4-3b-overrun-detached` red |
| GPT r3 #2, spawned half: acknowledged end at the wall | `spawned_lifecycle_20_9::a_telemetry_export_past_its_wall_is_cut_loose_so_shutdown_is_not_held_for_its_budget` (25 s export under a 60 s budget; shutdown within 12 s) | **green at `0e455c4e`**: there the dispatch was dropped at the wall, so shutdown never waited; the vector protects the new join, it does not fail before | `r4-4-link-never-cuts-loose` red (shutdown waits for the export) |
| GPT r3 closure: snapshot owner | `spawned_swap_20_9::a_slow_snapshot_of_a_refused_swap_does_not_fence_the_retry` (refused first attempt whose 5 s snapshot is still running when the retry installs its attempt) | `hot-swap coordinator error: snapshot failed … spawned_swap_unsupported` (the retry's own snapshot answered `Unsupported` by a spuriously fenced predecessor) | `r4-5-snapshot-owner-ignored` red |
| GLM r3 F1 swap-stage walls | `spawned_swap_20_9::a_successor_that_never_becomes_ready_is_refused_at_the_prewarm_wall` and `…never_answers_its_probe…` (successor declaring a 600 s budget; refused with `deadline` between 55 and 80 s, child gone, predecessor serving) | both: `a silent successor held the swap past its 60 s wall` (the 80 s guard) | `r4-6a-ready-unwalled` red, `r4-6b-exchange-unwalled` red |
| GLM r3 F4 queue wait billed to the hook | `spawned_lifecycle_20_9::a_telemetry_export_that_queued_behind_a_turn_is_held_to_its_wall_from_its_own_start` (2 s queued + 4 s run inside a 5 s wall) | `a subscriber that waited 2 s and ran 4 s inside a 5 s wall was quarantined for the wait` | `r4-2-wall-from-dispatch` red |
| GLM r3 F5 `BUSY_DROPS` growth | removed: the per-invocation record is owned and dropped by the dispatch that registered it | — | — |
| GLM r3 F2, F3, F6, per-spirit upgrade lock | dispositioned, no code (kernel source or door routing) | — | `deferred-work.md` §20-9 L1 correction round 4 |

Not witnessed: (1) the inline-exchange error path that drops the successor's child and releases the predecessor directly (the probe guest cannot be made to hang inside `migrate`/`swap-in`; it shares `Prewarm::exchange` with the two vectors above); (2) the 30 s consolidate/forget wall itself (short holds only); (3) `last_consolidate_ns` advancing (asserted through the forget purge, which shares `fire_walled` and the record).

## Pins (exact, after `cargo fmt --all`, `kloc-check --json`)

| Item | Before (HEAD) | Now | Ceiling |
|---|---:|---:|---:|
| maos-bin | 37833 | 38019 (+186, of which the new `invocation.rs` is 74) | 38422 |
| maos-wasm-host | 3137 | 3137 | 3348 |
| xtask | 45315 | 45315 (the 20-5 candidate's pin, +379 over 44936; this story changes no xtask source) | 44980 in the assignment brief, already below the 20-5 pin before this round; not this story's |
| aggregate (clean) | 201814 | 202000 (hardfail 202001) | 202209 |

The tree also holds the operator's untracked `spikes/gpui-operator-shell/` and two `*gpui*` planning files (not mine, not touched): the unmodified aggregate reads 202222, and `(unknown:spikes)` is over its own budget, so `gate-kloc-check.log` is red for that reason alone. Kernel 26841 / 21042 unchanged.

## Observed verification (final tree, exit codes)

- `cargo fmt --all --check` 0; `cargo check -p maos-bin --no-default-features --lib` 0; `cargo test -p maos-bin --no-default-features --test verb_table_15_3` 0.
- clippy (`--workspace --all-targets`) 0; ratchet 0 at 1328 occurrences / 868 distinct (unchanged from round 3).
- Workspace nextest (one run, `--profile ci`, real `cargo build -p maos-bin -p maos-cli` first): 5958 run, 5955 passed, 3 failed, 137 skipped. The three: `kloc_check::tests::kloc_check_runs_on_workspace` (the operator's untracked `spikes/`, as for `gate-kloc-check.log`), `erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (the known flake), and `cohort_daemon_smoke_13_5c::composition_root_does_not_seed_manifest_scopes`, a real failure this round caused: its source roster must list every `maos-bin/src/*.rs` file and the new `invocation.rs` was missing. Fixed by ringing the doorbell (40 → 41, a comment and one entry); the binary alone then passed 9/9 with and without the 20-9 features (`final-cohort-doorbell*.log`). The workspace was not re-run.
- Explicit feature-gated focused run (`MAOS_17_2_CGROUP_MODE=undelegated`, `--features maos-bin/network,maos-bin/wasm-host`; all 20-9 suites, `spawned_scope_17_3c`, the 17-x suites, `erasure_live_20_11`, `cohort_daemon_smoke_13_5c`, `one_daemon_one_door_16_1`, `package(maos-wasm-host)`): 332 run, 331 passed, 1 failed (the same doorbell, before the fix), 3 skipped. The root-shutdown AST guard (`root_shutdown_unload_16_3`) is in that run and green.
- Gates at rc 0: kernel-baseline, dev-record-completeness, epic-close-coherence, exit-commands, decision-register, env-contract, manifest-schema-version, equiv-fixture-provenance, wasm-form-equiv, fkcs, `gen-abi-docs --check`, `stability-matrix --check`, invariant-lock; `kloc-check` rc 1 for the foreign `spikes/` reason only (`aggregate 202222 >= 202001`, `(unknown:spikes) 366 > 244`). The last four (dev-record, decision-register, epic-close, exit-commands) were re-run after the last record edits.
