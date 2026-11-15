# Operator ruling 2026-11-15: Spirit prompt engineering, evaluation and realistic floors

Source: operator free text in the orchestrator session on 2026-11-15, answering the report on the first successful live re-record (journey-nightly run 43708370483).

## Measured facts that triggered it (live, claude-haiku-5-5 for Butler, claude-sonnet-5-5 + Sonnet judge for Researcher)
- Butler halt eval (`maos eval halt --class butler`, corpus `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`, 30 rows, 15 expected halts): recall 1.0, precision 0.652 (TP 15, FP 8, FN 0) against the floor 0.85. The model reports high-confidence conflicts for overlaps that involve a tentative event (rows pb01, pb03, pb05 at 0.85-0.9; pb06 at 0.6); the corpus defines only confirmed-vs-confirmed overlap as a halt. Over-halting (safe direction), not missed halts.
- Researcher distill (live): recall 1.0, faithfulness 1.0, hedge_preservation 0.944 (17 of 18) against the floor 0.95; judge ran without HTTP 400, `judge_temperature_applied` false.
- These were the first end-to-end live measurements of either evaluation in this repository. A Haiku 4.5 diagnostic run (run 43713305197, diagnostic branch, deleted) stopped at row 1 with `malformed proposal: completion is not a Butler proposal`, so no 4.5 baseline exists; the 0.85 and 0.95 floors were never validated against any live model.

## Rulings
1. Open a prompt-adjustment story; before GA the Butler live evaluation must PASS its (calibrated) floor ("GA: (a) pass"). The Researcher live evaluation is handled by the same program.
2. `claude-haiku-5-5` is the reference model now; a later Haiku version is compared against it with the same protocol.
3. Prompts are to be context-engineered per Spirit: specific to the Spirit's objectives, its tests and its tasks, and deterministic in structure (fixed sections, explicit decision rules, strict output schema, versioned and hash-bound).
4. Evaluation of prompts, Spirit performance and behaviour must be simple and effective.
5. Expectations that are unrealistically high are adjusted realistically from measurements: floors are calibrated from measured distributions after prompt tuning, with the measurement, the interval and the rationale recorded; they are not lowered by assertion.

## Consequences recorded
- GA set: the program's GA-critical stories join the GA preconditions of `RELEASE-HOLDS.md` GA-tag procedure item 1 (Butler live evaluation passes its calibrated floor).
- Plan: Epic 28 "Spirit prompts and evaluation" (stories filed by the story-creator after this record).
- Evidence of the first live run is kept outside git (ruling B7a) in `/home/lunarpulse/maos-evidence-archive/2026-11-15/`.

## Corrections and rulings (same day)

- (a) Correction to line 6. The 8 Butler false positives (pb02, pb03, pb04, pb06, tn01, tn03, tn05, tn09) do not come from the model's `confidence`; the attribution to `confidence` in line 6 is withdrawn. They come from `user_preference_drift` = 0.5 on conflict-free rows: the manifest halts when drift is below 0.6 (`spirits/butler/manifest.toml:85-94`), and `primary_scalar` (`spirits/butler/src/lib.rs:661-692`, `:1145-1153`) writes that drift to the kernel when there is no conflict. `confidence` is not on the halt path. pb01 and pb05 (confidence 0.85) did not halt because their drift is 0.9.
- (b) hedge_preservation is a case-insensitive whole-word string match (`crates/maos-eval/src/distill_replay.rs:446-456`), not a judge verdict. The one miss is rd05-c1: the source hedge "seem to" was written as "appear to". The Sonnet judge decides faithfulness only (`distill_replay.rs:424-430`).
- (c) Operator rulings, 2026-11-15, on the Epic 28 D-rows (stories 28-1 to 28-4; `epics/epic-28-spirit-prompts-and-evaluation.md`):
  - D-28-3-01: RULED B. 28-3 is not a `v1.0.0` GA precondition; it is GA-recommended only. 28-1 and 28-2 are the GA preconditions.
  - D-28-1-03: RULED A. Floor = max(spec minimum, Wilson lower bound of the measured baseline). A measurement below the spec minimum opens a spec-change D-row for the operator. Floors are never lowered by assertion.
  - D-28-1-02: RULED k = 5 repeats per live run.
  - All other PROPOSED D-rows in the five Epic 28 files (D-28-1-01, D-28-1-04…10, D-28-2-01…06, D-28-3-02…05, D-28-4-01…05): ACCEPTED (operator: proceed with recommendation, 2026-11-15).
