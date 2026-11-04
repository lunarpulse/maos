# Commands and observed exits — combined follow-up

Baseline `73e7afbcae81f4fb4858e9d52ded3d592bed636d`; cwd `/home/lunarpulse/dev_ws`. Exact argv strings, environment removals/overrides and timings are in `commands.json`; shell blocks below reconstruct that recorded environment. `CARGO_TARGET_DIR` was absent in Eval and never set/overridden; the existing repository `target` symlink resolves to `/mnt/build/cargo-maos`. No new target/worktree, cargo clean, commit/push or status flip.

The initial custody-only88-test phase is retained; Main then added the enumeration race and S5 cases. Final combined results are the `combined-*` tests/lint and `final-*` structural gates.

Expected nonzero results: six deterministic original/plant nextest reds exit100; three `diff -u` commands exit1 because differences exist; first KLOC attempt exits1 on a peer evidence .rs, then two formatted measurements exit1 on the old exact pins before authorized re-pin. No unclassified command failure. The first attempt to JSON-parse the pre-JSON KLOC error raised an Eval JSONDecodeError; that bookkeeping exception is not a test result.

The read-only source scans, proc scans and guarded Python session-directory pruning were Eval/tool operations rather than invented shell commands. Cleanup operation completed under nonblocking flock/POSIX locks after proc fd/maps/cwd/exe checks; detailed evidence is `cleanup-proc-scan.json`, `cleanup-result.json`, `cleanup-final.json`. No named scratch or restore intermediary remains.

## 1. workflow-customization-resolver

```sh
python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow
```
Observed exit: **0**. Elapsed seconds: `not recorded`.

## 2. baseline

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git rev-parse HEAD
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 3. cidfile-red-before

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac5_only_absent_cid_and_empty_state_are_reclaimed)'
```
Observed exit: **100**. Elapsed seconds: `2.768`.

## 4. fmt-custody

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.618`.

## 5. kloc-measure

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **1**. Elapsed seconds: `0.064`.

## 6. capture-first-green

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac5_)'
```
Observed exit: **0**. Elapsed seconds: `10.43`.

## 7. save-capture-original

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/purge-runtime.restore
```
Observed exit: **0**. Elapsed seconds: `0.008`.

## 8. capture-discard-plant-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/diff -u --label a/crates/maos-bin/src/purge_runtime.rs --label b/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/purge-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **1**. Elapsed seconds: `0.008`.

## 9. capture-plant-red

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac5_only_absent_cid_and_empty_state_are_reclaimed)'
```
Observed exit: **100**. Elapsed seconds: `7.826`.

## 10. capture-copy-back

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/purge-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.004`.

## 11. capture-cmp

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/purge-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.002`.

## 12. capture-touch

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 13. remove-capture-restore

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/rm -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/purge-runtime.restore
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 14. fmt-final-capture

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.918`.

## 15. kloc-measure-after-witness-rename

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **1**. Elapsed seconds: `0.064`.

## 16. fmt-final

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.667`.

## 17. kloc-check

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 18. registered-fixtures

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit
```
Observed exit: **0**. Elapsed seconds: `1.165`.

## 19. focused-serial-final

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --bin maos --test purge_residue_20_10 --test egress_daemon_17_1 --test worker_terminal_facts_17_7 --test maos_uninstall_16_4 --test root_shutdown_unload_16_3 --profile ci --test-threads 1 --no-fail-fast -E 'binary(purge_residue_20_10) | binary(egress_daemon_17_1) | binary(worker_terminal_facts_17_7) | binary(maos_uninstall_16_4) | binary(root_shutdown_unload_16_3) | test(purge::runtime_residue::tests::)'
```
Observed exit: **0**. Elapsed seconds: `114.39`.

## 20. save-focused-junit

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/focused-serial-junit.xml
```
Observed exit: **0**. Elapsed seconds: `0.002`.

