# Plant runs (round 3) — every loop redone on the direct-spawn fixture

Host: Linux 7.3.3-1-cachyos, 32 cores. Driver: `plant-loop-driver.txt` in this directory (run as
`bash plant-loop-driver.txt LABEL FILTER EXPECT_RUN EXPECT`; read its header). It runs with `CI=true` declared, so a
capability skip would PANIC; every iteration is also failed if `SKIP` appears or the number of
`RUN (kernel ...)` lines is not the expected count. A green iteration needs exit 0; a red iteration needs a non-zero
exit whose own log has `left: (Unknown, None)` and `right: (Signaled(9), Pidfd)`. One log per iteration is kept under
`/tmp/s3fix-driver/` (not committed). 20 iterations plain, then 20 under 16 `while :; do :; done` loops.

Commands (the plants were applied one at a time with marker `S3FIX-PLANT` and removed afterwards;
`grep -rn S3FIX-PLANT crates xtask .github docs` = 0 matches):

```
fixed code : bash plant-loop-driver.txt fixed          follower_pidfd    5 green
plant 1    : bash plant-loop-driver.txt plant1-expired expired_snapshot  1 red
             bash plant-loop-driver.txt plant1-capped  capped_follower   1 green
plant 2    : bash plant-loop-driver.txt plant2-capped  capped_follower   1 red
             bash plant-loop-driver.txt plant2-expired expired_snapshot  4 green
```

Plant 1 = `facts_by` expiry branch -> `new(Err(DeadlineExpired), None)`. Plant 2 = `facts_capped` fallback ->
`new(Err(DeadlineExpired), None)`.

Results (iterations grouped by identical normalized line; "20 x" = iterations 1-20 of that mode):

| Variant | Mode | Normalized iteration line | Count | Verdict |
|---|---|---|---|---|
| fixed, both tests | plain | `rc=0 RUN=5 SKIP=0` | 20 | ok 20 / bad 0 |
| fixed, both tests | busy16 | `rc=0 RUN=5 SKIP=0` | 20 | ok 20 / bad 0 |
| plant 1, expired test | plain | `rc=101 RUN=1 SKIP=0 at child_follower_pidfd.rs:247:5 \| left: (Unknown, None) \| right: (Signaled(9), Pidfd)` | 20 | ok 20 / bad 0 (red) |
| plant 1, expired test | busy16 | same line | 20 | ok 20 / bad 0 (red) |
| plant 1, capped test | plain / busy16 | `rc=0 RUN=1 SKIP=0` | 20 / 20 | green, ok 20 / 20 |
| plant 2, capped test | plain | `rc=101 RUN=1 SKIP=0 at child_follower_pidfd.rs:247:5 \| left: (Unknown, None) \| right: (Signaled(9), Pidfd)` | 20 | ok 20 / bad 0 (red) |
| plant 2, capped test | busy16 | same line | 20 | ok 20 / bad 0 (red) |
| plant 2, expired test | plain / busy16 | `rc=0 RUN=4 SKIP=0` | 20 / 20 | green, ok 20 / 20 |

`child_follower_pidfd.rs:247:5` is the `assert_eq!` in `assert_sigkill_pidfd_facts` (payload/source pair). Every red
is that payload assertion; none is a timeout, a rendezvous panic, a poisoned lock, a SKIP or another assertion.
