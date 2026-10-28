# 05 — Citation check of R1 notes 01–04

- **Date:** 2026-10-28. **Author:** CitationCheck (sub-agent). **Language:** English.
- **Scope:** 48 load-bearing claims, 12 from each R1 note: `01-oh-my-pi.md`, `02-codex.md`, `03-landscape.md`, `04-maos-baseline.md`. A claim counts as load-bearing if an operator observation, a comparative table or a "lessons for maos" item depends on it.
- **Method:** For each claim, I re-opened the cited `path:line` in the pinned tree, or re-fetched the cited URL, and compared it with the note's wording. Every command is listed verbatim in §6 with its working directory and exit code. Command ids (`K<n>`) in the tables point there. Nothing was built, run, installed or edited. The only file written is this one.
- **Verdict scale:**
  - **CONFIRMED** — the cited lines say what the note says.
  - **LINE-DRIFT** — the content exists, but at other lines; the right lines are given.
  - **PARTIAL** — part of the claim is right and part is overstated or mis-described.
  - **NOT-FOUND** — the cited lines do not contain the claimed content.
  - **CONTRADICTED** — the source says something else.

## 0. Pins re-verified

| Tree | Expected | Observed | How |
|---|---|---|---|
| omp `/home/lunarpulse/research-src/oh-my-pi` | `9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1` | `9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1` | `git -C /home/lunarpulse/research-src/oh-my-pi rev-parse HEAD` |
| codex `/home/lunarpulse/research-src/codex` | `e6de6a2423f9a723c098db3d17ea6c71e93bfcf2` | `e6de6a2423f9a723c098db3d17ea6c71e93bfcf2` | `git -C /home/lunarpulse/research-src/codex rev-parse HEAD` |
| maos `/home/lunarpulse/dev_ws` | `d9bd59c4` | HEAD `d9bd59c456a68a9aaddbbee4594796beed288b91`, but `git status --short \| wc -l` = **3** (dirty) | `git -C /home/lunarpulse/dev_ws status --short \| wc -l; git -C /home/lunarpulse/dev_ws rev-parse HEAD` |

The maos working tree had 3 uncommitted changes when checked. So every maos read below goes through `git -C /home/lunarpulse/dev_ws show d9bd59c4:<path>` or `git grep … d9bd59c4 -- <paths>`, never the working tree. URLs were fetched live on 2026-10-28 with `curl -sL` and piped to `grep`. Nothing was saved to disk.

## 1. Totals

| Note | Checked | CONFIRMED | LINE-DRIFT | PARTIAL | NOT-FOUND | CONTRADICTED |
|---|---|---|---|---|---|---|
| 01-oh-my-pi | 12 | 10 | 2 | 0 | 0 | 0 |
| 02-codex | 12 | 11 | 0 | 1 | 0 | 0 |
| 03-landscape | 12 | 12 | 0 | 0 | 0 | 0 |
| 04-maos-baseline | 12 | 11 | 1 | 0 | 0 | 0 |
| **Total** | **48** | **44** | **3** | **1** | **0** | **0** |

All four non-CONFIRMED items are about precision (the line range or the bucket count). None changes a comparative conclusion or a lesson. Both operator observations are confirmed in source:

- omp ←← opens the Agent Hub (O1, O2).
- `codex agents` is the "Agent command center" dashboard, reachable with `←` from an empty composer (C1–C3, C8, C9).

## 2. Per-note results

### 2.1 `01-oh-my-pi.md` (omp@9d16dff; `CA/` = `packages/coding-agent/src/`, `TUI/` = `packages/tui/src/`)

