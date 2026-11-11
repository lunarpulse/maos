#![forbid(unsafe_code)]

//! Story 20-11 O4 (D-20-11-11 A) — the single receipt writer fences the pid.
//!
//! Every terminator (`unload`, crash, revocation) drains through
//! `terminate_spirit`. Before O4 an `Unloaded` SCB still accepted a durable
//! halt that `unload`'s second, silent drain discarded without a terminal
//! receipt. Now a halt in flight when the close happens is receipted by that
//! drain, and a halt arriving after it is refused with a typed
//! `InvokeHaltError::SpiritUnloading(pid)` before anything is written.
//!
//! The real scheduler, `invoke_halt`, Transparency Log and Journal are used;
//! the existing redaction-policy seam pauses a chosen row so the race is
//! deterministic, not timed.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltReceipt, InvokeHaltError};
use maos_iac::adapter::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::scheduler::{
    control_block::{ScbLifecycleState, SpiritManifestBundle},
    scheduler_loop::SpiritSchedulerAdapter,
};

const BOOT: u64 = 0x2011;

/// Pauses the first row whose bytes contain `needle` until released.
#[derive(Debug)]
struct Pause {
    needle: &'static str,
    fired: AtomicBool,
    reached: mpsc::SyncSender<()>,
    resume: Mutex<mpsc::Receiver<()>>,
    default: CorpusBackedRedactionPolicy,
}

impl RedactionPolicy for Pause {
    fn redact<'a>(&self, bytes: &'a [u8]) -> Cow<'a, [u8]> {
        if String::from_utf8_lossy(bytes).contains(self.needle)
            && !self.fired.swap(true, Ordering::SeqCst)
        {
            self.reached.send(()).expect("observer receives the pause");
            self.resume
                .lock()
                .expect("pause mutex")
                .recv_timeout(Duration::from_secs(10))
                .expect("the seam is resumed");
        }
        self.default.redact(bytes)
    }
}

#[derive(Default)]
struct Spirit;
impl maos_spirit_abi::lifecycle::Spirit for Spirit {}

struct World {
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    journal_path: std::path::PathBuf,
    registry: Arc<HaltRegistry>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    pid: u32,
    _dir: tempfile::TempDir,
}

/// A real scheduler over a log whose redaction seam pauses `needle`.
async fn world(needle: &'static str) -> (World, mpsc::Receiver<()>, mpsc::SyncSender<()>) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let dir = tempfile::tempdir().expect("scratch");
    let (reached_tx, reached_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_policy(
            &dir.path().join("tl.db"),
            BOOT,
            Box::new(Pause {
                needle,
                fired: AtomicBool::new(false),
                reached: reached_tx,
                resume: Mutex::new(resume_rx),
                default: CorpusBackedRedactionPolicy::new(),
            }),
        )
        .expect("transparency log"),
    );
    let journal_path = dir.path().join("journal.ndjson");
    let journal = Arc::new(JournalAdapter::open(&journal_path).expect("journal"));
    let registry = Arc::new(HaltRegistry::new());
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0; 32]),
            BOOT,
            Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let db = dir.path().join("memory.db");
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            dir.path().join("memory"),
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db).expect("shared")),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db)
                .expect("principal index"),
        ),
        Arc::clone(&tl),
    ));
    let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        capability,
        memory,
        iac,
        Arc::clone(&registry),
        telemetry,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let pid = scheduler
        .load("race-20-11", SpiritManifestBundle::default(), Spirit, BOOT)
        .await
        .expect("load");
    let world = World {
        tl,
        journal,
        journal_path,
        registry,
        scheduler,
        pid,
        _dir: dir,
    };
    (world, reached_rx, resume_tx)
}

fn halt(id: &str) -> EpistemicHaltPayload {
    EpistemicHaltPayload::new(
        id.into(),
        "ambiguity".into(),
        1.0,
        None,
        "policy-20-11".into(),
        "derived-20-11".into(),
    )
    .expect("valid halt")
}

fn raise(world: &World, id: &str) -> Result<HaltReceipt, InvokeHaltError> {
    invoke_halt(
        &world.tl,
        &world.journal,
        &world.registry,
        halt(id),
        world.pid,
        "race-20-11",
        BOOT,
    )
}

/// Every terminal receipt row's halt id, in insertion order.
fn receipt_ids(world: &World, kind: &str) -> Vec<String> {
    world
        .tl
        .query_frames(FrameFilter {
            spirit_pid: Some(world.pid),
            kind: Some(FrameKind::EpistemicHalt),
            order_by_insertion: true,
            ..Default::default()
        })
        .expect("durable halt rows")
        .iter()
        .filter(|row| row.intent == kind && !row.payload_redacted.is_empty())
        .filter_map(|row| serde_json::from_slice::<HaltReceipt>(&row.payload_redacted).ok())
        .map(|receipt| receipt.halt_id.as_str().to_owned())
        .collect()
}

