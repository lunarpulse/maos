//! Story 8.12 AC5 (FORK A — Winston host-grant model) — host-side capability
//! grant allowlist.
//!
//! The trust direction is **inverted** relative to the rejected "honor the
//! manifest-declared tier" design: the artifact under least trust never decides
//! its own sandbox (the 8.7–8.9 anti-pattern — never trust the self-declared
//! field). Instead:
//!
//! - the **manifest requests** a tier (and, later, egress destinations);
//! - the **host grants** via an operator-configured allowlist keyed on the
//!   attested-image + signing-key → `{permitted_tier, permitted_egress}`.
//!
//! **Generalization (Cross-Impact #4):** this is deliberately NOT CliWrapper-only.
//! Stories 8.14b/8.14c (real MCP drivers — Calendar/Slack/web/arXiv) need the
//! same host-grant + egress-allowlist surface; they reuse [`HostGrant`] +
//! [`HostGrantAllowlist`] keyed on the same attested-image + signing-key tuple.

use crate::invariants::i9::SandboxTier;

/// A single host-side grant: what the operator permits for an attested artifact.
///
/// Keyed on `(attested_image, signing_key_id)` — both are properties the host
/// verifies, never fields the artifact self-asserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrant {
    /// Attested container-image reference (or binary attestation id) the grant
    /// applies to. Matched against the host-verified image, not the manifest.
    pub attested_image: String,
    /// Signing-key identity (publisher key id) the grant applies to.
    pub signing_key_id: String,
    /// The maximum sandbox tier the host permits for this artifact. A request
    /// for a tier above this is denied fail-closed (no silent downgrade).
    pub permitted_tier: SandboxTier,
    /// Named egress destinations the host permits (e.g. `api.anthropic.com`).
    /// Empty = no egress permitted. Enforcement depth is recorded per-story
    /// (8.12 lands the grant seam; full enforced egress allowlisting may be a
    /// follow-up — log enforced-vs-declared, never a silent gap).
    pub permitted_egress_destinations: Vec<String>,
}

/// Host-side grant lookup. The operator config implements this; admission
/// consults it. NOT reachable from the artifact.
pub trait HostGrantAllowlist {
    /// Return the grant for `(attested_image, signing_key_id)`, or `None` if the
    /// host grants nothing for this artifact (→ fail-closed at the call site).
    fn lookup(&self, attested_image: &str, signing_key_id: &str) -> Option<&HostGrant>;
}

/// In-memory allowlist backed by a `Vec<HostGrant>` (the v0.9 operator-config
/// seam; Epic 9 productionizes the operator-facing management of it).
#[derive(Debug, Clone, Default)]
pub struct StaticHostGrantAllowlist {
    grants: Vec<HostGrant>,
}

impl StaticHostGrantAllowlist {
    pub fn new(grants: Vec<HostGrant>) -> Self {
        Self { grants }
    }

    pub fn with_grant(mut self, grant: HostGrant) -> Self {
        self.grants.push(grant);
        self
    }
}

impl HostGrantAllowlist for StaticHostGrantAllowlist {
    fn lookup(&self, attested_image: &str, signing_key_id: &str) -> Option<&HostGrant> {
        self.grants
            .iter()
            .find(|g| g.attested_image == attested_image && g.signing_key_id == signing_key_id)
    }
}

/// The host's decision on a tier request (AC5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierGrantDecision {
    /// The host grants exactly the requested tier (request ≤ permitted).
    Granted {
        tier: SandboxTier,
        permitted_egress_destinations: Vec<String>,
    },
    /// Fail-closed: no matching grant, or the requested tier exceeds the grant,
    /// or the platform cannot enforce the tier (non-Linux). NEVER a downgrade.
    Denied {
        requested: SandboxTier,
        /// The permitted tier if a grant exists (for the diagnostic), else the
        /// requested tier echoed back when no grant matched at all.
        permitted: SandboxTier,
        attested_image: String,
        reason: TierDenialReason,
    },
}

