#![forbid(unsafe_code)]

//! Story 9.4 — Transparency Log backup/DR: read-only Merkle cross-check,
//! RPO arithmetic oracle, and latest-timestamp helper.
//!
//! The write-side backup API lives in `maos-cli::backup` because `maos-audit`
//! is read-only by design (`SQLITE_OPEN_READ_ONLY`). The read-only functions
//! here are used by both `maosctl backup verify` (via cold restore) and the
//! air-gap binary.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Typed backup/DR error.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("merkle root mismatch: backup={backup_root}, source={source_root}")]
    MerkleRootMismatch {
        source_root: String,
        backup_root: String,
    },
    #[error("backup integrity failure: {0}")]
    IntegrityFailure(String),
    #[error("RPO violation: gap={gap_ns}ns exceeds threshold={threshold_ns}ns")]
    RpoViolation { gap_ns: u64, threshold_ns: u64 },
}

/// Compute the Merkle root from all frame_ids in a TL database.
///
/// Opens the DB read-only, reads all `frame_id` values from the
/// `transparency_log` table, builds the Merkle tree, returns the root.
///
/// Returns an error if any `frame_id` blob is not exactly 16 bytes (instead of
/// panicking).
pub fn compute_merkle_root(db_path: &Path) -> Result<[u8; 32], BackupError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let mut stmt = conn.prepare("SELECT frame_id FROM transparency_log ORDER BY frame_id")?;
    let frame_ids: Vec<[u8; 16]> = stmt
        .query_map([], |row| {
            let blob: Vec<u8> = row.get(0)?;
            if blob.len() != 16 {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "frame_id must be 16 bytes, got {}",
                    blob.len()
                )));
            }
            let mut fid = [0u8; 16];
            fid.copy_from_slice(&blob);
            Ok(fid)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if frame_ids.is_empty() {
        return Ok([0u8; 32]); // empty tree root
    }
    let tree = crate::erasure::merkle::build_tree_from_frame_ids(&frame_ids);
    Ok(tree.root)
}

/// Verify backup integrity via Merkle root cross-check (R-DR1).
///
/// Recomputes the Merkle root of the backup independently and byte-compares
/// against the source root. This is a read-only check; the cold-restore path
/// in `maos-cli::backup` performs an independent restore before calling this
/// function.
pub fn verify_backup_integrity(source_path: &Path, backup_path: &Path) -> Result<(), BackupError> {
    let source_root = compute_merkle_root(source_path)?;
    let backup_root = compute_merkle_root(backup_path)?;
    if source_root != backup_root {
        return Err(BackupError::MerkleRootMismatch {
            source_root: hex::encode(source_root),
            backup_root: hex::encode(backup_root),
        });
    }
    Ok(())
}

/// RPO arithmetic oracle (R-DR3): verify that the gap between the last
/// backup timestamp and the crash point is within the RPO threshold.
///
/// Uses synthetic monotonic timestamps (not wall-clock) — the caller
/// provides `last_backup_timestamp_ns` and `crash_point_ns`.
pub fn verify_rpo(
    last_backup_timestamp_ns: u64,
    crash_point_ns: u64,
    rpo_threshold_ns: u64,
) -> Result<(), BackupError> {
    if crash_point_ns <= last_backup_timestamp_ns {
        return Ok(()); // crash before or at backup — no gap
    }
    let gap = crash_point_ns - last_backup_timestamp_ns;
    if gap > rpo_threshold_ns {
        return Err(BackupError::RpoViolation {
            gap_ns: gap,
            threshold_ns: rpo_threshold_ns,
        });
    }
    Ok(())
}

