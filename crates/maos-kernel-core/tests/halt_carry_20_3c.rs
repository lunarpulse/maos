#![forbid(unsafe_code)]

//! 20-3c AC2 (FR53, D-20-3c-01 A, K1p): a hot swap CARRIES the predecessor's
//! pending halts — it neither drains them nor reads another Spirit's — and the
//! swap precheck judges the very same set the real swap gate judges.
//!
//! The end-to-end witness is `routed_halt_swap_20_3b`; these vectors pin the
//! rest of the contract:
//!
//! - (a) another Spirit's pending halt never changes the predecessor's verdict
//!   or precheck;
//! - (b) precheck verdict == swap verdict for 0, 1 and 2 carried halts;
//! - (c) an incompatible successor refuses the swap with the halt intact, and a
//!   swap compensated AFTER the I14 gate leaves the halt `PendingResolution`
//!   with its metadata;
//! - (d) the carried halt, resolved after a completed swap, writes exactly one
//!   approval row (the resolver path is unchanged);
//! - (e) the post-swap monitor watches the carried set: an operator resolving
//!   it inside the window is not a loss, a halt that left the registry is.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

use maos_director_surface::halt_ui::HaltFlow;
use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltContinuityError, HaltId, HaltJournal, HaltState, Resolution};
use maos_domain::hot_swap::{HotSwapError, PostSwapInvariantViolation};
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::halt::{
    invoke_halt, validate_swap_halt_continuity, HaltRegistry, KernelHaltResolver,
    OutputMarkerRegistry, PendingHaltMetadata, SwapVerdict,
};
use maos_kernel_core::hot_swap::{HotSwapCoordinator, HotSwapPrecheck, PrecheckOutcome};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::scheduler::hook_dispatch::HookDispatcher;
use maos_kernel_core::security::manifest::{HaltProtocolCompatibilitySection, LifecycleSection};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use maos_spirit_abi::ctx::Ctx;
use maos_spirit_abi::lifecycle::{Spirit, SwapInPayload};

const PID: u32 = 7;
const OTHER_PID: u32 = 9;
const SPIRIT_ID: &str = "halt-carry";
const BOOT_NONCE: u64 = 0x203c;

fn payload(halt_id: &str) -> EpistemicHaltPayload {
    EpistemicHaltPayload::new(
        halt_id.into(),
        "claim.ambiguity".into(),
        0.9,
        Some(0.8),
        "policy:clarify".into(),
        "frame:origin-203c".into(),
    )
    .unwrap()
}

fn seed(registry: &HaltRegistry, pid: u32, halt_id: &str) -> HaltId {
    let id = HaltId::new(halt_id).unwrap();
    registry
        .insert_pending_with_metadata(
            id.clone(),
            HaltState::PendingResolution,
            PendingHaltMetadata {
                spirit_pid: pid,
                spirit_id: format!("spirit-{pid}"),
                payload: payload(halt_id),
                fired_ns: 1_700_000_000_000 + u64::from(pid),
            },
        )
        .unwrap();
    id
}

// ---- (a) and (b): the gate and the precheck judge one set -----------------

#[test]
fn another_spirits_pending_halt_changes_neither_verdict_nor_precheck() {
    let registry = HaltRegistry::new();
    seed(&registry, OTHER_PID, "other-spirit-halt");
    // A legacy halt with no owner is nobody's to carry either.
    registry
        .insert_pending(
            HaltId::new("legacy-no-owner").unwrap(),
            HaltState::PendingResolution,
        )
        .unwrap();

    // The successor accepts only halt-protocol 2; the predecessor speaks 1. If
    // the other Spirit's halt counted, both would refuse.
    let swap = validate_swap_halt_continuity(&registry, PID, 1, Some(&[2]))
        .expect("the predecessor carries nothing, so nothing can be orphaned");
    assert_eq!(swap, SwapVerdict::SafeDrained { drained_count: 0 });
    let precheck = HotSwapPrecheck::check(&registry, PID, 1, &[2], 1, 1);
    assert_eq!(precheck.verdict, PrecheckOutcome::SafeDrained);
    assert_eq!(registry.pending_halt_ids().len(), 2, "nothing was drained");
}

