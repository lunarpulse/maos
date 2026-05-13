#![forbid(unsafe_code)]

//! IAC Bus — supervised service per §4.5.
//!
//! Routes frames between Spirits and the kernel. At v0.1-α this is an
//! empty hexagonal adapter shell; Story 6.1 lands the full IAC Bus
//! with retract primitive and DRR fairness scheduler.

pub use maos_domain::ports::IacBusPort;

/// Adapter shell — Story 6.1 implements `IacBusPort` for this type
/// with frame routing, transparency logging, and fairness scheduling.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct IacBusAdapter;
