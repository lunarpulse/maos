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

/// One swap attempted while the predecessor held pending halts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Swap {
    pub halts_before: u32,
    pub halts_after: u32,
    pub resolution_state_kept: bool,
    pub receipted_drains: u32,
    pub accepted: bool,
    pub predecessor_version: u32,
    pub successor_version: Option<u32>,
}

/// I14 observation: the swaps attempted against Spirits with pending halts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub swaps: Vec<Swap>,
}

/// I14 predicate: with a non-empty halt set, an accepted swap needs a successor declaring the
/// predecessor's halt-protocol version and either carries every halt with its resolution state
/// or drains every halt with a receipt; a refused swap leaves every halt intact.
pub fn check(obs: &Observation) -> Result<(), crate::invariants::Violation> {
    use crate::invariants::ensure;
    for s in obs.swaps.iter().filter(|s| s.halts_before > 0) {
        let carried = s.halts_after == s.halts_before && s.resolution_state_kept;
        let drained = s.halts_after == 0 && s.receipted_drains == s.halts_before;
        if s.accepted {
            ensure(
                s.successor_version == Some(s.predecessor_version),
                "I14",
                "successor declares no matching halt-protocol compatibility",
            )?;
            ensure(
                carried || drained,
                "I14",
                "swap neither carried nor drained every halt",
            )?;
        } else {
            ensure(
                carried,
                "I14",
                "refused swap altered the predecessor's halts",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_continuity_variants() {
        let c = HaltContinuityCheck::Drained;
        assert!(matches!(c, HaltContinuityCheck::Drained));
        let c2 = HaltContinuityCheck::MigratedSchemaCompatibleVN(2);
        assert!(matches!(
            c2,
            HaltContinuityCheck::MigratedSchemaCompatibleVN(2)
        ));
    }
}
