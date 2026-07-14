//! Story 11.4c AC1 (Task 1) - the OIDC verify TRIPWIRE: a correctly-signed,
//! in-audience, live assertion VERIFIES, and a token signed with the WRONG key
//! is rejected (fail-closed). This is the per-commit offline leg that proves the
//! verifier performs a REAL signature check against the configured JWKS.
//!
//! # The contract defended
//!
//! `maos_domain::ports::IdentityAssertionPort::verify` MUST return the
//! authenticated principal for a valid assertion and `IdentityError::SignatureInvalid`
//! for an assertion signed by a key NOT in the JWKS. The static JWKS contains
//! ONLY `keyA`; the wrong-key token is signed by `keyB` (a real RS256 token, just
//! a different signer) - so the reject is a real cryptographic mismatch, not a
//! string compare. A stubbed accept-all verifier would pass the wrong-key token
//! -> the `sso-fault-inject` leg (tests/fault_inject.rs) reds it.
//!
//! The tokens are real JWS (see tests/fixtures.rs; signatures independently
//! verified with `openssl dgst -sha256 -verify`). No live IdP, no network (L5).

#[path = "fixtures.rs"]
mod fixtures;

use maos_domain::ports::{AuthenticatedPrincipal, IdentityAssertionPort, IdentityError};
use maos_sso::{OidcAlgorithm, OidcVerifier};

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

/// AC1 - the canonical accept case. A token signed by the JWKS key, with live
/// exp, the expected audience, and a trusted issuer, verifies and yields the
/// authenticated principal with its claims projected onto attributes.
#[test]
fn good_rs256_token_verifies_and_yields_principal() {
    let principal = verifier()
        .verify(fixtures::TOKEN_GOOD_RS256)
        .expect("a correctly-signed, in-audience, live assertion MUST verify");

    // The principal is REAL: its identity fields come from the verified claims,
    // not a canned literal. These equal the JWT payload, not a hardcoded value.
    assert_eq!(principal.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(principal.issuer, fixtures::ISS_GOOD);
    assert_eq!(principal.audience, fixtures::AUD_EXPECTED);
    // Claim projection: the `email` claim must land on attributes (Task 2 feeds
    // these into principal_attributes -> PDP authorization + identity.asserted).
    assert_eq!(
        principal.attributes.get("email").map(String::as_str),
        Some(fixtures::EMAIL),
        "OIDC claims MUST project onto principal attributes"
    );
    let _: &AuthenticatedPrincipal = &principal; // type pins the contract
}

/// AC1 - the canonical REJECT case (the tripwire). The wrong-key token is a
/// real RS256 token (it verifies under keyB's pubkey) but signed by a key NOT
/// in the configured JWKS. A real signature check rejects it; a passthrough
/// verifier would accept it -> fault_inject.rs reds that.
#[test]
fn wrong_key_token_is_rejected_fail_closed() {
    let result = verifier().verify(fixtures::TOKEN_WRONG_KEY);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::SignatureInvalid),
        "an assertion signed by a key NOT in the JWKS MUST be rejected as \
         SignatureInvalid - never produce a principal (fail-closed)"
    );
}

/// AC1 - the verifier reports healthy once its static JWKS + config are loaded.
/// A configured-but-unreachable key source must NOT report healthy (the AC5
/// fail-closed leg depends on this signal).
#[test]
fn verifier_reports_healthy_with_jwks_loaded() {
    assert!(
        verifier().is_healthy(),
        "a loaded static JWKS is a healthy verifier"
    );
}
