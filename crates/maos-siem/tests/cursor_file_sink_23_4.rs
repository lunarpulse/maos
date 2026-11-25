#![cfg(not(feature = "siem-fault-inject"))]
#![forbid(unsafe_code)]

//! Story 23-4 — the durable cursor under the file sink (`MAOS_SIEM_FILE` alone keeps today's
//! sink, now with a cursor), and the gap rule of D-23-4-06 A at the library seam.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::cursor::CursorStore;
use maos_siem::export::{run_tick, BatchSink, FileSink, Tick, TickFailure};
use maos_siem::SiemExporter;
use tempfile::TempDir;

fn seed(db: &Path, count: usize) {
    let tl = TransparencyLogAdapter::open(db, 1).expect("open log");
    for n in 0..count {
        tl.insert_frame_event(
            FrameKind::CapabilityInvocation,
            1,
            None,
            "test.capability.invoke",
            format!("row {n}").as_bytes(),
            FrameOrigin::Kernel,
        );
    }
}

fn raw(db: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).expect("raw connection")
}

struct Rig {
    _dir: TempDir,
    db: PathBuf,
    sink_path: PathBuf,
    journal: Mutex<Vec<(String, serde_json::Value)>>,
}

impl Rig {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        Self {
            db: dir.path().join("audit.sqlite"),
            sink_path: dir.path().join("siem.log"),
            _dir: dir,
            journal: Mutex::default(),
        }
    }

    fn sink(&self) -> FileSink {
        FileSink::new(self.sink_path.clone(), Arc::new(SiemExporter))
    }

    fn tick(&self) -> Result<Tick, TickFailure> {
        let mut sink = self.sink();
        let store = CursorStore::beside(&self.db, &sink.identity());
        // As the daemon does: export a quiesced snapshot, never the live file.
        let snapshot = self.db.with_file_name("snapshot.sqlite");
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", snapshot.display()));
        }
        raw(&self.db)
            .execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
            .expect("snapshot");
        let result = run_tick(&snapshot, &store, &mut sink, &|intent, payload| {
            self.journal
                .lock()
                .unwrap()
                .push((intent.to_owned(), payload));
            Ok(())
        });
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", snapshot.display()));
        }
        result
    }

    /// Non-blank lines: the file sink may write a blank line (downstream ignores them).
    fn lines(&self) -> usize {
        std::fs::read_to_string(&self.sink_path)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.is_empty())
            .count()
    }
}

#[test]
fn the_file_sink_resumes_from_its_cursor_after_a_restart() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    assert_eq!(rig.tick().unwrap().delivered, 3);
    assert_eq!(rig.lines(), 3);
    assert_eq!(
        rig.tick().unwrap().delivered,
        0,
        "nothing new: nothing re-sent"
    );

    seed(&rig.db, 2);
    assert_eq!(rig.tick().unwrap().delivered, 2);
    assert_eq!(rig.lines(), 5, "every row exactly once across the restart");
    assert!(rig.journal.lock().unwrap().is_empty());
}

#[test]
fn an_unwritable_file_sink_keeps_the_cursor_and_reports_the_rows_after_it() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    assert_eq!(rig.tick().unwrap().delivered, 3);
    seed(&rig.db, 2);
    // A directory cannot be opened as an append-file.
    std::fs::remove_file(&rig.sink_path).unwrap();
    std::fs::create_dir(&rig.sink_path).unwrap();
    let failure = rig.tick().expect_err("sink down");
    assert_eq!(failure.code(), "siem_sink_file_io");
    assert_eq!(failure.buffered(), Some(2));
    std::fs::remove_dir(&rig.sink_path).unwrap();
    assert_eq!(
        rig.tick().unwrap().delivered,
        2,
        "delivered once the sink returns"
    );
}

// The adapter's schema already carries `transparency_log_segments`; the tests insert registry rows.

