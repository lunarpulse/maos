# nfr-onb-1 fixture corpus — `calendar-comms-v0.3.fixture.jsonl`

**Story:** 7.5b (NFR-Onb-1 30-Minute First Spirit Validation Gate).
**Status:** **STAND-IN FIXTURE — provisional.** This is **NOT** the canonical
Butler regression corpus.

## Ownership boundary (LOCKED Decision 1)

Story 8.1 owns the canonical Butler-class corpus at
`spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` **exclusively** ("no
other story authors this corpus"). Story 8.1 is `epic-8: backlog` and not
implemented. Rather than block 7.5b or reorder epics, this 7.5b-owned fixture
lets the NFR-Onb-1 harness be built + CI-wired now.

The corpus **resolver**
(`maos_eval::onboarding_gate_corpus::resolve_corpus`) **PREFERS** the real
Butler corpus and **falls back** to this fixture, logging which corpus + its
SHA-256 it used and stamping each scored outcome `corpus_source: butler|fixture`.
Every verdict scored against this fixture is stamped **`provisional: true`** so a
fixture-only PASS can never be mistaken for the live v0.3 gate.

## SHA-256 pin

```
1a5b0738e959b537f1dd07993ed0d9978c889e1348d20a2007a03235d03d9110  calendar-comms-v0.3.fixture.jsonl
```

The harness re-computes this digest at run time and records it in every
`outcomes.jsonl` row (`corpus_sha256`). A drift here changes the recorded digest,
making any tampering loud.

## Format

JSONL. The **first line** is a `{"stand_in_for": ...}` meta header marking the
file a STAND-IN. Every subsequent line is one scenario:

| Field | Meaning |
|---|---|
| `scenario_id` | unique id (`cc*` = calendar-conflict subset, `fp*`/`tn*` = non-conflict) |
| `calendar_conflict` | true iff in the calendar-conflict subset that **halt-recall** is measured over |
| `expected_halt` | ground truth — should a correct Spirit halt/flag this scenario? |
| `observed_halt` | v0.3 STAND-IN for the candidate's response (Story 8.1's real path derives this from the bus — see the module scoring contract) |

## Distribution (N=30)

| Class | Count | `calendar_conflict` | `expected_halt` | `observed_halt` |
|---|---|---|---|---|
| calendar-conflict true-positive | 10 | ✅ | ✅ | ✅ |
| non-conflict false-positive | 1 | — | — | ✅ |
| non-conflict true-negative | 19 | — | — | — |

A competent first-Spirit scored against this fixture therefore lands at
**halt-recall = 1.0** (10/10 on the calendar-conflict subset, ≥0.90 floor) and
**halt-precision = 10/11 ≈ 0.909** (≥0.85 floor) — i.e. a clean `succeed`,
demonstrating the math is wired without hard-coding `1.0` everywhere. The exact
boundary cases (recall exactly 0.90, precision exactly 0.85) are exercised by
`onboarding_gate_corpus`'s unit tests against synthetic corpora.
