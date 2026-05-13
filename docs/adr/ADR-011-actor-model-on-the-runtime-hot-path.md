---
Status: binding-v0.1
Phase: binding-v0.1
Gate: per-Spirit Tokio task supervision + bounded mailbox
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.1
---

# ADR-011 — Actor model on the runtime hot path

**Decision.** Each Spirit is a Tokio-supervised actor with a bounded mailbox; no shared mutable state between Spirit actors. The seven kernel services are not actors — they are shared services with their own task pools.

**Rationale.** Four properties for free: backpressure via bounded mailboxes, no locks on the Spirit-to-Spirit hot path, failure isolation via Tokio task supervision, and natural hot-swap (replace `behavior` while preserving `state` and `open_tokens`). Codex's `AgentRegistry` + `Mailbox` is the precedent.

**Alternatives considered.** Shared-memory state (rejected: violates I5 and complicates hot-swap). Channel-only architecture without supervisors (rejected: failure handling becomes ad-hoc).

**What would force a revisit.** Tokio's supervisor primitives change materially.
