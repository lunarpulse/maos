#![forbid(unsafe_code)]

//! Story 10.4a (NFR-Ops-10) — deterministic 10⁶-row SQLite TL corpus generator.
//!
//! Generates a SQLite Transparency Log database with exactly 1 000 000 rows in
//! the FULL 11-column production schema
//! (`maos-iac/src/adapter/transparency_log.rs:246-258`) for migration-engine
//! stress testing (multi-batch boundary: 100 batches at `BATCH_SIZE=10_000`).
//! The corpus is fully deterministic from a fixed seed — re-running produces
//! byte-identical rows, hence a stable SHA-256 / Merkle root / payload oracle.
//!
//! # Determinism (row `i`, 0-indexed)
//!
//! - `frame_id` (16 B): `[SEED_be(8) | i_be(8)]` — guaranteed unique.
//! - `timestamp_ns`: `BASE_TS_NS + i` (1 ns steps, monotonic).
//! - `spirit_pid`: `(i % 8)`.
//! - `from_spirit_id`: `spirit-{i%8}`; `to_spirit_id`: `orchestrator`.
//! - `boot_nonce`: `SEED`.
//! - `capability_token`: `Some(sha256(SEED|i))` when `i % 3 == 0`, else NULL.
//! - `kind`: `i % 5`; `origin`: `i % 2`.
//! - `intent`: one of 8 templates rotated by `i % 8`.
//! - `payload_redacted`: `64 + (i % 64)*64` bytes (64–4088 B) of repeating
//!   `sha256(SEED|i) || sha256(i|SEED)` blocks.
//!
//! The generator prints (stdout): `sha256`, `merkle_root`, `payload_oracle`,
//! `row_count` — for MANIFEST pinning.
//!
//! Run: `cargo run --release -p maos-loom-lite --example generate_migration_corpus -- --out /tmp/migration_corpus.sqlite`

use std::path::PathBuf;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

/// Fixed seed for deterministic generation. NEVER change without a MANIFEST update.
pub const SEED: u64 = 0x4A41_0D4A_A10E_CA7E;

/// Base timestamp: 2025-01-01T00:00:00Z in nanoseconds.
const BASE_TS_NS: i64 = 1_735_689_600_000_000_000;

/// Row count: exactly 10⁶ (100 batches at BATCH_SIZE=10_000).
const ROW_COUNT: u64 = 1_000_000;

/// Eight intent templates rotated by row index (matches MANIFEST description).
const INTENTS: &[&str] = &[
    "memory.write",
    "lifecycle.admit",
    "capability.issue",
    "iac.deliver",
    "distillate.write",
    "decision.halt",
    "log.recall",
    "mcp.invoke",
];

struct Args {
    out: PathBuf,
    rows: u64,
    print_root: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut out: Option<PathBuf> = None;
    let mut rows: u64 = ROW_COUNT;
    let mut print_root = true;
    let mut it = std::env::args().skip(1).peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out <path>")?)),
            "--rows" => {
                rows = it
                    .next()
                    .ok_or("--rows N")?
                    .parse()
                    .map_err(|e| format!("--rows: {e}"))?;
            }
            "--no-print-root" => print_root = false,
            other => return Err(format!("unknown arg: {other}")),
        }
    }
    Ok(Args {
        out: out.ok_or("missing --out <path>")?,
        rows,
        print_root,
    })
}

/// Production TL schema (11 columns).
const TL_SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
)";

fn main() {
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!("usage: generate_migration_corpus --out <path> [--rows N] [--no-print-root]");
        std::process::exit(2);
    });

    if args.out.exists() {
        std::fs::remove_file(&args.out).expect("failed to remove existing output");
    }

    let conn = Connection::open(&args.out).expect("failed to open sqlite db");
    conn.execute_batch(TL_SCHEMA)
        .expect("failed to create schema");

    let tx = conn.unchecked_transaction().expect("begin tx");
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO transparency_log
                   (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                    boot_nonce, capability_token, kind, intent, payload_redacted, origin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .expect("prepare insert");

        let seed_bytes = SEED.to_be_bytes();
        for i in 0..args.rows {
            let idx_bytes = i.to_be_bytes();

            let mut frame_id = [0u8; 16];
            frame_id[..8].copy_from_slice(&seed_bytes);
            frame_id[8..].copy_from_slice(&idx_bytes);

            let ts = BASE_TS_NS + i as i64;
            let pid = (i % 8) as i64;
            let from_spirit = format!("spirit-{}", i % 8);
            let to_spirit = "orchestrator".to_string();
            let boot_nonce = SEED as i64;

            let cap: Option<Vec<u8>> = if i % 3 == 0 {
                let mut h = Sha256::new();
                h.update(seed_bytes);
                h.update(idx_bytes);
                Some(h.finalize().to_vec())
            } else {
                None
            };

            let kind = (i % 5) as i64;
            let intent = INTENTS[(i % INTENTS.len() as u64) as usize];
            let origin = (i % 2) as i64;

            let mut hasher_a = Sha256::new();
            hasher_a.update(seed_bytes);
            hasher_a.update(idx_bytes);
            let hash_a: Vec<u8> = hasher_a.finalize().to_vec();
            let mut hasher_b = Sha256::new();
            hasher_b.update(idx_bytes);
            hasher_b.update(seed_bytes);
            let hash_b: Vec<u8> = hasher_b.finalize().to_vec();
            let block: Vec<u8> = hash_a.iter().chain(hash_b.iter()).copied().collect();
            let payload_len = 64 + ((i % 64) * 64) as usize;
            let payload: Vec<u8> = (0..payload_len).map(|p| block[p % block.len()]).collect();

            stmt.execute(rusqlite::params![
                &frame_id as &[u8],
                ts,
                pid,
                from_spirit,
                to_spirit,
                boot_nonce,
                &cap as &Option<Vec<u8>>,
                kind,
                intent,
                &payload as &[u8],
                origin,
            ])
            .expect("insert row");
        }
    }
    tx.commit().expect("commit tx");
    drop(conn);

    let count: i64 = Connection::open(&args.out)
        .and_then(|c| c.query_row("SELECT COUNT(*) FROM transparency_log", [], |r| r.get(0)))
        .expect("count");
    assert_eq!(count as u64, args.rows, "row count mismatch");
    eprintln!("generated {} rows at {}", args.rows, args.out.display());

    if args.print_root {
        let bytes = std::fs::read(&args.out).expect("read corpus");
        let sha = {
            let mut h = Sha256::new();
            h.update(&bytes);
            hex::encode(h.finalize())
        };
        println!("sha256 = {sha}");
        let root = maos_audit::backup::compute_merkle_root(&args.out).expect("compute merkle root");
        println!("merkle_root = {}", hex::encode(root));
        let frames = maos_loom_lite::canonical::read_sqlite_frames(&args.out)
            .expect("read canonical frames");
        let payload_oracle = maos_loom_lite::canonical::compute_payload_oracle(&frames);
        println!("payload_oracle = {}", hex::encode(payload_oracle));
        println!("row_count = {}", args.rows);
    }
}
