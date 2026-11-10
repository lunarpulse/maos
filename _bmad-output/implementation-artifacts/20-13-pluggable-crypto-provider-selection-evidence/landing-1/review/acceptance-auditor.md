# Acceptance audit — 20-13 L1 candidate a2e37988 (parent be3b9763)

Auditor: Acceptance layer, read-only. No cargo/nextest/clippy/xtask ran; recomputation used only `git show`/`git archive`
copies, the stock `tokei 14.0.0` with the exact argument list of `xtask/src/kloc_check.rs:294-318`, and a python SHA-256 of the FKCS
framing at `xtask/src/check_fkcs.rs:841-856`. Nothing outside this file was written.

**Counts: 0 blocker · 0 high · 3 medium · 5 low.** AC1–AC6 are met in code; the three mediums are (M1) a D-20-13-17 violation in the
new provider-path evaluator, (M2) an erasure `not_found` regression that reaches a user-visible 4→5 flip on repetition,
(M3) the AC2 "one boot / one instance" claim and several AC2/AC1 witnesses are not wired to any blocking gate and the
"one instance" witness cannot detect the defect it is named for.

---

## 1. Pins, ceilings, "nothing counted moved" — RECOMPUTED, all PASS

`tokei --output json --types Rust -e target -e tests -e benches -e examples -e fuzz -e spirits -e crates/maos-kernel-core/src/memory/spill_test_faults.rs .`
run on a `git archive be3b9763` copy and on the HEAD tree; per-crate = sum of per-file `code` under `crates/<c>/` (xtask under `xtask/`).

| crate | parent (be3b9763) | HEAD (a2e37988) | Δ | `kloc.toml` pin | rebased ceiling (story Dev Record) | verdict |
|---|---:|---:|---:|---:|---:|---|
| maos-bin | 35,914 | **36,209** | +295 | 36209 | 36,410 | PASS (201 room; **ZERO HEADROOM** pin) |
| maos-shell | 615 | **673** | +58 | 673 | 737 | PASS |
| maos-registry | 6,151 | **6,194** | +43 | 6194 | 6,253 | PASS |
| maos-compliance | 2,077 | **2,107** | +30 | 2107 | 2,151 | PASS |
| maos-domain | 10,198 | **10,198** | 0 | 10198 (unchanged line) | 10,213 | PASS (doc-only edit; comments are not code) |
| xtask | 44,700 | **44,874** | +174 | 44874 | 44,938 | PASS |
| maos-kernel-core (tokei) | 21,042 | **21,042** | 0 | untouched | 21,042 | PASS |
| aggregate | 198,235 | **198,835** | +600 | `_aggregate_hardfail = 198836` (= measured + 1) | 199,280 | PASS |

Ceilings re-derived: story allowances (bin 34687−34191=496, shell 122, registry 102, compliance 74, domain 15, xtask 238, aggregate 1045)
added to the `37baeedf` baseline (35914/615/6151/2077/10198/44700/198235) give 36410/737/6253/2151/10213/44938/199280 — exactly the
Dev Record figures. Sum of per-crate deltas 295+58+43+30+174 = 600 = aggregate delta.

* Kernel: `git diff --name-only be3b9763 a2e37988 -- crates/maos-kernel-core xtask/kernel-core-baseline.toml` lists only
  `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs` (a `tests/` file; excluded from both tokei and the physical baseline). No
  `src/` byte moved ⇒ 26,841 physical / 21,042 tokei untouched. `docs/invariants/*.md`: zero files in the diff (ADR-037 holds).
* "Nothing counted moved": per-file tokei deltas are all **non-negative** (crypto_provider.rs +163, main.rs +117, check_composition_root_completeness.rs +146,
  shell lib.rs +58, install.rs +28, evaluator.rs +28, build_test_fixtures.rs +24, admission.rs +15, installed_spirits.rs +11, check_mock_not_in_release.rs +4,
  delegation.rs +3, compliance lib.rs +2, bin lib.rs +1). No counted line was relocated under `tests/`. The integrated witness body
  (`tests/support/crypto_integrated_20_13.rs`, 264 lines) is NEW, is `include!`d into the cfg(test) module of `main.rs`, and tokei counts by file path,
  so it is uncounted; the harness glue that stayed inline in `main.rs` (SharedBoot, `boot_and_drive_with_shared`) IS counted (inside the +117). Consistent with the
  Dev Record's disclosure (36,431 inline → 36,209) and with 14-2b "new-test placement" (not relocation of an existing counted test).
