//! Integration test: LogRecallAdapter fires IsolationHookPoint hooks.
//!
//! Only runs when the `spirit_test` feature is enabled.

#![cfg(feature = "spirit_test")]

use std::sync::Arc;

use maos_domain::log_recall::LogRecallFilter;
use maos_domain::ports::LogRecallPort;

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_spirit_sdk::spirit_test::DefaultIsolationHook;
use parking_lot::Mutex;

fn seed_frames(tl: &Arc<TransparencyLogAdapter>, pid: u32, count: usize) {
    for i in 0..count {
        let payload = format!("payload-{pid}-{i}");
        let _token = tl.insert_frame_event(
            FrameKind::TaskAssign,
            pid,
            None,
            "delegate",
            payload.as_bytes(),
            FrameOrigin::HumanAuthored,
        );
    }
}

#[test]
#[cfg_attr(not(feature = "spirit_test"), ignore)]
fn isolation_hooks_fire_on_recall_and_fetch() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE431));
    seed_frames(&tl, 10, 3);

    let hook = Arc::new(Mutex::new(DefaultIsolationHook::default()));
    let adapter = LogRecallAdapter::new(Arc::clone(&tl)).with_isolation_hook(hook.clone());

    // recall fires hooks
    let page = adapter.recall(10, LogRecallFilter::default()).unwrap();
    let frame_id = page.entries[0].frame_id;

    // fetch fires hooks
    let _resp = adapter.fetch(10, frame_id).unwrap();

    let records = hook.lock().records.clone();
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "before_spirit_a_attempt"),
        "before_spirit_a_attempt should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "after_spirit_a_attempt"),
        "after_spirit_a_attempt should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "before_spirit_b_observe"),
        "before_spirit_b_observe should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "after_spirit_b_observe"),
        "after_spirit_b_observe should fire"
    );
}
