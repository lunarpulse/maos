# L1 candidate verification

All commands below ran with `CI` and `GITHUB_ACTIONS` unset. Full plant details,
including copy-back + `cmp` + `touch`, are in `preflight.md`.

- `cargo fmt --all`: exit 0 before the recorded formatted code measurement.
- `cargo run --locked -p xtask -- build-test-fixtures && cargo build --locked -p maos-bin -p maos-cli && cargo nextest run --locked -p maos-bin --test crypto_provider_20_13 --test crypto_provider_t3_20_13`: exit 0; 9 tests passed in 7.095 s after fixture build. The T3 test returned early under the local absent-substrate policy, so this is not a Worker T3 execution claim. The final focused test changes include the extra runtime endpoint case, missing-config journal source, and bare-run exit-78 check. Output `artifact://15663`.
- `cargo test --locked -p maos-bin --bin maos --features crypto-recording story_13_5a_enterprise_governance_reaches_the_booted_cohort_daemon -- --nocapture`: exit 0, one test passed; same selected recording instance saw a capability signature and real enterprise at-rest sealing in the booted governance harness. Output `artifact://15633`.
- `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`: exit 0, seven tests passed (`artifact://15633`).
- `cargo test --locked -p maos-compliance --test ccac_ship_gate_test`: exit 0, one test passed (`artifact://15633`).
- Final `CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json`: exit 0 after formatting and exact re-pin, aggregate 198835, no over-budget crates (`artifact://15798`).
- `cargo check --locked --release -p maos-bin --lib --features crypto-recording`: exited 101 as required at `crypto_provider.rs:110`, with the explicit `crypto-recording is dev/CI-only and MUST NOT ship in release builds` compile error; output `artifact://15668`.
- `cargo test --locked -p xtask crypto_construction_in_production_is_refused_after_test_module -- --nocapture && cargo build --locked -p xtask`: exit 0, one focused unit test passed, including direct path, default, alias and struct-expression controls; output `artifact://15706`. The rebuilt `check-composition-root-completeness --json` exited 0, 0 violations.
- D-20-13-17 failing-before: `cargo test --locked -p maos-compliance --test evaluator_test ring_direct_empty_claim_still_drifts_against_nonempty_runtime -- --exact --nocapture` exited 101 on the original shared-unpinned implementation (`artifact://15754`), after the earlier inverse witness showed admission (`artifact://15752`). The orchestrator ruled provider-only unpinned semantics; caller tracing is `evaluator-callers.md`.
- D-20-13-17 plant: restoring shared unpinned behavior for `evaluate_envelope_at` caused the ring-direct drift witness to fail again (exit 101, `artifact://15769`). The corrected evaluator was copied back, compared with `cmp`, and touched (all exit 0).
- Publisher verifier negative control: the test-local rejecting provider was planted to return `Ok(())`; `cargo test --locked -p maos-registry --test publisher_provider_20_13 -- --nocapture` exited 101 (`artifact://15757`). The original test file was copied back, compared with `cmp`, and touched (all exit 0); neither FKCS declared source file was modified for the plant.
- After D-20-13-17 and the uninstall fix, `cargo test --locked -p maos-compliance --test evaluator_test` passed 20; `cargo test --locked -p maos-registry --test publisher_provider_20_13` passed 1; `cargo run --locked -p xtask -- build-test-fixtures --only maos-crypto-recording && cargo build --locked -p maos-bin -p maos-cli && cargo nextest run --locked -p maos-bin --test crypto_provider_20_13 --test erasure_uninstall_13_5b` passed all 35 tests (entire chain exit 0, `artifact://15766`). This includes the two previously red empty-log vectors and the new prior-boot history control.
- `cargo check --locked --workspace --all-targets --target aarch64-apple-darwin` exited 101: local `cc` cannot handle Apple `-arch` / `-mmacosx-version-min` flags when building ring (`artifact://15747`). Apple SDK/toolchain is absent; the hosted recipe is unverified.
- `CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask check-fkcs --json` timed out after 120 s. It was inadvertently invoked while the correction cargo chain was active; inspection afterward showed that it spawns cargo itself. No gate result is claimed from this invocation.
- Workspace metadata preparation: `cargo metadata --locked --format-version 1` completed; subsequent `cargo nextest list --workspace --locked --list-type binaries-only --message-format json` timed out after 3000 s while compiling workspace test binaries, before any workspace test ran (`artifact://15710`). A resumed compilation was cancelled when Main assigned the hosted test fixes; the final compilation was restarted after the fixes and plant restoration.
  Final `cargo nextest list --workspace --locked --list-type binaries-only
  --message-format json` completed successfully after the fixes (exit 0,
  `artifact://15726`); the workspace execution reuses its binaries metadata.
