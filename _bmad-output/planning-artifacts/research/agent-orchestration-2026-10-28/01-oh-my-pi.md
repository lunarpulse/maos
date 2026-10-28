# oh-my-pi (omp): subagent orchestration and the operator's view of it

Study date: 2026-10-28 · Author: ResearchOmp (subagent) · Input to the BMad technical-research report on agent orchestration for maos.

## Sources and citation legend

| Tag | Meaning |
| --- | --- |
| `omp@9d16dff` | `https://github.com/can1357/oh-my-pi` at commit `9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1` (commit date 2026-10-28 12:20 +0100, "Merge PR #17243"), checked out at `/home/lunarpulse/research-src/oh-my-pi`. `packages/coding-agent/package.json` reports version `18.13.2`. |
| `CA/…` | Short for `packages/coding-agent/src/…` inside `omp@9d16dff` |
| `TUI/…` | Short for `packages/tui/src/…` inside `omp@9d16dff` |
| `docs/…` | `docs/…` inside `omp@9d16dff`. The same files ship in the installed harness as `omp://<name>.md`; `read omp://` lists 134 files. |
| `host:…` | Observed on this workstation on 2026-10-28. The installed CLI is `/opt/agent-clis/bin/omp`, a single 275,986,608-byte executable; `omp --version` prints `omp/18.13.1`. That is one patch release behind the studied source, so the source governs. |
| `[INFERENCE]` | My interpretation. The cited source does not state it outright. |

Line numbers are for `omp@9d16dff`. Where code and docs disagree, both are reported (see §5.4).

---

## 1. Summary

- **Agents come from one `task` tool.** The model fills in a batch `{context, tasks[]}`. Each item becomes one child `AgentSession` with its own JSONL transcript. With `async.enabled` on (the default), each child runs as a background job. The call returns agent ids at once, and results are later injected into the parent as `<system-notice>` messages (`docs/tools/task.md:3,74-78`; `CA/tools/settings.ts:950-961`; `CA/prompts/tools/async-result.md:1-17`).
- **Agent types are Markdown files with YAML frontmatter.** The frontmatter keys include `name`, `description`, `tools`, `spawns`, `model`, `thinking-level`, `output` (a JTD-style schema), `blocking`, `structured`, `prewalk` and `advisor`. Bundled agents are `scout`, `reviewer`, `security-reviewer`, `seance`, `task` and `sonic`. Custom agents live in `.omp/agents/*.md` (project) and `~/.omp/agent/agents/*.md` (user), and the first definition found for a name wins (`docs/task-agent-discovery.md:25-49,129-180`; `CA/task/agents.ts:45-71`).
- **Limits are enforced by code, not by prompts.** These include a per-session semaphore (`task.maxConcurrency` = 32), a spawn-depth cap (`task.maxRecursionDepth` = 2), per-agent spawn allow-lists, a soft request budget (200 requests, force-stopped at 1.5×), an optional wall-clock limit, and up to 3 forced `yield` reminders (`CA/task/settings.ts:295-399`; `CA/task/executor.ts:2412,2587-2598,4297-4327,1783-1788`).
- **Agents persist after they finish.** A process-global `AgentRegistry` tracks each agent as `running | idle | parked | aborted`. Finished agents stay `idle`. After `task.agentIdleTtlMs` (7 minutes) they become `parked`: the session is disposed but the transcript is kept. Any message, Hub action or click revives them from the JSONL. A kill writes a `.tombstone` marker, and the agent can still be revived (the four-state union is defined at `TUI/overlays/agent-hub-types.ts:7` and imported at `CA/registry/agent-registry.ts:17`; idle/parked retention at `CA/registry/agent-registry.ts:1-10`; `CA/registry/agent-lifecycle.ts:1-23,42-60`).
- **Agents talk to each other through files-as-URLs.** `write agent://<id>` sends a message through an IRC-style mailbox bus, and `agent://all` broadcasts. `read agent://<id>` returns the final output, `history://<id>` the transcript, `proc://` the jobs, `artifact://` spilled output, and `local://` shared scratch space (`CA/irc/bus.ts:1-31`; `CA/internal-urls/*-protocol.ts`).
- **The operator's "←← at the parent" gesture is real.** On an empty main-session editor, two ← presses 40–500 ms apart open the Agent Hub overlay. It only opens if there is at least one live or persisted subagent. The handlers are `CA/modes/controllers/input-controller.ts:765-781` and `:919-936`, with the editor hook at `TUI/prompt/custom-editor.ts:1711-1717`. The same Hub is bound to `Alt+A` and `Ctrl+S` (`TUI/app-keybindings.ts:214-221`).
- **The Hub is a roster plus an inspector.** Each row shows status, id, parent, unread IRC count, role/model, age, task or activity, cost, time, request and tool-call counts, and tokens. The inspector shows current tool, intent, retry state, a context gauge, lineage, and output/patch/branch artifacts. Keys: `t` switches between flat list and tree, `/` filters, `1`/`2` switch between Agents and Activity log (`TUI/overlays/agent-hub.ts:613-620`), `r` revives, `x` kills, `Enter` opens an agent (`TUI/overlays/agent-hub.ts:2406-2588`; `docs/agent-hub.md:24-69`).
- **Opening an agent re-points the whole main TUI at it.** Transcript, status line and editor all switch to the child session, so the operator can read it live and type to steer it. Model and effort switches apply to that child only. `Esc` or ←← returns to the main session (`CA/modes/controllers/session-focus-controller.ts:32-112`).
- **Observability is layered.** There is a pinned "Subagents · N running" block above the editor that also works as a click target list, a status-line agent count plus total cost of all subagents, per-agent cost/tokens/requests, and periodic self-reported completion estimates (`{"pct","stage","watch"}`) for direct children (`CA/modes/interactive-mode.ts:1197-1321,4378-4395`; `CA/task/completion-probe.ts:1-47`).
- **There is a headless control API.** RPC mode exposes `get_subagents`, `get_subagent_messages`, `steer_subagent`, `cancel_subagent` and `set_subagent_subscription` (`CA/modes/rpc/wire/commands.ts:149-185`; `docs/rpc.md:94,158-163`). This is the closest analogue to a `maosctl` door.
- **Orchestration rules live in prompt templates and are partly enforced.** Examples: "map slices before spawning", "fan out in one batch", "no builds/tests mid-flight; main agent verifies once", "NEVER poll". Shape errors (top-level `agent` in a batch, duplicate names, missing `context`) are rejected by code. Behavioural advice such as "verify once" and "don't poll" is prompt-only (`CA/prompts/system/system-prompt.md:147-209`; `CA/prompts/tools/task.md:11-30`; `CA/task/index.ts:234,262-300`).
- **Docs and code disagree in two places.** The focus status line says ←← "hops to parent", but the code and tests return to *main*. The follow-up template says isolated runs "cannot be resumed", but the docs describe a retained-workspace revive path (§5.4).

