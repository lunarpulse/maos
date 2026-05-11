# Invariant Register

This directory contains the canonical machine-readable register of MAOS invariants I1–I14.

## Files

- `I1.md` through `I14.md` — one file per invariant, with frontmatter enforcing the phase-cadence matrix from Architecture §3.2.1.
- `journal.jsonl` — append-only journal of every invariant-lock merge event.

## Lock Protocol (ADR-037)

Any PR touching an invariant file (`I*.md`), the `lock.toml` mapping, or the `maos-domain` invariant types must pass the `invariant-lock` CI gate. The gate requires:

1. **Machine-checkable diff** against the invariant set (present by construction).
2. **Corpus delta** — touch to `tests/coverage-matrix.yaml` (file-touched check at v0.1-α; row-count comparison at v0.3+).
3. **Phase-commitment update** — at least one touched `I*.md` has its enforcement-cadence table modified.
4. **≥2 maintainer sign-offs** verified via platform API.
5. **Forward-only progression** — no demotion of enforcement cells (`runtime → CI` is forbidden).

On merge success, the gating CI job appends to `journal.jsonl`.
