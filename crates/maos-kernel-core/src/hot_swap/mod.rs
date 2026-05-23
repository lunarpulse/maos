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

pub mod archive;
pub mod coordinator;
pub mod migrator;
pub mod post_swap_monitor;
pub mod precheck;
pub mod saga;
pub mod state_codec;

pub use archive::{ArchiveError, SpiritArchive};
pub use coordinator::HotSwapCoordinator;
pub use migrator::{matches_version_pattern, run_migrator};
pub use post_swap_monitor::{
    PostSwapInvariantSnapshot, PostSwapInvariantViolation, PostSwapMonitor,
};
pub use precheck::{HotSwapPrecheck, PrecheckVerdict};
pub use precheck::{PrecheckOutcome, SchemaCompat};
pub use saga::{HotSwapSaga, SagaCompensation, SagaPhase};
pub use state_codec::{StateCodec, StateCodecError, StateEnvelope};
