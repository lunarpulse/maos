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
//!
//! ## Story 2.1 additive surface (does NOT bump `ABI_VERSION`)
//!
//! Story 2.1 adds:
//! - `pub mod cancellation` — `CancellationSignal` trait + `NeverCancel`
//! - `pub mod lifecycle` — `Spirit` trait (11 hooks) + `SpiritVtable<T>` + payload types
//! - `pub mod ctx` — `Ctx` Spirit-author-facing context type
//!
//! All additions are ABI-additive per §8.5 rows 7+8. `ABI_VERSION` remains `1`.

extern crate alloc;

pub mod compliance;
pub mod cancellation;
pub mod lifecycle;
pub mod ctx;

/// ABI version constant for the MAOS Spirit ABI.
/// Bumped according to the ABI Stability Triple rules (§8.5).
///
/// **Story 1b.4 froze this at `1`** at the ComplianceClaim envelope freeze.
pub const ABI_VERSION: u32 = 1;
