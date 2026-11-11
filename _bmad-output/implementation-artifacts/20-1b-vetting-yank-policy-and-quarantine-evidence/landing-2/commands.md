# 20-1b L2 — commands and observed exits (Dev201bK, 2026-11-11, base `c2beedb4`)

All cargo runs: `env -u CI -u GITHUB_ACTIONS`, repo `target` symlink (`/mnt/build/cargo-maos`), one cargo process at a
time. Gates: prebuilt `/mnt/build/cargo-maos/debug/xtask`, `CARGO_MANIFEST_DIR=<root>/xtask`, `--json` into `gates/`.
Clean KLOC/pin figures: a `git archive HEAD` copy overlaid with the landing's changed/added files (the operator's
untracked `spikes/`, `*gpui*` and the 2026-11-11 sprint-change-proposal are excluded).

| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | T0: `git archive HEAD` copy + creation `krec-kq1-p-bundle.diff` re-cut + `rustfmt`; `check-kernel-baseline`, `kloc-check`, `check-service-boundary` | 1 / 1 / 1 (drift, STOP-lines, one surface symbol: expected) | kernel 26889/21073 -> **26967/21141 = +78/+68 = grant**; surface: only `CapabilityRegistryAdapter` flagged (`t0/`) |
| 2 | red at HEAD: `cargo test -p maos-kernel-core --test quarantine_fr13_20_1 --test on_revocation_three_actions --no-fail-fast` | 101 | 4 new vectors + the updated existing test fail (`red-head/red-head-kernel.log`) |
| 3 | red at HEAD: `cargo test -p maos-bin --test yank_policy_20_1b` with `is_quarantined`->`is_paused` (HEAD has no fence API); then the policy-matrix test alone | 101, 101 | AC8/hostile vector red; policy-matrix `false` vs `true` (`red-head/red-head-bin-policy*.log`); the four PoisonError failures in the first log are mutex cascades, not witnesses; test file restored by copy-back + `cmp` + `touch` |
| 4 | red at HEAD: genuine `cargo build -p maos-bin -p maos-cli` (0) then `cargo test -p maos-bin --test one_daemon_one_door_16_1 ac12_20_1b -- --test-threads=1` | 0, 101 | 12 `poll_once failed` lines in ~1.5 s (`red-head/red-head-poller.log`) |
| 5 | kernel change applied; `cargo fmt --all`; the same three groups | 0, 0, 0 | kernel 4/4 + 1/1, bin 11/11, poller 1/1 (`green/`) |
| 6 | six plants (`plants/`): `ac11-drain-in-arm` 4.2 s, `ac11-unload-keeps-fence` 4.1 s, `ac8-effect-ignored` 12.3 s, `ac8-unrelated-spirit-matched` 6.1 s, `ac8-hostile-row-accepted` 9.6 s, `ac12-print-every-failure` 11.4 s | 101 each | each restored by copy-back + `cmp` (rc 0) + `touch` (`plants.json`) |
| 7 | `cargo test --locked -p maos-kernel-core --lib` | 0 | 253 passed (`green/kernel-lib.log`) |
| 8 | `cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3` | 0 | 7 passed (`final/verb-table-no-default.log`) |
| 9 | gates (final tree, `gates/`): check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-dev-model-tier, check-kernel-baseline, check-service-boundary, check-pub-field-constructors, check-dependency-closure, `invariant-lock --changed-files gates/changed-files.txt`, `check-fkcs` (own cell, 8 s) | 0 each | all pass; kernel pin 26967, set hash `06386b44...` |
| 10 | `check-serde-error-handling` | 1 | one violation, `crates/maos-bin/src/main.rs:11090` (`serde_json::from_str(&text).unwrap_or_default()`), **present at HEAD `c2beedb4` line 11092 (introduced by `5ff0ead1`)**; not caused by this landing; deferred row owner 20-11 |
| 11 | `kloc-check --json` on the clean archive overlay | 0 | aggregate 202614, `over_budget []`: kernel 21141, bin 38392, audit 8297 (`gates/kloc-check-clean-archive-overlay.json`); the in-tree run fails only because of the operator's untracked files |
| 12 | AC13 negative control: scratch copy with `20-1b` flipped to `done` while its `deferred-work.md` rows remain open | 1 | 39 `STALE owner ... sprint status is done` violations (`gates/neg-control-record-completeness.*`); the unflipped copy and the real tree pass |
| 13 | `cargo clippy --locked --workspace --all-targets --message-format=json`, `xtask clippy-ratchet --baseline xtask/clippy-baseline.toml` | 0, 0 | see `final/clippy-ratchet.log` (no diagnostic on any line added by this landing) |
| 14 | `xtask build-test-fixtures --json`, genuine `cargo build -p maos-bin -p maos-cli`, `cargo nextest run --workspace --locked --profile ci --no-fail-fast` — **run 1** | 0, 0, 100 | 6007 run, 5999 passed, **8 failed**: 6 `fixture stale` refusals (my ordering: I `touch`ed `main.rs` after the fixture build — an invalid run, not product failures), `fr4_classifier_16_2` (**real finding**: the new writer site and the renamed marker were unclassified in `WRITER_SHAPES`), and the documented 1M oracle (`final/workspace-nextest-run1-stale-fixture-and-fr4.log`) |
| 15 | fix: two `WRITER_SHAPES` entries + fixture; `cargo test -p maos-audit --test fr4_classifier_16_2` | 0 | 22 passed (`final/green-fr4.log`) |
| 16 | fixtures rebuilt, genuine bin/CLI build, ONE more full `cargo nextest run --workspace --locked --profile ci --no-fail-fast` on the final tree | 0, 0, 100 | 6007 run, **6006 passed**, 1 failed: `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (42.6 s; the documented AC5 1M oracle, same-line owner 20-7 rows already in deferred-work; never retried or widened); 137 skipped (`final/workspace-nextest-final.log`) |

Not run: hosted CI, `prepush` (not requested for this landing), the AC11.5 hosted T3 leg, the AC15 hosted TLS/dev/macOS legs.
