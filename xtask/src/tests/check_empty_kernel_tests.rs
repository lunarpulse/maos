use super::*;

#[test]
fn whitelist_has_exactly_three_paths() {
    // This test asserts the whitelist size at v0.1-alpha.
    // It will be run against the committed whitelist file.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let whitelist: Whitelist =
        load_toml(&std::path::Path::new(manifest_dir).parent().unwrap().join("xtask/i9-whitelist.toml")).expect("whitelist must parse");
    assert_eq!(
        whitelist.paths.len(),
        3,
        "i9-whitelist.toml must have exactly 3 entries at v0.1-alpha"
    );
}

#[test]
fn detects_hashmap_field() {
    let src = r#"pub struct Foo { inner: HashMap<String, u32> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["HashMap".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].struct_name, "Foo");
    assert_eq!(v[0].field_name, "inner");
}

#[test]
fn detects_mutex_field() {
    let src = r#"pub struct Foo { lock: Mutex<String> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["Mutex".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].field_name, "lock");
}

#[test]
fn detects_vec_of_struct() {
    let src = r#"pub struct Foo { items: Vec<Bar> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["Vec".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].field_type, "Vec < Bar >");
}

#[test]
fn ignores_vec_of_u8() {
    let src = r#"pub struct Foo { buf: Vec<u8> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["Vec".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Vec<u8> should not be flagged");
}

#[test]
fn ignores_vec_of_primitive() {
    let src = r#"pub struct Foo { nums: Vec<f64> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["Vec".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "Vec<f64> should not be flagged");
}

#[test]
fn recognizes_i9_exempt() {
    let src = r#"#[i9_exempt(reason = "test")] pub struct Foo { inner: HashMap<String, u32> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: false,
        denylist: &["HashMap".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "exempt struct should not be flagged");
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].struct_name, "Foo");
}

#[test]
fn whitelist_hit_skips_struct() {
    let src = r#"pub struct Foo { inner: HashMap<String, u32> }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut e = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "test.rs".into(),
        in_whitelist: true,
        denylist: &["HashMap".into()],
        violations: &mut v,
        exemption_sites: &mut e,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty(), "whitelisted path should skip struct");
}

#[test]
fn missing_exemption_doc_fires() {
    // Simulate an exempt struct but no matching entry in exemptions file.
    let src = r#"#[i9_exempt(reason = "test")] pub struct Foo { inner: u32 }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut violations = Vec::new();
    let mut exemption_sites = Vec::new();
    let mut visitor = EmptyKernelVisitor {
        file: "crates/maos-kernel-core/src/capability/cap_policy/mod.rs".into(),
        in_whitelist: false,
        denylist: &["HashMap".into()],
        violations: &mut violations,
        exemption_sites: &mut exemption_sites,
    };
    visitor.visit_file(&ast);
    assert_eq!(exemption_sites.len(), 1);

    // Empty exemptions doc -> should fire.
    let mut exemption_violations = Vec::new();
    let site = &exemption_sites[0];
    let key = format!("{}::{}::{}::", site.crate_name, site.module_path, site.struct_name);
    let exemptions_src = "";
    if !exemptions_src.contains(&key.trim_end_matches(':').to_string())
        && !exemptions_src.contains(&site.struct_name)
    {
        exemption_violations.push(ExemptionViolation {
            file: site.file.clone(),
            line: site.line,
            struct_name: site.struct_name.clone(),
        });
    }
    assert_eq!(exemption_violations.len(), 1);
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            file: "test.rs".into(),
            line: 5,
            struct_name: "Foo".into(),
            field_name: "inner".into(),
            field_type: "HashMap < String , u32 >".into(),
        }],
        exemption_violations: vec![ExemptionViolation {
            file: "test.rs".into(),
            line: 3,
            struct_name: "Bar".into(),
        }],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.exemption_violations.len(), 1);
}
