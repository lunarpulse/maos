#![forbid(unsafe_code)]

//! Security Manager — supervised service per §4.3.
//!
//! Enforces sandbox tiers, secret isolation, and approval-class
//! mediation. At v0.1-α this is an empty hexagonal adapter shell;
//! Story 1b.3 lands the T0/T1/T2 tier enforcement.

pub use maos_domain::ports::SecurityManagerPort;

/// Adapter shell — Story 1b.3 implements `SecurityManagerPort` for this
/// type with sandbox tier enforcement and approval mediation.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct SecurityManagerAdapter;
