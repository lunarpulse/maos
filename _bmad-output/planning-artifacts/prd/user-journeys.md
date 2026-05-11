# User Journeys

The substrate exposes three cognitive modes in increasing complexity: **anticipatory** (one Spirit watches and proactively notifies), **exploratory** (one Spirit fans out and converges on findings), and **compositional** (multiple Spirits coordinate under an Orchestrator with full distillation, audit-chain, and consent enforcement). The journey set is sequenced to teach those modes progressively, with each journey's capabilities a strict superset of the prior. **The wedge demo that announces the substrate is J1 Founder's Loop at v0.8 — readers who want the visceral demonstration first should jump there.** J-Butler at v0.3 and J-Researcher at v0.5 are the single-Spirit proof points that earn the founder-loop's compositional ambition; v0.1 ships the foundational kernel + placeholder Spirit only.

**Eight journeys, ordered by ship phase:** J0 Evaluator (cross-cutting; applies from v0.1 minimum-viable install onward); J-Butler (v0.3 anchor — anticipatory single-Spirit); J-Researcher (v0.5 anchor — exploratory single-Spirit); J1 Founder's Loop (v0.8 wedge demo — compositional multi-Spirit, the demo that announces the substrate); J3 Marcus Team Nexus (v1.0 — peer mesh); J6 Diego Spirit-author (v1.0 — third-party builder); J4 Mira-Nash 2 AM (v1.5 — diagnostic-architect pair); Reza single-org cross-team Cortex (v2.0/2.5).

Long-form novelistic versions live in `maos-user-journeys.md` (sister doc); the PRD carries anchor scenes plus capability codas. The journeys honor the carry-forward signals from Step 2c — the wedge pain commitment, the Tier 3 reframe to OSS / single-org-Cortex, the audit-vs-legibility distinction, the eight kernel guarantees enumeration, and the halt-recall/halt-precision benchmark routing — and they exhibit the architecture decisions reached during this PRD workflow: ADR-012 typed-intent consent, ADR-013 `log.recall`, ADR-014 distillation audit-chain (I11), ADR-015 decision-context recording (I12), ADR-016 token-budget, ADR-017 hot-swap wire format, ADR-018 intent provenance (I13), ADR-019 halt continuity (I14), ADR-020 hot-swap migration policy, ADR-021 CliWrapperSpirit output-shape, and the §9.5 distillation pattern.

## Journey B — v0.3 anchor: Sandra's Butler reads the third 6 PM and reclaims dinner

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

## Journey R — v0.5 anchor: Hannah surveys the LLM-as-judge literature in 2 hours

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

## Journey 1 — v0.8 wedge demo: The Founder's Loop (Lunarpulse runs Epic 7 from his daughter's bedtime to school drop-off)

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

## Journey 4 — Tier 3 sub-pattern: Elena's 2 AM 90-minute Mira-Nash incident

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

## Journey 3 — Tier 2 normalcy: Marcus's day-30 Tuesday morning standup

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

## Reza — Tier 3 candidate: Single-org cross-team Cortex

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

## Diego — Cross-cutting: The third-party Spirit author

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

## J0 — Cross-cutting: The evaluator

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

## Journey requirements summary

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

## Journey carveout map (FR trace policy for downstream consumers)

Most FRs trace directly to a named user journey (per the Journey Requirements Summary table above). Two carveout categories exist for FRs whose trace is non-obvious — **downstream consumers (epic breakdown, story creation, sprint planning) MUST treat carveout FRs as legitimate, not as orphans.**

**Category A — Deferred-journey-traced (intentional future-phase scope):** FRs anchored to journeys deliberately deferred from the `architecture-maos-minimal-opus.md` v0.1–v1.5 scope per its §10.7. These are NOT orphans; they are the substrate's commitment to the deferred journeys' enablement primitives.

| FR | Trace target | Phase | Note |
|---|---|---|---|
| FR23a / FR23b | J3 Marcus Team Nexus + J4 Mira-Nash + Reza Cortex | v0.8 (loopback) → v1.0 (cross-host) | A2A peer mesh — enables team mesh + bilateral diagnostic-architect + cross-region Cortex |
| FR48 | Reza Cortex (enterprise/regulated) | v0.5 | Pluggable cryptographic provider (FIPS / hardware-backed / post-quantum); enterprise-readiness substrate primitive |
| FR54 | J3 Marcus Team Nexus + J6 Diego (hermes-tenant defense) | v1.0 | Gateway sub-modules (Telegram/Slack/Discord/Signal/email); operationalizes the v1.0 hermes-tenant positioning claim |
| FR60 | Reza Cortex (air-gapped enterprise) + Aisha-CVE scenario | v1.5 | Air-gapped Spirit/skill artifact import; enterprise-readiness substrate primitive |
| FR64 | Reza Cortex (enterprise multi-tenant cost accounting) | v1.0 | Per-Spirit cost attribution; "no enterprise deployment without per-tenant cost accounting" gate |

**Category B — Substrate-property (architectural-invariant trace):** FRs that serve all journeys uniformly via the kernel's invariant guarantees rather than a single journey. These trace to architectural anchors (invariants I1–I14 + ADRs) per the FR-to-architecture traceability table in §Functional Requirements.

| FR | Architectural anchor | Why all-journeys |
|---|---|---|
| FR47 | I1 (capability mediation) + ADR-005 (Inference Port closure) | Every Spirit's model call traverses the kernel — no journey is exempt from this audit boundary |
| FR50 | I10 (lifecycle journaling) | Crash-survival applies whenever any Spirit dies; serves operational reliability across all journeys |
| FR62 | Innovation #7 (Constitutional Substrate Evolution) | Audit-queryable governance artifacts (vetter-key admission, ABI-extension proposals, ComplianceClaim schema versions) underpin third-party trust for every Spirit author + every operator |
| FR65 | Defends v1.0 hermes-tenant positioning + GDPR cascade (FR45) | Spirit uninstall proof-of-erasure; serves J0 clean-uninstall (Tier 1 success criterion) AND enterprise GDPR-style requirements |

**Operational guidance for epic breakdown:**

- **Default path:** every FR maps to ≥1 epic; epic phase = FR phase.
- **Category A FRs:** epics exist but ship in the FR's named phase, NOT in v0.1. Story-level acceptance criteria reference the deferred journey's reproducibility milestone (e.g., FR23b acceptance includes "10-host Cortex zero-conversation-drop chaos test" because that's the v1.0 J3-reproducibility gate).
- **Category B FRs:** epics anchor to the architectural invariant rather than a single journey; acceptance criteria reference invariant-property tests (e.g., FR47 acceptance = NFR-Test-2 surface-invariant test passes; ≥98/100 events recoverable from logs per NFR-Aud-1).
