# 20-13 L1 — edge-case hunter findings (a2e37988 vs be3b9763)

Read-only review. No cargo/nextest/clippy/fmt/xtask was run; every claim below is from source reading
(`git show a2e37988:<path>`) plus the cited consumer-side code. "Failing-before witness" = a test/command that fails
on the candidate commit a2e37988 (and, where noted, passes on be3b9763) and would pass after the fix. Speculation is marked [INFERENCE].

Counts: 0 blocker, 0 high, 4 medium, 2 low (+ observations).

---

## F1 — medium — provider-path "unpinned" rule replaces the WHOLE `provider_endpoint`, masking `provider_id` / `model_id` drift

- **File:line:** `crates/maos-compliance/src/evaluator.rs:155-157`
- **Evidence:**
  ```rust
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint = claimed.provider_endpoint.clone();
  }
  ```
  D-20-13-06 A / AC4.4 say an attested *empty value* (`endpoint_url`) is unpinned. The assignment copies the entire
  `ProviderEndpointPin { provider_id, endpoint_url, model_id }` from the claim over the runtime value. In
  `verify_installed` (`maos-registry/src/install.rs:~311-318`) the runtime `provider_id` and `model_id` are the
  manifest's (`fields.provider_endpoint.clone()`), only `endpoint_url` is replaced from the catalog. So a signed claim with
  `provider_id = "evil"` / a different `model_id` and `endpoint_url = ""` now compares equal at step 4 (`claimed.provider_endpoint != actual_fp.provider_endpoint` is false)
  and the hash check at step 4b hashes the already-substituted `actual_fp`, so it cannot catch it either. On be3b9763 (ring-direct, no substitution) the same package is `ContextDrift{ProviderEndpoint}`;
  D-20-13-17 says only EMPTY attestations are unpinned — here non-empty `provider_id`/`model_id` are silently unpinned too.
  The new test `ring_direct_empty_claim_still_drifts_against_nonempty_runtime` only covers the ring-direct path; no provider-path vector
  varies `provider_id` or `model_id`.
- **Impact:** boot re-verification of installed Spirits (`installed_spirits.rs` → `verify_installed`) admits a claim whose provider id / model pin
  disagrees with the manifest whenever the claim's URL is empty — a partial re-introduction of the vacuous FR38 drift check this story closes.
- **Exact fix:**
  ```rust
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint.endpoint_url.clear();
  }
  ```
  (clear only the URL on the runtime side; `claimed.provider_endpoint.endpoint_url` is already empty, so the URL then compares equal while `provider_id`/`model_id` still compare exactly.)
- **Failing-before witness** (add to `crates/maos-compliance/tests/evaluator_test.rs`; fails on a2e37988 with `Admit`, expected `Reject(ContextDrift{field: ProviderEndpoint,..})`):
  ```rust
  #[test]
  fn provider_path_empty_url_does_not_unpin_provider_id_or_model() {
      let (kp, pk) = seeded_keypair(0x2014);
      let runtime = reference_fp();                       // non-empty provider_id/url
      let mut attested = runtime.clone();
      attested.provider_endpoint.provider_id = "other-provider".into();
      attested.provider_endpoint.model_id = Some("other-model".into());
      attested.provider_endpoint.endpoint_url.clear();
      let env = build_self_attested_envelope(&attested, &kp, pk);
      assert!(matches!(
          maos_compliance::evaluate_envelope_with(&env, &ctx_from(&runtime), &ring_provider()),
          ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field: DriftField::ProviderEndpoint, .. })
      ));
  }
  ```

## F2 — medium — D-16-2-F empty-vs-populated distinction decays after the first boot (`maos uninstall <unknown>` flips 4 → 5 on the 2nd invocation)

