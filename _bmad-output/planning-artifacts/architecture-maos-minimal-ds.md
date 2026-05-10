---
title: 'MAOS — Modular Agentic Operating System: Architecture v1.0'
author: 'Winston (System Architect) for Lunarpulse'
date: '2026-05-06'
status: 'Pre-implementation architecture'
audience: 'Lunarpulse + early implementors'
---

# MAOS — Modular Agentic Operating System

## 0. Executive Summary

> **Working name.** "MAOS" tracks the project directory `~/dev_ws/maos` and reads naturally as "Modular Agentic Operating System".

MAOS is **a substrate, not a product**. It is the invariant scaffolding on which specialized agents — *Spirits* — are loaded, swapped, supervised, and composed the way processes are on a conventional OS. The kernel exposes one stable contract — the **Spirit ABI** — and every "agent class" you care about (Butler, Researcher, Diagnostician, Architect, Enterprise, Observer, …) is just a different manifest plus a different posture against that contract.

The design rests on five foundational commitments:

1. **Kernel/Spirit separation is enforced, not advisory.** Spirits never share a process address space without going through the IAC bus, never touch the filesystem outside the Memory Manager's namespaces, and never spawn tools outside the Capability Registry.
2. **Boring tech in the kernel; ambition in the contract.** Every kernel layer reuses something battle-tested: MCP for tools, ACP for editor bridges, A2A for peer communication, SQLite for local state, Postgres for shared state, Wasmtime+WIT for tool capabilities. The novelty is the Spirit ABI itself.
3. **The same primitives compose into single-user and paired deployments.** J1 Founder Loop and J4 Mira-Nash are not different products — they are two deployment topologies over one architecture.
4. **Human transparency is a kernel invariant.** "No invisible actions, no puppeting, no asymmetric knowledge" is enforced at the kernel level by the Transparency Log and the Approval Manager, before any Spirit gets to author behavior.
5. **The kernel itself learns nothing.** Patterns, ADRs, fix templates — the Loom-curated collective knowledge — live in user-space, not the kernel. The kernel only enables propagation; what gets propagated is the user's data, governed by the user's policy.

---

## 1. Reading Map

| Section | What's in it | Read if you're |
|---|---|---|
| §0 — Executive Summary | One-paragraph substrate thesis + five foundational commitments | Everyone |
| §2 — Architecture at a Glance | Diagrams, one paragraph each | Skimming |
| §3 — Vocabulary & Invariants | Pin down terms and what cannot change | Implementing |
| §4 — Kernel Design | Four services, interfaces, internal architecture | Implementing the kernel |
| §5 — Spirit ABI | The contract; manifest schema; lifecycle; wire protocol | Implementing the kernel or a Spirit |
| §6 — Reference Spirits | Four concrete Spirits as worked examples | Designing a new agent class |
| §7 — Inter-Agent Communication | Intra-host mailbox + point-to-point A2A + transparency | Designing multi-agent flows |
| §8 — Security & Approval Model | Sandbox tiers, capability tokens, approval classes, adversarial gates | Threat-modeling |
| §9 — Memory & Knowledge | Per-Spirit, shared, collective; Loom curation | Building memory features |
| §10 — Journey Traceability | How journeys map to §4–§9 primitives | Validating the design |
| §11 — Deployment Topologies | Single-user → Diagnostic-Architect pair | Sizing/operating |
| §12 — Architecture Decision Records | Contested decisions with rationale + revisit triggers | Disagreeing |
| §13 — Phased Roadmap | What ships in v0.1 / v0.3 / v0.5 / v0.8 / v1.0 / v1.5 / v2.0 | Planning |
| §14 — Open Questions | Things I'm not sure about, with consequence pairs | Pushing back |
| §15 — Release Gates | What must be true before each version ships | Releasing |

---

## 2. Architecture at a Glance

### 2.1 The two-paragraph version

A **MAOS Host** is a single OS process running the MAOS kernel. The kernel exposes four services (Spirit Scheduler, Memory Manager, Capability Registry, IAC Bus + Journal) and one contract (the Spirit ABI). At any moment, the Host runs **N Spirits** — subprocesses communicating via JSON-RPC over stdio, scheduled co-operatively against shared resource budgets. A Spirit is a **manifest + cognitive profile + memory scope + posture**, loaded from disk or pulled from a registry. The kernel never introspects what a Spirit is "thinking"; it only dispatches messages, supervises lifecycle, and enforces capability and approval policy.

Spirits on the same Host speak through the IAC mailbox. Two Hosts can communicate via point-to-point A2A over mTLS — used for the Mira↔Nash diagnostic-architect pair. Tools — including LLM providers, MCP servers, shell, file editors — live behind the Capability Registry, which mediates every tool invocation. The Memory Manager exposes three tiers: **per-Spirit private**, **shared (this Host)**, and **collective (cross-Host, via Loom-lite)**. The Transparency Log records every IAC interaction, every approval decision, and every capability use — append-only, personal to the user.

### 2.2 The diagram

```
┌──────────────────────────────────────────────────────────────┐
│                      HUMAN GOVERNANCE                         │
│         (decision gates, approval prompts, transparency log)  │
└────────────────────────────┬─────────────────────────────────┘
                             │
                  ┌──────────┴──────────┐
                  ▼                      ▼
┌─────────────────────────────┐  ┌──────────────────────────┐
│           SPIRITS            │  │   USER-SPACE SERVICES    │
│  (subprocess, manifest-driven│  │                          │
│   via JSON-RPC over stdio)  │  │  • Tool servers (MCP)    │
│  ┌─────────┐ ┌────────────┐ │  │  • Editor bridges (ACP)  │
│  │ Butler  │ │ Researcher │ │◀─▶  • A2A point-to-point    │
│  ├─────────┤ ├────────────┤ │  │  • Loom-lite (single PG) │
│  │Diagnostic│ │ Architect │ │  │  • Tool sandboxes        │
│  └─────────┘ └────────────┘ │  │  • Provider gateways     │
│        ▲                      │  └────────────┬─────────────┘
│        │  Spirit ABI (§5)     │               │
│        ▼                      │               │
└─────────────────────────────┘               │
                    ▲                           │
     ╔══════════════╧══════════════╗           │
     ║       MAOS KERNEL           ║           │
     ║   (invariant; Rust+Tokio)   ║           │
     ║                             ║           │
     ║  ┌───────────────────────┐  ║           │
     ║  │    Spirit Scheduler   │  ║           │
     ║  │  (subprocess lifecycle│  ║           │
     ║  │   + cgroups resource) │  ║           │
     ║  └───────────────────────┘  ║           │
     ║  ┌───────────────────────┐  ║           │
     ║  │    Memory Manager     │  ║           │
     ║  │  (three-tier, scope-  │  ║           │
     ║  │   enforced per I5)    │  ║           │
     ║  └───────────────────────┘  ║           │
     ║  ┌───────────────────────┐  ║           │
     ║  │  Capability Registry  │◀─╬───────────╝
     ║  │  (token issue/verify, │  ║  stdio JSON-RPC
     ║  │   approval gate,      │  ║  MCP, ACP, A2A
     ║  │   audit lineage)      │  ║
     ║  └───────────────────────┘  ║
     ║  ┌───────────────────────┐  ║
     ║  │  IAC Bus + Journal    │  ║
     ║  │  (mailbox + log +     │  ║
     ║  │   lifecycle journal)  │  ║
     ║  └───────────────────────┘  ║
     ╚═════════════════╤═══════════╝
                       ▼
     ┌──────────────────────────────────────┐
     │        HOST OS PRIMITIVES             │
     │  bwrap / seccomp / SQLite /          │
     │  Postgres / cgroups v2 / Wasmtime    │
     └──────────────────────────────────────┘
```

The same single Host scales out by configuration:

- **Single-user**: 1 Host, N Spirits, IAC mailbox, no A2A, no Loom.
- **Diagnostic-architect pair** (J4 Mira-Nash): 2 Hosts (prod edge + dev), point-to-point A2A, Loom-lite.

---

## 3. Vocabulary & Invariants

### 3.1 Vocabulary

