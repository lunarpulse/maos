# 18-1 proven-red plants

Dev pass 2026-10-25, `anthropic/claude-opus-5-5`, one cargo session, `CARGO_TARGET_DIR=/mnt/build/cargo`, local runs
`env -u CI -u GITHUB_ACTIONS`. Each plant was made in place on a file that already carried uncommitted 18-1 edits, so
the file was copied aside before the plant, copied back after it, and checked with `cmp` (`CMP_OK` every time).
Diffs: `ac1-plant.diff`, `ac3-plant.diff`, `ac6a-plant.diff`, `ac6b-plant.diff`, `ac7-plant.diff` in this directory.
The `ac1` diff was regenerated after `cargo fmt --all` and is also the CI plant (`ci/plant.diff`, sha256 in
`ci/plant.diff.sha256`). Wall-clock = touch → red observed → restore → green observed.

5 plants: one each for AC1, AC3 and AC7, and two for AC6. AC6 has two because each digest-shipping class needs its own
production witness to be falsified (`proof-contract.md:26` exception, named in AC6.4). AC4 is the existing
`check-model-currency` gate, so it has no plant.

| AC | File · change | Red test → assertion | Restore | Wall-clock |
|---|---|---|---|---|
| AC1 *(dev; superseded by the review split TIA-1a/1b below — this plant disabled BOTH verdict arms, so its red named only the refused-pass arm)* | `crates/maos-bin/src/main.rs` — `verdict.map_err(\|error\| format!("maos run: {error}"))?;` becomes `if let Err(error) = verdict { eprintln!("maos run: {error}"); }` (the log-only shape check comes back) | `butler_inference_18_1`: `a_completion_missing_options_exits_non_zero_naming_the_field` FAIL at `:151` ("a completion missing `options` must exit non-zero"). `cassette_exhaustion_exits_non_zero` (`:171`) and `strict_prompt_drift_exits_non_zero` (`:191`) also red. Result: 3/6 | copy-back + `cmp`; 6/6 green | red 9.7 s, green 6.8 s, total 16.4 s |
| AC3 | `main.rs` — the `--once` `butler_morning_digest…render_if_new_day()` call removed | `butler_digest_18_1`: `the_production_digest_meets_fr17_against_a_seeded_tl` FAIL at `:161` ("no morning_digest event on stdout (AC3)"). `fr58_fresh_home_init_then_run_is_zero_config` FAIL at `:236`. Result: 0/2 | copy-back + `cmp`; 2/2 green | red 7.5 s, green 9.9 s, total 17.4 s |
| AC6 (a) Butler | `main.rs` `ButlerMorningDigest::render_if_new_day` — `morning_digest(…, now_ns - 3_600_000_000_000, …)` (truncated window; the reported `now_ns` is unchanged) | `the_production_digest_meets_fr17_against_a_seeded_tl` FAIL at `:211`: open-halt equality `left: {}` vs `right: {3 TL halt frames}`. Result: 1/2 | copy-back + `cmp`; 2/2 green | red 7.4 s, green 14.5 s, total 21.9 s |
| AC6 (b) Digest *(dev; re-planted in review AA-4/TIA-7 below)* | `main.rs` `render_j3_digest_scene` — the two `evidence.source_log_ref = hex::encode(transparency_log.last_frame_id());` lines for summaries and receipts deleted, so the fixture's refs are kept | `journey_j3::j3_team_digest_within_30s_cites_only_real_tl_frames` FAIL at `:101` (exit 0 expected). The daemon refused first, `Error: "invalid source_log_ref: local:self-report"`: the I11 persist rejects the unresolved fixture ref before `team_digest` prints, so the witness goes red at its exit precondition, not at its ref-resolution loop. The two older j3 scenes also red. Result: 1/4 | copy-back + `cmp`, then `cargo build -p maos-bin -p maos-cli` (journey binary freshness); 4/4 green | red 7.6 s, green 8.3 s, total 15.8 s |
| AC7 | `crates/maos-kernel-core/src/inference/mod.rs` — the 3-line `if req.fallback_provider_ids.iter().any(\|id\| id != provider_id) { return Err(CapabilityDenied) }` removed | `cargo test -p maos-kernel-core --lib inference`: `inference::tests::fallback_not_named_by_the_token_is_refused_before_dispatch` FAILED at `mod.rs:663`. Result: 13/14, the same figure as the creation prototype | copy-back + `cmp`; 14/14 green | red 2.2 s, green 1.3 s, total 8.1 s |

