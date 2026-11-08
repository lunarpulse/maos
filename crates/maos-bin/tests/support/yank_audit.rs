use syn::visit::Visit;
#[derive(Default, Debug)]
pub struct Report {
    pub releases: Vec<String>,
    pub drains: Vec<String>,
    pub errors: Vec<String>,
}
impl Report {
    pub fn has_expected_main_arms(&self) -> bool {
        self.releases
            == [
                "cohort-a2a-daemon",
                "macro-release",
                "run",
                "serving",
                "shell",
                "standalone",
                "topology",
            ]
            && self.drains
                == [
                    "cohort-a2a-daemon",
                    "hello-spirit",
                    "run",
                    "serving",
                    "shell",
                    "smoke-epic-4",
                    "smoke-spirit-5",
                    "smoke-supervision-5",
                    "smoke-upgrade-revoke-5",
                    "standalone",
                    "topology",
                ]
    }
}
fn strip(mut expression: &syn::Expr) -> &syn::Expr {
    loop {
        expression = match expression {
            syn::Expr::Paren(inner) => &inner.expr,
            syn::Expr::Group(inner) => &inner.expr,
            syn::Expr::Try(inner) => &inner.expr,
            _ => return expression,
        };
    }
}
fn named(expression: &syn::Expr, name: &str) -> bool {
    matches!(strip(expression), syn::Expr::Path(path) if path.path.is_ident(name))
}
fn tokens_contain(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => ident == name,
        proc_macro2::TokenTree::Group(group) => tokens_contain(group.stream(), name),
        _ => false,
    })
}
fn branch(condition: &syn::Expr) -> Option<String> {
    if named(condition, "shell_mode") {
        return Some("shell".into());
    }
    match strip(condition) {
        syn::Expr::Let(binding) => match strip(&binding.expr) {
            syn::Expr::MethodCall(call) if named(&call.receiver, "run_args") => Some("run".into()),
            syn::Expr::Call(call) => {
                if matches!(call.func.as_ref(), syn::Expr::Path(path) if path.path.segments.last().is_some_and(|part| part.ident == "topology_manifest_entries"))
                {
                    return Some("topology".into());
                }
                if call.args.iter().any(|argument| matches!(argument,syn::Expr::Lit(value) if matches!(&value.lit,syn::Lit::Str(text) if text.value() == "MAOS_ONE_SHOT"))) {
                    return Some("hello-spirit".into());
                }
                None
            }
            _ => None,
        },
        syn::Expr::MethodCall(call) if call.method == "is_some" => {
            if let syn::Expr::MethodCall(get) = strip(&call.receiver) {
                if named(&get.receiver,"manifest_root") && get.method == "get"
                    && get.args.iter().any(|argument| matches!(argument,syn::Expr::Lit(value) if matches!(&value.lit,syn::Lit::Str(text) if text.value() == "cli_wrapper"))) {
                    return Some("standalone".into());
                }
            }
            None
        }
        syn::Expr::Binary(binary)
            if named(&binary.left, "mode") && matches!(binary.op, syn::BinOp::Eq(_)) =>
        {
            if let syn::Expr::Lit(value) = strip(&binary.right) {
                if let syn::Lit::Str(text) = &value.lit {
                    return Some(text.value());
                }
            }
            None
        }
        _ => None,
    }
}
struct MacroBody(syn::Block);
impl syn::parse::Parse for MacroBody {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let matcher;
        syn::parenthesized!(matcher in input);
        if !matcher.is_empty() {
            return Err(matcher.error("release macro must have no arguments"));
        }
        input.parse::<syn::Token![=>]>()?;
        let body = input.parse()?;
        if input.peek(syn::Token![;]) {
            input.parse::<syn::Token![;]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("unexpected release macro arm"));
        }
        Ok(Self(body))
    }
}
#[derive(Default)]
struct Contains {
    found: bool,
}
impl<'ast> Visit<'ast> for Contains {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.found |= path.path.is_ident("audit_tx") || path.path.is_ident("audit_writer");
        syn::visit::visit_expr_path(self, path);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.found |= tokens_contain(mac.tokens.clone(), "audit_tx")
            || tokens_contain(mac.tokens.clone(), "audit_writer");
    }
}
fn contains(node: &syn::Expr) -> bool {
    let mut visitor = Contains::default();
    visitor.visit_expr(node);
    visitor.found
}
pub fn inspect(source: &str) -> Report {
    struct Audit {
        joined: bool,
        borrowed: usize,
        awaiting: usize,
        label: String,
        macro_defined: bool,
        report: Report,
    }
    impl Audit {
        fn require_join(&mut self, operation: &str) {
            if !self.joined {
                self.report
                    .errors
                    .push(format!("{}: {operation} before yank join", self.label));
            }
        }
        fn check_macro_body(&mut self, body: &syn::Block) {
            let inherited = self.joined;
            self.joined = false;
            self.visit_block(body);
            self.joined = inherited;
        }
    }
    impl<'ast> Visit<'ast> for Audit {
        fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
            // Unit fixtures and unrelated helper roots own different channels.
            if function.sig.ident == "main" {
                self.visit_block(&function.block);
            }
        }
        fn visit_block(&mut self, block: &'ast syn::Block) {
            let inherited = self.joined;
            for statement in &block.stmts {
                self.visit_stmt(statement);
                if let syn::Stmt::Expr(expression, _) = statement {
                    if let syn::Expr::Await(awaited) = strip(expression) {
                        if matches!(strip(&awaited.base),syn::Expr::MethodCall(call) if call.method == "shutdown" && named(&call.receiver,"yank_poller_task"))
                        {
                            self.joined = true;
                        }
                    }
                }
            }
            self.joined = inherited;
        }
        fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
            self.visit_expr(&expression.cond);
            let previous = self.label.clone();
            if let Some(label) = branch(&expression.cond) {
                self.label = label;
            }
            self.visit_block(&expression.then_branch);
            self.label = previous;
            if let Some((_, otherwise)) = &expression.else_branch {
                self.visit_expr(otherwise);
            }
        }
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = strip(&call.func) {
                if path
                    .path
                    .segments
                    .last()
                    .is_some_and(|part| part.ident == "unload_all_loaded")
                {
                    self.require_join("unload");
                }
                let parts = &path.path.segments;
                let drop = path.path.is_ident("drop")
                    || (parts.len() == 3
                        && (parts[0].ident == "std" || parts[0].ident == "core")
                        && parts[1].ident == "mem"
                        && parts[2].ident == "drop");
                if drop
                    && call.args.len() == 1
                    && call
                        .args
                        .first()
                        .is_some_and(|argument| named(argument, "audit_tx"))
                {
                    self.require_join("sender release");
                    self.report.releases.push(self.label.clone());
                    return;
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            if call.method == "stop_and_join" {
                self.require_join("supervisor stop");
            }
            if named(&call.receiver, "audit_tx") && call.method == "clone" {
                for argument in &call.args {
                    self.visit_expr(argument);
                }
                return;
            }
            syn::visit::visit_expr_method_call(self, call);
        }
        fn visit_expr_reference(&mut self, reference: &'ast syn::ExprReference) {
            self.borrowed += 1;
            self.visit_expr(&reference.expr);
            self.borrowed -= 1;
        }
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            if path.path.is_ident("audit_tx") && self.borrowed == 0 {
                self.report.errors.push(format!(
                    "{}: sender move outside explicit release",
                    self.label
                ));
            }
            if path.path.is_ident("audit_writer") {
                self.require_join("writer drain");
                if self.awaiting == 0 {
                    self.report
                        .errors
                        .push("writer moved without awaited drain".into());
                }
                self.report.drains.push(self.label.clone());
            }
            syn::visit::visit_expr_path(self, path);
        }
        fn visit_expr_await(&mut self, awaited: &'ast syn::ExprAwait) {
            self.awaiting += 1;
            self.visit_expr(&awaited.base);
            self.awaiting -= 1;
        }
        fn visit_expr_async(&mut self, expression: &'ast syn::ExprAsync) {
            let mut capture = Contains::default();
            capture.visit_block(&expression.block);
            if capture.found {
                self.report
                    .errors
                    .push("audit owner captured by async task".into());
            }
            syn::visit::visit_expr_async(self, expression);
        }
        fn visit_expr_closure(&mut self, expression: &'ast syn::ExprClosure) {
            if contains(&expression.body) {
                self.report
                    .errors
                    .push("audit owner captured by closure".into());
            }
            syn::visit::visit_expr_closure(self, expression);
        }
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            if mac.path.is_ident("release_audit_tx_holders") {
                if !self.macro_defined {
                    self.report.errors.push("unverified release macro".into());
                }
                self.joined = true;
                return;
            }
            if tokens_contain(mac.tokens.clone(), "audit_tx") {
                self.report
                    .errors
                    .push("sender hidden in opaque macro".into());
            }
            if tokens_contain(mac.tokens.clone(), "audit_writer") {
                self.require_join("macro writer drain");
                if !mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|part| part.ident == "select")
                {
                    self.report
                        .errors
                        .push("writer hidden in unknown macro".into());
                }
                self.report.drains.push(self.label.clone());
            }
        }
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            if item
                .ident
                .as_ref()
                .is_some_and(|name| name == "release_audit_tx_holders")
            {
                match syn::parse2::<MacroBody>(item.mac.tokens.clone()) {
                    Ok(body) => {
                        let previous = self.label.clone();
                        self.label = "macro-release".into();
                        self.check_macro_body(&body.0);
                        self.label = previous;
                        self.macro_defined = true;
                    }
                    Err(error) => self
                        .report
                        .errors
                        .push(format!("invalid release macro: {error}")),
                }
            } else {
                self.visit_macro(&item.mac);
            }
        }
    }
    let mut audit = Audit {
        joined: false,
        borrowed: 0,
        awaiting: 0,
        label: "serving".into(),
        macro_defined: false,
        report: Report::default(),
    };
    audit.visit_file(&syn::parse_file(source).expect("valid source"));
    audit.report.releases.sort();
    audit.report.drains.sort();
    audit.report
}
