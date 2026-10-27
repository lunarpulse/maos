//! T3 spawn path — `spawn_t3` runs one command inside a rootless T3 container
//! (Linux-only; other platforms return `SpawnError::SandboxUnavailable`).
//!
//! # Lifecycle (Story 17-7)
//!
//! A fresh private state directory is validated before any runtime call;
//! `create` writes the private CID receipt, which must agree with stdout, and
//! arms the CID guard at once; `init` makes the runtime write the pidfile while
//! the payload is gated; the pidfile is read through its held inode; then the
//! supervised `start --attach` client runs with no credential environment.
//! Nothing polls for startup and no name is ever a cleanup authority.
//!
//! # T2-inside-T3 layering
//!
//! The container boundary is the outer security ring. The in-container T2
//! stack (Landlock+seccomp) is the Spirit binary's own `t2_apply()` hook.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use maos_domain::sandbox::{parse_container_id, parse_pidfile, T3Error};

use crate::security::sandbox::{SandboxSpec, SpawnError};

use super::child::{ContainerLaunchCustody, SandboxedContainerChild};
use super::image_lock::VerifiedImageAttestation;

/// Every runtime-client call is bounded by this absolute deadline.
pub const CLIENT_DEADLINE: Duration = Duration::from_secs(10);

/// Context for a T3 container spawn.
#[maos_attrs::i9_exempt(
    reason = "borrowed per-spawn launch custody, released when startup returns"
)]
pub struct T3SpawnContext<'a> {
    /// Path to the Spirit binary mounted at `/maos/spirit`.
    pub spirit_binary_path: &'a Path,
    /// Boot nonce from the SCB.
    pub boot_nonce: u64,
    /// Requested runtime name; never a cleanup authority.
    pub container_name: String,
    /// Optional daemon-owned proxy socket.
    pub egress_socket: Option<&'a Path>,
    /// Fresh, private, euid-owned, empty per-spawn directory.
    pub state_dir: &'a Path,
    /// `false` forces the runtime-evidence fallback (test seam).
    pub payload_pidfd: bool,
    /// Explicit environment, given only to the `create` client.
    pub env: &'a [(String, String)],
    /// Shared root/binding custody registered before the create runtime call.
    pub launch_custody: Option<&'a std::sync::Arc<ContainerLaunchCustody>>,
}

/// Construct a runtime client command with ambient credentials removed.
pub(crate) fn runtime_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_clear();
    for key in [
        "PATH",
        "HOME",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

#[doc(hidden)]
#[derive(Debug)]
pub enum Invocation {
    Exited {
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        stdout_complete: bool,
        stderr_complete: bool,
    },
    TimedOut,
    Failed,
}

impl Invocation {
    pub(crate) fn succeeded(&self) -> bool {
        matches!(self, Self::Exited { status, .. } if status.success())
    }
}

/// One runtime-client call under an absolute deadline. Both pipes drain
/// concurrently and are capped; a client alive at the deadline is killed and
/// reaped off-thread, and a descendant holding a pipe cannot postpone the
/// result past the deadline.
#[cfg(target_os = "linux")]
pub(crate) fn invoke_bounded_command(
    command: &mut Command,
    args: &[&str],
    env: &[(String, String)],
    deadline: Instant,
) -> Invocation {
    use std::os::fd::AsRawFd;
    const CAP: usize = 64 * 1024;
    const DRAIN_GRACE: Duration = Duration::from_millis(200);
    fn drain(
        pipe: &mut Option<impl std::io::Read + AsRawFd>,
        buf: &mut Vec<u8>,
        complete: &mut bool,
        overflow: &mut bool,
        ready: &[libc::pollfd],
    ) {
        let Some(fd) = pipe.as_ref().map(AsRawFd::as_raw_fd) else {
            return;
        };
        if !ready.iter().any(|p| p.fd == fd && p.revents != 0) {
            return;
        }
        let mut chunk = [0u8; 8192];
        match pipe.as_mut().map(|p| p.read(&mut chunk)) {
            Some(Ok(0)) => {
                *pipe = None;
                *complete = true;
            }
            Some(Err(_)) => *pipe = None,
            Some(Ok(n)) => {
                let available = CAP.saturating_sub(buf.len());
                let captured = n.min(available);
                buf.extend_from_slice(&chunk[..captured]);
                *overflow |= captured != n;
            }
            None => {}
        }
    }
    if Instant::now() >= deadline {
        return Invocation::TimedOut;
    }
    let spawned = command
        .args(args)
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = spawned else {
        return Invocation::Failed;
    };
    let (mut out, mut err) = (child.stdout.take(), child.stderr.take());
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let (mut stdout_eof, mut stderr_eof) = (false, false);
    let (mut stdout_overflow, mut stderr_overflow) = (false, false);
    let (mut status, mut until) = (None, deadline);
    loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exited)) => {
                    status = Some(exited);
                    until = deadline.min(Instant::now() + DRAIN_GRACE);
                }
                Ok(None) => {}
                Err(_) => break,
            }
        }
        let now = Instant::now();
        if (status.is_some() && out.is_none() && err.is_none()) || now >= until {
            break;
        }
        let mut fds = [libc::pollfd {
            fd: -1,
            events: libc::POLLIN,
            revents: 0,
        }; 2];
        let mut fds_len = 0;
        if let Some(fd) = out.as_ref().map(AsRawFd::as_raw_fd) {
            fds[fds_len].fd = fd;
            fds_len += 1;
        }
        if let Some(fd) = err.as_ref().map(AsRawFd::as_raw_fd) {
            fds[fds_len].fd = fd;
            fds_len += 1;
        }
        let slice = (until - now).min(Duration::from_millis(10));
        if fds_len == 0
            || crate::security::sandbox::linux::poll_ready(&mut fds[..fds_len], slice).is_err()
        {
            std::thread::sleep(slice);
            continue;
        }
        drain(
            &mut out,
            &mut stdout,
            &mut stdout_eof,
            &mut stdout_overflow,
            &fds[..fds_len],
        );
        drain(
            &mut err,
            &mut stderr,
            &mut stderr_eof,
            &mut stderr_overflow,
            &fds[..fds_len],
        );
    }
    match status {
        Some(status) => Invocation::Exited {
            status,
            stdout,
            stderr,
            stdout_complete: stdout_eof && !stdout_overflow,
            stderr_complete: stderr_eof && !stderr_overflow,
        },
        None => {
            let _ = child.kill();
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = std::thread::Builder::new().spawn(move || child.wait());
            }
            Invocation::TimedOut
        }
    }
}

