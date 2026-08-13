#![cfg(not(feature = "sso-fault-inject"))]

//! Story 11.4c Task 2 — IDENTITY-ASSERTED PROVENANCE + reconciliation.
//!
//! # The contract defended
//!
//! `govern_authorization` emits an out-of-kernel `identity.asserted`
//! provenance record that binds the authorization to the VERIFIED identity:
//! kind, subject, issuer, spirit_pid, capability_key, and a decision time.
//! `reconcile_provenance` counts that record as exactly ONE governed
//! authorization.
//!
//! # The shape pinned (ADR-051 / NFR-Sec-18)
//!
//! The record binds on identity fields (subject/issuer/spirit_pid/
//! capability_key) and deliberately carries NO `token_id` / CapabilityToken
//! reference — identity provenance MUST NOT couple to capability-token
//! issuance (the kernel stays identity-agnostic; no CapabilityToken field is
//! added to the identity surface). The assertion below reads only the
//! identity-binding fields; a `token_id` is absent by design.

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

/// `govern_authorization` emits an `identity.asserted` provenance record bound
/// to the verified identity, and that single record reconciles to ONE governed
/// authorization. The kind string pins discriminator 30 on the SSO side (the
/// audit-map side is pinned by identity_asserted_kind_test.rs in maos-audit).
#[test]
fn govern_authorization_emits_identity_asserted_provenance_reconciling_to_one() {
    let governed = verifier()
        .govern_authorization(fixtures::TOKEN_GOOD_RS256, 4242, "fs.read")
        .expect("a correctly-signed, in-audience, live assertion MUST govern authorization");

    let provenance: &IdentityProvenanceRecord = &governed.provenance;

    // kind: the out-of-kernel identity-assertion audit event (discriminator 30).
    assert_eq!(
        provenance.kind, "identity.asserted",
        "the provenance kind MUST be the identity.asserted event"
    );
    // The record binds the authorization to the VERIFIED identity (these come
    // from the cryptographically verified claims, not literals):
    assert_eq!(provenance.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(provenance.issuer, fixtures::ISS_GOOD);
    assert_eq!(provenance.spirit_pid, 4242);
    assert_eq!(provenance.capability_key, "fs.read");
    // A real decision time was captured (not the default zero) — the value is
    // wall-clock-derived so only its presence is asserted, not an exact stamp.
    assert!(
        provenance.decision_time_ns > 0,
        "provenance MUST carry a captured decision time, not the zero default"
    );

    // The single real record reconciles to exactly one governed authorization.
    assert_eq!(
        reconcile_provenance(std::slice::from_ref(provenance)),
        1,
        "one real identity.asserted record MUST reconcile to one governed authorization"
    );
}
