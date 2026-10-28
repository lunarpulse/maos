# 19-1 proven-red plants

## Corrected-tree applicability (2026-10-28)

No plant was applied and no cargo command was run during this correction. All nine diffs across the 19-1 and 19-7 directories pass forward `git apply --check`; all fail `git apply --check -R` on the unplanted tree. Before correction, AC1 heading and AC4b halt failed forward checks; the other four 19-1 diffs passed. The fourth-host diff also passed reverse checks because its old context matched repeated existing workers; its context was expanded minimally to disambiguate it, without changing the defect.

| Re-cut diff | Current planted site / asserting control | sha256 | Corrected-tree red / green |
|---|---|---|---|
| `ac1-heading-plant.diff` | `spirits/orchestrator/src/epic.rs:125-127`: retain `unindent`, but fold non-first non-Story level-3 headings into the preceding section. Controls: `crates/maos-bin/tests/epic_dispatch_19_1.rs:470,491-494`; `spirits/orchestrator/tests/epic_split.rs:92,103-119` | `39eed7d2ad68a8c2eff1ab80617c28da2e0d9f79100e0c62c9700aa3c26ba77e` | Red rc=100, wall 71 s: `cargo nextest run --locked --color never --no-fail-fast -p maos-bin -p orchestrator -E 'binary(epic_dispatch_19_1) \| binary(epic_split)'` — 24 run, 20 passed, 4 failed: `orchestrator::epic_split only_an_exact_story_heading_opens_a_section` (panicked `epic_split.rs:157`), `a_section_not_headed_story_is_refused_naming_both_counts` (`epic_split.rs:104`, `left == right`), `a_heading_indented_one_to_three_spaces_is_still_a_heading`, and `maos-bin::epic_dispatch_19_1 a_heading_less_section_is_refused_before_any_emit`. Restored byte-identical: `spirits/orchestrator/src/epic.rs` sha256 `178a01f80cba8f91c90b3d768df43a5684f7ee3c99effb09643ef526103e976c`; green in the combined re-run below |
| `ac2-fourth-host-plant.diff` | append a fourth worker after `spirits/topologies/j1-founder-loop-epic.toml:31`; same gate defect, expanded context only. Asserting gate: `xtask/src/check_j1_loopback_delegation.rs:280-289` | `57062aa52d23df45427b8cf5e63ac3c5e20527f4835b17c1da697e559915d45d` | Red rc=1, wall 16 s: `cargo run --locked -q -p xtask -- check-j1-loopback-delegation --json` reported `passed:false` with exactly one finding, `frame-borne-route-intact`: "spirits/topologies/j1-founder-loop-epic.toml must declare exactly 3 `host = \"developer-remote-host\"` entries (found 4 `host` keys) — one frame-borne Worker per `### Story` section"; the leg's frame-borne checks: 9. Restored byte-identical: topology sha256 `94dd55c9b3f90315f72c83efa1cd017b726d33e915445fe37ec7618f21de96c3`; gate rc=0 in the green re-run below |
| `ac4b-halt-check-plant.diff` | `crates/maos-bin/src/orchestrator_root.rs:215`: replace the pending-halt operand with `false`, retaining the unloaded operand; caller `crates/maos-bin/src/main.rs:4868`. The halt remains written but is not consulted. Control: `crates/maos-bin/tests/epic_dispatch_19_1.rs:324,341`, expected one `dispatch_halted` missing (0 ≠ 1) | `67c384b25c7421ade601fc675e335a0cb3b9df056bbcd3707bb5d8c6a6456d3e` | Hosted T3 observation pending; not run here |

Re-run each local target planted (red), restore byte-identically, then re-run green, with `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`:

- AC1: `cargo nextest run --locked --no-fail-fast -p maos-bin --test epic_dispatch_19_1`, then `cargo nextest run --locked --no-fail-fast -p orchestrator --test epic_split`.
- AC2 fourth host: `cargo run --locked -q -p xtask -- check-j1-loopback-delegation --json`.
- AC4b (hosted T3): `cargo test --color never -p maos-bin --test epic_dispatch_19_1 --test epic_args_19_1 -- --test-threads=1`.

## Corrected-tree plant pass 2026-10-28