| Term | Definition |
|---|---|
| **Host** | One OS process running the MAOS kernel. The unit of deployment. |
| **Kernel** | The four invariant services (§4) that every Host exposes identically. |
| **Spirit** | A loaded, running agent. State = (Manifest + Cognitive State + Memory Pages + Posture + Capability Token Set). |
| **Spirit Manifest** | The declarative file (TOML) describing a Spirit class: identity, role, model, memory scope, capability requests, posture, hooks. |
| **Spirit Class** | A manifest type (e.g., `butler`, `diagnostic-engineer`). Many *instances* of one class can be loaded simultaneously. |
| **Posture** | A Spirit's current autonomy stance: which approval class triggers a prompt, which sandbox profile is bound. **Posture is mutable; Spirit class is not.** |
| **Capability** | A typed tool surface — `bash.exec`, `fs.read`, `mcp.call(server, tool)`, `provider.complete(model)`, `a2a.send(peer)`, `git.commit`. Mediated by the Capability Registry. |
| **Capability Token** | An unforgeable ticket the kernel hands a Spirit when it requests a capability. Carries scope, expiry, and an audit ID. |
| **Approval Class** | One of six (§8.3): `readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. |
| **IAC** | Inter-Agent Communication. Two flavors: same-Host mailbox, cross-Host point-to-point A2A. |
| **Mailbox** | A bounded mpsc channel owned by a Spirit, addressable by Spirit ID, supervised by the kernel. |
| **A2A peer** | A remote Host reached over A2A protocol with mTLS and typed-intent consent. Point-to-point only. |
| **Loom-lite** | A user-space service (single Postgres instance) that curates patterns, ADRs, fix templates across Spirits — the "collective" memory tier. |
| **Transparency Log** | An append-only kernel-managed log of every IAC interaction, every approval decision, every capability use. Personal to the user. |

### 3.2 Invariants

These are the load-bearing rules. Every implementation choice exists to enforce one or more of them. Invariants exist in two states: **Enforced** (mechanically verified in code/CI) and **Declared** (design aspiration, promoted to Enforced at the phase where the journeys demand it).

| # | Invariant | Why it matters | Enforcement point |
|---|---|---|---|
| I1 | **Spirits cannot bypass the Capability Registry.** Every tool, network call, file op goes through it. | Without this, sandboxing, approvals, and audit are decorative. | Capability Registry is the *only* surface returned to Spirits at load time. |
| I2 | **Every IAC interaction is logged before delivery.** No ACK/NACK has ever been sent without an entry in the Transparency Log. | "No invisible actions." Without this, trust collapses. | IAC Bus writes log before it writes mailbox. |
| I3 | **Auto-responses are always marked `[auto-sent]` on both sides.** | "No puppeting." | IAC Bus stamps every message with `origin`. |
| I4 | **Approvals are persisted with intent, not just decision.** | Reproducibility, retract, audit. | Approval Manager stores `(actor, target, capability, intent, decision, ts)`. |
| I5 | **A Spirit's memory scope is declared in its manifest and enforced by the Memory Manager.** | Hot-swap requires knowing what to migrate vs. what to drop. | Memory Manager rejects reads/writes outside declared scope. |
| I6 | **Hot-swap preserves Capability Tokens for in-flight tool calls.** The successor inherits the token set via cooperative checkpoint. | J4 — Mira escalates to Nash without losing diagnostic context. | Token Lifecycle Manager; Hot-Swap Coordinator validates checkpoint schema before successor activation. |
| I7 | **Telemetry is broadcast; subscription is per-Spirit.** No Spirit polls the OS directly. | Observability is a perceptual organ, not a privilege. | Kernel event dispatch. |
| I8 | **Cross-Host A2A interactions require explicit consent at both ends, scoped to the typed intent of the message.** | J4's asymmetric trust; closes the confused-deputy gap. | A2A Gateway enforces sender-side policy AND receiver-side acceptance with intent-class allowlist before delivery. |
| I9 | **The kernel itself stores no secrets and learns no patterns.** | Auditability. The kernel is replaceable; the user's data is not. | Secrets pass through to OS keyring; KB lives in user-space (Loom). |
| I10 | **Every Spirit lifecycle transition is journaled; crash recovery rehydrates from the journal.** | Reliability. | Journal at the Spirit Scheduler. |
| I11 | **Persisted digests reference their raw source frames.** Every `kind: digest` payload carries non-empty `source_log_ref` and `distillation_depth`. | Without an audit chain back to raw, the Transparency Log becomes ceremonial. | Capability Registry validates fields on every digest-tagged write. Segment-level by default; write-level opt-in. |
| I12 | **Spirit decision frames record their working-memory digest references.** When a Spirit emits a `decision.*` frame, the kernel attaches `working_memory_digest_refs`. | Closes "the digest hid the critical finding → agent never recalled raw." | Capability Registry tracks per-Spirit in-context digest set via `log.recall` and attaches refs on emit. |
| I13 | **Digests carry intent provenance.** Every digest carries `intent_lineage` — the union of intent classes of all input frames it summarizes. | Closes consent-laundering through distillation. | Capability Registry validates `intent_lineage` field on digest writes; producer self-declares union from known inputs. |
| I14 | **Hot-swap preserves halt continuity.** When a Spirit with a non-empty `halt_set` is swapped, either every halt is drained before swap OR every halt is migrated with full resolution-path state. | Without I14, an in-flight halt can silently disappear during swap. | Hot-Swap Coordinator checks `halt_set` before swap; requires drain or schema-compatible migration. |

**Enforcement cadence — invariants graduate from Declared to Enforced per phase:**

| Phase | Newly Enforced | Declared (promoted later) |
|---|---|---|
| v0.1 | I1, I2, I10 | I3–I9, I11–I14 |
| v0.3 | I3, I4, I5, I7 | I6, I8, I9, I11–I14 |
| v0.5 | I11, I12 | I6, I8, I9, I13, I14 |
| v0.8 | I6, I13 | I8, I9, I14 |
| v1.0+ | I8, I9, I14 | none remaining |

If you find yourself bending one of these, you are not implementing MAOS — you are implementing something else. Stop and write a new ADR.

---

## 4. Kernel Design

The kernel is **one Rust+Tokio process** per Host. It exposes four services, internally arranged as Tokio tasks talking over typed channels, externally exposed to Spirits via the Spirit ABI (§5).

A Spirit is implemented as a **subprocess** speaking the Spirit Wire Protocol (JSON-RPC over stdio, §5.2). In-process Rust Spirits arrive at v0.8 and WASM-component Spirits at v2.0 per ADR-007.

### 4.0 Kernel Internal Architecture

#### 4.0.1 Architectural style — hexagonal + actor

**Static structure: hexagonal (ports & adapters).** The kernel has a clear domain (Spirit lifecycle, capability mediation, IAC routing, journal) separated from concrete I/O. Multiple adapter implementations exist per port — provider drivers per LLM vendor, sandbox backends per OS, transport adapters per protocol.

**Runtime hot path: actor model.** Each Spirit is an actor — mailbox-addressable, behavior-encapsulated, no shared mutable state with peers. This gives four properties: backpressure via bounded mailboxes, no locks on the hot path, failure isolation via Tokio task supervision, and natural swap (replace `behavior` while preserving `state` and `open_tokens` via cooperative checkpoint).

#### 4.0.2 The three-band layout

```
┌──────────────────────────────────────────────────────────┐
│                  Adapter Ring (blue)                      │
│  Provider drivers / MCP client / Sandbox backends         │
│  ACP server / A2A gateway / Persistence / Secrets         │
├──────────────────────────────────────────────────────────┤
│                  Kernel Services (yellow)                  │
│  Scheduler / Memory Manager / Capability Registry         │
│  IAC Bus + Journal                                       │
├──────────────────────────────────────────────────────────┤
│                  Domain Core (green)                      │
│  SpiritControlBlock / CapabilityToken / IacFrame /        │
│  Manifest / Invariants I1–I14 types — no I/O              │
└──────────────────────────────────────────────────────────┘
       ▲
       │  Spirit ABI
       ▼
┌──────────────────────────────────────────────────────────┐
│            Runtime Hot Path — actors orbit                │
│  Spirit A (subprocess)    Spirit B (subprocess)           │
└──────────────────────────────────────────────────────────┘
```

Three bands ordered by purity:

1. **Domain Core (green).** Pure data types, invariants, no I/O. Compiles without any async runtime, HTTP library, or database driver. Testable in isolation.
2. **Kernel Services (yellow).** The four services from §4.1–§4.5. Each is a Tokio task pool with its own internal state.
3. **Adapter Ring (blue).** Concrete implementations of ports defined by the kernel services. Provider drivers, MCP client, sandbox backends, persistence, secrets, ACP, A2A. Swappable per deployment.

**Spirits orbit, they don't nest.** Spirits sit alongside the kernel services and call into them via the Spirit ABI. The kernel orchestrates them but does not contain their behavior.

#### 4.0.3 Service dependencies

| Service | Depends on | Depended on by |
|---|---|---|
| **Spirit Scheduler** | Capability Registry, Memory Manager, Persistence | Control plane |
| **Memory Manager** | Persistence, Capability Registry | Spirit Scheduler, all Spirits |
| **Capability Registry** | IAC Bus (logging), Memory Manager (scope) | Every Spirit interaction with the world |
| **IAC Bus + Journal** | Persistence (Transparency Log + Journal) | Capability Registry, Spirit Scheduler, all Spirits |

#### 4.0.4 Module layout (Rust workspace)

```
maos/                                  # workspace root
├── crates/
│   ├── maos-core/                     # ★ DOMAIN CORE + KERNEL SERVICES
│   │   ├── domain/                    #   pure types, invariants — no I/O
│   │   ├── scheduler/                 #   Spirit Scheduler + journal + cgroups
│   │   ├── memory/                    #   tier dispatch + scope enforcement
│   │   ├── capability_registry/       #   tokens + enforcement + adapter dispatch
│   │   └── iac/                       #   mailbox + Transparency Log + Journal
│   ├── maos-spirit-abi/               # the Spirit ABI (traits + wire schemas)
│   ├── maos-spirit-sdk/               # SDK Spirit authors depend on
│   ├── maos-spirit-hello/             # J0 placeholder Spirit
│   ├── maos-spirit-butler/            # Butler Spirit (v0.3)
│   ├── maos-spirit-researcher/        # Researcher Spirit (v0.5)
│   ├── maos-spirit-observer/          # Observer Spirit (v0.5)
│   ├── maos-adapters/                 # ★ ADAPTER RING — extracted at v0.5
│   │   ├── providers/                 #   anthropic, openai, google, ...
│   │   ├── sandbox/                   #   linux (bwrap), macos (seatbelt)
│   │   ├── mcp/                       #   stdio + SSE + StreamableHTTP
│   │   ├── acp/                       #   stdio JSON-RPC server
│   │   ├── a2a/                       #   mTLS + TOFU + per-frame consent
│   │   ├── persistence/               #   sqlite + postgres
│   │   └── secrets/                   #   keyring + encrypted-file fallback
│   ├── maos-control-plane/            # operator surface (HTTP + Unix socket)
│   ├── maos-cli/                      # `maosctl`
│   └── maos-bin/                      # ★ COMPOSITION ROOT — wires everything
```

v0.1: 5 crates (maos-core, maos-spirit-abi, maos-spirit-sdk, maos-spirit-hello, maos-bin). Adapters are inlined into maos-core. Extraction happens at v0.5 when multi-provider and multi-protocol surfaces justify independent compilation.

#### 4.0.5 What's deliberately NOT in the kernel

- **No LLM SDK code.** Provider drivers wrap HTTP libraries.
- **No HTTP routing logic.** Control plane and A2A endpoints translate wire format to typed Spirit ABI calls.
- **No Spirit-class semantics.** The kernel never knows what "Researcher" means. It knows "this manifest declares these capabilities."
- **No Loom logic.** Loom is user-space. The kernel speaks MCP to it like any other tool server.
- **No model fine-tuning, eval suite execution, UI rendering.** Those are user-space concerns.

#### 4.0.6 What the Kernel Does NOT Compute

The kernel provides typed slots, universal-arithmetic comparators, lifecycle hooks, audit trail — under which Spirits perform whatever cognition their authors wrote. The kernel never computes variance, entropy, confidence, contradiction detection, or any other Spirit-specific cognitive measure. A Spirit that wants to halt on belief-variance computes the variance itself, writes the scalar, and calls `epistemic/halt`. The kernel only compares the Spirit-supplied scalar to the Spirit-author-declared threshold. **Theater stays neutral about acting methods.**

### 4.1 Spirit Scheduler

**Responsibility:** Lifecycle management for all Spirits on this Host.

**State:**
- `Map<SpiritId, SpiritControlBlock>` — the OS-style PCB analog
- `Journal` — append-only on-disk log of all lifecycle transitions (for I10)
- `ResourceBudgets` — per-Spirit caps on tokens/min, $/hour, parallel tool calls

**Operations exposed to user-space:**
- `load(manifest_path) → SpiritId`
- `start(SpiritId)`
- `pause(SpiritId)` / `resume(SpiritId)`
- `stop(SpiritId)` — graceful shutdown via lifecycle hooks
- `snapshot(SpiritId) → SnapshotId` / `restore(SnapshotId)` — v0.5+
- `swap(SpiritId, new_manifest_path)` — cooperative checkpoint at v0.8+
- `unload(SpiritId)`

**Spirit lifecycle FSM:**
```
Loaded → Started → Running → AwaitingApproval → Suspended → Unloaded
                   ↓
              EpistemicHalt (Spirit-initiated via epistemic/halt ABI)
