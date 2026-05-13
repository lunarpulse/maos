#![forbid(unsafe_code)]

//! Capability Registry — supervised service per §4.6.
//!
//! Decomposed per ADR-030 into four sub-modules (hot path / policy /
//! audit / quota). At v0.1-α the four sub-module shells exist from
//! Story 1a.1; this story adds the port-trait re-export and the
//! `CapabilityRegistryAdapter` placeholder.

pub mod cap_tokens;
pub mod cap_policy;
pub mod cap_audit;
pub mod cap_quota;

pub use maos_domain::ports::CapabilityRegistryPort;

/// Adapter shell — Story 1b.2 implements `CapabilityRegistryPort` for
/// this type with token mediation, policy evaluation, audit writes,
/// and quota enforcement.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilityRegistryAdapter;
