//! `SandboxedContainerChild` — the RAII owner of one T3 container, identified
//! by its private CID receipt from `create` until confirmed removal.
//!
//! [`ContainerTerminal`] is the shared handle both exit feeds read: one
//! immutable terminal snapshot ([`ContainerTerminal::facts`], one absolute
//! deadline from first entry) and one idempotent teardown verdict
//! ([`ContainerTerminal::teardown`]) for finalization, stop and `Drop`. The
//! kernel performs mechanical cleanup only; it never invents supervising
//! intent, so a fault cleanup cannot become a planned stop.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use maos_domain::sandbox::{ContainerResidue, ContainerState, ContainerTerminalFacts};
use maos_domain::sandbox::{PayloadExit, TerminalUnavailable};

use super::runtime_detect::ContainerRuntime;
use super::spawn::{invoke_bounded, Invocation};

/// Kill plus termination confirmation, from teardown entry.
pub const KILL_BUDGET: Duration = Duration::from_secs(5);
/// Terminal snapshot, from the first feed's entry (podman 5.7 holds the lock ≈8 s post-exit).
pub const SNAPSHOT_BUDGET: Duration = Duration::from_secs(10);
/// Removal plus the existence check.
pub const REMOVE_BUDGET: Duration = Duration::from_secs(10);
/// The absolute teardown deadline every stage also respects.
pub const TEARDOWN_BUDGET: Duration = Duration::from_secs(25);

type Verdict = Result<(), ContainerResidue>;

#[maos_attrs::i9_exempt(
    reason = "one container's immutable evidence and bounded teardown progress, not learned state"
)]
struct Progress {
    snapshot_deadline: Option<Instant>,
    snapshot_inflight: bool,
    facts: Option<Arc<ContainerTerminalFacts>>,
    /// `None`: not begun; `Some(None)`: in flight; `Some(Some(_))`: final.
    teardown: Option<Option<Verdict>>,
    /// First caller's absolute teardown deadline; later callers only join.
    teardown_deadline: Option<Instant>,
    kill_failed: bool,
    kill_failure_reported: bool,
}

struct StartIdentity {
    historical_pid: u32,
    started: bool,
    #[cfg(target_os = "linux")]
    pidfd: Option<std::os::fd::OwnedFd>,
}

/// The shared terminal handle of one owned container.
#[maos_attrs::i9_exempt(
    reason = "per-container terminal custody shared by the exit observer, bridge \
              finalization and stop paths: one immutable snapshot, one removal verdict"
)]
pub struct ContainerTerminal {
    runtime: ContainerRuntime,
    cid: String,
    state_dir: PathBuf,
    identity: Mutex<StartIdentity>,
    confirmed: AtomicBool,
    progress: Mutex<Progress>,
    changed: Condvar,
}

impl ContainerTerminal {
    pub fn container_id(&self) -> &str {
        &self.cid
    }

    /// The pidfile PID read after `init`: history, never liveness.
    pub fn historical_pid(&self) -> u32 {
        self.identity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .historical_pid
    }

    fn started(&self) -> bool {
        self.identity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .started
    }

    pub fn runtime(&self) -> &ContainerRuntime {
        &self.runtime
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Progress> {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Monotonic: a CID/pidfd observation has proved the payload terminated.
    pub fn termination_confirmed(&self) -> bool {
        if self.pidfd_ready(Duration::ZERO) {
            self.confirmed.store(true, Ordering::Release);
        }
        self.confirmed.load(Ordering::Acquire)
    }

    /// The verified payload pidfd polled readable (the payload exited).
    fn pidfd_ready(&self, timeout: Duration) -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsFd;
            let identity = self
                .identity
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(pidfd) = &identity.pidfd {
                return crate::security::sandbox::linux::readable(pidfd.as_fd(), timeout);
            }
        }
        let _ = timeout;
        false
    }

