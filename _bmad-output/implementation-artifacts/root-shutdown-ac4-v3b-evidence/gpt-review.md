## Finding

### [P2] Await an in-progress disposition before the forced exit
**Location:** `crates/maos-bin/src/supervision.rs:3223`  
**Confidence:** 0.96

If `WorkerBinding::finish` sets `dispositioned = true` and is then descheduled before executing the journal write, the new fallback call treats the disposition as complete and returns without waiting. The listener can subsequently unload and call `process::exit(1)` before that thread writes `task.nacked`. The latch is set under the binding mutex at `supervision.rs:1298-1305`, but execution happens after releasing it at `:1306-1307`; neither registry retirement nor unloading joins that execution. Synchronize with completion of an already-claimed disposition before allowing the forced exit.

## Answers

### 1. v3b assertions and sibling compatibility

The fallback now implements the required sequence, **except for the race above**:

- **Exit 1:** unchanged, `supervision.rs:3255`.
- **Interrupted sentence:** `:3231-3234`; `main.rs:4955-4956` declares `--once`.
- **`task.nacked`, `topology:1`, originator `topology`:** fallback dispositions before unloading (`:3223-3227`), using the retained assignment. The payload includes both identifiers (`crates/maos-iac/src/adapter.rs:340-358`).
- **Worker/architect unload rows and PlannedUnload receipt:** existing `unload_all_loaded` remains in the fallback; scheduler unload writes these synchronously (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:519-557`).
- **Reviewer never loaded:** the main flow checks cancellation before continuing past the Worker (`main.rs:5692-5697`).
- **No false success:** fallback prints no completion sentence and exits nonzero.

These correspond to `root_shutdown_unload_16_3.rs:1430-1491`. Its “drains” assertion only checks absence of `"drain timed out"`; it does **not** prove the fallback drained the audit channel.

Read v1–v5, including v4b/v5b/v5c: no additional patch-induced sibling regression found. Normal paths are unchanged. **v5’s slow fallback still cannot guarantee `cap.revoke`**, because it deliberately skips the audit drain; that limitation predates this patch (`root_shutdown_unload_16_3.rs:2316-2325`, `supervision.rs:3253-3255`).

Repository-wide uses: `unstopped_workers` has only the listener caller (`supervision.rs:3236`); `spawn_signal_listener` has only `arm_with_seams` (`:3023`). Production arming sites are `main.rs:4430`, `:4943`, and `:9457`; public arm signatures remain unchanged.

### 2. Double-nack, finish, and locks

- **The latch is thread-safe:** it is a plain bool protected by the shared binding mutex, not an unprotected bool (`supervision.rs:1296-1307`).
- **Double execution through fallback versus `finish`:** prevented by that latch.
- **Completed clean Worker:** not selected; clean finish transitions to `ExitedClean`, whereas fallback selects only `PlannedStop` (`:488-516`, `:2191-2197`).
- **Completed session turn:** `complete_turn` clears `record_active` under the same mutex; an already-completed turn is excluded (`:1895-1904`).
- **Lock order:** `active()` clones the cores and releases the registry lock before phase/disposition locks; executor calls occur outside the binding lock (`:1373-1378`, `:2191-2197`). No new lock inversion found.
- **Important distinction:** atomic claiming is not atomic completion—the reported P2.

### 3. `once_pass` ordering

The listener **can receive a signal before** `declare_once_pass`: it is spawned inside `arm`, before `main.rs:4956` (`supervision.rs:3199-3207`).

However, that newly constructed supervisor has no admitted Worker yet. An early signal stops the empty supervisor, and `await_idle` returns true without entering the fallback (`:3212-3215`). Therefore that window does not produce the wrong fallback sentence in the production run path. The flag is loaded at exit, not captured when the signal arrives.

If false at a fallback read, the generic sentence is truthful (`:3234`).

### 4. Journaling before `process::exit`

**Completed writes survive ordinary process exit without destructors.** Nacks use synchronous Transparency Log inserts, not the asynchronous cap-audit channel:

`KernelBindingExecutor::disposition` → `enforce_disposition` → `emit_task_complete_nack` → SQLite insert.

Evidence: `supervision.rs:2615-2634`; `disposition.rs:16-32`; `adapter.rs:340-358`; `adapter/transparency_log.rs:1365-1445`.

SQLite uses WAL (`transparency_log.rs:896-898`); ordinary inserts are autocommit. No explicit checkpoint or `Drop` is needed for those committed rows. **Power-loss durability was not measured**, and synchronous mode is not explicitly configured here. The P2 concerns a write that has not begun, not flushing a completed write.

### 5. Honesty

The revised diagnostic correctly reports the measured timeout and qualifies the descendant explanation with **“may”** (`supervision.rs:3238-3244`). It does not claim successful draining.

The comments asserting that `finish` “never runs” are stronger than the concurrent implementation guarantees (`:3216-3222`): it may resume while fallback executes. That concurrency matters for the finding above.

### 6. Initialized-container hypothesis

**The readiness gap exists; its connection to the hosted hang remains unproven.**

- The helper accepts a published CID, nonzero inspected PID, matching cgroup, and process start times. It never checks running state (`root_shutdown_unload_16_3.rs:276-332`).
- Product launch performs `init`, reads its PID receipt, then merely spawns `start --attach` before returning (`crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs:497-546`).
- `cli_wrapper_loaded` is published afterward, without waiting for running-state confirmation (`worker_spawn.rs:1140-1187`).

Thus the helper can signal before container startup completes. For a test claiming a **live hanging fixture**, the minimal correction is to require running state in addition to the existing identity checks. That is a test synchronization correction—not permission for the product to mishandle startup cancellation.

`CID kill failed` does not establish this cause: teardown kills through the payload pidfd **before** invoking runtime kill (`child.rs:448-449`), so runtime kill failure can also follow successful termination. Neither the actual pipe holder nor the hosted container state was captured.

### 7. Serde and clippy

No new serde decoding, `unwrap`, or `expect` appears in the changed hunks. No clippy-visible issue found by inspection. No checks were executed.

## Counts and verdict

- **P0: 0 · P1: 0 · P2: 1 · P3: 0**
- **Overall correctness: incorrect — RC blocked by the P2.**
- **Overall confidence: 0.95**

**Unverified:** post-fix hosted/forced-fallback execution, real T3 sibling execution, race reproduction, actual slow-path cause, and power-loss durability. The evidence’s local T3 tests skipped their substantive paths.

The evidence’s KLOC failure is historical: current `xtask/kloc.toml:517,718` already pins **38,706** and aggregate hardfail **203,384**. No gate rerun performed.