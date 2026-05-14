//! Windows T2 sandbox enforcement: restricted-token + Job Object.
//!
//! At v0.1-β this is a fail-closed stub. Full `CreateRestrictedToken`
//! + `CreateProcessAsUser` + `win32job` integration ships when a
//! Windows CI runner is available. Until then, sandbox enforcement is
//! unavailable on Windows — callers receive `SpawnError::SandboxUnavailable`.
#![forbid(unsafe_code)]

use std::process::Command;

use super::{SandboxSpec, SpawnError};

pub fn spawn_sandboxed(
    _spec: &SandboxSpec,
    _command: &mut Command,
) -> Result<super::SandboxedChild, SpawnError> {
    Err(SpawnError::SandboxUnavailable {
        reason: "Windows sandbox enforcement not yet implemented; \
                 CreateRestrictedToken + win32job pending Windows CI runner"
            .into(),
    })
}
