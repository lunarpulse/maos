//! Story 12.2 — pin the single role/version consent-evaluation chokepoint.

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cohort_consent_has_one_port_call_and_two_route_uses() {
    let router = strip_line_comments(include_str!("../src/router.rs"));
    assert_eq!(
        router
            .matches("cohort_manifest_gate.consent_decision(")
            .count(),
        1,
        "only cohort_consent_decision may call the gate directly"
    );
    assert_eq!(
        router.matches("cohort_consent_decision(").count(),
        3,
        "one definition plus the outbound and inbound enforcement calls"
    );
}
