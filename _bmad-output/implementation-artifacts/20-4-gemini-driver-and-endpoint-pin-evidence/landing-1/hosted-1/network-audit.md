# Hosted-1 network-boundary audit

Baseline handoff: HEAD76efc19d, clean tree supplied by Main. Source of regression: 20-4 correction commit73e7afbc. Read-only diagnosis: agent DiagRun3, discipline41815429165 / check-exit-commands job126524848874.

The exact no-default verb-table control was run before edits and returned **101**, with E0433 at main.rs16802/16814: the helper's return/constructed types referenced the intentionally network-gated `maos_bin::admission` module. The fix places `#[cfg(feature = "network")]` on `configured_provider_catalog` and `#[cfg(all(test, feature = "network"))]` on its provider-only unit module. All five shared tuple constants remain attribute-free; lib.rs gating and the workflow control were not changed.

The same control after the fix returned **0: 7/7 passed**. The command retains the required `env -u CI -u GITHUB_ACTIONS` and an absolute manifest path; no feature is added to the negative control.

The original correction diff73e7afbc^..73e7afbc and complete L1 source range8c809f9c..73e7afbc were read and grep-audited for optional-module/provider references. Existing Anthropic imports and BoundInference are network-guarded; the production caller is inside the network-only main at main.rs1880. Admission changes remain behind lib.rs8–9. The ungated helper and provider-only test module were the additional production boundary defects found. CLI/DB integration supports do not add optional provider/admission symbol imports. Diff outputs and grep observations remain in the command evidence.

## Requested broader check: existing out-of-scope failure

`cargo check -p maos-bin --no-default-features --all-targets` was exercised and returned **101** in untouched `crates/maos-bin/tests/eval_halt_18_2.rs`: line26 imports `maos_bin::cassette_replay`; lines353/357 reference `maos_providers`. Both are unavailable without network. That file is outside both audited 20-4 diffs. Main was notified; no older test or library feature boundary was changed to hide this result. The broader check is not reported green or narrowed to exclude that test.

The workflow feature scan found maos-bin network/default, network+wasm-host (also default+wasm-host), and fixture_replay combinations. Their native all-target outcomes are retained in the command ledger. The workflow's aarch64-apple-darwin cross-target job is not claimed exercised locally. Other workflow feature flags belong to unrelated SDK/bench/kernel packages and were not broadened into this hotfix.
