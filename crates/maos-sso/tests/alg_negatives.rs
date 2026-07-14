//! Story 11.4c AC1 (Task 1) - the ALGORITHM ALLOWLIST gate negatives: `alg:none`
//! and the HS256 alg-confusion CVE class are REJECTED, while RS256 (in the
//! allowlist) is accepted. This is Vex's binding threat-model requirement (F4):
//! an explicit `Validation` algorithm allowlist, not a permissive default.
//!
//! # The contract defended
//!
//! The verifier is constructed with `allowed_algorithms = [RS256]`. Therefore:
//!   - `alg:none` (TOKEN_ALG_NONE) -> `IdentityError::AlgorithmRejected`;
//!   - `alg:HS256` (TOKEN_HS256_CONFUSION, a real HMAC-SHA256 keyed by keyA's
//!     RSA *public* PEM - the classic confusion vector) -> `AlgorithmRejected`;
//!   - `alg:RS256` (TOKEN_GOOD_RS256) -> `Ok` (the allowlist is permissive for
//!     the *allowed* alg, restrictive for everything else).
//!
//! A verifier that does not pin an allowlist would verify the confusion token
//! using the RSA pubkey as an HMAC key -> principal-for-an-attacker-token. The
//! reject is the per-commit defense.

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

/// AC1 / F4 - `alg:none` is NEVER acceptable, regardless of a valid payload.
/// The signature segment is empty; the verifier must reject the algorithm
/// before any signature/claim check.
#[test]
fn alg_none_token_is_rejected() {
    let result = verifier().verify(fixtures::TOKEN_ALG_NONE);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::AlgorithmRejected),
        "alg:none MUST be rejected by the algorithm allowlist (no unsigned tokens)"
    );
}

/// AC1 / F4 - the HS256 alg-confusion vector is rejected. This token is a real
/// HMAC-SHA256 over the signing input keyed by keyA's RSA public-key PEM; a
/// vulnerable server that accepts HS256 would verify it with the pubkey as the
/// HMAC secret. The allowlist (RS256 only) excludes HS256 -> rejected.
#[test]
fn hs256_confusion_token_is_rejected() {
    let result = verifier().verify(fixtures::TOKEN_HS256_CONFUSION);
    assert_eq!(
        result.map_err(|e| e as IdentityError),
        Err(IdentityError::AlgorithmRejected),
        "an HS256 token MUST be rejected even though its HMAC was keyed by the \
         RSA public key - the allowlist is RS256-only (alg-confusion CVE class)"
    );
}

/// AC1 - the allowlist is permissive for the allowed algorithm. The contrast
/// leg: the SAME verifier that rejects none/HS256 ACCEPTS a valid RS256 token.
/// (Without this, a verifier that rejected *everything* would vacuously pass the
/// two negatives above - this pins that RS256 genuinely verifies.)
#[test]
fn rs256_in_allowlist_is_accepted_contrast() {
    let result = verifier().verify(fixtures::TOKEN_GOOD_RS256);
    assert!(
        result.is_ok(),
        "RS256 is in the allowlist and the token is valid -> MUST verify (the \
         allowlist is restrictive, not a blanket reject)"
    );
}