## 21. workflow-contracts

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1 --no-capture
```
Observed exit: **0**. Elapsed seconds: `11.732`.

## 22. runtime-unit-safety

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(purge::runtime_residue::tests::)'
```
Observed exit: **0**. Elapsed seconds: `4.22`.

## 23. save-runtime-junit

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/runtime-unit-junit.xml
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 24. clippy-stream

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --workspace --all-targets --locked --message-format=json
```
Observed exit: **0**. Elapsed seconds: `137.12`.

## 25. clippy-ratchet

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/clippy.jsonl
```
Observed exit: **0**. Elapsed seconds: `0.215`.

## 26. rustc-version

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/rustc --version
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 27. hosted-removal-plant-applies

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git apply --check /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ac1-removal-noop.diff
```
Observed exit: **0**. Elapsed seconds: `0.007`.

## 28. stale-incremental-allocated

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/du -s -B1 -- /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-0cb54wbb9dp7l/s-hntq4tteqy-1ahh0r5-00ga0jqe2czt6gyiolp2n6m1u /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-3p00ttll6o2i3/s-hntpwt2z8a-1ewaz5k-4tnsrz1ghk12j02qbufjxb2id /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-0h4elxlvfei0e/s-hntp0y1mtm-1czv6o8-due9qi6vgjoietnz3q88peioz /mnt/build/cargo-maos/debug/incremental/purge_residue_20_10-2nzw0g951kesi/s-hntp0ra6g8-0jwkkjn-5gttueulyxq0owqx4jg9bxntl /mnt/build/cargo-maos/debug/incremental/purge_residue_20_10-34ednt38wdpee/s-hntomimgo3-1slnmbr-e78l1p20ibc80skux4or5g417 /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-0eweuw8ooxr3e/s-hntoltyjz3-0mhfr3j-0hnp7h831unwynifrwauybyx0 /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-2h3jwdc4u429k/s-hntn1wd08r-1ft94ov-1s6q3j9150z16omx4djiwtaka /mnt/build/cargo-maos/debug/incremental/purge_residue_20_10-3qlxh167u3u7y/s-hntazluajv-074vo2p-6y6q1j90i0uqt0dcki2hh4zkt /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-1zlmhvv0t8rqq/s-hntarc4aup-0096ass-939hmeun6d44c3rk6wsopfyww /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-3gz42dr5ovt02/s-hnt9zrpwwr-0nwq92f-52ko7rzjtf2hf9yl5krolcol5 /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-0m1vvvzo1me1u/s-hnt9kd5rfk-1bsoz85-54yf8onns0h3zpp1l0wwpnjlj /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-33mzm15tu1ake/s-hnrup097iz-0gzw1fk-ecs3sw8r8grwtg1dexknba9hz /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-1jktc27qtikxr/s-hnrrd5i61n-104ii3f-b020odi9lwx2223iq6prz6xca /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-1tn9kdbzhsixl/s-hnrr8ijg80-1m9n0pq-drxzcjbw5dp37rcyhov9tuaco /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-16pvcggczfowd/s-hnrr417064-0iws4rz-0gbenrvdc9h97ee3ntoiltj1k /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-3jqu1rj7aize9/s-hnrplzidbo-12khi5m-edclh4x1ke01dd8k2557bm6rd /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-2w0js2uygnfor/s-hnrpi6h2r4-0nkvg4m-4it1dtc1ib889m81ddyuxb1uk /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-3vbqfi0o4ajri/s-hnroxnpcbd-1p66yrq-6y1flp4u9vt9gb486datzwmuw /mnt/build/cargo-maos/debug/incremental/epic17_exit_workflow_17_7-0g9s5yfu0f9ef/s-hnrnpuiv3f-0ggq6qn-2y872txz8hxbf3k5372u07krx
```
Observed exit: **0**. Elapsed seconds: `0.068`.

