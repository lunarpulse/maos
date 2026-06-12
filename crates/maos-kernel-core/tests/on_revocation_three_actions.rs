#![forbid(unsafe_code)]

//! Integration test: on-revocation three action variants (AC5).
//!
//! Verifies `RevocationAction` default, serde, and the `[on_revocation]`
//! manifest section parsing.

#[test]
fn revocation_action_default_is_terminate_immediately() {
    use maos_domain::revocation::RevocationAction;
    assert_eq!(
        RevocationAction::default(),
        RevocationAction::TerminateImmediately
    );
}

#[test]
fn revocation_action_all_variants_serde_roundtrip() {
    use maos_domain::revocation::RevocationAction;
    for action in [
        RevocationAction::TerminateImmediately,
        RevocationAction::DrainThenTerminate,
        RevocationAction::Quarantine,
    ] {
        let json = serde_json::to_string(&action).unwrap();
        let back: RevocationAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action, "serde roundtrip failed for {action:?}");
    }
}

#[test]
fn revocation_action_serde_strings() {
    use maos_domain::revocation::RevocationAction;
    assert_eq!(
        serde_json::to_string(&RevocationAction::TerminateImmediately).unwrap(),
        "\"terminate_immediately\""
    );
    assert_eq!(
        serde_json::to_string(&RevocationAction::DrainThenTerminate).unwrap(),
        "\"drain_then_terminate\""
    );
    assert_eq!(
        serde_json::to_string(&RevocationAction::Quarantine).unwrap(),
        "\"quarantine\""
    );
}

#[test]
fn on_revocation_section_parses_terminate_immediately() {
    let section = maos_kernel_core::security::manifest::OnRevocationSection::from_toml_str(
        "action = \"terminate-immediately\"",
    )
    .unwrap();
    assert_eq!(
        section.action,
        maos_domain::revocation::RevocationAction::TerminateImmediately
    );
}

#[test]
fn on_revocation_section_parses_drain_then_terminate() {
    let section = maos_kernel_core::security::manifest::OnRevocationSection::from_toml_str(
        "action = \"drain-then-terminate\"",
    )
    .unwrap();
    assert_eq!(
        section.action,
        maos_domain::revocation::RevocationAction::DrainThenTerminate
    );
}

#[test]
fn on_revocation_section_parses_quarantine() {
    let section = maos_kernel_core::security::manifest::OnRevocationSection::from_toml_str(
        "action = \"quarantine\"",
    )
    .unwrap();
    assert_eq!(
        section.action,
        maos_domain::revocation::RevocationAction::Quarantine
    );
}

#[test]
fn on_revocation_section_default_empty_string() {
    let section =
        maos_kernel_core::security::manifest::OnRevocationSection::from_toml_str("").unwrap();
    assert_eq!(
        section.action,
        maos_domain::revocation::RevocationAction::TerminateImmediately
    );
}

#[test]
fn on_revocation_section_rejects_unknown_action() {
    let result = maos_kernel_core::security::manifest::OnRevocationSection::from_toml_str(
        "action = \"unknown-policy\"",
    );
    assert!(result.is_err(), "unknown action must be rejected");
}
