//! Story 23-4 — one cursor-driven export tick (D-23-4-03, -04, -06, -07, -08).
//!
//! A tick reads the quiesced log after the durable cursor, hands one bounded batch at a time
//! to a [`BatchSink`] and moves the cursor only after the sink acknowledged that batch. The
//! log itself is the spool: nothing is buffered beyond one batch, and a sink that stays down
//! leaves the cursor where it was.

use std::path::Path;
use std::sync::Arc;

use maos_audit::insertion::LogView;
use maos_domain::ports::SiemProjectionPort;

use crate::cursor::{Cursor, CursorStore, Loaded, ResetReason};

/// At most this many frames per batch (and so per TLS connection).
pub const MAX_BATCH_FRAMES: usize = 512;
/// At most this many bytes of frames per batch (a single larger frame still goes alone).
pub const MAX_BATCH_BYTES: usize = 1 << 20;

/// One row ready to leave the host: the same redacted projection the file sink writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub insertion_id: i64,
    /// Hex `frame_id`: the receiver's dedupe key (CEF `externalId`).
    pub frame_id_hex: String,
    /// The redacted audit entry as JSON (what [`SiemProjectionPort`] projects).
    pub ndjson: String,
    /// The RFC 5424 syslog message.
    pub syslog: String,
}

/// Why a sink did not acknowledge a batch. `code` is the typed, value-free reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkFailure {
    pub code: &'static str,
}

impl SinkFailure {
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

impl std::fmt::Display for SinkFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}

/// Somewhere a batch of frames goes.
pub trait BatchSink {
    /// What identifies the sink in the cursor (`tls://host:port`, `file:<path>`).
    fn identity(&self) -> String;
    /// Deliver `frames` and return only once the sink acknowledged all of them.
    ///
    /// # Errors
    /// Why the batch is not acknowledged; the caller re-sends it next tick.
    fn deliver(&mut self, frames: &[Frame]) -> Result<(), SinkFailure>;
}

/// The localhost file sink: acknowledged when the lines are written and synced.
pub struct FileSink {
    path: std::path::PathBuf,
    port: Arc<dyn SiemProjectionPort>,
}

impl FileSink {
    #[must_use]
    pub fn new(path: std::path::PathBuf, port: Arc<dyn SiemProjectionPort>) -> Self {
        Self { path, port }
    }
}

impl BatchSink for FileSink {
    fn identity(&self) -> String {
        format!("file:{}", self.path.display())
    }

    fn deliver(&mut self, frames: &[Frame]) -> Result<(), SinkFailure> {
        use std::io::Write;
        let mut out = Vec::new();
        for frame in frames {
            let line = self
                .port
                .project_redacted_entry(&frame.ndjson)
                .map_err(|_| SinkFailure::new("siem_projection_failed"))?;
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|_| SinkFailure::new("siem_sink_file_io"))?;
        file.write_all(&out)
            .and_then(|()| file.sync_data())
            .map_err(|_| SinkFailure::new("siem_sink_file_io"))
    }
}

/// Why a tick stopped. None of these moves the cursor.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The log could not be read (never a gap: a registered segment that is missing is this).
    #[error("siem_source_unavailable: {0}")]
    Source(#[from] maos_audit::AuditError),
    /// The sink did not acknowledge a batch; `buffered` rows lie after the cursor.
    #[error("{failure}; {buffered} record(s) remain after the cursor")]
    Sink { failure: SinkFailure, buffered: u64 },
    /// A `siem.gap` or `siem.cursor.reset` row could not be journaled.
    #[error("siem_journal_failed: {0}")]
    Journal(String),
    /// The cursor file could not be written or moved aside.
    #[error("siem_cursor_io: {0}")]
    Cursor(#[source] std::io::Error),
    #[error("siem_projection_failed: {0}")]
    Projection(String),
}

impl ExportError {
    /// The typed, value-free reason (one line per distinct reason in the consumer).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Source(_) => "siem_source_unavailable",
            Self::Sink { failure, .. } => failure.code,
            Self::Journal(_) => "siem_journal_failed",
            Self::Cursor(_) => "siem_cursor_io",
            Self::Projection(_) => "siem_projection_failed",
        }
    }

    /// Rows left after the cursor when the sink refused (0 for every other reason).
    #[must_use]
    pub fn buffered(&self) -> u64 {
        match self {
            Self::Sink { buffered, .. } => *buffered,
            _ => 0,
        }
    }
}