## 29. flush-incremental-cleanup

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/sync -f /mnt/build/cargo-maos/debug/incremental
```
Observed exit: **0**. Elapsed seconds: `0.565`.

## 30. implementation-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git diff 73e7afbcae81f4fb4858e9d52ded3d592bed636d -- /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/purge_residue_20_10.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/xtask/tests/epic17_exit_workflow_17_7.rs /home/lunarpulse/dev_ws/xtask/kloc.toml /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/docs/maos.dev/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/run-maos.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 31. verify-cut-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git diff 73e7afbcae81f4fb4858e9d52ded3d592bed636d -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml
```
Observed exit: **0**. Elapsed seconds: `0.003`.

## 32. vanished-candidate-red-before

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac1_listed_candidate_vanishing_before_inspect_is_skipped)'
```
Observed exit: **100**. Elapsed seconds: `2.768`.

## 33. s5-oracle-red-before

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(s5_budget_oracle_counts_exceeded_kind_not_a_delayed_warning)'
```
Observed exit: **100**. Elapsed seconds: `7.676`.

## 34. fmt-combined

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.773`.

## 35. combined-kloc-measure

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **1**. Elapsed seconds: `0.064`.

## 36. race-s5-first-green

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(ac1_listed_candidate_) | test(ac1_failed_inspect_) | test(s5_budget_oracle_)'
```
Observed exit: **0**. Elapsed seconds: `8.777`.

## 37. save-vanished-original

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/vanished-runtime.restore
```
Observed exit: **0**. Elapsed seconds: `0.008`.

## 38. save-s5-original

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/s5-oracle.restore
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 39. vanished-skip-plant-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/diff -u --label a/crates/maos-bin/src/purge_runtime.rs --label b/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/vanished-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **1**. Elapsed seconds: `0.006`.

## 40. vanished-skip-plant-red

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac1_listed_candidate_vanishing_before_inspect_is_skipped)'
```
Observed exit: **100**. Elapsed seconds: `6.374`.

## 41. vanished-copy-back

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/vanished-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 42. vanished-cmp

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/vanished-runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 43. vanished-touch

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 44. remove-vanished-restore

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/rm -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/vanished-runtime.restore
```
Observed exit: **0**. Elapsed seconds: `0.002`.

## 45. s5-ratio-plant-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/diff -u --label a/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs --label b/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/s5-oracle.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs
```
Observed exit: **1**. Elapsed seconds: `0.007`.

## 46. s5-ratio-plant-red

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(s5_budget_oracle_counts_exceeded_kind_not_a_delayed_warning)'
```
Observed exit: **100**. Elapsed seconds: `6.424`.

## 47. s5-copy-back

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/s5-oracle.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 48. s5-cmp

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/s5-oracle.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 49. s5-touch

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/touch /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 50. remove-s5-restore

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/rm -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/s5-oracle.restore
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 51. fmt-combined-restored

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.618`.

## 52. combined-kloc-check

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 53. combined-registered-fixtures

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit
```
Observed exit: **0**. Elapsed seconds: `1.115`.

## 54. combined-focused-serial

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test egress_daemon_17_1 --test worker_terminal_facts_17_7 --test maos_uninstall_16_4 --test root_shutdown_unload_16_3 --profile ci --test-threads 1 --no-fail-fast
```
Observed exit: **0**. Elapsed seconds: `108.833`.

## 55. save-combined-focused-junit

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/combined-focused-junit.xml
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 56. combined-runtime-unit-safety

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(purge::runtime_residue::tests::)'
```
Observed exit: **0**. Elapsed seconds: `3.118`.

## 57. combined-workflow-contracts

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1 --no-capture
```
Observed exit: **0**. Elapsed seconds: `10.129`.

## 58. combined-clippy-stream

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --workspace --all-targets --locked --message-format=json
```
Observed exit: **0**. Elapsed seconds: `132.35`.

## 59. combined-clippy-ratchet

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-3/combined-clippy.jsonl
```
Observed exit: **0**. Elapsed seconds: `0.214`.

