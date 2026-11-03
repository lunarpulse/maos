# Security review: 20-4 landing 1 (commit 2752e750) — security-reviewer layer

Saved verbatim in substance by the orchestrator (the reviewer had no write tool). Scope: owned files of landing 1; ACs 7–8 and stories 20-8/9/13/14 excluded. Read-only; no cargo.

Counts: Critical 0, High 0, Medium 0, Low 6 (F1–F6), Informational 3 (F7–F9).

## Verified controls (no finding)
- Gemini key header-only: `gemini.rs:82` sends `x-goog-api-key`; URL (`gemini.rs:72–75`) carries endpoint and model only.
- `validate_api_key` (`provider.rs`) accepts printable ASCII 0x21–0x7e only, "value withheld" on error (blocks CR/LF header injection); transport error echoes URL only.
- Endpoint pin runs before pid allocation: `admission.rs:931–933` in `load_admit_start`; all six `main.rs` call sites covered (4510, 5767, 6306, 9589, 10276, 11767); `AdmissionDeps.providers = None` refuses any manifest declaring `[providers]`.
- Manifest cannot redirect the endpoint: `check_provider_pins` (`admission.rs:773–803`) compares only against the boot catalog.
- Fallback constrained: kernel refuses fallback ids differing from `provider_id` (`inference/mod.rs:291–292`); capability checks `:250–272`.
- Kimi K2 ids refused: `xtask/model-currency.toml:146–159` allows only `kimi-k3.1-pro`.
- Purge covers all 12 `SecretKey::ALL` keys (`purge.rs` `known_secret_keys`).
- Cassettes labelled `provenance: seed`, `provider_id: replay`; no key-shaped literals.
- Attribution correct (`openai.rs:125,190`); responses capped (`read_capped`, `MAX_RESPONSE_BYTES`).

## Findings
- **F1 Low (latent).** `options.model_id` unvalidated and interpolated into the Gemini URL (`gemini.rs:68,72–75`); bypasses the admitted model pin at runtime. Unreachable from a Spirit today (callers set `model_id: None`; only the operator-configured eval judge sets it, `maos-eval/src/distill_replay.rs:64–67`). Fix: validate against `[A-Za-z0-9._:-]`; reject a per-request model id differing from the admitted pin or remove the field for non-eval callers.
- **F2 Low.** Token usage truncates/wraps: `gemini.rs:135–140` and `openai.rs:173–180` (`as_u64().unwrap_or(0) as u32`). Under-accounts cost. Fix: `u32::try_from` -> error; missing usage is an error.
- **F3 Low.** Missing `finishReason` treated as clean stop (`gemini.rs:129–130`). Fix: error or explicit StopReason.
- **F4 Low.** Unbounded vendor `finishReason` string copied (`gemini.rs:132`, `openai.rs:168`). Fix: cap ~64 bytes.
- **F5 Low.** Unredacted vendor error body reaches the Spirit (`gemini.rs:89–92`, `inference/mod.rs:362–365`, `provider.rs:36–37`); pre-existing pattern extended to Gemini. Fix: truncate ~512 bytes, strip control chars.
- **F6 Low.** Endpoint/model literals duplicated between catalog (`main.rs:3943–3946`) and drivers (`:4002–4003`, `:4022–4023`); identical today. Fix: one const per provider + a unit test asserting catalog endpoint == driver endpoint.
- **F7 Info.** Endpoint pin is opt-in (`admission.rs:931`); document.
- **F8 Info.** Credential fingerprint (first 8 bytes of SHA-256) held in rate-limit map; no fix required.
- **F9 Info.** Pin is exact-string equality (`admission.rs:795–797`); no DNS/TLS identity pinning (documented); trailing-slash mismatch is a false refusal, not a bypass.

## Rejected candidates
Spirit overriding provider at runtime; transport error echoing the key; cassette wire-proof claim; timing side channel on the pin comparison.

## Not reviewed
`rate_limit.rs` tests, `provider_endpoint_pin_20_4.rs`, `support/provider204.rs`, `support/wire204.rs`, gemini/kimi fixtures, `SCANNED_SOURCE_FILES`, keyring migration (`main.rs` ~3780–3910), story/decision files.
