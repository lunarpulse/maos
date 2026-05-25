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

use std::path::PathBuf;
use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;
use maos_domain::sandbox::{T3Error, T3ImageAttestation};

use crate::security::sandbox::{SandboxSpec, SpawnError};

use super::child::SandboxedContainerChild;
use super::image_verify;
use super::runtime_detect::{self};

/// Context for a T3 container spawn.
#[derive(Debug, Clone)]
pub struct T3SpawnContext {
    /// Path to the Spirit binary inside the container.
    pub spirit_binary_path: PathBuf,
    /// Boot nonce from the SCB.
    pub boot_nonce: u64,
    /// Container name for identification.
    pub container_name: String,
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
    image: &T3ImageAttestation,
    command: &[String],
    parent: T3SpawnContext,
) -> Result<SandboxedContainerChild, SpawnError> {
    #[cfg(not(target_os = "linux"))]
    {
        return Err(SpawnError::SandboxUnavailable {
            reason: "T3 container isolation not yet implemented on this platform; \
                     pending macOS/Windows CI runners and container-runtime equivalents \
                     — Linux Podman/Docker is the v0.5 baseline".into(),
        });
    }

    #[cfg(target_os = "linux")]
    {
        // 1. Detect runtime (cached after first call via OnceLock).
        let runtime = runtime_detect::detect_container_runtime()
            .map_err(|e| SpawnError::T3RuntimeUnavailable {
                reason: e.to_string(),
            })?;

        // 2. Verify local image SHA matches the attestation's pin
        //    (v0.5-α: placeholder — see image_verify::inspect_image_sha)
        if let Some(entry) = image.entries.first() {
            let local_sha = image_verify::inspect_image_sha(&runtime, &entry.image_uri)
                .map_err(|e| SpawnError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))?;
            if local_sha != entry.image_sha256 {
                return Err(SpawnError::SandboxImageMismatch {
                    expected: hex::encode(entry.image_sha256),
                    observed: hex::encode(local_sha),
                });
            }
        }

        // 3. Build argv via the pure-function argv builder.
        //    `parent.container_name` is the canonical identity used both
        //    for `--name` and for inspect/cleanup — argv and spawn MUST
        //    agree (Story 5.5a review finding §argv-divergence).
        let argv = super::argv::build_runtime_argv(
            &runtime,
            image,
            spec,
            &parent.spirit_binary_path,
            command,
            &spec.spirit_id,
            parent.boot_nonce,
            &parent.container_name,
        );

        // 4. Spawn via std::process::Command.
        // T3 has no pre_exec closure — the container itself is the boundary.
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..]);
        let child = cmd.spawn().map_err(SpawnError::Io)?;

        // 5. Capture host-namespace PID (this is the kernel's ADR-023 identity).
        let host_pid = inspect_container_host_pid(&runtime, &parent.container_name)
            .map_err(|e| SpawnError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))?;

        Ok(SandboxedContainerChild {
            child: Some(child),
            host_pid,
            container_name: parent.container_name,
            runtime,
        })
    }
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
        let output = std::process::Command::new(&runtime.path)
            .args(["inspect", "--format", "{{.State.Pid}}", container_name])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| T3Error::Inspect(format!("inspect container pid: {e}")))?;

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