* Forecast honesty: xtask +174 vs story forecast +59…+99; compliance +30 vs +22…+32; all inside ceilings (D-20-13-13 A ×2/×3 envelope), disclosed.
* HISTORY segments cite the key with `->` first (`20-13-pluggable-crypto-provider-selection L1 … 44700 -> 44874`) for xtask/bin/shell/registry/compliance/aggregate (D11 A) ✓.

**FKCS re-pin** `xtask/fkcs-baseline.toml` 3dc4d08e… → 6499fd6a…: recomputed independently — parent blobs hash to `3dc4d08e…49f9`,
HEAD blobs hash to `6499fd6af0bb83f274765434cf40b4660e3382d19708fc8c9edb7b54a0eda3b3` = the pin. `maos-skill/src/admission.rs` byte-identical;
the delta is the single `maos-registry/src/admission.rs:548-581` hunk. Re-examination of `review/fkcs-admission-review.md`: §0–§6 are correct against the code
(`publisher_signature_message` shared; `verify_publisher_sig_with` no fallback; `verify_installed` takes the provider as a required argument; `None` confined to the
two pre-check wrappers `signed_package_at`/`verify_deployed_package`). Its conditions 1–4 are satisfied (pin + comment in the same commit; negative test
`crates/maos-registry/tests/publisher_provider_20_13.rs`; required param; named pending item in `deferred-work.md`). One sentence in §7 is wrong:
"[ring-direct callers] can only refuse more than a non-ring provider would (a non-ring signature fails ring)" — Ed25519 is interoperable, a non-ring module's valid signature verifies under ring too;
the real exposure is key-class/hardware-held keys, not refusal asymmetry. Not a blocker. §7's evaluator-core concern was addressed by D-20-13-17 but **only for the "ring-direct" half; see M1**.
Note: no run of `fkcs_oracle` or `check-fkcs` after the re-pin is recorded (`verification.md`: check-fkcs timed out, "no gate result is claimed"); my hash recompute closes the arithmetic, not the gate run.

---

## 2. Story ACs → diff → evidence

