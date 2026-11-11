# 20-4 L2 — commands and observed exits (2026-11-11, working-tree base `89b64566`)

All cargo runs: `env -u CI -u GITHUB_ACTIONS`, inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos` (repo `target` symlink), one cargo process
at a time. Gates: prebuilt `/mnt/build/cargo-maos/debug/xtask`, `CARGO_MANIFEST_DIR=<repo>/xtask`, `--json` into `gates/`.

| # | Command | rc | Observed | Evidence |
|---|---|---:|---|---|
| 1 | T0: `git archive HEAD` copy + `patch -p1` of `creation/r-ka-kio1-kio2-bundle.diff` + `rustfmt`; `check-kernel-baseline --json`, `kloc-check --json` from the copy root | 1, 1 (pins unmoved, expected) | physical 26854 (+13), tokei 21051 (+9), set hash `2c223219…`, 95 files, aggregate 202121 | `t0-measurement.md`, `t0-*.json` |
| 2 | red-before: `cargo nextest run -p maos-kernel-core --test inference_provider_ids_20_4 --test io_redaction_and_status_20_4 --no-fail-fast` (new tests, kernel sources = HEAD) | 100 | 1 passed (fail-closed control), 4 failed | `logs/red-before-head.log` |
| 3 | `patch -p1 < r-ka-kio1-kio2-bundle.diff` in the repo, `cargo fmt --all` | 0 | `git diff --stat`: inference/mod.rs +2, io/mod.rs +15/−4 | — |
| 4 | green: same two binaries + `io_exchange_18_8` + `inference_fallback_authz_18_1` | 0 | 15/15 | `logs/green-after.log` |
| 5 | plants 1–3 (`plants.md`) | 100 ×3 | each named witness red, restored by copy-back + `cmp` rc 0 + `touch` | `logs/plant-*.log` |
| 6 | final: the two new binaries | 0 | 5/5 | `logs/final-new-binaries.log` |
| 7 | `cargo test --locked -p maos-kernel-core --lib` | 0 | 253 passed | `logs/kernel-lib.log` |
| 8 | `cargo nextest run -p xtask --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule` | 0 | 37/37 | `logs/xtask-pin-tests.log` |
| 9 | `check-kernel-baseline` (in-tree, after re-pin) | 0 | 26854 / 26854, 95 files, set hash `2c223219…` | `gates/check-kernel-baseline.json` |
| 10 | `kloc-check --json` in-tree; and on a `git archive HEAD` copy overlaid with the landing files | 0, 0 | kernel 21051, aggregate 202121, `over_budget []` (both) | `gates/kloc-check-intree.json`, `gates/kloc-check-clean-archive-overlay.json` |
| 11 | `check-service-boundary`, `check-env-contract`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`, `check-dev-model-tier`, `check-corpus` | 0 each | (recorded again after the final tracker edit, see story) | `gates/*.json` |
| 12 | `check-fkcs` in its own cell | 0 | all legs green incl. `kernel-abi-diff`, `blocking_red_legs []`, 10.3 s | `gates/check-fkcs.json` |
| 13 | `invariant-lock --changed-files <landing file list> --json` | 0 | `passed: true` | `gates/invariant-lock.json`, `gates/changed-files.txt` |
| 14 | `cargo clippy --locked --workspace --all-targets --message-format=json` + `clippy-ratchet --baseline xtask/clippy-baseline.toml` | 0, 0 | 1328 occurrences / 868 source-distinct (hosted baseline 1383 / 897); none on lines `inference/mod.rs:96-101`, `io/mod.rs` changes, or the two new test files | `clippy.jsonl`, `logs/clippy-ratchet.log` |
| 15 | `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1` | 0 | 7 passed | `logs/verb-table-no-default.log` |
| 16 | fixtures `xtask build-test-fixtures --json`; `cargo nextest list --workspace --profile ci --list-type binaries-only`; `cargo metadata`; genuine `cargo build -p maos-bin -p maos-cli`; `cargo nextest run --cargo-metadata … --binaries-metadata … --profile ci --no-fail-fast` (python `cryptography` venv `~/.venvs/maos-test` on PATH) | 0,0,0,0,**100** | 5976 run: **5973 passed**, 3 failed, 137 skipped, 94.5 s test span. Failures: `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (documented 20-7/20-11 local row, not retried), and two **pre-existing** reds `xtask::d11_xtask_ceiling_ratchet::{the_live_xtask_ceiling_cites_a_real_sprint_story,the_retirement_row_matches_its_exact_story_arrow}` (`Err(MissingStory)`: the xtask row's first HISTORY segment, written by `89b64566`, cites no complete sprint story key with `->`; neither the xtask row nor `sprint-status.yaml`'s xtask lines are touched by this landing) | `logs/workspace-nextest.log`, `workspace-nextest-junit.xml` |
| — | `prepush` | skipped | not run: its `workspace-nextest` leg is the one workspace nextest above; the L1 prepush was already non-green under load with 20-7-owned timing advisories, and an extra 4500 s-class run adds no new kernel evidence | — |
