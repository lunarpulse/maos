//! Spawned-form lifecycle triggers and the state-preserving swap (Story 20-9).
//!
//! A `WorkerSpirit` hook sends one [`LifecycleRequest`] through its [`LifecycleLink`], a bounded
//! slot its session drains one export at a time. The pre-warmed successor of a swap is the
//! exception: it has no session yet, so its `migrate` and `swap-in` run inline on its own child,
//! and a committed `swap-in` hands that child to the predecessor's session.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::Scope;
use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
    BridgePoll, FrameWriter, SpawnedBridge, SubStream,
};
use maos_kernel_core::security::sandbox::SandboxSpec;
use maos_wasm_host::lifecycle_wire::{self as wire, LifecycleReply, LifecycleRequest};

use crate::supervision::BindingCore;

/// What a hook learns when its session did not answer.
pub const SESSION_ENDED: &str = "session_ended";
pub const DEADLINE: &str = "deadline";

pub enum Answer {
    Reply(LifecycleReply),
    SessionEnded,
}

/// One queued export. `cancelled` is set by a hook that stopped waiting.
pub struct Call {
    pub request: LifecycleRequest,
    pub answer: SyncSender<Answer>,
    pub cancelled: Arc<AtomicBool>,
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
}

/// A committed successor: the predecessor's session adopts it.
pub struct Handover {
    pub core: Arc<BindingCore>,
    pub prewarm: Prewarm,
}

impl Prewarm {
    /// One request/reply on a child nobody else is serving. Stderr is drained and dropped.
    pub fn exchange(
        &mut self,
        request: &LifecycleRequest,
        cap: Duration,
    ) -> Result<LifecycleReply, String> {
        let body = wire::encode_request(request).map_err(|error| format!("encode: {error}"))?;
        self.writer
            .write_frame(&body)
            .map_err(|error| format!("child_write: {error}"))?;
        let deadline = Instant::now() + cap;
        loop {
            match self
                .bridge
                .poll_frame(Duration::from_millis(20))
                .map_err(|error| format!("child_poll: {error}"))?
            {
                BridgePoll::Idle if Instant::now() >= deadline => return Err(DEADLINE.into()),
                BridgePoll::Idle | BridgePoll::Data(SubStream::Stderr, _) => {}
                BridgePoll::Closed => return Err("child_closed".into()),
                BridgePoll::Data(SubStream::Stdout, bytes) if wire::is_reply(&bytes) => {
                    return wire::decode_reply(&bytes).map_err(|error| format!("decode: {error}"))
                }
                BridgePoll::Data(SubStream::Stdout, _) => return Err("child_protocol".into()),
            }
        }
    }

    /// Wait for the runner's empty `Ready` control frame.
    pub fn await_ready(&mut self, cap: Duration) -> Result<(), String> {
        let deadline = Instant::now() + cap;
        loop {
            match self
                .bridge
                .poll_frame(Duration::from_millis(20))
                .map_err(|error| format!("child_poll: {error}"))?
            {
                BridgePoll::Idle if Instant::now() >= deadline => return Err(DEADLINE.into()),
                BridgePoll::Idle | BridgePoll::Data(SubStream::Stderr, _) => {}
                BridgePoll::Closed => return Err("child_closed".into()),
                BridgePoll::Data(SubStream::Stdout, bytes) if bytes.is_empty() => return Ok(()),
                BridgePoll::Data(SubStream::Stdout, _) => return Err("child_protocol".into()),
            }
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
    fenced: AtomicBool,
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
            fenced: AtomicBool::new(false),
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
            let reply = reply?;
            if matches!(request, LifecycleRequest::SwapIn { .. }) && reply == LifecycleReply::Done {
                self.commit(core);
            }
            return Ok(reply);
        }
        // A core no session drives (a `[cli_wrapper]` Worker, or a spawned core before its
        // session starts) has nothing to run: its triggers are no-ops, as they were.
        if !self.attached.load(Ordering::Acquire) || *locked(&self.supported) == Some(false) {
            return Ok(LifecycleReply::Unsupported);
        }
        let deadline = Instant::now() + cap;
        let (answer, reply) = sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut call = Call {
            request,
            answer,
            cancelled: Arc::clone(&cancelled),
        };
        loop {
            if self.closed.load(Ordering::Acquire) {
                return Err(SESSION_ENDED.into());
            }
            match self.tx.try_send(call) {
                Ok(()) => break,
                Err(TrySendError::Full(back)) if Instant::now() < deadline => {
                    call = back;
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TrySendError::Full(_)) => return Err(DEADLINE.into()),
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
                Err(RecvTimeoutError::Timeout) if self.closed.load(Ordering::Acquire) => {
                    cancelled.store(true, Ordering::Release);
                    return Err(SESSION_ENDED.into());
                }
                Err(RecvTimeoutError::Timeout) if Instant::now() >= deadline => {
                    cancelled.store(true, Ordering::Release);
                    return Err(DEADLINE.into());
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// The successor's `swap-in` succeeded: its child now belongs to the predecessor's session.
    fn commit(&self, core: &Arc<BindingCore>) {
        let Some(predecessor) = locked(&self.predecessor).take() else {
            return;
        };
        if let Some(prewarm) = locked(&self.prewarm).take() {
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
            if !call.cancelled.load(Ordering::Acquire) {
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
        *locked(&self.predecessor) = Some(Arc::clone(predecessor));
        *locked(&self.outcome) = Some(Box::new(outcome));
    }
}

impl Drop for LifecycleLink {
    /// A successor that is dropped before its `swap-in` committed releases its predecessor.
    fn drop(&mut self) {
        if let Some(predecessor) = locked(&self.predecessor).take() {
            predecessor.set_fenced(false);
        }
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

type SessionBuilder = Arc<
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
    /// The spawned session of `spirit_id` a swap would replace, if there is one.
    pub fn target(&self, spirit_id: &str) -> Option<Arc<BindingCore>> {
        self.supervisor.upgrade()?.spawned_target(spirit_id)
    }

    /// A cold swap's admission of a component (D-20-9-12 A): the one spawned admission path
    /// `InstalledSpirits::boot` uses, at a new pid, with its session hosted by this root.
    pub async fn admit_spawned(
        &self,
        deps: &crate::admission::AdmissionDeps<'_>,
        boot_nonce: u64,
        gated: &crate::admission::GatedManifest,
        manifest: &std::path::Path,
        origin: &std::path::Path,
    ) -> Result<crate::admission::Admitted, crate::admission::AdmissionRefusal> {
        use crate::admission::{load_admit_start, AdmissionRefusal, StartPolicy};
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
        let admitted = load_admit_start(
            deps,
            gated,
            object,
            boot_nonce,
            StartPolicy::StartNow,
            &mut |pid| bound.bind_scheduler_pid(pid),
        )
        .await?;
        let session = (self.session)(supervisor);
        let (id, hold) = (gated.spirit_id.clone(), self.hosted.hold());
        let started = admitted.clone();
        tokio::spawn(async move {
            let _hold = hold;
            if let Err(error) =
                crate::spirit_session::run_admitted_session(session, started, core, launch).await
            {
                eprintln!("maos: spirit '{id}' session ended: {error}");
            }
        });
        Ok(admitted)
    }
}

/// How long a successor's pre-warm waits for the predecessor's own lifecycle probe.
const PROBE_WAIT: Duration = Duration::from_secs(5);

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
