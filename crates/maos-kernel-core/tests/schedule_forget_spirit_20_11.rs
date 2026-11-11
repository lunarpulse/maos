#![forbid(unsafe_code)]

//! Story 20-11 KS (D-20-11-04 A) — `ScheduleWatchdog::forget_spirit`.
//!
//! The watchdog keeps a rate bucket per `(spirit_id, schedule_id)` and a
//! last-fire stamp per `(pid, schedule_id)` for the life of the process.
//! An uninstall drops both for that Spirit and nobody else; a same-id
//! reinstall in the same root then starts with a fresh bucket instead of
//! inheriting the exhausted one.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::HookDispatcher,
    schedule_watchdog::ScheduleWatchdog,
};
use maos_kernel_core::security::manifest::{LifecycleSection, ScheduleEntry, SchedulesSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

#[derive(Default)]
struct Quiet;
impl maos_spirit_abi::lifecycle::Spirit for Quiet {}

/// A running Spirit whose one schedule may fire once per hour.
fn scb(pid: u32, spirit_id: &str) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_schedule".into()],
        },
        schedules: SchedulesSection {
            entries: vec![ScheduleEntry {
                id: "daily".into(),
                cadence_secs: 1,
                payload_bytes: Vec::new(),
                rate_limit_per_hour: 1,
                compliance_claim_ref: None,
                principal_revocability: false,
                side_effect_scopes: Vec::new(),
            }],
        },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(pid, spirit_id.into(), manifest, make_spirit_obj(Quiet), 0);
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

fn fires(tl: &TransparencyLogAdapter, pid: u32) -> usize {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    })
    .expect("fire rows")
    .iter()
    .filter(|row| row.intent == "schedule.fire:daily")
    .count()
}

/// Run the watchdog until every pid in `until` has a fire row.
async fn run_until(watchdog: &Arc<ScheduleWatchdog>, tl: &TransparencyLogAdapter, until: &[u32]) {
    std::env::set_var("MAOS_SCHEDULE_FAST", "1");
    let cancel = tokio_util::sync::CancellationToken::new();
    let handle = Arc::clone(watchdog).spawn(cancel.child_token());
    let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while until.iter().any(|pid| fires(tl, *pid) == 0) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    cancel.cancel();
    handle.await.expect("watchdog shuts down cleanly");
    observed.expect("every expected schedule fired within the deadline");
}

#[tokio::test]
async fn forget_spirit_drops_only_that_spirits_buckets_and_stamps() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x2011));
    let dispatcher = Arc::new(HookDispatcher::new(
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    ));
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    for (pid, id) in [(1, "gone"), (2, "kept")] {
        scbs.write().expect("scbs").insert(pid, scb(pid, id));
    }
    let watchdog = Arc::new(ScheduleWatchdog::new(
        Arc::clone(&scbs),
        dispatcher,
        Arc::clone(&tl),
    ));
    run_until(&watchdog, &tl, &[1, 2]).await;
    assert_ne!(watchdog.last_fire_ns(1, "daily"), 0);
    assert_ne!(watchdog.last_fire_ns(2, "daily"), 0);

    // One bucket (by Spirit id) plus one stamp (by pid) belong to `gone`.
    assert_eq!(watchdog.forget_spirit("gone", 1), 2);
    assert_eq!(watchdog.last_fire_ns(1, "daily"), 0, "stamp dropped");
    assert_ne!(
        watchdog.last_fire_ns(2, "daily"),
        0,
        "another Spirit's stamp"
    );
    assert_eq!(watchdog.forget_spirit("gone", 1), 0, "idempotent");

    // The uninstalled Spirit is gone from the SCB map; a same-id reinstall
    // (pid 3) gets a fresh bucket and fires, while a re-admitted `kept`
    // (pid 4, bucket still exhausted) does not. Pid 5 is evaluated after pid 4
    // in the same pass, so its fire row proves pid 4 was refused, not skipped.
    {
        let mut map = scbs.write().expect("scbs");
        map.remove(&1);
        map.insert(3, scb(3, "gone"));
        map.insert(4, scb(4, "kept"));
        map.insert(5, scb(5, "fresh"));
    }
    run_until(&watchdog, &tl, &[3, 5]).await;
    assert_eq!(fires(&tl, 3), 1, "a reinstall must not inherit the bucket");
    assert_eq!(fires(&tl, 4), 0, "an untouched bucket stays exhausted");
}
