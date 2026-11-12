# root_shutdown_unload_16_3 ac4_v3b — diagnosis, fix, proof (2026-11-12, FixAc4v3b)

Hosted failing-before excerpts: `hosted-failing-before.md`. Fix diff: `fix.diff` (crates/maos-bin only).

## What the three hosted failures have in common (from the logs)
* Same panic, `root_shutdown_unload_16_3.rs:1436:5`; the process exited with code 1 (the `:1431` exit-code assertion passed), but the root stderr
  contains NO `maos run: interrupted by signal before the --once pass completed` and NO `topology worker interrupted by signal`.
* Root stderr tail is identical in all three: `T3 container <cid> CID kill failed; later cleanup outcome is reported separately`, then
  `worker worker stopped but its output is still held open by a descendant; exiting without the door shutdown and the audit-channel drain`.
  That second line is printed only by the signal listener's grace exit (`supervision.rs` `spawn_signal_listener`, after `await_idle(ROOT_GRACE)` returned false,
  then `std::process::exit(1)`). The main flow's own exit (`main.rs:6187-6190`) never ran.
* No `unresolved container ... residue=` / `container ... residue` line: the CID teardown verdict was final-Ok (removal confirmed). The kill-failure line is printed
  by `ContainerTerminal::teardown` (`child.rs:373-392`) only when `verdict != Err(KillFailed)`, i.e. removal/confirmation succeeded anyway.
* Duration: nextest FAIL [16.433 s] (run 43155033280); the same test passes in 1.4-2.2 s in good runs (run 43186583280 job 131075153157: PASS [2.173 s]).
  Jobs 131173533987 / 131159286775 run the file with `--test-threads=1` (`discipline.yml:1879,1897`), so v3b ran alone: this is not sibling-test CPU contention.

## Product path (code)
SIGTERM -> `spawn_signal_listener` cancels `root_shutdown`, `stop_workers()` (blocking, bounded by `TEARDOWN_BUDGET` 25 s: pidfd SIGKILL of the runtime client,
then `ContainerTerminal::teardown`: payload pidfd kill + `podman kill` (<= `KILL_BUDGET` 5 s) -> confirm -> snapshot (<= `SNAPSHOT_BUDGET` 10 s) -> `rm --force --time=0` (<= `REMOVE_BUDGET` 10 s)),
then `await_idle(ROOT_GRACE = 5 s)`. If a Worker call path is still alive the listener unloads, prints the loss and exits 1.

Defects in that grace exit, independent of WHY the call path was slow:
1. It never nacked the stopped Worker's in-flight task. The only place a PlannedStop Worker's task is dispositioned is `WorkerBinding::finish` -> `BindingAction::DispositionAndUnload`
   (`supervision.rs` `BindingPhase::finish`), which the grace exit abandons. `unload_all_loaded` removes the SCB/ledger without a `task.nacked`. So even with the interrupted line present,
   the test's own `task.nacked` / `topology:1` assertion (`:1465-1474`) would have failed on this path: the AC (hung Worker's task nacked) was violated, not just a string.
2. It never said that the pass was interrupted (only what it was losing).
3. `unstopped_workers()` walked `bindings()` = completed + active views; a finished Worker keeps phase `PlannedStop` and has `child == None`, so it would be reported as
   "could not be signalled (no pidfd)" — a false statement.
4. The `held open by a descendant` sentence asserted a cause nothing had measured.

`CID kill failed` is NOT the failing line's cause: it is a diagnostic. `run_teardown` (`child.rs:448-449`) SIGKILLs the verified payload through its pidfd first and
only then runs `podman kill`; `kill()` (`child.rs:273-280`) latches `kill_failed` whenever that CLI call fails (container already dead, state improper, or the 5 s bound),
and the line is printed even though `killed = kill() || signalled` and removal were confirmed. Kill robustness is already layered (pidfd kill, CLI kill, `rm --force --time=0`);
nothing in `crates/maos-bin` can retry it (the verdict is once-per-CID, kernel-owned), and the kernel crate is out of this change's scope.

## What is NOT proven (needs a hosted run / kernel owner)
* WHY the Worker call path was still alive 5 s after `stop_workers()` returned on the hosted substrate (root stderr has no timestamps; podman is not usable in the local sandbox:
  `podman info` -> `cannot clone: Operation not permitted`, so `require_t3` skips). Candidate (unproven): the test's `wait_for_fixture_child` accepts a CID whose payload pid exists
  (`State.Pid` is set at `podman init`) before `podman start --attach` has actually started the container, so SIGTERM can land on an initialized-not-running container where `podman kill` is
  refused ("CID kill failed") and the attach client's pipes outlive the 5 s bound.
