#![forbid(unsafe_code)]

//! Story 16-6 — a Spirit that was loaded and never started must have an exit.
//!
//! At HEAD `is_transition_allowed` (`scheduler/control_block.rs`) carries
//! `(Loaded, Running)` but no `(Loaded, Unloaded)`, so
//! `SpiritSchedulerAdapter::unload` returns `InvalidStateTransition` from the
//! `?` on `scb.transition(...)` and none of the five cleanup steps after it
//! run: no NFR-Rel-11 receipt, no token revocation, no halt drain, and the SCB
//! is never removed from the `spirits` map — which also burns the `spirit_id`,
//! because `resolve_pid` is a linear scan over the surviving entries.
//!
//! These are the AC2 falsifiers. They are RED at HEAD by construction and the
//! story's obligation (b) requires them to be seen red before the kernel arm
//! lands.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_domain::invariants::i1::IntentClass;
use maos_domain::ports::task::TaskAssignmentRecord;
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::scheduler::{
    control_block::SpiritManifestBundle, scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

/// A Spirit whose `on_unload` panics on demand — the only way to reach
/// `HookOutcome::Panicked` on the unload path, since no first-party
/// compiled-in class overrides `on_unload` at all.
#[derive(Default)]
struct HookSpirit {
    unload_calls: Arc<AtomicU32>,
    panic_on_unload: bool,
}

impl maos_spirit_abi::lifecycle::Spirit for HookSpirit {
    fn on_unload(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.unload_calls.fetch_add(1, Ordering::Relaxed);
        if self.panic_on_unload {
            panic!("16-6 falsifier: on_unload panicked");
        }
    }
}

struct Harness {
    scheduler: Arc<SpiritSchedulerAdapter>,
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
    halt_registry: Arc<HaltRegistry>,
    telemetry: Arc<IacRtMetrics>,
    scratch: tempfile::TempDir,
}

fn harness() -> Harness {
    let scratch = tempfile::TempDir::new().expect("scratch dir");
    let db_path = scratch.path().join("audit.db");
    let memory_root = scratch.path().join("memory");

    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEEF));
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
            0xBEEF,
            Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            memory_root,
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap(),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(HaltRegistry::new());
    let telemetry = Arc::new(IacRtMetrics::new());

    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        Arc::clone(&memory),
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&telemetry),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    Harness {
        scheduler,
        tl,
        capability,
        iac,
        halt_registry,
        telemetry,
        scratch,
    }
}

impl Harness {
    /// Exercise the unload panic path with the production crash-handler wiring:
    /// deleting the hook-identity guard must not silently be covered by `None`.
    fn attach_crash_detector(&mut self) {
        let journal_path = self.scratch.path().join("crash-detector.ndjson");
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .expect("crash-detector journal"),
        );
        let crash_detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            self.scheduler.scbs(),
            Arc::clone(&self.tl),
            Arc::clone(&self.halt_registry),
            Arc::clone(&self.capability),
            Arc::clone(&self.iac),
            Arc::clone(&self.telemetry),
            journal,
        ));
        Arc::get_mut(&mut self.scheduler)
            .expect("harness owns the scheduler")
            .set_crash_detector(crash_detector);
    }
}

/// Every `term-{spirit_id}-{pid}-{ns}` receipt marker the TL carries for a pid.
/// Read from the log rather than asserting `terminate_spirit` was called —
/// obligation (e).
fn termination_receipts(
    tl: &TransparencyLogAdapter,
    spirit_pid: u32,
    spirit_id: &str,
) -> Vec<String> {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(spirit_pid),
        kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt),
        order_by_insertion: true,
        ..Default::default()
    })
    .expect("query transparency log")
    .into_iter()
    .filter_map(|entry| {
        let body = String::from_utf8_lossy(&entry.payload_redacted).into_owned();
        let needle = format!("term-{spirit_id}-{spirit_pid}-");
        body.contains(&needle).then_some(body)
    })
    .collect()
}

fn scb_present(scheduler: &SpiritSchedulerAdapter, pid: u32) -> bool {
    scheduler
        .scbs()
        .read()
        .expect("spirits lock")
        .contains_key(&pid)
}

/// AC2(a)(b)(c) — the whole reason this story holds a kernel grant.
///
/// A Spirit loaded over the door and never started must unload: exit cleanly,
/// leave the map, emit its NFR-Rel-11 receipt, and free its name for a
/// subsequent load.
#[tokio::test]
async fn loaded_but_never_started_spirit_unloads_with_a_receipt() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let h = harness();
    let unload_calls = Arc::new(AtomicU32::new(0));

    let pid = h
        .scheduler
        .load(
            "door-loaded",
            SpiritManifestBundle::default(),
            HookSpirit {
                unload_calls: Arc::clone(&unload_calls),
                panic_on_unload: false,
            },
            0xBEEF,
        )
        .await
        .expect("load must succeed");

    // The state this story makes durable and operator-visible for the first time.
    assert!(
        scb_present(&h.scheduler, pid),
        "a freshly loaded Spirit must be in the scheduler map"
    );

    h.scheduler
        .unload(pid)
        .await
        .expect("a loaded-but-never-started Spirit must be unloadable");

    assert!(
        !scb_present(&h.scheduler, pid),
        "unload must remove the SCB from the map (scheduler_loop.rs spirits.remove)"
    );
    assert_eq!(
        h.scheduler.resolve_pid("door-loaded"),
        None,
        "the spirit_id must no longer resolve once the SCB is gone"
    );
    assert_eq!(
        termination_receipts(&h.tl, pid, "door-loaded").len(),
        1,
        "unload must emit exactly one NFR-Rel-11 termination receipt"
    );
    assert_eq!(
        unload_calls.load(Ordering::Relaxed),
        1,
        "the loaded-but-never-started Spirit must receive on_unload once"
    );

    // (c) the name is not burned: the same id loads again.
    let second = h
        .scheduler
        .load(
            "door-loaded",
            SpiritManifestBundle::default(),
            HookSpirit::default(),
            0xBEEF,
        )
        .await
        .expect("re-loading the same id after a clean unload must succeed");
    assert_ne!(second, pid, "a re-load allocates a fresh pid");
}

