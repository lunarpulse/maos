#![no_std]
#![forbid(unsafe_code)]

//! `maos-spirit-abi` — wire-stable types ONLY (`#![no_std]`).
//!
//! `ABI_VERSION` was bumped to `1` in Story 1b.4 when the ComplianceClaim
//! schema was frozen under the joint Mary+Winston adversarial review (see
//! `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`).
//!
//! Bumping `ABI_VERSION` here is the **ABI-bump trigger** per §8.5.
//! The abi-diff gate (`--deny removed --deny changed`) is baselined at
//! `abi-baseline/v1-pre-bump.txt` after the 1b.4 freeze.

extern crate alloc;

pub mod compliance;

/// ABI version constant for the MAOS Spirit ABI.
/// Bumped according to the ABI Stability Triple rules (§8.5).
///
/// **Story 1b.4 froze this at `1`** at the ComplianceClaim envelope freeze.
pub const ABI_VERSION: u32 = 1;
