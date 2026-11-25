# Story 23-8 evidence (agent Dev238, host: local, branch e23-8-kms-adapter-2026-11-25, stacked on PR #59 head bfe9d997)

Env for every cargo command: `unset CI GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE; CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-maos-e238 MAOS_FIXTURE_DIR=/mnt/build/cargo-maos-e238/maos-fixtures`;
fixtures built with `cargo run --locked -q -p xtask -- build-test-fixtures` (18 artefacts, re-run after the plants because the fixture
freshness check keys on source mtimes).

Rulings applied: D-23-8-06…10 CONFIRMED A. **D-23-8-11 is PROPOSED A and was implemented as A**: the endpoint is operator configuration,
`https` (or `http` to a literal loopback address), no redirect, vetted once at boot, **no host-name SSRF claim** anywhere (ADR-051 amendment,
RELEASE-HOLDS 23-8.c, `docs/reference/kms-wire-v1.md`, the sidecar README).

## Build shapes (`cargo check --locked -p maos-bin <shape>`, all after the change)

| Shape | Result |
|---|---|
| default | exit 0 |
| `--features network` | exit 0 |
| `--features wasm-host` | exit 0 |
| `--no-default-features` | exit 0 |
| `--no-default-features --features network,wasm-host` (the 23-0 profile without `online-adapters`) | exit 0 |
| `--no-default-features --features network` | exit 0 |
| `cargo check --locked -p maos-secrets` / `--features http-kms` | exit 0 / exit 0 |
| `cargo run --locked -q -p xtask -- check-composition-root-completeness` | PASS (11 adapters, 20 constructions) |

The wiring test file also ran in the profile without `online-adapters`
(`cargo test --locked -p maos-bin --no-default-features --features network,wasm-host --test external_kms_wiring_23_8 --test online_adapters_23_0`):
9 + 3 passed; there `MAOS_KMS_ENDPOINT` is refused at boot with `kms_endpoint_unsupported_build` (asserted through a booted subprocess).

## Measured

| Item | Before (HEAD bfe9d997) | After | Note |
|---|---|---|---|
| kloc `maos-secrets` | 536 | 934 (+398) | forecast +250…+420: inside. `kloc-check` reports `over_budget: maos-secrets 934 > 636` and `passed:false` (the per-crate ceiling the operator ruled removed, decision 11, has not been blessed away: NOT touched, orchestrator applies) |
| kloc `maos-bin` | 44,851 | 45,176 (+325) | forecast +80…+180: **over the forecast by ~145** (see deviations) |
| kloc `maos-domain` | 11,174 | 11,178 (+4) | forecast +5…+15: at the floor (`SecretKey::KmsBearer`) |
| kloc `maos-kernel-core` | 22,143 | 22,143 (0) | alarm 23,000 / row 24,000: unchanged; `check-kernel-baseline --json`: passed, 28,324 = pinned 28,324, no changed/added/removed file, set-hash equal. No re-pin. |
| kloc aggregate (HEAD side measured from `git archive` of `crates xtask examples`, so spikes/templates = 0 there) | 216,073 | 216,955 (216,800 on the same basis: +727) | |
| stripped release binary (`cargo build --locked --release -p maos-bin --bin maos`, `strip -o`) | 38,524,752 B (23-0 record) | 38,651,088 B (+126,336 B) | limit 39,069,022 B: headroom 417,934 B. The 23-0 figure was measured by another agent, same method; not re-measured at HEAD here. |
| env contract (`check-env-contract`) | 159 | **162** | registered in `crates/maos-bin/src/env_contract.rs`: `MAOS_KMS_ENDPOINT`, `MAOS_KMS_BEARER`, `MAOS_KMS_ALLOW_PRIVATE`; `MAOS_KMS_MASTER_KEY` and `MAOS_SECRETS_BACKEND` purposes amended. PASS, 0 violations. |
| `Cargo.lock` | | `maos-secrets` gains `base64`, `serde_json`, `ureq` (all already locked: base64 0.22.1, ureq 2.12.1) | no new package, no version bump. No standalone lockfile contains `maos-secrets` (`grep -rl 'name = "maos-secrets"' --include=Cargo.lock`: root only), so native-twin and the fuzz locks need nothing; `build-test-fixtures` (`--locked`) passes. |
| `cargo deny check` | | `advisories ok, bans ok, licenses ok, sources ok` | |
| `cargo run --locked -q -p xtask -- check-enterprise-identity` | | PASSED, oracle green (10 legs), including `release-graph-absence` and `fault-inject-falsifiers` | |
| `check-test-cost` | | PASS | no `[[exception]]` row; every new test binary runs in under 3 s |
| `cargo fmt --all` | | clean (re-run after the last edit) | |
| clippy | | `maos-secrets` (`--features http-kms --lib --test external_kms_23_8 --no-deps -D warnings`): clean; `maos-bin --test external_kms_wiring_23_8 --no-deps`: no diagnostic in any 23-8 file | Pre-existing, not touched: `maos-spirit-abi` 4 `-D warnings` errors (block `-D warnings` on every crate that depends on it unless `--no-deps`); `maos-secrets` tests `at_rest_seal.rs:26` and `default_plaintext_preserved.rs:42` unused imports; `maos-domain` (`collective_memory.rs:109`, `audit_key.rs:349`, `:356`); `maos-bin` `enterprise_identity.rs` test module (`:901`…`:1072`: doc-list indentation, `type_complexity`) and `env_contract.rs` dead-code warnings under `network` only |

