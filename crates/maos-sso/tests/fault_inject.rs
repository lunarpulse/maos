//! Story 11.4c AC1 (Task 1) - the `sso-fault-inject` FALSIFIER (dev/CI only).
//!
//! # The two-test contrast
//!
//! - `wrong_key_token_is_rejected_fail_closed` (no feature, in
//!   `oidc_verify.rs`) - the REAL verifier rejects the wrong-key token
//!   (`SignatureInvalid`).
//! - `forged_token_accepted_under_fault_inject` (`sso-fault-inject`,
//!   `#[ignore]` below) - the SAME wrong-key token is ACCEPTED because the
//!   feature stubs verification to accept-all. The verdict FLIPPED (reject ->
//!   accept) because the real signature check was removed -> the reject is
//!   signature-derived, not a constant (Section A7.3).
//!
//! The `check-enterprise-identity` gate runs the accept-all test under
//! `--features sso-fault-inject --ignored` and asserts the contrast. The
//! `compile_error!` guard in `src/lib.rs` (added by the implementer) blocks any
//! release build with this feature; the gate's release-graph-absence leg is the
//! belt-and-suspenders graph guard (SHIP-BLOCKER).

#[path = "fixtures.rs"]
mod fixtures;

use maos_domain::ports::IdentityAssertionPort;
use maos_sso::{OidcAlgorithm, OidcVerifier};

// ──────────────────── sso-fault-inject falsifier (dev/CI only) ────────────────────
//
// The feature stubs `verify` to accept-all. With it ON, the wrong-key token
// (rejected by the real verifier) is ACCEPTED -> the reject leg REDS, proving
// the reject is signature-derived. Gate-controlled `#[ignore]` keeps this out
// of the default `cargo test` run.

#[cfg(feature = "sso-fault-inject")]
#[ignore = "requires --features sso-fault-inject; gate-controlled via check-enterprise-identity"]
#[test]
fn forged_token_accepted_under_fault_inject() {
    let verifier = OidcVerifier::from_static_jwks(
        fixtures::JWKS_KEY_A,
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
    )
    .expect("static JWKS + config parse");

    // Under sso-fault-inject the real signature check is REMOVED - `verify`
    // returns a principal for the wrong-key token. This is the falsifier: the
    // verdict flipped from SignatureInvalid (real verifier) to Ok (stub) because
    // the signature check was taken out.
    let result = verifier.verify(fixtures::TOKEN_WRONG_KEY);
    assert!(
        result.is_ok(),
        "sso-fault-inject stub MUST accept the forged token (the reject reds)"
    );
}
