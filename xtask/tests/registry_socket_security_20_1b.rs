//! Portable source contracts for behavior that needs macOS or descriptor faults.
use syn::spanned::Spanned;
use syn::visit::Visit;

fn source(path: &str) -> syn::File {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    syn::parse_file(&std::fs::read_to_string(root.join(path)).expect("source"))
        .expect("valid source")
}
#[derive(Default)]
struct AcceptAudit {
    blocking_line: Option<usize>,
    clone_line: Option<usize>,
    fallible_clone: bool,
    accept_backoff: bool,
}
impl<'ast> Visit<'ast> for AcceptAudit {
    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if matches!(expression.expr.as_ref(), syn::Expr::MethodCall(call) if call.method == "accept")
        {
            for arm in &expression.arms {
                if let syn::Pat::TupleStruct(pattern) = &arm.pat {
                    if pattern.path.is_ident("Ok") {
                        self.visit_expr(&arm.body);
                    } else if pattern.path.is_ident("Err") && arm.guard.is_none() {
                        struct Sleeps(bool);
                        impl<'ast> Visit<'ast> for Sleeps {
                            fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
                                self.0 |= matches!(call.func.as_ref(), syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "sleep"));
                                syn::visit::visit_expr_call(self, call);
                            }
                        }
                        let mut sleeps = Sleeps(false);
                        sleeps.visit_expr(&arm.body);
                        self.accept_backoff |= sleeps.0;
                    }
                }
            }
        } else {
            syn::visit::visit_expr_match(self, expression);
        }
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "set_nonblocking"
            && matches!(call.args.first(), Some(syn::Expr::Lit(literal)) if matches!(literal.lit, syn::Lit::Bool(ref boolean) if !boolean.value))
        {
            self.blocking_line = Some(call.span().start().line);
        }
        if call.method == "try_clone" {
            self.clone_line = Some(call.span().start().line);
        }
        syn::visit::visit_expr_method_call(self, call);
    }
    fn visit_expr_try(&mut self, expression: &'ast syn::ExprTry) {
        struct CloneCall(bool);
        impl<'ast> Visit<'ast> for CloneCall {
            fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
                self.0 |= call.method == "try_clone";
                syn::visit::visit_expr_method_call(self, call);
            }
        }
        let mut finder = CloneCall(false);
        finder.visit_expr(&expression.expr);
        self.fallible_clone |= finder.0;
        syn::visit::visit_expr_try(self, expression);
    }
}
fn accept_audit() -> AcceptAudit {
    let mut audit = AcceptAudit::default();
    audit.visit_file(&source("crates/maos-registry/src/server.rs"));
    audit
}
#[test]
fn accepted_registry_sockets_are_blocking_before_clone_p2() {
    let audit = accept_audit();
    assert!(
        matches!((audit.blocking_line, audit.clone_line), (Some(blocking), Some(clone)) if blocking < clone),
        "accepted sockets must explicitly clear inherited O_NONBLOCK before dispatch"
    );
}
#[test]
fn clone_failure_cannot_exit_the_registry_accept_loop_f3() {
    assert!(
        !accept_audit().fallible_clone,
        "a per-connection clone failure must not propagate out of serve"
    );
}
#[test]
fn registry_accept_errors_have_backoff_f3() {
    assert!(
        accept_audit().accept_backoff,
        "non-WouldBlock accept errors must back off"
    );
}
#[derive(Default)]
struct BootAudit {
    old_readers: usize,
    shared_readers: usize,
    installed_boot_line: Option<usize>,
    poller_line: Option<usize>,
}
impl<'ast> Visit<'ast> for BootAudit {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            let names = &path.path.segments;
            self.old_readers += usize::from(
                names.iter().any(|part| part.ident == "RegistrySection")
                    && names
                        .last()
                        .is_some_and(|part| part.ident == "resolve_from_env_and_disk"),
            );
            self.shared_readers += usize::from(
                names.len() == 3
                    && names[0].ident == "maos_registry"
                    && names[1].ident == "install"
                    && names[2].ident == "operator_config",
            );
            if names.iter().any(|part| part.ident == "InstalledSpirits")
                && names.last().is_some_and(|part| part.ident == "boot")
            {
                self.installed_boot_line = Some(call.span().start().line);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if matches!(&local.pat, syn::Pat::Ident(name) if name.ident == "yank_poller_task") {
            self.poller_line = Some(local.span().start().line);
        }
        syn::visit::visit_local(self, local);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        fn readers(tokens: proc_macro2::TokenStream) -> u8 {
            tokens.into_iter().fold(0, |flags, token| {
                flags
                    | match token {
                        proc_macro2::TokenTree::Group(group) => readers(group.stream()),
                        proc_macro2::TokenTree::Ident(name) if name == "RegistrySection" => 1,
                        proc_macro2::TokenTree::Ident(name)
                            if name == "resolve_from_env_and_disk" =>
                        {
                            2
                        }
                        proc_macro2::TokenTree::Ident(name) if name == "maos_registry" => 4,
                        proc_macro2::TokenTree::Ident(name) if name == "operator_config" => 8,
                        _ => 0,
                    }
            })
        }
        let flags = readers(mac.tokens.clone());
        self.old_readers += usize::from(flags & 3 == 3);
        self.shared_readers += usize::from(flags & 12 == 12);
    }
}
fn boot_audit() -> BootAudit {
    let mut audit = BootAudit::default();
    audit.visit_file(&source("crates/maos-bin/src/main.rs"));
    audit
}
#[test]
fn daemon_registry_policy_has_one_fail_closed_reader_d1_p10() {
    let audit = boot_audit();
    assert_eq!(
        audit.old_readers, 0,
        "do not combine lenient kernel policy with a second strict reader"
    );
    assert_eq!(
        audit.shared_readers, 1,
        "exactly one shared registry reader"
    );
}
#[test]
fn registry_poller_starts_after_shared_installed_boot_admission_f9() {
    let audit = boot_audit();
    assert!(
        matches!((audit.installed_boot_line, audit.poller_line), (Some(boot), Some(poller)) if boot < poller),
        "finish fallible shared boot admission before starting the reference-owning poller"
    );
}