## 60. combined-implementation-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git diff 73e7afbcae81f4fb4858e9d52ded3d592bed636d -- /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/purge_residue_20_10.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/xtask/tests/epic17_exit_workflow_17_7.rs /home/lunarpulse/dev_ws/xtask/kloc.toml /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/docs/maos.dev/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/run-maos.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/supervisor_scenes.rs /home/lunarpulse/dev_ws/.github/workflows/discipline.yml
```
Observed exit: **0**. Elapsed seconds: `0.032`.

## 61. combined-verify-cut-diff

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git diff 73e7afbcae81f4fb4858e9d52ded3d592bed636d -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml
```
Observed exit: **0**. Elapsed seconds: `0.008`.

## 62. combined-hosted-plant-applies

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git apply --check /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ac1-removal-noop.diff
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 63. final-kloc-check

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **0**. Elapsed seconds: `0.124`.

## 64. final-check-kernel-baseline

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 65. final-check-service-boundary

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-service-boundary
```
Observed exit: **0**. Elapsed seconds: `1.165`.

## 66. final-check-env-contract

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-env-contract
```
Observed exit: **0**. Elapsed seconds: `0.464`.

## 67. final-check-serde-error-handling

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-serde-error-handling
```
Observed exit: **0**. Elapsed seconds: `0.264`.

## 68. final-check-composition-root-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-composition-root-completeness
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 69. final-check-j1-loopback-delegation

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 70. final-check-dev-record-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-dev-record-completeness
```
Observed exit: **0**. Elapsed seconds: `0.114`.

## 71. final-check-epic-close-coherence

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-epic-close-coherence
```
Observed exit: **0**. Elapsed seconds: `0.032`.

## 72. final-check-review-findings-resolved

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-review-findings-resolved
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 73. final-check-test-cost

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-test-cost
```
Observed exit: **0**. Elapsed seconds: `0.364`.

## 74. final-invariant-lock

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask invariant-lock
```
Observed exit: **0**. Elapsed seconds: `0.032`.

## 75. final-coverage-matrix

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask coverage-matrix --dry-run
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 76. final-check-coverage-matrix-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 77. final-check-exit-commands

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-exit-commands
```
Observed exit: **0**. Elapsed seconds: `0.615`.

## 78. final-check-decision-register

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-decision-register
```
Observed exit: **0**. Elapsed seconds: `0.007`.

## 79. final-check-ff-j6

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-ff-j6
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 80. final-check-perf-runner-policy

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-perf-runner-policy
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## Final observed outcomes

- All three original predicates/behaviors and all three individual mutations are observed assertion-red100; copy/cmp/touch/restoration and final fmt0.
- Combined focused serial91/91 (egress5, uninstall23, purge16, shutdown12, terminal35), exit0; library safety3/3, exit0; workflow contracts5/5, exit0. No configured skips in the91-test selected suites; optional real-runtime scene returns remain unverified hosted proof.
- Non-denying workspace/all-target clippy0; complete-stream ratchet0 at1331/871 against unchanged1383/897 baseline. No new lint/file/message multiplicities vs complete corrected20-4 local stream; unchanged warnings remain. Local Rust1.98.1 only.
- Final18 structural/record gates each0: KLOC; kernel baseline; service; env; serde; composition roots; J1; dev-record completeness; epic coherence; review findings; test cost; invariant lock; coverage dry-run and completeness; exit commands; decision register; ff-j6; perf-runner policy. Open decision-register rows and dispositioned historical ownerless entries are not claimed closed.
- Exact measured maos-bin34551 <=34643 cap; aggregate193187 <=193526 cap, hardfail193188; xtask44452 unchanged. Kernel26841 physical/21042 code,95 files, no changed/added/removed files. Dated orchestrator-adjudicated D02 A re-base recorded in story.
- Cleanup19 stale session directories,234217472 allocated bytes unlinked,167460864-byte observed free-space increase after flush; filesystem-wide delta is not exclusive physical reclamation. No current sessions/deps/crate roots removed. Final process/scratch scan clear.
- Corrected hosted runtime/concurrent enumeration/real S5 and hosted-toolchain lint remain operator verification. No new full workspace/default/prepush, cargo-deny or docs-site build matrix requested/run in this scoped pass.