#[test]
fn an_expired_segment_that_held_unsent_rows_journals_exactly_one_gap() {
    let rig = Rig::new();
    seed(&rig.db, 5);
    // Rows 1..=2 were rotated into segment 1, and the segment expired before they were sent.
    raw(&rig.db)
        .execute_batch(
            "DELETE FROM transparency_log WHERE rowid <= 2; \
             INSERT INTO transparency_log_segments VALUES (1, 0, 1, 2, 2, 1, 2, 'a', NULL, 3, 9);",
        )
        .unwrap();
    let tick = rig.tick().unwrap();
    assert_eq!(tick.delivered, 3, "the retained rows are delivered");
    let journal = rig.journal.lock().unwrap().clone();
    assert_eq!(journal.len(), 1, "exactly one siem.gap row");
    assert_eq!(journal[0].0, "siem.gap");
    let payload = &journal[0].1;
    assert_eq!(
        (
            payload["from"].as_i64(),
            payload["to"].as_i64(),
            payload["segment_seq"].as_i64()
        ),
        (Some(1), Some(2), Some(1)),
        "ids and segment sequence"
    );
    assert_eq!(
        payload["sink"].as_str().map(str::len),
        Some(16),
        "sink tag, no row payload"
    );
    assert!(tick.notes.iter().any(|note| note.starts_with("siem_gap")));

    seed(&rig.db, 1);
    assert_eq!(rig.tick().unwrap().delivered, 1);
    assert_eq!(
        rig.journal.lock().unwrap().len(),
        1,
        "the gap is journaled once"
    );
}

#[test]
fn a_registered_segment_missing_at_read_time_is_a_tick_error_never_a_gap() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    raw(&rig.db)
        .execute_batch(
            "INSERT INTO transparency_log_segments VALUES (1, 0, 1, 2, 2, 1, 2, 'a', NULL, 3, NULL);"
        )
        .unwrap();
    let failure = rig.tick().expect_err("the sealed segment file is absent");
    assert_eq!(failure.code(), "siem_source_unavailable");
    assert!(rig.journal.lock().unwrap().is_empty(), "no gap is invented");
    assert_eq!(rig.lines(), 0);
}

// ── review round 1 ───────────────────────────────────────────────────────────

/// Expiry need not remove a prefix: a held segment 1 stays while segment 2 expires. The rows
/// of segment 1 are delivered BEFORE the cursor may pass segment 2's gap, and the gap is
/// journaled at that point (not before).
#[test]
fn retained_rows_below_an_expired_range_are_delivered_before_its_gap_is_journaled() {
    let rig = Rig::new();
    seed(&rig.db, 6);
    // Rows 3 and 4 were in segment 2, which expired; 1-2 (held) and 5-6 are still readable.
    raw(&rig.db)
        .execute_batch(
            "DELETE FROM transparency_log WHERE rowid IN (3, 4); \
             INSERT INTO transparency_log_segments VALUES (2, 2, 3, 4, 2, 1, 2, 'b', NULL, 3, 9);",
        )
        .unwrap();
    let lines_at_gap = Mutex::new(Vec::new());
    let mut sink = rig.sink();
    let store = CursorStore::beside(&rig.db, &sink.identity());
    let snapshot = rig.db.with_file_name("snap.sqlite");
    raw(&rig.db)
        .execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
        .unwrap();
    let tick = run_tick(&snapshot, &store, &mut sink, &|intent, payload| {
        lines_at_gap
            .lock()
            .unwrap()
            .push((intent.to_owned(), rig.lines(), payload));
        Ok(())
    })
    .unwrap();
    assert_eq!(tick.delivered, 4, "every retained row is delivered");
    let at = lines_at_gap.lock().unwrap().clone();
    assert_eq!(at.len(), 1);
    assert_eq!(at[0].0, "siem.gap");
    assert_eq!(
        at[0].1, 2,
        "rows 1-2 were already sent when the gap was journaled"
    );
    assert_eq!(
        (at[0].2["from"].as_i64(), at[0].2["to"].as_i64()),
        (Some(3), Some(4))
    );
    assert_eq!(rig.lines(), 4);
}

