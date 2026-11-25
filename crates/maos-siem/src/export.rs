//! Story 23-4 — one cursor-driven export tick (D-23-4-03, -04, -06, -07, -08).
//!
//! A tick reads the quiesced log after the durable cursor, hands one bounded batch at a time
//! to a [`BatchSink`] and moves the cursor only after the sink acknowledged that batch. The
//! log itself is the spool: nothing is buffered beyond one batch, and a sink that stays down
//! leaves the cursor where it was.
//!
//! Journaled events (`siem.gap`, `siem.cursor.reset`) carry a deterministic payload (the sink
//! hash, the range or the bad cursor's fingerprint); the journal writer skips an insert whose
//! identical payload is already in the log. The guarantee is therefore: at most one row per
//! cursor transition, however many ticks fail on the cursor file in between or crash between
//! the journal write and the cursor write.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use maos_audit::insertion::LogView;
use maos_domain::ports::SiemProjectionPort;

use crate::cursor::{Cursor, CursorStore, Loaded, ResetReason};

/// At most this many frames per batch (and so per TLS connection).
pub const MAX_BATCH_FRAMES: usize = 512;
/// At most this many bytes of frames per batch. A single row larger than this cannot be split
/// (RFC 5425 frames are whole log lines), so it goes alone: the one exception to the bound.
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
    /// Hand the sink the shutdown flag, so a wait inside `deliver` can end promptly.
    fn set_cancel(&mut self, _cancel: Arc<AtomicBool>) {}
}

/// The localhost file sink: acknowledged when the lines are written and synced, and the
/// directory entry of the file has been synced once.
pub struct FileSink {
    path: PathBuf,
    port: Arc<dyn SiemProjectionPort>,
    tail_clean: bool,
    dir_synced: bool,
    dir_sync: DirSync,
}

/// Syncs the directory holding a path (injectable so a failure can be simulated).
pub type DirSync = Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>;

impl FileSink {
    #[must_use]
    pub fn new(path: PathBuf, port: Arc<dyn SiemProjectionPort>) -> Self {
        Self {
            path,
            port,
            tail_clean: false,
            dir_synced: false,
            dir_sync: Arc::new(sync_parent),
        }
    }

    /// Replace the directory sync (tests inject a failing one).
    #[must_use]
    pub fn with_dir_sync(mut self, dir_sync: DirSync) -> Self {
        self.dir_sync = dir_sync;
        self
    }
}

/// A partial append (a crash or ENOSPC mid-write) can leave an unterminated record at the
/// end. The batch that wrote it was never acknowledged and is about to be re-sent whole, so
/// cut the file back to its last line terminator; a file with no terminator at all is
/// terminated instead (it is not ours to truncate).
fn recover_tail(path: &Path) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(());
    }
    let mut last = [0_u8; 1];
    file.seek(SeekFrom::Start(len - 1))?;
    file.read_exact(&mut last)?;
    if last[0] == b'\n' {
        return Ok(());
    }
    let mut end = len;
    let mut chunk = vec![0_u8; 8192];
    while end > 0 {
        let start = end.saturating_sub(chunk.len() as u64);
        let size = (end - start) as usize;
        file.seek(SeekFrom::Start(start))?;
        file.read_exact(&mut chunk[..size])?;
        if let Some(at) = chunk[..size].iter().rposition(|byte| *byte == b'\n') {
            file.set_len(start + at as u64 + 1)?;
            return file.sync_all();
        }
        end = start;
    }
    file.seek(SeekFrom::End(0))?;
    file.write_all(b"\n")?;
    file.sync_all()
}

