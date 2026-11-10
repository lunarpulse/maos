# FKCS admission-path review — 20-13 L1 (`crates/maos-registry/src/admission.rs`)

Reviewer: independent read-only review. Base HEAD `be3b9763`, working tree on top.
Subject: `xtask/fkcs-baseline.toml` `[admission_baseline]` drift.

## 0. Exact change under review

Declared file set (`xtask/fkcs-baseline.toml:[admission_baseline].files`):
`crates/maos-registry/src/admission.rs`, `crates/maos-skill/src/admission.rs`.

`git diff HEAD` over both files: **only `maos-registry/src/admission.rs`, one hunk at `@@ -548,6 +548,26` (new lines 548-581, +22/-3).** `maos-skill/src/admission.rs` is byte-identical to HEAD.

Fingerprint recomputed independently (path-stamped framing exactly as `xtask/src/check_fkcs.rs:841-856`: `path || 0x00 || len_u64_le || bytes`):
- HEAD blobs  -> `3dc4d08e…49f9` (= pinned value; baseline was honest at HEAD)
- working tree -> `6499fd6a…3b3` (= reported actual)
So the whole drift is this one hunk and nothing else.

The hunk (admission.rs:548-581):
1. `verify_publisher_sig(pkg)` (ring, unchanged signature/behaviour): now builds `msg` via new private `publisher_signature_message(pkg)` and verifies `UnparsedPublicKey::new(&ED25519, &pkg.publisher_pubkey).verify(&msg, &pkg.signature).is_ok()` — identical to HEAD's logic.
2. NEW `pub fn verify_publisher_sig_with(pkg, crypto: &dyn maos_domain::ports::CryptoProvider) -> bool` = `crypto.verify_signature(&pkg.publisher_pubkey, &publisher_signature_message(pkg), &pkg.signature).is_ok()`.
3. NEW private `publisher_signature_message(pkg) -> [u8; 32]`: SHA-256 over `u64_le(manifest.len) || manifest || u64_le(artifact.len) || artifact`; `hasher.finalize().into()`.

## 1. Same bytes, same keys, same refusal conditions — PASS
- Message: HEAD hashed the identical four updates in the identical order (diff shows the four `hasher.update` lines moved unchanged into the helper). `GenericArray<u8,32>` -> `[u8;32]` is the same 32 bytes. Both paths (legacy ring and `_with`) derive from the one helper, so they cannot diverge.
- Key: `pkg.publisher_pubkey` (`[u8;32]`) in both; signature `&pkg.signature` in both. No key is looked up from the provider, config, or environment.
- Refusal: both reduce the result to `.is_ok()`; any error (not only `SignatureInvalid`) is refusal. For the only shipped provider the adapter (`maos-kernel-core/src/security/crypto.rs:39-49`) is `UnparsedPublicKey::new(&ED25519, pk).verify(msg, sig)` mapped to `SignatureInvalid` — the same primitive and arguments as the legacy path, so `ring` selected == HEAD behaviour bit-for-bit. The provider is used solely as the verification primitive; every other check (key-class / tier / org-key / revocation / manifest identity) in `admit_spirit` is untouched (call sites 152, 296, 313 unchanged).

## 2. No removed/reordered/weakened checks; fail closed — PASS
- No check removed or reordered in admission.rs; the hunk adds code after `extract_manifest_tier` and before the tests module only.
- No fallback inside `verify_publisher_sig_with`: it has no `Option`, no ring default, no error swallowing beyond `is_ok()` (error => false => refuse).
- The only place a ring default could appear is `install.rs:234-237` (`crypto.map_or_else(|| verify_publisher_sig(pkg), |p| …_with(pkg,p))`) — outside the FKCS set, but checked: `None` is reachable only from the pre-existing public wrappers `signed_package_at` / `verify_deployed_package` (install.rs:184-186, 190-192), which are the D-20-13-05 A "install-time pre-check, ring-direct" surfaces. The daemon boot entry `verify_installed` takes `crypto: &dyn CryptoProvider` as a **required** argument (install.rs:295-301) and always passes `Some(crypto)` (install.rs:~305) — a missing provider does not compile. No silent substitution.

## 3. Error mapping — UNCHANGED
`verify_publisher_sig_with` returns `bool`, as `verify_publisher_sig` did; callers map `false` to the same `"installed publisher signature is invalid"` error (install.rs:236-238). No new variants, no `AdmissionError` change.

## 4. Empty/unknown/unpinned provider cannot make admission accept — PASS (for this file)
- Unknown name: refused at selection, before any admission, `crates/maos-bin/src/crypto_provider.rs:43-53,55-88` (`select` -> `validate` returns `Err` for any name not in `PROVIDERS`; absent `[crypto]` is a *recorded default* of `ring`, `defaulted:true`, journaled; a malformed `[crypto]`/`provider` non-string/unparsable TOML/unreadable file is `Err`, not default — lines 64-80). Only a missing `config.toml`/missing `[crypto]` table yields ring, which is the ruled D-20-13-02 A behaviour.
- admission.rs itself cannot receive an "empty" provider: the parameter is a non-nullable trait object.
- The ComplianceClaim "unpinned" rule (empty attested `crypto_provider`/`provider_endpoint` accepted, `evaluator.rs:~145-152`) affects only *context drift* comparison, never signature verification (`evaluate_envelope_core` still rejects `signing_alg != Ed25519` and a failing `verify(...)` first, evaluator.rs:117-126). It is ruled (D-20-13-06 A) and lives outside the FKCS set.

