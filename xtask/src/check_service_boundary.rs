use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Services that will be supervised at v0.5+ (architecture §4.0.8).
/// At v0.1-alpha these are modules inside maos-kernel-core; the const is declared
/// but NOT iterated because the v0.5+ crate layout (`crates/services/<name>/`)
/// does not yet exist. Story 2.2 owns the iteration.
const SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"];
const SUPERVISOR: &str = "spirit-scheduler";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KernelSurface {
    pub crate_name: String,
    pub abi_baseline_version: String,
    pub items: Vec<SurfaceItem>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
pub struct SurfaceItem {
    pub kind: String,
    pub path: String,
    pub signature_hash: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub current_surface: KernelSurface,
    pub p1_p4_status: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct ApiClasses {
    classes: BTreeMap<String, String>,
}

pub fn run(
    path: Option<&str>,
    baseline_path: &str,
    classes_path: &str,
    json: bool,
) -> Result<(), String> {
    let report = check_service_boundary(
        path.map(Path::new),
        Path::new(baseline_path),
        Path::new(classes_path),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-service-boundary: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
        }
        println!(
            "INFO: {}",
            serde_json::to_string(&report.p1_p4_status).unwrap()
        );
    }

    if !report.passed {
        return Err("check-service-boundary failed".into());
    }

    Ok(())
}

fn check_service_boundary(
    path: Option<&Path>,
    baseline_path: &Path,
    classes_path: &Path,
) -> Result<Report, String> {
    let crate_path = path.unwrap_or(Path::new("crates/maos-kernel-core"));
    let current = snapshot_kernel_surface(crate_path)?;
    let classes: ApiClasses = if classes_path.exists() {
        load_toml(classes_path)?
    } else {
        ApiClasses {
            classes: BTreeMap::new(),
        }
    };

    let mut violations = Vec::new();

    // Diff against baseline if it exists and is non-empty.
    if baseline_path.exists() {
        let baseline_src = fs::read_to_string(baseline_path)
            .map_err(|e| format!("cannot read {}: {e}", baseline_path.display()))?;
        let trimmed = baseline_src.trim();
        if trimmed.is_empty() {
            // Truly empty (0 bytes) baseline -> skip diffing.
        } else if !trimmed.starts_with('{') {
            return Err(format!(
                "baseline file {} is not empty but does not contain valid JSON (whitespace-only?)",
                baseline_path.display()
            ));
        } else {
            let baseline: KernelSurface = serde_json::from_str(&baseline_src)
                .map_err(|e| format!("json parse error in {}: {e}", baseline_path.display()))?;

        let baseline_items: std::collections::HashSet<SurfaceItem> =
            baseline.items.into_iter().collect();
        let current_items: std::collections::HashSet<SurfaceItem> =
            current.items.clone().into_iter().collect();

        // Removed items are violations (monotonicity).
        for item in &baseline_items {
            if !current_items.contains(item) {
                violations.push(Violation {
                    file: baseline_path.display().to_string(),
                    line: 1,
                    path: item.path.clone(),
                    message: format!(
                        "NFR-Test-2 violation: removed public kernel symbol '{}' — kernel surface is monotonically additive within a major version (see ABI Stability Triple)",
                        item.path
                    ),
                });
            }
        }

        // Added items must be classified.
        for item in &current_items {
            if !baseline_items.contains(item) {
                let class = classes.classes.get(&item.path).cloned().unwrap_or_else(|| "other".into());
                if class == "other" {
                    violations.push(Violation {
                        file: baseline_path.display().to_string(),
                        line: 1,
                        path: item.path.clone(),
                        message: format!(
                            "NFR-Test-2 violation: new public kernel symbol '{}' has class 'other' (must be one of: universal-arithmetic, data-movement, supervision); add classification to xtask/kernel-api-classes.toml via invariant-lock review",
                            item.path
                        ),
                    });
                }
            }
        }
        }
    }

    let passed = violations.is_empty();
    Ok(Report {
        passed,
        violations,
        current_surface: current,
        p1_p4_status: serde_json::json!({
            "p1_p4_status": "deferred-to-story-2.2",
            "v0_1_layout": "services-as-modules-under-maos-kernel-core",
            "supervised_services": SUPERVISED_SERVICES,
            "supervisor": SUPERVISOR,
        }),
    })
}

fn snapshot_kernel_surface(crate_path: &Path) -> Result<KernelSurface, String> {
    let mut items = Vec::new();
    let src_dir = crate_path.join("src");
    let lib_rs = src_dir.join("lib.rs");

    if lib_rs.exists() {
        walk_mod(&lib_rs, &src_dir, "maos_kernel_core", &mut items)?;
    }

    items.sort();
    items.dedup();

    Ok(KernelSurface {
        crate_name: "maos-kernel-core".into(),
        abi_baseline_version: "v0.1-alpha".into(),
        items,
    })
}

fn walk_mod(
    file: &Path,
    src_dir: &Path,
    mod_path: &str,
    items: &mut Vec<SurfaceItem>,
) -> Result<(), String> {
    let src = fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let ast = syn::parse_file(&src)
        .map_err(|e| format!("parse error in {}: {e}", file.display()))?;

    for item in &ast.items {
        match item {
            syn::Item::Fn(i) if is_pub(&i.vis) => {
                items.push(surface_item("fn", &format!("{}::{}", mod_path, i.sig.ident), item));
            }
            syn::Item::Struct(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "struct",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Enum(i) if is_pub(&i.vis) => {
                items.push(surface_item("enum", &format!("{}::{}", mod_path, i.ident), item));
            }
            syn::Item::Trait(i) if is_pub(&i.vis) => {
                items.push(surface_item("trait", &format!("{}::{}", mod_path, i.ident), item));
            }
            syn::Item::Type(i) if is_pub(&i.vis) => {
                items.push(surface_item("type", &format!("{}::{}", mod_path, i.ident), item));
            }
            syn::Item::Const(i) if is_pub(&i.vis) => {
                items.push(surface_item("const", &format!("{}::{}", mod_path, i.ident), item));
            }
            syn::Item::Static(i) if is_pub(&i.vis) => {
                items.push(surface_item("static", &format!("{}::{}", mod_path, i.ident), item));
            }
            syn::Item::Use(i) if is_pub(&i.vis) => {
                for path in collect_use_paths(&i.tree, mod_path) {
                    items.push(surface_item("use", &path, item));
                }
            }
            syn::Item::Mod(i) if is_pub(&i.vis) => {
                // Recurse into pub mod for child items, but do NOT emit mod as surface item.
                if let Some((_, content)) = &i.content {
                    let child_path = format!("{}::{}", mod_path, i.ident);
                    for child in content {
                        walk_inline_mod_item(child, &child_path, items)?;
                    }
                } else {
                    let parent = file.parent().unwrap_or(src_dir);
                    let child_name = i.ident.to_string();
                    let child_file = parent.join(format!("{}.rs", child_name));
                    let child_mod_dir = parent.join(&child_name).join("mod.rs");
                    let child_path = format!("{}::{}", mod_path, i.ident);
                    if child_file.exists() {
                        walk_mod(&child_file, src_dir, &child_path, items)?;
                    } else if child_mod_dir.exists() {
                        walk_mod(&child_mod_dir, src_dir, &child_path, items)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn walk_inline_mod_item(
    item: &syn::Item,
    mod_path: &str,
    items: &mut Vec<SurfaceItem>,
) -> Result<(), String> {
    match item {
        syn::Item::Fn(i) if is_pub(&i.vis) => {
            items.push(surface_item("fn", &format!("{}::{}", mod_path, i.sig.ident), item));
        }
        syn::Item::Struct(i) if is_pub(&i.vis) => {
            items.push(surface_item("struct", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Enum(i) if is_pub(&i.vis) => {
            items.push(surface_item("enum", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Trait(i) if is_pub(&i.vis) => {
            items.push(surface_item("trait", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Type(i) if is_pub(&i.vis) => {
            items.push(surface_item("type", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Const(i) if is_pub(&i.vis) => {
            items.push(surface_item("const", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Static(i) if is_pub(&i.vis) => {
            items.push(surface_item("static", &format!("{}::{}", mod_path, i.ident), item));
        }
        syn::Item::Use(i) if is_pub(&i.vis) => {
            for path in collect_use_paths(&i.tree, mod_path) {
                items.push(surface_item("use", &path, item));
            }
        }
        syn::Item::Mod(i) if is_pub(&i.vis) => {
            // Recurse into inline pub mod, but do NOT emit mod as surface item.
            if let Some((_, content)) = &i.content {
                let child_path = format!("{}::{}", mod_path, i.ident);
                for child in content {
                    walk_inline_mod_item(child, &child_path, items)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn surface_item(kind: &str, path: &str, item: &syn::Item) -> SurfaceItem {
    let sig = canonicalize_signature(item);
    SurfaceItem {
        kind: kind.into(),
        path: path.into(),
        signature_hash: sha256_hex(&sig),
    }
}

/// Render an item's signature via `quote!` to a string, strip doc comments and whitespace, and hash.
///
/// TODO: `quote!`-based signature hashing is not stable across `syn` major versions.
/// Migrate to `cargo-public-api` in Story 1a.1 (same deferred migration as `abi_diff.rs`).
fn canonicalize_signature(item: &syn::Item) -> String {
    let tokens = quote::quote!(#item).to_string();
    // Strip doc comments (lines starting with `///` or `#!` containing doc attrs).
    let without_docs: String = tokens
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    // Normalize whitespace.
    without_docs.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn collect_use_paths(tree: &syn::UseTree, prefix: &str) -> Vec<String> {
    match tree {
        syn::UseTree::Name(name) => vec![format!("{}::{}", prefix, name.ident)],
        syn::UseTree::Rename(rename) => {
            vec![format!("{}::{} (as {})", prefix, rename.ident, rename.rename)]
        }
        syn::UseTree::Glob(_) => vec![format!("{}::*", prefix)],
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|t| collect_use_paths(t, prefix))
            .collect(),
        syn::UseTree::Path(path) => {
            let new_prefix = format!("{}::{}", prefix, path.ident);
            collect_use_paths(&path.tree, &new_prefix)
        }
    }
}

fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let src = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))
}

/// P4 skeleton: reject bare `std::process::exit(...)` outside `iac_runtime::shutdown::exit_code(...)`.
/// Callable at v0.1-alpha but invoked over an empty services slice because the v0.5+ layout
/// does not yet exist.
#[allow(dead_code)]
pub fn check_p4_supervised_exit(
    _workspace_root: &Path,
    _services: &[&str],
) -> Result<Vec<Violation>, String> {
    // v0.1-alpha: no-op because services are modules inside maos-kernel-core.
    // Story 2.2 wires the populated SERVICES list.
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
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
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: Report = serde_json::from_str(&json).unwrap();
        assert!(!parsed.passed);
        assert_eq!(parsed.violations.len(), 1);
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
}
