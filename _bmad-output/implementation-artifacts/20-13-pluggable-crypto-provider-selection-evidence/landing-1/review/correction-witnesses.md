# L1 review correction witnesses

All observations local; hosted jobs remain unverified. `artifact://16034` records
the grouped negative controls and explicit per-command exit codes, followed by
copy-back, `cmp` and `touch` for each planted source (restore exit 0).

| Finding | Witness and negative control | Observed |
|---|---|---|
| P1 | `crypto_endpoint_drift_20_13::empty_url_does_not_unpin_provider_or_model`; restore whole-endpoint unpin | before exit 101 (`artifact://16005`), plant 101 (`artifact://16034`), corrected exit 0 (`artifact://16017`) |
| P2 | `unknown_provider_refused_without_changing_config`; restore old first-bare-flag parser | before 101 (`artifact://16006`), plant 101 (`artifact://16034`), corrected 0 (`artifact://16017`) |
| P3 | `crypto_construction_in_production_is_refused_after_test_module`; remove type-path visitor | before 101 (`artifact://16012`), plant 101 (`artifact://16034`), corrected 0 (`artifact://16014`); final gate to be recorded below |
| P4 | `prior_boot_provider_selection_is_not_spirit_history`; reinstate this-boot-only selection exception | before 101 (`artifact://16006`), plant 101 (`artifact://16034`), corrected 0 (`artifact://16023`). HEAD code derives 4 then 5: first NotFound appends a Decision counted by the next boot; no old-tree execution claimed. |
| P5 shell | `crypto_real_shell_20_13::real_shell_shares_provider_across_package_crl_and_token`; plant second CRL provider in main | corrected 0 (`artifact://16027`); plant 101 with identities `:1` and `:2` (`artifact://16029`), copy-back + cmp + touch exit 0 |
| P5 cohort | `crypto_real_boot_20_13::real_cohort_daemon_shares_provider_across_crl_token_and_seal`; plant separate enterprise seal provider in main | corrected 0 (`artifact://16023`); plant 101 with identity `:2` on seal (`artifact://16031`), copy-back + cmp + touch exit 0 |
| P6 | `init_repairs_and_reselects_inline_or_missing_provider`; restore old table-only writer | plant 101 (`artifact://16034`); corrected 0 (`artifact://16023`). Earlier incomplete-config attempt measured regeneration rather than this defect (`artifact://16020`). |
| P7 | `absent_home_never_reads_working_directory_config`; restore CWD-relative default | before 101 (`artifact://16008`), plant 101 (`artifact://16034`), corrected 0 (`artifact://16017`) |
| P8/F2/F3 | English and Korean guides and bypass inventory; no executable behavior change | explicit-run ring-direct boundary and Kernel/pid 0 audit tuple documented |
| P9 | `malformed_provider_reports_the_exact_type_error`; change message to another string containing `string` | plant 101 (`artifact://16034`) |
| P10 | `installed_claims_compare_actual_crypto_and_endpoint`; plant alternate typed admission code | plant 101 (`artifact://16034`); the shell process itself exits 0 because refusal is per Spirit, so the witness pins `admission_refused` and actual process status rather than inventing a process error. |
| P11 | `selected_provider_is_journaled_once_per_boot`; plant incorrect journal version | plant 101 (`artifact://16034`) |
| P12 | `descendant_terminal_state_accepts_reaped_or_dead_but_not_live`; remove lowercase x | before 101 (`artifact://16016`), plant 101 (`artifact://16034`), corrected 0 (`artifact://16017`) |
| P13 | Remove gate-registry row, keep independent blocking job; bound CRL readiness to 25 s; add scan-roster history and release-profile caveat | `coverage-matrix` and `check-ship-gate-completeness` exit 0 (`artifact://16019`); no hosted claim |
| D1 | `selection_journal_failure_stops_real_boot` injects SQLite ABORT on the exact selection intent | real process stopped with I2 write failure and no door, exit 0 for witness (`artifact://16020` journal=0); initial wrong DB-path test failed (`artifact://16017`), corrected to MAOS_HOME audit path |

P5 adjudication: the literal four-surface single boot is the in-process harness;
real binaries exercise per-mode subsets. The cohort one-shot excludes installed
inventory, so no single production mode reaches all four. That unresolved
security question is ownerless and dispositioned in `deferred-work.md` for the
first story touching cohort boot. No cohort boot behavior was changed.

P4 row inspection (`artifact://16020:574`) observed the first NotFound's
`(kind=10, pid=0, intent=principal.uninstall.not-found, origin=Kernel)` Decision
between the two boots; the second call then appended the failed terminal row.
The original HEAD's `has_frames()` counts the former without an exemption.

Final post-correction run (`artifact://16044`, exit 0): the recording fixture
builder and `cargo build --locked -p maos-bin -p maos-cli`, then
`cargo nextest run --locked -p maos-bin --test crypto_endpoint_drift_20_13
--test crypto_provider_20_13 --test crypto_real_shell_20_13 --test
crypto_real_boot_20_13 --test erasure_uninstall_13_5b --profile ci
--test-threads 1` (42/42); the xtask structural negative-control unit (1/1),
the cgroup terminal-state unit (1/1), and
`cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`
(7/7). `cargo fmt --all` then exact measurement gave xtask 44,935 and aggregate
198,914; the intermediate kloc check exited 1 only because it exposed the
one-line unpinned guard increment, after which the pins were updated.
`cargo build --locked -p xtask` exited 0 (`artifact://16049`).

Final rebuilt binary gate suite (`artifact://16050`): `kloc-check`,
`check-env-contract`, `check-service-boundary`,
`check-composition-root-completeness`, `check-ship-gate-completeness`,
`coverage-matrix`, `check-test-cost`, `check-exit-commands`,
`check-decision-register`, `check-dev-record-completeness`,
`check-epic-close-coherence`, `check-kernel-baseline`, and `invariant-lock`
each exited 0. Each used `env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask <gate> --json`;
the invariant invocation additionally supplied
`--changed-files <(git diff --name-only HEAD~1)`. The full workspace and
prepush limitations remain as separately recorded, not retried to green.

Cleanup: removed the review plant scratch and 863 non-newest incremental
directories older than three hours after nonblocking sibling-lock probes.
Estimated allocated space freed: 9,080,201,216 bytes (9,079,222,272
incremental and 978,944 scratch). `/proc/1/fd` and `/proc/7/fd` were
permission-denied, so the age and lock probes were the available safeguards.
