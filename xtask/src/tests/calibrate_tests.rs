use super::*;

#[test]
fn wilson_textbook_n100_p50_z196() {
    let (lower, upper) = wilson_ci(50, 100, 1.96).unwrap();
    assert!((lower - 0.4038).abs() < 0.001, "lower={}", lower);
    assert!((upper - 0.5962).abs() < 0.001, "upper={}", upper);
}

#[test]
fn wilson_textbook_n500_p95_z1645() {
    let (lower, upper) = wilson_ci(475, 500, 1.645).unwrap();
    assert!((lower - 0.9335).abs() < 0.003, "lower={}", lower);
    assert!((upper - 0.9650).abs() < 0.003, "upper={}", upper);
}

#[test]
fn wilson_empty_set() {
    let (lower, upper) = wilson_ci(0, 0, 1.96).unwrap();
    assert_eq!(lower, 0.0);
    assert_eq!(upper, 1.0);
}

#[test]
fn wilson_ci_successes_gt_n_is_err() {
    assert!(wilson_ci(101, 100, 1.96).is_err());
}

#[test]
fn n100_ci_width_within_threshold_passes() {
    let (l, u) = wilson_ci(50, 100, 1.96).unwrap();
    let width = u - l;
    assert!(width < 0.20, "width={}", width);
}

#[test]
fn n500_ci_width_above_threshold_fails() {
    let (l, u) = wilson_ci(250, 500, 1.645).unwrap();
    let width = u - l;
    assert!(width > 0.05, "width={}", width);
}

#[test]
fn json_round_trip() {
    let report = CalibrationReport {
        corpus: "test".into(),
        n: 100,
        pass_rate: 0.95,
        ci_lower: 0.9,
        ci_upper: 0.98,
        ci_width: 0.08,
        threshold: Some(0.20),
        passed: true,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: CalibrationReport = serde_json::from_str(&json).unwrap();
    assert!(parsed.passed);
    assert_eq!(parsed.n, 100);
}
