# 20-13 L1 (a2e37988 vs be3b9763) — review triage (Review2013L1)

Layers: Blind hunter, Edge-case hunter, Acceptance auditor (agent `reviewer`), Test-infra audit (`light-reviewer`). No security layer (orchestrator runs it). Read-only; no cargo run. Per-layer reports (full evidence/witness text): `blind-hunter.md`, `edge-hunter.md`, `acceptance-auditor.md`, `test-infra.md` in this dir. The reviewer first-hand checks by me: evaluator.rs:150-157, main.rs:10850-10866, crypto_provider.rs, structural gate line-350 literal, no `debug-assertions` profile override in any Cargo.toml, `maos-registry` kernel-core dep is dev-only.

Counts after dedupe: 0 blocker, 0 high, 6 medium (P1–P6), 7 low patch (P7–P13), 3 defer, 6 dismiss. Pins recomputed by the acceptance auditor (tokei 14.0.0, kloc_check args): all PASS (bin 36,209 ≤ 36,410; shell 673; registry 6,194; compliance 2,107; domain 10,198; xtask 44,874; kernel 21,042 untouched; aggregate 198,835; tests/ uncounted, nothing counted moved; FKCS admission pin recomputes).

## PATCH

### P1 (medium) Provider path unpins the WHOLE endpoint, not just the empty URL — violates D-20-13-17 "non-empty attestations compare exactly"
Sources: BH-01, Edge F1, Acceptance M1. `crates/maos-compliance/src/evaluator.rs:155-157` (`actual_fp.provider_endpoint = claimed.provider_endpoint.clone()` when claimed `endpoint_url` is empty). A claim with a different `provider_id`/`model_id` and empty URL admits on the provider (boot) path; the ring-direct path and HEAD reject it.
Fix: replace with `actual_fp.provider_endpoint.endpoint_url = claimed.provider_endpoint.endpoint_url.clone();` (same shape for crypto_provider, which is a single string and is fine).
Witness (fails on a2e37988): `evaluator_test` — claim with `provider_id = "other"` and empty `endpoint_url`, `evaluate_envelope_with(..)` must be `Reject(ProviderEndpointMismatch)` (it returns Accept now); and one with different `model_id`.

### P2 (medium) `maos init --crypto-provider=<x>` (and repeats) silently ignored → silent `ring`, exit 0
Sources: BH-02, Edge F3, Acceptance L1. `crates/maos-bin/src/main.rs:1549-1561` (`init_crypto_provider` matches only the bare token, first occurrence). AC1.3 forbids silent fallback; exit 78 contract bypassed.
Fix: scan all args; accept `--crypto-provider=<v>` or reject any `--crypto-provider`-prefixed arg that is not the exact two-token form; reject a second occurrence; unknown → exit 78 listing known.
Witness: `maos init --crypto-provider=bogus` and `maos init --crypto-provider ring --crypto-provider bogus` must exit 78 with the known list (both exit 0 now, and config.toml written with `ring`).

### P3 (medium) Composition-root structural gate bypassable; negative control covers only the written forms
Sources: BH-03/BH-05, Edge F4, Acceptance L2. `xtask/src/check_composition_root_completeness.rs` visitor (~177-248): misses `type X = RingCryptoProvider;` then `X`, `<RingCryptoProvider as Default>::default()`, `Default::default()` via an annotated type, any occurrence inside macro bodies (`vec!`, `select!`, `macro_rules!`) and `include!`d files; scans only `crates/` (not `spirits/*`); the T3 exemption is the literal `expr.span().start().line == 350` (shifts on any edit above it in `security/mod.rs`; conversely any ctor on line 350 of that fn is exempt). Aliases via `use ... as` and nested cfg(test) modules are handled.
Fix: drop the line literal, exempt by the exact path text `crate::security::crypto::RingCryptoProvider` inside `admit_spirit` only; flag `ItemType` aliases whose type path names `RingCryptoProvider`; flag any `QSelf` type / `syn::TypePath` use in expression context; add `visit_macro` scanning `tokens` for the ident (conservative); reject `include!` in scanned crates or scan its target; extend scan roots to every `src/` tree.
Witness: extend `crypto_construction_in_production_is_refused_after_test_module` with the four shapes above (each yields 0 violations today); and insert a blank line at the top of `crates/maos-kernel-core/src/security/mod.rs` → gate fails today (line 350 moves).

