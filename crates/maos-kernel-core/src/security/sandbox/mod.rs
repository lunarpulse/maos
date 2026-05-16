//! Sandbox enforcement — T0/T1/T2 platform dispatch.
//!
//! This is the sole deliberate `unsafe` zone in `maos-kernel-core`.
//! OS sandboxing requires `pre_exec`, Landlock, seccomp, setrlimit,
//! and FFI. Every `unsafe` block carries a `// SAFETY:` comment.
#![deny(unsafe_code)]

pub mod unsupported;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use std::process::{Child, Command, ExitStatus};

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use crate::security::manifest::ResolvedCaps;

/// Fully-resolved sandbox specification for spawning.
#[maos_attrs::i9_exempt(reason = "manifest-derived spawn parameter; created per-Spirit admission and dropped after spawn — not kernel-persistent state")]
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
    #[error("sandbox setup failed in pre_exec: {0}")]
    SandboxSetup(String),
    #[error("IO error during spawn: {0}")]
    Io(#[from] std::io::Error),
    #[error("cgroups v2 unavailable and no fallback configured")]
    CgroupUnavailable,
    #[error("sandbox unavailable on this platform: {reason}")]
    SandboxUnavailable { reason: String },
}

/// A sandbox violation detected from child exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxViolation {
    pub attempted_syscall: String,
    pub sandbox_tier: SandboxTier,
}

/// RAII guard for a sandboxed child process.
///
/// On Linux: owns the cgroup directory; `Drop` removes it after the
/// child has exited. On Windows: owns the Job Object handle.
pub struct SandboxedChild {
    child: Child,
    #[allow(dead_code)]
    cleanup: Cleanup,
}

enum Cleanup {
    #[cfg(target_os = "linux")]
    Cgroup { path: std::path::PathBuf },
    #[cfg(target_os = "windows")]
    JobObject { handle: std::os::windows::io::RawHandle },
    #[allow(dead_code)]
    None,
}

impl SandboxedChild {
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
        #[cfg(target_os = "linux")]
        if let Cleanup::Cgroup { path } = &self.cleanup {
            let _ = std::fs::remove_dir(path);
        }
    }
}

/// Spawn a command under the given sandbox spec.
///
/// Platform dispatch: Linux → Landlock+seccomp+cgroups; macOS →
/// sandbox-exec+setrlimit; Windows → restricted-token+Job Object.
pub fn spawn_sandboxed(
    spec: &SandboxSpec,
    command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
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
            if signal == libc::SIGKILL {
                return Some(SandboxViolation {
                    attempted_syscall: "possible-oom-or-resource-cap".into(),
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
