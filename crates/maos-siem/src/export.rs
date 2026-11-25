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

use std::io::Write;
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

/// The localhost file sink. It only ever APPENDS (`O_APPEND`, no truncate, no read, never a
/// byte it did not write), so a service with write permission alone can use it, and a batch
/// is acknowledged only when the whole batch was written and fsynced.
///
/// To keep records line-aligned, the FIRST append after process start and the FIRST append
/// after ANY failed append, and the first append to a file that replaced the one written
/// before, begin with a single `\n`: that terminates whatever fragment a crash or a short
/// write left, so a retried record is never glued to it. A blank line may therefore appear
/// in the file: downstream shippers MUST ignore blank lines (RELEASE-HOLDS 23-4.j).
///
/// Creation is decided atomically at open (`O_EXCL` first, then plain append): a file this
/// sink created owes a parent-directory sync, retried until it succeeds, before the batch is
/// acknowledged. The sink keeps the identity (device, inode) of the file it wrote last, so a
/// path that now names another file (logrotate, a replacement) is treated as a new
/// creation or replacement. A pre-existing file owes no directory sync, and a replacement
/// is synced with `EACCES` on opening the directory tolerated (documented limit).
///
/// The sink is long-lived (the runtime keeps one) so what it learned survives the tick.
pub struct FileSink {
    path: PathBuf,
    port: Arc<dyn SiemProjectionPort>,
    /// The next append begins with `\n`.
    lead_newline: bool,
    /// The file the previous append went to.
    last_file: Option<FileId>,
    /// A directory entry whose parent sync is owed: (file, created by this sink, the entry's
    /// own path: its parent is the directory to sync).
    owed_dir_sync: Option<(FileId, bool, PathBuf)>,
    /// The parent directory was synced since this process started.
    session_synced: bool,
    dir_sync: DirSync,
    writer: Writer,
}

/// Syncs the directory holding a path (injectable so a failure can be simulated).
pub type DirSync = Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>;

/// Writes the bytes to the open append handle and syncs its data (injectable: a short write).
pub type Writer = Arc<dyn Fn(&mut std::fs::File, &[u8]) -> std::io::Result<()> + Send + Sync>;

impl FileSink {
    #[must_use]
    pub fn new(path: PathBuf, port: Arc<dyn SiemProjectionPort>) -> Self {
        Self {
            path,
            port,
            lead_newline: true,
            last_file: None,
            owed_dir_sync: None,
            session_synced: false,
            dir_sync: Arc::new(sync_parent),
            writer: Arc::new(write_and_sync),
        }
    }

    /// Replace the directory sync (tests inject a failing one).
    #[must_use]
    pub fn with_dir_sync(mut self, dir_sync: DirSync) -> Self {
        self.dir_sync = dir_sync;
        self
    }

    /// Replace the write (tests inject a short write).
    #[must_use]
    pub fn with_writer(mut self, writer: Writer) -> Self {
        self.writer = writer;
        self
    }
}

fn write_and_sync(file: &mut std::fs::File, bytes: &[u8]) -> std::io::Result<()> {
    file.write_all(bytes)?;
    file.sync_data()
}

