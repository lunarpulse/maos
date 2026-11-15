# Epic 28 — Spirit prompts and evaluation

**Status:** `backlog` — filed 2026-11-15 by operator ruling (`_bmad-output/planning-artifacts/decisions-2026-11-15-spirit-quality.md`, rulings 1–5), after the first end-to-end live measurements of the Butler halt evaluation and the Researcher distill evaluation (journey-nightly run 43708370483; evidence outside git, ruling B7a: `/home/lunarpulse/maos-evidence-archive/2026-11-15/`). `sprint-status.yaml` is authoritative for status.

**Wave:** none. The W-labels name sequential lanes (W0–W6 recovery, W7–W11 Enterprise-Fit after `v1.0.0`); this epic runs before GA beside the GA set and belongs to neither lane (the same reason Epic 27 carries no wave). · **Makes true:** each model-driven Spirit's prompt is a versioned, hash-bound file with fixed sections, explicit decision rules and a strict output schema; its behaviour is scored by one simple method (labelled corpus with scenario families, offline replay in CI, k live repeats with Wilson intervals, a reference-vs-candidate comparison, floors calibrated from measurements); and the Butler live evaluation passes its calibrated floor on `claude-haiku-5-5` before GA.

**Closes:** ruling 1 (prompt-adjustment story; Butler live evaluation passes before GA) → `28-2`; ruling 2 (`claude-haiku-5-5` is the reference model; later Haiku compared by the same protocol) → `28-1` AC6, `28-2`; ruling 3 (context-engineered, deterministic prompts) → `28-2`, `28-3`, `28-4`; ruling 4 (simple and effective evaluation) → `28-1`; ruling 5 (floors calibrated from measurements, not lowered by assertion) → `28-1` AC7, applied in `28-2` and `28-3`. `28-1` and `28-2` are `v1.0.0` GA preconditions (ruling record "Consequences"; `RELEASE-HOLDS.md` GA-tag procedure item 1).

**Hermetic exit command (CI, no secrets; offline replay of committed cassettes; red until green):**

```
cargo test -p xtask --test spirit_eval_protocol_28_1   # 1 — eval-report over committed result fixtures: per-family confusion, Wilson intervals, reference-vs-candidate verdict, floor calibration rule (28-1 AC1)
cargo test -p butler --test halt_live_baseline_28_2   # 2 — the Butler class cassette is a live-record of claude-haiku-5-5 bound to the versioned prompt; train and test rows are disjoint; the calibrated floor record is complete (28-2 AC1)
MAOS_INFERENCE_MODE=replay maos eval halt --class butler   # 3 — offline replay of that cassette meets the calibrated Butler floors (28-2 AC2)
cargo test -p researcher --test distill_live_baseline_28_3   # 4 — the Researcher distill cassette is a live-record of claude-sonnet-5-5 bound to the versioned synthesis prompt, with measured judge agreement recorded (28-3 AC1)
MAOS_INFERENCE_MODE=replay maos eval distill --class researcher   # 5 — offline replay meets the calibrated NFR-Aud-7 floors (28-3 AC2)
```

