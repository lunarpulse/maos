//! Test helpers shared across kernel-core integration tests.

use maos_kernel_core::security::sandbox::t3::runtime_detect;

/// Returns `true` if no container runtime (Podman/Docker) is available,
/// indicating the test should be skipped.
pub fn skip_if_no_container_runtime() -> bool {
    runtime_detect::detect_container_runtime().is_err()
}