---

## 2. Capability inventory

| Capability | Mechanism | User-facing UX | Evidence | Replication |
| --- | --- | --- | --- | --- |
| Spawn subagents | `task` tool. Batch shape `{context, tasks[]}` (`task.batch` default true). Each item is `{name?, agent?, task, solutionSpace, …}`. `context` is rendered into every child's system prompt (`§ Context`). | The parent turn shows one task block that turns into live rows. With async on, the tool returns `Spawned N background agents …` with ids immediately. | `docs/tools/task.md:33-60,74-78`; `CA/task/settings.ts:241-253`; `CA/prompts/system/subagent-system-prompt.md:1-7` | `rg -n 'id: "task.batch"' packages/coding-agent/src/task/settings.ts` |
| Input validation | `validateSpawnParams` rejects an empty `tasks`, a top-level `task` or `agent` in batch shape, duplicate names (case-insensitive), and a missing `context`. | Tool error text is returned to the model. | `CA/task/index.ts:234,262-300` | `rg -n 'Duplicate task name\|Top-level .agent' packages/coding-agent/src/task/index.ts` |
| Speculative launch | While the batch call is still streaming, each `tasks[]` item starts as soon as its JSON object closes. If the final call differs, the speculative runs are aborted. | Children appear before the parent's tool call finishes streaming. | `CA/task/index.ts:688-700`; `docs/tools/task.md:137`; `CA/task/settings.ts:269-271` | `rg -n speculativeLaunch packages/coding-agent/src/task` |
| Agent definitions | Markdown + YAML frontmatter, parsed by `parseAgentFields`. Bundled ones are embedded at build time. `main` and `sub` are reserved names. | Write `.omp/agents/<name>.md` and pass `agent: "<name>"` on a task item. The `/agents` hub enables/disables agents and sets model overrides. | `CA/task/agents.ts:45-71,138-139`; `docs/task-agent-discovery.md:25-49,53-91,113`; `CA/prompts/agents/scout.md:1-25` | `sed -n 1,25p packages/coding-agent/src/prompts/agents/scout.md` |
| Discovery precedence | First definition per name wins: project `.omp/agents` → user `~/.omp/agent/agents` → extension `agents/` → Claude marketplace plugins → bundled. `.claude/agents`, `.codex/agents` and `.gemini/agents` are deliberately skipped. | Re-scanned on every spawn, so agents added mid-session work. | `docs/task-agent-discovery.md:146-180,207-220` | `host:~/.omp/agent/agents/` contains `coder.md` and `light-reviewer.md` (user agents in use on this host) |
| Model selection / roles | Model is chosen in this order: `task.agentModelOverrides[agent]`, then frontmatter `model` (list, `@role` aliases resolved through `modelRoles`), then the parent model. Bundled `task` uses `@task` with `auto` thinking; `sonic` and `scout` use `@smol`. | `Alt+P` while viewing a subagent changes only that session. `/agents` saves overrides. | `docs/task-agent-discovery.md:51-76,222-234`; `CA/task/agents.ts:51-71`; `docs/agent-hub.md:77` | `rg -n 'model: "@' packages/coding-agent/src/task/agents.ts` |
| Concurrency limit | One `Semaphore` per session (per `TaskTool`), resized from `task.maxConcurrency` (default 32, 0 = unlimited) before every acquire and release. A separate per-provider semaphore covers only the HTTP stream, so wide spawn trees cannot deadlock. | The system prompt says "Max N concurrent subagents; excess queue". Extra spawns wait in the queue. | `CA/task/index.ts:661-672`; `CA/task/settings.ts:295-298`; `CA/task/provider-concurrency.ts:1-12`; `CA/prompts/system/system-prompt.md:176-178` | `rg -n 'id: "task.maxConcurrency"' -A4 packages/coding-agent/src/task/settings.ts` |
| Depth / spawn policy | `task.maxRecursionDepth` defaults to 2. A child at the cap loses the `task` tool. Frontmatter `spawns` gives an allow-list. `PI_BLOCKED_AGENT` blocks an agent from spawning itself. Preflight error messages are explicit. | A capped child's `task` description reads "Agent spawning is currently disabled". | `CA/task/executor.ts:4297-4327`; `CA/task/types.ts:294-297`; `CA/task/structured-subagent.ts:293-307,368` | `rg -n 'Cannot spawn' packages/coding-agent/src/task/structured-subagent.ts` |
| Child settings | Each child gets an isolated settings snapshot: `tools.approvalMode: "yolo"` (no UI to approve in), advisor off, mid-turn compaction on. The parent-owned `todo` tool is removed unless prewalk is armed. | The parent's approval of the `task` call is the only approval gate. | `CA/task/executor.ts:1175-1198,4869-4872` | `rg -n '"tools.approvalMode": "yolo"' packages/coding-agent/src/task/executor.ts` |
| Isolation (opt-in) | `task.isolation.enabled` (default false) adds a per-item `isolated` flag. The workspace comes from the natives layer (`isoResolve`/`isoStart`: overlayfs, btrfs, zfs, APFS, reflink, ProjFS, rcopy). Merge is `patch` (default) or `branch` (`omp/task/<id>` then cherry-pick). The baseline is capped at 1 GiB. | The Hub inspector shows patch path and worktree branch. Artifacts include `<id>.patch`. | `CA/task/settings.ts:27-30,148-166`; `CA/task/worktree.ts:153,537,648,701`; `docs/tools/task.md:115-117,138-139` | `rg -n 'export (async )?function (parseIsolationBackend\|ensureIsolation)' packages/coding-agent/src/task/worktree.ts` |
| Structured output | `task.structured` defaults to false. With it on, or with frontmatter `structured: true`, the child gets a hidden `yield` tool. The schema comes from the item's `outputSchema`, then frontmatter `output`, then the parent session. `schemaMode` is `permissive` or `strict`. Data is written to `<id>.json`. | `agent://<id>/<field>/<index>` extracts JSON fields. Invalid payloads are previewed inline. | `docs/task-agent-discovery.md:279-289`; `docs/tools/task.md:51-52,94-96`; `CA/prompts/system/subagent-system-prompt.md:51-85` | `rg -n 'id: "task.structured"' -A3 packages/coding-agent/src/task/settings.ts` |
| Completion contract | A child must end with `yield` (structured) or a final message. If it does not, up to `MAX_YIELD_RETRIES = 3` reminders follow, and the last one forces `toolChoice = yield`. | A missing yield produces `SYSTEM WARNING: Subagent exited without calling yield …`. | `CA/task/executor.ts:2412,2580-2600`; `docs/tools/task.md:121,191` | `rg -n 'MAX_YIELD_RETRIES' packages/coding-agent/src/task/executor.ts` |
| Result delivery | `AsyncJobManager` registers one `type:"task"` job per spawn and injects an `async-result` `<system-notice>` into the owner when it settles. Inline output is capped at 11,500 characters; longer output gets a 5,000-character preview plus `agent://<id>`. Consumed rows are evicted after 30 s, unconsumed ones kept 5 min. | "Background job X has completed. Resume your work…" plus `Agent: agent://<id>, history://<id>`. | `CA/prompts/tools/async-result.md:1-17`; `CA/async/job-manager.ts:9-43`; `docs/tools/task.md:104-109,178` | `rg -n 'CONSUMED_RESULT_EVICTION_MS\|DEFAULT_RETENTION_MS' packages/coding-agent/src/async/job-manager.ts` |
| `wait` | Takes no arguments. Blocks until the first owned job settles, a peer message arrives, or a steering interrupt; safety cap 30 minutes. Running rows show `run time · req · $ · ctx · stage`. | The prompt says to use it only when completely blocked, and "NEVER poll". | `docs/tools/wait.md:10-23`; `CA/tools/wait.ts:24` (`WAIT_MAX_MS = 30 * 60_000`); `CA/prompts/tools/task-async-contract.md:1-8` | `read omp://tools/wait.md` |
| Lifecycle (idle / park / revive / kill) | `AgentLifecycleManager.adopt` arms an idle timer (`task.agentIdleTtlMs` = 420,000 ms). `park()` sets status to `parked` and then disposes the session. `ensureLive()` revives from JSONL and merges concurrent revive requests. `release(…, {tombstone:true})` writes `<transcript>.jsonl.tombstone`. | Hub `r` revives and `x` kills. Messaging or focusing a parked or killed agent revives it automatically. | `CA/registry/agent-lifecycle.ts:1-23,42-60,198,291-367,379-397,533-594`; `CA/registry/agent-tombstone.ts:1-7`; `CA/task/executor.ts:3641-3656`; `CA/task/settings.ts:384-386` | `rg -n 'async (park\|ensureLive\|release)\(' packages/coding-agent/src/registry/agent-lifecycle.ts` |
| Kill / cancel | `write proc://<id>/kill` cancels a job or an owned subagent. Hub `x` aborts the running turn, then releases with a tombstone. RPC `cancel_subagent`. `/jobs kill <id>\|all`. | `x` takes effect immediately. A killed agent shows as `aborted` and can be revived. | `TUI/overlays/agent-hub.ts:2560-2588`; `CA/internal-urls/proc-protocol.ts:39-60`; `docs/agent-hub.md:115`; `docs/tools/task.md:163` | `rg -n '#killSelected' -A28 packages/tui/src/overlays/agent-hub.ts` |
| Peer messaging (IRC) | Process-global `IrcBus`, mailbox capped at 100 messages per agent. A send never waits for a reply. Parked or killed recipients are revived, idle ones get a real turn, busy ones receive the message as an aside at the next step boundary. `to:"all"` broadcasts. Enabled for subagents, and for a top-level session that is allowed to spawn. | The recipient sees `<irc>Incoming IRC message from agent X…</irc>`. A parent message during `wait` interrupts the wait (`[Wait interrupted by message]`). | `CA/irc/bus.ts:1-31,96,154-196`; `CA/irc/messaging.ts:16-60`; `CA/prompts/system/irc-incoming.md:1-9`; `CA/prompts/steering/parent-irc.md:1-4` | This study's own session received exactly the `parent-irc.md` format from `Main` |
| Peer roster in the child prompt | The child system prompt lists `ircSelfId`, live peers with kind/status/activity, and a count of parked peers. | The child knows sibling ids at spawn time; the list can go stale. | `CA/prompts/system/subagent-system-prompt.md:27-49` | Compare to this session's own "§ Coop" block |
| Internal URLs | `agent://<id>[/json/path]` (final output, or live status and partial yields before completion). `history://[<id>]` (concise transcript, also for parked agents and from disk). `artifact://<id>`. `proc://[<id>][/kill\|/streams]`. `local://` (scratch root shared by parent and children). | The model reads and writes these with the ordinary `read`/`write` tools. | `CA/internal-urls/agent-protocol.ts:1-20`; `history-protocol.ts:1-19`; `artifact-protocol.ts:1-11`; `proc-protocol.ts:39-60`; `CA/prompts/internal-urls/proc.md:1`; `docs/tools/task.md:93-98,196` | `ls packages/coding-agent/src/internal-urls/` |
| Agent Hub (TUI) | `AgentHubOverlayComponent`: roster/tree, inspector, Activity log, in-hub transcript viewer for advisors and collab guests. Mounted full screen. | See §3. | `TUI/overlays/agent-hub.ts:1-15,336,588-623,2406-2588`; `CA/modes/controllers/selector-controller.ts:2495-2580` | Press `Alt+A` |
| ←← gesture | Two plain ← presses on an empty editor, with the gap in `[40, 500)` ms. Faster terminal bursts are ignored. On main it opens the Hub (only if non-empty). While viewing a subagent it returns to main. The opening ←← also arms a single ← to close the Hub. | Operator: "left arrow twice at the parent session". | `CA/modes/controllers/input-controller.ts:271-272,356-368,765-781,817-821,910-936`; `TUI/prompt/custom-editor.ts:1711-1717`; `TUI/overlays/agent-hub.ts:201,625-636,2460-2474` | §7.3 |
| Focus a subagent | `SessionFocusController.focusAgent(id)` calls `ensureLive(id)`, swaps the event subscription, transcript, status line and editor target, and shows a status message. If the agent parks or is killed while viewed, the view returns to main. | Type to steer, `Esc` or ←← to return. | `CA/modes/controllers/session-focus-controller.ts:32-172` | Press Enter on a Hub row |
| Pinned Subagents block | `renderSubagentHudLines`: "Subagents · N running" header with the Hub key hint, one tree row per running agent (`Id ⟨role⟩: description`), optional live tool preview, collapsed or expanded. With `tui.mouse` on, rows are clickable. | Always visible while children run (`display.pinnedAgents` default `collapsed`). | `CA/modes/interactive-mode.ts:1197-1321`; `CA/modes/settings.ts:573-586,619-648` | Spawn a task and look above the editor |
| Status-line agents segment and tree cost | Status line shows running and inspectable subagent counts with shortcut hint `←←`. `sumSubagentTreeCost` adds up descendant cost within the root session's artifact tree. | Operator sees spend for all subagents without opening the Hub. | `CA/modes/interactive-mode.ts:1596-1611,4378-4395`; `CA/modes/agent-hub-runtime.ts:57-85` | Visual check |
| Completion self-estimate | A side turn (like `/btw`, does not touch the transcript, reuses the prompt cache) runs at +2, 5, 10, 30, 60 min and every 60 min after. The child replies `{"pct","stage","watch"}`. Only for direct children of an interactive main session. | `stage` becomes the running row's label in `wait` and `proc://`. | `CA/task/completion-probe.ts:1-47`; `CA/prompts/system/subagent-completion-probe.md:17-22`; `docs/tools/wait.md:23` | `rg -n PROBE_DELAYS_MS packages/coding-agent/src/task/completion-probe.ts` |
| Session persistence | Main session: `~/.omp/agent/sessions/<encoded-cwd>/<ts>_<id>.jsonl`. Its artifact directory has the same name without `.jsonl`. Each subagent writes `<AgentId>.jsonl` and `<AgentId>.md` (plus `.json` and `.patch`) inside it. The child header's `parentSession` points at the parent JSONL. Nested children use dotted ids (`Parent.Child`). | `/resume`, `--continue`, `--restore` (also restarts subagents interrupted by a crash). When a session is resumed, the Hub lists its subagents as `parked`. | `docs/session.md:37-45,65-101`; `docs/tools/task.md:94-98,205`; `docs/cli-reference.md:86-88`; `host:` header shown in §7.4 | §7.4 |
| Compaction | Subagents force `compaction.midTurnEnabled: true`, because a child's whole assignment is one turn. `task.agentCompactionThresholdOverrides[agent]` sets per-agent thresholds. Compaction and branch summaries are stored as session entries. | Not shown to the user. | `CA/task/executor.ts:1188-1196`; `docs/task-agent-discovery.md:236-239`; `docs/compaction.md:1-8` | `read omp://compaction.md` |
| Todo tool | `todo` ops `init/start/done/drop/block/unblock/rm/append/view`, organised by phase. Owned by the parent: removed from children unless prewalk is armed. | Todos HUD in the main TUI. | `docs/tools/todo.md:1-40`; `CA/task/executor.ts:4869-4872` | `read omp://tools/todo.md` |
| RPC control | `--mode rpc` JSONL protocol: `get_subagents`, `get_subagent_messages{subagentId\|sessionFile, fromByte}`, `steer_subagent`, `cancel_subagent`, `set_subagent_subscription{off\|progress\|events}`. Frames: `subagent_lifecycle`, `subagent_progress`, `subagent_event`. | A headless host or IDE can drive the same roster. | `CA/modes/rpc/wire/commands.ts:149-185`; `docs/rpc.md:22,94,158-163` | `rg -n 'subagent' packages/coding-agent/src/modes/rpc/wire/commands.ts` |