| # | Claim (short) | Note's cite | Verdict | What the source shows | Cmd |
|---|---|---|---|---|---|
| O1 | ←← on an empty editor, 40–500 ms gap, opens the Hub; exactly two taps; faster bursts ignored | `CA/modes/controllers/input-controller.ts:271-272,765-781,919-936`; `TUI/prompt/custom-editor.ts:1711-1717` | CONFIRMED | `LEFT_DOUBLE_TAP_MIN_GAP_MS = 40` / `MAX = 500` at :271-272. The `onLeftAtStart` handler at :772-781 calls `showAgentHub({ requireContent: true, armCloseTap: true })` (:779). `#detectLeftDoubleTap` at :919-936 (`leftTapCount === 2 && sinceLast >= MIN`). The editor fires only for plain `left` on trimmed-empty text (:1714-1716). | K0, K1 |
| O2 | The gesture only opens the Hub if live or persisted subagents exist | `CA/modes/controllers/selector-controller.ts:2554-2579` | CONFIRMED | `if (options?.requireContent && hub.isEmpty) { done(); return; }` runs after `hub.persistedSubagentsReady`. | K2 |
| O3 | The same Hub is bound to `Alt+A` and `Ctrl+S` | `TUI/app-keybindings.ts:214-221` | CONFIRMED | `"app.agents.hub": defaultKeys "alt+a"`; `"app.session.observe": defaultKeys "ctrl+s"`. Both are described as "Open the agent hub". | K3 |
| O4 | Hub keys: `t` tree, `/` filter, `1`/`2` sections, `r` revive, `x` kill, `Enter` open | `TUI/overlays/agent-hub.ts:2406-2588` (§1) | **LINE-DRIFT** (partial range) | `/` :2436, `t` :2456, Enter :2490, `r` :2495, `x` :2499 are inside the range. `1`/`2` are at **:613-620** (`#switchSection("agents"/"activity")`), outside the §1 range. The note's own §3.1 step 7 cites `:613-620` correctly. | K4, K9 |
| O5 | Opening an agent re-points transcript, status line and editor at the child; `Esc` or ←← returns to *main*; Esc never interrupts the child | `CA/modes/controllers/session-focus-controller.ts:32-112`; `input-controller.ts:590-600,817-821` | CONFIRMED | Class doc :32-41: "re-points the transcript, streaming event subscription, status line, and editor…". Esc branch :590-599: "Esc never interrupts the focused agent's turn" → `unfocusSession()`. Focused ←← → `#handleFocusedLeftTap` → `unfocusSession()` (:817-821). Hub Enter comment "exact parity by construction" at `TUI/overlays/agent-hub.ts:2505-2510`. | K0, K8, K26, K28 |
| O6 | Discrepancy: the status text says "←← hops to parent", but code and tests return to main; `focusParent()` has no caller | `session-focus-controller.ts:109,114-122`; `test/input-controller-escape.test.ts:954-972`; `docs/agent-hub.md:22,78` | CONFIRMED | :109 reads `… ${formatDoubleTap("left")} hops to parent`. `focusParent()` is at :114-122. `focusParentSession` appears only at its definition (`interactive-mode.ts:1888`), its type (`types.ts:195`), a test mock (:240) and `expect(ctx.focusParentSession).not.toHaveBeenCalled()` (:971). The docs say "returns to the main session". | K5, K6, K7, K10 |
| O7 | Agent status is `running \| idle \| parked \| aborted`; finished agents stay `idle`, then get parked | `CA/registry/agent-registry.ts:1-10,66-95`; `agent-lifecycle.ts:1-23` | **LINE-DRIFT** | `agent-registry.ts:1-10` names `idle`/`parked` only. :92-95 `isResumable` names `parked`/`aborted`. The four-value union is defined at **`TUI/overlays/agent-hub-types.ts:7`** (`export type AgentStatus = "running" \| "idle" \| "parked" \| "aborted";`) and imported at `agent-registry.ts:17`. `agent-lifecycle.ts:1-23` confirms idle → parked → revived and the revivable `aborted`. | K12, K14, K15, K19, K24, K25 |
| O8 | Defaults: `task.maxConcurrency` 32, `task.maxRecursionDepth` 2, `task.agentIdleTtlMs` 420,000; isolation off | `CA/task/settings.ts:295-298,331-334,384-386,27-30` | CONFIRMED | `default: 32` (:298), `default: 2` (:334), `default: 420_000` (:386), `task.isolation.enabled default: false` (:27-30). Each of the first two also carries `protocolDefault: ["rpc","acp"]`. The note does not discuss this. | K11, K27 |
| O9 | Children run with `tools.approvalMode: "yolo"`; the parent's `task` approval is the boundary | `CA/task/executor.ts:1175-1198,1181-1184` | CONFIRMED | :1181-1184 comment: "the parent task approval is the authorization boundary. Use yolo mode…". `"tools.approvalMode": "yolo"` at :1184. Also `"advisor.enabled": false` and `"compaction.midTurnEnabled": true`. | K16 |
| O10 | `MAX_YIELD_RETRIES = 3`, the last reminder forced | `CA/task/executor.ts:2412,2587-2598` | CONFIRMED | `const MAX_YIELD_RETRIES = 3;` at :2412. `isFinalRetry = retryCount >= MAX_YIELD_RETRIES` at :2587. | K17, K20 |
| O11 | RPC verbs `get_subagents`, `get_subagent_messages{subagentId\|sessionFile,fromByte}`, `steer_subagent`, `cancel_subagent`, `set_subagent_subscription` | `CA/modes/rpc/wire/commands.ts:149-185` | CONFIRMED | All five are present at :149-185. `get_subagents` is documented as "List running subagents". `cancel_subagent` is "Hard-kill one running subagent". | K18 |
| O12 | Shape rules enforced in code (empty `tasks`, top-level `task`/`agent`, duplicate names case-insensitive, missing `context`); one semaphore per session | `CA/task/index.ts:234,262-300,661-672` | CONFIRMED | Messages at :234 ("Top-level `agent`…"), :266-267 ("Missing `tasks`"), :269-270 ("Top-level `task`…"), :287 ("Duplicate task name … (case-insensitive)"), :291-292 ("Missing `context`"). Semaphore doc at :661-667 ("One semaphore per TaskTool instance (i.e. per session)"). | K21, K22 |

Supplementary check (not counted): IRC `MAILBOX_CAP = 100` at `CA/irc/bus.ts:30-31` — CONFIRMED (K23).

### 2.2 `02-codex.md` (codex@e6de6a24)