fn invocation_rows(world: &World) -> usize {
    world
        .tl
        .query_frames(FrameFilter {
            spirit_pid: Some(world.pid),
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .expect("durable halt rows")
        .iter()
        .filter(|row| row.intent == "ambiguity")
        .count()
}

fn journal_halts(world: &World) -> usize {
    std::fs::read_to_string(&world.journal_path)
        .expect("journal text")
        .lines()
        .filter(|line| line.contains("\"Halt\""))
        .count()
}

fn is_refused(result: Result<HaltReceipt, InvokeHaltError>, pid: u32) -> bool {
    matches!(result, Err(InvokeHaltError::SpiritUnloading(refused)) if refused == pid)
}

/// The T0 witness, re-stated for the repair: a halt arriving after the first
/// drain (the `Unloaded` SCB, the planned-unload receipt being serialized) is
/// refused with the typed error, writes no durable row and no journal entry,
/// and leaves the registry empty and only the unload's own `term-…` receipt.
#[tokio::test]
async fn o4_late_halt_during_unload_is_refused_typed_and_never_lost() {
    let (world, reached, resume) = world("planned_unload").await;
    let world = Arc::new(world);
    let observer = {
        let world = Arc::clone(&world);
        std::thread::spawn(move || {
            reached
                .recv_timeout(Duration::from_secs(10))
                .expect("unload reaches its receipt");
            assert_eq!(
                world.scheduler.scbs().read().expect("scbs")[&world.pid].current_state(),
                ScbLifecycleState::Unloaded
            );
            let refused = is_refused(raise(&world, "late-20-11"), world.pid);
            resume.send(()).expect("resume unload");
            refused
        })
    };
    world.scheduler.unload(world.pid).await.expect("unload");
    assert!(
        observer.join().expect("observer"),
        "a halt after the close must be refused with SpiritUnloading(pid)"
    );
    assert_eq!(invocation_rows(&world), 0, "a refused halt writes no row");
    assert_eq!(journal_halts(&world), 0, "a refused halt journals nothing");
    assert!(world
        .registry
        .halt_metadata_for_spirit(world.pid, 0)
        .is_empty());
    assert_eq!(world.scheduler.resolve_pid("race-20-11"), None);
    let receipts = receipt_ids(&world, "planned_unload");
    assert!(
        receipts.len() == 1 && receipts[0].starts_with("term-race-20-11-"),
        "only the unload's own marker receipt exists: {receipts:?}"
    );
}

/// S2: a halt whose own TL row is in flight when `unload` runs is committed
/// first (the close waits for the invocation), so the drain receipts it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn o4_inflight_halt_is_receipted_and_the_next_is_refused() {
    let (world, reached, resume) = world("inflight-20-11").await;
    let world = Arc::new(world);
    let invoker = {
        let world = Arc::clone(&world);
        std::thread::spawn(move || raise(&world, "inflight-20-11"))
    };
    reached
        .recv_timeout(Duration::from_secs(10))
        .expect("the invocation is paused inside its own TL row");
    let unload = {
        let world = Arc::clone(&world);
        tokio::spawn(async move { world.scheduler.unload(world.pid).await })
    };
    // While the invocation is in flight the unload may not write a receipt:
    // its close waits for the invocation to finish.
    for _ in 0..40 {
        assert!(
            receipt_ids(&world, "planned_unload").is_empty(),
            "unload drained while an invocation was still committing"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        journal_halts(&world),
        0,
        "the invocation has not journaled yet"
    );
    resume.send(()).expect("resume the invocation");
    invoker
        .join()
        .expect("invoker")
        .expect("the in-flight halt committed");
    unload.await.expect("unload task").expect("unload");
    assert_eq!(
        receipt_ids(&world, "planned_unload"),
        ["inflight-20-11"],
        "the committed halt ends in exactly one terminal receipt"
    );
    assert_eq!(journal_halts(&world), 1, "the journal control saw the halt");
    assert!(world
        .registry
        .halt_metadata_for_spirit(world.pid, 0)
        .is_empty());
    assert!(is_refused(raise(&world, "after-20-11"), world.pid));
    assert!(world
        .registry
        .lookup_pending_metadata(&HaltId::new("after-20-11").expect("id"))
        .is_none());
}

/// S4: the crash path drains through the same writer, so it fences too.
#[tokio::test]
async fn o4_crash_path_refuses_a_late_halt() {
    let (world, reached, resume) = world("unplanned_crash").await;
    let world = Arc::new(world);
    let detector = maos_kernel_core::supervision::CrashDetector::new(
        world.scheduler.scbs(),
        Arc::clone(&world.tl),
        Arc::clone(&world.registry),
        Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0; 32]),
                BOOT,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                maos_kernel_core::capability::cap_audit::channel().0,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ),
        ),
        Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
                maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
            ))),
            Arc::clone(&world.tl),
        )),
        Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new()),
        Arc::clone(&world.journal),
    );
    let observer = {
        let world = Arc::clone(&world);
        std::thread::spawn(move || {
            reached
                .recv_timeout(Duration::from_secs(10))
                .expect("the crash handler reaches its receipt");
            let refused = is_refused(raise(&world, "late-crash-20-11"), world.pid);
            resume.send(()).expect("resume the crash handler");
            refused
        })
    };
    detector
        .handle_crash(world.pid, maos_domain::supervision::CrashCause::Voluntary)
        .await
        .expect("crash handled");
    assert!(
        observer.join().expect("observer"),
        "a halt after the crash close must be refused with SpiritUnloading(pid)"
    );
    assert_eq!(invocation_rows(&world), 0);
    assert!(world
        .registry
        .halt_metadata_for_spirit(world.pid, 0)
        .is_empty());
}