#[doc(hidden)]
#[cfg(target_os = "linux")]
pub fn invoke_bounded(
    program: &Path,
    args: &[&str],
    env: &[(String, String)],
    deadline: Instant,
) -> Invocation {
    invoke_bounded_command(&mut runtime_command(program), args, env, deadline)
}

#[doc(hidden)]
#[cfg(not(target_os = "linux"))]
pub fn invoke_bounded(_: &Path, _: &[&str], _: &[(String, String)], _: Instant) -> Invocation {
    Invocation::Failed
}

/// Read a small runtime-written receipt without ever blocking on the path:
/// hold the inode `O_PATH | O_NOFOLLOW`, require a regular file, reopen the held
/// inode through procfs non-blocking, re-check its identity, read `<= max` bytes.
#[doc(hidden)]
#[cfg(target_os = "linux")]
pub fn read_receipt(path: &Path, max: usize) -> Option<Vec<u8>> {
    read_receipt_after_hold(path, max, || {})
}

/// Deterministic held-inode seam: `after_hold` runs after `O_PATH` opens the
/// pathname and before bytes are read from that descriptor.
#[doc(hidden)]
#[cfg(target_os = "linux")]
pub fn read_receipt_after_hold(
    path: &Path,
    max: usize,
    after_hold: impl FnOnce(),
) -> Option<Vec<u8>> {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let held = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .ok()?;
    after_hold();
    let meta = held.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(format!("/proc/self/fd/{}", held.as_raw_fd()))
        .ok()?;
    let again = file.metadata().ok()?;
    if !again.is_file() || again.dev() != meta.dev() || again.ino() != meta.ino() {
        return None;
    }
    let mut bytes = Vec::with_capacity(max + 1);
    file.take(max as u64 + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= max).then_some(bytes)
}

/// The per-spawn state directory must be a fresh, empty, mode-0700 directory
/// owned by the daemon's effective uid. Checked before any runtime call.
#[doc(hidden)]
#[cfg(target_os = "linux")]
pub fn validate_state_dir(dir: &Path) -> Result<(), T3Error> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(dir).map_err(|_| T3Error::StateDir("missing"))?;
    let euid = std::fs::metadata("/proc/self")
        .map_err(|_| T3Error::StateDir("effective uid unknown"))?
        .uid();
    if !meta.is_dir() {
        Err(T3Error::StateDir("not a directory"))
    } else if meta.uid() != euid {
        Err(T3Error::StateDir("not owned by the daemon's effective uid"))
    } else if meta.mode() & 0o7777 != 0o700 {
        Err(T3Error::StateDir("not mode 0700"))
    } else if std::fs::read_dir(dir).map_or(true, |mut entries| entries.next().is_some()) {
        Err(T3Error::StateDir("not empty"))
    } else {
        Ok(())
    }
}