| AC | What the diff delivers | Evidence checked | Result |
|---|---|---|---|
| AC1.1 init flag | `init_crypto_provider` (`main.rs:1549`), `validate` against `PROVIDERS`, exit 78 listing `known_names()`; `run_init(color, Option<&str>)` writes `[crypto]` on fresh home, `record_crypto_provider` (toml_edit, stage + `create_new` + rename, permissions copied) rewrites only `[crypto]` and prints `old -> new` on an initialized home; air-gap `air_gap_init(&argv[1..])` same flag | `crypto_provider_20_13.rs:9,36,81` | MET, with low L1 (`--crypto-provider=<x>` form ignored) |
| AC1.2 one instance | `select` once at `main.rs:1984`; clones feed capability registry (2226), enterprise runtime (2304), CRL poller + door (3203/3207/3552/3718), Worker T3 (5510, 6102), host-B (`HostBWorkerContext.crypto`), smoke arm (8408), installed boot (`&selected_crypto`, 4318). No production `RingCryptoProvider` left in `maos-bin` outside `crypto_provider.rs` (grep) | structural gate + code read | MET in code; **witness gap M3** |
| AC1.3 refusal | unknown / malformed `[crypto]` / non-string → `Err` → `exit(78)` with `known providers:`; missing file or missing table → `ring`, `defaulted: true` | `unknown_provider_refused_without_changing_config` (bogus at init and at boot, `provider = 4`, bare `run`) | MET; the `contains("ring")` assertion is non-discriminating for the malformed vector ("string" contains "ring") — see L5 |
| AC1.4/1.5 witness + plant | fallback plant exit 101 recorded (`preflight.md`) | recorded | MET |
| AC2.1 feature | `crypto-recording = ["network"]`; `compile_error!` under `cfg(not(debug_assertions))` (`crypto_provider.rs:109-110`); `RecordingCryptoProvider` in `FORBIDDEN_PRODUCTION_SYMBOLS`; fixture entry `maos-crypto-recording` (`build_test_fixtures.rs:124`); no new env var | code | MET. `profile.release` / `profile.dev` in `Cargo.toml:105-111` set no `debug-assertions`; no custom profile inherits release, so the guard is unreachable via any committed profile; only a caller-supplied `RUSTFLAGS=-C debug-assertions` / `--config profile.release.debug-assertions=true` reaches it and then the `nm` symbol gate is the backstop (FKCS review §6 caveat is accurate). |
| AC2.2 four surfaces | token (`recording_provider_observes_boot_token_signing`, real fixture binary, `MAOS_ONE_SHOT`), CRL over the live door (`recording_provider_observes_crl_import_over_door`, real fixture binary + `maosctl`), installed boot ≥2 `verify_signature` (`installed_claims_compare_actual_crypto_and_endpoint`), seal, Worker T3 (`crypto_provider_t3_20_13.rs`, hosted only, returns early locally via `require_t3`) | see M3 | PARTIAL — see M3 |
| AC2.3/2.4 plant | CRL second-instance plant exit 101 recorded | `preflight.md` | MET |
| AC3.1 structural | `crypto_provider_violations` (syn visitor) over `crates/*/src`, allowlist `crypto_provider.rs`, `security/crypto.rs`, two `maos-bench` harness files, the `security/mod.rs` `admit_spirit` line-350 pending exception; `cfg(test)`/`cfg(all(test,…))` mods and fns cut; `use … as` rename, expr path, struct expr flagged | `crypto_construction_in_production_is_refused_after_test_module` (4 vectors) + live plant at `main.rs:1978` exit 1 | MET for the direct shapes; **L2 (blind spots)** |
| AC3.2 inventory | `landing-1/bypass-inventory.md` | re-read against code | MET; cites are stale (L4) |
| AC4.1 | `evaluate_envelope_with` + `evaluate_envelope_core(…, unpinned_empty, verify)`, `verify_publisher_sig_with` + shared `publisher_signature_message`; `verify_installed` (provider, name, endpoint closure) used by `InstalledSpirits::boot` | diff | MET; **M1** |
| AC4.2/4.3 | context carries `crypto.name`; `fips-module` claim → `ContextDrift{CryptoProvider}` and boot refusal `installed spirit … refused at boot`; `""` admits; ring-direct pre-checks unchanged | `crypto_provider_20_13.rs:156` | MET |
| AC4.4 | runtime endpoint = `ProviderCatalog` endpoint for the manifest's `provider_id` (`installed_spirits.rs:26-35`); evil endpoint → `ProviderEndpoint` drift | same test | MET (provider_id/model_id remain manifest-derived: disclosed in the deferred row as "crypto name and endpoint") |
| AC4.5 | `deferred-work.md` D:1379 row annotated "pending landing commit" | file | NOT YET CLOSED — legitimate only because the commit now exists; owed to the operator/landing step. Recorded as open obligation, not a defect. |
| AC4.6 plant | crypto + endpoint plants exit 101 | `preflight.md` | MET |
| AC5 | `journal()` after TL open (`main.rs:2375`): TelemetryEvent, pid 0, `FrameOrigin::Kernel`, payload `{provider,version,source}`; eprintln names selection; `ring_version_matches_lockfile`; `selected_provider_is_journaled_once_per_boot` (rows 1 → 2, one `config` + one `default`) | tests | MET. Payload carries no path/secret; nothing to redact. |
| AC6.1 docs | three copies (`docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md`, ko): English copies byte-identical in the section (verified with `cmp` over the section range); ko is a faithful translation covering the same 7 required elements (selection point/file, default, re-selection, exit 78, frame, v1.5 FIPS, claim boundary) | diff | MET |
| AC6.2 stale docs | `ports/crypto.rs:1-49` ✓, `tests/coverage-matrix.yaml` FR48 notes + gate ✓, NFR-Sec-15 note ✓; kernel `security/crypto.rs:20-27` deliberately L2 (stale now: says main.rs constructs `Arc::new(RingCryptoProvider)`) | diff | MET for L1 |
| AC6.3 | at `done` (L2) | — | n/a |
| AC7 | L2 | — | not claimed (kernel T3 stays a named exception) ✓ honest everywhere I looked |

