#![cfg(not(feature = "siem-fault-inject"))]
#![forbid(unsafe_code)]

//! Story 23-4 — the durable cursor under the file sink (`MAOS_SIEM_FILE` alone keeps today's
//! sink, now with a cursor), and the gap rule of D-23-4-06 A at the library seam.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::cursor::CursorStore;
use maos_siem::export::{run_tick, BatchSink, ExportError, FileSink, Tick};
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

    fn tick(&self) -> Result<Tick, ExportError> {
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
    assert_eq!(
        journal[0].1,
        serde_json::json!({ "from": 1, "to": 2, "segment_seq": 1 }),
        "ids and segment sequence, no payload"
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
