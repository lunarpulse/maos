# 19-2 creation — cites re-measured at `d9bd59c4` (2026-10-28)

Method: direct `read` of every range at HEAD; `git grep -n` for each anchor string at `f889b3e4` (the §R baseline), `f34f6b4e` (19-1/19-7 creation) and `d9bd59c4` (HEAD); `git diff --stat f889b3e4 d9bd59c4 -- <path>` for unchanged files. No cargo run. Tree clean except the sibling creators' untracked evidence dirs.

## Epic §19-2 (`epics/epic-19-founder-loop-w4.md:171-178`) and its §R rows

| Cite (where) | OLD | NEW @d9bd59c4 | How measured |
|---|---|---|---|
| serving-loop announce (AC1 `:175`, R19-13 `:80`) | `main.rs:5286-5290` | `crates/maos-bin/src/main.rs:5469-5475` (announce `:5470-5471`) | `git grep 'entering continuous serving loop'`: f889b3e4 `:5288`, f34f6b4e `:5288`, HEAD `:5471` (19-1 +183) |
| `--once` arm (AC1, R19-13) | `main.rs:5115` | `main.rs:5298` (`if run.once \|\| topology_load.is_err()`) | `git grep 'if run.once \|\| topology_load.is_err'`: `:5115` → `:5115` → `:5298` |
| the dispatch itself (premise, not in the epic) | — | runs INSIDE the topology load `for` (`main.rs:4687-5294`, async block `:4651-5297`) — before the `--once`/serving branch (`:5298`); the serving loop dispatches nothing (comment `:5465-5468`) | read |
| `maosctl orchestrator queue` (AC1, R19-13) | `crates/maos-cli/src/cli.rs:776-783` | `cli.rs:824-831` (`Queue`), enum `:822-837` | `git grep 'pub enum OrchestratorOp'`: `:774` → `:774` → `:822` (19-7 `a2a repin` +48) |
| "only `hello-spirit`" doc (AC1) | `cli.rs:777` | `cli.rs:825` (also `PauseArgs` `:842`) | same grep |
| `orchestrator status` (AC1, R19-13) | `cli.rs:785-788` | `cli.rs:833-836` | read |
| `OrchestratorBufferRegistry` (AC1, kernel para `:34`, R19-13) | `main.rs:2270-2271` | `main.rs:2292-2293` | `git grep 'OrchestratorBufferRegistry::new'`: `:2271` → `:2271` → `:2293` |
| door registry field / ctor arg (AC1, R19-13) | `operator_door.rs:213,263` | `crates/maos-bin/src/operator_door.rs:213,266` (enqueue handler `:1208-1261`, status `:1744-1760`) | `git grep orchestrator_registry`: `:263` → `:266` (19-7 +3) |
| `Orchestrator::drain_next` (AC1, kernel para `:34`) | `spirits/orchestrator/src/lib.rs:219-227` | `spirits/orchestrator/src/lib.rs:483-491` (gate `is_safe_point` `:452-454`, `begin_/complete_delegation` `:466-476`) | `git grep 'pub fn drain_next'`: `:219` → `:219` → `:483` (19-1 +264) |
| `dequeue_at_safe_point` (AC1) | `buffer.rs:62` | holds (`crates/maos-kernel-core/src/orchestrator/buffer.rs:62-67`); non-test callers = 0 (`git grep` over `crates/*/src`, `spirits/*/src`) | read + grep |
| Ctx ABI (AC2) | `crates/maos-spirit-abi/src/ctx.rs:58` | holds (`pub struct Ctx`) | read; `git diff --stat f889b3e4 d9bd59c4 -- ctx.rs` empty |
| on_idle doc (AC2) | `spirits/orchestrator/src/lib.rs:143-147` | `lib.rs:317-322` | `git grep 'The LIVE buffer-drain'`: `:144` → `:144` → `:318` |
| inert on_idle (AC2) | `lib.rs:149-157` | `lib.rs:323-331` | `git grep 'fn on_idle'`: `:149` → `:149` → `:323` |
| buffer contract (AC2, kernel para) | `buffer.rs:59` | holds (`:58-61`) | read; kernel orchestrator/ unchanged since f889b3e4 |
| FR51(c) text (AC3) | `prd/functional-requirements.md:54` | `:55` (FR20 is `:54`) | `git grep '^- \*\*FR51:'`: f72b557b `:54`, f889b3e4 `:55`, HEAD `:55` — stale before §R |
| `recall_all_pending` (AC3) | `buffer.rs:72` | holds (`:72-80`, `pub`) | read |
| resume re-enqueue (AC3, R19-14) | `scheduler_loop.rs:484-497`, call `:489` | holds | read; file unchanged since f889b3e4 |
| deferred row (AC4, R19-15) | `deferred-work.md:1277` | `deferred-work.md:1195` | grep `Owner: \`19-2-fr20-enqueue-door-and-safe-point\`` at HEAD (19-1 creation already noted `:1195`) |
| `handle_option_pick` (AC4, R19-15, R19-P10) | `spirits/butler/src/lib.rs:1486-1563`, signature `:1486-1491` | holds | `git grep 'pub fn handle_option_pick'` = `:1486` at all three revs |
| shell canned pick (AC4, R19-15) | `crates/maos-shell/src/lib.rs:509-526` | holds | read; `git diff --stat f889b3e4 d9bd59c4 -- crates/maos-shell/src` empty |
| "class Spirits are moved into `scheduler.load`" (AC4, R19-P10 `:106`) | `main.rs:4603-4608` | **gone** — the comment was deleted by 19-1 (`6f872383`). Fact at HEAD: class Spirits are wrapped by `make_spirit_obj` and admitted by `admission::load_admit_start` (topology `main.rs:5163-5281`; standalone `:5696-6104`); a topology REFUSES Butler/Researcher (`:5251-5256`); the standalone Butler arm keeps a clone `butler_pass_handle` (`:5678`, set `:5869`); `ButlerSeam::scene_butler` is a retained handle set by `attach` only when a principal binder exists (`spirits/butler/src/lib.rs:320-336`, set `:389-391`) and read by the scene driver (`main.rs:6259`) | `git show f889b3e4:…main.rs` `:4602-4612` vs `grep` at HEAD |
| deferred row's own cite (`deferred-work.md:1195`) | `spirits/butler/src/lib.rs:1039-1087` | `:1486-1563` (moved before f889b3e4 by 18-4) | read |

