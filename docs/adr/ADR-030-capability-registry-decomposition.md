---
Status: binding-v0.1
Phase: binding-v0.1
Gate: hot-path token verify <5µs P99 benchmark
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.6
---

# ADR-030 — Capability Registry decomposition

**Decision.** The Capability Registry is internally split into four sub-services: `cap-tokens` (hot path, lock-free token issue/verify), `cap-policy` (consent rules + intent allowlist), `cap-audit` (transparency log writer, slow path), `cap-quota` (per-Spirit budget tracking). IAC traverses only `cap-tokens` on the hot path; the audit/lineage path is async via bounded MPSC.

**Rationale.** A monolithic Capability Registry is a god-service. Decomposing it preserves the Capability Registry as a single mediation surface from the Spirit-API perspective while internally separating the hot path from the slow path so audit writes do not block frame delivery.

**Alternatives considered.** Monolithic Capability Registry (rejected: serializes IAC hot path). Per-Spirit Capability Registry instances (rejected: cross-Spirit mediation becomes ad-hoc).

**What would force a revisit.** A new capability surface emerges that does not fit into the four sub-service shapes.
