//! Sandbox enforcement — T0/T1/T2 platform dispatch.
//!
//! This is the sole deliberate `unsafe` zone in `maos-kernel-core`.
//! OS sandboxing requires `pre_exec`, Landlock, seccomp, setrlimit,
//! and FFI. Every `unsafe` block carries a `// SAFETY:` comment.
#![deny(unsafe_code)]

pub mod t3;
pub mod unsupported;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
use std::process::Child;
use std::process::{Command, ExitStatus};

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use crate::security::manifest::ResolvedCaps;

/// Fully-resolved sandbox specification for spawning.
#[maos_attrs::i9_exempt(
    reason = "manifest-derived spawn parameter; created per-Spirit admission and dropped after spawn — not kernel-persistent state"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    pub tier: SandboxTier,
    pub resolved_caps: ResolvedCaps,
    pub declared_scopes: Vec<Scope>,
    pub spirit_id: String,
    /// Output shape predicate scaffolding for Story 7.3 fail-loud enforcement.
    /// Defaults to `None`; populated when admission receives a parsed manifest
    /// with the `[output_shape]` section.
    pub output_shape_predicate: Option<crate::security::manifest::OutputShapePredicate>,
}

impl SandboxSpec {
    /// Create a spec for testing / probe processes.
    pub fn new_for_test(tier: SandboxTier) -> Self {
        Self {
            tier,
            resolved_caps: ResolvedCaps::default(),
            declared_scopes: vec![],
            spirit_id: String::new(),
            output_shape_predicate: None,
        }
    }
}

/// Error raised when spawning a sandboxed child fails.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("sandbox setup failed: {0}")]
    SandboxSetup(String),
    #[error("IO error during spawn: {0}")]
    Io(#[from] std::io::Error),
    #[error("declared CPU or PID cap requires a writable delegated cgroup")]
    CgroupUnavailable,
    #[error("sandbox unavailable on this platform: {reason}")]
    SandboxUnavailable { reason: String },
    #[error("T3 image SHA mismatch: expected {expected}, observed {observed}")]
    SandboxImageMismatch { expected: String, observed: String },
    #[error("T3 container runtime unavailable: {reason}")]
    T3RuntimeUnavailable { reason: String },
    #[error("T3 registry manifest inspection failed: {reason}")]
    T3ImageInspect { reason: String },
    #[error("T3 sandbox report commit failed: {reason}")]
    T3ReportCommit { reason: String },
}

/// Largest component the governed WASM route accepts (matches the runner's own cap).
const MAX_HELD_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;

/// Why a handle cannot be carried into the governed WASM runner.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HeldComponentError {
    /// This platform has no governed WASM artifact handoff; nothing was launched.
    #[error("governed WASM held-artifact transport is unsupported on this platform")]
    TransportUnsupported,
    #[error("held component is not a regular file")]
    NotRegular,
    #[error("held component handle is not a read-only descriptor")]
    NotReadOnly,
    #[error("held component is {len} bytes, exceeds the {MAX_HELD_COMPONENT_BYTES}-byte cap")]
    TooLarge { len: u64 },
    #[error("held component handle inspection failed: {0}")]
    Inspect(String),
}

/// One-shot, non-cloneable custody of the read-only regular component file an admission
/// step already verified. The only consumer is the dedicated WASM/T2 spawn: the intended
/// child receives this exact open file, never a path. The parent's copy stays
/// close-on-exec and is closed when this value drops (after the spawn resolves).
#[derive(Debug)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub struct HeldWasmComponent {
    file: std::fs::File,
    /// The descriptor number this open file already holds. The intended child keeps this
    /// exact number (close-on-exec cleared in that child only); no number is reassigned.
    fd: i32,
    device: u64,
    inode: u64,
    sha256: [u8; 32],
}

impl HeldWasmComponent {
    /// Take custody of `file`. `sha256` is the digest of the exact compilation bytes; the
    /// runner refuses a launch whose bytes differ, so it is required for every held launch.
    /// The handle must be a read-only regular file of at most 64 MiB.
    pub fn new(file: std::fs::File, sha256: [u8; 32]) -> Result<Self, HeldComponentError> {
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            linux::prepare_held_component(&file)?;
            let metadata = file
                .metadata()
                .map_err(|e| HeldComponentError::Inspect(e.to_string()))?;
            if !metadata.is_file() {
                return Err(HeldComponentError::NotRegular);
            }
            if metadata.len() > MAX_HELD_COMPONENT_BYTES {
                return Err(HeldComponentError::TooLarge {
                    len: metadata.len(),
                });
            }
            Ok(Self {
                fd: std::os::fd::AsRawFd::as_raw_fd(&file),
                file,
                device: metadata.dev(),
                inode: metadata.ino(),
                sha256,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (file, sha256);
            Err(HeldComponentError::TransportUnsupported)
        }
    }

    /// The private runner handoff flags this value contributes to the child's argv.
    pub(crate) fn runner_args(&self) -> Vec<String> {
        vec![
            "--held-component-fd".to_owned(),
            self.fd.to_string(),
            "--held-component-device".to_owned(),
            self.device.to_string(),
            "--held-component-inode".to_owned(),
            self.inode.to_string(),
            "--held-component-sha256".to_owned(),
            hex::encode(self.sha256),
        ]
    }
}

/// Spawn the runner under Linux T2 with `held` kept open (close-on-exec cleared) in the
/// intended child only, on the descriptor number it already holds. Any other tier fails closed.
#[cfg(target_os = "linux")]
pub(crate) fn spawn_sandboxed_with_held_component(
    spec: &SandboxSpec,
    command: &mut Command,
    held: &HeldWasmComponent,
) -> Result<SandboxedChild, SpawnError> {
    if spec.tier != SandboxTier::T2 {
        return Err(SpawnError::SandboxSetup(format!(
            "held component handoff requires tier T2, got {:?}",
            spec.tier
        )));
    }
    linux::spawn_sandboxed_with_held_component(spec, command, held)
}

/// A sandbox violation detected from child exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxViolation {
    pub attempted_syscall: String,
    pub sandbox_tier: SandboxTier,
}

