---
title: 'MAOS — Modular Agentic Operating System Architecture'
subtitle: 'Kernel design, Spirit ABI, reference Spirits, deployment topologies'
date: '2026-05-06'
author: 'Winston (System Architect)'
status: 'Vision-locked, pre-implementation'
language: 'English (en-US)'
companion_artifacts:
  - 'prd.md (the requirements source)'
  - 'maos-design-report.md (the conceptual companion)'
  - 'spirit-development-and-sharing.md (Spirit author guide)'
  - 'maos-kernel-implementation-guide.md (kernel implementer guide)'
  - 'industrial_agents.md (user journeys)'
---

# MAOS Architecture

> **MAOS kernel and sub-modules are the grand theater. Spirits are actors in the play held in the MAOS grand theater. The user is the director of the play.** The kernel — invariant, auditable, replaceable — provides the stage, lighting, sound, audit-trail, and exits. Spirits perform within manifest-declared roles. The director runs the play autonomously while at dinner, or steps in scene-by-scene to direct in detail. The kernel's job is to make every performance safe and legible without authoring any of it.

## 0. Executive Summary

MAOS is an open-source kernel substrate for agentic computing — infrastructure that hosts specialized AI agents (called *Spirits*) on a stable contract called the Spirit ABI. The kernel is invariant; Spirits are hot-swappable modules loaded against the ABI. The same primitives compose, by configuration alone, into single-user laptop deployments, founder-loop multi-Spirit overnight workflows, and diagnostic-architect production pairs — without architectural rewrite at any tier transition.

The substrate replaces vendor-promise trust with **kernel-invariant trust**: every external call mediated by capability token, every IAC interaction logged before delivery, every approval auditable, every lifecycle journaled, sandbox profiles enforced, manifest-declared policies non-bypassable. Fourteen kernel invariants and forty Architecture Decision Records commit the design to a buildable shape. The kernel itself stores no patterns, no secrets, no learned behaviors — it is mediator and supervisor, never knowledge accumulator.

A MAOS Host is a single OS process running the kernel. The kernel exposes **five services** with explicit trust boundaries (Spirit Scheduler, Memory Manager, Security Manager, IAC Bus, Capability Registry) plus **two internal kernel modules** (I/O Subsystem and Telemetry Stream — described in §4.4 and §4.7; collapsed to kernel modules at v0.1, may extract to services at v0.5+ when their respective stream-processing and provider-rate-limit stories justify a separate task pool) and one contract (the Spirit ABI). At any moment, the Host runs N Spirits scheduled cooperatively against shared resource budgets, with subprocess-form Spirits additionally bounded by OS-level cgroups v2 / `setrlimit` / Job Object ceilings. A Spirit is a manifest plus cognitive profile plus memory scope plus posture, loaded from disk or pulled from the Spirit registry. The kernel never introspects what a Spirit is "thinking"; it dispatches messages, supervises lifecycle, and enforces capability and approval policy.

Two Spirit forms are supported: **rust-inproc** (compiled into the kernel binary; factory-default for reference Spirits) and **subprocess** (any-language binary speaking JSON-RPC + CBOR over stdio; the third-party-shippable form). Cross-Host communication uses A2A over mTLS+TOFU between two pre-paired Hosts (the diagnostic-architect bilateral pattern). Same-Host communication uses the kernel-internal IAC mailbox. Editor integration uses ACP. Tool invocation uses MCP. The substrate invents no new wire protocols.

The substrate ships in six phases: v0.1 Foundational (kernel skeleton + placeholder Spirit), v0.3 Butler (anticipatory single-Spirit), v0.5 Researcher + Observer (exploratory single-Spirit + first opt-in distillation), v0.9 Founder Loop wedge demo (multi-Spirit Orchestrator+Worker on a single Host with A2A loopback), v1.0 Team-pair-ready (Spirit registry with Ed25519 signing, third-party Spirit authoring, hermes-tenant claim cashed), v1.5 Diagnostic-Architect bilateral (Mira on prod-edge + Nash in dev-environment, asymmetric postures, Loom-lite single-instance pattern library). The terminal milestone is the J4 Mira-Nash 90-minute incident-to-fix loop reproducible at bilateral 2-host scale.

The substrate-positioning claim — *MAOS at v1.0 can host a hermes-class Spirit as a tenant, with the audit, revocation, and substrate-uninstall primitives that hermes-as-application cannot itself provide* — cashes at v1.0 through capability tokens, the Transparency Log, the Approval Decision Log, the Spirit registry with three trust tiers, the ComplianceClaim envelope, the cross-Spirit memory isolation corpus, and the externally-verifiable uninstall receipt.

## 0.4 Document Conventions

Two conventions govern the document's internal structure. Both are mechanical and cheap to enforce.

**Convention 1 — Single sub-section numbering.** A section containing exactly one numbered subsection numbers it `.1` (not `.2`, `.4`, or any other index). If a future revision introduces additional siblings, the existing `.1` is preserved and new ones extend the sequence (`.2`, `.3`, …). Never use a non-`.1` index for a solitary child — it implies missing siblings the reader will hunt for and not find. **Bold-prose paragraph blocks within the same section are *not* counted as siblings for this rule** — they are informal structure (sticky notes within the chapter, not numbered sections).

**Convention 3 — Promotion rule for bold-prose blocks.** If a bold-prose paragraph block is referenced from outside its parent section (cross-reference, TOC entry, or boundary manifest), it must be promoted to a numbered subsection. Once promoted, sibling-counting under Convention 1 applies normally. The teaching analogy: numbers are addresses, not emphasis. The moment something needs to be *addressable from elsewhere*, it earns a number; if it stays purely structural-within-its-parent-section, it stays bold-prose. Example: §7.2.1.a/b were promoted from bold-prose to real `#####` headings during remediation pass 3 because they became cross-referenced from §3.2.1 and §13.

**Convention 2 — Body↔Appendix dedup signposting.** When a normative specification lives in the body and its derivation, rationale, or worked examples live in an appendix, each side carries one explicit pointer to the other, using fixed prose patterns:

- **Body side (normative home), placed immediately after the table/values:** *"For the derivation of [these values / this specification], see Appendix [X.Y]."*
- **Appendix side (derivation home), as the opening sentence:** *"This appendix derives [the values / the specification] whose normative current-version specification appears in §[N.M] ([Section Title], [Table/Figure ID if applicable]). Reference §[N.M] for the values that govern conformance; this appendix explains how they were chosen [and how to re-derive them when [trigger condition]]."*

The two sentences are reciprocal: each names the other's location, identifies which side is normative, and tells the reader why they would visit the non-current location. Examples in this document: §9.5 ↔ App-F.5; §13.1 ↔ App-D.2.

## 0.5 Reading Map by Reader Type

Three audience paths through the document. Pick the row that matches what you are about to do; read those sections; ignore the rest unless cross-referenced.

| If you are a... | Start with | Then | Why |
|---|---|---|---|
| **Implementer (v0.1 sprint)** | §0.6 Foundational Commitments · §3 Vocab + Invariants | §4 Kernel Design · §5 Spirit ABI · §13 v0.1 row · §12.0 index (the binding-v0.1 cluster at the top of the table) | The v0.1 cluster in the §12.0 index table is sorted to the top; start there and stop when you hit the first `binding-v0.3+` row. |
| **Reviewer / Auditor** | §3.2 Invariants + cadence matrix (§3.2.1) | §8 Security Model · §10 Journey Traceability · §12 full ADR set · §14 Open Questions | The reviewer needs the full surface — what is committed, what is gated, what is acknowledged-as-uncertain. The §12.0 index table by Status is the triage tool. |
| **Future-architect / contributor** | §0.6 Foundational Commitments · §3.2.1 cadence matrix · **§10.7 Deferred journeys (future milestones)** | §4 Kernel Design · §12 full ADR set · App-D Terminal-Shape Sketches · App-E v0.9+ Compliance Roadmap · App-F Distillation Pattern Body | Future amendments need to know what shapes the architecture is biased toward but has not committed to. §10.7 names the two PRD journeys (J3 Marcus, Reza Cortex) deferred from v1.5 scope and the substrate-readiness analysis for adding them. Appendices hold non-binding terminal-shape sketches. |

The §1 reader-by-section index below is the secondary view: same content, oriented by goal rather than by role. Use whichever index helps.

## 0.6 Foundational Commitments

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

## 1. Reading Map

| Reader | Goal | Sections to read |
|---|---|---|
| Solo evaluator | "Can I install this and try it in 5 minutes?" | §0, §2, §10.1, §13 (v0.1) |
| Spirit author | "How do I write a Spirit and publish it?" | §5 (ABI), §6 (reference Spirits), §10.6 (Diego), §13 (v1.0) |
| Founder running the wedge demo | "How does the multi-Spirit founder loop work?" | §7 (IAC + A2A loopback), §10.4, §13 (v0.9) |
| Diagnostic-architect operator | "How does the 2-host diagnostic-architect pair work?" | §7.2 (bilateral A2A), §10.5, §13 (v1.5) |
| Kernel implementer | "Build this kernel in Rust + Tokio" | §3 (vocab + invariants), §4 (kernel design), §12 (ADRs filtered to binding-v0.1) |
| Auditor | "Verify the substrate's claims" | §3.2 (invariants + cadence matrix §3.2.1), §8 (security model), §10 (journey traceability) |
| Skeptic | "Why this and not LangChain or AutoGen?" | §0 (substrate position), §0.6 (commitments), §3 (invariants), §6 (reference Spirits) |

## 2. Architecture at a Glance

### 2.1 The two-paragraph version

A **MAOS Host** is a single OS process running the MAOS kernel. The kernel exposes five services + two internal kernel modules and one contract. Spirits run as **subprocess-form binaries** at v0.1 (any language with a Spirit Wire Protocol implementation; the form third-party authors ship from day one). In-process Rust Spirits are gated by §13's measurement harness, not shipped at v0.1 (ADR-002). The kernel arbitrates nothing about Spirit cognition — it dispatches IAC frames, supervises lifecycle, mediates capability invocations, and journals every transition. Spirit-internal reasoning, distillation, plan revision, and posture inference are Spirit-side concerns by design.

Same-Host Spirits speak through the IAC mailbox. Different-Host Spirits speak A2A over mTLS+TOFU, in a bilateral 2-host pre-pairing pattern (used by the Diagnostic Engineer + Senior Architect operational pair at v1.5). Tools live behind the Capability Registry, which mediates every tool invocation through the Security Manager and the Approval Manager. The Memory Manager exposes three namespaces: per-Spirit private, shared (this Host), and collective (Loom-lite, a single Postgres+pgvector instance accessed via MCP-Streamable-HTTP). The Telemetry Stream is the perceptual organ; the Observer Spirit class is its canonical consumer.

### 2.2 The diagram

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

### 2.3 The substrate's load-bearing claims

1. **Kernel/Spirit separation is enforced, not advisory.** Spirits never share a process address space without going through the IAC bus, never touch the filesystem outside the Memory Manager's namespaces, and never spawn tools outside the Capability Registry. This makes hot-swap-without-dropping-in-flight-tool-calls achievable as a kernel guarantee, not a Spirit-author convention.

2. **The kernel itself learns nothing.** Patterns, ADRs, fix templates, regression tests, dialectical updates — all live in user-space (Loom-lite). The kernel only enables propagation and audits the propagation. What gets propagated is the user's data, governed by the user's policy.

3. **Human transparency is a kernel invariant.** "No invisible actions, no puppeting, no asymmetric knowledge" is enforced by the Transparency Log and the Approval Manager, before any Spirit gets to author behavior. A Spirit cannot bypass this; the kernel will not deliver a frame the Transparency Log refused to record.

4. **Generality designed in.** A Spirit's binary contains lifecycle hooks, decision logic, and a system prompt — and nothing else. HTTP libraries, LLM provider SDKs, MCP clients, sandbox runtimes are kernel-provided adapters. Polyglot Spirit ecosystems become natural; uniform audit boundaries are a side effect.

5. **Epistemic halt as a Layer-1 capability.** When a Spirit's evidence is insufficient or contradictory, the kernel exposes a structured halt; the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. Hallucination becomes a user-mediated, audit-trailed event, not a silent regression.

## 3. Vocabulary & Invariants

### 3.1 Vocabulary

Definitions every section below assumes. Read once, then nothing in this document is ambiguous.

| Term | Definition |
|---|---|
| **Spirit** | A role + cognitive profile + memory scope + posture, loaded against the Spirit ABI. The hot-swappable unit. *Not* synonymous with "agent" (which is overloaded — process? class? persona?). |
| **Host** | A single OS process running the MAOS kernel. One machine may run one Host. Two pre-paired Hosts form the bilateral diagnostic-architect topology. |
| **Spirit ABI** | The trait + manifest + wire protocol contract between the kernel and Spirits. Stable across kernel minor versions within a major. |
| **Spirit Manifest** | The declarative file (TOML) describing a Spirit class: identity, role, model, memory scope, capability requests, posture, lifecycle hooks, hot-swap schema, halt-protocol compatibility, intent-promotion set, migration sources. |
| **Capability** | A typed tool surface — `bash.exec`, `fs.read`, `mcp.call(server, tool)`, `provider.complete(model)`, `a2a.send(peer)`, `git.commit`. Mediated by the Capability Registry. |
| **Capability Token** | A short-lived, kernel-issued ticket authorizing a Spirit to invoke a specific Capability with specific arguments under a specific posture. Bound to (Spirit-PID + boot-nonce + expiry). |
| **Posture** | A Spirit's autonomy stance — `cautious` (every action prompts), `assistive` (writes prompt, reads silent), `autonomous-with-halt` (proceed unless epistemic gap detected), `autonomous` (rare; explicit user grant). Posture is mutable at runtime; class is not. |
| **IAC Frame** | A typed message between Spirits — `task.assign`, `task.complete`, `decision.*`, `epistemic.halt`, `telemetry.event`, `consent.request`, etc. Logged before delivery. |
| **Loom-lite** | The collective memory tier — single-instance Postgres+pgvector exposed as an MCP-Streamable-HTTP server. Curates ADR-pattern libraries, fix templates, regression-test references for the diagnostic-architect bilateral pattern. User-space, replaceable. |
| **Transparency Log** | The append-only kernel-managed SQLite log of every IAC frame, every capability invocation, every lifecycle transition. Per-Host. The personal audit trail. |
| **Approval Decision Log** | A separate kernel-managed SQLite table recording every approval prompt's `(actor, target, capability, intent, decision, reasoning_if_any)`. Distinct from the Transparency Log. |
| **Distillation** | The Spirit-side compression of raw IAC payloads into decision-relevant digests. The kernel provides primitives (Transparency Log + log.recall + the I11/I12/I13 audit-chain enforcement); Spirits compose the pattern. |

### 3.2 Invariants

Fourteen non-negotiable runtime guarantees. The kernel enforces each structurally; violations are typed errors, not soft warnings. These commit the substrate's contract — what every Spirit author, every operator, and every auditor can depend on regardless of deployment topology.

| # | Invariant | Why it exists | Enforcement point |
|---|---|---|---|
| **I1** | Spirits cannot bypass the Capability Registry. Every tool, network call, file op, sub-Spirit spawn goes through it. | Without this, sandboxing, approvals, and audit are decorative. | Kernel — Capability Registry is the *only* surface returned to Spirits at load time. |
| **I2** | Every IAC interaction is logged before delivery. No ACK/NACK has ever been sent without an entry in the Transparency Log. | The "no invisible actions" rule. Without this, peer trust collapses. | Kernel — IAC Bus writes log before it writes mailbox. |
| **I3** | Auto-responses are always marked `[auto-sent]` on both sides. | The "no puppeting" rule. | Kernel — IAC Bus stamps every message with `origin: human-authored \| spirit-auto \| spirit-drafted-human-approved`. |
| **I4** | Every approval captures intent, not just decision. `(actor, target, capability, intent, decision, reasoning_if_any)` lands in the Approval Decision Log. | Audit trail must answer "why did the user approve this?", not just "did they?". | Kernel — Approval Manager writes the structured record at every prompt resolution. |
| **I5** | Memory scopes are kernel-enforced. A Spirit cannot read another Spirit's private memory or write outside its declared namespace. | Multi-Spirit deployments depend on this; without it, isolation is a Spirit-author convention and breaks under one bug. | Kernel — Memory Manager namespace check on every read/write. |
| **I6** | Hot-swap preserves Capability Tokens for in-flight tool calls; the new Spirit inherits the token, not the call. In-flight A2A frames at the predecessor are inherited by the successor under a drain-barrier (not dropped); in-flight distillation steps restart at the successor with the same `digest_refs` set; user-input queues live in the Spirit's snapshot and survive swap; the state-transfer wire format is CBOR + per-Spirit-class schema; the kernel rejects swaps with incompatible schema versions. | The diagnostic-architect handoff requires Mira's escalation context to survive Nash's swap-in. The founder loop requires the Orchestrator's epic state to survive kernel restart. | Kernel — Token Lifecycle Manager + Hot-Swap Coordinator validate state-transfer schema compatibility before successor activation. |
| **I7** | Telemetry is broadcast; subscription is per-Spirit. Pre-halt scalar trajectory observable via the `scalar.tap` stream so Observer Spirits witness the runup, not just the alarm. | The Observer's job is "what's happening"; without scalar-tap visibility, Observer can describe halts but not the drift that produced them. | Kernel — Telemetry Stream broadcast topics + dedicated `scalar.tap` channel from the Capability Registry's tagged-scalar slot. |
| **I8** | Cross-Host A2A interactions require explicit consent at both ends, scoped to the typed intent of the message (not just the channel). | Channel-consent does not imply transaction-consent — an action under intent `consult` cannot be silently re-purposed under intent `delegate`. Closes the confused-deputy class of attacks. | Kernel — A2A Gateway enforces sender-side policy AND receiver-side acceptance with intent-class allowlist before delivery. |
| **I9** | The kernel itself stores no secrets and learns no patterns. **Caching is structural** (key→value, bounded TTL, no aggregation across keys, no parameter drift) and is permitted within `{Journal, TransparencyLog, CapabilityRegistry::tokens}` only. **Learning is statistical** (parameters that drift from observation distribution, model weights, frequency tables, recency-weighted scores) and is forbidden in any kernel-core crate. | Auditability. The kernel is replaceable; the user's data is not. The boundary between caching and learning makes I9 falsifiable, not a slogan. | Kernel — secrets pass through to OS keyring; pattern data lives in user-space (Loom-lite); structural-state lint blocks new persistent fields outside the three permitted holders. |
| **I10** | Every Spirit lifecycle transition is journaled; crash recovery rehydrates from the journal. Crash detection ≤2s; `task.orphaned` IAC frame ≤5s with exit-cause recorded. | Reliability. Every operational gap is recoverable from the journal; nothing crashes silently. | Kernel — Journal at the Spirit Scheduler + supervisor monitors child-process exit. |
| **I11** | Persisted digests reference their raw source frames. Every payload tagged `kind: digest` written to private/shared/collective memory carries non-empty `source_log_ref: [frame_id, ...]` (transitively flattened to original raw frames, not intermediate digests) and `distillation_depth: N` (raw=0). **Segment-level granularity is the default contractual unit** — `source_log_ref` references a frame range covering the segment of raw evidence the digest summarizes. **Write-level audit (per-frame `source_log_ref`) is opt-in** for forensic Spirits via manifest declaration, gated behind a `forensic-audit` capability the operator must grant. Kernel rejects malformed writes with `EDigestAuditChainMissing`. | Distillation is a substrate-level pattern. Without an audit chain back to raw, the Transparency Log becomes ceremonial. Segment granularity keeps the audit path through 10K-writes/sec workloads without saturating fsync cadence. | Kernel — Capability Registry validates `source_log_ref` and `distillation_depth` on every digest-tagged write at the declared granularity. Digest content is NOT validated (preserves I9). |
| **I12** | Every byte in Spirit context is traceable to a `log.recall` or `event/inbound + shadow-recall` entry. When a Spirit emits a `decision.*` frame, the kernel attaches `working_memory_digest_refs: [frame_id, ...]` populated from the Spirit's declared in-context digests AND from any raw frames delivered via inbound events. | Closes "the digest hid the critical finding → the agent never recalled raw → audit shows raw existed but the agent never reasoned over it." Without I12, audit can prove what raw + what digest, but not what the agent actually saw at decision time. | Kernel — Capability Registry tracks per-Spirit in-context digest set + inbound shadow-recall records; attaches refs on emit of any `decision.*` frame. Frame_ids only — no content inspection. |
| **I13** | Digests carry intent provenance. **The kernel computes the recall-union from `log.recall` tracking** — every digest derived from input frames whose `intent` field is set carries `intent_lineage: [intent_class, ...]` synthesized by the kernel from the union of intent classes of all input frames it summarizes. A consumer that operates under intent `Y` rejects digests whose `intent_lineage` is not contained in `allowed-promotion-set(Y)` (typed error `EIntentPromotionDenied`). | Closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. Without I13, the typed-intent consent leaks across the distillation boundary. Kernel-computed (not Spirit-self-reported) closes the asymmetric-enforcement gap. | Kernel — Capability Registry computes union from input frame_ids on digest write; consumer-side admission verifies set-containment per manifest's `allowed-promotion-set`. Promotion sets are flat allow-lists declared per consuming Spirit class; transitive promotion is not implied. |
| **I14** | Hot-swap preserves halt continuity. When a Spirit with a non-empty `halt_set` is hot-swapped, either every halt is drained (resolved before swap) OR every halt is migrated to the successor with full resolution-path state, AND the successor's manifest declares `halt_protocol_compatibility = N` (matching the predecessor's halt-protocol version). Halt-protocol versioning per Spirit class lives in `halt-registry.toml`; the kernel checks at swap time. Zero dropped halts; zero successor-confusion events. | Closes the hot-swap × halt interaction. Without I14, an in-flight halt can silently disappear during swap, dropping the user's clarification request and breaking trust in the halt mechanism. | Kernel — Hot-Swap Coordinator checks `halt_set` before swap; if non-empty, requires either drain-completion or schema-compatible migration; rejects swap with `EHaltContinuityViolation` otherwise. |

These cannot change without a major-version bump.

### 3.2.1 Invariant Enforcement Cadence

The 14 invariants do not all enforce at v0.1. Each phase promotes a subset from "design aspiration" to "mechanically verified." This matrix tracks which invariants are enforced at which phase, and at what tier. Reviewers asking *"as of this release, which invariants does the kernel actually mechanically enforce?"* read this matrix.

**Enforcement tiers** (forward-only progression):
- `—` Not yet enforced. The invariant is design-aspirational at this phase; structural enforcement land later.
- `CI` Mechanically checked in CI (lint, schema diff, structural-state assertion). Static guard, not runtime.
- `runtime` Asserted at runtime by kernel code. Violations produce typed errors; kernel rejects the bad path.
- `fuzz` Adversarially validated by fuzz/red-team corpora. The surface has been beaten on.

**Transition rule.** A cell may only ever advance: `—` → `CI` → `runtime` → `fuzz`. Backward transitions (e.g., demoting a `runtime` invariant to `CI`) are architecture changes and require an ADR + the `invariant-lock` CI gate (ADR-037).

| Phase | I1 | I2 | I3 | I4 | I5 | I6 | I7 | I8 | I9 | I10 | I11 | I12 | I13 | I14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **v0.1** | runtime | runtime | CI | runtime | runtime | — | — | — | CI | runtime | — | — | — | — |
| **v0.3** | runtime | runtime | CI | runtime | runtime | runtime | — | — | CI | runtime | — | — | — | — |
| **v0.5** | runtime | runtime | CI | runtime | runtime | runtime | runtime | — | CI | runtime | runtime | runtime | runtime | — |
| **v0.9** | runtime | runtime | CI | runtime | runtime | runtime | runtime | runtime | CI | runtime | runtime | runtime | runtime | runtime |
| **v1.0** | fuzz | runtime | CI | runtime | runtime | runtime | runtime | fuzz | CI | runtime | fuzz | runtime | runtime | runtime |
| **v1.5** | fuzz | runtime | CI | runtime | runtime | runtime | runtime | fuzz | CI | runtime | fuzz | runtime | runtime | fuzz |

**Diff-by-row** (read as "what newly hardens at this phase"):
- **v0.3** newly enforces I6 at runtime (hot-swap state-transfer + token preservation operational).
- **v0.5** newly enforces I7 (Telemetry Stream + `scalar.tap`), I11 (digest audit-chain), I12 (decision-context refs), I13 (intent_lineage).
- **v0.9** newly enforces I8 at runtime (cross-Host typed-intent consent on A2A loopback) and I14 (halt continuity across hot-swap).
- **v1.0** promotes I1 (capability mediation), I8 (cross-Host consent), and I11 (digest audit-chain) from `runtime` to `fuzz` — these are the three surfaces where the §8.1 80-scenario red-team corpus, the cross-Host adversarial corpus (Sec-14b), and the §8.5 CCAC corpus (per App-E) probe the invariant directly.
- **v1.5** promotes I14 (halt continuity across hot-swap) from `runtime` to `fuzz` — the bilateral 2-host topology and the planned **J4-extended halt-continuity corpus** (v1.5 deliverable, see §13) expose halt-continuity to adversarial agent-handoff timing that single-host enforcement cannot probe. The J4-extended corpus is a fuzz-tier seed (≥30 hot-swap-during-incident scenarios with mutation infrastructure built around them, not 150 hand-labeled scenarios — fuzz needs adversarial breadth, not statistical N). v1.5 is also where existing enforcement gets its terminal validation surface (J4 50-scenario incident corpus, Loom-lite chaos test, §7.2.1 mTLS rotation chaos test).

I3 stays at `CI` indefinitely because it is a structural lint over IAC frame origin stamps — there is no `runtime` upgrade path that adds value. I9 stays at `CI` for the same reason — the structural-state lint is the load-bearing check; there is no runtime guard against "did the kernel learn a pattern" that would not be redundant.

## 4. Kernel Design

### 4.0 Kernel Internal Architecture

Before the kernel decomposition individually, this section commits how the kernel is organized: the architectural style, the layout of code, how subsystems connect, and the principles that prevent kernel state-creep over time.

**Component classification (terminology lock).** The kernel comprises **one supervisor (Spirit Scheduler), four supervised services (Security Manager, Memory Manager, IAC Bus, Capability Registry), and two internal modules (I/O Subsystem, Telemetry Stream).** The supervisor / supervised-service / module taxonomy is the operational classification per §4.0.8's four-property test (P1–P4); a supervised service satisfies all four properties, a module fails at least one, the supervisor satisfies P1/P2/P4 and is exempt from P3 (boundary manifest) because its boundary *is* the union of its children's boundaries.

**v0.1 component-count rationale.** Two subsystems that earlier drafts modeled as separate services — I/O Subsystem (§4.4) and Telemetry Stream (§4.7) — collapse to **internal kernel modules** at v0.1 because their v0.1 surface (single Anthropic provider for I/O; pure-broadcast no-state for Telemetry) does not yet justify the separate-task-pool overhead a supervised service implies. **Security Manager remains a supervised service** even at v0.1 because its compilation boundary carries security invariants the type system enforces (capability-token signing-key isolation, audit log integrity, mTLS rotation injection point) — collapsing it would weaken those invariants to internal-API discipline. Extraction of I/O Subsystem and Telemetry Stream to full supervised services is a v0.5+ option, gated on real multi-provider and stream-processing demand.

References elsewhere in this document to "five services" should be read as "one supervisor + four supervised services" — the older shorthand predates the §4.0.8 formalization.

#### 4.0.1 Architectural style — hexagonal for static structure, actor model on the hot path

**Static structure: hexagonal (ports-and-adapters).** The kernel is structured as a domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring (concrete adapters for HTTP, stdio, mTLS, MCP, ACP, providers, persistence, secrets). This gives the kernel multi-adapter-per-port flexibility (swap SQLite for Postgres without touching domain logic), testability (every port has a mock adapter), and keeps the domain core small.

**Runtime hot path: actor model.** Each Spirit is an actor — mailbox-addressable, behavior-encapsulated, no shared mutable state with peers. This pattern gives four properties for free: backpressure via bounded mailboxes, no locks on the hot path (each actor owns its state), failure isolation via Tokio task supervision, and natural hot-swap (replace `behavior` while preserving `state` and `open_tokens`). The seven kernel services are *not* themselves actors — they are shared services that actors call into, with their own task pools.

The two styles do not conflict. Hexagonal owns the static dependency graph; actor model owns the runtime topology. A Spirit's `IacBus::send` call routes through the IAC Bus service (a Tokio task pool), which writes to the Transparency Log (a Persistence adapter), which fsyncs to SQLite (a domain-side WAL append). Every layer has a clean contract; nothing leaks across.

#### 4.0.2 Layout

```
maos/
├── crates/
│   ├── maos-domain/                    # v0.1 ✅  Pure types, invariants I1-I14, pure functions
│   ├── maos-spirit-abi/                # v0.1 ✅  Wire-stable types ONLY. #![no_std].
│   │   └── src/compliance.rs           #          ComplianceClaim schema (per §8.5, App-E).
│   │                                   #          Bumping = bumping ABI_VERSION.
│   ├── maos-kernel-core/               # v0.1 ✅  Five services + two internal modules.
│   │   ├── scheduler/                  #          Spirit Scheduler + journal + budget
│   │   ├── memory/                     #          Memory Manager + namespace enforcement
│   │   ├── security/                   #          Security Manager + sandbox + approval (compilation boundary)
│   │   ├── io/                         #          I/O module (HTTP, stdio, mTLS, ACP) — internal at v0.1
│   │   ├── iac/                        #          IAC Bus (mailbox, broadcast, retract)
│   │   ├── capability/                 #          Capability Registry (decomposed):
│   │   │   ├── cap-tokens/             #            Hot path: token issue/verify, lock-free
│   │   │   ├── cap-policy/             #            Consent rules + intent allowlist
│   │   │   ├── cap-audit/              #            Audit/lineage writer (slow path)
│   │   │   └── cap-quota/              #            Budget tracking + ContextPressure
│   │   ├── compliance/                 #          ComplianceClaim structural validator (~200 LOC, v0.1)
│   │   ├── pipeline/                   #          Emit pipeline (IACFrame + ComplianceClaim co-located)
│   │   ├── telemetry/                  #          Telemetry module + scalar.tap — internal at v0.1
│   │   └── hot_swap/                   #          Hot-Swap Coordinator
│   ├── maos-spirit-sdk/                # v0.1 ✅  Spirit-author helpers; #[spirit] proc-macro
│   ├── maos-spirit-hello/              # v0.1 ✅  Reference Spirit; validates SDK end-to-end
│   ├── maos-providers/                 # v0.1 ✅  Anthropic at v0.1; ≥3 providers in CI by v0.5
│   ├── maos-mcp/                       # v0.5    MCP client
│   ├── maos-acp/                       # v0.5    ACP server
│   ├── maos-a2a/                       # v0.9    Bilateral A2A peer module (loopback at v0.9, cross-Host at v1.0)
│   ├── maos-persistence/               # v0.1    SQLite at v0.1; Postgres+pgvector (Loom-lite) at v1.5
│   ├── maos-secrets/                   # v0.1    OS keyring adapter
│   ├── maos-compliance/                # v0.9 🔒  Semantic evaluator + N=600 corpus (App-E)
│   ├── maos-control/                   # v0.5    Control-plane HTTP API
│   ├── maos-cli/                       # v0.1    maosctl
│   └── maos-bin/                       # v0.1 ✅  Composition root
├── spirits/                            # Reference Spirit crates (in-process)
├── schemas/                            # JSON Schema + CBOR schemas
│   ├── trace-shape.schema.json
│   ├── halt-registry/<spirit-class>.toml
│   └── gateway-submodule.schema.json
├── docs/
└── fuzz/                               # Fuzz harnesses (manifest, wire, replay)
```