### P4 (medium, OPERATOR DECISION) D-16-2-F empty-vs-populated decays after first boot
Sources: Edge F2, Acceptance M2. `crates/maos-bin/src/main.rs:10850-10866` ignores only THIS boot's selection frame; a selection frame from any earlier boot counts as pre-existing evidence. So `maos uninstall <unknown>` on an otherwise empty home returns exit 4 (`not_found`) on its first boot and exit 5 (`failed`, "unknown Spirit in populated Transparency Log") on every repeat; `not_found` is reachable only on a home's first boot; at HEAD both return 4. Pinned by the author's `prior_boot_provider_selection_still_counts_as_populated_history`.
Recommended fix: exclude ALL kernel-origin `crypto.provider.selected` frames (drop the `boot_nonce = ?1` conjunct) — they are infrastructure; every other row still makes the log populated. Update the author's test to the new expectation (prior-boot frame only → exit 4; prior-boot frame + any other row → exit 5).
Operator decision: keep author reading (selection history counts; document that `not_found` only on first boot) or adopt the recommended reading. Witness: run `maos uninstall ghost` twice on a fresh home: second exit is 5 on a2e37988, 4 required.

### P5 (medium) AC2 "one boot / one identity" witness is true by construction; feature-gated witnesses run only from a branch-only recipe
Sources: Acceptance M3 (Test-infra TI-3 related; TI-infra's "run in CI" statement is refuted: `grep crypto-recording .github xtask/src` hits only `build_test_fixtures.rs`).
Evidence: `tests/support/crypto_integrated_20_13.rs:26-35,168-206,249-253` composes its own world and clones one `Arc`; the shared `crypto-recording.log` lets a second instance planted in `main.rs:3203` pass every real-binary test; `recording_provider_routes_real_at_rest_seal` calls `seal_at_rest` directly (true at HEAD) and is compiled out without the feature; `coverage-matrix.yaml` FR48 cites it as a gate witness.
Fix: (a) add to one real-process test (maos-crypto-recording fixture, one-shot or the door CRL test) an assertion that every line of `crypto-recording-identity.log` carries the same `pid:instance` prefix and distinct ops were seen (TI-3 same); (b) re-label seal test as unit-level, keep the booted-daemon assertion; (c) either run `--features crypto-recording` legs in a blocking job or reword the matrix/verification text ("branch-only recipe, not a gate"). Witness: plant `let crypto_provider = maos_bin::crypto_provider::select(&crypto_home).unwrap().provider;` at main.rs:3203 — all current tests stay green; fix (a) reds on the second `pid:instance`.

### P6 (medium) `maos init --crypto-provider` cannot repair/update a `[crypto]` the boot accepts/refuses
Sources: BH-08, Edge F6, TI-1. `crates/maos-shell/src/lib.rs:807-818` `record_crypto_provider` uses `as_table()` (inline `crypto = { provider = "ring" }` rejected though `select` accepts it) and errors on a `[crypto]` with no `provider` (boot also refuses it, so init, the repair path, refuses too). TI-1: re-selection is only tested ring→ring, so a no-op rewrite ships green.
Fix: `as_table_like()`; a missing `provider` → previous `None`; add a ring→recording test asserting stdout `ring -> recording` and config.toml contents (and that other config keys/comments survive).
Witness: init on a config with `crypto = { provider = "ring" }` then `--crypto-provider recording` fails today; ring→recording rewrite test.

### P7 (low) `maos_home()?.unwrap_or_default()` → CWD-relative `config.toml`
Sources: BH-06, Edge F5, Acceptance L6. `main.rs:1983`. Fix: `select` takes `Option<&Path>` (None → default `ring`, source `default`, no file read), or exit 78/1 when no home. Witness: run `maos <verb>` with `MAOS_HOME` and `HOME` unset in a cwd holding `config.toml` `[crypto] provider="bogus"`: exits 78 today.

### P8 (low) Operator docs / bypass inventory understate ring-direct surface
Sources: BH-04, Acceptance L3/L4. Pending ring-direct daemon callers not named: `maos-registry/src/admission.rs:578`, `yank_policy.rs:100`, `main.rs:193`, `storage.rs:475`, `handlers/publish.rs:13`, `install.rs:96`, `compliance_verify.rs:66`, `spirit-cli/vet.rs:51`, vetting-attestation checks inside `verify_installed`. Stale cites in `bypass-inventory.md` (actual: `crypto_provider.rs:123,131`, `main.rs:15622,15883`). `run-maos.md` (3 copies) must say only boot re-verification of installed packages is provider-routed. Fix: list them in bypass-inventory.md + run-maos copies. Witness: `grep -rn "UnparsedPublicKey\|ED25519" crates/maos-registry/src crates/maos-bin/src` hits not in the inventory.

### P9 (low) Malformed-selection stderr assertion cannot fail
Acceptance L5: asserts a substring "string" which also appears in the word "ring". Fix: assert the full `[crypto] provider must be a string` message and exit code 78. Witness: change `select`'s message to anything containing "string"; test stays green.

### P10 (low) Refused installed-boot leg does not pin exit code
TI-6, `crypto_provider_20_13.rs:283-291`. Fix: assert the typed boot-refusal code like sibling vectors.

### P11 (low) Frame `version` never asserted
TI-2, `crypto_provider_20_13.rs:340-341`. Fix: `payload["version"] == validate("ring").unwrap().version`.

### P12 (low) cgroup oracle edge arms
TI-4, `cgroup_ceiling_smoke.rs:684-686`: accepts `None` (unreachable from the real call site) but rejects lowercase `x` (EXIT_DEAD). R/S/D still rejected (verified by layer + unit test). Fix: `None | Some("Z" | "X" | "x")` or drop `None`. Not lax regarding running/sleeping.

### P13 (low) Bookkeeping
- TI-5: `crypto_provider_20_13.rs:462` 45 s internal deadline > 30 s default budget; lower to 30 s or add owned exception in `xtask/test-budgets.toml`.
- Acceptance L7: extend `SCANNED_SOURCE_FILES` history comment.
- Acceptance L8: `xtask/gate-registry.toml` row added despite D-20-13-08 A ("no registry row") and placed under the "Advisory only" comment though the gate is blocking: record the deviation (or move), operator confirm.
- BH-07/Edge: `crypto-recording` compile_error keys on `debug_assertions`; a release profile with `debug-assertions = true` compiles it. No such override exists in any Cargo.toml (checked) and the `RecordingCryptoProvider` symbol gate in `check_mock_not_in_release` is the backstop; optional hardening: also `compile_error!` under `cfg(not(debug_assertions))` is already present, add a gate test that builds `--release --features crypto-recording` with `debug-assertions=on` expecting the symbol gate to fire.

## DEFER
- D1: `journal()` discards the `insert_frame_event` result (`crypto_provider.rs` `let _token =`): selection-frame loss is silent. Low; consider failing boot with exit 78/1 when the frame cannot be appended (AC wording "journaled"). Decide at L2 with the kernel landing.
- D2: Ring-direct admission/yank/publish verification (P8 list) migrates to the provider in a later landing (D-20-13-17 ruled).
- D3: Kernel T3 `admit_spirit` ring construction remains the named L2 exception (AC7).

## DISMISS
- X1 erasure: D-16-2-F empty-log vector still passes (empty log → exit 4; populated log → exit 5; this boot's frame ignored). The concern is only the decay in P4.
- X2 ring-direct half of the D-20-13-17 split: byte-identical to HEAD (`evaluate_envelope_at` passes `&verify_ed25519`, `unpinned_empty=false`).
- X3 one provider instance per boot in `main.rs`: single `select`, every consumer `Arc::clone`; no new non-test ring constructions (grep checked: remaining hits in cfg(test) modules/docs).
- X4 `ci_shape_18_0` counts (20 wasm-host recall, 7 recall-wire) derived from source via syn with per-token negative controls; match `discipline.yml`.
- X5 No-default-features/macOS: `crypto_provider` module is unconditional but depends only on always-on `toml`/`serde_json`/kernel-core; `crypto-recording = ["network"]`; registry→kernel-core is dev-dependency only (no closure change). `serde` unwrap/expect: none added in production paths (errors mapped to String).
- X6 No kernel edit; `docs/invariants/*.md` untouched; unwrap/expect only in tests.

## Operator decisions needed
1. P4: ruling on prior-boot selection frames counting as "populated" (recommended: exclude all kernel `crypto.provider.selected` frames; test + D-row reword).
2. P5: whether the `--features crypto-recording` witnesses must run in a blocking gate or be re-labelled branch-only (matrix text).
3. P13: `gate-registry.toml` row vs D-20-13-08 A (record deviation).
4. D1: journal-failure policy (defer to L2 unless operator wants fail-closed now).
