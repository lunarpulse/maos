# Story 28-3 — independent-review fixes: evidence

Worktree `dev_ws-e283m`, branch `e28-3-integrate-2026-11-25` (HEAD `e5dfb080`), fixes uncommitted.
Offline only: no provider key was exported, `MAOS_INFERENCE_MODE` was never `live`; every run below is
a stub/cassette replay. **T4 (live k = 5) and T5 (promotion) are NOT run and nothing here substitutes for
them**; operator approval is pending.

## Findings and fixes

| # | Finding | Fix | Tests (red first) |
|---|---------|-----|-------------------|
| 1 | P1 `{"summary":…}` passes the one-field check, then `object["finding"]` panics | ONE shared strict decoder `maos_domain::distillation::parse_finding_completion` (missing / extra / duplicate field, array, scalar, fence, trailing text ⇒ `None`); `distill_replay::is_finding_completion` and the Spirit both call it | `maos-eval/tests/distill_28_3_review.rs::a_survey_reply_…_never_a_panic` (red: panic `no entry found for key`, captured), `maos-domain` decoder tests |
| 2 | judge-repeat digest is the survey's; `--cassette` may equal `MAOS_REPLAY_CASSETTE` | line: `cassette`/`cassette_sha256` = judge OUTPUT; `judge_repeat.survey_cassette`/`survey_cassette_sha256` = survey INPUT. Same-file (canonical path or dev+inode) refused at the CLI parse and the evaluator, before inference. `xtask eval-report` re-hashes both artifacts and reports both in `runs[]` | `eval_distill_18_3`: `a_judge_repeat_reports_both_artifacts…`, `the_survey_and_the_judge_output_may_not_be_the_same_file`; `spirit_eval_protocol_28_1`: `a_judge_repeat_line_is_checked_against_both_of_its_artifacts` |
| 3 | only the first of k replies reached the attribution check | `judge_reply_verdict` (pinned model + one of two words) runs on EVERY reply; the error is raised inside the loop, so the recording is still flushed first | `a_later_repeat_reply_from_another_model_is_refused_after_flushing` |
| 4 | line has only aggregates ⇒ `wilson95: null`, `--calibrate` refuses | line carries `decisions` (per clean claim: found / faithful / resolves / leaks / per-hedge kept). `eval-report` takes the per-claim majority over the k lines (claims for recall, faithfulness, traceability, leakage; hedges for preservation), cross-checks each line's `score.metrics` against its own decisions, refuses a half-decided set; a pre-28-3 line (no decisions) keeps `wilson95: null`. Workflow: a separate execution + archive step for the judge-only run (see below) | `decisions_give_the_distill_metrics_a_wilson_interval…` (lower bounds 0.8668 / 0.8046 hand-computed), `a_line_whose_metrics_disagree…`, `the_line_carries_the_decisions…`, `ci_shape_18_0::nightly_researcher_judge_repeats_use_a_fixed_survey` |
| 5 | `faithfulness_min` was the lowest per-finding fraction | per-pass totals kept; reported `faithfulness_per_pass`, `faithfulness_mean`, `faithfulness_worst_pass`, `agreement_rate`; `faithfulness_min` removed; `gate_basis: "first-verdict"` states that `pass`/`missed_floors` rest on each finding's first verdict and the repeat statistics never gate | `faithfulness_statistics_tell_a_one_pass_flip_from_four_passes_flipping` (same pass: worst = (N−4)/N; four passes: worst = (N−1)/N; equal mean and agreement) |
| 6 | Spirit accepted extra fields; on decode failure it persisted the raw model reply | Spirit uses the shared decoder; on failure it persists `summarize(source statement)` (the deterministic summary), never the model text; the completion is still consumed/counted | `inference_seam_18_3::a_malformed_completion_falls_back_to_the_source_summary_not_the_model_text` (7 malformed shapes) |
| 7 | `--judge-repeat` any usize; `0` silently erased | 1–10 enforced at CLI parse (also for a default) and at `evaluate_with_repeat`; 0 and 11 refused before any inference | `the_judge_repeat_count_is_one_to_ten_at_the_cli_and_the_evaluator` |
| 8 | `FloorEntry` cannot carry judge model/agreement; blank ruling accepted | `DistillFloorRecord` now requires `[judge]` (`JudgeEvidence`: pinned model, judge-prompt SHA, k, findings, calls, agree, result_file, rule, rationale — coherence checked); `apply(base, pinned_judge_model)`; non-blank `operator_ruling` required below the evidence floor; finite/ordered/[0,1] checks as Butler's | `distill_28_3_review.rs` (refusal tests: no `[judge]`, no `agree`, other model/prompt, k<2, calls≠k·findings, agree out of range, blank/space ruling, NaN/out-of-range evidence) |
| 9 | prompt test did not prove the 4 examples are the only ones / no held-out text | `the_prompt_holds_exactly_the_four_train_examples_and_no_held_out_text`: EXAMPLES claims == the 4 marked train rows in order, counts of `Claim:`/`hedges:`/`{"finding":`/`Example`, unmarked train rows absent, every held-out claim statement AND frozen finding statement (derived from the test split) absent from the template | the test + plant |
| 10 | journey pin test compared constants | `journey_forget_18_6::seed_cassette_has_only_real_prompt_hashes` derives hash and length from `researcher::seed_claims()` + `synthesis_prompt()` (dev-deps `researcher`, `sha2 = 0.10`; both already in the lock — `Cargo.lock` gains the two edges, no new crate) | plant: a prompt edit now turns it red |

