# F1 — a guest `console.log` (stdout/stderr write) killed the runner under T2

Date 2026-11-12, linux x86_64, local, kernel files untouched. Hosted symptom: runs 43185836600
(jobs 131099823892 `idle`, 131099823907 `swap-in`) and 20-8 run 43188003155 (plant leg):
`wasmtime-wasi-48.0.8/src/runtime.rs:32:10 … Os { code: 1, PermissionDenied }`, exit 101,
`unplanned_crash`, `Spirit not loaded`; in a hot swap `hot_swap.aborted … lifecycle_child_closed`.

## Root cause (CONFIRMED, and different from the thread-creation inference)

* Every guest stdout/stderr write, `poll`, and stream-resource drop goes through wasmtime-wasi's
  synchronous host bindings, which call `runtime::in_tokio` (`wasmtime-wasi-48.0.8/src/p2/host/io.rs:30-122`).
  With no ambient tokio runtime, `in_tokio` forces `static RUNTIME`, built at `runtime.rs:27-32` as
  `Builder::new_multi_thread().enable_time().enable_io().build().unwrap()`.
* The I/O driver creates an epoll instance and an eventfd waker (`epoll_create1`, `eventfd2`). The T2
  seccomp allow-list (`crates/maos-kernel-core/src/security/sandbox/linux.rs:273-332`, default action
  `Errno(EPERM)`, line 381) does not list them, so the build returns EPERM and the `unwrap` at
  `runtime.rs:32:10` panics: exit 101.
* Thread creation is NOT the blocker: `clone`/`clone3` are on the allow-list (lines 292, 329).
* Observed (`syscall-probe-under-t2.log`, probe `syscall-probe.c` run through the real `spawn_sandboxed`
  at T2): `epoll_create1`, `eventfd2`, `timerfd_create`, `socketpair` -> `-1 EPERM`; `pthread_create` -> ok.
  The same probe unsandboxed returns ok for all.
* It is not about stdout specifically: it is the first thing in the runner that needs the ambient runtime.
  The plain build never writes to stdio and never polls, so it never reached it.

## Fix (runner only; sandbox allow-list unchanged)

`crates/maos-wasm-host/src/runner.rs` `run()` now builds
`tokio::runtime::Builder::new_current_thread().build()` (no I/O driver, no time driver) and holds its
`enter()` guard for the whole run. `in_tokio` then finds an ambient `Handle` and drives its futures on
the runner's own thread: `Handle::try_current()` is `Ok`, so the lazy multi-thread runtime is never built.
`crates/maos-wasm-host/Cargo.toml` adds `tokio = { default-features = false, features = ["rt"] }`
(already in the lock; the native-twin fixture lock gets the one `"tokio"` edge under `maos-wasm-host`).

Security argument:
* No syscall is added to or relaxed in any filter: a current-thread runtime without drivers issues no
  syscall beyond allocation (mmap/brk/futex already allowed), spawns no thread, creates no fd.
  The guest gains no capability. The two things it did not have remain unavailable and are NOT enabled:
  epoll/eventfd/timerfd/socketpair stay EPERM for the whole process.
* Guest output still goes only to the runner's stderr (`host_state.rs` `stdout(stderr())`), never to
  the frame channel on stdout; the test asserts the frame channel stays framing-clean.
* Known limit, same as before the fix and now failing loudly rather than by the runner dying on an unrelated
  panic: a guest that subscribes to a monotonic-clock timer (setTimeout) is expected [INFERENCE, not exercised] to panic in tokio
  ("timers are disabled"); `Handle::block_on` on a current-thread runtime cannot drive timers, so enabling the time
  driver would hang instead of fail. Not widened. Fuel does not bound host calls, so a hang is worse.

## Witnesses (all `cargo test -p maos-wasm-host --test guest_stdio_t2_20_9`, real `spawn_sandboxed` T2)

New guest command `say NAME TEXT` in `guests/lifecycle-probe` (writes TEXT to guest stdout AND stderr
inside the named export or, for `frame`, inside `handleFrame`).

| Log | Runner | Result |
|---|---|---|
| `red-before-fix.log` | unmodified | 2 FAILED / 1 ok (negative control passes); child stderr `runtime.rs:32:10 … Os { code: 1, kind: PermissionDenied }`, `left: Some(101) right: Some(0)` — the hosted signature, reproduced locally |
| `green-after-fix.raw.log`, `green-after-fix-final.raw.log` | fixed | 3 passed |
| `plant-ambient-runtime-removed.raw.log` | fixed runner with the ambient-runtime lines removed (the pre-fix path) | 2 FAILED / 1 ok, same panic; restored by copy-back + `cmp` + `touch`, runner fixture rebuilt |
| `focused-tests.txt` | fixed | `lifecycle_runner_20_9` 4/4, `recall_runner_17_3d` 15/15, `t2_sandbox_kill` 5/5, `e2e_roundtrip` 8/8, `fuel_t2_matrix` 8/8 |

Vector 1 covers `handleFrame` (`say frame`), `idle`, and `swap-in` (the swapped-in counter then
continues: `state counter=42 swap-in=41`) in one live runner; vector 3 covers `on-shutdown`; the negative
control is the same guest, same sandbox, no writes. Every written line is asserted on the runner's stderr
(both streams), exit code 0, no `panicked`.

Real TypeScript component (not committed, needs componentize-js): the example with `console.log` in
`idle()`/`swapIn()` and `console.error` in `idle()`, componentized locally with the example's
`node_modules`, run under the same T2 spawn: fixed runner `ts-component-under-t2-after-fix.log`
(idle, snapshot "1", swap-in "7", idle, snapshot "8": all replies, exit 0, stderr carries every line);
planted runner `ts-component-under-t2-planted.raw.log` (dies on the first idle).

## Unverified

The hosted legs: the re-run of `ci/20-9-verify`'s `console.log` matrix cases (the two jobs that were
131099823892 / 131099823907 in run 43185836600) on a commit with this fix is the hosted proof; the
local runs above are the same T2 mechanism (Landlock + seccomp via `spawn_sandboxed`) on one host
(Linux 7.x, x86_64). arm64 images are not locally verified.
