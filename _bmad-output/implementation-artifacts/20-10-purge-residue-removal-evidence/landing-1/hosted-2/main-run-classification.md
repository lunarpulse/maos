## Verdict

**All four primary failed jobs have the same sole observed test failure:**  
`worker_terminal_facts_17_7::root_crash::root_crash_retains_stopped_container_and_purge_reports_it`.

It fails at **`root_crash.rs:230`**, because the deliberately live foreign root shares the target’s scratch ancestor and therefore holds a shared store lock that blocks dry purge.

**This is the NEW fixture-isolation defect already diagnosed in `41610893716`, not a known timing flake.** Neither other purge tests nor the stalled-snapshot witness caused these reds. The ship gates and aggregate are downstream cascades.

Line numbers below count each job’s raw `gh api …/jobs/<id>/logs` output.

### Log aliases

| Alias | Job | Captured log |
|---|---|---|
| W | [workspace-test-suite / 125832118866](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125832118866) | `artifact://11933` |
| L | [check-multi-tenant-loom / 125832118891](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125832118891) | `artifact://11934` |
| E26 | [epic-17-exit (ubuntu-26.04) / 125843642348](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125843642348) | `artifact://11935` |
| E24 | [epic-17-exit (ubuntu-24.04) / 125843642417](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125843642417) | `artifact://11936` |

## Per failed job

