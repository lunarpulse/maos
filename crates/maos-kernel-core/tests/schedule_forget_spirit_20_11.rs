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
    scb_of(pid, spirit_id, Quiet)
}

fn scb_of(
    pid: u32,
    spirit_id: &str,
    spirit: impl maos_spirit_abi::lifecycle::Spirit + Send + Sync + 'static,
) -> Arc<SpiritControlBlock> {
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
    let scb = SpiritControlBlock::new(pid, spirit_id.into(), manifest, make_spirit_obj(spirit), 0);
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
    assert_eq!(watchdog.forget_spirit("gone", 1).await, 2);
    assert_eq!(watchdog.last_fire_ns(1, "daily"), 0, "stamp dropped");
    assert_ne!(
        watchdog.last_fire_ns(2, "daily"),
        0,
        "another Spirit's stamp"
    );
    assert_eq!(watchdog.forget_spirit("gone", 1).await, 0, "idempotent");

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

/// Holds the watchdog inside Spirit A's schedule hook until released.
struct Blocking {
    entered: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<()>>>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
impl maos_spirit_abi::lifecycle::Spirit for Blocking {
    fn on_schedule(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        _payload: &maos_spirit_abi::lifecycle::SchedulePayload<'_>,
    ) {
        if let Some(entered) = self.entered.lock().expect("entered").take() {
            entered.send(()).expect("test observes the hook");
        }
        self.release
            .lock()
            .expect("release")
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("test releases the hook");
    }
}

/// Counts `on_schedule` calls.
struct Counting(Arc<std::sync::atomic::AtomicU32>);
impl maos_spirit_abi::lifecycle::Spirit for Counting {
    fn on_schedule(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        _payload: &maos_spirit_abi::lifecycle::SchedulePayload<'_>,
    ) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// P1 (review): a poll captures Spirits A, B and C, then waits inside A's
/// hook. B is unloaded and forgotten meanwhile. When A resumes the stale B
/// candidate must not recreate B's bucket or stamp, write a `schedule.fire`
/// row, or dispatch B's hook; C (captured after B) still fires.
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn a_candidate_captured_before_the_uninstall_never_fires_after_it() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    std::env::set_var("MAOS_SCHEDULE_FAST", "1");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x2011));
    let dispatcher = Arc::new(HookDispatcher::new(
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    ));
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let b_calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let b = scb_of(2, "b", Counting(Arc::clone(&b_calls)));
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    {
        let mut map = scbs.write().expect("scbs");
        map.insert(
            1,
            scb_of(
                1,
                "a",
                Blocking {
                    entered: std::sync::Mutex::new(Some(entered_tx)),
                    release: std::sync::Mutex::new(release_rx),
                },
            ),
        );
        map.insert(2, Arc::clone(&b));
        map.insert(3, scb(3, "c"));
    }
    let watchdog = Arc::new(ScheduleWatchdog::new(
        Arc::clone(&scbs),
        dispatcher,
        Arc::clone(&tl),
    ));
    let cancel = tokio_util::sync::CancellationToken::new();
    let handle = Arc::clone(&watchdog).spawn(cancel.child_token());
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the poll is inside A's hook, so B and C are already captured");
    // The door's order: unload (SCB leaves Running), then forget.
    b.state
        .store(ScbLifecycleState::Unloaded as u8, Ordering::Release);
    scbs.write().expect("scbs").remove(&2);
    let forgetting = {
        let watchdog = Arc::clone(&watchdog);
        tokio::spawn(async move { watchdog.forget_spirit("b", 2).await })
    };
    release_tx.send(()).expect("release A");
    forgetting.await.expect("forget task");
    let observed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while fires(&tl, 3) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    cancel.cancel();
    handle.await.expect("watchdog shuts down cleanly");
    observed.expect("C, captured after B in the same poll, fires");
    assert_eq!(fires(&tl, 2), 0, "a retired Spirit wrote a fire row");
    assert_eq!(
        b_calls.load(Ordering::SeqCst),
        0,
        "a retired Spirit's hook ran"
    );
    assert_eq!(watchdog.last_fire_ns(2, "daily"), 0, "a retired stamp");
    assert_eq!(
        watchdog.forget_spirit("b", 2).await,
        0,
        "a retired Spirit's bucket was recreated"
    );
}

/// Round 3 (N1): the fence covers admission and the stamp, never the hook. A
/// Spirit whose `on_schedule` has not returned must not stall the erasure path:
/// `forget_spirit` of another Spirit completes promptly, and a Spirit retired
/// while its own hook runs gets no stamp when the hook finally returns.
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn a_hanging_hook_never_stalls_forget_spirit_nor_stamps_a_retired_spirit() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    std::env::set_var("MAOS_SCHEDULE_FAST", "1");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x2011));
    let dispatcher = Arc::new(HookDispatcher::new(
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    ));
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let a = scb_of(
        1,
        "a",
        Blocking {
            entered: std::sync::Mutex::new(Some(entered_tx)),
            release: std::sync::Mutex::new(release_rx),
        },
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().expect("scbs").insert(1, Arc::clone(&a));
    scbs.write().expect("scbs").insert(2, scb(2, "b"));
    let watchdog = Arc::new(ScheduleWatchdog::new(Arc::clone(&scbs), dispatcher, tl));
    let cancel = tokio_util::sync::CancellationToken::new();
    let handle = Arc::clone(&watchdog).spawn(cancel.child_token());
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("A's hook is running and has not returned");
    let bound = std::time::Duration::from_secs(2);
    tokio::time::timeout(bound, watchdog.forget_spirit("b", 2))
        .await
        .expect("forget_spirit(B) queued behind A's hanging hook");
    a.state
        .store(ScbLifecycleState::Unloaded as u8, Ordering::Release);
    tokio::time::timeout(bound, watchdog.forget_spirit("a", 1))
        .await
        .expect("forget_spirit(A) queued behind A's own hanging hook");
    release_tx.send(()).expect("release A's hook");
    // The poll that was inside A's hook ends; let it run to completion.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    cancel.cancel();
    handle.await.expect("watchdog shuts down cleanly");
    assert_eq!(
        watchdog.last_fire_ns(1, "daily"),
        0,
        "a retired Spirit was stamped"
    );
}
