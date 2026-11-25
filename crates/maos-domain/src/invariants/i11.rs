//! I11: Persisted digests reference their raw source frames.
//!
//! Every payload tagged `kind: digest` carries non-empty `source_log_ref`
//! and `distillation_depth`. Kernel rejects malformed writes with
//! `EDigestAuditChainMissing`.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3**: `—` (unchanged).
//! - **v0.5**: `runtime` — Capability Registry validates `source_log_ref`
//!   and `distillation_depth` on every digest-tagged write.
//! - **v0.9 / v1.0 / v1.5**: `runtime` (v1.0 promoted to `fuzz`).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i11::{InvariantI11, DigestRef};
//!
//! let _marker: InvariantI11 = InvariantI11;
//! let digest = DigestRef {
//!     source_log_ref: vec!["frame-001".into(), "frame-002".into()],
//!     distillation_depth: 1,
//! };
//! assert_eq!(digest.distillation_depth, 1);
//! ```

/// I11 marker type — Persisted digests reference raw source frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI11;

/// Reference from a digest back to its raw source frames.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DigestRef {
    /// Frame IDs (transitively flattened to original raw frames).
    pub source_log_ref: Vec<String>,
    /// 0 = raw; N+1 = digest of depth N.
    pub distillation_depth: u32,
}

/// One digest write: what was requested, what was persisted and why a refusal was refused.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attempt {
    pub requested_sources: u32,
    pub requested_depth: u32,
    pub persisted: bool,
    pub persisted_sources: u32,
    pub persisted_raw_only: bool,
    pub refusal: Option<String>,
}

/// I11 observation: the digest writes attempted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub attempts: Vec<Attempt>,
}

/// I11 predicate: a persisted digest has a non-empty, flattened (raw-only) `source_log_ref` and a
/// depth of at least 1; a request without them is refused `EDigestAuditChainMissing`.
pub fn check(obs: &Observation) -> Result<(), crate::invariants::Violation> {
    use crate::invariants::ensure;
    for a in &obs.attempts {
        let chained = a.requested_sources > 0 && a.requested_depth >= 1;
        if a.persisted {
            ensure(
                chained,
                "I11",
                "digest persisted without source_log_ref or depth",
            )?;
            ensure(
                a.persisted_sources > 0,
                "I11",
                "persisted digest has an empty source_log_ref",
            )?;
            ensure(
                a.persisted_raw_only,
                "I11",
                "persisted digest source_log_ref is not flattened",
            )?;
        } else if !chained {
            let refused = a.refusal.as_deref() == Some("EDigestAuditChainMissing");
            ensure(
                refused,
                "I11",
                "chainless digest not refused EDigestAuditChainMissing",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_ref_depth() {
        let d = DigestRef {
            source_log_ref: vec!["f1".into()],
            distillation_depth: 2,
        };
        assert_eq!(d.distillation_depth, 2);
    }
}
