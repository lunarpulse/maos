#![forbid(unsafe_code)]

//! Authenticated loopback operator HTTP surface.
//!
//! This deliberately owns only the adapter: the scheduler remains the source
//! of truth for reports, and this crate never caches or reconstructs them.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use maos_domain::sandbox::SandboxInspectReport;
use ring::constant_time;

/// The live read seam implemented by the scheduler.
pub trait SandboxReportSource: Send + Sync + 'static {
    fn sandbox_report(&self, spirit_id: &str) -> Option<SandboxInspectReport>;
}

impl SandboxReportSource for maos_kernel_core::scheduler::SpiritSchedulerAdapter {
    fn sandbox_report(&self, spirit_id: &str) -> Option<SandboxInspectReport> {
        self.sandbox_report_by_spirit_id(spirit_id)
    }
}

/// Story 14-2a / AC1.5 — one open peer-certificate rotation window, as an
/// operator reads it.
///
/// Plain owned scalars ON PURPOSE: this crate depends on `maos-domain` and
/// `maos-kernel-core` and nothing else, so the rotation read surface adds NO
/// crate edge to `maos-a2a-core`, `maos-a2a-tcp` or `maos-cohort` — exactly the
/// shape [`SandboxReportSource`] already uses for the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationWindowRow {
    /// The cohort member whose certificate is rotating.
    pub peer: String,
    /// The generation still serving, retired when the grace elapses.
    pub retiring: String,
    /// The incoming generation the signed manifest declares.
    pub next: String,
    /// What plane A (the live router) ACTUALLY declares right now. During the
    /// window it equals `next`; a rotation wired to a DETACHED router core
    /// reports a `declared` that never moves, which is how a wrong-core wiring
    /// becomes visible instead of silently breaking every frame at promotion.
    pub declared: String,
    /// `"open"` (a live overlap) or `"diverged"` (the committed manifest names a
    /// fingerprint the live planes do not implement, because this peer's
    /// rotation was refused).
    pub state: String,
    /// The manifest version that opened the window.
    pub manifest_version: u64,
    /// Cohort-clock seconds at which it opened.
    pub opened_at_secs: u64,
}

/// Health of the installed rotation read control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationWindowStatus {
    Healthy(Vec<RotationWindowRow>),
    Unhealthy { detail: String },
}

/// Story 14-2a / AC1.5 — the live READ seam over the open-window set.
///
/// ⚠ **READ-ONLY, and that is a design decision, not an omission.** A MUTATING
/// rotation verb here was measured out: it would put a POST body through a
/// hand-rolled GET-only parser, and — decisively — it would invent a SECOND
/// trust path for certificate identity, which is precisely what
/// `main.rs:9844-9853` warns against. The WRITE path is the signed cohort
/// manifest reissue and nothing else. This surface exists because a mechanism
/// an operator cannot observe between open and close is a mechanism they cannot
/// operate: without it, confirming that a signed rotation took means grepping
/// SQLite.
pub trait RotationWindowSource: Send + Sync + 'static {
    /// `None` when NO rotation control is installed in this process — the route
    /// then answers 404. `Some(Healthy(vec![]))` means the control IS installed
    /// and nothing is in flight. `Some(Unhealthy { .. })` answers 503 rather
    /// than laundering a poisoned cache or invalid projection into an empty set.
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus>;
}

/// Story 14-2b / AC3 — one cohort peer's last self-declared manifest version,
/// as an operator reads it.
///
/// Plain owned scalars for the SAME reason as [`RotationWindowRow`]: this crate
/// declares `maos-domain`, `maos-kernel-core`, `ring` and `serde_json` and
/// nothing else, so this surface adds NO crate edge to `maos-cohort`. The
/// crossing is done by [`CohortConvergenceSource`], implemented in `maos-bin`
/// over `CohortManifestState` — never by importing a cohort type here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerVersionRow {
    /// The TLS-verified peer the declaration came from.
    pub peer: String,
    /// The version that peer declared it holds.
    pub declared_version: u64,
    /// The canonical hash it declared at that version, hex-encoded.
    pub declared_hash: String,
    /// Cohort-clock seconds at which the declaration was received.
    pub observed_at_secs: u64,
    /// Whether the record is still a convergence claim at all — `false` once
    /// the peer restarted or the observation aged past its derived bound.
    pub valid: bool,
    /// Why: `"observed"`, `"restarted"` or `"stale"`. Reported BESIDE `valid`
    /// because an operator acts differently on each: a restarted peer needs a
    /// re-pin, a stale one needs its pull path investigated.
    pub state: String,
}

/// Health of the installed convergence read control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerVersionStatus {
    Healthy(Vec<PeerVersionRow>),
    Unhealthy { detail: String },
}

/// Story 14-2b / AC3 — the live READ seam over retained peer manifest versions.
///
/// ⚠ **READ-ONLY for the same reason [`RotationWindowSource`] is.** Every value
/// behind this seam is a peer's own authenticated self-report received on its
/// own `Pull`; there is no verb here that could change one, and inventing one
/// would be inventing a second write path for cohort manifest state.
///
/// This seam exists because AC1's table would otherwise repeat this repo's
/// named failure mode: `CohortManifest::peer_configs_for` was built by Story
/// 12.1 for exactly one purpose and sat with ZERO production callers until
/// Story 14-2a found it dead two epics later. A retention mechanism with no
/// production reader is not observability.
pub trait CohortConvergenceSource: Send + Sync + 'static {
    /// `None` when this process holds NO cohort manifest state — the route then
    /// answers 404.
    ///
    /// ⚠ `Some(Healthy(vec![]))` means the cohort state IS present and NO peer
    /// has ever declared a version to it. **That is not agreement**, and the
    /// route must never let it read as one: it is the same distinction
    /// [`RotationWindowSource`] draws between "no control installed" and
    /// "nothing in flight".
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus>;
}

/// Story 14-2c — this host's signed and serving certificate identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfIdentityRow {
    pub declared: Option<String>,
    pub serving: Option<String>,
    /// `"agree"`, `"diverged"`, or `"unconfirmable"`.
    pub verdict: String,
    pub peers_observed: usize,
    pub peers_total: usize,
    /// Each peer's most recent failed pull, absent after its next success.
    pub last_pull_errors: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfIdentityStatus {
    Healthy(SelfIdentityRow),
    Unhealthy { detail: String },
}

/// Read-only self-identity diagnostics for a process holding cohort state.
pub trait CohortSelfIdentitySource: Send + Sync + 'static {
    /// `None` only when this process holds no cohort state.
    fn self_identity(&self) -> Option<SelfIdentityStatus>;
}

