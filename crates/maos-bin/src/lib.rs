#![forbid(unsafe_code)]

#[cfg(feature = "network")]
pub mod cross_team_consent;
#[cfg(feature = "network")]
pub mod cross_team_crossing;
#[cfg(feature = "network")]
pub mod cross_wall_log_read;
/// j1-crosshost-1a — frame-borne `developer-remote` delegation (loopback A2A).
/// `pub` so integration tests can drive the real production leg rather than a
/// re-implementation of it.
#[cfg(feature = "network")]
pub mod delegation;
#[cfg(feature = "network")]
pub mod enterprise_identity;
#[cfg(feature = "network")]
pub mod tenant_map;
/// The Worker-CLI **adapter** seam (J1 Tier-2 bridge). In the library, not
/// `main.rs`, so `crates/maos-bin/tests/` can execute it — an in-`src` test
/// module is budget-charged and CI-invisible. j1-crosshost-2a AC1.1 relocated
/// this module and its tests for exactly that reason: the completion oracle's
/// proven-red vector needs to name `ClaudeCli`/`CodexCli`/`parse_completion`
/// from an integration test.
#[cfg(feature = "network")]
pub mod worker_cli;

/// `[topology]` manifest parsing. In the library, not `main.rs`, so
/// `crates/maos-bin/tests/` can execute it — an in-`src` test module is
/// budget-charged and CI-invisible.
pub mod topology;
