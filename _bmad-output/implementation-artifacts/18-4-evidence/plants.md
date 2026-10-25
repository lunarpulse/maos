# 18-4 proven-red plants (dev pass, 2026-10-25, anthropic/claude-opus-5-5)

Protocol: in place in `spirits/butler/src/lib.rs` (a file carrying uncommitted story edits), so the file was copied aside
(`cp spirits/butler/src/lib.rs /tmp/p184/lib.rs.orig`) before every plant and restored with
`cp /tmp/p184/lib.rs.orig spirits/butler/src/lib.rs && cmp … && touch …`. One target dir (`/mnt/build/cargo`),
`env -u CI -u GITHUB_ACTIONS`. Five plants, one per AC. Wall-clock = touch → red observed / restore → green observed.

| AC | Plant | Command | Red observed | Restore → green | Wall-clock |
|---|---|---|---|---|---|
| AC1 | `assess`: `Some(_entropy) => (conflict_proxy, Some(window))` (conflict proxy while the ledger informs) | `cargo nextest run --locked -p maos-bin --test butler_scene_18_4 -E 'test(the_steady_twin_runs_21_days_without_a_halt)'` | FAIL `butler_scene_18_4.rs:232` "the steady director never halts": `{"event":"halt","render":"halted on belief_variance","day":1,…}` | Summary 1 passed | red 5 s / green 5 s |
| AC2 | `render_if_new_day` reads `SystemTime::now()` instead of the clock | `… --test butler_scene_18_4 -E 'test(exit_line_4_halts_upstream_on_the_variance_led_rule)'` | FAIL `:197` digests `left: [1792966871066207902]` (one wall-clock digest) vs `right:` the 14 scenario instants | Summary 1 passed | red 7 s / green 6 s |
| AC3 | `ensure_restored`: `let markdown: Option<String> = None` (skip the restore) | `… --test butler_scene_18_4 -E 'test(a_restart_resumes_from_the_principal_namespace_and_forget_erases_it)'` | FAIL `:279` "the resumed scene halts on the uninterrupted run's day" `left: Some(11) right: Some(14)` (cold start on the conflict proxy; a first run of this plant also showed `records: 0` vs 17) | Summary 6 passed | red 6 s / green 8 s |
| AC4 | `handle_option_pick`: `let action = OptionAction::ALL[pick - 1]` (dispatch by position) | `cargo nextest run --locked -p butler --test acceptance_18_4 -E 'test(a_pick_runs_the_action_the_option_declares_not_its_position)'` | FAIL `acceptance_18_4.rs:148` `left: Reminder right: LinearNote` (no Linear note on pick 2) | Summary 4 passed | red 2 s / green 1 s |
| AC5 | `anomaly_feed` filters intent `"PLANT-AC5:"` (stops reading `anomaly.flagged:`) | `cargo nextest run --locked -p butler --test digest_production_18_1 -E 'test(the_digest_lists_seeded_anomalies_above_the_floor)'` | FAIL `digest_production_18_1.rs:222` `left: 0 right: 1` (`"anomalies":[]`) | Summary 6 passed | red 2 s / green 2 s |

CI plant (AC1, line-4 steady twin): NOT YET PREPARED — `18-4-evidence/ci/plant.diff` + sha256 remain (see the dev
record's open items).