    /// Wait a short slice for the payload, never past `by`.
    fn pause(&self, by: Instant) {
        let slice = by
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(50));
        if !self.has_pidfd() {
            std::thread::sleep(slice);
        } else if self.pidfd_ready(slice) {
            std::thread::sleep(slice.min(Duration::from_millis(10)));
        }
    }

    fn has_pidfd(&self) -> bool {
        #[cfg(target_os = "linux")]
        return self
            .identity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pidfd
            .is_some();
        #[cfg(not(target_os = "linux"))]
        false
    }

    fn inspect_state(&self, by: Instant) -> Result<ContainerState, TerminalUnavailable> {
        let args = ["inspect", "--format", "{{json .State}}", self.cid.as_str()];
        match invoke_bounded(&self.runtime.path, &args, &[], by) {
            Invocation::Exited {
                status,
                stdout,
                stdout_complete: true,
                ..
            } if status.success() => ContainerState::from_inspect_json(&stdout)
                .ok_or(TerminalUnavailable::MalformedState),
            Invocation::TimedOut => Err(TerminalUnavailable::InspectTimedOut),
            _ => Err(TerminalUnavailable::InspectFailed),
        }
    }

    /// The payload's verified wait status, only with the returned EXIT bit.
    #[cfg(target_os = "linux")]
    fn pidfd_exit(&self, by: Instant) -> Option<PayloadExit> {
        use std::os::fd::AsFd;
        loop {
            let status = {
                let identity = self
                    .identity
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let pidfd = identity.pidfd.as_ref()?;
                crate::security::sandbox::linux::pidfd_exit_status(pidfd.as_fd())
            };
            match status {
                Ok(Some(raw)) => return PayloadExit::from_wait_status(raw),
                Ok(None) => {}
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return None,
            }
            if Instant::now() >= by {
                return None;
            }
            self.pause(by);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn pidfd_exit(&self, _: Instant) -> Option<PayloadExit> {
        None
    }

    fn observe(&self, by: Instant) -> ContainerTerminalFacts {
        let state = loop {
            match self.inspect_state(by) {
                Ok(state) if state.is_terminal() => break Ok(state),
                Ok(_) if Instant::now() < by => self.pause(by),
                Ok(_) => break Err(TerminalUnavailable::NonTerminal),
                Err(why) => break Err(why),
            }
        };
        if state.is_ok() {
            self.confirmed.store(true, Ordering::Release);
        }
        ContainerTerminalFacts::new(state, self.pidfd_exit(by))
    }

    /// The one immutable terminal snapshot. The first caller owns
    /// initialization under one absolute deadline; followers wait on that
    /// deadline, and expiry freezes typed unavailable evidence. A
    /// non-terminal state is never cached as an exit.
    pub fn facts(&self) -> Arc<ContainerTerminalFacts> {
        self.facts_by(Instant::now() + TEARDOWN_BUDGET, false)
            .expect("an uncapped observer joins the shared snapshot deadline")
    }

    /// A cap-limited follower returns `None` rather than freezing shared
    /// evidence when its own teardown budget expires before snapshot ownership.
    pub(crate) fn facts_by(
        &self,
        cap: Instant,
        respect_caller_cap: bool,
    ) -> Option<Arc<ContainerTerminalFacts>> {
        let mut progress = self.lock();
        if let Some(facts) = &progress.facts {
            return Some(Arc::clone(facts));
        }
        let Some(deadline) = progress.snapshot_deadline else {
            let deadline = cap.min(Instant::now() + SNAPSHOT_BUDGET);
            progress.snapshot_deadline = Some(deadline);
            progress.snapshot_inflight = true;
            drop(progress);
            let observed = Arc::new(self.observe(deadline));
            let mut progress = self.lock();
            progress.snapshot_inflight = false;
            let facts = Arc::clone(progress.facts.get_or_insert(observed));
            self.changed.notify_all();
            return Some(facts);
        };
        loop {
            if let Some(facts) = &progress.facts {
                return Some(Arc::clone(facts));
            }
            let now = Instant::now();
            if respect_caller_cap && now >= cap && now < deadline {
                return None;
            }
            if now >= deadline {
                let expired =
                    ContainerTerminalFacts::new(Err(TerminalUnavailable::DeadlineExpired), None);
                let facts = Arc::clone(progress.facts.get_or_insert(Arc::new(expired)));
                self.changed.notify_all();
                return Some(facts);
            }
            let until = if respect_caller_cap {
                cap.min(deadline)
            } else {
                deadline
            };
            progress = self
                .changed
                .wait_timeout(progress, until - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }

    /// One bounded CID kill. A failed outcome remains diagnostic even if
    /// later confirmation/removal succeeds.
    pub fn kill(&self, by: Instant) -> bool {
        let succeeded =
            invoke_bounded(&self.runtime.path, &["kill", &self.cid], &[], by).succeeded();
        if !succeeded {
            self.lock().kill_failed = true;
        }
        succeeded
    }

    pub fn kill_succeeded(&self) -> bool {
        !self.lock().kill_failed
    }

    /// Story 19-2 (FR51 a/b) — freeze (`podman pause`) or thaw (`podman
    /// unpause`) the CID's cgroup, bounded like [`Self::kill`]. A frozen
    /// Worker keeps its state and takes no further step until thawed.
    pub fn set_frozen(&self, frozen: bool, by: Instant) -> bool {
        let verb = if frozen { "pause" } else { "unpause" };
        invoke_bounded(&self.runtime.path, &[verb, &self.cid], &[], by).succeeded()
    }

    /// Preserve an owning caller's absolute deadline for any Drop/join that
    /// races its teardown request.
    pub fn constrain_teardown_deadline(&self, deadline: Instant) {
        let mut progress = self.lock();
        progress.teardown_deadline = Some(
            progress
                .teardown_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
    }

    fn confirm(&self, by: Instant) -> bool {
        loop {
            if self.termination_confirmed() {
                return true;
            }
            if matches!(self.inspect_state(by), Ok(state) if state.is_terminal()) {
                self.confirmed.store(true, Ordering::Release);
                return true;
            }
            if Instant::now() >= by {
                return false;
            }
            self.pause(by);
        }
    }

    /// CID kill → termination confirmation → snapshot if absent → bounded
    /// `rm --force --volumes` → `container exists` (1 = absent). One verdict
    /// per CID: concurrent callers join the work in flight with their own
    /// remaining budget, and a final verdict is never retried.
    pub fn teardown(&self, deadline: Instant) -> Result<(), ContainerResidue> {
        let mut progress = self.lock();
        loop {
            match progress.teardown {
                Some(Some(verdict)) => return verdict,
                None => break,
                Some(None) => {
                    let join_by = progress.teardown_deadline.unwrap_or(deadline).min(deadline);
                    let now = Instant::now();
                    if now >= join_by {
                        return Err(ContainerResidue::TerminationUnconfirmed);
                    }
                    progress = self
                        .changed
                        .wait_timeout(progress, join_by - now)
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .0;
                }
            }
        }
        let overall = progress
            .teardown_deadline
            .unwrap_or(deadline)
            .min(deadline)
            .min(Instant::now() + TEARDOWN_BUDGET);
        progress.teardown_deadline = Some(overall);
        progress.teardown = Some(None);
        drop(progress);
        let verdict = self.run_teardown(overall);
        let report_kill_failure = {
            let mut progress = self.lock();
            progress.teardown = Some(Some(verdict));
            if progress.kill_failed && !progress.kill_failure_reported {
                progress.kill_failure_reported = true;
                verdict != Err(ContainerResidue::KillFailed)
            } else {
                false
            }
        };
        self.changed.notify_all();
        match verdict {
            Ok(()) => {
                let _ = std::fs::remove_dir_all(&self.state_dir);
            }
            Err(residue) => self.report(residue),
        }
        if report_kill_failure {
            self.report_kill_failure();
        }
        verdict
    }

    fn teardown_pending(&self) -> bool {
        matches!(self.lock().teardown, Some(None))
    }

    /// One non-panicking root-stderr residue line (typed category, no runtime text).
    fn report(&self, residue: ContainerResidue) {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "maos: T3 container {} residue {residue:?}; state retained at {}",
            self.cid,
            self.state_dir.display()
        );
    }

    fn report_kill_failure(&self) {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "maos: T3 container {} CID kill failed; later cleanup outcome is reported separately",
            self.cid
        );
    }

    fn run_teardown(&self, deadline: Instant) -> Verdict {
        use ContainerResidue as R;
        let entry = Instant::now();
        let overall = deadline.min(entry + TEARDOWN_BUDGET);
        let started = self.started();
        let mut unconfirmed = None;
        if started && !self.termination_confirmed() {
            let confirm_by = overall.min(entry + KILL_BUDGET);
            let killed = self.kill(confirm_by);
            if !self.confirm(confirm_by) {
                unconfirmed = Some(if killed {
                    R::TerminationUnconfirmed
                } else {
                    R::KillFailed
                });
            }
        }
        if started {
            let facts = self.facts_by(overall, true);
            if facts.is_none() || self.lock().snapshot_inflight {
                return Err(R::TerminationUnconfirmed);
            }
        }
        let remove_by = overall.min(Instant::now() + REMOVE_BUDGET);
        let path = &self.runtime.path;
        let rm = invoke_bounded(
            path,
            &["rm", "--force", "--volumes", "--time=0", &self.cid],
            &[],
            remove_by,
        );
        let (removal, volumes) = match &rm {
            Invocation::Exited {
                status,
                stderr,
                stderr_complete: true,
                ..
            } if status.success() => {
                let volume = String::from_utf8_lossy(stderr)
                    .to_ascii_lowercase()
                    .contains("volume");
                (None, volume.then_some(R::VolumeCleanupUnconfirmed))
            }
            Invocation::Exited { status, .. } if status.success() => {
                (None, Some(R::VolumeCleanupUnconfirmed))
            }
            Invocation::TimedOut => (Some(R::RemoveTimeout), None),
            _ => (Some(R::RemoveFailed), None),
        };
        let absence =
            match invoke_bounded(path, &["container", "exists", &self.cid], &[], remove_by) {
                Invocation::Exited { status, .. } if status.code() == Some(1) => None,
                Invocation::Exited { status, .. } if status.success() => Some(R::StillExists),
                _ => Some(R::ExistsFailed),
            };
        if removal.is_none() && absence.is_none() {
            // A confirmed-absent forced removal resolves the kill stage.
            self.confirmed.store(true, Ordering::Release);
            unconfirmed = None;
        }
        unconfirmed
            .or(removal)
            .or(absence)
            .or(volumes)
            .map_or(Ok(()), Err)
    }
}

#[maos_attrs::i9_exempt(
    reason = "locked launch progress retains the single terminal CID receipt for shared teardown"
)]
struct LaunchProgress {
    active: bool,
    completed: bool,
    cancelled: bool,
    ownership_unknown: bool,
    terminal: Option<Arc<ContainerTerminal>>,
    teardown_deadline: Option<Instant>,
    report_emitted: bool,
}

