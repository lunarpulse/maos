# First aggregate integration smoke — 2026-10-27

**Result: RED, not story completion.** The real aggregate command ran to completion; this is not the AC3 warm/no-change measurement or the AC4 route-manifest plant.

## Command and environment

```text
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo MAOS_17_2_CGROUP_MODE=undelegated cargo run -q --locked -p xtask -- prepush --json
```

Container mode is explicitly undelegated; no delegated systemd/T3 proof is claimed. Session raw log: `artifact://5026`; parsed full records retained in the local session artifact `local://prepush-smoke-5026.json` (not a portable repository link). An earlier attempt failed compilation with E0502 in the archive metadata mismatch branch (`artifact://5021`); parent fixed it by formatting the mismatch before mutating `step_pins`, rather than retaining references across mutation.

## Observed result

- Exit **1**, aggregate internal wall **756.686 s**, shell command wall **764.52 s**.
- Records: **62 pass, 19 red, 64 skip, 1 advisory**. Skip records are emitted lines, including negative-control tests' own SKIP diagnostics; this count must not be presented as 64 independently verified absent substrates.
- Workspace nextest: **5109 tests run, 5108 passed, 1 failed, 135 skipped**, actual test phase **40.013 s**. The sole failed test was `xtask::bin/xtask kloc_check::tests::kloc_check_runs_on_workspace`, which observed the deliberately un-repinned story tree.
- Kernel-core lib command: exit 0; workspace doctest command: exit 0.
- `build-test-fixtures`, build-free docs checks and release binary-size leg: exit 0.
- Conditional fresh docs build/routes and cargo-deny correctly reported named skips for unchanged trigger paths. No claim is made about their triggered branches from this run.
- The generated lineage report restore and generic tracked-tree preservation both reported pass. No tracked mutation was hidden by a reset.

## Reds and disposition

| Class | Observed failure | Disposition |
|---|---|---|
| KLOC gate + workspace KLOC test | xtask **47601 > 46364**; aggregate **188674 >= 187438** | Expected unlanded-tree refusal, not exempted. At this smoke the deltas separated as 20-15 +37 and 20-16 +1200; a later recorded state was 1172 + 7 = **+1179** (historical, superseded by the corrected +38/+1019 split in "Subsequent exact candidate gate check"). Operator reaffirmed keep +1020 and redesign, not raise it. No ceiling increase or done flip. |
| CI ANSI lint, 16 records | Parsed cargo output under workflow default `CARGO_TERM_COLOR=always` | Corrected with a colorless default (the named drift fix: `CARGO_TERM_COLOR` global `never`), preserving explicit nextest `--color never` and command-scoped override detection. No blanket reader exemption. A final full aggregate rerun is not yet recorded. |
| Guard-only selection reader | Escape-detector's literal regex `$` end-anchor was rejected, despite no count/marker pin on that diagnostic-only step | Subsequently corrected so diagnostic guards require ANSI safety while count/marker pins keep strict selection binding; full aggregate rerun pending. |

## Timing boundary

| Leg | Observed wall (s) |
|---|---:|
| Workspace nextest, including compilation | 315.004 |
| Workspace doctests, including compilation | 146.468 |
| Release binary-size build | 66.156 |
| Fixture prerequisite build | 46.972 |
| Kernel-core lib | 9.155 |

This first integration run included compilation and per-family pin-list builds. It is **not** a warm no-change benchmark and cannot establish the roughly 61-second creation forecast or the 2× reporting threshold. Required final warm and one-line incremental measurements remain pending.

## Additional independent 20-15 findings

The Opus no-fallback transport audit found three integration blockers before any hosted push: a hidden compiling rootless canary, no-capture per-process summaries being counted alongside nextest summaries, and consumer-generated checkout logs invalidating strict provenance. Repairs, additional review findings and their limits are recorded in `../20-15-evidence/local-smoke-2026-10-27.md`. No final hosted bundle success, savings measurement, or full independent review closure is claimed.

Subsequent focused verification passed **76 Rust / 30 Python** checks, including the explicit transport-mode repairs. These checks do not replace a complete aggregate rerun. The real corrupt-archive refusal and local mini-Git driver are not the hosted missing-archive plant or an in-place story-tree plant. Human-mode aggregate smoke, the AC4 in-place route-manifest plant, final warm and one-line incremental timings, full review and landing runs are still required.

## Subsequent exact candidate gate check

