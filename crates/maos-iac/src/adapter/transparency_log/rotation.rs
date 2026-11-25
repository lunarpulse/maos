//! Story 22-6 (ADR-072) — the bounded audit store: segment rotation inside the live log.
//!
//! By policy the oldest PREFIX of the Transparency Log moves into a sealed segment file
//! (`<audit dir>/segments/<seq:010>.sqlite` + a signed `maos.audit-segment.v1` manifest) and is
//! then deleted from the live file, in one transaction that also records the segment in the
//! live registry. The live file and its `rowid` sequence continue: the cut is always below the
//! live high-water row, so SQLite's `max(rowid)+1` keeps growing and every consumer that
//! assumes ids only grow (escape tail, progress stamper, forget proof, ratification sequence,
//! forget sweep windows) stays correct without being edited. The cut is also kept below every
//! registered floor (the tails' committed cursors, passed in by the caller), below the oldest
//! open `principal.forget.intent`, the oldest outstanding redistill request and the newest
//! completed-forget marker the door re-checks.
//!
//! Signing is the sealer's job ([`SegmentSealer`], installed by the composition root): this
//! crate copies rows, never holds a key. A segment is never deleted from the live file before
//! its sealer returned a manifest and a re-verification under the writer lock found every
//! copied row still equal to its live twin.
use super::*;
use rusqlite::types::Value;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

/// The segment directory name beside the live log.
pub const SEGMENTS_DIR_NAME: &str = "segments";
/// The shortest `audit_keep_days` expiry will honour (NFR-Aud-4, architecture §7.3).
pub const KEEP_FLOOR_DAYS: u64 = 90;
/// Intent of the journal row written before a sealed segment is unlinked.
pub const SEGMENT_EXPIRED_INTENT: &str = "audit.segment.expired";
/// Intent of the journal row written before a scrubbed segment replaces its predecessor.
pub const SEGMENT_RESEALED_INTENT: &str = "audit.segment.resealed";

const COPY_WINDOW: i64 = 4096;
const RECOPY_ATTEMPTS: u32 = 3;
const DAY_NS: u64 = 86_400_000_000_000;

const LOG_COLUMNS: &str = "frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, \
    boot_nonce, capability_token, kind, intent, correlation_id, payload_redacted, origin";
const LOG_COLUMN_COUNT: usize = 12;
/// `(table, columns, frame-key column)` of the frame-keyed side tables that move with their frames.
const SIDE_TABLES: [(&str, &str, &str); 4] = [
    (
        "transparency_log_retractions",
        "original_frame_id, retract_frame_id, retracted_at_ns",
        "original_frame_id",
    ),
    (
        "transparency_log_recipients",
        "frame_id, recipient_pid, recipient_spirit_id, recipient_role",
        "frame_id",
    ),
    (
        "transparency_log_consent",
        "frame_id, consent_id, granter_spirit_id, granter_host_id, granter_role, \
         granted_at_key, intent_class, valid_until_key, recall_policy",
        "frame_id",
    ),
    (
        "transparency_log_consent_ruptures",
        "rupture_frame_id, recipient_pid, granter_spirit_id, granter_host_id, intent_class",
        "rupture_frame_id",
    ),
];

const SEGMENT_SCHEMA_SQL: &str = "\
CREATE TABLE transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    correlation_id      TEXT,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);
CREATE INDEX idx_tlog_correlation ON transparency_log(correlation_id, timestamp_ns);
CREATE TABLE transparency_log_retractions (
    original_frame_id   BLOB    NOT NULL PRIMARY KEY,
    retract_frame_id    BLOB    NOT NULL,
    retracted_at_ns     INTEGER NOT NULL
);
CREATE TABLE transparency_log_recipients (
    frame_id            BLOB    NOT NULL,
    recipient_pid       INTEGER NOT NULL,
    recipient_spirit_id TEXT    NOT NULL,
    recipient_role      TEXT,
    PRIMARY KEY (frame_id, recipient_pid)
);
CREATE INDEX idx_tlog_recipient_pid ON transparency_log_recipients(recipient_pid, frame_id);
CREATE TABLE transparency_log_consent (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    consent_id          BLOB    NOT NULL,
    granter_spirit_id   TEXT    NOT NULL,
    granter_host_id     TEXT,
    granter_role        TEXT,
    granted_at_key      INTEGER NOT NULL,
    intent_class        TEXT,
    valid_until_key     INTEGER,
    recall_policy       INTEGER NOT NULL
);
CREATE TABLE transparency_log_consent_ruptures (
    rupture_frame_id    BLOB    NOT NULL,
    recipient_pid       INTEGER NOT NULL,
    granter_spirit_id   TEXT    NOT NULL,
    granter_host_id     TEXT,
    intent_class        TEXT,
    PRIMARY KEY (rupture_frame_id, recipient_pid)
);
CREATE INDEX idx_tlog_rupture_recipient
    ON transparency_log_consent_ruptures(recipient_pid, granter_spirit_id);
";

