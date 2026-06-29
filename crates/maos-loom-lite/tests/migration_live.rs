#![forbid(unsafe_code)]

//! Live-Postgres integration + proven-red for the SQLite→Postgres migration
//! engine (Story 10.4a AC2, NFR-Ops-10).
//!
//! These tests drive the REAL engine against a live PostgreSQL instance.  They
//! are `#[ignore]`-gated on the `MAOS_TEST_POSTGRES` connection string so that
//! an environment without Postgres reports them as *ignored* (never a silent
//! pass — Winston 10.2 verdict axis).  Run them with a live backend:
//!
//! ```text
//! MAOS_TEST_POSTGRES="host=127.0.0.1 user=maos_test password=maos_test dbname=maos_test" \
//!   cargo test -p maos-loom-lite --test migration_live -- --ignored --nocapture
//! ```
//!
//! Every vector exercises the actual `migrate_sqlite_to_postgres` /
//! `verify_migration_integrity` / `rollback_migration` surface — no mocked
//! struct-literals (B5).  The forward-migration, the triple-oracle
//! re-derivation, and the rollback teardown all run end-to-end against a real
//! Postgres (B6).

use std::sync::Mutex;

use maos_loom_lite::migration::{
    migrate_sqlite_to_postgres, rollback_migration, verify_migration_integrity, MigrationError,
};
use rusqlite::Connection;
use tempfile::TempDir;
use tokio_postgres::NoTls;

/// Serialize tests on the shared Postgres instance (single `transparency_log`
/// table).  Acquired for the full body of each test.
static PG_LOCK: Mutex<()> = Mutex::new(());

/// Production TL schema (11 columns) — mirrors
/// `maos-iac/src/adapter/transparency_log.rs:246-258`.
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

/// Deterministic 16-byte frame_id from an index (content-addressed).
fn frame_id(i: u64) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"maos.10.4a.corpus");
    h.update(i.to_be_bytes());
    let digest = h.finalize();
    let mut fid = [0u8; 16];
    fid.copy_from_slice(&digest[..16]);
    fid
}

/// Build a deterministic SQLite TL with `n` rows in `tmpdir`.
fn build_sqlite_source(dir: &TempDir, n: u64) -> std::path::PathBuf {
    let path = dir.path().join("source.db");
    let conn = Connection::open(&path).expect("open sqlite");
    conn.execute_batch(TL_SCHEMA).expect("create schema");
    for i in 0..n {
        conn.execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                frame_id(i).as_slice(),
                1_700_000_000_000i64 + i as i64,
                42i64 + (i % 10) as i64,
                "spirit-a",
                "spirit-b",
                99i64,
                if i % 3 == 0 {
                    Some([0xAAu8; 32].as_slice())
                } else {
                    None::<&[u8]>
                },
                (i % 5) as i64,
                "memory.write",
                format!("payload-{i}-body").as_bytes(),
                (i % 2) as i64,
            ],
        )
        .expect("insert");
    }
    drop(conn);
    path
}

