//! Story 23-4 (D-23-4-06 A, D-23-4-07 A) — the insertion-id reader the SIEM cursor resumes from.
//!
//! The insertion id is the SQLite rowid of the Transparency Log: ADR-072 keeps it through a
//! rotation (a sealed segment carries its rows' original ids) and `VACUUM INTO` copies it, so
//! it orders the merged view (sealed segments, then the live file) where a timestamp does
//! not. Rows come out through the same redaction mapper as
//! [`crate::query_with_redaction`]. Like it, [`LogView::open`] refuses a database with an
//! active WAL (ADR-028 D6), so a caller passes a quiesced copy or snapshot; it checks once,
//! because the read-only opens that follow may themselves leave a `-wal` file beside a
//! WAL-mode database.

#[cfg(all(feature = "siem-fault-inject", not(debug_assertions)))]
compile_error!("siem-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use std::path::Path;

use rusqlite::Connection;

use crate::{segment, AuditEntry, AuditError};

/// One row with its insertion id.
#[derive(Debug, Clone)]
pub struct InsertionRow {
    pub insertion_id: i64,
    pub entry: AuditEntry,
}

/// What the merged view holds at one insertion id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// `frame_id` (hex) of the row at the id, `None` when no readable source holds it.
    pub frame_id_hex: Option<String>,
    /// The highest insertion id the log has ever assigned that this view can see.
    pub high_water: i64,
}

/// An expired sealed segment: its rows left the view, its registry row stayed (ADR-072 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredRange {
    pub seq: i64,
    pub first_rowid: i64,
    pub last_rowid: i64,
}

/// Every readable source of one quiesced Transparency Log, opened once: the unexpired sealed
/// segments, then the live file (last).
pub struct LogView {
    sources: Vec<Connection>,
}

impl LogView {
    /// Open the view of `db_path`.
    ///
    /// # Errors
    /// A live file with an active WAL, or an unreadable source (a registered segment that is
    /// missing is an error: never a silently shorter log).
    pub fn open(db_path: &Path) -> Result<Self, AuditError> {
        crate::refuse_active_wal(db_path)?;
        Ok(Self {
            sources: segment::open_log_sources(db_path)?,
        })
    }

    /// The sanctioned, redaction-applying read: up to `limit` rows with `insertion_id > after`,
    /// in insertion-id order across every source.
    ///
    /// # Errors
    /// A read failure of any source.
    pub fn read_after(&self, after: i64, limit: usize) -> Result<Vec<InsertionRow>, AuditError> {
        let limit_sql = i64::try_from(limit).map_err(|_| AuditError::ValueOverflow {
            field: "limit",
            value: limit as u64,
        })?;
        let mut rows = Vec::new();
        for conn in &self.sources {
            let mut statement = conn
                .prepare(
                    "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, \
                     kind, intent, payload_redacted, rowid FROM transparency_log \
                     WHERE rowid > ?1 ORDER BY rowid LIMIT ?2",
                )
                .map_err(AuditError::Read)?;
            let mapped = statement
                .query_map([after, limit_sql], |row| {
                    Ok(InsertionRow {
                        entry: sanctioned_entry(row)?,
                        insertion_id: row.get(8)?,
                    })
                })
                .map_err(AuditError::Read)?;
            for row in mapped {
                rows.push(row.map_err(AuditError::Read)?);
            }
        }
        rows.sort_by_key(|row| row.insertion_id);
        rows.dedup_by_key(|row| row.insertion_id);
        rows.truncate(limit);
        Ok(rows)
    }

    /// How many rows lie after `after` (what a down sink leaves buffered in the log itself).
    ///
    /// # Errors
    /// A read failure of any source.
    pub fn count_after(&self, after: i64) -> Result<u64, AuditError> {
        let mut total: u64 = 0;
        for conn in &self.sources {
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM transparency_log WHERE rowid > ?1",
                    [after],
                    |row| row.get(0),
                )
                .map_err(AuditError::Read)?;
            total = total.saturating_add(count.max(0) as u64);
        }
        Ok(total)
    }

    /// The row at `insertion_id`, if any readable source holds it, and the view's high water.
    ///
    /// # Errors
    /// A read failure of any source.
    pub fn probe_anchor(&self, insertion_id: i64) -> Result<Anchor, AuditError> {
        let mut frame_id_hex = None;
        let mut high_water = 0_i64;
        for conn in &self.sources {
            let found: Option<Vec<u8>> = match conn.query_row(
                "SELECT frame_id FROM transparency_log WHERE rowid = ?1",
                [insertion_id],
                |row| row.get(0),
            ) {
                Ok(blob) => Some(blob),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(error) => return Err(AuditError::Read(error)),
            };
            if let Some(blob) = found {
                frame_id_hex = Some(crate::hex_encode(&blob));
            }
            let max: Option<i64> = conn
                .query_row("SELECT max(rowid) FROM transparency_log", [], |row| {
                    row.get(0)
                })
                .map_err(AuditError::Read)?;
            high_water = high_water.max(max.unwrap_or(0));
        }
        for range in self.registry(false)? {
            high_water = high_water.max(range.last_rowid);
        }
        Ok(Anchor {
            frame_id_hex,
            high_water,
        })
    }

    /// Expired segments whose ids reach beyond `after`, oldest first (D-23-4-06 A: the rows
    /// are gone, so the exporter journals a gap rather than pretending they were delivered).
    ///
    /// # Errors
    /// A read failure of the live file's registry.
    pub fn expired_ranges_after(&self, after: i64) -> Result<Vec<ExpiredRange>, AuditError> {
        Ok(self
            .registry(true)?
            .into_iter()
            .filter(|range| range.last_rowid > after)
            .collect())
    }

    /// The segment registry lives in the live file, the last source.
    fn registry(&self, expired_only: bool) -> Result<Vec<ExpiredRange>, AuditError> {
        let Some(live) = self.sources.last() else {
            return Ok(Vec::new());
        };
        let registered: bool = live
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' \
                 AND name='transparency_log_segments')",
                [],
                |row| row.get(0),
            )
            .map_err(AuditError::Read)?;
        if !registered {
            return Ok(Vec::new());
        }
        let sql = if expired_only {
            "SELECT seq, first_rowid, last_rowid FROM transparency_log_segments \
             WHERE expired_at_ns IS NOT NULL ORDER BY seq"
        } else {
            "SELECT seq, first_rowid, last_rowid FROM transparency_log_segments ORDER BY seq"
        };
        let mut statement = live.prepare(sql).map_err(AuditError::Read)?;
        let ranges = statement
            .query_map([], |row| {
                Ok(ExpiredRange {
                    seq: row.get(0)?,
                    first_rowid: row.get(1)?,
                    last_rowid: row.get(2)?,
                })
            })
            .map_err(AuditError::Read)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AuditError::Read)?;
        Ok(ranges)
    }
}

#[cfg(not(feature = "siem-fault-inject"))]
fn sanctioned_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEntry> {
    crate::redacted_entry(row)
}

/// FAULT-INJECT twin of the mapper above: it drops the redaction provenance so a leak
/// regression suite can invert the guarantee (`maos-siem` `fault_inject.rs`).
#[cfg(feature = "siem-fault-inject")]
fn sanctioned_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEntry> {
    let mut entry = crate::redacted_entry(row)?;
    entry.redaction = None;
    Ok(entry)
}
