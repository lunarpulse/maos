# PRD Delta — Full-Spectrum Functional Completeness (Step 1 of the Full-PRD Planning Plan)

**Author:** John (PM) · **Date:** 2026-07-06 · **Status:** EXECUTED (operator-directed)
**Parent plan:** `full-prd-gap-map-and-planning-plan-2026-07-06.md` (Step 1 of 4)
**Operator directive (Lunarpulse, 2026-07-06):** *"Make the working project first, then worry about the ceremonies and community acceptance review. Let's make the project complete in functionality as I want in the PRD."*

---

## 1. The ratified decision: phase boundary

The operator directive resolves the Step-1 fork. A new engineering phase is inserted; the adoption phase is untouched but declared non-gating:

| Phase | Nature | Contents |
|---|---|---|
| **v2.2 — functional completeness** (NEW) | Engineering-owned; gates on nothing external | J3 Team Nexus journey closure; Reza Cortex journey closure; multi-tenant Loom (NFR-Ops-11/Tenancy-1); FR37 vetting machinery (internal keys); 100-host churn (NFR-Scale-2/Rel-7 second half); v2.0 remainder sweep |
| **v2.5 — ecosystem-adoption** (unchanged, now explicitly non-gating) | DevRel/BD; measured, never scheduled | External-author FKCS population, external N=12 cohort, accredited vetters, cert bodies, ≥20 external Spirits, consortium case study, RTL |

Rationale: everything the PRD *promises functionally* is engineering-ownable and lands at v2.2. Everything that depends on people we haven't met (authors, vetters, cert bodies, consortia) stays at v2.5 and can never block a ship. The Tier-3 consortium success criterion remains the falsifiable thesis milestone — tracked as release-gate evidence, not scheduled as stories.

Naming note: the parent plan's draft used "v2.5 = functional wave"; on contact with the documents (v2.5 already has a well-defined adoption-only identity used by ~10 NFRs and the scoping doc), the functional wave was numbered **v2.2** instead to avoid redefining an established term. Party-mode may rename at Step 2; the semantics are operator-ratified.

## 2. Changes applied (all marked `[DELTA-2026-07-06]` in place)

### `prd/user-journeys.md`
1. Journey roster (¶2): **J3 retagged v1.0 → v2.2** with correction note; **Reza promoted from "v2.0/2.5 candidate" to committed v2.2 target**; Diego annotated with the FR37 split.
2. J3 section: delta banner — v1.0 tag was dishonest (architecture §10.7.1 deferred it; Epics 1–11 never built it; Story 11.3 proved a churn envelope, not the journey); enumerates the missing J3 capabilities.
3. Reza section: delta banner — Epic 11 shipped the §10.7.2 enablers, not the journey; enumerates the outstanding v2.2 work.

### `prd/project-scoping-phased-development.md`
4. Phase-narrative (¶Phased delivery): v2.2 inserted; v2.5 marked non-gating by rule.
5. Resource table: v2.2 row added.
6. v1.0 milestone: delta note — the "J3 reproducible at v1.0" clause was not delivered (v1.0 J3 commitment was reduced to bilateral substrate readiness); retagged v2.2.
7. v1.0 localization line: ja/zh struck — deferred indefinitely (Epic 11 §8, 2026-06-29, post-Epic-10 fabricated-translation incident).
8. v2.0 milestone: **the long-open Q4 correction** — "FKCS protocol passes (3 external-author Spirits)" corrected to "FKCS-infrastructure passes (Chinese-wall proxy authors)" per NFR-Test-5's phase split; population stays v2.5.
9. v2.0 churn line: 100-host corrected to 25/30-host (matches NFR-Scale-2 cost-compression + ratified Epic 11 + delivered Story 11.3); 100-host retagged v2.2.
10. New **Phase v2.2 section** inserted before v2.5 (validation milestone: J3 scene + Reza scene reproducible on engineering-owned infra; zero FRs without a functional home).
11. Strategic Scope Commitments: three-way split recorded (v2.0 technical / v2.2 functional completeness / v2.5 adoption).

### `prd/non-functional-requirements.md`
12. **NFR-Rel-7**: full 100-host retagged v2.5 → v2.2.
13. **NFR-Doc-6**: ja/zh v1.5 target struck; indefinite deferral recorded with the re-introduction bar (real human translation + script-identity gate + native-reviewer runbook; "MT + script-pass" never claimable).
14. **NFR-Scale-2**: 100-host second half retagged v2.5 → v2.2; 25/30-host noted delivered (Story 11.3).
15. **NFR-Ops-11**: "v1.5+ (implemented)" corrected — did not happen at v1.5; full implementation retagged v2.2 (Reza prerequisite).
16. **NFR-Tenancy-1**: "out of scope before v2.5" → lands at v2.2; single-tenant-through-v2.0 guarantee unchanged; boundary stays loud.
17. Phase roll-up: v1.5 line ja/zh struck; new **v2.2 roll-up line** added; v2.5 line rewritten as adoption-only, non-gating.

### `prd/functional-requirements.md`
18. **FR37**: split — machinery (issuance/verification/journaling/revocation, internal vetter keys, closes Diego's final leg functionally) at v2.2; accredited external vetters (NFR-Comp-2 issuers) remain v2.5. Trace table updated.

### `prd/product-scope.md`
19. Supersession banner: file carries the pre-restructure phasing scheme; operative phasing is `project-scoping-phased-development.md`; version tags in this file must not be planned against.

### `maos-product-brief.md`
20. Delta banner: vision content canonical; journey numbering, 20-week timeline, and phase tags superseded by the PRD.

## 3. What was deliberately NOT changed
- No FR/NFR was weakened or removed — "no silent de-scoping" holds; every reclassification is a phase move with the original text preserved (strikethrough + note).
- v2.5 adoption content and the decoupling rationale are untouched.
- The ja/zh indefinite deferral was **recorded**, not relitigated.
- The two external v1.5 holds (pen-test, export counsel) are unaffected — they gate the shippable line, not planning.
- RTL stays v2.5 (no RTL locale targeted; not a functional promise of any journey).

## 4. Hand-off to Step 2 (full architecture, Winston + party-mode)
The PRD is now internally consistent for full-spectrum planning. Step 2 must disposition: the five Appendix-D terminal shapes (D.1 N-host topology — J3/Reza now demand a decision; D.2 rust-inproc; D.3 predicate stdlib/ADR-039; D.4 federation tier; D.5 multi-step migration); minimal-architecture open questions 1–10; ADR registry consolidation (planning 001–040 + implementation ≤049+); post-v2.0 KLOC ceiling (NFR-Maint-1 expires at v2.0); J3 mesh + multi-tenant Loom + FR37 flow architecture — all preserving the 8 foundational commitments + 14 invariants. Then Step 3 drafts epics (working shape: E12 = J3, E13 = Reza Cortex, E14 = ecosystem-proof infrastructure, E15 = v2.0 remainder sweep; 3–5 large demo-anchored stories each), and Step 4 runs the implementation-readiness check.
