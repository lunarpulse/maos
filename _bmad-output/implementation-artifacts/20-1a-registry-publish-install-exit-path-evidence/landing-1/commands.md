# 20-1a landing 1 — exercised commands and exits

This ledger preserves failed attempts as well as final results. It does not claim hosted execution. CI/GITHUB_ACTIONS were unset for cargo/check execution. Early Python-Eval subprocesses did not export CARGO_TARGET_DIR; subsequent bash captures inherited `/mnt/build/cargo-maos`. No CARGO_TARGET_DIR override or cargo clean was used. Environment columns describe inherited/configured values, not extra argv or a target-dir override.

Final source: workspace build rc0; fresh serial workspace 5526/5526 rc0; serial timing budget rc0; WASM-feature undelegated serial 820/820 rc0; required static gates and deny rc0; real non-denying clippy and ratchet rc0 (1339/875, unchanged1383/897 baseline); prescribed prepush rc0. The standalone unrestricted default run remains rc100 (six timing/drain cases), not retried-to-green. See [default-failures](default-failures.md), [verification-summary](verification-summary.json) and [plants](plants.md).

The first serial timing sample failed four 30 s limits; Main classified it as contention-tainted and authorized exactly one fresh serial sample. Both are retained. Fresh binary sums were 7.282, 21.064, 25.739 and16.380 s, all under30. No budget exceptions were added. The final prepush's default timing advisory remains visible (one-daemon30.541 s, participant-recall40.382 s); this is distinct from the passing required serial gate.

## Exact command records

