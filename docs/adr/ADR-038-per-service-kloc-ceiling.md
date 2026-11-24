---
Status: binding-v0.1; per-crate ceilings and the aggregate alarm REMOVED 2026-11-24 (operator decision 11, lean-gates WP-23): `xtask/kloc.toml` gates only the `maos-kernel-core` row (ADR-069); every other crate is measured and printed by `xtask kloc-check`
Gate: xtask/kloc.toml enforced by tokei in CI; aggregate ≤20 KLOC, alarm at 16
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.4
---

# ADR-038 — Per-service KLOC ceiling

**Decision.** Kernel ≤20 KLOC trusted core enforced as the sum of per-crate ceilings. Per-crate budgets in `xtask/kloc.toml`: `maos-kernel-core ≤6 KLOC`, `maos-cap-registry ≤3 KLOC`, `maos-wire ≤2 KLOC`, `maos-journal ≤2 KLOC`, etc. Aggregate ≤20 KLOC, alarm at 16. CI gate via `tokei`.

**Rationale.** "Kernel stays small" needs structural enforcement, not memo discipline. Per-crate ceilings make the KLOC budget legible and machine-checked.

**Alternatives considered.** Aggregate-only ceiling (rejected: no early warning when one crate consumes the budget). No ceiling (rejected: erodes silently).

**What would force a revisit.** A new kernel surface justifies a ceiling extension; amendment via ADR-037 process.
