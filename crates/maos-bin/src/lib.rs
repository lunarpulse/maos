#![forbid(unsafe_code)]

/// Story 15-6 — provider-level cassette replay/record adapters shared by every
/// inference consumer.
#[cfg(feature = "network")]
pub mod cassette_replay;
/// Story 14-2a — the production peer-certificate rotation trigger's wiring: the
/// real `T_grace` deadline and the operator read seam. In the library, not
/// `main.rs`, so the runtime gate leg can drive the production types.
#[cfg(feature = "network")]
pub mod cert_rotation;
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
/// Story 15-6 — the one authoritative live/record/replay selector. Public so
/// integration tests execute the production lattice; an in-`src` test module
/// would be budget-charged and CI-invisible.
pub mod inference_mode;
/// Story 16-1 (D-16-1-N) — the operator door's port implementation over the
/// daemon's real kernel objects, plus the store lock set (D-16-1-D). In the
/// library, not `main.rs`, so `tests/operator_door_16_1.rs` drives the REAL
/// port — a binary-crate `mod` would force its proofs inline, which kloc
/// charges and CI cannot run.
pub mod operator_door;
/// Story 16-4 — authoritative MAOS footprint and offline purge implementation.
pub mod purge;
pub mod shell_host;
/// Story 16-3 — Worker supervision (the Worker's SCB, its exit observer, its
/// progress stamp and its task record) and root shutdown (`unload_all_loaded`,
/// the one unload function every `maos run` root leaves through).
///
/// UNGATED, like `shell_host`, because the root-shutdown half must reach every
/// root including the no-default-features build; the Worker half inside is
/// `#[cfg(feature = "network")]` exactly like `worker_spawn`. In the library,
/// not `main.rs`, for the same reason as every module above: the corpora and
/// `tests/worker_supervision_16_3.rs` must NAME `WorkerSupervisor`,
/// `BindingPhase` and the port, and drive the real supervisor rather than a
/// re-implementation of it.
pub mod supervision;
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
