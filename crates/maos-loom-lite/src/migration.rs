#![forbid(unsafe_code)]

//! SQLite→Postgres migration engine (Story 10.4a AC2, NFR-Ops-10).
//!
//! Migrates a frozen (quiesced/snapshot) SQLite Transparency Log to Postgres,
//! with triple-oracle verification — Merkle root + payload oracle + row-count
//! oracle — all independently re-derived per backend.
//!
//! # Oracles (Murat's triple-oracle requirement, ratified §5)
//!
//! 1. **Merkle root** — `canonical::merkle_root_from_frame_ids` (mirrors
//!    `maos_audit::backup::compute_merkle_root`).  SET-only: proves frame_id
//!    set identity but is blind to payload corruption and dedup collapse.
//! 2. **Payload oracle** — `canonical::compute_payload_oracle`: SHA-256 over
//!    the sorted multiset of per-row canonical hashes (all 11 columns).  Catches
//!    any single-byte mutation the Merkle root misses.
//! 3. **Row-count oracle** — exact `COUNT(*)` equality.  Catches dedup collapse
//!    where frame_ids survive but rows merge.
//!
//! All three are RE-DERIVED from each backend's stored rows on read-back —
//! never co-computed once and written both places, never read from cached
//! metadata (the §5 self-reported-aggregate trap).
//!
//! # Source quiescence (B20)
//!
//! The SQLite source is opened READ-ONLY (`canonical::read_sqlite_frames`),
//! which blocks a concurrent writer from mutating the rows under inspection.
//! The pre-migration source root is captured BEFORE any target write so
//! rollback can prove source recoverability (B13).

use std::path::Path;
use std::time::Duration;

use crate::canonical::{self, CanonicalFrame};

/// Batch size for migration inserts — bounds in-memory Vec sizes and forces
/// the >1-batch-boundary path (AC2 proven-red).
pub const BATCH_SIZE: usize = 10_000;

/// Migration result carrying the three independently-re-derived oracles.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub source_merkle_root: [u8; 32],
    pub target_merkle_root: [u8; 32],
    pub source_payload_oracle: [u8; 32],
    pub target_payload_oracle: [u8; 32],
    pub source_row_count: u64,
    pub target_row_count: u64,
    /// Pre-migration SQLite source root — captured BEFORE any target writes.
    /// Rollback proves source recoverability by re-deriving the root and
    /// comparing it to this value (B13).
    pub pre_migration_source_root: [u8; 32],
}

impl MigrationResult {
    /// Verify all three oracles pass.  Returns a typed error identifying which
    /// oracle failed (never a silent PASS).
    pub fn verify(&self) -> Result<(), MigrationError> {
        // Cheapest gate first (fail-fast): if the row counts differ, the sets
        // cannot match — no point computing the roots.
        if self.source_row_count != self.target_row_count {
            return Err(MigrationError::RowCountMismatch {
                source_count: self.source_row_count,
                target_count: self.target_row_count,
            });
        }
        if self.source_merkle_root != self.target_merkle_root {
            return Err(MigrationError::MerkleRootMismatch {
                source_root: hex::encode(self.source_merkle_root),
                target_root: hex::encode(self.target_merkle_root),
            });
        }
        if self.source_payload_oracle != self.target_payload_oracle {
            return Err(MigrationError::PayloadOracleMismatch {
                source_oracle: hex::encode(self.source_payload_oracle),
                target_oracle: hex::encode(self.target_payload_oracle),
            });
        }
        Ok(())
    }
}

