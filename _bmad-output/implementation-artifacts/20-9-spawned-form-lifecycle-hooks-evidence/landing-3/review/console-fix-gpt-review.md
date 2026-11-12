## Finding requiring correction

### [P2] Apply the approved source-size repin before merging
**Location:** `crates/maos-wasm-host/src/runner.rs:182–185`  
**Confidence:** 0.99

The runner and probe additions raise charged `maos-wasm-host` source from 3,149 to 3,178 lines, while `xtask/kloc.toml:703` still permits 3,149. The supplied `landing-3/gates/kloc-check.json` explicitly reports `maos-wasm-host 3178 > 3149`; `xtask/src/kloc_check.rs:362–371` makes that a failing gate. Apply the approved **measured +29** repin and reconcile the aggregate pin during integration—not the entire +60 allowance. No additional behavioral witness is needed for this finding; final integrated accounting remains necessary.

## Correctness and test assessment

1. **Root cause is consistent with upstream source and T2.**  
   Wasmtime-WASI 48.0.8 `runtime.rs:27–32` builds the fallback multi-thread runtime with time and I/O enabled, then unwraps construction. Tokio 1.53.3 `runtime/io/driver.rs:118–121` creates Mio’s poller and waker; Mio’s Linux implementation calls `epoll_create1`. T2 omits epoll/eventfd syscalls and defaults to EPERM (`linux.rs:273–332,381`). The supplied syscall probe corroborates this; permitted `clone`/`clone3` contradict thread creation being the immediate blocker.

   One wording correction: **not every write invokes `in_tokio`**. Nonblocking `write` is forwarded directly; blocking flush/write and stream destruction invoke it. The planted shutdown log actually shows stdout emitted before `blocking_flush` panics.

2. **Guard lifetime is correct for the runner’s existing paths.**  
   `ambient` and its guard precede the store, linker and guest instance. Reverse destruction therefore keeps the runtime entered through resource destruction, ordinary early returns and panic unwinding. All three binding selections, initialization, frame turns, lifecycle calls and normal best-effort shutdown remain inside that scope.

   The runner uses synchronous generated bindings and `add_to_linker_sync`; upstream asynchronous implementations execute through those shims. The compilation worker executes compilation, not guest host calls. Fuel exhaustion returns through the existing shutdown path. Epoch interruption is not configured.

   This is **not a general background-task executor**: entering a current-thread runtime does not drive spawned async tasks, and `Handle::block_on` does not drive its scheduler/drivers. I found no affected background-task dependency in the configured direct-stderr/closed-stdin/no-preopens/network-disabled context. No patch-introduced starvation path is established.

3. **Timer and back-pressure guarantees need precise qualification.**
   - A normal positive monotonic-clock subscription creates a deadline; polling/blocking/asking readiness constructs `sleep_until` (`p2/host/clocks.rs:160`). Tokio’s `sleep.rs:254–256` immediately checks the absent timer driver and panics. Subscription alone need not panic. Zero-duration subscriptions use yielding rather than a timer.
   - Thus the ordinary finite-timer path has a source-supported immediate failure, **not the disabled-driver hang** feared here. Its exact runner exit behavior lacks an exercised witness.
   - This preserves neither timers nor general behavior outside T2: the old fallback could provide functioning timers unsandboxed. Under T2, its construction already panicked.
   - **A globally bounded failure is not guaranteed.** The configured stream calls blocking `std::io::stderr().write_all/flush` (`cli/stdout.rs:78–91`). A stalled sink can block a host call indefinitely; guest fuel does not interrupt that syscall. Neither this patch nor the old stream implementation provides a host-write deadline.
   - Runtime shutdown can also wait indefinitely for blocking tasks in general. No reachable newly introduced blocking task was identified here.

4. **The three tests exercise the real boundary and meaningful assertions.**  
   They invoke the kernel’s `spawn_sandboxed` with T2, which installs Landlock and seccomp before exec. They exercise `handleFrame`, `idle`, `swap-in`, and shutdown, answer recall traffic, check continued swapped-in state, verify framing, verify both guest streams on host stderr, and require clean exit. Fixture loading rejects missing/stale fixtures. Permission-denied setup can skip locally, but declared CI fails closed.

   Reviewed logs show:
   - Original runner: **1 passed, 2 failed**.
   - Fixed runner: **3 passed**.
   - Removed-runtime plant: **1 passed, 2 failed**.

   The plant is meaningful, but its combined positive case fails at `say frame` **before reaching idle/swap-in**. Green assertions cover those exports; the plant does not independently demonstrate their failure. Shutdown independently reproduces the upstream panic.

5. **Lock handling is consistent for active builds.**  
   The root lock already contains the direct `"tokio"` edge because Tokio was already a dev-dependency. Native-twin’s added edge references its existing Tokio package; `rt` adds no dependency requirement beyond the already-enabled Wasmtime-WASI Tokio features. No lock-format/checksum defect was found.

   The bridge, host and q1-driver spike locks omit the new edge, so they are not current `--locked` graphs. However, they **already** omit `maos-frame-codec` and retain Wasmtime 46-era dependencies against today’s 48.x manifest. Their locked-build incompatibility predates this patch; it is not a newly introduced finding. Rust-guest and t2-ladder do not depend on `maos-wasm-host`.

6. **Accounting:** supplied measurement confirms **3,149 → 3,178 (+29)**, within the pre-adjudicated +60. The remaining correction is recording the measured grant in the enforced pins.

## Deferred evidence rows

| Level | Witness still absent |
|---|---|
| Info | Positive-duration timer under T2, with an external deadline and captured exit/stderr. |
| Info | Stalled-sink/back-pressure behavior; current small-output tests do not establish bounded host writes. |
| Info | Independent idle/swap-in plants, hosted matrix rerun, and arm64 execution. |

**Rejected findings:** hypothetical background-task starvation without a reachable consumer; intentionally disabled timers as a newly accidental T2 regression; pre-existing spike-lock drift.

**Counts:** P0: 0 · P1: 0 · P2: 1 · P3: 0.  
**Overall correctness:** incorrect pending the accounting correction; no production runtime defect meeting the reporting criteria was established.  
**Overall confidence:** 0.96.

Read-only review only: no builds, tests, mutations or witness reruns were performed. Results above are inspected supplied evidence, not newly executed verification.