#![forbid(unsafe_code)]

//! Story 6.2 AC5 / AC6 + Story 8.12 — CliWrapperSpirit runtime **stdio bridge**.
//!
//! Story 6.2 landed the cap-token-binding helper ([`argv_prefix_hash`]) under a
//! doc comment describing the bridge as "scaffolding deferred to v0.5-α". Story
//! 8.12 graduates that scaffolding to a **working subprocess bridge**: a real
//! spawn, a per-child OS reader thread per stream, a framing layer keyed on
//! `posture.stdio_shape`, a control channel keyed on `posture.control_channel`,
//! and a recovery state-machine executor that *executes* the decision returned
//! by [`super::lifecycle::handle_subprocess_death`] (it never re-derives policy
//! — the executor moves bytes and restarts processes; it does not know what a
//! "founder loop" is — Winston trip-wire).
//!
//! ## FR52 invocation flow (unchanged contract)
//!
//! 1. Invoking Spirit obtains a `Scope::CliSubprocessSpawn` cap-token via the
//!    existing CapabilityRegistryPort (`verify` is the 5µs P99 hot path).
//! 2. The bridge re-derives the manifest's `argv_prefix_hash` at spawn and
//!    asserts equality with the hash bound into the cap-token at issue-time
//!    (TOCTOU correctness per ADR-023). Divergence ⇒ [`BridgeError::CapBindingMismatch`].
//! 3. Spawns the subprocess under the admitted [`BridgeSpawnSpec::sandbox`]
//!    (Story 17-6 AC5): `T2` through `spawn_sandboxed` on Linux (refused on
//!    every other OS, never a bare fallback); `T0`/`T3` directly via
//!    [`std::process::Command`] (the T3 container variant is 17-1's); any other
//!    tier is refused, typed ([`BridgeError::SandboxRefused`]).
//! 4. Each captured stdout/stderr line is written to the Transparency Log as a
//!    `FrameKind::CliSubprocessOutput = 21` row via the **real**
//!    [`TransparencyLogAdapter::insert_frame_event_with_sender`], which routes
//!    every payload through the I2 write-or-panic path AND the redaction
//!    scrubber (`self.redaction.redact` — 32-hex tokens never land in the log,
//!    the Story-8.2 redaction-trap discipline). Spawn-env credentials are
//!    host-injected and **never journaled**.
//! 5. Every captured row carries the sender identity captured at spawn
//!    (`from_spirit_id`) + the invoking Spirit's `intent_lineage`.
//! 6. On subprocess exit the bridge writes a `FrameKind::CapabilityInvocation`
//!    audit row and invokes the caller-supplied revoke closure (the composition
//!    root revokes the cap-token with `RevokeReason::CliSubprocessExit` — the
//!    cap-policy stays in `maos-capability`, not in this byte-moving bridge).
//!
//! ## ADR-010 sync-port / async-kernel
//!
//! The long-lived bridge uses dedicated **OS reader threads** (`std::thread`),
//! NOT the admission probe's poll-to-completion. Each thread owns one child
//! stream, frames in-thread, and hands frames across a **bounded** `mpsc` to the
//! kernel. The bridge owns the threads + their `JoinHandle`s and defines
//! drop/shutdown order: on `Drop` it closes stdin, kills+reaps the child (no
//! `<defunct>` zombie), then joins the readers (no orphaned threads).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

use sha2::Digest;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;

use crate::capability::cap_audit;
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};
use crate::security::sandbox::SandboxSpec;
#[cfg(target_os = "linux")]
use crate::security::sandbox::{spawn_sandboxed, SandboxedChild};

/// Story 6.2 AC6 — recompute the manifest's `argv_prefix_hash` for cap-token
/// binding verification. Re-derived at runtime; asserted equal to the
/// hash bound into the issued cap-token at issue-time per ADR-023 TOCTOU
/// correctness.
pub fn argv_prefix_hash(argv_prefix: &[String]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    for arg in argv_prefix {
        hasher.update((arg.len() as u32).to_le_bytes());
        hasher.update(arg.as_bytes());
    }
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

// ────────────────────────────────────────────────────────────────────────────
// Story 8.12 AC1 — the stdio bridge.
// ────────────────────────────────────────────────────────────────────────────

/// Which child stream a captured line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubStream {
    Stdout,
    Stderr,
}

impl SubStream {
    fn as_str(self) -> &'static str {
        match self {
            SubStream::Stdout => "stdout",
            SubStream::Stderr => "stderr",
        }
    }
}

/// Backpressure policy on the bounded reader→kernel channel (AC1 pinned seam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backpressure {
    /// Block the reader thread until the kernel drains a slot. Lossless;
    /// the default for audit-complete capture (no line is ever dropped).
    Block,
    /// Drop the line and increment an audited drop counter when the channel
    /// is full. Bounds memory under a runaway-output child at the cost of
    /// completeness; the drop count is surfaced in [`PumpOutcome::dropped`].
    DropWithAudit,
}

/// Disambiguated subprocess exit cause (ADR-022): signal-death is distinct from
/// exit-code death so the crash record never conflates `kill -9` with `exit 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitCause {
    /// Process called `exit(code)`.
    Exited { code: i32 },
    /// Process was terminated by a signal (SIGKILL/SIGSEGV/…).
    Signaled { signal: i32 },
    /// Neither a code nor a signal was recoverable from the status.
    Unknown,
}

impl ExitCause {
    /// ADR-022 crash predicate. EOF + **zero** exit is a clean finish — NOT a
    /// crash (the false-positive that pages people). Any signal death OR any
    /// non-zero exit code is a crash.
    pub fn is_crash(&self) -> bool {
        match self {
            ExitCause::Exited { code } => *code != 0,
            ExitCause::Signaled { .. } => true,
            ExitCause::Unknown => true,
        }
    }

    /// The exit code, if the process exited normally (None for signal death).
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            ExitCause::Exited { code } => Some(*code),
            _ => None,
        }
    }

    fn describe(&self) -> String {
        match self {
            ExitCause::Exited { code } => format!("exited(code={code})"),
            ExitCause::Signaled { signal } => format!("signaled(sig={signal})"),
            ExitCause::Unknown => "unknown".to_string(),
        }
    }

    fn classify(status: std::process::ExitStatus) -> ExitCause {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = status.signal() {
                return ExitCause::Signaled { signal: sig };
            }
        }
        match status.code() {
            Some(code) => ExitCause::Exited { code },
            None => ExitCause::Unknown,
        }
    }
}