- *Historical (2026-10-27, first candidate):* after formatted source measurement the STOPs were met as 20-15 **+37/+38**, 20-16 **+1016/+1020**, combined pin **47417**, aggregate **188490**, hardfail **188491**. **Superseded** by the post-correction exact grant: 20-15 **+38** (STOP38 exactly), 20-16 **+1019** (`prepush.rs` 1013 + `main.rs` +6, STOP1020); xtask **47421**, aggregate measured **188494**, hardfail **188495**. Story records and pin HISTORY carry the separate allocations.
- Parent ran `cargo nextest run --locked -p xtask --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule`: **20 passed, 0 skipped**.
- Parent then ran `kloc-check --json`: `passed:true`, `alarm:true`, no over-budget crates; at that time xtask 47417 and aggregate 188490 (historical; current candidate 47421 / 188494). The existing aggregate alarm is not a failed hard limit or the ADR-069 kernel alarm.
- `check-kernel-baseline` passed: **26367 physical lines, 95 files**, unchanged pin; KLOC measured kernel **20610**. Combined command wall **7.69 s**; raw session log `artifact://5117`.
- This resolves the candidate's obsolete-pin RED only. The first aggregate result above remains historical RED; at this point no complete post-fix prepush success was recorded (the later bg_19 observation below is pre-review-patch and not final-tree proof), and no final review, hosted landing, savings or E19 landed-base rebase is claimed.

## Observed post-fix full actual prepush (bg_19) — 2026-10-27

- JSON-mode full run; raw `artifact://5134`, parsed session artifact `local://prepush-postfix-5134.json` (not a portable repository link). **No red records: 64 pass, 64 skip, 1 advisory.** Internal wall **671.768 s**, shell wall **672.71 s**; base `271e2a33768a8f2b75c6cd41bd6f0c82d0526f39`; **68 changed paths**.
- This **predates the current review-correction patches** and is not designated warm/no-change (AC3), final-tree, plant, human-mode, hosted or landing proof. The 64 skip records include arbitrary captured negative-control diagnostics; they are **not** 64 independently absent substrates.
- It supersedes the first RED above only as an observation of that earlier snapshot; the first aggregate remains the historical record.

## D-20-16-02 C addendum and triage (2026-10-27)

- Operator addendum to D-20-16-02: **C** — dynamic-only markers are a named indeterminate SKIP; statically determinate markers compare exactly; a missing emission site is RED; dynamic actual counts remain CI-only. Original ruling A stays historical and is not read as having already permitted C. CI runtime marker guards are not weakened.
- A1 resolved by this addendum; T2 remains relevant under C. Add-only prepush assertions were removed and the archive-family/runtime regressions fixed; the final triage (fixed/dismissed with reasons) and parent verification figures are in the 20-15 and 20-16 Dev Agent Records. Producer-parse P3 (unknown-field erasure) was fixed with `deny_unknown_fields` on the 20-15 side; transport still refuses an unsealed producer. Final independent approval remains outstanding.
- Model roles are a SESSION-ONLY substitution (Opus 5.5 coding/high-risk, Sonnet 5.5 light reviews/docs; earlier Terra substitution superseded); standing tracked policy unchanged.

## Human-mode prepush bg_21 — 2026-10-27 (before the latest corrections)

- Exit **1** with **ONE** red: `tracked-tree`. Cause: the orchestrator was concurrently editing `.devcontainer/docker-compose.yml` and the sprint tracker during the run, so the tracked-tree guard correctly reported modified tracked files; it is correct guard detection, not a product defect.
- Otherwise 63 checks ran, 8 skipped-with-reason, 56 captured SKIP diagnostics (not independently absent substrates); internal wall **475.018 s**.
- Predates the final post-correction changes; not final-tree, AC3 warm/incremental, plant, hosted or landing proof. A final post-correction full prepush is not yet recorded.

## Final post-correction JSON prepush — 2026-10-27 (exclusive cargo)

- Earlier post-correction run bg_27 was killed at 3600 s and a diagnostic rerun at 1500 s. Root cause was external, not prepush: `/mnt/build/cargo` was deleted and recreated at 15:28:31 mid-run (reds "couldn't create a temp dir", "failed to move dependency graph", sccache fatal), and a separate `docker exec` session (pid 2358205) ran its own `cargo build -p xtask && xtask prepush --json` against the same target from ~15:36. Per operator ruling it was not killed; it exited 1 after 777 s (its only red: `error-catalog-check: CARGO_MANIFEST_DIR not set`, from running the binary outside `cargo run`).
- With no other cargo user and the target unchanged across the run (inode 97949418): `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo timeout 2400 cargo run -q -p xtask -- prepush --json` → **exit 0**, wall **232 s** (internal 231.642 s). Records **62 pass, 8 skip, 1 advisory, 0 red**; workspace-nextest 94.1 s, workspace-doc 8.9 s, kernel-core-lib 1.9 s, build-test-fixtures 4.4 s; binary-size trigger/build/check pass.
- Skips: `docs:build`/`docs:routes` (no docs-site, docs/errors or error-catalog changes), `cargo-deny` (no Cargo.lock/deny.toml change), five dynamic `ci-pins:marker` (operator C: SCOPED-VERDICT, GOVERNED-ATOMIC, HOSTILE-LIFTING, HOSTILE-LIFTING-CONTROL, RUNNER-SIGSYS). Advisory `test-budget`: local timing findings `maos-iac::participant_recall_17_8` 43.216 s and `xtask::bin/xtask` 31.371 s Σ per-test (local ≠ CI).
- The target had been rebuilt by the external run, so this is a warm measurement of the uncommitted candidate, not the AC3 warm/incremental pair on the committed landing tree; `cargo fmt --all -- --check` exit 0.