## Tests (all pass)

| Command | Result |
|---|---|
| `cargo test --locked -p maos-secrets --features http-kms --test external_kms_23_8` (the AC1 command) | 22 passed (17 before the review round, +5) |
| `cargo test --locked -p maos-bin --test external_kms_wiring_23_8` (the AC1 command) | 19 passed (17 before, +2). Profile without `online-adapters` (`--no-default-features --features network,wasm-host`): 10 passed; the `online::` module and three online-only boot tests compile out |
| `cargo test --locked -p maos-secrets` (and with `--features http-kms`) | at_rest_seal 7/9, default_plaintext_preserved 2, kms_fault_inject 1, all green |
| `cargo test --locked -p maos-bin --test enterprise_identity_wiring --test online_adapters_23_0 --test maos_uninstall_16_4 --test crypto_real_boot_20_13 --test researcher_8_14c` | 4, 3, 24, 1, 7 passed |
| `cargo test --locked -p maos-bin --lib` / `--bin maos story_13_5a` | 32 passed / 3 passed (the cohort KMS-down refusal test is unchanged and green) |
| same set with a random 0600 hex `MAOS_AUDIT_KEY` exported (key file deleted afterwards) | all green (an earlier run failed `crypto_real_boot_20_13` only because the fixture binary was stale after my plants: "test fixture stale"; rebuilt, green) |
| `external_kms_23_8` x10 and `external_kms_wiring_23_8` x10 under 16 busy loops (`while :; do :; done`, load average 5 at the start, 13.8 at the end), review round | 10/10 and 10/10 pass |

No test sleeps or asserts elapsed time. The never-answering peer is asserted under an outer 30 s deadline; the probe task is asserted by waiting
for three probes and then for the join, both under 30 s outer deadlines; the fake clock is advanced by hand.

## AC map

