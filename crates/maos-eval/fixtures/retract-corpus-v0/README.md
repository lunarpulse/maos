# retract-corpus-v0 — N=30 synthetic retract evaluation corpus

**Tag:** `synthetic-v0`
**Status:** Provisional (Story 6.1). May be extended before v1.0.
**Authoring discipline:** Generated via script (Epic 2 retro A2 compliance).

## Distribution

| Category | Count | Description |
|---|---|---|
| `before_delivery` | 10 | Sender retracts frame before recipient processes it |
| `after_delivery` | 10 | Sender retracts frame after recipient has processed it |
| `authority_violation` | 5 | Non-sender attempts to retract a frame they did not send |
| `idempotent` | 5 | Second retract of same frame returns `Already` |
| **Total** | **30** | |

## Scenario format

Each scenario is a JSON file describing:

- **`scenario_id`** — unique identifier
- **`category`** — one of the four categories above
- **`description`** — human-readable intent
- **`original_frame`** — descriptor of the frame being retracted
  - `frame_id_hex` — 32-char hex string (16 bytes)
  - `from_spirit` — sender spirit ID
  - `to_spirit` — recipient spirit ID
  - `kind` — IAC frame kind (TaskAssign, TaskComplete, DecisionDispatch, TelemetryEvent, ConsentRequest)
  - `payload_size_bytes` — approximate payload size
- **`retract_request`** — who is retracting and why
  - `retracting_spirit` — must match `original_frame.from_spirit` for success
  - `reason` — retraction reason string
- **`expected_outcome`** — what the kernel should return
  - `success` — true if retraction succeeds
  - `outcome_variant` — `Retracted`, `Already`, or `Error`
  - `error_variant` — `RetractAuthorityViolation` or `OriginalNotFound` when `success: false`

## Measurement

The corpus is validated by `crates/maos-eval/tests/retract_corpus.rs` which asserts:
- All 30 scenarios load without parse errors
- Each scenario has a valid category
- Category distribution matches the table above
- Outcome structure is internally consistent (failed scenarios have `error_variant`, successful ones do not)

The integration test at `crates/maos-kernel-core/tests/retract_corpus_v0.rs` exercises the actual retract primitive against a subset of representative scenarios.
