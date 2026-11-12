# Toolchain pin to rustc 1.99.1 — evidence (2026-11-12)

Change: `rust-toolchain.toml` only — `channel = "stable"` → `channel = "1.99.1"`, components `rustfmt`+`clippy`
and profile `minimal` kept, `targets = [wasm32-wasip2, aarch64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu]`
added, pin rationale in a TOML comment. No workflow line was changed (none is required, see below).

Root cause (from hosted logs of run 43255653207): stable moved 1.99.1 → 1.100.0 on 2026-11-12; cargo 1.100 puts test
executables under `target/<profile>/build/<pkg>/<hash>/out/`, while `tests/harness/fixtures.rs` and ~60 tests assume
`<target>/<profile>/deps/<test>-<hash>` via `std::env::current_exe()`. Post-tag fix + unpin: see "Follow-up" at the bottom.

## Target inventory (non-nightly toolchain)

| Source | Target | Host that compiles it |
|---|---|---|
| discipline.yml `rustup target add` (l.1460, example-spirit-ts-tests), `targets:` l.2127 / l.6275, xtask `build_test_fixtures.rs:86` | wasm32-wasip2 | x86_64 + aarch64 Linux |
| discipline.yml `macos-check` l.6190–6208 (`--target aarch64-apple-darwin`), release.yml matrix | aarch64-apple-darwin | macos-latest (arm64; native host) |
| release.yml matrix (ubuntu-latest, x86_64 host), discipline `release-dry-run` matrix (ubuntu-24.04-arm: host) | aarch64-unknown-linux-gnu | cross on x86_64 (release.yml), native on arm |
| discipline `t3-escape-corpus` l.2017, `ci-test-producer` l.2184 (x64 only), xtask `build_test_fixtures.rs:300-308` (`cfg!(target_arch="x86_64")`), release matrix | x86_64-unknown-linux-gnu | x86_64 host (native; listed for explicitness) |
| release.yml l.255 `--manifest-only --targets …,aarch64-apple-darwin` | — | no compilation |

Fuzz jobs (`RUSTUP_TOOLCHAIN: nightly-2026-09-25`, `cargo +nightly fuzz`) use their own toolchain.
`aarch64-apple-darwin` and `x86_64-unknown-linux-gnu` are redundant where they are the host (rustup counts the host
implicitly) and unused as a cross target on Linux; they are listed because the assignment asked for them. Cost: one extra
rust-std download per job — drop them if install time matters.

## Job-class compatibility

