---
Status: binding-v0.1
Gate: structural-state lint blocks new persistent fields outside {Journal, TransparencyLog, CapabilityRegistry::tokens}
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §3.2 invariant I9; §9.3
---

# ADR-006 — The kernel learns no patterns

**Decision.** Patterns, ADRs, fix templates, regression tests — the curated collective knowledge — live in user-space (Loom-lite), not the kernel. The kernel mediates access and audits the access; the kernel does not store, index, or learn from the contents.

**Rationale.** Auditability. The kernel is replaceable; the user's data is not. If patterns lived in the kernel, every kernel upgrade would risk corrupting accumulated knowledge, every audit would have to inspect kernel internals, and the substrate's "boring substrate" claim would erode.

**Alternatives considered.** Build a kernel-resident pattern store (rejected: violates I9; turns the kernel into a state machine that depends on accumulated history).

**What would force a revisit.** A use case emerges where Loom-lite's MCP-Streamable-HTTP latency is unacceptable for a hot-path operation. (Threshold: p99 > 200ms on diagnostic-architect bilateral pair operations.)