/// Open for append, creating atomically: `(file, created by this call, the path of the entry
/// whose directory must be synced)`. A symlink whose target does not exist yet is followed:
/// the target is created (`O_EXCL`) and its own directory is the one to sync.
fn open_append(path: &Path) -> std::io::Result<(std::fs::File, bool, PathBuf)> {
    let create = |target: &Path| {
        std::fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(target)
    };
    for _ in 0..3 {
        match create(path) {
            Ok(file) => return Ok((file, true, path.to_path_buf())),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                match std::fs::OpenOptions::new().append(true).open(path) {
                    Ok(file) => return Ok((file, false, path.to_path_buf())),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        // Either moved away between the two opens (create it on the next
                        // pass) or a dangling symlink (create its target).
                        if let Ok(link) = std::fs::read_link(path) {
                            let target = path.parent().unwrap_or_else(|| Path::new(".")).join(link);
                            match create(&target) {
                                Ok(file) => return Ok((file, true, target)),
                                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other("the sink file kept vanishing"))
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

        let (mut file, created, entry) = open_append(&self.path).map_err(io)?;
        let meta = file.metadata().map_err(io)?;
        let id = file_id(&meta);
        let replaced = self.last_file.is_some_and(|last| last != id);
        if created || replaced {
            self.owed_dir_sync = Some((id, created, entry.clone()));
        } else if self.owed_dir_sync.is_none() && !self.session_synced {
            // ONE parent-directory sync per process start before the first cursor advance, for
            // a file found existing too: a previous process may have died between creating
            // the entry and syncing its directory.
            self.owed_dir_sync = Some((id, false, entry.clone()));
        }
        if replaced {
            self.lead_newline = true;
        }
        self.last_file = Some(id);

        // A leading newline only where there is something to terminate.
        let mut payload = Vec::with_capacity(out.len() + 1);
        if self.lead_newline && meta.len() > 0 {
            payload.push(b'\n');
        }
        payload.extend_from_slice(&out);
        if let Err(error) = (self.writer)(&mut file, &payload) {
            self.lead_newline = true; // a fragment may now end the file
            return Err(io(error));
        }
        // A rotator that TRUNCATED the file under us (copytruncate, unsupported) leaves it
        // shorter than what we knew plus what we wrote: the batch is in neither file, so it is
        // not acknowledged. Only this detectable case is caught (fstat on the same fd).
        let end = meta.len().saturating_add(payload.len() as u64);
        if file.metadata().map_err(io)?.len() < end {
            self.lead_newline = true;
            return Err(SinkFailure::new("siem_file_truncated_during_append"));
        }
        self.lead_newline = false;

        // The cursor syncs the audit directory, not this one: a directory entry this sink
        // created, a replacement, and the first use after a process start are durable only
        // once the directory that CONTAINS the entry was synced.
        if let Some((owed, ours, entry)) = self.owed_dir_sync.clone() {
            if owed == id {
                match (self.dir_sync)(&entry) {
                    Ok(()) => {}
                    Err(error) if !ours && error.kind() == std::io::ErrorKind::PermissionDenied => {
                    }
                    Err(error) => return Err(io(error)),
                }
            }
            self.owed_dir_sync = None;
            self.session_synced = true;
        }
        Ok(())
    }
}

/// What names one file regardless of its path (device and inode where the platform has them).
type FileId = (u64, u64);

fn file_id(meta: &std::fs::Metadata) -> FileId {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (meta.dev(), meta.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        (0, 0)
    }
}

/// The operator wording of a backlog count: exact, a lower bound, or unknown (never 0).
#[must_use]
pub fn describe_backlog(buffered: Option<u64>, at_least: bool) -> String {
    match (buffered, at_least) {
        (Some(count), true) => format!("at least {count}"),
        (Some(count), false) => count.to_string(),
        (None, _) => "an unknown number of".to_owned(),
    }
}

/// Why a tick stopped. None of these moves the cursor.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The log could not be read (never a gap: a registered segment that is missing is this).
    #[error("siem_source_unavailable: {0}")]
    Source(#[from] maos_audit::AuditError),
    /// The sink did not acknowledge a batch; `buffered` rows lie after the cursor (at least
    /// that many when `at_least`; unknown when `None`).
    #[error("{failure}; {} record(s) remain after the cursor", describe_backlog(*buffered, *at_least))]
    Sink {
        failure: SinkFailure,
        buffered: Option<u64>,
        at_least: bool,
    },
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

    /// Rows left after the cursor when the sink refused: `None` when that count could not be
    /// read (unknown, never 0), and for every other reason.
    #[must_use]
    pub fn buffered(&self) -> Option<u64> {
        match self {
            Self::Sink { buffered, .. } => *buffered,
            _ => None,
        }
    }

    /// `true` when [`ExportError::buffered`] is only a lower bound (the count was capped).
    #[must_use]
    pub fn buffered_is_lower_bound(&self) -> bool {
        matches!(self, Self::Sink { at_least: true, .. })
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
    pub fn buffered(&self) -> Option<u64> {
        self.error.buffered()
    }

    /// `true` when [`TickFailure::buffered`] is a lower bound only.
    #[must_use]
    pub fn buffered_is_lower_bound(&self) -> bool {
        self.error.buffered_is_lower_bound()
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
    run_with(source, false, None, store, sink, journal, cancel)
}

/// Run one tick against the LIVE log a daemon is writing (Story 23-4 review round 3): no
/// snapshot copy. Each batch is read through a fresh bounded read transaction that is
/// released BEFORE the sink is touched, so a stalled collector never pins a WAL read mark;
/// a tick with nothing after the cursor opens nothing but one `max(rowid)` probe.
///
/// # Errors
/// [`TickFailure`]; the cursor stays at the last acknowledged batch.
pub fn run_tick_live(
    db: &Path,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
    cancel: &AtomicBool,
    idle: &IdleGuard,
) -> Result<Tick, TickFailure> {
    run_with(db, true, Some(idle), store, sink, journal, cancel)
}

/// Every this-many-th consecutive idle tick runs the FULL path (all sources opened, anchor
/// and registry checked) instead of the one-row probe.
pub const FULL_CHECK_EVERY_IDLE_TICKS: u32 = 60;

/// At most this many rows are counted for the "records remain" figure of a sink failure.
pub const BACKLOG_COUNT_CAP: u64 = 5_000;

/// Counts idle ticks so the cheap probe cannot hide a diagnostic forever, and latches a
/// failing full path: while the last FULL check failed, every tick takes the full path (the
/// cheap probe verifies nothing, so it must not report recovery).
#[derive(Debug, Default)]
pub struct IdleGuard {
    ticks: std::sync::atomic::AtomicU32,
    failing: AtomicBool,
    last_cheap: AtomicBool,
}

impl IdleGuard {
    /// `true` when this idle tick may take the cheap path: not while a full check is failing,
    /// and never on every [`FULL_CHECK_EVERY_IDLE_TICKS`]-th one.
    fn may_skip(&self) -> bool {
        !self.failing.load(Ordering::Relaxed)
            && self.ticks.fetch_add(1, Ordering::Relaxed) % FULL_CHECK_EVERY_IDLE_TICKS
                != FULL_CHECK_EVERY_IDLE_TICKS - 1
    }
}

fn run_with(
    source: &Path,
    live: bool,
    idle: Option<&IdleGuard>,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
    cancel: &AtomicBool,
) -> Result<Tick, TickFailure> {
    let mut tick = Tick::default();
    let outcome = drive(source, live, idle, store, sink, journal, cancel, &mut tick);
    if let (true, Some(idle)) = (live, idle) {
        // A cheap idle tick verified nothing: it neither clears nor sets the latch. Only a
        // FULL path success clears a failure only the full path could have found.
        let cheap = idle.last_cheap.swap(false, Ordering::Relaxed);
        match &outcome {
            Ok(()) if !cheap => idle.failing.store(false, Ordering::Relaxed),
            Ok(()) => {}
            Err(_) => idle.failing.store(true, Ordering::Relaxed),
        }
    }
    match outcome {
        Ok(()) => Ok(tick),
        Err(error) => Err(TickFailure { error, tick }),
    }
}

fn open_view(source: &Path, live: bool) -> Result<LogView, maos_audit::AuditError> {
    if live {
        LogView::open_live(source)
    } else {
        LogView::open(source)
    }
}

#[allow(clippy::too_many_arguments)]
fn drive(
    source: &Path,
    live: bool,
    idle: Option<&IdleGuard>,
    store: &CursorStore,
    sink: &mut dyn BatchSink,
    journal: Journal<'_>,
    cancel: &AtomicBool,
    tick: &mut Tick,
) -> Result<(), ExportError> {
    let loaded = store.load();
    // The idle probe: the newest live row IS the row the cursor names (same insertion id AND
    // same frame id), so there is nothing after the cursor. No view, no segment is opened and
    // no file is created. A different frame id at that id (a restored log) falls through to
    // the full path, which resets and journals; and every `FULL_CHECK_EVERY_IDLE_TICKS`-th
    // idle tick takes the full path anyway, so a missing segment or a registry change is
    // diagnosed within a bounded number of ticks.
    if let (true, Some(idle)) = (live, idle) {
        if let Loaded::Valid(cursor) = &loaded {
            if let (Some(frame), Some((id, latest))) =
                (&cursor.frame_id, LogView::live_latest(source)?)
            {
                if id == cursor.insertion_id && &latest == frame && idle.may_skip() {
                    idle.last_cheap.store(true, Ordering::Relaxed);
                    return Ok(());
                }
            }
        }
    }
    let mut pending = Some(loaded);
    let mut after = 0_i64;
    let sink_tag = &store.sink_hash()[..16];
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(ExportError::Cancelled);
        }
        // One view per batch: for a live log it holds ONE read transaction until it is
        // dropped below, before the sink is touched.
        let view = open_view(source, live)?;
        if let Some(loaded) = pending.take() {
            after = match loaded {
                Loaded::Absent => 0,
                Loaded::Io(kind) => return Err(ExportError::CursorRead(kind)),
                Loaded::Valid(cursor) => match check_anchor(&view, &cursor)? {
                    None => cursor.insertion_id,
                    Some(reason) => {
                        // Decided: release the read transaction, THEN journal and move the
                        // cursor aside, and start over from the oldest retained row.
                        drop(view);
                        reset(store, journal, reason, tick)?;
                        after = 0;
                        continue;
                    }
                },
                Loaded::Rejected(reason) => {
                    drop(view);
                    reset(store, journal, reason, tick)?;
                    after = 0;
                    continue;
                }
            };
        }
        // Expired ranges in insertion order, re-read for every batch (a segment may expire
        // mid-tick). Expiry need not remove a prefix (a legal hold can keep an older
        // segment), so retained rows below a range are delivered BEFORE the cursor may pass
        // the range; each range is journaled only when the next row to send lies beyond it.
        let ranges = view.expired_ranges_after(after)?;
        let gap = ranges.iter().find(|range| range.last_rowid > after);
        let mut rows = view.read_after(after, MAX_BATCH_FRAMES, MAX_BATCH_BYTES)?;
        if let Some(range) = gap {
            let from = range.first_rowid.max(after.saturating_add(1));
            rows.retain(|row| row.insertion_id < from);
            if rows.is_empty() {
                // The transition is DECIDED: end the read transaction BEFORE executing it
                // (the journal write, the cursor file and its directory sync), so no WAL
                // reader is held while a writer or a rotation checkpoint is waited for.
                let (to, seq) = (range.last_rowid, range.seq);
                drop(view);
                journal(
                    "siem.gap",
                    serde_json::json!({
                        "from": from,
                        "to": to,
                        "segment_seq": seq,
                        "sink": sink_tag,
                    }),
                )
                .map_err(ExportError::Journal)?;
                after = to;
                store
                    .save(&Cursor {
                        insertion_id: after,
                        frame_id: None,
                    })
                    .map_err(ExportError::Cursor)?;
                tick.notes.push(format!(
                    "siem_gap: ids {from}..={to} of expired segment {seq} were never delivered"
                ));
                continue;
            }
        }
        // The read transaction ends here: rows are plain values from now on.
        drop(view);
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
            // The backlog is counted through a fresh short view, bounded (a scan of a huge
            // backlog must not hold a read transaction); when the log cannot be read the
            // count is UNKNOWN, never zero.
            let counted = open_view(source, live)
                .and_then(|view| view.count_after_capped(after, BACKLOG_COUNT_CAP))
                .ok();
            let (buffered, at_least) = match counted {
                Some((count, capped)) => (Some(count), capped),
                None => (None, false),
            };
            return Err(ExportError::Sink {
                failure,
                buffered,
                at_least,
            });
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
