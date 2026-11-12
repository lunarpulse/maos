#![forbid(unsafe_code)]

//! Gate A5 — `check-composition-root-completeness`.
//!
//! Per Epic 4 retro §A5: parses `crates/maos-kernel-core/src/api.rs` for `pub use`
//! re-exports naming `*Adapter` symbols, then parses
//! `crates/maos-bin/src/main.rs` for `Arc::new(<...>::<Adapter>::new(...))`
//! construction sites.  Fails if any `api.rs`-re-exported `*Adapter` lacks a
//! matching construction in `main.rs`, OR if two `Arc::new(<same>::new(...))`
//! instances exist.
//!
//! v0.3-β implementation: regex-based.  A whitelist file
//! `xtask/composition-root-whitelist.toml` may exempt adapters that legitimately
//! have multiple instances (starts empty).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn pub_use_adapter_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    // Capture only the final type name (e.g. SpiritSchedulerAdapter), not the full path.
    RE.get_or_init(|| {
        regex::Regex::new(r"pub\s+use\s+(?:[\w:]+::)*(\w+Adapter)(?:\s+as\s+\w+)?\s*;").unwrap()
    })
}

fn arc_new_adapter_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    // Match Arc::new(...Adapter::new/open/default(...)) OR ...Adapter::new/open/default(...)
    RE.get_or_init(|| {
        regex::Regex::new(
            r"(?:Arc::new\s*\(\s*)?(?:[\w:]+::)?(\w+Adapter)::(?:new|open|default)\s*\(",
        )
        .unwrap()
    })
}

fn load_whitelist(path: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let p = PathBuf::from(path);
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(toml) = content.parse::<toml::Value>() {
                if let Some(arr) = toml.get("whitelist").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            set.insert(s.to_string());
                        }
                    }
                }
            }
        }
    }
    set
}

pub fn run(api_rs: &str, main_rs: &str, whitelist_path: &str, json: bool) -> Result<(), String> {
    let api_content =
        fs::read_to_string(api_rs).map_err(|e| format!("failed to read {api_rs}: {e}"))?;
    let main_content =
        fs::read_to_string(main_rs).map_err(|e| format!("failed to read {main_rs}: {e}"))?;
    let whitelist = load_whitelist(whitelist_path);

    // 1. Collect all *Adapter symbols re-exported from api.rs.
    let mut adapters: HashSet<String> = HashSet::new();
    for cap in pub_use_adapter_re().captures_iter(&api_content) {
        adapters.insert(cap[1].to_string());
    }

    // 2. Collect all Arc::new(...Adapter::new(...)) constructions in main.rs.
    let mut constructions: HashMap<String, Vec<usize>> = HashMap::new();
    for (line_no, line) in main_content.lines().enumerate() {
        for cap in arc_new_adapter_re().captures_iter(line) {
            let name = cap[1].to_string();
            constructions.entry(name).or_default().push(line_no + 1);
        }
    }

    // 3. Check for missing constructions.
    let mut violations: Vec<String> = Vec::new();
    for adapter in &adapters {
        if !constructions.contains_key(adapter) && !whitelist.contains(adapter) {
            violations.push(format!(
                "error: adapter {adapter} is re-exported from api.rs but NOT constructed in main.rs"
            ));
        }
    }

    // 4. Check for duplicate constructions (same adapter constructed twice).
    for (adapter, lines) in &constructions {
        if lines.len() > 1 && !whitelist.contains(adapter) {
            violations.push(format!(
                "error: adapter {adapter} is constructed {} times in main.rs (lines: {:?}) — possible duplicate shared-state instance",
                lines.len(),
                lines
            ));
        }
    }
    violations.extend(crypto_provider_violations(std::path::Path::new("crates"))?);

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
            "adapters_found": adapters.len(),
            "constructions_found": constructions.len(),
        });
        println!("{}", payload);
    } else if !violations.is_empty() {
        for v in &violations {
            eprintln!("{v}");
        }
        eprintln!(
            "check-composition-root-completeness: FAIL ({} violation(s))",
            violations.len()
        );
    } else {
        eprintln!(
            "check-composition-root-completeness: PASS ({} adapter(s), {} construction(s))",
            adapters.len(),
            constructions.len()
        );
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err("composition-root-completeness violations found".to_string())
    }
}

// Every production `RingCryptoProvider` naming outside these files is refused;
// kernel T3 admission verifies through the injected provider (20-13 L2).
const CRYPTO_ALLOWED_FILES: &[(&str, &str)] = &[
    ("maos-bin/src/crypto_provider.rs", "provider selection root"),
    (
        "maos-kernel-core/src/security/crypto.rs",
        "provider implementation",
    ),
    ("maos-bench/src/halt_eval.rs", "benchmark harness"),
    ("maos-bench/src/harness/j4.rs", "benchmark harness"),
];