---

## 3. UX flows

### 3.1 Parent → subagents with ←← (the flow the operator praised)

1. In the main session, the operator asks for parallel work. The model calls `task` with `{context, tasks[]}`. Under the editor's chrome a pinned **"Subagents · N running"** block appears, with one row per child (`● Id ⟨role⟩: description`) and the Hub key shown on the right (`CA/modes/interactive-mode.ts:1315-1321`). The status line gains an agents segment whose hint is the double-tap ← glyph (`CA/modes/interactive-mode.ts:1599-1611`).
2. With the **editor empty**, the operator presses **←**. The editor's key handler sees plain `left` on empty text and calls `onLeftAtStart` instead of moving the cursor (`TUI/prompt/custom-editor.ts:1711-1717`).
3. `#detectLeftDoubleTap` stores the timestamp, so the first tap does nothing (`CA/modes/controllers/input-controller.ts:919-927`).
4. The operator presses **←** again 40–499 ms later. Exactly two taps in that window count as the gesture. Bursts faster than 40 ms, or a third tap before a quiet gap, are ignored (`:271-272,928-935`; tests `test/input-controller-escape.test.ts:1186-1214`).
5. `showAgentHub({ requireContent: true, armCloseTap: true })` runs (`:777-780`). If there are no live subagents, it waits for the persisted-subagent scan (`persistedSubagentsReady`) and stays closed when nothing turns up (`CA/modes/controllers/selector-controller.ts:2554-2579`). Otherwise the Hub opens **full screen**.
6. The Hub shows the **Agents** section: a flat list ordered by operational state, each row with status glyph, id, parent, unread IRC badge, role/model badges, age, task or activity, and `$cost · time · N req · N tok` (`TUI/overlays/agent-hub.ts:1262-1284,2170-2226`; `TUI/overlays/agent-hub-renderer.ts:261-330`). On a wide terminal the **inspector** for the selected agent appears beside it, showing current tool, args, last intent, `retry n/m`, context gauge, lineage, output/patch/branch paths (`TUI/overlays/agent-hub.ts:1402-1445,2072-2131`).
7. Navigation (`TUI/overlays/agent-hub.ts:2406-2503`): `j`/`k` or arrows or wheel to select. `t` switches flat ↔ "By parent" tree. `/` filters by id or name. `Tab` toggles the inspector on narrow terminals. `PgUp`/`PgDn` scrolls the inspector. `1`/`2` switch Agents / Activity (`:613-620`).
8. **Enter** on a row calls `focusAgent(id)` and closes the Hub (`:2505-2529`). `SessionFocusController.focusAgent` revives the agent if needed (`ensureLive`) and re-attaches the transcript, event stream, status line and editor to that child (`CA/modes/controllers/session-focus-controller.ts:66-112,174-200`). A status message reads `Viewing agent <id> — Esc returns to main, ←← hops to parent` (see §5.4 on the "parent" wording).
9. The operator now sees the child's live transcript and tool cards. Typing and **Enter** steers a running turn or prompts an idle agent. The message goes through the normal prompt path, so it is saved in the child's JSONL. Model and effort pickers (`Alt+P`, `/effort`, …) apply to that child only (`docs/agent-hub.md:71-80`).
10. **Esc** on an empty editor, or **←←**, calls `unfocusSession()` and the main session's transcript is rebuilt from its saved state (`CA/modes/controllers/input-controller.ts:590-600,817-821`). Esc while focused **never interrupts** the child (`:591-593`).
11. If the Hub was opened with ←←, a **single ←** within 500 ms closes it again, because the opening gesture armed the close detector (`TUI/overlays/agent-hub.ts:201,625-636,2460-2474`; the issue reference is in `CA/modes/controllers/input-controller.ts:765-771`).

