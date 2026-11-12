# 20-13 L2 (AC7, KT3) — landing-2 evidence

Base: local HEAD `456793ca` (20-14 L2 review corrections), tree uncommitted. Operator ruling 2026-11-12, ask `kt3`:
the LITERAL KT3 shape (public-struct `crypto` field, `new(policy, crypto)`, `Default` removed) with exactly one
surface-hash refresh. Logs are raw; paths are relative to this directory.

## Surface-hash refresh (the only one)

`docs/ci-baselines/kernel-surface-v0.1-beta.json`, symbol `maos_kernel_core::security::SecurityManagerAdapter` (struct):

- old: `76:bb:bb:10:3f:98:d9:4e:a0:ff:58:74:cc:7a:2a:b2:b0:ad:ad:47:ba:ca:96:4c:49:8c:ec:e5:a5:c4:96:7e`
- new: `60:35:04:56:09:f4:28:36:2a:57:49:6b:70:65:ba:0b:ec:d0:31:54:51:62:35:d1:3f:44:8a:c6:50:73:a3:79`

Before the edit `check-service-boundary` returned exactly one violation (that symbol; `gates/check-service-boundary.before-refresh.json`);
a diff of the 400 baseline items against the current surface shows that one changed hash, no removed and no added
item. After the edit the gate exits 0 (`gates/check-service-boundary.json`). Authority: operator ruling `kt3`
(2026-11-12) and the standing `prepaid` (a) pre-approval (symbol named in this measured grant).

## Witnesses (`crates/maos-kernel-core/tests/t3_admission_crypto_20_13.rs`)

The test builds the adapter through one helper, `adapter_with(policy, crypto)`. The red-before form
(`red/t3_admission_crypto_20_13.head-form.rs.txt`) differs only in that helper: at HEAD the adapter has no provider
parameter, so the helper drops the provider and calls `SecurityManagerAdapter::new(policy)` — the transitional state
where the selected provider is not consulted and admission uses the old inline `ring`.

| Vector | Red before (HEAD) | Plant (inline ring restored in `admit_spirit`) | Final |
|---|---|---|---|
| `explicit_lock_is_refused_when_the_selected_provider_rejects` | FAILED (admitted) | FAILED | pass |
| `explicit_lock_verification_runs_on_the_selected_instance` (counting provider, 1 call) | FAILED (0 calls) | FAILED | pass |
| `environment_lock_uses_the_selected_provider` (ring admits; rejecting refuses; counting 1) | FAILED | FAILED | pass |
| `explicit_lock_valid_under_ring_is_admitted` (control) | pass | pass | pass |
| `bad_signature_and_wrong_trust_anchor_are_refused_by_ring` (control) | pass | pass | pass |
| `check_composition_root_completeness::crypto_tests` (kernel `admit_spirit` ring construction counts 2/2, no exemption) | FAILED (`left: 1`, `right: 2`) | — | pass |
| `check-composition-root-completeness` binary on the plant | — | exit 1: `maos-kernel-core/src/security/mod.rs:363 names RingCryptoProvider outside the selection root` | exit 0 |

Raw logs: `red/kernel-t3-admission-HEAD-red.log` (`3 failed, 2 passed`), `red/gate-unit-HEAD-red.log`,
`plants/p1-inline-ring-restored.log` (`3 failed, 2 passed`), `plants/p1-gate.json`. The plant was applied in the real
tree and restored by copy-back, `cmp` against the pre-plant copy, and `touch`; the following `check-kernel-baseline`
reports the pinned hashes again (`gates/check-kernel-baseline.json`). The explicit-lock and the environment-backed
branches both receive the injected provider; there is no ring fallback. A refused admission journals no `Load`
entry, an admitted one does (the privilege/policy/journal ordering is unchanged: the verification precedes the
`journal_lifecycle` call).

## Measurement (after `cargo fmt --all`)

| Crate | live pin before | measured | draw |
|---|---:|---:|---:|
| maos-kernel-core (tokei) | 21203 | 21202 | −1 |
| maos-kernel-core (physical, `check-kernel-baseline`) | 27043 | 27044 | +1 |
| maos-bin | 38556 | 38563 | **+7 (over the live pin; Main adjudication)** |
| xtask | 45379 | 45355 | −24 |
| maos-capability / domain / iac / cli | 1454 / 10274 / 8771 / 6612 | unchanged | 0 |
| aggregate | 202935 measured (hardfail 202936) | 202917 | −18; hardfail 202918 |