| Job / failed step | Exact failure and evidence | Classification / required change |
|---|---|---|
| **workspace-test-suite**, step **15**, “Run the full workspace test suite (nextest)” | **W:9293**: panic at `root_crash.rs:230:5`. **W:9294**: `dry purge failed: maos: /tmp/.tmpipYMNY is held by a running MAOS process — StoreInUse, refusing to erase from a live store`. **W:13505–13506**: **5573 tests run: 5572 passed (1 slow), 1 failed, 138 skipped**; only this test is listed failed. Doctests succeed at **W:15348**; step finally exits **100** at **W:15451**. | **NEW**, deterministic witness-layout contention. Apply **F1** below. |
| **check-multi-tenant-loom**, step **10**, “Run check-multi-tenant-loom gate” | **L:3403**: `check-multi-tenant-loom RED: maos-bin-whole-package-parallel-isolation [INDETERMINATE]:`. **L:4717**: same panic at `root_crash.rs:230:5`. **L:4718**: `dry purge failed: maos: /tmp/.tmp95VVC4 is held by a running MAOS process — StoreInUse, refusing to erase from a live store`. **L:4726**: terminal binary **33 passed; 1 failed**, 37.11 s. **L:5550**: `check-multi-tenant-loom: 1 leg(s) block (dev-lane RED or required evidence ABSENT/INDETERMINATE with its substrate up)`. Exit **1** at **L:5558**. | **Same NEW defect**, not another isolation test or S3 teardown failure. Apply **F1**. The separate ABSENT warning at **L:3402** is not an additional failing test; the blocking leg named here is whole-package isolation. |
| **epic-17-exit (ubuntu-26.04)**, step **12**, “Epic-17 exit line 4 — retained T3 terminal facts (Story 17-7)” | **E26:874–875**: same `root_crash.rs:230:5` panic, with `dry purge failed: maos: /tmp/.tmpKZpLa3 is held by a running MAOS process — StoreInUse, refusing to erase from a live store`. **E26:1175–1176**: **34 tests run: 33 passed, 1 failed, 0 skipped**, only this scene failed. **E26:1183**: `cgroup-ceiling check-log: suite 1 expected 34 tests run and passed; got 34 tests run: 33 passed, 1 failed, 0 skipped`. Step exits **1** at **:1184**. | **Same NEW defect**. **Yes: its only observed red is the root-crash dry-purge scene.** Apply **F1**; retain the 34-test count and required-name checks. |
| **epic-17-exit (ubuntu-24.04)**, same step **12** | **E24:873–874**: same panic and message, naming `/tmp/.tmpRpkTAJ`. **E24:1174–1175**: **34 tests run: 33 passed, 1 failed, 0 skipped**, only this scene failed. **E24:1183**: identical count-check failure quoted above. Exit **1** at **:1184**. | **Same NEW defect**, not OS-specific. **Only the root-crash dry-purge scene failed.** Apply **F1**. |
| **[v1-0-ship-gate / 125855775306](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125855775306)**, step **3**, “Fail if any sub-gate failed or was skipped” | Raw log **:112**, **:123** identify loom and workspace as failed; **:181**: `v1.0 ship gate FAILED: one or more sub-gates did not pass.` Exit **1** at **:219**. | **Cascade**, no independent fix. Workspace and loom must become green. |
| **[v1-5-ship-gate / 125855898325](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125855898325)**, step **3**, “Fail if any v1.5-blocking gate failed” | **:58**: `| v1-0-ship-gate | failure | blocking (inherited) |`. **:79**: `v1.5 ship gate FAILED: one or more v1.5-blocking sub-gates did not pass.` Exit **1** at **:81**. | **Cascade** from v1.0. No aggregator change. |
| **[aggregate / 125855898349](https://github.com/lunarpulse/maos/actions/runs/41610868202/job/125855898349)**, step **6**, “Fail if any gate failed” | **:1232**: `One or more discipline gates failed.` Exit **1** at **:1233**. Its prerequisite JSON records failed Epic-17 exit, workspace and v1.0. | **Cascade**. Repair the primary witness and obtain passing prerequisite jobs; do not relax aggregate. |

## F1 — Precise cause and fix

Source paths are absolute:

1. **Target home:**  
   `/home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs:161–162` creates `<scratch>/home`.
2. **Live foreign home:**  
   Same file **:213–216** creates `<same scratch>/foreign-home` and intentionally retains its running root.
3. **Shared user HOME:**  
   Helper **:17–18,30–35** derives HOME from `home.parent()`. Both commands therefore also use the same `<scratch>` user HOME.
4. **Production lock policy:**  
   `/home/lunarpulse/dev_ws/crates/maos-bin/src/operator_door.rs:2781–2802` adds up to **two ancestor levels** to the store-lock set. It excludes the process temporary root, not an arbitrary scratch directory below it.
5. **Actual conflict:**  
   Same file **:2842–2846** uses shared locks for live roots and exclusive locks for offline purge. The foreign root consequently holds `<scratch>` shared while target purge requests it exclusively. **Every logged refusal names precisely that scratch directory**, rather than the killed target’s store.

**Fix at `root_crash.rs:213–216`:** allocate the foreign root beneath a **second independent `tempfile::TempDir`**, outside the target scratch tree:

- Keep its TempDir alive through the ownership assertions.
- Derive `foreign_home` as that independent directory’s `home` child.
- Declare the TempDir before the foreign `LiveRoot` so the live process is stopped before its directory is dropped.
- Keep the foreign root live while purging the target; preserve assertions that foreign CID and label-less impostor survive.
- The existing manifest can remain the launch input; the conflicting state/HOME ancestry must be separated.

**Do not weaken `operator_door.rs:2781–2802` or accept `StoreInUse` as an expected result.** These logs demonstrate fixture contention, not failed CID removal. Confirmed purge and its owned-CID survival assertions at **`root_crash.rs:237–263` are not reached**.

This repair removes the **sole observed blocker** in all four primary jobs. Green-after-repair is not established by these logs because the failed scene currently stops before confirmed purge.

## Explicit exclusion of other purge failures and known flakes

### Other purge witnesses passed

- All **12 `purge_residue_20_10` tests** pass:
  - Workspace: **W:9000–9011**.
  - Loom: **L:4206–4220**, summary **12 passed; 0 failed**.
- `purge_refuses_live_store_without_changing_filesystem_identity` passes:
  - **W:8907**, **L:4045**.
- `root_crash::purge_cgroup_native_binding_and_dead_creator_twins` passes:
  - **W:9268**, **L:4678**, **E26:869**, **E24:868**.

Thus these reds do **not** establish another purge test sharing an ancestor with a live root.

### Known snapshot/S3 families were not triggered

Applicable deferred-work references:

- `/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/deferred-work.md:5`: **“Default-concurrency snapshot-owner teardown flake”**, owner `20-7-stable-runner-perf-gates`; signature is `teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` reporting **`instance 16: observer never entered inspect`**.  
  **This witness passes in this workspace run**, **W:11679**, 10.038 s.
- Same file **:1209–1213**: known S3 observer/orphan detection exceeding its bound; **:1371**: stalled snapshot producing `fault.truncated` instead of pidfd `fault.signaled`/9.  
  **S3 passes here:** **W:9276**, **L:4683**, **E26:934**, **E24:933**.
- Same file **:1203–1208**: historical undelegated whole-package loom setup problem is explicitly **closed**. Its historical mention of root-crash does not classify this new `StoreInUse` refusal as known.

**No failed test in this run matches a known-flake row.** The 34-test pin is correct; neither timeout/budget increases nor assertion weakening is indicated.

Read-only investigation completed; no repository edits, cargo runs, or test reruns performed.