| # | Job class | Result | Evidence |
|---|---|---|---|
| 1 | Every job that runs `cargo` in the repo (dtolnay installs `stable`, file overrides) | OK — rustup selects 1.99.1 from the file and **auto-installs** it with components + targets on the first `cargo`/`rustc` proxy call (Swatinem/rust-cache's `rustc -vV` or the first `cargo`) | Local: rustup 1.29.1 on a clean-of-1.99.1 machine: `rustup show active-toolchain` → `info: downloading 8 components` (cargo, clippy, rustc, rustfmt + 4× rust-std) → `1.99.1-x86_64-unknown-linux-gnu (overridden by …/rust-toolchain.toml)`; `rustup target list --installed --toolchain 1.99.1` = the 4 targets; `rustup component list --installed` = cargo, clippy, rustc, rustfmt. Hosted: run 43255653207 logs (job 131527080654, ubuntu-24.04 image 20261111.370.1; 131527098880 ubuntu-24.04-arm; macOS job 131506331120 of run 43310873144 macos-26-arm64) show `info: downloading 5 components` (the concurrent-download output introduced with **rustup 1.29.0**) and `info: note that the toolchain 'stable-…' is currently in use (overridden by '…/rust-toolchain.toml')` — i.e. runner rustup ≥ 1.29 already honours the file today. Rustup changelog: auto-install was removed in 1.28.0, restored by default in **1.28.1** (`RUSTUP_AUTO_INSTALL`, default 1); 1.30 (issue rust-lang/rustup#4836) removes auto-install only for `rustup` subcommands (show/target/component/…), NOT for the `cargo`/`rustc` proxies. The only `rustup` subcommand touching the active toolchain, discipline.yml l.1460 `rustup target add wasm32-wasip2`, runs after earlier cargo steps in that job and is redundant with the file's `targets`. No workflow change needed. |
| 2 | `toolchain: nightly` / `rustup toolchain install nightly…` / `cargo +nightly` / `RUSTUP_TOOLCHAIN: nightly-2026-09-25` (fuzz*, miri/nightly-install steps, abi-diff via `rustup run nightly`) | OK — `+toolchain` and `RUSTUP_TOOLCHAIN` rank above the toolchain file (rustup book, Overrides §1–2, §4). Explicit `rustup toolchain install <name>` is unaffected | Local in the repo: `cargo +nightly --version` → `cargo 1.100.0-nightly (98a09e7e7 2026-09-21)`; `RUSTUP_TOOLCHAIN=nightly rustc --version` → `rustc 1.100.0-nightly (f7575a9da 2026-09-24)` (nightly-2026-09-25 itself is not installed locally; `nightly` = 2026-09-24 was used) while `rustc --version` → 1.99.1. |
| 3 | release.yml build matrix (x86_64/aarch64-unknown-linux-gnu, aarch64-apple-darwin on macos-latest) and `macos-check` | OK — 1.99.1 + needed rust-std come from the file (`targets`), the dtolnay `targets: ${{ matrix.target }}` install only feeds the unused `stable` | Local: libcore present for all 4 targets under `rustc --print target-libdir --target <t>` on 1.99.1. Hosted macOS (arm64, rustup ≥1.29) honours the file (above). Not run locally: actual macOS/aarch64 cross builds. |
| 4 | discipline.yml l.130–140 `grep -rn --exclude-dir=.github 'cargo +nightly' … --include='*.toml'` | Unaffected — the new `rust-toolchain.toml` does not contain `cargo +nightly` (verified by running the same grep over the tree: no match) |
| 5 | xtask tests/gates that read `rust-toolchain.toml` or assert `toolchain: stable` | OK — `rust-toolchain.toml` is only referenced as a path in `macos_compile_guard_20_6.rs:729` (path-trigger list, no content read); `ci_shape_18_0` / `kernel_pin_ci_wiring_20_3a` / `macos_compile_guard_20_6` only match `dtolnay/rust-toolchain@` step shape, `components`, and never the `toolchain:` value; workflows stay `toolchain: stable` | Local: those 5 test binaries 73/73 pass (below). `release_dry_run.rs::host_target` runs `rustc -vV` through the proxy → host triple unchanged. |

Not covered by the pin (by design): `cargo install …` / commands run outside the repo directory still use `stable`
(rustup warns "default toolchain implicitly overridden"); nested test processes inherit `RUSTUP_TOOLCHAIN` from the
proxy and stay on 1.99.1.

Stale prose (not edited, out of scope): `docs/dev-discipline/abi-diff-migration.md:20,108` says the root
`rust-toolchain.toml` "remains stable"; update when unpinning.

## Local proof (shared target dir `/mnt/build/cargo-maos` was empty → full rebuilds; exit codes observed)

| Command | Result |
|---|---|
| `rustup show active-toolchain` (repo) | `1.99.1-x86_64-unknown-linux-gnu (overridden by '/home/lunarpulse/dev_ws/rust-toolchain.toml')`, auto-installed (exit 0) |
| `rustc --version` / `cargo --version` | rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1 (5f94df478 2026-08-27) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --locked -p xtask` | exit 0 (1m09s) |
| `env -u CI -u GITHUB_ACTIONS cargo run --locked -p xtask -- build-test-fixtures --only maosctl,maos-registry-server,maos-spirit` | exit 0; `3 artefact(s) → /mnt/build/cargo-maos/maos-fixtures/manifest.json` |
| `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-spirit-cli` | exit 0; 20 tests run, 20 passed |
| air-gap job commands: `cargo test -p maos-registry --test import_air_gap_test` | exit 0; 9 passed; binary at `/mnt/build/cargo-maos/debug/deps/import_air_gap_test-…` (the `deps/` layout the harness expects) |
| air-gap job commands: `cargo test -p maos-spirit-cli --features fixture_replay` | exit 0; all suites ok |
| `cargo nextest run -p xtask --locked --test ci_shape_18_0 --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` (+ `--test macos_compile_guard_20_6 --test kernel_pin_ci_wiring_20_3a`) | exit 0; 73 tests run, 73 passed |
| `/mnt/build/cargo-maos/debug/xtask check-rc-readiness --tag v1.0.0-rc.1 --json` | exit 0; `"passed":true`, all 6 legs pass (provisioning leg WARN = existing claim boundary) |
| `… check-env-contract` | exit 0; PASS (124 vars, 0 violations) |
| `… kloc-check --json` | exit 0; `"passed":true`, aggregate 203444 |
| `cargo +nightly --version` in repo | `cargo 1.100.0-nightly (98a09e7e7 2026-09-21)` |

Not run locally: the other ~20 failing CI jobs (workspace-test-suite, cgroup-ceiling-*, epic-17-exit, loom, …) — they
share the same `current_exe()` root cause; the evidence above shows the toolchain they will now run under restores the
`deps/` layout.

## Follow-up (post-tag story): layout-agnostic harness, then unpin

Make tests independent of where cargo places test executables, then revert `rust-toolchain.toml` to
`channel = "stable"` (and drop the `targets` list if the workflows install them):

1. `tests/harness/fixtures.rs`: stop deriving `target_root()`/`profile_dir()` from `std::env::current_exe()`'s
   `deps/` parent. Resolve the fixture manifest from an explicit root published by the fixture builder — e.g. `xtask
   build-test-fixtures` exports/records `MAOS_FIXTURE_DIR` (or writes the absolute manifest path into a file next to
   `CARGO_TARGET_DIR`), with `CARGO_TARGET_DIR`/`cargo metadata --format-version 1 .target_directory` as the fallback and
   `CARGO_MANIFEST_DIR` for the workspace root.
2. Sibling/product binaries: use `env!("CARGO_BIN_EXE_<name>")` (same-package bins) or `cargo build --message-format=json`
   artifact paths / `assert_cmd::cargo::cargo_bin` instead of `current_exe().parent().parent().join(<bin>)`.
3. Replace the ~60 ad-hoc `current_exe()` path derivations (`grep -rn "current_exe()" tests crates/*/tests xtask/tests`)
   with one shared helper, and add an xtask lint/test that rejects new `current_exe()`-relative `deps/` assumptions.
4. Verify on both 1.99.1 and 1.100.x (CI matrix or a temporary `toolchain: 1.100.0` job), then unpin and update
   `docs/dev-discipline/abi-diff-migration.md`.
