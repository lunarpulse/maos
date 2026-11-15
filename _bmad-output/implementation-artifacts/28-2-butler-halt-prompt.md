---
story_id: '28.2'
story_key: 28-2-butler-halt-prompt
created: 2026-11-15
context_commit: bf3cba25
baseline_commit: bf3cba25
security_sensitive: false
kernel_grant: "Not needed: kernel-Δ forecast 0 (method: read every touched file — spirits/butler/**, crates/maos-bin/tests/**, crates/maos-journey-test/cassettes/j-butler/*.json, tests/coverage-matrix.yaml; none is under crates/maos-kernel-core/src). Growth inside the ADR-069 envelope would be self-service (ruling `kfree`); the landing measures with `check-kernel-baseline --json` / `kloc-check --json` and, if the kernel moved, re-pins with `scripts/repin.py` and writes the HISTORY row in the same commit."
kloc_grant: "Forecast 0 charged lines: spirits/ is excluded by kloc-check (kloc_check.rs:303-304); tests, cassettes and TOML/markdown are uncharged. If the dev touches maos-bench beyond 28-1, it re-prices at T0."
review: "bmad-code-review under docs/testing/proof-contract.md; author and reviewer from different model families. Creation anthropic/claude-opus-5-5."
---

# Story 28.2: The Butler halt prompt

Status: ready-for-dev

<!-- Created 2026-11-15 at bf3cba25 by bmad-create-story under the lighter measured creation contract. Operator ruling: _bmad-output/planning-artifacts/decisions-2026-11-15-spirit-quality.md (rulings 1, 2, 3, 5). Live evidence outside git (ruling B7a): /home/lunarpulse/maos-evidence-archive/2026-11-15/live-rerecord-run-43708370483-cassettes/j-butler/halt-corpus-v0.3.live.json and the job log (line 4466). Per-row values below were read from that cassette at creation. No cargo or gate was run. D-28-2-01…06 are PROPOSED. Starts after 28-1's rulings and AC1. -->

**Creation rationale — ready-for-dev at filing, after 28-1; gating D-rows resolved 2026-11-15.** The prompt work, the train split and the replay are hermetic; the live measurement is the on-demand paid dispatch (28-1 AC5). **`v1.0.0` GA precondition** (ruling 2026-11-15 item 1: "before GA the Butler live evaluation must PASS its (calibrated) floor").

> **The capability:** *Butler's proposal prompt is a versioned file with fixed sections and explicit decision rules; on `claude-haiku-5-5` it stops raising halts for tentative, cancelled and back-to-back calendars and for scenarios with no preference signal, and the measured precision and recall meet floors calibrated from that measurement.*

## Story

As **the director who is interrupted by a Butler halt**,
I want **Butler's prompt to define exactly when a calendar is in conflict and what the preference-drift number means, so the model returns the same decision for the same situation**,
so that **a halt means a real confirmed-vs-confirmed conflict or real preference drift, and the GA evaluation measures that on the reference model**.

**Contract:** ruling items 1, 2, 3, 5; epic §28-2; 28-1's protocol as ruled. Create-story may tighten, not add uncited ACs.

## How a Butler proposal becomes a halt (measured at `bf3cba25`)

1. `maos eval halt --class butler` runs one bound `on_idle` pass per corpus row (`crates/maos-bench/src/halt_eval.rs:279-294`, `butler_pass` `:568-599`); the row's verdict is whether the kernel returned a halt receipt (`:598`).
2. The pass computes conflicts **arithmetically**: overlapping pairs of `Confirmed` events only, half-open intervals (`spirits/butler/src/lib.rs:2130-2151`); `belief_variance` = 0.2 without a conflict, 0.80…0.99 with one or more (`:1679-1683`).
3. The model is called every pass (`infer_proposal`, `:1171-1207`) with `proposal_prompt` (`:197-223`); its JSON (`ButlerProposal`: `pattern`, `confidence`, `evidence`, `options`, `user_preference_drift`, `:147-153`) is parsed strictly (`parse_proposal`, `:159-192`). Only **`user_preference_drift`** enters the halt path: it overwrites the assessment's value (`:1100`). `pattern`, `confidence`, `evidence` and `options` become the notification only (`:1101-1107`).
4. `primary_scalar` (`:661-692`): with a conflict it writes `belief_variance` (arithmetic); without one, if drift < 1.0 it writes `user_preference_drift`; the write goes to the kernel (`:1145-1153`).
5. The manifest's `[epistemic_policy]` halts on `belief_variance` > 0.7 and on `user_preference_drift` < 0.6 (`spirits/butler/manifest.toml:85-94`).