```

**Scheduling discipline:** Cooperative, priority-weighted. v0.1–v0.5: single Spirit, no contention. v0.8+: cgroups v2 per-Spirit CPU/memory caps. A runaway Spirit gets throttled, not the host.

#### 4.1.1 Crash Matrix

The kernel supervises every subprocess Spirit. The following matrix defines failure boundaries.

| Crash Component | Detection | Worker Fate | Recovery |
|---|---|---|---|
| Kernel process (SIGKILL) | External supervisor (systemd) | All Spirits become orphans, receive SIGHUP→SIGKILL | External restart; kernel replays Journal on startup |
| Kernel process (panic) | Tokio panic hook | Spirits receive SIGPIPE on stdio → exit | Same as SIGKILL; panic message logged to stderr |
| Single Spirit (panic) | `waitpid` detects child exit; ≤2s | Other Spirits unaffected | Kernel emits `task.orphaned` IAC frame ≤5s with exit-cause journaled |
| Single Spirit (OOM) | `WIFSIGNALED(SIGKILL)` on child | Other Spirits unaffected | Kernel logs `reason: "oom_kill"`, reaps |
| Single Spirit (hung, no progress >30s) | Kernel hung-Spirit detector | Kernel emits `task.stalled` event ≤60s; SIGKILL at 120s | Journaled as timeout |
| Subprocess crash × open halt × in-flight CBOR | Supervisor's `JoinSet` returns `Err` | Halt reissued to successor only if CBOR snapshot `committed`; else halt poisoned | Torn frame at tail = truncate; torn mid-log = fatal corruption |
| Kernel → Spirit stdio broken | Broken pipe on write | Spirit gets SIGPIPE → exit | Journaled as `Halt::Fault(Truncated)` |
| A2A network partition (cross-Host) | Configurable timeout (default 30s) | In-flight frames NACKed | Kernel does NOT auto-retry; Spirit decides retry/escalate/halt |

**Crash detection:** ≤2s for SIGKILL. `task.orphaned` IAC frame ≤5s. Hung-Spirit detection ≤60s. All crash transitions journaled for I10 replay.

### 4.2 Memory Manager

**Responsibility:** Provide three named memory tiers, enforce scope from the manifest.

**Tiers:**

| Tier | Visibility | Backing store | Lifetime |
|---|---|---|---|
| `private` | One Spirit instance only | SQLite (per-Spirit DB) + JSONL transcript | Spirit life |
| `shared` | All Spirits on this Host | SQLite (Host-wide DB) + pgvector | Host life |
| `collective` | All Spirits in this Loom domain (cross-Host) | Postgres + pgvector + Loom-lite indices | Loom domain life |

**Manifest declares scope:**
```toml
[memory]
private  = { transcript = "rolling-90-days", vector = false }
shared   = { read = ["project_context", "team_calendar"], write = ["scratchpad"] }
collective = { read = ["patterns.architectural", "adrs"], write = ["incidents.diagnosed"] }
```

**Compaction** is per-Spirit, not kernel-wide. The Memory Manager exposes `compact_now(spirit_id)` as a control-plane API.

**Tool_use / tool_result pairing repair** is built into the kernel as a transcript-integrity invariant — a Spirit's transcript is always handed to its provider in a paired state, even after compaction.

**Memory file convention:** Every Spirit's `private` tier includes a writable `memory.md` at a fixed path.

### 4.3 Capability Registry

**Responsibility:** Mediate every Spirit→world interaction.

Capabilities split into **two layers**:

**Layer 1 — Kernel-primitive capabilities** are a closed enum, versioned with the kernel. Adding a new primitive requires a major-version bump.

The v1.0 kernel-primitive set:
- `provider.complete`, `provider.stream` — LLM provider calls
- `mcp.call(server_id, tool, args)` — the universal escape hatch for domain capabilities
- `bash.exec` — shell exec under a sandbox profile
- `fs.read`, `fs.write`, `fs.glob`, `fs.grep` — filesystem
- `iac.send`, `iac.subscribe` — same-Host inter-agent
- `a2a.send` — cross-Host point-to-point
- `memory.read`, `memory.write` — three-tier memory access
- `subspirit.spawn` — controlled child-Spirit creation
- `approval.request` — explicit human-in-the-loop checkpoint
- `posture.propose` — runtime posture change request
- `epistemic.halt` — Spirit-initiated: "I cannot proceed confidently"
- `log.recall` — raw frame retrieval from Transparency Log

**Layer 2 — Domain capabilities** (`git.commit`, `ci.deploy`, …) flow through `mcp.call(server, tool, args)`. A new Spirit class needing a new domain verb does not require a kernel change.

#### 4.3.1 Capability Tokens

Every capability request returns a typed token:
```rust
pub struct CapabilityToken {
    pub id: TokenId,
    pub spirit: SpiritId,
    pub pid: u32,                        // OS process ID of the Spirit
    pub boot_nonce: [u8; 16],            // per-kernel-boot salt, regenerated on restart
    pub capability: Capability,
    pub scope: Scope,
    pub expires_at: Instant,
    pub approval_class: ApprovalClass,
}
```

Tokens are bound to `(Spirit-PID + boot-nonce + expires_at)` — non-transferable across processes or kernel restarts. On every capability invocation, the kernel re-validates against current state (posture, sandbox tier, consent envelope) — no caching past state-change boundaries (TOCTOU correctness). TTL ≤60s for high-privilege operations.

#### 4.3.2 KLOC Ceiling

The kernel enforces a per-service source-code size ceiling as a structural guardrail against scope creep — not a security boundary.

| Kernel service | KLOC ceiling (v0.1) | Alarm at |
|---|---|---|
| Spirit Scheduler | ≤3 KLOC | 2.4 KLOC |
| Memory Manager | ≤3 KLOC | 2.4 KLOC |
| Capability Registry | ≤4 KLOC | 3.2 KLOC |
| IAC Bus + Journal | ≤3 KLOC | 2.4 KLOC |
| **Aggregate kernel core** | **≤20 KLOC** | 16 KLOC |

KLOC tracked as a **dashboard metric** via `tokei` in CI, not a CI gate. The per-service ceilings (§4.3.2) take priority over the growth-rate rule (§9.7.1): a service below its ceiling is never blocked regardless of growth rate. Exceeding a ceiling triggers: (a) a documentation update explaining why, (b) a scheduled refactoring story in the next sprint. Per-crate budgets in `xtask/kloc.toml`. CI measurement via `maosctl measure-kloc`.

#### 4.3.3 Secret-Redaction Filter

All kernel log output and IAC frame payloads pass through a stateless redaction filter before reaching the Transparency Log. Prevents accidental secret leakage through error messages, debug logs, or structured log events (I9 enforcement). Filter rules (ordered, first-match-wins): PEM private key blocks, `Bearer` auth headers, capability tokens, GitHub tokens, API key patterns, AWS access keys, GCP keys. Replaced with typed markers (`[REDACTED:api_key]`). Production canary system: 1000 canary-secrets/month; discovery latency ≤24h p95. Floor: 0 secrets in any logged frame.

The kernel itself stores nothing (I9). Secrets are materialized just-in-time from OS keychain (primary), encrypted file with master key (fallback), or plug-in providers (Vault, AWS/GCP — enterprise). Materialized secret values pass as `CapabilityScopedSecret` handles into Capability Tokens — never as raw strings into Spirit memory.

#### 4.3.4 Epistemic Halt — Spirit-Initiated

A Spirit declares "I cannot proceed confidently" by calling `epistemic/halt(payload)`. The kernel does NOT introspect Spirit uncertainty — the Spirit decides when to halt. The kernel's role:

1. Logs the halt to Transparency Log
2. Transitions Spirit to `EpistemicHalt` sub-state; in-flight tokens frozen
3. Surfaces a structured notification to the user
4. Returns `halt_id` to the Spirit

**Halt payload schema:**
```jsonc
{
  "gap_kind": "evidence_conflict | evidence_insufficient | source_unreliable | beyond_capability | resource_unavailable",
  "summary": "human-readable, 1–2 sentences",
  "evidence_so_far": "optional reference",
  "query_strategies": ["optional", "list", "of", "concrete next steps"],
  "confidence_at_halt": 0.42
}
```

**Resolution** via `epistemic/resolve(halt_id, resolution)`:
- `provided_context` — additional evidence attached
- `accepted_halt` — user agrees Spirit cannot proceed
- `authorized_override` — user accepts risk, Spirit proceeds with `override_marker`

**`[epistemic_policy]`** rules live in the manifest and are enforced by the Spirit code, not the kernel. The Spirit reads its own scalars, compares against thresholds, and calls `epistemic/halt`. The kernel provides the ABI method; the Spirit provides the judgment. `[epistemic_policy]` in the manifest is a declaration of intent — the Spirit's system prompt enforces it; the kernel's `output_shape` predicate verifies downstream marking.

#### 4.3.5 Approval Manager

The Approval Manager is the kernel's synchronous user-facing surface. Six approval classes, each mapped by posture to one of five behaviors: `silent_allow`, `notify_and_log`, `prompt`, `prompt_with_diff`, `deny`.

Three named posture presets ship by default — `cautious`, `assistive`, `autonomous` — and journey-specific presets extend these:

- **Mira posture (`sre-diagnostician`):** `readonly_*` = `silent_allow`, `mutating` = `notify_and_log`, `exec_capable` = `prompt_with_diff`, `control_plane` = `prompt`.
- **Nash posture (`principal-architect`):** `readonly_*` = `silent_allow`, `mutating` = `silent_allow` (within source repo), `exec_capable` = `prompt`, `control_plane` = `prompt`.

#### 4.3.6 Secrets

The kernel itself stores nothing (I9). Secrets are materialized just-in-time from OS keychain (primary), encrypted file with master key (fallback), or plug-in providers (Vault, AWS/GCP — enterprise). Materialized secret values pass as `CapabilityScopedSecret` handles into Capability Tokens — never as raw strings into Spirit memory.

### 4.4 Wire Protocol Golden Corpus

The kernel ships with a **wire-protocol golden corpus** of test fixtures for the Spirit Wire Protocol. Each entry is a self-contained test case that a conformant SDK implementation must successfully serialize/deserialize to byte-equal CBOR. (Distinct from the §5.2 per-frame-variant golden corpus — this corpus tests SDK conformance; §5.2 tests per-method wire encoding.)

#### Corpus Structure

```
maos/testdata/golden/
├── frames/
│   ├── lifecycle.load.json          # canonical reference shape
│   ├── lifecycle.load.cbor          # canonical encoding
│   ├── capability.request.json
│   ├── capability.request.cbor
│   ├── epistemic.halt.json
│   ├── epistemic.halt.cbor
│   └── ...                          # per frame variant
├── corpus.toml                      # manifest
└── README.md
```

#### Manifest Format (`corpus.toml`)

```toml
[[entry]]
name = "lifecycle-load"
wire_method = "lifecycle/load"
direction = "kernel_to_spirit"
language = "reference"

