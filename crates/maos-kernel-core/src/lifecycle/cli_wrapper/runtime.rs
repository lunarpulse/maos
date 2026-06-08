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
//! 3. Spawns the subprocess via the AC5-resolved sandbox path (the deterministic
//!    fixture-CLI spawns directly via [`std::process::Command`] — exactly as the
//!    admission probe already does at `admission.rs:73`; the live agent-CLI path
//!    routes through the T3 network-permitted container variant — see AC5 grant
//!    gate in `admission.rs`).
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

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

use sha2::Digest;

use maos_domain::invariants::i3::FrameOrigin;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};

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
pub struct BridgeSpawnSpec {
    /// Resolved absolute path or PATH-resolvable name of the CLI binary.
    pub program: String,
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
pub struct SpawnedBridge {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    readers: Vec<JoinHandle<()>>,
    rx: Option<Receiver<ReaderMsg>>,
    child_pid: u32,
    from_spirit_id: String,
    control_channel: CliWrapperControlChannel,
    shutdown_signal: Option<String>,
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
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

/// Read one `Content-Length:`-framed record (`JsonRpcOverStdio`). Mirrors the
/// J1 bench framing (`j1.rs:63-76`) so the bench and production agree on the
/// wire shape. Returns `Ok(None)` on a clean EOF at a frame boundary.
fn read_content_length<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = String::new();
    let n = reader.read_line(&mut header)?;
    if n == 0 {
        return Ok(None);
    }
    let len: usize = header
        .trim()
        .strip_prefix("Content-Length:")
        .map(|s| s.trim())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("bad Content-Length header: {header:?}"),
            )
        })?;
    // Consume the blank separator line and validate it.
    let mut blank = String::new();
    reader.read_line(&mut blank)?;
    if !blank.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("expected blank separator after Content-Length, got {blank:?}"),
        ));
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
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
        let frame = match shape {
            CliWrapperStdioShape::NdjsonOverStdio | CliWrapperStdioShape::Raw => {
                read_newline_delimited(&mut reader)
            }
            CliWrapperStdioShape::JsonRpcOverStdio => read_content_length(&mut reader),
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
            Ok(Some(bytes)) => {
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

/// Spawn a subprocess and start its reader threads (AC1).
///
/// Spawns directly via [`std::process::Command`] — the deterministic fixture-CLI
/// path, identical to the admission probe at `admission.rs:73`. The cap-token
/// binding is asserted BEFORE the child can produce a single byte.
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

    let mut child = cmd
        .spawn()
        .map_err(|e| BridgeError::Spawn(format!("{}: {e}", spec.program)))?;

    let child_pid = child.id();
    let stdin = child.stdin.take();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| BridgeError::Spawn("stdout not captured".into()))?;
    let stderr = child
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
    let err_handle = std::thread::Builder::new()
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
        })
        .map_err(|e| BridgeError::Io(format!("spawn stderr reader: {e}")))?;

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
                    eprintln!(
                        "cli_wrapper: framing error on {stream:?}: {error}"
                    );
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
        revoke_on_exit: F,
    ) -> BridgeExit
    where
        F: FnOnce(Option<i32>),
    {
        let cause = match self.child.as_mut() {
            Some(child) => match child.wait() {
                Ok(status) => ExitCause::classify(status),
                Err(_) => ExitCause::Unknown,
            },
            None => ExitCause::Unknown,
        };
        // The child is now reaped (wait consumed it). Mark consumed so Drop does
        // not double-wait.
        self.child = None;

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
            let _ = child.kill();
            let _ = child.wait();
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
            let _ = child.kill();
            let _ = child.wait();
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
        assert_eq!(read_newline_delimited(&mut r).unwrap(), Some(b"alpha".to_vec()));
        assert_eq!(read_newline_delimited(&mut r).unwrap(), Some(b"beta".to_vec()));
        assert_eq!(read_newline_delimited(&mut r).unwrap(), Some(b"gamma".to_vec()));
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
        for cli in ["claude", "opencode", "gemini", "kimi", "/usr/local/bin/claude"] {
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

    #[test]
    fn content_length_framing_roundtrips() {
        let body = r#"{"k":"v"}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut r = std::io::BufReader::new(input.as_bytes());
        let got = read_content_length(&mut r).unwrap().unwrap();
        assert_eq!(String::from_utf8(got).unwrap(), body);
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
            child: Some(child),
            stdin: None,
            readers: vec![out_handle],
            rx: Some(rx),
            child_pid,
            from_spirit_id: "test".to_string(),
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
            dropped,
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
}
