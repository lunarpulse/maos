//! I12: Every byte in Spirit context is traceable to a `log.recall`
//! or `event/inbound + shadow-recall` entry.
//!
//! When a Spirit emits a `decision.*` frame, the kernel attaches
//! `working_memory_digest_refs` populated from in-context digests and
//! inbound shadow-recall records.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3**: `—` (unchanged).
//! - **v0.5**: `runtime` — Capability Registry tracks per-Spirit in-context
//!   digest set + inbound shadow-recall records.
//! - **v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i12::{InvariantI12, WorkingMemoryDigestRefs};
//!
//! let _marker: InvariantI12 = InvariantI12;
//! let refs = WorkingMemoryDigestRefs::new(vec!["frame-001".into()]);
//! assert_eq!(refs.as_slice(), &["frame-001"]);
//! ```

/// I12 marker type — Every byte in Spirit context is traceable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI12;

/// Newtype wrapping the digest refs attached to a `decision.*` frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkingMemoryDigestRefs(Vec<String>);

impl WorkingMemoryDigestRefs {
    /// Create from a vector of frame IDs.
    pub fn new(refs: Vec<String>) -> Self {
        Self(refs)
    }

    /// View the contained frame IDs.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl Default for WorkingMemoryDigestRefs {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_memory_digest_refs() {
        let r = WorkingMemoryDigestRefs::new(vec!["f1".into(), "f2".into()]);
        assert_eq!(r.as_slice().len(), 2);
    }
}