### Found while fixing, fixed (not in the ten)

* **Calibrated gate dropped the zero-leakage ceiling.** `missed_floors_with` (what `maos eval distill` runs via `gate_failures_with`) had no `secret_leakage` check, so a clean finding that trips redaction passed the CLI. `missed_floors` now delegates to `missed_floors_with`, which holds the compiled ceiling. Test `the_calibrated_gate_holds_the_zero_leakage_ceiling` (red first). The resume tree's notes record the same defect.
* **Judge-only repeat over a `seed` survey** asked the judge nothing and printed frozen verdicts: refused (`needs a live-record survey cassette`); the judge-output cassette must hold exactly the measured judge calls (`entries == judge calls`, zero calls refused). Tests `a_judge_repeat_over_a_seed_survey_is_refused…`, `the_judge_output_cassette_holds_exactly…` (the seed refusal was red first).
* **Consequence of the strict Spirit (6).** The three legacy seed cassettes (`forget-redistill-18-6`, `skill-authoring-18-7`, `survey-distill`) answered survey prompts in plain text, which the strict decoder now (correctly) treats as malformed ⇒ the Spirit would persist the source statement and the authoring prompt hash would drift (observed: `cassette drift at seq 3`). Their survey responses were migrated to the v1 schema `{"finding": <old text>}` by extending the repository's own generators (`prompt_28_3.rs`: `as_v1_finding`, run with `MAOS_GEN_DISTILL_CASSETTE=1 … --ignored generate_survey_distill generate_forget_redistill generate_skill_authoring`); prompt hashes, lengths and every downstream value are unchanged. Test stubs that answered prose (`five_metric_live_8_11`, `inference_seam_8_11`, `skill_authoring_18_7`, `inference_seam_18_3`) now answer the v1 object.

### Judge-only workflow step (finding 4, plain workflow edit — not run)

