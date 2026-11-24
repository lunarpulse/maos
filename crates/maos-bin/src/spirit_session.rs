//! Governed, scheduler-owned sessions over the opaque subprocess frame bridge.
//! The caller admits exactly one SCB; this module adopts it, never loads another.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
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
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgePoll, BridgeSpawnSpec, ChildStopHandle,
    ChildStopRefusal, ExitCause, FrameWriter, SpawnedBridge, SubStream,
};
use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};
use maos_kernel_core::security::sandbox::{HeldComponentError, HeldWasmComponent};
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
    /// The one held, read-only, regular component file this launch hands to its runner.
    /// `None` only for a child that is not a WASM component. Never cloned, never a path.
    pub component_artifact: Option<LaunchArtifact>,
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

/// How an installed Spirit's session tells the host owner it is ready, binds the one current
/// shell `TaskAssign` to its live child, and reports the turn's end. Created by the host owner
/// and handed to [`run_admitted_shell_session`]; every field is host-side, never guest input.
pub struct ShellPort {
    state: Mutex<PortState>,
    changed: Condvar,
    serial: Mutex<()>,
    turn_budget: Duration,
}

enum Phase {
    Starting,
    Ready { pid: u32, generation: u64 },
    Ended,
}

struct PortState {
    phase: Phase,
    slot: Option<ShellSlot>,
}

/// A shell request queued for delivery, waiting for the session to dequeue its exact frame.
struct ShellSlot {
    id: [u8; 16],
    sender: String,
    pid: u32,
    generation: u64,
    deadline: Instant,
    route: maos_iac::adapter::mailbox::ShellReplyRoute,
    done: std::sync::mpsc::SyncSender<()>,
}

/// The session's view of the one bound shell turn.
struct ShellBinding {
    id: [u8; 16],
    deadline: Instant,
    /// Taken by the first shell-addressed `TaskComplete`; later ones are refused.
    route: Option<maos_iac::adapter::mailbox::ShellReplyRoute>,
    done: std::sync::mpsc::SyncSender<()>,
}

/// Marks the port ended when the session's worker leaves, whatever the cause.
struct ShellEnd(Option<Arc<ShellPort>>);

impl Drop for ShellEnd {
    fn drop(&mut self) {
        if let Some(port) = &self.0 {
            let mut state = port.lock();
            state.phase = Phase::Ended;
            state.slot = None;
            port.changed.notify_all();
        }
    }
}

impl ShellPort {
    pub fn new(turn_budget: Duration) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(PortState {
                phase: Phase::Starting,
                slot: None,
            }),
            changed: Condvar::new(),
            serial: Mutex::new(()),
            turn_budget,
        })
    }

    pub fn turn_budget(&self) -> Duration {
        self.turn_budget
    }

    /// One shell turn at a time owns the port.
    pub fn serialize(&self) -> std::sync::MutexGuard<'_, ()> {
        self.serial
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PortState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn ready(&self, pid: u32, generation: u64) {
        let mut state = self.lock();
        if !matches!(state.phase, Phase::Ended) {
            state.phase = Phase::Ready { pid, generation };
            self.changed.notify_all();
        }
    }

    /// Wait, within the bounded startup (Ready, then the startup probe, each one turn budget),
    /// for the session's own readiness signal; returns the admitted PID and live generation.
    pub fn await_ready(&self) -> Result<(u32, u64), TurnError> {
        let until = Instant::now() + self.turn_budget * 2;
        let mut state = self.lock();
        loop {
            match state.phase {
                Phase::Ready { pid, generation } => return Ok((pid, generation)),
                Phase::Ended => return Err(TurnError::StartupEnded),
                Phase::Starting => {}
            }
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(TurnError::StartupDeadline);
            }
            state = self
                .changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner())
                .0;
        }
    }

    /// Queue one shell request before its frame is delivered. `id` is the host-minted frame ID
    /// of the outgoing `TaskAssign`; the session binds only that exact inbound frame.
    pub fn arm(
        self: &Arc<Self>,
        id: [u8; 16],
        sender: &str,
        pid: u32,
        generation: u64,
        deadline: Instant,
    ) -> Result<ShellTurn, TurnError> {
        self.arm_labelled(
            maos_iac::adapter::mailbox::ReplyLabel::Shell,
            id,
            sender,
            pid,
            generation,
            deadline,
        )
    }

    /// [`Self::arm`] for a caller whose reply row the audit trail labels (Story 22-4).
    pub fn arm_labelled(
        self: &Arc<Self>,
        label: maos_iac::adapter::mailbox::ReplyLabel,
        id: [u8; 16],
        sender: &str,
        pid: u32,
        generation: u64,
        deadline: Instant,
    ) -> Result<ShellTurn, TurnError> {
        let mut state = self.lock();
        match state.phase {
            Phase::Ready {
                pid: live,
                generation: current,
            } if live == pid && current == generation => {}
            Phase::Ended => return Err(TurnError::SessionEnded),
            _ => return Err(TurnError::SessionChanged),
        }
        if state.slot.is_some() {
            return Err(TurnError::Busy);
        }
        let (route, receiver) = maos_iac::adapter::mailbox::labelled_reply_route(
            label,
            id,
            SpiritId::from(sender),
            pid,
            deadline,
        );
        let (done, finished) = std::sync::mpsc::sync_channel(1);
        state.slot = Some(ShellSlot {
            id,
            sender: sender.to_owned(),
            pid,
            generation,
            deadline,
            route,
            done,
        });
        Ok(ShellTurn {
            port: Arc::clone(self),
            id,
            pid,
            generation,
            deadline,
            receiver,
            finished,
        })
    }

    /// Bind the dequeued `frame` if (and only if) it is the one queued shell request and the
    /// session is still the admitted PID/generation it was queued for. `Err` skips the frame.
    fn claim(
        &self,
        frame: &IacFrame,
        pid: u32,
        generation: u64,
    ) -> Result<Option<ShellBinding>, &'static str> {
        if frame.from.spirit_id.as_str() != maos_iac::adapter::mailbox::SHELL_REPLY_DESTINATION {
            return Ok(None);
        }
        let mut state = self.lock();
        let Some(slot) = state.slot.take_if(|slot| slot.id == frame.frame_id) else {
            return Err("shell turn is no longer active");
        };
        let one_target = frame.to.len() == 1
            && frame.to.iter().all(|to| {
                to.spirit_id.as_str() == slot.sender && to.host_id.is_none() && to.role.is_none()
            });
        let local_shell = frame.from.host_id.is_none() && frame.from.role.is_none();
        if frame.kind != FrameKind::TaskAssign
            || !local_shell
            || !one_target
            || !matches!(frame.auto_marker, FrameOrigin::HumanAuthored)
            || !matches!(frame.intent, IntentClass::Standard)
            || slot.pid != pid
            || slot.generation != generation
            || Instant::now() >= slot.deadline
        {
            return Err("shell turn binding mismatch");
        }
        Ok(Some(ShellBinding {
            id: slot.id,
            deadline: slot.deadline,
            route: Some(slot.route),
            done: slot.done,
        }))
    }
}

/// Why a queued turn produced no answer. `Display` is the text the shell REPL has always
/// printed; [`Self::reason`] is the typed reason a directed task records (Story 22-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnError {
    /// The session ended before it signalled readiness.
    StartupEnded,
    /// The session did not signal readiness inside its bounded startup.
    StartupDeadline,
    /// The session had ended when the turn was queued.
    SessionEnded,
    /// The live pid/generation moved before the turn was queued.
    SessionChanged,
    /// Another turn already holds the port's one slot.
    Busy,
    /// The turn's absolute deadline passed.
    DeadlineExceeded,
    /// The session ended while the turn was in flight.
    EndedDuring,
    /// The session was swapped to another generation while the turn was in flight.
    ChangedDuring,
    /// The Spirit finished its turn without an accepted answer.
    NoAnswer,
}

