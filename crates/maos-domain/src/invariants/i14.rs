//! I14: Hot-swap preserves halt continuity.
//!
//! When a Spirit with a non-empty `halt_set` is hot-swapped, either every
//! halt is drained OR every halt is migrated to the successor with full
//! resolution-path state, AND the successor's manifest declares
//! `halt_protocol_compatibility = N`.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3 / v0.5**: `—` (unchanged).
//! - **v0.9**: `runtime` — Hot-Swap Coordinator checks `halt_set` before
//!   swap; rejects with `EHaltContinuityViolation` otherwise.
//! - **v1.0 / v1.5**: `runtime` (v1.5 promoted to `fuzz`).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i14::{InvariantI14, HaltContinuityCheck};
//!
//! let _marker: InvariantI14 = InvariantI14;
//! let check = HaltContinuityCheck::MigratedSchemaCompatibleVN(1);
//! assert!(matches!(check, HaltContinuityCheck::MigratedSchemaCompatibleVN(1)));
//! ```

/// I14 marker type — Hot-swap preserves halt continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI14;

/// The halt-continuity gate type checked by the Hot-Swap Coordinator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HaltContinuityCheck {
    /// All halts were resolved before swap completed.
    Drained,
    /// All halts migrated to successor with compatible halt-protocol version.
    MigratedSchemaCompatibleVN(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_continuity_variants() {
        let c = HaltContinuityCheck::Drained;
        assert!(matches!(c, HaltContinuityCheck::Drained));
        let c2 = HaltContinuityCheck::MigratedSchemaCompatibleVN(2);
        assert!(matches!(c2, HaltContinuityCheck::MigratedSchemaCompatibleVN(2)));
    }
}
