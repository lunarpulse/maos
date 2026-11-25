# Plant runs (round 2) — failing assertion per iteration

Host: Linux 7.3.3-1-cachyos, 32 cores. Driver: `/tmp/s3fix-loop2.sh` (not committed): 20 runs of the kernel-core lib
test binary filtered to one test, plain, then 20 more under 16 `while :; do :; done` busy loops. For every
non-zero exit it kept the first panic location and the `left:`/`right:` lines of that iteration's own log
(no log is overwritten before it is read). Marker `S3FIX-PLANT`; each plant is removed afterwards
(`grep -rn S3FIX-PLANT crates xtask .github docs` = 0 matches).

`child_follower_pidfd.rs:281:5` is the `assert_eq!` in `assert_sigkill_pidfd_facts` (the payload/source pair); the
two tests share it, the filter names which test ran.

| Variant | Test | Mode | Green | Red | Failing assertion, grouped (count) |
|---|---|---|---|---|---|
| fixed code | both | plain | 20 | 0 | - |
| fixed code | both | busy16 | 20 | 0 | - |
| plant 1: `facts_by` expiry branch -> `new(Err(DeadlineExpired), None)` | `expired_snapshot_follower_keeps_the_pidfd_fact_s3` | plain | 0 | 20 | 20 x `child_follower_pidfd.rs:281:5`, `left: (Unknown, None)`, `right: (Signaled(9), Pidfd)`; iterations 1-20 all identical |
| plant 1 | same | busy16 | 0 | 20 | 20 x same line, same left/right; iterations 1-20 |
| plant 1 | `capped_follower_fallback_keeps_the_pidfd_fact_s3` | plain / busy16 | 20 / 20 | 0 / 0 | - |
| plant 2: `facts_capped` fallback -> `new(Err(DeadlineExpired), None)` | `capped_follower_fallback_keeps_the_pidfd_fact_s3` | plain | 0 | 20 | 20 x `child_follower_pidfd.rs:281:5`, `left: (Unknown, None)`, `right: (Signaled(9), Pidfd)`; iterations 1-20 |
| plant 2 | same | busy16 | 0 | 20 | 20 x same; iterations 1-20 |
| plant 2 | `expired_snapshot_follower_keeps_the_pidfd_fact_s3` | plain / busy16 | 20 / 20 | 0 / 0 | - |

Per-iteration rows as recorded (plant 1, plain; busy16 and plant 2 are the same 20 identical rows):

```
plain iter 1..20 | child_follower_pidfd.rs:281:5 | left: (Unknown, None) | right: (Signaled(9), Pidfd)
```

Every red is the payload assertion `(Unknown, None)` against `(Signaled(9), Pidfd)`; none is a timeout, a
rendezvous panic, a poisoned lock or a different assertion.
