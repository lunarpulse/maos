#![cfg(feature = "network")]

//! Story 17-4 — the host's two telemetry hook callers.
//!
//! [`HookPump`] is the first production caller of `fire_on_telemetry_event`.
//! It holds exactly one receiver on the root's broad `scalar.tap` channel
//! (`TelemetryStreamAdapter::subscribe_all`) and one delivery task drains it in
//! arrival order, so one emitting thread's writes reach a subscriber in that
//! thread's program order across tags. No per-topic forwarders, no reorder
//! buffer, no tag declarations.
//!
//! [`ConsolidationWatchdog`] is the first production caller of
//! `fire_on_consolidate`: every registered compiled-in Spirit, once per its own
//! idle window while it is `Running` and inbound-quiescent.
//!
//! ## Who receives telemetry
//!
//! `load_admit_start` registers a pid after the granted admission of a
//! compiled-in class and before `start`. Eligibility is decided at every
//! delivery from live state: the SCB is present, `Running`, and its CURRENT
//! manifest class is `observer`. A hot-swap that changes the class under a pid
//! therefore stops (or starts) delivery at the next event. The right to receive
//! comes from the operator admitting the compiled-in class; each grant and
//! revoke is a `governance:telemetry-subscription:*` row at the subscriber's
//! pid. The check and the dispatch are not atomic: a pause or unload in
//! between is FR51(a)'s known gap (E16-A6), shared with IdleWatchdog.
//!
//! ## Accounting
//!
//! Nothing is lost silently: lag is summed from `RecvError::Lagged(n)`, and
//! every registered pid an event did not reach is `ineligible`,
//! `skipped_manifest` or `failed`. A subscriber whose hook panics
//! [`QUARANTINE_AFTER_CONSECUTIVE_PANICS`] times in a row, or exceeds its
//! budget once, is quarantined for that runtime object — a hot-swapped
//! successor starts clean, `resume` does not lift it.
//!
//! ## Handles
//!
//! Both callers hold the scheduler's SCB map and dispatcher WEAKLY. The
//! dispatcher pins the capability registry and through it the audit channel,
//! so a strong handle would make every root's audit drain wait for its
//! timeout. Strong scheduler/dispatcher handles exist only during a dispatch,
//! bounded by its hook budget. Each registered runtime may also have a cached
//! dispatch-only SCB pinning its Spirit snapshot; unregister/shutdown releases
//! it. Hook callers join without a detaching timeout. Only the output-only
//! thread may detach after its bounded drain, with none of these handles.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;

use maos_domain::invariants::i7::ScalarTapEvent;
use maos_kernel_core::capability::cap_tokens::monotonic_now_ns;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::scheduler::control_block::{
    AnySpiritObj, ScbLifecycleState, ScbRuntimeSnapshot, SpiritControlBlock,
};
use maos_kernel_core::scheduler::{HookDispatcher, HookOutcome, SpiritSchedulerAdapter};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use observer::{ActivitySink, ObservedActivity};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// The class whose Spirits receive every `scalar.tap` write in the root.
pub const OBSERVER_CLASS: &str = "observer";
/// What a telemetry-subscription grant covers.
pub const SUBSCRIPTION_SCOPE: &str = "scalar.tap.*";
/// Intent of the grant row.
pub const SUBSCRIPTION_GRANTED: &str = "governance:telemetry-subscription:granted";
/// Intent of the revoke row.
pub const SUBSCRIPTION_REVOKED: &str = "governance:telemetry-subscription:revoked";
/// Consecutive hook panics after which a runtime object is quarantined.
pub const QUARANTINE_AFTER_CONSECUTIVE_PANICS: u32 = 3;
/// Records the activity sink queues ahead of the writer before it counts
/// overflow.
pub const ACTIVITY_QUEUE_CAPACITY: usize = 4096;

type ScbMap = RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>;

