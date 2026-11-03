# Scoped follow-up commands and observed exits

Working directory: `/home/lunarpulse/dev_ws`. `CARGO_TARGET_DIR` was absent from the Eval process environment and was never set or overridden; the repository’s existing `target` symlink resolves to `/mnt/build/cargo-maos`. No alternate target/worktree, cargo clean, commit or push. CI/GITHUB_ACTIONS were removed for cargo and gate commands. Source fixes were restored before the final focused run.

The two exit100 OS commands are the deliberate original-layout and nested-layout invariant reds, not flaky retries. All other OS commands exited0. Parent scoped this follow-up without a new full workspace/default/prepush matrix.

The proc scan and guarded granular Python cleanup are Eval operations, not shell commands with invented exit codes: both completed; full inputs/results are in `cleanup-proc-scan.json` / `cleanup-result.json`. No candidate references or same-UID scan denial; nonblocking session locks acquired. Workflow guard inspection and fixture scan were read-only tool/Eval operations with saved findings. An initial ledger-render-only Eval cell exited1 on `KeyError: CARGO_TARGET_DIR`; only the rendering step was corrected to record the actually unset environment. No check was rerun for that evidence-rendering error.

## 1. workflow-customization-resolver

```sh
python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow
```
Observed exit: **0**. Elapsed seconds: `not recorded`.

## 2. baseline

```sh
env -u CI -u GITHUB_ACTIONS git rev-parse HEAD
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 3. layout-red-before

```sh
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(root_crash_retains_stopped_container_and_purge_reports_it)'
```
Observed exit: **100**. Elapsed seconds: `51.24`.

## 4. fmt-layout

```sh
env -u CI -u GITHUB_ACTIONS cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.618`.

## 5. kloc-layout

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 6. save-layout-original

```sh
env -u CI -u GITHUB_ACTIONS cp /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/scene.restore
```
Observed exit: **0**. Elapsed seconds: `0.004`.

## 7. nested-layout-plant

```sh
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(root_crash_retains_stopped_container_and_purge_reports_it)'
```
Observed exit: **100**. Elapsed seconds: `3.219`.

## 8. layout-copy-back

```sh
env -u CI -u GITHUB_ACTIONS cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/scene.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 9. layout-cmp

```sh
env -u CI -u GITHUB_ACTIONS cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/scene.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs
```
Observed exit: **0**. Elapsed seconds: `0.002`.

## 10. layout-touch

```sh
env -u CI -u GITHUB_ACTIONS touch /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs
```
Observed exit: **0**. Elapsed seconds: `0.001`.

## 11. remove-layout-restore

```sh
env -u CI -u GITHUB_ACTIONS rm /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/scene.restore
```
Observed exit: **0**. Elapsed seconds: `0.002`.

## 12. fmt-restored-layout

```sh
env -u CI -u GITHUB_ACTIONS cargo fmt --all
```
Observed exit: **0**. Elapsed seconds: `2.567`.

## 13. focused-serial-green

```sh
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --test purge_residue_20_10 --test maos_uninstall_16_4 --test root_shutdown_unload_16_3 --profile ci --test-threads 1 --no-fail-fast
```
Observed exit: **0**. Elapsed seconds: `101.88`.

## 14. save-focused-junit

```sh
env -u CI -u GITHUB_ACTIONS cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/focused-serial-junit.xml
```
Observed exit: **0**. Elapsed seconds: `0.004`.

## 15. kloc-check

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 16. check-kernel-baseline

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json
```
Observed exit: **0**. Elapsed seconds: `0.032`.

## 17. check-env-contract

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-env-contract --json
```
Observed exit: **0**. Elapsed seconds: `0.464`.

## 18. check-exit-commands

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-exit-commands --json
```
Observed exit: **0**. Elapsed seconds: `0.314`.

## 19. check-decision-register

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-decision-register --json
```
Observed exit: **0**. Elapsed seconds: `0.007`.

## 20. invariant-lock

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask invariant-lock --json
```
Observed exit: **0**. Elapsed seconds: `0.015`.

## 21. clippy-stream

```sh
env -u CI -u GITHUB_ACTIONS cargo clippy --workspace --all-targets --locked --message-format=json
```
Observed exit: **0**. Elapsed seconds: `469.88`.

## 22. clippy-ratchet

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-2/clippy.jsonl
```
Observed exit: **0**. Elapsed seconds: `0.217`.