### 3.2 Manage: revive, kill, read-only advisor

1. In the Hub, select a `parked` or `aborted` (killed) row and press **r**. This calls `lifecycle.ensureLive(id)`, which reopens the JSONL. A killed agent becomes a fresh `parked` generation and its tombstone is removed (`TUI/overlays/agent-hub.ts:2531-2558`; `CA/registry/agent-lifecycle.ts:424-470`). `r` on a running or idle row shows a notice: only parked or killed agents can be revived (`:2539-2542`).
2. Press **x**. If the agent is `running`, its turn is aborted with the user-interrupt reason, then `release(id, ref, { tombstone: true })` runs (`TUI/overlays/agent-hub.ts:2560-2588`). The row stays as `aborted` until revived.
3. Advisor rows (`__advisor*.jsonl`) open only in the read-only in-hub transcript viewer. `r` and `x` are refused with a notice (`:2514-2519,2534-2537,2563-2566`; `docs/agent-hub.md:96-107`).

### 3.3 Activity log (cross-agent timeline)

1. `/hub` (alias `/subagents`) opens the Hub straight into **Activity** (`CA/slash-commands/builtin-session.ts:605-614`). Inside the Hub, press `2`.
2. Entries are responses, tool calls, IRC messages and lifecycle events. `f` cycles All/Errors/Responses/Tools. `s` cycles the scope: all agents, the selected agent, or its subtree. `/` searches. `Space` toggles following the newest entry. `Enter` jumps to that agent's transcript at that entry (`docs/agent-hub.md:61-69`; `TUI/overlays/agent-hub.ts:2304-2400`).