| # | Claim (short) | Note's cite | Verdict | What the source shows | Cmd |
|---|---|---|---|---|---|
| C1 | `codex agents` is clap `Agents(AgentsCommand)`, "Browse all agent sessions on the shared local app-server daemon" | `codex-rs/cli/src/main.rs:147-148` | CONFIRMED | Doc comment at :147, variant at :148. | K29 |
| C2 | The view is titled "Agent command center"; the module doc says "Dashboard for inspecting and managing the TUI's retained daemon tasks" | `tui/src/app/agent_center/render.rs:150-152`; `agents_overview_view.rs:1-3` | CONFIRMED | `"Agent command center".bold()` at :151; the module doc at :1. | K30, K31 |
| C3 | `←` on an empty composer opens the command center; the footer says "← for agents" | `tui/src/bottom_pane/chat_composer/agents_navigation.rs:30-39` + snapshot | CONFIRMED | `handle_agents_navigation` sends `AppEvent::OpenAgentsOverview` when the key is plain Left and `agents_navigation_available()` holds (that requires `command_center_enabled`, focus, `is_empty()`, no popup, no paste burst, no pending vim operator; :18-28). The snapshot footer reads `← for agents · ? for shortcuts`. | K32, K33 |
| C4 | Dashboard single keys o, `/`/F3, n, w, f, r, x, a, Backspace, h, p, g; Enter/→ open | `tui/src/keymap.rs:1973-1987`; `agents_overview_view.rs:699-826` | CONFIRMED | `AgentsKeymap` defaults at :1973-1987 match exactly. `ListAction::Accept => self.activate()` at :815; `MoveRight … => self.activate()` at :824. | K34, K39 |
| C5 | The dashboard lists roots only; subagent threads are filtered out | `tui/src/app/agents_overview_discovery.rs:77-84` | CONFIRMED | `is_overview_thread`: `!ephemeral && parent_thread_id.is_none() && !SubAgent(ThreadSpawn)`. | K35 |
| C6 | A root row's status is the most urgent over the root and all its descendants | `tui/src/app/agents_overview.rs:1232-1243` | CONFIRMED | A recursive `fold(for_thread(thread), \|g, child\| g.min(agents_overview_group(child,…)))`. | K36 |
| C7 | `ThreadStatus` maps to "Needs input ● / Working ● / Ready ○ / Inactive ○ / Error !" buckets, "which sort in that order" (§1 item 6) | `agents_overview_view.rs:69-102,434-446`; `app-server-protocol/src/protocol/v2/thread.rs:1701-1718` | **PARTIAL** | The protocol enum is confirmed (`NotLoaded, Idle, SystemError, Active{active_flags}`; flags `WaitingOnApproval`, `WaitingOnUserInput`). But there are **four** buckets, not five: `enum AgentsOverviewGroup { NeedsYou, Working, Ready, Finished }`, with derived `Ord` (:69-75). "Error !" is only a *display* variant of `NeedsYou` for a root `SystemError` (:436-440), so it sorts with Needs input, not after Inactive. The §2a "Status buckets" row describes this correctly; the §1 summary does not. | K37, K38 |
| C8 | `codex agents` auto-starts the daemon and refuses `--no-daemon`; socket at `$CODEX_HOME/app-server-control/app-server-control.sock` | `cli/src/main.rs:2503-2508,2538-2555`; `app-server-transport/src/transport/mod.rs:56-73` | CONFIRMED | Fatal message "--no-daemon cannot be used with codex agents…" at :2503-2506. `codex_app_server_daemon::run(AppServerLifecycleCommand::Start)` at :2549. Socket dir/file constants at :56-57 and `app_server_control_socket_path` at :67-73. | K40, K41 |
| C9 | On exit: "Disconnected from this task. Any running work continues." plus `resume` and `agents` commands | `tui/src/app/exit_summary.rs:95-119` | CONFIRMED | Literal at :97. "To reconnect, run:" + `resume <id>` at :105-111. "Stop the current turn: run … agents, select this task, and …" at :112-119. The golden snapshot matches (stop key `f10` from the test override). | K42, K43 |
| C10 | `multi_agent` is Stable and default-on; `multi_agent_v2` is Stable and default-off | `features/src/lib.rs:1556-1567` | CONFIRMED | `Feature::Collab, key "multi_agent", Stage::Stable, default_enabled: true`; `Feature::MultiAgentV2, key "multi_agent_v2", Stage::Stable, default_enabled: false`. | K44 |
| C11 | Defaults: 6 concurrent threads (v1), 4 (v2), depth 1 | `core/src/config/mod.rs:260-270` | CONFIRMED | `DEFAULT_AGENT_MAX_THREADS = Some(6)` (:260), `DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION = 4` (:261), `DEFAULT_AGENT_MAX_DEPTH = 1` (:270). Also the v2 wait timeout: 10 s minimum, 1 h maximum, 30 s default (:262-264). | K45 |
| C12 | A subagent's approval appears in the current view with "o to open thread", which jumps to that thread | `tui/src/bottom_pane/approval_overlay.rs:567-575,671-681` | CONFIRMED | The `open_thread` key sends `AppEvent::SelectAgentThread(request.thread_id())` when `thread_label().is_some()` (:567-574). The hint text `" to open thread"` at :680. | K46 |

Supplementary checks (not counted; codex lesson 6 and the §5 v1 caveat rest on them):
- The closed status enum `pending_init | running | interrupted | shutdown | not_found | {completed} | {errored}` at `multi_agents_spec.rs:384-413` — CONFIRMED (K48).
- v1 "Completed agents remain open and count toward the concurrency limit until closed" at `:347` — CONFIRMED (K47).

### 2.3 `03-landscape.md` (live URLs, fetched 2026-10-28)

| # | Claim (short) | Note's cite | Verdict | What the page shows | Cmd |
|---|---|---|---|---|---|
| L1 | `claude agents` opens an agent-view roster; `←` on an empty prompt backgrounds the session and opens agent view; footer `← for agents`; research preview | CC-AV §Quick start, §Switch sessions | CONFIRMED | agent-view.md l.9 ("Agent view, opened with `claude agents`…"), l.21 ("research preview"), l.53 (attach Enter/→, `←` detaches), l.61-63 (`← for agents` hint), l.262 ("pressing `←` on an empty prompt backgrounds it and opens agent view with that row selected"). | K75 |
| L2 | `claude agents --json` gives `state` ∈ working/blocked/done/failed/stopped and `waitingFor` (permission prompt, input needed, sandbox request, …) | CC-AV §Read session state from a script | CONFIRMED | l.814, l.822 (state values), l.824 (`waitingFor` values, also `worker request` and `dialog open`), l.829 (recommends polling for supervising sessions). | K76 |
| L3 | A supervisor runs the sessions; state in `~/.claude/jobs/<id>/state.json` and `~/.claude/daemon/roster.json`; unattached idle sessions are stopped after about 1 h and resume on attach; pinned sessions stay alive; groups Ready for review / Needs input / Working / Completed | CC-AV §The supervisor process, §Where state is stored, §Monitor sessions | CONFIRMED | l.850-858 ("Finished or waiting … unattached for about an hour: the supervisor stops the process … Pin a session with `Ctrl+T` to keep its process running"), l.885-886 (both files), l.305 (group order). | K77, K78, K79 |
| L4 | Nested subagents render "as a tree in the subagent panel" with `(+N)` and "a path back to `main`" | CC-SUB §Let subagents spawn their own subagents | CONFIRMED | sub-agents.md l.1076 has this exact sentence. | K80 |
| L5 | Defaults: 20 concurrent subagents (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`) and depth 3 (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`); `Concurrent subagent limit reached` tells Claude not to retry; `Agent` tool withheld at the depth limit | CC-SUB §Concurrent subagent limit | CONFIRMED | l.1055 ("up to three layers… withholds the `Agent` tool"), l.1062, l.1082 (20, error text, "tells Claude not to retry"). The page also says ultracode sessions are exempt, which the note omits. | K80 |
| L6 | Copilot applies a steer "after it finishes its current tool call"; Stop ends the Actions run and keeps commits; cloud sessions can't be deleted | GH-SESS | CONFIRMED | Article body l.33, l.39, l.49. | K81 |
| L7 | ANT-MARS: +90.2%; token usage explains 80% of variance; agents use about 4× and multi-agent about 15× chat tokens | ANT-MARS | CONFIRMED | Exact phrases are present in the page HTML. | K82 |
| L8 | COG-MAW: "writes stay single-threaded"; Devin Review catches about 2 bugs/PR, ~58% severe; swarms "mostly a distraction"; "map-reduce-and-manage" | COG-MAW | CONFIRMED | All four phrases are present. | K83 |
| L9 | CUR-SCALE: 20 agents slowed to "the effective throughput of two or three"; flat agents "became risk-averse" | CUR-SCALE | CONFIRMED | Both phrases are present. | K84 |
| L10 | Devin workflows key each `agent()` call "by a hash of its prompt, schema, and execution settings" and replay completed calls on resume | DEVIN-WF §Determinism and resuming | CONFIRMED | "each agent call is keyed by a hash of its prompt, schema, and execution settings"; "completed agents replay their recorded results instantly". | K85 |
| L11 | Jules auto-approves plans on a timer in the UI (fail-open gate) | JULES-PLAN | CONFIRMED | "If you navigate away, Jules will eventually auto-approve the plan, which is set on a timer." | K87 |
| L12 | Claude agent teams: a teammate "can't approve a permission prompt or supply consent on your behalf"; claiming uses file locking; JSON inbox per agent; launch without confirmation | CC-TEAM §Messages between agents, §Architecture | CONFIRMED | agent-teams.md l.302, l.200, l.243 (`~/.claude/teams/{team-name}/inboxes/{agent-name}.json`), l.228 ("doesn't ask you to confirm the launch"). | K88 |

