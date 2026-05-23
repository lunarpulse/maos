#![forbid(unsafe_code)]

//! Lifecycle module — upgrade orchestrator and related helpers.
//!
//! Story 5.4. Sibling of `scheduler/`, `supervision/`, `hot_swap/`.

pub mod upgrade;

pub use upgrade::{
    UpgradeError, UpgradeOrchestrator, UpgradeOutcome, UpgradePolicy, UpgradeReport,
};