## Other cites the story uses (new, read at HEAD)

| Anchor | Cite |
|---|---|
| door bind (before the topology load) | `main.rs:3412-3414` (door roots), `OperatorDoor::new` `:3530-3550`, `bind_with_commands` `:3554-3563`, announce `:3594` |
| maos-control routes | `OperatorCommand` `crates/maos-control/src/lib.rs:296-397`, `spirit_key` `:405-437`, `route_budget` `:445-462` (`_` arm `:460`), `OperatorOutcome` `:472-486`, `["v1","orchestrator",id]` `:1246-1284`, `["v1","spirits",id,verb]` `:1199-1208`, `spirit_command` `:1430+`, `post_guard` `:1644`, `validate_id` `:1658`, `MAX_BODY_BYTES` `:241`, server threads `:680-733` |
| door dispatch | `operator_door.rs` dispatch arm `:478-480`, `command_verb` `:1795-1824`, `BinPrivateOps` `:54`, impl `main.rs:9419`, `SpiritBuildDeps.butler_seam` `main.rs:939`, `:3523` |
| dispatch loop | emitter `main.rs:4660-4663`; plan `:4669-4677`; per-section goal `:4821-4852`; `dispatch_stop` `:4858-4879`; frame build `:4885-4917`; `begin_delegation` `:4919`; Worker run `block_in_place` `:5028-5048`; distillate `:5089-5111`; `complete_delegation` `:5115`; `delegation_completed` `:5117-5125`; topology `drain` event `:5352-5360` |
| orchestrator_root | `HALT_PLANT_ENV` `crates/maos-bin/src/orchestrator_root.rs:34`, `construct_at_admitted_pid` `:65-83`, `dispatch_stop` `:201-225`, `plant_test_halt` `:230-259`; env row `env_contract.rs:197` |
| goal → argv | `worker_spawn.rs:787-790` (`worker_cli.argv(task)`); fixture echoes it `spirits/worker/src/bin/worker-cli-fixture.rs:109-113` |
| Butler | `ButlerSeam` `spirits/butler/src/lib.rs:300-327`, `attach` `:348-393`, `record_decision` `:1259-1273`, `refresh_record` per pass `:990-994`, `block_on_sync` (noop waker) `:1919-1933`, `acceptance_binder` `crates/maos-bin/src/butler_root.rs:125`, boot seam `main.rs:3987-4053` (acceptance only from the scene `:4037-4046`) |
| shell | `ShellHost` `crates/maos-shell/src/lib.rs:256`; maos-bin impl `crates/maos-bin/src/shell_host.rs:86` (hello-spirit + halts; no Butler); canned-string tests `crates/maos-bin/tests/butler_8_14b.rs:115-157`, `crates/maos-shell/tests/shell_halt_repl_16_2.rs:541-549` |
| kernel journal shapes | `journal_orchestrator_queue` `crates/maos-kernel-core/src/orchestrator/mod.rs:38-58`; stale doc `:16-17` (`MAOS_ONE_SHOT=orchestrator-queue`, deleted by 16-1); writer `insert_approval_decision` `crates/maos-iac/src/adapter/transparency_log.rs:2168` (maos-iac, not kernel) |
| FR51(a)(d) routing | `routing-audit-2026-10-22.md:40,172`; tracker `sprint-status.yaml:499-501` ("The work itself is 19-2's"); `epic-17-retro-2026-10-24.md:32`; `requirements-inventory.md:600`; 16-1 D-16-1-U `16-1-daemon-post-surface-and-verb-retarget.md:178`; FR51 phase v1.0 `functional-requirements.md:118`; stale `--help` claims `crates/maos-cli/src/cli.rs:90-100` |
| CI | `workspace-test-suite` `.github/workflows/discipline.yml:5871`, T3 `:5914`, nextest `:6017`; `check-j1-loopback-delegation` `:3334`, T3 `:3346`; `J1_TEST_SUFFIXES` `xtask/src/check_j1_loopback_delegation.rs:129` |
| ADR-062 | Decision `docs/adr/ADR-062-mutating-operator-surface.md:46-80`, Consumers `:82-89` (19-2 "queue mutation" `:87`), Amendment 16-1 `:115-181`, Amendment 19-7 `:183-223` |

