---
story_id: '28.1'
story_key: 28-1-spirit-evaluation-protocol
created: 2026-11-15
context_commit: bf3cba25
baseline_commit: bf3cba25
security_sensitive: false
kernel_grant: "Not needed: kernel-Δ forecast 0 (method: read every touched file; none is under crates/maos-kernel-core/src). Growth inside the ADR-069 envelope would be self-service (ruling `kfree`, decisions-2026-11-12-dedrift.md:33; ADR-069 amendment 2026-11-12). The landing still measures with `check-kernel-baseline --json` and `kloc-check --json`; if the kernel moved, it re-pins with `scripts/repin.py` and writes the HISTORY row in the same commit."
kloc_grant: "Forecast (tokei, read-the-files estimate, no prototype): maos-bench +27…+49 (measured 2775, ceiling 2875); maos-eval +23…+40 (4680 / 4780); xtask +156…+260 (45504 / 46372); maos-bin 0 (38920 / 39699). A crate that breaches its ceiling gets one `# RAISED <date> 28-1-spirit-evaluation-protocol: <old> -> <new> (<reason>)` line above its row (kloc.toml:115-116)."
review: "bmad-code-review under docs/testing/proof-contract.md; author and reviewer from different model families. Creation anthropic/claude-opus-5-5."
---

# Story 28.1: The Spirit evaluation protocol

Status: ready-for-dev

