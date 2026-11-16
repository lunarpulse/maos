---
story_id: '28.3'
story_key: 28-3-researcher-distill-prompt
created: 2026-11-15
context_commit: bf3cba25
baseline_commit: bf3cba25
security_sensitive: false
kernel_grant: "Not needed: kernel-Δ forecast 0 (method: read every touched file — spirits/researcher/**, crates/maos-bench/src/distill_eval.rs, crates/maos-eval/src/distill_replay.rs, crates/maos-journey-test/cassettes/j-researcher/*.json, tests/corpora/MANIFEST.toml; none is under crates/maos-kernel-core/src). Growth inside the ADR-069 envelope would be self-service (ruling `kfree`); the landing measures with `check-kernel-baseline --json` / `kloc-check --json` and, if the kernel moved, re-pins with `scripts/repin.py` and writes the HISTORY row in the same commit."
kloc_grant: "Forecast (tokei, read-the-files estimate): maos-bench +25…+55 (judge-only repeat mode per D-28-3-03 A +20…40; distill floors from the floor record +5…15); maos-eval +5…+15 (format-error count in the distill score); spirits/ uncharged. maos-bench headroom is shared with 28-1 (measured 2775 / ceiling 2875): if both land, a `# RAISED <date> 28-3-researcher-distill-prompt: <old> -> <new> (<reason>)` line is likely (kloc.toml:115-116)."
review: "bmad-code-review under docs/testing/proof-contract.md; author and reviewer from different model families. Creation anthropic/claude-opus-5-5."
---

# Story 28.3: The Researcher distill prompt

Status: ready-for-dev

<!-- Created 2026-11-15 at bf3cba25 by bmad-create-story under the lighter measured creation contract. Operator ruling: _bmad-output/planning-artifacts/decisions-2026-11-15-spirit-quality.md (rulings 1, 3, 5: "The Researcher live evaluation is handled by the same program"). Live evidence outside git (B7a): /home/lunarpulse/maos-evidence-archive/2026-11-15/live-rerecord-run-43708370483-cassettes/j-researcher/distill-corpus-v0.live{.json,.verdicts.json} and the job log lines 4510-4511. No cargo or gate was run. D-28-3-01…05 are PROPOSED. Starts after 28-1's rulings and AC1. -->

**Creation rationale — ready-for-dev at filing, after 28-1; gating D-rows resolved 2026-11-15.** GA-recommended, not a `v1.0.0` GA precondition (**D-28-3-01 RULED B**, operator, 2026-11-15); the ruling names only the Butler evaluation as a GA bar.

> **The capability:** *the Researcher's finding-synthesis prompt is a versioned file that copies every hedge verbatim and answers in a strict schema; on `claude-sonnet-5-5` its five NFR-Aud-7 metrics meet calibrated floors, and the judge's own variance is measured instead of assumed.*

## Story

As **the director reading a Researcher digest**,
I want **each finding to keep the source's hedges word for word and to contain nothing but the finding**,
so that **the digest never sounds more certain than its sources, and the five-metric gate measures that on the pinned model with a known judge variance**.

**Contract:** ruling items 1, 3, 5; epic §28-3; 28-1's protocol as ruled. Create-story may tighten, not add uncited ACs.

## Inventory at HEAD (`bf3cba25`)

