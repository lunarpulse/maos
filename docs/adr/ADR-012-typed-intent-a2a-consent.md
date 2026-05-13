---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only at v0.1; runtime at v0.9) | A2A Gateway rejects frames with intent not in allowlist (at v0.9)
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §7.2, §13 v0.9 row
---

# ADR-012 — Typed-intent A2A consent

**Decision.** Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`. A read-only Spirit cannot pass a payload to a writeable Spirit that, when interpreted, causes a write the read-only Spirit was forbidden from.

**Rationale.** Channel-consent does not imply transaction-consent. The confused-deputy class of attacks at the inter-Spirit boundary requires intent-class scoping. Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected. Without ADR-012, Mira could trigger a Nash-side action she cannot trigger directly.

**Alternatives considered.** Channel-consent only (rejected: leaves the confused-deputy gap open). Typed-intent at the IAC bus layer for ALL frames (considered: more uniform; rejected because cross-Host frames are where the trust boundary actually is, and same-Host IAC frames already inherit the kernel's process-internal trust).

**What would force a revisit.** A workload pattern emerges where intent-class cardinality grows pathologically.
