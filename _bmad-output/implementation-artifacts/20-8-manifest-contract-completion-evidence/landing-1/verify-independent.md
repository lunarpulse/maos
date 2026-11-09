# 20-8 landing-1 independent verification (Verify208)

Tree: HEAD 64bf998d + dirty working tree. Raw artefacts in /tmp/verify208/. No source edits.

## Setup
- venv /tmp/verify208/venv with `cryptography` 50.0.2 on PATH.
- `cargo run --locked -p xtask -- build-test-fixtures` rc=0 (135 s); `cargo build --locked -p maos-bin -p maos-cli` rc=0 (61 s). Freshness guard never reported stale maosctl; no touch needed.

## Workspace (`cargo nextest run --workspace --locked --no-fail-fast --profile ci`)
- Run 1, inherited shell env `CI=true`: rc=100, 1959 s wall (231 s nextest), 5854 run, 84 failed + 1 timeout. 75 T3 "MAOS_T3_RUNTIME must select the provisioned runtime" (declared-CI turns skips into failures), 3 otel airgap (strace not installed), plus 7 others. Environment-contaminated.
- Run 2, `env -u CI -u GITHUB_ACTIONS` (local posture): rc=100, 201 s wall, 5849 passed, 4 failed + 1 timeout:
  - maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption TIMEOUT 97.0 s (junit; erasure_million_20_11.rs:186) -> KNOWN 20-7 1M oracle (deferred-work.md lines 3/24/29/44/50).
  - maos-bin::hook_pump_17_4 one_budget_overrun_quarantines_the_subscriber 3.8 s (hook_pump_17_4.rs:441 assert_eq) -> NEW, load-timing hypothesis
  - maos-bin::hook_pump_17_4 a_late_hook_result_appends_its_eventual_redistilled_receipt 6.5 s (:1995 Elapsed) -> NEW, load-timing hypothesis
  - maos-bin::installed_spirit_dispatch_20_1 installed_component_dispatch_reports_typed_boot_refusal 2.4 s (:76 boot took 2.06 s) -> NEW, wall-clock bound under load
  - maos-bin::observer_activity_corpus_17_4 observer_activity_corpus_production_path TIMEOUT 120.05 s -> NEW, load/timing hypothesis
  Not retried (per instruction). The 4 NEW were not isolated.

## Gates (env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask xtask <gate> --json): all rc=0
kloc-check, check-kernel-baseline, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-fkcs (376 s), error-catalog-check, check-manifest-schema-version, check-equiv-fixture-provenance, check-corpus, check-service-boundary, stability-matrix --check, gen-abi-docs --check (25 s), invariant-lock (changed list = git diff --name-only HEAD + untracked, 68 files, 0 docs/invariants/*.md -> gate not triggered; passed:true).
- `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`: 7 passed, rc=0.
- clippy --workspace --all-targets --locked JSON rc=0 (1492 s); clippy-ratchet rc=0: occurrences 1321, source_distinct 864 (baseline hosted 1383/897).

## Unverified
Hosted-only items (strace/T3 substrate/CI=true posture: 78 failures appear when CI=true without MAOS_T3_RUNTIME and strace), the kernel-core --lib libtest run, doctests, prepush wrapper, the 4 NEW timing failures in isolation.