/// Errors raised by the bridge. All are fail-loud — the bridge never silently
/// degrades a posture, a recovery policy, or a cap-token binding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BridgeError {
    /// Subprocess spawn failed (binary missing, exec error).
    #[error("cli bridge spawn failed: {0}")]
    Spawn(String),
    /// ADR-023 TOCTOU: the `argv_prefix_hash` re-derived at spawn does not match
    /// the hash bound into the cap-token at issue-time. The bridge REFUSES to
    /// run — a divergent argv prefix is exactly the substitution the cap-token
    /// binding exists to prevent.
    #[error("cli bridge cap-token binding mismatch (ADR-023 TOCTOU): argv_prefix_hash diverged at spawn")]
    CapBindingMismatch,
    /// AC1 / FORK C: `RespawnWithContext` is deferred (Epic 10 / NFR-Rel-3 HSIS).
    /// A manifest declaring it fails loud — NO silent downgrade to RespawnFresh
    /// or Escalate. The variant is reserved, not silently degraded.
    #[error(
        "cli bridge recovery_policy=respawn_with_context is not supported at v0.9 \
         (deferred to Epic 10 / NFR-Rel-3 HSIS — see Story 8.12 Deferred Work); \
         the executor fails loud rather than silently downgrading the policy"
    )]
    RespawnWithContextUnsupported,
    /// A control-channel operation is not available for the declared channel.
    #[error("cli bridge control operation unsupported for channel {0:?}: {1}")]
    ControlUnsupported(CliWrapperControlChannel, String),
    /// Generic I/O failure on the control channel or wait path.
    #[error("cli bridge io error: {0}")]
    Io(String),
    /// AC6 `ci_default` hermetic guard tripped — a real agent CLI or a network
    /// request was seen on the hermetic Tier-1 path (use `--live` for Tier-2).
    #[error("ci_default hermetic guard tripped: {0}")]
    CiGuardTripped(String),
    /// Story 17-6 AC5 — the admitted sandbox could not be applied (a refused T2
    /// spawn, or a tier no spawn path applies). Never a bare fallback.
    #[error("cli bridge sandbox refused: {0}")]
    SandboxRefused(String),
}

/// Story 8.12 AC6 — the `ci_default` hermetic guard.
///
/// On the hermetic Tier-1 path the bridge must spawn ONLY the deterministic
/// fixture-CLI: **zero network**, **no real agent CLI**. This guard trips
/// (returns `Err`) if pointed at a known real agent CLI (`claude`/`opencode`/
/// `gemini`/`kimi`) or if a network egress is requested. A guard with no
/// failure-mode test is decoration — its trip behavior is proven in
/// `tests::ci_default_guard_trips_on_real_cli`.
pub fn ci_default_guard(program: &str, network_requested: bool) -> Result<(), BridgeError> {
    const REAL_AGENT_CLIS: &[&str] = &[
        "claude",
        "opencode",
        "gemini",
        "gemini-cli",
        "kimi",
        "kimi-cli",
    ];
    if network_requested {
        return Err(BridgeError::CiGuardTripped(
            "network egress requested on the hermetic ci_default path (Tier-2 needs --live)".into(),
        ));
    }
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    if REAL_AGENT_CLIS.iter().any(|c| base == *c) {
        return Err(BridgeError::CiGuardTripped(format!(
            "real agent CLI '{base}' invoked on the hermetic ci_default path (Tier-2 needs --live)"
        )));
    }
    Ok(())
}

/// Spec for a single subprocess spawn through the bridge (AC1).
#[maos_attrs::i9_exempt(
    reason = "live CLI subprocess spawn spec; carries host-injected process \
              environment (credentials) and argv into the bridge — a transient \
              value object, not cached kernel domain state"
)]
pub struct BridgeSpawnSpec {
    /// Resolved absolute path or PATH-resolvable name of the CLI binary.
    pub program: String,
    /// Story 17-6 AC5 — the sandbox admission granted this child (tier, scopes,
    /// caps, cgroup id). The bridge applies it; it never invents one.
    pub sandbox: SandboxSpec,
    /// Manifest argv prefix (`["code"]` for `claude code`). Hashed for the
    /// cap-token binding assertion.
    pub argv_prefix: Vec<String>,
    /// Per-invocation task arguments appended after the prefix.
    pub task_args: Vec<String>,
    /// The `argv_prefix_hash` bound into the cap-token at issue-time. Re-derived
    /// from `argv_prefix` at spawn and asserted equal (ADR-023).
    pub expected_argv_prefix_hash: [u8; 32],
    /// Sender identity captured at spawn and carried into every journaled row
    /// (AC1 pinned seam (b): the reader thread is off-kernel and holds no kernel
    /// context, so `insert_frame_event_with_sender` is given a principal here).
    pub from_spirit_id: String,
    /// On-wire framing for the child's streams.
    pub stdio_shape: CliWrapperStdioShape,
    /// Control-channel mechanism for pause/resume/unload.
    pub control_channel: CliWrapperControlChannel,
    /// Signal name dispatched on `on_unload` for `Signals` channels (advisory at
    /// v0.9 — see [`SpawnedBridge::on_unload`]).
    pub shutdown_signal: Option<String>,
    /// Bounded reader→kernel channel capacity.
    pub channel_capacity: usize,
    /// Backpressure policy on that bounded channel.
    pub backpressure: Backpressure,
    /// Host-injected environment (credentials). NEVER journaled; passed only to
    /// the child's process environment.
    pub env: Vec<(String, String)>,
}

/// A message handed from an OS reader thread to the kernel over the bounded mpsc.
#[derive(Debug)]
enum ReaderMsg {
    Line {
        stream: SubStream,
        line_no: u64,
        bytes: Vec<u8>,
    },
    Eof {
        stream: SubStream,
    },
    FramingError {
        stream: SubStream,
        error: String,
    },
}

