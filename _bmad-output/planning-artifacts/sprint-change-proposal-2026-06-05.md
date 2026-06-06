# Sprint Change Proposal — 2026-06-05

**Type:** Direct Adjustment (planning-artifact registration; no scope/MVP change)
**Author:** Lunarpulse (dev) · **Decision body:** Winston (architect) + Murat (TEA) + security red-team
**Scope classification:** Minor → Moderate (artifact registration + one NEW backlog story; no replan)

---

## Section 1 — Issue Summary

**Problem.** Story 8.7 (*Fine-Grained Typed-Intent Consent Vocabulary over `maos-a2a-core`*) was `ready-for-dev` in `sprint-status.yaml` but had **no canonical home in the source-of-truth planning artifacts**. It existed only as:
- the epic-8 §AC-A6 "Noted gap" paragraph (Winston, 2026-06-04),
- a `sprint-status.yaml:90` comment, and
- a `deferred-work.md:226-227` entry.

There was **no `## Story 8.7` section** in `epic-8-…md`, **no entry** in `epics/index.md`, and **no node** in `epics/dependency-dag.md` — unlike Story 8.6, which received a formal split registration on 2026-06-04. Shipping a security-semantics change against an unregistered story is exactly the **tracker-vs-source-of-truth drift** the project's retros repeatedly flag (the "mechanical gates compound; promises decay" lesson).

**Discovery.** Surfaced while re-grounding the 8.7 story spec after Story 8.6 landed (`done`, 2026-06-05). During that work the team also resolved 8.7's four open design forks (Q2–Q5) by consensus, and the Q2 resolution **created a new committed follow-up** — fail-closed-for-cross-Host consent — that itself needed registering.

---

## Section 2 — Impact Analysis

- **Epic impact:** Epic 8 only. Two stories registered (8.7 formalized, 8.8 new). No change to other epics.
- **Story impact:** 8.7 gains a formal epic section + index + DAG node (status unchanged: `ready-for-dev`). 8.8 is NEW (`backlog`). Dependency edges added: **8.7 → 8.6** (done) and **8.8 → 8.7**.
- **Artifact conflicts resolved:** epic-8 markdown, `epics/index.md`, `epics/dependency-dag.md`, `sprint-status.yaml` were mutually inconsistent on 8.7's existence; now aligned.
- **Technical impact:** none from this proposal itself (documentation only). The underlying 8.7 work lands in `maos-a2a-core` (zero new crate, workspace stays 41, `maos-kernel-core` byte-identical 15505); 8.8 adds a discipline gate + a fail-closed flip, gated.

---

## Section 3 — Recommended Approach

**Direct Adjustment** — register both stories in-place, mirroring the 2026-06-04 Story 8.6 split. No rollback, no MVP-scope change. The work was already scoped (8.7 spec is `ready-for-dev`); this only reconciles source-of-truth with the tracker and records the committed end-state (8.8) so the dependency chain is auditable before `dev-story`.

**Effort:** ~1 session (documentation). **Risk:** negligible. **Timeline:** unblocks 8.7 `dev-story` immediately.

---

## Section 4 — Detailed Change Proposals

### 4.1 Epic-8 markdown (`epic-8-…miranash-v03-v15.md`)
- **ADDED** `## Story 8.7: Fine-Grained Typed-Intent Consent Vocabulary over maos-a2a-core` — short narrative (As-a/I-want/So-that) + a blockquote recording dependency on 8.6, the post-extraction grounding (`frame_intent_str` now `pub` → don't rename), and the team-consensus fork resolutions.
- **ADDED** `## Story 8.8: Fail-Closed-for-Cross-Host A2A Consent` — narrative + LOCKED precondition (the sender-completeness + fail-closed-readiness gate) + the security invariant.
- **EDITED** the §AC-A6 "Noted gap" blockquote: appended `✅ UPDATE (2026-06-05): RESOLVED — registered as Story 8.7 + 8.8…`.

### 4.2 `epics/index.md`
- **ADDED** TOC links for Story 8.7 and Story 8.8 after the Story 8.6 link.

### 4.3 `epics/dependency-dag.md`
- **ADDED** under `E8 Reference Spirits`: `Story 8.7 DEPENDS ON 8.6` and `Story 8.8 DEPENDS ON 8.7 + sender-completeness gate` arrows (re-flowed the tree connectors `├──→`/`└──→`).
- **EDITED** the v1.5 sprint invariant chain to append `→ Story 8.7 → Story 8.8`.

### 4.4 `sprint-status.yaml`
- **ADDED** `8-8-fail-closed-for-cross-host-a2a-consent: backlog` (with an explanatory comment) after the 8.7 row. 8.7 stays `ready-for-dev`. `last_updated` marker bumped.

### 4.5 Team-consensus fork resolutions (recorded in the 8.7 implementation spec; summarized here)
- **Q2 → Synthesis (3-0):** fine-grained-when-present NOW (transitional) + mandatory reference-sender population + **commit fail-closed-for-cross-Host as Story 8.8** behind a sender-completeness gate.
- **Q3 → (b) (3-0 floor):** additive `A2AIntent::is_canonical`/`parse` + `tracing::warn!` on unreachable entries; manifest-registry deferred (ADR-012 revisit trigger not fired).
- **Q4 → YES (3-0):** this registration.
- **Q5 → DELETE (2-1):** delete the dead `A2AConsentEnvelope` fail-open; accept a justified abi-diff Removed (ratified exception, flagged for Winston). Murat dissented on abi-discipline.

---

## Section 5 — Implementation Handoff

**Scope:** Minor (documentation) for this proposal; the downstream 8.7 build is Moderate (security-semantics in `maos-a2a-core`).

- **Story 8.7** → Developer agent (`dev-story`), recommended model `claude-opus-4-8`. Spec: `_bmad-output/implementation-artifacts/8-7-…md` (ready-for-dev).
- **Story 8.8** → remains `backlog`; opens once 8.7 lands AND its new sender-completeness discipline gate is GREEN-at-HEAD.

**Success criteria for this proposal:** epic markdown, `index.md`, `dependency-dag.md`, and `sprint-status.yaml` all reference Stories 8.7 + 8.8 consistently with correct dependency edges, and the §AC-A6 Noted-gap is marked resolved. ✅ All applied.
