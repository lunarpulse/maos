# 20-11 landing 1 — kernel-free implementation and observed verification

Baseline `cb0abc06e6b085cb63db1be60d4addbeb2963811`; original before controls at `76efc19d`. No kernel/domain source, kernel pin/content/surface hash, tracker status, commit or push changed. AC3, halt counts/receipt category, KS and O4 are final-L2 work, not this deliverable. Original T0 evidence, including `t0-unload-race-witness.rs.txt`, is retained.

## Acceptance witnesses

Full actual names:lines, every genuine red/plant, restore evidence and limitations: [l1-final-witnesses.md](l1-final-witnesses.md). Immutable-baseline/current citations: [l1-final-cites.md](l1-final-cites.md).

| AC | Representative actual witness (under crates/maos-bin/tests unless qualified) | Before / plant | Final local observation |
|---|---|---|---|
| AC1 | `erasure_inventory_20_11.rs::ac1_principal_free_distillate_gets_a_signed_proof:151`; both-verifier negative vector:266 | Original NotFound101; old-condition plant100; freshly re-signed missing-evidence negative | Pass: signed v2 query observations, post inclusions and both verifiers. |
| AC2 | inventory `ac2_all_boots_and_pid_reuse_are_isolated:188`; actual empty373; combined principal386; held401; live302 | Original missing/false-empty101; VerifiedEmpty plant100 | Pass: all owned boot/pid incarnations; tombstone plus Spirit marker; held derivations retained; other Spirit/cost bytes unchanged; live and refused-boot/offline recovery. |
| AC4 | inventory `ac4_installed_package_and_schedule_are_queried_without_principals:205`; failed proof449; shell snapshot1659 | Original NotFound101; empty SCB snapshot plant100; proof-failure package-loss101 | Pass: queried manifest/SCB ids, pre-unload capture, staged package/index rollback until proof persisted. Watchdog retention explicitly awaits KS. |
| AC5 | late hold7; shell serialized release1421,100k1503,1M completion/repeat1508; IAC `erasure_scan_work.rs`21/26/96/132 | Real old late-hold/serializer/scan101; actual kind/recovery/ordering/opaque-index101; final MAX-page plant101 at both sizes | Pass: actual five SQL predicates bounded, unchanged25% wall/work bounds, ordering/error semantics preserved. Pending503 is not erased/complete; actual repaired-run pending completion and identical repeat observed. |
| AC6 | `erasure_live_20_11.rs::ac6_running_t3_worker_uninstall_proves_live_inventory:306` | Ruled AC4 snapshot control; AC6.3 live-lineage fallback because schedule admission is existing20-9 work | Fixture implemented; explicit signed-T3-substrate runtime skip locally. Hosted Worker teardown proof remains unverified. Local real live-cascade sibling passes. |
| AC7 | inventory `ac7_archive_and_authored_skill_are_named_and_counted:224` | Original absent archive101; dropped typed archive entry plant100 | Pass: real archive/authored full-identity inventory, Lifecycle Journal/Worker workspace retention named. |

All plants were copied back, cmp-checked and touched. The last guarded-page log records `PLANT_RESTORE_COPY_CMP_TOUCH_EXIT=0`. The initial insensitive green page plant remains recorded, not relabelled a red. No plant remains applied.

## Exact final measured pins after cargo fmt --all

| Surface | Rebased exact baseline | Measured / pin | Growth | Unchanged granted ceiling |
|---|---:|---:|---:|---:|
| maos-bin |34552|34842|290|35704|
| maos-iac |8270|8551|281|8630|
| maos-audit |7721|7785|64|8261|
| maos-registry |4248|4322|74|4368|
| xtask |44466|44466|0|44466|
| aggregate |193202|193911|709|195404|

Strict hardfail **193912 =193911+1**. All rows/HISTORY move together; xtask's first HISTORY segment cites live20-11 with `44466 ->44466`, retaining the priority hotfix. Other crates have zero LOC growth, including domain10107. Kernel **26841 physical /21042 tokei**,95 files; set/content hash remains `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`. Kernel grant consumption is zero. Raw measurement `l1-fr4-complete-counts.json`; exact post-repin gate `l1-fr4-final-kloc-check.json`, exit0.

## Observed checks and exact commands

Cargo inherited `$CARGO_TARGET_DIR=/mnt/build/cargo-maos`; no assignment/override/clean. CI/GITHUB_ACTIONS were unset. Python verifier dependency was supplied by temporary PYTHONPATH, without replacing the host interpreter. Workspace consumers also unset NEXTEST_TEST_THREADS/NEXTEST_RETRIES and use default concurrency/no retry overrides.

