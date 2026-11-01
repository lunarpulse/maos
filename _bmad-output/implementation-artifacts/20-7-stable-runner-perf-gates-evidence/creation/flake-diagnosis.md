# 20-7 creation — flake families: calibration or defect (read at `a4c00897`)

Method: every failing hosted job named in the tracker, `deferred-work.md` and the creation brief was fetched with the
operator's authenticated `gh` token on this host (`gh api repos/lunarpulse/maos/actions/jobs/<id>/logs`, 2026-11-01;
the 403 recorded at `deferred-work.md:1193` no longer applies here). The failure blocks are in `ci-excerpts/`. The
code paths were read at HEAD. No test was run (no cargo). Each timing below comes from a nextest `PASS`/`FAIL` line
or from a failure message. "[INFERENCE]" marks a mechanism read from code that no log proves.

## Run inventory (corrections to the brief and to the ledger)

| Family | Hosted failure (run · job · job name) | Signature in the log |
|---|---|---|
| S3 | 38807569588 · 116672297542 · workspace-test-suite | `took 10.032837094s (bound 5s, runtime podman version 4.9.3)`; `residue TerminationUnconfirmed`; client `Exited { code: 125 }` |
| S3 | 38987395150 · 117261941488 · diag (bin-delegated) | `10.03602644s`; `TerminationUnconfirmed`; client 125 |
| S3 | 40200028265 · 121189060007 · check-multi-tenant-loom | `10.104222495s`; client 125; no residue line |
| S3 | 40671240744 · 122758268359 · workspace-test-suite | `10.06302991s`; client 125; no residue line |
| S3 | 41243340929 · 124661589790 · check-multi-tenant-loom | `10.039739216s`; `TerminationUnconfirmed`; client 125 |
| S3 | 41281999213 · 124775636112 · workspace-test-suite | `10.052044568s`; `TerminationUnconfirmed`; client 125 |
| — | **41233186774** · 124631445292 · workspace-test-suite | **not S3**: S3 `PASS [1.447s]` in this job; the red is `worker_freeze_19_2.rs:274` "no such table: transparency_log" (the TL-before-table harness race fixed by `714a1428`). The brief's S3 attribution to 41233186774 is wrong. |
| ac4 | 40980187631 · 123818024907 · nfr-rel-11-halt-receipt-999pct | `ac4_v4` `root_shutdown_unload_16_3.rs:348:9` "the standalone Worker (pid 15781) is still in /proc after 5s" |
| ac4 | 41008107054 · 123907517803 · check-multi-tenant-loom | `cohort::ac4_v5` same assertion, "host B's hang Worker (pid 33003)" |
| ac4 | 41281999213 · 124775636052 · check-multi-tenant-loom | `ac4_v4` same assertion |
| ac4 | 41289787129 · 124798452868 · workspace-test-suite | `ac4_v4` `FAIL [16.772s]` |
| ac4 | **"41275…"** | no discipline run id starting `41275` exists (`gh run list --limit 40`, 2026-11-01). The brief's id is unresolvable; the ledger's ac4 recurrences are `40080259088`, `40980187631`, `41008107054`, `41281999213`, `41289787129` (+ the cohort runs `38974347270`, `38988216897`). |
| ac3 (same oracle) | 40251998973 (`deferred-work.md:96`) | route-budget timing in `ac3_operator_door_unload_stops_a_live_worker_once` — same `wait_worker_stopped(.., 5 s)` (`root_shutdown_unload_16_3.rs:1505-1509`); log not re-fetched |
| supervisor | 41013625894 · 123924858906 · workspace-test-suite | `supervisor_scenes.rs:751` "hang guard (20 s): the stub runtime never recorded the kill call" → then `supervision.rs:2460:38` (@43ce3ea7) "Cannot start a runtime from within a runtime" from `WorkerBinding::drop` → `execute` → `KernelBindingExecutor::unload` → `block_on`, unwinding inside the async test |
| cap_token | 40237233599 · 121301291185 · workspace-test-suite | `P99 = 5670ns, budget = 5000ns` |
| cap_token | 41243340929 · 124661589929 · workspace-test-suite | `P99 = 5480ns, budget = 5000ns` |

## Timing distribution (nextest per-test wall, all downloaded jobs)

