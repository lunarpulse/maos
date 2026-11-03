# Landing 1 preflight — 2026-11-03

HEAD: `16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de`.

The prebuilt `xtask kloc-check --json` passed (exit 0): aggregate 192516; maos-bin 34188; providers 1425; domain 10099; manifest 4604; xtask 44406; kernel tokei 21042. Full output is in `commands.json`.

Rebased L1 bin envelope: 34188 + (145 × 2) + (54 × 3) = **34640** (maximum permitted delta 452). This is the story's original envelope calculation 33498 + 290 + 162 = 33950, rebased on current measured bin. Aggregate L1 ceiling: 192516 + (192833 - 191826) = **193523**. Providers 1898; domain 10115; manifest 4610; xtask 44466. Kernel must remain 26841 physical / 21042 tokei. No post-fmt measurement or repin has run.

Host `uptime`: load averages **15.28, 44.69, 31.94**.

## T0 vendor policy stop

The official Kimi pricing page, fetched 2026-11-03, states: “starting October 31, 2026, K2 series models will no longer be available to newly registered users; existing users are not affected. We plan to retire all K2 series models by the end of December 2026”. See `kimi-vendor-notice-2026-11-03.md` and https://platform.kimi.ai/docs/pricing/chat.md. The quickstart still uses `kimi-k2.6` (https://platform.kimi.ai/docs/guide/kimi-k2-6-quickstart.md).

[INFERENCE] A default restricted to existing accounts and scheduled for retirement conflicts with D-20-4-03's re-verified current-model premise and the currency policy's refusal of vendor-deprecated pins (`xtask/model-currency.toml:14-17`). Story T0 (`20-4-gemini-driver-and-endpoint-pin.md:245`) mandates STOP and a D-row for a T0 result contradicting a ruled option. The parent has been asked to resolve whether this is an explicit exception for existing accounts or requires a newer default. No substitute model or price has been fabricated.

## Resumption — operator ruling ask d204_12 (2026-11-03)

D-20-4-12 **RULED B**: `kimi-k3.1-pro` supersedes `kimi-k2.6`; Gemini stays
`gemini-3.5-flash-lite` (Stable on https://ai.google.dev/gemini-api/docs/models,
re-checked 2026-11-03). K2 ids are refused as vendor-deprecated.

T0 re-check (2026-11-03): https://platform.kimi.ai/docs/guide/kimi-k3-1-pro-quickstart.md
names the OpenAI-compatible `/v1/chat/completions` endpoint and successor model,
always-thinking mode, and access after a successful minimum $1 top-up.
https://platform.kimi.ai/docs/pricing/chat gives a **1,048,576-token context** and
standard uncached **$2 input / $10 output per million tokens**, i.e. **2000 / 10000
micro-USD per 1000 tokens**. Default prompt caching is enabled by the vendor;
this story's existing price-book schema prices standard uncached input/output,
not cache-write or cache-hit traffic.

https://platform.kimi.ai/docs/pricing/limits lists Tier0 at concurrency 1, RPM 3,
TPM 500,000 and TPD 1,500,000; higher account tiers differ. Those are vendor
account limits, not a claim that MAOS's default 1000-RPM fallback quota is a
vendor allowance. No quota-default widening was made here. Rendered HTML/DOM
tables are preserved in `kimi-pricing-tables.txt` and `kimi-limits.txt`, because
the vendor's `.md` representation omits the table cells.

Gemini standard text pricing: **$0.30 input / $2.50 output per million tokens**
(300 / 2500 micro-USD per 1000 tokens), read 2026-11-03 at
https://ai.google.dev/gemini-api/docs/pricing#gemini-3.5-flash-lite.
