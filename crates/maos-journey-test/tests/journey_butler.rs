#![forbid(unsafe_code)]

//! JB-1 and JB-2 remain RED / ignored pending the Story 8.15 harness.

#[test]
#[ignore = "RED: 8.15 harness not built"]
fn jb1_calendar_slack_linear_figma_live_integration() {
    // Story 8.15 PTY-level integration test
}

#[test]
#[ignore = "RED: 8.15 harness not built"]
fn jb2_director_option_pick_dispatches_to_butler() {
    // Story 8.15 PTY-level integration test
}

use std::time::{SystemTime, UNIX_EPOCH};

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn jb4_driver_integration_test() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("transparency.sqlite");
    let journal = tmp.path().join("journal.ndjson");

    let tl = maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db, 0x123).unwrap();
    let _ = tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete,
        1,
        None,
        "write live drivers",
        b"done",
        maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
    );
    let expected_id = tl.last_frame_id();

    let butler = butler::Butler::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let digest = butler.morning_digest(&db, &journal, now, &[], 0.0).unwrap();

    assert!(!digest.completed.is_empty());
    assert_eq!(digest.completed[0].source_log_ref, hex(&expected_id));
}