/// Migration error type.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("source (SQLite) error: {0}")]
    Source(String),
    #[error("target (Postgres) error: {0}")]
    Target(String),
    #[error("rollback error: {0}")]
    Rollback(String),
    #[error("merkle-root mismatch — source={source_root} target={target_root}")]
    MerkleRootMismatch {
        source_root: String,
        target_root: String,
    },
    #[error("payload-oracle mismatch — source={source_oracle} target={target_oracle} (Merkle root alone is insufficient)")]
    PayloadOracleMismatch {
        source_oracle: String,
        target_oracle: String,
    },
    #[error("row-count mismatch — source={source_count} target={target_count} (dedup collapse detected)")]
    RowCountMismatch {
        source_count: u64,
        target_count: u64,
    },
}
/// The production Transparency-Log schema mirrored to Postgres.
///
/// `frame_id BYTEA PRIMARY KEY` is valid on PostgreSQL 17 (the `bytea_ops`
/// btree operator class is present — empirically verified, 2026-06-22) and is
/// the most faithful mirror of SQLite's `BLOB PRIMARY KEY`.  `ORDER BY
/// frame_id` is supported by the same opclass.  All 11 production columns are
/// preserved (B1); `capability_token` is the only nullable column.
pub const POSTGRES_TL_SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BYTEA    NOT NULL PRIMARY KEY,
    timestamp_ns        BIGINT   NOT NULL,
    spirit_pid          BIGINT   NOT NULL,
    from_spirit_id      TEXT     NOT NULL DEFAULT '',
    to_spirit_id        TEXT     NOT NULL DEFAULT '',
    boot_nonce          BIGINT   NOT NULL,
    capability_token    BYTEA,
    kind                BIGINT   NOT NULL,
    intent              TEXT     NOT NULL,
    payload_redacted    BYTEA    NOT NULL,
    origin              BIGINT   NOT NULL
)";

/// Create the Transparency Log table in Postgres (migration target).
pub async fn create_postgres_tl_schema(
    client: &tokio_postgres::Client,
) -> Result<(), MigrationError> {
    client
        .batch_execute(POSTGRES_TL_SCHEMA)
        .await
        .map_err(|e| MigrationError::Target(e.to_string()))
}

/// Drop the Postgres TL table (for rollback / clean teardown).
pub async fn drop_postgres_tl(client: &tokio_postgres::Client) -> Result<(), MigrationError> {
    client
        .batch_execute("DROP TABLE IF EXISTS transparency_log")
        .await
        .map_err(|e| MigrationError::Rollback(e.to_string()))
}

/// Run the forward migration: capture the pre-migration source root, read all
/// rows from a read-only SQLite source, insert into Postgres inside a single
/// transaction, then verify all three oracles.
pub async fn migrate_sqlite_to_postgres(
    sqlite_path: &Path,
    pg_client: &mut tokio_postgres::Client,
) -> Result<MigrationResult, MigrationError> {
    // Phase 1 — read the (read-only) source frames ONCE.  P13: the pre-migration
    // source root below is derived from THESE in-memory frames (not a second
    // open of the SQLite file), so a concurrent writer between a separate
    // Phase 0 root read and the frame read cannot go undetected — the snapshot
    // is exactly what was migrated.
    let source_frames = canonical::read_sqlite_frames(sqlite_path).map_err(MigrationError::Source)?;
    let (source_merkle_root, source_payload_oracle, source_row_count) = derive_source_oracles(&source_frames);

    // Phase 0 (P13) — pre-migration source root captured BEFORE any target
    // write (B13 — non-tautological rollback snapshot).  Computed from the same
    // frames read above via the canonical primitive, so the snapshot is
    // consistent with what is actually migrated.  `merkle_root_from_frame_ids`
    // mirrors `maos_audit::backup::compute_merkle_root` (canonical.rs:137),
    // hence rollback's `compute_merkle_root(path)` cross-check still matches a
    // quiesced source; the value is identical to `source_merkle_root`.
    let pre_migration_source_root = source_merkle_root;

    // Phase 2 — create the target schema + insert all rows inside ONE
    // transaction (B12: atomic cutover; a mid-migration failure rolls the
    // whole target back so no partial commit survives).  A duplicate frame_id
    // errors rather than silently merging (B16).
    let txn = pg_client
        .transaction()
        .await
        .map_err(|e| MigrationError::Target(format!("begin: {e}")))?;
    txn.batch_execute(POSTGRES_TL_SCHEMA)
        .await
        .map_err(|e| MigrationError::Target(format!("schema: {e}")))?;
    insert_frames(&txn, &source_frames).await.map_err(MigrationError::Target)?;

    // Phase 3 (P4) — re-derive the target oracles from the UNCOMMITTED
    // transaction rows, then verify BEFORE committing.  Reading through the
    // transaction (not the bare `Client`) is what makes the uncommitted inserts
    // visible; if any oracle fails, `?` returns early and the dropped `txn`
    // auto-rolls-back — no partial commit ever survives a verification failure.
    let (target_merkle_root, target_payload_oracle, target_row_count) =
        derive_target_oracles(&txn).await?;

    let result = MigrationResult {
        source_merkle_root,
        target_merkle_root,
        source_payload_oracle,
        target_payload_oracle,
        source_row_count,
        target_row_count,
        pre_migration_source_root,
    };
    result.verify()?;

    txn.commit()
        .await
        .map_err(|e| MigrationError::Target(format!("commit: {e}")))?;
    Ok(result)
}