### 3.4 Click-to-focus without the Hub (opt-in)

1. Set `tui.mouse: true` (default false; Shift+drag is then needed to select text) (`CA/modes/settings.ts:573-586`).
2. Hover over a live task card or a pinned-block row to highlight it, then click to focus that exact agent. Parked agents revive. Clicking the expander row toggles collapsed/full (`CA/modes/controllers/input-controller.ts:823-908`).

### 3.5 The model-side loop (what the orchestrating agent "sees")

1. It calls `task` and receives ids at once (`docs/tools/task.md:76`).
2. It keeps working. Each child's result arrives as `<system-notice>Background job X has completed…</system-notice>` with `agent://` and `history://` pointers and schema status (`CA/prompts/tools/async-result.md`).
3. If blocked, it calls `wait`, which returns on the first settled job or message (`docs/tools/wait.md:13-20`).
4. It can coordinate with `write agent://<id>` (steer or follow up; revives parked agents), check status with `read proc://`, cancel with `write proc://<id>/kill`, and look back with `read history://<id>`.

---

## 4. Techniques and patterns

1. **Everything is an addressable id: one registry, many views.** One process-global `AgentRegistry` (`CA/registry/agent-registry.ts:1-10`) feeds the Hub, the pinned block, the status line, `history://`, `agent://`, IRC routing and RPC `get_subagents`. Ids are human-readable (`name` field, or a generated AdjectiveNoun), unique within the session, and dotted for nesting (`docs/tools/task.md:44,205`). The same name is used by the model (`agent://Foo`), the operator (Hub row "Foo") and on disk (`Foo.jsonl`).
2. **Keep finished agents alive instead of tearing them down.** Success and failure both leave the agent `idle` with its live session. A TTL then parks it, which keeps the transcript and drops the memory. Any message revives it (`docs/tools/task.md:122-128`; `CA/task/executor.ts:3641-3656`). This makes "ask the same agent a follow-up" cheaper than spawning again, and the prompt recommends it (`docs/tools/task.md:197`).
3. **Kill leaves a tombstone, and the agent can still come back.** An explicit kill writes a `.tombstone` file next to the transcript, so later scans show `aborted` rather than `parked`. Reviving removes it (`CA/registry/agent-lifecycle.ts:42-60,424-470,524-594`).
4. **Persisted roster scan.** When a session is resumed, the Hub scans the session's artifact tree, turns child JSONLs into `parked` rows and keeps lineage (`docs/agent-hub.md:96-98`; `CA/registry/persisted-agents.ts`). The UI never depends only on in-memory state.
5. **Focus means re-pointing the main view, not opening a second viewer.** The Hub does not render its own copy of a local agent's transcript. It re-points the main transcript renderer at the child session, which the source calls "exact parity by construction" (`TUI/overlays/agent-hub.ts:2505-2510`; `CA/modes/controllers/session-focus-controller.ts:32-41`). A separate tail-the-file viewer exists only where no local session exists (collab guests, advisors).
6. **Generation counters guard against stale async work.** `#focusRequestSeq` and `#attachGeneration` make sure a slow revive cannot replace a newer focus or an Esc (`CA/modes/controllers/session-focus-controller.ts:49-112,124-131`). The lifecycle manager binds every park and revive to the exact `AgentRef` it started from (`CA/registry/agent-lifecycle.ts:20-23`).
7. **Asynchronous mailbox messaging.** `send` never waits for a reply. Delivery depends on the recipient's state: revive, wake, or aside at the next step boundary. The bus records unanswered direct sends so `wait` knows whom it is waiting for (`CA/irc/bus.ts:1-10,48-68,140-152`; `docs/tools/wait.md:17`).
8. **Results are pushed, not polled.** Job settlement injects a system notice into the owner. `wait` exists only for "completely blocked", and the prompt says "NEVER poll" (`CA/prompts/tools/task-async-contract.md:1`; `CA/prompts/system/system-prompt.md:167`).
9. **Batch plus shared context.** One call carries the goal and contract (`context`) once and gives each item a `# Target / # Change / # Acceptance` brief. `solutionSpace` is passed separately to the child's thinking-effort classifier (`CA/prompts/tools/task.md:17-30`; `docs/tools/task.md:49`).
10. **Tests guard against UI misfires.** The double-tap needs a human-plausible interval, so terminal-generated arrow bursts cannot open the Hub. Tests cover this (`CA/modes/controllers/input-controller.ts:910-936`; `test/input-controller-escape.test.ts:1171-1226`).
11. **Self-reported progress from a cache-reusing side turn.** Completion probes run as a side turn that does not touch the transcript and uses an increasing schedule. Their cost is reported with each estimate (`CA/task/completion-probe.ts:1-55`).
12. **Cost is attributed down the tree.** Each agent's direct cost is kept in `history.directCost` (`CA/registry/agent-registry.ts:53-54`). The status line adds up descendants within the root's artifact tree (`CA/modes/agent-hub-runtime.ts:57-85`). The Hub shows "usage —" rather than an estimate when data is missing (`docs/agent-hub.md:42`).
13. **The parent's approval is the authority boundary.** Children run headless with `approvalMode: yolo`, but user `tools.approval` policies still apply (`CA/task/executor.ts:1181-1184`). Plan mode narrows children to read-only tools and turns off IRC, isolation and eval tools (`docs/task-agent-discovery.md:332-341`; `PLAN_MODE_TOOLS` is defined at `CA/task/structured-subagent.ts:227` and applied at `:260`).
14. **Prompt rules for delegation, partly backed by code.** See §5.3 for the split between advice and enforcement.