/// Configuration for the authenticated operator endpoint.
#[derive(Debug, Clone)]
pub struct OperatorHttpConfig {
    pub bind: SocketAddr,
    pub bearer_token: String,
}

impl OperatorHttpConfig {
    /// The safe shipped binding. Callers must still provide an operator token.
    pub fn loopback(bearer_token: String) -> Self {
        Self {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            bearer_token,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Story 16-1 — the MUTATING half of the operator surface.
//
// Until this story the door was GET-only, served by ONE thread that read to
// `\r\n\r\n` in a 16 KiB buffer, scanned every line of the request for the
// bearer (so a BODY line `Authorization: Bearer …` counted as the header),
// answered 404 for every unknown request line, and broke its accept loop on
// the first non-`WouldBlock` error — leaving the process alive with no
// listener. None of that carries a POST body safely.
//
// What follows is the bounded form ADR-062 (amended, Story 16-1) requires:
// auth before route and method, a header scan that stops at the blank line, a
// request-read deadline that covers headers AND body, `Content-Length`
// required for POST, 64 KiB refused WITHOUT reading, a bounded worker pool, a
// panic-isolating handler, and an accept loop that continues.
// ─────────────────────────────────────────────────────────────────────────────

/// The whole-request read deadline: headers plus body.
///
/// Replaces a 2 s timeout on each individual `read()`, which a client dripping
/// one byte every 1.9 s could hold open forever — BEFORE authenticating.
const REQUEST_READ_DEADLINE: Duration = Duration::from_secs(5);

/// Cap on the header section. A request whose headers do not end inside this
/// is refused rather than buffered.
const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Cap on a POST body (ADR-062: "bounded body parsing"). Refused from
/// `Content-Length` alone, so an oversized body is never read.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Concurrent connections served at once. One slow POST used to stall every
/// other verb: `fire_on_pause` runs Spirit hook code, an upgrade runs a
/// hot swap, a CRL import runs an admission guard.
const MAX_WORKERS: usize = 16;

/// How long a route waits for its kernel transition before answering 503.
const DEFAULT_ROUTE_BUDGET: Duration = Duration::from_secs(10);

/// The budget for the four routes measured to run long: upgrade, uninstall,
/// CRL import and hot-swap precheck.
const LONG_ROUTE_BUDGET: Duration = Duration::from_secs(30);

/// A submitted command that has not been picked up yet.
pub const COMMAND_QUEUED: u8 = 0;
/// A command the port has committed to running. From here it runs to
/// completion and writes its rows, whatever the server answers.
pub const COMMAND_STARTED: u8 = 1;
/// A command the SERVER withdrew because its route budget expired before the
/// port started it. It must never run.
pub const COMMAND_WITHDRAWN: u8 = 2;

/// Fixed body of every 405. Byte-equal across routes on purpose: a read route
/// must not leak which verbs exist by varying its refusal.
pub const METHOD_NOT_ALLOWED_BODY: &[u8] = br#"{"error":"method_not_allowed"}"#;
/// Fixed body of a pool-exhaustion 503. Nothing was submitted.
pub const BUSY_BODY: &[u8] = br#"{"error":"busy"}"#;
/// Fixed body of a per-Spirit serialization 503. The command was WITHDRAWN
/// before it started, so there is no `operation_id` to look up: retry the
/// command itself.
pub const SPIRIT_BUSY_BODY: &[u8] = br#"{"error":"spirit_busy"}"#;
/// Fixed body of a captured handler panic. The pool worker survives it.
pub const INTERNAL_BODY: &[u8] = br#"{"error":"internal"}"#;
const NOT_FOUND_BODY: &[u8] = br#"{"error":"not_found"}"#;
const UNAUTHORIZED_BODY: &[u8] = br#"{"error":"unauthorized"}"#;

/// Which upgrade policy the operator asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradePolicyRequest {
    /// The default, and the only one the door performs.
    HotSwap,
    /// Refused typed (D-16-1-W): the kernel's cold-swap arm starts the
    /// successor under a NEW pid without `admit_spirit`, so it would leave an
    /// unadmitted Spirit running. Fixing that is a kernel change.
    ColdSwap,
}

/// Every state-changing operation the door carries.
///
/// One enum rather than one trait method per verb: the port implementation in
/// `maos-bin` serializes these per Spirit id (D-16-1-R), and a shape it can
/// match on is what makes "which Spirit does this command touch" a total
/// function instead of eighteen ad-hoc answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorCommand {
    Start {
        spirit_id: String,
    },
    Pause {
        spirit_id: String,
    },
    Resume {
        spirit_id: String,
    },
    Unload {
        spirit_id: String,
    },
    Posture {
        spirit_id: String,
        posture: String,
    },
    Upgrade {
        spirit_id: String,
        target_manifest: String,
        policy: UpgradePolicyRequest,
        attestation: Option<String>,
        vetter_keyring: Option<String>,
        /// The predecessor version a persisted multi-hop plan starts from.
        /// Only meaningful with `create_plan`.
        from_version: Option<String>,
        /// Candidate successor manifests the resolver validates into a chain.
        candidates: Vec<String>,
        /// Resolve, validate, hash and persist a multi-hop migration plan
        /// without starting an upgrade.
        create_plan: bool,
    },
    HotSwapPrecheck {
        spirit_id: String,
        target_manifest: Option<String>,
    },
    Uninstall {
        spirit_id: String,
    },
    ResolveHalt {
        spirit_id: String,
        halt_id: String,
        resolution: String,
        rationale: Option<String>,
    },
    OrchestratorEnqueue {
        spirit_id: String,
        text: String,
    },
    RevokeToken {
        token_id: String,
    },
    /// The CRL's own bytes, never a path: a path would resolve in the daemon's
    /// working directory, which is not the operator's (D-16-1-X).
    ImportRevocations {
        crl: Vec<u8>,
    },
    ForgetMemory {
        principal: String,
        reason: Option<String>,
    },
    ReleaseLegalHold {
        principal: String,
    },
    AdmitGovernanceSchema {
        schema: serde_json::Value,
    },
}

impl OperatorCommand {
    /// The Spirit id this command mutates, when it names one.
    ///
    /// The serialization key of D-16-1-R. `None` for the commands whose effect
    /// is host-wide (a CRL import, a memory erase, a legal-hold release, a
    /// schema admission) — those are not serialized against a Spirit.
    pub fn spirit_key(&self) -> Option<&str> {
        match self {
            Self::Start { spirit_id }
            | Self::Pause { spirit_id }
            | Self::Resume { spirit_id }
            | Self::Unload { spirit_id }
            | Self::Posture { spirit_id, .. }
            | Self::Upgrade { spirit_id, .. }
            | Self::HotSwapPrecheck { spirit_id, .. }
            | Self::Uninstall { spirit_id }
            | Self::ResolveHalt { spirit_id, .. }
            | Self::OrchestratorEnqueue { spirit_id, .. } => Some(spirit_id),
            Self::RevokeToken { .. }
            | Self::ImportRevocations { .. }
            | Self::ForgetMemory { .. }
            | Self::ReleaseLegalHold { .. }
            | Self::AdmitGovernanceSchema { .. } => None,
        }
    }

