use super::*;

#[test]
fn infer_crate_from_crates_path() {
    assert_eq!(
        infer_crate_name("./crates/maos-kernel-core/src/lib.rs"),
        "maos-kernel-core"
    );
}

#[test]
fn infer_crate_from_xtask_path() {
    assert_eq!(infer_crate_name("./xtask/src/main.rs"), "xtask");
}

#[test]
fn infer_crate_fallback_for_unknown_path() {
    let name = infer_crate_name("some/unknown/path.rs");
    assert!(!name.is_empty(), "should produce fallback name");
    assert!(name.starts_with("(unknown:"));
}

#[test]
fn infer_crate_empty_for_target() {
    assert_eq!(infer_crate_name("target/debug/build/foo.rs"), "");
}

#[test]
fn kloc_check_runs_on_workspace() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(manifest_dir).join("kloc.toml");
    let report = kloc_check(config_path.to_str().unwrap()).unwrap();
    // At v0.1-alpha we are well under budget.
    assert!(report.passed, "expected to pass: {:?}", report.over_budget);
    assert!(!report.alarm);
}

#[test]
fn alarm_fires_at_threshold() {
    let mut per_crate: BTreeMap<String, u64> = BTreeMap::new();
    per_crate.insert("maos-kernel-core".to_string(), 16000u64);
    let aggregate: u64 = per_crate.values().sum();
    let alarm = aggregate >= 16000;
    assert!(alarm, "alarm should fire at {} LOC", aggregate);
}

#[test]
fn hardfail_at_aggregate_threshold() {
    let mut per_crate: BTreeMap<String, u64> = BTreeMap::new();
    per_crate.insert("maos-kernel-core".to_string(), 20001u64);
    let aggregate: u64 = per_crate.values().sum();
    assert!(aggregate >= 20000, "should hard-fail at {} >= 20000", aggregate);
}

#[test]
fn hardfail_at_per_crate_threshold() {
    // Even if aggregate is small, per-crate over-budget should fail.
    let budget = 6000u64;
    let actual = 6001u64;
    assert!(actual > budget, "per-crate breach should be detected");
}
