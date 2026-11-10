# Blind-hunter review — a2e37988 (parent be3b9763)

Scope: diff of `crates/`, `xtask/` (src + the two hosted-fix tests). No cargo run; all claims are from source reading. `[INFERENCE]` marks anything not directly provable from source.

Counts: 0 blocker, 0 high, 4 medium, 4 low.

## BH-01 (medium) Boot-path "unpinned empty endpoint" replaces the WHOLE endpoint pin, masking provider_id / model_id drift
- File: `crates/maos-compliance/src/evaluator.rs:155-157`
- Evidence:
  ```
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint = claimed.provider_endpoint.clone();
  }
  ```
  The condition tests only `endpoint_url`, but the assignment overwrites `provider_id` and `model_id` too. Step 4 then compares `claimed.provider_endpoint != actual_fp.provider_endpoint` (equal by construction) and the final `fingerprint_hash` is recomputed over the already-substituted `actual_fp`, so neither check can see a divergent `provider_id`/`model_id`.
- Impact: in `verify_installed` -> `evaluate_envelope_with`, a signed claim whose `provider_id` (or `model_id`) differs from the manifest/runtime value, but whose `endpoint_url` is empty, is ADMITTED on daemon boot; the ring-direct `evaluate_envelope_at` rejects the same input with `ContextDrift{ProviderEndpoint}`. D-20-13-17 only licenses treating an empty *attested value* as unpinned. The new test `installed_claims_compare_actual_crypto_and_endpoint` uses `verify("", "")` where claim and runtime `provider_id` are both "anthropic", so it cannot see this.
- Exact fix: copy only the empty field:
  ```
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint.endpoint_url.clear();
  }
  ```
- Failing-before witness (new test in `crates/maos-compliance/tests/evaluator_test.rs`): build a signed envelope whose fingerprint has `provider_endpoint{provider_id:"openai", endpoint_url:"", model_id:None}`; build a ctx with `runtime_provider_endpoint{provider_id:"anthropic", endpoint_url:"https://api.anthropic.com", model_id:None}`; assert `evaluate_envelope_with(&env,&ctx,&RingCryptoProvider)` is `Reject(ContextDrift{field: ProviderEndpoint,..})`. On the candidate it returns `Admit`.

## BH-02 (medium) `maos init --crypto-provider=<name>` (and any non-space spelling) is silently ignored and writes `ring`
- File: `crates/maos-bin/src/main.rs:1549-1560` (`init_crypto_provider`), callers `main.rs:1539` and `main.rs:1866`
- Evidence: `args.iter().position(|arg| arg == "--crypto-provider")` is the only recognizer; `verbs::dispatch` only matches the verb name and `Init` ignores every other flag. `maos init --crypto-provider=fips-module` therefore returns `Ok(None)`, `run_init(color, None)` writes `provider = "ring"` and exits 0.
- Impact: an operator who asked for a non-ring provider gets a success exit and a config that selects ring; the unknown-name exit-78-with-known-list contract is bypassed by spelling. Also `maos init --crypto-provider ring --crypto-provider x` silently takes the first.
- Exact fix: in `init_crypto_provider` also match `arg.strip_prefix("--crypto-provider=")` (route through the same `validate`), and reject a second occurrence.
- Failing-before witness: add to `crates/maos-bin/tests/crypto_provider_20_13.rs`: run `maos init --crypto-provider=bogus` with a clean env; assert exit code 78 and stderr containing `known: ring`. Candidate exits 0 and writes `provider = "ring"`.

## BH-03 (medium) The composition-root structural check is bypassed by type aliases, turbofish/qualified-self type positions and macro bodies
- File: `xtask/src/check_composition_root_completeness.rs:195-250` (`CryptoConstructionVisitor`)
- Evidence: the visitor flags only (a) `use ... as` renames, (b) `ExprPath` whose own `segments` contain `RingCryptoProvider`, (c) `ExprStruct`. It does not override `visit_type_path`/`visit_item_type`/`visit_macro`, so none of these report: `type Hidden = RingCryptoProvider; let _ = Hidden;` (alias, then `Hidden` is an ordinary ExprPath), `make::<RingCryptoProvider>()` / `<RingCryptoProvider as Default>::default()` (type sits in generic args / `qself`, not in `path.segments`), `let c: RingCryptoProvider = Default::default();`, and anything inside a `macro_rules!`/`vec![]`/`dbg![]` token stream (syn does not parse it). The unit test `crypto_construction_in_production_is_refused_after_test_module` covers exactly the four forms the visitor was written for, so the "negative control" proves only those.
- Impact: a production crate can construct the ring provider outside the selection root and the gate stays green, defeating the "one provider instance per boot" structural guarantee that AC1-AC6 rest on.
- Exact fix: add `fn visit_type_path(&mut self, t:&'ast syn::TypePath)` flagging any segment `RingCryptoProvider` (this covers alias definitions, turbofish and qself types), and `fn visit_macro` that scans `mac.tokens` for the ident `RingCryptoProvider` and reports it. Keep the existing pending_t3 exemption only on the expression path.
- Failing-before witness: add to `crypto_tests`: `assert_eq!(violations("type Hidden = RingCryptoProvider; fn root(){ let _ = Hidden; }"), 1); assert_eq!(violations("fn root(){ let _ = <RingCryptoProvider as Default>::default(); }"), 1); assert_eq!(violations("fn root(){ let _ = vec![RingCryptoProvider]; }"), 1);` Candidate returns 0 for each.