impl TurnError {
    /// The typed task reason (`reason` of `GET /v1/tasks/{id}`).
    pub fn reason(self) -> &'static str {
        match self {
            Self::StartupEnded | Self::SessionEnded | Self::EndedDuring => "session_ended",
            Self::StartupDeadline | Self::DeadlineExceeded => "deadline_exceeded",
            Self::SessionChanged | Self::ChangedDuring => "session_changed",
            Self::Busy => "dispatch_refused",
            Self::NoAnswer => "no_answer",
        }
    }
}

impl std::fmt::Display for TurnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::StartupEnded => "session ended before it was ready",
            Self::StartupDeadline => "session startup deadline exceeded",
            Self::SessionEnded => "session ended",
            Self::SessionChanged => "session generation changed before the turn was queued",
            Self::Busy => "a shell turn is already queued",
            Self::DeadlineExceeded => "turn deadline exceeded",
            Self::EndedDuring | Self::ChangedDuring => {
                "session ended or changed before the turn completed"
            }
            Self::NoAnswer => "spirit completed its turn without returning an answer",
        })
    }
}

/// The caller's end of one queued turn. Dropping it retires the request and closes the
/// reply route, so nothing late can satisfy it.
pub struct ShellTurn {
    port: Arc<ShellPort>,
    id: [u8; 16],
    pid: u32,
    generation: u64,
    deadline: Instant,
    receiver: maos_iac::adapter::mailbox::ShellReplyReceiver,
    finished: std::sync::mpsc::Receiver<()>,
}

impl ShellTurn {
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Hold the routed answer until the session reports the matching clean `TurnComplete`,
    /// then release it once. Timeout, session end or a turn with no accepted answer is an error.
    pub fn wait(mut self) -> Result<String, TurnError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        match self.finished.recv_timeout(remaining) {
            // A clean turn observed past the turn's own deadline publishes nothing.
            Ok(()) if Instant::now() >= self.deadline => {
                self.receiver.close();
                Err(TurnError::DeadlineExceeded)
            }
            Ok(()) => self
                .receiver
                .try_recv()
                .map(|answer| answer.result)
                .ok_or(TurnError::NoAnswer),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.receiver.close();
                Err(TurnError::DeadlineExceeded)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                self.receiver.close();
                let moved = match self.port.lock().phase {
                    Phase::Ready { pid, generation } => {
                        pid != self.pid || generation != self.generation
                    }
                    _ => false,
                };
                Err(if moved {
                    TurnError::ChangedDuring
                } else {
                    TurnError::EndedDuring
                })
            }
        }
    }
}

impl Drop for ShellTurn {
    fn drop(&mut self) {
        let mut state = self.port.lock();
        if state.slot.as_ref().is_some_and(|slot| slot.id == self.id) {
            state.slot = None;
        }
    }
}

/// Bind the dequeued frame to the host-queued shell turn, if it is that turn's exact frame.
/// `Some` is the reason the frame must be skipped.
fn claim_shell_turn(
    port: Option<&ShellPort>,
    frame: &IacFrame,
    pid: u32,
    generation: u64,
    bound: &mut Option<ShellBinding>,
) -> Option<&'static str> {
    match port?.claim(frame, pid, generation) {
        Ok(found) => {
            if found.is_some() {
                *bound = found;
            }
            None
        }
        Err(reason) => Some(reason),
    }
}

/// The only frame shape the scoped route carries: one `TaskComplete` addressed to the shell.
fn is_shell_reply(frame: &IacFrame) -> bool {
    frame.kind == FrameKind::TaskComplete
        && frame.to.len() == 1
        && frame.to.iter().all(|to| {
            to.spirit_id.as_str() == maos_iac::adapter::mailbox::SHELL_REPLY_DESTINATION
                && to.host_id.is_none()
                && to.role.is_none()
        })
}

#[cfg(test)]
mod shell_turn_tests {
    use super::*;
    use maos_iac::adapter::mailbox::SHELL_REPLY_DESTINATION;

    fn request(id: [u8; 16], sender: &str) -> IacFrame {
        let address = |name: &str| FrameAddress {
            spirit_id: SpiritId::from(name),
            host_id: None,
            role: None,
        };
        IacFrame {
            frame_id: id,
            timestamp_ns: 0,
            logical_clock: 0,
            from: address(sender),
            to: smallvec::smallvec![address("example")],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(maos_domain::frame::TaskAssignPayload {
                goal: "hello".into(),
                scope: Vec::new(),
                success_criteria: String::new(),
                posture_preferences: Default::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: Default::default(),
        }
    }

    #[test]
    fn late_shell_request_does_not_take_the_next_turn_after_handover() {
        let budget = Duration::from_secs(30);
        let port = ShellPort::new(budget);
        let deadline = Instant::now() + budget;
        port.ready(7, 1);
        let first = [1; 16];
        drop(port.arm(first, "example", 7, 1, deadline).unwrap());

        port.ready(7, 2);
        let next = [2; 16];
        let _turn = port.arm(next, "example", 7, 2, deadline).unwrap();
        assert!(matches!(
            port.claim(&request(first, SHELL_REPLY_DESTINATION), 7, 2),
            Err("shell turn is no longer active")
        ));
        assert!(matches!(
            port.claim(&request(next, "operator"), 7, 2),
            Ok(None)
        ));
        assert!(matches!(
            port.claim(&request(next, SHELL_REPLY_DESTINATION), 7, 2),
            Ok(Some(binding)) if binding.id == next
        ));
    }
}

/// Custody of the component a launch runs: the exact open read-only regular file, bound to the
/// SHA-256 of its bytes. Non-cloneable and one-shot: [`spawn_runner`] hands it to the child and
/// closes the parent's copy; a pathname is only ever an acquisition input, never a recovery one.
#[derive(Debug)]
pub struct LaunchArtifact {
    held: HeldWasmComponent,
    sha256: [u8; 32],
    /// The publisher a signed verification (or a restore of one) bound these bytes to; set
    /// only by [`Self::from_verified`] and [`Self::restored_under`], never by a caller.
    publisher: Option<[u8; 32]>,
}

const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

/// The refusal for a same-pid swap whose successor was verified under a different publisher.
pub const SWAP_PUBLISHER_CHANGED: &str = "swap_publisher_changed: the successor's verified \
     publisher differs from the one this pid was launched under";

fn held_error(error: HeldComponentError) -> SessionError {
    match error {
        HeldComponentError::TransportUnsupported => {
            SessionError::WasmTransportUnsupported(error.to_string())
        }
        other => SessionError::HeldComponent(other.to_string()),
    }
}

impl LaunchArtifact {
    /// SHA-256 of the exact bytes the runner will compile (it re-hashes its compile buffer).
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    /// Acquire the plain local component `artifact` names under the trusted `origin`, once.
    /// Only a relative path is accepted; every segment is opened with no symlink following, so
    /// neither a `..` climb, an absolute path, nor a symlink can leave `origin`. `pin` is the
    /// digest the bytes must have (a signed or previously launched component), else refused.
    pub fn acquire_local(
        origin: &std::path::Path,
        artifact: &str,
        pin: Option<[u8; 32]>,
    ) -> Result<Self, SessionError> {
        let file = open_inside(origin, artifact)?;
        Self::hold(file, pin)
    }

    /// [`Self::acquire_local`] for a caller that names a file, with its directory as the origin.
    pub fn acquire_local_file(path: &std::path::Path) -> Result<Self, SessionError> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| SessionError::Bridge(format!("{}: not a file path", path.display())))?;
        let origin = path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        Self::acquire_local(origin, name, None)
    }

