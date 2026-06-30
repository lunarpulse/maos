---
Status: Accepted (architecture) — ratified 2026-06-29 (Epic 11 party-mode); binding-v2.0 deferred to Story 11.4b. Authored in the v1.5 hold-window per the Epic 11 ratification.
Gate: Story 11.4b — out-of-kernel detector with TP-floor + FP-ceiling on a live-syscall proven-red (no mock); kernel emission carries no verdict field (structural-not-semantic test); detector crate has no `maos-kernel-core` dependency (`check-service-boundary` / Story-1a.4 decoupling rule).
Decided: 2026-06-29
Accepted-in-PR: <PR_NUMBER>
Supersedes: none (resolves the `speculative-vNext` ADR-024 placeholder tracked in `architecture-maos-minimal-opus/12-architecture-decision-records.md`)
Revisits: NFR-Sec-3; ADR-006 §I9; ADR-038 (kernel-core ceiling)
---

# ADR-024 — Out-of-kernel sandbox-escape structural detector

**Decision.** The sandbox-escape *anomaly detector* (NFR-Sec-3) lives **outside `maos-kernel-core`**, in a user-space consumer. The kernel emits only **raw structural facts** ("a T2 child died on `SIGSYS` attempting syscall X"); it never stores, indexes, scores, or learns escape patterns, and it never classifies whether an alarm constitutes malice. All correlation (syscall divergence from the manifest declaration, fd-table growth, unexpected outbound IAC) and all interpretation happen in user-space. This is the direct application of ADR-006 ("the kernel learns no patterns") to NFR-Sec-3.

## Context

**NFR-Sec-3** (`_bmad-output/planning-artifacts/prd/non-functional-requirements.md:36`) requires, verbatim:

> Sandbox-escape **structural** anomaly detection (syscall pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections). **The kernel raises a structural alarm; the *interpretation* of whether the alarm constitutes malice is Spirit-side or operator-side. The kernel does not classify intent.** v2.0 (ADR-024). [STRUCTURAL-NOT-SEMANTIC clarification per Winston]

This ADR was a `speculative-vNext` placeholder (`docs/adr/index.md` reserved-list note; `architecture-maos-minimal-opus/12-architecture-decision-records.md`). The Epic 11 party-mode (2026-06-29, workflow `wyksr4yce`, decision §4) ratified that it be **authored first**, before Story 11.4b builds the detector, precisely so the kernel/user-space boundary is settled before any code is written.

### What the kernel already has (survey, 2026-06-29)

A prep survey of the live tree established the real seam — this ADR is grounded in it, not in an aspirational "TraceSink":

- **The structural fact exists.** `SandboxViolation { attempted_syscall, sandbox_tier }` is exit-status-derived (post-mortem, not per-syscall): `crates/maos-kernel-core/src/security/sandbox/mod.rs:73-78`, classified by `classify_exit` (`mod.rs:164-198`, maps `SIGSYS`→seccomp-kill/T2, `SIGKILL`→resource-cap/T2, Windows `0xC0000005`→T2). The audited form is `CapAuditEvent::SandboxBlock { spirit_pid, attempted_syscall, sandbox_tier }` (`crates/maos-capability/src/cap_audit/mod.rs:99-104`).
- **The sink-of-record is the Transparency Log, not a live stream.** `CapAuditWriter` drains the cap-audit channel and writes `FrameKind::SandboxBlock = 8` (`crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs:107-122`; discriminator pinned at `crates/maos-iac/src/adapter/transparency_log.rs:66`; writer wired at `crates/maos-bin/src/main.rs:2078`). The TL is a **write-then-query** store.
- **There is no subscribable sandbox-violation stream.** The only live broadcast is `scalar.tap` (`broadcast::Receiver<ScalarTapEvent>`, `crates/maos-kernel-core/src/telemetry/mod.rs:79-126`); its payload type `ScalarTapEvent` **structurally cannot carry** a `SandboxBlock`, and `publish_event` is never called for sandbox events. `LogRecallPort` (`crates/maos-domain/src/ports/log_recall.rs`) is participant-scoped and would `ScopeViolation` for a third-party observer reading another Spirit's frames.
- **The out-of-kernel TL-consumer pattern already exists and already resolves these frames.** `maos-audit` opens the TL SQLite file **read-only** (`SQLITE_OPEN_READ_ONLY`) — honoring the Story-1a.4 decoupling rule (no `maos-kernel-core` dependency) — and its `kind_from_string` already maps `"sandbox.block" | "SandboxBlock" => 8` (`crates/maos-audit/src/lib.rs:696`). A structural detector is the same shape of consumer.
- **Load-bearing gap:** the producer is **unwired in production**. `SecurityManagerAdapter::emit_sandbox_block` (`crates/maos-kernel-core/src/security/mod.rs:470-486`), the real `emit_t3_escape_block` (`security/sandbox/t3/cap_audit_bridge.rs:14-28`), and `classify_exit` have **no production call site** on the spawn/wait path; only a T3 `eprintln` probe is wired (`maos-bin/src/main.rs:5305`). No end-to-end test asserts that a real sandbox kill produces a `SandboxBlock` TL row.

