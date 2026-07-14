# RELEASE-HOLDS.md — GA Ledger

> **Scope:** This ledger tracks the items that stand between the merged,
> feature-complete `main` line and a **GA release tag**. It exists to keep
> *development* and *GA* decoupled: the holds below gate a GA tag, **not**
> ongoing development on `main` (founder functionality-first directive;
> E11 retro A3 / E12 retro B2).

## Status

- **Feature-completeness:** ✅ **Done and merged.** The Epic 11 (v2.0) + Epic 12
  (v2.2, J3 Team Nexus) line landed on `main` via PR #5, merge-commit `b8caae53`
  (2026-07-14). All PRD journeys served (J3 was the last unserved).
- **GA tag:** ⏳ **Held.** Blocked only by the two external items below. Neither
  blocks further development, branch merges, or internal milestones.

## Holds

| # | Hold | Requirement | Owner | Status | GA effect |
|---|------|-------------|-------|--------|-----------|
| 1 | External penetration test | **NFR-Sec-7** — external pen-test report, **zero P0/P1 findings open at ship**. Triage by joint panel (pen-test lead + MAOS security owner); disagreement → PRD-author tiebreak. P0/P1 per OWASP Risk Rating Methodology, frozen at engagement start. | Security owner + external firm | ❌ Open (external, not yet scheduled) | **Blocks GA tag.** |
| 2 | Export-control classification | **NFR-Comp-1** — ECCN classification letter on file; EAR99 vs 5D002 determination published in `STABILITY.md §Export`; dual-use review for kernel crypto primitives. | Export counsel + security owner | ❌ Open (external counsel review pending) | **Blocks GA tag** for any externally-published binary that includes the WASM engine. |

## Standing mitigations (in force now, not a GA precondition)

- **WASM engine is OFF by default.** The `wasm-host` feature
  (`crates/maos-bin/Cargo.toml`) is off-by-default and gates the vendored
  `wasmtime` engine. Off-by-default is the **export-control precondition for
  5D002.c.1** — the shipped default artifact carries no controlled crypto/WASM
  engine, so internal builds and non-WASM distribution proceed while Hold 2 is
  open. See `docs/compliance/export-counsel-precondition.md` and
  `docs/compliance/eccn-classification.md`.

## GA-tag procedure (do not tag until both clear)

1. Hold 1: pen-test report received, joint-panel triage records **zero P0/P1
   open**; attach report reference here.
2. Hold 2: ECCN letter on file, `STABILITY.md §Export` reflects the final
   EAR99/5D002 determination; export counsel signs off on WASM-inclusive
   external publication (or that path stays flag-gated at GA).
3. Only then cut the GA tag. Update this ledger's Status to reflect the tag.

## References

- E12 retro action B2 (retro doc `epic-12-retro-2026-07-13.md`).
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
  (NFR-Sec-7 §L131, NFR-Comp-1 §L247).
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-07-10.md`
  (both holds marked NON-GATING for dev).
