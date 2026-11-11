//! Spawned-form lifecycle triggers and the state-preserving swap (Story 20-9).
//!
//! A `WorkerSpirit` hook sends one [`LifecycleRequest`] through its [`LifecycleLink`], a bounded
//! slot its session drains one export at a time. The pre-warmed successor of a swap is the
//! exception: it has no session yet, so its `migrate` and `swap-in` run inline on its own child,
//! and a committed `swap-in` hands that child to the predecessor's session.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::Scope;
use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
    BridgePoll, FrameWriter, SpawnedBridge, SubStream,
};
use maos_kernel_core::security::sandbox::SandboxSpec;
use maos_wasm_host::lifecycle_wire::{self as wire, LifecycleReply, LifecycleRequest};

use crate::invocation::{self, Invocation};
use crate::supervision::BindingCore;

/// What a hook learns when its session did not answer.
pub const SESSION_ENDED: &str = "session_ended";
pub const DEADLINE: &str = "deadline";
/// A trigger that found its session busy for half its budget: dropped, never a strike.
pub const BUSY: &str = "busy";

/// The longest one swap attempt may spend before its successor is adopted: launch, `Ready`, the
/// probe and every exchange of the swap (`migrate`, `swap-in`) share it, whatever budgets the
/// successor's manifest declares (up to a day). A successor that stays silent fences the
/// predecessor and holds the door's upgrade lock for this long per attempt, not per stage.
pub const ATTEMPT_WALL: Duration = Duration::from_secs(90);

/// Slack a pump trigger's link deadline leaves past the pump's wall, so the pump accounts an
/// overrun as the overrun it is before the link gives up on the export.
const WALL_SLACK: Duration = Duration::from_secs(2);

pub enum Answer {
    Reply(LifecycleReply),
    SessionEnded,
}

/// How far one queued export got. A hook that gives up and the session that answers race for
/// a started call and exactly one wins, so a snapshot answered after its hook's deadline can
/// never re-fence a predecessor the failed swap already released. A snapshot also remembers the
/// swap attempt it was taken for, and a pump dispatch its record, so the session can say which
/// swap a reply belongs to and when the export really started.
pub struct CallState {
    phase: AtomicU8,
    owner: Option<Arc<AtomicBool>>,
    invocation: Option<Arc<Invocation>>,
}

impl CallState {
    const QUEUED: u8 = 0;
    const STARTED: u8 = 1;
    const CANCELLED: u8 = 2;
    const ANSWERED: u8 = 3;

    fn new(owner: Option<Arc<AtomicBool>>, invocation: Option<Arc<Invocation>>) -> Arc<Self> {
        Arc::new(Self {
            phase: AtomicU8::new(Self::QUEUED),
            owner,
            invocation,
        })
    }

