#![forbid(unsafe_code)]

//! Hot-Swap Coordinator — kernel-side supervisor for Spirit hot-swap.
//!
//! Per architecture §4.0.2 line 47, `hot_swap/` lives as a sub-module of
//! `maos-kernel-core` (part of the Spirit Scheduler supervisor). At v0.3-β
//! the coordinator is a module; v0.5+ extraction to `crates/services/hot-swap/`
//! is the promotion path.
//!
//! ## Sub-modules
//!
//! - `coordinator` — `HotSwapCoordinator` entry point, 12-step protocol.
//! - `state_codec` — CBOR envelope encode/decode + same-major vs cross-major detection.
//! - `saga` — Compensating transactions for swap failure boundaries.
//! - `post_swap_monitor` — 30s window invariant checker + auto-revert.
//! - `migrator` — Cross-major migration path + `matches_version_pattern`.
//! - `archive` — Predecessor binary archive persistence.
//! - `precheck` — ADR-036 pure-function precheck reporter.

pub mod coordinator;
pub mod state_codec;
pub mod saga;
pub mod post_swap_monitor;
pub mod migrator;
pub mod archive;
pub mod precheck;

pub use coordinator::HotSwapCoordinator;
pub use state_codec::{StateCodec, StateEnvelope, StateCodecError};
pub use saga::{HotSwapSaga, SagaPhase, SagaCompensation};
pub use post_swap_monitor::{PostSwapMonitor, PostSwapInvariantSnapshot, PostSwapInvariantViolation};
pub use migrator::{run_migrator, matches_version_pattern};
pub use archive::{SpiritArchive, ArchiveError};
pub use precheck::{HotSwapPrecheck, PrecheckVerdict};
pub use precheck::{PrecheckOutcome, SchemaCompat};
