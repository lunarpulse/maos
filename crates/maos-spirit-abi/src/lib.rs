#![no_std]
#![forbid(unsafe_code)]

//! `maos-spirit-abi` — wire-stable types ONLY (`#![no_std]`).
//!
//! Bumping `ABI_VERSION` here is the **ABI-bump trigger** per §8.5.
//! At v0.1-α this constant stays `0`; Story 1b.4 freezes the
//! ComplianceClaim envelope shape and bumps to `1`.
//!
//! See `compliance.rs` for the binding-v0.1 ComplianceClaim schema
//! types committed under the joint Mary+Winston review (see
//! `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`).

extern crate alloc;

pub mod compliance;

/// ABI version constant for the MAOS Spirit ABI.
/// Bumped according to the ABI Stability Triple rules (§8.5).
///
/// **Story 1a.1 freezes this at `0`**; Story 1b.4 bumps to `1`
/// at the ComplianceClaim envelope freeze. Do NOT bump in this story
/// — bumping here breaks the ABI baseline diff in unintended ways.
pub const ABI_VERSION: u32 = 0;
