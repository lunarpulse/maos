# 20-10 landing 1 — review correction command ledger

Working directory: `/home/lunarpulse/dev_ws`. Every row explicitly removes `CI` and `GITHUB_ACTIONS` from the child environment; other environment is inherited, including cargo target configuration (no override/clean). Paths in command arguments are absolute where a path argument is required. Commands are serialized. `environment_overrides` in adjacent `commands.json` records any extra variables.

Red-before and mutation exits 100 are expected assertion failures. `focused-review` exit 100 exposed the existing top-level-root error-substring compatibility assertion; production error text was repaired, then `focused-final` passed 147 tests. Measurement exits 1 before re-pin are exact-pin mismatches, not ceiling violations; measured bin 34188 and aggregate 192516 remained within their 34548/192876 ceilings. Prepush exit 0 contains an unretried advisory budget exit 1, not a claim that all internal legs were green.

| Name | Exact argv | Extra environment | Exit | Wall seconds |
|---|---|---|---:|---:|
| review-red-before | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --test purge_residue_20_10 --lib --profile ci --test-threads 1 --no-fail-fast -E 'test(review_) \| test(purge_review_)'` | — | 100 | 59.672 |
| fmt-review | `cargo fmt --all` | — | 0 | 2.97 |
| kloc-review-measure | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 1 | 0.115 |
| focused-review | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_supervision_16_3 --test worker_terminal_facts_17_7 --lib --profile ci --test-threads 1 --no-fail-fast` | — | 100 | 133.224 |
| fmt-review-final | `cargo fmt --all` | — | 0 | 2.718 |
| kloc-review-final-measure | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 1 | 0.064 |
| save-purge | `cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore` | — | 0 | 0.004 |
| save-purge_runtime | `cp /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore` | — | 0 | 0.001 |
| plant-P1-live-cached-root | `cargo nextest run --locked -p maos-bin --test worker_supervision_16_3 --profile ci --test-threads 1 -E 'test(purge_review_p1)'` | — | 100 | 8.328 |
| restore-purge_runtime-1 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.004 |
| cmp-purge_runtime-1 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.002 |
| touch-purge_runtime-1 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.002 |
| plant-P2-shared-slice | `cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(review_p2_shared_slice)'` | — | 100 | 3.018 |
| restore-purge_runtime-2 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| cmp-purge_runtime-2 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| touch-purge_runtime-2 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| plant-P3-linked-leaf-and-ancestor | `cargo nextest run --locked -p maos-bin --lib --profile ci --test-threads 1 -E 'test(ac4_dead_empty_live_creator)'` | — | 100 | 2.718 |
| restore-purge_runtime-3 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| cmp-purge_runtime-3 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| touch-purge_runtime-3 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| plant-P4-container-before-workspace | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac1_owned_cid)'` | — | 100 | 6.775 |
| restore-purge-4 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| cmp-purge-4 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| touch-purge-4 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| plant-P5-retained-report-receipt | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac5_only_absent_cid)'` | — | 100 | 6.424 |
| restore-purge_runtime-5 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| cmp-purge_runtime-5 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| touch-purge_runtime-5 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| plant-F1-residue-exit | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(review_f1)'` | — | 100 | 10.231 |
| restore-purge-6 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.004 |
| cmp-purge-6 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| touch-purge-6 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| plant-F2-legacy-ancestor | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(review_f2)'` | — | 100 | 9.779 |
| restore-purge-7 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.003 |
| cmp-purge-7 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| touch-purge-7 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| plant-F3-home-boundary | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(review_f3)'` | — | 100 | 10.431 |
| restore-purge-8 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.004 |
| cmp-purge-8 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.001 |
| touch-purge-8 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs` | — | 0 | 0.002 |
| plant-AC6-ambient-runner-in-module | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --profile ci --test-threads 1 -E 'test(ac6_)'` | — | 100 | 7.125 |
| restore-purge_runtime-9 | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.004 |
| cmp-purge_runtime-9 | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| touch-purge_runtime-9 | `touch /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs` | — | 0 | 0.001 |
| fmt-after-plants | `cargo fmt --all` | — | 0 | 2.818 |
| kloc-pins-after-plants | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.121 |
| build-all-targets | `cargo build --workspace --all-targets --locked` | — | 0 | 630.195 |
| registered-fixtures | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,worker-cli-fixture,maos-registry-server,maos-spirit` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 1.418 |
| focused-final | `cargo nextest run --locked -p maos-bin --test purge_residue_20_10 --test maos_uninstall_16_4 --test worker_supervision_16_3 --test worker_terminal_facts_17_7 --test worker_workspace_19_3 --lib --profile ci --test-threads 1 --no-fail-fast` | — | 0 | 126.76 |
| host-before-review-serial | `uptime` | — | 0 | 0.004 |
| workspace-review-serial | `cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1` | — | 0 | 1323.145 |
| save-review-serial-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/workspace-review-serial-junit.xml` | — | 0 | 0.004 |
| host-before-review-budget | `uptime` | — | 0 | 0.004 |
| check-test-budget | `/mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/workspace-review-serial-junit.xml --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| workspace-review-default | `cargo nextest run --workspace --locked --no-fail-fast --profile ci` | — | 0 | 45.987 |
| save-review-default-junit | `cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/workspace-review-default-junit.xml` | — | 0 | 0.001 |
| clippy-review-stream | `cargo clippy --workspace --all-targets --locked --message-format=json` | — | 0 | 622.308 |
| clippy-review-ratchet | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/clippy-review.jsonl` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.216 |
| cargo-deny-review | `cargo deny check` | — | 0 | 1.567 |
| kloc-check | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.114 |
| check-kernel-baseline | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.016 |
| check-service-boundary | `/mnt/build/cargo-maos/debug/xtask check-service-boundary --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 1.166 |
| check-env-contract | `/mnt/build/cargo-maos/debug/xtask check-env-contract --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.464 |
| check-serde-error-handling | `/mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.264 |
| check-composition-root-completeness | `/mnt/build/cargo-maos/debug/xtask check-composition-root-completeness --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-j1-loopback-delegation | `/mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-dev-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.114 |
| check-epic-close-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| check-review-findings-resolved | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-test-cost | `/mnt/build/cargo-maos/debug/xtask check-test-cost --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.364 |
| invariant-lock | `/mnt/build/cargo-maos/debug/xtask invariant-lock --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| coverage-matrix | `/mnt/build/cargo-maos/debug/xtask coverage-matrix --json --dry-run` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.016 |
| check-coverage-matrix-completeness | `/mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.015 |
| check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.364 |
| check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.007 |
| check-ff-j6 | `/mnt/build/cargo-maos/debug/xtask check-ff-j6 --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| check-perf-runner-policy | `/mnt/build/cargo-maos/debug/xtask check-perf-runner-policy --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| save-review-intent-report | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/intent-lineage.restore` | — | 0 | 0.009 |
| prepush-review | `/mnt/build/cargo-maos/debug/xtask prepush --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask; MAOS_17_2_CGROUP_MODE=undelegated | 0 | 1354.889 |
| save-review-prepush-junit | `cp /mnt/build/cargo-maos/nextest/default/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/prepush-review-junit.xml` | — | 0 | 0.011 |
| compare-review-intent-report | `cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/intent-lineage.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` | — | 0 | 0.002 |
| review-record-date | `date --iso-8601=seconds` | — | 0 | 0.006 |
| remove-review-restore-scaffolds | `rm -f /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge.rs.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/purge_runtime.rs.restore /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/intent-lineage.restore` | — | 0 | 0.002 |
| check-dev-record-completeness-review-final-record | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.165 |
| check-epic-close-coherence-review-final-record | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.032 |
| check-review-findings-resolved-review-final-record | `/mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json` | CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask | 0 | 0.064 |
| implementation-tracked-diff | `git diff ac1fc12a7f95c4ab003f35d5ca479c181c0f22d2 -- /home/lunarpulse/dev_ws/crates/maos-bin/src/egress.rs /home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs /home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/maos_uninstall_16_4.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/purge_residue_20_10.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_supervision_16_3.rs /home/lunarpulse/dev_ws/crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs /home/lunarpulse/dev_ws/xtask/kloc.toml /home/lunarpulse/dev_ws/docs/adr/ADR-067-maos-data-root-layout-and-purge.md /home/lunarpulse/dev_ws/docs/maos.dev/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/run-maos.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md /home/lunarpulse/dev_ws/docs-site/docs/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/deferred-work.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal.md /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/plants.md` | — | 0 | 0.032 |
| save-review-implementation-diff | `cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/implementation-tracked-diff.log /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/implementation.diff` | — | 0 | 0.004 |
| review-implementation-diff-sha256 | `sha256sum /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/landing-1/review-1/implementation.diff` | — | 0 | 0.008 |