/// The pump's counters; `in_flight` is the number of dispatches running now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PumpStats {
    /// Dispatches that returned `Fired` or `BudgetWarning80`.
    pub delivered: u64,
    /// Events the broad receiver skipped (`RecvError::Lagged`).
    pub lagged_skipped: u64,
    /// Unread broadcast positions abandoned at the cancellation boundary,
    /// including lag not yet observed by the receiver.
    pub shutdown_skipped: u64,
    /// Event × registered pid pairs not dispatched: SCB absent, not
    /// `Running`, not Observer-class, or quarantined.
    pub ineligible: u64,
    /// Dispatches the manifest's `enabled_hooks` refused.
    pub skipped_manifest: u64,
    /// Dispatches that panicked, exceeded the budget or could not be encoded.
    pub failed: u64,
    /// Dispatches running at the time of the read.
    pub in_flight: u64,
}

/// The pump stopped before the request was served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PumpStopped;

impl std::fmt::Display for PumpStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the telemetry hook pump has stopped")
    }
}

impl std::error::Error for PumpStopped {}

/// One stderr line for a panicking hook. Every Spirit-controlled string is
/// `escape_debug`-ed, so no raw control or bidi character reaches a terminal.
pub fn hook_panic_line(
    hook: &str,
    pid: u32,
    spirit_id: &str,
    tag: Option<&str>,
    preview: &str,
) -> String {
    let on_tag = tag
        .map(|tag| format!(" on tag \"{}\"", tag.escape_debug()))
        .unwrap_or_default();
    format!(
        "maos: telemetry pump: {hook} panicked in subscriber pid {pid} (\"{}\"){on_tag}: \"{}\"",
        spirit_id.escape_debug(),
        preview.escape_debug()
    )
}

#[derive(Default)]
struct Counters {
    delivered: AtomicU64,
    lagged_skipped: AtomicU64,
    shutdown_skipped: AtomicU64,
    ineligible: AtomicU64,
    skipped_manifest: AtomicU64,
    failed: AtomicU64,
    in_flight: AtomicU64,
}

/// One registered compiled-in pid.
struct Subscriber {
    spirit_id: String,
    /// The runtime object the strikes and quarantine refer to. `Weak` keeps
    /// the allocation's address unique without keeping the Spirit alive.
    object: Option<Weak<dyn AnySpiritObj>>,
    /// Dispatch-only SCB, never installed in the scheduler. Its object cannot
    /// change under a queued blocking hook. Rebuilt only at runtime cutover.
    target: Option<Arc<SpiritControlBlock>>,
    strikes: u32,
    quarantined: bool,
    /// A grant row is outstanding for this pid.
    granted: bool,
    /// Monotonic ns of the last consolidation that was not `SkippedManifest`
    /// (seeded at registration).
    last_consolidate_ns: u64,
}

impl Subscriber {
    fn dispatch_target(
        &mut self,
        scb: &SpiritControlBlock,
        snapshot: ScbRuntimeSnapshot,
    ) -> Arc<SpiritControlBlock> {
        let target = self.target.get_or_insert_with(|| {
            Arc::new(SpiritControlBlock::new(
                scb.pid,
                scb.spirit_id.clone(),
                Default::default(),
                Arc::clone(&snapshot.spirit_obj),
                scb.boot_nonce,
            ))
        });
        // Move the already-read manifest into the view; no per-event SCB
        // allocation or second manifest copy. Eligibility uses the real SCB.
        target.replace_runtime(snapshot);
        Arc::clone(target)
    }
}

#[derive(Clone, Copy)]
enum Hook {
    Telemetry,
    Consolidate,
}

impl Hook {
    fn name(self) -> &'static str {
        match self {
            Hook::Telemetry => "on_telemetry_event",
            Hook::Consolidate => "on_consolidate",
        }
    }
}

struct Shared {
    scbs: Weak<ScbMap>,
    dispatcher: Weak<HookDispatcher>,
    tl: Arc<TransparencyLogAdapter>,
    registry: Mutex<BTreeMap<u32, Subscriber>>,
    counters: Counters,
    stopped: AtomicBool,
}

