//! T3 spawn path — `spawn_t3` wraps an in-container command with full
//! container isolation (Linux-only at v0.5-α).
//!
//! # Platform-availability
//!
//! - **Linux:** spawns via `podman run` / `docker run` with `--cap-drop=ALL`,
//!   `--security-opt=no-new-privileges`, `--network=none`, `--read-only`.
//! - **macOS / Windows:** returns `SpawnError::SandboxUnavailable` with
//!   a documented platform-availability message.
//!
//! # T2-inside-T3 layering
//!
//! The container boundary is the outer security ring at v0.5-α.
//! The in-container T2 stack (Landlock+seccomp) is invoked by the Spirit
//! binary's ABI-side `t2_apply()` hook at startup. v0.5-α's smoke arm
//! uses busybox which does NOT call `t2_apply()` — full T2-inside-T3
//! layering activates with Epic 6 subprocess form landing.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

use maos_domain::sandbox::T3Error;

use crate::security::sandbox::{SandboxSpec, SpawnError};

use super::child::SandboxedContainerChild;
use super::image_lock::VerifiedImageAttestation;
use super::image_verify;
use super::runtime_detect::{self};

/// Context for a T3 container spawn.
pub struct T3SpawnContext<'a> {
    /// Path to the Spirit binary inside the container.
    pub spirit_binary_path: &'a Path,
    /// Boot nonce from the SCB.
    pub boot_nonce: u64,
    /// Container name for identification; consumed by the child guard.
    pub container_name: String,
    /// Optional daemon-owned proxy socket.
    pub egress_socket: Option<&'a Path>,
    /// Explicit environment passed to the container (never as argument values).
    pub env: &'a [(String, String)],
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

/// Run one runtime-client invocation under an absolute deadline. A runtime
/// client that never exits is killed at the deadline so no spawn, teardown
/// or inspect path can hang the daemon on a stuck runtime. `None` means the
/// call did not complete within the deadline (or failed to start).
pub(crate) fn runtime_call_bounded(
    runtime: &Path,
    arguments: &[&str],
) -> Option<std::process::Output> {
    const CLIENT_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
    let mut child = runtime_command(runtime)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + CLIENT_DEADLINE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// The runtime must self-report rootless operation before it may carry a T3
/// spawn. `podman info` is the attestation surface; docker and rootful podman
/// do not report rootless and are refused.
fn ensure_rootless(runtime: &runtime_detect::ContainerRuntime) -> Result<(), T3Error> {
    let output = runtime_call_bounded(
        &runtime.path,
        &["info", "--format", "{{.Host.Security.Rootless}}"],
    )
    .ok_or(T3Error::RuntimeUnavailable)?;
    let rootless = String::from_utf8_lossy(&output.stdout).trim() == "true";
    if output.status.success() && rootless {
        Ok(())
    } else {
        Err(T3Error::RuntimeUnavailable)
    }
}

/// Spawn a command inside a T3 container.
///
/// # Platform
/// Linux-only at v0.5-α; other platforms return `SpawnError::SandboxUnavailable`.
///
/// # Steps (Linux)
/// 1. Detect runtime (cached via `OnceLock`).
/// 2. Verify image attestation against trust anchor.
/// 3. Build argv via pure-function `argv::build_runtime_argv`.
/// 4. Spawn via `std::process::Command`.
/// 5. Capture host-namespace PID via `inspect_container_host_pid`.
/// 6. Wrap in `SandboxedContainerChild`.
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
        let runtime = runtime_detect::detect_container_runtime().map_err(|e| {
            SpawnError::T3RuntimeUnavailable {
                reason: e.to_string(),
            }
        })?;
        // ADR-061 M3: the T3 boundary is a ROOTLESS container. A rootful
        // runtime (or the docker fallback) is a silently weaker boundary
        // than every published isolation claim, so it is refused here —
        // applies-or-refuses, never a weaker spawn.
        ensure_rootless(&runtime).map_err(|reason| SpawnError::T3RuntimeUnavailable {
            reason: reason.to_string(),
        })?;
        let entry = image.entry();
        let local_sha =
            image_verify::inspect_image_sha(&runtime, &entry.image_uri).map_err(|error| {
                SpawnError::T3ImageInspect {
                    reason: error.to_string(),
                }
            })?;
        if local_sha != entry.image_sha256 {
            return Err(SpawnError::SandboxImageMismatch {
                expected: hex::encode(entry.image_sha256),
                observed: hex::encode(local_sha),
            });
        }

        let argv = super::argv::build_runtime_argv(
            &runtime,
            image,
            spec,
            parent.spirit_binary_path,
            command,
            &spec.spirit_id,
            parent.boot_nonce,
            &parent.container_name,
            parent.egress_socket,
            parent.env,
        )
        .map_err(|error| SpawnError::T3ImageInspect {
            reason: error.to_string(),
        })?;

        let mut cmd = runtime_command(&argv[0]);
        cmd.args(&argv[1..])
            .envs(parent.env.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = cmd.spawn().map_err(SpawnError::Io)?;
        let mut guard = SandboxedContainerChild {
            child: Some(child),
            host_pid: 0,
            container_name: parent.container_name,
            runtime,
        };
        guard.host_pid = inspect_container_host_pid(&guard.runtime, &guard.container_name)
            .map_err(|e| {
                SpawnError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
        Ok(guard)
    }
}

/// Commit a successful spawn's observed identity to the owning live SCB and
/// journal.  This is deliberately separate from process creation so callers
/// can hand the child to their supervisor without holding a scheduler lock
/// across runtime I/O, while still making the commit explicit and mandatory at
/// the scheduler boundary.
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

/// Inspect the host-namespace PID of a container.
///
/// Runs `<runtime> inspect --format '{{.State.Pid}}' <container_name>`
/// and parses the output as a u32.
fn inspect_container_host_pid(
    runtime: &super::runtime_detect::ContainerRuntime,
    container_name: &str,
) -> Result<u32, T3Error> {
    let max_attempts = 10u32;
    for attempt in 1..=max_attempts {
        let output = runtime_call_bounded(
            &runtime.path,
            &["inspect", "--format", "{{.State.Pid}}", container_name],
        )
        .ok_or_else(|| T3Error::Inspect("inspect container pid failed".into()))?;

        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid > 0 {
                    return Ok(pid);
                }
            }
        }

        if attempt < max_attempts {
            std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
        }
    }

    Err(T3Error::Inspect(format!(
        "container {container_name}: PID not available after {max_attempts} attempts"
    )))
}