`.github/workflows/journey-nightly.yml`: the Researcher record step gets `id: researcher-record`; a new step *Measure Researcher judge agreement (fixed survey, k repeats)* (same `include_researcher` opt-in and key, gated on the record step's success) runs `maos eval distill --class researcher --cassette …live.1.json --judge-repeat "$REPEATS"` with `MAOS_REPLAY_CASSETTE` set to a distinct `…judge-only.live.json`, stdout to `eval-results/researcher.judge-only.json` (archived by the existing upload; never appended to `researcher.results.jsonl`, so `eval-report` does not count it as a repeat). A measured floor miss (exit 1 with a line) is kept; a structural refusal (exit 1, empty output) fails. Pin: `ci_shape_18_0::nightly_researcher_judge_repeats_use_a_fixed_survey`. No workflow was run.

## Plants (marker `E283-PLANT`)

Planted together in one tree, tests run, each planted site then restored by edit and grep-verified
(`grep -rn E283-PLANT` over the repository, excluding this note: no hit; the six edited sources are byte-identical to
copies taken before planting; prompt SHA-256 `489445b15eaf69ec5e21e01d97e9c3ed499c224e325d421485e6496fcc8188da`
unchanged).

| Plant | Site | Tests that went red |
|-------|------|---------------------|
| CLI count bound removed | `distill_eval.rs` parse (`check_repeat_count`) and evaluator | `the_judge_repeat_count_is_one_to_ten…` |
| alias refusal removed (parse + `refuse_aliased`) | `distill_eval.rs` | `the_survey_and_the_judge_output_may_not_be_the_same_file` |
| attribution only on the first reply | `distill_eval.rs` judge loop | `a_later_repeat_reply_from_another_model_is_refused_after_flushing` |
| worst pass := first pass | `distill_eval.rs` | `faithfulness_statistics_tell_a_one_pass_flip…` |
| `cassette_sha256` := survey bytes | `distill_eval.rs` | `a_judge_repeat_reports_both_artifacts…` |
| `decisions` := null | `distill_eval.rs` | `the_line_carries_the_decisions…` |
| scorer back to `Value`-indexing | `distill_replay.rs::is_finding_completion` | `a_survey_reply_…_never_a_panic` |
| blank ruling accepted (`is_none()`) | `distill_replay.rs::apply` | `a_below_spec_floor_needs_a_non_blank_operator_ruling` |
| Spirit falls back to `summarize(&text)` | `spirits/researcher/src/lib.rs` | `a_malformed_completion_falls_back_to_the_source_summary…` |
| 5th example = held-out `rd05-c1` statement | `synthesis-v1.txt` | `the_prompt_holds_exactly_the_four_train_examples…`, `prompt_file_is_pinned`, and the derived journey pin `seed_cassette_has_only_real_prompt_hashes` |
| decisions ignored | `xtask/src/eval_report.rs::decisions_of` | the two decision tests of `spirit_eval_protocol_28_1` |

## Scope notes the scorer cannot settle

* The scorer (`is_finding_completion`) and the Spirit check **shape only**: one JSON object, one string `finding`. A
  schema-valid string may still hold several sentences or meta-text; only the judge rubric (support / contradiction of
  the source claim) bears on content. This evidence does not claim a one-sentence or no-meta-text check exists.
* `rd05-c1` ("seem to" → "appear to", the miss that motivated the prompt) is now **taught by the prompt** (example 2 is
  the synonym-trap pattern; rule 2 names it). Its pass must be treated as a **tuned regression case**, not independent
  evidence of generalisation. The test split row itself is held out verbatim (asserted). The story text
  (`28-3-researcher-distill-prompt.md`) was searched: it records the miss as a measurement (line 40) and does not
  claim independence for it, so no sentence needs correcting for this point.
* Interval semantics: Wilson is over the per-claim (hedge) **majority** of the k full repeats — reported, not gated,
  as for Butler (D-28-1-01 B). Judge-repeat statistics (`faithfulness_per_pass`, worst pass, agreement) are
  disclosure; the gate is the first verdict of each finding. Whether k = 5 full repeats at the chosen floors passes
  can only be settled by the paid run.

## "Local archival digest" defect

Conclusion: it is finding 2. In judge-repeat mode `cassette` named the recorded judge OUTPUT while `cassette_sha256`
hashed the replayed survey INPUT, so an archived result line could never be re-verified by `xtask eval-report`
(`is not the cassette its result line names`), and with the same path for both inputs the recorder overwrote the source
survey. The resume tree's story note ("Local judge-only cassette digest correction") describes the same defect and the
same shape of fix (top-level fields = judge output, nested `judge_repeat.survey_*` = survey), which this tree now
carries (re-implemented here, with `eval-report` consuming both digests).

## Dev Agent Record lines to correct (docs worktree — not edited here)

`_bmad-output/implementation-artifacts/28-3-researcher-distill-prompt.md` (dev_ws-course28):

* L77–L79 (T1/T2/T3 "Implemented locally in `4e293036`", "`170c5bbb` and `c69ebd51`"): stale commit names; replace by the
  integrated head's commits once committed (`e5dfb080` = main `86fc6020` + the story's commits, then this fix commit).