- **Prompt:** `synthesis_prompt` — "Summarize this research claim in one sentence for a digest. Preserve any hedges; add no facts.\n\nClaim: {statement}" (`spirits/researcher/src/lib.rs:570-575`); one call per claim, `max_tokens` 256, `temperature: Some(0.0)` requested (`:1297-1321`) and omitted on the wire for `claude-sonnet-5-5` (`crates/maos-providers/src/anthropic.rs:105-107`); the text is bounded by `summarize` (`lib.rs:1302`). The claim's `hedges` list exists (`lib.rs:263-265`) but is not given to the model.
- **Metrics:** recall, hedge preservation, traceability, secret leakage are deterministic string facts; faithfulness is the judge's (`crates/maos-eval/src/distill_replay.rs:3-15`, `metrics` `:404-444`). Hedge preservation is a **case-insensitive whole-word match** of each source hedge in the finding (`keeps_hedge`, `:446-456`). Floors: NFR-Aud-7 constants (`:25-33`; `prd/non-functional-requirements.md:62-67`).
- **Judge:** `JUDGE_PROMPT` (`distill_replay.rs:45-47`), one word `faithful`/`unfaithful` (`judge_verdict`, `:83-93`), pinned in `tests/judge-config.toml` `[judge.researcher-faithfulness]`; Sonnet 5.5 rejects `temperature` (`judge_temperature_applied: false` in the live line).
- **Corpus:** `spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl`, 16 scenarios (rd01–rd12 clean, rd13 hedge-drop, rd14 fabricated-cite, rd15 planted-secret, rd16 dropped-claim); each row carries a frozen `survey` a replay must reproduce byte-for-byte and frozen judge verdicts (`distill_replay.rs:162-179`); SHA-pinned at `tests/corpora/MANIFEST.toml:168`. The committed cassette `distill-corpus-v0.json` (33 entries, `provenance: seed`) is the only cassette whose hashes match `synthesis_prompt` over corpus statements (measured at creation: 33 of 33; `survey-distill.json`, `forget-redistill-18-6.json`, `skill-authoring-18-7.json` match 0 corpus-statement prompts — whether they bind `synthesis_prompt` for other claims is `[INFERENCE]` unverified; the dev confirms by replay).

## Live measurement 2026-11-15 (run 43708370483, `claude-sonnet-5-5` survey and judge, k = 1)

Recall 1.0, faithfulness 1.0, traceability 1.0, secret leakage 0.0, **hedge preservation 17/18 = 0.944** (Wilson 95 % [0.742, 0.990]) against 0.95; all five negative controls detected; 58 recorded completions (29 survey, 29 judge), 6 144 input / 1 662 output tokens ≈ 0.029 USD at the price book's Sonnet 5.5 row. **The one miss:** `rd05-c1`, source hedge "seem to", model wrote "Ensembles of small judges **appear to** match a single large judge …" — a synonym, which the whole-word metric does not accept. **11 of the 29 survey completions carry meta-text** around the finding (a preamble or a second paragraph, e.g. sequence 1, `rd01-c2`: "The claim itself contains no hedging language, so here is a faithful one-sentence summary: … If you have the original passage …"); `summarize` bounds them, so the metrics did not move, but the output is not the schema the prompt asked for. With 18 hedges, any single miss fails the 0.95 floor (17/18 = 0.944).

## Current-tree corrections — read before implementing

| Premise (assignment) | Fact at `bf3cba25` |
|---|---|
| "hedge_preservation 0.944 … with the Sonnet judge; note judge variance" | OLD → NEW: hedge preservation is not judged; it is `keeps_hedge` (`distill_replay.rs:446-456`). The judge decides **faithfulness** only (`:424-430`). Judge variance therefore bears on faithfulness (1.0 measured, floor 0.98), not on the 0.944. |
| "the job run uses corpus spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl" | Confirmed (job log line 4511, `"corpus": "spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl"`, `corpus_sha256` 6f69f2e6…, equal to `MANIFEST.toml:168`). |
| `maos eval distill` lives in `crates/maos-eval` | The composition is `crates/maos-bench/src/distill_eval.rs`; the scorer is `crates/maos-eval/src/distill_replay.rs`. |

## Kernel and kloc pricing — forecast at creation

**Kernel:** forecast **0** (method: read the touched files listed in `kernel_grant`); no D-row owed. **kloc:** see `kloc_grant` (maos-bench +25…+55, maos-eval +5…+15; `[INFERENCE]` estimates, no prototype); `spirits/` excluded (`xtask/src/kloc_check.rs:303-304`).

