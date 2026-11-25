# Plant runs (round 4) — final fixture (`cat` payload blocked on parent-held stdin)

Host: Linux 7.3.3-1-cachyos, 32 cores. Driver: `plant-loop-driver.txt` in this directory (read its header for
prerequisites, path variables and criteria). It declares `CI=true` (a capability skip would PANIC), counts
`RUN (kernel ...)` lines per iteration and fails an iteration on `SKIP`.
Green iteration: exit 0 and `test result: ok`. Red iteration: exit 101, `test result: FAILED`,
`left: (Unknown, None)` / `right: (Signaled(9), Pidfd)` and the branch-specific assertion message (the owner
assertion of the capped test is not accepted). The driver exits non-zero when any iteration is BAD (every command
below exited 0). 20 iterations plain, then 20 under 16 `while :; do :; done` loops; one log per iteration under
`/tmp/s3fix-driver/` (not committed).

Plants (marker `S3FIX-PLANT`, one at a time, removed afterwards; `grep -rn S3FIX-PLANT crates xtask .github docs`
= 0 matches). Plant 1 = `facts_by` expiry branch -> `new(Err(DeadlineExpired), None)`; plant 2 = `facts_capped`
fallback -> `new(Err(DeadlineExpired), None)`.

```
M="the verified pidfd fact survives an unavailable snapshot"
fixed code : bash plant-loop-driver.txt fixed          follower_pidfd    5 green
plant 1    : bash plant-loop-driver.txt plant1-expired expired_snapshot  1 red   "follower: $M"
             bash plant-loop-driver.txt plant1-capped  capped_follower   1 green
plant 2    : bash plant-loop-driver.txt plant2-capped  capped_follower   1 red   "capped-follower fallback: $M"
             bash plant-loop-driver.txt plant2-expired expired_snapshot  4 green
```

| Variant | Mode | Normalized iteration line | Count | Verdict |
|---|---|---|---|---|
| fixed, both tests | plain | `rc=0 RUN=5 SKIP=0` | 20 | ok 20 / bad 0 |
| fixed, both tests | busy16 | `rc=0 RUN=5 SKIP=0` | 20 | ok 20 / bad 0 |
| plant 1, expired test | plain | `rc=101 RUN=1 SKIP=0 at child_follower_pidfd.rs:260:5 \| left: (Unknown, None) \| right: (Signaled(9), Pidfd)` | 20 | ok 20 / bad 0 (red) |
| plant 1, expired test | busy16 | same line | 20 | ok 20 / bad 0 (red) |
| plant 1, capped test | plain / busy16 | `rc=0 RUN=1 SKIP=0` | 20 / 20 | green, ok 20 / 20 |
| plant 2, capped test | plain | `rc=101 RUN=1 SKIP=0 at child_follower_pidfd.rs:260:5 \| left: (Unknown, None) \| right: (Signaled(9), Pidfd)` | 20 | ok 20 / bad 0 (red) |
| plant 2, capped test | busy16 | same line | 20 | ok 20 / bad 0 (red) |
| plant 2, expired test | plain / busy16 | `rc=0 RUN=4 SKIP=0` | 20 / 20 | green, ok 20 / 20 |

`child_follower_pidfd.rs:260:5` is the `assert_eq!` in `assert_sigkill_pidfd_facts` (payload/source pair).
