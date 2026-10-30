# Coverage violation dispositions — 20-3b landing 2

All 33 inherited violations are corrected at origin. No validation exemption or violation baseline is introduced. Registration below names existing controls, not invented commands or new ship-gate bindings. All test-level controls execute in the blocking workspace-test-suite job.

| # | Requirement | Original violation | Disposition / actual witness |
|---|---|---|---|
| 1 | ADR-050 | unknown gate `check-kernel-baseline` | Existing check-kernel-baseline xtask command / discipline job; register canonical name. |
| 2 | ADR-051 | unknown gate `check-kernel-baseline` | Existing check-kernel-baseline xtask command / discipline job; register canonical name. |
| 3 | ADR-055 | unknown gate `check-kernel-baseline` | Existing check-kernel-baseline xtask command / discipline job; register canonical name. |
| 4 | ADR-024 | unknown gate `check-kernel-baseline` | Existing check-kernel-baseline xtask command / discipline job; register canonical name. |
| 5 | I2 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 6 | I4 | unknown gate `approval_log_is_distinct_table` | Register existing maos-iac/src/adapter/transparency_log.rs unit test; workspace-test-suite. |
| 7 | I4 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 8 | I1 | unknown gate `cap_registry_integration` | Register existing crates/maos-kernel-core/tests/cap_registry_integration.rs; workspace-test-suite. |
| 9 | I10 | unknown gate `journal_survives_cold_restart` | Register existing kernel journal::tests::journal_survives_cold_restart; workspace-test-suite. |
| 10 | I10 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 11 | FR39 | unknown gate `journey-hermetic-tier-1` | Register existing scheduled discipline job journey-hermetic-tier-1. |
| 12 | FR4 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 13 | FR4 | unknown gate `iac_transparency_log_tests` | Register the existing maos-iac transparency_log unit-test module control; workspace-test-suite (no new CLI alias). |
| 14 | FR4 | unknown gate `fr4_1000_call_fixture` | Register existing crates/maos-kernel-core/tests/fr4_1000_call_fixture.rs; dedicated FR4 job and workspace-test-suite. |
| 15 | FR4 | unknown gate `cap_registry_integration` | Register existing crates/maos-kernel-core/tests/cap_registry_integration.rs; workspace-test-suite. |
| 16 | FR40 | unknown gate `smoke-skill-7-4` | Register existing scheduled discipline job smoke-skill-7-4 (four-step real skill smoke). |
| 17 | FR57 | unknown gate `journey-hermetic-tier-1` | Register existing scheduled discipline job journey-hermetic-tier-1. |
| 18 | FR9 | unknown gate `journal_survives_cold_restart` | Register existing kernel journal::tests::journal_survives_cold_restart; workspace-test-suite. |
| 19 | FR9 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 20 | NFR-Maint-8 | unknown gate `cap_registry_integration` | Register existing crates/maos-kernel-core/tests/cap_registry_integration.rs; workspace-test-suite. |
| 21 | NFR-Obs-4 | unknown gate `audit_spine_integration` | Register existing crates/maos-kernel-core/tests/audit_spine_integration.rs; workspace-test-suite. |
| 22 | NFR-Obs-5 | unknown gate `approval_log_is_distinct_table` | Register existing maos-iac/src/adapter/transparency_log.rs unit test; workspace-test-suite. |
| 23 | NFR-Perf-3 | unknown gate `cap_token_verify_assertion` | Register existing crates/maos-kernel-core/tests/cap_token_verify_assertion.rs; workspace-test-suite. No perf semantics changed. |
| 24 | NFR-Rel-3 | unknown gate `nfr-rel-3-hsis-95pct` | Register existing scheduled discipline job nfr-rel-3-hsis-95pct. |
| 25 | NFR-Rel-8 | unknown gate `journal_append_p99_measurement` | Register existing journal_fsync_assertion.rs::journal_append_p99_measurement; workspace-test-suite. No perf semantics changed; 20-7 remains owner. |
| 26 | NFR-Test-3 | unknown gate `coverage-matrix-nfr-test-3` | Register existing scheduled job coverage-matrix-nfr-test-3, running coverage-matrix --measure-nfr-test-3 --dry-run (not a separate CLI). |
| 27 | NFR-Test-5 | unknown gate `check-kernel-baseline` | Existing check-kernel-baseline xtask command / discipline job; register canonical name. |
| 28 | NFR-Aud-8 | unknown corpus `quarterly-audit-v0` | Remove manifest-corpus claim: directory corpus absent from MANIFEST.toml; retain shape/SHA test and generation path in notes. |
| 29 | NFR-Rel-3 | unknown corpus `hsis-corpus-v0` | Remove manifest-corpus claim: MANIFEST.toml:96-101 explicitly treats it as directory-based, SHA-pinned by hsis_runner. Existing nfr-rel-3-hsis-95pct gate remains. |
| 30 | NFR-Test-13 | unknown corpus `crates/maos-kernel-core/tests/fixtures/manifest` | Remove pseudo-corpus claim; fixture directory remains covered by manifest-field-coverage and is named in notes. |
| 31 | NFR-Rel-7 | phase `v2.2` absent from phase_order | Relabel v2.0+; preserve PRD v2.2 in notes, ruled row 03 B. |
| 32 | NFR-Scale-2 | phase `v2.2` absent from phase_order | Relabel v2.0+; preserve PRD v2.2 in notes, ruled row 03 B. |
| 33 | NFR-Test-8 | phase `v2.0` absent from phase_order | Relabel v2.0+; preserve PRD v2.0 in notes, ruled row 03 B. |

Hard mode lands independently of the spike NO-GO. NFR-Rel-1/2/11 keep gates/corpora empty at v0.8 (honest deferral above delivered v0.1-alpha); no unsupported coverage claim is added. `coverage_matrix_20_3b` tests drive the real binary to validate the committed matrix. The designated verifier must execute them; this edit-only unit did not run the gate.