## 3. D-20-13-01…17

| D-row | Landed as | Verdict |
|---|---|---|
| 01 A config.toml `[crypto] provider` | written by `maos init` (maos-shell, new `toml_edit` dep edge; the crate was already in `Cargo.lock`), read once by `maos run` (`toml`) | ✓ |
| 02 A re-selection / old homes | rewrite only `[crypto]`, `old -> new`, effective next boot; no `[crypto]` → `ring`, `source: default` | ✓ (inline-table `crypto = {…}` is accepted by `select` but refused by `record_crypto_provider` — cosmetic) |
| 03 A table identity | `PROVIDERS` row `ring 0.17.14` + lockfile pin test (`contains`) | ✓ |
| 04 A KT3 | not in L1 (by D-12 A) | n/a |
| 05 A daemon-only routing | only `verify_installed` | ✓ — pending list incomplete, see L3 |
| 06 A / 17 | provider path unpinned-empty; ring-direct byte-identical | ring-direct ✓ (`evaluate_envelope`/`evaluate_envelope_at` call the core with `false` and `verify_ed25519`; the `if false` branches are no-ops; `ring_direct_empty_claim_still_drifts_against_nonempty_runtime`); provider path **over-unpins, M1** |
| 07 A endpoint half | catalog endpoint of the manifest's `provider_id` | ✓ |
| 08 A extend existing gate, "no new job **or registry row**" | gate extended; but `xtask/gate-registry.toml` DID gain `check-composition-root-completeness` (needed so the FR48 matrix row resolves; workspace run exposed it). Deviation from the D-row text with no D-row/ledger note; the row sits under the registry's "Advisory only: absent from aggregate.needs" comment although the job is blocking (`discipline.yml:6667`). Not a defect in behaviour. | deviation noted (L8) |
| 09 A recording | feature, `compile_error!`, symbol, fixture entry, `<home>/crypto-recording.log` (+ an undeclared second file `crypto-recording-identity.log`) | ✓ |
| 10 A / 11 A | outside FR48 / at-rest open kept | ✓ recorded in `bypass-inventory.md` |
| 12 A landing shape | L1 kernel-free; kernel exception pinned by line 350 | ✓ (brittle: any kernel line shift before L2 reds the gate; noise, not a bypass) |
| 13 A envelope | all pins inside rebased ceilings | ✓ |
| 14 A frame | TelemetryEvent `crypto.provider.selected` | ✓ |
| 15 A | file-level overlap with 20-1b handled; FKCS re-pin | ✓ |
| 16 | kernel only | n/a |

## 4. Claims honesty (verification.md / evaluator-callers.md / bypass-inventory.md / docs / matrix)

Checked and **accurate**: pins and aggregate; "T3 not executed locally"; "Apple/ARM/hosted legs unverified"; `check-fkcs` "no gate result claimed"; workspace
run 5859/7 failures disclosed with the three timing failures routed to `deferred-work.md` rather than retried; D-20-13-17 text matches the evaluator; kernel T3 named as L2
exception everywhere; `compile_error` run at `crypto_provider.rs:110`; seven recorded plants each match a test that exists.

Inaccurate or overstated (each is a finding below):
* "one boot passes all four surfaces with one recorded identity" (`verification.md`, `coverage-matrix.yaml` FR48 note) — M3.
* "only boot re-verification is provider-routed; pending ring-direct callers named" — true, but the named list is incomplete — L3.
* `bypass-inventory.md` line cites are stale — L4.
* The sole workspace nextest (5859 pass / 7 fail) and the sole `prepush` (timed out) **predate** D-20-13-17, the uninstall fix, the hosted-fix tests, the integrated witness and the FKCS re-pin; the final tree was validated only by focused runs + static gates. The proof contract wants one final-tree run per AC; the Dev Record states this precisely, so it is disclosed, but the three unretried timing failures (`installed_component_dispatch_reports_typed_boot_refusal` 2.109 s vs 2 s is on the path this patch changed: select + journal + provider-routed `verify_installed`) are attributed to load without any evidence the patch is not the cause. Process note, no finding id.

## 5. Hosted-fix test changes — PASS