* L102 ("branch `e28-3-2026-11-16` at `c69ebd51…`; T1/T3 `4e293036`, T2 `170c5bbb` and `c69ebd51`; base `fad08ae0…`")
  and L104 ("Integrate this record before resuming work… use `fad08ae0...c69ebd51`"), L108 ("Complete 20-file local patch
  scope, `fad08ae0...c69ebd51`"): replace with the integrated branch `e28-3-integrate-2026-11-25`, base main `86fc6020`,
  head = the commit(s) of this fix; the File List must add `crates/maos-domain/src/distillation.rs`,
  `crates/maos-domain/…`/`Cargo.lock`, `crates/maos-journey-test/Cargo.toml`, `xtask/src/eval_report.rs`,
  `xtask/tests/{ci_shape_18_0,spirit_eval_protocol_28_1}.rs`, `.github/workflows/journey-nightly.yml`,
  `crates/maos-eval/tests/distill_28_3_review.rs`, the three seed cassettes' responses, the four researcher test files
  and this note.
* L103 (prepush log `/tmp/prepush-e283c.log:726–727`, 82 checks): belongs to the superseded branch, not this tree.
* The prompt SHA: the story text in the docs worktree carries no prompt SHA; the pinned value is
  `spirits/researcher/tests/common/mod.rs` `PINNED_PROMPT_SHA256` = `489445b15eaf69ec5e21e01d97e9c3ed499c224e325d421485e6496fcc8188da`,
  which equals `sha256sum spirits/researcher/prompts/synthesis-v1.txt` at this tree (checked). Any record naming another
  SHA is stale.
* Any sentence that says the Spirit "falls back to the bounded deterministic summary" of the *model text* must say
  "of the source statement" (finding 6).

## Provenance of ported pieces (review eligibility)

From the GPT/Codex-written candidate `/home/lunarpulse/dev_ws-e283-resume` (read-only; branch `resume-e283-2026-11-18`):
`parse_finding_completion` + `StrictFinding` visitor + its two unit tests (`maos-domain`; extended with the
`{"summary"}` case), `JudgeEvidence` and its `check`, the validity/non-blank-ruling body of `DistillFloorRecord::apply`,
`judge_reply_verdict`, and the shape of the judge-only workflow step with its `ci_shape_18_0` test (adapted: gated on
`researcher-record` only). **These pieces must be reviewed by a Claude-family or other non-GPT reviewer.** Not ported:
the resume Spirit fallback (`summarize(&text)` — the defect of finding 6), its `repeats-validation` gating of the paid
steps (a separate change, not in scope here), its `distill_eval.rs`. Everything else (evaluator, line, xtask, tests) was
written here.

## Verification run (all offline; `MAOS_AUDIT_KEY` = random 64-hex 0600 file for the maos-bin run)

`cargo test --locked`: `-p maos-domain` 353, `-p researcher` 79, `-p maos-eval` 94, `-p maos-bench` 38, `-p maos-bin
--test eval_distill_18_3 --test researcher_inference_18_3 --test epic_args_19_1 --test provider_endpoint_pin_20_4 --test
researcher_8_14c` 53, `-p maos-journey-test` 69, `-p xtask` 1291 — 0 failed. Gates: `check-kernel-baseline` PASSED
(28339, Δ 0, `kernel-core-baseline.toml` untouched), `kloc-check` PASSED, `check-env-contract` PASS (159, no new env var),
`check-composition-root-completeness` PASS, `check-dependency-closure` PASSED, `cargo check --locked -p maos-bin` in the four
shapes (default, `network`, `wasm-host`, `--no-default-features`) rc 0, `cargo fmt --all --check` rc 0, `cargo deny check`
ok, `cargo clippy --all-targets` on the touched crates rc 0 with only pre-existing warnings in the files touched
(`distill_eval.rs:418 obfuscated_if_else`, researcher test/`lib.rs` warnings).

# Round 2 — fixes after the two static reviews (GPT-6.2; Claude Opus for the ported pieces)

Base `b5141d52`, fixes uncommitted, offline only (no key exported, never `MAOS_INFERENCE_MODE=live`).
T4 (live k = 5) and T5 (promotion) remain unrun.

| # | Finding | Result |
|---|---------|--------|
| A | attribution came from the REQUEST | **Fixed (small change).** The Messages API response carries `model`; `crates/maos-providers/src/anthropic.rs::parse_response` now attributes the call to it (a `-YYYYMMDD` snapshot of the requested model keeps the requested id; any other served model is reported as itself; no `model` in the response ⇒ `model_id: None`, "unverified"). `distill_replay::judge_reply_verdict` refuses `None` ("unverified") as it refuses another model, so such a reply never counts as pinned-judge evidence. Test mocks that canned Anthropic replies without `model` now echo the served model: `maos-providers` (`anthropic.rs` unit tests, `tests/admitted_models_s1.rs` `"$echo"`, `tests/anthropic_5_5_live_leg_correction.rs`), `maos-bin/tests/manifest_model_pin_s1.rs`. **No committed cassette changes:** replay serves recorded attribution as recorded, and the seed/live-record cassettes already carry the request-echoed ids, which is what a snapshot-normalised response gives. Caveats: (i) previously archived live-record cassettes carried a request-derived attribution, so they are not retro-verified; (ii) `maos-bin/src/workflow.rs:846` compares attribution to the approved model with equality, so a workflow reply whose served model is another model, or absent, is now refused there too (intended; the real API always returns `model`); (iii) only the paid run shows what the API returns for `claude-sonnet-5-5` (an alias resolved to a form that is not the `-YYYYMMDD` suffix would make the judge-only step refuse, loudly). Test: `anthropic::tests::attribution_comes_from_the_response_model_not_the_request`, `distill_28_3_review::a_judge_reply_without_or_with_another_attribution_is_not_pinned_judge_evidence`. |
| B | alias guard covered two paths only | **Fixed.** `distill_eval::check_destinations`: every write (recorded cassette, its `.verdicts.json` sidecar) against every read (survey cassette and ITS sidecar, committed seed, corpus, judge config, floor record) and against each other, at the CLI parse and in `evaluate_with_repeat`, before any inference (also plain record, not only judge-repeat). Equality = same device+inode (hard links) or same resolved path: made absolute, nearest existing ancestor canonicalised (symlinks), missing tail appended (so an absent floor file or `a/../b` spelling is caught). Test `no_write_destination_may_alias_a_read_input_or_another_write` (10 cases: survey-sidecar name, seed, `trap.json`→sidecar==survey, corpus abs + relative, judge config, absent floors file, symlink, hard link, survey itself; survey/seed/sidecar bytes untouched, zero judge calls). |
| C | a refused repeat looked like a floor miss; judge-only ran on a short set | **Fixed.** Workflow: `id: repeats-validation` and `steps.repeats-validation.outcome == 'success'` restored on the record, structure and report steps (judge-only too); the record loop now stops when a repeat exits 1 without appending a result line; new step *Validate the Researcher result set (k lines)* (`id: researcher-structure`) runs `eval-report --structure-only`; the paid judge-only step needs validation + record + structure success. `eval-report --structure-only --repeats k`: exit 0 iff exactly k lines, each digest-checked (+ survey digest of a repeat line), consistent (`event/class/corpus/floors`), decisions coherent, no duplicate line and no two lines naming one cassette — a genuine floor miss is valid; it exits 1 for a missing/short/over-long/duplicated/digest-broken/inconsistent set. Tests: `ci_shape_18_0` (conditions incl. `RESEARCHER_CONDITION`, ids, order, plants for the dropped validation clause) + `the_record_loop_keeps_a_floor_miss_but_stops_on_a_refusal` (runs the step's real script with a stub `maos`: miss ⇒ all k run and exit 0; refusal at repeat 2 ⇒ stops, exit 1, 1 line) + `structure_only_separates_a_floor_miss_from_a_broken_result_set`. Not run: any workflow. The Butler record loop has the same exit-1 ambiguity; left unchanged (out of this story). |
| D | format errors invisible in the aggregate; `--calibrate` blind | **Fixed.** The aggregate reads each run's `score.metrics.format_errors` (required on a line that carries decisions; a pre-28-3 line predates it), **sums** it over the k runs (tolerance 0, the scorer's floor), reports `format_errors: {count, rows: ["run i"]}`, adds a structural failure `format_errors N above floor 0 (runs …)` (so the verdict is red), and a new `structural_failures` array. `--calibrate` on a distill result refuses any nonzero count or any structural failure ("structurally invalid evidence"). Test `format_errors_are_summed_and_refuse_a_green_verdict_and_a_calibration` (ratios all healthy, verdict red, calibration refused, missing count refused). |
| E | aggregate gated on compiled constants | **Fixed.** The numeric bounds are the lines' agreed `floors` object (already compared across lines); a line without one keeps the compiled constants; leakage keeps the compiled zero ceiling. Test `the_aggregate_is_gated_against_the_floors_the_runs_applied` (0.94 passes; absent ⇒ 0.95 fails; a lax `floors` cannot lift the leakage ceiling). Trust note: `eval-report` believes a line's `floors`; the lowering is justified only by `floors.toml` + its ruling at CLI time. |
| F | hedge vectors only length-checked against each other | **Fixed.** Each present finding's hedge count (and the claim ids) is checked against the digest-checked corpus' source claim. Test `hedge_vectors_are_checked_against_the_source_claims` (all five-line-style duplication refused; a dropped flag refused). |
| G | `[judge]` k = 11 passed; k = 1 mismatch | **Fixed.** One source: `distill_replay::{MAX_JUDGE_REPEAT = 10, MIN_AGREEMENT_REPEAT = 2}`; `distill_eval::MAX_JUDGE_REPEAT` re-exports it. `JudgeEvidence.k` must be in 2..=10 (separate messages). **k = 1 can and cannot establish:** `--judge-repeat 1` (and a hosted `repeats = 1`) is a legal pass — one live judge verdict per finding, the first-verdict gate, both artifacts with digests — but every finding trivially agrees with itself, so it establishes no agreement and cannot back a floor record's `[judge]` evidence. Tests assert WHICH check fired (messages), incl. the review's k 11 / findings 3 / calls 33 / agree 33 case. |
| H | decoder accepted blank / control-character findings | **Fixed in the shared decoder.** `parse_finding_completion` returns `None` for a finding blank after `trim()` or containing any control character (NUL, tab, newline…). **Multi-line decision:** a newline is rejected too — the prompt asks for ONE sentence (rule 1) and forbids a second paragraph (rule 5), and the story's measured defect was 11 of 29 completions carrying a preamble or second paragraph; no committed completion or corpus finding contains a control character (checked). Both consumers follow: the scorer counts a format error, the Spirit falls back to the source statement. Tests: domain decoder cases, `distill_28_3_review` (scorer), `inference_seam_18_3` (Spirit fallback). |
| I | vacuous tests | **Fixed + planted.** `ci_shape_18_0`: empty upload path is refused before every `starts_with(uploaded.trim())` (two places); the judge step's `MAOS_ANTHROPIC_API_KEY` must equal `${{ secrets.MAOS_ANTHROPIC_API_KEY }}` (not "equal to the other step's, both absent"); `distill_28_3_review` asserts the message of the check that fired for every evidence case (the "k below 2" case fails when only that check is deleted). |
| J | record only (no behaviour change except J-2 trivial case) | **Open decision J-1** — judge evidence `result_file` is only checked to be a relative path; k/findings/calls/agree are not checked against the archived result, and `prompt_sha256 = "ab"` is accepted in the Butler-shaped `FloorEntry`; forged evidence can only RAISE a floor unless an `operator_ruling` is present. *One option:* make `eval-report --calibrate` emit the `[judge]`/floor stanza itself (with the archived result's SHA-256 and the counts) and have `distill_floors()` re-hash `result_file` when present. **Open decision J-2** — any non-blank free-text `operator_ruling` lowers ANY floor, down to 0.0, and is not tied to a decision row; AC6 says this story lowers no minimum. *One option:* require the ruling to be a `decisions-*.md` row id that exists in the planning artifacts (checked by an xtask gate). **Tightened trivially (done):** no ruling lowers `traceability` (NFR-Aud-7: 100 %); leakage is not a calibrated floor. Test `no_ruling_lowers_traceability`. |
| K | defence in depth | **Done (plain edit).** Top-level `permissions: contents: read` in `journey-nightly.yml` (no job widens it; no step uses the token), pinned by `ci_shape_18_0` (the workflow's `permissions` mapping and no job-level override). Third-party actions are pinned by tag (`actions/checkout@v5`, `dtolnay/rust-toolchain@v1`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `actions/upload-artifact@v4`), not SHA — **recorded, not changed** (a SHA pin needs the repository's pin policy and verified SHAs). |

## Plants, round 2 (marker `E283-PLANT`; all removed, grep-verified; the sources are byte-identical to the copies taken before planting)

| Plant | Red |
|-------|-----|
| `anthropic.rs` attribution := request | `attribution_comes_from_the_response_model_not_the_request` |
| `check_destinations` never refuses | `the_survey_and_the_judge_output_may_not_be_the_same_file`, `no_write_destination_may_alias_…` |
| evidence `k < 2` check off / `k > 10` check off (each alone) | `the_record_carries_the_judge_model_and_agreement_or_it_is_refused` (wrong message / `unwrap_err` on `Ok`) |
| traceability ruling check off | `no_ruling_lowers_traceability` |
| decoder filter off | domain `finding_completion_rejects_every_other_shape…`, scorer test, Spirit `a_malformed_completion_falls_back…` |
| hedge-count check off | `hedge_vectors_are_checked_against_the_source_claims` |
| `floors` ignored | `the_aggregate_is_gated_against_the_floors_the_runs_applied` |
| format-error aggregation off | `format_errors_are_summed_…` |
| duplicate-line check off | `structure_only_separates_a_floor_miss_…` |
| `--calibrate` refusal off | `format_errors_are_summed_…` (calibration assertion) |
| workflow: structure gate dropped from the paid step | `nightly_researcher_judge_repeats_use_a_fixed_survey` (condition) |
| workflow: refusal check in the record loop disabled | same test (script pin) |
| workflow: `permissions` removed | same test (`missing key permissions`) |
| workflow: upload `path: ""` | same test and `epic_18_exit_runs_line_2_…` (empty path no longer vacuous) |
| workflow: judge step's API key removed | same test (key must be the secret) |

## Verification, round 2

`cargo test --locked`: `-p maos-domain` 353, `-p researcher` 79, `-p maos-eval` 96, `-p maos-bench` 38, `-p maos-providers` 115,
`-p maos-bin` (`eval_distill_18_3`, `researcher_inference_18_3`, `epic_args_19_1`, `provider_endpoint_pin_20_4`,
`researcher_8_14c`, `manifest_model_pin_s1`; random 0600 hex `MAOS_AUDIT_KEY`) 61, `-p maos-journey-test` 69, `-p xtask` 1296 — 0
failed. `check-kernel-baseline` 28,339 (Δ 0), `kloc-check`, `check-env-contract` 159, composition root, dependency closure,
`cargo check --locked -p maos-bin` in the four shapes, `cargo fmt --check`, `cargo deny check`: pass. `cargo clippy
--all-targets` on the touched crates (+ `maos-providers`) exits 0; no warning in a line this round added; the two extra
warnings vs round 1 are pre-existing ones in `maos-providers/tests/{ollama,openai}_round_trip_test.rs` (newly linted crate).
**Only the paid run can settle:** that the live Messages API returns `model` for the pinned judge in an id this adapter
maps to the pin; that k = 5 repeats pass at the floors; that the judge-only step and the new structure step run in Actions.
