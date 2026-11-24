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
| `cargo test --locked -p maos-secrets --features http-kms --test external_kms_23_8` (the AC1 command) | 17 passed |
| `cargo test --locked -p maos-bin --test external_kms_wiring_23_8` (the AC1 command) | 17 passed (profile without `online-adapters`: 9 passed, the `online::` module and three online-only boot tests compile out) |
| `cargo test --locked -p maos-secrets` (and with `--features http-kms`) | at_rest_seal 7/9, default_plaintext_preserved 2, kms_fault_inject 1, all green |
| `cargo test --locked -p maos-bin --test enterprise_identity_wiring --test online_adapters_23_0 --test maos_uninstall_16_4 --test crypto_real_boot_20_13 --test researcher_8_14c` | 4, 3, 24, 1, 7 passed |
| `cargo test --locked -p maos-bin --lib` / `--bin maos story_13_5a` | 32 passed / 3 passed (the cohort KMS-down refusal test is unchanged and green) |
| same set with a random 0600 hex `MAOS_AUDIT_KEY` exported (key file deleted afterwards) | all green (an earlier run failed `crypto_real_boot_20_13` only because the fixture binary was stale after my plants: "test fixture stale"; rebuilt, green) |
| `external_kms_23_8` x10 and `external_kms_wiring_23_8` x10 under 16 busy loops (`while :; do :; done`, load average 12.6) | 10/10 and 10/10 pass (and 10/10 again for the wiring file after the probe-task test was added) |

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
