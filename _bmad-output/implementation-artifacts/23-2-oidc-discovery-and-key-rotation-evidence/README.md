# Story 23-2 evidence (agent Dev232, host: local, branch e23-2-oidc-2026-11-26, base main 7628b3b9)

Env for every cargo command: `unset CI GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE; CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-maos-e232 MAOS_FIXTURE_DIR=$CARGO_TARGET_DIR/maos-fixtures`. Fixtures go stale after any source edit or `cargo fmt`: re-run `cargo run --locked -q -p xtask -- build-test-fixtures` before maos-bin daemon tests.

## Progress notes
- AC1/AC2 (maos-sso) done: `crates/maos-sso/src/discovery.rs`, `tests/oidc_discovery_23_2.rs` (18 tests), leg added to `check-enterprise-identity`.
- Production `ResolveHost` done: `crates/maos-bin/src/bounded_resolver.rs`, `tests/bounded_resolver_23_2.rs` (5).
- AC4 done: `enterprise_identity_wiring.rs` (6 new tests). AC3 done: `assertion_scope_23_2.rs` (6), `directed_task_22_4.rs` (5 daemon tests), control (2), cli (2). AC5 done: ADR-051 amendment, runbook, env contract (165), RELEASE-HOLDS 23-2.a..e.
- Step 1 (plants) done; step 2 (sweep) done; step 3 (gates) done; step 4 (10x) done; step 5 (numbers) done; step 6 below.

## Proven reds (marker `E232-PLANT`, one at a time, each removed; `grep -rn E232-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` empty after the last)
| Plant | File under `proven-red/` | Red tests |
|---|---|---|
| stale keys served past TTL | AC1/stale-keys-served-past-ttl.txt | past_the_ttl_every_assertion_fails_closed... |
| limiter removed | AC1/limiter-removed.txt | an_unknown_kid_inside_the_interval_causes_no_fetch, past_the_ttl... |
| issuer prefix compare | AC2/issuer-prefix-compare.txt | a_document_issuer_that_differs_by_any_byte_is_refused (trailing-slash case) |
| whole allowlist to decoder | AC4/mixed-allowlist-passed-whole-to-decoder.txt | a_mixed_algorithm_allowlist_verifies_an_rs256_assertion |
| environment before turn scope | AC3/environment-resolved-before-the-turn-scope.txt | daemon: an_ask_with_its_own_assertion..., a_replay_with_a_different_assertion... |
| assertion into accept row | AC3/assertion-written-into-accept-row.txt | daemon an_asks_assertion_appears_in_no_row_reply_or_error; in-process the_assertion_appears_in_no_record_row_or_debug |
| both key sources: discovery taken | AC4/discovery-taken-when-both-set.txt | jwks_and_discovery_together_are_configured_down |
| verify-at-accept removed | AC3/verify-at-accept-removed.txt | in-process an_invalid_assertion_is_refused..., a_replay_with_a_different_assertion...; daemon an_invalid_assertion_is_refused... |
Not planted: bounded-resolver cap/deadline, `--assertion-file` reader, `Assertion` Debug redaction (covered by passing tests only).

## Step 2: maos-bin sweep (`cargo test --locked -p maos-bin --no-fail-fast`, MAOS_17_2_CGROUP_MODE=undelegated, random 0600 hex MAOS_AUDIT_KEY deleted after)
First sweep: 6 failing targets, all root-caused and fixed or classified:
- `crypto_provider_20_13` (4), `crypto_real_boot_20_13`, `crypto_real_shell_20_13`: stale fixtures (`maos-crypto-recording` older than a source) -> fixtures rebuilt -> pass.
- `cohort_daemon_smoke_13_5c::composition_root_does_not_seed_manifest_scopes`: REAL regression of mine (two new `src/*.rs` files not enrolled in the scan list) -> `assertion_supply.rs`, `bounded_resolver.rs` enrolled, array 49 -> 51 -> pass.
- `online_adapters_23_0::the_umbrella_is_in_default_and_enables_network`: REAL (umbrella now also enables `maos-sso/discovery`) -> expectation updated -> pass.
- a lib-test compile error (`sso_down_reason` missing in a unit-test struct literal): REAL, fixed.
- `wasm_skill_authoring_20_8::signed::installed_signed_manifest_journals_verified_publisher_and_rejects_tamper`: still fails after the fixes; "the installed component starts" assertion. Classified environment-only per Main's list (not root-caused here; it fails at `signed.rs:114`, before any SSO code path, with SSO unset).
Re-run of the six targets plus `native_task_22_8` (16 passed, including `the_default_cast_boots_keyless_in_deterministic_mode` once fixtures were built) and `task_registry_22_4`: all pass except the wasm_skill_authoring one. I did not re-run the whole sweep after the last two fixes; the other 150+ result lines of the first sweep were `ok`.

