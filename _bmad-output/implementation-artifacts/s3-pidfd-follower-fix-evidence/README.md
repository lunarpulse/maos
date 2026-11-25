# s3-pidfd-follower-fix — evidence (2026-11-26, revised after the GPT review)

Branch `s3-pidfd-follower-2026-11-26` (from main 4c167d7f). RCA: `s3-pidfd-unknown-rca-2026-11-26.md`
(hosted failure `worker_terminal_facts_17_7::s3_external_sigkill_requires_pidfd_fact`, `payload_exit == unknown`).

## Hypothesis check (H1) against the code

Confirmed by reading (and by the GPT review's production trace), not by reproducing the hosted run.
`ContainerTerminal::facts_by` (`crates/maos-kernel-core/src/security/sandbox/t3/child.rs`) had a follower branch
that, at `now >= snapshot_deadline`, froze `ContainerTerminalFacts::new(Err(DeadlineExpired), None)` BEFORE the
owner (still inside `observe`, whose `pidfd_exit(by)` runs after its inspect loop) could publish. `new(_, None)` with
no runtime code gives `(Unknown, None)`. The second site, `wait_and_finalize` in `lifecycle/cli_wrapper/runtime.rs`,
built an unfrozen `DeadlineExpired` fallback with `None` for a capped follower. The hosted exit row's `container`
object and runner kernel are still unknown (see Residual).

## Change

- `child.rs`: `ContainerTerminal::unavailable_facts()` = `new(Err(DeadlineExpired), self.pidfd_exit(now))`, used by
  the `facts_by` expiry branch; `facts_capped(cap)` = `facts_by(cap, true)` or a local (unfrozen)
  `unavailable_facts()`, called by `runtime.rs`. The probe is one `PIDFD_GET_INFO` ioctl (`pidfd_exit(now)` returns
  at the first `Ok(None)` before `pause`). Lock order: `progress` -> `identity` only (documented at
  `unavailable_facts`); nothing takes identity -> progress.
- ONE `cfg(all(test, target_os = "linux"))` hook in the owner path of `facts_by`
  (`follower_pidfd_tests::hold_owner_before_publish()`, between `observe` and the publish lock). It is a no-op
  unless a test thread installs a gate; it does not exist in non-test builds.
- The test module is `cfg(all(test, target_os = "linux"))` (macOS/Windows `--all-targets` compile neither it nor
  the hook; it imports `sandbox::linux` and calls the Linux-only `arm`). `tests/unit/child_follower_pidfd.rs`
  is reachable only through that `#[path]` module (it is not a cargo integration target), and the `ci.rs` harness
  is included from inside it.
- Diagnostics: `s3_diagnostics` now starts with `kernel <uname -r>; exit rows [...]` (rows capped at 8 KiB with a
  `…[truncated N bytes]` marker; a failing `uname` prints `unavailable (<error>)`), before the 768-byte-truncated
  tail and the stderr.

## Tests (`crates/maos-kernel-core/tests/unit/child_follower_pidfd.rs`)

Branches are FORCED, not raced. The owner thread installs a thread-local gate; `facts_by` holds it after
`observe` and before publishing. The stub runtime's `inspect` fails fast (no stalled stub, no sleeps, no 300 ms
window); hang guards are 10 s upper bounds that fail loudly.

- `expired_snapshot_follower_keeps_the_pidfd_fact_s3` (4 instances): owner cap `Instant::now()` (the snapshot
  deadline is already expired), owner held; the follower's `facts_by(now+20s, false)` MUST take the expiry branch
  (asserted: `state == Err(DeadlineExpired)`, a state only that branch builds, while the owner's own would be
  `InspectFailed`/`InspectTimedOut`) and carry `(Signaled(9), Pidfd)`; after release the owner returns the same `Arc`.
