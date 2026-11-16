# Dev283 dispatch brief (interrupted by Anthropic 429 on 2026-11-16; NO edits were made)

Paste this to a `coder` agent (sonnet-5-5:high) together with the shared context block below.

## Shared context (send with both briefs)

- Repo: Rust workspace, remote lunarpulse/maos. Main is at fad08ae0.
- Hard rules: EDIT ONLY. Do NOT run cargo, tests, fmt, xtask gates, git, npm, or any build. The
  orchestrator runs all verification and formatting. Use ABSOLUTE paths under your worktree only.
  NEVER touch /home/lunarpulse/dev_ws or /home/lunarpulse/dev_ws-docs or any other worktree.
  Use the edit tool for edits (no sed/perl/python for edits). Do not use python to rewrite data files;
  if a data file must be regenerated, say so and leave it for the orchestrator. No stubs, no TODOs.
  Final message: every file changed, every decision, anything you could not verify.

## Target

Worktree: /home/lunarpulse/dev_ws-e283 (branch e28-3-2026-11-16, based on main fad08ae0).
Story: /home/lunarpulse/dev_ws-e283/_bmad-output/implementation-artifacts/28-3-researcher-distill-prompt.md
(read it fully, including Decisions D-28-3-01..05 which are ACCEPTED/RULED).
Pattern to follow: story 28-2's Butler implementation: `spirits/butler/prompts/proposal-v1.txt`, the
train split fixture, `spirits/butler/tests/halt_prompt_28_2.rs`, `spirits/butler/eval/floors.toml`,
`spirits/butler/tests/halt_live_baseline_28_2.rs`, `docs/research/spirit-evaluation-protocol.md`,
`crates/maos-eval/src/halt_replay.rs` (`HaltScore.format_errors`, `FloorRecord`),
`crates/maos-bench/src/halt_eval.rs`. Read these first.

## Change (story tasks T1, T2 and the code half of T3 only; NOT T4 paid dispatch, NOT T5 promotion)

1. T1 / AC3: create `spirits/researcher/prompts/synthesis-v1.txt` with fixed sections ROLE, OBJECTIVE,
   RULES (copy each listed hedge phrase verbatim; add no fact; no meta-text), OUTPUT SCHEMA
   (D-28-3-04 C: one JSON object `{"finding": string}` parsed strictly), EXAMPLES (drawn ONLY from the
   new train file), INPUT (statement + the claim's hedge list). Add `SYNTHESIS_PROMPT_VERSION` and make
   `synthesis_prompt` in `spirits/researcher/src/lib.rs` render the file byte-identically for the same
   claim (include_str! like Butler). Parse the model completion strictly as the JSON object; a
   malformed completion must be surfaced to the scorer as a format error (Butler pattern) and
   `summarize` still bounds the text. Pass the claim's `hedges` list to the prompt (lib.rs ~263-265).
2. AC4: create `spirits/researcher/tests/fixtures/distill-corpus-v0.train.jsonl` (hand-authored, with a
   provenance header like the Butler train file; hedged claims including synonym traps such as
   'seem to' vs 'appear to'), disjoint from the 16 test scenarios rd01..rd16.
3. T2: in `crates/maos-eval/src/distill_replay.rs` add a format-error count to the distill score and
   use the 28-1 FloorRecord mechanism for distill floors (floor = max(spec minimum, Wilson lower bound
   rounded down); NFR-Aud-7 minimums recall 0.90, faithfulness 0.98, hedge 0.95, traceability 1.0,
   leakage 0.0 are NEVER lowered). In `crates/maos-bench/src/distill_eval.rs` add the judge-only repeat
   mode of D-28-3-03 A: replay one recorded survey and call the live judge k times on the same
   findings; report per-finding agreement and faithfulness mean/min. Wire it into the existing 28-1
   dispatch (`.github/workflows/journey-nightly.yml`, `include_researcher`) ONLY if that is a small
   change; otherwise describe exactly what the orchestrator must add.
4. T3 code half: every place that hashes `synthesis_prompt` will change (cassette entry hashes in
   `crates/maos-journey-test/cassettes/j-researcher/*.json`, `spirits/researcher/tests/**`,
   `crates/maos-bin/tests/eval_distill_18_3.rs`, `crates/maos-bin/tests/researcher_inference_18_3.rs`).
   Do NOT hand-rewrite cassette JSON hashes. Find how Butler's generator test
   (`spirits/butler/tests/halt_eval_corpus_18_2.rs` or the Researcher equivalent) regenerates fixtures,
   update/extend the Researcher generator so the orchestrator can regenerate the seed cassette + corpus
   frozen surveys/verdicts with a single cargo command, and list that command. Update every
   assertion/pin that depends on the old prompt text; if a SHA must be computed, leave a clearly named
   instruction instead of guessing a hash (tests/corpora/MANIFEST.toml entries included).
5. Add `spirits/researcher/tests/prompt_28_3.rs`: assert section order/headings, train/corpus
   disjointness, and that the rendered prompt equals the file with placeholders substituted. The
   orchestrator will add the SHA pin after you finish (do not invent a hash).
6. Do NOT edit: judge prompt `JUDGE_PROMPT` or `tests/judge-config.toml`, kernel code,
   `.github/workflows/discipline.yml`.
7. Update the story file: tick T1, T2, note the T3 code half; fill 'Agent Model Used', Completion
   Notes, File List; add a Change Log line. Keep `Status: ready-for-dev` until the orchestrator flips it.

## Acceptance

All of the above exists and is internally consistent by reading; final message includes the
regeneration command(s), the files the orchestrator must compute SHA pins for, and a kloc-relevant
growth estimate (maos-bench, maos-eval).