So on the conflict rows (cc) the halt is arithmetic and the model cannot miss it unless its completion is malformed; on every conflict-free row the halt is the model's `user_preference_drift` against 0.6.

## Live measurement 2026-11-15 (run 43708370483, `claude-haiku-5-5`, k = 1)

| Family | Rows | Expected | Scenario | Model `user_preference_drift` | Result |
|---|---|---|---|---|---|
| cc | cc01–cc12 | halt | confirmed–confirmed overlap | 0.5–0.9 | 12 TP (arithmetic) |
| df | df01–df03 | halt | `preference_alignment` 0.4 / 0.5 / 0.35 | 0.4 / 0.5 / 0.35 | 3 TP |
| pb | pb01, pb03, pb05 | no halt | confirmed + **tentative** overlap, no `preference_alignment` | 0.9 / **0.5** / 0.9 | 1 FP (pb03), 2 TN |
| pb | pb02, pb04, pb06 | no halt | confirmed + **cancelled** overlap, no `preference_alignment` | **0.5 / 0.5 / 0.5** | 3 FP |
| tn | tn01, tn03, tn05, tn07, tn09 | no halt | back-to-back / gap, no `preference_alignment` | **0.5**, **0.5**, **0.5**, 0.9, **0.5** | 4 FP, 1 TN (tn07) |
| tn | tn02, tn04, tn06, tn08 | no halt | `preference_alignment` 0.9 / 0.75 / 0.8 / 0.95 | 0.9 / 0.75 / 0.9 / 0.95 | 4 TN |

Precision 15/23 = 0.652 (Wilson 95 % [0.449, 0.812]); recall 15/15 = 1.0 (calendar subset 12/12, arithmetic basis, `halt_eval.rs:112`). Every one of the 8 false positives is a conflict-free row without a preference signal where the model answered drift **0.5**; on three such rows (pb01, pb05, tn07) it answered 0.9 — the same situation, two answers. The model's `confidence` (0.85–0.92 on pb01/pb03/pb05) is not on the halt path. The prompt today defines drift only as "1.0 = fully aligned; lower = more drift" (`lib.rs:219-220`) and never says what an absent signal means; the hand-authored seed encodes "absent ⇒ 0.95" (`spirits/butler/tests/halt_eval_corpus_18_2.rs:207-210`). In production the MCP path never sets `preference_alignment` (`lib.rs:1068-1072`), so there the drift halt is the model's reading of comms alone.

## Current-tree corrections — read before implementing

