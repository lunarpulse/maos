# 20-9 L1 review corrections — evidence (Dev209L1d)

Reviews of `cdf3937d`: `../review/gpt-review.md` (6 P1 + 1 P2) and `../review/security.md` (GLM-5.3, 1 Medium, 4 Low, 1 Info), persisted verbatim.

## Method

- **Red-before:** a scratch `git archive cdf3937d` (`/tmp/base209d`, own target `/tmp/base209d-target`, never the shared target; deleted afterwards) with ONLY the new/changed tests, `support/` files and the two guest fixtures copied in; fixtures rebuilt there. One run, raw log `red/red-cdf3937d.log`: **13 selected tests, 13 failed**, each for the intended reason (below).
- **Plants:** in the final tree one mutation per fix (`plants.sh`, exactly-once replace, file backed up first), the proving test run, then copy-back + `cmp` + `touch`. Raw logs `plants/<label>.log`; every log ends `restored … cmp clean, touched`. A first plant pass was discarded: an exported `CARGO_TARGET_DIR` pointing at the scratch target made two plants compile stale artefacts (cargo hashes path dependencies relative to the workspace root, so the two trees collide), and the `=>` separator broke one mutation; the table is from the second pass on the shared target.
- **Final tree:** `final-focused.log`, `workspace-nextest.log` (see the end).

## Witness table

| Finding | Witness (final tree) | Red at `cdf3937d` (`red/red-cdf3937d.log`) | Plant → result |
|---|---|---|---|
| GPT #1 lifecycle callers | real `maos run` daemon: `spawned_door_20_9::a_component_run_by_maos_run_receives_the_idle_schedule_and_consolidate_triggers` (idle/consolidate/schedule read off the daemon's own TL: the probe's recall page size names the export; teardown joined, no `drain timed out`); AST guard `root_shutdown_unload_16_3::every_audit_sender_release_joins_the_yank_owner` | `these triggers never reached the component: [("idle", 1), ("consolidate", 2), ("schedule", 4)]`; guard: `the run arm starts its lifecycle callers and joins them` | `p1-no-callers` (callers never started) red; `p1-no-expect-session` (idle lost to the startup race) red; `p1-guard-no-join` (the join removed) guard red |
| GPT #2 installed roots | `spawned_installed_20_9::an_installed_component_is_upgraded_in_place_through_the_door_without_maos_run` (registry publish → `maosctl spirit install` → `maos shell` boot → door hot swap, same pid) | `upgrade_failed: successor factory failed: swap_form_mismatch: 'lifecycle-probe' is in-process and the successor is spawned (HTTP 500)` | `p2-installed-unarmed` red |
| GPT #3 admission readiness | real door: `spawned_door_20_9::a_cold_swap_to_an_existing_invalid_component_is_reverted_and_the_predecessor_serves`; in-process: `spawned_swap_20_9::a_component_that_cannot_start_is_unwound_and_refused_by_its_cold_swap_admission` | door reply `"outcome":"completed"` (no `cold_swap_reverted`); in-process: `Admitted { pid: 2 … }` for a non-component | `p3-admit-returns-at-spawn` red (both tests) |
| GPT #4 `on_swap_in` disabled | `spawned_swap_20_9::a_successor_whose_manifest_disables_on_swap_in_is_refused_and_the_predecessor_keeps_serving` | `a successor that cannot be handed the state is refused: ()` (the swap succeeded) | `p4-swap-in-gate-removed` red |
| GPT #5 late snapshot reply | `spawned_swap_20_9::a_snapshot_answered_after_its_callers_deadline_does_not_fence_the_predecessor` (3 s cap, 2 s turn, 2 s `sleep-in snapshot`) | `no report within 15 s for lifecycle-probe` (predecessor fenced for good) | `p5-late-reply-fences` red |
| GPT #6 prewarm deadline | `spawned_swap_20_9::a_prewarm_guest_that_never_stops_writing_diagnostics_is_refused_at_its_cap` (hostile-twin `stderr-flood` before `Ready`, `probe-flood` after it, through the real factory; child gone) | `stderr-flood: the swap outlasted the successor's 2 s cap by 10 s` | `p6-prewarm-deadline-only-idle` red |
| GPT #7 handover gap | `spawned_swap_20_9::a_hook_dispatched_in_the_committed_handover_gap_reaches_the_successor_once` (predecessor `sleep-in shutdown 3` holds retire open) | `exactly one guest invocation, none lost to the gap: counts idle=0` | `p7-gap-hooks-unsupported` red |
| GLM F1 pump stall | `spawned_lifecycle_20_9::one_slow_telemetry_subscriber_does_not_stall_the_others`; `hook_pump_17_4::repeated_budget_warnings_quarantine_the_subscriber` | `the fast subscriber still waits for the slow one after 1.8 s`; the near-budget subscriber is never quarantined | `f1-sequential-pump` red; `f1-slow-strikes-reset` red |
| GLM F1 overlap panic | `spawned_lifecycle_20_9::a_hook_that_cannot_start_behind_another_export_is_dropped_without_a_strike` | `the hook that lost the session is not a fault: BudgetExceeded { cap_seconds: 6 }` | `f1-queue-share-full-cap` red |
| GLM F2 containment | `spawned_swap_20_9::an_artifact_that_resolves_outside_its_directory_is_refused_typed` (symlinked directory through `admit_spawned`; absolute and `..` through `manifest_launch_in`) | `the symlink leaves the manifest's directory: Admitted { pid: 1 … }` | `f2-containment-removed` red |
| GLM F3, F4, F5b, F6 | dispositioned, no code (design rulings needed) | — | `deferred-work.md` §20-9 L1 review corrections |
| GLM F5a | `MAOS_ARCHIVE_DIR` registered in `env_contract.rs`; `check-env-contract` green | — | — |

GLM F4 (second half): a witness for "a second upgrade inside the adoption window" was written, observed to complete both swaps and then end the session `swap_reverted_after_retire` (the D-20-9-11 residual), and removed; the row says so.

## AC1-AC3 failing-before (baseline-compatible executable witness)

`base-daemon-triggers.sh` runs a real `maos run` of the lifecycle-probe component under `MAOS_IDLE_FAST`/`MAOS_SCHEDULE_FAST` with a `[[schedule]]` for 25 s and counts the daemon's own TL rows. At the pre-20-9 base `fc7373f0` (scratch build `/tmp/base209-target`, deleted): `baseline-daemon-triggers.log` — **`log.recall: 0`, `schedule.fire:tick: 0`** (no export ever ran, no schedule carried). Final tree: `final-daemon-triggers.log` — `log.recall: 379` (idle 1, consolidate 318, schedule 60 by page size), `schedule.fire:tick: 60`. This is the executable failing-before for AC2/AC3 and for "no path calls a lifecycle export" in AC1.3 (reworded, D-20-9-20). The runner/wire half of AC1 has no baseline-compatible executable witness (the lifecycle wire does not exist at the base): its failing-before stays by code (`../failing-before-head-code.log`) plus the AC1 plant.

## Pins (exact, after `cargo fmt --all`, `kloc-check --json`)

| Item | Before | Now | Ceiling |
|---|---:|---:|---:|
| maos-bin | 37350 | 37633 (+283) | 38422 |
| maos-wasm-host | 3103 | 3137 (+34) | 3348 |
| xtask | 44936 | 44936 | 44980 |
| aggregate | 200880 | 201197 (hardfail 201198) | 202209 |

Kernel 21042 / 26841 unchanged.

## Not covered (unchanged limits)

Hosted-only legs, macOS/Windows compile, AC6, L2 (AC7-AC9). The TS template/example legs.
