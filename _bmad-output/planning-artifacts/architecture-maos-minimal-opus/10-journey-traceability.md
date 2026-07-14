# 10. Journey Traceability

The substrate ships in support of **six user journeys at v1.5** (J0 Evaluator, J-Butler, J-Researcher, J1 Founder Loop, J4 Mira-Nash bilateral, J6 Diego). Each ships at a specific phase, with cited architectural primitives, FRs, and NFR floors. The substrate's architectural commitments are exactly the ones these journeys exercise.

**Two additional PRD journeys are deferred to future milestones (v2.0 / v2.x):** J3 Marcus Team Nexus (8-Host peer mesh) and Reza Cortex (single-org cross-team, multi-tenant Loom + WASM Spirit form). Their canonical scene descriptions remain in the PRD until the architecture revisions that support them ship; the v1.5 substrate is designed so neither journey requires architectural rewrite to add later. Full deferral notes including readiness analysis are in **§10.7 Deferred journeys (future milestones)**. **Update 2026-07-06:** both journeys are now **committed at v2.2** (operator-ratified functional-completeness phase); the architecture revision §10.7 anticipated is **§15** (`proposed-v2.2`, pending party-mode). §10.7's readiness analysis stands as written — its claims are what §15 cashes.

**§10.0 — Reading note for sequence flows.** Journey narratives below describe interactions between **Spirits** and the **kernel**. Per §4 the kernel is exactly *five services + two internal modules* (see §4.0.8 for the operational definition). When a narrative says "the kernel routes the frame," the routing is performed by the IAC Bus service; when it says "capability mediation," the work happens in the Capability Registry service; when it says "the Spirit observes telemetry," the subscription lives in the Telemetry Stream internal module. Sequence-level granularity (which kernel service did what, in which order) lives in the relevant service subsection — see §4.6.1 for halt sequencing, §7.2 for mTLS handshake choreography, §4.5 for IAC mailbox routing. §10 deliberately stays at the Spirit-↔-kernel layer to keep the journey legible; readers needing per-service step-by-step should follow the service-subsection cross-references inline below.

## 10.1 J0 — The Evaluator

**Persona.** Anonymous developer, four minutes into `cargo install maos`, deciding whether minute five happens.

**Phase.** v0.1 (the foundational gate).

**Architectural primitives exercised.** Kernel boot, lifecycle journal, capability tokens, single-Spirit subprocess form, `hello-spirit` placeholder Spirit, `maosctl` basic (`install`, `uninstall`, `audit query`, `spirit invoke`), clean uninstall, accessibility flags (`--plain`, `NO_COLOR`, `TERM=dumb`), `SECURITY.md` + disclosure pipeline.

