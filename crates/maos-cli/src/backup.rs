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
    #[error("backup team mismatch: expected {expected}, found {actual}")]
    TeamMismatch { expected: String, actual: String },
}

fn team_from_transparency_log_path(
    path: &Path,
) -> Result<Option<maos_domain::team::TeamId>, BackupError> {
    let Some(team_dir) = path.parent() else {
        return Ok(None);
    };
    let Some(teams_dir) = team_dir.parent() else {
        return Ok(None);
    };
    if teams_dir.file_name().and_then(|name| name.to_str()) != Some("teams") {
        return Ok(None);
    }
    let team = team_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            BackupError::IntegrityFailure("tenant backup path is not valid UTF-8".to_string())
        })?;
    maos_domain::team::TeamId::new(team)
        .map(Some)
        .map_err(|error| {
            BackupError::IntegrityFailure(format!(
                "tenant backup path carries a non-canonical team: {error}"
            ))
        })
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
    let source_team = team_from_transparency_log_path(source_path)?;
    if let Some(team) = source_team.as_ref() {
        maos_audit::validate_transparency_log_team_binding(source_path, team).map_err(|error| {
            BackupError::IntegrityFailure(format!(
                "tenant backup source binding is invalid: {error}"
            ))
        })?;
    }
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let src = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut dst = Connection::open(dest_path)?;
    let backup = rusqlite::backup::Backup::new(&src, &mut dst)?;
    backup.run_to_completion(100, std::time::Duration::from_millis(250), None)?;
    drop(backup);
    if let Some(team) = source_team {
        let result = dst
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS maos_backup_metadata (
                key TEXT NOT NULL PRIMARY KEY,
                value TEXT NOT NULL
             );",
            )
            .and_then(|_| {
                dst.execute(
                    "INSERT INTO maos_backup_metadata (key, value)
                 VALUES ('team_id', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    rusqlite::params![team.as_str()],
                )
            });
        if let Err(error) = result {
            drop(dst);
            let _ = std::fs::remove_file(dest_path);
            return Err(BackupError::Sqlite(error));
        }
    }
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

/// Cold-restore a tenant backup only when its embedded physical-team
/// provenance matches the manifest-selected destination team.
pub fn cold_restore_to_temp_for_team(
    backup_path: &Path,
    expected_team: &maos_domain::team::TeamId,
) -> Result<std::path::PathBuf, BackupError> {
    let source = Connection::open_with_flags(
        backup_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let actual: String = source
        .query_row(
            "SELECT value FROM maos_backup_metadata WHERE key = 'team_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            BackupError::IntegrityFailure(format!(
                "tenant backup has no readable team provenance: {error}"
            ))
        })?;
    if actual != expected_team.as_str() {
        return Err(BackupError::TeamMismatch {
            expected: expected_team.to_string(),
            actual,
        });
    }
    drop(source);
    cold_restore_to_temp(backup_path)
}

/// Read a backup's embedded physical-team provenance, if any. A global
/// (untenanted) backup carries no `maos_backup_metadata.team_id` row → `None`.
fn backup_team_provenance(
    backup_path: &Path,
) -> Result<Option<maos_domain::team::TeamId>, BackupError> {
    let conn = Connection::open_with_flags(
        backup_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let has_metadata: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master \
             WHERE type = 'table' AND name = 'maos_backup_metadata')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if !has_metadata {
        return Ok(None);
    }
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM maos_backup_metadata WHERE key = 'team_id'",
            [],
            |row| row.get(0),
        )
        .ok();
    match value {
        None => Ok(None),
        Some(raw) => maos_domain::team::TeamId::new(raw.trim())
            .map(Some)
            .map_err(|error| {
                BackupError::IntegrityFailure(format!(
                    "backup team provenance is non-canonical: {error}"
                ))
            }),
    }
}