* `xtask/tests/ci_shape_18_0.rs`: counts and names are derived with `syn` from `crates/maos-bin/tests/recall_17_3d.rs` (20 `#[test]` fns, I counted 20) and `crates/maos-wasm-host/src/recall_wire.rs` `mod tests` (7, I counted 7); every derived token is removed one at a time (`run.replace(token,"REMOVED")`) and must red; the matching workflow lines are `discipline.yml:4788,4822,5205,6207`. Not a tautology: adding a source test without updating the workflow reds the test. Minor: names are matched by substring (a name that is a prefix of another is not independently checked).
* `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs`: `descendant_is_dead(state) = None | Some("Z"|"X")`; unit test rejects `R`,`S`,`D`. Still rejects every live state including stopped `T`/`t`. `X` (dead) is a legitimate terminal state. The oracle is the same function the ignored test now calls; not too lax. (The `None` arm is unreachable from the production call, which always passes `Some`; harmless.)

## 6. Findings

### M1 — medium — `evaluate_envelope_core` unpins the whole `provider_endpoint` pin, not just an empty URL (violates D-20-13-17 "non-empty attestations compare exactly")
* **file:line** `crates/maos-compliance/src/evaluator.rs:155-157`
* **evidence**
  ```
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint = claimed.provider_endpoint.clone();
  }
  ```
  `ProviderEndpointPin` = `{ provider_id, endpoint_url, model_id }` (`maos-spirit-abi/src/compliance.rs:259-267`). Assigning the whole struct copies the claim's `provider_id` and `model_id` into the runtime fingerprint, so a claim that attests `provider_id = "evil"`, `endpoint_url = ""` matches any runtime pin, and the later hash comparison (`actual_hash` is computed from the overwritten fingerprint) agrees too. At the parent, `verify_installed` built the context from the manifest and `evaluate_envelope` rejected this claim with `ContextDrift{ProviderEndpoint}`; in the provider path it now admits. In the daemon boot path the runtime `provider_id`/`model_id` are the manifest's (`install.rs: runtime_endpoint = fields.provider_endpoint.clone()`), so a signed claim that disagrees with its own manifest about provider/model is the exact drift CCAC exists to refuse. D-20-13-06/17 define unpinned as an empty *attested value*; a non-empty `provider_id` is a non-empty attestation and must "compare exactly on both paths".
* **fix**
  ```rust
  if unpinned_empty && claimed.provider_endpoint.endpoint_url.is_empty() {
      actual_fp.provider_endpoint.endpoint_url = String::new();
  }
  ```
  (provider_id and model_id keep comparing exactly).
* **failing-before witness** add to `crates/maos-compliance/tests/evaluator_test.rs`: build a runtime fingerprint with `provider_id="anthropic", endpoint_url="https://api.anthropic.com"`, attest the same but `provider_id="evil", endpoint_url=""` via `build_self_attested_envelope`, assert `maos_compliance::evaluate_envelope_with(&envelope, &ctx_from(&runtime), &RingCryptoProvider)` is `Reject(ContextDrift{field: ProviderEndpoint,..})`. On a2e37988 it returns `Admit`. (`cargo test -p maos-compliance --test evaluator_test` — new case reds.)

