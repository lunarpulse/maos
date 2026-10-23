Prototype A regression list — executed 2026-10-23 at HEAD `8e93ffad` + `prototype-A.diff` (in place; removed afterwards).
Every row below was GREEN at HEAD with the prototype removed (`head-8e93ffad-the-seven.log`: 7 passed) and RED on the prototype
(`prototype-A-nextest-bin.log`). One root cause for all seven.

Root cause. The composition root mints `ProviderInfer { provider: <router default> }`, and with no API key in the environment the
router default is `ollama`: Ollama is registered without a probe unless `MAOS_OLLAMA_URL=skip` (`crates/maos-bin/src/main.rs:3571-3589`,
`default_id.get_or_insert_with(|| "ollama".into())` at `:3585`). The Spirits declare `anthropic`
(`spirits/hello-spirit/manifest.toml:12`, `spirits/researcher/manifest.toml:46`; prefix rule `crates/maos-manifest/src/manifest.rs:828`).
At HEAD the kernel issues the `ollama` token because `anthropic` has the same discriminant — the exact over-broad issuance this story closes.
Mint sites: shell turn `crates/maos-bin/src/shell_host.rs:91-98` (provider from `main.rs:3701`, passed `:3842`); Researcher
`main.rs:5667-5679` (provider from `:5438-5443` live, `:5467-5470` explicit mode, literal `"replay"` at `:5507` legacy cassette).

| # | Test (file:line of the failing assertion) | Path | Observed on prototype | Fix (per D-17-11-FW-05/FW-06 as recommended) |
|---|---|---|---|---|
| 1 | `inference_mode_15_6::explicit_replay_is_observable_before_an_empty_shell_exits` (`crates/maos-bin/tests/inference_mode_15_6.rs:129`) | shell, `MAOS_INFERENCE_MODE=replay`, `env_clear` | `Error: "capability issue failed: policy denied capability"` | FW-06 A (hello-spirit declares `ollama.llama3.1:8b`) → green unchanged; under FW-06 B the test sets `MAOS_OLLAMA_URL=skip` (its siblings already do, `:487`, `:586`) |
| 2 | `researcher_8_14c::researcher_8_14c_mcp_fanout` (`crates/maos-bin/tests/researcher_8_14c.rs:263-266`) | `maos run researcher --live --once` | `maos run: token issue failed: kernel policy denied capability issuance` | test env corrected: `MAOS_OLLAMA_URL=skip` → router default `anthropic` (`main.rs:3591-3595`), the Researcher's declared provider; `--live` without a mode resolves `Live` regardless of provider availability (`crates/maos-bin/src/inference_mode.rs:35-37`) |
| 3 | `shell_8_14a::audit_query_returns_shell_turn_rows` (`crates/maos-bin/tests/shell_8_14a.rs:206`) | shell | policy denied | as row 1 |
| 4 | `shell_8_14a::shell_hello_spirit_say_hi_and_audit_query` (`shell_8_14a.rs:98`) | shell | policy denied | as row 1 |
| 5 | `shell_host_16_2::shell_boot_then_butler_boot_on_one_tl_never_mix` (`crates/maos-bin/tests/shell_host_16_2.rs:1241`) | shell, replay (`:1228-1229`) | shell boot exit ≠ 0 | as row 1 |
| 6 | `shell_host_16_2::shell_boots_and_answers_from_a_cwd_with_no_spirits_dir` (`shell_host_16_2.rs:465`) | shell, replay (`:446-447`) | policy denied | as row 1 |
| 7 | `shell_host_16_2::repl_clarification_resolves_with_no_control_json` (`shell_host_16_2.rs:830`) | shell, replay (`:813-814`) | `BrokenPipe` — the shell exited on the refusal before the test wrote | as row 1 |

Not a regression: `maos-cli::load_test::load_discovery_failures_map_to_typed_exits` and
`maos-cli::revoke_token_test::invalid_token_ids_refused_locally_without_a_round_trip` red once on the prototype
(`prototype-A-nextest-journey.log`), green on a rerun of the same prototype (`prototype-A-nextest-journey-rerun.log`) and at HEAD — door flakes.

Static-only (no executing test reds it locally): the legacy cassette arm requests `ProviderInfer { "replay" }` (`main.rs:5507`), never
declared by any manifest → always refused under selector matching. `CassetteReplayPort` never reads the token
(`crates/maos-bin/src/cassette_replay.rs:170-174`), so the fix is at origin: mint the router default as the other arms do (FW-05).

Green on the prototype (no change owed): `-p maos-domain -p maos-kernel-core -p maos-capability -p maos-pdp -p maos-manifest` 944;
`maos-kernel-core --lib` 258; `-p maos-wasm-host -p maos-shell -p butler -p researcher -p maos-iac` 372; `-p maos-journey-test -p maos-cli` 201
(rerun); the other 553 `maos-bin` tests, including `mcp_mint_receipt_17_9`, `cli_wrapper_exhaustion_17_9`, `spawned_scope_17_3c`,
`recall_mediation_17_3d`, `audit_refusal_16_5`, `inference_mode_15_6` (other cases), `shell_host_16_2` (other cases), `one_shot_hello`
(it already sets `MAOS_OLLAMA_URL=skip`, `crates/maos-bin/tests/one_shot_hello.rs:19`).
Not exercised locally: T2/T3/delegated-cgroup substrates print `SKIP` off CI (`docs/testing/proof-contract.md:66-71`); `#[ignore]` rows.
