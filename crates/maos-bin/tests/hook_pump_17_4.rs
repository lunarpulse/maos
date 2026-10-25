#![cfg(feature = "network")]
#![forbid(unsafe_code)]
//! Story 17-4 — the telemetry hook pump, the consolidation watchdog and the
//! activity writer, driven through the production admission path
//! (`load_admit_start`) against real kernel objects: a real Transparency Log,
//! capability registry, telemetry adapter, scheduler and hook dispatcher.
//!
//! What a consumer can observe is asserted: what the Observer's sink received,
//! the pump's counters, and the `governance:telemetry-subscription:*` rows.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use maos_bin::admission::{
    gate_manifest, load_admit_start, AdmissionDeps, AdmissionRefusal, GatedManifest, StartPolicy,
    KNOWN_CLASS_NAMES,
};
use maos_bin::hook_pump::{
    hook_panic_line, ActivityWriter, ConsolidationCadence, ConsolidationWatchdog, HookPump,
    PumpStats, QUARANTINE_AFTER_CONSECUTIVE_PANICS, SUBSCRIPTION_GRANTED, SUBSCRIPTION_REVOKED,
};
use maos_domain::governance::{GovernanceEventKind, GovernanceEventPayload};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::control_block::{AnySpiritObj, SpiritControlBlock};
use maos_kernel_core::scheduler::{make_spirit_obj, SpiritSchedulerAdapter};
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_spirit_abi::ctx::Ctx;
use maos_spirit_abi::lifecycle::{ConsolidatePayload, Spirit, TelemetryEventPayload};
use observer::{ActivitySink, ObservedActivity, Observer};

const OBSERVER_MANIFEST: &str = include_str!("../../../spirits/observer/manifest.toml");
const NONCE: u64 = 0x17_04;
const EMITTER_PID: u32 = 990_001;

struct World {
    _dir: tempfile::TempDir,
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    security: Arc<SecurityManagerAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    iac: Arc<IacBusAdapter>,
    metrics: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    pump: HookPump,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        // 18-0 AC4(a): set the clock base ONCE per process. Under libtest (the
        // loom gate's `cargo test -p maos-bin` leg) sibling tests share this
        // process; a re-init re-stores MONOTONIC_BASE while BOOT_INSTANT keeps
        // the first value, so `monotonic_now_ns()` jumped forward mid-test.
        static CLOCK: std::sync::Once = std::sync::Once::new();
        CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.path().join("transparency.sqlite"), NONCE).unwrap(),
        );
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, mut audit_rx) = cap_audit::channel();
        std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
        let telemetry = Arc::new(TelemetryStreamAdapter::default());
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            policy,
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::clone(&telemetry),
        ));
        let database = dir.path().join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.path().join("private"),
                4,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&database).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&database)
                    .unwrap(),
            ),
            Arc::clone(&tl),
        ));
        let iac = Arc::new(IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
            Arc::clone(&tl),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let scheduler = Arc::new(SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            Arc::clone(&memory),
            Arc::clone(&iac),
            Arc::clone(&halt_registry),
            Arc::clone(&metrics),
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&dir.path().join("lifecycle.ndjson"))
                .unwrap(),
        );
        let pump = HookPump::new(&scheduler, &telemetry, Arc::clone(&tl));
        Self {
            _dir: dir,
            tl,
            capability,
            security,
            memory,
            journal,
            scheduler,
            halt_registry,
            iac,
            metrics,
            lookup: Arc::new(RwLock::new(BTreeMap::new())),
            pump,
        }
    }

    async fn admit_gated(
        &self,
        gated: &GatedManifest,
        obj: Arc<dyn AnySpiritObj>,
        start: StartPolicy,
    ) -> Result<u32, AdmissionRefusal> {
        load_admit_start(
            &AdmissionDeps {
                scheduler: &self.scheduler,
                security: &self.security,
                transparency_log: &self.tl,
                journal: &self.journal,
                memory: &self.memory,
                pid_by_spirit_id: &self.lookup,
                hooks: Some(&self.pump),
            },
            gated,
            obj,
            NONCE,
            start,
            &mut |_| {},
        )
        .await
        .map(|admitted| admitted.pid)
    }

    async fn admit(
        &self,
        manifest: &str,
        obj: Arc<dyn AnySpiritObj>,
        start: StartPolicy,
    ) -> Result<u32, AdmissionRefusal> {
        let gated = gate_manifest(manifest, &|name| KNOWN_CLASS_NAMES.contains(&name))?;
        self.admit_gated(&gated, obj, start).await
    }

    /// Publish one write through the production path and wait for the pump
    /// to dispatch or account it.
    async fn emit(&self, value: f64) -> u64 {
        let seq = self
            .capability
            .set_scalar(EMITTER_PID, "emitter", "belief_variance", value, "test")
            .unwrap()
            .seq;
        self.pump.flush().await.unwrap();
        seq
    }

    fn scb(&self, pid: u32) -> Arc<SpiritControlBlock> {
        self.scheduler
            .scbs()
            .read()
            .unwrap()
            .get(&pid)
            .cloned()
            .unwrap()
    }

    /// Hot-swap under `pid`: the class and object change, the pid stays.
    fn swap(&self, pid: u32, class: &str, obj: Arc<dyn AnySpiritObj>) {
        let scb = self.scb(pid);
        let mut runtime = scb.runtime_snapshot();
        runtime.manifest.class.as_mut().unwrap().name = class.into();
        runtime.spirit_obj = obj;
        scb.replace_runtime(runtime);
    }

    /// `(pid, granted, reason)` for every subscription row, in write order;
    /// each row's TL pid, intent and payload must agree.
    fn subscription_rows(&self) -> Vec<(u32, bool, String)> {
        self.tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::GovernanceEvent),
                order_by_insertion: true,
                ..Default::default()
            })
            .unwrap()
            .into_iter()
            .filter(|row| row.intent.starts_with("governance:telemetry-subscription:"))
            .map(|row| {
                let payload: GovernanceEventPayload =
                    serde_json::from_slice(&row.payload_redacted).unwrap();
                let GovernanceEventKind::TelemetrySubscription(sub) = payload.event else {
                    panic!("subscription row with another payload: {}", row.intent);
                };
                assert_eq!(row.spirit_pid, sub.subscriber_pid);
                assert_eq!(sub.scope, "scalar.tap.*");
                let expected = if sub.granted {
                    SUBSCRIPTION_GRANTED
                } else {
                    SUBSCRIPTION_REVOKED
                };
                assert_eq!(row.intent, expected);
                (sub.subscriber_pid, sub.granted, sub.reason)
            })
            .collect()
    }
}