#[test]
fn precheck_verdict_equals_swap_verdict_for_zero_one_and_two_carried_halts() {
    for carried in 0usize..=2 {
        for (successor, compatible) in [([1u32], true), ([2u32], false)] {
            let registry = HaltRegistry::new();
            seed(&registry, OTHER_PID, "noise-from-another-spirit");
            for n in 0..carried {
                seed(&registry, PID, &format!("carried-{n}"));
            }
            let ids = |registry: &HaltRegistry| -> std::collections::BTreeSet<String> {
                registry
                    .pending_halt_ids()
                    .iter()
                    .map(|id| id.as_str().to_owned())
                    .collect()
            };
            let before = ids(&registry);

            // Precheck first: it is pure. The swap gate must agree with it.
            let precheck = HotSwapPrecheck::check(&registry, PID, 1, &successor, 1, 1);
            let swap = validate_swap_halt_continuity(&registry, PID, 1, Some(&successor));

            let label = format!("{carried} carried, successor {successor:?}");
            match (carried, compatible) {
                (0, _) => {
                    assert_eq!(precheck.verdict, PrecheckOutcome::SafeDrained, "{label}");
                    assert_eq!(
                        swap,
                        Ok(SwapVerdict::SafeDrained { drained_count: 0 }),
                        "{label}"
                    );
                }
                (n, true) => {
                    assert_eq!(precheck.verdict, PrecheckOutcome::SafeMigrated, "{label}");
                    assert_eq!(precheck.migrated_count, Some(n), "{label}");
                    assert!(
                        matches!(swap, Ok(SwapVerdict::SafeMigrated { migrated_count, .. }) if migrated_count == n),
                        "{label}: {swap:?}"
                    );
                }
                (n, false) => {
                    assert_eq!(
                        precheck.verdict,
                        PrecheckOutcome::HaltContinuityViolation,
                        "{label}"
                    );
                    assert!(
                        matches!(
                            swap,
                            Err(HaltContinuityError::EHaltContinuityViolation { orphan_count, .. }) if orphan_count == n
                        ),
                        "{label}: {swap:?}"
                    );
                }
            }
            assert_eq!(
                ids(&registry),
                before,
                "{label}: neither judgement drains a halt"
            );
        }
    }
}

// ---- (c) and (d): the real coordinator -------------------------------------

/// Snapshots a fixed state blob; `swap_out_panics` refuses the swap AFTER the
/// I14 gate (step 4), which is what compensates the saga.
struct Probe {
    swap_out_panics: bool,
    restored: Arc<Mutex<Vec<u8>>>,
}

impl Spirit for Probe {
    fn on_swap_out(&self, _ctx: &mut Ctx) {
        assert!(!self.swap_out_panics, "swap-out refused after the I14 gate");
    }
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        b"replay:frame:origin-203c".to_vec()
    }
    fn on_swap_in<'a>(&self, _ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        *self.restored.lock().unwrap() = payload.predecessor_state.to_vec();
    }
}

struct Rig {
    coordinator: HotSwapCoordinator,
    registry: Arc<HaltRegistry>,
    tl: Arc<TransparencyLogAdapter>,
    resolver: Arc<KernelHaltResolver>,
    memory: Arc<MemoryManagerAdapter>,
    halt_id: HaltId,
    _tmp: tempfile::TempDir,
}

fn hooks() -> SpiritManifestBundle {
    SpiritManifestBundle {
        scheduling: Default::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["snapshot".into(), "on_swap_out".into(), "on_swap_in".into()],
            telemetry: None,
        },
        class: None,
        hot_swap: None,
        migrates_from: None,
        halt_protocol_compatibility: None,
        on_crash: None,
        on_revocation: None,
        supervision: None,
        schedules: Default::default(),
        gateways: Default::default(),
        budget: None,
        swap_invariants: None,
    }
}