### M2 — medium — prior-boot selection frames make `not_found` unreachable after a home's first boot; the same uninstall command flips exit 4 → 5 on repetition (D-16-2-F empty branch not preserved)
* **file:line** `crates/maos-bin/src/main.rs:10850-10866` (query `prior_frames`), `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:704-731` (the test that pins the behaviour)
* **evidence** The comment says "all other rows, including selections from earlier boots, still count", and the new test `prior_boot_provider_selection_still_counts_as_populated_history` asserts exit 5 for a log whose only row is a `crypto.provider.selected` frame from boot_nonce 1. Every boot now writes one (`main.rs:2375`, before the one-shot uninstall path runs). `fixture.run_uninstall(None)` in `not_found_uninstall_has_distinct_terminal_code` (exit 4 on a pristine home) leaves its own selection frame in the persisted audit DB; a second identical invocation sees that frame with a different `boot_nonce`, so `prior_frames` is true, `all_incarnations` is empty, and it returns `unknown Spirit … in populated Transparency Log` ⇒ exit 5. The same applies after any daemon boot that never ran a Spirit. Infrastructure telemetry from earlier boots carries no evidence about the requested Spirit, which is precisely what D-16-2-F's empty-vs-populated distinction protects ("not an identity inferred from unrelated rows").
* **fix** exclude every `crypto.provider.selected` selection frame regardless of boot: drop the `boot_nonce = ?1` conjunct (and the `?1` parameter) from the SQL at `main.rs:10857-10858`/`10860-10861`, and invert `prior_boot_provider_selection_still_counts_as_populated_history` to expect exit 4 `not_found`; add a control with one non-selection prior frame expecting exit 5. Optional dead-weight cleanup: `TransparencyLogAdapter::has_frames` (`crates/maos-iac/src/adapter/transparency_log.rs:1880`) now has no caller (previously only `main.rs`).
* **failing-before witness** in `erasure_uninstall_13_5b.rs`: `let f = Fixture::new(); let a = f.run_uninstall(None); let b = f.run_uninstall(None); assert_eq!(a.status.code(), Some(4)); assert_eq!(b.status.code(), Some(4));` — second assertion fails on a2e37988 (exit 5); both pass at the parent (no boot frame existed).

### M3 — medium — AC2 "one boot / one instance" witness is composed by the test, and the feature-gated AC2 witnesses are not run by any blocking gate
* **file:line** `crates/maos-bin/tests/support/crypto_integrated_20_13.rs:26-35,168-176,195-206,249-253`; `crates/maos-bin/tests/crypto_provider_20_13.rs:496-511`; `tests/coverage-matrix.yaml` FR48 notes; `verification.md` "Acceptance witness index" AC2 row
* **evidence**
  1. The integrated test never boots `main.rs`. It calls `select` once and then hands `Arc::clone(&selected.provider)` to a hand-built kernel world, a hand-built door (`regression_door_with_crypto`), `InstalledSpirits::boot` and the 13.5a harness; token issue is a direct `world.capability.issue`. The final "all rows share one instance id" assertion is true by construction. It cannot detect the defect AC1.2/AC2.4 name (the production root building a second provider); the recorded plant ("second provider instance planted only into the door") modified the **test's** door, not `main.rs`. The real-binary tests (`recording_provider_observes_*`) prove each surface reaches *a* recording provider, but they all append to the one shared `crypto-recording.log`; two instances would pass them. Nothing asserts a single `pid:instance` across surfaces in a real `maos` process even though `crypto-recording-identity.log` exists for exactly that.
  2. `recording_provider_routes_real_at_rest_seal` is `#[cfg(feature = "crypto-recording")]` and calls `maos_secrets::seal_at_rest(&kms, selected.provider.as_ref(), …)` directly, i.e. it proves `seal_at_rest` calls the provider it is given (already true at HEAD), not that the daemon's enterprise runtime does. Under the documented command (`cargo nextest run -p maos-bin --test crypto_provider_20_13`, no feature) it is compiled out; `verification.md`'s "9 tests passed" for that command is 8 + the T3 test, so it never ran there, and no run of it with `--features crypto-recording` is recorded.
  3. `grep -rn crypto-recording .github xtask .config scripts` finds only `xtask/src/build_test_fixtures.rs`; `crypto_provider_one_boot_reaches_all_four_surfaces` and the recording assertions inside `story_13_5a_enterprise_governance_reaches_the_booted_cohort_daemon` need `--features crypto-recording` and are invoked only by the branch-only `ci/ci-20-13-verify.yml` (not installed on main). The workspace nextest/prepush never run them. `coverage-matrix.yaml` FR48 nonetheless lists `crypto_integrated_20_13` as the AC2 witness "on one boot with one recording-provider identity".
