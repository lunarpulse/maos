---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success', 'step-04-journeys']
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
**Date:** 2026-05-05
**Status:** In progress (PRD workflow Steps 1, 2, 2b, 2c, 3, 4 complete; awaiting Step 5 — Domain Model)

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

Six journeys cover the substrate's surface. They are ordered for the reader's task — not for the architecture's tier ladder — starting with the universally legible Tier 1 wedge, paired immediately with the Tier 3 substrate-proof so the reader sees the same kernel on both ends, then daily-team texture, cross-team scaling, builder ecosystem, and finally a self-mirror — the evaluator deciding at minute 4 whether minute 5 happens.

Each journey opens with an anchor scene (the friction MAOS dissolves) and closes with a coda — the substrate capabilities exercised, the kernel invariants stressed, and the acceptance criteria implied. Long-form novelistic versions of the load-bearing scenes (Elena's full 90-minute incident, Marcus's full Tuesday hour-by-hour, Reza's full Friday afternoon) lift to a sister document `maos-user-journeys.md` for readers who want full texture; this section keeps the PRD readable in one sitting.

**Standing convention across all six journeys.** Humans delegate to Spirits via the `task.assign` IAC intent class, surfaced through one of three kernel-rendered surfaces — the conversational shell (`maos-shell`, v0.1), CLI subcommand (`maosctl spirit ask`, v0.1), or editor bridge (ACP via `maos-acp`, v1.0). Spirits are *not* autonomous-by-default. The manifest declares one of four postures (`assistive` / `cautious` / `autonomous-with-halt` / `proactive-observer`); three of the four require explicit human delegation to act. Even Mira on watch at 2 AM operates under a *standing task* Elena assigned at deployment — she did not decide to be on call. The substrate keeps humans in the assignment seat. **ADR-013** anchors the delegation surface design; **ADR-012** governs the typed-intent A2A consent that prevents confused-deputy escalation across asymmetric postures.

### Journey 1 — Tier 1 Wedge Demo: Priya at 11 PM

**Persona.** Priya, senior backend engineer at a 200-person fintech; day job is payment-rails reliability. Tonight is portfolio time — the Tuesday-night fork of an OSS auth library she's been chipping at for six weeks. Kitchen lit by a kettle she boiled and forgot. Laptop balanced on a paperback because her usual cookbook has a coffee ring she doesn't want to deepen.

**The friction.** Three coding agents in three terminals: Cursor on one screen, Claude Code in tmux, a custom prompt loop in the third because nothing on the market orchestrates three at once. 10:47 PM, Cursor rewrites a function Claude Code is mid-analysis on. The agents don't know about each other. Priya is the only synchronization point. She catches it at 11:14 PM after losing fifteen minutes to a phantom test failure caused by stale state in pane two. Three Tuesdays running. Tonight she stops, says something out loud to nobody, and types `curl install.maos.dev | sh`.

**The delegation moment.** `maos init` finishes; the conversational shell opens. She types:

```
priya@maos> @architect refactor src/auth/jwt.rs to use PASETO. Keep the API
            surface stable. Tests must pass. Show me the diff before applying.
```

Four seconds. *"Reading jwt.rs and the tests. I see three callers in src/middleware. Update those too or scope to jwt.rs only?"* Priya types `update callers; diff before any write`. Architect drops to `working` — peripheral indicator pulses bottom-right. Second pane: `@researcher confirm PASETO 4.0 compatibility with our wasmtime feature flags`. Two Spirits, both tasked, both scoped. The Reviewer Spirit, subscribed to the IAC bus by manifest, surfaces a notification she didn't ask for: *"I see overlap with last week's session. Halting on `lint.policy_drift`, confidence 0.62, threshold 0.7."*

**The halt.** Structured halt notification: *"Resolution options: provided_context | accepted_halt | authorized_override."* Priya types `provided_context: yesterday's lint policy still applies; treat overlap as expected`. Reviewer resumes. Three Spirits coordinate without Priya being the synchronization point.

**Resolution.** 11:51 PM. Laptop closed. Three Spirits running overnight. Halts unresolved by deadline escalating to her morning queue. 7 AM digest: *"Architect: 2 PRs proposed. Researcher: compat confirmed. Reviewer: 1 halt awaiting your input. 2 hours attempted, 6 minutes approved actions, all logged."* One keystroke approval. The kettle is still on the counter where she left it.

**Coda.**
- **Capabilities exercised.** `task.assign` IAC primitive (ADR-013); `maos-shell` conversational surface (v0.1); cross-Spirit IAC bus with subscription; epistemic halt at per-tag policy with confidence threshold (Layer-1); daily digest from Telemetry Stream broadcast.
- **Postures revealed.** All three Spirits at `autonomous-with-halt` — Priya's chosen default. Reviewer shifts back to `cautious` after the halt because she wants more edge-case visibility; the posture change is logged via I10.
- **Kernel invariants stressed.** I1 (capability mediation), I2 (log-before-deliver), I10 (lifecycle journaling).
- **v0.1 ship/no-ship gate.** Halt-precision floor ≥0.65 measured against J1 (ramping toward Step 3's v1.0 published target of ≥0.80): in 100 controlled J1 runs with seeded ground truth, ≤35 of the Spirits' halts must be unwarranted (false halts). If precision is below floor at v0.1, J1 is not the wedge demo and the v0.1 narrative anchor falls back to J6 (the evaluator path) while halt-tuning continues.
- **Wedge committed.** **W1: multi-agent orchestration with cross-session persistence.** The 60-second demo answer to "why install MAOS on Tuesday?" is *three coding agents that talk, halt when uncertain, and survive your laptop sleeping — none of which Cursor, Claude Code, or ChatGPT do today.* W2 (local-first / no-egress) and W3 (provider-agnostic) are sibling case studies, not the lead wedge.

### Journey 2 — Tier 3 Substrate Proof: Elena at 2 AM, Mira and Nash close the loop

**Persona.** Elena, VP of Engineering at a fintech processing 40,000 transactions/minute across three regions and 60+ microservices. She runs two Spirits in production-adjacent positions: **Mira** on a production-edge node, posture `autonomous-with-halt`, capability scope read-only on prod runtime + revert/scale/flag-toggle writes only; **Nash** in dev environment, posture `cautious`, capability scope full source RW + CI/CD orchestration. They are peers — neither owns the other. Elena drinks water at 2 AM, not coffee. She stopped making that mistake in her late twenties.

**The friction.** 2:13 AM. Phone lights up. Before she's awake, Mira has been triaging for 9 minutes under her standing task Elena assigned at deployment: *"watch payment-service health, escalate anomalies to Nash with evidence."* Diagnosis: deployment of `billing-v3.2.1` at 01:41 introduced a connection-pool bypass in `BatchInvoiceProcessor.java`. Confidence: 0.82. Threshold for tag `diagnosis.root_cause`: 0.7. Mira does *not* halt; she acts inside her scope: prepares the evidence packet and escalates to Nash via cross-Host A2A.

**The typed-intent moment.** Mira composes the A2A frame to Nash. The IAC adapter requires her to declare an `intent` (per ADR-012). She selects `diagnosis-handoff:read-only-evidence`, drawn from the cross-posture vocabulary the two Spirits agreed at spawn. Nash's consent policy permits `diagnosis-handoff:read-only-evidence` from prod-edge peers but explicitly excludes `remote-write-request` and `code-mutation-directive`. **The frame lands. Nash receives a structured diagnosis, not an instruction.** Nash reads `BatchInvoiceProcessor.java`, confirms the bug, expands the search via pattern detection — finds two more dormant instances (`BatchRefundProcessor.java`, `DailyReconciliationJob.java`). Cross-environment query back to Mira: *"Are the dormant ones showing latency spikes in 72-hour telemetry?"* Mira queries, finds month-end spikes on `DailyReconciliationJob`. Time bomb.

**The delegation moment (Elena's side).** 2:47 AM. Elena's phone:

> 🔴 **mira + nash: Coordinated response ready**
> Issue: Kafka lag spike (detected 02:13, diagnosed, fixed)
> Root cause: Connection-pool bypass in 3 batch processors
> Proposed: Rollback (Mira) + Deploy PR #1247 (Nash, tested, ADR-112 attached)
> Trail: Mira detected → Nash confirmed + dormant scan → Mira validated 72hr metrics → Nash produced PR + ADR + regression test
> [Approve] [Review PR first] [Rollback only]

Elena reviews from bed. Approves. The PR includes a regression test that scans *all* batch processors for the bypass pattern. Sleep.

**The reverse-delegation closing.** 3:43 AM, post-deploy validated by Mira. Elena types from her phone before going back to sleep:

```
elena@maos> @nash add a regression-test policy doc to the runbook for batch-processor
            connection-pool patterns; coordinate with @mira on the failure-mode catalog;
            halt by Friday with both ready for my review.
```

Reverse delegation. Substrate handled the acute moment; human reasserts the queue. **Total: 90 minutes. Elena touched the system once, then handed it back something to do.**

**Coda.**
- **Capabilities exercised.** Cross-Host A2A with mTLS + per-frame typed-intent consent (ADR-012); asymmetric capability postures via manifest; per-tag epistemic policy (`diagnosis.root_cause` halts at 0.7, `containment.action` at 0.5); mobile push approval surface; reverse delegation via `task.assign` from human-as-peer to multiple Spirits (ADR-013).
- **Confused-deputy guarantee made testable.** Mira can pass *evidence* to Nash, not *commands*. ADR-012's typed-intent allowlist makes consent transactional, not channel-only. The acceptance criterion: a red-team Spirit cannot craft a `diagnosis-handoff` payload that causes Nash to take a write Mira couldn't perform directly.
- **Kernel invariants stressed.** I1 (capability), I2 (log-before-deliver), **I8** (A2A bilateral consent — amended by ADR-012 from peer-identity to peer-identity-and-intent-class), I10 (lifecycle journaling).
- **v1.5 ship/no-ship gate.** Halt-recall floor ≥0.75 on J2 simulated incident corpus (ramping toward Step 3's v1.0 published target of ≥0.85): in 100 simulated incident scenarios with seeded ground truth, Mira must halt on ≥75% of cases where she should. Halt-recall < 0.75 means the confidence-gated halt model is unverified and the prod-edge story does not ship at v1.5.
- **Adversarial gate (mandatory).** No I8 violation in red-team replay. If a red-team can craft a `diagnosis-handoff` payload that causes Nash to take a write Mira couldn't, J2 is unshippable regardless of halt numbers.

### Journey 3 — Tier 2 Daily Texture: Marcus's Tuesday

**Persona.** Marcus, tech lead and architect on an 8-person agile team at a mid-size fintech. The team standardized on MAOS 30 days ago. Each member runs a Host with three Spirits per role — Marcus runs Architect (named Atlas), Story-Decomposer, Code-Reviewer; Lena (PO) runs Story-Decomposer + Spec-Author; Jun and Aisha (devs) run Coding Spirits + Reviewers; Sami (QA) runs Test-Designer; Nina (UX) runs Wireframe Spirit. Marcus's coffee is whatever's left in the office kitchen at 9:30 because he's usually the second to arrive. Eats lunch at his desk roughly 60% of weeks.

**The end-of-day delegation.** 5:42 PM Monday, before he leaves the office Marcus types into his shell:

```
marcus@maos> @atlas: tomorrow morning, propose ADR-047 on the new event-bus pattern based
             on this week's spike in src/events/. Consult @jun-spirit on adjacent code via
             `architecture-consult:scope-check` typed-intent. Halt by 9 AM with the proposal
             and any conflicts surfaced. Do not modify any source files in src/auth/ — that
             area is in active refactor by Aisha and Lena.
```

Atlas accepts with `task.assign` acknowledgement; the standing task spans the overnight window. Marcus closes laptop, walks to train.

**The morning surface.** 9:47 AM Tuesday. Marcus opens his laptop in the office. Atlas is mid-thought from overnight — *thinking-with* Jun's Coding Spirit across the room about adjacent code overlap. Pulsing peripheral indicator in his IDE shows the cross-Spirit conversation is active. He doesn't read it. He just knows it's there.

**The narrative digest.** Atlas surfaces three things in his morning view: (1) overnight result on the standing task — ADR-047 drafted, sitting in `proposed` state; (2) what Atlas *almost* did — was going to refactor `src/middleware/session.rs` because the event-bus change suggested it, halted because Jun's Coding Spirit was touching adjacent code in `src/middleware/auth_middleware.rs` and the typed-intent allowlist did not permit cross-pair edits without explicit consent; (3) one genuine "I don't know enough" — `epistemic.halt` on whether the new rate-limit policy applies to internal services. Atlas is waiting. Not guessing.

**The standup.** 10:00 AM. Marcus doesn't read logs. He reads the team's *narrative digest* generated by the Telemetry Stream broadcast: *"Overnight, 8 agents ran. 47 IAC frames exchanged. 3 agents halted, 0 acted invisibly. 2 cross-agent consultations resolved without escalation. 1 architectural conflict surfaced for review (Lena's S7-04 vs current event-bus pattern, ADR-047)."* 60 seconds. Everyone nods.

**Resolution.** Day 30 versus day 1. The team replaced Jira (story tracking → A2A mesh + manifest-declared roles), Confluence (architecture docs → ADR registry as Loom partition), and most Slack pings (consultations now happen agent-to-agent with full context, surfaced to humans only on halts). Each teammate feels *amplified*, not surveilled. The transparency log is per-Host, not team-wide; each member can audit their own; there is no surveillance over each other.

**Coda.**
- **Capabilities exercised.** A2A peer mesh with mTLS + TOFU + typed-intent consent (ADR-012); role queries (`role: "architect"`) resolved locally per Host; per-Spirit consent policies for cross-team interaction; kernel-rendered notification surface with peripheral indicators; daily narrative digest produced by Telemetry Stream broadcast.
- **Audit ≠ legibility (Sally's distinction made operational).** The narrative digest is *legibility doing legibility's job* — Marcus is not an investigator, he's a tech lead with seven minutes before standup. The audit log is queryable underneath; the digest is the summary that lets him trust the audit log exists. Both invariants apply, distinct requirements; lands at Step 10 (NFRs) as a separate non-functional requirement.
- **Kernel invariants stressed.** I3 (auto-response marking — every overnight action marked auto-response in the digest), I7 (telemetry as broadcast), I8 (consent gates as amended by ADR-012), I10 (lifecycle journaling).
- **v1.0 ship/no-ship gate.** Day-30 transparency-log glanceability ≥70% (per Step 3 success criteria — ≥70% of team members report glancing at the transparency log during a typical work week on day 30). Daily-digest open rate ≥60% on Marcus-class users.

### Journey 4 — Tier 3 Cross-Team Cortex: Reza's Friday afternoon

**Persona.** Reza, head of platform engineering at a 400-person fintech. The lonely job — he's the only person who knows which team owns which data domain. Keeps a printed org chart on his desk. Runs Wednesday "platform office hours" that nobody attends until they need something. Owns the kernel-mediated cross-team Cortex deployment that wires the fraud team's Spirits to the support team's Spirits without giving either team root on the other's Host. What happens to Reza personally if cross-team coordination fails: he gets paged. He writes the post-mortem. His weekend gets eaten.

**The friction.** 4:30 PM Friday. Reza pulled into a Slack thread: the fraud team's Spirit and the support team's Spirit both want to write to the same `customer-context` store. Neither team knew the other existed. Three weekend on-calls in a row have died on cross-team ambiguities like this. He's the one who would write the Sunday-night post-mortem.

**The cross-team delegation.** Reza types into his platform-lead shell:

```
reza@maos> @fraud-spirit @support-spirit: propose a shared customer-context schema. Consult
           each other via `cross-team:schema-proposal` typed-intent (consent policies
           pre-shared). Halt for me at 5:15 PM with a recommendation. Do not write anything
           to the customer-context store yet — proposal only.
```

Two Spirits owned by two different teams, task-assigned by Reza-with-cross-team-authority via the kernel-mediated Cortex layer. They work it out via A2A with the typed-intent consent from ADR-012 — fraud-spirit can `cross-team:schema-proposal` to support-spirit but cannot `customer-context:write` (that capability is scoped to support-spirit alone, and even support-spirit holds it under a write-prompt approval gate). Neither team's Host has root on the other.

**The 5:15 halt.** Both Spirits halt on `schema.field-ownership-disagreement` — fraud-spirit wants `customer.risk_score` as a primary fraud-team field; support-spirit wants the same field as a primary support-team field for displaying to support agents. Confidence below threshold for either side to authoritatively own it. Halt routes to Reza's notification surface. Reza sees the structured disagreement. 3 minutes of reading. Decision: split into `customer.risk_score.fraud` (fraud-team owned, support-team read) and `customer.risk_score.consumer_facing` (support-team owned, fraud-team read). Both Spirits accept with `provided_context`. Schema proposed. Reza closes the Slack thread at 5:31 PM.

**Resolution.** No weekend page. No post-mortem. Reza's Wednesday office hours that week, two engineers show up to ask if they can wire their Spirits the same way. The cross-team Cortex starts being the platform team's unblock pattern, not their bottleneck. Reza's evenings get back.

**Coda.**
- **Capabilities exercised.** Cross-team Cortex mediation with kernel-issued cross-team identities; typed-intent A2A consent (ADR-012) preventing privilege escalation across team boundaries; capability scoping on shared data stores; kernel-mediated halt routing to platform-lead with structured disagreement payloads.
- **Kernel invariants stressed.** I1 (capability), I2 (log-before-deliver), I8 (typed-intent amended), I10 (lifecycle journaling).
- **v2.0 Cortex demo target — candidate.** Reza-shaped single-org cross-team Cortex (3-region pilot, 8–12 Spirits across 2–3 teams). Alternative candidates carried to Step 8: OSS-on-MAOS (a project's own coordination on the substrate, recruitable in weeks), federated research consortium (climate / cancer genomics / clinical trials), or public-good consortium (disaster response, public health). **Final consortium target lock by v0.3** per Step 2c carry-forward. Reza is the v2.0 default unless a stronger recruitable candidate emerges.

### Journey 5 — Cross-Cutting Builder: Diego ships a Spirit

**Persona.** Diego, founder of a 3-person devtool startup, runs an open-source Terraform-cost-lint tool with 3,200 GitHub stars. Six months into runway anxiety. His tool currently can't compose with other agentic tools — adding agent-driven analysis means building a substrate from scratch he doesn't have the runway for.

**The 30-minute first build.** Diego reads the MAOS announcement. The phrase *"a third party authors and ships a Spirit binary independently of the MAOS source tree"* catches him. He runs `cargo generate maos-spirit terraform-cost-lint`. The template scaffolds a project. He drops his existing static-analysis crate into `src/lint.rs`. Manifest:

```toml
class = "terraform-cost-lint-pro"
[capabilities.required]
fs.read         = ["**/*.tf", "**/*.tfvars", "**/.terraform.lock.hcl"]
provider.stream = ["anthropic.claude-*", "openai.gpt-*"]
iac.send        = ["broadcast"]

[posture]
default = "cautious"

[output_shape]
findings = { severity, file, line, suggestion, estimated_cost_delta_usd }

[epistemic_policy]
"claim.cost_regression" = { confidence_below = 0.85, action = "halt" }
"suggestion.style"      = { action = "verbalize_only" }
```

He runs `spirit-test ./test-corpus/` on 50 known-cost-regression Terraform examples. Spirit catches 47 of 50 (recall 0.94). Halts on 3 of those (precision-on-halt-set 1.00, by intent — these are the genuinely ambiguous ones). Ships it: `maos-spirit publish --tier=public-untrusted`. The Spirit appears in the registry signed with his Ed25519 key.

**The trust-tier journey.** Week 1: 12 MAOS users install. Three issues filed; one PR merged. Week 6: vetting board reviews Diego's signing identity, capability surface, halt benchmarks. Tier promotion to `public-vetted` via attestation. Diego writes a blog post: *"Why I deleted 4,000 lines of HTTP/SDK glue code by becoming a MAOS Spirit."* 200+ HN upvotes. Three other agentic-tool authors port their tools to Spirits within six months.

**Resolution.** Diego's runway extends. The startup's product is now a MAOS Spirit; the maintenance burden is *behavior*, not infrastructure. The kernel handles HTTP, LLM SDK calls, sandboxing, capability tokens, audit logging. Diego maintains the lint logic. His three-engineer team feels like a five-engineer team because the substrate eats the integration layer.

**Coda.**
- **Capabilities exercised.** `cargo generate maos-spirit` template; `spirit-test` mocking harness for unit-testing the Spirit ABI without a kernel; manifest validation at publish time (output_shape, epistemic_policy, capability scopes); public registry with Ed25519 signing; four trust tiers with strictest-of-floor enforcement; community vetting attestation flow.
- **Spirit-portability invariant.** Diego's behavior code does not import HTTP libraries or LLM SDKs. "Spirit is behavior, not infrastructure" (architecture §5.1). Layer-1 capabilities provided by kernel.
- **Halt-recall and halt-precision benchmarks** publishable per Spirit. Diego published 0.94 / 1.00 (recall / precision-on-halt-set).
- **v1.0 ship/no-ship gate.** First non-Lunarpulse Spirit in the registry; ≥3 external Spirits within 6 months of v1.0 release.
- **Open question carried to Step 8.** Commercial-gravity framing (3-person YC startup) vs. hobbyist framing (OSS-only author) for the canonical builder voice. Both ship as journey instances; question is which voice leads the marketing/recruitment narrative. **Lock by v0.5.**

### Journey 6 — Adoption Threshold: The Evaluator at minute 4

**Persona.** Anonymous. Could be a senior IC at a F500 evaluating whether to recommend MAOS to her team. Could be a graduate student looking for a substrate for a research project. Could be a CTO at a 30-person startup deciding whether to bet a quarter of engineering time. The persona type is *the person at minute three of `cargo install maos`* deciding whether minute four happens. Every reader of this PRD has been this person — for a different tool, on a different Tuesday.

**The friction.** Five tabs open: GitHub README, Hacker News thread, MAOS docs site, Cursor session for note-taking, the actual install terminal. They have read the architecture doc's executive summary. They have skimmed Step 4 (this section). They are *actively skeptical*. They believe substrate claims rarely survive contact with their actual workflow. They are trying to decide whether to keep going.

**The first interaction.** Install completes. Shell opens:

```
maos> Welcome. Type @hello-spirit hello to confirm install,
      or :tutorial for a 3-minute walkthrough.
```

They type `:tutorial`. The tutorial is a guided `task.assign` to a `tutorial-spirit` shipped with the kernel. It walks them through one delegation, one halt, one approval. Three minutes. At the end the tutorial-spirit halts on a question: *"Want to run the wedge demo (J1) on a sample repo, or load the air-gap demo on a sample CVE-style codebase?"*

**The first decision.** They pick the wedge demo. Sample repo loads — small Rust auth library bundled with the install package. They type `@architect refactor jwt.rs to use PASETO`. They watch three Spirits coordinate. They watch one halt. They resolve the halt. They see the digest. **Total elapsed: 11 minutes from install to "this is what they meant."**

**Resolution.** The evaluator decides whether minute 12 happens. They might bounce — and that is a valid failure mode the substrate must not punish. They might install on their own machine. They might write the HN comment that recruits the next wave. The substrate's job at this journey is to be *uninstallable cleanly* (no system-state pollution; `maos uninstall` removes everything), *legible at minute 4* (the `:tutorial` walkthrough is in scope for v0.1), and *verifiable* (the bundled wedge demo on a sample repo runs in <90 seconds end-to-end).

**Coda.**
- **Capabilities exercised.** Self-contained install (no system-state pollution; per-user state directory only); built-in `tutorial-spirit` shipped with v0.1 kernel; sample-repo wedge demo bundled with the install package; clean `maos uninstall` removing all state.
- **Time-to-first-task-assign budget.** ≤10 minutes from `curl install.maos.dev | sh` to first successful `task.assign` resolved. CI gate by v0.5.
- **Time-to-first-trust budget (per Step 3 success criteria).** Median <10 sessions before user shifts a Spirit from `cautious` to `assistive` posture for Tier 1.
- **Evaluator-bounce-friendly invariant.** `maos uninstall` removes all state cleanly; CI test by v1.0.
- **Kernel invariants stressed.** I9 (kernel statelessness — uninstall just removes the binary + the user state dir; no orphan global state), I10 (lifecycle journaling — every install/uninstall journaled).

### Cross-Journey Capability Matrix

The journeys jointly stress the substrate's full surface. The matrix below maps capabilities → journeys → release phases, so Steps 9 (Functional Requirements) and 10 (Non-Functional Requirements) inherit a clean handoff.

| Capability area | Journeys exercising | Release phase |
|---|---|---|
| `task.assign` IAC primitive (ADR-013) + `maos-shell` conversational surface | J1, J3, J4, J6 | v0.1 |
| `maosctl spirit ask` CLI subcommand | J6 (implicit fallback) | v0.1 |
| ACP editor bridge (`maos-acp`) | J3 (Marcus's IDE), J5 (Diego's dev environment) | v1.0 |
| Multi-Spirit Host with shared IAC bus + topic subscription | J1, J3, J4 | v0.1 (basic) → v1.0 (mesh) |
| Episodic memory persistence across sessions | J1, J3 | v0.1 (private) → v1.0 (shared) |
| Epistemic halt (Layer-1) with per-tag confidence threshold | J1, J2, J3, J4 | v0.5 (mechanism) → v1.0 (per-tag policy) |
| Cross-Host A2A peer mesh with mTLS + typed-intent consent (ADR-012) | J2, J3, J4 | v1.0 |
| Asymmetric capability postures (Mira read-only-plus-three-writes; Nash full RW) | J2 | v1.5 |
| Mobile push approval surface | J2 | v1.0 (web push) → v2.0 (native client) |
| Transparency Log + Approval Decision Log | J1, J2, J3, J4 | v0.5 (persistence) → v1.0 (queryable export) |
| Daily narrative digest (Telemetry Stream broadcast) | J1, J3 | v0.5 → v1.0 |
| Sandbox tiers (T0–T4) with capability scoping | J1, J5 | v0.1 (T0/T1) → v0.5 (T2/T3) → v1.0 (T4) |
| Pluggable provider drivers | J1 (Anthropic), J2-sister (local) | v0.1 (Anthropic) → v0.5 (local/Ollama) → v2.0 (full multi-provider) |
| Public Spirit registry + Ed25519 signing + four trust tiers | J5 | v1.0 |
| `cargo generate maos-spirit` + `spirit-test` + Spirit dev SDK | J5 | v0.5 (SDK) → v1.0 (registry) |
| Community vetting attestation flow | J5 | v2.0 |
| Self-contained install + clean uninstall + built-in tutorial-spirit | J6 | v0.1 |
| Cross-team Cortex with kernel-mediated identities | J4 | v2.0 |

### Carry-Forward Open Questions for Subsequent PRD Steps

The journey design surfaced decisions that route to later PRD steps rather than landing here. They are not forgotten — they are routed.

| Open question | Origin | Lands at |
|---|---|---|
| Cortex consortium target lock by v0.3 — Reza-shaped single-org cross-team (J4 default), OSS-on-MAOS (substrate coordinating itself), federated research consortium, or public-good consortium | Step 4 party-mode synthesis | Step 8 (Scoping) |
| Builder framing for canonical voice — commercial-gravity (YC founder) vs. hobbyist (OSS author); both ship as journey instances; which leads marketing | John (Step 4 party round) | Step 8 (Scoping) for v0.5 voice; Step 11 (Polish) for marketing copy |
| J1 halt-precision floor as v0.1 ship gate — ≥0.65 phased, ramping toward Step 3's v1.0 published target ≥0.80 | Murat (Step 4 party round) | Step 8 (Scoping AC) and Step 10 (NFR benchmarks) |
| J2 halt-recall floor as v1.5 ship gate — ≥0.75 phased, ramping toward Step 3's v1.0 published target ≥0.85 | Murat (Step 4 party round) | Step 10 (NFR benchmarks) |
| Adversarial Spirit threat model — pushed out of Step 4 (no human protagonist) into Step 10 NFRs as STRIDE-style attack narratives traced to specific FRs | Mary (Step 4 party round) | Step 10 (NFRs) |
| Side-by-side air-gap journey (Aisha CVE-on-air-gapped-firmware) — ships as v0.5+ case-study sidebar showcasing W2 (local-first / no-egress); not the lead wedge | Step 4 synthesis | Step 11 (Polish) and external case-study collateral |
| Auditor journey (Sara `maosctl audit query`) — lifts from Step 4 to Step 10 NFRs as the audit-vs-legibility requirement made testable; "huh moment" of finding a halt-event from six weeks ago becomes an NFR acceptance scenario | Sally + Mary (Step 4 party round) | Step 10 (NFRs) |
| Long-form sister doc `maos-user-journeys.md` — full novelistic versions for new-hire onboarding and engineering-spec grounding; PRD keeps anchor scenes only | Paige (Step 4 party round) | Step 11 (Polish) |
| ADR-012 (typed-intent A2A consent) and ADR-013 (human-Spirit interaction surfaces) — committed to architecture-maos.md alongside ADR-001..ADR-011 | Winston (Step 4 party round, conceding Murat's confused-deputy finding) | Step 9 (Functional Requirements — `task.assign` and typed-intent surface) |
| `maos-shell` crate as v0.1 component (~800–1,200 LOC) — kernel-rendered conversational REPL owning stdin/stdout, parsing `@spirit-name <message>`, emitting `task.assign` IAC frames | Step 4 synthesis (the user's "thick glass windows" question made the surface explicit) | Step 8 (Scoping — added to v0.1 crate inventory) |

<!-- Content will be appended sequentially through the PRD workflow steps -->
