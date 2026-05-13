---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only; runtime at v0.5) | Capability Registry rejects digest writes with EDigestAuditChainMissing (at v0.5)
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §3.2 invariant I11; §9.5
---

# ADR-014 — Distillation audit-chain (introduces I11)

**Decision.** Add invariant I11. Every payload tagged `kind: digest` written to private/shared/collective memory carries non-empty `source_log_ref` (transitively flattened to original raw frames) and `distillation_depth`. Kernel rejects malformed writes with `EDigestAuditChainMissing`. Segment-level granularity is the default contractual unit; write-level audit is opt-in for forensic Spirits via manifest declaration.

**Rationale.** Distillation is a substrate-level pattern. Without an audit chain back to raw, the Transparency Log becomes ceremonial. Segment granularity keeps the audit path through 10K-writes/sec workloads without saturating fsync cadence.

**Alternatives considered.** Per-write audit by default (rejected: 10K writes/sec workloads stall on CAS contention). No audit chain (rejected: defeats the point of the Transparency Log).

**What would force a revisit.** A Spirit class needs forensic granularity by default and the segment-level option becomes too coarse.
