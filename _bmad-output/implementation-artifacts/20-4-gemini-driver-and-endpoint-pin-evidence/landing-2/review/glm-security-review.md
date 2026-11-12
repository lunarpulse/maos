Review complete. Full coverage of the 20-4 L2 kernel row (R = KA + KIO1 + KIO2), the witnesses, the plants, and the endpoint-pin/serde focus areas. Report follows.

---

# Security review — 251dc692, story 20-4 L2 kernel landing (R = KA + KIO1 + KIO2)

## Findings

**F1 — P3 (hardening) — Kernel IO adapter follows redirects; endpoint pin stops at the first hop and vendor key headers (`x-api-key`, `x-goog-api-key`) are replayed to the redirect target.**
- Evidence: `crates/maos-kernel-core/src/io/mod.rs:96-118` — `http_post_exchange`/`http_get` use one-off `ureq::get/post(url)`, which uses ureq's default agent (`ureq-2.12.1/src/agent.rs:262-263`: `redirects: 5, redirect_auth_headers: Never`). On a followed 301/302/303 (POST→GET, body dropped; 307/308 for POST are *not* followed — `ureq-2.12.1/src/unit.rs:190-203`), ureq retains all headers except `content-length`, `cookie`, `authorization` (`unit.rs:216-224`). Drivers send keys in non-`Authorization` headers: Anthropic `x-api-key` (`crates/maos-providers/src/anthropic.rs:87`), Gemini `x-goog-api-key` (`gemini.rs:89`). OpenAI/Kimi `Authorization: Bearer` **is** stripped — protection is inconsistent across drivers.
- Exploit path: the admission-time pin (`crates/maos-bin/src/admission.rs:996-1005`) constrains only the *initial* URL; a 3xx from that endpoint re-targets the request with the key header attached, silently (final status/body returned; `ProviderAttribution.endpoint_url` still names the configured URL). Requires the pinned endpoint itself (or a plaintext-path MITM, who can already read the key header on the first hop) to emit the redirect — i.e., no privilege gain over the pinned endpoint's existing trust. Hence P3, not P2. Also defeats the pin's purpose for 20-13's endpoint table post-first-hop.
- Fix sketch: hold a `ureq::Agent` in `IoSubsystemAdapter` built with `.redirects(0)`; a 3xx then surfaces as `Err(Error::Status(3xx, _))`, already handled by the `Ok(response) | Err(ureq::Error::Status(_, response))` arm → `HttpExchange.status = 30x` → driver `ProviderRejected`. Zero behavioral change elsewhere.
- Witness need: extend `io_redaction_and_status_20_4.rs` — loopback server answering `302` + `Location`; assert exchange status stays 302 and exactly one TCP connection is made.