    /// How long the route waits before withdrawing or reporting
    /// `handler_still_running`.
    pub fn route_budget(&self) -> Duration {
        match self {
            Self::Upgrade { .. }
            | Self::Uninstall { .. }
            | Self::ImportRevocations { .. }
            | Self::HotSwapPrecheck { .. } => LONG_ROUTE_BUDGET,
            _ => DEFAULT_ROUTE_BUDGET,
        }
    }
}

/// What a command did, as the door reports it.
///
/// Every variant names the KERNEL outcome, not an HTTP status, because the
/// handler's job is the transition and the server's job is the mapping. The
/// `code` is the machine-readable name `maosctl` prints and maps to an exit
/// code (D-16-1-P).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorOutcome {
    /// The transition happened. The value is the route's JSON body.
    Completed(serde_json::Value),
    /// 404 — no such Spirit, halt, token or principal.
    NotFound { code: String, detail: String },
    /// 409 — the transition is not legal from the current state, or the request
    /// asks for something the door refuses on principle (cold swap, a
    /// non-increasing version).
    Conflict { code: String, detail: String },
    /// 400 — the request itself is malformed in a way only the handler can see
    /// (an unreadable manifest, an unparseable CRL).
    Invalid { code: String, detail: String },
    /// 500 — the handler ran and failed.
    Failed { code: String, detail: String },
}

/// The handle a [`OperatorCommandPort::submit`] hands back.
pub struct OperatorSubmission {
    /// The id the PORT minted. The server never mints one: an id in a 503 must
    /// be findable in the Transparency Log, and only the side that writes the
    /// row can promise that.
    pub operation_id: String,
    /// The shared `Queued`/`Started`/`Withdrawn` word. The port CASes
    /// `Queued → Started` when it commits to running; the server CASes
    /// `Queued → Withdrawn` when its budget expires first. Exactly one wins,
    /// which is why "the budget expired" and "the command started" are never
    /// both reported (V-25: a starved runtime made a timer-based answer lie).
    pub state: Arc<AtomicU8>,
    /// Delivered exactly once, by the task that ran the command.
    pub completion: mpsc::Receiver<OperatorOutcome>,
}

/// One Spirit's live control block, as an operator reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiritStatusRow {
    pub spirit_id: String,
    pub pid: u32,
    pub boot_nonce: String,
    /// Read from `SpiritControlBlock::current_state`, never from a journal row:
    /// a journal-only handler must not be able to make this say `Paused`.
    pub lifecycle_state: String,
    /// Read from the same `PolicyTable` snapshot `evaluate_with_posture` reads.
    pub posture: String,
}

/// The daemon itself, for "is this the root that holds my home's stores".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatusRow {
    pub pid: u32,
    pub boot_nonce: String,
    pub version: String,
    pub spirit_ids: Vec<String>,
}

/// One Spirit's Orchestrator buffer occupancy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorStatusRow {
    pub spirit_id: String,
    pub pending: usize,
    pub capacity: usize,
}

/// One applied CRL, as `maosctl revocations list` renders it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationRow {
    /// Hex, never the 32-integer array `serde` produces for a `CrlId`.
    pub crl_id: String,
    pub matched_count: usize,
    pub revoked_count: usize,
}

/// Story 16-1 / D-16-1-F — the SYNC seam between this crate's blocking server
/// and the daemon's async kernel.
///
/// Sync on purpose. The alternative considered and rejected was handing
/// `bind` a `tokio::runtime::Handle` and calling `block_on` inside each
/// handler: that puts runtime lifetime into the server's signature, blocks a
/// runtime worker from a non-runtime thread, and gives this crate an opinion
/// about the daemon's executor. Instead `maos-bin` implements this trait over
/// a `Handle` it captured in its own `async main`, spawns each kernel future
/// there, and delivers the outcome down `completion`.
///
/// The read methods follow the `SandboxReportSource`/`RotationWindowSource`
/// shape: `None` means "no such object", never an empty value that could be
/// mistaken for one.
pub trait OperatorCommandPort: Send + Sync + 'static {
    /// Hand a command to the daemon. Returns as soon as the command is
    /// QUEUED — never blocks for the transition.
    ///
    /// `deadline` is the route budget, passed so the port can answer
    /// `spirit_busy` itself just before the server gives up rather than
    /// leaving the server to guess.
    fn submit(&self, command: OperatorCommand, deadline: Duration) -> OperatorSubmission;

    fn spirit_status(&self, spirit_id: &str) -> Option<SpiritStatusRow>;

    fn daemon_status(&self) -> DaemonStatusRow;

    fn orchestrator_status(&self, spirit_id: &str) -> Option<OrchestratorStatusRow>;

    fn applied_revocations(&self) -> Vec<RevocationRow>;
}

/// A running server with explicit shutdown for daemon teardown and tests.
pub struct OperatorHttpServer {
    local_addr: SocketAddr,
    stop: Arc<AtomicBool>,
    acceptor: Option<JoinHandle<()>>,
    workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

/// Everything a route may read, borrowed for the life of one request.
struct Routes<'a, S: SandboxReportSource> {
    source: &'a S,
    rotation: Option<&'a dyn RotationWindowSource>,
    convergence: Option<&'a dyn CohortConvergenceSource>,
    self_identity: Option<&'a dyn CohortSelfIdentitySource>,
    commands: Option<&'a dyn OperatorCommandPort>,
}

impl OperatorHttpServer {
    /// `rotation` is `None` in every process with no cohort daemon config: the
    /// route then answers 404 rather than an empty list, so "no rotation
    /// control in this process" and "no windows open" are never confused.
    ///
    /// `convergence` is `None` on the same condition and for the same reason
    /// (Story 14-2b / AC3): an absent cohort state must not render as a cohort
    /// in which no peer has diverged.
    ///
    /// No command port: every mutating route answers 404 on the same
    /// "the control is not installed here" convention. Story 16-1's door roots
    /// call [`Self::bind_with_commands`].
    pub fn bind<S: SandboxReportSource>(
        config: OperatorHttpConfig,
        source: Arc<S>,
        rotation: Option<Arc<dyn RotationWindowSource>>,
        convergence: Option<Arc<dyn CohortConvergenceSource>>,
        self_identity: Option<Arc<dyn CohortSelfIdentitySource>>,
    ) -> Result<Self, std::io::Error> {
        Self::bind_with_commands(config, source, rotation, convergence, self_identity, None)
    }