| Log | Exit | Seconds | Environment | Exact command argv |
|---|---:|---:|---|---|
| ac1-red.log | 101 | 13.007 | `{}` | `/home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-registry --test registry_server_bin_20_1 standalone_server_roundtrip_isolates_root_and_exits_cleanly` |
| ac1-green.log | 0 | 7.005 | `{}` | `/home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-registry --test registry_server_bin_20_1` |
| lockfile.log | 0 | 1.338 | `{}` | `/home/lunarpulse/.cargo/bin/cargo generate-lockfile --offline --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| ac2-red-plant.log | 101 | 44.766 | `{}` | `/home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-spirit-cli --test publish_live_20_1` |
| ac2-restore-copy.log | 0 | 0.012 | `{}` | `/usr/bin/cp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/restore/publish.rs /home/lunarpulse/dev_ws/crates/maos-spirit-cli/src/publish.rs` |
| ac2-restore-cmp.log | 0 | 0.004 | `{}` | `/usr/bin/cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/restore/publish.rs /home/lunarpulse/dev_ws/crates/maos-spirit-cli/src/publish.rs` |
| ac2-restore-touch.log | 0 | 0.004 | `{}` | `/usr/bin/touch /home/lunarpulse/dev_ws/crates/maos-spirit-cli/src/publish.rs` |
| ac3-red.log | 2 | — | `{}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/maosctl spirit install example-spirit-ts@0.1.0 --registry-uri http://127.0.0.1:6789/mcp` |
| ac2-green.log | 101 | 0.909 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-spirit-cli --test publish_live_20_1` |
| ac2-green-final.log | 101 | 16.358 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-spirit-cli --test publish_live_20_1` |
| ac2-green-identity.log | 0 | 6.75 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-spirit-cli --test publish_live_20_1` |
| ac3-green.log | 101 | 24.986 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-cli --test spirit_install_20_1` |
| ac5-red.log | 0 | 374.56 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p xtask --test registry_exit_20_1` |
| ac3-green-floor.log | not observed (cancelled) | 60.521 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-cli --test spirit_install_20_1` |
| fmt.log | 0 | 2.928 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| kloc-before-pins.log | 1 | 0.051 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/env CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| kloc-measured.log | 1 | 0.047 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/env CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| build-all-targets.log | 0 | 2168.909 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --all-targets` |
| ac3-final-green.log | 0 | 12.417 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-cli --test spirit_install_20_1` |
| ac1-plant.log | 101 | 6.471 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-registry --test registry_server_bin_20_1 standalone_server_roundtrip_isolates_root_and_exits_cleanly` |
| ac3-plant.log | 101 | 4.132 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-cli --test spirit_install_20_1 install_refuses_registry_rust_inproc_before_persisting` |
| ac4-plant.log | 101 | 38.257 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --test installed_spirit_dispatch_20_1` |
| ac5-red-before.log | 101 | 14.081 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p xtask --test registry_exit_20_1` |
| fmt-final.log | 0 | 2.858 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| kloc-final-measured.log | 1 | 0.049 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/env CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| build-final-all-targets.log | 0 | 140.459 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --all-targets` |
| focused-nextest.log | 100 | 189.427 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-registry -p maos-spirit-cli -p maos-cli -p maos-bin -p maos-shell -p maos-compliance -p maos-domain -p xtask --no-fail-fast` |
| fmt-test-enrollment.log | 0 | 3.002 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| workspace-nextest-serial.log | 100 | 1103.771 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --no-fail-fast --profile ci --test-threads 1` |
| kloc-check.log | 0 | 0.048 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/env CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| check-kernel-baseline.log | 0 | 0.014 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/env CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` |
| cleanup-snapshots.log | 0 | 0.002 | `{}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/rm /tmp/20-1a-server.snapshot /tmp/20-1a-install.snapshot /tmp/20-1a-shell.snapshot /tmp/20-1a-release.snapshot` |
| rebuild-fixtures.log | 0 | 1597.911 | `{}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask build-test-fixtures` |
| workspace-serial-repaired.log | 0 | 1403.987 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --no-fail-fast --profile ci --test-threads 1` |
| workspace-default.log | 100 | 208.256 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --no-fail-fast` |
| wasm-feature-build.log | 0 | 1375.888 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --features wasm-host --all-targets` |
| wasm-feature-tests.log | 100 | 90.103 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --features wasm-host --no-fail-fast` |
| registry-fixture-replay.log | 0 | 243.892 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-registry -p maos-spirit-cli --features maos-registry/fixture_replay,maos-spirit-cli/fixture_replay --no-fail-fast` |
| kernel-lib.log | 0 | 122.646 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-kernel-core --lib` |
| spirit-boundary-invariants.log | 0 | 108.283 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-kernel-core --test spirit_boundary_invariants` |
| perf-hosted-edit-tests.log | 101 | 7.128 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p xtask --test perf_gates_20_7 --test perf_budgets_20_7 --no-fail-fast` |
| fanout-hosted-edit-tests.log | 0 | 52.424 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --test nfr_perf_8_real_spawn_fanout_20_7 --no-fail-fast` |
| perf-counts-six-binding.log | 0 | 24.585 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p xtask -- check-perf-counts --dir /tmp/v207-tier-c-counts --json` |
| check-exit-commands.log | 0 | 0.325 | `{}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-exit-commands` |
| check-decision-register.log | 0 | 0.006 | `{}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-decision-register` |
| clippy-live.jsonl | 0 | 296.373 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --workspace --all-targets --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --message-format=json` |
| clippy-ratchet-live.log | 0 | 0.185 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/clippy-live.jsonl --baseline /home/lunarpulse/dev_ws/xtask/clippy-baseline.toml` |
| fmt-authorized-fixes.log | 0 | 3.044 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| post-bench-kloc.log | 0 | 0.075 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| post-bench-kernel.log | 0 | 0.013 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` |
| perf-hosted-edits-corrected-selector.log | 0 | 9.365 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p xtask --test perf_gates_20_7 --no-fail-fast` |
| fanout-authorized-lint-fix.log | 0 | 3.657 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --test nfr_perf_8_real_spawn_fanout_20_7 --no-fail-fast` |
| wasm-feature-undelegated-serial.log | 0 | 378.432 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --features wasm-host --no-fail-fast --test-threads 1` |
| all-targets-after-authorized-fixes.log | 0 | 303.717 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --all-targets` |
| clippy-final.jsonl | 0 | 3.576 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --workspace --all-targets --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --message-format=json` |
| clippy-ratchet-final.log | 0 | 0.183 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/clippy-final.jsonl --baseline /home/lunarpulse/dev_ws/xtask/clippy-baseline.toml` |
| final-kloc-check.log | 0 | 0.048 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask kloc-check` |
| final-check-kernel-baseline.log | 0 | 0.012 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-kernel-baseline` |
| final-check-service-boundary.log | 0 | 1.141 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-service-boundary` |
| final-check-env-contract.log | 0 | 0.427 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-env-contract` |
| final-check-serde-error-handling.log | 1 | 0.235 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-serde-error-handling` |
| final-check-composition-root-completeness.log | 0 | 0.04 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-composition-root-completeness` |
| final-check-j1-loopback-delegation.log | 0 | 0.04 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation` |
| final-check-dev-record-completeness.log | 0 | 0.108 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-dev-record-completeness` |
| final-check-epic-close-coherence.log | 0 | 0.021 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-epic-close-coherence` |
| final-check-review-findings-resolved.log | 0 | 0.062 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-review-findings-resolved` |
| final-check-test-cost.log | 0 | 0.322 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-test-cost` |
| final-invariant-lock.log | 0 | 0.032 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask invariant-lock` |
| final-coverage-matrix.log | 0 | 0.015 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask coverage-matrix` |
| final-check-coverage-matrix-completeness.log | 0 | 0.012 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness` |
| final-check-perf-runner-policy.log | 0 | 0.047 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-perf-runner-policy` |
| final-check-dependency-closure.log | 0 | 0.254 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-dependency-closure` |
| final-check-corpus.log | 0 | 0.103 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-corpus` |
| final-check-ship-gate-completeness.log | 0 | 0.03 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-ship-gate-completeness` |
| test-budget-serial.log | 1 | 0.017 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/serial-repaired-junit.xml` |
| cargo-deny.log | 0 | 0.769 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo deny --manifest-path /home/lunarpulse/dev_ws/Cargo.toml check` |
| prepush.log | 1 | 333.288 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask prepush --json` |
| fmt-serde-repair.log | 0 | 2.961 | `{"CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --all --manifest-path /home/lunarpulse/dev_ws/Cargo.toml` |
| kloc-serde-measured.log | 0 | 0.044 | `{"CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| repaired-kloc-check.log | 0 | 0.056 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask kloc-check --json` |
| repaired-check-kernel-baseline.log | 0 | 0.011 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` |
| repaired-check-serde-error-handling.log | 0 | 0.232 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` |
| budget-history-known-slow.log | 0 | 0.012 | `{"CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/git log --format=fuller -p -G 'observer_activity_corpus_17_4\|one_daemon_one_door_16_1\|participant_recall_17_8\|xtask::bin/xtask' -- xtask/test-budgets.toml xtask/src/check_test_budget.rs` |
| repaired-all-targets.log | 0 | 117.585 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --all-targets` |
| prepush-repaired.log | 0 | 446.14 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask prepush --json` |
| host-idle-load.log | 0 | 0.004 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/uptime` |
| host-idle-compilers.log | 1 | 0.003 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /usr/bin/ps -C cargo,rustc,clippy-driver -o pid,etime,args` |
| workspace-serial-idle-final.log | 0 | 619.099 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --workspace --no-fail-fast --profile ci --test-threads 1` |
| test-budget-serial-idle-final.log | 0 | 0.017 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/serial-idle-final-junit.xml --json` |
| clippy-serde-final.jsonl | 0 | 90.65 | `{}` | `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --workspace --all-targets --locked --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --message-format=json` |
| clippy-ratchet-serde-final.log | 0 | 0.162 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos", "MAOS_17_2_CGROUP_MODE": "undelegated"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/clippy-serde-final.jsonl --baseline /home/lunarpulse/dev_ws/xtask/clippy-baseline.toml` |
| completed-record-check.log | 0 | 0.11 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` |
| completed-coherence-check.log | 0 | 0.019 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` |
| completed-pins-check.log | 0 | 0.034 | `{"CARGO_MANIFEST_DIR": "/home/lunarpulse/dev_ws/xtask", "CARGO_TARGET_DIR": "/mnt/build/cargo-maos"}` | `env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask kloc-check --json` |

