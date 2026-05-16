use super::*;

#[test]
fn blocklist_has_exactly_four_entries() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let blocklist: Blocklist =
        load_toml(&std::path::Path::new(manifest_dir).parent().unwrap().join("xtask/loom-blocklist.toml")).expect("blocklist must parse");
    assert_eq!(
        blocklist.blocklist.len(),
        4,
        "loom-blocklist.toml must have exactly 4 entries at v0.1-alpha"
    );
}

#[test]
fn detects_planner_struct() {
    let src = r#"pub struct Planner { id: u32 }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> = ["Planner".into()].into_iter().collect();
    let allowset: HashSet<(String, String)> = HashSet::new();
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].identifier, "Planner");
    assert_eq!(v[0].kind, "ItemStruct");
}

#[test]
fn detects_use_orchestrator() {
    let src = r#"use spirits_api::Orchestrator;"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> = ["Orchestrator".into()].iter().cloned().collect();
    let allowset: HashSet<(String, String)> = HashSet::new();
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].identifier, "Orchestrator");
    assert_eq!(v[0].kind, "ItemUse");
}

#[test]
fn ignores_planner_in_tests_mod() {
    let src = r#"
        mod tests {
            pub struct Planner { id: u32 }
        }
    "#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> = ["Planner".into()].iter().cloned().collect();
    let allowset: HashSet<(String, String)> = HashSet::new();
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Planner inside mod tests should be ignored");
}

#[test]
fn ignores_planner_in_cfg_test() {
    let src = r#"
        #[cfg(test)]
        mod foo {
            pub struct Planner { id: u32 }
        }
    "#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> = ["Planner".into()].iter().cloned().collect();
    let allowset: HashSet<(String, String)> = HashSet::new();
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Planner inside #[cfg(test)] should be ignored");
}

#[test]
fn ignores_comment_loom() {
    // syn strips doc comments / regular comments, so they are never visited.
    let src = r#"
        // Loom
        /// Orchestrator
        pub struct Foo {}
    "#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> =
        ["Loom".into(), "Orchestrator".into()].iter().cloned().collect();
    let allowset: HashSet<(String, String)> = HashSet::new();
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Comments should not trigger violations");
}

#[test]
fn allowlist_hit_skips() {
    let src = r#"pub struct Planner { id: u32 }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let blockset: HashSet<String> = ["Planner".into()].iter().cloned().collect();
    let mut allowset: HashSet<(String, String)> = HashSet::new();
    allowset.insert(("test.rs".into(), "Planner".into()));
    let mut visitor = LoomVisitor {
        file: "test.rs".into(),
        blockset: &blockset,
        allowset: &allowset,
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Allowlisted entry should be skipped");
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            file: "test.rs".into(),
            line: 2,
            identifier: "Planner".into(),
            kind: "ItemStruct".into(),
        }],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.violations[0].identifier, "Planner");
}
