# s3-pidfd-follower-fix — evidence (2026-11-26)

Branch `s3-pidfd-follower-2026-11-26` (from main 4c167d7f). RCA: `s3-pidfd-unknown-rca-2026-11-26.md`
(hosted failure `worker_terminal_facts_17_7::s3_external_sigkill_requires_pidfd_fact`, `payload_exit == unknown`).

## Hypothesis check (H1) against the code

Confirmed by reading, not by reproducing the hosted run. `ContainerTerminal::facts_by`
(`crates/maos-kernel-core/src/security/sandbox/t3/child.rs`) had a follower branch that, at
`now >= snapshot_deadline`, froze `ContainerTerminalFacts::new(Err(DeadlineExpired), None)` with
`get_or_insert` BEFORE the owner (still inside `observe`, whose `pidfd_exit(by)` runs only after its inspect loop)
could publish. `ContainerTerminalFacts::new(_, None)` with no runtime code gives `(Unknown, None)`. The second site,
`wait_and_finalize` in `lifecycle/cli_wrapper/runtime.rs`, built an unfrozen `DeadlineExpired` fallback with `None`
for a capped follower. The exit row's `container` object and the runner kernel are still unknown for the hosted
failure (see Residual).

## Change

- `child.rs`: new `ContainerTerminal::unavailable_facts()` = `new(Err(DeadlineExpired), self.pidfd_exit(now))`;
  the `facts_by` expiry branch uses it; new `facts_capped(cap)` = `facts_by(cap, true)` or a local (unfrozen)
  `unavailable_facts()`; `runtime.rs` calls `facts_capped`.
- Non-blocking: `pidfd_exit(now)` is one `PIDFD_GET_INFO` ioctl and returns at the first `Ok(None)` before `pause`.
- Lock order (documented at `unavailable_facts`): `facts_by` holds `progress` and the probe takes `identity`
  (progress -> identity). Nothing takes identity -> progress (`kill_payload`, `pidfd_ready`, `has_pidfd`, `started`,
  `arm`, `pidfd_exit` take identity only), so no cycle.
- Diagnostics: `s3_diagnostics` in `crates/maos-bin/tests/worker_terminal_facts_17_7.rs` now starts with
  `kernel <uname -r>; exit rows [<every cli.subprocess.exit payload>]`, before the 768-byte-truncated tail and the
  stderr.
- Test: `crates/maos-kernel-core/tests/unit/child_follower_pidfd.rs`, compiled into the lib test binary via `#[path]`
  from `child.rs` (needs the `pub(crate)` `arm`; same convention as `hot_swap/coordinator.rs`).
  - `expired_snapshot_follower_keeps_the_pidfd_fact_s3`: 8 instances; stub runtime whose `inspect` hangs
    (`exec sleep 30`); an external payload (grandchild of an owner shell) with its pidfd open, SIGKILLed and reaped;
    owner `facts_by(now+300ms, false)`; the gate is the stub's own call log (`inspect` entered), then a follower
    `facts_by(now+20s, false)` waits to the owner's deadline. Asserts one `Arc` (ptr_eq), `state` is `Err`, and
    `(Signaled(9), Pidfd)` for both. No sleep is an oracle; no assertion reads a clock.
  - `capped_follower_fallback_keeps_the_pidfd_fact_s3`: owner cap 3 s (generous so the snapshot is in flight on a
    loaded host); `facts_by(now, true)` is `None`; `facts_capped(now)` carries `(Signaled(9), Pidfd)`.
  - Gate: `/proc/sys/kernel/osrelease` >= 6.15 (the production `PIDFD_INFO_EXIT` requirement). Below it:
    without `CI`/`GITHUB_ACTIONS` it prints `SKIP: <test>: Linux >= 6.15 ... (kernel X predates it)`; with a
    declared CI it PANICS. When it runs it prints `<test>: RUN (kernel X supports PIDFD_INFO_EXIT)`.

## Observed (this host: Linux 7.3.3-1-cachyos, 32 cores)

- Both tests RUN here (not skipped): `RUN (kernel 7.3.3-1-cachyos supports PIDFD_INFO_EXIT)`.
- Green with the fix: `cargo test --locked -p maos-kernel-core --lib follower_pidfd`: 2 passed.
- 20 consecutive runs of the lib test binary filtered to `follower_pidfd`: 20/20 pass. 20 more under 16 busy
  shell loops (`while :; do :; done`) on 32 cores: 20/20 pass. (16 loops on 32 cores is mild contention.)
- Plants (marker `S3FIX-PLANT`, one site at a time, each removed afterwards; `grep -rn S3FIX-PLANT crates xtask
  .github docs` finds nothing):
  - plant 1, `facts_by` expiry branch back to `new(Err(DeadlineExpired), None)`: `expired_snapshot_...` RED at
    instance 0 follower: `left: (Unknown, None) right: (Signaled(9), Pidfd)`; the capped test stayed green.
  - plant 2, `facts_capped` fallback back to `None`: `capped_follower_...` RED with the same
    `(Unknown, None)`; the expired test stayed green.
- The `runtime.rs` call site itself is not driven by a test (it needs a bridge with a container child); it is a
  one-line call of the tested `facts_capped`.
- `cargo fmt --all -- --check` exit 0; `cargo check --locked -p maos-bin` exit 0 (default and
  `--no-default-features`); `check-composition-root-completeness` PASS (11 adapters, 20 constructions);
  `maos-bin` test `worker_terminal_facts_17_7` compiles (`--no-run`; it needs real podman, not run here);
  `t3_terminal_helpers_17_7`: 8 passed; kernel-core lib `t3::` tests: 10 passed.

## Kernel pin

Measured with an xtask built from this tree. Kernel src 28372 -> 28390 physical (+18), tokei 22178 (alarm 23000,
row 24000: inside the envelope; `kloc-check` PASSED). `scripts/repin.py --apply --story s3-pidfd-follower-fix`
rewrote `xtask/kernel-core-baseline.toml` (`src_lines`, `set_hash`, two digests: `lifecycle/cli_wrapper/runtime.rs`,
`security/sandbox/t3/child.rs`); `--check` afterwards: PINS IN SYNC. The script writes no HISTORY block; history is
the commit message: "kernel-core src 28372 -> 28390 (+18 physical), tokei 22178, driver: s3-pidfd-follower-fix".
The story row `s3-pidfd-follower-fix` was added to `sprint-status.yaml` as `review` (repin refuses `done`).

## What hosted CI must prove

1. `worker_terminal_facts_17_7::s3_external_sigkill_requires_pidfd_fact` stays green over repeated main runs; if it
   fails again, the new leading `kernel ...; exit rows [...]` text identifies the cause (`payload_exit_source`).
2. The `follower_pidfd` lib tests RUN (not SKIP) on the runner: look for `RUN (kernel ...)` in the log.

## Residual (deferred-work style)

- **Unverified kernel of the hosted runner.** The failing run's exit row and `uname -r` were never logged, so H5
  (pidfd `None` at arm time: cgroup not yet naming the CID) and H6 (kernel < 6.15) are not excluded by evidence,
  only by argument (same image passed other runs). If the diagnostic shows kernel < 6.15, the pidfd contract
  (`require_pidfd_exit`) is the defect, not this follower path. Owner: the 20-7 family / E20-A4. Trigger: the next
  hosted occurrence of `payload_exit unknown`.
- The fix keeps a pidfd fact only when the payload has already exited at the instant the follower expires; a
  payload still running at that instant legitimately freezes no pidfd fact (unchanged behaviour).