/// Why a tier request was denied (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierDenialReason {
    /// No grant matched `(attested_image, signing_key_id)`.
    NoMatchingGrant,
    /// A grant matched but the requested tier exceeds `permitted_tier`.
    ExceedsPermittedTier,
    /// The platform cannot enforce the requested tier (Linux-only fails closed —
    /// no silent tier drop on macOS/Windows).
    PlatformCannotEnforce,
}

/// Resolve a manifest tier *request* against the host grant allowlist (AC5).
///
/// Fail-closed at every branch: no matching grant → `Denied`; request above the
/// grant → `Denied`; non-Linux → `Denied` (Linux-only fails closed). On success
/// the host grants exactly the requested tier (never a silent downgrade).
pub fn resolve_tier_grant(
    requested: SandboxTier,
    attested_image: &str,
    signing_key_id: &str,
    allowlist: &dyn HostGrantAllowlist,
) -> TierGrantDecision {
    // Linux-only fails closed — the T3 container substrate is Linux-only and a
    // silent tier drop on macOS/Windows is exactly the anti-pattern AC5 forbids.
    #[cfg(not(target_os = "linux"))]
    {
        let _ = allowlist;
        return TierGrantDecision::Denied {
            requested,
            permitted: requested,
            attested_image: attested_image.to_string(),
            reason: TierDenialReason::PlatformCannotEnforce,
        };
    }

    #[cfg(target_os = "linux")]
    {
        match allowlist.lookup(attested_image, signing_key_id) {
            None => TierGrantDecision::Denied {
                requested,
                permitted: requested,
                attested_image: attested_image.to_string(),
                reason: TierDenialReason::NoMatchingGrant,
            },
            Some(grant) => {
                if requested <= grant.permitted_tier {
                    TierGrantDecision::Granted {
                        tier: requested,
                        permitted_egress_destinations: grant.permitted_egress_destinations.clone(),
                    }
                } else {
                    TierGrantDecision::Denied {
                        requested,
                        permitted: grant.permitted_tier,
                        attested_image: attested_image.to_string(),
                        reason: TierDenialReason::ExceedsPermittedTier,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowlist_t3() -> StaticHostGrantAllowlist {
        StaticHostGrantAllowlist::new(vec![HostGrant {
            attested_image: "ghcr.io/maos/worker@sha256:abc".into(),
            signing_key_id: "maos-publisher-1".into(),
            permitted_tier: SandboxTier::T3,
            permitted_egress_destinations: vec!["api.anthropic.com".into()],
        }])
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn grant_permits_requested_tier_at_or_below_ceiling() {
        let al = allowlist_t3();
        let d = resolve_tier_grant(
            SandboxTier::T3,
            "ghcr.io/maos/worker@sha256:abc",
            "maos-publisher-1",
            &al,
        );
        assert!(matches!(d, TierGrantDecision::Granted { tier, .. } if tier == SandboxTier::T3));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn no_matching_grant_fails_closed() {
        let al = allowlist_t3();
        let d = resolve_tier_grant(SandboxTier::T3, "unknown-image", "unknown-key", &al);
        assert!(matches!(
            d,
            TierGrantDecision::Denied {
                reason: TierDenialReason::NoMatchingGrant,
                ..
            }
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn request_above_grant_fails_closed_no_downgrade() {
        let al = StaticHostGrantAllowlist::new(vec![HostGrant {
            attested_image: "img".into(),
            signing_key_id: "key".into(),
            permitted_tier: SandboxTier::T2,
            permitted_egress_destinations: vec![],
        }]);
        let d = resolve_tier_grant(SandboxTier::T3, "img", "key", &al);
        // Denied — NOT silently downgraded to T2.
        assert!(matches!(
            d,
            TierGrantDecision::Denied {
                reason: TierDenialReason::ExceedsPermittedTier,
                ..
            }
        ));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_fails_closed() {
        let al = allowlist_t3();
        let d = resolve_tier_grant(
            SandboxTier::T3,
            "ghcr.io/maos/worker@sha256:abc",
            "maos-publisher-1",
            &al,
        );
        assert!(matches!(
            d,
            TierGrantDecision::Denied {
                reason: TierDenialReason::PlatformCannotEnforce,
                ..
            }
        ));
    }
}