fn same_object(weak: &Weak<dyn AnySpiritObj>, live: &Arc<dyn AnySpiritObj>) -> bool {
    std::ptr::addr_eq(weak.as_ptr(), Arc::as_ptr(live))
}

fn class_of(snapshot: &ScbRuntimeSnapshot) -> &str {
    snapshot
        .manifest
        .class
        .as_ref()
        .map_or("", |class| class.name.as_str())
}

impl Shared {
    fn registry(&self) -> std::sync::MutexGuard<'_, BTreeMap<u32, Subscriber>> {
        self.registry
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn scb(&self, pid: u32) -> Option<Arc<SpiritControlBlock>> {
        let scbs = self.scbs.upgrade()?;
        let map = scbs.read().unwrap_or_else(|error| error.into_inner());
        map.get(&pid).cloned()
    }

    fn subscription_row(
        &self,
        pid: u32,
        spirit_id: &str,
        class: &str,
        granted: bool,
        reason: &str,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let payload = maos_domain::governance::GovernanceEventPayload {
            recorded_at_ns: now,
            effective_at_ns: now,
            event: maos_domain::governance::GovernanceEventKind::TelemetrySubscription(
                maos_domain::governance::TelemetrySubscriptionPayload {
                    subscriber_pid: pid,
                    spirit_id: spirit_id.to_owned(),
                    class: class.to_owned(),
                    scope: SUBSCRIPTION_SCOPE.to_owned(),
                    granted,
                    reason: reason.to_owned(),
                },
            ),
        };
        let Ok(bytes) = serde_json::to_vec(&payload) else {
            return;
        };
        let intent = if granted {
            SUBSCRIPTION_GRANTED
        } else {
            SUBSCRIPTION_REVOKED
        };
        let _ = self.tl.insert_frame_event(
            maos_kernel_core::iac::transparency_log::FrameKind::GovernanceEvent,
            pid,
            None,
            intent,
            &bytes,
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
    }

    /// Bring one entry in line with the pid's live runtime: a new object
    /// starts with no strikes and no quarantine, and the grant follows the
    /// current class.
    fn sync(&self, pid: u32, entry: &mut Subscriber, snapshot: &ScbRuntimeSnapshot) {
        if !entry
            .object
            .as_ref()
            .is_some_and(|weak| same_object(weak, &snapshot.spirit_obj))
        {
            entry.object = Some(Arc::downgrade(&snapshot.spirit_obj));
            entry.target = None;
            entry.strikes = 0;
            entry.quarantined = false;
        }
        let class = class_of(snapshot);
        let eligible = class == OBSERVER_CLASS && !entry.quarantined;
        if eligible && !entry.granted {
            entry.granted = true;
            self.subscription_row(pid, &entry.spirit_id, class, true, "eligible");
        } else if !eligible && entry.granted {
            entry.granted = false;
            self.subscription_row(pid, &entry.spirit_id, class, false, "class-changed");
        }
    }

    fn unregister(&self, pid: u32, reason: &str) {
        let Some(entry) = self.registry().remove(&pid) else {
            return;
        };
        if entry.granted {
            self.subscription_row(pid, &entry.spirit_id, OBSERVER_CLASS, false, reason);
        }
    }

    fn quarantine(&self, pid: u32, entry: &mut Subscriber, class: &str, why: &str) {
        entry.quarantined = true;
        eprintln!(
            "maos: telemetry pump: quarantined subscriber pid {pid} (\"{}\"): {why}",
            entry.spirit_id.escape_debug()
        );
        if entry.granted {
            entry.granted = false;
            self.subscription_row(
                pid,
                &entry.spirit_id,
                class,
                false,
                &format!("quarantined: {why}"),
            );
        }
    }

    /// Strikes and quarantine for one outcome, against the object that was
    /// dispatched to (a successor swapped in meanwhile is not charged).
    fn account(
        &self,
        pid: u32,
        object: &Arc<dyn AnySpiritObj>,
        // Only Observer grants can be revoked by quarantine.
        hook: Hook,
        tag: Option<&str>,
        outcome: &HookOutcome,
    ) {
        let mut registry = self.registry();
        let Some(entry) = registry.get_mut(&pid) else {
            return;
        };
        if !entry
            .object
            .as_ref()
            .is_some_and(|weak| same_object(weak, object))
        {
            return;
        }
        match outcome {
            HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { .. } => entry.strikes = 0,
            HookOutcome::Panicked {
                panic_payload_preview,
            } => {
                eprintln!(
                    "{}",
                    hook_panic_line(
                        hook.name(),
                        pid,
                        &entry.spirit_id,
                        tag,
                        panic_payload_preview
                    )
                );
                entry.strikes += 1;
                if entry.strikes >= QUARANTINE_AFTER_CONSECUTIVE_PANICS {
                    let why = format!("{} panicked {} times in a row", hook.name(), entry.strikes);
                    self.quarantine(pid, entry, OBSERVER_CLASS, &why);
                }
            }
            HookOutcome::BudgetExceeded { cap_seconds, .. } => {
                let why = format!("{} exceeded its {cap_seconds} s budget", hook.name());
                self.quarantine(pid, entry, OBSERVER_CLASS, &why);
            }
            HookOutcome::SkippedManifest | HookOutcome::DeferredToNextStory => {}
        }
    }

    async fn deliver(&self, event: &ScalarTapEvent) {
        let pids: Vec<u32> = self.registry().keys().copied().collect();
        if pids.is_empty() {
            return;
        }
        let payload = match serde_json::to_vec(event) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("maos: telemetry pump: event encode failed: {error}");
                self.counters
                    .failed
                    .fetch_add(pids.len() as u64, Ordering::Relaxed);
                return;
            }
        };
        for pid in pids {
            let Some(scb) = self.scb(pid) else {
                self.unregister(pid, "unloaded");
                self.counters.ineligible.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let snapshot = scb.runtime_snapshot();
            let object = Arc::clone(&snapshot.spirit_obj);
            let target = {
                let mut registry = self.registry();
                let Some(entry) = registry.get_mut(&pid) else {
                    continue;
                };
                self.sync(pid, entry, &snapshot);
                (entry.granted && scb.current_state() == ScbLifecycleState::Running)
                    .then(|| entry.dispatch_target(&scb, snapshot))
            };
            let Some(target) = target else {
                self.counters.ineligible.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let Some(dispatcher) = self.dispatcher.upgrade() else {
                self.counters.ineligible.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            self.counters.in_flight.fetch_add(1, Ordering::Relaxed);
            let outcome = dispatcher.fire_on_telemetry_event(&target, &payload).await;
            self.counters.in_flight.fetch_sub(1, Ordering::Relaxed);
            let counter = match &outcome {
                HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { .. } => {
                    &self.counters.delivered
                }
                HookOutcome::SkippedManifest => &self.counters.skipped_manifest,
                _ => &self.counters.failed,
            };
            counter.fetch_add(1, Ordering::Relaxed);
            self.account(pid, &object, Hook::Telemetry, Some(&event.tag), &outcome);
        }
    }

    fn lagged(&self, skipped: u64) {
        self.counters
            .lagged_skipped
            .fetch_add(skipped, Ordering::Relaxed);
        eprintln!("maos: telemetry pump lagged: {skipped} event(s) skipped");
    }
}

/// The telemetry pump of one composition root. See the module docs.
pub struct HookPump {
    shared: Arc<Shared>,
    flush_tx: mpsc::UnboundedSender<oneshot::Sender<()>>,
    cancel: CancellationToken,
    task: Mutex<Option<JoinHandle<()>>>,
    writer: ActivityWriter,
}

impl HookPump {
    /// Build the pump over the root's OWN telemetry adapter (never a second
    /// one) and take its one broad receiver. Call after the scheduler's
    /// exclusive wiring and before any root admission; Observer activity is
    /// written to stdout.
    pub fn new(
        scheduler: &SpiritSchedulerAdapter,
        telemetry: &TelemetryStreamAdapter,
        tl: Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self::with_writer(scheduler, telemetry, tl, ActivityWriter::stdout())
    }

    /// As [`HookPump::new`], with the activity writer supplied.
    pub fn with_writer(
        scheduler: &SpiritSchedulerAdapter,
        telemetry: &TelemetryStreamAdapter,
        tl: Arc<TransparencyLogAdapter>,
        writer: ActivityWriter,
    ) -> Self {
        let shared = Arc::new(Shared {
            scbs: Arc::downgrade(&scheduler.scbs()),
            dispatcher: Arc::downgrade(&scheduler.dispatcher_arc()),
            tl,
            registry: Mutex::new(BTreeMap::new()),
            counters: Counters::default(),
            stopped: AtomicBool::new(false),
        });
        let (flush_tx, flush_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let task = tokio::spawn(delivery_loop(
            Arc::clone(&shared),
            telemetry.subscribe_all(),
            flush_rx,
            cancel.clone(),
        ));
        Self {
            shared,
            flush_tx,
            cancel,
            task: Mutex::new(Some(task)),
            writer,
        }
    }

    /// The sink every compiled-in Observer is constructed with.
    pub fn activity_sink(&self) -> Arc<dyn ActivitySink> {
        self.writer.sink()
    }

    /// Register an admitted compiled-in pid (`load_admit_start`, after the
    /// granted admission and before `start`). An Observer-class pid gets its
    /// grant row now.
    pub fn register(&self, pid: u32, spirit_id: &str) {
        if self.shared.stopped.load(Ordering::Acquire) {
            return;
        }
        let mut entry = Subscriber {
            spirit_id: spirit_id.to_owned(),
            object: None,
            target: None,
            strikes: 0,
            quarantined: false,
            granted: false,
            last_consolidate_ns: monotonic_now_ns(),
        };
        let snapshot = self.shared.scb(pid).map(|scb| scb.runtime_snapshot());
        if let Some(snapshot) = &snapshot {
            entry.object = Some(Arc::downgrade(&snapshot.spirit_obj));
            if class_of(snapshot) == OBSERVER_CLASS {
                entry.granted = true;
                self.shared
                    .subscription_row(pid, spirit_id, OBSERVER_CLASS, true, "admitted");
            }
        }
        self.shared.registry().insert(pid, entry);
    }

    /// Withdraw a registration (a failed `start` rolls back here). An
    /// outstanding grant is revoked with `reason`.
    pub fn unregister(&self, pid: u32, reason: &str) {
        self.shared.unregister(pid, reason);
    }

    /// The counters, with `in_flight` read now.
    pub fn stats(&self) -> PumpStats {
        let c = &self.shared.counters;
        PumpStats {
            delivered: c.delivered.load(Ordering::Relaxed),
            lagged_skipped: c.lagged_skipped.load(Ordering::Relaxed),
            shutdown_skipped: c.shutdown_skipped.load(Ordering::Relaxed),
            ineligible: c.ineligible.load(Ordering::Relaxed),
            skipped_manifest: c.skipped_manifest.load(Ordering::Relaxed),
            failed: c.failed.load(Ordering::Relaxed),
            in_flight: c.in_flight.load(Ordering::Relaxed),
        }
    }

    /// Output accounting, including whether the output-only thread detached.
    pub fn activity_stats(&self) -> WriterStats {
        self.writer.stats()
    }

    /// Resolve once every event published before this call has been
    /// dispatched or accounted. The barrier is the delivery task's own
    /// acknowledgement, never a reading of the counters.
    pub async fn flush(&self) -> Result<(), PumpStopped> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.flush_tx.send(ack_tx).map_err(|_| PumpStopped)?;
        ack_rx.await.map_err(|_| PumpStopped)
    }

    /// Stop delivering: cancel, join the delivery task without a detaching
    /// timeout (an in-flight dispatch is bounded by its budget), revoke every
    /// outstanding grant (`root-shutdown`), then close the activity writer.
    /// Only that output-only thread may detach after its bounded drain.
    /// Idempotent.
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        let task = self
            .task
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(task) = task {
            if let Err(error) = task.await {
                eprintln!("maos: telemetry pump task failed during shutdown: {error}");
            }
        }
        if !self.shared.stopped.swap(true, Ordering::AcqRel) {
            let pids: Vec<u32> = self.shared.registry().keys().copied().collect();
            for pid in pids {
                self.shared.unregister(pid, "root-shutdown");
            }
        }
        self.writer.close().await;
    }
}

async fn delivery_loop(
    shared: Arc<Shared>,
    mut rx: broadcast::Receiver<ScalarTapEvent>,
    mut flush_rx: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    cancel: CancellationToken,
) {
    let mut closed = false;
    'delivery: loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            Some(ack) = flush_rx.recv() => {
                // Snapshot a finite receive boundary. Lag consumes positions
                // too; publications after this boundary never extend it.
                let mut remaining = rx.len() as u64;
                while remaining != 0 {
                    if cancel.is_cancelled() {
                        break 'delivery;
                    }
                    match rx.try_recv() {
                        Ok(event) => {
                            shared.deliver(&event).await;
                            remaining -= 1;
                        }
                        Err(TryRecvError::Lagged(skipped)) => {
                            shared.lagged(skipped);
                            remaining = remaining.saturating_sub(skipped);
                        }
                        Err(TryRecvError::Empty | TryRecvError::Closed) => break,
                    }
                }
                let _ = ack.send(());
            }
            received = rx.recv(), if !closed => match received {
                Ok(event) => shared.deliver(&event).await,
                Err(RecvError::Lagged(skipped)) => shared.lagged(skipped),
                Err(RecvError::Closed) => closed = true,
            },
        }
    }
    shared
        .counters
        .shutdown_skipped
        .fetch_add(rx.len() as u64, Ordering::Relaxed);
}

