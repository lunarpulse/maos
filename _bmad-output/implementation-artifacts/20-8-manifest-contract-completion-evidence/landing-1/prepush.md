# Once-only prepush

Command: `PATH=/tmp/20-8-verifier-venv/bin:$PATH env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json`

The 3,000-second cell deadline interrupted the command (wall 3,000.32 s). The gate buffers its JSON and emitted only `error: interrupted`, so no JSON gate dispositions, tracked-tree verdict, or overall exit code are available. It was run once and is not retried. This is not a pass. The Python virtual environment had the repository's hash-pinned verifier requirements, installed before this run. Standalone workspace results and repairs are separately recorded in `preflight.md` and `workspace-nextest.log`.

The fresh `target/nextest/default/junit.xml` from this run was timestamped
2026-11-09T03:15:17Z and completed before the interruption; it is preserved as
`prepush-junit.xml` (UUID `37c125b0-8426-4615-94a3-0864bc597e3c`).
It records 5,849 tests, one failure: the existing 20-7-owned 1M pending
operation completed the test in 47.138 s without observing its pending
operation finish. It does not establish completion of the remaining prepush
legs or an overall pass. No test or prepush retry was performed.

The default-feature WIT AST witness appears and passed in that JUnit, so its
owned binary exception is no longer absent. A separate post-timeout
`check-test-budget --junit .../prepush-junit.xml --json` exited 1 for six
per-binary timing findings; see `prepush-budget.txt` and the same-line 20-7
disposition. Aggregate test sum was 1,199.626 s, below 1,440 s. This budget
inspection is not a rerun of any test or the prepush.
