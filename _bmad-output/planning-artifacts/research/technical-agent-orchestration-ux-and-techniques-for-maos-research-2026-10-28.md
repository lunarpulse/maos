---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - _bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/01-oh-my-pi.md
  - _bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/02-codex.md
  - _bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/03-landscape.md
  - _bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/04-maos-baseline.md
  - _bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/05-citation-check.md
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Agent orchestration UX and techniques for maos (learning from oh-my-pi, openai/codex and the 2026 landscape)'
research_goals: 'An evidence-based, replicable report for agents to discuss and learn from: verify the operator''s two observations (codex `codex agents`; omp ←← subagent view), compare orchestration UX and techniques, and derive how maos can evolve orchestration in a maos way (kernel mechanism, one door, Transparency Log, T3 Workers), mapped onto the existing backlog'
user_name: 'Lunarpulse'
date: '2026-10-28'
web_research_enabled: true
source_verification: true
execution_mode: 'autonomous — delegated synthesis (agent SynthesisReport); every step''s [C] menu was taken without an operator prompt; scope came from the delegating brief'
review: '_bmad-output/planning-artifacts/research/agent-orchestration-2026-10-28/06-adversarial-review.md (2026-10-28; 31 findings, all applied; see the Changelog at the end)'
pinned_sources:
  omp: 'https://github.com/can1357/oh-my-pi @ 9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1'
  codex: 'https://github.com/openai/codex @ e6de6a2423f9a723c098db3d17ea6c71e93bfcf2'
  maos: 'lunarpulse/dev_ws @ d9bd59c456a68a9aaddbbee4594796beed288b91'
  otel_genai: 'open-telemetry/semantic-conventions-genai @ 51a0f580f8a6ad804ca4de5e033683c6fbbe5906'
  web_access_date: '2026-10-28'
---

# Research Report: technical

**Date:** 2026-10-28
**Author:** Lunarpulse (synthesised by the `SynthesisReport` agent from four phase-R1 studies)
**Research Type:** technical

---

## Research Overview

The operator asked for a report that agents can discuss and learn from. It covers agent-orchestration UX and techniques in oh-my-pi (omp) and openai/codex, with the wider 2026 landscape for context. The aim is for maos to evolve its orchestration *in a maos way*. The report draws on four phase-R1 studies written on 2026-10-28 against pinned sources: omp @ `9d16dff`, codex @ `e6de6a24`, live vendor documentation and papers, and maos @ `d9bd59c4`. A fifth note, an independent citation check ([05-citation-check](agent-orchestration-2026-10-28/05-citation-check.md)), audited 48 load-bearing R1 claims: 44 confirmed, 3 line-drift, 1 partial, 0 not found, 0 contradicted. This report uses its corrected lines and wording. Before relying on load-bearing claims, this synthesis also checked them again against the primary sources (§F.4 lists every command that was re-run). **Nothing in this report was runtime-verified**: no omp, codex or Claude Code UI was launched, and no maos binary was built or run. Every UX flow described here is reconstructed from code, golden snapshots or vendor docs. The only runtime evidence is a set of host observations, listed in §F.4.4.

Both operator observations hold, with one correction each. **omp**: a double-tap of ← (two presses 40–500 ms apart) on an empty editor in the *main* session opens the **Agent Hub**, which lists subagents. Pressed while you are viewing a child, the same gesture returns to *main*, not to the parent. **codex**: `codex agents` opens the **"Agent command center"**, a client view over sessions held by a shared app-server daemon. It lists *root* sessions only and rolls subagent state up into those rows. Claude Code's `claude agents` ("agent view") documents a similar design (documented-only; which product came first, and whether the designs are independent, is not established). Across the three products the common verbs are: a roster sorted by "needs you", attach/detach to sessions held by a server, queued steering applied at turn or tool boundaries *next to* an explicit interrupt, and limits on fan-out (source-enforced in omp and codex, documented for Claude Code).

maos already has most of the *mechanism* these surfaces need: a daemon that holds Spirits, one authenticated door, a live state store (SCB), an audit log (TL), and a safe-point steering buffer. It has almost none of the *operator surface*. The maos way is mostly a matter of exposing what exists, with no new kernel lines (kernel-Δ 0), through `maosctl` and the door, plus Orchestrator-side policy. The report answers the operator's ten required items in the sections listed below. The executive summary is in §F.1.