/// How often [`ConsolidationWatchdog`] polls and how idle windows scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsolidationCadence {
    /// Poll interval.
    pub poll: Duration,
    /// Each Spirit's `[scheduling].idle_window_ms` is divided by this.
    pub window_divisor: u64,
}

impl ConsolidationCadence {
    /// IdleWatchdog's cadence: 3 000 ms, or 40 ms with windows ÷100 under
    /// `MAOS_IDLE_FAST`.
    pub fn from_env() -> Self {
        if std::env::var_os("MAOS_IDLE_FAST").is_some() {
            Self {
                poll: Duration::from_millis(40),
                window_divisor: 100,
            }
        } else {
            Self {
                poll: Duration::from_millis(3_000),
                window_divisor: 1,
            }
        }
    }
}

/// Fires `on_consolidate` (empty payload: *curate now*) for every registered
/// compiled-in pid that is `Running`, has had no inbound frame for one idle
/// window, and has not consolidated for one idle window. Quarantine is the
/// pump's, shared.
pub struct ConsolidationWatchdog {
    cancel: CancellationToken,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl ConsolidationWatchdog {
    /// Spawn over `pump`'s registrations.
    pub fn spawn(pump: &HookPump, cadence: ConsolidationCadence) -> Self {
        let cancel = CancellationToken::new();
        let shared = Arc::clone(&pump.shared);
        let token = cancel.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(cadence.poll);
            loop {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => break,
                    _ = interval.tick() => consolidate_once(&shared, cadence).await,
                }
            }
        });
        Self {
            cancel,
            task: Mutex::new(Some(task)),
        }
    }

    /// Cancel and join without a detaching timeout: an in-flight
    /// consolidation finishes (its budget bounds it) before this returns.
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        let task = self
            .task
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(task) = task {
            if let Err(error) = task.await {
                eprintln!("maos: ConsolidationWatchdog task failed during shutdown: {error}");
            }
        }
    }
}