/// A cursor parked after a gap names no row; it is trusted only while an expired segment still
/// ends at it. A restored registry removes that evidence: reset, resend, never silent.
#[test]
fn a_gap_cursor_whose_expiry_evidence_is_gone_resets() {
    let rig = Rig::new();
    seed(&rig.db, 4);
    raw(&rig.db)
        .execute_batch(
            "DELETE FROM transparency_log WHERE rowid <= 2; \
             INSERT INTO transparency_log_segments VALUES (1, 0, 1, 2, 2, 1, 2, 'a', NULL, 3, 9);",
        )
        .unwrap();
    assert_eq!(rig.tick().unwrap().delivered, 2);
    // Restore: the registry no longer shows segment 1 as expired (and the rows are back).
    raw(&rig.db)
        .execute_batch("DELETE FROM transparency_log_segments;")
        .unwrap();
    let cursor = std::fs::read_to_string(rig.db.with_file_name("siem-cursor.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&cursor).unwrap();
    value["insertion_id"] = serde_json::json!(2);
    value["frame_id"] = serde_json::Value::Null;
    std::fs::write(rig.db.with_file_name("siem-cursor.json"), value.to_string()).unwrap();
    let tick = rig.tick().unwrap();
    let journal = rig.journal.lock().unwrap().clone();
    assert_eq!(journal.last().unwrap().0, "siem.cursor.reset");
    assert_eq!(
        journal.last().unwrap().1["reason"],
        "siem_cursor_gap_evidence_lost"
    );
    assert_eq!(tick.delivered, 2, "the retained rows are re-sent");
}

/// A cursor file that cannot be read (here: a directory stands where it should be) is a tick
/// error with the cursor kept, never a reset that re-sends the whole log.
#[test]
fn an_unreadable_cursor_file_is_a_tick_error_not_a_reset() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    std::fs::create_dir(rig.db.with_file_name("siem-cursor.json")).unwrap();
    let failure = rig.tick().expect_err("cursor unreadable");
    assert_eq!(failure.code(), "siem_cursor_io");
    assert!(
        rig.journal.lock().unwrap().is_empty(),
        "nothing was reset or journaled"
    );
    assert_eq!(rig.lines(), 0, "nothing was re-sent");
}

/// A blocked cursor temp file fails every tick; the journal payload is identical each time
/// (so the writer's dedupe leaves one row), and no tick delivers anything it could not record.
#[test]
fn a_blocked_cursor_write_journals_the_same_gap_payload_every_attempt() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    raw(&rig.db)
        .execute_batch(
            "DELETE FROM transparency_log WHERE rowid <= 1; \
             INSERT INTO transparency_log_segments VALUES (1, 0, 1, 1, 1, 1, 2, 'a', NULL, 3, 9);",
        )
        .unwrap();
    std::fs::create_dir(rig.db.with_file_name("siem-cursor.json.tmp")).unwrap();
    for _ in 0..3 {
        assert_eq!(
            rig.tick().expect_err("cursor write blocked").code(),
            "siem_cursor_io"
        );
    }
    let journal = rig.journal.lock().unwrap().clone();
    assert_eq!(journal.len(), 3);
    assert!(
        journal.iter().all(|entry| entry == &journal[0]),
        "{journal:?}"
    );
}

// ── review round 2 ───────────────────────────────────────────────────────────

impl Rig {
    /// One tick with a caller-owned sink (so its in-memory state survives between ticks).
    fn tick_using(&self, sink: &mut FileSink) -> Result<Tick, TickFailure> {
        let store = CursorStore::beside(&self.db, &sink.identity());
        let snapshot = self.db.with_file_name("snap2.sqlite");
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", snapshot.display()));
        }
        raw(&self.db)
            .execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
            .expect("snapshot");
        let result = run_tick(&snapshot, &store, sink, &|_, _| Ok(()));
        let _ = std::fs::remove_file(&snapshot);
        result
    }
}

/// R2-2: a service with write but no read permission on its SIEM file (the old daemon opened
/// it append-only) keeps exporting: tail recovery is best-effort and never gates the append.
#[cfg(unix)]
#[test]
fn a_write_only_sink_file_still_receives_batches() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::new();
    seed(&rig.db, 2);
    std::fs::write(&rig.sink_path, "").unwrap();
    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o200)).unwrap();
    assert!(
        std::fs::File::open(&rig.sink_path).is_err(),
        "the file is write-only for us"
    );
    assert_eq!(rig.tick().unwrap().delivered, 2);
    seed(&rig.db, 1);
    assert_eq!(rig.tick().unwrap().delivered, 1);
    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(rig.lines(), 3);
}

/// R2-3: the directory sync is retried until it has succeeded once; the cursor must not
/// advance on the attempt where it failed, even though the file now exists.
#[test]
fn a_failed_directory_sync_is_retried_and_the_cursor_waits_for_it() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let rig = Rig::new();
    seed(&rig.db, 2);
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let mut sink = rig.sink().with_dir_sync(Arc::new(move |_| {
        if counter.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(std::io::Error::other("injected dir sync failure"))
        } else {
            Ok(())
        }
    }));
    let cursor = rig.db.with_file_name("siem-cursor.json");
    let failure = rig.tick_using(&mut sink).expect_err("first dir sync fails");
    assert_eq!(failure.code(), "siem_sink_file_io");
    assert!(rig.sink_path.exists(), "the file exists now");
    assert!(!cursor.exists(), "the cursor did not advance");
    assert_eq!(
        rig.tick_using(&mut sink).unwrap().delivered,
        2,
        "the batch is re-sent"
    );
    assert!(cursor.exists());
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        2,
        "synced once it succeeded, not again"
    );
}

