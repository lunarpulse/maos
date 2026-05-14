#![forbid(unsafe_code)]

//! Fail-closed stub for unsupported targets.

use std::process::Command;

use super::{SandboxSpec, SandboxedChild, SpawnError};

pub fn spawn_sandboxed(
    _spec: &SandboxSpec,
    _command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
    Err(SpawnError::SandboxUnavailable {
        reason: "unsupported OS — sandbox enforcement not implemented for this target".into(),
    })
}
