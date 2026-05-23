use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::Attribute;

use crate::fs_walk;

/// Allowlist for unsafe-code exceptions.
/// At v0.1-alpha this list is empty by design.
/// Adding any entry requires the invariant-lock review process (ADR-037).
const ALLOWED: &[&str] = &[];

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub kind: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NFR-Sec-9 violation: zero-unsafe gate failed in capability-validation path at {}:{} (item: {})",
            self.file, self.line, self.kind
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub missing_forbid: Vec<String>,
}

pub fn run(path: &str, json: bool) -> Result<(), String> {
    let report = check_unsafe(Path::new(path))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-unsafe: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
            for m in &report.missing_forbid {
                eprintln!(
                    "NFR-Sec-9 violation: missing #![forbid(unsafe_code)] in crate root: {m}"
                );
            }
        }
    }

    if !report.passed {
        return Err("check-unsafe failed".into());
    }

    Ok(())
}

fn check_unsafe(capability_path: &Path) -> Result<Report, String> {
    let mut violations = Vec::new();
    let mut missing_forbid = Vec::new();

    // Collect all .rs files under the capability subtree.
    let mut rs_files = Vec::new();
    fs_walk::collect_rs_files(capability_path, &mut rs_files);

    // Identify crate roots: any directory under capability/ that contains a lib.rs.
    let mut crate_roots = Vec::new();
    for file in &rs_files {
        if file.file_name() == Some(std::ffi::OsStr::new("lib.rs")) {
            crate_roots.push(file.clone());
        }
    }

    // Verify each crate root carries #![forbid(unsafe_code)].
    for root in &crate_roots {
        let src =
            fs::read_to_string(root).map_err(|e| format!("cannot read {}: {e}", root.display()))?;
        let ast =
            syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", root.display()))?;
        if !has_forbid_unsafe_code(&ast.attrs) {
            missing_forbid.push(root.display().to_string());
        }
    }

    // Walk every .rs file for unsafe constructs.
    for file in &rs_files {
        let src =
            fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        let ast =
            syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", file.display()))?;
        let mut visitor = UnsafeVisitor {
            file: file.display().to_string(),
            violations: &mut violations,
        };
        visitor.visit_file(&ast);
    }

    let passed = violations.is_empty() && missing_forbid.is_empty();
    Ok(Report {
        passed,
        violations,
        missing_forbid,
    })
}

fn has_forbid_unsafe_code(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("forbid") {
            if let Ok(meta) = attr.meta.require_list() {
                return meta.tokens.to_string().contains("unsafe_code");
            }
        }
        false
    })
}

fn is_allow_unsafe_code(attr: &Attribute) -> bool {
    if attr.path().is_ident("allow") || attr.path().is_ident("warn") {
        if let Ok(meta) = attr.meta.require_list() {
            return meta.tokens.to_string().contains("unsafe_code");
        }
    }
    if attr.path().is_ident("cfg_attr") {
        if let Ok(meta) = attr.meta.require_list() {
            let tokens: String = meta
                .tokens
                .to_string()
                .replace(' ', "")
                .replace('\t', "")
                .replace('\n', "");
            return tokens.contains("allow(unsafe_code)") || tokens.contains("warn(unsafe_code)");
        }
    }
    false
}

struct UnsafeVisitor<'a> {
    file: String,
    violations: &'a mut Vec<Violation>,
}

impl<'a> Visit<'_> for UnsafeVisitor<'a> {
    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        if node.sig.unsafety.is_some() {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.sig.fn_token.span().start().line,
                kind: "unsafe fn".into(),
            });
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &syn::ItemImpl) {
        if node.unsafety.is_some() {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.impl_token.span().start().line,
                kind: "unsafe impl".into(),
            });
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        if node.unsafety.is_some() {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.trait_token.span().start().line,
                kind: "unsafe trait".into(),
            });
        }
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &syn::ExprUnsafe) {
        if !ALLOWED.is_empty() {
            // If ALLOWED is populated (ADR-037 amendment), the allowlist check
            // would happen here. At v0.1-alpha ALLOWED is empty, so every
            // unsafe block is a violation.
        }
        self.violations.push(Violation {
            file: self.file.clone(),
            line: node.unsafe_token.span.start().line,
            kind: "unsafe block".into(),
        });
        syn::visit::visit_expr_unsafe(self, node);
    }

    fn visit_attribute(&mut self, node: &Attribute) {
        if is_allow_unsafe_code(node) {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.span().start().line,
                kind: "allow(unsafe_code) attribute".into(),
            });
        }
        syn::visit::visit_attribute(self, node);
    }
}

#[cfg(test)]
mod tests {
    include!("tests/check_unsafe_tests.rs");
}
