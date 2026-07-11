//! Story 11.4b AC1 — Boundary concurrent-read catch.
//!
//! AC1 sub-clause under test: the detector's read-only reader
//! (`Detector::read_kernel_sandbox_blocks`, opened with `SQLITE_OPEN_READ_ONLY`)
//! tolerates a half-written frame while the writer is live — i.e. a read that
//! OVERLAPS an in-flight (uncommitted) writer transaction returns `Ok` and sees
//! ONLY committed rows, never the torn / uncommitted frame.
//!
//! SQLite hands a read-only connection a committed-consistent snapshot, so an
//! open uncommitted INSERT is invisible to it. This file proves that property
//! holds end-to-end through the detector's PUBLIC read API — it would regress
//! if the read-only flag were dropped (a read-write connection would still see
//! the committed snapshot, but the test pins the read-only contract the AC
//! names) or if a future change let the reader peek at an uncommitted writer
//! transaction's staging rows.

use std::path::PathBuf;

use maos_domain::invariants::i3::FrameOrigin;
use maos_escape_detector::Detector;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use rusqlite::params;

/// Open a fresh on-disk Transparency Log in a temp dir (boot_nonce = 1) and
/// return the live adapter (the committed-rows writer), the db path, and the
/// temp-dir guard. The dir guard is held by the caller for the test's lifetime
/// so the backing file is not deleted mid-test.
fn fresh_db() -> (tempfile::TempDir, PathBuf, TransparencyLogAdapter) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("tl.db");
    let tl = TransparencyLogAdapter::open(&db_path, 1).expect("open on-disk TL");
    (dir, db_path, tl)
}

/// Lowercase-hex of a 16-byte TL `frame_id`, matching the encoding the
/// detector writes into `SandboxBlockFrame::frame_id_hex`.
fn hex16(bytes: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(32);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// AC1 (Boundary concurrent-read catch): a read-only detector read that
/// OVERLAPS a live, uncommitted writer transaction returns `Ok` and sees ONLY
/// committed rows — never the torn / uncommitted frame.
///
/// Race-free single-process technique (no threads, no timing):
///   1. The adapter inserts `N` committed kernel-origin kind=8 rows.
///   2. A SECOND raw `rusqlite::Connection` runs `BEGIN IMMEDIATE` and INSERTs
///      one more kind=8 row WITHOUT committing.
///   3. WHILE that transaction is open, `Detector::read_kernel_sandbox_blocks`
///      reads through its `SQLITE_OPEN_READ_ONLY` connection. It must return
///      `Ok` and report exactly `N` rows — the uncommitted INSERT must be
///      invisible to the read-only committed snapshot.
///   4. Committing the second writer makes the new row appear (`N + 1`),
///      proving the earlier absence was snapshot isolation, not a dropped write.
#[test]
fn read_only_reader_isolates_uncommitted_writer_transaction() {
    let (_dir, db_path, tl) = fresh_db();

    // ── baseline: N committed kernel-origin kind=8 rows via the adapter ──
    let committed_count: usize = 3;
    for i in 0..committed_count {
        let frame_id = [0x10 + i as u8; 16];
        tl.insert_frame_event_with_id(
            Some(frame_id),
            FrameKind::SandboxBlock,
            8000 + i as u32,
            "",
            "",
            None,
            "sandbox.block.unknown",
            b"tier=2",
            FrameOrigin::Kernel,
        );
    }

    // ── a second writer opens a transaction and INSERTs an uncommitted row ──
    let uncommitted_id: [u8; 16] = [0xF0; 16];
    let payload: &[u8] = b"tier=2";
    let writer = rusqlite::Connection::open(&db_path).expect("open second writer");
    writer
        .execute_batch("BEGIN IMMEDIATE;")
        .expect("begin immediate");
    writer
        .execute(
            "INSERT INTO transparency_log \
             (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, \
              boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, ?3, '', '', ?4, NULL, ?5, ?6, ?7, ?8)",
            params![
                &uncommitted_id[..],
                0_i64,
                9000_i64,
                1_i64, // boot_nonce
                8_i64, // kind = FrameKind::SandboxBlock
                "sandbox.block.unknown",
                payload,
                3_i64, // origin = FrameOrigin::Kernel
            ],
        )
        .expect("insert uncommitted row");
    // The transaction is now OPEN and uncommitted — deliberately not committed
    // until after the overlapping read below.

    // ── teeth anchor: the uncommitted row genuinely EXISTS at this instant ──
    // The writer's OWN connection (inside its open transaction) sees its
    // uncommitted insert — `committed_count + 1`. This proves the detector's
    // read below returns fewer rows because of SNAPSHOT ISOLATION, not because
    // the insert was a no-op. It is exactly the regression this guards: were the
    // detector ever to read through the writer's connection (or under
    // `read_uncommitted` + shared-cache), it would see this dirty row too.
    let writer_visible: i64 = writer
        .query_row(
            "SELECT COUNT(*) FROM transparency_log WHERE kind = ?1",
            params![8_i64],
            |row| row.get(0),
        )
        .expect("count on writer's own connection");
    assert_eq!(
        writer_visible as usize,
        committed_count + 1,
        "the writer's own connection sees its uncommitted insert (the dirty row exists)"
    );

    // ── THE PROOF: read-only detector reads mid-transaction ──
    let read = Detector::read_kernel_sandbox_blocks(&db_path)
        .expect("read-only read returns Ok while writer txn is open");
    assert_eq!(
        read.len(),
        committed_count,
        "read-only snapshot shows only the committed rows, not the open uncommitted one"
    );
    assert!(
        !read
            .iter()
            .any(|f| f.frame_id_hex == hex16(&uncommitted_id)),
        "the uncommitted frame must be invisible to the read-only committed snapshot"
    );

    // ── commit the second writer; the row is now durable ──
    writer.execute_batch("COMMIT;").expect("commit");
    drop(writer);

    // ── re-read: the newly-committed row now appears (N + 1) ──
    let read_after =
        Detector::read_kernel_sandbox_blocks(&db_path).expect("read-only read after commit");
    assert_eq!(
        read_after.len(),
        committed_count + 1,
        "the newly-committed row is visible after commit (rules out a dropped write)"
    );
    assert!(
        read_after
            .iter()
            .any(|f| f.frame_id_hex == hex16(&uncommitted_id)),
        "the previously-uncommitted row is now visible"
    );
}