## Step 3 gates
`cargo fmt --all` then `--check`: exit 0. `cargo check --locked -p maos-bin`: default, `--features network`, `--features wasm-host`, `--no-default-features`, `--no-default-features --features network,wasm-host`: all Finished. `-p maos-sso` with and without `--features discovery`: Finished. `check-composition-root-completeness` PASS (11 adapters, 20 constructions); `check-env-contract` PASS 165; `check-dependency-closure` PASS; `check-pub-field-constructors` PASS; `check-kernel-baseline` PASSED 28,372 = pinned (unchanged, 89 files); `cargo deny check` advisories/bans/licenses/sources ok; `check-enterprise-identity` PASSED (10 legs, oidc-verify includes the discovery test). Clippy (non `-D warnings`; maos-spirit-abi has the 4 known pre-existing errors under -D): new warnings of mine remain: `Result<_, ()>` in `AssertionCheck::check` and `check_carried_assertion`, a complex tuple type in two test files, one `assert_eq!` with a bool literal; the other `enterprise_identity.rs` warnings are on pre-existing lines. Not fixed.

## Step 4: 10x under 16 busy loops
`oidc_discovery_23_2` (18), `bounded_resolver_23_2` (5), `assertion_scope_23_2` (6), daemon tests `an_ask*`, `a_replay*`, `an_invalid*` of `directed_task_22_4` (5): 10 of 10 rounds passed (a first attempt failed only because fixtures were stale after fmt).

## Step 5 numbers
- stripped release `maos`: 38,993,104 B (main before 23-0b: 38,911,056; limit 39,069,022; headroom 75,918; this story +82,048 vs that base, the limit file untouched).
- kloc (tokei, `kloc-check`): maos-sso 888 (was 391, forecast 641..841: over by 47), maos-bin 46,853 (main 46,402 at 23-0b: +451; story forecast +150..+330: over by ~120), maos-control 1,746 (was 1,707; +39, forecast +10..30), maos-cli 7,499 (was 7,374: +125, forecast +15..35), maos-domain 11,229 (0), maos-kernel-core 22,173 (unchanged; alarm 23,000), aggregate 220,298, `over_budget` empty. The overshoot is the thread/bounded-wait machinery, the bounded resolver, the SsoEnv/typed reasons and the CLI file reader, none of which the forecast priced. Inline tests are counted in these.
- env-contract: 165 registered (162 + 3).

## Limits and risks (deviations re-thought)
2. KMS boot check now uses the shared bounded resolver instead of a thread-per-lookup. Exactly what changed for 23-8's `configure_kms_from_env`: (a) no detached thread per lookup: one process-wide worker; (b) answers are de-duplicated and more than 16 distinct answers fail the lookup (before: any number was returned and vetted); (c) an empty answer is a failure at the resolver (before an empty vec reached `vet_host`, which also refused it as `Unresolved`: same outcome); (d) the 2 s deadline now includes queue wait and a hung OS lookup makes later lookups in the process fail until it returns (before each lookup got its own thread and could succeed). Tests: `bounded_resolver_23_2` (cap at 16 admitted / 17 refused, dedup, failing and empty lookups, hung lookup bounded with a single running lookup, expired queued jobs skipped). No test drives `configure_kms_from_env` itself; it still compiles and `external_kms_wiring_23_8` was part of the green sweep.
3. Mixed-allowlist defect: the original decoder call (`validation.algorithms` = whole allowlist) is what plant "mixed-allowlist-passed-whole-to-decoder" restores; with it `a_mixed_algorithm_allowlist_verifies_an_rs256_assertion` fails for a real RS256 assertion under `[RS256, ES256]` on the discovering verifier. On the static path I observed the same failure once, before the fix, as `static_jwks_still_verifies_without_discovery` failing with "identity assertion algorithm rejected" under the default `MAOS_SSO_ALGS`; the static verifier's decode call was byte-identical to main's. I did not run a test against an unmodified checkout of main. So row 23-2.e(7) is supported by that observation and by code identity, not by a run on main; read it that way.
6. A carried assertion on a Host without SSO is refused (`sso_assertion_invalid`). No existing client sends `assertion` (new optional field, CLI flag opt-in), so no existing request changes behaviour; only a new client against an SSO-less Host sees it.
7. Signature changes migrated: `TaskRunner::run(task_id, text)` -> `(task_id, text, assertion)`: implementors `PortRunner` (installed_spirits.rs), `MiraRunner` (native_task.rs), test runners `GatedRunner` and `ScriptedRunner` (task_registry_22_4.rs). `DirectedTasks::accept(spirit, text, key)` -> `+ assertion`: caller `operator_door.rs::ask_command`; tests `task_registry_22_4.rs` (all 31 call sites) and `native_task_22_8.rs` (7). `run_turn` gained a last param: callers `PortRunner::run` and the shell REPL path (`None`). `OperatorCommand::Ask` gained `assertion`: constructor in maos-control's parse; patterns in operator_door.rs (`..` / named) and `ask_route_22_4.rs` (2).
Other: `directed_task_22_4.rs` Host now strips every inherited `MAOS_SSO_*` and `MAOS_AUDIT_KEY` for all its tests (hygiene); `online_adapters_23_0` and `cohort_daemon_smoke_13_5c` expectations updated for the new feature edge and two new source files.
