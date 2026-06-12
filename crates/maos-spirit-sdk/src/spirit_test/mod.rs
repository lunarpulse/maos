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

pub mod assert;
pub mod halt;
pub mod harness;
pub mod isolation;
pub mod manifest;
pub mod regression;

pub use halt::{HaltResolutionKind, HaltResolutionRecord};
pub use harness::{ExtendedRunReport, SpiritTest};
pub use isolation::{
    AttemptResult, CrossSpiritIsolationFixture, DefaultIsolationHook, HookCallRecord,
    IsolationAttackCase, IsolationAttackCategory, IsolationHookOutcome, IsolationHookPoint,
    IsolationOutcome, ObservationResult,
};
pub use manifest::{manifest_self_check, ManifestSelfCheckReport, ManifestSelfCheckViolation};
pub use regression::{RegressionCase, RegressionCorpus, SpiritClass};

// Story 7.1 v0.5 binding — convenience aliases so authors can write
// `spirit_test::assert!`, `spirit_test::expect_frame!`, `spirit_test::expect_halt!`.
pub use crate::assert_no_deprecations;
pub use crate::spirit_test_assert as assert;
pub use crate::spirit_test_expect_frame as expect_frame;
pub use crate::spirit_test_expect_halt as expect_halt;