/// Shared launch lifecycle from before `create` through terminal CID cleanup.
#[maos_attrs::i9_exempt(
    reason = "per-launch CID custody and cancellation shared by startup, root shutdown and binding"
)]
pub struct ContainerLaunchCustody {
    state_dir: PathBuf,
    progress: Mutex<LaunchProgress>,
    changed: Condvar,
    on_terminal: Box<dyn Fn(Arc<ContainerTerminal>) + Send + Sync>,
}

impl ContainerLaunchCustody {
    pub fn new(
        state_dir: PathBuf,
        on_terminal: impl Fn(Arc<ContainerTerminal>) + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            state_dir,
            progress: Mutex::new(LaunchProgress {
                active: false,
                completed: false,
                cancelled: false,
                ownership_unknown: false,
                terminal: None,
                teardown_deadline: None,
                report_emitted: false,
            }),
            changed: Condvar::new(),
            on_terminal: Box::new(on_terminal),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LaunchProgress> {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn begin(self: &Arc<Self>) -> Option<LaunchPermit> {
        let mut progress = self.lock();
        if progress.cancelled || progress.active || progress.completed {
            return None;
        }
        progress.active = true;
        Some(LaunchPermit {
            custody: Arc::clone(self),
        })
    }

    pub(crate) fn publish_terminal(&self, terminal: Arc<ContainerTerminal>) -> bool {
        let (cancelled, deadline) = {
            let mut progress = self.lock();
            progress.terminal = Some(Arc::clone(&terminal));
            (progress.cancelled, progress.teardown_deadline)
        };
        if let Some(deadline) = deadline {
            terminal.constrain_teardown_deadline(deadline);
        }
        (self.on_terminal)(terminal);
        !cancelled
    }

    pub(crate) fn mark_ownership_unknown(&self) {
        self.lock().ownership_unknown = true;
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    fn complete(&self) {
        let mut progress = self.lock();
        progress.active = false;
        progress.completed = true;
        drop(progress);
        self.changed.notify_all();
    }

    /// Cancel startup, join its current bounded runtime call, and teardown the
    /// CID under the caller's one absolute deadline.
    pub fn cancel_and_teardown(&self, deadline: Instant) -> Result<(), ContainerResidue> {
        let mut progress = self.lock();
        progress.cancelled = true;
        progress.teardown_deadline = Some(
            progress
                .teardown_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
        if let Some(terminal) = &progress.terminal {
            terminal.constrain_teardown_deadline(deadline);
        }
        while progress.active {
            let now = Instant::now();
            if now >= deadline {
                let terminal = progress.terminal.clone();
                let should_report = !progress.report_emitted;
                progress.report_emitted |= should_report;
                drop(progress);
                if should_report {
                    self.report_unresolved(terminal.as_deref());
                }
                return Err(ContainerResidue::TerminationUnconfirmed);
            }
            progress = self
                .changed
                .wait_timeout(progress, deadline - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        let terminal = progress.terminal.clone();
        let ownership_unknown = progress.ownership_unknown;
        if terminal.is_none() && !ownership_unknown {
            progress.completed = true;
        }
        let should_report = ownership_unknown && !progress.report_emitted;
        progress.report_emitted |= should_report;
        drop(progress);
        if let Some(terminal) = terminal {
            terminal.constrain_teardown_deadline(deadline);
            let result = terminal.teardown(deadline);
            if result.is_err() && Instant::now() >= deadline && terminal.teardown_pending() {
                let should_report = {
                    let mut progress = self.lock();
                    let should_report = !progress.report_emitted;
                    progress.report_emitted |= should_report;
                    should_report
                };
                if should_report {
                    self.report_unresolved(Some(&terminal));
                }
            }
            result
        } else if ownership_unknown {
            if should_report {
                self.report_unresolved(None);
            }
            Err(ContainerResidue::TerminationUnconfirmed)
        } else {
            Ok(())
        }
    }

    pub fn termination_confirmed(&self) -> bool {
        let progress = self.lock();
        match &progress.terminal {
            Some(terminal) => terminal.termination_confirmed(),
            None => progress.completed && !progress.ownership_unknown,
        }
    }

    pub fn kill_succeeded(&self) -> bool {
        self.lock()
            .terminal
            .as_ref()
            .is_none_or(|terminal| terminal.kill_succeeded())
    }

    /// Remove only the fresh empty directory when this launch never acquired
    /// or might have acquired a CID. Owned or uncertain state stays in custody.
    pub fn remove_state_dir_if_unowned(&self) {
        let progress = self.lock();
        if progress.active || progress.terminal.is_some() || progress.ownership_unknown {
            return;
        }
        drop(progress);
        let _ = std::fs::remove_dir(&self.state_dir);
    }

    fn report_unresolved(&self, terminal: Option<&ContainerTerminal>) {
        use std::io::Write;
        let mut stderr = std::io::stderr();
        if let Some(terminal) = terminal {
            let _ = writeln!(
                stderr,
                "maos: T3 launch CID {} remained in custody at shutdown deadline; state retained at {}",
                terminal.container_id(),
                self.state_dir.display()
            );
        } else {
            let _ = writeln!(
                stderr,
                "maos: T3 launch ownership unresolved at shutdown deadline; state retained at {}",
                self.state_dir.display()
            );
        }
    }
}

#[maos_attrs::i9_exempt(
    reason = "linear startup permit retaining only its launch coordinator until runtime calls finish"
)]
pub(crate) struct LaunchPermit {
    custody: Arc<ContainerLaunchCustody>,
}

impl Drop for LaunchPermit {
    fn drop(&mut self) {
        self.custody.complete();
    }
}

/// RAII guard for a T3 container and its supervised `start --attach` client.
#[maos_attrs::i9_exempt(
    reason = "process-bound custody of one runtime client and its owned container CID"
)]
pub struct SandboxedContainerChild {
    /// The supervised `start --attach` runtime client.
    pub child: Option<Child>,
    terminal: Arc<ContainerTerminal>,
    launch_custody: Option<Arc<ContainerLaunchCustody>>,
}

impl SandboxedContainerChild {
    /// Arm the CID guard the moment the receipt is validated.
    pub(crate) fn own(
        runtime: ContainerRuntime,
        cid: String,
        state_dir: &Path,
        launch_custody: Option<Arc<ContainerLaunchCustody>>,
    ) -> Self {
        Self {
            child: None,
            terminal: Arc::new(ContainerTerminal {
                runtime,
                cid,
                state_dir: state_dir.to_path_buf(),
                identity: Mutex::new(StartIdentity {
                    historical_pid: 0,
                    started: false,
                    #[cfg(target_os = "linux")]
                    pidfd: None,
                }),
                confirmed: AtomicBool::new(false),
                progress: Mutex::new(Progress {
                    snapshot_deadline: None,
                    snapshot_inflight: false,
                    facts: None,
                    teardown: None,
                    teardown_deadline: None,
                    kill_failed: false,
                    kill_failure_reported: false,
                }),
                changed: Condvar::new(),
            }),
            launch_custody,
        }
    }

    pub(crate) fn teardown(&self, deadline: Instant) -> Result<(), ContainerResidue> {
        match &self.launch_custody {
            Some(custody) => custody.cancel_and_teardown(deadline),
            None => self.terminal.teardown(deadline),
        }
    }

    /// Record the startup identity before `start`; custody may already be shared.
    #[cfg(target_os = "linux")]
    pub(crate) fn arm(&mut self, pid: u32, pidfd: Option<std::os::fd::OwnedFd>) {
        let mut identity = self
            .terminal
            .identity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        identity.historical_pid = pid;
        identity.pidfd = pidfd;
        identity.started = true;
    }

    /// Own an already-started CID on `runtime` without spawning (test seam
    /// for stub runtimes; production owns CIDs only through `spawn_t3`).
    #[doc(hidden)]
    pub fn adopt(runtime: ContainerRuntime, cid: String, state_dir: &Path) -> Self {
        let guard = Self::own(runtime, cid, state_dir, None);
        guard
            .terminal
            .identity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .started = true;
        guard
    }

    /// The shared terminal handle.
    pub fn terminal(&self) -> &Arc<ContainerTerminal> {
        &self.terminal
    }

    /// Wait for the client to exit and collect its output (once).
    pub fn wait_with_output(&mut self) -> Result<std::process::Output, std::io::Error> {
        self.child
            .take()
            .ok_or_else(|| std::io::Error::other("child already consumed"))?
            .wait_with_output()
    }

    /// Try to wait for the client without blocking.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        match &mut self.child {
            Some(child) => child.try_wait(),
            None => Ok(None),
        }
    }