/// A live subprocess bridge: owns the child, its stdin (control channel), the
/// reader threads, and the bounded receiver. RAII — `Drop` kills+reaps the child
/// and joins the readers so no orphaned process or thread survives.
#[maos_attrs::i9_exempt(
    reason = "live subprocess bridge RAII handle; owns OS resources (child \
              process, stdin control channel, reader thread JoinHandles, \
              bounded receiver, drop-counter) for one in-flight CLI invocation \
              — process-bound resource ownership, not cached kernel domain state"
)]
pub struct SpawnedBridge {
    child: Option<BridgeChild>,
    stdin: Option<ChildStdin>,
    readers: Vec<JoinHandle<()>>,
    rx: Option<Receiver<ReaderMsg>>,
    child_pid: u32,
    from_spirit_id: String,
    control_channel: CliWrapperControlChannel,
    shutdown_signal: Option<String>,
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
    sandbox_tier: SandboxTier,
    /// Stdout reached EOF: the stream is closed to `poll_frame` callers.
    stdout_eof: bool,
}

/// Outcome of draining the child's streams to the journal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PumpOutcome {
    pub stdout_lines: u64,
    pub stderr_lines: u64,
    /// Lines dropped under `Backpressure::DropWithAudit` (always 0 for `Block`).
    pub dropped: u64,
    /// Journal write failures — `insert_frame_event_with_sender` returned `Err`.
    /// Non-zero means audit trail gaps; the pump still continues draining.
    pub journal_failures: u64,
}

/// Owned duplex input; move to a writer thread so output is drained concurrently.
#[maos_attrs::i9_exempt(
    reason = "exclusive live child stdin resource, transferred to the duplex writer"
)]
pub struct FrameWriter {
    stdin: ChildStdin,
}

impl FrameWriter {
    pub fn write_frame(&mut self, body: &[u8]) -> std::io::Result<()> {
        maos_frame_codec::write_frame(&mut self.stdin, body)
    }
}

/// Transport facts only; the composition root owns deadlines and protocol state.
pub enum BridgePoll {
    Data(SubStream, Vec<u8>),
    Idle,
    Closed,
}
/// Result of waiting for and finalizing a subprocess (ADR-022).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeExit {
    pub cause: ExitCause,
    pub child_pid: u32,
}

/// Read one newline-delimited record (`NdjsonOverStdio` / `Raw`). `Raw` has no
/// self-delimiting boundary, so its **explicit, documented** boundary is the LF
/// byte: read-to-newline, buffered. Returns `Ok(None)` on EOF.
fn read_newline_delimited<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut buf = Vec::new();
    let n = reader.read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
        buf.pop();
    }
    Ok(Some(buf))
}

/// Reader-thread body: frame `reader` per `shape`, hand frames to `tx` per the
/// backpressure policy. Sends a terminal `Eof` then returns (closing the thread).
fn run_reader<R: BufRead>(
    mut reader: R,
    stream: SubStream,
    shape: CliWrapperStdioShape,
    tx: SyncSender<ReaderMsg>,
    backpressure: Backpressure,
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
) {
    let mut line_no: u64 = 0;
    loop {
        let frame = match (stream, shape) {
            (SubStream::Stderr, _) => maos_frame_codec::read_diagnostic(&mut reader),
            (_, CliWrapperStdioShape::NdjsonOverStdio | CliWrapperStdioShape::Raw) => {
                read_newline_delimited(&mut reader)
            }
            (_, CliWrapperStdioShape::JsonRpcOverStdio) => {
                maos_frame_codec::read_frame(&mut reader)
            }
            _ => {
                // Unknown framing variant — fail loud (no silent downgrade).
                let _ = tx.send(ReaderMsg::FramingError {
                    stream,
                    error: format!("unsupported stdio_shape: {shape:?}"),
                });
                return;
            }
        };
        match frame {
            Ok(Some(mut bytes)) => {
                if stream == SubStream::Stderr {
                    while matches!(bytes.last(), Some(b'\n') | Some(b'\r')) {
                        bytes.pop();
                    }
                }
                line_no += 1;
                let msg = ReaderMsg::Line {
                    stream,
                    line_no,
                    bytes,
                };
                match backpressure {
                    Backpressure::Block => {
                        // Lossless: a closed receiver means the kernel dropped
                        // the bridge — stop reading.
                        if tx.send(msg).is_err() {
                            return;
                        }
                    }
                    // Story 16-5 T0 disposal: these drops are SUBPROCESS
                    // OUTPUT LINES on the bridge channel, never CapAuditEvent
                    // sends — they are counted in PumpOutcome::dropped and
                    // are out of the audit-drop class (T0's tenth shape).
                    Backpressure::DropWithAudit => match tx.try_send(msg) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {
                            dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(TrySendError::Disconnected(_)) => return,
                    },
                }
            }
            Ok(None) => {
                let _ = tx.send(ReaderMsg::Eof { stream });
                return;
            }
            Err(e) => {
                // A framing/read error — surface as FramingError so the pump
                // can log it. The exit-cause path (ADR-022) still classifies
                // the death from the child's exit status.
                let _ = tx.send(ReaderMsg::FramingError {
                    stream,
                    error: e.to_string(),
                });
                return;
            }
        }
    }
}

/// Story 17-6 AC5 — the bridge child: a bare spawn, or one confined by
/// `spawn_sandboxed`, whose guard owns the child and its cgroup dir.
enum BridgeChild {
    Plain(Child),
    #[cfg(target_os = "linux")]
    Sandboxed(SandboxedChild),
}

impl BridgeChild {
    fn get(&mut self) -> &mut Child {
        match self {
            Self::Plain(child) => child,
            #[cfg(target_os = "linux")]
            Self::Sandboxed(guard) => guard.child_mut(),
        }
    }

    /// The child's cgroup directory while its guard is alive.
    #[cfg(target_os = "linux")]
    fn cgroup_path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Plain(_) => None,
            Self::Sandboxed(guard) => guard.cgroup_path(),
        }
    }
}

/// `oom_kill` from a cgroup's `memory.events` (0 when absent: no memory controller).
#[cfg(target_os = "linux")]
fn cgroup_oom_kills(cgroup: &std::path::Path) -> u64 {
    std::fs::read_to_string(cgroup.join("memory.events"))
        .map_or(0, |events| parse_oom_kills(&events))
}

