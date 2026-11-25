//! Spawned-form lifecycle triggers and the state-preserving swap (Story 20-9).
//!
//! A `WorkerSpirit` hook sends one [`LifecycleRequest`] through its [`LifecycleLink`], a bounded
//! slot its session drains one export at a time. The pre-warmed successor of a swap is the
//! exception: it has no session yet, so its `migrate` and `swap-in` run inline on its own child,
//! and a committed `swap-in` hands that child to the predecessor's session.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
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
/// The session ended before the guest was entered for this call: the trigger never ran.
pub const NOT_ENTERED: &str = "session_ended_not_entered";
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
    /// The session closed and drained this call before the guest was entered for it.
    NotEntered,
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
    /// The hook's link deadline for this call; a started call past it is EXPIRED.
    link_deadline: Instant,
    /// The one absolute wall the kernel captured before queuing an `on_idle` / `on_schedule`
    /// firing; the session ends the export at this same instant, never at a recomputed cap.
    firing: Option<Instant>,
}

impl CallState {
    const QUEUED: u8 = 0;
    const STARTED: u8 = 1;
    const CANCELLED: u8 = 2;
    const ANSWERED: u8 = 3;
    /// A started call whose firing wall or link deadline passed before a reply was claimed.
    const EXPIRED: u8 = 4;

