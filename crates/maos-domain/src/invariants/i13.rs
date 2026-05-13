//! I13: Digests carry intent provenance.
//!
//! The kernel computes the recall-union from `log.recall` tracking.
//! A consumer operating under intent `Y` rejects digests whose
//! `intent_lineage` is not contained in `allowed-promotion-set(Y)`.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3**: `—` (unchanged).
//! - **v0.5**: `runtime` — kernel-computed `intent_lineage`; consumer
//!   admission rejects with `EIntentPromotionDenied`.
//! - **v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i13::{InvariantI13, IntentLineage, AllowedPromotionSet};
//! use maos_domain::invariants::i8::A2AIntent;
//!
//! let _marker: InvariantI13 = InvariantI13;
//! let lineage = IntentLineage::new(vec![A2AIntent::new("consult")]);
//! let mut allowed = AllowedPromotionSet::new();
//! allowed.insert(A2AIntent::new("consult"));
//! assert!(allowed.allows(&lineage));
//! ```

/// I13 marker type — Digests carry intent provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI13;

/// The intent lineage computed by the kernel for a digest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntentLineage(Vec<crate::invariants::i8::A2AIntent>);

impl IntentLineage {
    /// Create from a vector of intents.
    pub fn new(intents: Vec<crate::invariants::i8::A2AIntent>) -> Self {
        Self(intents)
    }

    /// View the contained intents.
    pub fn as_slice(&self) -> &[crate::invariants::i8::A2AIntent] {
        &self.0
    }
}

/// Consumer-side allowlist: which intents may be promoted into this
/// consumer's reasoning context.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AllowedPromotionSet {
    intents: std::collections::BTreeSet<crate::invariants::i8::A2AIntent>,
}

impl AllowedPromotionSet {
    /// Create an empty promotion set.
    pub fn new() -> Self {
        Self {
            intents: std::collections::BTreeSet::new(),
        }
    }

    /// Insert an allowed intent.
    pub fn insert(&mut self, intent: crate::invariants::i8::A2AIntent) {
        self.intents.insert(intent);
    }

    /// Check whether the given lineage is contained in this promotion set.
    pub fn allows(&self, lineage: &IntentLineage) -> bool {
        lineage.as_slice().iter().all(|i| self.intents.contains(i))
    }
}

impl Default for AllowedPromotionSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariants::i8::A2AIntent;

    #[test]
    fn allowed_promotion_set() {
        let mut allowed = AllowedPromotionSet::new();
        allowed.insert(A2AIntent::new("consult"));
        let lineage = IntentLineage::new(vec![A2AIntent::new("consult")]);
        assert!(allowed.allows(&lineage));
        let bad = IntentLineage::new(vec![A2AIntent::new("delegate")]);
        assert!(!allowed.allows(&bad));
    }
}
