---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional']
releaseMode: phased
classification:
  projectType: 'developer_tool'
  secondaryTraits: ['cli_tool', 'api_backend', 'desktop_app']
  domain: 'scientific'
  domainNote: 'Agent infrastructure sub-domain — inherits concerns from scientific computing (reproducibility, validation), developer tooling (SDK, framework), and enterprise security (audit, trust tiers). Some deployments touch fintech/healthcare-adjacent compliance.'
  complexity: 'high'
  projectContext: 'greenfield'
inputDocuments:
  - '_bmad-output/planning-artifacts/maos-product-brief.md'
  - '_bmad-output/planning-artifacts/architecture-maos.md'
  - '_bmad-output/planning-artifacts/maos-design-report.md'
  - '_bmad-output/planning-artifacts/spirit-development-and-sharing.md'
  - '_bmad-output/planning-artifacts/maos-kernel-implementation-guide.md'
  - '_bmad-output/planning-artifacts/industrial_agents.md'
  - '_bmad-output/planning-artifacts/research/technical-ai-agent-frameworks-and-coding-tools-comparative-architectural-analysis-research-2026-05-04.md'
  - '_bmad-output/planning-artifacts/report-gemini.md'
documentCounts:
  briefs: 1
  research: 1
  brainstorming: 0
  projectDocs: 5
projectType: 'greenfield'
workflowType: 'prd'
projectName: 'maos'
---

# Product Requirements Document — MAOS (Modular Agentic Operating System)

**Author:** Lunarpulse
**Date:** 2026-05-05 (Step 8 phase-restructure: 2026-05-06)
**Status:** In progress (PRD workflow Steps 1, 2, 2b, 2c, 3, 4, 5, 6, 7, 8, 9, 10 complete; awaiting Step 11 — Polish Document)

> **📍 Canonical phasing source:** This PRD's Step 8 (Project Scoping & Phased Development) defines the canonical phase structure: v0.1 Foundational → v0.3 Butler → v0.5 Researcher + Observer → v0.8 Founder Loop wedge demo → v1.0 Team-ready → v1.5 Diagnostic-Architect → v2.0 (technical) → v2.5 (ecosystem-adoption). The companion architecture documents (`architecture-maos.md`, `spirit-development-and-sharing.md`, `maos-kernel-implementation-guide.md`) reflect an earlier phasing where v0.1 = Architect Spirit driving a coding task; those docs are scheduled for propagation to match this PRD's structure. Until propagation completes, **the PRD is the canonical source for phasing decisions**; the architecture docs remain canonical for everything else (ADRs, invariants, kernel internal architecture).

## Executive Summary

MAOS — Modular Agentic Operating System — is an open-source kernel substrate for agentic computing, built in the reference class of Linux, Postgres, Kubernetes, and Apache HTTPD: infrastructure that crystallizes a layer before any commercial integrator can enclose it. The kernel is invariant; specialized agents (called *Spirits*) are hot-swappable modules loaded against a stable Spirit ABI. The same primitives compose, by configuration alone, into single-user laptop deployments, 5–10 person team peer-meshes, diagnostic-architect production pairs, and continent-spanning enterprise Cortex deployments — without architectural rewrite at any tier transition.

The problem MAOS addresses is structural, not incremental. The current AI agent landscape offers three failed answers to "what should the substrate be?": vendor-monolithic runtimes (Claude Code, Cursor, ChatGPT app — agent and runtime ship together; no portability, no third-party agents, no auditable trust), cobble-it-yourself stacks (LangChain, AutoGen — substrate is whatever you assembled this week; no durable transparency log, no capability tokens, no standard memory model), or roll-your-own kernels (the cohort — openclaw, ironclaw, hermes, paperclip, rustain — each excellent within scope, none generalizes). The cost of the status quo: trust is built on vendor promises, not substrate guarantees; every agent ecosystem is one supply-chain incident from catastrophic blast radius; every multi-agent collaboration starts from "trust me." MAOS replaces vendor-promise trust with **kernel-invariant trust**: every external call mediated by capability token, every IAC interaction logged before delivery, every approval auditable, lifecycle journaled, sandbox profiles enforced — eight non-negotiable guarantees the kernel makes to every Spirit and every operator regardless of deployment tier.

Three user tiers ground the work, each with a substrate-shaped JTBD. **Tier 1 (solo power user / developer / knowledge worker):** *"When I'm coding alone at midnight, run a swarm of agents on my own machine against my own code without my repo or my context leaving my laptop."* **Tier 2 (5–10 person agile team):** *"When my team is shipping a feature, let our agents share context and divide work the way we already do in Slack, without us having to babysit which agent knows what — and without surveillance."* **Tier 3 (OSS infrastructure community as customer of substrate-proof):** *"When a 14-site research consortium runs a coordinated 28-agent investigation, prove the same kernel that ran my laptop's 3 agents handles the mesh — same primitives, same invariants, no rewrite."* The Tier 3 customer is the OSS movement watching, not an enterprise procurement office.

### What Makes This Special

Five load-bearing differentiators, ranked under the OSS-substrate reference class:

1. **Substrate-positioning (load-bearing).** Under OSS infrastructure framing, "we are a substrate" *is* the value proposition — it licenses the contribution model, the protocol-first design, and the long-horizon ambition. Reference precedents (Linux, Postgres, K8s, Apache) all won as "substrate-too-early" by commercial measure precisely because the substrate layer crystallized before commercial integrators could enclose it.
2. **Transparency as kernel invariant (load-bearing × 2).** "No invisible actions, no puppeting, no asymmetric knowledge" enforced by the Transparency Log and IAC Bus before any Spirit gets to author behavior. This is the architectural commitment competitors cannot bolt on without a rewrite. It is also what enables enterprise OSS adoption — the property Postgres has and MySQL did not in regulated shops.
3. **Generality designed in, expressed as protocol guarantee.** The Spirit ABI is specifiable independent of the MAOS reference implementation. Reference Spirits ship as forkable, replaceable proof-of-generality demos rather than as committed first-party features. The kernel hosts agent classes the architecture has not imagined.
4. **Multi-agent topology native.** All four classical orchestration patterns (supervisor/worker, blackboard, market-based, peer-to-peer) work on the same kernel primitives. The kernel arbitrates nothing — that neutrality is the architectural novelty under the K8s-style reference class.
5. **Epistemic halt as Layer-1 capability.** When a Spirit's evidence is insufficient or contradictory, the kernel exposes a structured halt; the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. Hallucination becomes a user-mediated, audit-trailed event — not a silent regression. Most defensible moat in the architecture; no prior agent runtime exposes this as a kernel guarantee.

Trust tiers + sandbox-floor enforcement (deployment configurations rather than primary differentiator) make a public Spirit registry safe by default — public-untrusted Spirits forced to T2 sandbox + cautious posture regardless of manifest claims.

**The core insight that licenses every differentiator:** separate Spirit behavior from kernel infrastructure cleanly enough that the kernel grows slowly and the ecosystem grows fast. A Spirit's binary contains lifecycle hooks, decision logic, and a system prompt — and nothing else. HTTP libraries, LLM provider SDKs, MCP clients, sandbox runtimes are kernel-provided adapters. This separation is what enables polyglot Spirit ecosystems, small Spirit binaries, uniform audit boundaries, and the v2.0 WASM-component Spirit form without re-architecture.

**Why users choose MAOS over alternatives.** Claude Code makes you a faster developer. Cursor makes you a smarter editor. ChatGPT makes you a better thinker. **MAOS makes you a person who can sleep while a system is sick, because there is a trustworthy presence sitting with it.** The skeptic's gut-check is "would I let this thing be alone in my house?" — and MAOS is the first runtime where the question is the whole product, with the answer engineered into the kernel rather than promised by a vendor.

## Project Classification

