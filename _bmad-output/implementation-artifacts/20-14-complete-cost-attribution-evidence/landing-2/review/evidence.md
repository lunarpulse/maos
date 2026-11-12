# 20-14 L2 review round 1 — evidence (HEAD `9e4be177`; Dev2014d; 2026-11-12)

Inputs, persisted verbatim: `gpt-review.md` (GPT `Review2014K`: 3 P2, verdict incorrect) and `security.md` (GLM `Sec2014K`: S1 Low, S2 Low, S3 Info). Orchestrator ruling (Main, handoff 2014-r1): the maos-capability
overdraft is ADJUDICATED (disclosed to the operator) only for the redesign that survives review; kernel growth from this round <= +0 if possible, STOP above +4 tokei.

## Findings and fixes

| Finding | Fix | Red before (raw log) | Final witness |
|---|---|---|---|
| GPT P2-1 + GLM S3: wide `tokens() -> &CapTokensShardRing` | removed; two narrow inherent delegates `bind_cost_scope(&TokenId, CostScope) -> bool`, `cost_scope(&TokenId)`; callers (`shell_host.rs`, emission, kernel test) migrated | `red/kernel-HEAD-red.log` (the delegates do not exist at HEAD: compile red) | kernel `the_cost_scope_delegates_cannot_mint_or_change_a_tokens_validity`; surface gate green, NO refresh |
| GPT P2-3 + GLM S1: second map rescanned per bind, never pruned | scope = `TokenState.cost_scope: OnceLock<CostScope>`, bound through the token's own shard; the map is deleted | `red/capability-HEAD-red.log` (no `TokenState.cost_scope` / `CapShard::bind_cost_scope` at HEAD: compile red) | `maos-capability/tests/cost_scope_20_14.rs` (3000 mint/revoke/expiry, eviction takes the scope); kernel `a_paid_completion_revoked_mid_call_keeps_its_attribution` |
| GPT P2-2: redactor rewrites 32-hex / `sk-` principals, `sk-a"b` malformed -> frame skipped | `maos_domain::cost::{encode_identity, decode_identity}` colon-hex cover + serde on `principal_id` / `task_ref`; erasure matches the decoded value | `red/bin-HEAD-red-vectors.log`: all 3 vectors RED at HEAD (`<REDACTED:type=capability_token,...>`, `<REDACTED:type=api_key_generic,...>`, `sk-a"b` row missing from the report) | bin `a_32_hex_principal_...`, `a_credential_prefixed_principal_...`, `a_principal_with_an_escaped_quote_...` (real `maos shell` -> real `maosctl cost-reconcile` -> real one-shot `forget`, dimensions 50/30 kept, unrelated `bob` frame untouched); domain `cost_frame_20_14` (3); iac `cost_identity_cover_20_14` |
| GLM S2: no length cap, invisible/bidi accepted | `parse_session_principal` <= 128 bytes, refuses U+061C, U+180E, U+200B-200F, U+202A-202E, U+2060-206F, U+FEFF, U+E0000-E007F; reader skips a SESSION principal > 128 bytes | `red/bin-HEAD-red.log` (`the_boot_refuses_a_long_or_invisible_principal` RED at HEAD: a 129-byte principal boots) | bin `the_boot_refuses_a_long_or_invisible_principal`, `the_boot_parse_follows_the_butler_principal_rule`; cli `a_session_principal_over_128_bytes_is_skipped_...` |
| GPT: decision record said "pending", `kloc.toml` said "adjudicated" | both say ADJUDICATED by Main 2026-11-12 (for the surviving redesign), with the re-measured 1454 PENDING re-adjudication | n/a | `kloc.toml`, `20-14-flag-winston-decision.md`, story, tracker |
| GPT: report-key plant does not prove same-principal task separation | new real-session witness: one principal over two boots = two rows with distinct tasks and 50/30 each | green-before by construction (HEAD already separates); the plant is the red | bin `one_principal_on_two_tasks_reports_two_rows` |

Withdrawn wording: "stale scopes are pruned" (production never reaches that case; the old test is replaced). GLM's "TL redactor interplay: no mangling" was wrong for the frame insert path
(`insert_frame_event` redacts the payload): GPT was right, see the red logs.

## Plants (copy-back + `cmp` identical + `touch`; raw logs in `plants/`, summary `plants/plant-summary.jsonl`)