| Test | PASS samples (s) | FAIL samples (s) |
|---|---|---|
| `s3_external_sigkill_requires_pidfd_fact` | 1.209, 1.447, 1.650, 1.651, 1.870, 3.772 | 11.539, 11.583 (libtest: elapsed 10.03–10.10 s ×6) |
| `ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan` | 1.276, 1.583, 1.661, 1.715, 2.028, 2.130, 2.444 | 16.772 |
| `cohort::ac4_v5_…` | 1.520–3.161 (8 samples) | (libtest only) |
| `supervisor_scenes::binding_revokes_bearer_…` | 5.032 ×3, 5.040, 5.042 ×2, 5.049 | 20.594 |

The distributions are **bimodal**, not wide: passes cluster at 1–4 s (S3, ac4) and 5.03–5.05 s (supervisor =
`KILL_BUDGET`); failures sit one whole budget higher (S3: `SNAPSHOT_BUDGET` = 10 s,
`crates/maos-kernel-core/src/security/sandbox/t3/child.rs:26`; ac4: the root took ≈ 10 s to exit, 16.77 − 5 (pid
wait) − ≈ 1.5 (boot); supervisor: the 20 s hang guard). A noise floor widens a distribution; a discrete stall moves a
sample by exactly one bound. None of the four is a calibration of a correctly-measured quantity.

## S3 — `s3_external_sigkill_requires_pidfd_fact` (`crates/maos-bin/tests/worker_terminal_facts_17_7.rs:397-456`)

- **What the bound measures.** `elapsed` runs from the external `kill -KILL <payload>` to the root's exit
  (`worker_terminal_facts_17_7.rs:268-286`): FR12 detection plus the whole T3 teardown plus root shutdown — not FR12's
  detection instant. FR12 is a corpus floor (≥99/100 within 2 s / 5 s,
  `prd/functional-requirements.md:41`); this test asserts it per sample.
- **What failed.** Every failure is the full `SNAPSHOT_BUDGET` plus 0.03–0.10 s, and every one is accompanied by
  the podman client exiting **125** (podman's own error, not the payload's 137) and, in 4 of 6, a
  `TerminationUnconfirmed` residue with the T3 state directory retained. Passing runs show 1.2–3.8 s.
- **Mechanism at HEAD (code).** The observer's snapshot loops `podman inspect` until a terminal state under one
  absolute deadline (`child.rs:196-209,222-242`). When podman holds the container (podman 4.9.3 here; podman 5.7's
  ≈ 8 s hold is the documented reason for the 10 s budget, `child.rs:25`), the snapshot runs to its deadline. Then
  `run_teardown` (`child.rs:401-466`) reaches `:418-422`: with the snapshot still in flight it returns
  `TerminationUnconfirmed` **before `rm`** — even when the payload pidfd had already confirmed the exit
  (`termination_confirmed`, `:102-107`). The container is therefore left behind and the residue line names the wrong
  fact (termination *was* confirmed; removal was skipped).
- **Client 125.** [INFERENCE] matches podman 4.x's `start --attach` exit-code race ("no such exit code", exit 125 =
  error in podman itself, `https://docs.podman.io/en/v5.1.2/markdown/podman.1.html`); not reproduced here.
- **Verdict.** Not a calibration. Raising the podman-4 bound to 12 s would still red at the fact assertions that
  follow it (`:432-439`: `exit_cause` 137, `removed == true`, `residue == null`) on exactly these runs. Two defects are
  read from code: (1) teardown abandons removal of a confirmed-dead container when the runtime snapshot stalls
  (kernel, `child.rs:418-422`); (2) the test's single-sample root-exit bound stands in for a corpus floor. A third
  is open: whether the journaled exit cause follows the client's 125 or the pidfd's `signaled(9)` on these runs
  (measurement owed, `measurements.md` item 3).

## ac4 family — `root_shutdown_unload_16_3` v3b / v4 / cohort v5 (+ ac3 door-unload)

- **The assertion.** `wait_pid_gone` (`root_shutdown_unload_16_3.rs:342-354`) treats the payload's
  `/proc/<pid>/stat` start time as "present" — a zombie included — and runs only **after** the root has exited
  (`:1439-1452` v4; `:2137-2147` v5; `:1349-1371` v3b; `:1504-1509` ac3).
