# violation-invariant-lock-14-regression

Pre-flight fixture for DF17 — verifies the `invariant-lock` regression-detection logic identifies the offending invariant by id, not just "a regression occurred."

## Purpose

13 of the 14 files (I1–I6, I8–I14) carry the same clean shape as the `clean-invariant-lock-14/` fixture. The 14th file (`I7.md`) carries a **deliberate enforcement-cadence regression**: the "after" state demotes `v0.3` from `runtime` to `CI`.

The corresponding "before" state for the regression check is implied by the test driver (not committed in this fixture — see Limitations below); in the eventual full integration test, the "before" state would be a sibling `xtask/tests/fixtures/clean-invariant-lock-14-pre/` directory containing the prior file shapes.

## Expected gate behavior against this fixture

`cargo run -p xtask -- invariant-lock --changed-files <list-of-14-paths>` should report:

```json
{
  "passed": false,
  "touched_invariants": ["I1", ..., "I14"],
  "regression_detected": [
    "ADR-037 violation: enforcement cadence cannot regress for I7 (was=runtime, now=CI)"
  ],
  ...
}
```

The key DF17 pre-flight assertions:

- The regression is caught for **exactly one** invariant (I7), not falsely detected for any of the other 13
- The error message includes the specific invariant id (`I7`), not a generic "regression detected"
- The error message includes the before and after cadence values (`was=runtime`, `now=CI`)
- The gate continues processing the remaining 13 invariants and does not short-circuit on the first regression — i.e., if a future variant of this fixture introduces a second regression, both should be reported

## Limitations

Same as `clean-invariant-lock-14/`: the xtask reads register files from workspace root, not from the fixture. Therefore the regression detection is tested at the **unit-test level** in `xtask/src/tests/invariant_lock_tests.rs` (see `regression_detection_in_14_invariant_batch`).

## References

- DF17 — `_bmad-output/implementation-artifacts/deferred-work.md`
- `clean-invariant-lock-14/README.md` (sibling fixture)