Dependencies point inward (adapter ring → kernel services → domain core), with the explicit exception of kernel services calling into Spirit ABI traits — that is the inversion of control that makes Spirits hot-swappable. The composition root in `maos-bin/main.rs` is the only place that knows about all crates.

#### 4.0.3 Service dependency map

| Service | Depends on (kernel side) | Used by (kernel side) |
|---|---|---|
| **Spirit Scheduler** | Capability Registry (token revocation on unload), Memory Manager (archive on swap), Persistence (journal) | Control plane (load/swap/unload commands) |
| **Memory Manager** | Persistence, Capability Registry (scope validation) | Spirit Scheduler (archive), all Spirits (memory.read/write) |
| **Security Manager** | Sandbox backends, Secrets adapter, Approval rendering | Capability Registry (sandbox profile lookup) |
| **I/O Subsystem** | Concrete transport adapters (HTTP, stdio, mTLS) | All inbound clients; outbound calls from Capability Registry |
| **IAC Bus** | Telemetry Stream (logging), Persistence (Transparency Log), Spirit Scheduler (mailbox addresses) | All Spirits (iac.send), control plane (broadcasting) |
| **Capability Registry** | Security Manager, I/O Subsystem, Memory Manager, Telemetry Stream | Every Spirit interaction with the world |
| **Telemetry Stream** | nothing (pure broadcast) | Spirit Scheduler, IAC Bus, Capability Registry, all Spirits (subscriptions) |

The Capability Registry is the busiest service — every external call funnels through it. It is decomposed into four sub-services (cap-tokens / cap-policy / cap-audit / cap-quota) so the hot path (token issue/verify) does not serialize on the audit/lineage path. The Telemetry Stream is the simplest — pure broadcast, no state, no I/O. It is the kernel's lung.

#### 4.0.4 Technology choices

| Concern | Choice | Why |
|---|---|---|
| Language | Rust + Tokio | Type-safe invariants, mature async runtime, zero-cost abstractions, no GC pauses on the hot path |
| In-process IPC | `tokio::sync::mpsc` + `tokio::sync::broadcast` | Bounded mailboxes, backpressure, codex precedent |
| Subprocess transport | LSP-style `Content-Length` framing over stdio with CBOR payloads | Boring, well-understood (LSP precedent), language-neutral, byte-stable across SDKs |
| Cross-Host transport | mTLS over TCP, JSON-RPC framing | Single transport for the bilateral case; well-understood TLS toolchain |
| Sandboxing | OS-native primitives (Landlock+seccomp on Linux, Seatbelt on macOS, restricted-token + Job Object on Windows) | Already production-grade; codex has shipped all three |
| Persistence | SQLite (per-Host Transparency Log + Approval Decision Log + Journal) + Postgres+pgvector (Loom-lite collective tier at v1.5) | SQLite for single-Host append-only audit; Postgres for the diagnostic-architect pair's shared pattern library |
| Secrets | OS keychain (Linux secret-service / macOS Keychain / Windows Credential Manager) | The kernel does not store secrets (I9) |
| Cryptography | Ed25519 for Spirit signing + signed export; mTLS via `rustls` | Boring, audited, FIPS-pluggable via provider trait |
| Hot-swap state-transfer | CBOR + per-Spirit-class versioned schema | Typed, compact, language-neutral, schema-evolved |

#### 4.0.5 Spirit-form abstraction

Two Spirit forms ship; both speak the same Spirit ABI through different runtime substrates:

| Form | Phase | Languages | Latency | Trust |
|---|---|---|---|---|
| `rust-inproc` | v0.1+ | Rust only | Function-pointer dispatch, nanoseconds | Implicit (compiled into kernel binary) |
| `subprocess` | v1.0+ | Any language with a Spirit Wire Protocol implementation | tens of microseconds round-trip | Explicit (Ed25519-signed; trust-tier-enforced; sandboxed) |

A Spirit's manifest declares which forms it ships in. Capability scopes are not portable across forms: a Spirit calling `std::process::Command` builds under `subprocess` and `rust-inproc` but not in any environment that forbids exec. The Spirit registry refuses incompatible builds at publish time.

#### 4.0.6 Why no kernel-resident memory store

The kernel itself stores no patterns, no Spirit memory beyond capability-token state, and no learned behaviors. Patterns, ADRs, fix templates, regression tests — the *Loom-curated collective knowledge* the diagnostic-architect bilateral pair maintains — live in user-space (Loom-lite, a Postgres+pgvector instance the operator deploys), not the kernel. The kernel only enables propagation. What gets propagated is the user's data, governed by the user's policy. This is Invariant I9 made concrete: the kernel is **mediator and supervisor**, not knowledge accumulator.

#### 4.0.7 What the Kernel Does NOT Compute

The kernel's value comes from what it deliberately refuses to do as much as from what it provides:

- **The kernel does NOT interpret tag semantics.** Tagged scalars and tagged frames carry meaning the kernel transports without reading. Variance, entropy, expected free energy, KL divergence, ensemble disagreement, calibration, similarity, derivatives, statistical tests, contradiction detection — all Spirit-side computations. The kernel performs universal arithmetic comparison only via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`).
- **The kernel does NOT author cognitive content.** Distillation, summarization, planning, reasoning, dialectical update, hypothesis generation, posture inference — all Spirit-side. The kernel provides storage, lineage, namespacing, and the Inference Port; cognitive work belongs to actors.
- **The kernel does NOT embed an orchestration policy.** Multi-Spirit coordination patterns (supervisor, peer, market, pipeline) are user-space Spirit patterns, not kernel features. The kernel routes typed-intent IAC frames neutrally; Orchestrator-class Spirits do the directing.
- **The kernel does NOT write skills, rank skills, or curate skills.** Skills are Spirit-author craft; admission is operator-mediated; the kernel hosts the registry mechanism only.
- **The kernel does NOT host collective knowledge directly.** Loom-lite is a user-space service running under MCP-Streamable-HTTP. The kernel mediates access; the merge strategy and curation policy belong to the operator.
- **The kernel does NOT own application-layer concerns.** Messaging gateways, UI presentation, narrative digest content, training-data generation — all Spirit-side. The kernel offers extension contracts; applications fill them.

These refusals are what keep the kernel small and replaceable.

#### 4.0.8 Service vs Internal Module — operational definition

The five-services-plus-two-internal-modules framing is not a stylistic distinction; it has testable consequences. A component is a **service** if and only if all four properties below hold; otherwise it is an **internal module** of a parent service.

| Property | Service | Internal module |
|---|---|---|
| **Crate boundary** | Separate Cargo crate under `crates/services/<name>/`; published `Cargo.toml` with own `[package]` section | Sub-module under a parent service crate (`crates/services/<parent>/src/<module>.rs` or `mod/`); shares parent's `Cargo.toml` |
| **Process boundary** | May run in its own OS process when the deployment topology requires (separate `tokio::main` or spawned binary; own `bin/` target). At v0.1 every service runs in the same kernel binary, but the service is *capable of* extraction without code change. | Always runs in the address space of its parent service; no independent binary |
| **IPC contract** | Inter-service calls go through the typed IAC bus (§7.1); contract defined in `crates/iac/proto/`; mockable for unit test | Intra-service calls are direct Rust function calls; no proto definition |
| **Failure domain** | Independently restartable by the supervisor (§4.1 Spirit Scheduler analog applies); a panic in this service does not take down peers | Crashes with parent service; supervisor restarts the parent, not the module |

**Reproduction test.** Given a candidate component X, an implementer answers the four yes/no questions against the codebase and gets a deterministic classification. Security Manager passes all four → service. The I/O Subsystem at v0.1 fails crate-boundary and process-boundary → internal module of `maos-kernel-core`. Telemetry Stream at v0.1 fails the same two → internal module.

**Boundary enforcement is mechanical, not type-system.** The four properties above are facts about the repository layout and Cargo manifests, not facts a Rust type can know — crate identity, bin-target presence, and supervisor restart policy are all external metadata that no `const` on a trait can encode. Enforcement lives in `xtask/src/check_service_boundary.rs`, run in CI as `cargo xtask check-service-boundary`.

The xtask asserts, for each entry in `SERVICES` (a const list in the xtask itself):

- **P1.** `crates/services/<name>/Cargo.toml` exists and declares `[lib]`.
- **P2.** `crates/services/<name>/src/bin/<name>.rs` exists OR Cargo.toml declares a `[[bin]]` target named `<name>`.
- **P3.** `crates/iac/proto/src/<name>.rs` exists and is `pub mod`-exported from `crates/iac/proto/src/lib.rs`.
- **P4.** `crates/services/<name>/src/main.rs` (or the bin target above) calls `std::process::exit` only via `iac_runtime::shutdown::exit_code(...)`, verified by `syn`-based AST scan rejecting bare `std::process::exit`.

Failure of any property fails CI. The xtask carries two const lists:

```rust
const SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"];
const SUPERVISOR: &str = "spirit-scheduler";
```

Adding a fifth supervised service requires adding it to `SUPERVISED_SERVICES` AND ensuring P1–P4 are satisfied in the codebase. Service-Boundary Manifests (e.g., §4.3.5 for Security Manager) **declare** the canonical filesystem locations the xtask checks; the **xtask** verifies those locations exist and the **test suite** verifies the running system honors the boundary. Three layers, distinct: spec declares (this document, §4.3.5), xtask verifies anchors exist (`cargo xtask check-service-boundary`), tests verify behavior (release gate per §13).

**Supervisor exception.** Exactly one component in the system — the Spirit Scheduler — is the composition root: the binary whose `main` instantiates and supervises the four supervised services. The supervisor satisfies P1, P2, and P4 but is exempt from P3 (boundary manifest in the standard shape) because its boundary *is* the union of its children's boundaries. The xtask verifier checks the four supervised services against P1–P4 and verifies the supervisor against P1, P2, P4 only. Any future component must declare itself either a supervised service (full P1–P4) or a module (no boundary, contained within a service); a second supervisor is a structural change requiring this section to be revisited.

**Telemetry-label divergence (intentional).** The §4.7.1 telemetry contract (`iac_rt_*` metrics) labels `service ∈ {security, memory, iac, capability, spirit_scheduler}` — five entries, the supervised four plus the supervisor. This is intentional: the xtask classification answers "what does this run as" (architecture); the telemetry label answers "who originated this RT" (operations). Spirit Scheduler does originate IAC RTs (supervisor-initiated capability checks, lifecycle frames) and must be observable by the same labels as everyone else.

```rust
// xtask/src/check_service_boundary.rs (skeleton)
const SERVICES: &[&str] = &["security", "memory", "iac", "capability"];

pub fn run(workspace_root: &Path) -> anyhow::Result<()> {
    for svc in SERVICES {
        check_p1_own_crate(workspace_root, svc)?;
        check_p2_own_bin(workspace_root, svc)?;
        check_p3_proto_module(workspace_root, svc)?;
        check_p4_supervised_exit(workspace_root, svc)?;
    }
    Ok(())
}
```

**v0.5+ extraction rule.** When extraction of an internal module to a service is proposed (e.g., I/O Subsystem becomes its own service when multi-provider rate-limiting demand justifies it), the change is mechanical: add the module's name to `SERVICES`, satisfy P1–P4 in the codebase, run `cargo xtask check-service-boundary`. The v0.5+ ADR for any extraction documents which properties flip and what the operational consequences are (independent restart policy, separate metric namespace, etc.).

The five services + two internal modules are detailed in §4.1–§4.7 below. **§4.1, §4.2, §4.3, §4.5, §4.6 describe services with their own task pools and explicit trust boundaries. §4.4 (I/O Subsystem) and §4.7 (Telemetry Stream) describe internal kernel modules** — they live inside `maos-kernel-core` rather than as separate services at v0.1. Read them in order — each builds on the previous.

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
- `migrate(SpiritId, target_host)` — bilateral A2A-mediated; serializes manifest + memory pages + token set; used by the diagnostic-architect pair handoff
- `snapshot(SpiritId) → SnapshotId` / `restore(SnapshotId)`
- `unload(SpiritId)` — graceful shutdown via lifecycle hooks (§5.3)

**Scheduling discipline (in-process):** Cooperative, priority-weighted, bounded by the Capability Registry's rate limits (so a runaway Spirit cannot starve peers via tool calls). LLM-bound Spirits yield naturally on streaming chunks; CPU-bound Spirits get a `tokio::task::yield_now` injection at sandbox boundaries. Cooperative-yield assumption holds inside a single Spirit's task pool only.

**OS-level CPU/memory budget enforcement:** subprocess-form Spirits run inside Linux cgroups v2 with declared `cpu.max` and `memory.max` ceilings — kernel sets these at spawn, enforced by the OS, not by Tokio cooperation. macOS uses POSIX `setrlimit(RLIMIT_CPU, RLIMIT_RSS)` per child; Windows uses Job Objects with `JOB_OBJECT_LIMIT_PROCESS_TIME` and `JOB_OBJECT_LIMIT_PROCESS_MEMORY`. Default ceilings declared in the `[resources]` table of the manifest; kernel applies the strictest-of (manifest, operator policy) at spawn. Across Spirit processes the OS, not the runtime, is the floor.

**Crash detection and recovery (I10).** The Scheduler supervises every subprocess Spirit. Crash detection ≤2s on SIGKILL; `task.orphaned` IAC frame emitted to in-flight task originators ≤5s with exit-cause journaled (signal, exit-code, stderr-tail). Hung-Spirit detection (alive but no progress IAC for >30s) emits `task.stalled` event within 60s. On crash mid-CBOR-snapshot-write, the supervisor's `JoinSet` returns `Err`; supervisor synchronously calls `cap_registry.revoke_all(spirit_id)`; journal records `HaltRecord{cause: Fault, in_flight_tokens: [...]}`; any half-written CBOR frame in the journal is marked `Torn` on replay and discarded. Replay rule: torn frame at tail = truncate; torn frame mid-log = fatal corruption requiring manual recovery.

**Trade-offs the Scheduler does NOT make:**
- It does not pick which Spirit handles a given user request. That is a user-space concern (the **Routing Spirit**, a default Butler-class instance, does it).
- It does not do auto-scaling, auto-replication, or HA. Those are deployment concerns, not kernel concerns.

### 4.2 Memory Manager

**Responsibility:** Provide three named memory tiers to every Spirit, enforce scope from the manifest, support hot-swap and migration.

**Three tiers:**

| Tier | Scope | Backed by | Lifetime | Use case |
|---|---|---|---|---|
| `private` | This Spirit instance only | `Arc<RwLock<HashMap<...>>>` per-Spirit, plus `fs.write` to per-Spirit-namespaced filesystem area | Spirit lifetime + episodic persistence if declared | Working memory, scratchpad, session state |
| `shared` | All Spirits on this Host (subject to `[memory.shared]` access list) | SQLite-backed key-value with namespace prefix per writer Spirit | Host lifetime | Cross-Spirit coordination on this Host (Orchestrator-Worker handoff payloads, founder-loop state) |
| `collective` (Loom-lite) | Both Hosts in a bilateral pair | Postgres+pgvector exposed via MCP-Streamable-HTTP | Loom domain lifetime | ADR-pattern library, fix templates, regression-test references for the diagnostic-architect bilateral pair |

**Namespace enforcement (I5):** every read/write goes through a kernel-mediated path. `mem.write(scope, key, value)` validates that the calling Spirit's manifest declares write access to `scope`; `mem.read(scope, key)` validates declared read access. Cross-Spirit reads on `shared` are explicit allow-list; cross-Spirit reads on `private` are forbidden by construction (no surface to read another Spirit's private namespace from outside).

**Principal Memory Namespace.** A typed namespace within the private tier — `principal:<principal_id>:<spirit-author-defined-schema>`. Writes to this namespace are tagged as principal-related data and inherit three kernel-mediated operations: **subject-access query** (DPO requests "show everything about principal X"), **right-to-be-forgotten** (operator command removes all principal-namespaced entries for a given subject), **redaction-on-export** (sealed-export scrubs principal-namespace entries unless explicit `--include-principal` flag). Schema is Spirit-author-declared; the kernel only knows that data tagged `principal:<id>:*` is subject to the three operations above.

**Hot-swap and migration:** the Memory Manager swaps memory scope along with Spirit class (I6). For `swap()`, private memory is preserved (the swapping-in Spirit inherits it via `on_swap_in`'s `predecessor_state` argument). For `migrate()`, private memory is serialized into the migration payload along with the manifest and the open token set; the receiving Host's Memory Manager rehydrates on `on_swap_in`. Shared memory is left in place (it belongs to the Host, not the Spirit). Collective memory (Loom-lite) is reachable from either Host in the bilateral pair without migration.

**The kernel does not interpret memory contents.** Schema is entirely Spirit-author-declared. The kernel only knows what scope a write targets, what `kind` tag (`raw`, `digest`, `principal:*`, ...) the payload carries, and — for digest-tagged writes — what `source_log_ref` and `distillation_depth` claim per I11.

### 4.3 Security Manager

**Responsibility:** Sandboxing, secret materialization, approval mediation, posture enforcement.

#### 4.3.1 Sandbox tiers

Four tiers, declared per Spirit in the manifest, enforced at process spawn:

| Tier | Profile | Use case |
|---|---|---|
| **T0** | No sandbox; full host privileges | Trusted local-tier Spirits only (operator-authored, `local` trust tier) |
| **T1** | Process isolation; UID separation; no special syscall filtering | Default for `org-internal` trust tier |
| **T2** | Linux: Landlock + seccomp-bpf with allow-listed syscalls + filesystem subtree restriction. macOS: Seatbelt with `.sbpl` profile. Windows: restricted token. Default for `public-untrusted` trust tier. | Default for third-party Spirits at `public-untrusted` |
| **T3** | T2 + container (Docker/Podman) | Spirits with broad capability surfaces (Researcher with web/arXiv/GitHub/citation-graph; Diagnostic Engineer with cross-environment telemetry queries) |

**Strictest-of-(manifest, trust-tier, operator-policy) floor.** The kernel applies the strictest sandbox tier from any of: the Spirit's manifest declaration, its trust tier, the operator's deployment policy. A `public-untrusted` Spirit declaring T0 in its manifest is forced to T2 by the trust-tier floor.

**Per-Spirit resource isolation.** Each subprocess-form Spirit runs under a resource cgroup (Linux cgroups v2; equivalent on macOS/Windows) with kernel-enforced caps on CPU, memory, file descriptors, and process count. Sandbox tiers cover the *security* boundary; resource cgroups cover the *resource* boundary. A runaway Spirit gets throttled, not the host.

#### 4.3.2 Secret materialization

The kernel itself stores nothing (I9). Secrets are materialized just-in-time from:
- OS keyring (Linux secret-service / macOS Keychain / Windows Credential Manager) — default
- Encrypted-file vault (`maos-secrets` with `encrypted-file` feature) — for headless operator deployments

Secrets pass through the Capability Registry to the calling adapter (e.g., `provider.complete` materializes the Anthropic API key just before the HTTPS request, redacts it from any log) and are never journaled in cleartext. **Pre-write secret-redaction filter at the Transparency Log boundary.** Frames passing through the IAC Bus are scanned for known secret patterns (API keys, capability tokens, mTLS private-key bytes) before being written to the log; any match is redacted with a typed marker (`<REDACTED:type=api_key,len=…,hash=…>`). Floor: 0 secrets in any logged frame across the bounded test populations (10⁴-case corpus per-commit, 10⁵-case quarterly audit, 1000-canary-secrets-per-month production canary system). Production canary leak detection halts the distillation pipeline until root-caused; discovery latency ≤24h p95.

#### 4.3.3 Approval class taxonomy

Six classes, with default policies the operator may override per Spirit:

| Class | Examples | Default policy |
|---|---|---|
| `readonly_scoped` | `fs.read` within manifest-declared subtree, `mem.read` private | Silent allow |
| `readonly_search` | `web.search`, `arxiv.search`, `mcp.tool` reads | Silent allow with rate limit |
| `mutating` | `fs.write` private, `mem.write` private | Silent allow within scope |
| `exec_capable` | `bash.exec`, `git.commit`, `provider.complete` (cost) | `prompt_with_diff` — show what will change before approving |
| `control_plane` | Spirit lifecycle (load/swap/unload), capability scope expansion, posture change | `prompt` — explicit approval, no remember-this-decision |
| `interactive` | Tool calls that emit audio/visual or external messages | `prompt` |

The Approval Manager's UX surface is owned by the IAC Bus — prompts can render in the local TUI, in the editor (via ACP), or as a mobile push notification.

#### 4.3.4 Token Lifecycle Manager

Capability tokens are short-lived (TTL ≤60s for high-privilege operations), bound to (Spirit-PID + boot-nonce + expiry), audit-logged at every use with origin-Spirit-ID. Tokens are non-transferable — they bind to the Spirit that requested them. **Hot-swap (I6)** preserves the token but rebinds the actor: when Spirit A is swapped to Spirit B, B inherits the in-flight tokens but its first action against any of them triggers a `posture_change` audit event.

The Token Lifecycle Manager handles re-validation at use against current state (TOCTOU correctness): every capability invocation re-reads the current posture, the current sandbox tier, the current consent envelope, and rejects if any have changed since issuance. There is no caching past state-change boundaries.

#### 4.3.5 Service-Boundary Manifest (P1–P4 per §4.0.8)

Security Manager is one of the kernel's services (per the §4.0.8 four-property test). The four boundary properties are recorded here as filesystem-and-Cargo-manifest facts; the §4.0.8 xtask reads these locations and fails CI on drift between manifest and filesystem.

| Property | Location at v0.1 |
|---|---|
| **P1: own crate** | `crates/services/security/Cargo.toml` (declares `[lib] name = "security"`); compiled as a separate Cargo crate within the `maos-kernel-core` workspace |
| **P2: own bin target** | `crates/services/security/src/bin/security.rs` (also declared as a `[[bin]]` target in the crate's `Cargo.toml`); v0.1 ships this in the same kernel binary via composition root, but the bin target exists for future extraction without code change |
| **P3: IPC contract crate** | `crates/iac/proto/src/security.rs` (re-exported as `iac_proto::security`); inter-service calls into Security Manager go through the typed IAC bus (§7.1) |
| **P4: independently restartable** | Supervised by `iac_runtime::supervisor::ServiceHandle`; restart-on-exit policy `RestartPolicy::Always { backoff_ms: 500..=30_000 }`; a panic in Security Manager does not take down peer services |

Analogous Service-Boundary Manifests for Memory Manager (§4.2), IAC Bus (§4.5), and Capability Registry (§4.6) — same four-property shape as this §4.3.5 — are recommended for a future revision; only Security Manager carries one in v1.0. Spirit Scheduler (§4.1) is the **supervisor** as defined in §4.0.8 and is exempt from P3 per that section's supervisor exception. It satisfies P1 (own crate), P2 (own bin target — `crates/services/spirit-scheduler/src/bin/spirit-scheduler.rs`, the kernel binary's `main`), and P4 (independently restartable — though as the supervisor it is the *target* of restart, not the *initiator*).

The two internal modules — I/O Subsystem (§4.4) and Telemetry Stream (§4.7) — fail at least one of P1–P4 at v0.1 (no separate crate, no bin target). They are eligible for extraction to services at v0.5+ when the four-property test can be satisfied.

### 4.4 I/O Subsystem (internal kernel module at v0.1; service-extraction at v0.5+)

**Status at v0.1:** Internal module within `maos-kernel-core`, not a separate service. Lives in `maos-kernel-core::io`. Service extraction (with its own task pool, retry budget, and circuit-breaker policy) is a v0.5+ option gated on real multi-provider deployment and observed contention.

**Responsibility:** Concrete transport adapters for everything that crosses the Host boundary.

**Adapters:**
- HTTP/HTTPS client (provider drivers, MCP-Streamable-HTTP servers, Spirit registry)
- HTTP/HTTPS server (control plane, ACP server, registry-side endpoints)
- Stdio transport (subprocess Spirit Wire Protocol; ACP server fallback)
- mTLS server + client (bilateral A2A peer)
- WebSocket (optional, for real-time editor integrations)

**Provider rate-limit isolation.** Per-(provider, credential) token bucket with kernel-mediated backpressure surfaced as a typed `RateLimited` IAC frame, not a stalled call. One Spirit hitting Anthropic's RPM limit must not block another Spirit on a different provider, or even the same provider with a different key. Bucket parameters declared in provider driver config.

**Network partition behavior in cross-host A2A.** A2A in-flight frames during partition are NACKed after a configurable timeout (default 30s); the kernel does NOT auto-retry. The application layer (the Orchestrator or peer Spirit) decides retry/escalate/halt.

### 4.5 IAC Bus (Inter-Agent Communication)

**Responsibility:** Same-Host frame routing, cross-Host bilateral A2A, the `retract` primitive, the notification surface dispatch.

**Same-Host (mailbox):** mpsc + broadcast, addressable by `SpiritId`. Bounded queues; backpressure via the Spirit Scheduler. Modeled on codex's `Mailbox`. Every frame is logged before delivery (I2).

**Cross-Host (bilateral A2A):** mTLS over TCP between two pre-paired Hosts. Each Host has the other's mTLS certificate fingerprint configured at deployment time (no discovery; the operator names the two endpoints). Per-frame ADR-012 typed-intent consent at both ends — sender's manifest declares which intent classes it will send under to which peer; receiver's manifest declares which intent classes it accepts from which peer. The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. Logical clocks (Lamport or hybrid logical clock) are used for cross-Host frame ordering; wall-clock is metadata only.

**Logical-clock frame ordering.** Cross-Host frame ordering is consistent under clock skew. Certificate validity windows remain wall-clock (X.509 conventions; the kernel does not reinvent).

**The IAC Bus also owns the `retract` primitive:** a Spirit can issue `retract(message_id, reason)`; the kernel marks the original log entry as retracted, sends a structured `retract` frame to the peer, and the peer's IAC Bus surfaces it to its human. **Retract is not delete** — the Transparency Log is append-only.

**Partial-consent failure semantics.** A frame whose sender approved but whose receiver rejected mid-frame (intent allowlist mismatch, posture change during transmission, token revocation) becomes a typed `ConsentRupture` event; the frame is quarantined, not delivered, not silently dropped. The sender's Spirit receives a `ConsentRupture` IAC frame; the operator surface logs the rupture for forensic review.

### 4.6 Capability Registry

**Responsibility:** Mediate every external call. Issue, verify, and revoke capability tokens. Enforce manifest-declared capability surfaces. Validate I11/I12/I13 audit-chain fields on digest writes. Track per-Spirit budget (ADR-016 token-budget accounting).

**Decomposition (round-2 ADR-030).** The Capability Registry is internally split into four sub-services so the hot path does not serialize on the slow path:

| Sub-service | Responsibility | Lock model |
|---|---|---|
| `cap-tokens` | Token issue, verify, revoke | Sharded `Arc<[CapShard; 64]>` where each shard is `RwLock<HashMap<TokenId, AtomicCounters>>`. Verify path takes read-lock on one shard (hash(token) % 64), CAS on `AtomicU64` quota counters. No global lock. |
| `cap-policy` | Consent rules, intent allowlists, posture-bound capability surfaces | Read-mostly; copy-on-write for policy updates |
| `cap-audit` | Transparency Log writer, lineage validation (I11/I12/I13) | bounded MPSC `tokio::sync::mpsc::channel(8192)` to a single `audit_writer` task that batches into the journal |
| `cap-quota` | Per-Spirit budget tracking, `ContextPressure`/`ContextLimit`/`EContextExhausted` typed frames | Per-Spirit atomic counters; soft threshold (80%) emits `ContextPressure`, hard (95%) emits `ContextLimit`, above 100% returns `EContextExhausted` on new tool calls |

The hot path (token verify on every IAC frame and every tool call) goes through `cap-tokens` only, with sharded atomic operations. The audit/lineage path is async (mpsc channel) so a slow Transparency Log write cannot block frame delivery.

#### 4.6.1 Epistemic Halt mechanism

When a Spirit's evidence is insufficient or contradictory, the Spirit invokes `epistemic.halt(payload)`. The kernel takes four actions atomically:

1. **Logs** the halt to the Transparency Log as a typed `epistemic_halt` entry, with the structured payload, the tasks/frames in flight, and the Spirit's confidence at halt time.
2. **Transitions** the Spirit to the `EpistemicHalt` lifecycle sub-state — distinct from `AwaitingApproval` (which gates Capability Tokens) and `Suspended` (user-initiated pause). All in-flight Capability Tokens are *frozen*, not released — if the user provides resolution and the Spirit resumes, the tokens come back live (subject to expiry).
3. **Surfaces** the halt to the user via the kernel-rendered notification surface as a structured "I cannot answer this confidently" outcome.
4. **Returns** a `halt_id` to the Spirit, which the Spirit retains for correlation when resolution arrives.

**Halt payload schema** (the kernel-known shape; v1.0 closed):

```jsonc
{
  "gap_kind": "evidence_conflict | evidence_insufficient | source_unreliable | beyond_capability | resource_unavailable",
  "summary": "human-readable, 1–2 sentences",
  "evidence_so_far": "optional reference: memory key, transcript range, or list of MCP-resource URIs",
  "query_strategies": ["optional", "list", "of", "concrete next steps the Spirit would take if given resolution"],
  "confidence_at_halt": 0.42  // optional float [0,1]
}
```

**Resolution.** The user (or another Spirit acting through the control plane) responds via `epistemic/resolve(halt_id, resolution)`. Three resolution kinds:

- `provided_context` — additional evidence, sources, or instructions are attached. The Spirit's `epistemic/resolve` handler decides whether the new context closes the gap. If yes, the Spirit transitions back to `Running` with frozen tokens reactivated. If no, the Spirit may halt again (with a refined payload) or accept the halt.
- `accepted_halt` — the user agrees the Spirit cannot proceed. The Spirit transitions to `Unloaded` (or to a clean checkpoint, depending on its `on_unload` hook). The original task is marked `abandoned` in the Transparency Log; downstream consumers can route the work elsewhere.
- `authorized_override` — the user explicitly accepts the risk and tells the Spirit to proceed despite the gap. The Transparency Log records the override with the user's stated reason. The Spirit's subsequent output carries an `override_marker` (mandatory for `output_shape` predicates), so downstream consumers can see the output proceeded past an acknowledged epistemic gap.

**Halt detection strategy — three-layer composition.** The kernel does NOT introspect Spirit-internal "uncertainty" — it cannot. There is no kernel-side LLM-state inspection, no Future-state probing, no statistical drift detector. Halt detection is composed:

1. **Spirit-self-invocation (primary).** The Spirit calls `epistemic.halt(payload)` from its `[epistemic_policy]` manifest rules (per-tag thresholds + four universal-arithmetic predicates from ADR-022). Trust model: the Spirit is the authority on whether its own evidence is sufficient; the kernel only enforces the declared policy on emit.
2. **Budget-based stall detection (secondary, kernel-side).** When a Spirit holds a `task.assign` and emits no progress IAC frame for >`timeout_no_progress` seconds (default 30s, configurable per manifest), the kernel emits a typed `task.stalled` event to the operator surface. This is NOT a halt — it is an external-detected stall — but it is the kernel's only mechanism for catching Spirits that are silently looping or wedged. Resolution is operator-mediated.
3. **Scalar trajectory tap (tertiary, instrumentation).** Observer Spirits subscribe to a `scalar.tap` stream that emits every Spirit's `working_memory.set_scalar` write. This lets diagnostic Spirits *observe* pre-halt scalar drift, but the *halt decision* still belongs to the Spirit being observed — Observer cannot force a halt on a peer.

**Manifest policy (`[epistemic_policy]`).** Spirits declare per-tag rules that map output frame tags (e.g., `claim.load_bearing`, `claim.exploratory`, `speculation`, `conversational`, `diagnosis.root_cause`) to one of three actions: `verbalize_only`, `flag`, or `halt`. Each rule may also specify `on_confidence_below` (numeric threshold) and `on_evidence_conflict` (boolean). Frames not matching any rule fall through to `default_action`, which itself defaults to `verbalize_only` — the kernel fails *open*, never closed. The Capability Registry intercepts emits on the path to the IAC Bus and enforces the rule for the frame's tag; Spirits cannot opt out of their own declared policy mid-task.

**A Spirit configured well halts rarely.** A Researcher tagged correctly might emit fifty `verbalize_only` frames (conversational, observational, exploratory), a few `flag` frames (claims with non-trivial uncertainty), and one `halt` per session at most — when a load-bearing conclusion sits on contradictory or insufficient evidence. The halt is the alarm bell, not the doorbell.

### 4.7 Telemetry Stream (internal kernel module at v0.1; service-extraction at v0.5+)

**Status at v0.1:** Internal module within `maos-kernel-core`, not a separate service. Lives in `maos-kernel-core::telemetry`. The module exposes a sink interface (so v0.5+ stream-processor implementations swap without API churn). Broadcast subscriptions are spawned onto the **shared kernel `tokio::runtime::Handle`** — no dedicated `LocalSet` or runtime instance, no separate worker-thread pool — which keeps fanout latency tight at v0.1's small subscriber counts. Tokio is **cooperatively scheduled at `.await` points** (no preemption, no time-slice quanta in the OS-scheduler sense); the per-task `coop` budget (Tokio's automatic yield after ~128 poll operations) protects only against accidental tight async loops, not against synchronous blocking. Synchronous blocking calls inside subscriber callbacks (file I/O, `std::sync::Mutex` held across await, CPU loops > 100 µs) MUST be offloaded via `tokio::task::spawn_blocking`. Service extraction is a v0.5+ option gated on real stream-processing demand (e.g., Observer fanout exceeding shared-runtime capacity, or stream-processor implementations that need a dedicated runtime). At v0.5+ extraction, the module would gain its own runtime and become a service per §4.0.8's four-property test.

**Responsibility:** Per-Spirit perceptual surface. Topic-based broadcast + filtered subscription.

The Telemetry Stream is the only kernel service Spirits consume passively. Spirits emit `telemetry.event` frames; the Stream broadcasts to all subscribers matching the topic. Observer-class Spirits subscribe broadly; other Spirits subscribe narrowly (Butler subscribes to Calendar/Slack/Figma topics; Mira subscribes to production-service-metrics topics).

**`scalar.tap` channel.** A dedicated read-only stream from the Capability Registry's tagged-scalar slot. Every `working_memory.set_scalar(tag, value, derived_from)` write emits a `scalar.tap` event with `(spirit_id, tag, value, timestamp)`. Observer Spirits subscribe to see pre-halt scalar drift in real time; this is the diagnostic signal Mira-class Spirits use to characterize an incident's runup.

**OpenTelemetry export adapter.** v0.5 ships basic OpenTelemetry export (every IAC frame, every capability invocation, every halt event); v1.0 adds SLO-class export with structured trace IDs and span linkage.

**Author-observability contract.** A Spirit author can read the same diagnostic surface the operator sees for their own Spirit, redacted of cross-Spirit data. This makes the Spirit author's debugging loop tight without exposing peer-Spirit state.

The Telemetry Stream owns no state. It is the kernel's lung — pure broadcast, no buffering beyond per-subscriber bounded queues.

#### 4.7.1 Telemetry Contract — IAC Round-Trip

The Telemetry Stream module is the producer for the IAC round-trip metrics §13.1 alert rules consume. The metric names, types, label sets, and histogram bucket boundaries are normative — implementers cannot wire the §13.1 PromQL without these.

Exposed by the kernel on `/metrics` (Prometheus text format, scrape interval 15s):

| Metric | Type | Unit | Labels |
|---|---|---|---|
| `iac_rt_duration_us` | histogram | microseconds | `service` ∈ {security, memory, iac, capability, spirit_scheduler}, `outcome` ∈ {ok, err, timeout} |
| `iac_rt_inflight` | gauge | requests | `service` |
| `iac_rt_errors_total` | counter | errors | `service`, `kind` ∈ {transport, decode, timeout, app} |

**Note on the `service` label set.** The label set includes `spirit_scheduler` (the supervisor per §4.0.8) in addition to the four supervised services. SS appears as `service` when it originates an IAC RT (supervisor-initiated capability check, lifecycle frame); appears as `peer_id` when it dispatches on behalf of another service. The label set therefore has five entries while the xtask `SUPERVISED_SERVICES` list has four; the divergence is intentional and explained in §4.0.8.

**Metric pair semantics.** `iac_rt_inflight` (gauge, count of in-flight requests) and `iac_rt_duration_us` (histogram, microseconds of round-trip duration) are linked by Little's Law in steady state: `E[inflight] ≈ arrival_rate × E[duration]`. Implementers MUST NOT multiply gauge × histogram-quantile to estimate traffic — use the histogram's `_count` series (Prometheus auto-derived) for arrival rate. The pair is exposed jointly because saturation diagnosis requires both load (inflight) and latency (duration) to discriminate "slow per request" from "more requests than headroom."

**Histogram buckets for `iac_rt_duration_us`** (exponential, base √2, anchored on the 1500µs SLO from §13.1):

```
le = [50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]
```

Rationale: 18 buckets (under Prometheus's soft 20-bucket guidance per histogram); the SLO threshold (1500µs) is itself a bucket boundary, so `histogram_quantile(0.95, ...)` interpolates within a bucket whose boundaries are explicit, not implementation-dependent. Buckets below 50µs are omitted — IAC round-trip never goes sub-50µs in practice (network syscalls dominate).

The `_bucket`, `_count`, `_sum` suffixes referenced in §13.1 PromQL are the standard Prometheus histogram-derived series; no separate definition is needed beyond standard Prometheus client-library behavior.

Reference Rust constant for the kernel's metric emitter:

```rust
pub const IAC_RT_BUCKETS_US: &[f64] = &[
    50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 450.0, 700.0,
    1000.0, 1500.0, 2200.0, 3300.0, 5000.0, 7500.0, 11000.0,
    16000.0, 25000.0,
];
```

## 5. Spirit ABI

The Spirit ABI is the contract between the kernel and a Spirit. Every Spirit conforms to it. The kernel does not negotiate; the Spirit either matches the ABI version or refuses to load.

**A Spirit's implementation is *behavior*, not *infrastructure*.** A Spirit's code contains lifecycle hook handlers, IAC frame handlers, telemetry handlers, decision logic, the system-prompt template, and (optionally) the output/explanation/epistemic predicate callbacks. **It does not contain HTTP libraries, LLM provider SDKs, MCP client implementations, socket code, or filesystem code.** All that work flows through Layer 1 capabilities which the kernel implements. The Spirit calls `capability/invoke(token, args)` and receives a stream of typed events; the kernel does the actual HTTP, the actual SDK calls, the actual sandboxed exec, the actual MCP wire protocol.

A Spirit binary therefore stays small (the Rust reference implementations target hundreds of KB to a few MB). Sharing a Spirit becomes cheap. Polyglot Spirit ecosystems become feasible — a TypeScript Spirit and a C# Spirit speak the same wire protocol because neither imports an HTTP library; both delegate to the kernel's adapters. And every provider call, every MCP invocation, every shell exec is uniformly audited via the Capability Registry — there is no Spirit shortcut path that bypasses the kernel by talking directly to an HTTP endpoint. **The Spirit author's job is to design behavior; the kernel's job is to be the substrate that behavior runs on.**

### 5.1 Spirit Manifest schema

The manifest is a TOML file declaring everything the kernel needs to load, sandbox, schedule, and audit a Spirit class.

```toml
[class]
name = "code-reviewer-pro"
version = "1.2.0"
abi = "1.0"
manifest_schema_version = 1
min_substrate_version = "1.0.0"     # kernel rejects load if its own version is below this
forms = ["subprocess"]               # which Spirit forms this class ships in: rust-inproc | subprocess
trust_tier = "public-untrusted"      # local | org-internal | public-untrusted
signing_key = "ed25519:xxx..."       # author's public key
description = "..."

