---
Status: binding-v0.1
Phase: binding-v0.1
Gate: TTL ≤60s for high-privilege; tokens bound to (Spirit-PID + boot-nonce + expiry); TOCTOU re-validation at use
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.3.4, §0.6 commitment 6
---

# ADR-023 — Capability-token TTL + bind-to-PID

**Status correction.** ADR-023 was previously tagged `binding-v1.5`. The token-binding mechanism (PID + boot-nonce + expiry, ed25519-signed) is required from v0.1 onward — without it, the Capability Token surface (ADR-030) has a replay vulnerability across Spirit restarts, which v0.1's Capability Registry mediation invariant (I1) cannot tolerate. The mechanism is implementation detail of v0.1's foundational commitment 6 (§0.6).

**Decision.** Capability-token TTL ≤60s for high-privilege operations. Tokens bound to (Spirit-PID + boot-nonce + expiry); audit-logged at every use with origin-Spirit-ID. Re-validation at use against current state, not cached state (TOCTOU correctness).

**Rationale.** Long-lived tokens are a replay-attack surface. Short TTL + PID binding makes token theft useless across process boundaries. Re-validation at use ensures posture changes during the token's lifetime are honored.

**Alternatives considered.** Long-lived tokens with revocation lists (rejected: revocation propagation latency too high). No expiry (rejected: replay-attack surface).

**What would force a revisit.** A workload pattern emerges where 60s TTL is too short for the task's natural duration.
