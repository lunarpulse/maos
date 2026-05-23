//! Integration test: post-revocation capability verify denial (AC6 gate test).
//!
//! v0.3-β stub — full verify-denial-under-load coverage lands with Task 15 completion.

#[test]
fn revocation_error_distinguishes_variants() {
    use maos_domain::revocation::RevocationError;
    let e1 = RevocationError::SignatureInvalid;
    let e2 = RevocationError::TrustAnchorMissing;
    assert_ne!(e1.to_string(), e2.to_string());
}
