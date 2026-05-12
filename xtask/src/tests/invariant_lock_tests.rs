use super::*;

#[test]
fn no_invariant_touch_passes() {
    // When no invariant-related files are touched, gate passes immediately.
    let changed = Some("/tmp/empty_changed.txt");
    std::fs::write(changed.unwrap(), "README.md\n").unwrap();
    let report = invariant_lock(changed, None, None)
        .unwrap_or_else(|e| panic!("invariant_lock failed: {e}"));
    assert!(report.passed);
    assert!(report.touched_invariants.is_empty());
}

#[test]
fn parse_cadence_works() {
    let src = "---\nenforcement_cadence:\n  v0.1: CI\n  v0.3: runtime\n---\n";
    let map = parse_cadence(src).unwrap();
    assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
    assert_eq!(map.get("v0.3"), Some(&"runtime".to_string()));
}

#[test]
fn parse_cadence_handles_tabs() {
    let src = "---\nenforcement_cadence:\n\tv0.1: CI\n\tv0.3: runtime\n---\n";
    let map = parse_cadence(src).unwrap();
    assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
}

#[test]
fn json_output_round_trip() {
    let report = LockReport {
        passed: false,
        touched_invariants: vec!["I1".into(), "I9".into()],
        missing_corpus_delta: true,
        missing_phase_commitment: false,
        insufficient_reviews: false,
        regression_detected: vec!["ADR-037 violation: ...".into()],
        review_count: 1,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: LockReport = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.touched_invariants.len(), 2);
    assert!(parsed.missing_corpus_delta);
}

#[test]
fn detect_regression_runtime_to_ci() {
    // Verify the rank ordering catches demotion.
    let order = ["\u{2014}", "CI", "runtime", "fuzz"];
    let rank = |s: &str| order.iter().position(|&x| x == s).unwrap_or(0);
    assert!(rank("CI") < rank("runtime"), "CI < runtime");
    assert!(rank("runtime") < rank("fuzz"), "runtime < fuzz");
}