    fn new(
        owner: Option<Arc<AtomicBool>>,
        invocation: Option<Arc<Invocation>>,
        firing: Option<Instant>,
        link_deadline: Instant,
    ) -> Arc<Self> {
        Arc::new(Self {
            phase: AtomicU8::new(Self::QUEUED),
            owner,
            invocation,
            link_deadline,
            firing,
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
        if self.lapsed() {
            // A reply observed past the firing wall or link deadline changes nothing: the
            // started call becomes EXPIRED in one atomic step (a concurrent waiter or an
            // earlier answer/cancel that already won leaves its own phase untouched).
            self.advance(Self::STARTED, Self::EXPIRED);
            return false;
        }
        self.advance(Self::STARTED, Self::ANSWERED)
    }

    /// The absolute wall the kernel captured for this firing, if it is one.
    pub fn firing(&self) -> Option<Instant> {
        self.firing
    }

    /// The earliest wall this call lives under: its firing wall, else its link deadline.
    pub fn expiry(&self) -> Instant {
        self.firing
            .map_or(self.link_deadline, |wall| wall.min(self.link_deadline))
    }

    /// The call's firing wall or link deadline has passed.
    fn lapsed(&self) -> bool {
        Instant::now() >= self.expiry()
    }

    /// The firing's absolute wall (not the link deadline) has passed.
    fn firing_lapsed(&self) -> bool {
        self.firing.is_some_and(|wall| Instant::now() >= wall)
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

    /// Hook: the link deadline passed. `Some(false)`: the call was still queued and is dropped;
    /// `Some(true)`: the call is EXPIRED while the session was executing it (whether this
    /// waiter or the session's own observation made the one atomic transition);
    /// `None`: the session claimed the reply first.
    fn expire(&self) -> Option<bool> {
        if self.cancel_queued() {
            Some(false)
        } else if self.advance(Self::STARTED, Self::EXPIRED) || self.link_expired() {
            Some(true)
        } else {
            None
        }
    }

    /// Session: this call's started execution is EXPIRED (distinct from CANCELLED/ANSWERED).
    pub fn link_expired(&self) -> bool {
        self.phase.load(Ordering::Acquire) == Self::EXPIRED
    }

    /// Session: observe a started call's deadline. `true` once the call is EXPIRED, either
    /// already or because its wall passed and this transition won against any reply claim;
    /// `false` for a call that is not started-and-lapsed (answered, cancelled, queued).
    pub fn observe_expiry(&self) -> bool {
        if self.link_expired() {
            return true;
        }
        self.lapsed() && self.advance(Self::STARTED, Self::EXPIRED)
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
    /// SHA-256 of the exact component bytes this child was launched with: what a restore of
    /// this incarnation must reacquire, never "whatever the pathname holds now".
    pub artifact_sha256: Option<[u8; 32]>,
    /// The publisher key verified when this incarnation was launched from a signed package
    /// (or restored from one that was); `None` for a local source.
    pub verified_publisher: Option<[u8; 32]>,
}

impl SwapContext {
    pub fn declared(&self) -> &[Scope] {
        &self.sandbox.declared_scopes
    }
}

/// A launched, `Ready`, lifecycle-probed successor child.
pub struct Prewarm {
    pub bridge: SpawnedBridge,
    /// The successor's exact stop handle, taken when it was spawned and moved to the host
    /// owner at handover; no fallible acquisition remains after the swap commits.
    pub(crate) stop: maos_kernel_core::lifecycle::cli_wrapper::runtime::ChildStopHandle,
    pub writer: FrameWriter,
    pub turn_budget: Duration,
    pub context_window: maos_kernel_core::capability::context_budget::ContextWindow,
    pub scopes: Vec<Scope>,
    /// The authority this successor was launched under: the predecessor's admitted sandbox
    /// narrowed to exactly the successor's own declared scopes (each entitled by a scope the
    /// predecessor held). The session's sends, recall and skill authority derive from it
    /// once the outer commit adopts this child, and the next link's swap context carries it.
    pub sandbox: SandboxSpec,
    /// Where this swap attempt must be over, however long its stages' budgets: set when the
    /// attempt began, shared by the pre-warm and every exchange of the swap.
    pub deadline: Instant,
    /// Where this successor's stderr rows go, until the session that adopts it takes over.
    pub(crate) log: PrewarmLog,
}

/// The journal row writer of a pre-warmed successor's stderr (Story 22-9): the Spirit's pid and
/// id, and the same row/byte bound a serving session applies.
pub(crate) struct PrewarmLog {
    pub(crate) tl: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    pub(crate) spirit_id: String,
    pub(crate) pid: u32,
    pub(crate) dropped: u64,
    pub(crate) budget: crate::spirit_session::DiagnosticBudget,
}

/// A committed successor: the predecessor's session adopts it.
pub struct Handover {
    pub core: Arc<BindingCore>,
    pub prewarm: Prewarm,
}

impl Prewarm {
    /// The next stdout frame of a child nobody else is serving, never past `deadline`. Stderr
    /// is journaled as `spirit.diagnostic` rows tagged `prewarm` (Story 22-9) under the serving
    /// bound, then dropped with the overflow counted; every line is a turn of this loop, so a
    /// guest that never stops writing diagnostics cannot outlast its cap; a frame that arrives
    /// late is refused.
    fn next_stdout(&mut self, deadline: Instant) -> Result<Vec<u8>, String> {
        next_stdout_on(&mut Live(self), deadline, &mut false)
    }

    /// The request's write, held to the same wall as its reply by the transport's own
    /// deadline (`write_frame_until`), on this thread. The `FrameWriter` owns the terminal
    /// partial-write state: after a timed-out or failed frame it refuses every later write.
    /// The refusal then tears the child down once, on its owner's path (the link drops the
    /// prewarm and releases its attempt); nothing here kills or reaps it.
    fn write_until(&mut self, body: &[u8], deadline: Instant) -> Result<(), String> {
        self.writer
            .write_frame_until(body, deadline)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::TimedOut {
                    DEADLINE.into()
                } else {
                    format!("child_write: {error}")
                }
            })
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
        let attempt = self.deadline;
        exchange_on(&mut Live(self), request, cap, attempt)
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

/// The clock, request write and frame poll of one pre-warm stage: the live child in production
/// (a monomorphized wrapper, no dynamic dispatch), a scripted logical clock in the tests.
trait StageIo {
    fn now(&self) -> Instant;
    fn write(&mut self, body: &[u8], deadline: Instant) -> Result<(), String>;
    fn poll(&mut self, wait: Duration) -> Result<BridgePoll, String>;
    /// One guest stderr line; the scripted clock of the unit tests has no journal.
    fn stderr(&mut self, _line: &[u8]) {}
    /// A stage's reply was valid but its stage-end sentinel never came before the stage's
    /// deadline or the child's close; the scripted clock of the unit tests has no operator.
    fn stage_end_missing(&mut self, _why: &str) {}
}

struct Live<'a>(&'a mut Prewarm);

impl StageIo for Live<'_> {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn write(&mut self, body: &[u8], deadline: Instant) -> Result<(), String> {
        self.0.write_until(body, deadline)
    }

    fn poll(&mut self, wait: Duration) -> Result<BridgePoll, String> {
        self.0
            .bridge
            .poll_frame(wait)
            .map_err(|error| format!("child_poll: {error}"))
    }

    fn stderr(&mut self, line: &[u8]) {
        let log = &mut self.0.log;
        crate::spirit_session::journal_guest_diagnostic(
            &log.tl,
            (&log.spirit_id, log.pid, self.0.bridge.child_pid()),
            (&mut log.dropped, &mut log.budget),
            line,
            Some("prewarm"),
        );
    }

    fn stage_end_missing(&mut self, why: &str) {
        eprintln!(
            "{{\"event\":\"prewarm_stage_end_missing\",\"child_pid\":{},\"why\":\"{why}\"}}",
            self.0.bridge.child_pid()
        );
    }
}

/// One stderr line of a stage: journaled, except the runner's stage-end sentinel, which is
/// never a diagnostic (only the guest bytes the sentinel joined on an unterminated line are).
/// `true` when the line carried the sentinel.
fn route_stderr<I: StageIo>(io: &mut I, line: &[u8]) -> bool {
    match maos_frame_codec::split_stage_end(line) {
        Some(guest) => {
            if !guest.is_empty() {
                io.stderr(guest);
            }
            true
        }
        None => {
            io.stderr(line);
            false
        }
    }
}

/// `stage_end` is set when the stage's sentinel line is read, before or after the reply.
fn next_stdout_on<I: StageIo>(
    io: &mut I,
    deadline: Instant,
    stage_end: &mut bool,
) -> Result<Vec<u8>, String> {
    loop {
        let left = deadline.saturating_duration_since(io.now());
        if left.is_zero() {
            return Err(DEADLINE.into());
        }
        match io.poll(left.min(Duration::from_millis(20)))? {
            BridgePoll::Idle => {}
            BridgePoll::Data(SubStream::Stderr, line) => {
                *stage_end |= route_stderr(io, &line);
            }
            BridgePoll::Closed => return Err("child_closed".into()),
            BridgePoll::Data(SubStream::Stdout, bytes) => {
                return if io.now() > deadline {
                    Err(DEADLINE.into())
                } else {
                    Ok(bytes)
                };
            }
        }
    }
}

/// After a stage's reply: the guest lines it wrote before that reply. The runner writes the
/// stage-end sentinel to stderr right before the reply frame, and stderr is one ordered stream,
/// so reading to the sentinel reads every earlier line however the two reader threads raced to
/// enqueue the reply and those lines (the reply can be dequeued before any of them). Each line
/// is journaled as it arrives; the sentinel itself never is. A deadline or a close before the
/// sentinel leaves the stage's valid reply as it is (the child may be dying): it is reported,
/// never a failure. A second stdout frame is the one thing a request/reply child must not send.
fn drain_to_stage_end<I: StageIo>(io: &mut I, deadline: Instant) -> Result<(), String> {
    loop {
        let left = deadline.saturating_duration_since(io.now());
        if left.is_zero() {
            io.stage_end_missing(DEADLINE);
            return Ok(());
        }
        match io.poll(left.min(Duration::from_millis(20)))? {
            BridgePoll::Idle => {}
            BridgePoll::Closed => {
                io.stage_end_missing("child_closed");
                return Ok(());
            }
            BridgePoll::Data(SubStream::Stdout, _) => return Err("child_protocol".into()),
            BridgePoll::Data(SubStream::Stderr, line) => {
                if route_stderr(io, &line) {
                    return Ok(());
                }
            }
        }
    }
}

/// The one `min(cap, attempt)` deadline of a stage, shared by the request's write and then the
/// reply's poll: the write's completion grants the reply no fresh budget.
fn exchange_on<I: StageIo>(
    io: &mut I,
    request: &LifecycleRequest,
    cap: Duration,
    attempt: Instant,
) -> Result<LifecycleReply, String> {
    let deadline = (io.now() + cap).min(attempt);
    let body = wire::encode_request(request).map_err(|error| format!("encode: {error}"))?;
    io.write(&body, deadline)?;
    let mut stage_end = false;
    let bytes = next_stdout_on(io, deadline, &mut stage_end)?;
    if !wire::is_reply(&bytes) {
        return Err("child_protocol".into());
    }
    let reply = wire::decode_reply(&bytes).map_err(|error| format!("decode: {error}"))?;
    if !stage_end {
        drain_to_stage_end(io, deadline)?;
    }
    Ok(reply)
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
    /// A successor whose guest `swap-in` answered `Done` but whose outer swap the coordinator
    /// has not yet accepted: provisional, adopts nothing and publishes nothing. Only
    /// [`Self::publish_commit`] (the host's `SwapCommitted` event) moves it to the predecessor;
    /// a refused, stale or detached swap-in leaves it here and it dies with this link.
    staged: Mutex<Option<(Weak<BindingCore>, Prewarm)>>,
    /// The validated effective `[lifecycle.telemetry]` wall of the manifest this link serves
    /// (`hook_pump::telemetry_wall`); `None`: the manifest receives no telemetry here.
    telemetry_wall: Mutex<Option<Duration>>,
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
            staged: Mutex::new(None),
            telemetry_wall: Mutex::new(None),
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
    /// `firing` is the absolute wall the kernel captured before queuing an `on_idle` /
    /// `on_schedule` firing; the queued call carries that same instant to the session.
    pub fn call(
        &self,
        core: &Arc<BindingCore>,
        request: LifecycleRequest,
        firing: Option<Instant>,
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
                self.stage(core);
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
        // Telemetry is held to the one effective wall the manifest's validated declaration
        // gives it (the pump's own); a link with none grants nothing, however the hook was fired.
        let limit = match request {
            LifecycleRequest::Telemetry { .. } => match *locked(&self.telemetry_wall) {
                Some(wall) => Some(wall),
                None => return Ok(LifecycleReply::Unsupported),
            },
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
        let to_deadline = limit.map_or(cap + WALL_SLACK, |limit| {
            cap.min(queue_share + limit + WALL_SLACK)
        });
        // The firing's absolute wall passed before it could even queue (the blocking pool or
        // the caller was late): dropped untouched, never a strike, and no running call or
        // session is disturbed.
        if firing.is_some_and(|wall| Instant::now() >= wall) {
            return Err(BUSY.into());
        }
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
        // A kernel firing's one absolute wall also bounds the queue wait, so a call still
        // unstarted at it is dropped as busy; once started the session ends itself at that
        // same instant, and this waiter's own backstop is the wall plus the pump slack.
        let queue_deadline =
            firing.map_or(began + queue_share, |wall| (began + queue_share).min(wall));
        let deadline = firing.map_or(began + to_deadline, |wall| {
            (began + to_deadline).min(wall + WALL_SLACK)
        });
        let (answer, reply) = sync_channel(1);
        let state = CallState::new(owner, invocation, firing, deadline);
        let mut call = Call {
            request,
            answer,
            state: Arc::clone(&state),
        };
        loop {
            if self.closed.load(Ordering::Acquire) {
                return Err(NOT_ENTERED.into());
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
                Err(TrySendError::Disconnected(_)) => return Err(NOT_ENTERED.into()),
            }
        }
        loop {
            match reply.recv_timeout(Duration::from_millis(20)) {
                Ok(Answer::Reply(reply)) => return Ok(reply),
                Ok(Answer::SessionEnded) => return Err(SESSION_ENDED.into()),
                Ok(Answer::NotEntered) => return Err(NOT_ENTERED.into()),
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
                    if now >= deadline {
                        match state.expire() {
                            // Still queued: nothing of this hook's is executing, so no session
                            // is touched and the busy session's own work is left alone.
                            Some(false) => return Err(DEADLINE.into()),
                            // Executing: giving up as the waiter would leave the export running
                            // past its link deadline, so the owning session is told (typed,
                            // `CallState::link_expired`) and ends itself, journaling the cause
                            // and killing and finalizing the child it owns.
                            Some(true) => return Err(DEADLINE.into()),
                            None => {}
                        }
                    }
                }
            }
        }
    }

    /// The successor's guest `swap-in` answered `Done`: provisional only. The child waits here
    /// until the coordinator accepts the outer swap ([`Self::publish_commit`]).
    fn stage(&self, core: &Arc<BindingCore>) {
        if let Some(prewarm) = locked(&self.prewarm).take() {
            *locked(&self.staged) = Some((Arc::downgrade(core), prewarm));
        }
    }

    /// The coordinator published this link's object after accepting the outer swap-in: the
    /// child now belongs to the predecessor's session, which adopts it. Until then this
    /// link's hooks queue for that session rather than vanish. A swap that is no longer the
    /// predecessor's live attempt hands nothing over (the staged child dies here).
    pub fn publish_commit(&self) {
        // Nothing staged: end the attempt (and its fence) while the predecessor and the
        // attempt token are still held; the outcome stays for `Drop` (`false`).
        let Some((core, prewarm)) = locked(&self.staged).take() else {
            self.release_predecessor();
            return;
        };
        let Some(predecessor) = locked(&self.predecessor).take() else {
            return;
        };
        // The core owns this link: it is attached to the handover only here, at publication,
        // so a staged child that is never published holds no reference back to its own owner.
        let Some(core) = core.upgrade() else {
            return;
        };
        let staged = Handover { core, prewarm };
        let live = locked(&self.own)
            .as_ref()
            .is_some_and(|own| predecessor.attempt_is_live(own));
        if !live {
            return;
        }
        // Story 21-7: the outcome callback installs the successor's consent declaration. It
        // runs BEFORE the handover becomes visible, so the session can never serve the
        // successor's child under the predecessor's policy.
        let outcome = locked(&self.outcome).take();
        if let Some(outcome) = outcome {
            outcome(true);
        }
        // `closed` is read under the same slot lock `take_handover` uses, and the session
        // closes the link before draining the slot: either the session drains what is parked
        // here, or this sees `closed` and refuses to park on a terminal session.
        let rejected = {
            let mut slot = locked(&predecessor.handover);
            if predecessor.closed.load(Ordering::Acquire) {
                Some(staged)
            } else {
                self.awaiting.store(true, Ordering::Release);
                slot.replace(staged)
            }
        };
        // A refused bridge or a displaced handover is torn down by its own drop, outside
        // every lock.
        drop(rejected);
    }

    // ── session side ────────────────────────────────────────────────────────────────────

    /// The next export that is still wanted.
    pub fn take(&self) -> Option<Call> {
        let rx = locked(&self.rx);
        while let Ok(call) = rx.try_recv() {
            if call.state.firing_lapsed() {
                // The firing's absolute wall passed while this call waited: it never starts
                // (the guest is never entered for it) and costs its hook no strike.
                if call.state.cancel_queued() {
                    let _ = call.answer.send(Answer::Reply(LifecycleReply::Unsupported));
                }
                continue;
            }
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
            let _ = call.answer.send(Answer::NotEntered);
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

    /// A swap holds this predecessor link: its `snapshot` fenced it, or a committed successor
    /// awaits adoption. The predecessor's session does not end cleanly while that is so.
    pub fn swap_in_flight(&self) -> bool {
        self.fenced() || locked(&self.handover).is_some()
    }

    pub fn take_handover(&self) -> Option<Handover> {
        locked(&self.handover).take()
    }

    // ── swap wiring ─────────────────────────────────────────────────────────────────────

    pub fn set_swap_context(&self, context: SwapContext) {
        *locked(&self.swap) = Some(context);
    }

    /// The effective telemetry wall this link holds a delivery to (`None`: no telemetry).
    pub fn set_telemetry_wall(&self, wall: Option<Duration>) {
        *locked(&self.telemetry_wall) = wall;
    }

    /// A swap this (predecessor) link was fenced for ended without its successor being
    /// adopted (stale or reverted): release the fence it still holds.
    pub fn end_current_attempt(&self) {
        if let Some(attempt) = locked(&self.attempt).take() {
            attempt.store(false, Ordering::Release);
        }
        self.set_fenced(false);
    }

    fn attempt_is_live(&self, own: &Arc<AtomicBool>) -> bool {
        locked(&self.attempt)
            .as_ref()
            .is_some_and(|live| Arc::ptr_eq(live, own) && live.load(Ordering::Acquire))
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

/// How long a spawned admission waits for the previous incarnation's session to finish tearing down.
pub const SESSION_END_WAIT: Duration = Duration::from_secs(15);

/// A session's hold on its end: the session drops it as its last act, which ends the session
/// for every waiter (its mailbox registration and identity are released by then).
pub type SessionEnd = tokio::sync::watch::Sender<()>;

/// The latest session of each Spirit id of a root, for the admission of the id's next incarnation
/// to wait on exactly that teardown, not on a guess.
#[derive(Default)]
pub struct SessionEnds(Mutex<std::collections::BTreeMap<String, tokio::sync::watch::Receiver<()>>>);

impl SessionEnds {
    pub fn begin(&self, id: &str) -> SessionEnd {
        let (end, watch) = tokio::sync::watch::channel(());
        locked(&self.0).insert(id.to_owned(), watch);
        end
    }

    /// Resolves once the latest session of `id` has ended (at once if it has, or never began).
    pub async fn ended(&self, id: &str) {
        let watch = locked(&self.0).get(id).cloned();
        if let Some(mut watch) = watch {
            let _ = watch.changed().await;
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
    /// The FR11 archive of this root: where a successor's identity record is written and the
    /// cross-major gate reads. Injected, not read from the process environment at each use, so
    /// two roots in one process never see each other's records.
    pub archive_root: std::path::PathBuf,
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
            archive_root: maos_audit::default_archive_dir(),
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
        artifact: crate::spirit_session::LaunchArtifact,
    ) -> Result<crate::admission::Admitted, crate::admission::AdmissionRefusal> {
        use crate::admission::{load_admit_start_unarchived, AdmissionRefusal, StartPolicy};
        let class_build = |detail: String| AdmissionRefusal::ClassBuild { detail };
        let supervisor =
            self.supervisor
                .upgrade()
                .ok_or(AdmissionRefusal::SpawnedSurfaceUnsupported {
                    surface: "operator door",
                })?;
        let launch = crate::spirit_session::manifest_launch_held(
            &*self.host,
            gated,
            manifest,
            origin,
            false,
            artifact,
        )
        .map_err(|error| class_build(error.to_string()))?;
        // The previous incarnation of this id has been unloaded and its session signalled, not
        // necessarily finished: until it releases its mailbox registration this admission loses.
        if tokio::time::timeout(
            SESSION_END_WAIT,
            supervisor.session_ends().ended(&gated.spirit_id),
        )
        .await
        .is_err()
        {
            return Err(class_build(format!(
                "session_not_ended: the previous session of '{}' did not finish tearing down within {} s",
                gated.spirit_id,
                SESSION_END_WAIT.as_secs()
            )));
        }
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
            &self.archive_root,
        )
        .await?;
        // The FR11 record precedes the start: a death inside the start window must not leave a
        // started version unrecorded. A component that never starts has it withdrawn again.
        let archive_root = self.archive_root.as_path();
        let provisional = crate::admission::archive_provisional(archive_root, gated);
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
        if provisional {
            crate::admission::withdraw_provisional(&self.archive_root, gated);
        }
        crate::admission::unwind_admitted(deps, admitted.pid, &gated.spirit_id).await;
        Err(class_build(format!("spawned_start_failed: {failure}")))
    }
}

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
    artifact: Option<crate::spirit_session::LaunchArtifact>,
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
    // D-21-PF-15 A / D-22-9-10 A: the door waits for the component's own lifecycle probe on
    // the one attempt deadline, never a retry the operator must issue.
    match target
        .lifecycle()
        .wait_supported(ATTEMPT_WALL.saturating_sub(began.elapsed()))
    {
        Some(true) => {}
        Some(false) => {
            return Err("spawned_swap_unsupported: the running Spirit exports no lifecycle".into())
        }
        None => {
            return Err(
                "spawned_swap_not_ready: the running Spirit did not finish starting within the attempt wall"
                    .into(),
            )
        }
    }
    // Each declared scope of the successor must be entitled (hierarchically, never widened) by
    // a scope the predecessor was admitted for; the successor then runs under exactly its
    // own declared scopes, a sandbox that is never wider than the predecessor's.
    let mut successor_scopes: Vec<Scope> = Vec::new();
    for scope in maos_manifest::capabilities_required_to_scopes(&gated.caps_required) {
        if !context.declared().iter().any(|held| held.entitles(&scope)) {
            return Err(format!(
                "swap_capability_widened: the successor declares {scope:?}, which this pid was not admitted for"
            ));
        }
        if !successor_scopes.contains(&scope) {
            successor_scopes.push(scope);
        }
    }
    let mut effective = context.sandbox.clone();
    effective.declared_scopes = successor_scopes;
    // Story 21-7: a swap keeps or narrows consent; the successor's declaration replaces the
    // predecessor's only at the committed handover.
    let session = (handles.session)(Arc::clone(&supervisor));
    crate::admission::check_consent_recipients(&gated, &|id| {
        crate::admission::host_knows(id, &|loaded| {
            supervisor.spawned_target(loaded).is_some()
                || session
                    .pid_by_spirit_id
                    .read()
                    .is_ok_and(|held| held.contains_key(loaded))
        })
    })
    .map_err(refused)?;
    let policy = Arc::clone(session.iac.consent_policy());
    let grants = gated.consent.as_ref().map(|c| c.recall.clone());
    if let Some(change) = policy.widened_by(&class.name, grants.as_ref()) {
        return Err(format!("swap_consent_widened: the successor {change}"));
    }
    let launch_origin = origin.unwrap_or_else(|| {
        path.parent()
            .map_or_else(|| ".".into(), std::path::Path::to_path_buf)
    });
    let artifact = artifact
        .ok_or("spawned_swap_launch_failed: no held component was acquired for this hop")?;
    let artifact_sha256 = artifact.sha256();
    let verified_publisher = artifact.publisher();
    // A same-pid swap keeps the Spirit's storage identity: a successor under a different
    // publisher (or signed vs unsigned) would silently re-key it, so it is refused.
    if verified_publisher != context.verified_publisher {
        return Err(crate::spirit_session::SWAP_PUBLISHER_CHANGED.into());
    }
    let launch = crate::spirit_session::manifest_launch_held(
        &*handles.host,
        &gated,
        &path,
        &launch_origin,
        false,
        artifact,
    )
    .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let (core, object) = supervisor
        .prepare_spawned(&class.name, manifest)
        .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    core.bind_scheduler_pid(target.spirit_pid());
    let (mut bridge, stop) = crate::spirit_session::spawn_runner(
        effective.clone(),
        launch.plan,
        launch.component_artifact,
        &class.name,
    )
    .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let writer = bridge
        .take_frame_writer()
        .map_err(|error| format!("spawned_swap_launch_failed: {error}"))?;
    let mut prewarm = Prewarm {
        bridge,
        stop,
        writer,
        turn_budget: launch.turn_budget,
        context_window: launch.context_window,
        scopes: launch.scopes,
        sandbox: effective.clone(),
        deadline: began + ATTEMPT_WALL,
        log: PrewarmLog {
            tl: Arc::clone(tl),
            spirit_id: class.name.clone(),
            pid: target.spirit_pid(),
            dropped: 0,
            budget: Default::default(),
        },
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
        sandbox: effective,
        origin: launch_origin,
        artifact_sha256: Some(artifact_sha256),
        verified_publisher,
    });
    let archive_root = handles.archive_root.clone();
    let (name, pid) = (class.name.clone(), target.spirit_pid());
    core.lifecycle()
        .attach_successor(prewarm, target.lifecycle(), move |committed| {
            if committed {
                policy.register(&name, pid, grants);
            }
            let outcome = if committed { "admitted" } else { "refused" };
            // The FR11 identity record of the version that actually took over the pid: only a
            // committed `swap-in` records it, the admitted successor's own manifest text.
            if committed {
                crate::admission::archive_admitted(&archive_root, &gated);
            }
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
        schedule: Arc<maos_kernel_core::scheduler::ScheduleWatchdog>,
        hooks: &crate::hook_pump::HookPump,
    ) -> Self {
        use maos_kernel_core::scheduler::IdleWatchdog;
        let cancel = tokio_util::sync::CancellationToken::new();
        let idle = Arc::new(IdleWatchdog::new(
            scheduler.scbs(),
            scheduler.dispatcher_arc(),
        ))
        .spawn(cancel.child_token());
        let schedule = schedule.spawn(cancel.child_token());
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
    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        for (name, handle) in std::mem::take(&mut self.watchdogs) {
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

/// A panic or early return that never reaches `shutdown` still stops every caller (the
/// consolidation watchdog cancels itself when dropped).
impl Drop for LifecycleCallers {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    /// Logical time: `t` ms past `base`. The write acks at `ack`, the poll answers `Idle` and
    /// advances to `idle_to`, or returns the scripted frame at `frame_at`.
    struct Scripted {
        base: Instant,
        now_ms: Cell<u64>,
        ack_ms: u64,
        idle_to_ms: u64,
        frame_at_ms: u64,
        reply: Vec<u8>,
        write_deadline: Option<Instant>,
        polls: Vec<Duration>,
    }

    impl StageIo for Scripted {
        fn now(&self) -> Instant {
            self.base + Duration::from_millis(self.now_ms.get())
        }

        fn write(&mut self, _body: &[u8], deadline: Instant) -> Result<(), String> {
            self.write_deadline = Some(deadline);
            self.now_ms.set(self.ack_ms);
            Ok(())
        }

        fn poll(&mut self, wait: Duration) -> Result<BridgePoll, String> {
            self.polls.push(wait);
            if self.polls.len() == 1 && self.frame_at_ms <= self.idle_to_ms {
                self.now_ms.set(self.frame_at_ms);
                return Ok(BridgePoll::Data(SubStream::Stdout, self.reply.clone()));
            }
            self.now_ms.set(self.idle_to_ms);
            Ok(BridgePoll::Idle)
        }
    }

    fn scripted(ack_ms: u64, idle_to_ms: u64, frame_at_ms: u64) -> Scripted {
        Scripted {
            base: Instant::now(),
            now_ms: Cell::new(0),
            ack_ms,
            idle_to_ms,
            frame_at_ms,
            reply: wire::encode_reply(&LifecycleReply::Supported(true)).unwrap(),
            write_deadline: None,
            polls: Vec::new(),
        }
    }

    fn run(io: &mut Scripted, cap_ms: u64, attempt_ms: u64) -> Result<LifecycleReply, String> {
        let attempt = io.base + Duration::from_millis(attempt_ms);
        exchange_on(
            io,
            &LifecycleRequest::Probe,
            Duration::from_millis(cap_ms),
            attempt,
        )
    }

    fn refused_after_shared_budget(io: &Scripted, result: Result<LifecycleReply, String>) {
        assert!(
            matches!(&result, Err(e) if e == DEADLINE),
            "late reply was not refused"
        );
        assert_eq!(
            io.write_deadline,
            Some(io.base + Duration::from_millis(12)),
            "write did not get the one min(stage, attempt) deadline"
        );
        assert_eq!(
            io.polls,
            vec![Duration::from_millis(3)],
            "reply poll was not given only what the write left, once"
        );
    }

    #[test]
    fn the_stage_cap_is_one_budget_for_the_write_then_the_reply_poll() {
        // Stage cap 12 ms binds under a 30 ms attempt; a valid reply would arrive at 13 ms.
        let mut io = scripted(9, 12, 13);
        let result = run(&mut io, 12, 30);
        refused_after_shared_budget(&io, result);
    }

    #[test]
    fn the_attempt_deadline_is_one_budget_for_the_write_then_the_reply_poll() {
        // Attempt wall 12 ms binds under a 30 ms stage cap.
        let mut io = scripted(9, 12, 13);
        let result = run(&mut io, 30, 12);
        refused_after_shared_budget(&io, result);
    }

    #[test]
    fn a_valid_reply_within_the_shared_budget_is_accepted() {
        let mut io = scripted(9, 12, 11);
        let result = run(&mut io, 12, 30);
        assert!(matches!(result, Ok(LifecycleReply::Supported(true))));
        // The reply poll, then the stage-end drain on the 1 ms the budget has left.
        assert_eq!(
            io.polls,
            vec![Duration::from_millis(3), Duration::from_millis(1)]
        );
    }

    #[test]
    fn a_frame_returned_past_the_shared_budget_is_refused_after_the_poll() {
        // The poll itself was granted 3 ms but returns the frame with time already at 13 ms.
        let mut io = scripted(9, 13, 13);
        let result = run(&mut io, 12, 30);
        refused_after_shared_budget(&io, result);
    }

    /// Frames in the order the two reader threads happened to enqueue them.
    struct Arrival {
        base: Instant,
        queue: std::collections::VecDeque<BridgePoll>,
        journaled: Vec<Vec<u8>>,
        missing: Vec<String>,
    }

    impl StageIo for Arrival {
        fn now(&self) -> Instant {
            self.base
        }

        fn write(&mut self, _body: &[u8], _deadline: Instant) -> Result<(), String> {
            Ok(())
        }

        fn poll(&mut self, _wait: Duration) -> Result<BridgePoll, String> {
            Ok(self.queue.pop_front().unwrap_or(BridgePoll::Closed))
        }

        fn stderr(&mut self, line: &[u8]) {
            self.journaled.push(line.to_vec());
        }

        fn stage_end_missing(&mut self, why: &str) {
            self.missing.push(why.into());
        }
    }

    fn arrival(order: Vec<BridgePoll>) -> Arrival {
        Arrival {
            base: Instant::now(),
            queue: order.into(),
            journaled: Vec::new(),
            missing: Vec::new(),
        }
    }

    fn stage_reply() -> BridgePoll {
        BridgePoll::Data(
            SubStream::Stdout,
            wire::encode_reply(&LifecycleReply::Supported(true)).unwrap(),
        )
    }

    fn stderr_line(line: &[u8]) -> BridgePoll {
        BridgePoll::Data(SubStream::Stderr, line.to_vec())
    }

    fn exchange(io: &mut Arrival) -> Result<LifecycleReply, String> {
        exchange_on(
            io,
            &LifecycleRequest::Probe,
            Duration::from_secs(1),
            io.base + Duration::from_secs(1),
        )
    }

    #[test]
    fn a_line_written_before_the_reply_is_journaled_whichever_arrived_first() {
        let sentinel = maos_frame_codec::STAGE_END_SENTINEL.as_bytes();
        // The stderr reader lost the race: the reply is in the channel ahead of the line the
        // guest wrote first, and of the sentinel the runner wrote right after it.
        let mut late = arrival(vec![
            stage_reply(),
            stderr_line(b"swap-in line"),
            stderr_line(sentinel),
        ]);
        assert!(matches!(
            exchange(&mut late),
            Ok(LifecycleReply::Supported(true))
        ));
        assert_eq!(late.journaled, vec![b"swap-in line".to_vec()]);
        assert!(late.missing.is_empty());
        // The reader that raced ahead by only the line: the reply sits between it and the sentinel.
        let mut middle = arrival(vec![
            stderr_line(b"swap-in line"),
            stage_reply(),
            stderr_line(sentinel),
        ]);
        assert!(matches!(
            exchange(&mut middle),
            Ok(LifecycleReply::Supported(true))
        ));
        assert_eq!(middle.journaled, late.journaled);
        assert!(middle.missing.is_empty());
        // The order the runner wrote them: line, sentinel, reply. Nothing is left to drain.
        let mut written = arrival(vec![
            stderr_line(b"swap-in line"),
            stderr_line(sentinel),
            stage_reply(),
        ]);
        assert!(matches!(
            exchange(&mut written),
            Ok(LifecycleReply::Supported(true))
        ));
        assert_eq!(written.journaled, late.journaled);
        assert!(written.missing.is_empty() && written.queue.is_empty());
    }

    #[test]
    fn the_stage_end_sentinel_is_never_a_diagnostic_but_an_unterminated_guest_line_is() {
        // The guest's last line had no LF, so the runner's sentinel joined it.
        let joined = [
            b"no newline".as_slice(),
            maos_frame_codec::STAGE_END_SENTINEL.as_bytes(),
        ]
        .concat();
        let mut io = arrival(vec![
            stage_reply(),
            stderr_line(b"first"),
            stderr_line(&joined),
        ]);
        assert!(exchange(&mut io).is_ok());
        assert_eq!(
            io.journaled,
            vec![b"first".to_vec(), b"no newline".to_vec()]
        );
    }

    #[test]
    fn a_missing_sentinel_never_fails_a_valid_reply() {
        let mut closed = arrival(vec![stage_reply(), stderr_line(b"before the child died")]);
        assert!(matches!(
            exchange(&mut closed),
            Ok(LifecycleReply::Supported(true))
        ));
        assert_eq!(closed.journaled, vec![b"before the child died".to_vec()]);
        assert_eq!(closed.missing, vec!["child_closed".to_string()]);
    }

    #[test]
    fn a_second_stdout_frame_after_the_reply_is_a_protocol_error() {
        let mut io = arrival(vec![stage_reply(), stage_reply()]);
        assert!(matches!(exchange(&mut io), Err(e) if e == "child_protocol"));
    }
    use super::*;

    /// A successor link wired behind `predecessor` without a child (no staged handover).
    fn bare_successor(predecessor: &Arc<LifecycleLink>) -> (Arc<LifecycleLink>, Arc<AtomicBool>) {
        let successor = LifecycleLink::new();
        let own = predecessor.begin_attempt();
        *locked(&successor.own) = Some(Arc::clone(&own));
        *locked(&successor.predecessor) = Some(Arc::clone(predecessor));
        (successor, own)
    }

    #[test]
    fn a_commit_with_nothing_staged_ends_its_attempt_and_unfences_the_predecessor() {
        let predecessor = LifecycleLink::new();
        let (successor, own) = bare_successor(&predecessor);
        predecessor.fence_for_snapshot(Some(&own));
        assert!(predecessor.fenced());
        successor.publish_commit();
        assert!(
            !predecessor.fenced(),
            "the commit left the predecessor fenced"
        );
        assert!(!own.load(Ordering::SeqCst), "the attempt was left live");
        // A second finalization is a no-op.
        successor.publish_commit();
        drop(successor);
        assert!(!predecessor.fenced());
    }

    #[test]
    fn a_commit_with_nothing_staged_never_touches_a_newer_attempt() {
        let predecessor = LifecycleLink::new();
        let (successor, old) = bare_successor(&predecessor);
        let newer = predecessor.begin_attempt();
        predecessor.fence_for_snapshot(Some(&newer));
        assert!(!old.load(Ordering::SeqCst));
        successor.publish_commit();
        assert!(
            predecessor.fenced() && newer.load(Ordering::SeqCst),
            "a stale commit cleared the newer attempt's fence"
        );
    }
}