CI: `epic-18-exit`'s twin step is the line-1 red on every run. If `ci/plant.diff` is applied, the twin exits 0 and the
step fails ("the planted violation exited 0"). The integrator runs that CI proven-red once. **Regenerated in review
(2026-10-25):** the verdict moved to `crates/maos-bin/src/butler_root.rs`, so `ci/plant.diff` is now the TIA-1b plant (the
refused-pass arm only; the twin's missing-`options` completion is refused by `parse_proposal` before the shape arm), sha256
`943be65d0bfbab1aa48ec86ccea357a0848a9062ac6bb7dac33d531f214204a7`. Simulated locally from the workflow text after the review
patches: line 1 rc=0 (seam line present); the twin rc=1 naming `options`.
**CI proven-red DONE 2026-10-25:** run `40183063919` (`ci/18-1-verify` @ `6cc6332e`, job `121138008130`, `ubuntu-24.04`
image 20261012.334.1; workflow `ci/ci-18-1-verify.yml`): `plant.diff: OK` (sha256 above); control — exit line 1 rc=0 with
`butler inference seam wired (provider \`anthropic\`)`; the planted twin logged `on_idle pass refused: malformed proposal …
missing field \`options\`` and still exited 0 (`--once complete — exiting cleanly`), i.e. `epic-18-exit`'s twin step reds
under the plant. Excerpt `ci/run-40183063919-excerpt.log`; branch deleted. **Landing run `40183021781` @ `08ae68ad`**
(13.23 job-h, 86.8 min, 182/189 success): `epic-18-exit` and `cargo-deny` green; reds = `cohort_daemon_smoke_13_5c::
composition_root_does_not_seed_manifest_scopes` (`:1050`, 18-1 added `src/butler_root.rs` unlisted — fixed by listing it,
carried with 18-8) in `workspace-test-suite`, `check-j1-loopback-delegation`, `check-reza-production-path`,
`check-multi-tenant-loom` (+ the three fan-in aggregates), and the 20-7-owned 5 s orphan vector
`root_shutdown_unload_16_3::cohort::ac4_v5…` (`:348`, "host B's hang Worker … still in /proc after 5s"). Workspace:
4,792 tests, 2 failed.

## Review pass (2026-10-25, `anthropic/claude-opus-5-5`)

One plant per new control from the 4-layer review (BH/EC/AA/TIA) and rulings D-18-1-R1…R6, made in place by a scripted runner: copy
aside → plant → focused run → copy back → `cmp` (`CMP_OK` every time) → `touch` (the copy-back restores the old mtime, and cargo then
keeps the planted artifact — first observed as a stale TIA-1b verdict; every `maos-bin` plant was then re-run on fresh artifacts and
those re-runs are the rows below; the spirit-only rows EC-6…R5 ran before the fix, each compiled fresh from its own plant, and their
files were touched once afterwards). Journey plants rebuild `maos-bin`/`maos-cli` first. Diffs: `review-plants/<row>.diff`.
Wall-clock = plant → red observed (→ restore); the restored tree is green in the review's focused runs (story §Review).

| Control | File · change | Red test → assertion | Restore | Wall-clock |
|---|---|---|---|---|
| BH-2 verdict outcome | `crates/maos-bin/src/butler_root.rs` — `&& !completed {` → `&& !completed && false {` (the hook outcome is ignored) | `an_incomplete_bound_pass_is_a_refusal` FAIL at `butler_inference_18_1.rs:335`. Result: 7 tests run: 6 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 5.3 s |
| TIA-1a shape arm only | `butler_root.rs` — `Some(Err(violation)) => Err(…)` → `Some(Err(_violation)) => Ok(())` (shape arm only). `butler_inference_18_1` stays 7/7 green — the split is real | `jb5_output_shape_violation` FAIL at `journey_butler.rs:177`. Result: 1 test run: 0 passed, 1 failed, 5 skipped / 7 tests run: 7 passed, 0 skipped | copy-back + `cmp` + `touch` | 10.5 s |
| TIA-1b refused-pass arm only | `butler_root.rs` — `take_pass_error` arm `.filter(\|_\| false)` (refused-pass arm only; = `ci/plant.diff`). JB-5 stays green | `cassette_exhaustion_exits_non_zero` FAIL at `butler_inference_18_1.rs:167`; `strict_prompt_drift_exits_non_zero` FAIL at `butler_inference_18_1.rs:187`; `a_completion_missing_options_exits_non_zero_naming_the_field` FAIL at `butler_inference_18_1.rs:147`. Result: 1 test run: 1 passed, 5 skipped / 7 tests run: 4 passed, 3 failed, 0 skipped | copy-back + `cmp` + `touch` | 9.5 s |
| EC-6 blanks | `spirits/butler/src/lib.rs` `parse_proposal` — `if value.trim().is_empty()` → `if false && …` | `blank_fields_are_refused` FAIL at `inference_seam_18_1.rs:335`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.9 s |
| BH-1 drift scale | `lib.rs` `proposal_prompt` — the scale sentence ("1.0 = fully aligned … lower = more drift") deleted | `the_prompt_states_the_drift_scale` FAIL at `inference_seam_18_1.rs:348`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.2 s |
| TIA-10 drift scalar | `lib.rs` `idle_pass` — `assessment.user_preference_drift = proposal.user_preference_drift;` → `let _ = …;` | `a_bound_no_conflict_pass_writes_the_models_drift_and_emits_nothing` FAIL at `inference_seam_18_1.rs:383`; `bound_pass_takes_the_proposal_from_the_completion` FAIL at `inference_seam_18_1.rs:221`. Result: 17 tests run: 15 passed, 2 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.2 s |
| R6 notification gate | `lib.rs` `idle_pass` — `(!assessment.conflicts.is_empty()).then(\|\| NotificationPayload {` → `Some(NotificationPayload {` | `a_bound_no_conflict_pass_writes_the_models_drift_and_emits_nothing` FAIL at `inference_seam_18_1.rs:381`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.3 s |
| BH-3 refused pass clears | `lib.rs` `idle_pass` — the refused branch no longer clears `last_notification` and the output channel | `a_refused_pass_clears_the_previous_notification` FAIL at `inference_seam_18_1.rs:409`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.2 s |
| TIA-5 MCP scenario | `lib.rs` `idle_pass` — the MCP scenario drops the fetched calendar/comms (`vec![]`) | `a_bound_butler_reasons_over_the_mcp_scenario` FAIL at `inference_seam_18_1.rs:456`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.2 s |
| R2 boot refusal | `lib.rs` `ButlerSeam::attach` — `if !registered.contains(&provider)` → `if false && …` | `the_seam_refuses_an_unregistered_declared_provider` FAIL at `inference_seam_18_1.rs:542`. Result: 17 tests run: 16 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.3 s |
| R1 successor/door binding | `crates/maos-bin/src/main.rs` `build_spirit_obj` Butler arm — passes `&butler::ButlerSeam { inference: None, ..seam.clone() }` (successor and door-loaded Butler unbound) | `ac4_root_e_a_faithful_successor_survives_the_hot_swap` FAIL at `one_daemon_one_door_16_1.rs:1057`; `the_serving_root_renders_at_start_mints_per_pass_and_survives_a_refused_pass` FAIL at `butler_daemon_18_1.rs:133`. Result: 2 tests run: 0 passed, 2 failed, 15 skipped | copy-back + `cmp` + `touch` | 52.9 s |
| R3 trust bar | `lib.rs` `yesterday_predicate_fire_rate` — `evaluations: evaluated.len() + 1` | `a_day_with_no_evaluation_reads_full_trust` FAIL at `digest_production_18_1.rs:170`; `the_trust_bar_is_yesterdays_predicate_fire_rate` FAIL at `digest_production_18_1.rs:130`. Result: 3 tests run: 1 passed, 2 failed, 0 skipped | copy-back + `cmp` + `touch` | 2.0 s |
| R5 day guard | `lib.rs` `MorningDigestRenderer::render_at` — `day <= d` → `day == d` | `the_renderer_renders_once_per_utc_day_and_never_on_a_clock_step_back` FAIL at `digest_production_18_1.rs:199`. Result: 3 tests run: 2 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.7 s |
| AA-2 digest TL frame | `lib.rs` `render_at` — `(self.frame_sink)(pid, …)` deleted | `the_production_digest_meets_fr17_against_a_seeded_tl` FAIL at `butler_digest_18_1.rs:224`. Result: 2 tests run: 1 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 9.1 s |
| AA-3 start render | `main.rs` serving arm — the start `digest.render_logged(pid)` deleted (the rollover timer's first tick is one period out, so nothing else renders before the first pass) | `the_serving_root_renders_at_start_mints_per_pass_and_survives_a_refused_pass` FAIL at `butler_daemon_18_1.rs:263`. Result: 1 test run: 0 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 7.5 s |
| D-05 cached token | `main.rs` the seam's mint — first issued token cached in a `Mutex<Option<_>>` and returned on every later call | `the_serving_root_renders_at_start_mints_per_pass_and_survives_a_refused_pass` FAIL at `butler_daemon_18_1.rs:133`. Result: 1 test run: 0 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 51.9 s |
| AA-4/TIA-7 AC6(b) absent hex ref | `main.rs` `render_j3_digest_scene` — the summaries' `source_log_ref` = `"f".repeat(32)` (well-formed hex, absent from the TL) | `j3_team_digest_within_30s_cites_only_real_tl_frames` FAIL at `journey_j3.rs:101`; daemon `Error: "distillation failed: source frame [255; 16] not found in transparency log"`; `j3_anti_canned_blinded_raw_input_moves_rendered_line` FAIL at `journey_j3.rs:199`; `j3_day_30_digest_renders_and_persists_via_real_daemon` FAIL at `journey_j3.rs:37`. Result: 4 tests run: 1 passed, 3 failed, 0 skipped | copy-back + `cmp` + `touch` | 9.2 s |
| TIA-6 window truth set | `spirits/digest/tests/hallucination_18_1.rs` `check` — refs resolved against the WHOLE TL again (pre-review shape) | `negative_control_a_ref_borrowed_from_another_window_reds_the_hallucination_check` FAIL at `hallucination_18_1.rs:236`. Result: 4 tests run: 3 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 1.2 s |
| TIA-8/TIA-9 line-1 binding grep | `.github/workflows/discipline.yml` line-1 step — the `butler inference seam wired` grep disabled (`if false; then`) | `epic_18_exit_runs_line_1_verbatim_and_its_proven_red_twin` FAIL at `ci_shape_18_0.rs:274`. Result: 5 tests run: 4 passed, 1 failed, 0 skipped | copy-back + `cmp` + `touch` | 0.6 s |

AA-4/TIA-7: the well-formed absent ref is refused FIRST by the I11 persist (`distillation failed: source frame … not found
in transparency log`) before `team_digest` prints, so the witness reds at its exit precondition again; the ref-resolution loop
(`journey_j3.rs`) is recorded as **defence in depth** behind I11, per the ruling. TIA-1: the AC1.6 shape control's red is JB-5
(TIA-1a); the CR-1 cases are the refused-pass arm's red (TIA-1b).