- **File:line:** `crates/maos-bin/src/main.rs:10855-10866` (+ the boot-time `journal` call at `main.rs:2373`, `crates/maos-bin/src/crypto_provider.rs` `journal`)
- **Evidence:** every boot, including the offline one-shot `MAOS_ONE_SHOT=uninstall` child (`main.rs:7370-7395`), now appends one `crypto.provider.selected` kernel frame before the cascade runs.
  The patch ignores only frames with `boot_nonce = <this boot>`:
  `WHERE NOT (boot_nonce = ?1 AND spirit_pid = 0 AND kind = ?2 AND intent = ?3 AND origin = ?4)`.
  The author's own test (`erasure_uninstall_13_5b.rs::prior_boot_provider_selection_still_counts_as_populated_history`) pins "a previous boot's selection frame counts as populated history → exit 5".
  Consequence: on a machine whose Transparency Log held nothing but infrastructure, the first `maos uninstall ghost` returns `not_found` (exit 4, D-16-2-O "absent-or-empty TL ⇒ NotFound"), but the second invocation sees the first invocation's own selection frame (different boot nonce) as "populated" and returns `failed`/exit 5 `unknown Spirit … in populated Transparency Log`.
  The same happens after any earlier `maos run`/shell/one-shot boot that left no Spirit evidence. On be3b9763 both invocations return 4 (no boot frame existed). The rule's intent (a log with *evidence* about other Spirits) is not served by a frame that is infrastructure by the patch's own comment ("It is infrastructure, not pre-existing evidence about the requested Spirit").
