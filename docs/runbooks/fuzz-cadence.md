# Fuzz Cadence Runbook (NFR-Sec-5 / NFR-Sec-6)

Operational runbook for the MAOS fuzz targets. Two targets, two crates:

| Target `[[bin]]` | Crate | Path | Fuzz surface |
|------------------|-------|------|--------------|
| `manifest_parser` | `maos-manifest` | `crates/maos-manifest/fuzz/fuzz_targets/manifest_parser.rs` | All 23 `*::from_toml_str` manifest section parsers |
| `frame_deser` | `maos-domain` | `crates/maos-domain/fuzz/fuzz_targets/frame_deser.rs` | `IacFrame` wire deserialization (JSON + canonical-CBOR) |

Both targets are standalone `cargo-fuzz` projects (`[package.metadata] cargo-fuzz = true`,
`libfuzzer-sys` harness via `fuzz_target!`). They build with the nightly toolchain.

## Tiered cadence

### T1 — nightly scheduled (wired in CI)

| Field | Value |
|-------|-------|
| Trigger | Nightly cron (03:00 UTC) via the `fuzz-cadence.yml` workflow (`.github/workflows/fuzz-cadence.yml`), plus a manual `workflow_dispatch` trigger for backfill / release close-out. It is a SEPARATE workflow from `discipline.yml` so the nightly timer does not re-run the entire v1.0 ship-gate suite. **Nightly is the floor-accrual driver**: one run ≈ 0.67 CPU-hr (600 s × 4 workers), so the ≥72 CPU-hr/target/90-day floor needs ~108 runs/target/quarter — only a nightly cadence reaches it. |
| Duration | 10 minutes per target |
| Workers | 4 (`-workers=4`) |
| Command | manifest: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-manifest/fuzz manifest_parser -- -max_total_time=600 -workers=4` <br> frame_deser: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-domain/fuzz frame_deser -- -max_total_time=600 -workers=4 -rss_limit_mb=0` |
| Ledger-append | The job uploads a per-target record as a workflow artifact; the separate `fuzz-ledger-collect` job appends to `fuzz-ledger.json` on the dedicated `fuzz-ledger` branch (see [Ledger append](#ledger-append-format)). Decoupled from `main` to serialize appends and avoid protected-branch write races. |

T1 is **non-blocking** — it never fails a merge. A crash in T1 files a bug
against the owning crate; the build-only `fuzz-build` job remains the
pre-merge compile gate.

### T2 — nightly cron

| Field | Value |
|-------|-------|
| Trigger | Scheduled cron, once per day |
| Duration | 4 hours per target |
| Workers | 8 (`-workers=8`) |
| Command | manifest: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-manifest/fuzz manifest_parser -- -max_total_time=14400 -workers=8` <br> frame_deser: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-domain/fuzz frame_deser -- -max_total_time=14400 -workers=8 -rss_limit_mb=0` |
| Ledger-append | One record per target after the run |

### T3 — pre-release (manual)

| Field | Value |
|-------|-------|
| Trigger | Manual, before tagging a release |
| Duration | 24 hours per target |
| Workers | 8 (`-workers=8`) |
| Command | manifest: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-manifest/fuzz manifest_parser -- -max_total_time=86400 -workers=8` <br> frame_deser: `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0 cargo +nightly fuzz run --fuzz-dir crates/maos-domain/fuzz frame_deser -- -max_total_time=86400 -workers=8 -rss_limit_mb=0` |
| Ledger-append | One record per target after the run; verify floors (see [Floor assertions](#floor-assertions)) before GA |

## Toolchain & invocation

cargo-fuzz needs the **nightly** toolchain (`-Zsanitizer=address` etc.), so all
commands use `cargo +nightly fuzz …`. cargo-fuzz 0.13.2 takes the fuzz project
via `--fuzz-dir <dir>` (NOT `--manifest-path`); each fuzz crate is its own
standalone workspace root (empty `[workspace]` table in its `Cargo.toml`).
Build-only pre-merge gate: `cargo +nightly fuzz build --fuzz-dir <dir> <target>`.

## frame_deser runtime configuration (REQUIRED)

`serde_cbor` 0.11 (unmaintained) TRUSTS attacker-controlled CBOR length prefixes
and amplifies tiny inputs (a handful of bytes) into multi-GB allocation
requests. This is a **library limitation, not a MAOS defect**: IacFrame's
production wire path (the JSON arm) is a streaming parser that never amplifies,
and IacFrame is never CBOR-serialized in production. To keep the harness from
aborting on these OOM-class requests, every `frame_deser` invocation MUST set:

- `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0` — ASAN returns NULL
  (→ serde `Err`, swallowed by the harness) instead of aborting on a
  `allocation-size-too-big` request (>1 TB).
- `-rss_limit_mb=0` — disables libFuzzer's malloc-hook abort on a multi-GB
  allocation request, so ASAN can refuse it (the >1 TB requests) / Rust can
  reject it without libFuzzer pre-aborting.

Empirically the 7.5 GB-class inputs execute in ~3 ms (allocation refused →
`Err` → swallowed); a 60 s soak completes 1.5 M+ runs with zero crashes. (Both
flags are also harmless for `manifest_parser`, which does not amplify.)

## Ledger append format

`fuzz-ledger.json` (repo root) is a **schema-versioned, append-only** ledger. CI
post-merge T1 jobs (and T2/T3 operators) append one record per completed run.

Initial content:

```json
{"schema_version":1,"records":[]}
```

Append contract — each record is:

```json
{
  "target": "manifest_parser",
  "commit": "<40-char git SHA>",
  "cpu_seconds": 2400,
  "corpus_size": 10,
  "timestamp": "2026-06-22T00:00:00Z"
}
```

- `target` — one of `manifest_parser`, `frame_deser`.
- `commit` — the git SHA the fuzzed binary was built from.
- `cpu_seconds` — `duration_seconds * workers` (wall time × worker count =
  CPU time). T1 10min×4 = 2400; T2 4h×8 = 115200; T3 24h×8 = 691200.
- `corpus_size` — number of corpus entries after the run (seed + discovered). Intentional superset of the spec's "corpus delta" field: the delta between runs is derivable as the difference of consecutive `corpus_size` values for the same target, while the absolute count is the more useful primary record.
- `timestamp` — RFC 3339 UTC.

Append-only: never mutate or delete existing records. The ledger grows
monotonically; floor assertions sum `cpu_seconds` over the trailing 90-day
window per target and in aggregate.

## Floor assertions

Enforced at release time by `cargo run -p xtask -- check-fuzz-floor` (a
ship-gate job). The gate is **advisory** (warn-only, exit 0) until the ledger
spans ≥90 days of history, then auto-promotes to **hard-fail** if the floor is
unmet — closing the bootstrap hole (the floor is logically unsatisfiable for
the first 90 days after wiring). The equivalent manual `jq` (for T3 close-out
or local verification):

```bash
# 90-day cutoff (RFC 3339 UTC). `tonumber` rejects string-typed cpu_seconds
# (fail-closed); the timestamp filter honors the documented 90-day window.
CUTOFF=$(date -u -d '90 days ago' +%Y-%m-%dT%H:%M:%SZ)

# Per-target summary over the trailing 90-day window:
jq -r --arg cutoff "$CUTOFF" '
  .records | map(select(.timestamp >= $cutoff)) | group_by(.target)[]
  | "\(.[0].target): \( (map(.cpu_seconds | tonumber) | add // 0) ) seconds"
' fuzz-ledger.json

# Per-target pass gate (>= 72 CPU-hours = 259200 s) — repeat for each target:
jq -e --arg cutoff "$CUTOFF" --arg target "manifest_parser" '
  ( .records | map(select(.target == $target and .timestamp >= $cutoff))
    | map(.cpu_seconds | tonumber) | add // 0 ) >= 259200
' fuzz-ledger.json

# Aggregate floor (>= 1000 CPU-hours = 3600000 s) across ALL targets:
jq -e --arg cutoff "$CUTOFF" '
  ( .records | map(select(.timestamp >= $cutoff))
    | map(.cpu_seconds | tonumber) | add // 0 ) >= 3600000
' fuzz-ledger.json
```

`jq -e` exits non-zero when the assertion fails. A string-typed
`cpu_seconds` makes `tonumber` error (non-zero exit) — fail-closed, never
fail-open.

## Crash handling

1. A crash produces a `crash-*`/`oom-*` artifact under the target's
   `fuzz/artifacts/<target>/` dir and a libfuzzer stack report.
2. **OOM-class first.** If the artifact is `oom-*` (or the report says
   `out-of-memory` / `allocation-size-too-big`), this is the known
   `serde_cbor` amplification, NOT a defect — confirm the run used the required
   `frame_deser` runtime configuration (see above). A correctly-configured run
   produces no `oom-*` artifacts. Do not file these as bugs.
3. Reproduce a genuine `crash-*` locally with the exact recorded input:
   `cargo +nightly fuzz run --fuzz-dir crates/<crate>/fuzz <target> crash-<sha>`.
4. File the crash against the owning crate; the harness is correct-by-design
   (all `Err` swallowed), so any genuine crash is a real parser/deserializer
   defect.
5. The fix MUST land with a regression corpus entry (the crashing input) added
   to the target's `corpus/<target>/` dir.

## Seed corpora

- `crates/maos-manifest/fuzz/corpus/manifest_parser/` — valid TOML section
  fragments mined from `spirits/hello-spirit/manifest.toml` (10 seeds).
- `crates/maos-domain/fuzz/corpus/frame_deser/` — valid `IacFrame` instances
  serialized to JSON (`.json`) and canonical CBOR (`.cbor`), built from the
  `frame.rs` test fixtures (5 frames × 2 formats = 10 seeds).
