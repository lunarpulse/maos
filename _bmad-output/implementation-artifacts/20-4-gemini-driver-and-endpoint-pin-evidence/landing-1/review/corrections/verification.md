# Review correction verification

Direct verification commands retain inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos`, absolute paths, and `env -u CI -u GITHUB_ACTIONS`. The registered fixture producer also uses its declared existing probe target paths; no agent-set target directory or extra worktree was introduced. No cargo clean, commit, push, kernel edit, or status flip was used.

## Observed focused checks

| Check | Observed result | Exit | Evidence |
|---|---:|---:|---|
| Complete `maos-providers` package, serial | 64 passed; 3 existing ignored | 0 | `logs/provider-package-green.log` |
| Final `xtask::model_currency_gate`, serial | 25 passed, including commented/cfg definition spoofing and unqualified/foreign reference refusal | 0 | `logs/currency-final-guarded-green.log` |
| Five affected root binaries, serial | 46 passed | 0 | `logs/five-root-qualified-final-green.log`, `five-root-serial-junit.xml` |
| Final full workspace inventory, default concurrency, profile `ci` | **5613 passed, 138 skipped**, 660 captured test binaries; 42.932 s execution / 44.18 s command | 0 | `logs/workspace-default-green.log`, `workspace-default-junit.xml` |
| Non-denying workspace/all-target clippy | 1331 occurrences / 871 source-distinct; successful build-finished event | 0 | `clippy.jsonl`, `clippy-summary.md` |
| Clippy ratchet | unchanged from prior local 1331/871; below hosted 1383/897 by 52/26 | 0 | `commands.json` |
| `cargo deny` | passed with existing allowance/duplicate warnings | 0 | `commands.json` |

The five root binaries are `maos` unit tests, `provider_selection_20_4`, `provider_endpoint_pin_20_4`, `enterprise_identity_wiring`, and `enterprise_daemon_seam_13_5a`. The final provider source was unchanged after its complete serial package run. Final gate outputs and exact commands/exits are retained in `gates/`, `commands.json`, and `commands.md`.

## Default-workspace prerequisite failures were retained, not retried as flakes

An initial full default run passed 5612/5612 before the additional shared-constant spoofing vector. Later ordinary workspace runs exposed:

1. 5604 passed / 9 failed / 138 skipped: every failure was the journey helper's stale `debug/maosctl` check against `maos-bin/src/main.rs` (`logs/workspace-final-stale-journey.log`, `workspace-stale-journey-junit.xml`, exit 100).
2. 5603 passed / 10 failed / 138 skipped: the same nine stale journey binaries plus the xtask unit binary's old embedded KLOC pin, compiled before the final exact re-pin (`logs/workspace-cache-swap-and-embedded-pin.log`, `workspace-cache-swap-junit.xml`, exit 100).

The helper-prescribed real `cargo build -p maos-bin -p maos-cli` made `maosctl` genuinely newer than the source: observed mtimes 1793733524 versus 1793732613. A subsequent workspace build restored the cached feature-set executable with mtime 1793731032. The source mtime did not change. This is a build-alias/freshness prerequisite problem, not a timing flake or a driver failure. No binary/source timestamp was forged, no assertion was suppressed, and journey source was not changed.

The final supported build-once protocol was:

1. Capture final workspace binary metadata with `cargo nextest list --workspace --profile ci --list-type binaries-only --message-format json`, after the exact re-pin. This also recompiles the xtask embedded pin.
2. Capture full `cargo metadata --locked --format-version 1`.
3. Run the prescribed genuine CLI package build **after** workspace binary capture.
4. Run nextest with `--cargo-metadata` and `--binaries-metadata`, profile `ci`, no fail-fast, and default concurrency. This uses the exact captured workspace inventory without another Cargo feature-cache alias swap.

The first two metadata-run drafts were rejected by Clap (exit 2): `--manifest-path` conflicts with `--cargo-metadata`, and `--workspace` conflicts with `--binaries-metadata`. They ran no tests and are preserved in the command record. The valid final command (executed, exit 0) was:

```sh
env -u CI -u GITHUB_ACTIONS /home/lunarpulse/.cargo/bin/cargo nextest run --cargo-metadata /tmp/204l1-review/cargo-metadata.json --binaries-metadata /tmp/204l1-review/binaries-metadata.json --profile ci --no-fail-fast
```

Byte-preserved copies of both inputs are `workspace-cargo-metadata.json` and `workspace-binaries-metadata.json`; hashes are in `SHA256SUMS`. For reproduction after scratch cleanup, substitute those retained absolute evidence paths. This is the same 5613-test inventory, not a narrowed package run or a new target directory.

No flaky failure was observed in this review pass. Therefore no new owner-20-7 flake assertion was added to `deferred-work.md`; the deterministic prerequisite failures above remain explicitly recorded instead.

## Bounds and unverified work

Final formatted code: providers 1679 / ceiling1898; bin34504 /34643; xtask44452 /44466; domain10107 /10115; manifest4604 /4610; aggregate193140 /193526. Exact aggregate hardfail is 193141 (= measured + 1). Kernel remains 26841 physical /21042 tokei, 95-file baseline unchanged. No envelope was breached.

Per the review assignment, full serial workspace nextest, a fresh full-serial-workspace `check-test-budget` audit, and the full prescribed prepush were not repeated. Serial verification covers the affected provider/currency/root suites. Paid Gemini/Kimi calls, hosted workflow execution, hosted-only clippy behavior, and all Landing 2 kernel work remain unverified/unperformed; the existing unpushed hosted/live cut was not changed. No previous non-green prepush observation is erased or relabelled by this review pass.

## Acceptance mapping for this correction pass

Original L1 per-AC HEAD/plant evidence remains in `../../plants.md`; this correction pass does not claim fresh HEAD reds for unchanged acceptance paths.

| AC | Correction evidence |
|---|---|
| AC1 | P1/P2/F1/F2/F4/F5 provider guards: thirteen observed HEAD-source reds become green; the existing native wire/header/status target stays green. Named line citations are in `disposition.md`. |
| AC2 | Compatible usage/stop/transport guards have observed reds and greens; Kimi and existing OpenAI round trips remain green in the 21-test focused provider integration run. |
| AC3 | P3 Gemini/Kimi real-registration omission plants each yield native red / explicit replay green; separate old-key precedence conflicts pass. |
| AC4 | F6 actual gate refusal, duplicate declaration, comment/cfg spoofing reds become the final 25-test currency suite and 22-pin gate green. |
| AC5 | Selection/admission behavior is intentionally unchanged; existing selection witnesses pass inside the final serial root run. No new red is claimed for unchanged code. |
| AC6 | Catalog/actual-driver endpoint mismatch plant reds; the restored capturing-transport unit and existing endpoint-pin typed-refusal integration witnesses pass. Pin opt-in is clarified in operator docs. |
| AC7–AC8 | Landing 2 excluded; kernel unchanged and no new proof claimed. |
| AC9 L1 | Dev Agent Record, dispositions, exact commands, raw/JUnit evidence and exact pins updated. No closure/status/hosted claim is promoted. |

## Completed cleanup

After final gates, the explicit `/proc` scan at 2026-11-03T19:48:28Z checked 26 executable processes, found no Cargo/rustc/nextest/clippy process and no unreadable live pid. Removed 87 stale incremental session directories; retained 5467 newest-per-crate sessions plus 1302 additional recent sessions. Recency conservatively uses maximum descendant mtime or ctime and a two-hour cutoff, rechecked before each deletion. Only `/mnt/build/cargo-maos/debug/incremental/*/s-*` candidates were pruned; no deps/build/fingerprint directory was touched.

Owned `/tmp/204l1-review` was removed after preserving all required proof and metadata inputs here. Protected `/tmp/204l1-wip/{tracked.patch,new-files.tar.gz,SHA256SUMS}` remains. No new repository `.orig`, `.rej`, or stray scratch output was created.

| `du -s -B1` scope | Immediately before | After | Observed freed bytes |
|---|---:|---:|---:|
| `/mnt/build/cargo-maos` | 222472871936 | 221931663360 | **541208576** |
| `/tmp` | 1428324352 | 1418465280 | **9859072** |

`cleanup.json` records the exact removed sessions and process/recency proof.