/// R2-7: a frame that is over 1 MiB ON THE WIRE (the batch bound) is sent alone, and the row
/// after it follows in the next batch: the batch boundaries differ from a full read's (a
/// 600-byte per-row allowance and CEF escape expansion), the row ORDER never does.
#[test]
fn a_frame_over_one_mebibyte_is_sent_alone_and_the_next_row_follows() {
    let rig = Rig::new();
    seed(&rig.db, 1);
    let big = vec![b'a'; (1 << 20) + 4096];
    let connection = raw(&rig.db);
    for (n, payload) in [(1_i64, big.as_slice()), (2, b"after".as_slice())] {
        connection
            .execute(
                "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, \
                 kind, intent, payload_redacted, origin) VALUES (randomblob(16), ?1, 1, 1, 4, 'big', ?2, 0)",
                rusqlite::params![n, payload],
            )
            .unwrap();
    }
    drop(connection);
    let tick = rig.tick().unwrap();
    assert_eq!(tick.delivered, 3);
    assert_eq!(
        tick.batches, 3,
        "the small row, the big frame alone, the row after it"
    );
    assert_eq!(rig.lines(), 3);
    let text = std::fs::read_to_string(&rig.sink_path).unwrap();
    assert!(
        text.lines().nth(1).unwrap().len() > (1 << 20),
        "the big line is over 1 MiB"
    );
}

// ── review rounds 3 and 4: the append-only file sink ─────────────────────────

/// A writer that writes half of the batch and then fails, `times` times, then behaves.
#[allow(deprecated)] // `fetch_update` is the name older toolchains have
fn short_writes(times: usize) -> maos_siem::export::Writer {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let left = Arc::new(AtomicUsize::new(times));
    Arc::new(move |file, bytes| {
        if left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
        {
            file.write_all(&bytes[..bytes.len() / 2])?;
            return Err(std::io::Error::other("injected ENOSPC after a short write"));
        }
        file.write_all(bytes)?;
        file.sync_data()
    })
}

/// The non-blank lines of a file (downstream shippers ignore blank lines).
fn records(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What a clean run of the same rows writes (another file, its own cursor).
fn clean_records(rig: &Rig) -> Vec<String> {
    let path = rig.sink_path.with_file_name("clean.log");
    let mut sink = FileSink::new(path.clone(), Arc::new(SiemExporter));
    rig.tick_using(&mut sink).expect("clean run");
    records(&path)
}

fn is_complete(line: &str) -> bool {
    line.starts_with("<134>1 ") && line.contains(" CEF:0|") && line.matches("<134>1 ").count() == 1
}

/// R4 (replaces the offset/truncate design): CONSECUTIVE short writes, then a clean retry.
/// The sink only appends; each retry begins with a newline that terminates the fragment the
/// failure left, so no record is ever glued to a fragment, the cursor moves only after the
/// whole batch was written, and the last records of the file are exactly a clean run's.
#[test]
fn consecutive_short_writes_then_a_clean_retry_keep_every_record_line_aligned() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    let clean = clean_records(&rig);
    let mut sink = rig.sink().with_writer(short_writes(2));
    let cursor = rig.db.with_file_name("siem-cursor.json");
    let identity = sink.identity();
    let _ = identity;
    for attempt in 1..=2 {
        let failure = rig.tick_using(&mut sink).expect_err("short write");
        assert_eq!(failure.code(), "siem_sink_file_io", "attempt {attempt}");
        assert!(!cursor_for(&rig, &sink).exists(), "the cursor stays");
    }
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 3);
    let _ = cursor;
    let lines = records(&rig.sink_path);
    assert_eq!(
        &lines[lines.len() - 3..],
        &clean[..],
        "the final batch is whole"
    );
    for line in &lines {
        assert!(
            line.matches("<134>1 ").count() <= 1,
            "a record was glued to a fragment: {line:?}"
        );
    }
    let fragments = lines.iter().filter(|line| !is_complete(line)).count();
    assert!(fragments <= 2, "only the failed attempts' fragments remain");
}