#[cfg(target_os = "linux")]
fn parse_oom_kills(events: &str) -> u64 {
    events
        .lines()
        .find_map(|line| line.strip_prefix("oom_kill "))
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or(0)
}

impl Drop for BridgeChild {
    fn drop(&mut self) {
        if let Self::Plain(child) = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Spawn under the admitted spec: T2 through `spawn_sandboxed` on Linux only (never
/// a bare fallback); T0 and T3 bare (T3 is applied by 17-1 AC4); all else refused.
fn spawn_child(
    spec: &SandboxSpec,
    cmd: &mut Command,
    program: &str,
) -> Result<BridgeChild, BridgeError> {
    match spec.tier {
        #[cfg(target_os = "linux")]
        SandboxTier::T2 => spawn_sandboxed(spec, cmd)
            .map(BridgeChild::Sandboxed)
            .map_err(|e| BridgeError::SandboxRefused(format!("{program}: {e}"))),
        SandboxTier::T0 | SandboxTier::T3 => cmd
            .spawn()
            .map(BridgeChild::Plain)
            .map_err(|e| BridgeError::Spawn(format!("{program}: {e}"))),
        tier => Err(BridgeError::SandboxRefused(format!(
            "{program}: no spawn path applies tier {tier:?}"
        ))),
    }
}

/// Spawn a subprocess under its admitted sandbox and start its reader threads
/// (AC1; Story 17-6 AC5). The cap-token binding is asserted BEFORE the child
/// can produce a single byte.
pub fn spawn_and_bridge(spec: BridgeSpawnSpec) -> Result<SpawnedBridge, BridgeError> {
    // ADR-023 TOCTOU — re-derive the argv_prefix_hash and assert the binding
    // BEFORE spawning. A divergent prefix is exactly the substitution the
    // cap-token binding exists to prevent; fail loud, never run.
    let observed = argv_prefix_hash(&spec.argv_prefix);
    if observed != spec.expected_argv_prefix_hash {
        return Err(BridgeError::CapBindingMismatch);
    }

    let mut argv: Vec<String> = spec.argv_prefix.clone();
    argv.extend(spec.task_args.iter().cloned());

    let mut cmd = Command::new(&spec.program);
    cmd.args(&argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Host-injected credentials → child env ONLY. Never journaled.
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }

    let mut child = spawn_child(&spec.sandbox, &mut cmd, &spec.program)?;
    let child_pid = child.get().id();
    // Framed sessions need duplex stdin even when lifecycle control uses signals.
    // Argv-driven CLIs still receive EOF immediately.
    let stdin = match (spec.control_channel, spec.stdio_shape) {
        (CliWrapperControlChannel::Signals, shape)
            if shape != CliWrapperStdioShape::JsonRpcOverStdio =>
        {
            drop(child.get().stdin.take());
            None
        }
        _ => child.get().stdin.take(),
    };
    let stdout = child
        .get()
        .stdout
        .take()
        .ok_or_else(|| BridgeError::Spawn("stdout not captured".into()))?;
    let stderr = child
        .get()
        .stderr
        .take()
        .ok_or_else(|| BridgeError::Spawn("stderr not captured".into()))?;

    let (tx, rx) = std::sync::mpsc::sync_channel::<ReaderMsg>(spec.channel_capacity.max(1));
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let shape = spec.stdio_shape;
    let bp = spec.backpressure;

    let tx_out = tx.clone();
    let drop_out = std::sync::Arc::clone(&dropped);
    let out_handle = std::thread::Builder::new()
        .name(format!("cli-bridge-stdout-{child_pid}"))
        .spawn(move || {
            run_reader(
                BufReader::new(stdout),
                SubStream::Stdout,
                shape,
                tx_out,
                bp,
                drop_out,
            );
        })
        .map_err(|e| BridgeError::Io(format!("spawn stdout reader: {e}")))?;

    let drop_err = std::sync::Arc::clone(&dropped);
    let err_handle = match std::thread::Builder::new()
        .name(format!("cli-bridge-stderr-{child_pid}"))
        .spawn(move || {
            run_reader(
                BufReader::new(stderr),
                SubStream::Stderr,
                shape,
                tx,
                bp,
                drop_err,
            );
        }) {
        Ok(handle) => handle,
        Err(error) => {
            drop(rx);
            let _ = child.get().kill();
            let _ = child.get().wait();
            let _ = out_handle.join();
            return Err(BridgeError::Io(format!("spawn stderr reader: {error}")));
        }
    };

    Ok(SpawnedBridge {
        child: Some(child),
        stdin,
        readers: vec![out_handle, err_handle],
        rx: Some(rx),
        child_pid,
        from_spirit_id: spec.from_spirit_id,
        control_channel: spec.control_channel,
        shutdown_signal: spec.shutdown_signal,
        dropped,
        sandbox_tier: spec.sandbox.tier,
        stdout_eof: false,
    })
}

impl SpawnedBridge {
    /// The OS PID of the spawned child. Carried into journaled rows for the AC6
    /// anti-theater proof (`child_pid != std::process::id()`).
    pub fn child_pid(&self) -> u32 {
        self.child_pid
    }

    /// The sender identity captured at spawn.
    pub fn from_spirit_id(&self) -> &str {
        &self.from_spirit_id
    }

    /// Observe a terminal child without turning its already-occurred fault into
    /// a planned kill. `Child` retains a reaped status for finalization.
    pub fn try_exit_cause(&mut self) -> Result<Option<ExitCause>, BridgeError> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| BridgeError::Io("child already finalized".into()))?;
        child
            .get()
            .try_wait()
            .map(|status| status.map(ExitCause::classify))
            .map_err(|error| BridgeError::Io(error.to_string()))
    }

    /// SIGKILL a not-yet-reaped child. `Ok` when it already exited or was
    /// finalized. Does not reap: [`Self::wait_and_finalize`] still does.
    pub fn kill(&mut self) -> Result<(), BridgeError> {
        match self.child.as_mut().map(|child| child.get().kill()) {
            None | Some(Ok(())) => Ok(()),
            // Older std reports an already-reaped child as InvalidInput.
            Some(Err(error)) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
            Some(Err(error)) => Err(BridgeError::Io(format!("kill: {error}"))),
        }
    }

    /// Report a kernel-observed sandbox enforcement on the audit channel; when it
    /// cannot be delivered (no channel, full, closed) the bridge journals the
    /// row the audit writer would have written (kind 8 `SandboxBlock`), so it is
    /// never silently lost and the escape consumer reads either path alike.
    #[cfg(unix)]
    fn report_sandbox_block(
        &self,
        journal: &TransparencyLogAdapter,
        spirit_pid: u32,
        audit: Option<&cap_audit::Sender>,
        attempted_syscall: String,
    ) {
        let event = cap_audit::CapAuditEvent::SandboxBlock {
            spirit_pid,
            attempted_syscall: attempted_syscall.clone(),
            sandbox_tier: self.sandbox_tier,
        };
        if let Some(audit) = audit {
            match audit.try_send(event) {
                Ok(()) => return,
                Err(error) => {
                    cap_audit::record_send_error(cap_audit::AuditDropSite::SandboxBlock, &error);
                }
            }
        }
        let _ = journal.insert_frame_event_with_sender(
            FrameKind::SandboxBlock,
            spirit_pid,
            &self.from_spirit_id,
            "",
            None,
            &format!("sandbox.block.{attempted_syscall}"),
            format!("tier={}", self.sandbox_tier.0).as_bytes(),
            FrameOrigin::Kernel,
        );
    }

    /// Transfer exclusive stdin ownership to a concurrent framed writer.
    pub fn take_frame_writer(&mut self) -> Result<FrameWriter, BridgeError> {
        self.stdin
            .take()
            .map(|stdin| FrameWriter { stdin })
            .ok_or_else(|| BridgeError::Io("bridge framed stdin already closed or taken".into()))
    }

    /// Poll opaque bodies/diagnostics while the owner also services its mailbox.
    /// Framing failures are terminal errors, not clean EOF or idle timeout. Once
    /// stdout reaches EOF the stream is closed to the owner: diagnostics still
    /// arriving are returned, and the first quiet `timeout` (or stderr EOF)
    /// yields [`BridgePoll::Closed`], even if a descendant holds stderr open.
    /// `timeout` is one deadline per call.
    pub fn poll_frame(&mut self, timeout: std::time::Duration) -> Result<BridgePoll, BridgeError> {
        let rx = self
            .rx
            .as_ref()
            .ok_or_else(|| BridgeError::Io("bridge receiver closed".into()))?;
        let mut deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let message = rx.recv_timeout(left).map_err(|error| match error {
                std::sync::mpsc::RecvTimeoutError::Timeout if !self.stdout_eof => BridgePoll::Idle,
                _ => BridgePoll::Closed,
            });
            match message {
                Ok(ReaderMsg::Line { stream, bytes, .. }) => {
                    return Ok(BridgePoll::Data(stream, bytes))
                }
                Ok(ReaderMsg::Eof { stream }) => {
                    if stream == SubStream::Stdout && !self.stdout_eof {
                        // Trailing diagnostics get one full quiet window.
                        self.stdout_eof = true;
                        deadline = std::time::Instant::now() + timeout;
                    }
                }
                Ok(ReaderMsg::FramingError { stream, error }) => {
                    return Err(BridgeError::Io(format!("{stream:?} framing: {error}")))
                }
                Err(outcome) => return Ok(outcome),
            }
        }
    }