/// Trust a pidfd on the pidfile PID only if `/proc/<pid>/cgroup` names the CID
/// in an exact component and the pidfd is still live after that read.
#[cfg(target_os = "linux")]
fn open_payload_pidfd(pid: u32, cid: &str) -> Option<std::os::fd::OwnedFd> {
    use crate::security::sandbox::linux;
    use std::os::fd::AsFd;
    let pidfd = linux::pidfd_open(pid).ok()?;
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    let owned = maos_domain::sandbox::cgroup_names_container(&cgroup, cid);
    (owned && !linux::readable(pidfd.as_fd(), Duration::ZERO)).then_some(pidfd)
}

/// The runtime must self-report rootless operation before it may carry a T3
/// spawn. Docker and rootful podman do not report rootless and are refused.
#[cfg(target_os = "linux")]
fn ensure_rootless(runtime: &super::runtime_detect::ContainerRuntime) -> Result<(), T3Error> {
    match super::spawn::invoke_bounded(
        &runtime.path,
        &["info", "--format", "{{.Host.Security.Rootless}}"],
        &[],
        Instant::now() + CLIENT_DEADLINE,
    ) {
        Invocation::Exited {
            status,
            stdout,
            stdout_complete: true,
            ..
        } if status.success() && String::from_utf8_lossy(&stdout).trim() == "true" => Ok(()),
        _ => Err(T3Error::RuntimeUnavailable),
    }
}

/// Spawn a command inside a T3 container (Linux only).
pub fn spawn_t3(
    spec: &SandboxSpec,
    image: &VerifiedImageAttestation,
    command: &[String],
    parent: T3SpawnContext<'_>,
) -> Result<SandboxedContainerChild, SpawnError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (spec, image, command, parent);
        return Err(SpawnError::SandboxUnavailable {
            reason: "T3 container isolation not yet implemented on this platform; \
                     pending macOS/Windows CI runners and container-runtime equivalents \
                     — Linux Podman/Docker is the v0.5 baseline"
                .into(),
        });
    }

    #[cfg(target_os = "linux")]
    {
        let refused = |error: T3Error| SpawnError::SandboxSetup(error.to_string());
        validate_state_dir(parent.state_dir).map_err(refused)?;
        let runtime = super::runtime_detect::detect_container_runtime().map_err(|e| {
            SpawnError::T3RuntimeUnavailable {
                reason: e.to_string(),
            }
        })?;
        // ADR-061 M3: the T3 boundary is a ROOTLESS container; a rootful
        // runtime is refused, never a silently weaker spawn.
        ensure_rootless(&runtime).map_err(|reason| SpawnError::T3RuntimeUnavailable {
            reason: reason.to_string(),
        })?;
        let entry = image.entry();
        let local_sha = super::image_verify::inspect_image_sha(&runtime, &entry.image_uri)
            .map_err(|error| SpawnError::T3ImageInspect {
                reason: error.to_string(),
            })?;
        if local_sha != entry.image_sha256 {
            return Err(SpawnError::SandboxImageMismatch {
                expected: hex::encode(entry.image_sha256),
                observed: hex::encode(local_sha),
            });
        }
        launch_t3(runtime, spec, image, command, parent)
    }
}