/// Validate that a restore target's implied team matches the backup's declared
/// team BEFORE any bytes are copied. A tenanted backup must land at the same
/// team's path; a global backup at the global path. Refuses the cross-team
/// planting vector (restoring team-A's backup onto team-B's shard path).
pub fn validate_restore_target_team(
    backup_path: &Path,
    target_path: &Path,
) -> Result<(), BackupError> {
    let backup_team = backup_team_provenance(backup_path)?;
    let target_team = team_from_transparency_log_path(target_path)?;
    match (backup_team.as_ref(), target_team.as_ref()) {
        (Some(backup), Some(target)) if backup == target => Ok(()),
        (None, None) => Ok(()),
        (Some(backup), Some(target)) => Err(BackupError::TeamMismatch {
            expected: target.to_string(),
            actual: backup.to_string(),
        }),
        (Some(_), None) => Err(BackupError::IntegrityFailure(
            "tenanted backup cannot be restored onto the global (untenanted) path".to_string(),
        )),
        (None, Some(_)) => Err(BackupError::IntegrityFailure(
            "global backup cannot be restored onto a tenanted path".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::team::TeamId;
    use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

    #[test]
    fn tenant_backup_round_trip_rejects_wrong_team() {
        let dir = tempfile::TempDir::new().unwrap();
        let security = TeamId::new("security").unwrap();
        let support = TeamId::new("support").unwrap();
        let security_path = dir.path().join("teams/security/transparency.sqlite");
        let support_path = dir.path().join("teams/support/transparency.sqlite");
        std::fs::create_dir_all(security_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(support_path.parent().unwrap()).unwrap();

        for (path, team, intent) in [
            (&security_path, &security, "security-only"),
            (&support_path, &support, "support-only"),
        ] {
            let log = TransparencyLogAdapter::open(path, 1).unwrap();
            std::fs::write(
                maos_audit::transparency_log_team_binding_path(path),
                team.as_str(),
            )
            .unwrap();
            let _ = log.insert_frame_event(
                FrameKind::CapabilityInvocation,
                7,
                None,
                intent,
                b"redacted",
                FrameOrigin::Kernel,
            );
        }

        let security_backup = dir.path().join("security.backup.sqlite");
        backup_transparency_log(&security_path, &security_backup).unwrap();
        let restored = cold_restore_to_temp_for_team(&security_backup, &security).unwrap();
        let restored_log = TransparencyLogAdapter::open(&restored, 2).unwrap();
        let rows = restored_log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].intent, "security-only");
        assert!(matches!(
            cold_restore_to_temp_for_team(&security_backup, &support),
            Err(BackupError::TeamMismatch { .. })
        ));
    }
    #[test]
    fn validate_restore_target_team_refuses_cross_team_planting() {
        let dir = tempfile::TempDir::new().unwrap();
        let security = TeamId::new("security").unwrap();
        let security_path = dir.path().join("teams/security/transparency.sqlite");
        let support_path = dir.path().join("teams/support/transparency.sqlite");
        let global_path = dir.path().join("transparency.sqlite");
        std::fs::create_dir_all(security_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(support_path.parent().unwrap()).unwrap();

        let log = TransparencyLogAdapter::open(&security_path, 1).unwrap();
        std::fs::write(
            maos_audit::transparency_log_team_binding_path(&security_path),
            security.as_str(),
        )
        .unwrap();
        let _ = log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            7,
            None,
            "security-only",
            b"redacted",
            FrameOrigin::Kernel,
        );

        let security_backup = dir.path().join("security.backup.sqlite");
        backup_transparency_log(&security_path, &security_backup).unwrap();

        // Same-team restore is allowed.
        assert!(validate_restore_target_team(&security_backup, &security_path).is_ok());
        // Cross-team planting is refused BEFORE any copy.
        assert!(matches!(
            validate_restore_target_team(&security_backup, &support_path),
            Err(BackupError::TeamMismatch { .. })
        ));
        // A tenanted backup may not collapse into the global (untenanted) path.
        assert!(matches!(
            validate_restore_target_team(&security_backup, &global_path),
            Err(BackupError::IntegrityFailure { .. })
        ));
    }
}
