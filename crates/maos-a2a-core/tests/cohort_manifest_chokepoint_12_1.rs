//! Story 12.1 / Task 4 — guard the single cohort-currentness router seam.

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn manifest_currentness_has_one_port_call_and_two_route_uses() {
    let router = strip_line_comments(include_str!("../src/router.rs"));
    assert_eq!(
        router.matches("cohort_manifest_gate.is_current(").count(),
        1,
        "only cohort_manifest_is_current may call the gate directly"
    );
    assert_eq!(
        router.matches("cohort_manifest_is_current(").count(),
        3,
        "one definition plus outbound and inbound enforcement calls"
    );
}