/// The lifecycle after the runtime gates: `create` (receipt + stdout) → CID
/// guard → `init` → pidfile → verified pidfd → `start --attach`. `spawn_t3`
/// calls it after the rootless and image gates; tests drive it with a stub.
#[doc(hidden)]
#[cfg(target_os = "linux")]
pub fn launch_t3(
    runtime: super::runtime_detect::ContainerRuntime,
    spec: &SandboxSpec,
    image: &VerifiedImageAttestation,
    command: &[String],
    parent: T3SpawnContext<'_>,
) -> Result<SandboxedContainerChild, SpawnError> {
    let mut owned_guard = None;
    let _launch_permit =
        match parent.launch_custody {
            Some(custody) => Some(custody.begin().ok_or_else(|| {
                SpawnError::SandboxSetup("T3 launch cancelled before create".into())
            })?),
            None => None,
        };
    let refused = |error: T3Error| SpawnError::SandboxSetup(error.to_string());
    validate_state_dir(parent.state_dir).map_err(refused)?;
    let argv = super::argv::build_runtime_argv(
        &runtime,
        image,
        spec,
        parent.spirit_binary_path,
        command,
        &spec.spirit_id,
        parent.boot_nonce,
        &parent.container_name,
        parent.state_dir,
        parent.egress_socket,
        parent.env,
    )
    .map_err(|error| SpawnError::T3ImageInspect {
        reason: error.to_string(),
    })?;
    if parent
        .launch_custody
        .is_some_and(|custody| custody.is_cancelled())
    {
        return Err(SpawnError::SandboxSetup(
            "T3 launch cancelled before create".into(),
        ));
    }
    let args: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
    let create = invoke_bounded(
        &runtime.path,
        &args,
        parent.env,
        Instant::now() + CLIENT_DEADLINE,
    );
    // The private receipt is the sole cleanup authority, even when the
    // create client failed or timed out after the runtime committed.
    let receipt = read_receipt(&parent.state_dir.join("cid"), 65);
    let Some(cid) = receipt.and_then(|bytes| parse_container_id(&bytes)) else {
        if let Some(custody) = parent.launch_custody {
            custody.mark_ownership_unknown();
        }
        return Err(refused(T3Error::CreationOwnershipUnknown {
            phase: "create",
            locator: parent.state_dir.display().to_string(),
        }));
    };
    owned_guard = Some(SandboxedContainerChild::own(
        runtime,
        cid.clone(),
        parent.state_dir,
        parent.launch_custody.cloned(),
    ));
    let guard = owned_guard
        .as_mut()
        .expect("validated receipt has an owned CID guard");
    let launch = |phase, reason| {
        refused(T3Error::LaunchRefused {
            phase,
            container_id: cid.clone(),
            reason,
        })
    };
    if let Some(custody) = parent.launch_custody {
        if !custody.publish_terminal(std::sync::Arc::clone(guard.terminal())) {
            return Err(launch("create", "launch cancelled"));
        }
    }
    let echoed = match &create {
        Invocation::Exited {
            status,
            stdout,
            stdout_complete: true,
            ..
        } if status.success() => parse_container_id(stdout.trim_ascii_end()),
        _ => {
            return Err(launch(
                "create",
                "create client failed or output was incomplete",
            ))
        }
    };
    if echoed.as_deref() != Some(cid.as_str()) {
        return Err(launch("create", "create stdout disagrees with the receipt"));
    }
    if parent
        .launch_custody
        .is_some_and(|custody| custody.is_cancelled())
    {
        return Err(launch("init", "launch cancelled"));
    }
    // conmon <= 2.1.10 writes `oom` into its cwd (cgroup.c:331): `init` runs in the state dir.
    if !invoke_bounded_command(
        runtime_command(&guard.terminal().runtime().path).current_dir(parent.state_dir),
        &["init", &cid],
        &[],
        Instant::now() + CLIENT_DEADLINE,
    )
    .succeeded()
    {
        return Err(launch("init", "init failed"));
    }
    if parent
        .launch_custody
        .is_some_and(|custody| custody.is_cancelled())
    {
        return Err(launch("init", "launch cancelled"));
    }
    let pid = read_receipt(&parent.state_dir.join("pid"), 11)
        .and_then(|bytes| parse_pidfile(&bytes))
        .ok_or_else(|| launch("init", "pidfile missing, special or invalid"))?;
    let pidfd = parent
        .payload_pidfd
        .then(|| open_payload_pidfd(pid, &cid))
        .flatten();
    guard.arm(pid, pidfd);
    if parent
        .launch_custody
        .is_some_and(|custody| custody.is_cancelled())
    {
        return Err(launch("start", "launch cancelled"));
    }
    let client = runtime_command(&guard.terminal().runtime().path)
        .args(["start", "--attach", &cid])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| launch("start", "start client failed"))?;
    guard.child = Some(client);
    if parent
        .launch_custody
        .is_some_and(|custody| custody.is_cancelled())
    {
        return Err(launch("start", "launch cancelled"));
    }
    Ok(owned_guard
        .take()
        .expect("successful launch retains its CID guard"))
}

/// Commit a successful spawn's observed identity to the owning live SCB and
/// journal. Separate from process creation so callers never hold a scheduler
/// lock across runtime I/O.
pub fn commit_spawn_report(
    scheduler: &crate::scheduler::SpiritSchedulerAdapter,
    journal: &dyn maos_domain::ports::scheduler::SpiritSchedulerPort,
    child: &SandboxedContainerChild,
    spirit_id: String,
    image: &VerifiedImageAttestation,
) -> Result<(), SpawnError> {
    scheduler
        .record_sandbox_application(child.inspect_report(spirit_id, image), journal)
        .map_err(|reason| SpawnError::T3ReportCommit { reason })
}
