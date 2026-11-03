# 20-4 landing 1 — Acceptance Auditor review (AC1–AC6, L1 records)

Commit under review: 2752e750 (parent 8c809f9c). Read-only static review; no builds, tests, clippy, fmt, xtask runs, and no git mutation.

## Verdict

overall_correctness: correct (no patch-introduced defect meeting the review criteria was found in the static read)
confidence: 0.6 (static reading only; compile, test, and gate results were not observed)

## Coverage by criterion

- AC1 Gemini native driver: `crates/maos-providers/src/gemini.rs` posts to `{endpoint}/v1beta/models/{model}:generateContent` via `http_post_exchange`; key is sent only in `x-goog-api-key`; non-2xx is data (`ProviderRejected`); `finishReason` STOP→StopSequence, MAX_TOKENS→MaxTokens, other→ProviderStop. Fixture witnesses in `crates/maos-providers/tests/gemini_round_trip_test.rs`.
- AC2 Kimi: `OpenAiProvider::compatible("kimi", SecretKey::KimiApiKey, …, "https://api.moonshot.ai", "kimi-k3.1-pro")` in `crates/maos-bin/src/main.rs` (~3997–4024); Bearer auth; attribution `provider_id: "kimi"` via `parse_openai_response`. Witness `kimi_round_trip_test.rs`.
- AC3 Credentials: `SecretKey::{GeminiApiKey,KimiApiKey}` in `crates/maos-domain/src/ports/secret_store.rs` (ALL 10→12, keyring names, env names); `validate_api_key` (0x21–0x7E, no value echo) in `crates/maos-providers/src/provider.rs`; invalid key → exit 78 at boot (`main.rs` registration matches). Witness `invalid_credentials_refuse_without_echoing_bytes` in `crates/maos-bin/tests/provider_selection_20_4.rs:58-75`. Keyring/env consumers iterate `SecretKey::ALL` (`main.rs:3781,3855,3894,3906`) and purge labels (`purge.rs:343-344`) cover the new variants.
- AC4 Model currency: `xtask/model-currency.toml` allows `gemini-3.5-flash-lite` and `kimi-k3.1-pro` only; K2 ids are listed as `[[providers.kimi.retired]]`; `check_model_currency` scans five hosts; witnesses `gemini_unknown_constructor_model_is_governed_20_4`, `kimi_k2_constructor_is_refused_as_deprecated_20_4`, `missing_kimi_constructor_fails_closed_20_4` in `xtask/tests/model_currency_gate.rs`.
- AC5 Selection: `MAOS_DEFAULT_PROVIDER` (empty = unset; unknown id refuses boot naming registered ids, `main.rs` after replay registration); `gate_manifest` reads `[providers]` (`admission.rs:709-730`), declared check → `provider_unavailable`, non-empty fallback → `fallback_unsupported`; `declared_providers` returns only the primary (`main.rs:896-900`); shell and one-shot bind the primary via `BoundInference`. Witnesses in `provider_selection_20_4.rs`.
- AC6 Endpoint pin: `check_provider_pins` (`admission.rs:~770-810`) runs before every providers-bearing `admit_spirit` (`load_admit_start` entry at `admission.rs:932`, shell `main.rs:~4520`, one-shot `main.rs:~9560`, control `main.rs:11790`-adjacent); SHA-256 of exact base-URL bytes; `endpoint_pin_mismatch`; catalog built before replay wrapping and `registered` captured after wrapping; Ollama URL read once. Witnesses in `crates/maos-bin/tests/provider_endpoint_pin_20_4.rs` (wrong pin, matching pin, assertion-not-reconfiguration, air-gap in keyless replay).
- Kernel unchanged: no `crates/maos-kernel-core` file in the commit stat; `ProvidersSection` and `admit_spirit`'s providers argument pre-existed.
- L1 records: the docs in `docs/maos.dev/run-maos.md` and `tests/coverage-matrix.yaml` FR3 notes describe seed-cassette routing proofs, not vendor round-trips, and the L2 boundary. The docs match the code paths read.

## Findings

None meeting the review criteria (provable impact, actionable, unintentional, introduced in the patch). Items considered and rejected:

- Whitespace-only credentials for Anthropic/OpenAI now refuse boot with exit 78 instead of degrading to Unconfigured (`trim().is_empty()` → `is_empty()`). This is the intended AC3.4 refusal of non-visible-ASCII bytes; not a defect.
- Gemini `parse_response` returns `Serde("missing candidates[0]")` when a response has no candidates (for example a blocked prompt). AC1 defines only the finishReason mapping, so the spec does not cover this shape; the impact depends on vendor response shapes not observed here. Not reported.
- A spirit may still declare `kimi.kimi-k2.6` in `provider.complete` without `[providers].model_id`; the constructor model (`kimi-k3.1-pro`) then serves the request. This matches the pre-existing declaration semantics for every provider (`openai.<model>` behaves the same), so it is not a patch regression. K2 refusal is enforced at the constructor by `check-model-currency`, and model assertions via `[providers].model_id` are refused by `check_provider_pins`.

## Checks the main agent should run

- `cargo nextest run -p maos-bin --test provider_selection_20_4 --test provider_endpoint_pin_20_4`
- `cargo nextest run -p maos-providers` (gemini_round_trip_test, kimi_round_trip_test)
- `cargo run -p xtask -- check-model-currency` and the `model_currency_gate` tests
- `kloc-check` against the kloc.toml pins (maos-providers 1608, maos-bin 34363 formatted, maos-domain 10107, xtask 44420) and kernel 26841 / 21042 unchanged