/// AC2(g) / obligation (aa) — the receipt gap the arm makes reachable.
///
/// `unload` flips the state to `Unloaded` before the fallible `on_unload`
/// dispatch. If the hook fails, cleanup must still run: the operator's Spirit
/// is gone either way, and a retry hits the idempotent early return and can
/// never write the receipt that was skipped.
#[tokio::test]
async fn unload_cleanup_runs_even_when_the_on_unload_hook_panics() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let mut h = harness();
    h.attach_crash_detector();
    let unload_calls = Arc::new(AtomicU32::new(0));

    let pid = h
        .scheduler
        .load(
            "panicking-unload",
            SpiritManifestBundle::default(),
            HookSpirit {
                unload_calls: Arc::clone(&unload_calls),
                panic_on_unload: true,
            },
            0xBEEF,
        )
        .await
        .expect("load must succeed");

    let outcome = h.scheduler.unload(pid).await;
    assert!(
        outcome.is_err(),
        "a panicking on_unload is still reported to the caller"
    );
    assert_eq!(
        unload_calls.load(Ordering::Relaxed),
        1,
        "the hook must actually have been dispatched"
    );

    assert!(
        !scb_present(&h.scheduler, pid),
        "a failed on_unload must NOT strand the SCB in the map"
    );
    assert_eq!(
        termination_receipts(&h.tl, pid, "panicking-unload").len(),
        1,
        "a failed on_unload must still produce exactly one NFR-Rel-11 receipt \
         — and exactly one, not a second from a parallel crash teardown"
    );

    // The retry the operator actually performs: it must not be a false success
    // that papers over a missing receipt.
    h.scheduler
        .unload(pid)
        .await
        .expect("unload is idempotent once the Spirit is gone");
    assert_eq!(
        termination_receipts(&h.tl, pid, "panicking-unload").len(),
        1,
        "the idempotent retry must not mint a second receipt"
    );
}

/// Story 16-6 (AC2(g)(ii)) — unload-hook failure emits the audit-classifiable
/// FR50 orphan frame after the kernel has released its SCB.
#[tokio::test]
async fn panicking_unload_orphans_in_flight_tasks_with_the_audit_payload_shape() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let h = harness();

    let pid = h
        .scheduler
        .load(
            "panicking-unload-with-task",
            SpiritManifestBundle::default(),
            HookSpirit {
                unload_calls: Arc::new(AtomicU32::new(0)),
                panic_on_unload: true,
            },
            0xBEEF,
        )
        .await
        .expect("load must succeed");
    let scb = h
        .scheduler
        .scbs()
        .read()
        .expect("spirits lock")
        .get(&pid)
        .cloned()
        .expect("loaded SCB");
    scb.task_assignments_in_flight
        .lock()
        .expect("task ledger lock")
        .push(TaskAssignmentRecord {
            task_id: "unload-orphan-task".into(),
            capability_token: None,
            ttl_deadline_ns: u64::MAX,
            intent_class: IntentClass::Standard,
            originator_spirit_id: "originating-spirit".into(),
        });

    assert!(
        h.scheduler.unload(pid).await.is_err(),
        "the panicking hook remains observable to the caller"
    );

    let frames =
        h.tl.query_frames(FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete),
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .expect("query transparency log");
    let orphans: Vec<_> = frames
        .iter()
        .filter(|frame| frame.intent == "task.orphaned")
        .collect();
    assert_eq!(orphans.len(), 1, "exactly one in-flight task is orphaned");
    let payload: serde_json::Value =
        serde_json::from_slice(&orphans[0].payload_redacted).expect("orphan payload is JSON");
    let payload = payload.as_object().expect("orphan payload is an object");
    let keys: BTreeSet<_> = payload.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        BTreeSet::from([
            "task_id",
            "originator_spirit_id",
            "exit_signal",
            "exit_code",
            "stderr_tail",
            "cause",
            "in_flight_tokens",
            "disposition",
        ]),
        "the exact key set is the FR4 classifier's scheduler_loop shape"
    );
    assert!(payload["task_id"].is_string());
    assert!(payload["originator_spirit_id"].is_string());
    assert!(payload["exit_signal"].is_null());
    assert!(payload["exit_code"].is_null());
    assert!(payload["stderr_tail"].is_string());
    assert!(payload["cause"].is_string());
    assert!(payload["in_flight_tokens"]
        .as_array()
        .is_some_and(|tokens| tokens.is_empty()));
    assert!(payload["disposition"].is_string());
}