    /// Take the registry verifier's already-open component for the gated manifest. The signed
    /// receipt (id, version, manifest digest) must match what was gated; the runner compiles
    /// only bytes that hash to the signed artifact digest.
    pub fn from_verified(
        component: maos_registry::install::VerifiedLaunchComponent,
        gated: &crate::admission::GatedManifest,
    ) -> Result<Self, SessionError> {
        let manifest_digest: [u8; 32] = Sha256::digest(gated.manifest_toml.as_bytes()).into();
        if component.spirit_id().as_str() != gated.spirit_id
            || component.version() != gated.class_section.version
            || component.manifest_sha256() != manifest_digest
        {
            return Err(SessionError::ArtifactDigestMismatch(
                "the signed receipt differs from the gated manifest".into(),
            ));
        }
        if gated
            .publisher
            .is_some_and(|bound| bound != component.publisher())
        {
            return Err(SessionError::ArtifactDigestMismatch(
                "the signed receipt's publisher differs from the gated manifest's".into(),
            ));
        }
        let publisher = Some(component.publisher());
        let (file, sha256) = component.into_parts();
        let held = HeldWasmComponent::new(file, sha256).map_err(held_error)?;
        Ok(Self {
            held,
            sha256,
            publisher,
        })
    }

    /// Cold restore of a previously launched incarnation: the exact bytes `recorded_sha256`
    /// (the digest its swap context recorded) re-enter under the publisher that incarnation
    /// was verified under. Nothing else can attach a publisher to a locally held artifact, and
    /// only for exactly those bytes.
    pub fn restored_under(
        mut self,
        publisher: [u8; 32],
        recorded_sha256: [u8; 32],
    ) -> Result<Self, SessionError> {
        if self.sha256 != recorded_sha256 || self.publisher.is_some_and(|held| held != publisher) {
            return Err(SessionError::ArtifactDigestMismatch(
                "the restored component is not the recorded signed incarnation".into(),
            ));
        }
        self.publisher = Some(publisher);
        Ok(self)
    }

    /// The publisher this artifact was verified (or restored) under; `None` for local bytes.
    pub fn publisher(&self) -> Option<[u8; 32]> {
        self.publisher
    }

    fn hold(mut file: std::fs::File, pin: Option<[u8; 32]>) -> Result<Self, SessionError> {
        use std::io::{Read, Seek, SeekFrom};
        let refuse = |why: String| SessionError::HeldComponent(why);
        let metadata = file.metadata().map_err(|e| refuse(e.to_string()))?;
        if !metadata.is_file() {
            return Err(refuse("component is not a regular file".into()));
        }
        if metadata.len() > MAX_ARTIFACT_BYTES {
            return Err(refuse(format!(
                "component exceeds the {MAX_ARTIFACT_BYTES}-byte cap"
            )));
        }
        let mut hasher = Sha256::new();
        // The streaming cap below still bounds a file that grows while it is read.
        let mut buffer = [0u8; 8 * 1024];
        let mut total = 0u64;
        loop {
            let read = match file.read(&mut buffer) {
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(refuse(format!("read: {error}"))),
            };
            if read == 0 {
                break;
            }
            total += read as u64;
            if total > MAX_ARTIFACT_BYTES {
                return Err(refuse(format!(
                    "component exceeds the {MAX_ARTIFACT_BYTES}-byte cap"
                )));
            }
            hasher.update(&buffer[..read]);
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|error| refuse(format!("rewind: {error}")))?;
        let sha256: [u8; 32] = hasher.finalize().into();
        if pin.is_some_and(|pin| pin != sha256) {
            return Err(SessionError::ArtifactDigestMismatch(
                "the component's bytes differ from the digest they were pinned to".into(),
            ));
        }
        let held = HeldWasmComponent::new(file, sha256).map_err(held_error)?;
        Ok(Self {
            held,
            sha256,
            publisher: None,
        })
    }
}

#[cfg(unix)]
fn open_inside(origin: &std::path::Path, artifact: &str) -> Result<std::fs::File, SessionError> {
    use rustix::fs::{fcntl_setfl, openat, Mode, OFlags};
    use rustix::io::Errno;
    let escape = |why: &str| SessionError::ArtifactEscapesOrigin(format!("{artifact:?} {why}"));
    let segments: Vec<&str> = artifact
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    if artifact.starts_with('/')
        || artifact.contains('\0')
        || segments.is_empty()
        || segments.contains(&"..")
    {
        return Err(escape("is not a path inside the manifest's directory"));
    }
    let unavailable = |error: Errno| {
        if matches!(error, Errno::LOOP | Errno::NOTDIR) {
            escape("resolves outside the manifest's directory")
        } else {
            SessionError::Bridge(format!("{artifact:?}: {error}"))
        }
    };
    let mut current: std::os::fd::OwnedFd = rustix::fs::open(
        origin,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| SessionError::Bridge(format!("{}: {error}", origin.display())))?;
    let last = segments.len() - 1;
    for (index, segment) in segments.iter().enumerate() {
        let flags = if index == last {
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK
        } else {
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
        };
        current = openat(&current, *segment, flags, Mode::empty()).map_err(unavailable)?;
    }
    let file = std::fs::File::from(current);
    // NONBLOCK only guarded the open against special files; the held handle is shared.
    fcntl_setfl(&file, OFlags::empty())
        .map_err(|error| SessionError::Bridge(format!("{artifact:?}: {error}")))?;
    Ok(file)
}

#[cfg(not(unix))]
fn open_inside(_origin: &std::path::Path, _artifact: &str) -> Result<std::fs::File, SessionError> {
    Err(SessionError::WasmTransportUnsupported(
        "no defensive component acquisition on this platform".into(),
    ))
}

/// Acquire the component a staged manifest declares, relative to `origin` (`pin` as in
/// [`LaunchArtifact::acquire_local`]). `None` when the manifest declares no WASM component.
pub fn acquire_staged_artifact(
    manifest: &std::path::Path,
    origin: &std::path::Path,
    pin: Option<[u8; 32]>,
) -> Result<Option<LaunchArtifact>, SessionError> {
    let text = crate::admission::read_manifest(manifest)
        .map_err(|error| SessionError::Bridge(error.to_string()))?;
    let gated = crate::admission::gate_manifest(&text, &|_| false)
        .map_err(|error| SessionError::Bridge(error.to_string()))?;
    if gated.class_section.forms != ["wasm-component"] {
        return Ok(None);
    }
    let artifact = declared_artifact(&gated)?;
    LaunchArtifact::acquire_local(origin, artifact, pin).map(Some)
}

fn declared_artifact(gated: &crate::admission::GatedManifest) -> Result<&str, SessionError> {
    gated
        .class_section
        .artifact
        .as_deref()
        .ok_or(SessionError::Protocol("WASM manifest has no artifact"))
}

impl From<SessionError> for crate::admission::AdmissionRefusal {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::WasmTransportUnsupported(detail) => {
                Self::WasmTransportUnsupported { detail }
            }
            other => Self::ClassBuild {
                detail: other.to_string(),
            },
        }
    }
}

/// A signed registry package beside `manifest` binds the gated manifest's publisher and yields
/// the component the verifier compared against the signed bytes, held. `Ok(None)` for a plain
/// local manifest. The signed receipt must match the gated text, version and id.
pub fn bind_registry_launch(
    gated: &mut crate::admission::GatedManifest,
    manifest: &std::path::Path,
) -> Result<Option<LaunchArtifact>, crate::admission::AdmissionRefusal> {
    use crate::admission::AdmissionRefusal;
    let Some((pkg, component)) =
        maos_registry::install::signed_package_at_launch(&gated.spirit_id, manifest)
            .map_err(|error| AdmissionRefusal::from_registry_error(&*error))?
    else {
        return Ok(None);
    };
    if pkg.manifest_toml != gated.manifest_toml.as_bytes()
        || pkg.version != gated.class_section.version
    {
        return Err(AdmissionRefusal::Admission {
            detail: "registry manifest snapshot differs from package".into(),
        });
    }
    gated.bind_registry_publisher(&pkg.publisher_pubkey);
    component
        .map(|component| LaunchArtifact::from_verified(component, gated))
        .transpose()
        .map_err(Into::into)
}

