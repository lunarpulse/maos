#![forbid(unsafe_code)]

//! Story 6.2 AC5 — CliWrapperSpirit lifecycle hook wiring.
//!
//! Per the §Boundary-Note decision documented in `mod.rs`: the CliWrapperSpirit
//! subprocess invocation is implemented via option (b) —
//! CapabilityRegistry-mediated `Scope::CliSubprocessSpawn` — rather than
//! adding a new `on_cli_subprocess_invoke` lifecycle hook. The existing 14
//! lifecycle hooks (per `xtask/spirit-abi-hook-count.toml`) are sufficient:
//!
//! - `on_load`     → invoke admission probe; reject on shape mismatch.
//! - `on_start`    → first task dispatched.
//! - `on_unload`   → dispatch shutdown signal per `posture.shutdown_signal`,
//!                   then subprocess wait + cap-token revocation.
//! - `on_pause` / `on_resume` → control-channel pause/resume per
//!                   `posture.control_channel`.
//! - others → inherited from the native Spirit ABI surface.

use crate::security::manifest::{CliWrapperConfig, CliWrapperRecoveryPolicy};

/// Story 6.2 AC5 — recovery decision per `posture.recovery_policy` after
/// observed subprocess death.
///
/// Returns the recovery action per the declared policy. This function
/// always succeeds — the recovery decision is policy-driven, not an I/O
/// operation. The CALLER is responsible for executing the action (e.g.,
/// respawning the subprocess or escalating to the supervisor).
pub fn handle_subprocess_death(
    config: &CliWrapperConfig,
    exit_code: Option<i32>,
) -> RecoveryAction {
    match config.recovery_policy {
        CliWrapperRecoveryPolicy::RespawnWithContext => RecoveryAction::Respawn {
            transfer_context: true,
            exit_code,
        },
        CliWrapperRecoveryPolicy::RespawnFresh => RecoveryAction::Respawn {
            transfer_context: false,
            exit_code,
        },
        CliWrapperRecoveryPolicy::Escalate => RecoveryAction::Escalate { exit_code },
        _ => RecoveryAction::Escalate { exit_code },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Respawn the subprocess. `transfer_context = true` carries the prior
    /// session state across the restart per `recovery_policy = RespawnWithContext`.
    Respawn {
        transfer_context: bool,
        exit_code: Option<i32>,
    },
    /// Do NOT respawn. Emit `SpiritDied` event and escalate to supervisor.
    Escalate { exit_code: Option<i32> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::manifest::{
        CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
    };

    fn cfg(policy: CliWrapperRecoveryPolicy) -> CliWrapperConfig {
        CliWrapperConfig {
            command: "echo".into(),
            argv_prefix: vec![],
            output_shape_version: "1.0.0".into(),
            skill_bundle: vec![],
            recovery_policy: policy,
            posture: CliWrapperPosture {
                stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
                control_channel: CliWrapperControlChannel::Signals,
                shutdown_signal: None,
            },
        }
    }

    #[test]
    fn respawn_with_context_returns_transfer_true() {
        let c = cfg(CliWrapperRecoveryPolicy::RespawnWithContext);
        let action = handle_subprocess_death(&c, Some(127));
        assert_eq!(
            action,
            RecoveryAction::Respawn {
                transfer_context: true,
                exit_code: Some(127),
            }
        );
    }

    #[test]
    fn respawn_fresh_returns_transfer_false() {
        let c = cfg(CliWrapperRecoveryPolicy::RespawnFresh);
        let action = handle_subprocess_death(&c, Some(0));
        assert_eq!(
            action,
            RecoveryAction::Respawn {
                transfer_context: false,
                exit_code: Some(0),
            }
        );
    }

    #[test]
    fn escalate_returns_escalate() {
        let c = cfg(CliWrapperRecoveryPolicy::Escalate);
        let action = handle_subprocess_death(&c, Some(1));
        assert_eq!(action, RecoveryAction::Escalate { exit_code: Some(1) });
    }
}