- **Impact:** the empty-log `not_found` branch becomes reachable only once per home; repeated/idempotent uninstall of a mistyped or already-removed name is no longer idempotent; automation that keys on exit 4 vs 5 sees spurious failure.
- **Exact fix:** drop the `boot_nonce = ?1` conjunct (treat every kernel pid-0 `crypto.provider.selected` TelemetryEvent as infrastructure, not just this boot's):
  ```rust
  "SELECT EXISTS(SELECT 1 FROM transparency_log WHERE NOT \
   (spirit_pid = 0 AND kind = ?1 AND intent = ?2 AND origin = ?3))",
  rusqlite::params![
      maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent as i64,
      "crypto.provider.selected",
      maos_domain::invariants::i3::FrameOrigin::Kernel as i64,
  ],
  ```
  and flip `prior_boot_provider_selection_still_counts_as_populated_history` to assert exit 4 (add a sibling that seeds one selection frame **plus** one unrelated row and asserts exit 5, so populated-log failure stays pinned).
- **Failing-before witness** (add to `crates/maos-bin/tests/erasure_uninstall_13_5b.rs`; fails on a2e37988 at the second assert, passes on be3b9763):
  ```rust
  #[test]
  fn repeated_unknown_uninstall_on_empty_home_is_not_found_each_time() {
      let fixture = Fixture::new();
      for attempt in 0..2 {
          let mut command = fixture.command();
          command.env("MAOS_SPIRIT_ID", "ghost-spirit");
          let output = command.output().unwrap();
          assert_eq!(output.status.code(), Some(4), "attempt {attempt}: {}", terminal(&output));
      }
  }
  ```

## F3 — medium — `maos init --crypto-provider=<name>` (and a repeated flag) is silently ignored → silent `ring` fallback, exit 0

- **File:line:** `crates/maos-bin/src/main.rs:1549-1560` (`init_crypto_provider`)
- **Evidence:**
  ```rust
  let Some(index) = args.iter().position(|arg| arg == "--crypto-provider") else {
      return Ok(None);
  };
  ```
  Only the exact two-token form is recognised. `verbs::dispatch` looks at `argv[0]` only (`verbs.rs:266-279`) and `init` rejects no unknown flags (`main.rs:1860-1870`, `1533-1545`), so `maos init --crypto-provider=fips-module` returns `Ok(None)`, `run_init(color, None)` writes `[crypto] provider = "ring"` and exits 0 (or, on an initialised home, changes nothing and prints "already initialized"). AC1.3 / story guardrail: unknown provider is exit 78, "never a silent fall-back to `ring`". The `=` spelling is the common CLI form and this is a security-relevant selection. Same class: `--crypto-provider ring --crypto-provider bogus` validates only the first occurrence.
- **Impact:** an operator who requests a non-`ring` module with `--crypto-provider=<x>` gets `ring` recorded and journaled as `source: config`, with exit 0 and no diagnostic.
- **Exact fix:** in `init_crypto_provider` scan all args: reject any `--crypto-provider=...` (or parse it into the value), and reject a second `--crypto-provider` occurrence, both with exit 78 via the existing error path, e.g.
  ```rust
  fn init_crypto_provider(args: &[String]) -> Result<Option<&str>, String> {
      let mut found: Option<&str> = None;
      let mut iter = args.iter();
      while let Some(arg) = iter.next() {
          let value = if arg == "--crypto-provider" {
              iter.next().map(String::as_str).ok_or_else(|| format!(
                  "--crypto-provider requires a value (known: {})",
                  maos_bin::crypto_provider::known_names()))?
          } else if let Some(v) = arg.strip_prefix("--crypto-provider=") {
              v
          } else {
              continue;
          };
          if found.replace(value).is_some() {
              return Err("--crypto-provider given more than once".into());
          }
      }
      if let Some(name) = found {
          maos_bin::crypto_provider::validate(name)?;
      }
      Ok(found)
  }
  ```
- **Failing-before witness** (add to `crates/maos-bin/tests/crypto_provider_20_13.rs`, same env scaffolding as `unknown_provider_refused_without_changing_config`):
  `Command::new(env!("CARGO_BIN_EXE_maos")).args(["init", "--crypto-provider=bogus"])…` ⇒ `assert_eq!(status.code(), Some(78))`; on a2e37988 the status is `Some(0)` and `config.toml` contains `provider = "ring"`.

## F4 — medium — `check-composition-root-completeness` provider scan misses macro bodies, qualified paths and type aliases; negative control only covers a direct path/struct literal

- **File:line:** `xtask/src/check_composition_root_completeness.rs:177-248` (visitor), `:259-291` (negative controls)
- **Evidence:** the visitor flags only `ExprPath` segments, `ExprStruct` paths and `use … as` renames of the literal ident `RingCryptoProvider`. Nothing overrides `visit_macro`/`visit_qself`/`visit_type_path`, and `syn::visit` does not descend into macro token streams. Undetected constructions in production code of an otherwise-scanned file (`crates/maos-bin/src/main.rs` is exactly such a file — it is full of `tokio::select!`/`vec!`/`matches!`/`assert!` bodies):
  1. `vec![std::sync::Arc::new(maos_kernel_core::security::crypto::RingCryptoProvider)]` / any `tokio::select!` arm / `lazy_static!` initialiser (macro tokens are never visited);
  2. `<RingCryptoProvider as Default>::default()` — the type sits in `ExprPath.qself`, `path.segments` is `Default::default`;
  3. `type P = RingCryptoProvider; let _ = P {};` (alias is an `ItemType`/`TypePath`, the struct literal path is `P`), or `let c: RingCryptoProvider = Default::default();` (the type-only mention is never visited; `RingCryptoProvider` derives `Default`, `security/crypto.rs:36`);
  4. `include!("…")` pulling a file the walker never parses.
  The only live negative control (preflight AC3 plant) is `let _c = maos_kernel_core::security::crypto::RingCryptoProvider;` and the four unit vectors are the same shapes, so none of the above would red. The scan also only walks `crates/` (`collect_rs_files(Path::new("crates"))`, line 101) although `spirits/{architect,butler,mira,observer,orchestrator,researcher,worker}` (shipping workspace members) depend on `maos-kernel-core` (`spirits/worker/Cargo.toml:30`).
- **Impact:** AC3's "no FR48 surface bypasses the selection" is enforceable only against the one spelling the plant used; a future bypass in any of the shapes above passes the blocking gate.
- **Exact fix:** (a) add `fn visit_macro(&mut self, mac: &'ast syn::Macro)` that recursively walks `mac.tokens` (`proc_macro2::TokenTree::{Group,Ident}`) and records a violation for any ident `RingCryptoProvider` (same span-line reporting); (b) replace `visit_expr_path`/`visit_expr_struct` with a single `visit_path` override (covers qself types, type aliases, turbofish, struct literals) and keep the `use … as` rename check; (c) treat `include!` (path macro `include`/`include_str` excluded) as a violation outside the allowlist; (d) extend the scan roots to `spirits` and `examples`. Add unit vectors for each shape (macro, qself, alias, turbofish).
- **Failing-before witness** (extend `crypto_tests::crypto_construction_in_production_is_refused_after_test_module`; each assertion is `0` on a2e37988, expected `1`):
  ```rust
  assert_eq!(violations("fn root() { let _v = vec![std::sync::Arc::new(RingCryptoProvider)]; }"), 1);
  assert_eq!(violations("fn root() { let _ = <RingCryptoProvider as Default>::default(); }"), 1);
  assert_eq!(violations("type P = RingCryptoProvider; fn root() { let _ = P {}; }"), 1);
  assert_eq!(violations("fn root() { let _c: RingCryptoProvider = Default::default(); }"), 1);
  ```

## F5 — low — `maos_home()?.unwrap_or_default()` turns "no home" into a CWD-relative `config.toml`

- **File:line:** `crates/maos-bin/src/main.rs:1983-1984`
- **Evidence:** `maos_home()` returns `Ok(None)` when neither `MAOS_HOME` nor `HOME` is set (`operator_door.rs:61-90`); `unwrap_or_default()` yields `PathBuf::new()`, so `select` reads `Path::new("").join("config.toml")` = `./config.toml` in the daemon's CWD (and the `recording` provider would write `./crypto-recording.log`). Every other consumer in this file handles `Ok(None)` explicitly (`main.rs:3444-3460`, `3695-3706`: "requires MAOS_HOME or HOME"). The relative-home hazard is exactly what `maos_home()` documents it refuses ("init, the daemon, and maosctl may have different working directories"). An unrelated `config.toml` in CWD (HOME unset is common for containers/systemd units without `Environment=HOME`) is parsed as MAOS config and a non-TOML or `[crypto]`-bearing file refuses boot with exit 78 or selects a provider.
  [INFERENCE: the second effect requires a foreign `config.toml` in CWD; the CWD read itself is certain.]
- **Impact:** wrong selection source / spurious exit 78 for homeless boots; behavior depends on CWD.
- **Exact fix:** make `select` take `Option<&Path>`: `None` ⇒ no config file read, provider `ring`, `defaulted = true`; build `recording` only with `Some(home)` (error otherwise). Call site: `select(maos_home()?.as_deref())`.
- **Failing-before witness:** `env_clear()`, no `HOME`/`MAOS_HOME`, `current_dir(tmp)` with `tmp/config.toml` = `not = [valid`; run `maos run`: a2e37988 exits 78 naming `config.toml`; be3b9763 does not read it (and the fix does not).

## F6 — low — `maos init --crypto-provider X` cannot repair or update a `[crypto]` the boot path accepts/refuses

- **File:line:** `crates/maos-shell/src/lib.rs:807-818` (`record_crypto_provider`)
- **Evidence:** `item.as_table().ok_or("[crypto] must be a table")?` rejects an inline table (`crypto = { provider = "ring" }`), which `crypto_provider::select` (via `toml::Value::get`) accepts; and `.get("provider")…ok_or("[crypto] provider must be a string")?` makes a table with no/wrong `provider` — the exact malformed state boot refuses with exit 78 — unfixable through the only sanctioned writer (D-20-13-01 "written only by `maos init`"): the re-run errors (exit 1, after `ensure_operator_door`) instead of rewriting the section.
- **Impact:** operator must hand-edit `config.toml`, contradicting D-20-13-02 A ("rewrites only `[crypto]`").
- **Exact fix:** use `item.as_table_like()` and treat a missing `provider` as `previous = None`; only reject a present non-string `provider`.
- **Failing-before witness:** `config.toml` = default config + `crypto = { provider = "ring" }` inserted before the first table, or `[crypto]\nnote = 1\n` appended; `maos init --crypto-provider ring` must exit 0 and leave `provider = "ring"`; a2e37988 exits 1 with "[crypto] must be a table" / "[crypto] provider must be a string".

---

## Observations (no finding filed; ranked by value)

1. **Integrated "one boot" witness** (`tests/support/crypto_integrated_20_13.rs`) is a hand-built composition (`kernel_world_with_crypto`, `regression_door_with_crypto`, `boot_and_drive_with_shared`) that hands the same `Arc` from one `select()` call to every surface. Its `pid:instance` identity check proves the *test harness* shares one instance, not that `main.rs` does; a `main.rs` regression that re-introduced a second provider is caught only by the separate process-level `recording_*` tests (presence of operation names, not identity) and by the AC3 gate (see F4 for its holes). Not filed: coverage is adequate for L1 given those, but the test name overstates.
2. **Kernel exception is keyed by source line** (`xtask/src/check_composition_root_completeness.rs:221`, `expr.span().start().line == 350`). Any edit above `security/mod.rs:350` (the story lists 20-1b/20-4/20-11 kernel rows as pending) turns the blocking gate red until xtask is hand-edited. [INFERENCE on landing order]. Key it on the enclosing fn + count of exactly one instead.
3. **`ci_shape_18_0` hosted-fix counts are source-derived** (`source_test_names` over `recall_17_3d.rs` / `recall_wire.rs`) — good — but the tokens are plain substrings: `format!("{n} passed; 0 failed; 0 ignored;")` is contained in a stale "17 passed; …" script when `n = 7`, and `--expected-count {n}` is contained in `--expected-count 18` when `n = 1`. Direction "script stale-high" is not caught; direction "source grew" is. Also `source_test_names` counts `#[ignore]`d and `cfg`-gated top-level tests and skips tests inside nested modules, so the derived count can diverge from nextest's. Low value; no failing case exists today.
4. **cgroup X-state oracle** (`cgroup_ceiling_smoke.rs`): `matches!(state, None | Some("Z" | "X"))` rejects `R`/`S`/`D` (and `T`, `t`, `I`), unit-tested; `None` is unreachable from the production call (always `Some`) and lowercase `x` (pre-3.14 kernels) is not accepted — neither is too lax. No finding.
5. **`crypto-recording` gating**: `compile_error!` is `#[cfg(not(debug_assertions))]`, so only an explicit `-C debug-assertions`/`profile.release.debug-assertions=true` build reaches it; no profile in `Cargo.toml` sets that and no workflow passes `--all-features` to a release build. The residual defence is the `RecordingCryptoProvider` symbol gate in `check-mock-not-in-release` (unchanged mechanism, depends on symbols surviving). No finding.
6. **Checked and sound:** evaluator split — `evaluate_envelope_at` → core with `unpinned_empty=false` and `&verify_ed25519` is the same comparison sequence as before, so ring-direct is unchanged; unknown/empty/case-variant/whitespace provider names all fall to `validate` (strict equality, exit 78, known list printed; `"Ring"`, `""`, `"ring "` refused); non-table `crypto`, non-string `provider`, TOML syntax error, unreadable file all refused with exit 78 while `NotFound` ⇒ default; `journal` swallow of the I2 token is safe (`insert_frame_event_with_sender` panics on write failure, `transparency_log.rs:1135-1139`); `boot_nonce`/`kind`/`origin` are bound as `i64` exactly as written (`transparency_log.rs:1396-1402`); fresh-init `format!` interpolates only a table-validated name; `record_crypto_provider` writes content only after copying permissions, cleans its temp file, and keeps unrelated keys/comments (toml_edit).
