# 19-6 creation — cites re-measured at `d9bd59c4` (2026-10-28)

Method: direct `read` of every range at HEAD `d9bd59c4` (clean tree, `git status --short` empty); `git grep -n` across
`f889b3e4` / `f34f6b4e` / `d9bd59c4` for the moved resolver row. No cargo run (another cargo process held the host).

## Epic / tracker / routing-audit cites (OLD → NEW)

| Cite (where) | OLD | NEW @d9bd59c4 | How measured |
|---|---|---|---|
| production `NullReplicaResolver` (tracker `sprint-status.yaml:292`; routing audit `:21`, `:177`) | `main.rs:2795-2796` | `crates/maos-bin/src/main.rs:2997-2998` (detector `:2999-3010`, scheduler `:3016`) | read; `git grep -n 'Arc::new(maos_domain::supervision::NullReplicaResolver)'` → f889b3e4 `:2970`, f34f6b4e `:2970`, d9bd59c4 `:2998` (moved +28 by 19-1/19-7, commits `6f872383`..`d9bd59c4`) |
| same (epic §R R19-24 `:91`, §R kloc row `:131`) | `main.rs:2970` (was `:2795-2796`) | `main.rs:2997-2998` | as above |
| 16-3 "journaled, never mailbox-delivered" (tracker `:292`; audit `:21,:176,:186`) | `16-3…md:35-37` | holds (`16-3-subprocess-crash-to-handle-crash.md:35-37`; residual row `:429`) | read |
| NFR-Rel-1 corpus (audit `:186`) | `16-3…md:297` | holds (D-16-3-K, the corpora row) | read |
| "19-1's loop treats a non-Completed Worker as a run error" (assignment brief: `main.rs ~4860-4930`) | `main.rs:4860-4930` | the refusal is `main.rs:5072-5078`; `:4853-4879` is the pending-halt check, `:4880-4919` the frame build + `begin_delegation` | read |
| kernel journals `task.orphaned` (16-3 §11 row 1, `:429`) | `crash_detector.rs:163-170` | holds (step 5 `crates/maos-kernel-core/src/supervision/crash_detector.rs:141-171`) | read |
| dispositions are TL rows at pid 0; `reassign_task_to` enqueues nothing (16-3 `:429`) | `maos-iac adapter.rs:239-301` | holds: `crates/maos-iac/src/adapter.rs:239-304` (`emit_task_complete_nack` `:242-258`, `emit_task_complete_escalated` `:261-280`, `reassign_task_to` `:283-304` — doc says "enqueue a reassign frame", body writes one TL row) | read |
| epic §19-6 section | (assignment brief: "Epic section `### 19-6-…`") | **absent** — the epic has no `### 19-6` (nor `### 19-5`) heading; only the stories row `:156`, the order `:159` and Dependencies `:212` name 19-6 | `grep -n '^### ' epic-19-founder-loop-w4.md` → 19-1 `:161`, 19-2 `:171`, 19-3 `:180`, 19-4 `:188`, 19-7 `:204` |
| deferred-work rows owned by 19-6 | — | none (`grep -n '19-6' deferred-work.md` = 0) | grep |
| PRD FR12 / FR50 / NFR-Rel-1 | — | `prd/functional-requirements.md:41`, `:44`; `prd/non-functional-requirements.md:20` | read |
| requirements inventory routing row | — | holds `requirements-inventory.md:592` | read |

## Code facts the story rests on (all @d9bd59c4)