    /// Drain both child streams to the Transparency Log until EOF on both,
    /// emitting one `FrameKind::CliSubprocessOutput = 21` row per line via the
    /// real `insert_frame_event_with_sender` (which redacts + I2-guards the
    /// payload). Blocks until the child closes both streams. For a long-lived
    /// streaming CLI the caller runs this on a dedicated thread; the founder-loop
    /// fixture is short-lived so the demo pumps inline.
    pub fn pump_to_journal(
        &mut self,
        journal: &TransparencyLogAdapter,
        spirit_pid: u32,
        to_spirit_id: &str,
        cli_name: &str,
        intent_lineage: &[String],
    ) -> PumpOutcome {
        let mut out = PumpOutcome::default();
        let mut eofs = 0u8;
        let rx = self
            .rx
            .as_ref()
            .expect("pump_to_journal: rx already taken (double-pump?)");
        while eofs < 2 {
            match rx.recv() {
                Ok(ReaderMsg::Line {
                    stream,
                    line_no,
                    bytes,
                }) => {
                    match stream {
                        SubStream::Stdout => out.stdout_lines += 1,
                        SubStream::Stderr => out.stderr_lines += 1,
                    }
                    // The line bytes may be non-UTF8 or carry a secret; the TL
                    // redaction scrubber runs on the full payload at insert.
                    let line_str = String::from_utf8_lossy(&bytes);
                    let payload = serde_json::json!({
                        "cli": cli_name,
                        "stream": stream.as_str(),
                        "line": line_str,
                        "line_no": line_no,
                        "child_pid": self.child_pid,
                        "intent_lineage": intent_lineage,
                    });
                    // I2 write-or-panic: insert_frame_event_with_sender returns
                    // LogBeforeDeliver (not Result) — it panics on write failure.
                    // The journal_failures counter stays 0 under I2 but exists for
                    // future non-panic journal backends.
                    let _ = journal.insert_frame_event_with_sender(
                        FrameKind::CliSubprocessOutput,
                        spirit_pid,
                        &self.from_spirit_id,
                        to_spirit_id,
                        None,
                        "cli.subprocess.output",
                        payload.to_string().as_bytes(),
                        FrameOrigin::Kernel,
                    );
                }
                Ok(ReaderMsg::Eof { .. }) => {
                    eofs += 1;
                }
                Ok(ReaderMsg::FramingError { stream, error }) => {
                    eprintln!("cli_wrapper: framing error on {stream:?}: {error}");
                    eofs += 1;
                }
                Err(_) => break, // both readers gone
            }
        }
        out.dropped = self.dropped.load(std::sync::atomic::Ordering::Relaxed);
        out
    }