[capabilities.required]
fs.read = ["**/*.rs"]
fs.write = ["**/*.rs"]                # gated by approval
provider.complete = ["anthropic.claude-3-5-sonnet"]
mcp.call = ["github.search"]
iac.send = ["broadcast", "spirit:peer:bilateral"]

[capabilities.parallelism]
max_concurrent_tool_calls = 4

[posture]
default = "assistive"                 # cautious | assistive | autonomous-with-halt | autonomous
allowed_max = "autonomous-with-halt"  # ceiling beyond which the Spirit cannot self-shift

[output_shape]
# Predicate over emitted frames. Kernel rejects emits failing this shape.
required_fields = ["severity", "file", "line", "suggestion"]
predicates = ["..."]

[explanation_shape]
# For decision.* frames: which "because" payload is mandatory
required_fields = ["evidence_refs", "alternatives_considered", "confidence"]

[epistemic_policy]
# Per-tag rules; kernel maps output frame tags to verbalize_only | flag | halt
[[epistemic_policy.rule]]
tag = "claim.security_vulnerability"
action = "halt"
on_confidence_below = 0.85
on_evidence_conflict = true

[[epistemic_policy.rule]]
tag = "claim.style_suggestion"
action = "verbalize_only"

default_action = "verbalize_only"

[budget]
context_window_size = 200000
context_pressure_threshold = 0.80     # emits ContextPressure
context_limit_threshold = 0.95        # emits ContextLimit
time_cap_seconds = 300                # soft warning at 80%; kernel emits BudgetWarning
cost_cap_usd_per_hour = 10.00

[skills.search_path]
paths = ["~/.maos/skills/", "_bmad/skills/", "/usr/share/maos/skills/"]

[hot_swap]
state_schema_uri = "https://schemas.maos.dev/spirit-state/code-reviewer-pro/v1.cbor"
state_schema_version = 1

[halt_protocol_compatibility]
version = 2                           # halts produced under this Spirit class can be migrated
                                       # to a successor declaring halt_protocol_compatibility >= 2

[intent_promotion_set]
# When THIS Spirit consumes a digest, which intent_lineage classes are admissible
allowed = ["consult", "review"]
# Digests with intent_lineage NOT in this set are rejected with EIntentPromotionDenied

[migrates_from]
versions = ["1.0", "1.1"]              # cross-major migration via the migrate() ABI entry point

[swap_invariants]
preserve = ["open_pr_state", "review_queue"]   # HSIS-tested invariants

[resources]
# Cgroups v2 / setrlimit / Job Object ceilings for subprocess-form Spirits
cpu_max_pct = 50
memory_max_mb = 512
fd_max = 64

[sandbox]
tier = "T2"                           # kernel applies strictest-of (manifest, trust-tier, operator-policy)

[forbidden_capabilities]
# Negative assertion; kernel enforces never holding tokens for these
deny = ["bash.exec", "git.commit"]

[lifecycle]
on_load    = ["spirit-code-reviewer-pro::hooks::on_load"]
on_idle    = ["spirit-code-reviewer-pro::hooks::on_idle"]
on_swap_in = ["spirit-code-reviewer-pro::hooks::on_swap_in"]

[author]
name = "Diego Hernandez"
contact = "diego@example.com"
homepage = "https://github.com/diego/code-reviewer-pro"
```

The schema is versioned (`manifest_schema_version`) independently of the kernel and ABI; the kernel ships a compatibility matrix in `STABILITY.md`.

### 5.2 Spirit Wire Protocol (subprocess form)

Subprocess Spirits speak a JSON-RPC-shaped protocol over stdio with CBOR payloads. Wire-level details:

**Framing.** LSP-style: `Content-Length: <decimal>\r\n\r\n` followed by exactly N bytes of CBOR-encoded payload. Header is ASCII, case-insensitive name, max header block 4 KiB.

**Backpressure.** `BufReader` cap = 1 MiB; frames exceeding cap = `WireError::Oversize`, halt the Spirit. Writer uses `tokio::io::AsyncWriteExt::write_all` over a bounded `mpsc<Frame>(64)` — channel full = backpressure to caller, never drop.

**Stderr separation.** A separate `tokio::process::ChildStderr` is piped to `tracing` at `WARN` level with the `spirit_id` span. Never multiplexed onto stdout; out-of-band Spirit logs go through stderr only.

**EOF semantics.** Clean EOF after the last full frame = `Halt::Voluntary`. EOF mid-frame = `Halt::Fault(Truncated)`.

**Signal handling.** SIGTERM → 5-second grace period → SIGKILL. The supervisor records the halt cause.

**Method set (kernel-to-Spirit, lifecycle):**
- `lifecycle/load(manifest)` → `loaded`
- `lifecycle/start()` → `started`
- `lifecycle/swap_in(predecessor_state)` → `running` — hot-swap; you inherit this state
- `lifecycle/snapshot()` → `<state CBOR>` — produce hot-swap snapshot
- `lifecycle/pause()` → `paused`
- `lifecycle/resume()` → `running`
- `lifecycle/unload()` → `unloaded`
- `lifecycle/migrate(predecessor_state)` → `successor_state` — cross-major migration entry point
- `event/inbound(frame)` → `()` — IAC frame delivery; kernel writes a shadow-recall record before invocation (per I12)
- `event/telemetry(event)` → `()` — telemetry tick delivery
- `event/idle()` → `()` — `on_idle` lifecycle hook fire
- `epistemic/resolve(halt_id, resolution)` → `resumed | unloaded | halted`

**Method set (Spirit-to-kernel, capability invocation):**
- `capability/invoke(token, args)` → stream of typed events
- `iac/send(frame)` → `frame_id`
- `iac/recall(filter, limit, cursor)` → `[frame_ids]` — `log.recall` per the audit-chain primitive
- `iac/fetch(frame_id)` → `frame_payload`
- `iac/broadcast(topic, frame)` → `()`
- `mem/read(scope, key)` → `value`
- `mem/write(scope, key, value)` → `()`
- `working_memory/set_scalar(tag, value, derived_from)` → `()`
- `epistemic/halt(payload)` → `halt_id`
- `approval/request(intent, target, capability)` → `decision`

**Cross-language byte-equal golden corpus.** Every frame variant ships with a `golden/<frame_name>.json` (authoritative reference shape) and `golden/<frame_name>.cbor` (canonical encoding); every language SDK must serialize a constructed frame to byte-equal CBOR and deserialize golden to a structurally-equal frame. Canonical encoding: sorted keys, no whitespace, UTF-8 NFC. Floor: 100% per frame variant per SDK at v1.0 ship gate.

**Wire-protocol fuzz commitment — tiered cadence ladder.** Three tiers, all mandatory; cumulative floor non-negotiable.

| Tier | Cadence | Time budget | Corpus seed | Failure gate |
|---|---|---|---|---|
| **T1 — Per-commit** | Every PR, blocking | **10 min wall-clock** on N=4 parallel workers (≈40 CPU-min) | Last-known-bad regressions + 500 mutated frames from coverage-guided pool | Any new crash, any stalled handshake >30s, any auth bypass |
| **T2 — Nightly** | 1×/day on main | **4 hours wall-clock** on N=8 workers (≈32 CPU-hours) | Full grammar fuzz + 5k mutated frames + dictionary-guided | Any crash, any state-machine deviation, p99 frame-parse latency regression >20% |
| **T3 — Pre-release** | Every release candidate, blocking | **24 hours wall-clock** on N=8 workers (≈192 CPU-hours) | T2 corpus + adversarial-Spirit transcripts + replay corpus | Zero crashes, zero auth bypasses, zero TLS downgrade paths |

**Cumulative pre-GA floor — per-target, not aggregate.** For each fuzz target T in `crates/iac/fuzz/`, the sum of `libfuzzer.exec_time_seconds` across all T1+T2+T3 runs in the 90 days preceding the GA tag MUST be ≥ **72 CPU-hours per target** (measured by libFuzzer's own runtime counter, summed across parallel workers; not wall-clock). Aggregate floor across all targets MUST be ≥ **1,000 CPU-hours**. CI publishes `fuzz_cpu_hours_per_target_90d` to Prometheus; release gate fails if any target < 72 or aggregate < 1,000.

**Catch-up rule (T2-elastic, T3-fixed).** If at the release-candidate cut, `fuzz_cpu_hours_per_target_90d` < 72 for any target, T2 nightly is extended on the deficient targets only — 4-hour nightly runs replaced with 12-hour nightly runs on those targets until each clears 72 CPU-hours or the GA date is reached. T3 budget remains fixed at 24h; T3 does not absorb T2 deficit. If GA arrives with any target still under floor, release is blocked.

*The tiered cadence is the execution model; the per-target floor is the gate.* Reducing T1 per-commit budget is a developer-experience optimization. Reducing the per-target or aggregate floor is a major-version conversation (ADR-037 `invariant-lock` gate fires). The unit is **CPU-hours by libFuzzer counter**, not wall-clock — wall-clock is gameable by adding workers; libFuzzer's own counter is not.

### 5.3 Lifecycle hooks

The hooks below are the part that makes hot-swap possible. The Spirit may handle any subset; unhandled hooks are no-ops.

| Hook | Fires when | What the Spirit does |
|---|---|---|
| `on_load` | Manifest read, capability tokens issued, Spirit loaded into memory | Initialize state, open persistent connections, load skills |
| `on_start` | First IAC frame routed to this Spirit | Begin operating |
| `on_frame(frame)` | An IAC frame addressed to this Spirit lands | Decide and emit response frame(s) |
| `on_telemetry(event)` | A subscribed telemetry topic emits | Update working state, possibly emit derived frames |
| `on_idle` | No work for ≥30s (configurable) | Proactive opportunity (Butler!); else no-op |
| `on_swap_out` | Kernel about to swap this Spirit out | Final state blob; in-flight tokens enumerated |
| `on_swap_in(predecessor_state)` | This Spirit is the successor in a hot-swap | Inherit state; rebind in-flight tokens |
| `snapshot()` → state | Kernel requests a hot-swap snapshot | Produce CBOR-encoded state per `[hot_swap].state_schema_version` |
| `migrate(predecessor_state)` → successor_state | Cross-major migration; predecessor's class is in this class's `migrates_from` list | Translate predecessor's schema to this class's schema |
| `epistemic_resolve(halt_id, resolution)` | User responded to a halt | Process resolution; transition back to `Running` or accept halt |
| `on_pause` | Operator paused this Spirit | Drop in-flight non-critical work; preserve halt state |
| `on_resume` | Operator resumed | Resume |
| `on_unload` | Graceful shutdown | Persist final state; close connections |
| `on_consolidate` | Spirit-author-defined cadence for memory-curation passes | Compact private memory; produce digests |

### 5.4 Posture

Posture is the Spirit's autonomy stance. Posture is mutable; class is not. The user can shift posture at runtime (`Butler, be more cautious for the next hour`); the kernel logs the shift and applies it to subsequent capability-scope decisions. Posture-shift propagation: P99 ≤2s, P99.9 ≤5s.

| Posture | Behavior |
|---|---|
| `cautious` | Every capability invocation prompts |
| `assistive` | Reads silent allow; writes prompt; default for most Spirit classes |
| `autonomous-with-halt` | Proceed unless `[epistemic_policy]` triggers a halt; user resolves via halt mechanism |
| `autonomous` | Proceed without prompts; rare; explicit user grant; halt mechanism still active |

The manifest's `[posture].allowed_max` sets a ceiling beyond which the Spirit cannot self-shift. The operator may override the ceiling per deployment.

## 6. Reference Spirits

Five reference Spirit classes ship in the kernel binary, plus three skill-package overlays for the founder loop. The reference Spirits prove the substrate; third-party authors ship their own classes via the Spirit registry.

### 6.1 Butler — Proactive Personal Agent

**Purpose:** Anticipatory single-Spirit assistant. Watches the user's calendar, communications, and active work; surfaces useful pre-emptive notifications without acting unsupervised.

**Cognitive shape (design report ¶154–¶168):** Active-Inference-style belief update over the user's current goal-state, then candidate-action ranking by expected free energy. The Butler maintains its own POMDP over user-goal-states; the kernel does not compute or interpret this — it only persists the Butler's tagged scalars and fires the `[epistemic_policy]` rules the Butler declared.

**Memory scope:** Per-user private memory. Episodic tier for the `notification_acceptance_log` (feeds future POMDP refinement across sessions). Optional shared scope on a single Host (the Butler can subscribe to other Spirits' "what are you working on" telemetry).

**Capabilities:** Calendar read (Google Calendar / Outlook via MCP), Slack read + draft (write gated by approval), Linear write (gated), Figma read, browser read.

**Posture:** `assistive` by default. Notifies, but does not act unsupervised. The user can shift to `cautious` (every notification prompts) or `autonomous-with-halt` (Butler may schedule small reversible actions, halts on uncertainty).

**Lifecycle hook anchor:** `on_idle`. The Butler runs its anticipatory reasoning loop whenever the kernel calls `on_idle` (no pending IAC frames + user activity stream shows >12 minutes since last meaningful interaction).

**Output shape:** Notifications carry structured `{pattern, confidence, evidence, options[]}` payload — the kernel rejects emit without these fields.

**Epistemic policy:** Butler's `[epistemic_policy]` halts on `self.belief_variance > 0.7` (the Butler computes variance using its own preferred uncertainty proxy; the kernel does universal-arithmetic comparison only). Halts on `claim.user_preference_drift` with `confidence_below = 0.6`. Verbalize-only on routine pattern-detection.

**Eval metrics:** notification precision (fraction acted on; floor ≥0.85 at v0.3), notification recall (fraction of relevant moments caught; floor ≥0.7), user-correction rate, time-to-action savings.

### 6.2 Researcher — Insightful Research Assistant

**Purpose:** Exploratory single-Spirit. Conducts rigorous literature surveys; proactively generates novel hypotheses and proposes future research directions.

**Cognitive shape (design report ¶87–¶89, ¶191):** Survey-mode posture (exploratory, reactive, divergent). Hypothesize-mode posture declared in the manifest's posture-set but ships fully at v1.0 (ILP + LLM hybrid for novel-hypothesis generation; ILP's structured rule discovery joined with LLM's pattern completion, output submitted to a Critic Spirit for refinement).

**Memory scope:** Private (the researcher's working knowledge graph). Episodic for cross-session bibliography persistence. Loom-lite collective tier opt-in for cross-team pattern reuse.

**Capabilities:** Web search, arXiv search, GitHub search, citation-graph traversal, MCP tool invocation broadly. Manifest `[capabilities.parallelism] = 8` — up to 8 concurrent tool dispatches.

**Posture:** `survey-mode` (exploratory, reactive, divergent) for routine surveys. `hypothesize-mode` for generative work (ILP+LLM hybrid).

**Output shape:** "findings + Open Questions + Confidence Map + Bibliography" — the kernel rejects emit without all four.

**Adaptive-chunk-ratio summarization** (design report ¶362; hermes-informed per §9.5): each paper digest under 4K tokens; citation-graph traversal identifies tight clusters of related work; the Spirit reads abstracts for many papers, full intros for some, full methods for the load-bearing few.

**Distillation pattern.** Researcher is the first opt-in production user of the §9.5 distillation pattern in a single-Spirit context. The five-metric gate applies (digest-recall ≥0.90, faithfulness ≥0.98, hedge-preservation ≥0.95, traceability=100%, secret-leakage=0%).

**Epistemic policy:** halts on `claim.methodology_strength` when two papers report contradictory findings both with strong methodology by the Spirit's scoring rubric. Halts on `claim.load_bearing` with `confidence_below = 0.7`. Verbalize-only on speculation tagged `claim.exploratory`.

**Eval metrics:** synthesis accuracy, citation correctness (≥95% reachable URLs), novelty of hypotheses (LLM-as-judge with rubric, in hypothesize-mode), Open Questions quality (rubric-judged).

### 6.3 Diagnostic Engineer (Mira-class)

**Purpose:** Production-edge diagnostic Spirit. Monitors live systems, detects anomalies, formulates and logs hypotheses, designs targeted software remediations, ships them, incorporates runtime feedback into iterative fixes.

**Memory scope:** Per-Spirit private (Mira's working diagnostic context). Loom-lite collective tier read+write — Mira reads ADR-pattern library to recognize known incident shapes; Mira writes new patterns when an incident produces a reusable diagnostic recipe.

**Capabilities:** Read-only on production runtime knobs (telemetry queries, log ingestion, thread-dump retrieval). Write on revert/scale/flag-toggle (gated by approval). Bash-exec whitelist for containment actions (`kubectl rollout undo`, `flag set`, etc.). Cross-environment telemetry queries to peer Hosts via bilateral A2A (sends to Architect-class Nash Spirit).

**Posture:** `sre-diagnostician` — `readonly_*` = silent allow; `mutating` = notify and log; `exec_capable` = prompt with diff (revert, scale, flag-toggle); `control_plane` = prompt.

**Asymmetric capability gates.** This is enforced by the Capability Registry, not by Spirit-side discipline. Read-only on source code; read-write on production runtime knobs. The asymmetry is what makes Mira safe to run on a production-edge node.

**Epistemic policy at production fidelity:**
- `diagnosis.root_cause` halts at `confidence_below = 0.6` or evidence conflict — when telemetry is consistent with multiple incompatible root-cause hypotheses, Mira does not pick the most plausible; she invokes `epistemic.halt(gap_kind: "evidence_conflict", evidence_so_far, query_strategies)`.
- `diagnosis.observation` is `verbalize_only` — observations of metrics or thread dumps don't need confidence gating; the data is what it is.
- `containment.action` halts at `confidence_below = 0.5` — Mira would rather block on an uncertain containment plan than execute one and worsen the incident.

**Bilateral A2A escalation.** When Mira's diagnosis confidence is high enough (above the halt threshold) but the fix requires source-code changes, Mira escalates to Nash via cross-host A2A. The frame carries typed intent `diagnosis-handoff:read-only-evidence` — Nash's consent policy permits this from the prod-edge peer; Nash's policy explicitly excludes `remote-write-request` and `code-mutation-directive` from the same source. The bilateral consent envelope ensures Mira cannot trigger Nash's source-modification capabilities directly; Nash decides whether to act on Mira's diagnosis.

*Corpus methodology for Mira (and other safety-critical Spirits) is in §6.6.*

### 6.4 Senior Architect (Nash-class)

**Purpose:** Dev-environment principal architect. Produces production-grade code conforming to organizational standards; owns testing, deployment, CI/CD; closes the loop with telemetry from the Diagnostic Engineer.

**Memory scope:** Per-repo private memory (the user's coding style guide, prior decisions, ADRs). Shared on-Host for cross-Spirit coordination during the founder loop. Collective (Loom-lite) read+write for ADR-pattern library, fix templates, regression-test references.

**Capabilities:** Full RW source repo. CI/CD orchestration (GitHub Actions / equivalent). `provider.complete` for code generation. `git.commit` (gated). `bash.exec` whitelist for build and test commands. Cross-environment telemetry queries to peer Hosts (bilateral A2A from Mira).

**Posture:** `principal-architect` — silent on source mutations within a configured workspace; prompts on every deploy (the human gate is load-bearing); uses the granular approval mode for fine-grained "yes to this PR but not the next one" control.

**Failure mode to design against:** Nash autonomously deploying something subtly wrong because the test suite has gaps. Mitigation: deploys always go through the Sentinel-validated canary protocol when an Observer Spirit is colocated.

**Bilateral A2A receive-side.** Nash receives diagnosis-handoff frames from Mira via cross-host A2A. The receive-side consent policy admits `diagnosis-handoff:read-only-evidence` only; explicitly excludes `remote-write-request`. Nash decides whether to confirm the diagnosis (read source, walk patterns), propose a fix (PR draft + ADR), and close the loop back to Mira.

### 6.5 Observer

**Purpose:** Read-only perceptual layer. Subscribes broadly to the Telemetry Stream; renders the local "what's happening" view; can be a passive Spirit or a notification source.

**Memory scope:** Per-Spirit private (rolling window of observations). Optional shared on-Host for surfacing aggregate views.

**Capabilities:** Telemetry Stream broadcast subscription (broad). `scalar.tap` subscription to see pre-halt scalar drift across peer Spirits. No write capabilities by default; the Observer cannot send IAC frames except `notification.surface` (kernel-rendered to the user).

**Posture:** `passive-observer` — silent allow on all reads; no exec; no mutating; no control-plane.

**Use case in the founder loop.** The Observer subscribes to the Orchestrator's `task.assign` and Worker `task.complete` frames (read-only); renders a live "what are the agents doing" view in the operator's TUI or editor banner.

**Use case in the diagnostic-architect bilateral pair.** The Observer colocated with Nash watches `scalar.tap` from Mira; surfaces pre-halt scalar drift before Mira's halt actually fires, so Nash can pre-stage source-walks while Mira is still gathering evidence.

### 6.6 Safety-critical Spirit corpus methodology

Applies to §6.3 Mira and §6.4 Nash; future safety-critical Spirit classes inherit this methodology when specified. Confidence-threshold halts (Mira's `diagnosis.root_cause` <0.6, `containment.action` <0.5, and equivalent thresholds for other safety-critical Spirits) require empirical calibration against a labeled corpus. The corpus is the substrate for §8.0 floor 6 (precision ≥0.85, recall ≥0.95).

**Corpus size.** N ≥ 150 labeled scenarios per safety-critical Spirit at v1.0. v0.5 lower bound: N ≥ 50 (calibration phase, no enforcement). v0.7: N ≥ 100 (precision floor enforced, recall floor advisory). v1.0: N ≥ 150 (both floors enforced).

**Stratification (mandatory, all five classes represented):**
- **40% true-halt-required** — scenarios where halt is the correct action (root cause genuinely ambiguous, containment action genuinely risky).
- **20% borderline** — scenarios near the threshold where reasonable labelers might disagree; tests calibration sensitivity.
- **20% false-positive-bait** — scenarios that *appear* to warrant halt but where confident proceed is correct (tests over-cautious failure mode).
- **10% adversarial-evasion** — scenarios constructed to slip past the halt logic (tests under-cautious failure mode).
- **10% benign-suspicious** — scenarios with surface markers of risk but benign ground truth (tests false-positive baseline).

**Labeling protocol.** Two independent labelers per scenario, blind to each other's judgments. Inter-rater agreement Cohen's κ ≥ 0.7 required for corpus admission; below 0.7, scenario is rewritten or discarded. Disagreements resolved by third labeler; if third labeler also disagrees, scenario is discarded (do not force consensus on genuinely ambiguous cases — they are precisely what the corpus needs to discriminate).

**Refresh cadence.** 20% of corpus rotated quarterly. Rotation is mandatory, not opportunistic — it prevents corpus overfit and surfaces drift in operational distribution. Retired scenarios retained in versioned archive for regression checks.

**Refresh IAA gate (no grandfathering).** Each refresh requires Cohen's κ ≥ 0.75 computed over a sample of ≥ 20 scenarios drawn (stratified by scenario class above) from the refresh corpus, with ≥ 2 independent annotators labeling each sampled scenario. Disagreements are adjudicated per the labeling protocol before κ is computed. Sample is re-drawn each refresh; scenarios are not reused across consecutive refreshes. Quarterly rotation report MUST publish per-scenario κ values; any scenario with κ < 0.7 is rejected and the slot is refilled before the rotation closes. **No grandfathering** — scenarios admitted under prior policy versions are re-IAA'd at the next refresh cycle they appear in.

**Reporting.** Each Spirit's halt-precision/recall reported per-stratum AND overall. Aggregate floor (≥0.85 / ≥0.95) is necessary but not sufficient — any single stratum below 0.7 precision or 0.85 recall triggers calibration review even if aggregate passes.

The asymmetric floor (precision ≥0.85, recall ≥0.95) encodes the cost model: missed halt is unrecoverable (a safety event escapes), over-halting is recoverable (operator can resume). Recall floor is therefore higher than precision floor by ~10× cost asymmetry.

### 6.7 Skill-package overlays for the Founder Loop

Three skill packs — **Orchestrator** (`orchestrator-bmad`), **Developer-Worker**, **Reviewer-Worker** — load into agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) via the `maos-bridge` skill. These are not first-class Rust Spirit crates; they are skill-package overlays on the kernel-builtin `CliWrapperSpirit` class.

**CliWrapperSpirit specification.** Configured with: CLI binary path; skill bundle (`maos-bridge` + persona skills); `output_shape_version: "<semver>"` (kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed != declared); posture declaration (stdio shape, control-channel mechanism, shutdown signal); capability scope mapping; output-shape adapter implementation (registered as `cli-wrapper-template:<cli-name>:<shape-version>` in the Spirit registry); crash semantics (kernel observes EOF on stdio + non-zero exit → `SpiritDied` event journaled; recovery policy declared in wrapper config: `respawn-with-context` / `respawn-fresh` / `escalate`).

**Fail-loud rule:** wrappers cannot fall back to "best-effort parsing" on shape mismatch. Audit drift is the failure mode the substrate cannot tolerate.

**Founder loop usage.** Lunarpulse spawns the Orchestrator (a Claude Code process loaded with `orchestrator-bmad` and `maos-bridge` skills, posture `autonomous-with-halt`, halt policy preferring recall over precision). The Orchestrator dispatches `task.assign` IAC frames to Developer-Worker Spirits (local + remote on a second laptop, A2A loopback within the same logical "founder topology"). Each Worker is an agent CLI process with `developer` + `maos-bridge` skills. Reviewer-Worker handles code review per `bmad-code-review`. Distillation pattern operational: raw Worker output → Transparency Log → Spirit-side LLM distillation → digest in working memory + episodic.

### 6.8 Extensibility — defining a new Spirit class

A new Spirit class is a new manifest plus a Spirit binary that conforms to the Spirit ABI. The kernel does not need modification. The author writes the manifest, implements the lifecycle hooks (handling whichever subset they care about), declares the capability surface, declares the epistemic policy, declares the output shape, and signs the binary with their Ed25519 key. Diego's `code-reviewer-pro` is the canonical example.

Spirits the substrate has not yet imagined — Negotiator, Tutor, Wet-Lab Coordinator — slot in by declaring their own capability surfaces, epistemic policies, and output shapes. The kernel grows slowly so the ecosystem can grow fast.

## 7. Inter-Agent Communication

### 7.1 Same-Host: the mailbox

The IAC Bus on a single Host uses `tokio::sync::mpsc` and `tokio::sync::broadcast` channels addressable by `SpiritId`. Bounded queues; backpressure via the Spirit Scheduler. Modeled on codex's `Mailbox`.

**Frame shape:**
```jsonc
{
  "frame_id": "ulid:01J...",
  "timestamp": "2026-05-06T18:42:01.234Z",
  "logical_clock": 12847,
  "from": { "spirit_id": "...", "host_id": "..." },
  "to": [ { "spirit_id": "...", "role": null } ],   // role is null for direct addressing
  "kind": "task.assign | task.complete | decision.dispatch | epistemic.halt | telemetry.event | consent.request | retract | ...",
  "intent": "delegate | consult | review | broadcast | request | ...",
  "payload": { ... },
  "auto_marker": "human-authored | spirit-auto | spirit-drafted-human-approved",
  "consent_envelope": { "intent_class": "...", "scope": "...", "valid_until": "..." }
}
```

Every frame is logged before delivery (I2). The kernel writes the Transparency Log entry first, then routes to the recipient mailbox; the IAC Bus does not deliver frames the log refused to record.

### 7.2 Cross-Host: bilateral A2A

Cross-Host communication uses A2A over mTLS+TOFU between two pre-paired Hosts. This is the topology the Diagnostic Engineer + Senior Architect bilateral pair runs on.

**Pairing model.** Each Host's deployment configuration names the other Host's mTLS certificate fingerprint. There is no discovery protocol because there is nothing to discover — the operator names the two endpoints. First-contact TOFU pinning verifies the configured fingerprint; subsequent connections re-verify against the pinned cert.

**Per-frame consent (ADR-012 typed-intent).** Each Host's manifest declares which intent classes it sends to its peer and which it accepts from its peer. The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. This is what makes Mira's `diagnosis-handoff:read-only-evidence` admissible at Nash while `code-mutation-directive` is rejected.

**Logical-clock frame ordering.** Cross-Host frame ordering uses logical clocks (Lamport or hybrid logical clock — final pick by v0.5); wall-clock is metadata only. Cross-Host frame ordering is consistent under clock skew.

**Network partition behavior.** A2A in-flight frames during partition are NACKed after a configurable timeout (default 30s); the kernel does NOT auto-retry. The application layer (the Spirit) decides retry/escalate/halt.

**Certificate rotation.** Cross-reference: full mTLS rotation chaos test specification in §7.2.1 below.

#### 7.2.1 mTLS Rotation Chaos Test (Quarterly, Mandatory)

Steady-state mTLS verification is necessary but insufficient. The failure mode worth testing is *forced rotation under live load* — the moment when issuing CA, agent SDS endpoints, and active connections must reconcile within bounded time without dropping data-plane traffic.

**Test schedule.** Quarterly, on calendar (not opportunistic). Production-equivalent staging environment. Synthetic load at p95 of trailing-30-day production traffic.

##### 7.2.1.a Pre-staged-overlap rotation procedure

**Variable definitions.** `p99_handshake_rtt` = trailing 30-day p99 of TLS 1.3 handshake duration (ClientHello → Finished, measured at the initiator) for IAC service-to-service connections, computed in steady state (excluding any active rotation drill window). Source metric: `iac_handshake_duration_us` (histogram, see §4.7.1). Recomputed daily; cached value used for the duration of any single rotation drill. If <30 days of data exist (cold deployment), use the maximum observed handshake duration over available history, floored at 500 ms. Then: `T_grace = max(2 × p99_handshake_rtt, 5 s)`.

**Procedure.** Agents MUST be provisioned with the replacement cert at least `T_grace` before the old cert's revocation timestamp. During `[t_provision, t_revoke + T_grace]`, agents accept either cert on inbound handshakes; clients prefer the new cert on new connections. Handshake failures with `BAD_CERTIFICATE` or `CERTIFICATE_EXPIRED` MUST trigger client retry: 3 attempts at 100 ms / 300 ms / 1000 ms backoff (jittered ±20%), retrying handshake (NOT request) only. After `t_revoke + T_grace`, old cert is hard-revoked; subsequent handshake attempts with old cert MUST fail-closed (rejection logged as `cert_post_grace_reject`, NOT counted as data-plane error). Gate: `cert_post_grace_reject` count ≤ 0.1% of total handshakes during the rotation window.

**Backoff derivation.** Schedule is 100 ms / 300 ms / 1000 ms across 3 attempts (4 total tries including the original). Derivation: base interval = ⌈p50(handshake_rtt)⌉ rounded to nearest 100 ms (target ~100 ms in steady-state IAC handshake measurements); growth factor = 3× (vs. envoy/istio default 2×) chosen because IAC handshake failures cluster on transient peer-state-sync issues that resolve on the order of seconds, not hundreds of milliseconds — wider spacing converges faster than more retries at tighter intervals; cap at 3 attempts because pre-deployment expectation: ≥ 99% of recoverable failures resolve within attempt 3, with attempt 4+ contributing < 0.5% additional success at 4× the latency cost. Schedule jittered ±20% to prevent thundering-herd on shared peer recovery. *(Note: empirical floors will be re-validated against measured rotation-drill telemetry in v0.7; if measured p50 differs materially from ~100 ms, the schedule re-derives from that measurement.)*

This pre-staged-overlap mechanism resolves the apparent contradiction between "zero data-plane errors" and "no fail-open on cert mismatch": errors during the grace window are absorbed by client retry; errors after the grace window are isolated, logged separately, and bounded; fail-open never occurs because both certs are validly trusted during overlap.

##### 7.2.1.b Cert rotation timing gates

During the scheduled rotation drill, instrument three timestamps per agent:
- `t_0` — `revoke()` API call returns success at CA
- `t_1` — agent's OCSP/CRL check first returns `revoked` for old cert
- `t_2` — agent completes successful TLS handshake with replacement cert AND first data-plane request succeeds

Compute and gate three distributions across the agent fleet:

| Metric | Definition | Floor (p50) | Floor (p99) | Owner |
|---|---|---|---|---|
| Revocation propagation latency | `t_1 − t_0` | ≤ 30 s | ≤ 90 s | PKI |
| Re-handshake latency | `t_2 − t_1` | ≤ 30 s | ≤ 60 s | Platform |
| End-to-end rotation latency | `t_2 − t_0` | ≤ 60 s | ≤ 150 s | Joint |

A drill PASSES only if all three rows pass at both p50 and p99. Per-row failure routes to the owning team. Additionally: `cert_post_grace_reject` rate ≤ 0.1% (per §7.2.1.a above).

**Failure response.** Any breach of any floor is a release-blocking issue at v0.7+. v0.5 reports all three metrics without enforcement (calibration phase). v0.7 enforces revocation propagation and re-handshake latency floors. v1.0 enforces all four including the `cert_post_grace_reject` ≤0.1% rate.

**Why "zero data-plane errors" is achievable, not aspirational.** The pre-staged-overlap procedure (§7.2.1.a) absorbs all transient cert-mismatch errors into client-side retry — invisible to the application. Post-grace `cert_post_grace_reject` events are intentional rejections of straggler clients with stale certs, logged separately and not counted as data-plane errors. The "zero" floor refers to client-visible request failures, not handshake retries.

### 7.3 Transparency Log

Per-Host SQLite append-only log. Every IAC frame, every capability invocation, every lifecycle transition lands in the log before delivery (I2). Default retention: 90 days private tier, configurable per-deployment; Merkle-root anchoring optional for tamper-evidence in regulated deployments.

**Audit query surfaces.** Four complementary primitives:

| Surface | Stakeholder | Primitive |
|---|---|---|
| `audit query` | Internal auditor / SRE | Frame-by-frame log query with replay (covered by `log.recall`) |
| `audit subject-access` | DPO / data subject | Subject-indexed query — "show me everything about data subject X across all Spirits and Hosts." Indexes on PII tags in IAC frames; respects redaction policy |
| `audit posture-delta` | CISO / security operations | Posture-drift query — "what capability scopes / sandbox tiers / consent policies have changed across Spirits in the last 30 days, and what was the approval chain" |
| `audit sealed-export` | External auditor | Cryptographically sealed audit bundle — Ed25519-signed by the operator's audit key, third-party-verifiable; not raw log. Includes Merkle anchoring if enabled |

**Right-to-be-forgotten (GDPR Article 17).** Per-Spirit private memory is removable on operator command (`maos forget --principal <id>`). The Transparency Log is not removable (it is the audit spine), but personally identifying payloads in the log can be redacted via `maos audit redact --frame <id> --reason <legal-hold>`; redactions are themselves logged. Cross-Spirit cascade: forgetting cascades to working-memory references in other Spirits where principal data was shared; distillates containing principal data are marked redacted with re-distillation triggered. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries.

**Replay determinism.** Determinism is over the **shape of the trace** — IAC frame ordering, capability-token issuances, halt events, decision-frame emission — NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders carrying the same structural shape. The trace-shape contract is specified in `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12); the schema is validated in CI. v1.0 is best-effort; v1.5 is the hard target.