**Reading map (operator's required items → section)**

| # | Required item | Section |
|---|---|---|
| 1 | Executive summary + research questions | §F.1 (summary), §A.2 (questions), §F.2 (answers) |
| 2 | Verification of the two observations + Claude Code convergence | §D.1 |
| 3 | Comparative matrix | §D.2 |
| 4 | Technique catalogue with measured outcomes | §D.3 |
| 5 | The maos way | §E.1–§E.2 |
| 6 | Roadmap onto backlog + NEW candidate stories | §E.4 |
| 7 | Risks and anti-patterns | §E.3 |
| 8 | Discussion agenda for agents (BMad roles) | §F.3 |
| 9 | Replication recipe | §F.4 |
| 10 | Evidence index | §F.5 |

**Evidence conventions used throughout**

| Prefix | Meaning |
|---|---|
| `omp:` | `can1357/oh-my-pi@9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1:`. `CA/` = `packages/coding-agent/src/`, `TUI/` = `packages/tui/src/`, `docs/` = repo `docs/` |
| `cx:` | `openai/codex@e6de6a2423f9a723c098db3d17ea6c71e93bfcf2:codex-rs/` |
| `M:` | `maos@d9bd59c456a68a9aaddbbee4594796beed288b91:` (repo root `/home/lunarpulse/dev_ws`) |
| `CC-AV`, `CC-SUB`, `ANT-MARS`, … | Landscape source keys defined in [03 §Source keys](agent-orchestration-2026-10-28/03-landscape.md#source-keys); the URLs are repeated in §F.4.1. All were accessed 2026-10-28 |
| `[01 §n]` … `[05 §n]` | Sections of the R1 notes: [01-oh-my-pi](agent-orchestration-2026-10-28/01-oh-my-pi.md), [02-codex](agent-orchestration-2026-10-28/02-codex.md), [03-landscape](agent-orchestration-2026-10-28/03-landscape.md), [04-maos-baseline](agent-orchestration-2026-10-28/04-maos-baseline.md), and the citation audit [05-citation-check](agent-orchestration-2026-10-28/05-citation-check.md) |
| `[INFERENCE]` | Reasoning by this report. No cited source states it. |
| `[SECONDARY]` | Rests only on a search summary or other secondary page. Never load-bearing here. |
| ✔ re-run | This synthesis re-ran the check on 2026-10-28 (≈11:50–11:58Z) and saw the cited text (§F.4) |

---

## A. Technical Research Scope Confirmation (step 1)

**Research Topic:** Agent orchestration UX and techniques for maos, learning from oh-my-pi, openai/codex and the 2026 landscape.
**Research Goals:** An evidence-based, replicable report that agents can discuss and learn from. It verifies the operator's observations, compares UX and techniques, and turns the findings into maos-shaped principles and backlog proposals.

**Technical Research Scope:**

- Architecture Analysis: client/daemon splits, registries, session trees, lifecycle state machines
- Implementation Approaches: limits enforced in code vs prompt advice, steering points, isolation, structured results
- Technology Stack: TUIs, storage (JSONL + index), control-plane protocols
- Integration Patterns: JSON-RPC app-server, RPC verbs, the maos door, A2A/MCP/OTel
- Performance Considerations: fan-out caps, demand-driven fetch, cost and token multipliers, measured multi-agent outcomes

**Research Methodology:**

- Primary sources first: pinned source trees and vendor/primary docs. Secondary sources are used only to find primary pages.
- Load-bearing claims were checked again against the pinned source or a live page (§F.4). The independent audit in [05 §1–§3] was applied where it corrected R1 lines or wording.
- Confidence is shown by verification level (source-verified / documented-only / runtime-verified), not by adjectives (§F.4.4).
- maos claims cite `M:` file:line at `d9bd59c4` or a tracker key in `M:_bmad-output/implementation-artifacts/sprint-status.yaml`.

**Scope Confirmed:** 2026-10-28. The scope was confirmed autonomously: the delegating brief fixed it and no operator prompt was issued.

### A.1 Workflow execution record

| Skill step | What ran |
|---|---|
| Activation | `customize.toml` was read directly. `activation_steps_prepend/append` and `on_complete` are empty. No `_bmad/custom/bmad-technical-research*.toml` override exists (`ls _bmad/custom/` lists only code-review, dev-story and config files). `persistent_facts` = `file:**/project-context.md`, and the glob matched no file. Config: `M:_bmad/bmm/config.yaml` (user Lunarpulse, English, planning_artifacts). |
| Topic discovery | Skipped. The topic came from the brief. |
| Step 1 scope | §A |
| Step 2 technology stack | §B.1 |
| Step 3 integration patterns | §B.2 |
| Step 4 architectural patterns | §D (observations, matrix, techniques) plus §C (system patterns) |
| Step 5 implementation research | §E |
| Step 6 synthesis | §F, plus the Research Overview above |
| Web research | Phase R1 ([03 §7](agent-orchestration-2026-10-28/03-landscape.md#7-replication-appendix)) did the bulk. This synthesis re-fetched CC-AV, CC-SUB, ANT-MARS, COG-MAW, CUR-SCALE and arXiv MAST, and ran 2 discovery searches (§F.6) |

### A.2 Research questions

- **RQ1.** Are the operator's two observations accurate? What exactly do omp's ←← and `codex agents` do, and what do they show?
- **RQ2.** What do the omp, codex and Claude Code management surfaces share, and where do they differ (unit listed, server-held sessions, attach/detach, steering, revive/park, approvals)?
- **RQ3.** Which orchestration techniques have source evidence behind them, and which have *measured* outcomes?
- **RQ4.** What does maos have today against each technique, and what is missing?
- **RQ5.** How must each borrowed technique change to respect maos's binding constraints?
- **RQ6.** Which existing backlog keys already carry parts of this, and which NEW candidate stories would close the rest, in what order?
- **RQ7.** Which risks and anti-patterns must maos avoid?
- **RQ8.** What is still open for a multi-agent discussion, and who should argue which side?

---

## B. Technology Stack and Integration Analysis (steps 2–3)

### B.1 Technology Stack Analysis

#### Programming Languages and Frameworks

| System | Language / structure | UI layer and its tests | Evidence |
|---|---|---|---|
| omp 18.13.2 | TypeScript monorepo (`packages/coding-agent`, `packages/tui`). Installed host binary `omp/18.13.1` (single 276 MB executable) | Own TUI. The ←← gesture is pinned by tests (`test/input-controller-escape.test.ts:1171-1226`) | [01 §Sources, §4.10] |
| codex | Rust workspace `codex-rs/`: `cli`, `tui`, `app-server*`, `core`, `rollout`, `state`, `agent-graph-store`, `cloud-tasks` | TUI behaviour is fixed by committed **insta golden snapshots** (e.g. `…command_center__live_center_columns.snap`). The dashboard is ≈11 k lines including tests | [02 header, §5, §7 R4] |
| Claude Code | Closed source. Documentation only | Agent view is "research preview" (CC-AV line 21, ✔ re-run) | [03 §2.1] |
| maos | Rust workspace: `maos-kernel-core`, `maos-control` (door), `maos-cli` (`maosctl`), `maos-bin` (composition root), `maos-shell`, `spirits/*` | No TUI agents view. `maosctl --plain`, `NO_COLOR` and `TERM=dumb` must emit zero ANSI (NFR-Ops-5, `M:crates/maos-cli/src/cli.rs:18-21`) | [04 §2.1, §5.1] |

_Language evolution:_ the two open-source references are written in different languages, but both split "agent runtime" from "operator view". maos's Rust and crate split already gives it the same seam: kernel/maos-bin on one side, maos-control/maos-cli on the other [INFERENCE].

#### Database and Storage Technologies

All four systems use **append-only transcripts plus a small index**:

| System | Transcript | Index / state | Evidence |
|---|---|---|---|
| omp | `~/.omp/agent/sessions/<cwd>/<ts>_<id>.jsonl`. Each subagent writes `<AgentId>.jsonl`/`.md`/`.json`/`.patch` in the parent's artifact dir. The child header carries `parentSession` | Process-global `AgentRegistry`, plus a persisted-roster scan on resume | `omp:docs/session.md:37-45,65-101`; `omp:CA/registry/agent-registry.ts:1-10`; host observation [01 §7.4] |
| codex | `sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl`. Archive moves it to `archived_sessions/` | SQLite `state_5.sqlite` (+ `thread_history_1`, `queue_1`, …), plus `AgentGraphStore` parent→child edges | `cx:rollout/src/recorder.rs:1770-1792`; `cx:state/src/sqlite.rs:48-53`; `cx:agent-graph-store/src/store.rs:13-55` |
| Claude Code | `agent-{agentId}.jsonl` subagent transcripts | `~/.claude/jobs/<id>/state.json`, `~/.claude/daemon/roster.json` | CC-AV §Where state is stored; CC-SUB |
| maos | TL rows in SQLite (e.g. `cli.subprocess.output` at the Worker's pid). `maosctl audit query` opens the DB directly | SCB (`SpiritControlBlock`) = live truth. Lifecycle journal + Approval Decision Log | `M:crates/maos-bin/src/worker_spawn.rs:1183-1189`; `M:crates/maos-cli/src/subcommands.rs:2901`; `M:crates/maos-control/src/lib.rs:5-6,510-512` |

_Implication:_ maos needs **no new store** for a roster or transcript. The SCB and TL already hold the required data. What is missing is a set of read projections [INFERENCE grounded in [04 §5.1] "a dashboard must not keep a third state"].

#### Development Tools and Platforms

- **Gesture and keymap safety:** omp debounces ←← (min gap 40 ms, max 500 ms, `omp:CA/modes/controllers/input-controller.ts:271-272`, ✔ re-run). codex validates remappable keymaps and rejects conflicts at load time (`cx:tui/src/keymap.rs:2040-2059,3711-3838`).
- **Golden renders as the UX contract:** codex insta snapshots [02 §7 R4]. maos has no equivalent today. Its journey tests count ANSI bytes (NFR-Ops-5) [04 §5.1].

#### Cloud Infrastructure and Deployment

- **Local daemon:** codex app-server reached over a Unix socket at `$CODEX_HOME/app-server-control/app-server-control.sock`, or `--remote ws/wss/unix` (`cx:app-server-transport/src/transport/mod.rs:56-73`). Claude Code uses a local supervisor process (CC-AV line 158, ✔ re-run). In maos, `maos run`/`maos shell`/`cohort-a2a-daemon` bind one loopback door (ADR-062).
- **Remote/cloud runners:** `codex cloud` with best-of-N 1–4 (`cx:cloud-tasks/src/cli.rs:15-61`); Copilot on GitHub Actions (GH-SESS); Devin with one VM per child (DEVIN-ADV); Cursor cloud agents on VMs (CUR-CLOUD). maos's cross-host leg is A2A over mTLS/TOFU to a *second daemon and door* (`M:crates/maos-bin/src/delegation.rs:3-39`; ADR-062 `:122-127`).

#### Technology Adoption Trends

- Three products ship a similar "agents" command center: omp Hub, codex command center, Claude Code agent view. They differ on what a row is (§D.1). Claude Code's is a research preview (CC-AV l.21) and documented-only. [03 §8 Q1] and [02 §5] could not establish which came first or whether the designs are independent (shallow codex clone; no dated omp history read).
- The surrounding standards are still immature: OTel GenAI agent spans are **Development** status (OTEL-AGENT); MCP tasks are experimental (MCP-TASKS); `langgraph-supervisor` is unmaintained (LG-SUPMIG) [03 §5.11].

### B.2 Integration Patterns Analysis

#### API Design Patterns: the operator control plane

| Verb | omp | codex | Claude Code | maos today |
|---|---|---|---|---|
| Transport | `--mode rpc` JSONL | JSON-RPC over UDS / ws (app-server) | CLI verbs + `--json` | HTTP/1.1 loopback, bearer, `Connection: close` |
| List | `get_subagents` | `thread/list`, `thread/loaded/list` | `claude agents --json [--all]` | `GET /v1/daemon` (no CLI consumer) |
| Read transcript | `get_subagent_messages{fromByte}` | `thread/read`, `thread/turns/list` | `claude logs <id>` | `maosctl audit query` (direct SQLite, post-hoc) |
| Steer | `steer_subagent` | `turn/steer`, `thread/queue/*`, `codex queue` | peek-reply (queued) | `POST /v1/orchestrator/{id}` (enqueue only) |
| Stop | `cancel_subagent` | `turn/interrupt` | `claude stop` | `unload` (Worker killed), `pause` (no-op for CLI Workers) |
| Subscribe | `set_subagent_subscription{off\|progress\|events}` | streamed `thread/*`, `turn/*`, `item/*` notifications | hooks; poll `--json` | none (stdout of `maos run` only) |
| Evidence | `omp:CA/modes/rpc/wire/commands.ts:149-185` | `cx:app-server-protocol/src/protocol/common.rs:552-901,1996-2083` | CC-AV §Manage sessions from the shell, l.824-834 ✔ | `M:crates/maos-control/src/lib.rs:233-253,1101-1118,1246-1284,1430-1525,1960` |

_Pattern:_ in every product the **UI is just one client of a verb set that scripts can also call**. maos already has this shape: one door, and every mutation is an `OperatorCommand` with an `operation_id` and a completion row written before the reply (`M:crates/maos-control/src/lib.rs:289-502`; ADR-062 `:168-174`). The missing verbs are reads.

#### Communication Protocols

- **codex** has one versioned JSON-RPC protocol. Requests include `thread/start|resume|fork|list|read|archive|…` and `turn/start|steer|interrupt`. The server pushes notifications (`thread/status/changed`, `thread/tokenUsage/updated`, …) and approval *requests* (`item/commandExecution/requestApproval`) [02 §1.10].
- **omp** uses in-process URLs (`agent://`, `history://`, `proc://`, `artifact://`, `local://`) for model-side addressing, and the RPC verbs above for hosts [01 §2].
- **Standards:** A2A 1.0.0 task states include `INPUT_REQUIRED` and `AUTH_REQUIRED` and use push via SSE or webhook. MCP tasks have `input_required`, polling, and *must* reject cross-authorization-context operations "if context-binding is available". OTel `invoke_agent`/`execute_tool` spans [03 §2.2].
- **maos:** typed-intent IAC frames are logged before delivery (I2, `M:docs/invariants/I2.md:13,30-32`). Cross-host `task.assign` carries `(peer, intent-class)` consent (ADR-012 `:12-14`). An ACP NDJSON editor surface runs as a separate one-shot root (`M:crates/maos-acp/src/frame.rs:19-77`).

#### Data Formats and Standards: status vocabularies

| Source | States | Evidence |
|---|---|---|
| omp registry | `running \| idle \| parked \| aborted` | `omp:TUI/overlays/agent-hub-types.ts:7` (`export type AgentStatus`, imported at `omp:CA/registry/agent-registry.ts:17`) [05 O7] |
| codex protocol | `NotLoaded \| Idle \| SystemError \| Active{WaitingOnApproval, WaitingOnUserInput}`, folded into **four** buckets: `enum AgentsOverviewGroup { NeedsYou, Working, Ready, Finished }`, ordered by derived `Ord`. These render as "Needs input ●", "Working ●", "Ready ○" and "Inactive ○". "Error !" is only how `NeedsYou` renders a root `SystemError`, so error rows sort with Needs input | `cx:app-server-protocol/src/protocol/v2/thread.rs:1701-1718`; `cx:tui/src/app/agents_overview_view.rs:69-75,78-92,434-446` [05 C7] |
| codex subagent tool | `pending_init \| running \| interrupted \| shutdown \| not_found \| {completed} \| {errored}` | `cx:core/src/tools/handlers/multi_agents_spec.rs:384-413` |
| Claude Code `--json` | `state`: working/blocked/done/failed/stopped, plus `waitingFor` (`permission prompt`, `input needed`, `sandbox request`, `worker request`, `dialog open`) | CC-AV l.824-834 ✔ |
| A2A 1.0.0 | SUBMITTED, WORKING, INPUT_REQUIRED, AUTH_REQUIRED, COMPLETED, FAILED, CANCELED, REJECTED | A2A §4.1.3 |
| maos | SCB `lifecycle_states[]` via `/v1/daemon`. No delegation-status vocabulary on any read route | `M:crates/maos-control/src/lib.rs:1101-1118` ✔; [04 §2.2(e)] |

#### System Interoperability Approaches

OTel GenAI spans (`invoke_agent` CLIENT/INTERNAL, `gen_ai.conversation.id`) would let maos export TL lineage without making OTel the source of truth. That matters because the spec is still Development status (OTEL-AGENT) [03 L12, INFERENCE].

#### Event-Driven Integration

- **Push results, not polling:** omp injects an `async-result` `<system-notice>` into the owner when a job settles (`omp:CA/prompts/tools/async-result.md:1-17`). codex notifications mutate the view, with a bounded reseed on start, reconnect and lag (`cx:tui/src/app/agents_overview_threads.rs:1-3,81-140`). Claude Code exposes hooks (`SubagentStop`, `TaskCompleted`, `Notification`) (CC-HOOK).
- **maos:** the only live feed is JSON lines on the `maos run` stdout (`spirit_loaded`, `delegation_routed`, `worker_completion`, `dispatch_halted`, …: `M:crates/maos-bin/src/main.rs:4941-5355`, `M:crates/maos-bin/src/worker_spawn.rs:1162-1300`). The door cannot stream (`M:crates/maos-control/src/lib.rs:233-253,1960`).

#### Integration Security Patterns

- **Authenticated control plane:** in maos the door authenticates *before* routing, compares the bearer in constant time, is loopback-only, and keeps `control.json` at mode 0600 (`M:crates/maos-control/src/lib.rs:198-209,658-668,1023-1029`; ADR-062 `:46-59`). The R1 study did not examine how codex authenticates its local socket. The only path seen was `$CODEX_HOME/app-server-control/…` [02 §5] (not studied; no claim made).
- **Saved state is a trust boundary:** OpenAI `RunState.from_json()` does not authenticate snapshots (OAI-HITL). MAF warns "Never load checkpoints from untrusted … sources" (MAF-CKPT). MCP tasks bind operations to the authorization context (MCP-TASKS) [03 §5.7].
- **Agent messages are not human authority:** "A teammate can't approve a permission prompt or supply consent on your behalf" (CC-TEAM). maos can enforce this *structurally*, because approvals are operator-principal rows (I4) [03 T19, L11].

---

## C. System Architecture Patterns behind the UX (step 4, part 1)

1. **Client/daemon split.** The UI is a viewer and a daemon owns the agents. codex: "Any running work continues" on exit (`cx:tui/src/app/exit_summary.rs:97` ✔). Claude Code: the supervisor keeps sessions running without a terminal (CC-AV l.158 ✔). omp is the exception: a process-global registry and bus (`omp:CA/irc/bus.ts:33-41`). It compensates with persisted-roster scans that bring children back as `parked` on resume (`omp:docs/agent-hub.md:96-98`).
2. **One registry, many views.** omp's `AgentRegistry` id feeds the Hub, pinned block, status line, `agent://`, `history://`, IRC and RPC [01 §4.1]. codex's `Thread` record carries `session_id`, `parent_thread_id`, `agent_nickname` and `agent_role` (`cx:app-server-protocol/src/protocol/v2/thread_data.rs:205-295`).
3. **Event-sourced read model.** A view is a projection over notifications plus periodic reseed (codex P8). In maos the TL is already the event source and the SCB is the snapshot [INFERENCE].
4. **Two-level hierarchy.** codex's dashboard lists roots and drills down per root (P4). omp's Hub lists all agents with a flat ↔ tree toggle. Claude Code separates agent view (sessions) from the subagent panel (tree).
5. **Re-point, don't copy.** When omp "opens" an agent, it re-points the main renderer at the child session ("exact parity by construction", `omp:TUI/overlays/agent-hub.ts:2505-2510`). Generation counters stop a stale revive from replacing a newer focus (`omp:CA/modes/controllers/session-focus-controller.ts:49-131`).

---

## D. Architectural Patterns and Design (step 4, part 2)

### D.1 Verification of the operator's two observations

#### Observation A: omp, "press ← twice at the parent session to see subagents". **Verified, with a correction.**

| Claim | Evidence | Level |
|---|---|---|
| Two ← presses on an **empty editor** form the gesture. The editor calls `onLeftAtStart` instead of moving the cursor | `omp:TUI/prompt/custom-editor.ts:1711-1717` | source |
| Window is `[40, 500)` ms. Faster terminal bursts and third taps are ignored | `omp:CA/modes/controllers/input-controller.ts:271-272` ✔ (`LEFT_DOUBLE_TAP_MIN_GAP_MS = 40`, `…MAX_GAP_MS = 500`), `:910-936`; tests `test/input-controller-escape.test.ts:1186-1214` | source ✔ |
| From the main session it calls `showAgentHub({ requireContent: true, armCloseTap: true })`, so it stays inert when there are no live or persisted subagents | `omp:CA/modes/controllers/input-controller.ts:765-781` ✔; `omp:CA/modes/controllers/selector-controller.ts:2554-2579` | source ✔ |
| Same Hub on `Alt+A` and `Ctrl+S` ("Open the agent hub"), and `/hub` (Activity) | `omp:TUI/app-keybindings.ts:214-221` ✔; `omp:CA/slash-commands/builtin-session.ts:605-614` | source ✔ |
| Hub = roster (status, id, parent, unread IRC, role/model, age, task/activity, `$cost · time · req · tok`) + inspector (tool, intent, retry, context gauge, lineage, patch/branch). Keys: `t` tree, `/` filter, `r` revive, `x` kill, `Enter` open (`agent-hub.ts:2406-2588`); `1`/`2` Agents/Activity (`agent-hub.ts:613-620`) | `omp:TUI/overlays/agent-hub.ts:613-620,1262-1284,1402-1445,2406-2588`; `omp:docs/agent-hub.md:24-69` [05 O4] | source |
| `Enter` re-points transcript, status line and editor at the child. Typing steers it, and `Esc`/←← return | `omp:CA/modes/controllers/session-focus-controller.ts:32-112` | source |
| **Correction:** pressed while viewing a child, ←← returns to **main**, not to the parent. The status text "←← hops to parent" is stale | Handler calls `unfocusSession()` (`input-controller.ts:817-821` ✔, and the code comment at `:765-766` ✔ says "returns the focused subagent view to the main session"); `focusParentSession` has no caller, and a test asserts it is not called (`test/input-controller-escape.test.ts:954-972`) [01 §5.4.1] | source ✔ |
| Installed `omp/18.13.1` vs studied 18.13.2 | `host:` `omp --version` [01 §Sources] | host observation; UX **not** runtime-verified |

#### Observation B: codex, "`codex agents` gives an agent-management dashboard where sessions are easy to enter and manage". **Verified, with a scoping note.**

| Claim | Evidence | Level |
|---|---|---|
| `codex agents` = clap `Agents(AgentsCommand)`: "Browse all agent sessions on the shared local app-server daemon." | `cx:cli/src/main.rs:147-148` ✔ | source ✔ |
| Title "Agent command center" | `cx:tui/src/app/agent_center/render.rs:150-152` ✔ | source ✔ |
| Starts the daemon if needed and refuses `--no-daemon` | `cx:cli/src/main.rs:2538-2555,2503-2508` | source |
| Enter/`→` attaches: `thread/read`, resumes if `NotLoaded`, keeps a per-thread draft | `cx:tui/src/app/agents_overview.rs:429-571` | source |
| Single-key management: `n` new, `w` worktree, `o` resume picker, `f` fork, `r` rename, `x` stop, `a` archive, `Backspace` delete, `h` hide, `p` pin, `g` regroup, `/` search, `Tab` filter | `cx:tui/src/keymap.rs:1973-1987`; `cx:tui/src/app/agents_overview_view.rs:699-826` | source |
| `←` on an empty composer opens it from inside a session ("← for agents") | `cx:tui/src/bottom_pane/chat_composer/agents_navigation.rs:30-39` ✔ | source ✔ |
| **Scope:** rows are **root** sessions. Subagent threads are filtered out (`parent_thread_id.is_none()` and not `SubAgent(ThreadSpawn)`), and their state is **rolled up** (most urgent wins) | `cx:tui/src/app/agents_overview_discovery.rs:77-84` ✔; `cx:tui/src/app/agents_overview.rs:1232-1243` ✔ | source ✔ |
| Leaving does not stop work: "Disconnected from this task. Any running work continues." | `cx:tui/src/app/exit_summary.rs:97` ✔ | source ✔ |
| Official doc wording: "Open the command center to browse chats and switch between agents on a shared server" | https://learn.chatgpt.com/docs/developer-commands?surface=cli [02 §1.1] | documented |
| Runtime behaviour | Not executed (no build allowed) [02 §7] | **not** runtime-verified |

#### Convergence: Claude Code `claude agents` (documented only)

- "Agent view, opened with `claude agents`, is one screen for all your background sessions: what's running, what needs your input, and what's done" (CC-AV l.9 ✔).
- "Press `Enter` or `→` on a row to attach … Press `←` on an empty prompt to detach and return to the table" (l.53 ✔).
- "pressing `←` on an empty prompt opens agent view" from a regular session (l.61 ✔).
- Background sessions run under a separate supervisor process (l.158 ✔).
- `claude agents --json` is "the supported way to read session state from outside", with `state`/`status`/`waitingFor` (l.824-834 ✔).
- In-session subagents are a separate surface: "nested subagents as a tree in the subagent panel … with a path back to `main`" (CC-SUB l.1076 ✔).

#### What is common and what differs

| Dimension | omp (Agent Hub) | codex (Agent command center) | Claude Code (agent view + subagent panel) |
|---|---|---|---|
| Unit listed | **Subagents** of this session, live + persisted, nested, flat ↔ tree | **Root sessions** on the daemon. Subagents rolled up; drill-down via `/subagents`, `Alt+←/→` (`cx:tui/src/app/agent_picker.rs:40-60`) | **Top-level background sessions**. Subagents in a separate in-session tree panel |
| Entry | ←← (40–500 ms) on empty editor; `Alt+A`; `Ctrl+S`; `/hub` | `codex agents`; `←` on empty composer; `/agents` | `claude agents`; `←` on empty prompt (also backgrounds the current session) |
| Who holds the sessions | The omp process (process-global registry). JSONL persistence; resumed sessions show children as `parked`; `--restore` restarts crash-interrupted children (documented only: `omp:docs/cli-reference.md:86-88`, [05 §4.2]) | Shared **app-server daemon**. The TUI is a client | **Supervisor** process. Restarts crashed workers and reaps idle ones after ~1 h (CC-AV §The supervisor process) |
| Attach / detach | Focus re-points the main TUI. `Esc`/←← back. Esc never interrupts the child (`input-controller.ts:590-600`) | Attach = read + resume. Detach = `thread/unsubscribe`, work continues (`cx:tui/src/app_server_session.rs:1744`) | Attach shows a recap of what happened while away. `←` detaches. `Space` = **peek** without attaching |
| Steering | Type into the focused child (normal prompt path). IRC `write agent://` delivered as an aside at the next step boundary (`omp:CA/irc/bus.ts:154-196`). RPC `steer_subagent`; hard stop `cancel_subagent` | `turn/steer`; v1 `send_input{interrupt}`: `interrupt:true` "interrupts the current task and handles this message immediately" (`cx:core/src/tools/handlers/multi_agents_spec.rs:181-183`); v2 `send_message` (queue) vs `followup_task` (wake), `interrupt_agent`; `turn/interrupt`; `codex queue` | Peek-reply is queued "instead of interrupting the response" (CC-AV §Peek and reply). `Ctrl+C` while attached "cancels a running response" (CC-AV l.256) |
| Revive / park | idle → `parked` after 420,000 ms (`omp:CA/task/settings.ts:384-386` ✔). Any message/Hub `r` revives from JSONL. Kill writes a `.tombstone` and the agent can still be revived | `NotLoaded` threads resume on attach. v2 residency unloads idle runtimes (`cx:core/src/agent/control/residency.rs:1`). Archive/unarchive | Idle unattached sessions stopped after ~1 h, resumed on attach. Pinned stay alive |
| Approvals | Children run `tools.approvalMode: "yolo"` (`omp:CA/task/executor.ts:1184` ✔). The parent's `task` approval is the gate. A code comment says user `tools.approval` policies still apply; the enforcement path was not traced ([05 §4.2]) | Cross-thread approval overlay with "o to open thread" (`cx:tui/src/bottom_pane/approval_overlay.rs:567-575`). Needs-input bucket rolls up to the root | "Needs input" group. `waitingFor` names the blocker. Teammate permission prompts surface in the lead; teammates "can't approve … on your behalf" (CC-TEAM) |
| Triage order | "Ordered by operational state" (`omp:TUI/overlays/agent-hub.ts:1262-1284`) | Four buckets: NeedsYou (incl. root errors) > Working > Ready > Finished ("Inactive"), stable within bucket (`cx:tui/src/app/agents_overview_view.rs:69-75,227-244`) | Pinned, then needs-you at top (CC-AV l.95 ✔) |
| Destructive verbs | `x` = abort + tombstone (revivable) | Archive/Delete **cascade** to children, with explicit confirmation; delete is irreversible (`cx:tui/src/app/agents_overview_actions.rs:203-237`) | `Ctrl+X` stop, twice = delete. Refuses to drop unpushed commits |
| Headless | RPC verbs | App-server JSON-RPC + CLI `resume/fork/archive/delete/queue` | `--json`; `attach/logs/stop/respawn/rm` |
| Maturity | v18.13.2 source | Shallow clone; feature stage of the dashboard not determined [02 §8.6] | Research preview (CC-AV l.21 ✔) |

**Common core** [INFERENCE from the table]:
- one roster, sorted around the human;
- a one-key, zero-typing path from a conversation to the roster and back, guarded so it only fires on an empty input line;
- sessions that outlive the view (except omp, which persists them and revives later);
- queued steering applied at a turn or tool boundary, *next to* a separate explicit interrupt (codex `send_input{interrupt:true}`, `interrupt_agent`, `turn/interrupt`; Claude Code `Ctrl+C` while attached; omp `cancel_subagent`);
- per-row cost and tokens.

**Key divergence:** what a "row" is.
- codex and Claude Code put *top-level sessions* in the roster and push the *subagent tree* down one level.
- omp's Hub *is* the subagent tree.

This is the main design choice maos must make (agenda Q4).

### D.2 Comparative matrix (capability × system)

Cells are terse. Every cell is backed by the cited source or R1 section. The "maos gap" column is the delta against the best-evidenced practice.

| # | Capability | omp | codex | Claude Code | Other notable | maos today (`M:` @ d9bd59c4) | maos gap |
|---|---|---|---|---|---|---|---|
| 1 | Roster / command center | Agent Hub roster + inspector (`omp:TUI/overlays/agent-hub.ts:2406-2588`) | Agent command center, root tasks `Tasks·Status·Updated` + details (`cx:tui/src/app/agent_center/rows.rs:80-85`) | Agent view grouped by state (CC-AV) | Copilot agents page (GH-SESS); Devin sidebar (DEVIN-ADV); Cursor Agents Window (CUR-WT) | `GET /v1/daemon` → `spirit_ids`, `lifecycle_states` (`crates/maos-control/src/lib.rs:1101-1118` ✔) | **No `maosctl` verb** (`rg 'v1/daemon\|spirit_ids' crates/maos-cli/src` = 0 ✔). No role/parent/activity/cost columns [04 §2.2a] |
| 2 | Entry / navigation | ←←, `Alt+A`, `Ctrl+S` | `←`, `/agents`, `codex agents` | `←`, `claude agents` | GitHub panel on any page | `maos shell` `@<spirit>` addresses only `hello-spirit` (`crates/maos-shell/src/lib.rs:500-537`) | No navigation surface. Any TUI is bound by NFR-Ops-5 |
| 3 | Parent→child tree | `t` tree; dotted ids; child `parentSession` (`omp:docs/session.md:65-101`) | Roots + `/subagents` by `ancestor_thread_id`; `AgentGraphStore` edges Open/Closed (`cx:agent-graph-store/src/store.rs:13-55`) | Tree panel with `(+N)` (CC-SUB l.1076 ✔) | OTel `invoke_agent` nesting (OTEL-AGENT) | SCB has no parent field (`crates/maos-kernel-core/src/scheduler/control_block.rs:258-287` ✔). In-flight ledger has `originator_spirit_id` but is not exposed (`crates/maos-domain/src/ports/task.rs:12-24`) | No parentage on any route. TL lineage broken by pid-0 delegation frames: `deliver_typed(frame, 0, None)` at `crates/maos-bin/src/delegation.rs:452,581,681,750` ✔ (20-3c member 17, `sprint-status.yaml:300`; the tracker's own line cites 273/384/484/553 are stale [05 M5]) |
| 4 | Server-held sessions / detach | In-process; persisted + revived as parked | Daemon; "Any running work continues" ✔ | Supervisor process | Copilot on Actions; Devin VMs | `maos run` daemon holds Spirits; door per ADR-062 | Live view = the run terminal's stdout. No detach/reattach of a view |
| 5 | Attach to a live agent | Focus re-points main TUI | Enter/→ = read + resume | Enter/→ attach; Space peek | Cursor remote-desktop takeover (CUR-CLOUD) | `maosctl spirit inspect` snapshot (`crates/maos-cli/src/subcommands.rs:2536-2567` [05 M2]); post-hoc `audit query` | No follow/tail, no `--pid` filter. Door has no stream (`crates/maos-control/src/lib.rs:233-253,1960`) |
| 6 | Steering | Focused prompt; IRC aside at step boundary; `steer_subagent` | `turn/steer`; queue vs wake (v2) | Peek-reply queued | Copilot "after it finishes its current tool call" (GH-SESS); Amp follow-up = extra instructions (AMP-SUB) | `maosctl orchestrator queue` → `OrchestratorBuffer` (cap 32) + ADL row (`crates/maos-bin/src/operator_door.rs:1211-1261`) | **No consumer**: `rg 'drain_next\|dequeue_at_safe_point' crates/maos-bin/src` = doc comment only ✔ (19-2). Workers have no stdin under T3 (`crates/maos-bin/src/worker_spawn.rs:665-667` ✔) |
| 7 | Lifecycle (park / revive / kill) | idle→parked 7 min; revive; tombstone kill (`omp:CA/registry/agent-lifecycle.ts:42-60,291-397`) | NotLoaded→resume; residency unload; archive/delete cascade | Idle reap ~1 h; resume on attach; stop/delete | Devin stop puts children to sleep (DEVIN-WF) | `start/pause/resume/unload/uninstall/upgrade` per id (`crates/maos-control/src/lib.rs:1430-1525`); `stop` refused (`crates/maos-cli/src/subcommands.rs:67-71`) | Worker pause is a documented **no-op** (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:1138-1151` ✔). No parked/revive state. No tree-wide verb |
| 8 | Needs-input triage | Hub ordered by state; unread IRC badge | Buckets + descendant rollup (`cx:tui/src/app/agents_overview.rs:1232-1243` ✔) | "Needs input" + `waitingFor` | A2A `INPUT_REQUIRED`/`AUTH_REQUIRED`; MCP `input_required`; Jules `AWAITING_PLAN_APPROVAL` | `dispatch_halted` stdout event; `maosctl halt list` reads TL (`crates/maos-cli/src/subcommands.rs:2035-2039`) | No "needs you" state on any read route. No live pending-halt GET |
| 9 | Approvals inbox | Children run yolo ✔ (a comment says user `tools.approval` policies still apply; not traced) | Approval overlay routed to the human's view + jump `o` | Prompts surface in lead; no agent can approve for you | Devin "Auto-approve child sessions" toggle | Three lists: `halt list/resolve`, `skills list/approve/reject`, `a2a repin list/approve/reject` (cohort door only) [04 §2.2g] | No unified inbox. No per-action approval prompt in production: the kernel `ApprovalManager` would auto-allow ("v0.3-β auto-allows", `crates/maos-kernel-core/src/security/approval.rs:43-75` ✔) but is constructed as `let _approval` and never called (`crates/maos-bin/src/main.rs:3386`, review ✔) |
| 10 | Workspace / sandbox isolation | Opt-in (`task.isolation.enabled` default false); patch/branch merge (`omp:CA/task/settings.ts:27-30,148-166`) | `w` new worktree; `SandboxMode` read-only default (`cx:protocol/src/config_types.rs:104-114`) | Background sessions enforced into worktrees; teams **not** isolated (CC-AV, CC-PAR) | Cursor worktrees/best-of-n; Devin/Cursor VMs; ANT-CCC containers | T3 rootless podman, `--network=none`, per-spawn egress proxy, scoped bearer (ADR-061 `:14-37`) | Process isolation is strong. Per-Worker workspace + effect oracle = 19-3 (story drafted 2026-10-28, `ready-for-dev`) |
| 11 | Fan-out limits (code, or documented) | Code: 32 concurrent, depth 2, 200-request soft budget, forced stop at 1.5× (`omp:CA/task/settings.ts:295-334,397-405` ✔) | Code: 6 (v1) / 4 (v2) threads, depth 1 (`cx:core/src/config/mod.rs:260-270` ✔) | Documented only (closed source): 20 concurrent, depth 3; ultracode sessions exempt from the 20-limit; "Large workflow" warning >25 agents / 1.5M tokens (CC-SUB l.1055,1082 ✔; CC-WF) | Devin per-child ACU; Factory autonomy clamp (FAC-SUB) | Topology loop is a sequential `for` (`crates/maos-bin/src/main.rs:4687`); Workers run synchronously under `block_in_place` (`main.rs:5028`). 18-5 context budget refuses tool calls past 100 % | No concurrency/depth/request budget for Orchestrator fan-out found [04 §5.2]. NFR-Perf-8 "50 concurrent Workers" not exercised; its subprocess fan-out bench now sits with `20-7-stable-runner-perf-gates` (19-3 ruling D-19-3-11 C) |
| 12 | Structured results / references | Hidden `yield` + schema; `agent://<id>/<field>`; push notice with `agent://`/`history://` | Closed status enum incl. `{completed: msg}` | Summary back to parent; `maxTurns` marks output partial | Devin `agent(prompt, phase, schema)` | Distillate-fed dispatch (`PriorDistillateRef`, `spirits/orchestrator/src/lib.rs:16-24`); `worker_completion` with `last_stdout_tl_ref` | No closed delegation-status vocabulary exposed to operators [INFERENCE from 04 §2.2] |
| 13 | Transcripts + replay | Per-agent JSONL/MD; `history://`; Activity log scoped all/selected/subtree | Rollout JSONL; fork at turn (`thread/fork`) | Transcripts; `/rewind` checkpoints; workflows' deterministic relaunch (CC-WF) | Devin hash-keyed replay (DEVIN-WF); LangGraph fork-not-rollback (LG-TT) | TL rows per pid; trajectory `export`/`replay`, sealed export, `reconcile-hosts` (`crates/maos-cli/src/cli.rs:515-620,678-711`) | Post-hoc only. No per-agent "session" object (no stable `task_id`) [04 §2.2b] |
| 14 | Cost / tokens | Per-agent `$`, tokens, requests; subtree total; "usage —" when missing (`omp:CA/modes/agent-hub-runtime.ts:57-85`) | Per-task tokens; credits/$ only on some plans, selected row, 60 s (`cx:tui/src/app/agents_overview_usage.rs:1-95`) | Per-agent tokens in `/workflows`; each session draws quota | Devin ACU per child; Copilot session tokens | `cost.attribution` (kind 29) from the Inference Port only; monthly reconcile (`crates/maos-kernel-core/src/inference/mod.rs:414-434`; `crates/maos-cli/src/cli.rs:712-729`) | No task axis (20-14). Worker-CLI spend through egress **not attributed** (rg = 0, [04 §2.2f]). No live gauge |
| 15 | Notifications | In-TUI only: pinned "Subagents · N running", status line, completion self-estimates `{pct,stage,watch}` (`omp:CA/task/completion-probe.ts:1-47`) | OSC 9 / BEL, focus-conditional; external `notify` with `agent-turn-complete` (`cx:config/src/types.rs:723-827`) | Notifications on needs-input/finish/fail; `Notification` hook | Devin coordinator woken on child finish/needs-input | `NotificationDispatcher` terminal channel real (`crates/maos-director-surface/src/lib.rs:3-12`) | `maos-notify-push` (from Story 8.13 AC3, `epic-8…md:453-462`) is an optional maos-bin dependency (`crates/maos-bin/Cargo.toml:23,62`) with **no** `src/` caller [04 §2.1]. J1 halt push unwired |
| 16 | Headless control API | RPC verbs | App-server JSON-RPC | `--json` + shell verbs | A2A; MCP tasks | Door HTTP + bearer; `maosctl` | Missing read verbs (list, tree, tail, inbox) |
| 17 | Inter-agent messaging | IRC bus, mailbox 100, `agent://all` (`omp:CA/irc/bus.ts:30-31`) | v2 mailbox, survives unload (`cx:core/src/agent/control/mailbox.rs:1-2`) | Team inboxes `~/.claude/teams/{team}/inboxes/{agent}.json`; `SendMessage` labelled | Devin coordinator messages children | IAC typed-intent frames, log-before-deliver (I2) | Bus is fine. Workers cannot receive (T3, by design) |

### D.3 Technique catalogue

"Measured outcome" lists only published outcome reports that bear on the technique itself. Where none exists, the cell says "None measured", and qualitative or illustrative evidence is labelled as such. All reports are vendor- or author-reported and were not reproduced here. None is a controlled comparison on coding work [03 §8 Q3].

| ID | Technique | Definition | Who does it (evidence) | Measured outcome | maos reshaping |
|---|---|---|---|---|---|
| T01 | **Roster / command center** | One screen listing every agent with state, activity, age, cost. A row opens the session | omp Hub; codex command center; CC agent view; Copilot; Devin; Cursor (§D.2 row 1) | **None found.** "I found no published evaluation … that measures operator throughput or error rate with roster/peek/attach" [03 §8 Q2]. Design rationale only | §E.2 R1 |
| T02 | **Session tree (parentage as data)** | Parent→child edges stored, not inferred; dotted ids or graph store | omp dotted ids + `parentSession`; codex `AgentGraphStore`; CC tree panel | None | §E.2 R2 |
| T03 | **Descendant status rollup** | A root row shows its most urgent descendant | codex `agents_overview_group` fold with `min()` (`cx:tui/src/app/agents_overview.rs:1232-1243` ✔) | None | §E.2 R1 (client-side) |
| T04 | **Attach/detach with server-held sessions** | A viewer attaches to and detaches from sessions a daemon owns. Detach never stops work. Exit prints a runbook | codex daemon + exit summary (`cx:tui/src/app/exit_summary.rs:95-119`); CC supervisor | None | §E.2 R3 |
| T05 | **Parked / revive lifecycle** | Finished agents stay addressable: idle → parked (memory freed, transcript kept) → revived on message; kill = revivable tombstone | omp (`omp:CA/registry/agent-lifecycle.ts:1-23,42-60`); CC idle reap + resume on attach; codex v2 residency | None. omp prompt advises "ask the same agent a follow-up" as cheaper than respawn (`omp:docs/tools/task.md:197`) — design claim | §E.2 R5 |
| T06 | **Steering at turn/tool boundaries** | Operator input is queued and applied at the next safe boundary. Every surveyed product *also* offers an explicit interrupt, so boundary steering is the default path, not the only one | Copilot (GH-SESS); CC peek-reply; Amp; omp IRC aside; codex `send_message` vs `followup_task` | None measured. Qualitative: a synchronous lead "can't steer subagents" (ANT-MARS §Synchronous execution). Counter-evidence to "never pre-empt": codex `send_input{interrupt:true}` "interrupts the current task and handles this message immediately" (`cx:core/src/tools/handlers/multi_agents_spec.rs:181-183`) and v2 `interrupt_agent`; CC `Ctrl+C` cancels a running response while attached (CC-AV l.256); omp `cancel_subagent` | §E.2 R4 (FR20 safe point + FR51(a) bounded interrupt, both now in 19-2) |
| T07 | **Needs-input-first sorting + typed reason** | A human-blocked state is first-class, sorted first, with a reason (`waitingFor`) | codex buckets; CC `blocked` + `waitingFor` (l.824-834 ✔); A2A/MCP `input_required` | None | §E.2 R6 |
| T08 | **Approvals inbox with jump-to-source** | Approval requests from any agent reach the human's current view, with one key to open the requester | codex overlay `o` (`cx:tui/src/bottom_pane/approval_overlay.rs:567-575`); CC lead receives teammate prompts | Friction is documented: "Teammate permission requests bubble up to the lead, which can create friction" (CC-TEAM) | §E.2 R6 |
| T09 | **Workspace / sandbox isolation** | Each writer gets its own worktree, VM or container | CC enforced worktrees; Cursor; Devin; ANT-CCC; omp opt-in | ANT-CCC (one run, vendor-reported): 16 parallel agents, ~2,000 sessions, ~$20k, 100k-line compiler, coordinated through `current_tasks/` lock files + git; parallelism collapsed on one giant task until a GCC oracle split it. Counter: CC teams un-isolated → "Two teammates editing the same file leads to overwrites" (CC-TEAM) | §E.2 R7 |
| T10 | **Single writer + read-only helpers** | One agent writes; others only contribute "intelligence" | COG-MAW; Factory read-only `explorer`; COG-DBMA | COG-MAW: multi-agent works "when writes stay single-threaded" (quote ✔ re-run). Cursor: flat lock-based coordination cut 20 agents to "the effective throughput of two or three" and made them "risk-averse" (✔ re-run) | §E.2 R8 |
| T11 | **Clean-context reviewer / verifier** | A second agent with no shared context checks the output | Devin Review (COG-MAW); CC `/deep-research` voting (CC-WF); Cursor judge; ANT-CCC test oracle | COG-MAW, a vendor reporting on its own product (no baseline, control or false-positive rate): ~2 bugs per PR, "roughly 58% are severe" (✔ re-run). ANT-CCC (single run): "the task verifier [must be] nearly perfect". MAST: task verification is one of 3 failure categories | §E.2 R9 |
| T12 | **Fan-out limits / depth / budget enforced in code** | Concurrency cap, depth cap, request/turn/token budget, spawn allow-lists rejected by code, with a typed "don't retry" error | omp semaphore, depth, budget (`omp:CA/task/index.ts:661-672`, `executor.ts:1783-1788`); codex; CC (documented: `Concurrent subagent limit reached` ✔, ultracode sessions exempt) | None measured for limits themselves. Qualitative: ANT-MARS's early failure "spawning 50 subagents for simple queries", which it fixed with effort-scaling rules in the prompt, not with code. The value and cost numbers often quoted here (90.2 %, 80 %, 15×) measure multi-agent fan-out, not limits: see §F.1 item 5 and X5 | §E.2 R10 |
| T13 | **Structured results + addressable references** | Children return schema-checked data. Large output is passed by reference (`agent://`, ids), not inline | omp `yield` + `agent://<id>/<field>`, 11,500-char inline cap (docs only, [05 §4.2]; eviction in `omp:CA/async/job-manager.ts:9-43`); codex status enum; Devin `agent(…, schema)` | None measured. LC-MA gives illustrative worked examples, not measurements: for a one-shot request, subagents 4 calls vs handoffs 3; for a multi-domain request, subagents 5 calls / ~9K tokens vs handoffs 7+ / ~14K+ | §E.2 R11 |
| T14 | **Push results, don't poll** | Completion is injected into the owner. `wait` only when fully blocked | omp async-result + "NEVER poll" (prompt only, `omp:CA/prompts/tools/task-async-contract.md:1`); codex notifications; Devin coordinator woken | None | §E.2 R12 (19-6) |
| T15 | **Transcripts + replay / fork** | Every turn persisted. Resume replays recorded calls. Rewind forks rather than rewriting | Devin hash-keyed call cache (DEVIN-WF); CC deterministic workflows; LangGraph "does **not** roll back a thread" (LG-TT) | None | §E.2 R13 |
| T16 | **Cost/token accounting by lineage** | Per-agent direct cost, summed down the tree; honest "unknown" | omp `sumSubagentTreeCost`, "usage —"; codex per-row tokens | ANT-MARS 15× multiplier; ANT-CCC ≈$20k for one project | §E.2 R14 |
| T17 | **Notifications on needs-input / done / failed** | Out-of-band signal when a human is needed | codex OSC 9/BEL + `notify`; CC notifications + `Notification` hook | None | §E.2 R15 |
| T18 | **Cascading lifecycle with explicit consequences** | Archive/delete/stop of a root states, and applies, its effect on children; irreversible verbs need a second confirmation | codex modals (`cx:tui/src/app/agents_overview_actions.rs:203-237`) | None | §E.2 R16 |
| T19 | **Agent message ≠ human authority** | No inter-agent message can satisfy an operator approval | CC-TEAM, CC-SUB "no message from any agent counts as your approval" | None | §E.2 R6 + MW-6 |
| T20 | **Shape rules in code, advice in prompt** | Reject malformed fan-out in code. Keep "verify once"/"don't poll" as prompt advice | omp enforced vs advised split [01 §5.3] | None. Note that omp's "verify once" and "NEVER poll" are **not** enforced | §E.2 R10 |

---

## E. Implementation Approaches and Technology Adoption (step 5)

### E.1 The maos way: design principles derived from maos constraints

Each principle cites the constraint it comes from (`M:` @ d9bd59c4, consolidated in [04 §5.1]).

| ID | Principle | Constraint and evidence | Consequence for orchestration UX |
|---|---|---|---|
| MW-1 | **The kernel owns mechanism, not policy and not UI** | "The kernel does NOT embed an orchestration policy" and does not own "UI presentation" (`M:_bmad-output/planning-artifacts/prd/functional-requirements.md:11,14` ✔). The FR20 buffer's kernel only orders and bounds the queue; the Orchestrator decides *when* (`M:crates/maos-kernel-core/src/orchestrator/mod.rs:8-12` ✔). Architecture assigns the live "what are the agents doing" view to the Observer Spirit (`M:_bmad-output/planning-artifacts/architecture-maos-minimal-opus/6-reference-spirits.md:92` ✔) | Rosters, trees, inboxes and navigation live in `maosctl` or a Spirit. Tree-level verbs ("stop this epic") are Orchestrator policy triggered by one door command |
| MW-2 | **Kernel-Δ 0 by default** | ADR-069: kernel-core ≤ 23,000 tokei with zero headroom; every line needs a FLAG-Winston grant; `21-5-orchestrator-port-and-move` (≈ −1,028) is the relief valve (`M:docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46` ✔) | Prefer projections in `maos-control`/`maos-bin`/`maos-cli`. `operator_door.rs` already holds `scheduler.scbs()` and the SCB ledger field is `pub` (`control_block.rs:278`) [04 §5.3] |
| MW-3 | **One authenticated door; every action is an `OperatorCommand`** | ADR-062: one door, bearer before route, `maosctl` must not link `maos-control` or the kernel; `operation_id` + exactly one completion row before reply (`M:crates/maos-control/src/lib.rs:289-502`) | Dashboard actions are ordinary door commands: auditable, serialized per Spirit. No second port, no side channel |
| MW-4 | **Log before act** | I2 (log before deliver) and I4 (approval captures intent) (`M:docs/invariants/I2.md:13,30-32`; `I4.md:13,30-31`). Re-pin is strictly log-before-act (ADR-062 `:214-216`). **Observed exception:** door lifecycle verbs transition first and journal second; `scheduler_loop.rs:437-455` ignores TL write errors with `let _` [04 §5.1] | New inbox/steer verbs must write their decision row first. The lifecycle-order exception is a known risk to carry into the agenda (Q10) |
| MW-5 | **SCB = live truth; TL = history; no third store** | "never caches or reconstructs" (`M:crates/maos-control/src/lib.rs:5-6,510-512`). FR4: every external call verifiable from the TL | A roster reads the SCB through the door. Transcript/tree/cost read the TL. No dashboard cache (unlike codex SQLite or CC `roster.json`) |
| MW-6 | **Capability tokens and consent are the only authority** | Spirit-bound short-lived tokens (I1/FR4); typed-intent consent envelopes (ADR-012 `:12-24`); production granter not wired yet (21-7). The kernel `ApprovalManager` (auto-allow) is constructed but never called (`M:crates/maos-bin/src/main.rs:3386`) | No "yolo children" (contrast omp's `approvalMode: "yolo"`, `executor.ts:1184` ✔). No auto-approve timers (contrast Jules). An agent message can never satisfy an operator approval. This is the design; it is not yet true in production until 21-7 lands |
| MW-7 | **T3 Workers are sealed** | `reject_t3_stdin_posture` (`M:crates/maos-bin/src/worker_spawn.rs:665-667` ✔); no host home, so no `control.json` (ADR-061 `:21-37`; ADR-062 `:161-167`) | "Steer a Worker" means "queue for the Orchestrator's next dispatch" (FR20). "Interrupt a Worker" means a door command that acts on its in-flight work within a bound (FR51(a), P99 ≤ 2 s; 19-2 AC7, ruled 2026-10-28). Under D-19-2-14 A the door freezes the Worker's container (`podman pause`). If the hosted freeze spike (CI check #15) fails, the C fallback applies: 409 `worker_pause_unsupported`, with FR51(a) labelled in-process-only for Workers, never a stop. Neither path ever writes to Worker stdin |
| MW-8 | **A control is real, or it is labelled** | Confidence rule 11 (`M:_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md:56` ✔) | The UI must render "pause: no-op for this Worker form" and "queued: not consumed (19-2)", not imply control |
| MW-9 | **`None` ≠ empty; fail loud** | Read routes 404 when the object is absent, never a fabricated empty value (`M:crates/maos-control/src/lib.rs:78-83,132-141`) | A roster that cannot reach the door errors. It never shows "no agents" |
| MW-10 | **Accessible, plain-text first** | NFR-Ops-5 (`M:crates/maos-cli/src/cli.rs:18-21`) | Ship `--json` and `--plain` list views before any interactive TUI. Gestures are optional sugar |
| MW-11 | **Epic kloc envelopes are already spent** | E19 envelopes are sized to the 19-2…19-6 forecasts (`M:_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md:36-48` @ d9bd59c4): maos-bin 31,166 = 30,366 measured + 800, raised to **31,353** (+187 = 19-6's upper forecast × 1.3) by the operator's E19 kloc ruling of 2026-10-28 (ask `w2_kloc`), with the aggregate hardfail endpoint 191,061 → **191,248**; maos-cli 6,382 = 6,317 measured + 65 for 19-2; maos-control 1,520 forecast vs 1,539 (19 lines unforecast) | Every NEW candidate below needs its own envelope ruling or a later epic. None fits inside E19 as ratified [INFERENCE] |

### E.2 Re-shaping each borrowed technique to fit maos

Every row is **[INFERENCE] / proposal, not decided** unless the status column names a ratified backlog key.

| Ref | Borrowed technique | maos shape | Layer · kernel-Δ (est.) | Principles honoured | Status |
|---|---|---|---|---|---|
| R1 | T01 Roster + T03 rollup + T07 sorting | `maosctl agents [--json\|--plain]`: one call to `GET /v1/daemon` plus `GET /v1/spirits/{id}` per row. Buckets **Needs you** (pending halt/consent/pick) › **Working** › **Ready/Paused** › **Inactive**. The door has no read route for pending halts (its only halts route is `POST /v1/halts/{id}/resolve`, `M:crates/maos-control/src/lib.rs:1209`), so **Needs you** waits for NEW-3's route. Until then NEW-1 shows lifecycle buckets only, or a TL-derived count labelled with its lag. Rollup computed client-side once parentage exists (R2) | maos-cli only · 0 (Needs-you bucket: after NEW-3) | MW-2, 3, 5, 9, 10 | [INFERENCE]; NEW-1 |
| R2 | T02 Session tree | Parentage = SCB in-flight ledger (`originator_spirit_id`) exposed by a read route such as `GET /v1/spirits/{id}/tasks`, cross-checked against `task.assign` TL rows **after** the pid-0 fix. No parent field added to the SCB | maos-control + operator_door · 0 (ledger is `pub`) | MW-1, 2, 5 | [INFERENCE]; depends on 20-3c member 17; NEW-4 |
| R3 | T04 Attach/detach | The daemon already holds Spirits, so "detach" is free. "Attach" = pid-scoped TL follow (`maosctl agents show <id> --follow`), polling the TL by timestamp cursor. [INFERENCE] No ADR change appears needed, since `maosctl audit query` already reads the TL directly. But this read bypasses the door's bearer: its authority is the `0700` home and `0600` file modes (ADR-062 `:141-160`), and ADR-062 says nothing about reads racing a live writer. A streaming door route needs an ADR-062 amendment (agenda Q2). `maos run` prints a reattach/stop runbook on exit (T04 codex pattern) | maos-cli + maos-audit read · 0 | MW-3, 5 | [INFERENCE]; NEW-2, NEW-6 |
| R4 | T06 Steering at boundaries + explicit interrupt | Operator → `orchestrator queue` → the root drains at the Orchestrator's **safe point** (after a section's `delegation_completed`, before the next section's first frame; never while `in_flight > 0`) and folds the instruction into the next `task.assign`. Plus a recall route (FR51(c)) and option-pick delivery (19-2 AC1–AC4). Kernel-internal dequeue stays forbidden. Ruling D-19-2-01 B absorbed FR51(a)(d) as **19-2 AC7–AC9**; D-19-2-13…19 were ruled A on 2026-10-28 (D-14 with the C fallback). **AC7 — pause:** a capability fence on every mediated action of a paused pid, a Worker container freeze (or the C fallback if CI check #15 fails), and a `dispatch_paused` hold in the `--epic` loop; P99 ≤ 2 s, measured. **AC8 — revoke:** a post-call re-check refuses a response that was on the wire, and a Worker spawned under the revoked token is stopped. **AC9:** rule-11 relabels | maos-bin loop + door routes; kernel-Δ 0 for AC1–AC6. AC7/AC8: ◆ FLAG-Winston kernel grant **ruled D-19-2-20 B**, capped at **+93 physical / +58 tokei**. The prototype measured +74 / +46; the cap adds a 25 % margin, and the landed figure is re-pinned exactly at landing. E19 kernel endpoint ≤ **26,483 / ≤ 20,686**, on a base of 26,390 / 20,628 that already holds 19-3's K2 ceiling | MW-1, 4, 7, 8 | **19-2 `ready-for-dev`, all D-rows ruled 2026-10-28** (`implementation-artifacts/19-2-fr20-enqueue-door-and-safe-point.md`; tracker `sprint-status.yaml:288`) |
| R5 | T05 Parked / revive | Option A: no new state. "Parked" = `Unloaded` with the manifest + TL kept, "revive" = `load` again with a distillate of its TL. Option B: a scheduler-owned `Parked` SCB state (memory released, record kept), with TTL **policy** in the Orchestrator manifest or operator posture | A: maos-bin/Orchestrator · 0. B: kernel scheduler · >0 (needs its own measured FLAG-Winston grant under ADR-069's 23,000 ceiling; 21-5 matters only if the grant would cross the 22,000 alarm) | MW-1, 2 | [INFERENCE]; agenda Q5; NEW-10 |
| R6 | T07/T08 Needs-you inbox + T19 | `maosctl inbox` = read-only aggregation over pending **halts** (needs a live `HaltRegistry` GET; today `halt list` reads the TL), **skills**, **re-pins** (19-7), **option picks** (19-2 AC4). Each row names its existing decision verb. Only operator-principal door commands can resolve; inter-agent frames cannot | maos-control routes + maos-cli · 0 | MW-3, 4, 6 | [INFERENCE]; NEW-3; sources 19-2, 19-4, 19-7 |
| R7 | T09 Isolation | Keep T3 process isolation. Give each Worker its own workspace, mounted read-write at `/workspace` and set as `--workdir`. Snapshot it before the spawn and diff it after the exit: the diff is the effect oracle (an empty diff from a real CLI ⇒ `not_completed:no_effect_evidence`). Workers stay sequential (19-3 AC3) | Kernel **+9 tokei** (FLAG-Winston K2 T3 workspace mount, D19-2 A ratified 2026-10-27; grant +12 physical per `epic-19-founder-loop-w4.md:34`, tracker records +11 physical measured at d9bd59c4) + maos-bin `worker_workspace.rs` | MW-2, 6, 7 | **19-3 `ready-for-dev`** (tracker `sprint-status.yaml:289`; D-19-3-01…10 A; D-19-3-11 C routes AC3's real-spawn bench to `20-7-stable-runner-perf-gates`) |
| R8 | T10 Single writer + read-only helpers | Today each Worker writes only its own workspace and Workers run in sequence (19-3), so a single writer holds by construction. **Precondition for any future concurrent-write proposal** (NEW-8, agenda Q9): if Workers ever share a workspace, grant the `development-task:write-workspace` consent (`DELEGATION_CONSENT_INTENT`, `spirits/orchestrator/src/lib.rs:227`) for that workspace to **one** Worker at a time (a lease with TTL, like short-lived tokens). Reviewers and verifiers run read-only postures (cf. Observer's `passive-observer` posture, `6-reference-spirits.md:90` ✔) | Capability/consent (after 21-7) · 0–small | MW-6 | [INFERENCE]; agenda Q9; not part of 19-3 or 21-7 |
| R9 | T11 Clean-context reviewer | The J1 topology already includes reviewer/architect Spirits [04 §7.3]. Feed the reviewer **distillates only** (FR21 already forbids raw output) and journal its verdict as a TL row the roster can show | spirits + Orchestrator · 0 | MW-1, 5 | [INFERENCE]; NEW-7 (status vocabulary carries the verdict) |
| R10 | T12/T20 Fan-out limits | Orchestrator manifest declares `max_concurrent_workers`, `max_depth`, `request_budget`. **Enforcement** sits at admission (maos-bin) and the PDP/capability layer, refusing with a typed "limit reached, do not retry" error and a TL row. The kernel keeps only its existing context budget (18-5) | maos-bin/PDP · 0 if it stays out of kernel-core | MW-1, 2, 6 | [INFERENCE]; NEW-8; agenda Q8 |
| R11 | T13 Structured results + references | Closed delegation-status vocabulary (`pending \| running \| halted \| completed \| errored \| orphaned \| recalled`) on TL rows. References are TL frame ids (`last_stdout_tl_ref`, `PriorDistillateRef`), the maos analogue of `agent://` | spirit-abi / maos-bin · 0–small | MW-5 | [INFERENCE]; NEW-7 |
| R12 | T14 Push results | Worker completion already reaches the root as events. Worker **crash** must reach the originator's mailbox (`task.orphaned` is journaled but not delivered) | maos-bin · 0 (19-6 under its A rows) | MW-4 | **19-6 `ready-for-dev`** (tracker `sprint-status.yaml:292`; D-19-6-01…11 A; D-19-6-10 superseded by the `w2_kloc` envelope raise, maos-bin 31,353) |
| R13 | T15 Transcripts + replay | TL is the transcript. Add `--pid` to `audit query` and a per-agent view. "Rewind" = fork that references an old frame, never a rewrite (LG-TT model fits append-only TL) | maos-cli · 0 | MW-5 | [INFERENCE]; NEW-2 |
| R14 | T16 Cost by lineage | Per-pid totals from `cost.attribution` + `context.*` rows, summed down R2's tree. Worker-CLI spend labelled **"not attributed"** until egress reports usage | maos-cli/audit · 0; egress (ADR-061) later | MW-8 | **Backlog 20-14** (`sprint-status.yaml:311`) + NEW-5 |
| R15 | T17 Notifications | Wire the existing `maos-notify-push` adapter (no caller today) and the terminal bell for Needs-you transitions | maos-bin · 0 | MW-8 | [INFERENCE]; NEW-6 |
| R16 | T18 Cascading consequences | `unload orchestrator` mid-epic already stops dispatch (`orchestrator_root.rs:196-200`). The door reply and `maosctl` confirmation should *say so*, and list the Workers affected | maos-cli copy + door reply · 0 | MW-8 | [INFERENCE]; part of NEW-1/NEW-9 |
| R17 | T01 interactive view + ←/→ navigation | An interactive "Spirit command center" (←/→ spatial keys, guarded to empty input, `--plain` fallback) **either** in `maosctl` **or** rendered by the Observer Spirit | maos-cli or spirits/observer · 0 | MW-1, 10 | [INFERENCE]; NEW-9; agenda Q1, Q12 |

### E.3 Risks and anti-patterns to avoid

| # | Anti-pattern | Evidence it happens | maos exposure today | Guard |
|---|---|---|---|---|
| X1 | **Auto-approve defaults / fail-open gates** | Jules auto-approves plans on a UI timer (JULES-PLAN). Devin starts children "without asking first" (DEVIN-ADV). CC teams launch "without confirming" (CC-TEAM). omp children run `yolo` (`omp:CA/task/executor.ts:1184` ✔; a comment says user `tools.approval` policies still apply). CC hooks that crash let actions through unless `onFailure: "block"` (CC-HOOK) | No per-action approval path in production: the kernel `ApprovalManager`, which would auto-allow (`approval.rs:43-75` ✔), is constructed as `let _approval` and never called (`crates/maos-bin/src/main.rs:3386`). Wiring it as-is would import a fail-open gate. Production consent granter unwired (21-7) | MW-6. Fail closed by posture. No timers. Never wire `ApprovalManager::prompt` with its auto-allow. 21-7 before any multi-Worker write fan-out |
| X2 | **False success signals** | omp docs/code drift (stale "hops to parent" text). omp prompt rules not enforced [01 §5.3–5.4] | FR20 `queue` returns `completed` and writes an approval row, but nothing consumes it (rg ✔; [04 §5.2]). `maosctl --help` says queued instructions are consumed "via `OrchestratorBuffer::dequeue_at_safe_point`" (`crates/maos-cli/src/cli.rs:82-88`) | MW-8 labels until 19-2 lands; 19-2 AC5 rewrites this help text. Proven-red tests that a "queued" item is eventually consumed (19-2 AC1) |
| X3 | **Coordination overhead / flat swarms** | Cursor: 20 agents → throughput of 2–3, "risk-averse" ✔. ANT-MARS: "spawning 50 subagents for simple queries", "distracting each other with excessive updates", and "most coding tasks involve fewer truly parallelizable tasks than research" (✔ review re-fetch). MAST: 14 failure modes incl. inter-agent misalignment; gains "often minimal". COG-MAW: unstructured swarms "mostly a distraction" ✔ | Today sequential (19-3 AC3 keeps it so), so low exposure. Rises with NFR-Perf-8 concurrency | Orchestrator–workers + judge (T05 in [03]). Code-enforced fan-out caps (R10). Push, not chatter. Agenda Q13 asks whether concurrency is wanted at all |
| X4 | **Unauthenticated saved state** | `RunState.from_json()` unauthenticated (OAI-HITL). MAF pickle warning (MAF-CKPT) | Low: SCB in memory, TL signed exports, bearer-gated door. Risk returns if a dashboard cache or replay cache is added | MW-5 (no third store). Any resume artefact is a TL frame, signed in export |
| X5 | **Cost blow-up** | ≈15× chat tokens for multi-agent (ANT-MARS ✔). ≈$20k for one project (ANT-CCC). Agent-team cost scales per teammate (CC-TEAM) | Worker-CLI spend unmeasured (egress rg = 0) | R14 honest labels. 20-14. Budget caps in R10. A "Large fan-out" warning like CC-WF's (>25 agents / 1.5M tokens) |
| X6 | **Context loss between agents** | "Share full agent traces, not just individual messages" (COG-DBMA ✔ string in [03 §7.2]). Managers "assume they share state with their children when they don't" (COG-MAW). omp children get only `context` + task (`omp:docs/tools/task.md:201`) | By design, Workers get distillates, not raw traces (FR21). A deliberate trade-off | Keep FR21 for *safety*, but expose the full TL trace to verifiers and the operator. Discussion Q9 |
| X7 | **Dashboard as a third source of truth** | codex SQLite + JSONL; CC `roster.json`; omp in-memory registry | Not present | MW-5. Projections only |
| X8 | **Unlabelled no-op controls** | In maos itself: `maosctl --help` says `Pause` "interrupts in-flight autonomous actions … P99 ≤2s interruption (FR51 a)" and `RevokeToken` makes in-flight operations "fail-safe with bounded time (FR51 d)" (`crates/maos-cli/src/cli.rs:90-100`, review ✔); neither mechanism exists at d9bd59c4 | Worker pause is a no-op on Signals/NamedPipe channels (`runtime.rs:1138-1151` ✔), while the help text claims a bounded interrupt | MW-8. Render the label in every view. 19-2 AC5 relabels the help text; FR51(a)(d) is now 19-2 scope (D-19-2-01 B) |
| X9 | **Destructive cascades without confirmation** | codex states cascades and requires a second confirm for delete (`cx:tui/src/app/agents_overview_actions.rs:203-237`) | `unload orchestrator` stops dispatch. The run's stdout prints `dispatch_halted` (`orchestrator_root.rs:196-200`), but the door reply and `maosctl` say nothing about the Workers affected | R16 |
| X10 | **Roster clutter at scale** | omp's agent-facing coop roster, in this synthesis agent's own system prompt, listed "331 parked peer(s) omitted" (the review agent's prompt later showed 336; also [01 §5.2]). This is a prompt roster for agents, not the Hub UI | Not yet | Default filter to live + needs-you. `--all` opt-in (CC `--all`) |
| X11 | **Prompt-only orchestration rules** | omp "verify once", "NEVER poll", delegation gates are prompt-only [01 §5.3] | Orchestrator is deterministic; inference only for `judge_ambiguity` (`spirits/orchestrator/src/lib.rs:29-35`) | Keep decomposition deterministic. Shape limits in code (R10) |
| X12 | **Kloc creep from UX** | — | E19 envelopes are fully forecast (MW-11). Epic-18 retro measured maos-bin +1,693 vs +250 planned (`sprint-status.yaml` epic-18-retrospective row) | One small verb per story. Kernel-Δ 0 target. Ask for envelope rulings up front |

### E.4 Technical Research Recommendations: roadmap

All **NEW** rows are **proposals only**. No NEW story was created, and placement needs operator envelope rulings (MW-11). Existing keys are cited from `M:_bmad-output/implementation-artifacts/sprint-status.yaml` @ d9bd59c4 (✔ re-run, lines 287–322).

**Status update (review 2026-10-28, refreshed after the 19-2 rulings).** 19-2, 19-3 and 19-6 are story files with `Status: ready-for-dev`, created 2026-10-28 at `d9bd59c4`, and every decision row is ruled:
- **19-2:** D-19-2-01 **B** absorbed FR51(a)(d) as AC7–AC9. D-19-2-02 B, 03…11 A and 13…19 A (D-14 with the C fallback if CI check #15 fails). D-19-2-12 is superseded by `w2_kloc`. D-19-2-20 **B**: the ◆ FLAG-Winston kernel grant is capped at +93 physical / +58 tokei and re-pinned at landing, giving an E19 kernel endpoint ≤ 26,483 / ≤ 20,686.
- **19-3:** D-19-3-01…10 A. D-19-3-11 **C** moves AC3's real-spawn bench to `20-7-stable-runner-perf-gates`.
- **19-6:** D-19-6-01…11 A, with D-19-6-10 superseded by the E19 kloc ruling `w2_kloc` (maos-bin 31,166 → 31,353; aggregate hardfail 191,061 → 191,248).

Tracker: 19-2 (`:288`), 19-3 (`:289`) and 19-6 (`:292`) all read `ready-for-dev`.

Ordering rule: value to the operator's two observed needs (see and enter agents; manage them), divided by cost (layer, kernel-Δ, kloc). Dependencies win ties.

| Rank | Item | Technique(s) | Value | Cost | Acceptance-signal ideas | Motivating evidence |
|---|---|---|---|---|---|---|
| 1 | **NEW-1 `maosctl agents`** (list; `--json`, `--plain`) | T01 (T07 after NEW-3) | High: closes "no list verb" | maos-cli only, kernel-Δ 0, small. Needs a maos-cli envelope ruling (E19 has 65 lines, all forecast for 19-2). The Needs-you bucket needs NEW-3's pending-halt route (no GET exists, `crates/maos-control/src/lib.rs:1209`) | Lists every SCB id + lifecycle from `/v1/daemon`. Door unreachable → typed error, never an empty list (MW-9, proven-red: stop the daemon, expect non-zero exit). `--plain` emits 0 ANSI bytes. `--json` field set is stable and documented | §D.2 row 1; CC `--json` (CC-AV l.829 ✔) |
| 2 | **19-2** `fr20-enqueue-door-and-safe-point` (story `ready-for-dev`, tracker `:288`; all D-rows ruled 2026-10-28; D-19-2-01 B absorbed FR51(a)(d) as AC7–AC9) | T06 | High: steering, interrupt and revoke all become real; removes X2's and X8's help-text claims | maos-bin, door, maos-cli, maos-shell; kernel-Δ 0 for AC1–AC6. AC7/AC8: ◆ FLAG-Winston kernel grant **≤ +93 physical / ≤ +58 tokei** (D-19-2-20 B; prototype +74 / +46; re-pinned exactly at landing; E19 kernel endpoint ≤ 26,483 / ≤ 20,686). maos-bin stays inside 31,353 and the aggregate inside 191,248 | As written in the story: AC1 (queue mid-delegation; `pending ≥ 1` while in flight; one instruction drained after `delegation_completed` into the next `task.assign`), AC3 (recall route, journalled), AC4 (option pick), AC5 (help text tells the truth), AC6 (CI runs the vectors), AC7 (pause fence, Worker freeze or the D-14 C fallback, dispatch hold; P99 ≤ 2 s measured), AC8 (revoke: post-call re-check, Worker stop), AC9 (rule-11 relabels). This report adds no AC | [04 §3 Flow C]; GH-SESS / CC-AV boundary steering; codex / CC interrupt (T06) |
| 3 | **NEW-2 per-agent session view** (`maosctl agents show <id>`; `audit query --pid`; `--follow` by TL timestamp cursor) | T04, T15 | High: "enter a session" in maos terms | maos-cli + maos-audit read, kernel-Δ 0. No door change | Shows `task.assign`, `cli.subprocess.output`, `distillate`, `epistemic.halt`, `context.*`, `cost.attribution` for one pid in order. `--follow` prints new rows within one poll interval. No payload leak beyond today's FR4 rules | [04 §6.3]; omp Activity scope (`omp:docs/agent-hub.md:61-69`) |
| 4 | **NEW-3 needs-you inbox** (`GET /v1/halts` live + `maosctl inbox`) | T07, T08, T19 | High: one place for halts/skills/re-pins/picks | maos-control route + maos-cli, kernel-Δ 0. maos-control envelope ruling | Each pending item appears once with its decision verb. Resolving through the named verb removes it. An IAC frame cannot resolve it (negative test). Read is from live registries, not TL lag | [04 §2.2g]; codex `o` jump; CC `waitingFor` |
| 5 | **20-3c** member 17: pid-0 attribution (backlog, `:300`) | T02 prerequisite | Medium (enabler) | kernel-Δ 0 (tracker) | Delegation frames carry the admitted emitter's pid instead of `deliver_typed(frame, 0, None)` (`M:crates/maos-bin/src/delegation.rs:452,581,681,750` ✔). Lineage rebuilt from the TL matches the run's `delegation_routed` stdout | `sprint-status.yaml:300`; [05 M5] |
| 6 | **NEW-4 parentage route + tree** (`GET /v1/spirits/{id}/tasks`; `maosctl agents --tree`) | T02, T03 | Medium-high: the omp/CC tree | maos-control + operator_door, kernel-Δ 0 (`pub` ledger) | Tree shows Orchestrator → Workers with in-flight assignment ids. Rollup puts a Worker's pending halt on the Orchestrator row. Cross-host children shown as "on host B (frame_id …)" via the `reconcile-hosts` join | [04 §6.2, §6.12]; codex rollup ✔ |
| 7 | **19-6** crash → originator delivery (story `ready-for-dev`, tracker `:292`; D-19-6-01…11 A, D-19-6-10 superseded by `w2_kloc`) | T14 | Medium-high: the "Error" truth a roster would show | maos-bin, kernel-Δ 0; maos-bin envelope 31,353 | As written in the story: a killed Worker's `task.orphaned` reaches the Orchestrator as one kernel-sent frame within 5 s, stdout prints `delegation_orphaned`, and the declared disposition (nack / reassign / escalate) is applied (AC1–AC5). Rendering it on a roster row belongs to NEW-1 / NEW-7 | `sprint-status.yaml:292`; story file |
| 8 | **19-4** AC2/AC5 (backlog, `:290`) | T07 | Medium: halts are actually *produced* and resumable | per epic | `judge_ambiguity` has a production caller. A resolved halt resumes dispatch in the same run | [04 §5.2] (`judge_ambiguity` in `crates/maos-bin/src`: doc comment only ✔; its one other caller is the eval harness `crates/maos-bench/src/halt_eval.rs:594`) |
| 9 | **20-14** complete cost attribution (backlog, `:311`) + **NEW-5** honest per-agent cost column | T16 | Medium | per 20-14. NEW-5 maos-cli only | Per-pid token totals. Worker-CLI rows render "not attributed" (proven: no egress usage parser → label present). Task axis after 20-14 | [04 §2.2f]; omp "usage —" |
| 10 | **NEW-6 detach runbook + needs-you notifications** | T04, T17 | Medium | maos-bin, kernel-Δ 0 | `maos run` exit prints door address, reattach (`maosctl agents show …`), stop commands. A Needs-you transition calls the push adapter or a terminal bell; disabled → nothing sent | codex exit summary ✔; `maos-notify-push` (Story 8.13 AC3, `epic-8…md:453-462`) is an optional maos-bin dependency with no `src/` caller |
| 11 | **NEW-7 closed delegation-status vocabulary** | T11, T13 | Medium: one vocabulary for TL, door, UIs | spirit-abi/maos-bin, kernel-Δ 0–small | Every delegation ends in exactly one terminal status row. Reviewer verdict is a status-bearing row | codex status enum; A2A states |
| 12 | **NEW-8 Orchestrator fan-out budget** | T12, T20 | Medium now, high once Workers run concurrently (NFR-Perf-8) | manifest + admission/PDP. Kernel-Δ to be measured. NFR-Perf-8's subprocess fan-out bench belongs to `20-7-stable-runner-perf-gates` (19-3 D-19-3-11 C), so NEW-8 must not duplicate it | Spawn beyond the cap refused with a typed do-not-retry error + TL row. Depth beyond the cap refused. If Workers may share a workspace, the single-writer lease (R8) is part of this story | omp/codex caps (source ✔); CC caps (documented); ANT-MARS 50-subagent failure |
| 13 | **NEW-9 interactive Spirit command center** (←/→, peek, attach view) | T01, T04, T18 | Medium (ergonomics) | maos-cli **or** Observer Spirit | Buckets match NEW-1 `--json` exactly. Every action is an existing door verb. `--plain` parity. Cascade copy shown | §D.1; architecture Observer intent ✔ |
| 14 | **19-3** per-Worker workspace + effect oracle (story `ready-for-dev`, tracker `:289`; D-19-3-11 C routes the real-spawn bench to 20-7); **21-7** production consent granter (backlog, `:322`) | T09 (T10 precondition) | High for safety before parallel writers | 19-3: kernel +9 tokei (ratified) + maos-bin; 21-7: maos-bin | As written in the stories: 19-3 AC2 (each Worker its own `/workspace`; an empty diff from a real CLI ⇒ `not_completed:no_effect_evidence`) and AC3 (sequence; three distinct workspaces). The single-writer lease (R8) is **not** an acceptance signal for 19-3 or 21-7. It belongs to whichever story first lets Workers share a workspace concurrently (NEW-8 / Q9) | COG-MAW; CUR-SCALE; CC-TEAM overwrite |
| 15 | **21-5** orchestrator port and move (backlog, `:320`) | enabler | Relief valve, not a gate. Kernel-Δ>0 options need their own measured FLAG-Winston grant under ADR-069's 23,000 ceiling, and 21-5's promotion is decided when a grant crosses the 22,000 alarm. The kernel is at ≤ 20,686 tokei once 19-3 and 19-2 (D-19-2-20 B cap) land, so about 1,314 tokei remain below the alarm. That gap is not a budget (ADR-069 Decision 2) | ≈ −1,028 kernel lines (ADR-069 ✔) | Only if a grant (e.g. NEW-10) would cross the alarm | ADR-069 `:35-46` |
| 16 | **NEW-10 `Parked` scheduler state** (only if Option B wins Q5) | T05 | Low-medium | kernel, FLAG-Winston | Parked SCB frees runtime memory and keeps the record; any door message revives; revive reuses the same id | omp lifecycle |
| 17 | **NEW-11 → extension, not a new story: OTel GenAI agent-span mapping** | — (tracing; no T-row) | Low now (spec in Development) | `crates/maos-telemetry` (`otel_sink.rs` exists @ d9bd59c4) under the existing NFR-Obs-2 / NFR-Aud-11 OTel requirement (`requirements-inventory.md:157,188`) | TL → `invoke_agent`/`execute_tool` spans with `gen_ai.conversation.id`, pinned semconv SHA | OTEL-AGENT; [03 L12] |

**Success metrics** [INFERENCE]:
1. **Needs-you-to-action latency.** The TL timestamp gap between a halt, consent or pick becoming pending and the operator's resolving door command, compared before and after NEW-1/NEW-3 on the same J1 fixture.
2. **Controls that claim a missing mechanism.** The count of operator-visible controls (`maosctl --help`, door replies, any UI) that claim a mechanism with no production caller. Today this includes at least `Pause` (FR51 a), `RevokeToken` (FR51 d) and `orchestrator` consumption (`crates/maos-cli/src/cli.rs:82-100`). It should fall to zero as 19-2 (including FR51(a)(d)), 19-6 and 21-7 land.

No published study measures roster UX (T01 has no measured outcome), so maos would have to measure this itself (agenda Q11).

---

## F. Research Synthesis (step 6)

### F.1 Executive Summary

1. **Both operator observations are accurate, with one correction each.** omp's ←← (empty editor, 40–500 ms) opens the Agent Hub from *main* and, from a child, returns to *main*, not to the parent (`omp:CA/modes/controllers/input-controller.ts:271-272,765-781,817-821` ✔). `codex agents` opens the "Agent command center" over a shared daemon (`cx:cli/src/main.rs:147-148`, `cx:tui/src/app/agent_center/render.rs:150-152` ✔).
2. **The scope differs.** omp's Hub lists *subagents*. codex lists *root sessions* and rolls subagent state up (`cx:tui/src/app/agents_overview_discovery.rs:77-84`, `agents_overview.rs:1232-1243` ✔). Claude Code splits the two: agent view for sessions, a tree panel for subagents (CC-AV ✔, CC-SUB l.1076 ✔).
3. **Three products ship a similar pattern** (which came first, and whether they are independent, is not established; Claude Code is documented-only):
   - a roster sorted by "needs you";
   - a single-key path from an empty prompt (← or ←←);
   - attach/detach to sessions the server holds;
   - queued steering at turn or tool boundaries, *next to* an explicit interrupt (codex `send_input{interrupt:true}`, Claude Code `Ctrl+C`, omp `cancel_subagent`);
   - fan-out limits: source-enforced in omp (32/depth 2) and codex (6|4/depth 1) ✔; documented only for Claude Code (20/depth 3, ultracode sessions exempt).
4. **The roster pattern has no published measurement.** The evidence for it is design rationale [03 §8 Q2]. maos would have to measure it itself.
5. **The multi-agent evidence is conditional, and weakest for coding.**
   - +90.2 % on Anthropic's internal *research* eval (Opus 4 lead + Sonnet 4 subagents), at ≈15× chat tokens (ANT-MARS ✔). The same article says "most coding tasks involve fewer truly parallelizable tasks than research" and calls such domains "not a good fit for multi-agent systems today" (✔ review re-fetch). maos J1 is a coding loop.
   - Works "when writes stay single-threaded"; unstructured swarms are "mostly a distraction" (COG-MAW ✔).
   - Flat lock-based coordination collapsed 20 agents to the throughput of 2–3 (CUR-SCALE ✔).
   - 14 failure modes across 7 frameworks, and "performance gains on popular benchmarks are often minimal" (MAST ✔).
   - All of these are vendor or author reports, not reproduced here. None is a controlled comparison on coding work [03 §8 Q3].
6. **The add-on with the most consistent support is a clean-context reviewer**: ~2 bugs/PR, ~58 % severe (COG-MAW ✔). These are Cognition's figures for its own product, with no baseline or false-positive rate.
7. **maos has the mechanism but no operator surface.** It has a daemon, one authenticated door, the SCB as live truth, the TL as history, and an FR20 safe-point buffer. But there is no `maosctl` list verb, no parentage on any route, no attach/follow, and no unified approvals inbox [04 §2.2].
8. **maos steering currently signals success without effect.** `orchestrator queue` succeeds and journals, but nothing dequeues it (rg ✔). `maosctl --help` also advertises an FR51(a) P99 ≤ 2 s interrupt and an FR51(d) bounded fail-safe that do not exist (`crates/maos-cli/src/cli.rs:82-100`). 19-2 (story drafted 2026-10-28, `ready-for-dev`, now absorbing FR51(a)(d)) is the fix. This is the most urgent honesty fix.
9. **The maos way:**
   - kernel mechanism only;
   - kernel-Δ 0 by default under ADR-069's zero headroom;
   - every action a door `OperatorCommand`;
   - log before act;
   - SCB/TL as the only truths (no dashboard cache);
   - capability and consent as the only authority (no yolo children, no auto-approve). This is the design; production waits on 21-7;
   - T3 Workers steered via the Orchestrator and interrupted via door commands, never through stdin;
   - "a control is real, or it is labelled".
10. **Re-shaped techniques:**
    - roster = `maosctl agents` over `/v1/daemon`;
    - tree = SCB in-flight ledger via a read route (after the pid-0 fix, 20-3c);
    - attach = pid-scoped TL follow;
    - steering = 19-2 safe-point queue, plus the FR51(a) bounded interrupt that 19-2 now absorbs;
    - inbox = halts + skills + re-pins + picks, each with its existing verb.
11. **Proposed order:**
    1. NEW-1 `maosctl agents`
    2. 19-2
    3. NEW-2 session view
    4. NEW-3 inbox
    5. 20-3c → NEW-4 tree
    6. 19-6
    7. 19-4
    8. 20-14 + NEW-5
    9. then notifications, status vocabulary, fan-out budget, interactive view
12. **Placement is blocked on kloc envelope rulings.** E19's envelopes are fully forecast to 19-2…19-6 (`epic-19-founder-loop-w4.md:36-48`; maos-bin raised to 31,353 by the operator's `w2_kloc` ruling, 2026-10-28), so every NEW candidate needs an operator ruling or a later epic.
13. **Anti-patterns maos must not import:**
    - auto-approve timers and yolo children;
    - a third state store;
    - unlabelled no-op controls (Worker pause; the FR51(a)(d) claims in `maosctl --help`);
    - destructive cascades without confirmation;
    - unmeasured Worker-CLI spend shown as if complete.
14. **Thirteen open questions are queued for a party-mode roundtable (§F.3).** The two largest: where the agents view lives (maosctl vs Observer Spirit), and whether a row is a top-level session or a tree node. Q13 asks the prior question: does maos need a command center or concurrent fan-out now at all?
15. **Replication:** every load-bearing claim has a grep/sed command against a pinned SHA or a live URL (§F.4). No UX flow was runtime-verified.

### F.2 Answers to the research questions

| RQ | Answer | Where |
|---|---|---|
| RQ1 | Yes, both accurate. omp ←← returns to main, not parent. codex lists roots and rolls subagents up | §D.1 |
| RQ2 | Common: roster sorted by human need, single-key entry, server-held sessions, boundary steering next to an explicit interrupt, fan-out limits (Claude Code's documented only). Differ: row unit, who holds sessions, peek, approvals routing, cascade semantics | §D.1 table |
| RQ3 | 20 techniques catalogued. Published outcome reports (all vendor- or author-reported, none controlled on coding work) exist only for multi-agent value/cost, single-writer, clean-context review, coordination overhead and failure taxonomy. None exist for roster UX. LC-MA's figures are illustrative, not measured | §D.3 |
| RQ4 | Mechanism present, surface absent. 17-row gap column | §D.2 |
| RQ5 | 11 principles, 17 re-shapings. Kernel-Δ 0 except: 19-3's ratified T3 workspace mount (+9 tokei); 19-2's FR51(a)(d) fence, freeze and re-check (ruled D-19-2-20 B: ≤ +58 tokei / ≤ +93 physical, re-pinned at landing); and the optional `Parked` state | §E.1–§E.2 |
| RQ6 | 19-2, 19-3, 19-6 (drafted 2026-10-28, `ready-for-dev`), 19-4, 20-3c, 20-7, 20-14, 21-5, 21-7 + NEW-1…NEW-10 (NEW-11 re-labelled an extension of the NFR-Obs-2 owner) | §E.4 |
| RQ7 | 12 anti-patterns with evidence and guards | §E.3 |
| RQ8 | 13 questions with sides and owners | §F.3 |

### F.2.1 Table of contents (template step-06 structure → where covered)

| Template section | Covered in |
|---|---|
| 1. Introduction and methodology | Research Overview; §A; §F.6 |
| 2. Technical landscape and architecture | §C; §D.1; §D.2 |
| 3. Implementation approaches and best practices | §D.3; §E.2 |
| 4. Technology stack evolution and trends | §B.1 |
| 5. Integration and interoperability | §B.2 |
| 6. Performance and scalability | §D.2 rows 11, 14; §D.3 T10, T12; §E.3 X3, X5 |
| 7. Security and compliance | §B.2 security; MW-3, MW-6, MW-7; §E.3 X1, X4 |
| 8. Strategic recommendations | §E.1; §E.2 |
| 9. Roadmap and risk | §E.3; §E.4 |
| 10. Future outlook | §B.1 trends; §F.3 |
| 11. Methodology and source verification | §F.4; §F.6 |
| 12. Appendices | §F.4; §F.5 |

### F.3 Discussion agenda for agents (input for `bmad-party-mode`)

Roles: **Winston** (architect), **John** (PM), **Sally** (UX), **Amelia** (dev), **Murat** (test architect), **Mary** (analyst). Each question lists the competing positions and the evidence each side would cite. The facilitator should require each speaker to cite a line from this report or an R1 note.

**Q1. Where does the agents view live: `maosctl`, the Observer Spirit, or the ACP editor surface?** *Lead: Winston; with John, Sally.*
- **A (maosctl):** a client of the door, accessible (`--plain`), testable. codex and CC both ship it as a CLI subcommand (§D.1). MW-3, MW-10. Cost lands in maos-cli kloc (MW-11).
- **B (Observer Spirit):** the architecture already assigns "a live 'what are the agents doing' view" to the Observer (`6-reference-spirits.md:92` ✔). It is user-space by MW-1 and posture `passive-observer` (`:90`). Cost: a Spirit needs a render surface, and the ACP root is a separate one-shot that may not see the daemon's SCBs [04 §8 Q1].
- **C (both, layered):** the list and JSON verbs in maosctl (NEW-1), and the interactive render later in whichever surface the operator uses.

**Q2. Live follow: client-side TL polling or an ADR-062 streaming route?** *Lead: Winston; with Amelia, Murat.*
- **A (poll):** no ADR change. The door is request/response with `Connection: close` and a 5 s read deadline (`crates/maos-control/src/lib.rs:233-253,1960`). codex reseeds on lag anyway (P8).
- **B (stream):** codex and omp push notifications (`cx:tui/src/app/agents_overview_threads.rs:81-140`; omp RPC subscription). Polling SQLite may race the TL writer. Needs an ADR-062 amendment and auth-before-route for long-lived connections.
- **Murat's test question:** which choice gives a deterministic proven-red?

**Q3. Who owns "parent"?** *Lead: Winston; with Murat, Amelia.*
- **A (SCB ledger):** live, `pub`, kernel-Δ 0 (`control_block.rs:278` ✔; `ports/task.rs:12-24`). It only covers *in-flight* work.
- **B (TL `task.assign` rows):** historical, survives restarts, but wrong until 20-3c fixes pid 0 (`sprint-status.yaml:300`; source `crates/maos-bin/src/delegation.rs:452,581,681,750`).
- **C (Orchestrator exports a lineage frame):** policy-owned (MW-1), like codex's `AgentGraphStore` (`cx:agent-graph-store/src/store.rs:13-55`). Adds a frame kind.

**Q4. What is a roster row: top-level session or tree node?** *Lead: Sally; with John.*
- **A (roots + rollup, the codex/CC shape):** stays small with many Workers; "Needs you" surfaces through rollup (`agents_overview.rs:1232-1243` ✔).
- **B (all agents, flat ↔ tree, the omp shape):** maos runs few Spirits per daemon today (sequential loop), so a full tree is readable. The operator's own observation was of this shape. That is a reason to test it, not evidence that it works better.
- **Evidence gap:** no study measures either (T01). Mary to propose a measurement.

**Q5. Is "parked" a new scheduler state or a reuse of `Unloaded` + reload?** *Lead: Winston; with Amelia.*
- **A (reuse):** kernel-Δ 0. A distillate + TL makes a revive meaningful. MW-2.
- **B (new state):** omp's parked/revive is what makes follow-ups cheap (`omp:docs/tools/task.md:197`, a design claim). But it costs kernel lines, and each needs its own measured FLAG-Winston grant under ADR-069 (✔). 21-5 matters only if the grant would cross the 22,000 alarm.

**Q6. How may an operator act on a running T3 Worker?** *Lead: Sally; with Winston, John.*
- **Context:** ruling D-19-2-01 B (2026-10-28) put FR51(a), a bounded interrupt of an in-flight action (P99 ≤ 2 s), into 19-2 as AC7. Its kernel grant is ruled D-19-2-20 B (≤ +93 physical / ≤ +58 tokei). Every surveyed product offers both a queued message and a hard interrupt (T06).
- **A (Orchestrator-mediated messages + door-level interrupt only):** ADR-061 refuses stdin postures (`worker_spawn.rs:665-667` ✔). Messages go through FR20's safe point. An interrupt is a door command that stops the Worker within a bound and journals first (MW-4).
- **B (eventually, a mediated message channel into the Worker):** every surveyed product lets the human talk to a running agent (§D.2 row 6), and the operator praised *entering* sessions. ADR-061 says stdin is refused "until the first production stdin writer" [04 §8 Q4]. This costs a new posture and a T3 threat-model change.
- **Ruled for now:** D-19-2-14 A freezes a T3 Worker's container (`podman pause`, state kept, FR51(b)). If the hosted freeze spike (CI check #15) fails, the C fallback applies: 409 `worker_pause_unsupported`, FR51(a) labelled in-process-only for Workers, never a stop. **Still open for the roundtable:** whether B (a mediated message channel) is ever wanted, and what replaces the freeze if the C fallback fires (kill-and-re-dispatch through 19-6's reassign path, or a signal the CLI honours).

**Q7. One inbox or per-kind lists? Live registries or the TL?** *Lead: John; with Sally, Murat.*
- **A (one inbox, live):** codex/CC route every approval to one place (T08). The J1 journey wants halt notifications with candidate answers (`prd/user-journeys.md:126-161`). Needs new GET routes.
- **B (keep per-kind verbs, add only a count):** each kind has different semantics (nonce echo for re-pin, three resolution kinds for halts). Smaller.
- **Murat:** an inbox is a *control* under rule 11, so what is the proven-red for "item resolvable only by its operator verb"?

**Q8. Where are fan-out limits enforced, and with what defaults?** *Lead: Winston; with Murat.*
- **A (Orchestrator manifest + admission/PDP):** policy stays in user space (MW-1). Enforcement is real in omp and codex (source ✔); Claude Code documents its caps (closed source; ultracode sessions exempt).
- **B (kernel scheduler admission):** uniform across all Orchestrators, but costs kernel lines.
- **Defaults to argue:** omp 32/2, codex 6|4/1, CC 20/3 (documented). NFR-Perf-8 targets 50 concurrent Workers; its subprocess fan-out bench now belongs to 20-7 (19-3 D-19-3-11 C).

**Q9. Parallel writers or single writer for maos coding work?** *Lead: Mary; with John, Winston.*
- **A (single writer + read-only helpers):** COG-MAW "writes stay single-threaded" ✔. CUR-SCALE lock collapse ✔. CC teams overwrite without isolation.
- **B (parallel writers in isolated worktrees + judge):** ANT-CCC 16 agents → 100k-line compiler (one vendor run); Cursor best-of-N; CC enforced worktrees. Builds on 19-3's per-Worker workspaces, and needs concurrent Workers (out of 19-3's scope) plus single-writer leases if any workspace is shared (R8).
- **Mary:** no controlled comparison exists [03 §8 Q3]. What would a maos experiment look like?

**Q10. How do we prove a UX control is real (rule 11)?** *Lead: Murat; with Amelia.*
- **A (per-verb proven-red):** each NEW story plants a defect (e.g. drop the dequeue, return an empty list on door failure) and shows the test fails (§E.4 acceptance column).
- **B (journey-level):** one J1 journey asserts the whole loop (list → inbox → resolve → resume). Fewer tests, coarser failure.
- **Also on the table:** the observed lifecycle order (transition before journal; `let _` on TL writes, [04 §5.1]). Is it acceptable for dashboard-triggered verbs under I2/I4?

**Q11. Cost and success measurement: live gauge before 20-14? Worker-CLI spend? Roster UX metrics?** *Lead: Mary; with Amelia, John.*
- **A (ship labels now, numbers later):** the omp "usage —" pattern; MW-8.
- **B (block the cost column until egress attribution exists):** avoids an incomplete number being read as complete (X5).
- **Mary:** propose and defend a roster-UX metric, since none is published. Candidates include needs-you-to-action latency from TL timestamps (§E.4 success metrics).

**Q12. Gesture and keymap: ← (codex/CC) or ←← (omp), and in which surface?** *Lead: Sally; with Amelia.*
- **A (single ← on empty input):** codex and CC agree (§D.1). Discoverable through a footer hint.
- **B (←← with a 40–500 ms debounce):** avoids misfires from terminal bursts and is tested (`omp:CA/modes/controllers/input-controller.ts:910-936`). Either must be guarded to an empty prompt, be remappable with conflict validation (`cx:tui/src/keymap.rs:3711-3838`), and keep `--plain` parity (NFR-Ops-5).

**Q13. Does maos need a command center or concurrent fan-out now?** *Lead: Mary; with John, Winston.*
- **A (not yet):** J1 runs Workers in sequence (19-3 AC3) and maos runs few Spirits per daemon. The coding-specific evidence is weak: ANT-MARS says coding has "fewer truly parallelizable tasks"; MAST says gains are "often minimal"; COG-DBMA defaults to a single-threaded agent. Ship only the honesty fixes (19-2 AC5, MW-8 labels) and NEW-1's `--json` list.
- **B (yes, build the surface first):** the operator's observations ask for it, and the read verbs are kernel-Δ 0. A roster is useful even for sequential runs (what is running, what needs me).
- **Evidence gap:** no study measures either (T01). What observation would change the decision?

### F.4 Replication recipe

#### F.4.1 Pinned sources

| Source | Clone / URL | Pin | Accessed |
|---|---|---|---|
| oh-my-pi | `git clone https://github.com/can1357/oh-my-pi && git -C oh-my-pi checkout 9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1` | `9d16dff…` (pkg 18.13.2) | 2026-10-28 |
| codex | `git init codex && cd codex && git remote add origin https://github.com/openai/codex && git fetch --depth 1 origin e6de6a2423f9a723c098db3d17ea6c71e93bfcf2 && git checkout --detach FETCH_HEAD` | `e6de6a24…` (shallow) | 2026-10-28 |
| maos | local `/home/lunarpulse/dev_ws`; `git rev-parse HEAD` | `d9bd59c456a68a9aaddbbee4594796beed288b91` (✔ re-run) | 2026-10-28 |
| OTel GenAI | `https://raw.githubusercontent.com/open-telemetry/semantic-conventions-genai/51a0f580f8a6ad804ca4de5e033683c6fbbe5906/docs/gen-ai/gen-ai-agent-spans.md` | `51a0f580…` | 2026-10-28 |
| Live docs (unpinnable) | CC-AV https://code.claude.com/docs/en/agent-view.md · CC-SUB https://code.claude.com/docs/en/sub-agents.md · CC-TEAM https://code.claude.com/docs/en/agent-teams.md · CC-WF https://code.claude.com/docs/en/workflows.md · CC-HOOK https://code.claude.com/docs/en/hooks.md · GH-SESS https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents · JULES-PLAN https://jules.google/docs/review-plan/ · DEVIN-ADV https://docs.devin.ai/work-with-devin/advanced-capabilities · DEVIN-WF https://docs.devin.ai/work-with-devin/dynamic-workflows · CUR-WT https://cursor.com/docs/configuration/worktrees.md · CUR-SCALE https://cursor.com/blog/scaling-agents · AMP-SUB https://ampcode.com/docs/models-and-subagents · FAC-SUB https://docs.factory.com/harness/subagents.md · ANT-MARS https://www.anthropic.com/engineering/multi-agent-research-system · ANT-CCC https://www.anthropic.com/engineering/building-c-compiler · COG-DBMA https://cognition.com/blog/dont-build-multi-agents · COG-MAW https://cognition.com/blog/multi-agents-working · MAST https://arxiv.org/abs/2503.13657 · OAI-HITL https://openai.github.io/openai-agents-python/human_in_the_loop/ · LG-TT https://docs.langchain.com/oss/python/langgraph/use-time-travel.md · MAF-CKPT https://learn.microsoft.com/en-us/agent-framework/workflows/checkpoints · A2A https://a2a-protocol.org/latest/specification/ · MCP-TASKS https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks.md · codex docs https://learn.chatgpt.com/docs/developer-commands?surface=cli | save with `curl -sL URL -o file` and record `date -u` before grepping | 2026-10-28 |

#### F.4.2 Commands that re-verify each load-bearing claim

Commands marked ✔ were re-run by this synthesis on 2026-10-28 and printed the expected text. The others are copied from the R1 appendices, which state they ran them.

```bash
# --- omp (repo root at 9d16dff) -------------------------------------------
sed -n 271,272p packages/coding-agent/src/modes/controllers/input-controller.ts   # ✔ 40 / 500 ms
sed -n 765,781p packages/coding-agent/src/modes/controllers/input-controller.ts   # ✔ onLeftAtStart → showAgentHub({requireContent,armCloseTap})
sed -n 817,821p packages/coding-agent/src/modes/controllers/input-controller.ts   # ✔ focused ←← → unfocusSession() (main, not parent)
sed -n 214,221p packages/tui/src/app-keybindings.ts                               # ✔ alt+a, ctrl+s "Open the agent hub"
rg -n 'id: "task\.(maxConcurrency|maxRecursionDepth|agentIdleTtlMs)"' -A3 packages/coding-agent/src/task/settings.ts  # ✔ 32 / 2 / 420_000
rg -n '"tools.approvalMode": "yolo"' packages/coding-agent/src/task/executor.ts   # ✔ line 1184
sed -n 2406,2588p packages/tui/src/overlays/agent-hub.ts                          # Hub keys (t,/,Enter,r,x)
rg -n 'focusParentSession' packages/coding-agent/src packages/coding-agent/test   # stale "hops to parent" (01 §5.4.1)
rg -n 'subagent' packages/coding-agent/src/modes/rpc/wire/commands.ts             # RPC verbs

# --- codex (repo root at e6de6a24) ----------------------------------------
sed -n 147,148p codex-rs/cli/src/main.rs                                          # ✔ "Browse all agent sessions on the shared local app-server daemon."
sed -n 150,152p codex-rs/tui/src/app/agent_center/render.rs                       # ✔ "Agent command center"
sed -n 30,39p codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs     # ✔ ← on empty composer → OpenAgentsOverview
sed -n 77,84p codex-rs/tui/src/app/agents_overview_discovery.rs                   # ✔ roots only
sed -n 1232,1243p codex-rs/tui/src/app/agents_overview.rs                         # ✔ rollup via min()
rg -n 'Any running work continues' codex-rs/tui/src/app/exit_summary.rs           # ✔ line 97
rg -n 'DEFAULT_AGENT_MAX_THREADS|DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT|DEFAULT_AGENT_MAX_DEPTH' codex-rs/core/src/config/mod.rs  # ✔ 6 / 4 / 1
sed -n 181,194p codex-rs/core/src/tools/handlers/multi_agents_spec.rs            # ✔ (review) send_input interrupt=true "interrupts the current task and handles this message immediately"
rg -n 'agents: AgentsKeymap \{' -A 14 codex-rs/tui/src/keymap.rs                  # dashboard keys
rg -n 'open_thread' codex-rs/tui/src/bottom_pane/approval_overlay.rs              # approval jump
cat codex-rs/tui/src/app/snapshots/codex_tui__app__agents_overview__tests__command_center__live_center_columns.snap

# --- maos (/home/lunarpulse/dev_ws) — read the pinned commit, never the working tree, which has moved on ---
R=d9bd59c456a68a9aaddbbee4594796beed288b91
git -C /home/lunarpulse/dev_ws cat-file -t $R                                    # ✔ commit
cd /home/lunarpulse/dev_ws
git show $R:crates/maos-control/src/lib.rs | sed -n 1101,1118p                   # ✔ GET /v1/daemon fields
git grep -c 'v1/daemon\|spirit_ids' $R -- crates/maos-cli/src                     # ✔ no matches (exit 1) → no CLI consumer
git grep -n 'drain_next\|dequeue_at_safe_point' $R -- crates spirits             # ✔ definitions, doc comments and tests only
git grep -n 'judge_ambiguity' $R -- crates/maos-bin/src crates/maos-bench/src    # ✔ maos-bin: orchestrator_root.rs:40 doc comment; maos-bench halt_eval.rs:594 (eval harness)
git show $R:crates/maos-bin/src/worker_spawn.rs | sed -n 663,667p                 # ✔ reject_t3_stdin_posture
git show $R:crates/maos-bin/src/main.rs | sed -n '4687p;5028p'                    # ✔ (review) sequential `for` over topology entries; block_in_place
git show $R:crates/maos-kernel-core/src/orchestrator/mod.rs | sed -n 3,17p        # ✔ kernel does not decide WHEN
git show $R:crates/maos-kernel-core/src/scheduler/control_block.rs | sed -n 258,287p  # ✔ no parent field; pub task_assignments_in_flight
git show $R:crates/maos-kernel-core/src/security/approval.rs | sed -n 43,75p      # ✔ "v0.3-β auto-allows"
git show $R:crates/maos-bin/src/main.rs | sed -n 3385,3387p                       # ✔ (review) `let _approval = ApprovalManager::new(…)`: never called
git show $R:crates/maos-cli/src/cli.rs | sed -n 82,101p                           # ✔ (review) help claims FR20 consumption, FR51(a) P99 ≤2s, FR51(d) fail-safe
git show $R:crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs | sed -n 1138,1151p  # ✔ Worker pause no-op
git show $R:docs/adr/ADR-069-kernel-core-envelope-through-v1.md | sed -n 35,46p   # ✔ 23,000 / zero headroom / per-story grants / 21-5 at the 22,000 alarm
git show $R:_bmad-output/planning-artifacts/prd/functional-requirements.md | sed -n 11,14p  # ✔ no orchestration policy, no UI
git show $R:_bmad-output/planning-artifacts/architecture-maos-minimal-opus/6-reference-spirits.md | sed -n 90,92p  # ✔ Observer live view intent
git show $R:_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md | sed -n 56p      # ✔ rule 11
git show $R:_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md | sed -n 34,48p   # ✔ E19 envelopes; 19-3 kernel grant +12 physical / +9 tokei
git show $R:_bmad-output/implementation-artifacts/sprint-status.yaml | grep -n '^  \(19-[0-9]\|20-3c\|20-7\|20-14\|21-5\|21-7\)'  # ✔ keys + status at the pin
git grep -n 'deliver_typed(frame, 0, None)' $R -- crates/maos-bin/src/delegation.rs  # ✔ :452,581,681,750 (pid-0 sender; 20-3c member 17)
git ls-tree --name-only $R crates/maos-telemetry/src/ | grep otel_sink             # ✔ (review) existing OTel sink (NEW-11 is an extension)
# 19-2/19-3/19-6 story drafts (2026-10-28) are untracked files under _bmad-output/implementation-artifacts/; read them in the working tree.

# --- web (documented-only claims) -----------------------------------------
curl -sL https://code.claude.com/docs/en/agent-view.md -o cc-av.md
grep -n 'claude agents\|on an empty prompt\|waitingFor\|research preview' cc-av.md   # ✔ l.9,53,59,61,824,829,834,21
grep -n 'keeps its standard interrupt behavior' cc-av.md                          # ✔ (review) l.256: Ctrl+C cancels a running response while attached
curl -sL https://code.claude.com/docs/en/sub-agents.md -o cc-sub.md
grep -n 'as a tree in the subagent panel\|Concurrent subagent limit reached\|three layers\|ultracode' cc-sub.md  # ✔ l.1055,1076,1082 (l.1082 also exempts ultracode sessions)
curl -sL https://www.anthropic.com/engineering/multi-agent-research-system | grep -o 'outperformed single-agent Claude Opus 4 by 90.2%\|about 15× more tokens\|explains 80% of the variance\|fewer truly parallelizable tasks than research' | sort -u   # ✔ all four (the coding caveat added by the review)
curl -sL https://cognition.com/blog/multi-agents-working | grep -o 'writes stay single-threaded\|roughly 58% are severe\|map-reduce-and-manage\|mostly a distraction' | sort -u   # ✔ all four
curl -sL https://cursor.com/blog/scaling-agents | grep -o 'effective throughput of two or three\|risk-averse' | sort -u   # ✔ both
curl -sL 'http://export.arxiv.org/api/query?id_list=2503.13657' | grep -o '14 unique modes'   # ✔
```

Further per-claim commands: omp [01 §7.2](agent-orchestration-2026-10-28/01-oh-my-pi.md#72-verify-the-key-claims-by-grep-run-from-the-repo-root), codex [02 §7 R1–R10](agent-orchestration-2026-10-28/02-codex.md#7-replication-appendix), landscape [03 §7.2](agent-orchestration-2026-10-28/03-landscape.md#72-fetch-and-grep-checks-one-per-key-claim), maos [04 §7.2](agent-orchestration-2026-10-28/04-maos-baseline.md#72-static-checks-read-only-each-reproduces-a-key-claim). The independent audit's full command log, with cwd and exit code for each command (K1–K97), is in [05 §6](agent-orchestration-2026-10-28/05-citation-check.md#6-command-log-verbatim-cwd-exit-code). To avoid working-tree drift, it reads maos through `git -C /home/lunarpulse/dev_ws show d9bd59c4:<path>`. When this synthesis ran, `git status --short` listed only untracked files (new 19-2/19-3/19-6 story drafts and this research directory), so the tracked files cited here match `d9bd59c4`.

#### F.4.3 Runtime replays (NOT executed; listed for a later agent)

- omp UX: [01 §7.3](agent-orchestration-2026-10-28/01-oh-my-pi.md#73-reproduce-the-ux-installed-omp-interactive-tui). Spawn two scouts, clear the editor, then ← … ← (0.1–0.4 s apart) → Hub; Enter; Esc; `x` → `<id>.jsonl.tombstone`.
- codex UX: [02 §7 Runtime check](agent-orchestration-2026-10-28/02-codex.md#runtime-check-not-executed-in-this-study-hard-rule-no-builds-or-installs). Needs `cargo build -p codex-cli` + login.
- Claude Code: [03 §7.3](agent-orchestration-2026-10-28/03-landscape.md#73-interactive-verification-not-executed-in-this-study-requires-the-product).
- maos: [04 §7.3](agent-orchestration-2026-10-28/04-maos-baseline.md#73-live-replay-not-executed-in-this-study--needs-a-build-and-a-provisioned-t3-substrate). Needs a build + T3 substrate. Expected: `/v1/daemon` lists Spirits; `orchestrator status` pending never falls; `maosctl stop` refused.

#### F.4.4 Verification level of each load-bearing claim

| Level | Claims |
|---|---|
| **Runtime-verified** | **Nothing.** No omp, codex or Claude Code UI was launched by any R1 study, the citation audit, this synthesis or the adversarial review, and no maos binary was built or run ([05 §4.1]). Every UX flow in this report (omp ←← → Hub → Enter → steer; codex F1–F6; Claude Code agent view; maos Flows A–H) is a reconstruction from code, golden snapshots or vendor docs and must not be quoted as "observed". The only host observations, which are not UX runs: the installed `omp/18.13.1` version string; an on-disk omp subagent JSONL header with `parentSession` ([01 §7.4]); omp's agent-facing coop roster text "331 parked peer(s) omitted" in [01]'s and this synthesis agent's own system prompt (a prompt roster, not the Hub UI; the count drifts: 336 in the review agent's prompt) |
| **Source-verified** (pinned SHA, re-run ✔ where marked) | omp gesture, Hub, focus, lifecycle, limits, yolo children, RPC verbs ([01]); codex command center, daemon requirement, root-only listing, rollup, exit message, limits, approval routing, status enums ([02]); all maos baseline facts ([04]) and tracker keys |
| **Documented-only** (vendor docs and live pages; no product installed or launched; may drift) | **Every landscape product claim** ([05 §4.4]: twelve re-fetched and still matching on 2026-10-28): all Claude Code (agent view, subagent tree, teams, hooks, workflows, checkpoints), Copilot, Jules, Devin, Cursor, Amp, Factory, OpenHands, OpenAI Agents SDK, LangGraph/LangChain, MAF, CrewAI, A2A, MCP, OTel. Also: codex's official docs page and its `codex app-server --listen` flag syntax ([05 §4.3]); omp claims that rest only on `docs/` (discovery precedence, no `AGENTS.md` to children, `--restore`, inline 11,500-char cap) ([05 §4.2]); maos product intent (Observer live view, J1 digest, 2026-07-17 delegation analysis) ([05 §4.5]) |
| **Reported outcomes (vendor- or author-reported, not reproduced)** | ANT-MARS 90.2 % / 80 % / 15× (research tasks; the article itself calls coding a weaker fit); COG-MAW 2 bugs/PR, 58 % severe (self-report on its own product); CUR-SCALE 20 → 2–3 throughput (one harness); ANT-CCC 16 agents/$20k (one run); MAST 14 modes, κ = 0.88, gains "often minimal" (arXiv preprint). **Illustrative, not measured:** LC-MA call/token worked examples |
| **[SECONDARY], excluded from load-bearing use** | Devin child indentation; Jules quotas; OpenHands parallel `DelegateTool` [03 §8 Q4–Q6]; the two discovery searches in §F.6 |

### F.5 Evidence index

| R1 note | Sections used | Used in |
|---|---|---|
| [01-oh-my-pi](agent-orchestration-2026-10-28/01-oh-my-pi.md) | §1 Summary; §2 Capability inventory; §3.1 ←← flow; §3.2 revive/kill; §3.3 Activity; §4 Techniques 1–14; §5.1 limits; §5.2 risks; §5.3 enforced vs advised; §5.4 discrepancies; §6 lessons; §7 replication; §8 open questions | §B.1, §B.2, §C, §D.1–§D.3, §E.3 X1/X2/X10/X11, §F.4 |
| [02-codex](agent-orchestration-2026-10-28/02-codex.md) | §1 Summary 1–12; §2a command center; §2b actions; §2c sessions/subagents/control plane; §2d observability; §3 flows F1–F6; §4 patterns P1–P16; §5 limits; §6 lessons; §7 R1–R10; §8 open questions | §B, §C, §D.1–§D.3, §E.2 R1/R3/R16, §F.4 |
| [03-landscape](agent-orchestration-2026-10-28/03-landscape.md) | Source keys; §1 Summary; §2.1 products; §2.2 frameworks/protocols; §3 flows; §4.0 evidence base; §4.1 T1–T23; §4.2 relevance table; §5 risks 1–12; §6 L1–L13; §7 replication; §8 open questions | §B.2, §D.1 convergence, §D.2 "Claude Code"/"Other", §D.3 measured outcomes, §E.3, §F.3 Q9/Q11, §F.4 |
| [04-maos-baseline](agent-orchestration-2026-10-28/04-maos-baseline.md) | §1 Summary; §2.1 inventory; §2.2 needs → gaps (a)–(g); §3 flows A–H; §4 techniques 1–13; §5.1 constraints; §5.2 risks; §5.3 costs; §6 lessons 1–12; §7 replication; §8 open questions | §D.2 "maos today"/"gap", §E.1–§E.4, §F.3 |
| [05-citation-check](agent-orchestration-2026-10-28/05-citation-check.md) | §0 pins (maos tree read via `git show d9bd59c4:`); §1 totals (48 claims: 44 confirmed, 3 line-drift, 1 partial, 0 not-found, 0 contradicted); §2 per-note verdicts O1–O12, C1–C12, L1–L12, M1–M12; §3 corrected wording (O4 Hub `1`/`2` keys at `agent-hub.ts:613-620`; O7 status union at `agent-hub-types.ts:7`; C7 codex has **four** buckets; M2 `subcommands.rs:2536-2567`; M5 pid-0 source lines `delegation.rs:452,581,681,750`); §4 documented-only and runtime caveats; §6 command log K1–K97 | Research Overview; §B.2 status table; §D.1 Observation A; §D.1 comparison "Triage order"; §D.2 rows 3 and 5; §E.4 rank 5; §F.3 Q3; §F.4.2; §F.4.4 |
| [06-adversarial-review](agent-orchestration-2026-10-28/06-adversarial-review.md) | §1 re-verification V01–V31 (commands, observed output, verdicts); §2 findings F01–F31; §3 roadmap status alignment; §5 command log | Changelog "Review 2026-10-28"; corrections throughout |

Cross-reference of R1 "candidate lessons" to this report's proposals:

| Proposal | omp [01 §6] | codex [02 §6] | landscape [03 §6] | maos [04 §6] |
|---|---|---|---|---|
| NEW-1 roster | 1, 3 | 1, 2 | L1 | 1 |
| NEW-2 session view / follow | 4, 5 | 4 | L1 | 3, 5 |
| NEW-3 inbox | 12 | 8 | L3, L10, L11 | 7 |
| NEW-4 tree | 1 | 3 | L2 | 2, 12 |
| 19-2 steering | — | — | L4 | 6 |
| NEW-5 cost | 9 | 11 | — | 10 |
| NEW-6 runbook/notify | — | 10 | — | — |
| NEW-7 status vocabulary | — | 6 | — | — |
| NEW-8 fan-out budget | 7 | 7 | L9 | — |
| NEW-9 interactive view | 3, 11 | 9 | L1, L2 | 4 |
| NEW-10 parked | 2 | — | — | — |
| NEW-11 OTel (an extension of the NFR-Obs-2 owner) | — | — | L12 | — |
| Single-writer / reviewer | — | — | L5, L6 | — |
| Labels for unreal controls | — | — | — | 11 |

### F.6 Technical Research Methodology and Source Verification

- **Inputs:** four R1 studies plus the citation audit `05-citation-check.md` (all 2026-10-28), read by this synthesis. The audit's corrections (C7 four buckets, O4/O7/M2 line drift, M5 source lines) are applied in the body.
- **Re-verification by this synthesis:** the shell commands marked ✔ in §F.4.2, run 2026-10-28 ≈11:50–11:58Z, all exit 0 except the intentional zero-match `rg -c` (exit 1). Web re-fetches: CC-AV (148,269 bytes), CC-SUB, ANT-MARS, COG-MAW, CUR-SCALE, arXiv 2503.13657. The synthesis did not record the outputs of these re-runs. The adversarial review ([06](agent-orchestration-2026-10-28/06-adversarial-review.md) §1, §5) re-ran 31 load-bearing checks against the pins, recorded the observed output and exit codes, and rewrote the maos commands in §F.4.2 to read `d9bd59c4` through `git show`/`git grep` instead of the working tree.
- **Discovery web searches (this synthesis):** `"codex agents" command center app-server daemon sessions` and `multi-agent coding orchestration operator dashboard needs input roster 2026`. Both returned only secondary summaries (blogs, package pages, a vendor announcement). Nothing in this report rests on them. They also did not contradict the source-verified codex facts. One summary mentions a `--thread` flag on `codex agents` that was not source-checked, so it is not used.
- **R1 web queries and fetches:** [03 §7.2] lists every fetch-and-grep.
- **Limitations:**
  - No UX was run.
  - The codex clone is shallow (no feature dates).
  - The installed omp is one patch behind the source.
  - Vendor docs move ([03 §5.12]). For example, CC-SUB now also says ultracode sessions are exempt from the 20-subagent concurrency limit (sub-agents.md l.1082 ✔; [05 §4.4]).
  - No published roster-UX measurement exists.
  - maos claims are at `d9bd59c4`. Stories 19-1 and 19-7 are in progress, and 19-2, 19-3 and 19-6 were drafted on 2026-10-28 (`ready-for-dev`); all of them will move line numbers.

### Technical Research Conclusion

**Summary.** Source confirms the operator's instincts for omp and codex; Claude Code's docs describe the same shape. Three products now give the operator one roster, sorted around the human, reachable with one key, with sessions that outlive the view, steering that waits for a safe boundary, and a separate explicit interrupt. maos already holds the hard parts: an authenticated door, live SCB state, an append-only TL, sealed T3 Workers, and a safe-point steering buffer. It lacks the read projections and the honesty fixes that turn those into an operator surface.

**Strategic impact.** maos does not need to copy omp or codex internals. A maos-shaped command center is a set of kernel-Δ 0 read verbs (list, session, inbox, tree) plus Orchestrator policy (steering, limits, cascades). It is governed by the rules maos has adopted: one door, log before act, no third state store, consent as the only authority, and "a control is real, or it is labelled". By design this is structurally stronger than the surveyed products on authority (no yolo children, no auto-approve) [INFERENCE]. It is not yet true in production: the consent granter is unwired (21-7), lifecycle verbs transition before they journal (MW-4), and `maosctl --help` advertises controls that do not exist until 19-2 lands.

**Next steps.**
1. Run a `bmad-party-mode` roundtable on §F.3, with Q13 first, then Q1, Q4 and Q8.
2. Ask the operator for kloc-envelope placement of NEW-1…NEW-4.
3. Land 19-2 before any UI shows a "steer" control.
4. Keep this report's verification labels in place when any of it is quoted into a story.

---

**Technical Research Completion Date:** 2026-10-28
**Research Period:** sources pinned and accessed 2026-10-28
**Source Verification:** every claim cites a pinned `repo@sha:path:line`, a URL accessed 2026-10-28, or an R1 section. Inferences are tagged `[INFERENCE]`.
**Confidence:** stated per claim by verification level (§F.4.4), not by a single label. No UX claim is runtime-verified.

---

## Changelog

### Review 2026-10-28

The `ResearchReview` agent ran an adversarial review ([06-adversarial-review](agent-orchestration-2026-10-28/06-adversarial-review.md)).

- **Verification:** 31 load-bearing claims were re-checked against the pins: 19 confirmed, 5 partial, 6 contradicted or misclassified, 1 duplicate.
- **Findings:** 31 (6 High, 16 Medium, 9 Low). All are applied here, as follows.

**Status and scope**
- **F01 Statuses.** 19-2, 19-3 and 19-6 are now story files (`ready-for-dev`, 2026-10-28) with recorded operator rulings. Changed: §E.2 R4/R7/R12; §E.4 intro and ranks 2, 7 and 14; §F.2 RQ6; §F.6.
- **F02 19-2 scope.** D-19-2-01 B absorbs FR51(a)(d). Changed: MW-7, R4, rank 2, Q6, §F.1 items 8–10.
- **F03 Interrupt counter-evidence.** codex, Claude Code and omp all offer a hard interrupt next to queued steering. Changed: Research Overview, §D.1 steering row and common core, T06, §F.1 item 3, RQ2, Conclusion.
- **F04 19-3 kernel cost and F05 ADR-069 reading.** 19-3 is +9 tokei (ratified). Kernel-Δ>0 options need their own grant; 21-5 is a relief valve, not a gate. Changed: R5, R7, rank 15, Q5, RQ5.

**Roadmap rows**
- **F06 Scope creep, F07 duplicate ACs, F08 roster-row acceptance.** The single-writer lease is no longer an acceptance signal for 19-3/21-7; rank 2 cites 19-2's own ACs; rank 7 cites 19-6's own ACs. Changed: R8, ranks 2, 7, 12 and 14, Q9.
- **F09–F11 Duplicates, origins and a hidden dependency.** NEW-11 is an extension of NFR-Obs-2 / `maos-telemetry`; NEW-6 cites Story 8.13; NEW-1's Needs-you bucket waits for NEW-3.

**Evidence quality**
- **F12–F17.** Added the ANT-MARS coding caveat, MAST "often minimal" and COG-MAW "mostly a distraction". T12's outcome cell now holds limit evidence only. LC-MA is relabelled illustrative. COG-MAW is labelled a vendor self-report. Claude Code limits are labelled documented, with the ultracode exemption. "Independent", "near-identical" and "converged" are softened.

**maos facts**
- **F18–F23.** `ApprovalManager` is never called. `maosctl --help` mislabels FR51(a)(d) and FR20 consumption (X2, X8, success metrics). The sequential-loop cite is corrected. "Silently" is removed from X9. omp approvals wording is corrected. E19 envelopes now reflect `w2_kloc` (maos-bin 31,353; aggregate 191,248).

**Replication and agenda**
- **F24 Replication.** The §F.4.2 maos commands read `d9bd59c4` through `git show`/`git grep`. Review checks were added for the codex interrupt, Claude Code `Ctrl+C`, the ANT-MARS coding caveat, the CC ultracode exemption, `ApprovalManager`, the `--help` claims and the telemetry sink.
- **F25 Agenda.** Q4, Q6 and Q11 are neutralised. Q13 (the null option) is added and comes first in Next steps.

**Smaller fixes (F26–F31)**
- R3 is tagged [INFERENCE], with a note on read authority.
- The X10 / §F.4.4 roster observation is relabelled.
- Success metrics are now measurable.
- The scope of the rank-8 grep is stated.
- The strategic-impact claim is qualified.
- T-row cross-references are corrected (R3 → T04; rank 17 has no T-row).

**Follow-up (same day, after the 19-2 amendment and the tracker update)**
- **F02 refreshed.** 19-2 absorbs FR51(a)(d) as AC7 (pause fence, Worker freeze, `dispatch_paused` hold; P99 ≤ 2 s measured), AC8 (revoke: post-call re-check, Worker stop) and AC9 (rule-11 relabels). It carries a ◆ FLAG-Winston kernel request of **+74 physical / +46 tokei** (D-19-2-20; prototype line counts, gate measurement owed; E19 kernel endpoint → 26,464 / 20,674 if granted). D-19-2-13…20 await the operator's ruling. Changed: MW-7, R4, §E.4 status update, rank 2, rank 15, RQ5, Q6.
- **F01 refreshed.** The tracker now reads `ready-for-dev` for 19-3 (`:289`) and 19-6 (`:292`); 19-2 (`:288`) stays `backlog` pending the ruling. Changed: R7, R12, §E.4 status update, ranks 2, 7 and 14. R7 also notes the tracker's +11-physical measurement for 19-3 next to the +12 grant.
- **F20 carried into the R1 note.** `04-maos-baseline.md` §2.1 "Topology run loop" no longer cites `worker_spawn.rs:55-79` (that is `RunArgs`). It now cites `main.rs:4687` (the sequential `for`) and `main.rs:5028` (`block_in_place`), verified at `d9bd59c4`.

**Follow-up 2 (same day, after the operator's 19-2 rulings)**
- **F02 settled.** D-19-2-13…19 were ruled A (D-14 with the C fallback, 409 `worker_pause_unsupported`, if CI check #15 fails). D-19-2-20 was ruled **B**: the kernel grant is capped at **+93 physical / +58 tokei** (prototype +74 / +46 plus a 25 % margin), re-pinned at landing, with an E19 kernel endpoint ≤ 26,483 / ≤ 20,686 on top of 19-3's K2. "Pending ruling" is removed throughout. Changed: MW-7, R4, §E.4 status update, ranks 2 and 15, RQ5, Q6.
- **F01 settled.** The tracker now reads `ready-for-dev` for 19-2 (`:288`) as well. Changed: R4, §E.4 status update, rank 2.
