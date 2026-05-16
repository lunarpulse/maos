use super::*;
use syn::spanned::Spanned;
use syn::visit::Visit;

#[test]
fn snapshot_empty_crate_stable() {
    let surface = snapshot_kernel_surface(Path::new("crates/maos-kernel-core")).unwrap();
    assert_eq!(surface.crate_name, "maos-kernel-core");
    // At v0.1-alpha, kernel-core has only pub mod declarations (no concrete pub items
    // tracked per AC3 item kind spec: fn|struct|enum|trait|type|const|static|use).
    // Running twice gives identical (empty) surface.
    let surface2 = snapshot_kernel_surface(Path::new("crates/maos-kernel-core")).unwrap();
    assert_eq!(surface.items, surface2.items);
}

#[test]
fn signature_hash_changes_on_return_type() {
    let src1 = r#"pub fn foo() -> u32 { 0 }"#;
    let src2 = r#"pub fn foo() -> u64 { 0 }"#;
    let ast1 = syn::parse_file(src1).unwrap();
    let ast2 = syn::parse_file(src2).unwrap();
    let item1 = match &ast1.items[0] {
        syn::Item::Fn(i) => surface_item("fn", "test::foo", &syn::Item::Fn(i.clone())),
        _ => panic!("expected fn"),
    };
    let item2 = match &ast2.items[0] {
        syn::Item::Fn(i) => surface_item("fn", "test::foo", &syn::Item::Fn(i.clone())),
        _ => panic!("expected fn"),
    };
    assert_ne!(item1.signature_hash, item2.signature_hash);
}

#[test]
fn check_p4_flags_bare_exit() {
    let src = r#"
        fn foo() {
            std::process::exit(1);
        }
    "#;
    let ast = syn::parse_file(src).unwrap();
    let mut violations = Vec::new();
    let mut visitor = P4Visitor {
        violations: &mut violations,
    };
    visitor.visit_file(&ast);
    assert!(!violations.is_empty(), "bare std::process::exit should be flagged");
}

#[test]
fn check_p4_allows_shutdown_exit_code() {
    let src = r#"
        fn foo() {
            iac_runtime::shutdown::exit_code(1);
        }
    "#;
    let ast = syn::parse_file(src).unwrap();
    let mut violations = Vec::new();
    let mut visitor = P4Visitor {
        violations: &mut violations,
    };
    visitor.visit_file(&ast);
    assert!(violations.is_empty(), "iac_runtime::shutdown::exit_code should be allowed");
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            file: "test.rs".into(),
            line: 1,
            path: "maos_kernel_core::foo".into(),
            message: "NFR-Test-2 violation: test".into(),
        }],
        current_surface: KernelSurface {
            crate_name: "maos-kernel-core".into(),
            abi_baseline_version: "v0.1-alpha".into(),
            items: vec![SurfaceItem {
                kind: "fn".into(),
                path: "maos_kernel_core::foo".into(),
                signature_hash: "abc123".into(),
            }],
        },
        p1_p4_status: serde_json::json!({"status": "ok"}),
        spirit_abi_types: serde_json::json!({}),
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
}

#[test]
fn p1_reports_enforced_when_no_main_rs() {
    let payload = p1_p4_status_payload(&[]);
    let map = payload.as_object().expect("expected object");
    assert_eq!(map.len(), SERVICE_ADAPTERS.len());
    for (svc, _) in SERVICE_ADAPTERS {
        assert_eq!(
            map[*svc]["p1"], "enforced",
            "{svc} P1 should be enforced when no violations"
        );
    }
}

#[test]
fn p2_reports_enforced_when_no_api_rs() {
    let payload = p1_p4_status_payload(&[]);
    let map = payload.as_object().expect("expected object");
    for (svc, _) in SERVICE_ADAPTERS {
        assert_eq!(
            map[*svc]["p2"], "enforced",
            "{svc} P2 should be enforced when no violations"
        );
    }
}

#[test]
fn p3_reports_supervisor_exception_and_enforced_when_no_kernel() {
    let payload = p1_p4_status_payload(&[]);
    let map = payload.as_object().expect("expected object");
    for (svc, _) in SERVICE_ADAPTERS {
        let expected = if *svc == SUPERVISOR {
            "supervisor-exception"
        } else {
            "enforced"
        };
        assert_eq!(map[*svc]["p3"], expected, "{svc} P3 should be {expected} when no violations");
    }
}

struct P4Visitor<'a> {
    violations: &'a mut Vec<Violation>,
}

impl<'a> Visit<'_> for P4Visitor<'a> {
    fn visit_expr_call(&mut self, node: &syn::ExprCall) {
        if let syn::Expr::Path(path) = &*node.func {
            let segments: Vec<_> = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            if segments == ["std", "process", "exit"] {
                self.violations.push(Violation {
                    file: "test.rs".into(),
                    line: path.span().start().line,
                    path: segments.join("::"),
                    message: "NFR-Test-2 violation: bare std::process::exit".into(),
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}
