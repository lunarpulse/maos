//! Story 11.4c Task 4 (AC4) — SIEM forward-count derive-and-reconcile (RED-first).
//!
//! Pins AC4 / F5 (story `11-4c-…siem.md:141,165`): the forwarded-record count is
//! **DERIVED from the real TL tail** — never a committed literal — and an empty
//! TL is reported **N/A, not a vacuous pass**. The empty-TL leg is the named
//! canned-green trap for this slice: a SIEM export that silently no-ops and
//! forwards nothing must NOT read as "0 forwarded, all clean".
//!
//! ## The contract this suite defends
//!
//! `export_report_from_tl` reconciles the rows actually read out of a real,
//! quiesced Transparency Log and reports them as `forwarded_count: Option<usize>`:
//!
//! - **`Some(n)`** — a real TL tail was read and reconciled; `n` is the count of
//!   rows actually projected for forwarding. The two-row case below pins `n == 2`
//!   because exactly two distinct frames were written — a dropped row (2→1) or a
//!   hardcoded count reddens the test.
//! - **`None`** — the TL was empty (zero rows). Reported N/A, NOT a green zero.
//!   `Some(0)` is the vacuous-pass shape a silent-no-op export would emit; `None`
//!   forces an explicit unmeasured disposition the empty-TL case demands.
//!
//! This mirrors the `JourneyResult::not_measured` disposition precedent
//! (`maos-bench/src/report.rs`): a measurement that did not happen is a typed
//! `None`, never a plausible `Some(0)`.
//!
//! **RED** until `maos-siem` ships `export_report_from_tl` (+ `ExportReport`).
//! The format slices (`format_projection.rs`) and the redaction-before-forward
//! slices (`redaction_before_forward.rs`) are GREEN; this file pins the
//! count/disposition layer on top of them.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_audit::AuditFilter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::{export_report_from_tl, ExportReport};
use tempfile::TempDir;

/// Seed an isolated TL with two DISTINCT real frames, then drop the writer so the
/// read-only export path sees a quiesced / checkpointed DB (mirrors
/// `redaction_before_forward.rs`'s setup, including the `drop(tl)` that satisfies
/// `query_with_redaction`'s no-active-WAL precondition). Two distinct rows —
/// different `spirit_pid` / kind / payload — so the reconciled count is exactly 2
/// with no dedup collapse.
fn seed_tl_with_two_rows() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 1).unwrap());

    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1, // spirit_pid — row 1
        None,
        "test.capability.invoke",
        b"first real frame payload",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        2, // spirit_pid — distinct from row 1
        None,
        "test.capability.revoke",
        b"second real frame payload",
        FrameOrigin::Kernel,
    );

    drop(tl); // quiesce / checkpoint the writer for deterministic export
    (dir, db_path)
}

/// Open a real TL schema but insert NOTHING — a genuinely empty (zero-row) TL —
/// then drop the writer so the read path sees a checkpointed, WAL-free DB. This
/// is the disposition-discriminator fixture: the export reads zero rows, and the
/// report MUST classify that as N/A, not a measured green zero.
fn seed_empty_tl() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 1).unwrap());
    drop(tl); // quiesce so query_with_redaction's no-active-WAL precondition holds
    (dir, db_path)
}

#[test]
fn forwarded_count_derives_from_two_real_tl_rows() {
    let (_dir, db_path) = seed_tl_with_two_rows();

    let report: ExportReport = export_report_from_tl(&db_path, AuditFilter::default())
        .expect("export_report_from_tl must succeed for a real, quiesced TL");

    // Derive-and-reconcile: `forwarded_count` must come from the 2 real TL rows
    // actually written and projected, NEVER a committed literal. A bug that drops
    // a row (2→1), hardcodes the count, or misclassifies a non-empty TL as N/A
    // (Some→None) reddens this assertion.
    assert_eq!(
        report.forwarded_count,
        Some(2),
        "forwarded_count must DERIVE from the 2 real TL rows written — a dropped \
         row, a hardcoded count, or a Some→None misclassification must red: \
         {report:?}"
    );
}

#[test]
fn empty_tl_reports_forwarded_count_none_not_a_green_zero() {
    let (_dir, db_path) = seed_empty_tl();

    let report: ExportReport = export_report_from_tl(&db_path, AuditFilter::default())
        .expect("export_report_from_tl must succeed for a quiesced empty TL");

    // AC4 / F5 landmine: an empty TL MUST report `forwarded_count == None` (N/A),
    // NOT `Some(0)`. `Some(0)` is the vacuous-pass trap — a silent-no-op export
    // that forwards nothing would read as "0 forwarded, all clean" and hide the
    // regression behind a green zero. `None` forces an explicit unmeasured
    // disposition; returning `Some(_)` for an empty TL reddens this assertion.
    assert_eq!(
        report.forwarded_count, None,
        "an empty TL must report forwarded_count == None (N/A), NOT Some(0) — a \
         green zero hides a silent-no-op export regression: {report:?}"
    );
}

#[test]
fn non_empty_tl_with_zero_filter_matches_reports_some_zero_not_none() {
    let (_dir, db_path) = seed_tl_with_two_rows();

    // A filter that matches NO row, while the TL is genuinely non-empty (2 rows).
    let filter = AuditFilter {
        spirit_pid: Some(9999),
        ..Default::default()
    };

    let report = export_report_from_tl(&db_path, filter)
        .expect("export_report_from_tl must succeed for a non-empty TL");

    // AC4 disposition split: a NON-empty TL whose filter matches nothing must
    // report Some(0) (a measured zero), NOT None (empty-TL / N/A). Conflating
    // the two would hide a filter-miss behind an unmeasured N/A — the exact
    // ambiguity this slice exists to kill.
    assert_eq!(
        report.forwarded_count,
        Some(0),
        "a non-empty TL with zero filter matches must report Some(0) (measured \
         zero), NOT None (empty-TL N/A) — conflation hides a filter-miss: {report:?}"
    );
}
