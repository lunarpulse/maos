# 20-1b landing 2 r1 — commands and observed exits (Dev201bK, 2026-11-12, base `692b1e59`)

Handoff `CARGO HANDOFF 201bK-r1`; ruling `q` (narrow, no kernel code) and `size`. `env -u CI -u GITHUB_ACTIONS`, one cargo process at a time,
`~/.venvs/maos-test/bin` on PATH for verifier tests. Kernel unchanged (27,009 / 21,178).

| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | red at HEAD: `cargo test -p maos-audit --test fr4_classifier_16_2` with the new vector (+ its measured fixture) | 101 | historical row classifies as `Call` (`red-head/red-fr4.log`) |
| 2 | red at HEAD: `cargo test -p maos-bin --test yank_policy_20_1b a_parked_invalid` | 101 | one row per poll, not once (`red-head/red-f2.log`) |
| 3 | red at HEAD: `cargo test -p maos-bin --test erasure_inventory_20_11 review_serde` | 101 | no gap for a non-JSON kind-3 row (`red-head/red-serde.log`) |
| 4 | red at HEAD limits (restored `git show HEAD:` files): `cargo test -p xtask --test binary_size_limit_pin_20_1b` | 101 | `max_size=33554432` (`red-head/red-size.log`); files restored by copy-back + touch |
| 5 | fixes applied; same tests | 0 | fr4 23/23, yank 12/12, erasure_inventory 25/25, size pin + prepush tests 1+16+5 (`green/`) |
| 6 | first serde fix without the empty-payload exception | 101 | 5 existing `erasure_inventory_20_11` tests red (kernel termination markers are empty kind-3 rows); corrected to non-empty payloads only |
| 7 | researcher variant `cargo test -p maos-bin --test erasure_live_20_11 -- --nocapture` | 0 | 2 passed in 2.85 s; the worker variant prints `SKIP: T3 ... substrate not provisioned` (unverified locally) (`green/green-erasure-live.log`) |
| 8 | five plants (`plants/`): `fr4-historical-prefix` 4.5 s, `f2-no-dedup` 14.2 s, `serde-silent-default` 7.0 s, `erasure-live-researcher-expects-empty` 4.7 s, `size-prepush-old-limit` 16.7 s | 101 each | each restored by copy-back + `cmp` + `touch` (`plants.json`) |
| 9 | `cargo test -p maos-bin --no-default-features --test verb_table_15_3` | 0 | 7 passed |
| 10 | `cargo clippy --locked --workspace --all-targets --message-format=json` + `xtask clippy-ratchet` | 0, 0 | 1331 / 869 (baseline 1383 / 897); no diagnostic on any line added by r1 (the +3 / +1 over the previous 1328 / 868 are not on r1 lines) |
| 11 | `xtask build-test-fixtures`, genuine `cargo build -p maos-bin -p maos-cli`, ONE `cargo nextest run --workspace --locked --profile ci --no-fail-fast` | 0, 0, 100 | 6015 run, 6013 passed, **2 failed**: the documented 1M oracle (`erasure_million_20_11`, 43.1 s, unchanged, not retried) and `xtask::coverage_matrix_20_3b committed_hand_kept_matrix...` caused by **my** YAML note (a `: ` inside an unquoted scalar) |
| 12 | fix: quote the FR13 note; `cargo test -p xtask --test coverage_matrix_20_3b --test coverage_matrix_integration --test coverage_matrix_completeness_tests` | 0 | 3 + 3 + 4 passed (targeted re-run of the three coverage-matrix binaries only; no second full workspace run) |
| 13 | gates: record completeness, epic-close coherence, exit commands, decision register, env contract, dev-model tier, kernel baseline (27,009), service boundary, pub-field constructors, dependency closure, **check-serde-error-handling**, `invariant-lock --changed-files`, `check-fkcs` | 0 each | all green (`gates/`) |
| 14 | `kloc-check --json` on a `git archive HEAD` copy overlaid with the r1 files | 0 | aggregate 202718, `over_budget []`: bin 38446, audit 8308, kernel 21178 |

Not run: hosted CI, `prepush`, the T3 worker variant of `ac6_running_t3_worker_uninstall_proves_live_inventory`, the AC11.5 / AC15 hosted legs.
