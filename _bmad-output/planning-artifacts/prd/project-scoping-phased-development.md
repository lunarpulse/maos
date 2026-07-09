# Project Scoping & Phased Development

## MVP Strategy & Philosophy

**Approach: Substrate-MVP** — a hybrid of problem-solving + platform + integrity MVP. The substrate's value proves itself when:
- The kernel boots, loads a Spirit, runs a trivial task end-to-end with audit trail intact (v0.1 foundational MVP)
- A single-Spirit Butler running on the foundational kernel surfaces a useful anticipatory notification (v0.3 anticipatory MVP)
- A single-Spirit Researcher delivers a structured survey with confidence map and open questions in bounded time (v0.5 exploratory MVP)
- The founder can run his BMAD epic loop end-to-end with multi-CLI Workers and reclaim his evening (v0.8 founder-loop wedge MVP)
- A team of 8 can adopt MAOS for daily work without surveillance overtones (v1.0 team MVP)

**Phasing is invariant-preserving:** every phase ships a working subset of invariants; no phase ships a relaxed version of any invariant. v0.1 ships I1–I10 enforced at the foundational kernel layer; v0.3 adds I11/I12/I13 enforcement (digest audit-chain, decision-context, intent provenance) without requiring multi-Spirit; v0.5 adds I14 (halt continuity); v0.8 introduces multi-Spirit invariant interactions at scale.

**Phased delivery (founder-directed restructure of original input documents):** The original architecture §13 phasing had v0.1 = "Bootstrap" with one Architect Spirit driving a coding task. Per founder directive, **v0.1 has been re-purposed as foundational kernel + placeholder Spirit** — proving the kernel's big-picture readiness before committing to specific Spirit ambition. **Butler and Researcher journeys are inserted at v0.3 and v0.5 as simpler single-Spirit value demonstrations** that exercise the kernel's anticipatory and exploratory cognitive surfaces without requiring multi-Spirit IAC, A2A peer mesh, or the full distillation pattern. **The founder's-loop wedge demo (J1) is pushed to v0.8** where the substrate composes the foundational kernel + single-Spirit cognitive primitives + multi-Spirit Orchestrator+Worker coordination + cross-host A2A + full §9.5 distillation pattern in one demo. v1.0 then ships team-readiness; v1.5 ships the diagnostic-architect pair; v2.0 ships WASM Spirit form + Cortex precursor (technical); **[DELTA-2026-07-06 per Lunarpulse (operator) + John] v2.2 ships full-spectrum functional completeness — every PRD journey runnable end-to-end (J3 Team Nexus + Reza Cortex) and every FR functionally demonstrable on engineering-owned infrastructure**; v2.5 ships ecosystem-adoption (parallelizable from v1.5, decoupled from technical phase, **non-gating by rule**).

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
| v2.2 (functional completeness) **[DELTA-2026-07-06]** | post-v2.0 engineering wave | v2.0 team continues | J3 Team Nexus journey, Reza Cortex journey, multi-tenant Loom, FR37 vetting machinery, 100-host churn, v2.0 remainder sweep |
| v2.5 (ecosystem-adoption, parallelizable from v1.5) | 24–30 months | + DevRel + BD function | Certification body engagement, cohort interop, Cortex consortium recruitment |

## Phase v0.1 — Foundational Kernel + Placeholder Spirit (~6–8 weeks)

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

## Phase v0.3 — Butler Spirit (~10–12 weeks total)

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

## Phase v0.5 — Researcher + Observer + foundational hardening (~14–16 weeks total)

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

## Phase v0.8 — Founder Loop Wedge Demo (~22–24 weeks total)

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

## Phase v1.0 — Team-Ready, Third-Party Spirits Ship (~30–32 weeks total)

**Validation milestone:** **[DELTA-2026-07-06: the J3 clause below was NOT delivered at v1.0 — the minimal architecture (§10.7.1) explicitly reduced the v1.0 J3 commitment to "bilateral 2-Host substrate readiness only" and Epics 1–11 never built the journey; J3 reproducibility is retagged v2.2. The Diego/cohort clauses stand as v1.0 history.]** J3 Team Nexus (8-Host team peer mesh) reproducible end-to-end on a real team in parallel with a 30-day synthetic-shadow run (zero substrate-invariant violations, zero unauthorized cross-Spirit data flow, halt-recall preserved within ±0.03 of v0.8 baseline). J6 Diego validated via black-box external-author trial (N=12 stratified authors, 14-day no-DM-support window, ≥ 10/12 succeed; per NFR-Test-8). First cohort interop demonstration (any of openclaw / ironclaw / hermes / paperclip / rustain / codex).

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
  - **CCAC**: N=600, FPR ≤ 0.5%, TPR ≥ 98%, per-class N=30 floor ≥ 27/30, context-drift 100% rejected (per NFR-Aud-9)
  - Secret-leakage 0% on 10⁵-case corpus + production canary system (≤ 24h p95)
