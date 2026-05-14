use crate::check_fr47;
use std::path::Path;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is directly under workspace root")
}

#[test]
fn fixture_with_anthropic_dep_fails() {
    let report = check_fr47::check_fr47(
        Some(workspace_root().join("xtask/tests/fixtures/fr47-violation").as_path()),
        workspace_root().join("xtask/fr47-vendor-sdk-denylist.toml").as_path(),
        workspace_root().join("xtask/fr47-allowlist.toml").as_path(),
    )
    .expect("check should run without error");
    assert!(!report.passed, "expected failure for fixture with anthropic dep");
    assert_eq!(report.violations.len(), 1);
    let v = &report.violations[0];
    assert_eq!(v.crate_name, "fr47-violation-fixture");
    assert_eq!(v.dependency, "anthropic");
    let msg = format!("{v}");
    assert!(
        msg.contains("FR47 violation: Spirit must obtain inference via kernel Inference Port"),
        "message must contain the literal AC2 string: {msg}"
    );
}

#[test]
fn fixture_without_vendor_sdk_passes() {
    let report = check_fr47::check_fr47(
        Some(workspace_root().join("xtask/tests/fixtures/fr47-clean").as_path()),
        workspace_root().join("xtask/fr47-vendor-sdk-denylist.toml").as_path(),
        workspace_root().join("xtask/fr47-allowlist.toml").as_path(),
    )
    .expect("check should run without error");
    assert!(report.passed, "expected pass for clean fixture");
    assert!(report.violations.is_empty());
}

#[test]
fn fixture_with_renamed_dep_fails() {
    let report = check_fr47::check_fr47(
        Some(workspace_root().join("xtask/tests/fixtures/fr47-renamed").as_path()),
        workspace_root().join("xtask/fr47-vendor-sdk-denylist.toml").as_path(),
        workspace_root().join("xtask/fr47-allowlist.toml").as_path(),
    )
    .expect("check should run without error");
    assert!(!report.passed, "expected failure for fixture with renamed anthropic dep");
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].dependency, "anthropic");
}
