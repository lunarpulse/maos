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
        2,
        "one definition plus the UNVERIFIED intake fallback — the verified \
         accept path consumes the precomputed verdict instead (Story 13.6a \
         review P2)"
    );
    // Story 13.6a review P1/P2 — the single-snapshot consent+team port has
    // exactly two route uses: the Send seam (verdict + source-team stamp) and
    // the verified Accept seam (team-identity check + precomputed verdict).
    assert_eq!(
        router
            .matches("cohort_manifest_gate.consent_and_team(")
            .count(),
        2,
        "the combined consent+team port is called at the two seams only"
    );
}
