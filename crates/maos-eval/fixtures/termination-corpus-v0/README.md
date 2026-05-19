# termination-corpus-v0 — N=1000 termination receipt measurement corpus

**Generator:** `xtask/src/gen_termination_corpus.rs`
**Status:** Scaffold (Story 4.1). Story 5.3 extends with unplanned-termination path at runtime.
**Authoring discipline:** Mechanically generated (deterministic SHA-pinned output).

## Distribution

| Kind | Count |
|---|---|
| `planned_unload` | 250 |
| `halt_accepted` | 250 |
| `unplanned_crash` | 250 |
| `halt_rejection` | 250 |
| **Total** | **1000** |

## Generation methodology

The corpus is generated deterministically by `cargo run -p xtask -- gen-termination-corpus`.
Re-running produces byte-identical JSON files. The generator uses sequential indices for
scenario IDs and deterministic patterns for halt IDs.

### Scenario shape

Each file is a single termination scenario describing the Spirit's pending halt set
at termination time:

- **`planned_unload`**: Director-initiated unloads with halt-set sizes 0, 1, 3, 10
- **`halt_accepted`**: One halt per scenario, simulating accepted_halt resolution
- **`unplanned_crash`**: SIGKILL-style process death with varying halt-set sizes
- **`halt_rejection`**: Policy `verbalize_only` scenarios where halt is rejected

## Measurement threshold (AC4)

| Metric | Floor | Measured by |
|---|---|---|
| Receipt production rate | ≥99.9% (≥999/1000) | `maos-kernel-core/tests/halt_receipt_production_rate.rs` |
