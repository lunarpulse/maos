//! I5: Memory scopes are kernel-enforced.
//!
//! A Spirit cannot read another Spirit's private memory or write outside
//! its declared namespace. Multi-Spirit deployments depend on this.
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — Memory Manager namespace check on every read/write.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i5::{InvariantI5, MemoryScope, NamespaceKey};
//!
//! let _marker: InvariantI5 = InvariantI5;
//! let key: NamespaceKey<MemoryScope> = NamespaceKey::new("spirit-nash::session-7");
//! assert_eq!(key.as_str(), "spirit-nash::session-7");
//! ```

/// I5 marker type — Memory scopes are kernel-enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI5;

/// Memory scope tier — the kernel enforces these boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum MemoryScope {
    /// Private to a single Spirit instance.
    Private = 0,
    /// Shared among Spirits within the same Host.
    Shared = 1,
    /// Collective across pre-paired Hosts.
    Collective = 2,
}

/// Typed namespace key — the `S` parameter carries the memory scope at
/// the type level so the kernel can enforce scope boundaries structurally.
///
/// At v0.1-α the type parameter `S` is unconstrained (any type satisfies
/// it). A sealed trait bound restricting `S` to `MemoryScope` variants
/// can be added when the kernel wiring ships in Story 1b.2, at which point
/// the type-level enforcement becomes non-bypassable.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NamespaceKey<S> {
    key: String,
    _scope: core::marker::PhantomData<S>,
}

impl<S> NamespaceKey<S> {
    /// Create a new namespace key. The scope type parameter is set at
    /// construction and cannot change without a type-level transform.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            _scope: core::marker::PhantomData,
        }
    }

    /// Return the key string.
    pub fn as_str(&self) -> &str {
        &self.key
    }
}

/// A private-tier read: who asked, and whose value (if any) it returned.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Read {
    pub reader: String,
    pub value_owner: Option<String>,
}

/// A namespaced write: who wrote, whether the namespace was theirs to write, and whether it landed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Write {
    pub writer: String,
    pub authorized: bool,
    pub landed: bool,
}

/// I5 observation: every read and write a Spirit attempted against kernel memory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub reads: Vec<Read>,
    pub writes: Vec<Write>,
}

/// I5 predicate: a read returns only the reader's own value, and a write lands only inside a
/// namespace the writer is authorized for.
pub fn check(obs: &Observation) -> Result<(), crate::invariants::Violation> {
    use crate::invariants::ensure;
    for r in &obs.reads {
        let own = r
            .value_owner
            .as_ref()
            .is_none_or(|owner| *owner == r.reader);
        ensure(own, "I5", "read disclosed another Spirit's private memory")?;
    }
    for w in &obs.writes {
        ensure(
            !w.landed || w.authorized,
            "I5",
            "write landed outside the declared namespace",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_key_typed_scope() {
        // Note: Rust does not permit enum variants as type parameters.
        // The type-level scope enforcement is expressed via NamespaceKey<MemoryScope>.
        let key: NamespaceKey<MemoryScope> = NamespaceKey::new("k1");
        assert_eq!(key.as_str(), "k1");
    }
}