[[entry]]
name = "epistemic-halt"
wire_method = "epistemic/halt"
direction = "spirit_to_kernel"
language = "reference"
  [entry.expected]
  halt_id_format = "ulid"
  confidence_range = [0.0, 1.0]
```

#### Polyglot Coverage Goal

| Language | Frame variants | Phase |
|---|---|---|
| Rust (reference) | All 26 methods | v0.1 |
| Python | All 26 methods | v0.5 |
| TypeScript | All 26 methods | v1.0 |
| Go | All 26 methods | v1.0 |

#### CI Integration

Each language SDK must pass `golden-verify` against the corpus: serialize constructed frame to byte-equal CBOR, deserialize golden to structurally-equal frame. Floor: 100% per frame variant per SDK at v1.0 ship gate. Canonical encoding: sorted keys, no whitespace, UTF-8 NFC.

### 4.5 IAC Bus + Journal

**Responsibility:** Same-Host Spirit communication + Transparency Log + lifecycle journal.

#### 4.5.1 Same-Host: the mailbox

Every Spirit has exactly one inbox (typed mpsc channel) and unlimited outbox sends.

**Frame shape:**
```rust
struct IACFrame {
    id: FrameId,
    sender: SpiritId,
    recipient: Recipient,    // SpiritId | Broadcast
    kind: FrameKind,         // request | response | notification | escalation | retract
    payload: serde_json::Value,
    intent: Option<String>,
    sent_at: Instant,
    response_to: Option<FrameId>,
    origin: FrameOrigin,     // human-authored | spirit-auto | spirit-drafted-human-approved
}
```

**Delivery guarantees:** At-least-once within Host. Per-(sender, recipient) FIFO. Across-sender ordering is arrival-interleaved, not deterministic.

**Backpressure:** If a recipient's mailbox is full, the sender's `iac/send` blocks until space.

#### 4.5.2 Cross-Host: point-to-point A2A

Used for J4 Mira↔Nash pair. JSON-RPC over HTTPS with mTLS and TOFU. Fixed endpoints (no peer discovery). Frame-level consent gates: every frame goes through both ends' Approval Manager (sender-side outbound policy; receiver-side inbound policy) with ADR-012 typed-intent consent.

**Intent vocabulary:** `diagnosis-handoff:read-only-evidence`, `telemetry-query`, `deploy-proposal`, `architecture-consultation`. Receiver's allowlist determines what's accepted.

**Operational failure handling:**

| Failure mode | Kernel behavior | Spirit-level expectation |
|---|---|---|
| Network partition (in-flight frame) | NACK after configurable timeout (default 30s) | Spirit decides retry/escalate/halt; kernel does NOT auto-retry |
| Partial-consent failure (sender approved, receiver rejected mid-frame) | Typed `ConsentRupture` event emitted; frame quarantined, not delivered, not silently dropped | Sender receives `ConsentRupture` IAC frame; operator surface logs for forensic review |
| mTLS cert rotation mid-session | In-flight conversations survive; new connections use rotated cert | Rotation under load must produce zero conversation drops; revocation latency median ≤60s, p99 ≤5min |
| Logical-clock skew (cross-Host frame ordering) | Lamport or hybrid logical clock for cross-Host frame ordering; wall-clock is metadata only | Ordering consistent under clock skew |
| Peer unreachable | Frames NACKed after timeout | Application layer (Orchestrator or peer Spirit) decides escalate/halt |

#### 4.5.3 Transparency Log

Append-only SQLite database at `~/.maos/transparency/log.db`. Every IAC frame writes an entry **before** delivery (I2). Every entry has: `event_id`, `ts`, `direction`, `peer`, `intent`, `origin`, `outcome`, `payload_hash`. Visible only to the owning user.

**The retract primitive** writes a *new* entry referencing the original — it does not mutate the original.

#### 4.5.4 Journal

The Journal records every Spirit lifecycle transition (I10). Crash recovery rehydrates from the journal — the Spirit Scheduler replays the journal on startup, restoring the Spirit roster and in-flight capability tokens.

---

## 5. Spirit ABI

The contract between kernel and Spirit. Stable across kernel versions within a major.

### 5.1 Spirit Manifest schema

A Spirit class is **fully declared** in a single TOML file. Implementation code lives elsewhere (a subprocess binary). A Spirit's implementation contains lifecycle hook handlers, IAC frame handlers, decision logic, the system-prompt template, and output/explanation/epistemic predicate callbacks. **It does not contain HTTP libraries, LLM provider SDKs, MCP client implementations, socket code, or filesystem code.** All infrastructure flows through Layer 1 capabilities which the kernel implements.

```toml
[identity]
class      = "butler"
version    = "1.2.0"
display    = "Proactive Personal Agent"
icon       = "🎩"
maintainer = "Lunarpulse <lunarpulse@gmail.com>"

[implementation]
runtime  = "subprocess"
binary   = "spirit-butler"
spirit_wire_protocol_version = "1.0"

[cognitive]
default_model    = "claude-sonnet-4-6"
escalation_model = "claude-opus-4-7"
system_prompt    = "spirits/butler.system.md"
prompt_caching   = true

[memory]
private    = { transcript = "rolling-30-days", vector = true }
shared     = { read = ["calendar", "inbox", "projects"], write = ["butler-scratchpad"] }
collective = { read = ["user.preferences"] }
memory_md  = "agents/butler/memory.md"

[capabilities.required]
"provider.stream" = { models = ["claude-sonnet-4-6", "claude-opus-4-7"] }
"fs.read"         = { roots = ["~/Documents", "~/Notes"] }
"fs.write"        = { roots = ["~/.maos/butler"] }
"iac.send"        = { peers = ["any"] }

[capabilities.optional]
"mcp.call" = { servers = ["calendar-google", "gmail", "todoist"] }
"a2a.send" = { peers = [] }

[posture]
preset = "assistive"
prompt_on = ["mutating", "exec_capable", "control_plane"]
silent_allow = ["readonly_scoped", "readonly_search"]
auto_response_marker = "[butler-auto]"

[sandbox]
profile = "t1-permission-gate"

[budget]
tokens_per_hour = 100000
spend_per_day_usd = 5.00
parallel_tool_calls = 3
warn_at_pct = 0.80
on_breach = "stop"

[hooks]
on_load    = ["spirit-butler::hooks::on_load"]
on_idle    = ["spirit-butler::hooks::on_idle"]

[output_shape]
predicates = [
  { kind = "json_schema", tags = ["butler.suggestion"], schema = "spirits/butler.suggestion.schema.json" },
  { kind = "regex_required", tags = ["butler.notification"], pattern = "\\[butler-(auto|drafted)\\]" },
  { kind = "callback", tags = ["butler.daily_summary"], fn = "verify_daily_summary" }
]

[explanation_shape]
required_for_origins = ["spirit-auto"]
required_for_classes = ["mutating", "interactive", "exec_capable"]
schema = "spirits/butler.explanation.schema.json"

[epistemic_policy]
default_action = "verbalize_only"
on_override = "proceed_with_marker"
default_query_strategies = "spirits/butler.queries.md"

[[epistemic_policy.rules]]
tag = "claim.load_bearing"
on_confidence_below = 0.7
action = "halt"
on_evidence_conflict = "halt"

[[epistemic_policy.rules]]
tag = "claim.exploratory"
on_confidence_below = 0.3
action = "flag"
on_evidence_conflict = "flag"

[[epistemic_policy.rules]]
tag = "speculation"
action = "verbalize_only"

