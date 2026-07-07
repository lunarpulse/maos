//! Story 11.4c Task 2 — PRINCIPAL GOVERNS AUTHORIZATION.
//!
//! # The contract defended
//!
//! `OidcVerifier::govern_authorization` re-runs the full OIDC verification
//! (signature + algorithm + issuer + audience + time claims) and, only when it
//! passes, returns the VERIFIED principal's projected attributes shaped to
//! populate the additive `PolicyDecisionRequest.principal_attributes` field
//! (the 11.4c identity layer over the 11.4a PDP request — F7, non-breaking).
//!
//! Two invariants pinned here:
//!   1. A bad-signature assertion is rejected BEFORE any authorization
//!      attributes are produced (fail-closed — the same tripwire as `verify`,
//!      not attribute-extraction-without-verification).
//!   2. The returned `principal_attributes` are the cryptographically-verified
//!      principal's projected claims (email/sub from the JWT payload), and
//!      they slot directly into `PolicyDecisionRequest.principal_attributes`
//!      (Some, not the 11.4a `None` default).
//!
//! The attributes equal the JWT payload, NOT a canned literal — they are
//! re-derived from the verified claims every call.

#[path = "fixtures.rs"]
mod fixtures;

use maos_domain::ports::{IdentityError, PolicyDecisionRequest};
use maos_sso::{GovernedAuthorization, OidcAlgorithm, OidcVerifier};

/// Build the reference verifier: static JWKS = {keyA}, allowlist = [RS256],
/// trusted issuer = ISS_GOOD, expected audience = AUD_EXPECTED.
fn verifier() -> OidcVerifier {
    OidcVerifier::from_static_jwks(
        fixtures::JWKS_KEY_A,
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
    )
    .expect("static JWKS + algorithm allowlist + issuer/aud config MUST parse")
}

/// govern_authorization re-verifies: an assertion signed by a key NOT in the
/// JWKS is rejected as `SignatureInvalid` and produces NO `GovernedAuthorization`.
/// If this returned Ok, an attacker token would govern authorization.
#[test]
fn govern_authorization_rejects_unverified_assertion_fail_closed() {
    let err = verifier()
        .govern_authorization(fixtures::TOKEN_WRONG_KEY, 4242, "fs.read")
        .expect_err(
            "an assertion signed by a key NOT in the JWKS MUST NOT govern authorization",
        );
    assert_eq!(
        err,
        IdentityError::SignatureInvalid,
        "govern_authorization must re-verify the signature — a bad signature yields \
         SignatureInvalid, never a GovernedAuthorization"
    );
}

/// The canonical accept case: a valid assertion governs authorization, the
/// returned `principal_attributes` are the VERIFIED principal's projected
/// claims, and those attributes populate a `PolicyDecisionRequest` (the 11.4c
/// additive identity layer — `Some`, not the 11.4a `None` default).
#[test]
fn governed_principal_attributes_populate_policy_decision_request() {
    let governed = verifier()
        .govern_authorization(fixtures::TOKEN_GOOD_RS256, 4242, "fs.read")
        .expect("a correctly-signed, in-audience, live assertion MUST govern authorization");

    // Type pin: govern_authorization returns a GovernedAuthorization carrying
    // principal_attributes + provenance (Task 2 shape).
    let _: &GovernedAuthorization = &governed;

    // The verified principal's `email` claim projects onto principal_attributes.
    // This is the JWT payload value, NOT a hardcoded literal — govern_authorization
    // re-derived it from the cryptographically verified claims.
    assert_eq!(
        governed.principal_attributes.get("email").map(String::as_str),
        Some(fixtures::EMAIL),
        "govern_authorization must surface the VERIFIED principal's email attribute"
    );
    // The verified subject is bound onto principal_attributes too — this is the
    // identity the PDP can authorize on, again from the verified token.
    assert_eq!(
        governed.principal_attributes.get("sub").map(String::as_str),
        Some(fixtures::PRINCIPAL_SUB),
        "the verified subject MUST be bound onto principal_attributes"
    );

    // The 11.4c additive layer: principal_attributes populate a
    // PolicyDecisionRequest without an ABI break (the field is Option, None in 11.4a).
    let request = PolicyDecisionRequest {
        spirit_pid: 4242,
        capability_key: "fs.read".to_string(),
        principal_attributes: Some(governed.principal_attributes.clone()),
    };
    assert!(
        request.principal_attributes.is_some(),
        "an SSO-governed PDP request MUST carry principal_attributes \
         (not the 11.4a None default)"
    );
    assert_eq!(
        request
            .principal_attributes
            .as_ref()
            .and_then(|a| a.get("email"))
            .map(String::as_str),
        Some(fixtures::EMAIL),
        "the governed principal's attributes MUST be the ones submitted to the PDP"
    );
    assert_eq!(request.spirit_pid, 4242);
    assert_eq!(request.capability_key, "fs.read");
}
