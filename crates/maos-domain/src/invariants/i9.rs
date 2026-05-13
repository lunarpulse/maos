//! I9: The kernel itself stores no secrets and learns no patterns.
//!
//! Caching is structural (key→value, bounded TTL, no aggregation across
//! keys, no parameter drift) and permitted within `{Journal, TransparencyLog,
//! CapabilityRegistry::tokens}` only. Learning is forbidden in any
//! kernel-core crate.
//!
//! # Enforcement
//!
//! - **v0.1**: `CI` — structural-state lint blocks new persistent fields
//!   outside the three permitted holders.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `CI` (unchanged; structural
//!   lint is the load-bearing check).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i9::{InvariantI9, KernelCaching};
//!
//! let _marker: InvariantI9 = InvariantI9;
//! // KernelCaching is a typestate marker: instances live only in the
//! // I9 whitelist holders: Journal, TransparencyLog, CapabilityRegistry::tokens.
//! let cache: KernelCaching<&str, i32> = KernelCaching::new("session-7", 42);
//! assert_eq!(cache.value(), &42);
//! ```

/// I9 marker type — The kernel stores no secrets and learns no patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI9;

/// Typed-empty newtype for sandbox tier classification.
///
/// At v0.1-α this is a placeholder; Story 1b.3 lands T0–T2
/// enforcement with per-Spirit resource caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct SandboxTier(pub u8);

/// Typestate marker for kernel-cached data — instances of this type
/// document that they live only in the I9 whitelist holders.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KernelCaching<K, V> {
    key: K,
    value: V,
}

impl<K, V> KernelCaching<K, V> {
    /// Create a new kernel-cached entry.
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    /// Return the cached value.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Return the cache key.
    pub fn key(&self) -> &K {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_caching_shape() {
        let c = KernelCaching::new("k", 100);
        assert_eq!(c.value(), &100);
        assert_eq!(c.key(), &"k");
    }
}