/// F3 (security review): a receipt must cite ITS OWN frame. The row id was
/// read back with `last_frame_id()`, which any concurrent writer may overwrite
/// between the insert and the read. A tight foreign writer runs while many
/// invocations and terminations are issued; every returned frame id must name
/// the row that was written for it.
#[tokio::test]
async fn receipts_cite_their_own_frame_under_a_concurrent_writer() {
    use maos_domain::halt::{HaltId, HaltState, TerminationKind};
    use maos_kernel_core::halt::{terminate_spirit, PendingHaltMetadata};

    let (world, _reached, _resume) = world("no-pause-20-11").await;
    let world = Arc::new(world);
    let stop = Arc::new(AtomicBool::new(false));
    let writer = {
        let (tl, stop) = (Arc::clone(&world.tl), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                tl.insert_frame_event(
                    FrameKind::TelemetryEvent,
                    0,
                    None,
                    "foreign.writer",
                    b"{}",
                    maos_domain::invariants::i3::FrameOrigin::Kernel,
                );
            }
        })
    };
    let owner = |frame_id: [u8; 16]| {
        world
            .tl
            .query_frames(FrameFilter {
                spirit_pid: Some(world.pid),
                kind: Some(FrameKind::EpistemicHalt),
                ..Default::default()
            })
            .expect("halt rows")
            .into_iter()
            .find(|row| row.frame_id == frame_id)
    };
    let mut misbound = Vec::new();
    for round in 0..400 {
        let receipt = raise(&world, &format!("own-{round}")).expect("an open pid accepts a halt");
        if !owner(receipt.frame_id).is_some_and(|row| row.intent == "ambiguity") {
            misbound.push(format!("invoke own-{round}"));
        }
    }
    let registry = HaltRegistry::new();
    for round in 0..400 {
        let id = format!("term-{round}");
        registry
            .insert_pending_with_metadata(
                HaltId::new(id.clone()).expect("id"),
                HaltState::PendingResolution,
                PendingHaltMetadata {
                    spirit_pid: world.pid,
                    spirit_id: "race-20-11".into(),
                    payload: halt(&id),
                    fired_ns: 0,
                },
            )
            .expect("pending");
        for receipt in terminate_spirit(
            &world.tl,
            &registry,
            world.pid,
            "race-20-11",
            TerminationKind::PlannedUnload,
            BOOT,
        ) {
            if !owner(receipt.frame_id).is_some_and(|row| {
                row.intent == "planned_unload" && row.payload_redacted.is_empty()
            }) {
                misbound.push(format!("terminate {id}"));
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    writer.join().expect("writer");
    assert!(
        misbound.is_empty(),
        "{} receipts cite a foreign frame: {:?}",
        misbound.len(),
        &misbound[..misbound.len().min(5)]
    );
}

/// Round 3: a halt whose provenance is large (a cold Butler assessment's
/// `derived_from` exceeds 4096 bytes) is still invoked, journaled, registered
/// and receipted; no size policy may turn a halt into "no halt".
#[tokio::test]
async fn an_oversized_provenance_halt_is_still_invoked_and_receipted() {
    let (world, _reached, _resume) = world("no-pause-20-11").await;
    let payload = EpistemicHaltPayload::new(
        "halt-big-20-11".into(),
        "ambiguity".into(),
        0.99,
        Some(0.7),
        "policy-20-11".into(),
        "e".repeat(100_000),
    )
    .expect("a large provenance never refuses the halt");
    let receipt = invoke_halt(
        &world.tl,
        &world.journal,
        &world.registry,
        payload,
        world.pid,
        "race-20-11",
        BOOT,
    )
    .expect("the halt is invoked");
    assert_eq!(receipt.halt_id.as_str(), "halt-big-20-11");
    assert!(world
        .registry
        .lookup_pending_metadata(&HaltId::new("halt-big-20-11").expect("id"))
        .is_some());
    assert_eq!(journal_halts(&world), 1);
    assert_eq!(invocation_rows(&world), 1);
}
