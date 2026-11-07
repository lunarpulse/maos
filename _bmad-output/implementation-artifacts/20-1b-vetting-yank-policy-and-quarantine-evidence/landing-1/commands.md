# 20-1b L1 — exact observed command ledger

Commands below are argv actually executed, not suggested commands. Normal run_record invocations remove CI/GITHUB_ACTIONS and set CARGO_MANIFEST_DIR to the repository xtask directory; the workspace sequence additionally sets MAOS_17_2_CGROUP_MODE=undelegated. Target resolution is inherited, never overridden by this agent. Prepush argv explicitly includes the prescribed env command, with declared Python requirements supplied by the recorded temporary venv/PATH prerequisite. No test failure is retried-to-green.

Detailed output is in each labelled stdout/stderr (or the explicitly named prepush JSON/JUnit); JSON command records are authoritative for annotations. A supplementary asynchronous fixture delivery exposed no numeric exit code: it is marked unknown rather than fabricated, and a later captured final fixture builder exits0. Metadata startup refusals execute zero tests; the predecessor-report budget attribution is explicitly invalid and excluded from this landing's outcome.

## Implementation / focused / structural commands

| Label | Exact argv | Exit | Seconds |
|---|---|---:|---:|
| baseline-kloc | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 0 | 0.091 |
| baseline-kernel | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | 0 | 0.011 |
| ac14-ac16-before | `cargo test --locked -p maos-cli --test spirit_install_20_1 ac1 -- --nocapture` | 101 | 14.161 |
| ac6-before | `cargo test --locked -p maos-registry --test vetting_attestation_gate artifact_and_publisher_swaps_are_refused_ac6 -- --nocapture` | 101 | 22.275 |
| ac6-both-before | `cargo test --locked -p maos-registry --test vetting_attestation_gate swap_is_refused_ac6 -- --nocapture` | 101 | 2.137 |
| affected-build-initial | `cargo build --locked -p maos-bin -p maos-cli -p maos-spirit-cli -p maos-registry` | 0 | 710.35 |
| resume-baseline-kloc | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.184 |
| focused-initial | `cargo test --locked -p maos-registry -p maos-compliance -p maos-spirit-cli --all-targets` | 101 | 488.231 |
| policy-initial | `cargo test --locked -p maos-bin --test yank_policy_20_1b -- --nocapture --test-threads=1` | 101 | 1139.199 |
| auth-initial | `cargo test --locked -p maos-registry --test yank_auth_20_1b -- --nocapture` | 0 | 202.591 |
| policy-corrected | `cargo test --locked -p maos-bin --test yank_policy_20_1b -- --nocapture --test-threads=1` | 101 | 134.597 |
| registry-focused-corrected | `cargo test --locked -p maos-registry --all-targets --no-fail-fast` | 101 | 108.546 |
| format-first | `cargo fmt --all` | 0 | 3.24 |
| policy-labelled-corrected | `cargo test --locked -p maos-bin --test yank_policy_20_1b -- --nocapture --test-threads=1` | 101 | 765.444 |
| fixtures-first | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures --only maosctl,maos-spirit,maos-registry-server` | 0 | 814.292 |
| policy-shadow-repaired | `cargo test --locked -p maos-bin --test yank_policy_20_1b -- --nocapture --test-threads=1` | 0 | 258.462 |
| registry-focused-next | `cargo test --locked -p maos-registry --all-targets --no-fail-fast` | 101 | 164.629 |
| install-focused-next | `cargo test --locked -p maos-cli --test spirit_install_20_1 -- --nocapture` | 0 | 102.517 |
| vet-lifecycle-first | `cargo test --locked -p maos-spirit-cli --test vet_lifecycle_20_1b -- --nocapture` | 101 | 30.85 |
| format-second | `cargo fmt --all` | 0 | 3.216 |
| formatted-measurement | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.093 |
| kernel-after-format | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | 0 | 0.012 |
| trust-codec-repaired | `cargo test --locked -p maos-registry --test yank_auth_20_1b -- --nocapture` | 0 | 43.128 |
| vet-crl-codec-repaired | `cargo test --locked -p maos-spirit-cli --test vet_lifecycle_20_1b -- --nocapture` | 101 | 16.871 |
| ac6-plant-diff | `diff -u /tmp/201b-plants/ac6.rs crates/maos-compliance/src/vetting/mod.rs` | 1 | 0.008 |
| ac6-plant | `cargo test --locked -p maos-registry --test vetting_attestation_gate artifact_swap_is_refused_ac6 -- --exact --nocapture` | 101 | 20.639 |
| ac6-restore | `bash -c 'cp /tmp/201b-plants/ac6.rs crates/maos-compliance/src/vetting/mod.rs && cmp /tmp/201b-plants/ac6.rs crates/maos-compliance/src/vetting/mod.rs && touch crates/maos-compliance/src/vetting/mod.rs'` | 0 | 0.009 |
| ac7-plant-diff | `diff -u /tmp/201b-plants/ac7.rs crates/maos-registry/src/handlers/deprecate.rs` | 1 | 0.004 |
| ac7-plant | `cargo test --locked -p maos-registry --test yank_auth_20_1b unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17 -- --exact --nocapture` | 101 | 7.894 |
| ac7-restore | `bash -c 'cp /tmp/201b-plants/ac7.rs crates/maos-registry/src/handlers/deprecate.rs && cmp /tmp/201b-plants/ac7.rs crates/maos-registry/src/handlers/deprecate.rs && touch crates/maos-registry/src/handlers/deprecate.rs'` | 0 | 0.006 |
| ac8-plant-diff | `diff -u /tmp/201b-plants/ac8.rs crates/maos-registry/src/yank.rs` | 1 | 0.005 |
| ac8-plant | `cargo test --locked -p maos-bin --test yank_policy_20_1b signed_yank_decisions_warn_request_quarantine_and_apply_crl_ac8 -- --exact --nocapture` | 101 | 117.867 |
| ac8-restore | `bash -c 'cp /tmp/201b-plants/ac8.rs crates/maos-registry/src/yank.rs && cmp /tmp/201b-plants/ac8.rs crates/maos-registry/src/yank.rs && touch crates/maos-registry/src/yank.rs'` | 0 | 0.016 |
| ac9-plant-diff | `diff -u /tmp/201b-plants/ac9.rs crates/maos-bin/src/yank_policy.rs` | 1 | 0.001 |
| ac9-plant | `cargo test --locked -p maos-bin --test yank_policy_20_1b expiry_and_vetter_revocation_observe_once_and_request_policy_ac9 -- --exact --nocapture` | 101 | 56.304 |
| ac9-restore | `bash -c 'cp /tmp/201b-plants/ac9.rs crates/maos-bin/src/yank_policy.rs && cmp /tmp/201b-plants/ac9.rs crates/maos-bin/src/yank_policy.rs && touch crates/maos-bin/src/yank_policy.rs'` | 0 | 0.024 |
| ac10-plant-diff | `diff -u /tmp/201b-plants/ac10.rs crates/maos-bin/src/admission.rs` | 1 | 0.003 |
| ac10-plant | `cargo test --locked -p maos-bin --test yank_policy_20_1b registry_publishers_cannot_share_principal_spill_ac10 -- --exact --nocapture` | 101 | 24.582 |
| ac10-restore | `bash -c 'cp /tmp/201b-plants/ac10.rs crates/maos-bin/src/admission.rs && cmp /tmp/201b-plants/ac10.rs crates/maos-bin/src/admission.rs && touch crates/maos-bin/src/admission.rs'` | 0 | 0.007 |
| ac14-plant-diff | `diff -u /tmp/201b-plants/ac14.rs crates/maos-registry/src/storage.rs` | 1 | 0.002 |
| ac14-plant | `cargo test --locked -p maos-registry --test yank_auth_20_1b installed_replacement_and_yanked_install_are_refused_before_commit_ac14_ac16 -- --exact --nocapture` | 101 | 7.818 |
| ac14-restore | `bash -c 'cp /tmp/201b-plants/ac14.rs crates/maos-registry/src/storage.rs && cmp /tmp/201b-plants/ac14.rs crates/maos-registry/src/storage.rs && touch crates/maos-registry/src/storage.rs'` | 0 | 0.005 |
| ac16-plant-diff | `diff -u /tmp/201b-plants/ac16.rs crates/maos-registry/src/install.rs` | 1 | 0.001 |
| ac16-plant | `cargo test --locked -p maos-registry --test yank_auth_20_1b installed_replacement_and_yanked_install_are_refused_before_commit_ac14_ac16 -- --exact --nocapture` | 101 | 7.145 |
| ac16-restore | `bash -c 'cp /tmp/201b-plants/ac16.rs crates/maos-registry/src/install.rs && cmp /tmp/201b-plants/ac16.rs crates/maos-registry/src/install.rs && touch crates/maos-registry/src/install.rs'` | 0 | 0.006 |
| ac17-plant-diff | `diff -u /tmp/201b-plants/ac17.rs crates/maos-registry/src/handlers/publish.rs` | 1 | 0.001 |
| ac17-plant | `cargo test --locked -p maos-registry --test yank_auth_20_1b unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17 -- --exact --nocapture` | 101 | 7.832 |
| ac17-restore | `bash -c 'cp /tmp/201b-plants/ac17.rs crates/maos-registry/src/handlers/publish.rs && cmp /tmp/201b-plants/ac17.rs crates/maos-registry/src/handlers/publish.rs && touch crates/maos-registry/src/handlers/publish.rs'` | 0 | 0.009 |
| format-pre-clippy | `cargo fmt --all` | 0 | 4.259 |
| clippy-live | `cargo clippy --locked --workspace --all-targets --message-format=json` | 0 | 1410.169 |
| clippy-ratchet | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input _bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/clippy-live.stdout` | 0 | 0.239 |
| format-final | `cargo fmt --all` | 0 | 3.229 |
| formatted-final-measurement | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.128 |
| structural-check-service-boundary | `/mnt/build/cargo-maos/debug/xtask check-service-boundary --json` | 0 | 1.168 |
| structural-check-pub-field-constructors | `/mnt/build/cargo-maos/debug/xtask check-pub-field-constructors --json` | 0 | 0.257 |
| structural-check-dependency-closure | `/mnt/build/cargo-maos/debug/xtask check-dependency-closure --json` | 0 | 0.87 |
| structural-check-env-contract | `/mnt/build/cargo-maos/debug/xtask check-env-contract --json` | 0 | 0.486 |
| structural-check-serde-error-handling | `/mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` | 0 | 0.244 |
| structural-check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | 0 | 0.55 |
| structural-check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | 0 | 0.007 |
| inactive-generation-before | `cargo test --locked -p maos-registry --test yank_auth_20_1b installed_replacement_and_yanked_install_are_refused_before_commit_ac14_ac16 -- --exact` | 101 | 21.67 |
| fixtures-background | `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask build-test-fixtures` | not exposed | 155.54 |
| format-generation-repair | `cargo fmt --all` | 0 | 3.241 |
| fixtures-generation-repair | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures` | 0 | 43.593 |
| genuine-journey-build | `cargo build --locked -p maos-bin -p maos-cli` | 0 | 89.408 |
| registry-trust-final | `cargo test --locked -p maos-registry --all-targets` | 0 | 65.023 |
| vet-lifecycle-final | `cargo test --locked -p maos-spirit-cli --test vet_lifecycle_20_1b` | 0 | 4.406 |
| policy-final | `cargo test --locked -p maos-bin --test yank_policy_20_1b` | 0 | 68.376 |
| install-final | `cargo test --locked -p maos-cli --test spirit_install_20_1` | 0 | 3.472 |
| compliance-final | `cargo test --locked -p maos-compliance --all-targets` | 0 | 15.661 |
| domain-final | `cargo test --locked -p maos-domain --all-targets` | 0 | 16.194 |
| networkless-verbs-final | `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3` | 0 | 40.526 |
| formatted-generation-measurement | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 1 | 0.093 |
| head-before-final-gates | `git log --oneline -5` | 0 | 0.007 |
| clippy-final | `cargo clippy --locked --workspace --all-targets --message-format=json` | 0 | 1048.673 |
| clippy-ratchet-final | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/clippy-final.stdout --json` | 2 | 0.048 |
| xtask-final-build | `cargo build --locked -p xtask` | 0 | 115.003 |
| pin-provenance-final | `cargo test --locked -p xtask --test d11_xtask_ceiling_ratchet --test check_test_budget_proven_red` | 0 | 10.161 |
| final-kloc-check | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 0 | 0.047 |
| final-check-kernel-baseline | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | 0 | 0.01 |
| final-check-service-boundary | `/mnt/build/cargo-maos/debug/xtask check-service-boundary --json` | 0 | 1.153 |
| final-check-pub-field-constructors | `/mnt/build/cargo-maos/debug/xtask check-pub-field-constructors --json` | 0 | 0.251 |
| final-check-dependency-closure | `/mnt/build/cargo-maos/debug/xtask check-dependency-closure --json` | 0 | 0.453 |
| final-check-env-contract | `/mnt/build/cargo-maos/debug/xtask check-env-contract --json` | 0 | 0.457 |
| final-check-serde-error-handling | `/mnt/build/cargo-maos/debug/xtask check-serde-error-handling --json` | 0 | 0.238 |
| final-check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | 0 | 0.49 |
| final-check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | 0 | 0.006 |
| final-check-test-cost | `/mnt/build/cargo-maos/debug/xtask check-test-cost --json` | 0 | 0.334 |
| final-check-epic-close-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | 0 | 0.021 |
| final-check-dev-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | 0 | 0.065 |
| clippy-ratchet-final-corrected | `/mnt/build/cargo-maos/debug/xtask clippy-ratchet --input /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/clippy-final.stdout` | 0 | 0.24 |
| citation-revisions | `git rev-parse HEAD ebf2ab03` | 0 | 0.089 |
| workspace-launcher-unavailable-python | `python /tmp/201b-workspace-run.py` | 127 | not exposed |
| landing-tracked-file-list | `git diff --name-only` | 0 | 0.038 |
| landing-new-file-list | `git ls-files --others --exclude-standard` | 0 | 0.019 |
| nextest-list-help | `cargo nextest list --help` | 0 | 0.048 |
| lineage-restore-baseline | `git show bdfc143041613473e7a5f564de8fe30af7c2b0f7:_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` | 0 | 0.007 |
| verifier-venv-setup | `/usr/bin/python3 -m venv /tmp/201b-verifier-venv` | 0 | 1.147 |
| verifier-declared-dependencies | `/tmp/201b-verifier-venv/bin/python3 -m pip install --no-cache-dir --only-binary=:all: --require-hashes -r tools/verify-erasure/requirements.txt` | 0 | 0.842 |
| fkcs-admission-review-diff | `git diff -- crates/maos-registry/src/admission.rs crates/maos-skill/src/admission.rs` | 0 | 0.004 |
| verifier-venv-preserving-system-tools | `/usr/bin/python3 -m venv --system-site-packages /tmp/201b-verifier-venv` | 0 | 1.118 |
| fkcs-oracle-repin | `cargo test --locked -p xtask --test fkcs_oracle` | 0 | 28.462 |
| fkcs-gate-repin | `/mnt/build/cargo-maos/debug/xtask check-fkcs --json` | 0 | 36.851 |
| kloc-with-fkcs-pin | `/mnt/build/cargo-maos/debug/xtask kloc-check --json` | 0 | 0.161 |
| kernel-with-fkcs-pin | `/mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` | 0 | 0.03 |
| landing-tracked-file-list-final | `git diff --name-only` | 0 | 0.072 |
| head-before-prepush | `git log --oneline -5` | 0 | 0.005 |
| prepush-ready-check-dev-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | 0 | 0.141 |
| prepush-ready-check-epic-close-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | 0 | 0.024 |
| prepush-ready-check-exit-commands | `/mnt/build/cargo-maos/debug/xtask check-exit-commands --json` | 0 | 0.336 |
| prepush-ready-check-decision-register | `/mnt/build/cargo-maos/debug/xtask check-decision-register --json` | 0 | 0.008 |
| prepush-once | `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json` | 1 | 908.019 |
| handoff-record-completeness | `/mnt/build/cargo-maos/debug/xtask check-dev-record-completeness --json` | 0 | 0.185 |
| handoff-epic-coherence | `/mnt/build/cargo-maos/debug/xtask check-epic-close-coherence --json` | 0 | 0.022 |

