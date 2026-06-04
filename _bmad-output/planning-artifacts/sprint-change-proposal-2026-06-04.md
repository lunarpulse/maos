# Sprint Change Proposal — 2026-06-04

**Project:** maos  **Author:** Lunarpulse (via correct-course)  **Scope:** Minor (Direct Adjustment)
**Change:** Register Story 8.6 (split from Story 8.5) across planning artifacts.

## §1 Issue Summary

**Triggering story:** Story 8.5 — Mira+Nash Diagnostic-Architect Bilateral Pair (during `/bmad-create-story`).
**Issue type:** Technical limitation discovered during implementation/story-creation (a scope assumption that did not hold).

During substrate mapping for Story 8.5, the live `A2AProfile::CrossHost` TCP/mTLS transport was found **fully absent** — not scaffolding:

- `LoopbackA2ARouter` is in-memory `mpsc` only (`crates/maos-a2a/src/adapter.rs:84,284-289`); **no `TcpListener`/`TcpStream`** anywhere in `maos-a2a`.
- `A2AProfile::CrossHost` is an enum variant **never dispatched on** (no match arm branches on it in production code).
- JSON-RPC framing exists but is **never serialized to a socket**.
- Explicit deferral markers: *"the cross-Host TCP connector at v0.7"* (`transport/json_rpc.rs:127-130`), *"behavioral integration test against a real stalling TCP intake … deferred to follow-up"* (`tests/cross_host_consent_v1.rs:177-185`), FR23b *declared-not-implemented* (`src/lib.rs:13`).
- Building it ≈ **1000+ LOC of security-critical socket+TLS** in a **new crate** (`maos-a2a` is already over its own 1500-line ceiling at 2550) — a **distinct risk class** from Epic-8's reference-Spirit stories.

**Resolution (user-confirmed fork):** Split the work. Story 8.5 ships the **loopback-simulated** bilateral pair (proves the cross-Host *protocol*: pre-paired `PeerCertFingerprint`s, TOFU `verify_pinned`, ADR-012 consent, rotation chaos). A **new Story 8.6** ships the live two-process *transport*.

**Inconsistency corrected by this proposal:** the `8-6-…: backlog` entry had been added to the derived tracker (`sprint-status.yaml`) but not to the source-of-truth planning artifacts (epic-8 markdown, `index.md`, `dependency-dag.md`).

## §2 Impact Analysis

| Area | Finding |
|---|---|
| Epic 8 | Still completable as planned — Story 8.5 satisfies the epic's Mira+Nash deliverable (rotation-chaos + κ≥0.7) on existing substrate. Story 8.6 is **additive**, not a blocker. |
| Stories | ADD Story 8.6 to Epic 8. Depends on Story 8.5 (loopback pair) + Story 6.3 (A2A mesh). Slots into the v1.5 sprint after 8.5. |
| PRD | No conflict — FR23b (cross-Host) already anticipates this. No edit. |
| Architecture | No edit now. The new `maos-a2a-tcp` crate will touch the workspace-count sentinel + crate list in `4-kernel-design.md` **when 8.6 is built** (flagged in the 8.6 ACs). §7.2/7.3 already describe CrossHost FR23b. |
| UI/UX | N/A. |
| sprint-status.yaml | Already carries `8-6-…: backlog` (2026-06-04). No status change. |

## §3 Recommended Approach

**Direct Adjustment** — add the Story 8.6 definition to the three source artifacts so they match the tracker. No epic redefinition, no rollback, no MVP-scope change. Full rationale already lives in the Story 8.5 spec's Decision B/J.

## §4 Detailed Change Proposals (applied)

1. **`epics/epic-8-…miranash-v03-v15.md`** — appended a `## Story 8.6: Ship the Live Cross-Host A2A TCP/mTLS Transport (maos-a2a-tcp)` section (split note + As-a/I-want/So-that statement + 5 BDD acceptance-criteria blocks: real TCP+mTLS dispatch / JSON-RPC over socket / two-process mTLS connect with TOFU+consent / partition+rotation over real sockets / zero-kernel-KLOC + workspace reconcile).
2. **`epics/index.md`** — added the Story 8.6 TOC entry after Story 8.5.
3. **`epics/dependency-dag.md`** — annotated the E8 node with the 8.6→{8.5,6.3} dependency, and extended the v1.5 sprint invariant (#5) with Story 8.6.

(Verbatim inserted text recorded in the workflow transcript.)

## §5 Implementation Handoff

**Minor scope → direct implementation.** `sprint-status.yaml` already carries `8-6-…: backlog`; no status changes. Source-of-truth (epics) and tracker are now aligned.

**Next steps:**
- When ready to build Story 8.6: run `bmad-create-story 8-6-…` to produce its dedicated story file (like 8.5).
- Optional: `bmad-check-implementation-readiness` to confirm PRD/Arch/Epics/Stories alignment.

**Success criteria:** Epic 8 lists Stories 8.1–8.6; `index.md` and `dependency-dag.md` reference 8.6; `sprint-status.yaml` and the epic agree; Story 8.5 remains `ready-for-dev` with its loopback scope intact.