    /// Story 16-1 — the full door, with the mutating surface installed.
    pub fn bind_with_commands<S: SandboxReportSource>(
        config: OperatorHttpConfig,
        source: Arc<S>,
        rotation: Option<Arc<dyn RotationWindowSource>>,
        convergence: Option<Arc<dyn CohortConvergenceSource>>,
        self_identity: Option<Arc<dyn CohortSelfIdentitySource>>,
        commands: Option<Arc<dyn OperatorCommandPort>>,
    ) -> Result<Self, std::io::Error> {
        if config.bearer_token.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "MAOS_OPERATOR_BEARER_TOKEN must be configured for operator HTTP",
            ));
        }
        if !config.bind.ip().is_loopback() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "operator HTTP must bind a loopback address",
            ));
        }
        let listener = TcpListener::bind(config.bind)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let workers: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
        let acceptor = {
            let stop = Arc::clone(&stop);
            let workers = Arc::clone(&workers);
            let in_flight = Arc::new(AtomicUsize::new(0));
            let token = Arc::new(config.bearer_token.clone());
            thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            // Keep at most MAX_WORKERS spawned connections.
                            // Overload is handled synchronously by the acceptor:
                            // this bounds threads while still authenticating
                            // before returning a typed 503.
                            let previous = in_flight.fetch_add(1, Ordering::AcqRel);
                            if previous >= MAX_WORKERS {
                                in_flight.fetch_sub(1, Ordering::AcqRel);
                                let routes = Routes {
                                    source: source.as_ref(),
                                    rotation: rotation.as_deref(),
                                    convergence: convergence.as_deref(),
                                    self_identity: self_identity.as_deref(),
                                    commands: commands.as_deref(),
                                };
                                let _ = serve_connection(stream, &routes, token.as_bytes(), true);
                                continue;
                            }
                            let source = Arc::clone(&source);
                            let rotation = rotation.clone();
                            let convergence = convergence.clone();
                            let self_identity = self_identity.clone();
                            let commands = commands.clone();
                            let token = Arc::clone(&token);
                            let in_flight_worker = Arc::clone(&in_flight);
                            let worker = thread::spawn(move || {
                                let routes = Routes {
                                    source: source.as_ref(),
                                    rotation: rotation.as_deref(),
                                    convergence: convergence.as_deref(),
                                    self_identity: self_identity.as_deref(),
                                    commands: commands.as_deref(),
                                };
                                let _ = serve_connection(stream, &routes, token.as_bytes(), false);
                                in_flight_worker.fetch_sub(1, Ordering::AcqRel);
                            });
                            if let Ok(mut workers) = workers.lock() {
                                workers.retain(|handle| !handle.is_finished());
                                workers.push(worker);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        // ⚠ CONTINUE, never break. Breaking here left the
                        // process alive with no listener and no diagnostic —
                        // every subsequent operator verb reported "daemon not
                        // running" against a daemon that was running.
                        Err(error) => {
                            eprintln!("maos: operator HTTP accept error (continuing): {error}");
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            })
        };
        Ok(Self {
            local_addr,
            stop,
            acceptor: Some(acceptor),
            workers,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stop accepting and join the accept thread and every pool worker.
    ///
    /// Called by [`Drop`], and directly by the daemon's SIGTERM path, which
    /// must release the listener BEFORE it drains the audit channel: every
    /// worker holds a port clone, the port holds the scheduler, the scheduler
    /// holds the capability adapter, and that holds an `audit_tx` clone — so a
    /// live server makes the drain time out at 10 s (Trap 16).
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
        let handles = match self.workers.lock() {
            Ok(mut workers) => std::mem::take(&mut *workers),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };
        for handle in handles {
            let _ = handle.join();
        }
    }
}

impl Drop for OperatorHttpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// One HTTP status and body.
struct Response {
    status: u16,
    body: Vec<u8>,
}

impl Response {
    fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    fn value(status: u16, body: serde_json::Value) -> Self {
        match serde_json::to_vec(&body) {
            Ok(body) => Self::new(status, body),
            Err(_) => Self::new(500, INTERNAL_BODY),
        }
    }

    fn error(status: u16, code: &str, detail: &str) -> Self {
        Self::value(
            status,
            serde_json::json!({ "error": code, "detail": detail }),
        )
    }
}

impl OperatorOutcome {
    fn into_response(self) -> Response {
        match self {
            Self::Completed(value) => Response::value(200, value),
            Self::NotFound { code, detail } => Response::error(404, &code, &detail),
            Self::Conflict { code, detail } => Response::error(409, &code, &detail),
            Self::Invalid { code, detail } => Response::error(400, &code, &detail),
            Self::Failed { code, detail } => Response::error(500, &code, &detail),
        }
    }
}

/// One authenticated request, with its body still on the wire.
///
/// The body is read **lazily**, by [`Self::body`], and that ordering is
/// load-bearing: `POST /v1/cohort/self-identity` must answer 405 because the
/// route is read-only, NOT 411 because a refusal-by-method was overtaken by a
/// `Content-Length` requirement the route never had.
struct Incoming<'a> {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    stream: &'a mut TcpStream,
    deadline: Instant,
    /// Bytes already read past the header terminator.
    buffered: Vec<u8>,
    body: Option<Vec<u8>>,
}

impl Incoming<'_> {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The request body, read on first call and cached.
    ///
    /// `Content-Length` is REQUIRED here (411) and the 64 KiB bound is
    /// enforced from that header alone (413) — reading 4 GiB to discover it is
    /// too large is the denial of service the bound exists to prevent.
    fn body(&mut self) -> Result<&[u8], Response> {
        if self.body.is_none() {
            if self
                .headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
            {
                return Err(Response::error(
                    400,
                    "bad_request",
                    "Transfer-Encoding is unsupported",
                ));
            }
            let mut lengths = self
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.as_str());
            let Some(declared) = lengths.next() else {
                return Err(Response::error(
                    411,
                    "length_required",
                    "a body-bearing method requires Content-Length",
                ));
            };
            if lengths.next().is_some() {
                return Err(Response::error(
                    400,
                    "bad_request",
                    "duplicate Content-Length",
                ));
            }
            let length: usize = declared.parse().map_err(|_| {
                Response::error(400, "bad_request", "Content-Length is not a number")
            })?;
            if length > MAX_BODY_BYTES {
                return Err(Response::error(
                    413,
                    "payload_too_large",
                    "request body exceeds 64 KiB",
                ));
            }
            if self.buffered.len() > length {
                return Err(Response::error(
                    400,
                    "bad_request",
                    "request body longer than Content-Length",
                ));
            }
            let mut body = std::mem::take(&mut self.buffered);
            while body.len() < length {
                let mut chunk = vec![0_u8; (length - body.len()).min(8192)];
                let read = read_within(self.stream, self.deadline, &mut chunk)?;
                if read == 0 {
                    return Err(Response::error(
                        400,
                        "bad_request",
                        "request body shorter than Content-Length",
                    ));
                }
                body.extend_from_slice(&chunk[..read]);
            }
            self.body = Some(body);
        }
        Ok(self.body.as_deref().unwrap_or_default())
    }
}