## BH-04 (medium) Docs/inventory claim narrower-than-actual ring-direct surface: daemon-resident publisher-signature checks after boot still bypass the selected provider
- Files: `crates/maos-bin/src/admission.rs:578` (`bind_registry_package_at` -> `signed_package_at` -> ring `verify_publisher_sig`, called from `main.rs:5667`, `6211`, `10188`), `crates/maos-bin/src/yank_policy.rs:100` (`verify_publisher_sig`), `crates/maos-bin/src/main.rs:193` (`verify_deployed_package`), plus `maos-registry/src/storage.rs:475`.
- Evidence: `docs/maos.dev/run-maos.md` (new "Crypto provider" section) says "Daemon boot re-verifies installed publisher signatures and ComplianceClaims using the selected provider ... the daemon re-verifies before anything runs" and lists only "audit-artifact signatures, publisher tooling and rustls TLS" as outside the boundary; `bypass-inventory.md` names only the maosctl install/import pre-checks. The daemon's own run-time admission, vetted-upgrade and yank-policy paths above still verify the same publisher signature with ring directly (`verify_publisher_sig`, no provider parameter).
- Impact: with `provider = "recording"` (or any future non-ring provider) a `maos run` of an installed spirit performs publisher-signature verification that never reaches the selected provider, yet the documentation reads as if daemon publisher verification is provider-routed. Claims-honesty gap; the operator expecting a FIPS provider would be misled. [INFERENCE: author may intend these as the "named pending" set, but they are not named.]
- Exact fix: either thread `&dyn CryptoProvider` into `bind_registry_package_at`/`yank_policy` and use `verify_publisher_sig_with`, or add these four call sites by name to the doc paragraph and `bypass-inventory.md` as pending ring-direct daemon callers.
- Failing-before witness: with the `maos-crypto-recording` fixture, install the package from `installed_claims_compare_actual_crypto_and_endpoint`, run `maos run <installed manifest>`; count `verify_signature` lines in `crypto-recording.log` before/after the admission step: admission adds none although it verified `publisher_pubkey`.

## BH-05 (low) Hard-coded source line number (`== 350`) in the structural-check exemption
- File: `xtask/src/check_composition_root_completeness.rs:221`
- Evidence: `&& !(self.pending_t3 && expr.span().start().line == 350)` ties the T3 exception to line 350 of `maos-kernel-core/src/security/mod.rs`. Any unrelated edit above that line (that file is shared with later 20-13 L2 work, which also removes this construction) moves the line and turns the gate red with a message pointing at the wrong thing; conversely any *other* `RingCryptoProvider` expression that lands on line 350 inside `admit_spirit` is silently exempted.
- Exact fix: exempt by structure instead of line: require `self.path == "maos-kernel-core/src/security/mod.rs"`, enclosing fn `admit_spirit`, and allow at most one such expression per fn (count it), or remove the exemption when L2 lands.
- Failing-before witness: unit test feeding `maos-kernel-core/src/security/mod.rs` source in which `admit_spirit` contains the construction on line 351 expects 0 violations; candidate reports 1 (and a construction placed on line 350 of a second statement expects 1; candidate reports 0).