async fn connect() -> tokio_postgres::Client {
    let conn_str = std::env::var("MAOS_TEST_POSTGRES")
        .expect("MAOS_TEST_POSTGRES set (test is #[ignore]-gated)");
    let (client, conn) = tokio_postgres::connect(&conn_str, NoTls)
        .await
        .expect("connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
}

/// Ensure the target table is absent before a run.
async fn reset(client: &tokio_postgres::Client) {
    client
        .batch_execute("DROP TABLE IF EXISTS transparency_log")
        .await
        .expect("reset");
}

fn guard() -> std::sync::MutexGuard<'static, ()> {
    PG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn green_faithful_migration_triple_oracle_passes() {
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 500);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");
    result.verify().expect("triple oracle GREEN");
    assert_eq!(result.source_row_count, 500);
    assert_eq!(result.target_row_count, 500);
    assert_eq!(result.source_merkle_root, result.target_merkle_root);
    assert_eq!(result.source_payload_oracle, result.target_payload_oracle);
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn red_corrupt_payload_byte_root_matches_payload_oracle_fails() {
    // The AC2-mandated vector: corrupt ONE payload byte in the target after
    // migration.  The frame_id set is intact → Merkle root still matches; the
    // payload oracle catches it (proving the Merkle root alone is insufficient).
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 300);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");
    assert_eq!(result.source_merkle_root, result.target_merkle_root);

    // Corrupt one payload byte in Postgres.
    let target_fid = frame_id(7);
    client
        .execute(
            "UPDATE transparency_log SET payload_redacted = $1 WHERE frame_id = $2",
            &[
                &b"tampered-payload-CORRUPT".as_slice(),
                &target_fid.as_slice(),
            ],
        )
        .await
        .expect("corrupt");

    let err = verify_migration_integrity(&src, &client)
        .await
        .expect_err("must RED");
    assert!(
        matches!(err, MigrationError::PayloadOracleMismatch { .. }),
        "expected PayloadOracleMismatch, got {err:?}"
    );
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn red_alter_one_frame_id_merkle_mismatch() {
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 200);

    let mut client = connect().await;
    reset(&client).await;
    let _ = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");

    // Remove one frame_id and insert a different one → set changes → Merkle mismatch.
    let old = frame_id(3);
    let mut newid = old;
    newid[0] ^= 0xFF;
    client
        .execute(
            "DELETE FROM transparency_log WHERE frame_id = $1",
            &[&old.as_slice()],
        )
        .await
        .expect("delete");
    client
        .execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            &[
                &newid.as_slice(),
                &1i64,
                &1i64,
                &"x",
                &"y",
                &1i64,
                &None::<&[u8]>,
                &0i32,
                &"z",
                &[0u8].as_slice(),
                &0i32,
            ],
        )
        .await
        .expect("insert alt");

    let err = verify_migration_integrity(&src, &client)
        .await
        .expect_err("must RED");
    assert!(matches!(err, MigrationError::MerkleRootMismatch { .. }));
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn red_delete_target_row_count_mismatch() {
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 150);

    let mut client = connect().await;
    reset(&client).await;
    let _ = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");

    client
        .execute(
            "DELETE FROM transparency_log WHERE frame_id = $1",
            &[&frame_id(10).as_slice()],
        )
        .await
        .expect("delete");

    let err = verify_migration_integrity(&src, &client)
        .await
        .expect_err("must RED");
    assert!(matches!(err, MigrationError::RowCountMismatch { .. }));
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn rollback_restores_source_and_tears_down_target() {
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 100);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");

    rollback_migration(&src, &client, result.pre_migration_source_root)
        .await
        .expect("rollback GREEN");

    // Target must be gone.
    let exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name='transparency_log')",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert!(!exists, "target table must be torn down");

    // Source root must still match the pre-migration snapshot (B13, non-tautological).
    let src_root = maos_audit::backup::compute_merkle_root(&src).expect("source root");
    assert_eq!(src_root, result.pre_migration_source_root);
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn rollback_red_when_source_changed() {
    // B13: rollback must FAIL if the source root no longer matches the
    // pre-migration snapshot (non-tautological check).
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 50);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN migrate");

    // Mutate the source AFTER migration so the snapshot no longer matches.
    {
        let conn = Connection::open(&src).unwrap();
        conn.execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                frame_id(99_999).as_slice(),
                9999i64,
                1i64,
                "x",
                "y",
                1i64,
                None::<&[u8]>,
                0i64,
                "z",
                b"extra".as_ref(),
                0i64,
            ],
        )
        .unwrap();
    }

    let err = rollback_migration(&src, &client, result.pre_migration_source_root)
        .await
        .expect_err("must RED — source changed");
    assert!(matches!(err, MigrationError::Rollback { .. }));
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn empty_corpus_green_zero_root_both_sides() {
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 0);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN empty migrate");
    assert_eq!(result.source_row_count, 0);
    assert_eq!(result.target_row_count, 0);
    // B14: empty sentinel is [0u8;32] on both sides.
    assert_eq!(result.source_merkle_root, [0u8; 32]);
    assert_eq!(result.target_merkle_root, [0u8; 32]);
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn green_across_multiple_batch_boundaries() {
    // AC2 proven-red: >1 batch boundary.  25_001 rows / BATCH_SIZE(10_000) = 3 batches.
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let src = build_sqlite_source(&dir, 25_001);

    let mut client = connect().await;
    reset(&client).await;
    let result = migrate_sqlite_to_postgres(&src, &mut client)
        .await
        .expect("GREEN multi-batch");
    result.verify().expect("triple oracle GREEN across batches");
    assert_eq!(result.target_row_count, 25_001);
}

#[tokio::test]
#[ignore = "requires live Postgres (set MAOS_TEST_POSTGRES)"]
async fn red_duplicate_frame_id_rejected_by_pk() {
    // B16: under BYTEA PK + no ON CONFLICT, a duplicate frame_id in the SOURCE
    // never reaches Postgres (SQLite PK rejects it first).  Verify the source
    // builder itself rejects a dup, proving the collapse path is unreachable.
    let _g = guard();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dup.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(TL_SCHEMA).unwrap();
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
         VALUES (?1,1,1,'a','b',1,NULL,0,'i',x'00',0)",
        rusqlite::params![frame_id(1).as_slice()],
    )
    .unwrap();
    let dup = conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
         VALUES (?1,2,2,'c','d',2,NULL,1,'j',x'01',1)",
        rusqlite::params![frame_id(1).as_slice()],
    );
    assert!(dup.is_err(), "SQLite PK must reject the duplicate frame_id");
}