---

## 5. Limits, risks, costs

### 5.1 Hard limits (defaults, from code)

| Limit | Default | Evidence |
| --- | --- | --- |
| Concurrent subagents per session | 32 (`0` = unlimited) | `CA/task/settings.ts:295-298` |
| Spawn depth | 2 (negative = no cap) | `CA/task/settings.ts:331-334`; `CA/task/types.ts:296-297` |
| Idle → parked TTL | 420,000 ms (7 min) | `CA/task/settings.ts:384-386` |
| Soft request budget | 200 requests; force stop at 1.5×; scout/sonic capped at 100 | `CA/task/settings.ts:397-399`; `CA/task/executor.ts:171,211,1783-1788`; `docs/tools/task.md:175` |
| Wall-clock per spawn | 0 (off) | `CA/task/settings.ts:351-353` |
| Yield reminders | 3, last one forced | `CA/task/executor.ts:2412,2587-2598` |
| Async jobs | `async.maxJobs` 100 | `CA/tools/settings.ts:963-968` |
| IRC mailbox | 100 per agent, oldest dropped | `CA/irc/bus.ts:30-31,337` |
| Per-subagent output kept in memory | 500,000 bytes / 5,000 lines (full output in `<id>.md`) | `CA/task/types.ts:29,32` |
| Isolation baseline | 1 GiB | `CA/task/worktree.ts:153` |

### 5.2 Risks and costs

- **Children start without the conversation history.** They get only `context`, `task`, the plan reference and workspace context files. Any file named `AGENTS.md` is deliberately not passed down (`docs/tools/task.md:201`). A poor brief means a poor child [INFERENCE].
- **Shared checkout by default.** Isolation is off (`CA/task/settings.ts:27-30`), so sibling edits can collide. The mitigation is advisory only: "Shared edits need one integration owner; siblings coordinate via `write agent://<id>`" (`CA/prompts/tools/task.md:15`).
- **Headless children run in yolo mode.** Once the parent's `task` call is approved, children's tool calls are not individually confirmed (`CA/task/executor.ts:1181-1184`).
- **Staleness.** The peer roster in a child prompt is "when this list was written" (`CA/prompts/system/subagent-system-prompt.md:29,45`). Delivered job rows expire after about 30 s (`CA/async/job-manager.ts:24`).
- **Scale and clutter.** The registry keeps finished agents until release. This very session's prompt reported "331 parked peer(s) omitted" (observed in this agent's system prompt, 2026-10-28). The roster prompt and Hub must collapse such counts. The Hub's flat/tree toggle and filter exist for this reason [INFERENCE].
- **Cost.** Every spawn is a full model session. Completion probes add side requests (only for direct children in the interactive TUI) (`CA/task/completion-probe.ts:39-47`). Advisors, if enabled per agent, add more.
- **Single process.** Registry, bus and lifecycle manager are process-global singletons (`CA/irc/bus.ts:33-41`; `CA/registry/agent-registry.ts:1-3`). Cross-process access goes only through JSONL files and leases (`docs/session.md:546-547`) [INFERENCE: no network peer bus was found; searched `CA/irc/`, `CA/registry/`].

### 5.3 Enforced by code vs advised by prompt

| Rule | Where stated | Enforced? |
| --- | --- | --- |
| No top-level `agent` in a batch; unique names; `context` required | `CA/prompts/tools/task.md:13,18,29` | **Yes**: `CA/task/index.ts:234,262-300` |
| Depth cap / spawn allow-list / no self-recursion / disabled agents | `docs/task-agent-discovery.md:304-330` | **Yes**: `CA/task/structured-subagent.ts:293-307,368`; `CA/task/executor.ts:4312-4327` |
| "Max N concurrent subagents; excess queue" | `CA/prompts/system/system-prompt.md:176-178` | **Yes**: semaphore at `CA/task/index.ts:661-672` |
| Child must return via `yield` / final message | `CA/prompts/system/subagent-system-prompt.md:51-97` | **Partly**: 3 reminders and a forced toolChoice (`CA/task/executor.ts:2412,2580-2600`), then a warning |
| Soft request budget | Budget notice text `CA/task/executor.ts:211` | **Yes** at 1.5× (`:1783-1788`) |
| "Every task MUST skip build/lint/tests/formatters mid-flight; run once afterward" | `CA/prompts/tools/task.md:15`; `CA/prompts/system/system-prompt.md:205-209` | **No**: prompt only (no tool gate found; searched `CA/task/` for build/test gating) |
| "NEVER poll"; use `wait` only when blocked | `CA/prompts/tools/task-async-contract.md:1` | **No**: advice; `wait` itself returns "Nothing to wait for" when nothing is pending (`docs/tools/wait.md:18`) |
| Delegation gates ("map slices first", "fan genuine slices in one batch", "NEVER outsource top-level plan") | `CA/prompts/system/system-prompt.md:172-179` | **No**: prompt only. Strength set by `task.eager` (`default\|preferred\|always`; `always` adds a first-turn reminder) (`CA/task/settings.ts:218-239`; `CA/prompts/system/system-prompt.md:147-170`) |
| "Message Main only for questions/blockers, never progress" | `CA/prompts/system/subagent-system-prompt.md:48` | **No**: prompt only |
| Children cannot run `todo` | (implicit) | **Yes**: tool removed (`CA/task/executor.ts:4869-4872`) |
| Plan mode = read-only children | `docs/task-agent-discovery.md:332-341` | **Yes**: tool list restricted (`CA/task/structured-subagent.ts:227,260`) |

### 5.4 Doc/code discrepancies found

