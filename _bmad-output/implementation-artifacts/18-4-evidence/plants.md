# 18-4 proven-red plants

## Dev pass (2026-10-25, anthropic/claude-opus-5-5)

Protocol: in place in `spirits/butler/src/lib.rs` (a file carrying uncommitted story edits), so the file was copied aside
(`cp spirits/butler/src/lib.rs /tmp/p184/lib.rs.orig`) before every plant and restored with
`cp /tmp/p184/lib.rs.orig spirits/butler/src/lib.rs && cmp … && touch …`. One target dir (`/mnt/build/cargo`),
`env -u CI -u GITHUB_ACTIONS`. Five plants, one per AC. Wall-clock = touch → red observed / restore → green observed.
Failure lines are re-anchored to the committed-plus-review tree (review TIA-6: the dev record cited pre-`cargo fmt`
lines); the dev-pass outputs are unchanged.

| AC | Plant | Command | Red observed | Restore → green | Wall-clock |
|---|---|---|---|---|---|
| AC1 | `assess`: keep `conflict_proxy` while the window informs (`Some(window)` with the proxy value) | `cargo nextest run --locked -p maos-bin --test butler_scene_18_4 -E 'test(the_steady_twin_runs_21_days_without_a_halt)'` | FAIL `butler_scene_18_4.rs:311` "the steady director never halts": `{"event":"halt","render":"halted on belief_variance","day":1,…}` | Summary 1 passed | red 5 s / green 5 s |
| AC2 | `render_if_new_day` reads `SystemTime::now()` instead of the clock | `… --test butler_scene_18_4 -E 'test(exit_line_4_halts_upstream_on_the_variance_led_rule)'` | FAIL `:242` digests `left: [1792966871066207902]` (one wall-clock digest) vs `right:` the 14 scenario instants | Summary 1 passed | red 7 s / green 6 s |
| AC3 | skip the restore (`let markdown: Option<String> = None`) | `… --test butler_scene_18_4 -E 'test(a_restart_resumes_from_the_principal_namespace_and_forget_erases_it)'` | FAIL `:369` "the resumed scene halts on the uninterrupted run's day" `left: Some(11) right: Some(14)` | Summary 6 passed | red 6 s / green 8 s |
| AC4 | `handle_option_pick`: `let action = OptionAction::ALL[pick - 1]` (dispatch by position) | `cargo nextest run --locked -p butler --test acceptance_18_4 -E 'test(a_pick_runs_the_action_the_option_declares_not_its_position)'` | FAIL `acceptance_18_4.rs:165` `left: Reminder right: LinearNote` (no Linear note on pick 2) | Summary 4 passed | red 2 s / green 1 s |
| AC5 | the anomaly reader (dev: `anomaly_feed`; review: `anomaly_items`) filters intent `"PLANT-AC5:"` | `cargo nextest run --locked -p butler --test digest_production_18_1 -E 'test(the_digest_lists_seeded_anomalies_above_the_floor)'` | FAIL `digest_production_18_1.rs:261` `left: 0 right: 1` (`"anomalies":[]`) | Summary 6 passed | red 2 s / green 2 s |

## Review patch pass (2026-10-26, anthropic/claude-opus-5-5; rulings D-18-4-R1…R5)

Same protocol: originals copied to `/tmp/p184/{butler_root.rs,lib.rs,acceptance.rs}.orig`, each plant applied in
place with the editor, the one test run, then `cp … && cmp … && touch …` and the test re-run green. Commands are
`env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test butler_scene_18_4 -E 'test(<name>)'`.

| Control | Plant | Test | Red observed | Restore → green | Wall-clock |
|---|---|---|---|---|---|
| TIA-1/BH-2 — the root fixes the pid | `butler_root.rs` `acceptance_binder`: `PidBoundPrincipalPort::new(memory.clone(), pid.max(9))` (the port ignores the root's resolved pid) | `the_root_fuses_the_principal_port_and_another_pids_record_is_unreachable` | FAIL `butler_scene_18_4.rs:633` `left: 0 right: 8` (pid 1 holds nothing — the Butler wrote over the second authorised pid's record) | Summary 1 passed | red 3 s / green 4 s |
| EC-1/AA-1 — the principal namespace is the record of truth | `lib.rs` `update_record`: `let mut stored = ledger.clone();` (write back the in-memory record) | `a_forget_between_decisions_resurrects_nothing` | FAIL `:702` `left: 9 right: 1` (the forgotten 8 records written back) | Summary 1 passed | red 4 s / green 5 s |
| R2 — 5-decision minimum | `acceptance.rs`: `MIN_WINDOW_OFFERS = 7` | `a_resume_after_the_halt_day_tunes_on_the_window` | FAIL `:465` `left: "conflict_proxy" right: "acceptance_window"` | Summary 1 passed | red 6 s / green 6 s |
| R5 (BH-1) — scene digest reads the day's pass | `lib.rs` `digest_value` scene arm: `since_ns: Some(since_ns.min(0))` (every frame of the boot, not the day's pass) | `exit_line_4_halts_upstream_on_the_variance_led_rule` | FAIL `:251` `left: 2 right: 1` (day 2's digest counts day 1's evaluation) | Summary 1 passed | red 6 s / green 5 s |
| R5 (label) | `render_at` passes `None` (wall-clock window in the scene) | same | FAIL `:244` (`frames` is `last_24h`, not `scene_pass`) | Summary 1 passed | red 7 s / green 6 s |

## CI plant (AC1, line-4 steady twin) — PREPARED and simulated locally

- `ci/plant.diff` regenerated at the review pass against the new `assess` arm (review EC-2 moved the line):
  sha256 `bfb733616c4c24d615bc15bb1440f2cf993c635a48722c7f8dd006a3cc964dad` (`ci/plant.diff.sha256`; the dev
  pass's `3e2be7e2…` no longer applied). `git apply --check` passes on the review tree (offset only).
- `ci/ci-18-4-verify.yml` (modelled on `18-3-evidence/ci/ci-18-3-verify.yml`): sha check, apply the plant, build
  `maos`, control = exit line 4 green, then require the steady twin to halt (the twin step's condition fails).
- Local simulation 2026-10-26 (`/tmp/p184/sim.sh`, the workflow's commands against `/mnt/build/cargo/debug/maos`):
  sha OK, plant applied, build OK; **control** rc=0, `butler inference seam wired`, halt day 1 at rate 1.0, line-4
  checks GREEN; **twin** rc=0 with `{"event":"halt",…,"day":1,"derived_from":"acceptance_window","belief_variance":0.8,…}`
  → the twin step reds (PROVEN-RED). Restored by `cp /tmp/p184/lib.rs.sim-orig … && cmp && touch`. Wall-clock 19 s.
