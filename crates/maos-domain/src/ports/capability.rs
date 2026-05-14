//! Capability Registry port trait per architecture §4.6 + ADR-030 decomposition.
//!
//! The Capability Registry mediates every external call. At v0.1-α this
//! port declares the four ADR-022 universal-arithmetic predicates — the
//! kernel-side surface that fires epistemic halts when a Spirit's
//! tagged-scalar slot crosses a threshold. The full
//! issue/verify/revoke/audit-write surface lands in Story 1b.2; the
//! mailbox-side `iac.send` mediation lands in Story 6.1.

use crate::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use crate::invariants::i9::SandboxTier;
use thiserror::Error;

/// Capability Registry — mediates every external call, evaluates ADR-022
/// universal-arithmetic predicates against per-Spirit tagged scalars.
///
/// Per ADR-030: split internally into `cap_tokens` (hot path),
/// `cap_policy`, `cap_audit`, `cap_quota`. The port trait surface at v0.1-α
/// declares only the universal-arithmetic predicates; the cap-tokens
/// hot-path surface (issue/verify/revoke) lands in Story 1b.2.
pub trait CapabilityRegistryPort {
    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value > threshold`. One of the four ADR-022
    /// predicates — the ENTIRE kernel-side computational surface at v0.1.
    /// Spirit-side `[epistemic_policy]` rules reference this predicate
    /// for halt-on-scalar-drift policies (architecture §4.6.1).
    fn on_value_above(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < threshold`. ADR-022 predicate.
    fn on_value_below(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `lower <= value <= upper`. ADR-022 predicate;
    /// inclusive bounds at v0.1-α (open/half-open variants deferred to
    /// Story 4.2 if a Spirit class demands them).
    fn on_value_within(&self, value: f64, lower: f64, upper: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < lower OR value > upper`. ADR-022 predicate.
    fn on_value_outside(&self, value: f64, lower: f64, upper: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Issue a capability token for `spirit_pid` to exercise `scope`.
    /// TTL is capped per `intent_class` (60s HighPrivilege, 300s Standard,
    /// 900s Readonly per ADR-023). The token is bound to
    /// (spirit_pid + boot_nonce + expiry + posture_snapshot_hash).
    fn issue(
        &self,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
        posture_snapshot_hash: [u8; 32],
        intent_class: IntentClass,
    ) -> Result<CapabilityToken, CapError>;

    /// Class: universal-arithmetic
    ///
    /// Verify a capability token against current state. Re-reads the
    /// current state from the shard ring AND cross-checks against the
    /// `posture_snapshot_hash` and `intent_class` baked into the token at
    /// issue. Rejects if posture changed since issuance (TOCTOU correctness).
    fn verify(
        &self,
        token: &CapabilityToken,
        current_posture_hash: [u8; 32],
        current_sandbox: SandboxTier,
    ) -> Result<(), CapError>;

    /// Class: universal-arithmetic
    ///
    /// Revoke a single token by `token_id`. Flips the in-memory revoked
    /// flag and emits a `CapAuditEvent::Revoke` to the audit channel.
    fn revoke(&self, token_id: TokenId) -> Result<(), CapError>;

    /// Class: data-movement
    ///
    /// Record an invocation in the audit log. Forwards the audit event to
    /// the writer task without semantic interpretation.
    fn record_invocation(
        &self,
        token: &CapabilityToken,
        intent: String,
        payload: &[u8],
    ) -> Result<(), CapError>;
}

/// Capability-registry error taxonomy.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CapError {
    /// Crypto operation failed.
    #[error("crypto operation failed: {0}")]
    CryptoFailed(#[from] crate::ports::crypto::CryptoError),
    /// Token not found in shard ring.
    #[error("token not found in shard ring")]
    UnknownToken,
    /// Token expired (TTL elapsed).
    #[error("token expired (TTL elapsed)")]
    Expired,
    /// Token revoked.
    #[error("token revoked")]
    Revoked,
    /// Token spirit-id mismatch — possible token theft / replay.
    #[error("token spirit-id mismatch — possible token theft / replay")]
    SpiritIdMismatch,
    /// Token signature integrity violation.
    #[error("token signature integrity violation")]
    SignatureMismatch,
    /// Current posture differs from token-issued posture — TOCTOU rejection.
    #[error("current posture differs from token-issued posture — TOCTOU rejection")]
    PostureMismatch,
    /// Spirit quota exhausted.
    #[error("Spirit {spirit_id} quota exhausted")]
    ContextExhausted { spirit_id: u32 },
    /// Policy denied capability.
    #[error("policy denied capability")]
    PolicyDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_error_distinguishes_variants() {
        assert_ne!(
            CapError::UnknownToken,
            CapError::Expired
        );
        assert_ne!(
            CapError::ContextExhausted { spirit_id: 1 },
            CapError::ContextExhausted { spirit_id: 2 }
        );
    }
}