* Whether `kill()` should stop latching `kill_failed` after a successful pidfd kill (kernel `child.rs`) — a kernel-owner decision, deliberately not made here.

## Fix (maos-bin only) — see `fix.diff`
* `supervision.rs`: `WorkerSupervisor::disposition_stopped_unfinished()` (idempotent with `finish` through the `dispositioned` latch) called by the grace exit BEFORE `unload_all_loaded`;
  the grace exit prints `maos run: interrupted by signal before the --once pass completed` (the same sentence main's own exit prints) when the root is a `--once` pass
  (`RootSupervision::declare_once_pass`, set in `main.rs` right after `arm`), else `maos run: interrupted by signal`; `unstopped_workers()` reports only ACTIVE bindings;
  the per-Worker line states what is known (call path did not finish within 5 s; output *may* be held by a descendant). Exit code stays 1, `ROOT_GRACE` stays 5 s, nothing retried.
* `main.rs`: `armed_supervision` + `declare_once_pass()` when `run.once`.
Why this is not test-weakening: the test is unchanged; its assertions (interrupted line, `task.nacked` of `topology:1`, unload rows, receipt, architect unloaded, reviewer never loaded, exit 1)
state what the product promises on EVERY interrupted `--once` exit, and the grace exit previously broke two of them.

## Local proof (observed)
```
    Starting 15 tests across 1 binary
        PASS [   0.004s] ( 1/15) maos-bin::root_shutdown_unload_16_3 ac4_v2_butler_once_receipts_the_raised_halt_and_drains
        PASS [   0.004s] ( 2/15) maos-bin::root_shutdown_unload_16_3 ac4_v3_topology_once_unloads_every_loaded_spirit
        PASS [   0.004s] ( 3/15) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5c_cohort_shutdown_stops_worker_container_descendants
        PASS [   0.004s] ( 4/15) maos-bin::root_shutdown_unload_16_3 ac4_v1_serving_butler_receipts_the_raised_halt_on_sigterm
        PASS [   0.004s] ( 5/15) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5b_sigterm_during_daemon_startup_exits_through_its_teardown
        PASS [   0.005s] ( 6/15) maos-bin::root_shutdown_unload_16_3 ac4_v4b_grandchild_is_stopped_with_its_worker_container
        PASS [   0.005s] ( 7/15) maos-bin::root_shutdown_unload_16_3 ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker
        PASS [   0.005s] ( 8/15) maos-bin::root_shutdown_unload_16_3 ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan
        PASS [   0.005s] ( 9/15) maos-bin::root_shutdown_unload_16_3 ac3_operator_door_unload_stops_a_live_worker_once
        PASS [   0.006s] (10/15) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5_cohort_sigterm_with_live_worker_unloads_and_drains
        PASS [   0.156s] (11/15) maos-bin::root_shutdown_unload_16_3 cohort::declared_capable_cohort_worker_prerequisite_never_green_skips
        PASS [   0.182s] (12/15) maos-bin::root_shutdown_unload_16_3 harness_owns_grandchildren_before_discovery_even_when_startup_panics
        PASS [   0.210s] (13/15) maos-bin::root_shutdown_unload_16_3 every_audit_sender_release_joins_the_yank_owner
        PASS [   0.353s] (14/15) maos-bin::root_shutdown_unload_16_3 ac4_v8_one_shot_acp_server_has_no_listener_and_dies_by_signal_15
        PASS [   0.430s] (15/15) maos-bin::root_shutdown_unload_16_3 cohort::cohort_sigterm_without_worker_joins_local_yank_timer_and_drains
     Summary [   0.430s] 15 tests run: 15 passed, 0 skipped
```
(T3 tests print PASS at ~0.005 s = `require_t3` skip: no provisioned T3 runtime locally.)

| check | command | result |
|---|---|---|
| focused test | `env -u CI -u GITHUB_ACTIONS cargo nextest run -p maos-bin --locked --test root_shutdown_unload_16_3` | exit 0, 15 passed (T3 tests skipped locally); with CI set they fail `MAOS_T3_RUNTIME must select the provisioned runtime` (declared-capable, pre-existing) |
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy -p maos-bin --locked` (`-D warnings` stops on pre-existing rustc 1.98 lints in maos-spirit-abi, 4 errors) | exit 0; 0 warnings inside the changed hunks |
| check-serde-error-handling | xtask `--json` | exit 0, `passed:true`, 0 violations |
| check-kernel-baseline | xtask `--json` | exit 0, 26062 == 26062 |
| check-service-boundary | xtask `--json` | exit 0, passed |
| invariant-lock | `--changed-files` supervision.rs, main.rs | exit 0, passed |
| kloc-check | xtask `--json` | exit 1 (expected): maos-bin measured 38706 vs pin 38,677 (+29); aggregate 203383 vs 203,355; `scripts/repin.py` needed |

## Correction 1 (FixAc4v3b2) — GPT review P2 + fixture readiness

**P2 (claimed ≠ completed).** `BindingCore::disposition` sets `dispositioned` under the binding mutex (the CLAIM) and runs the executor after releasing it, so the
grace-exit fallback could observe the latch, skip, unload and `process::exit(1)` before `finish`'s `task.nacked` insert landed. Fix (`supervision.rs`, maos-bin only):
* `BindingState.dispositions_in_flight: u32` + `BindingCore.disposition_done: Condvar`. `disposition()` increments the counter at the claim, under the same lock; a drop guard
  (`DispositionInFlight`) decrements it and `notify_all`s when the executor returns — on every exit, a panicking executor included.
* `BindingCore::await_disposition(deadline)` waits (`wait_timeout`, poison-tolerant like `lock()`) until the counter is zero or the deadline passes.
* `WorkerSupervisor::disposition_stopped_unfinished(bound) -> Vec<String>`: for each ACTIVE `PlannedStop` binding, `disposition()` (claim-or-observe: runs the write itself when it wins the
  claim, no-ops when `finish` holds it) then `await_disposition(shared deadline)`; returns the spirit ids whose claimed write had not completed.
* Single-execution is unchanged (the `dispositioned` latch still decides who writes; no second `task.nacked`).
* Listener: runs the sweep in `tokio::task::spawn_blocking` (it blocks on a `Condvar`) with `ROOT_GRACE`; an unconfirmed Worker prints
  `maos: worker <id>'s task disposition was still being written 5s after the stop; exiting anyway, its task.nacked may be lost`; a failed sweep task prints a truthful line; exit stays 1.
* Comments that said `finish` "never runs" are rewritten (it may be running or resume).

**The bound: `ROOT_GRACE` = 5 s, shared by the whole sweep.** The awaited operation is one synchronous Transparency Log SQLite insert (milliseconds), so a 5 s wait means the write is
stuck, and a hung shutdown is the one outcome the fallback exists to prevent. Worst case fallback = 5 s (existing grace) + 5 s (sweep) + unload ≈ 10 s + unload, far under the test's 45 s exit wait.
Reusing the existing constant adds no new tunable.

**Test readiness.** `inspect_payload_pid` (the only inspect, used by `fixture_children_of` → `wait_for_fixture_child[_with]`) now inspects `{{.State.Pid}} {{.State.Running}}` in ONE call and returns
`Err` ("not running yet") unless `Running == true`, so the helper retries (existing `STARTUP_TIMEOUT`/`LISTEN_TIMEOUT`, 50 ms poll) until the container is running with its cgroup/start-time identity.
Other users of the helper (v4, ac3-door-unload, v4b, v5, v5c cohort paths) all wait for a `hang`/`hang-with-grandchild` fixture that stays running, so the stricter precondition
only removes the init-but-not-started window; none relies on an initialized-not-running container. (v5b does NOT use this helper: it signals at the armed marker during daemon startup, so
its meaning is unchanged — corrected in Correction 2; Correction 1 originally listed it among the helper's users.)

**Deterministic unit witness: not possible without a new production hook.** The claimed-not-completed window sits between the latch and `KernelBindingExecutor::disposition`; reaching it needs a custom
`BindingExecutor` (or a pause seam in `disposition`) inside a `WorkerSupervisor`, which constructs its own executor privately and exposes no seam there (`SupervisorSeams` has none for the disposition),
and inline `#[cfg(test)]` code is kloc-charged. Adding a hook was out of scope. The race remains argued by construction, not witnessed.

### Proof (observed)
| check | command | result |
|---|---|---|
| focused | `env -u CI -u GITHUB_ACTIONS cargo nextest run -p maos-bin --locked --test root_shutdown_unload_16_3` | exit 0, 15 passed; T3 tests skip locally (`require_t3`, ~5 ms) — the strengthened readiness and the fallback are NOT exercised here |
| fmt | `cargo fmt --all -- --check` | exit 0 (after one rustfmt reshaping of the `spawn_blocking` call) |
| clippy | `cargo clippy -p maos-bin --locked --all-targets` | 0 diagnostics inside the changed hunks (existing warnings elsewhere in both files) |
| check-serde-error-handling / check-kernel-baseline / check-service-boundary | xtask `--json` | all exit 0, passed (kernel 26062 == 26062) |
| invariant-lock | `--changed-files` supervision.rs; root_shutdown_unload_16_3.rs | both exit 0, passed |
| kloc-check | xtask `--json` | exit 1: maos-bin **38,758** vs pin 38,706 (+52); aggregate **203,435** vs hardfail 203,384 — re-pin needed (kloc.toml untouched) |

## Correction 2 (FixAc4v3b3) — GPT review 2: end-to-end sweep deadline; honest messages; record fixes

**P2 (the sweep itself was unbounded).** Correction 1 awaited a `spawn_blocking` task with no timeout; its own deadline started only once the blocking pool ran it, and
`core.disposition()` (scheduler/ledger locks + a journal insert) ran synchronously before the timed Condvar wait. A saturated blocking pool or a stuck write would
therefore have stalled the listener before its diagnostic, the unload and `process::exit(1)`.

Fix (`supervision.rs`, maos-bin only):
* `disposition_stopped_unfinished(deadline: Instant)` now takes an absolute deadline instead of a `Duration` (single caller).
* The listener fixes `started = Instant::now()` immediately before `spawn_blocking`, passes `started + ROOT_GRACE` as the sweep's deadline, and awaits the join handle under
  `tokio::time::timeout(ROOT_GRACE, ..)`: ONE budget from the moment the listener starts the sweep, so pool queueing counts against it. (No new constant.)
* Listener-side timeout: it stops waiting and prints `maos: the task disposition sweep did not finish within 5s of starting it (<n>ms elapsed); exiting anyway, task.nacked rows for
  unknown Workers may be lost. The sweep cannot be aborted; exiting is the bound`, then unloads and `process::exit(1)` as before. The ids are "unknown" because the sweep returns them
  only on completion. A blocking task cannot be aborted; it is left running and `process::exit` is what ends it.
* Unchanged: the `dispositioned` latch (single execution), the `dispositions_in_flight` counter, the Condvar wait and its guard.
* Honesty (review note 5): the per-Worker line no longer says "5s after the stop" (the first 5 s grace had already elapsed and the sweep budget starts later). It now reads
  `... was still being written when the 5s disposition budget ended (<n>ms after the sweep started); exiting anyway, its task.nacked may be lost`, with `<n>` the measured elapsed.
  (Residual: if the sweep's Condvar deadline and the listener's `timeout` expire at the same instant, the listener may take the timeout branch and print the generic line instead of
  naming the Worker. Both lines state a possible loss truthfully.)

**Budget arithmetic from SIGTERM (corrects Correction 1's "≈10 s + unload").** Nominal worst case, each term an upper bound of its own bounded step:
`stop_workers()` (T3 container teardown, `TEARDOWN_BUDGET` = 25 s, `crates/maos-kernel-core/.../t3/child.rs:30`) + `await_idle(ROOT_GRACE)` 5 s + sweep budget `ROOT_GRACE` 5 s + unload
(synchronous journal writes, unbudgeted) = **35 s + unload**. The tests' exit wait is 45 s (`root_shutdown_unload_16_3.rs:1271,1442`), so the nominal budgets leave ~10 s for the unload,
the stderr rendering and process exit. That is a margin from nominal budgets, NOT a proven bound: unload is not itself timed out, and `stop_workers()` teardown is bounded by the
kernel's budget, not by anything in this change. The observed hosted failure took 16.4 s total, i.e. well inside this envelope. What this fix does establish is that the fallback's
own added wait (the sweep) can no longer exceed `ROOT_GRACE` regardless of pool or lock state.

**Record fix.** Correction 1 listed v5b among the users of the `Running == true` readiness helper. That is wrong: v5b signals at the armed marker during daemon startup and does not
call `wait_for_fixture_child`/`inspect_payload_pid`. Its semantics are unchanged by the readiness change. Line 105-107 above is corrected.

**Not witnessed.** No test exercises pool saturation or a stuck disposition; hosted T3 behaviour and actual shutdown duration remain unobserved here (T3 skips locally).

### Proof (observed, Correction 2)
| check | command | result |
|---|---|---|
| focused | `env -u CI -u GITHUB_ACTIONS cargo nextest run -p maos-bin --locked --test root_shutdown_unload_16_3` | exit 0, 15 passed; T3 tests skip locally (`require_t3`, ~20-30 ms) — the fallback, the timeout branch and the new messages are NOT exercised |
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy -p maos-bin --locked --all-targets` | Finished; 0 diagnostics at supervision.rs lines in the changed hunks (existing warnings elsewhere) |
| check-serde-error-handling / check-kernel-baseline / check-service-boundary | xtask `--json` | exit 0, passed (kernel 26062 == 26062) |
| invariant-lock | `--changed-files crates/maos-bin/src/supervision.rs` | exit 0, no touched invariants |
| kloc-check | xtask `--json` | exit 1: maos-bin **38,767** vs pin 38,706 (+61); aggregate **203,444** vs hardfail 203,384 — re-pin needed (kloc.toml untouched) |
(Correction 1's statement "Worst case fallback ≈ 10 s + unload" at line 100 is superseded by the arithmetic above: 35 s + unload from SIGTERM.)