<!-- Created 2026-11-15 at bf3cba25 by bmad-create-story under the lighter measured creation contract. Operator ruling: _bmad-output/planning-artifacts/decisions-2026-11-15-spirit-quality.md (rulings 2, 4, 5). Every cite was re-read at bf3cba25 (origin/main f9d5f3e3 is two CI-workflow commits ahead and touches none of the cited source files; it moves journey-nightly.yml's eval steps by +5 lines). The live-run evidence is outside git (ruling B7a): /home/lunarpulse/maos-evidence-archive/2026-11-15/live-rerecord-run-43708370483-{cassettes,job.log}. No gate and no cargo were run at creation; tokei 14.0.0 was run with the kloc-check exclusions (kloc_check.rs:287-311) for the headroom figures. D-28-1-01…10 are PROPOSED; the operator rules. -->

**Creation rationale — ready-for-dev at filing; gating D-rows resolved 2026-11-15.** Every AC is hermetic (committed fixtures, offline replay) except AC5, which changes the on-demand paid workflow and is proven by its first dispatch. **`v1.0.0` GA precondition** (ruling 2026-11-15, "Consequences recorded"; `RELEASE-HOLDS.md` GA-tag procedure item 1).

> **The capability:** *one small, repeatable way to say how well a Spirit's prompt works: a labelled corpus split into scenario families, an offline replay in CI that a prompt change cannot pass without a live re-record, k live repeats summarised with a mean and a Wilson interval, a reference-vs-candidate verdict, and floors that move only from measurements.*

## Story

As **the operator deciding whether a Spirit is good enough to ship on a given model**,
I want **one evaluation protocol — corpora with families and a hold-out, hash-bound replay, k live repeats with intervals, a model comparison and a floor-calibration rule — reusing `maos eval halt|distill`**,
so that **a prompt or model change is judged by measured numbers and a stated rule, not by a single run or by assertion**.

**Contract:** ruling 2026-11-15 items 2, 4, 5; epic `epic-28-spirit-prompts-and-evaluation.md` §28-1; E21 confidence rules 1–11. Create-story may tighten, not add uncited ACs.

## Inventory at HEAD — what exists and what is new

Method: every cite read at `bf3cba25`.

| Piece | Exists at HEAD | New in 28-1 |
|---|---|---|
| Labelled corpora | Butler `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` (30 rows; `expected_halt`, `calendar_conflict`, `prompt_sha256`, `completion {cassette, sequence}`; 15 expected halts); Researcher `spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl` (16 scenarios: rd01–rd12 clean, rd13–rd16 negative controls); Orchestrator `spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.jsonl` (30 rows) | Family rule (D-28-1-09) and hold-out rule (D-28-1-10); no test corpus changes |
| Replay scoring | `maos eval halt --class <c>` (`crates/maos-bench/src/halt_eval.rs:256-395`; CLI `crates/maos-bin/src/main.rs:1907-1939`); `maos eval distill --class researcher` (`crates/maos-bench/src/distill_eval.rs`); strict replay refuses a prompt-hash or length drift (`crates/maos-bin/src/cassette_replay.rs:151-152`); corpus/cassette hash binding tests (`spirits/butler/tests/halt_eval_corpus_18_2.rs:96-105`) | Per-row verdicts in the eval line; malformed-completion scoring (D-28-1-05); stale-cassette rule (AC3) |
| Scorers and floors | `score_class`, `missed_floors` (`crates/maos-eval/src/halt_replay.rs:49,112`); distill metrics (`crates/maos-eval/src/distill_replay.rs:404-444`); floor constants `HALT_RECALL_FLOOR` 0.90, `HALT_PRECISION_FLOOR` 0.85, `NFR_TEST_4_RECALL_FLOOR` 0.70 (`crates/maos-eval/src/onboarding_gate_corpus.rs:60-65`), NFR-Aud-7 floors (`distill_replay.rs:25-33`); class table (`halt_eval.rs:101-147`) | Floor record + calibration rule (D-28-1-03/04); the constants stay as the spec minimums |
| Signed result | `maos.benchmark.halt/v1` sidecar (`halt_replay.rs:152-178`); `maosctl spirit benchmark` verifies it (`crates/maos-cli/src/cli.rs:1001-1005`) | unchanged (one optional score field, AC4) |
| Live run | on-demand `tier-2-rerecord` job, k = 1, writes sibling `*.live.json` and uploads `refreshed-cassettes` (`.github/workflows/journey-nightly.yml:84-178`) | k repeats, every result line kept, `eval-report` run in the job (AC5) |
| Aggregation, intervals, comparison | none | `xtask eval-report` (AC1, AC6) |
| Cost | per-entry `usage` in cassettes; price book `xtask/provider-pricing.toml:21-37` (Haiku 5.5 100/500, Sonnet 5.5 2000/10000 micro-USD per 1k tokens) | tokens and cost in the result file (AC8) |
| Cassette age | `xtask cassette-age-gate` (`xtask/src/cassette_age_gate.rs:4`, 14 days; WARN unless `.tier2-first-success` exists, `:57-74`; the stamp is deliberately not created, `journey-nightly.yml:114-116`) | unchanged (D-28-1-08) |

**First live measurement (archive, run 43708370483; numbers re-derived at creation from the archived cassette and job log):** Butler on `claude-haiku-5-5`: TP 15, FP 8, FN 0; recall 1.0; precision 0.652, Wilson 95 % [0.449, 0.812] (n = 23 halts); 30 completions, 10 349 input / 8 157 output tokens ≈ 0.0051 USD at the price book's Haiku 5.5 row. Researcher on `claude-sonnet-5-5` (survey and judge): recall 1.0, faithfulness 1.0, hedge_preservation 17/18 = 0.944, Wilson 95 % [0.742, 0.990]; 58 recorded completions, 6 144 / 1 662 tokens ≈ 0.029 USD at the Sonnet 5.5 row. Wilson arithmetic: with zero errors, a lower bound ≥ 0.85 needs n ≥ 22, ≥ 0.90 needs n ≥ 35, ≥ 0.95 needs n ≥ 73 — the Butler corpus has 15 expected halts and the Researcher 18 hedges.

## Current-tree corrections — read before implementing

| Premise (assignment / ruling record) | Fact at `bf3cba25` |
|---|---|
| "reuse crates/maos-eval and the `maos eval ...` commands" | The `maos eval` compositions live in **`crates/maos-bench`** (`halt_eval.rs`, `distill_eval.rs`); `maos-eval` holds the kernel-free scorers and floor constants. Both are priced below. |
| Halt eval floor "0.85 (spirits/butler/tests/corpus_halt.rs …)" | `corpus_halt.rs:302-306` asserts 0.85 on the **arithmetic** self-trial; the live eval's floor is `HALT_PRECISION_FLOOR` (`onboarding_gate_corpus.rs:62`) via `halt_eval.rs:109`. The same constant is the NFR-Onb-1 cohort floor (`docs/research/nfr-onb-1-protocol.md:61`), so a calibrated class floor must not edit it (D-28-1-04). |
| "cassette bound to prompt hash … define the gate" | The binding exists (strict replay `cassette_replay.rs:151-152`; row/entry hash tests). What is missing is the model and provenance binding (AC3). |
| Haiku 4.5 diagnostic "malformed proposal" | `parse_proposal` (`spirits/butler/src/lib.rs:159-161`) refuses non-JSON; the refusal aborts the whole eval (`halt_eval.rs:593-595`), so the run yields no numbers at all (D-28-1-05). |

## Kernel and kloc pricing — forecast at creation

**Kernel:** forecast **0** (method: read the touched files — `crates/maos-bench/src/halt_eval.rs`, `crates/maos-eval/src/{halt_replay,onboarding_gate_corpus}.rs`, `xtask/src/main.rs`, new `xtask/src/eval_report.rs`, `.github/workflows/journey-nightly.yml`; none is kernel). No D-row is owed (below the 22 000 alarm; ruling `kfree`).

**kloc forecast** (tokei code; `[INFERENCE]` read-the-files estimate, no prototype; headroom measured with tokei 14.0.0 and the gate's exclusions):

| Crate · component | Forecast | Measured / ceiling (headroom) |
|---|---:|---|
| maos-bench · per-row verdicts in the halt line (+2…4); malformed completion scored per D-28-1-05 in `butler_pass`/`orchestrator_pass`/`model_verdicts` (+10…20); class floors read from the floor record (+15…25) | **+27…+49** | 2775 / 2875 (100) |
| maos-eval · `format_errors` in `HaltScore` + its missed-floor line (+8…15); `FloorRecord` type shared with xtask (+15…25) | **+23…+40** | 4680 / 4780 (100) |
| xtask · `eval_report.rs` (read k lines, families, Wilson, flip rate, cost, compare, calibrate) (+150…250); `Commands` entry (+6…10) | **+156…+260** | 45504 / 46372 (868) |
| maos-bin · none (`eval` dispatch unchanged; flags parse in maos-bench) | **0** | 38920 / 39699 (779) |
| tests, fixtures, workflow YAML, docs, floor-record TOML | uncharged | `-e tests`; non-Rust |

## Decisions — RULED or ACCEPTED (operator, 2026-11-15)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-28-1-01** | Pass rule and interval basis | **A** pass = mean over k repeats ≥ floor; Wilson 95 % over the pooled k×n decisions, reported · **B** pass = mean over k repeats ≥ floor; Wilson 95 % over the per-row majority decision (n = scenario rows), reported, not gated · **C** pass = Wilson lower bound (per-row majority) ≥ floor | **B.** The scenarios, not the repeats, are the sample: pooling repeats of the same 30 rows (A) narrows the interval without new evidence. C is infeasible on the committed corpora: precision ≥ 0.85 needs ≥ 22 halts with zero errors (Butler has 15). | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-02** | Repeats per live measurement | **A** k = 5 · **B** k = 3 · **C** k = 1 (today) | **A.** Measured cost per pass: Butler ≈ 0.0051 USD, Researcher ≈ 0.029 USD (archive tokens × `provider-pricing.toml`); k = 5 costs cents and makes the per-row flip rate (behaviour stability) measurable. | RULED k = 5 repeats per live run (option A; operator, 2026-11-15) |
| **D-28-1-03** | Floor calibration rule (ruling item 5) | **A** calibrated floor = max(spec minimum, Wilson lower bound of the tuned measurement rounded down to 0.01); a tuned mean below the spec minimum opens an operator spec-change D-row carrying the measurement, interval and rationale (never lowered by the story) · **B** floor = mean − 2·SD over the k repeats, clipped at the spec minimum · **C** floor = spec minimum always; a miss opens a spec-change D-row | **A.** Uses the measured distribution and its interval, raises a floor only on strong evidence, and keeps the PRD minimums (NFR-Test-4 `prd/non-functional-requirements.md:81`, NFR-Onb-1 `:125`, NFR-Aud-7 `:62-67`) unless the operator changes them. B's SD over 5 repeats is unstable; C never calibrates. | RULED A (operator, 2026-11-15): floor = max(spec minimum, Wilson lower bound of the measured baseline); a measurement below the spec minimum opens a spec-change D-row for the operator; floors are never lowered by assertion. |
| **D-28-1-04** | Where calibrated floors live | **A** per-class record `spirits/<class>/eval/floors.toml` (`maos.eval.floors/v1`: metric, spec minimum + source, measured mean, Wilson interval, k, model, prompt SHA-256, rule, rationale, date, result-file reference), `include_str!`-read by `halt_eval.rs` `CLASSES` like the manifest (`:113`); the `maos-eval` constants stay the spec minimums · **B** constants in `halt_eval.rs`/`distill_replay.rs` plus a markdown record · **C** an `[eval]` table in `manifest.toml` | **A.** One versioned file holds the number and its evidence; the NFR-Onb-1 cohort gate keeps its constant (`onboarding_gate_corpus.rs:60-62`). B lets number and record drift; C changes the manifest schema (`maos-manifest`) for test data. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-05** | A malformed completion during an evaluation | **A** score the row as its production consequence — a refused pass writes no scalar, so no halt (`spirits/butler/src/lib.rs:1090-1093,1109`) — and report `format_error_rate` with floor 0.0 (any malformed completion fails the run) · **B** score it as a wrong decision (FN or FP) plus the same rate · **C** keep today's abort (`halt_eval.rs:593-595`) | **A.** The metric reflects what the shipped Spirit does, the zero floor keeps the schema strict, and a candidate like Haiku 4.5 gets measured instead of stopping at row 1 (run 43713305197). | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-06** | Reference-vs-candidate tolerance (ruling item 2) | **A** candidate passes when every calibrated floor holds; differences reported only · **B** A + no metric worse than the reference by more than one decision (1/denominator) + `format_error_rate` 0 · **C** A + Wilson non-inferiority against the reference | **B.** Simple, deterministic and readable ("at most one more wrong row"); C's intervals at n ≈ 15–30 accept almost any candidate; A ignores the reference. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-07** | Cost/token budget per live run | **A** record tokens and cost in every result; `repeats` input bounded (≤ 10); `max_tokens` per call stays as coded (Butler 512, `lib.rs:1189`) · **B** A + a hard USD ceiling env var that stops the loop | **A.** Measured k = 5 cost is ≈ 0.03 USD (Butler) and ≈ 0.15 USD (Researcher); a ceiling adds code without a measured risk. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-08** | Cassette age for promoted live eval cassettes | **A** unchanged: `cassette-age-gate` stays WARN without the stamp; an eval cassette is bound by prompt hash, model id and provenance (AC3) · **B** exempt promoted eval cassettes from the age check · **C** require a re-record every 14 days | **A.** Age is not a correctness signal for a hash- and model-bound cassette; C needs a paid key and a human dispatch every two weeks (ruling 2026-11-12: re-records are on-demand). | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-09** | Scenario family encoding | **A** derived, no corpus change: halt corpora use the scenario-id prefix (leading letters: `cc`, `df`, `pb`, `tn`); the distill corpus uses the row's existing `control` field (`clean` when null), because every distill id starts `rd` · **B** an explicit `family` field in each corpus row | **A.** No corpus change: the 30-row Butler corpus is SHA-pinned (`spirits/butler/tests/corpus_pin.rs:28`) and is the NFR-Onb-1 corpus (`CORPUS_SCENARIO_COUNT` 30, `onboarding_gate_corpus.rs:68`); the distill corpus is pinned at `tests/corpora/MANIFEST.toml:168`. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-1-10** | Hold-out form | **A** a separate train file per corpus (`<corpus>.train.jsonl`) whose rows' `input` differs from every test row's, enforced by a per-class test; few-shot examples only from it · **B** a `split` field in the existing corpus | **A.** Leaves the test corpus, its pin and the NFR-Onb-1 count untouched; B re-pins the corpus and mixes train rows into the scored file. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |

## Acceptance Criteria

*(Clauses marked (D-…) assume the recommended option and change if the operator rules otherwise. **local** = hermetic on the dev host; **hosted** = the discipline run or the on-demand dispatch.)*

1. **AC1 (command; exit line 1).** `cargo test -p xtask --test spirit_eval_protocol_28_1` passes (local + hosted). It runs `xtask eval-report` on committed fixtures under `xtask/tests/fixtures/eval-report/` built from the archived live Butler run (30 per-row verdicts and its eval line, k = 1, plus a k = 3 variant) and asserts: precision mean 0.652; confusion by family cc 12 TP, df 3 TP, pb 4 FP / 2 TN, tn 4 FP / 5 TN; Wilson 95 % [0.449, 0.812]; tokens 10 349 / 8 157 and the cost from `xtask/provider-pricing.toml`. Proven red: flipping one fixture verdict changes the family counts and the interval; removing a repeat changes `k` and the flip rate.
2. **AC2 — families and hold-out (D-28-1-09 A, D-28-1-10 A).** `eval-report` derives the family per D-28-1-09 (halt: scenario-id prefix; distill: the row's `control`, `clean` when null). The protocol states: few-shot examples come only from `<corpus>.train.jsonl`; each class's baseline test (28-2 AC1, 28-3 AC1) refuses a train row whose `input` equals a test row's and a prompt file that contains a test row's input.
3. **AC3 — the stale-cassette rule.** A class cassette is stale when (a) any entry's `prompt_sha256`/`prompt_len` differs from the current prompt (exists: strict replay refuses, `cassette_replay.rs:151-152`), (b) its `model_id` differs from the class's manifest pin (`spirits/butler/manifest.toml` `provider.complete`), or (c) a GA class's cassette is not `provenance: live-record`. A prompt change therefore makes CI red until a live re-record is promoted. (b) and (c) are asserted by each class's baseline test.
4. **AC4 — per-row verdicts and malformed completions (D-28-1-05 A).** The `maos eval halt` stdout line gains `verdicts` (scenario id → halted) and `format_errors` (scenario ids); a malformed completion no longer aborts the run: the row scores as no halt and `format_error_rate` > 0 is a missed floor. The signed sidecar body keeps schema `maos.benchmark.halt/v1` with the new score field defaulted on read (`benchmark_sidecar_18_2.rs` literals updated). Proven red: a cassette entry replaced by fenced JSON yields `format_errors: ["cc01"]` and exit 1, where HEAD exits with "malformed proposal".
5. **AC5 — live protocol (D-28-1-02 A).** `journey-nightly.yml` `tier-2-rerecord` gains a `repeats` dispatch input (default 5, ≤ 10, D-28-1-07 A); each class step runs k times into `<cassette>.live.<i>.json`, appends every stdout line to `<class>.results.jsonl` without stopping on exit 1, runs `xtask eval-report` and uploads results with the cassettes. Hosted proof: the first dispatch after landing (owner `ops-nightly-cassette-rerecord`; the archive keeps the artifact, ruling B7a).
6. **AC6 — model comparison (D-28-1-06 B).** `xtask eval-report --reference <a.json> --candidate <b.json>` prints `maos.eval.compare/v1` (per-metric delta, rows whose majority decision changed, verdict) and exits 1 when the candidate misses a calibrated floor, has any format error, or is worse than the reference by more than one decision on any metric. The reference is the `claude-haiku-5-5` result 28-2 records.
7. **AC7 — floor calibration (D-28-1-03 A, D-28-1-04 A).** `eval-report --calibrate --spec <min>` prints the calibrated floor, interval and rule; `maos-bench` reads class floors from `spirits/<class>/eval/floors.toml`; with no record the class keeps today's constants. A floor below a spec minimum is refused unless the record cites an operator ruling.
8. **AC8 — cost.** The result file records input/output tokens summed from the cassettes and the USD cost from the price book; an unpriced model is an error, not 0.
9. **AC9 — record.** `docs/research/spirit-evaluation-protocol.md` (one page): corpus and families, hold-out, replay binding and staleness, k-repeat protocol, result schema, comparison rule, calibration rule, cost. `docs/research/nfr-onb-1-protocol.md` is unchanged.

`maos.eval.live/v1` (AC1/AC5): `class`, `eval` (`halt`|`distill`), `provider`, `model_id`, `prompt_version`, `prompt_sha256` (template), `corpus`, `corpus_sha256`, `k`, `runs[]` (`cassette_sha256`, workflow run id), `metrics{name: {mean, min, max, majority, n, wilson95: [lo, hi], floor, pass}}`, `confusion_by_family{family: {tp, fp, tn, fn}}` (majority decision), `flip_rows[]`, `flip_rate`, `format_errors{count, rows[]}`, `tokens{input, output}`, `cost_usd`, `price_book`.

## Tasks / Subtasks

- [ ] T0 Operator rulings on D-28-1-01…10 recorded in this file; re-measure headroom (`kloc-check --json`) at the dev base.
- [ ] T1 maos-eval: `format_errors` in `HaltScore` (serde default) and missed floor; `FloorRecord` type (AC4, AC7).
- [ ] T2 maos-bench: per-row `verdicts`/`format_errors` in the halt line; malformed completion scored per D-28-1-05 in `butler_pass`, `orchestrator_pass`, `model_verdicts`; floor record read (AC4, AC7). Update `crates/maos-bin/tests/eval_halt_18_2.rs`, `eval_halt_orchestrator.rs`, `crates/maos-cli/tests/benchmark_sidecar_18_2.rs` literals.
- [ ] T3 xtask `eval-report` (aggregate, families, Wilson, flip rate, cost, `--reference/--candidate`, `--calibrate`) and its test + fixtures (AC1, AC6, AC7, AC8).
- [ ] T4 `journey-nightly.yml` k-repeat loop, results file, `eval-report` step, upload (AC5); `xtask/tests/ci_shape_18_0.rs` if it pins the step text.
- [ ] T5 `docs/research/spirit-evaluation-protocol.md` (AC9).
- [ ] T6 Landing: `cargo fmt --all`; `check-kernel-baseline --json` (expect unchanged; if moved, `scripts/repin.py` + HISTORY row in the same commit); `kloc-check --json`; a breach gets one `RAISED` line (kloc.toml:115-116); prepush.

## Dev Notes

- **Preserve:** strict replay's hash refusal; the NFR-Onb-1 constants and cohort gate (`onboarding_gate_corpus.rs:52-73`); the Researcher halt class stays arithmetic (`halt_eval.rs:125-128`); the sidecar signature path (`halt_replay.rs:257-290`).
- **Record mode:** rows must reference the class cassette by name (`halt_eval.rs:508-514`); record writes `MAOS_REPLAY_CASSETTE` (`:198-201`), so k repeats are k invocations with k paths.
- **No new runtime surface:** no `maos` verb or flag is added; aggregation is offline in xtask.
- **Tests and commands:** `cargo test -p xtask --test spirit_eval_protocol_28_1`; `cargo test -p maos-bin --test eval_halt_18_2 --test eval_halt_orchestrator`; `cargo test -p maos-cli --test benchmark_sidecar_18_2`.

### References

- Ruling: `_bmad-output/planning-artifacts/decisions-2026-11-15-spirit-quality.md:10-15`.
- Epic: `_bmad-output/planning-artifacts/epics/epic-28-spirit-prompts-and-evaluation.md`.
- Evidence (outside git): `/home/lunarpulse/maos-evidence-archive/2026-11-15/live-rerecord-run-43708370483-job.log` lines 4466 (halt line), 4510-4511 (distill).

## Dev Agent Record

### Agent Model Used

<set by dev at story start>

### Completion Notes List

### File List

## Change Log

- 2026-11-15: created (bmad-create-story, anthropic/claude-opus-5-5) at `bf3cba25`; D-28-1-01…10 PROPOSED.
- 2026-11-15: D-28-1-02 and D-28-1-03 RULED (operator); D-28-1-01 and D-28-1-04…10 ACCEPTED (operator: proceed with recommendation). Statuses in the Decisions table above.
