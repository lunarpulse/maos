---
title: 'MAOS — Modular Agentic Operating System'
subtitle: 'Executive Product Brief'
author: 'Lunarpulse'
date: '2026-05-05'
status: 'Vision-locked, pre-implementation'
companion_artifacts:
  - 'architecture-maos.md (the decision source)'
  - 'maos-design-report.md (the conceptual companion)'
  - 'spirit-development-and-sharing.md (the third-party Spirit author guide)'
  - 'maos-kernel-implementation-guide.md (the kernel-implementer guide)'
  - 'industrial_agents.md (the user journeys)'
---

# Product Brief: MAOS — Modular Agentic Operating System

> **[DELTA-2026-07-06]** This brief is the **vision document** (2026-05-05) and remains the canonical statement of ambition, differentiators, and the "out forever" list. Its journey numbering ("Journey 10/11/12"), 20-week v1.0 timeline, and phase tags are **superseded by the PRD** (`prd/user-journeys.md`, `prd/project-scoping-phased-development.md`) — plan against those, not this file.

## Executive Summary

The AI agent landscape in 2026 is a fragmented archipelago. Claude Code, Codex, Gemini CLI, opencode, Cursor — each is a competent agent, each is a closed runtime. Teams that want multi-agent collaboration must accept one vendor's lock-in, cobble together brittle integrations, or build their own substrate from scratch. There is no portable, capability-isolated, vendor-neutral runtime for arbitrary AI agents — and emphatically nothing that lets a personal "Butler," a team's peer-mesh, a production diagnostic-architect pair, and a 28-agent enterprise nervous system all run on the same primitives.

**MAOS is that substrate.** It is a kernel — invariant, auditable, replaceable — on which specialized agents (called *Spirits*) are loaded, swapped, and composed like processes on a conventional OS. The kernel exposes one stable contract (the Spirit ABI) and seven services (scheduler, memory, security, I/O, IAC, capability registry, telemetry). Spirits are hot-swappable: a Butler today, a Tutor tomorrow, a Wet-Lab Coordinator next year — none requires kernel changes. The same primitives compose, by configuration alone, into single-user laptops, team meshes, diagnostic-architect pairs, and continent-spanning enterprise Cortex deployments. **One substrate, many shapes, infinite Spirits.**

The architecture is vision-locked, pre-implementation, with five core planning artifacts (~430 KB / ~6,500 lines) covering the full design from research foundation through kernel-implementation guide. v0.1 is roughly six weeks for a focused implementer; v1.0 (team-ready, third-party Spirits shipping) is twenty weeks; v2.0 (enterprise Cortex, WASM Spirit ecosystem) is the eighteen-month milestone.

## The Problem

Every agent ecosystem today picks one of three failed answers to "what should the substrate be?":

1. **Vendor-monolithic.** Claude Code, ChatGPT app, Cursor — the agent and its runtime ship together. Want a different model? Different language? Different sandbox tier? You can't have it. Want two agents that respect each other's boundaries? They have to be built by the same vendor.
2. **Cobble-it-yourself.** Use LangChain or AutoGen and stitch agents together. The substrate is whatever you assembled this week. No durable transparency log. No capability tokens. No standard memory model. Multi-host coordination is an exercise.
3. **Roll your own kernel.** What ironclaw, paperclip, hermes, openclaw, and rustain each did. Each is excellent within its scope; none is the others' substrate; none generalizes to "an agent class we haven't imagined yet."

The cost of the status quo: **trust is built on vendor promises, not substrate guarantees.** A user installing a third-party agent has no kernel-level enforcement that it cannot bypass approvals, leak secrets, exceed budgets, or hallucinate confidently into production. Every agent ecosystem is one supply-chain incident away from catastrophic blast radius. Every multi-agent collaboration starts from "trust me." Every enterprise that wants to deploy 28 agents across 14 sites discovers there is no substrate to deploy them onto — only point solutions that stop short of the journey.

## The Solution

MAOS separates the **kernel** (invariant; small; ~15 Rust crates) from **Spirits** (hot-swappable; manifest-declared; any language). The kernel guarantees eight things — every external call goes through a capability token, every IAC interaction is logged before delivery, every approval decision is auditable, lifecycle transitions are journaled, in-flight work survives hot-swap, sandbox profiles are enforced, manifest-declared policies are non-bypassable, and the kernel itself stores no secrets and learns no patterns.

Spirits do behavior; the kernel does infrastructure. A Spirit's binary contains lifecycle hooks, IAC handlers, decision logic, and a system prompt — and *nothing else*. No HTTP libraries, no LLM SDKs, no MCP clients, no socket code. Those are kernel-provided adapters. The result: Spirit binaries stay small, polyglot ecosystems become natural, and every external call uniformly hits the audited Capability Registry path.