- Kernel crash path: `CrashDetector::handle_crash` `crash_detector.rs:87-238` — step 5 `task.orphaned` rows `:141-171`, step 6 `enforce_disposition` `:173-179`, `with_replica_resolver` `:74-78`; the budget comment (2 s / 5 s) `:85-86`.
- `enforce_disposition` `crates/maos-kernel-core/src/supervision/disposition.rs:16-68`: reassign arm `:35-50` discards the resolved pid (`_replica_pid`, `:39`) and passes the ORIGINATOR id as the replica (`:41`); keyed by `IntentClass` string (`:37-38`).
- Scheduler on_unload orphan path `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:689-731`; resolver slot `:93`, `set_replica_resolver` `:163-168`.
- Domain: `OnCrashAction` `crates/maos-domain/src/supervision.rs:19-30` (default `Nack`); `ReplicaResolver` `:181-187`; `NullReplicaResolver` `:189-196`; in-src test `:326-329`.
- maos-bin supervision: `WorkerTask::record_ids` `crates/maos-bin/src/supervision.rs:264-279` (delegation originator = `delegation::FROM_SPIRIT`); `BindingPhase::CrashHandled` `:322-323`; binding `disposition()` `:1152-1162`; record built at bind `:1557-1590` (`on_crash_action` from the bundle `:1581-1585`); `spawn_crash_handler` `:2197-2205`; executor `disposition` passes `None` as resolver `:2252-2265` (`None` at `:2263`); root supervisor ctor holds the `NotificationDispatcher` `:2411-2425`.
- Ledger writers in production src: only `KernelBindingExecutor::push_record` `supervision.rs:2072-2091` (+ two smoke arms `main.rs:7292-7293,7345-7346`).
- Loop: `delegation_leg` install `main.rs:2940`; resolver `:2996-3017`; root supervision arm `:4546-4556`; `epic_halted` `:4683`, skip arm `:4759`; halt check `:4853-4879`; `begin_delegation` `:4919`; Worker run `:5028-5048`; non-completion refusal `:5072-5078`; `journal_completion` + `complete_delegation` `:5112-5115`.
- `DelegationLeg` `crates/maos-bin/src/delegation.rs:298-322`; registers `developer-remote` and `orchestrator` `:390-395`; `journal_completion` `:571-589`; `outcome_frame_id` `:791`; `completion_frame` `:1101-1133`; consts `:58-70`.
- Orchestrator: `is_safe_point`/`in_flight`/`begin_delegation`/`complete_delegation` `spirits/orchestrator/src/lib.rs:452-476`.
- 19-1 helpers: `dispatch_stop` `crates/maos-bin/src/orchestrator_root.rs:201-225`; `plant_test_halt` (real `invoke_halt`) `:230-259`.
- Worker verdict: `WorkerNonCompletion::ProcessCrash` `crates/maos-bin/src/worker_cli.rs:46-48`; labels `:87-106`.
- `[on_crash]` parsed at pid 0 `crates/maos-bin/src/worker_spawn.rs:620-639`, into the bundle `:923-927`; no shipped manifest declares it (`grep on_crash spirits/` = 0) ⇒ every J1 Worker is `Nack`.
- Delivery precedents: `IacBusAdapter::deliver_typed` `crates/maos-iac/src/adapter.rs:329-630`; kernel sender `KERNEL_SENDER_SPIRIT_ID = "__kernel"` `crates/maos-iac/src/adapter/mailbox.rs:61`, consent-gate exemption `:338-346`, `prepare` `:324-393`; kernel-sent frame precedent `crates/maos-iac/src/adapter/context_signal.rs:75-87`; recipients table `transparency_log_recipients` `crates/maos-iac/src/adapter/transparency_log.rs:552`, `recipient_resolver` `:685`, `mint_frame_id` `:691`.
- Notifications: `NotificationEvent` `crates/maos-domain/src/notification.rs:29-66` (`AnomalyFlagged` `:50-65`); dispatcher `crates/maos-director-surface/src/notification.rs:49-66`; root wiring `main.rs:2119-2125`.
- Tests: TI-3 `crates/maos-bin/tests/epic_dispatch_19_1.rs:405-490` (asserts `"did not complete"` `:455-458`); escalate vector `crates/maos-bin/tests/worker_supervision_16_3.rs:613-657`; sigkill vector `:493-611` (`sigkill-self` `:512`); corpus `crates/maos-bin/tests/worker_crash_corpus_16_3.rs` floors `:89-94`, `task.nacked` `:401-403`, test `:515`, Standalone tasks `:614,715`; job `nfr-rel-1-crash-detection-2s` `.github/workflows/discipline.yml:1567-1609` (`timeout-minutes: 20` `:1590`, healthy 2m01s `:1573`).
- J1 gate suffixes `xtask/src/check_j1_loopback_delegation.rs:129` (`"_19_1.rs"` enrolled; D-2 digit rule `:843-846`).
- Kernel pins: `xtask/kernel-core-baseline.toml:505` `src_lines = 26378`; content pins `supervision/crash_detector.rs` `:971`, `supervision/disposition.rs` `:972`; tokei `xtask/kloc.toml:264` = 20619; maos-bin `:517` = 30380; maos-iac `:406` = 8270; maos-domain `:425` = 9975; aggregate hardfail `:718` = 189878.