**Exit-block provenance (measured at worktree HEAD `bf3cba25`; `origin/main` is `f9d5f3e3`, two CI-workflow commits ahead, which touch none of the cited source files):**
- Line 1 — `xtask/tests/spirit_eval_protocol_28_1.rs` is created by 28-1 AC1. **HEAD: red** (no such test target).
- Line 2 — `spirits/butler/tests/halt_live_baseline_28_2.rs` is created by 28-2 AC1. **HEAD: red** (no such test target; the committed class cassette is `provenance: seed`, `model_id: hand-authored-seed`, asserted by `spirits/butler/tests/halt_eval_corpus_18_2.rs:121-127`).
- Line 3 — `maos eval halt` exists (`crates/maos-bin/src/main.rs:1907-1939`, `crates/maos-bench/src/halt_eval.rs:164-207`; `MAOS_INFERENCE_MODE=replay` is accepted at `:187`). **HEAD: green on the hand-authored seed** (precision 1.0, asserted by `crates/maos-bin/tests/eval_halt_18_2.rs:124-128`), which proves nothing about a model; the line becomes meaningful when line 2 is green (28-2 AC2 replays the promoted live cassette).
- Line 4 — `spirits/researcher/tests/distill_live_baseline_28_3.rs` is created by 28-3 AC1. **HEAD: red** (no such test target).
- Line 5 — `maos eval distill` exists (`crates/maos-bin/src/main.rs:1912-1925`, `crates/maos-bench/src/distill_eval.rs:86`). **HEAD: green on the seed** (frozen verdicts, `provenance: seed`); meaningful when line 4 is green (28-3 AC2).
- Shape: one command per line with a trailing `#` comment; no secret, key, human, wall-clock or network (strict replay checks every entry's prompt SHA-256, `crates/maos-bin/src/cassette_replay.rs:151-152`). `28-4` adds one line per Spirit when it leaves `backlog`.

**Kernel-Δ:** forecast 0 for all four stories (method: every touched file was read — `spirits/butler`, `spirits/researcher`, `crates/maos-bench/src/{halt_eval,distill_eval}.rs`, `crates/maos-eval/src/{halt_replay,distill_replay}.rs`, `xtask/src`, `.github/workflows/journey-nightly.yml`, committed cassettes; none is under `crates/maos-kernel-core/src`). Kernel growth would be self-service inside the ADR-069 envelope (ruling `kfree`, `decisions-2026-11-12-dedrift.md:33`); each landing still measures with `check-kernel-baseline` / `kloc-check --json`.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

---

## Stories

| Key | Story | Closes · state |
|---|---|---|
| `28-1-spirit-evaluation-protocol` | The simple evaluation method: families + hold-out, hash-bound replay, k live repeats with Wilson intervals, model comparison, floor calibration, cost record | rulings 2, 4, 5 · `ready-for-dev` (gating D-rows D-28-1-01…10 resolved 2026-11-15); **`v1.0.0` GA precondition** |
| `28-2-butler-halt-prompt` | Context-engineered Butler proposal prompt; live measurement on `claude-haiku-5-5`; calibrated floor; promoted live cassette | rulings 1, 2, 3, 5; NFR-Test-4, NFR-Onb-1 (Butler class) · `ready-for-dev` (after 28-1; D-28-2-01…06 resolved 2026-11-15); **`v1.0.0` GA precondition** |
| `28-3-researcher-distill-prompt` | Same treatment for the Researcher synthesis prompt; judge variance measured | rulings 1, 3, 5; NFR-Aud-7 · `ready-for-dev` (after 28-1; D-28-3-01…05 resolved 2026-11-15); GA-recommended, not a GA precondition (D-28-3-01 RULED B) |
| `28-4-other-spirits-prompts-and-evals` | Inventory and order for orchestrator, hello-spirit, observer, mira, nash, architect, reviewer, worker; each gets the 28-1 method as ruled in D-28-4-01…05 | ruling 3 · `backlog`; not GA |

Story files: `_bmad-output/implementation-artifacts/28-1-spirit-evaluation-protocol.md`, `28-2-butler-halt-prompt.md`, `28-3-researcher-distill-prompt.md`, `28-4-other-spirits-prompts-and-evals.md`. `bmad-create-story` re-measured every cite at `bf3cba25`; the stories may tighten, not add uncited ACs.

### 28-1-spirit-evaluation-protocol — The Spirit evaluation protocol

*Closes · Δ:* rulings 2, 4, 5 · kernel-Δ forecast 0 · **`v1.0.0` GA precondition** (ruling 2026-11-15).

- *HEAD at `bf3cba25`:* `maos eval halt` (`crates/maos-bench/src/halt_eval.rs`) and `maos eval distill` (`crates/maos-bench/src/distill_eval.rs`) score one pass and print one JSON line; the halt line carries no per-row verdicts (`halt_eval.rs:360-386`); a malformed completion aborts the whole run (`halt_eval.rs:593-595`); live runs are the on-demand `tier-2-rerecord` job, k = 1 (`.github/workflows/journey-nightly.yml:122-168`).
- **AC1 (command).** `xtask eval-report` aggregates the k result lines of a live measurement into the `maos.eval.live/v1` file (per-metric mean, min, max and Wilson 95 % interval; confusion matrix by scenario family; per-row flip rate; format-error rate; tokens and cost) and compares a candidate with the reference. This AC creates `cargo test -p xtask --test spirit_eval_protocol_28_1` (exit line 1).
- **AC2.** Families are the scenario-id prefix (`cc`/`df`/`pb`/`tn`, `rd`); few-shot examples come only from a separate train file whose inputs are disjoint from every test row.
- **AC3.** The tier-2 re-record runs each GA class k times and keeps every result line; the floor rule and the comparison rule are the ruled D-rows.

### 28-2-butler-halt-prompt — The Butler halt prompt

*Closes · Δ:* rulings 1, 2, 3, 5 · kernel-Δ forecast 0 · **`v1.0.0` GA precondition** (ruling 2026-11-15: "before GA the Butler live evaluation must PASS its (calibrated) floor").

- *HEAD at `bf3cba25`:* the prompt is one `format!` string (`spirits/butler/src/lib.rs:197-223`); the halt for a conflict-free row is the model's `user_preference_drift` below 0.6 (`lib.rs:1100`, `:679-684`; `spirits/butler/manifest.toml:91-94`). The live run's 8 false positives are exactly the 8 conflict-free rows without a `preference_alignment` signal where the model returned drift 0.5 (pb02, pb03, pb04, pb06, tn01, tn03, tn05, tn09).
- **AC1 (command).** `cargo test -p butler --test halt_live_baseline_28_2` (exit line 2) proves the committed class cassette is a live-record of `claude-haiku-5-5` bound to the versioned prompt, train/test disjointness, and a complete calibrated floor record.
- **AC2 (command).** `MAOS_INFERENCE_MODE=replay maos eval halt --class butler` (exit line 3) exits 0 on that cassette.
- **AC3.** The live measurement under 28-1 on `claude-haiku-5-5` meets the calibrated precision and recall floors.

### 28-3-researcher-distill-prompt — The Researcher distill prompt

*Closes · Δ:* rulings 1, 3, 5; NFR-Aud-7 · kernel-Δ forecast 0 · not a GA precondition: GA-recommended only (**D-28-3-01 RULED B**, operator, 2026-11-15).

- *HEAD at `bf3cba25`:* the prompt is two sentences (`spirits/researcher/src/lib.rs:570-575`); hedge preservation is a deterministic whole-word match (`crates/maos-eval/src/distill_replay.rs:446-456`), not a judge verdict; the one live miss is `rd05-c1` ("seem to" rendered "appear to"); the judge decides faithfulness only (`distill_replay.rs:424-430`).
- **AC1 (command).** `cargo test -p researcher --test distill_live_baseline_28_3` (exit line 4).
- **AC2 (command).** `MAOS_INFERENCE_MODE=replay maos eval distill --class researcher` (exit line 5) exits 0 on the promoted cassette.

### 28-4-other-spirits-prompts-and-evals — The other Spirits

*Closes · Δ:* ruling 3 · kernel-Δ forecast 0 · not GA · `backlog`.

- Inventory at `bf3cba25`: model-driven today are `orchestrator` (`ambiguity_prompt`, `spirits/orchestrator/src/lib.rs:185-195`; `maos eval halt --class orchestrator` on a seed cassette) and `hello-spirit` (`crates/maos-spirit-hello/src/lib.rs:84-91`); `observer`, `mira`, `nash`, `architect`, `reviewer` and `worker` declare no `provider.complete` and call no Inference Port. Order and scope are D-28-4-01…05.

**Left out:** sampling-parameter determinism (Haiku 5.5 and Sonnet 5.5 reject `temperature`, `crates/maos-providers/src/anthropic.rs:99-107`); provider structured-output modes; a model-graded halt judge; corpus growth to NFR-Aud-8's N=100/N=500 tiers; any new key beyond `28-1`…`28-4`.

## Requirement trace

| Source | Requirement | Story |
|---|---|---|
| Ruling 2026-11-15 item 1 | Prompt-adjustment story; Butler live eval passes its calibrated floor before GA | 28-2 (GA), 28-1 (GA) |
| Ruling item 2 | `claude-haiku-5-5` reference; later Haiku compared by the same protocol | 28-1 AC6, 28-2 |
| Ruling item 3 | Per-Spirit, deterministic, versioned, hash-bound prompts | 28-2, 28-3, 28-4 |
| Ruling item 4 | Simple, effective evaluation of prompts, performance and behaviour | 28-1 |
| Ruling item 5 | Floors calibrated from measured distributions; measurement, interval, rationale recorded | 28-1 AC7; 28-2, 28-3 |
| NFR-Test-4 (`prd/non-functional-requirements.md:81`) | Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class | 28-2, 28-4 (orchestrator) |
| NFR-Onb-1 (`:125`; `docs/research/nfr-onb-1-protocol.md:60-61`) | Butler-class corpus: recall ≥ 0.90 on the calendar subset, precision ≥ 0.85 | 28-2 (spec minimums; the corpus stays 30 rows) |
| NFR-Aud-7 (`:62-67`) | Five-metric distillation gate (hedge-preservation ≥ 0.95) | 28-3 |

## Dependencies

- `28-2` and `28-3` start after `28-1`'s ruled D-rows and its AC1 land (they use its result schema, `eval-report` and the k-repeat workflow).
- The reference pin reaches the wire through `27-1-manifest-model-pin-to-wire` (in `review`, hosted proof pending); the cost record uses `27-2-provider-usage-fixes` (Anthropic usage fails closed).
- Paid live runs are the on-demand `tier-2-rerecord` dispatch (operator ruling 2026-11-12; `journey-nightly.yml:84-92`); `ops-nightly-cassette-rerecord` owns the dispatch, `28-2`/`28-3` own the first promotions of their class cassettes (`deferred-work.md:1413`).
