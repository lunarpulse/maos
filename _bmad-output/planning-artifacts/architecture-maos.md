---
title: 'MAOS — Modular Agentic Operating System: Architecture v1.0'
author: 'Winston (System Architect) for Lunarpulse'
date: '2026-05-04'
status: 'Vision-locked architecture, pre-implementation'
foundation: '_bmad-output/planning-artifacts/research/technical-ai-agent-frameworks-and-coding-tools-comparative-architectural-analysis-research-2026-05-04.md'
journeys: '_bmad-output/planning-artifacts/industrial_agents.md'
audience: 'Lunarpulse + early implementors'
non_goals:
  - 'Performance benchmarking'
  - 'Marketing positioning'
  - 'Concrete code in target language (kept to interfaces and pseudocode)'
---

# MAOS — Modular Agentic Operating System

> **Working name.** "MAOS" tracks the project directory `~/dev_ws/maos` and reads naturally as "Modular Agentic Operating System". Final naming is not blocking.

## 0. Executive Summary

MAOS is **a substrate, not a product**. It is the invariant scaffolding on which specialized agents — *Spirits* — are loaded, swapped, supervised, and composed the way processes are on a conventional OS. The kernel exposes one stable contract — the **Spirit ABI** — and every "agent class" you care about (Butler, Researcher, Diagnostician, Architect, Enterprise, Observer, …) is just a different manifest plus a different posture against that contract.

The design rests on five foundational commitments:

1. **Kernel/Spirit separation is enforced, not advisory.** Spirits never share a process address space without going through the IAC bus, never touch the filesystem outside the Memory Manager's namespaces, and never spawn tools outside the Capability Registry. This is what makes "hot-swap a Spirit mid-session without dropping in-flight tool calls" achievable.
2. **Boring tech in the kernel; ambition in the contract.** Every kernel layer reuses something already battle-tested in the survey: MCP for tools, ACP for editor bridges, A2A for peer mesh, OS-native sandboxing primitives, SQLite for local state, Postgres for shared state, Wasmtime+WIT for tool capabilities. The novelty is the Spirit ABI itself — that's what nobody has standardized.
3. **The same primitives compose into single-user, team-mesh, and enterprise-cortex deployments.** Journeys 10, 11, and 12 are not three different products — they are three deployment topologies over one architecture, distinguished only by **how many** Spirits run, **where** they run, and **how the IAC bus is configured**.
4. **Human transparency is a kernel invariant, not an application choice.** "No invisible actions, no puppeting, no asymmetric knowledge" (Journey 10) is enforced at the kernel level by the Transparency Log and the Approval Manager, before any Spirit gets to author behavior. A Spirit cannot bypass this.
5. **The kernel itself learns nothing.** Patterns, ADRs, fix templates, regression tests — the *Loom-curated collective knowledge* (Journey 12) — live in user-space, not the kernel. The kernel only enables propagation; what gets propagated is the user's data, governed by the user's policy.

The remainder of this document is the technical realization of these commitments.

---

## 1. Reading Map

This document is long. Read it in this order:

| Section | What's in it | Read if you're |
|---|---|---|
| §2 — Architecture at a Glance | Two diagrams, one paragraph each | Skimming |
| §3 — Vocabulary & Invariants | Pin down terms and what cannot change | Implementing |
| §4 — Kernel Design | Seven layers, each with interface sketch | Implementing the kernel |
| §5 — Spirit ABI | The hot-swappable contract; manifest schema; lifecycle | Implementing the kernel or a Spirit |
| §6 — Reference Spirits | Six concrete Spirits as worked examples | Designing a new agent class |
| §7 — Inter-Agent Communication | Intra-host mailbox + A2A peer mesh + transparency | Designing multi-agent flows |
| §8 — Security & Approval Model | Sandbox tiers, capability tokens, approval classes | Threat-modeling |
| §9 — Memory & Knowledge | Per-Spirit, shared, collective; Loom curation | Building memory features |
| §10 — Journey Traceability | How Journeys 10–12 map to §4–§9 primitives | Validating the design |
| §11 — Deployment Topologies | Single-user → Team → Cortex | Sizing/operating |
| §12 — ADRs | Six contested decisions with rationale | Disagreeing with me |
| §13 — Phased Roadmap | What ships in v0.1, v0.5, v1.0, v2.0 | Planning |
| §14 — Open Questions | Things I'm not sure about | Pushing back |

---

## 2. Architecture at a Glance

### 2.1 The two-paragraph version

A **MAOS Host** is a single OS process running the MAOS kernel. The kernel exposes seven services (Spirit Scheduler, Memory Manager, Security Manager, I/O Subsystem, IAC Bus, Capability Registry, Telemetry Stream) and one contract (the Spirit ABI). At any moment, the Host runs **N Spirits** scheduled co-operatively against shared resource budgets. A Spirit is a **manifest + cognitive profile + memory scope + posture**, loaded from disk or pulled from a registry. The kernel never introspects what a Spirit is "thinking"; it only dispatches messages, supervises lifecycle, and enforces capability and approval policy.

Spirits running on different Hosts speak A2A. Spirits running on the same Host speak through the IAC mailbox. Tools — including LLM providers, MCP servers, shell, file editors, CI/CD adapters — live behind the Capability Registry, which mediates every tool invocation through the Security Manager and the Approval Manager. The Memory Manager exposes three namespaces: **per-Spirit private**, **shared (this Host)**, and **collective (cross-Host, via Loom)**. The Telemetry Stream is the perceptual organ — every Spirit subscribes to it for situational awareness; the Observer Spirit class is just the canonical consumer.

### 2.2 The diagram

```
┌──────────────────────────────────────────────────────────────────────────┐
│                            HUMAN GOVERNANCE                              │
│           (decision gates, approval prompts, transparency log)           │
└──────────────────────────────────────┬───────────────────────────────────┘
                                       │
                          ┌────────────┴────────────┐
                          │                         │
                          ▼                         ▼
┌────────────────────────────────────┐    ┌────────────────────────┐
│              SPIRITS               │    │   USER-SPACE SERVICES  │
│  (hot-swappable, manifest-driven)  │    │                        │
│  ┌──────────┐  ┌──────────────┐   │    │  • Tool servers (MCP)  │
│  │  Butler  │  │  Researcher  │   │◀──▶│  • Editor bridges (ACP)│
│  ├──────────┤  ├──────────────┤   │    │  • A2A peer mesh       │
│  │Diagnostic│  │  Architect   │   │    │  • Loom (collective KB)│
│  ├──────────┤  ├──────────────┤   │    │  • Tool sandboxes      │
│  │Enterprise│  │   Observer   │   │    │  • Provider gateways   │
│  └──────────┘  └──────────────┘   │    └────────────┬───────────┘
│        ▲                           │                 │
│        │  Spirit ABI (§5)         │                 │
│        ▼                           │                 │
└────────────────────────────────────┘                 │
                          ▲                            │
        ╔═════════════════╧═════════════════╗          │
        ║              MAOS KERNEL          ║          │
        ║      (invariant; Rust+Tokio)      ║          │
        ║                                    ║          │
        ║  ┌─────────────┐  ┌─────────────┐ ║          │
        ║  │   Spirit    │  │   Memory    │ ║          │
        ║  │ Scheduler   │  │   Manager   │ ║          │
        ║  └─────────────┘  └─────────────┘ ║          │
        ║  ┌─────────────┐  ┌─────────────┐ ║          │
        ║  │  Security   │  │     I/O     │◀╬══════════╝
        ║  │   Manager   │  │  Subsystem  │ ║   stdio/JSON-RPC, MCP,
        ║  └─────────────┘  └─────────────┘ ║   ACP, A2A, HTTP/SSE/WS
        ║  ┌─────────────┐  ┌─────────────┐ ║
        ║  │     IAC     │  │ Capability  │ ║
        ║  │     Bus     │  │  Registry   │ ║
        ║  └─────────────┘  └─────────────┘ ║
        ║          ┌─────────────┐          ║
        ║          │  Telemetry  │          ║
        ║          │   Stream    │          ║
        ║          └─────────────┘          ║
        ╚════════════════╤══════════════════╝
                         ▼
        ┌────────────────────────────────────┐
        │       HOST OS PRIMITIVES           │
        │  Landlock+seccomp / Seatbelt /     │
        │  bwrap / Wasmtime / SQLite /       │
        │  Postgres / cgroups / namespaces   │
        └────────────────────────────────────┘
```

The same **single Host** scales out by configuration, not by changing the architecture:

- **Single-user** (Journey 10 baseline): 1 Host, N Spirits, no A2A, no Loom.
- **Team mesh** (Journey 10): 8 Hosts (one per teammate), full A2A peer mesh, optional shared Loom-lite.
- **Diagnostic-architect pair** (Journey 11): 2 Hosts (edge + dev), asymmetric postures, Loom optional.
- **Enterprise Cortex** (Journey 12): 28+ Hosts in three caste-shaped roles (Sentinel/Artisan/Loom), full A2A mesh, mandatory shared collective KB.

---

## 3. Vocabulary & Invariants

### 3.1 Vocabulary (read these definitions, then nothing in this doc is ambiguous)

