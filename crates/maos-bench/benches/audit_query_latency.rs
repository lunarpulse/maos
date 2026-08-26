#![forbid(unsafe_code)]

//! audit_query_latency — Criterion benchmark for `maos_audit::query()` read-path latency.
//!
//! Seeds a temp-file SQLite with 12 000 Transparency-Log rows across 5 spirits,
//! 30 days of nanosecond timestamps, and all relevant frame kinds, then measures
//! both single-spirit filtered queries and global (unfiltered) 30-day scans.
//!
//! Run:
//!   cargo bench -p maos-bench --bench audit_query_latency
//!   cargo bench -p maos-bench --bench audit_query_latency -- --test

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use maos_audit::{query, AuditFilter};
use rusqlite::Connection;
use std::path::PathBuf;

// ── Fixture constants ──────────────────────────────────────────────────

const ROW_COUNT: usize = 12_000;
const SPIRIT_PIDS: [u32; 5] = [1001, 1002, 1003, 1004, 1005];
const FRAME_KINDS: [i64; 5] = [7, 11, 17, 19, 22]; // CapabilityInvocation, Distillate, SpiritRevoked, SpiritAdmitted, ConsentRupture
const INTENTS: &[&str] = &[
    "capability.invoke",
    "distill.commit",
    "lifecycle.revoke",
    "lifecycle.admit",
    "consent.rupture",
];

/// 30 days in nanoseconds.
const THIRTY_DAYS_NS: u64 = 30u64 * 24 * 60 * 60 * 1_000_000_000;

/// Simple linear congruential generator for deterministic "random" frame_ids.
/// Parameters from Numerical Recipes (multiplier 1664525, increment 1013904223).
struct Lcg {
    state: u64,
}
impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    /// Produce 16 deterministic bytes for a frame_id.
    fn next_frame_id(&mut self) -> [u8; 16] {
        let hi = self.next_u64().to_le_bytes();
        let lo = self.next_u64().to_le_bytes();
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&hi);
        out[8..].copy_from_slice(&lo);
        out
    }

    /// Produce 32 deterministic bytes for a capability_token.
    fn next_cap_token(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for chunk in out.chunks_exact_mut(8) {
            chunk.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        out
    }
}

/// Create a temp-file SQLite, seed it with `ROW_COUNT` rows.
/// Returns `(TempDir, PathBuf)` — caller must keep the `TempDir` alive.
fn seed_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("audit_bench.sqlite");

    {
        let conn = Connection::open(&db_path).expect("open sqlite for seeding");

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .expect("pragma");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                 frame_id BLOB NOT NULL PRIMARY KEY,
                 timestamp_ns INTEGER NOT NULL,
                 spirit_pid INTEGER NOT NULL,
                 from_spirit_id TEXT NOT NULL DEFAULT '',
                 to_spirit_id TEXT NOT NULL DEFAULT '',
                 boot_nonce INTEGER NOT NULL,
                 capability_token BLOB,
                 kind INTEGER NOT NULL,
                 intent TEXT NOT NULL,
                 payload_redacted BLOB NOT NULL,
                 origin INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS approval_decision_log (
                 decision_id INTEGER PRIMARY KEY AUTOINCREMENT,
                 timestamp_ns INTEGER NOT NULL,
                 actor TEXT NOT NULL,
                 target TEXT NOT NULL,
                 capability TEXT NOT NULL,
                 intent TEXT NOT NULL,
                 decision INTEGER NOT NULL,
                 reasoning TEXT
             );
             CREATE TABLE IF NOT EXISTS principal_index (
                 principal_id TEXT NOT NULL,
                 writer_spirit_pid INTEGER NOT NULL,
                 schema TEXT NOT NULL,
                 key TEXT NOT NULL,
                 timestamp_ns INTEGER NOT NULL,
                 PRIMARY KEY (principal_id, writer_spirit_pid, schema, key)
             );",
        )
        .expect("create tables");

        let base_ts: u64 = 1_700_000_000_000_000_000; // a plausible recent epoch-ns base
        let mut lcg = Lcg::new(0xDEAD_BEEF_CAFE_BABE);

        let tx = conn.unchecked_transaction().expect("begin tx");
        {
            let mut insert_stmt = tx
                .prepare(
                    "INSERT INTO transparency_log
                         (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                          boot_nonce, capability_token, kind, intent, payload_redacted, origin)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                )
                .expect("prepare insert");

            for i in 0..ROW_COUNT {
                let frame_id = lcg.next_frame_id();
                // Spread timestamps evenly across 30 days.
                let ts = base_ts + (THIRTY_DAYS_NS / ROW_COUNT as u64) * (i as u64);
                let pid = SPIRIT_PIDS[i % SPIRIT_PIDS.len()];
                let boot_nonce = (pid as u64) / 100; // deterministic per spirit
                let kind_idx = i % FRAME_KINDS.len();
                let kind = FRAME_KINDS[kind_idx];
                let intent = INTENTS[kind_idx];

                // ~60 % of rows get a capability_token (kind 7 always does).
                let cap_token: Option<Vec<u8>> = if kind == 7 || (i % 3 != 0) {
                    let t = lcg.next_cap_token();
                    Some(t.to_vec())
                } else {
                    None
                };

                let from_id = format!("spirit-{pid}");
                let to_id = if i % 2 == 0 {
                    "host".to_owned()
                } else {
                    format!("spirit-{}", SPIRIT_PIDS[(i + 1) % SPIRIT_PIDS.len()])
                };
                let payload = format!("{{\"i\":{i}}}");
                let origin = 0i64; // host origin

                insert_stmt
                    .execute(rusqlite::params![
                        frame_id.as_slice(),
                        ts as i64,
                        pid as i64,
                        from_id,
                        to_id,
                        boot_nonce as i64,
                        cap_token,
                        kind,
                        intent,
                        payload.as_bytes(),
                        origin,
                    ])
                    .unwrap_or_else(|e| panic!("insert row {i}: {e}"));
            }
        }
        tx.commit().expect("commit tx");
    }

    (dir, db_path)
}