### 7.4 Notification UX (kernel-rendered)

Three notification levels — `immediate`, `queue`, `digest`. **These are kernel-rendered, not Spirit-rendered.** A Spirit cannot bypass the user's notification policy by emitting a different kind of event; the kernel intercepts every IAC frame whose recipient is the human and routes it through the configured notification surface.

Surfaces: TUI, editor (ACP), browser, mobile push (HTTP push at v1.0; native push at v1.5+).

**Approval Decision Log distinct from Transparency Log.** Full intent + decision + reasoning chain per Invariant I4. Both logs are queryable via the control-plane API; both can be exported for compliance.

### 7.5 ACP / MCP

**ACP (Agent Communication Protocol).** NDJSON over stdio for editor-hosted Spirits. v1.0 ships with Zed + VSCode tested. JetBrains via plugin-bridge at v1.5.

**MCP (Model Context Protocol).** All-three-transports MCP client (stdio / SSE / Streamable HTTP). Streamable HTTP is the default for Loom-lite, the Spirit registry, and most production tool servers. Tool-side WASM sandboxing for untrusted MCP tools is not in this version; trusted MCP tools run at their declared sandbox tier.

**Four-protocol commitment.** Kernel-internal IAC + bilateral A2A + ACP + MCP. The substrate invents no new wire protocols. A fifth protocol requires (a) a use case unsatisfiable by IAC + adapter, (b) a new ADR, (c) demonstration that adding the protocol does not violate kernel-stays-small.

## 8. Security & Approval Model

### 8.0 Non-negotiable testability floors

The seven floors below cannot be deferred, weakened, or staged without a major-version bump (v0.x → v1.x → v2.x) and an ADR documenting the regression rationale (per ADR-037 `invariant-lock` gate). Cadence and tooling around each floor may evolve; the floors themselves are versioned commitments.

1. **§8.1 red-team corpus** — N=80, full taxonomy across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), every Spirit, every release. ≥9/10 per class detected/blocked, ≥72/80 aggregate, 0 unmitigated category. Pre-frozen, content-addressed, externally authored.
2. **§5.2 wire-protocol fuzz** — per-target floor ≥72 CPU-hours per fuzz target across the 90 days preceding GA; aggregate floor ≥1,000 CPU-hours pre-GA across all targets. Zero crashes / zero auth bypasses / zero TLS downgrade paths on T3. Tiered cadence (T1 10-min per-commit / T2 4h nightly / T3 24h pre-release) is the execution model; per-target + aggregate floors are the gate. (The earlier "≥168h cumulative" framing was a single-bucket aggregate floor; the per-target rewrite in §5.2 supersedes it because aggregate-only allows one well-fuzzed target to mask several under-fuzzed siblings.)
3. **§8.1 secret leakage** — 0% on per-commit canary (10⁴ synthetic secrets), 100% detection on quarterly audit (10⁵ + adversarial mutations), p95 ≤24h discovery latency on production canary (1000 live tokens/month).
4. **§8.5 ComplianceClaim cross-Spirit agreement** — ±2% agreement floor on whatever corpus is current for the version (see App-E staging table). v0.5 calibration ±5%, v0.9 ±2%, v1.0 ±2% active + ≤0.5% drift quarter-over-quarter.
5. **§7.2 mTLS rotation chaos** — quarterly forced rotation under live load; revocation latency median ≤60s, p99 ≤5min; zero data-plane errors during rotation.
6. **§6.3 halt precision-recall** — halt-precision ≥0.85, halt-recall ≥0.95 on labeled corpus N≥150 per safety-critical Spirit. Asymmetry (recall > precision) encodes the cost model: missed halt is unrecoverable, over-halting is operator-resumable.
7. **§8.1 isolation corpus** — 200 scenarios, no Spirit-to-Spirit info leakage. Sec-14a (same-Host: namespace, seccomp, capability-token forgery) and Sec-14b (cross-Host: A2A frame injection, mTLS pinning, replay) per ADR-040.

**Rule against drift.** Staging refers to delivery cadence and *enforcement onset*, not to whether a floor exists. Every floor in this section has a defined value at v1.0; some floors enter advisory reporting earlier and graduate to enforcement at a specified version (e.g., §8.5 ComplianceClaim ±2% agreement: advisory at v0.5 per the App-E v0.5 non-degeneracy criterion, enforced at v0.9). A floor whose value is undefined at v1.0 is not a floor — it is an aspiration and must be removed or rewritten. A floor whose enforcement is deferred past v1.0 is also not a floor — staging cannot push enforcement past the version where the floor is claimed to exist.

**Corpus-pending floors.** Floor 6 (halt precision-recall) and Floor 4 (CCAC ±2% agreement) are corpus-dependent — the floor value exists from v0.5 onward as advisory, but enforcement requires the corpus itself, which is itself a staged deliverable. Until the corpus exists, the floor is reported but not gated. See §6.6 and App-E for the per-corpus staging detail.

### 8.1 Threat model

The substrate is designed against the following:

| Threat | Mitigation |
|---|---|
| Compromised LLM provider returning malicious tool-call args | Sandbox tier on every exec; arg validation at Capability Registry; approval prompts on `exec_capable` and `mutating` |
| Compromised MCP server running arbitrary code on the Host | Container sandbox (T3) for less-untrusted MCP; allowlist for first-party MCP |
| Prompt-injection via tool output (e.g., a search result containing instructions) | Output redaction at the Transparency Log boundary; explicit "tool output is data, not instructions" framing in system prompts; intent-vs-source mismatch detection |
| Compromised peer Host in bilateral A2A pair | mTLS + TOFU pin verification at every connection; explicit consent envelope on every frame; per-frame intent allow-list |
| Spirit escalating its own posture beyond manifest ceiling | Posture changes are kernel-managed; manifest sets a hard ceiling; posture restricts only |
| Spirit reading another Spirit's private memory | Memory Manager namespace enforcement (I5) |
| Spirit silently exfiltrating data | Transparency Log (I2) — every IAC frame logged before delivery; user can audit |
| Approval prompt fatigue → user clicks through | Approval batching with explicit scope; `prompt_with_diff` makes the cost of cleanup visible before approval |
| Capability token replay | Tokens bound to (Spirit-PID + boot-nonce + expiry); re-validation at use against current state (TOCTOU correctness) |
| LLM jailbreak via adversarial input (paste-into-context) | Input provenance tagging at IAC frame creation; intent-vs-source mismatch detection; epistemic halt on adversarial-intent indicators |
| Capability-token leak via logs / digests / distillates | Pre-write secret-redaction filter at the Transparency Log boundary (universal to all logged frames); §9.5 fifth metric `digest-secret-leakage = 0%`; production canary system catches leaks at runtime |
| Provider supply-chain compromise | Response cross-validation across providers for high-stakes decisions (Orchestrator may dispatch the same task to two Workers on different providers and compare digests for divergence); provider-driver integrity checks at MAOS startup |
| Sandbox-escape via syscall-pattern divergence | Anomaly detector on top of Landlock/seccomp (syscall-pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections); structural alarm only — kernel does not classify intent |

The substrate does **not** design against:

- Ring-0 host compromise (out of scope for the kernel; that is the OS's job).
- Side-channel timing attacks against the LLM (out of scope).
- Adversarial model fine-tunes producing specific bad behaviors (out of scope; framework cannot fix model alignment).

**Adversarial-Spirit red-team corpus.** 80-scenario corpus across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), N=10 per class. Floor: ≥9/10 per class detected/blocked by kernel; ≥72/80 aggregate; 0 unmitigated category. Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed.

**Cross-Spirit memory isolation corpus.** 200-scenario adversarial corpus where Spirit-A actively attempts to enumerate, read, side-channel, or timing-attack Spirit-B's substrate state. Categories: namespace enumeration, working-memory read-across, decision-frame observation, halt-signal observation, transparency-log cross-read, working-memory-digest cross-read, capability-token forgery cross-Spirit, sandbox-escape lateral. Split into Sec-14a (same-Host attack vectors) and Sec-14b (cross-Host bilateral attack vectors). Floor: 200/200 isolation maintained; any leak = P0 ship-blocker.

### 8.2 Sandboxing — re-cap of §4.3.1

OS-native primitives per Spirit form and trust tier. For v1.0:
- Linux: bwrap + Landlock + seccomp inside Docker for T3; Landlock + seccomp narrow for T2.
- macOS: Seatbelt with `.sbpl` profiles. Codex's `seatbelt_base_policy.sbpl` and `seatbelt_network_policy.sbpl` are the prior art.
- Windows: restricted-token sandbox + Job Object resource constraints.

**Strictest-of-(manifest, trust-tier, operator-policy) floor.** A `public-untrusted` Spirit declaring T0 in its manifest is forced to T2 by the trust-tier floor.

### 8.3 Approval class taxonomy — re-cap of §4.3.3

`readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Lifted from openclaw because it is the most expressive taxonomy in the survey and because openclaw has already proven it scales to 100+ tool types.

### 8.4 Audit

The Transparency Log is the personal audit trail. The Approval Decision Log is a separate kernel-managed SQLite table that records every approval prompt's `(actor, target, capability, intent, decision, reasoning_if_any)`. Both logs are queryable via a control-plane API; both can be exported for compliance.

Both logs additionally stream to OpenTelemetry endpoints when configured, for integration into the operator's existing observability stack.

### 8.5 ComplianceClaim envelope

A first-class kernel object: `ComplianceClaim`. Ed25519-signed by an attesting party, references an *execution-context fingerprint* — the precise tuple of (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity) under which the claim applies.

The kernel verifies `ComplianceClaim` envelopes at admission time and refuses to load Spirits whose runtime context drifts from the attested context (typed error `EComplianceContextDrift`). This makes attestations falsifiable rather than marketing copy.

**Schema location.** The ComplianceClaim schema is defined in [`maos-spirit-abi/src/compliance.rs`](../crates/maos-spirit-abi/src/compliance.rs); any change to its wire shape bumps `ABI_VERSION`. The structural validator and emit pipeline live in `maos-core::compliance` and `maos-core::pipeline` respectively, both shipped at v0.1. The semantic evaluator (principle engine, N=600 corpus, ±2% agreement target) lives in `maos-compliance` and ships at v0.9 — see **App-E "v0.9+ Compliance Roadmap"** for the staging table, generation mechanism (Mechanisms A/B/C), and per-phase ship-blocking gates.

**v0.1 ship-blocking surface (binding here).** The schema is frozen, the structural validator is implemented, the emit pipeline is live on every Spirit decision. Schema validation 100%, emit-rate 100%. No semantic eval, no corpus, no agreement floor — those are App-E.

**ABI break rule.** Adding any required field, removing any field, renaming, type-changing, or removing/reordering enum variants of `Verdict` / `PrincipleRef` / `EvidenceKind` bumps `ABI_VERSION`. Adding optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`, additive enum variants at the end with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback, or loosening bounds — does NOT bump.

### 8.6 Pluggable crypto provider

The kernel's cryptographic operations (signing, mTLS, secret encryption) are mediated by a `CryptoProvider` trait with a default implementation (`ring` / `rustls` / equivalent). Alternate implementations can be swapped at composition root for FIPS 140-3-validated module compatibility, hardware-backed crypto, or air-gapped deployments using on-prem HSMs. v1.0 architectural commitment: the seam exists; specific FIPS modules are downstream distributor concern.

### 8.7 Constitutional substrate evolution

Architecture Decision Records (ADRs) are the substrate's evolution mechanism. ADR amendments touching invariants I1–I14 require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update — all three enforced by CI gate `invariant-lock`, not by founder discipline.

The "no fifth protocol unless..." commitment is enforced architecturally — adding a fifth wire protocol requires a passing `invariant-lock` gate plus a new ADR with two-reviewer sign-off. The kernel's evolution is bounded by structurally-enforced governance, not personality.

## 9. Memory & Knowledge

### 9.1 The three tiers — re-cap

| Tier | Scope | Backed by | Lifetime | Use case |
|---|---|---|---|---|
| `private` | This Spirit instance | `Arc<RwLock<HashMap<...>>>` per-Spirit + `fs.write` to per-Spirit-namespaced filesystem | Spirit lifetime + episodic persistence if declared | Working memory, scratchpad, session state |
| `shared` | All Spirits on this Host | SQLite-backed key-value with namespace prefix per writer | Host lifetime | Cross-Spirit coordination on this Host (founder-loop Orchestrator-Worker handoff, peer telemetry sharing) |
| `collective` | Both Hosts in the bilateral pair | Postgres+pgvector exposed via MCP-Streamable-HTTP (Loom-lite) | Loom domain lifetime | ADR-pattern library, fix templates, regression-test references |

### 9.2 Memory file (`memory.md`)

Spirits MAY persist a `memory.md` file in their private namespace as a human-readable working memory dump. The `*.md` memory file convention is universal in the cohort (codex / openclaw / ironclaw / hermes / paperclip all use a similar pattern). It is the user's lever to read what the Spirit "remembers" and to edit it. The kernel does not interpret the file; it stores it like any other private-tier write.

### 9.3 Loom-lite — the collective tier

**Single-instance Postgres+pgvector, exposed as an MCP-Streamable-HTTP server.** The collective tier is a service the operator deploys; the kernel mediates access but does not host the data. Used by:

- The Diagnostic Engineer + Senior Architect bilateral pair for ADR-pattern lookup, fix-template retrieval, regression-test reference. Mira reads patterns to recognize known incident shapes; Nash writes new patterns when an incident produces a reusable diagnostic recipe.
- Distillation-shipping Spirits in the founder loop (Orchestrator) for cross-session digest retrieval if the operator opts in.
- Researcher for cross-session bibliography persistence.

**Schema.** The collective tier carries `kind: pattern` records (ADRs, fix templates, regression tests) with vector embeddings for similarity search. Curation is Spirit-side (Nash decides what is worth persisting); the kernel only enforces the I11/I12/I13 audit-chain invariants on writes.

**Backup / DR.** RPO ≤1h, RTO ≤4h; backup integrity verified weekly via Merkle-root cross-check.

### 9.4 Memory hot-swap