**F2 — P3 (pre-existing, adjacent to the mapping) — duplicate same-id fallback entries amplify provider calls (N+1)× per single rate-limit token.**
- Evidence: the kernel only requires fallback ids to *equal* the primary (`crates/maos-kernel-core/src/inference/mod.rs:293-295`); the limiter consumes one token before dispatch (`:302-320`); `dispatch_with_fallback` retries retriable errors (transport, 429, 5xx) through the whole chain (`router.rs:62-92`); the manifest neither bounds fallback length nor rejects primary duplicates (`crates/maos-manifest/src/manifest.rs:2530-2537`; contrast MCP's `fallback_transport must differ from primary`, `:2652-2658`). Today every production caller passes `Vec::new()` (e.g., `crates/maos-eval/src/distill_replay.rs:78`), so this is latent.
- Exploit path: a hostile-but-admitted manifest declaring `[[fallback]] id = "gemini"` × N (each duplicate passes `check_provider_pins` trivially) gets N+1 real provider HTTP calls — including while the provider is actively 429ing — per one bucket token. Budget undercount only; **no cross-provider contamination** (unequal ids are `CapabilityDenied`).
- Fix sketch: at admission (or in `complete()`), reject fallback entries equal to the primary or dedupe the chain; alternatively consume a bucket token per dispatch attempt.
- Witness need: rate-limit harness with fallback `[id; 3]` and a 429ing counting driver, asserting call-count vs tokens consumed.

No P0–P2 findings → no correction round required; both rows are deferred-row material per operator policy.

## Rejected candidates (investigated, no credible exploit)
1. **URL echo in transport errors/journal** (`io/mod.rs:88,96,111,115` — `ureq POST {url}`): no driver embeds credentials in URLs (`anthropic.rs:80`, `openai.rs:105`, `gemini.rs:77-79`, `ollama.rs:55` — all `{endpoint}{fixed-path}`; Gemini key header-only by design). MCP URLs are operator-configured. `io_call_journal` is feature-off by default and records URLs only.
2. **Pin bypass via case/IDN/userinfo/port/trailing dot**: impossible — the pin is SHA-256 over the raw bytes of `registered.endpoint` with no URL parsing (`admission.rs:997-1000`); any byte variant mismatches → `EndpointPinMismatch`. `eq_ignore_ascii_case` applies to the hex digest only.
3. **Pin TOCTOU / env re-read**: catalog and drivers are built from one boot-time env snapshot; `registered` set from the final driver map (`main.rs:4067-4082`); catalog is an immutable `OnceLock`; parity witnessed by `catalog_endpoint_matches_driver_endpoint_f6` (`main.rs:17784+`). Missing catalog ⇒ `ProviderUnavailable` (fail closed).
4. **Cross-provider rate-limit contamination via fallback**: denied at `inference/mod.rs:293-295`; provider ids interned case-sensitively, unknown ⇒ `Unconfigured` fail-closed (`inference/mod.rs:92-108`), parity with `PROVIDER_IDS` (`maos-providers/src/lib.rs:27`) witnessed (`inference_provider_ids_20_4.rs:212`).
5. **`RateLimited` frame fingerprint leak** (`inference/mod.rs:135-141`): 8-byte SHA-256 prefix, documented design (`rate_limit.rs` header), non-invertible, scoped to the invoking Spirit's own provider calls.
6. **KIO2 non-2xx 10 MiB body retention**: bounded at the cap (`io/mod.rs:56-73`); driver error text further bounded to 512 chars with control chars stripped (`provider.rs:55-70`).
7. **KIO1 completeness**: `BadHeader` is the only ureq error whose message echoes header bytes (confirmed by plant-2 log); MCP pre-validates bearer bytes (`streamable_http.rs:83-89`); provider keys are pre-validated printable-ASCII (`provider.rs:39-49`).
8. **unwrap/expect after serde**: none in the landed production code — all serde errors propagated/matched (`io/mod.rs` has none; `inference/mod.rs` `unwrap_or*` sites at :342, :373, :378, :402, :410, :424 are Option/SystemTime/transparency-log, not serde results). Gate-compliant on inspection.
9. **Anthropic/OpenAI/Kimi/Ollama credential-in-URL** — rejected (see 1).
10. **Docs secrets scan** (`docs/maos.dev/run-maos.md`, `docs/runbooks/provisioning-checklist.md`): key names only, no values.

## Counts
- Findings: **2** (P3 ×2; P0–P3 → 0/0/0/2). Rejected candidates: **10**.
- Files reviewed: `crates/maos-kernel-core/src/inference/mod.rs` (full), `src/inference/router.rs`, `src/io/mod.rs` (full), `crates/maos-providers/src/{provider,gemini,anthropic,openai,ollama,rate_limit,lib}.rs`, `crates/maos-bin/src/admission.rs` (pin section), `main.rs` (provider catalog/driver construction 3876–4085, request-construction sites), `crates/maos-manifest/src/manifest.rs` (provider config/fallback), `crates/maos-mcp/src/transport/streamable_http.rs`, `crates/maos-kernel-core/tests/{inference_provider_ids_20_4,io_redaction_and_status_20_4}.rs`, `crates/maos-bin/tests/provider_endpoint_pin_20_4.rs`, ureq 2.12.1 source (`agent.rs`, `unit.rs`, `lib.rs`), landing-2 evidence (`plants.md`, `files-changed.txt`, FLAG-Winston decision). Landed docs and xtask/coverage configs inspected as metadata only (non-security).

## Not verified
- I have no shell/git access in this session, so `git show 251dc692` itself could not be run; HEAD is `41b937cc` and I could not recompute the landing digests (`inference/mod.rs` `2ed62363…`, `io/mod.rs` `57d2b794…`) to prove byte-identity. The working-tree files do contain exactly the landed shapes the FLAG record describes (gemini/kimi intern arms, BadHeader redaction arm, KIO2 truncate flag), and landing-1 reviews were not redone per instructions. No tests/builds were executed (read-only mandate).