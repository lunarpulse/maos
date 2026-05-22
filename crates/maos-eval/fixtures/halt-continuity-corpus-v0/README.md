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
| Empty halt set | 2 | SafeDrained with 0 drained |
| Drain succeeds | 3 | SafeDrained with varying drained_count |
| Schema-compat migrate | 2 | SafeMigrated with version compatibility |
| Schema-incompat | 2 | EHaltContinuityViolation |
| Missing compat | 1 | MissingHaltProtocolCompatibility |
| Race condition | 2 | Halt arriving during swap |

## References

- `crates/maos-kernel-core/src/halt/mod.rs` — `validate_swap_halt_continuity`
- `crates/maos-domain/src/halt.rs` — `HaltContinuityError`
- ADR-019 — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md`
