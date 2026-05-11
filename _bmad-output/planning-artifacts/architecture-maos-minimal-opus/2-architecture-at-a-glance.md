# 2. Architecture at a Glance

## 2.1 The two-paragraph version

A **MAOS Host** is a single OS process running the MAOS kernel. The kernel exposes five services + two internal kernel modules and one contract. Spirits run as **subprocess-form binaries** at v0.1 (any language with a Spirit Wire Protocol implementation; the form third-party authors ship from day one). In-process Rust Spirits are gated by §13's measurement harness, not shipped at v0.1 (ADR-002). The kernel arbitrates nothing about Spirit cognition — it dispatches IAC frames, supervises lifecycle, mediates capability invocations, and journals every transition. Spirit-internal reasoning, distillation, plan revision, and posture inference are Spirit-side concerns by design.

Same-Host Spirits speak through the IAC mailbox. Different-Host Spirits speak A2A over mTLS+TOFU, in a bilateral 2-host pre-pairing pattern (used by the Diagnostic Engineer + Senior Architect operational pair at v1.5). Tools live behind the Capability Registry, which mediates every tool invocation through the Security Manager and the Approval Manager. The Memory Manager exposes three namespaces: per-Spirit private, shared (this Host), and collective (Loom-lite, a single Postgres+pgvector instance accessed via MCP-Streamable-HTTP). The Telemetry Stream is the perceptual organ; the Observer Spirit class is its canonical consumer.

## 2.2 The diagram

```
┌──────────────────── MAOS Host (one OS process) ─────────────────────┐
│                                                                      │
│  ┌─── Spirits (in-process Rust actors + subprocess children) ──────┐ │
│  │   Butler  Researcher  Architect(Nash)  Diagnostic(Mira)         │ │
│  │   Observer  + skill-package overlays (Orchestrator/Worker/      │ │
│  │   Reviewer for the founder loop)                                │ │
│  └──────────────┬──────────────────────────────────┬───────────────┘ │
│                 │                                  │                 │
│                 ↓ Spirit interface (manifest + IPC) ↓                 │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │            KERNEL — five services + two modules                  ││
│  │  Spirit Scheduler   Memory Manager   Security Manager            ││
│  │  Capability Registry   IAC Bus   I/O Subsystem   Telemetry Stream││
│  └──────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌──── Adapters ─────┬──────────────┬─────────────┬───────────────┐  │
│  │  Provider drivers │  MCP client  │  ACP server │  A2A peer     │  │
│  │  (Anthropic etc.) │  (tools)     │  (editors)  │  (bilateral)  │  │
│  └───────────────────┴──────────────┴─────────────┴───────────────┘  │
│                                                                      │
│  ┌──── Persistence ───────────────────────────────────────────────┐  │
│  │  SQLite (Transparency Log, Approval Decision Log, Journal)      │  │
│  │  OS keyring (secrets pass-through)                              │  │
│  │  Postgres+pgvector (Loom-lite collective tier — at v1.5)        │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘

       ┌────── Bilateral A2A (v1.5, J4 Mira-Nash) ──────┐
       │  Two Hosts pre-paired with mTLS cert pins.     │
       │  ADR-012 typed-intent consent at both ends.    │
       │  No discovery; no role queries; exactly two    │
       │  endpoints named in deployment configuration.  │
       └────────────────────────────────────────────────┘
```

## 2.3 The substrate's load-bearing claims

1. **Kernel/Spirit separation is enforced, not advisory.** Spirits never share a process address space without going through the IAC bus, never touch the filesystem outside the Memory Manager's namespaces, and never spawn tools outside the Capability Registry. This makes hot-swap-without-dropping-in-flight-tool-calls achievable as a kernel guarantee, not a Spirit-author convention.

2. **The kernel itself learns nothing.** Patterns, ADRs, fix templates, regression tests, dialectical updates — all live in user-space (Loom-lite). The kernel only enables propagation and audits the propagation. What gets propagated is the user's data, governed by the user's policy.

3. **Human transparency is a kernel invariant.** "No invisible actions, no puppeting, no asymmetric knowledge" is enforced by the Transparency Log and the Approval Manager, before any Spirit gets to author behavior. A Spirit cannot bypass this; the kernel will not deliver a frame the Transparency Log refused to record.

4. **Generality designed in.** A Spirit's binary contains lifecycle hooks, decision logic, and a system prompt — and nothing else. HTTP libraries, LLM provider SDKs, MCP clients, sandbox runtimes are kernel-provided adapters. Polyglot Spirit ecosystems become natural; uniform audit boundaries are a side effect.

5. **Epistemic halt as a Layer-1 capability.** When a Spirit's evidence is insufficient or contradictory, the kernel exposes a structured halt; the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. Hallucination becomes a user-mediated, audit-trailed event, not a silent regression.
