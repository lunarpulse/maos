//! Story 11.4c Task 4 (AC4) — SIEM localhost file-sink integration.
//!
//! Pins the [`maos_siem::forward_to_file`] sink: it tails a real, quiesced
//! Transparency Log read-only, projects each row redacted (via the sanctioned
//! `export_from_tl`), and APPENDS one RFC5424-framed CEF line per record to a
//! local file. Returns the projected count, and surfaces I/O errors instead of
//! silently dropping records. The sink is localhost-only by construction.
//!
//! Note: each test reads its TL exactly once — `maos_audit::query_with_redaction`
//! refuses a second read of the same DB (its WAL guard treats the `-wal` file a
//! first read materialises as an active writer), mirroring every other SIEM /
//! audit integration test.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_audit::AuditFilter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::forward_to_file;
use tempfile::TempDir;

/// Seed an isolated TL with two DISTINCT frames, then quiesce the writer.
fn seed_tl_with_two_rows() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 1).unwrap());
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1,
        None,
        "test.capability.invoke",
        b"first real frame payload",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        2,
        None,
        "test.capability.revoke",
        b"second real frame payload",
        FrameOrigin::Kernel,
    );
    drop(tl);
    (dir, db_path)
}

#[test]
fn forward_to_file_appends_one_syslog_line_per_record_and_returns_count() {
    let (_dir, db_path) = seed_tl_with_two_rows();
    let sink = TempDir::new().unwrap();
    let sink_path = sink.path().join("siem.log");

    // Pre-seed the sink with a sentinel line to prove the sink APPENDS rather
    // than truncates (the second-read pattern other suites use is unavailable
    // here — see the module doc on the WAL guard).
    std::fs::write(&sink_path, "PRE-EXISTING-SENTINEL\n").unwrap();

    let count = forward_to_file(&db_path, AuditFilter::default(), &sink_path)
        .expect("forward_to_file must succeed for a real TL + writable sink");
    assert_eq!(
        count, 2,
        "forward_to_file must return the projected record count (2 rows)"
    );

    let written = std::fs::read_to_string(&sink_path).expect("sink file must be readable");
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "sink must hold sentinel + 2 projected lines (append, not truncate)"
    );
    assert_eq!(
        lines[0], "PRE-EXISTING-SENTINEL",
        "append must preserve pre-existing content"
    );

    for line in &lines[1..] {
        // Each appended line is an RFC5424-framed CEF record.
        assert!(
            line.starts_with("<134>1 "),
            "each appended line must be an RFC5424 frame, got: {line}"
        );
        assert!(
            line.contains("CEF:0|MAOS|maos-siem|"),
            "each appended line must carry the CEF message, got: {line}"
        );
        assert!(
            !line.contains('\0'),
            "no raw NUL may survive into the sink line: {line:?}"
        );
    }
}

#[test]
fn forward_to_file_surfaces_io_error_instead_of_silently_dropping() {
    let (_dir, db_path) = seed_tl_with_two_rows();
    let sink = TempDir::new().unwrap();
    // A path whose parent does not exist — opening for append must fail.
    let sink_path = sink.path().join("no-such-dir").join("siem.log");

    let err = forward_to_file(&db_path, AuditFilter::default(), &sink_path)
        .expect_err("forward to an unwritable path must surface an error");
    assert!(
        matches!(err, maos_siem::SiemError::Io(_)),
        "unwritable sink must surface SiemError::Io, not silently drop: {err:?}"
    );
}
