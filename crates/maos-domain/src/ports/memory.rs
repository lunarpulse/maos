//! Memory Manager port trait per architecture §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. At v0.1-α this is an empty hexagonal
//! adapter shell; Story 4.3 lands the three-tier mechanics.

use crate::invariants::i5::{MemoryScope, NamespaceKey};

/// Memory Manager — namespace scope enforcement and tiered memory.
///
/// Per §4.2: "The kernel enforces three memory tiers: private,
/// shared, and collective. Spirits cannot read outside their scope."
pub trait MemoryManagerPort {
    /// Class: data-movement
    ///
    /// Validates that a read from the given namespace key is permitted
    /// under the provided memory scope. Returns `true` if the read
    /// would not cross a scope boundary.
    fn validate_namespace_read(&self, key: &NamespaceKey<MemoryScope>) -> bool;

    /// Class: data-movement
    ///
    /// Validates that a write to the given namespace key is permitted
    /// under the provided memory scope. Returns `true` if the write
    /// would not cross a scope boundary.
    fn validate_namespace_write(&self, key: &NamespaceKey<MemoryScope>) -> bool;
}