/// What one tick did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tick {
    /// Rows acknowledged by the sink.
    pub delivered: usize,
    pub batches: usize,
    /// Typed lines for the operator (gaps, resets), in the order they happened.
    pub notes: Vec<String>,
}

/// Journals one operator-relevant event (`siem.gap`, `siem.cursor.reset`) into the live log.
pub type Journal<'a> = &'a dyn Fn(&str, serde_json::Value) -> Result<(), String>;

/// Run one tick against `source` (a quiesced copy of the log: see `maos-audit`'s
/// `insertion` module) until the log is exhausted or a batch is not acknowledged.
///
/// # Errors
/// [`ExportError`]; the cursor stays at the last acknowledged batch.
pub fn run_tick(
    source: &Path,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
) -> Result<Tick, ExportError> {
    let mut tick = Tick::default();
    let view = LogView::open(source)?;
    let mut after = match store.load() {
        Loaded::Absent => 0,
        Loaded::Valid(cursor) => match check_anchor(&view, &cursor)? {
            None => cursor.insertion_id,
            Some(reason) => {
                reset(store, journal, reason, &mut tick)?;
                0
            }
        },
        Loaded::Rejected(reason) => {
            reset(store, journal, reason, &mut tick)?;
            0
        }
    };

    // D-23-4-06 A: rows of an expired segment are gone. Journal the range, then move past it.
    for range in view.expired_ranges_after(after)? {
        let from = range.first_rowid.max(after.saturating_add(1));
        if from > range.last_rowid {
            continue;
        }
        journal(
            "siem.gap",
            serde_json::json!({
                "from": from,
                "to": range.last_rowid,
                "segment_seq": range.seq,
            }),
        )
        .map_err(ExportError::Journal)?;
        after = range.last_rowid;
        store
            .save(&Cursor {
                insertion_id: after,
                frame_id: None,
            })
            .map_err(ExportError::Cursor)?;
        tick.notes.push(format!(
            "siem_gap: ids {from}..={} of expired segment {} were never delivered",
            range.last_rowid, range.seq
        ));
    }

    loop {
        let rows = view.read_after(after, MAX_BATCH_FRAMES)?;
        let mut frames = Vec::with_capacity(rows.len());
        let mut bytes = 0_usize;
        for row in rows {
            let record = crate::project_one(&row.entry)
                .map_err(|error| ExportError::Projection(error.to_string()))?;
            if !frames.is_empty() && bytes + record.rfc5424.len() > MAX_BATCH_BYTES {
                break;
            }
            bytes += record.rfc5424.len();
            frames.push(Frame {
                insertion_id: row.insertion_id,
                frame_id_hex: row.entry.frame_id_hex,
                ndjson: record.ndjson,
                syslog: record.rfc5424,
            });
        }
        let Some(last) = frames.last() else {
            return Ok(tick);
        };
        if let Err(failure) = sink.deliver(&frames) {
            let buffered = view.count_after(after).unwrap_or(0);
            return Err(ExportError::Sink { failure, buffered });
        }
        after = last.insertion_id;
        store
            .save(&Cursor {
                insertion_id: last.insertion_id,
                frame_id: Some(last.frame_id_hex.clone()),
            })
            .map_err(ExportError::Cursor)?;
        tick.delivered += frames.len();
        tick.batches += 1;
    }
}

/// `Some(reason)` when the log is not the one this cursor was taken from (D-23-4-07 A).
fn check_anchor(view: &LogView, cursor: &Cursor) -> Result<Option<ResetReason>, ExportError> {
    let anchor = view.probe_anchor(cursor.insertion_id)?;
    if cursor.insertion_id > anchor.high_water {
        return Ok(Some(ResetReason::AheadOfLog));
    }
    match (&cursor.frame_id, &anchor.frame_id_hex) {
        (Some(expected), Some(found)) if expected != found => Ok(Some(ResetReason::AnchorMismatch)),
        _ => Ok(None),
    }
}

/// D-23-4-08 A: journal, print, move the bad file aside and start from the oldest retained row.
fn reset(
    store: &CursorStore,
    journal: Journal<'_>,
    reason: ResetReason,
    tick: &mut Tick,
) -> Result<(), ExportError> {
    journal(
        "siem.cursor.reset",
        serde_json::json!({ "reason": reason.code() }),
    )
    .map_err(ExportError::Journal)?;
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    store
        .move_aside(u64::try_from(now_ns).unwrap_or(u64::MAX))
        .map_err(ExportError::Cursor)?;
    tick.notes.push(format!(
        "{}: the SIEM cursor was moved aside; every retained row is re-sent",
        reason.code()
    ));
    Ok(())
}