- `capped_follower_fallback_keeps_the_pidfd_fact_s3`: owner cap now+3600 s (deadline = the 10 s `SNAPSHOT_BUDGET`
  ahead: a precondition that is never asserted on a clock and cannot be reached while the owner is held); owner held
  ACROSS both `facts_by(now, true) == None` and `facts_capped(now)`; the local record is `DeadlineExpired` with
  `(Signaled(9), Pidfd)`; after release the owner publishes `Err(InspectFailed)` + `(Signaled(9), Pidfd)`, and the
  local record is not the frozen one (`!ptr_eq`, `terminal.facts()` is the owner's).
- Fixture: SIGKILL goes through the held pidfd (`pidfd_send_signal`), never the numeric PID; the two pipe reads are
  bounded by a 10 s `recv_timeout`; `PayloadShell` kills the payload through its pidfd and kills and reaps the shell
  on drop (also on a failed assertion).
- Gate = a capability probe, not a version check: after the payload is killed and reaped, `pidfd_exit_status` must
  return `signaled(9)`. Otherwise: without `CI`/`GITHUB_ACTIONS` print `SKIP: <test>: PIDFD_INFO_EXIT capability
  probe failed (...) on kernel X`; with a declared CI PANIC. When it runs it prints `RUN (kernel X: PIDFD_INFO_EXIT
  probe returned signaled(9))`. This host (Linux 7.3.3-1-cachyos) RUNS both.

## Observed

- Fixed code: `follower_pidfd` lib tests 20/20 green plain, 20/20 green under 16 busy loops (32-core host).
- Plant 1 (`S3FIX-PLANT`, `facts_by` expiry branch back to `new(Err(DeadlineExpired), None)`): the expired test
  20/20 RED plain and 20/20 RED under 16 busy loops; the capped test stayed 20/20 + 20/20 green.
- Plant 2 (`facts_capped` fallback back to `None`): the capped test 20/20 RED plain and 20/20 RED busy; the expired
  test stayed 20/20 + 20/20 green.
- Both plants removed; `grep -rn S3FIX-PLANT crates xtask .github docs` finds nothing.
- `cargo fmt --all -- --check` rc 0; `cargo check --locked -p maos-bin` ok (default and `--no-default-features`);
  `worker_terminal_facts_17_7` compiles (`--no-run`; needs real podman, not run); `t3_terminal_helpers_17_7`: 8
  passed; kernel-core lib `t3::` tests: 10 passed.
- macOS: `cargo check -p maos-kernel-core --all-targets --target aarch64-apple-darwin` was attempted (the target is
  installed) and FAILED in `ring`'s build script (no Darwin C cross toolchain on this host). It was not run to
  completion; the evidence that the gate is sufficient is by construction (every Linux-only item added is under
  `cfg(all(test, target_os = "linux"))`; `pidfd_exit`/`unavailable_facts`/`facts_capped` use the existing
  non-Linux `pidfd_exit` stub). The hosted macOS guard must prove it.

## Kernel pin

Kernel src 28372 -> 28390 -> 28394 (+22 physical over main, +4 for the test hook), tokei 22180 (alarm 23000, row
24000; `kloc-check` PASSED). `scripts/repin.py --apply --story s3-pidfd-follower-fix` rewrote
`xtask/kernel-core-baseline.toml`; `--check`: PINS IN SYNC. History is the commit message: "kernel-core src 28372 ->
28394 (+22 physical), tokei 22180, driver: s3-pidfd-follower-fix". Story row in `sprint-status.yaml` is `review`
(repin refuses `done`).

## What hosted CI must prove

1. `s3_external_sigkill_requires_pidfd_fact` stays green over repeated main runs on ubuntu-24.04 and 26.04, including
   a genuinely stalled-inspect sample (a green run where inspect answers does not prove the fix); on failure the
   leading `kernel ...; exit rows [...]` identifies the cause.
2. The two `follower_pidfd` lib tests print `RUN (kernel ...)`, not `SKIP`, on the runner.
3. The macOS `cargo check --workspace --all-targets` guard is green.
4. The kernel content-hash suite (`xtask/tests/kernel_pin_content_hash_16_0.rs`) passes (not run here).

## Residual (deferred-work style; owner: the 20-7 family / E20-A4; trigger: next hosted `payload_exit unknown`)

- **Unverified kernel of the hosted runner.** The failing run's exit row and `uname -r` were never logged. H5 (pidfd
  `None` at arm time) and H6 (no `PIDFD_INFO_EXIT`) are excluded only by argument. The hosted log cannot tell
  follower expiry from a missing pidfd, an unsupported/failing ioctl or an undecodable status.
- **Not covered by the fix:** a payload SIGKILLed but NOT YET reaped when the follower probes at expiry: the follower
  freezes `Unknown`, the owner's later probe would obtain 9 but loses to the frozen value. Likewise an unsupported or
  failing pidfd ioctl, a missing pidfd, or an undecodable status with no/ambiguous runtime code (e.g. 137) still
  publish `unknown`/`none`. The tests reap before either probe and do not cover this interval.
- The `facts_capped` local record is unfrozen and may disagree with the frozen snapshot (predates this change; the
  bridge can journal the local value).
- `pidfd_ready` holds the `identity` mutex across a poll (up to one 50 ms `pause` slice), so the new probe can wait on
  `identity` for that long. No identity -> progress acquisition exists, so no deadlock.
- The version gate was replaced by a capability probe in the tests only; production `pidfd_exit` already is a
  capability use (an `Err` returns `None`). `exit_row` in the S3 test can still panic before the enriched message on a
  missing, duplicate or malformed row.

## Round 2 (test defects fixed; production unchanged)

- Gate lock: `wait_reached` copies the arrival flag and drops the lock before asserting; `release`/`hold` recover from
  poisoning (`PoisonError::into_inner`), so `ReleaseOnDrop` can never abort the lib-test process. A missed release makes
  the OWNER thread panic outside the lock (a panic in a spawned thread cannot abort other tests; the test's
  `join().unwrap()` reports it). On that failure path the dead owner leaves the snapshot in flight, so the test's
  `Armed` teardown may wait up to the snapshot deadline before the failure surfaces.
