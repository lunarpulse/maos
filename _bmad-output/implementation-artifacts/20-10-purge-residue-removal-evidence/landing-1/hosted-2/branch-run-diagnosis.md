## Diagnosis

Log lines below are numbered from each job’s raw `gh api …/jobs/<id>/logs` output.

| Failing leg | First real failure | Classification | Intended red? |
|---|---|---|---|
| [24.04 head](https://github.com/lunarpulse/maos/actions/runs/41610893716/job/125836507950) | **L6246–6247:** `root_crash.rs:230`, dry purge refused with `StoreInUse` on `/tmp/.tmpwnFs6C` | **Witness isolation defect:** live foreign root shares the target’s ancestor lock directory | **No.** Head must be green |
| [26.04 head](https://github.com/lunarpulse/maos/actions/runs/41610893716/job/125836507889) | **L6253–6254:** same assertion/refusal on `/tmp/.tmpitiBS2` | **Same witness defect**, not an OS-specific runtime failure | **No.** Head must be green |
| [24.04 base](https://github.com/lunarpulse/maos/actions/runs/41610893716/job/125836507782) | **L5488–5489:** `root_crash.rs:341`, `dead-creator empty leaf survived` | **Expected baseline production absence (AC4).** Subsequently also hits the witness defect | **Partially:** valid AC4 red, **not** valid AC1 red |
| [24.04 plant](https://github.com/lunarpulse/maos/actions/runs/41610893716/job/125836507766) | **L5477–5478:** `root_crash.rs:230`, dry purge refused with `StoreInUse` on `/tmp/.tmpNgzQFL` | **Same witness defect**, before planted removal can be exercised | **No.** Does not prove sensitivity to removal no-op |

## Per-leg evidence

### Head — both images

- **24.04 L6234–6241:** delegated cgroup/native-binding witness **passes**.
- **26.04 L6241–6248:** same witness **passes**.
- The root-crash test then fails at its **dry-run success assertion**, before confirmed purge:
  - 24.04 **L6246–6250**.
  - 26.04 **L6253–6257**.
- Both summaries report **34 tests, 33 passed, one failed**. The later injected `on_unload` panic belongs to an intentionally passing scene, not the first failure.

**Causal source chain:**

All file paths below are relative to **`/home/lunarpulse/dev_ws/`**.

1. `crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs:161–162` puts the target home at `<scratch>/home`.
2. `:213–216` puts the foreign home at `<same scratch>/foreign-home` and deliberately leaves its root live.
3. Helper `:17–18,30–35` derives **both commands’ HOME from `home.parent()`**, so these “independent” roots also share the same user HOME.
4. `crates/maos-bin/src/operator_door.rs:2781–2802` adds up to two ancestor directories as stable store-lock guards. It excludes `/tmp`, **not `<scratch>`**.
5. The foreign root’s shared lock on `<scratch>` blocks the target purge’s exclusive lock. The logged refusal names exactly that shared scratch directory.

This is deterministic fixture-layout contention, **not evidence that the killed roots retained their locks**, and not a reason to relax the live-store refusal.

### Base

- **L5488–5489:** empty dead-creator cgroup leaf survives; this is the intended baseline AC4 failure.
- **L5505–5506:** root-crash subsequently fails at dry purge with the same shared-scratch `StoreInUse`.
- **L5520–5522:** both selected tests failed.

The workflow requires **both** `owned CID .* survived purge` and `dead-creator empty leaf survived` (`ci/ci-20-10-verify.yml:173–175`). Only the latter is observed. Therefore the base job is legitimately red for **AC4**, but its **AC1 failing-before evidence is blocked**, and the workflow correctly does not accept the overall control.

### Plant

- **L223:** the hashed no-op patch passes its SHA check and is applied.
- **L5477–5478:** root-crash stops at dry purge’s `StoreInUse`.
- The owned-CID survival assertion at `root_crash.rs:257` is **never reached**.
- Workflow `ci/ci-20-10-verify.yml:172` requires that specific survival diagnostic; it is absent. The restore-and-green step at `:178–188` therefore does not execute.

The plant is **not red for the intended reason**. Its current failure cannot establish that the scene detects a disabled removal call.

## Precise fixes and limits

1. **Repair the witness layout at `root_crash.rs:213–216`.** Allocate the foreign root under a **second independent `tempfile::TempDir`**, outside the target scratch tree, then create `foreign_dir.path().join("home")`. Keep that TempDir alive through the assertions and ensure the foreign `LiveRoot` is dropped before it. This gives the foreign root a distinct HOME and distinct non-`/tmp` ancestor guards while retaining the live foreign-container twin.
2. **Keep the workflow’s diagnostic requirements** at `ci/ci-20-10-verify.yml:163–175`. Do not accept arbitrary nonzero exits or add `StoreInUse` to an expected-red allowlist. After witness repair, base must reach the owned-CID survival assertion as well as its already-observed cgroup failure; plant must reach owned-CID survival, followed by restored green.
3. **No allowlist, unit-name, argv, or delegated-env correction is supported by this run.** For example, 24.04 head **L4822–4825** shows the unit started, `Delegate=yes`, `cpu,memory,pids` controllers, and a successful provisioned rootless Podman canary. These are real executed scenes, unlike the previous invalid-unit run.
4. **Do not change 20-10 production runtime predicates on this evidence.** The failing root-crash paths stop at the store lock, before runtime inventory/removal. Thus these logs do **not** diagnose mount-source form, label-filter combination, `rm`/`exists` semantics, custody creator-PID proof, or receipt/exit conventions. Delegated cgroup binding/removal is positively exercised and passes on both heads. The broader ancestor-lock policy at `operator_door.rs:2781–2802` can cause cross-home contention for sibling layouts, but altering that production concurrency policy is not necessary to repair this witness and should not be conflated with a demonstrated 20-10 removal defect.

Read-only investigation only; no edits or test reruns performed.