# 20-11 L2 review corrections — commands and observed exits (Dev2011k, 2026-11-11, base `c402c29b`)
All cargo runs `env -u CI -u GITHUB_ACTIONS`, repo `target` symlink, one cargo process at a time, `~/.venvs/maos-test/bin` on PATH.
| # | Command | rc | Observed |
|---|---|---:|---|
| 1 | T0: `git archive HEAD` copy + P1/F3 prototype + rustfmt; `check-kernel-baseline`, `kloc-check`, `check-service-boundary` | 0 / 1 / 1 | kernel +7 physical / +6 tokei (26,887/21,075); gate flags only `ScheduleWatchdog` (private `fence`) |
| 2 | failing-before at HEAD: kernel P1, F3; bin p2/f2/f1; audit verifier; wasm lift; domain caps | 101 each | `logs/red-*.log` (F4 tests killed after >60 s each at HEAD) |
| 3 | plants (10), each restored `cmp` rc 0 | 101 each | `logs/plant-*.log` |
| 4 | final witnesses f1…f11 (kernel O4/KS/P1/F3, kernel `--lib` 281, domain, audit 11, wasm 22, bin inventory 24, door 2, composition 8, live 2, xtask pin tests 9+25+12, no-default verb table) | 0 each | `final/f*.log`, `final/exits.txt` |
| 5 | `cargo clippy --locked --workspace --all-targets --message-format=json`; `clippy-ratchet` | 0, 0 | 1328 / 868, equal to the L2 figure |
| 6 | `check-fkcs --json` own cell | 0 | `passed: true`, no blocking legs |
| 7 | `build-test-fixtures`; genuine `cargo build -p maos-bin -p maos-cli`; ONE `cargo nextest run --workspace --profile ci --no-fail-fast` | 0, 0, 100 | 5997 run, **5995 passed**, 2 failed: the known local AC5 1M row (43.3 s, not retried or widened) and the 20-7 snapshot-owner timing family (`t3_terminal_helpers_17_7`, deferred row with same-line evidence) |
| 8 | gates (see `final/gates/`) | 0 | recorded after the final record edits |
