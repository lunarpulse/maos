#![forbid(unsafe_code)]

//! Collective→Private/Shared downgrade router (Story 11.2a AC4).
//!
//! When the collective tier (`maos-loom-lite`, Postgres+pgvector) is unavailable,
//! the kernel must decide whether a collective read/scan can be transparently
//! **degraded** to a Private/Shared fallback, or whether it must **fail closed**.
//! This module provides the routing primitive that makes that decision.
//!
//! # Decision shape
//!
//! [`DowngradeOutcome`] captures the three terminal states of a downgrade
//! attempt:
//!
//! - [`DowngradeOutcome::Served`] — the collective tier answered normally; no
//!   downgrade happened.
//! - [`DowngradeOutcome::Degraded`] — the collective tier was unreachable/timed
//!   out (a recoverable outage), so the caller transparently fell back to a
//!   Private/Shared copy. The reason is recorded for audit.
//! - [`DowngradeOutcome::FailedClosed`] — the failure is NOT a recoverable
//!   outage (e.g. a data-layer error), so serving a fallback would silently mask
//!   corruption. The op refuses and surfaces the error.
//!
//! # Wiring
//!
//! This is a greenfield routing **primitive**. It is consumed by the existing
//! `CollectiveMemoryPort` adapter (or a wrapper around it) — the adapter maps a
//! `CollectivePortError` into a [`DowngradeOutcome`] via
//! [`DowngradeRouter::should_degrade`], and validates any foreign
//! re-admittance via [`DowngradeRouter::check_region_identity`] before serving
//! re-attested data. The router itself holds no I/O: it only decides.

use maos_domain::ports::collective_memory::CollectivePortError;

/// The terminal outcome of a collective-tier downgrade attempt.
///
/// See the module docs for the full decision model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DowngradeOutcome {
    /// The collective tier served the request normally; nothing was downgraded.
    Served,

    /// The collective tier suffered a recoverable outage (unreachable/timeout),
    /// so the caller fell back to a Private/Shared copy. The reason is kept for
    /// audit/replay.
    Degraded {
        /// Human-readable reason the downgrade was triggered (e.g. the typed
        /// `CollectivePortError` rendered as a string).
        reason: String,
    },

    /// The failure is NOT a recoverable outage, so a fallback would silently
    /// mask corruption or a data-layer fault. The op refused and surfaced the
    /// error instead of serving stale/foreign data.
    FailedClosed {
        /// Human-readable reason the op failed closed.
        reason: String,
    },
}

/// Stateless-with-identity routing primitive for collective→Private/Shared
/// downgrade decisions.
///
/// Carries only the router's own `home_region` so that
/// [`DowngradeRouter::check_region_identity`] can reject loopback (serving a
/// region's own data back to itself re-attested as "foreign") and empty source
/// labels.
pub struct DowngradeRouter {
    /// The region this router is mediating for. Used to reject self-loopback.
    home_region: String,
}

impl DowngradeRouter {
    /// Construct a router bound to `home_region`.
    pub fn new(home_region: String) -> Self {
        Self { home_region }
    }

    /// Decide whether a collective-tier error is a recoverable outage that
    /// justifies a transparent downgrade.
    ///
    /// Returns `true` ONLY for the recoverable transport-level failures:
    /// [`CollectivePortError::Unreachable`] and [`CollectivePortError::Timeout`].
    /// Everything else (a forwarded `Memory`-layer error, or an internal
    /// `Transport` protocol fault) is treated as non-recoverable — downgrading
    /// would mask a real fault, so the caller must fail closed instead.
    pub fn should_degrade(error: &CollectivePortError) -> bool {
        matches!(
            error,
            CollectivePortError::Unreachable { .. } | CollectivePortError::Timeout { .. }
        )
    }

