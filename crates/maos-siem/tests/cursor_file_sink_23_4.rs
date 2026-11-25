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
    assert_eq!(failure.buffered(), 2);
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