/// A running predecessor with ONE already-active halt (`invoke_halt`, the
/// production path) and the real coordinator + resolver over the same registry.
fn rig(swap_out_panics: bool) -> Rig {
    cap_tokens::init_monotonic_base();
    let tmp = tempfile::tempdir().unwrap();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap());
    let registry = Arc::new(HaltRegistry::new());
    let metrics = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0; 32]),
        BOOT_NONCE,
        Arc::new(PolicyTable::new()),
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let scb = Arc::new(SpiritControlBlock::new(
        PID,
        SPIRIT_ID.into(),
        hooks(),
        make_spirit_obj(Probe {
            swap_out_panics,
            restored: Arc::default(),
        }),
        BOOT_NONCE,
    ));
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    let spirits = Arc::new(RwLock::new(BTreeMap::from([(PID, scb)])));
    let receipt = invoke_halt(
        &tl,
        &journal,
        &registry,
        payload("halt-fr53-original"),
        PID,
        SPIRIT_ID,
        BOOT_NONCE,
    )
    .unwrap();

    let private = Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4 * 1024));
    let db_path = tmp.path().join("audit.db");
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        Arc::new(SharedMemoryStore::open(&db_path).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap()),
        Arc::clone(&tl),
    ));
    let resolver = Arc::new(KernelHaltResolver::new(
        Arc::clone(&registry),
        Arc::clone(&tl),
        Arc::new(OutputMarkerRegistry::new()),
        Arc::new(Mailbox::new(Arc::clone(&metrics))),
        BOOT_NONCE,
        Arc::clone(&memory),
        Arc::new(WorkingMemoryOrchestrator::new(
            Arc::clone(&capability),
            Arc::clone(&registry),
        )),
    ));
    let coordinator = HotSwapCoordinator::new(
        spirits,
        journal,
        Arc::clone(&tl),
        Arc::clone(&registry),
        capability,
        iac,
        Arc::new(HookDispatcher::new(Arc::clone(&tl), Arc::clone(&metrics))),
        metrics,
        tmp.path().join("archives"),
    );
    Rig {
        coordinator,
        registry,
        tl,
        resolver,
        memory,
        halt_id: receipt.halt_id,
        _tmp: tmp,
    }
}

fn successor(version: Option<u32>) -> SpiritManifestBundle {
    SpiritManifestBundle {
        halt_protocol_compatibility: version
            .map(|version| HaltProtocolCompatibilitySection { version }),
        ..hooks()
    }
}

fn probe() -> Arc<dyn maos_kernel_core::scheduler::control_block::AnySpiritObj> {
    make_spirit_obj(Probe {
        swap_out_panics: false,
        restored: Arc::default(),
    })
}

fn metadata(rig: &Rig) -> PendingHaltMetadata {
    rig.registry
        .lookup_pending_metadata(&rig.halt_id)
        .expect("the active halt still has its identity and replay metadata")
}

#[tokio::test]
async fn an_incompatible_successor_refuses_the_swap_with_the_halt_intact() {
    let rig = rig(false);
    let before = metadata(&rig);

    let error = rig
        .coordinator
        .initiate_swap(SPIRIT_ID, &successor(Some(2)), probe())
        .await
        .expect_err("a successor that cannot carry the halt must refuse the swap");
    assert!(
        matches!(
            error,
            HotSwapError::HaltContinuityViolation(HaltContinuityError::EHaltContinuityViolation {
                orphan_count: 1,
                ..
            })
        ),
        "{error:?}"
    );
    assert_eq!(
        rig.registry.lookup_state(&rig.halt_id),
        Some(HaltState::PendingResolution)
    );
    assert_eq!(metadata(&rig), before, "identity, owner, fire time, replay");
}

