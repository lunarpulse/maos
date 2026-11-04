# Hosted-1 exact commands and exits

Baseline: HEAD76efc19d, supplied clean by Main. Every Cargo process guard is retained (exit1 means no matching process). Profiling child commands include their actual exit codes. Failed controls and rejected drafts are retained, not retried to green.

Setup resolver observed exit0 before ledger initialization:
```sh
/usr/bin/python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow
```

## C001 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C002 — exit 101: Reproducing no-network control before correction

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1
```

## C003 — exit 127: Profiling baseline currency audit CPU and memory

```sh
/usr/bin/time -v env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C004 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C005 — exit 0: Measuring unchanged serial currency baseline

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --test model_currency_gate --profile ci --test-threads 1 --no-fail-fast
```

## C006 — exit 0: Saving fresh serial currency baseline JUnit

```sh
cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/currency-before-junit.xml
```

## C007 — exit 0: CPU profile before

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C008 — exit 0: CPU profile before

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C009 — exit 0: CPU profile before

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C010 — exit 0: CPU profile before

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C011 — exit 0: CPU profile before

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C012 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C013 — exit 101: Proving whole-file AST work before scanner correction

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --test model_currency_gate shared_default_scan_skips_unrelated_item_bodies_hosted_1 -- --exact --test-threads=1
```

## C014 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C015 — exit 0: Formatting feature guards and root-only scan

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --all
```

## C016 — exit 1: Pricing hotfix within inherited source envelopes

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```

## C017 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C018 — exit 0: Checking no-network control after feature guards

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo test --manifest-path /home/lunarpulse/dev_ws/Cargo.toml -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1
```

## C019 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C020 — exit 101: Checking all maos-bin targets without network defaults

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo check --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --no-default-features --all-targets
```

Settlement: `{"job": "bg_288", "exit": 101, "artifact": "13421", "seconds": 352.27}`.

## C021 — exit 0: Recording hosted hotfix date and host load

```sh
date -u && uptime
```

## C022 — exit 0: Inspecting assigned original feature-symbol diff

```sh
git diff 73e7afbc^ 73e7afbc -- crates/maos-bin/src/main.rs crates/maos-bin/src/lib.rs crates/maos-bin/tests/provider_selection_20_4.rs crates/maos-bin/tests/provider_endpoint_pin_20_4.rs crates/maos-bin/tests/support/provider204.rs
```

## C023 — exit 0: Inspecting complete L1 network-boundary additions

```sh
git diff 8c809f9c 73e7afbc -- crates/maos-bin/src crates/maos-bin/tests/provider_selection_20_4.rs crates/maos-bin/tests/provider_endpoint_pin_20_4.rs crates/maos-bin/tests/support/provider204.rs
```

## C024 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C025 — exit 0: Formatting final opaque item scan

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --all
```

## C026 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C027 — exit 0: Checking all refusal vectors and opaque-body regression serially

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --test model_currency_gate --profile ci --test-threads 1 --no-fail-fast
```

Settlement: `{"job": "bg_289", "exit": 0, "artifact": "13432", "seconds": 194.97}`.

## C028 — exit 0: Saving fresh serial optimized currency JUnit

```sh
cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/currency-after-junit.xml
```

## C029 — exit 0: Premature after profile of pre-rebuild executable

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

Rejected measurement: Captured before background compilation settled.

## C030 — exit 0: Premature after profile of pre-rebuild executable

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

Rejected measurement: Captured before background compilation settled.

## C031 — exit 0: Premature after profile of pre-rebuild executable

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

Rejected measurement: Captured before background compilation settled.

## C032 — exit 0: Premature after profile of pre-rebuild executable

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

Rejected measurement: Captured before background compilation settled.

## C033 — exit 0: Premature after profile of pre-rebuild executable

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

Rejected measurement: Captured before background compilation settled.

## C034 — exit 0: Saving completed twenty-six-test serial JUnit

```sh
cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/currency-after-junit.xml
```

## C035 — exit 0: CPU profile intermediate token-only scan

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C036 — exit 0: CPU profile intermediate token-only scan

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C037 — exit 0: CPU profile intermediate token-only scan

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C038 — exit 0: CPU profile intermediate token-only scan

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C039 — exit 0: CPU profile intermediate token-only scan

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C040 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C041 — exit 0: Formatting cheap constant-prefix parser

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --all
```

## C042 — exit 0: Pricing final constant-prefix scanner

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```

## C043 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C044 — exit 0: Measuring final prefix-filtered refusal suite serially

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --test model_currency_gate --profile ci --test-threads 1 --no-fail-fast
```

Settlement: `{"job": "bg_290", "exit": 0, "artifact": "13441", "seconds": 234.17}`.

## C045 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C046 — exit 0: Formatting fully opaque brace-group scanning

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo fmt --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --all
```

## C047 — exit 1: Measuring final opaque-body scan footprint

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```

## C048 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C049 — exit 0: Building isolated audit profiler executable

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo build --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --bin xtask
```

Settlement: `{"job": "bg_291", "exit": 0, "artifact": "13449", "seconds": 255.8}`.

