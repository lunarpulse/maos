# 20-10 landing 1 — command ledger

All implementation/build/test/gate subprocesses below ran in
`/home/lunarpulse/dev_ws`, with CI and GITHUB_ACTIONS unset. There was no
CARGO_TARGET_DIR override and no cargo clean; the existing target symlink
resolved to `/mnt/build/cargo-maos`. `commands.json` records exact environment
overrides, including private T0 HOME/TMPDIR/XDG roots and the prescribed
prepush manifest/cgroup environment. Cargo was never launched concurrently.
The initial shell fact commands and workflow resolver are included separately;
their wall times were not recorded. File read/edit tools are not shell commands.
A Python probe setup cell raised KeyError before spawning its first subprocess;
the failed lookup was replaced by the observed absolute binary path.

Exit 100 is an expected nextest red for HEAD witnesses/plants. The initial
focused timeout and initial serial workflow-reader red were repaired; those
non-green runs remain in the ledger. Intermediate kloc rc 1 is the expected
old-pin mismatch before exact re-pinning, never a ruled-envelope breach.
`prepush` rc 0 retains its advisory-red loaded budget findings; it was not
retried-to-green. Fresh serial budget rc 0 is a separate recorded result.

| Name | Exact argv / shell command | Exit | Wall seconds |
|---|---|---:|---:|
| workflow-resolver | `python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow` | 0 | not recorded |
| initial-host-prerequisites | `eval 'printf "target=%s\n" "$CARGO_TARGET_DIR"; uptime; df -h /home/lunarpulse/dev_ws "$CARGO_TARGET_DIR"; git rev-parse HEAD; command -v docker; command -v podman'` | 0 | not recorded |
| cargo-shell-environment | `eval 'printf "CARGO_HOME=%s\nRUSTUP_HOME=%s\nPATH=%s\n" "$CARGO_HOME" "$RUSTUP_HOME" "$PATH"'` | 0 | not recorded |
| baseline-binary-mtimes | `stat -c '%Y %n' /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-0fc3dee264a5ed2b /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-3870559fb9502ad1 /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-4411605c1c2778f4 /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-4c200981a819086d /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-51d13ad7720cc66a /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-650c6ee6017b5903 /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-7739f8dd2668007c /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-8e69a24c1fa45fbd /mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-e471644dfe4721e3` | 0 | not recorded |
| host-cpus | `nproc` | 0 | not recorded |
| red-before-head | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 --no-fail-fast` | 100 | 410.162 |
| t0-init | `/mnt/build/cargo-maos/debug/maos init --plain` | 0 | 0.033 |
| t0-oneshot-0 | `/mnt/build/cargo-maos/debug/maos run /home/lunarpulse/dev_ws/spirits/researcher/manifest.toml --once` | 0 | 0.915 |
| t0-oneshot-1 | `/mnt/build/cargo-maos/debug/maos run /home/lunarpulse/dev_ws/spirits/researcher/manifest.toml --once` | 0 | 0.114 |
| t0-oneshot-2 | `/mnt/build/cargo-maos/debug/maos run /home/lunarpulse/dev_ws/spirits/researcher/manifest.toml --once` | 0 | 0.114 |
| t0-supervisor-fixtures | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --test worker_crash_corpus_16_3 --test worker_terminal_facts_17_7 --profile ci --test-threads 1 --no-fail-fast` | 0 | 834.087 |
| t0-single-worker-root | `/mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-650c6ee6017b5903 --exact a_supervised_worker_completes_on_a_single_worker_runtime --nocapture` | 0 | 3.776 |
| t0-mint-stop-root | `/mnt/build/cargo-maos/debug/deps/worker_supervision_16_3-650c6ee6017b5903 --exact a_stop_during_the_mint_never_spawns_the_child --nocapture` | 0 | 2.918 |
| fmt-all | `cargo fmt --all` | 0 | 3.123 |
| kloc-measure | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.164 |
| build-all-targets | `cargo build --locked --workspace --all-targets` | 0 | 2321.791 |
| focused-green | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_workspace_19_3 --test worker_terminal_facts_17_7 --test worker_supervision_16_3 --profile ci --test-threads 1 --no-fail-fast` | 100 | 249.422 |
| focused-cgroups | `cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(purge::runtime_residue)'` | 0 | 13.95 |
| xtask-help | `/mnt/build/cargo-maos/debug/xtask --help` | 0 | 0.117 |
| host-load-before-suite | `uptime` | 0 | 0.009 |
| fmt-final | `cargo fmt --all` | 0 | 3.02 |
| kloc-final-measure | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.064 |
| whole-package-fixtures | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit` | 0 | 1.616 |
| plant-backup-runtime | `cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore` | 0 | 0.005 |
| plant-ac2 | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac2_other_home)'` | 100 | 25.205 |
| ac2-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| ac2-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.002 |
| ac2-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| plant-ac4 | `cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(ac4_dead_empty_live_creator)'` | 100 | 3.419 |
| ac4-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.002 |
| ac4-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| ac4-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| plant-ac5 | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac5_only_absent_cid)'` | 100 | 8.487 |
| ac5-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.002 |
| ac5-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| ac5-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| plant-ac6 | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac6_runtime_helper)'` | 100 | 6.575 |
| ac6-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| ac6-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| ac6-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | 0 | 0.001 |
| plant-backup-purge | `cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge.restore` | 0 | 0.001 |
| plant-ac3 | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac3_dry_run)'` | 100 | 5.723 |
| ac3-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | 0 | 0.001 |
| ac3-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | 0 | 0.001 |
| ac3-touch | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | 0 | 0.001 |
| plant-backup-adr | `cp /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/adr.restore` | 0 | 0.001 |
| plant-ac7 | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac7_adr)'` | 100 | 5.823 |
| ac7-copy-back | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/adr.restore /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md` | 0 | 0.001 |
| ac7-cmp | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/adr.restore /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md` | 0 | 0.001 |
| ac7-touch | `touch /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md` | 0 | 0.001 |
| fmt-restored-tree | `cargo fmt --all` | 0 | 2.768 |
| focused-restored-green | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_workspace_19_3 --test worker_terminal_facts_17_7 --test worker_supervision_16_3 --lib --profile ci --test-threads 1 --no-fail-fast` | 0 | 120.052 |
| save-intent-lineage-report | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/intent-lineage.restore` | 0 | 0.002 |
| host-before-workspace-serial | `uptime` | 0 | 0.004 |
| workspace-serial | `cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1` | 100 | 1158.837 |
| save-serial-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/workspace-serial-junit.xml` | 0 | 0.006 |
| check-test-budget-help | `/mnt/build/cargo-maos/debug/xtask check-test-budget --help` | 0 | 0.066 |
| coverage-matrix-help | `/mnt/build/cargo-maos/debug/xtask coverage-matrix --help` | 0 | 0.009 |
| invariant-lock-help | `/mnt/build/cargo-maos/debug/xtask invariant-lock --help` | 0 | 0.007 |
| clippy-ratchet-help | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --help` | 0 | 0.003 |
| fmt-workflow-reader | `cargo fmt --all` | 0 | 2.969 |
| clippy-stream | `cargo clippy --workspace --all-targets --locked --message-format=json` | 0 | 458.3 |
| clippy-ratchet | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/clippy.jsonl` | 0 | 0.215 |
| fmt-lint-correction | `cargo fmt --all` | 0 | 2.826 |
| host-before-final-serial | `uptime` | 0 | 0.008 |
| workspace-final-serial | `cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1` | 0 | 651.652 |
| save-final-serial-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/workspace-final-serial-junit.xml` | 0 | 0.007 |
| host-before-budget | `uptime` | 0 | 0.01 |
| check-test-budget | `/mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/workspace-final-serial-junit.xml --json` | 0 | 0.032 |
| workspace-default | `cargo nextest run --workspace --locked --no-fail-fast --profile ci` | 0 | 49.457 |
| kloc-check | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 0 | 0.064 |
| check-kernel-baseline | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | 0 | 0.016 |
| check-service-boundary | `/mnt/build/cargo-maos/debug/xtask check-service-boundary --json` | 0 | 1.165 |
| check-env-contract | `/mnt/build/cargo-maos/debug/xtask check-env-contract --json` | 0 | 0.464 |
| check-serde-error-handling | `/mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` | 0 | 0.264 |
| check-composition-root-completeness | `/mnt/build/cargo-maos/debug/xtask check-composition-root-completeness --json` | 0 | 0.064 |
| check-j1-loopback-delegation | `/mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation --json` | 0 | 0.064 |
| check-dev-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | 0 | 0.114 |
| check-epic-close-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | 0 | 0.032 |
| check-review-findings-resolved | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | 0 | 0.064 |
| check-test-cost | `/mnt/build/cargo-maos/debug/xtask check-test-cost --json` | 0 | 0.264 |
| invariant-lock | `/mnt/build/cargo-maos/debug/xtask invariant-lock --json` | 0 | 0.032 |
| coverage-matrix | `/mnt/build/cargo-maos/debug/xtask coverage-matrix --json --dry-run` | 0 | 0.016 |
| check-coverage-matrix-completeness | `/mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness --json` | 0 | 0.016 |
| check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | 0 | 0.214 |
| check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | 0 | 0.007 |
| check-ff-j6 | `/mnt/build/cargo-maos/debug/xtask check-ff-j6 --json` | 0 | 0.064 |
| check-perf-runner-policy | `/mnt/build/cargo-maos/debug/xtask check-perf-runner-policy --json` | 0 | 0.064 |
| build-final-all-targets | `cargo build --locked --workspace --all-targets` | 0 | 259.02 |
| clippy-final-stream | `cargo clippy --workspace --all-targets --locked --message-format=json` | 0 | 6.076 |
| cargo-deny | `cargo deny check` | 0 | 1.667 |
| clippy-final-ratchet | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/clippy-final.jsonl` | 0 | 0.216 |
| producer-fixed-0 | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --profile ci --test-threads 1 -E 'test(a_stop_during_the_mint_never_spawns_the_child)'` | 0 | 0.715 |
| producer-fixed-1 | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --profile ci --test-threads 1 -E 'test(a_stop_during_the_mint_never_spawns_the_child)'` | 0 | 0.565 |
| producer-fixed-2 | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --profile ci --test-threads 1 -E 'test(a_stop_during_the_mint_never_spawns_the_child)'` | 0 | 0.615 |
| prepush | `/mnt/build/cargo-maos/debug/xtask prepush --json` | 0 | 1524.437 |
| save-prepush-junit | `cp /mnt/build/cargo-maos/nextest/default/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/prepush-default-junit.xml` | 0 | 0.218 |
| compare-generated-intent-report | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/intent-lineage.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` | 0 | 0.002 |
| remove-plant-restore-scaffolds | `rm -f /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge_runtime.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/purge.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/adr.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/intent-lineage.restore` | 0 | 0.002 |
| remove-private-t0-probe | `rm -rf /tmp/maos-20-10-producer-zuwvqmpz` | 0 | 0.867 |
| local-toolchain | `rustc -Vv` | 0 | 0.273 |
| check-dev-record-completeness-final-record | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | 0 | 0.215 |
| check-epic-close-coherence-final-record | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | 0 | 0.032 |
| check-review-findings-resolved-final-record | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | 0 | 0.064 |
| implementation-tracked-diff | `git diff HEAD -- /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/maos_uninstall_16_4.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_supervision_16_3.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/.github/workflows/discipline.yml /home/lunarpulse/dev_ws/xtask/tests/epic17_exit_workflow_17_7.rs /home/lunarpulse/dev_ws/xtask/kloc.toml /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/docs/maos.dev/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/run-maos.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/tests/coverage-matrix.yaml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/deferred-work.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal.md` | 0 | 0.032 |
