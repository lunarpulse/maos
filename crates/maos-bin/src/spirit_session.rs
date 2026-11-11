//! Governed, scheduler-owned sessions over the opaque subprocess frame bridge.
//! The caller admits exactly one SCB; this module adopts it, never loads another.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::{CapError, CapabilityRegistryPort};
use maos_host::SpiritLaunchPlan;
use maos_iac::adapter::mailbox::{Mailbox, SpiritMailboxHandle};
use maos_iac::adapter::transparency_log::{FrameKind as JournalKind, TransparencyLogAdapter};
use maos_iac::adapter::IacBusAdapter;
use maos_kernel_core::capability::{is_mediated_scope, CapabilityRegistryAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgePoll, BridgeSpawnSpec, ExitCause,
    SubStream,
};
use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use sha2::{Digest, Sha256};

use crate::admission::Admitted;
use crate::enterprise_identity::EnterpriseRuntime;
use crate::enterprise_pdp_runtime::EnterprisePdpRuntime;
use crate::recall_mediation::MediatedLogRecall;
use crate::spawned_lifecycle::{Answer, SwapContext};
use crate::supervision::{BindingCore, LiveWorker, WorkerSupervision, WorkerSupervisor};
use crate::worker_spawn::{issue_enterprise_governed_capability, GovernedMintError};
#[cfg(target_os = "linux")]
use maos_domain::invariants::i10::{
    JournalEntry, LifecycleEntry, LifecycleEvent, ResourceCeilingDowngradedPayload,
};
#[cfg(target_os = "linux")]
use maos_kernel_core::capability::cap_tokens::monotonic_now_ns;
use maos_wasm_host::lifecycle_wire::{self, LifecycleReply, LifecycleRequest};

/// Guest frames one turn may emit before the session is a protocol violation.
pub const MAX_FRAMES_PER_TURN: usize = 64;
/// Recall requests one active export may issue before the channel is closed.
pub const MAX_RECALLS_PER_TURN: usize = 64;
/// `TaskAssign` scopes one guest frame may request; more is a journaled denial.
pub const MAX_SCOPES_PER_FRAME: usize = 16;
/// Guest stderr rows journaled per session; the rest are dropped and counted.
pub const MAX_DIAGNOSTIC_ROWS: usize = 256;
/// Guest stderr bytes journaled per session; the rest are dropped and counted.
pub const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
/// Serialized guest-claims bytes kept verbatim in an authorization record.
pub const MAX_AUTHORIZATION_CLAIMS_BYTES: usize = 16 * 1024;
/// How long a session whose loop ended waits for the child to exit by itself
/// before it is stopped (planned) and, if still alive, killed.
pub const TERMINATION_GRACE: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct SessionDeps {
    pub supervisor: Arc<WorkerSupervisor>,
    pub capability: Arc<CapabilityRegistryAdapter>,
    pub security: Arc<SecurityManagerAdapter>,
    pub iac: Arc<IacBusAdapter>,
    pub journal: Arc<TransparencyLogAdapter>,
    pub log_recall: Arc<maos_iac::adapter::log_recall::LogRecallAdapter>,
    pub lifecycle_journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    pub pid_by_spirit_id: Arc<RwLock<BTreeMap<String, u32>>>,
    pub enterprise: Option<Arc<EnterpriseRuntime>>,
    pub pdp: Option<EnterprisePdpRuntime>,
    pub skill: Option<crate::skill_authoring::SkillBuilder>,
}

pub struct SessionLaunch {
    pub plan: SpiritLaunchPlan,
    /// Only the contained runner receives this read grant. It is not a policy grant.
    pub component_artifact: Option<PathBuf>,
    pub turn_budget: Duration,
    /// Story 18-5 — the admitted manifest's context window and thresholds.
    pub context_window: maos_kernel_core::capability::context_budget::ContextWindow,
    pub once: bool,
    pub initial_frame: Option<IacFrame>,
    pub scopes: Vec<maos_domain::invariants::i1::Scope>,
    /// The directory the manifest's relative `artifact` resolves against. A swap or cold swap
    /// of this Spirit launches its successor, and restores it, relative to the same place
    /// (Story 20-9): the door holds manifest bytes, not the operator's directory.
    pub origin: PathBuf,
}

/// Resolve metadata before admission; component validation stays in the T2 child.
pub fn manifest_launch(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    once: bool,
) -> Result<SessionLaunch, SessionError> {
    let origin = manifest.parent().unwrap_or(std::path::Path::new("."));
    manifest_launch_in(host, gated, manifest, origin, once)
}

/// The component a manifest's `artifact` names, resolved against `origin`. Only a relative path
/// whose real location stays inside `origin` resolves: an absolute path, a `..` climb, or a
/// symlink out of the directory is refused typed, so the runner's read grant and the code that
/// runs under the admitted sandbox are the operator's named directory's and nothing else's.
pub fn resolve_artifact(
    origin: &std::path::Path,
    artifact: &str,
) -> Result<std::path::PathBuf, SessionError> {
    let escape = |why: &str| SessionError::ArtifactEscapesOrigin(format!("{artifact:?} {why}"));
    let relative = std::path::Path::new(artifact);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(escape("is not a path inside the manifest's directory"));
    }
    let root = origin
        .canonicalize()
        .map_err(|error| SessionError::Bridge(format!("{}: {error}", origin.display())))?;
    let resolved = root
        .join(relative)
        .canonicalize()
        .map_err(|error| SessionError::Bridge(format!("{artifact:?}: {error}")))?;
    if resolved.starts_with(&root) {
        Ok(resolved)
    } else {
        Err(escape("resolves outside the manifest's directory"))
    }
}