## Separate workspace sequence

Exactly one standalone invocation executes test cases. The exit2 CLI-selection refusal and exit96 metadata-schema refusal run zero tests. The actual run is workspace-nextest-execution, exit100 (5716/5720 passed); its own fresh budget passes.

| Label | Exact argv | Exit | Seconds |
|---|---|---:|---:|
| kernel-lib-final | `cargo test --locked -p maos-kernel-core --lib` | 0 | 85.429 |
| genuine-journey-before-workspace | `cargo build --locked -p maos-bin -p maos-cli` | 0 | 1.424 |
| workspace-cargo-metadata | `cargo metadata --locked --format-version 1` | 0 | 0.434 |
| workspace-binaries-metadata | `cargo nextest list --locked --workspace --message-format json` | 0 | 810.876 |
| workspace-nextest-once | `cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1 --cargo-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-cargo-metadata.stdout --binaries-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-binaries-metadata.stdout` | 2 | 0.124 |
| workspace-test-budget | `/mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-junit.xml --json` | 1 | 0.052 |
| workspace-nextest-final | `cargo nextest run --no-fail-fast --profile ci --test-threads 1 --cargo-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-cargo-metadata.stdout --binaries-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-binaries-metadata.stdout` | 96 | 0.049 |
| format-final-cursor-comment | `cargo fmt --all` | 0 | 2.908 |
| fixtures-final-source | `/mnt/build/cargo-maos/debug/xtask build-test-fixtures` | 0 | 24.775 |
| genuine-journey-final-source | `cargo build --locked -p maos-bin -p maos-cli` | 0 | 18.22 |
| workspace-binaries-only-metadata | `cargo nextest list --locked --workspace --list-type binaries-only --message-format json` | 0 | 252.45 |
| workspace-nextest-execution | `cargo nextest run --no-fail-fast --profile ci --test-threads 1 --cargo-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-cargo-metadata.stdout --binaries-metadata /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-binaries-only-metadata.stdout` | 100 | 801.319 |
| workspace-budget-execution | `/mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/workspace-junit.xml --json` | 0 | 0.017 |

## Once-only prescribed prepush

`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json`

Exit1 after908.019s;83 pass/7 policy skip/1 red/1 advisory records. Its own workspace JUnit has5719/5720 passes, solely the unchanged20-11 million pending-operation observation. Six budget advisories are preserved under owner20-7, not widened or retried. All30 landing witnesses pass in both actual runs. The generated lineage report remains byte-identical and tracked-tree passes.

## Custody and cleanup

Main froze tracked-tree/HEAD writes and authorized deleting the parked archive only after verification and copying MANIFEST. park-MANIFEST.txt/park-custody.json retain that history. cleanup.json records the fresh /proc scan, complete specialized directory inventories (working around the reported glob200-result clamp), newest/recent retention and measured byte accounting. No deps/build/fingerprint or protected worktree is pruned.
