# 20-10 hosted incident — exact command ledger

Baseline: `16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de`. Working directory: `/home/lunarpulse/dev_ws`. Cargo operations were serialized; the 20-4 source lane was parked by its owner before final validation. No agent CARGO_TARGET_DIR override/clean or extra worktree was used. Environment removals/overrides are explicit below and in adjacent `commands.json`; other environment was inherited.

Most command outputs use `<name>.log`. Rows with unmeasured wall time show —, not an estimate. `unobserved` is the combined-cell interruption during compilation, not a fabricated process exit. Controller wall times for background commands are preserved.

Expected reds: baseline/plant assertion exits100. Deterministic fixture-stale exits100 were fixed by actual registered-fixture provisioning, never by suppressing freshness checks. Final serial5573/5573 and focused164/164 passed. Final default5572/5573 exited100 on the known snapshot-owner timing family and was NOT retried. Prescribed prepush independently exited0; its budget leg was advisory1. First ratchet1 was truncated artifact input; the SAME successful clippy stream was paged completely, then ratchet0 (1335/873), without rerunning clippy. Baseline1383/897 unchanged.

| Name | Exact command | Removed environment | Extra environment | Exit | Wall seconds |
|---|---|---|---|---:|---:|
| job-125746775456 | `gh api repos/lunarpulse/maos/actions/jobs/125746775456/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 2.067 |
| job-125746775593 | `gh api repos/lunarpulse/maos/actions/jobs/125746775593/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 1.516 |
| job-125746775660 | `gh api repos/lunarpulse/maos/actions/jobs/125746775660/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 1.616 |
| job-125749866755 | `gh api repos/lunarpulse/maos/actions/jobs/125749866755/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 2.017 |
| job-125749866879 | `gh api repos/lunarpulse/maos/actions/jobs/125749866879/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 2.067 |
| job-125749866883 | `gh api repos/lunarpulse/maos/actions/jobs/125749866883/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 2.417 |
| job-125749866891 | `gh api repos/lunarpulse/maos/actions/jobs/125749866891/logs --allow-escape-sequences` | CI, GITHUB_ACTIONS | — | 0 | 1.967 |
| local-baseline | `git rev-parse HEAD` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| workflow-contract-red | `cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1 -E 'test(purge_cut_units_pass_the_real_harness_unit_validation)'` | CI, GITHUB_ACTIONS | — | 100 | 48.803 |
| fmt-hosted-contract | `cargo fmt --all` | CI, GITHUB_ACTIONS | — | 0 | 2.718 |
| measure-hosted-contract | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| workflow-contract-green | `cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1` | CI, GITHUB_ACTIONS | — | 0 | 2.217 |
| save-workflow-plant-original | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workflow.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.008 |
| workflow-contract-plant | `cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1 -E 'test(purge_cut_units_pass_the_real_harness_unit_validation)'` | CI, GITHUB_ACTIONS | — | 100 | 0.514 |
| workflow-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workflow.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| workflow-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workflow.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| workflow-touch | `touch /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| workflow-contract-restored | `cargo nextest run --locked -p xtask --test epic17_exit_workflow_17_7 --profile ci --test-threads 1` | CI, GITHUB_ACTIONS | — | 0 | 0.665 |
| fmt-final | `cargo fmt --all` | CI, GITHUB_ACTIONS | — | 0 | 2.567 |
| kloc-exact-final | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| build-all-targets | `cargo build --workspace --all-targets --locked` | CI, GITHUB_ACTIONS | — | 0 | 2804.9 |
| registered-fixtures | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 563.614 |
| save-main-nfr-rel-11 | `cp /tmp/l10-nfr-rel-11.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-nfr-rel-11.log` | CI, GITHUB_ACTIONS | — | 0 | 0.127 |
| save-main-workspace | `cp /tmp/l10-workspace.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-workspace.log` | CI, GITHUB_ACTIONS | — | 0 | 0.464 |
| save-main-loom | `cp /tmp/l10-loom.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-loom.log` | CI, GITHUB_ACTIONS | — | 0 | 0.164 |
| save-main-epic17-24 | `cp /tmp/l10-epic17-24.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-epic17-24.log` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| save-main-epic17-26 | `cp /tmp/l10-epic17-26.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-epic17-26.log` | CI, GITHUB_ACTIONS | — | 0 | 0.003 |
| save-main-budget | `cp /tmp/l10-budget.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/main-budget.log` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| focused-purge-interrupted | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_supervision_16_3 --test worker_terminal_facts_17_7 --test worker_workspace_19_3 --lib --profile ci --test-threads 1 --no-fail-fast` | CI, GITHUB_ACTIONS | — | unobserved | — |
| hosted-regressions-red | `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test egress_daemon_17_1 --test worker_terminal_facts_17_7 --profile ci --test-threads 1 --no-fail-fast -E 'test(hosted_length_home_keeps_egress_socket_below_sun_path_limit) \| test(purge_cgroup_native_binding_and_dead_creator_twins)'` | CI, GITHUB_ACTIONS | — | 100 | 506.78 |
| fmt-regressions | `cargo fmt --all` | CI, GITHUB_ACTIONS | — | 0 | 3.273 |
| measure-regressions | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 1 | 0.174 |
| focused-prerequisite-stale | `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --lib --test egress_daemon_17_1 --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_supervision_16_3 --test worker_terminal_facts_17_7 --test worker_workspace_19_3 --test root_shutdown_unload_16_3 --profile ci --test-threads 1 --no-fail-fast` | CI, GITHUB_ACTIONS | — | 100 | 540.81 |
| fmt-before-source-plants | `cargo fmt --all` | CI, GITHUB_ACTIONS | — | 0 | 3.027 |
| save-egress-original | `cp /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/egress.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.004 |
| save-scene-original | `cp /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/scene.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| plant-F1-egress | `cargo nextest run --locked -p maos-bin --test egress_daemon_17_1 --profile ci --test-threads 1 -E 'test(hosted_length_home_keeps_egress_socket_below_sun_path_limit)'` | CI, GITHUB_ACTIONS | — | 100 | 479.676 |
| F1-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/egress.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.006 |
| F1-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/egress.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.004 |
| F1-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| plant-F2-home | `cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --profile ci --test-threads 1 -E 'test(purge_cgroup_native_binding_and_dead_creator_twins)'` | CI, GITHUB_ACTIONS | — | 100 | 190.06 |
| F2-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/scene.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.004 |
| F2-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/scene.restore /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| F2-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| fmt-final-restored | `cargo fmt --all` | CI, GITHUB_ACTIONS | — | 0 | 2.868 |
| kloc-repinned | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.165 |
| fixtures-after-park | `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 283.69 |
| remove-plant-restore-scaffolds | `rm -f /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workflow.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/egress.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/scene.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.008 |
| save-intent-lineage-original | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/intent-lineage.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.009 |
| build-all-targets-final | `env -u CI -u GITHUB_ACTIONS cargo build --workspace --all-targets --locked` | CI, GITHUB_ACTIONS | — | 0 | 597.74 |
| resolve-workflow | `python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow` | — | — | 0 | — |
| hosted-job-roster | `gh run view 41584515474 --json jobs` | — | — | 0 | — |
| process-guard-portable | `ps -C cargo,rustc,cargo-nextest -o pid,ppid,args` | — | — | 1 | — |
| process-guard-system | `/usr/bin/ps -C cargo,rustc,cargo-nextest -o pid,ppid,args` | — | — | 0 | — |
| process-guard-arguments | `/usr/bin/ps -ww -p 3359843,3366449 -o pid,ppid,args` | — | — | 0 | — |
| stop-interrupted-build | `kill -TERM 3359843 3366449` | — | — | 1 | — |
| focused-final | `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --lib --test egress_daemon_17_1 --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_supervision_16_3 --test worker_terminal_facts_17_7 --test worker_workspace_19_3 --test root_shutdown_unload_16_3 --profile ci --test-threads 1 --no-fail-fast` | CI, GITHUB_ACTIONS | — | 0 | 134.72 |
| host-before-final-serial | `uptime` | CI, GITHUB_ACTIONS | — | 0 | 0.011 |
| workspace-serial-prerequisite-stale | `env -u CI -u GITHUB_ACTIONS cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1` | CI, GITHUB_ACTIONS | — | 100 | 1089.87 |
| save-stale-serial-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workspace-serial-prerequisite-stale-junit.xml` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| all-registered-fixtures | `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask build-test-fixtures` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 23.09 |
| host-before-refreshed-serial | `uptime` | CI, GITHUB_ACTIONS | — | 0 | 0.004 |
| workspace-serial-final | `env -u CI -u GITHUB_ACTIONS cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1` | CI, GITHUB_ACTIONS | — | 0 | 647.09 |
| save-final-serial-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workspace-serial-final-junit.xml` | CI, GITHUB_ACTIONS | — | 0 | 0.003 |
| host-before-final-budget | `uptime` | CI, GITHUB_ACTIONS | — | 0 | 0.004 |
| check-test-budget-final | `/mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workspace-serial-final-junit.xml --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| workspace-default-final | `env -u CI -u GITHUB_ACTIONS cargo nextest run --workspace --locked --no-fail-fast --profile ci` | CI, GITHUB_ACTIONS | — | 100 | 46.37 |
| save-final-default-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/workspace-default-final-junit.xml` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| kloc-check | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-kernel-baseline | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.015 |
| check-service-boundary | `/mnt/build/cargo-maos/debug/xtask check-service-boundary --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 1.165 |
| check-env-contract | `/mnt/build/cargo-maos/debug/xtask check-env-contract --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.464 |
| check-serde-error-handling | `/mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.264 |
| check-composition-root-completeness | `/mnt/build/cargo-maos/debug/xtask check-composition-root-completeness --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-j1-loopback-delegation | `/mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-dev-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.114 |
| check-epic-close-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| check-review-findings-resolved | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-test-cost | `/mnt/build/cargo-maos/debug/xtask check-test-cost --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.264 |
| invariant-lock | `/mnt/build/cargo-maos/debug/xtask invariant-lock --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| coverage-matrix | `/mnt/build/cargo-maos/debug/xtask coverage-matrix --json --dry-run` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.015 |
| check-coverage-matrix-completeness | `/mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.016 |
| check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.214 |
| check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.008 |
| check-ff-j6 | `/mnt/build/cargo-maos/debug/xtask check-ff-j6 --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-perf-runner-policy | `/mnt/build/cargo-maos/debug/xtask check-perf-runner-policy --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| local-toolchain | `rustc -Vv` | CI, GITHUB_ACTIONS | — | 0 | 0.032 |
| incident-record-date | `date --iso-8601=seconds` | CI, GITHUB_ACTIONS | — | 0 | 0.006 |
| clippy-stream-final | `env -u CI -u GITHUB_ACTIONS cargo clippy --workspace --all-targets --locked --message-format=json` | CI, GITHUB_ACTIONS | — | 0 | 446.13 |
| clippy-ratchet-final | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/clippy-final.jsonl` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 1 | 0.064 |
| cargo-deny-final | `cargo deny check` | CI, GITHUB_ACTIONS | — | 0 | 0.765 |
| clippy-ratchet-complete-stream | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/clippy-final.jsonl` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.164 |
| prepush-final | `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask; MAOS_17_2_CGROUP_MODE=undelegated | 0 | 960.91 |
| save-prepush-final-junit | `cp /mnt/build/cargo-maos/nextest/default/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/prepush-final-junit.xml` | CI, GITHUB_ACTIONS | — | 0 | 0.005 |
| compare-intent-lineage | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/intent-lineage.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| remove-final-restore-scaffold | `rm -f /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/intent-lineage.restore` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| check-dev-record-completeness-final-record | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.165 |
| check-epic-close-coherence-final-record | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.033 |
| check-review-findings-resolved-final-record | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | CI, GITHUB_ACTIONS | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| implementation-tracked-diff | `git diff 16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de -- /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/egress_daemon_17_1.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/purge_residue_20_10.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/root_shutdown_unload_16_3.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/xtask/tests/epic17_exit_workflow_17_7.rs /home/lunarpulse/dev_ws/xtask/kloc.toml /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/deferred-work.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/plants.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml` | CI, GITHUB_ACTIONS | — | 0 | 0.016 |
| save-implementation-diff | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/implementation-tracked-diff.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/implementation.diff` | CI, GITHUB_ACTIONS | — | 0 | 0.001 |
| ci-cut-diff | `git diff 16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de -- /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml` | CI, GITHUB_ACTIONS | — | 0 | 0.003 |
| save-ci-cut-diff | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/ci-cut-diff.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/ci-cut.diff` | CI, GITHUB_ACTIONS | — | 0 | 0.002 |
| final-diff-sha256 | `sha256sum /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/implementation.diff /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/hosted-1/ci-cut.diff` | CI, GITHUB_ACTIONS | — | 0 | 0.009 |

## Non-green command dispositions

- **focused-purge-interrupted**: Outer eval cell hit 3600 seconds during compilation after the recorded build and fixture commands. No test result or exit code observed. Orphan build PIDs3359843/3366449 were observed; TERM returned1 (failed to send signal), so no successful termination is claimed.
- **focused-prerequisite-stale**: 146 passed / 18 failed; stale registered fixture after 20-4 source parking, not a flake. Reprovisioning required before final matrix.
- **hosted-job-roster**: Full JSON preserved in jobs.json.
- **process-guard-portable**: Harness ps does not support -C.
- **process-guard-system**: Observed orphaned own focused cargo compile.
- **process-guard-arguments**: Exact --test target list matched the interrupted focused command.
- **stop-interrupted-build**: Failed to send signal to either PID; no successful termination claimed.
