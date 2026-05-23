//! Integration test: RevocationApplier pipeline (AC5).
//!
//! v0.3-β stub — full signature-accept/reject/idempotency/version-range coverage
//! lands with Task 15 completion.

#[test]
fn revocation_action_default() {
    use maos_domain::revocation::RevocationAction;
    assert_eq!(
        RevocationAction::default(),
        RevocationAction::TerminateImmediately
    );
}

#[test]
fn revocation_origin_serde_roundtrip() {
    use maos_domain::revocation::RevocationOrigin;
    for origin in [
        RevocationOrigin::Operator,
        RevocationOrigin::Publisher,
        RevocationOrigin::RegistryYank,
    ] {
        let json = serde_json::to_string(&origin).unwrap();
        let back: RevocationOrigin = serde_json::from_str(&json).unwrap();
        assert_eq!(back, origin);
    }
}