#[tokio::test]
async fn a_swap_compensated_after_the_i14_gate_leaves_the_halt_pending_with_its_metadata() {
    let _window = monitor_window(false).await;
    let rig = rig(true);
    let before = metadata(&rig);

    let error = rig
        .coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect_err("the predecessor's swap-out panics, after the gate has passed");
    assert!(
        matches!(error, HotSwapError::SwapOutFailed { .. }),
        "{error:?}"
    );
    assert_eq!(
        rig.registry.lookup_state(&rig.halt_id),
        Some(HaltState::PendingResolution),
        "a compensated swap restores the predecessor WITH its halt"
    );
    assert_eq!(metadata(&rig), before);
    assert_eq!(rig.registry.pending_halt_ids(), vec![rig.halt_id.clone()]);
}

#[tokio::test]
async fn a_halt_resolved_after_the_swap_writes_exactly_one_approval_row() {
    let _window = monitor_window(false).await;
    let rig = rig(false);
    let before = metadata(&rig);
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("a same-major swap carries the halt");
    assert_eq!(
        rig.registry.lookup_state(&rig.halt_id),
        Some(HaltState::PendingResolution),
        "the swap neither resolved nor ended the halt"
    );
    assert_eq!(metadata(&rig), before);
    assert!(
        rig.tl.query_approvals(None).unwrap().is_empty(),
        "the swap itself writes no approval row"
    );

    let text = "the operator supplied the missing cite";
    let journal: Arc<dyn HaltJournal> = Arc::clone(&rig.tl) as Arc<dyn HaltJournal>;
    HaltFlow::new(
        Arc::clone(&rig.resolver),
        Arc::new(NotificationDispatcher::new()),
        journal,
    )
    .submit_resolution(
        rig.halt_id.clone(),
        Resolution::ProvidedContext { text: text.into() },
        SPIRIT_ID,
    )
    .expect("the carried halt resolves through the unchanged resolver path");

    assert_eq!(
        rig.registry.lookup_state(&rig.halt_id),
        Some(HaltState::Resumed)
    );
    let approvals = rig.tl.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 1, "exactly one approval row");
    assert_eq!(approvals[0].target, SPIRIT_ID);
    assert!(approvals[0]
        .reasoning
        .as_deref()
        .unwrap()
        .contains(rig.halt_id.as_str()));
    match rig
        .memory
        .read(
            PID,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            &format!("halt_context::{}", rig.halt_id.as_str()),
        )
        .unwrap()
    {
        Some(MemoryValue::Text(stored)) => assert_eq!(stored, text),
        other => panic!("the supplied context must reach the Spirit's private tier: {other:?}"),
    }
}

// ---- (e): the post-swap monitor watches the CARRIED set ---------------------

/// `PostSwapMonitor::new` reads `MAOS_AUTO_REVERT_FAST` when a swap builds its
/// monitor, and the environment is process-global. Every vector that completes
/// a swap (so spawns a monitor) holds this lock for its whole body and sets the
/// window explicitly, so a FAST window can never leak into a concurrent test.
static MONITOR_WINDOW: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn monitor_window(fast: bool) -> tokio::sync::MutexGuard<'static, ()> {
    let guard = MONITOR_WINDOW.lock().await;
    if fast {
        std::env::set_var("MAOS_AUTO_REVERT_FAST", "1");
    } else {
        std::env::remove_var("MAOS_AUTO_REVERT_FAST");
    }
    guard
}

