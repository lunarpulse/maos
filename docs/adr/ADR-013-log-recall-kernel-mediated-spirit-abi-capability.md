---
Status: binding-v0.1; amended by ADR-058
Gate: participant-scoped log recall; cross-wall extension governed by ADR-058
Decided: 2026-04-15
Ported: 2026-07-21
Source: _bmad-output/archives/architecture-maos.md:1506-1514
---

### ADR-013 — `log.recall` as Kernel-Mediated Spirit ABI Capability

**Decision:** The kernel exposes a new Spirit ABI capability `log.recall(filter, limit, cursor) -> [frame_header]` and `log.fetch(frame_id) -> frame_payload`, scoped strictly to frames in which the calling Spirit is a participant (sender, receiver, or addressed by role). For A2A frames, the kernel additionally validates the ADR-012 typed-intent consent envelope before returning the frame; frames whose consent did not permit participant-recall are omitted, and frames whose consent permitted only header-recall return with payload elided. The capability is read-only, self-scoped (no cross-Spirit recall, no admin override, no "audit mode" for Spirits), and recall queries are themselves logged as IAC frames, producing a recall-of-recall chain. Permission gated by a new scope `log:recall:self` granted by default to all Worker, Orchestrator, and Cortex roles; revocable per-deployment.

**Alternatives considered:** `fs.read` on the Transparency Log file directly — rejected: violates I5 (memory scope) and I8 (A2A consent), gives any Spirit panopticon access to all frames on the host. Per-Spirit log partitions read via `fs.read` — rejected: either duplicates frames across partitions (storage and consistency cost) or destroys the single-totally-ordered-journal property that makes audit reconstruction tractable. An out-of-process audit daemon proxying queries — rejected: more LOC than kernel-side, adds a new trust boundary. No recall at all; Spirits keep everything they need in working memory — rejected: forces unbounded working-memory growth and contradicts the distillation pattern's premise.

**Rationale:** The Transparency Log is the substrate's audit spine (I2). It must be readable by participants for them to reason about their own history, but it must NOT be freely browseable by all Spirits sharing a host. Only the kernel knows the full participant graph and consent state, so only the kernel can scope recall correctly. The cost — one new ABI verb, ~200 LOC, one new permission scope — is small compared to the invariant erosion of the alternatives. The recall-of-recall chain ensures that even the audit access pattern is itself auditable. This primitive is foundational for the distillation pattern (§9.5): the Spirit can write digests to working memory and recall raw frames on demand for high-stakes decisions. Auditors are humans operating outside the Spirit sandbox — they read the log file directly with the operator's filesystem credentials. The kernel ABI never grants cross-Spirit recall; that keeps the blast radius bounded.

**What would force a revisit:** Per-frame consent envelopes evolve to include richer policies than typed-intent (ADR-012) supports — recall-policy may need its own envelope. Cross-host recall becomes a common operation and operators want a federated recall protocol — would warrant a new ADR rather than extending this one. The recall-of-recall chain produces unbounded log growth in practice — may need a "recall queries above tier-N are logged in compressed form" exception. A Spirit role legitimately needs cross-Spirit recall (e.g., a kernel-internal observability daemon) — handle by carving a separate capability, not by relaxing this one.

## Port correction note

This text is ported verbatim from the archived architecture generation. A newer registry reassigned **ADR-013** to the unrelated two-level `task.assign` decision; that is an ADR-number collision, not evidence that the `log.recall` decision was absent. ADR-058 is the new ADR explicitly anticipated by this record's cross-host revisit clause and narrowly amends the no-cross-Spirit-recall sentence through a separate, directional, manifest-consented capability.

The archived `log.recall` authority is the intended target of citations in `prd/user-journeys.md`, `prd/domain-specific-requirements.md`, `prd/project-scoping-phased-development.md`, `prd/developer-tool-specific-requirements.md`, `epic-4`, `maos-kernel-implementation-guide.md`, `spirit-development-and-sharing.md`, and `archives/architecture-maos-minimal-ds.md`. Those citations resolve to this port; they do not resolve to the later `task.assign` use of the same number.