/// [`manifest_launch`] with the artifact resolved against `origin`, not the manifest's own
/// directory: the door stages a successor's bytes in a private file, and its artifact stays
/// where the operator named it.
pub fn manifest_launch_in(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    origin: &std::path::Path,
    once: bool,
) -> Result<SessionLaunch, SessionError> {
    let artifact = gated
        .class_section
        .artifact
        .as_ref()
        .ok_or(SessionError::Protocol("WASM manifest has no artifact"))?;
    let artifact = resolve_artifact(origin, artifact)?;
    let plan = host
        .resolve_launch(&maos_host::SpiritLaunchRequest {
            form: maos_host::SpiritForm::WasmComponent,
            artifact: artifact.to_string_lossy().into_owned(),
            form_config: Vec::new(),
        })
        .map_err(|error| SessionError::Bridge(error.to_string()))?;
    let initial_frame = once.then(|| IacFrame {
        frame_id: ulid::Ulid::new().to_bytes(),
        timestamp_ns: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("operator"),
            host_id: None,
            role: None,
        },
        to: smallvec::smallvec![FrameAddress {
            spirit_id: SpiritId::from(gated.spirit_id.as_str()),
            host_id: None,
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(maos_domain::frame::TaskAssignPayload {
            goal: format!("maos run {} --once", manifest.display()),
            scope: Vec::new(),
            success_criteria: "Complete one mailbox export".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: Default::default(),
    });
    Ok(SessionLaunch {
        plan,
        component_artifact: Some(artifact),
        turn_budget: Duration::from_secs(u64::from(
            gated
                .budget
                .as_ref()
                .map_or(30, |budget| budget.time_cap_seconds),
        )),
        context_window: crate::butler_root::context_window(gated.budget.as_ref()),
        once,
        initial_frame,
        scopes: maos_manifest::capabilities_required_to_scopes(&gated.caps_required),
        origin: origin.to_path_buf(),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct SessionReport {
    pub spirit_pid: u32,
    pub child_pid: u32,
    pub turns: usize,
    pub delivered_frames: usize,
    pub denied_frames: usize,
    /// Guest stderr lines dropped after the per-session diagnostic budget ran out.
    pub dropped_diagnostics: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("spawned session requires an admitted T2 sandbox")]
    SandboxTier,
    #[error("session supervision failed: {0}")]
    Supervision(String),
    #[error("session bridge failed: {0}")]
    Bridge(String),
    #[error("session frame codec failed: {0}")]
    Codec(String),
    #[error("session mailbox failed: {0}")]
    Mailbox(String),
    #[error("intent promotion denied")]
    IntentPromotionDenied,
    #[error("session journal failed: {0}")]
    Journal(String),
    #[error("session token rollback failed: {0}")]
    Revoke(String),
    #[error("EOF before Ready or TurnComplete")]
    UnexpectedEof,
    #[error("session protocol violation: {0}")]
    Protocol(&'static str),
    #[error("session stopped by its supervisor")]
    Stopped,
    #[error("session {0} deadline exceeded")]
    Deadline(&'static str),
    #[error("session child terminated: {0:?}")]
    Child(ExitCause),
    #[error("incompatible_world: the runner refused a component importing an unsupported world")]
    IncompatibleWorld,
    #[error("invalid_component: the runner refused a non-conformant component")]
    InvalidComponent,
    #[error("unrepresentable_frame: the runner refused an inbound frame its world cannot express")]
    UnrepresentableFrame,
    #[error("swap_reverted_after_retire: the post-swap monitor restored a predecessor whose child was retired")]
    SwapReverted,
    #[error("artifact_escapes_origin: component artifact {0}")]
    ArtifactEscapesOrigin(String),
}

struct Recipient {
    mailbox: Arc<Mailbox>,
    handle: SpiritMailboxHandle,
    id: String,
}

impl Drop for Recipient {
    fn drop(&mut self) {
        while let Ok(Some(_)) = self.handle.try_recv() {}
        self.mailbox.unregister_spirit(&self.id);
    }
}

struct IdentityOwner {
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    id: String,
    pid: u32,
}

impl Drop for IdentityOwner {
    fn drop(&mut self) {
        let mut lookup = self
            .lookup
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if lookup.get(&self.id) == Some(&self.pid) {
            lookup.remove(&self.id);
        }
    }
}

/// What the kernel stamps onto a guest's emitted frames: the active turn's
/// inbound lineage, and a Lamport clock that outlives the turn.
#[derive(Default)]
struct TurnState {
    lineage: IntentLineage,
    inbound_clock: u64,
    frames: usize,
    recalls: usize,
    clock: u64,
}

/// Guest stderr rows/bytes already journaled this session.
#[derive(Default)]
struct DiagnosticBudget {
    rows: usize,
    bytes: usize,
}

/// Spawn the contained runner for `plan` under `sandbox`, plus a read grant on its artifact.
/// The session and the swap's pre-warmed successor start a child the same way.
pub(crate) fn spawn_runner(
    mut sandbox: maos_kernel_core::security::sandbox::SandboxSpec,
    plan: SpiritLaunchPlan,
    artifact: Option<&PathBuf>,
    spirit_id: &str,
) -> Result<maos_kernel_core::lifecycle::cli_wrapper::runtime::SpawnedBridge, SessionError> {
    if let Some(artifact) = artifact {
        sandbox.declared_scopes.push(Scope::FsRead {
            subtree: artifact.to_string_lossy().into_owned(),
        });
    }
    let prefix_hash = argv_prefix_hash(&plan.argv);
    spawn_and_bridge(BridgeSpawnSpec {
        program: plan.program,
        sandbox,
        argv_prefix: plan.argv,
        task_args: Vec::new(),
        expected_argv_prefix_hash: prefix_hash,
        from_spirit_id: spirit_id.to_owned(),
        stdio_shape: CliWrapperStdioShape::JsonRpcOverStdio,
        control_channel: CliWrapperControlChannel::Signals,
        shutdown_signal: None,
        channel_capacity: 1,
        backpressure: Backpressure::Block,
        env: plan.env,
        t3: None,
    })
    .map_err(|error| SessionError::Bridge(error.to_string()))
}

/// A committed swap retires the predecessor child at a planned stop. Its pid, tokens and
/// working memory now belong to the successor: nothing is revoked, and the SCB is not unloaded.
fn retire(deps: &SessionDeps, pid: u32, old: LiveWorker) {
    let LiveWorker {
        binding,
        mut bridge,
    } = old;
    let grace_end = Instant::now() + TERMINATION_GRACE;
    while matches!(bridge.try_exit_cause(), Ok(None)) && Instant::now() < grace_end {
        std::thread::sleep(Duration::from_millis(10));
    }
    if matches!(bridge.try_exit_cause(), Ok(None)) {
        binding.core().stop_session();
        let _ = bridge.kill();
    }
    let _ = bridge.wait_and_finalize(
        &deps.journal,
        pid,
        Some(deps.capability.audit_sender()),
        |_| {},
    );
    binding.release_for_swap();
}

/// One lifecycle export in flight on the child: its hook awaits `answer` (a probe has none).
struct Pending {
    answer: Option<std::sync::mpsc::SyncSender<Answer>>,
    /// The hook's call state: its reply is delivered, and a snapshot's fence set, only if the
    /// hook is still waiting when the reply arrives (a probe has no hook).
    state: Option<Arc<crate::spawned_lifecycle::CallState>>,
    request: LifecycleRequest,
}

/// How long after a committed swap the session watches for the post-swap monitor reverting it.
const REVERT_WATCH: Duration = Duration::from_secs(35);

/// Run the actual child on a blocking worker while root supervision remains live.
/// Every post-admission return owns an unload guard, including launch refusal.
pub async fn run_admitted_session(
    deps: SessionDeps,
    admitted: Admitted,
    core: Arc<BindingCore>,
    launch: SessionLaunch,
) -> Result<SessionReport, SessionError> {
    let owner = IdentityOwner {
        lookup: Arc::clone(&deps.pid_by_spirit_id),
        id: admitted.spirit_id.clone(),
        pid: admitted.pid,
    };
    let binding = deps
        .supervisor
        .adopt_spawned(core)
        .map_err(|error| SessionError::Supervision(error.to_string()))?;
    tokio::task::spawn_blocking(move || {
        let _owner = owner;
        if admitted.sandbox.tier != SandboxTier::T2 {
            return Err(SessionError::SandboxTier);
        }
        #[cfg(target_os = "linux")]
        let effective_tier = admitted.sandbox.tier;
        #[cfg(target_os = "linux")]
        let memory_max_mb = admitted.sandbox.resolved_caps.memory_max_mb;
        #[cfg(target_os = "linux")]
        let fd_max = admitted.sandbox.resolved_caps.fd_max;
        // Declared `iac.send` peer classes come from the kernel's own admission
        // result (the SCB adopted above), never from the policy table: the
        // composition root neither seeds nor reads the manifest-derived table.
        let declared_sends: Vec<Scope> = admitted.sandbox.declared_scopes.iter()
            .filter(|scope| matches!(scope, Scope::IacSend { .. })).cloned().collect();
        let swap_sandbox = admitted.sandbox.clone();
        let mut recipient = Recipient {
            mailbox: Arc::clone(deps.iac.mailbox()),
            handle: deps.iac.register_spirit_typed(&SpiritId::from(admitted.spirit_id.as_str()))
                .map_err(|error| SessionError::Mailbox(error.to_string()))?,
            id: admitted.spirit_id.clone(),
        };
        let bridge = spawn_runner(admitted.sandbox, launch.plan, launch.component_artifact.as_ref(),
            &admitted.spirit_id)?;
        let mut live = LiveWorker { binding, bridge };
        deps.supervisor.watch(live.binding.core(), live.bridge.child_pid());
        #[cfg(target_os = "linux")]
        if let Some(missing_mechanism) = live.bridge.take_resource_downgrade() {
            let payload = ResourceCeilingDowngradedPayload::new(
                missing_mechanism,
                memory_max_mb,
                fd_max,
            );
            let payload = serde_json::to_vec(&payload)
                .map_err(|error| SessionError::Journal(error.to_string()))?;
            deps.lifecycle_journal
                .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                    timestamp: monotonic_now_ns(),
                    lifecycle_event: LifecycleEvent::ResourceCeilingDowngraded,
                    spirit_id: admitted.spirit_id.clone(),
                    effective_sandbox_tier: Some(effective_tier),
                    payload: Some(payload),
                }));
        }
        let mut writer_thread = None;
        let mut leases = Vec::new();
        let mut report = SessionReport {
            spirit_pid: admitted.pid,
            child_pid: live.bridge.child_pid(),
            turns: 0,
            delivered_frames: 0,
            denied_frames: 0,
            dropped_diagnostics: 0,
        };
        let mut ready = false;
        let mut active = false;
        let mut link = Arc::clone(live.binding.core().lifecycle());
        link.attach_session(launch.turn_budget);
        link.set_swap_context(SwapContext { sandbox: swap_sandbox, origin: launch.origin.clone() });
        let mut pending: Option<Pending> = None;
        let mut turn_budget = launch.turn_budget;
        let mut swapped_at: Option<Instant> = None;
        let result = (|| {
            let mut writer = live.bridge.take_frame_writer()
                .map_err(|error| SessionError::Bridge(error.to_string()))?;
            let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);
            writer_thread = Some(std::thread::Builder::new()
                .name(format!("spirit-frame-writer-{}", admitted.pid))
                .spawn(move || {
                    for body in rx {
                        writer.write_frame(&body)?;
                    }
                    Ok::<(), std::io::Error>(())
                }).map_err(|error| SessionError::Bridge(error.to_string()))?);
            let mut tx = Some(tx);
            let mut draining = false;
            let mut initial_frame = launch.initial_frame;
            let mut seed_id = None;
            let mut turn = TurnState::default();
            let mut diagnostics = DiagnosticBudget::default();
            let mut recall = MediatedLogRecall::new(
                Arc::clone(&deps.log_recall), Arc::clone(&deps.capability),
                Arc::clone(&deps.security), Arc::clone(&deps.journal),
                deps.enterprise.clone(), deps.pdp.clone(), admitted.spirit_id.clone(),
            );
            let pid = admitted.pid;
            // Story 18-5 (CR-3) — the root fixes the session's pid and window; a committed
            // swap rebinds them to the successor's manifest.
            let bind_recall = |recall: &mut MediatedLogRecall,
                               window: maos_kernel_core::capability::context_budget::ContextWindow,
                               scopes: &[Scope]| -> Result<(), SessionError> {
                let spirit = maos_iac::adapter::context_signal::AdmittedSpirit {
                    pid, spirit_id: admitted.spirit_id.as_str().into(), window,
                };
                recall.context = Some(Arc::new(maos_iac::adapter::context_signal::ContextSignalAdapter::new(
                    Arc::clone(deps.capability.context_budget()), Arc::downgrade(&deps.iac),
                    Arc::clone(deps.iac.transparency_log()),
                    Arc::new(move || Some(spirit.clone())),
                )));
                recall.skill = deps.skill.as_ref()
                    .map(|bind| bind(Arc::new(move || Some(pid)), scopes))
                    .transpose().map_err(SessionError::Bridge)?;
                Ok(())
            };
            bind_recall(&mut recall, launch.context_window, &launch.scopes)?;
            let mut deadline = Some((Instant::now() + launch.turn_budget, "startup"));
            let runtime = tokio::runtime::Handle::current();
            loop {
                if deps.supervisor.is_stopping() {
                    return Err(SessionError::Stopped);
                }
                if let Some((until, stage)) = deadline {
                    if Instant::now() >= until {
                        if pending.is_some() {
                            journal_session_event(&deps, admitted.pid, &admitted.spirit_id,
                                "spirit.lifecycle.deadline", serde_json::json!({ "stage": stage }));
                        }
                        return Err(SessionError::Deadline(stage));
                    }
                }
                if !active && pending.is_none() {
                    if let Some(next) = link.take_handover() {
                        // The successor's `swap-in` committed (Story 20-9 AC4): its child now
                        // serves this pid. The predecessor child retires at a planned stop.
                        tx.take();
                        if let Some(thread) = writer_thread.take() {
                            let _ = thread.join();
                        }
                        let crate::spawned_lifecycle::Handover { core: next_core, prewarm } = next;
                        let next_writer = prewarm.writer;
                        let binding = deps.supervisor.adopt_spawned(Arc::clone(&next_core))
                            .map_err(|error| SessionError::Supervision(error.to_string()))?;
                        retire(&deps, admitted.pid, std::mem::replace(&mut live,
                            LiveWorker { binding, bridge: prewarm.bridge }));
                        deps.supervisor.watch(live.binding.core(), live.bridge.child_pid());
                        live.binding.core().mark_ready();
                        let (next_tx, next_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);
                        let mut next_writer = next_writer;
                        writer_thread = Some(std::thread::Builder::new()
                            .name(format!("spirit-frame-writer-{}", admitted.pid))
                            .spawn(move || {
                                for body in next_rx {
                                    next_writer.write_frame(&body)?;
                                }
                                Ok::<(), std::io::Error>(())
                            }).map_err(|error| SessionError::Bridge(error.to_string()))?);
                        tx = Some(next_tx);
                        link.close();
                        link = Arc::clone(next_core.lifecycle());
                        link.attach_session(prewarm.turn_budget);
                        link.set_supported(true);
                        turn_budget = prewarm.turn_budget;
                        bind_recall(&mut recall, prewarm.context_window, &prewarm.scopes)?;
                        report.child_pid = live.bridge.child_pid();
                        swapped_at = Some(Instant::now());
                    }
                }
                if swapped_at.is_some_and(|at| at.elapsed() < REVERT_WATCH)
                    && !deps.supervisor.holds_spirit_object(live.binding.core())
                {
                    journal_session_event(&deps, admitted.pid, &admitted.spirit_id,
                        "spirit.swap.reverted_after_retire", serde_json::json!({ "child_pid": report.child_pid }));
                    return Err(SessionError::SwapReverted);
                }
                if ready && !active && !draining && pending.is_none() && !link.fenced() {
                    if let Some(frame) = initial_frame.take() {
                        let id = frame.frame_id;
                        deliver_bounded(&deps, &runtime, Instant::now() + turn_budget,
                            deps.iac.deliver_typed(frame, 0, None))?;
                        seed_id = launch.once.then_some(id);
                        deadline = Some((Instant::now() + turn_budget, "seed turn"));
                    }
                    if let Some((_, frame)) = recipient.handle.try_recv()
                        .map_err(|error| SessionError::Mailbox(error.to_string()))?
                    {
                        if seed_id.is_some_and(|id| id != frame.frame_id) {
                            // A `--once` session's only turn is the operator seed.
                            journal_undeliverable(&deps, admitted.pid, &admitted.spirit_id,
                                frame.frame_id, "once session accepts only its operator seed");
                        } else {
                            match maos_wasm_host::codec::encode_cbor(&frame) {
                                Err(error) => journal_undeliverable(&deps, admitted.pid,
                                    &admitted.spirit_id, frame.frame_id, &error),
                                Ok(body) => {
                                    deps.supervisor.begin_turn(live.binding.core(),
                                        format!("turn:{}", ulid::Ulid::from_bytes(frame.frame_id)))
                                        .map_err(|error| SessionError::Supervision(error.to_string()))?;
                                    active = true;
                                    turn.lineage = frame.intent_lineage;
                                    turn.inbound_clock = frame.logical_clock;
                                    turn.frames = 0;
                                    turn.recalls = 0;
                                    deadline = Some((Instant::now() + turn_budget, "active turn"));
                                    tx.as_ref()
                                        .ok_or(SessionError::Protocol("session stdin already closed"))?
                                        .send(body)
                                        .map_err(|error| SessionError::Bridge(error.to_string()))?;
                                }
                            }
                        }
                    }
                }
                // A queued frame turn goes first: a busy trigger caller must not starve the mailbox.
                if ready && !active && !draining && pending.is_none() {
                    if let Some(call) = link.take() {
                        if link.fenced() {
                            // After a swap's `snapshot` the predecessor does no more work.
                            let _ = call.answer.send(Answer::Reply(LifecycleReply::Unsupported));
                        } else {
                            let body = lifecycle_wire::encode_request(&call.request)
                                .map_err(SessionError::Codec)?;
                            turn.recalls = 0;
                            deadline = Some((Instant::now() + turn_budget, "lifecycle export"));
                            tx.as_ref()
                                .ok_or(SessionError::Protocol("session stdin already closed"))?
                                .send(body)
                                .map_err(|error| SessionError::Bridge(error.to_string()))?;
                            pending = Some(Pending { answer: Some(call.answer), state: Some(call.state), request: call.request });
                        }
                    }
                }
                match live.bridge.poll_frame(Duration::from_millis(20))
                    .map_err(|error| SessionError::Bridge(error.to_string()))?
                {
                    BridgePoll::Idle => {}
                    BridgePoll::Closed => {
                        if !ready || active || pending.is_some() {
                            return Err(SessionError::UnexpectedEof);
                        }
                        if launch.once && report.turns == 0 {
                            return Err(SessionError::Protocol("once session ended before its turn"));
                        }
                        return Ok(());
                    }
                    BridgePoll::Data(SubStream::Stderr, bytes) => {
                        journal_diagnostic(&deps, &admitted.spirit_id, admitted.pid,
                            &mut report, &mut diagnostics, &bytes);
                    }
                    BridgePoll::Data(SubStream::Stdout, bytes) if bytes.is_empty() => {
                        if !ready {
                            ready = true;
                            // The probe below sets the next deadline.
                            live.binding.core().mark_ready();
                            println!("{}", serde_json::json!({
                                "event": "spirit_loaded", "spirit_id": admitted.spirit_id,
                                "pid": admitted.pid, "child_pid": report.child_pid, "ready": true,
                            }));
                            let probe = lifecycle_wire::encode_request(&LifecycleRequest::Probe)
                                .map_err(SessionError::Codec)?;
                            tx.as_ref()
                                .ok_or(SessionError::Protocol("session stdin already closed"))?
                                .send(probe)
                                .map_err(|error| SessionError::Bridge(error.to_string()))?;
                            pending = Some(Pending { answer: None, state: None, request: LifecycleRequest::Probe });
                            deadline = Some((Instant::now() + turn_budget, "lifecycle probe"));
                        } else if active {
                            deps.supervisor.complete_turn(live.binding.core());
                            revoke(&deps.capability, &mut leases)?;
                            active = false;
                            report.turns += 1;
                            deadline = None;
                            if launch.once {
                                draining = true;
                                tx.take();
                                deadline = Some((Instant::now() + turn_budget, "shutdown"));
                            }
                        } else {
                            return Err(SessionError::Protocol("unexpected empty control frame"));
                        }
                    }
                    BridgePoll::Data(SubStream::Stdout, bytes) => {
                        if lifecycle_wire::is_reply(&bytes) {
                            let Some(done) = pending.take() else {
                                return Err(SessionError::Protocol("lifecycle reply without a request"));
                            };
                            let reply = lifecycle_wire::decode_reply(&bytes).map_err(SessionError::Codec)?;
                            deadline = None;
                            // A hook that already stopped waiting leaves its reply unconsumed.
                            let wanted = match &done.state {
                                Some(state) => state.answer(),
                                None => true,
                            };
                            match (&done.request, &reply) {
                                (LifecycleRequest::Probe, LifecycleReply::Supported(supported)) =>
                                    link.set_supported(*supported),
                                (LifecycleRequest::Probe, _) =>
                                    return Err(SessionError::Protocol("unexpected probe reply")),
                                // The predecessor's state is now taken: no more work until the swap
                                // resolves. A snapshot its caller gave up on fences nothing.
                                (LifecycleRequest::Snapshot, LifecycleReply::State(_)) if wanted =>
                                    link.fence_for_snapshot(),
                                _ => {}
                            }
                            if let Some(answer) = done.answer.filter(|_| wanted) {
                                let _ = answer.send(Answer::Reply(reply));
                            }
                            continue;
                        }
                        if maos_wasm_host::recall_wire::is_tagged(&bytes) {
                            let in_export = active
                                || pending.as_ref().is_some_and(|p| p.request.imports_allowed());
                            let reply = if !ready || !in_export {
                                Err(crate::recall_mediation::ProtocolViolation("recall outside an active export"))
                            } else if turn.recalls >= MAX_RECALLS_PER_TURN {
                                Err(crate::recall_mediation::ProtocolViolation("recall budget exceeded"))
                            } else {
                                turn.recalls += 1;
                                recall.serve_wire(admitted.pid, &bytes)
                            };
                            match reply {
                                Err(error) => {
                                    recall.channel_closed(admitted.pid, error.0);
                                    return Err(SessionError::Protocol(error.0));
                                }
                                Ok(body) => tx.as_ref()
                                    .ok_or(SessionError::Protocol("session stdin already closed"))?
                                    .send(body)
                                    .map_err(|error| SessionError::Bridge(error.to_string()))?,
                            }
                            continue;
                        }
                        if !ready || !active {
                            return Err(SessionError::Protocol("data outside an active export"));
                        }
                        turn.frames += 1;
                        if turn.frames > MAX_FRAMES_PER_TURN {
                            return Err(SessionError::Protocol("frame budget exceeded"));
                        }
                        let frame: IacFrame = maos_wasm_host::codec::decode_cbor(&bytes)
                            .map_err(SessionError::Codec)?;
                        let until = deadline
                            .ok_or(SessionError::Protocol("active export has no deadline"))?
                            .0;
                        if authorize_and_deliver(&deps, &runtime, admitted.pid,
                            &admitted.spirit_id, &declared_sends, frame, until, &mut leases,
                            &mut turn)?
                        {
                            report.delivered_frames += 1;
                        } else {
                            report.denied_frames += 1;
                        }
                    }
                }
            }
        })();
        if let Some(Pending { answer: Some(answer), .. }) = pending.take() {
            let _ = answer.send(Answer::SessionEnded);
        }
        link.close();
        // Whatever ended the loop, the child gets a bounded grace to leave by
        // itself (its stdin closed with the loop), then a planned stop, then a
        // kill: `wait_and_finalize` below can never block forever.
        let grace_end = Instant::now() + TERMINATION_GRACE;
        let exited = loop {
            if !matches!(live.bridge.try_exit_cause(), Ok(None)) {
                break true;
            }
            if Instant::now() >= grace_end {
                break false;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !exited {
            live.binding.core().stop_session();
            let _ = live.bridge.kill();
        }
        let revoke_result = revoke(&deps.capability, &mut leases);
        let exit = live.bridge.wait_and_finalize(&deps.journal, admitted.pid,
            Some(deps.capability.audit_sender()), |_| {
                deps.capability.revoke_all_for_pid(admitted.pid);
                deps.capability.context_budget().remove(admitted.pid);
            });
        let writer_result = match writer_thread {
            Some(thread) => thread.join()
                .map_err(|_| SessionError::Bridge("frame writer panicked".into()))
                .and_then(|result| result.map_err(|error| SessionError::Bridge(error.to_string()))),
            None => Ok(()),
        };
        live.binding.finish(&exit);
        if let Err(error) = result {
            if matches!(error, SessionError::UnexpectedEof) {
                if !ready {
                    // The runner's typed refusals are launch refusals, not crashes.
                    match exit.cause {
                        ExitCause::Exited { code: 2 } => return Err(SessionError::IncompatibleWorld),
                        ExitCause::Exited { code: 3 } => return Err(SessionError::InvalidComponent),
                        ExitCause::Exited { code: 5 } => return Err(SessionError::UnrepresentableFrame),
                        _ => {}
                    }
                }
                if exit.cause.is_crash() {
                    return Err(SessionError::Child(exit.cause));
                }
            }
            return Err(error);
        }
        revoke_result?;
        writer_result?;
        if exit.cause != (ExitCause::Exited { code: 0 }) {
            return Err(SessionError::Child(exit.cause));
        }
        Ok(report)
    }).await.map_err(|error| SessionError::Supervision(error.to_string()))?
}

/// Revoke every lease. A token that is already revoked (or no longer known)
/// is the state this wants, not a failure.
fn revoke(
    capability: &CapabilityRegistryAdapter,
    leases: &mut Vec<TokenId>,
) -> Result<(), SessionError> {
    let mut failure = None;
    for token in leases.drain(..) {
        match capability.revoke(token) {
            Ok(()) | Err(CapError::Revoked | CapError::UnknownToken) => {}
            Err(error) => {
                failure.get_or_insert_with(|| SessionError::Revoke(error.to_string()));
            }
        }
    }
    failure.map_or(Ok(()), Err)
}

/// Roll the frame's tokens back on an error path; the primary error wins over
/// any rollback failure.
fn fail(
    capability: &CapabilityRegistryAdapter,
    issued: &mut Vec<TokenId>,
    error: SessionError,
) -> SessionError {
    let _ = revoke(capability, issued);
    error
}

/// Drive one bus delivery to completion within the turn's remaining time,
/// noticing a supervisor stop while it waits.
fn deliver_bounded<T>(
    deps: &SessionDeps,
    runtime: &tokio::runtime::Handle,
    deadline: Instant,
    work: impl Future<Output = Result<T, maos_domain::iac_bus_types::IacBusError>>,
) -> Result<T, SessionError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    runtime.block_on(async {
        let work = tokio::time::timeout(remaining, work);
        tokio::pin!(work);
        let mut tick = tokio::time::interval(Duration::from_millis(25));
        loop {
            tokio::select! {
                done = &mut work => {
                    return match done {
                        Ok(result) => result.map_err(|error| match error {
                            maos_domain::iac_bus_types::IacBusError::IntentPromotionDenied { .. } =>
                                SessionError::IntentPromotionDenied,
                            other => SessionError::Mailbox(other.to_string()),
                        }),
                        Err(_) => Err(SessionError::Deadline("delivery")),
                    };
                }
                _ = tick.tick() => {
                    if deps.supervisor.is_stopping() {
                        return Err(SessionError::Stopped);
                    }
                }
            }
        }
    })
}

/// A frame the kernel will not deliver, journaled instead of silently dropped.
fn journal_undeliverable(
    deps: &SessionDeps,
    pid: u32,
    spirit_id: &str,
    frame_id: [u8; 16],
    reason: &str,
) {
    let record = serde_json::json!({
        "event": "spirit_frame_undeliverable",
        "spirit_pid": pid,
        "frame_id": ulid::Ulid::from_bytes(frame_id).to_string(),
        "reason": reason,
    });
    let _ = deps.journal.insert_frame_event_with_sender(
        JournalKind::GovernanceEvent,
        pid,
        spirit_id,
        "",
        None,
        "spirit.frame.undeliverable",
        record.to_string().as_bytes(),
        FrameOrigin::Kernel,
    );
}

/// One journaled row for a session event that ends or changes the session (Story 20-9).
fn journal_session_event(
    deps: &SessionDeps,
    pid: u32,
    spirit_id: &str,
    intent: &str,
    record: serde_json::Value,
) {
    let _ = deps.journal.insert_frame_event_with_sender(
        JournalKind::GovernanceEvent,
        pid,
        spirit_id,
        "",
        None,
        intent,
        record.to_string().as_bytes(),
        FrameOrigin::Kernel,
    );
}

/// Journal one guest stderr line while the session's row/byte budget lasts;
/// afterwards count the drop, with one marker row when dropping starts.
fn journal_diagnostic(
    deps: &SessionDeps,
    spirit_id: &str,
    pid: u32,
    report: &mut SessionReport,
    budget: &mut DiagnosticBudget,
    line: &[u8],
) {
    let record =
        if budget.rows >= MAX_DIAGNOSTIC_ROWS || line.len() > MAX_DIAGNOSTIC_BYTES - budget.bytes {
            report.dropped_diagnostics += 1;
            if report.dropped_diagnostics > 1 {
                return;
            }
            serde_json::json!({
                "event": "spirit_diagnostic_dropped",
                "child_pid": report.child_pid,
                "max_rows": MAX_DIAGNOSTIC_ROWS,
                "max_bytes": MAX_DIAGNOSTIC_BYTES,
            })
        } else {
            budget.rows += 1;
            budget.bytes += line.len();
            serde_json::json!({
                "event": "spirit_diagnostic",
                "child_pid": report.child_pid,
                "line": String::from_utf8_lossy(line),
            })
        };
    let _ = deps.journal.insert_frame_event_with_sender(
        JournalKind::CliSubprocessOutput,
        pid,
        spirit_id,
        "",
        None,
        "spirit.diagnostic",
        record.to_string().as_bytes(),
        FrameOrigin::Kernel,
    );
}

#[derive(serde::Serialize)]
struct ScopeDecision<'a> {
    scope: &'a Scope,
    kernel_verdict: Option<&'static str>,
    refusal: Option<String>,
}

/// The guest's own claims, verbatim while small; otherwise only their size and digest.
#[derive(serde::Serialize)]
#[serde(untagged)]
enum Claims<'a> {
    Frame(&'a IacFrame),
    Truncated {
        truncated: bool,
        bytes: usize,
        sha256: String,
    },
}

#[derive(serde::Serialize)]
struct AuthorizationRecord<'a> {
    event: &'static str,
    spirit_pid: u32,
    trusted_sender: &'a str,
    trusted_intent: &'static str,
    trusted_origin: &'static str,
    allowed: bool,
    /// Why the whole frame was refused before any scope was considered.
    refusal: Option<&'static str>,
    /// The `iac.send` mediation of the delivery itself.
    send: Option<&'a ScopeDecision<'a>>,
    decisions: &'a [ScopeDecision<'a>],
    claims: Claims<'a>,
}

/// Mint one governed token (enterprise PDP, then kernel policy) for `scope`
/// and record its verdict; a granted token joins `issued`.
fn mint<'a>(
    deps: &SessionDeps,
    pid: u32,
    posture_hash: [u8; 32],
    scope: &'a Scope,
    issued: &mut Vec<TokenId>,
) -> ScopeDecision<'a> {
    match issue_enterprise_governed_capability(
        &deps.capability,
        deps.enterprise.as_deref(),
        deps.pdp.as_ref(),
        pid,
        scope.clone(),
        60,
        posture_hash,
        IntentClass::Standard,
    ) {
        Ok(token) => {
            issued.push(token.token_id);
            ScopeDecision {
                scope,
                kernel_verdict: Some("allow"),
                refusal: None,
            }
        }
        Err(error) => ScopeDecision {
            scope,
            // Story 17-9: the kernel evaluated the scope and either denied it
            // or refused it for a spent issuance window; any other refusal
            // (SSO, PDP, mediation) never reached a kernel verdict.
            kernel_verdict: match error {
                GovernedMintError::KernelPolicyDenied => Some("deny"),
                GovernedMintError::QuotaExhausted => Some("exhausted"),
                GovernedMintError::ContextExhausted => Some("context-exhausted"),
                _ => None,
            },
            refusal: Some(error.to_string()),
        },
    }
}