/// Count `HotSwapAborted` rows (the auto-revert receipt), waiting out the
/// 300 ms `MAOS_AUTO_REVERT_FAST` window plus its 50 ms ticks.
async fn auto_reverts_within(rig: &Rig, wait: std::time::Duration) -> usize {
    let deadline = std::time::Instant::now() + wait;
    loop {
        let rows = rig
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::HotSwapAborted),
                ..Default::default()
            })
            .unwrap()
            .len();
        if rows > 0 || std::time::Instant::now() >= deadline {
            return rows;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn resolving_the_carried_halt_inside_the_monitor_window_is_not_a_loss() {
    let _window = monitor_window(true).await;
    let rig = rig(false);
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("a same-major swap carries the halt");
    rig.registry
        .resolve(&rig.halt_id, HaltState::Terminated)
        .expect("the operator resolves the carried halt inside the window");
    assert_eq!(
        auto_reverts_within(&rig, std::time::Duration::from_millis(900)).await,
        0,
        "a resolved halt is terminal, not lost: the valid swap must not be auto-reverted"
    );
}

#[tokio::test]
async fn a_carried_halt_that_leaves_the_registry_inside_the_window_is_still_a_loss() {
    let _window = monitor_window(true).await;
    let rig = rig(false);
    let predecessor = rig
        .coordinator
        .spirits_map()
        .read()
        .unwrap()
        .get(&PID)
        .unwrap()
        .runtime_snapshot()
        .spirit_obj;
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("a same-major swap carries the halt");
    assert_eq!(rig.registry.drain_for_spirit(PID).len(), 1);
    assert_eq!(
        auto_reverts_within(&rig, std::time::Duration::from_secs(5)).await,
        1,
        "a halt that vanished without a resolution still trips HaltSetLoss"
    );
    let live = rig
        .coordinator
        .spirits_map()
        .read()
        .unwrap()
        .get(&PID)
        .unwrap()
        .runtime_snapshot()
        .spirit_obj;
    assert!(
        std::ptr::addr_eq(Arc::as_ptr(&live), Arc::as_ptr(&predecessor)),
        "auto-revert restores the exact predecessor runtime object"
    );
}

/// A clean window releases the predecessor runtime it held for rollback exactly
/// when the (paused) Tokio clock reaches the window, and nothing earlier.
#[tokio::test(start_paused = true)]
async fn a_clean_window_releases_the_predecessor_at_its_deadline() {
    let _window = monitor_window(false).await;
    let rig = rig(false);
    let predecessor = {
        let map = rig.coordinator.spirits_map().read().unwrap();
        Arc::downgrade(&map.get(&PID).unwrap().runtime_snapshot().spirit_obj)
    };
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("a same-major swap carries the halt");

    let window = std::time::Duration::from_secs(30);
    let epsilon = std::time::Duration::from_nanos(1);
    tokio::time::advance(window - epsilon).await;
    tokio::task::yield_now().await;
    assert!(
        predecessor.upgrade().is_some(),
        "the rollback window still holds the predecessor one tick before it closes"
    );

    tokio::time::advance(epsilon).await;
    tokio::task::yield_now().await;
    assert!(
        predecessor.upgrade().is_none(),
        "the window releases the predecessor once it closes"
    );
    let violation = PostSwapInvariantViolation::halt_set_loss(vec![rig.halt_id.as_str().into()])
        .expect("nonempty carried halt id");
    assert!(matches!(
        rig.coordinator.auto_revert(PID, violation).await,
        Err(HotSwapError::NotLoaded { .. })
    ));
    assert!(
        rig.coordinator
            .spirits_map()
            .read()
            .unwrap()
            .contains_key(&PID),
        "a clean window leaves the successor registered"
    );
}

// ---- publication identity and revert custody (Story 21-9) -------------------

fn lost_halt() -> PostSwapInvariantViolation {
    PostSwapInvariantViolation::halt_set_loss(vec!["halt-fr53-original".into()])
        .expect("nonempty carried halt id")
}

/// Holds ONLY its first `on_swap_out` (the auto-revert's) until released, so a test
/// decides what happens while a revert waits in the successor hook.
struct Gate {
    calls: std::sync::atomic::AtomicUsize,
    entered: Mutex<std::sync::mpsc::Sender<()>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

impl Spirit for Gate {
    fn on_swap_out(&self, _ctx: &mut Ctx) {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.entered.lock().unwrap().send(()).unwrap();
            let _ = self.release.lock().unwrap().recv();
        }
    }
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        b"replay:frame:origin-203c".to_vec()
    }
}

fn gate() -> (
    Arc<dyn maos_kernel_core::scheduler::control_block::AnySpiritObj>,
    std::sync::mpsc::Receiver<()>,
    std::sync::mpsc::Sender<()>,
) {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let obj = make_spirit_obj(Gate {
        calls: Default::default(),
        entered: Mutex::new(entered_tx),
        release: Mutex::new(release_rx),
    });
    (obj, entered_rx, release_tx)
}