| Premise (ruling record / assignment) | Fact at `bf3cba25` |
|---|---|
| "The model reports high-confidence conflicts for overlaps that involve a tentative event (pb01, pb03, pb05 at 0.85-0.9; pb06 at 0.6)" (`decisions-2026-11-15-spirit-quality.md:6`) | OLD → NEW: `confidence` does not reach the kernel. The 8 FPs are pb02, pb03, pb04, pb06, tn01, tn03, tn05, tn09 — each with model drift 0.5 < 0.6; pb01 and pb05 (confidence 0.85) did **not** halt (drift 0.9). The ruling record is not edited (outside this story's files); this table is the correction. |
| "floor 0.85 (spirits/butler/tests/corpus_halt.rs and docs/research/nfr-onb-1-protocol.md:61)" | `corpus_halt.rs:302-306` asserts the arithmetic self-trial; the live eval floor is `HALT_PRECISION_FLOOR` (`crates/maos-eval/src/onboarding_gate_corpus.rs:62`, via `halt_eval.rs:109`). `nfr-onb-1-protocol.md:61` confirmed. |
| "the prompt is built where `prompt_sha256` comes from" | `proposal_prompt` (`lib.rs:197-223`); the row and entry hashes are asserted equal to it by `halt_eval_corpus_18_2.rs:96-105`; `temperature: Some(0.0)` is requested (`lib.rs:1190`) but omitted on the wire for `claude-haiku-5-5` (`crates/maos-providers/src/anthropic.rs:105-107,146-148`). |
| Prompt change touches only the halt corpus | `proposal_prompt` also binds `j-butler/on-idle-halt.json` (21 entries; asserted by `crates/maos-bin/tests/butler_scene_18_4.rs:136-166`; read by 11 more maos-bin test files and `journey_forget_18_6.rs:65`) and `j-butler/context-budget-18-5.json` (`context_budget_18_5.rs:127-129`), plus the two negative controls (`halt_eval_corpus_18_2.rs:26-35`). All move together (D-28-2-06). |

## Kernel and kloc pricing — forecast at creation

**Kernel:** forecast **0** (method: read the touched files listed in `kernel_grant`). No D-row owed. **kloc:** **0** charged (`spirits/` excluded, `xtask/src/kloc_check.rs:303-304`; tests/cassettes/TOML uncharged). The 28-1 maos-bench/maos-eval/xtask lines are priced there.

## Decisions — ACCEPTED (operator: proceed with recommendation, 2026-11-15)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-28-2-01** | Fix locus for the false positives | **A** prompt only: define drift (absent or null `preference_alignment` and no explicit comms evidence ⇒ 1.0; an explicit `preference_alignment` is reported as given), conflicts (Butler's computed list is authoritative; tentative/cancelled overlaps and back-to-back events are never conflicts), decision procedure and schema · **B** take drift out of the model: arithmetic `preference_alignment`, 1.0 when absent; the model writes only the notification · **C** A, plus Butler ignores a model drift < 1.0 when the scenario has no comms awaiting reply | **A.** The operator asked for a prompt adjustment (ruling 1, 3); the measured cause is an undefined default (0.5 vs 0.9 for the same situation). B reverses 18-1's ruling that drift is the model's (D-18-1-03/04 A, `lib.rs:1090-1097`) and would make the class half arithmetic; C adds a code rule the corpus has not measured. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-2-02** | Prompt artefact | **A** `spirits/butler/prompts/proposal-v1.txt` with fixed sections (ROLE, OBJECTIVE, DEFINITIONS, DECISION PROCEDURE, OUTPUT SCHEMA, EXAMPLES, INPUT), `include_str!`-ed; `PROPOSAL_PROMPT_VERSION = "butler-proposal/v1"`; its SHA-256 pinned in a test · **B** keep the `format!` string in `lib.rs` with a version constant | **A.** Versioned and hash-bound as ruling 3 asks; a reviewer reads the prompt as a file. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-2-03** | Few-shot examples | **A** 4 boundary examples from the train split (tentative overlap; cancelled overlap; back-to-back without overlap and no preference signal; explicit low `preference_alignment`) · **B** zero-shot with decision rules only · **C** B first; add A only if the measurement misses the floor | **A.** Every measured FP is one of these boundaries; the examples cost ≈ 400–600 input tokens per call (≈ 0.00006 USD at the Haiku 5.5 row) [INFERENCE: token count estimated, not measured]. C needs two paid rounds and two promotions. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-2-04** | Corpus | **A** test corpus unchanged (30 rows, NFR-Onb-1, SHA-pinned `corpus_pin.rs:28`); new `spirits/butler/tests/fixtures/calendar-comms-v0.3.train.jsonl` (hand-authored, provenance header, disjoint inputs) · **B** add hold-out boundary rows to a `v0.4` test corpus | **A.** B changes the NFR-Onb-1 corpus and its count constant (`onboarding_gate_corpus.rs:68`). More halts would narrow the precision interval (≥ 22 needed for a lower bound ≥ 0.85, 28-1), but that is a spec decision, not this story's. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-2-05** | Promotion of the live cassette | **A** the median-scoring of the k live cassettes (ties → lowest index) replaces `halt-corpus-v0.3.json` (`provenance: live-record`, `model_id: claude-haiku-5-5`); the two negative controls are re-derived from it by planting · **B** keep the seed as the CI cassette and commit the live one beside it with a second CI replay | **A.** CI then replays what the reference model said (`deferred-work.md:1413` asks exactly this: accept `live-record`, derive the controls from the promoted cassette). B keeps a hand-authored cassette as the gated one. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-2-06** | The other Butler cassettes bound to `proposal_prompt` | **A** keep them hand-authored seeds; re-derive their `prompt_sha256`/`prompt_len` from the v1 prompt with a generator test in the same landing · **B** re-record them live too | **A.** They exercise plumbing (21 identical scene prompts, `butler_scene_18_4.rs:150-166`; context-budget), not model quality; B adds paid runs with no metric. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |

## Acceptance Criteria

1. **AC1 (command; exit line 2).** `cargo test -p butler --test halt_live_baseline_28_2` passes. It asserts: the class cassette `crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json` has `provenance: live-record` and `model_id` equal to the manifest pin `claude-haiku-5-5` (`spirits/butler/manifest.toml:21`); every entry hash equals the v1 prompt for its row; the prompt file's SHA-256 equals the pinned value; no train row's `input` equals a test row's and the prompt file contains no test row input (28-1 AC2); `spirits/butler/eval/floors.toml` holds every Butler floor with measurement, Wilson interval, k, model, prompt SHA, rule and rationale (28-1 AC7). Proven red: a seed cassette, a one-byte prompt edit, a train row copied from cc01, and a floor record without an interval each fail it.
2. **AC2 (command; exit line 3).** `MAOS_INFERENCE_MODE=replay maos eval halt --class butler` exits 0 on the promoted cassette with the calibrated floors, `format_errors: []` and `completions_consumed: 30`; `crates/maos-bin/tests/eval_halt_18_2.rs:124-128` asserts the promoted figures instead of the seed's 1.0/1.0/`seed`.
3. **AC3 — the prompt (D-28-2-01 A, D-28-2-02 A, D-28-2-03 A).** `proposal_prompt` renders `prompts/proposal-v1.txt` with the scenario and Butler's conflict list deterministically (byte-identical for the same input). DEFINITIONS: an unresolved conflict is an overlap `[start, end)` of two `confirmed` events, and only Butler's listed pairs are conflicts; `tentative` and `cancelled` events never conflict; touching or gapped events do not overlap; `user_preference_drift` is the director's alignment (1.0 = aligned), reported as `preference_alignment` when given, 1.0 when absent unless a comms message states a preference change. DECISION PROCEDURE: numbered rules mapping each case to `pattern` and drift. OUTPUT SCHEMA: exactly one JSON object, first character `{`, no code fence, the five `ButlerProposal` fields, the closed option vocabulary (`acceptance::check_options`, `lib.rs:190`).
4. **AC4 — the train split (D-28-2-04 A).** `calendar-comms-v0.3.train.jsonl` with a provenance header (hand-authored 2026-11-xx, purpose: few-shot boundary examples) covering tentative, cancelled, back-to-back and explicit-drift cases; the four examples in the prompt are drawn from it.
5. **AC5 — the live measurement (hosted, paid, on demand).** One 28-1 dispatch on `claude-haiku-5-5` with k per D-28-1-02; the `maos.eval.live/v1` result is kept in the evidence archive (B7a) and summarised in this file: per-metric mean/min/max, Wilson intervals, confusion by family, flip rows, format errors, tokens, cost. GA bar: precision and recall at or above the calibrated floors and `format_error_rate` 0.
6. **AC6 — calibration (ruling item 5; D-28-1-03).** `spirits/butler/eval/floors.toml` is written from AC5's result by `xtask eval-report --calibrate`; a floor below the NFR spec minimum (precision 0.85, recall 0.90) is not written — a measured shortfall after tuning opens an operator spec-change D-row carrying the measurement, interval and rationale.
7. **AC7 — promotion (D-28-2-05 A, D-28-2-06 A).** The selected live cassette replaces the seed; `the_seed_says_what_it_is` (`halt_eval_corpus_18_2.rs:121-127`) is replaced by AC1's provenance assertions; `generate()` (`:185-257`) derives the controls from the promoted cassette; `on-idle-halt.json`, `context-budget-18-5.json` and both controls carry v1 hashes; `corpus_pin.rs:28` and `tests/coverage-matrix.yaml:1489` are re-pinned (the rows' `prompt_sha256` move). `deferred-work.md:1413` (Butler half) is closed with this key.
8. **AC8 — reference result.** The AC5 result is the reference for future Haiku candidates (28-1 AC6); its archive path and SHA-256 are recorded in `floors.toml`.