/// Read the latest `timestamp_ns` from the Transparency Log.
pub fn latest_timestamp(db_path: &Path) -> Result<Option<u64>, BackupError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let ts: Option<u64> = conn.query_row(
        "SELECT MAX(timestamp_ns) FROM transparency_log",
        [],
        |row| row.get(0),
    )?;
    Ok(ts)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests — R-DR2: three distinct corruption-bite reds + RPO oracle
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Transparency Log schema — mirrors `maos-iac` SCHEMA_SQL (read-side
    /// cannot depend on `maos-iac`).
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
);";

    /// Insert a synthetic TL row with a given frame_id (16 bytes).
    fn insert_frame(conn: &Connection, frame_id: &[u8; 16], ts_ns: u64) {
        conn.execute(
            "INSERT INTO transparency_log \
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, 1, 1, 1, 'test', X'00', 0)",
            rusqlite::params![&frame_id[..], ts_ns],
        )
        .expect("insert frame");
    }

    /// Create a TL database with `n` synthetic frames, returning the path.
    fn create_source_tl(dir: &TempDir, name: &str, n: usize) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(TL_SCHEMA).unwrap();
        for i in 0..n {
            let mut fid = [0u8; 16];
            fid[0..8].copy_from_slice(&(i as u64).to_le_bytes());
            insert_frame(&conn, &fid, (i as u64 + 1) * 1_000_000);
        }
        path
    }

    /// Test-only backup helper: uses the same SQLite backup API as the
    /// production `maos-cli::backup::backup_transparency_log`, but lives inside
    /// `maos-audit` tests so the read-only crate can verify its own read-only
    /// functions without taking a production dependency on `maos-cli`.
    fn backup_for_test(source_path: &Path, dest_path: &Path) {
        let src = Connection::open_with_flags(
            source_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        let mut dst = Connection::open(dest_path).unwrap();
        let backup = rusqlite::backup::Backup::new(&src, &mut dst).unwrap();
        backup
            .run_to_completion(100, std::time::Duration::from_millis(250), None)
            .unwrap();
    }

    // ── R-DR1: backup + Merkle cross-check ──────────────────────────────

    #[test]
    fn backup_and_verify_round_trip() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 10);
        let backup = dir.path().join("backup.sqlite");

        backup_for_test(&source, &backup);
        verify_backup_integrity(&source, &backup).unwrap();
    }

    #[test]
    fn verify_empty_tl() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 0);
        let backup = dir.path().join("backup.sqlite");

        backup_for_test(&source, &backup);
        verify_backup_integrity(&source, &backup).unwrap();
    }

    // ── R-DR2: three distinct corruption-bite reds ──────────────────────

    /// R-DR2 red 1: flip a byte in a frame_id → Merkle root changes →
    /// `MerkleRootMismatch`.
    #[test]
    fn corruption_bite_frame_id_byte_flip() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 5);
        let backup_path = dir.path().join("backup.sqlite");
        backup_for_test(&source, &backup_path);

        // Corrupt: flip a byte in the first frame_id
        {
            let conn = Connection::open(&backup_path).unwrap();
            let mut fid: Vec<u8> = conn
                .query_row(
                    "SELECT frame_id FROM transparency_log ORDER BY frame_id LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            fid[0] ^= 0xFF; // flip all bits in first byte
            conn.execute(
                "UPDATE transparency_log SET frame_id = ?1 \
                 WHERE rowid = (SELECT rowid FROM transparency_log ORDER BY frame_id LIMIT 1)",
                rusqlite::params![&fid[..]],
            )
            .unwrap();
        }

        let err = verify_backup_integrity(&source, &backup_path).unwrap_err();
        assert!(
            matches!(err, BackupError::MerkleRootMismatch { .. }),
            "expected MerkleRootMismatch, got: {err}"
        );
    }

    /// R-DR2 red 2: insert an extra frame into the backup → the Merkle tree
    /// gains an additional leaf → root changes → `MerkleRootMismatch`.
    /// Distinct from red 1 (mutate existing frame_id) and red 3 (delete frames).
    #[test]
    fn corruption_bite_inserted_frame() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 5);
        let backup_path = dir.path().join("backup.sqlite");
        backup_for_test(&source, &backup_path);

        // Corrupt: insert a new synthetic frame with a frame_id outside the
        // original range so the sorted set of frame_ids changes.
        {
            let conn = Connection::open(&backup_path).unwrap();
            let mut fid = [0u8; 16];
            fid[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
            insert_frame(&conn, &fid, 99_000_000);
        }

        let err = verify_backup_integrity(&source, &backup_path).unwrap_err();
        assert!(
            matches!(err, BackupError::MerkleRootMismatch { .. }),
            "expected MerkleRootMismatch, got: {err}"
        );
    }

    /// R-DR2 red 3: delete frames from the backup → tree structure changes
    /// (fewer leaves) → Merkle root changes → `MerkleRootMismatch`.
    #[test]
    fn corruption_bite_truncation() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 5);
        let backup_path = dir.path().join("backup.sqlite");
        backup_for_test(&source, &backup_path);

        // Corrupt: delete the last 2 frames
        {
            let conn = Connection::open(&backup_path).unwrap();
            conn.execute(
                "DELETE FROM transparency_log WHERE rowid IN \
                 (SELECT rowid FROM transparency_log ORDER BY frame_id DESC LIMIT 2)",
                [],
            )
            .unwrap();
        }

        let err = verify_backup_integrity(&source, &backup_path).unwrap_err();
        assert!(
            matches!(err, BackupError::MerkleRootMismatch { .. }),
            "expected MerkleRootMismatch, got: {err}"
        );
    }

    /// R-DR2 red 4: corrupt a frame_id blob length → `compute_merkle_root`
    /// returns an error instead of panicking.
    #[test]
    fn corruption_bite_bad_frame_id_length_rejected() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 5);
        let backup_path = dir.path().join("backup.sqlite");
        backup_for_test(&source, &backup_path);

        {
            let conn = Connection::open(&backup_path).unwrap();
            let rowid: i64 = conn
                .query_row(
                    "SELECT rowid FROM transparency_log ORDER BY frame_id LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let bad_fid = vec![0u8; 8]; // too short
            conn.execute(
                "UPDATE transparency_log SET frame_id = ?1 WHERE rowid = ?2",
                rusqlite::params![&bad_fid[..], rowid],
            )
            .unwrap();
        }

        let err = compute_merkle_root(&backup_path).unwrap_err();
        assert!(
            matches!(
                err,
                BackupError::Sqlite(rusqlite::Error::InvalidParameterName(_))
            ),
            "expected InvalidParameterName, got: {err}"
        );
    }

    // ── R-DR3: RPO arithmetic oracle ────────────────────────────────────

    #[test]
    fn rpo_within_threshold() {
        let backup_ts = 1_000_000_000u64;
        let crash_ts = 1_500_000_000u64;
        let threshold = 1_000_000_000u64; // 1s
        verify_rpo(backup_ts, crash_ts, threshold).unwrap();
    }

    #[test]
    fn rpo_exactly_at_threshold() {
        let backup_ts = 1_000_000_000u64;
        let crash_ts = 2_000_000_000u64;
        let threshold = 1_000_000_000u64;
        verify_rpo(backup_ts, crash_ts, threshold).unwrap();
    }

    #[test]
    fn rpo_exceeds_threshold() {
        let backup_ts = 1_000_000_000u64;
        let crash_ts = 3_000_000_001u64;
        let threshold = 1_000_000_000u64; // 1s
        let err = verify_rpo(backup_ts, crash_ts, threshold).unwrap_err();
        match err {
            BackupError::RpoViolation {
                gap_ns,
                threshold_ns,
            } => {
                assert_eq!(gap_ns, 2_000_000_001);
                assert_eq!(threshold_ns, 1_000_000_000);
            }
            _ => panic!("expected RpoViolation, got: {err}"),
        }
    }

    #[test]
    fn rpo_one_hour_threshold_passes() {
        // AC-3 requires RPO ≤ 1 hour. Exercise the contract at the real scale.
        let backup_ts = 0u64;
        let crash_ts = 3_600_000_000_000u64; // 1 hour in nanoseconds
        let threshold = 3_600_000_000_000u64; // 1 hour
        verify_rpo(backup_ts, crash_ts, threshold).unwrap();
    }

    #[test]
    fn rpo_one_hour_threshold_fails_when_exceeded() {
        let backup_ts = 0u64;
        let crash_ts = 3_600_000_000_001u64; // 1 hour + 1 ns
        let threshold = 3_600_000_000_000u64; // 1 hour
        let err = verify_rpo(backup_ts, crash_ts, threshold).unwrap_err();
        match err {
            BackupError::RpoViolation {
                gap_ns,
                threshold_ns,
            } => {
                assert_eq!(gap_ns, 3_600_000_000_001);
                assert_eq!(threshold_ns, 3_600_000_000_000);
            }
            _ => panic!("expected RpoViolation, got: {err}"),
        }
    }

    #[test]
    fn rpo_crash_before_backup() {
        // crash_point <= last_backup — no gap
        verify_rpo(5_000_000_000, 3_000_000_000, 1_000_000_000).unwrap();
    }

    #[test]
    fn rpo_crash_at_backup() {
        verify_rpo(5_000_000_000, 5_000_000_000, 0).unwrap();
    }

    // ── latest_timestamp ────────────────────────────────────────────────

    #[test]
    fn latest_timestamp_populated() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 3);
        let ts = latest_timestamp(&source).unwrap();
        assert_eq!(ts, Some(3_000_000)); // frame 2 → ts = 3_000_000
    }

    #[test]
    fn latest_timestamp_empty() {
        let dir = TempDir::new().unwrap();
        let source = create_source_tl(&dir, "source.sqlite", 0);
        let ts = latest_timestamp(&source).unwrap();
        assert_eq!(ts, None);
    }
}
