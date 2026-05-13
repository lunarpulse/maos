---
Status: binding-v0.1
Phase: binding-v0.1
Gate: crate boundary lint enforces port/adapter ring; domain core compiles without async runtime
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.1
---

# ADR-010 — Hexagonal architecture for static structure

**Decision.** The kernel is structured hexagonally: a domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring (concrete adapters for HTTP, stdio, mTLS, MCP, ACP, providers, persistence, secrets).

**Rationale.** Hexagonal gives multi-adapter-per-port flexibility (swap SQLite for Postgres without touching domain logic), testability (every port has a mock adapter), and keeps the domain core small. Clean Architecture's call-direction discipline does not fit a runtime kernel where the kernel calls into Spirit ABI traits as part of its control flow.

**Alternatives considered.** Clean Architecture (rejected: call-direction discipline contradicts the kernel-calls-into-Spirit-ABI inversion of control). Layered (rejected: less flexible for adapter-per-port).

**What would force a revisit.** A subsystem emerges where hexagonal's port abstraction is more friction than value.
