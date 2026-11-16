# LoomFix dispatch brief (interrupted by Anthropic 429 on 2026-11-16; NO edits were made)

Paste this to a `coder` agent (sonnet-5-5:high) together with the shared context block from
`local://dev283-brief.md` (EDIT ONLY, absolute paths, edit tool, no stubs, final message format).

## Target

Worktree: /home/lunarpulse/dev_ws-loomfix (branch loomfix-2026-11-16, based on main fad08ae0).

## Background

Hosted gate `check-multi-tenant-loom` (xtask/src/check_multi_tenant_loom.rs, run via
`python3 scripts/run_cgroup_ceiling_17_2.py run --mode delegated --unit maos-17-2-multi-tenant-loom
--timeout 1500 -- cargo run -p xtask -- check-multi-tenant-loom --json` in
.github/workflows/discipline.yml ~line 4604) once timed out at 1500 s with NO log output, so the leg
that stalled is unknown; the gate prints nothing until it finishes. Separately, `cargo test -p maos-bin`
(libtest, one process) fails two unit tests locally:
`migration_plan::tests::persisted_plan_file_drift_is_refused_after_a_candidate_mutates_on_disk` and
`migration_plan::tests::plan_is_atomically_persisted_under_maos_home_and_round_trips` (panic
crates/maos-bin/src/migration_plan.rs:408 'plan file exists') — they share MAOS_HOME in one process.
Nextest (process per test) is green. Both rows are at the end of
/home/lunarpulse/dev_ws-loomfix/_bmad-output/implementation-artifacts/deferred-work.md.

## Change

1. Streaming: make the gate emit, to STDERR (live in the job log; stdout `--json` contract untouched),
   one line when each test leg starts and one when it ends with elapsed seconds and verdict, e.g.
   `LEG start <name>` / `LEG end <name> <secs>s <verdict>`, added in `run_exact_test_leg`. Confirm by
   reading `scripts/run_cgroup_ceiling_17_2.py` that the child's stderr is forwarded unbuffered; if it
   buffers or captures stderr until exit, change it to stream (minimal change). Keep xtask growth
   minimal (xtask is at zero headroom: kloc ceiling 46744; report estimated added production lines).
2. Race: fix the two `migration_plan` unit tests so they cannot race on MAOS_HOME inside one process —
   serialize them (or all tests in that module that touch MAOS_HOME) behind the existing MAOS_HOME lock
   used by the integration test named in the comment at xtask/src/check_multi_tenant_loom.rs ~1840-1860
   (`crates/maos-bin/tests/cross_team_consent_13_3.rs` and its `:497`), or give each its own root if the
   code allows. Do not weaken, delete, or skip any assertion. First READ migration_plan.rs tests and the
   maos-bin unit-test helpers to find which other unit tests mutate MAOS_HOME and use the same mechanism.
3. Update the two deferred-work rows' Status lines to say the fix is implemented in this branch pending
   hosted proof (rows otherwise unchanged).

## Acceptance

By reading: the stream lines can be produced without changing stdout JSON; the race is removed with all
assertions intact. Final message: files changed, estimated xtask/maos-bin production-line delta, and
the exact cargo commands the orchestrator should run (e.g. `cargo test -p maos-bin --bin maos --
migration_plan`, repeated N times).