    fn advance(&self, from: u8, to: u8) -> bool {
        self.phase
            .compare_exchange(from, to, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Session: the call goes to the guest. `false`: its hook gave up while it was queued.
    pub fn start(&self) -> bool {
        let started = self.advance(Self::QUEUED, Self::STARTED);
        if let (true, Some(invocation)) = (started, &self.invocation) {
            invocation.started();
        }
        started
    }

    /// Session: claim the right to deliver the reply. `false`: its hook stopped waiting, and
    /// the reply must change nothing.
    pub fn answer(&self) -> bool {
        self.advance(Self::STARTED, Self::ANSWERED)
    }

    /// The swap attempt a snapshot was taken for.
    pub fn owner(&self) -> Option<&Arc<AtomicBool>> {
        self.owner.as_ref()
    }

    fn cancel_queued(&self) -> bool {
        let dropped = self.advance(Self::QUEUED, Self::CANCELLED);
        if let (true, Some(invocation)) = (dropped, &self.invocation) {
            invocation.dropped();
        }
        dropped
    }

    /// Hook: stop waiting. `false`: the session already claimed the reply, which is on its way.
    fn cancel(&self) -> bool {
        self.cancel_queued() || self.advance(Self::STARTED, Self::CANCELLED)
    }
}

/// One queued export.
pub struct Call {
    pub request: LifecycleRequest,
    pub answer: SyncSender<Answer>,
    pub state: Arc<CallState>,
}

/// What a successor launches under: the predecessor's admitted sandbox (never widened by a
/// swap), and the directory its manifest's relative `artifact` resolves against.
#[derive(Clone)]
pub struct SwapContext {
    pub sandbox: SandboxSpec,
    pub origin: std::path::PathBuf,
}

impl SwapContext {
    pub fn declared(&self) -> &[Scope] {
        &self.sandbox.declared_scopes
    }
}

/// A launched, `Ready`, lifecycle-probed successor child.
pub struct Prewarm {
    pub bridge: SpawnedBridge,
    pub writer: FrameWriter,
    pub turn_budget: Duration,
    pub context_window: maos_kernel_core::capability::context_budget::ContextWindow,
    pub scopes: Vec<Scope>,
    /// Where this swap attempt must be over, however long its stages' budgets: set when the
    /// attempt began, shared by the pre-warm and every exchange of the swap.
    pub deadline: Instant,
}

/// A committed successor: the predecessor's session adopts it.
pub struct Handover {
    pub core: Arc<BindingCore>,
    pub prewarm: Prewarm,
}

impl Prewarm {
    /// The next stdout frame of a child nobody else is serving, never past `deadline`. Stderr
    /// is drained and dropped, but every line is a turn of this loop, so a guest that never
    /// stops writing diagnostics cannot outlast its cap; a frame that arrives late is refused.
    fn next_stdout(&mut self, deadline: Instant) -> Result<Vec<u8>, String> {
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(DEADLINE.into());
            }
            match self
                .bridge
                .poll_frame(left.min(Duration::from_millis(20)))
                .map_err(|error| format!("child_poll: {error}"))?
            {
                BridgePoll::Idle | BridgePoll::Data(SubStream::Stderr, _) => {}
                BridgePoll::Closed => return Err("child_closed".into()),
                BridgePoll::Data(SubStream::Stdout, bytes) => {
                    return if Instant::now() > deadline {
                        Err(DEADLINE.into())
                    } else {
                        Ok(bytes)
                    };
                }
            }
        }
    }

    /// When a stage allowed `cap` must end: no later than this attempt's one deadline.
    fn until(&self, cap: Duration) -> Instant {
        (Instant::now() + cap).min(self.deadline)
    }

    /// One request/reply on a child nobody else is serving.
    pub fn exchange(
        &mut self,
        request: &LifecycleRequest,
        cap: Duration,
    ) -> Result<LifecycleReply, String> {
        let deadline = self.until(cap);
        let body = wire::encode_request(request).map_err(|error| format!("encode: {error}"))?;
        self.writer
            .write_frame(&body)
            .map_err(|error| format!("child_write: {error}"))?;
        let bytes = self.next_stdout(deadline)?;
        if !wire::is_reply(&bytes) {
            return Err("child_protocol".into());
        }
        wire::decode_reply(&bytes).map_err(|error| format!("decode: {error}"))
    }

    /// Wait for the runner's empty `Ready` control frame.
    pub fn await_ready(&mut self, cap: Duration) -> Result<(), String> {
        let bytes = self.next_stdout(self.until(cap))?;
        if bytes.is_empty() {
            Ok(())
        } else {
            Err("child_protocol".into())
        }
    }
}

/// Ends the successor's declaration attempt: `true` at commit, `false` when it is dropped first.
type Outcome = Box<dyn FnOnce(bool) + Send>;

pub struct LifecycleLink {
    tx: SyncSender<Call>,
    rx: Mutex<Receiver<Call>>,
    cap: Mutex<Duration>,
    supported: Mutex<Option<bool>>,
    probed: Condvar,
    closed: AtomicBool,
    attached: AtomicBool,
    /// A committed successor's child is waiting for the predecessor's session to adopt it.
    awaiting: AtomicBool,
    fenced: AtomicBool,
    /// The live swap attempt this link (as predecessor) is being replaced in. A snapshot fences
    /// the predecessor only while its attempt is live, and the successor's drop ends the attempt
    /// and unfences under this same lock, so a reply that arrives after a refused swap fences
    /// nothing.
    attempt: Mutex<Option<Arc<AtomicBool>>>,
    /// On a successor's link: the attempt it began on its predecessor, which only it releases.
    own: Mutex<Option<Arc<AtomicBool>>>,
    swap: Mutex<Option<SwapContext>>,
    prewarm: Mutex<Option<Prewarm>>,
    predecessor: Mutex<Option<Arc<LifecycleLink>>>,
    handover: Mutex<Option<Handover>>,
    outcome: Mutex<Option<Outcome>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

impl LifecycleLink {
    pub fn new() -> Arc<Self> {
        let (tx, rx) = sync_channel(1);
        Arc::new(Self {
            tx,
            rx: Mutex::new(rx),
            cap: Mutex::new(Duration::from_secs(30)),
            supported: Mutex::new(None),
            probed: Condvar::new(),
            closed: AtomicBool::new(false),
            attached: AtomicBool::new(false),
            awaiting: AtomicBool::new(false),
            fenced: AtomicBool::new(false),
            attempt: Mutex::new(None),
            own: Mutex::new(None),
            swap: Mutex::new(None),
            prewarm: Mutex::new(None),
            predecessor: Mutex::new(None),
            handover: Mutex::new(None),
            outcome: Mutex::new(None),
        })
    }

