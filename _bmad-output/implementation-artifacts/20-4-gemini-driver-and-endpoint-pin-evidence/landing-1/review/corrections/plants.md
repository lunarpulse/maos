# Landing 1 review correction plants

Baseline: `ef134444`; owned scratch `/tmp/204l1-review/`. No plant is left in the working tree. Every restore below was actually executed with copy-back, `cmp`, and `touch`, and exited 0. Exact command ordering/results are retained in `commands.json`.

## P3: native registration must not be inferred from replay

Final witnesses in `crates/maos-bin/tests/provider_selection_20_4.rs`:

- `gemini_key_really_registers_without_replay_p3:207`
- `kimi_key_really_registers_without_replay_p3:211`
- `existing_key_precedence_is_preserved_p3:215`
- `new_keys_route_in_replay:120` (explicit replay routing only)

The registration witnesses use supported `MAOS_INFERENCE_MODE=live`, no replay cassette, only the respective new secret key, and EOF stdin. The selected real id must boot, the absent other new id must fail, and neither leg may emit an `infer:` Transparency Log row. No paid request is made. The separate precedence witness checks each new key against Anthropic and OpenAI keys without an explicit selector.

For each final plant, the **single real map insertion line** was removed from `crates/maos-bin/src/main.rs`; its `live_provider_available` and `default_id.get_or_insert_with` assignments were retained:

| Plant | Mutation | Observed red / control green | Exit / retained output |
|---|---|---|---|
| Gemini | Omit `providers_map.insert("gemini".into(), Arc::new(provider));` at line 3981 | Gemini live-EOF registration fails because the selected id is absent; explicitly selected replay routing still passes. | 100; 1 failed / 1 passed; `logs/gemini-omission-replay-green-native-red.log` |
| Kimi | Omit `providers_map.insert("kimi".into(), Arc::new(provider));` at line 4001 | Kimi live-EOF registration fails because the selected id is absent; explicitly selected replay routing still passes. | 100; 1 failed / 1 passed; `logs/kimi-omission-replay-green-native-red.log` |

The final planted commands were:

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --test provider_selection_20_4 --profile ci --test-threads 1 --no-fail-fast -E 'test(gemini_key_really_registers_without_replay_p3) | test(new_keys_route_in_replay)'
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-bin --test provider_selection_20_4 --profile ci --test-threads 1 --no-fail-fast -E 'test(kimi_key_really_registers_without_replay_p3) | test(new_keys_route_in_replay)'
```

Each was followed immediately by this successful exact restoration:

```sh
cp /tmp/204l1-review/main-config.rs /home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs && cmp /tmp/204l1-review/main-config.rs /home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs && touch /home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs
```

After both restores and formatting, all five affected root binaries pass serially: **46/46**, exit 0 (`logs/five-root-binaries-final-green.log`).

### Invalid/superseded observations, not claimed as the final proof

The first registration test draft used unsupported mode `deterministic`; its failures are retained but do not prove registration. After correcting to `live`, both registration witnesses passed on the unplanted tree (`logs/selection-valid-baseline-green.log`).

The original implicit-default replay witness also went red under a pure omission: when the map becomes empty, fallback rewrites `default_id` to Anthropic before replay repopulates ids. These real **two-red** outcomes are retained in `logs/gemini-omission-two-red.log` and `logs/kimi-omission-two-red.log`; they are not relabelled as a replay green. The replay-only witness now explicitly selects the id, matching its limited routing claim, while real registration and default precedence have independent witnesses. Only the final one-red/one-green plants above prove that separation. This decision was reported to Main.

## F6: catalog and actual driver endpoint must agree

`catalog_endpoint_matches_driver_endpoint_f6:16876` in `crates/maos-bin/src/main.rs` captures all five real driver request URLs without a network request and compares their stripped base endpoint to the admission catalog.

The plant changed only the catalog Gemini tuple to:

```rust
("gemini", ("https://wrong.invalid", GEMINI_DEFAULTS.1))
```

The driver retained its shared endpoint. The test observed `Some("https://generativelanguage.googleapis.com")` versus `Some("https://wrong.invalid")`, failed, and exited 101 (`logs/catalog-endpoint-drift-red.log`). The same copy-back/`cmp`/`touch` command above restored the saved formatted file, exit 0. The restored unit passes in the final 46-test serial root run.

Moving model strings into shared defaults separately made the old actual model-currency gate fail closed (exit 1). The new shared-declaration vector exposed duplicate diagnostics before deduplication (exit 101). Those observations remain in `logs/currency-shared-reference-red.log` and `logs/shared-duplicate-declaration-red.log`. Commented and inactive `#[cfg(any())]` definition vectors separately exposed fail-open masking before syntax-aware, attribute-free root extraction (both exit 101; `logs/shared-comment-forgery-red.log` and `logs/shared-cfg-forgery-red.log`). These hermetic vectors mutate only temporary fixture trees, not production source, so no production restore is needed. Final serial currency vectors pass **25/25** (`logs/currency-final-guarded-green.log`); unqualified/foreign references remain refused.