`host-idle-compilers` exit1 is the expected `ps` no-match result, not a failed test. A cancelled command whose numeric process exit was not observed is explicitly marked instead of inventing a code.

## Prescribed prepush environment

```sh
env -u CI -u GITHUB_ACTIONS \
  CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask \
  MAOS_17_2_CGROUP_MODE=undelegated \
  /mnt/build/cargo-maos/debug/xtask prepush --json
```

Final exit0. The initial exit1 was solely the now-repaired registry serde gate. The final result contains84 pass records,7 skips and1 advisory; it is not an all-unconditional-pass claim.

## Final prepush check records

| Check | Status | Detail |
|---|---|---|
| `changed-paths` | pass | exit 0 |
| `stability-matrix` | pass | exit 0 |
| `error-catalog-check` | pass | exit 0 |
| `gen-abi-docs` | pass | exit 0 |
| `check-serde-error-handling` | pass | exit 0 |
| `check-workspace-count` | pass | exit 0 |
| `check-manifest-schema-version` | pass | exit 0 |
| `check-skill-schema` | pass | exit 0 |
| `check-epic-close-coherence` | pass | exit 0 |
| `check-coverage-matrix-completeness` | pass | exit 0 |
| `check-test-cost` | pass | exit 0 |
| `check-env-contract` | pass | exit 0 |
| `check-unsafe` | pass | exit 0 |
| `check-service-boundary` | pass | exit 0 |
| `check-kernel-baseline` | pass | exit 0 |
| `kloc-check` | pass | exit 0 |
| `check-breaking-md` | pass | exit 0 |
| `check-composition-root-completeness` | pass | exit 0 |
| `check-pub-field-constructors` | pass | exit 0 |
| `check-dependency-closure` | pass | exit 0 |
| `coverage-matrix` | pass | exit 0 |
| `check-corpus` | pass | exit 0 |
| `abi-diff` | pass | exit 0 |
| `check-empty-kernel` | pass | exit 0 |
| `check-loom` | pass | exit 0 |
| `check-fr47` | pass | exit 0 |
| `check-a2a-sender-completeness` | pass | exit 0 |
| `check-judge-config` | pass | exit 0 |
| `check-security-md` | pass | exit 0 |
| `templates-regen` | pass | exit 0 |
| `corpus-staleness` | pass | exit 0 |
| `calibrate` | pass | exit 0 |
| `check-review-findings-resolved` | pass | exit 0 |
| `check-dev-record-completeness` | pass | exit 0 |
| `check-dev-model-tier` | pass | exit 0 |
| `check-ship-gate-completeness` | pass | exit 0 |
| `check-pentest-gate` | pass | exit 0 |
| `check-third-party-trial` | pass | exit 0 |
| `check-trial-attestation` | pass | exit 0 |
| `check-red-team-gate` | pass | exit 0 |
| `check-migration-merkle` | pass | exit 0 |
| `nfr-onb-1-gate` | pass | exit 0 |
| `cassette-age-gate` | pass | exit 0 |
| `build-test-fixtures` | pass | exit 0 |
| `binary-size:trigger` | pass | exit 0 |
| `binary-size:build` | pass | exit 0 |
| `binary-size` | pass | exit 0 |
| `fmt` | pass | exit 0 |
| `docs:troubleshoot-bidi` | pass | exit 0 |
| `docs:glossary-lock` | pass | exit 0 |
| `docs:anchor-ids` | pass | exit 0 |
| `docs:ko-coverage` | pass | exit 0 |
| `docs:build` | skip | no docs-site/**, docs/errors/** or xtask/error-catalog.toml changes |
| `docs:routes` | skip | no docs-site/**, docs/errors/** or xtask/error-catalog.toml changes |
| `cargo-deny` | pass | exit 0 |
| `workspace-nextest` | pass | exit 0 |
| `kernel-core-lib` | pass | exit 0 |
| `workspace-doc` | pass | exit 0 |
| `test-budget` | advisory | expected exit 0, actual exit status: 1; 0 lines omitted; {"gate":"check-test-budget","passed":false,"junit":"/mnt/build/cargo-maos/nextest/default/junit.xml","binaries":591,"total_span_seconds":372.72500920295715,"total_sum_seconds":865.6350000000004,"findings":["maos-bin::one_daemon_one_door_16_1 took 30.541 s (Σ per-test time) over 38 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml","m ::error::check-test-budget: FAIL — 2 finding(s): - maos-bin::one_daemon_one_door_16_1 took 30.541 s (Σ per-test time) over 38 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml - maos-iac::participant_recall_17_8 took 40.382 s (Σ per-test time) over 29 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml check-test-budget: FAIL — 2 finding(s): - maos-bin::one_daemon_one_door_16_1 took 30.541 s (Σ per-test time) over 38 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml - maos-iac::participant_recall_17_8 took 40.382 s (Σ per-test time) over 29 test(s), budget 30.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml |
| `ci-pins` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:marker` | skip | "SCOPED-VERDICT ": 1 static emission sites; pin 8 has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative |
| `ci-pins:marker` | skip | "GOVERNED-ATOMIC ": 1 static emission sites; pin 10 has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative |
| `ci-pins:marker` | skip | "HOSTILE-LIFTING ": 1 static emission sites; pin 2 has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative |
| `ci-pins:marker` | skip | "HOSTILE-LIFTING-CONTROL ": 1 static emission sites; pin 1 has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative |
| `ci-pins:marker` | skip | "RUNNER-SIGSYS ": 1 static emission sites; pin 2 has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative |
| `ci-pins:marker` | pass | "REPEATED-MAILBOX ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-PROVENANCE ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-REFUSALS ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-FLOODS ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-DIAGNOSTICS ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-EXITS ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-ONCE ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "HOSTILE-IAC-SEND ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:marker` | pass | "WASM-IAC-SEND ": expected 1, actual 1 literal emission sites (heuristic, not runtime proof) |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `ci-pins:count` | pass | exit 0 |
| `tracked-tree` | pass | exit 0 |

## Unrun and custody

Hosted `epic-20-exit`, no-server readiness red control, real installed-component execution and T3 require the cut CI handoff and were never pushed/run here. See `../ci/ci-20-1a-verify.yml`; readiness patch SHA-256 is `8cdf9a4ca1acf57083cdbc03f18d065b25d9c1541b8324b8d7f2643785fb33dc`. No author-self-review or hosted post-fix clippy count is claimed. Kernel src/baselines are unchanged; Main authorized only unused-mut removal in an uncharged kernel benchmark. Auth/yank/cursor/key-pin/kernel-poller work remains20-1b. Story status and trackers were not flipped; nothing committed or pushed.
