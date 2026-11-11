# 20-4 L2 — witnesses, reds, plants (2026-11-11, working-tree base `89b64566`)

All runs `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test inference_provider_ids_20_4 --test io_redaction_and_status_20_4 --no-fail-fast`
(inherited target dir `/mnt/build/cargo-maos`). Raw logs under `logs/`.

## Failing-before (kernel sources untouched = `89b64566`, tests new)

`logs/red-before-head.log` — rc **100**, 5 tests: 1 passed, 4 failed.

| AC | Witness (`name:line`) | HEAD result |
|---|---|---|
| AC7 | `inference_provider_ids_20_4.rs:175` `gemini_and_kimi_pass_the_limiter_and_consume_their_own_bucket` | FAIL at `:181` — `gemini must pass the rate-limit id set, got Unconfigured` |
| AC7 | `inference_provider_ids_20_4.rs:212` `every_provider_id_the_root_can_register_passes_the_limiter` | FAIL at `:218` — `gemini is in PROVIDER_IDS but the kernel refuses it: Unconfigured` |
| AC7 | `inference_provider_ids_20_4.rs:197` `an_id_outside_the_closed_set_stays_unconfigured_and_never_runs_a_driver` | PASS (fail-closed control: `bedrock` is `Unconfigured` before and after, driver never runs) |
| AC8.1 | `io_redaction_and_status_20_4.rs:55` `a_header_ureq_rejects_is_never_echoed_in_the_error` | FAIL at `:67` — `ureq POST http://127.0.0.1:45981/v1/complete: Bad Header: invalid header 'Authorization: Bearer sk-secrettail'` (the credential is echoed) |
| AC8.2 | `io_redaction_and_status_20_4.rs:82` `a_non_2xx_body_over_the_cap_is_cut_and_keeps_its_status` | FAIL at `:90` — `401 over the cap must keep its status, got Err(decode error: response body exceeds 10485760 byte limit)` |

## After (KA + KIO1 + KIO2 applied, `cargo fmt --all`)

`logs/green-after.log` (the two new binaries plus `io_exchange_18_8` and `inference_fallback_authz_18_1`, unchanged):
rc 0, 15/15. `logs/final-new-binaries.log` (final tree, after the plants were restored): rc 0, 5/5.
`logs/kernel-lib.log`: `cargo test --locked -p maos-kernel-core --lib` rc 0, 253 passed. The unchanged
`io_exchange_18_8` binary keeps the 2xx-over-cap `Decode` (POST and GET) and exactly-cap `Ok` witnesses green, which is
the "2xx over the cap is still `Decode`" half of AC8.2.

## Plants (variants cut in `/tmp/t204l2-plants/`, copied over the real file, run, copied back; `cmp` against the pre-plant
copy rc 0, then `touch`; `git status` shows only the intended diff afterwards)

| # | Plant | Run | rc | Wall | Observed red (log) |
|---|---|---|---:|---:|---|
| 1 | drop the `"gemini" => "gemini"` arm in `bucket_key` (`inference/mod.rs`) | `logs/plant-1-drop-gemini-arm.log` | 100 | 23 s (includes the kernel recompile) | `inference_provider_ids_20_4.rs:181` `gemini must pass the rate-limit id set, got Unconfigured`; `:218` PROVIDER_IDS parity; bedrock control + both `io_*` witnesses stay green (2 failed / 3 passed) |
| 2 | revert KIO1 (remove the `BadHeader` arm in `http_post_exchange`, `io/mod.rs`) | `logs/plant-2-revert-kio1.log` | 100 | 8 s | `io_redaction_and_status_20_4.rs:67` credential echoed (`invalid header 'Authorization: Bearer sk-secrettail'`); KIO2 witness green (1 failed / 4 passed) |
| 3 | revert KIO2 (`read_capped(response, "POST", false)`) | `logs/plant-3-revert-kio2.log` | 100 | 6 s | `io_redaction_and_status_20_4.rs:90` `Err(decode error: response body exceeds 10485760 byte limit)`; KIO1 witness green (1 failed / 4 passed) |

Each row has no other falsifier (the story's "each row has no other falsifier"): plant 2 leaves AC8.2 green and plant 3 leaves
AC8.1 green, so the two kernel IO rows are independently necessary.
