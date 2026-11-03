# 20-13 creation — measurements (2026-11-03, HEAD `8c809f9c`)

No cargo ran (parent rule: a sibling owns cargo; no build, no test, no new target dir). No crate source in the tree was
modified: every prototype was made on a `git archive 8c809f9c` copy under `/tmp/proto2013/<name>/` (deleted after
measurement), every changed file `rustfmt --edition 2021` (rustfmt 1.9.0; each touched HEAD file was `rustfmt --check`
clean first, so the deltas are the edit, not reformatting), then measured with the prebuilt gate binary **from that copy's
root**:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto2013/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
(tokei inside `kloc-check`). Every `.diff` here passes `git apply --check` against a fresh `git archive HEAD` copy (the
working tree carries a sibling's uncommitted 20-4 edits, so the diffs target HEAD). Raw results:
`kloc-kernel-8c809f9c-and-prototypes.json`. Prototypes were not compiled ([INFERENCE] that they build).

## Instrument replication

| Figure | Method | `/tmp` copy `base` | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26841**, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`, `passed: true` | `xtask/kernel-core-baseline.toml:505` |
| Kernel tokei | `kloc-check --json` | **21042** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed: true`, `over_budget: []`) | maos-bin **34191** · maos-domain **10099** · maos-shell **615** · maos-registry **4248** · maos-compliance **2032** · maos-secrets **536** · maos-audit **7721** · maos-capability **1433** · maos-cli **6429** · xtask **44406** · aggregate **192519** | `:517` · `:425` · `:691` · `:688` · `:481` (2117) · `:480` (1000) · `:409` · `:265` · `:489` (6856) · `:339` · `_aggregate_hardfail` `:718` = 192520 |

Aggregate room: **1 line**. maos-compliance (85) and maos-secrets (464) have numeric pin room; under D02 A every added line
still needs the aggregate grant.

## Kernel-touching options, priced by throwaway prototype (D-20-13-03, -04, -11)

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Also |
|---|---|---|---:|---:|---|
| **KT3** | `SecurityManagerAdapter::new(policy, crypto)` — a required `Arc<dyn CryptoProvider>`; T3 admission (`security/mod.rs:350`) uses `self.crypto`; `impl Default` removed (only callers: kernel tests `provider_switched_journal.rs` ×4, `xtask/tests/story_10_4a_ac1_proven_red.rs:996`); `#[derive(Debug)]` replaced by a 7-line manual `Debug` (the trait object has no `Debug`) (D-04 A, **recommended**) | `kt3-required-ctor-param.diff` | **+1** (26842) | **−1** (21041) | 22 `::new(` callers gain an argument: 4 in `main.rs` (production `:2279`, smoke `:14813`, tests `:15321,16107`), the rest under `tests/` (uncharged) |
| KT1 | builder `with_crypto_provider(crypto)`; `new()`/`Default` keep a `RingCryptoProvider` default; manual `Debug` (D-04 B) | `kt1-builder-ring-default.diff` | +20 (26861) | +14 (21056) | no caller churn; a root that forgets the builder silently verifies with `ring` |
| KTd | KT1 with `Debug` as a `CryptoProvider` supertrait instead of the manual `Debug` (variant of D-04 B) | `ktd-builder-debug-supertrait.diff` | +11 (26852) | +6 (21048) | domain trait change; 20 of 22 impls lack `Debug` (1 kernel src `image_verify.rs:233`, `maos-capability/src/cap_tokens/mod.rs:346`, `maos-bench/src/bin/tier_c_harness.rs:71`, 17 tests/benches/fuzz) — not measured, +1 physical each |
| KI | `CryptoProvider::identity()` (name + version) required method; `RingCryptoProvider` and the two kernel test providers implement it (D-03 B) | `ki-trait-identity.diff` | +19 (26860) | +18 (21060) | maos-domain +6; 20 more impls across the tree |
| KO | `CryptoProvider::open_sealed` + `RingCryptoProvider` impl + two kernel test providers; `open_at_rest` could then route (D-11 B) | `ko-trait-open-sealed.diff` | +38 (26879) | +37 (21079) | maos-domain +7; 20 more impls |
| none | T3 admission keeps its inline `RingCryptoProvider` (D-04 C) | — | 0 | 0 | FR48 false for the kernel T3 admission path; a permanent gate allowlist entry |

Kernel tokei counts `#[cfg(test)]` modules inside `src/` (KI/KO: the two kernel test providers are +12 of their figure).
Every kernel shape moves `security/mod.rs`'s content hash (`kernel-core-baseline.toml` digest row) and both pins; KI/KO also
move `security/crypto.rs` and `security/sandbox/t3/image_verify.rs`.