fn serve_connection<S: SandboxReportSource>(
    mut stream: TcpStream,
    routes: &Routes<'_, S>,
    expected_token: &[u8],
    overloaded: bool,
) -> Result<(), std::io::Error> {
    let deadline = Instant::now() + REQUEST_READ_DEADLINE;
    let response = match read_head(&mut stream, deadline, expected_token) {
        Ok((method, path, headers, buffered)) if overloaded => {
            let _ = (method, path, headers, buffered);
            Response::new(503, BUSY_BODY)
        }
        Ok((method, path, headers, buffered)) => {
            let mut request = Incoming {
                method,
                path,
                headers,
                stream: &mut stream,
                deadline,
                buffered,
                body: None,
            };
            // A handler panic must cost one request, not the pool worker and
            // not the daemon.
            std::panic::catch_unwind(AssertUnwindSafe(|| route(routes, &mut request)))
                .unwrap_or_else(|_| Response::new(500, INTERNAL_BODY))
        }
        Err(response) => response,
    };
    respond_and_close(&mut stream, response.status, &response.body)
}

/// Read the header section and authenticate.
///
/// Authentication happens here, BEFORE the method or the path is looked at and
/// before any body byte is read, so an anonymous request learns nothing about
/// which routes exist.
#[allow(clippy::type_complexity)]
fn read_head(
    stream: &mut TcpStream,
    deadline: Instant,
    expected_token: &[u8],
) -> Result<(String, String, Vec<(String, String)>, Vec<u8>), Response> {
    let mut buffer = Vec::with_capacity(2048);
    let head_end = loop {
        if buffer.len() > MAX_HEADER_BYTES {
            return Err(Response::error(
                413,
                "header_too_large",
                "request header section exceeds 16 KiB",
            ));
        }
        let mut chunk = [0_u8; 2048];
        let read = read_within(stream, deadline, &mut chunk)?;
        if read == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = find_head_end(&buffer) {
            if end > MAX_HEADER_BYTES {
                return Err(Response::error(
                    413,
                    "header_too_large",
                    "request header section exceeds 16 KiB",
                ));
            }
            break Some(end);
        }
    };
    let Some(head_end) = head_end else {
        return Err(Response::error(
            400,
            "bad_request",
            "request header section did not terminate",
        ));
    };
    let head = std::str::from_utf8(&buffer[..head_end])
        .map_err(|_| Response::error(400, "bad_request", "request header is not UTF-8"))?
        .to_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_ascii_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let version = parts.next().unwrap_or("");
    let request_line_valid = !method.is_empty()
        && !path.is_empty()
        && matches!(version, "HTTP/1.0" | "HTTP/1.1")
        && parts.next().is_none();

    let mut malformed_header = false;
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| match line.split_once(':') {
            Some((key, value)) if !key.trim().is_empty() => {
                Some((key.trim().to_owned(), value.trim().to_owned()))
            }
            _ => {
                malformed_header = true;
                None
            }
        })
        .collect();

    let authorized = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case("authorization"))
        .filter_map(|(_, value)| value.strip_prefix("Bearer "))
        .any(|presented| {
            constant_time::verify_slices_are_equal(expected_token, presented.as_bytes()).is_ok()
        });
    if !authorized {
        return Err(Response::new(401, UNAUTHORIZED_BODY));
    }
    if !request_line_valid || malformed_header {
        return Err(Response::error(
            400,
            "bad_request",
            "malformed request head",
        ));
    }
    Ok((method, path, headers, buffer[head_end + 4..].to_vec()))
}

fn find_head_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

/// One `read` bounded by the WHOLE-request deadline, so a client that keeps
/// each individual read inside the timeout still cannot hold the connection.
fn read_within(
    stream: &mut TcpStream,
    deadline: Instant,
    into: &mut [u8],
) -> Result<usize, Response> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Response::error(
            408,
            "request_timeout",
            "request not received within the read deadline",
        ));
    }
    stream
        .set_read_timeout(Some(remaining))
        .map_err(|error| Response::error(500, "internal", &error.to_string()))?;
    stream.read(into).map_err(|error| match error.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => Response::error(
            408,
            "request_timeout",
            "request not received within the read deadline",
        ),
        _ => Response::error(400, "bad_request", &error.to_string()),
    })
}