fn row(pid: u32, granted: bool, reason: &str) -> (u32, bool, String) {
    (pid, granted, reason.to_owned())
}

#[derive(Debug, Default)]
struct Recorded(Mutex<Vec<ObservedActivity>>);

impl ActivitySink for Recorded {
    fn emit(&self, activity: ObservedActivity) {
        self.0.lock().unwrap().push(activity);
    }
}

impl Recorded {
    fn scalar_seqs(&self) -> Vec<u64> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|record| match record {
                ObservedActivity::Scalar { seq, .. } => Some(*seq),
                _ => None,
            })
            .collect()
    }
}

fn observer_with(sink: &Arc<Recorded>) -> Arc<dyn AnySpiritObj> {
    make_spirit_obj(
        Observer::default()
            .with_id("observer")
            .with_activity_sink(Arc::clone(sink) as Arc<dyn ActivitySink>),
    )
}

fn stats(delivered: u64, ineligible: u64, failed: u64) -> PumpStats {
    PumpStats {
        delivered,
        ineligible,
        failed,
        ..PumpStats::default()
    }
}

/// A subscriber whose telemetry hook always panics.
struct PanicsOnTelemetry;
impl Spirit for PanicsOnTelemetry {
    fn on_telemetry_event(&self, _ctx: &mut Ctx, _payload: &TelemetryEventPayload<'_>) {
        panic!("subscriber fault");
    }
}

/// A subscriber whose telemetry hook outlives its budget.
struct SlowOnTelemetry;
impl Spirit for SlowOnTelemetry {
    fn on_telemetry_event(&self, _ctx: &mut Ctx, _payload: &TelemetryEventPayload<'_>) {
        std::thread::sleep(Duration::from_millis(1_500));
    }
}

