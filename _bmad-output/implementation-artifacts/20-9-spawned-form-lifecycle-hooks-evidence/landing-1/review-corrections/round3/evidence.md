# 20-9 L1 correction round 3 — evidence (Dev209L1d)

Reviews of `8c669a21` (the corrections): `../../review/gpt-review-r2.md` (2 P1, 3 P2) and `../../review/security-r2.md` (GLM-5.3: 3 Medium, 2 Low, 2 Info), persisted verbatim.

## Method

- **Red-before:** a scratch `git archive c8e5d035` (the corrections, outside the repo, own target dir `/tmp/base209r3-target`, never the shared one; deleted afterwards) with ONLY the new/changed tests and `support/spawned_world_20_9.rs` (its log now binds the legal-hold authority, for the forget vectors) copied in; fixtures rebuilt there. One run, raw log `red/red-c8e5d035.log`: **8 selected tests, 8 failed**, each for the intended reason.
- **Plants:** in the final tree one mutation per fix (`../plants.sh`, exactly-once replace, `PLANT_DIR=round3/plants`, `@@` as the old/new separator), the proving test, then copy-back + `cmp` + `touch`. 10 plants, raw logs `plants/r3-*.log`, each ending `restored … cmp clean, touched`, none `DID NOT APPLY`.
- **Final tree:** `final-focused.log`, `workspace-nextest.log`, `gate-*.log`, `clippy-ratchet.log`, `verb-table-no-default.log`, `check-no-default.log`, `fmt-check.log` (this directory).

## Witness table

| Finding | Witness (final tree) | Red at `c8e5d035` | Plant → result |
|---|---|---|---|
| GPT r2 #1 snapshot fence owner | `spawned_swap_20_9::a_snapshot_answered_after_the_dispatcher_gave_up_but_before_the_link_does_fences_nothing` (predecessor SCB budget set to 2 s while the link cap stays 30 s; guest `sleep-in snapshot 3`: the dispatcher refuses at 2 s, the reply lands at about 3 s) | `no report within 15 s for lifecycle-probe` (predecessor fenced for good) | `r3-1-fence-owner` (fence without the attempt gate) red |
| GPT r2 #2 archive before readiness | in process `spawned_swap_20_9::a_cross_major_component_that_never_starts_leaves_the_predecessors_archive_current`; real door `spawned_door_20_9::a_cross_major_successor_that_never_starts_is_reverted_not_left_absent` | `MigratorMissing { archived_version: "2.0.0", successor_version: "1.0.0" }`; door: `cold_swap_failed: successor refused … restore refused` | `r3-2-archive-before-readiness` red (both) |
| GPT r2 #3, GLM R2 busy ≠ overrun | `spawned_lifecycle_20_9::a_telemetry_event_behind_a_long_turn_is_dropped_without_quarantine_or_a_late_run` (real pump, 6 s frame turn, 30 s component; no revoke row, `delivered == 0`, no telemetry run after the turn) | `a subscriber that merely waited behind a turn keeps its grant` | `r3-3a-telemetry-queue-share` red; `r3-3b-busy-counted-as-clean` red (`delivered` would be 1) |
| GPT r2 #4, GLM R3 history | `hook_pump_17_4::three_slow_telemetry_deliveries_of_the_last_five_quarantine_whatever_runs_between` (real consolidation watchdog between slow/quick/slow/quick/slow telemetry; quarantine on the fifth) | `left: Some((1, true, "admitted")) right: … quarantined … in 3 of its last 5 deliveries` | `r3-4a-fired-resets-history` red; `r3-4b-consolidation-shares-history` red |
| GPT r2 #5, GLM R7 registration | `spawned_swap_20_9::a_failed_spawned_admission_withdraws_its_hook_registration` (no watchdog, no scalar write; the grant row is revoked `spawned-start-failed`) | `the grant row is revoked by the unwind, not left for a later event` | `r3-5-unwind-keeps-registration` red |
| GLM R1 consolidate pass | `spawned_lifecycle_20_9::one_slow_consolidating_subscriber_does_not_stall_the_others_passes` | `the fast subscriber consolidated 0 times in 2 s while the slow one slept` | `r3-6-consolidate-tick-joins-passes` red |
| GLM R1 forget notices | `spawned_lifecycle_20_9::forget_notices_reach_every_subscriber_side_by_side` | `the fast subscriber took its notice while the slow one held its own: left 0 right 1` | `r3-7-forget-notices-sequential` red |
| GLM R4 | the wall overrun's row names the wall actually applied (`cap_seconds = wall`); the policy question is a row | — | — |
| GLM R5, R6, TL-write lock, R3 pacing floor, teardown claim | dispositioned, no code | — | `deferred-work.md` §20-9 L1 correction round 3 |

Not witnessed: the 30 s consolidate/forget wall itself (a 30 s vector is too slow; the concurrency vectors above exercise the same `fire_walled` path with a 3-4 s hold).

## Pins (exact, after `cargo fmt --all`, `kloc-check --json`)

| Item | Before | Now | Ceiling |
|---|---:|---:|---:|
| maos-bin | 37633 | 37832 (+199) | 38422 |
| maos-wasm-host | 3137 | 3137 | 3348 |
| xtask | 44936 | 44936 | 44980 |
| aggregate (clean) | 201197 | 201396 (hardfail 201397) | 202209 |

The tree also holds the operator's untracked `spikes/` (not mine, not touched), so the unmodified aggregate reads higher and `(unknown:spikes)` is over its own budget: `gate-kloc-check.log` is red for that reason alone. Kernel 21042 / 26841 unchanged.