## Kernel

- `crates/maos-kernel-core/src/**.rs` physical lines (git-tracked, `xargs cat | wc -l`) = **26378** = pin `xtask/kernel-core-baseline.toml:505`; tokei 20619 = `xtask/kloc.toml:264`.
- `git diff --stat f889b3e4 d9bd59c4 -- crates/maos-kernel-core/src/orchestrator crates/maos-kernel-core/src/scheduler/scheduler_loop.rs` empty.

## Amendment — FR51(a)(d) surfaces, read at `9ce0e47a` (crate source = `d9bd59c4`; `git diff --stat d9bd59c4 9ce0e47a -- crates spirits xtask/src` empty)

| Anchor | Cite | How measured |
|---|---|---|
| pause / resume / unload revoke | `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:436-461`, `:463-505`, `:564-565`; scheduler holds `capability` `:84` | read |
| hook dispatch cannot cancel | `scheduler/hook_dispatch.rs:281-289` (`fire_on_idle`, `HaltPending`), `:546-613` (`spawn_blocking` under `timeout`, `Ctx::for_rust_inproc_hook` `:574-577`) | read |
| `Ctx` cancellation is `NeverCancel` | `crates/maos-spirit-abi/src/ctx.rs:59` (field), `:174-175` (doc), `:199-211` (ctor); trait `cancellation.rs:35-60`, `NeverCancel` `:104-110` | read |
| no Spirit implements on_pause/on_resume | `grep -rn "fn on_pause\|fn on_resume" spirits/*/src crates/maos-spirit-hello/src crates/maos-digest/src` = 0 | grep |
| known gap labels | `crates/maos-bin/src/hook_pump.rs:22-24`; `crates/maos-bin/tests/hook_pump_17_4.rs:742-744` | read |
| capability issuance / verify / revoke | `crates/maos-kernel-core/src/capability/mod.rs:124-135` (struct), `:152-177` (ctor), `:190-251` (`issue_with_mediation`), `:305-314` (`verify_and_audit`), `:384-387` (`revoke`, `Operator`), `:412-414` (`revoke_all_for_pid`) | read |
| revoke-all reason | `crates/maos-capability/src/cap_tokens/mod.rs:93-107` (`RevokeReason`), `:306-317` (`revoke_all` → `SpiritUnload`) | read |
| verify outcome / TL intent | `crates/maos-capability/src/cap_audit/mod.rs:103-136` (`_ => UnknownToken` `:132`); `crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs:54-57` (`cap.verify.ok`/`fail` only) | read |
| adapters check before, never after | `crates/maos-kernel-core/src/inference/mod.rs:227-249`, `:253-447` (call `:299-311`, TL `:358-365`, return `:446`); `crates/maos-kernel-core/src/mcp/mod.rs:47-85`, `:89-138` (TL `:118-125`, return `:137`) | read |
| `CapError` | `crates/maos-domain/src/ports/capability.rs:113-147`; exhaustive test match `crates/maos-wasm-host/tests/equiv_harness.rs:909` | read + grep |
| Worker container control | `crates/maos-kernel-core/src/security/sandbox/t3/child.rs:26,30` (budgets), `:150-163` (`inspect`), `:271-284` (`kill`); `invoke_bounded` `t3/spawn.rs:223-238`; cgroup limits `t3/argv.rs` `--cpus/--memory/--pids-limit` | read |
| Worker binding | `crates/maos-bin/src/supervision.rs:786-790` (`stop_session`), `:1821-1825` (`stop_binding`), `:2009-2017` (`bind_token` records the cap token) | read |
| door lifecycle / revoke | `crates/maos-bin/src/operator_door.rs:404-435` (dispatch), `:541-646` (`lifecycle_command`, `stop_worker_after_unload` `:591`), `:1263-1313` (`revoke_token_command`); root hook install `main.rs:4577-4588`, shutdown token `:4589` | read |
| Worker spawn token | `crates/maos-bin/src/worker_spawn.rs:974-988` | read |
| "FR51" test that measures journal writes | `crates/maos-kernel-core/tests/nfr_perf_4_pause_resume_latency.rs:1-95` (thresholds `:18-24`) | read |
| fixture heartbeat mode | `spirits/worker/src/bin/worker-cli-fixture.rs:75-79` (`hang-chatty`) | read |
| FR51 text / phase | `prd/functional-requirements.md:55`, `:118` | read |
| kloc pins touched by alternatives | `xtask/kloc.toml:265` maos-capability 1432 (exact), `:270` maos-spirit-abi 3000 (ceiling; measured 1042), `:425` maos-domain 9975 (exact) | read + tokei |
