# 02 — openai/codex: agent orchestration and agent/session management UX

- **Study date:** 2026-10-28. **Author:** ResearchCodex (sub-agent). **Language:** English.
- **Subject:** openai/codex at commit `e6de6a2423f9a723c098db3d17ea6c71e93bfcf2`. This is a shallow clone of https://github.com/openai/codex. HEAD commit date is 2026-10-28 11:16 UTC: "Move Guardian review evidence into the reviewer extension (#57702)".
- **Consumer:** the maos BMad technical-research report ("for agents to discuss and learn"). The maos repo was read at `d9bd59c4` and only used to name surfaces.
- **Method:** source reading only (`read`/`grep`/`git`). Nothing was built, run, or installed. No claim in this file was checked against a running binary. Runtime UX is reconstructed from three sources: key handlers, render code, and committed **insta snapshot files**. Snapshots are the TUI's own golden renders.

## Evidence notation

- `C:<path>:<lines>` = `openai/codex@e6de6a24:<path>:<lines>`, where the path is relative to the repo root.
- `M:<path>:<lines>` = `maos@d9bd59c4:<path>:<lines>` (in `/home/lunarpulse/dev_ws`).
- URLs were accessed 2026-10-28.
- `[INFERENCE]` marks a claim that was derived, not directly shown by the cited source.

---

## 1. Summary