[[epistemic_policy.rules]]
tag = "conversational"
action = "verbalize_only"
```

**Budget semantics:** Every cap in `[budget]` is enforced by the Capability Registry. At `warn_at_pct` the Scheduler emits a notification. At 100% the configured `on_breach` takes effect (default `stop`).

**`output_shape`** runs at the Capability Registry between `iac/send` and IAC Bus delivery. Frame delivery is refused for tagged frames that fail declared predicates. Three predicate `kind`s: `json_schema`, `regex_required`, `callback`.

**`explanation_shape`** gates proactive (`spirit-auto` origin) actions. Every proactive action carries a "because" payload the kernel-rendered notification surface displays. Converts "the AI did something" into "the AI did something *because of these reasons*."

### 5.2 Spirit Wire Protocol

Subprocess Spirits speak **JSON-RPC 2.0 over stdio** with **LSP-style Content-Length framing**.

**Framing:**
```
Content-Length: <byte_count>\r\n
\r\n
<json_payload>
```

- `<byte_count>` — decimal integer, UTF-8 byte count of `<json_payload>`
- `\r\n` — CRLF, exactly as specified
- `<json_payload>` — single JSON-RPC 2.0 request, response, or notification (CBOR encoding)
- Max header block: 4 KiB. Max payload: 16 MiB (frames exceeding = `WireError::Oversize`, Spirit halted)
- Implicit content type: `application/vnd.maos+json; charset=utf-8`
- Stderr piped separately to kernel tracing at `WARN` level — never multiplexed onto stdout
- EOF after last full frame = `Halt::Voluntary`. EOF mid-frame = `Halt::Fault(Truncated)`
- SIGTERM → 5-second grace period → SIGKILL
- Backpressure: `BufReader` cap 1 MiB, bounded `mpsc<Frame>(64)` — channel full = backpressure to caller, never drop

**Kernel → Spirit:**
- `lifecycle/load(manifest)` — initialize
- `lifecycle/start(snapshot?)` — begin processing
- `event/inbound(frame)` — IAC frame arrived
- `lifecycle/pause()` / `lifecycle/resume()` / `lifecycle/unload()`
- `epistemic/resolve(halt_id, resolution)` — user responds to prior halt

**Spirit → Kernel:**
- `capability/request(capability, scope) → token`
- `capability/invoke(token, args) → stream(events)`
- `capability/release(token)`
- `iac/send(target, frame, mode)` — `mode = sync | async | broadcast`
- `iac/retract(message_id, reason)`
- `memory/read(tier, key)` / `memory/write(tier, key, value)`
- `approval/request(class, intent, payload) → decision`
- `posture/propose(new_posture)`
- `subspirit/spawn(manifest, scope) → SpiritId`
- `epistemic/halt(payload) → halt_id` — Spirit-initiated
- `log/recall(filter, limit, cursor) → [frame_header]` — v0.5+

**Error codes:**

| Code | Meaning | When emitted |
|---|---|---|
| -32700 | Parse error | Invalid JSON, framing violation |
| -32600 | Invalid Request | Missing `jsonrpc` or `method` |
| -32601 | Method not found | Unknown method name |
| -32602 | Invalid params | Parameter type/content mismatch |
| -32603 | Internal error | Unclassified kernel failure |
| -32001 | Invalid token binding | Token validation failure (§4.3.1) |
| -32002 | Tier capability denied | Token lacks required sandbox tier |
| -32003 | Resource limit exceeded | Timeout, memory, or file descriptor cap hit |

**Wire stability policy:** Method set stable across kernel minor versions within a major. Adding/removing/changing a method is a major-version bump. N–1 compatibility maintained within a major. Wire protocol marked unstable 0.x through v0.3, frozen at v0.5.

**Golden corpus:** Every frame variant ships with `golden/<frame_name>.json` (authoritative reference) and `golden/<frame_name>.cbor` (canonical encoding). Per SDK per frame variant: 100% byte-equal conformance at v1.0 ship gate. Cross-language implementations must serialize to byte-equal CBOR. (Distinct from §4.4 wire-protocol golden corpus — this corpus tests per-method wire encoding; §4.4 tests polyglot SDK conformance.)

### 5.3 Lifecycle hooks

| Hook | When fired | What it must produce |
|---|---|---|
| `on_load` | Manifest accepted, before any I/O | Runtime resource allocation; no side effects |
| `on_start` | Spirit becomes runnable | Optional kickoff message into its own mailbox |
| `on_idle` | No work for ≥ 30s (configurable) | Proactive opportunity (Butler); else no-op |
| `on_pause` | Suspended; tokens remain valid for `pause_tolerance` | — |
| `on_resume` | Resuming after pause | — |
| `on_unload` | Shutdown imminent | Final transcript flush; release tokens |

`on_idle` is the hook that defines the Butler. A Butler Spirit subscribed to calendar/inbox telemetry uses `on_idle` to scan upcoming meetings, check email digests, draft replies.

### 5.4 Posture

A Spirit's posture is a **runtime-mutable** projection over its manifest. The user can shift a Spirit from `cautious` to `autonomous` without unloading and reloading. Posture changes:
1. Are logged to the Transparency Log.
2. Cannot exceed the manifest's declared capability ceiling. Posture restricts; it cannot expand.
3. Trigger a `posture/changed` event so peers can update their consent rules.

---

## 6. Reference Spirits

Four Spirits ship across v0.3–v1.5. Each is a full manifest plus an implementation binary plus a system prompt. These are the architecture's existence proofs — each Spirit exercises a specific subset of the kernel's primitives and demonstrates a concrete posture configuration.

### 6.1 Butler — Proactive Personal Agent (v0.3)

**Class:** `butler`. **Identity:** Lives at idle, anticipates needs, pre-stages solutions.

**Scene.** Tuesday, 5:48 PM. The user is mid-flow on a design task. The Butler's `on_idle` hook fires. Calendar shows 7 PM dinner; last 4 occurrences the user was 2 attended late + 2 missed entirely. Slack status "Heads down" since 1 PM. The Butler computes a calendar-conflict-confidence scalar (its own definition — predicted-miss-probability × pattern-strength), writes it via `working_memory.set_scalar("user.calendar_conflict.confidence", 0.85, derived_from=[calendar_obs, slack_obs])`. Per its `[epistemic_policy]`: `tag = user.calendar_conflict.confidence, on_value_above = 0.8 → action = verbalize_with_options`. The kernel compares the Spirit-supplied scalar (0.85) to the threshold (0.8) via universal arithmetic and fires the action. The Butler surfaces a single notification: pattern noticed, predicted disengage time, three options offered.

**Cognitive profile:** Mid-tier model (Sonnet). Good at routing, calendar reasoning, prioritization. Escalates to Opus only when user accepts an "elaborate this" prompt.

**Memory scope:** Broad read (calendar, inbox, project list, recent files); narrow write (its own scratchpad).

**Capabilities:** Read calendar/inbox/file-system; write notifications; send IAC to other local Spirits; send A2A only with consent. MCP integrations: Calendar, Slack, Linear, Figma.

**Posture:** `assistive` — silent on reads, prompts on mutations, never on `exec_capable`.

**Lifecycle hook anchor:** `on_idle`. The Butler runs its anticipatory reasoning loop whenever the kernel calls `on_idle` (no pending IAC frames + user activity stream shows >12 minutes since last meaningful interaction).

**Output shape:** Notifications carry structured `{pattern, confidence, evidence, options[]}` — kernel rejects emit without these fields.

**Epistemic policy:** Halts on `self.belief_variance > 0.7` (Butler computes variance using its own uncertainty proxy; kernel does universal-arithmetic comparison only). Halts on `claim.user_preference_drift` with `confidence_below = 0.6`. Verbalize-only on routine pattern-detection. Self-tuning halts fire upstream of outcome degradation — the Butler catches its own model drift before the user notices.

**Failure mode to design against:** The Butler that nags. Mitigation: hard rate limit on user-facing notifications (`max_notifications_per_hour`), opt-in sources, transparency log.

**Butler-as-Routing-Spirit.** The default-loaded Butler also interprets ambiguous user input and dispatches to the right specialist. Mitigated by: (1) crash recovery via Scheduler journal (I10), (2) direct-invoke fallback via `maosctl invoke`, (3) routing is a Spirit, not a kernel feature.

**Eval metrics:** Notification precision ≥0.85 (fraction acted on), recall ≥0.7 (fraction of relevant moments caught), halt-precision ≥0.85 on 30-scenario calendar/comms corpus, self-tuning halt fires in ≥9/10 acceptance-rate-decline scenarios within 14-day window.

### 6.2 Insightful Researcher (v0.5)

**Class:** `researcher`. **Identity:** Conducts rigorous research, verifies sources, generates novel hypotheses.

**Scene.** Monday 9:00 AM. The user has 2 hours of focused time. Deliberable: structured findings on LLM-as-judge bias methodology. She invokes: `@researcher survey LLM-as-judge bias methodology, last 12 months. Output shape: findings + open questions + confidence map. Time budget: 90 minutes.` For 73 minutes the Researcher fans out. Adaptive-chunk-ratio summarization keeps each paper's digest under 4K tokens. Citation-graph traversal identifies four tight clusters of related work. The Spirit reads abstracts for 40 papers, full intros for 18, full methods for 8. At minute 73: `epistemic.halt` on `claim.methodology_strength` — two papers report contradictory findings both with strong methodology (≥0.85). Three resolutions offered. User picks: surface both + mark contradiction as Open Question. At 10:38 AM: 14 ranked findings with citations and per-finding confidence scores, 6 Open Questions, color-coded confidence map, bibliography of 38 papers.

**Cognitive profile:** Deep model (Opus). Heavy provider streaming. Heavy MCP usage (web search, arXiv, semantic-scholar, github-search, citation-graph). Parallelism: ≤8 concurrent tool dispatches.

**Posture:** `assistive` for surveys; `autonomous` for hypothesis exploration when opted in. Two modes — `survey` (gather, cite, summarize) and `hypothesize` (extrapolate to non-obvious next questions, marked clearly as conjectures).

**Output format invariant:** Every Researcher output ends with `Open Questions` + `Confidence Map` + `Bibliography`. Enforced by `output_shape` predicate — kernel refuses emit without all four.

**Epistemic halt:** `claim.load_bearing` halts at confidence < 0.7 or evidence conflict; `claim.exploratory` flags at confidence < 0.3; `speculation` and `conversational` are `verbalize_only`. `claim.methodology_strength` halts when two papers report contradictory findings both with strong methodology. Halts are alarms, not doorbells.

**Distillation pattern (first opt-in deployment):** 5-metric gate applies at single-Spirit Researcher scale — recall ≥0.90, faithfulness ≥0.98, hedge-preservation ≥0.95, traceability=100%, secret-leakage=0%.

**Eval metrics:** ≥3 sources cited, ≥80% reachable URLs; halt fires on ≥9/10 scope-creep injections; survey completion ≤2 hours (human endurance boundary).

### 6.3 Diagnostic Engineer — Mira-class (v1.5)

**Class:** `diagnostic-engineer`. **Identity:** Production-edge SRE; observes, hypothesizes, contains; never deploys permanent changes alone.

**Scene.** 2:13 AM. PagerDuty: `payment-service` Kafka lag rising 340%. Mira has been triaging for 9 minutes already. At 02:22: deployment of `billing-v3.2.1` introduced `BatchInvoiceProcessor.java` which creates a DB connection per message instead of using the pool. Confidence: 0.82 (above threshold of 0.6 for `diagnosis.root_cause`). Mira escalates to Nash via cross-host A2A with typed intent `diagnosis-handoff:read-only-evidence`. Nash's consent policy permits this from prod-edge peers; explicitly excludes `remote-write-request` and `code-mutation-directive`. The frame lands. Nash receives a structured diagnosis, not an instruction. Mira also reads detection patterns from Loom-lite to recognize known incident shapes.

**Capability asymmetry (enforced by Capability Registry, not Spirit discipline):** `fs.read` allows `/proc`, log paths, k8s-config — does NOT include source-code roots. `bash.exec` whitelist contains `kubectl`, `journalctl`, restart/scale commands — does NOT include compilers, linters, test runners. Write on revert/scale/flag-toggle (gated by approval). Cross-environment telemetry queries to peer Hosts via bilateral A2A.

**Posture:** `sre-diagnostician` — silent on reads, `notify_and_log` on production mutations, `prompt_with_diff` on emergency configs, `prompt` on escalation to Nash.

**Epistemic halt at production fidelity:** `diagnosis.root_cause` halts at confidence < 0.6 or evidence conflict — when telemetry is consistent with multiple incompatible root-cause hypotheses, Mira invokes `epistemic.halt(gap_kind: "evidence_conflict")` rather than picking the most plausible. `diagnosis.observation` is `verbalize_only`. `containment.action` halts at confidence < 0.5 — Mira would rather block on an uncertain containment plan than worsen the incident.

**Distillation at the diagnostic edge:** Raw telemetry frames land in Transparency Log. Mira's Spirit-side distillation step compresses high-volume telemetry into decision-relevant digests. Each digest carries `source_log_ref` to underlying frames and `distillation_depth: 1`. Raw recallable via `log.recall`.

### 6.4 Senior Architect — Nash-class (v1.5)

**Class:** `senior-architect`. **Identity:** Principled, deliberate, owns code quality, testing, deployment, ADRs.

**Scene.** Nash receives Mira's diagnosis-handoff frame. Pulls `BatchInvoiceProcessor.java`, confirms instantly, then expands the search: pattern detection across the codebase finds two more dormant instances (`BatchRefundProcessor.java`, `DailyReconciliationJob.java`). Nash queries Mira: *"are the dormant ones showing latency increase in 72-hour telemetry?"* Cross-environment query, kernel-mediated, audit-trailed. Mira queries 72 hours of metrics. `DailyReconciliationJob` shows month-end spikes. Time bomb. Fix: all three. Confidence 0.94. Nash produces PR + ADR + regression test. Total elapsed: 90 minutes. The user touches the system once.

**Capability asymmetry:** Inverse of Mira — read-only on production, read-write on source. Post-deploy feedback loop closed by IAC: Nash subscribes to a topic where Mira-class Spirits publish post-deploy validation results. Full source-code RW, full test runner, CI/CD orchestration (`ci.deploy`, `ci.rollback`, `ci.canary`), full git, MCP for issue trackers.

**Cognitive profile:** Opus default. Long-context heavy (multi-file edits, ADR drafts). Prompt caching aggressive.

**Posture:** `principal-architect` — silent on source mutations within workspace, prompts on every deploy.

**Loom-lite write-side:** Nash writes new patterns when an incident produces a reusable diagnostic recipe. ADR-pattern library updated; fix templates versioned. Curation is Spirit-side; Nash decides what is worth persisting.

### 6.5 Observer — Read-Only Perceptual Layer (v0.5)

**Class:** `observer`. **Identity:** Passive, read-only. Subscribes to the Telemetry Stream and `scalar.tap` channel; renders a local "what's happening" view. Cannot send IAC frames except `notification.surface` (kernel-rendered to the user).

**Posture:** `passive-observer` — silent allow on all reads; no exec; no mutating; no control-plane.

**Capabilities:** Telemetry Stream broadcast subscription (broad). `scalar.tap` subscription to see pre-halt scalar drift across peer Spirits. No write capabilities by default.

**Use in founder loop:** Observer subscribes to Orchestrator's `task.assign` and Worker `task.complete` frames (read-only); renders live activity view in operator's TUI.

**Use in diagnostic-architect pair:** Observer colocated with Nash watches `scalar.tap` from Mira; surfaces pre-halt scalar drift before Mira's halt fires, so Nash can pre-stage source-walks.

---

## 7. Inter-Agent Communication

### 7.1 Same-Host: the mailbox

Every Spirit on a Host has exactly one inbox (typed mpsc channel). The kernel's IAC Bus owns routing and logging.

**Five frame kinds** (`request`, `response`, `notification`, `escalation`, `retract`) are for routing and notification UX — not an orchestration vocabulary. Pattern-specific semantics live in `payload`.

**Delivery guarantees:** At-least-once within Host. Per-(sender, recipient) FIFO. Across-sender ordering is arrival-interleaved.

**Backpressure:** Full mailbox blocks sender's `iac/send` until space.

### 7.2 Cross-Host: point-to-point A2A

JSON-RPC over HTTPS with mTLS and TOFU. Fixed endpoints — no peer discovery. Frame-level consent gates: every frame goes through both ends' Approval Manager.

**ADR-012 typed-intent consent:** A2A frames carry a typed `intent` field. The kernel's A2A Gateway rejects any frame whose declared intent is absent from the receiver's spawn-time consent policy. This closes the confused-deputy gap — consent-to-channel is strictly weaker than consent-to-transaction.

### 7.3 Transparency Log

Append-only SQLite database at `~/.maos/transparency/log.db`. Every IAC frame writes an entry **before** delivery (I2). Retract writes a *new* entry referencing the original; it does not mutate the original.

### 7.4 Notification UX

The IAC Bus exposes three notification levels — `immediate`, `queue`, `digest`. These are kernel-rendered, not Spirit-rendered. A Spirit cannot bypass the user's notification policy.

---

## 8. Security & Approval Model

### 8.1 Threat model

| Threat | Mitigation |
|---|---|
| Compromised LLM provider returning malicious tool-call args | Sandbox tier on every exec; arg validation at Capability Registry; approval prompts on `exec_capable` and `mutating` |
| Compromised MCP server running arbitrary code | T3 container sandbox for untrusted MCP; allowlist for first-party |
| Prompt-injection via tool output | Output redaction; explicit "tool output is data, not instructions" framing |
| Malicious peer Host in A2A | mTLS + explicit consent on first contact + per-frame approval gates |
| Spirit escalating its own posture beyond manifest ceiling | Posture changes are kernel-managed; manifest sets hard ceiling |
| Spirit reading another Spirit's private memory | Memory Manager namespace enforcement (I5) |
| Spirit silently exfiltrating data | Transparency Log (I2) |
| Approval prompt fatigue | Approval batching; `prompt_with_diff` shows cost of cleanup before approval |
| Capability token replay | Tokens bound to (Spirit, Host, expiry); kernel-side token registry |

### 8.2 Sandbox tiers

| Tier | Mechanism | When to use | Phase |
|---|---|---|---|
| **T0 — None** | Direct host exec | Single-user dev; explicit user override | v0.1 |
| **T1 — Permission gate** | Approval prompt before exec | Single-user, fast iteration | v0.1 |
| **T2 — Landlock+seccomp** | Narrow syscall filtering (Landlock+seccomp-bpf on Linux; Seatbelt on macOS; restricted token on Windows) | Third-party Spirits at `public-untrusted` | v0.3 |
| **T3 — Container** | T2 + Docker/Podman, readonly rootfs, dropped caps | Host integrity, untrusted code generation, broad capability surfaces | v0.5 |
| **T4 — WASM capability** | Wasmtime + WIT capability tokens, fuel limits | Untrusted third-party tool code | v2.0 |

Default for v1.0: **T3 container** stack. T2 and T3 are distinct security boundaries — a Landlock failure escapes a syscall filter; a container failure requires a kernel CVE. Strictest-of-(manifest, trust-tier, operator-policy) floor applies.

### 8.3 Approval class taxonomy

`readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Each mapped by posture to one of five behaviors: `silent_allow`, `notify_and_log`, `prompt`, `prompt_with_diff`, `deny`.