fn cursor_for(rig: &Rig, sink: &FileSink) -> PathBuf {
    CursorStore::beside(&rig.db, &sink.identity())
        .path()
        .to_path_buf()
}

/// A fragment already in a pre-existing file (a crash) is never modified: the first append of
/// the process starts with a newline and the records follow on their own lines.
#[test]
fn a_fragment_left_by_a_crash_is_terminated_never_modified() {
    let rig = Rig::new();
    seed(&rig.db, 2);
    let before = "earlier line\n<134>1 PARTIAL-RECORD-WITHOUT-NEWLINE";
    std::fs::write(&rig.sink_path, before).unwrap();
    assert_eq!(rig.tick().unwrap().delivered, 2);
    let text = std::fs::read_to_string(&rig.sink_path).unwrap();
    assert!(
        text.starts_with(before),
        "not a byte of it was touched: {text}"
    );
    assert_eq!(text[before.len()..].chars().next(), Some('\n'));
    let lines = records(&rig.sink_path);
    assert_eq!(lines.len(), 4);
    assert!(is_complete(&lines[2]) && is_complete(&lines[3]));
}

/// A path that now names a populated UNRELATED file (replaced between ticks): nothing is
/// truncated or modified, and the new records follow a leading newline.
#[cfg(unix)]
#[test]
fn a_populated_replacement_file_is_appended_to_after_a_newline_and_never_truncated() {
    let rig = Rig::new();
    seed(&rig.db, 2);
    let mut sink = rig.sink();
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 2);
    let other = rig.sink_path.with_file_name("unrelated.log");
    let unrelated = "unrelated A\nunrelated B\nunrelated C without a newline";
    std::fs::write(&other, unrelated).unwrap();
    std::fs::rename(&other, &rig.sink_path).unwrap();
    seed(&rig.db, 1);
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 1);
    let text = std::fs::read_to_string(&rig.sink_path).unwrap();
    assert!(
        text.starts_with(unrelated),
        "nothing truncated or modified: {text}"
    );
    assert_eq!(&text[unrelated.len()..unrelated.len() + 1], "\n");
    assert!(is_complete(text[unrelated.len() + 1..].trim_end()));
}

/// logrotate (move, the sink creates the new file): the created entry owes a parent-directory
/// sync before the cursor may move; a failing sync keeps the cursor, the retry syncs.
#[cfg(unix)]
#[test]
fn a_file_created_after_a_logrotate_move_owes_a_directory_sync_before_the_cursor_moves() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let rig = Rig::new();
    seed(&rig.db, 2);
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let mut sink = rig.sink().with_dir_sync(Arc::new(move |_| {
        // 1: the first creation; 2: the creation after the move FAILS; 3+: fine.
        if counter.fetch_add(1, Ordering::SeqCst) == 1 {
            Err(std::io::Error::other("injected dir sync failure"))
        } else {
            Ok(())
        }
    }));
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 2);
    std::fs::rename(&rig.sink_path, rig.sink_path.with_file_name("siem.log.1")).unwrap();
    seed(&rig.db, 1);
    let cursor = cursor_for(&rig, &sink);
    let before = std::fs::read(&cursor).unwrap();
    rig.tick_using(&mut sink)
        .expect_err("the new entry is not durable yet");
    assert_eq!(
        std::fs::read(&cursor).unwrap(),
        before,
        "the cursor did not move"
    );
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_ne!(std::fs::read(&cursor).unwrap(), before);
    assert_eq!(
        records(&rig.sink_path.with_file_name("siem.log.1")).len(),
        2
    );
}

/// copytruncate (the file is truncated to 0 under the sink): `O_APPEND` writes land at the new
/// end, and the records stay aligned.
#[cfg(unix)]
#[test]
fn a_copytruncated_file_keeps_receiving_aligned_records_at_its_new_end() {
    let rig = Rig::new();
    seed(&rig.db, 3);
    let mut sink = rig.sink();
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 3);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&rig.sink_path)
        .unwrap()
        .set_len(0)
        .unwrap();
    seed(&rig.db, 2);
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 2);
    let lines = records(&rig.sink_path);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| is_complete(line)));
    let text = std::fs::read_to_string(&rig.sink_path).unwrap();
    assert!(text.starts_with("<134>1 "), "no stale offset, no NUL hole");
}