async fn consolidate_once(shared: &Shared, cadence: ConsolidationCadence) {
    let pids: Vec<u32> = shared.registry().keys().copied().collect();
    for pid in pids {
        let Some(scb) = shared.scb(pid) else {
            shared.unregister(pid, "unloaded");
            continue;
        };
        if scb.current_state() != ScbLifecycleState::Running {
            continue;
        }
        let snapshot = scb.runtime_snapshot();
        let window_ns = (u64::from(snapshot.manifest.scheduling.idle_window_ms)
            / cadence.window_divisor.max(1))
        .saturating_mul(1_000_000);
        let now = monotonic_now_ns();
        let last_inbound = scb.last_inbound_frame_ns.load(Ordering::Relaxed);
        if now.saturating_sub(last_inbound) < window_ns {
            continue;
        }
        let object = Arc::clone(&snapshot.spirit_obj);
        let target = {
            let mut registry = shared.registry();
            let Some(entry) = registry.get_mut(&pid) else {
                continue;
            };
            shared.sync(pid, entry, &snapshot);
            if entry.quarantined || now.saturating_sub(entry.last_consolidate_ns) < window_ns {
                continue;
            }
            entry.dispatch_target(&scb, snapshot)
        };
        let Some(dispatcher) = shared.dispatcher.upgrade() else {
            return;
        };
        let outcome = dispatcher.fire_on_consolidate(&target, &[]).await;
        drop(dispatcher);
        if !matches!(outcome, HookOutcome::SkippedManifest) {
            if let Some(entry) = shared.registry().get_mut(&pid) {
                if entry
                    .object
                    .as_ref()
                    .is_some_and(|weak| same_object(weak, &object))
                {
                    entry.last_consolidate_ns = monotonic_now_ns();
                }
            }
        }
        shared.account(pid, &object, Hook::Consolidate, None, &outcome);
    }
}

