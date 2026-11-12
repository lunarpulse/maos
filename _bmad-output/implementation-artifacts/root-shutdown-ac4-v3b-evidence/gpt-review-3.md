## Findings

None. **P0: 0 · P1: 0 · P2: 0 · P3: 0**

### 1. Round-2 P2: closed

`supervision.rs:3291-3297` records the start before `spawn_blocking`, passes an absolute deadline, and wraps the join in `timeout(ROOT_GRACE, sweep)`. This covers blocking-pool queueing and synchronous disposition execution **from the listener’s waiting perspective**. The sole consumer passes that deadline to `disposition_stopped_unfinished` (`:2241-2248`).

### 2. Timeout race, detached task, and locks

- **Simultaneous deadlines:** the documented generic-timeout outcome is acceptable. It reports uncertainty rather than falsely identifying unfinished Workers.
- **No introduced lock-order deadlock or journal corruption found.** The executor releases SCB/ledger guards before journal insertion (`supervision.rs:2677-2703`). The disposition and unload share the same Transparency Log adapter, whose `FairMutex` serializes connection access (`transparency_log.rs:675-677,1379-1380`).
- **Important limit:** if the detached sweep remains stuck holding that journal mutex, unload can subsequently wait for the same mutex (`scheduler_loop.rs:539-546`). The timeout therefore bounds the sweep await—not reaching `process::exit`. This is the acknowledged untimed-unload limitation, not a newly demonstrated circular deadlock.
- Exiting while a blocking thread remains active can lose its unfinished nack. No patch-introduced corruption path was identified; completed autocommit inserts do not require Rust destructors to commit (`transparency_log.rs:1387-1445`).

### 3. Wording

The elapsed-time diagnostics now describe the **disposition budget**, not “5s after the stop” (`supervision.rs:3301-3318`). They truthfully qualify possible nack loss.

“The sweep cannot be aborted; exiting is the bound” must not be interpreted as a bounded total shutdown: unload still precedes exit (`:3321-3352`).

### 4. Evidence arithmetic

**25 + 5 + 5 = 35 seconds, plus untimed unload** is accurate nominal arithmetic:

- Shared teardown deadline: `supervision.rs:2056-2057`; `child.rs:30`.
- Initial grace: `supervision.rs:3260-3266`.
- Sweep wait: `:3291-3297`.
- Test exit wait: `root_shutdown_unload_16_3.rs:1441-1442`.

The resulting ten-second margin is **not a guaranteed margin**, as Correction 2 explicitly acknowledges.

## Verdict

**Overall correctness: correct. Code-review verdict: SHIP Correction 2.** No concrete new blocking defect found; the round-2 P2 is addressed. The evidence’s reported KLOC failure remains uncleared and is separate from this correctness verdict.

**Confidence: 0.94.** Static review only; no builds, tests, or writes performed.