// ── Benchmarks ─────────────────────────────────────────────────────────

fn bench_single_spirit_query(c: &mut Criterion) {
    let (_dir, db_path) = seed_fixture();

    let mut group = c.benchmark_group("audit_query_single_spirit");
    group.sample_size(20);

    for &pid in &SPIRIT_PIDS {
        let filter = AuditFilter {
            spirit_pid: Some(pid),
            ..Default::default()
        };
        group.bench_with_input(BenchmarkId::new("spirit_pid", pid), &filter, |b, f| {
            b.iter(|| {
                let entries = query(&db_path, f.clone()).expect("query");
                black_box(entries);
            });
        });
    }

    group.finish();
}

fn bench_global_query(c: &mut Criterion) {
    let (_dir, db_path) = seed_fixture();

    let mut group = c.benchmark_group("audit_query_global");
    group.sample_size(10);

    // Full 30-day scan, no filters.
    group.bench_function("full_30day", |b| {
        let filter = AuditFilter::default();
        b.iter(|| {
            let entries = query(&db_path, filter.clone()).expect("query");
            black_box(entries);
        });
    });

    group.finish();
}

fn bench_kind_filtered_query(c: &mut Criterion) {
    let (_dir, db_path) = seed_fixture();

    let mut group = c.benchmark_group("audit_query_kind_filtered");
    group.sample_size(20);

    // Filter to only CapabilityInvocation frames (kind 7) for one spirit.
    //
    // D12 (Story 14-0 AC3.3) — REPAIRED, NOT RETIRED. This read
    // `"capability.invoke"`, which is not a frame kind: `kind_from_string`
    // (`maos-audit/src/lib.rs:716`) accepts `"capability.invocation"` /
    // `"CapabilityInvocation"` and nothing else, so `query` returned
    // `AuditError::UnknownKind` and the `.expect` below panicked. Broken since
    // Epic 9.1 and run ZERO times, because the bench was declared in
    // `maos-bench/Cargo.toml` and appeared in no workflow and no xtask — a
    // D11(b) instance in its own right. Repairing the string without giving it
    // an execution path would have left it exactly as unrun, so `discipline.yml`
    // now executes it in criterion `--test` mode on every push.
    //
    // NOT a defect, contrary to the filed row: `INTENTS[0]` at `:24` is also
    // `"capability.invoke"`, but that is the free-text `intent` COLUMN, not a
    // frame kind. It is left alone.
    let filter = AuditFilter {
        spirit_pid: Some(SPIRIT_PIDS[0]),
        kind: Some("capability.invocation".to_owned()),
        ..Default::default()
    };

    // FAIL LOUDLY BEFORE TIMING ANYTHING. `UnknownKind` already panics, but a
    // kind that is VALID and simply wrong would return zero rows and this bench
    // would happily report the latency of matching nothing. The fixture seeds
    // kind 7 for every fifth row across five spirits, so a correct filter cannot
    // be empty. Benchmarking a degenerate query is the same defect one turn on:
    // a number that looks like evidence and measures nothing.
    let seeded = query(&db_path, filter.clone()).expect("kind-filtered query must resolve");
    assert!(
        !seeded.is_empty(),
        "audit_query_latency: the kind filter matched ZERO of {ROW_COUNT} seeded rows — \
         the bench would time an empty scan and report it as a latency measurement"
    );

    group.bench_function("capability_invocation_single_spirit", |b| {
        b.iter(|| {
            let entries = query(&db_path, filter.clone()).expect("query");
            black_box(entries);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_spirit_query,
    bench_global_query,
    bench_kind_filtered_query,
);
criterion_main!(benches);