1. **Focused ←← target.** The status message says `… ←← hops to parent` (`CA/modes/controllers/session-focus-controller.ts:109`). `focusParent()` exists (`:114-122`) and is wired as `focusParentSession` (`CA/modes/interactive-mode.ts:1888-1890`), but no caller in `CA/` invokes it (`rg -n focusParentSession packages/coding-agent/src` shows only the definition and the type). The ←← handler calls `unfocusSession()` (`CA/modes/controllers/input-controller.ts:817-821`). A test checks that `focusParentSession` is *not* called (`test/input-controller-escape.test.ts:954-972`). `docs/agent-hub.md:22,78` agrees with the code ("returns to the main session"). **The code returns to main. The status text is stale.**
2. **Isolated follow-ups.** `CA/prompts/tools/task-follow-up.md:8-9` tells the model an isolated agent "cannot be resumed or messaged". `docs/tools/task.md:54,78,126` describes kept-alive isolated agents that keep their workspace and can be revived, and flags the template as out of date itself.
3. **`solutionSpace` "required".** The schema says it is required, but lenient argument validation still spawns a call without it (`docs/tools/task.md:49`; tool mode `lenientArgValidation=true`, `:167`).
4. **Version skew.** Installed `omp/18.13.1` (`host:/opt/agent-clis/bin/omp --version`) vs studied source `18.13.2`. Behaviour could differ slightly; this study did not check behaviour against the installed binary.

---

## 6. Candidate lessons for maos

All items are [INFERENCE]. maos surfaces are named from `dev_ws@d9bd59c4` file paths and were not deeply studied.

