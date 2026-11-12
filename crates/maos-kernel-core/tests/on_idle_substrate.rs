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
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
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
    // Story 5.1 review backfill — corrected test. Per kernel_invocation_allowed,
    // empty enabled_hooks means "all hooks allowed" (matches
    // `kernel_invocation_allowed(&[], _) → true`). To verify manifest gating
    // ACTUALLY skips on_idle, the manifest must declare a NON-EMPTY enabled_hooks
    // subset that EXCLUDES on_idle.
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scb = make_scb(vec!["on_start".into()], 100); // on_idle NOT in subset
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

    // After the review backfill, the watchdog only updates last_idle_fire_ns
    // when the hook actually fired (not SkippedManifest). A Spirit whose
    // manifest excludes on_idle MUST leave last_idle_fire_ns at 0.
    assert_eq!(
        scb.last_idle_fire_ns.load(Ordering::Relaxed),
        0,
        "Spirit without on_idle in enabled_hooks must not advance last_idle_fire_ns"
    );
}

/// A Spirit whose `on_idle` rendezvous with a peer's `on_idle`: the waiter
/// blocks until the peer has fired (bounded), the signaller fires at once.
struct RendezvousSpirit {
    wait_for: Option<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>,
    signal: Option<std::sync::Mutex<std::sync::mpsc::Sender<()>>>,
    met: Arc<std::sync::atomic::AtomicBool>,
}

impl maos_spirit_abi::lifecycle::Spirit for RendezvousSpirit {
    fn on_idle(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        if let Some(rx) = &self.wait_for {
            let seen = rx
                .lock()
                .expect("rendezvous lock")
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_ok();
            self.met.store(seen, Ordering::SeqCst);
        }
        if let Some(tx) = &self.signal {
            let _ = tx.lock().expect("rendezvous lock").send(());
        }
    }
}

fn rendezvous_scb(pid: u32, spirit: RendezvousSpirit) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection {
            priority_weight: 100,
            yield_every_polls: 64,
            idle_window_ms: 100,
        },
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_idle".into()],
        },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        pid,
        format!("idle-{pid}"),
        manifest,
        make_spirit_obj(spirit),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    scb.last_inbound_frame_ns.store(1, Ordering::Relaxed);
    Arc::new(scb)
}

/// 20-9 — one Spirit's long `on_idle` does not hold back another Spirit's
/// trigger in the same poll: the lower pid's hook waits for the higher pid's
/// hook to have fired, which only happens when the fires run side by side.
#[tokio::test]
async fn idle_watchdog_fires_due_spirits_side_by_side() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let (tx, rx) = std::sync::mpsc::channel();
    let met = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let waiter = rendezvous_scb(
        1,
        RendezvousSpirit {
            wait_for: Some(std::sync::Mutex::new(rx)),
            signal: None,
            met: Arc::clone(&met),
        },
    );
    let signaller = rendezvous_scb(
        2,
        RendezvousSpirit {
            wait_for: None,
            signal: Some(std::sync::Mutex::new(tx)),
            met: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        },
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(1, Arc::clone(&waiter));
    scbs.write().unwrap().insert(2, Arc::clone(&signaller));

    let cancel = tokio_util::sync::CancellationToken::new();
    let watchdog = Arc::new(IdleWatchdog::new(Arc::clone(&scbs), make_dispatcher()));
    let handle = watchdog.spawn(cancel.child_token());
    let both_fired = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        while waiter.last_idle_fire_ns.load(Ordering::Acquire) == 0
            || signaller.last_idle_fire_ns.load(Ordering::Acquire) == 0
        {
            tokio::task::yield_now().await;
        }
    })
    .await;
    cancel.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), handle).await;

    assert!(both_fired.is_ok(), "both Spirits fired on_idle");
    assert!(
        met.load(Ordering::SeqCst),
        "the waiter saw the peer's on_idle fire while it was still held"
    );
}
