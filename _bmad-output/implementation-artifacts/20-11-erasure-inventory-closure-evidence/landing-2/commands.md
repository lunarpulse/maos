# 20-11 L2 — commands and observed exits (Dev2011k, 2026-11-11, base `345ae30f`)
All cargo runs: `env -u CI -u GITHUB_ACTIONS`, repo `target` symlink (`/mnt/build/cargo-maos`), one cargo process at a time, `~/.venvs/maos-test/bin` on PATH.
Gates: prebuilt `/mnt/build/cargo-maos/debug/xtask`, `CARGO_MANIFEST_DIR=<repo>/xtask`, `--json` into `gates/`.

| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | T0: `git archive HEAD` copy + KS + O4 prototype diffs + rustfmt; `check-kernel-baseline`, `kloc-check`, `check-service-boundary` | 0 / 1 (drift, expected) / 0 | kernel 26854/21051 → 26880/21069 (+26/+18 = grant), domain +4, surface gate passes with one hash refresh (`t0-*`) |
| 2 | red: `cargo test -p maos-kernel-core --test halt_unload_fence_20_11` (HEAD + domain variant) | 101 | 3 failed (`logs/o4-red-head-plus-variant.log`) |
| 3 | red at HEAD production: `erasure_inventory_20_11 ac3_`; `erasure_live_20_11 ac2_live` | 101, 101 | `logs/ac3-offline-red-head.log` (3/3), `logs/ac4-live-red-head.log` |
| 4 | plants O4(a), O4(b), KS-1, KS-2, AC3 fresh-registry, AC3 door read-back, KS door, verifier kind | 101 each | `logs/*plant*.log`; each restored by copy-back + `cmp` rc 0 + `touch` |
| 5 | final-tree witnesses `final/f1…f9` (kernel O4+KS 4/4, kernel `--lib` 253, audit 11, bin inventory 20, door 2, composition 7, live 2, xtask pin tests 9+25+12, no-default verb table) | 0 each | `final/f*.log`, `final/exits3.txt` |
| 6 | `cargo clippy --locked --workspace --all-targets --message-format=json` + `xtask clippy-ratchet --baseline xtask/clippy-baseline.toml` | 0, 0 | 1328 occurrences / 868 source-distinct (equal to the 20-4 L2 figure; none on the lines added here) |
| 7 | `xtask check-fkcs --json` (own cell) | 0 | `passed: true`, `blocking_red_legs []` (run before and after the final main.rs repair) |
| 8 | `xtask build-test-fixtures --json`; genuine `cargo build -p maos-bin -p maos-cli`; `cargo nextest run --workspace --profile ci --no-fail-fast` — **first run (defective tree)** | 0, 0, 100 | 5987 run, 5967 passed, **20 failed** (19 `audit writer drain timed out after 5s` caused by my unconditional `ScheduleWatchdog`; 1 known AC5 1M) — `final/workspace-nextest.log` |
| 9 | after the repair (watchdog built only by its consumer, `Weak` slot): the nine affected bin binaries and journey_j0 / journey_skill_authoring_18_7 | 0 | `final/retest2-bin.log`, `final/retest3-journey.log` |
| 10 | **final** fixtures (after `touch` of the two `main.rs`), genuine bin/CLI build, ONE workspace `cargo nextest run --workspace --locked --profile ci --no-fail-fast` | 0, 0, 100 | 5987 run, **5986 passed**, 1 failed (`maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption`, 43.4 s, the documented local row, not retried or widened), 137 skipped, 95.5 s — `final/workspace-nextest-final.log` |
| 11 | gates: check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-dev-model-tier, check-kernel-baseline, check-service-boundary, `invariant-lock --changed-files gates/changed-files.txt` | 0 each | `gates/*.json`, `gates/rc.txt` |
| 12 | `kloc-check --json` on a `git archive HEAD` copy overlaid with the landing files and the re-pinned `xtask/kloc.toml` | 0 | aggregate 202370, `over_budget []` (`gates/kloc-check-clean-archive-overlay.json`); in-tree kloc-check fails only because of the operator's untracked spikes/ and *gpui* files |
