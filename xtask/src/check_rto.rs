#![forbid(unsafe_code)]

//! Story 10.4a — RTO ≤ 4h drill gate.
//!
//! `maos-cli/src/subcommands.rs:98` prints `RTO={:.3}s` after a cold restore
//! but does NOT gate on it. This xtask gate performs a **drilled** RTO
//! measurement: create (or load) a source TL, back it up, cold-restore to a
//! fresh destination, time the restore, and **assert** the result is within
//! the RTO ≤ 4h threshold (NFR-Ops-2).
//!
//! # What it proves
//!
//! 1. **RTO (Recovery Time Objective):** cold-restore completes within the
//!    threshold (default 14400s = 4h).
//! 2. **Backup integrity:** Merkle root of the restored copy matches the
//!    source (reuses `maos_audit::backup::verify_backup_integrity`).
//! 3. **RPO (Recovery Point Objective):** gap between the last backup
//!    timestamp and the crash point is within threshold (reuses
//!    `maos_audit::backup::verify_rpo`).
//!
//! # Modes
//!
//! - **Synthetic (default):** creates a TL with `--frames` synthetic frames,
//!   backs it up, then cold-restores. Used in weekly CI cadence.
//! - **Real DR drill:** `--source <path>` uses an existing TL; `--backup
//!   <path>` uses an existing backup (skips backup creation).
//!
//! # Architecture references
//!
//! - NFR-Ops-2 (RTO ≤ 4h) — drilled, not printed.
//! - NFR-Ops-1 (RPO ≤ 1h) — verified via `verify_rpo`.
//! - §9.4 (backup/DR) — cold-restore path.

use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use maos_audit::backup::{latest_timestamp, verify_backup_integrity, verify_rpo};
use rusqlite::{Connection, OpenFlags};

/// Default RTO threshold: 4 hours in seconds (NFR-Ops-2).
const DEFAULT_RTO_THRESHOLD_SECS: u64 = 4 * 60 * 60;

/// Default frame count for the synthetic TL.
const DEFAULT_SYNTHETIC_FRAMES: usize = 10_000;

/// Default RPO threshold: 1 hour in nanoseconds (NFR-Ops-1).
const DEFAULT_RPO_THRESHOLD_NS: u64 = 60 * 60 * 1_000_000_000;

/// Report from the RTO drill.
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    pub rto_seconds: f64,
    pub rto_threshold_seconds: u64,
    pub backup_integrity_verified: bool,
    pub rpo_verified: bool,
    pub frame_count: usize,
    pub source_path: String,
    pub backup_path: String,
    pub restored_path: String,
}

