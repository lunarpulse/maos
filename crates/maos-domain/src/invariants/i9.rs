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

/// Error returned when a `SandboxTier` value is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid sandbox tier value: {0} (valid range: 0..=4)")]
pub struct SandboxTierError(pub u8);

/// Typed-empty newtype for sandbox tier classification.
///
/// Story 1b.3 hardened: associated constants, validating constructors,
/// fail-closed Default (T2, the most restrictive enforceable tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SandboxTier(pub u8);

impl SandboxTier {
    /// T0 — trusted local; least restrictive.
    pub const T0: Self = SandboxTier(0);
    /// T1 — process isolation + UID separation.
    pub const T1: Self = SandboxTier(1);
    /// T2 — Landlock+seccomp (Linux), Seatbelt (macOS), restricted-token (Windows).
    pub const T2: Self = SandboxTier(2);
    /// T3 — container isolation (v0.5); representable but rejected at v0.1-β enforcement.
    pub const T3: Self = SandboxTier(3);
    /// T4 — WASM-component sandbox (v2.0); reserved.
    pub const T4: Self = SandboxTier(4);

    /// The default floor for fail-closed fallbacks: T2 (most restrictive
    /// tier the kernel can enforce at v0.1-β). Resolves DF18.
    pub const DEFAULT_FLOOR: Self = Self::T2;

    /// Maximum representable tier value (inclusive).
    pub const MAX_VALID: u8 = 4;

    /// Validate a raw u8 into a `SandboxTier`.
    /// Rejects values outside `0..=4`.
    pub const fn try_from_u8(v: u8) -> Result<Self, SandboxTierError> {
        if v > Self::MAX_VALID {
            Err(SandboxTierError(v))
        } else {
            Ok(SandboxTier(v))
        }
    }

    /// Parse a manifest string into a `SandboxTier`.
    /// Accepts exactly `"T0"`..="T3"` (case-sensitive).
    pub fn try_from_manifest_str(s: &str) -> Result<Self, SandboxTierError> {
        match s {
            "T0" => Ok(Self::T0),
            "T1" => Ok(Self::T1),
            "T2" => Ok(Self::T2),
            "T3" => Ok(Self::T3),
            _ => Err(SandboxTierError(u8::MAX)),
        }
    }
}

impl Default for SandboxTier {
    /// DF18: security-sensitive type defaults to the most restrictive
    /// enforceable tier (T2), not the least restrictive (T0).
    fn default() -> Self {
        Self::DEFAULT_FLOOR
    }
}

impl serde::Serialize for SandboxTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for SandboxTier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SandboxTier::try_from_manifest_str(&s)
            .map_err(|e| serde::de::Error::custom(format!("invalid sandbox tier: {e}")))
    }
}

impl std::fmt::Display for SandboxTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

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

    #[test]
    fn sandbox_tier_consts() {
        assert_eq!(SandboxTier::T0, SandboxTier(0));
        assert_eq!(SandboxTier::T1, SandboxTier(1));
        assert_eq!(SandboxTier::T2, SandboxTier(2));
        assert_eq!(SandboxTier::T3, SandboxTier(3));
        assert_eq!(SandboxTier::T4, SandboxTier(4));
    }

    #[test]
    fn sandbox_tier_default_is_t2() {
        assert_eq!(SandboxTier::default(), SandboxTier::T2, "DF18: default must be most restrictive enforceable tier");
    }

    #[test]
    fn try_from_u8_valid() {
        assert_eq!(SandboxTier::try_from_u8(0).unwrap(), SandboxTier::T0);
        assert_eq!(SandboxTier::try_from_u8(2).unwrap(), SandboxTier::T2);
        assert_eq!(SandboxTier::try_from_u8(4).unwrap(), SandboxTier::T4);
    }

    #[test]
    fn try_from_u8_rejects_out_of_range() {
        assert!(SandboxTier::try_from_u8(5).is_err());
        assert!(SandboxTier::try_from_u8(255).is_err());
    }

    #[test]
    fn try_from_manifest_str() {
        assert_eq!(SandboxTier::try_from_manifest_str("T0").unwrap(), SandboxTier::T0);
        assert_eq!(SandboxTier::try_from_manifest_str("T1").unwrap(), SandboxTier::T1);
        assert_eq!(SandboxTier::try_from_manifest_str("T2").unwrap(), SandboxTier::T2);
        assert_eq!(SandboxTier::try_from_manifest_str("T3").unwrap(), SandboxTier::T3);
        assert!(SandboxTier::try_from_manifest_str("t0").is_err()); // case-sensitive
        assert!(SandboxTier::try_from_manifest_str("T5").is_err());
        assert!(SandboxTier::try_from_manifest_str("foo").is_err());
        assert!(SandboxTier::try_from_manifest_str("0").is_err(), "numeric strings must be rejected");
        assert!(SandboxTier::try_from_manifest_str("4").is_err(), "T4 string must be rejected");
    }

    #[test]
    fn display_format() {
        assert_eq!(SandboxTier::T0.to_string(), "T0");
        assert_eq!(SandboxTier::T2.to_string(), "T2");
    }
}
