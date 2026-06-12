//! Container runtime detection — probes `/usr/bin/podman` and
//! `/usr/bin/docker`, respects `MAOS_T3_RUNTIME` env-var, caches
//! result via `OnceLock`.
//!
//! Detection order (default `auto`):
//! 1. Probe `/usr/bin/podman --version` → return `Podman`
//! 2. Probe `/usr/bin/docker --version` → return `Docker`
//! 3. Return `Err(T3Error::RuntimeUnavailable)`
//!
//! Operator override: `MAOS_T3_RUNTIME=podman|docker|auto|none`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use maos_domain::sandbox::{ContainerRuntimeKind, T3Error};

static RUNTIME_CACHE: OnceLock<Result<ContainerRuntime, T3Error>> = OnceLock::new();

/// Detected container runtime.
#[derive(Debug, Clone)]
pub struct ContainerRuntime {
    pub kind: ContainerRuntimeKind,
    pub path: PathBuf,
    pub version: String,
}

/// Detect the container runtime with caching.
/// First call performs actual detection; subsequent calls return the
/// cached result (zero-cost).
pub fn detect_container_runtime() -> Result<ContainerRuntime, T3Error> {
    RUNTIME_CACHE
        .get_or_init(|| detect_container_runtime_uncached())
        .clone()
}

/// Un-cached detection body. Probes binaries sequentially.
fn detect_container_runtime_uncached() -> Result<ContainerRuntime, T3Error> {
    let mode = std::env::var("MAOS_T3_RUNTIME").unwrap_or_else(|_| "auto".to_string());

    match mode.as_str() {
        "podman" => probe_runtime("/usr/bin/podman", ContainerRuntimeKind::Podman),
        "docker" => probe_runtime("/usr/bin/docker", ContainerRuntimeKind::Docker),
        "none" => Err(T3Error::RuntimeUnavailable),
        "auto" | _ => {
            // Probe Podman first (rootless, preferred), fall back to Docker.
            match probe_runtime("/usr/bin/podman", ContainerRuntimeKind::Podman) {
                Ok(rt) => Ok(rt),
                Err(_) => probe_runtime("/usr/bin/docker", ContainerRuntimeKind::Docker),
            }
        }
    }
}

/// Probe a single runtime binary by running `<path> --version`.
fn probe_runtime(bin: &str, kind: ContainerRuntimeKind) -> Result<ContainerRuntime, T3Error> {
    let output = Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|child| child.wait_with_output())
        .map_err(|e| T3Error::Io(format!("probe {bin}: {e}")))?;

    if !output.status.success() {
        return Err(T3Error::RuntimeUnavailable);
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(ContainerRuntime {
        kind,
        path: PathBuf::from(bin),
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_container_runtime_caches_result() {
        // First call (may fail if no runtime installed, which is fine).
        let r1 = detect_container_runtime();
        // Second call must return the same cached value.
        let r2 = detect_container_runtime();
        match (r1, r2) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a.path, b.path);
                assert_eq!(a.version, b.version);
            }
            (Err(_), Err(_)) => {
                // Both failed, cached error — expected on systems without runtime.
            }
            _ => panic!("cached and non-cached results must match"),
        }
    }
}
