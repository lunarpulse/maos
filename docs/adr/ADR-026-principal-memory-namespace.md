---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only; runtime at v0.5) | subject-access query / right-to-be-forgotten (at v0.5)
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.2
---

# ADR-026 — Principal Memory Namespace

**Decision.** The kernel adds a typed namespace within the existing private-tier memory: `principal:<principal_id>:<spirit-author-defined-schema>`. Writes to this namespace are tagged as principal-related data and inherit three kernel-mediated operations: subject-access query, right-to-be-forgotten, redaction-on-export. The kernel does NOT interpret principal-namespace content; schema is entirely Spirit-author-declared.

**Rationale.** Privacy-aware Spirits (Butler watching the user's calendar; Researcher accumulating per-author bibliographies) need a namespace where principal data inherits the three operations. Without this primitive, every Spirit author would re-invent principal-aware curation.

**Prior art.** The principal-scoped memory model is informed by hermes-agent's principal-namespaced memory pattern lifted into a kernel-allocated contract. Hermes-as-application demonstrated the operational shape; MAOS lifts it into a kernel primitive so the substrate can offer the contract uniformly to any Spirit-author.

**Alternatives considered.** Spirit-author-handled principal scope (rejected: every Spirit re-invents the wheel). Dedicated principal-store as a new memory tier (rejected: tier inflation; the existing private tier suffices with the namespace tag).

**What would force a revisit.** A workload pattern emerges where the three operations are insufficient and a fourth is needed.