Supplementary check (not counted): DEVIN-ADV — children each "in its own isolated VM", started "without asking first" by default, toggle "Auto-approve child sessions", coordinator "woken automatically" — CONFIRMED (K86).

### 2.4 `04-maos-baseline.md` (maos@d9bd59c4 via `git show`)

| # | Claim (short) | Note's cite | Verdict | What the source shows | Cmd |
|---|---|---|---|---|---|
| M1 | `GET /v1/daemon` returns `spirit_ids[]`, `lifecycle_states[]` (plus pid, boot_nonce, version, audit_*); no `maosctl` code calls it | `crates/maos-control/src/lib.rs:1101-1118`; `rg … crates/maos-cli/src` = 0 | CONFIRMED | Route `["v1","daemon"]` at :1101 with that JSON body. `git grep 'v1/daemon\|spirit_ids\|lifecycle_states'` in `crates/maos-cli/src` → 0 hits (exit 1). | K49, K50 |
| M2 | `maosctl spirit inspect <id>` → `GET /v1/spirits/{id}`; prints pid, boot, lifecycle, posture, ceiling | `crates/maos-cli/src/subcommands.rs:2536-2563` | **LINE-DRIFT** (range short by 4 lines) | The route and the pid/boot_nonce prints are at :2536-2564. `lifecycle_state`, `posture` and `posture_ceiling` are printed at **:2565-2567**. | K51, K53 |
| M3 | The door is request/response: `Connection: close`, 5 s read deadline, 16 workers, no streaming route | `crates/maos-control/src/lib.rs:233-253,1960` | CONFIRMED | `REQUEST_READ_DEADLINE = 5s` (:233), `MAX_BODY_BYTES = 64*1024` (:241), `MAX_WORKERS = 16` (:246), route budgets 10 s / 30 s (:249,253), `Connection: close` header (:1960). `git grep -i 'text/event-stream\|chunked\|websocket'` in maos-control → 0. | K52, K54 |
| M4 | The SCB has no parent field; its in-flight ledger records `originator_spirit_id`; no route exposes it | `crates/maos-kernel-core/src/scheduler/control_block.rs:258-287`; `:278`; `crates/maos-domain/src/ports/task.rs:12-24` | CONFIRMED | The struct fields list no parent. `pub task_assignments_in_flight: Mutex<Vec<TaskAssignmentRecord>>` at :278. `pub originator_spirit_id: String` at task.rs:23. `git grep task_assignments_in_flight` in `crates/maos-control` + `operator_door.rs` → 0. | K55, K56, K58 |
| M5 | Delegation frames are journaled with sender pid 0 (known defect, 20-3c member 17) | `_bmad-output/implementation-artifacts/sprint-status.yaml:300` | CONFIRMED (tracker) + source-corroborated | :300 records member (17). **Source check (not in the note):** `deliver_typed(frame, 0, None)` at `crates/maos-bin/src/delegation.rs:452,581,681,750`; the second parameter is `spirit_pid: u32` (`crates/maos-iac/src/adapter.rs:329-332`). The tracker's own line cites (`delegation.rs:273,384,484,553`) are **stale** at d9bd59c4. | K57, K89, K94 |
| M6 | FR20 `orchestrator queue` enqueues through the door; no production code dequeues it | `crates/maos-bin/src/operator_door.rs:1211-1261`; `rg 'drain_next\|dequeue_at_safe_point' crates/maos-bin/src` = 0 | CONFIRMED | `orchestrator_enqueue_command` → `buffer.enqueue` + `journal_orchestrator_queue` → `Completed{instruction_id,pending,capacity}`. In maos-bin the only hit is the doc comment at `orchestrator_root.rs:40`. A repo-wide check (`-- crates spirits`) finds `dequeue_at_safe_point` / `drain_next` only in the definition (`buffer.rs:62`, `spirits/orchestrator/src/lib.rs:483`), doc comments, unit tests (`lib.rs:738,756`, `buffer.rs` tests) and `tests/` files. There is no production caller. | K59, K60, K74 |
| M7 | T3 Workers refuse stdin-keeping postures, so Workers cannot be messaged | `crates/maos-bin/src/worker_spawn.rs:665-667` | CONFIRMED | `if requested_tier == SandboxTier::T3 { …reject_t3_stdin_posture(&config)?; }`. The predicate (`admission.rs:285-297`) admits only `(Raw, Signals)` / `(NdjsonOverStdio, Signals)`. | K62, K65 |
| M8 | Pausing a Worker CLI under the Signals/NamedPipe channel is a documented no-op | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:1138-1151` | CONFIRMED | `CliWrapperControlChannel::Signals \| NamedPipe => { eprintln!("…documented v0.9 no-op…"); Ok(()) }`. | K63 |
| M9 | `unload` really stops the Worker | `crates/maos-bin/src/operator_door.rs:371,590-592` | CONFIRMED | `if verb == "unload" { self.stop_worker_after_unload(pid).await; }` (:590-592). The fn is at :371. | K64 |
| M10 | Cost frames come only from the kernel Inference Port; Worker-CLI egress spend is not attributed | `crates/maos-kernel-core/src/inference/mod.rs:414-434`; `rg 'CostAttribution\|usage'` egress = 0 | CONFIRMED | `CostAttributionPayload{… attribution_source: WriteTargetProxy …}` → `insert_frame_event(FrameKind::CostAttribution, …)` at :414-434. A repo-wide `git grep FrameKind::CostAttribution -- crates spirits` finds one production emitter (`inference/mod.rs:431`); the other hits are labels, docs and tests. Egress grep → 0. | K67, K68, K73 |
| M11 | The kernel `ApprovalManager` still auto-allows per-action prompts | `crates/maos-kernel-core/src/security/approval.rs:43-75` | CONFIRMED | Doc: "v0.3-β auto-allows". `decision: true`, `Ok(true)`. | K66 |
| M12 | Hard constraints: the kernel embeds no orchestration policy and owns no UI; kernel-core ≤ 23,000 with zero headroom; currently 20,619 | `prd/functional-requirements.md:11,14`; `ADR-069…md:35-46`; `xtask/kloc.toml:264` | CONFIRMED | FR :11 "The kernel does NOT embed an orchestration policy"; :14 "…UI presentation… all Spirit-side". ADR-069 :36 "may not exceed **23,000**", :37 "**zero headroom**". `maos-kernel-core = 20619 … ZERO HEADROOM` at kloc.toml:264. | K70, K71, K90 |

Supplementary check (not counted): the status caveat that 19-1 and 19-7 are in-progress and 19-2…19-6 are backlog, at `sprint-status.yaml:287-293` — CONFIRMED (K72).

## 3. Non-CONFIRMED items and suggested wording

| Item | Note location | Verdict | What the note should say instead |
|---|---|---|---|
| O4 | `01-oh-my-pi.md` §1 bullet 7 ("The Hub is a roster plus an inspector") | LINE-DRIFT | Cite `TUI/overlays/agent-hub.ts:613-620` for `1`/`2` (Agents/Activity) next to `:2406-2588` for `t`, `/`, Enter, `r`, `x`. |
| O7 | `01-oh-my-pi.md` §1 bullet 4 ("Agents persist after they finish") | LINE-DRIFT | "A process-global `AgentRegistry` tracks each agent as `running \| idle \| parked \| aborted` (`TUI/overlays/agent-hub-types.ts:7`, imported at `CA/registry/agent-registry.ts:17`; lifecycle doc `CA/registry/agent-lifecycle.ts:1-23`)." Keep `agent-registry.ts:1-10` for the idle/parked retention sentence. |
| C7 | `02-codex.md` §1 item 6 | PARTIAL | "It maps to **four** triage buckets — NeedsYou ('Needs input ●', or 'Error !' for a root `SystemError`), Working ●, Ready ○, Finished ('Inactive ○') — ordered by the enum's derived `Ord` (`agents_overview_view.rs:69-75,78-92,434-446`). A root error row sorts with Needs input." Lesson 1 in §6 is unaffected. |
| M2 | `04-maos-baseline.md` §1 bullet 2 and §2.1 "Inspect one agent" | LINE-DRIFT | Cite `crates/maos-cli/src/subcommands.rs:2536-2567`. The lifecycle, posture and ceiling prints are at :2565-2567. |

Also worth fixing (verdict CONFIRMED, but the note leans on a non-source cite): M5 in `04-maos-baseline.md` §1 bullet 3 and §5.2 should add the source evidence `crates/maos-bin/src/delegation.rs:452,581,681,750` (`deliver_typed(frame, 0, None)`). The line numbers that `sprint-status.yaml:300` itself quotes (273, 384, 484, 553) no longer match at d9bd59c4.

## 4. Claims stated as fact that are documented-only, or verified only by reading

"Documented-only" here means the note's only evidence is a doc page or a planning/tracker document, not code or a run. Where I found source support during this check, I say so.

### 4.1 Cross-cutting: no claim in any note was runtime-verified

- 01 §7.3, 02 header ("No claim in this file was checked against a running binary"), 03 §3 ("All flows are documented behaviour") and 04 header ("I did not build or run any binary") each say this about themselves.
- So every UX flow (omp ←← → Hub → Enter → steer; codex F1–F6; maos Flows A–H) is a reconstruction from code, golden snapshots or vendor docs.
- A synthesis must not describe these flows as "observed".
- One exception for omp: the "331 parked peer(s)" roster count and the subagent JSONL header in 01 §7.4 are host observations.
- A known risk for omp: the installed binary is `omp/18.13.1`, the studied source is 18.13.2 (01 §5.4.4).

### 4.2 `01-oh-my-pi.md`: claims resting only on `docs/`

| Claim | Note cite | Status after this check |
|---|---|---|
| Discovery precedence (project → user → extension → marketplace → bundled; `.claude/agents` and similar skipped) | `docs/task-agent-discovery.md:146-180,207-220` only | Documented-only. Not checked in source. |
| §5.3 "Plan mode = read-only children — **Yes**, tool list restricted" | `docs/task-agent-discovery.md:332-341` only | Source corroborates: `PLAN_MODE_TOOLS = ["read","grep","glob","web_search"]` at `CA/task/structured-subagent.ts:227`, applied at :260 (K97). The note should cite this. |
| `wait` safety cap 30 min | `docs/tools/wait.md:10-23` only | Source corroborates: `WAIT_MAX_MS = 30 * 60_000` at `CA/tools/wait.ts:24` (K92). |
| Children never receive `AGENTS.md` | `docs/tools/task.md:201` only | Documented-only. |
| `--restore` restarts subagents interrupted by a crash | `docs/cli-reference.md:86-88` | Documented-only. The note's own open question 3 says `CA/task/restore.ts` was not traced. |
| Inline result cap 11,500 chars / 5,000-char preview | `docs/tools/task.md:104-109` plus `CA/async/job-manager.ts:9-43` | The character caps come from docs; job-manager covers eviction timings. Not checked here. |
| "User `tools.approval` policies still apply" to yolo children | `CA/task/executor.ts:1181-1184` | Only a **code comment** (K16). No enforcement path was traced. |

### 4.3 `02-codex.md`

- "The official docs agree … command center" (§1 item 1) and the `/subagents` docs-anchor naming (§5): docs URL only. They are presented as corroboration, so this is acceptable.
- `codex app-server --listen stdio://|ws://|unix://` in §2c: cited to the docs URL next to code. The transport list is source-backed by `transport/mod.rs`, but the CLI flag syntax is docs-only.
- "Hide is not persisted across TUI restarts" and "Desktop/IDE use the protocol" are already tagged `[INFERENCE]`.

