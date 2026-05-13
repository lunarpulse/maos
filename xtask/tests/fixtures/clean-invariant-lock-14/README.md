# clean-invariant-lock-14

Pre-flight fixture for Story 1a.1's 14-invariant landing (DF17 in `_bmad-output/implementation-artifacts/deferred-work.md`).

## Purpose

Story 1a.1 will land 14 binding-v0.1 ADRs and codify invariants I1–I14 as types simultaneously. The PR will touch all 14 register files (`docs/invariants/I*.md`). The `invariant-lock` gate has only been exercised against a single-invariant diff (Story 0.2 / I9). This fixture provides the canonical "clean multi-invariant landing" shape so the gate can be tested against 14 invariants before 1a.1's PR opens.

## Contents

14 synthetic `I*.md` files (I1.md through I14.md), each carrying:

- Frontmatter with `id`, `title`, and an `enforcement_cadence` block
- Cadence rows: `v0.1-alpha-pre: —` (new) AND `v0.1: CI` (new) — i.e., this fixture represents the "after merge" state where Story 1a.1 has added the v0.1-alpha-pre and v0.1 rows to a previously cadence-empty register

## Expected gate behavior against this fixture

`cargo run -p xtask -- invariant-lock --changed-files <list-of-14-paths>` should report:

```json
{
  "passed": false,  // because corpus-delta and reviewer requirements are unmet in fixture context
  "touched_invariants": ["I1", "I10", "I11", "I12", "I13", "I14", "I2", "I3", "I4", "I5", "I6", "I7", "I8", "I9"],
  "missing_corpus_delta": true,
  "missing_phase_commitment": <depends on git context>,
  "regression_detected": [],
  ...
}
```

The key assertions for DF17 pre-flight:

- `touched_invariants.len() == 14` and contains all of I1–I14
- `regression_detected.is_empty()` against the clean fixture
- The gate completes without panicking on 14 simultaneous invariants

## Limitations

The current `invariant-lock` xtask resolves register-file paths against the workspace root (`docs/invariants/I*.md`), not against the fixture directory. The fixture's `I*.md` files therefore serve as **documentation of the expected shape** for an eventual full integration test, not as a self-contained gate-input set.

A complete integration test requires one of:

1. Refactoring `xtask/src/invariant_lock.rs` to accept a `--root` flag (preferred; matches the pattern in `check-corpus`, `check-empty-kernel`, etc.)
2. Running the test in a synthetic git workspace with these files committed at the canonical `docs/invariants/` path

Until then, the multi-invariant case is tested at the **unit-test level** in `xtask/src/tests/invariant_lock_tests.rs` (see the `parse_cadence_handles_14_distinct_files` and `regression_detection_in_14_invariant_batch` tests added 2026-05-13).

## References

- DF17 — `_bmad-output/implementation-artifacts/deferred-work.md`
- `docs/dev-discipline/1a1-adr-landing.md` — names this as a Story 1a.1 pre-flight blocker
- Story 0.2 — first single-invariant dogfood