/// A Spirit whose `on_start` panics: admission is granted, start fails.
struct PanicsOnStart;
impl Spirit for PanicsOnStart {
    fn on_start(&self, _ctx: &mut Ctx) {
        panic!("start fault");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delivery_follows_running_state_and_the_live_class_under_a_pid() {
    let world = World::new();
    let sink = Arc::new(Recorded::default());
    let pid = world
        .admit(
            OBSERVER_MANIFEST,
            observer_with(&sink),
            StartPolicy::LeaveLoaded,
        )
        .await
        .unwrap();
    assert_eq!(world.subscription_rows(), [row(pid, true, "admitted")]);

    world.emit(0.1).await;
    assert_eq!(world.pump.stats(), stats(0, 1, 0), "Loaded is not Running");

    world.scheduler.start(pid).await.unwrap();
    let delivered = world.emit(0.2).await;
    assert_eq!(world.pump.stats(), stats(1, 1, 0));
    assert_eq!(sink.scalar_seqs(), [delivered]);

    world.scheduler.pause(pid).await.unwrap();
    world.emit(0.3).await;
    assert_eq!(world.pump.stats(), stats(1, 2, 0), "Paused is not Running");
    world.scheduler.resume(pid).await.unwrap();
    world.emit(0.4).await;
    assert_eq!(world.pump.stats(), stats(2, 2, 0));

    // An upgrade that changes the class under the pid stops delivery at the
    // next event and revokes the grant.
    world.swap(pid, "nash", make_spirit_obj(nash::Nash::default()));
    world.emit(0.5).await;
    assert_eq!(world.pump.stats(), stats(2, 3, 0));
    assert_eq!(sink.scalar_seqs().len(), 2);

    // An upgrade back to Observer starts it again, with a new grant.
    let successor = Arc::new(Recorded::default());
    world.swap(pid, "observer", observer_with(&successor));
    let resumed = world.emit(0.6).await;
    assert_eq!(world.pump.stats(), stats(3, 3, 0));
    assert_eq!(successor.scalar_seqs(), [resumed]);
    assert_eq!(
        world.subscription_rows(),
        [
            row(pid, true, "admitted"),
            row(pid, false, "class-changed"),
            row(pid, true, "eligible"),
        ]
    );

    world.pump.shutdown().await;
    assert_eq!(
        world.subscription_rows().last(),
        Some(&row(pid, false, "root-shutdown"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_panicking_subscriber_is_quarantined_until_a_successor_replaces_it() {
    let world = World::new();
    let pid = world
        .admit(
            OBSERVER_MANIFEST,
            make_spirit_obj(PanicsOnTelemetry),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    for _ in 0..QUARANTINE_AFTER_CONSECUTIVE_PANICS {
        world.emit(0.1).await;
    }
    let k = u64::from(QUARANTINE_AFTER_CONSECUTIVE_PANICS);
    assert_eq!(world.pump.stats(), stats(0, 0, k));
    assert_eq!(
        world.subscription_rows(),
        [
            row(pid, true, "admitted"),
            row(
                pid,
                false,
                "quarantined: on_telemetry_event panicked 3 times in a row"
            ),
        ]
    );

    world.emit(0.2).await;
    assert_eq!(
        world.pump.stats(),
        stats(0, 1, k),
        "quarantined: not dispatched"
    );
    world.scheduler.pause(pid).await.unwrap();
    world.scheduler.resume(pid).await.unwrap();
    world.emit(0.3).await;
    assert_eq!(
        world.pump.stats(),
        stats(0, 2, k),
        "resume does not lift quarantine"
    );

    let sink = Arc::new(Recorded::default());
    world.swap(pid, "observer", observer_with(&sink));
    let clean = world.emit(0.4).await;
    assert_eq!(
        world.pump.stats(),
        stats(1, 2, k),
        "a successor starts clean"
    );
    assert_eq!(sink.scalar_seqs(), [clean]);
    assert_eq!(
        world.subscription_rows().last(),
        Some(&row(pid, true, "eligible"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_budget_overrun_quarantines_the_subscriber() {
    let world = World::new();
    let manifest = OBSERVER_MANIFEST.replace("time_cap_seconds = 10", "time_cap_seconds = 1");
    let pid = world
        .admit(
            &manifest,
            make_spirit_obj(SlowOnTelemetry),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    world.emit(0.1).await;
    assert_eq!(world.pump.stats(), stats(0, 0, 1));
    assert_eq!(
        world.subscription_rows().last(),
        Some(&row(
            pid,
            false,
            "quarantined: on_telemetry_event exceeded its 1 s budget"
        ))
    );
    world.emit(0.2).await;
    assert_eq!(world.pump.stats(), stats(0, 1, 1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refused_admission_or_a_failed_start_leaves_no_registration() {
    let world = World::new();
    let too_new = OBSERVER_MANIFEST.replace(
        "min_substrate_version = \"0.1.0-alpha\"",
        "min_substrate_version = \"99.0.0\"",
    );
    let refused = world
        .admit(
            &too_new,
            make_spirit_obj(Observer::default()),
            StartPolicy::StartNow,
        )
        .await;
    assert!(
        matches!(refused, Err(AdmissionRefusal::Admission { .. })),
        "{refused:?}"
    );
    assert!(
        world.subscription_rows().is_empty(),
        "no grant before the decision"
    );

    let failed = world
        .admit(
            OBSERVER_MANIFEST,
            make_spirit_obj(PanicsOnStart),
            StartPolicy::StartNow,
        )
        .await;
    assert!(
        matches!(failed, Err(AdmissionRefusal::Start { .. })),
        "{failed:?}"
    );
    let rows = world.subscription_rows();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!((rows[0].1, rows[0].2.as_str()), (true, "admitted"));
    assert_eq!(rows[1], row(rows[0].0, false, "start-failed"));

    world.emit(0.1).await;
    assert_eq!(
        world.pump.stats(),
        PumpStats::default(),
        "nothing is registered"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_spawned_form_never_registers_even_when_the_caller_passes_the_pump() {
    let world = World::new();
    let mut gated =
        gate_manifest(OBSERVER_MANIFEST, &|name| KNOWN_CLASS_NAMES.contains(&name)).unwrap();
    gated.class_section.forms = vec!["wasm-component".into()];
    let sink = Arc::new(Recorded::default());
    let pid = world
        .admit_gated(&gated, observer_with(&sink), StartPolicy::StartNow)
        .await
        .expect("the spawned-form admission itself is granted");
    assert!(world.subscription_rows().is_empty());

    let watchdog = ConsolidationWatchdog::spawn(
        &world.pump,
        ConsolidationCadence {
            poll: Duration::from_millis(10),
            window_divisor: 1_000,
        },
    );
    world.emit(0.1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    watchdog.shutdown().await;
    assert_eq!(world.pump.stats(), PumpStats::default());
    assert!(sink.0.lock().unwrap().is_empty(), "pid {pid} got a hook");
}

#[derive(Debug)]
struct BrokenPipe;
impl Write for BrokenPipe {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_closed_output_is_counted_and_never_fails_a_hook() {
    let world = World::new();
    let writer = ActivityWriter::spawn(Box::new(BrokenPipe), 64);
    let observer = Observer::default()
        .with_id("observer")
        .with_activity_sink(writer.sink());
    world
        .admit(
            OBSERVER_MANIFEST,
            make_spirit_obj(observer),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    for i in 0..20 {
        world.emit(f64::from(i)).await;
    }
    assert_eq!(world.pump.stats(), stats(20, 0, 0));
    writer.close().await;
    let out = writer.stats();
    assert_eq!((out.written, out.write_errors, out.overflowed), (0, 20, 0));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_full_pipe_never_blocks_a_hook() {
    let world = World::new();
    // Fill an owned pipe, restore blocking mode, and never drain it. Only
    // the output thread may detach; hook dispatch still completes normally.
    let (_unread, pipe) = std::io::pipe().unwrap();
    let flags = rustix::fs::fcntl_getfl(&pipe).unwrap();
    rustix::fs::fcntl_setfl(&pipe, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
    while rustix::io::write(&pipe, &[b'x'; 4096]).is_ok() {}
    while rustix::io::write(&pipe, b"x").is_ok() {}
    rustix::fs::fcntl_setfl(&pipe, flags).unwrap();
    let writer = ActivityWriter::spawn(Box::new(pipe), 8);
    let observer = Observer::default()
        .with_id("observer")
        .with_activity_sink(writer.sink());
    world
        .admit(
            OBSERVER_MANIFEST,
            make_spirit_obj(observer),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    let started = Instant::now();
    for i in 0..2_000 {
        world
            .capability
            .set_scalar(
                EMITTER_PID,
                "emitter",
                "a_rather_long_tag_to_fill_the_pipe",
                f64::from(i),
                "t",
            )
            .unwrap();
    }
    world.pump.flush().await.unwrap();
    let pump = world.pump.stats();
    assert_eq!(pump.failed, 0, "no hook may exceed its budget on output");
    assert_eq!(pump.delivered + pump.lagged_skipped, 2_000);
    assert!(
        started.elapsed() < Duration::from_secs(9),
        "dispatch stalled: {:?}",
        started.elapsed()
    );
    assert!(
        writer.stats().overflowed > 0,
        "the bounded queue overflowed and counted it"
    );
    tokio::time::timeout(Duration::from_secs(2), writer.close())
        .await
        .expect("activity writer shutdown waited for the unread pipe");
    let output = writer.stats();
    assert!(
        output.shutdown_abandoned > 0,
        "unconfirmed records must be accounted at detach"
    );
    assert_eq!(
        output.written + output.write_errors + output.overflowed + output.shutdown_abandoned,
        pump.delivered,
        "every sink emission must be written or accounted"
    );
    assert!(output.detached);
}

#[test]
fn stderr_lines_escape_spirit_controlled_text() {
    let line = hook_panic_line(
        "on_telemetry_event",
        7,
        "obs\u{202e}erver",
        Some("tag\u{1b}[31m\nnext"),
        "boom\u{7}\r",
    );
    for raw in ['\u{1b}', '\u{202e}', '\n', '\r', '\u{7}'] {
        assert!(!line.contains(raw), "raw {raw:?} reached the line: {line}");
    }
    for escaped in ["\\u{202e}", "\\u{1b}[31m", "\\n", "\\r", "\\u{7}"] {
        assert!(line.contains(escaped), "{escaped} missing from {line}");
    }
    assert!(
        line.starts_with("maos: telemetry pump: on_telemetry_event panicked in subscriber pid 7")
    );
}

// ── AC3 — the consolidation watchdog ──────────────────────────────────────

/// Counts `on_consolidate` calls and the payload bytes it was handed; when a
/// test installs `fired_at`, each call also reports its production-clock stamp.
#[derive(Default)]
struct Consolidations {
    count: AtomicU32,
    payload_bytes: AtomicU32,
    fired_at: Mutex<Option<tokio::sync::mpsc::UnboundedSender<u64>>>,
}

struct CountsConsolidations(Arc<Consolidations>);
impl Spirit for CountsConsolidations {
    fn on_consolidate(&self, _ctx: &mut Ctx, payload: &ConsolidatePayload<'_>) {
        self.0.count.fetch_add(1, Ordering::SeqCst);
        self.0
            .payload_bytes
            .fetch_add(payload.batch_len as u32, Ordering::SeqCst);
        if let Some(tx) = self.0.fired_at.lock().unwrap().as_ref() {
            // A closed receiver means the test has its stamps; nothing to report.
            let _ = tx.send(maos_kernel_core::capability::cap_tokens::monotonic_now_ns());
        }
    }
}

fn with_window(manifest: &str, idle_window_ms: u32) -> String {
    format!("{manifest}\n[scheduling]\nidle_window_ms = {idle_window_ms}\n")
}

fn fast(pump: &HookPump) -> ConsolidationWatchdog {
    ConsolidationWatchdog::spawn(
        pump,
        ConsolidationCadence {
            poll: Duration::from_millis(10),
            window_divisor: 1,
        },
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consolidation_repeats_every_quiescent_window_and_only_while_running() {
    let world = World::new();
    let counts = Arc::new(Consolidations::default());
    let pid = world
        .admit(
            &with_window(OBSERVER_MANIFEST, 100),
            make_spirit_obj(CountsConsolidations(Arc::clone(&counts))),
            StartPolicy::LeaveLoaded,
        )
        .await
        .unwrap();
    let watchdog = fast(&world.pump);

    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(
        counts.count.load(Ordering::SeqCst),
        0,
        "Loaded never consolidates"
    );

    let (fired_tx, mut fired) = tokio::sync::mpsc::unbounded_channel();
    *counts.fired_at.lock().unwrap() = Some(fired_tx);
    world.scheduler.start(pid).await.unwrap();
    // Barrier, not a deadline: await three fires. The 10 s timeout is a hang
    // guard only (a healthy run fires every ~100 ms).
    let mut stamps = Vec::new();
    while stamps.len() < 3 {
        let stamp = tokio::time::timeout(Duration::from_secs(10), fired.recv())
            .await
            .expect("hang guard (10 s): a Running, quiescent Spirit stopped consolidating")
            .expect("the fire channel stays open while the Spirit is loaded");
        stamps.push(stamp);
    }
    // "At most once per window", on the clock the watchdog itself reads; and
    // "every quiescent window" (review 2026-10-25): each gap < 5 s = 50 fast
    // windows, which catches the ÷100 fast knob regressing (≈10 s gaps) while
    // leaving wide slack so no wall-clock flake returns.
    for pair in stamps.windows(2) {
        let gap = pair[1].saturating_sub(pair[0]);
        assert!(
            gap >= 100_000_000,
            "at most once per 100 ms window on the production clock: {stamps:?}"
        );
        assert!(
            gap < 5_000_000_000,
            "a quiescent Spirit consolidates every fast window (gap < 5 s): {stamps:?}"
        );
    }
    assert_eq!(
        counts.payload_bytes.load(Ordering::SeqCst),
        0,
        "curate now, never what"
    );

    world.scheduler.pause(pid).await.unwrap();
    // A Running check may legitimately precede pause while its dispatch
    // finishes afterwards (FR51(a)). Join that work before sampling.
    watchdog.shutdown().await;
    let paused_at = counts.count.load(Ordering::SeqCst);
    let watchdog = fast(&world.pump);
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(
        counts.count.load(Ordering::SeqCst),
        paused_at,
        "Paused never consolidates"
    );
    watchdog.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fresh_inbound_frames_hold_consolidation_off() {
    let world = World::new();
    let counts = Arc::new(Consolidations::default());
    let pid = world
        .admit(
            &with_window(OBSERVER_MANIFEST, 150),
            make_spirit_obj(CountsConsolidations(Arc::clone(&counts))),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    let scb = world.scb(pid);
    let traffic = Arc::new(AtomicBool::new(true));
    let feeder = {
        let traffic = Arc::clone(&traffic);
        let scb = Arc::clone(&scb);
        tokio::spawn(async move {
            // What `Mailbox::deliver` does for every inbound frame.
            while traffic.load(Ordering::SeqCst) {
                scb.last_inbound_frame_ns.store(
                    maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                    Ordering::Relaxed,
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
    };
    let watchdog = fast(&world.pump);
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        counts.count.load(Ordering::SeqCst),
        0,
        "inbound traffic is not quiescence"
    );
    traffic.store(false, Ordering::SeqCst);
    feeder.await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(
        counts.count.load(Ordering::SeqCst) >= 1,
        "quiescence resumes consolidation"
    );
    watchdog.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_manifest_without_on_consolidate_is_skipped_and_never_stamped() {
    let world = World::new();
    let counts = Arc::new(Consolidations::default());
    let manifest = format!(
        "{}\n[lifecycle]\nenabled_hooks = [\"on_load\", \"on_start\", \"on_idle\"]\n",
        with_window(OBSERVER_MANIFEST, 1_000)
    );
    let pid = world
        .admit(
            &manifest,
            make_spirit_obj(CountsConsolidations(Arc::clone(&counts))),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    let watchdog = fast(&world.pump);
    tokio::time::sleep(Duration::from_millis(1_300)).await;
    assert_eq!(counts.count.load(Ordering::SeqCst), 0, "SkippedManifest");

    // Enable the hook in place. Had each skipped poll stamped the window, the
    // first consolidation would wait a further full second; unstamped, it
    // fires on the next poll.
    let scb = world.scb(pid);
    let mut runtime = scb.runtime_snapshot();
    runtime
        .manifest
        .lifecycle
        .enabled_hooks
        .push("on_consolidate".into());
    scb.replace_runtime(runtime);
    let enabled_at = Instant::now();
    while counts.count.load(Ordering::SeqCst) == 0 {
        assert!(
            enabled_at.elapsed() < Duration::from_millis(500),
            "a skipped poll stamped the window"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    watchdog.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_delivery_and_consolidation_never_split_a_window() {
    let world = World::new();
    let sink = Arc::new(Recorded::default());
    world
        .admit(
            &with_window(OBSERVER_MANIFEST, 100),
            observer_with(&sink),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    let watchdog = fast(&world.pump);
    for i in 0..1_500 {
        world
            .capability
            .set_scalar(
                EMITTER_PID + (i % 3),
                &format!("e{}", i % 3),
                "t",
                f64::from(i),
                "x",
            )
            .unwrap();
        if i % 50 == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    world.pump.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    watchdog.shutdown().await;

    let records = sink.0.lock().unwrap().clone();
    let mut since_close = 0_u64;
    let mut closes = 0;
    for record in &records {
        match record {
            ObservedActivity::Scalar { .. } => since_close += 1,
            ObservedActivity::Consolidated { window_events, .. } => {
                assert_eq!(
                    *window_events, since_close,
                    "a window counts what preceded it"
                );
                since_close = 0;
                closes += 1;
            }
            ObservedActivity::Anomaly { .. } => panic!("no watches configured"),
        }
    }
    assert!(closes >= 2, "consolidation ran during delivery ({closes})");
    assert_eq!(since_close, 0, "the last window was closed");
    assert_eq!(world.pump.stats().delivered, 1_500);
}

/// Sleeps 6 s inside `on_consolidate` under the manifest's 10 s budget.
#[cfg(unix)]
struct SlowConsolidation {
    entered: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    unload_capture: Arc<parking_lot::Mutex<Option<(bool, bool)>>>,
    tl: Arc<TransparencyLogAdapter>,
}

#[cfg(unix)]
impl Spirit for SlowConsolidation {
    fn on_consolidate(&self, _ctx: &mut Ctx, _payload: &ConsolidatePayload<'_>) {
        if self.entered.swap(true, Ordering::SeqCst) {
            return;
        }
        std::thread::sleep(Duration::from_secs(6));
        self.finished.store(true, Ordering::SeqCst);
    }

    fn on_unload(&self, _ctx: &mut Ctx) {
        let pump_joined = self
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::GovernanceEvent),
                ..Default::default()
            })
            .unwrap()
            .iter()
            .any(|row| {
                if row.intent != SUBSCRIPTION_REVOKED {
                    return false;
                }
                let payload: GovernanceEventPayload =
                    serde_json::from_slice(&row.payload_redacted).unwrap();
                matches!(
                    payload.event,
                    GovernanceEventKind::TelemetrySubscription(sub)
                        if sub.reason == "root-shutdown"
                )
            });
        *self.unload_capture.lock() = Some((self.finished.load(Ordering::SeqCst), pump_joined));
    }
}

/// AC3.1 — real SIGTERM reaches RootSupervision's installed handler while a
/// six-second hook runs; the serving root's shared teardown owns all joins.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_joins_an_in_flight_consolidation_instead_of_detaching_it() {
    const CHILD_DIR: &str = "MAOS_17_4_SHUTDOWN_CHILD_DIR";
    const SLOW_CONSOLIDATION_ENTRY_TIMEOUT: Duration = Duration::from_secs(10);
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

    if let Some(dir) = std::env::var_os(CHILD_DIR) {
        let dir = std::path::PathBuf::from(dir);
        let world = World::new();
        let crash = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            world.scheduler.scbs(),
            Arc::clone(&world.tl),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.capability),
            Arc::clone(&world.iac),
            Arc::clone(&world.metrics),
            Arc::clone(&world.journal),
        ));
        let root = maos_bin::supervision::RootSupervision::arm(
            Arc::clone(&world.scheduler),
            crash,
            Arc::clone(&world.tl),
            Arc::clone(&world.journal),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.iac),
            Arc::clone(&world.metrics),
            Arc::new(maos_director_surface::notification::NotificationDispatcher::default()),
            NONCE,
        );
        let entered = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let unload_capture = Arc::new(parking_lot::Mutex::new(None));
        let pid = world
            .admit(
                &with_window(OBSERVER_MANIFEST, 100),
                make_spirit_obj(SlowConsolidation {
                    entered: Arc::clone(&entered),
                    finished: Arc::clone(&finished),
                    unload_capture: Arc::clone(&unload_capture),
                    tl: Arc::clone(&world.tl),
                }),
                StartPolicy::StartNow,
            )
            .await
            .unwrap();
        let watchdog = fast(&world.pump);
        tokio::time::timeout(SLOW_CONSOLIDATION_ENTRY_TIMEOUT, async {
            while !entered.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("slow consolidation never entered before its named entry deadline");
        // RootSupervision installed the real signal streams synchronously
        // before this handshake; the parent may now deliver SIGTERM safely.
        std::fs::write(dir.join("consolidation-entered"), b"ready").unwrap();
        tokio::time::timeout(SHUTDOWN_TIMEOUT, root.shutdown_token().cancelled())
            .await
            .expect("production root signal handler did not observe SIGTERM");
        assert!(
            !finished.load(Ordering::SeqCst),
            "SIGTERM must arrive while the six-second consolidation is still running"
        );
        let report = tokio::time::timeout(
            SHUTDOWN_TIMEOUT,
            maos_bin::supervision::finish_serving_shutdown(
                &watchdog,
                &world.pump,
                root.stop_and_join(),
                world.scheduler.as_ref(),
                world.halt_registry.as_ref(),
                None,
            ),
        )
        .await
        .expect("production serving shutdown failed to join within its deadline");
        assert!(!report.had_failures());
        assert_eq!(
            *unload_capture.lock(),
            Some((true, true)),
            "on_unload ran before consolidation finished or before pump shutdown"
        );
        assert!(world.scheduler.scbs().read().unwrap().get(&pid).is_none());
        assert_eq!(
            world.subscription_rows(),
            [row(pid, true, "admitted"), row(pid, false, "root-shutdown")],
            "a hook within its budget is never quarantined"
        );
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "shutdown_joins_an_in_flight_consolidation_instead_of_detaching_it",
            "--nocapture",
        ])
        .env(CHILD_DIR, dir.path())
        .env("MAOS_HOME", dir.path().join("maos-home"))
        .env("HOME", dir.path())
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn isolated real-signal regression");
    tokio::time::timeout(SLOW_CONSOLIDATION_ENTRY_TIMEOUT, async {
        while !dir.path().join("consolidation-entered").exists() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "signal regression subprocess exited before slow consolidation entered"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("subprocess slow consolidation never entered before its named entry deadline");
    assert!(
        tokio::process::Command::new("kill")
            .args(["-TERM", &child.id().unwrap().to_string()])
            .status()
            .await
            .unwrap()
            .success(),
        "deliver real SIGTERM to the production root signal handler"
    );
    let output = tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait_with_output())
        .await
        .expect("real-SIGTERM serving teardown exceeded its deadline")
        .unwrap();
    assert!(
        output.status.success(),
        "real-SIGTERM serving teardown failed: status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Keep both standard pipes saturated throughout emission AND writer shutdown.
/// A subprocess isolates descriptor flags and prevents the test harness itself
/// from consuming the bytes whose backpressure is the regression trigger.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_stdout_and_stderr_do_not_block_activity_or_shutdown() {
    const CHILD: &str = "MAOS_17_4_OUTPUT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        fn fill(fd: impl std::os::fd::AsFd) -> rustix::fs::OFlags {
            let flags = rustix::fs::fcntl_getfl(&fd).unwrap();
            rustix::fs::fcntl_setfl(&fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
            loop {
                match rustix::io::write(&fd, &[b'x'; 4096]) {
                    Ok(_) => {}
                    Err(rustix::io::Errno::AGAIN) => break,
                    Err(error) => panic!("fill standard pipe: {error}"),
                }
            }
            while rustix::io::write(&fd, b"x").is_ok() {}
            rustix::fs::fcntl_setfl(&fd, flags).unwrap();
            flags
        }
        let stdout_flags = fill(std::io::stdout());
        let stderr_flags = fill(std::io::stderr());
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let stderr = std::io::stderr();
            let mut lock = stderr.lock();
            locked_tx.send(()).unwrap();
            let _ = writeln!(lock, "another root diagnostic");
        });
        locked_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let writer = ActivityWriter::stdout();
        let sink = writer.sink();
        for seq in 0..10_000 {
            sink.emit(ObservedActivity::Scalar {
                observer: "observer".into(),
                spirit_id: "emitter".into(),
                tag: "belief_variance".into(),
                value: 0.8,
                seq,
                timestamp_ms: 0,
            });
        }
        tokio::time::timeout(Duration::from_secs(3), writer.close())
            .await
            .expect("full standard pipes blocked activity writer close");
        let output = writer.stats();
        assert!(output.overflowed > 0);
        assert!(output.shutdown_abandoned > 0);
        assert!(output.detached);
        assert_eq!(output.written, 0);
        assert_eq!(output.write_errors, 0);
        assert_eq!(output.overflowed + output.shutdown_abandoned, 10_000);
        assert_eq!(
            rustix::fs::fcntl_getfl(std::io::stdout()).unwrap(),
            stdout_flags
        );
        assert_eq!(
            rustix::fs::fcntl_getfl(std::io::stderr()).unwrap(),
            stderr_flags
        );
        // The harness would print to the intentionally full pipe after return.
        std::process::exit(0);
    }
    let mut child = tokio::process::Command::new("/bin/sh")
        .args(["-c", "exec \"$@\" 2>&1", "activity-output-test"])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "full_stdout_and_stderr_do_not_block_activity_or_shutdown",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let status = tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("overflow emission or writer shutdown blocked on a full standard pipe")
        .unwrap();
    assert!(status.success(), "backpressure child failed: {status}");
}

struct FeedbackTelemetry {
    capability: Arc<CapabilityRegistryAdapter>,
    active: Arc<AtomicBool>,
    emitted: Arc<AtomicU64>,
}

impl Spirit for FeedbackTelemetry {
    fn on_telemetry_event(&self, _ctx: &mut Ctx, _payload: &TelemetryEventPayload<'_>) {
        if self.active.load(Ordering::Acquire) {
            self.capability
                .set_scalar(EMITTER_PID, "emitter", "belief_variance", 0.1, "feedback")
                .unwrap();
            self.emitted.fetch_add(1, Ordering::Relaxed);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flush_and_shutdown_finish_while_publication_continues() {
    let world = World::new();
    let active = Arc::new(AtomicBool::new(true));
    let emitted = Arc::new(AtomicU64::new(1));
    world
        .admit(
            OBSERVER_MANIFEST,
            make_spirit_obj(FeedbackTelemetry {
                capability: Arc::clone(&world.capability),
                active: Arc::clone(&active),
                emitted: Arc::clone(&emitted),
            }),
            StartPolicy::StartNow,
        )
        .await
        .unwrap();
    world
        .capability
        .set_scalar(EMITTER_PID, "emitter", "belief_variance", 0.0, "seed")
        .unwrap();
    let flushed = tokio::time::timeout(Duration::from_secs(2), world.pump.flush()).await;
    // The feedback hook enqueues another event before it returns: the receiver
    // cannot become empty while this flag is set.
    let stopped = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(world.pump.flush(), world.pump.shutdown())
    })
    .await;
    active.store(false, Ordering::Release);
    world.pump.shutdown().await;
    assert!(
        matches!(flushed, Ok(Ok(()))),
        "flush waited for subsequent publications"
    );
    assert!(stopped.is_ok(), "flush drain ignored shutdown cancellation");
    assert!(world.pump.stats().delivered >= 1);
    let stats = world.pump.stats();
    assert_eq!(
        stats.delivered + stats.lagged_skipped + stats.shutdown_skipped,
        emitted.load(Ordering::Relaxed),
        "cancellation must account for the unread suffix"
    );
    assert_eq!(
        stats.shutdown_skipped, 1,
        "one feedback event remains at cancellation"
    );
}

#[test]
fn a_queued_hook_invokes_and_accounts_the_validated_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let world = World::new();
        let pid = world
            .admit(
                OBSERVER_MANIFEST,
                make_spirit_obj(PanicsOnTelemetry),
                StartPolicy::StartNow,
            )
            .await
            .unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let occupied = tokio::task::spawn_blocking(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        entered_rx.await.unwrap();
        world
            .capability
            .set_scalar(
                EMITTER_PID,
                "emitter",
                "belief_variance",
                0.1,
                "before-swap",
            )
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while world.pump.stats().in_flight != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("telemetry dispatch never queued");
        let successor = Arc::new(Recorded::default());
        world.swap(pid, "observer", observer_with(&successor));
        release_tx.send(()).unwrap();
        occupied.await.unwrap();
        world.pump.flush().await.unwrap();
        assert_eq!(
            world.pump.stats(),
            stats(0, 0, 1),
            "the queued predecessor must panic, not invoke its successor"
        );
        assert!(successor.scalar_seqs().is_empty());
        let seq = world.emit(0.2).await;
        assert_eq!(successor.scalar_seqs(), [seq]);
        assert_eq!(
            world.pump.stats(),
            stats(1, 0, 1),
            "successor starts with clean accounting"
        );
        world.pump.shutdown().await;
    });
}