The kernel figure is the creation-priced KT3 (+1 / −1) exactly. The bin +7 is the cost of migrating the four
`main.rs` call sites after `cargo fmt` (production root +3, smoke-arm parameter +2, two test-module roots +1 each);
the story's stale-absolute ceilings are not the live pins, so the live pin 38556 is stated and the L2 draw is +7.
`gates/kloc-check.before-repin.json` records the pre-repin measurement (exit 1, bin over its pin); `gates/kloc-check.json`
the re-pinned pass.

## Gates and suites (observed, exit 0 unless stated)

Gates, `env -u CI -u GITHUB_ACTIONS … --json` (`gates/`): check-dev-model-tier, check-kernel-baseline,
check-service-boundary, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands,
check-decision-register, check-env-contract, check-serde-error-handling, check-fkcs, check-pub-field-constructors,
check-dependency-closure, check-abi-ratification, check-composition-root-completeness, check-empty-kernel, kloc-check,
`stability-matrix --check`, `gen-abi-docs --check`, `invariant-lock --changed-files` (the changed-file list including
the new test). `abi-diff` was NOT run: it refuses a dirty tree.

Suites: `cargo nextest run -p maos-kernel-core --locked` 501 passed, 4 skipped; `cargo test -p maos-kernel-core --lib`
253 passed; `cargo nextest run -p xtask --locked` 1315 passed, 12 skipped (includes `d11_xtask_ceiling_ratchet`,
`recovery_lane_ceiling_rule`, `kernel_pin_content_hash_16_0`, `story_10_4a_ac1_proven_red`,
`service_boundary_integration`); `cargo check --workspace --all-targets --locked` and
`cargo check -p maos-bin -p maos-kernel-core -p maos-escape-detector -p xtask --all-targets --all-features --locked`
no errors (`green/cargo-check-*.log`, pre-existing warnings only).

`maos-bin` + `maos-escape-detector` nextest after `build-test-fixtures` and `cargo build -p maos-bin -p maos-cli`
(`green/maos-bin-escape-detector-nextest.log`): 956 run, 952 passed, 15 skipped, 4 failed: three
`erasure_inventory_20_11` vectors fail only because the scratch environment lacked the python `cryptography`
backend (`ValueError: declared cryptography backend missing`); with a scratch venv
(`tools/verify-erasure/requirements.txt`, `--require-hashes`) on PATH they pass (`green/maos-bin-erasure-rerun-with-venv.log`:
31 run, 30 passed). The fourth,
`erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption`, fails (pending
operation did not finish after the 10 s route budget) on the first run and on the rerun; it is the unchanged known
1M-row timing/I-O oracle owned by `20-7-stable-runner-perf-gates` (`deferred-work.md:3`, rule 10(a)) and touches no
crypto or `SecurityManagerAdapter` path. Not retried to green, no budget widened.

## Not exercised

Worker/T3 container admission (hosted substrate) and the bin-side recording leg of AC7.2; the hosted discipline run;
`abi-diff` (dirty tree). Same-instance routing in the root is compiler-required (the constructor takes the provider) and
read at `main.rs` (`SecurityManagerAdapter::new(.., Arc::clone(&crypto))` next to the registry's
`Arc::clone(&crypto)`).

## Clippy, ratchet, verb table

- `cargo clippy --workspace --all-targets --locked --message-format=json`: exit 0; `clippy-ratchet --input`:
  exit 0, **1331 occurrences / 869 source-distinct** (baseline 1383 / 897, `xtask/clippy-baseline.toml`). The only
  diagnostics on touched kernel/xtask files are the pre-existing `clippy::too_many_arguments` on `admit_spirit`
  (`security/mod.rs:259`, signature unchanged); none of `result_large_err`, `collapsible_if`, `unused_mut`,
  `trim_split_whitespace` on the changes. The raw JSON stream (≈ tens of MB) is not stored.
- `cargo test -p maos-bin --no-default-features --test verb_table_15_3 --locked`: 7 passed
  (`green/verb-table-no-default.log`).
- Closing-commit tests: `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule
  --test kernel_pin_content_hash_16_0`: 46 passed.