/// RAII guard for a sandboxed child process.
///
/// On Linux: owns the cgroup directory; `Drop` kills remaining members and
/// attempts bounded removal after reaping the child, logging any residue.
/// On Windows: closes the Job Object last so kill-on-close remains effective.
pub struct SandboxedChild {
    #[cfg(not(target_os = "windows"))]
    child: Child,
    #[cfg(target_os = "windows")]
    child: windows::WindowsChild,
    #[allow(dead_code)]
    cleanup: Cleanup,
    #[cfg(target_os = "linux")]
    resource_downgrade: Option<String>,
}

enum Cleanup {
    #[cfg(target_os = "linux")]
    Cgroup { path: std::path::PathBuf },
    #[allow(dead_code)]
    None,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Cleanup::Cgroup { path } = self {
            if let Err(error) = std::fs::write(path.join("cgroup.kill"), b"1") {
                eprintln!(
                    "maos-sandbox: failed to kill cgroup {}: {error}",
                    path.display()
                );
            }
            // `cgroup.kill` is asynchronous: rmdir answers EBUSY until the killed
            // members are gone. Retry with a short bounded backoff (~0.25 s total).
            let mut removal = Err(std::io::Error::from_raw_os_error(libc::EBUSY));
            for delay_ms in [0, 1, 2, 4, 8, 16, 32, 64, 128] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                removal = std::fs::remove_dir(path.as_path());
                if !matches!(&removal, Err(error) if error.raw_os_error() == Some(libc::EBUSY)) {
                    break;
                }
            }
            if let Err(error) = removal {
                eprintln!(
                    "maos-sandbox: cgroup cleanup failed; residue may remain at {}: {error}",
                    path.display()
                );
            }
        }
    }
}

impl SandboxedChild {
    /// The child's cgroup directory, when it was placed in one (Linux).
    #[cfg(target_os = "linux")]
    pub fn cgroup_path(&self) -> Option<&std::path::Path> {
        match &self.cleanup {
            Cleanup::Cgroup { path } => Some(path),
            Cleanup::None => None,
        }
    }
    /// Consume the per-spawn memory-cgroup fallback reason (Linux only).
    #[cfg(target_os = "linux")]
    pub fn take_resource_downgrade(&mut self) -> Option<String> {
        self.resource_downgrade.take()
    }

    /// Story 17-6 AC5 — the confined child, for a caller that pipes its stdio.
    /// The guard keeps owning it (and the cgroup dir) until drop.
    #[cfg(target_os = "linux")]
    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    pub fn wait(&mut self) -> Result<ExitStatus, std::io::Error> {
        self.child.wait()
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, std::io::Error> {
        self.child.try_wait()
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for SandboxedChild {
    fn drop(&mut self) {
        // Kill first to ensure the child exits before we clean up resources.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Spawn a command under the given sandbox spec.
///
/// Platform dispatch: T3 → container isolation; otherwise
/// Linux → Landlock+seccomp+cgroups; macOS →
/// sandbox-exec+setrlimit; Windows → restricted-token+Job Object.
pub fn spawn_sandboxed(
    spec: &SandboxSpec,
    command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
    if spec.tier == SandboxTier::T3 {
        // T3 container isolation: forwarded to t3::spawn::spawn_t3.
        // The Command is ignored; spawn_t3 builds its own argv.
        // v0.5-α: T3 spawn requires the full SandboxSpec + image
        // attestation + T3SpawnContext, so this arm returns
        // SandboxUnavailable to guide callers to use spawn_t3 directly.
        return Err(SpawnError::SandboxUnavailable {
            reason: "T3 container isolation cannot be spawned via generic Command; use t3::spawn::spawn_t3 directly".into(),
        });
    }
    #[cfg(target_os = "linux")]
    {
        linux::spawn_sandboxed(spec, command)
    }
    #[cfg(target_os = "macos")]
    {
        macos::spawn_sandboxed(spec, command)
    }
    #[cfg(target_os = "windows")]
    {
        windows::spawn_sandboxed(spec, command)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        unsupported::spawn_sandboxed(spec, command)
    }
}

/// Classify a child exit status into an optional sandbox violation.
pub fn classify_exit(status: ExitStatus) -> Option<SandboxViolation> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            if signal == libc::SIGSYS {
                return Some(SandboxViolation {
                    attempted_syscall: "unknown".into(),
                    sandbox_tier: SandboxTier::T2,
                });
            }
        }
    }
    #[cfg(windows)]
    {
        // On Windows, Job Object termination uses exit code 1.
        // STATUS_ACCESS_VIOLATION (0xC0000005) may indicate sandbox violation.
        if let Some(code) = status.code() {
            if code == 0xC0000005u32 as i32 {
                return Some(SandboxViolation {
                    attempted_syscall: "access-violation-possible-sandbox".into(),
                    sandbox_tier: SandboxTier::T2,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_spec_construction() {
        let spec = SandboxSpec::new_for_test(SandboxTier::T2);
        assert_eq!(spec.tier, SandboxTier::T2);
    }

    #[test]
    fn spawn_error_display() {
        let e = SpawnError::SandboxSetup("landlock failed".into());
        assert!(e.to_string().contains("landlock failed"));
    }
}
