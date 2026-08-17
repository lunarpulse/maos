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
/// Story 11.4a — the enterprise PDP (Cedar) reconciler. Under the library, not
/// `main.rs`, because [`worker_spawn::issue_enterprise_governed_capability`]
/// takes `&EnterprisePdpRuntime` and moved (j1-crosshost-2b AC1.1). The FILE
/// stays at `src/enterprise_pdp_runtime.rs` — `cohort_daemon_smoke_13_5c.rs:886`
/// reads it by `include_str!`.
#[cfg(feature = "network")]
pub mod enterprise_pdp_runtime;
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
/// The `maos run` **worker-spawn surface** — `[cli_wrapper]` admission, the
/// host-managed grant allowlist, the enterprise-governed capability mint and the
/// real subprocess bridge. Relocated out of `main.rs` by j1-crosshost-2b AC1.1
/// on the same doctrine as `worker_cli` above: a private item of the BINARY crate
/// cannot be named from `crates/maos-bin/tests/`, so typed `WorkerCompletion`
/// assertions and port injection were impossible — subprocess-only coverage
/// (`worker_completion_2a.rs:871`) was the ceiling, and the host-B drain needs to
/// call this surface directly. The `cohort-a2a-daemon` region did NOT move
/// (Trap 9: two suites assert its literal text via `include_str!`).
#[cfg(feature = "network")]
pub mod worker_spawn;

/// `[topology]` manifest parsing. In the library, not `main.rs`, so
/// `crates/maos-bin/tests/` can execute it — an in-`src` test module is
/// budget-charged and CI-invisible.
pub mod topology;