## Tasks / Subtasks

- [ ] T0 Rulings D-28-2-01…06 recorded; 28-1 AC1/AC4/AC7 landed.
- [ ] T1 Train split + prompt file v1 + `proposal_prompt` rendering + prompt pin test (AC3, AC4).
- [ ] T2 Re-derive hashes for the seed scene/context cassettes and controls; corpus `prompt_sha256` fields via `generate()`; re-pin `corpus_pin.rs` and `coverage-matrix.yaml` (AC7).
- [ ] T3 Hermetic dry run on a hand-planted v1 cassette (replay green, controls red as designed).
- [ ] T4 Paid dispatch (`tier-2-rerecord`, `repeats` per D-28-1-02); archive the artifact; summarise here (AC5).
- [ ] T5 Calibrate and write `floors.toml`; promote the cassette; `halt_live_baseline_28_2.rs`; update `eval_halt_18_2.rs` (AC1, AC2, AC6, AC7, AC8).
- [ ] T6 If AC5 misses after tuning: stop and raise the spec-change D-row (no floor lowering in code).
- [ ] T7 Landing: `cargo fmt --all`; `check-kernel-baseline --json` (expect unchanged; if moved, `scripts/repin.py` + HISTORY row in the same commit); `kloc-check --json` (expect 0 charged); prepush.