1. [INFERENCE] **One registry, many views.** Give every Spirit instance (Orchestrator, Workers) a stable, human-readable id. Use that one id across the operator door, `maosctl`, Transparency Log rows and on-disk artifacts, the way omp's `AgentRegistry` id is used by the Hub, `agent://`, `history://` and `<id>.jsonl`. Touches `crates/maos-domain/src/orchestrator.rs`, `crates/maos-registry`, `crates/maos-cli`.
2. [INFERENCE] **A parked lifecycle state.** Add `parked` (memory released, record and transcript kept, revivable on message) between `idle` and terminal states, plus a kill tombstone that can be undone by revive. This maps onto the kernel scheduler port (`crates/maos-domain/src/ports/scheduler.rs`) and the Spirit lifecycle journal.
3. [INFERENCE] **A two-keystroke path from parent to children.** `maosctl` could offer an interactive roster (`maosctl agents`/`top`-style). Rows would show status, parent, unread messages, role/model, age, activity, cost/tokens/requests. Keys: Enter = attach to the Spirit's live stream and steer, `r` = revive, `x` = kill with a TL frame, `t` = tree view. Touches `crates/maos-cli` and `crates/maos-bin/src/operator_door.rs`.
4. [INFERENCE] **Attach means re-pointing, not copying.** When the operator "opens" a Worker, re-point the operator stream at that Worker's live event feed (omp's `SessionFocusController`), with request-generation counters so a late revive cannot replace a newer selection. Touches the operator door protocol (`crates/maos-cli/src/door_client.rs`).
5. [INFERENCE] **Activity log scoped all / one agent / subtree, built from the Transparency Log.** omp's Activity section (`f` kind filter, `s` scope) suggests a `maosctl` view over `transparency_log` filtered by `spirit_pid`/lineage. Touches `crates/maos-audit` (read-only query + NDJSON export already exist).
6. [INFERENCE] **Push results, don't poll.** The Orchestrator Spirit should receive Worker completion as a kernel-injected event carrying pointers (output id, transcript id, schema status), with a blocking `wait` only for when it is fully blocked. Touches `spirits/orchestrator`, `crates/maos-kernel-core` IAC.
7. [INFERENCE] **Put shape rules in code and keep behavioural advice in the prompt.** omp rejects malformed fan-out (duplicate names, missing shared context, depth and allow-list violations) in code and leaves "verify once" and "don't poll" to the prompt. maos could encode fan-out width, depth, spawn allow-lists and per-Spirit request budgets as kernel/PDP policy (`crates/maos-pdp`, `crates/maos-capability`) rather than Orchestrator prompt text.
8. [INFERENCE] **A self-reported stage beside measured metrics.** A low-cost periodic `{pct, stage, watch}` probe (side turn, no transcript write) gives operators a narrative label next to measured cost and tokens. In maos it could be a TL frame kind that is clearly marked as a Worker's own claim, not a verified fact (`crates/maos-kernel-core` transparency_log `FrameKind`).
9. [INFERENCE] **Cost totals by lineage.** Keep each Spirit's direct cost and add up descendants for the status line. Show "usage —" rather than an estimate when data is missing. Touches `crates/maos-telemetry`, `crates/maos-audit`.
10. [INFERENCE] **A headless control API that matches the TUI.** omp's RPC `get_subagents / get_subagent_messages(fromByte) / steer_subagent / cancel_subagent / set_subagent_subscription` is a ready-made verb set for the operator door, so `maosctl` and any UI use the same verbs. Touches `crates/maos-bin/src/operator_door.rs`, `crates/maos-cli`.
11. [INFERENCE] **Guard against fat-finger input.** Gestures need debouncing (omp's 40–500 ms double-tap window), and destructive keys (`x`) need clear notices for read-only rows (omp advisors). This is relevant if maos adds an interactive door UI.
12. [INFERENCE] **Do not copy omp's yolo-for-children default.** omp's approval boundary is the parent's `task` approval. maos's capability model likely needs per-Worker capability grants instead (`crates/maos-capability`, `crates/maos-pdp`). This is a point where omp is weaker than what an "agent OS" should provide.

---

## 7. Replication appendix

### 7.1 Get the exact source

```bash
git clone https://github.com/can1357/oh-my-pi /tmp/omp-src
cd /tmp/omp-src
git checkout 9d16dff66da7fefb50d3083cc2f2a7cde6de2ee1
grep '"version"' packages/coding-agent/package.json      # expect 18.13.2
```

### 7.2 Verify the key claims by grep (run from the repo root)

```bash
# ←← gesture: constants, detector, wiring
rg -n 'LEFT_DOUBLE_TAP_M(IN|AX)_GAP_MS\s*=' packages/coding-agent/src/modes/controllers/input-controller.ts   # 271-272: 40 / 500
sed -n 765,781p packages/coding-agent/src/modes/controllers/input-controller.ts                               # onLeftAtStart → showAgentHub
sed -n 910,936p packages/coding-agent/src/modes/controllers/input-controller.ts                               # #detectLeftDoubleTap
sed -n 1711,1717p packages/tui/src/prompt/custom-editor.ts                                                     # editor hook
# Hub keys + key handler
sed -n 214,221p packages/tui/src/app-keybindings.ts                                                            # alt+a, ctrl+s
sed -n 2406,2503p packages/tui/src/overlays/agent-hub.ts                                                       # table input (t,/,Tab,←,j,k,Enter,r,x)
sed -n 2495,2580p packages/coding-agent/src/modes/controllers/selector-controller.ts                           # showAgentHub (requireContent gate)
# Focus controller and the parent-vs-main discrepancy
sed -n 66,131p packages/coding-agent/src/modes/controllers/session-focus-controller.ts
rg -n 'focusParentSession' packages/coding-agent/src packages/coding-agent/test
# Defaults
rg -n 'id: "task\.(maxConcurrency|maxRecursionDepth|agentIdleTtlMs|softRequestBudget|batch|structured|isolation\.enabled)"' -A4 packages/coding-agent/src/task/settings.ts
rg -n 'id: "async\.(enabled|maxJobs)"' -A4 packages/coding-agent/src/tools/settings.ts
# Lifecycle, tombstone, bus
rg -n 'async (park|ensureLive|release)\(|persistAgentTombstone' packages/coding-agent/src/registry/agent-lifecycle.ts
rg -n 'MAILBOX_CAP' packages/coding-agent/src/irc/bus.ts
# Enforcement
rg -n 'MAX_YIELD_RETRIES|"tools.approvalMode": "yolo"|atMaxDepth' packages/coding-agent/src/task/executor.ts
rg -n 'Duplicate task name|Top-level .agent' packages/coding-agent/src/task/index.ts
# Prompt guidance
sed -n 147,209p packages/coding-agent/src/prompts/system/system-prompt.md
cat packages/coding-agent/src/prompts/tools/task.md packages/coding-agent/src/prompts/system/subagent-system-prompt.md
# RPC verbs
rg -n 'subagent' packages/coding-agent/src/modes/rpc/wire/commands.ts
# Tests that pin the gesture
sed -n 1171,1226p packages/coding-agent/test/input-controller-escape.test.ts
```

### 7.3 Reproduce the UX (installed omp, interactive TUI)

Not executed in this study, which was read-only with no launches. The steps come from the cited code.

1. `cd` into any git repo and run `omp`.
2. Prompt: `Use the task tool to spawn two scout agents in parallel: one lists the top-level files, one lists the docs folder.` With defaults (`async.enabled=true`, `task.batch=true`), two ids come back immediately and a **"Subagents · 2 running"** block appears above the editor.
3. Clear the editor, press **←**, wait about 0.1–0.4 s, press **←**. The Agent Hub opens full screen. Presses less than 40 ms apart do nothing.
4. `j`/`k` to select, `t` for tree view, `/` to filter, `2` for Activity, `1` back.
5. **Enter** on a row. The main view now shows that agent ("Viewing agent <id> …"). Type a message and press Enter to steer or prompt it.
6. **Esc** (empty editor) or **←←** returns to main.
7. **Alt+A**, select a finished agent, wait more than 7 min (or set `task.agentIdleTtlMs` lower) until it shows `parked`, press **r** to revive. Press **x** to kill: the status becomes `aborted` and `<id>.jsonl.tombstone` appears in the session's artifact directory.
8. `/hub` opens the Hub on the Activity section. `/jobs` prints the async-job snapshot. `/hotkeys` lists `Alt+A / Ctrl+S / double-tap ← (empty editor)` (`packages/tui/src/hotkeys-markdown.ts:156-166`).

### 7.4 Inspect persistence on disk

```bash
ls -t ~/.omp/agent/sessions/                               # buckets: -<cwd-relative-to-home>
ls -t ~/.omp/agent/sessions/-dev_ws | head                 # <ts>_<id>.jsonl and its same-named artifacts dir
ls ~/.omp/agent/sessions/-dev_ws/<ts>_<id>/ | grep jsonl   # one <AgentId>.jsonl per subagent
```

Observed on this host (2026-10-28), line 2 of a subagent transcript (`…/-dev_ws/2026-10-16T22-06-41-966Z_01a146c0-…/BlindSampling.jsonl`):

```json
{"type":"session","version":3,"id":"01a14829-e239-703f-956c-d0b0156777f7","timestamp":"2026-10-17T04:41:06.873Z","cwd":"/home/lunarpulse/dev_ws","parentSession":"/home/lunarpulse/.omp/agent/sessions/-dev_ws/2026-10-16T22-06-41-966Z_01a146c0-c92e-7329-9912-5b7dbca9f209.jsonl"}
```

Line 1 is the fixed-width `{"type":"title",…}` slot, and lines 3–5 are `model_change`, `thinking_level_change` and `session_init` (with the full system prompt). This matches `docs/session.md:65-101`.

### 7.5 Harness docs

`read omp://agent-hub.md`, `omp://tools/task.md`, `omp://tools/wait.md`, `omp://task-agent-discovery.md`, `omp://session.md`, `omp://rpc.md`. These are the same files as `docs/` in the source.

---

## 8. Open questions

1. Does the installed 18.13.1 binary behave the same as source 18.13.2 for the Hub and gesture? Not checked; the study was read-only and nothing was launched.
2. Is the stale "←← hops to parent" status text (§5.4.1) a bug or planned behaviour (focus-parent navigation for nested agents)? `focusParent()` is implemented but has no callers. Searched: `rg -n focusParentSession packages/` (definition, type and one negative test only).
3. How does `--restore` re-establish interrupted subagents (`CA/task/restore.ts`, `CA/task/persisted-revive.ts`)? Only the CLI doc line was read (`docs/cli-reference.md:88`).
4. How does collab mode (remote guests driving `hubRemote.revive/kill`) handle authority? Only the Hub's remote branches were seen (`TUI/overlays/agent-hub.ts:2545-2549,2569-2573`). `docs/collab.md` was not studied.
5. Eval `agent()` / `workpool()` (fan-out from the Python/JS kernel, always structured) is another orchestration path. It was only touched via `CA/prompts/system/workflow-notice.md:18` and `docs/task-agent-discovery.md:287`. Its UX in the Hub was not traced.
6. How does vibe mode (`vibe_spawn` routing to other CLIs: `fast` → `sonic`, `good` → `task`) appear in the Hub? See `docs/task-agent-discovery.md:111-127`; `docs/vibe-mode.md` was not read.
7. What do the Hub roster metrics (`AgentMetricsSummary`: tokens, requests, tools, cost, durationMs, durationKind) look like at scale, for example the 331 parked peers counted in this session? The rendering budget was not measured; benchmarks exist at `packages/coding-agent/bench/subagent-hud-paint.bench.ts` and `subagent-tree-cost.bench.ts`.

Corrections 2026-10-28 (05-citation-check.md): §1 bullet 4 four-state union now cited to `TUI/overlays/agent-hub-types.ts:7` (was `agent-registry.ts:66-95`); §1 bullet 7 now cites Hub `1`/`2` keys at `TUI/overlays/agent-hub.ts:613-620`; added source cites for plan-mode read-only children (`CA/task/structured-subagent.ts:227,260`, in §4.13 and §5.3) and the 30-minute `wait` cap (`CA/tools/wait.ts:24`, in §2).
