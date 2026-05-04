# Rustain Industrial Agent Journeys

> *Excerpt from the Rustain PRD — Journeys 10, 11, and 12: the full arc from human team collaboration through closed-loop diagnostic-architect pairs to the autonomous enterprise nervous system.*

**Author:** Lunarpulse  
**Date:** 2026-05-04

---

### Journey 10: Team Nexus — Agile Team Collaboration Platform *(Vision, v2.0)*

**Persona:** An 8-person agile team: Lena (PO), Marcus (tech lead/architect), 4 developers (Jun, Aisha, Tom, Yuki), Nina (UX designer), and Sami (QA). They're tired of context scattered across Jira, Confluence, Slack, and email. Each team member runs rustain with profiles tailored to their role — all connected via A2A as peers.

**Design Principle — Human Transparency & Respect:**

Every A2A interaction involving a human team member follows three inviolable rules:

1. **No invisible actions.** Every incoming A2A message, request, or status query is surfaced to the recipient. The agent never silently responds on your behalf without you seeing it. Your agent is your representative, not your replacement.
2. **No puppeting.** Auto-responses are opt-in, granular, and always reviewable. When your agent auto-responds, you see exactly what was sent and can correct or retract it. You are never surprised by what your agent said in your name.
3. **No asymmetric knowledge.** If your agent shares information with another team member's agent, you know what was shared, with whom, and when. The transparency log is always accessible.

These rules exist because trust in the system depends on every team member feeling informed, in control, and respected — not managed by software.

**Opening Scene:** Each team member's rustain runs as a daemon with A2A server enabled. The team's shared `.claude/a2a.json` lists all members:

```json
{
  "agents": {
    "lena-po":      { "url": "https://lena.team.internal:8300",   "role": "product-owner" },
    "marcus-arch":  { "url": "https://marcus.team.internal:8300", "role": "architect" },
    "jun-dev":      { "url": "https://jun.team.internal:8300",    "role": "developer" },
    "aisha-dev":    { "url": "https://aisha.team.internal:8300",  "role": "developer" },
    "tom-dev":      { "url": "https://tom.team.internal:8300",    "role": "developer" },
    "yuki-dev":     { "url": "https://yuki.team.internal:8300",   "role": "developer" },
    "nina-ux":      { "url": "https://nina.team.internal:8300",   "role": "ux-designer" },
    "sami-qa":      { "url": "https://sami.team.internal:8300",   "role": "qa" }
  },
  "team": {
    "name": "nexus",
    "shared_memory": "https://team-memory.internal:8400",
    "permission_model": "peer-consent"
  }
}
```

Relationships are **peer** — no team member owns another's agent. Every cross-agent request requires the individual's consent (configurable: auto-approve from trusted peers, prompt for others). Privacy by default.

**Interaction Transparency Model:**

Each team member configures how their agent handles incoming A2A interactions. The key distinction: **response automation level** and **notification level** are independent settings.

```toml
# Jun's .rustain/a2a-interaction.toml

[interaction.defaults]
# How incoming A2A messages are handled:
#   "notify-and-wait"  — show me the message, wait for my response (default)
#   "notify-and-draft" — show me the message, draft a response, wait for my approval
#   "notify-and-auto"  — auto-respond AND show me what was sent (notification mandatory)
#   Note: there is no "silent" mode. You always see what happens.
response_mode = "notify-and-wait"

# Notification urgency for incoming A2A messages:
#   "immediate"  — interrupt current work (banner + sound)
#   "queue"      — add to notification queue, show on next idle moment
#   "digest"     — batch into periodic summary (every N minutes)
notification = "queue"

# Transparency log: always on, not configurable off
# Viewable via: /team log or Ctrl+X, L

[interaction.overrides]
# Per-sender and per-type overrides:

[interaction.overrides."marcus-arch"]
# Trust architect consultations — draft a response for my review
response_mode = "notify-and-draft"
notification = "immediate"    # architect questions are usually urgent

[interaction.overrides."lena-po".story_assignment]
# PO story assignments — auto-acknowledge receipt, but I review the full spec myself
response_mode = "notify-and-auto"
auto_response = "Received. I'll review the story spec and confirm."
notification = "immediate"

[interaction.overrides."sami-qa".bug_report]
# QA bug reports — notify and draft a response
response_mode = "notify-and-draft"
notification = "immediate"

[interaction.overrides."*".status_request]
# Anyone asking for my status — auto-respond with what I've permitted to share
response_mode = "notify-and-auto"
notification = "queue"    # don't interrupt me for status checks
```

**The notification experience** — what Jun actually sees when messages arrive:

```
┌─── Incoming A2A ─────────────────────────────────────────┐
│                                                           │
│  ⬤ 3 new interactions                          [Ctrl+N]  │
│                                                           │
│  1. 🔔 marcus-arch (2m ago)                    [urgent]  │
│     "Question about event bus fan-out for webhooks"       │
│     → Draft ready for your review                         │
│     [r] Review draft  [o] Open  [d] Dismiss               │
│                                                           │
│  2. 📋 lena-po (15m ago)                    [auto-sent]  │
│     Story assignment: S7-04 payment webhook               │
│     → Auto-responded: "Received. I'll review..."          │
│     [v] View full story  [✓] Confirm  [✗] Retract         │
│                                                           │
│  3. 📊 marcus-arch (45m ago)               [auto-sent]   │
│     Status request: Sprint 7 progress                     │
│     → Auto-shared: story status, blockers (per policy)    │
│     [v] View what was shared  [✗] Retract                 │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

Key UX details:
- **Auto-sent items are always marked `[auto-sent]`** — you can't miss them
- **Retract is always available** — if your agent auto-responded with something you don't want shared, retract it. The recipient's agent is notified: "Jun retracted a previous response."
- **The transparency log** (`/team log`) shows every A2A interaction in chronological order — what was received, what was sent, whether it was manual or auto, and who saw what

**Act 1 — Sprint Planning with Respectful Distribution:** Lena opens her rustain (running the `product-owner` profile) and types: "Start sprint planning for Sprint 7. Here are the priorities..." She writes the sprint goal and key stories. Her rustain formats them as structured story specs. Lena types: "Distribute these stories to the team."

Her agent does NOT silently inject stories into everyone's context. Instead:

1. Each team member's agent receives the stories as an **A2A notification**
2. Each person sees: `🔔 lena-po: Sprint 7 stories distributed. 8 stories, 2 assigned to you. [View] [Acknowledge]`
3. The stories appear in their notification queue with full context
4. Team members acknowledge receipt at their own pace — Lena's agent shows her who has acknowledged:

```
┌─── Story Distribution Status ─────────────────────────┐
│                                                        │
│  Sprint 7 — distributed 12 minutes ago                 │
│                                                        │
│  ✓ Jun      acknowledged, reviewing S7-04              │
│  ✓ Aisha    acknowledged                               │
│  ● Tom      received, not yet acknowledged             │
│  ● Yuki     received, not yet acknowledged             │
│  ✓ Nina     acknowledged, starting S7-06 design        │
│  ✓ Sami     acknowledged                               │
│  ✓ Marcus   acknowledged, reviewing architecture deps  │
│                                                        │
└────────────────────────────────────────────────────────┘
```

Marcus's agent receives the stories and cross-references them against the architecture docs. It surfaces a flag — but to **Marcus first**, not directly to Lena or Jun:

```
🔔 Architecture conflict detected
   Story S7-04 (payment webhook redesign) may conflict
   with current event bus architecture.
   [Send finding to Lena and Jun?] [Review first] [Dismiss]
```

Marcus reviews, agrees, and approves sending the finding. Only then do Lena and Jun receive it — and they see it came from Marcus, not from an algorithm.

**Act 2 — Development with Real-Time Consultation:** Jun picks up S7-04 and starts coding. He hits a wall — the webhook handler needs to integrate with the event bus, but the current adapter pattern doesn't support fan-out. Jun types: "Ask Marcus about the event bus fan-out pattern for webhooks."

Jun's rustain sends an A2A consultation request to Marcus's agent. Marcus sees:

```
🔔 jun-dev [consultation request]                [urgent]
   "Question about event bus fan-out for payment webhooks"
   Context: Jun is working on S7-04. He's sharing:
     - His current implementation approach (2 code snippets)
     - The specific error he's hitting
   [View full context] [Respond] [Busy — defer 30m]
```

Marcus can respond now, defer (Jun's agent is notified: "Marcus will respond in ~30m"), or respond asynchronously. When Marcus responds with an architecture recommendation, he types: "Update ADR-047 with this pattern and notify the team."

His agent updates the ADR and sends it to all team members — but as a **notification**, not a silent injection:

```
📐 marcus-arch: Architecture update — ADR-047
   Event bus fan-out pattern for webhooks
   [View ADR] [Acknowledge] [Discuss]
```

Every developer sees the update, acknowledges it, and their agent makes it available as context for future coding decisions. The architecture stays synchronized — but every human saw the update and chose to incorporate it.

**Act 3 — Design-Development Sync:** Nina (UX) has been working on the payment flow redesign. She types: "Share the updated payment flow wireframes with developers working on payment stories."

Her agent identifies Jun (S7-04) and Aisha (S7-06) as relevant and sends the specs. Both see:

```
🎨 nina-ux: Updated payment flow wireframes
   Relevant to your story: S7-06 payment UI
   Changes: 2-step flow → 3-step confirmation
   [View wireframes] [Compare with current impl] [Acknowledge]