    /// Wait for the child to exit, classify the cause (ADR-022: signal-death vs
    /// exit-code death disambiguated), journal a `FrameKind::CapabilityInvocation`
    /// exit row, then invoke the caller-supplied revoke closure (the composition
    /// root revokes the `Scope::CliSubprocessSpawn` cap-token with
    /// `RevokeReason::CliSubprocessExit{exit_code}` — cap-policy is NOT in this
    /// bridge). Reaps the zombie (no `<defunct>`).
    pub fn wait_and_finalize<F>(
        &mut self,
        journal: &TransparencyLogAdapter,
        spirit_pid: u32,
        audit: Option<&cap_audit::Sender>,
        revoke_on_exit: F,
    ) -> BridgeExit
    where
        F: FnOnce(Option<i32>),
    {
        let cause = match self.child.as_mut() {
            Some(child) => match child.get().wait() {
                Ok(status) => ExitCause::classify(status),
                Err(_) => ExitCause::Unknown,
            },
            None => ExitCause::Unknown,
        };
        // Read the cgroup's OOM count while the guard still owns the directory.
        #[cfg(target_os = "linux")]
        let oom_kills = self
            .child
            .as_ref()
            .and_then(BridgeChild::cgroup_path)
            .map_or(0, cgroup_oom_kills);
        // The child is now reaped (wait consumed it). Mark consumed so Drop does
        // not double-wait.
        self.child = None;

        #[cfg(unix)]
        if self.sandbox_tier == SandboxTier::T2
            && cause
                == (ExitCause::Signaled {
                    signal: libc::SIGSYS,
                })
        {
            self.report_sandbox_block(
                journal,
                spirit_pid,
                audit,
                "seccomp-kill-process (SIGSYS; syscall number unavailable)".into(),
            );
        }
        #[cfg(target_os = "linux")]
        if self.sandbox_tier == SandboxTier::T2 && oom_kills > 0 {
            self.report_sandbox_block(
                journal,
                spirit_pid,
                audit,
                format!("resource-cap: cgroup memory.max oom_kill x{oom_kills}"),
            );
        }

        let payload = serde_json::json!({
            "event": "cli_subprocess_exit",
            "cli_child_pid": self.child_pid,
            "exit_cause": cause.describe(),
            "is_crash": cause.is_crash(),
        });
        let _ = journal.insert_frame_event_with_sender(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            &self.from_spirit_id,
            "",
            None,
            "cli.subprocess.exit",
            payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );

        revoke_on_exit(cause.exit_code());

        BridgeExit {
            cause,
            child_pid: self.child_pid,
        }
    }

