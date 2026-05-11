# 0.6 Foundational Commitments

Eight numbered commitments that bind the substrate at v0.1. Every later section either implements one of these or defers to a phase column in §13. If a future ADR amendment proposes weakening any of these, the `invariant-lock` CI gate fires and a major-version bump is required (see ADR-037).

1. **Kernel/Spirit separation is enforced, not advisory.** Spirits never share process address space without going through the IAC bus; never touch the filesystem outside the Memory Manager's namespaces; never spawn tools outside the Capability Registry. *Implements:* ADR-001, ADR-010, ADR-011, ADR-030. *Invariants:* I1, I5.

2. **The kernel learns nothing.** Patterns, ADRs, fix templates, regression tests, dialectical updates — all live in user-space (Loom-lite). The kernel mediates and audits propagation; it does not store, index, or learn from the contents. *Implements:* ADR-006. *Invariant:* I9.

3. **Human transparency is a kernel invariant.** No invisible actions, no puppeting, no asymmetric knowledge. Every IAC frame writes the Transparency Log entry **before** delivery; auto-responses are stamped with `origin: spirit-auto`; approval decisions capture `(actor, target, capability, intent, decision, reasoning_if_any)`. *Implements:* ADR-037. *Invariants:* I2, I3, I4.

4. **One Spirit form at v0.1.** Subprocess-only over the Spirit Wire Protocol (LSP-style `Content-Length` framing + CBOR payloads). In-process Rust Spirits unlock only via §13's measurement gate (`benches/iac_roundtrip.rs`) and a superseding ADR; no rust-inproc form ships at v0.1. *Implements:* ADR-002, ADR-032. *Companion:* ADR-031 (`speculative-vNext`).

5. **Every external call is mediated through the Capability Registry.** The Registry is the only surface returned to Spirits at load time. The hot path (cap-tokens) and slow path (cap-audit) are decomposed; the audit path cannot block frame delivery. *Implements:* ADR-030. *Invariant:* I1.

6. **Capability tokens are unforgeable, short-lived, and bound to the issuing Spirit.** TTL ≤60s for high-privilege operations. Tokens bound to (Spirit-PID + boot-nonce + expiry); re-validated against current state at every use, not cached past state-change boundaries. No replay across processes. *Implements:* ADR-023.

7. **Epistemic halt is a Layer-1 capability.** Spirits compute their own scalars (variance, entropy, ensemble disagreement, confidence — Spirit-author choice); the kernel compares via four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`); the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. The kernel never introspects Spirit cognition. *Implements:* ADR-022 (the load-bearing ADR; halt is governed by the tagged-scalar/predicate contract, not by any I1–I14 invariant directly). *Calibrated by:* §6.6 safety-critical Spirit corpus methodology (currently scoped to §6.3 Mira and §6.4 Nash) and §6.1 `[epistemic_policy]` per-Spirit threshold declarations (non-safety-critical Spirits including Butler). Extension of this calibration to additional Spirit classes is non-binding in v0.1 and will be re-scoped when those classes are specified.

8. **Constitutional governance is structural, not procedural.** Amendments touching invariants I1–I14 require the `invariant-lock` CI gate (machine-checkable diff + corpus delta + phase-commitment update). Per-crate KLOC ceilings enforced by `tokei`; aggregate ≤20 KLOC kernel core, alarm at 16. *Implements:* ADR-037, ADR-038.

Every other commitment in this document either reduces to these eight or is explicitly phased (binding-v0.3+, binding-v0.5+, binding-v0.9+, binding-v1.0+, binding-v1.5+) per §12's index table and §13's roadmap.
