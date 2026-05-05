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

The substrate's promise is felt rather than read. Each journey below traces a specific user from pain to resolution, with the capabilities the kernel and reference Spirits must deliver explicitly named at the end. Six journeys cover the substrate's full surface: J1 anchors the Tier 1 wedge from the founder's own workflow; J4 and J3 cover production-incident response and team-normalcy; Reza covers single-org cross-team scale-out; Diego covers the third-party builder JTBD; J0 covers the unbought evaluator. Sequencing follows reader-task order — vocabulary first (J1), proof second (J4), normalcy third (J3), scale fourth (Reza), extension fifth (Diego), self-mirror last (J0). Long-form novelistic versions live in `maos-user-journeys.md` (sister doc); the PRD carries anchor scenes plus capability codas.

The journeys honor the carry-forward signals from Step 2c — the wedge pain commitment, the Tier 3 reframe to OSS / single-org-Cortex, the audit-vs-legibility distinction, the eight kernel guarantees enumeration, and the halt-recall/halt-precision benchmark routing — and they exhibit the architecture decisions reached during this PRD workflow: ADR-012 typed-intent consent, ADR-013 `log.recall`, ADR-014 distillation audit-chain (I11), ADR-015 decision-context recording (I12), and the §9.5 distillation pattern.

### Journey 1 — Tier 1 wedge: The Founder's Loop (Lunarpulse runs Epic 7 from his daughter's bedtime to school drop-off)

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

- **Reference Spirit set v0.5+:** Orchestrator + Developer + Reviewer (3 skill packages, no per-CLI Rust crates). Worker Spirits are agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded. ADR-014 (use existing protocols, no new IAC MCP server).
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
- Per-tag epistemic policy at both Spirits with confidence-gated halts.
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

Six journeys collectively reveal the capability areas the kernel and reference Spirits must deliver:

| Capability area | Journeys | Phase |
|---|---|---|
| Multi-Spirit Host with shared IAC bus | J1, J3, J4, Reza | v0.1 (basic) → v0.5 (multi-Spirit + A2A pulled forward per ADR-014) → v1.0 (full mesh) |
| Episodic memory persistence across sessions | J1, J3, J4 | v0.1 (private tier) → v1.0 (shared tier) → v1.5 (collective tier via Loom) |
| Epistemic halt as Layer-1 capability | J1, J4, J0, Diego | v0.5 (mechanism) → v1.0 (per-tag policy) |
| Transparency Log + Approval Decision Log + `log.recall` | J1, J3, J4, J0 | v0.5 (persistence) → v0.5 (`log.recall` ABI per ADR-013) → v1.0 (queryable export) |
| Distillation pattern + I11 audit-chain + I12 decision-context | J1, J3, J4, Reza | v0.5 (kernel enforcement) → v1.0 (benchmarks per §9.5 four-metric suite) |
| Sandbox tiers (T0–T4) with capability scoping | All | v0.1 (T0/T1) → v0.5 (T2/T3) → v2.0 (T4 WASM) |
| Pluggable provider drivers (Anthropic / OpenAI / local / Bedrock) | J1, J4 | v0.1 (Anthropic) → v0.5 (multi-provider via CLI-wrapped agents) → v2.0 (full multi-provider proxies) |
| Cross-Host A2A peer mesh + ADR-012 typed-intent consent | J3, J4, Reza | v0.5 (pulled forward from v1.0) |
| Mobile-friendly approval surface (phone push) | J1, J4 | v1.0 (HTTP push) → v2.0 (native push) |
| Asymmetric capability postures (sre-diagnostician vs principal-architect vs platform-lead) | J4, Reza | v1.5 |
| Loom collective tier with cross-Host pattern propagation | Reza | v1.5 (Loom-lite) → v2.0 (multi-instance) |
| Public Spirit registry with Ed25519 signing + four trust tiers | Diego | v1.0 |
| `cargo generate maos-spirit` + `spirit-test` + Spirit dev SDK | Diego | v0.5 (SDK) → v1.0 (registry) |
| `maos audit query` CLI + cleanup | J0, J7-equivalent in NFRs | v0.5 (basic queries) → v1.0 (signed export) |

These map directly to Functional Requirements (Step 9) and Non-Functional Requirements (Step 10). The wedge pain commitment (W1″ — Orchestrator-led parallel multi-CLI execution with kernel-mediated audit-chain) is now load-bearing across J1 and Reza; Steps 8 (Scoping) and 9 (FRs) inherit it. Step 10 (NFRs) inherits the four-metric distillation ship-gate from §9.5 (digest-recall ≥ 0.90; faithfulness ≥ 0.98; hedge-preservation ≥ 0.95; traceability = 100%).

Open questions routed forward:
- **Cortex consortium target for v2.0 demo** — Reza-style single-org cross-team is the leading candidate; OSS-on-MAOS (Debian / Wikimedia / Apache Foundation) and federated research consortium are alternatives. **Final lock by v0.3** per Step 2c.
- **Adversarial-Spirit threat model** — Mary's stakeholder list for Step 10 NFR (STRIDE-style attack narratives with kernel defenses traced to FRs).
- **Halt-recall vs halt-precision floors per Spirit class** — uniform across distillation-shipping Spirits per §9.5; per-Spirit floors for non-distillation Spirits (J4 Mira: halt-recall ≥ 0.7; J1 Orchestrator: halt-precision ≥ 0.85). Routes to Step 10.

<!-- Content will be appended sequentially through the PRD workflow steps -->