The orchestrator re-ran the local plants on the corrected tree — uncommitted corrections on top of `6f872383`, model `anthropic/claude-opus-5-5`, one cargo process, every run under `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Each plant was applied in place: copy aside → `git apply <diff>` → run the target (red) → copy back → `cmp` → `touch` → green re-run; restore was proven by sha256 equality of the file before planting and after restore, not `git diff --quiet 6f872383`. Wall is plant applied → red observed, including the incremental compile.

- `ac2-unenrolled-plant.diff` (re-run although unchanged): red rc=1, wall 1 s, same gate: exactly one finding, `completion-vectors-enrolled` (".github/workflows/discipline.yml's `check-j1-loopback-delegation` JOB does not invoke `cargo test -p maos-bin --test xyz_19_1` …"); completion-vectors-enrolled checks 11 (10 when green). Restored by deletion; absence checked.
- `ac4b-halt-check-plant.diff` was not run here; hosted T3 observation still pending.
- Green after restore: `cargo nextest run --locked --color never --no-fail-fast -p maos-bin -p orchestrator -p maos-a2a -E 'binary(epic_dispatch_19_1) | binary(epic_split) | binary(repin_door_19_7) | binary(restart_invalidates_pin_nfr_rel_6)'` — 56 run, 56 passed (the T3 vectors return early locally: no rootless podman); gate rc=0.

## Historical observations @6f872383


Plant pass 2026-10-28, `anthropic/claude-opus-5-5`, on candidate `6f872383` (clean tree before and after). This host has
one cargo process, and every run used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. A scratch
runner outside the repo did each plant in place, one at a time:

1. Copy the file aside.
2. `git apply <row>-plant.diff` (the diffs in this directory).
3. Run the targeted test or gate (red).
4. Copy the file back, `cmp` it against the saved copy, and check `git diff --quiet 6f872383 -- <file>`.
5. `touch` the file, because the copy-back restores the old mtime and cargo would otherwise keep the planted artifact.
6. Run the target again (green).

The new-file plant was restored by deletion, and its absence was checked. Wall-clock is plant → red observed (including
the incremental rebuild), then restore → green observed. All six local runs were red as designed, and all came back green
on the byte-identical tree.

Two of the six rows (AC4a and AC4b) need the T3 substrate. Rootless podman fails `cannot clone` on this host, so
`require_t3` skips those tests here: the test returns early and nextest reports it as passed. They are recorded below as
hosted plants for `ci/19-1-verify`. The AC4a diff also has a local partial witness.

| AC | File · change (at `6f872383`) | Command | Red observed | Restore | Wall |
|---|---|---|---|---|---|
| AC1 deterministic split | `spirits/orchestrator/src/epic.rs:118`: a level-3 heading opens a section only if it is the first or a `### Story` heading, so the planted `### Notes` folds into section 1 instead of being refused (`ac1-heading-plant.diff`) | `cargo nextest run --locked --no-fail-fast -p maos-bin --test epic_dispatch_19_1`, then `… -p orchestrator --test epic_split` | `a_heading_less_section_is_refused_before_any_emit` FAIL at `epic_dispatch_19_1.rs:344` ("the refusal names the heading and both counts"). stderr carries only `the epic splits into 2 \`### Story\` sections vs 3 host-bearing entries`, with no `is headed \`### Notes: …\``. Result: 7 run, 6 passed, 1 failed (the T3 tests return early here). `epic_split`: `a_section_not_headed_story_is_refused_naming_both_counts` (`:104`) and `only_an_exact_story_heading_opens_a_section` (`:157`) FAIL. Result: 9 run, 7 passed, 2 failed | `cmp` OK; `git diff` vs `6f872383` clean; `touch`; green 7/7 and 9/9 | red 33.2 s + 7.8 s; green 9.7 s + 3.4 s |
| AC2 gate, epic topology | `spirits/topologies/j1-founder-loop-epic.toml:31`: a fourth `[[topology.spirits]]` worker entry with `host = "developer-remote-host"` appended (`ac2-fourth-host-plant.diff`) | `cargo run --locked -q -p xtask -- check-j1-loopback-delegation --json` | exit 1, `"passed":false`, and exactly one finding: `{"check":"frame-borne-route-intact","detail":"spirits/topologies/j1-founder-loop-epic.toml must declare exactly 3 \`host = \"developer-remote-host\"\` entries (found 4 \`host\` keys) — one frame-borne Worker per \`### Story\` section"}` | `cmp` OK; `git diff` clean; `touch`; gate exit 0, `"passed":true`, 0 findings | red 39.5 s (includes the xtask build); green 0.8 s |
| AC2 gate, enrolment | NEW `crates/maos-bin/tests/xyz_19_1.rs` (`#[test] fn never_run_in_ci() {}`), not enrolled in the job (`ac2-unenrolled-plant.diff`) | same gate | exit 1, and exactly one finding: `{"check":"completion-vectors-enrolled","detail":".github/workflows/discipline.yml's \`check-j1-loopback-delegation\` JOB does not invoke \`cargo test -p maos-bin --test xyz_19_1\` …"}`. The leg's check count rose 10 → 11, which shows the new file was derived | file removed; absence checked; gate exit 0, 0 findings | red 0.4 s; green 0.3 s |
| AC4a FR21 follow-up, local partial witness (the full observable is hosted, below) | `spirits/orchestrator/src/lib.rs:689`: `followup_dispatch_remote` builds its payload with `prior_distillate_ref: None` (`let _ = prior;`) (`ac4a-followup-no-ref-plant.diff`) | `cargo nextest run --locked --no-fail-fast -p orchestrator --test followup` | `remote_followup_carries_the_prior_distillate_hosts_and_a_granter_bound_envelope` FAIL at `followup.rs:53` (`left: None` / `right: Some(PriorDistillateRef { … distillation_depth: 1 … })`). `remote_followup_equals_the_composed_remote_frame` FAIL at `:113`. Result: 6 run, 4 passed, 2 failed | `cmp` OK; `git diff` clean; `touch`; green 6/6 | red 10.3 s; green 1.9 s |
| AC5 class gate (miss twin) | `crates/maos-bench/src/halt_eval.rs:137`: the `orchestrator` row's `halt_recall_overall` floor `NFR_TEST_4_RECALL_FLOOR` → `0.60`, so the miss twin's 0.667 no longer misses (`ac5-recall-floor-plant.diff`) | `cargo nextest run --locked --no-fail-fast -p maos-bin --test eval_halt_orchestrator` | `the_miss_twin_exits_1_naming_the_missed_recall_floor` FAIL at `eval_halt_orchestrator.rs:162`: the miss twin exits 0 (`left: Some(0)` / `right: Some(1)`). `the_orchestrator_class_replays_its_seed_and_holds_every_floor` and `the_class_gate_runs_from_a_cwd_other_than_the_workspace_root` FAIL at `:121` (`floors.halt_recall_overall` `0.6` ≠ `0.7`). Result: 6 run, 3 passed, 3 failed | `cmp` OK; `git diff` clean; `touch`; green 6/6 | red 8.5 s; green 9.7 s |