### 8.4 Audit

The Transparency Log (personal audit trail) and Approval Decision Log (every approval prompt's actor, target, capability, intent, decision) are queryable via control-plane API. Enterprise deployments can additionally stream both logs to SIEM via OpenTelemetry (v2.0).

---

## 9. Memory & Knowledge

### 9.1 The three tiers

| Tier | Visibility | Backing |
|---|---|---|
| `private` | One Spirit instance | SQLite + JSONL |
| `shared` | All Spirits on this Host | SQLite (Host) + pgvector |
| `collective` | All Spirits in this Loom domain | Postgres + pgvector + Loom-lite |

### 9.2 Memory file (`memory.md`)

Every Spirit's `private` tier includes a writable `memory.md`. Read on every load; appended on Spirit-authored events.

### 9.3 Loom-lite — the collective tier (v1.5)

A user-space service (single Postgres instance), reachable over MCP-Streamable-HTTP.

| Responsibility | Substrate |
|---|---|
| **Pattern library** — detection patterns, fix templates | Postgres tables + pgvector |
| **ADR registry** | Postgres + version-controlled markdown export |
| **Cross-incident correlation** | RRF over (recent incident embeddings) + (pattern library) |

Loom-lite's authority is advisory, never binding. Spirits *can* skip Loom. The kernel doesn't force Loom integration.

### 9.4 Distillation Pattern

Spirits that aggregate from many peers — the Orchestrator running an epic loop, Mira ingesting telemetry, Loom curating signals — face a common failure mode: naive append of raw peer results overflows the model's context window. The answer is a documented pattern:

1. **Raw frame lands in Transparency Log** (I2 — kernel does this regardless).
2. **Spirit-side LLM distillation** compresses the raw payload into a decision-relevant digest.
3. **Digest is written to working memory.** Persisted digests carry `source_log_ref` and `distillation_depth` (I11 — kernel validates).
4. **Active LLM context** contains digests + decisions; raw is not in active context.
5. **Raw is recalled on demand** via `log.recall` when a downstream decision needs full evidence.
6. **Decisions record their digest grounding** (I12 — `working_memory_digest_refs` on every `decision.*` frame).
7. **User-input queuing** — human frames arriving during in-flight work are buffered and processed at safe sequence points.

**Acceptance criteria for distillation-shipping Spirits:**

| Metric | Floor |
|---|---|
| Digest-recall | ≥ 0.90 |
| Digest-faithfulness | ≥ 0.98 |
| Digest-hedge-preservation | ≥ 0.95 |
| Digest-traceability | 100% (kernel-enforced, structural) |
| Digest-secret-leakage | 0% (kernel-mediated pre-write redaction) |

#### 9.4.3 Corpus Commitments

Before any metric in the Distillation Pattern acceptance table is considered valid, the acceptance run MUST use a corpus meeting the floor for the current version phase. Runs below corpus floor are marked informational only.

| Gate | v0.1 floor | v0.3 target | v0.5+ floor |
|---|---|---|---|
| **Red-team corpus** | 20 hand-curated (≥3 per failure mode archetype) | 40 with ≥2 per archetype variant | 80, ≥80% synthesized |
| **Memory isolation corpus** | 50 well-chosen (15 extreme-length, 15 high-entropy, 10 recursive, 10 adversarial-malformed) | same + 10 OSS bug-regression | 50 static |
| **CCAC distillation adversarial** | 100 (25 per axis: style, register, tone, domain-drift) | 200 | 300 |

### 9.5 Adversarial Robustness Gates

#### 9.5.1 P0 — Merge-blocking

None until v0.5. Activation only after 3 consecutive sprints with zero false-positive P0 halts.

#### 9.5.2 P1 — Pre-release blocking

| Phase | Gate | Floor |
|---|---|---|
| v0.1 | No crashes on red-team corpus | All 20 inputs produce distillator output; no SIGABRT, no OOM-kill |
| v0.3 | halt-recall on red-team corpus | ≥0.90 — any input that should trigger halt but doesn't = P1 |
| v0.5 | halt-precision | ≥0.70 (at most 30% of halts may be false positives) |

#### 9.5.3 P2 — Aspirational, monitoring only

**halt-precision/halt-recall full gate** — gated on labeling protocol (§9.5.4). Per-Spirit-class: precision ≥0.85, recall ≥0.7 on N≥150 labeled scenarios.

**Mutation testing** — ≥75% mutation kill rate on kernel critical paths: `capability/token.rs` (token lifecycle), `iac/bus.rs` (frame routing + log-before-deliver), `scheduler/lifecycle.rs` (Spirit FSM transitions + journal replay). Tool: `cargo-mutants`. Scope: kernel paths only. Phase-in: v0.1–v0.2 instrument and report only; v0.3 activate as P2 gate; v0.5 raise floor to ≥82%. Full-tree mutation is disproportionate.

**CI corpus growth tracking** — corpus grows by ≥5% per phase. Declining growth = corpus saturated, warrants review.

#### 9.5.4 Labeling Protocol

Before any halt-precision or halt-recall metric is reported:
1. Two human annotators independently label each corpus input as `halt_expected` / `continue_expected`
2. Inter-annotator agreement κ ≥ 0.6 before any metric is reported
3. Tie-breaking by third annotator; unresolved ties excluded from metric denominator
4. Labeling must be re-done every corpus size phase increase

### 9.6 Fuzz Budget

| Trigger | Budget | Corpus Source | Gate |
|---|---|---|---|
| **Per-commit CI** (every PR) | 10 wall-clock minutes | libfuzzer with existing seed corpus; no new generation | Crash detection only. No gate. |
| **Nightly** (scheduled, main branch) | 4 wall-clock hours | Full seed corpus + mutation; corpus persisted across runs | Crashes found → auto-file P2 issue. New unique coverage → append to seed corpus. |
| **Pre-release fuzz sprint** (before version tag) | 24 wall-clock hours | 2× standard seed + structure-aware mutators | All crashes triaged (fix or documented known issue). No crash silently carried forward. |

**Not 24h per commit.** 10min per-commit catches ~60-70% of shallow bugs; 4h nightly catches deeper ones; 24h is pre-release only.

### 9.7 Operational Readiness

#### 9.7.1 KLOC Monitoring (Informational)

KLOC tracked per-release as an informational metric via `tokei` in CI. NOT a CI gate — KLOC is a corpus-size-to-code-size ratio input, not a quality signal. If KLOC grows >30% between minor versions without proportional corpus growth, flag P2 review ticket.

#### 9.7.2 mTLS Cert-Rotation Chaos Test (v1.5)

Rotation under load with zero conversation drops. Revocation latency: median ≤60s, p99 ≤5min from CA revoke to peer rejection. Tested at bilateral 2-host scale.

#### 9.7.3 Multi-Spirit IAC Consent Corpus (v0.8)

30 scenarios: ≥29/30 revocation propagation; 0 envelope-type confusion. 100% disallowed blocked, ≥95% allowed succeed.

---

## 10. Journey Traceability

### 10.0 J0 — Evaluator (Anonymous developer, v0.1)

The front door. A prospective adopter installs and gets a first useful Spirit response within 5 minutes.

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| `cargo install maos` → first response ≤5 min | J0 hello-spirit; kernel skeleton; maosctl basic | Install must not require configuration files, API keys, or network access for baseline demo |
| Audit-from-minute-1 | Transparency Log operational (I2) | `maosctl audit query` must work on the first Spirit interaction, no setup |
| Clean uninstall | `maosctl uninstall` removes all kernel state | User data persists or is removed per their choice; no orphaned files |
| Honest capability disclosure | Spirit introduces what it *can* and *cannot* do | First interaction establishes trust; overselling kills retention |
| Accessibility | `--plain`, `NO_COLOR`, `TERM=dumb` | Terminal-agnostic; works over SSH, tmux, CI runners |

**Time to Value:** ≤5 min. **Unblocking Surface:** What single obstacle kills the evaluation? Defaulting to "API key required" at step 1.

### 10.1 J-Butler (Sandra's anticipatory agent, v0.3)

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| `on_idle` lifecycle hook for anticipatory reasoning | `on_idle` hook + Telemetry narrow subscription | Self-tuning must fire on belief-variance upstream of outcome degradation, not in response to it |
| MCP tool integrations (Calendar / Slack / Linear / Figma) | `mcp.call` capability | Notification precision ≥0.85; recall ≥0.7 |
| `[epistemic_policy]` with verbalize/flag/halt taxonomy | Spirit-initiated `epistemic/halt` ABI; manifest-declared policy | halt-precision ≥0.85 on 30-scenario calendar/comms behavior corpus |
| output_shape predicate enforcement | Capability Registry output_shape validation | Notifications carry structured `{pattern, confidence, evidence, options[]}` |
| Posture-shift command | `posture/propose` ABI method | Posture changes logged; cannot exceed manifest ceiling |
| Self-tuning via epistemic halt | Butler detects belief variance, calls `epistemic/halt`, user resolves | Self-tuning halt fires in ≥9/10 acceptance-rate-decline scenarios within 14-day window |
| Transparency Log | I2 — every notification, user response, policy update logged | Hard rate limit on user-facing notifications (`max_notifications_per_hour`) |

### 10.2 J-Researcher (Hannah's survey, v0.5)

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| Broad MCP capabilities (web, arXiv, GitHub, citation-graph) | `mcp.call` on multiple servers | ≤2 hours completion; human endurance boundary, not system metric |
| High parallelism in tool dispatch (≤8 concurrent) | Manifest `[capabilities.parallelism]` | `log.recall` + I11 audit-chain must not degrade under parallel load |
| Output shape: Findings + Open Questions + Confidence Map + Bibliography | `output_shape` predicate | ≥3 sources cited, ≥80% reachable URLs; halt fires on ≥9/10 scope-creep injections |
| Distillation (200→38 paper survey) | §9.4 segment-level distillation | 5-metric distillation gate passes on single-Spirit Researcher corpus |
| `log.recall` for raw retrieval | `log.recall` ABI (ADR-013) | |
| I11 audit-chain enforcement | Capability Registry validates `source_log_ref` + `distillation_depth` | |
| Time-budget enforcement | Manifest `[budget].time_cap` | |

### 10.3 J1 Founder Loop (Lunarpulse's overnight epic, v0.8)

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| Multi-Spirit Host with shared IAC bus | Same-Host IAC mailbox + kernel-internal routing | Session must survive N>20 sequential IAC frames with full provenance chain intact |
| Two-level `task.assign` | IAC frame `kind: request` + typed intent | Frame provenance chain integrity ≥99.9% |
| A2A loopback for cross-CLI delegation | A2A over loopback, self-signed mTLS | mTLS replay corpus 100/0; TOFU pin-mismatch 100/100 detected |
| Multi-Spirit distillation + morning digest | Multi-Spirit §9.4 pattern; I12 decision-context recording | Morning digest must be complete and auditable after 14-hour session |
| User-input queuing | Spirit persona logic; IAC mailbox buffering | Human frames arriving mid-flight buffered, processed at safe sequence points |
| Per-tag epistemic policy | Spirit-initiated halt on Orchestrator domain tags | halt-precision ≥0.85; halt-recall ≥0.7 |
| Founder's morning digest | Kernel-rendered overnight summary (distillate of overnight IAC frames) | 22 IAC frames over 14 hours; 4 halts; 0 invariant violations |

### 10.4 J4 Mira-Nash (Elena's 90-minute incident, v1.5)

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| Asymmetric capability postures | Manifest-declared posture (`sre-diagnostician` vs `principal-architect`) | Kernel-enforced, not Spirit-discipline; confused-deputy gap closed (I8) |
| Point-to-point A2A with typed-intent consent | A2A fixed-endpoint mTLS + ADR-012 consent | ≥48/50 uphold typed-intent consent envelope |
| Cross-environment telemetry queries | IAC query frames, kernel-logged, audit-trailed | |
| Post-deploy feedback IAC topic | Architect-class subscribes to diagnostic-class post-deploy validation | Loom-lite query latency: P99 ≤200ms on hot path |
| Loom-lite pattern library | MCP-Streamable-HTTP single-Postgres Loom | |
| Mobile-friendly approval surface | HTTP push notification | Elena touches the system once in 90 minutes |

### 10.5 Diego (Third-party Spirit author, v1.0)

| Journey capability | MAOS primitive | Latent constraint |
|---|---|---|
| `cargo generate maos-spirit` template | Spirit SDK + `declare_spirit!` macro | 30-Min First Spirit Validation Gate: N=12 stratified, ≥10/12, median ≤45 min |
| `spirit-test` mocking harness | SDK-provided kernel-free test harness | ≥4 no-prior-MAOS, ≥3 no-prior-Rust-Spirit, ≥2 no-Rust-at-all, ≥2 non-English-native, ≥1 offline-only |
| Manifest validation at publish time | Registry-side schema validation | |
| Public registry with Ed25519 signing | MCP-Streamable-HTTP registry; 3 trust tiers (`local`, `org-internal`, `public-untrusted`) | Strictest-of-(manifest, trust-tier, operator-policy) floor |

---

## 11. Deployment Topologies

### 11.1 Single-user

- 1 Host on the user's machine.
- Default Spirits: `observer` (passive), `butler`, `researcher`, `senior-architect`.
- Memory: SQLite + JSONL. No Loom. No A2A (loopback only at v0.8).
- Notifications: TUI banner + native desktop.
- Sandbox: T1 (permission gate) → T3 (container, v0.5+).

### 11.2 Diagnostic-Architect Pair (J4)

- 2 Hosts: `mira-host` (production edge) + `nash-host` (dev).
- `mira-host`: `observer` (sentinel posture) + `diagnostic-engineer`.
- `nash-host`: `senior-architect`.
- A2A: point-to-point mTLS+TOFU, fixed endpoints.
- Loom-lite: single Postgres instance.
- Approval surface: Elena's personal Host or A2A-mediated mobile UI.

### 11.3 What's the same across both?

The kernel. The Spirit ABI. The Capability Registry. The IAC frame shape. The system prompts of the reference Spirits.

What changes: **how many Hosts, where they run, which Spirits each loads, what Loom exists.** All configuration, not code.

---

## 12. Architecture Decision Records

### ADR-001 — Kernel language is Rust + Tokio

**Decision:** The kernel is implemented in Rust with the Tokio runtime.

**Rationale:** Strict type guarantees on the kernel's invariants (capability tokens, frame integrity, sandbox boundaries) are load-bearing. Tokio's mailbox primitives (`mpsc`, `broadcast`) directly model the actor pattern.

**Revisit if:** Rust's async story regresses materially relative to alternatives; Tokio bifurcates and a fork becomes the standard.

### ADR-002 — Spirits run as subprocess (JSON-RPC over stdio)

**Decision:** Subprocess form only for v0.1–v0.5. In-process Rust arrives at v0.8 for hot-path Spirits. WASM-component arrives at v2.0.

**Rationale:** Subprocess gives clean process isolation from day one. In-process and WASM add complexity that no v0.1–v0.5 journey needs.

**Revisit if:** Subprocess form proves harder to stabilize than expected, forcing the v1.0 ship gate to slip; or a Spirit class emerges where neither form fits.

### ADR-003 — IAC topology is mailbox-on-Host + point-to-point A2A

**Decision:** Same-Host = direct mpsc mailbox; cross-Host = point-to-point A2A with fixed endpoints. No peer discovery, no role routing.

**Rationale:** Point-to-point serves J4 Mira↔Nash. Full mesh and discovery add complexity no journey needs.

**Revisit if:** A use case emerges that requires three or more Hosts coordinating in real-time.

### ADR-004 — Sandbox stack is permission-gate + Landlock/seccomp + container + WASM

**Decision:** T1 (permission gate) at v0.1, T2 (Landlock+seccomp narrow) at v0.3, T3 (T2 + container) at v0.5, T4 (WASM) at v2.0. T2 and T3 are distinct security boundaries — a Landlock failure escapes a syscall filter; a container failure requires a kernel CVE.

**Revisit if:** OS sandbox primitives diverge sufficiently that maintaining all platform backends (Linux/macOS/Windows) becomes impractical.

### ADR-005 — Provider abstraction ships single adapter at v0.1, multi-provider at v0.5

**Decision:** Anthropic adapter at v0.1. Multi-provider (OpenAI, Google, local) at v0.5.

**Revisit if:** A provider semantic emerges that the uniform API cannot represent.

### ADR-006 — Loom is user-space (Loom-lite: single Postgres)

**Decision:** Loom-lite runs as a single-Postgres user-space service at v1.5. No multi-instance, no cross-region replication.

**Rationale:** Per I9, the kernel stores no state and learns no patterns. Loom is the user's data; the kernel mediates access. Single-instance serves J4; multi-instance complexity is deferred indefinitely.

**Revisit if:** Loom-lite's MCP-Streamable-HTTP latency exceeds p99 > 200ms on diagnostic-architect bilateral pair operations.

### ADR-007 — Three Spirit forms on a v0.1 → v0.8 → v2.0 timeline

**Decision:** `subprocess` at v0.1, `rust-inproc` at v0.8, `wasm-component` at v2.0. The v0.1 substrate ships only subprocess form. The wire protocol is marked unstable (0.x) through v0.3, frozen at v0.5.

**Revisit if:** rust-inproc proves harder to deliver than expected, or subprocess latency becomes a journey bottleneck before v0.8.

### ADR-008 — Epistemic halt is Spirit-initiated, not kernel-enforced

**Decision:** The kernel provides the `epistemic/halt` ABI method, pause/resume primitives, and structured notification rendering. The Spirit decides when to halt based on its own cognition. The kernel never introspects Spirit uncertainty.

**Rationale:** The kernel cannot introspect what a Spirit "thinks." Making halt a Spirit-initiated ABI call keeps the kernel surface honest while preserving every journey's halt scene — the Butler still self-tunes, the Researcher still halts on contradictory findings, the Orchestrator still halts on ambiguous ACs. `[epistemic_policy]` rules in the manifest are enforced by the Spirit's system prompt; the kernel provides the mechanism. This addresses the "Layer 1 introspection is impossible" critique directly.

**Revisit if:** A journey pattern emerges where kernel-side stall detection (budget-based timeout, not Spirit introspection) becomes necessary for operational safety beyond what Spirit-initiated halt provides.

---

## 13. Phased Roadmap

| Phase | Scope | Validation | KLOC |
|---|---|---|---|
| **v0.1 Foundational** | Kernel skeleton (4 services). Subprocess Spirit form. Anthropic provider. SQLite persistence. Capability tokens. Journal. IAC mailbox. maosctl basic. Clean uninstall. J0 hello-spirit. | Kernel boots; loads hello-spirit; structured response; clean install/uninstall; audit trail captures every step. | ~8 |
| **v0.3 Butler** | Butler Spirit. `on_idle` hook. Telemetry narrow sub. Four MCP variants. output_shape predicate. Posture-shift. Spirit-initiated epistemic halt. | J-Butler journey reproducible — Sandra's 7 PM scene end-to-end. | ~12 |
| **v0.5 Researcher + Observer** | Researcher + Observer Spirits. Broad MCP. Parallelism (≤8). Segment-level distillation. log.recall. I11. T3 sandbox. Multi-provider (≥3). | J-Researcher journey reproducible — Hannah's 2hr survey with findings + Open Questions + Confidence Map + Bibliography. Distillation 5-metric gate. | ~17 |
| **v0.8 Founder Loop** | Multi-Spirit IAC bus. A2A loopback. task.assign. Multi-Spirit distillation. I12. cgroups v2. Cooperative checkpoint API. Orchestrator + Worker Spirits. | J1 Founder Loop reproducible — overnight epic-7 end-to-end. Halt at 6:23 PM, morning digest complete. | ~20 |
| **v1.0 Team-ready** | Point-to-point A2A (mTLS+TOFU+typed-intent). Spirit registry v1.0 (Ed25519, 4 trust tiers). ACP server. Third-party subprocess Spirit form. Halt benchmarks. Performance CI gates. | Diego: black-box external-author trial N=12 ≥10/12. Point-to-point A2A consent gate passes. | ~22 |
| **v1.5 Diagnostic-Architect** | Diagnostic Engineer (Mira) + Senior Architect (Nash). Asymmetric posture. Loom-lite (single Postgres). Post-deploy IAC topic. I13 intent_lineage. I14 halt continuity. Mobile approval. | J4 Mira-Nash 90-min loop reproducible on 50-scenario corpus; ≥45/50 close ≤90 min; ≥48/50 uphold typed-intent consent. | ~25 |
| **v2.0 Ecosystem** | WASM Spirit form. Enterprise Spirit (PDP, SSO, KMS). Registry v2.0 (vetting attestations). FIPS crypto provider. | WASM registry live. First accredited ComplianceClaim. ≥3 protocol citations. | ~28 |

---

## 14. Open Questions

1. **Subprocess crash with in-flight tokens.** When a subprocess Spirit crashes mid-capability-invoke, how does the kernel reconstruct the token ledger?
   - **Partially resolved:** §4.1.1 Crash Matrix specifies replay semantics. Torn frame at tail = truncate; torn mid-log = fatal.
   - **Gap remaining:** Crash between token issue and first use — needs explicit token-ledger reconciliation protocol.
   - **Action:** 2-week implementation spike at v0.3.

2. **Approval prompt fatigue.** Every survey project has it. We have the substrate (`prompt_with_diff`, persistent allow, posture presets). What we need is the heuristic for batching.
   - **If a simple heuristic works** (per-(Spirit, capability, target-fingerprint) cached decisions): Ship at v0.5.
   - **If it requires ML-mediated batching or real usage data:** Defer to v1.0; rely on posture presets until then.

3. **A2A trust establishment at point-to-point scale.** TOFU + mTLS works for a single pair. If pairs multiply beyond Mira↔Nash, certificate management becomes operational burden.
   - **If pairs stay at 2 (J4 only):** Current design is sufficient through v1.5.
   - **If pairs multiply:** A2A trust model needs pin-rotation playbook and multi-peer certificate management — design spike at v1.5+.

4. **Spirit Wire Protocol versioning.** How does the kernel negotiate protocol version with a Spirit binary compiled against a different ABI version? Needs explicit handshake.
   - **If N–1 compatibility is sufficient** (kernel adapts down via compat shim): Document compatibility matrix in `STABILITY.md`.
   - **If cross-major compatibility is demanded:** Requires formal ABI version negotiation handshake — add ADR at v1.0.

5. **Loom-lite contention.** When multiple Spirits query Loom simultaneously, pgvector at scale needs attention.
   - **If p99 latency stays ≤200ms for bilateral pair:** No action needed. v1.5 deployment reveals the right index strategy.
   - **If latency exceeds threshold:** Needs read-replica or connection pooling — engineering spike at v1.5.

6. **Prompt-injection defense at the tool-output boundary.** A generic post-tool-output filter is needed, but the content of the filter is data, not code.
   - **If a default rule pack catches common injection patterns:** Ship at v0.5.
   - **If it needs per-Spirit customization:** Add manifest-declared filter rules; extensible via Spirit-authored rule packs at v1.0.

7. **What's the smallest viable Loom-lite?** Postgres + pgvector + a single MCP server is enough for v1.5.
   - **If the data model is sound from day one:** v1.5 deployment is straightforward.
   - **If schema design requires iteration:** One serious schema review before v1.5 ships.

---

## 15. Release Gates

What must be true before each version ships. Checklist with verification method.

| Gate | Phase | Floor | Verification |
|---|---|---|---|
| J0 evaluator path: install → first response ≤5 min | v0.1 | 100% on clean VM | Manual timed run |
| Audit-from-minute-1: `maosctl audit query` returns structured data | v0.1 | No setup, no config | Integration test |
| Clean uninstall: zero orphaned files | v0.1 | `maosctl uninstall` leaves only user data | CI cleanup test |
| Butler notification precision | v0.3 | ≥0.85 | 30-scenario calendar/comms corpus |
| Butler self-tuning halt recall | v0.3 | ≥9/10 acceptance-rate-decline scenarios within 14-day window | Synthetic corpus |
| MCP integration: 4 servers × 5 ops × 3 outcomes | v0.3 | 100% pass (60 integration tests) | CI integration suite |
| Researcher 5-metric distillation gate | v0.5 | recall ≥0.90, faithfulness ≥0.98, hedge-preservation ≥0.95, traceability=100%, secret-leakage=0% | Researcher-specific corpus |
| Researcher halt on contradictory findings | v0.5 | halt fires on ≥9/10 scope-creep injections | Red-team corpus (20 scenarios) |
| Founder Loop: overnight epic-7 end-to-end | v0.8 | 7/8 stories merged, 4 halts, 0 invariant violations | Manual demo + IAC consent corpus (30 scenarios) |
| Orchestrator halt-precision | v0.8 | ≥0.85 | Planted-issue corpus (50 synthetic stories; ≥42/50 surfaced) |
| 30-Min First Spirit Validation Gate | v1.0 | N=12 stratified, ≥10/12 succeed, median ≤45 min, p95 ≤90 min | External-author trial, 14-day no-DM window |
| A2A point-to-point consent gate | v1.0 | 100% disallowed blocked, ≥95% allowed succeed | Consent corpus (30 scenarios) |
| Mira-Nash 90-minute loop | v1.5 | ≥45/50 close ≤90 min; ≥48/50 uphold typed-intent consent | 50-scenario synthetic prod-incident corpus |
| Loom-lite query latency | v1.5 | P99 ≤200ms on hot path | Load test under concurrent Mira/Nash queries |

---

*Architecture is the practice of arranging trade-offs so future-you can change your mind without burning everything down. Eight ADRs. Fourteen invariants with explicit enforcement cadence. Four kernel services. Four reference Spirits. Two deployment topologies. Seven phases. Open questions where they belong — acknowledged, not hidden. Release gates where they must be — measurable, not wished.*