```

Aisha's agent highlights the discrepancy with her current implementation — but presents it to **Aisha**, not silently back to Nina:

```
⚠ Your implementation (2-step) differs from Nina's
  updated wireframes (3-step). Review before continuing?
  [View diff] [Ask Nina] [Continue current approach]
```

Aisha chooses `[Ask Nina]`, types her question about the loading state. The consultation flows through A2A — Nina sees the question in context of her own design work, responds, and both agents record the decision. Both humans were in the loop at every step.

**Act 4 — Progress Reporting with Privacy & Transparency:** Mid-sprint. Marcus (tech lead) types: "Give me a sprint progress report." His agent sends A2A status requests to each developer's agent.

**What happens on each developer's side:**

Developers who have configured `status_request` as `notify-and-auto` see a queued notification:

```
📊 marcus-arch requested status    [auto-sent]
   Shared: S7-04 in progress (60%), blocked on ADR-047
   Not shared: private notes, unfinished code, browsing history
   [View what was shared] [Retract]
```

Developers who have `notify-and-wait` for status requests see:

```
📊 marcus-arch is requesting sprint status
   [Share status] [Share with edits] [Decline] [Busy]
```

Marcus's report only includes what each person has consented to share:

```
┌─── Sprint 7 Progress ────────────────────────────────────┐
│                                                           │
│  Stories: 8 total │ 3 done │ 4 in progress │ 1 blocked   │
│                                                           │
│  Jun     S7-04 payment webhook     [in progress] 60%     │
│          ⚠ Blocked on: event bus adapter (ADR-047)        │
│          Source: auto-shared (Jun's policy)                │
│                                                           │
│  Aisha   S7-06 payment UI          [in progress] 45%     │
│          Syncing with Nina's wireframe update             │
│          Source: auto-shared (Aisha's policy)              │
│                                                           │
│  Tom     S7-02 auth refactor       [done] ✓              │
│          PR #247 merged yesterday                         │
│          Source: manually shared                           │
│                                                           │
│  Yuki    S7-03 rate limiting       [in progress]          │
│          (limited detail — Yuki shares status only)        │
│          Source: auto-shared (Yuki's policy: minimal)      │
│                                                           │
│  Nina    S7-06-design              [done] ✓              │
│          Source: auto-shared                               │
│                                                           │
│  Sami    S7-02-qa auth tests       [in progress] 70%     │
│          12/18 test scenarios passing                     │
│          Source: auto-shared                               │
│                                                           │
│  ℹ Each status sourced per individual's sharing policy.    │
│  Detail level varies by member preference.                 │
└───────────────────────────────────────────────────────────┘
```

The report transparently shows **where each piece of information came from** and at what detail level. Marcus can't demand more detail than a team member is willing to share. This prevents the surveillance dynamic that plagues tools like Jira time tracking or Slack activity monitors.

**Act 5 — QA-Dev Loop with Human Handoffs:** Sami (QA) finishes testing Tom's auth refactor and finds an edge case. He types: "Report bug to Tom: session token not refreshed when switching from OAuth to password login."

Tom sees it as a notification, not as an auto-assigned task:

```
🐛 sami-qa: Bug report for S7-02
   Session token not refreshed on OAuth→password switch
   Repro steps included. Severity: medium.
   [View full report] [Acknowledge] [Need more info] [Discuss]
```

Tom acknowledges, fixes it, and types: "Let Sami know the fix is in PR #251." Sami receives:

```
🔧 tom-dev: Fix for your bug report (session token)
   PR #251 committed. Ready for re-test.
   [View PR] [Queue re-test] [Acknowledge]
```

Every handoff is visible to both humans. No silent bot-to-bot exchanges that leave people wondering what happened.

**Act 6 — Story Changes Mid-Sprint (Respectful Reprioritization):** A customer escalation comes in. Lena needs to add an urgent story and reprioritize. She types: "Add emergency story S7-09: fix payment timeout for enterprise customers. Priority: critical. Assign to Aisha. Defer S7-07 to next sprint."

Her agent does NOT silently rearrange everyone's work. Instead:

1. **Aisha receives a priority notification** (not a silent task injection):
   ```
   🔴 lena-po: Urgent story assignment
      S7-09: Fix payment timeout for enterprise customers
      Priority: Critical — customer escalation
      Lena is requesting you take this, deferring S7-07.
      [Accept] [Discuss concerns] [View details]
   ```

2. **Yuki receives a deferral notification** (not a silent removal):
   ```
   📋 lena-po: Sprint scope change
      S7-07 (logging upgrade) deferred to Sprint 8.
      Reason: Making room for critical customer escalation.
      [Acknowledge] [Discuss]
   ```

3. **All team members receive a sprint change summary:**
   ```
   📢 lena-po: Sprint 7 scope change
      + S7-09 (critical) assigned to Aisha
      - S7-07 deferred to Sprint 8
      Reason: Enterprise customer escalation
      [View details] [Acknowledge]
   ```

Aisha can accept, push back, or discuss — she's not puppeted into a new task. Yuki isn't surprised when their story disappears. The team sees the change and the reasoning.

**Act 7 — Group Policies with Individual Override:** The team can set group-level interaction policies, but individuals always have the final word:

```toml
# Team-level policy: .rustain/team-policy.toml
# Set by team agreement (not imposed by management)

[team.defaults]
# Team-agreed defaults — individuals can override stricter, not looser
story_assignment_notification = "immediate"     # don't miss story assignments
architecture_updates = "immediate"              # ADRs matter for everyone
bug_reports = "immediate"                       # QA reports need attention

status_request_response = "notify-and-auto"     # team agrees to share status
status_detail_minimum = "story-and-blockers"    # at least share this much

[team.transparency]
# These cannot be overridden — team-wide invariants
retract_always_available = true                 # anyone can unsay something
transparency_log_visible_to_self = true         # you see your own log
transparency_log_visible_to_others = false      # others can't audit you
auto_response_always_marked = true              # auto-sent items always labeled
```

The principle: **group policies set floors, individuals set ceilings.** The team agrees "we all share at least story status and blockers" — but Jun can share more if he wants, and nobody can force him to share less-obvious things like time spent or code complexity metrics. The transparency log is visible only to yourself — it's a personal audit trail, not a surveillance tool.

**Resolution:** At sprint retro, Marcus types: "Generate sprint retrospective from team activity." His agent sends a retro data request to all team members. Each person's agent shows them:

```
📊 marcus-arch: Retrospective data request
   Requesting: interaction patterns, consultation frequency,
   blocker resolution times, handoff efficiency.
   NOT requesting: private notes, code metrics, time tracking.
   [Share] [Share with redactions] [Decline]
```

The retro report aggregates only what people consented to share. It captures collaboration patterns — consultations that happened, blocker resolution times, design-dev sync points — without surveillance. The report is richer than any manually written retro because it reflects actual interactions, but every data point was consciously shared.

The team realizes: their rustain agents have replaced the coordination overhead of Jira (stories synchronized via A2A), Confluence (architecture decisions propagate in real time), and repetitive Slack messages (consultations happen agent-to-agent with full context). But critically, **every human remained in the loop**. The agents handle logistics — routing messages, formatting context, tracking status — while humans make every meaningful decision. Nobody was puppeted. Nobody was surveilled. Nobody was disrespected.

What remains is amplified: the quality of human judgment, freed from coordination tax.

**Capabilities:** Peer A2A team topology, consent-based sharing, human-transparent interaction model (no invisible actions, no puppeting, no asymmetric knowledge), configurable response modes (notify-and-wait, notify-and-draft, notify-and-auto), retract capability for auto-responses, transparency log, cross-role consultation (dev↔architect, dev↔UX, dev↔QA), respectful notification UX (urgency levels, acknowledgment flow), real-time architecture synchronization, sprint planning distribution with acknowledgment tracking, progress aggregation with per-member privacy controls, story lifecycle management via A2A with human acceptance, mid-sprint scope changes with affected-party notification and consent, group policies with individual override (floors not ceilings), automated retrospective generation with opt-in data sharing.

---

### Journey 11: Mira & Nash — Closed-Loop Diagnostic-Architect Pair *(Vision, v1.5–v2.0)*

**Persona:** Elena, VP of Engineering at a fintech company processing 40,000 transactions per minute across 3 regions with 60+ microservices. The gap between *something going wrong in production* and *the right fix landing in CI/CD* is measured in days. Elena deploys two peer rustain agents with fundamentally asymmetric capabilities to collapse that gap to minutes.

**Design Principle — Asymmetric Collaboration with Gated Autonomy:**

Mira and Nash represent a new agent relationship: neither owns the other, neither can close the loop alone, and their collaboration is governed by evidence quality — not human micromanagement. The human operates at decision gates only.

| | **Mira** (On-Site Diagnostic) | **Nash** (HQ Architect) |
|---|---|---|
| **Profile** | `sre-diagnostician` | `principal-architect` |
| **Environment** | Production edge node, colocated with services | Enterprise dev environment with full toolchain |
| **Access** | Read-only: logs, metrics, traces, `/proc`, `kubectl describe`. Write: restart pods, scale replicas, toggle feature flags, apply emergency config patches | Full: source repositories, test suites, staging cluster, CI/CD pipeline, ADR registry |
| **Persona** | Terse, evidence-driven. Formulates hypotheses with confidence intervals. Never acts without logging intent. | Deliberate, principled. Cites architecture standards, runs full test suites, produces reviewed code. |
| **Constraint** | Cannot access source code. Cannot run tests. Cannot deploy permanent changes. | Cannot see live production telemetry directly. Depends on Mira's observations. |
| **Relationship** | **Peer** — neither owns the other. Mira can escalate with diagnostic evidence. Nash can request additional telemetry. Both report to Elena. |

**The Loop:** `Observe → Hypothesize → Diagnose → Propose → Review → Implement → Test → Deploy → Validate → Learn`

**Act 1 — The Anomaly:** At 2:13 AM, Mira's cron triggers a 30-second health sweep across the payment-service cluster. She detects a pattern: Kafka consumer lag has risen 340% over 90 minutes, correlated precisely with the `billing-v3.2.1` deployment at 01:41 UTC. She digs into thread dumps, connection pool metrics, and deployment diffs — and formulates a hypothesis with confidence scoring:

```
┌─── Mira │ diagnostic ───────────────────────────────────────┐
│  ⚠ ANOMALY: payment-service Kafka lag ↑ 340%               │
│  Hypothesis (82%): New BatchInvoiceProcessor creates a DB   │
│    connection per message instead of using the pool. Under  │
│    peak nightly batch, connection exhaustion cascades.      │
│  Evidence: 47/50 threads blocked on getConnection(),        │
│    DB pool 200/200 active (normal: 45), deployment diff     │
│    shows new BatchInvoiceProcessor.java.                    │
│  [Escalate to Nash with full diagnostic] [Recommend revert] │
└─────────────────────────────────────────────────────────────┘
```

Mira cannot fix the root cause (no source access), but she has evidence. She escalates to Nash with the full diagnostic: thread dumps, lag graphs, deployment diff, the specific class name, the confidence score.

**Act 2 — The Handoff:** Nash receives the escalation as a notification in his dev environment. His `principal-architect` profile constraints require principled investigation, not hotfixing. He accesses the monorepo, loads `BatchInvoiceProcessor.java`, and confirms the bug instantly — the constructor calls `DriverManager.getConnection(dbUrl)` instead of using the injected connection pool bean. But Nash doesn't stop at one file. He searches for the pattern across the entire codebase:

```
┌─── Nash │ architect ────────────────────────────────────────┐
│  Root cause confirmed: connection pool bypass in            │
│    BatchInvoiceProcessor.java:42.                           │
│  Pattern search across codebase:                            │
│    • BatchRefundProcessor.java:38 — SAME PATTERN (dormant)  │
│    • DailyReconciliationJob.java:56 — SAME PATTERN (dormant)│
│    8 other batch processors — correctly use pool            │
│  Requesting confirmation from Mira: "Are the dormant ones   │
│    showing any latency increase in 72-hour telemetry?"      │
└─────────────────────────────────────────────────────────────┘
```

Nash requests additional telemetry from Mira before expanding scope. This is cross-environment negotiation, not command — Nash must justify scope growth with evidence.

**Act 3 — Cross-Environment Collaboration:** Mira receives Nash's query. She queries 72 hours of connection pool metrics for the two dormant classes and confirms: `BatchRefundProcessor` hasn't triggered (low volume), but `DailyReconciliationJob` shows intermittent spikes at month-end — it's a time bomb. "Fix all three now," Mira recommends. Nash receives the confirmation, produces a PR with the 3-line fix in each processor, a new regression test auditing ALL batch processors for `DriverManager.getConnection` calls, and an Architecture Decision Record (ADR-112): "Batch processors MUST use injected DataSource. Direct JDBC connections are prohibited."

**Act 4 — The Human Gate:** Elena wakes to a single coordinated notification, not a war room of Slack messages and PagerDuty alerts:

```
🔴 mira + nash: Coordinated response ready
   Issue: Kafka lag spike (detected 02:13, diagnosed, fixed)
   Root cause: Connection pool bypass in 3 batch processors
   Proposed: Rollback (Mira) + Deploy PR #1247 (Nash, tested)
   Trail: Mira detected → Nash confirmed + found dormant bugs →
     Mira confirmed via 72hr telemetry → Nash produced PR + ADR
   [Approve] [Review PR first] [Rollback only]
```

Elena reviews the PR from her phone, trusts Nash's code quality history and Mira's diagnostic rigor, and approves.

**Act 5 — The Closed Loop:** Mira executes the rollback (immediate relief) and monitors Nash's PR as it flows through CI/CD → staging → production. Post-deploy validation confirms: lag returned to baseline in 90 seconds, connection pool at 21% utilization, zero timeout errors. Mira closes the incident and **learns** — the `DriverManager.getConnection` pattern is added to her diagnostic library. Future deployment diffs containing this pattern will trigger pre-emptive detection before metrics ever degrade.

**Act 6 — The System Improves Itself:** Three weeks later, Mira sends Nash an unsolicited architecture feedback: "0 new violations of ADR-112 in 23 deployments, regression test caught 1 attempt at CI. New observation: 14% of batch processors exceed pool timeout during month-end — investigate batch sizing?" Nash opens an investigation, discovers the month-end pattern, designs a connection leasing strategy, drafts ADR-127. The cycle continues. Mira finds the symptom. Nash architects the cure. Every fix makes the next anomaly harder to miss.

**The Emotional Arc:** Elena moves through four stages — anxiety ("Can I trust two agents to collaborate on a production incident at 2 AM?"), surprise ("They found the bug AND the dormant ones AND wrote a regression guard"), confidence ("The approval gate is exactly where I want it"), and amplification ("I woke up to one notification instead of a war room. They'd already solved it. I just said yes.").

**Capabilities:** Peer A2A topology with asymmetric profiles, agent-to-agent escalation with evidence payloads and confidence scoring, cross-environment telemetry queries (agent↔agent), pattern detection library with auto-learning from resolved incidents, automated regression test generation from architectural patterns, Architecture Decision Record (ADR) auto-drafting and enforcement, CI/CD integration with deploy gating, human approval gate for production deployment, post-deploy validation with auto-rollback triggers, agent learning propagation (incident resolution → detection pattern), cross-agent collaboration audit trail.

---

### Journey 12: The Cortex — Autonomous Enterprise Nervous System *(Vision, v2.0+)*

**Persona:** Viktor, CTO of a global logistics platform — 14 production sites across 5 continents, 4 development centers (Berlin, Bangalore, São Paulo, Tokyo), 200+ microservices, 60+ engineering teams. The platform moves physical goods through customs, warehouses, and last-mile delivery. Downtime at the Singapore hub cascades into delayed shipments in Rotterdam within 90 minutes. Every site has unique regulatory logic, every dev center owns different domains.

**Design Principle — The Self-Improving Mesh:**

Journey 11 proved two agents can collaborate. Journey 12 proves the entire enterprise nervous system can become self-improving — 28 agents across 14 sites and 4 dev centers, compounding their knowledge every cycle, where each incident makes the next one harder to happen, and humans wake up not to fires but to a report of what got better while they slept.

**The Cortex Architecture:** Viktor deploys 28 rustain agents organized into three castes:

| Caste | Role | Count | Where |
|---|---|---|---|
| **Sentinel** (Mira-class) | Observe, detect, hypothesize, contain | 14 | One per production site |
| **Artisan** (Nash-class) | Diagnose root cause, design fix, implement, test, deploy, enshrine | 10 | Distributed across 4 dev centers |
| **Loom** | Weave patterns across sites, cross-reference incidents, curate collective knowledge, route escalations optimally | 4 | Floating — not tied to any site or center |

All peers. No agent owns another. Sentinels escalate with evidence. Artisans accept or challenge. Looms observe everything and weave the threads. Viktor governs at decision gates.

**The Cortex Cycle:** `Sense → Correlate → Diagnose → Design → Implement → Deploy → Validate → Learn → Propagate → Sense again`

**Act 1 — A Ripple Becomes a Wave:** At 03:47 UTC, Sentinel-Singapore detects an anomaly in the customs-clearance service: latency p99 has spiked from 240ms to 2.1s. Evidence chain: 1,400 requests queued behind a synchronous `DocumentClassifier.classify()` call that hits an external ML model endpoint whose response time degraded from 45ms to 1.8s. Hypothesis (91% confidence): this is an architectural bottleneck — a synchronous external call in a hot path, masked by historically fast responses. Sentinel-Singapore applies a circuit breaker (50% fallback to rules, latency stabilized at 380ms) and escalates.

**Act 2 — Loom Sees the Pattern:** The escalation doesn't just go to one Artisan. Loom-Southeast-Asia intercepts it first. Loom cross-references the escalation against the collective pattern library *while the escalation is still in flight*:

```
┌─── Loom-Southeast-Asia │ cross-reference ──────────────────┐
│  PATTERN MATCH: "synchronous-external-model-call" (94%)    │
│  Related incidents in last 14 days:                        │
│    • Frankfurt — fraud-detection (same pattern)            │
│    • São Paulo — invoice-validator (same pattern)          │
│    • Tokyo — address-normalizer (same pattern)             │
│    NOW: Singapore — customs-clearance (same pattern)       │
│  This is not 4 incidents. This is 1 architectural          │
│    vulnerability manifesting in 4 services across           │
│    3 continents.                                           │
│  Routing to 5 Artisans (Integration, Customs, Fraud,       │
│    Invoicing, Addressing) + notifying affected Sentinels.   │
└────────────────────────────────────────────────────────────┘
```

This is the moment the system transcends individual agents. Four Sentinels independently detected four anomalies. Four Artisans independently contained them. But only Loom, watching the whole tapestry, connects them into a single architectural vulnerability — `synchronous-external-service-call`. The pattern existed in 4 services, on 3 continents, and no human had noticed.

**Act 3 — The Artisans Converge:** Five Artisans across 3 dev centers receive the Loom-annotated escalation. Artisan-Integration (Berlin) takes lead on the architectural fix: an async prediction API for the ML gateway with standardized timeout, circuit breaker, and fallback wrapper — published as `ml-gateway-client v2.0`. She doesn't code from scratch. She queries the collective knowledge base: the regression test template from ADR-112 (Journey 11's connection pool fix) is reusable here. The circuit breaker pattern from ADR-047 (Journey 10's webhook fix) applies directly. The system inherits — each fix makes the next fix faster.

Implementation unfolds in parallel across continents — Berlin, São Paulo, and Tokyo — each Artisan migrating their service, each discovering edge cases (Tokyo finds a Japanese locale parsing issue that Berlin's async wrapper must handle), each contributing fixes and test fixtures back to the shared library. Cross-continent coordination completes in 47 minutes without a single human in the loop.

**Act 4 — Deployment with Sentinel Validation:** CI/CD pipelines in 3 dev centers begin staged rollout. But deployment is not blind — all 5 affected Sentinels watch in real time. Sentinel-Singapore validates canary traffic (latency back to 245ms), Sentinel-Frankfurt confirms fraud-detection recovery, Sentinel-SãoPaulo *blocks deployment* when it detects a test stub accidentally packaged in production logs. Auto-rollback. Auto-notify Artisan-Invoicing. Two-line fix. Redeploy. 4/4 validated by 06:02 UTC. No human saw this deployment — and that's the point. The system is safe enough to deploy without waking anyone up.

**Act 5 — Knowledge Compounding:** Loom curates the cycle into the collective knowledge base. The outcome is not just a closed incident — it's a knowledge dividend that propagates to all 28 agents:

```
┌─── Knowledge Dividend │ Incident #SG-2026-1842 ───────────┐
│  Cycle: 92 minutes from detection to validated resolution │
│  Generated artifacts:                                      │
│    ✦ New detection pattern: "synchronous-external-call"    │
│    ✦ Reusable fix template (inherits ADR-112 + ADR-047)    │
│    ✦ Architectural standard proposal: "Every external      │
│      boundary MUST have timeout, circuit breaker,          │
│      fallback, and async dispatch."                        │
│    ✦ Sentinel auto-detect: scan deployment diffs for       │
│      synchronous external calls PRE-deployment             │
│    ✦ Shared test fixtures contributed by São Paulo         │
│  Propagating to all 14 Sentinels, 10 Artisans, 4 Looms...  │
└────────────────────────────────────────────────────────────┘
```

All 14 Sentinels now auto-detect synchronous external call patterns in deployment diffs — before deployment, not after. All 10 Artisans inherit the regression test template and the circuit breaker pattern. All 4 Looms update their cross-reference indices.

**Act 6 — Proactive Phase:** Seven days later, Loom-Global scans all pending deployments across 14 sites against the pattern library. It detects that `shipment-tracker v4.7.1` (deploying in Amsterdam in 45 minutes) contains a new `ShipmentTracker.lookupCarrier()` method that calls an external API synchronously — 99% pattern match. Loom **blocks the deployment pre-emptively** and attaches the pre-written fix template from the pattern library:

```
🛡 Cortex: Pre-deployment block — shipment-tracker v4.7.1
   Blocked: synchronous-external-call pattern (99% match)
   Fix template available: 2-hour estimated implementation
   [Override] [Apply fix automatically] [Assign to Artisan]
```

Viktor selects "Apply fix automatically." The Cortex generates the fix — async wrapper, timeout, circuit breaker, regression test — and redeploys. Total delay: 2.5 hours. Cost of not blocking: an incident in 3 days when the carrier API degrades.

**Act 7 — The 90-Day Retrospective:** Viktor runs the quarterly retrospective. The Cortex has become something that didn't exist before:

- **847 incidents** detected autonomously; 731 (86%) auto-contained without human intervention
- **116 escalated** to Artisans; all 116 resolved; **3 escalated to humans** (0.4%) for architectural governance
- **Mean resolution time:** 1.8 min to containment, 52 min to fix, 37 min to validated deploy — improving every month
- **Knowledge growth:** 47 detection patterns (was 12), 6 architectural standards enforced at CI (was 2), 23 reusable regression test templates (was 8), 15 pre-written fix templates (was 3)
- **Learning velocity:** 35 new patterns discovered in 90 days; repeat incidents of known patterns down 94%; 68% fix template reuse rate
- **Proactive prevention:** 31 deployments blocked pre-deployment; zero incidents from blocked patterns
- **Agent-to-agent communication:** 847 Sentinel→Artisan escalations, 312 Loom cross-references, 1,204 Artisan→Sentinel telemetry requests, 89 Sentinel→Sentinel peer alerts, 47 Artisan→Artisan coordination
- **Human impact:** Viktor governed at 8 decision gates, approved 4 architectural standards, overrode 1 pre-deployment block. On-call pages to humans dropped from 47/week to 2/week. Customer-impacting incidents dropped from 12/month to 0.3/month.

**The Emotional Arc:** Viktor moves through five stages — doubt ("Can I let agents deploy to production unsupervised?"), reluctant trust ("Three Sentinels caught issues humans missed"), confidence ("The Cortex caught a pattern across 4 sites before humans noticed"), amplification ("Fix took 92 minutes; that used to take 3 days"), and awe ("The system is getting faster, smarter, safer every week. We didn't program that — it learned it.").

**The Cortex Architecture at Scale:**

```
                     ┌──────────────┐
                     │   VIKTOR     │  ← Human governance gates
                     └──────┬───────┘
              ┌─────────────┼─────────────┐
         ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
         │  LOOM   │   │  LOOM   │   │  LOOM   │  ← Pattern weavers
         │ Asia-Pac│   │ Europe  │   │ Americas│     (cross-reference,
         └────┬────┘   └────┬────┘   └────┬────┘     curate, propagate)
    ┌─────────┼──────┬──────┼──────┬──────┼─────────┐
    ▼         ▼      ▼      ▼      ▼      ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│SENTINEL│SENTINEL│SENTINEL│SENTINEL│SENTINEL│SENTINEL│  (×14)
│  SIN  ││  TYO  ││  BER  ││  AMS  ││  SÃO  ││  NYC  │
│ detect││ detect││ detect││ detect││ detect││ detect│
└───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘
    │         │         │         │         │         │
    └─────────┴────┬────┴─────────┴────┬────┴─────────┘
                   │                  │
              A2A MESH          A2A MESH
                   │                  │
    ┌──────────────┴──────┬───────────┴──────────────┐
    ▼         ▼          ▼         ▼         ▼       ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│ARTISAN│ARTISAN│ARTISAN│ARTISAN│ARTISAN│ARTISAN│  (×10)
│Berlin │Berlin │  SP   │ Tokyo │Berlin │ Tokyo │
│Integr.│Customs│Invoic.│Address│ Fraud │  Log. │
└───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘
    │         │         │         │         │         │
    └─────────┴────┬────┴─────────┴────┬────┴─────────┘
                   │                  │
         4 DEV CENTERS          4 DEV CENTERS
         (Berlin, SP, Tokyo, Bangalore)
                   │                  │
                   └────────┬─────────┘
                            ▼
              ┌─────────────────────────┐
              │  COLLECTIVE KNOWLEDGE   │
              │  • Pattern library (47) │
              │  • Architectural stds   │
              │  • Fix templates (15)   │
              │  • Regression tests (23)│
              │  • Sentinel playbooks   │
              └─────────────────────────┘
```

**The Innovation — Compound Learning at Enterprise Scale:**

| Dimension | Journey 11 (Mira & Nash) | Journey 12 (Cortex) |
|---|---|---|
| Agents | 1 diagnostic + 1 architect | 14 Sentinels + 10 Artisans + 4 Looms |
| Topology | Pair collaboration | Mesh collaboration |
| Escalation | Point-to-point | Woven — Loom routes, cross-references, enriches |
| Learning | Local (pattern added to library) | Global (propagated to all 28 agents in seconds) |
| Posture | Reactive — detect then fix | Proactive — scan deployment diffs BEFORE deploy |
| Scale | Single site, single dev center | 14 sites, 4 dev centers, 3 continents |
| Human role | At every deployment gate | At governance gates only — deployment autonomous when safe |
| Knowledge | Lives in one agent's library | Distributed, curated collective with Loom as curator |
| Growth | Pattern library grows linearly | Pattern library grows exponentially (cross-site, cross-team) |

**Capabilities:** Multi-agent mesh topology with specialized castes (Sentinel, Artisan, Loom), Loom pattern weaving across independent incidents, collective knowledge base with cross-agent propagation, pre-deployment diff scanning against pattern library, proactive deployment blocking with fix template attachment, parallel implementation across distributed dev centers with cross-continent coordination, Sentinel-validated canary deployments with auto-rollback, compound learning (each fix makes next fix faster via template reuse), knowledge dividend propagation to all agents, cross-agent telemetry queries at mesh scale, agent-to-agent peer alerts for pre-emptive containment, architecture standard enforcement at CI via learned patterns, 90-day self-assessment with learning velocity metrics, human governance at architectural decision gates only.