fn sync_parent(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let dir = path.parent().filter(|dir| !dir.as_os_str().is_empty());
        std::fs::File::open(dir.unwrap_or_else(|| Path::new(".")))?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

impl BatchSink for FileSink {
    fn identity(&self) -> String {
        format!("file:{}", self.path.display())
    }

    fn deliver(&mut self, frames: &[Frame]) -> Result<(), SinkFailure> {
        let io = |_: std::io::Error| SinkFailure::new("siem_sink_file_io");
        let mut out = Vec::new();
        for frame in frames {
            let line = self
                .port
                .project_redacted_entry(&frame.ndjson)
                .map_err(|_| SinkFailure::new("siem_projection_failed"))?;
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
        // Tail recovery is best-effort and needs READ access, which a write-only service
        // file (the old daemon opened it append-only) may not grant: when the file cannot be
        // read the recovery is skipped (documented limit) and the append still goes through.
        // After any successful append the tail is known clean, so recovery runs only at the
        // first attempt and after a failed append.
        if !self.tail_clean && self.path.exists() {
            let _ = recover_tail(&self.path);
        }
        let appended = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            file.write_all(&out)?;
            file.sync_data()
        })();
        if let Err(error) = appended {
            self.tail_clean = false; // a partial line may now end the file
            return Err(io(error));
        }
        self.tail_clean = true;
        // The cursor syncs the audit directory, not this one: make the directory entry
        // durable, and keep retrying until that has succeeded ONCE (existence of the file
        // proves nothing about its directory entry).
        if !self.dir_synced {
            (self.dir_sync)(&self.path).map_err(io)?;
            self.dir_synced = true;
        }
        Ok(())
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
    /// The cursor file could not be read (not a content problem: nothing is reset).
    #[error("siem_cursor_io: cursor file unreadable ({0:?})")]
    CursorRead(std::io::ErrorKind),
    #[error("siem_projection_failed: {0}")]
    Projection(String),
    /// The shutdown flag was raised.
    #[error("siem_export_cancelled")]
    Cancelled,
}

impl ExportError {
    /// The typed, value-free reason (one line per distinct reason in the consumer).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Source(_) => "siem_source_unavailable",
            Self::Sink { failure, .. } => failure.code,
            Self::Journal(_) => "siem_journal_failed",
            Self::Cursor(_) | Self::CursorRead(_) => "siem_cursor_io",
            Self::Projection(_) => "siem_projection_failed",
            Self::Cancelled => "siem_export_cancelled",
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

/// A tick that stopped on an error, with everything it completed before that: the typed
/// lines of a gap or reset journaled earlier in the tick are not lost to the failure.
#[derive(Debug)]
pub struct TickFailure {
    pub error: ExportError,
    pub tick: Tick,
}

impl TickFailure {
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.error.code()
    }

    #[must_use]
    pub fn buffered(&self) -> u64 {
        self.error.buffered()
    }
}

impl std::fmt::Display for TickFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for TickFailure {}

/// Journals one operator-relevant event (`siem.gap`, `siem.cursor.reset`) into the live log.
/// The payload is deterministic: the writer must not insert a row whose intent and payload
/// already exist.
pub type Journal<'a> = &'a dyn Fn(&str, serde_json::Value) -> Result<(), String>;

/// [`run_tick_cancellable`] with a flag that is never raised.
///
/// # Errors
/// [`TickFailure`]; the cursor stays at the last acknowledged batch.
pub fn run_tick(
    source: &Path,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
) -> Result<Tick, TickFailure> {
    run_tick_cancellable(source, store, sink, journal, &AtomicBool::new(false))
}

/// Run one tick against `source` (a quiesced copy of the log: see `maos-audit`'s
/// `insertion` module) until the log is exhausted, a batch is not acknowledged, or `cancel`
/// is raised (checked before every batch).
///
/// # Errors
/// [`TickFailure`]; the cursor stays at the last acknowledged batch.
pub fn run_tick_cancellable(
    source: &Path,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
    cancel: &AtomicBool,
) -> Result<Tick, TickFailure> {
    let mut tick = Tick::default();
    match drive(source, store, sink, journal, cancel, &mut tick) {
        Ok(()) => Ok(tick),
        Err(error) => Err(TickFailure { error, tick }),
    }
}

fn drive(
    source: &Path,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
    cancel: &AtomicBool,
    tick: &mut Tick,
) -> Result<(), ExportError> {
    let view = LogView::open(source)?;
    let mut after = match store.load() {
        Loaded::Absent => 0,
        Loaded::Io(kind) => return Err(ExportError::CursorRead(kind)),
        Loaded::Valid(cursor) => match check_anchor(&view, &cursor)? {
            None => cursor.insertion_id,
            Some(reason) => {
                reset(store, journal, reason, tick)?;
                0
            }
        },
        Loaded::Rejected(reason) => {
            reset(store, journal, reason, tick)?;
            0
        }
    };

    // Expired ranges in insertion order. Expiry need not remove a prefix (a legal hold can keep
    // an older segment), so retained rows below a range are delivered BEFORE the cursor may
    // pass the range; each range is journaled only when the next row to send lies beyond it.
    let ranges = view.expired_ranges_after(after)?;
    let sink_tag = &store.sink_hash()[..16];
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(ExportError::Cancelled);
        }
        let gap = ranges.iter().find(|range| range.last_rowid > after);
        let mut rows = view.read_after(after, MAX_BATCH_FRAMES, MAX_BATCH_BYTES)?;
        if let Some(range) = gap {
            let from = range.first_rowid.max(after.saturating_add(1));
            rows.retain(|row| row.insertion_id < from);
            if rows.is_empty() {
                journal(
                    "siem.gap",
                    serde_json::json!({
                        "from": from,
                        "to": range.last_rowid,
                        "segment_seq": range.seq,
                        "sink": sink_tag,
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
                continue;
            }
        }
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
            return Ok(());
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
        // A cursor parked after a gap names no row: it is only valid while an expired segment
        // still ends at it. A restored registry shows that segment unexpired again, and the
        // rows it held would be readable and never sent.
        (None, _)
            if cursor.insertion_id != 0
                && !view.expired_segment_ends_at(cursor.insertion_id)? =>
        {
            Ok(Some(ResetReason::GapEvidenceLost))
        }
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
        serde_json::json!({
            "reason": reason.code(),
            "sink": &store.sink_hash()[..16],
            "cursor": store.fingerprint(),
        }),
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
