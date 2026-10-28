---
title: "maos baseline — what maos already has and intends for agent orchestration and operator visibility"
study: 04-maos-baseline
research_set: agent-orchestration-2026-10-28
date: 2026-10-28
author: ResearchMaosBaseline (sub-agent; read-only study)
source: maos @ d9bd59c456a68a9aaddbbee4594796beed288b91 (`/home/lunarpulse/dev_ws`; working tree clean when read — `git status --short | wc -l` = 0)
language: English
---

# maos baseline: orchestration and operator visibility (maos @ `d9bd59c4`)

**Citation convention.** Unless a citation has a different prefix, every `path:line` refers to
`maos@d9bd59c4` (`/home/lunarpulse/dev_ws`, HEAD `d9bd59c456a68a9aaddbbee4594796beed288b91`,
commit message "19-1, 19-7 (in-progress): halt-row assertion admits the unload's Terminated receipt; dev records").
`[INFERENCE]` marks anything this study did not see directly in source or docs. "Not found" means the
listed search returned zero hits. I **did not build or run** any binary (the task ruled that out), so every
runtime behaviour below comes from reading code and docs. The live commands in §7.3 are given so a
later agent can replay them; I did not run them.

**Status caveat.** Stories `19-1` (Orchestrator dispatch loop + `--epic`) and `19-7` (re-pin consent)
are **in-progress** (`_bmad-output/implementation-artifacts/sprint-status.yaml:287,293`). Their code is
in the tree at this SHA but has not passed review or landed. `19-2`, `19-3`, `19-4`, `19-5`, `19-6`,
`20-14` and `21-5` are **backlog** (`sprint-status.yaml:288-292,311,320`).

---

## 1. Summary

- **One daemon, one door.** `maos run` / `maos shell` (and the `cohort-a2a-daemon` one-shot) bind one bearer-authenticated
  loopback HTTP door. `maosctl` finds it through `MAOS_HOME/control.json`. Every mutating operator verb goes through
  `OperatorCommand` → `OperatorCommandPort::submit` (`docs/adr/ADR-062-mutating-operator-surface.md:46-59,122-127`;
  `crates/maos-control/src/lib.rs:296-397`). The door is plain request/response (`Connection: close`, 5 s read deadline,
  16 workers), with no streaming route (`crates/maos-control/src/lib.rs:233-253,1960`).
- **No `maosctl` verb lists agents.** The daemon serves `GET /v1/daemon`, which returns `spirit_ids[]` and `lifecycle_states[]`
  (`crates/maos-control/src/lib.rs:1101-1118`). No `maosctl` subcommand calls it: `rg 'v1/daemon|spirit_ids' crates/maos-cli/src` = 0.
  The one per-agent read is `maosctl spirit inspect <id>` → `GET /v1/spirits/{id}`, which shows pid, boot, lifecycle,
  posture and ceiling (`crates/maos-cli/src/subcommands.rs:2536-2567`).
- **No delegation tree anywhere.** The `SpiritControlBlock` has no parent field (`crates/maos-kernel-core/src/scheduler/control_block.rs:258-287`).
  It does keep an in-flight ledger whose records name `originator_spirit_id` (`:278`; `crates/maos-domain/src/ports/task.rs:12-24`),
  but no route exposes it. Delegation frames on the Transparency Log (TL) are journaled with sender pid 0 — a known defect
  (`sprint-status.yaml:300`, 20-3c member 17; source: `deliver_typed(frame, 0, None)` at `crates/maos-bin/src/delegation.rs:452,581,681,750`.
  The line numbers quoted inside the tracker entry are stale at d9bd59c4).
- **The live view is the stdout of the `maos run` process.** It is a JSON-lines event stream (`spirit_loaded`,
  `topology_worker_admit`, `delegation_routed`, `cli_wrapper_loaded`, `cli_wrapper_exit`, `worker_completion`,
  `distillate_written`, `delegation_completed`, `dispatch_halted`, `drain`; `crates/maos-bin/src/main.rs:4941-5355`,
  `crates/maos-bin/src/worker_spawn.rs:1162-1300`, `crates/maos-bin/src/orchestrator_root.rs:216-220`). Nothing can attach
  to it after the fact: no `attach`/`tail`/`watch` verb exists.
- **A Worker's "session" is TL rows.** Each Worker CLI's stdout/stderr lines are journaled as `cli.subprocess.output` rows at
  the Worker's own pid (`worker_spawn.rs:1183-1189,1250-1259`). `maosctl audit query` reads them straight from SQLite after the
  fact. Workers get ids `worker`, `worker-2`, `worker-3`, … (`crates/maos-bin/src/supervision.rs:1925-1932`).
- **Steering is designed but not wired.** FR20's `maosctl orchestrator queue` enqueues into the daemon's `OrchestratorBuffer`
  through the door (`crates/maos-bin/src/operator_door.rs:1211-1261`). No production code dequeues it:
  `rg 'drain_next|dequeue_at_safe_point' crates/maos-bin/src` = 0 code hits. That consumer is 19-2, which is in backlog
  (`_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md:175-177`). Workers cannot be messaged at all, because
  T3 refuses every stdin-keeping Worker posture (`worker_spawn.rs:665-667`; `docs/adr/ADR-061-worker-egress-and-scoped-credential.md:35-37`).
- **Lifecycle control exists per Spirit id** through `start`, `pause`, `resume`, `unload`, `posture`, `upgrade` (hot-swap only) and
  `uninstall` (`crates/maos-control/src/lib.rs:1430-1525`). Pausing a Worker CLI under the Signals channel is a **documented no-op**
  (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:1138-1151`). `unload` really stops the Worker
  (`operator_door.rs:371,590-592`).
- **Halts work through the door.** An Orchestrator halt stops dispatch before the next `task.assign` and drains the run
  (`orchestrator_root.rs:196-225`; `main.rs:4853-4878`). The operator runs `maosctl halt list` (a direct TL read) and then
  `maosctl halt resolve` over the door, with three kinds (`crates/maos-cli/src/cli.rs:769-812`). There is no live
  pending-halt route, and resume-after-halt within a run is 19-4 AC5 (backlog). The Orchestrator's ambiguity judgement
  (`judge_ambiguity`) exists but has **zero production callers** (rg = 0 in `crates/maos-bin/src`).
- **Approvals are spread over three lists.** Halts (`halt list/resolve`), skills (`skills list/approve/reject`) and A2A re-pins
  (`a2a repin list/approve/reject`, only on the cohort daemon door). There is no unified inbox. The kernel `ApprovalManager`
  still auto-allows per-action prompts (`crates/maos-kernel-core/src/security/approval.rs:43-75`).
- **Cost is monthly and inference-only.** `maosctl audit cost-reconcile --month` groups cost frames by month × principal × Spirit × provider × model,
  with no task axis (`cli.rs:712-729`). Cost frames come only from the kernel Inference Port (`crates/maos-kernel-core/src/inference/mod.rs:414-434`).
  Worker-CLI token spend through the egress proxy is not attributed: `rg 'CostAttribution|usage'` over `crates/maos-bin/src/egress.rs`
  and `crates/maos-egress/src` = 0. Context-window pressure is journaled as `context.pressure` / `context.limit` TL rows
  (`crates/maos-audit/src/lib.rs:780-782`).
- **Hard constraints for any new orchestration UX** (cited in §5.1):
  - the kernel embeds no orchestration policy and owns no UI (`_bmad-output/planning-artifacts/prd/functional-requirements.md:11,14`);
  - the kernel-core envelope is ≤ 23,000 tokei with zero headroom (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`);
  - one door, bearer before route, and `maosctl` must not link `maos-control` or the kernel (ADR-062);
  - live state comes from the SCB and history from the TL (`crates/maos-control/src/lib.rs:5-6,510-512`);
  - I2 (log before deliver) and I4 (approval captures intent);
  - T3 Workers with no stdin and no host home;
  - Epic-19 per-crate kloc envelopes are already near exhausted (`epic-19-founder-loop-w4.md:38-48`; `xtask/kloc.toml:517`).