- `cargo fmt --all`: final post-FR4/file-guard execution0 (`l1-fr4-complete-fmt.*`).
- `cargo test --locked --no-fail-fast -p maos-bin --test erasure_inventory_20_11 --test erasure_live_20_11 --test erasure_scan_20_11 --test shell_host_16_2 --test erasure_uninstall_13_5b --test forget_floors_18_6 -- --nocapture`: exit **0**; **53** reported successful, **52** exercised plus one explicit hosted-T3 runtime skip.
- Sequential preservation: `cargo test --locked -p maos-kernel-core --test forget_cascade_18_6 -- --nocapture && cargo test --locked -p maos-audit --test erasure_smoke_test -- --nocapture && cargo test --locked -p maos-registry --lib -- --nocapture`:0;13+3+58 passed,2 registry ignored.
- Final classifier repair `cargo test --locked -p maos-audit --test fr4_classifier_16_2 -- --nocapture`:0,20/20. Its preceding genuine production-writer and file-guard reds remain. The file guard's production-only negative control passes; no basename exemption or phantom test-writer production disposition.
- Required final control `cargo test -p maos-bin --no-default-features --test verb_table_15_3`:0,7/7.
- After control, `cargo test --locked -p maos-iac --lib -- --nocapture && cargo build --locked -p maos-bin -p maos-cli`:0,103/103 then genuine default binaries restored (`l1-final-guard-iac-and-default-reset.*`).
- Registered preparation `cargo run --locked -p xtask -- build-test-fixtures`:0,16 artifacts; then genuine `cargo build --locked -p maos-bin -p maos-cli`:0 before metadata producers. Prepush also ran its registered builder0.
- Final `cargo clippy --workspace --all-targets --locked --message-format=json`:0 (`l1-fr4-final-clippy.jsonl`). `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input <evidence>/l1-fr4-final-clippy.jsonl`:0;1329 occurrences/870 distinct vs unchanged1383/897 baseline, **-54/-27**. No warning suppression or baseline widening.
- Locked `cargo metadata --locked --format-version 1` producer:0. Locked full-workspace `cargo nextest list --workspace --locked --message-format json --list-type binaries-only`: successful cell0,58 members/663 targets. Earlier3000s build deadline and Eval30s cancellation have no invented child exit code; empty compiler process scans preceded continuation. Full provenance: `l1-metadata-execution-record.json`.
- One actual standalone consumer: `cargo nextest run --no-fail-fast --profile ci --cargo-metadata <evidence>/l1-final-cargo-metadata.json --binaries-metadata <evidence>/l1-final-binaries-metadata-cell3.json --success-output immediate --failure-output final`: **100**,5640 run/5639 passed/1 failed/138 skipped,46.919s; UUID `16ed5fb9-66a1-4774-a94d-3e94c2321a0f`. Sole deterministic missing FR4 writer inventory was repaired above; no standalone retry. The earlier incompatible `--workspace --locked` consumer was CLI exit2 before tests; dropping those flags preserves the full locked producer scope.

## Once-only final prepush

Command (with only the documented temporary PYTHONPATH dependency and default-concurrency/retry unsets additionally supplied):

`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json`

**Exit0**,1186.646s, once; no tracked edits while running. `l1-final-prepush.json`: **83 pass,8 skip,1 advisory** over92 records. Current-tree workspace-nextest, kernel-core-lib, workspace doctests, fixture builder, kernel baseline/KLOC/boundaries, serde/env/composition/constructors/test-cost, epic coherence and Dev Agent Record all pass. Same-run **default-profile** JUnit: `l1-final-prepush-default-junit.xml`, UUID `ad63b740-ae30-44f2-8c68-1aee1df88f31`, **5640 cases,0 failures/errors**,46.635s. The CI-profile JUnit belongs to the earlier standalone red and is kept separately as `l1-final-workspace-junit.xml`; they are not interchanged.

### Aggregate test-budget advisory — unchanged budgets, open owner20-7

Prepush's advisory subcheck actually exits1; the wrapper remains0 by existing policy. It sums per-test time per binary, not the whole-run wall time. This is an observed cost finding, not an AC5 quarter-latency failure or a claim all gates were green. Same-run JUnit and the raw record at `.records[58]` are retained; no retry, budget increase or new exception.

