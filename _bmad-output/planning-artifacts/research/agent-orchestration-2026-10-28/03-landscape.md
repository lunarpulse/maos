# 03 — Landscape: agent orchestration UX and techniques beyond codex / oh-my-pi

| Field | Value |
|---|---|
| Study | 03-landscape (input to the BMad technical-research report "agent orchestration for maos") |
| Author | `ResearchLandscape` subagent |
| Date / access date for every URL below | 2026-10-28 (UTC fetch window ≈ 10:30–11:45Z) |
| Method | Primary sources fetched and searched with regex (official docs, specs, engineering blogs, arXiv). Secondary sources (search-engine summaries, DeepWiki, blogs) used only to *find* primary pages; any claim resting only on them is marked `[SECONDARY]`. |
| maos reference | `lunarpulse/dev_ws@d9bd59c456a68a9aaddbbee4594796beed288b91` (read-only) |
| Pinned spec snapshot | OpenTelemetry GenAI conventions: `open-telemetry/semantic-conventions-genai@51a0f580f8a6ad804ca4de5e033683c6fbbe5906` |
| Out of scope | codex and oh-my-pi internals (sibling studies 01/02). This file only cross-references them. |

Tag legend: `[INFERENCE]` = my reasoning, not stated by the source. `[SECONDARY]` = not confirmed on a primary page. "not found" = searched, not present (search stated).

## Source keys

Docs are living pages; quotes below are the replication anchor (not line numbers, which shift between renders). Every key resolves to one URL accessed 2026-10-28.

| Key | URL |
|---|---|
| CC-PAR | https://code.claude.com/docs/en/agents.md ("Run agents in parallel") |
| CC-AV | https://code.claude.com/docs/en/agent-view.md ("Manage multiple agents with agent view") |
| CC-TEAM | https://code.claude.com/docs/en/agent-teams.md ("Orchestrate teams of Claude Code sessions") |
| CC-SUB | https://code.claude.com/docs/en/sub-agents.md ("Create custom subagents") |
| CC-HOOK | https://code.claude.com/docs/en/hooks.md ("Hooks reference") |
| CC-WF | https://code.claude.com/docs/en/workflows.md ("Orchestrate subagents at scale with dynamic workflows") |
| CC-CKPT | https://code.claude.com/docs/en/checkpointing.md ("Checkpointing") |
| GH-SESS | https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents (fetched as markdown via `https://docs.github.com/api/article/body?pathname=/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents`) |
| GH-MC | https://github.blog/changelog/2025-10-28-a-mission-control-to-assign-steer-and-track-copilot-coding-agent-tasks/ |
| JULES-PLAN | https://jules.google/docs/review-plan/ |
| JULES-API | https://jules.google/docs/api/reference/types/ |
| DEVIN-ADV | https://docs.devin.ai/work-with-devin/advanced-capabilities |
| DEVIN-WF | https://docs.devin.ai/work-with-devin/dynamic-workflows |
| CUR-WT | https://cursor.com/docs/configuration/worktrees.md |
| CUR-CLOUD | https://cursor.com/docs/cloud-agent.md |
| CUR-SCALE | https://cursor.com/blog/scaling-agents |
| AMP-SUB | https://ampcode.com/docs/models-and-subagents |
| FAC-SUB | https://docs.factory.com/harness/subagents.md |
| OH-TASK | https://docs.openhands.dev/sdk/guides/task-tool-set |
| ANT-BEA | https://www.anthropic.com/engineering/building-effective-agents |
| ANT-MARS | https://www.anthropic.com/engineering/multi-agent-research-system |
| ANT-CCC | https://www.anthropic.com/engineering/building-c-compiler (canonical `https://www.anthropic.com/research/building-c-compiler`, published 2026-02-05) |
| COG-DBMA | https://cognition.com/blog/dont-build-multi-agents (2025-06) |
| COG-MAW | https://cognition.com/blog/multi-agents-working (2026-04-22) |
| MAST | https://arxiv.org/abs/2503.13657 (Cemri et al., "Why Do Multi-Agent LLM Systems Fail?") |
| OAI-HO | https://openai.github.io/openai-agents-python/handoffs/ |
| OAI-GR | https://openai.github.io/openai-agents-python/guardrails/ |
| OAI-TR | https://openai.github.io/openai-agents-python/tracing/ |
| OAI-HITL | https://openai.github.io/openai-agents-python/human_in_the_loop/ |
| LG-INT | https://docs.langchain.com/oss/python/langgraph/interrupts.md |
| LG-TT | https://docs.langchain.com/oss/python/langgraph/use-time-travel.md |
| LC-MA | https://docs.langchain.com/oss/python/langchain/multi-agent.md |
| LG-SUPMIG | https://docs.langchain.com/oss/python/migrate/langgraph-supervisor.md |
| MAF-OV | https://learn.microsoft.com/en-us/agent-framework/overview/ |
| MAF-ORCH | https://learn.microsoft.com/en-us/agent-framework/workflows/orchestrations/ |
| MAF-CKPT | https://learn.microsoft.com/en-us/agent-framework/workflows/checkpoints |
| CREW-PROC | https://docs.crewai.com/en/concepts/processes |
| A2A | https://a2a-protocol.org/latest/specification/ ("Latest Released Version 1.0.0") |
| MCP-TASKS | https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks.md |
| MCP-ELIC | https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation.md |
| OTEL-AGENT | https://raw.githubusercontent.com/open-telemetry/semantic-conventions-genai/51a0f580f8a6ad804ca4de5e033683c6fbbe5906/docs/gen-ai/gen-ai-agent-spans.md |

---

## 1. Summary

