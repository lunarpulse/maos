#![forbid(unsafe_code)]

//! `spirit_test` — the spirit-test SDK seed (Story 2.4 v0.3 prerequisite).
//!
//! Wraps `LocalRunner` (Story 2.3) with: IAC frame I/O capture, halt
//! resolution simulator (3 kinds — forward-anchor for Story 4.1), manifest
//! self-check primitive, class-specific regression corpus skeleton,
//! assertion macros, and cross-Spirit isolation framework hooks.
//!
//! Per Epic 2 line 14: "spirit-test SDK seed: local runner without kernel +
//! manifest self-check + class-specific regression corpus skeleton." Full
//! per-language SDK with assertion macros + halt resolution + manifest
//! self-check + class-specific regression corpus is Story 7.1 at v0.5+.

pub mod harness;
pub mod assert;
pub mod halt;
pub mod manifest;
pub mod regression;
pub mod isolation;

pub use harness::{SpiritTest, ExtendedRunReport};
pub use halt::{HaltResolutionKind, HaltResolutionRecord};
pub use manifest::{ManifestSelfCheckReport, ManifestSelfCheckViolation, manifest_self_check};
pub use regression::{RegressionCorpus, RegressionCase, SpiritClass};
pub use isolation::{
    CrossSpiritIsolationFixture, IsolationAttackCategory, IsolationAttackCase,
    IsolationHookPoint, IsolationHookOutcome, IsolationOutcome, DefaultIsolationHook,
    HookCallRecord, AttemptResult, ObservationResult,
};