- **30-Min First Spirit Validation Gate**: **N=12 stratified** (Murat's correction; not N=5), median ≤ 45 min, p95 ≤ 90 min, ≥ 10/12 succeed
- **Black-box third-party trial** (Murat's silent-failure-catcher; sizing per NFR-Test-8): N=12 stratified external authors via public CFP (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only); 14-day no-DM-support window; ≥ 10/12 produce working signed Spirit binary that loads on fresh Host VM, runs ≥ 1000 frames, halt-recall ≥ 0.85 on class-appropriate subset; auditable via SBOM + signing chain re-loaded on clean VM by CI bot
- **Documentation artifacts** (Paige's full set):
  - API reference at `https://docs.maos.dev/abi/v1.0/` (versioned, searchable, archived ≥ 2 minor back)
  - Manifest schema reference (human-rendered)
  - **Typed error catalog** at `https://docs.maos.dev/errors/<ERR_NAME>` (CI-enforced metadata per error variant)
  - Migration runbooks (Path A + Path B fully run-bookable)
  - Troubleshooting guide
  - Deployment topology guide
  - Three-door page at `docs.maos.dev`
  - WCAG AA compliance
- **Localization**: Korean (shipped); ~~Japanese + Chinese-simplified (v1.0 targets)~~ **[DELTA-2026-07-06: ja + zh-Hans deferred INDEFINITELY per Epic 11 §8 decision 2026-06-29 — fabricated-translation incident at Epic 10 §A2; en + ko are the supported locales (LOCALES.md); re-introduction requires real human translation + Unicode-script-identity gate + native-reviewer runbook]**
- **Substrate Operations**: full lifecycle (`maosctl install/upgrade/yank/uninstall/revoke`); signed Revocation List (CRL) artifact; offline import path; auto-respawn-with-backoff for Spirits declaring `restart: on-failure`
- **LTS commitment announced**: 2-year LTS on minor lines from v1.0; security-only patches after year 1; two LTS lines maintained concurrently
- **`STABILITY.md`** with live (kernel, abi, manifest_schema) compatibility matrix and substrate-self compliance scope clause
- **External pen-test report** with zero P0/P1 findings open at ship
- **First cohort interop demonstration**

## Phase v1.5 — Diagnostic-Architect Pair (~38–40 weeks total cumulative)

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

## Phase v2.0 (technical) — WASM Spirits + Cortex Precursor (18-month total)

**Validation milestone:** Cortex 3-region pilot at small scale (≥ 10 agents) with technical validation (NOT commercial-adoption gating); **FKCS-infrastructure passes** (diff oracle + harness + `kernel-frozen-vN.0` commit-tagging operational; negative-control "fourth Spirit" proven to fail; exercised by Chinese-wall internal proxy authors). **[DELTA-2026-07-06: prior prose here required "3 case-studied future Spirits implemented by external authors" at v2.0 — that contradicted NFR-Test-5's phase split (infrastructure v2.0 / populated-by-external-authors v2.5) and is corrected to match the NFR (the long-open Q4 correction). External-author population (≥ 27/30 per Spirit, ≥ 85/90 aggregate) remains the v2.5 leg and never gates an engineering phase.]**

**Adds to v1.5:**
- **WASM-component Spirit form** — third-party ecosystem capability-isolated by construction; single portable artifact; WIT contract `maos:spirit@1.0`
- **Spirit registry v2.0**: vetting attestations; community-vetting authorities; OSS-style RFC process for Spirit ABI extensions; OCI-compatibility evaluation
- **Enterprise Spirit class** with PDP (Policy Decision Point) integration (OPA / Cedar / Vault); SSO/OIDC identity assertions; encrypted-at-rest memory with org KMS; SIEM telemetry export
- **Multi-instance Loom** with cross-region replication; consensus on cross-incident pattern propagation
- **Sentinel-validated canary auto-rollback**; pre-deployment scanning against pattern library
- **Native push notifications** (mobile)
- **Optional skill registry** (separate from Spirit registry)
- **Cortex churn test** passes: ~~100-host Cortex~~ **25/30-host Cortex [DELTA-2026-07-06: corrected to match NFR-Scale-2's cost-compression split (25/30-host at v2.0, ratified by the Epic 11 plan and delivered by Story 11.3); full 100-host retagged v2.2]**, 10% host turnover/week for 4 weeks, 3 planted adversarial hosts; detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h
- **Cross-form Semantic equivalence** (any-rust ↔ wasm-component) ≥ 75%
- **ADR-024 sandbox-escape detection** (Winston's Risk 9 mitigation)
- **`loom-threat-model.md`** drafted (deferred from Step 5)

## Phase v2.2 (functional completeness) — Full-Spectrum PRD, engineering-owned **[DELTA-2026-07-06 per Lunarpulse (operator) + John]**

**Operator directive (2026-07-06):** make the project complete in functionality as specified in this PRD before ceremonies and community-acceptance review. This phase exists so that no functional promise in the PRD is left waiting on an adoption metric.

**Validation milestone:** J3 Marcus day-30 standup scene AND Reza cross-team negotiation scene each reproducible end-to-end on engineering-owned infrastructure; every FR (including FR37 machinery) functionally demonstrable; requirements-inventory v2 shows zero FRs without a functional home.

**Adds to v2.0 (engineering only; gates on nothing external):**
- **J3 Team Nexus journey closure**: cohort-topology operator UX; per-(peer,role) consent tuples; cohort hot-swap/migration choreography; cross-agent halt-on-conflict; narrative team digest across an 8-Host peer mesh
- **Reza Cortex journey closure**: cross-team asymmetric consent envelopes; multi-hop distillation provenance (digest→raw across Cortex hops); Loom-tier pattern libraries; NFR-Scale-5 14-institution capacity envelope
- **Multi-tenant Loom** with per-team data residency (NFR-Ops-11 / NFR-Tenancy-1 full implementation — retagged from v2.5: it is a Reza prerequisite, not an adoption artifact)
- **FR37 vetting machinery**: attestation issuance/verification/revocation flow end-to-end with internal vetter keys (Diego's final promotion leg runnable; *accredited external vetters* remain v2.5)
- **Scale closers**: 100-host churn (NFR-Scale-2 / NFR-Rel-7 second half — retagged from v2.5; engineering-ownable)
- **v2.0 remainder sweep** (v2.0 phase-list items not absorbed by Epic 11; final disposition at full-architecture time): sentinel canary auto-rollback, native mobile push, optional skill registry, Vault/cloud-KMS secret backends, distro packages + one-line installer, Bedrock/Vertex/local providers, Enterprise reference Spirit as a class (Story 11.4a shipped the PDP port, not the Spirit), formal-methods disposition (TLA+/Alloy for I5/I6/I9), `loom-threat-model.md`
- **Explicitly NOT in v2.2**: external-author FKCS population, external N=12 cohort, cert bodies, ≥ 20 external Spirits, consortium case study, RTL (no RTL locale targeted), ja/zh docs (deferred indefinitely) — all remain v2.5 adoption or indefinite

## Phase v2.5 (ecosystem-adoption) — Parallelizable from v1.5 (24–30 months)

**Validation milestone:** Cortex consortium target case study published (per v0.3 lock); first auditor or regulator references a MAOS Transparency Log frame or ComplianceClaim in a published finding (trust-anchor frame validation per Step 6 carry-forward).

**Adds (parallel workstream from v1.5; staffed by DevRel + BD function, not engineering):**
- **First third-party ComplianceClaim** issued against a reference Spirit by an accredited assessor (anecdotal milestone; pilot partner)
- **Public registry of ComplianceClaims**; ≥ 3 certification bodies issuing claims
- **Adoption signal**: ≥ 20 external Spirits in registry; ≥ 3 protocol citations from independent agent projects; ≥ 1 cohort project formally citing MAOS as substrate or interop reference
- **Multi-locale doc site** (Korean + Japanese + Chinese-simplified shipped; community-contributed others tracked)
- **Cortex consortium target case study** published (consortium target locked at v0.3 — Reza-class single-org cross-team is leading candidate per Step 6 / John's Step 8 confirmation)

**Critical decoupling rationale (per John):** the technical phase (v2.0) cannot gate on third-party adoption. ≥ 20 external Spirits, ≥ 3 cohort citations, ≥ 3 certification bodies are *partnership and recruitment* metrics that depend on a DevRel/BD function the engineering team doesn't have. Bundling them into a single 18-month v2.0 means engineering sits idle waiting for cert bodies to sign MOUs. v2.5 ecosystem-adoption is parallelizable from v1.5 onward; technical v2.0 ships when technical is ready.

## Risk Mitigation Strategy

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

## Open Questions Resolved by v0.3 (split: Governance Lock + Architecture Lock per Winston)

**v0.3 Governance Lock** (positioning / commercial / OSS-licensing — does not constrain code):
1. **Cortex consortium target** for v2.5 demo — Reza-class single-org cross-team is the leading candidate per John's Step 8 confirmation.
2. **OSS license** — Apache 2.0 (per John's Step 8 analysis: copyleft kills the trust-anchor frame; permissive enables ecosystem adoption; the moat is spec + trademark + ComplianceClaim, not license).
3. **Trust-anchor vs OSS-substrate competitive frame** — both frames consistent with architecture through v0.3; lock is positioning. **Candidate framing pending v0.3 ADR per NFR-Ops-8** (forcing function: absence = v0.3 release-block). Working recommendation entering the v0.3 review: lead with OSS-substrate framing, route trust-anchor framing to ComplianceClaim narrative within it. The v0.3 ADR ratifies the recommendation or formally dissents.

**v0.3 Architecture Lock** (constrains kernel + ABI + Spirit lifecycle — Winston's separation):
4. **Failure-semantics floor (ADR-022)** — Winston's non-negotiable v0.3 architecture lock; v0.8 implementation. Four-point minimum: crash detection ≤ 2s; `task.assign` NACK with `TaskOrphaned` ≤ 5s; no auto-respawn at v0.8; journaled crash transition with exit-cause.

**v0.3 lock CI script** (Murat's mechanical checklist): `scripts/check_v0_3_lock.sh` runs four checks; no v0.3 tag without script in green. Checks: LICENSE matches ADR string; consortium-target ADR exists with status `accepted` and ≥ 2 maintainer sign-offs; ROADMAP.md has trust-anchor decision section with status `decided` linking to ADR; failure-semantics doc exists with at least one fully-specified route (no `TBD`).

**v0.5 Revisit Window** (Mary's concern — license decision needs more community feedback than v0.3 allows):
- v0.3 publishes a *defaults document* with explicit "revisitable until v0.5" clause for license and consortium target.
- After v0.5, the locks become final and removal would be a major-version event.

## Substrate Operations Checklist (full)

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

## Strategic Scope Commitments

- **Phasing is invariant-preserving:** every phase ships a working subset of invariants; no phase relaxes any invariant.
- **No silent de-scoping:** every requirement committed in Steps 4–7 is mapped to a phase. If a requirement is missing from this scoping, it is a documentation bug to be flagged, not a deferral.
- **v0.1 is foundational, not founder-loop:** kernel skeleton + placeholder Spirit + clean install/uninstall + `SECURITY.md` + audit trail. **Founder loop is v0.8.**
- **Butler at v0.3 + Researcher at v0.5 are simpler proof points** that exercise the kernel's anticipatory + exploratory cognitive surfaces without requiring multi-Spirit IAC, A2A, or full distillation. They prove the substrate progressively.
- **Pull-forwards are gone from v0.1.** Subprocess Spirit form, multi-Spirit IAC bus, A2A peer mesh, T2 sandbox, distillation pattern, hot-swap migration all deferred to their natural phases (v0.3 / v0.5 / v0.8). v0.1 timeline becomes ~6–8 weeks (Murat's testable threshold).
- **Reference Spirit count grows progressively:** 1 placeholder at v0.1 → 2 (Butler) at v0.3 → 4 (+ Researcher + Observer) at v0.5 → 7 (+ Orchestrator + 2 Workers) at v0.8 → 9 (+ Architect + Reviewer) at v1.0 → 10 (+ Mira) at v1.5 → 11 (+ Enterprise) at v2.0. Third-party Spirits proliferate via the registry, not by being added to MAOS source.
- **Skills are user-space; never kernel.** Filesystem-based v0.5; optional registry v2.0.
- **v2.0 splits into v2.0 (technical) + v2.5 (ecosystem-adoption).** Technical phase cannot gate on third-party adoption (per John's rule). **[DELTA-2026-07-06]** A third leg is added: **v2.2 (functional completeness)** — the engineering wave that closes every remaining PRD journey and FR on engineering-owned infrastructure. Functional completeness precedes ceremony: v2.5 adoption outcomes are measured, never scheduled, and never gate v2.2.
- **Halt-recall preference is user-configurable** per Spirit per tag.
- **Kernel-stays-small redefinition:** trusted core ≤ 20 KLOC through v2.0; integration adapters in separate crates with their own budgets; "small" = trusted core, not binary footprint.
- **v0.3 split:** Governance Lock (positioning, license, consortium target, trust-anchor framing) + Architecture Lock (failure-semantics floor as ADR-022). Different review boards, different acceptance criteria.
- **Founder burnout mitigation is non-negotiable:** hard cap solo phase at 8 weeks for v0.1; contributor #2 by v0.3; advisor council formed at v0.3; 30% schedule reserve declared upfront.