    /// The longest a hook waits: the Spirit's `[budget].time_cap_seconds`, the dispatcher's cap.
    pub fn set_cap(&self, cap: Duration) {
        *locked(&self.cap) = cap;
    }

    /// A session will drain this link (a spawned core, before that session has started): its
    /// triggers queue for it rather than vanish as no-ops. The cap, not the queue, bounds them.
    pub fn expect_session(&self) {
        self.awaiting.store(true, Ordering::Release);
    }

    /// A session now drains this link: triggers reach the guest, never past `cap`.
    pub fn attach_session(&self, cap: Duration) {
        self.set_cap(cap);
        self.attached.store(true, Ordering::Release);
    }

    // ── hook side ───────────────────────────────────────────────────────────────────────

    /// Run one export and wait for its reply, never past the cap. `core` is the caller's
    /// core: a pre-warmed successor's committed `swap-in` hands that core to its predecessor.
    pub fn call(
        &self,
        core: &Arc<BindingCore>,
        request: LifecycleRequest,
    ) -> Result<LifecycleReply, String> {
        let cap = *locked(&self.cap);
        let inline = {
            let mut prewarm = locked(&self.prewarm);
            prewarm
                .as_mut()
                .map(|prewarm| prewarm.exchange(&request, cap))
        };
        if let Some(reply) = inline {
            if matches!(
                reply,
                Err(_) | Ok(LifecycleReply::Declined(_) | LifecycleReply::Failed(_))
            ) {
                // A successor that failed or refused an exchange is not served again: its child
                // dies and its attempt ends now, not when the coordinator's compensation (or a
                // pin on the object) finally drops this link.
                drop(locked(&self.prewarm).take());
                self.release_predecessor();
            }
            let reply = reply?;
            if matches!(request, LifecycleRequest::SwapIn { .. }) && reply == LifecycleReply::Done {
                self.commit(core);
            }
            return Ok(reply);
        }
        // A core no session drives (a `[cli_wrapper]` Worker, or a spawned core before its
        // session starts) has nothing to run: its triggers are no-ops, as they were. A
        // committed successor awaiting adoption is not that: its hooks queue for its session.
        let driven = self.attached.load(Ordering::Acquire) || self.awaiting.load(Ordering::Acquire);
        if !driven || *locked(&self.supported) == Some(false) {
            return Ok(LifecycleReply::Unsupported);
        }
        // A trigger that cannot even start in half its budget is dropped without a strike:
        // the budget it missed is the busy session's, and the hook that holds it is charged
        // for that. A telemetry or consolidate trigger gives up inside the pump's own wall for
        // it, so the pump never charges a subscriber for time it spent queued. A snapshot waits
        // behind the active turn for the whole cap.
        let limit = match request {
            LifecycleRequest::Telemetry { .. } => Some(crate::hook_pump::TELEMETRY_WALL),
            LifecycleRequest::Consolidate { .. } => Some(crate::hook_pump::CONSOLIDATE_WALL),
            _ => None,
        };
        let queue_share = match (&request, limit) {
            (_, Some(limit)) => cap.min(limit) / 2,
            (LifecycleRequest::Snapshot, None) => cap,
            _ => cap / 2,
        };
        // A pump trigger is also cut loose at its wall (plus slack for the pump to account the
        // overrun first): its hook is quarantined there, so it must not outlive the wall by
        // the manifest's whole budget, which a shutdown would then have to wait out.
        let to_deadline = limit.map_or(cap, |limit| cap.min(queue_share + limit + WALL_SLACK));
        let pid = core.spirit_pid();
        let (owner, invocation) = match &request {
            LifecycleRequest::Snapshot => (locked(&self.attempt).clone(), None),
            LifecycleRequest::Telemetry { seq, .. } => {
                (None, invocation::claim(pid, invocation::TELEMETRY, *seq))
            }
            LifecycleRequest::Consolidate { batch } => (
                None,
                invocation::claim(pid, invocation::CONSOLIDATE, invocation::digest(&batch.0)),
            ),
            _ => (None, None),
        };
        if let Some(invocation) = &invocation {
            invocation.queued();
        }
        let began = Instant::now();
        let (deadline, queue_deadline) = (began + to_deadline, began + queue_share);
        let (answer, reply) = sync_channel(1);
        let state = CallState::new(owner, invocation);
        let mut call = Call {
            request,
            answer,
            state: Arc::clone(&state),
        };
        loop {
            if self.closed.load(Ordering::Acquire) {
                return Err(SESSION_ENDED.into());
            }
            match self.tx.try_send(call) {
                Ok(()) => break,
                Err(TrySendError::Full(back)) if Instant::now() < queue_deadline => {
                    call = back;
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TrySendError::Full(_)) => {
                    state.cancel_queued();
                    return Err(BUSY.into());
                }
                Err(TrySendError::Disconnected(_)) => return Err(SESSION_ENDED.into()),
            }
        }
        loop {
            match reply.recv_timeout(Duration::from_millis(20)) {
                Ok(Answer::Reply(reply)) => return Ok(reply),
                Ok(Answer::SessionEnded) => return Err(SESSION_ENDED.into()),
                // A session that ended after this call queued (its `close` drained first) never
                // answers: fail now, not at the cap.
                Err(RecvTimeoutError::Disconnected) => return Err(SESSION_ENDED.into()),
                Err(RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    if self.closed.load(Ordering::Acquire) && state.cancel() {
                        return Err(SESSION_ENDED.into());
                    }
                    if now >= queue_deadline && state.cancel_queued() {
                        return Err(BUSY.into());
                    }
                    if now >= deadline && state.cancel() {
                        return Err(DEADLINE.into());
                    }
                }
            }
        }
    }

