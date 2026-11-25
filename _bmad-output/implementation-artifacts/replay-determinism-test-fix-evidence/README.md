# replay_determinism_two_process_byte_identical: fix evidence

Hosted CI failure (PR #59, 2026-11-24 21:17 UTC): both children's "output" was the libtest banner
(`running 1 test ... finished in 0.0Xs`), differing only in elapsed time.

## Root cause (two defects)
1. Vacuous: `replay_worker` returns immediately unless `MAOS_REPLAY_DB` is set. The test passed the DB as
   an argument and never set it, so no child replayed anything.
2. Flaky: the test compared libtest's own stdout, which includes `finished in N s`.

## Fix (`crates/maos-audit/tests/replay_schema_test.rs`)
- Parent sets `MAOS_REPLAY_DB`, `MAOS_REPLAY_INPUT`, `MAOS_REPLAY_OUT`; child run with
  `--exact replay_worker --nocapture --test-threads=1`, stdout discarded.
- Worker writes the canonical replay bytes to the file named by `MAOS_REPLAY_OUT`; the parent compares files.
- Also asserted: bytes non-empty; equal to an in-process `replay_to_canonical_bytes` of the same bundle;
  a worker run without `MAOS_REPLAY_DB` succeeds and writes nothing (harness no-op preserved).
- Each replay opens its own pristine copy of the DB: `query_with_redaction` refuses a log with a `-wal` file
  (ADR-028 D6) and a first replay leaves one behind, so a shared DB made the second worker fail.
  (Surfaced while fixing; the old concurrent spawn would also have raced on this.)

## Plants (marker `REPLAY-PLANT`, removed; `grep -c REPLAY-PLANT` = 0 afterwards)
| Plant | Change | Result |
|---|---|---|
| (a) nondeterministic worker | worker appends `std::process::id()` to its bytes | test FAILED at the byte-identical assertion ("two OS-process replays ... must produce byte-identical output") |
| (b) old vacuous shape | parent never sets `MAOS_REPLAY_DB` | test FAILED ("worker 1 must write canonical bytes: NotFound"), no longer passes |

## Runs (fixed test file, same binary)
- 20 idle runs: 20 pass / 0 fail. 10 runs under 16 busy loops: 10 pass / 0 fail.
- `cargo test --locked -p maos-audit`: exit 0.

## Similar-pattern scan (not fixed)
No other vacuous two-process test found. Reviewed `current_exe()` re-spawn tests:
- `maos-bench/tests/t_10_4c_j4_gate_integrity.rs`, `t_10_4c_j4_latency_gate.rs`: parse a marker line
  (`__J4_RESULT_JSON__`) and panic if absent, so not vacuous and not timing-compared.
- `maos-a2a-tcp/tests/h_guards.rs` (`unreadable_fd_limits_*`): child gated by `MAOS_FD_REFUSAL_CHILD`, which the
  parent sets; parent asserts the specific exit status and SKIP text.
- `maos-bin/tests/cli_binary_layout_resolution.rs` and other `current_exe()` users were not found to compare libtest stdout.
