#![cfg(not(feature = "sso-fault-inject"))]

//! Story 11.4c AC1 (Task 1) - CLAIMS fail-closed: expired, wrong-audience,
//! untrusted-issuer, unknown-kid, and malformed assertions are each rejected
//! with their named error. A verified principal is produced ONLY when every
//! claim + signature check passes.
//!
//! # The contracts defended
//!
//!   - `exp` in the past        -> `IdentityError::Expired`;
//!   - `aud` != this deployment -> `IdentityError::AudienceMismatch`
//!     (no cross-RP replay - the audience is THIS deployment);
//!   - `iss` not in allowlist   -> `IdentityError::IssuerUntrusted`;
//!   - `kid` not in JWKS        -> `IdentityError::JwksUnavailable`
//!     (no key available -> fail-closed, never accept-without-key);
//!   - not a JWT at all         -> `IdentityError::MalformedAssertion`.
//!
//! Each token is signed by the trusted keyA (so the signature is VALID) - the
//! reject comes from the CLAIM check, not the signature. This separates claim
//! enforcement from signature enforcement (the alg/verify legs cover the latter).

#[path = "fixtures.rs"]
mod fixtures;

use maos_domain::ports::{IdentityAssertionPort, IdentityError};
use maos_sso::{OidcAlgorithm, OidcVerifier};

fn verifier() -> OidcVerifier {
    OidcVerifier::from_static_jwks(
        fixtures::JWKS_KEY_A,
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
    )
    .expect("static JWKS + config parse")
}

/// AC1 - an expired token (exp = 1, i.e. 1970) MUST be rejected. The signature
/// is valid (signed by keyA); only the temporal validity fails.
#[test]
fn expired_token_is_rejected() {
    let result = verifier().verify(fixtures::TOKEN_EXPIRED);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::Expired),
        "an expired assertion MUST be rejected (exp in the past)"
    );
}

/// AC1 - an assertion for a DIFFERENT relying party (`aud` != this deployment)
/// MUST be rejected, even when signed by the trusted issuer. Prevents a token
/// minted for another service from being replayed against this MAOS deployment.
#[test]
fn wrong_audience_token_is_rejected() {
    let result = verifier().verify(fixtures::TOKEN_WRONG_AUD);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::AudienceMismatch),
        "an assertion for another audience MUST be rejected (no cross-RP replay)"
    );
}

/// AC1 - an assertion whose issuer is not in the trusted-issuer allowlist MUST
/// be rejected. The signature is valid (keyA) but the issuer claim is the
/// attacker domain -> untrusted.
#[test]
fn untrusted_issuer_token_is_rejected() {
    let result = verifier().verify(fixtures::TOKEN_BAD_ISS);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::IssuerUntrusted),
        "an assertion from an issuer not in the allowlist MUST be rejected"
    );
}

/// AC1 - an assertion whose `kid` is not present in the JWKS MUST fail closed.
/// There is no key to verify against; the verifier must NOT fall open (e.g.
/// try-without-kid or accept-all). JwksUnavailable = "no key available for this
/// assertion's kid" (the offline static-JWKS analogue of JWKS-fetch-failure).
#[test]
fn unknown_kid_fails_closed() {
    let result = verifier().verify(fixtures::TOKEN_UNKNOWN_KID);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::JwksUnavailable),
        "an assertion whose kid is not in the JWKS MUST fail closed (no key)"
    );
}

/// AC1 - a structurally-malformed input (not three dot-separated base64url
/// segments) MUST be rejected as MalformedAssertion, never panic, never accept.
#[test]
fn malformed_assertion_is_rejected_not_panicked() {
    let result = verifier().verify("not-a-jwt");
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::MalformedAssertion),
        "a non-JWT input MUST be rejected as MalformedAssertion (no panic, no accept)"
    );
}
