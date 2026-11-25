#![forbid(unsafe_code)]

//! Story 23-4 (D-23-4-06 A, D-23-4-07 A) — the insertion-id reader the SIEM cursor resumes
//! from, and the `VACUUM INTO` claim its snapshot reads depend on (AC3).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maos_audit::insertion::{ExpiredRange, LogView};
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use tempfile::TempDir;

fn raw(db: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).expect("raw connection")
}

fn rows(db: &Path) -> Vec<(i64, String)> {
    let connection = raw(db);
    let mut statement = connection
        .prepare("SELECT rowid, hex(frame_id) FROM transparency_log ORDER BY rowid")
        .unwrap();
    let mapped = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    mapped.map(Result::unwrap).collect()
}

fn seeded(count: usize) -> (TempDir, PathBuf, Arc<TransparencyLogAdapter>) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = Arc::new(TransparencyLogAdapter::open(&db, 1).unwrap());
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
    (dir, db, tl)
}

/// AC3: `VACUUM INTO` keeps the rowid of every row of a log whose prefix was rotated away
/// (ADR-072 deletes the sealed prefix, so the live rowids start above 1). The snapshot is
/// taken the way the daemon takes it: from a second connection while the adapter holds the
/// live WAL. SQLite 3.45.0 (`sqlite3.c:135136-135140`) copies source rowids for `VACUUM INTO`.
#[test]
fn vacuum_into_keeps_the_insertion_ids_of_a_rotated_log() {
    let (dir, db, tl) = seeded(10);
    raw(&db)
        .execute("DELETE FROM transparency_log WHERE rowid <= 5", [])
        .unwrap();
    let live = rows(&db);
    assert_eq!(
        live.first().map(|row| row.0),
        Some(6),
        "the live ids start above 1"
    );
    assert!(
        db.with_extension("sqlite-wal").exists(),
        "the adapter still holds the WAL"
    );

    let snapshot = dir.path().join("transparency.siem-snapshot-1-0.sqlite");
    raw(&db)
        .execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
        .unwrap();
    assert_eq!(
        rows(&snapshot),
        live,
        "VACUUM INTO preserves rowids and rows"
    );

    // The sanctioned reader sees the same insertion ids through the snapshot.
    let read = LogView::open(&snapshot)
        .unwrap()
        .read_after(0, 100)
        .unwrap();
    assert_eq!(
        read.iter().map(|row| row.insertion_id).collect::<Vec<_>>(),
        live.iter().map(|row| row.0).collect::<Vec<_>>()
    );
    drop(tl);
}

#[test]
fn the_reader_resumes_after_an_insertion_id_in_order_through_the_redaction_mapper() {
    let (_dir, db, tl) = seeded(6);
    drop(tl);
    let view = LogView::open(&db).unwrap();
    let all = view.read_after(0, 100).unwrap();
    assert_eq!(all.len(), 6);
    assert!(all
        .windows(2)
        .all(|pair| pair[0].insertion_id < pair[1].insertion_id));

    let tail = view.read_after(all[2].insertion_id, 100).unwrap();
    assert_eq!(
        tail.iter()
            .map(|row| &row.entry.frame_id_hex)
            .collect::<Vec<_>>(),
        all[3..]
            .iter()
            .map(|row| &row.entry.frame_id_hex)
            .collect::<Vec<_>>()
    );
    assert_eq!(view.read_after(0, 2).unwrap().len(), 2, "limit");
    assert_eq!(view.count_after(all[2].insertion_id).unwrap(), 3);

    // The sanctioned mapper populates the redaction provenance; the fault twin drops it.
    #[cfg(not(feature = "siem-fault-inject"))]
    assert!(all.iter().all(|row| row.entry.redaction.is_some()));
    #[cfg(feature = "siem-fault-inject")]
    assert!(all.iter().all(|row| row.entry.redaction.is_none()));

    let anchor = view.probe_anchor(all[3].insertion_id).unwrap();
    assert_eq!(
        anchor.frame_id_hex.as_deref(),
        Some(all[3].entry.frame_id_hex.as_str())
    );
    assert_eq!(anchor.high_water, all[5].insertion_id);
    assert_eq!(view.probe_anchor(999).unwrap().frame_id_hex, None);
}

#[test]
fn a_live_wal_is_refused_like_every_other_export_read() {
    let (_dir, db, _tl) = seeded(2);
    assert!(db.with_extension("sqlite-wal").exists());
    assert!(LogView::open(&db).is_err());
}

#[test]
fn a_registered_segment_whose_file_is_missing_is_an_open_error_never_a_shorter_log() {
    let (_dir, db, tl) = seeded(3);
    drop(tl);
    raw(&db)
        .execute_batch("INSERT INTO transparency_log_segments VALUES (2, 40, 41, 80, 40, 1, 2, 'b', 'a', 3, NULL);")
        .unwrap();
    assert!(LogView::open(&db).is_err());
}

#[test]
fn an_expired_segment_is_reported_as_a_range_beyond_the_cursor() {
    let (_dir, db, tl) = seeded(3);
    drop(tl);
    // The adapter's schema already carries the registry; segment 1 expired, its rows are gone.
    raw(&db)
        .execute_batch("INSERT INTO transparency_log_segments VALUES (1, 0, 1, 40, 40, 1, 2, 'a', NULL, 3, 9);")
        .unwrap();
    let view = LogView::open(&db).unwrap();
    let expired = ExpiredRange {
        seq: 1,
        first_rowid: 1,
        last_rowid: 40,
    };
    assert_eq!(view.expired_ranges_after(0).unwrap(), vec![expired]);
    assert!(
        view.expired_ranges_after(40).unwrap().is_empty(),
        "a cursor at the end of the range has nothing left to lose"
    );
    // The registry's ids count toward the high water: a cursor inside them is not "ahead".
    assert_eq!(view.probe_anchor(30).unwrap().high_water, 40);
}