## Decisions — RULED or ACCEPTED (operator, 2026-11-15)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-28-3-01** | Is 28-3 a `v1.0.0` GA precondition? (operator must rule) | **A** yes, beside 28-1 and 28-2 · **B** no: GA-recommended, lands before GA when ready, not a gate | **B.** The ruling names the Butler evaluation as the GA bar and says the Researcher "is handled by the same program" (`decisions-2026-11-15-spirit-quality.md:11`); the measured gap is one synonym in 18 hedges, and the 0.95 floor sits inside the interval [0.742, 0.990]. A is defensible if the operator wants NFR-Aud-7 proven live at GA. | RULED B (operator, 2026-11-15): not a `v1.0.0` GA precondition; GA-recommended only. 28-1 and 28-2 are the GA preconditions. |
| **D-28-3-02** | Fix locus for the hedge miss | **A** prompt: pass the claim's `hedges` list and require each phrase copied verbatim · **B** metric: accept a hedge-equivalence lexicon ("seem to" ≈ "appear to") · **C** A and B | **A.** Keeps the metric deterministic and literal (NFR-Aud-7 unchanged); the hedge list already exists on the claim (`lib.rs:263-265`). B needs a curated lexicon whose completeness nobody can measure. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-3-03** | How judge variance is measured | **A** a judge-only repeat: replay one recorded survey and call the live judge k times on the same findings; report per-finding agreement and the faithfulness mean/min (maos-bench +20…40) · **B** infer agreement from k full repeats on findings whose text is identical across repeats (xtask only) · **C** not measured | **A.** Isolates the judge from survey variance; B has few comparable findings when Sonnet rephrases. Cost per judge pass ≈ 29 calls (live line `judge_calls` 29). | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-3-04** | Strict output schema for a finding | **A** prompt asks for the sentence only; no format check · **B** a deterministic one-sentence/no-meta-text check counted as a format error · **C** one JSON object `{"finding": string}` parsed strictly; a malformed completion is a format error (28-1 D-28-1-05) | **C.** Ruling 3 asks for a strict output schema; 11 of 29 live completions ignored the free-text instruction; a JSON field is checkable without heuristics. `summarize` (`lib.rs:1302`) still bounds the text. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-3-05** | Promotion | **A** the median-scoring live run replaces `distill-corpus-v0.json` (`live-record` layout, `distill_replay.rs:231-243`); frozen surveys and verdicts re-pinned in the corpus (`journey-nightly.yml:155-161` procedure); completion-planted controls stay served from the committed entries (`:144-150`) · **B** keep the seed in CI, commit the live cassette beside it | **A.** Same reason as D-28-2-05; closes the Researcher half of the promotion owed under `ops-nightly-cassette-rerecord`. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |

## Acceptance Criteria