Kernel: no plant touched `crates/maos-kernel-core`.

## Hosted plants (T3; `ci/19-1-verify`, not run here)

The run happens via the branch-only workflow `ci-19-1-verify.yml`, to be committed beside plants.md as
`ci/ci-19-1-verify.yml` and copied to `.github/workflows/` on `ci/19-1-verify` only, never merged; it mirrors
`check-j1-loopback-delegation`'s T3 setup and runs an unplanted control first. Both diffs pass `git apply --check`
against the corrected tree (AC4b applies only on top of the correction-round commit). Record the run id here once it
has run.

| AC | Diff (sha256) | File · change | Expected red |
|---|---|---|---|
| AC4a FR21 (`proof-contract.md:28` control a) | `ac4a-followup-no-ref-plant.diff` (`525d76d742c870165c992f0585ab743fc0269ecd8e3897f5f0134780be3634e8`) | `spirits/orchestrator/src/lib.rs:689`: the remote follow-up planted with `prior_distillate_ref: None`. This is the same diff as the local partial witness above | `epic_dispatches_each_section_to_its_worker_with_distillate_fed_followups` FAIL at `epic_dispatch_19_1.rs:206-209` ("the epic run must exit 0; stderr:") with stderr `delegation emit/route failed: orchestrator dispatch references raw worker output not a distillate` and `EOrchestratorDispatchRawOutput` |
| AC4b pending halt (D-19-1-09 A; control b) | `ac4b-halt-check-plant.diff` (`67c384b25c7421ade601fc675e335a0cb3b9df056bbcd3707bb5d8c6a6456d3e`) | `crates/maos-bin/src/orchestrator_root.rs:215`: `(unloaded \|\| !halt_ids.is_empty())` → `(unloaded \|\| false)`; caller `crates/maos-bin/src/main.rs:4868`. The planted halt (`MAOS_TEST_ONLY_HALT_BEFORE_SECTION=2`) is still written but no longer consulted | `a_pending_halt_before_section_two_stops_dispatch` FAIL at `epic_dispatch_19_1.rs:341` ("one dispatch_halted event", left 0 / right 1) |

## Notes

- Partway through the run, another process emptied part of the shared target dir `/mnt/build/cargo`. One invocation of
  the 19-7 run in the same session failed to compile (`libhashbrown-….rmeta: No such file`, `E0463`). It was rebuilt
  and re-run clean; see `19-7-…-evidence/plants.md`. None of the 19-1 reds above is a build error: each is the test or
  gate assertion quoted.
- The extra local `epic_split` run under the AC1 plant is the same plant observed in the orchestrator unit vectors, not
  a second plant. The plant count per `proof-contract.md:28` is AC1 ×1, AC2 ×2 (the two independent gate legs named in
  AC2.4), AC4 ×2 (named in AC4.3: (a) hosted, with a local partial witness; (b) hosted) and AC5 ×1.
