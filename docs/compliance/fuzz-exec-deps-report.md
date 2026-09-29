# Fuzz Report — ELF Dependency Parser (`fuzz_exec_deps_parse_elf`)

**Status: wired** — the target passes a local stable `cargo check`. `cargo-fuzz`
was unavailable on this developer host, so no local libFuzzer smoke is recorded.
T1/T2/T3 runs populate the CPU-hour ledger after merge.

## Target

| Field | Value |
|-------|-------|
| Story | 17-6 RT-3 / V-3 |
| Target `[[bin]]` | `fuzz_exec_deps_parse_elf` |
| Crate | `maos-exec-deps` |
| Harness | `crates/maos-exec-deps/fuzz/fuzz_targets/fuzz_exec_deps_parse_elf.rs` |
| Fuzz surface | `maos_exec_deps::parse_elf(Cursor<&[u8]>)`, the hostile-input ELF64 little-endian parser used before T2 Landlock grants |
| Seed corpus | `crates/maos-exec-deps/fuzz/corpus/fuzz_exec_deps_parse_elf/` — crafted truncated, static ELF64, and dynamic-ELF images |

## Harness design

`#![no_main]` + `libfuzzer_sys::fuzz_target!(|data: &[u8]| { … })` passes every
byte sequence to `parse_elf(std::io::Cursor::new(data))`. Parse `Err`s are the
expected refusal path for malformed ELF; a panic, abort, or sanitizer finding
is a parser defect.

## LibFuzzer invocation

```bash
# Build-only gate (pre-merge):
cargo +nightly fuzz build --fuzz-dir crates/maos-exec-deps/fuzz fuzz_exec_deps_parse_elf

# Local smoke (60s):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-exec-deps/fuzz fuzz_exec_deps_parse_elf -- \
  -max_total_time=60

# Full T3 run (24h):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-exec-deps/fuzz fuzz_exec_deps_parse_elf -- \
  -max_total_time=86400 -workers=8
```

## Tiered cadence

| Tier | Trigger | Duration | Workers | cpu_seconds/record |
|------|---------|----------|---------|--------------------|
| T1 | nightly scheduled CI | 10 min | 4 | 2400 |
| T2 | nightly cron | 4 h | 8 | 115200 |
| T3 | pre-release manual run | 24 h | 8 | 691200 |

Pre-merge is build-only. T1/T2/T3 ledger collection and floor enforcement are
specified in `docs/runbooks/fuzz-cadence.md`.

## CPU-hour ledger (`fuzz_exec_deps_parse_elf`)

| Run | Date | Commit | Duration | Workers | cpu_seconds | Crashes | Notes |
|-----|------|--------|----------|---------|-------------|---------|-------|
| T1 | — | — | — | — | — | — | pending CI post-merge |
| T2 | — | — | — | — | — | — | pending nightly |
| T3 | — | — | — | — | — | — | pending pre-release |

**Cumulative cpu_seconds (this target):** 0 — below the 72 CPU-hour pre-GA
floor; scheduled CI records close the gap. The target is subject to the same
per-target floor as every entry in `check_fuzz_floor.rs`.