- **What failed.** The payload (or host B's Worker) was still in `/proc` 5 s after the root had exited. The one
  nextest sample (16.772 s vs passes 1.3–2.4 s) means the root itself took ≈ 10 s to exit — a budget-sized stall,
  the S3 signature. The failure message carries no root stderr, no `/proc` state and no container state, so which
  budget expired (snapshot 10 s, or `KILL_BUDGET` 5 s + `ROOT_GRACE` 5 s, `supervision.rs:2619`) is not determinable
  from the log.
- **Mechanism at HEAD (code).** Root shutdown runs `stop_workers` to completion (`supervision.rs:1911-1945`) and
  each binding's teardown kills only through the runtime CLI: `podman kill <CID>` bounded by `KILL_BUDGET`
  (`child.rs:273-280,407-417`). The supervisor's pidfd kill reaches the *direct child* — the podman client
  (`supervision.rs:3201-3210`) — never the payload, although the verified payload pidfd is already held
  (`spawn.rs:310-320,519-523`; `StartIdentity.pidfd`, `child.rs:49-54`). When the runtime CLI stalls past
  `KILL_BUDGET`, teardown returns `KillFailed`/`TerminationUnconfirmed`, the root exits, and the payload lives on.
  [INFERENCE] until the T0 hosted diagnostic shows the root stderr of a failing sample.
- **Verdict.** Not a calibration: if the payload is alive when the root exits, NFR-Rel-11's planned half and the
  test's "no orphan" claim are false on that run; raising 5 s would hide a live orphan. If the T0 diagnostic shows a
  zombie awaiting conmon's reap instead, the oracle (not the product) is wrong and the repair is the oracle.

## Supervisor kill-call barrier — `supervisor_scenes::binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease`

- **The barrier** (`crates/maos-bin/tests/worker_terminal_facts_17_7/supervisor_scenes.rs:744-756`) waits for the
  stub runtime's `argv=<kill>` record, written as the first lines of every stub invocation
  (`stub_scenes.rs:31-32`). The stop runs on `spawn_blocking` → `stop_session` (`supervision.rs:849-853`) →
  `begin_stop` (`Running` ⇒ `Signal`, `:451-463`) → `signal` (`:1183-1221`) → `ContainerTerminal::teardown` →
  `run_teardown` → `kill` (`child.rs:407-409`). Nothing on that path at HEAD waits on anything but the binding and
  progress mutexes before the stub is spawned.
- **What failed.** Once (41013625894 @ `43ce3ea7`): no `argv=<kill>` record within 20 s; seven other hosted samples
  pass at 5.03–5.05 s (the stub's gated kill bounded by `KILL_BUDGET`). The calls log, the binding phase and
  whether the stop task had started were not printed.
- **A second, deterministic defect in the same failure.** After the first panic the test's `WorkerBinding` dropped
  inside the async test body; its `Drop` (`supervision.rs:1453-1470` at HEAD, whose own comment says "Never panic
  out of a drop — this can run during unwinding", `:1459`) runs `execute` → `KernelBindingExecutor::unload` →
  `self.handle.block_on(..)` (`:2503-2504`), which panicked "Cannot start a runtime from within a runtime". The happy
  path avoids this only by dropping inside `block_in_place` (`supervisor_scenes.rs:781`). Whether a production
  binding is ever dropped on a runtime worker thread is not established here [INFERENCE: not searched exhaustively].
- **Verdict.** Cause open (no reproduction, no calls log). Not wall-clock calibration: the guard is a hang guard, and
  the passing distribution is fixed at the 5 s kill budget. The repair is diagnosis plus the deterministic
  drop-on-unwind defect; the cause stays an open row if a loaded-host reproduction does not recur.

## `cap_token_verify_p99_under_5us_hot_path` (`crates/maos-kernel-core/tests/cap_token_verify_assertion.rs:67-90`)

- **What it measures.** 10,000 wall-clock samples of `CapTokensShardRing::verify` with a mock crypto provider
  (`:17-43`), no warm-up, in the debug build (`target/debug` in every job log), inside a 5,000-test parallel nextest
  run on a shared runner. ADR-030's 5 µs is a hot-path figure for which ADR-068 names the stable runner
  (`ADR-068-stable-performance-runner.md:6,161`).
- **What failed.** 5.67 µs and 5.48 µs P99, i.e. 10 % over, twice. No functional assertion failed.
- **Also:** `cap_token_verify_p99_under_100us_overall` (`:92-111`) claims "verify + audit-enqueue" (`:3`) but times
  the identical loop with no audit enqueue — the NFR-Perf-3 "overall" figure is not what it measures.
- **Verdict.** Instrument problem (calibration class), not a product defect: a debug-profile wall-clock P99 on a
  shared hypervisor cannot carry a 5 µs claim (ADR-068 §1). The repair is to move the 5 µs claim to an instrument
  that can carry it (Tier C count per PR; Tier W P99 on the runner) and make the "overall" test honest.