/// What the activity writer has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WriterStats {
    pub written: u64,
    pub overflowed: u64,
    pub write_errors: u64,
    /// Records still unconfirmed when a blocked writer was detached. An
    /// in-flight write may finish later; this is not a claim of definite loss.
    pub shutdown_abandoned: u64,
    pub detached: bool,
}

#[derive(Default)]
struct WriterCounters {
    written: AtomicU64,
    overflowed: AtomicU64,
    write_errors: AtomicU64,
    pending: AtomicU64,
    shutdown_abandoned: AtomicU64,
    detached: AtomicBool,
}

/// One bounded output queue. Only its writer performs I/O, including diagnostics.
pub struct ActivityWriter {
    tx: mpsc::Sender<ObservedActivity>,
    counters: Arc<WriterCounters>,
    stopping: Arc<AtomicBool>,
    abandon: Arc<AtomicBool>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

#[derive(Clone)]
struct QueueSink {
    tx: mpsc::Sender<ObservedActivity>,
    counters: Arc<WriterCounters>,
    stopping: Arc<AtomicBool>,
}

impl std::fmt::Debug for QueueSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueSink").finish_non_exhaustive()
    }
}

impl ActivitySink for QueueSink {
    fn emit(&self, activity: ObservedActivity) {
        if self.stopping.load(Ordering::Acquire) {
            self.counters.overflowed.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.counters.pending.fetch_add(1, Ordering::Relaxed);
        if self.tx.try_send(activity).is_err() {
            self.counters.pending.fetch_sub(1, Ordering::Relaxed);
            self.counters.overflowed.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Operator-ratified review amendment: only the output thread may detach.
/// It owns records and output handles, never scheduler, Spirit or audit handles.
const ACTIVITY_DRAIN_GRACE: Duration = Duration::from_millis(250);

impl ActivityWriter {
    /// Write each complete JSON line under the ordinary stdout lock. Never
    /// mutate shared descriptor flags, including when stdout/stderr alias.
    pub fn stdout() -> Self {
        Self::start(
            |line| {
                let stdout = std::io::stdout();
                let mut output = stdout.lock();
                output.write_all(line)?;
                output.flush()
            },
            ACTIVITY_QUEUE_CAPACITY,
        )
    }

    /// Inject an output. Blocking is confined to the output-only thread.
    pub fn spawn(mut out: Box<dyn Write + Send>, capacity: usize) -> Self {
        Self::start(
            move |line| {
                out.write_all(line)?;
                out.flush()
            },
            capacity,
        )
    }

    fn start(
        mut output: impl FnMut(&[u8]) -> std::io::Result<()> + Send + 'static,
        capacity: usize,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<ObservedActivity>(capacity);
        let counters = Arc::new(WriterCounters::default());
        let thread_counters = Arc::clone(&counters);
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopping);
        let abandon = Arc::new(AtomicBool::new(false));
        let abandoned = Arc::clone(&abandon);
        let thread = std::thread::Builder::new()
            .name("maos-observer-activity".into())
            .spawn(move || {
                let mut overflow_reported = false;
                let mut error_reported = false;
                while !abandoned.load(Ordering::Acquire) {
                    if stop.load(Ordering::Acquire) {
                        rx.close();
                    }
                    if !overflow_reported
                        && thread_counters.overflowed.load(Ordering::Relaxed) != 0
                    {
                        overflow_reported = true;
                        eprintln!("maos: observer activity queue full or closed: records are being dropped and counted");
                    }
                    if !error_reported
                        && thread_counters.write_errors.load(Ordering::Relaxed) != 0
                    {
                        error_reported = true;
                        eprintln!("maos: observer activity write failed; failures are counted");
                    }
                    match rx.try_recv() {
                        Ok(activity) => {
                            let result = serde_json::to_vec(&activity)
                                .map_err(std::io::Error::other)
                                .and_then(|mut line| {
                                    line.push(b'\n');
                                    output(&line)
                                });
                            let counter = match result {
                                Ok(()) => &thread_counters.written,
                                Err(_) => &thread_counters.write_errors,
                            };
                            counter.fetch_add(1, Ordering::Relaxed);
                            thread_counters.pending.fetch_sub(1, Ordering::Relaxed);
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => break,
                        Err(mpsc::error::TryRecvError::Empty) => {
                            std::thread::park_timeout(Duration::from_millis(10));
                        }
                    }
                }
            })
            .expect("spawn the observer activity writer thread");
        Self {
            tx,
            counters,
            stopping,
            abandon,
            thread: Mutex::new(Some(thread)),
        }
    }

    pub fn sink(&self) -> Arc<dyn ActivitySink> {
        Arc::new(QueueSink {
            tx: self.tx.clone(),
            counters: Arc::clone(&self.counters),
            stopping: Arc::clone(&self.stopping),
        })
    }

    pub fn stats(&self) -> WriterStats {
        WriterStats {
            written: self.counters.written.load(Ordering::Relaxed),
            overflowed: self.counters.overflowed.load(Ordering::Relaxed),
            write_errors: self.counters.write_errors.load(Ordering::Relaxed),
            shutdown_abandoned: self.counters.shutdown_abandoned.load(Ordering::Relaxed),
            detached: self.counters.detached.load(Ordering::Relaxed),
        }
    }

    /// Stop accepting records and allow a bounded output-only drain. Join a
    /// finished thread; detach a stalled one without blocking root teardown.
    /// Telemetry and consolidation dispatch tasks are always joined separately.
    pub async fn close(&self) {
        self.stopping.store(true, Ordering::Release);
        let thread = self
            .thread
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(thread) = thread {
            thread.thread().unpark();
            let deadline = tokio::time::Instant::now() + ACTIVITY_DRAIN_GRACE;
            while !thread.is_finished() && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            if thread.is_finished() {
                if thread.join().is_err() {
                    self.counters.write_errors.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                self.abandon.store(true, Ordering::Release);
                self.counters.shutdown_abandoned.store(
                    self.counters.pending.load(Ordering::Relaxed),
                    Ordering::Relaxed,
                );
                self.counters.detached.store(true, Ordering::Release);
                // No join task enters Tokio's blocking pool: runtime shutdown
                // must not inherit the same unbounded wait under another name.
            }
        }
    }
}