| Plant | What it breaks | Red witnesses |
|---|---|---|
| p1-revoked-scope-hidden | a revoked token's scope reads as none | capability lifecycle test; kernel paid-in-flight test |
| p2-bind-mints-for-absent-token | bind succeeds for a token the ring never held | capability lifecycle test; kernel delegates test |
| p3-delegate-always-binds | the adapter delegate always reports success | kernel delegates test |
| p4-encode-never-covers | redactor-shaped identity text journaled raw | domain (2) and bin vectors (3) |
| p5-decode-never-reads | the reader does not decode | domain (2) and bin vectors (3) |
| p6-erasure-matches-raw-form-only | `forget` compares the encoded text | bin vectors (3) |
| p7-boot-parse-without-caps | no length cap, no invisible/bidi refusal | bin boot-refusal test and parse unit test |
| p8-reader-without-principal-cap | reader accepts a 129-byte session principal | cli reader test |
| p9-report-key-ignores-task | report key drops the task | bin `one_principal_on_two_tasks_reports_two_rows`, `two_sessions_...` |

## Pins (exact formatted `cargo fmt --all` / `kloc-check --json`)

| Crate | Before | After | Note |
|---|---:|---:|---|
| maos-kernel-core tokei / physical | 21201 / 27040 | **21203 / 27043** (+2 / +3) | x2 ceiling 21221, shared STOP 21250 (47 left); net kernel growth +2 (<= +4 STOP); no surface refresh |
| maos-capability | 1444 | **1454** | +21 over 1433; the adjudicated figure was 1444: re-adjudication of +10 pending |
| maos-domain | 10217 | **10274** (+57) | pending adjudication |
| maos-iac | 8770 | **8771** (+1) | pending adjudication |
| maos-cli | 6609 | **6612** (+3) | pending adjudication (L1 overdraft ceiling 6610) |
| maos-bin | 38549 | **38556** (+7) | inside the +250 L2 allowance |
| aggregate / hardfail | 202855 / 202856 | **202935 / 202936** | measured + 1 |

Tooling note: tokei treats a `'"'` char literal as a string opener and miscounts the lines after it as code (+63 in `cost.rs`); the source uses `'\u{22}'` instead.

## Verification observed (env `-u CI -u GITHUB_ACTIONS`; cryptography venv on PATH; gate binary rebuilt from this tree)

- Focused (`green/`): capability `cost_scope_20_14` 1 passed; kernel `cost_attribution_20_14` 6 passed; bin `cost_attribution_20_14` 11 passed (real `maos` / `maosctl` / one-shot `forget`); cli lib `cost_*` 9 passed; domain `cost_frame_20_14` 3 passed; iac `cost_identity_cover_20_14` 1 passed.
- `cargo fmt --all -- --check` rc 0; workspace clippy rc 0, `clippy-ratchet` rc 0 (1331 occurrences / 869 source-distinct, unchanged: no new diagnostic); `cargo test -p maos-kernel-core --lib` 253 passed;
  `cargo test -p maos-bin --no-default-features --test verb_table_15_3` 7 passed; xtask `d11_xtask_ceiling_ratchet` 9, `kernel_pin_content_hash_16_0` 25, `recovery_lane_ceiling_rule` 12, `kernel_pin_ci_wiring_20_3a` 3, `clippy_ratchet_20_3b` 6 passed.
- Fixture builder rc 0, genuine `cargo build --locked -p maos-bin -p maos-cli` rc 0, ONE workspace nextest (`workspace-nextest-final.log`): **rc 100, 6040 run, 6039 passed, 1 failed (113.5 s)**:
  `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (61.752 s), the known AC5 local 1M oracle; not retried.
- Gates rc 0 (`gates/*.json`): kloc-check (aggregate 202935 <= hardfail 202936), check-kernel-baseline (27043), **check-service-boundary (no surface-hash refresh)**, check-dev-model-tier, check-dev-record-completeness, check-epic-close-coherence,
  check-exit-commands, check-decision-register, check-env-contract, check-serde-error-handling, check-fkcs, check-pub-field-constructors, check-dependency-closure, check-abi-ratification, check-manifest-schema-version, coverage-matrix,
  invariant-lock `--changed-files`, stability-matrix `--check`, gen-abi-docs `--check`. `docs/invariants/*.md` and `docs/ci-baselines/kernel-surface-v0.1-beta.json` untouched.
- Not run / unverified: `abi-diff` (refuses a dirty tree), `prepush`, hosted discipline, Apple and Windows legs, the WASM-component witness. The bounded per-bind work is a structural argument
  (one entry in one shard, no collection to scan; eviction removes the scope with the state), not a timing measurement.
