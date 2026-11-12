# 20-5 landing 2 — ruling alignment (Dev205L2, 2026-11-12, base `6bb4f339`, uncommitted)

Operator ruling 2026-11-12 (`ga` boundary, `rc` = b, `kb` keep). Kernel-free; no tag, signing, dispatch or version bump.

## Failing-before (HEAD code, new tests)

`failing-before-head-code.log`: the five source files (`check_release_precondition.rs`, `check_rc_readiness.rs`, `main.rs`,
`stability_matrix.rs`, `release.yml`) restored to `git show HEAD:` bytes with the new/updated tests in place:
**45 run, 16 failed** (all `rc_readiness_20_5` fixtures: no sixth leg / no waiver; `release_precondition_perf_20_5`
PASS/ABSENT vectors: `--perf-verdicts` unknown; `release_gates_20_5r1` perf step; `release_precondition_t3_20_5` rc|ga arm pin).
The crash/foreign-SHA vector and the 404 vector pass at HEAD only because the flag is unknown (a clap refusal), which the
plants below remove. The in-src `lts_clock` test is not a HEAD witness (its file was reverted with the source); its wording
witness is plant p10. New sources restored byte-identical (`cmp`) and touched.

## Plants (`plants/`, `plants.sh`; each: sed on a live file, run, restore by `cp` + `cmp` + `touch`)

| plant | mutation | red witness |
|---|---|---|
| p1 | ABSENT accepted without `--perf-absent-ok` | `absent_is_accepted_only_under_the_absent_flag_and_never_with_a_fail` |
| p2 | no SHA match on the perf report | `a_crash_a_foreign_sha_or_an_empty_report_is_never_absent` |
| p3 | waiver for any version (drop `is_rc_version`) | `the_20_7_waiver_is_in_progress_only_rc_only_and_prints_waived` |
| p4 | waiver at any status | same |
| p5 | waiver without the perf label (tied leg removed) | `the_perf_label_leg_needs_the_notes_and_the_exact_line` |
| p6 | environment WARN without the notes line | `an_unprovisioned_release_environment_is_a_labelled_warn`, `the_provisioning_leg_needs_every_row_present` |
| p7 | label matched by `contains` not an exact line | `the_perf_label_leg_needs_the_notes_and_the_exact_line` |
| p8 | GA also passes `--perf-absent-ok` | `perf_verdicts_are_judged_for_rc_and_ga_and_only_a_candidate_accepts_absent` |
| p9 | any `gh` failure read as an empty ledger | `an_absent_perf_ledger_branch_is_an_empty_ledger_and_any_other_failure_refuses` |
| p10 | placeholder reverted to "Holds 1–2 clear" | in-src `lts_clock_reads_the_committed_record_and_never_git` |

## Final

- `final-focused.log`: 124 passed, 0 failed (rc_readiness_20_5, release_gates_20_5r1, release_precondition_perf_20_5,
  release_precondition_t3_20_5, check_release_precondition_gate, release_workflow_15_4, d11_xtask_ceiling_ratchet,
  recovery_lane_ceiling_rule, kernel_pin_content_hash_16_0, gate_honesty_20_3a, pentest_gate_tests,
  decision_adrs_and_provisioning). In-src `stability_matrix` / `gate_common` / `check_export_control`: 54 passed.
- `gates.log`: check-kernel-baseline, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands,
  check-decision-register, check-env-contract, check-abi-ratification, stability-matrix --check, error-catalog-check,
  check-manifest-schema-version, gen-abi-docs --check (no `--json` flag), check-breaking-md, check-workspace-count,
  check-fkcs, kloc-check, check-export-control: exit 0; `invariant-lock --changed-files` exit 0 (no regression).
  (The log also shows three of my own mistyped invocations — a nonexistent gate name and `--json` on a gate without it — exit 2; not gate failures.)
- Clippy (`cargo clippy -p xtask --all-targets --locked`): one warning in my code (`check_rc_readiness.rs` method chain)
  fixed; none remain in the touched files; the other xtask warnings (e.g. `stability_matrix.rs:280`, `gate_common.rs`) are pre-existing.
- Pins: xtask 45355 → **45461** (+106, orchestrator-adjudicated draw inside ceiling 45527); aggregate 203240, hardfail
  **203241**; kernel 27103 / 21256 unchanged. `repin.py --check`: PINS IN SYNC (`repin-check.log`).
- Real tree `check-rc-readiness`: red as expected (version `0.1.0-alpha.1`; nine `20-*` keys open, 20-7 among them and not waived because there is no notes file).
