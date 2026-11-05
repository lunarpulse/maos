# AC5 — profiled kernel-free repair and final local evidence

Base: `cb0abc06`. This continues the superseded interim handoff, not an operator-authorized scope reduction. No kernel/domain code, route deadline, observation budget, timeout, or quarter-scan threshold was widened.

## Root causes, not timing exceptions

1. **A rowid WHERE clause did not bound the chosen access path.** SQLite selected `idx_tlog_kind(kind,timestamp_ns)` and re-examined the whole kind-11 population per nominal page. The genuine Rust before witness records **724587 VM steps** for the first 100k-row window (`l1-ac5-work-before.log:81-120`, exit101). The independent 1M/1KiB profile records **7024500 steps** for the first page. Forced direct rowid access takes45000–45100 steps; the selected kind-only index, whose implicit second key is rowid, takes36800 in that profile. See `l1-ac5-resume-profile-before.json`, `l1-ac5-resume-profile-1m.json`, `l1-ac5-kind-rowid-index-profile.json`.
2. **A second full-TL scan was outside the original pager.** `HookPump::complete_forget` called `outstanding_redistill_requests` on an async worker. Its anti-join held the TL mutex across the entire population, including logs containing no requests. The genuine before witness records300011/3000011 VM steps at100k/1M, each a single window consuming the entire query's work (`l1-ac5-recovery-before.log:82-110,139`, exit101). This explains why fixing only kind access still left the real 1M handler pending beyond its follow-on observation interval; that failed run is retained in `l1-ac5-index-and-door-final.log`, not reclassified as host noise.
3. **Receipt ordering needed explicit restoration.** The new rowid traversal exposed malformed ids in insertion order6,4,5 instead of timestamp/frame-id order5,4,6 (`l1-ac5-malformed-order-before.log`, exit101). Malformed receipt ids now sort by their timestamp/id pair before the public vector is returned.
4. **The new outcome index must not change the opaque TL write contract.** An unguarded JSON expression index made a previously storable opaque payload panic during insertion (`l1-ac5-outcome-index-before.log`, exit101). The final expression guards JSON parsing, uses a disjoint internal BLOB key for invalid JSON, and performs a second indexed lookup that preserves an explicit malformed-JSON read error rather than silently ignoring the bad outcome. JSON5-valid inputs retain the prior SQLite JSON extraction semantics. The witness asserts both successful storage and the honest read error.

## Final implementation

- Stable4096-rowid windows release the TL mutex between pages and reach later appended rows.
- General predicates pin direct rowid access; kind predicates pin `idx_tlog_kind_rowid`.
- Recovery requests pin `idx_tlog_intent_rowid`; outcome matching and malformed-outcome detection use expression-index point lookups, not correlated population rescans. Recovery results retain timestamp/frame-id ordering.
- Both the recovery query and late sweep leave Tokio workers. Durable hold placement/release and body mutation retain their serialization; the live door releases through the Memory Manager serializer.
- Only test builds collect SQLite VM work from the actual prepared production statements. No production profiling holder, per-page logging allocation, dependency, or environment variable was added.
- The VM fixture isolates query work in memory; the actual door fixtures retain100k/1M rows with1KiB payloads in disk/WAL storage. The continuous writer now runs through handler completion, including any pending interval, and has panic-safe shutdown.

## Witnesses and controls

Paths under `crates/maos-iac/src/adapter/erasure_scan_work.rs`:

- `ac5_every_scan_has_bounded_sqlite_work_at_100k:21`
- `ac5_every_scan_has_bounded_sqlite_work_at_1m:26`
- `ac5_rowid_scans_restore_timestamp_and_frame_id_order:96`
- `ac5_opaque_outcome_payload_is_storable_but_bad_recovery_is_reported:132`

`l1-ac5-all-boundaries-final.log`: all4 pass. `l1-final-iac-lib.log`: the complete affected IAC unit suite passes103/103 after final plant restoration. Maximum dense principal/lineage window work is110619 VM steps; kind/malformed windows36881. Maximum constituent fractions are0.04095960 at100k and0.004095994 at1M, below the unchanged0.25 bound. All five forget/recovery predicates are exercised.

The final guarded `i64::MAX` page plant makes **both sizes red at fraction1**, specifically the quarter-scan assertion (`l1-ac5-final-guarded-plant.log:81,102,127-133`, exit101). The same log records `PLANT_RESTORE_COPY_CMP_TOUCH_EXIT=0`; the exact one-line diff is `l1-ac5-final-guarded-plant.diff`. No mutation remains applied.

Paths under `crates/maos-bin/tests/`:

- `shell_host_16_2.rs::ac5_door_bounded_scan_admits_continuous_writer:1503`
- `shell_host_16_2.rs::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption:1508`
- `shell_host_16_2.rs::ac5_door_hold_release_waits_for_inflight_forget:1421`
- `erasure_scan_20_11.rs::ac5_hold_between_forget_and_late_sweep_preserves_late_frame:7`

## Final real door measurements

`l1-final-focused-and-preservation.log:684-695`, exit0; writer timing covers the full handler interval:

| Rows | Handler wall | Max insert | Raw wall ratio | Outcome |
|---|---:|---:|---:|---|
|100000|1.070610323s|232.483050ms|0.2171500171|Completed; identical subsequent request completed; tombstone/unrelated bytes intact|
|1000000|8.480294334s|2.000848329s|0.2359409061|Completed within the unchanged10s route; identical subsequent request completed; tombstone/unrelated bytes intact|

Both raw observations also satisfy the literal25% bound. Earlier failures remain recorded, not retried away. The earlier recovery-fixed run (`l1-ac5-recovery-fixed.log`) exercised a genuine1M `handler_still_running`: it was logged **pending**, its durable completed row was observed, and the identical subsequent request completed. The final stricter full-writer interval run completed without needing that branch. A pending response is never represented as an erased report or a proof bundle.

The final local bin focus passes53 test cases, of which one is an explicit missing-substrate T3 runtime skip, not hosted proof: inventory9; live local1 plus hosted skip; late-hold1;13.5b26;18-6 floors1; shell14. The independently required kernel/audit/registry preservation command also exits0. Hosted T3, delegated cgroup/network and final-kernel AC3/O4/KS remain unverified/untouched here.

## AC5-stage exact source measurement after formatting

`l1-final-boundaries-formatted-counts.json` measures bin34842, IAC8550, audit7775, registry4322, xtask44466, aggregate193900. Exact pins/hardfail193901 move together, all within the original stricter grants. Kernel26841 physical/21042 tokei and domain10107 are unchanged. `l1-final-kloc-check.json` and `l1-final-check-kernel-baseline.json` exit0.

The subsequent production FR4 marker/file-guard closure adds10 audit LOC and1 IAC LOC: final IAC8551, audit7785, aggregate193911, hardfail193912; all other rows/ceilings above are unchanged. Exact final gate `l1-fr4-final-kloc-check.json` exits0. The complete IAC target again passes103/103 after its explicit file-level test guard (`l1-final-guard-iac-and-default-reset.stdout`). Final whole-landing commands, once-only prepush and distinct budget advisories are in `l1-final-report.md`.
