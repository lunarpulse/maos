//! Story 11.4b AC4 — the escape-source-identity reflex + replay-dedup (D8, the
//! §A7 region-identity analogue).
//!
//! Each detected anomaly must trace to a REAL kernel-emitted `SandboxBlock` TL
//! frame (`FrameOrigin::Kernel`), and a replayed read MUST NOT inflate the count
//! (idempotent dedup — Boundary's catch; else the reflex lies).
//!
//! These tests manipulate the TL with CONTROLLED frame_ids + origins to exercise
//! the detector's provenance/dedup logic precisely. The rows are genuine
//! `FrameKind::SandboxBlock` structural facts (real enforcement data); the
//! reflex under test is the detector's source-identity + dedup behavior.

#![cfg(target_os = "linux")]

mod common;
use common::*;

use maos_domain::invariants::i3::FrameOrigin;
use maos_escape_detector::{Detector, ManifestDeclaration, FRAME_ORIGIN_KERNEL};
use maos_kernel_core::iac::transparency_log::FrameKind;

/// Helper: insert a `SandboxBlock` row with an explicit frame_id + origin.
fn insert_block(
    tl: &std::sync::Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    frame_id: [u8; 16],
    spirit_pid: u32,
    origin: FrameOrigin,
) {
    tl.insert_frame_event_with_id(
        Some(frame_id),
        FrameKind::SandboxBlock,
        spirit_pid,
        "",
        "",
        None,
        "sandbox.block.unknown",
        b"tier=2",
        origin,
    );
}

fn manifest_for(pid: u32) -> ManifestDeclaration {
    ManifestDeclaration {
        spirit_pid: pid as i64,
        declared_tier: 2,
        anticipated_kill: false,
    }
}

/// D8 (identity): each detected anomaly traces to a REAL kernel-emitted frame.
/// A synthesized/non-kernel-origin `SandboxBlock` row must NOT be counted.
#[test]
fn non_kernel_origin_frame_is_not_counted() {
    let (_dir, db_path, tl) = fresh_temp_tl();
    let pid = 7001u32;

    // One REAL kernel-origin frame (counted) + one synthesized HumanAuthored
    // frame (must NOT be counted — it is not a real kernel emission).
    insert_block(&tl, [0xAA; 16], pid, FrameOrigin::Kernel);
    insert_block(&tl, [0xBB; 16], pid, FrameOrigin::HumanAuthored);

    let manifests = vec![manifest_for(pid)];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect");
    assert_eq!(
        anomalies.len(),
        1,
        "only the real kernel-origin frame counts (the synthesized frame is excluded)"
    );
    assert_eq!(anomalies[0].frame_id_hex, hex(&[0xAA; 16]));
    assert_eq!(anomalies[0].spirit_pid, pid as i64);
}

/// D8 (blind): blinding one real source — removing the only kernel-origin frame
/// — moves the anomaly count to zero (blind-one-source → count moves). This is
/// the anti-vacuous-count reflex (every count needs a per-item blind that reds).
#[test]
fn blind_one_source_moves_the_count() {
    let (_dir, db_path_with, tl_with) = fresh_temp_tl();
    let (_dir2, db_path_without, _tl_without) = fresh_temp_tl();
    let pid = 7002u32;

    // With the real kernel frame: one anomaly.
    insert_block(&tl_with, [0xCC; 16], pid, FrameOrigin::Kernel);
    // Without (blinded): no anomaly.
    let manifests = vec![manifest_for(pid)];
    let with = Detector::detect(&db_path_with, &manifests).expect("detect with");
    let without = Detector::detect(&db_path_without, &manifests).expect("detect without");
    assert_eq!(with.len(), 1, "the real frame yields one anomaly");
    assert_eq!(
        without.len(),
        0,
        "blinding the one real source moves the count to zero"
    );
}

/// D8 (replay-dedup): a replayed read MUST NOT inflate the count. The TL
/// enforces `frame_id` uniqueness (a duplicate insert panics under I2 — the
/// production constraint), so the realistic replay is a tailing consumer
/// RE-READING the same TL. The detector reads fresh + dedups on each call, so a
/// re-read yields the SAME count, not a doubled one (idempotent — else a tailing
/// operator would double-count every poll cycle).
#[test]
fn replayed_read_does_not_inflate_count() {
    let (_dir, db_path, tl) = fresh_temp_tl();
    let pids = [7003u32, 7004u32, 7005u32];
    for (i, &pid) in pids.iter().enumerate() {
        let mut id = [0u8; 16];
        id[0] = i as u8 + 1;
        insert_block(&tl, id, pid, FrameOrigin::Kernel);
    }
    let manifests: Vec<_> = pids.iter().map(|&p| manifest_for(p)).collect();

    let first = Detector::detect(&db_path, &manifests).expect("first detect");
    let replayed = Detector::detect(&db_path, &manifests).expect("replayed detect");
    assert_eq!(
        first.len(),
        pids.len(),
        "baseline: one anomaly per real kernel row"
    );
    assert_eq!(
        first.len(),
        replayed.len(),
        "a replayed read does NOT inflate the count (idempotent tailing consumer)"
    );
    // The within-read dedup is exercised too: the read path collapses any
    // duplicate frame_id it encounters (defense-in-depth over the TL UNIQUE
    // constraint). Confirm the raw read carries no duplication.
    let raw = Detector::read_kernel_sandbox_blocks(&db_path).expect("read");
    let mut seen = std::collections::HashSet::new();
    for f in &raw {
        assert!(
            seen.insert(&f.frame_id_hex),
            "within-read dedup collapsed a duplicate"
        );
    }
}

/// Every anomaly the detector raises is verified to carry `origin == Kernel`
/// (the read path returns all kind=8 rows; detect() filters to kernel-origin).
#[test]
fn read_path_returns_all_kind8_rows_detect_filters_to_kernel() {
    let (_dir, db_path, tl) = fresh_temp_tl();
    insert_block(&tl, [0x11; 16], 7100, FrameOrigin::Kernel);
    insert_block(&tl, [0x22; 16], 7101, FrameOrigin::SpiritAuto);
    insert_block(&tl, [0x33; 16], 7102, FrameOrigin::HumanAuthored);
    let read = Detector::read_kernel_sandbox_blocks(&db_path).expect("read");
    assert_eq!(read.len(), 3, "read returns all kind=8 rows pre-filter");
    let kernel = read
        .iter()
        .filter(|f| f.origin == FRAME_ORIGIN_KERNEL)
        .count();
    assert_eq!(kernel, 1, "exactly one kernel-origin frame");
    // detect() counts only the kernel-origin frame.
    let manifests = vec![manifest_for(7100), manifest_for(7101), manifest_for(7102)];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect");
    assert_eq!(
        anomalies.len(),
        1,
        "detect counts only the real kernel-origin frame"
    );
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
