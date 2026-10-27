# T1 hosted spike record — `maos-wasm-host` archive roundtrip (run 40489534163)

One-shot GitHub Actions spike of the T1 design on branch `ci/20-15-spike` (branch deleted
after evidence download; nothing merged). All figures below are read from the checked-in
evidence under `ci/spike-40489534163/`; nothing is inferred beyond it.

## Run identity

| Field | Value | Source |
|---|---|---|
| Run ID / attempt | `40489534163`, attempt 1 | `run_id`, `run_attempt` in all six `producer.json`/`consumer.json` |
| Commit | `19143e607f318e38bfe8c7604c1194ce7e4b0aba` (tree `8bbdcd0052515469c12cf545b73c351239421f3a`) | `sha`, `tree` in all six records |
| Workflow | `ci-20-15-spike` (push-triggered on `ci/20-15-spike` only; inert copy at `ci/ci-20-15-spike.yml`) | workflow file |
| Jobs | 2 producers (`producer (x64, ubuntu-24.04)`, `producer (arm64, ubuntu-24.04-arm)`) + 4 consumers + report gate | workflow matrix; 6 evidence dirs |
| Job IDs | **Not captured in downloaded evidence** — only artifact IDs were recorded (below) | — |
| When | 2026-10-27, consumer junit timestamps 10:36:30–10:36:33 UTC | `junit.xml` |

## Producers (2)

Toolchain: rustc 1.99.1 (`58cf8f9c3`, 2026-10-13), cargo-nextest 0.9.149 (`3fe4ef319`, 2026-10-20)
— `rustc-version.txt`, `nextest-version.txt`. Command (workflow step):
`cargo nextest archive --locked -p maos-wasm-host --test e2e_roundtrip --archive-file …/wasm-host.tar.zst`,
cold with no rust-cache. Workspace `/home/runner/work/maos/maos`, target `/home/runner/work/maos/maos/target`.

| Metric | x64 (`ubuntu-24.04`, x86_64) | arm64 (`ubuntu-24.04-arm`, aarch64) |
|---|---|---|
| build_and_pack_seconds (measured, incl. archive packing) | 84.86 s | 88.68 s |
| build.log corroboration | `Finished test profile … in 1m 20s`; `Archived 47 files … in 0.38s` | `Finished … in 1m 25s`; `Archived 38 files … in 0.32s` |
| archive_bytes (`wasm-host.tar.zst`) | 37,877,332 B (36.12 MiB) | 36,858,212 B (35.15 MiB) |
| upload_step_wall_seconds (bracketed Actions step) | 2.008 s | 1.856 s |
| bundle artifact ID | 12728615518 | 12727685828 |

Producer-recorded sha256 (`files` in producer.json):

- x64: `target/debug/deps/e2e_roundtrip-95a4359ca25f15c1` `de910e6b…3b686`;
  `target/debug/maos-wasm-runner` `81677701…2282c`;
  `tests/fixtures/wasm/echo_spirit_component.wasm` `bb919df9…c86681`
- arm64: `target/debug/deps/e2e_roundtrip-9acdf414beb53c63` `d3141b15…47959`;
  `target/debug/maos-wasm-runner` `4da2f766…3c57c`; fixture `bb919df9…c86681` (identical to x64)

Transport sha256 (`transport_files`): x64 `wasm-host.tar.zst`
`462f4411494d0d165a6709b00925a5df67b95d8fb24adf1be5ae11417006e8a2`, `nextest.tar`
`5d3afa7cbea60a00bccd5828316b6b3e055eb5830aba9faceac3596cd068a5f9`; arm64 `wasm-host.tar.zst`
`2be5dbb9a49a04b186c7ff9e31e75e6dd01b2ee365d32110b8765b87f4958c51`, `nextest.tar`
`ba6320e8e10f463e4da02bf1265888bd65bd02ea6b8d457e787d87d2281ef5f9`.
Consumers re-hashed both transport files against these before extraction (workflow assertion).

`ldd` on `target/debug/maos-wasm-runner` (producer-ldd.txt, consumer-ldd.txt, all four
consumers): only `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `/lib64/ld-linux-…so.2` — no
runner-toolchain or image-specific library dependencies.

## Consumers (4)

All four ran with compiler guards `RUSTC=/bin/false`, `CARGO_BUILD_RUSTC=/bin/false`,
`CARGO=/bin/false` (workflow job env, echoed into `compiler_guards` in each consumer.json),
no Rust toolchain installed, no cache, no fallback. Extraction used
`tar --zstd -xvf wasm-host.tar.zst -C "$GITHUB_WORKSPACE"`, then
`cargo-nextest nextest run --archive-file … --extract-to "$GITHUB_WORKSPACE"
--workspace-remap "$GITHUB_WORKSPACE" --profile ci -E "binary_id(maos-wasm-host::e2e_roundtrip)
& test(=real_runner_subprocess_adr032_roundtrip_through_guest)"`.

