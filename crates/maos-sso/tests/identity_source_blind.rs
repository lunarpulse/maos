#![cfg(not(feature = "sso-fault-inject"))]

//! Story 11.4c Task 2 — BLIND-SOURCE reconciliation reflex.
//!
//! # The contract defended
//!
//! `reconcile_provenance` MUST reject provenance records that did not come
//! from a real `govern_authorization` verification. A hand-built / synthetic
//! record with IDENTICAL-looking fields must NOT inflate the reconciled count
//! — only records actually minted by a verified `govern_authorization` count.
//!
//! # The mechanism pinned (red-tests are the spec)
//!
//! `IdentityProvenanceRecord` carries a private attestation seal that only
//! `govern_authorization` sets. The public constructor
//! [`IdentityProvenanceRecord::synthetic`] builds a record with every
//! observable field populated but the seal UNSET — i.e. an unverified/blind
//! source. `reconcile_provenance` counts records whose seal is set, so a
//! synthetic record — however plausible its fields — reconciles to zero.
//!
//! This is the "you cannot fabricate provenance" invariant: correct-looking
//! data is not enough; the record must be ATTESTED to by a real verification.
//! A mutation that makes `reconcile_provenance` count every record (ignoring
//! the seal) would let an attacker forge identity-provenance after the fact.

#[path = "fixtures.rs"]
mod fixtures;

use maos_sso::{reconcile_provenance, IdentityProvenanceRecord, OidcAlgorithm, OidcVerifier};

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

/// A hand-built/synthetic provenance record — every observable field shaped to
/// look exactly like a real one — MUST reconcile to ZERO, and MUST NOT inflate
/// the count of a real record. Only records minted by `govern_authorization`
/// (attested) count.
#[test]
fn synthetic_record_does_not_inflate_reconciled_count() {
    let governed = verifier()
        .govern_authorization(fixtures::TOKEN_GOOD_RS256, 4242, "fs.read")
        .expect("a correctly-signed, in-audience, live assertion MUST govern authorization");
    let real = governed.provenance;

    // A blind-source record: same kind, same subject/issuer/spirit_pid/
    // capability_key, same decision time — indistinguishable by field value
    // from the real one. It is the strongest possible forgery the seal must
    // defeat.
    let blind = IdentityProvenanceRecord::synthetic(
        "identity.asserted",
        fixtures::PRINCIPAL_SUB,
        fixtures::ISS_GOOD,
        4242,
        "fs.read",
        real.decision_time_ns,
    );

    // A purely synthetic record reconciles to ZERO on its own.
    assert_eq!(
        reconcile_provenance(std::slice::from_ref(&blind)),
        0,
        "a synthetic/blind-source record MUST NOT count — it carries no attestation \
         that it was minted by a real govern_authorization verification"
    );

    // A blind record must NOT inflate the count of a real one — only the
    // attested record counts, even when the blind one is field-identical.
    assert_eq!(
        reconcile_provenance(&[real, blind]),
        1,
        "only the real (attested) record counts; the blind one is rejected even \
         with identical-looking fields"
    );
}