    /// The successor's `swap-in` succeeded: its child now belongs to the predecessor's session.
    /// Until that session adopts it, this link's hooks queue for it rather than vanish.
    fn commit(&self, core: &Arc<BindingCore>) {
        let Some(predecessor) = locked(&self.predecessor).take() else {
            return;
        };
        if let Some(prewarm) = locked(&self.prewarm).take() {
            self.awaiting.store(true, Ordering::Release);
            *locked(&predecessor.handover) = Some(Handover {
                core: Arc::clone(core),
                prewarm,
            });
        }
        if let Some(outcome) = locked(&self.outcome).take() {
            outcome(true);
        }
    }

    // ── session side ────────────────────────────────────────────────────────────────────

    /// The next export that is still wanted.
    pub fn take(&self) -> Option<Call> {
        let rx = locked(&self.rx);
        while let Ok(call) = rx.try_recv() {
            if call.state.start() {
                return Some(call);
            }
        }
        None
    }

    /// The session ended: every queued export is answered `SessionEnded`, and later hooks fail.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        while let Some(call) = self.take() {
            let _ = call.answer.send(Answer::SessionEnded);
        }
    }

    pub fn set_supported(&self, supported: bool) {
        *locked(&self.supported) = Some(supported);
        self.probed.notify_all();
    }

    /// Whether the guest exports `lifecycle`, waiting for the session's probe.
    pub fn wait_supported(&self, wait: Duration) -> Option<bool> {
        let guard = locked(&self.supported);
        let (guard, _) = self
            .probed
            .wait_timeout_while(guard, wait, |supported| supported.is_none())
            .unwrap_or_else(|error| error.into_inner());
        *guard
    }

    /// After a swap's `snapshot` the predecessor takes no new frame turn or trigger.
    pub fn set_fenced(&self, fenced: bool) {
        self.fenced.store(fenced, Ordering::Release);
    }

    /// The snapshot was taken for the swap attempt `owner`: fence the predecessor, unless that
    /// attempt already ended (its successor was dropped, which released the fence for good) or a
    /// later attempt replaced it, so a slow snapshot of a refused swap never fences the retry.
    pub fn fence_for_snapshot(&self, owner: Option<&Arc<AtomicBool>>) {
        let attempt = locked(&self.attempt);
        if let (Some(live), Some(owner)) = (attempt.as_ref(), owner) {
            if Arc::ptr_eq(live, owner) && live.load(Ordering::Acquire) {
                self.fenced.store(true, Ordering::Release);
            }
        }
    }

    pub fn fenced(&self) -> bool {
        self.fenced.load(Ordering::Acquire)
    }

    pub fn take_handover(&self) -> Option<Handover> {
        locked(&self.handover).take()
    }

    // ── swap wiring ─────────────────────────────────────────────────────────────────────

    pub fn set_swap_context(&self, context: SwapContext) {
        *locked(&self.swap) = Some(context);
    }

    pub fn swap_context(&self) -> Option<SwapContext> {
        locked(&self.swap).clone()
    }

    /// Install a pre-warmed child on a successor's link, behind `predecessor`.
    pub fn attach_successor(
        &self,
        prewarm: Prewarm,
        predecessor: &Arc<LifecycleLink>,
        outcome: impl FnOnce(bool) + Send + 'static,
    ) {
        self.set_cap(prewarm.turn_budget);
        *locked(&self.prewarm) = Some(prewarm);
        *locked(&self.own) = Some(predecessor.begin_attempt());
        *locked(&self.predecessor) = Some(Arc::clone(predecessor));
        *locked(&self.outcome) = Some(Box::new(outcome));
    }
}