- Capped test deadline: after the owner is held, the test widens `snapshot_deadline` to now+24 h through the
  private `terminal.lock()` (the test module is a descendant of `child`, so no production seam). The owner does not
  re-read it; neither follower call can cross a deadline however long the thread is descheduled. The rendezvous guard
  (10 s) is separate.
- Fixture lifetime: the shell reaps the payload only after the parent writes the release pipe (`read _` before
  `wait`), so the payload's PID stays reserved until the parent has opened and cloned its pidfd. The guard closes
  the pipe on any exit path; the shell then SIGKILLs and reaps the payload itself (also when pidfd acquisition
  failed), and the guard kills the shell only after a 10 s bounded wait. Because of that the shell's `kill -9 $p` is a
  harmless duplicate of the pidfd kill in the normal path. If the shell itself hangs past 10 s the sleeper can outlive
  the test by up to its 30 s.
- Diagnostics: the 8 KiB cap now includes the `…[truncated]` marker.
- Plant evidence with the failing assertion per iteration: `plant-runs.md` (20/20 plain and 20/20 under 16 busy
  loops, each red is the payload assertion `(Unknown, None)` vs `(Signaled(9), Pidfd)`; fixed code 20/20 green both).
- Kernel src unchanged at 28394 (no re-pin); `repin.py --check` PINS IN SYNC; tokei 22180.
- Residual: a non-waiting re-read of the frozen snapshot after teardown and before journaling in the bridge could
  make the journaled value agree with a snapshot that appeared meanwhile. The reviewer advised against it in this PR;
  not done.

## Round 3 (fixture rewritten; supersedes the round-2 shell description above)

- The shell fixture is gone. The payload is `sleep 30` spawned DIRECTLY by the test (null stdio, nothing inherited)
  and kept unreaped until the end, so its PID cannot be recycled; `pidfd_open(child.id())` is safe; the kill goes
  ONLY through the pidfd (`pidfd_send_signal`), then `Child::wait` reaps it and asserts `signal() == Some(9)`. The
  guard kills through the pidfd if held, else `Child::kill` (safe: unreaped), and waits, on every path. No
  numeric-PID signalling and no shell anywhere.
- Consequence, disclosed: the payload is now the test's own child, not a grandchild. The production arm path takes
  only `(pid, OwnedFd)`, so nothing in it needs a grandchild; the capability probe still reads the pidfd exit data
  AFTER the reap and returned `signaled(9)` on this kernel (7.3.3), so the probe covers "exit data survives the reap
  for an own child". On a kernel where it does not, the tests SKIP locally / PANIC under a declared CI (loud).
- The owner's runtime-derived failure in the capped test accepts `InspectFailed` or `InspectTimedOut` (a descheduled
  owner can see its initial deadline pass before `inspect`); `DeadlineExpired` is still rejected, and the owner's
  `Arc` must still be the frozen one. The expired test never asserts the owner's own state (it must equal the
  follower's frozen `DeadlineExpired`), so it has no such assumption.
- Evidence redone on the new fixture with a driver that DECLARES CI (a skip would panic), counts `RUN (kernel ...)`
  lines per iteration and fails on `SKIP`: `plant-loop-driver.txt` (the driver text) and `plant-runs.md` (fixed 20/20 +
  20/20 busy16; each plant red 20/20 + 20/20 on the payload assertion `(Unknown, None)`; the other test green).
- Kernel src unchanged (28394, tokei 22180): no re-pin.

## Round 4 (final fixture and driver; supersedes the `sleep 30` payload of round 3)

- The payload is `cat` spawned directly (no shell, stdout/stderr null) with its stdin PIPED and held by the parent, so
  it blocks until the parent kills it (no timer can end it early); it stays unreaped, is signalled only through the
  pidfd, then reaped. The stdin handle is a field of the `Payload` guard, so it is closed only AFTER `Drop` has run
  the kill and the reap (asserted in the struct's doc comment); EOF on a harness exit also ends `cat`.
- Driver hardened (`plant-loop-driver.txt`): a red iteration needs exit 101, `test result: FAILED`, the
  `(Unknown, None)` payload assertion and the branch-specific message (the capped test's owner assertion is not
  accepted); the script exits non-zero if any iteration is BAD; paths derive from the script location
  (`S3FIX_ROOT`, `CARGO_TARGET_DIR`, `S3FIX_LOG_DIR` override); prerequisites are at the top of the file.
- All loops redone on this fixture: `plant-runs.md` (fixed 20/20 plain + 20/20 busy16; plant 1 and plant 2 each red
  20/20 + 20/20 busy16 on the payload assertion, the other test green 20/20 + 20/20; every driver run exited 0).
- Kernel src unchanged (28394, tokei 22180): no re-pin.
