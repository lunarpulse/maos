# 20-4 closure note (2026-11-12)

Story `20-4-gemini-driver-and-endpoint-pin`; reviewed commit `251dc692` (landing 2, kernel row R = KA + KIO1 + KIO2). Record-only closure: no cargo, no code change.

## Operator exception (ask `q20`, answer `p4` = accept, 2026-11-12)

For 20-4 the operator accepted that the green hosted discipline run `43150757804` (199/199 jobs, head `d00802aa144a77bc81994730e50e0954ab75723d`, a descendant of `251dc692`) substitutes for a local `prepush` and is the final-tree proof. Checked 2026-11-12: `gh run view 43150757804 --repo lunarpulse/maos --json conclusion,headSha` gave `success` / `d00802aa…`; `git merge-base --is-ancestor 251dc692 d00802aa` exited 0.

## AC table

| AC | Status | Evidence |
|---|---|---|
| AC1 Gemini driver, shell select | LANDED (2752e750, 73e7afbc) | `crates/maos-providers/src/gemini.rs`, `gemini_round_trip_test.rs`, `provider_selection_20_4.rs` |
| AC2 Kimi via OpenAI-compatible driver | LANDED | `kimi_round_trip_test.rs` |
| AC3 credentials, registration, default | LANDED | `provider_selection_20_4.rs`, `docs/maos.dev/run-maos.md` |
| AC4 currency pins | LANDED | `xtask/tests/model_currency_gate.rs` |
| AC5 per-Spirit provider | LANDED | `provider_selection_20_4.rs`; L1 corrections in `landing-1/review/corrections/disposition.md` |
| AC6 endpoint pin refuses | LANDED | `provider_endpoint_pin_20_4.rs` |
| AC7 rate-limit id set | LANDED (251dc692) | `inference/mod.rs:100`, `inference_provider_ids_20_4.rs` |
| AC8 BadHeader redaction, non-2xx status | LANDED (251dc692) | `io/mod.rs`, `io_redaction_and_status_20_4.rs` |
| AC9 records | LANDED | deferred closures D:1297/D:1298 stamped `@251dc692`; coverage and docs |
| `ops-live-gemini-leg` | CLAIM-BOUNDARY | live Gemini and Kimi vendor round trips are UNCLAIMED, no paid keys. Hermetic recording-transport tests and seed cassettes carry the proof, labelled seed provenance in release notes. |

## Review disposition

GPT gpt-6.1-sol: 0 P0/P1, one P2 (record state), resolved by the exception above, the persisted disposition and the `@251dc692` stamps. GLM glm-5.3 security: 0 P0-P2, F1 (redirect replay of vendor key headers) and F2 (duplicate-fallback N+1) are P3 rule-10 rows in `deferred-work.md` under 'Observed during: 20-4 L2 review'.

## Honest caveats

- The hosted result is read from `gh` conclusions only; job logs were not inspected.
- The `ci-20-4-verify.yml` recipe was never dispatched; the exception, not the recipe, closes the hosted clause.
- No local prepush ran for L2 (L1's was non-green under load; timing rows owned by 20-7).
- macOS and Windows legs never ran; live vendor legs UNCLAIMED.
- `q20`/`p4` is an operator message to the orchestrator; it is not in `decisions-2026-11-12-acceleration.md`.