## BH-06 (low) Selection reads `./config.toml` from the CWD when neither MAOS_HOME nor HOME is set
- File: `crates/maos-bin/src/main.rs:1983`
- Evidence: `maos_home()?.unwrap_or_default()` yields an empty `PathBuf`; `select` does `home.join("config.toml")` = relative `config.toml`. `maos_home()` returns `Ok(None)` exactly when both variables are unset/empty (`operator_door.rs:78-85`).
- Impact: a stray `config.toml` in the daemon's working directory (e.g. a service started without HOME in a project dir) is parsed as the MAOS config: invalid TOML or `[crypto] provider = "x"` there refuses boot with exit 78, and a valid `[crypto]` section there selects the provider. Every other home-dependent path in the repo errors with "neither MAOS_HOME nor HOME is set" (`maos-shell/src/lib.rs` `maos_home`).
- Exact fix: treat `None` as "no config" (provider = ring default) without touching the filesystem, e.g. `let selected = match maos_home()? { Some(h) => select(&h), None => select_default() }`, or fail closed with the same message the shell uses.
- Failing-before witness: `cd $(mktemp -d); printf '[crypto]\nprovider="bogus"\n' > config.toml; env -u HOME -u MAOS_HOME maos <boot path>` -> exits 78 with the bogus provider message from the CWD file (the file is not a MAOS home). [INFERENCE: whether the rest of boot tolerates an unset home was not traced.]

## BH-07 (low) `crypto-recording` guard keys on `debug_assertions`, not the profile
- File: `crates/maos-bin/src/crypto_provider.rs:109-110`
- Evidence: `#[cfg(not(debug_assertions))] compile_error!(...)`. A release build with `debug-assertions = true` (`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`, a profile override, or `RUSTFLAGS=-Cdebug-assertions`) compiles the module and ships a selectable `recording` provider that appends every crypto call (including capability-token signing operations) to `$MAOS_HOME/crypto-recording*.log`. The only remaining backstop is `check-mock-not-in-release` (`nm` symbol match on `RecordingCryptoProvider`), which is only run if the release artifact is passed to it. [INFERENCE: no such profile is configured today; `Cargo.toml` has no `[profile.release] debug-assertions`.]
- Exact fix: key the guard on something profile-independent that CI controls, e.g. also refuse when `cfg(not(test))` and `PROFILE=release` via a `build.rs` `cargo:rustc-cfg=maos_release_profile` and `compile_error!` on that cfg.
- Failing-before witness: `RUSTFLAGS=-Cdebug-assertions cargo build --release -p maos-bin --features crypto-recording` succeeds on the candidate (expected: compile error).

## BH-08 (low) `maos init --crypto-provider` cannot re-pin a config the daemon accepts (inline `[crypto]` table)
- File: `crates/maos-shell/src/lib.rs:809-810`
- Evidence: `item.as_table().ok_or("[crypto] must be a table")?` rejects a `crypto = { provider = "ring" }` inline table (toml_edit gives `Item::Value`, not `Item::Table`), whereas the daemon's `select` (`toml::Value`) accepts it. `run_init` then returns the error (exit 1) from the "already initialized" branch after `ensure_operator_door` has already run.
- Exact fix: handle `Item::Value(Value::InlineTable)` (use `item.as_table_like()` and `get`/`insert` through `TableLike`).
- Failing-before witness: home whose `config.toml` has `[slots]`, `[retention]` and, before the first header, `crypto = { provider = "ring" }`; `maos init --crypto-provider ring` exits 1 with "[crypto] must be a table" while `maos_bin::crypto_provider::select(&home)` returns Ok.

## Checked, no finding
- Erasure interplay (`main.rs:10847-10867`): the `NOT (boot_nonce=? AND spirit_pid=0 AND kind=? AND intent=? AND origin=?)` filter ignores only this boot's selection frame; prior-boot selection frames and every other row still make the log "populated", matching the added test. Column encodings (`kind as i64`, `FrameOrigin::Kernel as i64`, `boot_nonce as i64`, high bit masked at `main.rs:2210-2220`) match `insert_frame_row`.
- One instance per boot in `main()`: `select` is called once (`main.rs:1984`), `journal` once (`2375`), every later consumer (`capability` 2226, enterprise 2304, revocation 3203, door 3552/3718, worker 5510/6102, cohort 9309, T3 8408, installed boot 4318) clones that Arc; no new `RingCryptoProvider` constructions in non-test maos-bin code. No unwrap/expect after serde in new non-test code (`select`, `journal`, `record_crypto_provider`, xtask visitor use `?`/`map_err`).
- No-default-features/macOS: `crypto_provider.rs` depends only on ungated crates (`maos-kernel-core`, `toml`, `serde_json`); `init_crypto_provider` is un-gated and used by both mains; `maos-registry` addition is dev-only. No platform-specific APIs added.
- Hosted-fix tests: `xtask/tests/ci_shape_18_0.rs` counts are derived from source and agree with the current `discipline.yml` (20 recall tests, 7 recall_wire tests); minor non-blocking brittleness: `source_test_names` counts `#[ignore]`d/`cfg`-gated `#[test]` fns, which the workflow's "N tests run" would not. `cgroup_ceiling_smoke.rs` oracle accepts only `None|Z|X` and the new unit test rejects R/S/D.
