# 19-3 creation — measurements owed, results (2026-10-28)

Run by the orchestrator (`anthropic/claude-opus-5-5`), one cargo process, HEAD `9ce0e47a` (crate source identical to `d9bd59c4`), `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Gates run with the xtask binary built at `d9bd59c4` (`/tmp/xtask-maos-d9bd59c4`, `CARGO_MANIFEST_DIR` set as `cargo run` would; the shared target dir's `debug/xtask` was being overwritten by another workspace). Each prototype: `git apply --exclude='*.orig' <diff>` → `cargo check -q --locked` → `check-kernel-baseline --json` → `kloc-check --json` → `git checkout -- <files>` → `git diff --quiet -- crates` (clean after every revert). Script: throwaway `/tmp/measure-wave2.sh`; raw outputs `/tmp/measure-wave2/`.

| Step | Result | Matches creation figure |
|---|---|---|
| 1 baseline | `check-kernel-baseline` PASS (26378, 95 files); `kloc-check` exit 0, aggregate 189877, kernel 20619, maos-bin 30380 | yes |
| 2 K2 `rw,U` (`k2-prototype-d9bd59c4.diff`) | `cargo check` kernel-core + maos-bin rc 0; baseline FAIL actual 26389 vs pinned 26378; kloc kernel **20628**, maos-bin **30382**, aggregate **189888** | yes (+11 / +9) |
| 3 keep-id variant (`k2-keepid-variant-d9bd59c4.diff`) | `cargo check` rc 0; baseline actual **26390**; kloc kernel **20629** (1 over the 20628 endpoint), maos-bin 30382, aggregate 189889 | yes (+12 / +10) |
| 4 surface pin (with step 2 applied) | `check-service-boundary` exit 1, exactly 3 hash changes reported against `docs/ci-baselines/kernel-surface-v0.1-beta.json`: `lifecycle::cli_wrapper::runtime::T3Launch`, `security::sandbox::t3::argv::build_runtime_argv`, `security::sandbox::t3::spawn::T3SpawnContext`; no other symbol | yes |
| 5 D-07 A (`d1044-a-liveness-removal-sketch.diff`) | `cargo check -p maos-bin` rc 0; kloc maos-bin **30325** (−55), aggregate 189822, kernel unchanged | yes |
| 6 D-07 C prototype | not run (D-19-3-07 ruled A) | — |
| after | baseline PASS 26378; kloc aggregate 189877; `git status --short -- crates` empty | — |

Owed to hosted CI at dev time: one run confirming the `:U` mount works on the rootless CI images.