/// A service that may only SEARCH the directory of a provisioned file appends to it: a
/// pre-existing entry owes no directory sync.
#[cfg(unix)]
#[test]
fn a_search_only_parent_directory_does_not_stop_appends_to_an_existing_file() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::new();
    seed(&rig.db, 2);
    let sub = rig.db.parent().unwrap().join("sink");
    std::fs::create_dir(&sub).unwrap();
    let path = sub.join("siem.log");
    std::fs::write(&path, "").unwrap();
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o300)).unwrap();
    let mut sink = FileSink::new(path.clone(), Arc::new(SiemExporter));
    let result = rig.tick_using(&mut sink);
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.expect("append and cursor").delivered, 2);
    assert_eq!(records(&path).len(), 2);
    assert!(rig.db.with_file_name("siem-cursor.json").exists());
}

/// R3-8: when the sink refuses a batch AND the log can no longer be read to count what remains,
/// the backlog is UNKNOWN (`None`), never reported as 0.
#[test]
fn an_unreadable_log_after_a_sink_failure_reports_an_unknown_backlog_not_zero() {
    struct RemovesTheSource(PathBuf);
    impl BatchSink for RemovesTheSource {
        fn identity(&self) -> String {
            "test:removes-the-source".to_owned()
        }
        fn deliver(
            &mut self,
            _: &[maos_siem::export::Frame],
        ) -> Result<(), maos_siem::export::SinkFailure> {
            std::fs::remove_file(&self.0).unwrap();
            Err(maos_siem::export::SinkFailure::new("siem_sink_timeout"))
        }
    }
    let rig = Rig::new();
    seed(&rig.db, 2);
    let snapshot = rig.db.with_file_name("unknown-backlog.sqlite");
    raw(&rig.db)
        .execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
        .unwrap();
    let mut sink = RemovesTheSource(snapshot.clone());
    let store = CursorStore::beside(&rig.db, &sink.identity());
    let failure =
        run_tick(&snapshot, &store, &mut sink, &|_, _| Ok(())).expect_err("the sink refuses");
    assert_eq!(failure.code(), "siem_sink_timeout");
    assert_eq!(failure.buffered(), None, "unknown, not zero");
}

/// R4-1: the gap and reset transitions are DECIDED under the read transaction but EXECUTED
/// after it ended. The journal callback (what the daemon runs as an audit write while rotation
/// may be checkpointing) runs `PRAGMA wal_checkpoint(TRUNCATE)` itself: it can only complete
/// (busy = 0) when the exporter holds no WAL reader at that moment. Live adapter, live runtime
/// entry point.
#[test]
fn gap_and_reset_transitions_hold_no_wal_reader_while_they_journal() {
    use maos_siem::export::{run_tick_live, IdleGuard};
    use std::sync::atomic::AtomicBool;
    for case in ["gap", "reset"] {
        let rig = Rig::new();
        let tl = TransparencyLogAdapter::open(&rig.db, 1).expect("live adapter");
        for n in 0..5 {
            tl.insert_frame_event(
                FrameKind::CapabilityInvocation,
                1,
                None,
                "test.capability.invoke",
                format!("row {n}").as_bytes(),
                FrameOrigin::Kernel,
            );
        }
        let mut sink = rig.sink();
        let store = CursorStore::beside(&rig.db, &sink.identity());
        if case == "gap" {
            raw(&rig.db)
                .execute_batch(
                    "DELETE FROM transparency_log WHERE rowid <= 2; \
                     INSERT INTO transparency_log_segments VALUES (1, 0, 1, 2, 2, 1, 2, 'a', NULL, 3, 9);",
                )
                .unwrap();
        } else {
            std::fs::write(store.path(), "not a cursor").unwrap();
        }
        let seen = Mutex::new(Vec::new());
        let db = rig.db.clone();
        let journal = |intent: &str, _payload: serde_json::Value| {
            let busy: i64 = raw(&db)
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
                .unwrap();
            seen.lock().unwrap().push((intent.to_owned(), busy));
            Ok(())
        };
        run_tick_live(
            &rig.db,
            &store,
            &mut sink,
            &journal,
            &AtomicBool::new(false),
            &IdleGuard::default(),
        )
        .unwrap_or_else(|failure| panic!("{case}: {failure}"));
        let seen = seen.into_inner().unwrap();
        assert!(!seen.is_empty(), "{case}: the transition journaled");
        assert!(
            seen.iter().all(|(_, busy)| *busy == 0),
            "{case}: a WAL reader was held while journaling: {seen:?}"
        );
        drop(tl);
    }
}