The Memory Manager swaps memory scope along with Spirit class (I6). Private memory is preserved through `swap()` (the swapping-in Spirit inherits via `on_swap_in`'s `predecessor_state` argument). For `migrate()` (cross-host bilateral), private memory is serialized into the migration payload; the receiving Host's Memory Manager rehydrates on `on_swap_in`.

`forgotten_set` semantics on swap-out: a Spirit may declare per-key TTLs in its manifest's `[memory.forgotten_set]` block; the Memory Manager garbage-collects expired keys on swap.

### 9.5 Distillation Pattern (substrate-level) — interface sketch

Spirits that aggregate from many peers — Orchestrator running an epic loop, Mira ingesting telemetry, distillation-shipping Spirits in any topology — face naive-append context overflow. The substrate's answer is a **documented pattern** built on kernel primitives, not a kernel feature. The kernel provides primitives (Transparency Log + I11 + I12 + I13 + `log.recall`); Spirit authors compose the pattern.

**Substrate interface (binding-v0.5).** Five contracts the kernel honors so the pattern works:

1. **Raw lands in Transparency Log first** (I2 — kernel writes log before any IAC delivery).
2. **Digest writes carry `source_log_ref` + `distillation_depth`** (I11 — `EDigestAuditChainMissing` on missing fields).
3. **Decision frames carry `working_memory_digest_refs`** (I12 — kernel attaches refs from declared in-context digests + shadow-recall on inbound events).
4. **Digest writes carry `intent_lineage`** computed kernel-side from input frames (I13 — `EIntentPromotionDenied` if consumer's `allowed-promotion-set` does not contain the digest's lineage).
5. **`log.recall(filter)`** is the on-demand raw retrieval API; calls are auditable.

**Acceptance floor (v0.5 ship gate, all distillation-shipping Spirits) — Table 9.5-1:**

| Metric | Floor |
|---|---|
| Digest-recall | ≥0.90 |
| Digest-faithfulness (no-contradiction) | ≥0.98 |
| Digest-hedge-preservation | ≥0.95 (requires IAA ≥0.85 gold corpus) |
| Digest-traceability | 100% (kernel-enforced via I11) |
| Digest-secret-leakage | 0% (kernel-mediated pre-write redaction) |

For the derivation of these floor values from the threat model and observed operational data — including why each metric was chosen and how to re-derive the thresholds when the threat model changes — see Appendix F.5.

**Implementation prose** — Spirit-author conventions (first-turn / last-turn anchoring, target token budget, compressor model class, hermes-agent reference implementation) and the full step-by-step pattern walkthrough live in **App-F "Distillation Pattern Body"**. The kernel does not enforce conventions; it enforces the five contracts above.

## 10. Journey Traceability

The substrate ships in support of **six user journeys at v1.5** (J0 Evaluator, J-Butler, J-Researcher, J1 Founder Loop, J4 Mira-Nash bilateral, J6 Diego). Each ships at a specific phase, with cited architectural primitives, FRs, and NFR floors. The substrate's architectural commitments are exactly the ones these journeys exercise.

**Two additional PRD journeys are deferred to future milestones (v2.0 / v2.x):** J3 Marcus Team Nexus (8-Host peer mesh) and Reza Cortex (single-org cross-team, multi-tenant Loom + WASM Spirit form). Their canonical scene descriptions remain in the PRD until the architecture revisions that support them ship; the v1.5 substrate is designed so neither journey requires architectural rewrite to add later. Full deferral notes including readiness analysis are in **§10.7 Deferred journeys (future milestones)**.

**§10.0 — Reading note for sequence flows.** Journey narratives below describe interactions between **Spirits** and the **kernel**. Per §4 the kernel is exactly *five services + two internal modules* (see §4.0.8 for the operational definition). When a narrative says "the kernel routes the frame," the routing is performed by the IAC Bus service; when it says "capability mediation," the work happens in the Capability Registry service; when it says "the Spirit observes telemetry," the subscription lives in the Telemetry Stream internal module. Sequence-level granularity (which kernel service did what, in which order) lives in the relevant service subsection — see §4.6.1 for halt sequencing, §7.2 for mTLS handshake choreography, §4.5 for IAC mailbox routing. §10 deliberately stays at the Spirit-↔-kernel layer to keep the journey legible; readers needing per-service step-by-step should follow the service-subsection cross-references inline below.

### 10.1 J0 — The Evaluator

**Persona.** Anonymous developer, four minutes into `cargo install maos`, deciding whether minute five happens.

**Phase.** v0.1 (the foundational gate).

**Architectural primitives exercised.** Kernel boot, lifecycle journal, capability tokens, single-Spirit subprocess form, `hello-spirit` placeholder Spirit, `maosctl` basic (`install`, `uninstall`, `audit query`, `spirit invoke`), clean uninstall, accessibility flags (`--plain`, `NO_COLOR`, `TERM=dumb`), `SECURITY.md` + disclosure pipeline.

**Acceptance.** From install completion to first useful Spirit response within 5 minutes. Honest capability disclosure on first interaction (the Spirit introduces what it *can* and *cannot* do). Audit-from-minute-1 (`maosctl audit query` works on the local Transparency Log). Clean uninstall (kernel is reversible; user's data persists or is removed per their choice).

### 10.2 J-Butler — Sandra's Butler reads the third 6 PM and reclaims dinner

**Persona.** Sandra, designer at a 30-person SaaS company. Figma + Linear + Slack + Google Calendar. Time-zoned across her team. Has a recurring 7 PM dinner with friends she keeps missing. Her partner is starting to take it personally.

**Phase.** v0.3 (the anticipatory single-Spirit anchor).

**Opening scene.** Tuesday, 5:48 PM. Sandra is mid-flow on a Profile Page wireframe. The Butler Spirit's `on_idle` lifecycle hook fires. Beliefs: Calendar shows 7 PM dinner T/Th, last 4 occurrences 2 attended late + 2 missed entirely (pattern detected); Slack status "Heads down" since 1 PM; Figma continuous active session for 4h 40m; predicted disengage time at current pace ≈7:28 PM; probability the dinner gets missed: 0.78; confidence 0.85.

The Butler computes a calendar-conflict-confidence scalar (its own definition — predicted-miss-probability × pattern-strength) and writes it via `working_memory.set_scalar("user.calendar_conflict.confidence", 0.85, derived_from=[calendar_obs, slack_obs, figma_obs])`. Per Butler's `[epistemic_policy]` rule — `tag = user.calendar_conflict.confidence, on_value_above = 0.8 → action = verbalize_with_options` — the kernel reads the scalar, compares to threshold, fires the action. **The kernel does not compute confidence; it compares the Spirit-supplied scalar to the threshold via universal arithmetic.**

The Butler surfaces a single notification through the kernel-rendered notification surface: pattern noticed, predicted disengage time, partner's unanswered message at 4:15, three options offered. Sandra picks (a) snooze the wireframe for 75 min. The Butler's follow-up writes a Linear note via MCP, sets a calendar reminder for 6:55 PM, archives the suggestion-acceptance signal to its `notification_acceptance_log`. Sandra arrives at dinner at 7:08 PM.

**Resolution (three weeks later).** The Butler observes that its **posterior uncertainty** over Sandra's preferred prompt-sensitivity has grown beyond threshold. The Butler computes its belief variance using its own definition (Shannon entropy over the work-mode-conditioned acceptance distribution) and writes via `working_memory.set_scalar("self.belief_variance", 0.78, derived_from=[14-day notification log refs])`. Per its `[epistemic_policy]`: `tag = self.belief_variance, on_value_above = 0.7 → action = halt`. The kernel compares the Spirit-supplied scalar (0.78) to the threshold (0.7) and fires the halt. Sandra resolves with `provided_context: split the policy by work-mode-context and lower sensitivity for shallow-work mode`. The Butler is self-tuning under human supervision — *upstream of* outcome degradation, not in response to it.

**Architectural primitives exercised.** Single-Spirit subprocess form. `on_idle` lifecycle hook as substrate for anticipatory reasoning. Telemetry Stream narrow per-Spirit subscription (Calendar/Slack/Figma topics). `[epistemic_policy]` per-tag rules with `verbalize_with_options` / `verbalize_only` / `flag` / `halt` actions. Output_shape predicate enforcement at the Capability Registry. MCP tool integrations (Calendar / Slack / Linear / Figma). Posture-shift command at runtime. Self-tuning via `epistemic.halt`. Transparency Log capturing every notification, every user response, every `[epistemic_policy]` update.

**Acceptance.** Notification precision ≥0.85; notification recall ≥0.7; halt-precision ≥0.85 on a 30-scenario calendar/comms behavior corpus; self-tuning halt fires in ≥9/10 synthetic acceptance-rate-decline scenarios within a 14-day rolling window; 4 MCP servers × 5 representative ops × 3 outcomes (success / scope-violation / network-error) = 60 integration tests, 100% pass.

### 10.3 J-Researcher — Hannah surveys the LLM-as-judge literature in 2 hours

**Persona.** Hannah, ML researcher at a 50-person AI safety lab. Surveys emerging research, proposes hypotheses, hands off to senior researchers. Reads ~30 papers/week. Currently overwhelmed — the LLM-evaluation field is publishing 200+ relevant arXiv submissions per six-month window.

**Phase.** v0.5 (the exploratory single-Spirit anchor; first opt-in distillation deployment).

**Opening scene.** Monday 9:00 AM. Her director: *"How is the field handling LLM-as-judge bias? I want to know by Wednesday."* Hannah has 2 hours of focused time before 11 AM standup, then meetings until 4 PM. The deliverable: structured findings, a confidence map, and a recommendation for which threads warrant a senior researcher's deeper investigation.

She invokes her Researcher Spirit:

> `@researcher survey LLM-as-judge bias methodology, last 12 months. Focus: detection, mitigation, positional bias. Output shape: findings + open questions + confidence map. Time budget: 90 minutes.`

For 73 minutes the Researcher fans out. Adaptive-chunk-ratio summarization keeps each paper's digest under 4K tokens. Citation-graph traversal identifies four tight clusters of related work. The Spirit reads abstracts for 40 papers, full intros for 18, full methods for 8.

At minute 73 the Researcher hits an `epistemic.halt` on `claim.methodology_strength`: two papers report contradictory findings on positional bias in pairwise judgment, both with strong methodology by the Spirit's scoring rubric (≥0.85). The Spirit lacks grounds to rank them. Three resolutions offered. Hannah picks (a) Surface both in findings + mark contradiction as Open Question. The halt resolution + reasoning is journaled (I10). The Spirit resumes.

At 10:38 AM the Researcher delivers a structured output that the kernel's Capability Registry validated against the manifest's `[output_shape]` predicate (rejection on missing fields): 14 ranked findings with citations and per-finding confidence scores; 6 Open Questions including the contradictory-finding pair flagged with both citations side-by-side; color-coded confidence map; bibliography of 38 papers.

**Architectural primitives exercised.** Single-Spirit subprocess form. Broad MCP capabilities (web search, arXiv search, GitHub search, citation-graph traversal). High parallelism in tool dispatch (manifest-declared `[capabilities.parallelism] = 8`). Posture: `survey-mode`. Output_shape predicate enforcement. Adaptive-chunk-ratio summarization. `[epistemic_policy]` Spirit-detection-triggered halts (the Researcher computes its own contradiction-detection score, low-confidence-high-impact-product, methodology-strength-tie indicator; writes each as a tagged scalar; kernel fires halt when scalar crosses Spirit-author-declared threshold via universal-arithmetic comparison). Time-budget enforcement. `log.recall` for raw-payload retrieval. Distillation pattern §9.5 first opt-in production use in a single-Spirit context. Five-metric gate applies.

**Acceptance.** Researcher-specific corpus: 25-scenario `bmad-technical-research`; ≥3 sources cited with ≥80% reachable URLs; halt fires on ≥9/10 scope-creep injections; output_shape predicate satisfied 100%. Distillation five-metric gate (single-Spirit Researcher context): all floors per §9.5 + 10⁵ secret-leakage corpus.

### 10.4 J1 — The Founder's Loop

**Persona.** Lunarpulse, founder. Heavy user of AI coding agents. Two kids, a marriage, a startup, and a workflow that ostensibly automates coding but in practice has chained him to the laptop because every cycle requires his approval, his next-prompt, his clarification. Each "wait" is 4–30 minutes. Each "approve" is one keystroke. The asymmetry is grotesque.

**Phase.** v0.9 (the wedge demo; multi-Spirit Orchestrator+Worker on a single Host with A2A loopback).

**Opening scene.** Tuesday, 4:17 PM. Epic 7 has eight stories. He's two stories in. He has not had an unbroken hour of thought today. He spawns the Orchestrator Spirit — a `claude` process loaded with `orchestrator-bmad` and `maos-bridge` skills, posture `autonomous-with-halt`, halt policy preferring recall over precision. From his terminal:

> `@orchestrator run epic-7. workers: developer-local, developer-remote (laptop in office); reviewer-local. halt-recall over halt-precision. wake me when in doubt.`

The Orchestrator reads `_bmad-output/planning/epic-07/`, builds the dependency DAG, and emits its first delegation — a natural-language `task.assign` IAC frame routed by the kernel-internal IAC bus to a local Developer Spirit (another `claude` process with `developer` + `maos-bridge` skills loaded). In parallel, Story 7-3 goes to a remote Developer Spirit on Lunarpulse's office laptop — the kernel's A2A adapter ferries the same-shape `task.assign` frame across mTLS, validates the typed-intent consent envelope (`intent: task.assign / development-task` is in the remote Host's allowlist), and the remote Spirit (an opencode session, Gemini provider) loads `bmad-dev-story` from its own filesystem mirror and executes. Different host, different CLI, different provider, same protocol.

**Distillation in action.** When the local Developer reports `task.complete` for Story 7-1, the result payload is large — full diff (3,200 tokens), test output (800 tokens), reasoning trace (2,400 tokens). The kernel writes the full payload to the Transparency Log (I2). The Orchestrator's bridge logic invokes its distillation step: an LLM call summarizing the payload into ~150 tokens. The digest is persisted to episodic memory tagged `kind: digest` with `source_log_ref: [task-complete-frame-id]` and `distillation_depth: 1` — kernel validates per I11 and accepts. The digest joins the Orchestrator's active LLM context. Raw payload sits in the log, recallable via `log.recall` if a future decision needs it.

**Halt with non-blocking continuation.** At 6:23 PM the Orchestrator hits Story 7-5 — acceptance criterion reads "system handles concurrent users gracefully," undefined. Orchestrator halts (`tag: story.acceptance_criterion.ambiguous`). The halt frame is itself a `decision.*` type, so the kernel attaches `working_memory_digest_refs` per I12 — the auditor can later prove which digests the Orchestrator was reasoning over when it halted. Lunarpulse, mid-bedtime-routine, sees the halt notification on his phone with three candidate operationalizations. He picks one. The Orchestrator continues with Stories 7-3, 7-6, 7-7 in parallel via three Developer Spirits while the AC clarification is processed.

**User-input queuing.** Around 5:50 PM Lunarpulse had typed: `also remind the reviewer to check that we're not regressing the OAuth flow in PR-1213`. The Orchestrator's bridge enqueued this without interrupting in-flight Workers. At 6:08 PM, when Worker-1's `task.complete` landed and the Orchestrator was at a decision point, it dequeued the user input, folded it into the next Reviewer dispatch, and continued.

**Resolution.** He closes his laptop at 8:40 PM. The Orchestrator continues. He goes to a 9 PM dinner with his wife — first one in three weeks. He wakes at 6 AM, opens the morning digest: 7 of 8 stories merged. Story 7-5 paused at second halt. Retrospective draft attached. 22 IAC frames over 14 hours. 4 halts (3 resolved, 1 pending). 0 invariant violations. 0 capability scope breaches.

**Architectural primitives exercised.** Multi-Spirit IAC bus (kernel-internal Spirit↔Spirit communication on the same Host). A2A peer at the loopback profile (`127.0.0.1`-bound, mTLS with self-signed certs, TOFU pinning). Two-level `task.assign` typed-intent IAC primitive (human → Orchestrator at epic granularity; Orchestrator → Worker at story granularity). Typed-intent consent enforcement on cross-host A2A frames. Token-budget accounting with `ContextPressure` / `ContextLimit` / `EContextExhausted` typed frames. Hot-swap state-transfer wire format. ADR-022 failure-semantics floor. Skill-package overlays (`orchestrator-bmad`, `developer`, `reviewer`) loaded into agent CLI processes via `CliWrapperSpirit`. Full distillation pattern §9.5 multi-Spirit deployment. Multi-CLI Worker parallelism (3+ concurrent). Halt-recall and halt-precision benchmarks per Spirit class published in the Spirit registry. Cross-Spirit memory isolation corpus (200-scenario same-Host adversarial) as gate before the v1.0 hermes-tenant positioning sentence is allowed in marketing.

**Acceptance.** A2A loopback floors: mTLS replay corpus 100/0; TOFU pin-mismatch 100/100 detected/rejected/logged; handshake-fault 20/0. Multi-Spirit IAC consent corpus 30 scenarios (100% disallowed blocked, ≥95% allowed succeed, 0 envelope-type confusion); revocation propagation ≥29/30. Orchestrator Epic-corpus: 5 epics, ≥4 complete without halt-storm; halt-precision ≥0.85. Planted-issue corpus: 50 synthetic stories; Orchestrator surfaces ≥42/50 (84%); ≥45/50 on security-relevant subset. Ambiguous-AC corpus: 10 stories; halt fires in ≥9/10 within 2 frames.

### 10.5 J4 — Elena's 2 AM 90-minute Mira-Nash incident

**Persona.** Elena, VP Engineering at a fintech processing 40,000 transactions per minute across 3 regions and 60+ microservices. Runs two MAOS Spirits in production-adjacent positions: **Mira** (diagnostic Spirit on a production-edge node; read-only on production runtime, write only for revert/scale/flag-toggle) and **Nash** (architect Spirit in the dev environment; full source repo access, CI/CD orchestration). They are bilateral peers across hosts; neither owns the other.

**Phase.** v1.5 (the diagnostic-architect bilateral pair anchor).

**Opening scene.** 2:13 AM. Elena's phone lights up. PagerDuty alert: `payment-service` Kafka lag rising 340%. Before she's awake, the alert is also a MAOS notification — Mira has been triaging for 9 minutes already.

**Mira's diagnosis.** At 02:22: deployment of `billing-v3.2.1` at 01:41 UTC introduced `BatchInvoiceProcessor.java` which creates a DB connection per message instead of using the pool. Confidence: 0.82 (above the diagnostic Spirit's halt threshold of 0.6 for `diagnosis.root_cause`). Mira escalates to Nash via cross-host A2A — the kernel's A2A adapter validates the typed-intent envelope: Mira declares `intent: diagnosis-handoff:read-only-evidence`, which Nash's consent policy permits from prod-edge peers; Nash's policy explicitly excludes `remote-write-request` and `code-mutation-directive`. The frame lands. Nash receives a structured diagnosis, not an instruction.

Nash pulls `BatchInvoiceProcessor.java`, confirms instantly, then expands the search: pattern detection across the codebase finds two more dormant instances (`BatchRefundProcessor.java`, `DailyReconciliationJob.java`). Nash queries Mira: *"are the dormant ones showing latency increase in 72-hour telemetry?"* Cross-environment query, kernel-mediated, audit-trailed. Mira queries 72 hours of metrics. `DailyReconciliationJob` shows month-end spikes. Time bomb. Recommend: fix all three. Confidence 0.94.

**Distillation at the diagnostic edge.** Mira's incoming telemetry stream is high-volume. Per the §9.5 pattern, raw telemetry frames land in the Transparency Log; Mira's Spirit-side distillation step compresses 72 hours of cross-service metrics into ~200-token decision-relevant digests per service. Each digest carries `source_log_ref` to the underlying telemetry frames and `distillation_depth: 1`. Mira's working memory holds the digests; raw is recallable via `log.recall` for any downstream decision.

**Climax.** Elena wakes at 2:47 AM to one notification:

> 🔴 **mira + nash: Coordinated response ready**
> Issue: Kafka lag spike (detected 02:13, diagnosed, fixed)
> Root cause: Connection pool bypass in 3 batch processors
> Proposed: Rollback (Mira) + Deploy PR #1247 (Nash, tested, ADR-112 attached)
> Trail: Mira detected → Nash confirmed + found dormant bugs → Mira confirmed via 72hr telemetry → Nash produced PR + ADR + regression test

The notification carries decision-context refs (I12) — Elena can see exactly which Mira digests Nash reasoned over and which Nash digests Mira used to validate the 72-hour telemetry query. She approves from her phone. Mira executes rollback. Nash's PR ships through CI/CD. Total elapsed: 2:13 to 3:43 AM. **90 minutes. Elena touched the system once.**

**Resolution.** Three weeks later, Mira sends Nash an unsolicited architecture note: *"0 new violations of ADR-112 in 23 deployments. Regression test caught 1 attempt at CI. New observation: 14% of batch processors exceed pool timeout during month-end — investigate batch sizing?"* Nash opens an investigation. The cycle continues.

**Architectural primitives exercised.** Asymmetric capability postures via manifest (sre-diagnostician vs principal-architect). Cross-Host bilateral A2A with mTLS + TOFU + ADR-012 typed-intent consent envelopes — Mira's `diagnosis-handoff:read-only-evidence` is allowed; `code-mutation-directive` is blocked. Cross-environment telemetry queries with audit trail. Per-tag epistemic policy at both Spirits with Spirit-detection-triggered halts. Mobile-friendly approval surface. ADR auto-drafting and CI-enforced pattern detection (Loom-lite ADR-pattern library). Distillation pattern at the production edge — high-volume telemetry compressed to decision-relevant digests; raw recallable via `log.recall`. I12 decision-context recording across cross-host A2A. Capability-token TTL ≤60s for high-privilege operations; bound to Spirit-PID + boot-nonce. mTLS cert rotation chaos test passes at bilateral 2-host scale.

**J4-extended (planned, v1.5 deliverable).** The 50-scenario incident-response corpus described above is the v1.5 *baseline*. A planned **J4-extended halt-continuity corpus** (≥30 hot-swap-during-incident scenarios + mutation/fuzzing infrastructure) extends J4 to specifically probe halt-continuity (I14) under adversarial agent-handoff timing — this is the substrate behind the §3.2.1 cadence promotion of I14 from `runtime` to `fuzz` at v1.5. The two corpora share the J4 incident set but exercise different invariants: the 50-scenario baseline probes Mira/Nash bilateral coordination; J4-extended probes hot-swap correctness *during* a coordination event. See §3.2.1 and §13 v1.5 row.

**Acceptance.** J4 Mira-Nash 90-minute loop reproducible on 50-scenario synthetic prod-incident corpus; ≥45/50 close in ≤90 min; ≥48/50 uphold typed-intent consent envelope. Loom-lite query latency: P99 ≤200ms on hot path. Halt-recall and halt-precision per Spirit class meet floors (recall ≥0.7, precision ≥0.85).

### 10.6 J6 — Diego, the third-party Spirit author

**Persona.** Diego runs an open-source code-review tool with ~3,000 GitHub stars. He wants to make it agentic but doesn't want to build a substrate. Reads the MAOS announcement post; the phrase *"a third party authors and ships a Spirit independently of the MAOS source tree"* catches his eye.

**Phase.** v1.0 (third-party Spirit ecosystem opens; Spirit registry with Ed25519 signing).

**Opening scene.** Diego opens `spirit-development-and-sharing.md`. Skims to *"Build your first Spirit in 30 minutes."* Runs `cargo generate maos-spirit`, gets a templated project. Imports his existing static-analysis logic.

**Manifest.**
- `class = "code-reviewer-pro"`
- `[capabilities.required]` — `fs.read` on `**/*.rs`, `provider.stream` on Anthropic models, `iac.send` on broadcast.
- `[posture]` — assistive; prompt on writes, silent allow on reads.
- `[output_shape]` — JSON schema for code-review findings (severity, file, line, suggestion).
- `[epistemic_policy]` — halts on `claim.security_vulnerability` with confidence < 0.85; verbalize-only on style suggestions.

Diego runs `spirit-test` on a corpus of 50 known-buggy code examples. His Spirit catches 47 of 50 (recall 0.94). Halts on 3 of those (precision uncertain, but he ships it). Runs `maos-spirit publish --tier=public-untrusted`. His Spirit appears in the public registry signed with his Ed25519 key.

**Resolution.** Within a week, MAOS users install Diego's Spirit. Three file issues; one files a PR. Operators promote individual installations to `org-internal` based on local trust evaluation. Diego's GitHub stars double in 4 months. Diego writes a blog post: *"Why I deleted 4,000 lines of HTTP/SDK glue code by becoming a MAOS Spirit."*

**Architectural primitives exercised.** `cargo generate maos-spirit` template scaffolding. `spirit-test` mocking harness for unit-testing Spirit ABI without a kernel. Manifest validation (output_shape, epistemic_policy, capability scopes) at publish time. Spirit registry with Ed25519 signing, three trust tiers (`local`, `org-internal`, `public-untrusted`), strictest-of-(manifest, tier) sandbox/posture floor. Spirit-portable architecture — Diego's behavior code does not import HTTP libraries or LLM SDKs. *Spirit is behavior, not infrastructure.* Halt-recall and halt-precision benchmarks publishable per Spirit. ComplianceClaim envelope verified at admission.

**Acceptance — Diego Onboarding Gate (Staged).**

The full N=12 stratified gate is enforced at v1.0. Earlier versions run smaller, lower-signal gates whose primary purpose is methodology validation, not pass/fail certification.

| Version | N | First-useful threshold | Multi-Spirit threshold | SUS floor | Stratification | Gate effect |
|---|---|---|---|---|---|---|
| **v0.5 (dry-run)** | 4 | ≥3/4 to first-useful in **60 min** | not measured | not measured | convenience sample | Advisory only. Mandatory structured exit interview (recorded, transcribed, coded). Output: methodology refinements, not pass/fail. |
| **v0.7 (expansion)** | 8 | ≥6/8 to first-useful in **45 min** | ≥5/8 to multi-Spirit task in **90 min** | SUS ≥ **68** (industry-average baseline) | partial — at least 2 distinct experience levels, 2 distinct Spirit roles attempted | Soft gate. Failure triggers diagnosis cycle but not release block. Required: published gap analysis if any threshold missed. |
| **v1.0 (full)** | 12 | ≥10/12 to first-useful in **30 min** | ≥8/12 to multi-Spirit task in **60 min** | SUS ≥ **72** (above-average baseline) | full Opus stratification (3 experience tiers × Spirit-role coverage matrix; ≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only) | Hard gate. Failure blocks v1.0 release. 14-day no-DM-support window. Wilson CI [0.552, 0.962] meaningful at N=12. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot. |

The 60/45/30 min progression on first-useful is intentional — we expect the *product* to get better, not just the cohort.

**Why staged.** The N=4 dry-run is too small for statistical inference and is not intended for it. Its purpose is to surface task-instruction defects, recording-protocol defects, and rubric-ambiguity defects *before* burning the N=8 cohort on noise. The N=8 expansion validates the SUS instrument and timing thresholds against a realistic distribution. Only the N=12 v1.0 cohort is statistically meaningful as a release gate.

**Phrasing lock.** The v0.5 dry-run is a calibration instrument for the v1.0 gate, not a substitute. The v1.0 N=12 stratified study is the GA-blocker.

Run the v1.0 gate at major releases (v1.0, v2.0); minor releases use the v0.7 expansion as proxy.

### 10.7 Deferred journeys (future milestones)

Two journeys from the PRD are **explicitly deferred** from the v1.5 architecture scope. They are not cancelled — they are committed as future work, with their full canonical scene descriptions remaining in the PRD as the source of truth until the architecture work to support them is undertaken. The substrate's primitives are designed so neither journey requires architectural rewrite to add later; both ship as configuration + new Spirit classes layered onto the existing kernel.

#### 10.7.1 J3 — Marcus Team Nexus (deferred to v2.0)

**Persona.** Marcus, tech lead / architect on an 8-person agile team at a fintech, day 30 of MAOS adoption. Peer A2A mesh; every team member has their own Host with three Spirits (Architect Atlas, Story-Decomposer, Coder Spirits, Test-Designer, Wireframe Spirit). Cross-Spirit conversations across the team are continuous, audit-trailed, and visible in narrative digest at standup.

**Why deferred.** J3 requires a **full peer A2A mesh** (8 Hosts, role queries, multi-party typed-intent consent across the cohort), where the v1.5 substrate ships only the **bilateral 2-Host A2A** topology (Mira ↔ Nash diagnostic-architect pair). The mesh extension is additive — same wire format, same per-frame consent envelope, same logical-clock discipline restricted from N=2 to N=8 — but requires (a) operator UX for cohort topology declaration, (b) role-query semantics across the consent envelope (intent class allowlists become per-(peer, role) tuples), and (c) cohort-level hot-swap and migration choreography. None of those break the kernel; all of them require substantive design work the v1.5 scope does not absorb.

**Substrate readiness for J3 at v1.5.** Bilateral A2A (§7.2) with mTLS+TOFU + ADR-012 typed-intent consent generalizes to N-host mesh additively. Hot-swap (ADR-017) and halt-continuity (I14) remain correct under cohort topology. Distillation pattern (§9.5) and decision-context recording (I12) operate per-Spirit regardless of cohort size. The §12.0 ADR set's binding-v1.5 commitments do not preclude J3.

**Future milestone target.** v2.0 (or v2.x). PRD §[J3 Marcus] holds the canonical scene; the v2.0 architecture revision will extend §7.2 to cover N-Host mesh, add operator-UX for cohort declaration, and document the role-query consent semantics. **No architectural decision in v1.5 forecloses J3.**

**Reference.** PRD line ~431 ("Journey 3 — Tier 2 normalcy: Marcus's day-30 Tuesday morning standup") is the canonical persona, scene, and capability list. The PRD's §[Capabilities revealed (J3)] enumerates the kernel surfaces J3 exercises — all of them are commitments this architecture already makes (peer mesh A2A, ADR-012 consent, narrative digest per §9.5, audit chain per I11/I12/I13, halt continuity per I14).

#### 10.7.2 Reza — Single-org cross-team Cortex (deferred to v2.0+)

**Persona.** Reza, head of platform engineering at a 400-person fintech. Three teams (security, support, data) run their own Spirits independently across a single-org Cortex. Cross-team Spirits coordinate through cross-host A2A with ADR-012 typed-intent consent; data-residency patterns load from Loom; the Orchestrator unifies recommendations across team-owned Spirits without violating per-team write boundaries.

**Why deferred.** Reza Cortex requires (a) a **multi-tenant Loom** with per-team data-residency enforcement (the v1.5 Loom-lite is single-instance, single-tenant), (b) **WASM Spirit form** as the third deployment form for sandboxed third-party-team Spirits (the v1.5 substrate ships subprocess-only at v0.1, with rust-inproc gated on §13.1 measurement; WASM is out of scope per ADR-002 / ADR-007), (c) **PDP integration** for cross-team policy decisions, (d) **Spirit registry vetting attestations** beyond the three trust tiers v1.0 ships, and (e) cross-team A2A topology beyond the bilateral pair. Every one of these is additive to the kernel; none retract a v1.5 commitment.

**Substrate readiness for Reza at v1.5.** Loom-lite (§9.3) is designed for extraction to multi-tenant Loom without API churn (ADR-006 keeps the kernel learning nothing; multi-tenant boundaries land in user-space). The Spirit registry (ADR-008) at MCP-Streamable-HTTP is registry-protocol-ready for vetting attestation extensions. The trust-tier model (ADR-009, three tiers: local / org-internal / public-untrusted) admits a fourth tier (e.g., `org-vetted-public`) without altering the strictest-of-floor enforcement logic. Distillation multi-hop with `source_log_ref` flattening (I11, ADR-014) is the substrate Reza's "14 prior schema decisions cited in one consolidated proposal" scene depends on — already binding-v0.5.

**Future milestone target.** v2.0 (single-org Cortex pilot, 3-region, ≥10 agents) or v2.x as the architecture revision adds multi-tenant Loom, WASM Spirit form, PDP integration, and vetting attestations. PRD §[Reza] line ~453 holds the canonical scene; the v2.0 architecture revision will extend §9 (Memory & Knowledge), introduce the WASM Spirit form (deferring the long-standing ADR-007 v2.0 commitment from "out of scope" to "in scope"), and document cross-team A2A topology.

**Reference.** PRD line ~453 ("Reza — Tier 3 candidate: Single-org cross-team Cortex") is the canonical persona, scene, and capability list. PRD §[Capabilities revealed (Reza)] enumerates the kernel surfaces Reza exercises — all of them are either (a) commitments this architecture already makes for v1.5 (distillation multi-hop, ADR-012 typed-intent consent, peer A2A) or (b) explicitly-named v2.0 substrate extensions (multi-tenant Loom, WASM Spirit form, PDP integration, vetting attestations).

#### 10.7.3 Substrate-stability commitment under deferral

**The v1.5 architecture does not foreclose either deferred journey.** Specifically:

- The Spirit ABI (§5) is wire-stable and the v0.1 commitment to subprocess form (ADR-002) does not preclude WASM Spirit form being added at v2.0 — the wire protocol (ADR-032) is form-agnostic by design.
- Bilateral A2A (§7.2) is documented as "exactly two pre-paired Hosts" because that is the v1.5 deployment shape; the per-frame typed-intent consent envelope (ADR-012) generalizes to N-Host mesh without protocol change.
- Loom-lite (§9.3) is single-instance because v1.5 ships single-instance; the MCP-Streamable-HTTP transport admits multi-tenant deployment without kernel modification.
- The §0.6 Foundational Commitments hold across deferred journeys — kernel/Spirit separation, kernel learns nothing, human transparency, capability mediation — apply identically at any cohort size.

When a deferred journey is undertaken, the architecture revision adds new sections (J3 mesh topology, Reza multi-tenant Loom, WASM Spirit form per ADR-002/007 successor) and may revise specific ADRs, but the §0.6 commitments and the §3.2 invariants do not change.

The PRD remains the canonical home for J3 Marcus and Reza Cortex personas, scenes, and capability lists until those architecture revisions ship.

## 11. Deployment Topologies

Two deployment topologies. Configuration alone composes the substrate into either.

### 11.1 Single-user (single Host)

The J0 / J-Butler / J-Researcher / J1 baseline. One MAOS Host on a laptop or workstation. Up to 5–10 Spirits scheduled cooperatively, with subprocess-form Spirits each in their own cgroups v2 process. Loom-lite optional (typically used by founder-loop distillation-shipping Spirits, not by Butler / Researcher).

**Configuration shape:**
- Single Host runs the kernel with all 5 reference Spirit classes available.
- Default Spirit set: Butler + Researcher + Architect + Observer (loaded on demand).
- Founder loop deployment: add Orchestrator + Developer-Worker + Reviewer-Worker skill packs loaded into agent CLI processes via CliWrapperSpirit.
- Persistence: SQLite for Transparency Log, Approval Decision Log, Journal. Optional Postgres for Loom-lite if the user has founder-loop Spirits opting in.
- Provider: any subset of (Anthropic / OpenAI / local-LLM via Ollama / etc.) per `maos-providers` config.
- Networking: localhost-only by default; A2A loopback profile for the founder-loop multi-CLI pattern.

### 11.2 Diagnostic-architect pair (bilateral 2-Host)

The J4 Mira-Nash deployment. Exactly two MAOS Hosts: Mira on a prod-edge node (read-only on production runtime; RW only on runtime knobs with approval; bash-exec whitelist for containment), Nash in a dev-environment (RW source repo, prompts on every deploy). Hosts are pre-paired at deployment time with each other's mTLS certificate fingerprint.

**Configuration shape:**
- Host A (prod-edge): kernel runs Mira + Observer (sentinel-style narrow capability set focused on production telemetry).
- Host B (dev-environment): kernel runs Nash + Observer + optionally Orchestrator/Workers for the team's coding workflow.
- Loom-lite: a single Postgres+pgvector instance accessible from both Hosts as MCP-Streamable-HTTP. Houses the ADR-pattern library, fix templates, regression-test references. Curation is Spirit-side; Nash decides what is worth persisting.
- Bilateral A2A: configured at deployment with the peer's mTLS cert fingerprint. Per-frame ADR-012 typed-intent consent: Mira's send-allowlist includes `diagnosis-handoff:read-only-evidence`, `cross-environment-telemetry-query`; Nash's accept-allowlist mirrors. Code-mutation-directive frames are blocked at the kernel boundary.
- Mobile push: HTTP push to Elena's phone for high-confidence diagnoses; the operator can approve from the mobile surface.

### 11.3 What's the same across both topologies

The substrate's invariants. The Spirit ABI. The manifest schema. The Capability Registry's mediation policy. The Transparency Log shape. The distillation pattern. The Approval Manager's UX. The hot-swap mechanism. The 14 invariants. The 39 ADRs (with their phased Status tags per §12.0). **Topology is configuration; architecture is invariant.** This is the substrate-positioning claim cashed: same primitives compose into either deployment.

## 12. Architecture Decision Records

Thirty-nine contested decisions, each with rationale, alternatives considered, and what would force a revisit. ADRs are the substrate's evolution mechanism; amendments touching invariants I1–I14 require the `invariant-lock` CI gate to pass.

Every ADR carries a `Status:` and `Gate:` frontmatter line. **Status** is one of:
- `binding-v0.1` — the kernel must implement this from v0.1 onwards. Most-load-bearing tier; changes require a major-version bump and the `invariant-lock` CI gate.
- `binding-v0.3` / `binding-v0.5` / `binding-v0.9` / `binding-v1.0` / `binding-v1.5` — the kernel must implement this by the named phase. Changes before that phase are free; after, they require the `invariant-lock` gate.
- `speculative-vNext` — the decision is on the table for a future phase but the substrate has not yet committed. May be cut, may be promoted to `binding-*`. Speculative ADRs additionally carry a `Resolves-by:` field naming the gate that promotes them.

**Gate** names the falsifiable acceptance test that proves the ADR is implemented at its `Status` phase — a corpus, an invariant CI check, a measurement threshold, or a milestone artifact. `design-only` means no runtime gate yet; promotion to a measurable gate is required before the next phase boundary.

### 12.0 ADR Index Table

Sorted by Status (binding-v0.1 first), then by ADR number. Reviewers triaging "what must I read?" can stop after the binding-v0.1 cluster.

| ADR | Title | Status | Gate |
|---|---|---|---|
| 001 | Kernel language is Rust + Tokio | `binding-v0.1` | v0.1 ships in Rust + Tokio; alternative-language proposals require ADR + benchmark |
| 002 | Spirit form at v0.1 — subprocess only, inproc gated on measurement | `binding-v0.1` | §13 measurement gate (`benches/iac_roundtrip.rs`); promotion to inproc requires three-condition check + superseding ADR |
| 003 | IAC topology — mailbox-on-Host + bilateral A2A cross-Host | `binding-v0.1` | mailbox at v0.1; A2A loopback at v0.9; cross-Host A2A at v1.0 |
| 004 | Sandbox tiers T0/T1/T2/T3 with strictest-of-floor | `binding-v0.1` | T0/T1 at v0.1; T2 at v0.3; T3 at v0.5; trust-tier floor enforced by Capability Registry |
| 005 | Pluggable provider drivers | `binding-v0.1` | Anthropic driver at v0.1; ≥3 providers in CI by v0.5 |
| 006 | The kernel learns no patterns (I9) | `binding-v0.1` | structural-state lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}` |
| 007 | Spirit-form phasing | `retired` | Subsumed by ADR-002 (see §12 ADR-007 stub) |
| 010 | Hexagonal architecture for static structure | `binding-v0.1` | crate boundary lint enforces port/adapter ring |
| 011 | Actor model on the runtime hot path | `binding-v0.1` | per-Spirit Tokio task supervision + bounded mailbox |
| 030 | Capability Registry decomposition (cap-tokens / cap-policy / cap-audit / cap-quota) | `binding-v0.1` | hot-path token verify <5µs P99 benchmark |
| 032 | Spirit Wire Protocol bytes-on-wire (LSP framing + CBOR) | `binding-v0.1` | byte-equal golden corpus per frame variant per SDK; 168h cumulative pre-GA fuzz floor |
| 037 | Constitutional amendment process | `binding-v0.1` | `invariant-lock` CI gate runs on every PR touching I1–I14 |
| 038 | Per-service KLOC ceiling | `binding-v0.1` | `xtask/kloc.toml` enforced by `tokei` in CI; aggregate ≤20 KLOC, alarm at 16 |
| 017 | Hot-swap state-transfer wire format (CBOR + per-class schema + saga rollback) | `binding-v0.3` | swap conformance corpus passes; auto-revert ≤30s on post-swap invariant violation |
| 022 | Tagged-scalar working-memory slot with epistemic-policy binding | `binding-v0.3` | `[epistemic_policy]` rules trigger halts via four universal-arithmetic predicates |
| 025 | Proactive scheduling (`on_schedule` lifecycle hook) | `binding-v0.3` | Butler `on_idle` Sandra-scene replay |
| 033 | Subprocess supervision and halt-crash intersection (crash matrix) | `binding-v0.3` | crash-recovery corpus: torn-frame-at-tail truncate, mid-log fatal |
| 008 | Spirit registry as MCP-Streamable-HTTP server | `binding-v0.5` | `registry.search`/`manifest`/`artifact` operational; MCP-Streamable-HTTP transport |
| 009 | Three trust tiers with strictest-of-floor enforcement | `binding-v0.5` | strictest-of-(manifest, trust-tier, operator-policy) floor in registry admission tests |
| 014 | Distillation audit-chain (I11 — `source_log_ref`, `distillation_depth`) | `binding-v0.5` | Capability Registry rejects digest writes with `EDigestAuditChainMissing`; segment-level by default |
| 015 | Decision-context recording (I12 — `working_memory_digest_refs`) | `binding-v0.5` | shadow-recall record on `event/inbound`; refs attached on `decision.*` emit |
| 018 | Intent provenance preservation across distillation (I13 — `intent_lineage`) | `binding-v0.5` | kernel-computed `intent_lineage`; consumer admission rejects with `EIntentPromotionDenied` |
| 020 | Hot-swap migration policy (`migrate(predecessor_state)`) | `binding-v0.5` | kernel refuses load with `EMigratorMissing` if predecessor archive exists and migrator absent |
| 026 | Principal Memory Namespace with redaction-aware operations | `binding-v0.5` | subject-access query / right-to-be-forgotten / redaction-on-export operate on `principal:*` namespace |
| 027 | Skill-package external-standard interop (`maos.skill.v1`) | `binding-v0.5` | Spirit-form adapter loads ≥1 third-party skill format |
| 035 | Observer scalar trajectory channel (`scalar.tap`) | `binding-v0.5` | Observer subscribers see pre-halt scalar drift in real time |
| 040 | Threat-model split — Sec-14a (same-Host) + Sec-14b (cross-Host) | `binding-v0.5` | 200-scenario isolation corpus passes (Sec-14a at v0.9, Sec-14b at v1.0) |
| 012 | Typed-intent A2A consent (closes confused-deputy gap) | `binding-v0.9` | A2A Gateway rejects frames with intent not in send-allowlist or accept-allowlist |
| 013 | Two-level `task.assign` typed-intent IAC primitive | `binding-v0.9` | founder-loop epic-7 reproducible end-to-end |
| 016 | Token-budget accounting (`ContextPressure`/`ContextLimit`/`EContextExhausted`) | `binding-v0.9` | typed frames emit at 80%/95%; new tool calls fail above 100% |
| 019 | Halt continuity across hot-swap (I14) | `binding-v0.9` | kernel refuses swap with `EHaltContinuityViolation` if drain or schema-compatible migration absent |
| 021 | CliWrapperSpirit output-shape adapter contract (fail-loud) | `binding-v0.9` | startup `EOutputShapeAdapterMismatch` if observed shape ≠ declared version |
| 024 | Spirit-authored skills (admission queue + audit) | `binding-v0.9` | `skill.author.self` capability + operator-admission queue operational |
| 034 | Partial-consent failure semantics (`ConsentRupture`) | `binding-v0.9` | sender receives `ConsentRupture` IAC frame on receiver-side rejection |
| 036 | Hot-swap × halt continuity precondition check | `binding-v0.9` | `maosctl swap` surfaces precondition status before initiating |
| 028 | Replay determinism primitive (trace-shape, not payload) | `binding-v1.0` | trace-shape contract validated in CI per `schemas/trace-shape.schema.json`; v1.0 best-effort, v1.5 hard target |
| 029 | Provider/CLI Gateway sub-module contract (`GatewaySubmodule` trait) | `binding-v1.0` | gateway sub-modules registered via `gateway.toml`; per-FR54 conformance |
| 031 | Cross-Form Spirit Equivalence (`spirit-conformance`) | `speculative-vNext` | Resolves-by: ADR-002 measurement gate triggering inproc unlock; conformance suite ≥90% on 200-scenario class corpus |
| 023 | Capability-token TTL + bind-to-PID | `binding-v0.1` | TTL ≤60s for high-privilege; tokens bound to (Spirit-PID + boot-nonce + expiry); TOCTOU re-validation at use |

**Reserved:** ADR-039 — number reserved; not in scope for the substrate. The four universal-arithmetic predicates from ADR-022 cover the journeys this architecture ships. Future predicate-vocabulary extensions, if justified by a Spirit class, would land in a user-space stdlib without altering the kernel surface (see App-D.3).

### ADR-001 — Kernel language is Rust + Tokio

`Status: binding-v0.1` · `Gate: v0.1 ships in Rust + Tokio; alternative-language proposals require ADR + benchmark` · `Decided: 2026-04-15` · `Revisits: §13 v0.1 row`

**Decision.** The kernel is implemented in Rust on the Tokio async runtime. Spirit-side runtimes inherit this for the in-process form; subprocess and cross-form Spirits use language-neutral wire protocols.

**Rationale.** Type-safe invariants (the 14 invariants are easier to enforce structurally in Rust than in Go or TypeScript). Mature async runtime with work-stealing scheduler. Zero-cost abstractions for the hot path (token verify under 5µs P99). No GC pauses. The cohort survey confirmed the choice: codex, ironclaw, rustain are all Rust+Tokio.

**Alternatives considered.** Go (rejected: lack of trait-based zero-cost abstractions; GC pauses unacceptable on capability-token verify). TypeScript with Deno (rejected: no path to FIPS-validated crypto provider; runtime overhead). C++ (rejected: memory safety burden too high for a substrate kernel).

**What would force a revisit.** Rust's async story regresses materially relative to alternatives (unlikely). Tokio bifurcates and a fork becomes the standard (low probability).

### ADR-002 — Spirit form at v0.1 — subprocess only, inproc gated on measurement

`Status: binding-v0.1` · `Gate: §13 measurement gate (benches/iac_roundtrip.rs); promotion to inproc requires three-condition check + superseding ADR` · `Decided: 2026-04-15` · `Revisits: §13 measurement gate; ADR-031` · `Subsumes: ADR-007`

**Decision.** v0.1 ships **subprocess form only**. Spirits run as subprocess binaries speaking the Spirit Wire Protocol (LSP-style `Content-Length` framing + CBOR payloads, ADR-032) over stdio. In-process Rust Spirits (`rust-inproc`) are **not** an alternative on the table at v0.1; they are a future option gated by §13's measurement harness.

**Rationale.** Subprocess gives polyglot reach and process isolation; it is the form Diego's `code-reviewer-pro` ships in, the form the Orchestrator/Worker/Reviewer skill-package overlays use, and the form that makes third-party Spirit publication safe. Adding a second form at v0.1 would double the invariant-enforcement surface (two crash recovery semantics, two memory models, two hot-paths) for a latency win no in-scope journey has been measured to require.

**Alternatives considered.** Two forms at v0.1 (`rust-inproc` + `subprocess`) — rejected: doubles ABI surface during the foundational phase; the operational complexity is not journey-justified. rust-inproc only — rejected: forces every Spirit author into Rust; kills polyglot ambition. Three forms (+ WASM-component) — rejected: third tier adds substantial toolchain complexity without journey-driving demand at this scope.

**Status reconciliation with §13 (Measurement Gate).** This ADR commits to subprocess-only IAC at v0.1. In-process transport is **not** an alternative on the table at v0.1; it is a future option gated by §13's harness (`benches/iac_roundtrip.rs`, journeys J1/J-Butler/J-Researcher). Promotion to inproc requires (a) sustained 24h breach of one threshold in §13's table, (b) confirmation that J-Butler p95 is not >4× J1 p95 (rules out fixable code overhead), and (c) a follow-up ADR superseding this one. Until those three conditions land in writing, "subprocess-only" is the architecture, not a default.

**What would force a revisit.** §13's measurement gate trips for a journey-required Spirit class, with the three-condition check satisfied. A capability-isolation requirement emerges that subprocess's process boundary cannot meet (in which case WASM-component, not rust-inproc, is the candidate). ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext` and resolves only when this revisit fires.

### ADR-003 — IAC topology is mailbox-on-Host + bilateral A2A cross-Host

`Status: binding-v0.1` · `Gate: mailbox at v0.1; A2A loopback at v0.9; cross-Host A2A at v1.0` · `Decided: 2026-04-15` · `Revisits: §7.2`

**Decision.** Same-Host IAC uses the kernel-internal mailbox (mpsc + broadcast). Cross-Host IAC uses bilateral A2A — exactly two pre-paired Hosts, mTLS+TOFU, per-frame typed-intent consent.

**Rationale.** Same-Host mailbox is the codex-precedent pattern: low latency, kernel-internal, easy to log-before-deliver. Bilateral A2A is the topology the Diagnostic Engineer + Senior Architect pair operates on; the operator names the two Hosts in deployment configuration, and there is no discovery to hide. The bilateral case is a strict subset of the general A2A protocol — same wire format, same consent envelope, same logical clock — restricted to two endpoints.

**Alternatives considered.** Single-Host only (rejected: J4 Mira-Nash requires production-edge / dev-environment separation). Gateway-based cross-Host (rejected: introduces a single point of failure and makes the gateway a privileged kernel-external component).

**What would force a revisit.** A use case emerges that requires three or more Hosts coordinating in real-time. (At that point this is a different architecture, not an extension.)

### ADR-004 — Hexagonal sandboxing with OS-native primitives

`Status: binding-v0.1` · `Gate: T0/T1 at v0.1; T2 at v0.3; T3 at v0.5; trust-tier floor enforced by Capability Registry` · `Decided: 2026-04-15` · `Revisits: §4.3.1, §8.2`

**Decision.** Sandbox tiers T0 (trusted), T1 (UID separation), T2 (Landlock+seccomp narrow / Seatbelt / Windows restricted-token), T3 (T2 + container) form the security boundary. The strictest-of-(manifest, trust-tier, operator-policy) floor applies.

**Rationale.** OS-native primitives are production-grade (Landlock+seccomp on Linux 5.13+; Seatbelt's `.sbpl` profiles on macOS; restricted-token + Job Object on Windows). Codex has shipped all three in production. Adding a process-level container at T3 layers defense-in-depth without inventing new sandbox primitives.

**Alternatives considered.** WASM-component sandbox for Spirits (considered: capability-isolation by construction; rejected for this scope because subprocess + Ed25519 signing + T2 is sufficient for Diego's third-party publishing). Pure container-based isolation (rejected: containers do not give per-syscall granularity).

**What would force a revisit.** The OS sandbox primitives diverge sufficiently that maintaining all three becomes impractical.

### ADR-005 — Pluggable provider drivers

`Status: binding-v0.1` · `Gate: Anthropic driver at v0.1; ≥3 providers in CI by v0.5` · `Decided: 2026-04-15` · `Revisits: §4.4`

**Decision.** LLM provider access is mediated by `maos-providers`, a feature-gated crate exposing a uniform `provider/complete`, `provider/stream`, `provider/embed` capability surface. Spirit manifests declare which providers they can use; the kernel materializes provider credentials at the capability boundary.

**Rationale.** Provider lock-in is a substrate-level risk. Having pluggable drivers means a Spirit author writes against the kernel's provider API once and runs against any driver. Providers are independent crates; new drivers ship without kernel changes.

**Alternatives considered.** Bundle one provider (Anthropic) and require Spirit authors to call HTTP directly for others (rejected: violates I1 capability mediation). Use a single SDK like LiteLLM (rejected: introduces a third-party dependency on the substrate's hot path).

**What would force a revisit.** A provider semantic emerges that the uniform API cannot represent.

### ADR-006 — The kernel learns no patterns

`Status: binding-v0.1` · `Gate: structural-state lint blocks new persistent fields outside {Journal, TransparencyLog, CapabilityRegistry::tokens}` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I9; §9.3`

**Decision.** Patterns, ADRs, fix templates, regression tests — the curated collective knowledge — live in user-space (Loom-lite), not the kernel. The kernel mediates access and audits the access; the kernel does not store, index, or learn from the contents.

**Rationale.** Auditability. The kernel is replaceable; the user's data is not. If patterns lived in the kernel, every kernel upgrade would risk corrupting accumulated knowledge, every audit would have to inspect kernel internals, and the substrate's "boring substrate" claim would erode.

**Alternatives considered.** Build a kernel-resident pattern store (rejected: violates I9; turns the kernel into a state machine that depends on accumulated history).

**What would force a revisit.** A use case emerges where Loom-lite's MCP-Streamable-HTTP latency is unacceptable for a hot-path operation. (Threshold: p99 > 200ms on diagnostic-architect bilateral pair operations.)

### ADR-007 — Spirit-form phasing (retired — subsumed by ADR-002)

`Status: retired` · `Subsumed-by: ADR-002`

**Decision retired.** The phasing content of this ADR has been merged into ADR-002 ("Spirit form at v0.1 — subprocess only, inproc gated on measurement"), which is the single source of truth for the Spirit-form question (deployment, isolation, phasing, WASM scope). ADR-007's number is preserved to keep the ADR sequence stable; the number is not reused.

**Why retired.** ADR-002 and ADR-007 had drifted into near-duplicate commitments after the editing pass that converged both on "subprocess at v0.1, rust-inproc gated on §13 measurement, no WASM in scope." Two ADRs saying the same thing means future amendments would be made in one place and not the other; the number-preserving merge eliminates the drift surface.

**Where to look instead.** All Spirit-form decisions: ADR-002. Cross-form equivalence (when/if rust-inproc unlocks): ADR-031 (`speculative-vNext`). Wire protocol: ADR-032. Crash matrix: ADR-033.

### ADR-008 — Spirit registry as MCP-Streamable-HTTP server

`Status: binding-v0.5` · `Gate: registry.search/manifest/artifact operational; MCP-Streamable-HTTP transport` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** The Spirit registry is itself an MCP-Streamable-HTTP server. `maosctl install` calls `registry.search` / `registry.manifest` / `registry.artifact`; `maos-spirit publish` calls `registry.publish`; `registry.deprecate` for yanks.

**Rationale.** The kernel already speaks MCP for tools and Loom-lite. Reusing MCP for the Spirit registry means zero new transport code, and operators can self-host a registry on any MCP-compatible server.

**Alternatives considered.** Custom protocol (rejected: invents a fifth wire protocol). OCI registry (considered: well-understood; rejected because Spirit packages are not OCI-shaped — they include a manifest, a binary, and signing metadata in a structure OCI does not natively support).

**What would force a revisit.** MCP's evolution diverges in a way that makes registry-over-MCP brittle.

### ADR-009 — Three trust tiers with strictest-of-floor enforcement

`Status: binding-v0.5` · `Gate: strictest-of-(manifest, trust-tier, operator-policy) floor in registry admission tests` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** Three trust tiers exist: `local` (operator-authored Spirits or Spirits the operator has personally vetted), `org-internal` (Spirits authored within the organization, vouched for by the operator's signing key), `public-untrusted` (Spirits authored by anyone, signed with the author's Ed25519 key, no organizational vouch). Strictest-of-(manifest, trust tier, operator-policy) floor: a Spirit at `public-untrusted` is forced to T2 sandbox + cautious posture regardless of what its manifest claims.

**Rationale.** Trust tiers enable a public Spirit registry to be safe by default. A Spirit at `public-untrusted` runs under T2 sandbox + cautious posture; the operator can promote individual installations to `org-internal` based on local trust evaluation.

**Alternatives considered.** No trust tiers (rejected: a public registry would be a supply-chain-attack surface). Centralized vetting (rejected: gatekeeping fails the substrate-not-product framing; promotion is operator-local).

**What would force a revisit.** A trust tier is needed between `org-internal` and `public-untrusted` for federations of cooperating organizations.

### ADR-010 — Hexagonal architecture for static structure

`Status: binding-v0.1` · `Gate: crate boundary lint enforces port/adapter ring; domain core compiles without async runtime` · `Decided: 2026-04-15` · `Revisits: §4.0.1`

**Decision.** The kernel is structured hexagonally: a domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring (concrete adapters for HTTP, stdio, mTLS, MCP, ACP, providers, persistence, secrets).

**Rationale.** Hexagonal gives multi-adapter-per-port flexibility (swap SQLite for Postgres without touching domain logic), testability (every port has a mock adapter), and keeps the domain core small. Clean Architecture's call-direction discipline does not fit a runtime kernel where the kernel calls into Spirit ABI traits as part of its control flow.

**Alternatives considered.** Clean Architecture (rejected: call-direction discipline contradicts the kernel-calls-into-Spirit-ABI inversion of control). Layered (rejected: less flexible for adapter-per-port).

**What would force a revisit.** A subsystem emerges where hexagonal's port abstraction is more friction than value.

### ADR-011 — Actor model on the runtime hot path

`Status: binding-v0.1` · `Gate: per-Spirit Tokio task supervision + bounded mailbox` · `Decided: 2026-04-15` · `Revisits: §4.0.1`

**Decision.** Each Spirit is a Tokio-supervised actor with a bounded mailbox; no shared mutable state between Spirit actors. The seven kernel services are not actors — they are shared services with their own task pools.

**Rationale.** Four properties for free: backpressure via bounded mailboxes, no locks on the Spirit-to-Spirit hot path, failure isolation via Tokio task supervision, and natural hot-swap (replace `behavior` while preserving `state` and `open_tokens`). Codex's `AgentRegistry` + `Mailbox` is the precedent.

**Alternatives considered.** Shared-memory state (rejected: violates I5 and complicates hot-swap). Channel-only architecture without supervisors (rejected: failure handling becomes ad-hoc).

**What would force a revisit.** Tokio's supervisor primitives change materially.

### ADR-012 — Typed-intent A2A consent

`Status: binding-v0.9` · `Gate: A2A Gateway rejects frames with intent not in send-allowlist or accept-allowlist` · `Decided: 2026-04-15` · `Revisits: §7.2, §13 v0.9 row`

**Decision.** Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`. A read-only Spirit cannot pass a payload to a writeable Spirit that, when interpreted, causes a write the read-only Spirit was forbidden from.

**Rationale.** Channel-consent does not imply transaction-consent. The confused-deputy class of attacks at the inter-Spirit boundary requires intent-class scoping. Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected. Without ADR-012, Mira could trigger a Nash-side action she cannot trigger directly.

**Alternatives considered.** Channel-consent only (rejected: leaves the confused-deputy gap open). Typed-intent at the IAC bus layer for ALL frames (considered: more uniform; rejected because cross-Host frames are where the trust boundary actually is, and same-Host IAC frames already inherit the kernel's process-internal trust).

**What would force a revisit.** A workload pattern emerges where intent-class cardinality grows pathologically.

### ADR-013 — Two-level `task.assign` typed-intent IAC primitive

`Status: binding-v0.9` · `Gate: founder-loop epic-7 reproducible end-to-end` · `Decided: 2026-04-15` · `Revisits: §10.4, §13 v0.9 row`

**Decision.** `task.assign` is a typed IAC frame with two levels of granularity: human → Orchestrator at epic granularity (the founder loop entry point); Orchestrator → Worker at story granularity (the Orchestrator decomposes the epic into stories and dispatches to Workers). Same primitive, different topology.

**Rationale.** The founder loop wedge demo requires both levels — the founder dispatches an epic; the Orchestrator dispatches stories within. A single primitive at both levels means the kernel mediates uniformly and the auditor walks one frame type.

**Alternatives considered.** Separate primitives per level (rejected: kernel surface inflation without gain).

**What would force a revisit.** A use case emerges that needs a third level.

### ADR-014 — Distillation audit-chain (introduces I11)

`Status: binding-v0.5` · `Gate: Capability Registry rejects digest writes with EDigestAuditChainMissing; segment-level granularity by default` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I11; §9.5`

**Decision.** Add invariant I11. Every payload tagged `kind: digest` written to private/shared/collective memory carries non-empty `source_log_ref` (transitively flattened to original raw frames) and `distillation_depth`. Kernel rejects malformed writes with `EDigestAuditChainMissing`. Segment-level granularity is the default contractual unit; write-level audit is opt-in for forensic Spirits via manifest declaration.

**Rationale.** Distillation is a substrate-level pattern. Without an audit chain back to raw, the Transparency Log becomes ceremonial. Segment granularity keeps the audit path through 10K-writes/sec workloads without saturating fsync cadence.

**Alternatives considered.** Per-write audit by default (rejected: 10K writes/sec workloads stall on CAS contention). No audit chain (rejected: defeats the point of the Transparency Log).

**What would force a revisit.** A Spirit class needs forensic granularity by default and the segment-level option becomes too coarse.

### ADR-015 — Decision-context recording (introduces I12)

`Status: binding-v0.5` · `Gate: shadow-recall record on event/inbound; refs attached on decision.* emit` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I12`

**Decision.** Add invariant I12. When a Spirit emits a `decision.*` frame, the kernel attaches `working_memory_digest_refs` populated from the Spirit's declared in-context digests AND from any raw frames delivered via inbound events (kernel writes a shadow-recall record before invoking the Spirit's `event/inbound` handler).

**Rationale.** Closes the gap where the digest hid the critical finding → the agent never recalled raw → audit shows raw existed but the agent never reasoned over it. Without I12, audit can prove what raw + what digest, but not what the agent actually saw at decision time.

**Alternatives considered.** Track in-context digest set without inbound-event shadow-recall (rejected: leaves the audit gap open for raw frames delivered via push, not pull). Track every byte of the LLM context window (rejected: requires kernel introspection of LLM-internal state).

**What would force a revisit.** A Spirit class operates on raw frames in working memory without ever calling `log.recall` and without inbound delivery (e.g., reading from a private memory cache populated outside the audit chain).

### ADR-016 — Token-budget accounting

`Status: binding-v0.9` · `Gate: typed frames emit at 80%/95%; new tool calls fail above 100%` · `Decided: 2026-04-15` · `Revisits: §4.6`

**Decision.** The kernel's Capability Registry tracks per-Spirit `context_window_size`, `context_used`, `context_pressure_threshold`. Soft threshold (default 80%) emits typed `ContextPressure` IAC frame; hard threshold (default 95%) emits `ContextLimit`; above 100% the kernel returns `EContextExhausted` on new tool calls.

**Rationale.** Context tokens are agent-infrastructure's analog of provider-billed resources. Spirits need to know they are approaching limits before they hit them; the kernel surfaces the signal so the Spirit's persona logic decides whether to distill, hand off, or halt.

**Alternatives considered.** Provider-side rate limiting only (rejected: providers do not surface per-Spirit context state). Token counting in the kernel (rejected: model-specific, requires kernel to interpret provider configurations).

**What would force a revisit.** Token-counting becomes provider-uniform and the kernel can take it on without violating I9.

### ADR-017 — Hot-swap state-transfer wire format

`Status: binding-v0.3` · `Gate: swap conformance corpus passes; auto-revert ≤30s on post-swap invariant violation` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I6; §4.1`

**Decision.** Hot-swap state transfer uses CBOR-encoded payloads conforming to a per-Spirit-class schema declared in the manifest (`[hot_swap].state_schema_uri` + `state_schema_version`). The kernel rejects swap operations where the predecessor's schema version is not declared compatible by the successor's manifest. Compatibility rules: same major + additive forward = forward-compat; same major + breaking = forbidden (use major bump); cross-major requires explicit migrator. The Hot-Swap Coordinator implements saga-style compensating transactions: on `on_swap_out` failure, the kernel restores the predecessor; on `on_swap_in` failure, it discards the successor and restores the predecessor with original tokens; on post-swap invariant violation, it auto-reverts within 30s.

**Rationale.** Hot-swap correctness depends on predecessor and successor agreeing on state-blob meaning. CBOR + per-class schema gives typed encoding, compactness, language-neutrality, and a kernel-mediated compatibility check. Saga rollback closes the "what if the swap itself fails" gap.

**Alternatives considered.** Untyped opaque blob (rejected: makes hot-swap a demo trick). serde-json without schema (rejected: textual JSON fails forward-compat silently). Single transaction without rollback (rejected: leaves operators with broken state on swap failure).

**What would force a revisit.** A Spirit-class evolution pattern emerges that CBOR + schema does not cover (e.g., embedded streaming-state with seek points).

### ADR-018 — Intent provenance preservation across distillation (introduces I13)

`Status: binding-v0.5` · `Gate: kernel-computed intent_lineage; consumer admission rejects with EIntentPromotionDenied` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I13; §9.5`

**Decision.** Add invariant I13. The kernel computes `intent_lineage` from input frame_ids on every digest write — the union of intent classes of all input frames the digest was distilled from. A consumer that operates under intent `Y` rejects digests whose `intent_lineage` is not contained in `allowed-promotion-set(Y)` declared in the consuming Spirit's manifest. Producer-side enforcement is kernel-computed (not Spirit-self-reported).

**Rationale.** Closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. Kernel-computed (not Spirit-self-reported) closes the asymmetric-enforcement gap.

**Alternatives considered.** Make intent_lineage advisory (rejected: makes I13 advisory; consent laundering becomes silent the moment one Spirit forgets to propagate). Track intent_lineage at the IAC bus layer for ALL frames, not just digests (considered: more uniform, but explodes header overhead for frames that never cross consent boundaries).

**What would force a revisit.** A workload pattern emerges where intent_lineage cardinality grows pathologically.

### ADR-019 — Halt continuity across hot-swap (introduces I14)

`Status: binding-v0.9` · `Gate: kernel refuses swap with EHaltContinuityViolation if drain or schema-compatible migration absent` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I14`

**Decision.** Add invariant I14. When a Spirit with non-empty `halt_set` is hot-swapped, either every halt is drained (resolved before swap) OR every halt is migrated to the successor with full resolution-path state, AND the successor's manifest declares `halt_protocol_compatibility = N` (matching the predecessor's halt-protocol version registered in `halt-registry/<spirit-class>.toml`). Kernel refuses the swap with `EHaltContinuityViolation` otherwise.

**Rationale.** An in-flight halt represents a user's open question; if hot-swap silently drops it, the substrate's halt-resolution-path-completeness claim collapses.

**Alternatives considered.** Always drain before swap (rejected: forces operators to wait on user resolution before urgent kernel updates). Drop halts on swap (rejected: breaks the user trust contract).

**What would force a revisit.** A Spirit class adopts a halt protocol that does not version cleanly.

### ADR-020 — Hot-swap migration policy

`Status: binding-v0.5` · `Gate: kernel refuses load with EMigratorMissing if predecessor archive exists and migrator absent` · `Decided: 2026-04-15` · `Revisits: §4.1`

**Decision.** Cross-major hot-swap with persistent state requires a `migrate(predecessor_state) -> Result<successor_state, Error>` entry point declared in the successor's manifest's `migrates_from` field. Kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared. Predecessor's historical journal stays in cold storage, addressed by `(class, version, instance_id)`.

**Rationale.** Cross-major migration with persistent state is a known graveyard pattern. Forcing Spirit authors to declare migration intent and provide the entry point makes the kernel's contribution structural (allow-list check, migrator presence check); migration logic itself is Spirit-author concern.

**Alternatives considered.** Implicit migration (rejected: silent state corruption). No cross-major hot-swap (rejected: forces full restarts on every breaking change).

**What would force a revisit.** Migration-authoring proves operationally infeasible for typical Spirit classes.

### ADR-021 — CliWrapperSpirit output-shape adapter contract

`Status: binding-v0.9` · `Gate: startup EOutputShapeAdapterMismatch if observed shape ≠ declared version` · `Decided: 2026-04-15` · `Revisits: §6.6, §13 v0.9 row`

**Decision.** CLI-wrapper Spirits use the kernel-builtin `CliWrapperSpirit` class with declared `output_shape_version`. The kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed shape does not match declared version. Wrappers cannot fall back to "best-effort parsing" on shape mismatch — fail-loud.

**Rationale.** Audit drift is the failure mode the substrate cannot tolerate. The founder loop's CLI-wrapper Spirits (Orchestrator + Workers) speak the wrapped CLI's output format; if the CLI's output format drifts (CLI upgrade), the kernel must catch it at startup, not after a corrupted IAC frame lands in the Transparency Log.

**Alternatives considered.** Best-effort parsing with logged warnings (rejected: corrupts audit trail). Per-CLI native Rust wrapper crate (rejected: forces forking the wrapped CLI's release cycle into the MAOS team).

**What would force a revisit.** A wrapped CLI's output format becomes versionless (i.e., changes within minor releases without an explicit version field).

### ADR-022 — Tagged-scalar working-memory slot with epistemic-policy binding

`Status: binding-v0.3` · `Gate: [epistemic_policy] rules trigger halts via four universal-arithmetic predicates` · `Decided: 2026-04-15` · `Revisits: §4.0.7, §4.6.1, §6.1, §6.2, §6.3`

**Decision.** Spirits write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics. Kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Spirit's `[epistemic_policy]` rules reference tagged scalars via these predicates; kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from).

**Rationale.** The tagged-scalar slot is the smallest theater-side primitive that lets the actor's epistemic state become legible to the kernel's halt mechanism without the kernel knowing what the actor is reasoning about. Theater-side primitive: minimal — one typed slot, two APIs, four universal-arithmetic predicate forms. Actor-side responsibility: total — Spirit decides what to track (variance, entropy, ensemble disagreement, KL, EFE, custom proxy), how to compute it, when to update it.

**Alternatives considered.** Kernel computes Spirit-specific scalars (rejected: violates §4.0.7 — kernel does no Spirit-specific cognitive computation). Spirit-author declares custom predicate functions (rejected: opens the kernel surface to arbitrary code execution).

**What would force a revisit.** A Spirit class needs to compare two scalars to each other rather than a scalar to a constant. (At that point: extend the predicate vocabulary additively, not redesign.)

### ADR-023 — Capability-token TTL + bind-to-PID

`Status: binding-v0.1` · `Gate: TTL ≤60s for high-privilege; tokens bound to (Spirit-PID + boot-nonce + expiry); TOCTOU re-validation at use` · `Decided: 2026-04-15` · `Revisits: §4.3.4, §0.6 commitment 6`

**Status correction.** ADR-023 was previously tagged `binding-v1.5`. The token-binding mechanism (PID + boot-nonce + expiry, ed25519-signed) is required from v0.1 onward — without it, the Capability Token surface (ADR-030) has a replay vulnerability across Spirit restarts, which v0.1's Capability Registry mediation invariant (I1) cannot tolerate. The mechanism is implementation detail of v0.1's foundational commitment 6 (§0.6).

**Decision.** Capability-token TTL ≤60s for high-privilege operations. Tokens bound to (Spirit-PID + boot-nonce + expiry); audit-logged at every use with origin-Spirit-ID. Re-validation at use against current state, not cached state (TOCTOU correctness).

**Rationale.** Long-lived tokens are a replay-attack surface. Short TTL + PID binding makes token theft useless across process boundaries. Re-validation at use ensures posture changes during the token's lifetime are honored.

**Alternatives considered.** Long-lived tokens with revocation lists (rejected: revocation propagation latency too high). No expiry (rejected: replay-attack surface).

**What would force a revisit.** A workload pattern emerges where 60s TTL is too short for the task's natural duration.

### ADR-024 — Spirit-authored skills

`Status: binding-v0.9` · `Gate: skill.author.self capability + operator-admission queue operational` · `Decided: 2026-04-15` · `Revisits: §13 v0.9 row`

**Decision.** Spirits may author skills (markdown with TOML frontmatter conforming to `maos.skill.v1`) and either ship them in the Spirit's package or write them dynamically at runtime via the `skill.author.self` capability scope. New skills land in pending state pending operator admission. Operator-admission queue handles the pending state.

**Rationale.** A Spirit author may want to ship a Spirit-authored-skills-loop (the Spirit improves its own skill library based on user feedback). The kernel's contribution: a registry mechanism for skills + a pending-admission queue + audit on every skill admission.

**Alternatives considered.** No Spirit-authored skills (rejected: forecloses self-improving-loop patterns). Auto-admission of Spirit-authored skills (rejected: cargo-culting risk; LLM-generated skills entering the library without operator review is a known failure mode).

**What would force a revisit.** Spirit-authored skills accumulate at scale and operators want kernel-mediated bulk admission.

### ADR-025 — Proactive scheduling

`Status: binding-v0.3` · `Gate: Butler on_idle Sandra-scene replay; on_schedule fires at declared cadence` · `Decided: 2026-04-15` · `Revisits: §5.3, §6.1`

**Decision.** Spirits may declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist.

**Rationale.** Butler's anticipatory loop, Mira's periodic-health-check pattern, Researcher's daily-arXiv-watch — all need scheduled invocations beyond the user's explicit request. The kernel provides the scheduling primitive; Spirits decide the semantic.

**Alternatives considered.** Spirits self-schedule via internal timers (rejected: kernel-mediated scheduling lets the operator surface scheduled work in audit and lets the kernel rate-limit). Cron-style external scheduling (rejected: violates I1 capability mediation).

**What would force a revisit.** Scheduled invocations require sub-second cadence (the kernel's tick is currently ≥1s).

### ADR-026 — Principal Memory Namespace with redaction-aware operations

`Status: binding-v0.5` · `Gate: subject-access query / right-to-be-forgotten / redaction-on-export operate on principal:* namespace` · `Decided: 2026-04-15` · `Revisits: §4.2`

**Decision.** The kernel adds a typed namespace within the existing private-tier memory: `principal:<principal_id>:<spirit-author-defined-schema>`. Writes to this namespace are tagged as principal-related data and inherit three kernel-mediated operations: subject-access query, right-to-be-forgotten, redaction-on-export. The kernel does NOT interpret principal-namespace content; schema is entirely Spirit-author-declared.

**Rationale.** Privacy-aware Spirits (Butler watching the user's calendar; Researcher accumulating per-author bibliographies) need a namespace where principal data inherits the three operations. Without this primitive, every Spirit author would re-invent principal-aware curation.

**Prior art.** The principal-scoped memory model is informed by hermes-agent's principal-namespaced memory pattern lifted into a kernel-allocated contract. Hermes-as-application demonstrated the operational shape; MAOS lifts it into a kernel primitive so the substrate can offer the contract uniformly to any Spirit-author.

**Alternatives considered.** Spirit-author-handled principal scope (rejected: every Spirit re-invents the wheel). Dedicated principal-store as a new memory tier (rejected: tier inflation; the existing private tier suffices with the namespace tag).

**What would force a revisit.** A workload pattern emerges where the three operations are insufficient and a fourth is needed.

### ADR-027 — Skill-package external-standard interop

`Status: binding-v0.5` · `Gate: Spirit-form adapter loads ≥1 third-party skill format` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** Skills are markdown with TOML frontmatter conforming to `maos.skill.v1`. The format is intentionally close to (but distinct from) the Anthropic Skills format and similar emerging conventions. A Spirit-form adapter can load at least one third-party skill format without kernel modification.

**Rationale.** Skill ecosystems are converging across vendors. The substrate supports the convergence by making `maos.skill.v1` close to the dominant external standards while retaining the kernel-mediated admission flow.

**Alternatives considered.** Adopt a third-party skill format wholesale (rejected: gives up control over admission semantics). Define a wholly novel format (rejected: forces every author to re-learn).

**What would force a revisit.** A dominant skill format emerges that `maos.skill.v1` cannot interop with cleanly.

### ADR-028 — Replay determinism primitive

`Status: binding-v1.0` · `Gate: trace-shape contract validated in CI per schemas/trace-shape.schema.json; v1.0 best-effort, v1.5 hard target` · `Decided: 2026-04-15` · `Revisits: §7.3`

**Decision.** Replay determinism is over the **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders carrying the same structural shape. The trace-shape contract is specified in `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12); the schema is validated in CI on every kernel commit.

**Rationale.** Replay-vs-redaction is an architectural tension: bit-exact replay makes the Transparency Log a forensic record; full redaction makes the log privacy-respecting. The shape-of-trace compromise satisfies both — the shape replays bit-exact (auditor can verify ordering, frame types, halts), the content redacts cleanly. The WAL+MVCC pattern from databases applied to the agent-runtime layer.

**Alternatives considered.** Bit-exact replay including payload (rejected: inconsistent with right-to-be-forgotten). No replay (rejected: defeats the audit story).

**What would force a revisit.** A regulatory regime emerges requiring bit-exact payload replay regardless of redaction.

### ADR-029 — Provider/CLI Gateway sub-module contract

`Status: binding-v1.0` · `Gate: gateway sub-modules registered via gateway.toml; per-FR54 conformance` · `Decided: 2026-04-15` · `Revisits: §13 v1.0 row`

**Decision.** Provider and CLI gateway sub-modules (FR54) are first-class crates implementing the `GatewaySubmodule` trait — `auth`, `model_capabilities`, `stream_translate`, `halt_lift`. No direct kernel coupling; gateway sub-modules registered via `gateway.toml`. Schema specified in `schemas/gateway-submodule.schema.json`.

**Rationale.** A hermes-class tenant needs a gateway abstraction that mediates between Spirits and external-CLI subprocesses with revocation, rate-limit, secret-redaction, and per-subprocess capability scoping. ADR-021 covers fail-loud parsing; ADR-029 covers the gateway-side contract.

**Alternatives considered.** Direct kernel-side gateway (rejected: violates kernel-stays-small). Spirit-side gateway (rejected: leaves the secret-redaction boundary inside Spirit code).

**What would force a revisit.** A gateway-able external surface emerges that the trait's four method shapes cannot represent.

### ADR-030 — Capability Registry decomposition

`Status: binding-v0.1` · `Gate: hot-path token verify <5µs P99 benchmark` · `Decided: 2026-04-15` · `Revisits: §4.6`

**Decision.** The Capability Registry is internally split into four sub-services: `cap-tokens` (hot path, lock-free token issue/verify), `cap-policy` (consent rules + intent allowlist), `cap-audit` (transparency log writer, slow path), `cap-quota` (per-Spirit budget tracking). IAC traverses only `cap-tokens` on the hot path; the audit/lineage path is async via bounded MPSC.

**Rationale.** A monolithic Capability Registry is a god-service. Decomposing it preserves the Capability Registry as a single mediation surface from the Spirit-API perspective while internally separating the hot path from the slow path so audit writes do not block frame delivery.

**Alternatives considered.** Monolithic Capability Registry (rejected: serializes IAC hot path). Per-Spirit Capability Registry instances (rejected: cross-Spirit mediation becomes ad-hoc).

**What would force a revisit.** A new capability surface emerges that does not fit into the four sub-service shapes.

### ADR-031 — Cross-Form Spirit Equivalence

`Status: speculative-vNext` · `Gate: ≥90% on 200-scenario class corpus when both forms exist` · `Resolves-by: ADR-002 measurement gate triggering rust-inproc unlock + superseding ADR` · `Decided: 2026-04-15` · `Revisits: ADR-002, §13 measurement gate`

**Decision.** rust-inproc and subprocess Spirits MUST pass an identical conformance suite (`spirit-conformance` crate). Form is a deployment knob, not a semantic one. Cross-form Semantic equivalence floor: ≥90% on a 200-scenario class corpus. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs per scenario).

**Rationale.** Spirit authors must be able to develop in rust-inproc and ship as subprocess (or vice versa) without behavior drift. The conformance suite enforces this empirically.

**Alternatives considered.** Allow per-form behavior divergence (rejected: defeats the form-portability claim).

**What would force a revisit.** A Spirit class emerges where the two forms cannot match (e.g., rust-inproc-only filesystem semantics).

### ADR-032 — Spirit Wire Protocol bytes-on-wire

`Status: binding-v0.1` · `Gate: byte-equal golden corpus per frame variant per SDK; 168h cumulative pre-GA fuzz floor` · `Decided: 2026-04-15` · `Revisits: §5.2`

**Decision.** LSP-style `Content-Length` framing over stdout: `Content-Length: <decimal>\r\n\r\n` followed by exactly N bytes of CBOR-encoded payload. Header is ASCII, case-insensitive name, max header block 4 KiB. Stderr reserved for diagnostics; never multiplexed onto stdout. EOF after a clean frame = `Halt::Voluntary`; mid-frame EOF = `Halt::Fault(Truncated)`. Backpressure via credit-based windowing on bounded `mpsc<Frame>(64)`.

**Rationale.** LSP framing is well-understood and implementations are abundant. CBOR is compact, language-neutral, schema-evolved. The framing details are spelled out so subprocess implementations across languages produce byte-equal output.

**Alternatives considered.** Newline-delimited JSON (rejected: large payloads break easily on partial newline encoding). Raw JSON-RPC without length prefix (rejected: parser ambiguity on partial frames).

**What would force a revisit.** A use case emerges where Content-Length framing cannot represent the message structure cleanly.

### ADR-033 — Subprocess supervision and halt-crash intersection

`Status: binding-v0.3` · `Gate: crash-recovery corpus: torn-frame-at-tail truncate, mid-log fatal` · `Decided: 2026-04-15` · `Revisits: §4.1`

**Decision.** Defines the (open-halt × in-flight CBOR × SIGKILL) crash matrix. Supervisor reissues halt to successor only if CBOR snapshot is `committed`; otherwise halt is poisoned and surfaced to operator. Per-Spirit `Arc<RwLock<TokenLedger>>` lives kernel-side, not Spirit-side. On crash mid-CBOR-write: supervisor's `JoinSet` returns `Err`, supervisor calls `cap_registry.revoke_all(spirit_id)` synchronously, journal records `HaltRecord{cause: Fault, in_flight_tokens: [...]}`. Replay rule: torn frame at tail = truncate; torn frame mid-log = fatal corruption requiring manual recovery.

**Rationale.** The intersection of three independent mechanisms (subprocess form + hot-swap state-transfer + halt continuity) at the moment a subprocess Spirit dies is the most subtle correctness boundary in the architecture. Specifying it explicitly closes the "what happens when..." gap.

**Alternatives considered.** Treat crash + open halt as fatal (rejected: too restrictive). Allow successor to inherit crashed predecessor's halts unconditionally (rejected: silent successor-confusion events are exactly the failure mode I14 exists to prevent).

**What would force a revisit.** A Spirit class needs different crash semantics than the matrix supports.

### ADR-034 — Partial-consent failure semantics

`Status: binding-v0.9` · `Gate: sender receives ConsentRupture IAC frame on receiver-side rejection` · `Decided: 2026-04-15` · `Revisits: §4.5`

**Decision.** Sender-approved / receiver-rejected mid-frame becomes a `ConsentRupture` event; frame is quarantined, not delivered, not silently dropped. Sender receives `ConsentRupture` IAC frame; operator surface logs the rupture for forensic review.

**Rationale.** ADR-022's failure-semantics floor covers crash detection and `task.orphaned`; partial-consent failure is a third class of failure that needed explicit semantics.

**Alternatives considered.** Best-effort delivery with logged warning (rejected: violates I8). Reject the entire conversation on first rupture (rejected: too restrictive).

**What would force a revisit.** Partial-consent ruptures become common enough that the operator surface needs aggregation.

### ADR-035 — Observer scalar trajectory channel

`Status: binding-v0.5` · `Gate: Observer subscribers see pre-halt scalar drift in real time` · `Decided: 2026-04-15` · `Revisits: §4.7, §6.5`

**Decision.** A read-only `scalar.tap` stream emits every Spirit's `working_memory.set_scalar` write so Observer Spirits see pre-halt scalar trajectory, not just halt events.

**Rationale.** Mira-class diagnostic Spirits use pre-halt scalar drift as a diagnostic signal. Without scalar.tap visibility, Observer can describe halts but not the runup.

**Alternatives considered.** Telemetry stream emits scalar writes (considered: similar; rejected because telemetry stream already carries non-scalar events and conflating the two raises subscriber complexity).

**What would force a revisit.** Scalar writes become high-frequency enough that the dedicated channel saturates.

### ADR-036 — Hot-swap × halt continuity precondition check

`Status: binding-v0.9` · `Gate: maosctl swap surfaces precondition status before initiating` · `Decided: 2026-04-15` · `Revisits: ADR-019, §4.1`

**Decision.** `maosctl swap` precondition check: predecessor open-halts ⊆ successor accepted protocol versions per `halt-registry/<spirit-class>.toml`. Operator UX surfaces "predecessor has 3 open halts at protocol v2; successor accepts v2; safe" before initiating the swap.

**Rationale.** I14 enforcement at the kernel boundary prevents `EHaltContinuityViolation` at swap-time; ADR-036 surfaces the same check at the operator UX so operators see the safety status before triggering the swap.

**Alternatives considered.** Kernel-only enforcement (rejected: leaves operator without pre-flight visibility).

**What would force a revisit.** Halt-protocol versioning becomes finer-grained than registry-table allows.

### ADR-037 — Constitutional amendment process

`Status: binding-v0.1` · `Gate: invariant-lock CI gate runs on every PR touching I1–I14` · `Decided: 2026-04-15` · `Revisits: §3.2, §8.7`

**Decision.** ADRs touching invariants I1–I14 require two-reviewer + invariant-test diff; CI gate `invariant-lock` enforces. ADR amendments require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update.

**Rationale.** The constitutional commitment (Innovation #7 in PRD Step 6) requires architectural enforcement, not founder discipline. Without ADR-037, ADRs are markdown that one human can rewrite.

**Alternatives considered.** Process-only amendment (rejected: relies on founder discipline). External governance board (rejected: scope inflation).

**What would force a revisit.** The reviewer pool becomes too small for the two-reviewer requirement.

### ADR-038 — Per-service KLOC ceiling

`Status: binding-v0.1` · `Gate: xtask/kloc.toml enforced by tokei in CI; aggregate ≤20 KLOC, alarm at 16` · `Decided: 2026-04-15` · `Revisits: §4.0.4`

**Decision.** Kernel ≤20 KLOC trusted core enforced as the sum of per-crate ceilings. Per-crate budgets in `xtask/kloc.toml`: `maos-kernel-core ≤6 KLOC`, `maos-cap-registry ≤3 KLOC`, `maos-wire ≤2 KLOC`, `maos-journal ≤2 KLOC`, etc. Aggregate ≤20 KLOC, alarm at 16. CI gate via `tokei`.

**Rationale.** "Kernel stays small" needs structural enforcement, not memo discipline. Per-crate ceilings make the KLOC budget legible and machine-checked.

**Alternatives considered.** Aggregate-only ceiling (rejected: no early warning when one crate consumes the budget). No ceiling (rejected: erodes silently).

**What would force a revisit.** A new kernel surface justifies a ceiling extension; amendment via ADR-037 process.

### ADR-040 — Threat-model split: same-Host vs A2A

`Status: binding-v0.5` · `Gate: 200-scenario isolation corpus passes (Sec-14a at v0.9, Sec-14b at v1.0)` · `Decided: 2026-04-15` · `Revisits: §8.1`

**Decision.** NFR-Sec-14 (cross-Spirit memory isolation) splits into Sec-14a (same-Host: namespace, seccomp, capability tokens) and Sec-14b (A2A: mTLS, signed frames, replay window). Each has its own 200-scenario adversarial corpus.

**Rationale.** Same-Host attack vectors (one Spirit subvert another via shared filesystem, broadcast topic, or capability-token forgery) and cross-Host attack vectors (peer Host injecting false frames, certificate-pin attack, replay) are sufficiently different that separate corpora are needed.

**Alternatives considered.** Combined corpus (rejected: dilutes coverage of either attack class).

**What would force a revisit.** A third attack class emerges that does not fit either category.

## 13. Phased Roadmap

Seven phases (v0.1, v0.3, v0.5, v0.7, v0.9, v1.0, v1.5), each with one observable validation milestone.

**Phase numbering convention.** Phase numbers are spaced at 0.2 intervals through v0.7 / v0.9 as nominal labels (v0.2, v0.4, v0.6, v0.9 are reserved for unplanned interim releases and not currently scheduled); the v0.1 → v0.3 → v0.5 → v0.7 → v0.9 progression preserves consistent 0.2 spacing through the maturity-gates phase. v1.0 and v1.5 follow the major-release semver convention. Phase numbers are nominal milestone labels, not arithmetic intervals — adoption order is fixed by the dependency arrows in the table below, not by the numeric distance between adjacent labels.

| Phase | Scope | Validation milestone |
|---|---|---|
| **v0.1 — Foundational** | Kernel skeleton (Scheduler + Memory + Capability Registry + IAC mailbox basic — single-Spirit routing, no cross-Spirit fan-out); Spirit ABI v0.1 (machine-checked frozen via CI ABI-diff); single-Spirit subprocess form (Spirit Wire Protocol over JSON-RPC; one Spirit instance per kernel); local SQLite persistence; sandbox tiers T0/T1; Anthropic provider driver; Transparency Log (I2 — log-before-deliver) operational; capability tokens (I1 — every external call mediated); lifecycle journaling (I10) operational; one placeholder Spirit (`hello-spirit`) — minimal reference Spirit demonstrating the ABI; `maosctl` basic (`install`, `uninstall`, `audit query`, `spirit invoke`); accessibility (`NO_COLOR`, `TERM=dumb`, `--plain`); clean uninstall; `SECURITY.md` with disclosure pipeline; doctest CI gate. | J0 evaluator path: install + first useful Spirit response within 5 minutes; clean uninstall removes all kernel state; audit-from-minute-1. |
| **v0.3 — Butler** | Adds: Butler Spirit (first reference cognitive Spirit with full anticipatory reasoning surface); `on_idle` lifecycle hook; Telemetry Stream (I7) basic narrow per-Spirit subscription; `[epistemic_policy]` per-tag rules; output_shape predicate enforcement at Capability Registry; MCP tool integrations (Calendar / Slack / Linear / Figma); posture-shift command at runtime; sandbox tier T2 (Landlock+seccomp narrow); episodic memory (private tier); hot-swap mechanism (ADR-017 wire format) functional; contributor #2 onboarded; advisor council formed; `RFC_TEMPLATE.md` + `GOVERNANCE.md` drafted. | J-Butler journey reproducible — Sandra's 7 PM scene runs end-to-end including the self-tuning halt three weeks later. Notification precision ≥0.85, recall ≥0.7; halt-precision ≥0.85. **30-Min First Spirit Validation Gate (NFR-Onb-1) as v0.3 release criterion** (N=12 stratified, ≥10/12 succeed). |
| **v0.5 — Researcher + Observer + foundational hardening** | Adds: Researcher Spirit (second cognitive Spirit; survey-mode); Observer Spirit (third reference Spirit; passive activity-stream subscriber via Telemetry Stream); broad MCP capabilities (web/arXiv/GitHub/citation-graph); parallelism in tool dispatch (≤8 concurrent); `hypothesize-mode` posture declared (full ILP+LLM hybrid implementation deferred to v1.0); adaptive-chunk-ratio summarization; sandbox tier T3 (containerized); per-Spirit resource isolation via cgroups v2; distillation pattern (single-Spirit opt-in deployment) with five-metric gate; `log.recall` capability; I11 audit-chain enforcement; I12 decision-context recording; I13 intent_lineage propagation; pre-write secret-redaction filter; multi-provider LLM drivers (≥3 providers tested in CI); Spirit registry basic (MCP-Streamable-HTTP server, Ed25519 signing, install-time signature verification); `maosctl audit query` family; Approval Manager prompt UX; Transparency Log persistence with 90-day retention; ACP server (Zed + VSCode tested); `CODE_OF_CONDUCT.md`, `BREAKING.md`, `LOCALES.md`, `TRADEMARK.md`, `PRIVACY.md`, namespace grammar ADR locked. | J-Researcher journey reproducible — Hannah's 2-hour LLM-judge survey delivers structured findings + Open Questions + Confidence Map + Bibliography in ≤90 minutes; Researcher halts on contradictory findings; output passes output_shape predicate. Observer subscribes to multi-Spirit Telemetry Stream showing live activity for the Butler + Researcher pair. |
| **v0.7 — Maturity gates** | Adds: staged Diego N=8 expansion cohort per §10.6 (≥6/8 to first-useful in 45 min, SUS ≥68); §7.2.1 mTLS revocation latency floors enforced (median ≤60s, p99 ≤5min — zero-data-plane-error floor advisory until v1.0); §6.6 halt-precision/recall corpus expanded to N≥100 (precision floor enforced, recall floor advisory); App-E v0.7 row activated — judge in shadow mode (N≥50, ρ ≥0.45, κ ≥0.75, accuracy ≥70% on held-out set, judge scores logged but not gating). v0.7 deliverables depend on telemetry from v0.5 deployments. | Maturity-gate dry-run passes: Diego N=8 cohort completes; mTLS revocation under load measured at scale; halt corpus N=100 produced with κ ≥0.7 inter-rater. **No new journey reproducibility milestone — v0.7 is a measurement phase, not a feature phase.** |
| **v0.9 — Founder Loop wedge demo** | Adds: Multi-Spirit IAC bus (kernel-internal Spirit↔Spirit routing on the same Host); A2A peer at the loopback profile (`127.0.0.1`-bound, mTLS with self-signed certs, TOFU pinning); two-level `task.assign` typed-intent IAC primitive; ADR-012 typed-intent consent enforcement on A2A frames; ADR-016 token-budget accounting; ADR-017 hot-swap state-transfer wire format operational; ADR-020 hot-swap migration policy; ADR-021 CliWrapperSpirit output-shape adapter contract; ADR-022 failure-semantics floor (crash detection ≤2s, `task.orphaned` NACK ≤5s, journaled crash transition with exit-cause); Orchestrator Spirit (skill-package overlay on Claude Code process); Developer-Worker + Reviewer-Worker skill packages; full distillation pattern (§9.5) operational multi-Spirit; multi-CLI Worker parallelism (3+ concurrent); halt-recall and halt-precision benchmarks per Spirit class published; cross-Spirit memory isolation corpus (Sec-14a same-Host, 200 scenarios) passed. | J1 Founder's Loop reproducible — Lunarpulse runs Epic-7 BMAD loop end-to-end with Orchestrator + multi-CLI Worker pattern; halts on AC ambiguity at 6:23 PM; closes laptop at 8:40 PM; wakes to a completed digest at 6 AM. **The substrate's "moment of full ambition observable in one demo."** |
| **v1.0 — Team-pair-ready, third-party Spirits ship** | Adds: 5 reference Spirit classes complete (Butler / Researcher / Architect / Diagnostic Engineer / Observer); cross-host bilateral A2A peer mesh with full mTLS + TOFU + ADR-012 consent; Spirit registry v1.0 (`registry.search` / `manifest` / `artifact` / `publish` / `deprecate`; three trust tiers; strictest-of enforcement); ComplianceClaim envelope + admission-time verification (first-class kernel object); audit sealed-export (Ed25519-signed regulator-ready bundle); HSIS ≥95% pass rate per Spirit class with zero invariant violations; CCAC N=600 with per-class N=30 floor ≥27/30; black-box third-party trial N=12 stratified ≥10/12; manifest fuzz + wire fuzz tiered cadence (§5.2 ladder: T1 10-min per-commit + T2 4h nightly + T3 24h per-RC; ≥168h cumulative pre-GA floor); external pen-test report with zero P0/P1 findings; typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>`; GDPR right-to-be-forgotten; cascade erasure receipt; cost-attribution accuracy ≥98%; crypto-module pluggability; air-gapped deployment validation; 1-year LTS announced; documentation artifacts (API reference, manifest schema reference, typed error catalog, migration runbooks Path A + Path B, troubleshooting guide, deployment topology guide, three-door page, WCAG AA). | **J3 baseline at v1.0 = bilateral 2-Host substrate readiness only** (not the full 8-Host team mesh, which is deferred to v2.0 — see §10.7.1). J6 Diego validated via N=12 stratified black-box external-author trial. **A third party authors and ships a Spirit binary independently of the MAOS source tree.** Hermes-tenant claim cashed at v1.0. |
| **v1.5 — Diagnostic-Architect bilateral pair** | Adds: Diagnostic Engineer Spirit class (Mira-class) with full asymmetric capability gates; per-tag epistemic policy at production fidelity; post-deploy feedback IAC topic (Architect-class subscribes to Diagnostic-class post-deploy validation); Loom-lite (single-instance Postgres-backed pattern library, exposed as MCP-Streamable-HTTP server); `maos-persistence` Postgres support; MAOS-mediated provider proxies (intercept HTTP calls to LLM providers for substrate-layer audit); asymmetric postures (sre-diagnostician vs principal-architect); mobile-friendly approval surface (HTTP push notifications); five-metric distillation gate passes on all distillation-shipping reference Spirits; JetBrains plugin-bridge for ACP integration; bilateral 2-host mTLS rotation chaos test passes per §7.2.1 (revocation latency median ≤60s, p99 ≤5min, zero data-plane errors); halt-protocol-version registry per Spirit class; **J4-extended halt-continuity corpus (≥30 hot-swap-during-incident scenarios + mutation/fuzzing infrastructure, adversarial agent-handoff timing, I14 fuzz-tier substrate per §3.2.1 cadence promotion)**. (Note: ADR-023 capability-token mechanism ships at v0.1 per its corrected Status; v1.5 only validates the mechanism at bilateral 2-host scale.) | J4 Mira-Nash 90-minute diagnostic-architect loop reproducible on 50-scenario synthetic prod-incident corpus; ≥45/50 close in ≤90 min; ≥48/50 uphold typed-intent consent envelope. **Terminal milestone of the substrate.** |

Three principles guide phasing:

- **Each phase has a single observable validation milestone.** "We have v0.5" means the milestone is met, not that the to-do list is empty.
- **No phase ships without ADR review.** If a phase forces a revisit of any architectural ADR, the phase boundary moves.
- **Phasing is invariant-preserving.** Every phase ships a working subset of the fourteen invariants; no phase ships a relaxed version of any invariant. v0.1 ships I1–I10 enforced at the foundational kernel layer; v0.5 adds I11/I12/I13 enforcement; v0.9 adds I14.

**Resource requirements (per phase):**

| Phase | Cumulative effort | Team | Key skills added |
|---|---|---|---|
| v0.1 | ~6–8 weeks | 1 founder | Rust + Tokio, kernel design, capability systems, Spirit ABI specification, single-Spirit subprocess |
| v0.3 | ~12–14 weeks total | 1 founder + advisor council formed | Active Inference / POMDP cognitive modeling, MCP integrations, `on_idle` lifecycle hook, narrow Telemetry Stream subscription |
| v0.5 | ~20–22 weeks total | 2 implementers (founder + contributor #2) + 3-person advisor council | Broad MCP capabilities, parallelism in tool dispatch, output_shape predicate enforcement, T2/T3 sandbox, distillation pattern, v0.5 onboarding artifacts |
| v0.9 | ~26–28 weeks total | 2–3 implementers + community contributors | Multi-Spirit IAC bus, A2A loopback, Orchestrator+Worker coordination, ADR-022 failure-semantics floor, full distillation pattern, multi-CLI Worker parallelism |
| v0.7 | ~30–34 weeks total (v0.5 + 4–6 weeks for maturity gates) | 2–3 implementers + advisor council | Diego N=8 cohort coordination, mTLS revocation latency instrumentation, halt corpus N=100 with κ ≥0.7 IAA labeling, App-E v0.7 judge in shadow mode |
| v1.0 | ~42–46 weeks total† | 3–4 implementers + ≥1 community contributor + DevRel | Bilateral A2A cross-host, Spirit registry, Ed25519 signing, ACP server, ComplianceClaim, HSIS/CCAC/manifest-fuzz/wire-fuzz gates, 30-min validation gate, typed error catalog, 8 doc artifacts, §6.6 halt corpus N=150, §7.2.1 mTLS rotation chaos infrastructure |
| v1.5 | ~50–54 weeks total† | 4–5 implementers + ≥3 community contributors | Postgres + pgvector for Loom-lite, MAOS-mediated provider proxies, mobile push, JetBrains plugin, asymmetric postures, J4-extended halt-continuity corpus + I14 fuzz harness |

† **Remediation-delta footnote.** v1.0's `~42–46 weeks` and v1.5's `~50–54 weeks` are *cumulative* totals (start-of-project to end-of-phase), inclusive of all prior phases. The remediation pass added ~6–10 weeks to the **prior v1.0 estimate** (~36–40 weeks) for §6.6 halt corpus N=150 with κ ≥0.7 IAA labeling, §7.2.1 mTLS rotation chaos infrastructure, and the full Diego N=12 stratified cohort. v1.5 added ~6–8 weeks for the J4-extended halt-continuity corpus + mutation/fuzzing infrastructure. The deltas attach to the **prior estimate of the same phase**, not to the increment from the previous phase row.

### 13.1 Spirit-form Measurement Gate (subprocess → inproc)

ADR-002 commits to subprocess-only at v0.1, with rust-inproc gated (not deferred). This subsection defines the gate so future "should we unlock inproc?" conversations have a falsifiable answer instead of a vibe.

**Harness.** `benches/iac_roundtrip.rs` using `criterion`. Three workloads:

| Journey | Description | What it measures |
|---|---|---|
| **J1** (synthetic floor) | Echo Spirit; 256-byte CBOR payload; single producer, single consumer; pinned cores | Bus floor — serialize + framing + pipe + deserialize |
| **J-Butler** | Realistic butler flow: 1 inbound → 2 outbound `mcp.call` fan-out → 1 aggregated response; payloads 1–4 KB | p50/p95/p99 round-trip under typical load |
| **J-Researcher** | Long-running researcher: 50-frame burst, 16–64 KB payloads; includes one EpistemicHalt + Resume cycle | Tail latency + halt/resume cost |

**Metrics emitted per journey:** `iac_rt_p50_us`, `iac_rt_p95_us`, `iac_rt_p99_us`, `iac_rt_max_us`, `cpu_user_pct`, `cpu_sys_pct`, `rss_max_mb`.

**Per-journey latency budgets (subprocess, v0.1):**

| Journey scope | Budget | Notes |
|---|---|---|
| J0 Butler conversational turn | end-to-end < 400ms P95; Spirit-IPC budget < 60ms | The user-perceived budget; IPC is one slice |
| J1 Founder loop CliWrapperSpirit per-tool-call | IPC overhead < 25ms P95 | Tool calls compose; tight budget needed |
| J4 Mira-Nash Observer colocation | < 10ms P95 | Colocation is the whole point — Observer subscribing to `scalar.tap` cannot lag the producer |
| J6 Diego onboarding cold-start | < 500ms acceptable | Not latency-sensitive; correctness gate dominates |

**Unlock thresholds (any one trips → ADR-required move to inproc for that path).** Each threshold is operationalized as a Prometheus alert rule with a 24-hour `for:` clause. Prometheus evaluates the expression at every `evaluation_interval` tick (project default: 1 minute, set in `prometheus.yml`); the `for: 24h` clause requires the boolean result to remain `true` at **every** tick across 24 hours — i.e., 1,440 consecutive 1-minute evaluations with no false reading. The `rate(...[5m])` window inside the expression is independent of evaluation cadence: each evaluation computes a per-second rate over the trailing 5 minutes. A single 1-minute evaluation where p95 ≤ 1500 µs **or** sustained rate ≤ 100 req/s (averaged over the trailing 5-minute `rate()` window; the `[5m]` is the averaging window, not the unit denominator) resets the `for` timer to zero and the alert re-arms.

| Metric (J-Butler) | Subprocess budget | Trip threshold (operational) |
|---|---|---|
| `iac_rt_p95_us` | ≤ 800 µs | `histogram_quantile(0.95, sum by (le) (rate(iac_rt_duration_us_bucket[5m]))) > 1500` AND `sum(rate(iac_rt_duration_us_count[5m])) > 100`, both true for `for: 24h` |
| `iac_rt_p99_us` | ≤ 2000 µs | Same shape with quantile=0.99 and threshold `> 5000`; same min-rate gate; same `for: 24h` |
| `cpu_sys_pct` | ≤ 15% | `avg_over_time(node_cpu_seconds_total{mode="system"}[5m]) / avg_over_time(node_cpu_seconds_total[5m]) > 0.15`, true for `for: 24h` |

The min-rate gate (`> 100 req/s` sustained over the trailing 5 minutes) suppresses idle-period noise — without it, an overnight quiet window with 1 spurious slow request would trip the p95. The 5-minute rate window plus the 24-hour `for:` clause means the kernel must be sustainedly slow — a single 1-minute evaluation in which either condition fails resets the `for` timer to zero. (See §4.7.1 for the histogram bucket boundaries that make `histogram_quantile(0.95, ...)` interpolate within explicit, not implementation-dependent, buckets at the 1500 µs SLO edge.)

Reference alert rule (drop-in for Prometheus):

```yaml
- alert: IacRtP95Breach
  expr: |
    histogram_quantile(0.95, sum by (le) (rate(iac_rt_duration_us_bucket[5m]))) > 1500
    and sum(rate(iac_rt_duration_us_count[5m])) > 100
  for: 24h
  labels: { severity: page, gate: release-block }
```

**J1 is the floor reference.** If J-Butler p95 > 4× J1 p95, the overhead is in our code, not the IPC. **Fix our code first.** Do not migrate to inproc to mask code-path overhead.

**Three-condition unlock check** (ADR-002):
1. Sustained 24h breach of one threshold above on a journey-required Spirit class.
2. Confirmation that J-Butler p95 is not >4× J1 p95 (i.e., the breach is not fixable code).
3. A follow-up ADR superseding ADR-002 is reviewed and merged. ADR-031 (Cross-Form Spirit Equivalence) resolves out of `speculative-vNext` at the same time.

**Bench cadence.** Runs in CI nightly. Results land in `benches/results/{date}.json`. Gate review at the end of each phase milestone; explicit go/no-go on inproc-unlock recorded in the milestone retrospective.

**Bench-results retention policy.** Daily results live 90 days hot in `benches/results/`. Beyond 90 days, results are aggregated to weekly summaries (`benches/results/weekly/{year}-W{wk}.json`) and the daily JSON files are pruned. Weekly summaries retained 1 year. Tagged-release benchmarks (`benches/results/release/{semver}.json`) are retained indefinitely under git LFS. Pruning runs in CI on the 1st of each month; the prune job opens a PR (not a force-merge) so an operator can audit what's leaving hot storage. Trend-tracking dashboards (Grafana) read from weekly summaries for >90-day windows; daily JSON for <90-day windows.

## 14. Open Questions

These are the genuine "I'm not sure" items. They are **not** blockers for v0.1 — they are signals for where the design will need to learn.

1. **Spirit hot-swap semantics for in-flight LLM streams.** Mid-stream, the predecessor's `on_swap_out` fires. What happens to the partial response? Drop it (waste a half-completion of a Sonnet-tier model, but keep semantics simple)? Hand it to the successor as a `partial_response` input (clean but every successor must know what to do)? Stash it in `private` memory for later retrieval (easy but never actually used)? **Lean: drop it, log it, charge the user, keep semantics simple.** Revisit if cost data says the dropped completions are material.

2. **Approval prompt fatigue.** Every survey project has it. The substrate (`prompt_with_diff`, persistent allow, posture presets) exists. What is missing is **the heuristic** for "this looks like the same kind of thing the user has approved before, batch it." Possible answers: per-(Spirit, capability, target-fingerprint) cached decisions; an LLM-mediated batcher; plain-English summary of "the next 10 things the agent wants to do" as a single approval. **Probably need real usage data to pick.**

3. **A2A trust establishment under churn.** TOFU + mTLS is the v1.0 plan for the bilateral case. For longer-running deployments where one of the two Hosts is replaced (laptop swap, prod-edge node migration), pin re-establishment is operator workflow. Probably need a documented playbook before v1.5 ships.

4. **Spirit class portability across kernels — committed to a triple.** Compatibility is `(kernel_version, abi_version, manifest_schema_version)` — a triple, not a pair. `abi_version` governs the `Spirit`/`KernelHandle` vtable + capability ID space (SemVer; major break = vtable layout or capability semantics change). `manifest_schema_version` governs the TOML surface independently. `kernel_version` is product-facing and includes both as a compatibility set. **Rule:** Spirit declares `abi`; kernel adapts down via `Compat` shim layer; N-1 supported, N-2 hard refusal with typed `EAbiTooOld`. **Deprecation:** 2 minor releases of warning, 1 major to remove. The live version-compatibility matrix lives in `STABILITY.md`. v0.5→v1.0 transition is breaking by design; documented in CHANGELOG with migration path.

5. **Loom-lite contention on the diagnostic-architect bilateral pair.** When Mira is fetching the latest detection patterns and Nash is publishing fix templates concurrently, Loom-lite is the hot service. Single-instance Postgres+pgvector at the bilateral scale is well-understood (well within Postgres's normal load envelope), but pattern-search latency under concurrent reads-and-writes is worth measuring. **The v1.5 deployment will reveal the right index strategy.**

6. **Researcher's "novel hypothesis" mode operationally.** A Researcher running on a corporate Host with constraints needs to know what is allowed for hypothesis generation (data-residency for the source material; provider selection for the cogitation; collective tier writes for the conjectures). Probably resolved by the operator's deployment policy. **Mark as PDP-integration test if a Spirit class needs PDP.**

7. **Mobile push UX for halt resolution.** Lunarpulse approves from his phone in the founder loop; Elena approves from her phone in the diagnostic-architect pair. For v1.0 the substrate ships HTTP push; native mobile clients are a v1.5 deliverable. Editor banners can lean on ACP's existing diff-display for in-editor resolution.

8. **Prompt-injection defense at the tool-output boundary.** The kernel hosts a generic post-tool-output filter; the **content** of the filter (what is a leak? what is instruction injection?) is data, not code. Ship a default rule pack; let Spirits add to it. **Will not be perfect; aim for "raises the bar."**

9. **What is the smallest viable Loom-lite?** Postgres + pgvector + a single MCP server is enough for the diagnostic-architect bilateral pair. The shape of the Loom-lite data model needs to be sound from day one even if v1.5 is the first deployment. **One serious schema review before v1.5 ships.**

10. **Cohort interop signal.** A first cohort project (openclaw / ironclaw / hermes / paperclip / rustain / codex) integrating MAOS as their substrate or interoperating cleanly via ACP/MCP/A2A is a v1.0 success criterion. **Cohort interop is a sociological signal, not an engineering metric.** It is achievable but not gateable.

## Appendix A — Cohort prior-art map

For each major design decision, here is the project we mined it from. Useful when reviewing.

| MAOS feature | Survey project | Why we picked it |
|---|---|---|
| In-process Spirit mailbox | codex `core/src/agent/{registry,mailbox}.rs` | Cleanest actor model in the cohort; supervised, depth-limited |
| 7-state ToolCall FSM | rustain `src/domain/services/tool_scheduler.rs` | Cancellation as first-class state |
| Sandbox primitives (Landlock+seccomp / Seatbelt / Win) | codex `codex-rs/sandboxing/` + `linux-sandbox/` | Only project shipping all three OS-native |
| ACP server NDJSON stdio | openclaw `src/acp/server.ts`, opencode `acp/agent.ts`, hermes `acp_adapter/server.py` | Convergent; ratified by Zed |
| MCP all-three transports (stdio / SSE / Streamable HTTP) | opencode `mcp/index.ts` | Future-compatible |
| Approval class taxonomy (6 classes) | openclaw `src/acp/approval-classifier.ts` | Most expressive taxonomy; battle-tested |
| Headless server + thin TUI | opencode | Clean split; mDNS optional discovery |
| Sub-Spirit `delegate_task` with depth cap | hermes-agent `tools/delegate_tool.py` | Recursion safety pattern |
| `*.md` memory file convention | universal in cohort | Table stakes |
| Hot-swap with token rebinding | codex's `SpawnAgentForkMode` lifted to whole-Spirit | Preserves in-flight context across role changes |
| Provider hot-swap via `ArcSwap` | rustain `src/infrastructure/startup.rs` | Live provider replacement is non-trivial |
| Hexagonal `domain/adapters/infrastructure` for the kernel layout | rustain `CLAUDE.md` | Survives kernel evolution |
| Approval Decision Log + Transparency Log split | openclaw audit suite | Two distinct audit needs |
| `cargo-deny`-style dep policy + SBOM/provenance | openclaw `docker-release.yml` + ironclaw `deny.toml` | Supply chain hygiene from day one |
| Daily-rotated journal for crash recovery | rustain `rustain.log.YYYY-MM-DD` | Cheap durability |
| Posture preset hierarchy (presets + custom) | gemini-cli `ApprovalMode` enum + openclaw classifier | Composable |
| Distillation-after-execution (`trajectory_compressor.py`) | hermes-agent | Canonical reference for the §9.5 pattern |
| Principal-namespaced memory model | hermes-agent | Lifted into kernel-allocated contract per ADR-026 |
| LSP-style Content-Length framing | LSP / language servers ecosystem | Boring, well-understood, abundant implementations |
| In-kernel notification rendering (TUI + ACP + native + push) | new — no precedent in survey | Required by the substrate's transparency invariants |

## Appendix B — Glossary diff vs. the survey

Where MAOS terms diverge from common cohort usage. Useful for newcomers.

| MAOS term | Cohort common term | Why we renamed |
|---|---|---|
| **Spirit** | "agent", "subagent", "agent class" | "Agent" is overloaded (process? persona? class?). "Spirit" disambiguates the *role + cognitive profile + posture* abstraction from a running process. |
| **Host** | not standardized | We need one word for "an OS process running the kernel" |
| **Posture** | "approval mode", "permission preset" | Posture is broader — it is the autonomy stance, of which approval policy is one component |
| **Capability Token** | not standardized; ad-hoc per project | Bringing OS-style explicit token semantics |
| **Loom-lite** | "collective KB", "pattern library", "shared memory" | The collective tier is the user's data, not the kernel's; the diminutive denotes the single-instance shape |
| **IAC frame** | "message", "event", "RPC" | Frame is closer to wire-format reality; doesn't conflate with LLM "messages" |

## Appendix C — What this document deliberately is NOT

To save reviewer time later:

- **Not a UI spec.** TUI / editor / mobile shells are application-level work. The kernel's notification primitives are documented; the visual designs are not.
- **Not a benchmark plan.** Performance targets are defined where they are load-bearing; broader benchmarking is out of scope until v0.5 ships and there are real numbers to optimize against.
- **Not a marketing document.** No tagline. No one-pager. Those are downstream.
- **Not a project plan.** §13's roadmap is sequence + validation milestones, not Gantt-able tasks.
- **Not a security audit.** §8 is a threat model and mitigation summary. A real audit happens before v1.0 ships (external pen-test with zero P0/P1 findings open).
- **Not the final answer on names.** "MAOS", "Spirit", "Loom-lite", "Host", "Posture" — all are working names. Renaming is cheap before v0.1.

## Appendix D — Terminal-Shape Sketches

This appendix describes terminal shapes — directions the architecture is biased toward but has not committed to. **Nothing in this appendix is binding.** Sections in the main body (§0–§14) reference appendix entries by ID (e.g., D.3) when a v0.1-or-later decision deliberately leaves room for one of these shapes.

The convention is simple: **if a behavior is in §0–§14, it is binding for its declared phase. If a behavior is in App-D / E / F, it is non-binding by construction.** A reader in the main body never has to ask "is this real?" — if it is in §0–§14, it is real.

### D.1 — Multi-host topologies beyond bilateral

Bilateral A2A (exactly two pre-paired Hosts, mTLS+TOFU) is the v1.5 commitment. Triadic and N-host meshes have appeared in cohort discussions (gateway-mediated, supervisor-fanout, peer-discovery DHT) but no journey in scope demands them. If a future deployment justifies one, the substrate's primitives (typed-intent consent, mTLS pinning, logical clock) extend additively. The wire format does not change.

### D.2 — In-process Rust Spirits unlocked via measurement gate

The v0.1 commitment is subprocess-only (ADR-002). The full measurement gate spec — harness, latency budgets per journey, Prometheus alert rules, three-condition unlock check — is **canonically specified in §13.1**. This appendix entry exists only to record the *terminal-shape* implication: if the gate trips and a superseding ADR lands, MAOS gains a second Spirit form (`rust-inproc`) without altering ABI for existing subprocess Spirits. ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext`; resolution depends on this gate firing.

Normative specification: §13.1.

### D.3 — Cognitive predicate vocabulary beyond universal arithmetic

The kernel exercises only four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`) per ADR-022. Spirit classes with richer cognitive needs (Negotiator comparing scalars to scalars; Tutor needing trajectory-shape predicates) may, in some future phase, justify a stdlib of additional predicates loaded from user-space. The substrate's current position: the universal-arithmetic surface is sufficient for the journeys this architecture ships. If a future Spirit class justifies an extension, the kernel surface stays untouched and the predicates live in a user-space stdlib (the ADR-039 number is reserved for that proposal; see §12).

### D.4 — Federation tier between `org-internal` and `public-untrusted`

The three-tier trust model (ADR-009) covers `local`, `org-internal`, `public-untrusted`. A federation tier — Spirits authored by one organization and consumed by a partnered organization — has appeared in compliance discussions. It would slot between `org-internal` and `public-untrusted` with a peer-organization signing key pinned in operator policy. No journey in scope exercises this; the slot is reserved by structure, not by code.

### D.5 — Hot-swap migration patterns beyond single-step

ADR-020 specifies single-step `migrate(predecessor_state) -> successor_state`. Multi-step migration (e.g., `v0.5 → v0.7 → v1.0` via two intermediate hops) would be useful when Spirit classes evolve schemas faster than operators upgrade kernels. The substrate's invariant (kernel refuses load with `EMigratorMissing`) extends to multi-step trivially via chained migrators; the operator UX for chain composition is the open question.

## Appendix E — v0.9+ Compliance Roadmap

This appendix describes the ComplianceClaim semantic evaluator that ships at v0.9 and the associated corpus methodology. The schema (defined in `maos-spirit-abi/src/compliance.rs`) is binding from v0.1; the **evaluator and corpus are non-binding until v0.9**, per ADR-005's general staging principle: stable schema first, semantic evaluation later.

### E.1 — CCAC staging table

Each phase has a falsifiable gate. The schema does not move; only the corpus and evaluator phase in.

| Version | Schema | Generation mechanism | Corpus N | Cross-Spirit agreement gate | Ship-blocking? |
|---|---|---|---|---|---|
| v0.1 | Frozen, validator implemented, emit pipeline live | N/A — mechanism specified, not exercised | 0 (smoke fixtures only, N≈10) | Schema validation 100%, emit-rate 100% | Yes — schema |
| v0.5 | Stable | **Mechanism A** (cross-Spirit independent re-decision on shared input set) | N=100, stratified across 4 decision classes | **Non-degeneracy criterion** (no fixed numeric floor — see E.1.1 below) | Yes — non-degeneracy gate |
| v0.7 | Stable | A + first calibration pass against v0.5 distribution | N≥150 (v0.5 N=100 + 50 fresh stratified additions), expanded to 6 decision classes | **First numeric agreement floor (deferred-numeric, formula-bound)** — see E.1.2 below for the formula | Yes — agreement floor at the formula-computed value |
| v0.9 | Stable | A + **Mechanism B** (planted-disagreement injection — validates the metric is not degenerate) | **N=600**, full stratification across 8 decision classes, balanced across Spirits | **±2% agreement** | Yes — full corpus delivered, full floor |
| v1.0 | Stable, deprecation policy for schema fields published | A + B + **Mechanism C** (drift detection: re-run v0.9 corpus quarterly, flag agreement regression >0.5%) | N=600 active + drift-corpus | ±2% on active, ≤0.5% drift quarter-over-quarter | Yes — GA gate |

**E.1.1 — v0.5 non-degeneracy criterion (replaces the placeholder ±5% floor).** The v0.5 acceptance criterion is *non-degeneracy*, not a fixed numeric agreement floor. The agreement metric must satisfy all three of:

1. **Computable** across at least 3 distinct ComplianceClaim instances on the v0.5 N=100 stratified corpus.
2. **Non-constant** — variance >0 across those instances. If the metric returns the same value on every input, it is not measuring agreement; it is measuring a constant.
3. **Directionally correlated** with independent reviewer judgment on a sample of **N ≥ 30 paired claims** (judge metric value, reviewer "agree / disagree / unclear" coded as +1 / −1 / 0). Spearman's **ρ ≥ 0.40** required (two-tailed p < 0.05; ρ_crit at N=30 is 0.364, so ρ ≥ 0.40 carries effect-size headroom above the significance threshold). Additionally: 95% bootstrap CI on ρ (10,000 resamples) MUST NOT cross zero. A passing gate establishes that the metric is not behaving as a random scorer; it does NOT establish metric fitness — see App-E.1.2 (planned, v0.7) for accuracy gates.

**Derivation of (N=30, ρ ≥0.40).** Critical values for Spearman's ρ at α=0.05 two-tailed: N=10 → ρ_crit=0.648; N=20 → 0.450; N=30 → 0.364; N=50 → 0.279. The earlier draft used N=10 with ρ ≥0.30 — at N=10, ρ ≥0.30 corresponds to p ≈ 0.4, which trips ~40% of the time on uncorrelated noise. (N=30, ρ ≥0.40) is the smallest pair that delivers a statistically discriminating gate while keeping the corpus burden modest. Bootstrap CI requirement closes the residual edge case where a single outlier inflates ρ.

If those three conditions hold, the metric is admissible to v0.7's tightening pass. v0.7 introduces the first numeric agreement floor (App-E v0.7 row, calibrated against the v0.5 distribution); v1.0 enforces ±2%.

**Why no v0.5 numeric agreement floor.** The original draft of this table proposed ±5% as the v0.5 floor. There was no derivation for ±5% — it was chosen because it felt looser than the v1.0 ±2% by enough margin to be obviously a calibration phase, but "felt looser" is not a floor. The v0.5 question is "is this metric meaningful at all?" not "does it hit 5%?" The non-degeneracy criterion answers the former honestly.

**Falsification rule.** At v0.5, if any of the three non-degeneracy conditions fail (N < 30, ρ < 0.40, or bootstrap CI crosses zero), ship is blocked or v0.5 is rebadged as v0.4-preview. No "we'll figure it out by v0.9."

**E.1.2 — v0.7 agreement floor (deferred-numeric, formula-bound).** At v0.7 release, the numeric agreement floor is computed as `floor_v0.7 = max(floor_v0.5, μ_v0.5 − k·σ_v0.5)` where `μ_v0.5` and `σ_v0.5` are the mean and standard deviation of the per-scenario Spearman ρ distribution measured across the v0.5 production window (minimum 30 scenarios, ≥ 14 days post-v0.5-GA), and `k = 1.0` (one standard deviation below v0.5 mean, clamped to never regress below the v0.5 floor). Until the v0.5 distribution is measured, v0.7 inherits the v0.5 non-degeneracy criterion verbatim. **The formula, the window, the minimum-N, and `k` are frozen at this revision; only the numeric output is deferred.** No numeric range, no "calibrated" judgment call at v0.7 release — just a deterministic computation against measured v0.5 telemetry.

### E.2 — Generation mechanisms

- **Mechanism A — Cross-Spirit independent re-decision.** Two reference Spirits independently emit ComplianceClaim verdicts on the same input set. Agreement metric: per-class verdict-equality rate. Any input where ≥3 Spirits disagree is flagged for human adjudication and excluded from the agreement floor.
- **Mechanism B — Planted-disagreement injection.** Synthetic inputs constructed to produce a known-correct verdict. If two Spirits disagree on a planted input where the verdict is unambiguous, the metric is degenerate (Spirit disagreement is noise, not signal). N=30 planted inputs per decision class.
- **Mechanism C — Drift detection.** Quarterly re-run of the v0.9 corpus against the current kernel + Spirit versions. Agreement regression >0.5% triggers a `ComplianceClaimDrift` audit ticket and a forced ADR review of any Spirit whose verdicts shifted.

### E.3 — Why the schema ships at v0.1 but the evaluator does not

If the schema were deferred to v0.9, every ComplianceClaim emitted between v0.1 and v0.9 would either be undefined (no schema) or would require a v0.9-breaking schema change at v0.9 (ABI break by construction). Shipping the schema at v0.1 means:

- Spirits emit well-formed ComplianceClaim objects from day one. The structural validator catches malformed emit; the semantic evaluator does not exist yet.
- Operators see the audit trail of emitted claims even before the evaluator scores them. The trail itself is useful — "the substrate has been emitting compliance claims since v0.1" is a stronger story than "compliance arrives at v0.9."
- The crate boundary makes the document boundary honest: the schema is in the wire-stable `maos-spirit-abi` crate; the evaluator is in `maos-compliance` (v0.9+, isolated, can break freely until v0.9 ships).

This is the pattern for any substrate-level commitment with a deferred validation surface: schema lands early, validation phases in. The same shape applies to halt-precision/recall (§6.3 — spec at v0.1, corpus by v0.5), Diego onboarding (§10.6 — staged), and replay determinism (§7.3 — v1.0 best-effort, v1.5 hard target).

## Appendix F — Distillation Pattern Body

§9.5 in the main body sketches the substrate interface (the five contracts the kernel honors so the pattern works). This appendix gives the **full Spirit-author convention prose** — the implementation guidance that does not need to live in the binding-v0.5 document body. Conventions here are non-binding; the binding floors are the five-metric gate in §9.5.

### F.1 — Reference implementation (hermes-agent)

[hermes-agent's `trajectory_compressor.py`](https://github.com/NousResearch/hermes-agent/blob/main/trajectory_compressor.py) is the canonical reference for distillation-after-execution. Its compression strategy:

- Protect first turns (system, human, first gpt, first tool) — uncompressed.
- Protect last N turns (final actions and conclusions) — uncompressed.
- Compress only middle turns into a single human summary message.
- Effective-temperature tuning per compressor model.
- Target-token-budget enforcement.
- Percent-sample compression for long trajectories.

MAOS distillation Spirit-authors should adopt this shape unless they have a domain-specific reason not to.

### F.2 — The pattern, step by step

1. **Raw frame lands in the Transparency Log.** Invariant I2 — the kernel does this regardless of distillation.
2. **Spirit-side LLM distillation** compresses the raw payload into a decision-relevant digest. The Spirit chooses the summarization model, the prompt, the token budget, and the redaction policy.
3. **Digest is written to working memory** (in-process Spirit state — no kernel involvement). Optionally elevated to **episodic memory** (`fs.write` to private namespace) for cross-session retention or **shared/collective memory** (existing `memory.share` capability) for inter-Spirit dissemination. Per Invariant I11, every persisted digest carries `source_log_ref` and `distillation_depth`; the kernel rejects digest writes that lack these fields.
4. **Active LLM context** contains digests + decisions + recent I/O + queued external input. Raw payloads are *not* in active context.
5. **Raw is recalled on demand** via `log.recall` when a downstream decision needs full evidence. Recall is auditable (recall-of-recall chain).
6. **Decisions record their digest grounding.** Invariant I12 — every `decision.*` frame the Spirit emits carries `working_memory_digest_refs` so post-hoc audit can prove which summaries the agent actually reasoned over.
7. **User-input queuing.** Human-originated frames arriving during in-flight work are buffered by the Spirit's persona logic and processed at safe sequence points (between task completions, before new dispatches) — preventing preemption of in-flight delegations.

### F.3 — Multi-hop generalization

Digests of digests compound information loss. `source_log_ref` flattens transitively at write time so any digest at any hop references the *original raw frames*, not intermediate digests. Auditors and Spirits walk a single hop from any digest back to raw evidence. `distillation_depth` is monotonic; Spirits may decide policy on max acceptable depth (e.g., halt-and-escalate at depth 3+).

### F.4 — Hermes-informed conventions

**First-turn / last-turn anchoring.** Distillation-shipping Spirits SHOULD preserve the original task statement (the first turn that initiated the work) and the final output (the closing turn that delivered the result) uncompressed in the digest. Compress only the middle. The v0.5 ship-gate test corpus measures task-preservation via cosine-similarity ≥0.95 between the digest's task-statement section and the original task statement.

**Target token budget.** Distillation-shipping Spirits SHOULD declare `target_max_tokens` per distillation invocation; default `max(2048, 0.15 × original_tokens)`, overridable per Spirit class via manifest `[distillation].target_max_tokens`. Compression ratios outside `[0.05, 0.25]` (relative to original) indicate either a compressor that is dropping content (too aggressive) or not compressing (too conservative); the v0.5 ship-gate flags both.

**Compressor model class.** Distillation reliability is downstream of compressor model quality. Spirit-author convention: the compression LLM call SHOULD use a model class ≥ Sonnet-tier or 70B+ open-weights, with temperature ≤0.3.

### F.5 — Acceptance criteria — derivation

This appendix derives the metric floor values whose normative current-version specification appears in §9.5 (Distillation Pattern interface, Table 9.5-1). Reference §9.5 for the values that govern conformance; this appendix explains how they were chosen and how to re-derive them when the threat model or operational data changes.

**Why these five metrics, not three or seven?** Recall, faithfulness, and traceability are the irreducible triple — without recall the digest is useless, without faithfulness the digest is misleading, without traceability the digest is unauditable. Hedge-preservation and secret-leakage were added when the bounded test populations (10⁴/10⁵) revealed two specific failure modes the irreducible triple did not catch: (a) digests that flatten "possibly X" into "X" pass faithfulness checks but degrade decision quality downstream, and (b) digests that summarize secret-bearing raw frames may leak the secret pattern even after pre-write redaction catches the literal token.

**Why ≥0.90 for digest-recall?** Held-out replicator LLM was tuned against the v0.5 R&D corpus; ≥0.90 is the threshold above which inter-replicator-LLM disagreement (the noise floor) dominates over true digest-quality variance. Below 0.90, the metric still discriminates between bad and good digests; at-or-above, the metric becomes noise-limited. ≥0.90 is therefore the highest meaningful floor.

**Why ≥0.98 for faithfulness, not ≥0.95 or ≥0.99?** Faithfulness measures unflagged contradictions — a stricter floor than recall because false positives here propagate into downstream decisions silently. ≥0.98 was chosen because the judge-LLM's own false-flag rate on the v0.5 R&D corpus measured ~0.5%; allowing a 2% unflagged contradiction window (1 - 0.98) leaves headroom above the judge's noise floor while gating real contradictions. ≥0.99 would be tighter than the judge-LLM's own resolution.

**Why ≥0.95 for hedge-preservation, gated on IAA ≥0.85?** Hedge-preservation is the only metric that requires inter-annotator agreement because hedges are linguistically ambiguous ("might be" vs "could be" vs "appears to" — different annotators score these differently). The IAA ≥0.85 floor (Cohen's κ) ensures the gold corpus is itself reliable before the metric becomes load-bearing. Below IAA 0.85, the hedge-preservation score is calibrated against noise. ≥0.95 on the metric itself is then the achievable target conditional on a reliable corpus.

**Why 0% for secret-leakage, not "≤N"?** Same argument as §7.2.1's zero-data-plane-error floor on mTLS rotation: any non-zero error budget on a security-critical path creates an incentive to suppress the metric rather than fix the underlying issue. Zero is absolute.

**Corpus size derivation.** 10 hedge-preservation cases is the minimum for the IAA computation to converge on the binary hedge/no-hedge label set. 10 contradiction cases is the minimum for the judge-LLM to discriminate above its own noise floor. 10 planted-secret cases is the minimum for the kernel's pre-write redaction to be exercised against each major secret class (API keys, capability tokens, mTLS private-key bytes — three classes × ≥3 instances). The 10⁵ secret-leakage corpus is a separate scaling assertion: at 10⁵ frames the false-negative rate of the redaction filter has measurable confidence intervals.

For current floor values and the canonical metric table, see §9.5.

### F.6 — Intent provenance interaction (I13)

Every digest also carries `intent_lineage: [intent_class, ...]` — the union of `intent` field values from all input frames it summarizes, computed by the kernel from `log.recall` tracking. Consumers operating under intent `Y` admit the digest only if `intent_lineage ⊆ allowed-promotion-set(Y)` declared in their manifest (typed error `EIntentPromotionDenied` on rejection). This closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. The mechanism is kernel-side (not Spirit-self-reported), which is what prevents the asymmetric-enforcement gap.

---

*Architecture is the practice of arranging trade-offs so future-you can change your mind without burning everything down. Thirty-nine ADRs, fourteen invariants, ten open questions. The substrate ships in six phases, terminating at v1.5 with the diagnostic-architect bilateral pair operational. The kernel grows slowly so the ecosystem can grow fast. The hermes-tenant positioning claim cashes at v1.0 through capability tokens, the Transparency Log, the Approval Decision Log, the Spirit registry with three trust tiers, the ComplianceClaim envelope, the cross-Spirit memory isolation corpus, and the externally-verifiable uninstall receipt. Same primitives compose, by configuration alone, into single-user laptop deployments and diagnostic-architect production pairs — without architectural rewrite at any tier transition.*
