---
Status: binding-v0.1
Superseded-by: ADR-031 (single-form clause only; subprocess + ADR-032 substrate + T2 path reaffirmed)
Phase: binding-v0.1
Gate: §13 measurement gate (benches/iac_roundtrip.rs); promotion to inproc requires three-condition check + superseding ADR
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §13 measurement gate; ADR-031
---

# ADR-002 — Spirit form at v0.1 — subprocess only, inproc gated on measurement

**Decision.** v0.1 ships **subprocess form only**. Spirits run as subprocess binaries speaking the Spirit Wire Protocol (LSP-style `Content-Length` framing + CBOR payloads, ADR-032) over stdio. In-process Rust Spirits (`rust-inproc`) are **not** an alternative on the table at v0.1; they are a future option gated by §13's measurement harness.

**Rationale.** Subprocess gives polyglot reach and process isolation; it is the form Diego's `code-reviewer-pro` ships in, the form the Orchestrator/Worker/Reviewer skill-package overlays use, and the form that makes third-party Spirit publication safe. Adding a second form at v0.1 would double the invariant-enforcement surface (two crash recovery semantics, two memory models, two hot-paths) for a latency win no in-scope journey has been measured to require.

**Alternatives considered.** Two forms at v0.1 (`rust-inproc` + `subprocess`) — rejected: doubles ABI surface during the foundational phase; the operational complexity is not journey-justified. rust-inproc only — rejected: forces every Spirit author into Rust; kills polyglot ambition. Three forms (+ WASM-component) — rejected: third tier adds substantial toolchain complexity without journey-driving demand at this scope.

**Status reconciliation with §13 (Measurement Gate).** This ADR commits to subprocess-only IAC at v0.1. In-process transport is **not** an alternative on the table at v0.1; it is a future option gated by §13's harness (`benches/iac_roundtrip.rs`, journeys J1/J-Butler/J-Researcher). Promotion to inproc requires (a) sustained 24h breach of one threshold in §13's table, (b) confirmation that J-Butler p95 is not >4× J1 p95 (rules out fixable code overhead), and (c) a follow-up ADR superseding this one. Until those three conditions land in writing, "subprocess-only" is the architecture, not a default.

**What would force a revisit.** §13's measurement gate trips for a journey-required Spirit class, with the three-condition check satisfied. A capability-isolation requirement emerges that subprocess's process boundary cannot meet (in which case WASM-component, not rust-inproc, is the candidate). ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext` and resolves only when this revisit fires.