/// The `iac.send` peer class a delivery exercises: a frame with no recipient
/// is a broadcast; an addressed frame reaches Spirit peers.
fn send_scope(frame: &IacFrame) -> Scope {
    Scope::IacSend {
        peer_class: if frame.to.is_empty() {
            "broadcast"
        } else {
            "spirit:peer"
        }
        .into(),
    }
}

/// Frame-level refusals that need no kernel mediation to decide. Same-Spirit
/// addressing stays legal (ADR-018); bounded delivery keeps it from wedging.
fn frame_refusal(frame: &IacFrame) -> Option<&'static str> {
    if frame.to.iter().any(|address| address.host_id.is_some()) {
        return Some("cross_host_destination");
    }
    if matches!(
        frame.kind,
        FrameKind::BudgetWarning
            | FrameKind::BudgetExceeded
            | FrameKind::ConsentRupture
            | FrameKind::RateLimited
            | FrameKind::ContextPressure
            | FrameKind::ContextLimit
    ) {
        return Some("kernel_reserved_kind");
    }
    match &frame.payload {
        FramePayload::TaskAssign(task) if task.scope.len() > MAX_SCOPES_PER_FRAME => {
            Some("scope_limit")
        }
        _ => None,
    }
}

/// A guest `Retract` may only undo a frame this very Spirit sent.
fn retract_refusal(
    deps: &SessionDeps,
    spirit_id: &str,
    retract: &maos_domain::frame::RetractPayload,
) -> Result<Option<&'static str>, SessionError> {
    if maos_domain::frame::RetractPayload::new(
        retract.original_frame_id,
        retract.reason.clone(),
        None,
    )
    .is_err()
    {
        return Ok(Some("retract_reason_too_long"));
    }
    let original = deps
        .journal
        .query_frame_by_id(retract.original_frame_id)
        .map_err(|error| SessionError::Journal(error.to_string()))?;
    Ok(match original {
        None => Some("retract_original_not_found"),
        Some(entry) if entry.from_spirit_id != spirit_id => Some("retract_authority_violation"),
        Some(_) => None,
    })
}