    /// The startup report: the owned CID and the historical startup PID.
    pub fn inspect_report(
        &self,
        spirit_id: String,
        image: &super::image_lock::VerifiedImageAttestation,
    ) -> maos_domain::sandbox::SandboxInspectReport {
        maos_domain::sandbox::SandboxInspectReport {
            spirit_id,
            payload_pid: maos_domain::sandbox::ObservedPid::Historical(
                self.terminal.historical_pid(),
            ),
            container_id: self.terminal.cid.clone(),
            runtime: self.terminal.runtime.kind.as_str().to_owned(),
            image_sha: hex::encode(image.entry().image_sha256),
            applied_t2_protections: maos_domain::sandbox::T2ProtectionSummary {
                landlock_rules: 0,
                seccomp_allow_count: 0,
                seccomp_kill_count: 0,
            },
            strictest_of_reasoning: maos_domain::sandbox::StrictestOfReasoning {
                manifest_tier: "T3".into(),
                trust_tier_floor: "T3".into(),
                operator_policy_floor: "T0".into(),
                effective_tier: "T3".into(),
                dominant_axis: "manifest".into(),
            },
        }
    }
}

impl Drop for SandboxedContainerChild {
    /// Last resort, not the reporting path: kill the client, then join (or run)
    /// the one CID teardown under the remaining budget.
    fn drop(&mut self) {
        let deadline = Instant::now() + TEARDOWN_BUDGET;
        if let Some(mut client) = self.child.take() {
            let _ = client.kill();
            let reap_by = Instant::now() + Duration::from_millis(500);
            while !matches!(client.try_wait(), Ok(Some(_))) {
                if Instant::now() >= reap_by {
                    // Never promise SIGKILL reaped it: report custody, reap off-thread.
                    self.terminal
                        .report(ContainerResidue::RuntimeClientUnconfirmed);
                    let _ = std::thread::Builder::new().spawn(move || client.wait());
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let _ = self.teardown(deadline);
    }
}