- **Product intent already asks for a dashboard-like view.**
  - J1 wants halt notifications with candidate answers, queued user input, and a morning digest (`prd/user-journeys.md:126-161`).
  - Architecture §6.5 has the Observer Spirit render "a live 'what are the agents doing' view in the operator's TUI or editor banner"
    (`architecture-maos-minimal-opus/6-reference-spirits.md:92`).
  - The 2026-07-17 delegation-UX analysis proposes `maos task submit/watch/review/approve` with a task state machine
    (`analysis-maos-coding-delegation-ux-2026-07-17.md:324-412`).

---

## 2. Capability inventory

### 2.1 Capability inventory table

All paths are maos@d9bd59c4. "Repl." points to the numbered commands in §7.

| Capability | How it works (mechanism) | User-facing UX | Evidence | Repl. |
|---|---|---|---|---|
| Daemon + operator door | `maos run` / `maos shell` / `cohort-a2a-daemon` bind a loopback HTTP server. It refuses an empty bearer and non-loopback binds, checks the bearer in constant time, and authenticates before routing. | `maos init` writes `control.json` (`{"version":1,"endpoint":"tcp://127.0.0.1:<port>","token":"<64 hex>"}`, mode 0600). `maosctl` finds it, or uses the env override pair. | `crates/maos-control/src/lib.rs:198-209,658-668,1023-1029`; ADR-062 `:46-59,122-127,156-160`; `crates/maos-cli/src/door_client.rs:3-9,171-199` | R1, R2 |
| Door command model | One `OperatorCommand` enum. The port mints a 96-bit `operation_id`. Commands are serialized per Spirit id (`spirit_key`) and run inside a route budget (10 s / 30 s). Exactly one completion TL row `operator.<verb>.<op>:<outcome>` is written **before** the answer is delivered. | `maosctl` prints typed codes and exit codes. A 503 `handler_still_running` carries the `operation_id`, which `maosctl audit query --intent-contains <id>` can find. | `crates/maos-control/src/lib.rs:289-397,405-462,472-502`; `crates/maos-bin/src/operator_door.rs:1606-1652,1658-1663`; ADR-062 `:168-174` | R2 |
| List agents | `GET /v1/daemon` → `pid`, `boot_nonce`, `version`, `spirit_ids[]`, `lifecycle_states[]`, `audit_degraded`, `audit_drop_count`. One sorted pass over all SCBs, Workers included. | **No maosctl verb.** Reachable only with raw HTTP plus the bearer (tests do exactly this). | `crates/maos-control/src/lib.rs:1101-1118,529-548`; `crates/maos-bin/src/operator_door.rs:1719-1742`; `crates/maos-bin/tests/maosctl_load_16_6.rs:798-813`; not found: `rg 'v1/daemon\|spirit_ids' crates/maos-cli/src` = 0 | R2, R6 |
| Inspect one agent (live) | `GET /v1/spirits/{id}` reads `SpiritControlBlock::current_state` and the live `PolicyTable`, never a journal row. | `maosctl spirit inspect <id>` prints spirit_id, pid, lifecycle_state, posture and ceiling. `--sandbox` gives the sandbox report (`GET /v1/spirits/{id}/sandbox`). | `crates/maos-control/src/lib.rs:504-525,1163-1198`; `crates/maos-cli/src/subcommands.rs:2536-2567,2615-2619`; `crates/maos-bin/src/operator_door.rs:1693-1717` | R7 |
| Lifecycle verbs | `POST /v1/spirits/{id}/{start\|pause\|resume\|unload\|uninstall\|posture\|upgrade\|hot-swap-precheck}` and `POST /v1/spirits` (load). The scheduler transitions the SCB and fires the hook. The door then appends the lifecycle journal and an Approval Decision Log row. | `maosctl load <manifest>`, `start`, `pause`, `resume`, `unload`, `uninstall`, `posture <id> --shift`, `spirit upgrade`, `spirit hot-swap-precheck`. `maosctl stop` is refused on purpose ("use pause or unload"). | `crates/maos-control/src/lib.rs:1150-1162,1199-1208,1430-1525`; `crates/maos-bin/src/operator_door.rs:554-645`; `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:410-505,525-548`; `crates/maos-cli/src/subcommands.rs:31-48,67-71,261-281` | R8 |
| Upgrade | Over the door: hot-swap only, forward version only. Cold swap is refused with a typed error. Cross-form upgrade is 19-5 (backlog). | `maosctl spirit upgrade <id> --to <manifest> [--policy hot-swap]` | ADR-062 `:175-181`; `crates/maos-control/src/lib.rs:1453-1507`; `crates/maos-cli/src/cli.rs:898-925`; `sprint-status.yaml:291` | — |
| Orchestrator (FR20) buffer | `POST /v1/orchestrator/{id}` → `OrchestratorBuffer::enqueue` (bounded, capacity 32) plus an Approval Decision Log row `orchestrator.queue`. `GET` returns pending/capacity. **Nothing consumes the buffer in production** (19-2). | `maosctl orchestrator queue --spirit <id> "<text>"`, `maosctl orchestrator status --spirit <id>` | `crates/maos-control/src/lib.rs:1246-1284`; `crates/maos-bin/src/operator_door.rs:1211-1261,1744-1761`; `crates/maos-kernel-core/src/orchestrator/mod.rs:3-17,38-58`; `crates/maos-cli/src/subcommands.rs:2173-2218`; not found: `rg 'drain_next\|dequeue_at_safe_point' crates/maos-bin/src` = 0 | R5, R9 |
| Orchestrator Spirit | Reference Spirit `orchestrator` v0.8 (T0, `autonomous-with-halt`). It splits an epic deterministically (one section per `### Story`), dispatches distillate-fed follow-ups (FR21), and keeps a safe-point gate (`is_safe_point` = in-flight == 0). Its inference port is used **only** by `judge_ambiguity`. Zero kernel KLOC. | Started as the first entry of a topology manifest, e.g. `spirits/topologies/j1-founder-loop-epic.toml`. | `spirits/orchestrator/src/lib.rs:3-45,227,396,452-454,483,636-694`; `spirits/orchestrator/src/epic.rs:1-16`; `spirits/orchestrator/manifest.toml:11-58`; `spirits/topologies/j1-founder-loop-epic.toml:1-31` | R10 |
| Topology run loop | `maos run <topology> [--epic <file>] [--once] [--live]`. Sequential `for` over `[[topology.spirits]]`. Host-bearing entries become frame-borne `task.assign` delegations. Each Worker runs synchronously (`block_in_place`). After each Worker the root writes a Distillate at the Worker's pid, and the next section carries its ref. | The operator watches stdout JSON lines. `--once` = one `on_idle` pass, then drain. | Command line: `crates/maos-bin/src/worker_spawn.rs:55-79` (`RunArgs` + `RUN_USAGE`). Sequential `for`: `crates/maos-bin/src/main.rs:4687`. `block_in_place`: `main.rs:5028` (cites corrected 2026-10-28 by the adversarial review [06 F20]; `worker_spawn.rs:55-79` holds only the argument struct, not the loop). Also `crates/maos-bin/src/main.rs:4669-4687,4885-4901,5010,5028,5093-5120`; `crates/maos-bin/src/topology.rs:11-24,57`; `crates/maos-bin/src/orchestrator_root.rs:142-194,274-286` | R10 |
| Delegation leg (loopback / cross-host) | The Orchestrator emits `task.assign` with a consent envelope (intent `development-task:write-workspace`). It routes over loopback A2A, or over mTLS/TOFU to host B, where `cohort-a2a-daemon` spawns the Worker. | Host A stdout: `delegation_routed`, `topology_worker_delegated_offhost`. Host B stdout: `host_b_delegation_served`. Afterwards: `maosctl audit reconcile-hosts` joins the two signed halves on `frame_id`. | `crates/maos-bin/src/delegation.rs:3-39,58-70,966`; `crates/maos-bin/src/main.rs:4941-4997`; `crates/maos-cli/src/cli.rs:587-620`; `spirits/orchestrator/src/lib.rs:227` | R10 |
| Worker (CLI-wrapper) | A `[cli_wrapper]` manifest; T3 is required (rootless podman, `--network=none`, egress via a per-spawn proxy socket, scoped bearer). Stdout/stderr are pumped into the TL at the Worker's pid. The adapter's `parse_completion` oracle decides completion. | stdout: `cli_wrapper_loaded`, `cli_wrapper_exit`, `worker_completion` (with `last_stdout_tl_ref`). | `crates/maos-bin/src/worker_spawn.rs:585,665-667,842-845,1106,1162-1300`; ADR-061 `:14-37`; `crates/maos-bin/src/supervision.rs:1925-1932` | R11 |
| Halts (FR15) | `invoke_halt` writes a TL row, a journal entry and a `HaltRegistry` insert. The Orchestrator's dispatch stops when its pid has a pending halt or no longer resolves (unloaded). Resolution goes over the door (`POST /v1/halts/{id}/resolve`) with three kinds. | stdout `dispatch_halted`. `maosctl halt list [--spirit]` (TL read). `maosctl halt resolve <id> --spirit <s> --kind provided-context\|accepted-halt\|authorized-override`. | `crates/maos-kernel-core/src/halt/mod.rs:101,243,297,530`; `crates/maos-bin/src/orchestrator_root.rs:196-225`; `crates/maos-bin/src/main.rs:4853-4878`; `crates/maos-control/src/lib.rs:1209-1245`; `crates/maos-cli/src/cli.rs:769-812`; `crates/maos-cli/src/subcommands.rs:2035-2039` | R12 |
| Audit / transcript | `maosctl audit query` opens the TL SQLite **directly** (not through the door). Filters: `--spirit`, `--range`, `--frame-kind`, `--intent-contains`, `--capability`, `--boot`, `--all-boots`. With `--spirit` it prints the FR4 six-field projection without payload; without it, payloads are included. | Post-hoc only. No follow/tail mode. | `crates/maos-cli/src/cli.rs:467-514`; `crates/maos-cli/src/subcommands.rs:2889-3026` (FR4 switch `:3003-3009`, direct DB `:2901`); `crates/maos-audit/src/lib.rs:93-120,738-784` | R13 |
| Signed export / two-host reconcile | `sealed-export`, `verify-bundle`, `reconcile-hosts` (joined on `frame_id`), `record-capture`, `export` (trajectory), `replay`. | Offline evidence artifacts. | `crates/maos-cli/src/cli.rs:515-620,678-711` | — |
| Cost (FR64) | The kernel emits `cost.attribution` (kind 29) TL frames with raw TokensIn/TokensOut and a write-target-proxy principal on Inference Port calls only. Money is computed at read time. | `maosctl audit cost-reconcile --month YYYY-MM` (no task axis until 20-14). | `crates/maos-domain/src/cost.rs:1-30,53-61`; `crates/maos-kernel-core/src/inference/mod.rs:414-434`; `crates/maos-cli/src/cli.rs:712-729`; ADR-046 `:22-44`; `epic-20-ship-it-w5.md:233-240` | R14 |
| Context budget (18-5) | Spirits self-report tokens. The kernel emits `ContextPressure` (80 %) / `ContextLimit` (95 %) and refuses tool calls past 100 %. | Readable only as TL rows `context.pressure` / `context.limit`. No status route. | `_bmad-output/implementation-artifacts/18-5-context-token-budget-accounting.md:1-30`; `crates/maos-audit/src/lib.rs:780-782`; `crates/maos-bin/src/spirit_session.rs:792-794` | R13 |
| Re-pin consent (19-7, in-progress) | The receiver records a restart detection. The operator approves or rejects by echoing the observed boot nonce. The Approval Decision Log row is written before the pin changes, and requests expire after 300 s. Only the `cohort-a2a-daemon` door serves it. | `maosctl a2a repin list\|approve <peer> --boot-nonce N\|reject` | ADR-062 `:183-223`; `crates/maos-control/src/lib.rs:1119-1136,1861-1888`; `crates/maos-cli/src/cli.rs:212-256` | — |
| Interactive shell | `maos shell` REPL parses `@<spirit> <msg>`. Only `hello-spirit` is dispatched; `@butler pick` prints canned strings. A tick renders halt resolutions that arrive from any surface. | `@hello-spirit say hi`; typing a clarification resolves a pending halt. | `crates/maos-shell/src/lib.rs:3-8,332-395,500-537,628-677` | — |
| ACP editor surface | `MAOS_ONE_SHOT=acp-server` speaks NDJSON frames: `session_start/end`, `lifecycle_verb`, `halt_resolve` in; receipts and `notification_dispatch` out. It is a separate one-shot root, not one of the three door roots. | Editor-hosted (Zed). | `crates/maos-acp/src/frame.rs:19-77`; `crates/maos-bin/src/verbs.rs:164-165`; ADR-062 `:122-127` | — |
| Notifications | `maos-director-surface` `NotificationDispatcher`: the terminal channel is real; ACP/mobile were stubs at v0.3-β. The `maos-notify-push` HTTP adapter exists but has **no maos-bin caller**. | stderr / ACP only. | `crates/maos-director-surface/src/lib.rs:3-12`; `crates/maos-notify-push/src/lib.rs:1-25`; not found: `rg 'maos_notify_push\|PushConfig\|NotificationSurface' crates/maos-bin/src` = 0 | R15 |
| Observer Spirit | Read-only watchdog: receives `scalar.tap` writes and raises a drift early warning before a halt fires. | Notification events. No "agents view" renderer was found in code. | `spirits/observer/src/lib.rs:1-30`; architecture intent `6-reference-spirits.md:82-94` | — |

