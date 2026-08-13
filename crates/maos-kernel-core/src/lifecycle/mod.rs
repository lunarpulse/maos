#![forbid(unsafe_code)]

//! Lifecycle module — upgrade orchestrator and related helpers.
//!
//! Story 5.4. Sibling of `scheduler/`, `supervision/`, `hot_swap/`.
//! Story 6.2 — `cli_wrapper` subdirectory hosts the CliWrapperSpirit class
//! per architecture §6.7 + ADR-021. Dev judgment placed it under
//! `lifecycle/` rather than a new top-level `spirit/` directory: the
//! cli_wrapper module hooks into Spirit lifecycle (admission probe,
//! runtime stdio bridge, on_unload signal dispatch) which is the natural
//! home for this surface.

pub mod cli_wrapper; // Story 6.2 AC5 — CliWrapperSpirit class
pub mod upgrade;

pub use upgrade::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOrchestrator, UpgradeOutcome, UpgradePolicy,
    UpgradeReport,
};
