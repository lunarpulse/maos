//! Enterprise identity assertion port — sync trait for out-of-kernel OIDC/SSO
//! verification (Story 11.4c, ADR-051 / NFR-Sec-18).
//!
//! The kernel stays identity-agnostic. Verified principals are projected onto
//! `PolicyDecisionRequest::principal_attributes` by the composition/adapters and
//! provenance is recorded out-of-kernel; no `CapabilityToken` field is added.

use std::collections::HashMap;

/// Principal produced only after an identity assertion has been cryptographically
/// verified and its issuer/audience/time claims have passed fail-closed checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub subject: String,
    pub issuer: String,
    pub audience: String,
    pub attributes: HashMap<String, String>,
}

/// Story 23-2 (round 4) — what a verification relied on, so the issuance point can ask whether
/// it still holds: the `kid` of the key that verified, the publication stamp of the key set that
/// held it, and the clock reading (milliseconds, the verifier's clock) at which that set's hard
/// TTL ends. Opaque to everyone but the verifier that made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFreshness {
    pub kid: String,
    pub key_set: u64,
    pub expires_at_ms: u64,
}

/// Fail-closed identity verification errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("identity key material unavailable")]
    JwksUnavailable,
    #[error("identity assertion algorithm rejected")]
    AlgorithmRejected,
    #[error("identity assertion signature invalid")]
    SignatureInvalid,
    #[error("identity assertion expired")]
    Expired,
    #[error("identity assertion not yet valid")]
    NotYetValid,
    #[error("identity assertion audience mismatch")]
    AudienceMismatch,
    #[error("identity assertion issuer untrusted")]
    IssuerUntrusted,
    #[error("malformed identity assertion")]
    MalformedAssertion,
    #[error("system clock unavailable for provenance stamp")]
    ClockUnavailable,
}

/// Sync port trait for enterprise identity assertion verification.
pub trait IdentityAssertionPort: Send + Sync {
    /// Class: supervision
    ///
    /// Verify an OIDC/JWT assertion and return a principal only when signature,
    /// algorithm allowlist, issuer, audience, `exp`, and `nbf` all pass.
    fn verify(&self, assertion: &str) -> Result<AuthenticatedPrincipal, IdentityError>;

    /// Class: supervision
    ///
    /// Whether verifier configuration and key material are loaded. A configured
    /// but unhealthy verifier must fail closed at composition-root call sites.
    fn is_healthy(&self) -> bool;

    /// Class: supervision
    ///
    /// As [`Self::verify`], but MUST NOT wait on key discovery or any network refresh: it
    /// answers from the keys already held, against the verifier's own clock. Used to re-check
    /// a principal at a commit point that holds a lock. The default is correct ONLY for a
    /// verifier that never waits; every implementor that can wait MUST override it.
    fn verify_without_refresh(
        &self,
        assertion: &str,
    ) -> Result<AuthenticatedPrincipal, IdentityError> {
        self.verify(assertion)
    }

    /// Class: supervision
    ///
    /// [`Self::verify`] (`wait`) or [`Self::verify_without_refresh`] (`!wait`), and, when the
    /// key came from a rotating source, the [`KeyFreshness`] the verdict rests on. The default
    /// (a static key set that never expires or rotates) carries no proof; a verifier whose keys
    /// expire or are replaced MUST override this and [`Self::key_still_fresh`].
    fn verify_with_freshness(
        &self,
        assertion: &str,
        wait: bool,
    ) -> Result<(AuthenticatedPrincipal, Option<KeyFreshness>), IdentityError> {
        let principal = if wait {
            self.verify(assertion)
        } else {
            self.verify_without_refresh(assertion)
        }?;
        Ok((principal, None))
    }

    /// Class: supervision
    ///
    /// Whether the key a [`KeyFreshness`] names is still the published, unexpired key. MUST NOT
    /// wait on discovery or hold a lock past its own answer; the default (no rotation) is `true`.
    fn key_still_fresh(&self, _proof: &KeyFreshness) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticated_principal_is_trait_object_safe_payload() {
        let mut attributes = HashMap::new();
        attributes.insert("email".to_string(), "reza@maos.example".to_string());
        let principal = AuthenticatedPrincipal {
            subject: "reza@maos.example".to_string(),
            issuer: "https://idp.maos.example".to_string(),
            audience: "maos-deploy-alpha".to_string(),
            attributes,
        };
        assert_eq!(
            principal.attributes.get("email").map(String::as_str),
            Some("reza@maos.example")
        );
    }
}
