#![forbid(unsafe_code)]

//! Resource ceiling enforcement — OS-level CPU/memory limits.
//!
//! Architecture §4.1 + Story 5.1 Task 10.
//!
//! At v0.3-β the rust-inproc form does NOT call this function — there's
//! no separate process to constrain. The API surface exists for Story 5.5x's
//! subprocess form to call at spawn. The Linux path writes cgroups v2
//! files; macOS delegates to setrlimit; Windows returns Unimplemented.

use std::path::PathBuf;

use maos_domain::invariants::i9::SandboxTier;

use crate::security::manifest::ResourceCaps;

/// RAII guard that removes cgroup directories on drop (Linux only).
pub struct ResourceCeilingHandle {
    /// Path to the cgroup directory (None on non-Linux).
    cgroup_dir: Option<PathBuf>,
    /// Whether the ceiling was applied successfully.
    applied: bool,
}

impl ResourceCeilingHandle {
    fn noop() -> Self {
        Self {
            cgroup_dir: None,
            applied: false,
        }
    }

    fn linux(cgroup_dir: PathBuf) -> Self {
        Self {
            cgroup_dir: Some(cgroup_dir),
            applied: true,
        }
    }

    pub fn was_applied(&self) -> bool {
        self.applied
    }
}

impl Drop for ResourceCeilingHandle {
    fn drop(&mut self) {
        if let Some(ref dir) = self.cgroup_dir {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// Error returned by resource ceiling operations.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("unimplemented on this platform: {0}")]
    Unimplemented(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Apply OS-level CPU and memory ceilings for a Spirit process.
///
/// On Linux: creates `/sys/fs/cgroup/maos/spirit-<pid>/` and writes
/// `cpu.max` + `memory.max` per the manifest's `[resources]` section.
/// Falls back to a no-op handle if the cgroup path is unwritable.
///
/// On macOS: delegates to the existing `setrlimit` path (already in
/// `security/sandbox/macos.rs`).
///
/// On Windows: returns `Err(IoError::Unimplemented(...))`.
///
/// This function is NOT called at v0.3-β for rust-inproc Spirits;
/// the API surface is forward-shaped for Story 5.5x's subprocess form.
pub fn apply_resource_ceiling(
    spirit_pid: u32,
    caps: &ResourceCaps,
) -> Result<ResourceCeilingHandle, IoError> {
    #[cfg(target_os = "linux")]
    {
        apply_cgroup_v2(spirit_pid, caps)
    }
    #[cfg(target_os = "macos")]
    {
        apply_setrlimit_macos(spirit_pid, caps)
    }
    #[cfg(windows)]
    {
        Err(IoError::Unimplemented(
            "Job Objects scheduled for Story 5.5x — subprocess form".into(),
        ))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        Err(IoError::Unimplemented(format!(
            "unsupported platform for resource ceiling"
        )))
    }
}

#[cfg(target_os = "linux")]
fn apply_cgroup_v2(
    spirit_pid: u32,
    caps: &ResourceCaps,
) -> Result<ResourceCeilingHandle, IoError> {
    let cgroup_dir = PathBuf::from(format!("/sys/fs/cgroup/maos/spirit-{}", spirit_pid));

    // Create the cgroup directory
    if let Err(e) = std::fs::create_dir_all(&cgroup_dir) {
        // Best-effort: cgroup unavailable — return no-op handle.
        // The v0.3-β substrate works on systems without cgroups v2.
        if e.kind() == std::io::ErrorKind::PermissionDenied
            || e.kind() == std::io::ErrorKind::NotFound
        {
            eprintln!(
                "maos: cgroup_unavailable — cannot create {} ({}); \
                 falling through to setrlimit. Ensure the MAOS service \
                 has write access to /sys/fs/cgroup/maos/.",
                cgroup_dir.display(),
                e
            );
            return Ok(ResourceCeilingHandle::noop());
        }
        return Err(IoError::Io(e));
    }

    // Write cpu.max: "<quota_us> <period_us>" (period = 100000us = 100ms)
    if let Some(cpu_pct) = caps.cpu_max_pct {
        let quota_us = compute_cpu_quota_us(cpu_pct);
        let cpu_max_content = format!("{} 100000", quota_us);
        std::fs::write(cgroup_dir.join("cpu.max"), cpu_max_content)?;
    }

    // Write memory.max: bytes
    if let Some(mem_mb) = caps.memory_max_mb {
        let mem_bytes = (mem_mb as u64) * 1024 * 1024;
        std::fs::write(
            cgroup_dir.join("memory.max"),
            mem_bytes.to_string(),
        )?;
    }

    Ok(ResourceCeilingHandle::linux(cgroup_dir))
}

#[cfg(target_os = "linux")]
fn compute_cpu_quota_us(cpu_max_pct: u32) -> u32 {
    // period is 100000us (100ms), so quota = period * pct / 100
    (100_000u64 * cpu_max_pct as u64 / 100) as u32
}

#[cfg(target_os = "macos")]
fn apply_setrlimit_macos(
    _spirit_pid: u32,
    _caps: &ResourceCaps,
) -> Result<ResourceCeilingHandle, IoError> {
    // The setrlimit path is already applied at sandbox admission
    // (security/sandbox/macos.rs). This function is a forward-shaped
    // seam for Story 5.5x.
    Ok(ResourceCeilingHandle::noop())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn compute_cpu_quota_us_values() {
        assert_eq!(compute_cpu_quota_us(10), 10000);
        assert_eq!(compute_cpu_quota_us(50), 50000);
        assert_eq!(compute_cpu_quota_us(100), 100000);
        assert_eq!(compute_cpu_quota_us(0), 0);
    }
}