fn crypto_provider_violations(crates: &std::path::Path) -> Result<Vec<String>, String> {
    use syn::visit::Visit;
    let mut files = Vec::new();
    crate::fs_walk::collect_rs_files(crates, &mut files);
    crate::fs_walk::collect_rs_files(std::path::Path::new("spirits"), &mut files);
    crate::fs_walk::collect_rs_files(std::path::Path::new("examples"), &mut files);
    crate::fs_walk::collect_rs_files(std::path::Path::new("xtask/src"), &mut files);
    let mut violations = Vec::new();
    for file in files {
        let path = file.to_string_lossy().replace('\\', "/");
        let rel = path.strip_prefix("crates/").unwrap_or(&path);
        if !rel.contains("/src/")
            || rel.contains("/src/tests/")
            || CRYPTO_ALLOWED_FILES.iter().any(|(p, _)| rel == *p)
        {
            continue;
        }
        let text = fs::read_to_string(&file).map_err(|e| format!("{path}: {e}"))?;
        let ast = syn::parse_file(&text).map_err(|e| format!("{path}: {e}"))?;
        let mut visitor = CryptoConstructionVisitor {
            path: rel,
            violations: &mut violations,
        };
        visitor.visit_file(&ast);
    }
    Ok(violations)
}

struct CryptoConstructionVisitor<'a> {
    path: &'a str,
    violations: &'a mut Vec<String>,
}
impl CryptoConstructionVisitor<'_> {
    fn reject(&mut self, span: proc_macro2::Span) {
        self.violations.push(format!(
            "error: {}:{} names RingCryptoProvider outside the selection root",
            self.path,
            span.start().line
        ));
    }
}

fn names_ring(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .any(|segment| segment.ident == "RingCryptoProvider")
}

fn tokens_name_ring(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => ident == "RingCryptoProvider",
        proc_macro2::TokenTree::Group(group) => tokens_name_ring(group.stream()),
        _ => false,
    })
}

impl<'ast> syn::visit::Visit<'ast> for CryptoConstructionVisitor<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if cfg_test(&item.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !cfg_test(&item.attrs) {
            syn::visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !cfg_test(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_use_rename(&mut self, item: &'ast syn::UseRename) {
        use syn::spanned::Spanned;
        if item.ident == "RingCryptoProvider" {
            self.reject(item.span());
        }
        syn::visit::visit_use_rename(self, item);
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        use syn::spanned::Spanned;
        if names_ring(&expr.path) {
            self.reject(expr.span());
        }
        syn::visit::visit_expr_path(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        use syn::spanned::Spanned;
        if names_ring(&expr.path) {
            self.reject(expr.span());
        }
        syn::visit::visit_expr_struct(self, expr);
    }

    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        use syn::spanned::Spanned;
        if names_ring(&ty.path) {
            self.reject(ty.span());
        }
        syn::visit::visit_type_path(self, ty);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        use syn::spanned::Spanned;
        let tokens = mac.tokens.to_string();
        let generated = self.path == "maos-corpus-gen/src/lib.rs"
            && ["sha_check_sr.rs", "sha_check_rt.rs", "sha_check_ccac.rs"]
                .iter()
                .any(|name| tokens == format!("concat ! (env ! (\"OUT_DIR\") , \"/{name}\")"));
        if tokens_name_ring(mac.tokens.clone()) || (mac.path.is_ident("include") && !generated) {
            self.reject(mac.span());
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg") && {
            let text = quote::quote!(#attr).to_string().replace(' ', "");
            text == "#[cfg(test)]" || text.starts_with("#[cfg(all(test,")
        }
    })
}

#[cfg(test)]
mod crypto_tests {
    use super::*;
    use syn::visit::Visit;

    fn violations_at(path: &str, source: &str) -> usize {
        let parsed = syn::parse_file(source).unwrap();
        let mut found = Vec::new();
        CryptoConstructionVisitor {
            path,
            violations: &mut found,
        }
        .visit_file(&parsed);
        found.len()
    }

    fn violations(source: &str) -> usize {
        violations_at("maos-bin/src/main.rs", source)
    }

    #[test]
    fn crypto_construction_in_production_is_refused_after_test_module() {
        assert_eq!(violations("#[cfg(test)] mod tests { fn t() { let _ = RingCryptoProvider; } } fn root() { let _ = maos_kernel_core::security::RingCryptoProvider; }"), 1);
        assert_eq!(
            violations("fn root() { let _ = RingCryptoProvider::default(); }"),
            1
        );
        assert_eq!(
            violations("fn root() { let _ = RingCryptoProvider {}; }"),
            1
        );
        assert_eq!(
            violations("use module::RingCryptoProvider as Hidden; fn root() { let _ = Hidden; }"),
            1
        );
        for source in [
            "type Hidden = RingCryptoProvider; fn f() { let _ = Hidden; }",
            "fn f() { let _ = <RingCryptoProvider as Default>::default(); }",
            "fn f() { let _: RingCryptoProvider = Default::default(); }",
            "fn f() { let _ = vec![RingCryptoProvider]; }",
            "macro_rules! hidden { () => { RingCryptoProvider } }",
            "include!(\"hidden.rs\");",
        ] {
            assert!(violations(source) > 0, "missed {source}");
        }
        // The kernel's former T3 admission line is no longer exempt: every
        // construction in `security/mod.rs` is refused, `admit_spirit` included.
        assert_eq!(
            violations_at(
                "maos-kernel-core/src/security/mod.rs",
                "impl SecurityManagerAdapter { fn admit_spirit() { let _ = crate::security::crypto::RingCryptoProvider; } fn other() { let _ = crate::security::crypto::RingCryptoProvider; } }"
            ),
            2
        );
        // Test-only code stays outside the scan.
        assert_eq!(
            violations("impl Root { #[cfg(test)] fn t() { let _ = RingCryptoProvider; } }"),
            0
        );
    }
}