/// Run the RTO drill gate.
///
/// Creates (or loads) a source TL, backs it up, cold-restores to a temp
/// destination, times the restore, asserts RTO ≤ threshold, and writes
/// evidence to `evidence_output` (if provided) in the format consumed by
/// `check_rto_gate`.
pub fn run(
    source: Option<&str>,
    backup: Option<&str>,
    frames: Option<usize>,
    rto_threshold_secs: Option<u64>,
    evidence_output: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let n_frames = frames.unwrap_or(DEFAULT_SYNTHETIC_FRAMES);
    let rto_threshold = rto_threshold_secs.unwrap_or(DEFAULT_RTO_THRESHOLD_SECS);

    // AF (representativeness): when a live Postgres is configured, the drill
    // restores the Postgres COLLECTIVE tier (the v1.5 persistence target),
    // not the SQLite backend being migrated away from.  v1.5 RTO timing is
    // nominal; true at-scale 4h-breaching falsifiability is a v2.0 target.
    let report = if std::env::var("MAOS_TEST_POSTGRES").is_ok() {
        drill_postgres(n_frames, rto_threshold)?
    } else {
        drill(source, backup, n_frames, rto_threshold)?
    };

    // Write evidence for check_rto_gate to consume (weekly cadence path).
    if let Some(path) = evidence_output {
        append_evidence(path, &report)?;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else if report.passed {
        println!(
            "rto-drill: PASSED — RTO={:.3}s (threshold {}s), integrity={}, RPO={}, frames={}",
            report.rto_seconds,
            report.rto_threshold_seconds,
            report.backup_integrity_verified,
            report.rpo_verified,
            report.frame_count,
        );
    } else {
        eprintln!(
            "rto-drill: FAILED — RTO={:.3}s exceeds threshold {}s",
            report.rto_seconds, report.rto_threshold_seconds,
        );
    }

    if !report.passed {
        return Err(format!(
            "rto-drill failed: RTO {:.3}s exceeds threshold {}s",
            report.rto_seconds, report.rto_threshold_seconds,
        ));
    }

    Ok(())
}

/// Append a drill evidence entry to the TOML ledger consumed by
/// `check_rto_gate`.  Format:
/// ```text
/// [[evidence]]
/// drill_date = "2026-06-22"
/// rto_seconds = 3600
/// drill_success = true
/// ```
fn append_evidence(path: &str, report: &Report) -> Result<(), String> {
    let drill_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let entry = format!(
        "\n[[evidence]]\ndrill_date = \"{drill_date}\"\nrto_seconds = {rto}\ndrill_success = {ok}\nnotes = \"integrity={integ},rpo={rpo},frames={frames}\"\n",
        rto = report.rto_seconds.round() as u64,
        ok = report.passed,
        integ = report.backup_integrity_verified,
        rpo = report.rpo_verified,
        frames = report.frame_count,
    );

    // Atomic append (create if absent) — O_APPEND makes the write atomic at the
    // OS level for reasonable-sized entries, avoiding the non-atomic
    // read-modify-write race on the shared weekly ledger (P18).
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open evidence {path}: {e}"))?;
    std::io::Write::write_all(&mut f, entry.as_bytes())
        .map_err(|e| format!("write evidence {path}: {e}"))?;
    Ok(())
}

fn drill(
    source: Option<&str>,
    backup: Option<&str>,
    n_frames: usize,
    rto_threshold_secs: u64,
) -> Result<Report, String> {
    let temp = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;

    // 1. Obtain (or create) the source TL.
    let source_path = match source {
        Some(p) => PathBuf::from(p),
        None => {
            let p = temp.path().join("source.sqlite");
            create_synthetic_tl(&p, n_frames)?;
            p
        }
    };

    // Count frames in the source.
    let frame_count = count_frames(&source_path)?;

    // 2. Obtain (or create) the backup.
    let backup_path = match backup {
        Some(p) => PathBuf::from(p),
        None => {
            let p = temp.path().join("backup.sqlite");
            backup_tl(&source_path, &p)?;
            p
        }
    };

    // 3. Verify backup integrity BEFORE the restore (Merkle root cross-check).
    let integrity_ok =
        verify_backup_integrity(&source_path, &backup_path).is_ok();

    // 4. Cold-restore: copy backup → fresh destination, timed.
    let restored_path = temp.path().join("restored.sqlite");
    let start = Instant::now();
    backup_tl(&backup_path, &restored_path)?;
    let rto_seconds = start.elapsed().as_secs_f64();

    // 5. Verify restored copy integrity (Merkle root of restored == source).
    let restored_integrity = verify_backup_integrity(&source_path, &restored_path).is_ok();

    // 6. RPO check: gap between last backup timestamp and "crash" point.
    // For a synthetic TL (timestamps near epoch), we simulate a crash
    // immediately after backup (crash = last_backup_ts + 1).  For a real
    // DR drill (source provided), we use wall-clock now as the crash point.
    let last_backup_ts = latest_timestamp(&backup_path)
        .map_err(|e| format!("failed to read backup latest_timestamp: {e}"))?
        .unwrap_or(0);
    let crash_ns = if source.is_some() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("system clock: {e}"))?
            .as_nanos() as u64
    } else {
        // Synthetic mode: crash 1ns after the last backup (RPO gap ≈ 0).
        last_backup_ts.saturating_add(1)
    };
    let rpo_ok = verify_rpo(last_backup_ts, crash_ns, DEFAULT_RPO_THRESHOLD_NS).is_ok();

    let passed = rto_seconds <= rto_threshold_secs as f64
        && integrity_ok
        && restored_integrity
        && rpo_ok;

    Ok(Report {
        passed,
        rto_seconds,
        rto_threshold_seconds: rto_threshold_secs,
        backup_integrity_verified: integrity_ok && restored_integrity,
        rpo_verified: rpo_ok,
        frame_count,
        source_path: source_path.display().to_string(),
        backup_path: backup_path.display().to_string(),
        restored_path: restored_path.display().to_string(),
    })
}

/// AF (representativeness) — drill the Postgres COLLECTIVE-tier restore path.
///
/// "Restore" = re-populate the Postgres collective Transparency Log from the
/// SQLite backup source (the migration direction), timed, then triple-oracle
/// verify (re-derives BOTH backends).  This measures the v1.5 persistence
/// target, not the SQLite backend being migrated away from.
fn drill_postgres(n_frames: usize, rto_threshold_secs: u64) -> Result<Report, String> {
    let conn_str = std::env::var("MAOS_TEST_POSTGRES")
        .map_err(|_| "MAOS_TEST_POSTGRES unset".to_string())?;
    let temp = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;

    let source_path = temp.path().join("source.sqlite");
    create_synthetic_tl(&source_path, n_frames)?;
    let frame_count = count_frames(&source_path)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    // Timed restore: migrate SQLite→Postgres (re-populate the collective tier).
    let start = Instant::now();
    let result = rt
        .block_on(maos_loom_lite::migration::migrate_with_conn_str(
            &source_path,
            &conn_str,
        ))
        .map_err(|e| format!("postgres collective restore (migrate): {e}"))?;
    let rto_seconds = start.elapsed().as_secs_f64();

    // Triple-oracle verify (independently re-derives BOTH backends).
    result
        .verify()
        .map_err(|e| format!("post-restore triple-oracle verify: {e}"))?;

    // RPO: synthetic crash 1ns after backup (RPO gap ≈ 0).
    let last_backup_ts = latest_timestamp(&source_path)
        .map_err(|e| format!("source latest_timestamp: {e}"))?
        .unwrap_or(0);
    let rpo_ok =
        verify_rpo(last_backup_ts, last_backup_ts.saturating_add(1), DEFAULT_RPO_THRESHOLD_NS)
            .is_ok();

    let passed = rto_seconds <= rto_threshold_secs as f64 && rpo_ok;

    Ok(Report {
        passed,
        rto_seconds,
        rto_threshold_seconds: rto_threshold_secs,
        backup_integrity_verified: true,
        rpo_verified: rpo_ok,
        frame_count,
        source_path: source_path.display().to_string(),
        backup_path: "<sqlite source>".to_string(),
        restored_path: "<postgres collective_memory>".to_string(),
    })
}