    /// Validate that `source_region` names a genuine foreign origin before
    /// serving re-attested cross-region data.
    ///
    /// Rejects two cases that must never silently succeed:
    ///
    /// - **Empty** `source_region`: an unlabelled origin cannot be attested, so
    ///   serving it would let un-re-attested foreign data flow unchecked.
    /// - **Loopback** (`source_region == home_region`): a region must not
    ///   re-attest its OWN data as "foreign" — that is an identity-confusion
    ///   attack on the audit trail.
    ///
    /// Returns `Ok(())` when the origin is a non-empty, genuinely foreign
    /// region label.
    pub fn check_region_identity(&self, source_region: &str) -> Result<(), String> {
        if source_region.is_empty() {
            return Err(format!(
                "cannot serve un-re-attested foreign data: source_region is empty \
                 (home_region = {home:?})",
                home = self.home_region
            ));
        }
        if source_region == self.home_region {
            return Err(format!(
                "region-identity violation: source_region {src:?} equals home region \
                 {home:?} — loopback re-attestation is forbidden",
                src = source_region,
                home = self.home_region
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::memory::MemoryError;

    #[test]
    fn test_unreachable_should_degrade() {
        let err = CollectivePortError::Unreachable {
            reason: "connection refused".to_string(),
        };
        assert!(
            DowngradeRouter::should_degrade(&err),
            "Unreachable must be a degradable (recoverable) outage"
        );
    }

    #[test]
    fn test_timeout_should_degrade() {
        let err = CollectivePortError::Timeout { timeout_ms: 5_000 };
        assert!(
            DowngradeRouter::should_degrade(&err),
            "Timeout must be a degradable (recoverable) outage"
        );
    }

    #[test]
    fn test_memory_error_should_not_degrade() {
        // A forwarded memory-layer error is a data-layer fault, not a
        // recoverable outage: downgrading would mask corruption → fail closed.
        let err = CollectivePortError::Memory(MemoryError::NamespaceViolation(
            "boom".to_string(),
        ));
        assert!(
            !DowngradeRouter::should_degrade(&err),
            "Memory(...) must NOT degrade — it is a non-recoverable data fault"
        );
    }

    #[test]
    fn test_region_identity_rejects_empty() {
        let router = DowngradeRouter::new("us-east-1".to_string());
        let res = router.check_region_identity("");
        assert!(
            res.is_err(),
            "empty source_region must be rejected (un-re-attested origin)"
        );
    }

    #[test]
    fn test_region_identity_rejects_self() {
        let router = DowngradeRouter::new("eu-west-2".to_string());
        let res = router.check_region_identity("eu-west-2");
        assert!(
            res.is_err(),
            "source_region == home_region is loopback and must be rejected"
        );
    }

    #[test]
    fn test_region_identity_accepts_foreign() {
        let router = DowngradeRouter::new("us-east-1".to_string());
        let res = router.check_region_identity("ap-southeast-2");
        assert!(
            res.is_ok(),
            "a genuinely foreign, non-empty region label must be accepted"
        );
    }

    #[test]
    fn test_transport_error_should_not_degrade() {
        // A Transport error is a protocol fault (not a recoverable outage),
        // so it must NOT trigger degrade — fail closed.
        let err = CollectivePortError::Transport("protocol error".to_string());
        assert!(
            !DowngradeRouter::should_degrade(&err),
            "Transport error must NOT degrade — it is a non-recoverable fault"
        );
    }

    #[test]
    fn test_outcome_variants_distinct() {
        // Verify the three DowngradeOutcome variants are distinct.
        let served = DowngradeOutcome::Served;
        let degraded = DowngradeOutcome::Degraded {
            reason: "unreachable".to_string(),
        };
        let failed = DowngradeOutcome::FailedClosed {
            reason: "data fault".to_string(),
        };
        assert_ne!(served, degraded);
        assert_ne!(served, failed);
        assert_ne!(degraded, failed);
    }

    #[test]
    fn test_region_identity_case_sensitive() {
        // Region comparison is case-sensitive (canonical ascii-v1).
        let router = DowngradeRouter::new("us-east-1".to_string());
        // Different case = different region → accepted.
        assert!(
            router.check_region_identity("US-EAST-1").is_ok(),
            "case-different region must be accepted (canonical form differs)"
        );
    }
}