## 23. stale-incremental-allocated

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/du -s -B1 -- /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-0eweuw8ooxr3e/s-hntfgfsns9-0fv2w0o-0mx1q05c1p7id7wjlccbxvqhy /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-0h4elxlvfei0e/s-hntdkfwtwj-071hqgm-0gwm3pkzoqi3yvci40qw1d3xp /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-07kv3rmp2me99/s-hntbd1gdb1-1pivyn0-cltrwgrsuys0phr0vao3mfjt6 /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-2wdbdmvyekoad/s-hnsowv2gpf-1lh1c1k-1s7vetwafn52ryeoyxi7uk2en /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-3vu4050p375h1/s-hnrxaz0vnb-0m2jh6f-5q985dxyu3f13y16sbr1od5yv /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-2y11eebiilif3/s-hnruptpthy-0cuja0d-8jo0t45gj8484pud6inlct0w1 /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-0lsr1s0eth572/s-hnrrk4lfyk-10d5rci-14haja6f0at9272r3kdvw4y6f /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-36pgj1fijhapr/s-hnrrdqtoe0-0kwukrq-aj8pxgwqzmga6aei2rspwll0s /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-11focwc3c2zhc/s-hnrr91q8v4-172b402-61p1k6ntv1wcxgoh3r0ds5djr /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-08hlehv5077qn/s-hnrr4ygm22-1is17v2-6zxo9n34na1kgwqn58sxtvj38 /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-22fcmf2on45gg/s-hnrpmhqyjf-027ltq2-ep4ioy60i1uvjp8hxocwas56d /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-30f7fssklqbzk/s-hnrpjbjeet-1edbf6v-c7t6lvqyhm64ts0ojmfyrkwje /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-3etr28h9p7qlu/s-hnrnq9tkge-0yemajk-arx73jzeodzsq9mda36uac5hk /mnt/build/cargo-maos/debug/incremental/worker_terminal_facts_17_7-378vt572dxlrd/s-hnrn9h0vr2-1to7imn-azca0vv8zwhclxcodnkomadtt
```
Observed exit: **0**. Elapsed seconds: `0.116`.

## 24. incremental-filesystem

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/stat -f -c %T /mnt/build/cargo-maos/debug/incremental
```
Observed exit: **0**. Elapsed seconds: `0.008`.

## 25. flush-incremental-cleanup

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/sync -f /mnt/build/cargo-maos/debug/incremental
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 26. layout-fix

```sh
env -u CI -u GITHUB_ACTIONS /usr/bin/git diff 2752e750c8cc7cfcb6fed2ccabbd4e4e0ed4f040 -- /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs
```
Observed exit: **0**. Elapsed seconds: `0.016`.

## 27. rustc-version

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/rustc --version
```
Observed exit: **0**. Elapsed seconds: `0.071`.

## 28. check-service-boundary

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-service-boundary
```
Observed exit: **0**. Elapsed seconds: `1.216`.

## 29. check-serde-error-handling

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-serde-error-handling
```
Observed exit: **0**. Elapsed seconds: `0.264`.

## 30. check-composition-root-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-composition-root-completeness
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 31. check-j1-loopback-delegation

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 32. check-test-cost

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-test-cost
```
Observed exit: **0**. Elapsed seconds: `0.314`.

## 33. coverage-matrix

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask coverage-matrix --dry-run
```
Observed exit: **0**. Elapsed seconds: `0.015`.

## 34. check-coverage-matrix-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness
```
Observed exit: **0**. Elapsed seconds: `0.015`.

## 35. check-ff-j6

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-ff-j6
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 36. check-perf-runner-policy

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-perf-runner-policy
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## 37. check-dev-record-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-dev-record-completeness
```
Observed exit: **0**. Elapsed seconds: `0.114`.

## 38. check-epic-close-coherence

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-epic-close-coherence
```
Observed exit: **0**. Elapsed seconds: `0.032`.

## 39. check-review-findings-resolved

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-review-findings-resolved
```
Observed exit: **0**. Elapsed seconds: `0.064`.

## Final outcomes

- Original-layout red100 and nested-layout mutation red100 on the intended canonical shared-ancestor assertion; cp/cmp/touch restore0; final fmt0.
- Focused serial81/81, terminal suite34 unchanged, exit0. Fresh junit: `focused-serial-junit.xml`.
- Eighteen structural/record gates passed: KLOC, kernel baseline, env contract, exit commands, decision register, invariant lock, service boundary, serde handling, composition roots, J1 delegation, test cost, coverage matrix dry-run and completeness, ff-j6, performance-runner policy, dev-record completeness, epic coherence, review resolution.
- Workspace/all-target non-denying clippy0, complete-stream ratchet0:1331/871 against unchanged1383/897 baseline. Changed Rust file primary warnings0 (`clippy-layout-delta.json`); local Rust1.98.1, not a hosted-toolchain result.
- KLOC unchanged: maos-bin34363, aggregate192896 / inherited hardfail192897, xtask44420; kernel26841 physical /21042 code, zero changed kernel files.
- Removed14 individual stale terminal incremental sessions (653934592 allocated bytes); observed free-space increase after Btrfs flush12709888 bytes. Free-space delta is filesystem-wide, not exclusive physical-reclamation proof.
- New real delegated/podman baseline/head/plant remains operator-owned and unverified. Prior StoreInUse reds are not accepted removal reds; workflow intended diagnostics/restoration/count34 unchanged.