/// The durable-request SQL of [`TransparencyLogAdapter::outstanding_redistill_requests`],
/// shared with the rotation floor (column 3 is the insertion id).
pub(super) const OUTSTANDING_REDISTILL_SQL: &str = "SELECT r.payload_redacted, r.timestamp_ns, r.frame_id, r.rowid \
     FROM transparency_log r INDEXED BY idx_tlog_redistill_request_rowid \
     WHERE r.rowid > ?2 AND r.rowid <= ?3 AND ?1 IS ?1 \
     AND r.intent = 'distillate.redistill.requested' \
     AND NOT EXISTS (SELECT 1 FROM transparency_log o INDEXED BY idx_tlog_redistill_outcome \
       WHERE o.intent IN ('distillate.redistilled','distillate.redistill.pending') \
       AND CASE WHEN json_valid(CAST(o.payload_redacted AS TEXT),2) \
         THEN json_extract(CAST(o.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
         ELSE X'' END = json_extract(CAST(r.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
       UNION ALL SELECT 1 FROM transparency_log bad INDEXED BY idx_tlog_redistill_outcome \
       WHERE bad.intent IN ('distillate.redistilled','distillate.redistill.pending') \
       AND CASE WHEN json_valid(CAST(bad.payload_redacted AS TEXT),2) \
         THEN json_extract(CAST(bad.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
         ELSE X'' END = X'' \
       AND json_extract(CAST(bad.payload_redacted AS TEXT),'$') IS NULL)";

pub(super) fn segments_dir_for(db_path: &Path) -> Option<PathBuf> {
    db_path
        .parent()
        .map(|parent| parent.join(SEGMENTS_DIR_NAME))
}

/// One registry row: a sealed segment the live log knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentRef {
    pub seq: u64,
    /// Exclusive lower bound: the segment holds exactly the rows with `after_rowid < rowid <= last_rowid`.
    pub after_rowid: i64,
    pub first_rowid: i64,
    pub last_rowid: i64,
    pub row_count: u64,
    pub min_timestamp_ns: i64,
    pub max_timestamp_ns: i64,
    /// sha256 of the current manifest file (hex).
    pub manifest_sha256: String,
    pub prev_manifest_sha256: Option<String>,
    pub sealed_at_ns: i64,
    /// Set once the segment was expired; the row stays so the chain and the id floor survive.
    pub expired_at_ns: Option<i64>,
}

impl SegmentRef {
    pub fn file_name(&self) -> String {
        format!("{:010}.sqlite", self.seq)
    }
    pub fn manifest_name(&self) -> String {
        format!("{:010}.manifest.json", self.seq)
    }
}

/// What the sealer is asked to sign.
#[derive(Debug, Clone)]
pub struct SegmentSealRequest {
    pub seq: u64,
    /// The finished SQLite file to digest (a temporary name while a segment is being re-sealed).
    pub segment_path: PathBuf,
    /// The name the manifest must give the file (`<seq:010>.sqlite`).
    pub file_name: String,
    pub after_rowid: i64,
    pub first_rowid: i64,
    pub last_rowid: i64,
    pub row_count: u64,
    pub min_timestamp_ns: i64,
    pub max_timestamp_ns: i64,
    pub prev_manifest_sha256: Option<String>,
    /// The manifest this one replaces (a re-seal), if any.
    pub replaces_manifest: Option<PathBuf>,
    /// Frame ids (hex) scrubbed by this re-seal.
    pub scrubbed_frame_ids: Vec<String>,
}

/// A signed manifest and its sha256 (hex).
#[derive(Debug, Clone)]
pub struct SealedSegment {
    pub manifest: Vec<u8>,
    pub manifest_sha256: String,
}

/// Signs a finished segment. Installed once by the composition root; an adapter without one
/// refuses to rotate and the live log simply keeps growing.
pub trait SegmentSealer: Send + Sync + std::fmt::Debug {
    fn seal(&self, request: &SegmentSealRequest) -> Result<SealedSegment, String>;
}

/// A segment sealer is already installed on this adapter.
#[derive(Debug, thiserror::Error)]
#[error("a segment sealer is already installed")]
pub struct SealerAlreadyInstalled;

#[derive(Debug, thiserror::Error)]
pub enum RotationError {
    #[error("no segment sealer is installed: {0}")]
    Unsealed(String),
    #[error("this Transparency Log has no segment directory")]
    NoSegmentDirectory,
    #[error("audit store: {0}")]
    Audit(#[from] AuditError),
    #[error("the segment sealer refused: {0}")]
    Sealer(String),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("live rows changed under the rotation after {0} attempts; nothing was deleted")]
    LiveRowsChanged(u32),
    #[error("audit_keep_days = {0} is below the {KEEP_FLOOR_DAYS}-day expiry floor")]
    KeepBelowFloor(u64),
    #[error("segment {0} is unavailable: {1}")]
    SegmentUnavailable(u64, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationOutcome {
    Sealed(SegmentRef),
    /// No row is both old enough (policy) and below every floor.
    NothingToSeal {
        cut: i64,
    },
    /// An erasure sweep is in flight: sealing now could move a frame it has yet to scrub.
    Deferred,
}

/// `<16 hex>:<16 hex>:…` — the journal redacts plain 32+ hex runs.
pub fn group_hex(text: &str) -> String {
    text.as_bytes()
        .chunks(16)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

fn io_error(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

fn placeholders(count: usize) -> String {
    (1..=count)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn read_values(row: &rusqlite::Row<'_>, count: usize) -> rusqlite::Result<Vec<Value>> {
    (0..count).map(|i| row.get::<_, Value>(i)).collect()
}

/// The registry of a live connection (empty when the table does not exist yet).
pub(super) fn read_registry(conn: &Connection) -> Result<Vec<SegmentRef>, AuditError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' \
             AND name='transparency_log_segments')",
            [],
            |row| row.get(0),
        )
        .map_err(AuditError::SqliteRead)?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = conn
        .prepare(
            "SELECT seq, after_rowid, first_rowid, last_rowid, row_count, min_timestamp_ns, \
             max_timestamp_ns, manifest_sha256, prev_manifest_sha256, sealed_at_ns, expired_at_ns \
             FROM transparency_log_segments ORDER BY seq",
        )
        .map_err(AuditError::SqliteRead)?;
    let rows = statement
        .query_map([], |row| {
            Ok(SegmentRef {
                seq: row.get::<_, i64>(0)? as u64,
                after_rowid: row.get(1)?,
                first_rowid: row.get(2)?,
                last_rowid: row.get(3)?,
                row_count: row.get::<_, i64>(4)? as u64,
                min_timestamp_ns: row.get(5)?,
                max_timestamp_ns: row.get(6)?,
                manifest_sha256: row.get(7)?,
                prev_manifest_sha256: row.get(8)?,
                sealed_at_ns: row.get(9)?,
                expired_at_ns: row.get(10)?,
            })
        })
        .map_err(AuditError::SqliteRead)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AuditError::SqliteRead)
}

struct SealedRow {
    id: [u8; 16],
    kind: i64,
    pid: u32,
    boot: u64,
    body: Vec<u8>,
    ts: i64,
    intent: String,
}

impl SealedRow {
    fn hex(&self) -> String {
        format!("{:032x}", u128::from_be_bytes(self.id))
    }
}

impl TransparencyLogAdapter {
    /// The directory holding this log's sealed segments, when the log is file-backed.
    pub fn segments_dir(&self) -> Option<&Path> {
        self.segments_dir.as_deref()
    }

    /// Install the one segment sealer (composition root). `Err` if one is already installed.
    pub fn install_segment_sealer(
        &self,
        sealer: std::sync::Arc<dyn SegmentSealer>,
    ) -> Result<(), SealerAlreadyInstalled> {
        self.sealer.set(sealer).map_err(|_| SealerAlreadyInstalled)
    }

    /// Whether a segment sealer is installed (rotation is refused until one is).
    pub fn has_segment_sealer(&self) -> bool {
        self.sealer.get().is_some()
    }

    /// The highest insertion id the live log holds (0 when empty).
    pub fn max_insertion_id(&self) -> Result<i64, AuditError> {
        self.frame_high_water()
    }

    /// Every registry row, expired ones included, oldest first.
    pub fn segment_registry(&self) -> Result<Vec<SegmentRef>, AuditError> {
        read_registry(&self.inner.lock().conn)
    }

    /// How many ids the sealed-id table holds (peer-replay guard, D-22-6-09 A).
    pub fn sealed_id_count(&self) -> Result<u64, AuditError> {
        self.inner
            .lock()
            .conn
            .query_row(
                "SELECT count(*) FROM transparency_log_sealed_ids",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n as u64)
            .map_err(AuditError::SqliteRead)
    }

    pub(super) fn is_sealed_id(conn: &Connection, frame_id: &[u8; 16]) -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM transparency_log_sealed_ids WHERE frame_id = ?1)",
            [frame_id.as_slice()],
            |row| row.get(0),
        )
    }

    fn live_segments_on(&self, conn: &Connection) -> Result<Vec<SegmentRef>, AuditError> {
        Ok(read_registry(conn)?
            .into_iter()
            .filter(|segment| segment.expired_at_ns.is_none())
            .collect())
    }

    fn open_segment(&self, segment: &SegmentRef) -> Result<Connection, AuditError> {
        let dir = self.segments_dir.as_ref().ok_or_else(|| {
            AuditError::Io(io_error("this Transparency Log has no segment directory"))
        })?;
        Connection::open_with_flags(
            dir.join(segment.file_name()),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|error| {
            AuditError::Io(io_error(format!(
                "sealed segment {} is unavailable: {error}",
                segment.seq
            )))
        })
    }

    /// Run one frame query over every sealed segment and the live log and merge: insertion
    /// order is the segment order (ids are global), time order is `(timestamp_ns, frame_id)`
    /// over the union, so a clock step across a segment boundary is still ordered correctly.
    pub(super) fn merge_sources<T>(
        &self,
        conn: &Connection,
        mut filter: FrameFilter,
        run: impl Fn(&Connection, FrameFilter) -> Result<Vec<T>, AuditError>,
        time_key: impl Fn(&T) -> (u64, [u8; 16]),
    ) -> Result<Vec<T>, AuditError> {
        let segments = self.live_segments_on(conn)?;
        if segments.is_empty() {
            return run(conn, filter);
        }
        if filter.order_by_insertion {
            if let Some(anchor) = filter.cursor_frame_id.take() {
                // `rowid > (SELECT rowid … WHERE frame_id = ?)`: an unknown anchor matches nothing.
                let Some(rowid) = self.rowid_of(conn, &segments, &anchor)? else {
                    return Ok(Vec::new());
                };
                filter.cursor_insertion_id =
                    Some(filter.cursor_insertion_id.map_or(rowid, |c| c.max(rowid)));
            }
        }
        let mut parts: Vec<Vec<T>> = Vec::with_capacity(segments.len() + 1);
        for segment in &segments {
            if filter
                .cursor_insertion_id
                .is_some_and(|cursor| segment.last_rowid <= cursor)
            {
                continue;
            }
            let sealed = self.open_segment(segment)?;
            parts.push(run(&sealed, filter.clone())?);
        }
        let limit = filter.limit;
        let descending = filter.descending;
        let insertion = filter.order_by_insertion;
        parts.push(run(conn, filter)?);
        let mut merged: Vec<T> = Vec::new();
        if insertion {
            if descending {
                parts.reverse();
            }
            parts
                .into_iter()
                .for_each(|mut part| merged.append(&mut part));
        } else {
            parts
                .into_iter()
                .for_each(|mut part| merged.append(&mut part));
            merged.sort_by_key(|item| time_key(item));
            if descending {
                merged.reverse();
            }
        }
        if let Some(limit) = limit {
            merged.truncate(limit);
        }
        Ok(merged)
    }

    fn rowid_of(
        &self,
        live: &Connection,
        segments: &[SegmentRef],
        frame_id: &[u8; 16],
    ) -> Result<Option<i64>, AuditError> {
        let find = |conn: &Connection| -> Result<Option<i64>, AuditError> {
            conn.query_row(
                "SELECT rowid FROM transparency_log WHERE frame_id = ?1",
                [frame_id.as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(AuditError::SqliteRead)
        };
        if let Some(rowid) = find(live)? {
            return Ok(Some(rowid));
        }
        for segment in segments {
            if let Some(rowid) = find(&self.open_segment(segment)?)? {
                return Ok(Some(rowid));
            }
        }
        Ok(None)
    }

    /// Run `lookup` against the live log, then each sealed segment, until one answers.
    pub(super) fn first_source<T>(
        &self,
        conn: &Connection,
        lookup: impl Fn(&Connection) -> Result<Option<T>, AuditError>,
    ) -> Result<Option<T>, AuditError> {
        if let Some(found) = lookup(conn)? {
            return Ok(Some(found));
        }
        for segment in self.live_segments_on(conn)? {
            if let Some(found) = lookup(&self.open_segment(&segment)?)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Every sealed segment connection, oldest first (read-only).
    pub(super) fn sealed_connections(
        &self,
        conn: &Connection,
    ) -> Result<Vec<Connection>, AuditError> {
        self.live_segments_on(conn)?
            .iter()
            .map(|segment| self.open_segment(segment))
            .collect()
    }

    fn journal(&self, intent: &str, payload: serde_json::Value) {
        let _ = self.insert_frame_event(
            FrameKind::TelemetryEvent,
            0,
            None,
            intent,
            payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );
    }

    fn gate(&self) -> std::sync::MutexGuard<'_, ()> {
        self.rotation_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    // ── policy ───────────────────────────────────────────────────────────────

    /// The policy cut: the highest insertion id the policy would seal (`i64::MAX` = everything
    /// the floors allow), or `None` when no trigger fired. `rotate_bytes` compares USED live
    /// bytes (pages minus the free list): a rotated file does not shrink, so file size would
    /// re-trigger forever.
    pub fn rotation_due(
        &self,
        rotate_bytes: Option<u64>,
        rotate_days: Option<u64>,
        now_ns: u64,
    ) -> Result<Option<i64>, AuditError> {
        let inner = self.inner.lock();
        let conn = &inner.conn;
        let last_sealed = read_registry(conn)?
            .iter()
            .map(|segment| segment.last_rowid)
            .max()
            .unwrap_or(0);
        let mut cut: Option<i64> = None;
        if let Some(limit) = rotate_bytes {
            let pragma = |name: &str| -> Result<i64, AuditError> {
                conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
                    .map_err(AuditError::SqliteRead)
            };
            let used = (pragma("page_count")? - pragma("freelist_count")?).max(0) as u64
                * pragma("page_size")?.max(0) as u64;
            if used >= limit {
                cut = Some(i64::MAX);
            }
        }
        if let Some(days) = rotate_days {
            let threshold = now_ns.saturating_sub(days.saturating_mul(DAY_NS)) as i64;
            // The oldest live row decides cheaply: when it is newer than the threshold, no
            // prefix is old enough and the full scan below is skipped.
            let oldest: Option<i64> = conn
                .query_row(
                    "SELECT timestamp_ns FROM transparency_log ORDER BY rowid LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(AuditError::SqliteRead)?;
            if oldest.is_some_and(|oldest| oldest < threshold) {
                let aged: i64 = conn
                    .query_row(
                        "SELECT coalesce(min(rowid) - 1, (SELECT coalesce(max(rowid), 0) FROM transparency_log)) \
                         FROM transparency_log WHERE timestamp_ns >= ?1",
                        [threshold],
                        |row| row.get(0),
                    )
                    .map_err(AuditError::SqliteRead)?;
                if aged > last_sealed {
                    cut = Some(cut.map_or(aged, |current| current.max(aged)));
                }
            }
        }
        Ok(cut)
    }

    /// The highest insertion id rotation may seal on the adapter's own evidence: below the
    /// newest row, below the oldest open `principal.forget.intent`, below the oldest
    /// outstanding redistill request and below the newest completed-forget marker.
    pub fn rotation_floor(&self) -> Result<i64, AuditError> {
        let mut floor = self.frame_high_water()? - 1;
        let kernel = FrameOrigin::Kernel as i64;
        let open_intent: Option<i64> = {
            let inner = self.inner.lock();
            inner
                .conn
                .query_row(
                    "SELECT min(i.rowid) FROM transparency_log i INDEXED BY idx_tlog_forget_intent \
                     WHERE i.intent = 'principal.forget.intent' AND i.origin = ?1 \
                     AND NOT EXISTS (SELECT 1 FROM transparency_log f INDEXED BY idx_tlog_forget_done \
                       WHERE f.intent = 'principal.forget' AND f.origin = ?1 AND f.rowid > i.rowid \
                       AND CASE WHEN json_valid(CAST(f.payload_redacted AS TEXT),2) \
                         THEN json_extract(CAST(f.payload_redacted AS TEXT),'$.completed_from_intent') END \
                         = substr(lower(hex(i.frame_id)),1,16)||':'||substr(lower(hex(i.frame_id)),17))",
                    [kernel],
                    |row| row.get(0),
                )
                .map_err(AuditError::SqliteRead)?
        };
        let mut oldest_request: Option<i64> = None;
        self.scan_rowid_windows(OUTSTANDING_REDISTILL_SQL, None, |row| {
            let rowid: i64 = row.get(3).map_err(AuditError::SqliteRead)?;
            oldest_request = Some(oldest_request.map_or(rowid, |current| current.min(rowid)));
            Ok(())
        })?;
        let marker: Option<i64> = {
            let inner = self.inner.lock();
            inner
                .conn
                .query_row(
                    "SELECT max(rowid) FROM transparency_log \
                     WHERE intent = 'principal.uninstall.frame_sweep_complete' AND origin = ?1",
                    [kernel],
                    |row| row.get(0),
                )
                .map_err(AuditError::SqliteRead)?
        };
        for protected in [open_intent, oldest_request, marker].into_iter().flatten() {
            floor = floor.min(protected - 1);
        }
        Ok(floor)
    }

    // ── rotation ─────────────────────────────────────────────────────────────

    /// Seal the oldest prefix: rows with `rowid <= min(policy_cut, external_floor, rotation_floor)`.
    ///
    /// `external_floor` is the minimum of the committed cursors of every in-process tail
    /// (`i64::MAX` when none registered). The cut is computed ONCE here and is the only bound
    /// the copy, the verification and the delete use.
    pub fn rotate_through(
        &self,
        policy_cut: i64,
        external_floor: i64,
    ) -> Result<RotationOutcome, RotationError> {
        let _gate = self.gate();
        if self
            .active_erasures
            .load(std::sync::atomic::Ordering::SeqCst)
            > 0
        {
            return Ok(RotationOutcome::Deferred);
        }
        let sealer = self
            .sealer
            .get()
            .ok_or_else(|| RotationError::Unsealed("no audit signing key was loaded".into()))?
            .clone();
        let dir = self
            .segments_dir
            .clone()
            .ok_or(RotationError::NoSegmentDirectory)?;
        let registry = self.segment_registry()?;
        let last_sealed = registry.iter().map(|s| s.last_rowid).max().unwrap_or(0);
        let wanted = policy_cut.min(external_floor).min(self.rotation_floor()?);
        let cut: Option<i64> = if wanted > last_sealed {
            self.inner
                .lock()
                .conn
                .query_row(
                    "SELECT max(rowid) FROM transparency_log WHERE rowid > ?1 AND rowid <= ?2",
                    [last_sealed, wanted],
                    |row| row.get(0),
                )
                .map_err(AuditError::SqliteRead)?
        } else {
            None
        };
        let Some(cut) = cut else {
            return Ok(RotationOutcome::NothingToSeal { cut: wanted });
        };
        let seq = registry.iter().map(|s| s.seq).max().unwrap_or(0) + 1;
        let prev = registry.last().map(|s| s.manifest_sha256.clone());
        std::fs::create_dir_all(&dir)?;
        let segment_path = dir.join(format!("{seq:010}.sqlite"));
        let manifest_path = dir.join(format!("{seq:010}.manifest.json"));
        let partial = dir.join(format!("{seq:010}.sqlite.partial"));
        for _ in 1..=RECOPY_ATTEMPTS {
            let _ = std::fs::remove_file(&partial);
            let stats = self.copy_prefix(&partial, last_sealed, cut)?;
            std::fs::rename(&partial, &segment_path)?;
            let request = SegmentSealRequest {
                seq,
                segment_path: segment_path.clone(),
                file_name: format!("{seq:010}.sqlite"),
                after_rowid: last_sealed,
                first_rowid: stats.first,
                last_rowid: stats.last,
                row_count: stats.rows,
                min_timestamp_ns: stats.min_ts,
                max_timestamp_ns: stats.max_ts,
                prev_manifest_sha256: prev.clone(),
                replaces_manifest: None,
                scrubbed_frame_ids: Vec::new(),
            };
            let sealed = sealer.seal(&request).map_err(RotationError::Sealer)?;
            let staged = dir.join(format!("{seq:010}.manifest.json.partial"));
            write_synced(&staged, &sealed.manifest)?;
            std::fs::rename(&staged, &manifest_path)?;
            let record = SegmentRef {
                seq,
                after_rowid: last_sealed,
                first_rowid: stats.first,
                last_rowid: stats.last,
                row_count: stats.rows,
                min_timestamp_ns: stats.min_ts,
                max_timestamp_ns: stats.max_ts,
                manifest_sha256: sealed.manifest_sha256.clone(),
                prev_manifest_sha256: prev.clone(),
                sealed_at_ns: wall_clock_now_ns() as i64,
                expired_at_ns: None,
            };
            if self.delete_sealed_prefix(&segment_path, &record, cut)? {
                return Ok(RotationOutcome::Sealed(record));
            }
        }
        let _ = std::fs::remove_file(&segment_path);
        let _ = std::fs::remove_file(&manifest_path);
        Err(RotationError::LiveRowsChanged(RECOPY_ATTEMPTS))
    }

    /// Copy `(after, cut]` window by window; the writer lock is held for one window at a time.
    fn copy_prefix(&self, path: &Path, after: i64, cut: i64) -> Result<CopyStats, RotationError> {
        let sealed = Connection::open(path)?;
        sealed.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")?;
        sealed.execute_batch(SEGMENT_SCHEMA_SQL)?;
        let log_insert = format!(
            "INSERT INTO transparency_log(rowid, {LOG_COLUMNS}) VALUES ({})",
            placeholders(LOG_COLUMN_COUNT + 1)
        );
        let mut cursor = after;
        loop {
            let window = {
                let inner = self.inner.lock();
                let conn = &inner.conn;
                let upto: Option<i64> = conn.query_row(
                    "SELECT max(rowid) FROM (SELECT rowid FROM transparency_log \
                     WHERE rowid > ?1 AND rowid <= ?2 ORDER BY rowid LIMIT ?3)",
                    rusqlite::params![cursor, cut, COPY_WINDOW],
                    |row| row.get(0),
                )?;
                let Some(upto) = upto else { break };
                let mut rows = Vec::new();
                {
                    let mut statement = conn.prepare(&format!(
                        "SELECT rowid, {LOG_COLUMNS} FROM transparency_log \
                         WHERE rowid > ?1 AND rowid <= ?2 ORDER BY rowid"
                    ))?;
                    let mut query = statement.query(rusqlite::params![cursor, upto])?;
                    while let Some(row) = query.next()? {
                        rows.push(read_values(row, LOG_COLUMN_COUNT + 1)?);
                    }
                }
                let mut sides: Vec<(&str, usize, Vec<Vec<Value>>)> = Vec::new();
                for (table, columns, key) in SIDE_TABLES {
                    let count = columns.split(',').count();
                    let mut statement = conn.prepare(&format!(
                        "SELECT {columns} FROM {table} WHERE {key} IN \
                         (SELECT frame_id FROM transparency_log WHERE rowid > ?1 AND rowid <= ?2)"
                    ))?;
                    let mut found = Vec::new();
                    let mut query = statement.query(rusqlite::params![cursor, upto])?;
                    while let Some(row) = query.next()? {
                        found.push(read_values(row, count)?);
                    }
                    sides.push((table, count, found));
                }
                (upto, rows, sides)
            };
            let (upto, rows, sides) = window;
            let transaction = sealed.unchecked_transaction()?;
            for row in &rows {
                transaction.execute(&log_insert, rusqlite::params_from_iter(row.iter()))?;
            }
            for (table, count, found) in &sides {
                let columns = SIDE_TABLES
                    .iter()
                    .find(|(name, _, _)| name == table)
                    .map(|(_, columns, _)| *columns)
                    .unwrap_or_default();
                let insert = format!(
                    "INSERT INTO {table}({columns}) VALUES ({})",
                    placeholders(*count)
                );
                for row in found {
                    transaction.execute(&insert, rusqlite::params_from_iter(row.iter()))?;
                }
            }
            transaction.commit()?;
            cursor = upto;
        }
        let stats = sealed.query_row(
            "SELECT count(*), coalesce(min(rowid),0), coalesce(max(rowid),0), \
             coalesce(min(timestamp_ns),0), coalesce(max(timestamp_ns),0) FROM transparency_log",
            [],
            |row| {
                Ok(CopyStats {
                    rows: row.get::<_, i64>(0)? as u64,
                    first: row.get(1)?,
                    last: row.get(2)?,
                    min_ts: row.get(3)?,
                    max_ts: row.get(4)?,
                })
            },
        )?;
        drop(sealed);
        std::fs::File::open(path)?.sync_all()?;
        Ok(stats)
    }

    /// Under the writer lock: re-verify every copied row against its live twin, then delete the
    /// prefix, its side rows, record the sealed ids and the registry row, in ONE transaction.
    /// `Ok(false)` means a copied row no longer equals live (nothing was deleted).
    fn delete_sealed_prefix(
        &self,
        segment_path: &Path,
        record: &SegmentRef,
        cut: i64,
    ) -> Result<bool, RotationError> {
        let mut inner = self.inner.lock();
        let TransparencyLogInner { conn, .. } = &mut *inner;
        conn.execute(
            "ATTACH DATABASE ?1 AS rot_seg",
            [segment_path.to_string_lossy().as_ref()],
        )?;
        let outcome = (|| -> Result<bool, RotationError> {
            let transaction =
                conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let after = record.after_rowid;
            let live_rows: i64 = transaction.query_row(
                "SELECT count(*) FROM main.transparency_log WHERE rowid > ?1 AND rowid <= ?2",
                [after, cut],
                |row| row.get(0),
            )?;
            let sealed_rows: i64 = transaction.query_row(
                "SELECT count(*) FROM rot_seg.transparency_log",
                [],
                |row| row.get(0),
            )?;
            let differing: i64 = transaction.query_row(
                "SELECT count(*) FROM main.transparency_log m JOIN rot_seg.transparency_log s ON s.rowid = m.rowid \
                 WHERE m.rowid > ?1 AND m.rowid <= ?2 AND (m.frame_id IS NOT s.frame_id \
                 OR m.timestamp_ns IS NOT s.timestamp_ns OR m.spirit_pid IS NOT s.spirit_pid \
                 OR m.from_spirit_id IS NOT s.from_spirit_id OR m.to_spirit_id IS NOT s.to_spirit_id \
                 OR m.boot_nonce IS NOT s.boot_nonce OR m.capability_token IS NOT s.capability_token \
                 OR m.kind IS NOT s.kind OR m.intent IS NOT s.intent \
                 OR m.correlation_id IS NOT s.correlation_id \
                 OR m.payload_redacted IS NOT s.payload_redacted OR m.origin IS NOT s.origin)",
                [after, cut],
                |row| row.get(0),
            )?;
            if live_rows != sealed_rows || live_rows as u64 != record.row_count || differing != 0 {
                transaction.rollback()?;
                return Ok(false);
            }
            transaction.execute(
                "INSERT INTO transparency_log_sealed_ids(frame_id, segment_seq) \
                 SELECT frame_id, ?3 FROM main.transparency_log WHERE rowid > ?1 AND rowid <= ?2",
                rusqlite::params![after, cut, record.seq as i64],
            )?;
            // Side rows first: they are keyed by the frames the next statement removes.
            for (table, _, key) in &SIDE_TABLES[1..] {
                transaction.execute(
                    &format!(
                        "DELETE FROM {table} WHERE {key} IN \
                         (SELECT frame_id FROM main.transparency_log WHERE rowid > ?1 AND rowid <= ?2)"
                    ),
                    [after, cut],
                )?;
            }
            // A retraction that arrived after the copy stays live: remove only copied rows.
            transaction.execute(
                "DELETE FROM main.transparency_log_retractions WHERE EXISTS (SELECT 1 FROM \
                 rot_seg.transparency_log_retractions s WHERE s.original_frame_id = \
                 main.transparency_log_retractions.original_frame_id AND s.retract_frame_id = \
                 main.transparency_log_retractions.retract_frame_id)",
                [],
            )?;
            let deleted = transaction.execute(
                "DELETE FROM main.transparency_log WHERE rowid > ?1 AND rowid <= ?2",
                [after, cut],
            )?;
            if deleted as i64 != live_rows {
                transaction.rollback()?;
                return Ok(false);
            }
            transaction.execute(
                "INSERT INTO transparency_log_segments(seq, after_rowid, first_rowid, last_rowid, row_count, \
                 min_timestamp_ns, max_timestamp_ns, manifest_sha256, prev_manifest_sha256, sealed_at_ns) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    record.seq as i64,
                    record.after_rowid,
                    record.first_rowid,
                    record.last_rowid,
                    record.row_count as i64,
                    record.min_timestamp_ns,
                    record.max_timestamp_ns,
                    record.manifest_sha256,
                    record.prev_manifest_sha256,
                    record.sealed_at_ns,
                ],
            )?;
            transaction.commit()?;
            Ok(true)
        })();
        let _ = conn.execute_batch("DETACH DATABASE rot_seg");
        let sealed = outcome?;
        if sealed {
            // Best effort: a busy checkpoint only delays the file-size bound.
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
        }
        Ok(sealed)
    }

    // ── expiry ───────────────────────────────────────────────────────────────

    /// Delete whole sealed segments older than `keep_days` (floor 90). A segment that holds a
    /// frame of a legal-held principal or of a principal with an open erasure is kept. Each
    /// expiry journals `audit.segment.expired` BEFORE the unlink. Returns the expired seqs.
    pub fn expire_segments(&self, keep_days: u64, now_ns: u64) -> Result<Vec<u64>, RotationError> {
        if keep_days < KEEP_FLOOR_DAYS {
            return Err(RotationError::KeepBelowFloor(keep_days));
        }
        let _gate = self.gate();
        let dir = self
            .segments_dir
            .clone()
            .ok_or(RotationError::NoSegmentDirectory)?;
        let threshold = now_ns.saturating_sub(keep_days.saturating_mul(DAY_NS)) as i64;
        let segments = self.segment_registry()?;
        let candidates: Vec<&SegmentRef> = segments
            .iter()
            .filter(|s| s.expired_at_ns.is_none() && s.max_timestamp_ns < threshold)
            .collect();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        // Fail closed: holds that cannot be read, or an open erasure with no readable subject.
        let mut protected: Vec<String> = self
            .list_legal_holds()?
            .into_iter()
            .map(|hold| hold.principal_id)
            .collect();
        let open = self.outstanding_forget_intents(None)?;
        if open.iter().any(|(_, principal, _)| principal.is_none()) {
            return Ok(Vec::new());
        }
        protected.extend(open.into_iter().filter_map(|(_, principal, _)| principal));
        let mut expired = Vec::new();
        for segment in candidates {
            let sealed = self.open_segment(segment)?;
            if protected
                .iter()
                .any(|principal| Self::segment_mentions(&sealed, principal).unwrap_or(true))
            {
                continue;
            }
            drop(sealed);
            self.journal(
                SEGMENT_EXPIRED_INTENT,
                serde_json::json!({
                    "seq": segment.seq,
                    "manifest_sha256": group_hex(&segment.manifest_sha256),
                    "first_rowid": segment.first_rowid,
                    "last_rowid": segment.last_rowid,
                    "rows": segment.row_count,
                }),
            );
            for name in [segment.file_name(), segment.manifest_name()] {
                match std::fs::remove_file(dir.join(name)) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let mut inner = self.inner.lock();
            let transaction = inner
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            transaction.execute(
                "UPDATE transparency_log_segments SET expired_at_ns = ?1 WHERE seq = ?2",
                rusqlite::params![now_ns as i64, segment.seq as i64],
            )?;
            transaction.execute(
                "DELETE FROM transparency_log_sealed_ids WHERE segment_seq = ?1",
                [segment.seq as i64],
            )?;
            transaction.commit()?;
            expired.push(segment.seq);
        }
        Ok(expired)
    }

    fn segment_mentions(sealed: &Connection, principal: &str) -> rusqlite::Result<bool> {
        let mut statement = sealed.prepare(
            "SELECT kind, payload_redacted FROM transparency_log \
             WHERE kind IN (28, 29) OR instr(payload_redacted, CAST(?1 AS BLOB)) > 0",
        )?;
        let mut rows = statement.query([principal])?;
        while let Some(row) = rows.next()? {
            let body: Vec<u8> = row.get(1)?;
            if Self::subject_in_frame(row.get(0)?, &body, principal) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ── forget over sealed segments (D-22-6-07 A) ────────────────────────────

    /// Scrub the principal's frames (and the distillates that cite a scrubbed frame) in every
    /// sealed segment, re-seal each affected segment with `supersedes`, and report the scrubbed
    /// frames through the sweep state exactly as the live pass does.
    pub(super) fn sweep_sealed_segments(&self, state: &mut ErasureSweep) -> Result<(), AuditError> {
        if state.subjects.is_empty() || state.sealed_scanned == Some(state.sources.len()) {
            return Ok(());
        }
        // The gate first: a rotation that is mid-copy finishes (and its segment is then scrubbed
        // here); one that starts later defers while this sweep is counted in `active_erasures`.
        let _gate = self.gate();
        let segments = self.segment_registry()?;
        if segments
            .iter()
            .all(|segment| segment.expired_at_ns.is_some())
        {
            state.sealed_scanned = Some(state.sources.len());
            return Ok(());
        }
        let holds: Vec<String> = self
            .list_legal_holds()?
            .into_iter()
            .map(|hold| hold.principal_id)
            .collect();
        for segment in segments.iter().filter(|s| s.expired_at_ns.is_none()) {
            self.scrub_segment(segment, &holds, state)?;
        }
        state.sealed_scanned = Some(state.sources.len());
        Ok(())
    }

    fn scrub_segment(
        &self,
        segment: &SegmentRef,
        holds: &[String],
        state: &mut ErasureSweep,
    ) -> Result<(), AuditError> {
        let sealed = self.open_segment(segment)?;
        let mut names: Vec<&String> = state.subjects.iter().chain(holds.iter()).collect();
        names.sort();
        names.dedup();
        let mut sql = String::from(
            "SELECT frame_id, kind, spirit_pid, boot_nonce, payload_redacted, timestamp_ns, intent \
             FROM transparency_log WHERE kind IN (11, 28, 29)",
        );
        for index in 0..names.len() {
            sql.push_str(&format!(
                " OR instr(payload_redacted, CAST(?{} AS BLOB)) > 0",
                index + 1
            ));
        }
        let mut rows: Vec<SealedRow> = Vec::new();
        {
            let mut statement = sealed.prepare(&sql).map_err(AuditError::SqliteRead)?;
            let mut query = statement
                .query(rusqlite::params_from_iter(names.iter()))
                .map_err(AuditError::SqliteRead)?;
            while let Some(row) = query.next().map_err(AuditError::SqliteRead)? {
                let bytes: Vec<u8> = row.get(0).map_err(AuditError::SqliteRead)?;
                rows.push(SealedRow {
                    id: bytes
                        .try_into()
                        .map_err(|v: Vec<u8>| AuditError::MalformedFrameId(v.len()))?,
                    kind: row.get(1).map_err(AuditError::SqliteRead)?,
                    pid: row.get::<_, i64>(2).map_err(AuditError::SqliteRead)? as u32,
                    boot: row.get::<_, i64>(3).map_err(AuditError::SqliteRead)? as u64,
                    body: row.get(4).map_err(AuditError::SqliteRead)?,
                    ts: row.get(5).map_err(AuditError::SqliteRead)?,
                    intent: row.get(6).map_err(AuditError::SqliteRead)?,
                });
            }
        }
        drop(sealed);
        let excluded = |row: &SealedRow| {
            matches!(
                row.intent.as_str(),
                "principal.forget"
                    | "principal.forget.held"
                    | "principal.forget.intent"
                    | "distillate.redacted"
            ) || row.intent.starts_with("principal.uninstall.")
        };
        let mentions = |row: &SealedRow, who: &[String]| {
            !excluded(row)
                && who
                    .iter()
                    .any(|p| Self::subject_in_frame(row.kind, &row.body, p))
        };
        let refs_of = |row: &SealedRow| -> Vec<String> {
            if row.kind == FrameKind::Distillate as i64 {
                Self::distillate_source_refs(&row.body).unwrap_or_default()
            } else {
                Vec::new()
            }
        };
        // Seeds, then distillates reaching a scrubbed (or already tombstoned) source.
        let mut scrubbed: HashSet<String> = state.sources.clone();
        let mut chosen: BTreeSet<usize> = BTreeSet::new();
        for (index, row) in rows.iter().enumerate() {
            if row.kind == FrameKind::Distillate as i64 && Self::is_kernel_tombstone(&row.body) {
                scrubbed.insert(row.hex());
            } else if mentions(row, &state.subjects) {
                chosen.insert(index);
                scrubbed.insert(row.hex());
            }
        }
        loop {
            let mut grew = false;
            for (index, row) in rows.iter().enumerate() {
                if chosen.contains(&index) || row.kind != FrameKind::Distillate as i64 {
                    continue;
                }
                if refs_of(row).iter().any(|id| scrubbed.contains(id)) {
                    chosen.insert(index);
                    scrubbed.insert(row.hex());
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        // A frame of a held principal, and anything derived from it, stays.
        let mut protected: HashSet<String> = rows
            .iter()
            .filter(|row| mentions(row, holds))
            .map(SealedRow::hex)
            .collect();
        loop {
            let mut grew = false;
            for row in &rows {
                if row.kind == FrameKind::Distillate as i64
                    && !protected.contains(&row.hex())
                    && refs_of(row).iter().any(|id| protected.contains(id))
                {
                    protected.insert(row.hex());
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        let mut targets: Vec<&SealedRow> = Vec::new();
        for index in chosen {
            let row = &rows[index];
            if protected.contains(&row.hex()) {
                state.held.push((row.ts, row.hex()));
            } else {
                targets.push(row);
            }
        }
        if targets.is_empty() {
            return Ok(());
        }
        targets.sort_by_key(|row| (row.ts, row.id));
        let reason = "gdpr-forget";
        let mut replacements: Vec<([u8; 16], Vec<u8>)> = Vec::new();
        for row in &targets {
            let body = if matches!(row.kind, 28 | 29) {
                Self::redacted_principals_body(
                    row.kind,
                    &row.body,
                    &state.subjects,
                    reason,
                    row.id,
                )?
            } else {
                let kind = if row.kind == 11 {
                    serde_json::json!("Distillate")
                } else {
                    serde_json::json!(row.kind)
                };
                serde_json::to_vec(&serde_json::json!({ "redacted": true, "reason": reason, "original_kind": kind }))
                    .map_err(|error| AuditError::Serialization(error.to_string()))?
            };
            replacements.push((row.id, body));
        }
        let mut requests = Vec::new();
        for row in targets.iter().filter(|row| row.kind == 11) {
            requests.push(RedistillRequest {
                redacted_distillate_frame_id: row.hex(),
                writer_spirit_pid: row.pid,
                writer_boot_nonce: (row.boot as u128).to_le_bytes(),
                surviving_source_log_ref: refs_of(row)
                    .into_iter()
                    .filter(|id| !scrubbed.contains(id))
                    .collect(),
            });
        }
        // Durable requests before the scrub: a lost request cannot be rediscovered afterwards.
        for request in &requests {
            self.insert_redistill_request(request)?;
        }
        let scrubbed_ids: Vec<String> = targets.iter().map(|row| row.hex()).collect();
        self.reseal_segment(segment, &replacements, &scrubbed_ids)
            .map_err(|error| match error {
                RotationError::Audit(audit) => audit,
                other => AuditError::Io(io_error(format!(
                    "re-sealing segment {} failed: {other}",
                    segment.seq
                ))),
            })?;
        for row in &targets {
            let id = row.hex();
            // The live pass's marker shape: the colon-grouped id form.
            let shown = format_frame_id_hex(&row.id);
            let marker = match (row.kind, state.subjects.first()) {
                (11, Some(principal)) if state.subjects.len() == 1 => {
                    serde_json::json!({"principal_id": principal, "redacted_distillate_frame_id": shown})
                }
                _ => serde_json::json!({"redacted_frame_id": shown, "original_kind": row.kind}),
            };
            let marker = serde_json::to_vec(&marker)
                .map_err(|error| AuditError::Serialization(error.to_string()))?;
            {
                let mut inner = self.inner.lock();
                let TransparencyLogInner {
                    conn,
                    next_frame_id_counter,
                    boot_nonce,
                    last_frame_id,
                    ..
                } = &mut *inner;
                *last_frame_id = Self::append_erasure_row(
                    conn,
                    next_frame_id_counter,
                    *boot_nonce,
                    FrameKind::TaskComplete,
                    if row.kind == 11 {
                        "distillate.redacted"
                    } else {
                        "frame.redacted"
                    },
                    &marker,
                )?;
            }
            state.sources.insert(id.clone());
            if row.kind == 11 {
                state.distillates.push((row.ts, id));
            } else {
                state.principal_frames.push((row.ts, id));
            }
        }
        state.requests.extend(requests);
        Ok(())
    }

    /// Replace the bodies of `replacements` in a copy of the segment, sign a manifest that
    /// supersedes the old one and names the scrubbed ids, journal `audit.segment.resealed`,
    /// then swap the files. The old file is gone only after the new one verified its digests.
    fn reseal_segment(
        &self,
        segment: &SegmentRef,
        replacements: &[([u8; 16], Vec<u8>)],
        scrubbed_ids: &[String],
    ) -> Result<(), RotationError> {
        let sealer = self
            .sealer
            .get()
            .ok_or_else(|| RotationError::Unsealed("no audit signing key was loaded".into()))?
            .clone();
        let dir = self
            .segments_dir
            .clone()
            .ok_or(RotationError::NoSegmentDirectory)?;
        let current = dir.join(segment.file_name());
        let manifest_path = dir.join(segment.manifest_name());
        let staged = dir.join(format!("{}.resealing", segment.file_name()));
        let _ = std::fs::remove_file(&staged);
        std::fs::copy(&current, &staged)?;
        {
            let copy = Connection::open(&staged)?;
            let transaction = copy.unchecked_transaction()?;
            for (id, body) in replacements {
                let changed = transaction.execute(
                    "UPDATE transparency_log SET payload_redacted = ?1 WHERE frame_id = ?2",
                    rusqlite::params![body, id.as_slice()],
                )?;
                if changed != 1 {
                    return Err(RotationError::SegmentUnavailable(
                        segment.seq,
                        "a scrub target is missing from the segment".into(),
                    ));
                }
            }
            transaction.commit()?;
        }
        std::fs::File::open(&staged)?.sync_all()?;
        let sealed = sealer
            .seal(&SegmentSealRequest {
                seq: segment.seq,
                segment_path: staged.clone(),
                file_name: segment.file_name(),
                after_rowid: segment.after_rowid,
                first_rowid: segment.first_rowid,
                last_rowid: segment.last_rowid,
                row_count: segment.row_count,
                min_timestamp_ns: segment.min_timestamp_ns,
                max_timestamp_ns: segment.max_timestamp_ns,
                prev_manifest_sha256: segment.prev_manifest_sha256.clone(),
                replaces_manifest: Some(manifest_path.clone()),
                scrubbed_frame_ids: scrubbed_ids.to_vec(),
            })
            .map_err(RotationError::Sealer)?;
        self.journal(
            SEGMENT_RESEALED_INTENT,
            serde_json::json!({
                "seq": segment.seq,
                "superseded_manifest_sha256": group_hex(&segment.manifest_sha256),
                "manifest_sha256": group_hex(&sealed.manifest_sha256),
                "scrubbed_frames": scrubbed_ids.len(),
            }),
        );
        let staged_manifest = dir.join(format!("{}.resealing", segment.manifest_name()));
        write_synced(&staged_manifest, &sealed.manifest)?;
        std::fs::rename(&staged, &current)?;
        std::fs::rename(&staged_manifest, &manifest_path)?;
        self.inner.lock().conn.execute(
            "UPDATE transparency_log_segments SET manifest_sha256 = ?1 WHERE seq = ?2",
            rusqlite::params![sealed.manifest_sha256, segment.seq as i64],
        )?;
        Ok(())
    }

    /// Every id the segment registry accounts for, merged with the live ids, in the order
    /// `all_frame_ids` promises (`timestamp_ns`, `frame_id`).
    pub(super) fn all_ids_on(conn: &Connection) -> Result<Vec<(i64, [u8; 16])>, AuditError> {
        let mut statement = conn
            .prepare("SELECT timestamp_ns, frame_id FROM transparency_log")
            .map_err(AuditError::SqliteRead)?;
        let rows = statement
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(1)?;
                // P8: a corrupt/non-16-byte row must not panic the cascade.
                let id: [u8; 16] = bytes.as_slice().try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        16,
                        rusqlite::types::Type::Blob,
                        format!(
                            "transparency_log frame_id is {} bytes, expected 16",
                            bytes.len()
                        )
                        .into(),
                    )
                })?;
                Ok((row.get::<_, i64>(0)?, id))
            })
            .map_err(AuditError::SqliteRead)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuditError::SqliteRead)
    }
}

struct CopyStats {
    rows: u64,
    first: i64,
    last: i64,
    min_ts: i64,
    max_ts: i64,
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
