# Epic 5 Context: Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Make Spirit operation safe and routine from v0.3 through v1.0: operators can control a Spirit’s lifecycle, upgrade it without losing the declared state needed to continue work, detect and disposition failures promptly, revoke compromised versions, and choose among supported model providers. The epic establishes the runtime guarantees and ecosystem adapters that let independent Spirit implementations remain portable, supervised, auditable, and bounded by declared policy.

## Stories

- Story 5.1: Ship Full Lifecycle Verbs and 11 Triggers with Priority-Weighted Scheduling
- Story 5.2: Implement Hot-Swap State Transfer and Cross-Major Migration Against HSIS ≥95%
- Story 5.3: Detect Spirit Crashes, Hangs, and Silent Failures with Halt-Receipt 99.9%
- Story 5.4: Run Spirit Upgrades and Propagate Signed Revocations in ≤5s
- Story 5.5a: Sandbox Tier T3 — Container Isolation via Docker / Podman
- Story 5.5b: Run the Multi-Provider CI Matrix Across Anthropic, OpenAI, and Ollama
- Story 5.5c: MCP Client + ACP Server — Tool Servers and Editor Hosts
- Story 5.5d: Spirit Registry over MCP-Streamable-HTTP with Three Trust Tiers
- Story 5.5e: §13.1 rust-inproc Measurement Gate — Subprocess vs In-Process Latency Decision

## Requirements & Constraints

- Support authenticated load, start, pause, resume, and unload through CLI, ACP editor, and operator HTTP control planes; lifecycle transitions must be journaled and lifecycle hooks must run within manifest-declared resource budgets. Scheduling is cooperative and priority-weighted, but CPU and memory limits require OS-level enforcement.
- Preserve in-flight capability tokens, working memory, and active halts for a valid hot-swap. Same-major additive state-schema changes are forward-compatible; same-major breaking changes are forbidden; cross-major stateful upgrades require a declared migrator. A failed transition or invariant breach must leave or restore a valid predecessor within 30 seconds.
- Meet the reliability floors: crash detection within 2 seconds and orphan notification within 5 seconds (at least 99/100 SIGKILL cases); stalled Spirits reclassified within 60 seconds after over 30 seconds without progress (at least 48/50); and silent-failure suspicion for healthy-heartbeat/no-progress cases (at least 45/50). Termination must produce halt receipts at least 99.9% of the time; graceful cold restart is at most 30 seconds without data loss, while a hard kill loses at most one in-flight message.
- Enforce the manifest-selected dead-Spirit task policy—NACK, replica reassignment, or operator escalation—and journal both exit cause and disposition.
- Provide hot, cold, and migrator-mediated upgrade policies. A signed CRL must support registry polling every five minutes and offline import; revocation blocks later use and notifies running Spirits, which follow their declared terminate, drain, or quarantine policy. Under 10⁴ concurrent token validations, propagation must be at most 5 seconds p99.
- Deliver T3 container isolation with Docker or Podman, retaining T2 protections and the strictest sandbox floor derived from manifest, trust tier, and operator policy. Image identity must be pinned and signature-verified; escape attempts must be fully blocked and audited.
- Run the same provider contract fixtures for Anthropic, OpenAI, and Ollama. Provider differences of 10% or more from the fixture median must be reported; an Ollama-only air-gapped configuration must issue no outbound provider calls.
- Achieve HSIS at least 95% per Spirit class across six 50-scenario corpora, with no CVSS-7-class invariant violations. Hot-swap same-major latency is P99 below 500 ms.

## Technical Decisions

- The scheduler uses supervised, bounded-mailbox Spirit actors without shared mutable state. Lifecycle supervision, failure isolation, backpressure, and state-preserving replacement are kernel responsibilities; Spirit behavior and migration logic remain Spirit-side.
- State transfer is CBOR encoded against a per-Spirit-class schema declared by `state_schema_uri` and `state_schema_version`. The coordinator validates compatibility before activation and uses saga-style compensation: a failed swap-out retains/restores the predecessor, and a failed swap-in discards the successor. Post-swap monitoring can auto-revert on state-shape, capability, or halt-continuity violations.
- Cross-major migration is explicit: the successor declares supported predecessors in `migrates_from` and provides `migrate(predecessor_state)`. If an archived predecessor exists without that declaration, loading fails with `EMigratorMissing` rather than guessing a conversion.
- Active halts are either resolved before a swap or migrated with identity, replay context, and resumption guarantees. The successor must declare compatibility with the predecessor halt schema; otherwise the coordinator rejects the operation with `EHaltContinuityViolation`.
- Provider access is through the kernel Inference Port and pluggable drivers, never vendor SDKs imported by Spirit binaries. The provider contract supports completion, streaming, and embedding; credentials are materialized at the capability boundary.
- MCP is the adapter protocol for external tools and the registry; Streamable HTTP is the production default while the client also supports stdio and SSE. ACP is an editor-hosting adapter using NDJSON over stdio. Do not add a new protocol or let domain logic depend on MCP or ACP types.
- The registry offers search, manifest, artifact, publish, and deprecate operations over MCP-Streamable-HTTP. Its v0.5 admission model has `local`, `org-internal`, and `public-untrusted` tiers; the strictest manifest/trust-tier/operator-policy floor wins. A yank is a registry publication event and remains distinct from a signed runtime revocation.
- The rust-inproc form is conditional, not presumed. Measure the subprocess form over at least 1,000 J1 and J4 invocations. If J1 P95 is at most 25 ms and J4 P95 at most 10 ms, defer rust-inproc; otherwise unlock it and require at least 90% cross-form semantic equivalence. An accepted ADR must record the measurement and decision before the v0.5 release.

## UX & Interaction Patterns

- Lifecycle and upgrade controls are authenticated operator actions available consistently from CLI, ACP-hosted editor workflows, and the operator HTTP API. ACP-hosted Spirits must have lifecycle and task/halt behavior equivalent to terminal-hosted Spirits; Zed and VSCode cover the complete editor-hosted lifecycle.
- Operators need inspectable outcomes rather than hidden automation: lifecycle/version transitions, task disposition, sandbox reasoning, revocations, and yanks are journaled and auditable. The sandbox inspection view reports runtime, image identity, applied protections, and the strictest-floor rationale.

## Cross-Story Dependencies

- E1b supplies the lifecycle hooks, sandbox infrastructure, audit spine, capability mediation, and frozen manifest groundwork used throughout this epic. E2 supplies ABI lifecycle-hook and migration declarations; E3 supplies the authenticated director/control-plane and notification surfaces.
- Story 5.2 depends on E4’s single-owner halt mechanism for halt-continuity validation. Story 5.3 depends on E4 halt receipts for its termination-rate measurement. E4 contributes 100 HSIS scenarios; Story 5.2 authors the remaining 200 for the six-class 300-scenario suite, and that authoring must complete before E4’s production halt-receipt gate closes.
- Story 5.4 consumes the hot-swap and migration paths from Story 5.2 and the cold-swap task disposition from Story 5.3. Story 5.1 provides the lifecycle substrate for the other runtime stories. Story 5.5e must run last in this epic because its measured go/no-go decision controls whether rust-inproc work is unlocked.
- E6 requires this epic’s lifecycle triggers and crash supervision for A2A peers; E7 builds on the registry over MCP-Streamable-HTTP. E10 later consumes E5’s 200 HSIS scenarios as part of its v1.0 ship-gate verification.
