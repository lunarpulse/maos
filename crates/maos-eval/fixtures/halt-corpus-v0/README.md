# halt-corpus-v0 — N=62 synthetic halt evaluation corpus

**Tag:** `synthetic-v0`
**Status:** Provisional (Story 4.1 + 4.2). To be replaced by Story 4.5's HSIS production corpus before v1.0.
**Authoring discipline:** Hand-authored (Epic 2 retro A2 compliance).

## Distribution

| Ground Truth Class | Count | Floor |
|---|---|---|
| `true_positive` | 26 | ≥15 |
| `true_negative` | 19 | ≥15 |
| `false_positive` | 11 | ≤10 |
| `false_negative` | 6 | ≤10 |
| **Total** | **62** | |

## Predicate coverage

| Predicate | TP | TN | FP | FN | Total |
|---|---|---|---|---|---|
| `on_value_above` | 10 | 8 | 5 | 2 | 25 |
| `on_value_below` | 10 | 7 | 5 | 3 | 25 |
| `on_value_within` | 3 | 2 | 1 | 0 | 6 |
| `on_value_outside` | 3 | 2 | 0 | 1 | 6 |
| **Total** | **26** | **19** | **11** | **6** | **62** |

## Authoring Methodology

Each scenario is a JSON file describing a single Spirit's epistemic-policy
rule firing (or non-firing) against a synthetic scalar write. The scenario
contains:

- **`epistemic_policy_rules`** — the rule(s) the Spirit evaluates (mirrors
  Story 4.2's predicate semantics: `on_value_above`, `on_value_below`,
  `on_value_within`, `on_value_outside`).
- **`scalar_writes`** — the synthetic scalar value written to the tagged
  slot (simulates `working_memory.set_scalar` from Story 4.2).
- **`expected_halt_invocation`** — whether the predicate is expected to fire.
- **`ground_truth_class`** — the correctness label driving recall/precision
  math.

### Ground truth class definitions

- **`true_positive`**: The predicate **correctly** fires when the halt was
  warranted (e.g., value exceeds threshold for `on_value_above`, within
  bounds for `on_value_within`, outside bounds for `on_value_outside`).
- **`true_negative`**: The predicate **correctly** does NOT fire when the
  halt was not warranted.
- **`false_positive`**: The predicate fires but the halt was NOT warranted
  — counts against precision.
- **`false_negative`**: The predicate does NOT fire but the halt WOULD have
  been correct — counts against recall.

### Scenario design strategy

- TP scenarios use known-bad values exceeding/within/outside thresholds by
  small margins (≥0.01) to exercise boundary sensitivity.
- TN scenarios use safe values below/outside/within thresholds by significant
  margins (≥0.03) to validate non-firing.
- FP scenarios use values barely triggering where halts are over-cautious —
  exercises precision degradation patterns.
- FN scenarios use values barely missing where halts should fire —
  exercises recall degradation patterns.

### Story 4.2 additions (scenarios 051–062)

Scenarios 051–062 add coverage for the `on_value_within` and
`on_value_outside` universal-arithmetic predicates. Each new predicate
gets 3 TP + 2 TN scenarios, rounded to 12 total entries to cover
boundary cases (value equal to lower/upper for within, value equal to
lower/upper for outside).

### Forward-compatibility

The `tag: "synthetic-v0"` field is the forward anchor for Story 4.5's
replacement. The test `halt_recall_floor.rs` asserts that every scenario
carries this tag; when Story 4.5 lands the production HSIS corpus, the
assertion will fail and force re-evaluation.

## Measurement thresholds (AC6)

| Metric | Floor | Measured by |
|---|---|---|
| Halt-recall (TP/(TP+FN)) | ≥0.70 | `maos-eval/tests/halt_recall_floor.rs` |
| Halt-precision (TP/(TP+FP)) | ≥0.85 | `maos-eval/tests/halt_recall_floor.rs` |
| Predicate-firing recall | ≥0.85 (FR32) | `maos-eval/tests/halt_recall_floor.rs` |