- One workspace execution with `cargo nextest run --cargo-metadata
  /tmp/maos-2013-cargo-metadata.json --binaries-metadata
  /tmp/maos-2013-binaries.json --profile ci --no-fail-fast` exited 100:
  5859 passed, 7 failed, 137 skipped (`artifact://15730`). Failures:
  `erasure_uninstall_13_5b::{not_found_uninstall_has_distinct_terminal_code,
  empty_transparency_log_is_not_found_even_for_unknown_spirit}` (5 vs 4);
  `installed_spirit_dispatch_20_1::installed_component_dispatch_reports_typed_boot_refusal`
  (2.109 s exceeded 2 s); `hook_pump_17_4::a_late_hook_result_appends_its_eventual_redistilled_receipt`
  (timeout); `coverage_matrix_20_3b::committed_hand_kept_matrix_is_hard_and_all_references_resolve`
  (FR48's existing composition gate not registered); `fkcs_oracle::every_declared_admission_file_reds_a_one_byte_mutation`
  (admission hash changed); `erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption`
  (pending timeout). No test failure was retried. The gate-registration
  defect was fixed; `coverage-matrix --json` then passed in hard mode. The
  other failures require investigation or review; the new boot journal frame
  is a plausible cause of the two empty-log regressions.
- Hosted fixes carried separately: Main reported CI run 42722028970's two stale
  recall-shape tests and the ARM descendant `X` failure. The unchanged strict
  state rule plus a new synthetic `Some("X")` vector exited 101 locally
  (`artifact://15717`). After fixing it, `cargo test --locked -p
  maos-kernel-core --test cgroup_ceiling_smoke
  descendant_terminal_state_accepts_reaped_or_dead_but_not_live -- --exact
  --nocapture` passed one test; `cargo test --locked -p xtask --test
  ci_shape_18_0 -- --nocapture` passed all 20 tests (`artifact://15720`).
  The latter derives the 20 recall and 7 recall-wire tests directly from parsed
  source and independently removes every count/name token as a negative
  control. Kernel baseline and formatted `kloc-check` both exited 0 afterward.
- Restoring the original strict-only `Z` classifier as an explicit plant made
  `descendant_terminal_state_accepts_reaped_or_dead_but_not_live` fail on `X`
  again (exit 101, `artifact://15724`); the fixed file was copied back, compared
  with `cmp`, and touched (all exit 0).

The integrated 13.5a-based one-boot witness
`tests/support/crypto_integrated_20_13.rs:3` performs installed-package boot
verification (two signatures), capability token issuance and verification,
CRL import through a live HTTP operator door while the cohort daemon is
running, and real governed at-rest sealing. Its second log records the same
process/instance identity on every crypto operation. The package is present
before boot; no restart is used. Token verification is the existing constant-
time cached-signature check; the selected provider signs the token. Normal
nextest listing finds the feature-enabled test and its run passed 1/1
(`artifact://15796`). A second provider instance planted only into the door
produced distinct identities and failed (exit 101, `artifact://15792`); the
original witness was restored by copy-back + `cmp` + `touch`. Hosted Worker
T3, ARM and Apple legs remain unverified locally.

Static gates run with `CARGO_MANIFEST_DIR=$PWD/xtask` and
`/mnt/build/cargo-maos/debug/xtask <gate> --json`: `check-env-contract`,
`check-service-boundary`, `check-test-cost`, `check-corpus`,
`check-kernel-baseline`, `check-exit-commands`, `check-decision-register`,
`check-dev-record-completeness`, and `check-epic-close-coherence` all exited 0.
`invariant-lock --changed-files <(git diff --name-only HEAD~1) --json` exited
0 with no touched invariants. Full output: `artifact://15665`.

The one `CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask
prepush --json` invocation, with `CI` and `GITHUB_ACTIONS` unset and the
scratch verifier venv on PATH, timed out at 3000 s; its buffered JSON did not
arrive. No tracked edits were made while it ran.

Final record gates `check-env-contract`, `check-service-boundary`,
`check-composition-root-completeness`, `check-exit-commands`,
`check-decision-register`, `check-dev-record-completeness`,
`check-epic-close-coherence`, and the changed-files `invariant-lock` all
exited 0 (`artifact://15774`). The generated intent-lineage report compared
unchanged. Cleanup removed 1,586 stale incremental session directories
(estimated 17,958,240,256 allocated bytes) and 34,459,648 allocated bytes of
own scratch; 671 recent sessions were retained. `/proc/1/fd` and `/proc/7/fd`
were unreadable, so cleanup used the non-blocking lock probe plus age checks,
never the newest session per crate.

A second safe cleanup after the integrated witness removed 108 more eligible
incremental sessions (1,470,177,280 allocated bytes) and 12,288 bytes of
new scratch. Combined estimates: 19,428,417,536 incremental bytes and
34,471,936 scratch bytes. The same unreadable-proc limitation and lock/age/
newest-session protections apply.

After integrated feature-only instrumentation and test-harness changes,
`kloc-check`, `check-kernel-baseline`, `check-env-contract`,
`check-service-boundary`, `check-composition-root-completeness`,
`check-test-cost`, `coverage-matrix`, `check-exit-commands`,
`check-decision-register`, `check-dev-record-completeness`,
`check-epic-close-coherence`, and changed-files `invariant-lock` each exited 0
(`artifact://15798`). The sole workspace nextest and sole prepush preceded
this test-only change; the three timing-sensitive failures were not retried to
green. The recording feature is absent by default and refuses release builds.

## Acceptance witness index

| Criterion | Witness (candidate line) | Before / plant | Observed final |
|---|---|---|---|
| AC1 | `crypto_provider_20_13.rs:9,36,81` | old init missing table; fallback plant exit 101 | focused 3 passed |
| AC2 | `tests/support/crypto_integrated_20_13.rs:3`; `crypto_real_shell_20_13.rs`; `crypto_real_boot_20_13.rs` | in-process CRL identity split plant and real-shell CRL split plant exit 101 | all four on one in-process boot; real processes prove per-mode subsets, not all four on one real boot; T3 hosted-only |
| AC3 | `check_composition_root_completeness.rs` `crypto_construction_in_production_is_refused_after_test_module` | original scan five sites; root plant gate exit 1 | unit and final structural gate passed; T3 kernel exception explicitly retained for L2 |
| AC4 | `crypto_provider_20_13.rs:156` | manifest crypto and endpoint plants each exit 101 | drift / unpinned integration vector passed |
| AC5 | `crypto_provider_20_13.rs:69,288` | omitted-frame plant exit 101 | version pin and once-per-boot query passed |
| AC6 | operator guides and `tests/coverage-matrix.yaml` | documentation/records criterion; no executable plant | documents updated; kernel doc and epic final closure remain L2 |
