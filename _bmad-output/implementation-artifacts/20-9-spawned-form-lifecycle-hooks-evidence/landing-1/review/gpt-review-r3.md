> Independent review (round 3) by OpenAI gpt (agent `Review209L1r3`) of the 20-9 round-3 corrections, commit f56959c2. Persisted verbatim by the correction-round dev (round 4) because the reviewer was read-only.

## Verdict: incorrect — 2 P2 findings

Reviewed committed sources and evidence only; no builds, tests, or mutations executed.

### 1. [P2] Attribute busy drops to the individual invocation
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/hook_pump.rs:1415` — confidence 1.0**

`busy_drops(pid) > before` detects another concurrent hook’s contention drop, not necessarily this dispatch’s. For example, a default-budget forget consolidation starts and finishes successfully after four seconds; telemetry queued behind it gives up after 2.5 seconds and increments the same pid counter. The successful consolidation is then classified as busy: `complete_forget` falsely reports its purge undelivered, or the watchdog fails to advance `last_consolidate_ns`. **Fix:** return a per-invocation started/dropped disposition through the dispatcher instead of inferring it from a shared pid counter. **Required failing-before:** overlap a four-second successful forget consolidation with a queued telemetry drop; assert the purge remains delivered and ordinary successful consolidation advances its timestamp.

### 2. [P2] Retain actual consolidation workers through shutdown
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/hook_pump.rs:1408–1414` — confidence 0.99**

The new consolidation wall drops `HookDispatcher::fire_on_consolidate`, whose implementation awaits `spawn_blocking`; dropping that future detaches rather than stops the blocking hook. A compiled-in hook budgeted for 60 seconds and finishing after 40 seconds therefore disappears from the watchdog’s `JoinSet` at 30 seconds, allowing shutdown and `on_unload` while its computation and captured runtime handles remain live. Previously this within-budget invocation was awaited to completion. **Fix:** retain the underlying execution handle separately from wall accounting and drain it before unloading its runtime; spawned exports need acknowledged cancellation/termination if their execution must stop at the wall. **Required failing-before:** hold consolidation past its wall but inside its manifest cap, initiate shutdown, and assert unload cannot precede actual hook completion.

## Closure assessment

- **Snapshot fence:** the reported single-attempt Drop/late-reply race is fixed by the shared mutex. **Not fully attempt-specific:** a timed-out snapshot can finish after a subsequent serialized retry installs its attempt; `fence_for_snapshot` checks the new attempt rather than the snapshot’s original owner, spuriously fencing and refusing that retry. Carry the attempt token in `Call`/`Pending` and compare ownership when fencing/clearing. This is a residual of the earlier defect, not counted as newly introduced here.
- **FR11 archive:** failed cold-swap readiness no longer publishes the successor record; the hot-swap path is unchanged. Publication is not crash-atomic: a crash before the archive write leaves the predecessor current. No successful admission has returned then; existing best-effort archive-write behavior remains.
- **Busy queue:** queue cancellation now precedes the pump wall and prevents later execution. Accounting remains incorrect under concurrent calls because of finding 1. The global ledger also never removes historical pid entries.
- **Slow history:** telemetry-only, bounded five-bit history is correct; alternating two-slow/one-fast cannot evade it. D-20-9-21 explicitly refines the older “three consecutive” policy.
- **Unregister:** performed before both unwind branches.
- **Consolidation/privacy:** fan-out and wrapper joins are concurrent; privacy bypass remains wall-bounded. Actual execution is not joined by those wrapper joins—finding 2. Certification still checks the transactional `through_rowid` boundary, but no longer necessarily follows completion of started hook work.
- **Cfg:** `unwind_admitted` and its caller have matching gates; no patch-introduced default/no-default/network-only reference mismatch found. Compilation was not independently exercised.
- **Serde:** no new serde-followed `unwrap`/`expect` defect found.

## Evidence and claims

- Red log records **8 selected tests, 8 intended failures**.
- There are **9 plant runs/logs containing 10 textual replacements**, not ten independent plant runs. All nine record intended failures, exit 100, and clean restoration.
- Recorded focused results: **325 passed, 3 skipped**; after-cfg subset **31 passed**.
- Workspace is **not green: 5,913 passed, 2 failed, 137 skipped**—KLOC and the million-row pending-forget test.
- Bin **37,832 ≤ 38,422** is directly recorded. Clean aggregate **201,396 ≤ 202,209** is the documented adjusted figure; the retained actual gate reports **201,618**, with aggregate and spike-budget breaches.
- The **30-second wall is explicitly unwitnessed**. Short concurrency tests do not prove cancellation or shutdown at that boundary.

**Headlines: 2 P2 — cross-invocation busy misclassification; consolidation workers outlive joined shutdown. Overall confidence: 0.98.**