**This refines the ratified Epic 11 plan.** Story 11.4b's row hypothesized "FLAG-Winston bounded to an emission-seam ONLY if `SandboxViolation` lacks a subscribable sink (verify at prep; possibly ZERO)." The verification result is: **it lacks a subscribable sink, and the producer is unwired** → the kernel delta for 11.4b is **not zero**. It is bounded, and this ADR scopes exactly what it is.

## Decision

### 1. The detector is a user-space consumer; the kernel learns no patterns

The escape detector lives in a new out-of-kernel crate (proposed `maos-escape-detector`) sited in the same read-only-TL-consumer space as `maos-audit`. It:

- reads the kernel's structural facts (the `SandboxBlock` frames, plus any bounded runtime structural emissions per §3) **and** the Spirit's manifest declaration;
- performs all correlation (declared-vs-actual syscall divergence, fd-table-growth threshold, outbound-IAC-vs-allowlist) and emits an **anomaly signal** with a structural rationale;
- has **no dependency on `maos-kernel-core`** (the Story-1a.4 rule; enforced by `check-service-boundary`).

The kernel stores no escape patterns, no fix templates, no per-Spirit history of "what an escape looks like." That curated knowledge is user-space (ADR-006). A kernel upgrade cannot corrupt it; an audit need not inspect kernel internals to review it.

### 2. The kernel emits only a structural alarm — never an interpretation

The kernel's contribution is the raw fact and nothing more. `CapAuditEvent::SandboxBlock` / `FrameKind::SandboxBlock` carry `{ spirit_pid, attempted_syscall, sandbox_tier }` — a structural description of what the OS enforcement layer observed. There is **no `malice`, `verdict`, `severity`, or `intent` field**, and this ADR forbids adding one to the kernel emission. "Is this malice?" is answered by the operator's policy or the watching Spirit, never by the kernel (NFR-Sec-3, the STRUCTURAL-NOT-SEMANTIC clarification). The structural alarm is necessary and sufficient for the kernel's role; the semantic judgment is delegated.

### 3. Consumption path = query/tail over the Transparency Log, NOT a new kernel push-stream

The detector consumes `SandboxBlock` (kind = 8) frames by **querying/tailing the TL read-only**, reusing the `maos-audit` pattern (`SELECT … WHERE kind = 8`). Rationale:

- The TL is already the sink-of-record for these frames; no new kernel surface is needed to read them.
- Building a kernel-resident push/subscribe stream for sandbox events would expand the kernel-core surface (a new broadcast topic + a broad-observer read capability) against ADR-038's ≤6 KLOC kernel-core ceiling and ADR-006's "mediate-and-audit, do not push" posture.
- A **live low-latency broadcast** (mirroring the `scalar.tap` `broadcast::Receiver` template at `telemetry/mod.rs:121`) is a **future, bounded option** — gated on a *measured* detection-latency requirement that the query/tail path fails to meet, and even then it carries only the structural scalar, never a verdict. It is explicitly **out of scope** for ADR-024's initial form.

### 4. The single authorized kernel delta in Story 11.4b = wire the producer (+ bounded raw-fact emissions if NFR-Sec-3 requires)

Because the producer is unwired (Context), Story 11.4b's authorized, FLAG-Winston-bounded kernel-core delta is exactly:

1. **Wire the existing producer onto the real spawn/wait path.** On child exit, run `classify_exit` and, on `Some(violation)`, call `emit_sandbox_block` so a real sandbox kill produces a real `SandboxBlock` TL row (closing the gap the survey found). This emits the *existing* structural fact; it adds no new field and no interpretation.
2. **Only if NFR-Sec-3's runtime signals are required** (fd-table growth, unexpected outbound IAC) — which `classify_exit`'s post-mortem exit classification cannot supply — add **bounded raw-fact emissions**, each FLAG-Winston, each emitting a single structural scalar (an fd count, a connection 5-tuple) with **no verdict**. The threshold logic and the allowlist comparison stay in the user-space detector. This is the "emission seam ONLY" the plan anticipated, now scoped: the kernel emits a number; the detector decides what the number means.

Every byte of this delta is recorded in `xtask/kernel-core-baseline.toml` HISTORY with the named surface (baseline 22964; Story 11.4b's row in the Epic 11 budget marks it "FLAG-Winston bounded to an emission-seam"). Out-of-surface churn is RED.

### 5. Considered and rejected

- **In-kernel pattern matcher / escape classifier.** Rejected: violates ADR-006 §I9 (turns the kernel into a state machine that accumulates "what an escape looks like"), expands the ≤6 KLOC ceiling, and makes the malice judgment a kernel concern in direct contradiction of NFR-Sec-3.
- **A kernel-resident broadcast topic for sandbox violations at v2.0.** Rejected *for the initial form*: no measured latency requirement justifies the new kernel surface; the query/tail path over the existing TL sink-of-record meets the structural-detection need. Retained as a future bounded option under a measured revisit (§3).
- **A `severity`/`malice` field on the kernel emission to "help" the detector.** Rejected: it is precisely the intent classification NFR-Sec-3 forbids the kernel from making; the structural-not-semantic test (Gate) asserts its absence.

## Consequences

- **NFR-Sec-3 is satisfiable without growing the kernel's responsibilities.** The kernel raises a structural alarm (existing `SandboxBlock` + bounded raw scalars); the interpretation is out-of-kernel. ADR-006 and ADR-038 both hold.
- **Story 11.4b's kernel delta is bounded and named** (producer-wiring + optional raw-fact emission seam), not zero — and not open-ended. The detector itself is zero-kernel.
- **The detector is auditable and replaceable** like `maos-audit`: read-only, no kernel dependency, reasoning fully in user-space where it can be inspected, tuned (TP-floor / FP-ceiling), and versioned without a kernel upgrade.
- **The structural-not-semantic boundary is mechanically enforced**, not merely asserted (Gate).

## Gate

Binding at **Story 11.4b** (binding-v2.0). The gate is:

- **No-verdict invariant (structural-not-semantic).** A test asserts the kernel sandbox-violation emission type carries no `malice`/`verdict`/`severity`/`intent` field. Adding one is RED.
- **Out-of-kernel invariant.** `maos-escape-detector` has no `maos-kernel-core` dependency (`check-service-boundary` / Story-1a.4 decoupling rule, the `maos-audit` precedent).
- **Detection quality.** TP-floor + FP-ceiling measured on a **live-syscall proven-red** (a real sandboxed child that actually trips seccomp/Job-Object enforcement — no mock); the Windows runtime proven-red is `windows-latest`-CI-only.
- **Producer-wired proven-red.** An end-to-end test asserts that a real sandbox kill produces a `SandboxBlock` TL row (the gap this ADR identifies must be closed and proven).
- Registered in `docs/adr/index.md`.

## Ratification

Architecture ratified by the Epic 11 party-mode consensus authority (Winston · John · Murat · Amelia + Lunarpulse sign-off, 2026-06-29, workflow `wyksr4yce`, decision §4), consistent with ADR-006 (kernel learns no patterns), ADR-038 (kernel-core ceiling), and ADR-047 §substrate-as-substrate (mechanisms, not assertions — the kernel records what happened, the operator judges what it means). The binding gate and the FLAG-Winston producer-wiring delta land at Story 11.4b. Authored during the v1.5 hold-window (a ratified hold-window carve-out: ADR authoring has no Epic-11-dev dependency).
