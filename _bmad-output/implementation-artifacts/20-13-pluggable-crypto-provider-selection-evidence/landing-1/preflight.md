# 20-13 L1 preflight


Rebased at `37baeedf` after 20-8 L1 and its review: bin 35914 + 496 = 36410;
shell 615 + 122 = 737; registry 6151 + 102 = 6253; compliance 2077 + 74
= 2151; domain 10198 + 15 = 10213; xtask 44700 + 238 = 44938; aggregate
198235 + 1045 = 199280. Kernel remains 26841 physical / 21042 tokei. The
older measurements below are retained as dated pre-rebase evidence.

After formatting the restored implementation, `kloc-check --json` measured
aggregate 198693 (bin 36111, shell 673, registry 6194, compliance 2102,
domain 10198, xtask 44835, kernel 21042); exited 1 against the prior pins.
`invariant-lock --changed-files <(git diff --name-only HEAD~1) --json` passed
with no touched invariants.

On the restored `37baeedf` tree, `check-kernel-baseline --json` passed with
unchanged 26,841 lines and file-set hash; `check-env-contract --json` passed
with 122 registered variables and no violations; `check-corpus --json` passed
16 entries. `check-test-cost`, `check-exit-commands`, and
`check-decision-register` also passed (the latter records its existing
unqueryable D19 deadline). These gate invocations used the prebuilt xtask and
unset `CI` and `GITHUB_ACTIONS`.

Baseline `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR="$PWD/xtask" /mnt/build/cargo-maos/debug/xtask kloc-check --json` passed, aggregate 198074, kernel 21042; observed per-crate values agree with the supplied current pins. Applying D02 A to the story's original allowances, not adding them to its old ceilings: bin 35861 + 496 = 36357; shell 615 + 122 = 737; registry 6151 + 102 = 6253; compliance 2077 + 74 = 2151; domain 10190 + 15 = 10205; xtask 44715 + 238 = 44953; aggregate 198074 + 1045 = 199119. Kernel L1 is unchanged (physical 26841 / tokei 21042); the KT3 grant is rebased at L2. These are ceilings, not reservations. Other pending stories share the same crates and must rebase separately.

