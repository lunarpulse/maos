# 20-11 L2 correction round 3 — commands and exits (Dev2011k, 2026-11-11, base `5ff0ead1`)
| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | failing-before at HEAD (domain/kernel oversized provenance, I2 source scan, approval history, N1 hook fence) | 101 each | `logs-r3/red-*.log` |
| 2 | plants (5) restored `cmp` rc 0 | 101 each | `logs-r3/plant-r3-*.log` (one earlier plant variant self-deadlocked the fence and was killed; its file was restored, then replaced by a non-deadlocking plant) |
| 3 | final witnesses f1…f11 (kernel O4/KS/P1/F3/N1/oversized, kernel `--lib`, domain, audit verifiers + I2 scan, wasm, bin inventory 24, door, composition, live, xtask pins, no-default verb table) | 0 each | `final-r3/f*.log` |
| 4 | clippy + ratchet; `check-fkcs` | 0, 0, 0 | 1328 / 868 unchanged; `passed: true` |
| 5 | fixtures; genuine bin/CLI build; ONE `cargo nextest run --workspace --profile ci --no-fail-fast` | 0, 0, 100 | 6001 run, **6000 passed**, 1 failed: the known local `erasure_million_20_11` AC5 1M row (49.7 s), not retried or widened — `final-r3/workspace-nextest.log` |
| 6 | gates | 0 | `final-r3/gates/`; clean-overlay `kloc-check` aggregate 202,505, `over_budget []` |
