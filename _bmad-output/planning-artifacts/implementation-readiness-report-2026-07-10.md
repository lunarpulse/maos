---
stepsCompleted: [step-01-document-discovery, step-02-prd-analysis, step-03-epic-coverage-validation, step-04-ux-alignment, step-05-epic-quality-review, step-06-final-assessment]
scope: v2.2 functional-completeness phase (Epics 12–14)
inputDocuments:
  - prd/ (sharded, canonical — 15 shards, "full prd expansion" @ 0abffcce)
  - prd-delta-full-spectrum-2026-07-06.md (delta record, folded into sharded prd)
  - architecture-maos-minimal-opus/ (sharded, canonical — 26 shards incl. §15 RATIFIED)
  - epics/ (sharded — index + epic-12/13/14 v2.2 draft)
  - requirements-inventory.md (v2.2 coverage map added)
---

# Implementation Readiness Assessment Report

**Date:** 2026-07-10
**Project:** maos
**Scope:** v2.2 functional-completeness phase — Epics 12–14 (17 stories), against the RATIFIED architecture §15 (ADR-054–057) + ratified PRD delta.

---

## Step 1 — Document Inventory

| Type | Canonical source | Status |
|------|------------------|--------|
| **PRD** | `prd/` (sharded, 15 shards; "full prd expansion" @ `0abffcce`, `[DELTA-2026-07-06]` folded in) | ✅ present, canonical |
| PRD (supplementary) | `prd-delta-full-spectrum-2026-07-06.md` (delta record), `full-prd-gap-map-and-planning-plan-2026-07-06.md` (plan), `prd-validation-report.md` (QA) | ✅ supplementary, not duplicates |
| **Architecture** | `architecture-maos-minimal-opus/` (sharded, 26 shards; §15 v2.2 **RATIFIED 2026-07-09**) | ✅ present, canonical |
| Architecture (working) | `architecture/architecture-maos-2026-07-06/` (Step-2 memlog + reviewer output) | ℹ️ working dir, NOT a competing canonical |
| **Epics** | `epics/` (sharded; `index.md` + `epic-12/13/14-*.md` v2.2 draft) | ✅ present, canonical |
| Requirements | `epics/requirements-inventory.md` (v2.2 coverage map appended) | ✅ present |
| **UX** | — | N/A (kernel/CLI project; no UI surface; dev-tool + a11y requirements live in PRD) |

**Duplicate check:** ✅ none — no whole/sharded conflict for any type. The two architecture directories are *canonical* (`…-minimal-opus/`) vs *working* (`architecture/…-2026-07-06/`), not duplicates. The 3 standalone PRD files are supplementary records, not competing whole-PRDs.

**Missing check:** UX absent — **expected and acceptable** for a kernel/CLI project; the v2.2 epics carry no UX-DRs. Not a readiness gap.

---

## Step 2 — PRD Analysis (completeness · clarity · testability)