`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR="$PWD/xtask" /mnt/build/cargo-maos/debug/xtask check-kernel-baseline --json` passed: 26841 physical lines, 95 files, set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`, no changed, added or removed files.

20-4 L1's immutable configured endpoint table is `crates/maos-bin/src/admission.rs` `ProviderCatalog`, captured in `main.rs` before `InstalledSpirits::boot`.

T0 source inventory (`RingCryptoProvider` references under `crates/*/src`): daemon constructions at `maos-bin/src/main.rs:2158,3179,8381` and `worker_spawn.rs:888`; kernel T3 at `maos-kernel-core/src/security/mod.rs:350`; two benchmark harnesses at `maos-bench/src/halt_eval.rs:625` and `harness/j4.rs:154`. The other matches are imports, documentation, the implementation, or test modules. See creation bypass inventory for primitive-level dispositions.

AC1 failing-before: `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test crypto_provider_20_13 init_persists_selected_provider -- --exact --nocapture` exited 101, 258.42 s; `crypto_provider_20_13.rs:19` observed `None` rather than `Some("ring")` after successful `maos init --crypto-provider ring`.

`env -u CI -u GITHUB_ACTIONS cargo check --locked -p maos-bin -p maos-registry -p xtask --all-targets --features maos-bin/crypto-recording` completed successfully in 449.66 s during implementation; files were subsequently edited, so this is not a final-tree check. Output artifact: `artifact://15372`.

Intermediate post-format measurement: `kloc-check --json` exited 1 against old exact pins, as expected before re-pin: aggregate 198532; bin 36058 (+197), shell 673 (+58), registry 6194 (+43), compliance 2102 (+25), domain 10190 (+0), xtask 44850 (+135), kernel 21042 (+0). All measured figures are within the re-based ceilings; this is not a landing measurement.

Fixture builder: `env -u CI -u GITHUB_ACTIONS cargo run --locked -p xtask -- build-test-fixtures` exited 0 in 322.73 s, writing 20 artifacts to `/mnt/build/cargo-maos/maos-fixtures/manifest.json`; output `artifact://15398`. The fixture includes the debug-only recording binary; this does not constitute a hosted result.

After building xtask, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR="$PWD/xtask" /mnt/build/cargo-maos/debug/xtask check-composition-root-completeness --json` passed (0 violations, 11 adapters) in 2.05 s. The same invocation of `check-service-boundary --json` passed in 1.26 s (`artifact://15401`). These are interim results, not final-tree proof.

Focused implementation run: `env -u CI -u GITHUB_ACTIONS cargo build --locked -p maos-bin -p maos-cli && env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test crypto_provider_20_13` exited 100 after 207.27 s: 9 run, 7 passed, 2 failed (`artifact://15405`). `unknown_provider_refused_without_changing_config:59` observed boot exit 1, expected 78; `recording_provider_observes_boot_token_signing:133` found no recording log. This is an incomplete, failing L1, not a landing claim. No plants were run.

After rebase, `env -u CI -u GITHUB_ACTIONS cargo run --locked -p xtask --
build-test-fixtures`, a genuine `cargo build --locked -p maos-bin -p maos-cli`,
then `cargo nextest run --locked -p maos-bin --test crypto_provider_20_13`
(all with CI variables unset) exited 0 in 439.93 s; 9 tests passed, including
the valid-manifest refusal and the real `hello-spirit` token issue. T3 is a
named local skip inside its passing test, not hosted evidence. Subsequently
selection was moved before run argument parsing so even bare `maos run` rejects
the unknown persisted provider with 78; that change and the new at-rest boot
witness require their own final run.

Post-rebase, `cargo test --locked -p maos-bin --bin maos --features
crypto-recording story_13_5a_enterprise_governance_reaches_the_booted_cohort_daemon`,
`cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`,
and `cargo test --locked -p maos-compliance --test ccac_ship_gate_test`, with
CI variables unset, all exited 0: respectively 1, 7 and 1 tests passed. The
first boots the cohort runtime and observed token signing and the real at-rest
seal through the same selected recording provider. Combined wall 88.73 s.
AC3 plant: inserted `let _c = maos_kernel_core::security::crypto::RingCryptoProvider;`
at `main.rs:1978`; `check-composition-root-completeness --json` exited 1 in
1.98 s with exactly that violation. Restored using copy-back, `cmp`, and
`touch`; all three commands exited 0.

AC4 crypto-before-equivalent plant: passed `fields.crypto_provider` instead of
the selected provider at `maos-registry/src/install.rs`; `cargo test --locked
-p maos-bin --test crypto_provider_20_13
installed_claims_compare_actual_crypto_and_endpoint -- --exact --nocapture`
exited 101 in 22.88 s. The `fips-module` package was incorrectly admitted
(`unwrap_err` observed `Ok`), proving the old manifest-derived context.
Restored with copy-back, `cmp`, and `touch` (all exit 0). This is a real
before-equivalent plant, not a claim that the historical tree was executed.

AC4 endpoint-before-equivalent plant: passed the manifest's endpoint rather
than the configured catalog endpoint. The same exact focused test exited 101
in 16.54 s: the package attesting `https://evil.example` returned `Ok` instead
of context drift. Copy-back, `cmp`, and `touch` all exited 0. Both AC4 plants
are needed to falsify the independently sourced crypto and endpoint fields.

AC5 before-equivalent/plant: removing the boot journal call made
`cargo test --locked -p maos-bin --test crypto_provider_20_13
selected_provider_is_journaled_once_per_boot -- --exact --nocapture` exit 101
in 22.22 s: query returned zero frames rather than one. Copy-back, `cmp`, and
`touch` all exited 0.

AC1 plant: changed `select` to silently fall back to the first provider on
unknown names. `cargo test --locked -p maos-bin --test crypto_provider_20_13
unknown_provider_refused_without_changing_config -- --exact --nocapture`
exited 101 in 14.42 s: the boot vector returned 1 rather than 78. The
provider source was restored with copy-back, `cmp`, and `touch` (all exit 0).

AC2 CRL-before-equivalent plant: restored the CRL poller and door's independent
`RingCryptoProvider`. `cargo run --locked -p xtask -- build-test-fixtures
--only maos-crypto-recording && cargo test --locked -p maos-bin --test
crypto_provider_20_13 recording_provider_observes_crl_import_over_door --
--exact --nocapture` exited 101 in 146.91 s: the import succeeded but the
recording log was absent. Main source restored using copy-back, `cmp`, and
`touch` (all exit 0). This is also the observed L1 before-equivalent for the
second-instance defect.

Formatted candidate after D-20-13-17, empty-log correction and integrated
test instrumentation: bin 36209 (+295 from 35914; ceiling 36410), shell 673
(+58; ceiling 737), registry 6194 (+43; ceiling 6253), compliance 2107 (+30;
ceiling 2151), xtask 44874 (+174; ceiling 44938), domain 10198 (+0), kernel
21042 (+0), aggregate 198835 (+600; ceiling 199280). The NEW integrated test
body resides in `tests/support/` per the 14-2b convention and is not counted;
the measured bin source total includes feature-only recorder instrumentation
and inline test harness glue. The temporary inline form measured 36431 and
triggered a STOP; Main authorized normal new-test placement, not relocation of
an existing counted test. `xtask/kloc.toml` was re-pinned to exact source
measurements and aggregate hardfail 198836. Kernel physical baseline 26841.