/// Resolve metadata before admission and acquire the plain local component once, under the
/// manifest's own directory. A registry-bound manifest never launches from a pathname.
pub fn manifest_launch(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    once: bool,
) -> Result<SessionLaunch, SessionError> {
    let origin = manifest.parent().unwrap_or(std::path::Path::new("."));
    manifest_launch_in(host, gated, manifest, origin, once)
}

/// [`manifest_launch`] with the artifact acquired under `origin`, not the manifest's own
/// directory: the door stages a successor's bytes in a private file, and its artifact stays
/// where the operator named it.
pub fn manifest_launch_in(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    origin: &std::path::Path,
    once: bool,
) -> Result<SessionLaunch, SessionError> {
    if gated.publisher.is_some() {
        return Err(SessionError::SignedLaunchNeedsHeldArtifact);
    }
    let artifact = LaunchArtifact::acquire_local(origin, declared_artifact(gated)?, None)?;
    manifest_launch_held(host, gated, manifest, origin, once, artifact)
}

/// [`manifest_launch`] for a caller that may already hold the verified component (`maos run`
/// of an installed package): `Some` is used as is, `None` acquires the local component.
pub fn manifest_launch_for(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    once: bool,
    held: Option<LaunchArtifact>,
) -> Result<SessionLaunch, SessionError> {
    match held {
        Some(artifact) => {
            let origin = manifest.parent().unwrap_or(std::path::Path::new("."));
            manifest_launch_held(host, gated, manifest, origin, once, artifact)
        }
        None => manifest_launch(host, gated, manifest, once),
    }
}