1. **The operator's two highlighted UX facts are now an industry pattern, and Claude Code documents both shapes in one product.** `claude agents` opens an "agent view" roster of background sessions with dispatch / peek / attach / detach, and pressing `←` on an empty prompt backgrounds the current session and opens that roster (CC-AV §Quick start, §Switch sessions). Separately, in-session subagents render "as a tree in the subagent panel … with a path back to `main`" (CC-SUB §Let subagents spawn their own subagents). That these mirror codex's `codex agents` and omp's left-arrow navigation is `[INFERENCE]` pending sibling studies 01/02.
2. **"Mission control" is converging on the same five verbs: dispatch, observe (state-grouped roster + one-line summary), peek-and-reply, attach/detach, stop/archive.** Claude Code agent view (CC-AV), GitHub Copilot's agents page (GH-SESS, GH-MC), Devin's parent/child sidebar plus coordinator (DEVIN-ADV), and Cursor's Agents Window (CUR-WT) all offer these verbs.
3. **The roster is sorted around the human, not the agent.** "Needs input" is grouped at the top and has its own state. Notifications fire on needs-input/completed/failed. A footer counter (`← 2 agents`) works as a passive inbox (CC-AV §Read session state, §Quick start step 5).
4. **Steering lands at tool-call boundaries, not mid-token.** Copilot "implements your input after it finishes its current tool call" (GH-SESS). Claude Code queues peek replies "instead of interrupting the response" (CC-AV §Peek and reply). Amp turns a follow-up to a running Task subagent into "extra instructions for its current work" (AMP-SUB).
5. **Durable, resumable pause points are built into frameworks and protocols as first-class objects:** OpenAI `RunState` with `interruptions` (OAI-HITL), LangGraph `interrupt()` + checkpointer + `thread_id` (LG-INT), MAF superstep checkpoints that include pending requests (MAF-CKPT), A2A `TASK_STATE_INPUT_REQUIRED`/`AUTH_REQUIRED` (A2A), MCP task `input_required` (MCP-TASKS, experimental).
6. **Two products record the orchestration itself so it can be replayed deterministically.** Devin workflows key each `agent()` call "by a hash of its prompt, schema, and execution settings" and replay completed calls on resume (DEVIN-WF). Claude Code workflows make `Date.now()`/`Math.random()` throw so a relaunch "repeats the same `agent()` calls" (CC-WF).
7. **Isolation is per-worker workspace:** git worktrees (Claude Code background sessions and `isolation: worktree`, Cursor `/worktree` and `/best-of-n`), per-agent VMs (Devin managed Devins, Cursor cloud agents, Jules), containers (Anthropic's 16-agent compiler harness). Claude Code agent teams explicitly do **not** isolate teammates and tell users to partition files (CC-PAR, CC-TEAM).
8. **Budgets show up as hard caps and advisory warnings**, not one unified ledger. Examples: 20 concurrent subagents and depth 3 by default (CC-SUB); `maxTurns`; a "Large workflow" warning above 25 agents or 1.5M projected tokens (CC-WF); per-child ACU limits (DEVIN-ADV); daily/concurrent task quotas (Jules) `[SECONDARY]`.
9. **Evidence for multi-agent is real but conditional.** Anthropic measured +90.2% on an internal research eval and found token usage explains 80% of BrowseComp variance, at ~15× chat tokens (ANT-MARS). Cognition's 2026 position: multi-agent works "when writes stay single-threaded and the additional agents contribute intelligence rather than actions" (COG-MAW). MAST finds 14 failure modes in 3 categories across 7 frameworks (MAST).
10. **Clean-context verifiers are the best-evidenced add-on.** Devin Review catches ~2 bugs/PR (~58% severe) and works best when coder and reviewer share no context (COG-MAW). Claude Code `/deep-research` votes on claims and filters those that fail cross-checking (CC-WF). The Cursor and Anthropic long-run harnesses rely on judges or test oracles (CUR-SCALE, ANT-CCC).
11. **Flat peer swarms and lock-based self-coordination have documented failure reports.** Lock contention cut 20 agents to "the effective throughput of two or three", and flat agents became "risk-averse". Planners/workers/judge fixed it (CUR-SCALE). Cognition calls unstructured swarms "mostly a distraction" (COG-MAW).
12. **Observability standards are still in development.** OTel GenAI defines `create_agent`, `invoke_agent` (CLIENT/INTERNAL), `invoke_workflow`, `execute_tool`, and `plan` operations with `gen_ai.agent.*` and `gen_ai.conversation.id` attributes, all marked **Development** (OTEL-AGENT). The OpenAI Agents SDK emits nested `agent_span`/`handoff_span`/`guardrail_span` traces by default (OAI-TR).

---

## 2. Capability inventory

Abbreviation: "repl." = how to re-verify (see §7 for full commands).

### 2.1 Coding-agent products

| Capability | How it works (mechanism) | User-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **Claude Code — agent view (session roster)** | Each background session is "its own Claude Code process under the supervisor". The supervisor keeps sessions running with no terminal attached. State lives in `~/.claude/jobs/<id>/state.json` and `~/.claude/daemon/roster.json`. Idle sessions unattached for ~1h are stopped and resumed on attach. Pinned sessions stay alive. | `claude agents` takes over the terminal. Rows are grouped Pinned / Ready for review / Needs input / Working / Completed. Each row shows name, Haiku-generated one-line summary, age, and PR label `#N` coloured by CI/review status. | CC-AV §Monitor sessions, §Row summaries, §Pull request status, §The supervisor process, §Where state is stored | grep CC-AV for `supervisor process`, `roster.json`, `Haiku-class model` |
| Claude Code — dispatch, peek, attach, detach | Dispatch input at the bottom starts a new background session per prompt. `Space` opens a peek panel (latest output or pending question). `Enter`/`→` attaches (full session, recap of what happened while away). `←` on an empty prompt detaches. `/bg` or `←` backgrounds a foreground session. `/fork` copies it. | See flow §3.1. Keys: `↑/↓`, `Space`, `Enter`/`→`, `←`, `Ctrl+X` stop (twice = delete), `Ctrl+T` pin, `Ctrl+R` rename, `?` help. Filters `a:` `s:` `n:` `o:`. | CC-AV §Quick start, §Peek and reply, §Attach to a session, §Keyboard shortcuts, §Filter sessions | grep CC-AV for `Press \`←\` on an empty prompt` |
| Claude Code — scriptable roster | `claude agents --json [--all] [--cwd]` returns `id`, `state` (`working`/`blocked`/`done`/`failed`/`stopped`), `status`, and `waitingFor` (`permission prompt`, `input needed`, `sandbox request`, …). Shell verbs: `claude attach|logs|stop|respawn|rm <id>`, `claude daemon status|logs|stop`. | Scripts, status bars, or "another Claude session that supervises background work" poll JSON instead of reading files. | CC-AV §Manage sessions from the shell, §Read session state from a script | grep CC-AV for `waitingFor` |
| Claude Code — subagents | Markdown + YAML (`.claude/agents/`, `~/.claude/agents/`). Fresh context window. Tool allow/deny lists, `model`, `permissionMode`, `maxTurns`, `isolation: worktree`, `background`, `memory`. The parent gets a summary. Transcripts persist as `agent-{agentId}.jsonl`. Resume by `SendMessage` to the ID. | Delegation shows as a tool row `name(task)`. A panel below the prompt lists running subagents. Nested subagents show as a tree with `(+N)` descendant counts. `/tasks` lists and lets you attach to or stop background items. | CC-SUB §Frontmatter reference, §Let subagents spawn their own subagents, §Resume subagents; CC-PAR §Check on running work | grep CC-SUB for `as a tree in the subagent panel` |
| Claude Code — limits on subagents | Default 20 concurrent (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`). Depth limit of 3 layers (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`). No total cap per session. At the depth limit the `Agent` tool is withheld. | Spawning past the limit fails with `Concurrent subagent limit reached` and "tells Claude not to retry". | CC-SUB §Concurrent subagent limit, §Let subagents spawn… | grep CC-SUB for `Concurrent subagent limit reached` |
| Claude Code — agent teams (experimental, off by default) | A lead session plus teammates (full sessions). A shared task list with pending/in-progress/completed states and dependencies ("blocked"). Claiming uses file locking. Each mailbox is a JSON file `~/.claude/teams/{team}/inboxes/{agent}.json`. Structured protocol messages: plan approval, shutdown request. | An agent panel under the prompt: `↑/↓` select, `Enter` opens transcript + direct message, `Esc` interrupts the viewed teammate, `x` stops, `Ctrl+T` toggles task list. Optional tmux/iTerm2 split panes. | CC-TEAM §Control your agent team, §Architecture, §Assign and claim tasks | grep CC-TEAM for `file locking`, `inboxes` |
| Claude Code — inter-agent message trust | A message over `SendMessage` is labelled as coming from another Claude session. "A teammate can't approve a permission prompt or supply consent on your behalf". In auto mode a classifier reviews each inter-agent message before delivery. | Permission prompts from teammates and background subagents surface in the lead/main session, naming the requester. | CC-TEAM §Messages between agents, §Permissions; CC-SUB §Tools, permissions, and results of background subagents | grep CC-TEAM for `can't approve a permission prompt` |
| Claude Code — lifecycle hooks | 30+ events, including `SubagentStart/Stop`, `TaskCreated`, `TaskCompleted`, `TeammateIdle`, `PreToolUse`, `PermissionRequest`, `Notification` (`agent_needs_input`, `agent_completed`), `WorktreeCreate/Remove`. Exit code 2 blocks or sends back (e.g. `TaskCompleted` exit 2 "Prevents the task from being marked as completed"). | Hooks act as quality gates and notification bridges. `/hooks` browses them. | CC-HOOK §Hook lifecycle, §Exit code 2 behavior per event; CC-TEAM §Enforce quality gates with hooks | grep CC-HOOK for `TeammateIdle` |
| Claude Code — dynamic workflows | Claude writes a JS script (`agent()`, `pipeline()`, `parallel()`, `phase()`, `log()`) run in an isolated runtime. Intermediate results stay in script variables. Deterministic relaunch (`Date.now()`/`Math.random()` throw). Stalled agents auto-restart. A "Large workflow" warning fires above 25 agents or 1.5M projected tokens. | `/workflows` progress view lists phases with agent counts, drill-down `Enter/→`, back `Esc/←`, `p` pause, `x` stop, `r` restart agent, per-agent token usage. An approval prompt shows planned phases. | CC-WF §Run a bundled workflow, §Watch the run (key table), §How a workflow runs, §Manage runs | grep CC-WF for `Large workflow` |
| Claude Code — checkpoints | A checkpoint is taken before each user prompt. Snapshots are kept for the last 100. `/rewind` or `Esc Esc` → restore code and/or conversation, or summarize. **Limits:** bash-only changes and most subagent edits are not restored. | Rewind menu lists prompts. Actions: restore code + conversation / conversation / code / summarize from or up to here. | CC-CKPT §How checkpoints work, §Limitations | grep CC-CKPT for `Subagent edits not restored` |
| Claude Code — worktree isolation | Background sessions move into `.claude/worktrees/` before editing. Isolation is enforced (writes to the shared checkout are blocked until the move). Work is preserved by commit+push or a draft PR. Deleting a session refuses to drop unpushed commits. | Row shows the PR label. `claude rm` refusal messages name the branch and the unpushed count. | CC-AV §How file edits are isolated, §What deleting a session removes | grep CC-AV for `bgIsolation` |
| **GitHub Copilot cloud agent — agents page** | Sessions run in an ephemeral dev environment on GitHub Actions. Commits are authored by Copilot with the human as co-author and link to session logs ("Verified"). | Agents panel or page lists sessions. The session log shows reasoning and tools, plus token usage and length. A prompt box under the log steers. **Stop session** ends the Actions run (pushed commits kept). Archive (cloud sessions can't be deleted). Natural-language query over past sessions. | GH-SESS (all sections); GH-MC "Real-time steering", "Centralizing your task management" | `curl` GH-SESS markdown and grep `after it finishes its current tool call` |
| **Google Jules — plan gate** | The `SessionState` enum lists `QUEUED`, `PLANNING`, `AWAITING_PLAN_APPROVAL`, `AWAITING_USER_FEEDBACK`, `IN_PROGRESS`, `PAUSED`, `FAILED`, `COMPLETED` (transitions not documented on that page). API flag `requirePlanApproval` ("If true, plans require explicit approval"). | **Approve plan** button. In the web UI, "If you navigate away, Jules will eventually auto-approve the plan, which is set on a timer." | JULES-PLAN; JULES-API §SessionState | grep JULES-PLAN for `auto-approve` |
| **Devin — managed Devins** | A coordinator session spawns child sessions, each "in its own isolated VM", with per-child prompt, playbook, tags, and ACU limit. It can message, sleep, or terminate children and schedule self-reminders. It is "woken automatically" when a child finishes or needs input. | Child sessions start without asking by default. Toggle **Auto-approve child sessions** to review batches. Children appear under the parent in the sidebar `[SECONDARY: release notes "Child Session Indentation"]`. | DEVIN-ADV §Managed Devins | grep DEVIN-ADV for `Auto-approve child sessions` |
| Devin — dynamic workflows | A Python script orchestrates agents: `agent(prompt, phase, schema)` returns structured dict, `pipeline()` (no barrier), `parallel()`. Every call is recorded against a run ID and keyed by a hash of prompt+schema+settings. On resume, completed calls replay. Max run budget is 7 days. | Workflow panel shows phases, agents, live status, and per-agent ACU. Stop "puts the remaining child sessions to sleep". | DEVIN-WF §Determinism and resuming, §Cost | grep DEVIN-WF for `keyed by a hash` |
| **Cursor — worktrees / best-of-N / cloud agents** | Agents Window runs agents in separate git worktrees (setup via `.cursor/worktrees.json`). `/best-of-n` runs the same task across models, one worktree each. Cloud agents run in isolated VMs, on a separate branch, and push for handoff. | Agents Window review. `/apply-worktree`, `/delete-worktree`. Cloud agents produce artifacts (screenshots, videos, logs) and allow remote-desktop takeover. | CUR-WT; CUR-CLOUD | grep CUR-WT for `best-of-n` |
| **Amp — Task subagent / Oracle** | Each subagent has its own context window and the same model as its thread. Subagents "can't communicate with each other, you can't guide them directly". A follow-up to a running Task becomes extra steering. Context auto-compacts. | Each delegated task shows as a subagent step in the thread. The main agent receives the final summary only. | AMP-SUB §Subagents | grep AMP-SUB for `can’t communicate with each other` |
| **Factory — custom droids** | `Task(subagent_type, prompt, run_in_background, resume)`. `TaskOutput` (`block`) and `TaskStop` (SIGTERM→SIGKILL). Subagents are non-interactive (`AskUser` disabled) and cannot spawn subagents. Autonomy levels are **clamped to the org Maximum Autonomy Level**. Read-only droids never exceed Low. | `/droids` manager. Foreground streams progress. Background returns a `task_id`. | FAC-SUB §Resuming and parallel runs, §Autonomy level | grep FAC-SUB for `clamped to the organization` |
| **OpenHands — TaskToolSet** | Synchronous sub-agent per call. The conversation persists to disk with `task_id`. `resume` reloads the full history. A `DelegationVisualizer` exists. (Parallel `DelegateTool`: **not verified on a primary page** — only `[SECONDARY]` DeepWiki/search.) | Parent blocks until the sub-agent returns status `completed`/`error`. | OH-TASK | grep OH-TASK for `resumed` |

### 2.2 Frameworks and protocols

| Capability | How it works (mechanism) | User/dev-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **Anthropic workflow taxonomy** | Prompt chaining, routing, parallelization (sectioning, voting), orchestrator-workers, evaluator-optimizer, autonomous agents. Recommends "the simplest solution possible", sandboxed testing, and stopping conditions. | Design vocabulary, not a UI. | ANT-BEA §Workflow: … | grep ANT-BEA for `Orchestrator-workers` |
| **OpenAI Agents SDK — handoffs** | Handoff = a tool `transfer_to_<agent>`. The receiver sees the full history unless `input_filter` / `nest_handoff_history` (beta) reshapes it. Distinct from `Agent.as_tool()` (nested specialist without transfer). | Code API. | OAI-HO §Input filters | grep OAI-HO for `transfer_to_` |
| OpenAI Agents SDK — guardrails | Input guardrails run on the first agent only. Output guardrails run on the final agent only. Tool guardrails run per function-tool call. Parallel (default) vs blocking execution. A tripwire raises an exception and halts. | Code API. | OAI-GR §Workflow boundaries, §Execution modes | grep OAI-GR for `run_in_parallel` |
| OpenAI Agents SDK — tracing | On by default. A trace holds spans (`trace_id`, `parent_id`). Built-in `agent_span`, `generation_span`, `function_span`, `guardrail_span`, `handoff_span`. Custom processors. A sensitive-data toggle. | OpenAI Traces dashboard or third-party exporters. | OAI-TR §Default tracing, §Custom tracing processors | grep OAI-TR for `handoff_span` |
| OpenAI Agents SDK — HITL | `needs_approval` on tools. The run pauses with `interruptions` → `result.to_state()` → `state.approve/reject` → resume with `Runner.run(agent, state)`. `RunState` is JSON-serializable and durable. Warnings: snapshots are unauthenticated, and "Approval … does not guarantee exactly-once execution". | App-built approval UI. | OAI-HITL §How the approval flow works, §Keep approval state on the server, §Recovering after a crash | grep OAI-HITL for `exactly-once` |
| **LangGraph — interrupts** | `interrupt()` inside a node needs a checkpointer and `thread_id`. Resume with `Command(resume=…)`. **The node restarts from its beginning**, so side effects before `interrupt` must be idempotent. | App-built UI. | LG-INT §Rules of interrupts | grep LG-INT for `restarts the entire node` |
| LangGraph — time travel | `get_state_history` → replay from a checkpoint (later nodes re-execute, including LLM calls). `update_state` forks a new branch. "does **not** roll back a thread". | Studio/app UI `[INFERENCE]`. | LG-TT §Replay, §Fork | grep LG-TT for `does \*\*not\*\* roll back` |
| LangChain multi-agent patterns | Subagents (agents as tools), handoffs (state-driven), skills, router, custom workflow. Comparison of model calls and tokens per pattern. `langgraph-supervisor` "is no longer actively maintained". | — | LC-MA §Patterns, §Performance; LG-SUPMIG | grep LG-SUPMIG for `no longer actively maintained` |
| **Microsoft Agent Framework** (successor to AutoGen + Semantic Kernel) | Orchestrations: Sequential, Concurrent, Handoff, Group Chat, Magentic. Checkpoints at the end of each **superstep** capture executor state, pending messages, pending requests/responses, and shared state. Rehydration requires identical topology/executor IDs. Pickle-based stores use a restricted unpickler: "Never load checkpoints from untrusted … sources." | — | MAF-OV; MAF-ORCH; MAF-CKPT | grep MAF-CKPT for `end of each superstep` |
| **CrewAI processes** | `Process.sequential` or `Process.hierarchical`. Hierarchical requires `manager_llm`/`manager_agent`, which plans, delegates, and validates. | — | CREW-PROC | grep CREW-PROC for `manager_llm` |
| **A2A protocol 1.0.0** | Agent Card (identity, skills, capabilities `streaming`/`pushNotifications`/`extendedAgentCard`, optional JWS signatures). Task states: SUBMITTED, WORKING, INPUT_REQUIRED, AUTH_REQUIRED (interrupted), COMPLETED, FAILED, CANCELED, REJECTED (terminal). Delivery by polling, SSE stream (`SendStreamingMessage`, `SubscribeToTask`), or webhook push. `contextId` groups tasks. | — | A2A §4.1.3 TaskState, §3.5, §4.4.1 | grep A2A for `TASK_STATE_INPUT_REQUIRED` |
| **MCP tasks (2025-11-25, experimental)** | Any request can be task-augmented → `CreateTaskResult` (`taskId`, `status: working`, `ttl`, `pollInterval`). `tasks/get`, `tasks/result` (blocks), `tasks/list`, `tasks/cancel`. States: working → input_required → completed/failed/cancelled. Status notifications are optional, so requestors must poll. "If context-binding is available", receivers MUST reject task operations from a different authorization context. | — | MCP-TASKS §Task Status, §Security Considerations | grep MCP-TASKS for `currently considered` |
| MCP elicitation | `elicitation/create` in `form` (flat schema) or `url` mode. Response `accept`/`decline`/`cancel`. Secrets **MUST NOT** use form mode. The client must show which server is asking. | Client renders a form or URL consent. | MCP-ELIC §Security Considerations | grep MCP-ELIC for `MUST NOT\*\* use form mode` |
| **OTel GenAI agent spans** | `create_agent {name}` (CLIENT). `invoke_agent {name}` (CLIENT for remote, INTERNAL for in-process). `gen_ai.operation.name` values include `invoke_workflow`, `execute_tool`, `plan`. Attributes: `gen_ai.agent.id/name/version/description`, `gen_ai.conversation.id`, `gen_ai.usage.*_tokens`. Status: **Development**. Do not record transient in-memory IDs as `gen_ai.agent.id`. | Any OTel backend. | OTEL-AGENT §Create agent span, §Invoke agent client span, §Invoke agent internal span | `curl` pinned raw file, grep `invoke_agent` |

---

## 3. UX flows (what the user sees and presses)

All flows are **documented** behaviour. I did not launch these products (no installs allowed). A replicator with the product installed can execute them (§7.3).

### 3.1 Claude Code — "mission control" loop (closest analogue to the operator's two facts)
Source: CC-AV §Quick start, §Switch sessions without leaving the terminal, §Keyboard shortcuts.
1. In a shell run `claude agents`. A full-screen table opens with a dispatch input at the bottom (the workspace-trust dialog may appear first).
2. Type a task and press `Enter`. A new background session row appears under **Working**. Each further prompt starts another session in parallel.
3. Rows regroup as state changes: **Needs input** (yellow), **Ready for review** (open PR / failing checks), **Completed**. The terminal tab shows `2 awaiting input · claude agents`. A desktop/terminal notification fires on needs-input, finish, or failure.
4. Select a row with `↑/↓` and press `Space` to **peek**: the exact pending question, result, or status sentence, plus `waiting 3m`. Type a reply and press `Enter`. A working session queues it. Numbered choices can be picked by digit. Permission dialogs need a full attach.
5. Press `Enter` or `→` to **attach**: the full session takes over, opening with a recap of what happened while away.
6. Press `←` on an empty prompt (or `/exit`, `Ctrl+Z`, double `Ctrl+C`) to **detach**. The session keeps running.
7. From any ordinary `claude` session, pressing `←` on an empty prompt backgrounds it and opens agent view with that row selected. The footer hints `← for agents`, later `← 2 agents` (waiting count) or `← 2 done`. `Esc` returns to the conversation.
8. `Ctrl+X` stops a session (twice = delete). Deletion removes the Claude-created worktree only when safe. Unpushed commits trigger a refusal naming branch and count.

### 3.2 Claude Code — in-session subagent tree
Source: CC-SUB §Let subagents spawn their own subagents, §Finished subagents in the panel and task list; CC-PAR §Check on running work.
1. Claude delegates. The transcript shows a tool row such as `code-improver(Suggest code improvements)`.
2. A panel under the prompt lists running subagents. Nested ones render as a tree, with `(+N)` on rows that still have descendants.
3. Open a row to see that subagent's siblings and direct children "with a path back to `main`". Type into its transcript to resume it yourself.
4. `/tasks` lists background items, including recently finished subagents (30 s). `Enter` opens the transcript, `x` stops.

### 3.3 Claude Code — agent team panel (experimental)
Source: CC-TEAM §Start your first agent team, §Talk to teammates directly.
1. Enable `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`, then ask in natural language for N teammates with roles.
2. Teammates appear in the agent panel below the prompt. More than 3 idle teammates collapse into `N idle agents`.
3. `↑/↓` select, `Enter` opens the transcript, and typing messages that teammate directly. `Esc` interrupts the viewed teammate. `x` stops. `Ctrl+T` toggles the shared task list.
4. Teammate permission prompts appear in the lead session. Plan-approval requests go to the lead.

### 3.4 Claude Code — workflow progress view
Source: CC-WF §Run a bundled workflow, §Watch the run.
1. `/deep-research <question>` (or a Claude-written workflow). An approval prompt lists planned phases (Desktop: Once/Always/Deny with a token-usage caution).
2. `/workflows` → phases with agent counts. `Enter/→` drills into a phase, then into an agent. `Esc/←` backs out.
3. `f` filters by status, `p` pauses/resumes, `x` stops an agent or the run, `r` restarts an agent. Per-agent token usage is visible.
4. At the end a single cited report arrives in the session. Claims that failed cross-checking are filtered. Claims that could not be verified are listed as "unverified".

### 3.5 Claude Code — rewind
Source: CC-CKPT §Rewind and summarize.
1. Press `Esc` twice on an empty prompt (or `/rewind`). A menu lists your prior prompts.
2. Choose: Restore code and conversation / Restore conversation / Restore code / Summarize from here / Summarize up to here.

### 3.6 GitHub Copilot — steer a cloud agent
Source: GH-SESS §Steer an agent session, §Stop a session, §Archive sessions.
1. Open the agents panel (any GitHub page) or `github.com/copilot/agents`, then select a session. The log and overview show progress, token usage, and duration.
2. Type a follow-up in the box under the log and press `Enter`. Copilot applies it "after it finishes its current tool call". Each steer consumes AI credits.
3. **Stop session** ends the Actions run and keeps pushed commits. Then use ⋯ → **Archive session** → **Yes, archive**.

### 3.7 Google Jules — plan approval
Source: JULES-PLAN.
1. Submit a task and Jules produces a plan. Optionally give chat feedback to revise it.
2. Click **approve plan**. If you leave, the plan auto-approves on a timer (UI). The API requires `requirePlanApproval: true` for a hard gate (JULES-API).

### 3.8 Devin — coordinator with children
Source: DEVIN-ADV §Managed Devins.
1. Prompt: "spin up a managed Devin for each module" (or let Devin decide).
2. Children launch automatically unless **Auto-approve child sessions** is off (Settings → Preferences).
3. The coordinator is woken on child finish or needs-input. It can message, sleep, or terminate children and watch per-child ACU.

### 3.9 OpenAI Agents SDK / LangGraph — programmatic approve-and-resume
Source: OAI-HITL §Example: pause, approve, resume; LG-INT §Pause using `interrupt`.
1. The run returns `result.interruptions` (OpenAI) or `__interrupt__` / `stream.interrupts` (LangGraph).
2. Persist: `result.to_state().to_json()` (OpenAI). Under LangGraph the checkpointer persists under `thread_id`.
3. A human decides, then `state.approve(i)` / `state.reject(i, rejection_message=…)` → `Runner.run(agent, state)`. Under LangGraph: `graph.invoke(Command(resume=value), config)` with the same `thread_id`.

---

## 4. Techniques and patterns

### 4.0 Evidence base: multi-agent vs single-agent

| Finding | Source |
|---|---|
| Opus 4 lead + Sonnet 4 subagents beat single-agent Opus 4 by **90.2%** on Anthropic's internal research eval. Token usage alone explains **80%** of BrowseComp variance (95% together with tool-call count and model choice). | ANT-MARS: "outperformed single-agent Claude Opus 4 by 90.2%"; "token usage by itself explains 80% of the variance" |
| Cost: agents ≈ **4×** chat tokens, multi-agent ≈ **15×**. Poor fit when "all agents [must] share the same context or involve many dependencies". | ANT-MARS: "multi-agent systems use about 15× more tokens than chats" |
| Early failure modes: "spawning 50 subagents for simple queries", "distracting each other with excessive updates". Synchronous lead means "the lead agent can't steer subagents". | ANT-MARS §Prompt engineering…, §Synchronous execution creates bottlenecks |
| Principles: "Share context, and share full agent traces, not just individual messages". "Actions carry implicit decisions, and conflicting decisions carry bad results". Default: single-threaded linear agent. | COG-DBMA *Principle 1*, *Principle 2* |
| Update: a narrower class works, "where agents contribute intelligence while writes stay single-threaded". Clean-context review loop catches "an average of 2 bugs per PR, of which roughly 58% are severe". Unstructured swarms "mostly a distraction"; the practical shape is "map-reduce-and-manage". Managers "assume they share state with their children when they don't". | COG-MAW |
| MAST: 1600+ traces, 7 frameworks, 14 failure modes in 3 categories (system design, inter-agent misalignment, task verification), κ = 0.88. "performance gains on popular benchmarks are often minimal". | MAST abstract |
| Flat self-coordination with locks: "Twenty agents would slow down to the effective throughput of two or three". Flat agents "became risk-averse". Planners/workers + judge with fresh cycles fixed most problems. An integrator role "created more bottlenecks than it solved". | CUR-SCALE |
| 16 parallel agents, ~2,000 sessions, 2B input / 140M output tokens, ~$20k → 100k-line C compiler. Coordination via lock files in `current_tasks/` + git. Parallelism collapsed on one giant task until a GCC oracle split it. "the task verifier [must be] nearly perfect". | ANT-CCC §Running Claude in parallel, §Make parallelism easy, §Evaluation |

### 4.1 Named techniques

Each entry gives a definition, then who does it and the evidence.

- **T1 Orchestrator–workers.** A central LLM decomposes dynamically, delegates, and synthesizes. Seen in ANT-BEA §Orchestrator-workers, ANT-MARS (LeadResearcher), DEVIN-ADV (coordinator), CREW-PROC (hierarchical manager), MAF-ORCH (Magentic), CC-SUB/CC-TEAM (lead).
- **T2 Parallel fan-out + synthesis (sectioning / voting / best-of-N).** Seen in ANT-BEA §Parallelization, CUR-WT `/best-of-n`, CC-WF `/deep-research` "votes on each claim", CC-TEAM parallel review with three lenses.
- **T3 Clean-context verifier / reviewer.** A second agent with no shared context checks output. Seen in COG-MAW (Devin Review), ANT-BEA (evaluator-optimizer), CC-WF ("adversarially verify each finding"), CUR-SCALE (judge), ANT-CCC (test oracle as verifier).
- **T4 Single writer, many advisors.** Read-only helpers plus one writer. COG-MAW (thesis). FAC-SUB `explorer` read-only. Amp Oracle `[SECONDARY]`. COG-DBMA notes June-2025 Claude Code subagents mostly "answering a question, not writing any code".
- **T5 Planner / worker / judge with periodic fresh starts.** Seen in CUR-SCALE.
- **T6 Handoff (control transfer) vs agent-as-tool (call and return).** Seen in OAI-HO (`transfer_to_*` vs `Agent.as_tool`), LC-MA (handoffs vs subagents), MAF-ORCH (Handoff). LC-MA quantifies the trade-off: subagents cost 4 calls one-shot vs 3 for handoffs, but use ~9K vs ~14K+ tokens multi-domain.
- **T7 Shared task list with dependencies + claim locks.** Seen in CC-TEAM ("Task claiming uses file locking"), ANT-CCC (`current_tasks/` lock files via git). Counter-evidence: CUR-SCALE lock bottleneck.
- **T8 Mailboxes / addressed messaging.** Seen in CC-TEAM (JSON inbox per agent; a send counts as successful only when the write succeeds), CC-SUB (`SendMessage` with name→ID identity check), DEVIN-ADV (coordinator messages children).
- **T9 Mission-control roster grouped by "needs human".** Seen in CC-AV, GH-SESS/GH-MC, DEVIN-ADV, CUR-WT (Agents Window).
- **T10 Detachable sessions under a supervisor daemon.** Sessions outlive the terminal. Seen in CC-AV §The supervisor process (restarts crashed workers, reaps idle ones after ~1h, keeps sessions through sleep, version-skew tolerant `roster.json`).
- **T11 Peek-and-reply and steering at tool boundaries.** Seen in CC-AV §Peek and reply, GH-SESS, AMP-SUB.
- **T12 Durable pause / approve / resume objects.** Seen in OAI-HITL `RunState`, LG-INT, MAF-CKPT (pending requests in checkpoint), A2A `INPUT_REQUIRED`/`AUTH_REQUIRED`, MCP-TASKS `input_required`, JULES-API `AWAITING_PLAN_APPROVAL`.
- **T13 Plan gate before writes.** Seen in JULES-PLAN, CC-TEAM (teammate in plan mode → approval request to lead), CC-WF (phase list approval).
- **T14 Checkpoints, rewind, and fork.** Seen in CC-CKPT, LG-TT (replay vs fork; fork never rewrites history), MAF-CKPT.
- **T15 Recorded, deterministic orchestration replay.** Seen in DEVIN-WF (hash-keyed call cache), CC-WF (non-determinism banned in scripts).
- **T16 Per-worker workspace isolation.** Seen in CC-AV/CC-SUB (worktrees, enforced), CUR-WT (worktrees), CUR-CLOUD/DEVIN-ADV (VMs), ANT-CCC (Docker per agent), Copilot (ephemeral Actions env, GH-SESS).
- **T17 Budgets and caps.** Seen in CC-SUB (concurrency 20, depth 3, `maxTurns` marks output "partial"), CC-WF (Large-workflow warning), DEVIN-ADV (per-child ACU limit), DEVIN-WF (7-day run budget), ANT-MARS (effort-scaling rules in prompt), FAC-SUB (autonomy clamp).
- **T18 Lifecycle hooks as policy and quality gates.** Seen in CC-HOOK (`TaskCompleted`/`TeammateIdle`/`SubagentStop` exit 2 sends work back; `PreToolUse` deny; `onFailure: block`).
- **T19 Agent messages are not human authority.** Seen in CC-TEAM §Messages between agents, CC-SUB ("no message from any agent counts as your approval"), CC-SUB §Subagent output scanning (reports scanned for injected instructions).
- **T20 Hierarchical tracing.** Seen in OAI-TR (spans with `parent_id`), OTEL-AGENT (`invoke_agent` nesting, `gen_ai.conversation.id`), ANT-MARS ("Adding full production tracing").
- **T21 Change provenance: commit → session log.** Seen in GH-SESS ("Each commit message includes a link to the session logs").
- **T22 Role-based model routing / "smart friend".** Seen in COG-MAW, CUR-SCALE ("use the model best suited for each role"), CC-TEAM/CC-SUB model precedence, FAC-SUB complexity→model tiers.
- **T23 Cheap-model status summaries.** Seen in CC-AV §Row summaries (Haiku-class summary at each turn end; ≤15 s refresh from local output without model calls).

### 4.2 Comparison table

The last column is `[INFERENCE]` throughout. It reads each technique against an OS-style runtime with a kernel scheduler, capability tokens, and an append-only audit log, which is maos's stated shape: TL-before-delivery, short-lived Spirit-bound capability tokens (`lunarpulse/dev_ws@d9bd59c4:README.md:77-79,162-164`).

| Technique | Who does it | Evidence | Relevance to an OS-style runtime `[INFERENCE]` |
|---|---|---|---|
| T1 Orchestrator–workers | Anthropic Research, Devin, CrewAI hierarchical, MAF Magentic, Claude Code | ANT-BEA, ANT-MARS, DEVIN-ADV, CREW-PROC, MAF-ORCH | Map naturally to Orchestrator Spirit → Workers. The kernel should own spawn/kill/accounting so the orchestrator is a *policy*, not a privileged process. |
| T2 Fan-out + synthesis | Claude Code `/deep-research`, Cursor best-of-N | CC-WF, CUR-WT | Fan-out is a scheduler concern (admission, concurrency cap). Synthesis is user-space. Each fork should carry a derived, narrower capability set. |
| T3 Clean-context verifier | Devin Review, Cursor judge, Claude workflows | COG-MAW, CUR-SCALE, CC-WF | A verifier is cheap to model as a read-only Spirit with no write capability. Its verdict belongs in the audit log as a first-class record. |
| T4 Single writer, many advisors | Cognition, Factory explorer | COG-MAW, FAC-SUB | Capability tokens can *enforce* single-writer per resource: write capability for a path or branch is granted to one Worker at a time. |
| T5 Planner / worker / judge + fresh starts | Cursor | CUR-SCALE | Fresh starts are a kernel-schedulable "respawn from checkpoint". The judge decision is an audit-log row. |
| T6 Handoff vs agent-as-tool | OpenAI SDK, LangChain, MAF | OAI-HO, LC-MA | Handoff = transfer of the *session's* authority and should be an audited capability transfer. Agent-as-tool = call and return within the caller's authority. |
| T7 Shared task list + claim locks | Claude agent teams, Anthropic compiler harness | CC-TEAM, ANT-CCC; counter: CUR-SCALE | A kernel-mediated claim (CAS over a task table, journaled) avoids "agents … forget to release" locks: leases with TTL, like short-lived tokens. |
| T8 Addressed mailboxes | Claude teams / `SendMessage`, Devin | CC-TEAM, CC-SUB, DEVIN-ADV | An IPC bus with the identity check that Claude does by name→ID. Every frame should be journaled before delivery. |
| T9 Needs-human roster | Claude agent view, Copilot, Devin, Cursor | CC-AV, GH-SESS, DEVIN-ADV, CUR-WT | The roster is a *read model* over kernel state plus the halt queue. It belongs in `maosctl` and must not require attaching to the process. |
| T10 Detach under supervisor | Claude Code supervisor | CC-AV §The supervisor process | maos's host daemon already plays the supervisor role. Attach/detach becomes a door verb, and sessions persist independently of `maosctl`. |
| T11 Steer at tool boundary | Copilot, Claude, Amp | GH-SESS, CC-AV, AMP-SUB | The scheduler's natural preemption point is between mediated calls. Queue operator messages and deliver them at the next mediation boundary, with an audit row. |
| T12 Durable pause / approve / resume | OpenAI SDK, LangGraph, MAF, A2A, MCP tasks, Jules | OAI-HITL, LG-INT, MAF-CKPT, A2A, MCP-TASKS, JULES-API | maos's epistemic halt is the same primitive. The OpenAI warnings (unauthenticated snapshots, no exactly-once) argue for kernel-held state and signed approvals. |
| T13 Plan gate | Jules, Claude teams, Claude workflows | JULES-PLAN, CC-TEAM, CC-WF | Plan approval as a posture-dependent gate (`cautious` = required). Beware Jules's auto-approve timer, which is the opposite of fail-closed. |
| T14 Checkpoint / rewind / fork | Claude Code, LangGraph, MAF | CC-CKPT, LG-TT, MAF-CKPT | An append-only log forbids rewriting history. LangGraph's "fork, never roll back" model fits: rewind = new branch referencing an old checkpoint. |
| T15 Deterministic orchestration replay | Devin workflows, Claude workflows | DEVIN-WF, CC-WF | If every Worker call is journaled keyed by content hash, resume can replay from the log, so the audit log doubles as a memo table. |
| T16 Per-worker isolation | Claude, Cursor, Devin, Copilot, Anthropic harness | CC-AV, CUR-WT, CUR-CLOUD, DEVIN-ADV, ANT-CCC | Maps to sandbox tiers T0–T3 (`README.md` / `STABILITY.md:67`). Workspace isolation (worktree) is orthogonal to process isolation and should be a separate, declared capability. |
| T17 Budgets / caps | Claude, Devin, Factory, Anthropic | CC-SUB, CC-WF, DEVIN-ADV, FAC-SUB, ANT-MARS | A kernel scheduler can enforce concurrency, depth, turn, and token/ACU budgets uniformly, where products spread them across env vars and prompts. |
| T18 Lifecycle hooks as gates | Claude Code | CC-HOOK | Equivalent to kernel event subscriptions with veto. Each veto must be logged, and hook failure should be fail-closed by default (Claude makes this opt-in, `onFailure: block`). |
| T19 Agent message ≠ human authority | Claude Code | CC-TEAM, CC-SUB | Capability tokens bound to the *operator principal* cannot be minted by an agent message. This is structurally stronger than a classifier. |
| T20 Hierarchical tracing | OpenAI SDK, OTel GenAI | OAI-TR, OTEL-AGENT | Audit-log frames could carry `trace_id`/`parent_id` and map to `invoke_agent`/`execute_tool` spans for export. OTel status is Development, so treat it as an exporter, not the source of truth. |
| T21 Commit → log provenance | GitHub Copilot | GH-SESS | Worker commits could embed a TL frame ID and audit-query handle, making code review traceable to the log. |
| T22 Role-based model routing | Cognition, Cursor, Claude, Factory | COG-MAW, CUR-SCALE, FAC-SUB | Model choice per Spirit role is manifest data. Org/operator caps (Factory clamp) map to posture. |
| T23 Cheap status summaries | Claude Code | CC-AV §Row summaries | The roster needs a "what is it doing" line. Generate it in user-space (a summarizer Spirit), not in the kernel, which "learns nothing" (`README.md:80-82`). |

---

## 5. Limits, risks, costs

1. **Token cost multiplies with agents.** ~15× chat for multi-agent (ANT-MARS). Agent teams "use significantly more tokens than a single session" and cost scales linearly per teammate (CC-TEAM §Token usage). Each agent-view session draws quota independently (CC-AV §Quick start step 2). The C compiler run cost ~$20k (ANT-CCC). Copilot steering messages each consume credits (GH-SESS).
2. **Context loss and conflicting implicit decisions** when parallel writers lack each other's traces (COG-DBMA). Teammates "don't inherit the lead's conversation history" (CC-TEAM §Context and communication). Managers wrongly assume shared state (COG-MAW).
3. **Coordination overhead and lock pathologies.** Lock bottlenecks and forgotten locks (CUR-SCALE). Agent-team task status "can lag" and block dependents (CC-TEAM §Limitations). "Two teammates editing the same file leads to overwrites" because teams don't isolate in worktrees (CC-TEAM §Avoid file conflicts).
4. **Autonomous spawning without confirmation.** With teams enabled, a named subagent "launches as a teammate", and "Claude Code doesn't ask you to confirm the launch" (CC-TEAM §How Claude starts agent teams). Devin starts children "without asking first" by default (DEVIN-ADV).
5. **Fail-open defaults on human gates.** Jules auto-approves plans on a timer in the UI (JULES-PLAN). Claude hooks that crash let actions through unless `onFailure: "block"` (CC-HOOK §Block the action when a hook fails). A teammate plan in plan mode is auto-approved unless its mode was changed (CC-TEAM §Have teammates plan before implementing).
6. **Resume and rewind gaps.** `/resume` and `/rewind` don't restore in-process teammates (CC-TEAM §Limitations). Rewind doesn't restore bash-only changes or most subagent edits (CC-CKPT §Limitations). LangGraph resume re-runs the whole node, and replay re-fires LLM/API calls (LG-INT, LG-TT). OpenAI: approval ≠ exactly-once, use idempotency keys (OAI-HITL §Recovering after a crash).
7. **State integrity is a trust boundary.** `RunState.from_json()` does not authenticate snapshots (OAI-HITL). MAF pickle-backed checkpoint stores: "Never load checkpoints from untrusted or potentially tampered sources" (MAF-CKPT §Security). MCP tasks: when context-binding is available, operations from another authorization context MUST be rejected; rate limiting is only a SHOULD (MCP-TASKS §Security Considerations).
8. **Prompt-injection through agent outputs and messages.** Claude Code scans subagent reports and labels inter-agent messages (CC-SUB §Subagent output scanning; CC-TEAM §Messages between agents). The need for these controls is itself evidence of the risk.
9. **Permission-prompt friction** when many workers bubble prompts to one human: "Teammate permission requests bubble up to the lead, which can create friction" (CC-TEAM §Too many permission prompts).
10. **Verification is the ceiling.** "it is easy to see tests pass and assume the job is done" (ANT-CCC). MAST's third failure category is task verification (MAST).
11. **Maturity.** Agent view is "research preview" (CC-AV). Agent teams are experimental (CC-TEAM). MCP tasks are experimental (MCP-TASKS). OTel GenAI agent spans are Development status (OTEL-AGENT). `langgraph-supervisor` is unmaintained (LG-SUPMIG). OpenAI nested handoff history is beta (OAI-HO).
12. **Docs move fast.** Claude Code pages cite versions up to v2.1.309 inside the text (CC-TEAM §Permissions). Re-fetch and diff before relying on details (§7).

---

## 6. Candidate lessons for maos

All lessons are `[INFERENCE]`. maos surfaces named are from `lunarpulse/dev_ws@d9bd59c4:README.md:162-164` (capability tokens: "unforgeable, short-lived, and Spirit-bound"), `:224-241` (`maosctl` verbs: install/start/stop/unload/posture/halt/audit/skills), `:77-82` (TL-before-delivery; "the kernel learns nothing"), and `RELEASE-HOLDS.md:66` (codex vs claude Worker completion-oracle asymmetry).

1. **L1 `maosctl agents` roster as a read model** `[INFERENCE]`. State-grouped list (needs-halt-resolution first, then working, then done) with dispatch/peek/attach/detach and a `--json` form like `claude agents --json`. Touches `maosctl`, the operator door, and the halt queue.
2. **L2 Parent→child tree navigation for Orchestrator-spawned Workers** `[INFERENCE]`. A tree with descendant counts and a "path back to main", as in CC-SUB. Touches the Orchestrator Spirit registry and the `maosctl` TUI/REPL.
3. **L3 "Needs operator" is a first-class state, distinct from blocked/idle, with a `waitingFor` reason** (halt, approval, capability request) `[INFERENCE]`. Touches epistemic halt, the Approval Log, and door notifications.
4. **L4 Steering = an operator frame queued to the next mediation boundary** `[INFERENCE]`. Journal it before delivery, so steering is auditable like any call. Touches kernel mediation and the Transparency Log.
5. **L5 Single-writer leases via capability tokens** `[INFERENCE]`. Grant write capability on a workspace/branch to one Worker at a time, with TTL, and let read-only advisor/verifier Spirits multiply freely. Encodes COG-MAW's thesis structurally. Touches the Capability Registry and the scheduler.
6. **L6 A clean-context verifier Spirit as a default stage** `[INFERENCE]`, especially where a Worker adapter lacks an effect oracle (the claude adapter per `RELEASE-HOLDS.md:66`). Touches the Worker CLI adapters and the Orchestrator plan.
7. **L7 Journal-keyed deterministic replay** `[INFERENCE]`. Key Worker invocations by content hash (prompt + manifest + capability scope) so an interrupted orchestration resumes by replaying TL results (DEVIN-WF pattern). Touches the Transparency Log, Journal, and Orchestrator.
8. **L8 Fork, never rewind** `[INFERENCE]`. A rewind/retry creates a new branch of state that references an old checkpoint frame, preserving the append-only invariant (LG-TT model). Touches Journal and TL.
9. **L9 Kernel-enforced budgets per Spirit tree** `[INFERENCE]`: concurrency, depth, turns, tokens/ACU, wall-clock. Return typed "limit reached, do not retry" errors (CC-SUB). Touches the kernel scheduler and posture.
10. **L10 Gates fail closed by posture** `[INFERENCE]`. No auto-approve timers (contrast JULES-PLAN). Hook/verifier failure blocks under `cautious`. Touches `maosctl posture` and the Approval Log.
11. **L11 Agent messages can never satisfy an operator approval** `[INFERENCE]`. Enforce it by principal-bound tokens rather than classifiers (contrast CC-TEAM's classifier). Touches capability tokens and the IAC bus.
12. **L12 OTel export mapping** `[INFERENCE]`. Map TL frames to `invoke_agent`/`execute_tool` spans with `gen_ai.conversation.id`, as an *exporter* only, since the spec is Development status. Touches telemetry broadcast (I7) and the audit export.
13. **L13 Commit provenance** `[INFERENCE]`. Worker commits carry a TL frame/audit handle (GH-SESS pattern). Touches Worker git adapters and `maosctl audit query`.

---

## 7. Replication appendix

### 7.1 Environment
- Any POSIX shell with `curl`, `grep` (or `rg`), and `git`. No product installs are needed for §7.2.
- Docs are live pages. Record `date -u` and save copies (`curl -sL URL -o file`) before grepping. If a quote is missing, search the page's newer wording before concluding it changed.

### 7.2 Fetch-and-grep checks (one per key claim)
```sh
# Claude Code — agent view, ← shortcut, supervisor, JSON roster
curl -sL https://code.claude.com/docs/en/agent-view.md -o cc-av.md
grep -n 'claude agents' cc-av.md | head
grep -n 'Press `←` on an empty prompt' cc-av.md
grep -n 'The supervisor is a background service' cc-av.md
grep -n 'waitingFor' cc-av.md
grep -n 'Haiku-class model' cc-av.md
grep -n 'bgIsolation' cc-av.md

# Claude Code — subagent tree, limits, resume, message trust
curl -sL https://code.claude.com/docs/en/sub-agents.md -o cc-sub.md
grep -n 'as a tree in the subagent panel' cc-sub.md
grep -n 'Concurrent subagent limit reached' cc-sub.md
grep -n 'up to three layers below the main conversation' cc-sub.md
grep -n 'no message from any agent counts as your approval' cc-sub.md

# Claude Code — agent teams
curl -sL https://code.claude.com/docs/en/agent-teams.md -o cc-team.md
grep -n 'file locking' cc-team.md
grep -n 'inboxes/{agent-name}.json' cc-team.md
grep -n "doesn't ask you to confirm the launch" cc-team.md
grep -n "can't approve a permission prompt" cc-team.md

# Claude Code — hooks, workflows, checkpoints, overview
curl -sL https://code.claude.com/docs/en/hooks.md | grep -n 'TeammateIdle\|TaskCompleted'
curl -sL https://code.claude.com/docs/en/workflows.md | grep -n 'Large workflow\|Date.now()'
curl -sL https://code.claude.com/docs/en/checkpointing.md | grep -n 'Subagent edits not restored'
curl -sL https://code.claude.com/docs/en/agents.md | grep -n "Agent teams don't isolate teammates"

# GitHub Copilot
curl -sL 'https://docs.github.com/api/article/body?pathname=/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents' \
  | grep -n 'after it finishes its current tool call\|link to the session logs'

# Jules, Devin, Cursor, Amp, Factory, OpenHands
curl -sL https://jules.google/docs/review-plan/ | grep -n 'auto-approve'
curl -sL https://jules.google/docs/api/reference/types/ | grep -n 'AWAITING_PLAN_APPROVAL\|requirePlanApproval'
curl -sL https://docs.devin.ai/work-with-devin/advanced-capabilities | grep -n 'Auto-approve child sessions\|isolated VM'
curl -sL https://docs.devin.ai/work-with-devin/dynamic-workflows | grep -n 'keyed by a hash'
curl -sL https://cursor.com/docs/configuration/worktrees.md | grep -n 'best-of-n'
curl -sL https://cursor.com/docs/cloud-agent.md | grep -n 'isolated VMs'
curl -sL https://ampcode.com/docs/models-and-subagents | grep -n 'communicate with each other'
curl -sL https://docs.factory.com/harness/subagents.md | grep -n 'clamped to the organization'
curl -sL https://docs.openhands.dev/sdk/guides/task-tool-set | grep -n 'resumed'

# Evidence posts and paper
curl -sL https://www.anthropic.com/engineering/multi-agent-research-system | grep -o 'outperformed single-agent Claude Opus 4 by 90.2%\|about 15× more tokens\|explains 80% of the variance'
curl -sL https://www.anthropic.com/engineering/building-c-compiler | grep -o 'current_tasks/\|\$20,000 in API costs'
curl -sL https://cognition.com/blog/dont-build-multi-agents | grep -o 'share full agent traces, not just individual messages'
curl -sL https://cognition.com/blog/multi-agents-working | grep -o 'writes stay single-threaded\|roughly 58% are severe\|map-reduce-and-manage'
curl -sL https://cursor.com/blog/scaling-agents | grep -o 'effective throughput of two or three\|risk-averse'
curl -sL 'http://export.arxiv.org/api/query?id_list=2503.13657' | grep -o '14 unique modes'
curl -sL https://www.anthropic.com/engineering/building-effective-agents | grep -o 'Orchestrator-workers\|Evaluator-optimizer'

# Frameworks / protocols
curl -sL https://openai.github.io/openai-agents-python/handoffs/ | grep -o 'transfer_to_refund_agent'
curl -sL https://openai.github.io/openai-agents-python/guardrails/ | grep -o 'run_in_parallel=False'
curl -sL https://openai.github.io/openai-agents-python/tracing/ | grep -o 'handoff_span()'
curl -sL https://openai.github.io/openai-agents-python/human_in_the_loop/ | grep -o 'does not guarantee exactly-once execution'
curl -sL https://docs.langchain.com/oss/python/langgraph/interrupts.md | grep -n 'restarts the entire node'
curl -sL https://docs.langchain.com/oss/python/langgraph/use-time-travel.md | grep -n 'roll back a thread'
curl -sL https://docs.langchain.com/oss/python/migrate/langgraph-supervisor.md | grep -n 'no longer actively maintained'
curl -sL https://learn.microsoft.com/en-us/agent-framework/workflows/checkpoints | grep -o 'end of each superstep'
curl -sL https://learn.microsoft.com/en-us/agent-framework/overview/ | grep -o 'direct successor'
curl -sL https://docs.crewai.com/en/concepts/processes | grep -o 'manager_llm'
curl -sL https://a2a-protocol.org/latest/specification/ | grep -o 'Latest Released Version[^*]*\|TASK_STATE_INPUT_REQUIRED' | sort -u
curl -sL https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks.md | grep -n 'currently considered'
curl -sL https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation.md | grep -n 'MUST NOT\*\* use form mode'

# OpenTelemetry (pinned SHA; the old semconv path now says "Moved")
git ls-remote https://github.com/open-telemetry/semantic-conventions-genai HEAD   # observed 51a0f580…
curl -sL https://raw.githubusercontent.com/open-telemetry/semantic-conventions-genai/51a0f580f8a6ad804ca4de5e033683c6fbbe5906/docs/gen-ai/gen-ai-agent-spans.md \
  | grep -n 'invoke_agent\|Status.*Development' | head
```

### 7.3 Interactive verification (not executed in this study; requires the product)
- **Claude Code agent view:** install Claude Code (version supporting agent view; the docs reference ≥ v2.1.2xx features). In a git repo:
  1. Run `claude`, send one message, press `←` on the empty prompt. Expect agent view with the row selected and the footer `Your conversation moved to the background`.
  2. Type a prompt and press `Enter` (new row). Press `Space` (peek), `→` (attach), then `←` on an empty prompt (detach).
  3. Run `claude agents --json --all` and confirm the `state`/`waitingFor` fields.
- **Claude Code subagent tree:** ask Claude to run a reviewer subagent that dispatches a verifier per finding. Observe nested rows with `(+N)` in the panel.
- **GitHub Copilot:** assign an issue to Copilot, open `github.com/copilot/agents`, steer via the prompt box, then **Stop session**.

### 7.4 maos references
```sh
git -C /home/lunarpulse/dev_ws rev-parse HEAD     # d9bd59c456a68a9aaddbbee4594796beed288b91
sed -n '77,82p;162,166p;224,241p' /home/lunarpulse/dev_ws/README.md
sed -n '66p' /home/lunarpulse/dev_ws/RELEASE-HOLDS.md
```

---

## 8. Open questions

1. **Lineage of the left-arrow and `agents` affordances.** Do codex's `codex agents` and omp's left-arrow navigation predate, follow, or independently converge with Claude Code's `claude agents` and `←`? This needs dates from sibling studies 01/02 and the products' changelogs. Not searched here.
2. **Quantified benefit of mission-control UIs.** I found no published evaluation (searched vendor docs and blogs above) that measures operator throughput or error rate with roster/peek/attach vs per-session terminals. Evidence is design rationale only.
3. **Agent-team effectiveness on writes.** Claude docs recommend starting teams on "research and review", and Cognition argues for single-writer. Is there any controlled comparison of parallel *writers* with worktree isolation vs a single writer on real repos? Not found. CUR-SCALE and ANT-CCC are single anecdotal runs with test oracles.
4. **OpenHands parallel `DelegateTool`.** It exists per secondary sources. The primary doc page was not found at `docs.openhands.dev/sdk/guides/agent-delegation(.md)` (grep returned no matches). Needs a source-level check in the OpenHands SDK repo.
5. **Jules concurrency and daily quotas** (15/3, 100/15, 300/60) are `[SECONDARY]` (search summary of `jules.google/docs/usage-limits/`), and that page was not fetched. Verify before citing.
6. **Devin child-session tree UI.** The "Child Session Indentation" release note and multi-level manager trees are `[SECONDARY]` (search results citing `docs.devin.ai/release-notes/2025` and an X post). Verify on primary pages.
7. **OTel stability.** When will `invoke_agent`/`invoke_workflow` leave Development? Should maos pin a semconv-genai SHA in its exporter?
8. **How should a kernel scheduler arbitrate operator steering vs in-flight capability tokens** (TTL ≤ 60 s) when a steer arrives mid-call? Products only document "after the current tool call". maos would need a precise rule `[INFERENCE]`.
9. **Can deterministic replay coexist with GDPR erasure in an append-only log?** If TL results serve as a replay cache (L7), an erasure cascade could invalidate replays. Unresolved `[INFERENCE]`.