**Acceptance.** From install completion to first useful Spirit response within 5 minutes. Honest capability disclosure on first interaction (the Spirit introduces what it *can* and *cannot* do). Audit-from-minute-1 (`maosctl audit query` works on the local Transparency Log). Clean uninstall (kernel is reversible; user's data persists or is removed per their choice).

## 10.2 J-Butler — Sandra's Butler reads the third 6 PM and reclaims dinner

**Persona.** Sandra, designer at a 30-person SaaS company. Figma + Linear + Slack + Google Calendar. Time-zoned across her team. Has a recurring 7 PM dinner with friends she keeps missing. Her partner is starting to take it personally.

**Phase.** v0.3 (the anticipatory single-Spirit anchor).

**Opening scene.** Tuesday, 5:48 PM. Sandra is mid-flow on a Profile Page wireframe. The Butler Spirit's `on_idle` lifecycle hook fires. Beliefs: Calendar shows 7 PM dinner T/Th, last 4 occurrences 2 attended late + 2 missed entirely (pattern detected); Slack status "Heads down" since 1 PM; Figma continuous active session for 4h 40m; predicted disengage time at current pace ≈7:28 PM; probability the dinner gets missed: 0.78; confidence 0.85.

The Butler computes a calendar-conflict-confidence scalar (its own definition — predicted-miss-probability × pattern-strength) and writes it via `working_memory.set_scalar("user.calendar_conflict.confidence", 0.85, derived_from=[calendar_obs, slack_obs, figma_obs])`. Per Butler's `[epistemic_policy]` rule — `tag = user.calendar_conflict.confidence, on_value_above = 0.8 → action = verbalize_with_options` — the kernel reads the scalar, compares to threshold, fires the action. **The kernel does not compute confidence; it compares the Spirit-supplied scalar to the threshold via universal arithmetic.**

The Butler surfaces a single notification through the kernel-rendered notification surface: pattern noticed, predicted disengage time, partner's unanswered message at 4:15, three options offered. Sandra picks (a) snooze the wireframe for 75 min. The Butler's follow-up writes a Linear note via MCP, sets a calendar reminder for 6:55 PM, archives the suggestion-acceptance signal to its `notification_acceptance_log`. Sandra arrives at dinner at 7:08 PM.

**Resolution (three weeks later).** The Butler observes that its **posterior uncertainty** over Sandra's preferred prompt-sensitivity has grown beyond threshold. The Butler computes its belief variance using its own definition (Shannon entropy over the work-mode-conditioned acceptance distribution) and writes via `working_memory.set_scalar("self.belief_variance", 0.78, derived_from=[14-day notification log refs])`. Per its `[epistemic_policy]`: `tag = self.belief_variance, on_value_above = 0.7 → action = halt`. The kernel compares the Spirit-supplied scalar (0.78) to the threshold (0.7) and fires the halt. Sandra resolves with `provided_context: split the policy by work-mode-context and lower sensitivity for shallow-work mode`. The Butler is self-tuning under human supervision — *upstream of* outcome degradation, not in response to it.

**Architectural primitives exercised.** Single-Spirit subprocess form. `on_idle` lifecycle hook as substrate for anticipatory reasoning. Telemetry Stream narrow per-Spirit subscription (Calendar/Slack/Figma topics). `[epistemic_policy]` per-tag rules with `verbalize_with_options` / `verbalize_only` / `flag` / `halt` actions. Output_shape predicate enforcement at the Capability Registry. MCP tool integrations (Calendar / Slack / Linear / Figma). Posture-shift command at runtime. Self-tuning via `epistemic.halt`. Transparency Log capturing every notification, every user response, every `[epistemic_policy]` update.

**Acceptance.** Notification precision ≥0.85; notification recall ≥0.7; halt-precision ≥0.85 on a 30-scenario calendar/comms behavior corpus; self-tuning halt fires in ≥9/10 synthetic acceptance-rate-decline scenarios within a 14-day rolling window; 4 MCP servers × 5 representative ops × 3 outcomes (success / scope-violation / network-error) = 60 integration tests, 100% pass.

## 10.3 J-Researcher — Hannah surveys the LLM-as-judge literature in 2 hours

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

## 10.4 J1 — The Founder's Loop

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

## 10.5 J4 — Elena's 2 AM 90-minute Mira-Nash incident

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

## 10.6 J6 — Diego, the third-party Spirit author

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

## 10.7 Deferred journeys (future milestones)

Two journeys from the PRD are **explicitly deferred** from the v1.5 architecture scope. They are not cancelled — they are committed as future work, with their full canonical scene descriptions remaining in the PRD as the source of truth until the architecture work to support them is undertaken. The substrate's primitives are designed so neither journey requires architectural rewrite to add later; both ship as configuration + new Spirit classes layered onto the existing kernel.

### 10.7.1 J3 — Marcus Team Nexus (deferred to v2.0 → **committed v2.2, design in §15.2**)

**Persona.** Marcus, tech lead / architect on an 8-person agile team at a fintech, day 30 of MAOS adoption. Peer A2A mesh; every team member has their own Host with three Spirits (Architect Atlas, Story-Decomposer, Coder Spirits, Test-Designer, Wireframe Spirit). Cross-Spirit conversations across the team are continuous, audit-trailed, and visible in narrative digest at standup.

**Why deferred.** J3 requires a **full peer A2A mesh** (8 Hosts, role queries, multi-party typed-intent consent across the cohort), where the v1.5 substrate ships only the **bilateral 2-Host A2A** topology (Mira ↔ Nash diagnostic-architect pair). The mesh extension is additive — same wire format, same per-frame consent envelope, same logical-clock discipline restricted from N=2 to N=8 — but requires (a) operator UX for cohort topology declaration, (b) role-query semantics across the consent envelope (intent class allowlists become per-(peer, role) tuples), and (c) cohort-level hot-swap and migration choreography. None of those break the kernel; all of them require substantive design work the v1.5 scope does not absorb.

**Substrate readiness for J3 at v1.5.** Bilateral A2A (§7.2) with mTLS+TOFU + ADR-012 typed-intent consent generalizes to N-host mesh additively. Hot-swap (ADR-017) and halt-continuity (I14) remain correct under cohort topology. Distillation pattern (§9.5) and decision-context recording (I12) operate per-Spirit regardless of cohort size. The §12.0 ADR set's binding-v1.5 commitments do not preclude J3.

**Future milestone target.** v2.0 (or v2.x). PRD §[J3 Marcus] holds the canonical scene; the v2.0 architecture revision will extend §7.2 to cover N-Host mesh, add operator-UX for cohort declaration, and document the role-query consent semantics. **No architectural decision in v1.5 forecloses J3.** *(2026-07-06: target now **v2.2**; the anticipated revision is §15.2 / proposed ADR-052 — cohort manifest, per-(peer,role) consent tuples, cohort hot-swap choreography with migration chains. Items (a)/(b)/(c) above map 1:1 onto §15.2's design.)*

**Reference.** PRD line ~431 ("Journey 3 — Tier 2 normalcy: Marcus's day-30 Tuesday morning standup") is the canonical persona, scene, and capability list. The PRD's §[Capabilities revealed (J3)] enumerates the kernel surfaces J3 exercises — all of them are commitments this architecture already makes (peer mesh A2A, ADR-012 consent, narrative digest per §9.5, audit chain per I11/I12/I13, halt continuity per I14).

### 10.7.2 Reza — Single-org cross-team Cortex (deferred to v2.0+ → **committed v2.2, design in §15.3/§15.4; enablers shipped by Epic 11**)

**Persona.** Reza, head of platform engineering at a 400-person fintech. Three teams (security, support, data) run their own Spirits independently across a single-org Cortex. Cross-team Spirits coordinate through cross-host A2A with ADR-012 typed-intent consent; data-residency patterns load from Loom; the Orchestrator unifies recommendations across team-owned Spirits without violating per-team write boundaries.

**Why deferred.** Reza Cortex requires (a) a **multi-tenant Loom** with per-team data-residency enforcement (the v1.5 Loom-lite is single-instance, single-tenant), (b) **WASM Spirit form** as the third deployment form for sandboxed third-party-team Spirits (the v1.5 substrate ships subprocess-only at v0.1, with rust-inproc gated on §13.1 measurement; WASM is out of scope per ADR-002 / ADR-007), (c) **PDP integration** for cross-team policy decisions, (d) **Spirit registry vetting attestations** beyond the three trust tiers v1.0 ships, and (e) cross-team A2A topology beyond the bilateral pair. Every one of these is additive to the kernel; none retract a v1.5 commitment.

**Substrate readiness for Reza at v1.5.** Loom-lite (§9.3) is designed for extraction to multi-tenant Loom without API churn (ADR-006 keeps the kernel learning nothing; multi-tenant boundaries land in user-space). The Spirit registry (ADR-008) at MCP-Streamable-HTTP is registry-protocol-ready for vetting attestation extensions. The trust-tier model (ADR-009, three tiers: local / org-internal / public-untrusted) admits a fourth tier (e.g., `org-vetted-public`) without altering the strictest-of-floor enforcement logic. Distillation multi-hop with `source_log_ref` flattening (I11, ADR-014) is the substrate Reza's "14 prior schema decisions cited in one consolidated proposal" scene depends on — already binding-v0.5.

**Future milestone target.** v2.0 (single-org Cortex pilot, 3-region, ≥10 agents) or v2.x as the architecture revision adds multi-tenant Loom, WASM Spirit form, PDP integration, and vetting attestations. *(2026-07-06 status: of the five deferral reasons above, (b) WASM form, (c) PDP, and the 3-region substrate shipped with Epic 11 (ADR-031/050/049); (a) multi-tenant Loom = §15.3, (d) vetting attestations = §15.4, (e) cross-team topology = §15.2 — all target **v2.2**.)* PRD §[Reza] line ~453 holds the canonical scene; the v2.0 architecture revision will extend §9 (Memory & Knowledge), introduce the WASM Spirit form (deferring the long-standing ADR-007 v2.0 commitment from "out of scope" to "in scope"), and document cross-team A2A topology.

**Reference.** PRD line ~453 ("Reza — Tier 3 candidate: Single-org cross-team Cortex") is the canonical persona, scene, and capability list. PRD §[Capabilities revealed (Reza)] enumerates the kernel surfaces Reza exercises — all of them are either (a) commitments this architecture already makes for v1.5 (distillation multi-hop, ADR-012 typed-intent consent, peer A2A) or (b) explicitly-named v2.0 substrate extensions (multi-tenant Loom, WASM Spirit form, PDP integration, vetting attestations).

### 10.7.3 Substrate-stability commitment under deferral

**The v1.5 architecture does not foreclose either deferred journey.** Specifically:

- The Spirit ABI (§5) is wire-stable and the v0.1 commitment to subprocess form (ADR-002) does not preclude WASM Spirit form being added at v2.0 — the wire protocol (ADR-032) is form-agnostic by design.
- Bilateral A2A (§7.2) is documented as "exactly two pre-paired Hosts" because that is the v1.5 deployment shape; the per-frame typed-intent consent envelope (ADR-012) generalizes to N-Host mesh without protocol change.
- Loom-lite (§9.3) is single-instance because v1.5 ships single-instance; the MCP-Streamable-HTTP transport admits multi-tenant deployment without kernel modification.
- The §0.6 Foundational Commitments hold across deferred journeys — kernel/Spirit separation, kernel learns nothing, human transparency, capability mediation — apply identically at any cohort size.

When a deferred journey is undertaken, the architecture revision adds new sections (J3 mesh topology, Reza multi-tenant Loom, WASM Spirit form per ADR-002/007 successor) and may revise specific ADRs, but the §0.6 commitments and the §3.2 invariants do not change.

The PRD remains the canonical home for J3 Marcus and Reza Cortex personas, scenes, and capability lists until those architecture revisions ship.