/// The launch for an artifact the caller already holds. `origin` is kept for later swaps only;
/// this launch never reopens anything from it.
pub fn manifest_launch_held(
    host: &dyn maos_host::SpiritHostPort,
    gated: &crate::admission::GatedManifest,
    manifest: &std::path::Path,
    origin: &std::path::Path,
    once: bool,
    artifact: LaunchArtifact,
) -> Result<SessionLaunch, SessionError> {
    let declared = declared_artifact(gated)?.to_owned();
    let plan = host
        .resolve_launch(&maos_host::SpiritLaunchRequest {
            form: maos_host::SpiritForm::WasmComponent,
            artifact: declared,
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
    /// The executing lifecycle export outlived its hook's link deadline: the hook gave up and
    /// the session ended itself, the child killed and finalized as for any other deadline.
    #[error("session {0} link deadline exceeded")]
    LinkDeadline(&'static str),
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
    #[error("artifact_digest_mismatch: {0}")]
    ArtifactDigestMismatch(String),
    #[error("signed_launch_needs_held_artifact: a registry-bound component launches only from its verified held file, never a pathname")]
    SignedLaunchNeedsHeldArtifact,
    #[error("wasm_transport_unsupported: {0}")]
    WasmTransportUnsupported(String),
    #[error("held_component_refused: {0}")]
    HeldComponent(String),
    #[error("lifecycle_stop_unavailable: {0}")]
    ChildStop(#[from] ChildStopRefusal),
}

/// What a session's sends, task scopes and recall are held to. The first incarnation is
/// admitted by the kernel (an empty send list defers to its mint policy, as before). After an
/// outer-committed swap the authority is the successor's own declared scopes and ENFORCED
/// here: the kernel's per-pid policy is not narrowed by a session, so an empty or narrower list
/// is a denial, never a delegation to the (wider) predecessor policy.
struct Authority {
    declared: Vec<Scope>,
    enforced: bool,
}

impl Authority {
    fn admitted(sandbox: &maos_kernel_core::security::sandbox::SandboxSpec) -> Self {
        Self {
            declared: sandbox.declared_scopes.clone(),
            enforced: false,
        }
    }

    fn successor(sandbox: &maos_kernel_core::security::sandbox::SandboxSpec) -> Self {
        Self {
            declared: sandbox.declared_scopes.clone(),
            enforced: true,
        }
    }

    fn entitles(&self, scope: &Scope) -> bool {
        self.declared.iter().any(|held| held.entitles(scope))
    }

    /// Whether a frame's `iac.send` scope may be minted at all.
    fn send_granted(&self, scope: &Scope) -> bool {
        if self.enforced {
            return self.entitles(scope);
        }
        let mut sends = self
            .declared
            .iter()
            .filter(|s| matches!(s, Scope::IacSend { .. }));
        match sends.clone().next() {
            None => true,
            Some(_) => sends.any(|declared| declared == scope),
        }
    }

    /// A recall or fetch the successor does not declare: the typed denial reply.
    fn recall_denial(&self, body: &[u8]) -> Option<Vec<u8>> {
        use maos_wasm_host::recall_wire::{self as wire, RecallReply, RecallRequest, WireError};
        if !self.enforced {
            return None;
        }
        let needed = match wire::decode_request(body).ok()? {
            RecallRequest::Recall { .. } => Scope::LogRecall,
            RecallRequest::Fetch { .. } => Scope::LogFetch,
            _ => return None,
        };
        if self.entitles(&needed) {
            return None;
        }
        Some(
            wire::encode_reply(&RecallReply::Error(WireError::CapabilityDenied))
                .unwrap_or_default(),
        )
    }
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
    iac: Arc<maos_iac::adapter::IacBusAdapter>,
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    id: String,
    pid: u32,
}

impl Drop for IdentityOwner {
    fn drop(&mut self) {
        // The declaration goes with the identity, owner-checked like the lookup below: a
        // replacement incarnation's declaration survives this teardown.
        self.iac.consent_policy().unregister(&self.id, self.pid);
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
pub(crate) struct DiagnosticBudget {
    rows: usize,
    bytes: usize,
}

/// Spawn the contained runner for `plan` under `sandbox`. A component artifact travels only as
/// the held file the dedicated WASM/T2 spawn gives the child; there is no path argv and no
/// synthesized read grant. The session and the swap's pre-warmed successor start a child the
/// same way, and the parent's copy of the file closes when this returns, spawned or refused.
pub(crate) fn spawn_runner(
    sandbox: maos_kernel_core::security::sandbox::SandboxSpec,
    plan: SpiritLaunchPlan,
    artifact: Option<LaunchArtifact>,
    spirit_id: &str,
) -> Result<(SpawnedBridge, ChildStopHandle), SessionError> {
    use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
        spawn_wasm_t2_and_bridge, BridgeError,
    };
    let prefix_hash = argv_prefix_hash(&plan.argv);
    let spec = BridgeSpawnSpec {
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
    };
    let mut bridge = match artifact {
        Some(artifact) => spawn_wasm_t2_and_bridge(spec, artifact.held),
        None => spawn_and_bridge(spec),
    }
    .map_err(|error| match error {
        BridgeError::WasmTransportUnsupported(why) => SessionError::WasmTransportUnsupported(why),
        BridgeError::HeldComponent(why) => SessionError::HeldComponent(why),
        other => SessionError::Bridge(other.to_string()),
    })?;
    // The bridge's one stop handle moves out now, while nothing else is committed to this
    // child, so a missing handle can never surface after a predecessor was retired.
    let stop = bridge.take_child_stop_handle()?;
    Ok((bridge, stop))
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

/// How a decoded reply fared in the single arbitration with expiry.
enum Claim {
    Clean,
    Cancelled,
    Expired,
}

/// One frame for the writer thread. `wall`, when set, is the absolute instant past which the
/// frame must not start being written: the export it belongs to has already expired.
struct Outbound {
    body: Vec<u8>,
    wall: Option<Instant>,
}

impl Outbound {
    fn open(body: Vec<u8>) -> Self {
        Self { body, wall: None }
    }

    fn within(body: Vec<u8>, wall: Option<Instant>) -> Self {
        Self { body, wall }
    }
}

/// The capacity-one writer for one child. A deadline-tagged write uses the same
/// absolute wall through pipe backpressure; a partial expired frame is terminal.
fn spawn_frame_writer(
    pid: u32,
    mut writer: FrameWriter,
) -> std::io::Result<(
    std::sync::mpsc::SyncSender<Outbound>,
    std::thread::JoinHandle<std::io::Result<()>>,
)> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<Outbound>(1);
    let thread = std::thread::Builder::new()
        .name(format!("spirit-frame-writer-{pid}"))
        .spawn(move || {
            for out in rx {
                match out.wall {
                    Some(wall) => writer.write_frame_until(&out.body, wall)?,
                    None => writer.write_frame(&out.body)?,
                }
            }
            Ok(())
        })?;
    Ok((tx, thread))
}

/// One armed expiry: the exact incarnation's shared handle (never a numeric PID), its binding,
/// and the absolute monotonic wall.
struct Armed {
    id: u64,
    wall: Instant,
    stop: Arc<ChildStopHandle>,
    core: Arc<BindingCore>,
}

#[derive(Default)]
struct WatchState {
    next: u64,
    armed: Option<Armed>,
    /// The arm whose stop the watch thread is issuing right now, with this lock released.
    stopping: Option<u64>,
    shutdown: bool,
}

struct WatchShared {
    state: Mutex<WatchState>,
    changed: Condvar,
}

impl WatchShared {
    fn lock(&self) -> std::sync::MutexGuard<'_, WatchState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

/// One sleepable monotonic watch lane per live session. An export that can import is armed
/// (exact child, absolute wall) before the guest is entered; at the wall the lane latches the
/// binding's planned stop and signals the exact child, with no mediation, journal, pipe or
/// writer in its path and no lock held across the stop. A clean reply and expiry compete for
/// the same arm: whoever takes it from the slot wins. Dropping the lane disarms it and joins
/// its thread, which only ever waits on its own condition variable or issues one signal.
struct DeadlineWatch {
    shared: Arc<WatchShared>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl DeadlineWatch {
    fn start(pid: u32) -> std::io::Result<Self> {
        let shared = Arc::new(WatchShared {
            state: Mutex::new(WatchState::default()),
            changed: Condvar::new(),
        });
        let lane = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name(format!("spirit-deadline-watch-{pid}"))
            .spawn(move || watch_loop(&lane))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    /// Arm the lane for one export on exactly this child.
    fn arm(&self, wall: Instant, stop: Arc<ChildStopHandle>, core: Arc<BindingCore>) -> u64 {
        let mut state = self.shared.lock();
        state.next += 1;
        let id = state.next;
        state.armed = Some(Armed {
            id,
            wall,
            stop,
            core,
        });
        self.shared.changed.notify_all();
        id
    }

    /// The one short arbitration between a decoded clean reply and expiry, under the lane's
    /// lock and with only atomic compare-and-swap inside it. The arm stays armed through the
    /// reply's decode; it is removed here only when the reply is timely (the wall has not
    /// passed) AND the call's own verdict allows it: `Clean` (the reply claim won) or
    /// `Cancelled` (the waiter gave up first and nothing was lost: disarm, no response, no
    /// kill). If the wall has passed, or the call became EXPIRED, the arm is never removed
    /// here: this waits only until the lane's independent stop attempt has been issued, and
    /// returns `Expired` before any caller journals anything.
    fn claim(&self, id: u64, call: &crate::spawned_lifecycle::CallState) -> Claim {
        let mut state = self.shared.lock();
        if let Some(wall) = state
            .armed
            .as_ref()
            .filter(|armed| armed.id == id)
            .map(|armed| armed.wall)
        {
            if Instant::now() < wall {
                if call.answer() {
                    state.armed = None;
                    return Claim::Clean;
                }
                if !call.link_expired() {
                    state.armed = None;
                    return Claim::Cancelled;
                }
            }
            // Let the lane re-evaluate the wall at once.
            self.shared.changed.notify_all();
        }
        self.settle(state, id);
        Claim::Expired
    }

    /// Wait until this arm's stop has been issued by the lane (or the arm is gone), so that
    /// nothing slower than the stop precedes it. No lock is held while waiting.
    fn await_stop(&self, id: u64) {
        let state = self.shared.lock();
        self.settle(state, id);
    }

    fn settle(&self, mut state: std::sync::MutexGuard<'_, WatchState>, id: u64) {
        while !state.shutdown
            && (state.armed.as_ref().is_some_and(|armed| armed.id == id)
                || state.stopping == Some(id))
        {
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    /// Retire any arm and wait out a stop attempt in flight: nothing of this lane acts on a
    /// child once this returns.
    fn quiesce(&self) {
        let mut state = self.shared.lock();
        state.armed = None;
        while state.stopping.is_some() {
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}

impl Drop for DeadlineWatch {
    fn drop(&mut self) {
        {
            let mut state = self.shared.lock();
            state.shutdown = true;
            state.armed = None;
            self.shared.changed.notify_all();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn watch_loop(shared: &WatchShared) {
    let mut state = shared.lock();
    loop {
        if state.shutdown {
            return;
        }
        let Some(wall) = state.armed.as_ref().map(|armed| armed.wall) else {
            state = shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
            continue;
        };
        let now = Instant::now();
        if now < wall {
            state = shared
                .changed
                .wait_timeout(state, wall - now)
                .unwrap_or_else(|error| error.into_inner())
                .0;
            continue;
        }
        let Some(armed) = state.armed.take() else {
            continue;
        };
        state.stopping = Some(armed.id);
        drop(state);
        // The binding's planned-stop transition, then the exact child's own signal. A failed
        // signal is not evidence of termination: the session's cleanup still kills and reaps.
        armed.core.begin_planned_stop();
        let _ = armed.stop.stop();
        drop(armed);
        state = shared.lock();
        state.stopping = None;
        shared.changed.notify_all();
    }
}

/// One lifecycle export in flight on the child: its hook awaits `answer` (a probe has none).
struct Pending {
    answer: Option<std::sync::mpsc::SyncSender<Answer>>,
    /// The hook's call state: its reply is delivered, and a snapshot's fence set, only if the
    /// hook is still waiting when the reply arrives (a probe has no hook).
    state: Option<Arc<crate::spawned_lifecycle::CallState>>,
    request: LifecycleRequest,
    /// The deadline-watch arm guarding this export while the guest can import; claimed with its
    /// reply. A probe has none.
    arm: Option<u64>,
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
    run_session(deps, admitted, core, launch, None).await
}

/// [`run_admitted_session`] for an installed Spirit whose host owner serves shell turns
/// through `shell`.
pub async fn run_admitted_shell_session(
    deps: SessionDeps,
    admitted: Admitted,
    core: Arc<BindingCore>,
    launch: SessionLaunch,
    shell: Arc<ShellPort>,
) -> Result<SessionReport, SessionError> {
    run_session(deps, admitted, core, launch, Some(shell)).await
}

async fn run_session(
    deps: SessionDeps,
    admitted: Admitted,
    core: Arc<BindingCore>,
    launch: SessionLaunch,
    shell: Option<Arc<ShellPort>>,
) -> Result<SessionReport, SessionError> {
    let owner = IdentityOwner {
        iac: Arc::clone(&deps.iac),
        lookup: Arc::clone(&deps.pid_by_spirit_id),
        id: admitted.spirit_id.clone(),
        pid: admitted.pid,
    };
    let ended = deps.supervisor.session_ends().begin(&admitted.spirit_id);
    let binding = deps
        .supervisor
        .adopt_spawned(core)
        .map_err(|error| SessionError::Supervision(error.to_string()))?;
    tokio::task::spawn_blocking(move || {
        // Declared first, so dropped last: the session has ended only after the mailbox
        // registration and the identity below are released.
        let _ended = ended;
        let _owner = owner;
        let _shell_end = ShellEnd(shell.clone());
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
        let mut authority = Authority::admitted(&admitted.sandbox);
        let swap_sandbox = admitted.sandbox.clone();
        let mut recipient = Recipient {
            mailbox: Arc::clone(deps.iac.mailbox()),
            handle: deps.iac.register_spirit_typed(&SpiritId::from(admitted.spirit_id.as_str()))
                .map_err(|error| SessionError::Mailbox(error.to_string()))?,
            id: admitted.spirit_id.clone(),
        };
        let artifact_sha256 = launch.component_artifact.as_ref().map(LaunchArtifact::sha256);
        let verified_publisher = launch
            .component_artifact
            .as_ref()
            .and_then(LaunchArtifact::publisher);
        let (bridge, stop) = spawn_runner(admitted.sandbox, launch.plan, launch.component_artifact,
            &admitted.spirit_id)?;
        let mut live = LiveWorker { binding, bridge };
        // The one handle moves to this host owner, shared only with its one watcher.
        let mut stop = Arc::new(stop);
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
        link.set_swap_context(SwapContext {
            sandbox: swap_sandbox,
            origin: launch.origin.clone(),
            artifact_sha256,
            verified_publisher,
        });
        let mut pending: Option<Pending> = None;
        let mut turn_budget = launch.turn_budget;
        let mut swapped_at: Option<Instant> = None;
        let result = (|| {
            let writer = live.bridge.take_frame_writer()
                .map_err(|error| SessionError::Bridge(error.to_string()))?;
            let (first_tx, first_thread) = spawn_frame_writer(admitted.pid, writer)
                .map_err(|error| SessionError::Bridge(error.to_string()))?;
            writer_thread = Some(first_thread);
            let mut tx = Some(first_tx);
            // The session's one monotonic child-stop lane; dropped (disarmed and joined) when
            // this loop returns, before any cleanup below.
            let watch = DeadlineWatch::start(admitted.pid)
                .map_err(|error| SessionError::Bridge(error.to_string()))?;
            let mut draining = false;
            let mut initial_frame = launch.initial_frame;
            let mut seed_id = None;
            let mut turn = TurnState::default();
            let mut generation = 1u64;
            let mut bound: Option<ShellBinding> = None;
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
            // Observed expiry of the current stage deadline or of the pending call's firing
            // wall / link deadline. A started call's expiry is one atomic EXPIRED transition
            // (`CallState::observe_expiry`), so a concurrent reply claim cannot also win.
            let expiry = |deadline: Option<(Instant, &'static str)>,
                          pending: &Option<Pending>| -> Option<SessionError> {
                if let Some((until, stage)) = deadline {
                    if Instant::now() >= until {
                        if pending.is_some() {
                            // The independent stop is issued before anything slower.
                            if let Some(arm) = pending.as_ref().and_then(|p| p.arm) {
                                watch.await_stop(arm);
                            }
                            journal_session_event(&deps, admitted.pid, &admitted.spirit_id,
                                "spirit.lifecycle.deadline", serde_json::json!({ "stage": stage }));
                        }
                        return Some(SessionError::Deadline(stage));
                    }
                }
                if pending.as_ref().and_then(|p| p.state.as_ref()).is_some_and(|s| s.observe_expiry()) {
                    if let Some(arm) = pending.as_ref().and_then(|p| p.arm) {
                        watch.await_stop(arm);
                    }
                    journal_session_event(&deps, admitted.pid, &admitted.spirit_id,
                        "spirit.lifecycle.deadline", serde_json::json!({ "stage": "lifecycle link" }));
                    return Some(SessionError::LinkDeadline("lifecycle export"));
                }
                None
            };
            // The soonest wall a recall reply's write may still begin under.
            let wall_of = |deadline: Option<(Instant, &'static str)>,
                           pending: &Option<Pending>| -> Option<Instant> {
                let call = pending.as_ref().and_then(|p| p.state.as_ref()).map(|state| state.expiry());
                match (deadline.map(|(until, _)| until), call) {
                    (Some(stage), Some(call)) => Some(stage.min(call)),
                    (stage, call) => stage.or(call),
                }
            };
            loop {
                if deps.supervisor.is_stopping() {
                    return Err(SessionError::Stopped);
                }
                if let Some(error) = expiry(deadline, &pending) {
                    return Err(error);
                }
                if !active && pending.is_none() {
                    if let Some(next) = link.take_handover() {
                        // The coordinator accepted the successor's swap-in and published it
                        // (Story 20-9 AC4); only that outer commit hands this child over. A
                        // candidate the SCB no longer runs (reverted or unloaded since) is
                        // stale: it dies unadopted and this predecessor, restored, keeps its pid.
                        let crate::spawned_lifecycle::Handover { core: next_core, prewarm } = next;
                        if !deps.supervisor.holds_spirit_object(&next_core) {
                            journal_session_event(&deps, admitted.pid, &admitted.spirit_id,
                                "spirit.swap.handover_stale", serde_json::json!({}));
                            link.end_current_attempt();
                            continue;
                        }
                        let next_stop = Arc::new(prewarm.stop);
                        // No arm outlives its export: nothing of the watch may still act on the
                        // child about to be released.
                        watch.quiesce();
                        tx.take();
                        if let Some(thread) = writer_thread.take() {
                            let _ = thread.join();
                        }
                        // From here on only the successor's declared scopes grant anything.
                        authority = Authority::successor(&prewarm.sandbox);
                        let next_writer = prewarm.writer;
                        let binding = deps.supervisor.adopt_spawned(Arc::clone(&next_core))
                            .map_err(|error| SessionError::Supervision(error.to_string()))?;
                        retire(&deps, admitted.pid, std::mem::replace(&mut live,
                            LiveWorker { binding, bridge: prewarm.bridge }));
                        stop = next_stop;
                        deps.supervisor.watch(live.binding.core(), live.bridge.child_pid());
                        live.binding.core().mark_ready();
                        let (next_tx, next_thread) = spawn_frame_writer(admitted.pid, next_writer)
                            .map_err(|error| SessionError::Bridge(error.to_string()))?;
                        writer_thread = Some(next_thread);
                        tx = Some(next_tx);
                        link.close();
                        link = Arc::clone(next_core.lifecycle());
                        link.attach_session(prewarm.turn_budget);
                        link.set_supported(true);
                        turn_budget = prewarm.turn_budget;
                        bind_recall(&mut recall, prewarm.context_window, &prewarm.scopes)?;
                        report.child_pid = live.bridge.child_pid();
                        swapped_at = Some(Instant::now());
                        generation += 1;
                        if let Some(port) = &shell {
                            port.ready(admitted.pid, generation);
                        }
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
                        if deps.capability.is_quarantined(admitted.pid) {
                            // FR13: a quarantined incarnation starts no new frame turn; the
                            // frame is refused typed (lifecycle requests below stay available).
                            journal_undeliverable(&deps, admitted.pid, &admitted.spirit_id,
                                frame.frame_id, "spirit is quarantined: ordinary frame refused");
                        } else if seed_id.is_some_and(|id| id != frame.frame_id) {
                            // A `--once` session's only turn is the operator seed.
                            journal_undeliverable(&deps, admitted.pid, &admitted.spirit_id,
                                frame.frame_id, "once session accepts only its operator seed");
                        } else if let Some(reason) = claim_shell_turn(shell.as_deref(), &frame,
                            admitted.pid, generation, &mut bound)
                        {
                            journal_undeliverable(&deps, admitted.pid, &admitted.spirit_id,
                                frame.frame_id, reason);
                        } else {
                            match maos_wasm_host::codec::encode_cbor(&frame) {
                                Err(error) => {
                                    // The slot claimed for this frame must not stay bound.
                                    bound = None;
                                    journal_undeliverable(&deps, admitted.pid,
                                        &admitted.spirit_id, frame.frame_id, &error);
                                }
                                Ok(body) => {
                                    deps.supervisor.begin_turn(live.binding.core(),
                                        format!("turn:{}", ulid::Ulid::from_bytes(frame.frame_id)))
                                        .map_err(|error| SessionError::Supervision(error.to_string()))?;
                                    active = true;
                                    turn.lineage = frame.intent_lineage;
                                    turn.inbound_clock = frame.logical_clock;
                                    turn.frames = 0;
                                    turn.recalls = 0;
                                    // One absolute shell turn deadline clips the active export.
                                    let mut until = Instant::now() + turn_budget;
                                    if let Some(shell_turn) = &bound {
                                        until = until.min(shell_turn.deadline);
                                    }
                                    deadline = Some((until, "active turn"));
                                    tx.as_ref()
                                        .ok_or(SessionError::Protocol("session stdin already closed"))?
                                        .send(Outbound::open(body))
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
                            // A kernel firing ends at the one absolute wall it captured before
                            // queuing; only an export without one is bounded by the turn budget.
                            let export_wall = call.state.firing()
                                .unwrap_or_else(|| Instant::now() + turn_budget);
                            let wall = export_wall.min(call.state.expiry());
                            if Instant::now() >= wall {
                                let _ = call.answer.send(Answer::Reply(LifecycleReply::Unsupported));
                                continue;
                            }
                            // Arm this incarnation before its guest can enter the hook.
                            let arm = watch.arm(wall, Arc::clone(&stop), Arc::clone(live.binding.core()));
                            deadline = Some((export_wall, "lifecycle export"));
                            pending = Some(Pending {
                                answer: Some(call.answer),
                                state: Some(call.state),
                                request: call.request,
                                arm: Some(arm),
                            });
                            if let Err(error) = tx.as_ref()
                                .ok_or(SessionError::Protocol("session stdin already closed"))?
                                .send(Outbound::within(body, Some(wall)))
                            {
                                return Err(expiry(deadline, &pending)
                                    .unwrap_or_else(|| SessionError::Bridge(error.to_string())));
                            }
                        }
                    }
                }
                // The poll never waits past the soonest wall (stage deadline or the pending
                // call's firing wall / link deadline), so nothing can be returned late.
                let poll_wait = {
                    let now = Instant::now();
                    let mut wait = Duration::from_millis(20);
                    if let Some((until, _)) = deadline {
                        wait = wait.min(until.saturating_duration_since(now));
                    }
                    if let Some(state) = pending.as_ref().and_then(|p| p.state.as_ref()) {
                        wait = wait.min(state.expiry().saturating_duration_since(now));
                    }
                    wait
                };
                let polled = live.bridge.poll_frame(poll_wait);
                // A deadline can truncate a reply in the middle of framing.
                // Classify expiry before forwarding either output or the framing error.
                if let Some(error) = expiry(deadline, &pending) {
                    return Err(error);
                }
                match polled.map_err(|error| SessionError::Bridge(error.to_string()))? {
                    BridgePoll::Idle => {}
                    BridgePoll::Closed => {
                        if !ready || active || pending.is_some() {
                            return Err(SessionError::UnexpectedEof);
                        }
                        // A predecessor that exits right after its `snapshot` must not take its
                        // pid down while the swap still replaces its runtime (20-9 F2): the
                        // session outlives its child until the swap resolves. A committed
                        // successor is adopted at the top of the loop; a refused swap releases
                        // the fence, and the next pass ends the session as before.
                        if link.swap_in_flight() {
                            std::thread::sleep(Duration::from_millis(20));
                            continue;
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
                                .send(Outbound::open(probe))
                                .map_err(|error| SessionError::Bridge(error.to_string()))?;
                            pending = Some(Pending { answer: None, state: None, request: LifecycleRequest::Probe, arm: None });
                            deadline = Some((Instant::now() + turn_budget, "lifecycle probe"));
                        } else if active {
                            deps.supervisor.complete_turn(live.binding.core());
                            revoke(&deps.capability, &mut leases)?;
                            // The completion work above is blocking: a clean turn observed
                            // past its wall publishes no answer.
                            if let Some(error) = expiry(deadline, &pending) {
                                return Err(error);
                            }
                            active = false;
                            report.turns += 1;
                            if let Some(shell_turn) = bound.take() {
                                let _ = shell_turn.done.send(());
                            }
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
                            if pending.is_none() {
                                return Err(SessionError::Protocol("lifecycle reply without a request"));
                            }
                            // Decode with the arm still held: expiry stays independent of it.
                            let reply = lifecycle_wire::decode_reply(&bytes).map_err(SessionError::Codec)?;
                            let verdict = match pending.as_ref()
                                .and_then(|p| p.arm.zip(p.state.as_ref()))
                            {
                                Some((arm, call)) => watch.claim(arm, call),
                                None => Claim::Clean,
                            };
                            // A reply that lost to expiry is discarded unseen; the lane has
                            // already issued the stop, and the typed deadline is journaled after.
                            let wanted = match verdict {
                                Claim::Expired => {
                                    return Err(expiry(deadline, &pending)
                                        .unwrap_or(SessionError::Deadline("lifecycle export")));
                                }
                                Claim::Clean => true,
                                // The waiter gave up in time: no response, and no kill.
                                Claim::Cancelled => false,
                            };
                            let Some(done) = pending.take() else {
                                return Err(SessionError::Protocol("lifecycle reply without a request"));
                            };
                            deadline = None;
                            match (&done.request, &reply) {
                                (LifecycleRequest::Probe, LifecycleReply::Supported(supported)) => {
                                    link.set_supported(*supported);
                                    if let Some(port) = &shell {
                                        port.ready(admitted.pid, generation);
                                    }
                                }
                                (LifecycleRequest::Probe, _) =>
                                    return Err(SessionError::Protocol("unexpected probe reply")),
                                // The predecessor's state is now taken: no more work until the swap
                                // resolves. A snapshot its caller gave up on fences nothing, and a
                                // snapshot of a refused swap never fences a later attempt.
                                (LifecycleRequest::Snapshot, LifecycleReply::State(_)) if wanted =>
                                    link.fence_for_snapshot(done.state.as_ref().and_then(|state| state.owner())),
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
                            } else if let Some(denied) = authority.recall_denial(&bytes) {
                                turn.recalls += 1;
                                Ok(denied)
                            } else {
                                turn.recalls += 1;
                                recall.serve_wire(admitted.pid, &bytes)
                            };
                            // Mediation is synchronous and cannot be preempted: a result that
                            // returns past the wall is discarded, not even refused.
                            if let Some(error) = expiry(deadline, &pending) {
                                return Err(error);
                            }
                            let wall = wall_of(deadline, &pending);
                            match reply {
                                Err(error) => {
                                    recall.channel_closed(admitted.pid, error.0);
                                    return Err(SessionError::Protocol(error.0));
                                }
                                Ok(body) => tx.as_ref()
                                    .ok_or(SessionError::Protocol("session stdin already closed"))?
                                    .send(Outbound::within(body, wall))
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
                            &admitted.spirit_id, &authority, frame, until, &mut leases,
                            &mut turn, bound.as_mut())?
                        {
                            report.delivered_frames += 1;
                        } else {
                            report.denied_frames += 1;
                        }
                    }
                }
            }
        })();
        // An export that outlived its hard wall is not left to leave by itself: closing stdin
        // cannot interrupt a running guest call, so the child is stopped now (planned stop,
        // then kill) without the ordinary graceful wait. Leases, finalization and reap follow
        // below in the same order as for any other end.
        let hard_expiry = matches!(
            result,
            Err(SessionError::Deadline(_) | SessionError::LinkDeadline(_))
        );
        if hard_expiry {
            live.binding.core().stop_session();
            let _ = live.bridge.kill();
        }
        if let Some(Pending { answer: Some(answer), .. }) = pending.take() {
            let _ = answer.send(Answer::SessionEnded);
        }
        link.close();
        // A committed successor nobody adopted dies with its handover, not with the dropped core.
        drop(link.take_handover());
        // Whatever ended the loop, the child gets a bounded grace to leave by
        // itself (its stdin closed with the loop), then a planned stop, then a
        // kill: `wait_and_finalize` below can never block forever.
        let grace_end = Instant::now() + TERMINATION_GRACE;
        let exited = hard_expiry || loop {
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
        deps.supervisor.before_session_release(admitted.pid);
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

/// Map a bus refusal onto the session's typed error.
fn bus_error(error: maos_domain::iac_bus_types::IacBusError) -> SessionError {
    match error {
        maos_domain::iac_bus_types::IacBusError::IntentPromotionDenied { .. } => {
            SessionError::IntentPromotionDenied
        }
        other => SessionError::Mailbox(other.to_string()),
    }
}

fn shell_reply_error(error: maos_iac::adapter::mailbox::ShellReplyError) -> SessionError {
    match error {
        maos_iac::adapter::mailbox::ShellReplyError::Refused(refusal) => {
            SessionError::Mailbox(format!("shell reply refused: {refusal:?}"))
        }
        maos_iac::adapter::mailbox::ShellReplyError::Bus(error) => bus_error(error),
    }
}

/// Drive one bus delivery to completion within the turn's remaining time,
/// noticing a supervisor stop while it waits.
fn deliver_bounded<T>(
    deps: &SessionDeps,
    runtime: &tokio::runtime::Handle,
    deadline: Instant,
    work: impl Future<Output = Result<T, maos_domain::iac_bus_types::IacBusError>>,
) -> Result<T, SessionError> {
    deliver_bounded_with(deps, runtime, deadline, work, bus_error)
}

fn deliver_bounded_with<T, E>(
    deps: &SessionDeps,
    runtime: &tokio::runtime::Handle,
    deadline: Instant,
    work: impl Future<Output = Result<T, E>>,
    map: impl FnOnce(E) -> SessionError,
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
                        Ok(result) => result.map_err(map),
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
    journal_guest_diagnostic(
        &deps.journal,
        (spirit_id, pid, report.child_pid),
        (&mut report.dropped_diagnostics, budget),
        line,
        None,
    );
}

/// [`journal_diagnostic`]'s one writer, shared with a pre-warmed successor (`phase =
/// "prewarm"`, Story 22-9): the same 256-row / 64 KiB bound, with the overflow counted.
pub(crate) fn journal_guest_diagnostic(
    tl: &TransparencyLogAdapter,
    (spirit_id, pid, child_pid): (&str, u32, u32),
    (dropped, budget): (&mut u64, &mut DiagnosticBudget),
    line: &[u8],
    phase: Option<&str>,
) {
    let mut record =
        if budget.rows >= MAX_DIAGNOSTIC_ROWS || line.len() > MAX_DIAGNOSTIC_BYTES - budget.bytes {
            *dropped += 1;
            if *dropped > 1 {
                return;
            }
            serde_json::json!({
                "event": "spirit_diagnostic_dropped",
                "child_pid": child_pid,
                "max_rows": MAX_DIAGNOSTIC_ROWS,
                "max_bytes": MAX_DIAGNOSTIC_BYTES,
            })
        } else {
            budget.rows += 1;
            budget.bytes += line.len();
            serde_json::json!({
                "event": "spirit_diagnostic",
                "child_pid": child_pid,
                "line": String::from_utf8_lossy(line),
            })
        };
    if let Some(phase) = phase {
        record["phase"] = phase.into();
    }
    let _ = tl.insert_frame_event_with_sender(
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
        peer_class: if frame.to.is_empty() && frame.kind != FrameKind::Retract {
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
    authority: &Authority,
    mut frame: IacFrame,
    deadline: Instant,
    leases: &mut Vec<TokenId>,
    turn: &mut TurnState,
    shell: Option<&mut ShellBinding>,
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
    if expected_kind == FrameKind::EpistemicHalt {
        return Err(SessionError::Protocol(
            "epistemic halt frames are kernel-reserved",
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
    let shell = shell.filter(|_| is_shell_reply(&frame));
    if refusal.is_none()
        && shell
            .as_ref()
            .is_some_and(|binding| binding.route.is_none())
    {
        refusal = Some("shell_reply_already_sent");
    }
    // The reply pseudo-destination carries only the one bound turn's `TaskComplete`: no other
    // frame, and no unbound or later reply, may name it, and it never routes ordinarily.
    if refusal.is_none()
        && shell.is_none()
        && frame
            .to
            .iter()
            .any(|to| to.spirit_id.as_str() == maos_iac::adapter::mailbox::SHELL_REPLY_DESTINATION)
    {
        refusal = Some("shell_reply_destination_reserved");
    }
    let mut allowed = refusal.is_none();
    // `iac.send` mediation (FR4): every delivery carries its own kernel token.
    // A declared peer class must match; undeclared `iac.send` is the kernel's
    // own deny.
    let send_scope = send_scope(&frame);
    let mut send = None;
    let mut send_token = None;
    if allowed {
        let decision = if authority.send_granted(&send_scope) {
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
                    if authority.enforced && !authority.entitles(scope) {
                        decisions.push(ScopeDecision {
                            scope,
                            kernel_verdict: None,
                            refusal: Some("scope_not_granted".into()),
                        });
                        allowed = false;
                        break;
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
    let reply = shell.and_then(|binding| binding.route.take().map(|route| (route, binding.id)));
    let mut all_refused = false;
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
        frame.stamp_as_spirit(spirit_id, &turn.lineage);
        // Story 21-7 — a guest-supplied envelope is discarded; the granter stamps the one the
        // authenticated sender's declaration implies, pinned to this frame for the gate, so it
        // commits in the routed transaction. Telemetry is ungated: never "all refused". A
        // scoped shell reply is addressed to the host-only `@maos-shell` pseudo-address, never
        // a Spirit: it is unstamped (no declaration can cover it) and never "all refused".
        let policy = deps.iac.consent_policy();
        if reply.is_some() {
            frame.consent_envelope = None;
        } else {
            all_refused = policy.stamp(&mut frame, pid) && frame.kind != FrameKind::TelemetryEvent;
        }
        let frame_id = frame.frame_id;
        turn.clock = turn.clock.max(turn.inbound_clock) + 1;
        frame.logical_clock = turn.clock;
        let delivered = match reply {
            Some((route, correlation)) => match send_token {
                Some(token) => deliver_bounded_with(
                    deps,
                    runtime,
                    deadline,
                    deps.iac
                        .deliver_shell_reply(frame, pid, token, &route, correlation),
                    shell_reply_error,
                )
                .map(drop),
                None => Err(SessionError::Protocol("shell reply has no send token")),
            },
            None => deliver_bounded(
                deps,
                runtime,
                deadline,
                deps.iac.deliver_typed(frame, pid, send_token),
            )
            .map(drop),
        };
        policy.release(frame_id);
        delivered
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
    Ok(!all_refused)
}