- **Project Type:** `developer_tool` — SDK, framework, and operator CLI; Spirit-author-as-customer is the load-bearing relationship. Secondary characteristics include `cli_tool` (the `maosctl` operator surface), `api_backend` (ACP server, A2A peer, control-plane HTTP, MCP-Streamable-HTTP outbound), and `desktop_app` (one Host process per machine).
- **Domain:** `scientific` — closest match in the standard taxonomy; properly *agent infrastructure*, a sub-domain that inherits concerns from scientific computing (reproducibility via rollout journals, validation methodology via per-Spirit eval suites, performance budgets), developer tooling (SDK ergonomics, polyglot reach, registry/distribution), and enterprise security (audit trails, capability tokens, trust tiers, PDP integration for compliance-bound deployments).
- **Complexity:** **High.** Multi-language ecosystem (Rust kernel + polyglot Spirits via Wire Protocol). Real-time streaming constraints. Cross-process coordination across three Spirit forms (rust-inproc / subprocess / wasm-component, phased v0.1/v1.0/v2.0 per ADR-007). Hot-swap with token preservation. Cross-host A2A with mTLS+TOFU+per-frame consent. Multi-tenant Loom for v2.0 Cortex. Eleven Architecture Decision Records and ten kernel invariants commit the design.
- **Project Context:** **Greenfield.** Pre-implementation, vision-locked architecture, no existing codebase. The architectural foundation already comprises ~430 KB of planning artifacts (architecture decisions doc, design report, Spirit dev guide, kernel implementation guide, three industrial user-journey vision documents, comparative research foundation, executive product brief).
- **OSS character (project commitments):** Apache 2.0 + MIT dual-license; public RFC process for ABIs and protocols post-v0.1; reference implementation, not gatekeeping; standards ambition for the integrated protocol surface (ACP + MCP + A2A composed coherently).
- **Substrate-positioning claim (v1.0 commitment, founder-blessed via Mary's hermes-comparison framing):** **MAOS at v1.0 can host a hermes-class Spirit as a tenant, with full audit, revocation, and substrate-uninstall guarantees that hermes-as-application cannot itself provide.** Hermes is one excellent improv actor with her own travel kit. MAOS is the theater she could perform in. The test of a theater is not how good a single play is — it's how many different plays can be staged on the same stage with the same safety guarantees. Empirical validation: v1.0 black-box external-author trial (Murat's ship gate — 5 external authors, 14-day no-DM-support window, ≥4/5 produce a working signed Spirit binary). If this claim survives that gate, the substrate has earned its theater. If not, we discover the real scope problem at v1.0 instead of v2.5.

## Carry-Forward Signals from Vision Discovery

Critiques surfaced during Step 2b party-mode validation are deliberately not absorbed into this Executive Summary; they are recorded here as carry-forward signals to the PRD steps where they belong:

| Signal | Origin | Lands at |
|---|---|---|
| Wedge pain commitment — pick one of (W1) multi-agent orchestration + persistence / (W2) local-first runtime / (W3) provider-agnostic orchestration — by v0.2 | 📋 John (Round 2 + 3) | Step 4 (User Journeys) for Tier 1's killer-workload journey; Step 8 (Scoping) for v0.1 acceptance criteria |
| Tier 3 is a proof tier rather than a user tier — restructure JTBD or voice it from a framework-builder | 📚 Paige + 📋 John (Round 3) | Step 4 (User Journeys) |
| Cortex consortium target — name a candidate by v0.3 (federated research / OSS project's own infra / university public-good) | 📋 John (Round 2) | Step 4 (Cortex deployment journey); Step 8 (v2.0 acceptance) |
| Legibility ≠ audit — distinct kernel invariant alongside auditability | 🎨 Sally (Rounds 1 + 3) | Step 10 (Non-Functional Requirements) |
| Eight kernel guarantees — enumerate inline as the FR spine | 📚 Paige (Round 3) | Step 9 (Functional Requirements) |
| Killer Tier 1 standalone-utility before substrate ambition | 📊 Mary (Round 2) | Step 4 (User Journeys); Step 8 (Scoping) |
| 4-crate vs 14-crate v0.1 scope decision | 💻 Amelia (Round 2) | Step 8 (Scoping) |
| Halt-recall / halt-precision benchmarks before v1.0 | 🧪 Murat (Round 1) | Step 10 (NFRs); v1.0 acceptance criteria |
| Formal methods (TLA+/Alloy) for invariants I5, I6, I9 | 🧪 Murat + 💻 Amelia | Step 10 (NFRs); deferred to v0.5 evaluation per Amelia |
| OSS recruitment strategy — Spirit SDK + cargo-generate + public registry day one + 5 bait Spirits + Spirit Jam at v0.3 + ≥5 non-Lunarpulse Spirits in registry by v0.5 | 📊 Mary + 📋 John (Round 2) | Step 4 (recruitment journey); Step 8 (v0.5 scope) |
| 6-month leading indicator: external Spirit count, protocol citations, the boring fork | 📊 Mary (Round 2) | Step 3 (Success Criteria) |

These are not forgotten — they are routed.

## Success Criteria

Success criteria for MAOS reflect its OSS-infrastructure-standard character: the goal is **adoption as the standard**, not revenue capture. Metrics are therefore weighted toward ecosystem health, technical correctness, and felt-trust signals rather than commercial outcomes. Three categories — User, Business (Adoption/Community), and Technical — converge on what "the substrate is working" means.

### User Success

User success is felt before it is measured. The substrate's promise is that a user can run agents that act on their behalf without losing track of what those agents are doing or why. Felt-trust is the engineering target.

- **Tier 1 (solo power user) — first-30-minutes test:** A new user installs MAOS, loads at least one Spirit, and completes one useful action — measured as "agent-driven outcome the user accepts and keeps" — within their first 30 minutes. Failure to meet this bar by v1.0 indicates the install path or the first-Spirit experience is too friction-laden for OSS-style adoption.
- **Tier 2 (team peer-mesh) — Day-30 transparency-log glanceability:** On day 30 of team usage, ≥70% of team members report glancing at the transparency log during a typical work week. This is Sally's load-bearing critique made measurable: mandatory transparency that nobody reads is theatre. If users have stopped looking by day 30, legibility has failed regardless of audit completeness.
- **Tier 3 (substrate proof in OSS / research community):** A 14-site research consortium successfully deploys a 28-agent Cortex using only the public MAOS kernel — same primitives, same manifests, no architectural fork. The deployment publishes its experience publicly (paper, conference talk, or blog series). This is the binary "did the substrate generalize" test.
- **Felt-trust dashboard (per Tier, exposed via Telemetry Stream):**
  - **Surprise rate:** percentage of agent actions the user marks "I didn't expect this." Target: declining over time within a single user's deployment as the agents calibrate to the user's surprise budget.
  - **Halt acceptance rate:** percentage of `epistemic.halt` invocations the user resolves with `provided_context` or `accepted_halt` rather than `authorized_override`. Target: ≥80% by v1.0 — meaning the Spirit's halt judgment is right >4-of-5 times.
  - **Digest open rate:** percentage of "what did your Spirits do while you were gone" digests the user actually opens. Target: ≥60% by v1.0 for daily active users.
  - **Time-to-first-trust by tier:** median sessions until a user shifts a Spirit from `cautious` to `assistive` posture. Target: <10 sessions for Tier 1; <30 for Tier 2; gated by external review for Tier 3.

### Business Success (Adoption / Community Health)

For an OSS infrastructure standard, "business success" is community velocity and ecosystem indicators — not revenue. The reference class is Linux/Postgres/K8s adoption curves at equivalent post-launch phases.

- **Phase validation milestones (binary at each release):**
  - **v0.1 milestone:** The Architect reference Spirit drives a real coding task on a local repository end-to-end with approval prompts. Six ACs from the kernel implementation guide pass in CI.
  - **v0.5 milestone:** A single user runs all six default Spirits on a laptop simultaneously (Butler, Researcher, Architect, Diagnostic Engineer, Enterprise stub, Observer). Sandbox tiers T0–T3 active. Transparency log persisted.
  - **v1.0 milestone:** An 8-person team reproduces Journey 10 (Team Nexus) end-to-end with peer A2A mesh, mTLS, per-frame consent, role queries. **A third party authors and ships a Spirit binary independently of the MAOS source tree** — the "first non-Lunarpulse Spirit in the registry" milestone.
  - **v1.5 milestone:** Journey 11 (Mira & Nash) reproducible — diagnostic-architect Spirit pair closes a prod-incident-to-deployed-fix loop in ≤90 minutes with humans gating only at architectural decisions.
  - **v2.0 milestone:** Journey 12 (Cortex) reproducible at small scale (3-region pilot, ≥14 sites if feasible at pilot scope, ≥10 agents minimum). WASM Spirit registry live with Ed25519 signing + four trust tiers operational.
- **OSS leading indicators (the trackable signals):**
  - **Month 6 (post-v0.1):** ≥3 Spirits in the public registry whose `Cargo.toml` author is not Lunarpulse. ≥1 "boring fork" (someone forked, modified, ran). ≥1 protocol citation (a third party's blog post / RFC / implementation references the Spirit ABI as an interface they're targeting).
  - **Month 12:** ≥10 external Spirits. ≥3 protocol citations from independent agent projects. At least one cohort project (openclaw / ironclaw / hermes / paperclip / rustain) interoperating cleanly via ACP/MCP/A2A or integrating MAOS as substrate.
  - **Month 18:** First "Spirit Jam" event held at v0.3; ≥5 community-authored Spirits emerging from it. Total external Spirit count ≥20.
- **Single most diagnostic question at month 6:** "Has someone we have never met shipped something that depends on MAOS's protocol surface?" Yes/no. Binary. *No* is a falsification of the substrate-thesis; partial-yes is an early-confirmation signal.

### Technical Success

Technical success means the eight kernel guarantees and ten kernel invariants hold under stress, the performance budgets are met, and the substrate's claims are empirically verifiable rather than aspirational.

- **All 10 kernel invariants (I1–I10) empirically verified by v1.0** through a per-invariant property test suite. Every invariant has at least one falsifiable predicate documented and a property-based test running continuously in CI. No predicate, no claim — the invariant is downgraded from "guaranteed" to "asserted" until tested.
- **Adversarial Spirit suite passing by v1.0:** a maintained pen-test pack of malicious Spirits attempting to violate each invariant via memory residue, timing side channels, capability scope evasion, IAC log bypass attempts. Failure to break invariants becomes evidence that the kernel guarantees are real. This is non-negotiable for the "trust grounded in kernel invariants" claim.
- **Hot-path performance budgets enforced as CI gates by v1.0:**
  - `iac/send` (same-Host): <10μs P99
  - Capability token issuance (cached posture, no prompt): <5μs P99
  - Capability invocation dispatch (excluding adapter): <5μs P99
  - `memory/read` (cached): <50μs P99 / `memory/read` (uncached, SQLite): <5ms P99
  - Hot-swap (rust-inproc): <50ms P99 / Hot-swap (subprocess): <500ms P99
  - Telemetry broadcast (one event, 10 subscribers): <1μs
- **Epistemic halt empirical validation by v1.0:** Per-Spirit halt-recall and halt-precision numbers published on a public benchmark for every reference Spirit class. Acceptable thresholds: halt-recall ≥0.85 (the Spirit halts on ≥85% of cases where it should), halt-precision ≥0.80 (≥80% of halts are warranted, false-halt rate ≤20%). Without these numbers, the "epistemic halt as Layer-1 capability" claim is downgraded from differentiator to mechanism-only.
- **Formal methods for invariants I5 (memory scope enforcement), I6 (hot-swap token preservation), I9 (kernel statelessness)** evaluated by v0.5; TLA+ or Alloy specs landed by v2.0 if property tests prove insufficient. Pragmatic position: property tests now via `proptest`, formal methods only when an invariant violation ships to a real user — `cargo test` carries 90% of the load at 10% of the cost.
- **OSS supply-chain hygiene from day one:** Apache 2.0 + MIT dual-license; SBOM published per release; SLSA-level attestations; reproducible builds (`cargo build --locked`, no nightly, no `unsafe` in `maos-kernel` core); signed Spirit-registry artifacts (Ed25519 publisher keypairs); `cargo deny check` in CI gating dependency drift.
- **Three-protocol coherence empirically demonstrated:** ACP, MCP, A2A all integrated via the kernel's I/O subsystem; ≥1 MAOS Host successfully launched by Zed (ACP), pulling tools from a public MCP server (MCP), and peering with another Host (A2A) — all in one session — by v1.0.

### Measurable Outcomes

The minimum bar across categories at each release:

| Phase | User signal | Adoption signal | Technical signal |
|---|---|---|---|
| v0.1 | 6 ACs pass | Public repo + first commit + CI green | Property tests for I1, I2, I10 |
| v0.5 | All six default Spirits run on a laptop | First "Spirit Jam" candidates identified | Property tests cover I1–I10; T2/T3 sandbox empirically isolating |
| v1.0 | Journey 10 reproducible by 8-person team | ≥1 non-Lunarpulse Spirit in registry; ≥3 external Spirits by month 6 post-release | All 10 invariants empirically verified; halt-recall ≥0.85 published per Spirit; performance budgets enforced as CI gates |
| v1.5 | Journey 11 reproducible (Mira-Nash 90-min loop) | ≥10 external Spirits; ≥3 protocol citations | Loom-lite operational; post-deploy IAC topic working |
| v2.0 | Journey 12 reproducible at 3-region pilot | ≥20 external Spirits; ≥1 cohort project interoperating; "Spirit Jam" annual event | All three Spirit forms operational; signed registry; PDP integration |

## Product Scope

### MVP — Minimum Viable Product (v0.1)

**Validation milestone:** the Architect reference Spirit drives a real coding task on a local repository end-to-end with approval prompts.

**In scope:**
- Kernel skeleton: scheduler + memory manager + capability registry + IAC bus + telemetry stream (architecture §4.1–§4.7); Security Manager and I/O Subsystem stubbed minimally.
- Domain crate (`maos-domain`): pure types + invariants I1–I10 declared; property tests for I1, I2, I10.
- Spirit ABI v0.1 (`maos-spirit-abi`): trait + wire-protocol shapes.
- Spirit SDK v0.1 (`maos-spirit-sdk`): trait export, harness library, declare_spirit! macro.
- One reference Spirit: **Architect** (in-process Rust, `rust-inproc` form only — subprocess and WASM deferred).
- One LLM provider adapter: Anthropic only (`anthropic` feature in `maos-providers`).
- T0/T1 sandbox only (no real OS-native sandbox); T2/T3 deferred to v0.5; T4 WASM-tool deferred to v1.0.
- SQLite persistence (`maos-persistence` with `sqlite` feature); Postgres deferred to v1.5.
- OS keychain for secrets (`maos-secrets` with `keyring` feature).
- HTTP control plane only (Unix socket deferred to v0.5).
- Basic MCP client (`maos-mcp`) for tool-server smoke tests.
- `maosctl` CLI: load / invoke / unload commands.
- `maos` binary as composition root.
- Six AC-style acceptance criteria:
  - **AC-V01-1:** `cargo run -- run examples/two-spirit-handshake.toml` exits 0.
  - **AC-V01-2:** One Spirit sends an Envelope to another via the kernel; both Witnesses appear in `./witnesses.jsonl` in causal order.
  - **AC-V01-3:** A Spirit attempting an action outside its declared Capability set is denied; denial Witness emitted; kernel does not panic.
  - **AC-V01-4:** `cargo test -p maos-kernel` ≥80% line coverage on `scheduler.rs` and `capability.rs`.
  - **AC-V01-5:** Reproducible build: `cargo build --locked` on Rust stable, no nightly, no `unsafe` in `maos-kernel` core.
  - **AC-V01-6:** One end-to-end test green in CI in <30s.

**Out of scope for v0.1:** subprocess Spirit form (v1.0), WASM Spirit form (v2.0), A2A peer mesh (v1.0), ACP server (v1.0), all reference Spirits beyond Architect (v0.5), Approval Manager prompt UX (v0.5), Transparency Log persistence (v0.5), Loom (v1.5), Enterprise Spirit + PDP (v2.0), Spirit registry (v2.0).

**v0.1 crate scope question (carry-forward from Step 2c):** four crates (lean position: `maos-domain`, `maos-kernel`, `maos-spirit`, `maos-bin`) versus 14 crates (kernel implementation guide's full topology). **Decision deferred to Step 8 (Scoping).**

### Growth Features (Post-MVP — v0.5 and v1.0)

**v0.5 (Realistic single-user Host):**
- Five additional reference Spirits: Butler, Researcher, Observer, Diagnostic Engineer (skeleton), Enterprise (stub).
- T2 (container) and T3 (OS-native: Landlock+seccomp / Seatbelt / Win restricted-token) sandbox tiers.
- Approval Manager prompt UX surfaces (TUI, control-plane HTTP).
- Transparency Log persistence and `maosctl audit` query CLI.
- Encrypted-file secrets backend (`maos-secrets` with `encrypted-file` feature).
- Unix-socket control plane.
- Spirit dev experience: `cargo generate maos-spirit` template; first three "bait" reference Spirits ready for adjacent OSS communities (Aider users, Continue.dev users, Neovim AI plugin authors).

**v1.0 (Team-ready, third-party Spirit ecosystem opens):**
- **Subprocess Spirit form** — first third-party-shippable form. JSON-RPC over stdio; stable Spirit Wire Protocol v1.0.
- A2A peer mesh: mTLS + TOFU + per-frame consent gates; role queries; `a2a.json` discovery.
- ACP server: editor-bridged Spirit invocation (Zed, VS Code with ACP plugin).
- T4 WASM tool sandbox: capability-isolated third-party MCP tools running under Wasmtime + WIT.
- Kernel-rendered notification surface across TUI / editor / push.
- Six reference Spirits in production-ready form (subprocess form for those that benefit).
- **Spirit registry v1.0:** MCP-Streamable-HTTP server endpoints (`registry.search`, `registry.manifest`, `registry.artifact`, `registry.publish`, `registry.deprecate`); Ed25519 signing; four trust tiers (`local`, `org-internal`, `public-untrusted`, `public-vetted`); strictest-of-(manifest, tier) sandbox/posture floor.
- Halt-recall and halt-precision benchmarks published per reference Spirit.
- Performance budgets enforced as CI gates.
- All 10 kernel invariants empirically verified.
- **Ecosystem milestone:** first "Spirit Jam" event held at v0.3; ≥5 community-authored Spirits in registry by v0.5; ≥1 non-Lunarpulse Spirit by v1.0; first cohort project interoperating cleanly with MAOS via ACP/MCP/A2A.

### Vision (v1.5 and v2.0)

**v1.5 (Diagnostic-architect pair, Loom-lite):**
- Diagnostic Engineer Spirit class with full asymmetric capability gates (read-only on production runtime knobs; bash-exec whitelist for containment actions; cross-environment telemetry queries to Architect-class Spirits).
- Per-tag epistemic policy operational — `diagnosis.root_cause` halts at `confidence_below = 0.6` or evidence conflict; `diagnosis.observation` is `verbalize_only`; `containment.action` halts at `confidence_below = 0.5`.
- Post-deploy feedback IAC topic; Architect-class Spirits subscribe to Diagnostic-class post-deploy validation results.
- Loom-lite: single-instance Postgres-backed pattern library, exposed as MCP-Streamable-HTTP server.
- `maos-persistence` Postgres support.
- **Validation:** Journey 11 (Mira-Nash) reproducible — diagnostic-architect Spirit pair closes prod-incident-to-deployed-fix loop in ≤90 minutes.

**v2.0 (Enterprise & Cortex, WASM Spirit ecosystem):**
- **WASM-component Spirit form** — third-party ecosystem capability-isolated by construction; single portable artifact; WIT contract `maos:spirit@1.0`.
- Spirit registry v2.0: vetting attestations; community-vetting authorities; OSS-style RFC process for Spirit ABI extensions; OCI-compatibility evaluation.
- Enterprise Spirit class with PDP (Policy Decision Point) integration (OPA / Cedar / Vault-style); SSO/OIDC identity assertions; encrypted-at-rest memory with org KMS; SIEM telemetry export.
- Multi-instance Loom with cross-region replication; consensus on cross-incident pattern propagation.
- Sentinel-validated canary auto-rollback; pre-deployment scanning against pattern library.
- **Validation:** Journey 12 (Cortex) reproducible at small scale — 3-region pilot deployment with ≥10 agents minimum; published case study from a federated research consortium, OSS project's own infrastructure (Debian / Wikimedia / Apache Foundation), or university public-good consortium (target consortium named by v0.3 per Step 2c carry-forward).
- **Ecosystem maturity:** ≥20 external Spirits in registry; ≥3 protocol citations from independent agent projects; ≥1 cohort project formally citing MAOS as substrate or interop reference.

## User Journeys

The substrate exposes three cognitive modes in increasing complexity: **anticipatory** (one Spirit watches and proactively notifies), **exploratory** (one Spirit fans out and converges on findings), and **compositional** (multiple Spirits coordinate under an Orchestrator with full distillation, audit-chain, and consent enforcement). The journey set is sequenced to teach those modes progressively, with each journey's capabilities a strict superset of the prior. **The wedge demo that announces the substrate is J1 Founder's Loop at v0.8 — readers who want the visceral demonstration first should jump there.** J-Butler at v0.3 and J-Researcher at v0.5 are the single-Spirit proof points that earn the founder-loop's compositional ambition; v0.1 ships the foundational kernel + placeholder Spirit only.

**Eight journeys, ordered by ship phase:** J0 Evaluator (cross-cutting; applies from v0.1 minimum-viable install onward); J-Butler (v0.3 anchor — anticipatory single-Spirit); J-Researcher (v0.5 anchor — exploratory single-Spirit); J1 Founder's Loop (v0.8 wedge demo — compositional multi-Spirit, the demo that announces the substrate); J3 Marcus Team Nexus (v1.0 — peer mesh); J6 Diego Spirit-author (v1.0 — third-party builder); J4 Mira-Nash 2 AM (v1.5 — diagnostic-architect pair); Reza single-org cross-team Cortex (v2.0/2.5).

Long-form novelistic versions live in `maos-user-journeys.md` (sister doc); the PRD carries anchor scenes plus capability codas. The journeys honor the carry-forward signals from Step 2c — the wedge pain commitment, the Tier 3 reframe to OSS / single-org-Cortex, the audit-vs-legibility distinction, the eight kernel guarantees enumeration, and the halt-recall/halt-precision benchmark routing — and they exhibit the architecture decisions reached during this PRD workflow: ADR-012 typed-intent consent, ADR-013 `log.recall`, ADR-014 distillation audit-chain (I11), ADR-015 decision-context recording (I12), ADR-016 token-budget, ADR-017 hot-swap wire format, ADR-018 intent provenance (I13), ADR-019 halt continuity (I14), ADR-020 hot-swap migration policy, ADR-021 CliWrapperSpirit output-shape, and the §9.5 distillation pattern.

### Journey B — v0.3 anchor: Sandra's Butler reads the third 6 PM and reclaims dinner

**Persona.** Sandra, designer at a 30-person SaaS company. Figma + Linear + Slack + Google Calendar. Time-zoned across her team (US/EU mix). Recently switched to remote-first work. Has a recurring 7 PM Tuesday-Thursday dinner with friends she keeps missing. Her partner is starting to take it personally.

**Opening scene.** Tuesday, 5:48 PM. Sandra is mid-flow on a Profile Page wireframe. She's been heads-down since 9 AM. Last Tuesday she also worked late. Last Thursday she also worked late. She's missed dinner three times in two weeks. She has not yet noticed the pattern. The Butler Spirit has.

**Inciting moment.** She installed the Butler Spirit two weeks ago and gave it narrow capability scope: read-only on Google Calendar, Slack status, and Figma activity. Posture: `assistive` (notify, but don't act unsupervised). She left it alone.

At 5:48 PM the Butler's `on_idle` lifecycle hook fires (the kernel calls it whenever the Spirit has no pending IAC frames and the user's activity stream shows >12 minutes since last meaningful interaction). The Butler runs its anticipatory reasoning loop: Active-Inference-style belief update over the user's current goal-state, then candidate-action ranking by **expected free energy** (design report ¶158-160) — a sum of pragmatic value (probability of achieving the user's likely goals) and epistemic value (information gained about uncertain beliefs). Beliefs:
- Calendar: 7 PM dinner, recurring T/Th. Last 4 occurrences: 2 attended late, 2 missed entirely. Pattern detected.
- Slack: status set to "Heads down" since 1 PM. No reply to her partner's 4:15 PM message ("Are we still on for dinner?").
- Figma: continuous active session for 4h 40m. Saved-state cadence consistent with deep-work mode.
- Pattern model: predicted disengage time at current pace ≈ 7:28 PM. Probability the dinner gets missed: 0.78. Confidence: 0.85.

The Butler computes a calendar-conflict-confidence scalar (its own definition — predicted-miss-probability × pattern-strength) and writes it via `working_memory.set_scalar("user.calendar_conflict.confidence", 0.85, derived_from=[calendar_obs, slack_obs, figma_obs])`. Per the Butler's `[epistemic_policy]` rule: `tag = user.calendar_conflict.confidence, on_value_above = 0.8 → action = verbalize_with_options` — the kernel reads the scalar, compares to threshold, fires the action. The kernel does NOT compute confidence; it only compares the Spirit-supplied scalar to the threshold. Halts are reserved for tags like `claim.security_vulnerability`. The Butler surfaces a single notification through the kernel-rendered telemetry surface:

> 🤖 **Butler:**
> Sandra — pattern noticed: you've worked past 7 PM the last 3 Tuesdays and missed dinner with friends. You're in deep-work mode; your typical disengage time today is ~7:28 PM. I see your partner's message at 4:15 unanswered. Three options:
> &nbsp;&nbsp;(a) Snooze the wireframe for 75 min — I'll create a Linear note "Resume: Profile Page wireframe at 7:00"
> &nbsp;&nbsp;(b) Reschedule the 7 PM dinner now (drafts a Slack message I can send with one tap)
> &nbsp;&nbsp;(c) Keep working — I'll re-prompt at 6:30 PM
> *Or: ignore me. I'll learn the pattern from your response.*

**Climax.** Sandra reads the notification at 5:51 PM. She picks (a). The Butler's `on_idle` follow-up writes a Linear note via MCP, sets a calendar reminder for 6:55 PM, archives the suggestion-acceptance signal to its `notification_acceptance_log` (private memory, episodic tier — feeds future POMDP predictions). At 6:50 PM Butler's reminder fires. Sandra closes Figma at 6:53 PM. Arrives at dinner at 7:08 PM. Her partner notices.

**Resolution.** Three weeks later the Butler observes that its **posterior uncertainty** over Sandra's preferred prompt-sensitivity has grown beyond threshold (she's been ignoring early-stage prep prompts in shallow-work mode but accepting them in deep-work mode — the underlying preference is bimodal but the Butler's policy is still treating it as unimodal). The Butler computes its belief variance using its own definition (Shannon entropy over the work-mode-conditioned acceptance distribution) and writes via `working_memory.set_scalar("self.belief_variance", 0.78, derived_from=[14-day notification log refs])`. Per its `[epistemic_policy]`: `tag = self.belief_variance, on_value_above = 0.7 → action = halt` — the kernel compares the Spirit-supplied scalar (0.78) to the threshold (0.7) and fires the halt. **The kernel does not compute variance, entropy, or any Spirit-specific cognitive measure (per ADR-022's load-bearing non-responsibility list); the Butler computes its own preferred uncertainty proxy and the kernel does universal arithmetic comparison only.** Per design report ¶162: halt-on-belief-variance is the upstream epistemic signal — the Spirit halts *before* outcomes degrade, not after; outcome metrics like notification-acceptance-rate stay as derived diagnostics that confirm the variance signal but do not themselves trigger halt. It halts:

> 🟡 **Butler halted on `self.belief_variance`**
> My posterior over your preferred sensitivity is bimodal: I suspect deep-work-Sandra wants my prompts and shallow-work-Sandra does not, but my current policy doesn't condition on work-mode. Acceptance-rate diagnostic confirms (0.42 in shallow-work, 0.81 in deep-work over last 14 days). Two options: (a) split the policy by work-mode-context and lower sensitivity for shallow-work mode; (b) keep the unimodal policy and accept the noise. Resolve with `provided_context: <preference>` or `accepted_halt`.

Sandra confirms (a). The kernel updates the Butler's `[epistemic_policy]` per the `provided_context` resolution; the change is journaled (I10) with full reasoning chain. The Butler is self-tuning under human supervision — *upstream of* outcome degradation, not in response to it.

**Capabilities revealed (J-Butler):**

- **Single-Spirit operation** — no IAC bus, no A2A, no multi-Spirit coordination. Butler runs alone.
- **`on_idle` lifecycle hook** as the substrate for anticipatory reasoning (Active Inference / POMDP shape per design report ¶154–¶168).
- **Telemetry Stream** with narrow per-Spirit subscription (calendar, Slack, Figma — no other Spirit's activity).
- **`[epistemic_policy]` per-tag rules** with `verbalize_with_options` / `flag` / `halt` actions and confidence + impact thresholds. Default action is `verbalize_only`; halt is the rare alarm, not the doorbell.
- **MCP tool integrations**: Google Calendar (read), Slack (read + draft-but-don't-send), Linear (write — gated), Figma (read).
- **Output_shape predicate**: notifications carry structured `{pattern, confidence, evidence, options[]}` payload — kernel rejects emit without these fields.
- **Posture-shift command** — Sandra can say "Butler, be more cautious for the next hour"; kernel logs the shift; subsequent capability requests prompt that wouldn't have.
- **Self-tuning via `epistemic.halt`** when notification-acceptance rate drops below threshold — the substrate makes the Spirit's own calibration auditable.
- **Transparency Log** captures every notification, every user response, every `[epistemic_policy]` update.
- **Eval metrics**: notification precision (% acted on; v0.3 floor ≥ 0.85), notification recall (% of relevant moments caught; v0.3 floor ≥ 0.7), user-correction rate, time-to-action savings.
- **No distillation pattern needed** — Butler's per-event reasoning fits in working memory; episodic memory accumulates anonymized acceptance signals for POMDP refinement.

**Ship phase:** **v0.3.** This journey is reproducible at v0.3 with: kernel skeleton + single-Spirit subprocess form + `on_idle` hook + Telemetry Stream + `[epistemic_policy]` enforcement + four MCP integrations + Transparency Log. **No multi-Spirit IAC bus required. No A2A required. No distillation pattern required.** Butler is the proof point for the substrate's anticipatory-reasoning surface.

### Journey R — v0.5 anchor: Hannah surveys the LLM-as-judge literature in 2 hours

**Persona.** Hannah, ML researcher at a 50-person AI safety lab. Her job: survey emerging research, propose hypotheses, hand off to senior researchers. Reads ~30 papers/week. Currently overwhelmed — the LLM-evaluation field is publishing 200+ relevant arXiv submissions per six-month window.

**Opening scene.** Monday 9:00 AM. Her director: *"How is the field handling LLM-as-judge bias? I want to know by Wednesday."* Hannah has 2 hours of focused time before her 11 AM standup, then meetings until 4 PM. The deliverable: structured findings, a confidence map, and a recommendation for which threads warrant a senior researcher's deeper investigation.

**Inciting moment.** She invokes her Researcher Spirit:

> `@researcher survey LLM-as-judge bias methodology, last 12 months. Focus: detection, mitigation, positional bias. Output shape: findings + open questions + confidence map. Time budget: 90 minutes.`

**Rising action.** The Researcher Spirit (loaded with `survey-mode` posture per design report ¶87–¶89) ingests the prompt. Per its manifest's `[capabilities.required]`: `web.search`, `arxiv.search`, `github.search`, `citation_graph.traverse`. Per `[capabilities.parallelism] = 8`: up to 8 concurrent tool dispatches. Cognitive style ("be exploratory, divergent, broad") lives in the system prompt; cognitive capacity (parallelism, recall depth) lives in the manifest.

For 73 minutes the Researcher fans out. Adaptive-chunk-ratio summarization (openclaw pattern, design report ¶362) keeps each paper's digest under 4K tokens. Citation-graph traversal identifies four tight clusters of related work. The Spirit reads abstracts for 40 papers, full intros for 18, full methods for 8.

At minute 73 the Researcher hits an `epistemic.halt`:

> 🟡 **Researcher halted on `claim.methodology_strength`**
> Two papers (Chen et al. 2025-Q3, Tanaka 2025-Q4) report contradictory findings on positional bias in pairwise judgment. Both have strong methodology by my scoring rubric (≥ 0.85). I lack grounds to rank them. Three resolutions:
> &nbsp;&nbsp;(a) Surface both in findings + mark contradiction as Open Question (recommended)
> &nbsp;&nbsp;(b) Apply tiebreak: citation count weight
> &nbsp;&nbsp;(c) Skip both — only cite consensus papers

Hannah picks (a). The halt resolution + reasoning is journaled (I10). The Spirit resumes.

**Climax.** At 10:38 AM the Researcher delivers a structured output that the kernel's Capability Registry validated against the manifest's `[output_shape]` predicate (rejection on missing fields):

- **Findings:** 14 ranked findings, each with citations and per-finding confidence scores
- **Open Questions:** 6 open questions; the contradictory-finding pair flagged with both citations side-by-side
- **Confidence Map:** color-coded heat map of finding-confidence × evidence-density. Two findings sit at low-confidence/high-impact — risky-to-cite-but-might-be-the-most-important-thing.
- **Bibliography:** 38 papers with full citations + summary one-liners + arxiv URLs

Hannah scans the output in 12 minutes. The Open Questions section is exactly the kind of "we don't know yet" pointers her director values most. She forwards the pair to a senior researcher as a deeper-investigation recommendation.

**Resolution.** Hannah delivers her survey by Wednesday 9 AM. The senior researcher takes the Chen-vs-Tanaka contradictory-finding investigation; six weeks later the lab publishes an internal position paper grounded in the Researcher Spirit's first-pass survey. The Researcher's `confidence_map` becomes a recurring deliverable shape — the lab's "research dashboard" feeds it into a Notion view.

**Capabilities revealed (J-Researcher):**

- **Single-Spirit operation** — no multi-Spirit coordination needed for survey-mode work.
- **Broad MCP capabilities**: web search, arXiv search, GitHub search, citation-graph traversal.
- **High parallelism in tool dispatch** (manifest-declared `[capabilities.parallelism]`; v0.5 cap of 8 concurrent).
- **Posture: `survey-mode`** (exploratory, reactive, divergent) with `hypothesize-mode` (generative; **ILP + LLM hybrid for novel-hypothesis generation** per design report ¶191 — ILP's structured rule discovery joined with LLM's pattern completion, output submitted to a Critic Spirit for refinement) declared in the manifest's posture-set but gated until v1.0. v0.5 ships survey-mode-only operation; the hypothesize-mode declaration signals to a sophisticated reader that the substrate's posture surface is genuinely heterogeneous and the ILP component is the architecturally distinctive claim.
- **Output_shape predicate**: "findings + Open Questions + Confidence Map + Bibliography" close — kernel rejects emit without all four.
- **Adaptive-chunk-ratio summarization** (openclaw pattern; default for Researcher and Architect classes).
- **`[epistemic_policy]`** Spirit-detection-triggered halts: the Researcher computes its own contradiction-detection score, low-confidence-high-impact-product, and methodology-strength-tie indicator (using its own definitions); writes each as a tagged scalar; kernel fires halt when scalar crosses Spirit-author-declared threshold via universal-arithmetic comparison. **The kernel does not detect contradictions or compute confidence; the Researcher does, and the kernel only compares the Spirit-supplied scalar.**
- **Time-budget enforcement** via manifest `[budget].time_cap`; soft warning at 80%; the kernel emits `BudgetWarning` IAC frame to the Spirit's mailbox.
- **Transparency Log + log.recall**: full reasoning chain auditable. Hannah's director can replay any cited finding back to the Spirit's source-paper retrieval call.
- **Distillation pattern** for very-large-corpus surveys (200+ candidate papers compressed to 38-paper bibliography) — first opt-in production use of §9.5 in a single-Spirit context. Five-metric gate applies (digest-recall ≥ 0.90, faithfulness ≥ 0.98, hedge-preservation ≥ 0.95, traceability = 100%, secret-leakage = 0%).
- **Eval metrics**: synthesis accuracy, citation correctness (≥ 95% reachable URLs), novelty of hypotheses (when in hypothesize-mode; LLM-as-judge with rubric), open-question quality (rubric-judged).

**Ship phase:** **v0.5.** This journey is reproducible at v0.5 with: kernel skeleton + Spirit subprocess form + broad MCP capability + parallelism + output_shape predicate + `[epistemic_policy]` + distillation pattern's first opt-in deployment. **No multi-Spirit IAC bus required. No A2A required.** Researcher proves the substrate's exploratory-reasoning surface and the distillation pattern in a bounded single-Spirit context before the founder loop composes them with multi-Spirit coordination at v0.8.

### Journey 1 — v0.8 wedge demo: The Founder's Loop (Lunarpulse runs Epic 7 from his daughter's bedtime to school drop-off)

**Persona.** Lunarpulse, founder. Heavy user of AI coding agents — BMAD framework for planning; Claude Code, Gemini CLI, opencode, and Kimi CLI rotating through implementation; `bmad-party-mode` when long-term direction needs cross-checking. Two kids, a marriage, a startup, and a workflow that ostensibly automates coding but in practice has chained him to the laptop because every `bmad-create-story` → `bmad-dev-story` → `bmad-code-review` cycle requires his approval, his next-prompt, his clarification, his "yes, fix that test." Each "wait" is 4–30 minutes. Each "approve" is one keystroke. The asymmetry is grotesque.

**Opening scene.** Tuesday, 4:17 PM. Epic 7 has eight stories. He's two stories in. He has not had an unbroken hour of thought today. His daughter walked in at 3:45 wanting to show him a drawing; he held up a finger because Claude Code was waiting on his approval. He felt the asymmetry of that finger. He still feels it.

**Inciting moment.** He spawns the **Orchestrator Spirit** — a `claude` process loaded with `orchestrator-bmad` and `maos-bridge` skills, posture `autonomous-with-halt`, halt policy preferring recall over precision. Capability scope: `fs.rw` on the project root, `fs.read` on skill search paths (`~/.maos/skills/`, `_bmad/skills/`), `mcp.tools.invoke` on tool servers, `iac.send` for delegating to Worker Spirits, `provider.stream` on his configured LLM, `log:recall:self` for raw-payload retrieval. From his terminal:

> `@orchestrator run epic-7. workers: developer-local, developer-remote (laptop in office); reviewer-local. halt-recall over halt-precision. wake me when in doubt.`

**Rising action.** The Orchestrator reads `_bmad-output/planning/epic-07/`, builds the dependency DAG, and emits its first delegation — a natural-language `task.assign` IAC frame routed by the kernel-internal IAC bus to a **local Developer Spirit** (another `claude` process with `developer` + `maos-bridge` skills loaded):

> `@developer-local task.assign skill=bmad-dev-story target=stories/7-1.md posture="use locally-installed claude-code; halt by writing HALT_REQUEST.md if any AC is ambiguous; existing project conventions"`

The Developer Spirit's behavior loads `bmad-dev-story` from `_bmad/skills/` into its context, reads the story doc, decides to use Bash to invoke `claude -p "..."` (Pattern A), watches for completion, emits progress frames. In parallel, Story 7-3 goes to a **remote Developer Spirit** on Lunarpulse's office laptop — the kernel's A2A adapter ferries the same-shape `task.assign` frame across mTLS, validates the ADR-012 typed-intent consent envelope (`intent: task.assign / development-task` is in the remote Host's allowlist), and the remote Spirit (an opencode session, Gemini provider) loads `bmad-dev-story` from its own filesystem mirror and executes. Different host, different CLI, different provider, same protocol.

**Distillation in action.** When the local Developer reports `task.complete` for Story 7-1, the result payload is large — full diff (3,200 tokens), test output (800 tokens), reasoning trace (2,400 tokens). The kernel writes the full payload to the Transparency Log (I2). The Orchestrator's bridge logic invokes its distillation step: an LLM call summarizing the payload into ~150 tokens — *"Story 7-1: 7 file edits in src/auth/jwt.rs and 4 callers in src/middleware/. 23/23 tests pass. Cargo clippy clean. No new dependencies. Implementation aligns with ADR-007 (PASETO-over-JWT). Ready for review."* The digest is persisted to episodic memory (`fs.write` on private namespace) tagged `kind: digest` with `source_log_ref: [task-complete-frame-id]` and `distillation_depth: 1` — kernel validates per I11 and accepts. The digest joins the Orchestrator's active LLM context. Raw payload sits in the log, recallable via `log.recall` if a future decision needs it.

**Halt with non-blocking continuation.** At 6:23 PM the Orchestrator hits Story 7-5 — acceptance criterion reads "system handles concurrent users gracefully," undefined. Orchestrator halts (`tag: story.acceptance_criterion.ambiguous`). The halt frame is itself a `decision.*` type, so the kernel attaches `working_memory_digest_refs` per I12 — the auditor can later prove which digests the Orchestrator was reasoning over when it halted. Lunarpulse, mid-bedtime-routine, sees the halt notification on his phone:

> 🟡 **orchestrator-bmad halted on Story 7-5**
> AC: "system handles concurrent users gracefully" — undefined. Three candidate operationalizations:
> &nbsp;&nbsp;(a) Load test passes at ≤100 RPS
> &nbsp;&nbsp;(b) Queue-and-degrade above threshold with user feedback
> &nbsp;&nbsp;(c) Hard cap at 50 concurrent; return 429 above
> Pick (a/b/c) or write your own. **I have 3 non-dependent stories I can proceed on while you decide.**

He picks (b). The Orchestrator continues with Stories 7-3, 7-6, 7-7 in parallel via three Developer Spirits while the AC clarification is processed.

**User-input queuing.** Around 5:50 PM Lunarpulse had typed: `also remind the reviewer to check that we're not regressing the OAuth flow in PR-1213`. The Orchestrator's bridge enqueued this without interrupting in-flight Workers. At 6:08 PM, when Worker-1's `task.complete` landed and the Orchestrator was at a decision point, it dequeued the user input, folded it into the next Reviewer dispatch (`@reviewer-local /bmad-code-review review 7-1 — also verify OAuth flow in PR-1213 is not regressed`), and continued.

**Resolution.** He closes his laptop at 8:40 PM. The Orchestrator continues. He goes to a 9 PM dinner with his wife — first one in three weeks. He wakes at 6 AM, opens the morning digest:

> **Epic 7 — overnight summary**
> 7 of 8 stories merged. Story 7-5 paused at second halt (your guidance applied; one follow-up question pending). Retrospective draft attached. 22 IAC frames over 14 hours. 4 halts (3 resolved, 1 pending). 0 invariant violations. 0 capability scope breaches.

He reads the retro on his phone, approves the merge train, takes his daughter to school, and answers the follow-up clarification on the train. Story 7-5 closes by 9:30 AM. He didn't sit at the laptop for it.

**The substrate isn't a coding-agent multiplier. The substrate is the founder's evening back.**

**Capabilities revealed (J1):**

- **Reference Spirit set v0.8+:** Orchestrator + Developer-Worker + Reviewer-Worker (3 skill packages, no per-CLI Rust crates). Worker Spirits are agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded. ADR-014 (use existing protocols, no new IAC MCP server). Butler ships earlier at v0.3, Researcher + Observer at v0.5.
- **Two-level `task.assign`** (ADR-013 — Human-Spirit interaction surfaces; ADR-012 typed-intent consent envelopes for cross-host A2A). Human → Orchestrator at epic granularity; Orchestrator → Worker at story granularity. Same primitive, different topology.
- **Skills as named-and-loaded-on-demand.** Orchestrator names the skill in `task.assign`; Worker resolves and loads from its skill search path; kernel never hosts skills. Skills proliferate freely; Spirits stay small in number. Skills ≠ Spirits.
- **Multi-protocol Spirit communication.** Same-host: kernel-internal IAC bus. Cross-host: A2A peer mesh with mTLS + ADR-012 typed-intent consent. Editor-hosted: ACP. Tool invocations: MCP. Orchestrator's delegation API is uniform across topologies.
- **Distillation pattern operational** (architecture §9.5). Raw → Transparency Log → Spirit-side LLM distillation → digest in working memory + episodic. `log.recall` (ADR-013) for raw-payload retrieval on demand.
- **I11 audit-chain enforcement** (ADR-014). Every persisted digest carries `source_log_ref` and `distillation_depth`; kernel rejects malformed writes.
- **I12 decision-context recording** (ADR-015). Every `decision.*`-typed frame the Orchestrator emits carries `working_memory_digest_refs` so post-hoc audit can prove which summaries the agent reasoned over.
- **User-input queuing** in the Orchestrator's persona logic. Human frames buffered; processed at safe sequence points.
- **Per-tag epistemic policy** anchored to Orchestrator domain tags (`story.acceptance_criterion.ambiguous`, `test.persistent_failure`, `scope.expansion_detected`, `architecture.novel_decision_required`, `security.finding`). Halt-recall over halt-precision is user-configurable (the founder's choice).
- **Mobile-friendly halt notifications** with structured frames (AC text + candidate resolutions + non-blocking story queue).
- **Founder's morning digest** — kernel-rendered overnight summary.
- **Capability scoping at story granularity** — Orchestrator computes per-story scope from planning artifacts; Worker Spirits get fs.rw bounded to their story's files, not project-wide.

**Wedge committed.** This journey commits **W1″ — Orchestrator-led parallel multi-CLI execution with kernel-mediated audit-chain and decision-context recording** as the Tier 1 wedge. The original W1 (multi-agent orchestration + persistence) is the underlying capability. The 60-second demo answer to "what does MAOS do that Claude Code, Cursor, Aider, and a tmux session cannot?" — *"runs your full epic loop while you eat dinner, halts when it has questions, returns you a completed epic in the morning, and gives you an audit trail every regulator and skeptic can read."*

**Ship phase:** **v0.8.** The founder's-loop wedge demo composes everything the v0.1/v0.3/v0.5 phases built: foundational kernel guarantees (v0.1), single-Spirit anticipatory reasoning (v0.3 Butler), single-Spirit exploratory reasoning + bounded distillation (v0.5 Researcher), and now multi-Spirit Orchestrator+Worker coordination + cross-host A2A + full distillation pattern. v0.8 is the first phase where the substrate's full architectural ambition is observable in one demo. The Butler and Researcher journeys at v0.3 / v0.5 are the proof points that justify the v0.8 ambition; this journey is the wedge demo that announces the substrate.

### Journey 4 — Tier 3 sub-pattern: Elena's 2 AM 90-minute Mira-Nash incident

**Persona.** Elena, VP Engineering at a fintech processing 40,000 transactions per minute across 3 regions and 60+ microservices. She runs two MAOS Spirits in production-adjacent positions: **Mira**, a diagnostic Spirit on a production-edge node (read-only on production runtime; write only for revert/scale/flag-toggle); **Nash**, an architect Spirit in the dev environment (full source repo access; CI/CD orchestration). They are peers across hosts; neither owns the other.

**Opening scene.** 2:13 AM. Elena's phone lights up. PagerDuty alert: `payment-service` Kafka lag rising 340%. Before she's awake, the alert is also a MAOS notification — Mira has been triaging for 9 minutes already.

**Rising action.** Mira's diagnosis at 02:22: deployment of `billing-v3.2.1` at 01:41 UTC introduced `BatchInvoiceProcessor.java` which creates a DB connection per message instead of using the pool. Confidence: 0.82 (above the diagnostic Spirit's halt threshold of 0.6 for `diagnosis.root_cause`). Mira escalates to Nash via cross-host A2A — the kernel's A2A adapter validates the ADR-012 typed-intent envelope: Mira declares `intent: diagnosis-handoff:read-only-evidence`, which Nash's consent policy permits from prod-edge peers; Nash's policy explicitly excludes `remote-write-request` and `code-mutation-directive`. The frame lands. Nash receives a structured diagnosis, not an instruction.

Nash pulls `BatchInvoiceProcessor.java`, confirms instantly, then *expands the search*: pattern detection across the codebase finds two more dormant instances (`BatchRefundProcessor.java`, `DailyReconciliationJob.java`). Nash queries Mira: *"are the dormant ones showing latency increase in 72-hour telemetry?"* Cross-environment query, kernel-mediated, audit-trailed. Mira queries 72 hours of metrics. `DailyReconciliationJob` shows month-end spikes. Time bomb. Recommend: fix all three. Confidence 0.94.

**Distillation in action — at the diagnostic edge.** Mira's incoming telemetry stream is high-volume. Per the §9.5 pattern, raw telemetry frames land in the Transparency Log; Mira's Spirit-side distillation step compresses 72 hours of cross-service metrics into ~200-token decision-relevant digests per service ("payment-service: lag 340% above baseline since 01:41; correlates with `billing-v3.2.1` deploy; connection pool 200/200 saturated; 47/50 threads blocked on `getConnection()`"). Each digest carries `source_log_ref` to the underlying telemetry frames and `distillation_depth: 1`. Mira's working memory holds the digests; raw is recallable via `log.recall` for any downstream decision.

**Climax.** Elena wakes at 2:47 AM to one notification:

> 🔴 **mira + nash: Coordinated response ready**
> Issue: Kafka lag spike (detected 02:13, diagnosed, fixed)
> Root cause: Connection pool bypass in 3 batch processors
> Proposed: Rollback (Mira) + Deploy PR #1247 (Nash, tested, ADR-112 attached)
> Trail: Mira detected → Nash confirmed + found dormant bugs → Mira confirmed via 72hr telemetry → Nash produced PR + ADR + regression test
> [Approve] [Review PR first] [Rollback only]

The notification carries decision-context refs (I12) — Elena can see exactly which Mira digests Nash reasoned over and which Nash digests Mira used to validate the 72-hour telemetry query. She approves from her phone. Mira executes rollback. Nash's PR ships through CI/CD. Total elapsed: 2:13 to 3:43 AM. **90 minutes. Elena touched the system once.**

**Resolution.** Three weeks later, Mira sends Nash an unsolicited architecture note: *"0 new violations of ADR-112 in 23 deployments. Regression test caught 1 attempt at CI. New observation: 14% of batch processors exceed pool timeout during month-end — investigate batch sizing?"* Nash opens an investigation. The cycle continues.

**Capabilities revealed (J4):**

- Asymmetric capability postures via manifest (sre-diagnostician vs principal-architect).
- Cross-Host A2A with ADR-012 typed-intent consent envelopes — Mira's `diagnosis-handoff:read-only-evidence` is allowed; `code-mutation-directive` is blocked. Closes the confused-deputy class of attacks.
- Cross-environment telemetry queries with audit trail.
- Per-tag epistemic policy at both Spirits with Spirit-detection-triggered halts (Mira and Nash each compute their own confidence/conflict scalars; kernel does universal-arithmetic comparison per ADR-022 — kernel does not interpret cognitive concepts).
- Mobile-friendly approval surface.
- ADR auto-drafting and CI-enforced pattern detection.
- Distillation pattern at the production edge — high-volume telemetry compressed to decision-relevant digests; raw recallable via `log.recall`.
- I12 decision-context recording across cross-host A2A — Elena can see which digests at each Spirit drove the coordinated response.

### Journey 3 — Tier 2 normalcy: Marcus's day-30 Tuesday morning standup

**Persona.** Marcus, tech lead / architect on an 8-person agile team at a fintech. The team has run MAOS for 30 days, peer A2A mesh, every member has their own Host with three Spirits. Marcus runs an Architect Spirit, Atlas. Lena (PO) runs a Story-Decomposer; Jun and Aisha (devs) run Coder Spirits; Sami (QA) runs a Test-Designer Spirit; Nina (UX) runs a Wireframe Spirit.

**Opening scene.** 9:47 AM Tuesday. Marcus opens his laptop. Atlas is mid-thought from yesterday — *thinking-with* Jun's Coding Spirit across the room about the auth refactor. A pulsing peripheral indicator in Marcus's IDE shows the cross-Spirit conversation is active. He doesn't read it. He just knows it's there.

**Rising action.** Atlas surfaces three things in Marcus's morning view: (1) overnight work — drafted ADR-047 about the new event-bus pattern, in *proposed* state; (2) what Atlas *almost* did — was going to refactor session middleware, halted because Jun's Spirit was touching adjacent code; (3) one genuine "I don't know enough" — epistemic halt on whether the new rate-limit policy applies to internal services. Atlas is waiting. Not guessing. Waiting.

**Climax.** The standup at 10:00. Marcus doesn't read logs. He reads the team's narrative digest (per §9.5): *"Overnight, 8 agents ran. 47 IAC frames exchanged. 3 agents halted, 0 acted invisibly. 2 cross-agent consultations resolved without escalation. 1 architectural conflict surfaced for review (Lena's S7-04 vs current event-bus pattern, ADR-047)."* 60 seconds. Everyone nods. Standup moves on.

The digest is itself a distillate — produced by a per-Host summarization step over the day's IAC frames. Per I11, the digest carries `source_log_ref` to the underlying frames; per I12, the digest's emit-frame records which agent-level digests fed into the standup summary. An auditor can replay any sentence in the digest back to raw evidence in seconds.

**Resolution.** Day 30. The team has replaced Jira (story tracking → A2A mesh + manifest-declared roles), Confluence (architecture docs → ADR registry as Loom partition), and most Slack pings (consultations now happen agent-to-agent with full context, surfaced to humans only on halts). Each team member feels *amplified*, not surveilled. Nobody was puppeted. Nobody read each other's transparency logs without consent.

**Capabilities revealed (J3):**

- A2A peer mesh with mTLS + TOFU + ADR-012 typed-intent consent gates.
- Role queries (`role: "architect"`) resolved locally per Host.
- Per-Spirit consent policies (auto-share ADR drafts; not internal scratchpads).
- Kernel-rendered narrative digest UX — distillation pattern made visible at the team boundary.
- Transparency Log per-Host (no team-wide surveillance); cross-agent halt-on-conflict.

### Reza — Tier 3 candidate: Single-org cross-team Cortex

**Persona.** Reza, head of platform engineering at a 400-person fintech. Three teams (security, support, data) run their own Spirits independently. They don't normally talk. Reza is the platform lead who gets paged when their Spirits collide on shared resources — and then writes the post-mortem.

**Opening scene.** Friday 4:30 PM. A printed org chart sits on Reza's desk because he's the only person who knows which team owns which data domain. His "platform office hours" on Wednesdays are usually empty until someone needs something. He gets pulled into a Slack thread — the fraud team's Spirit and the support team's Spirit both want to write to the same customer-context store, and neither team knew the other existed. He has 90 minutes before the weekend.

**Rising action.** Reza opens his platform-lead shell, types into a kernel-rendered Orchestrator session:

> `@fraud-spirit @support-spirit propose a shared customer-context schema; consult each other; halt for me at 5 PM with a recommendation.`

Two Spirits owned by two different teams, task-assigned by Reza-with-cross-team-authority, work it out via cross-host A2A using ADR-012 typed-intent consent. The fraud Spirit's `intent: schema-proposal:read-only-evidence` is in the support Spirit's allowlist; payload-shape consent prevents either Spirit from authoring the other's writes. Both Spirits load their team's data-residency patterns from Loom.

**Distillation in action — multi-hop.** The fraud Spirit's proposal cites 14 prior schema decisions across the team's history. Each citation is itself a digest written months ago. Per ADR-014, when the fraud Spirit produces its consolidated proposal-digest, `source_log_ref` flattens transitively to point to the *original raw schema decisions* — not the intermediate digests. Reza, reading the proposal at 4:55 PM, can trace any claim back to raw evidence in one hop.

**Climax.** At 5:00 PM the Orchestrator halts with a unified recommendation: *"Shared schema with namespaced sub-objects per team. Customer-context.fraud / customer-context.support. Read-only across teams; writes scoped to the namespace owner. Migration path: 3 weeks. ADR draft attached."* Reza approves the schema, dispatches a follow-up task to a Coder Spirit to implement the migration, and closes his laptop at 5:08 PM.

**Resolution.** Three weeks later, the migration ships. Reza's Wednesday office hours stay empty for a different reason — the cross-team coordination problem the platform-lead role used to handle is now substrate work. Reza becomes a designer of policies, not a routing table.

**Capabilities revealed (Reza):**

- Cross-team A2A with asymmetric consent envelopes (ADR-012) — payload-shape consent gives fraud and support Spirits a cleaner trust boundary than role-based ACLs alone.
- Multi-hop distillation provenance per ADR-014 — schema-proposal-digests reference original schema decisions, not intermediate digests.
- Loom-tier integration for team-specific pattern libraries (data-residency, schema conventions).
- Kernel-rendered platform-lead Orchestrator surface (one-line task input; structured halt at 5 PM).
- v2.0 Cortex demo target candidate — single-org cross-team is the achievable, design-partner-friendly version of the federated-research-consortium ambition. Final Cortex consortium target locked by v0.3 per Step 2c carry-forward.

### Diego — Cross-cutting: The third-party Spirit author

**Persona.** Diego runs an open-source code-review tool with ~3,000 GitHub stars. He wants to make it agentic but doesn't want to build a substrate. Reads the MAOS announcement post; the phrase *"a third party authors and ships a Spirit independently of the MAOS source tree"* catches his eye.

**Opening scene.** Diego opens `spirit-development-and-sharing.md`. Skims to *"Build your first Spirit in 30 minutes."* Runs `cargo generate maos-spirit`, gets a templated project. Imports his existing static-analysis logic.

**Rising action.** Manifest:
- `class = "code-reviewer-pro"`
- `[capabilities.required]` — `fs.read` on `**/*.rs`, `provider.stream` on Anthropic models, `iac.send` on broadcast.
- `[posture]` — assistive; prompt on writes, silent allow on reads.
- `[output_shape]` — JSON schema for code-review findings (severity, file, line, suggestion).
- `[epistemic_policy]` — halts on `claim.security_vulnerability` with confidence < 0.85; verbalize-only on style suggestions.

Diego runs `spirit-test` on a corpus of 50 known-buggy code examples. His Spirit catches 47 of 50 (recall 0.94). Halts on 3 of those (precision uncertain, but he ships it). Runs `maos-spirit publish --tier=public-untrusted`. His Spirit appears in the public registry signed with his Ed25519 key.

**Climax.** Within a week, 12 MAOS users install Diego's Spirit. Three file issues; one files a PR. The community vetting board reviews Diego's signing identity and capability surface — within 2 months, his Spirit is promoted to `public-vetted` tier via attestation. Diego's GitHub stars double in 4 months.

**Resolution.** Diego writes a blog post: *"Why I deleted 4,000 lines of HTTP/SDK glue code by becoming a MAOS Spirit."* 200+ HN upvotes. Three other agentic-tool authors port their tools to Spirits within 6 months.

**Capabilities revealed (Diego):**

- `cargo generate maos-spirit` template scaffolding.
- `spirit-test` mocking harness for unit-testing Spirit ABI without a kernel.
- Manifest validation (output_shape, epistemic_policy, capability scopes) at publish time.
- Public registry with Ed25519 signing, four trust tiers, vetting attestations.
- Spirit-portable architecture — Diego's behavior code does not import HTTP libraries or LLM SDKs. *Spirit is behavior, not infrastructure.*
- Halt-recall and halt-precision benchmarks publishable per Spirit.

### J0 — Cross-cutting: The evaluator

**Persona.** Anonymous developer, 4 minutes into `cargo install maos`, deciding whether minute 5 happens. Compared MAOS to Claude-Code-plus-bash. Skeptical.

**Opening scene.** Install completes. Terminal prompt waits. They type `maos init` in a scratch directory. Five Worker Spirit slots and one Orchestrator slot are configured by default; pre-installed skills include the BMAD set. They type into the kernel-rendered shell:

> `@hello-spirit say hi and tell me what you can do.`

A `claude` process spawns inside a T2 sandbox, with `developer` + `maos-bridge` skills loaded. The Spirit responds in 4 seconds with a structured introduction: capabilities scoped, posture stated, expected halt-tags listed, link to the local Transparency Log.

**Rising action.** They try a bigger task: `@hello-spirit refactor src/main.rs to be more idiomatic`. The Spirit halts immediately on `task.acceptance_criterion.ambiguous` — *"'more idiomatic' is undefined; please specify the dimensions you care about."* They laugh, type a clarification, the Spirit proceeds. Four file edits, 9 IAC frames, all logged. They `maos audit query` to read their own Transparency Log. The audit trail is queryable from minute 6.

**Climax.** They uninstall: `cargo uninstall maos`. The kernel removes itself cleanly. The user's Transparency Log persists in `~/.maos/logs/` for review (configurable retention). They reinstall the next morning. They tell their tech-lead about it.

**Resolution.** First-time installer who didn't bounce — because the substrate set expectations honestly within 6 minutes (capability scope visible; halt visible; audit visible) and was reversible.

**Capabilities revealed (J0):**

- Time-to-first-Spirit ≤ 5 minutes (pre-installed skills, default Spirit slots).
- Honest capability disclosure on first interaction (the Spirit introduces what it *can* and *cannot* do).
- Halt-on-ambiguity demo from minute 4 — sets the substrate's character before any commitment.
- Audit-from-minute-1 (`maos audit query` works on the local Transparency Log).
- Clean uninstall (kernel is reversible; user's data persists or is removed per their choice).

### Journey requirements summary

Eight journeys collectively reveal the capability areas the kernel and reference Spirits must deliver, sequenced by ship phase:

| Capability area | Journeys | Phase |
|---|---|---|
| Single-Spirit subprocess form + Spirit ABI v0.1 | J0, J-Butler, J-Researcher | v0.1 (placeholder Spirit + foundational primitives) → v0.3 (Butler) → v0.5 (Researcher + Observer) |
| `on_idle` lifecycle hook for anticipatory reasoning | J-Butler | v0.3 |
| Telemetry Stream subscription (narrow per-Spirit) | J-Butler, J-Researcher (broad), Observer | v0.3 (basic narrow subscription) → v0.5 (broader; Observer broadcast subscriber) |
| `[epistemic_policy]` per-tag rules with verbalize/flag/halt taxonomy | J-Butler, J-Researcher, J1, J4, J0, Diego | v0.3 (mechanism with confidence + impact thresholds) → v0.5 (per-Spirit-class policies) → v1.0 (community policies) |
| Output_shape predicate enforcement (kernel rejects malformed emit) | J-Butler, J-Researcher | v0.3 (basic) → v0.5 (full per-class predicates) |
| MCP tool integrations (Calendar / Slack / Linear / Figma / arXiv / GitHub / web-search / citation-graph) | J-Butler, J-Researcher | v0.3 (Butler set: Calendar/Slack/Linear/Figma) → v0.5 (Researcher set: web/arXiv/GitHub/citation-graph) → v1.0 (broader ecosystem) |
| High parallelism in tool dispatch | J-Researcher | v0.5 (≥ 8 concurrent) |
| Adaptive-chunk-ratio summarization (openclaw pattern) | J-Researcher, distillation-shipping Spirits | v0.5 |
| Distillation pattern + I11 audit-chain + I12 decision-context + I13 intent_lineage | J-Researcher (single-Spirit opt-in), J1, J3, J4, Reza | v0.5 (single-Spirit opt-in) → v0.8 (multi-Spirit Orchestrator pattern, full §9.5 deployment) → v1.0 (five-metric gate on all distillation-shipping Spirits) |
| Posture-shift command (runtime supervision knob) | J-Butler, J1, J4 | v0.3 (Butler "be more cautious for the next hour") → v0.5+ (broader) |
| Self-tuning via `epistemic.halt` (Spirit's own calibration auditable) | J-Butler | v0.3 |
| Multi-Spirit Host with shared IAC bus | J1, J3, J4, Reza | v0.8 (multi-Spirit IAC + Orchestrator pattern) → v1.0 (full peer mesh) |
| Two-level `task.assign` typed-intent IAC primitive (ADR-013) | J1, J3, J4, Reza | v0.8 (introduced for Orchestrator+Worker) → v1.0 (full A2A peer mesh) |
| Cross-Host A2A peer mesh + ADR-012 typed-intent consent | J1 (loopback only at v0.8), J3, J4, Reza | v0.8 (loopback-only profile per Winston) → v1.0 (cross-host with mTLS+TOFU) |
| Episodic memory persistence across sessions | J-Butler, J-Researcher (private tier), J1, J3, J4 | v0.3 (private tier) → v1.0 (shared tier) → v1.5 (collective tier via Loom) |
| Transparency Log + Approval Decision Log + `log.recall` | All journeys | v0.1 (basic Transparency Log) → v0.3 (replay) → v0.5 (`log.recall` ABI per ADR-013) → v1.0 (queryable export + sealed export) |
| Sandbox tiers (T0–T4) with capability scoping | All | v0.1 (T0/T1) → v0.3 (T2 narrow) → v0.5 (T2/T3) → v2.0 (T4 WASM) |
| Pluggable provider drivers (Anthropic / OpenAI / local / Bedrock) | J-Researcher, J1, J4 | v0.1 (Anthropic single) → v0.5 (multi-provider via CLI-wrapped agents) → v1.5 (MAOS-mediated provider proxies) → v2.0 (full multi-provider) |
| Mobile-friendly approval surface (phone push) | J1, J4 | v1.0 (HTTP push) → v2.0 (native push) |
| Asymmetric capability postures (sre-diagnostician vs principal-architect vs platform-lead) | J4, Reza | v1.5 |
| Loom collective tier with cross-Host pattern propagation | Reza | v1.5 (Loom-lite) → v2.0 (multi-instance) |
| Public Spirit registry with Ed25519 signing + four trust tiers | Diego, all third-party Spirits | v0.5 (basic) → v1.0 (full registry + vetting attestations) |
| `cargo generate maos-spirit` + `spirit-test` + Spirit dev SDK | Diego, J-Butler/Researcher (Spirit-author paths) | v0.3 (SDK basic) → v0.5 (full) → v1.0 (registry integration) |
| `maos audit query` CLI + cleanup + clean uninstall | J0, J7-equivalent in NFRs | v0.1 (clean uninstall — J0 requirement) → v0.5 (basic queries) → v1.0 (signed export) |

These map directly to Functional Requirements (Step 9) and Non-Functional Requirements (Step 10). The wedge pain commitment (W1″ — Orchestrator-led parallel multi-CLI execution with kernel-mediated audit-chain) is now load-bearing across J1 and Reza, *anchored at v0.8*. Step 10 (NFRs) inherits the five-metric distillation ship-gate from §9.5 (digest-recall ≥ 0.90; faithfulness ≥ 0.98; hedge-preservation ≥ 0.95; traceability = 100%; secret-leakage = 0%).

**The phase-anchoring decision (per founder directive):** Butler ships at v0.3 and Researcher at v0.5 *before* the founder's-loop wedge demo at v0.8. v0.1 is foundational (kernel skeleton + placeholder Spirit + clean install/uninstall), proving the kernel's big-picture readiness without committing to multi-Spirit ambition. v0.3 and v0.5 deliver progressively richer single-Spirit value (Butler's anticipatory reasoning; Researcher's exploratory reasoning + bounded distillation) on top of the foundational kernel — neither requires multi-Spirit IAC, A2A, or the full distillation pattern. v0.8 then composes everything into the founder's-loop wedge demo with multi-CLI Orchestrator+Worker coordination.

Open questions routed forward:
- **Cortex consortium target for v2.0 demo** — Reza-style single-org cross-team is the leading candidate; OSS-on-MAOS (Debian / Wikimedia / Apache Foundation) and federated research consortium are alternatives. **Final lock by v0.3** per Step 2c.
- **Adversarial-Spirit threat model** — Mary's stakeholder list for Step 10 NFR (STRIDE-style attack narratives with kernel defenses traced to FRs).
- **Halt-recall vs halt-precision floors per Spirit class** — uniform across distillation-shipping Spirits per §9.5; per-Spirit floors for non-distillation Spirits (J4 Mira: halt-recall ≥ 0.7; J1 Orchestrator: halt-precision ≥ 0.85). Routes to Step 10.

## Domain-Specific Requirements

MAOS sits in the *agent infrastructure* sub-domain — a substrate layer that inherits concerns from three established domains (scientific computing, developer tooling, enterprise security) and synthesizes a fourth (the trust-mediation needs unique to autonomous-agent runtimes). The requirements below are domain-specific in the sense that a generic developer tool or generic enterprise system would not face them; they fall out of the substrate's claim to be *the trusted runtime under which third-party autonomous agents execute on behalf of humans*.

### Compliance & Regulatory

**Audit retention and integrity (universal across deployment tiers).** Every external call, every IAC frame, every approval decision, and every Spirit lifecycle transition lands in the Transparency Log + Approval Decision Log + lifecycle journal (Invariants I2, I4, I10). Default retention: 90 days private tier, configurable per-deployment; Merkle-root anchoring optional for tamper-evidence in regulated deployments. Logs are queryable via `maos audit query` (J0, J7-equivalent NFR) and exportable to JSONL / SIEM pipelines (v2.0 enterprise).

**Right-to-explanation (EU AI Act adjacent; broader regulatory direction).** Every `decision.*`-typed frame carries `working_memory_digest_refs` per Invariant I12 (ADR-015). Combined with I11 (digest audit-chain to raw frames) and `log.recall` (ADR-013), every autonomous-agent decision can be reconstructed end-to-end: what raw evidence existed, what summary the agent reasoned over, what decision the agent emitted, who approved it. This is the substrate-level mechanism that lets MAOS deployments meet "explainable AI" regulatory expectations without bolting on after-the-fact tooling.

**Reproducibility (scientific-computing inheritance).** Spirit binaries ship with Ed25519 signatures; manifests are content-hashed; eval results are publishable per-Spirit. Deterministic builds for reference Spirits; Cargo.lock pinning; SBOM generation on every release. The substrate's audit claims are reproducible — auditors with the same logs and Spirit packages can replay any decision sequence.

**Data residency (fintech / healthcare-adjacent).** Capability scoping is per-region: a Spirit's manifest can declare `[capabilities.required].region_lock = "EU"` and the kernel refuses delivery of frames whose payloads are sourced outside the locked region. Provider drivers can be locked to specific endpoints (e.g., air-gapped Bedrock-private for the Aisha-CVE case). The kernel itself stores no PII (Invariant I9); residency enforcement happens at sandbox boundary + capability scope.

**Right-to-be-forgotten (GDPR / similar).** Per-Spirit private memory is removable on operator command (`maos forget --spirit <id>`). The Transparency Log is not removable (it's the audit spine), but personally identifying payloads in the log can be redacted via `maos audit redact --frame <id> --reason <legal-hold>`; redactions are themselves logged. Subject-rights requests are operator-tooling concerns; the substrate provides the primitives.

**License and OSS provenance.** MAOS ships under an OSI-approved license (Apache 2.0 working assumption; final lock by v0.3). Reference Spirits ship with permissive licenses. The Spirit registry enforces license declaration in every manifest. Public-tier Spirits without OSI-approved licenses are flagged at install.

**Export control (Mary's gap).** OSS distribution does not exempt MAOS from EAR / ECCN classification. The substrate ships cryptographic modules (Ed25519, mTLS, secret encryption); ECCN classification work is part of the v1.0 release prep. Wassenaar-adjacent concerns when a Spirit-as-cyber-tool gets distributed are Spirit-author + registry-operator responsibility, surfaced via the existing trust-tier flag plus an export-classification field in the manifest.

**Pluggable crypto provider trait (defense / FIPS readiness).** The kernel's cryptographic operations (signing, mTLS, secret encryption) are mediated by a `CryptoProvider` trait with a default implementation (`ring` / `rustls` / equivalent). Alternate implementations can be swapped at composition root for FIPS 140-3-validated module compatibility, NIAP evaluation eligibility, or air-gapped deployments using on-prem HSMs. **v1.0 architectural commitment**: the seam exists; specific FIPS modules are downstream distributor concern.

**Compliance-as-attestation primitive (Mary's load-bearing addition).** A new first-class kernel object: `ComplianceClaim`. Ed25519-signed by an attesting third party, references an *execution-context fingerprint* — the precise tuple of (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity) under which the claim applies. Examples: a HIPAA-attestation issued by an accredited assessor for `code-reviewer-pro v2.1, public-vetted, T2, scope X, on-prem Bedrock-private`; an SOC 2 Type II attestation against a specific reference-Spirit deployment configuration. The kernel verifies `ComplianceClaim` envelopes at admission time and refuses to load Spirits whose runtime context drifts from the attested context (typed error `EComplianceContextDrift`). This makes attestations falsifiable rather than marketing copy. **The attesting third party becomes a named substrate stakeholder**, alongside Spirit author / vetter / operator / auditor. v1.0 first-class object.

**Vetter trust model commitment.** MAOS v1.0 ships with a documented vetter trust model — accreditation procedure, attestation revocation semantics, conflict resolution when two vetters disagree, and trust-graph scope per Host. Specific parameters (revocation TTL, accreditation criteria) are deferred to Step 10 NFRs; the *existence* of the model is a v1.0 ship gate.

**Role-distinct query primitives (Mary's compliance-loop addition).** `maosctl audit query` is one of four complementary surfaces:

| Surface | Stakeholder | Primitive |
|---|---|---|
| `audit query` | Internal auditor / SRE | Frame-by-frame log query with replay (covered by `log.recall`). |
| `audit subject-access` | DPO / data subject | Subject-indexed query — "show me everything about data subject X across all Spirits and Hosts." Indexes on PII tags in IAC frames; respects redaction policy. |
| `audit posture-delta` | CISO / security operations | Posture-drift query — "what capability scopes / sandbox tiers / consent policies have changed across Spirits in the last 30 days, and what was the approval chain." |
| `audit sealed-export` | External regulator / certification body | Cryptographically sealed audit bundle — Ed25519-signed by the operator's audit key, third-party-verifiable; not raw log. Includes Merkle anchoring if enabled. |

These are different primitives, not different views of one log. v1.0 ships all four.

**Regulatory regime applicability (Mary's prioritization).** Two regimes change v1.0 *primitives*: **EU AI Act** (high-risk system classification, foundation model obligations — addressed by I12 + ComplianceClaim + audit-from-minute-1) and **NIS2** (digital infrastructure component the moment one deployment touches an essential-services operator — addressed by audit retention + sealed-export + cross-host A2A consent envelopes). PIPL / LGPD / PIPEDA / PDPA / Colorado AI Act / SB-1047 / state-level US AI regs change *configuration* of these primitives and are satisfiable by enterprise distributions tuning policy. SOC 2 / ISO 27001 / FedRAMP are certification programs handled by enterprise distros, not v1.0 substrate concerns.

### Technical Constraints

**Sandbox tiers (security domain).** T0 (no sandbox; trusted only — local-tier Spirits) → T1 (process isolation; UID separation) → T2 (Landlock+seccomp on Linux, Seatbelt on macOS, WinRT job objects on Windows; the default for public-untrusted Spirits per ADR-009) → T3 (containerized — Docker/Podman) → T4 (WASM component model; v2.0). Strictest-of-(manifest, trust tier) floor enforced (ADR-009).

**Per-Spirit resource isolation (Winston's addition).** Each Spirit runs under a resource cgroup (Linux cgroups v2; equivalent constraint primitives on macOS/Windows) with kernel-enforced caps on CPU, memory, file descriptors, and process count. Sandbox tiers cover the *security* boundary; resource cgroups cover the *resource* boundary. A runaway Spirit gets throttled, not the host. Caps declared in manifest `[resources]` table; defaults per Spirit class.

**Capability mediation (security domain).** Every external call (file op, network, exec, sub-Spirit spawn) goes through the Capability Registry (Invariant I1). Token-scoped, expiry-bounded, posture-bound. The kernel issues no implicit ambient authority; Spirits cannot escape capability scope via wrapped-CLI or skill invocation (sandbox boundary catches it at syscall level). Capability tokens are re-validated at use against current state, not cached past their state-change boundary (TOCTOU correctness as Step 10 NFR).

**Secret handling (security domain).** Secrets pass through to OS keyring (Linux secret-service / macOS Keychain / Windows Credential Manager); no kernel storage (Invariant I9). v2.0 enterprise integration adds Vault / cloud-KMS via PDP. The kernel materializes secrets just-in-time at the capability boundary; in-memory lifetime bounded; never logged. **Pre-write secret-pattern redaction filter at the Transparency Log boundary** — corpus of API-key / capability-token / private-key patterns auto-redacted before frames land in the log; zero secrets in any logged frame, ever. This filter is the substrate's contribution to the §9.5 five-metric distillation gate's `digest-secret-leakage = 0%` requirement.

**Performance envelope (Winston's correction — drop sub-millisecond).** Realistic v0.1 target on a typical Linux box: **p50 < 5ms, p99 < 50ms for routed-and-logged IAC frames; sustained throughput 5–10K frames/sec single-host before the log writer becomes the bottleneck.** Sub-millisecond is achievable only by violating the spirit of I2's "log-before-deliver" (delivering before durable commit). The log writer is the throughput-defining component; a per-Spirit fairness scheduler in front of the log handles fan-in pressure (Reza's 28-Spirit Cortex aggregate steady-state ~56 fps, burst ~560 fps — well within envelope, but FIFO is wrong shape). Absolute performance numbers are Step 10 NFR territory; this section commits the *architectural shape* (synchronous log + capability mediation + scope check + serde, with realistic floors).

**Token-budget accounting (ADR-016, Winston's addition).** Context tokens are agent-infrastructure's analog of OS memory. The kernel's Capability Registry tracks per-Spirit `context_window_size` / `context_used` / `context_pressure_threshold`. Soft threshold (default 80%) emits a typed `ContextPressure` IAC frame; hard threshold (default 95%) emits `ContextLimit`; above 100% the kernel returns `EContextExhausted` on new tool calls. The Spirit's persona logic decides whether to distill, hand off, or halt. Token counts are Spirit-self-reported; the kernel does not estimate.

**Provider rate-limit isolation (Winston's addition).** Per-(provider, credential) token bucket with kernel-mediated backpressure surfaced as a typed `RateLimited` IAC frame, not a stalled call — the LLM-substrate analog of `EAGAIN`. One Spirit hitting Anthropic's RPM limit must not block another Spirit on a different provider, or even the same provider with a different key. Bucket parameters declared in provider driver config.

**Distillation as async with deadline (Winston's addition).** The §9.5 distillation step runs with a deadline; if exceeded, the parent Spirit gets a `DigestPending` reply with a continuation handle, not a blocked call. Parent decides whether to wait, proceed with stale digest, or kill. Shape: `gen_server:call`-with-timeout. Default deadline: 10 seconds; configurable per Spirit class.

**Network partition behavior in cross-host A2A (Winston's addition).** v0.1 explicit: A2A in-flight frames during partition are NACKed after a configurable timeout (default 30s); the kernel does NOT auto-retry. Application layer (the Orchestrator or peer Spirit) decides retry/escalate/halt. Cortex-scale partition tolerance is v2.0+ work.

**Logical clocks for Transparency Log ordering (Winston's addition).** Frame ordering uses logical clocks (Lamport or hybrid logical clock — final pick by v0.5); wall-clock is metadata only. Cross-host frame ordering is consistent under clock skew. Certificate validity windows remain wall-clock (X.509 conventions; we don't reinvent).

**Context-window upper bound (Winston's addition).** I12 `working_memory_digest_refs` cardinality is kernel-enforced. Soft cap: 80% of model context window (emits `ContextPressure`). Hard cap: 95% (emits `ContextLimit`). Above 95%, the Spirit must distill, hand off, or halt — kernel refuses new tool calls.

**Multi-protocol substrate (developer-tooling inheritance).** Kernel-internal IAC bus + A2A peer mesh + ACP server + MCP client (architecture §7; ADR-014 commits to using the existing four protocols rather than inventing a fifth). A2A frames carry typed-intent consent envelopes (ADR-012). MCP for tools and Loom; ACP for editor bridges; never conflate. **Four-protocol commitment (Winston's clause):** "MAOS will not add a fifth protocol unless (a) a use case is unsatisfiable by IAC + adapter, (b) a new ADR justifies it, and (c) demonstration that adding the protocol does not violate kernel-stays-small. Webhooks, gRPC, SSE, WebSocket — all are adapters into IAC, not peer protocols."

**Hot-swap correctness with sub-clauses (Winston's addition; ADR-017).** Invariant I6 is extended with explicit sub-clauses: in-flight A2A frames at the predecessor are inherited by the successor under a drain-barrier (not dropped); in-flight distillation steps restart at the successor with the same `digest_refs` set; Orchestrator-class user-input queues live in the Spirit's snapshot and survive swap; I12 `working_memory_digest_refs` are inherited; the state-transfer wire format is CBOR + per-Spirit-class schema (ADR-017), and the kernel rejects swaps with incompatible schema versions (typed `ESwapSchemaMismatch`).

**Memory tiering with kernel-enforced scope (Invariant I5).** Private (one Spirit instance) / shared (Host-wide) / collective (Loom-domain). Kernel rejects reads/writes outside declared scope. Distillation pattern (§9.5) operates within these tiers; digests can be elevated from working → episodic → shared with consent gates and audit-chain (I11) preserved.

### Integration Requirements

**Editor bridges (developer-tooling).** ACP server (NDJSON over stdio; convergent across Zed, opencode, hermes) for editor-hosted Spirits; v1.0 ships with Zed + VSCode tested. JetBrains via plugin-bridge in v1.5.

**Agentic CLI ecosystem (the wedge).** Worker Spirits are unmodified agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded (ADR-014). v0.1 / v0.5 ships with claude-code as the gold-standard reference; opencode / gemini-cli / kimi-cli reach parity in v0.5–v1.0.

**LLM provider drivers.** Pluggable per ADR-005. v0.1: Anthropic. v0.5: multi-provider via wrapped-CLI provider configs (no MAOS provider proxies yet). v1.5: MAOS-mediated provider proxies (intercept HTTP calls for substrate-layer audit). v2.0: full provider parity including local LLMs (Ollama / vLLM), air-gapped Bedrock-private, on-prem Bedrock, Vertex AI.

**Tool ecosystem (developer-tooling).** MCP-Streamable-HTTP client (ADR-008) — can call any MCP-compliant tool server. Loom is itself MCP-Streamable-HTTP. Spirit registry is MCP-Streamable-HTTP. Tool-side WASM sandboxing (T4) at v1.0; tool-as-Spirit at v2.0.

**Cross-host peer mesh (enterprise-security).** A2A peer mesh with mTLS + TOFU + per-frame ADR-012 typed-intent consent. v0.5 (pulled forward from v1.0 per Reza journey). Multi-Host certificate management is operator tooling; v2.0 adds optional org-internal CA support for Cortex-scale deployments.

**Identity and policy (enterprise; v2.0+).** OIDC / SAML for human authentication into operator surfaces. PDP integration (OPA / Cedar / Vault) for the Enterprise Spirit class — v2.0. SSO assertions flow through to capability-token issuance. Pre-v2.0 deployments use OS-user identity.

**Telemetry and observability (enterprise).** Per-Spirit telemetry stream (Invariant I7); broadcast subscriber model. v1.0 ships OpenTelemetry export adapter. v2.0 adds SIEM export (Splunk / ELK / Datadog). Transparency Log export distinct from telemetry — separate adapter, distinct retention policy.

**Skill ecosystem (developer-tooling).** Filesystem-discovered skill packages; v1.0 ships filesystem-only; v2.0 optional skill registry (separate from Spirit registry). Skills are user-space; kernel hosts no skills. Worker Spirits load skills on-demand by name when the Orchestrator delegates.

### Risk Mitigations (with v0.1 / v1.0 acceptance tests)

Per Murat's audit: every mitigation gets at least one falsifiable test. Eleven risks below — eight original + three Murat-added; Loom-specific threat model is explicitly deferred to a v2.0 threat-model doc.

**Risk 1 — Supply-chain compromise of Spirit registry.** Mitigations: Ed25519 signing on every Spirit version; trust tiers (ADR-009) with public-untrusted floored to T2 + cautious posture; community vetting attestations for promotion to public-vetted; SBOM publication; reproducible builds for reference Spirits; install-time signature verification mandatory. **Acceptance (v0.1):** every install is signature-verified; corpus of 10 unsigned and 10 forged-signature Spirits, all rejected. **Acceptance (v1.0):** vetter trust model documented (accreditation, revocation, conflict resolution); revocation latency median ≤ 60s, p99 ≤ 5min from CA revoke to peer rejection.

**Risk 2 — Adversarial Spirit (compromised or malicious third party).** Mitigations: capability scoping at manifest declaration + sandbox floor at trust tier (ADR-009); typed-intent consent on cross-Spirit frames (ADR-012); epistemic-halt-on-uncertainty surfaces the agent to the human before catastrophic action; Transparency Log makes silent betrayal mechanically detectable post-hoc. **Acceptance (v1.0):** adversarial-Spirit red-team corpus of 50 Spirits attempting (a) widening intent post-consent, (b) chaining a small consent into a large action, (c) using one capability to forge another's preconditions, (d) abusing epistemic-halt as a side channel, (e) Transparency Log evasion via timing. **Pass condition:** ≥ 48/50 detected pre-action by typed-intent consent; remaining ≤ 2 detected post-hoc by Transparency Log within 24h; zero undetected at 30-day audit.

**Risk 3 — Distillation audit-bypass (legibility attack).** Mitigations: I11 (digest must reference raw); I12 (decision frames record which digests the agent reasoned over); §9.5 five-metric ship gate (digest-recall ≥ 0.90, faithfulness ≥ 0.98, hedge-preservation ≥ 0.95, traceability = 100%, **secret-leakage = 0%**); judge-LLM async sampling in production; human spot-checks during v0.1 stabilization. **Acceptance (v0.1):** §9.5 five-metric gate passes on 100-case calibration corpus + 10⁵-case secret-leakage corpus.

**Risk 4 — LLM hallucination silently entering production.** Mitigations: epistemic halt as Layer-1 kernel capability — Spirits halt structurally on insufficient/contradictory evidence; per-tag epistemic policy with confidence thresholds; user resolves via `provided_context` / `accepted_halt` / `authorized_override`. **Acceptance (v0.5):** halt-recall ≥ 0.7 and halt-precision ≥ 0.85 per Spirit class on the `bmad-eval` standard corpus; results published in registry.

**Risk 5 — Cross-host trust at Cortex scale.** Mitigations: TOFU + mTLS for v1.0; org-internal CA + per-Host certificates for v2.0 Cortex-scale. **Acceptance (v1.0):** mTLS handshake replay-attack test (1000 captured handshakes replayed, 0 succeed); TOFU pin-mismatch on second connection blocks + logs + alerts (100% detection). **Acceptance (v2.0):** Cortex churn test — 100-host Cortex, 10% host turnover/week for 4 weeks, with 3 planted adversarial hosts; detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h with no human intervention beyond alert ack. TOFU-pin-conflict resolution: deterministic policy or escalation-to-operator (not silent split-brain).

**Risk 6 — Kernel-itself trust dependency.** Mitigations: kernel is small (LOC ceiling testable), OSS (license check testable), replaceable (compatibility-promise-testable). **Acceptance (v0.1):** kernel ≤ 15 KLOC excluding tests; capability-registry fuzz coverage ≥ 80% line, ≥ 60% branch, zero crashes on 1M-iteration libFuzzer run; zero `unsafe` blocks in capability-validation path. **Acceptance (v1.0):** external pen-test report with zero P0/P1 findings open at ship; capability-token TOCTOU test passes (100% of capabilities re-validated at use against current state).

**Risk 7 — Provider lock-in / concentration risk.** Mitigations: pluggable provider drivers (ADR-005); CLI-wrapper Spirits enable provider parallelism; v2.0 adds local-LLM and air-gapped provider parity. **Acceptance (v0.5):** ≥ 3 LLM providers tested in CI as drop-in replacements (Anthropic + OpenAI + local-LLM via Ollama).

**Risk 8 — Data residency violation (multi-region deployments).** Mitigations: per-region capability scoping; provider endpoint locks; A2A consent envelopes can include region-policy fields. **Acceptance (v1.0):** corpus of 100 cross-region operations, 100% of operations attempting region escape rejected with typed `ERegionLockViolation`.

**Risk 9 — LLM jailbreak via adversarial input (paste-into-context) [Murat-added].** Mitigations: input provenance tagging at IAC frame creation (every Worker-readable input frame carries a `source_class` field — `human-typed`, `tool-response`, `peer-spirit`, `web-fetched`, `file-read`); intent-vs-source mismatch detection (a Worker that emits `task.assign` with a higher-than-source-class intent triggers human-in-the-loop); epistemic halt on adversarial-intent indicators. **Acceptance (v1.0):** prompt-injection corpus of 100 known attack patterns (paste-into-system-prompt, indirect-injection-via-tool-output, role-confusion); ≥ 95 detected pre-action by intent-mismatch + halt; remaining ≤ 5 detected post-hoc by Transparency Log within 24h. Human-in-the-loop bypass-rate < 5% measured on 500 production interactions.

**Risk 10 — Capability-token leak via logs / digests / distillates [Murat-added].** Mitigations: pre-write secret-pattern redaction filter at the Transparency Log boundary (universal to all logged frames); §9.5 fifth metric `digest-secret-leakage = 0%`; I12 privacy refinement (separate ADR — decision-context frame_ids are themselves opaque references; the kernel does NOT cross-reference them with capability tokens that the requesting Spirit lacks read access to). **Acceptance (v0.1):** 10⁵-case planted-secret corpus with API keys / capability tokens / private-key bytes; pre-write filter must catch 100%; any false negative is a P0 ship-blocker.

**Risk 11 — Provider supply-chain compromise [Murat-added].** Mitigations: response cross-validation across providers for high-stakes decisions (Orchestrator may dispatch the same task to two Workers on different providers and compare digests for divergence); provider-driver integrity checks at MAOS startup (signature verification on the provider SDK package); telemetry stream emits provider-fingerprint per LLM call. **Acceptance (v1.5):** for Spirits configured with `cross_validation_required = true` (high-stakes class), divergence between provider responses triggers epistemic halt; corpus of 50 stress-tests with one provider returning corrupted output, halt fires in ≥ 48/50.

**Loom poison-pattern attack — explicitly deferred.** Loom-tier supply-chain and poison-pattern threats are tracked in a separate v2.0 threat-model document (`loom-threat-model.md`, drafted at v1.5 alongside Loom-lite). v0.1 / v1.0 do not address Loom-tier threats because Loom is not yet load-bearing; deferred is not silent.

### Cross-domain inheritance check

| Inherited domain | What we cover | What we defer |
|---|---|---|
| **Scientific computing** (reproducibility, validation) | Reproducible builds, signed Spirits, eval results, Transparency Log replay, halt-as-explainability, ComplianceClaim attestation chain | Cross-Spirit-version replay (Spirit ABI N-1 compatibility — §14 #4) |
| **Developer tooling** (SDK, framework, polyglot) | Spirit dev SDK, three Spirit forms, filesystem-discovered skills, Spirit registry, ACP/MCP/A2A, four-protocol commitment | Mobile-developer-experience surfaces (deferred to v2.0) |
| **Enterprise security** (audit, trust tiers, sandboxing) | Audit retention, sandbox tiers, capability mediation, secret pass-through, mTLS A2A, vetting attestations, role-distinct query primitives, ComplianceClaim, pluggable crypto trait | OIDC/SAML, PDP integration, SIEM export, FIPS/NIAP modules (all v2.0+) |
| **Agent infrastructure** (substrate-unique) | Epistemic halt as Layer-1, distillation audit-chain, decision-context recording, typed-intent consent, kernel statelessness, token-budget accounting (ADR-016), hot-swap state-transfer wire format (ADR-017), `ContextPressure`/`RateLimited`/`DigestPending`/`NetworkPartition` typed frames | Adversarial-Spirit threat model details (continues in Step 10), Loom threat model (separate v2.0 doc) |

## Innovation & Novel Patterns

### Detected Innovation Areas

The substrate-not-product framing is the **thesis** the innovations below collectively prove — not itself an innovation. Postgres, SQLite, Linux, K8s all describe themselves as substrates; the framing is shared aspiration across serious OSS infrastructure. What makes MAOS' substrate claim mechanically defensible (rather than rhetorical) is the integration of seven load-bearing primitives — six runtime/protocol/audit, one governance — under one set of invariants enforced uniformly across the kernel. Each primitive carries explicit prior-art citation; novelty claims are scoped to "first agent runtime to..." rather than the broader "first..." that wouldn't survive a capability-systems reviewer.

**1. Empty-kernel invariant operationalized (Invariant I9 made structural).** *The architectural keystone the other six depend on.* The kernel stores no patterns (Loom is user-space per ADR-006), no secrets (pass-through to OS keyring, never persisted), no skills (filesystem-discovered per Step 5, kernel hosts no registry), no Spirit memory beyond capability-token state, and no learned behaviors. **Novelty:** every prior agent runtime in the cohort survey accumulates state in the controller — prompts, memories, learned patterns, retrieval indexes. MAOS commits to *intentional kernel emptiness* and pushes all of that into Spirits and overlays. This is the reason ComplianceClaim is meaningful, Spirit portability is meaningful, and the substrate posture is mechanically defensible. **Prior art:** seL4's microkernel minimality discipline and the L4 family more broadly; Mach's externalized policy. **Differentiation:** seL4 minimizes the trusted computing base; MAOS minimizes the *stateful* computing base for a specific class of system (agent runtimes that would otherwise drift toward stateful controllers).

**2. Epistemic halt as Layer-1 kernel capability.** When a Spirit's evidence is insufficient or contradictory, the kernel exposes a structured halt; the user resolves with `provided_context`, `accepted_halt`, or `authorized_override`. Per-tag epistemic policy (verbalize / flag / halt) keeps the halt rare without losing recall on critical signals. **Novelty:** first agent runtime to expose "I don't know" as a kernel-mediated outcome with a typed resolution surface and per-tag policy. Database `RAISE EXCEPTION`, OS `EAGAIN`/`EINTR`, and Erlang OTP supervisors are all let-it-crash variants — none are epistemic halt at the agent-runtime layer. **Prior art:** Anthropic prompt-level "ask the user" conventions; OpenAI's `function_call: null`; uncertainty-aware control-flow research (literature pass pending before final lock). **Differentiation:** prior runtimes collapse epistemic uncertainty into a refusal-string or a confidence-weighted continuation; MAOS makes it a kernel event with a structured resolution path bound to I3 (auto-response marking), I7 (telemetry broadcast), and the Approval Decision Log.

**3. Substrate-level distillation pattern with kernel-enforced audit chain.** **Frame this as a theorem:** *given Invariants I11 (audit-chain) + I12 (decision-context recording), a legibility attack on a digest is mechanically detectable in O(1) per decision via digest-to-raw hash divergence and decision-context replay.* The five-metric ship gate (digest-recall ≥ 0.90, faithfulness ≥ 0.98, hedge-preservation ≥ 0.95, traceability = 100%, secret-leakage = 0%) makes the property verifiable at runtime. **Novelty:** first agent runtime to distinguish *legibility* (the digest the agent reasons over) from *audit* (the raw the auditor inspects), and the first to make both kernel-mediated. Components are borrowed (Postgres WAL since 1990s, content-addressed storage from Git/IPFS, structured-output gating from constrained decoding); the composition under I11+I12 with mechanical legibility-attack detection is the invention. **Prior art:** WAL+MVCC, in-toto attestation, accountable-systems literature (Weitzner et al. 2008). **Differentiation:** prior systems either show the agent the raw stream (legibility attack vector) or show it a summary with no kernel-enforced link to raw (unauditable). MAOS structurally rejects both failure modes.

**4. ComplianceClaim as runtime-context attestation primitive.** Ed25519-signed envelope binding (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint pinning + crypto-provider identity) into a single attestable subject. Third parties sign; the kernel verifies at admission and refuses Spirits whose runtime context drifts from the attested context. **Novelty:** first agent runtime to import the attestation discipline from the supply-chain world into the agent-runtime world, where it has been conspicuously absent. **Prior art:** TPM/TEE remote attestation (TCG spec since 2003); SLSA, in-toto, Sigstore for supply-chain provenance; accountable-systems literature (Weitzner, Feigenbaum 2008). **Differentiation:** prior attestation primitives attest to *artifact* provenance (this binary was built from this source); ComplianceClaim attests to *runtime agentic context* (this agent, in this scope, on this provider, was certified for this use case). The kernel's runtime verification is what makes the claim falsifiable rather than marketing copy.

**5. Typed-intent A2A consent (ADR-012).** Cross-Spirit consent is `(peer-identity, intent-class)`, not `(peer-identity)`. A read-only Spirit cannot pass a payload to a writeable Spirit that, when interpreted, causes a write the read-only Spirit was forbidden from. Extended in I13 (ADR-018) so digests carry `intent_lineage` and the consent envelope survives the §9.5 distillation boundary. **Novelty:** first agent runtime to operationalize capability-attenuated consent at the A2A boundary, treating Lampson's confused-deputy problem as a first-class concern rather than an emergent bug. **Prior art (cited explicitly):** Hardy's 1988 confused-deputy paper; the EROS/KeyKOS/Capn'Proto capability-systems lineage; the broader capability-theory tradition (capabilities-as-pure-pointer-tickets going back to the 1970s). **Differentiation:** the cohort survey treats inter-agent communication as channel-consent (talk-or-not), inheriting the confused-deputy problem the moment payloads can encode actions. MAOS recognizes that agent payloads are programs over the receiver's authority, and consent must be over the program's intent.

**6. Skill-package overlay model for heterogeneous CLI Spirits (ADR-014/015).** Worker Spirits are unmodified agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) with two skill packages loaded — a universal `maos-bridge` skill connecting via existing protocols (kernel-internal IAC / A2A / ACP / MCP per topology), and a persona skill defining the role. Skills are filesystem-discovered, loaded by name on the Orchestrator's instruction. **Novelty:** first agent runtime to make heterogeneous CLI Spirits a *first-class form*, exploiting the existing MCP capability of every target CLI without writing per-CLI Rust wrapper crates. The novel mechanism is the skill-package contract; the wrapping pattern itself is paperclip's. **Prior art (cited):** paperclip's CLI-as-subprocess-worker pattern; Unix philosophy (small composable tools sharing a substrate). **Differentiation:** paperclip's pattern was project-specific; MAOS generalizes via a kernel-mediated `task.assign` typed-intent IAC primitive that lets any MCP-capable CLI participate as a Worker Spirit by installing two skill packages. *Faithful application of Unix philosophy to LLM CLIs is a strength, not a weakness.*

**7. Constitutional substrate evolution (governance category).** *Mary's contribution; an innovation category the runtime list missed.* The kernel commits in writing to which extension surfaces exist (Spirits, skills, ComplianceClaim issuers, the four committed protocols) and what it takes to add a new one. Vetter trust model + ComplianceClaim chain + the "no fifth protocol unless..." constitutional commitment + cohort-interop posture together constitute a governance primitive. **Novelty:** first agent runtime to publish substrate-evolution rules as part of the substrate contract. **Prior art (cited):** Linux de facto governance (Linus + LKML + stable-ABI rule); K8s KEPs + SIGs; IETF RFC process; W3C consortium model. **Differentiation:** the cohort survey's agent runtimes are products with roadmaps, not substrates with constitutions. MAOS' constitutional evolution rules are the thing that makes "substrate-not-product" credible long-term — without it, any feature that breaks a Spirit erodes the substrate posture.

**Plus three architectural inventions worth surfacing at integration-coherence level:**

- **Integration coherence as load-bearing.** The fourteen invariants (I1–I14) are enforced uniformly across capability mediation, IAC, transparency log, approval surface, hot-swap, sandbox, distillation, decision context, intent provenance, halt continuity. **The unfair advantage**: a hyperscaler can ship any individual invariant in a quarter, but cannot ship the integration coherence without redesigning their architecture against the invariant set. Test: can a reviewer point to any of the fourteen and find a subsystem that violates it? If no, the claim is real.
- **Four-protocol no-invention commitment.** "We use kernel-internal IAC + A2A + ACP + MCP and invent zero new wire protocols. A fifth protocol requires (a) a use case unsatisfiable by IAC + adapter, (b) a new ADR, (c) demonstration that adding the protocol does not violate kernel-stays-small." **Architectural restraint as a feature** in a field that ships a new protocol per quarter.
- **Hexagonal-actor dual decomposition (ADR-010 ⊥ ADR-011).** Static-structure-via-ports-and-adapters orthogonal to runtime-structure-via-supervised-actors. Probably not primitive-level novel — someone has done it — but worth surfacing as the structural backbone that lets the kernel scale across implementation choices without violating the invariant set.

**Plus two internal mechanisms (less load-bearing for narrative; honest framing):**

- **Token-budget accounting (ADR-016) — cloud-cost FinOps with kernel enforcement.** Not "OS memory analog." The kernel accounts for a resource it doesn't own (LLM tokens are priced and consumed by the provider, not the kernel). Closer to FinOps than to memory accounting. Novelty: first agent runtime to make context-token budget a kernel-mediated concern with typed `ContextPressure`/`ContextLimit`/`EContextExhausted` frames.
- **Hot-swap CBOR + per-class versioned schema (ADR-017).** Not a novel wire format. Erlang OTP's `code_change/3` callback has done versioned state migration during hot code-load since the 1990s. CBOR is RFC 8949. Per-class schema versioning is every protobuf shop. The *novelty* is hot-swapping a Spirit (model + system-prompt + tool-set) mid-session with bounded state transfer, an agent-runtime concern Erlang never had.

### Market Context & Competitive Landscape

The agent-runtime landscape in 2026 splits into four classes (the original three "failed answers" plus an emerging substrate-shaped category at the tool-call and graph-orchestration layers):

| Class | Examples | Substrate position relative to MAOS |
|---|---|---|
| **Vendor-monolithic** | Claude Code, Cursor, ChatGPT app, Copilot | Agent + runtime ship together; closed; no third-party agents; structural incentive against MAOS-compatibility |
| **Cobble-it-yourself** | LangChain (legacy), AutoGen (legacy), prompt-driven shell scripts | Substrate is whatever you assembled; durable transparency log absent; no capability tokens; MAOS replaces this layer |
| **Roll-your-own kernel** | openclaw, ironclaw, hermes, paperclip, rustain, codex | Excellent within scope; doesn't generalize; trust is project-promise; MAOS is the layer they could share |
| **Substrate at adjacent layers** | Anthropic MCP (tool-call layer); LangGraph, AutoGen v2 (graph-orchestration layer) | Aspiring substrates above MAOS' kernel + trust layer; complementary, not competitive — MAOS is the layer beneath them |

MAOS positions in the **kernel + trust** layer (this PRD's working competitive frame, retained from earlier steps): the trusted runtime under which third-party autonomous agents execute. The reference class is OSS infrastructure (Linux / Postgres / Kubernetes / Apache HTTPD); the bet is that "substrate-too-early" by commercial measure is "substrate-just-in-time" by ecosystem measure (per Step 2c). An alternate framing — "agent-ecosystem-trust-anchor" in the Mozilla-CA / IETF / W3C reference class — was raised by John in Step 6 party mode and **routed to Step 8 (Scoping) as an explicit alternative-framing carry-forward**. The choice between the two frames affects which innovation is the spear tip (substrate framing → empty-kernel + the integrated set; trust-anchor framing → ComplianceClaim alone). Final lock by v0.3 per Step 2c carry-forward.

**Observable competitive validation signals:**

- **2026 Q4 onward:** any cohort project (openclaw / ironclaw / hermes / paperclip / rustain / codex) integrating MAOS as their substrate or interoperating via ACP/MCP/A2A. First cohort interop is a v1.0 success criterion.
- **2026 H2 (alternative early signal — John's contribution, retained):** first auditor or regulator references a MAOS Transparency Log frame (or ComplianceClaim) in a published finding. Sharper, earlier indicator under trust-anchor framing.
- **2027:** vendor-monolithic competitors adding "MAOS-compatible" mode (export action history as MAOS Transparency Log frames; accept MAOS skill packages). Substrate-crystallization signal under the OSS-substrate framing.
- **2028:** established certification body issuing first ComplianceClaim against a third-party Spirit running in MAOS T2 sandbox at public-vetted tier. Compliance-as-product validation.

### Validation Approach

Each innovation has falsifiable validation methods. Per Murat's Step 6 testability audit, every cell below is either rigorous (corpus-based with floors), structural (kernel-enforced, mechanically verifiable), or explicitly anecdotal-by-design (where N=1 partner adoption is the validation goal). Cells flagged "anecdotal" are program-risk; alternative synthetic protocols are provided where they exist.

| Innovation | v0.1 validation | v1.0 validation | v2.0 validation |
|---|---|---|---|
| **1. Empty-kernel invariant** | Kernel LOC ceiling ≤ 15 KLOC excluding tests; capability-registry fuzz coverage ≥ 80% line / ≥ 60% branch with zero crashes on 1M-iteration libFuzzer; zero `unsafe` blocks in capability-validation path; **structural test:** every state-bearing field in kernel source documented and audited against I9 ("kernel stores no patterns"). | External pen-test report with zero P0/P1 findings open at ship; capability-token TOCTOU test passes (100% re-validation at use). | **FKCS — Frozen-Kernel Conformance Suite (Murat).** Tag commit `kernel-frozen-v2.0`; external Spirit authors (no kernel-repo commit history) implement Negotiator/Tutor/Wet-Lab against published ABI documentation only; floor ≥27/30 success per Spirit, ≥85/90 aggregate; diff oracle (third-party certifier) asserts `git diff kernel-frozen-v2.0..at-test-time` is empty for `kernel/` and `abi/`; negative-control "fourth Spirit" uses an undocumented kernel internal and MUST fail FKCS or the test has no teeth. |
| **2. Epistemic halt** | Halt mechanism functional in Architect Spirit; resolution paths complete; **structural test:** halt frame schema validates; per-tag policy parser covers all five action types. | Halt-recall ≥ 0.7 and halt-precision ≥ 0.85 per Spirit class on `bmad-eval` standard corpus, published in registry. **Literature-pass deliverable:** Winston-led prior-art review of 2024–2026 uncertainty-aware-control-flow research before final novelty-claim lock. | Halt patterns adopted as a published standard; cohort projects citing the per-tag epistemic-policy schema. |
| **3. Distillation pattern (frame as theorem)** | Five-metric gate passes on Orchestrator + 100-case calibration corpus + 10⁵-case secret-leakage corpus. **Structural property:** given I11+I12, legibility-attack detection is O(1) per decision via digest-to-raw hash divergence — verified by reference test suite. | Five-metric gate passes on all distillation-shipping reference Spirits; kernel I11+I12 enforcement validated by external pen-test (pen-tester quality bound: ≥3 pen-testers from different organizations). **Canary system (Murat):** quarterly 20% corpus rotation; production canaries 1000 unique synthetic secrets/month with cryptographic markers, cross-Spirit canaries 100/month; floor 0 canary leaks/month; ≥1 detected → distillation pipeline halts (real halt) until root-caused; discovery-latency floor ≤24h p95. | Multi-hop distillation chain audit (audit-Spirit walks digest → raw across Cortex hops); divergence detection automated; intent_lineage propagation verified per I13. |
| **4. ComplianceClaim** | Envelope schema specified in architecture; admission-time verification implemented; **structural test:** `ComplianceClaim` parser validates well-formed envelopes (schema conformance test corpus). | **CCAC — ComplianceClaim Adversarial Corpus (Murat).** N=500 synthetic claims: 200 well-formed (≥199/200 admitted, FPR ≤ 0.5%); 200 malformed across 10 violation classes (≥196/200 rejected, ≥18/20 per class); 100 context-drift claims (100/100 rejected with drift-specific error code). Cross-validation across ≥3 reference Spirits, agreement within ±2% per metric. Reproducible CI gate, no pilot-partner dependency. | First third-party ComplianceClaim issued against a reference Spirit by an accredited assessor (anecdotal milestone, partner-dependent — pilot-partner version stays as v2.0 real-world signal); public registry of ComplianceClaims; ≥3 certification bodies issuing claims. |
| **5. Typed-intent consent + I13 intent-lineage** | ADR-012 implemented; A2A frames carry typed `intent` field; receiver consent allowlist enforced. **Structural test:** I13 enforced — every digest write rejected without `intent_lineage`; consumer admission rejected when `intent_lineage ⊄ allowed-promotion-set(Y)`. | Adversarial-Spirit red-team corpus: 50 attacks (widening intent post-consent, chaining consents, capability forgery, halt-side-channel, log-evasion-via-timing). Floor: ≥48/50 detected pre-action by typed-intent consent; remaining ≤2 detected post-hoc within 24h; zero undetected at 30-day audit. **Plus I13 cross-innovation test (Murat):** 200 synthetic distillation traces, 80 mixed-intent, 40 consent-laundering attempts; floor 40/40 detected and rejected, 0 false rejections in 120 well-formed cases. | Typed-intent vocabulary published as community-evolved standard; cohort projects adopting the consent shape (sociological signal, not engineering metric). |
| **6. Skill-package overlay (CLI-wrapper Spirits)** | Orchestrator + claude-code-Worker reference deployment runs Lunarpulse's Epic-7 BMAD loop end-to-end. **De-risk dependency:** ensure MAOS works on at least one fully-open CLI agent (codex-class) before betting on closed CLIs (John's pushback — vendor incentive to close hooks is a real risk). | Multi-CLI Worker parallelism (claude-code + opencode + gemini-cli + at least one open codex-class CLI) on same epic; provider parallelism is structural. **Plus ComplianceClaim × CLI-drift test (Murat):** 100 claims across `claude-code@1.4.x` patches → ≥99/100 valid; cross-minor bumps → 100/100 auto-rejected; drift detection ≤1 cycle on 100/100 synthetic upgrades. | Public skill registry (separate from Spirit registry) with ≥20 community-authored bridge skills; if vendor-closes-hooks scenario materializes, native Spirit Wire Protocol implementation per CLI is the v2.0 fallback (ADR-014 alternative path). |
| **7. Constitutional substrate evolution (governance)** | Vetter trust model documented; "no fifth protocol unless..." commitment in architecture; bounded-extension governance rules published. | First externally-issued vetting attestation against a public-untrusted Spirit promotes it to public-vetted; revocation latency median ≤ 60s, p99 ≤ 5min; conflict-resolution semantics tested with 10 synthetic two-vetters-disagree scenarios. | Cohort-interop demonstration: ≥1 cohort project formally citing MAOS substrate-evolution rules in their own contribution model. |

**I14 cross-innovation test (Murat — hot-swap × halt continuity):** 50 hot-swap scenarios with non-empty halt sets, mixed across drain-eligible and migration-required. Floor: 50/50 either drained before swap OR migrated cleanly with halt-protocol-compatibility verified; 0 dropped halts; 0 successor-confusion events (defined as: successor responds to halt with a resolution that doesn't match the halt's declared resolution schema). v0.5 ship gate when hot-swap and halt both ship.

The most expensive validations are #4 v2.0 (pilot partner showing up) and #5 v2.0 (community-evolved-standard adoption — sociological). Both are tractable; both are deferred from earlier-phase ship gates per Murat's "kill the pilot dependency" reframe — CCAC at v1.0 makes ComplianceClaim's primitive ship-gate testable without external dependencies.

### Risk Mitigation

For each innovation, the failure mode and the fallback:

| Innovation | Failure mode | Fallback |
|---|---|---|
| **1. Empty-kernel invariant** | Kernel state creep over time (a feature lands that requires kernel-side accumulated state) | Every kernel-state addition requires an ADR explicitly amending I9, with explicit rationale why the user-space alternative was rejected; reviews block silent state addition. The invariant is the load-bearing constraint — its erosion is treated as an architecture incident. |
| **2. Epistemic halt** | Halt over-triggers (precision < 0.85 → users disable halt) or under-triggers (recall < 0.7 → hallucination still leaks) | Per-tag epistemic policy gives operators per-Spirit knobs; failure-bound v0.5 ship gate (halt-precision floor) blocks ship until over-triggering is fixed. If a Spirit class genuinely cannot meet the floor, it ships with `default_action = "verbalize_only"` and a documented limitation. |
| **3. Distillation pattern** | Five-metric gate fails on real workloads (digest-recall < 0.85 in production despite passing in calibration); canary system detects production leaks | Mandatory raw-recall on any decision frame; ship-with-warning state for Spirits in 0.85–0.90 zone; canary detection halts the distillation pipeline; fallback to "all results in active context, accept context-window failures" pattern for distillation-failing Spirits while the gate is fixed. |
| **4. ComplianceClaim** | No certification body adopts the envelope format; remains substrate-only with no third-party signers; or vendor-closes-hooks scenario invalidates the runtime-context fingerprinting | The internal audit / DPO / CISO surfaces (Mary's role-distinct queries from Step 5) still serve internal compliance even without external attestations; ComplianceClaim becomes optional but the binding primitive remains for self-attestation in regulated enterprises. CLI-drift mitigated by patch-level fingerprint with explicit invalidation-on-minor policy. |
| **5. Typed-intent consent + I13** | Confused-deputy gap closed at consent layer leaks at Spirit-layer (a Spirit asks the human to widen intent; human rubber-stamps); intent_lineage propagation has implementation bugs | Human-in-the-loop bypass-rate < 5% becomes a v1.0 NFR ship gate; intent-vs-source mismatch detection (Step 5 Risk 9) layers underneath; epistemic halt on adversarial-intent indicators is the third layer; I13 enforcement is structurally verifiable (digest writes without `intent_lineage` rejected by kernel). |
| **6. Skill-package overlay** | Target CLI's MCP/hook surface degrades or is removed (vendor-closes-hooks scenario — John's risk) | Per-CLI native Rust wrapper crate is the v2.0 fallback (ADR-014 alternative `(a)`); native Spirits implementing Spirit Wire Protocol directly remain the v2.0+ path for Spirit classes whose runtime needs to be MAOS-native. v0.5 de-risk: prove out the substrate on codex-class fully-open CLI alongside Claude Code. |
| **7. Constitutional substrate evolution** | Substrate-evolution rules are violated (a kernel feature lands that breaks a Spirit without going through the documented ADR + revisit-trigger chain) | Treat as an architecture incident; require post-hoc ADR + Spirit-author migration support; document the rule violation in the Open Questions section; community-vetting board can flag breaches. The invariant is the substrate's governance promise — its erosion is treated like Linux's stable-ABI rule violations (rare, high-cost, repaired). |

### Carry-Forward to Step 7 — Failure Semantics Gap (Mary)

The Step 6 innovations cover *uncertainty* (epistemic halt) but NOT *failure* — Spirit crashes, partial-result recovery, cohort-state divergence after a network partition. Every prior OS substrate has a failure-semantics story (Erlang's let-it-crash, K8s reconciliation loop, Linux OOM killer); MAOS' is currently distributed across I10 (lifecycle journaling) and ADR-011 (Tokio actor supervision) without a unifying primitive named as innovation. **Carry forward to Step 7 (Project Type Analysis):** evaluate whether failure semantics warrants its own innovation slot or stays as a kernel-mechanism backbone. If the former, expect a new ADR + invariant pair; if the latter, document the integration of I10 + ADR-011 as the substrate's failure-semantics story.

### Deferrals and Routing

- **Trust-anchor competitive reframe (John):** routed to Step 8 (Scoping) as an alternative-framing carry-forward. Choosing OSS-substrate vs trust-anchor frame affects which innovation is the spear tip. Final lock by v0.3.
- **Literature pass on epistemic halt prior art (Winston):** before final novelty-claim lock. If 2024–2026 academic work on uncertainty-aware control flow already operationalizes what MAOS does at the kernel level, soften the claim further; otherwise hold.
- **Loom poison-pattern threat model:** explicitly deferred to v2.0 threat-model document (per Step 5 routing).
- **Adversarial-Spirit threat model:** continues into Step 10 (NFRs) with STRIDE-style attack narratives.

## Developer Tool Specific Requirements

MAOS' primary classification is `developer_tool` (per Step 2). The substrate's customers are *Spirit authors, kernel implementers, and operators*. Secondary traits — `cli_tool` (`maosctl`), `api_backend` (ACP server, A2A peer, control-plane HTTP, MCP outbound), `desktop_app` (one Host process per machine) — surface as integration concerns, not primary product shape. This section pulls the developer-tool-specific commitments scattered across the architecture and implementation guides into a single PRD reference, plus the operational, testing, and documentation commitments needed to ship a substrate-class OSS developer tool credibly.

### Project-Type Overview

The Spirit-author-as-customer relationship is load-bearing. The substrate's value compounds when third-party Spirit authors can ship Spirit binaries independently of the MAOS source tree, in any language, signed, with capability scopes and trust tiers verified at install. Two adjacent customer relationships sit alongside:

- **Kernel implementer** — the Rust developer building the `maos` binary itself. Audience for `maos-kernel-implementation-guide.md`.
- **Operator** — the person running MAOS on a Host. Audience for `maosctl` and the deployment-topology docs.

The three customers share Spirit ABI as the contract; they diverge on tooling needs.

### Language Matrix

Three Spirit forms over the v0.1 → v1.0 → v2.0 timeline (per ADR-007):

| Form | Phase | Languages | Toolchain | Reference |
|---|---|---|---|---|
| `rust-inproc` | v0.1+ | Rust only. Spirit binary linked into kernel binary. | `cargo build` against `maos-spirit-sdk`. | `spirit-development-and-sharing.md` §4.1 |
| `subprocess` (incl. CLI-wrapper Spirits per ADR-014/015) | v0.5+ | Any language with a Spirit Wire Protocol implementation. Reference SDKs: Rust (canonical), TypeScript (v0.5), Python (v1.0), Go (v1.5+). For CLI-wrapper Spirits: any agent CLI process loaded with `maos-bridge` + persona skills. | Spirit-author's preferred toolchain; `spirit-test` SDK harness. | `spirit-development-and-sharing.md` §4.2; ADR-014, ADR-015 |
| `wasm-component` | v2.0+ | Any WASM Component Model language: Rust, C/C++, JS/TS (Jco), Python (componentize-py), Go (TinyGo). | `cargo component` or language-specific component-model toolchain. | `spirit-development-and-sharing.md` §4.3; ADR-007 |

**Cross-form portability commitment.** Same crate, three feature flags, shared core: `cargo build --features=form-{rust-inproc,subprocess,wasm-component}` — author writes against `Spirit` once, form glue is feature-gated. *Capability scopes are not portable*: a Spirit calling `std::process::Command` builds under `subprocess` and `rust-inproc` but is rejected by the `wasm-component` build at compile time. Manifest declares `forms = ["subprocess", "rust-inproc"]` if WASM is impossible; the registry refuses WASM builds for that class. Don't promise three-form portability as default — promise *form-explicit* portability where the author opts into the forms they support, and `spirit-test` is the source of truth.

Skill packages are markdown + frontmatter — language-agnostic by design. Bridge skills require no per-language compilation.

### Installation Methods

**Kernel (`maos` binary):**
- v0.1: source build (`cargo install --path crates/maos-bin`); Linux + macOS only.
- v0.5: pre-built binaries for Linux (amd64, arm64) and macOS (arm64) via GitHub Releases. SHA256 + Ed25519 signature verification mandatory.
- v1.0: Homebrew tap, AUR (Arch), Debian/Ubuntu deb, RHEL/Fedora rpm. Container images on Docker Hub / GHCR. Windows binary at v1.5.
- v2.0: official Linux distro packages (Debian/Ubuntu main, Fedora repo). One-line install script (`curl install.maos.dev | sh`) for the founder-loop demo.

**Spirits:**
- Reference Spirits ship with the `maos` binary.
- Third-party Spirits install via `maosctl install <spirit-id>[@version]`. Per ADR-008: MCP-Streamable-HTTP call to the Spirit registry; Ed25519 signature verified; trust-tier floor enforced (ADR-009); ComplianceClaim verified at admission (ADR-015 / Step 5); manifest validated.
- Custom Spirits load from local filesystem (`maosctl install --from-path ./my-spirit/`).

**Skills:**
- v0.1–v1.0: filesystem only; conventional locations (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`).
- v2.0: optional skill registry, separate from Spirit registry, content-addressed with Ed25519 signing.
- BMAD skills (`bmad-create-story`, `bmad-dev-story`, `bmad-code-review`, etc.) work as-is — no MAOS-specific port required.

**Agent CLIs (Worker Spirits):** brought in via existing distribution — `npm install`, `pip install`, `brew install`. MAOS does not redistribute or vendor agent CLIs.

### Spirit Lifecycle — install, upgrade, yank, uninstall, revoke

A first-class commitment: every install verb has a documented inverse and a kernel-side enforcement story. The substrate's install promise obligates the substrate's revoke promise (Mary's launch-day-embarrassment gap).

| Verb | Trigger | Kernel behavior | Effect on running instances | Effect on claims/audit |
|---|---|---|---|---|
| **install** | `maosctl install <spirit-id>[@version]` | Pull from registry → verify signature → verify ComplianceClaim envelopes if present → enforce trust-tier sandbox floor → register Spirit class in local index | None (this is admission, not instantiation) | None |
| **upgrade** | `maosctl upgrade <spirit-id>[@version]` | Pull new version → verify → if running instances exist, defer to `maosctl swap` (hot-swap with ADR-020 migration policy applied) | Hot-swap per ADR-017/020/I14; running instances migrate or drain | Outstanding ComplianceClaims re-verified against new runtime context (ADR-015); migrate forward if compatible, halt if drift detected |
| **yank** | Registry author marks version withdrawn (publication event) | Kernel polls registry every 5 min for yank events; on yank, emits typed `SpiritYanked{spirit-id, version, reason}` to operator surface; **does NOT auto-stop running instances** | Instances continue running unless operator explicitly stops; operator notification is mandatory | Audit log records the yank event with timestamp and reason |
| **uninstall** | `maosctl uninstall <spirit-id>[@version]` | Remove from local index; refuse if running instances exist (force flag required: `--force`); refuse if outstanding ComplianceClaims (review flag required: `--orphan-claims`) | Operator must stop instances first (or use `--force` with explicit confirmation) | Outstanding ComplianceClaims become orphaned and tagged as such in the Approval Decision Log; audit trail preserved (Spirit history not deleted) |
| **revoke** | Operator-issued or registry-issued *trust event* (signed, dated, distributable offline) | Kernel honors revocation list (CRL-shaped artifact: `(spirit-id, version, revocation-key, reason, ts)` signed with operator or distributor key); on revocation, **immediately blocks new instantiation and emits typed `SpiritRevoked` to all running instances** | Running instances receive `SpiritRevoked` and follow declared policy: `terminate-immediately` (default for security) / `drain-then-terminate` (configurable per Spirit posture) / `quarantine` (running but no new tool calls) | Audit log records the revocation event with full chain |

**Signed Revocation List (CRL artifact).** v1.0 ships with two distribution paths: (a) registry-pushed (kernel polls registry's `/revocations` MCP endpoint every 5 min); (b) offline-import (`maosctl revocations import <bundle.crl>` for air-gapped deployments). CRL signing follows the same Ed25519 chain as Spirit signing; operator can pin trusted revocation signers.

**The yank vs revoke distinction.** Yank is a registry-side publication event (the registry will not serve this version to fresh consumers); revoke is a kernel-side trust event (the kernel will not run this version on this Host regardless of where it came from). They are different artifacts with different signing chains. The PRD commits to both.

**Substrate Operations Checklist** (Mary's organizing line):

| Concern | Owner | Target version | Artifact |
|---|---|---|---|
| Install/upgrade UX | core team | v0.1 | `maosctl install` + tests |
| Yank notification | core team | v0.5 | Registry polling + operator notification |
| Uninstall semantics | core team | v0.5 | `maosctl uninstall` with claim-check guards |
| Signed revocation list | core team | v1.0 | CRL artifact spec + distribution paths |
| Audit query / SIEM export | core team | v1.0 (basic), v2.0 (signed export) | `maosctl audit query` + sealed-export |
| Telemetry opt-out | core team | v1.0 | Opt-in default + `PRIVACY.md` + per-field redaction layer |
| LTS window | maintainer team | v1.0 announcement | `STABILITY.md` + LTS branch policy |
| Namespace grammar | architecture | v0.5 | New ADR (flat vs scoped Spirit names) |

### Namespace Grammar (Mary's gap)

**Commitment:** by v0.5, an ADR locks the Spirit namespace grammar — flat names (`bmad-orchestrator`) vs scoped (`@bmad/orchestrator`, `org.bmad.orchestrator`). Without a grammar, the first publication race decides the namespace forever and trademark / squatting become permanent. Default working assumption: scoped (`@scope/name`) following npm/Cargo convention; final lock in the v0.5 ADR.

### API Surface (Spirit ABI)

Three logical surfaces backed by `maos-spirit-abi` (per `maos-kernel-implementation-guide.md` §3.2):

**1. The `Spirit` trait — kernel calls into Spirit.** Lifecycle hooks: `on_load`, `on_start`, `on_frame`, `on_telemetry`, `on_idle`, `on_swap_in`, `snapshot`, `epistemic_resolve`, `on_pause`, `on_resume`, `on_unload`. Plus, per ADR-020, optional `migrate(predecessor_state)` for cross-major migration.

**2. The `KernelHandle` trait — Spirit calls into kernel.** IAC (`iac.send`/`iac.receive`/`iac.broadcast` with ADR-012 typed-intent consent), Memory (`memory.read`/`memory.write` with I5/I11/I13 enforcement), Capabilities (`capability.invoke`), Provider (`provider.stream`), Log (`log.recall`/`log.fetch` per ADR-013), Halt (`epistemic.halt`), Approval (`approval.request`).

**3. Manifest schema (TOML).** `[class]`, `[capabilities.required]`, `[posture]`, `[output_shape]`, `[explanation_shape]`, `[epistemic_policy]`, `[budget]`, `[skills.search_path]`, `[forms]` (cross-form portability declaration), `[hot_swap]` with `state_schema_uri` + `state_schema_version` (ADR-017), `[halt_protocol_compatibility]` (I14), `[intent_promotion_set]` (I13), `[migrates_from]` (ADR-020), `[swap_invariants]` (HSIS — Murat's gate). Full schema in `architecture-maos.md` §5.1.

### ABI Stability Triple (Winston's commitment; matrix in `STABILITY.md`)

Compatibility is `(kernel_version, abi_version, manifest_schema_version)` — a triple, not a pair. `abi_version` governs the `Spirit`/`KernelHandle` vtable + capability ID space; `manifest_schema_version` governs the TOML surface independently; `kernel_version` is product-facing.

**Rule:** Spirit declares `abi`; kernel adapts down via `Compat` shim; **N-1 supported, N-2 hard refusal** with typed `EAbiTooOld`.

**Deprecation timeline:** 2 minor releases of warning, 1 major to remove. Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`.

**Live matrix:** lives in `STABILITY.md` (separate doc; grows over time without re-approving the PRD). PRD commits to the triple's existence and the N-1/N-2 rule.

### CLI-Wrapper Spirit Specification (Winston's gap; ADR-021)

CLI-wrapper Spirits (Path A migration) use the kernel-builtin `CliWrapperSpirit` class, configured with: CLI binary path; skill bundle (`maos-bridge` + persona skills); **`output_shape_version: "<semver>"`** (ADR-021 — kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed != declared); posture declaration (stdio shape, control-channel mechanism, shutdown signal); capability scope mapping; output-shape adapter implementation (registered as `cli-wrapper-template:<cli-name>:<shape-version>` in the Spirit registry); crash semantics (kernel observes EOF on stdio + non-zero exit → `SpiritDied` event journaled; recovery policy declared in wrapper config: `respawn-with-context` / `respawn-fresh` / `escalate`).

**Fail-loud rule:** wrappers cannot fall back to "best-effort parsing" on shape mismatch. Audit drift is the failure mode the substrate cannot tolerate.

**Realistic 30-minute claim:** valid only when a wrapper template for the target CLI already exists in the registry. First-time-wrapping a net-new CLI class (kimi-cli, codex, future CLIs) is **half-day minimum** because the author is also authoring the output-shape adapter. The PRD distinguishes both numbers honestly.

### Hot-Swap Migration Policy (Winston's decision tree; ADR-020)

Four cells keyed on (schema-evolution × persistent-state):

| Schema evolution | No persistent state | With persistent state |
|---|---|---|
| Same major, additive | Auto-migrate | Auto-migrate |
| Same major, breaking | Forbidden (use major bump) | Forbidden (use major bump) |
| Cross-major, no archives | Swap permitted; predecessor archives refused | N/A |
| Cross-major, archives present | Migrator Spirit required | Migrator Spirit required |

Manifest field `migrates_from = ["1.x", "2.x"]` declares which predecessor versions a Spirit can hot-swap from. Cross-major migration with persistent state requires a `migrate(predecessor_state) -> Result<successor_state, Error>` entry point. Kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared. **Predecessor's historical journal stays in cold storage**, addressed by `(class, version, instance_id)`; successor reads via capability but does not own (preserves I10 across version boundaries).

### Code Examples

Three canonical examples in the Spirit-development guide:

1. **Minimal Rust in-process Spirit** (~40 lines + manifest) — the "30-minute first Spirit" tutorial in §4.1.
2. **Subprocess Spirit in TypeScript** — Diego's `code-reviewer-pro` (Step 4 J6); demonstrates JSON-RPC over stdio Spirit Wire Protocol, output_shape enforcement, signing for `public-untrusted` registry submission.
3. **Skill-package overlay** — `developer` + `maos-bridge` skills loaded into a `claude-code` process; demonstrates Pattern A (Bash-invoke external CLI) and Pattern B (direct slash-command).

**Worked example — the founder's epic-7 loop.** End-to-end trace from `@orchestrator run epic-7` through `task.assign` IAC frame routing, Worker Spirit skill loading, distillation pattern execution, halt-on-AC-ambiguity resolution, and morning digest production.

### Migration Guide

Two paths:

**Path A — agentic CLI tool already exists.** Install `maos-bridge` skill + persona skill; configure CLI to start with both loaded; declare `output_shape_version` (ADR-021); register with `maosctl spirit register --form=cli-wrapper`. **30 minutes for CLIs with published wrapper templates; half-day for net-new CLI classes** (output-shape adapter authoring).

**Path B — third-party agentic framework or tool.** `cargo generate maos-spirit --form=subprocess --lang=<lang>`; implement `Spirit` trait or Wire Protocol equivalent; author manifest declaring capability scopes, posture, output_shape, epistemic_policy; run `spirit-test`; publish: `maos-spirit publish --tier=public-untrusted`; iterate to community-vetted via attestations (ADR-009). **Effort: weeks per port, mostly behavior-code authoring.**

Diego's *"Why I deleted 4,000 lines of HTTP/SDK glue code by becoming a MAOS Spirit"* (Step 4 J6) is the canonical Path-B success narrative.

### Numeric Ship-Gate Floors (Murat's audit)

Every developer-tool quality claim gets a falsifiable numeric floor. Aspirational language ("substrate quality") is replaced with verifiable thresholds.

| Gate | Floor | Phase | Owner |
|---|---|---|---|
| `spirit-test` fixture-corpus pass rate | ≥ 98% | v1.0 ship to public-untrusted | Spirit author |
| `spirit-test` class regression-corpus pass rate | ≥ 95% | v1.0 (corpus sealed by registry) | Spirit author |
| Manifest self-check (declared output_shape matches produced) | 100% | v0.1 | Spirit author / kernel |
| Cross-form Semantic equivalence (rust-inproc ↔ subprocess) | ≥ 90% on 200-scenario class corpus | v1.0 | Cross-form harness |
| Cross-form Semantic equivalence (any-rust ↔ wasm-component) | ≥ 75% (lower because wasm has different determinism) | v2.0 | Cross-form harness |
| CLI-wrapper Behavioral-Distributional equivalence | Mann-Whitney U-test p > 0.05 over 30 runs per scenario | v1.0; separate registry label `conformance: behavioral-distributional` | Wrapper author |
| ABI compatibility matrix coverage within current major | 100% (every minor pair, both directions) | v0.1 | Kernel team |
| ABI N-1 major boundary | 100% incl. negative typed-error cases (`EAbiTooOld`) | v1.0 | Kernel team |
| Manifest field test coverage | ≥ 3 cases per field (well-formed / malformed-rejected / edge-case) | v0.1, CI-enforced | Kernel team |
| **Manifest parser fuzz** | 24h `cargo-fuzz`, zero crashes / OOMs / infinite loops | **v1.0 ship gate** (log4shell territory) | Kernel team |
| Wire protocol cross-language byte-equal golden corpus | 100% per frame variant per SDK (Rust / TS / Python / Go) | v1.0 | Kernel + SDK teams |
| Wire protocol schema-evolution coverage | 4 cases per frame variant (old→new additive; new→old additive-only; new→old with deprecated; new→old breaking → typed reject) | v1.0 | Kernel team |
| Wire protocol adversarial-input fuzz | 24h fuzz, zero crashes | v1.0 ship gate | Kernel team |
| **Hot-Swap Invariant Suite (HSIS)** — pass rate per Spirit class | **≥ 95%, zero invariant violations (CVSS-7 class)** | **v1.0 ship gate** | Spirit author + kernel |

**HSIS specification.** For every Spirit class, run a 50-scenario corpus where a swap is injected at randomized lifecycle phases. Floor:
1. Successor emits `on_swap_in` ack within 100ms of swap signal.
2. Successor preserves declared invariant set (manifest `swap_invariants: [...]` field).
3. Final task output passes ≥ 95% Semantic equivalence vs no-swap control.
4. Total work-product divergence within manifest-declared `tolerance_band`.

**Cross-language byte-equal golden corpus.** Every frame variant gets a `golden/<frame_name>.json` committed to repo; every language SDK serializes a constructed frame → byte-equal golden; deserializes golden → structurally-equal frame. Canonical encoding: sorted keys, no whitespace, UTF-8 NFC.

### Typed Error Catalog (Paige's must-not-ship-without)

The substrate's most-trafficked reference page must exist on day one. **PRD ship-gate commitment:** every typed error declared in `maos-spirit-abi` has a corresponding catalog page generated from a structured docstring; CI fails if a new error variant lacks the catalog metadata.

**Catalog format (per error):**
- Error name (`EDigestAuditChainMissing`)
- One-line description
- What caused it (typically: which kernel check failed, which precondition was violated)
- How to recover (Spirit-author-side fix)
- Code example of the trigger
- Code example of the handler
- Related errors (cross-references)
- Stability: which kernel version introduced it; deprecation status if any

**Stable URL pattern:** `https://docs.maos.dev/errors/<ERR_NAME>`. Versioned per kernel release; archived versions retained ≥ 2 minor releases back.

**v1.0 covered errors (current named set, will grow):** `EDigestAuditChainMissing`, `EIntentPromotionDenied`, `EHaltContinuityViolation`, `EContextExhausted`, `EComplianceContextDrift`, `ESwapSchemaMismatch`, `ERegionLockViolation`, `EAbiTooOld`, `EMigratorMissing`, `EOutputShapeAdapterMismatch`, `SpiritYanked`, `SpiritRevoked`, `SpiritDied`, `EChannelClosed`. CI lint enforces the catalog metadata presence per variant.

### Documentation Artifacts (Paige's missing-list)

**Diátaxis honesty.** The current doc set is *Diátaxis-aware at the section level, Diátaxis-violating at the document level*. The PRD does not claim Diátaxis compliance; it commits to specific artifacts that move toward it.

| Artifact | Form | v0.5 | v1.0 | v2.0 |
|---|---|---|---|---|
| **API reference site** at `https://docs.maos.dev/abi/<version>/` | `cargo doc` published to GitHub Pages on every release tag, with `abi/latest` alias; versioned, searchable (Algolia DocSearch or Pagefind), deep-linkable | ✓ basic | ✓ search + version dropdown + archived ≥ 2 minor back | ✓ multi-locale builds |
| **Manifest schema reference (human-rendered)** | Rendering of the JSON Schema with examples; every field documented | ✓ | ✓ comprehensive | ✓ multi-locale |
| **Typed error catalog** (per Paige; see above) | One page per error; CI-enforced metadata | ✓ initial set | ✓ all errors covered | ✓ multi-locale |
| **Pattern cookbook** | Orchestrator pattern, distillation pattern, multi-CLI parallelism, halt-on-AC-ambiguity, plus future patterns | partial | ✓ initial canonical patterns | ✓ community contributions |
| **Migration runbooks** (Path A / Path B + per-source-tool runbooks) | Preconditions / step list / verification / rollback / known failures | sketches | ✓ Path A + Path B fully run-bookable | ✓ per-tool runbooks (LangChain, Cursor, etc.) |
| **Troubleshooting guide** | Symptom → cause → diagnostic command → fix; cross-references typed error catalog | partial | ✓ comprehensive | ✓ multi-locale |
| **Deployment topology guide** | Solo / team / Cortex shapes; how-to flavor (vs architecture §11 reference flavor) | sketches | ✓ comprehensive | ✓ multi-locale |
| **`LOCALES.md`** with glossary lock | Translation contribution flow; terms never translated (`Spirit`, `Worker`, `kernel`, ADR ids, error codes); review process; staleness policy | ✓ | ✓ | ✓ |
| **Doc tooling pipeline** | mdBook + i18n / Docusaurus / VitePress with versioning — pick one | pick + commit | ✓ in production | ✓ |
| **Three-door page** at `docs.maos.dev` | "I want to write a Spirit" / "I want to run MAOS" / "I want to understand MAOS" — reader-task-first navigation | ✓ | ✓ | ✓ |

**Localization v1.0 targets:** Korean (shipped); Japanese (paperclip + Spirit-author overlap); Chinese-simplified (kimi-cli community leverage). Spanish/German/French defer to community pull.

**Doc-quality coverage targets (v1.0):**
- Every public ABI method has ≥ 1 doctested example (CI-enforced).
- Every typed error has cause / recovery / example trigger / example handler (CI-enforced).
- Doc site builds on every kernel PR; broken links + out-of-sync code samples block merge.
- WCAG AA — color contrast, keyboard nav, screen reader on code blocks, alt text on every diagram including Mermaid renderings.

### 30-Minute First Spirit Validation Gate (Paige's gate)

The "Build your first Spirit in 30 minutes" tutorial (`spirit-development-and-sharing.md` §4.1) becomes a v1.0 ship gate, not aspirational copy.

**Validation protocol:**
- 5 Spirit authors, **none of whom are MAOS contributors**
- Fresh machine (no Rust toolchain assumed; install commands part of the tutorial)
- Recorded sessions (with consent) reviewed for friction points
- **Floor: ≤ 45 minutes median, ≤ 90 minutes p95**
- Tutorial revised iteratively until target met
- Re-validation required after every breaking tutorial update (any change touching install, manifest, or first-Spirit code template)

If the floor is not met by v1.0, **the substrate ships with a tightened tutorial scope** (e.g., "Build your first Spirit in 60 minutes") rather than ship the unverified 30-minute claim.

### Onboarding and Governance Artifacts (Mary's gaps)

| Artifact | Phase | Owner |
|---|---|---|
| `RFC_TEMPLATE.md` | v0.5 | Maintainer team |
| `GOVERNANCE.md` (maintainers list, lazy-consensus on RFCs, tiebreak by maintainer vote) | v0.5 | Founder + initial maintainers |
| `CODE_OF_CONDUCT.md` (Contributor Covenant baseline) | v0.5 | Maintainer team |
| `PRIVACY.md` (telemetry schema, retention, jurisdiction, deletion path; GDPR Art. 17 compliance) | v1.0 (before telemetry endpoint ships) | Maintainer team |
| `BREAKING.md` (every breaking change requires an entry with migration steps; CI grep-enforced) | v0.5 | Kernel team |
| `STABILITY.md` (live (kernel, abi, manifest_schema) compatibility matrix; LTS branch policy) | v1.0 | Kernel team |
| `LOCALES.md` (translation contribution flow + glossary lock) | v0.5 | Doc team |

**`maosctl` accessibility (Mary's CLI-tool concern):**
- Respect `NO_COLOR` environment variable
- Respect `TERM=dumb` (no spinners, no Unicode box-drawing)
- `--plain` flag for screen-reader-friendly output
- Target: usable for blind operators in production-adjacent environments
- v0.5 ship gate

### LTS and Deprecation Policy (Mary's commitment numbers)

**LTS commitment (Mary's "pick a number"):**
- 2-year LTS on minor lines starting at v1.0
- Security-only patches after year 1 of LTS
- Two LTS lines maintained concurrently (current + previous)

**Deprecation timeline (Winston's commitment):**
- 2 minor releases of warning before removal
- 1 major release to actually remove
- Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`
- All deprecations entered in `BREAKING.md` (CI grep-enforced)

**Telemetry default and policy (Mary's three answers):**
- **Opt-in default.** Operator must explicitly enable.
- **Schema published** in `PRIVACY.md`: every field, every value type, redaction layer documented and source-published.
- **Storage:** v1.0 in maintainer-controlled aggregator with documented retention (90 days); GDPR Art. 17 deletion path via signed request.

### Implementation Considerations

**Documentation generation.** Rust API docs auto-generated via `cargo doc` for `maos-spirit-abi` and `maos-spirit-sdk` crates, published to versioned URL per Documentation Artifacts table. Manifest schema as versioned JSON Schema (machine-checked).

**Versioning and release.** Kernel SemVer with explicit ABI-stability promises per the triple (kernel_version, abi_version, manifest_schema_version). Spirit ABI machine-checked diff on every PR. Reference Spirits independent SemVer per Spirit. Skills SemVer per skill package. Release cadence: kernel monthly during v0.x; quarterly during v1.x; semi-annually during v2.x stable. LTS policy applies from v1.0.

**Telemetry and feedback loops.** v0.5 ships OpenTelemetry spans for every IAC frame, every capability invocation, every halt. v1.0 adds an opt-in anonymous-telemetry endpoint per `PRIVACY.md`. Feedback mechanism: GitHub issues + RFC process per `RFC_TEMPLATE.md` and `GOVERNANCE.md`.

**Skipped sections per CSV (visual_design / store_compliance).** MAOS has no first-party UI design system — operator surfaces are CLI + ACP-mediated editor banners + Transparency Log JSONL output. MAOS does not distribute through app stores — distribution is OSS package managers, GitHub Releases, and the Spirit registry.

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**Approach: Substrate-MVP** — a hybrid of problem-solving + platform + integrity MVP. The substrate's value proves itself when:
- The kernel boots, loads a Spirit, runs a trivial task end-to-end with audit trail intact (v0.1 foundational MVP)
- A single-Spirit Butler running on the foundational kernel surfaces a useful anticipatory notification (v0.3 anticipatory MVP)
- A single-Spirit Researcher delivers a structured survey with confidence map and open questions in bounded time (v0.5 exploratory MVP)
- The founder can run his BMAD epic loop end-to-end with multi-CLI Workers and reclaim his evening (v0.8 founder-loop wedge MVP)
- A team of 8 can adopt MAOS for daily work without surveillance overtones (v1.0 team MVP)

**Phasing is invariant-preserving:** every phase ships a working subset of invariants; no phase ships a relaxed version of any invariant. v0.1 ships I1–I10 enforced at the foundational kernel layer; v0.3 adds I11/I12/I13 enforcement (digest audit-chain, decision-context, intent provenance) without requiring multi-Spirit; v0.5 adds I14 (halt continuity); v0.8 introduces multi-Spirit invariant interactions at scale.

**Phased delivery (founder-directed restructure of original input documents):** The original architecture §13 phasing had v0.1 = "Bootstrap" with one Architect Spirit driving a coding task. Per founder directive, **v0.1 has been re-purposed as foundational kernel + placeholder Spirit** — proving the kernel's big-picture readiness before committing to specific Spirit ambition. **Butler and Researcher journeys are inserted at v0.3 and v0.5 as simpler single-Spirit value demonstrations** that exercise the kernel's anticipatory and exploratory cognitive surfaces without requiring multi-Spirit IAC, A2A peer mesh, or the full distillation pattern. **The founder's-loop wedge demo (J1) is pushed to v0.8** where the substrate composes the foundational kernel + single-Spirit cognitive primitives + multi-Spirit Orchestrator+Worker coordination + cross-host A2A + full §9.5 distillation pattern in one demo. v1.0 then ships team-readiness; v1.5 ships the diagnostic-architect pair; v2.0 ships WASM Spirit form + Cortex precursor (technical); v2.5 ships ecosystem-adoption (parallelizable from v1.5, decoupled from technical phase).

This restructure addresses three concerns from Step 8 party mode:
- **John's "v0.1 timeline 2x optimistic" concern** — by reducing v0.1 scope to foundational kernel + placeholder Spirit, the v0.1 milestone becomes shippable in ~6–8 weeks (Murat's testable threshold), not the 10–12-week aspirational claim.
- **Mary's "operational invariants silently de-scoped" concern** — by adding intermediate v0.3 / v0.5 phases, operational artifacts (SECURITY.md, GOVERNANCE.md, RFC_TEMPLATE.md, doctest CI gate, clean uninstall, pre-write secret-redaction) have natural homes in early phases rather than being deferred to v1.0.
- **Winston's "failure-semantics floor" concern** — by deferring multi-Spirit IAC/A2A to v0.8, the failure-semantics floor (ADR-022) ships at v0.8 alongside the multi-Spirit primitives that need it, not retrofitted into a v0.1 with insufficient distributed-system surface to test against.

**Resource requirements (per phase):**

| Phase | Effort | Team | Key skills added |
|---|---|---|---|
| v0.1 (foundational) | ~6–8 weeks | 1 founder | Rust + Tokio, kernel design, capability systems, Spirit ABI specification, single-Spirit subprocess |
| v0.3 (Butler) | ~10–12 weeks total | 1 founder + advisor council formed | Active Inference / POMDP cognitive modeling, MCP integrations (Calendar/Slack/Linear/Figma), `on_idle` lifecycle hook, narrow Telemetry Stream subscription |
| v0.5 (Researcher + Observer + foundational hardening) | ~14–16 weeks total | 2 implementers (founder + contributor #2) + 3-person advisor council | Broad MCP capabilities, parallelism in tool dispatch, output_shape predicate enforcement, T2/T3 sandbox, distillation pattern (single-Spirit opt-in), v0.5 onboarding artifacts |
| v0.8 (Founder loop wedge demo) | ~22–24 weeks total | 2–3 implementers + community contributors | Multi-Spirit IAC bus, A2A loopback-only, Orchestrator+Worker coordination, ADR-022 failure-semantics floor, full distillation pattern, multi-CLI Worker parallelism |
| v1.0 (Team-ready) | ~30–32 weeks total | 3–4 implementers + ≥1 community contributor + DevRel | Spirit registry, Ed25519 signing, ACP server, vetting attestations, ComplianceClaim, HSIS/CCAC/manifest-fuzz/wire-fuzz gates, 30-min validation gate, typed error catalog, 8 doc artifacts |
| v1.5 (Diagnostic-architect pair) | ~38–40 weeks total | 4–5 implementers + ≥3 community contributors | Postgres + pgvector for Loom-lite, MAOS-mediated provider proxies, mobile push, JetBrains plugin, asymmetric postures |
| v2.0 (technical) | 18-month total | 5+ implementers + active community + ≥1 ecosystem partner | WASM Component Model, PDP integration, multi-region Loom coordination, FKCS protocol |
| v2.5 (ecosystem-adoption, parallelizable from v1.5) | 24–30 months | + DevRel + BD function | Certification body engagement, cohort interop, Cortex consortium recruitment |

### Phase v0.1 — Foundational Kernel + Placeholder Spirit (~6–8 weeks)

**Validation milestone:** Kernel boots; loads a placeholder `hello-spirit` (subprocess form, single Spirit instance); receives a trivial `task.assign` IAC frame from the user via `maosctl`; returns a response; clean install + clean uninstall both work. Audit trail captures every step. **No founder-loop ambition; no multi-Spirit; no A2A.** Foundational proof-of-life demonstrating the kernel's big-picture readiness.

**Core User Journeys Supported (v0.1):** J0 evaluator (5-minute install + first Spirit + clean uninstall).

**Must-Have Capabilities:**
- **Kernel skeleton:** Scheduler + Memory + Capability Registry + IAC mailbox (basic — single-Spirit routing, no cross-Spirit fan-out; multi-Spirit IAC bus deferred to v0.8)
- **Spirit ABI v0.1** — `Spirit` trait + `KernelHandle` trait + manifest schema (full schema per architecture §5.1); machine-checked frozen via CI ABI-diff
- **Single-Spirit subprocess form** (Spirit Wire Protocol over JSON-RPC; one Spirit instance per kernel at v0.1; multi-Spirit deferred to v0.8)
- **Local SQLite persistence**
- **Sandbox tiers T0/T1** (T2 deferred to v0.3; T3 deferred to v0.5; pulled-forward attack surface eliminated)
- **Anthropic provider driver** (single-provider; multi-provider deferred to v0.5)
- **Transparency Log** (I2 — log-before-deliver) operational; basic queryable via `maosctl audit query`
- **Capability tokens** (I1 — every external call mediated)
- **Lifecycle journaling** (I10) operational
- **One placeholder Spirit** (`hello-spirit`) — minimal reference Spirit demonstrating the ABI; not a working agent, just a proof of life
- **`maosctl` basic** — `install`, `uninstall`, `audit query`, `spirit invoke`
- **`maosctl` accessibility:** respect `NO_COLOR`, respect `TERM=dumb`, `--plain` flag for screen-reader-friendly output (Mary's gap closed at v0.1)
- **Clean uninstall** (J0 evaluator requirement; Mary's hill-to-die-on item #2): `cargo uninstall maos` removes capability-token caches, sandbox tmpfs mounts, ACP socket files, no orphaned state
- **`SECURITY.md`** with disclosure address (`security@maos.dev`), embargo window (90-day default), GPG key, advisory format committed (Mary's hill-to-die-on item #1)
- **Doctest CI gate** — every public ABI method has ≥ 1 doctested example; CI broken-link blocking on the doc site (Mary's gap closed at v0.1)

**Numeric ship-gate floors (v0.1 — Murat's audit applied):**
- Kernel ≤ 10 KLOC excluding tests (lower than original 15 KLOC because v0.1 surface is reduced)
- Capability-registry fuzz coverage ≥ 80% line / ≥ 60% branch with zero crashes on 1M-iteration libFuzzer run
- Zero `unsafe` blocks in capability-validation path
- ABI compatibility matrix within current major: 100% (every minor pair, both directions)
- Manifest field test coverage: ≥ 3 cases per field
- Manifest self-check 100% (declared output_shape matches produced)
- Spirit-test fixture-corpus pass rate ≥ 98%
- **Hard cap solo phase at 8 calendar weeks regardless of scope completion** (John's burnout-mitigation rule); if not at v0.1 by then, de-scope further

**Out of v0.1 scope (deferred, not de-scoped):**
- Butler Spirit (v0.3)
- Researcher Spirit + Observer (v0.5)
- Multi-Spirit IAC bus (v0.8)
- A2A peer mesh (v0.8 loopback-only; v1.0 cross-host)
- Distillation pattern enforcement (v0.5 single-Spirit opt-in; v0.8 multi-Spirit deployment)
- ComplianceClaim (v1.0 first-class object; CCAC ship-gate at v1.0)
- Hot-swap / migration policy (ADR-020) — kernel ships I6 mechanism but no in-flight hot-swap testing until v0.3 has ≥ 1 working Spirit
- Spirit registry over MCP-Streamable-HTTP (v0.5 basic; v1.0 full)
- All other multi-Spirit primitives, ACP server, WASM, Loom

### Phase v0.3 — Butler Spirit (~10–12 weeks total)

**Validation milestone:** J-Butler journey reproducible — Sandra's 7 PM scene runs end-to-end with the Butler Spirit, including the self-tuning halt three weeks later. Butler eval metrics published in registry: notification precision ≥ 0.85, notification recall ≥ 0.7, halt-precision ≥ 0.85.

**Adds to v0.1:**
- **Butler Spirit** — first reference Spirit with full cognitive surface (anticipatory reasoning per design report ¶83–¶85, ¶154–¶168)
- **`on_idle` lifecycle hook** as substrate for anticipatory reasoning (Active Inference / POMDP shape)
- **Telemetry Stream** (I7) operational; basic narrow per-Spirit subscription (Butler subscribes to Calendar/Slack/Figma topics)
- **`[epistemic_policy]` per-tag rules** with `verbalize_with_options` / `verbalize_only` / `flag` / `halt` actions and confidence + impact thresholds
- **Output_shape predicate** enforcement at the Capability Registry (kernel rejects Spirit emit on missing fields)
- **MCP tool integrations**: Google Calendar (read), Slack (read + draft-but-don't-send), Linear (write — gated by approval), Figma (read)
- **Posture-shift command** — runtime supervision knob; kernel logs the shift; subsequent capability requests prompt that wouldn't have
- **Sandbox tier T2** (Landlock+seccomp on Linux narrow scope for Butler's MCP-only capability surface)
- **Episodic memory** (private tier) for Butler's `notification_acceptance_log` — feeds POMDP refinement across sessions
- **Hot-swap mechanism** functional (ADR-017 wire format); first real hot-swap test happens here when Butler v0.3.1 → v0.3.2
- **Contributor #2 onboarding** — Spirit-author-facing documentation pulled forward from v1.0 to v0.3 per John's recruit-readiness gate ("Spirit ABI is stable enough that a second contributor can write a reference Spirit in <1 week without founder pairing")
- **Advisor council** formed (3-person, advisory not voting; Mary's mitigation against solo-founder slippage)
- **`RFC_TEMPLATE.md`** drafted (Mary's gap; pulled forward from v0.5 to v0.3)
- **`GOVERNANCE.md`** drafted (lazy consensus on RFCs, tiebreak by maintainer vote; pulled forward to v0.3)

**Numeric ship-gate floors (v0.3 — Murat-style):**
- All v0.1 floors maintained
- **Butler-specific behavior corpus**: 30-scenario calendar/comms; halt-recall ≥ 0.90 on calendar-conflict subset (15 scenarios); halt-precision ≥ 0.85 overall
- **Notification precision ≥ 0.85** (% acted on); **notification recall ≥ 0.7** (% of relevant moments caught)
- **Self-tuning halt corpus**: 10 synthetic acceptance-rate-decline scenarios; Butler halts in ≥ 9/10 within 14-day rolling window
- **MCP integration tests**: 4 MCP servers × 5 representative ops × 3 outcomes (success / scope-violation / network-error) = 60 tests, 100% pass

### Phase v0.5 — Researcher + Observer + foundational hardening (~14–16 weeks total)

**Validation milestone:** J-Researcher journey reproducible — Hannah's 2-hour LLM-judge survey delivers structured findings + Open Questions + Confidence Map + Bibliography in ≤ 90 minutes; Researcher halts on contradictory findings; output passes output_shape predicate. Plus: Observer Spirit subscribes to multi-Spirit telemetry stream (read-only, no IAC writes) showing live activity for the Butler + Researcher pair.

**Adds to v0.3:**
- **Researcher Spirit** — second cognitive Spirit; survey-mode (exploratory, reactive, divergent per design report ¶87–¶89)
- **Observer Spirit** — third reference Spirit; passive activity-stream subscriber; first multi-Spirit-aware Spirit. **Important architectural distinction:** Observer subscribes to the kernel's **Telemetry Stream** (broadcast topic-based, operational at v0.3 narrow / v0.5 broad), NOT the **IAC Bus** (directed mailbox-based, deferred to v0.8). Telemetry Stream is a separate kernel service from IAC Bus per architecture §4.5 / §4.7. Observer's read-only broadcast subscription does not require the multi-Spirit IAC Bus primitives; the "no IAC bus until v0.8" claim holds. Subscribes broadly per design report ¶842.
- **Broad MCP capabilities** for Researcher: web.search, arxiv.search, github.search, citation_graph.traverse
- **Parallelism in tool dispatch** — manifest `[capabilities.parallelism]` declared; v0.5 cap of 8 concurrent dispatches
- **`hypothesize-mode` posture** declared in manifest (full ILP+LLM hybrid implementation deferred to v1.0; v0.5 ships the mode declaration + survey-mode-only operation)
- **Adaptive-chunk-ratio summarization** (openclaw pattern; default for Researcher)
- **Sandbox tier T3** (containerized — Docker/Podman) for Researcher's broader capability surface
- **Per-Spirit resource isolation via cgroups v2** (Linux) — Winston's gap closed at v0.5; sandbox = "you can't escape", resource governance = "you can't starve peers"
- **Distillation pattern (single-Spirit opt-in)** — Researcher's first opt-in deployment of §9.5; bounded context for very-large-corpus surveys (200+ candidate papers compressed to bibliography); five-metric gate applies (digest-recall ≥ 0.90, faithfulness ≥ 0.98, hedge-preservation ≥ 0.95, traceability = 100%, secret-leakage = 0%)
- **`log.recall` capability** (ADR-013) — kernel-mediated participant-scoped Transparency Log retrieval
- **I11 audit-chain enforcement** on memory writes (digest_audit_chain.rs in capability_registry/)
- **I12 decision-context recording** on `decision.*` frame emit
- **I13 intent_lineage propagation** in distillation pipelines
- **Pre-write secret-redaction filter** at the Transparency Log boundary (Mary's §9.5 metric #5 commitment; v0.5 deployment)
- **Multi-provider LLM drivers** tested in CI (≥ 3 providers: Anthropic + OpenAI + local-LLM via Ollama)
- **Spirit registry basic** (MCP-Streamable-HTTP server, Ed25519 signing, install-time signature verification)
- **`maosctl audit query` family** — `audit query`, `audit retract-history`, `audit policy-violations`
- **Approval Manager prompt UX** (synchronous user-facing surface)
- **Transparency Log persistence** with 90-day retention default (Step 5 commitment)
- **ACP server** (editor bridge — Zed + VSCode tested)
- **Onboarding artifacts at v0.5**: `CODE_OF_CONDUCT.md`, `BREAKING.md`, `LOCALES.md`, `TRADEMARK.md`, `PRIVACY.md` (before any telemetry endpoint), namespace grammar ADR locked
- **Sustainability vehicle** committed (Open Collective minimum; Mary's gap closed at v0.5)
- **Synthetic governance dry-run** — two maintainers walk a contrived RFC end-to-end, publish decision log as canonical example (Mary's gap closed at v0.5)
- **Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class** on `bmad-eval` standard corpus, published in registry

**Numeric ship-gate floors (v0.5):**
- All v0.3 floors maintained
- **Researcher-specific corpus**: 25-scenario `bmad-technical-research`; ≥ 3 sources cited with ≥ 80% reachable URLs; halt fires on ≥ 9/10 scope-creep injections; output_shape predicate satisfied 100%
- **Observer-specific corpus**: 20-scenario activity-stream; **missed-event rate ≤ 2%**, **causal-ordering correctness ≥ 99%** (read-only Spirit; halt-recall is wrong metric per Murat)
- **Distillation five-metric gate** (single-Spirit Researcher context): all floors per §9.5 + 10⁵ secret-leakage corpus
- **Multi-CLI partial parallelism**: 2 Workers (claude-code + opencode) on a coordinated task without context bleed (≤ 1/20 leak detected on 20-story corpus); preview of v0.8 multi-Spirit pattern
- **Halt-recall ≥ 0.7 + halt-precision ≥ 0.85** per Spirit class

### Phase v0.8 — Founder Loop Wedge Demo (~22–24 weeks total)

**Validation milestone:** J1 Founder's Loop reproducible — Lunarpulse runs Epic-7 BMAD loop end-to-end with the Orchestrator + multi-CLI Worker pattern, halts on AC ambiguity at 6:23 PM, closes laptop at 8:40 PM, wakes to a completed digest at 6 AM. The wedge demo shippable. **This is the substrate's "moment of full ambition observable in one demo."**

**Adds to v0.5:**
- **Multi-Spirit IAC bus** — kernel-internal routing for Spirit↔Spirit communication on the same Host (per architecture §4.5)
- **A2A peer mesh** — **loopback-only profile at v0.8** (Winston's bounded attack surface): `127.0.0.1`-bound, mTLS with self-signed certs, TOFU pinning, no cross-host traffic. Cross-host A2A deferred to v1.0.
- **Two-level `task.assign` typed-intent IAC primitive** (ADR-013): human → Orchestrator at epic granularity; Orchestrator → Worker at story granularity
- **ADR-012 typed-intent consent** on A2A frames (kernel rejects frames whose declared intent is absent from receiver's spawn-time consent policy)
- **ADR-016 token-budget accounting** — `ContextPressure` / `ContextLimit` / `EContextExhausted` typed frames
- **ADR-017 hot-swap state-transfer wire format** — CBOR + per-class versioned schema
- **ADR-020 hot-swap migration policy** — `migrates_from` manifest field + `EMigratorMissing` enforcement
- **ADR-021 CliWrapperSpirit output-shape adapter contract** — `output_shape_version` declaration + fail-loud rule
- **ADR-022 failure-semantics floor (NEW)** — Winston's non-negotiable v0.3-architecture-locked, v0.8-implemented commitment:
  1. Crash detection SLO ≤ 2s
  2. In-flight `task.assign` NACK with `TaskOrphaned` IAC frame ≤ 5s
  3. No auto-respawn at v0.8 (deferred to v1.0); kernel says "it's dead" fast and reliably
  4. Journaled crash transition with exit-cause (signal, exit-code, stderr-tail)
- **Orchestrator Spirit** — fourth reference Spirit; `class = "orchestrator-bmad"` per Step 4 J1; orchestrator persona skill + `maos-bridge` skill loaded into a Claude Code process
- **Developer-Worker + Reviewer-Worker** — fifth/sixth reference Spirits as skill packages; Worker Spirits are agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded
- **Full distillation pattern (§9.5)** — multi-Spirit deployment; Orchestrator's distillate-then-decide pattern operational
- **Multi-CLI Worker parallelism** — 3+ Workers (claude-code + opencode + gemini-cli) on the same epic
- **Halt-recall and halt-precision benchmarks per Spirit class** published in registry — uniform across Spirits + per-Spirit-class corpora
- **Numeric ship-gate floors:**
  - **A2A loopback v0.8 floors** (Murat's pull-forward testability): mTLS replay corpus 100/0; TOFU pin-mismatch 100/100 detected/rejected/logged; handshake-fault 20/0
  - **Multi-Spirit IAC ADR-012 floors**: cross-Spirit consent corpus 30 scenarios (100% disallowed blocked, ≥ 95% allowed succeed, 0 envelope-type confusion); revocation propagation ≥ 29/30
  - **Orchestrator Epic-corpus**: 5 epics, ≥ 4 complete without halt-storm; halt-precision ≥ 0.85
  - **Planted-issue corpus**: 50 synthetic stories; Orchestrator surfaces ≥ 42/50 (84%); ≥ 45/50 on security-relevant subset
  - **Ambiguous-AC corpus**: 10 stories; halt fires in ≥ 9/10 within 2 frames

### Phase v1.0 — Team-Ready, Third-Party Spirits Ship (~30–32 weeks total)

**Validation milestone:** J3 Team Nexus (8-Host team peer mesh) reproducible end-to-end on a real team in parallel with a 30-day synthetic-shadow run (zero substrate-invariant violations, zero unauthorized cross-Spirit data flow, halt-recall preserved within ±0.03 of v0.8 baseline). J6 Diego validated via black-box external-author trial (5 authors, 14-day no-DM-support window, ≥ 4/5 succeed). First cohort interop demonstration (any of openclaw / ironclaw / hermes / paperclip / rustain / codex).

**Adds to v0.8:**
- **Architect (Nash-class) + Reviewer reference Spirits** added; **6 reference Spirits total** (Butler / Researcher / Architect / Worker / Reviewer / Observer)
- **Cross-host A2A peer mesh** with full mTLS + TOFU + ADR-012 typed-intent consent (lifted from loopback-only)
- **Spirit registry v1.0 (full)**: `registry.search` / `manifest` / `artifact` / `publish` / `deprecate`; four trust tiers operational; strictest-of-(manifest, tier) enforcement
- **Sandbox tier T4 WASM (tools, not Spirits yet)**
- **ComplianceClaim envelope** + admission-time verification (first-class kernel object)
- **Vetter trust model documented**: accreditation, revocation semantics, conflict resolution
- **Audit sealed-export** (signed JSONL — regulator-ready)
- **Numeric v1.0 ship gates** (full set):
  - Spirit-test class regression-corpus pass ≥ 95%
  - Cross-form Semantic equivalence (rust-inproc ↔ subprocess) ≥ 90% on 200-scenario class corpus
  - ABI N-1 major boundary 100% incl. negative typed-error cases
  - Manifest parser fuzz: 24h cargo-fuzz, zero crashes (P0 ship gate)
  - Wire protocol cross-language byte-equal golden corpus (Rust + TS + Python SDKs)
  - Wire protocol schema-evolution: 4 cases per frame variant
  - Wire protocol adversarial-input fuzz: 24h, zero crashes
  - **HSIS** ≥ 95% pass rate per Spirit class, zero invariant violations (6 class-specific corpora)
  - Capability-token TOCTOU test: 100% re-validation
  - Adversarial-Spirit red-team corpus: ≥ 48/50
  - Intent-provenance test: 40/40 detected
  - **CCAC**: N=500, FPR ≤ 0.5%, TPR ≥ 98%, per-class ≥ 18/20, context-drift 100% rejected
  - Secret-leakage 0% on 10⁵-case corpus + production canary system (≤ 24h p95)
- **30-Min First Spirit Validation Gate**: **N=12 stratified** (Murat's correction; not N=5), median ≤ 45 min, p95 ≤ 90 min, ≥ 10/12 succeed
- **Black-box third-party trial** (Murat's silent-failure-catcher): 5 external authors via public CFP; 14-day no-DM-support window; ≥ 4/5 produce working signed Spirit binary that loads on fresh Host VM, runs ≥ 1000 frames, halt-recall ≥ 0.85 on class-appropriate subset; auditable via SBOM + signing chain re-loaded on clean VM by CI bot
- **Documentation artifacts** (Paige's full set):
  - API reference at `https://docs.maos.dev/abi/v1.0/` (versioned, searchable, archived ≥ 2 minor back)
  - Manifest schema reference (human-rendered)
  - **Typed error catalog** at `https://docs.maos.dev/errors/<ERR_NAME>` (CI-enforced metadata per error variant)
  - Migration runbooks (Path A + Path B fully run-bookable)
  - Troubleshooting guide
  - Deployment topology guide
  - Three-door page at `docs.maos.dev`
  - WCAG AA compliance
- **Localization**: Korean (shipped); Japanese + Chinese-simplified (v1.0 targets)
- **Substrate Operations**: full lifecycle (`maosctl install/upgrade/yank/uninstall/revoke`); signed Revocation List (CRL) artifact; offline import path; auto-respawn-with-backoff for Spirits declaring `restart: on-failure`
- **LTS commitment announced**: 2-year LTS on minor lines from v1.0; security-only patches after year 1; two LTS lines maintained concurrently
- **`STABILITY.md`** with live (kernel, abi, manifest_schema) compatibility matrix and substrate-self compliance scope clause
- **External pen-test report** with zero P0/P1 findings open at ship
- **First cohort interop demonstration**

### Phase v1.5 — Diagnostic-Architect Pair (~38–40 weeks total cumulative)

**Validation milestone:** J4 Mira-Nash 90-min loop reproducible on 50-scenario synthetic prod-incident corpus; ≥ 45/50 close in ≤ 90 min; ≥ 48/50 uphold typed-intent consent envelope.

**Adds to v1.0:**
- **Diagnostic Engineer Spirit class (Mira)** with full asymmetric capability gates
- **Per-tag epistemic policy** at production fidelity (`diagnosis.root_cause` halts at confidence_below=0.6 or evidence conflict; `diagnosis.observation` is verbalize_only; `containment.action` halts at confidence_below=0.5)
- **Post-deploy feedback IAC topic**; Architect-class Spirits subscribe to Diagnostic-class post-deploy validation results
- **Loom-lite**: single-instance Postgres-backed pattern library, exposed as MCP-Streamable-HTTP server
- **`maos-persistence` Postgres support**
- **MAOS-mediated provider proxies** (intercept HTTP calls to LLM providers for substrate-layer audit)
- **Asymmetric postures** (sre-diagnostician vs principal-architect)
- **Mobile-friendly approval surface** (HTTP push notifications)
- **Five-metric gate passes on all distillation-shipping reference Spirits**
- **JetBrains plugin-bridge** for ACP integration
- **Skill ecosystem**: BMAD framework first-class supported (full skill set tested in CI)
- **mTLS cert rotation chaos test** passes
- **Revocation latency**: median ≤ 60s, p99 ≤ 5min from CA revoke to peer rejection
- **ADR-023 capability-token TTL + bind-to-PID** (Winston's Risk 8 mitigation)

### Phase v2.0 (technical) — WASM Spirits + Cortex Precursor (18-month total)

**Validation milestone:** Cortex 3-region pilot at small scale (≥ 10 agents) with technical validation (NOT commercial-adoption gating); FKCS protocol passes (3 case-studied future Spirits implemented by external authors against published ABI documentation only; ≥ 27/30 success per Spirit, ≥ 85/90 aggregate; diff oracle confirms zero kernel changes).

**Adds to v1.5:**
- **WASM-component Spirit form** — third-party ecosystem capability-isolated by construction; single portable artifact; WIT contract `maos:spirit@1.0`
- **Spirit registry v2.0**: vetting attestations; community-vetting authorities; OSS-style RFC process for Spirit ABI extensions; OCI-compatibility evaluation
- **Enterprise Spirit class** with PDP (Policy Decision Point) integration (OPA / Cedar / Vault); SSO/OIDC identity assertions; encrypted-at-rest memory with org KMS; SIEM telemetry export
- **Multi-instance Loom** with cross-region replication; consensus on cross-incident pattern propagation
- **Sentinel-validated canary auto-rollback**; pre-deployment scanning against pattern library
- **Native push notifications** (mobile)
- **Optional skill registry** (separate from Spirit registry)
- **Cortex churn test** passes: 100-host Cortex, 10% host turnover/week for 4 weeks, 3 planted adversarial hosts; detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h
- **Cross-form Semantic equivalence** (any-rust ↔ wasm-component) ≥ 75%
- **ADR-024 sandbox-escape detection** (Winston's Risk 9 mitigation)
- **`loom-threat-model.md`** drafted (deferred from Step 5)

### Phase v2.5 (ecosystem-adoption) — Parallelizable from v1.5 (24–30 months)

**Validation milestone:** Cortex consortium target case study published (per v0.3 lock); first auditor or regulator references a MAOS Transparency Log frame or ComplianceClaim in a published finding (trust-anchor frame validation per Step 6 carry-forward).

**Adds (parallel workstream from v1.5; staffed by DevRel + BD function, not engineering):**
- **First third-party ComplianceClaim** issued against a reference Spirit by an accredited assessor (anecdotal milestone; pilot partner)
- **Public registry of ComplianceClaims**; ≥ 3 certification bodies issuing claims
- **Adoption signal**: ≥ 20 external Spirits in registry; ≥ 3 protocol citations from independent agent projects; ≥ 1 cohort project formally citing MAOS as substrate or interop reference
- **Multi-locale doc site** (Korean + Japanese + Chinese-simplified shipped; community-contributed others tracked)
- **Cortex consortium target case study** published (consortium target locked at v0.3 — Reza-class single-org cross-team is leading candidate per Step 6 / John's Step 8 confirmation)

**Critical decoupling rationale (per John):** the technical phase (v2.0) cannot gate on third-party adoption. ≥ 20 external Spirits, ≥ 3 cohort citations, ≥ 3 certification bodies are *partnership and recruitment* metrics that depend on a DevRel/BD function the engineering team doesn't have. Bundling them into a single 18-month v2.0 means engineering sits idle waiting for cert bodies to sign MOUs. v2.5 ecosystem-adoption is parallelizable from v1.5 onward; technical v2.0 ships when technical is ready.

### Risk Mitigation Strategy

**Technical Risks:**

| Risk | Mitigation | Phase |
|---|---|---|
| Kernel performance bottleneck under fan-in pressure (28-Spirit Cortex) | Per-Spirit fairness scheduler in front of log writer (not FIFO); v0.5 perf gates; v1.0 absolute SLOs in Step 10 | v0.5–v1.0 |
| ABI stability erosion | Machine-checked ABI-diff CI on every PR; STABILITY.md live matrix; deprecation 2-minor-warning + 1-major-remove | v0.1+ |
| Hot-swap silent state corruption | HSIS ship gate (≥ 95% pass, zero invariant violations); ADR-017 typed wire format; ADR-020 migration policy | v1.0 |
| CLI-wrapper output-shape drift | ADR-021 fail-loud rule; `output_shape_version` declared; per-CLI adapter versioned | v0.8+ |
| Provider rate-limit cascading | Per-(provider, credential) token bucket; typed `RateLimited` IAC frame | v0.5+ |
| Distillation legibility-attack | Five-metric ship gate; canary system; cross-LLM judge sampling; CI-enforced I11/I12 | v0.5–v1.0 |
| Loom poison-pattern (Cortex-scale) | Provenance-chain audit; explicit `loom-threat-model.md` | v1.5–v2.0 |
| **Risk 8: Capability-token theft / TOCTOU** (Winston's add) | Short TTL (≤ 60s for high-privilege); bind tokens to Spirit-PID + boot-nonce; audit-log every check with origin-Spirit-ID; ADR-023 | **v1.5** |
| **Risk 9: Sandbox-escape detection vs containment** (Winston's add) | Anomaly detector on top of Landlock/seccomp (syscall-pattern divergence, fd-table growth, unexpected outbound IAC); ADR-024 | **v2.0** |
| **Hot-swap rollback** (sub-policy of ADR-020) | If successor health-check fails within N seconds of cutover, kernel auto-reverts to predecessor and emits `HotSwapAborted` | v1.0 |
| **A2A trust establishment under churn** (Winston's #1 production risk) | "Spirit-restart invalidates prior pins" rule + re-pin protocol; ADR-022 failure-semantics floor closes the underlying gap | v0.8+ |

**Market Risks:**

| Risk | Mitigation | Phase |
|---|---|---|
| Vendor-monolithic competitors close hook/MCP surface (kills CLI-wrapper Spirit form) | De-risk by ensuring MAOS works on at least one fully-open CLI agent (codex-class) before betting on closed CLIs; per-CLI native Rust wrapper crate as v2.0 fallback | v0.8+ |
| Substrate-too-early failure | Trust-anchor framing as alternative competitive frame (carried forward from Step 6 to v0.3 lock); 2026 H2 auditor-citation signal as early validation; OSS-substrate validation via cohort interop at v1.0 | v0.3 lock; v1.0 / 2026 H2 signals |
| ComplianceClaim adoption slow | CCAC ship-gate at v1.0 makes envelope testable without partner dependency; pilot partner is v2.5 anecdotal-but-not-blocking | v1.0 / v2.5 |
| Cortex consortium target undecided | Final lock by v0.3 (Reza-class single-org cross-team is leading candidate) | v0.3 lock |
| Cohort projects don't standardize on MAOS contracts | Constitutional substrate evolution governance; ACP/MCP/A2A interop maintained as protocol commitments; first cohort interop is v1.0 success criterion | v1.0 |

**Resource Risks:**

| Risk | Mitigation | Phase |
|---|---|---|
| **Founder burnout in solo phase** (John's #1 timeline-killer; Mary's #7 silent-slip risk) | Hard cap solo phase at 8 calendar weeks for v0.1; **contributor #2 by v0.3 (not v0.5)**; advisor council formed at v0.3 (3-person); 30% schedule reserve declared upfront | v0.1–v0.3 |
| v0.1 timeline overrun due to scope creep | v0.1 explicitly de-scoped to foundational-only (placeholder Spirit, no multi-Spirit, no A2A); all pull-forwards moved out to v0.8 where they belong | v0.1 |
| Skill ecosystem doesn't grow | Lead by example: BMAD skills as canonical reference; skill-package authoring guide in spirit-development-and-sharing.md §13 | v1.0+ |
| Community vetting bottleneck | Initial vetting authority is the founder + initial maintainers; vetter trust model documented at v1.0; community vetting authorities accredited starting v2.5 | v1.0–v2.5 |

### Open Questions Resolved by v0.3 (split: Governance Lock + Architecture Lock per Winston)

**v0.3 Governance Lock** (positioning / commercial / OSS-licensing — does not constrain code):
1. **Cortex consortium target** for v2.5 demo — Reza-class single-org cross-team is the leading candidate per John's Step 8 confirmation.
2. **OSS license** — Apache 2.0 (per John's Step 8 analysis: copyleft kills the trust-anchor frame; permissive enables ecosystem adoption; the moat is spec + trademark + ComplianceClaim, not license).
3. **Trust-anchor vs OSS-substrate competitive frame** — both frames consistent with architecture through v0.3; lock is positioning. Recommendation: lead with OSS-substrate framing, route trust-anchor framing to ComplianceClaim narrative within it.

**v0.3 Architecture Lock** (constrains kernel + ABI + Spirit lifecycle — Winston's separation):
4. **Failure-semantics floor (ADR-022)** — Winston's non-negotiable v0.3 architecture lock; v0.8 implementation. Four-point minimum: crash detection ≤ 2s; `task.assign` NACK with `TaskOrphaned` ≤ 5s; no auto-respawn at v0.8; journaled crash transition with exit-cause.

**v0.3 lock CI script** (Murat's mechanical checklist): `scripts/check_v0_3_lock.sh` runs four checks; no v0.3 tag without script in green. Checks: LICENSE matches ADR string; consortium-target ADR exists with status `accepted` and ≥ 2 maintainer sign-offs; ROADMAP.md has trust-anchor decision section with status `decided` linking to ADR; failure-semantics doc exists with at least one fully-specified route (no `TBD`).

**v0.5 Revisit Window** (Mary's concern — license decision needs more community feedback than v0.3 allows):
- v0.3 publishes a *defaults document* with explicit "revisitable until v0.5" clause for license and consortium target.
- After v0.5, the locks become final and removal would be a major-version event.

### Substrate Operations Checklist (full)

| Concern | Owner | Target version | Artifact |
|---|---|---|---|
| Install / upgrade / clean uninstall UX | Core team | **v0.1** | `maosctl install`, `maosctl upgrade`, `maosctl uninstall` (clean) + tests |
| `SECURITY.md` + disclosure pipeline + GPG key | Maintainer team | **v0.1** | `SECURITY.md` + `security@maos.dev` + advisory format committed (Mary's hill-to-die-on) |
| Doctest CI gate | Kernel team | **v0.1** | Every public ABI method has ≥ 1 doctested example; CI broken-link blocking |
| `maosctl` accessibility | Core team | **v0.1** | NO_COLOR / TERM=dumb / `--plain` flag |
| Yank notification | Core team | v0.5 | Registry polling + operator notification |
| Pre-write secret-redaction filter | Core team | **v0.5** | Mary's §9.5 metric #5 deployment |
| Synthetic governance dry-run | Maintainer team | **v0.5** | Two-maintainer contrived-RFC walkthrough; published decision log |
| `RFC_TEMPLATE.md` | Maintainer team | v0.3 (drafted) → v0.5 (locked) | RFC template + governance procedure |
| `GOVERNANCE.md` | Founder + initial maintainers | v0.3 (drafted) → v0.5 (locked) | Maintainers list, lazy-consensus on RFCs, tiebreak by maintainer vote |
| `CODE_OF_CONDUCT.md` | Maintainer team | v0.5 | Contributor Covenant baseline |
| `BREAKING.md` | Kernel team | v0.5 | Every breaking change requires entry; CI grep-enforced |
| `LOCALES.md` | Doc team | v0.5 | Translation contribution flow + glossary lock |
| `TRADEMARK.md` | Maintainer team | v0.5 | Brand policy (Mary's gap closed at v0.5) |
| Sustainability vehicle | Maintainer team | v0.5 | Open Collective minimum (Mary's gap closed) |
| Namespace grammar ADR | Architecture | v0.5 | Flat vs scoped Spirit names (Mary's gap closed) |
| Advisor council formed | Founder | v0.3 | 3 people, advisory not voting |
| Contributor #2 onboarded | Founder + recruit | v0.3 | "Spirit ABI stable enough that contributor writes a reference Spirit in <1 week" |
| Spirit-author docs pulled forward | Doc team | v0.3 | Documentation artifacts at v0.3-readiness for second-contributor onboarding |
| Spirit registry basic | Core team | v0.5 | MCP-Streamable-HTTP server, Ed25519 signing |
| Audit query / SIEM export | Core team | v0.5 (basic), v1.0 (signed export + multi-region) | `maosctl audit query` family + sealed-export |
| Telemetry opt-out + `PRIVACY.md` | Core team | v0.5 (PRIVACY.md drafted before any endpoint), v1.0 (endpoint ships) | Opt-in default + per-field redaction layer |
| LTS window | Maintainer team | v1.0 announcement | `STABILITY.md` + LTS branch policy |
| Vetter accreditation | Maintainer team | v1.0 documented | Vetter trust model document |
| ComplianceClaim issuance | Issuance Spirit class | v1.0 first-class object; v2.5 multiple issuers | `ComplianceClaim` envelope spec + first issuer pilot |
| Doc tooling pipeline | Doc team | v0.5 (pick), v1.0 (in production) | mdBook + i18n / Docusaurus / VitePress with versioning |
| Three-door doc page | Doc team | v0.5 | `docs.maos.dev` reader-task-first navigation |
| Typed error catalog | Kernel team | v1.0 | Per-error pages with cause / recovery / examples; CI-enforced |
| 30-Min First Spirit Validation Gate | Doc team + community | v1.0 | **N=12 stratified**, ≤ 45 min median, ≤ 90 min p95, ≥ 10/12 succeed |

### Strategic Scope Commitments

- **Phasing is invariant-preserving:** every phase ships a working subset of invariants; no phase relaxes any invariant.
- **No silent de-scoping:** every requirement committed in Steps 4–7 is mapped to a phase. If a requirement is missing from this scoping, it is a documentation bug to be flagged, not a deferral.
- **v0.1 is foundational, not founder-loop:** kernel skeleton + placeholder Spirit + clean install/uninstall + `SECURITY.md` + audit trail. **Founder loop is v0.8.**
- **Butler at v0.3 + Researcher at v0.5 are simpler proof points** that exercise the kernel's anticipatory + exploratory cognitive surfaces without requiring multi-Spirit IAC, A2A, or full distillation. They prove the substrate progressively.
- **Pull-forwards are gone from v0.1.** Subprocess Spirit form, multi-Spirit IAC bus, A2A peer mesh, T2 sandbox, distillation pattern, hot-swap migration all deferred to their natural phases (v0.3 / v0.5 / v0.8). v0.1 timeline becomes ~6–8 weeks (Murat's testable threshold).
- **Reference Spirit count grows progressively:** 1 placeholder at v0.1 → 2 (Butler) at v0.3 → 4 (+ Researcher + Observer) at v0.5 → 7 (+ Orchestrator + 2 Workers) at v0.8 → 9 (+ Architect + Reviewer) at v1.0 → 10 (+ Mira) at v1.5 → 11 (+ Enterprise) at v2.0. Third-party Spirits proliferate via the registry, not by being added to MAOS source.
- **Skills are user-space; never kernel.** Filesystem-based v0.5; optional registry v2.0.
- **v2.0 splits into v2.0 (technical) + v2.5 (ecosystem-adoption).** Technical phase cannot gate on third-party adoption (per John's rule).
- **Halt-recall preference is user-configurable** per Spirit per tag.
- **Kernel-stays-small redefinition:** trusted core ≤ 20 KLOC through v2.0; integration adapters in separate crates with their own budgets; "small" = trusted core, not binary footprint.
- **v0.3 split:** Governance Lock (positioning, license, consortium target, trust-anchor framing) + Architecture Lock (failure-semantics floor as ADR-022). Different review boards, different acceptance criteria.
- **Founder burnout mitigation is non-negotiable:** hard cap solo phase at 8 weeks for v0.1; contributor #2 by v0.3; advisor council formed at v0.3; 30% schedule reserve declared upfront.

## Functional Requirements

> **Capability contract.** This section is binding. Any capability not listed here will NOT exist in MAOS unless explicitly added via amendment. Steps 10 (NFRs) and 11 (Polish) inherit this contract.

### Kernel Non-Goals (preface; mirrors §4.0.7 of architecture-maos.md)

**The kernel's value comes from what it deliberately refuses to do as much as from what it provides.** The 65 FRs below describe what MAOS *delivers*. This Non-Goals subsection — load-bearing for the §4.0.7 principle and the hermes-tenant positioning sentence — names what the kernel *refuses to become*:

- **The kernel does NOT interpret tag semantics.** Tagged scalars (FR27) and tagged frames carry meaning the kernel transports without reading. Variance, entropy, EFE, KL, ensemble disagreement, calibration, similarity, derivatives, statistical tests, contradiction detection — all Spirit-side computations. The kernel does universal arithmetic comparison only.
- **The kernel does NOT author cognitive content.** Distillation, summarization, planning, reasoning, dialectic update, hypothesis generation, posture inference — all Spirit-side. The kernel provides storage, lineage, namespacing, and the Inference Port; cognitive work belongs to actors.
- **The kernel does NOT embed an orchestration policy.** Multi-Spirit coordination patterns (supervisor, peer, market, pipeline) are user-space Spirit patterns, not kernel features. The kernel routes typed-intent IAC frames neutrally; Orchestrator-class Spirits (when present) do the directing.
- **The kernel does NOT write skills, rank skills, or curate skills.** Skills are Spirit-author craft; admission is operator-mediated; the kernel hosts the registry mechanism only (per ADR-006 / ADR-024).
- **The kernel does NOT host Loom-class collective knowledge.** Loom is a user-space Spirit (per ADR-006). Pattern propagation across Cortex deployments is Spirit-side coordination over the kernel's IAC bus.
- **The kernel does NOT own application-layer concerns.** Messaging gateways, UI presentation, narrative digest content, training-data generation — all Spirit-side. The kernel offers extension contracts (e.g., FR54 gateway sub-modules); applications fill the contracts.

These non-goals make the substrate's *restraint* legible. Without them, the FR list reads as an active platform; the kernel's value as a tenant-host (defending the v1.0 positioning sentence) becomes invisible.

---

The 65 functional requirements below are organized into seven capability areas reflecting the substrate's grand-theater architecture. The metaphor: **theater (kernel)** provides invariant infrastructure; **actors (Spirits)** perform; **director (user)** controls the production with autonomy spectrum from hands-off to scene-by-scene. Each FR is a testable capability stated implementation-agnostically; numeric ship-gate floors are promoted into FR text where applicable (per Murat's discipline).

### A. Kernel Substrate Operations (9 FRs)

- **FR1:** User can install MAOS kernel via OS package manager (Homebrew/AUR/deb/rpm), `cargo install`, or signed GitHub Releases binary with mandatory Ed25519 signature verification.
- **FR2:** User can uninstall MAOS kernel cleanly, removing all installed Spirits, capability tokens, sandbox mounts, ACP sockets, and operator caches without leaving orphaned state.
- **FR3:** Operator can configure provider drivers (Anthropic, OpenAI, Gemini, Kimi, local-LLM via Ollama, air-gapped Bedrock) per Spirit, including locking provider endpoints for air-gapped deployment.
- **FR4:** Operator can verify every Spirit's external call (file op, network, exec, provider call, sub-Spirit spawn) was mediated by kernel-issued capability tokens by reading the Transparency Log; verification floor is 100% mediation in any 1000-call sample.
- **FR5:** Operator can configure sandbox tier per Spirit (T0/T1/T2/T3/T4); kernel enforces strictest-of-(manifest, trust-tier, operator-policy) floor. **Spirit cannot exfiltrate data outside its declared capability scope** — sandbox enforcement combined with FR4 capability mediation makes this property mechanically auditable.
- **FR6:** Operator can configure per-Spirit resource caps (CPU, memory, file descriptors) via cgroups v2 on Linux or platform equivalent.
- **FR7:** Operator can disable anonymous telemetry; default is opt-in with published schema and redaction layer.
- **FR47:** Spirit obtains all model inference exclusively via the kernel-provided Inference Port; the kernel routes to the configured provider driver and records the call in the Transparency Log. Spirit binaries do not import vendor LLM SDKs directly. (Closes ADR-005 coverage gap.)
- **FR48:** Operator can configure pluggable cryptographic provider for kernel signature verification, sealed-export encryption, and capability-token signing — enabling FIPS-validated, hardware-backed, or post-quantum implementations without recompiling Spirits. (FIPS / NIAP / export-control readiness.)

### B. Spirit Lifecycle Management (8 FRs)

- **FR8:** Spirit author can declare a Spirit class via manifest (TOML) covering `class`, `capabilities`, `posture`, `output_shape`, `explanation_shape`, `epistemic_policy`, `budget`, `skills`, `hot_swap`, `halt_protocol_compatibility`, `intent_promotion_set`, `migrates_from`, `swap_invariants`, `schedule`, **`min_substrate_version`** (kernel rejects load if its own version is below the declared minimum). Manifest declarations are signed and journaled.
- **FR9:** User can load, start, pause, resume, and unload Spirits at runtime via authenticated control plane (CLI, ACP editor surface, or operator API).
- **FR10:** Operator can hot-swap a Spirit class to a new version preserving in-flight capability tokens and working-memory state per the kernel-enforced migration decision tree (ADR-020). Both Spirit forms (in-process Rust ABI and subprocess) ship with parity on lifecycle and IAC semantics (ADR-002).
- **FR11:** Spirit author can declare cross-major migration via `migrates_from` manifest field and a `migrate(predecessor_state)` entry point; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared.
- **FR12:** Kernel detects Spirit-process crash within 2s and emits `task.orphaned` IAC frames to in-flight task originators within 5s with exit-cause journaled. Floor: ≥99/100 detected within 2s in a SIGKILL crash corpus; ≥99/100 NACKed within 5s. Hung-Spirit detection (alive but no progress IAC for >30s) emits `task.stalled` event; ≥48/50 reclassified within 60s on a hang corpus.
- **FR13:** User or operator can revoke a Spirit at runtime via signed Revocation List artifact; running Spirit instances receive `SpiritRevoked` event and execute their declared revocation policy (terminate-immediately / drain-then-terminate / quarantine).
- **FR49:** Operator can upgrade a Spirit (replace v0.3.1 with v0.3.2) with declared migration policy: hot-swap with state preservation (default), cold-swap with re-init, or migrator-mediated cross-major upgrade. Distinct from FR9 (lifecycle verbs); FR49 covers state-bearing version transitions.
- **FR50:** Spirit author can declare dead-Spirit task disposition policy in manifest (`on_crash.action`); kernel applies the policy to in-flight tasks held by the dead Spirit (NACK / reassign-to-replica / escalate-to-operator). Operational-failure handling distinct from epistemic halt (FR15).

### C. Human-Spirit Interaction — Director's Surface (8 FRs)

- **FR14:** User can assign a task to a Spirit via natural-language `task.assign` IAC frame (terminal shell, ACP editor surface, mobile push) with goal + scope + success criteria + posture preferences.
- **FR15:** User can resolve a Spirit-emitted `epistemic.halt` via three documented resolution pathways — supplying missing context, accepting the halt as final, or authorizing override under operator policy; kernel journals the resolution with full reasoning chain. Halt-recall floor ≥0.7 and halt-precision floor ≥0.85 per Spirit class on the `bmad-eval` standard corpus.
- **FR16:** User can shift Spirit posture at runtime ("be more cautious for the next hour"; "switch to autonomous-with-halt"); the shift is journaled and applied to subsequent capability-scope decisions. **Posture-shift propagation latency: P99 ≤2s, P99.9 ≤5s** in a 1000-shift corpus.
- **FR17:** User can read a per-Spirit morning digest containing: (a) tasks completed in the last 24h with outcome tags, (b) open halts requiring resolution, (c) flagged anomalies with confidence ≥0.6, (d) trust-bar reflecting yesterday's predicate-fire rate. Digest is generated by a Companion-class Spirit (NOT kernel) within 30s of the user's first session of the day, using kernel-provided log-composition primitives. **Hallucination floor: 0 hallucinated tasks tolerated** in any 100-digest corpus, verified against the actual Transparency Log; ≥95/100 digests must include all open halts and cite source log refs for all claimed completions.
- **FR18:** User can audit any Spirit decision retrospectively; every `decision.*` frame carries `working_memory_digest_refs` (I12) so post-hoc audit can reconstruct what the agent reasoned over at decision time.
- **FR19:** User can configure halt-recall vs halt-precision preference per Spirit per tag via a halt-policy schema (extension to ADR-013); kernel parses the preference into the Spirit's runtime epistemic policy thresholds.
- **FR20:** User can buffer multiple instructions to an Orchestrator Spirit (NOT kernel-buffered — Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives); the Orchestrator processes queued instructions at safe sequence points between task completions, never preempting in-flight delegations. **(Phase: v0.8, advanced from v0.5 — required for the founder-loop wedge demo's halt-and-resume-overnight pattern.)**
- **FR51:** Director can instantaneously pause, resume, or shift posture of any Spirit including: (a) interrupting in-flight autonomous actions with bounded-time guarantee (P99 ≤2s), (b) preserving Spirit state across pause/resume without reload, (c) recalling pending Orchestrator-buffered actions per FR20, (d) revoking any active capability token with in-flight operations failing-safe within bounded time. Override is auditable per FR42 with director identity and reason. **Operationalizes the director's autonomy-spectrum control surface that defends the theater/actor/director metaphor.**

### D. Multi-Spirit Coordination (10 FRs)

- **FR21:** Orchestrator Spirit can dispatch `task.assign` IAC frames to Worker Spirits with named skill (e.g., `bmad-dev-story`), scoped capability set, posture preferences, and halt policy. **Orchestrator dispatches subsequent tasks against the distillate of prior Worker output, not the raw output** (closes raw-output context-overflow loophole). **Sustained fan-out floor: 50 concurrent Worker Spirits with task-dispatch latency P99 ≤500ms; 0 dropped tasks under 10 tasks/sec for 1h.**
- **FR22:** Spirits on the same Host can communicate via the kernel-internal IAC bus with mailbox-per-Spirit routing and log-before-deliver guarantee (I2). **Orchestrator-Worker communication uses distillate frames in steady state; raw frames recallable via `log.recall` for ground-truth verification.**
- **FR23a:** (v0.8 loopback) Spirits across Hosts can communicate via A2A peer mesh on `127.0.0.1`-bound endpoints with self-signed mTLS certs and TOFU pinning. Test corpus: mTLS replay 100/0; TOFU pin-mismatch 100/100 detected; handshake-fault 20/0; cross-Spirit consent 30 scenarios with 100% disallowed blocked.
- **FR23b:** (v1.0 full mesh) FR23a extends to cross-host with operator-managed PKI, full mTLS handshake corpus, certificate rotation chaos test (10-host Cortex, zero conversation drops), revocation latency median ≤60s p99 ≤5min, clock-skew tolerance ±5min, partial-partition fail-safe within 10s.
- **FR24:** Spirit can run autonomously under `autonomous-with-halt` posture, halting only when its `[epistemic_policy]` triggers; user can shift to `assistive` (every action prompts) or `cautious` (auto-approve routine, prompt for novel) at runtime. **All cross-Spirit IAC frames carry intent provenance metadata linking each intent to its originating task envelope, preserved across re-emission (per ADR-018 / I13).**
- **FR25:** Worker Spirit can be a wrapped agent CLI process (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded; kernel-builtin CliWrapperSpirit hosts it with declared `output_shape_version` (fail-loud on shape mismatch).
- **FR26:** Spirit can declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist (per ADR-025).
- **FR52:** Spirit can invoke external CLI subprocess (e.g., `claude code`, `opencode`) under capability-token authority; stdout/stderr captured into the Transparency Log with provenance to the invoking Spirit. Tier-3 sandbox profile; explicit manifest declaration required. **(v0.8 wedge-critical — operationalizes Worker Spirit's CLI-shelling pattern.)**
- **FR53:** Active halts associated with a Spirit retain identity, replay context, and resumption guarantees across hot-swap (per ADR-019 / I14); the kernel rejects swaps that would orphan halts unless the Spirit-author has declared `halt_protocol_compatibility = true` for the predecessor's halt schema. Closes I14 coverage gap.
- **FR54:** Spirit author can declare gateway sub-modules in manifest (e.g., Telegram, Slack, Discord, Signal, email) running as long-lived connection holders under the Spirit's principal namespace (FR31); kernel hosts the lifecycle and capability-scope contracts; gateway implementation is Spirit-side. Defends the v1.0 hermes-tenant positioning claim.
- **FR55:** Spirits SHALL be able to register for kernel-emitted lifecycle triggers including `on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate` (Spirit-author-defined cadence for memory-curation passes). Each trigger carries declared resource budgets per manifest. Butler's `on_idle` substrate for anticipatory reasoning is the v0.3 anchor.

### E. Memory, Cognition Substrate, and Distillation (7 FRs)

- **FR27:** Spirit can write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. **The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics** — kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Per §4.0.7: kernel performs no Spirit-specific cognitive computation (no variance, entropy, EFE, KL, ensemble disagreement, derivatives, or statistical tests — Spirit computes those itself).
- **FR28:** Spirit can write to private, shared, or collective memory tiers (per I5); **memory compaction is Spirit-authored — the Spirit's persona logic declares compaction policy; kernel provides persistence and quota enforcement only.**
- **FR29:** Spirit can recall historical Transparency Log frames it was a participant in via `log.recall(filter, limit, cursor)` with payload-on-demand fetch via `log.fetch(frame_id)`; kernel scopes results to participant frames and honors A2A consent envelopes. **Distillation work — selecting which frames to preserve, summarizing, abstracting — is Spirit-authored.**
- **FR30:** Spirit can produce distillates (digests) via Spirit-side LLM compression; kernel enforces I11 audit-chain on digest writes (mandatory `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage`).
- **FR31:** Spirit can write principal-related data to the `principal:<principal_id>:<spirit-author-defined-schema>` namespace (per ADR-026); data inherits subject-access query, right-to-be-forgotten, and redaction-on-export operations. **The kernel allocates the namespace and enforces isolation; the kernel does not index or interpret content.**
- **FR32:** Spirit author can declare per-tag epistemic policies referencing tagged scalars and the four universal-arithmetic predicates; **kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from). Cognitive work — choosing the threshold, designing the predicate semantics, computing the underlying scalars — is Spirit-authored.** Predicate-firing recall floor ≥0.85 per Spirit class; precision floor ≥0.85.
- **FR56:** Spirit can read its own performance telemetry (success/failure counts, latency distributions, halt-recall events, distillation outcomes) scoped to its principal namespace per FR31, without requiring per-read operator admission. Self-telemetry feeds Spirit-side calibration and skill-revision proposals (FR57). **Spirit's own data; Spirit reads it.**

### F. Spirit Ecosystem and Distribution (12 FRs)

- **FR33:** Spirit author can scaffold a new Spirit via `cargo generate maos-spirit` (Rust v0.1+) or per-language template (TypeScript v0.5+, Python v1.0+, Go v1.5+).
- **FR34:** Spirit author can test a Spirit via `spirit-test` SDK harness without spinning up a kernel; harness covers lifecycle hooks, IAC frame I/O, halt resolution, manifest self-check, and class-specific regression corpus. **Coverage floor: 80% of Spirit author's manifest-declared capabilities reachable via SDK fixtures**, validated by external-author trial in 5 third-party Spirits.
- **FR35:** Spirit author can publish a Spirit package via `maos-spirit publish --tier=<tier>` with Ed25519 signing; package conforms to `maos.spirit.v1` schema.
- **FR36:** User can install third-party Spirits via authenticated control plane with mandatory signature verification, trust-tier floor enforcement, and ComplianceClaim envelope verification at admission.
- **FR37 (DEFERRED to v2.5):** Vetter (third-party authority) can issue a vetting attestation promoting a Spirit from `public-untrusted` to `public-vetted` tier; attestation is Ed25519-signed and journaled with revocation semantics. **Phase deferred from v1.0 → v2.5 ecosystem-adoption phase per John's first-cut-if-slip rule** (vetter ecosystem requires public-Spirit marketplace which v1.0 team-readiness doesn't need).
- **FR38:** Third-party assessor can issue a ComplianceClaim envelope binding (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) to a compliance attestation; kernel verifies at admission and refuses to load Spirits whose runtime context drifts.
- **FR39:** Spirit author can author skills (markdown with TOML frontmatter conforming to `maos.skill.v1`) and either ship them in the Spirit's package or write them dynamically at runtime via the `skill.author.self` capability scope; new skills land in pending state pending operator admission.
- **FR40:** Spirit author can publish a CLI-wrapper Spirit configuration declaring `output_shape_version`; kernel-builtin CliWrapperSpirit refuses to start if observed CLI shape doesn't match declared version (fail-loud, never silent).
- **FR57:** Spirit can query its own performance telemetry within its principal namespace (FR31) and emit skill-revision proposals carrying (a) the target skill id and version, (b) the proposed diff, (c) the telemetry evidence supporting the proposal. Such proposals enter the operator-admission queue (FR39) and are subject to the same vetting and audit obligations. **Operationalizes the "actors learn from each performance" claim that defends the hermes-tenant positioning.**
- **FR58:** User can complete the zero-config path from `install` to first useful Spirit response within the J0 evaluator budget (≤30 minutes including install). At least one bundled or auto-fetchable reference Spirit is suitable for evaluation. **Onboarding-validation gate (NFR-Onboarding-1 in Step 10).**
- **FR59:** Spirit registry supports publisher- and vetter-initiated yank events that propagate to operators on next sync (≤5min poll cadence default), distinguishable in audit from operator-local revocation (FR13), with documented operator response semantics (warn / quarantine / auto-revoke per operator policy).
- **FR60:** Substrate supports import of signed Spirit and skill artifacts (with vetter attestations and ComplianceClaims) from offline media or mirrored registries, preserving the full verification chain (FR36). Air-gapped deployments operational.

### G. Audit, Compliance, and Operator Surfaces (11 FRs)

- **FR41:** Operator can run frame-by-frame log queries via authenticated audit interface with filters by Spirit, capability, time-range, frame-kind, and tag. **Query latency floor: P99 ≤2s for queries scoped to a single Spirit on a 30-day window; P99 ≤10s for global queries; completeness floor 100% (no entries silently dropped).** Query language is specified separately (audit-query-surface ADR — extension to ADR-013).
- **FR42:** DPO can run subject-access queries via `maosctl audit subject-access --principal <id>`; returns all principal-namespace entries across all Spirits with provenance (Spirit, time, derived-from observations).
- **FR43:** CISO can run posture-delta queries via `maosctl audit posture-delta --range=<timespan>` surfacing capability-scope changes, sandbox-tier changes, and consent-policy changes over a configurable time-range with approval-chain attribution.
- **FR44:** External regulator can request a sealed-export via `maosctl audit sealed-export <bundle-spec>`; bundle is Ed25519-signed by the operator's audit key, third-party-verifiable, conforms to `maos.audit-bundle.v1` schema.
- **FR45:** User can exercise GDPR Article 17 right-to-be-forgotten via `maosctl forget --principal <id> [--reason <legal-hold>]`; kernel removes all principal-namespace entries; the deletion event itself is journaled (preserving lifecycle invariant) but the principal data is gone. Cross-Spirit cascade: forgetting cascades to working-memory references in other Spirits where principal data was shared; distillates containing principal data are marked redacted with re-distillation triggered. **Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries.**
- **FR46:** Operator can export filtered raw trajectories via `journal.export(filter, redaction_policy)` per ADR-023; bundle conforms to versioned `maos.trajectory.v1` schema with Ed25519 signing and applied-redaction flag.
- **FR61:** Substrate project publishes and maintains `SECURITY.md` documenting (a) disclosure contact (`security@maos.dev` with published GPG key), (b) coordinated-disclosure window and CVE-assignment process, (c) supported-versions matrix for security backports, (d) advisory-publication channel. **v0.1 binding** — not deferred; security disclosure pipeline must exist before any Spirit is shipped to a third party.
- **FR62:** Substrate exposes audit-queryable artifacts for governance: (a) vetter-key admission and rotation events, (b) ABI-extension proposals and their ratification status, (c) ComplianceClaim schema versions and their effective dates. **Operationalizes Constitutional Substrate Evolution (Innovation #7 from Step 6).**
- **FR63:** All kernel-emitted errors carry stable typed codes from a published catalog at `https://docs.maos.dev/errors/<ERR_NAME>` with documented retryability, cause-chain semantics, and version-stability guarantees consistent with the LTS policy. **CI-enforced metadata** per error variant; v1.0 binding (catalog initial set covers the 14+ named errors documented in architecture-maos.md).
- **FR64:** Operator can attribute cost (token-spend per provider, subprocess CPU-time, storage I/O) per Spirit per task per principal in the Transparency Log. **Enterprise-readiness gate** — no enterprise deployment without per-tenant cost accounting.
- **FR65:** Operator can uninstall a Spirit; kernel emits a proof-of-erasure record enumerating all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations). **Defends the v1.0 hermes-tenant positioning claim** that substrate-uninstall is a real guarantee, not a hope.

---

### FR-to-architecture traceability (updated; 65 FRs across 27 ADRs and 14 invariants)

| Capability area | Primary architectural anchors | Phase first delivered |
|---|---|---|
| A. Kernel Substrate Operations | I1, I2, I3, I9, I10; ADR-001/004/005/009 | v0.1 (FR1, FR2, FR4, FR47); v0.5 (FR3, FR5, FR6, FR7, FR48) |
| B. Spirit Lifecycle Management | I6, I10; ADR-002/007/016/017/020/022 | v0.1 (FR8, FR9); v0.3 (FR10–FR11); v0.8 (FR12, FR13, FR50); v1.0 (FR49) |
| C. Human-Spirit Interaction | ADR-013 task.assign; I4 approvals; I12 decision-context | v0.1 (FR14, FR15); v0.3 (FR16, FR17, FR18); v0.5 (FR19); v0.8 (FR20); v1.0 (FR51) |
| D. Multi-Spirit Coordination | ADR-008/012/014/015/021/025; I8 typed-intent consent; ADR-018/019/I13/I14 | v0.3 (FR55); v0.5 (FR25, FR26); v0.8 (FR21–FR23a, FR24, FR52, FR53, FR54-stub); v1.0 (FR23b, FR54-full) |
| E. Memory, Cognition Substrate, Distillation | I5, I11, I12, I13; ADR-013/022/026; §4.0.7 principle | v0.3 (FR27, FR32); v0.5 (FR28, FR29); v0.8 (FR30, FR31, FR56) |
| F. Spirit Ecosystem and Distribution | ADR-008/009/015/021/024/027 | v0.1 (FR58 onboarding-tutorial); v0.3 (FR33, FR34); v0.5 (FR35, FR36, FR38, FR39, FR40); v0.8 (FR57); v1.0 (FR59); v1.5 (FR60); v2.5 (FR37) |
| G. Audit, Compliance, Operator Surfaces | I2; ADR-013/015/023/026; Step 5 role-distinct queries | v0.1 (FR61); v0.5 (FR41); v1.0 (FR42–FR46, FR62, FR63, FR64, FR65) |

**65 functional requirements total.** Each is a testable capability stated implementation-agnostically. Each can be implemented in multiple ways within the architectural commitments. Each will need an epic or stories during implementation. The Kernel Non-Goals preface (above) is load-bearing alongside the FRs — without it, the kernel-stays-neutral principle is invisible.

### NFR Carry-Forward to Step 10

The following commitments are functional in spirit but quality-attribute in shape; they route to Step 10 (Non-Functional Requirements):

- **NFR-Reliability-1 (Murat):** Silent-failure detection. Kernel surfaces a `silent_failure_suspect` event when a Spirit emits no progress IAC frames for >N seconds despite holding an active task — even if heartbeats are healthy.
- **NFR-Observability-1 (Murat):** Author-observability contract. Spirit author can read the same diagnostic surface the operator sees for their own Spirit, redacted of cross-Spirit data.
- **NFR-Auditability-1 (Murat):** Capability-contract introspection. External party (vetter, regulator, auditor) can query `maosctl capability inspect <spirit>` and receive a machine-readable list of declared capabilities, observed capabilities used in last 30d, capability-token issuance count per type.
- **NFR-Auditability-2 (Murat):** Drift detection. Kernel compares a Spirit's runtime behavior against its declared manifest (capabilities used, tags written, halts emitted) and flags drift exceeding a configured threshold.
- **NFR-Auditability-3 (John):** Deterministic replay. Given the same Transparency Log + same Spirit binaries, replay produces the same output (modulo external-CLI nondeterminism, which is itself logged). Hard at v1.0; revisit at v1.5.
- **NFR-Testability-1 (Murat):** Test-corpus reproducibility. All ship-gate test corpora are reproducible from a published seed and dataset; third-party assessor can re-run and obtain bit-identical pass/fail.
- **NFR-Testability-2 (Murat):** Kernel-API surface invariant test. Run on every kernel commit. Enumerate every kernel API exported to Spirits (build-time reflection); classify each function by computational class (universal-arithmetic / data-movement / supervision / **other**); **floor: zero functions in class "other"**. Any new function entering class "other" is a build-break. Plus a static analyzer for predicate definitions that proves reduction to universal arithmetic.
- **NFR-Security-1 (Murat):** Negative-capability assertion. Spirit author can declare `forbidden_capabilities` in the manifest; kernel enforces that the Spirit never holds tokens for forbidden surfaces, even transitively via A2A.
- **NFR-Onboarding-1 (Paige + John):** 30-Min First Spirit Validation Gate. N=12 stratified external Spirit authors (≥4 with no prior MAOS contribution; ≥3 who've never written a Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only); ≤45 minutes median, ≤90 minutes p95, ≥10/12 succeed. **v0.3 release criterion** — Butler is the first Spirit a new user actually installs.

These nine NFR commitments get formalized in Step 10 with explicit acceptance criteria and ship-gate phases.

## Non-Functional Requirements

> **Quality contract.** NFRs specify HOW WELL MAOS must perform — not WHAT it does (Step 9 FRs). Each NFR carries a numeric floor (or structural test) and a phase commitment. Untestable NFRs are unfalsifiable promises and are excluded. **All party-mode amendments from Step 10 round 2 are integrated** (Mary's compliance/operational gaps, Murat's corpus sizing + meta-testing + reproducibility honest revision, Winston's ADR-coverage gaps + tensions + structural-not-semantic clarifications, John's phase moves + cost/reliability/scope additions).

The 13 categories below cover **~85 NFRs** anchoring the substrate's quality contract. Several categories grew during party-mode review: a new **Compliance & Regulatory** category formalizes Mary's jurisdictional gaps, a new **Meta-Testing** category captures Murat's corpus-of-corpora discipline, and **Cost & Tenancy** entries formalize John's substrate-credibility additions.

### Performance

- **NFR-Perf-1:** IAC frame routing latency P50 < 5ms, P99 < 50ms on a typical Linux box (NVMe + 16-core tier). v0.5.
- **NFR-Perf-2:** Sustained IAC frame throughput 5,000–10,000 frames/sec single-host before log writer becomes bottleneck. Per-Spirit fairness scheduler in front of log writer (NOT FIFO). v0.5.
- **NFR-Perf-3:** Capability-token validation latency P99 < 100µs per check; 100% re-validation at use against current state, not cached state (TOCTOU correctness). v0.1.
- **NFR-Perf-4:** Posture-shift propagation P99 ≤ 2s, P99.9 ≤ 5s in 1000-shift corpus. v0.3.
- **NFR-Perf-5:** Audit query latency P99 ≤ 2s for single-Spirit queries on 30-day window; P99 ≤ 10s for global queries. v0.5 (basic), v1.0 (signed-export tier).
- **NFR-Perf-6:** Distillation step latency budget declared per Spirit class via manifest `[budget].time_cap`; soft warning at 80%; kernel emits `BudgetWarning` IAC frame. v0.5.
- **NFR-Perf-7:** Hot-swap latency P99 < 500ms (mode switch + state transfer + capability rebinding) for same-major same-additive swaps. v0.8.

### Reliability

- **NFR-Rel-1:** Spirit-process crash detection ≤ 2s; `task.orphaned` IAC frame ≤ 5s. Floor: ≥99/100 detected within 2s on SIGKILL crash corpus. v0.8.
- **NFR-Rel-2:** Hung-Spirit detection (no-progress IAC for >30s) → `task.stalled` event within 60s. Floor: ≥48/50 reclassified within 60s on hang corpus. v0.8.
- **NFR-Rel-3:** HSIS (Hot-Swap Invariant Suite) ≥ 95% pass per Spirit class; **zero invariant violations** (CVSS-7 class). 6 class-specific corpora at 50 scenarios each; stratified swap-lifecycle phase distribution. v1.0.
- **NFR-Rel-4:** Silent-failure detection. Kernel emits `silent_failure_suspect` event when Spirit emits no progress IAC frames for >30s despite healthy heartbeats. Floor: ≥45/50 detected on adversarial zombie-heartbeat corpus. v1.0.
- **NFR-Rel-5:** Hot-swap rollback within 30s if successor health-check fails. Kernel auto-reverts to predecessor and emits `HotSwapAborted` IAC frame. v1.0.
- **NFR-Rel-6:** Spirit-restart invalidates prior A2A TOFU pins; re-pin protocol with consent confirmation. v1.0.
- **NFR-Rel-7:** A2A trust establishment under churn — 100-host Cortex (or compressed 30-host scale per Murat's cost-compression), 10–20% host turnover/week for 4 weeks, 3 planted adversarial hosts. Floor: detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h. v2.0 (compressed) / v2.5 (full 100-host). [PHASE-MOVE per John]
- **NFR-Rel-8:** Lifecycle journal durability — fsync per state transition; ring-buffer flush latency < 1ms. v0.1.
- **NFR-Rel-9** [NEW, Mary]: Revocation propagation latency ≤ 5s p99 under 10⁴ concurrent capability-token validations. Closes Winston's "A2A trust establishment under churn" production-risk gap and the weakest leg of the hermes-tenant positioning sentence. v0.8.
- **NFR-Rel-10** [NEW, John]: Kernel cold-restart ≤ 30s with no data loss on graceful shutdown; ≤ 1 in-flight message loss on hard kill. v0.8.
- **NFR-Rel-11** [NEW, Winston]: Halt-receipt production rate ≥ 99.9%. Every Spirit termination, planned or unplanned, produces a halt receipt before process exit. Closes I14 directly (separate from HSIS aggregate). v0.8.

### Security

- **NFR-Sec-1:** Sandbox tier enforced per Spirit; strictest-of-(manifest, trust-tier, operator-policy) floor. v0.1 (T0/T1/T2); v0.5 (T3); v2.0 (T4 WASM).
- **NFR-Sec-2:** Capability-token TTL ≤ 60s for high-privilege operations; bound to Spirit-PID + boot-nonce; audit-logged at every use with origin-Spirit-ID. v1.5 (ADR-023).
- **NFR-Sec-3:** Sandbox-escape **structural** anomaly detection (syscall pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections). **The kernel raises a structural alarm; the *interpretation* of whether the alarm constitutes malice is Spirit-side or operator-side. The kernel does not classify intent.** v2.0 (ADR-024). [STRUCTURAL-NOT-SEMANTIC clarification per Winston]
- **NFR-Sec-4:** Pre-write secret-redaction filter at Transparency Log boundary. Floor: **0 secrets in any logged frame, ever**. Two-tier: 10⁴-case corpus per-commit + 10⁵-case quarterly audit + production canary system (1000 unique synthetic secrets/month with cryptographic markers). Discovery latency ≤ 24h p95. Any false negative is P0 ship-blocker. v0.5. [TWO-TIER per Murat]
- **NFR-Sec-5:** Manifest parser fuzz: 24h `cargo-fuzz`, zero crashes/OOMs/infinite loops. v1.0 ship gate.
- **NFR-Sec-6:** Wire protocol adversarial-input fuzz: 24h, zero crashes. v1.0.
- **NFR-Sec-7:** External pen-test report with zero P0/P1 findings open at v1.0 ship. **Triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to PRD-author tiebreak. P0/P1 definitions per OWASP Risk Rating Methodology, frozen at engagement start.** Pen-tester engagement scheduled 6–8 weeks before v1.0 ship as critical-path dependency. [TIGHTENED per Murat + John]
- **NFR-Sec-8:** Negative-capability assertion via manifest `forbidden_capabilities`; kernel enforces never holding tokens for forbidden surfaces, even transitively via A2A. v1.0.
- **NFR-Sec-9:** Zero `unsafe` blocks in kernel capability-validation path (Rust). v0.1 ship gate.
- **NFR-Sec-10** [AMENDED, Murat]: Adversarial-Spirit red-team **80-scenario** corpus across **8 attack classes** (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), **N=10 per class**. Floor: **≥9/10 per class** detected/blocked by kernel; ≥72/80 aggregate; **0 unmitigated category** (no class scores 0). Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed. v1.5 [PHASE-MOVE per John, paired with pen-test budget].
- **NFR-Sec-11:** mTLS handshake replay-attack test: 1000 captured handshakes replayed, 0 succeed. v0.5 (loopback) / v1.0 (cross-host).
- **NFR-Sec-12:** TOFU pin-mismatch on second connection: 100% detected, blocked, alerted. v0.5.
- **NFR-Sec-13:** mTLS cert rotation chaos test: 3-host at v1.5; 10-host at v2.0; rotation under load with zero conversation drops; revocation latency median ≤ 60s, p99 ≤ 5min. [PHASE-SPLIT per John]
- **NFR-Sec-14** [NEW, Mary + Winston merged]: **Cross-Spirit memory isolation corpus** — 200-scenario adversarial corpus where Spirit-A actively attempts to enumerate, read, side-channel, or timing-attack Spirit-B's substrate state. Categories: namespace enumeration, working-memory read-across, decision-frame observation, halt-signal observation, transparency-log cross-read, working-memory-digest cross-read, capability-token forgery cross-Spirit, sandbox-escape lateral. Floor: **200/200 isolation maintained**; any leak = P0 ship-block. Defends the v1.0 hermes-tenant positioning sentence. v0.8 (must be in place before the positioning sentence is allowed in marketing).
- **NFR-Sec-15** [NEW, Mary]: Crypto-module pluggability with FIPS 140-3-validated default option. Kernel-internal cryptographic operations (signature verification, sealed-export encryption, capability-token signing) route through a provider trait permitting substitution of FIPS-validated, hardware-backed, or post-quantum implementations without recompilation of Spirits. v1.0.
- **NFR-Sec-16** [NEW, Winston]: Manifest-evolution lint forcing binary `secret`/`non-secret` annotation on every new manifest field — no default. Mitigates structural-vs-semantic redaction tension (Tension A) by shifting cost from runtime detection (forbidden by §4.0.7) to authoring time. v0.5.

### Auditability & Compliance

- **NFR-Aud-1:** Capability-contract introspection via `maosctl capability inspect <spirit>`. Returns machine-readable list of declared capabilities, observed capabilities used in last 30d, capability-token issuance count per type. **Log-completeness corpus with N=100 injected events; floor ≥98/100 events recoverable from logs.** v1.0. [TIGHTENED per Murat]
- **NFR-Aud-2:** Drift detection — kernel compares Spirit's **set-membership and frequency-distribution** (capabilities used, tags written, halts emitted) against manifest declarations. **Set-membership and frequency-distribution comparison only — no semantic interpretation. Per §4.0.7, the kernel does not classify whether observed behavior is "suspicious" or "malicious"; it surfaces structural divergence and the operator (or Spirit-side cognition) interprets.** v1.0. [STRUCTURAL-NOT-SEMANTIC clarification per Winston]
- **NFR-Aud-3:** Deterministic replay anchored by ADR-028. Replay determinism is over the **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders. v1.0 best-effort; v1.5 hard target.
- **NFR-Aud-4:** Audit retention ≥ 90 days private tier (default); configurable per-deployment; Merkle-root anchoring optional for tamper-evidence. v0.5.
- **NFR-Aud-5:** Right-to-explanation via I12 — 100% of `decision.*` frames carry `working_memory_digest_refs` for explainability replay. EU AI Act adjacent compliance. v0.8.
- **NFR-Aud-6:** Sealed-export Ed25519-signed by operator audit key; third-party-verifiable; conforms to `maos.audit-bundle.v1` schema. **Bundle includes both working-memory digest refs (I12) AND distilled-output content (I11).** v1.0.
- **NFR-Aud-7:** Five-metric distillation gate per distillation-shipping Spirit:
  - Digest-recall ≥ 0.90
  - Digest-faithfulness ≥ 0.98 unflagged contradictions
  - Digest-hedge-preservation ≥ 0.95
  - Digest-traceability = 100% (kernel-enforced via I11)
  - Digest-secret-leakage = 0% (zero-tolerance)
- **NFR-Aud-8** [AMENDED, Murat]: **Two-tier corpus**: N=100 calibration per-commit (CI width 0.124, fine for trend detection) + **N=500 quarterly audit** (CI width ≤0.05 at p=0.90 for digest-recall; tight statistical confidence). Plus 10⁵-case secret-leakage corpus + production canary system per NFR-Sec-4. v0.5 (per-commit), v1.0 (quarterly).
- **NFR-Aud-9** [AMENDED, Murat]: ComplianceClaim Adversarial Corpus (CCAC) v1.0 — N=600 (200 well-formed + 400 malformed). **Per-class N=30, floor ≥ 27/30** (Wilson CI tightened from N=20 vagueness; detects 95% → 70% degradation reliably). 100 context-drift claims (100/100 rejected). Cross-validation across ≥3 reference Spirits, agreement within ±2%. v1.0 ship gate.
- **NFR-Aud-10:** GDPR Article 17 right-to-be-forgotten — 50-scenario corpus with cross-Spirit cascade. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries. v1.0.
- **NFR-Aud-11:** SIEM export at v2.0. OpenTelemetry adapter at v1.0.
- **NFR-Aud-12** [NEW, Mary + Winston merged]: **Storage cascade erasure completeness + externally-verifiable uninstall receipt.** Substrate-uninstall produces a portable, externally-verifiable erasure receipt (signed Merkle inclusion + signed Merkle exclusion proof, retained independent of the substrate). 100% of registered storage backends prove erasure within bounded window for any given principal. **Closes the weakest leg of the hermes-tenant positioning sentence.** v1.0.
- **NFR-Aud-13** [NEW, Mary]: Time-to-erasure SLA. Floor: 95% of right-to-be-forgotten requests complete within 30 days (configurable to 7 for enterprise tier); audit log entry within 24h of request acceptance. v1.0.
- **NFR-Aud-14** [NEW, Winston]: Intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain back to originating principal intent. Closes ADR-018/I13 NFR coverage gap. v0.8.

### Testability

- **NFR-Test-1** [AMENDED, Murat — honest revision]: All ship-gate test corpora are **static artifacts content-addressed in the repo** (SHA-256 of JSONL); generation provenance is documented but not required to be reproducible. Pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash committed alongside, retry budget=1, quarterly re-baseline with ≥98% agreement on golden snapshot. v1.0.
- **NFR-Test-2:** **Kernel-API surface invariant test (per-commit gate).** Build-time reflection enumerates every kernel API exported to Spirits via `kernel::api::*`; classifies each function by computational class (universal-arithmetic / data-movement / supervision / **other**); **floor: 0 functions in class "other"**; new function entering class "other" is build-break. Static analyzer on Rust `syn` walking allowlist-based predicate definitions; **decidable for permitted subset (no theorem prover)**. **Kernel-utility crate (`kernel::util::*`) has separate looser invariant: no I/O except via injected trait, no global state. The allowlist is the contract; PR-amendment process (not flag) for changes; sign-off from PRD author + tech lead.** Per the §4.0.7 founder principle. v0.1 build gate (surface-diff only); v0.5 adds static analyzer for predicates [PHASE-SPLIT per John].
- **NFR-Test-3:** spirit-test SDK harness coverage ≥ 80% of Spirit author's manifest-declared capabilities reachable via fixtures; validated by external-author trial in 5+ third-party Spirits. v1.0.
- **NFR-Test-4:** Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class on `bmad-eval` standard corpus. v0.5.
- **NFR-Test-5** [PHASE-SPLIT per John]: FKCS (Frozen-Kernel Conformance Suite). **FKCS-infrastructure (diff oracle, test harness, kernel-frozen-vN.0 commit-tagging) at v2.0**; **FKCS-populated (3 future Spirits implemented by external authors) at v2.5** (requires ecosystem of three external authors). Floor: ≥27/30 per Spirit, ≥85/90 aggregate; diff oracle confirms zero kernel changes; negative-control "fourth Spirit" deliberately uses undocumented kernel internal and MUST fail.
- **NFR-Test-6** [AMENDED, Murat]: LCAS (Long-context Ambiguity Stress) corpus — **N=210 scenarios** in 3 buckets (clearly-decidable n=70 / genuinely-ambiguous n=70 / **adversarially-misleading n=70** — raised from 60 for statistical power; Mann-Whitney U at p<0.01 needs ~64 per group at power=0.84). Adversarial trajectories contain a planted load-bearing claim contradicting a louder repeated claim. v0.5 ship gate.
- **NFR-Test-7** [PHASE-MOVE per John]: Cross-form Semantic equivalence (rust-inproc ↔ subprocess) ≥ 90%; (any-rust ↔ wasm-component) ≥ 75%. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs). **v1.5** (rust↔subprocess; cohort interop at v1.0 is rust-rust); **v2.0** (any-rust↔wasm).
- **NFR-Test-8** [AMENDED, Murat]: **Black-box third-party trial v1.0 — N=12 stratified** (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). 14-day no-DM-support window. Floor: **≥10/12 produce working signed Spirit binary** that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85. Wilson CI [0.552, 0.962] meaningful at N=12; meaningless at N=5. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot. **Run only at major releases (v1.0, v2.0); minor releases use NFR-Onb-1 (12-author onboarding) as proxy** [COST-COMPRESSION per Murat].
- **NFR-Test-9** [NEW, Winston]: Loom-not-in-kernel structural test. `grep` of kernel crate for orchestration/planning symbols returns ∅. Per-commit gate. Covers ADR-006's negative commitment (Loom is user-space). v0.5.
- **NFR-Test-10** [NEW, Winston]: Skill-format conformance — at least one third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification. Covers ADR-027's external-standard interop assertion empirically. v1.5.
- **NFR-Test-11** [NEW, Mary]: Namespace grammar lock test. Grammar `.lark` (or equivalent) hash pinned in CI; any change requires architecture-lock review process, not regular PR. v0.5.
- **NFR-Test-12** [NEW, Mary]: **v0.3 architecture lock script as per-commit gate.** `scripts/check_v0_3_lock.sh` runs four mechanical checks: (1) `LICENSE` matches ADR-decided license string; (2) consortium-target ADR exists with status `accepted` and ≥2 maintainer sign-offs; (3) `ROADMAP.md` has trust-anchor decision section with status `decided` linking to ADR; (4) failure-semantics doc exists with at least one fully-specified route. **No v0.3 tag without script in green.** v0.3.
- **NFR-Test-13:** Manifest field test coverage ≥ 3 cases per field (well-formed, malformed-rejected, edge-case); CI-enforced. v0.1.
- **NFR-Test-14:** Wire protocol cross-language byte-equal golden corpus per frame variant per SDK (Rust + TS v0.5 + Python v1.0 + Go v1.5+). v1.0.

### Meta-Testing [NEW CATEGORY, Murat]

- **NFR-Meta-1:** **Corpus-quality audit.** Each ship-gate corpus reviewed by independent assessor (not corpus author) on a 10-point rubric (representativeness, edge-case coverage, label correctness, distribution match to production). Floor: ≥8/10 per corpus. Cadence: at corpus creation + every 12 months. v1.0.
- **NFR-Meta-2:** **Corpus-staleness.** Every corpus carries a `valid_until` date in metadata. CI fails if any active gate references an expired corpus. Default validity: 12 months. Extension requires explicit "no-update justification" PR with assessor sign-off. v1.0.
- **NFR-Meta-3:** **Coverage matrix.** Single source-of-truth file `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}. CI fails if any FR/NFR has zero corpus coverage. Generated report surfaces gaps automatically. Floor: 100% FR coverage, 100% NFR coverage at v1.0. v1.0.

### Observability

- **NFR-Obs-1:** Author-observability contract — Spirit author can read same diagnostic surface as operator for their own Spirit, redacted of cross-Spirit data. **Metric M is queryable in <500ms with cardinality ≤10⁴.** v1.0. [TIGHTENED per Murat]
- **NFR-Obs-2:** OpenTelemetry export per IAC frame, capability invocation, halt event. v0.5 basic; v1.0 SLO-class.
- **NFR-Obs-3:** Per-Spirit telemetry stream with topic-based broadcast + filtered subscription. v0.3 (Butler narrow); v0.5 (Observer broad).
- **NFR-Obs-4:** Transparency Log per-Host SQLite (append-only), exportable to JSONL/SIEM with redaction policy applied. v0.5.
- **NFR-Obs-5:** Approval Decision Log distinct from Transparency Log; full intent + decision + reasoning chain per Invariant I4. v0.3.

### Documentation Quality

- **NFR-Doc-1:** Every public ABI method has ≥ 1 doctested example; CI broken-link blocking on doc site **at v0.5 (when doc site lands)**; doctest CI gate at v0.1 [PHASE-SPLIT per John].
- **NFR-Doc-2:** Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` covering all 14+ named typed errors. **CI-enforced metadata: each variant has 6 fields (code, severity, recovery-class, owner, kernel-or-spirit, since-version). CI runs `cargo run --bin error-metadata-check` which exits non-zero if any variant is missing any field; each field has its own assertion.** v1.0. [TIGHTENED per Murat]
- **NFR-Doc-3:** API reference site at `https://docs.maos.dev/abi/<version>/`; versioned, searchable, deep-linkable, archived ≥ 2 minor versions back. v1.0.
- **NFR-Doc-4:** Manifest schema reference (human-rendered); pattern cookbook; migration runbooks (Path A + Path B); troubleshooting guide; deployment topology guide. v1.0.
- **NFR-Doc-5:** WCAG AA compliance for doc site. v1.0.
- **NFR-Doc-6** [PHASE-MOVE per John]: Localization v1.0 = **Korean only** (shipped); Japanese + Chinese-simplified at v1.5. `LOCALES.md` with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- **NFR-Doc-7** [PHASE-MOVE per John]: Doc tooling supports per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown. **RTL layout support deferred to v2.5** (no RTL locale targeted before v2.5). Pick mdBook + i18n / Docusaurus / VitePress by v0.5; v1.0 in production.

### Onboarding

- **NFR-Onb-1:** **30-Min First Spirit Validation Gate.** N=12 stratified external Spirit authors (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). Floor: median ≤ 45 min, p95 ≤ 90 min, AND **≥ 10/12 succeed where "succeed" = author produces Spirit binary that (a) compiles against published ABI, (b) passes ≥27/30 of the FKCS scenarios for chosen Spirit class, (c) does so within 14 calendar days from kit handoff with zero direct-message support; forum/docs questions allowed and logged.** v0.3 release criterion. [TIGHTENED per Murat]
- **NFR-Onb-2:** First-time installer J0 evaluator path — install + first useful Spirit response within 5 minutes. v0.1.
- **NFR-Onb-3:** Three-door page at `docs.maos.dev` ("write a Spirit" / "run MAOS" / "understand MAOS"). v0.5.
- **NFR-Onb-4** [NEW, Mary]: 30-Min Gate iteration cadence. If floor missed, run fresh 6-author cohort within 2 weeks; three consecutive misses escalate to v0.3 release-criterion review. Operational commitment, not one-shot gate. v0.3.

### Maintainability

- **NFR-Maint-1:** **Kernel trusted core ≤ 20 KLOC excluding tests through v2.0** (core scheduler + IAC bus + capability check + journal). Integration adapters in separate crates with their own LOC budgets. v2.0.
- **NFR-Maint-2** [PHASE-SPLIT per John]: Capability-registry fuzz coverage **≥60% line at v0.1; ≥80% line / ≥60% branch at v0.5** on 1M-iteration libFuzzer run; zero crashes.
- **NFR-Maint-3:** ABI compatibility matrix 100% within current major; 100% N-1 boundary including negative typed-error cases. v0.1 (within-major); v1.0 (N-1).
- **NFR-Maint-4:** STABILITY.md publishes live (kernel_version, abi_version, manifest_schema_version) compatibility matrix. v1.0.
- **NFR-Maint-5:** Deprecation timeline: 2 minor releases of warning, 1 major release to remove. v1.0.
- **NFR-Maint-6** [PHASE-SPLIT per John]: **1-year LTS commitment at v1.0**; 2-year LTS commitment at v1.5 once support load is known; security-only patches after year 1. Don't write a check the v0.8 team can't cash.
- **NFR-Maint-7** [PHASE-MOVE per John]: BREAKING.md required entry for every breaking change with migration steps; CI grep-enforced. **v1.0** (you don't break things until you have stable surface to break).
- **NFR-Maint-8:** Capability-token TOCTOU test: 100% re-validation at use against current state. v1.0.
- **NFR-Maint-9** [NEW, Winston]: Manifest schema N-1 compatibility — kernel version V can load manifests written for V-1 with documented degradation paths. Closes ADR-025 NFR-coverage gap. v1.0.

### Scalability

- **NFR-Scale-1:** Cortex 3-region pilot at v2.0 with ≥ 10 agents minimum; sustained operation for 30 days; zero substrate-invariant violations.
- **NFR-Scale-2** [PHASE-SPLIT per John + cost-compression per Murat]: **25-host churn test at v2.0; 100-host churn at v2.5** (cost compression: 100→30 hosts at v2.0 same churn-events-per-week, full 100-host moves to v2.5).
- **NFR-Scale-3:** Per-Spirit fairness scheduler in front of log writer (NOT FIFO). v0.5.
- **NFR-Scale-4:** Provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame. v0.5.
- **NFR-Scale-5:** Multi-host A2A peer mesh scales to 14-institution Cortex; v2.0 target with documented capacity envelope.

### Operational

- **NFR-Ops-1:** Substrate operations checklist fully delivered: install, upgrade, yank, uninstall, revoke. v0.1 (install/uninstall) → v0.5 (upgrade/yank) → v1.0 (revoke).
- **NFR-Ops-2:** Signed Revocation List (CRL) artifact; registry-pushed (kernel polls every 5min) + offline-import path. v1.0.
- **NFR-Ops-3:** Telemetry opt-in default; `PRIVACY.md` with retention, jurisdiction, deletion path; per-field redaction layer. v1.0.
- **NFR-Ops-4:** **`SECURITY.md`** with disclosure address (`security@maos.dev`), GPG key, embargo window (90-day default), advisory-publication channel, supported-versions matrix. **v0.1 ship gate.** **CNA registration through MITRE moves to v0.5** (6–12 weeks elapsed paperwork; v0.1 just needs disclosure pipeline to exist). [PHASE-SPLIT per John]
- **NFR-Ops-5:** maosctl `--plain` flag + `NO_COLOR` + `TERM=dumb` accessibility. v0.1.
- **NFR-Ops-6** [PHASE-MOVES per John]: Onboarding artifacts — `RFC_TEMPLATE.md` at **v0.8** (was v0.5), `GOVERNANCE.md` at v0.5 (basic) + v0.8 (locked), `CODE_OF_CONDUCT.md` at v0.5, `LOCALES.md` at **v1.0** (was v0.5), `TRADEMARK.md` at **v1.0** (was v0.5), `BREAKING.md` at **v1.0** (was v0.5; matches NFR-Maint-7).
- **NFR-Ops-7** [PHASE-MOVE per John]: Sustainability vehicle — declared-intent at v0.5 (Open Collective open, accepting $0 expected); **legal/fiscal-sponsor work at v0.8**.
- **NFR-Ops-8** [NEW, Mary]: Trust-anchor framing carry-forward decision. Published ADR by v0.3 declaring which competitive framing is committed (substrate-as-substrate vs substrate-as-trust-anchor); absence = v0.3 release-block. v0.3.
- **NFR-Ops-9** [NEW, Mary]: Transparency Log backup/DR. RPO ≤ 1h, RTO ≤ 4h, backup integrity verified weekly via Merkle-root cross-check. v1.0.
- **NFR-Ops-10** [NEW, Mary]: Database migration test corpus. SQLite→Postgres at v1.5 (committed in roadmap). Floor: forward-migration test on 10⁶-row corpus, byte-identical Merkle-root preservation post-migration, rollback path tested. v1.4 (gates v1.5).
- **NFR-Ops-11** [NEW, Mary]: Multi-operator tenancy isolation — primitive-reservation only at v1.0 (declared as primitive-reserved in namespace grammar so v0.5 grammar lock doesn't paint us into a corner; full implementation v1.5+). Per-operator namespace, per-operator transparency-log shard, per-operator capability-token signing key, per-operator GDPR-erasure scope. v1.0 (reserved); v1.5+ (implemented).
- **NFR-Ops-12** [NEW, Mary]: Air-gapped deployment validation. Substrate boots, runs, produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation; documented Spirit-author guidance for air-gapped capability tokens. v1.0.

### Compliance & Regulatory [NEW CATEGORY, Mary]

- **NFR-Comp-1:** Export-control classification artifact. ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel. v0.8 (before any v1.0 enterprise-distribution conversation).
- **NFR-Comp-2:** Vetter accreditation parameters — published vetter qualification matrix (cryptography review credential OR 5+ years agentic-security review OR equivalent), conflict-of-interest disclosure required, vetter rotation policy (no single vetter on >40% of Spirit-class promotions in any 12-month window), vetter audit-trail retained 7 years. v0.8.
- **NFR-Comp-3:** Substrate-self compliance scope declaration. `STABILITY.md` contains scope-disclaimer paragraph explicitly stating SOC 2 / ISO 27001 / FedRAMP scope is the *operator's* responsibility, not the substrate's, with kernel-as-service boundary drawn. **Structural test that the four named regimes appear with disclaimer; failure = ship-block.** v0.5.
- **NFR-Comp-4:** Region-pinning primitive (PIPL §40 / data localization). Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication. Without primitive, enterprise distribution cannot configure for PIPL. v1.0.
- **NFR-Comp-5:** Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent). Manifest declares covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission. v1.0.

### Cost & Tenancy [NEW CATEGORY, John]

- **NFR-Cost-1:** Cost-attribution accuracy ≥ 98% reconciliation against provider billing, sampled monthly. Per-Spirit per-task per-principal attribution. **Without this NFR, FR64 (cost accounting) is theater.** v1.0.
- **NFR-Tenancy-1:** Explicit single-tenant per kernel instance commitment through v2.0; multi-tenant primitive-reserved at v1.0 per NFR-Ops-11; full multi-tenant out of scope before v2.5. **Make the boundary loud** (avoids hidden-multi-tenancy assumptions in design reviews). v0.1 (declared); v2.0 (single-tenant guaranteed).

---

### NFR-to-architecture traceability (updated; ~85 NFRs across 28 ADRs and 14 invariants)

| Category | Anchors | Phase distribution |
|---|---|---|
| Performance (7) | ADR-001/010/011; I2 | v0.1, v0.3, v0.5, v0.8 |
| Reliability (11) | I6/I10; ADR-017/019/020/022 | v0.1, v0.8 (heavy), v1.0, v2.0/v2.5 |
| Security (16) | I1/I9; ADR-009/012/023/024 | v0.1, v0.5 (heavy), v1.0 (heavy), v1.5, v2.0 |
| Auditability & Compliance (14) | I2/I11/I12/I13; ADR-013/015/023/026/028 | v0.5, v0.8, v1.0 (heavy) |
| Testability (14) | All ADRs; §4.0.7 | v0.1, v0.3, v0.5, v1.0 (heavy), v1.5, v2.0/v2.5 |
| Meta-Testing (3) | Cross-cutting | v1.0 |
| Observability (5) | I7/I4; ADR-013 | v0.3, v0.5, v1.0 |
| Documentation Quality (7) | Step 7 commitments | v0.1, v0.5, v1.0, v1.5, v2.5 |
| Onboarding (4) | Step 7+8 commitments | v0.1, v0.3, v0.5 |
| Maintainability (9) | ABI triple §14 #4; STABILITY.md | v0.1, v0.5, v1.0, v1.5 |
| Scalability (5) | ADR-006; A2A peer mesh | v0.5, v2.0, v2.5 |
| Operational (12) | Step 7 substrate operations | **v0.1 (NFR-Ops-4 SECURITY.md, NFR-Ops-5 accessibility)**, v0.3, v0.5, v0.8, v1.0 |
| Compliance & Regulatory (5) | EU AI Act, NIS2, PIPL, SB-1047 | v0.5, v0.8, v1.0 |
| Cost & Tenancy (2) | FR64 anchor | v1.0 |

**~85 NFRs total.** Each carries a numeric floor (or structural test) and a phase commitment. Each will need a CI gate, test corpus, or operational artifact during implementation.

---

### NFR ship-gate consolidation by phase (post-Tier-3)

The most contested ship gates by phase, after John's rebalancing:

**v0.1 (foundational, ~6–8 weeks for one founder):** SECURITY.md basic (NFR-Ops-4, **CNA registration deferred to v0.5**), kernel-API surface invariant test surface-diff-only (NFR-Test-2, **static analyzer deferred to v0.5**), ABI matrix within-major (NFR-Maint-3), capability-registry fuzz **≥60% line** at v0.1 (NFR-Maint-2, **≥80% deferred to v0.5**), zero `unsafe` in capability-validation (NFR-Sec-9), manifest field test coverage (NFR-Test-13), accessibility flags (NFR-Ops-5), doctest CI gate (NFR-Doc-1, **broken-link blocker deferred to v0.5**), J0 evaluator path (NFR-Onb-2), lifecycle journal durability (NFR-Rel-8), capability-token TOCTOU (NFR-Perf-3 + NFR-Maint-8), explicit single-tenant commitment (NFR-Tenancy-1).

**v0.3 (Butler):** 30-Min First Spirit Validation Gate (NFR-Onb-1) + iteration cadence (NFR-Onb-4) — v0.3 release criterion. Posture-shift latency (NFR-Perf-4). v0.3 architecture lock script per-commit gate (NFR-Test-12). Trust-anchor framing carry-forward (NFR-Ops-8).

**v0.5 (Researcher + Observer):** LCAS test corpus N=210 (NFR-Test-6). Five-metric distillation gate baseline (NFR-Aud-7..8). Pre-write secret redaction with two-tier corpus (NFR-Sec-4). IAC routing latency (NFR-Perf-1, NFR-Perf-2). Halt-recall/precision per Spirit class (NFR-Test-4). Substrate-self compliance scope clause (NFR-Comp-3). Manifest-evolution lint (NFR-Sec-16). Loom-not-in-kernel structural test (NFR-Test-9). Namespace grammar lock (NFR-Test-11). Static-analyzer-for-predicates upgrade (NFR-Test-2 follow-on). 

**v0.8 (Founder Loop wedge demo):** Crash detection + hung-Spirit detection (NFR-Rel-1, NFR-Rel-2). Hot-swap latency (NFR-Perf-7). Right-to-explanation via I12 (NFR-Aud-5). **Cross-Spirit isolation corpus (NFR-Sec-14) — must be in place before positioning sentence allowed in marketing.** Revocation propagation latency (NFR-Rel-9). Kernel cold-restart (NFR-Rel-10). Halt-receipt production rate (NFR-Rel-11). Intent-lineage propagation (NFR-Aud-14). Export-control ECCN (NFR-Comp-1). Vetter accreditation parameters (NFR-Comp-2).

**v1.0 (Team-ready):** HSIS (NFR-Rel-3). CCAC N=600 (NFR-Aud-9). Black-box third-party trial N=12 (NFR-Test-8). Manifest fuzz + wire fuzz (NFR-Sec-5, NFR-Sec-6). External pen-test (NFR-Sec-7). Typed error catalog (NFR-Doc-2). **1-year LTS announcement** (NFR-Maint-6, **2-year deferred to v1.5**). GDPR right-to-be-forgotten (NFR-Aud-10). Cascade erasure receipt (NFR-Aud-12). Time-to-erasure SLA (NFR-Aud-13). Storage cascade completeness (NFR-Aud-12). Cost-attribution accuracy (NFR-Cost-1). Region-pinning primitive (NFR-Comp-4). Spirit model-provenance manifest (NFR-Comp-5). Crypto-module pluggability (NFR-Sec-15). TX log backup/DR (NFR-Ops-9). Air-gapped deployment validation (NFR-Ops-12). Multi-operator primitive-reservation (NFR-Ops-11). Three Meta-NFRs (corpus quality, staleness, coverage matrix). Manifest schema N-1 compat (NFR-Maint-9).

**v1.5 (Mira-Nash):** Capability-token TTL + bind-to-PID (NFR-Sec-2). 3-host mTLS cert rotation chaos test (NFR-Sec-13). Adversarial-Spirit red-team (NFR-Sec-10) — paired with pen-test budget. Cross-form rust↔subprocess equivalence (NFR-Test-7). Skill-format conformance (NFR-Test-10). Deterministic replay hard target (NFR-Aud-3). 2-year LTS commitment (NFR-Maint-6 follow-on). Localization JA + ZH (NFR-Doc-6 follow-on). DB migration test (NFR-Ops-10).

**v2.0 (technical):** FKCS-infrastructure (NFR-Test-5 first half). 25-host Cortex churn test (NFR-Scale-2 first half). 10-host mTLS chaos (NFR-Sec-13 follow-on). Sandbox-escape detection (NFR-Sec-3). SIEM export (NFR-Aud-11). Cross-form any-rust↔wasm (NFR-Test-7 follow-on).

**v2.5 (ecosystem):** FKCS-populated (NFR-Test-5 second half). 100-host Cortex churn (NFR-Scale-2 second half). RTL layout support (NFR-Doc-7 follow-on). Vetter ecosystem maturity. Multi-operator full implementation (NFR-Ops-11 follow-on).

These gates are the substrate's quality contract. They are non-negotiable at the named phase or the phase doesn't ship.