1. **The real name of "codex agents" is the *Agent command center*.** `codex agents` is registered as the clap subcommand `Agents` with the help text "Browse all agent sessions on the shared local app-server daemon" (`C:codex-rs/cli/src/main.rs:147-148`). It opens the TUI in a full view titled **"Agent command center"** (`C:codex-rs/tui/src/app/agent_center/render.rs:150-152`). Internally it is called `agents_overview` / `AgentsOverviewView`; its module doc reads "Dashboard for inspecting and managing the TUI's retained daemon tasks" (`C:codex-rs/tui/src/app/agents_overview_view.rs:1-3`). The official docs agree: "Open the command center to browse chats and switch between agents on a shared server" (https://learn.chatgpt.com/docs/developer-commands?surface=cli).
2. **This matches the operator's "manage sessions easily" description.**
   - Each row is a root session ("task"), with columns **Tasks | Status | Updated** and a **Task details** side pane (`C:codex-rs/tui/src/app/agent_center/rows.rs:80-85`, snapshot `…live_center_columns.snap`).
   - Single keys act on the selected row: `Enter`/`→` open (attach), `n` new, `w` new worktree, `o` resume picker, `f` fork, `r` rename, `x` stop, `a` archive, `Backspace` delete, `h` hide, `p` pin, `g` regroup, `/`/`F3` search, `Tab` status filter, `?` help (`C:codex-rs/tui/src/keymap.rs:1973-1987`, `C:codex-rs/tui/src/app/agents_overview_view.rs:699-826`).
3. **The dashboard is possible because the TUI is only a client.**
   - Sessions ("threads") live in a shared **app-server daemon** that the TUI reaches over JSON-RPC: a Unix socket at `$CODEX_HOME/app-server-control/app-server-control.sock`, or a `--remote ws/wss/unix` endpoint (`C:codex-rs/app-server-transport/src/transport/mod.rs:56-73`, `C:codex-rs/tui/src/lib.rs:327-345,1063-1100`).
   - `codex agents` starts the daemon if needed (`C:codex-rs/cli/src/main.rs:2538-2555`) and refuses `--no-daemon` (`:2503-2508`).
   - Leaving the TUI does not stop work. The exit message prints "Disconnected from this task. Any running work continues." followed by exact `resume` and `agents` commands (`C:codex-rs/tui/src/app/exit_summary.rs:95-119`).
4. **Codex also has a one-key "left" gesture.** On an **empty composer**, pressing `←` opens the command center, and the footer advertises "← for agents" (`C:codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs:30-39`; snapshot `…agents_navigation_enabled.snap`). Inside the dashboard, `→` (like `Enter`) opens the selected task (`C:codex-rs/tui/src/app/agents_overview_view.rs:824`).
   - This is the closest codex analogue of omp's "left arrow twice". Codex's `←` reaches the **root-session** dashboard, not the subagent list. Subagents are reached with `/subagents` or `Alt+←`/`Alt+→` (below).
5. **The dashboard has two levels.**
   - It lists **root** threads only: subagent threads (`parent_thread_id` set or `SubAgent(ThreadSpawn)` source) are filtered out (`C:codex-rs/tui/src/app/agents_overview_discovery.rs:77-84`).
   - Their state is **rolled up into the root row**: the most urgent status of the root and all descendants wins (`C:codex-rs/tui/src/app/agents_overview.rs:1232-1243`).
   - The details pane names the most urgent child, e.g. "Agent: <nickname>" (`C:codex-rs/tui/src/app/agents_overview_details.rs:134-190`).
6. **Status is a triage bucket, not a raw state.** The protocol `ThreadStatus` is `NotLoaded | Idle | SystemError | Active{WaitingOnApproval, WaitingOnUserInput}` (`C:codex-rs/app-server-protocol/src/protocol/v2/thread.rs:1701-1718`). It maps to **four** buckets, ordered by the enum's derived `Ord`: NeedsYou ("Needs input ●", or "Error !" for a root `SystemError`), Working ●, Ready ○, and Finished ("Inactive ○"). "Error !" is only a display form of NeedsYou, so a root error row sorts with Needs input, not last (`C:codex-rs/tui/src/app/agents_overview_view.rs:69-102,434-446`).
7. **Sub-agents are model-mediated tools.** There are two generations:
   - **v1** (`multi_agent`, Stable, on by default): `spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`.
   - **v2** (`multi_agent_v2`, Stable, off by default): `spawn_agent` (with a required `task_name`), `send_message`, `followup_task`, `wait_agent` (mailbox), `list_agents`, `interrupt_agent`.
   - Sources: `C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:70-382`, `C:codex-rs/features/src/lib.rs:1556-1567`.
   - Agents report a typed status: `pending_init | running | interrupted | shutdown | not_found | {completed: msg} | {errored: msg}` (`…multi_agents_spec.rs:384-413`).
8. **Roles and limits come from config.**
   - Built-in roles are `default`, `explorer`, and `worker`. An `awaiter` role is commented out (`C:codex-rs/core/src/agent/role.rs:344-406`).
   - Custom roles are declared as `[agents.<role>]` (description, config_file, nickname_candidates) or as `agents/**/*.toml` files in each config layer (`C:codex-rs/config/src/config_toml.rs:764-814`, `C:codex-rs/agent-roles/src/loader.rs:75-83`).
   - Defaults: at most **6** concurrent threads (v1) or **4** (v2), and **depth 1** (`C:codex-rs/core/src/config/mod.rs:260-270`).
9. **Sessions are durable.** Each session is a JSONL **rollout** at `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ts>-<threadId>.jsonl` (`C:codex-rs/rollout/src/recorder.rs:1770-1792`), with metadata in SQLite (`state_5.sqlite`, `C:codex-rs/state/src/sqlite.rs:48-53`).
   - Archiving moves the rollout to `archived_sessions/` (`C:codex-rs/thread-store/src/local/archive_thread.rs:80-83`).
   - A thread record carries `session_id` (shared by the whole spawn tree), `forked_from_id`, `parent_thread_id`, `agent_nickname`, and `agent_role` (`C:codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs:205-295`).
10. **The control plane is one versioned JSON-RPC protocol shared by every UI.**
    - Requests include `thread/start|resume|fork|list|read|archive|unarchive|delete|name/set|section/move|loaded/list|unsubscribe`, `turn/start|steer|interrupt`, and `thread/queue/*`.
    - The server streams notifications such as `thread/started`, `thread/status/changed`, `thread/tokenUsage/updated`, `turn/*`, and `item/*`.
    - The server also sends approval *requests* such as `item/commandExecution/requestApproval` (`C:codex-rs/app-server-protocol/src/protocol/common.rs:552-901,1845-1870,1996-2083`).
11. **Approvals are routed across agents.** A subagent's approval prompt appears in the user's current view with an **"o to open thread"** jump (`C:codex-rs/tui/src/bottom_pane/approval_overlay.rs:567-575,671-681`). A pending approval also turns the root's dashboard row into "Needs input".
    - The policy options are `untrusted | on-request | granular | never` (`C:codex-rs/protocol/src/protocol.rs:1145-1168`). Sandbox modes are `read-only | workspace-write | danger-full-access` (`C:codex-rs/protocol/src/config_types.rs:104-114`).
12. **Observability and cloud.**
    - The dashboard shows per-task tokens. It shows estimated credits/$ only for certain enterprise plans, fetched only for the selected row, at most once a minute (`C:codex-rs/tui/src/app/agents_overview_usage.rs:1-3,28,43-58`).
    - Notifications: desktop notifications via OSC 9 or BEL, sent when the terminal is unfocused or always, plus an external `notify` program that receives `{"type":"agent-turn-complete",…}` (`C:codex-rs/config/src/types.rs:723-827`, `C:codex-rs/core/src/config/mod.rs:784-800`).
    - `codex cloud` is a separate remote-task browser: `exec/status/list/apply/diff`, with best-of-N attempts (1–4) (`C:codex-rs/cloud-tasks/src/cli.rs:15-61`).

---

## 2. Capability inventory

### 2a. Agent command center (`codex agents`)

| Capability | How it works (mechanism) | User-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **`codex agents` subcommand** | Clap variant `Subcommand::Agents(AgentsCommand)` takes `--remote`, `--remote-auth-token-env`, `--cd/-C DIR`, and `--no-alt-screen`; a hidden `--no-daemon` is rejected. Dispatch shares the interactive-TUI path and sets `interactive.agents_overview = true`. It rejects an initial prompt or images, local provider overrides on remote, and workload identity. | `codex agents` opens straight into the dashboard; no chat thread is started (`SessionSelection::AgentsOverview` → `open_agents_overview`). | `C:codex-rs/cli/src/main.rs:147-148,338-353,1104-1148,2503-2508`; `C:codex-rs/tui/src/app/startup.rs:571-572,1109-1113` | R1, R2 |
| **Daemon auto-start** | If no `--remote` is given, the CLI calls `codex_app_server_daemon::run(Start)` before the TUI. Failure prints "The agents overview requires a shared server". The TUI's `AppServerTarget` is `Embedded`, `LocalDaemon`, or `Remote`; only the last two support the command center. | Transparent. `--no-daemon` users get an explicit error. | `C:codex-rs/cli/src/main.rs:2538-2555`; `C:codex-rs/tui/src/lib.rs:327-345,1063-1100` | R2 |
| **In-session entry points** | `/agents` slash command; `←` on an empty, focused composer (when the target supports the command center); `Esc` or `←` in the read-only "external writer" view; a configurable `tui.keymap.global.open_agents` (empty by default); a misalignment-policy action labelled "Agent command center". | Footer hint "← for agents". | `C:codex-rs/tui/src/slash_command.rs:47,144`; `C:codex-rs/tui/src/chatwidget/slash_dispatch.rs:411-413`; `C:codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs:18-39`; `C:codex-rs/tui/src/bottom_pane/chat_composer/footer_state.rs:111-113`; `C:codex-rs/tui/src/app/input.rs:410-422,621-633,747-750`; `C:codex-rs/tui/src/keymap.rs:105-106,1693`; `C:codex-rs/tui/src/chatwidget/misalignment_policy.rs:162-163` | R3, R7 |
| **List view** | `thread/list` paged at 10 per source, across two sources: interactive, and `Exec`+`AppServer`. They are merged by `recency_at`/`updated_at`. Ephemeral and subagent threads are dropped. A "Show more" row fetches the next page. Pinned threads come from the server-side section `PINNED_THREAD_SECTION_ID`. | Header: "Agent command center  Group: Project  g". Filter row: "All N · Needs you N · Working N · Ready N · Inactive N" (`tab/⇧tab filter`). Columns: `Tasks · Status · Updated`. A "Pinned" group appears first. Empty states: "Loading tasks…", "No tasks yet", "Could not load tasks", "Reconnecting…". | `C:codex-rs/tui/src/app/agents_overview_discovery.rs:1-2,19-84,117-158`; `C:codex-rs/tui/src/app/agent_center/rows.rs:13-35,52-85,120-145`; `C:codex-rs/tui/src/app/agent_center/mod.rs:11-17`; `C:codex-rs/tui/src/app/agent_center/render.rs:150-160` | R4 (snapshots) |
| **Row title** | `thread.name`, else `thread.preview` (first user message), else "Untitled task". The colour is a per-thread identity colour shared with the footer. | — | `C:codex-rs/tui/src/app/agents_overview_view.rs:118-121,335-341` | R4 |
| **Status buckets** | `AgentsOverviewGroup::for_thread`: Active with `WaitingOnApproval`/`WaitingOnUserInput` → NeedsYou; other Active → Working; Idle → Ready; root SystemError → NeedsYou (shown as "Error !"); subagent SystemError and NotLoaded → Finished ("Inactive"). Rows keep their relative order within a bucket across refreshes. | `●` red Needs input, `●` green Working, `○` cyan Ready, `○` dim Inactive, `!` red Error. | `C:codex-rs/tui/src/app/agents_overview_view.rs:69-106,227-244,434-446` | R4 |
| **Descendant rollup** | Each root's bucket = `min()` (most urgent) over the root and all descendants. The details pane walks descendants to pick the most urgent one with live detail. | A root whose child awaits approval shows "Needs input". Details show "Agent: <nickname>", "Needs attention", the request preview, "Waiting for approval.", and "Open task to review.". | `C:codex-rs/tui/src/app/agents_overview.rs:342-399,1232-1243`; `C:codex-rs/tui/src/app/agents_overview_details.rs:134-203`; test `C:codex-rs/tui/src/app/agent_center_tests.rs:125-141` | R4, R6 |
| **Task details pane** | Fields: title, status, Model, Reasoning, Project (cwd), usage lines, Branch (git), "Prompt" (2 lines), live reasoning header / "Last message". | Right-hand pane with a `│` divider. | `C:codex-rs/tui/src/app/agents_overview_view.rs:448-564`; snapshot `…usage__agents_overview_usage.snap` | R4 |
| **Grouping and search** | `g` cycles Project → Directory → Status → Model; the choice is persisted to `tui.agents_overview_grouping` in config.toml. `/` or `F3` opens a search filter. | "Group: Project  g". | `C:codex-rs/tui/src/app/agents_overview_view.rs:666-675,703-716`; `C:codex-rs/tui/src/app/agents_overview_grouping.rs:143-161`; `C:codex-rs/config/src/types.rs:1008` | R4 |
| **Live updates** | Overview membership is retained per TUI and fed by server notifications: `thread/started` inserts a row; `thread/tokenUsage/updated` updates usage; `thread/archived`/`thread/deleted` remove it; `thread/closed`. A bounded reseed runs at startup, after reconnect, and after event lag. | Rows change state without a manual refresh. | `C:codex-rs/tui/src/app/agents_overview_threads.rs:1-3,81-140` | R6 |

### 2b. Dashboard actions

| Capability | How it works (mechanism) | User-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **Open / attach** (`Enter`, `→`) | Sends `SelectAgentsOverviewThread`, then `attach_agents_overview_thread`. That saves the current thread's draft and pending approvals, calls `thread/read`, and if the thread is `NotLoaded` builds a resume config and resumes it. Otherwise it attaches to the live thread. | The chat view switches to that session. Draft input is preserved per thread. | `C:codex-rs/tui/src/app/agents_overview_view.rs:381-415,815,824`; `C:codex-rs/tui/src/app/event_dispatch.rs:2912-2925`; `C:codex-rs/tui/src/app/agents_overview.rs:429-571` | R5 |
| **New** (`n`), **New worktree** (`w`) | `NewAgentsOverviewSession{cwd}` uses the selected row's cwd. `w` creates a managed git worktree first (only when worktrees are enabled and the session is local). | Starts a fresh task in the same project. | `C:codex-rs/tui/src/app/agents_overview_view.rs:729-742`; `C:codex-rs/tui/src/app/event_dispatch.rs:2906-2908,2954-2971` | R5 |
| **Resume picker** (`o`) | Sends `AppEvent::OpenResumePicker`, which reaches older and archived sessions outside the dashboard window. | Opens the session picker. | `C:codex-rs/tui/src/app/agents_overview_view.rs:699-702`; `C:codex-rs/tui/src/app/event_dispatch.rs:491-493` | R5 |
| **Fork** (`f`) | Attaches to the source thread, then forks it (`thread/fork`; params support `lastTurnId`/`beforeTurnId`). The UI refuses while a side conversation is open on another task. | A new task appears, branched from the selected one. | `C:codex-rs/tui/src/app/agents_overview_view.rs:743-750`; `C:codex-rs/tui/src/app/event_dispatch.rs:2926-2952`; `C:codex-rs/app-server-protocol/src/protocol/v2/thread.rs:544-583` | R5 |
| **Rename** (`r`) | Inline single-line editor (vim-aware), then `thread/name/set`. On failure the editor reopens and shows "Failed to rename task". | The row title changes in place. | `C:codex-rs/tui/src/app/agents_overview_view.rs:751-765,389-400`; `C:codex-rs/tui/src/app/event_dispatch.rs:2972-2987`; `C:codex-rs/tui/src/app_server_session.rs:1626` | R5 |
| **Stop** (`x`) | Only for Active rows. Finds the in-progress turn (locally, or via `thread/read`/`thread/turns/list`) and sends `turn/interrupt`. | The task drops to Ready. The exit message points users here: "…run codex agents, select this task, and press x". | `C:codex-rs/tui/src/app/agents_overview_view.rs:789-798`; `C:codex-rs/tui/src/app/agents_overview.rs:1123-1178`; `C:codex-rs/tui/src/app/exit_summary.rs:55-62,112-119` | R5, R8 |
| **Archive** (`a`) / **Delete** (`Backspace`) | A confirmation modal states the cascade. A lifecycle modal then runs `thread/archive` / `thread/delete`. Removal cascades to all descendants (matched by `session_id`/parent). If the removed task was the one on screen, the TUI detaches and returns to the dashboard. | Archive: "This stops any running work in this task and its child agents, then archives them. Their history can be restored from the resume picker." Delete: "…permanently deletes their history. This cannot be undone." Delete needs an explicit second confirmation. | `C:codex-rs/tui/src/app/agents_overview_actions.rs:183-238,240-441`; snapshot `…actions__delete_task_confirmation.snap` | R4, R5 |
| **Hide** (`h`) | Adds the id to the TUI's `hidden_threads` set; no server call. [INFERENCE: not persisted across TUI restarts — no write path found.] | The row disappears locally. | `C:codex-rs/tui/src/app/agents_overview_view.rs:781-788`; `C:codex-rs/tui/src/app/event_dispatch.rs:3072-3076` | R5 |
| **Pin** (`p`) | `thread/section/move` into the pinned section, which is server-side and therefore visible to other clients. Only for CLI/VSCode/Exec/AppServer/atlas/chatgpt sources. | The row moves under "Pinned". | `C:codex-rs/tui/src/app/agents_overview_actions.rs:99-132`; `C:codex-rs/tui/src/app/agents_overview_discovery.rs:86-91` | R5 |
| **Help** (`?`), **Back** (`Esc`) | `?` toggles a three-column shortcut sheet. `Esc` closes the view (`ViewCompletion::Cancelled`). | Help sheet groups: Navigate / Tasks / View. | `C:codex-rs/tui/src/app/agent_center/input.rs:83-91`; `C:codex-rs/tui/src/app/agents_overview_view.rs:816-820`; snapshot `…live_center_help_three_columns.snap` | R4 |
| **Keymap customization** | Every dashboard action can be rebound under `tui.keymap.agents.*` and `tui.keymap.global.open_agents`. Conflicts are validated: for example `Backspace` is reserved, and `ctrl-c` conflicts with interrupt. | `/keymap`. | `C:codex-rs/tui/src/keymap.rs:457-470,1393-1430,3711-3838` | R3 |

### 2c. Sessions, subagents, and the control plane

| Capability | How it works (mechanism) | User-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **Session storage** | JSONL rollout per thread under `sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl`. A revert writes `…_<rolloutId>.jsonl`. Thread ids are UUIDv7. SQLite holds the index/state (`state_5.sqlite`, plus `thread_history_1`, `queue_1`, `goals_1`, `logs_2`). | `/rollout` prints the rollout path. | `C:codex-rs/rollout/src/lib.rs:86-87`; `C:codex-rs/rollout/src/recorder.rs:77-104,1770-1792`; `C:codex-rs/state/src/sqlite.rs:48-53`; `C:codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs:206-207`; `C:codex-rs/tui/src/slash_command.rs:160` | R6 |
| **CLI session management** | `codex resume [ID\|name] [--last] [--all] [--include-non-interactive]`; `codex fork [ID] [--last] [--all]`; `codex archive\|unarchive\|delete <ID\|name>` (delete `--force` is UUID-only); `codex queue --thread <ID\|name> --message <TEXT> [--resume]`. All accept `--remote`. | Picker by default; `--last` skips it. | `C:codex-rs/cli/src/main.rs:205-227,355-437`; `C:codex-rs/cli/src/queue_cmd.rs:18-39`; https://learn.chatgpt.com/docs/developer-commands?surface=cli | R1 |
| **Subagent tools v1** (namespace `multi_agent_v1`) | `spawn_agent{message\|items, agent_type, fork_context, model, reasoning_effort}` → `{agent_id, nickname}`. `send_input{target, message\|items, interrupt}`. `resume_agent{id}`. `wait_agent` → `{status:{id→status}, timed_out}` (the final message is included). `close_agent` closes the agent and its descendants. | The transcript shows "Spawned …", "Waiting for N agents", "Closed …" cells. | `C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:14-15,70-106,166-202,265-301,336-359,610-647`; `C:codex-rs/core/src/tools/handlers/multi_agents/{spawn,send_input,resume_agent,wait,close_agent}.rs`; `C:codex-rs/tui/src/multi_agents.rs:339-430` | R9 |
| **Subagent tools v2** | `spawn_agent{task_name*, message*, agent_type, fork_turns: all\|none, model, reasoning_effort}` → `{task_name, nickname}`. `send_message` queues without triggering a turn. `followup_task` triggers a turn. `wait_agent` waits on a mailbox and returns only a summary. `list_agents{path_prefix}`. `interrupt_agent`. There is no `close_agent`: idle runtimes are unloaded by residency accounting. | Same transcript cells; agents are addressed by a canonical task path. | `C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:108-164,204-263,303-334,361-382,649-684`; `C:codex-rs/core/src/tools/handlers/multi_agents_v2/*.rs`; `C:codex-rs/core/src/agent/control/residency.rs:1` | R9 |
| **Result return and cancellation** | The controller delivers terminal child results to the agent tree (best effort, queue-only). A mailbox retains mail while recipients are unloaded. The v2 interrupt rejects root/self targets and never reloads a dead runtime. | — | `C:codex-rs/core/src/agent/control/completion.rs:1-4`; `…/mailbox.rs:1-2`; `…/interrupt.rs:1-4`; `…/watch.rs:1-2` | R9 |
| **Roles** | Built-in `default`, `explorer` (with an `explorer.toml` layer), `worker`. User roles come from `[agents.<name>]` or `<config_folder>/agents/**/*.toml`. A role's description is injected into the `spawn_agent` tool description. | Picker and footer show "nickname [role]"; the primary shows "Main [default]". | `C:codex-rs/core/src/agent/role.rs:33,271-342,344-421`; `C:codex-rs/config/src/config_toml.rs:788-814`; `C:codex-rs/agent-roles/src/loader.rs:75-83`; `C:codex-rs/tui/src/multi_agents.rs:85-104` | R9 |
| **Concurrency and depth limits** | `[agents] max_concurrent_threads_per_session` (alias `max_threads`): default 6 (v1) / 4 (v2). `max_depth`: default 1 (v1 only). `default_subagent_model` / `default_subagent_reasoning_effort`. `wait_agent` timeout: 30 s default, 10 s minimum, 1 h maximum. | — | `C:codex-rs/config/src/config_toml.rs:764-786`; `C:codex-rs/core/src/config/mod.rs:260-270`; `C:codex-rs/core/src/tools/handlers/multi_agents_common.rs:22-24` | R9 |
| **Persisted spawn graph** | `AgentGraphStore`: directional parent→child edges with status `Open\|Closed`; children and breadth-first descendants can be listed. | — | `C:codex-rs/agent-graph-store/src/lib.rs:1`; `…/store.rs:13-55`; `…/types.rs:4-11` | R9 |
| **Subagent switching inside a session** | `/subagents` (enum `MultiAgents`) opens the "Subagents" picker: `thread/list` with `source_kinds=[SubAgentThreadSpawn]` and `ancestor_thread_id=root`, up to 1000 rows. `Alt+←`/`Alt+→` cycle in **first-seen spawn order**. The footer labels the watched agent once there is more than one. | Subtitle: "Select an agent to watch. Alt+← previous, Alt+→ next." Rows show a green dot (`•`) for open agents and a dim dot for closed ones, the name, and the uuid. | `C:codex-rs/tui/src/slash_command.rs:82-83,145`; `C:codex-rs/tui/src/app/agent_picker.rs:16-60`; `C:codex-rs/tui/src/app/agent_navigation.rs:1-20,394-448`; `C:codex-rs/tui/src/app/session_lifecycle.rs:171-224`; `C:codex-rs/tui/src/keymap.rs:2573-2574`; `C:codex-rs/tui/src/multi_agents.rs:76-112` | R7 |
| **App-server protocol** (control plane) | JSON-RPC methods as listed in summary item 10, plus streamed notifications. Collaboration items are typed (`CollabAgentTool`: SpawnAgent … ListAgents; `CollabAgentStatus`; `SubAgentActivityKind`). Transports: stdio, ws, unix socket. | Used by the TUI, IDE, and Desktop clients. [INFERENCE: Desktop/IDE use is from the `SessionSource` names; their code is not in this repo.] | `C:codex-rs/app-server-protocol/src/protocol/common.rs:500-1570,1845-1894,1996-2124`; `C:codex-rs/app-server-protocol/src/protocol/v2/item.rs:1195-1205,1330-1359`; https://learn.chatgpt.com/docs/developer-commands?surface=cli (`codex app-server --listen stdio://\|ws://\|unix://`) | R6 |
| **Human-in-the-loop approvals** | `AskForApproval`: `untrusted\|on-request(default)\|granular{sandbox_approval, rules, skill_approval, request_permissions, mcp_elicitations}\|never`. `SandboxMode`: `read-only(default)\|workspace-write\|danger-full-access`. The server sends approval requests to the client. Approval overlay keys: `y` approve, `a` approve for session, `p` approve prefix, `d` deny, `n`/`Esc` decline, `c` cancel, `o` open the source thread, `Ctrl+A` fullscreen. | A subagent's approval shows "… or o to open thread". | `C:codex-rs/protocol/src/protocol.rs:1145-1185`; `C:codex-rs/protocol/src/config_types.rs:104-114`; `C:codex-rs/app-server-protocol/src/protocol/common.rs:1845-1870`; `C:codex-rs/tui/src/keymap.rs:1988-2004`; `C:codex-rs/tui/src/bottom_pane/approval_overlay.rs:567-575,671-681` | R8 |

### 2d. Observability and cloud

| Capability | How it works (mechanism) | User-facing UX | Evidence | Replication |
|---|---|---|---|---|
| **Usage and cost** | Dashboard: tokens come from `thread/tokenUsage/updated`. Credits/$ are fetched for the selected row only, every 60 s, on Ent/Business/EnterpriseCBP plans. The status line can show `context-used`, `used-tokens`, input/output tokens, `five-hour-limit`, `weekly-limit`, `thread-credits`, `estimated-thread-cost`, `task-progress`, and `thread-id`. `/status` shows tokens; exit prints "Token usage so far: …". | "Tokens: 13K in · 4K out", "Est. usage: 5 credits · ~$0.20". | `C:codex-rs/tui/src/app/agents_overview_usage.rs:1-95`; `C:codex-rs/tui/src/app/agents_overview_threads.rs:103-112`; `C:codex-rs/tui/src/bottom_pane/status_line_setup.rs:57-168`; `C:codex-rs/tui/src/app/exit_summary.rs:121-127` | R4 |
| **Notifications** | `[tui] notifications = true\|[events]`, `notification_method = auto\|osc9\|bel`, `notification_condition = unfocused\|always`. External `notify = ["prog", …]` is called with a JSON argument such as `{"type":"agent-turn-complete",…}`. | Desktop or terminal notification. | `C:codex-rs/config/src/types.rs:723-827`; `C:codex-rs/core/src/config/mod.rs:784-803` | R10 |
| **Background continuation** | The TUI unsubscribes (`thread/unsubscribe`); the daemon keeps running. The exit summary distinguishes "Any running work continues" from "The current turn was stopped". | Exit prints the resume command and how to stop. | `C:codex-rs/tui/src/app_server_session.rs:1744`; `C:codex-rs/tui/src/app/exit_summary.rs:24-130`; snapshot `C:codex-rs/tui/src/app/tests/snapshots/codex_tui__app__tests__background_exit_tests__daemon_disconnect_exit.snap` | R8 |
| **Cloud tasks** (`codex cloud`, alias `cloud-tasks`) | Separate TUI and CLI over the Codex Cloud backend: `exec --env --attempts 1..4 --branch`, `status`, `list --json`, `apply --attempt`, `diff`. TUI hints include `n` New Task, `o` Set Env, Apply (`y` apply, `p` preflight, `n` cancel), cycle attempts, `q` quit. | Browse remote tasks, then apply a diff locally. | `C:codex-rs/cli/src/main.rs:229-231`; `C:codex-rs/cloud-tasks/src/cli.rs:15-120`; `C:codex-rs/cloud-tasks/src/ui.rs:241-272,486` | R1 |
| **Side conversation and queue** | `/side` (alias `/btw`) starts "a side conversation in an ephemeral fork". `codex queue` and `thread/queue/*` queue messages for an existing session. | — | `C:codex-rs/tui/src/slash_command.rs:146-148`; `C:codex-rs/app-server-protocol/src/protocol/common.rs:644-678` | R1 |

---

## 3. UX flows (reconstructed from code and snapshots; not executed)

**F1 — Open the dashboard from the shell and enter a session (the operator's flow).**

1. Run `codex agents`. If the local daemon is not running, the CLI starts it (`C:codex-rs/cli/src/main.rs:2538-2555`).
2. The screen shows "Agent command center  Group: Project  g", the filter row "All N  Needs you N  Working N  Ready N  Inactive N", and the column header `Tasks / Status / Updated`. The right side shows "Task details" for the highlighted row. The footer reads `? help  esc back  ↑/↓ move  enter open  n new` (snapshot `…command_center__live_center_columns.snap`).
3. Move with `↑`/`↓` (also `PgUp`/`PgDn`/`Home`/`End`). Press `Tab` to filter to "Needs you", and `g` to regroup.
4. Press `Enter` or `→` on a row. The TUI reads the thread, resumes it if it was not loaded, and switches the chat view to it (`C:codex-rs/tui/src/app/agents_overview.rs:429-571`).
5. Work in the session. To come back, clear the composer and press `←`; the footer hint says "← for agents". Or type `/agents`.
6. Pick another task and press `Enter`. Each task's unsent draft is preserved (`input_states`, `C:codex-rs/tui/src/app/agents_overview.rs:499-528`).

**F2 — Manage tasks from the dashboard.** Select a row, then:

- `r`, type a new name, `Enter` → renamed.
- `f` → forked copy.
- `p` → pinned to the top.
- `h` → hidden locally.
- `x` (only on Working rows) → current turn interrupted.
- `a` → confirmation "Archive “<title>”? This stops any running work in this task and its child agents…" → choose "Archive task and child agents".
- `Backspace` → "Permanently delete “<title>”? … This cannot be undone." → explicit confirmation (`C:codex-rs/tui/src/app/agents_overview_actions.rs:203-237`).
- `o` → full resume picker for older and archived sessions.
- `n` / `w` → new task, or new task in a fresh git worktree, in the selected project.

**F3 — Leave and come back later.**

1. Exit the TUI with `Ctrl+C`/`Ctrl+D`/`/quit`. The terminal prints:
   - "Disconnected from this task. Any running work continues."
   - "To reconnect, run: codex resume <THREAD_ID>"
   - "Stop the current turn: run codex agents, select this task, and press <stop key>." The snapshot shows `f10` because of a test keymap override; the default stop key is `x`.
   - Sources: `C:codex-rs/tui/src/app/exit_summary.rs:95-119`, `C:codex-rs/tui/src/keymap.rs:1980`.
2. Later, run `codex agents` (or `codex resume <id>`). Tasks that kept running show "Working". Tasks waiting on a human show "Needs input".

**F4 — Watch and drive subagents inside one session.**

1. Ask the model to delegate. It calls `spawn_agent`, and the transcript shows "Spawned <nickname> [role]" (`C:codex-rs/tui/src/multi_agents.rs:339-347`).
2. Press `Alt+→` / `Alt+←` to cycle between Main and each subagent in spawn order. Or type `/subagents` for the "Subagents" picker ("Select an agent to watch. Alt+← previous, Alt+→ next.") and press `Enter`.
3. The footer names the agent being watched, e.g. "`<agent_path>`" or "nickname [role]" (`C:codex-rs/tui/src/app/agent_navigation.rs:394-435`).
4. The parent's `wait_agent` shows "Waiting for N agents". Completed statuses are delivered back to the parent (`C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:287-313`).

**F5 — Approve something a subagent wants.**

1. A child runs a command that needs approval. The server sends `item/commandExecution/requestApproval` (`C:codex-rs/app-server-protocol/src/protocol/common.rs:1845`).
2. The approval overlay appears in the current view, with a thread label and the hint "… or o to open thread". Press `y`/`a`/`p`/`d`/`n`, or `o` to jump into the child (`C:codex-rs/tui/src/bottom_pane/approval_overlay.rs:567-575,671-681`).
3. In the dashboard, the root row stays "● Needs input" until the request is resolved. The details pane shows "Needs attention · Waiting for approval. · Open task to review." (`C:codex-rs/tui/src/app/agents_overview_details.rs:183-199`).

**F6 — Cloud tasks.**

1. Run `codex cloud`. The TUI lists cloud tasks; `o` sets the environment and `n` starts a New Task (with an N-attempt count).
2. Select a task to view its diff. Cycle attempts with `Tab`/`Shift-Tab` or `[` `]`.
3. Apply: the prompt says "Press Y to apply, P to preflight, N to cancel." (`C:codex-rs/cloud-tasks/src/ui.rs:241-272,413,486`).
4. Scripted equivalents: `codex cloud list --json`, `codex cloud apply <TASK_ID> --attempt N`.

---

## 4. Techniques and patterns

| # | Pattern | What codex does | Evidence |
|---|---|---|---|
| P1 | **Client/daemon split** — the UI is a viewer, the daemon owns agents | All TUIs, IDE extensions and remote clients talk to one app-server over JSON-RPC (UDS/ws). A session outlives its window, so a dashboard of "all sessions" is just `thread/list`. | `C:codex-rs/tui/src/lib.rs:327-345`; `C:codex-rs/app-server-transport/src/transport/mod.rs:56-73`; `C:codex-rs/tui/src/app/exit_summary.rs:95-99` |
| P2 | **Triage buckets ordered by urgency** | Raw status plus active flags are folded into Needs you > Working > Ready > Inactive. Sorting by bucket puts human-blocked work first. Filter tabs show the counts. | `C:codex-rs/tui/src/app/agents_overview_view.rs:69-102`; `C:codex-rs/tui/src/app/agent_center/mod.rs:11-17` |
| P3 | **Descendant status rollup** | A root row reflects the most urgent descendant, so a buried subagent asking for approval surfaces at the top level. | `C:codex-rs/tui/src/app/agents_overview.rs:1232-1243`; `C:codex-rs/tui/src/app/agents_overview_details.rs:140-168` |
| P4 | **Two-level hierarchy: roots dashboard plus per-root drill-down** | The dashboard lists roots only. Subagents are reached inside a root via `/subagents` or `Alt+←/→`. This keeps the dashboard small even when one task spawns many workers. | `C:codex-rs/tui/src/app/agents_overview_discovery.rs:77-84`; `C:codex-rs/tui/src/app/agent_picker.rs:40-60` |
| P5 | **Spatial navigation** | `←` on an empty composer goes out to the dashboard; `→`/`Enter` goes into a task; `Alt+←/→` moves sideways among sibling agents. The shortcut is only live when it cannot steal text-editing keys (empty draft, no popup, no vim operator pending). | `C:codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs:9-28`; `C:codex-rs/tui/src/app/agents_overview_view.rs:824`; `C:codex-rs/tui/src/keymap.rs:2573-2574` |
| P6 | **Cascading lifecycle with explicit consequences** | Archive and delete stop running work, then cascade to child agents. The modal spells this out; delete needs an explicit confirmation; failure text says "Work may have stopped. Resume the task to continue, or retry". | `C:codex-rs/tui/src/app/agents_overview_actions.rs:203-237,413-437`; `C:codex-rs/thread-store/src/local/archive_thread.rs:54-65` |
| P7 | **Demand-driven fetch** | Pages of 10, "Show more", and usage fetched only for the selected row every 60 s. Cost stays bounded with thousands of sessions. | `C:codex-rs/tui/src/app/agents_overview_discovery.rs:1-2,19`; `C:codex-rs/tui/src/app/agents_overview_usage.rs:1-3,28` |
| P8 | **Event-sourced live view with periodic reseed** | Notifications mutate the view; a bounded reseed runs on start, reconnect, and lag. | `C:codex-rs/tui/src/app/agents_overview_threads.rs:1-3,81-140` |
| P9 | **Stable ordering** | Rows keep their position within a bucket across refreshes. Subagent cycling follows first-seen spawn order, not id order. | `C:codex-rs/tui/src/app/agents_overview_view.rs:227-244`; `C:codex-rs/tui/src/app/agent_navigation.rs:18-20` |
| P10 | **Delegation as typed model tools** | Sub-agents are spawned by the model through tools with JSON schemas and a closed status enum. The human only steers through prompts. | `C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:384-413` |
| P11 | **Role registry where the description is the prompt** | A role's `description` becomes part of the `spawn_agent` tool text. Roles can carry a config layer (model, instructions) and nickname pools. | `C:codex-rs/core/src/agent/role.rs:271-342,359-371`; `C:codex-rs/config/src/config_toml.rs:801-814` |
| P12 | **Mailbox and queue-only delivery** | v2 `send_message` queues without waking the agent; `followup_task` wakes it. Mail survives runtime unload. | `C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:204-263`; `C:codex-rs/core/src/agent/control/mailbox.rs:1-2` |
| P13 | **Persisted spawn graph** | Parent→child edges with Open/Closed status, stored independently of the runtime, support cascade, listing, and reload. | `C:codex-rs/agent-graph-store/src/store.rs:13-55` |
| P14 | **Exit message as a recovery runbook** | On exit the TUI prints the exact reconnect command and the stop procedure, adapted to remote endpoints and custom keymaps. | `C:codex-rs/tui/src/app/exit_summary.rs:32-63,105-119` |
| P15 | **Cross-thread approval routing** | An approval from any agent appears where the human is, with a one-key jump to its thread. | `C:codex-rs/tui/src/bottom_pane/approval_overlay.rs:567-575` |
| P16 | **Validated, remappable keymap** | Every dashboard verb is configurable; conflicts and reserved keys are rejected at load time. | `C:codex-rs/tui/src/keymap.rs:2040-2059,3711-3838` |

---

## 5. Limits, risks, costs

- **The command center needs the shared daemon.** It is unavailable in `Embedded` mode, with `--no-daemon`, or under workload identity (`C:codex-rs/tui/src/lib.rs:340-344`; `C:codex-rs/cli/src/main.rs:1134-1137,2503-2508`). On platforms other than unix/windows it needs `--remote` (`:1144-1145`). One socket per `CODEX_HOME` means one dashboard scope per home [INFERENCE from the socket path].
- **Subagents are not first-class rows.** They appear only as a rolled-up status plus one "Agent:" line. Direct access is per-root (`/subagents`, `Alt+←/→`). Ephemeral threads (e.g. `/side`) never appear (`C:codex-rs/tui/src/app/agents_overview.rs:347`).
- **Destructive cascades.** Archive and delete stop the whole subtree. Delete is irreversible. A failure can leave work partially stopped (failure copy at `C:codex-rs/tui/src/app/agents_overview_actions.rs:418-427`).
- **Cost visibility depends on the plan.** Credits/$ in the dashboard are fetched only on Ent/Business/EnterpriseCBP plans; the rest see token counts only (`C:codex-rs/tui/src/app/agents_overview_usage.rs:47-55`).
- **Parallelism is capped, and v2 is off by default.**
  - Defaults: 6 concurrent threads (v1) / 4 (v2), depth 1 (`C:codex-rs/core/src/config/mod.rs:260-270`). `multi_agent_v2` is Stable but `default_enabled: false` (`C:codex-rs/features/src/lib.rs:1562-1567`).
  - In v1, completed agents "remain open and count toward the concurrency limit until closed" (`C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:347`).
  - The model decides when to delegate. The docs/search summary notes there is no user-callable spawn API; the code confirms spawning happens only through model tools [INFERENCE: no CLI verb found that spawns a subagent; searched `main.rs` subcommands].
- **Token cost.** Every subagent is a full model loop. With `fork_context`/`fork_turns=all` it inherits parent history (`C:codex-rs/core/src/tools/handlers/multi_agents_spec.rs:627-631,665-668`), which multiplies input tokens [INFERENCE].
- **Complexity cost.** The dashboard alone is about 11 k lines of Rust: `wc -l` over `tui/src/app/agents_overview*.rs agent_center*` gave 10,965 lines including tests, plus 972 lines in `agent_center/`. Lifecycle races (stale refreshes, removal during rename, reconnect) are handled explicitly throughout (e.g. `C:codex-rs/tui/src/app/agents_overview_actions.rs:324-339`).
- **The clone is shallow (1 commit).** Feature introduction dates and history could not be checked.
- **Docs and code drift.**
  - The docs list `/subagents` with anchor `switch-agent-threads-with-agent` and describe it as "Switch the active subagent thread". The code calls the variant `MultiAgents` with description "switch between this session's subagents" (`C:codex-rs/tui/src/slash_command.rs:82-83,145`). Same meaning, different names.
  - A web-search summary claimed `multi_agent` is "experimental and may be off by default". The code says `Stage::Stable, default_enabled: true` (`C:codex-rs/features/src/lib.rs:1556-1561`). **Trust the code.**

---

## 6. Candidate lessons for maos

Every item is `[INFERENCE]`. The maos surfaces are named from these sources:

- `M:crates/maos-bin/src/operator_door.rs:15-18,248-250`
- `M:crates/maos-bin/src/admission.rs:649-661`
- `M:crates/maos-bin/src/shell_host.rs:49-51`
- `M:crates/maos-audit/README.md:6-10`
- `M:crates/maos-audit/src/lib.rs:826-830`

Lessons:

1. **[INFERENCE] Add a "Spirit command center" read view on the operator door.** List live Spirits in triage buckets — Needs-you (pending consent, halt, or approval) / Working / Ready (Loaded / Paused) / Inactive — rather than raw lifecycle states. Surface: `maosctl` plus maos-shell.
2. **[INFERENCE] Make `maosctl` a client of the running daemon for list, attach, and stop, as codex's `AppServerTarget` does.** maos comments describe one-shot `MAOS_ONE_SHOT` children versus door verbs (`M:crates/maos-bin/src/cert_rotation.rs:10-17`); a dashboard needs the door path. Surface: operator door.
3. **[INFERENCE] Roll Worker status up into the Orchestrator Spirit's row**, so a Worker blocked on consent surfaces at the top level, and show "Worker: <name>" in the details. Surface: kernel scheduler state, plus a Transparency Log query.
4. **[INFERENCE] Feed the dashboard from Transparency Log / lifecycle events (subscribe), with a bounded reseed on reconnect** — codex's notifications + reseed pattern — instead of polling `maosctl spirit inspect`. Surface: maos-audit / Lifecycle Journal.
5. **[INFERENCE] Spell out cascades in confirmations.** Unloading or halting an Orchestrator should state that it stops its Workers. Make irreversible verbs (e.g. `forget`) need a second explicit confirmation. Surface: operator door verbs `unload`, `halt resolve`, `forget`.
6. **[INFERENCE] Adopt a closed delegation-status enum for Orchestrator→Worker** (`pending_init | running | interrupted | completed(msg) | errored(msg) | shutdown | not_found`), so TL rows and UIs share one vocabulary. Surface: maos-a2a / maos-spirit-abi.
7. **[INFERENCE] Enforce per-Orchestrator budgets** — max concurrent Workers and max delegation depth — at admission time and record refusals in the TL. Codex defaults: 6 / depth 1. Surface: kernel scheduler admission.
8. **[INFERENCE] Route approvals to the operator with a jump.** A Worker's consent request should appear in whatever maos-shell view the operator has open, with a one-key "open source Spirit". Surface: maos-shell + approval log.
9. **[INFERENCE] Use spatial keys in maos-shell:** `←` out to the Spirit list on an empty prompt, `→`/`Enter` into a Spirit, `Alt+←/→` across sibling Workers. Show the hint in the footer only when it cannot steal editing keys. Surface: maos-shell.
10. **[INFERENCE] Print a recovery runbook on detach** ("work continues; reattach with …; stop with …"), using the actual door address. Surface: `maos run` / maos-shell exit path.
11. **[INFERENCE] Show per-Spirit cost in the details pane** from TL `cost` frames (`maosctl audit query --kind cost`, FrameKind 29), fetched for the selected row only, at a fixed cadence. Surface: maos-audit + shell.

---

## 7. Replication appendix

### R1 — Fetch the pinned source and confirm the CLI surface

```bash
git init codex && cd codex
git remote add origin https://github.com/openai/codex
git fetch --depth 1 origin e6de6a2423f9a723c098db3d17ea6c71e93bfcf2
git checkout --detach FETCH_HEAD
git log -1 --format='%H %ad %s'    # e6de6a24… Wed Oct 28 11:16:31 2026 +0000 Move Guardian review evidence…

rg -n 'Agents\(AgentsCommand\)|Resume\(ResumeCommand\)|Fork\(ForkCommand\)|Archive\(|Delete\(|Queue\(|Cloud\(' codex-rs/cli/src/main.rs
rg -n 'struct AgentsCommand' -A 15 codex-rs/cli/src/main.rs
rg -n 'pub enum Command' -A 12 codex-rs/cloud-tasks/src/cli.rs
```

### R2 — Daemon requirement and target selection

```bash
rg -n 'agents overview requires a shared server|no-daemon cannot be used with codex agents' codex-rs/cli/src/main.rs
rg -n 'fn supports_command_center' -A 6 codex-rs/tui/src/lib.rs
rg -n 'fn app_server_target_for_launch' -A 38 codex-rs/tui/src/lib.rs
rg -n 'APP_SERVER_CONTROL_SOCKET' codex-rs/app-server-transport/src/transport/mod.rs
```

### R3 — Keys

```bash
rg -n 'agents: AgentsKeymap \{' -A 14 codex-rs/tui/src/keymap.rs          # o / F3 n w f r x a Backspace h p g
rg -n 'fixed.(previous|next)_agent' codex-rs/tui/src/keymap.rs            # Alt+Left / Alt+Right
rg -n 'approval: ApprovalKeymap' -A 16 codex-rs/tui/src/keymap.rs
```

### R4 — What the user sees (committed golden renders; no build needed)

```bash
cd codex-rs/tui/src/app/snapshots
cat codex_tui__app__agents_overview__tests__command_center__live_center_columns.snap
cat codex_tui__app__agents_overview__tests__command_center__live_center_help_three_columns.snap
cat codex_tui__app__agents_overview__tests__usage__agents_overview_usage.snap
cat codex_tui__app__agents_overview__tests__actions__delete_task_confirmation.snap
cat codex_tui__app__agents_overview__tests__agents_overview_recent_sessions.snap
cat ../../bottom_pane/snapshots/codex_tui__bottom_pane__chat_composer__tests__agents_navigation_enabled.snap   # "← for agents"
cat ../tests/snapshots/codex_tui__app__tests__background_exit_tests__daemon_disconnect_exit.snap
```

### R5 — Action → RPC mapping

```bash
rg -n 'agents_keymap\.(resume|toggle_grouping|toggle_pin|new_task|new_worktree|fork|rename|hide|stop)|AgentsOverviewAction::(Archive|Delete)' codex-rs/tui/src/app/agents_overview_view.rs
rg -n 'AppEvent::(SelectAgentsOverviewThread|ForkAgentsOverviewThread|RenameAgentsOverviewThread|HideAgentsOverviewThread|StopAgentsOverviewThread)' codex-rs/tui/src/app/event_dispatch.rs
rg -n 'thread_archive\(thread_id\)|thread_delete\(thread_id\)|turn_interrupt\(thread_id' codex-rs/tui/src/app/agents_overview*.rs
rg -n 'ClientRequest::(ThreadSetName|ThreadArchive|ThreadDelete|TurnInterrupt|ThreadFork|ThreadUnsubscribe) \{' codex-rs/tui/src/app_server_session.rs
```

### R6 — Protocol, status model, storage

```bash
rg -n '=> "thread/|=> "turn/' codex-rs/app-server-protocol/src/protocol/common.rs
rg -n 'pub enum ThreadStatus|pub enum ThreadActiveFlag' -A 10 codex-rs/app-server-protocol/src/protocol/v2/thread.rs
rg -n 'pub struct Thread \{' -A 90 codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs
rg -n 'SESSIONS_SUBDIR|ARCHIVED_SESSIONS_SUBDIR' codex-rs/rollout/src/lib.rs
rg -n 'Resolve ~/.codex/sessions/YYYY/MM/DD' -A 16 codex-rs/rollout/src/recorder.rs
rg -n '_DB_FILENAME' codex-rs/state/src/sqlite.rs
rg -n 'fn agents_overview_group' -A 11 codex-rs/tui/src/app/agents_overview.rs     # rollup
```

### R7 — Entry gestures and subagent navigation

```bash
cat codex-rs/tui/src/bottom_pane/chat_composer/agents_navigation.rs
rg -n 'SlashCommand::(Agents|MultiAgents) =>' codex-rs/tui/src/chatwidget/slash_dispatch.rs codex-rs/tui/src/slash_command.rs
rg -n 'fn picker_subtitle' -A 8 codex-rs/tui/src/app/agent_navigation.rs
rg -n 'ancestor_thread_id|SubAgentThreadSpawn' codex-rs/tui/src/app/agent_picker.rs
```

### R8 — Background continuation and approvals

```bash
rg -n 'Any running work continues|Stop the current turn' codex-rs/tui/src/app/exit_summary.rs
rg -n 'pub enum AskForApproval' -A 24 codex-rs/protocol/src/protocol.rs
rg -n 'pub enum SandboxMode' -A 10 codex-rs/protocol/src/config_types.rs
rg -n 'open_thread' codex-rs/tui/src/bottom_pane/approval_overlay.rs
```

### R9 — Multi-agent tools, roles, limits

```bash
rg -n 'name: "(spawn_agent|send_input|send_message|followup_task|resume_agent|wait_agent|list_agents|close_agent|interrupt_agent)"' codex-rs/core/src/tools/handlers/multi_agents_spec.rs
ls codex-rs/core/src/tools/handlers/multi_agents codex-rs/core/src/tools/handlers/multi_agents_v2
rg -n 'DEFAULT_AGENT_MAX_THREADS|DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT|DEFAULT_AGENT_MAX_DEPTH' codex-rs/core/src/config/mod.rs
rg -n 'key: "multi_agent' -B 2 -A 3 codex-rs/features/src/lib.rs
rg -n '"explorer"|"worker"|DEFAULT_ROLE_NAME' codex-rs/core/src/agent/role.rs
rg -n 'pub struct AgentsToml' -A 35 codex-rs/config/src/config_toml.rs
head -1 codex-rs/core/src/agent/control/{completion,mailbox,interrupt,residency,watch}.rs
```

### R10 — Notifications and status line

```bash
rg -n 'pub struct TuiNotificationSettings' -B 90 -A 16 codex-rs/config/src/types.rs | rg -n 'Notification'
rg -n 'agent-turn-complete' codex-rs/core/src/config/mod.rs
rg -n 'pub\(crate\) enum StatusLineItem' -A 112 codex-rs/tui/src/bottom_pane/status_line_setup.rs
```

### Runtime check (NOT executed in this study; hard rule: no builds or installs)

Build and launch:

```bash
cd codex-rs && cargo build -p codex-cli          # or an installed codex release built from ≥ this commit
./target/debug/codex login                       # an account is required for model turns
./target/debug/codex agents                      # → "Agent command center"
```

Keys to press in the dashboard:

| Key | Expected |
|---|---|
| `?` | help sheet |
| `Tab` | filter cycles All → Needs you → Working → Ready → Inactive |
| `g` | grouping cycles Project → Directory → Status → Model |
| `n` | new task |
| type a prompt, then clear the composer and press `←` | back to the dashboard |
| `Enter` | re-open the task |
| `r` | rename |
| `x` | stop (only on a Working row) |
| `a` | archive confirmation |
| `Esc` | back |

To see subagents, prompt: "Spawn two explorer agents to answer X and Y, then wait for both". Then press `Alt+→` / `Alt+←`, or type `/subagents`.

To test detach: quit with `Ctrl+C` while a turn is running. Expect the "Any running work continues" message, then `codex agents` shows the task as Working.

Optional tests (not executed): `cargo test -p codex-tui agents_overview` and `cargo test -p codex-tui agent_center`.

---

## 8. Open questions

1. Does `h` (hide) persist across TUI restarts? Only an in-memory `hidden_threads` set was found (`C:codex-rs/tui/src/app/event_dispatch.rs:3072-3076`). Searched the `agents_overview*` files for a persistence write and found none.
2. Do running turns survive a **daemon** restart or update? `loaded-threads.json` (`C:codex-rs/app-server-transport/src/transport/mod.rs:59-65`) and `app-server-daemon/src/thread_recovery.rs` ("Removes stale snapshots before planned daemon replacements") suggest loaded threads are recovered. Whether in-flight turns are resumed was not verified.
3. The v2 multi-agent tool namespace default is `"collaboration"` (`C:codex-rs/core/src/config/mod.rs:265`). The exact tool names the model sees under v2 (namespaced or not) were not traced end-to-end.
4. How does the Desktop app's "command center" relate to this TUI view? Both are reported to use the app-server protocol [INFERENCE from `SessionSource` names], but the Desktop code is not in this repo.
5. Can a user list or attach to subagent threads **across roots** from one view? Not found. The dashboard filters them out, and `/subagents` is scoped to `ancestor_thread_id=root` (`C:codex-rs/tui/src/app/agent_picker.rs:59`).
6. When were `codex agents` and `←`-for-agents introduced? This could not be determined from a shallow clone; it would need `git fetch --unshallow` and `git log -S 'Browse all agent sessions'`.

Corrections 2026-10-28 (05-citation-check.md): §1 item 6 now says codex has four status buckets (NeedsYou, Working, Ready, Finished), and that "Error !" is how NeedsYou renders a root error and sorts with Needs input, not last (was five buckets with Error last).