/// The route table.
///
/// ⚠ Anti-rot anchors for ADR-062's gate
/// (`xtask/tests/decision_adrs_and_provisioning.rs`, the `"062"` arm): the four
/// read routes it pins are `GET /v1/a2a/rotation-windows`,
/// `GET /v1/cohort/peer-versions`, `GET /v1/cohort/self-identity` and
/// `GET /v1/spirits/` — matched below by path segments rather than by whole
/// request-line literals, which is why they are named here. A dev who renames
/// a path re-points the ADR in the SAME commit.
///
/// Segment matching, not request-line string comparison: `/v1/spirits/x/sandbox`
/// and `/v1/spirits/x` used to be told apart by a suffix strip, so any new
/// `/v1/spirits/{id}` route would have swallowed the sandbox one depending on
/// declaration order.
fn route<S: SandboxReportSource>(routes: &Routes<'_, S>, request: &mut Incoming<'_>) -> Response {
    // Owned copies so the segment borrows do not conflict with the lazy,
    // `&mut self` body read the POST arms perform.
    let method = request.method.clone();
    let method = method.as_str();
    let path = request.path.clone();
    let segments: Vec<&str> = path
        .split('?')
        .next()
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .collect();
    match segments.as_slice() {
        ["v1", "daemon"] => get_only(method, || match routes.commands {
            Some(port) => {
                let daemon = port.daemon_status();
                Response::value(
                    200,
                    serde_json::json!({
                        "pid": daemon.pid,
                        "boot_nonce": daemon.boot_nonce,
                        "version": daemon.version,
                        "spirit_ids": daemon.spirit_ids,
                    }),
                )
            }
            None => Response::new(404, NOT_FOUND_BODY),
        }),
        ["v1", "a2a", "rotation-windows"] => get_only(method, || rotation_windows(routes)),
        ["v1", "cohort", "peer-versions"] => get_only(method, || peer_versions(routes)),
        ["v1", "cohort", "self-identity"] => get_only(method, || self_identity(routes)),
        ["v1", "spirits", id, "sandbox"] => {
            let id = match validate_id(id) {
                Ok(id) => id,
                Err(response) => return response,
            };
            get_only(method, || match routes.source.sandbox_report(id) {
                Some(report) => match serde_json::to_vec(&report) {
                    Ok(body) => Response::new(200, body),
                    Err(_) => Response::new(500, INTERNAL_BODY),
                },
                None => Response::new(404, NOT_FOUND_BODY),
            })
        }
        ["v1", "spirits", id] => {
            let id = match validate_id(id) {
                Ok(id) => id,
                Err(response) => return response,
            };
            get_only(method, || match routes.commands {
                Some(port) => match port.spirit_status(id) {
                    Some(row) => Response::value(
                        200,
                        serde_json::json!({
                            "spirit_id": row.spirit_id,
                            "pid": row.pid,
                            "boot_nonce": row.boot_nonce,
                            "lifecycle_state": row.lifecycle_state,
                            "posture": row.posture,
                        }),
                    ),
                    None => Response::new(404, NOT_FOUND_BODY),
                },
                None => Response::new(404, NOT_FOUND_BODY),
            })
        }
        ["v1", "spirits", id, verb] => {
            let id = match validate_id(id) {
                Ok(id) => id,
                Err(response) => return response,
            };
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            spirit_command(routes, id, verb, request)
        }
        ["v1", "halts", halt_id, "resolve"] => {
            let halt_id = match validate_id(halt_id) {
                Ok(halt_id) => halt_id,
                Err(response) => return response,
            };
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            let spirit_id = match required_str(&body, "spirit_id") {
                Ok(value) => match validate_id(&value) {
                    Ok(_) => value,
                    Err(response) => return response,
                },
                Err(response) => return response,
            };
            let resolution = match required_str(&body, "resolution") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let rationale = match optional_str(&body, "rationale") {
                Ok(value) => value,
                Err(response) => return response,
            };
            submit(
                routes,
                OperatorCommand::ResolveHalt {
                    spirit_id,
                    halt_id: halt_id.to_owned(),
                    resolution,
                    rationale,
                },
            )
        }
        ["v1", "orchestrator", id] => {
            let id = match validate_id(id) {
                Ok(id) => id,
                Err(response) => return response,
            };
            match method {
                "GET" => match routes
                    .commands
                    .and_then(|port| port.orchestrator_status(id))
                {
                    Some(row) => Response::value(
                        200,
                        serde_json::json!({
                            "spirit_id": row.spirit_id,
                            "pending": row.pending,
                            "capacity": row.capacity,
                        }),
                    ),
                    None => Response::new(404, NOT_FOUND_BODY),
                },
                "POST" => {
                    let body = match body_json(request) {
                        Ok(body) => body,
                        Err(response) => return response,
                    };
                    match required_str(&body, "text") {
                        Ok(text) => submit(
                            routes,
                            OperatorCommand::OrchestratorEnqueue {
                                spirit_id: id.to_owned(),
                                text,
                            },
                        ),
                        Err(response) => response,
                    }
                }
                _ => method_not_allowed(),
            }
        }
        ["v1", "tokens", token_id, "revoke"] => {
            let token_id = match validate_id(token_id) {
                Ok(token_id) => token_id.to_owned(),
                Err(response) => return response,
            };
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            submit(routes, OperatorCommand::RevokeToken { token_id })
        }
        ["v1", "revocations"] => match method {
            "GET" => match routes.commands {
                Some(port) => {
                    let rows: Vec<serde_json::Value> = port
                        .applied_revocations()
                        .into_iter()
                        .map(|row| {
                            serde_json::json!({
                                "crl_id": row.crl_id,
                                "matched_count": row.matched_count,
                                "revoked_count": row.revoked_count,
                            })
                        })
                        .collect();
                    Response::value(200, serde_json::json!({ "applied": rows }))
                }
                None => Response::new(404, NOT_FOUND_BODY),
            },
            // The CRL's own bytes, opaque here: only the daemon holds the trust
            // anchor that can verify them.
            "POST" => match request.body() {
                Ok(crl) => submit(
                    routes,
                    OperatorCommand::ImportRevocations { crl: crl.to_vec() },
                ),
                Err(response) => response,
            },
            _ => method_not_allowed(),
        },
        ["v1", "memory", "forget"] => {
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            match required_str(&body, "principal") {
                Ok(principal) => {
                    let reason = match optional_str(&body, "reason") {
                        Ok(value) => value,
                        Err(response) => return response,
                    };
                    submit(routes, OperatorCommand::ForgetMemory { principal, reason })
                }
                Err(response) => response,
            }
        }
        // ⚠ The principal travels in the BODY, not as a path segment, and the
        // route is symmetric with `/v1/memory/forget` for that reason.
        // Measured: every legal-hold principal in the tree is EMAIL-shaped
        // (`held-uninstall@example.org`, `held@example.org`), and `@` is
        // outside the `[A-Za-z0-9._-]` class every path segment is validated
        // against. Keeping `{principal}` in the path would have meant either
        // widening that class for one route or percent-decoding in the door —
        // and a door that decodes its own paths is a door with a second,
        // weaker parser. Both principal-scoped verbs now look the same.
        ["v1", "legal-holds", "release"] => {
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            match required_str(&body, "principal") {
                Ok(principal) => submit(routes, OperatorCommand::ReleaseLegalHold { principal }),
                Err(response) => response,
            }
        }
        ["v1", "governance", "schemas"] => {
            if let Err(response) = post_guard(method, request) {
                return response;
            }
            match body_json(request) {
                Ok(schema) => submit(routes, OperatorCommand::AdmitGovernanceSchema { schema }),
                Err(response) => response,
            }
        }
        _ => Response::new(404, NOT_FOUND_BODY),
    }
}