## 5. Panics / serde / deps / signatures — PASS
- No `unwrap`/`expect`/indexing/serde in the hunk. `finalize().into()` is infallible.
- Public surface in the FKCS set: +1 pub fn `verify_publisher_sig_with` (declared AC4.1, D-20-13-05). `verify_publisher_sig` signature unchanged; `publisher_signature_message` private.
- Dependencies: admission.rs adds none (`maos-domain` and `sha2`/`ring` were already normal deps of maos-registry, Cargo.toml:11,20,21). `Cargo.toml` diff adds only a **dev-dependency** on `maos-kernel-core` (used by `yank_auth_20_1b.rs` test) — not in the release dependency graph.
- Public signature changes elsewhere (install.rs `verify_installed` gained 3 params; its callers updated: `installed_spirits.rs:26`, test `yank_auth_20_1b.rs:607`, `crypto_provider_20_13.rs:211`) are declared by AC4.1/D-05/D-07; not in the FKCS set.

## 6. Test-only recording provider not selectable in release — PASS (with a caveat)
- `crates/maos-bin/src/crypto_provider.rs:19-24` the `recording` row is `#[cfg(feature = "crypto-recording")]`; the module has `#[cfg(not(debug_assertions))] compile_error!` (lines 107-110) — a release build with the feature fails to compile rather than shipping the provider. Feature is non-default (`maos-bin/Cargo.toml:37`, `crypto-recording = ["network"]`), built only as a test fixture (`xtask/src/build_test_fixtures.rs:124-144`).
- Second line of defence: `RecordingCryptoProvider` is added to `FORBIDDEN_PRODUCTION_SYMBOLS` in `check_mock_not_in_release.rs:29-33`.
- Caveat (not a blocker; ruled D-20-13-09 A, `kms-fault-inject` precedent): the guard keys on `debug_assertions`, so a release profile that sets `debug-assertions = true` would compile it; the symbol gate is the backstop. Nothing in admission.rs references the recording provider.
- Even if selected, the recording provider delegates verification to ring (`crypto_provider.rs:137-146`) and only appends a line to a log — it cannot widen acceptance.

## 7. Observations (not blockers for the re-pin; none in the hashed hunk)
- The implementer's summary "admission signature checks go through the selected provider" is narrower than stated: within `admission.rs`, `admit_spirit` (152, 296, 313) and the shared ring `verify_publisher_sig` remain ring-direct; only the daemon **boot re-verification** (`verify_installed`) uses `_with`. That is exactly D-20-13-05 A. Other daemon-side ring-direct callers remain: `maos-bin/src/yank_policy.rs:100`, `maos-bin/src/admission.rs:578` (`signed_package_at`), `main.rs:193` (`verify_deployed_package`), `main.rs:10769`. They can only refuse more than a non-ring provider would (a non-ring signature fails ring), so they fail closed; with ring as the sole shipped provider no behaviour differs. Document/track for the v1.5 FIPS module (AC3 scan only counts `RingCryptoProvider` construction, not `ring::` direct use).
- There is no unit-level negative test of `verify_publisher_sig_with` (tampered artifact/manifest/signature, and a provider that returns `Err`, each => `false`; and equivalence with `verify_publisher_sig` for ring). `crypto_provider_20_13.rs:155+` covers claim drift, not the publisher `_with` path. Place it under `crates/maos-registry/tests/` (NOT in admission.rs `mod tests`, which would perturb the fingerprint again).
- Out of FKCS scope: `evaluate_envelope_core` now overwrites the actual fingerprint's `crypto_provider`/`provider_endpoint` with the claimed value when the claimed one is empty, and this core is shared with the non-`_with` `evaluate_envelope` used by ring-direct maosctl paths. If any such caller builds context from manifest-derived non-empty values, an empty attested claim that previously drifted now admits. Ruled by D-20-13-06 A ("as today"); confirm in the compliance review.

## Verdict: **RE-PIN JUSTIFIED**

The declared-file delta is exactly the one admission.rs hunk above; it is a behaviour-preserving extraction plus one additive, fail-closed provider-parameterised verifier. Recomputed fingerprint `6499fd6af0bb83f274765434cf40b4660e3382d19708fc8c9edb7b54a0eda3b3` corresponds to that change alone (maos-skill admission unchanged).

Conditions:
1. Re-pin `xtask/fkcs-baseline.toml` `sha256` and the provenance comment (`3dc4d08e… -> 6499fd6a…`, "20-13 L1, after independent review: …/landing-1/review/fkcs-admission-review.md; exact hunk admission.rs:548-581, `verify_publisher_sig_with` + shared `publisher_signature_message`; skill admission, frozen tag, file set unchanged") in the SAME landing commit as the code, with the declared files byte-identical to the reviewed state. Any further edit to either declared file (including adding tests inside admission.rs) invalidates this review.
2. Add the negative/equivalence test for `verify_publisher_sig_with` outside admission.rs (see §7) before or at landing.
3. Keep `verify_installed`'s provider parameter required and `None` confined to the two ring-direct pre-check wrappers (install.rs:184-192).
4. Record the §7 ring-direct daemon callers as a named pending item (L2/v1.5) so the landing text does not claim all admission verification is provider-routed.