### 4.4 `03-landscape.md`

- Every product claim is **vendor-documented only** (no product was installed or launched). Twelve were re-fetched and still match the live pages.
- §1 item 1 states as fact that the operator's two UX facts "are now an industry pattern". That rests on vendor docs only. The note does tag the codex/omp mirroring as `[INFERENCE]`.
- Claims already tagged `[SECONDARY]` (Jules quotas, Devin sidebar indentation, OpenHands parallel `DelegateTool`, Amp Oracle) remain unverified on a primary page. I did not re-check them.
- Docs are moving targets: CC-SUB now also says ultracode sessions are exempt from the concurrency limit (sub-agents.md l.1082), and the note does not mention it.

### 4.5 `04-maos-baseline.md`

- **Pid-0 attribution** (M5): the note cites only the tracker. It is source-confirmed here, and the tracker's own line numbers are stale (§3).
- **Product-intent items** (architecture §6.5 Observer "live view", J1 morning digest, 2026-07-17 `maos task submit/watch/…` analysis): planning documents. The note presents them as intent, which is correct. They describe no shipped behaviour.
- **"Expected observations"** in §7.3 (for example, `/v1/daemon` lists `orchestrator`, `architect`, `reviewer`, `worker`): predicted from code, never run. The note says so.

## 5. Replication notes