* **fix** (a) in a real-process test (fixture `maos-crypto-recording`, `MAOS_ONE_SHOT=hello-spirit` or the door test) assert every line of `crypto-recording-identity.log` carries the same `pid:instance` prefix and that distinct operations were observed; (b) make the seal check boot-driven (the 13.5a daemon harness already asserts `seal_for_export`; keep that and delete or reword the direct `seal_at_rest` test); (c) add `--features crypto-recording` runs of the bin unit test and `crypto_provider_20_13` to a blocking job (or re-label the matrix note "branch-only recipe, not a gate"). Reword the matrix/verification text from "one boot with one identity" to what is proven.
* **failing-before witness** plant in `main.rs:3203`: replace `let crypto_provider = Arc::clone(&crypto);` with `let crypto_provider = maos_bin::crypto_provider::select(&crypto_home).unwrap().provider;` (second recording instance), rebuild fixtures, run `cargo nextest run -p maos-bin --test crypto_provider_20_13` — all tests stay green on a2e37988 (shared log); with fix (a) the CRL test reds on the second `pid:instance`.

### L1 — low — `--crypto-provider=<name>` is silently ignored
* **file:line** `crates/maos-bin/src/main.rs:1549-1561` (`args.iter().position(|arg| arg == "--crypto-provider")`)
* **evidence** only the two-token spelling is recognised; `maos init --crypto-provider=bogus` returns `Ok(None)`, exits 0 and records the default `ring` (fresh home) or leaves the old selection (initialized home). That is the failing-before behaviour AC1.4 describes ("flag is ignored"), for the `=` spelling. An operator who types `--crypto-provider=fips` believes a module was selected.
* **fix** also match `arg.strip_prefix("--crypto-provider=")` (refuse an empty value), or reject any unrecognised `--crypto-provider*` token with exit 78.
* **witness** `crypto_provider_20_13.rs`: `command(&["init","--crypto-provider=bogus"])` expects status 78 and unchanged config; on a2e37988 status is 0.

### L2 — low/medium (0.6) — structural gate misses macro bodies, qualified-self paths, `Default::default()` through an annotated type, and type aliases
* **file:line** `xtask/src/check_composition_root_completeness.rs:177-257` (visitor hooks only `visit_expr_path`, `visit_expr_struct`, `visit_use_rename`, mod/fn cfg cuts)
* **evidence** `syn` does not parse macro invocation bodies, so `RingCryptoProvider` inside `tokio::select!{…}`/`vec![…]`/`format!`/any `macro_rules!` is invisible (`main.rs` is `select!`-heavy); `<RingCryptoProvider as Default>::default()` is an `ExprPath` whose *qself* is not inspected (only `path.segments`); `let c: RingCryptoProvider = Default::default();` and `type R = RingCryptoProvider;` name the type only in type position, never visited, and `RingCryptoProvider` derives `Default` (`security/crypto.rs:36`). The negative control (`crypto_construction_in_production_is_refused_after_test_module`) covers path, `::default()`, `{}` and `use … as`. Nothing in the tree is currently bypassing (grep + live gate), so this is gate robustness, not an active leak. `cfg_test` (line 250) also cuts `cfg(all(test, …))` by prefix only for mod/fn/impl-fn, which is conservative (false-positive) rather than permissive.
* **fix** add `visit_macro` that token-scans `mac.tokens` for the ident `RingCryptoProvider`, `visit_type_path` flagging any path segment named `RingCryptoProvider` outside `use` items (which also catches `ExprPath` qself and `type` aliases), and extend the unit test.
* **witness** extend `crypto_tests::violations` with `"fn root(){ let _ = vec![RingCryptoProvider]; }"`, `"fn root(){ let _ = <RingCryptoProvider as Default>::default(); }"`, `"type R = RingCryptoProvider;"` each expecting 1; on a2e37988 each returns 0.

