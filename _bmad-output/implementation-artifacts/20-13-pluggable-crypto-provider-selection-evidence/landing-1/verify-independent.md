# 20-13 landing-1 independent verification (Verify2013)

Tree: HEAD a2e37988, working tree clean at start. Raw artefacts /tmp/verify2013/. No source edits.

## Setup
- venv /tmp/verify2013/venv, cryptography 50.0.2 on PATH.
- `cargo run --locked -p xtask -- build-test-fixtures` rc=0 (354 s); `cargo build --locked -p maos-bin -p maos-cli` rc=0 (61 s). No stale-maosctl guard report.

## Workspace
`env -u CI -u GITHUB_ACTIONS cargo nextest run --workspace --locked --no-fail-fast --profile ci`: rc=100, 945 s wall (incl. compile), nextest 92.9 s; 5869 run, 5868 passed, 1 failed, 137 skipped.
- FAIL maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption, 42.878 s, "pending operation did not finish: op-67b78d23e68263148839c207" (erasure_million_20_11.rs:186) -> KNOWN 20-7 1M-row oracle flake (deferred-work.md; same test as the 20-8 verification).
- The two hosted-fix files (ci_shape_18_0, cgroup_ceiling_smoke) passed in this run.

## Gates (env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask xtask <gate> --json)
rc=0: kloc-check, check-kernel-baseline, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-fkcs (23 s, passed:true oracle_green:true), error-catalog-check, check-service-boundary, check-composition-root-completeness, check-mock-not-in-release.
- invariant-lock: `--changed-files` takes a FILE path (one name per line), not a CSV; my first CSV attempt rc=1 (usage). Rerun with file of `git diff --name-only HEAD~1` (53 files, 0 docs/invariants): passed:true, touched_invariants [], rc=0.
- check-test-budget --junit (copy of target/nextest/ci/junit.xml) rc=1 FAIL, 4 findings (Σ per-test time over 30 s budget, 629 binaries):
  maos-bin::erasure_uninstall_13_5b 33.989 s/27 tests; maos-bin::hook_pump_17_4 35.283 s/29; maos-bin::one_daemon_one_door_16_1 37.495 s/43; maos-iac::participant_recall_17_8 41.045 s/29. Σ-of-parallel-test times under full default concurrency; not classified against baseline (I did not compare with the parent commit; unverified whether NEW vs pre-existing/load). Hypothesis: concurrency load, no 20-13 file named (participant_recall_17_8 is in maos-iac, the 20-13 diff area is maos-bin crypto_provider).
- `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`: 7 passed, rc=0 (55 s).
- `cargo check -p maos-bin --features crypto-recording --release`: rc=101 as REQUIRED, message: `error: crypto-recording is dev/CI-only and MUST NOT ship in release builds --> crates/maos-bin/src/crypto_provider.rs:110:5 compile_error!(...)`.
- clippy `--workspace --all-targets --locked --message-format=json` rc=0 (945 s); clippy-ratchet rc=0: occurrences 1323, source_distinct 865 (baseline hosted 1383/897).

## Unverified
Hosted-only posture (CI=true, strace, T3 substrate), kernel-core --lib libtest run, doctests, prepush wrapper, ERASURE_million failure in isolation, whether the 4 test-budget findings exist at HEAD~1.
