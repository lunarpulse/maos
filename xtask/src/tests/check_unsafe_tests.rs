use super::*;

#[test]
fn allowed_list_is_empty() {
    assert!(ALLOWED.is_empty(), "ALLOWED must be empty at v0.1-alpha");
}

#[test]
fn detects_unsafe_fn() {
    let src = r#"unsafe fn foo() {}"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut visitor = UnsafeVisitor {
        file: "test.rs".into(),
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, "unsafe fn");
}

#[test]
fn detects_unsafe_block() {
    let src = r#"fn foo() { unsafe { println!("hi"); } }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut visitor = UnsafeVisitor {
        file: "test.rs".into(),
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, "unsafe block");
}

#[test]
fn detects_allow_unsafe_code() {
    let src = r#"#[allow(unsafe_code)] fn foo() {}"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut visitor = UnsafeVisitor {
        file: "test.rs".into(),
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, "allow(unsafe_code) attribute");
}

#[test]
fn detects_cfg_attr_allow_unsafe() {
    let src = r#"#[cfg_attr(test, allow(unsafe_code))] fn foo() {}"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut visitor = UnsafeVisitor {
        file: "test.rs".into(),
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, "allow(unsafe_code) attribute");
}

#[test]
fn recognizes_forbid_unsafe_code() {
    let src = r#"#![forbid(unsafe_code)] fn foo() {}"#;
    let ast = syn::parse_file(src).unwrap();
    assert!(has_forbid_unsafe_code(&ast.attrs));
}

#[test]
fn clean_code_has_no_violations() {
    let src = r#"fn foo() { let x = 1; }"#;
    let ast = syn::parse_file(src).unwrap();
    let mut v = Vec::new();
    let mut visitor = UnsafeVisitor {
        file: "test.rs".into(),
        violations: &mut v,
    };
    visitor.visit_file(&ast);
    assert!(v.is_empty());
}

#[test]
fn json_output_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            file: "test.rs".into(),
            line: 42,
            kind: "unsafe fn".into(),
        }],
        missing_forbid: vec!["crates/maos-kernel-core/src/lib.rs".into()],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.violations[0].kind, "unsafe fn");
    assert_eq!(parsed.missing_forbid.len(), 1);
}