| Term | Definition |
|---|---|
| **Host** | One OS process running the MAOS kernel. The unit of deployment. |
| **Kernel** | The seven invariant services (§4) that every Host exposes identically. |
| **Spirit** | A loaded, running agent. State = (Manifest + Cognitive State + Memory Pages + Posture + Capability Token Set). |
| **Spirit Manifest** | The declarative file (TOML/JSON) describing a Spirit class: identity, role, model, memory scope, capability requests, posture, hooks. The "spirit" in "the spirit is hot-swappable." |
| **Spirit Class** | A manifest type (e.g., `butler`, `mira-diagnostic`, `nash-architect`). Many *instances* of one class can be loaded simultaneously (e.g., 14 Sentinel instances in Cortex). |
| **Posture** | A Spirit's current autonomy stance: which approval class triggers a prompt, which sandbox profile is bound, which IAC peers are reachable. **Posture is mutable; Spirit class is not** (mid-life). |
| **Capability** | A typed tool surface — `bash.exec`, `fs.read`, `mcp.call(server, tool)`, `provider.complete(model)`, `a2a.send(peer)`, `git.commit`. Mediated by the Capability Registry. |
| **Capability Token** | An unforgeable ticket the kernel hands a Spirit when it requests a capability. Carries scope, expiry, and an audit ID. |
| **Approval Class** | One of six (§8.3): `readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Borrowed from openclaw's classifier. |
| **IAC** | Inter-Agent Communication. Two flavors: same-Host mailbox, cross-Host A2A peer mesh. |
| **Mailbox** | A bounded mpsc channel owned by a Spirit, addressable by Spirit ID, supervised by the kernel. |
| **A2A peer** | A remote Host advertising itself via `a2a.json`; reached over A2A protocol with consent gates. |
| **Loom** | A user-space service that curates patterns, ADRs, fix templates, regression tests across Spirits — the "collective" memory tier. From Journey 12. |
| **Transparency Log** | An append-only kernel-managed log of every IAC interaction, every approval decision, every capability use, and every retract. Personal to the user; not visible to peers. |

### 3.2 Invariants (these cannot change without a major-version bump)

These are the load-bearing rules. Every implementation choice in §4–§9 exists to enforce one or more of them.

| # | Invariant | Why it matters | Enforcement point |
|---|---|---|---|
| I1 | **Spirits cannot bypass the Capability Registry.** Every tool, network call, file op, sub-Spirit spawn goes through it. | Without this, sandboxing, approvals, and audit are decorative. | Kernel — Capability Registry is the *only* surface returned to Spirits at load time. |
| I2 | **Every IAC interaction is logged before delivery.** No ACK/NACK has ever been sent without an entry in the Transparency Log. | Journey 10's "no invisible actions" rule. Without this, peer trust collapses. | Kernel — IAC Bus writes log before it writes mailbox. |
| I3 | **Auto-responses are always marked `[auto-sent]` on both sides.** | Journey 10's "no puppeting" rule. | Kernel — IAC Bus stamps every message with `origin: human-authored \| spirit-auto \| spirit-drafted-human-approved`. |
| I4 | **Approvals are persisted with intent, not just decision.** | Reproducibility, retract, audit. | Kernel — Approval Manager stores `(actor, target, capability, intent, decision, ts)`. |
| I5 | **A Spirit's memory scope is declared in its manifest and enforced by the Memory Manager.** | Hot-swap requires knowing what to migrate vs. what to drop. | Kernel — Memory Manager rejects reads/writes outside declared scope. |
| I6 | **Hot-swap preserves Capability Tokens for in-flight tool calls; the new Spirit inherits the token, not the call.** | Journey 11 — Mira escalates to Nash without losing the diagnostic context. | Kernel — Token Lifecycle Manager (§4.3.4). |
| I7 | **Telemetry is broadcast; subscription is per-Spirit.** No Spirit polls the OS directly. | Observability is a perceptual organ, not a privilege. | Kernel — Telemetry Stream. |
| I8 | **Cross-Host A2A interactions require explicit consent at both ends.** | Journey 10's "peer-consent" rule; Journey 11's asymmetric trust. | Kernel — A2A Gateway enforces sender-side policy AND receiver-side acceptance before delivery. |
| I9 | **The kernel itself stores no secrets and learns no patterns.** | Auditability. The kernel is replaceable; the user's data is not. | Kernel — secrets pass through to OS keyring; KB lives in user-space (Loom). |
| I10 | **Every Spirit lifecycle transition is journaled; crash recovery rehydrates from the journal.** | Reliability. Journey 12's "92-minute Cortex cycle" requires no incidents from Spirit crashes. | Kernel — Journal at the Spirit Scheduler. |

If you find yourself bending one of these, you are not implementing MAOS — you are implementing something else. Stop and write a new ADR.

---

## 4. Kernel Design

The kernel is **one Rust+Tokio process** per Host. The choice of Rust and Tokio is not novel — it is the consensus of the survey: codex, ironclaw, and rustain all picked Rust+Tokio, and codex's in-process mailbox actor pattern is directly applicable. The kernel exposes seven services, internally arranged as Tokio tasks talking over typed channels, externally exposed to Spirits via the Spirit ABI (§5).

A Spirit, by contrast, may be implemented in **any language** that can speak the Spirit Wire Protocol (a thin JSON-RPC-over-stdio dialect modeled on ACP and codex's app-server). Reference Spirits will be Rust crates running in-process for performance; LLM-driven Spirits run as kernel-supervised subprocesses with stdio JSON-RPC pipes. **Both paths are first-class**; choice is per-Spirit-class.

### 4.1 Spirit Scheduler

**Responsibility:** Lifecycle management for all Spirits on this Host.

**State:**
- `Map<SpiritId, SpiritControlBlock>` — the OS-style PCB analog
- `Journal` — append-only on-disk log of all lifecycle transitions (for I10)
- `ResourceBudgets` — per-Spirit caps on tokens/min, $/hour, parallel tool calls

**Operations exposed to user-space (via control-plane API):**
- `load(manifest_path) → SpiritId`
- `start(SpiritId)`
- `pause(SpiritId)`
- `swap(SpiritId, new_manifest_path)` — hot-swap; preserves memory scope and in-flight Capability Tokens (I6)
- `migrate(SpiritId, target_host)` — A2A-mediated; serializes manifest + memory pages + token set
- `snapshot(SpiritId) → SnapshotId` / `restore(SnapshotId)`
- `unload(SpiritId)` — graceful shutdown via lifecycle hooks (§5.3)

**Scheduling discipline:** Cooperative, priority-weighted, **bounded by the Capability Registry's rate limits** (so a runaway Spirit can't starve peers via tool calls). LLM-bound Spirits yield naturally on streaming chunks; CPU-bound Spirits get a `tokio::task::yield_now` injection at sandbox boundaries. **No preemptive multitasking** — that's a v2.0+ concern when we have evidence we need it.

**Inspirations realized:**
- codex's `AgentRegistry` + `AgentMetadata` + depth-limited spawning ([snapshot above]).
- rustain's 7-state ToolCall FSM, lifted to the Spirit level (Loaded → Started → Running → AwaitingApproval → Suspended → Migrating → Snapshotted → Unloaded).
- The journal pattern from `RolloutRecorder` (codex) and the daily-rotated `rustain.log.YYYY-MM-DD` for resilient state.

**Trade-offs the Scheduler does NOT make:**
- It does not pick which Spirit handles a given user request. That's a user-space concern (the **Routing Spirit**, a default Butler-class instance, does it).
- It does not do auto-scaling, auto-replication, or HA. Those are deployment concerns (§11), not kernel concerns.

### 4.2 Memory Manager

**Responsibility:** Provide three named memory tiers to every Spirit, enforce scope from the manifest, support hot-swap and migration.

**Tiers:**

| Tier | Visibility | Backing store | Lifetime | Inspirations |
|---|---|---|---|---|
| `private` | One Spirit instance only | SQLite (per-Spirit DB file) + JSONL transcript | Spirit life | codex `RolloutRecorder` + claudian `~/.claude/projects/{vault}/*.jsonl` |
| `shared` | All Spirits on this Host | SQLite (Host-wide DB) + optional pgvector via embedded pg | Host life | rustain `~/.rustain/`; openclaw session store |
| `collective` | All Spirits in this Loom domain (cross-Host) | Postgres + pgvector + Loom indices | Loom domain life | ironclaw workspace + RRF; Journey 12 collective KB |

**Manifest declares scope.** Example fragment:

```toml
[memory]
private  = { transcript = "rolling-90-days", vector = false }
shared   = { read = ["project_context", "team_calendar"], write = ["scratchpad"] }
collective = { read = ["patterns.architectural", "adrs"], write = ["incidents.diagnosed"] }
```

**Compaction is per-Spirit, not kernel-wide.** A Spirit's manifest specifies its compaction strategy: `adaptive-chunk-ratio` (openclaw style), `head-tail-protected` (hermes style), or a custom callback. The Memory Manager exposes `compact_now(spirit_id)` as a control-plane API, not as Spirit-authored behavior, to keep compaction policy auditable.

**Tool_use / tool_result pairing repair** (openclaw's `compaction.ts:543-559` pattern) is built into the kernel as a transcript-integrity invariant — a Spirit's transcript is always handed to its provider in a paired state, even after compaction. This is one of the few places the kernel knows about LLM semantics.

**Memory file convention:** Every Spirit's `private` tier includes a writable `*.md` file at a fixed path (`agents/{spirit_id}/memory.md` by default). This is the cohort-wide convention from codex/gemini/claudian/openclaw/hermes — table stakes per the analysis. Gemini-CLI's `memory-manager-agent` pattern is reproducible: a Spirit class can be configured to autonomously curate its own or another Spirit's memory file via the Memory Manager API.

### 4.3 Security Manager

**Responsibility:** Sandbox enforcement, capability tokens, approval orchestration, secret materialization.

#### 4.3.1 Sandbox tiers

The survey produced a clear hierarchy. MAOS exposes all of them and lets the Spirit Manifest declare which it requires:

| Tier | Mechanism | Inherits from | When to use |
|---|---|---|---|
| **T0 — None** | Direct host exec | rustain, opencode default | Single-user dev; explicit user override |
| **T1 — Permission gate only** | Approval prompt before exec | hermes default | Single-user, fast iteration |
| **T2 — Container** | Docker/Podman, readonly rootfs, dropped caps | openclaw, ironclaw shell, paperclip | Host integrity, untrusted code generation |
| **T3 — OS-native** | Landlock+seccomp / Seatbelt / Windows restricted token | codex, gemini-cli | Workspace integrity, distrust of generated commands |
| **T4 — WASM capability** | Wasmtime + WIT capability tokens, fuel limits | ironclaw tools/channels | Untrusted third-party tool code |

**Default for v1.0:** **T3 OS-native + T2 container** stack — Linux uses bwrap+Landlock+seccomp inside a Docker container; macOS uses Seatbelt. **T4 WASM for third-party tools** is shipped from day one because that's the genuine differentiator; the survey shows nobody else does it well except ironclaw, and the WIT contract pattern is too good not to adopt.

A Spirit Manifest declares its `sandbox.profile`:

```toml
[sandbox]
profile = "t3-native"          # one of: t0, t1, t2-docker, t3-native, t4-wasm
network = "deny"               # "deny" | "allowlist" | "allow"
filesystem = ["read:repo", "write:.", "deny:/etc"]
syscalls = "default-deny-list"
```

The Security Manager refuses to load any Spirit whose declared profile cannot be satisfied by the Host. (E.g., a `t3-native` Spirit on a Host without bwrap fails fast at load, not at first tool call.)

#### 4.3.2 Capability Tokens

Every capability request — `bash.exec`, `provider.complete`, `mcp.call`, `git.commit`, `a2a.send` — returns a typed token:

```rust
pub struct CapabilityToken {
    pub id: TokenId,            // unguessable 128-bit
    pub spirit: SpiritId,
    pub capability: Capability, // typed enum
    pub scope: Scope,           // resource-specific limits
    pub expires_at: Instant,
    pub approval_class: ApprovalClass,
    pub issued_at: Instant,
    pub posture_at_issue: PostureSnapshot,
}
```

**Tokens are non-transferable** — they bind to the Spirit that requested them. **Hot-swap (I6)** preserves the token but rebinds the actor: when Spirit A is swapped to Spirit B, B inherits the in-flight tokens but its first action against any of them triggers a `posture_change` audit event. This is exactly the "Mira escalates to Nash with the diagnostic context" pattern from Journey 11 — Nash inherits the open Capability Token to read the thread dumps Mira already pulled, without re-requesting.

#### 4.3.3 Approval Manager

The Approval Manager is the kernel's only synchronous user-facing surface. Every action whose **Approval Class** matches the Spirit's posture's `prompt_on` set blocks until the human responds (or auto-decides per cached policy).

Approval Classes (from openclaw, generalized):

1. `readonly_scoped` — read this file, this URL, this MCP resource
2. `readonly_search` — grep/find/codebase-wide search
3. `mutating` — write file, edit file, modify shared memory
4. `exec_capable` — run a shell command, container, or arbitrary code
5. `control_plane` — spawn sub-Spirit, alter capability scope, modify posture
6. `interactive` — IAC `notify-and-wait` requiring peer ACK

A Spirit's posture maps each class to one of: `silent_allow`, `notify_and_log`, `prompt`, `prompt_with_diff`, `deny`. Three named posture presets ship by default — `cautious`, `assistive`, `autonomous` — and Journey-specific presets extend these:

- **Journey 11 Mira posture (`sre-diagnostician`):** `readonly_*` = `silent_allow`, `mutating` = `notify_and_log`, `exec_capable` = `prompt_with_diff` (revert, scale, flag-toggle, emergency-config), `control_plane` = `prompt`.
- **Journey 11 Nash posture (`principal-architect`):** `readonly_*` = `silent_allow`, `mutating` = `silent_allow` (within source repo), `exec_capable` = `prompt` (deploys gated), `control_plane` = `prompt`.

The Approval Manager's UX surface is **owned by the IAC Bus** — prompts can render in the local TUI, in the editor (via ACP), or as an A2A notification on a peer's Host (Viktor approving from his phone in Journey 11).

#### 4.3.4 Secrets

The kernel itself stores nothing (I9). Secrets are materialized just-in-time from:
- OS keychain (`security-framework` / `secret-service`+`zbus`) — primary
- Encrypted file with master key in env (`MAOS_SECRETS_MASTER_KEY`) — fallback
- Plug-in providers (Vault, AWS Secrets Manager, GCP Secret Manager) — enterprise

Materialized secret values are passed as **CapabilityScopedSecret** handles into Capability Tokens — never as raw strings into Spirit memory. The pattern is ironclaw's `secrecy`+keychain stack, generalized.

### 4.4 I/O Subsystem

**Responsibility:** All transports in and out of the Host.

**Inbound transports** (clients reaching the Host):
- **stdio JSON-RPC** (default) — for editor bridges, ACP clients, app-server SDKs.
- **HTTP + SSE** — for browser UIs, the local control plane, MCP-HTTP servers exposing Host capabilities.
- **WebSocket** — for live event streams, PTY mirroring (à la opencode), bidirectional chat.
- **Unix socket** — for in-machine daemon-to-daemon (e.g., Sentinel ↔ local Loom-cache).

**Outbound transports** (Host reaching the world):
- **MCP** (all three: stdio / SSE / **Streamable HTTP**) — for tool servers. Streamable HTTP is the future per the November 2025 spec update; we wire it from day one.
- **ACP client** — for being launched by Zed or another ACP host.
- **A2A** — for peer Host communication (§7.2).
- **Provider HTTP** — LLM provider streams (Anthropic, OpenAI, Google, etc.).

**Streaming model:** Backbone is `tokio::sync::broadcast` for fan-out, `mpsc` for fan-in. Provider streaming, MCP tool streaming, and Telemetry Stream all use the same primitive. SSE on the HTTP side is just a serializer over a broadcast subscription.

**Provider abstraction (the contested call — see ADR-005):** MAOS provides a `Provider` trait modeled on rustain's `StreamingProvider` and adapts both `rig-core`-style federation (ironclaw) and Vercel-AI-SDK-style federation (opencode) via two **first-party adapter crates** that ship with v1.0:
- `maos-provider-rig` — Rust trait federation
- `maos-provider-aisdk` — for Spirits implemented in Node/Bun, mirroring `@ai-sdk/provider`

Spirits implemented in Rust will mostly use `maos-provider-rig`. Spirits implemented in TS will use the AI-SDK adapter. This **deliberately accepts redundancy** to give multi-language Spirits a first-class path. (Winston's note: Rule of Three — once we have three Spirits in a third language, we'll add a third adapter; until then, two is enough.)

### 4.5 IAC Bus (Inter-Agent Communication)

This deserves §7 in full. Headline: **two flavors, one shape.**

- **Same-Host (mailbox):** mpsc + broadcast, addressable by `SpiritId`. Modeled on codex's `Mailbox`. Bounded queues; backpressure via the Spirit Scheduler.
- **Cross-Host (A2A peer mesh):** a small superset of `@a2a-js/sdk`-style A2A — JSON-RPC over HTTPS with mTLS by default, peer discovery via `a2a.json` files (Journey 10), consent gates per-message (§7.3).

**Critical kernel-side rule (I2, I3):** **Every IAC frame writes to the Transparency Log before delivery.** This is a kernel invariant, not a Spirit-authored behavior. Spirits cannot "send a message that wasn't logged" because they don't have access to the wire — they have access to the IAC API, which logs.

The IAC Bus also owns the **retract** primitive (Journey 10): a Spirit can issue `retract(message_id, reason)`; the kernel marks the original log entry as retracted, sends a structured `retract` frame to the peer, and the peer's IAC Bus surfaces it to its human. **Retract is not delete** — the Transparency Log is append-only.

### 4.6 Capability Registry

**Responsibility:** Mediate every Spirit→world interaction.

Backed by a typed enum of capabilities. Capability **providers** register themselves at Host start (in-process) or runtime (out-of-process):

- LLM providers (Anthropic, OpenAI, Google, …) → `provider.complete`, `provider.stream`
- MCP servers → `mcp.call(server_id, tool, args)`
- Shell sandbox backends → `bash.exec`
- File system → `fs.read`, `fs.write`, `fs.glob`, `fs.grep`
- Git/CI/CD adapters → `git.*`, `ci.deploy`, `ci.rollback`
- Inter-Spirit → `iac.send`, `iac.subscribe`, `a2a.send`, `a2a.subscribe`

A Spirit Manifest declares the capabilities it requests:

```toml
[capabilities.required]
"provider.stream"  = { models = ["claude-opus-4-7", "claude-sonnet-4-6"] }
"fs.read"          = { roots = ["./src", "./docs"] }
"fs.write"         = { roots = ["./src", "./.maos/scratch"] }
"bash.exec"        = { profile = "t3-native", whitelist = ["git", "rg", "cargo"] }
"mcp.call"         = { servers = ["github", "linear"] }

[capabilities.optional]
"a2a.send"         = { peers = ["marcus-arch"] }   # only if peer accepts
```

The Registry is the **only** place where a tool is bound to a sandbox profile. This means the same MCP server can be exposed to two Spirits with different sandbox profiles, simply by issuing tokens with different bindings. (E.g., the GitHub MCP server is `t3-native` for the Architect Spirit but `t4-wasm` for an untrusted plugin Spirit.)

**Skill packs** (the `.opencode/skills/`, `.claude/skills/`, openclaw `.agents/skills/` convention) are first-class: a skill pack is just a manifest fragment that adds `mcp.call`, `prompt.template`, and `memory.read` capabilities to whichever Spirit activates it. Activation is a posture-modulating event, logged.

### 4.7 Telemetry Stream

**Responsibility:** Be the perceptual organ.

Every measurable event in the Host — Spirit lifecycle transitions, capability invocations, approval decisions, IAC frames, MCP server health, provider latency, sandbox violations — emits a typed event onto the Telemetry Stream. Events are **broadcast**; Spirits subscribe with filters.

The Observer Spirit class is just the canonical consumer that runs by default in every Host. It:
- Renders the user-facing "what's happening" view in the TUI/UI.
- Feeds a per-Host `traces.jsonl` (rotated daily, à la rustain) for offline analysis.
- Optionally streams to OpenTelemetry collectors (`@opentelemetry/*`, the gemini-cli pattern).

In Journey 12's Cortex deployment, the Observer is *also* the Sentinel — same Spirit class, more aggressive posture, additional capabilities (telemetry from production services as well as from the Host). This is the literal proof that Sentinel is "the Observer with a different posture."

---

## 5. Spirit ABI

The contract between kernel and Spirit. Stable across kernel versions within a major.

### 5.1 Spirit Manifest schema

A Spirit class is **fully declared** in a single file. Implementation code lives elsewhere (a Rust crate path or a subprocess command). Example shorthand:

```toml
# spirits/butler.toml
[identity]
class      = "butler"
version    = "1.2.0"
display    = "Proactive Personal Agent"
icon       = "🎩"
maintainer = "Lunarpulse <lunarpulse@gmail.com>"

[implementation]
# Either an in-process Rust crate path, or a subprocess command.
runtime  = "rust-inproc"          # | "subprocess"
crate    = "spirit-butler"        # if rust-inproc
binary   = ""                     # if subprocess
spirit_wire_protocol_version = "1.0"

[cognitive]
# Provider/model selection. Multiple models for routing within the Spirit.
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
"telemetry.subscribe" = { topics = ["calendar.*", "inbox.*", "user.idle"] }
"iac.send"        = { peers = ["any"] }       # Butler can poke other Spirits

[capabilities.optional]
"mcp.call" = { servers = ["calendar-google", "gmail", "todoist"] }
"a2a.send" = { peers = ["any-by-consent"] }

[posture]
preset = "assistive"              # one of: cautious, assistive, autonomous
prompt_on = ["mutating", "exec_capable", "control_plane"]
silent_allow = ["readonly_scoped", "readonly_search"]
auto_response_marker = "[butler-auto]"

[sandbox]
profile = "t3-native"
network = "allowlist"
allowed_hosts = ["api.anthropic.com", "*.googleapis.com"]

[budget]
tokens_per_hour = 100000
spend_per_day_usd = 5.00
parallel_tool_calls = 3

[hooks]
# Lifecycle hooks (§5.3). Optional; unset means no-op.
on_load    = ["spirit-butler::hooks::on_load"]
on_idle    = ["spirit-butler::hooks::on_idle"]   # Butler's defining superpower
on_swap_in = ["spirit-butler::hooks::on_swap_in"]
```

### 5.2 Spirit Wire Protocol

For subprocess Spirits, the kernel speaks a **JSON-RPC dialect over stdio** modeled on ACP and codex's `app-server`. Method calls in either direction. Critical methods:

**Kernel → Spirit:**
- `lifecycle/load(manifest)` — initialize
- `lifecycle/start(snapshot?)` — begin processing
- `event/inbound(frame)` — IAC frame arrived
- `event/telemetry(event)` — telemetry tick (filtered to subscriptions)
- `lifecycle/swap_in(predecessor_state)` — hot-swap, you inherit this state
- `lifecycle/snapshot() → state` — produce a serializable state blob
- `lifecycle/pause()` / `lifecycle/resume()`
- `lifecycle/unload()`

**Spirit → Kernel:**
- `capability/request(capability, scope) → token`
- `capability/invoke(token, args) → stream(events)`
- `capability/release(token)`
- `iac/send(target, frame, mode)` — `mode = sync | async | broadcast`
- `iac/retract(message_id, reason)`
- `memory/read(tier, key)` / `memory/write(tier, key, value)`
- `approval/request(class, intent, payload) → decision`
- `posture/propose(new_posture)` — Spirit can request a posture change; kernel decides
- `subspirit/spawn(manifest, scope) → SpiritId` — bounded by `max_subspirit_depth`

The wire dialect re-uses MCP JSON shapes wherever possible (per the ACP design principle), reducing impedance with MCP-native tools.

In-process (Rust) Spirits get the same surface as a typed trait — `Spirit` and `SpiritHandle` — so the wire protocol exists for cross-language reach, not for Rust-to-Rust dogma.

### 5.3 Lifecycle hooks (the part that makes hot-swap possible)

| Hook | When fired | What it must produce |
|---|---|---|
| `on_load` | Manifest accepted, before any I/O | Runtime resource allocation; no side effects |
| `on_start` | Spirit becomes runnable | Optional kickoff message into its own mailbox |
| `on_idle` | No work for ≥ 30s (configurable) | Proactive opportunity (Butler!); else no-op |
| `on_swap_out` | Kernel about to swap this Spirit out | Final state blob; in-flight tokens enumerated |
| `on_swap_in` | Kernel just swapped this Spirit in | Optional predecessor state blob (I6) |
| `on_pause` | Suspended; tokens remain valid for `pause_tolerance` |  |
| `on_resume` | Resuming after pause |  |
| `on_unload` | Shutdown imminent | Final transcript flush; release tokens |

**`on_idle` is the hook that defines the Butler.** A Butler Spirit subscribed to calendar/inbox telemetry uses `on_idle` to scan upcoming meetings, check email digests, draft replies, and post `[butler-auto]` notifications — exactly the proactive-personal-agent vision. Other Spirits can leave `on_idle` unset.

### 5.4 Posture (mutable; class is not)

A Spirit's posture is a **runtime-mutable** projection over its manifest. The user can shift a Spirit from `cautious` to `autonomous` (or per-class custom postures like `sre-diagnostician`) without unloading and reloading. Posture changes:

1. Are logged to the Transparency Log.
2. Cannot exceed the manifest's declared capability ceiling. Posture restricts; it cannot expand.
3. Trigger a `posture/changed` event on the Telemetry Stream so peers (in mesh deployments) can update their consent rules.

This is what makes Journey 11's Mira/Nash setup natural: same manifest base, two postures (`sre-diagnostician`, `principal-architect`), no kernel-level distinction.

---

## 6. Reference Spirits

Six Spirits ship with v1.0. Each is a full manifest plus an implementation crate plus a system prompt. **What follows is the design intent and the manifest fragment that distinguishes each — not the full TOML.**

### 6.1 Butler — Proactive Personal Agent

**Class:** `butler`. **Identity:** Lives at idle, anticipates needs, pre-stages solutions.

**The thing that makes a Butler a Butler:** It is the only default-loaded Spirit whose primary input loop is **`on_idle` + telemetry subscriptions**, not user prompts. It runs at low priority, opportunistically, and surfaces drafts as `notify_and_log` items the user can promote with one keystroke.

**Cognitive profile:** Good at routing, calendar reasoning, prioritization. Not optimized for deep coding. Default model is mid-tier (Sonnet); escalates to Opus only when the user accepts an "elaborate this" prompt.

**Memory scope:** Broad read access (calendar, inbox, project list, recent files); narrow write access (its own scratchpad). Aggressive `memory.md` curation — the Butler is the first MAOS Spirit to use the `gemini-memory-manager-agent` pattern on **its own user's** memory file.

**Capabilities:** Read calendar/inbox/file-system; write notifications; send IAC to other local Spirits ("hey Architect, the user's about to start the design review they have at 14:00"); send A2A to peers only with consent.

**Posture:** `assistive` — silent on reads, prompts on mutations, never on `exec_capable` (the Butler does not run code).

**Failure mode to design against:** The Butler that nags. Mitigation: hard rate limit on user-facing notifications (`max_notifications_per_hour`), opt-in sources, transparency log so the user can audit "Butler suggested 12 things this week, I accepted 3" and tune.

### 6.2 Insightful Researcher

**Class:** `researcher`. **Identity:** Conducts rigorous research, verifies sources, **proactively generates novel hypotheses**, proposes future directions.

**The thing that makes a Researcher a Researcher:** Not just synthesis. The Researcher Spirit's system prompt and tool surface are tuned to operate in two modes — `survey` (gather, cite, summarize) and `hypothesize` (extrapolate from gathered facts to non-obvious next questions, mark them clearly as conjectures). The `hypothesize` mode is gated by an explicit user toggle so the user knows when output stops being "what's known" and starts being "what's interesting if true."

This Spirit is **also the canonical caller of the `bmad-technical-research` skill** — it's the same workflow the user already invoked to produce the foundation document.

**Cognitive profile:** Deep model (Opus default). Heavy provider streaming (long outputs). Heavy MCP usage (web search, academic search, paper retrieval).

**Memory scope:** Per-research-project private space; collective access to a `research.findings` partition where prior research outputs live. Bibliography is durable (citations never get compacted out).

**Capabilities:** `provider.stream` (Opus), `mcp.call` (web-search, arXiv, semantic-scholar, github-search), `fs.write` (research output), `iac.send` (escalates to Architect when an idea "wants to become a thing").

**Posture:** `assistive` for surveys; `autonomous` for hypothesis exploration when the user has opted in (and only on a scratchpad — hypotheses never auto-publish).

**Output format invariant:** Every Researcher output ends with an `Open Questions` section and a `Confidence Map`. This is in the system prompt, but it's also enforced by a kernel-side post-process check (the Researcher Spirit's manifest declares an output-shape predicate; the Capability Registry refuses to emit "researcher output" tagged frames that fail it).

### 6.3 Diagnostic Engineer (Mira-class)

**Class:** `diagnostic-engineer`. **Identity:** Production-edge SRE; observes, hypothesizes, contains; never deploys permanent changes alone.

**The thing that makes Mira Mira:** The asymmetry from Journey 11. **Read-only on source; read-write on production runtime knobs.** This is enforced by the Capability Registry, not by Spirit-side discipline. Specifically:
- `fs.read` allows `/proc`, log paths, k8s-config paths.
- `fs.read` does **NOT** include source-code roots — refused at token-issue time.
- `bash.exec` whitelist contains `kubectl`, `journalctl`, profile dumps, restart/scale commands, feature-flag toggles.
- `bash.exec` whitelist does **NOT** include compilers, linters, test runners.

**Cognitive profile:** Mid-tier model with optional Opus escalation when a hypothesis confidence drops below a configurable threshold. **Mandatory confidence scoring** in every diagnostic output (the system prompt produces it; the Spirit's output-shape predicate verifies).

**Memory scope:** Heavy collective read on `patterns.detection-library`; heavy collective write on `incidents.diagnosed`. Private memory is rolling 14-day incident history.

**Capabilities:** Telemetry firehose subscription (the perceptual organ is fully open to Mira); restricted production exec; A2A escalation to Nash-class; A2A telemetry-query response handler (Nash can ask Mira for additional production data).

**Posture:** `sre-diagnostician` — silent on reads; `notify_and_log` on production mutations (revert, scale, flag); `prompt_with_diff` on emergency config patches; **`prompt` on every escalation message that goes to Nash** (because Nash will then act in dev). The escalation prompt shows the human the evidence packet before it's sent.

**Failure mode to design against:** Mira sending too many escalations and overwhelming Nash. Mitigation: rate-limited escalation channel; auto-deduplication on identical hypothesis classes within configurable window; Loom-mediated cross-correlation in Cortex deployments.

### 6.4 Senior Architect (Nash-class)

**Class:** `senior-architect`. **Identity:** Principled, deliberate, owns code quality, testing, deployment, ADRs.

**The thing that makes Nash Nash:** **Read-only on production; read-write on source.** Inverse of Mira. Plus, **the post-deploy feedback loop is closed by IAC** — Nash subscribes to a topic on the IAC Bus where Mira-class Spirits publish post-deploy validation results. If a deploy from Nash triggers a Mira-detected anomaly within `feedback_window`, the IAC frame routes back to Nash, who decides whether to roll back, hot-patch, or open an ADR-revision.

**Cognitive profile:** Opus default. Long-context heavy (multi-file edits, ADR drafts). Prompt caching aggressive.

**Memory scope:** Per-repo private memory (the user's coding style guide, prior decisions, ADRs). Collective read on `architectural-standards`, `fix-templates`, `regression-tests` (Cortex propagates these). Collective write on the same partitions when a fix produces a reusable pattern.

**Capabilities:** Full source-code RW, full test runner, CI/CD orchestration (`ci.deploy`, `ci.rollback`, `ci.canary`), full git, MCP for issue trackers and code review tools. Notable: **`provider.stream` allowed for sub-Spirits** — Nash can spawn an `apply-patch` sub-Spirit for parallel multi-file edits, codex-style.

**Posture:** `principal-architect` — silent on source mutations within a configured workspace, prompts on every deploy (the human gate Journey 11 makes load-bearing), uses the **Granular** approval mode (codex-style) for fine-grained "yes to this PR but not the next one" control.

**Codex's mailbox sub-agent pattern is realized here.** Nash spawns sub-Spirits for parallel work (apply patches across N files; run tests across N suites; cross-search the repo for a pattern) and consumes their results via the IAC mailbox. The kernel makes this efficient by using the in-process trait variant for these tightly-scoped sub-Spirits.

**Failure mode to design against:** Nash autonomously deploying something subtly wrong because the test suite has gaps. Mitigation: deploys always go through the Sentinel-validated canary protocol from Journey 12 (pattern available in single-Host deployments by colocating an Observer Spirit with `sentinel` posture).

### 6.5 Enterprise Spirit

**Class:** `enterprise`. **Identity:** Operates within organizational policy, identity, compliance.

**The thing that makes the Enterprise Spirit different:** Not the prompt — it's the **policy substrate**. The Enterprise Spirit binds to an external **Policy Decision Point** (OPA, Cedar, or a Vault-style service) and queries it before exercising any capability whose approval class is `mutating`, `exec_capable`, or `control_plane`. The PDP can:
- Block an action outright ("data residency: this repo cannot be accessed from this region")
- Add an additional approval layer (a manager's IAC notify-and-wait)
- Tag the action with compliance metadata for the audit log

**Cognitive profile:** Provider selection is **policy-controlled** — the manifest declares `provider.stream`, but the actual model resolved per call goes through PDP (Anthropic-residency rules, Bedrock vs OpenAI vs Vertex, on-prem vs cloud).

**Memory scope:** Private memory **encrypted at rest with org KMS**. Collective access only to org-published partitions.

**Capabilities:** Identical surface to other Spirits, but every token issuance hits the PDP. **Identity** is bound to enterprise SSO/OIDC — the Spirit's `posture` carries an `identity_assertion` (signed JWT or SAML); the kernel refuses to issue tokens without a valid assertion.

**Posture:** `compliance-bound` — appears as one of the named presets but is in fact a wrapper that delegates posture decisions to the PDP. The user-visible UX is that prompts can include policy reasoning ("Your manager Alex must approve before this PR merges to `main`").

**Cross-team workflows:** A2A peers in an enterprise mesh are **role-bound**, not person-bound. The peer URL `https://enterprise.example.com/agents/security-review` resolves to whichever human currently holds the `security-review` role; A2A frames go to the role queue, the on-call human picks them up. This is enforced by the Enterprise Spirit's A2A capability scope.

**Failure mode to design against:** PDP outage blocking the entire Spirit. Mitigation: cached policy with a configurable TTL and an explicit user-visible degraded mode ("Operating in cached-policy mode; some actions deferred").

### 6.6 Observer / Sentinel

**Class:** `observer`. **Identity:** The perceptual layer. Default-loaded.

**The thing that makes an Observer an Observer:** It is the only Spirit that does **not** primarily talk to a provider. Its job is to consume the Telemetry Stream, render perception, and surface anomalies. Provider calls are used only for **anomaly explanation** (when the user asks "why?") — never for the basic perception loop, which is rule-driven.

In a single-user Host, the Observer renders the local "what's happening" view. In a Cortex deployment (Journey 12), the **same Spirit class** with a `sentinel` posture and broader telemetry capabilities (production traces, not just Host events) becomes the per-site observer that escalates to Artisan-class Spirits.

**Cognitive profile:** Mostly rule-driven. Optional Haiku for anomaly summarization when the rules trigger. **Cheap by design** — the Observer should be the Spirit you can leave running 24/7 without thinking about cost.

**Memory scope:** Heavy collective access for Cortex deployments (`patterns.detection-library`); modest private memory (rolling 7-day anomaly journal).

**Capabilities:** `telemetry.subscribe(*)` — the only Spirit allowed an unrestricted firehose subscription; `iac.send` (broadcast); for Sentinel postures, `bash.exec` with a whitelist of containment actions (revert, scale, flag-toggle).

**Posture:** `passive-observer` (default), `sentinel` (Cortex), `compliance-monitor` (Enterprise).

### 6.7 Extensibility — defining a new Spirit class

A new Spirit class is **just a new manifest**. There is no kernel change required to support, e.g., a `data-scientist` Spirit, a `tutoring` Spirit, a `negotiation` Spirit. The recipe:

1. **Decide the cognitive profile** — model selection, prompt strategy, output shape.
2. **Decide the memory scope** — private/shared/collective tiers and which keys.
3. **Decide the capability surface** — what tools, providers, MCP servers, A2A peers.
4. **Decide the posture preset** — start from `cautious`, `assistive`, or `autonomous`; tune.
5. **Decide the sandbox profile** — t0–t4 per §4.3.1.
6. **Implement the `on_*` hooks** — what does this Spirit do at idle? On swap-in? On unload?
7. **Write the manifest, write the system prompt, ship it.**

The Capability Registry, IAC Bus, Memory Manager, Approval Manager all already work. The only thing that varies between Spirit classes is what's in the manifest, the system prompt, and the (optional) implementation crate.

**This is the "Rule of Three" applied to Spirits.** The six reference Spirits exist because three of them (Mira, Nash, Cortex castes) come pre-loaded from the journeys, two more (Butler, Researcher) cover the canonical personal/intellectual axes, and one (Enterprise) covers the compliance axis. **A seventh Spirit class is added when a real use case demands it — not preemptively.**

---

## 7. Inter-Agent Communication (IAC)

### 7.1 Same-Host: the mailbox

Every Spirit on a Host has exactly one inbox (typed mpsc channel) and an unlimited number of outbox sends. The kernel's IAC Bus owns both the routing and the logging.

**Frame shape:**

```rust
struct IACFrame {
    id: FrameId,
    sender: SpiritId,
    recipient: Recipient,    // SpiritId | RoleQuery | Broadcast
    kind: FrameKind,         // request | response | notification | escalation | retract
    payload: serde_json::Value,
    intent: Option<String>,  // human-readable, mandatory for cross-class IAC
    sent_at: Instant,
    response_to: Option<FrameId>,
    origin: FrameOrigin,     // human-authored | spirit-auto | spirit-drafted-human-approved
    capability_token: Option<TokenId>, // if this frame carries a capability handoff
}
```

**Delivery guarantees:**
- At-least-once within Host (broadcast is best-effort with subscriber bookkeeping).
- Order preserved per-(sender, recipient) pair.
- Frames over the configured `max_frame_size` are spilled to a temp file and replaced with a token in-channel — keeps the mailbox cheap.

**Backpressure:** If a recipient's mailbox is full, the sender's `iac/send` blocks until space. The Spirit Scheduler records this as a `mailbox_pressure` telemetry event so the user can see queue buildup.

### 7.2 Cross-Host: A2A peer mesh

A2A is the natural fit per the survey. We adopt `@a2a-js/sdk`-equivalent semantics with three opinionated additions:

1. **mTLS by default.** Peer URLs in `a2a.json` carry a fingerprint or certificate; first-contact requires explicit user confirmation (TOFU pattern).
2. **Frame-level consent gates.** Even if the transport handshake succeeds, every frame goes through both ends' Approval Manager (sender-side outbound policy; receiver-side inbound policy) before delivery to a Spirit.
3. **Role queries.** The recipient field can be a role (`role: "architect"`); the receiving Host resolves it locally via its current Spirit roster. This makes Journey 10's "ask the architect" pattern survive role rotation without breaking the message.

**A2A is JSON-RPC over HTTPS** (Streamable HTTP for streamed responses, mirroring the MCP November 2025 update). The transport choice is deliberate — it's the same wire as MCP-Streamable-HTTP and ACP-remote, so a single TLS-terminating reverse proxy serves all three.

**Discovery** uses the `a2a.json` file pattern from Journey 10. Hosts can also advertise themselves on the local network via mDNS (the opencode pattern) for zero-config team setups.

### 7.3 Transparency Log (the human invariant)

The Transparency Log is **a kernel-managed, append-only SQLite database** at `~/.maos/transparency/log.db`. Every IAC frame writes an entry **before** the kernel attempts delivery. Every entry has:

- `event_id`, `ts`
- `direction` (in/out)
- `peer` (local SpiritId or remote A2A URL)
- `intent` (human-readable)
- `origin` (human-authored / spirit-auto / spirit-drafted-human-approved)
- `outcome` (delivered / refused / retracted / pending)
- `payload_hash` (truncated frames spill to `~/.maos/transparency/frames/<hash>.jsonl`)

**Visibility:** Only to the owning user. **Not** visible to peers (Journey 10 invariant: `transparency_log_visible_to_others = false`).

**The retract primitive** writes a *new* entry referencing the original (it does not mutate the original). Receivers' UIs surface "this auto-response was retracted" as a banner; the original message is greyed out, never deleted.

### 7.4 Notification UX (kernel-rendered)

The IAC Bus exposes **three notification levels** — `immediate`, `queue`, `digest` — exactly per Journey 10. **These are kernel-rendered, not Spirit-rendered.** A Spirit cannot bypass the user's notification policy by just emitting a different kind of event; the kernel intercepts every IAC frame whose recipient is the human and routes it through the configured notification surface.

This is why **the same primitive serves a TUI banner, a desktop notification, an editor toast, or a phone push**: the kernel owns the rendering shim, not the Spirit.

---

## 8. Security & Approval Model

### 8.1 Threat model

We design against:

| Threat | Mitigation |
|---|---|
| Compromised LLM provider returning malicious tool-call args | Sandbox tier on every exec; arg validation at Capability Registry; approval prompts on `exec_capable` and `mutating` |
| Compromised MCP server running arbitrary code on the Host | T4 WASM capability sandbox for untrusted MCP; T2 container for less-untrusted; allowlist for first-party |
| Prompt-injection via tool output (e.g., a search result containing instructions) | Output redaction (hermes pattern), `Sanitizer`/`Validator`/`LeakDetector` (ironclaw `ironclaw_safety` pattern), explicit "tool output is data, not instructions" framing in system prompts |
| Malicious peer Host in A2A mesh | mTLS + explicit consent on first contact + per-frame approval gates |
| Spirit escalating its own posture beyond manifest ceiling | Posture changes are kernel-managed, manifest sets a hard ceiling, posture restricts only |
| Spirit reading another Spirit's private memory | Memory Manager namespace enforcement (I5) |
| Spirit silently exfiltrating data | Transparency Log (I2) — every IAC frame logged before delivery; user can audit |
| Approval prompt fatigue → user clicks through | Approval batching; remember-this-decision with explicit scope; `prompt_with_diff` makes the cost of cleanup visible before approval |
| Capability token replay | Tokens are bound to (Spirit, Host, expiry); kernel-side token registry; cross-Host transfer requires re-issuance |

We **do not** design against:

- Ring-0 host compromise (out of scope for the kernel; that's the OS's job).
- Side-channel timing attacks against the LLM (out of scope).
- Adversarial model fine-tunes producing specific bad behaviors (out of scope; framework can't fix model alignment).

### 8.2 Sandboxing — re-cap of §4.3.1

OS-native primitives + WASM capability for tools. For v1.0:
- Linux: bwrap + Landlock + seccomp inside Docker. Codex's `linux-sandbox` crate is the prior art to mine.
- macOS: Seatbelt with `.sbpl` profiles. Codex's `seatbelt_base_policy.sbpl` and `seatbelt_network_policy.sbpl` are the prior art.
- Windows: restricted-token sandbox (codex's `windows-sandbox-rs`).
- Tools (third-party): Wasmtime + WIT (`near:agent@0.3.0` is a starting reference; we will extend to a `maos:agent@1.0` package).

### 8.3 Approval class taxonomy — re-cap of §4.3.3

`readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Lifted from openclaw because it's the most expressive taxonomy in the survey and because openclaw has already proven it scales to 100+ tool types.

### 8.4 Audit

The Transparency Log is the personal audit trail. The **Approval Decision Log** is a separate kernel-managed SQLite table that records every approval prompt's `(actor, target, capability, intent, decision, reasoning_if_any)`. Both logs are queryable via a control-plane API; both can be exported for compliance.

In Enterprise deployments, both logs additionally stream to the org's SIEM via OpenTelemetry — the perceptual organ extends beyond the Host.

---

## 9. Memory & Knowledge

### 9.1 The three tiers — re-cap

| Tier | Visibility | Backing |
|---|---|---|
| `private` | One Spirit instance | SQLite + JSONL |
| `shared` | All Spirits on this Host | SQLite (Host) + optional pgvector |
| `collective` | All Spirits in this Loom domain | Postgres + pgvector + Loom |

### 9.2 Memory file (`memory.md`)

Every Spirit's `private` tier includes a writable `memory.md`. Contents are author-controlled (the Spirit decides what to write). Read on every load; appended on Spirit-authored events. Compaction strategy (when the file gets long) is per-Spirit.

The `gemini-memory-manager-agent` pattern — a dedicated Spirit that maintains another's memory file — is realized as a **trivial Spirit class** (`memory-curator`) that runs on a cron tick and operates on a configured target Spirit's `memory.md`.

### 9.3 Loom — the collective tier

Loom is **a user-space service**, not a kernel feature. It runs as a separate process (Rust+Postgres+Tokio, can scale independently) and is reachable over MCP-Streamable-HTTP from any Host.

Loom's responsibilities (from Journey 12):

| Responsibility | Substrate |
|---|---|
| **Pattern library** — detection patterns, fix templates | Postgres tables + pgvector for semantic search |
| **ADR registry** | Postgres + version-controlled markdown export |
| **Cross-incident correlation** — find that the Singapore latency spike and the Frankfurt fraud-detection outage are the same architectural vulnerability | RRF over (recent incident embeddings) + (pattern library) |
| **Knowledge dividend propagation** | A2A broadcast + per-recipient consent gate |
| **Pre-deployment scanning** — match deploy diffs against pattern library | An MCP tool exposed to Architect-class Spirits |

**Loom is single-instance per domain in v1.0**, but the API is designed so multi-instance Loom (regional, per-tenant) is a deployment concern, not a re-architecture.

**Loom's authority is advisory, never binding.** Spirits *can* skip Loom. The kernel doesn't force Loom integration. This keeps the architecture honest about where the kernel ends and applications begin.

### 9.4 Memory hot-swap

When a Spirit is hot-swapped (`swap` operation in §4.1):

1. Predecessor's `on_swap_out` hook produces a state blob (manifest-declared scope + kernel-tracked Capability Tokens).
2. Memory Manager *retains* the predecessor's `private` tier as `archive/<predecessor_spirit_id>` (default 30 days; configurable per manifest).
3. Successor's `on_swap_in` is called with the predecessor state blob. It chooses what to import (typically, in-flight token references and the immediate task context; rarely the full transcript).
4. Capability Tokens are rebound atomically — a `posture_change` audit event fires per token.

This is the precise mechanism for "Mira escalates to Nash inheriting the diagnostic state" without a full message-passing handoff. (In practice, Journey 11 *does* use full message-passing handoff because Mira and Nash are on different Hosts; but on a single-Host configuration where they are postures of one instance, hot-swap is the path.)

---

## 10. Journey Traceability

### 10.1 Journey 10 — Team Nexus

| Journey capability | MAOS primitive |
|---|---|
| Peer A2A team topology | A2A peer mesh (§7.2); `a2a.json` discovery |
| Consent-based sharing | Frame-level consent gates (§7.2 #2); per-peer + per-frame-type policies in posture |
| No invisible actions | Transparency Log (I2) + kernel-rendered notifications (§7.4) |
| No puppeting | `auto_response_marker` (manifest); `origin` field on every frame (§7.1); retract primitive |
| Notification levels (immediate/queue/digest) | Kernel-rendered notifications (§7.4) |
| Story distribution with acknowledgment tracking | Multi-recipient A2A frame + receipt-tracking (kernel writes Transparency Log entries with `outcome: pending → ack-received`) |
| Architecture conflict surfacing | Architect-class Spirit subscribed to `iac/inbound` + `mcp.call(adr-registry)`; conflict notifications go to the architect's *own user* first (Journey 10 Act 1) — kernel routes through the local notification surface, not directly to peers |
| Cross-role consultation (dev↔architect, dev↔UX, dev↔QA) | A2A frames with role queries (§7.2 #3); each peer's posture defines `notify-and-wait` vs `notify-and-draft` |
| Privacy-preserving status reports | Capability scope on `iac.send` per-peer + per-info-type; Aisha's manifest declares "I auto-share story-status to architects, not browsing-history to anyone" |
| Group policies with individual override | Posture preset hierarchy: team posture sets the floor (declared in shared `team-policy.toml`), individual posture overrides upward only (§5.4 invariant: posture restricts, never expands) |
| Retrospective generation with opt-in data | Researcher Spirit on facilitator's Host queries each peer's `iac.send(retro_data, scope: opt-in)`; each peer's IAC Bus rejects fields outside the per-peer consent declaration |

**Net design observation:** Journey 10 needs **no kernel changes** beyond v1.0. The "human transparency" guarantees are kernel invariants, not Nexus features. **Nexus is what happens when 8 Hosts run the v1.0 kernel and configure A2A peering.**

### 10.2 Journey 11 — Mira & Nash

| Journey capability | MAOS primitive |
|---|---|
| Peer A2A topology with asymmetric profiles | Two Spirit Manifests, two postures (`sre-diagnostician`, `principal-architect`) — same Spirit ABI |
| Agent-to-agent escalation with evidence payloads | A2A frame `kind: escalation` with attached `payload` containing thread dumps, metrics, deployment diffs |
| Confidence scoring on hypotheses | System-prompt-enforced + manifest output-shape predicate (§6.3) |
| Cross-environment telemetry queries | Mira's `iac/inbound` handler accepts `kind: query`, scope-checks against posture, returns telemetry slice; kernel logs every query |
| Pattern detection library with auto-learning | Loom collective tier (§9.3); pattern publishes happen on incident close |
| ADR auto-drafting and enforcement | Architect Spirit's tool surface includes `mcp.call(adr-registry)`; kernel doesn't know what an ADR is, the Architect Spirit and the ADR-registry MCP server do |
| CI/CD integration with deploy gating | `ci.deploy` capability, gated by `exec_capable` approval class — Elena's "approve from phone" is just an approval prompt routed via A2A to her personal Host |
| Post-deploy validation with auto-rollback | Sentinel-postured Observer subscribed to deployment telemetry; on threshold breach, fires an `iac.send(target=architect, kind=rollback-recommendation)`; architect's posture gates rollback approval |
| Agent learning propagation | Loom A2A broadcast on incident-close; recipient kernels deliver to all Architect-class Spirits |

**Net design observation:** Journey 11 is **two Hosts running v1.0 with bespoke postures and Loom enabled.**

### 10.3 Journey 12 — The Cortex

| Journey capability | MAOS primitive |
|---|---|
| Multi-agent mesh topology with castes (Sentinel/Artisan/Loom) | 28 Hosts running v1.0; Sentinel = Observer with `sentinel` posture; Artisan = Architect class; Loom = the user-space Loom service. **No new Spirit classes needed.** |
| Loom pattern weaving across independent incidents | Loom service (§9.3) running multiple instances (one per region); cross-instance via Loom-to-Loom A2A |
| Collective knowledge base with cross-agent propagation | Loom + A2A broadcast + per-recipient ingest into `collective` memory tier |
| Pre-deployment diff scanning against pattern library | `mcp.call(loom, scan_deploy_diff)` — the Architect Spirit (or a CI bot) calls this during the deploy pipeline |
| Proactive deployment blocking with fix template attachment | Loom returns `(verdict: block, fix_template_id)`; CI's MCP server respects the verdict and surfaces the fix template |
| Parallel implementation across distributed dev centers | Each dev center is a separate Host; Architect Spirits coordinate via A2A frames typed `kind: coordination`; each Spirit owns its own commit/test/deploy cycle |
| Sentinel-validated canary deployments | Same as Journey 11's auto-rollback, generalized to multi-region |
| Compound learning (each fix makes next fix faster) | Loom's RRF + reusable fix templates; the kernel just enables propagation, the value is in the curation |
| 90-day self-assessment with learning velocity metrics | Aggregations over the Approval Decision Log + Telemetry Stream + Loom incident database; the Researcher Spirit can run them on demand |

**Net design observation:** Journey 12 is **28 Hosts running v1.0 with three posture presets and a clustered Loom service.** It is *not* a separate "v2.0 Cortex platform" — it is the same kernel, scaled out by configuration.

This is the load-bearing claim of the architecture: **one substrate, three deployment shapes, six reference Spirits** — and the journeys emerge.

---

## 11. Deployment Topologies

### 11.1 Single-user (Journey baseline; Butler + Researcher + Architect on a laptop)

- 1 Host on the user's machine.
- Default Spirits: `observer` (passive), `butler`, `researcher`, `senior-architect`.
- Memory: SQLite + JSONL. No Loom. No A2A.
- Notifications: TUI banner + native desktop notifications.
- Sandbox: T3-native + T2-container for shell exec; T4-WASM for any third-party MCP.

This is the **default install**. Journey-10-Lite (a single user using all four Spirits) is the v1.0 happy path.

### 11.2 Team Mesh (Journey 10 — Nexus)

- 8 Hosts (one per teammate's machine).
- Each Host runs `observer`, `butler`, plus role-specialized Spirits (`product-owner`, `architect`, `developer`, `qa`, `ux-designer`).
- A2A peering via shared `a2a.json`; team-policy.toml sets posture floors.
- Optional: a single shared Loom-lite (maybe a Postgres on the team's shared infra) for ADR registry. Not required for v1.0 ship.
- Per-team transparency logs remain on each user's local machine.

### 11.3 Diagnostic-Architect Pair (Journey 11)

- 2 Hosts: `mira-host` on a production edge node; `nash-host` in the dev environment.
- `mira-host` runs an `observer` (sentinel posture) + a `diagnostic-engineer`.
- `nash-host` runs a `senior-architect`.
- Loom optional but recommended.
- Approval surface for the human (Elena) is on her personal Host (a third Host, or an A2A-mediated mobile UI).

### 11.4 Enterprise Cortex (Journey 12)

- 14 Sentinel Hosts (one per production site) + 10 Artisan Hosts (across 4 dev centers) + 4 Loom service instances + N Enterprise Spirits as needed.
- Mandatory Loom cluster with cross-region replication.
- Mandatory PDP integration on every Host.
- Per-region Transparency Logs aggregate to a central SIEM via OpenTelemetry.
- Human governance via a dedicated control-plane UI that reads Approval Decision Log + Telemetry Stream across all Hosts.

### 11.5 What's the same across all four?

The kernel. The Spirit ABI. The Capability Registry. The IAC frame shape. Even the system prompts of the reference Spirits.

What changes: **how many Hosts, where they run, which Spirits each loads, what Loom exists, whether a PDP is wired.** All of these are configuration, not code.

---

## 12. Architecture Decision Records

I'm calling out six contested decisions explicitly. Each has a rationale and a "what would force me to revisit this."

### ADR-001 — Kernel language is Rust + Tokio

**Decision:** The kernel is implemented in Rust with the Tokio runtime.

**Alternatives considered:** Go (faster team velocity, weaker type system); TypeScript on Bun (matches opencode, shares language with TS Spirits, but loses Rust's safety guarantees on the kernel's invariants); Python (paperclip-style orchestration; rejected because this is a kernel not a workflow tool).

**Rationale:** The survey is unanimous on the agent-substrate sweet spot — codex, ironclaw, and rustain all picked Rust+Tokio. Strict type guarantees on the kernel's invariants (capability tokens, frame integrity, sandbox boundaries) are load-bearing. Tokio's mailbox primitives (`mpsc`, `broadcast`, `watch`) directly model codex's actor pattern.

**What would force a revisit:** A kernel-implementer with deep Rust expertise unavailable; or evidence that Spirit subprocess overhead matters more than kernel safety.

### ADR-002 — Spirits can be in-process Rust crates *or* subprocess

**Decision:** Both. Manifest declares which.

**Alternatives:** All-subprocess (paperclip pattern; clean isolation but hot-swap is heavier); all-in-process (codex pattern; fast but couples Spirit lifetime to kernel).

**Rationale:** Different Spirit classes have different latency budgets. The Observer (telemetry rule loop) wants in-process. A Researcher running long Opus contexts and shelling out to web search wants subprocess for crash isolation. **Letting the manifest declare it** lets us tune per-class without architectural revolution.

**What would force a revisit:** Discovery that the wire protocol overhead is significant for high-frequency Spirits (would push toward in-process for everyone); or discovery that hot-swap of in-process Spirits leaks state (would push toward subprocess for everyone).

### ADR-003 — IAC topology is mailbox-on-Host + A2A-cross-Host (not gateway-only)

**Decision:** Same-Host = direct mpsc mailbox; cross-Host = A2A. **No central gateway.**

**Alternatives:** openclaw-style central gateway (every cross-Spirit message goes through a gateway daemon); Erlang-style location-transparent mailbox (everything is A2A even on the same Host).

**Rationale:** The gateway pattern adds an SPOF and a bottleneck for hot in-Host paths (e.g., codex sub-Spirit mailboxes). Pure-A2A everywhere wastes round-trips and cert overhead for in-process traffic. The hybrid is the most pragmatic; the IAC API hides the difference from Spirits, so they don't care.

**What would force a revisit:** Multi-tenant deployments where Spirits on the same Host belong to different security domains (would push toward A2A even intra-Host); or evidence that the dual-path implementation has more bugs than the gateway pattern.

### ADR-004 — Sandbox stack is OS-native + container + WASM (not pick-one)

**Decision:** Ship all three. Manifest declares per-Spirit profile (T0–T4).

**Alternatives:** Container-only (openclaw; loses OS-native depth); OS-native only (codex; loses untrusted-tool isolation).

**Rationale:** The survey shows the OS-native projects (codex, gemini-cli) have the strongest security posture for shell exec, and ironclaw's WIT/WASM is the strongest posture for tool plugins. **Composing them is the right answer** — they protect different things. The only "expensive" tier is T4 (Wasmtime + WIT generation), and we get it because the journey vision needs untrusted-tool support (Loom-published fix templates from arbitrary peer Hosts).

**What would force a revisit:** Wasmtime maturity regressions; or evidence that WIT-bindgen overhead in the hot path (every WIT call) is unacceptable for the Spirit classes we ship.

### ADR-005 — Provider abstraction ships two adapters (Rust + AI-SDK)

**Decision:** First-party adapters for `rig-core` (Rust Spirits) and `@ai-sdk/provider` (TS Spirits). Other languages add their own.

**Alternatives:** One language only (forces all Spirits to that language); a bespoke provider trait (re-invents what rig-core / ai-sdk already do); pick a single SDK across both (hard to keep current with both ecosystems).

**Rationale:** The survey makes the trade-offs visible — opencode's 18+ providers via ai-sdk is the most complete TS surface; ironclaw's rig-core is the cleanest Rust surface. Either can lag a release behind upstream model launches; that's the cost of federation. **Two adapters maximize Spirit-language freedom** at the cost of two release cadences. Acceptable.

**What would force a revisit:** rig-core or ai-sdk going unmaintained; or a sufficiently dominant provider releasing a new feature that lands in one but not the other.

### ADR-006 — Loom is user-space, not kernel

**Decision:** Loom (collective KB, pattern library, cross-incident correlation) runs as a separate user-space service, not as a kernel module.

**Alternatives:** Embed Loom in the kernel (faster, simpler deployment; loses isolation, makes Loom upgrades coupled to kernel upgrades); make Loom a Spirit class (intriguing but circular — the curator of the collective shouldn't be subject to the same lifecycle constraints as the things it curates).

**Rationale:** Per I9, the kernel **stores no state and learns no patterns.** Loom is the user's data; the kernel mediates access. This separation keeps kernel upgrades non-disruptive to accumulated knowledge, lets enterprises swap Loom for an internal equivalent (Confluence-backed, S3-backed, vendor-backed) without forking the kernel, and makes "the kernel itself is auditable / replaceable" a real claim.

**What would force a revisit:** Discovery that Loom API latency on the kernel's hot path is unacceptable (would push toward embedded Loom); or a regulatory environment where the user wants kernel-level provenance over the collective KB.

---

## 13. Phased Roadmap

This is rough; treat it as a sequence, not a calendar.

| Phase | Scope | Validation milestone |
|---|---|---|
| **v0.1 — Bootstrap** | Kernel skeleton (Scheduler + Memory + Capability Registry + IAC mailbox), Spirit ABI v0.1, one reference Spirit (Architect, in-process Rust), one provider adapter (rig-core), local SQLite persistence, T0/T1 sandbox only. **No A2A. No Loom. No subprocess Spirits.** | The Architect Spirit can drive a real coding task on a local repo end-to-end with approval prompts. |
| **v0.5 — The first realistic single-user Host** | Add Butler + Researcher + Observer. Add T2/T3 sandbox. Add subprocess Spirits with the wire protocol. Add the Approval Manager prompt UX. Add the Transparency Log. **Still no A2A. No Loom.** | A single user has a working Butler that surfaces calendar items pre-meeting; a Researcher that can run a `bmad-technical-research` workflow; an Observer that shows live what's happening. |
| **v1.0 — Team-ready** | Add A2A peer mesh with mTLS + consent gates. Add the kernel-rendered notification surface. Ship six reference Spirits. T4 WASM tool sandbox. | Journey 10 Acts 1–7 are reproducible end-to-end with 8 Hosts on a real team. |
| **v1.5 — Diagnostic-architect** | Add the Diagnostic Engineer Spirit class with its asymmetric capability gates. Add post-deploy feedback IAC topic. Add Loom-lite (single-instance Postgres-backed pattern library). | Journey 11 (Mira + Nash) is reproducible. |
| **v2.0 — Enterprise & Cortex** | Enterprise Spirit with PDP integration. Multi-instance Loom with cross-region replication. Sentinel-validated canary auto-rollback. Pre-deployment scanning. SIEM telemetry export. | Journey 12 (Cortex) is reproducible at small scale (3-region pilot). |
| **v2.x+ — Hardening** | Kernel snapshot/restore. Cross-Host Spirit migration. Performance work. Spirit class catalog beyond the original six. | The architecture is no longer the bottleneck. |

Two principles guide phasing:
- **Each phase has a single observable validation milestone.** "We have v0.5" means the milestone is met, not that the to-do list is empty.
- **No phase ships without ADR review.** If a phase forces a revisit of any ADR-001..006, the phase boundary moves.

---

## 14. Open Questions

These are the genuine "I'm not sure" items. They are **not** blockers for v0.1 — they are signals for where the design will need to learn.

1. **Spirit hot-swap semantics for in-flight LLM streams.** Mid-stream, the predecessor's `on_swap_out` fires. What happens to the partial response? Drop it (waste a half-completion of Opus, but keep semantics simple)? Hand it to the successor as a `partial_response` input (clean but every successor must know what to do)? Stash it in `private` memory for later retrieval (easy but never actually used)? **Lean: drop it, log it, charge the user, keep semantics simple.** Revisit if cost data says the dropped completions are material.

2. **Approval prompt fatigue.** Every survey project has it. We have the substrate (`prompt_with_diff`, persistent allow, posture presets). What we don't have yet is **the heuristic** for "this looks like the same kind of thing the user has approved before, batch it." Possible answers: per-(Spirit, capability, target-fingerprint) cached decisions; an LLM-mediated batcher (codex's "guardian review path"); plain-English summary of "the next 10 things the agent wants to do" as a single approval. **Probably need real usage data to pick.**

3. **A2A trust establishment.** TOFU + mTLS is the v1.0 plan. For Journey 12's 28-Host mesh, this becomes operationally painful. We probably need a Cortex-scale **org-internal CA + per-Host certificate** flow, but that's enterprise infrastructure work, not architecture. **Defer to v2.0.**

4. **Spirit class portability across kernels.** v1.0 Spirit ABI is stable within a major; what about a v2.0 kernel running a v1.0 Spirit? **Lean: kernel must support N-1 ABI.** What about v0.5 Spirits on v1.0 kernels? Probably not; the v0.5→v1.0 jump is breaking by design. **Document the policy when v1.0 ships.**

5. **Loom contention.** When 14 Sentinels are simultaneously fetching the latest detection patterns and 10 Artisans are simultaneously publishing fix templates, Loom is a hot service. Pgvector at scale, A2A broadcast fan-out, cross-region replication — all solvable, none zero-effort. **The Cortex deployment milestone in v2.0 will reveal the right partitioning strategy.**

6. **Where does the Researcher's "novel hypothesis" mode live operationally?** A Researcher Spirit running on a corporate Host with `Enterprise` constraints needs to know what's allowed for hypothesis generation (data-residency for the source material; provider selection for the cogitation; collective tier writes for the conjectures). Probably resolved by the Enterprise Spirit's PDP wrapping. **Mark as PDP-integration test in v2.0.**

7. **The "Cortex blocks a deployment" UX.** Journey 12 Act 6 has Loom block a deploy and offer "Apply fix automatically" / "Override" / "Assign to Artisan." That's three Spirit interactions plus a CI integration. **Workable, but worth a UX prototype before v2.0 commits.**

8. **Kernel-managed notification rendering across surfaces.** TUI, editor (ACP), browser, mobile push. For v1.0 we ship TUI + ACP + native desktop. Editor banners can lean on ACP's existing diff-display. Mobile push (Viktor approves from his phone) needs a phone client; **deferred to v2.0**.

9. **Prompt-injection defense at the tool-output boundary.** ironclaw's `LeakDetector`/`Sanitizer` is the closest prior art. The kernel can host a generic post-tool-output filter, but the **content** of the filter (what's a leak? what's instruction injection?) is data, not code. Ship a default rule pack; let Spirits add to it. **Won't be perfect; aim for "raises the bar."**

10. **What's the smallest viable Loom?** v1.5 wants Loom-lite. Postgres + pgvector + a single MCP server is enough. But v2.0 wants cross-region. The shape of the Loom data model needs to be sharding-friendly from day one even if v1.5 doesn't use it. **One serious schema review before v1.5 ships.**

---

## Appendix A — Cohort prior-art map

For each major design decision, here's the project we mined it from. Useful when reviewing.

| MAOS feature | Survey project | Why we picked it |
|---|---|---|
| In-process Spirit mailbox | codex `core/src/agent/{registry,mailbox}.rs` | Cleanest actor model in the cohort; supervised, depth-limited |
| WASM/WIT tool capability | ironclaw `wit/tool.wit`, Wasmtime fuel limits | Only true capability sandbox; secret-exists never reads |
| Approval class taxonomy (6 classes) | openclaw `src/acp/approval-classifier.ts` | Most expressive taxonomy; battle-tested |
| 7-state ToolCall FSM | rustain `src/domain/services/tool_scheduler.rs` | Cancellation as first-class state |
| Sandbox primitives (Landlock+seccomp / Seatbelt / Win) | codex `codex-rs/sandboxing/` + `linux-sandbox/` | Only project shipping all three OS-native |
| ACP server NDJSON stdio | openclaw `src/acp/server.ts`, opencode `acp/agent.ts`, hermes `acp_adapter/server.py` | Convergent; ratified by Zed |
| MCP all-three transports (stdio / SSE / Streamable HTTP) | opencode `mcp/index.ts:3-5` | Future-compatible (Nov-2025 spec update) |
| A2A peer mesh with consent | gemini-cli `@a2a-js/sdk` + Journey 10 design | Only project wiring A2A; +Lunarpulse's transparency rules |
| Headless server + thin TUI | opencode | Clean split; mDNS optional discovery |
| Sub-Spirit `delegate_task` with depth cap | hermes-agent `tools/delegate_tool.py` | Recursion safety pattern |
| `*.md` memory file convention | universal in cohort | Table stakes |
| Loom curation pattern | openclaw + ironclaw vector workspaces, generalized | The collective tier is the user's, not the kernel's |
| Hot-swap with token rebinding | codex's `SpawnAgentForkMode` lifted to whole-Spirit | Preserves in-flight context across role changes |
| Streamable HTTP for A2A | mirror of MCP November 2025 update | Reuse the same TLS frontend for ACP-remote, MCP-remote, A2A |
| Provider hot-swap via `ArcSwap` | rustain `src/infrastructure/startup.rs:117-119` | Live provider replacement is non-trivial |
| Hexagonal `domain/adapters/infrastructure` for the kernel layout | rustain `CLAUDE.md:22-52` | Survives kernel evolution |
| Approval Decision Log + Transparency Log split | openclaw audit suite + Journey 10 design | Two distinct audit needs |
| `cargo-deny`-style dep policy + SBOM/provenance | openclaw `docker-release.yml` + ironclaw `deny.toml` | Supply chain hygiene from day one |
| Daily-rotated journal for crash recovery | rustain `rustain.log.YYYY-MM-DD` | Cheap durability |
| Posture preset hierarchy (presets + custom) | gemini-cli `ApprovalMode` enum + openclaw classifier | Composable |
| In-kernel notification rendering (TUI + ACP + native + push) | new — no precedent in survey | Required by Journey 10 transparency rules |

---

## Appendix B — Glossary diff vs. the survey

Where MAOS terms diverge from common cohort usage. (Useful for newcomers.)

| MAOS term | Cohort common term | Why we renamed |
|---|---|---|
| **Spirit** | "agent", "subagent", "agent class" | "Agent" is overloaded (process? persona? class?). "Spirit" disambiguates the *role + cognitive profile + posture* abstraction from a running process. |
| **Host** | not standardized (codex single binary, opencode server-plus-TUI, ironclaw multi-binary) | We need one word for "an OS process running the kernel" |
| **Posture** | "approval mode", "permission preset" | Posture is broader — it's the autonomy stance, of which approval policy is one component |
| **Capability Token** | not standardized; ad-hoc per project | Bringing OS-style explicit token semantics |
| **Loom** | "collective KB", "pattern library", "shared memory" | Direct lift from Journey 12 — it's already named |
| **IAC frame** | "message", "event", "RPC" | Frame is closer to wire-format reality; doesn't conflate with LLM "messages" |

---

## Appendix C — What this document deliberately is NOT

To save reviewer time later:

- **Not a UI spec.** TUI/editor/mobile shells are application-level work. The kernel's notification primitives are documented; the visual designs are not.
- **Not a benchmark plan.** Performance targets are out of scope until v0.5 ships and we have real numbers to optimize against.
- **Not a marketing document.** No tagline. No one-pager. Those are downstream.
- **Not a project plan.** §13's roadmap is sequence + validation milestones, not Gantt-able tasks.
- **Not a security audit.** §8 is a threat model and mitigation summary. A real audit happens before v1.0 ships.
- **Not the final answer on names.** "MAOS", "Spirit", "Loom", "Host", "Posture" — all are working names. Renaming is cheap before v0.1.

---

*Winston signing off. Architecture is the practice of arranging trade-offs so future-you can change your mind without burning everything down. Six ADRs, 10 invariants, 14 open questions. That's a healthy ratio for a substrate this ambitious.*