#[allow(clippy::too_many_arguments)]
fn authorize_and_deliver(
    deps: &SessionDeps,
    runtime: &tokio::runtime::Handle,
    pid: u32,
    spirit_id: &str,
    declared_sends: &[Scope],
    mut frame: IacFrame,
    deadline: Instant,
    leases: &mut Vec<TokenId>,
    turn: &mut TurnState,
) -> Result<bool, SessionError> {
    let expected_kind = match &frame.payload {
        FramePayload::TaskAssign(_) => FrameKind::TaskAssign,
        FramePayload::TaskComplete(_) => FrameKind::TaskComplete,
        FramePayload::DecisionDispatch(_) => FrameKind::DecisionDispatch,
        FramePayload::EpistemicHalt(_) => FrameKind::EpistemicHalt,
        FramePayload::TelemetryEvent(_) => FrameKind::TelemetryEvent,
        FramePayload::ConsentRequest(_) => FrameKind::ConsentRequest,
        FramePayload::Retract(_) => FrameKind::Retract,
        FramePayload::BudgetWarning(_) => FrameKind::BudgetWarning,
        FramePayload::BudgetExceeded(_) => FrameKind::BudgetExceeded,
        FramePayload::ConsentRupture(_) => FrameKind::ConsentRupture,
        FramePayload::RateLimited(_) => FrameKind::RateLimited,
        FramePayload::ContextPressure(_) => FrameKind::ContextPressure,
        FramePayload::ContextLimit(_) => FrameKind::ContextLimit,
    };
    if frame.kind != expected_kind {
        return Err(SessionError::Protocol(
            "frame kind does not match its payload",
        ));
    }
    let policy = deps.security.policy().inner().load_full();
    let posture_hash = policy
        .spirit_postures
        .get(&pid)
        .ok_or(SessionError::Protocol("admitted posture is absent"))?
        .posture_hash();
    let mut issued = Vec::new();
    let mut decisions = Vec::new();
    let mut refusal = frame_refusal(&frame);
    if refusal.is_none() {
        if let FramePayload::Retract(retract) = &frame.payload {
            refusal = retract_refusal(deps, spirit_id, retract)?;
        }
    }
    let mut allowed = refusal.is_none();
    // `iac.send` mediation (FR4): every delivery carries its own kernel token.
    // A declared peer class must match; undeclared `iac.send` is the kernel's
    // own deny.
    let send_scope = send_scope(&frame);
    let mut send = None;
    let mut send_token = None;
    if allowed {
        let decision = if declared_sends.is_empty() || declared_sends.contains(&send_scope) {
            mint(deps, pid, posture_hash, &send_scope, &mut issued)
        } else {
            ScopeDecision {
                scope: &send_scope,
                kernel_verdict: None,
                refusal: Some("iac_peer_class_not_granted".into()),
            }
        };
        allowed = decision.kernel_verdict == Some("allow");
        send_token = allowed.then(|| issued.last().copied()).flatten();
        send = Some(decision);
    }
    if allowed {
        if let FramePayload::TaskAssign(task) = &frame.payload {
            // Root-only authoring and unmediated scopes are never delegated.
            if let Some(scope) = task
                .scope
                .iter()
                .find(|scope| matches!(scope, Scope::SkillAuthorSelf) || !is_mediated_scope(scope))
            {
                decisions.push(ScopeDecision {
                    scope,
                    kernel_verdict: Some(if matches!(scope, Scope::SkillAuthorSelf) {
                        "nondelegable"
                    } else {
                        "unmediated"
                    }),
                    refusal: Some("scope cannot be delegated through TaskAssign".into()),
                });
                allowed = false;
            } else {
                for scope in &task.scope {
                    if Instant::now() >= deadline || deps.supervisor.is_stopping() {
                        return Err(fail(
                            &deps.capability,
                            &mut issued,
                            SessionError::Deadline("scope authorization"),
                        ));
                    }
                    let decision = mint(deps, pid, posture_hash, scope, &mut issued);
                    allowed = decision.kernel_verdict == Some("allow");
                    decisions.push(decision);
                    if !allowed {
                        break;
                    }
                }
            }
        }
    }
    let claims_bytes = serde_json::to_vec(&frame).map_err(|error| {
        fail(
            &deps.capability,
            &mut issued,
            SessionError::Journal(error.to_string()),
        )
    })?;
    let truncated = || Claims::Truncated {
        truncated: true,
        bytes: claims_bytes.len(),
        sha256: hex::encode(Sha256::digest(&claims_bytes)),
    };
    let record = |decisions: &[ScopeDecision<'_>], claims| -> Result<Vec<u8>, SessionError> {
        let mut body = maos_frame_codec::BodyBuffer::default();
        serde_json::to_writer(
            &mut body,
            &AuthorizationRecord {
                event: "spawned_frame_authorization",
                spirit_pid: pid,
                trusted_sender: spirit_id,
                trusted_intent: "standard",
                trusted_origin: "SpiritAuto",
                allowed,
                refusal,
                send: send.as_ref(),
                decisions,
                claims,
            },
        )
        .map_err(|error| SessionError::Journal(error.to_string()))?;
        Ok(body.into_bytes())
    };
    let claims = if claims_bytes.len() <= MAX_AUTHORIZATION_CLAIMS_BYTES {
        Claims::Frame(&frame)
    } else {
        truncated()
    };
    // An unrecordable verdict must not abort the session: fall back to a
    // record that carries only the verdict and the claims' digest.
    let body = match record(&decisions, claims) {
        Ok(body) => body,
        Err(_) => {
            record(&[], truncated()).map_err(|error| fail(&deps.capability, &mut issued, error))?
        }
    };
    // Log before deliver: the verdict row is durable before anything moves (I2:
    // the log write halts the kernel rather than return without the row).
    let _ = deps.journal.insert_frame_event_with_sender(
        JournalKind::GovernanceEvent,
        pid,
        spirit_id,
        "",
        None,
        "spirit.frame.authorization",
        &body,
        FrameOrigin::Kernel,
    );
    if !allowed {
        revoke(&deps.capability, &mut issued)?;
        return Ok(false);
    }
    let delivery = if let FramePayload::Retract(retract) = &frame.payload {
        deliver_bounded(
            deps,
            runtime,
            deadline,
            deps.iac.retract(
                retract.original_frame_id,
                retract.reason.clone(),
                &SpiritId::from(spirit_id),
                send_token,
            ),
        )
        .map(drop)
    } else {
        frame.frame_id = ulid::Ulid::new().to_bytes();
        frame.timestamp_ns = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
        frame.from = FrameAddress {
            spirit_id: SpiritId::from(spirit_id),
            host_id: None,
            role: None,
        };
        frame.intent = IntentClass::Standard;
        frame.auto_marker = FrameOrigin::SpiritAuto;
        frame.intent_lineage = turn.lineage.clone();
        frame.consent_envelope = None;
        turn.clock = turn.clock.max(turn.inbound_clock) + 1;
        frame.logical_clock = turn.clock;
        deliver_bounded(
            deps,
            runtime,
            deadline,
            deps.iac.deliver_typed(frame, pid, send_token),
        )
        .map(drop)
    };
    if matches!(delivery, Err(SessionError::IntentPromotionDenied)) {
        revoke(&deps.capability, &mut issued)?;
        return Ok(false);
    }
    if let Err(error) = delivery {
        return Err(fail(&deps.capability, &mut issued, error));
    }
    if leases.is_empty() {
        *leases = issued;
    } else {
        leases.extend(issued);
    }
    Ok(true)
}
