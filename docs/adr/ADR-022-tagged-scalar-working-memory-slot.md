---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only; runtime at v0.3) | [epistemic_policy] rules trigger halts via four universal-arithmetic predicates (at v0.3)
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.7, §4.6.1, §6.1, §6.2, §6.3
---

# ADR-022 — Tagged-scalar working-memory slot

**Decision.** Spirits write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics. Kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Spirit's `[epistemic_policy]` rules reference tagged scalars via these predicates; kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from).

**Rationale.** The tagged-scalar slot is the smallest theater-side primitive that lets the actor's epistemic state become legible to the kernel's halt mechanism without the kernel knowing what the actor is reasoning about. Theater-side primitive: minimal — one typed slot, two APIs, four universal-arithmetic predicate forms. Actor-side responsibility: total — Spirit decides what to track (variance, entropy, ensemble disagreement, KL, EFE, custom proxy), how to compute it, when to update it.

**Alternatives considered.** Kernel computes Spirit-specific scalars (rejected: violates §4.0.7 — kernel does no Spirit-specific cognitive computation). Spirit-author declares custom predicate functions (rejected: opens the kernel surface to arbitrary code execution).

**What would force a revisit.** A Spirit class needs to compare two scalars to each other rather than a scalar to a constant. (At that point: extend the predicate vocabulary additively, not redesign.)