**ADR-069 room and the cumulative** (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-48`): HEAD 21042 tokei. Ruled,
unlanded kernel rows: 20-1b +68, 20-4 +9, 20-11 +6 → expected **21125**. With KT3 −1 → **21124** expected (ceiling 21127);
KT1 → 21139; KT3 + KI → 21142; KT3 + KI + KO → 21179 — each under the internal STOP **21209** and the 22000 alarm on its own.
With the concurrent PROPOSED rows of the same day and HEAD (20-8 KS +33, 20-14 KT +25, 20-9 R +21): 21204 before this
story, **21203 with KT3** (6 below 21209); KT1 **21218**; KTd **21210**; KT3+KI **21221**; any KO ≥ **21240** → story D-20-13-16. Physical: 26841 + 78 + 13 + 10 + 1 = **26943** expected with KT3.

## Out-of-kernel, priced by prototype (`nk-l1-bundle-plus-kt3.diff`, KT3 included so the callers compile in shape)

| # | Prototype | Measured (tokei) |
|---|---|---:|
| SEL | NEW `crates/maos-bin/src/crypto_provider.rs`: `PROVIDERS` table (name, version, constructor; `ring` 0.17.14), `validate` (typed "unknown crypto provider 'x' (known: ring)"), `select(home)` reading `[crypto] provider` from `<home>/config.toml` via `toml` (absent table → `ring`, `defaulted`), `journal` (TelemetryEvent `crypto.provider.selected` `{provider, version, source}`), `lib.rs` module | maos-bin **+95** |
| REC | `crypto-recording` feature (`Cargo.toml`), `mod recording` (`compile_error!` without `debug_assertions`, as `maos-secrets/src/lib.rs:9-10`): delegates to `ring`, appends one op name per call to `<home>/crypto-recording.log` | maos-bin **+50** |
| WIRE | root: `select` + exit 78 on error, one instance (`:2218`), CRL instance removed (`:3238`), `SecurityManagerAdapter::new(policy, crypto)` (4 sites), boot journal after the TL exists (`:2404` < `:3266`), smoke `:8403`, Worker parameter (`worker_spawn.rs`, `main.rs:5504,6088`, `delegation.rs` host-B context + `run_cohort_a2a_daemon`), `InstalledSpirits::boot(.., &selected_crypto, ..)`, init `--crypto-provider` parse + `validate` | maos-bin **+43** |
| INIT | `maos_shell::run_init(color, crypto_provider)`: fresh home writes `[crypto] provider` (default `ring`); initialized home with a flag rewrites only `[crypto]` (stage + rename) and prints `old -> new` (D-02 A) | maos-shell **+46** |
| COMP | `evaluate_envelope_with(envelope, ctx, crypto)`; one `evaluate_envelope_core` with an injected verifier; unpinned rule (empty attested `crypto_provider` is compared as empty, D-06 A); `lib.rs` export | maos-compliance **+22** |
| REG | `verify_publisher_sig_with(pkg, crypto)` + shared `publisher_sig_message`; `verify_installed(name, path, crypto, crypto_provider)` uses both and the selected name in the runtime context | maos-registry **+21** |
| GATE | `check-composition-root-completeness` gains a crate-wide production scan for `RingCryptoProvider` (test modules cut by regex; `use`/`pub use` lines skipped; allowlist with reasons) | xtask **+59** |
| **Measured total** | SEL + REC + WIRE + INIT + COMP + REG + GATE + KT3 | maos-bin **+188**, maos-shell **+46**, maos-compliance **+22**, maos-registry **+21**, xtask **+59**, kernel **−1**; aggregate **+335** (192854) |

## Gate simulation (AC3 failing-before, measured)

`gatesim.py` (the GATE prototype's logic in Python) over the base copy → **5 production constructions**
(`gate-sim-base.txt`): `main.rs:2218`, `main.rs:3239`, `main.rs:8403`, `worker_spawn.rs:888`,
`maos-kernel-core/src/security/mod.rs:350`. Over the `nk` copy → **1** (`gate-sim-nk.txt`): `main.rs:14843` — the
`smoke_abi_7_5a` one-shot root the prototype fed a fresh `RingCryptoProvider`; the dev routes it through
`crypto_provider::select` (one instance per process) instead. At L1 (before KT3) the kernel line is a named pending entry.

## Not measured here (owed to the dev pass; a measured figure above the ruled envelope is STOP + D-row)

1. Compilation of every prototype (`cargo check --locked -p maos-kernel-core -p maos-bin -p maos-shell -p maos-compliance -p maos-registry -p xtask`, and `--features crypto-recording`); in particular the `fn(&Path) -> Arc<dyn CryptoProvider>` closures in a `const` table and the manual `Debug`.
2. The kernel-surface baseline (`docs/ci-baselines/kernel-surface-v0.1-beta.json:912` signature hash of `SecurityManagerAdapter`) after a private field is added and `impl Default` removed: run the surface/ABI gate that reads it (`xtask/src/main.rs:215`); a red is a re-pin under the same FLAG-Winston row, never a silent update.
3. The hermetic trigger for an at-rest seal (`seal_at_rest_opt`, `enterprise_identity.rs:290,477`) in a spawned `maos` (the enterprise KMS env of the 13.5a tests) — AC2's seal leg.
4. CCAC: `check-corpus` / the CCAC ship gate after the unpinned rule (expected unchanged: drift seeds attest non-empty `fips-module-drift`, `maos-corpus-gen/src/ccac/expansion.rs:240`; missing-field seeds stay `MalformedClaim`, `ccac-seeds-v1.0.toml:259-275`).
5. `abi-diff` / `stability-matrix --check` if `evaluate_envelope_with` or `verify_installed`'s signature is on a tracked surface.
6. 20-4 L1's endpoint table (in flight, uncommitted at HEAD) — the shape AC4.4 reads under D-20-13-07 A.
7. Unmeasured forecasts below (smoke-arm routing, `build-test-fixtures` entry, help text, docs).