    /// Bench/test support — write a raw line (newline appended) to the child's
    /// stdin. Used by the J1 measurement to drive a request→response round-trip
    /// through the real bridge framing path (AC4). Returns the underlying I/O
    /// error if stdin is closed.
    pub fn write_stdin_line(&mut self, line: &[u8]) -> std::io::Result<()> {
        match self.stdin.as_mut() {
            Some(stdin) => {
                stdin.write_all(line)?;
                stdin.write_all(b"\n")?;
                stdin.flush()
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "bridge stdin already closed",
            )),
        }
    }

    /// Bench/test support — block for the next framed line from the reader
    /// thread, WITHOUT journaling it. Returns `None` on EOF/disconnect. Used by
    /// the J1 measurement to isolate the bridge IPC overhead from SQLite writes.
    pub fn recv_line(&mut self) -> Option<(SubStream, Vec<u8>)> {
        let rx = self.rx.as_ref().expect("recv_line: rx already taken");
        loop {
            match rx.recv() {
                Ok(ReaderMsg::Line { stream, bytes, .. }) => return Some((stream, bytes)),
                Ok(ReaderMsg::Eof { .. }) => continue,
                Ok(ReaderMsg::FramingError { .. }) => continue,
                Err(_) => return None,
            }
        }
    }

    /// `on_pause` lifecycle hook, keyed on `posture.control_channel`.
    pub fn on_pause(&mut self) -> Result<(), BridgeError> {
        self.control("pause")
    }

    /// `on_resume` lifecycle hook, keyed on `posture.control_channel`.
    pub fn on_resume(&mut self) -> Result<(), BridgeError> {
        self.control("resume")
    }

    fn control(&mut self, verb: &str) -> Result<(), BridgeError> {
        match self.control_channel {
            CliWrapperControlChannel::StdinCommands => {
                if let Some(stdin) = self.stdin.as_mut() {
                    let line = format!("{{\"control\":\"{verb}\"}}\n");
                    stdin
                        .write_all(line.as_bytes())
                        .map_err(|e| BridgeError::Io(format!("control write: {e}")))?;
                    stdin
                        .flush()
                        .map_err(|e| BridgeError::Io(format!("control flush: {e}")))?;
                    Ok(())
                } else {
                    Err(BridgeError::ControlUnsupported(
                        self.control_channel,
                        "stdin already closed".into(),
                    ))
                }
            }
            // SIGSTOP/SIGCONT pause/resume require `libc::kill`, which is
            // `unsafe` and excluded by this crate's `#![forbid(unsafe_code)]`.
            // For short-lived founder-loop fixtures pause/resume are no-ops; a
            // live long-running agent CLI uses the container-control path (the
            // T3 runtime's `stop`/`pause` subcommands) — wired but documented
            // as best-effort at v0.9, NOT silently claimed-as-done.
            CliWrapperControlChannel::Signals | CliWrapperControlChannel::NamedPipe => {
                eprintln!(
                    "cli bridge: {verb} on {:?} channel is a documented v0.9 no-op \
                     (signal/named-pipe pause-resume deferred to the container-control path)",
                    self.control_channel
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// `on_unload` lifecycle hook: dispatch the shutdown per `control_channel`,
    /// close stdin (stream EOF), and kill+reap the child if still alive.
    pub fn on_unload(&mut self) -> Result<(), BridgeError> {
        // Send an in-band shutdown for stdin-based channels first.
        if matches!(
            self.control_channel,
            CliWrapperControlChannel::StdinCommands | CliWrapperControlChannel::NamedPipe
        ) {
            if let Some(stdin) = self.stdin.as_mut() {
                let _ = stdin.write_all(b"{\"control\":\"unload\"}\n");
                let _ = stdin.flush();
            }
        }
        // Close stdin → the child reads EOF on stdin (graceful for CLIs that
        // treat stdin-close as shutdown). `shutdown_signal` (SIGTERM/SIGINT) is
        // honored at the container-stop layer for live CLIs; for the directly
        // spawned fixture path the only `forbid(unsafe_code)`-safe terminator is
        // `Child::kill` (SIGKILL), used below if the child outlives stdin-close.
        let _ = self.shutdown_signal; // advisory; see doc above
        self.stdin = None;
        if let Some(child) = self.child.as_mut() {
            let _ = child.get().kill();
            let _ = child.get().wait();
        }
        self.child = None;
        Ok(())
    }
}
impl Drop for SpawnedBridge {
    fn drop(&mut self) {
        // Defined drop/shutdown order (AC1 pinned seam): close stdin, kill+reap
        // the child (no `<defunct>` zombie), then drop the receiver (unblocking
        // any `Block`ed reader), then join the readers (no orphaned threads).
        self.stdin = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.get().kill();
            let _ = child.get().wait();
        }
        // Drop the receiver first so any `Block`ed reader send returns Err and
        // the thread exits, then join. This MUST happen before joining — without
        // it, a reader blocked on tx.send() (channel full) deadlocks forever.
        self.rx.take();
        for handle in self.readers.drain(..) {
            let _ = handle.join();
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Story 8.12 AC1/AC6 — recovery state-machine executor.
// ────────────────────────────────────────────────────────────────────────────

/// Outcome of executing a recovery decision (AC1/AC6). The executor *executes*
/// the decision from [`super::lifecycle::handle_subprocess_death`]; it never
/// re-derives policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    /// Respawned a fresh child. `transfer_context` is ALWAYS false — `RespawnFresh`
    /// carries NO prior context (AC6 negative assertion).
    Respawned {
        attempt: u32,
        transfer_context: bool,
    },
    /// Escalated to the supervisor: journaled + surfaced, NOT silently
    /// loop-respawned. Reached either by `recovery_policy = escalate` or by the
    /// respawn-attempt bound (AC6).
    Escalated {
        reason: &'static str,
        exit_code: Option<i32>,
    },
}

/// Execute the recovery decision for an observed subprocess death (AC1/AC6).
///
/// `decision` is the value [`super::lifecycle::handle_subprocess_death`] returned
/// — the executor consumes it, it does not re-derive policy. `attempt` is the
/// number of respawns already performed; `max_attempts` bounds the respawn loop
/// (reaching it routes to `Escalate` — an unbounded respawn loop is a missing
/// requirement, so the bound is mandatory). `respawn` performs a single fresh
/// respawn and must NOT transfer context.
pub fn execute_recovery<F>(
    decision: super::lifecycle::RecoveryAction,
    attempt: u32,
    max_attempts: u32,
    mut respawn: F,
) -> Result<RecoveryOutcome, BridgeError>
where
    F: FnMut() -> Result<(), BridgeError>,
{
    use super::lifecycle::RecoveryAction;
    match decision {
        // FORK C: RespawnWithContext is deferred — fail loud, never downgrade.
        RecoveryAction::Respawn {
            transfer_context: true,
            ..
        } => Err(BridgeError::RespawnWithContextUnsupported),
        RecoveryAction::Respawn {
            transfer_context: false,
            exit_code,
        } => {
            if attempt >= max_attempts {
                Ok(RecoveryOutcome::Escalated {
                    reason: "respawn-attempt bound reached",
                    exit_code,
                })
            } else {
                respawn()?;
                Ok(RecoveryOutcome::Respawned {
                    attempt: attempt + 1,
                    transfer_context: false,
                })
            }
        }
        RecoveryAction::Escalate { exit_code } => Ok(RecoveryOutcome::Escalated {
            reason: "recovery_policy=escalate",
            exit_code,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_prefix_hash_empty_is_stable() {
        let h1 = argv_prefix_hash(&[]);
        let h2 = argv_prefix_hash(&[]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn argv_prefix_hash_differs_with_args() {
        let h1 = argv_prefix_hash(&["code".to_string()]);
        let h2 = argv_prefix_hash(&["chat".to_string()]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn argv_prefix_hash_deterministic() {
        let args = vec!["code".to_string(), "--verbose".to_string()];
        let h1 = argv_prefix_hash(&args);
        let h2 = argv_prefix_hash(&args);
        assert_eq!(h1, h2);
    }

    // ── ExitCause / ADR-022 crash classification ──

    #[test]
    fn exit_zero_is_not_a_crash() {
        assert!(!ExitCause::Exited { code: 0 }.is_crash());
    }

    #[test]
    fn exit_nonzero_is_a_crash() {
        assert!(ExitCause::Exited { code: 1 }.is_crash());
        assert_eq!(ExitCause::Exited { code: 1 }.exit_code(), Some(1));
    }

    #[test]
    fn signal_death_is_a_crash_and_has_no_exit_code() {
        let c = ExitCause::Signaled { signal: 9 };
        assert!(c.is_crash());
        assert_eq!(c.exit_code(), None);
    }

    // ── recovery executor ──

    #[test]
    fn respawn_with_context_fails_loud() {
        let decision = super::super::lifecycle::RecoveryAction::Respawn {
            transfer_context: true,
            exit_code: Some(1),
        };
        let r = execute_recovery(decision, 0, 3, || Ok(()));
        assert_eq!(r, Err(BridgeError::RespawnWithContextUnsupported));
    }

    #[test]
    fn respawn_fresh_carries_no_context() {
        let decision = super::super::lifecycle::RecoveryAction::Respawn {
            transfer_context: false,
            exit_code: Some(1),
        };
        let mut spawned = 0;
        let r = execute_recovery(decision, 0, 3, || {
            spawned += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            r,
            RecoveryOutcome::Respawned {
                attempt: 1,
                transfer_context: false
            }
        );
        assert_eq!(spawned, 1);
    }

    #[test]
    fn respawn_bound_routes_to_escalate() {
        let decision = super::super::lifecycle::RecoveryAction::Respawn {
            transfer_context: false,
            exit_code: Some(1),
        };
        // attempt == max_attempts → escalate, no respawn.
        let mut spawned = 0;
        let r = execute_recovery(decision, 3, 3, || {
            spawned += 1;
            Ok(())
        })
        .unwrap();
        assert!(matches!(r, RecoveryOutcome::Escalated { .. }));
        assert_eq!(spawned, 0, "bound reached → no respawn");
    }

    #[test]
    fn escalate_policy_does_not_respawn() {
        let decision = super::super::lifecycle::RecoveryAction::Escalate { exit_code: Some(2) };
        let mut spawned = 0;
        let r = execute_recovery(decision, 0, 3, || {
            spawned += 1;
            Ok(())
        })
        .unwrap();
        assert!(matches!(r, RecoveryOutcome::Escalated { .. }));
        assert_eq!(spawned, 0);
    }

    // ── newline framing ──

    #[test]
    fn newline_framing_strips_crlf() {
        let data = b"alpha\r\nbeta\ngamma";
        let mut r = std::io::BufReader::new(&data[..]);
        assert_eq!(
            read_newline_delimited(&mut r).unwrap(),
            Some(b"alpha".to_vec())
        );
        assert_eq!(
            read_newline_delimited(&mut r).unwrap(),
            Some(b"beta".to_vec())
        );
        assert_eq!(
            read_newline_delimited(&mut r).unwrap(),
            Some(b"gamma".to_vec())
        );
        assert_eq!(read_newline_delimited(&mut r).unwrap(), None);
    }

    // ── ci_default hermetic guard (AC6) ──

    #[test]
    fn ci_default_guard_passes_for_fixture() {
        assert!(ci_default_guard("worker-cli-fixture", false).is_ok());
        assert!(ci_default_guard("/abs/path/to/worker-cli-fixture", false).is_ok());
        assert!(ci_default_guard("sh", false).is_ok());
    }

    #[test]
    fn ci_default_guard_trips_on_real_cli() {
        // The guard must TRIP on a real agent CLI (decoration otherwise).
        for cli in [
            "claude",
            "opencode",
            "gemini",
            "kimi",
            "/usr/local/bin/claude",
        ] {
            assert!(
                matches!(
                    ci_default_guard(cli, false),
                    Err(BridgeError::CiGuardTripped(_))
                ),
                "guard must trip on real CLI {cli}"
            );
        }
    }

    #[test]
    fn ci_default_guard_trips_on_network() {
        assert!(matches!(
            ci_default_guard("worker-cli-fixture", true),
            Err(BridgeError::CiGuardTripped(_))
        ));
    }

    /// P-6: Drop-without-pump must NOT deadlock when the child has filled the
    /// bounded channel. The fix (self.rx.take() before join) prevents the reader
    /// thread from blocking on a full channel whose receiver is still alive.
    #[test]
    fn drop_without_pump_does_not_deadlock() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::Duration;

        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg("yes | head -n 200")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");
        let child_pid = child.id();
        let (tx, rx) = std::sync::mpsc::sync_channel::<ReaderMsg>(8); // small channel
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let out_handle = std::thread::spawn({
            let dropped = dropped.clone();
            move || {
                run_reader(
                    BufReader::new(stdout),
                    SubStream::Stdout,
                    CliWrapperStdioShape::NdjsonOverStdio,
                    tx,
                    Backpressure::Block,
                    dropped,
                )
            }
        });
        let err_handle = std::thread::spawn({
            let dropped = dropped.clone();
            move || {
                let (tx2, _rx2) = std::sync::mpsc::sync_channel::<ReaderMsg>(1);
                run_reader(
                    BufReader::new(stderr),
                    SubStream::Stderr,
                    CliWrapperStdioShape::NdjsonOverStdio,
                    tx2,
                    Backpressure::Block,
                    dropped,
                )
            }
        });

        // Build a minimal SpawnedBridge without calling spawn_and_bridge
        let bridge = SpawnedBridge {
            child: Some(BridgeChild::Plain(child)),
            stdin: None,
            readers: vec![out_handle],
            rx: Some(rx),
            child_pid,
            from_spirit_id: "test".to_string(),
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
            dropped,
            sandbox_tier: SandboxTier::T0,
            stdout_eof: false,
        };

        // Drop without ever calling pump_to_journal. The channel may be full.
        // Before the fix, this would deadlock if the reader blocked on tx.send().
        let start = std::time::Instant::now();
        drop(bridge);
        let _ = err_handle.join();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Drop without pump took {:?} — likely deadlocked",
            start.elapsed()
        );
    }

    /// Regression (J1 live codex): a worker driven by `Signals` is NOT fed via
    /// stdin, so the bridge must close the child's stdin. Otherwise a CLI that
    /// reads stdin-until-EOF (`codex exec`) deadlocks — it blocks reading stdin
    /// while the bridge blocks reading its output. The hermetic fixture never
    /// read stdin, so this stayed invisible through T1–T5. Here a REAL `cat`
    /// blocks on stdin then echoes: it can only reach the echo if the bridge sent
    /// EOF by closing stdin. (`StdinCommands` keeps stdin — the J1 bench drives it
    /// via `write_stdin_line`; not closed here.)
    #[test]
    fn signals_control_closes_worker_stdin_so_a_stdin_reader_proceeds() {
        let argv_prefix = vec!["-c".to_string(), "cat >/dev/null; echo done".to_string()];
        let spec = BridgeSpawnSpec {
            program: "/bin/sh".to_string(),
            sandbox: SandboxSpec::new_for_test(SandboxTier::T0),
            expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
            argv_prefix,
            task_args: vec![],
            from_spirit_id: "worker".to_string(),
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
            channel_capacity: 8,
            backpressure: Backpressure::Block,
            env: vec![],
        };
        let mut bridge = spawn_and_bridge(spec).expect("spawn /bin/sh");

        // The bridge closed the worker's stdin (Signals) → it is gone.
        assert!(
            bridge.write_stdin_line(b"x").is_err(),
            "a Signals worker's stdin must be closed so a stdin-reading CLI gets EOF"
        );

        // And `cat` actually reached `echo done` — proof it received EOF and did
        // not deadlock. Before the fix, stdin stayed open, `cat` blocked forever,
        // and `done` never arrived.
        let line = bridge.recv_line();
        assert!(
            matches!(&line, Some((SubStream::Stdout, b)) if b == b"done"),
            "expected 'done' after EOF-driven completion, got {line:?}"
        );
    }
}