| AC | Tests (file::name) | Proven red |
|---|---|---|
| AC1 (a)(b) | `external_kms_23_8::seal_wraps_through_the_endpoint_once_and_open_unwraps_through_it` | `proven-red/AC1a-wrap-returns-the-data-key.txt` |
| AC1 (c) | `external_kms_23_8::an_encrypted_file_store_over_the_endpoint_round_trips_a_credential`, `::with_the_endpoint_down_a_read_is_an_error_never_plaintext` | (same file: the plant fails the round trip) |
| AC1 (d)(e) | `external_kms_wiring_23_8::online::with_the_endpoint_down_sealed_writes_are_refused_and_resume_when_it_returns` | `proven-red/AC1d-skip-the-health-check.txt` |
| AC2 | `external_kms_wiring_23_8::no_vendor_string_is_in_the_release_sources` (no `transit/`, no `X-Vault-Token` under `crates/maos-secrets/src` and `crates/maos-bin/src`), `::the_reference_sidecar_passes_its_own_self_test` (`examples/kms-sidecar-openbao/selftest.py` against a stub Transit); `release-graph-absence` green | none required |
| AC3 | `external_kms_23_8::a_redirect_is_a_refusal_and_is_never_followed`, `::a_peer_that_never_answers_makes_wrap_unavailable`, `::the_bearer_goes_only_on_wrap_and_unwrap_and_never_into_errors_or_debug`, `::a_response_over_the_64_kib_cap_is_an_error`, `::an_endpoint_must_be_a_plain_base_url`; `external_kms_wiring_23_8::online::http_is_admitted_only_to_a_literal_loopback_host`, `::https_passes_the_outbound_policy_and_private_ranges_need_the_opt_in`, `::a_url_that_could_carry_a_credential_is_refused`, `::boot_refuses_a_conflict_a_remote_http_endpoint_and_a_stray_variable` | `proven-red/AC1a-…txt` plant 2 (timeout), `proven-red/AC3-drop-the-loopback-check.txt` |
| AC4 | `external_kms_23_8::requests_carry_the_version_and_a_32_byte_key_and_compare_by_key`, `::a_malformed_answer_is_a_malformed_payload_and_flips_health`, `::an_oversized_wrapped_key_is_refused_before_any_request`, `::a_typed_422_is_a_crypto_refusal_and_leaves_health_alone`; the wire is `docs/reference/kms-wire-v1.md` | none required |
| AC5 | `external_kms_23_8::a_stopped_prober_goes_unhealthy`, `::is_healthy_does_no_io`, `::a_5xx_flips_health_down_at_once_and_a_probe_restores_it`, `::an_unreachable_endpoint_is_unavailable_and_a_probe_after_restart_restores_health`; `external_kms_wiring_23_8::the_probe_task_runs_until_the_shutdown_token_fires` | `proven-red/AC5-staleness-ignored.txt`; extra: `AC1a-…txt` plant 3 (a 422 that flips health) |
| AC6 | `external_kms_wiring_23_8::both_endpoint_and_master_key_is_a_conflict_naming_both_variables_and_no_value`, `::a_kms_variable_without_a_key_source_is_refused_and_no_variable_means_plaintext_default`, `::boot_refuses_a_conflict_a_remote_http_endpoint_and_a_stray_variable` (booted subprocess, scrubbed env, exit 78; in the profile without `online-adapters` it asserts `kms_endpoint_unsupported_build`), `::without_any_kms_variable_the_default_is_unchanged`; `default_plaintext_preserved.rs` green; `check-env-contract` PASS (162) | none required |
| AC7 | `external_kms_wiring_23_8::online::a_kms_down_at_boot_never_yields_a_plaintext_collective_row`, `::a_configured_down_runtime_has_a_refusing_hook_never_none` (regression of the old `None`), `::an_unusable_master_key_is_a_refusal_not_a_silent_downgrade`, `::a_malformed_master_key_refuses_the_boot_instead_of_booting_plaintext` (booted Host: exit 78, never reaches the runtime line), `::an_unreachable_endpoint_is_not_a_boot_refusal`; the cohort check and its existing test stay | `proven-red/AC7-hook-early-return.txt`, `proven-red/D-23-8-07-unusable-key-boots.txt` |
| AC8 | `external_kms_wiring_23_8::encrypted_file_works_over_the_endpoint_and_a_down_endpoint_never_falls_back_to_the_environment` (booted Host, `MAOS_SECRETS_BACKEND=encrypted-file` + endpoint + bearer: the provider registers from a credential sealed through the fixture; the fixture saw `POST /unwrap` with `Authorization: Bearer <canary>`; then with the endpoint stopped and `MAOS_ANTHROPIC_API_KEY` set no provider registers); `maos_uninstall_16_4::…` extended: the purge receipt names `keyring://maos/kms-bearer`; `SecretKey::ALL` is 13 (`at_rest_seal.rs`) | none required |
| AC9 | ADR-051 amendment + `docs/adr/index.md`; `RELEASE-HOLDS.md` Epic 23 rows 23-8.a…f; runbook `ent-1`; both `run-maos.md`; `crates/maos-secrets/src/lib.rs` crate doc; this file | none required |

## Proven reds

Every plant carried the marker `E238-PLANT`, was run once, and was reverted. After the last revert
`grep -rn E238-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask examples` returned nothing (the marker appears only in this directory).

| File | Plant | Fails |
|---|---|---|
| `AC1a-wrap-returns-the-data-key.txt` | `wrap_data_key` returns the key; `TimedHttp` has no timeouts; a 422 flips health | 7 tests; the AC1 (a) assertion on the envelope; `a_peer_that_never_answers…` at its outer deadline; `a_typed_422…` |
| `AC1d-skip-the-health-check.txt` | health check skipped in `seal_at_rest` and `seal_row_at_rest` | `with_the_endpoint_down_sealed_writes_are_refused_and_resume_when_it_returns`; in maos-secrets `a_stopped_prober_goes_unhealthy` |
| `AC3-drop-the-loopback-check.txt` | plain-http refusal removed in `vet_url` | `http_is_admitted_only_to_a_literal_loopback_host`, `boot_refuses_a_conflict_a_remote_http_endpoint…` |
| `AC5-staleness-ignored.txt` | `is_healthy` ignores staleness | `a_stopped_prober_goes_unhealthy` only |
| `AC7-hook-early-return.txt` | `if !kms.is_healthy() { return None }` restored in `at_rest_seal_hook` | `a_kms_down_at_boot_never_yields_a_plaintext_collective_row`, and the (d) test |
| `D-23-8-07-unusable-key-boots.txt` | an unusable master key no longer refuses the boot | `an_unusable_master_key_is_a_refusal…`, `a_malformed_master_key_refuses_the_boot…` |

