# Spirit evaluation protocol — how a prompt or a model is judged

One small, repeatable method (Story 28-1; operator ruling 2026-11-15, items 2, 4, 5): a labelled corpus split
into families, an offline replay in CI that a prompt change cannot pass without a live re-record, k live
repeats summarised with a mean and a Wilson interval, a reference-vs-candidate verdict, and floors that move
only from measurements. It reuses `maos eval halt|distill`; aggregation is offline (`xtask eval-report`).
`docs/research/nfr-onb-1-protocol.md` (the human cohort gate and its constants) is unchanged.

## 1. Corpus, families, hold-out

- **Corpus.** One JSONL file per class (Butler `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`, Orchestrator
  `…/halt-corpus-v0.1.jsonl`, Researcher `…/distill-corpus-v0.jsonl`); rows carry the label (`expected_halt`) and,
  for model-driven classes, the completion they replay. Corpora are SHA-pinned and are not edited by tuning.
- **Family** (derived, no corpus change; D-28-1-09 A): a halt row's family is the leading letters of its
  `scenario_id` (Butler: `cc`, `df`, `pb`, `tn`); a distill row's is its
  `control` (`clean` when null). `eval-report` prints the confusion matrix per family, so a regression names the
  scenario kind it came from.
- **Hold-out** (D-28-1-10 A): few-shot examples in a prompt come only from `<corpus>.train.jsonl`. Each class's baseline
  test (28-2 AC1, 28-3 AC1) refuses a train row whose `input` equals a test row's and a prompt file that contains a
  test row's input.

## 2. Replay binding and staleness

Offline replay is strict: every cassette entry carries the prompt's SHA-256 and length, and a drift refuses the run.
A class cassette is **stale** when (a) any entry's `prompt_sha256`/`prompt_len` differs from the current prompt, (b) its
`model_id` differs from the class's manifest pin (`provider.complete`), or (c) a GA class's cassette is not
`provenance: live-record`. A prompt change therefore turns CI red until a live re-record is promoted. Age is not a
correctness signal for a hash- and model-bound cassette: `cassette-age-gate` is unchanged (D-28-1-08 A).

## 3. The k-repeat live protocol

The on-demand `tier-2-rerecord` job (`journey-nightly.yml`; dispatch with `rerecord=true`) runs each class `repeats`
times (input, 1-10, default 5; D-28-1-02 A). Repeat *i* records `<cassette>.live.<i>.json` and appends its one stdout
line to `eval-results/<class>.results.jsonl`; exit 1 (a missed floor) is a measured result and the loop goes on. Then
`xtask eval-report --results … --repeats k --run-id <run>` writes `<class>.report.json`; the job uploads cassettes
and results together. A malformed completion no longer aborts a run: the row scores as **no halt** (what the shipped
Spirit does) and is listed in the line's `format_errors`; `format_error_rate` has floor 0 (D-28-1-05 A).
The line also carries `verdicts` (scenario id → halted).

## 4. The result: `maos.eval.live/v1`

`class`, `eval` (`halt`|`distill`), `provider`, `model_id`, `prompt_version` (the line's `spirit_version`),
`prompt_sha256` (a digest of the run-1 cassette's per-entry prompt hashes, in order — not the hash of the prompt
template, which xtask cannot see), `corpus`, `corpus_sha256`, `corpus_families`, `k`, `runs[]`
(`cassette_sha256`, workflow run id), `metrics{name: {mean, min, max, majority, n, wilson95, floor, direction, pass}}`,
`confusion_by_family{family: {tp, fp, tn, fn}}`, `majority_verdicts`, `flip_rows[]`, `flip_rate`,
`format_errors{count, rows}`, `tokens{input, output}`, `cost_usd`, `price_book`, `failures[]`, `pass`.

- **Pass rule** (D-28-1-01 B): the *mean over the k repeats* meets the floor (`format_error_rate`: mean ≤ 0).
- **Interval:** Wilson 95 % over the per-row *majority* decision (a row halts when more than half of the repeats halted):
  the scenarios, not the repeats, are the sample. It is reported, not gated; with zero errors a lower bound ≥ 0.85 needs
  n ≥ 22, ≥ 0.90 needs n ≥ 35.
- **Flip rate:** the share of rows whose decision differs between repeats (behaviour stability).
- **Integrity:** the corpus and every cassette must match the SHA-256 in the result lines; each line's TP/FP/FN must
  match its own verdicts. `maos eval distill` lines carry metric values, not per-row decisions, so its report has
  mean/min/max against the NFR-Aud-7 constants, no interval and no confusion matrix; its `n` is derived from the
  corpus (the clean rows' 25 claims, and their 18 hedges for hedge preservation; findings are one per claim). A
  numeric miss is judged on the mean; a structural failure of *any* repeat (stale judge prompt, changed frozen
  survey, writer/cite disagreement) or an undetected negative control fails the aggregate (`failures[]`).

## 5. Comparing a model (D-28-1-06 B)

`xtask eval-report --reference a.json --candidate b.json` prints `maos.eval.compare/v1` (per-metric delta, rows whose
majority decision changed, verdict) and exits 1 when the candidate has any aggregate failure of its own (a missed floor,
a malformed completion, an undetected control, a structural failure) or any metric is worse than the reference by more
than one decision, 1 / the reference's `n` (halt: the decision count of the metric; distill: the corpus-derived `n`
above, e.g. recall 24/25 against 1.0 passes, 23/25 does not). The reference is the `claude-haiku-5-5` result 28-2
records; a later Haiku is judged against it with the same command.

## 6. Floors move only from measurements (D-28-1-03 A, D-28-1-04 A)

`xtask eval-report --calibrate --spec <minimum> --metric <name> --result <live.json>` prints
`floor = max(spec minimum, Wilson lower bound of the measured baseline, rounded down to 0.01)`. A measured mean below the
spec minimum exits 1 and asks for an operator spec-change D-row carrying the measurement, interval and rationale; a floor
is never lowered by assertion. The result is recorded in `spirits/<class>/eval/floors.toml` (`maos.eval.floors/v1`: per
metric `floor`, `spec_minimum`, `spec_source`, `measured_mean`, `wilson95`, `k`, `model_id`, `prompt_sha256`, `rule`,
`rationale`, `date`, `result_file`, optional `operator_ruling`). `maos eval halt` reads that file when it exists, else the
spec-minimum constants (`maos-eval`: NFR-Test-4, NFR-Onb-1, NFR-Aud-7). The rule is enforced at load: a floor below
the larger of its spec minimum, the compiled constant and the record's own Wilson lower bound (rounded down to 0.01) is
refused unless the record cites an operator ruling; a hand-edited floor therefore cannot go below what the stored
measurement supports.

## 7. Cost (D-28-1-07 A)

Input and output tokens are summed from the cassettes' per-entry `usage` and priced per (provider, model) from
`xtask/provider-pricing.toml`; a model the price book lacks is an error, not 0. First measurement: Butler on
`claude-haiku-5-5`, 30 completions, 10 349 / 8 157 tokens ≈ 0.0051 USD per pass; k = 5 costs cents.
