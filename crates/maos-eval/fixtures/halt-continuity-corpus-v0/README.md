# Halt-Continuity Corpus v0

I14 end-to-end integration test corpus for hot-swap halt continuity
enforcement (Story 5.2). Mirrors `isolation-corpus-v0/` shape from Story 4.5.

## Methodology

- **Tier:** `scripted-v0` (synthetic kernel-internal fixtures)
- **Threat model:** halt-set loss across hot-swap boundary
- **Derivation:** ADR-019 (I14 halt continuity), ADR-017 (hot-swap state transfer)
- **Scenario count:** ≥10 (per Epic 5 AC4)
- **Pass threshold:** 100% (halt continuity is mechanical, not statistical)

## Scenario categories

| Category | Count | Description |
|---|---|---|
| Empty halt set | 3 | SafeDrained with 0 drained (scenarios 001, 008, 010) |
| Carry, compatible successor | 5 | SafeMigrated{n}: the predecessor's own pending halts are carried, never drained (20-3c K1p; scenarios 002, 003, 004, 011, 012) |
| Schema-incompat | 3 | EHaltContinuityViolation (scenarios 005, 007, 009) |
| Empty successor list | 1 | EHaltContinuityViolation: `Some(&[])` is a declared-but-empty list, so the gate reports a violation, not `MissingHaltProtocolCompatibility` (scenario 006) |

`hot_swap_halt_continuity_test.rs` seeds each scenario's `pending_halts` into a `HaltRegistry`, runs `validate_swap_halt_continuity`, and fails on any mismatch against `expected_outcome`. `MissingHaltProtocolCompatibility` needs `successor_accepted_versions = None`, which the swap coordinator never passes (it defaults a missing manifest field to `[1]`; see `deferred-work.md`), so no fixture reaches it.

## References

- `crates/maos-kernel-core/src/halt/mod.rs` — `validate_swap_halt_continuity`
- `crates/maos-domain/src/halt.rs` — `HaltContinuityError`
- ADR-019 — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md`