1. **AC1 (command; exit line 4).** `cargo test -p researcher --test distill_live_baseline_28_3` passes: the class cassette is `live-record` with `model_id` `claude-sonnet-5-5` (the manifest's first pin, `spirits/researcher/manifest.toml:49`); every survey entry hash equals the v1 synthesis prompt for its claim; the prompt file SHA-256 is pinned; train/test disjointness per 28-1 AC2; `spirits/researcher/eval/floors.toml` complete (measurement, interval, k, model, judge model, judge agreement, rule, rationale). Proven red: seed cassette; one-byte prompt edit; floor record without the judge agreement.
2. **AC2 (command; exit line 5).** `MAOS_INFERENCE_MODE=replay maos eval distill --class researcher` exits 0 on the promoted cassette at the calibrated floors with every control detected and no format error; `crates/maos-bin/tests/eval_distill_18_3.rs` assertions follow the promoted figures.
3. **AC3 — the prompt (D-28-3-02 A, D-28-3-04 C).** `spirits/researcher/prompts/synthesis-v1.txt` (fixed sections: ROLE, OBJECTIVE, RULES — copy each listed hedge verbatim, add no fact, no meta-text — OUTPUT SCHEMA, EXAMPLES from a train file, INPUT: statement and hedge list); `SYNTHESIS_PROMPT_VERSION`; `synthesis_prompt` renders it byte-identically for the same claim.
4. **AC4 — train split.** `spirits/researcher/tests/fixtures/distill-corpus-v0.train.jsonl` (hand-authored, provenance header; hedged claims with synonym traps such as "seem to"/"appear to"), disjoint from the 16 test scenarios.
5. **AC5 — live measurement (hosted, paid).** One 28-1 dispatch with `include_researcher=true` and k per D-28-1-02, plus the judge-only repeat (D-28-3-03 A); the `maos.eval.live/v1` result (families: the row's `control`, `clean` when null) and the judge agreement are archived (B7a) and summarised here.
6. **AC6 — calibration.** Floors per D-28-1-03 from AC5; NFR-Aud-7 minimums (recall 0.90, faithfulness 0.98, hedge 0.95, traceability 1.0, leakage 0.0) are not lowered by this story — a shortfall after tuning opens an operator spec-change D-row with measurement, interval and rationale.
7. **AC7 — promotion (D-28-3-05 A).** Cassette promoted; corpus frozen surveys and verdicts re-pinned; `tests/corpora/MANIFEST.toml:168` SHA updated; `deferred-work.md:1413`'s Researcher half closed with this key.

- [x] T0 Rulings D-28-3-01…05 recorded (D-28-3-01 RULED B: not a GA precondition, so `RELEASE-HOLDS.md` GA-tag item 1 and the sprint-status GA comment do not change); 28-1 landed.
- [x] T1 Prompt file v1 + JSON finding parse + train split + prompt pin test (AC3, AC4) — done 2026-11-16, commit `4e293036`, all researcher tests green.
- [~] T2 Judge-only repeat mode and distill floors from the record (D-28-3-03, AC6); format-error count in the distill score — scorer half done (commit `beb63d2e`); judge-only repeat mode remains.
- [x] T3 Hermetic dry run on a v1 cassette (controls still detected) — done: generators ran, all five controls detected in every hermetic run (`eval_distill_18_3` 6/6).
- [ ] T4 Paid dispatch; archive; summarise (AC5).
- [ ] T5 Calibrate, promote, re-pin, `distill_live_baseline_28_3.rs`, `eval_distill_18_3.rs` updates (AC1, AC2, AC6, AC7).
- [ ] T6 Landing: `cargo fmt --all`; `check-kernel-baseline --json` (expect unchanged; if moved, `scripts/repin.py` + HISTORY row in the same commit); `kloc-check --json`; `RAISED` line if maos-bench breaches; prepush.
## Dev Notes

- **Preserve:** every negative control must still miss its metric (`distill_replay.rs:500-512`); completion-planted controls are served from committed entries in a live replay (`:144-150`, `:225-229`); the judge prompt and its pin are unchanged unless a D-row rules otherwise (a change re-records every verdict).
- **Hedge copy rule:** `keeps_hedge` is case-insensitive and whole-word; the prompt asks for the exact phrase, not a paraphrase.
- **Tests and commands:** `cargo test -p researcher`; `cargo test -p maos-bin --test eval_distill_18_3 --test researcher_inference_18_3`; `MAOS_INFERENCE_MODE=replay maos eval distill --class researcher`.

### References

- Ruling `decisions-2026-11-15-spirit-quality.md:7,11-15`; epic `epic-28-spirit-prompts-and-evaluation.md`; protocol `28-1-spirit-evaluation-protocol.md`.

## Dev Agent Record

### Agent Model Used

<set by dev at story start> 2026-11-16: Dev283 (coder/claude-sonnet-5-5) was dispatched and died on
an Anthropic 429 rate limit before any edit; the orchestrator (zai/glm-5.3) wrote the T1 slice
directly, per the session disclosure.

### Completion Notes List

- 2026-11-16: Dev283 (coder/claude-sonnet-5-5) died on an Anthropic 429 before any edit; the
  orchestrator (zai/glm-5.3) wrote the T1 slice directly (disclosed).
- `spirits/researcher/prompts/synthesis-v1.txt` (sections ROLE, OBJECTIVE, RULES, OUTPUT SCHEMA
  `{"finding": string}` per D-28-3-04 C, EXAMPLES = the four `example` train rows, INPUT with
  `{{statement}}`/`{{hedges}}`), pinned `5cf08b16…`.
- `spirits/researcher/tests/fixtures/distill-corpus-v0.train.jsonl`: provenance header + tr01–tr08;
  tr02/tr05 are the "seem to"/"appear to" synonym traps; disjoint from rd01–rd16 by id and statement.
- `spirits/researcher/src/lib.rs`: `SYNTHESIS_PROMPT_VERSION` "researcher-synthesis/v1",
  `SYNTHESIS_PROMPT_TEMPLATE` include_str, `synthesis_prompt(statement, hedges)` one-pass render,
  `survey_with` closure receives the claim's hedges (D-28-3-02 A), `survey_bound` parses the
  completion as the strict JSON object and falls back to the bounded deterministic summary when
  malformed (the eval scorer, not the Spirit, counts format errors).
- `spirits/researcher/tests/inference_seam_18_3.rs` call sites updated; new
  `spirits/researcher/tests/prompt_28_3.rs` + `tests/common/mod.rs` (pin, section order,
  byte-identical render, train/test disjointness, examples-verbatim, env-gated cassette generator).
- Generator mapping measured: cassette entries 0..32 are the corpus claims in row order skipping each
  row's `withheld` ids (33/33 old-prompt hash matches; `rd16-dropped-claim-c3` is the withheld one).
- UNVERIFIED: nothing compiled or tested; the cassette still pins the OLD prompt hashes until
  `generate` runs; `five_metric_live_8_11.rs`, `distillation_i11.rs`, `inference_seam_8_11.rs` and
  `crates/maos-bin/tests/eval_distill_18_3.rs` not yet audited for old-prompt pins.


2026-11-16 (commit `beb63d2e`, orchestrator/GLM): `distill_replay.rs` gained `FiveMetrics.format_errors`
(floor 0 over SURVEY entries only — judge replies excluded), `is_finding_completion`, `DistillFloors`/
`DistillFloorRecord::apply` (same rule as 28-1; `secret_leakage` stays the compiled zero-tolerance
ceiling — a Wilson bound has no meaning against 0.0), `missed_floors_with`/`gate_failures_with`;
`distill_eval.rs` reads `spirits/researcher/eval/floors.toml` when present (spec minimums otherwise;
parse/apply failure is an error, never a silent fallback) and the line carries `floors`. Seed
cassettes regenerated via the idempotent generators (distill-corpus-v0's completions are now
v1-schema `{"finding": <old text>}` — frozen surveys byte-identical; forget-redistill-18-6 and
skill-authoring-18-7 hash/len rewrites; `journey_forget_18_6.rs` pins updated; the eval_distill
stub answers as a compliant v1 model). kloc maos-eval 4780 -> 4903 (RAISED line). Verified: 58
binaries green across researcher/maos-eval/maos-bench/maos-journey-test; maos-bin eval_distill_18_3
6/6; replay smoke exit 0 with format_errors 0 at spec floors. REMAINING in T2: the judge-only
repeat mode needs a live router for the judge leg — belongs with the T4 nightly dispatch wiring.
### File List

- `spirits/researcher/prompts/synthesis-v1.txt` (new)
- `spirits/researcher/tests/fixtures/distill-corpus-v0.train.jsonl` (new)
- `spirits/researcher/src/lib.rs` (SYNTHESIS_PROMPT_VERSION/TEMPLATE, synthesis_prompt(statement, hedges), survey_with/survey_bound)
- `spirits/researcher/tests/inference_seam_18_3.rs` (call sites)
- `spirits/researcher/tests/prompt_28_3.rs`, `spirits/researcher/tests/common/mod.rs` (new)

## Change Log

- 2026-11-15: created (bmad-create-story, anthropic/claude-opus-5-5) at `bf3cba25`; D-28-3-01…05 PROPOSED.
- 2026-11-15: D-28-3-01 RULED B (operator; not a GA precondition, GA-recommended only); D-28-3-02…05 ACCEPTED (operator: proceed with recommendation). Statuses in the Decisions table above.
- 2026-11-16: T1 slice + generator written by the orchestrator (Anthropic quota exhausted; disclosed); unverified.
