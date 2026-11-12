## Finding

### [P2] Enforce a deadline around the entire blocking disposition sweep
**Location:** `crates/maos-bin/src/supervision.rs:3285-3288`  
**Confidence:** 0.98

The listener awaits this blocking task without a timeout, while the sweep creates its deadline only after the blocking pool schedules it (`:2239-2240`). If that pool is occupied by stuck work, the fallback can wait indefinitely without starting its five-second budget. Once scheduled, `core.disposition()` also executes synchronously before the timed Condvar wait (`:2244-2245`); its scheduler/ledger locks and journal insertion are not bounded by that deadline. Consequently the claimed shutdown bound does not hold, and the listener may never reach its diagnostic, unload, or exit. Enforce a deadline covering queueing and execution, rather than only waiting for a disposition claimed elsewhere.

## Answers

### 1. Original P2: synchronization race addressed

- **No missed late claim:** the latch and counter increment share the binding mutex (`:1304-1314`). The sweep calls `disposition()` before waiting (`:2243-2245`), so either it executes the disposition itself or observes the existing claim and its counter.
- **No later reset on stopped bindings:** `begin_turn` requires `Running` and a supervisor that is not stopping (`:1903-1921`).
- **No lost wakeup:** the predicate check and `wait_timeout` use the same mutex (`:1323-1334`). The guard decrements under that mutex and then notifies (`:1341-1345`).
- **Panic:** executor unwinding drops the guard (`:1316-1319`), preventing a permanently elevated counter. This establishes executor termination, **not successful journaling** after a panic.
- **Poison:** both initial locking (`:962-966`) and Condvar reacquisition (`:1331-1334`) recover the guard.

Thus the original claim-versus-completion scheduling race is closed for a normally returning disposition within the permitted wait. The explicitly accepted timeout can still lose a nack.

### 2. Shutdown bound: not established

`spawn_blocking` is not a timeout mechanism. The pinned Tokio 1.53.3 queues work when its blocking pool is full; its default limit is 512. Queue delay is outside the sweep’s deadline, and executing blocking tasks cannot be aborted.

Normal orderly teardown joins the listener before runtime destruction (`:3131-3144`), so “the runtime is already shutting down” is not the ordinary production ordering. Pool saturation and synchronous executor blocking remain real holes in the bound.

The evidence’s **“≈10 s + unload”** also omits the preceding container teardown budget: `stop_workers()` precedes the first grace (`:3259-3266`), and T3’s budget is **25 seconds** (`child.rs:30`). Even assuming the sweep were bounded, nominal budget arithmetic is **25 + 5 + 5 + unload**, not ten seconds from SIGTERM. Unload itself performs synchronous journal writes. A worst-case exit comfortably below 45 seconds is therefore **not proven**.

### 3. Locks: no new lock-order deadlock found

- `registry.active()` clones the cores and releases the registry mutex (`:1411-1415`) before the sweep waits.
- The sweep retains no scheduler or journal lock during its Condvar wait.
- `wait_timeout` releases the binding mutex, allowing `finish` and the completion guard to proceed.
- The executor runs outside the binding mutex (`:1314-1319`).
- Registry retirement uses registry→binding ordering (`:1380-1390`); the new wait introduces no inverse acquisition.

Executor lock contention can exceed the deadline, but that is the bounding defect above—not an identified circular lock dependency.

### 4. Readiness: sound correction; startup sibling unchanged

The parser (`root_shutdown_unload_16_3.rs:295-311`) accepts exactly two whitespace-separated fields, requires lowercase `true`, and parses the PID. Newlines, tabs and surrounding whitespace are harmless; malformed or additional output fails closed and retries.

Both [Podman 4.9.3](https://github.com/containers/podman/blob/v4.9.3/libpod/define/container_inspect.go) and [Podman 5.0](https://github.com/containers/podman/blob/v5.0.0/libpod/define/container_inspect.go) expose `Pid` as an integer and `Running` as a boolean. Both versions’ inspect implementations execute custom Go templates. No incompatible representation was found by source inspection.

Sibling review:

- **v4/ac3-door-unload:** wait for the live hanging Worker (`:1530`, `:1583`).
- **v4b/v5c:** additionally wait for grandchild evidence (`:1643-1645`, `:2494-2502`).
- **v5:** waits for a live delegated Worker (`:2298-2303`).
- **v5b intentionally signals during daemon startup**, but does **not** use this helper. It signals immediately upon the armed marker (`:2368-2418`), so its meaning is unchanged.

The correction record’s implication that v5b also waits through this helper is inaccurate.

### 5. Output honesty

- Interrupted wording and nonzero exit are appropriate; `process::exit(1)` remains at `:3335`.
- “May be lost” correctly avoids claiming successful journaling or draining.
- Sweep-task failure reporting is appropriately qualified (`:3299-3303`).
- **“5s after the stop” is not the measured interval** (`:3293-3295`): the initial grace already elapsed, and the additional deadline starts inside the sweep. Describe the disposition-wait budget or report measured elapsed time instead.
- The descendant explanation remains qualified with “may”.

## RC verdict

**NO SHIP — overall correctness: incorrect.**

The original synchronization race is addressed, and the readiness correction preserves the named siblings’ intent. However, the new blocking sweep has no end-to-end deadline, so the fallback’s claimed bounded exit is unsupported.

**Counts:** P0: 0 · P1: 0 · P2: 1 · P3: 0  
**Overall confidence:** 0.96

**Unverified:** runtime execution, forced-race/panic behavior, pool saturation, hosted T3 behavior and actual shutdown duration. No tests or builds were run. The correction record also reports a failing KLOC gate; that result was not rerun.