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
