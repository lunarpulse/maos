# Fuzz Report — Manifest Parser (`manifest_parser`)

**Status: scaffold** — target builds and passes a 60s smoke with zero crashes.
T1/T2/T3 runs populate the CPU-hour ledger below.

## Target

| Field | Value |
|-------|-------|
| NFR | NFR-Sec-5 (manifest parser fuzz hardening) |
| Target `[[bin]]` | `manifest_parser` |
| Crate | `maos-manifest` |
| Harness | `crates/maos-manifest/fuzz/fuzz_targets/manifest_parser.rs` |
| Fuzz surface | All 23 `*::from_toml_str` manifest section parsers (SandboxConfig, ResourceCaps, ClassSection, CapabilitiesRequired, PostureSection, OutputShape, Budget, Author, EpistemicPolicySection, SchedulingSection, LifecycleSection, OnCrashSection, OnRevocationSection, SchedulesSection, SupervisionSection, ModelProvenanceSection, ProvidersSection, McpSection, HotSwapManifestSection, MigratesFromSection, HaltProtocolCompatibilitySection, CliWrapperConfig, GatewaysSection) |
| Seed corpus | `crates/maos-manifest/fuzz/corpus/manifest_parser/` — **10 seeds** (TOML fragments mined from `spirits/hello-spirit/manifest.toml`) |

## Harness design

`#![no_main]` + `libfuzzer_sys::fuzz_target!(|data: &[u8]| { … })`. The harness
converts the raw bytes to `&str` via `std::str::from_utf8` (non-UTF-8 returns
early — non-crash), then feeds the SAME `&str` to all 23 `from_toml_str` entry
points. Every call is `let _ = Type::from_toml_str(s);`, swallowing any
`Result<_, ManifestError>` Err. A parse/validation failure is the expected,
non-crashing contract for adversarial TOML; the harness reports a bug only when
a parser panics or aborts.

## Libfuzzer invocation

```bash
# Build-only gate (pre-merge):
cargo +nightly fuzz build --fuzz-dir crates/maos-manifest/fuzz manifest_parser

# Local smoke (60s):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-manifest/fuzz manifest_parser -- \
  -max_total_time=60

# Full T3 run (24h):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-manifest/fuzz manifest_parser -- \
  -max_total_time=86400 -workers=8
```

## Tiered cadence

| Tier | Trigger | Duration | Workers | cpu_seconds/record |
|------|---------|----------|---------|--------------------|
| T1 | per-commit post-merge (CI) | 10 min | 4 | 2400 |
| T2 | nightly cron | 4 h | 8 | 115200 |
| T3 | pre-release (manual) | 24 h | 8 | 691200 |

Pre-merge is build-only (`cargo fuzz build` succeeds). T1 is the first soak.
See `docs/runbooks/fuzz-cadence.md`.

## CPU-hour ledger (per-target floor >= 72 CPU-hours / 90 days)

> Populated by CI from `fuzz-ledger.json`. Per-target floor: 72 CPU-hours =
> 259 200 seconds of `cpu_seconds` per target over the trailing 90 days.

| Run | Date | Commit | Duration | Workers | cpu_seconds | Crashes | Notes |
|-----|------|--------|----------|---------|-------------|---------|-------|
| _(local smoke — not CI-reproduced)_ | 2026-06-22 | — | 60 s | 1 | 60 | 0 | 143,394 runs; zero crashes (cov 3077 / ft 5811) |
| T1 | — | — | — | — | — | — | pending CI post-merge |
| T2 | — | — | — | — | — | — | pending nightly |
| T3 | — | — | — | — | — | — | pending pre-release |

> **Provenance:** the smoke row above is a single 60 s run on a developer host
> (2026-06-22), recorded by hand — NOT a CI-reproduced artifact. The run-count
> and coverage figures are point-in-time and are superseded by the CI-appended
> T1/T2/T3 records in `fuzz-ledger.json`, which are the authoritative evidence
> for the NFR-Sec-5 CPU-hour floor.
**Last-run duration:** 60 s (local smoke). **Crash count:** 0 (target: 0).
**Cumulative cpu_seconds (this target):** 0 — below floor; T1/T2/T3 runs close
the gap.