fn live_runtime(rig: &Rig) -> maos_kernel_core::scheduler::control_block::ScbRuntimeSnapshot {
    rig.coordinator
        .spirits_map()
        .read()
        .unwrap()
        .get(&PID)
        .unwrap()
        .runtime_snapshot()
}

/// Start an `auto_revert` and wait until it sits inside the gated successor hook.
async fn revert_waiting_in_swap_out(
    rig: &Arc<Rig>,
    entered: std::sync::mpsc::Receiver<()>,
) -> tokio::task::JoinHandle<Result<(), HotSwapError>> {
    let rig = Arc::clone(rig);
    let reverting =
        tokio::spawn(async move { rig.coordinator.auto_revert(PID, lost_halt()).await });
    tokio::task::spawn_blocking(move || entered.recv())
        .await
        .unwrap()
        .expect("the revert reached the successor's on_swap_out");
    reverting
}

#[tokio::test]
async fn a_revert_waiting_in_swap_out_cannot_overwrite_a_newer_publication_of_the_same_object() {
    let _window = monitor_window(false).await;
    let rig = Arc::new(rig(false));
    let (obj, entered, release) = gate();
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), Arc::clone(&obj))
        .await
        .expect("first swap");
    let reverting = revert_waiting_in_swap_out(&rig, entered).await;

    // The caller reuses the same object for a second swap with a different manifest.
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(Some(1)), Arc::clone(&obj))
        .await
        .expect("second swap of the same object");
    release.send(()).unwrap();

    assert!(matches!(
        reverting.await.unwrap(),
        Err(HotSwapError::NotLoaded { .. })
    ));
    let live = live_runtime(&rig);
    assert!(std::ptr::addr_eq(
        Arc::as_ptr(&live.spirit_obj),
        Arc::as_ptr(&obj)
    ));
    assert!(
        live.manifest.halt_protocol_compatibility.is_some(),
        "the newer publication's manifest survives the stale revert"
    );
    assert_eq!(
        auto_reverts_within(&rig, std::time::Duration::ZERO).await,
        0
    );
}

#[tokio::test]
async fn cancelling_the_revert_future_inside_swap_out_keeps_the_rollback_claimable() {
    let _window = monitor_window(false).await;
    let rig = Arc::new(rig(false));
    let predecessor = live_runtime(&rig).spirit_obj;
    let (obj, entered, release) = gate();
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), Arc::clone(&obj))
        .await
        .expect("swap");
    let reverting = revert_waiting_in_swap_out(&rig, entered).await;

    reverting.abort();
    assert!(reverting.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();

    rig.coordinator
        .auto_revert(PID, lost_halt())
        .await
        .expect("a retry still finds the rollback record");
    assert!(std::ptr::addr_eq(
        Arc::as_ptr(&live_runtime(&rig).spirit_obj),
        Arc::as_ptr(&predecessor)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_revert_claimed_the_instant_the_window_opens_is_journaled_after_the_swap() {
    let _window = monitor_window(false).await;
    let rig = Arc::new(rig(false));
    let hammer = {
        let rig = Arc::clone(&rig);
        tokio::spawn(async move {
            while rig.coordinator.auto_revert(PID, lost_halt()).await.is_err() {
                tokio::task::yield_now().await;
            }
        })
    };
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("swap");
    tokio::time::timeout(std::time::Duration::from_secs(10), hammer)
        .await
        .expect("the revert lands once the window opens")
        .unwrap();

    let journal = std::fs::read_to_string(rig._tmp.path().join("journal.ndjson")).unwrap();
    let at = |event: &str| journal.lines().position(|line| line.contains(event));
    let swapped = at("\"HotSwap\"").expect("the swap is journaled");
    let reverted = at("\"HotSwapAutoReverted\"").expect("the revert is journaled");
    assert!(swapped < reverted, "HotSwap precedes HotSwapAutoReverted");
}

#[tokio::test(start_paused = true)]
async fn a_monitor_first_polled_with_little_window_left_still_checks_before_expiry() {
    let _window = monitor_window(false).await;
    let rig = rig(false);
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), probe())
        .await
        .expect("a same-major swap carries the halt");
    assert_eq!(rig.registry.drain_for_spirit(PID).len(), 1);

    // The monitor task has not been polled yet; its first poll lands 0.5s before the
    // window closes, less than one cadence tick.
    tokio::time::advance(std::time::Duration::from_millis(29_500)).await;
    assert_eq!(
        auto_reverts_within(&rig, std::time::Duration::from_secs(2)).await,
        1,
        "the loss is caught by the first check, not skipped to expiry"
    );
}

/// Cancel a public revert that sits in the gated successor hook, then release the hook.
async fn cancel_revert_in_swap_out(rig: &Arc<Rig>) {
    let (obj, entered, release) = gate();
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), obj)
        .await
        .expect("swap");
    let reverting = revert_waiting_in_swap_out(rig, entered).await;
    reverting.abort();
    assert!(reverting.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
}

