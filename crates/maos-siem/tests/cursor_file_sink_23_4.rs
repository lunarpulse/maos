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

    fn lines(&self) -> usize {
        std::fs::read_to_string(&self.sink_path)
            .unwrap_or_default()
            .lines()
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

/// A partial append leaves an unterminated record; the retry cuts it back to the last line
/// terminator before appending, so the file never holds a fragment.
#[test]
fn a_partial_tail_is_cut_back_before_the_retry_appends() {
    let rig = Rig::new();
    seed(&rig.db, 2);
    std::fs::write(
        &rig.sink_path,
        "earlier line\n<134>1 PARTIAL-RECORD-WITHOUT-NEWLINE",
    )
    .unwrap();
    assert_eq!(rig.tick().unwrap().delivered, 2);
    let text = std::fs::read_to_string(&rig.sink_path).unwrap();
    assert!(!text.contains("PARTIAL"), "{text}");
    assert!(text.starts_with("earlier line\n"));
    assert_eq!(text.lines().count(), 3);
    assert!(text.ends_with('\n'));
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

// ── review round 3 ───────────────────────────────────────────────────────────

/// An appender that writes half of the batch and then fails once (ENOSPC mid-write).
fn short_write_once() -> maos_siem::export::Appender {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    let failing = Arc::new(AtomicBool::new(true));
    Arc::new(move |path, bytes| {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        if failing.swap(false, Ordering::SeqCst) {
            file.write_all(&bytes[..bytes.len() / 2])?;
            return Err(std::io::Error::other("injected ENOSPC after a short write"));
        }
        file.write_all(bytes)?;
        file.sync_data()
    })
}

fn assert_whole_lines(path: &Path, expected: usize) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.ends_with('\n'), "the file ends at a record boundary");
    assert_eq!(text.lines().count(), expected);
    for line in text.lines() {
        assert!(
            line.starts_with("<134>1 ") && line.contains(" CEF:0|"),
            "a malformed record was acknowledged: {line:?}"
        );
    }
}

/// R3-2: a short write leaves a fragment; the retry cuts the file back to the offset recorded
/// before the failed append (write permission alone is enough, so a WRITE-ONLY file works),
/// and every record boundary is intact afterwards.
#[cfg(unix)]
#[test]
fn a_short_write_is_cut_back_before_the_retry_even_on_a_write_only_file() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::new();
    seed(&rig.db, 3);
    std::fs::write(&rig.sink_path, "").unwrap();
    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o200)).unwrap();
    let mut sink = rig.sink().with_appender(short_write_once());
    let cursor = rig.db.with_file_name("siem-cursor.json");
    let failure = rig
        .tick_using(&mut sink)
        .expect_err("the short write fails");
    assert_eq!(failure.code(), "siem_sink_file_io");
    assert!(!cursor.exists());
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 3);
    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_whole_lines(&rig.sink_path, 3);
}

/// R3-2: a known partial tail that cannot be repaired (no write permission to cut it) is never
/// appended to and acknowledged: the cursor stays and the error is typed; once the file is
/// writable again the batch goes out whole.
#[cfg(unix)]
#[test]
fn an_unrepairable_partial_tail_refuses_to_advance_the_cursor() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::new();
    seed(&rig.db, 3);
    let mut sink = rig.sink().with_appender(short_write_once());
    let cursor = rig.db.with_file_name("siem-cursor.json");
    rig.tick_using(&mut sink)
        .expect_err("the short write fails");
    let fragment = std::fs::read(&rig.sink_path).unwrap();
    assert!(!fragment.is_empty() && !fragment.ends_with(b"\n"));

    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o400)).unwrap();
    let failure = rig
        .tick_using(&mut sink)
        .expect_err("the tail cannot be cut");
    assert_eq!(failure.code(), "siem_file_tail_unrecoverable");
    assert!(!cursor.exists(), "the cursor did not move");
    assert_eq!(
        std::fs::read(&rig.sink_path).unwrap(),
        fragment,
        "nothing was appended"
    );

    std::fs::set_permissions(&rig.sink_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 3);
    assert_whole_lines(&rig.sink_path, 3);
}

/// R3-3: a service that may only SEARCH the directory of a provisioned file appends to it
/// (no directory sync is required for a pre-existing entry).
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
    assert_whole_lines(&path, 2);
    assert!(rig.db.with_file_name("siem-cursor.json").exists());
}

/// R3-3: a pathname replaced between attempts does not inherit the directory-sync debt (or
/// credit) of the file this sink created: the new entry is not ours to sync.
#[test]
fn a_replaced_file_does_not_inherit_the_directory_sync_state_of_the_old_one() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let rig = Rig::new();
    seed(&rig.db, 2);
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let mut sink = rig.sink().with_dir_sync(Arc::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        Err(std::io::Error::other("injected dir sync failure"))
    }));
    rig.tick_using(&mut sink)
        .expect_err("the created entry cannot be synced");
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    // Replace the entry with ANOTHER file (the new one exists while the old one does, so
    // the inode numbers differ).
    let other = rig.sink_path.with_file_name("replacement.log");
    std::fs::write(&other, "").unwrap();
    std::fs::rename(&other, &rig.sink_path).unwrap();
    assert_eq!(rig.tick_using(&mut sink).unwrap().delivered, 2);
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "no directory sync is owed for an entry this sink did not create"
    );
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