/// Independently re-derive all three oracles from BOTH backends (B9).  This is
/// the standalone cross-check the ship gate invokes; it never trusts a cached
/// or co-computed value.
pub async fn verify_migration_integrity(
    sqlite_path: &Path,
    pg_client: &tokio_postgres::Client,
) -> Result<MigrationResult, MigrationError> {
    let source_frames = canonical::read_sqlite_frames(sqlite_path).map_err(MigrationError::Source)?;
    let (source_merkle_root, source_payload_oracle, source_row_count) = derive_source_oracles(&source_frames);

    // P13 — pre-migration source root derived from the same in-memory frames
    // (single read); identical to `source_merkle_root`.  `merkle_root_from_frame_ids`
    // mirrors `compute_merkle_root`, so a quiesced source round-trips.
    let pre_migration_source_root = source_merkle_root;

    let (target_merkle_root, target_payload_oracle, target_row_count) =
        derive_target_oracles(pg_client).await?;

    let result = MigrationResult {
        source_merkle_root,
        target_merkle_root,
        source_payload_oracle,
        target_payload_oracle,
        source_row_count,
        target_row_count,
        pre_migration_source_root,
    };
    result.verify()?;
    Ok(result)
}

/// Connect to Postgres via a connection string, drop any stale target, then run
/// the forward migration + triple-oracle verification.  Convenience for the
/// CLI subcommand and the `check-migration-merkle` gate (keeps `tokio_postgres`
/// out of the gate's own dependency surface).  Re-derives BOTH backends.
pub async fn migrate_with_conn_str(
    sqlite_path: &Path,
    conn_str: &str,
) -> Result<MigrationResult, MigrationError> {
    use tokio_postgres::NoTls;
    // P7 — bound the connect (an unreachable host must not hang the gate
    // forever).  Mirrors the CLI path's connect timeout (dispatch_migrate).
    let (mut client, conn) = match tokio::time::timeout(
        Duration::from_secs(30),
        tokio_postgres::connect(conn_str, NoTls),
    )
    .await
    {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => return Err(MigrationError::Target(format!("connect: {e}"))),
        Err(_) => {
            return Err(MigrationError::Target(
                "timed out connecting to Postgres after 30s".to_string(),
            ))
        }
    };
    tokio::spawn(async move {
        let _ = conn.await;
    });
    // P7 — bound per-statement execution (matches dispatch_migrate's 60s
    // statement_timeout) so a wedged server fails fast instead of hanging.
    client
        .batch_execute("SET statement_timeout = 60000")
        .await
        .map_err(|e| MigrationError::Target(format!("statement_timeout: {e}")))?;
    // WARNING (P16): `DROP TABLE IF EXISTS transparency_log` is unconditional.
    // Two migration runs (or two `check-migration-merkle` gate processes)
    // targeting the SAME Postgres database/schema will race — one run's DROP
    // can tear down the other run's in-flight target, corrupting its
    // verification.  Concurrent runs MUST use separate Postgres databases (or
    // distinct schemas).  The xtask gate is serialized by a process-level
    // PG_LOCK in tests, but cross-process / CI concurrency is NOT protected
    // here; isolate at the database/schema level until the target table is
    // parameterized by a run id (tracked future work).
    client
        .batch_execute("DROP TABLE IF EXISTS transparency_log")
        .await
        .map_err(|e| MigrationError::Target(format!("reset: {e}")))?;
    migrate_sqlite_to_postgres(sqlite_path, &mut client).await
}