- Re-run any omp row with `cd /home/lunarpulse/research-src/oh-my-pi && <cmd>`, and any codex row with `cd /home/lunarpulse/research-src/codex && <cmd>`. maos commands are self-contained (`git -C … show d9bd59c4:…`). URL commands need network access only.
- Two of my own greps (K61, K69) used the pathspecs `'crates/*/src' 'spirits/*/src'`. Those pathspecs do not recurse into files, so the zero-hit results were **invalid**. I re-ran both as K73 and K74 with `-- crates spirits`, and the verdicts use the re-runs only.
- Exit code 1 from `grep` / `git grep` means zero matches. Where a row cites an exit-1 command, that absence is the evidence.

## 6. Command log (verbatim; cwd; exit code)

omp cwd = `/home/lunarpulse/research-src/oh-my-pi`; codex cwd = `/home/lunarpulse/research-src/codex`; maos cwd = `/home/lunarpulse/dev_ws`; URL commands ran with no particular cwd.

| Id | cwd | Exit | Command |
|---|---|---|---|
| K0 | omp | 0 | `sed -n '269,274p;763,782p;815,822p;917,937p' packages/coding-agent/src/modes/controllers/input-controller.ts` |
| K1 | omp | 0 | `sed -n '1709,1718p' packages/tui/src/prompt/custom-editor.ts` |
| K2 | omp | 0 | `sed -n '2552,2580p' packages/coding-agent/src/modes/controllers/selector-controller.ts` |
| K3 | omp | 0 | `sed -n '212,222p' packages/tui/src/app-keybindings.ts` |
| K4 | omp | 0 | `grep -nE 'matches\(\|case "\|=== "(t\|/\|1\|2\|r\|x)"\|"enter"\|#killSelected\|#reviveSelected\|focusAgent' packages/tui/src/overlays/agent-hub.ts \| awk -F: '$1>=2400 && $1<=2595'` |
| K5 | omp | 0 | `sed -n '100,122p' packages/coding-agent/src/modes/controllers/session-focus-controller.ts` |
| K6 | omp | 0 | `grep -rn 'focusParentSession' packages/coding-agent/src packages/coding-agent/test` |
| K7 | omp | 0 | `sed -n '950,975p' packages/coding-agent/test/input-controller-escape.test.ts` |
| K8 | omp | 0 | `sed -n '588,602p' packages/coding-agent/src/modes/controllers/input-controller.ts` |
| K9 | omp | 0 | `sed -n '611,622p' packages/tui/src/overlays/agent-hub.ts` |
| K10 | omp | 0 | `sed -n '22p;78p' docs/agent-hub.md` |
| K11 | omp | 0 | `sed -n '293,300p;329,336p;382,388p' packages/coding-agent/src/task/settings.ts` |
| K12 | omp | 0 | `sed -n '1,12p;64,96p' packages/coding-agent/src/registry/agent-registry.ts` |
| K13 | omp | 0 | `grep -n 'tombstone' packages/coding-agent/src/registry/agent-tombstone.ts \| head -5` |
| K14 | omp | 1 | `grep -n 'export type AgentStatus' packages/coding-agent/src/registry/agent-registry.ts` |
| K15 | omp | 1 | `grep -n -A1 'export type AgentStatus' packages/coding-agent/src/registry/agent-registry.ts` |
| K16 | omp | 0 | `sed -n '1173,1200p' packages/coding-agent/src/task/executor.ts` |
| K17 | omp | 0 | `grep -n 'MAX_YIELD_RETRIES' packages/coding-agent/src/task/executor.ts` |
| K18 | omp | 0 | `sed -n '149,186p' packages/coding-agent/src/modes/rpc/wire/commands.ts` |
| K19 | omp | 0 | `grep -rn 'type AgentStatus' packages/coding-agent/src/registry/` |
| K20 | omp | 0 | `sed -n '2408,2414p' packages/coding-agent/src/task/executor.ts` |
| K21 | omp | 0 | `sed -n '232,236p;262,300p' packages/coding-agent/src/task/index.ts` |
| K22 | omp | 0 | `sed -n '659,673p' packages/coding-agent/src/task/index.ts` |
| K23 | omp | 0 | `sed -n '28,32p' packages/coding-agent/src/irc/bus.ts` |
| K24 | omp | 0 | `grep -n -A2 'export type AgentStatus' packages/tui/src/overlays/agent-hub-types.ts` |
| K25 | omp | 0 | `sed -n '1,23p' packages/coding-agent/src/registry/agent-lifecycle.ts` |
| K26 | omp | 0 | `sed -n '32,48p;66,80p' packages/coding-agent/src/modes/controllers/session-focus-controller.ts` |
| K27 | omp | 0 | `sed -n '25,31p' packages/coding-agent/src/task/settings.ts` |
| K28 | omp | 0 | `sed -n '2505,2511p' packages/tui/src/overlays/agent-hub.ts` |
| K29 | codex | 0 | `sed -n '146,149p' codex-rs/cli/src/main.rs` |
| K30 | codex | 0 | `sed -n '148,153p' codex-rs/tui/src/app/agent_center/render.rs` |
| K31 | codex | 0 | `sed -n '1,4p' codex-rs/tui/src/app/agents_overview_view.rs` |
| K32 | codex | 0 | `sed -n '9,40p' codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs` |
| K33 | codex | 0 | `cat codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__chat_composer__tests__agents_navigation_enabled.snap` |
| K34 | codex | 0 | `sed -n '1971,1988p' codex-rs/tui/src/keymap.rs` |
| K35 | codex | 0 | `sed -n '75,85p' codex-rs/tui/src/app/agents_overview_discovery.rs` |
| K36 | codex | 0 | `sed -n '1230,1244p' codex-rs/tui/src/app/agents_overview.rs` |
| K37 | codex | 0 | `sed -n '66,106p;432,447p' codex-rs/tui/src/app/agents_overview_view.rs` |
| K38 | codex | 0 | `sed -n '1699,1719p' codex-rs/app-server-protocol/src/protocol/v2/thread.rs` |
| K39 | codex | 0 | `sed -n '812,826p' codex-rs/tui/src/app/agents_overview_view.rs` |
| K40 | codex | 0 | `sed -n '2500,2510p;2536,2556p' codex-rs/cli/src/main.rs` |
| K41 | codex | 0 | `sed -n '54,74p' codex-rs/app-server-transport/src/transport/mod.rs` |
| K42 | codex | 0 | `sed -n '93,120p' codex-rs/tui/src/app/exit_summary.rs` |
| K43 | codex | 0 | `cat codex-rs/tui/src/app/tests/snapshots/codex_tui__app__tests__background_exit_tests__daemon_disconnect_exit.snap` |
| K44 | codex | 0 | `sed -n '1554,1568p' codex-rs/features/src/lib.rs` |
| K45 | codex | 0 | `sed -n '258,271p' codex-rs/core/src/config/mod.rs` |
| K46 | codex | 0 | `sed -n '565,576p;669,682p' codex-rs/tui/src/bottom_pane/approval_overlay.rs` |
| K47 | codex | 0 | `sed -n '345,349p' codex-rs/core/src/tools/handlers/multi_agents_spec.rs` |
| K48 | codex | 0 | `sed -n '384,414p' codex-rs/core/src/tools/handlers/multi_agents_spec.rs` |
| K49 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-control/src/lib.rs \| sed -n '1099,1119p'` |
| K50 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -nE 'v1/daemon\|spirit_ids\|lifecycle_states' d9bd59c4 -- crates/maos-cli/src` |
| K51 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-cli/src/subcommands.rs \| sed -n '2534,2564p'` |
| K52 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-control/src/lib.rs \| sed -n '231,254p;1958,1962p'` |
| K53 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-cli/src/subcommands.rs \| sed -n '2564,2572p'` |
| K54 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -nEi 'text/event-stream\|chunked\|websocket' d9bd59c4 -- crates/maos-control/src` |
| K55 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-kernel-core/src/scheduler/control_block.rs \| sed -n '256,288p'` |
| K56 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-domain/src/ports/task.rs \| sed -n '10,25p'` |
| K57 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:_bmad-output/implementation-artifacts/sprint-status.yaml \| sed -n '300p'` |
| K58 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -n 'task_assignments_in_flight' d9bd59c4 -- crates/maos-control crates/maos-bin/src/operator_door.rs` |
| K59 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-bin/src/operator_door.rs \| sed -n '1211,1261p'` |
| K60 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -nE 'drain_next\|dequeue_at_safe_point\|judge_ambiguity' d9bd59c4 -- crates/maos-bin/src` |
| K61 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -nE 'drain_next\|dequeue_at_safe_point' d9bd59c4 -- 'crates/*/src' 'spirits/*/src'` — **invalid pathspec (no recursion); superseded by K74** |
| K62 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-bin/src/worker_spawn.rs \| sed -n '660,668p'` |
| K63 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs \| sed -n '1136,1152p'` |
| K64 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-bin/src/operator_door.rs \| sed -n '369,372p;583,593p'` |
| K65 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -n -A12 'pub fn reject_t3_stdin_posture' d9bd59c4 -- crates/maos-kernel-core/src` |
| K66 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-kernel-core/src/security/approval.rs \| sed -n '43,75p'` |
| K67 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:crates/maos-kernel-core/src/inference/mod.rs \| sed -n '414,434p'` |
| K68 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -nE 'CostAttribution\|usage' d9bd59c4 -- crates/maos-bin/src/egress.rs crates/maos-egress/src` |
| K69 | maos | 1 | `git -C /home/lunarpulse/dev_ws grep -nE 'FrameKind::CostAttribution' d9bd59c4 -- 'crates/*/src' 'spirits/*/src'` — **invalid pathspec; superseded by K73** |
| K70 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:docs/adr/ADR-069-kernel-core-envelope-through-v1.md \| sed -n '35,46p'` |
| K71 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:_bmad-output/planning-artifacts/prd/functional-requirements.md \| sed -n '11p;14p'` |
| K72 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:_bmad-output/implementation-artifacts/sprint-status.yaml \| sed -n '287,293p'` |
| K73 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -nE 'FrameKind::CostAttribution' d9bd59c4 -- crates spirits` |
| K74 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -nE 'drain_next\|dequeue_at_safe_point' d9bd59c4 -- crates spirits` |
| K75 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-view.md \| grep -n -i -E 'claude agents\|on an empty prompt\|for agents\|research preview' \| head -20` |
| K76 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-view.md \| grep -n -E 'waitingFor\|"state"\|working.*blocked\|blocked.*done' \| head -12` |
| K77 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-view.md \| grep -n -i -E 'supervisor\|state.json\|roster.json\|an hour\|1 hour\|idle' \| head -20` |
| K78 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-view.md \| grep -n -E 'roster\|jobs/' \| head -10` |
| K79 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-view.md \| grep -n -E 'Needs input\|Ready for review\|Completed\|Working' \| head -12` |
| K80 | — | 0 | `curl -sL https://code.claude.com/docs/en/sub-agents.md \| grep -n -i -E 'as a tree in the subagent panel\|path back to\|Concurrent subagent limit reached\|MAX_CONCURRENT_SUBAGENTS\|SPAWN_DEPTH\|three layers\|20 ' \| head -15` |
| K81 | — | 0 | `curl -sL 'https://docs.github.com/api/article/body?pathname=/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents' \| grep -n -i -E 'current tool call\|Stop session\|archive' \| head -8` |
| K82 | — | 0 | `curl -sL https://www.anthropic.com/engineering/multi-agent-research-system \| grep -o -E '.{0,80}(90\.2%\|explains 80%\|15× more tokens\|15x more tokens).{0,60}' \| head -6` |
| K83 | — | 0 | `curl -sL https://cognition.com/blog/multi-agents-working \| grep -o -i -E '.{0,100}(single-threaded\|2 bugs per PR\|58%\|mostly a distraction\|map-reduce-and-manage).{0,80}' \| head -8` |
| K84 | — | 0 | `curl -sL https://cursor.com/blog/scaling-agents \| grep -o -i -E '.{0,80}(two or three\|risk-averse).{0,60}' \| head -6` |
| K85 | — | 0 | `curl -sL https://docs.devin.ai/work-with-devin/dynamic-workflows \| grep -o -i -E '.{0,100}(keyed by a hash\|replay\|7 days).{0,80}' \| sort -u \| head -8` |
| K86 | — | 0 | `curl -sL https://docs.devin.ai/work-with-devin/advanced-capabilities \| grep -o -i -E '.{0,100}(Auto-approve child sessions\|without asking\|own isolated VM\|woken).{0,80}' \| sort -u \| head -8` |
| K87 | — | 0 | `curl -sL https://jules.google/docs/review-plan/ \| grep -o -i -E '.{0,120}auto-approve.{0,80}' \| sort -u \| head -4` |
| K88 | — | 0 | `curl -sL https://code.claude.com/docs/en/agent-teams.md \| grep -n -i -E "can't approve a permission prompt\|file locking\|inboxes\|doesn't ask you to confirm" \| head -8` |
| K89 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -n 'deliver_typed(' d9bd59c4 -- crates/maos-bin/src/delegation.rs` |
| K90 | maos | 0 | `git -C /home/lunarpulse/dev_ws show d9bd59c4:xtask/kloc.toml \| sed -n '262,266p'` |
| K91 | omp | 0 | `grep -rn -i 'plan mode\|planMode\|isPlanMode' packages/coding-agent/src/task/*.ts \| head -8` |
| K92 | omp | 0 | `grep -rn -E '30 \* 60\|1_800_000\|1800000\|MAX_WAIT' packages/coding-agent/src/tools/wait*.ts packages/coding-agent/src/tools/*wait* 2>/dev/null \| head -5` |
| K93 | omp | 0 | `sed -n '330,342p' docs/task-agent-discovery.md` |
| K94 | maos | 0 | `git -C /home/lunarpulse/dev_ws grep -n -A4 'pub async fn deliver_typed' d9bd59c4 -- crates` |
| K95 | omp | 0 | `grep -rn 'resolveEffectiveSubagentPolicy' packages/coding-agent/src \| head -3` |
| K96 | omp | 0 | `grep -rn -A12 'export function resolveEffectiveSubagentPolicy' packages/coding-agent/src \| grep -n -E 'read\|grep\|glob\|web_search\|spawns' \| head -8` (no output: the function is `async`, so the anchor did not match; superseded by K97) |
| K97 | omp | 0 | `grep -n -E 'resolveEffectiveSubagentPolicy\|PLAN_MODE_.*TOOLS\|"web_search"' packages/coding-agent/src/task/structured-subagent.ts \| head -10` |

In the table cells, `\|` stands for a literal `|` in the command. The commands were run as plain shell strings through `subprocess.run(…, shell=True, executable='/bin/bash')` from an eval cell; the exit codes are the ones it returned.
