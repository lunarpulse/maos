# 18-3 proven-red plants (dev pass)

Dev pass 2026-10-25, `anthropic/claude-opus-5-5`. One cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`,
`env -u CI -u GITHUB_ACTIONS`. Each plant: file copied aside, planted in place, focused `cargo nextest run --locked
--no-fail-fast`, copied back, `cmp` (`CMP_OK` every time), `touch`, then a green re-run. Diffs: `<ac>-plant.diff`.
Wall-clock: plant → red observed (incl. rebuild), restore → green observed.

| AC | File · change | Red test → assertion | Restore | Wall-clock |
|---|---|---|---|---|
| AC1 | `spirits/researcher/src/lib.rs` `survey_bound` — `complete` error → `return Ok(summarize(statement))` (the removed fallback) | `researcher_inference_18_3::empty_drifted_and_short_cassettes_exit_1_naming_the_cause` FAIL at `:228` (no `researcher on_idle pass refused`); `a_zero_config_live_pass_is_refused_unconfigured` FAIL at `:299`; `inference_seam_8_11::live_error_is_a_typed_refusal_never_a_fallback` FAIL at `:188` (`unwrap_err` on `Ok`). 10 run: 7 passed, 3 failed. Note: the empty cassette did NOT exit 0 — the root's consumed ≠ surveyed-claims guard (AC1.3 third red) refused it independently (`researcher consumed 0 completions for 3 surveyed claims`, exit 1); the red is on the typed-refusal text. The CI twin therefore asserts both `researcher on_idle pass refused` and `exhausted`. | copy-back + `cmp` + `touch`; 10/10 green | red ≈ 26 s, green 16 s |
| AC2 | `crates/maos-eval/src/distill_replay.rs` `metrics` — hedge kept iff `finding.hedges.contains(h)` (structural list) instead of the statement text | `distillate_five_metrics_floor::test_distillate_five_metrics_floor` FAIL at `:108` (`negative control rd13-hedge-drop (HedgeDrop) not detected`); `eval_distill_18_3` committed-cassette (`:78`) and record (`:206`) cases FAIL on the same message. 7 run: 4 passed, 3 failed | copy-back + `cmp` + `touch`; 7/7 green | red ≈ 6 s, green ≈ 5 s |
| AC3 | `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:582` `saturating_mul(800)` → `saturating_mul(1000)` (temporary kernel edit) | `hook_dispatch_paused_time_18_3::researcher_cap_warns_at_eighty_percent_under_paused_time` FAIL at `:132` (`a BudgetWarning frame is due`). 3 run: 2 passed, 1 failed. (First attempt hung to the 120 s nextest timeout because the test awaited the frame unboundedly; the wait was made bounded and the plant re-run.) | copy-back + `cmp` + `touch`; `git status crates/maos-kernel-core/src` clean; 8/8 green (both envelope tests) | red 2.3 s, green 4.3 s |
| AC4 | `crates/maos-bin/src/main.rs` Researcher boot arm — the scalar port built but not attached | `researcher_inference_18_3::exit_line_3_…` FAIL at `:186` (`no epistemic.halt row for methodology_conflict`). (First run asserted any kind-3 row and reddened later at the halt-event assertion: `planned_unload` rows are kind 3 too; the assertion was sharpened and the plant re-run.) | copy-back + `cmp` + `touch`; 7/7 green | red 7.4 s, green 23 s |
| AC5 | `spirits/researcher/src/lib.rs` `survey_bound` request — `Some(binding.provider_id.clone())` → `None` (adapter falls back to the router default the token does not name) | `researcher_inference_18_3::exit_line_3_replays_one_completion_per_seed_claim_through_the_declared_provider` FAIL at `:151` (keyless line 3 refused); `empty_drifted_and_short_cassettes_exit_1_naming_the_cause` FAIL at `:233`; `a_serving_root_mints_per_pass_and_binds_a_door_loaded_researcher` FAIL at `:330` (no `infer:` frame within the wait). 7 run: 4 passed, 3 failed | copy-back + `cmp` (`CMP_OK`) + `touch`; 7/7 green | red ≈ 55 s (45 s door-load wait), green ≈ 16 s test time |

CI plant (`ci/plant.diff`, sha256 `c59ac2191e4d889ae5483e8c1f947ecb27095d7401aed07d59ab35fb536213f2` in `ci/plant.diff.sha256`): the AC1 diff (restored
fallback), which reds the `epic-18-exit` line-3 twin through its `researcher on_idle pass refused` assertion. `git apply --check`: all six diffs (ac1–ac5, ci) apply at the dev tree.

**CI proven-red DONE 2026-10-26:** run `40237276846` (`ci/18-3-verify` @ `90eaa26c`, job `121301416295`, `ubuntu-24.04`
image 20261012.334.1; workflow `ci/ci-18-3-verify.yml`): `plant.diff: OK` (sha256 above); control — exit line 3 rc=0 with
`researcher inference seam wired (provider \`anthropic\`)` and `maos run: --once complete — exiting cleanly`; the planted
empty-cassette twin printed only `maos: CASSETTE EXHAUSTED — cassette replay: exhausted (0 entries consumed, no more
available)` — no `researcher on_idle pass refused: … exhausted` refusal — so `epic-18-exit`'s line-3 twin step reds under
the plant. Excerpt `ci/run-40237276846-excerpt.log`; branch deleted. **Landing run 1 `40237233599` @ `b96d123a`** reds:
`fr4_classifier_16_2` (doorbell scanner misread the `insert_frame_event_with_id` kind) and `check-test-budget`
(`researcher_inference_18_3` 37.341 s > 30 s) — both fixed in `b50ea844` (kind argument read; 47 s budget exception) — and
`cap_token_verify_p99_under_5us_hot_path` 5670 ns (shared-runner timing; owner `20-7-stable-runner-perf-gates`,
`deferred-work.md:1266`). **Landing run 2 `40251998973` @ `79f49709`:** `workspace-test-suite`, `epic-18-exit` (exit
lines 1–4 + both twins), `check-test-budget`, `check-serde-error-handling` green; the only reds are `onb-nfr2-timing`
(binary size, 18-4's) and `check-multi-tenant-loom` `root_shutdown_unload_16_3::ac3_operator_door_unload_stops_a_live_worker_once`
route-budget timing (passed in the same run's `workspace-test-suite`; owner 20-7).

## Review patch pass (2026-10-25, `anthropic/claude-opus-5-5`, rulings D-18-3-R1…R5)

Same protocol: file copied to `/tmp/plants183/`, planted in place with the edit tool, focused `cargo nextest run --locked
--no-fail-fast`, copied back, `cmp` OK, `touch`, green re-run. Wall-clock = plant run incl. incremental rebuild.
`ac2-plant.diff` and `ac4-plant.diff` no longer applied after the review moves and were regenerated at the review tree
(AC2: `keeps_hedge(&finding.statement, h)` → `finding.hedges.contains(h)` → floor test + `eval_distill_18_3` committed
and record cases red on `rd13-hedge-drop (HedgeDrop) not detected`, 9.3 s; AC4: Researcher boot-arm scalar port built
but not attached → `exit_line_3_…` red at `:204`, 15.5 s). All six diffs (ac1–ac5, ci) `git apply --check` OK; the CI
plant is unchanged (sha256 above), and now reds the twin's single `grep -qE 'researcher on_idle pass refused: .*exhausted'`.

| Finding | File · plant | Red test → assertion | Wall |
|---|---|---|---|
| R5 (TIA-7) | `hook_dispatch.rs` `fire_on_idle` gate `if false && …` (kernel, temporary) | `j_researcher_survey_18_3` `:453` — `left: Fired, right: HaltPending` ("a pending halt gates the pass") | 14.1 s |
| TIA-1/AA-4 | `researcher_root.rs` production `Semaphore::new(RESEARCHER_PARALLELISM + 4)` | `j_researcher_survey_18_3` `:401` — peak `12` ≠ `8` | 6.6 s |
| R2 | `ResearcherHost::new` seeds under `--live` (`(!live \|\| true)`) | `researcher_inference_18_3::a_live_pass_without_an_mcp_server_is_refused_typed_never_seeded` `:323` | 9.2 s |
| BH-1 | `build_spirit_obj` passes `ResearcherWiring { legacy_cassette: None, .. }` | `a_legacy_cassette_root_binds_a_door_loaded_researcher` `:355` (no door issuance within 45 s) | 56.2 s |
| TIA-2/AA-7 | `ResearcherInferenceBinding::per_pass` memoizes the first token | `the_root_binding_mints_once_per_pass_on_one_instance` `:571` — mints `1` ≠ `2` | 14.2 s |
| TIA-3 | `ac1-plant.diff` (fallback restored) | `the_legacy_cassette_arm_refuses_an_exhausted_cassette_typed` `:277` now reds (it stayed green at the dev pass) + `empty_drifted_and_short_…` `:247`; 9 run, 2 failed | 13.6 s |
| EC-8 | `bound_pass` zero-claim check `if false && …` | `inference_seam_18_3::a_zero_claim_bound_pass_refuses_typed_and_writes_no_scalar` `:251` (refusal is `Distillation`, not `NoClaims`; still no scalar — the request-first order holds) | 2.7 s |
| EC-9 | `hook_dispatch.rs:572` timeout `from_millis(cap * 990)` (kernel, temporary) | `hook_dispatch_paused_time_18_3::researcher_cap_exceeds_…` `:203` (BudgetExceeded just under the cap) | 4.8 s |
| R3 | `apply_judge` model check `if false && …` | `eval_distill_18_3::record_refuses_a_judge_reply_attributed_to_another_model_after_flushing` `:398` | 9.0 s |
| R3 | `AnthropicProvider::complete` ignores `options.model_id` | `anthropic::tests::request_model_id_overrides_the_configured_model` `:376` — `claude-3-haiku…` ≠ `claude-sonnet-5-5` | 2.0 s |
| R4 | live replay scores frozen verdicts (`survey(s, &router, !record)` + `apply_judge` record-only) | `a_live_record_replay_scores_the_recorded_judge_replies_not_the_frozen_verdicts` `:382` — exit `0` ≠ `1` | 8.0 s |
| R4 | unconsumed-entry check removed | `record_through_the_router_writes_a_live_cassette_and_sidecar_that_replay_green` `:348` — exit `0` ≠ `1` | 5.2 s |
| R4 | sidecar write skipped (with the check above) | both record tests red at `read_json` (no `distill-corpus-v0.live.verdicts.json`) | 5.0 s |
| TIA-4 | `UnfaithfulFinding` overall `detected: false && …` | floor test `:132` + `eval_distill_18_3` `:108` — `rd13-hedge-drop,rd15-planted-secret (UnfaithfulFinding) not detected` | 14.6 s |
| EC-6 | `keeps_hedge` → `statement.contains(&hedge)` | `hedges_match_whole_words_only` `:185` — `unlikely` matched `likely` | 3.3 s |
| BH-4 | `judge_verdict` back to `trim().trim_matches('`')` | `judge_verdicts_strip_non_letters_around_the_word` `:210` — `Faithful.` refused | 3.1 s |
| AA-2 | `stale_judge_prompts` filter `false && …` | `the_judge_config_and_the_manifest_pin_the_judge_prompt` `:167` — `[]` ≠ `["rd01: …"]` | 2.8 s |
| TIA-8 | nightly `MAOS_REPLAY_CASSETTE` → the committed `distill-corpus-v0.json` | `ci_shape_18_0` `:405` (the Researcher nightly pin) | 7.7 s |

Not planted (no hermetic witness): BH-3 per-call judge mint (TTL needs a clock; deferred → 18-4); the R2 boot refusal
(`McpClientImpl::new` only fails on an internal transport-map shape, unreachable from env); AA-5 consumed ≠ claims guard
(a mismatch is unreachable without the fallback — witnessed only by the AC1 plant run, where it caught the empty cassette).