fn spirit_command<S: SandboxReportSource>(
    routes: &Routes<'_, S>,
    spirit_id: &str,
    verb: &str,
    request: &mut Incoming<'_>,
) -> Response {
    let spirit_id = spirit_id.to_owned();
    let command = match verb {
        "start" => OperatorCommand::Start { spirit_id },
        "pause" => OperatorCommand::Pause { spirit_id },
        "resume" => OperatorCommand::Resume { spirit_id },
        "unload" => OperatorCommand::Unload { spirit_id },
        "uninstall" => OperatorCommand::Uninstall { spirit_id },
        "posture" => {
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            match required_str(&body, "posture") {
                Ok(posture) => OperatorCommand::Posture { spirit_id, posture },
                Err(response) => return response,
            }
        }
        "upgrade" => {
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            let target_manifest = match required_str(&body, "target_manifest") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let policy_value = match optional_str(&body, "policy") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let policy = match policy_value.as_deref() {
                None | Some("hot-swap") => UpgradePolicyRequest::HotSwap,
                Some("cold-swap") => UpgradePolicyRequest::ColdSwap,
                Some(other) => {
                    return Response::error(
                        400,
                        "bad_request",
                        &format!("unknown upgrade policy {other:?}"),
                    )
                }
            };
            let attestation = match optional_str(&body, "attestation") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let vetter_keyring = match optional_str(&body, "vetter_keyring") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let from_version = match optional_str(&body, "from_version") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let candidates = match optional_string_array(&body, "candidates") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let create_plan = match optional_bool(&body, "create_plan") {
                Ok(value) => value.unwrap_or(false),
                Err(response) => return response,
            };
            OperatorCommand::Upgrade {
                spirit_id,
                target_manifest,
                policy,
                attestation,
                vetter_keyring,
                from_version,
                candidates,
                create_plan,
            }
        }
        "hot-swap-precheck" => {
            let body = match body_json(request) {
                Ok(body) => body,
                Err(response) => return response,
            };
            let target_manifest = match optional_str(&body, "target_manifest") {
                Ok(value) => value,
                Err(response) => return response,
            };
            OperatorCommand::HotSwapPrecheck {
                spirit_id,
                target_manifest,
            }
        }
        _ => return Response::new(404, NOT_FOUND_BODY),
    };
    submit(routes, command)
}

/// Submit a command and wait for it inside the route budget.
fn submit<S: SandboxReportSource>(routes: &Routes<'_, S>, command: OperatorCommand) -> Response {
    let Some(port) = routes.commands else {
        return Response::new(404, NOT_FOUND_BODY);
    };
    match submit_and_wait(port, command) {
        SubmitOutcome::Completed(outcome) => outcome.into_response(),
        SubmitOutcome::Internal => Response::new(500, INTERNAL_BODY),
        SubmitOutcome::SpiritBusy => Response::new(503, SPIRIT_BUSY_BODY),
        SubmitOutcome::HandlerStillRunning { operation_id } => Response::value(
            503,
            serde_json::json!({
                "error": "handler_still_running",
                "operation_id": operation_id,
            }),
        ),
    }
}

/// Story 16-2 / §15 R4 — what ONE submit-and-withdraw did, as a typed value.
///
/// The HTTP server maps this to its existing responses (byte-for-byte), and
/// the shell's [`ShellHost`](maos_bin::shell_host::ShellHost) maps it to REPL
/// lines. Neither re-implements the withdraw CAS — this function is the one
/// copy.
#[derive(Debug, Clone, PartialEq)]
pub enum SubmitOutcome {
    /// The handler ran and delivered an outcome.
    Completed(OperatorOutcome),
    /// The completion channel closed without delivering — the handler died.
    Internal,
    /// The budget expired and the withdraw CAS WON: the command never ran.
    SpiritBusy,
    /// The budget expired and the withdraw CAS LOST: the port had already
    /// committed to running, so the task finishes and writes its row — the
    /// id is findable.
    HandlerStillRunning { operation_id: String },
}

/// Story 16-2 / §15 R4 — THE one submit-and-withdraw implementation.
///
/// `port.submit`, wait on the completion channel for the command's route
/// budget, and on timeout perform the `Queued → Withdrawn` CAS that decides
/// `SpiritBusy` vs `HandlerStillRunning`. The server's route handler and the
/// shell's in-process resolution both call THIS; before this extraction each
/// would have had to copy the CAS, and a copied CAS is how "withdrew" and
/// "still running" start meaning different things on different surfaces.
pub fn submit_and_wait(port: &dyn OperatorCommandPort, command: OperatorCommand) -> SubmitOutcome {
    let budget = command.route_budget();
    submit_and_wait_with_deadline(port, command, budget)
}

/// The same protocol with a caller-chosen deadline — the seam the
/// `crates/maos-control/tests/` vectors use to drive the timeout arms in
/// milliseconds instead of the 10 s route budget.
pub fn submit_and_wait_with_deadline(
    port: &dyn OperatorCommandPort,
    command: OperatorCommand,
    budget: Duration,
) -> SubmitOutcome {
    let submission = port.submit(command, budget);
    match submission.completion.recv_timeout(budget) {
        Ok(outcome) => SubmitOutcome::Completed(outcome),
        Err(mpsc::RecvTimeoutError::Disconnected) => SubmitOutcome::Internal,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // ⚠ The shared word decides, never the clock. If this CAS wins the
            // command was still QUEUED and is now WITHDRAWN: it never ran, so
            // there is nothing to look up and no id is handed out. If it loses,
            // the port already CASed to STARTED and the task runs to
            // completion and writes its row — so the id IS findable.
            if submission
                .state
                .compare_exchange(
                    COMMAND_QUEUED,
                    COMMAND_WITHDRAWN,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                SubmitOutcome::SpiritBusy
            } else {
                SubmitOutcome::HandlerStillRunning {
                    operation_id: submission.operation_id,
                }
            }
        }
    }
}

/// Story 16-1 / D-16-1-G — a read route answers 405 to every other method,
/// with a byte-equal fixed body, AFTER authentication.
///
/// The three HEAD `POST → 404` assertions (rotation windows, peer versions,
/// self-identity) become guards here rather than being narrowed: ADR-062 `:63`
/// preserves self-identity as a read, and "this host's signed and serving
/// certificate identity" is exactly the second trust path a mutating verb
/// would invent.
fn get_only(method: &str, handler: impl FnOnce() -> Response) -> Response {
    if method == "GET" {
        handler()
    } else {
        method_not_allowed()
    }
}

/// A POST-only route: refuse the wrong method FIRST (405), then enforce the
/// body bound (411 without `Content-Length`, 413 over 64 KiB) for EVERY POST.
///
/// The bound applies even to verbs that take no parameters — `pause` reads
/// nothing from its body, but a POST with no declared length is still an
/// unbounded stream into this process, and ADR-062 `:57-58` bounds the body,
/// not the parse.
///
/// The order matters the other way too: a 411 must never overtake a 405, or
/// `POST /v1/cohort/self-identity` would answer "declare a length" about a
/// route that accepts no writes at all.
fn post_guard(method: &str, request: &mut Incoming<'_>) -> Result<(), Response> {
    if method != "POST" {
        return Err(method_not_allowed());
    }
    request.body().map(|_| ())
}

fn method_not_allowed() -> Response {
    Response::new(405, METHOD_NOT_ALLOWED_BODY)
}

