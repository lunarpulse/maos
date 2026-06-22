# Fuzz Report — Wire Protocol (`frame_deser`)

**Status: scaffold** — target builds and passes a 60s smoke with zero crashes.
T1/T2/T3 runs populate the CPU-hour ledger below.

## Target

| Field | Value |
|-------|-------|
| NFR | NFR-Sec-6 (wire protocol / IacFrame deserialization fuzz) |
| Target `[[bin]]` | `frame_deser` |
| Crate | `maos-domain` |
| Harness | `crates/maos-domain/fuzz/fuzz_targets/frame_deser.rs` |
| Fuzz surface | `IacFrame` wire deserialization — arm 1: `serde_json` (PRODUCTION wire format); arm 2: `serde_cbor` (canonical CBOR); arm 3: CBOR round-trip (deserialize → re-serialize → deserialize) |
| Seed corpus | `crates/maos-domain/fuzz/corpus/frame_deser/` — **10 seeds** (5 valid IacFrames × 2 formats: `.json` + `.cbor`), built from `frame.rs` test fixtures |

## CBOR crate decision (preflight N6 — RESOLVED)

IacFrame's **PRODUCTION** on-wire path is `serde_json`: the JSON-RPC transport
(`maos-a2a-core/src/transport/json_rpc.rs`), `maos-a2a-tcp`, and `maos-iac` all
deserialize frames via `serde_json`. IacFrame is **never** CBOR-serialized in
production. The workspace's canonical-CBOR crate is `serde_cbor = "0.11"` (used
by `maos-compliance/src/canonical_cbor.rs` for Merkle/digest hashing). This
harness uses `serde_cbor` for the CBOR arms purely to exercise the `Deserialize`
impl against a second self-describing format — the production wire format
(JSON) is arm 1.

## SmallVec (preflight N5 — RESOLVED, NON-ISSUE)

The harness deserializes raw `&[u8]` via serde, so `SmallVec`'s `Arbitrary`
impl is irrelevant — there is no `derive(Arbitrary)` struct and no manual
`Arbitrary` impl anywhere in this harness.

## Harness design

`#![no_main]` + `libfuzzer_sys::fuzz_target!(|data: &[u8]| { … })`. Two arms,
both swallowing `Err` (a deserialize failure is the expected, non-crashing
contract for malformed bytes):

1. `let _ = serde_json::from_slice::<IacFrame>(data);` — production wire path.
2. CBOR deserialize + round-trip — `if let Ok(frame) = serde_cbor::from_slice::<IacFrame>(data) { if let Ok(bytes) = serde_cbor::to_vec(&frame) { let _ = serde_cbor::from_slice::<IacFrame>(&bytes); } }`. The `if let Ok` performs the CBOR deserialize on every input (so the bare Deserialize-against-CBOR coverage is preserved), then additionally round-trips to catch non-idempotent Serialize/Deserialize pairs.

A bug is reported only when serde (or the `Deserialize` impl it drives) panics
or aborts. (Review 2026-06-22: the prior standalone single-pass CBOR arm was
dropped as fully redundant with this arm's first operation.)

## Libfuzzer invocation

```bash
# Build-only gate (pre-merge):
cargo +nightly fuzz build --fuzz-dir crates/maos-domain/fuzz frame_deser

# Local smoke (60s) — REQUIRED runtime config (see below):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-domain/fuzz frame_deser -- \
  -max_total_time=60 -rss_limit_mb=0

# Full T3 run (24h):
ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 \
  cargo +nightly fuzz run --fuzz-dir crates/maos-domain/fuzz frame_deser -- \
  -max_total_time=86400 -workers=8 -rss_limit_mb=0
```

### Required runtime config (serde_cbor amplification)

`serde_cbor` 0.11 trusts attacker-controlled CBOR length prefixes and amplifies
tiny inputs into multi-GB allocation requests (a library limitation, not a MAOS
defect — IacFrame's production wire path is the JSON arm and never amplifies).
Every `frame_deser` run MUST set `ASAN_OPTIONS=allocator_may_return_null=1:
detect_leaks=0` and `-rss_limit_mb=0`; otherwise the harness aborts on OOM-class
inputs before they are refused. Full rationale + verification in
`docs/runbooks/fuzz-cadence.md` § "frame_deser runtime configuration".

## Tiered cadence

| Tier | Trigger | Duration | Workers | cpu_seconds/record |
|------|---------|----------|---------|--------------------|
| T1 | per-commit post-merge (CI) | 10 min | 4 | 2400 |
| T2 | nightly cron | 4 h | 8 | 115200 |
| T3 | pre-release (manual) | 24 h | 8 | 691200 |

Pre-merge is build-only (`cargo fuzz build` succeeds). T1 is the first soak.
Reference: `docs/runbooks/fuzz-cadence.md`.

## Floor assertions (BOTH required pre-GA)

This target satisfies **two** floors, enforced via `jq -e` against
`fuzz-ledger.json` (see `docs/runbooks/fuzz-cadence.md`):

1. **Per-target** — `>= 72 CPU-hours (259 200 s)` of `cpu_seconds` for
   `frame_deser` over the trailing 90 days.
2. **Aggregate** — `>= 1000 CPU-hours (3 600 000 s)` across **all** fuzz
   targets (`manifest_parser` + `frame_deser`) over the trailing 90 days.

## CPU-hour ledger (frame_deser)

| Run | Date | Commit | Duration | Workers | cpu_seconds | Crashes | Notes |
|-----|------|--------|----------|---------|-------------|---------|-------|
| _(local smoke — not CI-reproduced)_ | 2026-06-22 | — | 60 s | 1 | 60 | 0 | 1,577,085 runs; zero crashes with required runtime config |
| T1 | — | — | — | — | — | — | pending CI post-merge |
| T2 | — | — | — | — | — | — | pending nightly |
| T3 | — | — | — | — | — | — | pending pre-release |

> **Provenance:** the smoke row above is a single 60 s run on a developer host
> (2026-06-22, with the required `ASAN_OPTIONS`/`-rss_limit_mb=0` runtime
> config), recorded by hand — NOT a CI-reproduced artifact. The run count is
> point-in-time and is superseded by the CI-appended T1/T2/T3 records in
> `fuzz-ledger.json`, which are the authoritative evidence for the NFR-Sec-6
> CPU-hour floor.
**Last-run duration:** 60 s (local smoke). **Crash count:** 0 (target: 0).
**Cumulative cpu_seconds (this target):** 0 — below per-target floor; T1/T2/T3
runs close the gap. **Aggregate cpu_seconds (all targets):** 0 — below the
1000 CPU-hours pre-GA floor.