#[tokio::test(start_paused = true)]
async fn cancelling_a_public_revert_leaves_the_windows_monitor_checking() {
    let _window = monitor_window(false).await;
    let rig = Arc::new(rig(false));
    cancel_revert_in_swap_out(&rig).await;

    // The re-armed window kept its own monitor: a carried halt that now leaves the
    // registry is caught by that monitor's check, with no further public call.
    assert_eq!(rig.registry.drain_for_spirit(PID).len(), 1);
    assert_eq!(
        auto_reverts_within(&rig, std::time::Duration::from_secs(5)).await,
        1
    );
}

#[tokio::test(start_paused = true)]
async fn a_window_re_armed_after_a_cancelled_revert_still_expires_at_its_deadline() {
    let _window = monitor_window(false).await;
    let rig = Arc::new(rig(false));
    let predecessor = Arc::downgrade(&live_runtime(&rig).spirit_obj);
    cancel_revert_in_swap_out(&rig).await;

    tokio::time::advance(std::time::Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    assert!(
        predecessor.upgrade().is_none(),
        "the window's own expiry still releases the rollback"
    );
    assert!(matches!(
        rig.coordinator.auto_revert(PID, lost_halt()).await,
        Err(HotSwapError::NotLoaded { .. })
    ));
}

#[tokio::test(start_paused = true)]
async fn shutdown_cancels_a_monitor_revert_waiting_in_the_successor_hook() {
    let _window = monitor_window(false).await;
    let rig = rig(false);
    let (obj, entered, release) = gate();
    rig.coordinator
        .initiate_swap(SPIRIT_ID, &successor(None), Arc::clone(&obj))
        .await
        .expect("swap");
    let spirits = Arc::clone(rig.coordinator.spirits_map());

    // The carried halt left the registry: the window's own monitor starts the revert
    // at its check and waits inside the gated successor hook.
    assert_eq!(rig.registry.drain_for_spirit(PID).len(), 1);
    tokio::task::spawn_blocking(move || entered.recv())
        .await
        .unwrap()
        .expect("the monitor's revert reached the successor's on_swap_out");

    let Rig {
        coordinator,
        tl,
        _tmp,
        ..
    } = rig;
    drop(coordinator);
    release.send(()).unwrap();
    // Grace for the released hook to finish, so a surviving monitor would have acted.
    tokio::task::spawn_blocking(|| std::thread::sleep(std::time::Duration::from_millis(250)))
        .await
        .unwrap();
    tokio::task::yield_now().await;

    let reverts = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::HotSwapAborted),
            ..Default::default()
        })
        .unwrap()
        .len();
    assert_eq!(reverts, 0, "no revert may complete after shutdown");
    let live = spirits
        .read()
        .unwrap()
        .get(&PID)
        .unwrap()
        .runtime_snapshot();
    assert!(
        std::ptr::addr_eq(Arc::as_ptr(&live.spirit_obj), Arc::as_ptr(&obj)),
        "the predecessor was not republished after shutdown"
    );
}
