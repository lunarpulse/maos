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

/// `[topology]` manifest parsing. In the library, not `main.rs`, so
/// `crates/maos-bin/tests/` can execute it — an in-`src` test module is
/// budget-charged and CI-invisible.
pub mod topology;
