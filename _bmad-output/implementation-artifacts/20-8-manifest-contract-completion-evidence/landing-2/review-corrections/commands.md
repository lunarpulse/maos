# 20-8 L2 review corrections — commands and observed exits (Dev208K, 2026-11-11, base `a2bbd687`)

`env -u CI -u GITHUB_ACTIONS`, the repo `target` symlink, one cargo process at a time. Gates: prebuilt `/mnt/build/cargo-maos/debug/xtask`
(`CARGO_MANIFEST_DIR=<root>/xtask`). Clean KLOC figures: `git archive HEAD` overlaid with the corrections' modified/added files.

| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | T0 (read-only prep): `git archive HEAD` copy, items (1)+(2) prototype + `rustfmt`; `check-kernel-baseline`, `kloc-check` | 1 / 1 (drift) | kernel 27009/21178 → **27014/21181 = +5/+3**, inside 27043/21207 and STOP 21250 (priced to Main before the handoff) |
| 2 | red at HEAD: `cargo test -p maos-kernel-core --test swap_invariants_20_8` (two new vectors + the label check added) | 101 | 3 of 5 FAIL: scalar vector (swap completes), the label/receipt vectors; `red-head/red-head-kernel.log` |
| 3 | red at HEAD: genuine build (0), `cargo test -p maos-bin --test one_daemon_one_door_16_1 ac8_20_8_a_cold -- --test-threads=1` | 0, 101 | the cold swap of the dropping successor exits 0 instead of 1 (`red-head/red-head-door-cold.log`) |
| 4 | corrections applied; `cargo fmt --all` (same modified set); kernel test, genuine build, door tests | 0, 0, 0 | kernel 5/5, door `ac8_20_8` 2/2; kernel 27014 (`green/`) |
| 5 | five plants (`plants/`): `scalar-error-swallowed` 4.4 s, `raw-name-in-receipt` 4.4 s, `raw-name-under-hex-label` 4.1 s, `cold-swap-check-dropped` 21.1 s | 101 each | each red at its named assertion; restored by copy-back + `cmp` (rc 0) + `touch` (`plants.json`) |
| 6 | `cargo clippy --locked --workspace --all-targets --message-format=json`; `xtask clippy-ratchet` | 0, 0 | 5 m 13 s; zero diagnostics on any added line |
| 7 | `cargo test -p maos-kernel-core --lib`; the 13-binary focused kernel set | 0, 0 | 253/253; 13 binaries ok (`final/`) |
| 8 | `cargo test -p xtask --test d11_xtask_ceiling_ratchet --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule` | 0 | 9 / 25 / 12 passed |
| 9 | `cargo test -p maos-bin --no-default-features --test verb_table_15_3` | 0 | 7 passed |
| 10 | gates (`gates/`): check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-dev-model-tier, check-kernel-baseline, check-service-boundary (no flagged symbol), check-manifest-schema-version, check-equiv-fixture-provenance, check-pub-field-constructors, check-dependency-closure, check-corpus, check-abi-ratification, check-fkcs, check-serde-error-handling (now green at this base), `stability-matrix --check`, `gen-abi-docs --check`, `invariant-lock --changed-files` | 0 each | all pass; `abi-diff` not re-run (dirty-tree refusal, clean post-commit run owed) |
| 11 | `kloc-check --json` on the clean archive overlay | 0 | aggregate 202750, `over_budget []`: kernel 21181, bin 38475 (`gates/kloc-check-clean-archive-overlay.json`) |
| 12 | `xtask build-test-fixtures --json` (0); genuine build (0); `PATH=~/.venvs/maos-test/bin:$PATH cargo nextest run --workspace --locked --profile ci --no-fail-fast` — the ONE workspace run | 0, 0, 100 | 6018 run, **6017 passed**, 1 failed: `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (44.3 s; the documented AC5 1M oracle, 20-7-owned same-line rows, not retried) (`final/workspace-nextest-final.log`) |

Not run: hosted CI, `prepush`, `abi-diff`, the WASM-component witness, genuine 2.1/2.2 guests.
