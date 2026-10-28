# 19-3 creation measurements (2026-10-28, HEAD `d9bd59c4`)

**No cargo was run** (another cargo process held the host; parent's hard rule). Every figure below is either a direct read, a `git` query, or the kloc-check instrument replicated without cargo: `tokei 14.0.0` with the argv of `xtask/src/kloc_check.rs:294-318` and the crate grouping of `:386-417`. Prototypes were applied to **scratch copies under `/tmp`** built from `git show d9bd59c4:<path>`; no file under `crates/`, `xtask/` or `spirits/` was touched, even temporarily. Tree before and after: `git status --short` shows only siblings' untracked dirs (`19-6-…-evidence/`, `planning-artifacts/research/agent-orchestration-2026-10-28/`) plus this story's own files. No test was run, so `intent-lineage-coverage-report.md` was not touched.

## Baseline at HEAD (replicated, no cargo)

| Figure | Value | Pin |
|---|---|---|
| kernel physical lines, git-tracked `crates/maos-kernel-core/src/**.rs` (`xargs cat \| wc -l`) | **26378** | `xtask/kernel-core-baseline.toml:505` = 26378 |
| kernel tokei | **20619** | `xtask/kloc.toml:264` = 20619 |
| maos-bin | **30380** | `:517` = 30380 (exact) |
| xtask | **47451** | `:339` = 47451 (exact) |
| maos-bench | **2451** | `:520` = 2451 (exact) |
| maos-control · maos-cli · maos-shell · maos-a2a-core | 1442 · 6317 · 613 · 5037 | `:484` 1791 · `:489` 6856 · `:691` 613 · `:470` |
| aggregate | **189877** | `_aggregate_hardfail` `:718` = 189878 |

Per-file tokei of the files this story edits: `worker_spawn.rs` 1044, `worker_cli.rs` 526, `main.rs` 12773, `xtask/src/demo_j1.rs` 1186, `xtask/src/main.rs` 1236, `maos-bench/src/harness/j1.rs` 232, kernel `runtime.rs` 1176, `t3/argv.rs` 81, `t3/spawn.rs` 484 (`tokei-d9bd59c4.json`).

## Prototype 1 — K2 T3 workspace mount, `rw,U` (D19-2 A shape; D-19-3-01 A), scratch

`k2-prototype-d9bd59c4.diff` = 19-1's `creation/k2-prototype.diff` (f34f6b4e) re-applied with `patch -p1` to HEAD copies: kernel hunks apply at their original lines (no kernel file changed since f34f6b4e: `git diff --stat f34f6b4e..d9bd59c4` lists no `maos-kernel-core` path); maos-bin hunks apply with offsets (`main.rs` +193 → T3 smoke literal `:7822-7836`, `worker_spawn.rs` +22 → `T3Launch` literal `:1106-1114`).

| Figure | Base | Prototype | Δ |
|---|---:|---:|---:|
| kernel physical (3 files) | 2189 | 2200 | **+11** |
| kernel tokei (3 files) | 1741 | 1750 | **+9** |
| maos-bin tokei (2 literal sites) | 13817 | 13819 | +2 |

Implied endpoints: physical 26378 + 11 = **26389** (≤ 26390, D-19-1-01 A); tokei 20619 + 9 = **20628** (= 20628 — zero tokei slack); ADR-069 alarm 22,000 not crossed.

## Prototype 2 — keep-id variant (D-19-3-01 B), scratch

`k2-keepid-variant-d9bd59c4.diff`: as 1, but `--volume=<dir>:/workspace:rw` plus `--userns=keep-id:uid=65534,gid=65534`. Kernel **+12 physical / +10 tokei** → tokei endpoint 20629 > 20628 (breaches the D19-2 A tokei endpoint by 1; needs a grant extension).

## Prototype 3 — `deferred-work.md:1044` option A (host liveness probe removed), scratch

`d1044-a-liveness-removal-sketch.diff`: `run_liveness_probe` (`worker_cli.rs:259-306`) deleted, `ProbeStrategy::Liveness{argv}` → `SpawnIsProbe`, the `Liveness` arm of `worker_spawn.rs:833-858` keeps `check_egress` + the T3 floor only. maos-bin tokei over the two files 1570 → 1515 = **−55**. Not compiled (no cargo); tests `worker_completion_2a.rs:150-192` would be rewritten/deleted with it.

## Measurements owed (the orchestrator runs these; one cargo process)

Prefix every command with `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`; gates run as `cargo run -q --locked -p xtask -- <gate>`. Apply a diff with `git apply <file>` (paths are `a/…`/`b/…`; if the header form refuses, `patch -p1 < <file>`), revert with `git checkout -- <the five files>`.

1. **Baseline confirm.** `check-kernel-baseline` → expect `PASSED (… 26378 lines, 95 files, pinned 26378)`; `kloc-check --json` → expect exit 0, aggregate **189877**, kernel 20619, maos-bin 30380, xtask 47451, maos-bench 2451, `over_budget` []. Why: the replication above used tokei directly.
2. **K2 `rw,U` (D-19-3-01 A).** Apply `k2-prototype-d9bd59c4.diff`; `cargo check --locked -p maos-kernel-core --lib` and `cargo check --locked -p maos-bin` → expect exit 0; `check-kernel-baseline` → expect exit 1 naming **26389** (+11) and the three changed files `lifecycle/cli_wrapper/runtime.rs`, `security/sandbox/t3/argv.rs`, `security/sandbox/t3/spawn.rs`; `kloc-check --json` → expect exit 1 (exact rows), kernel **20628**, maos-bin **30382**, aggregate **189888**. Revert; re-run 1 → PASSED 26378 / exit 0 189877. Why: confirms the kernel grant basis at d9bd59c4 (D19-2 A re-measure owed by the epic, `epic-19:50` (a)).
3. **keep-id variant (D-19-3-01 B).** Apply `k2-keepid-variant-d9bd59c4.diff`; same checks → expect **26390 / 20629**, maos-bin 30382. Revert; re-run 1. Why: prices option B (tokei endpoint breach by 1).
4. **Surface pin delta for 2.** With 2 applied: `cargo run -q --locked -p xtask -- check-service-boundary` → expect a struct-hash / signature change for `T3Launch`, `T3SpawnContext`, `build_runtime_argv` (class rows exist, `xtask/kernel-api-classes.toml:673,686,688`), no unclassified symbol. Revert. Why: FLAG-Winston obligation (b) (`kernel-core-baseline.toml:1000-1008`).
5. **`deferred-work.md:1044` option A (D-19-3-07 A).** Apply `d1044-a-liveness-removal-sketch.diff`; `cargo check --locked -p maos-bin`; `kloc-check --json` → expect maos-bin ≈ **30325** (−55 ± 3 after `cargo fmt`). Revert. Why: prices the recommended row (expected compile errors only in `crates/maos-bin/tests/worker_completion_2a.rs:150-192`, which `cargo check` without `--tests` does not build).
6. **Option C of D-19-3-07 (kernel probe into T3) — only if the operator considers C.** Prototype: `admission.rs` `probe_and_verify_shape` (`:48-170`) launches through `security::sandbox::t3::spawn::spawn_t3` instead of `Command::new` (`:74-94`), carrying the image/state dir it needs from `admit_cli_wrapper_journaled` (`:220`). Expected [INFERENCE] kernel **+15–30 tokei**, which exceeds the D19-2 A grant → a separate FLAG-Winston row. Measure with `check-kernel-baseline` + `kloc-check --json`; revert.

Every step leaves no crate source changed; revert `_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` if any run modifies it.
