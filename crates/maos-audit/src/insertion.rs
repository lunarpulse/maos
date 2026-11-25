//! Story 23-4 (D-23-4-06 A, D-23-4-07 A) — the insertion-id reader the SIEM cursor resumes from.
//!
//! The insertion id is the SQLite rowid of the Transparency Log: ADR-072 keeps it through a
//! rotation (a sealed segment carries its rows' original ids) and `VACUUM INTO` copies it, so
//! it orders the merged view (sealed segments, then the live file) where a timestamp does
//! not. Rows come out through the same redaction mapper as
//! [`crate::query_with_redaction`]. Like it, [`LogView::open`] refuses a database with an
//! active WAL (ADR-028 D6), so a caller passes a quiesced copy or snapshot. Every source is then
//! opened `immutable=1`, so reading never creates a `-wal`/`-shm` sidecar that would make the
//! original database look un-quiesced to the next reader.

#[cfg(all(feature = "siem-fault-inject", not(debug_assertions)))]
compile_error!("siem-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::{Connection, OptionalExtension};

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

/// Fixed per-row allowance added to a row's stored bytes when planning a batch against a byte
/// bound (the RFC 5424/CEF envelope around the payload).
const ROW_OVERHEAD_BYTES: usize = 600;

/// Every readable source of one quiesced Transparency Log, opened once: the unexpired sealed
/// segments, then the live file (last).
pub struct LogView {
    sources: Vec<Connection>,
    peak_lookahead: AtomicUsize,
}

struct Meta {
    id: i64,
    frame: Vec<u8>,
    bytes: usize,
    source: usize,
}

impl LogView {
    /// The next entry of `source` after `after`: one indexed lookup, no window.
    fn next_meta(&self, source: usize, after: i64) -> Result<Option<Meta>, AuditError> {
        let mut statement = self.sources[source]
            .prepare_cached(
                "SELECT rowid, frame_id, length(payload_redacted) + length(intent) \
                 FROM transparency_log WHERE rowid > ?1 ORDER BY rowid LIMIT 1",
            )
            .map_err(AuditError::Read)?;
        statement
            .query_row([after], |row| {
                Ok(Meta {
                    id: row.get(0)?,
                    frame: row.get(1)?,
                    bytes: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
                    source,
                })
            })
            .optional()
            .map_err(AuditError::Read)
    }

    /// The most lookahead entries the planner has held at once since this view opened (a
    /// memory proxy for tests: it is bounded by the number of sources).
    #[doc(hidden)]
    #[must_use]
    pub fn peak_lookahead(&self) -> usize {
        self.peak_lookahead.load(Ordering::Relaxed)
    }

    /// Open the view of `db_path`.
    ///
    /// # Errors
    /// A live file with an active WAL, or an unreadable source (a registered segment that is
    /// missing is an error: never a silently shorter log).
    pub fn open(db_path: &Path) -> Result<Self, AuditError> {
        crate::refuse_active_wal(db_path)?;
        Ok(Self {
            sources: segment::open_log_sources_immutable(db_path)?,
            peak_lookahead: AtomicUsize::new(0),
        })
    }

    /// The sanctioned, redaction-applying read: rows with `insertion_id > after`, in
    /// insertion-id order across every source, at most `max_rows` of them and, once the first
    /// row is taken, no more than about `max_bytes` of stored payload (each row also counts a
    /// fixed envelope allowance). A single row larger than `max_bytes` is returned alone: a
    /// log row cannot be split. Only the chosen rows are materialised, so memory is bounded by
    /// the batch, never by the number of sources or the size of the backlog.
    ///
    /// # Errors
    /// A read failure of any source, or two sources holding the same insertion id under
    /// different `frame_id`s (a corrupt or tampered set of segments).
    pub fn read_after(
        &self,
        after: i64,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<Vec<InsertionRow>, AuditError> {
        // Pass 1: a bounded ordered k-way merge over the sources. Each source holds ONE
        // lookahead entry (ids, frame id and size only), so the planner's memory is the number
        // of sources plus the chosen rows, never the backlog or a per-source row window.
        let mut heads: Vec<Option<Meta>> = Vec::with_capacity(self.sources.len());
        for source in 0..self.sources.len() {
            heads.push(self.next_meta(source, after)?);
        }
        let mut chosen: Vec<Meta> = Vec::new();
        let mut planned = 0_usize;
        loop {
            self.peak_lookahead
                .fetch_max(heads.iter().flatten().count(), Ordering::Relaxed);
            let Some(id) = heads.iter().flatten().map(|meta| meta.id).min() else {
                break;
            };
            // Every source holding `id` must hold the same frame; the first one is kept.
            let mut taken: Option<Meta> = None;
            for source in 0..heads.len() {
                if heads[source].as_ref().is_some_and(|meta| meta.id == id) {
                    let meta = heads[source].take().expect("checked");
                    heads[source] = self.next_meta(source, id)?;
                    match &taken {
                        Some(first) if first.frame != meta.frame => {
                            return Err(AuditError::Row(format!(
                                "insertion id {id} is held by two different frames"
                            )));
                        }
                        Some(_) => {}
                        None => taken = Some(meta),
                    }
                }
            }
            let meta = taken.expect("a head held the minimum id");
            let cost = meta.bytes.saturating_add(ROW_OVERHEAD_BYTES);
            if !chosen.is_empty() && planned.saturating_add(cost) > max_bytes {
                break;
            }
            planned = planned.saturating_add(cost);
            chosen.push(meta);
            if chosen.len() >= max_rows {
                break;
            }
        }
        // Pass 2: only the chosen rows, in full.
        let mut rows = Vec::with_capacity(chosen.len());
        for meta in chosen {
            let mut statement = self.sources[meta.source]
                .prepare_cached(
                    "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, \
                     kind, intent, payload_redacted, rowid FROM transparency_log WHERE rowid = ?1",
                )
                .map_err(AuditError::Read)?;
            let row = statement
                .query_row([meta.id], |row| {
                    Ok(InsertionRow {
                        entry: sanctioned_entry(row)?,
                        insertion_id: row.get(8)?,
                    })
                })
                .map_err(AuditError::Read)?;
            rows.push(row);
        }
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

    /// Expired segments whose ids reach beyond `after`, in insertion order (D-23-4-06 A: the
    /// rows are gone, so the exporter journals a gap rather than pretending they were
    /// delivered). Expiry need not remove a prefix: a legal hold can keep an older segment.
    ///
    /// # Errors
    /// A read failure of the live file's registry.
    pub fn expired_ranges_after(&self, after: i64) -> Result<Vec<ExpiredRange>, AuditError> {
        let mut ranges: Vec<ExpiredRange> = self
            .registry(true)?
            .into_iter()
            .filter(|range| range.last_rowid > after)
            .collect();
        ranges.sort_by_key(|range| (range.first_rowid, range.seq));
        Ok(ranges)
    }

    /// Whether some CURRENTLY expired segment ends exactly at `insertion_id` (the evidence
    /// behind a cursor the exporter parked after a gap).
    ///
    /// # Errors
    /// A read failure of the live file's registry.
    pub fn expired_segment_ends_at(&self, insertion_id: i64) -> Result<bool, AuditError> {
        Ok(self
            .registry(true)?
            .iter()
            .any(|range| range.last_rowid == insertion_id))
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