| Consumer image (machine) | download wall s | tar extract s | nextest extract+run s | junit test s | nextest run UUID | cross-image |
|---|---|---|---|---|---|---|
| ubuntu-24.04 (x86_64) | 2.5015 | 0.5360 | 1.6697 | 1.324 | `4d638d1e-b17a-41b2-a01d-01f2f09b9182` | no |
| ubuntu-24.04-arm (aarch64) | 2.3415 | 0.2897 | 2.2295 | 1.955 | `e28f3183-e04f-45d4-90ff-efdda2802fcb` | no |
| ubuntu-26.04 (x86_64) | 1.3405 | 0.6736 | 2.9853 | 2.525 | `9068b7f8-afad-4ca0-b28f-7780f68997dd` | **yes** |
| ubuntu-26.04-arm (aarch64) | 1.2718 | 0.3341 | 2.1772 | 1.881 | `e3a4d7c3-be02-455e-8f62-5ca37d3141cc` | **yes** |

Exact passing test (every `junit.xml` and `test.log`):
`maos-wasm-host::e2e_roundtrip::real_runner_subprocess_adr032_roundtrip_through_guest` —
`tests="1" failures="0" errors="0" skipped="0"`; test.log summary
`1 test run: 1 passed, 7 skipped` (the 7 other tests in the binary were filtered out by the
expression, not skipped at runtime). During the nextest archive run, nextest re-extracted
47 files (x64 images) / 38 files (arm images) into the workspace (test.log line 2).

## Identity, path, and hash verification (what actually makes this a proof)

- Same run/attempt/sha/tree in all six records; report job asserted `workspace` is a single
  distinct value across all six jobs: `/home/runner/work/maos/maos` — the absolute producer
  path was reproduced bit-identically on every consumer image and arch (`workspace ==
  producer_workspace` per consumer.json).
- `identical_producer_files` in each consumer.json is byte-for-byte the producer's `files`
  map for that arch (x64 consumers ↔ x64 producer; arm64 consumers ↔ arm64 producer;
  fixture hash identical on both arches) — consumer-side re-hash after extraction matched
  the producer-side hash of the test binary, `maos-wasm-runner`, and the
  `echo_spirit_component.wasm` fixture.
- The test binary embeds compile-time absolute source paths
  (`CARGO_MANIFEST_DIR`-derived) that pointed at the producer workspace; running green on
  all four images with `RUSTC`/`CARGO`/`CARGO_BUILD_RUSTC` = `/bin/false` proves no rebuild
  occurred anywhere (any cargo/rustc invocation would fail instantly), and the workflow
  gate also asserted no `Compiling `/`Finished `/`SKIP` lines in extract/test logs.

## Timing semantics (do not misread)

- `upload_step_wall_seconds` / `download_step_wall_seconds` bracket whole Actions steps
  (include inter-step overhead), not pure network transfer.
- `build_and_pack_seconds` includes nextest archive packing; `extract_seconds` is the
  standalone `tar --zstd` unpack; `nextest_extract_and_run_seconds` includes BOTH the
  nextest archive re-extraction and test execution — it is not pure test time. Pure test
  time is the junit `time` column above.
- Producer builds were cold (deliberately no rust-cache); these are not incremental times.

## Narrow scope (explicit non-claims)

This spike proves only the `maos-wasm-host` T1 family: a debug/test-profile
`cargo nextest archive` of `--test e2e_roundtrip` produced on ubuntu-24.04/amd64 and
ubuntu-24.04-arm, transported via `upload-artifact`/`download-artifact` within one run, and
executed (one selected test, real Wasmtime subprocess ADR-032/CBOR roundtrip) on four
images with compilers disabled. It does **not** verify: any other D01-C producer family,
release-profile bundles, the Fixture2015 fixture set, Cgroup2015 cgroup tests, or the
prepush gate (separate tasks own those); nor billing/pricing effects (creation/ dir).
Ubuntu-26.04 image availability was an assumption resolved by this run only for these two
labels in this repository at this time.

## Evidence index (all under `ci/spike-40489534163/`)

- `ci-20-15-evidence-producer-{x64,arm64}-19143e6…/`: `producer.json`, `build.log`,
  `rustc-version.txt`, `nextest-version.txt`, `producer-ldd.txt`
- `ci-20-15-evidence-consumer-ubuntu-{24.04,24.04-arm,26.04,26.04-arm}-19143e6…/`:
  `consumer.json`, `extract.log`, `test.log`, `junit.xml`, `consumer-ldd.txt`
- Workflow as run: `ci/ci-20-15-spike.yml`. The report job's `T1_PROVED:` print (line 384)
  is workflow code, not a downloaded log; run success is the session-provided outcome of
  run 40489534163, corroborated by all six records reporting `result: passed` /
  completed steps and by the downloaded evidence artifacts.