impl LifecycleLink {
    /// A new swap attempt on this (predecessor) link. It ends whichever attempt it replaces,
    /// fence included, and the token it returns is the only one that may later release it.
    pub fn begin_attempt(&self) -> Arc<AtomicBool> {
        let token = Arc::new(AtomicBool::new(true));
        if let Some(old) = locked(&self.attempt).replace(Arc::clone(&token)) {
            old.store(false, Ordering::Release);
            self.set_fenced(false);
        }
        token
    }

    /// End `attempt`, and release the fence only if it is still the installed one: a late end
    /// of an older attempt must not unfence a predecessor a newer attempt froze.
    pub fn end_attempt(&self, attempt: &Arc<AtomicBool>) {
        attempt.store(false, Ordering::Release);
        let mut installed = locked(&self.attempt);
        if installed
            .as_ref()
            .is_some_and(|live| Arc::ptr_eq(live, attempt))
        {
            installed.take();
            self.set_fenced(false);
        }
    }

    /// End the swap attempt this (successor) link was installed for, once: the first of a
    /// failed or refused exchange and the link's drop does it.
    fn release_predecessor(&self) {
        let own = locked(&self.own).take();
        if let (Some(predecessor), Some(own)) = (locked(&self.predecessor).take(), own) {
            predecessor.end_attempt(&own);
        }
    }
}

impl Drop for LifecycleLink {
    /// A successor that is dropped before its `swap-in` committed releases its predecessor.
    fn drop(&mut self) {
        self.release_predecessor();
        if let Some(outcome) = locked(&self.outcome).take() {
            outcome(false);
        }
    }
}

/// Sessions the door starts for a cold swap, and a hold that keeps the root up across one.
#[derive(Default)]
pub struct HostedSessions {
    live: std::sync::atomic::AtomicUsize,
}

pub struct HostedGuard(Arc<HostedSessions>);

impl Drop for HostedGuard {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::AcqRel);
    }
}

impl HostedSessions {
    pub fn hold(self: &Arc<Self>) -> HostedGuard {
        self.live.fetch_add(1, Ordering::AcqRel);
        HostedGuard(Arc::clone(self))
    }

