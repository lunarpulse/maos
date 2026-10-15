# Fuzz Cadence Runbook (NFR-Sec-5 / NFR-Sec-6)

Operational runbook for the MAOS fuzz targets. Three targets, three crates:

| Target `[[bin]]` | Crate | Path | Fuzz surface |
|------------------|-------|------|--------------|
| `manifest_parser` | `maos-manifest` | `crates/maos-manifest/fuzz/fuzz_targets/manifest_parser.rs` | All 23 `*::from_toml_str` manifest section parsers |
| `frame_deser` | `maos-domain` | `crates/maos-domain/fuzz/fuzz_targets/frame_deser.rs` | `IacFrame` wire deserialization (JSON + canonical-CBOR) |
| `fuzz_exec_deps_parse_elf` | `maos-exec-deps` | `crates/maos-exec-deps/fuzz/fuzz_targets/fuzz_exec_deps_parse_elf.rs` | Hostile-input ELF64 dependency parsing before T2 Landlock grants |

All three targets are standalone `cargo-fuzz` projects (`[package.metadata] cargo-fuzz = true`,
`libfuzzer-sys` harness via `fuzz_target!`). They build with the nightly toolchain.

## Tiered cadence

### T1 — per-commit build only

`discipline.yml`'s `fuzz-build` compiles all three targets. It does not claim
runtime fuzz evidence.

### T2 — nightly scheduled

| Field | Value |
|-------|-------|
| Trigger | `fuzz-cadence.yml`, 03:00 UTC daily; `workflow_dispatch` for backfill |
| Duration | 4 hours per target (`-max_total_time=14400`) |
| Workers | Runner cores (`nproc`), each with an independent libFuzzer log |
| Runner | GitHub-hosted Ubuntu; 355-minute job timeout includes setup |
| Command | `bash scripts/run-fuzz-link.sh <fuzz-dir> <target> 14400 T2 nightly [rss-flag]` |
| Ledger append | One measured record per target; separate collector on `fuzz-ledger` |

S1 (operator, 2026-10-15) reverses the Epic-12 weekly downgrade: architecture
`5-spirit-abi.md:185-197` and PRD NFR-Sec-5/6 require the non-negotiable floor.
At four cores, 90 nightly runs × 4h × 4 workers yields approximately 1,440
CPU-h/target and 4,320 aggregate, above the unchanged 72/1,000 CPU-h floors.
These are capacity estimates, not ledger values. Crashes fail the cadence;
the separate workflow does not replace the per-commit compile gate.

### T3 — pre-release chain

| Field | Value |
|-------|-------|
| Trigger | `fuzz-prerelease.yml`: `workflow_dispatch` or a `v*-rc*` tag |
| Duration | Five sequential 5h links per target; final assertion requires ≥86,400 measured libFuzzer seconds per target |
| Workers | Runner cores; independent worker counters, zero crashes |
| Resume | Each link downloads the previous link's target-specific corpus artifact |
| Ledger append | One `tier:\"T3\"` record per completed target/link, with the triggering SHA |

Four ≤6h links cannot hold 24h of measured fuzzing plus setup under the hosted
6h job cap. The operator ruled that link **count** yields to the ≥24h requirement
(PRD NFR-Sec-5/6): five 5h links, each with a 355-minute job timeout. Any failed
worker, crash artifact, missing counter, missing link, or measured total below
24h fails the chain. Completed-link evidence remains collectible even if a later
link fails. No self-hosted runner is required. On a tag, all links record that
tagged SHA; on a manual run, they record the dispatched SHA. The release-candidate
precondition belongs to **20-5** and is not implemented here.

## Toolchain & invocation

cargo-fuzz needs the **nightly** toolchain (`-Zsanitizer=address` etc.). Cadence
workflows select `nightly-2026-09-25` through `RUSTUP_TOOLCHAIN`, keeping shell
scripts free of inline toolchain overrides. Local `cargo +nightly fuzz …` examples
require an equivalent toolchain. cargo-fuzz takes the fuzz project
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

`fuzz-ledger.json` (repo root) is a **schema-versioned, append-only** ledger.
T2/T3 jobs upload records; `scripts/collect-ledger.sh` appends them.

Operator-provisioned initial content (the collector never creates a branch/file):

```json
{"schema_version":1,"records":[]}
```

Append contract — each record is:

```json
{
  "target": "manifest_parser",
  "commit": "<40-char git SHA>",
  "cpu_seconds": 14403,
  "corpus_size": 10,
  "timestamp": "2026-10-15T00:00:00Z",
  "tier": "T2",
  "workers": 1,
  "run_id": "<workflow-run-id>-nightly"
}
```

- `target` — one of `manifest_parser`, `frame_deser`, or `fuzz_exec_deps_parse_elf`.
- `commit` — the git SHA the fuzzed binary was built from.
- `cpu_seconds` — sum of each worker's libFuzzer `Done <N> runs in <S> second(s)`
  counter. `scripts/record-fuzz.py` refuses a record if any worker's counter is
  missing/ambiguous or its log is unreadable. Never configured duration × workers.
- `corpus_size` — number of corpus entries after the run (seed + discovered). Intentional superset of the spec's "corpus delta" field: the delta between runs is derivable as the difference of consecutive `corpus_size` values for the same target, while the absolute count is the more useful primary record.
- `timestamp` — RFC 3339 UTC.
- `tier`, `workers`, `run_id` — collection provenance; the floor gate reads
  only `target`, `cpu_seconds`, and `timestamp`, ignoring extra fields.

The operator lane **ops-provisioning-secrets-and-accounts** must provision the
`fuzz-ledger` branch and `fuzz-ledger.json` and grant the workflow write access.
A missing branch/file or rejected push fails the collector. It never seeds a
branch, invents records, or treats an artifact upload as a successful append.
Checkout's persisted credentials use `FUZZ_LEDGER_WRITE_TOKEN` when configured,
otherwise `GITHUB_TOKEN`. Soaks run independently; no workflow-wide concurrency
group can cancel a queued release chain. On Git's porcelain `[rejected] (fetch
first)` or `[rejected] (non-fast-forward)` status only, collectors refetch and
reapply artifacts with deduplication, bounded to five push attempts. Hook/policy/
authentication rejections fail immediately. Malformed or multi-value JSON
record files fail before commit; an existing identical fuzz record is not
appended twice. Records are applied in a detached temporary worktree so the gate
always executes the current source checkout, not historical ledger-branch code.

Append-only: never mutate or delete existing records. The ledger grows
monotonically; floor assertions sum `cpu_seconds` over the trailing 90-day
window per target and in aggregate.

## Floor assertions

Enforced by `cargo run -p xtask -- check-fuzz-floor --json` (a ship-gate job).
Missing, unreadable, or zero-record ledgers report `evidence_state:"ABSENT"`,
`passed: false`, and exit 1 at `v1_5` under the registry's blocking disposition.
A nonempty ledger spanning <90 days reports `INDETERMINATE`, `passed: false`,
`advisory: true`, and exits 0; its accumulated `per_target_seconds` and
`aggregate_seconds` are still reported. At ≥90 days the floor is enforceable.
Malformed JSON or records always fail. Downstream jobs always invoke the gate,
even when collection failed; they load only the evidence file, never switch gate
code to the ledger branch. The following manual `jq` summaries do not replace the
gate:

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
- `crates/maos-exec-deps/fuzz/corpus/fuzz_exec_deps_parse_elf/` — crafted
  truncated, static ELF64, and dynamic-ELF parser inputs (3 seeds).