The architecture is **hexagonal** for static structure (domain core / kernel services / adapter ring), with the **actor model** on the runtime hot path (each Spirit is a Tokio-supervised actor with a bounded mailbox, no shared mutable state). Reactive properties — responsive, resilient, elastic, message-driven — fall out as emergent. Three Spirit forms ship over time: in-process Rust at v0.1, subprocess binary at v1.0 (the first third-party-shippable form), WASM component at v2.0 (capability-isolated by construction; the form that powers the public ecosystem).

## What Makes This Different

- **Substrate, not product.** Other agent tools sell you the agent. MAOS gives you the OS underneath. The Spirits ship separately, by anyone, in any language.
- **Human transparency as kernel invariant, not application choice.** "No invisible actions, no puppeting, no asymmetric knowledge" (from Journey 10's design) is enforced by the kernel before any Spirit gets to author behavior. Spirits cannot bypass it.
- **Generality designed in, not retrofitted.** Three case-studied Spirits that don't yet exist — Negotiator, Tutor, Wet-Lab Coordinator — slot in with zero kernel changes. The architecture welcomes agents not yet imagined; the kernel grows slowly so the ecosystem can grow fast.
- **Multi-agent topology native.** All four orchestration patterns (supervisor/worker, blackboard, market-based, peer-to-peer) work on the same kernel primitives. The kernel arbitrates nothing — that's the point.
- **Epistemic halt as a Layer-1 capability.** Hallucination becomes a user-mediated, audit-trailed event, not a silent regression. When a Spirit's evidence is insufficient or contradictory, it halts; the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. This is novel — no prior agent runtime makes "I don't know" a first-class outcome.
- **Trust tiers + sandbox-floor enforcement** make a public Spirit registry safe by default. Untrusted Spirits get T2 sandbox + cautious posture regardless of what their manifest claims. Decentralized vetting (anyone can be a vetting authority via attestation signatures) avoids the App Store gatekeeper failure mode.

The unfair advantage is *integration coherence*: every part of the substrate (capability mediation, IAC, transparency log, approval surface, hot-swap, sandbox enforcement) is designed against the same eight invariants. There's no patchwork of "we'll bolt on auditing later" — auditing is the kernel's reason for existing.

## Who This Serves

**Primary — power users, developers, knowledge workers** who want a personal agent ecosystem they own. Their Butler anticipates calendar conflicts. Their Researcher conducts rigorous literature surveys. Their Architect drives coding tasks against their style guide. All running locally, all auditable in their own Transparency Log, none owned by a vendor.

**Secondary — agile teams (5–10 people)** who need multi-agent collaboration with no surveillance overtones. Each teammate runs MAOS as a peer; A2A mesh respects consent; no agent owns another's. The substrate replaces the coordination tax of Jira + Confluence + Slack with agent-to-agent context flow that humans see and approve at every step.

**Tertiary — enterprises (CTO / VP-Eng tier)** with cross-site engineering needs. Mira-class diagnostic agents on production edges, Nash-class architects in dev centers, Loom-curated pattern libraries propagating fixes across continents. The Cortex deployment is the proof: 28 agents across 14 sites, 4 dev centers, 3 continents — humans govern at decision gates only; the substrate makes the deployment safe by construction.

## Success Criteria

| Phase | Concrete validation milestone |
|---|---|
| **v0.1** | Architect Spirit drives a real coding task on a local repo end-to-end with approval prompts. ~6 weeks; one focused implementer. |
| **v0.5** | Single-user has working Butler, Researcher, Observer, Architect on a laptop. ~12 weeks total. |
| **v1.0** | 8-person team uses peer A2A mesh end-to-end; **Journey 10 reproducible**. Third party can author and ship a Spirit binary independently of the MAOS source tree. ~20 weeks total. |
| **v1.5** | Mira-Nash diagnostic-architect pair closes prod-to-fix loop in ~90 minutes; **Journey 11 reproducible**. ~28 weeks total. |
| **v2.0** | Cortex deployment at small scale (3-region pilot); **Journey 12 reproducible**. WASM Spirit registry live with signing + trust tiers. ~18 months total. |

Beyond shipping milestones, success looks like: a third-party Spirit registry with non-Lunarpulse-authored Spirits; a published 12-factor-aligned implementation that other open-source agent projects cite; and adoption signal from at least one of the cohort projects (openclaw / ironclaw / hermes / paperclip) integrating MAOS as their substrate or interoperating cleanly via ACP/MCP/A2A.

## Scope

**In v1.0:** the kernel (~15 crates), six reference Spirits (Butler, Researcher, Architect, Diagnostic Engineer, Enterprise stub, Observer), the Spirit Wire Protocol, the Spirit dev SDK, the registry contract, MCP/ACP/A2A integration, three sandbox tiers (T0/T1/T2/T3), Ed25519 signing, four trust tiers, in-process Rust + subprocess Spirit forms.

**Out for v1.0:** WASM-component Spirit form (deferred to v2.0), payment/monetization, registry quota enforcement, mobile push notifications, model fine-tuning infrastructure, eval-suite execution as a kernel feature, multi-instance Loom (deferred to v2.0).

**Out forever (or until forced):** central App Store-style review of every Spirit, vendor-monolithic deployment, hidden state in the kernel, audit-bypass capabilities for "trusted" Spirits, automatic compose-orchestration-pattern selection.

## Vision

In two to three years, MAOS is the Linux kernel of agentic computing — the boring, trusted substrate underneath specialized agent ecosystems. Spirits are published like npm packages but trusted like Cargo crates: signed, capability-isolated, audited at every kernel boundary. Users compose personal ecosystems from third-party authors; teams run peer-mesh deployments without vendor coordination; enterprises run cross-continent Cortexes that govern themselves at decision gates and amplify human judgment everywhere else.

The deeper bet: that **trust between humans and agents will be built on kernel invariants, not vendor promises**. When an enterprise deploys 28 agents into production, the question "do we trust this?" is answered by reading the substrate's invariants and audit trails, not by reviewing thirty separate vendors' security posture documents. When a user installs a third-party Tutor Spirit, the question "what can this thing do?" is answered by reading its manifest against the architecture's known capability surface, not by hoping its developer is honest.

If MAOS works, agentic computing stops being a fragmented archipelago of single-purpose tools and starts being an ecosystem in the way Unix was, the way Kubernetes is, the way the web is — small kernel, large surface, generative composability, trust grounded in transparent mechanism rather than vendor reputation.

That's the bet. The architecture is ready. Implementation is the next chapter.

---

## Appendix — Key technical commitments (for reviewers)

A condensed reference for technical reviewers; full detail in `architecture-maos.md`.

| Decision | Reference | Rationale |
|---|---|---|
| Rust + Tokio kernel | ADR-001 | Cohort survey unanimous (codex, ironclaw, rustain); type-safe invariants; codex's actor pattern proven at scale |
| Hexagonal architecture (not Clean) | ADR-010 | Clean's call-direction discipline doesn't fit a runtime kernel; hexagonal gives multi-adapter-per-port flexibility |
| Actor model on the hot path | ADR-011 | Each Spirit = Tokio-supervised actor with bounded mailbox; codex precedent; four free properties (backpressure, lock-free, fault isolation, hot-swap) |
| Three Spirit forms over v0.1/v1.0/v2.0 | ADR-007 | Phased rollout: rust-inproc (v0.1) → subprocess (v1.0) → wasm-component (v2.0); same manifest contract across all forms |
| Spirit registry as MCP-Streamable-HTTP server | ADR-008 | Kernel already speaks MCP for tools and Loom; zero new transport code |
| Trust tiers + strictest-of-floor enforcement | ADR-009 | Public-untrusted Spirits forced to T2 sandbox + cautious posture regardless of manifest; decentralized vetting via attestations |
| Hexagonal sandboxing (T0–T4) | ADR-004 | OS-native (Landlock+seccomp / Seatbelt / WinRT) for shell exec; WASM capability for tool plugins; composed not picked |
| MCP for tools, ACP for editor bridges, A2A for peer mesh | architecture §7 | Industry-aligned protocol choices; reuse existing infrastructure |
| Loom is user-space, not kernel | ADR-006 | Kernel stores no patterns; Loom is the user's data; replaceable per deployment |
| Per-tag epistemic policy | architecture §4.6.1, §5.1 | Verbalize / flag / halt taxonomy; halt is the rare alarm, not the doorbell |
| Kernel-mandated explanation predicates | architecture §5.1 | Proactive Spirits must carry structured "because" payloads; addresses the psychological reactance / self-threat failure mode |

**Eleven Architecture Decision Records** (ADR-001 through ADR-011) capture the contested decisions with their rationale and revisit triggers. Every ADR follows a four-paragraph template: Decision / Alternatives considered / Rationale / What would force a revisit.

**Ten kernel invariants** (I1–I10) are non-negotiable runtime guarantees: capability mediation (I1), log-before-deliver (I2), explicit auto-response marking (I3), approval persistence (I4), memory scope enforcement (I5), hot-swap token preservation (I6), telemetry as broadcast (I7), A2A bilateral consent (I8), kernel as state-free / pattern-free (I9), lifecycle journaling (I10).

The corpus is implementation-ready. The architectural shape is fixed. The remaining work is typing, testing, and the slow accumulation of trust that the substrate does what it claims.
