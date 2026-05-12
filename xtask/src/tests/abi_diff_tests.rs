use super::*;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn snapshot_current_abi() {
    let path = workspace_root().join("crates/maos-spirit-abi");
    let snap = snapshot_abi(path.to_str().unwrap()).unwrap();
    assert_eq!(snap.abi_version, 0);
    assert!(!snap.items.is_empty());
    for item in &snap.items {
        println!("kind={} name={} sig={:?}", item.kind, item.name, item.signature);
    }
}

#[test]
fn diff_against_baseline_passes() {
    // Change cwd to workspace root so baseline path resolves.
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap();
    let report = abi_diff("HEAD~1").unwrap();
    // At v0.1-alpha with no changes, diff should pass.
    assert!(report.passed, "expected diff to pass: {:?}", report);
}

#[test]
fn json_output_round_trip() {
    let report = DiffReport {
        passed: true,
        added: vec![ApiItem {
            kind: "fn".into(),
            name: "new_item".into(),
            signature: "pub fn new_item()".into(),
        }],
        removed: vec![],
        changed: vec![],
        abi_version_current: 1,
        abi_version_baseline: 0,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: DiffReport = serde_json::from_str(&json).unwrap();
    assert!(parsed.passed);
    assert_eq!(parsed.added.len(), 1);
    assert_eq!(parsed.abi_version_current, 1);
}
