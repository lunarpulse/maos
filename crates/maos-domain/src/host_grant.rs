//! Story 8.12 AC5 (FORK A — Winston host-grant model) — host-side capability
//! grant allowlist.
//!
//! The trust direction is **inverted** relative to the rejected "honor the
//! manifest-declared tier" design: the artifact under least trust never decides
//! its own sandbox (the 8.7–8.9 anti-pattern — never trust the self-declared
//! field). Instead:
//!
//! - the **manifest requests** a tier and egress destinations;
//! - the **host grants** via an operator-configured allowlist keyed on the
//!   attested-image + signing-key → `{permitted_tier, permitted_egress}`.
//!
//! **Generalization (Cross-Impact #4):** this is deliberately NOT CliWrapper-only.
//! Stories 8.14b/8.14c (real MCP drivers — Calendar/Slack/web/arXiv) need the
//! same host-grant + egress-allowlist surface; they reuse [`HostGrant`] +
//! [`HostGrantAllowlist`] keyed on the same attested-image + signing-key tuple.

use crate::invariants::i9::SandboxTier;

/// Canonical lower-case DNS/IPv4 authority; manifests require an explicit port.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct EgressDestination {
    pub host: String,
    pub port: u16,
}

impl TryFrom<String> for EgressDestination {
    type Error = &'static str;
    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        let (host, port) = value.rsplit_once(':').ok_or("egress requires host:port")?;
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0 && !port.starts_with('+'))
            .ok_or("egress port must be 1..=65535")?;
        if host.len() > 253
            || !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            || (host.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                && host.parse::<std::net::Ipv4Addr>().is_err())
        {
            return Err("egress host must be lower-case DNS labels or an IPv4 literal");
        }
        let host_len = host.len();
        value.truncate(host_len);
        Ok(Self { host: value, port })
    }
}

impl EgressDestination {
    /// Preserve operator grants: a bare host grants only port 443.
    pub fn parse_granted(value: &str) -> Result<Self, &'static str> {
        Self::try_from(if value.contains(':') {
            value.to_owned()
        } else {
            format!("{value}:443")
        })
    }
}

impl std::fmt::Display for EgressDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl serde::Serialize for EgressDestination {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

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
    /// Host-granted destinations; empty means no egress permission.
    pub permitted_egress_destinations: Vec<EgressDestination>,
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
        permitted_egress_destinations: Vec<EgressDestination>,
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
            permitted_egress_destinations: vec![EgressDestination::parse_granted(
                "api.anthropic.com",
            )
            .unwrap()],
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
