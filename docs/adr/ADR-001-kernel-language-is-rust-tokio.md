---
Status: binding-v0.1
Phase: binding-v0.1
Gate: v0.1 ships in Rust + Tokio; alternative-language proposals require ADR + benchmark
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §13 v0.1 row
---

# ADR-001 — Kernel language is Rust + Tokio

**Decision.** The kernel is implemented in Rust on the Tokio async runtime. Spirit-side runtimes inherit this for the in-process form; subprocess and cross-form Spirits use language-neutral wire protocols.

**Rationale.** Type-safe invariants (the 14 invariants are easier to enforce structurally in Rust than in Go or TypeScript). Mature async runtime with work-stealing scheduler. Zero-cost abstractions for the hot path (token verify under 5µs P99). No GC pauses. The cohort survey confirmed the choice: codex, ironclaw, rustain are all Rust+Tokio.

**Alternatives considered.** Go (rejected: lack of trait-based zero-cost abstractions; GC pauses unacceptable on capability-token verify). TypeScript with Deno (rejected: no path to FIPS-validated crypto provider; runtime overhead). C++ (rejected: memory safety burden too high for a substrate kernel).

**What would force a revisit.** Rust's async story regresses materially relative to alternatives (unlikely). Tokio bifurcates and a fork becomes the standard (low probability).