## Dev Notes

- **Preserve:** the arithmetic conflict detector and `belief_variance` (`lib.rs:1673-1730`, `:2130-2151`); the strict parser (`:159-192`); the closed option vocabulary; the drift-blind control must still be detected (drift 0.99 on df rows, `halt_eval_corpus_18_2.rs:33`) and the precision-planted control must still miss precision.
- **No sampling control:** Haiku 5.5 rejects `temperature`; determinism comes from the prompt, the schema and replay (`anthropic.rs:99-107`).
- **One user message:** the driver sends the prompt as a single user turn (`anthropic.rs:139-145`); the prompt file carries all sections.
- **Tests and commands:** `cargo test -p butler`; `cargo test -p maos-bin --test eval_halt_18_2 --test butler_scene_18_4 --test context_budget_18_5 --test butler_inference_18_1`; `MAOS_INFERENCE_MODE=replay maos eval halt --class butler`.

### References

- Ruling `decisions-2026-11-15-spirit-quality.md:6,11-15`; epic `epic-28-spirit-prompts-and-evaluation.md`; protocol story `28-1-spirit-evaluation-protocol.md`.
- Evidence (outside git): `/home/lunarpulse/maos-evidence-archive/2026-11-15/live-rerecord-run-43708370483-cassettes/j-butler/halt-corpus-v0.3.live.json`.

## Dev Agent Record

### Agent Model Used

<set by dev at story start>

### Completion Notes List

### File List

## Change Log

- 2026-11-15: created (bmad-create-story, anthropic/claude-opus-5-5) at `bf3cba25`; D-28-2-01…06 PROPOSED.
- 2026-11-15: D-28-2-01…06 ACCEPTED (operator: proceed with recommendation). Statuses in the Decisions table above.
