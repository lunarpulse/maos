//! I8: Cross-Host A2A interactions require explicit consent at both ends.
//!
//! Scoped to the typed intent of the message (not just the channel).
//! Channel-consent does not imply transaction-consent.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3 / v0.5**: `—` (unchanged).
//! - **v0.9**: `runtime` — A2A Gateway rejects frames with intent not in
//!   send-allowlist or accept-allowlist.
//! - **v1.0 / v1.5**: `runtime` (unchanged; promoted to `fuzz` at v1.5
//!   for cross-Host adversarial corpus).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i8::{InvariantI8, A2AIntent, IntentAllowlist};
//!
//! let _marker: InvariantI8 = InvariantI8;
//! let intent = A2AIntent::new("diagnosis-handoff");
//! let mut allowlist = IntentAllowlist::new();
//! allowlist.insert(intent.clone());
//! assert!(allowlist.contains(&A2AIntent::new("diagnosis-handoff")));
//! ```

/// I8 marker type — Cross-Host A2A requires explicit typed-intent consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI8;

/// Typed intent for A2A consent — the granularity of cross-Host consent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct A2AIntent(String);

impl A2AIntent {
    /// Create a new A2A intent.
    pub fn new(intent: impl Into<String>) -> Self {
        Self(intent.into())
    }

    /// Return the intent string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed allowlist wrapper around a set of A2A intents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntentAllowlist {
    intents: std::collections::BTreeSet<A2AIntent>,
}

impl IntentAllowlist {
    /// Create an empty allowlist.
    pub fn new() -> Self {
        Self {
            intents: std::collections::BTreeSet::new(),
        }
    }

    /// Insert an intent into the allowlist.
    pub fn insert(&mut self, intent: A2AIntent) {
        self.intents.insert(intent);
    }

    /// Check whether the allowlist contains the given intent.
    pub fn contains(&self, intent: &A2AIntent) -> bool {
        self.intents.contains(intent)
    }
}

impl Default for IntentAllowlist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_allowlist_membership() {
        let mut list = IntentAllowlist::new();
        list.insert(A2AIntent::new("consult"));
        assert!(list.contains(&A2AIntent::new("consult")));
        assert!(!list.contains(&A2AIntent::new("delegate")));
    }
}
