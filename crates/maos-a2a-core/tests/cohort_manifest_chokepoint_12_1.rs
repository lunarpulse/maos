//! Story 12.1 / Task 4 — guard atomic cohort-consent evaluation.

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn manifest_currentness_is_not_a_second_router_read() {
    let router = strip_line_comments(include_str!("../src/router.rs"));
    assert_eq!(
        router.matches(".is_current(").count(),
        0,
        "the router must not re-read cohort state after the consent verdict"
    );
    assert_eq!(
        router.matches("cohort_consent_decision(").count(),
        3,
        "one atomic gate call site plus outbound and inbound enforcement"
    );
}