| Check | Verdict |
|-------|---------|
| All FRs defined + homed | ✅ 65 FRs; **FR37 was the only unserved FR → now homed at E13.4**. Zero unserved. |
| NFRs complete for v2.2 | ✅ delta re-targeted NFR-Scale-2/Rel-7/Scale-5 → v2.2; NFR-Ops-11/Tenancy-1 → v2.2 (all mapped). |
| Journeys served | ✅ J0/J-Butler/J-Researcher/J1/J4/J6 done; **J3 → E12, Reza → E13** now homed. |
| Testability | ✅ requirements are mechanically testable (project's gate discipline); each v2.2 gate has a named proven-red. |
| Delta integrity | ✅ `[DELTA-2026-07-06]` folded into sharded prd; "No FR/NFR weakened or removed." |

**PRD note (hygiene, non-blocking):** `prd/product-scope.md` carries a **SUPERSEDED pre-restructure phasing** banner — correctly marked, but plan against `prd/project-scoping-phased-development.md` (operative). No action required beyond awareness.

## Step 3 — Epic Coverage Validation (requirements → epic/story traceability)

Every v2.2 residual requirement traces to a story (full map in `epics/requirements-inventory.md`):

- **FR37** → E13.4 · **J3** → E12.4 · **Reza** → E13.6 · **NFR-Scale-2/Rel-7** (100-host) → E14.1 · **NFR-Sec-13** (10-host rotation) → E14.2 · **NFR-Scale-5** → E13.6 · **NFR-Ops-11/Tenancy-1** → E13.1/13.2 · **v2.0 sweep** → E14.4/14.5 · **NFR-Maint-1 ceiling** → E14.6.
- **ADR coverage:** ADR-054 (E12) · ADR-055/056 (E13) · ADR-057 (E14) — all ratified §15.11.

**Dependency integrity:** ✅ every dependency resolves to a `done` epic (0–11) or an earlier v2.2 story. No forward/circular dependency. Critical path: **E12 → E13** (E13 needs the E12 org manifest); E14.1/14.2 parallel-able (depend on done 11.x), **E14.3 waits on E13.4**.

## Step 4 — UX Alignment

**N/A.** Kernel/CLI project, no UI surface; v2.2 epics carry zero UX-DRs. Accessibility/dev-tool requirements are PRD-owned and already shipped (a11y flags in `maosctl`, WCAG docs Epic 9). Not a readiness gap.

## Step 5 — Epic Quality Review (ACs · sizing · gates · kernel-Δ)

| Check | Verdict |
|-------|---------|
| ≤6 ACs / story | ✅ all 17 stories ≤6 ACs |
| Sizing / demo-anchored | ✅ 5–6 stories/epic, each demo/journey-anchored (E12.4, E13.6 = scenes) |
| Gate discipline | ✅ each epic names §A7 reflexes + derive-and-reconcile + real-subsystem proven-red + region/team/identity source-reflex |
| Kernel-Δ | ✅ ZERO by default @23081; 3 FLAG-Winston seams explicitly flagged **verify-at-preflight** (12.5, 13.2, 13.3) — correct posture, not a gap |
| Model/review | ✅ frontier-allowlist + §A6 net binding; non-degradable on the security-adjacent stories |

## Step 6 — Final Assessment

### External release holds (re-checked)
| Hold | Status | v2.2 dev impact |
|------|--------|-----------------|
| Real external pen-test zero-P0/P1 (NFR-Sec-7) | ❌ open (external) | **NON-GATING** — GA ledger; does NOT block Epic 12 |
| Export counsel 5D002.c.1 (NFR-Comp-1) | ❌ open (external) | **NON-GATING for dev**; entanglement = keep external WASM binary publication flag-gated (see G5) |

**Confirmed: neither external hold blocks per-story preflight or Epic 12 dev** (founder functionality-first directive).

### Readiness gaps (none blocking; address at preflight)
| # | Sev | Gap | Action |
|---|-----|-----|--------|
| **G1** | Top preflight action | 3 FLAG-Winston kernel-Δ seams (12.5 `EMigrationPlanDrift`; 13.2/13.3 cross-team `CrossRegionReadmit` reuse) are "verify-at-preflight" | Resolve verify-or-prove-zero at each story's preflight (the 11.4b CATCH-0 pattern); re-pin named + disclosed if any |
| **G2** | Medium | E14.6 folds ADR-041 Phase-3/4 kernel decomposition (~6.5 KLOC extraction) as a *note*, not a story | Decide at E14 preflight whether the extraction warrants its own story/track (it is real kernel-refactoring toward the ~11.2K target) |
| **G3** | Minor | `loom-threat-model.md` gates E13.1 but is only a checklist item, not a tracked story/AC | Promote to an explicit E13.1 pre-task or AC so it cannot slip before the tenant wall ships |
| **G4** | Minor | `docs/adr/ADR-054..057-*.md` files not yet created (only the §15.11 record exists) | Create at E12/E13/E14 kickoff (already in checklists); reconcile the ADR-053 number collision noted in §15 |
| **G5** | Minor | E14.4 distro packaging could touch the WASM-distributable 5D002.c.1 entanglement | Keep external WASM binary publication flag-gated until export counsel clears (founder-directive honesty guard) |

### Verdict

🟢 **READY for per-story preflight and Epic 12 dev.** The v2.2 spec set (PRD ↔ architecture §15 ↔ Epics 12–14) is complete, internally consistent, and fully traceable — every FR/NFR/journey has a home, dependencies resolve to done work, story quality (≤6 ACs, demo-anchored, gate discipline, kernel-Δ posture) is sound, and the two external holds are confirmed non-gating for dev. The five gaps above are **preflight refinements, not blockers**; G1 (the three kernel-Δ seams) is the top thing to nail at each story's preflight.

**Recommended first action:** Epic 12.1 (cohort manifest + full-pairwise mesh) preflight → dev, frontier model + full §A6 net.