| Binary / cases | Observed sum | Unchanged budget | Same-run evidence |
|---|---:|---:|---|
| maos-bin::hook_pump_17_4 /26 |32.528s|30s|`l1-final-prepush-default-junit.xml:1164` + prepush record58|
| maos-bin::one_daemon_one_door_16_1 /42 |33.478s|30s|`l1-final-prepush-default-junit.xml:1298` + prepush record58|
| maos-bin::shell_host_16_2 /14 |48.677s|30s|`l1-final-prepush-default-junit.xml:1517-1531` + prepush record58; mandatory1M case35.441s includes fixture setup|
| maos-iac::participant_recall_17_8 /29 |42.116s|30s|`l1-final-prepush-default-junit.xml:3213` + prepush record58|

Three targets are unchanged; shell_host contains this landing's required100k/1M vectors. These findings are explicitly recorded under20-7, not called unchanged functional test failures or silently waived. EC-13's actual bounded scan/hold correctness and literal25% witnesses pass; the aggregate-cost advisory remains open separately.

Eight skips: unchanged docs build/routes and dependency-deny inputs, plus five hosted dynamic-marker cardinalities whose CI runtime guards remain authoritative. No skipped hosted result is claimed as a pass.

## Performance and limitations

Final focused real disk/WAL writer ratios:100k **0.217150017**,1M **0.235940906**; the independent default-concurrency standalone workspace observed **0.198724650 /0.207434044**. Actual dense scan work max110619 VM steps/window and quarter-work fractions0.04095960 /0.004095994; all five predicates tested. Deadlines/observation budgets and4096-window/25% criteria were not widened. Earlier repaired1M run genuinely exercised pending503→durable completion→identical completed repeat; later runs completed before10s. Pending never claims erased/complete.

Hosted signed-T3 Worker, delegated cgroup/rootless runtime/network namespaces, hosted toolchain/discipline run id remain **unverified**. Schedule admission defaults are existing20-9 AC3 /D-20-9-08 A; the explicitly allowed AC6.3 real live-lineage fallback is used. Installed removal retains the existing cross-store crash-atomicity limitation; ordinary proof-write failure is now rollback-protected. FR65/NFR-Aud-12 and the story stay partial for hosted/final-kernel work; no tracker flip.

## Files changed

- Production: audit `src/erasure/proof.rs`, `src/fr4_classifier.rs`; IAC `src/adapter/transparency_log.rs`; registry `src/storage.rs`; bin `src/{main,operator_door,hook_pump}.rs`; CLI `src/cli.rs`; `tools/verify-erasure/verify.py`.
- Tests: audit `tests/{erasure_smoke_test,fr4_classifier_16_2}.rs`; new IAC test-only `src/adapter/erasure_scan_work.rs`; bin `tests/{erasure_inventory_20_11,erasure_live_20_11,erasure_scan_20_11,shell_host_16_2}.rs`, `tests/support/worker_revoke_root.rs`.
- Records: story Dev Agent Record/tasks/changelog/envelopes, `deferred-work.md`, `tests/coverage-matrix.yaml`, ADR-044, exact `xtask/kloc.toml`, and requested landing1 evidence. No new bin source/env var; source/env scanner lists intentionally unchanged. Original T0 preserved.

## Final records and cleanup

After recording the real prepush advisory/EC-13 closure, the prescribed prebuilt `check-dev-record-completeness --json`, `check-decision-register --json` and `check-exit-commands --json` each exit **0**. Raw `l1-final-records-*.json` artifacts retain those final record checks. No prepush rerun.

Final guarded cleanup exits **0** (`l1-final-cleanup.json`). Its `/proc` scan finds **no cargo/rustc** before deletion. It removes only owned `/tmp/dev2011b-resume-1l82gyty` and `/tmp/dev2011b-resume-runner.py`, plus **976** stale `debug/incremental/*/s-*` sessions, retaining newest-per-crate artifact directory and every session/descendant touched in the last2h. `deps`, `build`, fingerprints and other worktrees are untouched.

- Final pass: **11,480,789,636 logical bytes** removed; **10,136,141,824 exclusive allocated bytes** accounted; observed free-space increase **8,119,390,208 bytes** across the two filesystems (build7,837,392,896; tmp281,997,312).
- Earlier explicit interim cleanup (`l1-cleanup.json`):622 sessions,3,223,033,786 logical bytes; observed free-space increase2,865,713,152.
- Cumulative Dev2011b explicit cleanup: **1,598 sessions**, **14,703,823,422 logical bytes**, **13,192,818,688 exclusive allocated bytes**; sum of the two observed filesystem free-space increases **10,985,103,360 bytes**. Logical removal/exclusive allocation and observed free-space deltas are deliberately distinguished; unrelated filesystem activity is not claimed as measured reclamation.

No cargo/check process is started after cleanup. Remaining limitations are only the explicitly recorded hosted/final-kernel work and open20-7 aggregate-cost/timing observations, not unfinished L1 implementation.