/// Rollback: drop the Postgres target and verify the SQLite source root still
/// matches the pre-migration snapshot (B13 — non-tautological because the
/// snapshot was captured BEFORE any target write).
pub async fn rollback_migration(
    sqlite_path: &Path,
    pg_client: &tokio_postgres::Client,
    expected_source_root: [u8; 32],
) -> Result<(), MigrationError> {
    drop_postgres_tl(pg_client).await?;

    // Verify Postgres target is gone.
    let exists = pg_client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'transparency_log')",
            &[],
        )
        .await
        .map_err(|e| MigrationError::Rollback(e.to_string()))?;
    let table_exists: bool = exists.get(0);
    if table_exists {
        return Err(MigrationError::Rollback(
            "transparency_log table still exists after DROP".to_string(),
        ));
    }

    // Verify the source root is still recoverable AND unchanged.
    let source_root = maos_audit::backup::compute_merkle_root(sqlite_path)
        .map_err(|e| MigrationError::Rollback(format!("source root check: {e}")))?;
    if source_root != expected_source_root {
        return Err(MigrationError::Rollback(format!(
            "source root changed: expected={}, got={}",
            hex::encode(expected_source_root),
            hex::encode(source_root)
        )));
    }
    Ok(())
}

/// Compute the three source (SQLite-side) oracles from canonical frames.
fn derive_source_oracles(
    source_frames: &[CanonicalFrame],
) -> ([u8; 32], [u8; 32], u64) {
    let frame_ids: Vec<[u8; 16]> = source_frames.iter().map(|f| f.frame_id).collect();
    let root = canonical::merkle_root_from_frame_ids(&frame_ids);
    let payload = canonical::compute_payload_oracle(source_frames);
    (root, payload, source_frames.len() as u64)
}

/// Independently re-derive the three target (Postgres-side) oracles by
/// reading rows back from Postgres.
async fn derive_target_oracles<C>(
    pg_client: &C,
) -> Result<([u8; 32], [u8; 32], u64), MigrationError>
where
    C: tokio_postgres::GenericClient + Sync,
{
    let target_frames = canonical::read_postgres_frames(pg_client)
        .await
        .map_err(MigrationError::Target)?;
    let frame_ids: Vec<[u8; 16]> = target_frames.iter().map(|f| f.frame_id).collect();
    let root = canonical::merkle_root_from_frame_ids(&frame_ids);
    let payload = canonical::compute_payload_oracle(&target_frames);
    Ok((root, payload, target_frames.len() as u64))
}

/// Insert all frames into Postgres inside the given transaction.
///
/// A single prepared statement is reused across all rows (B18); rows are
/// chunked at `BATCH_SIZE` so the >1-batch-boundary path is exercised.  Empty
/// text/payload columns map to `''`/empty BYTEA, never NULL (B10); non-UTF-8
/// intent is rejected (B11); a duplicate frame_id errors (B16, no ON CONFLICT).
async fn insert_frames(
    txn: &tokio_postgres::Transaction<'_>,
    frames: &[CanonicalFrame],
) -> Result<(), String> {
    let stmt = txn
        .prepare(
            "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                 boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .await
        .map_err(|e| format!("prepare: {e}"))?;

    for chunk in frames.chunks(BATCH_SIZE) {
        for frame in chunk {
            let from_spirit_id = vec_u8_to_str(&frame.from_spirit_id)
                .map_err(|e| format!("from_spirit_id non-utf8: {e}"))?;
            let to_spirit_id = vec_u8_to_str(&frame.to_spirit_id)
                .map_err(|e| format!("to_spirit_id non-utf8: {e}"))?;
            let intent = vec_u8_to_str(&frame.intent)
                .map_err(|e| format!("intent non-utf8: {e}"))?;
            let cap: Option<&[u8]> = frame.capability_token.as_deref();

            txn.execute(
                &stmt,
                &[
                    &frame.frame_id.as_slice(),
                    &(frame.timestamp_ns as i64),
                    &(frame.spirit_pid as i64),
                    &from_spirit_id,
                    &to_spirit_id,
                    &(frame.boot_nonce as i64),
                    &cap,
                    &frame.kind,
                    &intent,
                    &frame.payload.as_slice(),
                    &frame.origin,
                ],
            )
            .await
            .map_err(|e| format!("insert frame: {e}"))?;
        }
    }
    Ok(())
}

/// Strict UTF-8 conversion (B11 — reject, never mask).
fn vec_u8_to_str(bytes: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    // Live-Postgres integration tests live in `tests/migration_live.rs` so they
    // can be gated on a real Postgres connection (MAOS_TEST_POSTGRES).  The
    // pure-oracle unit tests live in `canonical.rs`.
}
