#![forbid(unsafe_code)]

//! Memory Manager — supervised service per §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. At v0.1-α this is an empty hexagonal
//! adapter shell; Story 4.3 lands the three-tier mechanics.

pub use maos_domain::ports::MemoryManagerPort;

/// Adapter shell — Story 4.3 implements `MemoryManagerPort` for this
/// type with three-tier memory and namespace scope enforcement.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryManagerAdapter;
