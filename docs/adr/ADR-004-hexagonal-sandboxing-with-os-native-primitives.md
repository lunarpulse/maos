---
Status: binding-v0.1
Phase: binding-v0.1
Gate: T0/T1 at v0.1; T2 at v0.3; T3 at v0.5; trust-tier floor enforced by Capability Registry
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.3.1, §8.2
---

# ADR-004 — Hexagonal sandboxing with OS-native primitives

**Decision.** Sandbox tiers T0 (trusted), T1 (UID separation), T2 (Landlock+seccomp narrow / Seatbelt / Windows restricted-token), T3 (T2 + container) form the security boundary. The strictest-of-(manifest, trust-tier, operator-policy) floor applies.

**Rationale.** OS-native primitives are production-grade (Landlock+seccomp on Linux 5.13+; Seatbelt's `.sbpl` profiles on macOS; restricted-token + Job Object on Windows). Codex has shipped all three in production. Adding a process-level container at T3 layers defense-in-depth without inventing new sandbox primitives.

**Alternatives considered.** WASM-component sandbox for Spirits (considered: capability-isolation by construction; rejected for this scope because subprocess + Ed25519 signing + T2 is sufficient for Diego's third-party publishing). Pure container-based isolation (rejected: containers do not give per-syscall granularity).

**What would force a revisit.** The OS sandbox primitives diverge sufficiently that maintaining all three becomes impractical.
