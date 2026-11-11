> Independent review (round 4) by OpenAI gpt (agent `Review209L1r4`) of the 20-9 round-4 corrections, commit fba6af2a. Persisted verbatim by the correction-round dev (round 5) because the reviewer was read-only.

## Verdict: incorrect — 3 P2 findings

Reviewed commit `fba6af2a`, not the concurrently edited working tree. No builds, tests, or mutations executed.

### 1. [P2] Give identical concurrent batches distinct invocation records
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/invocation.rs:95–100` — confidence 1.0**

`claim()` repeatedly returns the first matching record without reserving it. Concurrent forget operations can produce identical batches—for example, two principals with no matching frame IDs or outstanding redistillation requests both produce the same empty `ForgetNotice`. With a 30-second budget, let the first consolidation run for 16 seconds and submit the second shortly afterward: both calls attach to the first record; the second expires after its 15-second queue allowance and marks that shared record dropped. Consequently, the successfully executed first notice reports its purge undelivered, while the unexecuted second notice reports delivery. **Fix:** pass a unique invocation token or handle through the exact dispatcher execution into `CallState`; content equality must not identify an invocation. **Witness needed:** concurrent identical forget batches, one successful and one queue-dropped; assert both reports independently match actual execution.

### 2. [P2] Mark full-channel contention as a dropped invocation
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/supervision.rs:2718–2725` — confidence 1.0**

Removing `note_busy()` leaves the `LifecycleLink::call()` full-channel path unaccounted: `TrySendError::Full(_)` returns `BUSY` without calling `state.cancel_queued()`. For example, a long frame turn holds the session while a consolidation occupies its one queued slot; telemetry then exhausts its 2.5-second enqueue allowance. The wrapper returns normally, but its invocation remains `QUEUED`, so `fire_walled()` counts the dropped event as delivered and accounts a clean telemetry run. Previously the wrapper’s busy counter covered this return path. **Fix:** call `state.cancel_queued()` before returning `BUSY` from the full-channel branch. **Witness needed:** hold a frame turn, fill the lifecycle slot with consolidation, publish telemetry, and assert zero telemetry executions and zero delivered increment.

### 3. [P2] Reap completed parked tasks during normal operation
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/hook_pump.rs:274–281` — confidence 1.0**

Every overrun adds a wrapper task to `parked`, but completed tasks are removed only by shutdown’s `drain_parked()`. Repeated forget notices bypass quarantine, so one persistent subscriber whose consolidation takes 31 seconds under a 60-second budget adds another retained completed task after every notice. This grows the daemon’s task bookkeeping indefinitely even when no execution remains active; subscriber unload does not clear it. **Fix:** reap ready entries with `try_join_next()` during normal parking/reaping, retaining only outstanding executions. **Witness needed:** repeat completed wall-overruns without shutting down and assert retained parked entries do not grow with completed dispatch count.

## Closure and evidence

- **Spawned cancellation is not execution acknowledgment.** The new link deadline cancels waiting, but the session never checks cancellation of its running `Pending` export. Its child continues until replying or reaching the manifest-based session deadline. Thus the 25-second test proves prompt `HookPump::shutdown`, **not** that the export ended before subscription revocation. This is an unclosed prior safety concern, not counted above as newly introduced.
- Snapshot replies now compare attempt identity under the fencing mutex. Normal watchdog→pump shutdown drains parked dispatch handles before root-shutdown revocations; compiled-in execution beyond the kernel dispatcher’s cap remains explicitly excluded.
- Pre-warm read/probe waits use the 60-second minimum. Inline exchanges still perform blocking `write_frame()` before polling their deadline, so the evidence does not establish a wall against stdin backpressure. This existing limitation is not newly introduced.
- Doorbell roster update preserves the guard and includes `invocation.rs`.
- Retained logs support **9 plant runs / 10 replacements**, intended failures, exit 100, and restoration markers. They accurately disclose the spawned test’s green baseline.
- Recorded results are **5,955 passed / 3 failed / 137 skipped**, with the doorbell subsequently **9/9** in both configurations—not a green workspace run.
- Recorded bin is **38,019 ≤ 38,422**. Clean aggregate **202,000 ≤ 202,209** is the documented adjustment; the retained gate actually measures **202,222** and fails.
- No patch-introduced serde-followed `unwrap`/`expect` defect found. GLM F2/F3/F6 remain explicitly deferred.

**Overall confidence: 0.99. Headlines: 3 P2 — identical-batch record aliasing; full-channel drops counted delivered; completed parked tasks retained indefinitely.**