    /// Resolves when no cold swap is in flight and every session it started has ended.
    pub async fn wait_idle(&self) {
        while self.live.load(Ordering::Acquire) != 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

pub type SessionBuilder = Arc<
    dyn Fn(Arc<crate::supervision::WorkerSupervisor>) -> crate::spirit_session::SessionDeps
        + Send
        + Sync,
>;

/// What a swap needs from the root, filled once root supervision is armed.
pub struct SwapHandles {
    pub supervisor: std::sync::Weak<crate::supervision::WorkerSupervisor>,
    pub host: Arc<dyn maos_host::SpiritHostPort>,
    /// Builds the session's dependencies around a live supervisor.
    pub session: SessionBuilder,
    pub hosted: Arc<HostedSessions>,
}

/// A weak handle to the root's [`SwapHandles`], filled once root supervision is armed. The
/// supervisor owns them (`WorkerSupervisor::keep_swap_handles`), so the door and the
/// successor factory that read this slot never keep the root's `audit_tx` owners alive.
#[derive(Clone, Default)]
pub struct SwapSlot(Arc<std::sync::OnceLock<std::sync::Weak<SwapHandles>>>);

impl SwapSlot {
    pub fn set(&self, handles: &Arc<SwapHandles>) {
        let _ = self.0.set(Arc::downgrade(handles));
    }

    /// `None` before the root arms, and after its teardown dropped the supervisor.
    pub fn get(&self) -> Option<Arc<SwapHandles>> {
        self.0.get()?.upgrade()
    }
}

impl SwapHandles {
    /// Root-owned setup shared by every composition that arms supervision (`maos run`, and a
    /// serving or shell root that boots installed components): the swap handles go to the slot
    /// the door reads and to the supervisor that owns them.
    pub fn arm(
        supervisor: &Arc<crate::supervision::WorkerSupervisor>,
        host: &Arc<dyn maos_host::SpiritHostPort>,
        session: &SessionBuilder,
        hosted: &Arc<HostedSessions>,
        slot: &SwapSlot,
    ) {
        let handles = Arc::new(SwapHandles {
            supervisor: Arc::downgrade(supervisor),
            host: Arc::clone(host),
            session: Arc::clone(session),
            hosted: Arc::clone(hosted),
        });
        slot.set(&handles);
        supervisor.keep_swap_handles(handles);
    }

    /// The spawned session of `spirit_id` a swap would replace, if there is one.
    pub fn target(&self, spirit_id: &str) -> Option<Arc<BindingCore>> {
        self.supervisor.upgrade()?.spawned_target(spirit_id)
    }

    /// A cold swap's admission of a component (D-20-9-12 A): the one spawned admission path
    /// `InstalledSpirits::boot` uses, at a new pid, with its session hosted by this root. It
    /// returns only once the component answered its lifecycle probe (its runner is `Ready` and
    /// its start succeeded); a component that never starts is unwound — session stopped and
    /// joined, SCB unloaded — and refused, so the door can restore the predecessor.
    pub async fn admit_spawned(
        &self,
        deps: &crate::admission::AdmissionDeps<'_>,
        boot_nonce: u64,
        gated: &crate::admission::GatedManifest,
        manifest: &std::path::Path,
        origin: &std::path::Path,
    ) -> Result<crate::admission::Admitted, crate::admission::AdmissionRefusal> {
        use crate::admission::{load_admit_start_unarchived, AdmissionRefusal, StartPolicy};
        let class_build = |detail: String| AdmissionRefusal::ClassBuild { detail };
        let supervisor =
            self.supervisor
                .upgrade()
                .ok_or(AdmissionRefusal::SpawnedSurfaceUnsupported {
                    surface: "operator door",
                })?;
        let launch =
            crate::spirit_session::manifest_launch_in(&*self.host, gated, manifest, origin, false)
                .map_err(|error| class_build(error.to_string()))?;
        let (core, object) = supervisor
            .prepare_spawned(&gated.spirit_id, &gated.bundle())
            .map_err(|error| class_build(error.to_string()))?;
        let bound = Arc::clone(&core);
        let admitted = load_admit_start_unarchived(
            deps,
            gated,
            object,
            boot_nonce,
            StartPolicy::StartNow,
            &mut |pid| bound.bind_scheduler_pid(pid),
        )
        .await?;
        let session = (self.session)(Arc::clone(&supervisor));
        let (id, hold) = (gated.spirit_id.clone(), self.hosted.hold());
        let (started, link, budget) = (
            admitted.clone(),
            Arc::clone(core.lifecycle()),
            launch.turn_budget,
        );
        let mut task = Some(tokio::spawn(async move {
            let _hold = hold;
            let result =
                crate::spirit_session::run_admitted_session(session, started, core, launch).await;
            if let Err(error) = &result {
                eprintln!("maos: spirit '{id}' session ended: {error}");
            }
            result.map(|_| ())
        }));
        let until = Instant::now() + budget.min(ATTEMPT_WALL) + READY_GRACE;
        let failure = loop {
            if link.wait_supported(Duration::ZERO).is_some() {
                // Only a component that started is the version the FR11 archive names.
                crate::admission::archive_admitted(&maos_audit::default_archive_dir(), gated);
                return Ok(admitted);
            }
            if task.as_ref().is_some_and(|task| task.is_finished()) {
                break match task.take() {
                    Some(task) => match task.await {
                        Ok(Err(error)) => error.to_string(),
                        Ok(Ok(())) => "the session ended before the component was ready".into(),
                        Err(error) => error.to_string(),
                    },
                    None => String::new(),
                };
            }
            if Instant::now() >= until {
                break "the component did not become ready in time".into();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        if let Some(task) = task {
            supervisor.stop_binding(admitted.pid);
            let _ = tokio::time::timeout(Duration::from_secs(10), task).await;
        }
        crate::admission::unwind_admitted(deps, admitted.pid, &gated.spirit_id).await;
        Err(class_build(format!("spawned_start_failed: {failure}")))
    }
}

/// How long a successor's pre-warm waits for the predecessor's own lifecycle probe.
const PROBE_WAIT: Duration = Duration::from_secs(5);

/// Slack past a component's start budget before a cold swap gives up waiting for its probe.
const READY_GRACE: Duration = Duration::from_secs(5);

/// The spawned arm of the successor factory (D-20-9-09). Every refusal comes before any
/// coordinator step, so the predecessor keeps running, unchanged. It admits the successor
/// through `gate_manifest`, requires its declared scopes inside what the operator admitted at
/// this pid, launches and probes its runner at the predecessor's pid, and journals its
/// declaration through the one writer. Dropping the object it returns kills the pre-warmed child.
pub fn spawned_successor(
    handles: &SwapHandles,
    tl: &Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    manifest: &maos_kernel_core::scheduler::control_block::SpiritManifestBundle,
    staged: Option<std::path::PathBuf>,
    origin: Option<std::path::PathBuf>,
) -> Result<Arc<dyn maos_kernel_core::scheduler::control_block::AnySpiritObj>, String> {
    use crate::admission::{
        emit_manifest_declaration_event, emit_manifest_declaration_outcome, gate_manifest,
        read_manifest,
    };
    let began = Instant::now();
    let refused = |error: crate::admission::AdmissionRefusal| format!("{}: {error}", error.code());
    let supervisor = handles
        .supervisor
        .upgrade()
        .ok_or("spawned_swap_unsupported: the root is shutting down")?;
    let class = manifest
        .class
        .as_ref()
        .ok_or("successor manifest lacks [class]")?;
    if class.forms != ["wasm-component"] {
        return Err(format!(
            "swap_form_mismatch: '{}' is a spawned Spirit and the successor is {:?}",
            class.name, class.forms
        ));
    }
    let enabled: Vec<&str> = manifest
        .lifecycle
        .enabled_hooks
        .iter()
        .map(|hook| hook.as_str())
        .collect();
    // The coordinator treats a manifest-skipped `swap-in` as a successful swap, but only a
    // `swap-in` that reaches the guest hands the child over: refuse before anything is fenced.
    if !maos_spirit_abi::lifecycle::kernel_invocation_allowed(&enabled, "on_swap_in") {
        return Err(format!(
            "swap_hook_disabled: the successor's [lifecycle].enabled_hooks omits on_swap_in, so '{}' \
             could not be handed the predecessor's state",
            class.name
        ));
    }
    let target = supervisor.spawned_target(&class.name).ok_or_else(|| {
        format!(
            "swap_form_mismatch: '{}' is not a spawned Spirit",
            class.name
        )
    })?;
    let path = staged.ok_or("spawned_swap_unsupported: the successor manifest is not staged")?;
    let gated =
        gate_manifest(&read_manifest(&path).map_err(refused)?, &|_| false).map_err(refused)?;
    let context = target
        .lifecycle()
        .swap_context()
        .ok_or("spawned_swap_unsupported")?;
    match target.lifecycle().wait_supported(PROBE_WAIT) {
        Some(true) => {}
        Some(false) => {
            return Err("spawned_swap_unsupported: the running Spirit exports no lifecycle".into())
        }
        // A slow component start (a TypeScript guest compiles for seconds) is not a missing export.
        None => {
            return Err(
                "spawned_swap_not_ready: the running Spirit has not finished starting; retry"
                    .into(),
            )
        }
    }
    if let Some(scope) = maos_manifest::capabilities_required_to_scopes(&gated.caps_required)
        .into_iter()
        .find(|scope| !context.declared().contains(scope))
    {
        return Err(format!(
            "swap_capability_widened: the successor declares {scope:?}, which this pid was not admitted for"
        ));
    }
    let launch_origin = origin.unwrap_or_else(|| {
        path.parent()
            .map_or_else(|| ".".into(), std::path::Path::to_path_buf)
    });
    let launch = crate::spirit_session::manifest_launch_in(
        &*handles.host,
        &gated,
        &path,
        &launch_origin,
        false,
    )
    .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let (core, object) = supervisor
        .prepare_spawned(&class.name, manifest)
        .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    core.bind_scheduler_pid(target.spirit_pid());
    let mut bridge = crate::spirit_session::spawn_runner(
        context.sandbox.clone(),
        launch.plan,
        launch.component_artifact.as_ref(),
        &class.name,
    )
    .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let writer = bridge
        .take_frame_writer()
        .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let mut prewarm = Prewarm {
        bridge,
        writer,
        turn_budget: launch.turn_budget,
        context_window: launch.context_window,
        scopes: launch.scopes,
        deadline: began + ATTEMPT_WALL,
    };
    let launch_failed = |error: String| format!("spawned_swap_launch_failed: {error}");
    prewarm
        .await_ready(launch.turn_budget)
        .map_err(launch_failed)?;
    match prewarm.exchange(&LifecycleRequest::Probe, launch.turn_budget) {
        Ok(LifecycleReply::Supported(true)) => {}
        Ok(_) => return Err("spawned_swap_unsupported: the successor exports no lifecycle".into()),
        Err(error) => return Err(launch_failed(error)),
    }
    let attempt = emit_manifest_declaration_event(tl, &gated)
        .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let tl = Arc::clone(tl);
    core.lifecycle().set_swap_context(SwapContext {
        sandbox: context.sandbox,
        origin: launch_origin,
    });
    core.lifecycle()
        .attach_successor(prewarm, target.lifecycle(), move |committed| {
            let outcome = if committed { "admitted" } else { "refused" };
            emit_manifest_declaration_outcome(&tl, &attempt, outcome);
        });
    Ok(object)
}

/// The kernel callers of a spawned component's idle, schedule and consolidate triggers. A
/// serving root starts them in its serving tail; `maos run` awaits the component's session
/// instead and never reaches it, so it starts them around that session and joins them before
/// it stops supervision (the telemetry pump has its own owner).
pub struct LifecycleCallers {
    cancel: tokio_util::sync::CancellationToken,
    watchdogs: Vec<(&'static str, tokio::task::JoinHandle<()>)>,
    consolidation: crate::hook_pump::ConsolidationWatchdog,
}

impl LifecycleCallers {
    pub fn start(
        scheduler: &maos_kernel_core::scheduler::SpiritSchedulerAdapter,
        tl: &Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
        capability: &Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
        hooks: &crate::hook_pump::HookPump,
    ) -> Self {
        use maos_kernel_core::scheduler::{IdleWatchdog, ScheduleWatchdog};
        let cancel = tokio_util::sync::CancellationToken::new();
        let idle = Arc::new(IdleWatchdog::new(
            scheduler.scbs(),
            scheduler.dispatcher_arc(),
        ))
        .spawn(cancel.child_token());
        let schedule = Arc::new(
            ScheduleWatchdog::new(scheduler.scbs(), scheduler.dispatcher_arc(), Arc::clone(tl))
                .with_capability(Arc::clone(capability)),
        )
        .spawn(cancel.child_token());
        Self {
            cancel,
            watchdogs: vec![("IdleWatchdog", idle), ("ScheduleWatchdog", schedule)],
            consolidation: crate::hook_pump::ConsolidationWatchdog::spawn(
                hooks,
                crate::hook_pump::ConsolidationCadence::from_env(),
            ),
        }
    }

    /// Stop and join every caller; each holds the scheduler and the capability registry, so
    /// the audit writer cannot drain while one runs.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        for (name, handle) in self.watchdogs {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("maos: {name} task returned error during drain: {error}")
                }
                Err(_) => eprintln!("maos: {name} drain timed out after 5s"),
            }
        }
        self.consolidation.shutdown().await;
    }
}
