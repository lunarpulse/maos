# 19-6 creation — measurements owed, results (2026-10-28)

Run by the orchestrator (`anthropic/claude-opus-5-5`), one cargo process, HEAD `9ce0e47a` (crate source identical to `d9bd59c4`), `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Gates run with the xtask binary built at `d9bd59c4` (`/tmp/xtask-maos-d9bd59c4`, `CARGO_MANIFEST_DIR` set as `cargo run` would). Each prototype: `git apply <diff>` → `cargo check -q --locked` → `check-kernel-baseline --json` → `kloc-check --json` → `git checkout -- <files>` → `git diff --quiet -- crates` (clean after every revert). Script: throwaway `/tmp/measure-wave2.sh`; raw outputs `/tmp/measure-wave2/`.

| Step | Result | Matches creation figure |
|---|---|---|
| 1 baseline | `check-kernel-baseline` PASS (26378, 95 files) | yes |
| 2 baseline | `kloc-check` exit 0, aggregate 189877, maos-bin 30380 | yes |
| 3 K1 (`k1-kernel-delivery-prototype.diff`) | `cargo check -p maos-kernel-core` rc 0; baseline actual **26405**; kloc kernel **20643**, aggregate 189901; revert clean | yes (+27 / +24) |
| 4 K2 (`k2-replica-pid-prototype.diff`) | `cargo check` rc 0; baseline FAIL with actual 26378 = pinned (content/set hash moved); kloc unchanged | yes (±0, pinned-byte change) |
| 5 K3 (`k3-trait-reshape-prototype.diff`) | `cargo check -p maos-kernel-core -p maos-domain` rc 0; baseline actual **26377**; kloc kernel **20618**, maos-domain **9986** (over its 9975 row), aggregate 189887 | yes (−1 / −1, domain +11) |
| 6 `epic_dispatch_19_1` per-binary time | hosted run 40671240805 (T3, ubuntu-24.04): libtest `finished in 9.05s` for 8 tests (7 passed, 1 failed on the since-fixed halt-row assertion) — under the 30 s budget | — |
| 7 `nfr-rel-1-crash-detection-2s` wall | not measured here (no hosted log fetched yet); owed at dev time from the next discipline run | owed |

All recommended rows (D-19-6-01…11 A, ruled 2026-10-28) are kernel-Δ 0; none of K1–K3 is taken.