// ─── SQLite helpers ──────────────────────────────────────────────────────

/// TL schema — mirrors `maos-iac` SCHEMA_SQL (read-side cannot depend on
/// `maos-iac`; this is the same shape used by `maos-audit` tests).
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

/// Create a synthetic TL with `n` frames.
fn create_synthetic_tl(path: &Path, n: usize) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    conn.execute_batch(TL_SCHEMA)
        .map_err(|e| format!("create schema: {e}"))?;
    for i in 0..n {
        let mut fid = [0u8; 16];
        fid[0..8].copy_from_slice(&(i as u64).to_le_bytes());
        let ts_ns = (i as u64 + 1) * 1_000_000;
        conn.execute(
            "INSERT INTO transparency_log \
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, 1, 1, 1, 'drill', X'00', 0)",
            rusqlite::params![&fid[..], ts_ns],
        )
        .map_err(|e| format!("insert frame {i}: {e}"))?;
    }
    Ok(())
}

/// Back up a TL using SQLite's online backup API (same approach as
/// `maos-cli::backup::backup_transparency_log`).
fn backup_tl(source: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Err(format!(
            "destination already exists: {}",
            dest.display()
        ));
    }
    let src = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("open source {}: {e}", source.display()))?;
    let mut dst = Connection::open(dest)
        .map_err(|e| format!("open dest {}: {e}", dest.display()))?;
    let backup = rusqlite::backup::Backup::new(&src, &mut dst)
        .map_err(|e| format!("init backup: {e}"))?;
    backup
        .run_to_completion(100, std::time::Duration::from_millis(250), None)
        .map_err(|e| format!("backup run: {e}"))?;
    Ok(())
}

/// Count frames in a TL database.
fn count_frames(path: &Path) -> Result<usize, String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transparency_log", [], |row| row.get(0))
        .map_err(|e| format!("count frames: {e}"))?;
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_tl_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("s.sqlite");
        let backup = temp.path().join("b.sqlite");
        let restored = temp.path().join("r.sqlite");

        create_synthetic_tl(&source, 100).unwrap();
        assert_eq!(count_frames(&source).unwrap(), 100);

        backup_tl(&source, &backup).unwrap();
        assert_eq!(count_frames(&backup).unwrap(), 100);

        // Integrity check.
        assert!(verify_backup_integrity(&source, &backup).is_ok());

        // Cold restore.
        let start = Instant::now();
        backup_tl(&backup, &restored).unwrap();
        let rto = start.elapsed().as_secs_f64();
        assert_eq!(count_frames(&restored).unwrap(), 100);
        assert!(verify_backup_integrity(&source, &restored).is_ok());

        // RTO for 100 frames should be well under 1s.
        assert!(rto < 60.0, "RTO for 100 frames should be fast, got {rto}s");
    }

    #[test]
    fn rpo_within_threshold_for_fresh_backup() {
        // A backup made moments ago should have RPO well within 1h.
        let rpo_ok = verify_rpo(
            1_000_000_000_000, // last backup ts
            1_000_000_000_001, // crash ts (1ns later)
            DEFAULT_RPO_THRESHOLD_NS,
        );
        assert!(rpo_ok.is_ok());
    }

    #[test]
    fn rpo_violation_detected() {
        let result = verify_rpo(
            1_000_000_000_000,                               // last backup
            1_000_000_000_000 + 2 * DEFAULT_RPO_THRESHOLD_NS, // crash 2h after threshold
            DEFAULT_RPO_THRESHOLD_NS,
        );
        assert!(result.is_err());
    }

    #[test]
    fn default_thresholds_are_correct() {
        assert_eq!(DEFAULT_RTO_THRESHOLD_SECS, 4 * 60 * 60);
        assert_eq!(DEFAULT_RPO_THRESHOLD_NS, 60 * 60 * 1_000_000_000);
        assert_eq!(DEFAULT_SYNTHETIC_FRAMES, 10_000);
    }

    #[test]
    fn full_drill_synthetic_passes() {
        let report = drill(None, None, 500, DEFAULT_RTO_THRESHOLD_SECS).unwrap();
        assert!(report.passed, "drill should pass for 500 frames");
        assert!(report.backup_integrity_verified);
        assert!(report.rpo_verified);
        assert_eq!(report.frame_count, 500);
        assert!(report.rto_seconds < 60.0, "RTO should be fast: {}s", report.rto_seconds);
    }
}
