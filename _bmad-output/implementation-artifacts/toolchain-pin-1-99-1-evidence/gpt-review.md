## Verdict: SHIP the pin

**Findings: none. P0: 0 · P1: 0 · P2: 0.** No proven patch-introduced blocker.

### Workflow compatibility

- **No dedicated latest-stable or MSRV job exists.** `Cargo.toml:72` declares Rust 1.88, but workflows do not select/test `1.88`, `msrv`, or `rust-version`. Selecting 1.99.1 for ordinary repo-local commands is the intended change.
- `dtolnay/rust-toolchain` sets the default; the file overrides it. Explicit `cargo +nightly`, `rustup run nightly`, and `RUSTUP_TOOLCHAIN` overrides remain effective.
- **WASM/componentize:** `wasm32-wasip2` covers Rust fixture builds. TypeScript componentization uses npm/componentize-js, not a source-installed Cargo componentizer.
- **Release/macOS:** all three release targets are covered. `release.yml:67–76` retains the ARM Linux cross-compiler installation and linker/CC variables. Discipline’s ARM dry-run is native; macOS checks/releases use `aarch64-apple-darwin`. The pin does not alter linker selection.
- `cargo-deny@0.19.4` and `cargo-nextest@0.9.149` use prebuilt install-action binaries. No cargo-audit installation found.

### Source-installed tools

**16 installation commands, five distinct tools:**

| Tool | Installation sites | Compatibility with 1.99.1 |
|---|---|---|
| tokei 14.0.0 | discipline:286,6340 | [Declared MSRV 1.71](https://crates.io/api/v1/crates/tokei/14.0.0) |
| wasm-tools 1.259.0 | discipline:2132 | [Declared MSRV 1.88](https://crates.io/api/v1/crates/wasm-tools/1.259.0) |
| cargo-public-api 0.51.0 | discipline:307,753,4311,5783,6304 | No declared MSRV; **successfully source-built on 1.99.1** in [job 131270517190](https://github.com/lunarpulse/maos/actions/runs/43243673212/job/131270517190) |
| cargo-fuzz, currently 0.13.2 | discipline:4964; cadence:39; prerelease:38,101,164,227,290 | No declared MSRV; **successfully source-built on 1.99.1** in [job 131270542768](https://github.com/lunarpulse/maos/actions/runs/43243673212/job/131270542768) |
| maos-bin, local path | discipline:6018 | Workspace-declared MSRV 1.88 |

**Evidence correction:** README:36–38 incorrectly excludes repo-local `cargo install` from the pin. Those invocations **do use 1.99.1**; the compatibility inventory above accounts for them.

### Gates and recorded toolchains

- No xtask gate/test reads the toolchain file’s contents or asserts `stable`. Existing assertions check action/components/targets shape.
- `macos_compile_guard_20_6.rs:729` treats the file as a platform-sensitive trigger, correctly scheduling macOS coverage.
- `reproducible-build` compares two builds’ hashes; it does not record a compiler version.
- Release signing covers artifact hashes, with **no toolchain field or signed compiler attestation**.
- WASM fixture provenance records historical **1.98.1**; its validator checks nonempty metadata and source/binary hashes, not equality with the current compiler.
- Performance fingerprints include actual `rustc -V`. The potential recalibration issue predates this patch; the `perf-ledger` branch currently returns HTTP 404, so no existing calibration regression is established.

### Hosted version confirmation

Fetched [check-fmt job 131270517710](https://github.com/lunarpulse/maos/actions/runs/43243673212/job/131270517710):

> `2026-11-12T15:10:02.4477859Z rustc 1.99.1 (58cf8f9c3 2026-10-13)`

Its rust-cache output independently reports that exact compiler. **1.99.1 matches the successful jobs.** However, run **43243673212 was not globally green**: its overall conclusion and aggregate were failure.

**overall_correctness:** correct  
**explanation:** Ship this toolchain change; the reviewed workflows, tools, targets, and gates show no introduced blocker. This is not release authorization: the pin still needs its hosted aggregate result.  
**confidence:** 0.95

No builds or tests were rerun; repository contents were unchanged.