### L3 — low — pending ring-direct list is incomplete; operator docs do not mention ring-direct daemon admission
* **file:line** `_bmad-output/implementation-artifacts/deferred-work.md` new row after :1480; story Dev Record "D-05 boundary / pending"; `docs/maos.dev/run-maos.md` "Crypto provider" (+2 copies)
* **evidence** production ring-direct verifiers not named: `maos-registry/src/storage.rs:475` (`import_legacy_pin`, reached from the daemon boot enumeration `default_installed_manifests()` at `main.rs:4284` → `storage.rs:329`, i.e. before `verify_installed`), `maos-registry/src/handlers/publish.rs:13` (registry server `server.rs:534`), `maos-registry/src/install.rs:96` (pre-check), `maos-registry/src/compliance_verify.rs:66`, `maos-spirit-cli/src/vet.rs:51`; and `verify_installed` itself still verifies vetting attestations/keyring events ring-direct (`install.rs` `promotion.verify` → `maos-compliance/src/vetting/*::ed25519_verify`) for promoted packages (covered by D-10's "vetting" outside FR48, but the "boot re-verification is provider-routed" sentence does not say so). The docs' claim boundary lists audit artifacts, publisher tooling and TLS only; "the daemon re-verifies before anything runs" reads wider than the evidence.
* **fix** extend the deferred row with the five sites above and one sentence about vetting; add "other admission and yank paths in the daemon still verify with `ring` until L2/v1.5" to the three docs.
* **witness** `grep -rn "verify_publisher_sig(" crates --include=*.rs` outside `/tests/` returns the unnamed sites.

### L4 — low — evidence cites are stale
* **file:line** `landing-1/bypass-inventory.md` bullets 1 and 3
* **evidence** "Lines 118 and 124 are the feature-gated test recorder's delegate type and construction" — actual `crypto_provider.rs:123,131`; "`main.rs:15609,15870`" — actual `main.rs:15622,15883`.
* **fix** update the numbers (or cite by symbol).
* **witness** `grep -n RingCryptoProvider crates/maos-bin/src/crypto_provider.rs crates/maos-bin/src/main.rs`.

### L5 — low — malformed-selection vector's stderr assertion cannot fail
* **file:line** `crates/maos-bin/tests/crypto_provider_20_13.rs:55-58`
* **evidence** for `provider = 4` the error text is `… [crypto] provider must be a string (known providers: …)`; `contains("ring")` is satisfied by the word "string". Removing the known-providers list from the message would not red this vector (the bogus vector at :46-48 is discriminating).
* **fix** assert `contains("known providers: ring")`.
* **witness** make `known_names()` return `""`: the malformed vector still passes on a2e37988.

### L6 — low — `maos_home()?.unwrap_or_default()` reads `./config.toml` when neither `MAOS_HOME` nor `HOME` is set
* **file:line** `crates/maos-bin/src/main.rs:1983`
* **evidence** `maos_home()` returns `Ok(None)` in that case (`operator_door.rs:65-92`); `PathBuf::default().join("config.toml")` is the relative `config.toml`, so the selection (and an exit-78 refusal) is taken from whatever file sits in the cwd. Every other root consumer treats `None` as "no home" (`main.rs:3444-3446`, `3595`).
* **fix** `let selected = match maos_home()? { Some(home) => select(&home), None => select_default() }` (no file read) — or refuse with a named message.
* **witness** run `env -i maos run --once …` from a dir containing `config.toml` with `[crypto] provider="bogus"`: exits 78 citing `config.toml` instead of defaulting.

### L7 — low — `SCANNED_SOURCE_FILES` comment ledger not extended
* **file:line** `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:~929-934`
* **evidence** array 38 → 39 with `crypto_provider.rs`, but the "34 → 35: `replica_resolver.rs` joins it" history comment gained no 20-13 line (story T1 says 37 → 38; the count at HEAD was already 38). Cosmetic ledger drift.
* **fix** add the comment line.

### L8 — low — D-20-13-08 A says "no new … registry row"; one was added without a recorded deviation
* **file:line** `xtask/gate-registry.toml:41`
* **evidence** `check-composition-root-completeness` added inside the block headed "Advisory only: absent from aggregate.needs and local Blocking prepush set" although the job is blocking (`discipline.yml:431-443,6667`); needed to satisfy `coverage_matrix_20_3b` once the FR48 matrix row names the gate (`verification.md` workspace-run bullet).
* **fix** record the deviation in the story ledger (D-20-13-08 note) and move the row out of the advisory block (or fix the comment).
* **witness** `grep -n "composition" xtask/gate-registry.toml .github/workflows/discipline.yml`.

## 7. Open obligations that are NOT defects

* `deferred-work.md` D:1379 → `CLOSED … by 20-13 @<commit>` (AC4.5) still reads "pending landing commit".
* Worker T3 leg, ARM, macOS legs, FKCS gate run and `prepush` are unrun locally; disclosed.
* Kernel T3 line-350 exception, `security/crypto.rs:20-27` stale doc, KT3 and AC7 are L2.