/// Server-side id validation. The client validated too, but a door that trusts
/// its client for path segments is a door with no validation: `maosctl` is not
/// the only thing that can reach a loopback port with a bearer token.
fn validate_id<'a>(id: &'a str) -> Result<&'a str, Response> {
    if id.is_empty() || id.len() > 128 {
        return Err(Response::error(
            400,
            "bad_request",
            "identifier must be 1..=128 characters",
        ));
    }
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(Response::error(
            400,
            "bad_request",
            "identifier must match [A-Za-z0-9._-]",
        ));
    }
    Ok(id)
}

/// An empty body is `{}` — `POST /v1/spirits/x/pause` takes no parameters, and
/// requiring `{}` on the wire would be ceremony.
fn body_json(request: &mut Incoming<'_>) -> Result<serde_json::Value, Response> {
    let body = request.body()?;
    if body.is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_slice(body)
        .map_err(|error| Response::error(400, "bad_request", &error.to_string()))
}

fn required_str(body: &serde_json::Value, key: &str) -> Result<String, Response> {
    match body.get(key).and_then(serde_json::Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.to_owned()),
        _ => Err(Response::error(
            400,
            "bad_request",
            &format!("field `{key}` must be a non-empty string"),
        )),
    }
}

fn optional_str(body: &serde_json::Value, key: &str) -> Result<Option<String>, Response> {
    match body.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Ok(Some(value.to_owned()))
        }
        _ => Err(Response::error(
            400,
            "bad_request",
            &format!("field `{key}` must be a non-empty string or null"),
        )),
    }
}

fn optional_bool(body: &serde_json::Value, key: &str) -> Result<Option<bool>, Response> {
    match body.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(Response::error(
            400,
            "bad_request",
            &format!("field `{key}` must be a boolean or null"),
        )),
    }
}

fn optional_string_array(body: &serde_json::Value, key: &str) -> Result<Vec<String>, Response> {
    match body.get(key) {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    Response::error(
                        400,
                        "bad_request",
                        &format!("field `{key}` must contain only strings"),
                    )
                })
            })
            .collect(),
        _ => Err(Response::error(
            400,
            "bad_request",
            &format!("field `{key}` must be an array or null"),
        )),
    }
}

fn rotation_windows<S: SandboxReportSource>(routes: &Routes<'_, S>) -> Response {
    let Some(status) = routes
        .rotation
        .and_then(|source| source.open_rotation_windows())
    else {
        return Response::new(404, NOT_FOUND_BODY);
    };
    let windows = match status {
        RotationWindowStatus::Healthy(windows) => windows,
        RotationWindowStatus::Unhealthy { detail } => {
            return Response::value(
                503,
                serde_json::json!({
                    "error": "rotation_status_unhealthy",
                    "detail": detail,
                }),
            );
        }
    };
    let rows: Vec<serde_json::Value> = windows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "peer": row.peer,
                "retiring": row.retiring,
                "next": row.next,
                "declared": row.declared,
                "state": row.state,
                "manifest_version": row.manifest_version,
                "opened_at_secs": row.opened_at_secs,
            })
        })
        .collect();
    Response::value(200, serde_json::json!({ "open_windows": rows }))
}

fn peer_versions<S: SandboxReportSource>(routes: &Routes<'_, S>) -> Response {
    // Story 14-2b / AC3 — the SAME auth gate every other route uses,
    // deliberately: a second authentication path for the same operator surface
    // is a second thing to get wrong.
    let Some(status) = routes
        .convergence
        .and_then(|source| source.peer_manifest_versions())
    else {
        return Response::new(404, NOT_FOUND_BODY);
    };
    let observations = match status {
        PeerVersionStatus::Healthy(rows) => rows,
        PeerVersionStatus::Unhealthy { detail } => {
            return Response::value(
                503,
                serde_json::json!({
                    "error": "peer_versions_unhealthy",
                    "detail": detail,
                }),
            );
        }
    };
    let rows: Vec<serde_json::Value> = observations
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "peer": row.peer,
                "declared_version": row.declared_version,
                "declared_hash": row.declared_hash,
                "observed_at_secs": row.observed_at_secs,
                "valid": row.valid,
                "state": row.state,
            })
        })
        .collect();
    // ⚠ The key is `peer_versions`, NOT `converged`: an empty array means NO
    // peer has declared a version to this host, and naming the array after
    // agreement would make that read as agreement.
    Response::value(200, serde_json::json!({ "peer_versions": rows }))
}

fn self_identity<S: SandboxReportSource>(routes: &Routes<'_, S>) -> Response {
    let Some(status) = routes
        .self_identity
        .and_then(|source| source.self_identity())
    else {
        return Response::new(404, NOT_FOUND_BODY);
    };
    let identity = match status {
        SelfIdentityStatus::Healthy(identity) => identity,
        SelfIdentityStatus::Unhealthy { detail } => {
            return Response::value(
                503,
                serde_json::json!({
                    "error": "self_identity_unhealthy",
                    "detail": detail,
                }),
            );
        }
    };
    Response::value(
        200,
        serde_json::json!({
            "self_identity": {
                "declared": identity.declared,
                "serving": identity.serving,
                "verdict": identity.verdict,
                "peers_observed": identity.peers_observed,
                "peers_total": identity.peers_total,
                "last_pull_errors": identity.last_pull_errors,
            }
        }),
    )
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), std::io::Error> {
    // ⚠ Every status this server can emit has its phrase here. Before Story
    // 16-1 the table held 200/401/404/503 and everything else rendered as
    // "Internal Server Error", so a 405 would have been indistinguishable from
    // a crash on the wire.
    let phrase = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {phrase}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

/// Write the response, then drain whatever the client still had in flight and
/// close both directions.
///
/// Not politeness — correctness. Closing a socket that still holds unread
/// bytes in its receive queue makes the kernel send RST instead of FIN, and an
/// RST lets the peer's stack discard data it had not yet handed to the
/// application. Every refusal this server makes WITHOUT reading the body
/// (401, 405, 411, 413, and the pool-exhaustion 503) is therefore exactly the
/// answer most at risk of being lost — the operator would see a closed
/// connection instead of the reason. Measured: the 17th concurrent request
/// intermittently read zero bytes before this drain existed.
fn respond_and_close(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
) -> Result<(), std::io::Error> {
    let written = respond(stream, status, "application/json", body);
    let _ = stream.flush();
    let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
    let mut sink = [0_u8; 4096];
    // Bounded: a client that keeps writing after a refusal does not get to
    // hold this thread. Eight chunks is enough to clear a refused body's
    // already-in-flight segments.
    for _ in 0..8 {
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(_) => continue,
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
    written
}