Two honest notes on the plants. (1) "Skipping the health check must fail (d)" is not true of the literal (d) (endpoint stopped): the wrap
request fails on the transport and the write is refused anyway. (d) was sharpened (restart the endpoint, do not probe, the write must still be
refused) and now fails under the plant. (2) "Wrap returns the data key unchanged must fail (a)": `sealed != plaintext` alone stayed green
(the row is still real ciphertext); the plant failed at `open_at_rest`, and (a) now also asserts that the wrapped key in the envelope is longer
than the 32-byte data key.

## Deviations from the story as written (numbered)

1. **New env name `MAOS_KMS_ALLOW_PRIVATE`** (not listed as a certainty by the story; AC3 says "private … only with its explicit opt-in" and AC6 "and the policy opt-in if 23-0 makes it per adapter"). 23-0 has the `allow_private` knob but no env name, and a KMS in a private network is the common case, so the opt-in is per adapter here: `1`/`true` only; anything else is off. Registered (env count 159 → 162).
2. **`maos-bin` kloc +325 against a forecast of +80…+180.** The boot-refusal enum with its `Display`, `ConfiguredKms`, the bounded resolver, the probe task and the runtime accessors are larger than forecast. No ceiling is enforced (decision 11) and the number is reported, not trimmed.
3. **D-23-8-08 ordering made concrete.** The story says the KMS is built before the secret store AND the bearer is read through the configured store; those conflict (the store is built later in `main()`). Resolution: the adapter is built once before the runtime and the store and handed to both (`HttpKms::pending_bearer`); it refuses every call and reports unhealthy until `provision_bearer` runs right after the secret store exists, reading through the configured store, or through the env store under `encrypted-file`. A store read error leaves it unprovisioned (fail closed) and logs the error without a value.
4. **`/health` carries no bearer.** `IoSubsystemPort::http_get` has no request headers (story inventory) and a kernel/domain port change is out of scope; `/health` is a liveness probe without a credential (`kms-wire-v1.md`). `/wrap` and `/unwrap` carry it.
5. **The bearer and the unwrapped data key are not zeroized.** `maos-secrets` is `#![forbid(unsafe_code)]` and has no `zeroize` dependency; adding one is a new dependency. Held in a `String` / `Vec<u8>` and dropped normally.
6. **Probe task spawned once, early.** AC5 says "beside `spawn_siem_export_consumer`". The task is spawned right after the boot probe (with the serve loop's `cancel` token, which moved up from after the one-shot branch) so that a slow boot cannot outlive the 30 s staleness bound and so that the cohort one-shot daemon, which runs inside the same `main`, shares it. Same token; one spawn instead of two.
7. **`maos-secrets` feature `http-kms` also enables `encrypted-file`**, so the AC1 command (`--features http-kms` only) can run AC1 (c). maos-bin enables both anyway.
8. **`RELEASE-HOLDS.md` has no "row 22" that mentions the plaintext-at-boot path** in this branch (the Epic 22 rows here are 22-4 and 22-6.a…e and none mentions KMS). Instead of editing a row, the Epic 23 section gains row 23-8.f stating that the fail-open is fixed by 23-8 and only by 23-8. If row 22 lives on the PR #55 line, the integration owner updates it there.
9. **The cohort daemon no longer refuses to start when the external endpoint is merely unhealthy at boot** (it refuses for `ConfiguredDown` / missing adapter, `kms_unbuilt()`, as before). Previously "unhealthy at boot" made the hook `None` and the cohort refused; with D-23-8-07 A an unreachable endpoint refuses writes instead. The existing cohort test (configured-down runtime refuses) is unchanged and green.
10. **`maos-bin` test `online_adapters_23_0.rs` edited**: the umbrella's member list is now `["network", "maos-secrets/http-kms"]` (D-23-8-09 A makes the umbrella enable the KMS feature); the test asserted exactly `["network"]`.
11. **Encrypted-file refusal text** keeps the phrase `requires a healthy` and `MAOS_KMS_MASTER_KEY` (an existing test matches them) and adds `configured KMS` / `MAOS_KMS_ENDPOINT`.

## Not verified here (hosted CI or operator)

- macOS check; packaging (deb/rpm/AUR/brew), cgroup and systemd legs; the air-gap/namespace witness (23-6); the job-wide `MAOS_AUDIT_KEY` export (only a per-command random-key run was done); fuzz-target builds (no fuzz lockfile contains `maos-secrets`, so none were touched; not built).
- A live run of the sidecar against `bao server -dev` (an operator leg by the story, never gated); only the stub-Transit self-test ran.
- Prepush (`xtask prepush`) and `docs:build` were not run (docusaurus is not installed on this host, as recorded by 23-0).
- Windows: `HttpKms` and `TimedHttp` are target-independent; no Windows build was attempted.
- Release size was measured once on this host against 23-0's recorded figure, not against a re-measured HEAD.

## Independent security review round (agent Fix238)

Findings of the independent review, what was done, and the proof. Plants carry the marker `E238-PLANT`, each was run once and reverted
(`grep -rn E238-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask examples` returns nothing); records are in `proven-red/P3-*.txt`.

| Finding | Result | Where | Proof |
|---|---|---|---|
| P2-1 identity rows are not sealed | **Claim fixed** (no code change): "and identity rows" removed; one-line boundary "identity assertions in the Transparency Log are not sealed by this story" added | RELEASE-HOLDS 23-8.a, ADR-051 amendment (Limits), `kms-wire-v1.md` (What this does not do), runbook `ent-1` §At-rest, sidecar README, `maos-secrets` crate doc, `enterprise_identity.rs` module doc | `grep -rn 'identity rows\?' docs RELEASE-HOLDS.md crates examples` finds no sealing claim. The ADR's 11.4c scope sentence ("plus out-of-kernel identity assertion writes", line 30) is original text; the amendment Limits now says it is design intent, not shipped behaviour |
| P2-2 loopback `http` does not authenticate the listener | **Documented** as a required deployment condition (unclaimable port, or same OS user with the port bound before MAOS starts, or TLS in front; Unix-socket owner check not implemented, D-23-8-01 B not chosen). No code change | RELEASE-HOLDS 23-8.b, `kms-wire-v1.md` Transport, runbook `ent-1`, sidecar README, ADR amendment Transport | docs only |
| P2-3 no private-CA `https` | **Confirmed and documented**: `ureq` 2.12.1 with `default-features = false, features = ["tls"]` links `webpki-roots` only (`Cargo.lock` ureq deps: base64, flate2, log, once_cell, rustls, rustls-pki-types, url, webpki-roots; `rtls.rs` `root_certs()` uses `webpki_roots` unless `native-certs`). Handshake against an internal PKI fails and reads as unavailable (writes refused). **Open decision: a CA-bundle setting** | new RELEASE-HOLDS row 23-8.g, wire doc, runbook, sidecar README, `http_kms.rs` module doc | docs only |
| P2-4 DNS lookup is not bounded | **Confirmed and documented**: `ureq-2.12.1/src/stream.rs:364` `// TODO: Find a way to apply deadline to DNS lookup`. The 5 s / 2 s / "overall 5 s" claims now read "for a literal-IP endpoint"; a host name can block longer, fail-closed still holds (health stale after 30 s). **Open limit, no fix** | RELEASE-HOLDS 23-8.d, wire doc Transport and Health, ADR amendment Transport, runbook, sidecar README, `Timeouts` and crate docs | docs only. **Async worker (read, not changed):** yes. `LoomLiteStore::write_with_source_attested_at` is an `async fn` (`store.rs:605`) and calls `self.at_rest_sealer.seal(...)` inline at `store.rs:648` (no `spawn_blocking`; the only `spawn_blocking` in that file is a doc comment, line 7); `replication/bundle.rs:~508` awaits `store.write_native_with_source_attested` the same way. `HttpKms::wrap_data_key` is a blocking `ureq` call, so a stalled DNS lookup on a host-name endpoint stalls a tokio worker for as long as the lookup takes; for a literal IP the bound is 5 s. The boot probe and the periodic probe use `spawn_blocking` (not affected). Recorded in row 23-8.d |
| P2-5 `MAOS_KMS_ALLOW_PRIVATE` docs | **Docs fixed to match the code**: with the opt-in, private AND loopback addresses are admitted, including a name that resolves to them, and the connection re-resolves so it can move between them; link-local/reserved never; `http` stays literal-loopback only | env contract text (`env_contract.rs`, count stays 162), `kms-wire-v1.md`, sidecar README, runbook, RELEASE-HOLDS 23-8.c, ADR amendment | new test `external_kms_wiring_23_8::online::with_the_opt_in_private_and_loopback_addresses_are_admitted_including_names` (pins both directions: refused without the opt-in; `https://localhost:8443` with names resolving to loopback, private and both is admitted with it; link-local still refused; `http://localhost` still refused). No plant (test of existing behaviour) |
| P3-1 silent bearer failures, misleading lines | **Fixed.** `HttpKms` gains `BearerStatus` (Pending/Unreadable/Settled), `bearer_unreadable()`, `calls_failing()` and a notice sink (default stderr): one line per change of failure class (bearer pending, bearer unreadable, 401, 403, redirect, other 4xx, 5xx, transport, malformed), one line on recovery, never a value, URL or body. `main.rs` marks a failed bearer read unreadable and says so; the probe returns `KmsProbeOutcome` (Unreachable / ReachableBearerPending / ReachableBearerUnreadable / ReachableCallsFailing / Ready) and only `Ready` prints "sealed writes resume"; the boot line no longer claims more than "reachable" while the bearer is pending. `KmsProbeFn` now returns the outcome (callers and tests migrated) | `http_kms.rs`, `enterprise_identity.rs`, `main.rs` | `external_kms_23_8::a_failed_call_is_logged_once_per_change_with_its_status_class_and_never_a_secret`, `::a_pending_or_unreadable_bearer_is_logged_once_and_sends_nothing`; `external_kms_wiring_23_8::online::the_probe_outcome_follows_the_bearer_and_no_line_claims_writes_resume_until_calls_work`, and the booted `encrypted_file_works_over_the_endpoint_...` asserts "external KMS bearer read". Plants A and B in `proven-red/P3-1-...txt` (both red) |
| P3-2 `mark_ok` can clear a concurrent `down` | **Fixed** with an epoch: `probe()` takes `Health.epoch` before its I/O, `mark_down` bumps it, `mark_ok(epoch)` applies only if unchanged | `http_kms.rs` | `external_kms_23_8::a_probe_in_flight_cannot_clear_a_failure_recorded_after_it_started` (deterministic: the failing wrap runs inside the probe's scripted transport; no thread, no sleep). Plant `proven-red/P3-2-...txt` (red) |
| P3-3 probe timeout claim | **Fixed in code** (the preferred option): `Timeouts.probe` (default 2 s), applied per request on `TimedHttp::http_get`; wrap/unwrap keep 5 s. A running probe still cannot be cancelled, so a shutdown can wait for it up to 2 s for a literal IP (documented in the wire doc) | `http_kms.rs` | `external_kms_23_8::the_probe_has_its_own_overall_limit_and_a_hanging_endpoint_fails_it` (probe limit 400 ms, every other limit 120 s, outer 30 s failure bound only). Plant `proven-red/P3-3-...txt` (red at the 30 s bound) |
| P3-4 last wiring test too weak | **Fixed**: the down-endpoint boot must also print "external KMS endpoint UNREACHABLE at boot" and reach `ProviderRateLimiter initialized` (past provider resolution); the up boot must print "external KMS bearer read" | `external_kms_wiring_23_8.rs` | Plant `proven-red/P3-4-...txt` (an early config conflict, exit 78, passed the old assertion and fails the new ones) |
| P3-5 fixtures never contained the bearer | **Fixed**: fixture `Mode::EchoAuthorization(status)` reflects the `Authorization` value in the body, `X-Echo` and `Location`; the new test drives 200/401/403/422/500/302 through wrap, unwrap and the bare transport and asserts the value is in no error, `Debug` or notice | `kms_fixture_23_8.rs`, `external_kms_23_8.rs` | `external_kms_23_8::a_response_that_echoes_the_bearer_never_reaches_an_error_a_debug_or_a_log_line`. Plant `proven-red/P3-5-...txt` (body copied into the error; red; the older bearer test stayed green under it, which was the finding) |
| P3-6 example sidecar | **Fixed**: `opener()` = `build_opener(ProxyHandler({}), _NoRedirect())` built per call (a 3xx from OpenBao is a 502; proxy variables ignored); `Handler.timeout` (10 s) bounds each client socket; `log_message` tolerates a request that never parsed. Self-test adds redirect trap, proxy trap and stalled-client arms; README notes it | `kms_sidecar.py`, `selftest.py`, `README.md` | `python3 selftest.py` ok; `the_reference_sidecar_passes_its_own_self_test` (wiring file). Plant `proven-red/P3-6-...txt` (default opener; red) |

Behaviour changed by the fixes, beyond the findings: `provision_bearer` and `bearer_unreadable` clear the remembered failure (found by the new
probe-outcome test: a wrap refused while the bearer was pending would otherwise have made the probe report "calls failing" after provisioning).

### Open decisions and limits (for the operator; none was changed)

- **CA-bundle setting** for private-CA `https` (RELEASE-HOLDS 23-8.g). `TimedHttp` could take a `rustls` config; that is a decision, not done here.
- **DNS lookup is outside every timeout** (ureq 2.12 limitation). A resolver hook or a pre-resolved literal address would fix it; not scheduled.
- **Loopback `http` listener authentication** (D-23-8-01 B, a Unix-domain-socket owner check, was not chosen): documented as a deployment condition.
- Identity assertions in the Transparency Log are **not sealed**; sealing them needs a decision (the helper `append_identity_asserted` writes plain JSON and `seal_row_at_rest` has no production caller).

### Deviations that need the operator's ruling (unchanged, listed accurately)

1. **#1** — new env name `MAOS_KMS_ALLOW_PRIVATE` (env count 159 → 162). Its documented meaning is now the code's: private **and** loopback.
2. **#2** — `maos-bin` kloc +325 against a forecast of +80…+180 (this review round adds to it: the `KmsProbeOutcome` type, the bearer failure branch and tests; not re-measured).
3. **#4** — `/health` carries no bearer. The review notes that `IoSubsystemPort::http_post_exchange` already carries headers, so a `POST /health` with the bearer would need no port change; it would be a wire change (`maos.kms.v1`) and needs a ruling. Until then a sidecar's `/health` is unauthenticated and the probe cannot detect a wrong bearer: the new per-call notices (401/403) are what reveals it.
4. **#5** — no zeroize of the bearer and the unwrapped key. The stated reason ("no `zeroize` dependency") is **wrong**: `zeroize` is already in `Cargo.lock`. The real choice is a new dependency edge on `maos-secrets`, which needs a ruling.

### Not verifiable here, hosted CI only

The job-wide `MAOS_AUDIT_KEY` export, macOS/Windows shapes, packaging, cgroup/systemd legs, and `docs:build` (docusaurus not installed). A live
OpenBao run of the sidecar (stub Transit only). The behaviour of a real unreachable *host name* (DNS stall) and a real private-CA handshake were
read from ureq's source and `Cargo.lock`, not exercised.

## Second review round (GPT static review; agent Fix238)

Tests after this round: `external_kms_23_8` 23 passed (+1), `external_kms_wiring_23_8` 20 passed (+1). Plants in `proven-red/KMS-1-and-KMS-3.txt` (all red, all reverted).

| Finding | Result | Where | Proof |
|---|---|---|---|
| **KMS-1 (P1)** the endpoint can return the data key as `wrapped` | **Fixed (refuse obvious disclosure only).** `wrap_data_key` refuses, with `KmsError::Crypto` (no bytes in the error, health unchanged, one log line, no envelope, no plaintext fallback), a `wrapped` that contains the data key bytes, is a prefix or suffix of them (this covers equality), or contains their base64 (standard/URL-safe, padded or not) or hex (either case) text. **Confidentiality requires a TRUSTED wrapping endpoint**: an opaque response cannot prove encryption; a trivially derived value (reversed, XORed, truncated and re-keyed) is NOT detected. Stated in RELEASE-HOLDS new row 23-8.h, `kms-wire-v1.md` (Messages and Errors), here | `http_kms.rs` (`discloses`, `Failure::Disclosure`) | `external_kms_23_8::a_wrapped_key_that_discloses_the_data_key_is_refused_and_nothing_is_sealed` (hostile fixture `Mode::HostileWrap` x 7 disclosures, each also through `seal_at_rest`). Plant KMS-1 (red) |
| **KMS-3 (P3)** `probe()` reported the HTTP result when `mark_ok` discarded it, so the wrapper logged "sealed writes resume" | **Fixed.** `mark_ok` returns whether it applied; `probe()` is now true only when the answer was healthy **and** applied; new `probe_checked() -> ProbeResult {Unreachable, Discarded, Applied}`; `enterprise_identity::probe_outcome(result, bearer, healthy, calls_failing)` maps it: `Discarded`, or `Applied` with the adapter unhealthy or calls failing, is `ReachableCallsFailing`, never `Ready` | `http_kms.rs`, `enterprise_identity.rs` | `external_kms_23_8::a_probe_in_flight_cannot_clear_a_failure_recorded_after_it_started` (scripted transport, fake clock, no sleeps; now asserts `Discarded`), `external_kms_wiring_23_8::online::a_discarded_or_unhealthy_probe_is_never_reported_as_ready`. Plants KMS-3a and KMS-3b (red) |
| **KMS-2 (P1)** `seal_at_rest` persists whatever `wrapped` the endpoint returned without an unwrap check; `EncryptedFileSecretStore::store` can replace an existing credential with an unreadable envelope | **DECISION PENDING (operator). Not implemented.** Options A/B/C below; RELEASE-HOLDS 23-8.h states the limit meanwhile | none | none |

### KMS-2 decision pending: options (nothing below is implemented)

Facts: `seal_at_rest` (`crates/maos-secrets/src/lib.rs`, ~l.472-510) wraps, then seals with the data key and returns the envelope; its callers are `EncryptedFileSecretStore::store` (~l.321-341, `tempfile.persist` over the existing `<key>.sealed`), the loom-lite at-rest hook (`EnterpriseRuntime::at_rest_seal_hook`, `main.rs` serve path) and `seal_row_at_rest` (no production caller). `open_at_rest` (~l.538) already does unwrap + AEAD open. kernel-core is untouched by every option (0 kloc, baseline unchanged).

| | A: verify by unwrap-before-commit, encrypted-file credential store only | B: verify in every `seal_at_rest` | C: accept and document |
|---|---|---|---|
| Files / functions | `maos-secrets/src/lib.rs`: `EncryptedFileSecretStore::store` (or a small `seal_at_rest_verified` helper beside `seal_at_rest`): after sealing, `open_at_rest(self.kms, &sealed)` and compare with the value before the temp-file write. Tests in `external_kms_23_8.rs` + fixture | `maos-secrets/src/lib.rs`: `seal_at_rest`, after `wrap_data_key`: `kms.unwrap_data_key(&wrapped)?` must equal `data_key` (constant-time compare), else `KmsError::Crypto`. Same fixture/tests, plus the Collective path through `at_rest_seal_hook`; existing call-count assertions (`/wrap` once; unwrap counts in the AC1 and bearer tests) change | docs only (RELEASE-HOLDS 23-8.h already says it) |
| kloc | maos-secrets +10..+20 (code), tests +60..+90; maos-bin 0; maos-domain 0; kernel-core 0 | maos-secrets +8..+15, tests +80..+120 (more call-count churn); maos-bin 0; kernel-core 0 | 0 |
| Latency | +1 sidecar round trip (`POST /unwrap`, ≤5 s, usually <10 ms loopback) per credential write; credential writes are rare operator actions | +1 round trip per sealed row, on the Collective write path, which runs inline on an async runtime worker (RELEASE-HOLDS 23-8.d): doubles the KMS calls, and for a host-name endpoint doubles the unbounded-DNS exposure. Local master key: negligible | none |
| Tests / plants | fixture mode whose `/wrap` returns an unusable `wrapped` (base64 of one byte) and whose `/unwrap` refuses; assert `store` returns `Err`, the previous `<key>.sealed` is byte-identical, no temp file left; a second case where `/unwrap` returns a different 32-byte key. Plant: skip the verify (red: the file is replaced) | same fixture; assert `seal_at_rest` and the hook return `Err` and no envelope; call-count updates; plant: skip the verify (red) | none |
| What it buys | an acknowledged credential update can no longer become unreadable because of a bad `wrapped`; the one place a write destroys an existing durable secret | every sealed write is proven readable at write time; also catches an endpoint that wraps with a key it cannot later unwrap | nothing; an endpoint-caused unreadable write is accepted data loss |
| Residual | a Collective row can still be unreadable (today no reader exists, so nothing reads them) | an endpoint can still unwrap now and refuse later (rotation, outage, policy); a "verified" envelope is only as good as the endpoint's later behaviour | the credential case is a real loss |

**Recommendation: A.** It closes the only path that overwrites an existing secret, costs one call on a rare operation and ~15 lines, and leaves the hot Collective path alone. Collective rows are new rows nobody can read back yet (row 23-8.a), so B adds latency and an inline blocking call on the async worker for no present benefit; revisit B when a Collective reader story lands. C is defensible only if credentials are never stored under `encrypted-file` with an external endpoint, which is exactly the AC8 configuration.

### What only hosted CI can confirm (this round)

Same as the first round (job-wide `MAOS_AUDIT_KEY`, macOS/Windows, packaging, cgroup/systemd, `docs:build`, live OpenBao). The disclosure check was exercised against the in-process fixture only; no real KMS sidecar.
