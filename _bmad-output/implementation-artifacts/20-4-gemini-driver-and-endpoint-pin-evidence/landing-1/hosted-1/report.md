# Hosted-1 regression correction

Baseline: clean `76efc19d` supplied by Main; regression source `73e7afbc`.
No commit, push, kernel change, status flip, dependency change or budget exception.

## Code and witnesses

- `crates/maos-bin/src/main.rs:16802,16825`: gate the admission catalog and its provider unit module on network. Keep shared defaults attribute-free; no library or CI-gate weakening.
- Exact no-default-feature `verb_table_15_3`: observed red exit101 before correction (two E0433), green exit0, 7 passed afterward.
- `xtask/src/check_model_currency.rs:238`: lex root items, reject nonconstant prefixes, discard opaque brace bodies before syn parses only candidate constants. Retain original spans, root binding, qualified reference and attribute/shadow refusals; no cross-audit cache.
- `xtask/tests/model_currency_gate.rs:672`, `shared_default_scan_skips_unrelated_item_bodies_hosted_1`: old whole-file AST observed red exit101; final suite exit0, 26/26 (all original25 names retained).
- Full original/correction diff audit and source grep are retained in `network-audit.md`, `network-symbol-grep.json` and command logs.

## Observed checks

| Command scope | Exit / result |
|---|---|
| Exact no-default verb-table control | 0; 7 passed |
| Default verb-table plus catalog unit, serial nextest | 0; 8 passed,24 filtered |
| `check -p maos-bin --no-default-features --all-targets` | 101; untouched `eval_halt_18_2.rs:26,353,357` network-only test imports; outside L1/correction diffs, Main notified, no authorization to alter that older test |
| Native all-target check `network` | 0 |
| Native all-target check `fixture_replay` | 0 |
| Native all-target check `network,wasm-host` | 0 |
| Model-currency serial nextest, fresh final JUnit | 0; 26 passed,15.879 s sum |
| fmt | 0 |
| Full non-denying workspace/all-target clippy | 0; build-finished success=true |
| Unchanged clippy ratchet | 0; 1331 occurrences/871 distinct, delta0/0 versus prior local; below hosted1383/897 |

Final gates, each exit0 (raw named logs in `gates/`):
kloc-check; check-kernel-baseline; check-service-boundary; check-env-contract;
check-serde-error-handling; check-composition-root-completeness;
check-j1-loopback-delegation; check-dev-record-completeness;
check-epic-close-coherence; check-review-findings-resolved; check-test-cost;
invariant-lock; coverage-matrix; check-coverage-matrix-completeness;
check-exit-commands; check-decision-register; check-ff-j6;
check-perf-runner-policy; check-model-currency.

The unchanged budget is assessed with the expressly permitted fresh per-test
serial JUnit sum, not by pretending a focused JUnit contains every exception
binary required by the whole-workspace check-test-budget gate.

## Cost evidence and limits

Hosted failing run41815429165/job126540412403: 30.601 s/25 versus30.000 s.
Fresh local before13.640 s/25; after15.879 s/26, so no local wall-sum speedup
is claimed. Five controlled independent audits measured mean user CPU
0.5305912 →0.4310398 s (18.762% reduction), wall0.5437248 →0.4401365 s.
The full AST had measurable cost, not established as the majority of audit cost.
See `performance.md`, profiles, per-test timing JSON and both JUnits.
Rejected stale measurement drafts and intermediate attempts remain labeled as
such. `/usr/bin/time -v` was unavailable (exit127); it supplies no profile claim.
No hosted-budget green is claimed without a hosted rerun.

## Exact formatted envelope

| Scope | Before | After | Delta | Ceiling |
|---|---:|---:|---:|---:|
| maos-bin |34551|34552|+1|34643|
| xtask |44452|44466|+14|44466|
| aggregate |193187|193202|+15|193526|

Aggregate hardfail193203 = measured +1. Kernel26841 physical/21042 tokei,
95 baseline hashes unchanged. xtask is exactly at its ceiling.

## Cleanup

After final gates, a /proc executable scan checked27 live executable processes;
no cargo/rustc/nextest/clippy process and no unreadable executable remained.
Only28 stale `debug/incremental/*/s-*` directories were removed, retaining
newest5929 sessions and625 additional recent sessions. No deps/build/fingerprint
prune, cargo clean, whole-target cleanup or target override occurred.

| Storage | Before bytes | After bytes | Freed bytes |
|---|---:|---:|---:|
| `/mnt/build/cargo-maos` |222600966144|221768646656|832319488|
| `/tmp` |1453056000|1453056000|0|

No owned scratch remained except the protected `/tmp/204l1-wip` archive trio,
which is retained. No stray .orig/.rej or repo-root scratch logs were created.
Host load at13:34:15:89.62/97.98/100.20.

## Handoff

`commands.md` and `commands.json` retain all92 exact commands and exits,
including process guards, red controls and failed attempts; raw outputs are in
`logs/`, clippy JSONL and named gate logs. Setup resolver exit0 is also recorded.
Story Dev Agent Record has the dedicated hosted-1 section and cleanup results.

Open: the older no-default all-target test-import failure described above;
aarch64-apple-darwin cross-target and hosted rerun not exercised. Full workspace
nextest/prepush, paid live vendor legs and cargo deny were not rerun in this
hotfix scope. The parent expressly did not require full prepush.
