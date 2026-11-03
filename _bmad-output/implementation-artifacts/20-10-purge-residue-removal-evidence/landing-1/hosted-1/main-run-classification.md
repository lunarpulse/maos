## Classification

**This run is not “known flake only.”** Its primary reds are two NEW 20-10 regressions:

1. **[INFERENCE, strongly supported] Egress socket paths became too long:** the new `process-<pid>-<nonce>` naming unintentionally applies to durable `HOME/egress` directories as well as volatile T3 custody roots.
2. **Confirmed fixture regression:** the terminal purge scenes set `HOME == MAOS_HOME`, which the newly strengthened purge safety check correctly refuses.

The Epic-17 **33→34 count pin is correct**. Both images actually executed **34 tests: 28 passed, six failed, zero skipped**.

### Failed jobs

| Job / hosted job ID | Failing step/test | Classification | Probable cause / fix |
|---|---|---|---|
| [nfr-rel-11-halt-receipt-999pct / 125746581361](https://github.com/lunarpulse/maos/actions/jobs/125746581361) | Step 8, **Run root shutdown unload suite (release — 9 vectors)**; `ac3_operator_door_unload_stops_a_live_worker_once` | **NEW**, not known AC4 teardown flake | Egress socket path overflow; fix **F1** |
| [workspace-test-suite / 125746587735](https://github.com/lunarpulse/maos/actions/jobs/125746587735) | Step 15, **Run the full workspace test suite (nextest)**; nine failures listed below | **NEW** | Seven startup/egress failures (**F1**), two purge-fixture failures (**F2**) |
| [check-multi-tenant-loom / 125746588022](https://github.com/lunarpulse/maos/actions/jobs/125746588022) | Step 10, **Run check-multi-tenant-loom gate**; leg `maos-bin-whole-package-parallel-isolation`; AC3 test above | **NEW**, not known AC4/S3 flake | Same startup/egress regression; **F1** |
| [epic-17-exit (ubuntu-24.04) / 125758309982](https://github.com/lunarpulse/maos/actions/jobs/125758309982) | Step 12, **Epic-17 exit line 4 — retained T3 terminal facts (Story 17-7)**; six tests below | **NEW** | Four startup failures (**F1**), two purge-fixture failures (**F2**); **not count mismatch** |
| [epic-17-exit (ubuntu-26.04) / 125758310043](https://github.com/lunarpulse/maos/actions/jobs/125758310043) | Same step and six tests | **NEW** | Same two regressions on the second runtime/image |
| [check-test-budget / 125758528438](https://github.com/lunarpulse/maos/actions/jobs/125758528438) | Step 6, **Run check-test-budget on the workspace junit** | **NEW consequential failure** | Startup waits inflate two binaries and suite sum; fix **F1**, not budget allowances |
| [v1-0-ship-gate / 125763113042](https://github.com/lunarpulse/maos/actions/jobs/125763113042) | Step 3, **Fail if any sub-gate failed or was skipped** | Cascade, no independent test defect | Workspace, loom and budget failed |
| [aggregate / 125763144382](https://github.com/lunarpulse/maos/actions/jobs/125763144382) | Step 6, **Fail if any gate failed** | Cascade | Upstream discipline failures |
| [v1-5-ship-gate / 125763144407](https://github.com/lunarpulse/maos/actions/jobs/125763144407) | Step 3, **Fail if any v1.5-blocking gate failed** | Cascade | Inherits failed v1.0 gate |

## Per-job evidence

Log aliases below resolve to these **absolute paths**:

- **N**: `/tmp/l10-nfr-rel-11.log`
- **W**: `/tmp/l10-workspace.log`
- **L**: `/tmp/l10-loom.log`
- **E24**: `/tmp/l10-epic17-24.log`
- **E26**: `/tmp/l10-epic17-26.log`
- **B**: `/tmp/l10-budget.log`

### NFR-Rel-11 and loom

Exact failure in both jobs:

> `no published hang OCI Worker appeared within 60s; root output:`

- **N:3492–3494**: panic at `root_shutdown_unload_16_3.rs:333:9`.
- **N:3501–3504**: **8 passed, one failed**, release step exits 101.
- **L:3405**: `check-multi-tenant-loom RED: maos-bin-whole-package-parallel-isolation [INDETERMINATE]:`
- **L:4330–4338**: identical AC3 panic; **11 passed, one failed**, 60.10 s.

These fail **before obtaining a live Worker**, not during teardown. The known AC4 vectors pass in the same jobs: **N:3482–3484**, **L:4315–4322**.

The empty diagnostic after `root output:` is explained by `/home/lunarpulse/dev_ws/crates/maos-bin/tests/root_shutdown_unload_16_3.rs:316–317`: this caller supplies `String::new` as the failure-context callback. Consequently, the underlying spawn error is not printed; the socket diagnosis below remains explicitly an inference.

### Workspace: all nine failures

**W:13659–13668** records **5,571 tests, 5,562 passed, nine failed**.

| Failing test | Exact failure text / assertion | Log lines |
|---|---|---|
| `root_shutdown_unload_16_3::ac3_operator_door_unload_stops_a_live_worker_once` | `no published hang OCI Worker appeared within 60s; root output:` | **W:9280–9282** |
| `worker_supervision_16_3::the_fr4_call_feed_survives_every_worker_row_it_must_omit` | `the Worker never produced task.stalled in the single AC5 root` | **W:9302–9303** |
| `worker_supervision_16_3::the_crash_scene_through_the_real_binary_lands_at_one_real_pid` | `the T3 Worker never journaled its first output line` | **W:9344–9345** |
| `worker_terminal_facts_17_7::root_crash::purge_cgroup_native_binding_and_dead_creator_twins` | `maos: purge configuration invalid: refusing unsafe purge root /tmp/.tmpac4SMF/home (equal to HOME or an ancestor)` | **W:9323–9324** |
| `worker_terminal_facts_17_7::root_crash::root_crash_retains_stopped_container_and_purge_reports_it` | `dry purge failed: maos: purge configuration invalid: refusing unsafe purge root /tmp/.tmpxfmrp7/home (equal to HOME or an ancestor)` | **W:9432–9433** |
| `worker_terminal_facts_17_7::s1_immediate_exit_terminal_facts` | `S1 must complete:` — no captured stdout follows | **W:9366–9367** |
| `worker_terminal_facts_17_7::s2_intentional_exit_137_is_not_a_signal` | Assertion diagnostic begins ``assertion `left == right` failed: one Worker exit row:``; `left: 0`, `right: 1` | **W:9385–9388** |
| `worker_terminal_facts_17_7::s3_external_sigkill_requires_pidfd_fact` | `worker did not publish cli_wrapper_loaded CID` | **W:9486–9487** |
| `worker_terminal_facts_17_7::s4_oom_marker_is_not_payload_evidence_alone` | Same exit-row assertion; `left: 0`, `right: 1` | **W:9407–9410** |

The full, unabridged dynamic row vectors are at **W:9386** and **W:9408**. They show admission/load/start followed by **`task.nacked`, unload, planned-unload and revocation**, but **no `cli.subprocess.exit`**. That supports a failure before successful subprocess launch, not an incorrect terminal-exit classification.

### Both Epic-17 images: all six failures

| Test suffix within `worker_terminal_facts_17_7` | Ubuntu 24.04 evidence | Ubuntu 26.04 evidence |
|---|---|---|
| `root_crash::purge_cgroup_native_binding_and_dead_creator_twins` | **E24:865–866**: unsafe purge root `/tmp/.tmpmEfPSe/home` | **E26:866–867**: unsafe purge root `/tmp/.tmpsPU3iV/home` |
| `root_crash::root_crash_retains_stopped_container_and_purge_reports_it` | **E24:883–884**: `dry purge failed: … /tmp/.tmpvgVtw2/home (equal to HOME or an ancestor)` | **E26:884–885**: same error for `/tmp/.tmpr7gQ2w/home` |
| `s1_immediate_exit_terminal_facts` | **E24:928–929**: `S1 must complete:` | **E26:929–930**: identical |
| `s2_intentional_exit_137_is_not_a_signal` | **E24:945–948**: exit-row equality assertion, **0 versus 1** | **E26:946–949**: identical |
| `s3_external_sigkill_requires_pidfd_fact` | **E24:964–965**: `worker did not publish cli_wrapper_loaded CID` | **E26:965–966**: identical |
| `s4_oom_marker_is_not_payload_evidence_alone` | **E24:981–984**: exit-row equality assertion, **0 versus 1** | **E26:982–985**: identical |

For the first row, the complete error template is:

> `maos: purge configuration invalid: refusing unsafe purge root <path> (equal to HOME or an ancestor)`

For the second row, it has the exact additional prefix `dry purge failed: `.

**Count-pin evidence:** **E24:860**, **E26:861** start 34 tests. **E24:1224**, **E26:1225** report `34 tests run: 28 passed, 6 failed, 0 skipped`. Both **:1238** messages say:

> `cgroup-ceiling check-log: suite 1 expected 34 tests run and passed; got 34 tests run: 28 passed, 6 failed, 0 skipped`

Thus the accounting check fails because tests are **red**, not because 34 is wrong. The newly added cgroup purge scene really is red; the injected `on_unload` panic later in these logs belongs to an intentionally passing panic witness, not another failed test.

### Check-test-budget: published JUnit offenders

Downloaded and parsed artifact **13209425274**, `workspace-test-suite-junit/junit.xml`; findings match **B:1937–1941**.

| JUnit binary / scope | Measured Σ test seconds | Budget | Main contributors |
|---|---:|---:|---|
| `maos-bin::root_shutdown_unload_16_3` — 12 tests | **70.157 s** | 30.000 s | Failed AC3 startup wait: **60.080 s**; all other tests total **10.077 s** |
| `maos-bin::worker_supervision_16_3` — 32 tests | **73.575 s** | 46.000 s | Failed crash scene **30.069 s**; failed FR4 scene **20.042 s**; other tests total **23.464 s** |
| Entire suite | **1114.292 s** | 1100.000 s | Over by **14.292 s** |

Other largest supervision contributors are the passing intentional kill-failure witness **10.026 s**, hung-worker witness **6.487 s**, and three-worker stall witness **6.032 s**. The newly added cached-root purge witness costs only **0.182 s**.

Exact finding messages:

> `maos-bin::root_shutdown_unload_16_3 took 70.157 s (Σ per-test time) over 12 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml`

> `maos-bin::worker_supervision_16_3 took 73.575 s (Σ per-test time) over 32 test(s), budget 46.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml`

> `suite test time 1114.292 s (Σ per-test time) exceeds total_seconds 1100.000`

These are **startup-regression consequences**, not evidence for raising budgets.

### Cascade jobs

- `/tmp/l10-v1-0.log:181`:  
  `v1.0 ship gate FAILED: one or more sub-gates did not pass.`  
  Failed upstreams explicitly printed at **:202, :212, :214**: loom, workspace, budget.
- `/tmp/l10-v1-5.log:79`:  
  `v1.5 ship gate FAILED: one or more v1.5-blocking sub-gates did not pass.`  
  **:58** identifies inherited `v1-0-ship-gate | failure`; the other displayed blocking gates succeeded.
- `/tmp/l10-aggregate.log:1232–1233`:  
  `One or more discipline gates failed.`  
  `Process completed with exit code 1.`  
  Upstream failure entries include **:350–351, :384–385, :596–601**.

No independent code fix is indicated for these aggregators.

## Precise fixes and causal evidence

### F1 — Keep creator-PID naming confined to volatile T3 custody

Relevant locations:

- `/home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs:88`: durable egress uses `create_root_dir(&home.join("egress"))`.
- Same file **:264–270**: that shared helper now always requests a process-prefixed child.
- Same file **:290–300**: emits `process-<pid>-<8hex>`.
- `/home/lunarpulse/dev_ws/crates/maos-egress/src/proxy.rs:152–157`: appends `/<spirit_pid>/egress.sock`, then calls `UnixListener::bind`.

**[INFERENCE]** This shared-helper change explains the startup failures. Static reconstruction using existing fixture prefixes, 19-digit timestamps and five-digit hosted PIDs gives:

| Fixture | Previous socket path bytes | New bytes |
|---|---:|---:|
| Terminal S1/S2/S4 | 102 | **116** |
| Terminal S3 | 107 | **121** |
| Root-shutdown AC3 | 105 | **119** |
| Supervision crash scene | 94 | **108** |
| Supervision FR4 scene | 102 | **116** |

Linux pathname Unix sockets require fewer than 108 bytes. Short `tempfile` homes used by the root-crash scenes can still launch Workers, consistent with their reaching purge rather than failing startup.

**Fix:** parameterize/split `create_root_dir` so **egress start at :88 retains short nonce-only directories**, while `create_worker_state_root` at **:242–254** retains `process-<pid>-<nonce>`. Preserve the volatile-root creator proof required by `/home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs:381–390,515–516`. Add the boundary regression witness beside existing egress integration coverage in `/home/lunarpulse/dev_ws/crates/maos-bin/tests/egress_daemon_17_1.rs:15–35`.

This is a **socket-path length break**, not evidence of a custody inventory regex rejecting the new root format.

### F2 — Separate user HOME from purgeable MAOS_HOME in terminal fixtures

- `/home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs:29–31` assigns the same path to `HOME`, `MAOS_HOME` and `XDG_DATA_HOME`.
- `/home/lunarpulse/dev_ws/tests/harness/t3_substrate.rs:44–68` returns the supplied home unchanged.
- `/home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs:1206–1211` now correctly rejects a purge root equal to HOME or its ancestor.

**Fix the fixture environment at `root_crash.rs:17–37`:** give each scene an isolated user HOME and a **distinct child MAOS_HOME**, consistently across root starts and purge commands; retain the provisioned Podman storage configuration through `isolated_runtime_home`. **Do not weaken the production HOME boundary check.**

The supervision test changes themselves are not the startup cause: the Arc→Weak cycle repair and new cached-root witness both pass (**W:9243**, **W:9258**).

## Known-flake disposition

**None of these failed jobs can be classified “known flake only.”**

Applicable deferred-work distinctions:

- `/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/deferred-work.md:1208–1212`: known **S3 post-kill observer/snapshot-bound** failures. This run instead fails **before publishing a CID**, at `worker_terminal_facts_17_7.rs:261`.
- Same file **:1213–1222,1280–1283,1374**: known **AC4 orphan/teardown** failures. This run fails **AC3 startup**, while the AC4 vectors pass.
- Same file **:1370**: stalled-runtime S3 classification (`fault.truncated` versus pidfd signal). This run never reaches that classification.
- Same file **:1202–1207**: historical undelegated loom setup defect is explicitly **closed**; it does not justify treating this new AC3 startup failure as known.

Read-only investigation completed; no repository edits, cargo runs, or test reruns performed.