### 2.2 Operator UX inventory: need → maos surface today → gap

| Need | maos surface today (file:line) | Gap |
|---|---|---|
| **(a) List running agents / Spirits** | `GET /v1/daemon` → `spirit_ids` + `lifecycle_states` (`crates/maos-control/src/lib.rs:1101-1118`; `crates/maos-bin/src/operator_door.rs:1719-1742`). Startup events `spirit_loaded` / `topology_worker_admit` / `cli_wrapper_loaded` on the run's stdout (`main.rs:5010,5286`; `worker_spawn.rs:1162`). | **No `maosctl` verb.** The CLI enum has no list/ps/agents verb (`crates/maos-cli/src/cli.rs:40-143`; rg = 0 for `v1/daemon` in maos-cli). The listing has no role, parent, current task, host, age, token use or last activity. Workers on another host are not listed at all; host B is a separate daemon and door (ADR-062 `:122-127`). |
| **(b) See one agent's activity / transcript** | Live, only if you own the terminal: the `maos run` stdout JSON lines. After the fact: `maosctl audit query --frame-kind cli.subprocess.output` (payload included without `--spirit`; `subcommands.rs:3003-3009`). Worker rows are keyed by the Worker's own pid (`worker_spawn.rs:1250-1259`). `spirit inspect` gives a status snapshot (`subcommands.rs:2536-2567`). | No attach/follow/tail; the door cannot stream (`crates/maos-control/src/lib.rs:233-253,1960`). No `--pid` filter. `--spirit worker` switches to the payload-less FR4 projection. No per-agent "session" object ties instruction → output → completion → distillate together (no stable `task_id`; `analysis-maos-coding-delegation-ux-2026-07-17.md:49-57`; ADR-046 `:38-40`). |
| **(c) Steer / message an agent** | Orchestrator only: `maosctl orchestrator queue --spirit orchestrator "<text>"` → daemon buffer + Approval Decision Log (`operator_door.rs:1211-1261`). `maos shell` `@hello-spirit` (`crates/maos-shell/src/lib.rs:500-537`). | The buffer is **never drained** in production (rg = 0; 19-2 backlog, `epic-19-founder-loop-w4.md:175`). `cli.rs:825,842` docs still say "only `hello-spirit`". Workers **cannot** be messaged: T3 refuses stdin postures (`worker_spawn.rs:665-667`; ADR-061 `:35-37`). The shell knows only `hello-spirit` / canned `butler` (`crates/maos-shell/src/lib.rs:509-536`). No `@orchestrator` (FR14 `prd/functional-requirements.md:48`). |
| **(d) Pause / halt / resume / kill / unload / upgrade** | Door verbs `pause`, `resume`, `unload`, `uninstall`, `start`, `posture`, `upgrade` (hot-swap), `hot-swap-precheck`, `revoke-token` (`crates/maos-control/src/lib.rs:1285-1294,1430-1525`). `unload` of a Worker kills it (`operator_door.rs:371,590-592`). Unloading the Orchestrator mid-epic stops dispatch (`orchestrator_root.rs:196-200`). | Pausing a Worker CLI on the Signals/NamedPipe channel is a **no-op** (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:1138-1151`). Resume re-enqueues the buffered instructions but nothing consumes them (`scheduler_loop.rs:484-499`). FR51(c) "recall pending buffered actions" has no route (`sprint-status.yaml:288`). Cold / cross-form swap: 19-5 backlog. No "halt this whole tree" verb; each id gets its own command. |
| **(e) Delegation tree (Orchestrator → Workers, cross-host)** | Implicit only: topology file order (`spirits/topologies/j1-founder-loop-epic.toml`); stdout `delegation_routed` / `delegation_completed` / `distillate_written` (`main.rs:4941,5102,5120`); `task.assign` / `distillate` TL rows; `audit reconcile-hosts` across two signed bundles (`cli.rs:587-620`). The SCB in-flight ledger has `originator_spirit_id` (`crates/maos-domain/src/ports/task.rs:22-23`). | No parent field on the SCB (`control_block.rs:258-287`). No route exposes the in-flight ledger. Delegation frames are journaled with **pid 0** (`sprint-status.yaml:300`; source `deliver_typed(frame, 0, None)` at `crates/maos-bin/src/delegation.rs:452,581,681,750`; ADR-012 `:24`), so a tree rebuilt from the TL loses the emitter. Cross-host children live in another host's TL and door; the join is a post-hoc bundle reconcile. |
| **(f) Cost / tokens** | `maosctl audit cost-reconcile --month` (`cli.rs:712-729`); `cost.attribution` rows from the Inference Port (`inference/mod.rs:414-434`); `context.pressure` / `context.limit` rows (`crates/maos-audit/src/lib.rs:780-782`). | Month granularity, no task axis (20-14 backlog, `epic-20-ship-it-w5.md:233-238`). Principal is a write-target proxy (`cost.rs:53-61`). **Worker CLI tokens are not attributed** (egress rg = 0). No live per-agent token/context gauge on any door route. CPU/storage dimensions are v1.1 (`cost.rs:20-29`). |
| **(g) Approve / consent** | `halt resolve` with three kinds (`cli.rs:779-812`); `skills approve/reject` (`cli.rs:372-389`; door `/v1/skills/decide` `lib.rs:1324-1354`); `a2a repin approve/reject` (cohort door only, `cli.rs:220-241`); `posture` shift (`cli.rs:744-759`); `revoke-token` (`cli.rs:853-862`). Every decision writes an I4 Approval Decision Log row. | Three unrelated pending lists; no unified "needs you" inbox. `halt list` reads the TL, not the daemon's live `HaltRegistry` (no GET halts route; routes at `lib.rs:1100-1427`). The kernel `ApprovalManager` auto-allows per-action prompts (`approval.rs:43-75`). J1's option-pick (a/b/c) and push notification are not wired (19-2 AC4 `epic-19-founder-loop-w4.md:178`; push rg = 0). |

---

## 3. UX flows (what the operator types and sees today)

These come from reading the source; none of them was executed (see §7.3).

**Flow A — Bring up a daemon and find out what is running**
1. `maos init` writes `MAOS_HOME/control.json` (endpoint + 64-hex token, 0600) (ADR-062 `:52-55,156-160`).
2. `maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md` (no `--once` = continuous serving loop; the operator keeps this terminal) (`worker_spawn.rs:78-79`).
3. That terminal shows `{"event":"spirit_loaded",...}` per Spirit (`main.rs:5286`).
4. In a second terminal there is **no `maosctl` verb** that lists Spirits. The operator either reads the first terminal or sends raw HTTP: `GET /v1/daemon` with `Authorization: Bearer <token>` → `{"spirit_ids":[...],"lifecycle_states":[...]}` (`crates/maos-control/src/lib.rs:1101-1118`).
5. `maosctl spirit inspect orchestrator` → `spirit_id/pid/lifecycle_state/posture/...` (`subcommands.rs:2536-2567`).

**Flow B — Watch an epic run (J1 loopback arm)**
1. For each host-bearing entry k, the root checks the dispatch stop (pending halt or Orchestrator unloaded) (`main.rs:4853-4878`).
2. The Orchestrator emits section k's `task.assign`. The first section uses `build_task_assign`; later ones use `followup_dispatch_remote` carrying the prior Distillate ref (`main.rs:4885-4916`). stdout: `delegation_routed` (`main.rs:4941`), `topology_worker_admit` (`:5010`).
3. The Worker spawns under T3. stdout: `host_grant_disposition`, `cli_wrapper_loaded` (`worker_spawn.rs:1149,1162`). Its output lines go to the TL as `cli.subprocess.output` at its pid (`:1183-1189`).
4. The Worker exits. stdout: `cli_wrapper_exit` (line counts, `exit_cause`), then `worker_completion` (label + `last_stdout_tl_ref`) (`:1216-1300`).
5. The root writes the section's Distillate at the Worker's pid. stdout: `distillate_written`, then `delegation_completed` (`main.rs:5102,5120`).
6. After the last entry: `drain` (`main.rs:5355`).
7. The operator **cannot** move from "the Orchestrator" to "its Workers" on any surface except by reading this one stdout stream.

**Flow C — Message the Orchestrator mid-run (FR20)**
1. `maosctl orchestrator queue --spirit orchestrator "also verify the OAuth flow"` → `POST /v1/orchestrator/orchestrator` (`subcommands.rs:2173-2195`).
2. The daemon enqueues and writes the Approval Decision Log row `orchestrator.queue`. The reply is `{"instruction_id":..,"pending":n,"capacity":32}` (`operator_door.rs:1211-1261`).
3. `maosctl orchestrator status --spirit orchestrator` shows `pending` (`operator_door.rs:1744-1761`).
4. **Today nothing more happens.** No code dequeues the instruction (rg = 0). In the target behaviour (19-2 AC1), the instruction is folded into the next `task.assign` after the in-flight `delegation_completed` (`epic-19-founder-loop-w4.md:175`).

**Flow D — Resolve a halt**
1. A pending halt on the Orchestrator's pid prints `{"event":"dispatch_halted","orchestrator_pid":..,"halt_ids":[..]}`, and the run drains (`orchestrator_root.rs:196-225`).
2. `maosctl halt list --spirit orchestrator` reads the TL kind `epistemic.halt` (`subcommands.rs:2035-2039`).
3. `maosctl halt resolve <halt_id> --spirit orchestrator --kind provided-context --text "..."` → `POST /v1/halts/{id}/resolve` (`crates/maos-control/src/lib.rs:1209-1245`).
4. Dispatch does not continue in this run. "Resume dispatches the third section" is 19-4 AC5, which is backlog (`epic-19-founder-loop-w4.md:196`). In `maos shell`, the REPL tick renders the resolution and continues the turn (`crates/maos-shell/src/lib.rs:628-677`).

**Flow E — Pause, then stop, a Worker**
1. `maosctl pause worker` → the SCB becomes `Paused`, a TL `lifecycle.pause` row is written and the `on_pause` hook fires (`scheduler_loop.rs:437-461`). For a T3 Signals-channel CLI the bridge's pause is a documented no-op (`runtime.rs:1138-1151`).
2. `maosctl unload worker` → the scheduler unloads and `stop_worker_after_unload` stops the process (`operator_door.rs:371,583-592`).
3. `maosctl stop` → refused: "use `maosctl pause` (resumable) or `maosctl unload` (terminal)" (`subcommands.rs:67-71`).

**Flow F — Read a Worker's transcript after the fact**
1. `maosctl audit query --frame-kind cli.subprocess.output --range 1h` → NDJSON rows with `spirit_pid`, `intent` and `payload` (`{"stream":"stdout","line":...}`; `worker_spawn.rs:1262-1279`; `crates/maos-audit/src/lib.rs:111-115`).
2. The operator filters by pid themselves (for example with `jq`), because there is no `--pid` flag (`cli.rs:475-514`).

**Flow G — Approve a peer re-pin (cross-host, 19-7 in-progress)**
1. `maosctl a2a repin list` → `GET /v1/a2a/repins` on the cohort daemon's door.
2. `maosctl a2a repin approve <peer> --boot-nonce <decimal>`. A mismatched nonce → 409. More than 300 s → 409 expired. The Approval Decision Log row is written before the pin changes (ADR-062 `:192-220`).

**Flow H — Monthly cost**
1. `maosctl audit cost-reconcile --month 2026-10 --format plain` (pricing from `xtask/provider-pricing.toml`) → rows per (principal, Spirit, provider, model), plus `host-unallocated` (`cli.rs:712-729`; `epic-20-ship-it-w5.md:233`).

---

## 4. Techniques and patterns (named)

1. **Single command port ("one door").** Every mutation is one `OperatorCommand` variant behind `OperatorCommandPort::submit`. A blocking HTTP adapter sits over the async kernel and hands work across through `mpsc` (`crates/maos-control/src/lib.rs:289-298,567-608`). `maos-cli` speaks a data contract (`control.json` + HTTP) and never links the kernel (ADR-062 `:37-40,57-59`).
2. **Operation id plus outcome-bearing completion row.** The port mints an id. A `Queued→Started` CAS races the route budget's `Queued→Withdrawn`, so exactly one wins. One TL row `operator.<verb>.<op>:<outcome>` is written before the reply (`lib.rs:489-502`; `operator_door.rs:1606-1652`; ADR-062 `:168-174`).
3. **Per-Spirit command serialization.** `spirit_key()` makes "which Spirit does this touch" a total function, and host-wide commands carry no key (`lib.rs:399-437`).
4. **Live state from the SCB, history from the TL.** Status routes read `SpiritControlBlock::current_state`, never a journal row ("a journal-only handler must not be able to make this say `Paused`") (`lib.rs:510-512`). The door "never caches or reconstructs" reports (`lib.rs:5-6`).
5. **`None` ≠ empty.** Every read route answers 404 when the object or control is absent, never a fabricated empty value (`lib.rs:78-83,132-141`; `operator_door.rs:1747-1755`).
6. **Safe-point buffered steering (FR20).** The kernel bounds and orders the queue only. The Orchestrator Spirit decides *when* to dequeue: only when `in_flight == 0`, never preempting a delegation. Kernel-internal dequeue is forbidden (`crates/maos-kernel-core/src/orchestrator/mod.rs:10-14`; `spirits/orchestrator/src/lib.rs:8-15,452-454`; `epic-19-founder-loop-w4.md:176`).
7. **Distillate-fed dispatch (FR21).** Follow-ups carry a `PriorDistillateRef`, never raw output. A typed `first_dispatch` / `followup_dispatch` split enforces this, and so does a kernel IAC gate (`spirits/orchestrator/src/lib.rs:16-24`; `orchestrator_root.rs:274-286`).
8. **Deterministic decomposition, model only for judgement.** The epic split, section count and section→Worker binding never come from a completion. Inference is used only for the `dispatch_ambiguity` scalar, and the kernel halts on it through manifest policy (`spirits/orchestrator/src/lib.rs:29-35`; `spirits/orchestrator/manifest.toml:41-58`).
9. **Policy-driven halts with three resolution kinds.** `[epistemic_policy]` thresholds produce kernel halts. Resolution is `provided-context` / `accepted-halt` / `authorized-override` (FR15, `prd/functional-requirements.md:49`; `cli.rs:804-812`).
10. **Worker as a sealed subprocess.** T3 container, `--network=none`, per-spawn egress socket, a scoped bearer and never the raw vendor key, no stdin. Output is evidence (TL rows) and completion is an adapter oracle over those rows (ADR-061 `:21-37`; `worker_spawn.rs:1206-1300`).
11. **Typed-intent consent on delegation.** A cross-host `task.assign` carries `(peer, intent-class)` consent (`development-task:write-workspace`) (ADR-012 `:12-14`; `spirits/orchestrator/src/lib.rs:227,586-594`).
12. **Nonce-echo consent.** The operator approves exactly the observed event (the boot nonce) within a TTL, the approval row is written first, and nothing consents automatically (ADR-062 `:208-220`).
13. **Fail-loud, cite-or-cut planning discipline.** "A control is real, or it is labelled" (confidence rule 11, `epic-19-founder-loop-w4.md:56`). Every AC names file:line and kernel-Δ (`:54`).

---

## 5. Limits, risks, costs

### 5.1 Constraints any new orchestration UX must respect

| Constraint | What it forbids or requires | Evidence |
|---|---|---|
| Kernel embeds no orchestration policy and owns no UI | Coordination patterns and "UI presentation" are Spirit-side. A dashboard or tree view lives in a client or a Spirit, not in `maos-kernel-core`. | `prd/functional-requirements.md:11,14`; `architecture-maos-minimal-opus/4-kernel-design.md:186,189` |
| Kernel KLOC envelope (ADR-069) | `maos-kernel-core` ≤ 23,000 tokei (alarm 22,000). Currently 20,619 with **zero headroom**. Every kernel line needs its own FLAG-Winston grant. 21-5 (moving `orchestrator/`, −1,028 physical lines) is the pre-named relief valve. | ADR-069 `:35-46,56`; `xtask/kloc.toml:264`; `epic-21-multi-host-on-live-substrate-w6.md:46,90-94` |
| Epic-19 crate envelopes | maos-bin measured 30,380 (`kloc.toml:517`, zero headroom) against an E19 envelope of 31,166. maos-control: 1,442 measured + 78 forecast = 1,520 against an envelope of 1,539. maos-cli envelope 6,382 (6,317 measured + 65). maos-shell envelope 626. Each is one ratified envelope per epic, Σ(forecast) × 1.3. A new UX verb or route competes with 19-2…19-6 for these lines. | `epic-19-founder-loop-w4.md:36-48`; `xtask/kloc.toml:484,489,517,691` |
| One daemon, one door, one trust path | The door binds only in `maos run`, `maos shell` and `cohort-a2a-daemon`. `control.json` has one writer (`maos init`). Env overrides are both-or-neither. There is no second port or fallback. | ADR-062 `:48-55,122-127,141-160`; `door_client.rs:3-9,171-199` |
| Door auth and shape | Loopback only; bearer compared in constant time; authentication **before** route and method (anonymous callers learn nothing); fixed 405 bodies with no reflection; ids `[A-Za-z0-9._-]{1,128}`; body ≤ 64 KiB; request/response with `Connection: close`. A streaming or watch route would need an ADR-062 amendment. [INFERENCE: the ADR forbids nothing about streaming explicitly; the server shape simply has none.] | `crates/maos-control/src/lib.rs:233-253,658-668,1023-1029,1144-1146,1960`; ADR-062 `:71-77` |
| `maosctl` dependency boundary | `maos-cli` must not depend on `maos-control` or `maos-kernel-core`. A TUI built inside maosctl speaks HTTP and reads SQLite only. | ADR-062 `:37-40,57-59` |
| Identity-state write path | Cohort membership, peer certificates and rotation change only through signed manifest reissue, never an operator POST. A dashboard cannot "fix a peer". | ADR-062 `:61-64,187-207` |
| TL as audit source of truth; SCB as live truth | Live status reads the SCB. History, transcripts and cost come from the TL. FR4: every external call must be verifiable from the TL. A dashboard must not keep a third state. | `crates/maos-control/src/lib.rs:5-6,510-512`; `prd/functional-requirements.md:27` |
| Log-before-deliver (I2) and approval-captures-intent (I4) | IAC frames are written to the TL before delivery. Every approval writes an Approval Decision Log row with actor, target and intent. Note the observed order: door lifecycle verbs transition first and then journal (`operator_door.rs:583-635`; `scheduler_loop.rs:437-455` ignores TL write errors with `let _`). Re-pin is strictly log-before-act (ADR-062 `:214-216`). | `docs/invariants/I2.md:13,30-32`; `docs/invariants/I4.md:13,30-31` |
| Capability tokens and consent | Every external call is mediated by a kernel token (I1/FR4). Cross-host delegation needs a typed-intent consent envelope. Production consent granters are not wired yet (21-7). | ADR-012 `:12-24`; `sprint-status.yaml:322` |
| T3 isolation for Workers | Workers run only under T3 (`ECliWrapperRequiresT3`). No stdin postures, so no interactive steering of a Worker. No host home is mounted, so a Worker cannot read `control.json` or the bearer. | `worker_spawn.rs:665-667,842-845`; ADR-061 `:21-37`; ADR-062 `:161-167` |
| "A control is real, or it is labelled" | A UX that implies a control (pause, recall, approve) must have a production path, or be labelled as not wired. Example: Worker pause is a labelled no-op. | `epic-19-founder-loop-w4.md:56`; `runtime.rs:1138-1151` |
| Accessibility | `maosctl --plain`, `NO_COLOR` and `TERM=dumb` must produce zero ANSI bytes (NFR-Ops-5) — this binds any TUI. | `crates/maos-cli/src/cli.rs:18-21`; `prd/non-functional-requirements.md:160` |

### 5.2 Current limits and risks (observed)

- **The FR20 steering path can give a false success signal.** `queue` returns `completed` and writes an approval row, but no consumer exists. The 2026-07-17 analysis warned about this exact class ("the enqueue may be journaled even though no coding Worker can receive it") (`analysis-maos-coding-delegation-ux-2026-07-17.md:75-85`). The one-shot detail of that analysis is stale since 16-1 moved queue onto the door (`epic-19-founder-loop-w4.md:175`).
- **The Orchestrator's halt producer is not wired.** `dispatch_ambiguity` / `delegation_confidence` rules exist in the manifest (`spirits/orchestrator/manifest.toml:50-58`), but `judge_ambiguity` has zero production callers. The scalar port is 19-4 AC2 (`epic-19-founder-loop-w4.md:193`). Today an Orchestrator halt comes only from the debug plant (`orchestrator_root.rs:30-34,227-259`) or from an unload.
- **Sequential fan-out only.** The topology loop is a sequential `for`, and Workers run synchronously. FR21 / NFR-Perf-8's "50 concurrent Workers" is not exercised on this path (`epic-19-founder-loop-w4.md:186`; `prd/non-functional-requirements.md:16`). A tree view today would show at most one live Worker per root. [INFERENCE from the sequential loop]
- **Pid-0 attribution** of delegation and host-B frames corrupts any TL-derived lineage (`sprint-status.yaml:300`; source: `deliver_typed(frame, 0, None)` at `crates/maos-bin/src/delegation.rs:452,581,681,750`). The line numbers the tracker quotes are stale at d9bd59c4.
- **Crash → originator delivery is missing.** `task.orphaned` is journaled (`scheduler_loop.rs:716-723`) but never delivered to the originator's mailbox (19-6, `sprint-status.yaml:292`). An Orchestrator therefore cannot react to a dead Worker.
- **Halt continuity across runs is not there.** `dispatch_halted` drains the run, and resume-into-the-same-home is 19-4 AC5.
- **Approval UX is uneven.** Per-action prompts auto-allow (`approval.rs:43-75`). The J1 phone notification is not wired (`maos-notify-push` has no caller).
- **Shell gap.** `maos shell` cannot address `orchestrator` (`crates/maos-shell/src/lib.rs:528-536`), so FR14 "assign a task via the terminal shell" is unmet for J1.

### 5.3 Costs

- **Process and token cost of each story.** E19 has measured velocity and ratified kloc envelopes. Every new verb or route costs a measured kloc grant and a review under the proof contract (`epic-19-founder-loop-w4.md:36-58`).
- **CI cost** is 10.63 job-hours per landing run after 20-15 (`sprint-status.yaml:312`). A UX that needs a new gate or journey test adds to it. [INFERENCE: magnitude depends on design]
- **Kernel lines are the scarcest resource.** Anything needing a new kernel read seam (for example exposing `task_assignments_in_flight`) is a FLAG-Winston grant under zero headroom. Doing it in `maos-bin` + `maos-control` keeps kernel-Δ at 0. [INFERENCE: `operator_door.rs` already holds `scheduler.scbs()` (`:1695,1720`), so it can read the ledger without kernel changes, provided the ledger field is `pub` — it is: `control_block.rs:278`]

---

## 6. Candidate lessons for maos

All lessons are `[INFERENCE]`. They take the operator's two highlighted facts (codex `codex agents` dashboard; omp "left arrow twice shows subagents") as reported user experience, not as verified features. Peer studies 01–03 verify those.

1. `[INFERENCE]` **Ship a list verb before anything fancier.** A `maosctl agents` (or `ps`) built over the existing `GET /v1/daemon` plus `GET /v1/spirits/{id}` is maos-cli-only, kernel-Δ 0. Surface: `crates/maos-cli/src/cli.rs`, `subcommands.rs` (maos-cli envelope 6,382).
2. `[INFERENCE]` **Make parentage first-class data, not a UI guess.** Expose the SCB's `task_assignments_in_flight` (`originator_spirit_id`) through a read route such as `GET /v1/spirits/{id}/tasks`. Surface: `maos-control` route + `OperatorCommandPort` read method + `operator_door.rs`. Prerequisite: fix pid-0 delegation attribution (20-3c member 17).
3. `[INFERENCE]` **"Get to the agent's session" = a pid-scoped TL view.** Add `maosctl audit query --pid` or a `maosctl agents show <id>` that renders `cli.subprocess.output`, `task.assign`, `distillate`, `epistemic.halt`, `context.*` and `cost.attribution` rows for one pid. It reads SQLite like `audit query` does, so there is no door change. Surface: `crates/maos-cli/src/subcommands.rs:2889-3026`, `crates/maos-audit/src/lib.rs`.
4. `[INFERENCE]` **Keep navigation (omp-style "left arrow to children") in a client or in the Observer Spirit, never the kernel.** Architecture already assigns "a live 'what are the agents doing' view in the operator's TUI" to the Observer (§6.5 `:92`), and the kernel non-goal excludes UI (`prd/functional-requirements.md:14`). Surface: a `maosctl` TUI (NFR-Ops-5 bound) or `spirits/observer`.
5. `[INFERENCE]` **Live follow without breaking the door.** Either poll TL rows by `timestamp_ns` cursor client-side (no ADR change), or amend ADR-062 for a bounded streaming read. The first fits "one door, request/response" today. Surface: maos-cli (polling) vs `maos-control` + ADR-062 amendment (streaming).
6. `[INFERENCE]` **Steering in maos is "to the Orchestrator, at safe points", not "into a Worker's prompt".** Finish 19-2 (drain + recall route). Present Worker messaging as "queue for the Orchestrator's next dispatch", because T3 Workers have no stdin by design. Surface: `crates/maos-bin` 19-2 loop; `operator_door.rs` recall route (FR51(c)).
7. `[INFERENCE]` **One "needs you" inbox.** A read-only aggregation of pending halts (from the daemon's live `HaltRegistry`, which needs a GET route), pending skills, pending re-pins and (after 19-2 AC4) pending option-picks. Each item links to its existing decision verb. Surface: `maos-control` read routes; `maosctl` verb.
8. `[INFERENCE]` **Every dashboard action stays a door command.** It gets an `operation_id`, a completion row and per-Spirit serialization, so dashboard actions are auditable exactly like `maosctl`. Surface: no new mechanism; reuse `OperatorCommand`.
9. `[INFERENCE]` **Tree-level verbs are Orchestrator policy.** "Pause the whole tree" should be an Orchestrator-Spirit behaviour (stop dispatch at the safe point, then unload its Workers) triggered by one door command, not a kernel group primitive (§4.0.7). Surface: `spirits/orchestrator`, `orchestrator_root.rs::dispatch_stop`.
10. `[INFERENCE]` **Show cost per agent honestly.** Per-pid token totals from `cost.attribution` plus `context.*` rows. Label Worker-CLI spend as "not attributed" until the egress proxy reports usage. Surface: `maos-audit`, maos-cli; egress (ADR-061) for Worker tokens; 20-14 for the task axis.
11. `[INFERENCE]` **Label what is not real.** Under rule 11, a dashboard must render "pause: no-op for this Worker form" and "queued: not consumed (19-2)" rather than imply control. Surface: client rendering; `runtime.rs:1138-1151`.
12. `[INFERENCE]` **Cross-host: list per door, join on `frame_id`.** A multi-host agents view would query each host's door with its own `control.json`, and correlate children through the shared `task.assign` `frame_id` that `reconcile-hosts` already uses. Surface: maos-cli; `cli.rs:587-620`.

---

## 7. Replication appendix

### 7.1 Pin the source

```bash
cd /home/lunarpulse/dev_ws
git rev-parse HEAD            # expect d9bd59c456a68a9aaddbbee4594796beed288b91
git status --short | wc -l    # expect 0 (citations were read from a clean tree)
# Another machine: obtain the repo (homepage named in spirits/orchestrator/manifest.toml:31 is
# https://github.com/lunarpulse/maos — [INFERENCE] availability not verified), then:
# git checkout d9bd59c456a68a9aaddbbee4594796beed288b91
```

### 7.2 Static checks (read-only; each reproduces a key claim)

```bash
# R1/R2 door shape, auth, routes
sed -n 198,253p crates/maos-control/src/lib.rs
sed -n 1100,1427p crates/maos-control/src/lib.rs | rg -n '\["v1"'
# R3 maosctl verb list (no list/ps/agents verb)
sed -n 40,143p crates/maos-cli/src/cli.rs
# R6 GET /v1/daemon has no CLI consumer  (expect: no output)
rg -n 'v1/daemon|spirit_ids|lifecycle_states' crates/maos-cli/src
# R5/R9 no production consumer of the FR20 buffer or the ambiguity judgement
#   (expect: only the doc comment at orchestrator_root.rs:40)
rg -n 'drain_next|dequeue_at_safe_point|judge_ambiguity' crates/maos-bin/src
# R8 lifecycle order (transition, then journal)
sed -n 583,645p crates/maos-bin/src/operator_door.rs
sed -n 437,505p crates/maos-kernel-core/src/scheduler/scheduler_loop.rs
# R11 Worker T3/no-stdin, pause no-op, worker ids
sed -n 663,667p crates/maos-bin/src/worker_spawn.rs
sed -n 1138,1151p crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs
sed -n 1925,1932p crates/maos-bin/src/supervision.rs
# R10 run-loop events and dispatch stop
rg -n '"event": "' crates/maos-bin/src/main.rs crates/maos-bin/src/worker_spawn.rs crates/maos-bin/src/orchestrator_root.rs
sed -n 4853,4916p crates/maos-bin/src/main.rs
# R12 halts
sed -n 769,812p crates/maos-cli/src/cli.rs
sed -n 196,225p crates/maos-bin/src/orchestrator_root.rs
# no parent on SCB; originator only in the in-flight ledger
sed -n 258,287p crates/maos-kernel-core/src/scheduler/control_block.rs
sed -n 10,24p crates/maos-domain/src/ports/task.rs
# R13 audit query reads SQLite directly; FR4 projection switch
sed -n 2889,3026p crates/maos-cli/src/subcommands.rs
# R14 cost emission only at the Inference Port; egress has none (expect: no output)
sed -n 414,434p crates/maos-kernel-core/src/inference/mod.rs
rg -n 'CostAttribution|usage' crates/maos-bin/src/egress.rs crates/maos-egress/src
# R15 push notifications unwired (expect: no output)
rg -n 'maos_notify_push|PushConfig|NotificationSurface' crates/maos-bin/src
# approvals auto-allow
sed -n 43,75p crates/maos-kernel-core/src/security/approval.rs
# shell addressable Spirits
sed -n 500,537p crates/maos-shell/src/lib.rs
# constraints
sed -n 33,56p docs/adr/ADR-069-kernel-core-envelope-through-v1.md
sed -n 46,77p docs/adr/ADR-062-mutating-operator-surface.md
sed -n 115,223p docs/adr/ADR-062-mutating-operator-surface.md
sed -n 5,14p _bmad-output/planning-artifacts/prd/functional-requirements.md
rg -n '^maos-(kernel-core|control|cli|bin|shell) =' xtask/kloc.toml
# tracker state of the relevant stories
sed -n 287,293p _bmad-output/implementation-artifacts/sprint-status.yaml
sed -n 300p _bmad-output/implementation-artifacts/sprint-status.yaml
```

### 7.3 Live replay (NOT executed in this study — needs a build and a provisioned T3 substrate)

Prerequisites: `cargo build -p maos-bin -p maos-cli` (not run here). The `--epic` run spawns T3 Workers, which need rootless podman, a pinned image and the `MAOS_T3_*` variables (`epic-19-founder-loop-w4.md:24`; `scripts/provision_t3_tests.py`). Leave `MAOS_INFERENCE_MODE` unset: the Orchestrator's port stays unbound (`orchestrator_root.rs:36-40`), and the J1 cassette does not exist yet (`epic-19-founder-loop-w4.md:26`).

```bash
export MAOS_HOME="$(mktemp -d)"                       # isolate state
maos init                                             # writes $MAOS_HOME/control.json
# Terminal 1 — the only live event stream (JSON lines on stdout):
maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md
# Terminal 2 — list agents (raw HTTP; no maosctl verb exists):
EP="$(jq -r .endpoint "$MAOS_HOME/control.json" | sed 's#^tcp://##')"
TOK="$(jq -r .token "$MAOS_HOME/control.json")"
curl -s -H "Authorization: Bearer $TOK" "http://$EP/v1/daemon" | jq .
maosctl spirit inspect orchestrator                   # live SCB view
maosctl orchestrator queue --spirit orchestrator "also verify OAuth flow"
maosctl orchestrator status --spirit orchestrator     # pending stays >0: nothing drains (19-2)
maosctl halt list --spirit orchestrator
maosctl pause worker                                  # SCB Paused; CLI pause is a no-op
maosctl unload worker                                 # stops the Worker process
maosctl stop worker                                   # refused: use pause or unload
maosctl audit query --frame-kind cli.subprocess.output --range 1h | jq -c '{spirit_pid,payload}'
maosctl audit cost-reconcile --month 2026-10 --format plain   # run from repo root (pricing path default)
```

Expected observations, from the code cited above:
- `/v1/daemon` lists `orchestrator`, `architect`, `reviewer`, `worker`, … with their states.
- `orchestrator status` `pending` does not fall after a `delegation_completed`.
- `stop` exits with the refusal text from `subcommands.rs:67-71`.

---

## 8. Open questions

1. **Where does the agents view live?** A `maosctl` TUI (maos-cli kloc, NFR-Ops-5), the Observer Spirit (§6.5 intent), or the ACP editor surface? The ACP server is a separate one-shot root, not one of the three door roots. Does it see the `maos run` daemon's SCBs at all? `[INFERENCE: probably not; not verified]`
2. **Is polling the TL acceptable for live follow?** The alternative is an ADR-062 amendment for a streaming read, and the door today is `Connection: close` with a 5 s read deadline.
3. **Who owns "parent"?** Should parentage come from the SCB in-flight ledger (`originator_spirit_id`), from `task.assign` TL rows (after the pid-0 fix), or from the Orchestrator Spirit's own state exported as a frame?
4. **Should a T3 Worker ever be steerable directly?** ADR-061 refuses stdin postures until "the first production stdin writer". Is direct steering ever in scope, or is Orchestrator-mediated steering (FR20) the maos answer?
5. **What should "pause" mean for a Worker CLI?** Is there a container-level pause (`podman pause`)? The runtime comment calls the container-control path "wired but … best-effort at v0.9" (`runtime.rs:1141-1143`). I did not verify a production caller.
6. **Tree-wide verbs.** Should "pause / stop the epic" be one door command routed to the Orchestrator, or N per-Spirit commands issued by the client? This turns on D-16-1-R per-Spirit serialization (`lib.rs:399-437`).
7. **Cross-host agents view.** One client with several `control.json`s, or a cohort-level read route? ADR-062 forbids identity mutation, not reads, but the cohort read routes are deliberately narrow (`lib.rs:67-84,120-141`).
8. **Live vs post-hoc cost.** Is a running per-agent token gauge in scope before 20-14? How should Worker-CLI vendor spend through the egress proxy be measured (ADR-061 surface; no usage parsing found)?
9. **Approval inbox source.** `halt list` reads the TL rather than the live `HaltRegistry`. Should a "needs you" view read live registries (new GET routes), or the TL (eventually consistent)?
10. **Stale intent docs.** `cli.rs:825,842` ("only `hello-spirit`") and the 2026-07-17 analysis (one-shot queue) describe pre-16-1 behaviour. Should these be corrected before agents learn from them? 19-2 AC1 already plans the `cli.rs` fix.

Corrections 2026-10-28 (05-citation-check.md): `spirit inspect` cite widened to `subcommands.rs:2536-2567` (§1, §2.1, §3; was `2536-2563`); pid-0 delegation defect (§1, §2.1 (e), §5.2) now cites source `crates/maos-bin/src/delegation.rs:452,581,681,750` (`deliver_typed(frame, 0, None)`) and notes the tracker's quoted line numbers are stale at d9bd59c4.