#[test]
fn registry_bodies_do_not_preallocate_declared_lengths_f2() {
    struct Allocations(bool);
    impl<'ast> Visit<'ast> for Allocations {
        fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
            self.0 |= expression.mac.path.is_ident("vec")
                && expression.mac.tokens.to_string().contains("content_length");
            syn::visit::visit_expr_macro(self, expression);
        }
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if matches!(call.func.as_ref(),syn::Expr::Path(path) if path.path.segments.last().is_some_and(|part|part.ident == "with_capacity"))
            {
                self.0 |= call.args.iter().any(|argument| matches!(argument,syn::Expr::Path(path) if path.path.is_ident("content_length")));
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let mut audit = Allocations(false);
    audit.visit_file(&source("crates/maos-registry/src/server.rs"));
    assert!(
        !audit.0,
        "never allocate a request's declared body before receiving bytes"
    );
}

#[test]
fn registry_tls_features_are_explicit_not_transitive_defaults_d2() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    for (package, dependency) in [
        ("maos-bin", "maos-cli"),
        ("maos-shell", "maos-cli"),
        ("maos-cli", "maos-registry"),
        ("maos-fkcs", "maos-registry"),
    ] {
        let manifest: toml::Value =
            std::fs::read_to_string(root.join("crates").join(package).join("Cargo.toml"))
                .unwrap()
                .parse()
                .unwrap();
        assert_eq!(
            manifest["dependencies"][dependency]["default-features"].as_bool(),
            Some(false),
            "{package} must not inherit {dependency}'s TLS default"
        );
    }
    let manifest = |package: &str| -> toml::Value {
        std::fs::read_to_string(root.join("crates").join(package).join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap()
    };
    let cli = manifest("maos-cli");
    assert!(cli["features"]["tls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature.as_str() == Some("maos-registry/tls")));
    let bin = manifest("maos-bin");
    assert!(bin["features"]["network"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature.as_str() == Some("maos-cli/tls")));
}
