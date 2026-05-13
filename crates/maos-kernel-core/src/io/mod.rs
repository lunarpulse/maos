#![forbid(unsafe_code)]

//! I/O Subsystem — internal module at v0.1 per §4.4.
//!
//! Provides HTTP and filesystem I/O adapters for Spirits. At v0.1-α
//! this is an empty hexagonal adapter shell; Story 1b.4 lands the
//! full I/O mediation with per-Spirit bandwidth quotas.

pub use maos_domain::ports::IoSubsystemPort;

/// Adapter shell — Story 1b.4 implements `IoSubsystemPort` for this
/// type with HTTP client and filesystem mediation.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct IoSubsystemAdapter;
