#![forbid(unsafe_code)]

//! Integration test: on_idle substrate (AC4).
//!
//! Verifies that the IdleWatchdog detects mailbox quiescence and fires
//! on_idle for Running spirits whose manifest enables the hook.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::HookDispatcher,
    idle_watchdog::{pick_poll_interval, IdleWatchdog},
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct IdleSpirit {
    on_idle_count: AtomicU32,
}

impl Default for IdleSpirit {
    fn default() -> Self {
        Self {
            on_idle_count: AtomicU32::new(0),
        }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for IdleSpirit {
    fn on_idle(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_idle_count.fetch_add(1, Ordering::Relaxed);
    }
}

fn make_scb(enabled_hooks: Vec<String>, idle_window_ms: u32) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection {
            priority_weight: 100,
            yield_every_polls: 64,
            idle_window_ms,
        },
        lifecycle: LifecycleSection { enabled_hooks },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        1,
        "idle-test".into(),
        manifest,
        make_spirit_obj(IdleSpirit::default()),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

fn make_dispatcher() -> Arc<HookDispatcher> {
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0),
    );
    let metrics = Arc::new(IacRtMetrics::new());
    Arc::new(HookDispatcher::new(tl, metrics))
}

#[test]
fn pick_poll_interval_bounds() {
    assert_eq!(
        pick_poll_interval(100),
        std::time::Duration::from_millis(100)
    );
    assert_eq!(
        pick_poll_interval(3000),
        std::time::Duration::from_millis(300)
    );
    assert_eq!(
        pick_poll_interval(30_000),
        std::time::Duration::from_millis(3000)
    );
    assert_eq!(
        pick_poll_interval(3_600_000),
        std::time::Duration::from_millis(5000)
    );
    assert_eq!(
        pick_poll_interval(10_000_000),
        std::time::Duration::from_millis(5000)
    );
}

#[tokio::test]
async fn idle_watchdog_fires_on_idle_after_quiescence() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scb = make_scb(vec!["on_idle".into()], 100); // 100ms idle window
                                                     // Simulate a past inbound frame so the idle check sees quiescence.
    scb.last_inbound_frame_ns.store(1, Ordering::Relaxed);
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(1, Arc::clone(&scb));

    let dispatcher = make_dispatcher();
    let cancel = tokio_util::sync::CancellationToken::new();
    let watchdog = Arc::new(IdleWatchdog::new(
        Arc::clone(&scbs),
        Arc::clone(&dispatcher),
    ));
    let handle = watchdog.spawn(cancel.child_token());

    // Wait for the idle window + poll interval to pass.
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

    // Cancel the watchdog.
    cancel.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await;

    // We can't downcast through AnySpiritObj easily, but we know the counter
    // should have incremented. Since we can't directly access the inner Spirit
    // through the type-erased vtable, we verify via the SCB's last_idle_fire_ns.
    assert!(
        scb.last_idle_fire_ns.load(Ordering::Relaxed) > 0,
        "on_idle must have fired (last_idle_fire_ns updated)"
    );
}

#[tokio::test]
async fn idle_watchdog_skips_paused_spirit() {
    let scb = make_scb(vec!["on_idle".into()], 100);
    scb.state
        .store(ScbLifecycleState::Paused as u8, Ordering::Release);
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(1, Arc::clone(&scb));

    let dispatcher = make_dispatcher();
    let cancel = tokio_util::sync::CancellationToken::new();
    let watchdog = Arc::new(IdleWatchdog::new(
        Arc::clone(&scbs),
        Arc::clone(&dispatcher),
    ));
    let handle = watchdog.spawn(cancel.child_token());

    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
    cancel.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await;

    assert_eq!(
        scb.last_idle_fire_ns.load(Ordering::Relaxed),
        0,
        "Paused spirit must NOT fire on_idle"
    );
}

#[tokio::test]
async fn idle_watchdog_skips_manifest_disabled_hook() {
    let scb = make_scb(vec![], 100); // empty enabled_hooks → on_idle NOT declared
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(1, Arc::clone(&scb));

    let dispatcher = make_dispatcher();
    let cancel = tokio_util::sync::CancellationToken::new();
    let watchdog = Arc::new(IdleWatchdog::new(
        Arc::clone(&scbs),
        Arc::clone(&dispatcher),
    ));
    let handle = watchdog.spawn(cancel.child_token());

    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
    cancel.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await;

    assert_eq!(
        scb.last_idle_fire_ns.load(Ordering::Relaxed),
        0,
        "Spirit without on_idle in manifest must NOT fire"
    );
}
