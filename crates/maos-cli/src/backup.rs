#![forbid(unsafe_code)]

//! Story 9.4 AC-3 — Transparency Log backup/restore write side.
//!
//! This module intentionally lives in `maos-cli` (not `maos-audit`) because
//! `maos-audit` is read-only by design (`SQLITE_OPEN_READ_ONLY`). The backup
//! API writes a new SQLite file from a read-only source, so it belongs on the
//! operator/CLI side of the boundary.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Typed backup/restore error.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("backup integrity failure: {0}")]
    IntegrityFailure(String),
}

/// Create a WAL-checkpoint-consistent backup of the Transparency Log.
///
/// Uses rusqlite's backup API (wraps SQLite's online backup API) which
/// creates a consistent snapshot even while the TL is being written to.
/// The backup is region-scoped (single-region redundancy only — AC-3 /
/// forward-coupling to 9.4b).
///
/// Fails if `dest_path` already exists (no silent overwrite) or if
/// `source_path == dest_path`.
pub fn backup_transparency_log(source_path: &Path, dest_path: &Path) -> Result<(), BackupError> {
    if source_path == dest_path {
        return Err(BackupError::IntegrityFailure(
            "source and destination paths must differ".to_string(),
        ));
    }
    if dest_path.exists() {
        return Err(BackupError::IntegrityFailure(format!(
            "destination already exists: {}",
            dest_path.display()
        )));
    }
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let src = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut dst = Connection::open(dest_path)?;
    let backup = rusqlite::backup::Backup::new(&src, &mut dst)?;
    // pages_per_step = 100 pages at a time, 250ms sleep between steps.
    // For small TLs this completes in one step; for large TLs it yields
    // periodically to avoid blocking the source connection's WAL for too long.
    backup.run_to_completion(100, std::time::Duration::from_millis(250), None)?;
    Ok(())
}

/// Restore a backup to a new destination (cold restore) without touching the
/// live source TL. Returns the path of the restored database.
pub fn cold_restore_to_temp(backup_path: &Path) -> Result<std::path::PathBuf, BackupError> {
    let temp_dir = std::env::temp_dir().join(format!("maos-cold-restore-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;
    let restored = temp_dir.join("restored.sqlite");
    backup_transparency_log(backup_path, &restored)?;
    Ok(restored)
}