## C050 — exit 0: CPU profile after

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C051 — exit 0: CPU profile after

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C052 — exit 0: CPU profile after

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C053 — exit 0: CPU profile after

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C054 — exit 0: CPU profile after

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C055 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C056 — exit 0: Checking final opaque-body scanner and all original vectors serially

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p xtask --test model_currency_gate --profile ci --test-threads 1 --no-fail-fast
```

Settlement: `{"job": "bg_292", "exit": 0, "artifact": "13453", "seconds": 71.99}`.

## C057 — exit 0: Preserving final twenty-six-test serial timings

```sh
cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/currency-after-junit.xml
```

## C058 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C059 — exit 0: Checking workflow network feature targets

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo check --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --all-targets --features network
```

Settlement: `{"job": "bg_293", "exit": 0, "artifact": "13458", "seconds": 259.49}`.

## C060 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C061 — exit 0: Checking workflow fixture-replay feature targets

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo check --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --all-targets --features fixture_replay
```

Settlement: `{"job": "bg_294", "exit": 0, "artifact": "13462", "seconds": 245.86}`.

## C062 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C063 — exit 0: Checking workflow network and wasm-host feature targets

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo check --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --all-targets --features network,wasm-host
```

Settlement: `{"job": "bg_295", "exit": 0, "artifact": "13467", "seconds": 663.19}`.

## C064 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C065 — exit 0: Running default feature control and network catalog unit serially

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --bin maos --test verb_table_15_3 --profile ci --test-threads 1 --no-fail-fast -E 'binary(verb_table_15_3) | test(catalog_endpoint_matches_driver_endpoint_f6)'
```

Settlement: `{"job": "bg_296", "exit": 0, "artifact": "13471", "seconds": 405.48}`.

## C066 — exit 0: Retaining focused network and verb control JUnit

```sh
cp /mnt/build/cargo-maos/nextest/ci/junit.xml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/root-control-junit.xml
```

## C067 — exit 1: Cargo process guard

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C068 — exit 0: Capturing non-denying full workspace clippy delta

```sh
set -o pipefail && env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo clippy --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked --workspace --all-targets --message-format=json | tee /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/clippy.jsonl
```

Settlement: `{"job": "bg_297", "exit": 0, "artifact": "13475", "seconds": 1031.91}`.

## C069 — exit 0: Checking final warning counts against the existing ratchet

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/clippy.jsonl --baseline /home/lunarpulse/dev_ws/xtask/clippy-baseline.toml
```

## C070 — exit 0: Checking hotfix kloc-check

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask kloc-check --json
```

## C071 — exit 0: Checking hotfix check-kernel-baseline

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json
```

## C072 — exit 0: Checking hotfix check-service-boundary

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-service-boundary --json
```

## C073 — exit 0: Checking hotfix check-env-contract

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-env-contract --json
```

## C074 — exit 0: Checking hotfix check-serde-error-handling

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json
```

## C075 — exit 0: Checking hotfix check-composition-root-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-composition-root-completeness --json
```

## C076 — exit 0: Checking hotfix check-j1-loopback-delegation

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-j1-loopback-delegation --json
```

## C077 — exit 0: Checking hotfix check-epic-close-coherence

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json
```

## C078 — exit 0: Checking hotfix check-review-findings-resolved

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-review-findings-resolved --json
```

## C079 — exit 0: Checking hotfix check-test-cost

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-test-cost --json
```

## C080 — exit 0: Checking hotfix invariant-lock

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask invariant-lock --json
```

## C081 — exit 0: Checking hotfix coverage-matrix

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask coverage-matrix --json
```

## C082 — exit 0: Checking hotfix check-coverage-matrix-completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-coverage-matrix-completeness --json
```

## C083 — exit 0: Checking hotfix check-exit-commands

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-exit-commands --json
```

## C084 — exit 0: Checking hotfix check-decision-register

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-decision-register --json
```

## C085 — exit 0: Checking hotfix check-ff-j6

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-ff-j6 --json
```

## C086 — exit 0: Checking hotfix check-perf-runner-policy

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-perf-runner-policy --json
```

## C087 — exit 0: Checking hotfix check-model-currency

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-model-currency --json
```

## C088 — exit 0: Checking hosted-1 Dev Agent Record completeness

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json
```

## C089 — exit 0: Measuring storage immediately before final hotfix cleanup

```sh
du -s -B1 /mnt/build/cargo-maos /tmp
```

Settlement: `{"job": "bg_298", "exit": 0, "seconds": 170.98, "inline_observed_output": "222600966144\t/mnt/build/cargo-maos\n1453056000\t/tmp\n"}`.

## C090 — exit 1: Checking no compiler before authorized cleanup

```sh
ps -eo args | grep -E 'cargo|rustc|nextest|clippy' | grep -v grep
```

## C091 — exit 0: Measuring storage after bounded hotfix cleanup

```sh
du -s -B1 /mnt/build/cargo-maos /tmp
```

Settlement: `{"job": "bg_299", "exit": 0, "seconds": 95.36, "inline_observed_output": "221768646656\t/mnt/build/cargo-maos\n1453056000\t/tmp\n"}`.

## C092 — exit 0: Recording final host load alongside timing evidence

```sh
/usr/bin/uptime
```

