# 20-9 landing-1 independent verification (Verify209L1)

Tree: HEAD cdf3937d, `git status --short` empty at start. Scratch /tmp/verify209/. No source edits, no git mutation, sole cargo process.

## Setup
- venv /tmp/verify209/venv (cryptography on PATH).
- `cargo run --locked -p xtask -- build-test-fixtures` rc=0 (22 s, 18 artefacts); `cargo build --locked -p maos-bin -p maos-cli` rc=0 (19 s). No freshness-guard complaint.

## Workspace nextest
`env -u CI -u GITHUB_ACTIONS cargo nextest run --workspace --locked --no-fail-fast --profile ci`: rc=100, 244 s wall, nextest 102.9 s; 5913 run, 5912 passed (1 slow), 1 failed, 137 skipped.
- FAIL maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption, 51.494 s, "pending operation did not finish: op-a899af1ddeb366684051bd23" -> KNOWN 20-7 flake (deferred-work.md row 3, "One-million-row pending", erasure_million_20_11.rs:186; same test failed in 20-8 and 20-13 verifications).
- No NEW failures.

## Feature-gated 20-9 coverage (NOT in the default workspace run)
`spawned_lifecycle_20_9`, `spawned_swap_20_9`, `spawned_door_20_9`, `spawned_scope_17_3c` and others are `#![cfg(feature = "network","wasm-host")]`; the plain workspace run compiles/lists zero of them (my first `MAOS_17_2_CGROUP_MODE=undelegated` filter ran 0 tests for that reason). So I ran:
`env -u CI -u GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE=undelegated cargo nextest run --locked -p maos-bin -p maos-wasm-host --features maos-bin/network,maos-bin/wasm-host --no-fail-fast --profile ci -E '<binaries: spawned_scope_17_3c, spawned_lifecycle_20_9, spawned_swap_20_9, spawned_door_20_9, hook_pump_17_4, live_escape_anomaly_17_10, recall_17_3d, participant_recall_17_8, wasm_form_admission_17_3b, wasm_skill_authoring_20_8, erasure_live_20_11, cohort_daemon_smoke_13_5c, package(maos-wasm-host)>'`
rc=0, 77 s wall, 246 run, 246 passed, 2 skipped, 0 failed (spawned_lifecycle 5, spawned_swap 7, spawned_door 3, spawned_scope_17_3c 24, wasm-host lifecycle_runner_20_9 4 PASS lines).

## Gates (env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask xtask <gate> --json)
rc=0, passed: kloc-check (aggregate 200880), check-kernel-baseline (26841/26841), check-dev-record-completeness (199 stories, 0 violations), check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract (122), check-fkcs (6 s, oracle_green), error-catalog-check (69), check-manifest-schema-version (7), check-equiv-fixture-provenance (4), check-wasm-form-equiv (14 s, oracle_green), check-service-boundary, check-composition-root-completeness, check-mock-not-in-release, `stability-matrix --check` (PASS), `gen-abi-docs --check`, invariant-lock (changed-files file of `git diff --name-only HEAD~1`, 93 files, 0 docs/invariants; touched_invariants []).
- `check-test-budget --junit` (copy of the workspace run's junit) rc=1, 5 findings over 30 s (Σ per-test): erasure_uninstall_13_5b 33.238 s/27; hook_pump_17_4 34.201 s/29; one_daemon_one_door_16_1 37.889 s/43; maos-iac::participant_recall_17_8 45.978 s/29; xtask::bin/xtask 31.047 s/464. All five are exactly the deferred-work.md row 4 "Workspace binary budget observations" (20-7, erasure-uninstall/hook-pump/one-door/participant-recall/xtask) -> KNOWN, not 20-9-specific. Note hook_pump_17_4 is a file the 20-9 diff touched; row-4 value was 32.368 s, now 34.2 s (load-sensitive Σ; HEAD~1 not re-measured, unverified).
- `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`: 7 passed, rc=0.
- `cargo check --workspace --all-targets --locked`: rc=0 (145 s).
- `cargo clippy --workspace --all-targets --locked --message-format=json`: rc=0 (113 s); clippy-ratchet (`--input` json, default baseline) rc=0: occurrences 1327, source_distinct 867 (matches the implementer's clippy-ratchet.log; prior baseline 1323/865 at 20-13).

## Unverified remainder
Hosted posture (CI=true, strace, T3/delegated cgroup under systemd-run, the real `run_17_3c_delegated.sh`), kernel-core `--lib` libtest run, doctests, prepush wrapper, TS template/example `npm` tests (`examples/example-spirit-ts`, `templates/spirit-ts`), erasure_million failure in isolation, whether the 5 budget findings exist at HEAD~1, the other feature-gated binaries not named above (wasm-host-feature suites outside the filter), `check-wasm-form-equiv` per-form equivalence beyond oracle_green.
